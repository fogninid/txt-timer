mod maximals;
mod timer;

use crate::maximals::maximals::Maximals;
use crate::timer::timer::{Stamp, Timer};
use clap::{CommandFactory, ErrorKind, Parser};
use colored::Colorize;
use regex::Regex;
use std::fmt::Formatter;
use std::io::BufRead;
use std::path::PathBuf;
use std::{fmt, fs, io, mem};
use itertools::Itertools;

#[derive(Parser)]
/// Pipe through standard input while highlighting and keeping track of delays between lines.
///
/// When completed print summary of maximum delays
struct Cli {
    /// number of top differences to print at the end
    #[clap(short, long, value_parser, default_value_t = 5)]
    count: usize,
    /// colorized output
    #[clap(long, value_parser, default_value_t = false)]
    color: bool,
    /// range for color scale of delay, in seconds
    #[clap(long, value_parser, default_value_t = 0.2)]
    color_range: f32,
    /// prepend time to output
    #[clap(short, long, value_parser)]
    time_regex: Option<Regex>,
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
    line: String,
    previous_line: Option<String>,
}

impl fmt::Display for MaximalsStampsEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Δ{:.4} @{:.4}",
            self.stamp.last.as_secs_f32(),
            self.stamp.total.as_secs_f32()
        )?;

        match &self.previous_line {
            Some(s) => {
                write!(f, "{}", s)?;
            }
            _ => {}
        }
        write!(f, "{}", self.line)
    }
}

struct MaximalsStampsBuffer {
    max: Maximals<MaximalsStampsEntry>,
    previous_line: Option<String>,
}

impl MaximalsStampsBuffer {
    fn new(count: usize) -> Self {
        MaximalsStampsBuffer {
            max: Maximals::new(count),
            previous_line: None,
        }
    }

    fn insert(&mut self, stamp: Stamp, value: &String) {
        let mut previous_line = Some(value.clone());
        mem::swap(&mut self.previous_line, &mut previous_line);
        let line = value.clone();
        self.max.insert(MaximalsStampsEntry {
            stamp,
            line,
            previous_line,
        });
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

fn main() -> io::Result<()> {
    let cli: Cli = Cli::parse();

    if cli.color_range <= 0.0 {
        Cli::command()
            .error(ErrorKind::InvalidValue, "color range must be positive")
            .exit();
    }
    match cli.time_regex {
        Some(r) => {
            if !r.capture_names().contains(&Some("time")) {
                Cli::command()
                    .error(ErrorKind::InvalidValue, "regex must have a `(?P<time>exp)` capturing group")
                    .exit();
            }
        }
        _ => {}
    }

    let mut max: MaximalsStampsBuffer = MaximalsStampsBuffer::new(cli.count);

    {
        let mut timer = Timer::new();

        let mut buffer = String::new();
        let mut stdin = io::stdin().lock();
        while stdin.read_line(&mut buffer)? > 0 {
            let stamp = timer.stamp();

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
            print!("{}", buffer);

            max.insert(stamp, &buffer);

            buffer.clear();
        }
    }

    match cli.output_maximals {
        None => {
            if cli.color {
                println!("\n{}:\n{}", "Maximals".yellow().bold(), max);
            } else {
                println!("\n{}:\n{}", "Maximals", max);
            }
        }
        Some(filename) => {
            fs::write(filename, format!("{}", max))?;
        }
    }

    Ok(())
}
