use std::{cell::RefCell, time::Instant};

pub(super) struct StartLimit {
    hit: RefCell<bool>,
    start_limit: RefCell<RateLimit>,
}

#[derive(PartialEq, Eq)]
pub(crate) enum StartLimitResult {
    StartLimitNotHit,
    StartLimitHit,
}

impl StartLimit {
    pub(super) fn new() -> Self {
        StartLimit {
            hit: RefCell::new(false),
            start_limit: RefCell::new(RateLimit::new(10, 5)),
        }
    }

    pub(super) fn set_hit(&self, hit: bool) {
        *self.hit.borrow_mut() = hit
    }

    #[allow(dead_code)]
    pub(super) fn hit(&self) -> bool {
        *self.hit.borrow_mut()
    }

    pub(super) fn ratelimit_below(&self) -> bool {
        self.start_limit.borrow_mut().ratelimit_below()
    }

    pub(super) fn init_from_config(&self, interval: u64, burst: u32) {
        self.start_limit
            .borrow_mut()
            .init_from_config(interval, burst);
    }
}

struct RateLimit {
    interval: u64,
    burst: u32,
    begin: Option<Instant>,
    nums: u32,
}

impl RateLimit {
    fn new(interval: u64, burst: u32) -> Self {
        RateLimit {
            interval,
            burst,
            begin: None,
            nums: 0,
        }
    }

    fn ratelimit_below(&mut self) -> bool {
        if !self.ratelimit_enabled() {
            return true;
        }

        let now = Instant::now();
        if self.begin.is_none() || now.duration_since(self.begin.unwrap()).as_secs() > self.interval
        {
            self.begin = Some(now);
            self.nums = 1;
            return true;
        }

        if self.nums < self.burst {
            self.nums += 1;
            return true;
        }

        false
    }

    fn ratelimit_enabled(&self) -> bool {
        if self.interval > 0 && self.burst > 0 {
            return true;
        }

        false
    }

    pub(super) fn init_from_config(&mut self, interval: u64, burst: u32) {
        self.interval = interval;
        self.burst = burst;
    }
}

#[cfg(test)]
mod tests {
    use super::RateLimit;
    #[test]
    fn test_ratelimit() {
        let mut tmp = RateLimit::new(0, 0);
        assert!(tmp.ratelimit_below());

        let mut tmp2 = RateLimit::new(3, 2);
        assert!(tmp2.ratelimit_below());
        assert!(tmp2.ratelimit_below());
        assert!(!tmp2.ratelimit_below());
    }
}
