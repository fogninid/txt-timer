mod maximals;
mod timer;

use crate::maximals::Maximals;
use crate::timer::{ChronoTimer, RegexTimer, Stamp, Timer};
use clap::{CommandFactory, Parser};
use clap::error::ErrorKind;
use colored::Colorize;
use itertools::Itertools;
use regex::Regex;
use std::collections::VecDeque;
use std::fmt::Formatter;
use std::io::BufRead;
use std::path::PathBuf;
use std::rc::Rc;
use std::{fmt, fs, io, thread, vec};
use std::sync::mpsc;
use std::sync::mpsc::{SyncSender};
use std::thread::JoinHandle;

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
    #[clap(short, long, value_parser)]
    output_maximals: Option<PathBuf>,
    /// buffer size for asynchronous processing
    #[clap(long, value_parser, default_value_t = 128)]
    async_buffer_line_count: usize,
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
        for e in self.max.iter() {
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
                "Δ{} @{} {}",
                format!("{:.4}", x).truecolor(r, g, 0),
                format!("{:.4}", stamp.total.as_secs_f32()).blue(),
                stamp.utc.to_rfc3339().bold().white()
            );
        } else {
            println!(
                "{} @ {} {}",
                stamp.last.as_secs_f32(),
                stamp.total.as_secs_f32(),
                stamp.utc.to_rfc3339()
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

trait Handler {
    fn process_line(&mut self, buffer: &str);

    fn print_and_end(self: Box<Self>) -> io::Result<()>;
}

fn make_handler(cli: Cli) -> Box<dyn Handler> {
    if cli.async_buffer_line_count > 0 {
        Box::new(ASyncHandler::new(cli))
    } else {
        Box::new(SyncHandler::new(cli))
    }
}

struct SyncHandler {
    timer: Box<dyn Timer>,
    max: MaximalsStampsBuffer,
    cli: Cli,
}

struct ASyncHandler {
    join_handle: Option<JoinHandle<()>>,
    tx: Option<SyncSender<String>>,
}

impl Handler for SyncHandler {
    fn process_line(&mut self, buffer: &str) {
        if let Some(stamp) = self.timer.stamp(buffer) {
            print_stamp(&self.cli, &stamp);
            self.max.insert(stamp, buffer);
        };
        if !self.cli.quiet {
            print!("{}", buffer);
        }
    }

    fn print_and_end(self: Box<Self>) -> io::Result<()> {
        let max = self.max;
        let cli = self.cli;
        match cli.output_maximals {
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
}

impl SyncHandler {
    fn new(mut cli: Cli) -> Self {
        let max = MaximalsStampsBuffer::new(cli.count, cli.lines_before);

        let timer = make_timer(&mut cli);

        SyncHandler { timer, max, cli }
    }
}

impl Handler for ASyncHandler {
    fn process_line(&mut self, buffer: &str) {
        self.tx.as_ref().unwrap().send(String::from(buffer)).unwrap();
    }

    fn print_and_end(mut self: Box<Self>) -> io::Result<()> {
        drop(self.tx.take());
        self.join_handle.take().unwrap().join().unwrap();
        Ok(())
    }
}

impl Drop for ASyncHandler {
    fn drop(&mut self) {
        drop(self.tx.take());
        if let Some(join_handle) = self.join_handle.take() {
            match join_handle.join() {
                Ok(_) => {}
                Err(e) => println!("Can't join threads: {:?}", e),
            }
        }
    }
}

impl ASyncHandler {
    fn new(cli: Cli) -> Self {
        let (tx, rx) = mpsc::sync_channel::<String>(cli.async_buffer_line_count);

        let join_handle = thread::spawn(move || {
            let mut sync_handler = SyncHandler::new(cli);
            while let Ok(buffer) = rx.recv() {
                sync_handler.process_line(&buffer);
            }
            Box::new(sync_handler).print_and_end().expect("failed to print");
        });

        ASyncHandler { join_handle: Some(join_handle), tx: Some(tx) }
    }
}

fn main() -> io::Result<()> {
    let cli: Cli = Cli::parse_and_validate();

    let mut handler: Box<dyn Handler> = make_handler(cli);

    let mut buffer = String::new();
    let mut stdin = io::stdin().lock();
    while stdin.read_line(&mut buffer)? > 0 {
        handler.process_line(&buffer);
        buffer.clear();
    }
    drop(stdin);

    handler.print_and_end()?;

    Ok(())
}
