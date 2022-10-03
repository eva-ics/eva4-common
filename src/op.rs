use crate::{EResult, Error};
use std::time::Duration;
use std::time::Instant;

pub struct Op {
    t: Instant,
    timeout: Duration,
}

impl Op {
    #[inline]
    pub fn new(timeout: Duration) -> Self {
        Self {
            t: Instant::now(),
            timeout,
        }
    }
    #[inline]
    pub fn for_instant(t: Instant, timeout: Duration) -> Self {
        Self { t, timeout }
    }
    pub fn is_timed_out(&self) -> bool {
        let el = self.t.elapsed();
        el > self.timeout
    }
    pub fn timeout(&self) -> EResult<Duration> {
        let el = self.t.elapsed();
        if el > self.timeout {
            Err(Error::timeout())
        } else {
            Ok(self.timeout - el)
        }
    }
    #[inline]
    pub fn is_enough(&self, expected: Duration) -> bool {
        self.t.elapsed() + expected < self.timeout
    }
    #[inline]
    pub fn enough(&self, expected: Duration) -> EResult<()> {
        if self.is_enough(expected) {
            Ok(())
        } else {
            Err(Error::timeout())
        }
    }
    #[inline]
    pub fn remaining(&self, timeout: Duration) -> EResult<Duration> {
        let el = self.t.elapsed();
        if el > timeout {
            Err(Error::timeout())
        } else {
            Ok(timeout - el)
        }
    }
}
