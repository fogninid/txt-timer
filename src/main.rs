mod maximals;
mod timer;

use crate::maximals::Maximals;
use crate::timer::{ChronoTimer, RegexTimer, Stamp, Timer};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use colored::Colorize;
use itertools::Itertools;
use regex::Regex;
use signal_hook::consts::TERM_SIGNALS;
use signal_hook::flag;
use signal_hook::iterator::Signals;
use std::collections::VecDeque;
use std::fmt::Formatter;
use std::io::BufRead;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{fmt, fs, io, thread, vec};

#[derive(Parser)]
/// Pipe through standard input while highlighting and keeping track of delays between lines.
///
/// When completed print summary of maximum delays
struct Cli {
    /// do not output stdin
    #[clap(short = 'q', long, value_parser, default_value_t = false)]
    quiet: bool,
    /// number of top differences to print at the end
    #[clap(short, long, value_parser, default_value_t = 5)]
    count: usize,
    #[clap(short = 'B', long, value_parser, default_value_t = 5)]
    lines_before: usize,
    /// range for color scale of delay, in seconds
    #[clap(long, value_parser, default_value_t = 0.2)]
    color_range: f32,
    /// use regex to extract timestamp from lines instead of using real time, expecting iso8601=ms
    /// YYYY-mm-ddTHH-MM-SS.3fZ
    #[clap(long, value_parser)]
    time_regex_iso: bool,
    /// use regex to extract timestamp from lines instead of using real time, must have one (?<time> ) named capturing group
    #[clap(long, value_parser)]
    time_regex: Option<Regex>,
    /// format of timestamp, without timezone see `strftime`. Example `%Y-%m-%d %H:%M:%S%.3f`
    #[clap(long, value_parser)]
    time_regex_format: Option<String>,
    /// prepend time to output
    #[clap(short, long, value_parser, default_value_t = false)]
    prepend_time: bool,
    /// redirect output of maximum differences to a file
    #[clap(short, long, value_parser)]
    output_maximals: Option<PathBuf>,
}

impl Cli {
    fn parse_and_validate() -> Cli {
        let cli = Cli::parse();

        if cli.color_range <= 0.0 {
            Cli::command()
                .error(ErrorKind::InvalidValue, "color range must be positive")
                .exit();
        }

        cli
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
struct MaximalsStampsEntry {
    stamp: Stamp,
    lines: Vec<Rc<str>>,
}

impl fmt::Display for MaximalsStampsEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Δ{:.4} @{:.4}",
            self.stamp.last.as_secs_f32(),
            self.stamp.total.as_secs_f32()
        )?;

        for l in &self.lines {
            write!(f, "{l}")?;
        }
        Ok(())
    }
}

struct MaximalsStampsBuffer {
    max: Maximals<MaximalsStampsEntry>,
    lines: VecDeque<Rc<str>>,
    lines_count: usize,
}

impl MaximalsStampsBuffer {
    fn new(count: usize, c: usize) -> Self {
        MaximalsStampsBuffer {
            max: Maximals::new(count),
            lines: VecDeque::with_capacity(c),
            lines_count: c,
        }
    }

    fn insert(&mut self, stamp: Stamp, value: &str) {
        self.lines.push_back(Rc::from(value));
        if self.lines.len() > self.lines_count + 1 {
            self.lines.pop_front();
        }

        if let Some(b) = self.max.insert(MaximalsStampsEntry {
            stamp,
            lines: vec![],
        }) {
            b.lines.extend(self.lines.iter().cloned());
        };
    }
}

impl fmt::Display for MaximalsStampsBuffer {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for e in self.max.iter() {
            writeln!(f, "{e}")?;
            writeln!(f)?;
        }
        Ok(())
    }
}

fn print_stamp<T: io::Write>(cli: &Cli, stamp: &Stamp, writer: &mut T) -> io::Result<()> {
    if cli.prepend_time {
        let x = stamp.last.as_secs_f32();
        let x_scale = x / cli.color_range;
        let r: u8 = (255.0 * (2.0 * x_scale)).clamp(0.0, 255.0) as u8;
        let g: u8 = (255.0 * (2.0 - 2.0 * x_scale)).clamp(0.0, 255.0) as u8;
        writeln!(
            writer,
            "Δ{} @{} {}",
            format!("{x:.4}").truecolor(r, g, 0),
            format!("{:.4}", stamp.total.as_secs_f32()).blue(),
            stamp.utc.to_rfc3339().bold().white()
        )
    } else {
        Ok(())
    }
}

fn make_timer(cli: &mut Cli) -> Box<dyn Timer> {
    match (
        cli.time_regex.take(),
        cli.time_regex_format.take(),
        cli.time_regex_iso,
    ) {
        (Some(regex), Some(fmt), false) => {
            if !regex.capture_names().contains(&Some("time")) {
                Cli::command()
                    .error(
                        ErrorKind::InvalidValue,
                        "regex must have a `(?P<time>exp)` capturing group",
                    )
                    .exit();
            }
            Box::new(RegexTimer::new(regex, fmt.as_str()))
        }
        (None, None, true) => {
            let regex = Regex::new(
                r"(?P<time>[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}\.[0-9]{3})Z",
            )
            .unwrap();
            Box::new(RegexTimer::new(regex, "%Y-%m-%dT%H:%M:%S%.3f"))
        }
        (None, None, false) => Box::new(ChronoTimer::new()),
        _ => Cli::command()
            .error(
                ErrorKind::InvalidValue,
                "time regex and format must be either both present or absent",
            )
            .exit(),
    }
}

struct Handler {
    timer: Box<dyn Timer>,
    max: MaximalsStampsBuffer,
    cli: Cli,
}

impl Handler {
    fn new(mut cli: Cli) -> Self {
        let max = MaximalsStampsBuffer::new(cli.count, cli.lines_before);

        let timer = make_timer(&mut cli);

        Handler { timer, max, cli }
    }

    fn process_line<T: io::Write>(&mut self, buffer: &str, writer: &mut T) -> io::Result<()> {
        if let Some(stamp) = self.timer.stamp(buffer) {
            print_stamp(&self.cli, &stamp, writer)?;
            self.max.insert(stamp, buffer);
        };
        if !self.cli.quiet {
            write!(writer, "{buffer}")?;
        }
        writer.flush()
    }

    fn print_and_end<T: io::Write>(self, writer: &mut T) -> io::Result<()> {
        let max = self.max;
        let cli = self.cli;
        match cli.output_maximals {
            None => writeln!(writer, "\n{}:\n{}", "Maximals".yellow().bold(), max),
            Some(filename) => fs::write(filename, format!("{max}")),
        }
    }
}

fn read_and_process(cli: Cli, term_flag: Arc<AtomicBool>) -> io::Result<()> {
    let mut handler = Handler::new(cli);

    let mut buffer = String::new();
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    while !term_flag.load(Ordering::Relaxed) && stdin.read_line(&mut buffer)? > 0 {
        handler.process_line(&buffer, &mut stdout)?;
        buffer.clear();
    }
    handler.print_and_end(&mut stdout)
}

fn main() -> io::Result<()> {
    let cli: Cli = Cli::parse_and_validate();

    let term = Arc::new(AtomicBool::new(false));

    for sig in TERM_SIGNALS {
        flag::register_conditional_shutdown(*sig, 1, Arc::clone(&term))?;
        flag::register(*sig, Arc::clone(&term))?;
    }

    let mut signals = Signals::new(TERM_SIGNALS)?;

    let signals_handle = signals.handle();

    let join_handle = thread::spawn(move || -> io::Result<()> {
        let rv = read_and_process(cli, term);
        signals_handle.close();
        rv
    });

    signals.wait();

    join_handle
        .join()
        .expect("waiting processing thread failed")
}
