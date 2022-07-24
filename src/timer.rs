pub mod timer {
    use std::time::{Duration, Instant};

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
}
