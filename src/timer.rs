pub mod timer {
    use std::time::{Duration, Instant};
    use dateparser::DateTimeUtc;
    use chrono::{DateTime, Utc};
    use regex::{Match, Regex};

    pub struct Timer {
        begin: Instant,
        last: Instant,
    }

    #[derive(Eq, PartialEq, Ord, PartialOrd)]
    pub struct Stamp {
        pub last: Duration,
        pub total: Duration,
    }

    impl Timer {
        pub fn stamp(&mut self) -> Stamp {
            let now = Instant::now();
            let last = now.saturating_duration_since(self.last);
            let total = now.saturating_duration_since(self.begin);
            self.last = now;
            Stamp { last, total }
        }

        pub fn new() -> Self {
            let now = Instant::now();
            Timer {
                begin: now,
                last: now,
            }
        }
    }

    pub struct RegexTimer {
        regex: Regex,
        last: Option<DateTime<Utc>>,
        begin: Option<DateTime<Utc>>,
    }

    impl RegexTimer {
        pub fn stamp(&mut self, line: &str) -> Option<Stamp> {
            self.regex
                .captures(line)
                .and_then(|m| m.name("time"))
                .and_then(|s| s.as_str().parse::<DateTimeUtc>().ok())
                .map(|x| x.0)
                .and_then(|t| {
                    match (&self.begin, &self.last) {
                        (None, _) => {
                            self.begin = Some(t);
                            self.last = Some(t);
                            Some(Stamp {
                                last: Duration::ZERO,
                                total: Duration::ZERO,
                            })
                        }
                        (Some(begin), Some(last)) => {
                            let last = t.signed_duration_since(*last).to_std().ok()?;
                            let total = t.signed_duration_since(*begin).to_std().ok()?;
                            self.last = Some(t);
                            Some(Stamp {
                                last,
                                total,
                            })
                        }
                        _ => None
                    }
                })
        }

        pub fn new(regex: Regex) -> Self {
            RegexTimer {
                regex,
                last: None,
                begin: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use regex::Regex;
    use crate::timer::timer::RegexTimer;

    #[test]
    fn time_parser() {
        let regex = Regex::new(r"(?P<time>\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}\.?\d*)").unwrap();
        let mut regex_timer = RegexTimer::new(regex);

        let _op1 = regex_timer.stamp("test 2021-12-03 08:19:00 something");
        let op2 = regex_timer.stamp("test 2021-12-03 08:19:01 something");
        let op3 = regex_timer.stamp("test 2021-12-03 08:19:01.1 something");

        assert_eq!(op2.expect("failed to extract").last, Duration::from_secs(1));
        assert_eq!(op3.expect("failed to extract").last, Duration::from_millis(100));
    }
}