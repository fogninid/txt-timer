mod maximals;
mod timer;

use crate::maximals::Maximals;
use crate::timer::{ChronoTimer, RegexTimer, Stamp, Timer};
use clap::{CommandFactory, ErrorKind, Parser};
use colored::Colorize;
use itertools::Itertools;
use regex::Regex;
use std::collections::VecDeque;
use std::fmt::Formatter;
use std::io::BufRead;
use std::path::PathBuf;
use std::rc::Rc;
use std::{fmt, fs, io, vec};

#[derive(Parser)]
/// Pipe through standard input while highlighting and keeping track of delays between lines.
///
/// When completed print summary of maximum delays
struct Cli {
    /// number of top differences to print at the end
    #[clap(short, long, value_parser, default_value_t = 5)]
    count: usize,
    #[clap(short = 'B', long, value_parser, default_value_t = 5)]
    lines_before: usize,
    /// colorized output
    #[clap(long, value_parser, default_value_t = false)]
    color: bool,
    /// range for color scale of delay, in seconds
    #[clap(long, value_parser, default_value_t = 0.2)]
    color_range: f32,
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
    #[clap(short, long, parse(from_os_str))]
    output_maximals: Option<PathBuf>,
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
            write!(f, "{}", l)?;
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
        for e in self.max.data() {
            writeln!(f, "{}", e)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

fn print_stamp(cli: &Cli, stamp: &Stamp) {
    if cli.prepend_time {
        if cli.color {
            let x = stamp.last.as_secs_f32();
            let x_scale = x / cli.color_range;
            let r: u8 = (255.0 * (2.0 * x_scale)).min(255.0).max(0.0) as u8;
            let g: u8 = (255.0 * (2.0 - 2.0 * x_scale)).min(255.0).max(0.0) as u8;
            println!(
                "Δ{} @{}",
                format!("{:.4}", x).truecolor(r, g, 0),
                format!("{:.4}", stamp.total.as_secs_f32()).blue()
            );
        } else {
            println!(
                "{} @ {}",
                stamp.last.as_secs_f32(),
                stamp.total.as_secs_f32()
            );
        }
    }
}

fn make_timer(cli: &mut Cli) -> Box<dyn Timer> {
    match (cli.time_regex.take(), cli.time_regex_format.take()) {
        (Some(regex), Some(fmt)) => {
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
        (None, None) => Box::new(ChronoTimer::new()),
        _ => Cli::command()
            .error(
                ErrorKind::InvalidValue,
                "time regex and format must be either both present or absent",
            )
            .exit(),
    }
}

fn print_maximals(cli: &mut Cli, max: &MaximalsStampsBuffer) -> io::Result<()> {
    match cli.output_maximals.take() {
        None => {
            if cli.color {
                println!("\n{}:\n{}", "Maximals".yellow().bold(), max);
            } else {
                println!("\nMaximals:\n{}", max);
            }
        }
        Some(filename) => {
            fs::write(filename, format!("{}", max))?;
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let mut cli: Cli = Cli::parse();

    if cli.color_range <= 0.0 {
        Cli::command()
            .error(ErrorKind::InvalidValue, "color range must be positive")
            .exit();
    }
    let mut timer = make_timer(&mut cli);

    let mut max = MaximalsStampsBuffer::new(cli.count, cli.lines_before);

    let mut buffer = String::new();
    let mut stdin = io::stdin().lock();
    while stdin.read_line(&mut buffer)? > 0 {
        if let Some(stamp) = timer.stamp(&buffer) {
            print_stamp(&cli, &stamp);
            max.insert(stamp, &buffer);
        };
        print!("{}", buffer);

        buffer.clear();
    }
    drop(stdin);

    print_maximals(&mut cli, &max)?;

    Ok(())
}
