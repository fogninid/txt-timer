use chrono::{DateTime, NaiveDateTime, Utc};
use regex::Regex;
use std::time::{Duration, Instant};

pub trait Timer {
    fn stamp(&mut self, line: &str) -> Option<Stamp>;
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
pub struct Stamp {
    pub last: Duration,
    pub total: Duration,
    pub utc: DateTime<Utc>
}

pub struct ChronoTimer {
    begin: Instant,
    last: Instant,
}

impl Timer for ChronoTimer {
    fn stamp(&mut self, _line: &str) -> Option<Stamp> {
        let now = Instant::now();
        let utc = Utc::now();
        let last = now.saturating_duration_since(self.last);
        let total = now.saturating_duration_since(self.begin);
        self.last = now;
        Some(Stamp { utc, last, total })
    }
}

impl ChronoTimer {
    pub fn new() -> Self {
        let now = Instant::now();
        ChronoTimer {
            begin: now,
            last: now,
        }
    }
}

pub struct RegexTimer {
    regex: Regex,
    fmt: String,
    last: Option<NaiveDateTime>,
    begin: Option<NaiveDateTime>,
}

impl Timer for RegexTimer {
    fn stamp(&mut self, line: &str) -> Option<Stamp> {
        let matched_time = self
            .regex
            .captures(line)
            .and_then(|m| m.name("time"))
            .and_then(|s| NaiveDateTime::parse_from_str(s.as_str(), self.fmt.as_str()).ok());

        match (matched_time, &self.begin, &self.last) {
            (Some(t), Some(begin), Some(last)) => {
                let last = t.signed_duration_since(*last).to_std().ok()?;
                let total = t.signed_duration_since(*begin).to_std().ok()?;
                let utc = t.and_utc();
                self.last = Some(t);
                Some(Stamp { utc, last, total })
            }
            (Some(t), None, _) => {
                self.begin = Some(t);
                self.last = Some(t);
                let utc = t.and_utc();
                Some(Stamp {
                    utc: utc,
                    last: Duration::ZERO,
                    total: Duration::ZERO,
                })
            }
            _ => None,
        }
    }
}

impl RegexTimer {
    pub fn new(regex: Regex, fmt: &str) -> RegexTimer {
        RegexTimer {
            regex,
            fmt: String::from(fmt),
            last: None,
            begin: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::timer::{RegexTimer, Timer};
    use regex::Regex;
    use std::time::Duration;

    #[test]
    fn time_parser() {
        let regex = Regex::new(r"(?P<time>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.?\d*)").unwrap();
        let mut regex_timer = RegexTimer::new(regex, "%Y-%m-%d %H:%M:%S%.3f");

        let op1 = regex_timer.stamp("test 2021-12-03 08:19:00.000 something");
        let op2 = regex_timer.stamp("test 2021-12-03 08:19:01.000 something");
        let op3 = regex_timer.stamp("test 2021-12-03 08:19:01.100 something");

        assert_eq!(op1.expect("failed to extract").last, Duration::ZERO);
        assert_eq!(op2.expect("failed to extract").last, Duration::from_secs(1));
        assert_eq!(
            op3.expect("failed to extract").last,
            Duration::from_millis(100)
        );
    }
}
