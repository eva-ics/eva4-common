use crate::{EResult, Error, Value};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::time::Instant;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn serialize_time_now<S>(_value: &(), serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(Time::now().into())
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Time {
    sec: u64,
    nsec: u64,
}

#[allow(clippy::module_name_repetitions)]
pub fn deserialize_time<'de, D>(deserializer: D) -> Result<Time, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Time::from_timestamp(f64::deserialize(deserializer)?))
}

pub fn serialize_uptime<S>(value: &Instant, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(value.elapsed().as_secs_f64())
}

impl Serialize for Time {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.timestamp())
    }
}

impl Time {
    #[inline]
    #[allow(clippy::similar_names)]
    pub fn new(sec: u64, nsec: u64) -> Self {
        Self { sec, nsec }
    }
    /// # Panics
    ///
    /// Will panic if the system clock is not available
    #[inline]
    #[allow(clippy::cast_sign_loss)]
    pub fn now() -> Self {
        let t = nix::time::clock_gettime(nix::time::ClockId::CLOCK_REALTIME).unwrap();
        Self {
            sec: t.tv_sec() as u64,
            nsec: t.tv_nsec() as u64,
        }
    }
    #[inline]
    pub fn from_timestamp_ns(timestamp_ns: u64) -> Self {
        Self {
            sec: timestamp_ns / 1_000_000_000,
            nsec: timestamp_ns % 1_000_000_000,
        }
    }
    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    #[inline]
    pub fn from_timestamp(timestamp: f64) -> Self {
        Self {
            sec: timestamp.trunc() as u64,
            nsec: (timestamp.fract() * 1_000_000_000_f64) as u64,
        }
    }
    #[allow(clippy::cast_precision_loss)]
    #[inline]
    pub fn timestamp(&self) -> f64 {
        self.sec as f64 + self.nsec as f64 / 1_000_000_000.0
    }
    #[inline]
    pub fn timestamp_ns(&self) -> u64 {
        self.sec * 1_000_000_000 + self.nsec
    }
}

impl From<Time> for Value {
    #[inline]
    fn from(t: Time) -> Value {
        Value::F64(t.timestamp())
    }
}

impl From<Time> for f64 {
    #[inline]
    fn from(t: Time) -> f64 {
        t.timestamp()
    }
}

impl TryFrom<SystemTime> for Time {
    type Error = Error;
    #[inline]
    fn try_from(t: SystemTime) -> EResult<Self> {
        Ok(Time::from_timestamp(
            t.duration_since(UNIX_EPOCH)
                .map_err(|_| Error::core("systime before UNIX EPOCH"))?
                .as_secs_f64(),
        ))
    }
}

/// # Panics
///
/// Will panic if the monotonic clock is not available
#[allow(clippy::cast_sign_loss)]
pub fn monotonic() -> u64 {
    nix::time::clock_gettime(nix::time::ClockId::CLOCK_MONOTONIC)
        .unwrap()
        .tv_sec() as u64
}

/// # Panics
///
/// Will panic if the monotonic clock is not available
#[allow(clippy::cast_sign_loss)]
pub fn monotonic_ns() -> u64 {
    let t = nix::time::clock_gettime(nix::time::ClockId::CLOCK_MONOTONIC).unwrap();
    t.tv_sec() as u64 * 1_000_000_000 + t.tv_nsec() as u64
}

/// # Panics
///
/// Will panic if the system clock is not available
#[allow(clippy::cast_sign_loss)]
pub fn now() -> u64 {
    nix::time::clock_gettime(nix::time::ClockId::CLOCK_REALTIME)
        .unwrap()
        .tv_sec() as u64
}

/// # Panics
///
/// Will panic if the system clock is not available
#[allow(clippy::cast_precision_loss)]
#[inline]
pub fn now_ns_float() -> f64 {
    Time::now().timestamp()
}

/// # Panics
///
/// Will panic if the system clock is not available
#[allow(clippy::cast_sign_loss)]
pub fn now_ns() -> u64 {
    let t = nix::time::clock_gettime(nix::time::ClockId::CLOCK_REALTIME).unwrap();
    t.tv_sec() as u64 * 1_000_000_000 + t.tv_nsec() as u64
}

/// Convert f64 timestamp to nanoseconds
#[inline]
pub fn ts_to_ns(ts: f64) -> u64 {
    let t = Time::from_timestamp(ts);
    t.timestamp_ns()
}

/// Convert nanoseconds to f64 timestamp
#[inline]
pub fn ts_from_ns(ts: u64) -> f64 {
    let t = Time::from_timestamp_ns(ts);
    t.timestamp()
}

#[cfg(test)]
mod tests {
    use super::Time;
    #[test]
    fn test_time() {
        let timestamp = 1632093707.1893349;
        let time = Time::from_timestamp(timestamp);
        assert_eq!(time.timestamp(), timestamp);
        assert_eq!(time.timestamp_ns(), 1632093707189334869);
        let timestamp_ns = 1632093707123456789;
        let time = Time::from_timestamp_ns(timestamp_ns);
        assert_eq!(time.timestamp_ns(), timestamp_ns);
        assert_eq!(time.timestamp(), 1632093707.123456789);
    }
}
