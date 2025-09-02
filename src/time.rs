use crate::{EResult, Error, Value};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
static STARTED_AT: once_cell::sync::Lazy<Instant> = once_cell::sync::Lazy::new(|| Instant::now());

pub fn serialize_time_now<S>(_value: &(), serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(Time::now().into())
}

/// Time
///
/// Serialized as f64
/// Deserialized from unsigned integers (seconds), floats, [sec, nsec] seqs
///
/// With "db" feature provides sqlx interfaces for Sqlite (stored as nanoseconds integer) and
/// Postgres (stored as TIMESTAMP/TIMESTAMPTZ)
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Time {
    sec: u64,
    nsec: u64,
}

impl FromStr for Time {
    type Err = Error;
    fn from_str(s: &str) -> EResult<Self> {
        if let Ok(v) = s.parse::<f64>() {
            Ok(v.into())
        } else {
            Ok(dateparser::parse(s).map_err(Error::invalid_data)?.into())
        }
    }
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

struct TimeVisitor;

impl<'de> serde::de::Visitor<'de> for TimeVisitor {
    type Value = Time;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string, float, an unsigned integer, or a 2-element array")
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Time {
            sec: value,
            nsec: 0,
        })
    }

    fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(value.into())
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(value.into())
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        let s: u64 = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let ns: u64 = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        Ok(Time { sec: s, nsec: ns })
    }
    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        value
            .parse()
            .map_err(|_| serde::de::Error::custom("invalid time string"))
    }
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        value
            .parse()
            .map_err(|_| serde::de::Error::custom("invalid time string"))
    }
}

impl<'de> Deserialize<'de> for Time {
    fn deserialize<D>(deserializer: D) -> Result<Time, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(TimeVisitor)
    }
}

impl Default for Time {
    #[inline]
    fn default() -> Self {
        Self::now()
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
    /// Will panic if the system real-time clock is not available
    /// Will panic on Windows if the clock is set before 1.1.1970
    #[allow(clippy::cast_sign_loss)]
    #[cfg(target_os = "linux")]
    #[inline]
    pub fn now() -> Self {
        let t = nix::time::clock_gettime(nix::time::ClockId::CLOCK_REALTIME).unwrap();
        Self {
            sec: t.tv_sec() as u64,
            nsec: t.tv_nsec() as u64,
        }
    }
    #[cfg(target_os = "windows")]
    #[inline]
    pub fn now() -> Self {
        let t = SystemTime::now();
        t.try_into().unwrap()
    }
    /// On Windows returns time since the first access
    ///
    /// # Panics
    ///
    /// Will panic if the system monotonic clock is not available
    #[inline]
    #[allow(clippy::cast_sign_loss)]
    #[cfg(target_os = "linux")]
    pub fn now_monotonic() -> Self {
        let t = nix::time::clock_gettime(nix::time::ClockId::CLOCK_MONOTONIC).unwrap();
        Self {
            sec: t.tv_sec() as u64,
            nsec: t.tv_nsec() as u64,
        }
    }
    #[cfg(target_os = "windows")]
    #[inline]
    pub fn now_monotonic() -> Self {
        STARTED_AT.elapsed().into()
    }
    #[inline]
    pub fn from_timestamp_ns(timestamp_ns: u64) -> Self {
        Self {
            sec: timestamp_ns / 1_000_000_000,
            nsec: timestamp_ns % 1_000_000_000,
        }
    }
    #[inline]
    pub fn from_timestamp_us(timestamp_us: u64) -> Self {
        Self {
            sec: timestamp_us / 1_000_000,
            nsec: timestamp_us % 1_000_000 * 1_000,
        }
    }
    #[inline]
    pub fn from_timestamp_ms(timestamp_ms: u64) -> Self {
        Self {
            sec: timestamp_ms / 1_000,
            nsec: timestamp_ms % 1_000 * 1_000_000,
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
    pub fn timestamp_sec(&self) -> u64 {
        self.sec
    }
    #[inline]
    pub fn timestamp_ns(&self) -> u64 {
        self.sec * 1_000_000_000 + self.nsec
    }
    #[inline]
    pub fn timestamp_us(&self) -> u64 {
        self.sec * 1_000_000 + self.nsec / 1_000
    }
    #[inline]
    pub fn timestamp_ms(&self) -> u64 {
        self.sec * 1_000 + self.nsec / 1_000_000
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

impl From<f64> for Time {
    #[inline]
    fn from(v: f64) -> Time {
        Time::from_timestamp(v)
    }
}

impl From<f32> for Time {
    #[inline]
    fn from(v: f32) -> Time {
        Time::from_timestamp(v.into())
    }
}

impl TryFrom<SystemTime> for Time {
    type Error = Error;
    #[inline]
    fn try_from(t: SystemTime) -> EResult<Self> {
        Ok(t.duration_since(UNIX_EPOCH)
            .map_err(|_| Error::core("systime before UNIX EPOCH"))?
            .into())
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.timestamp())
    }
}

impl From<Duration> for Time {
    fn from(v: Duration) -> Self {
        Self {
            sec: v.as_secs(),
            nsec: u64::from(v.subsec_nanos()),
        }
    }
}

/// # Panics
///
/// Will panic if duration in nanoseconds > u64::MAX
impl core::ops::Add<Duration> for Time {
    type Output = Time;
    fn add(self, dur: Duration) -> Time {
        let t_ns = self.timestamp_ns() + u64::try_from(dur.as_nanos()).unwrap();
        Time::from_timestamp_ns(t_ns)
    }
}

impl core::ops::Add<f64> for Time {
    type Output = Time;
    fn add(self, value: f64) -> Time {
        Time::from_timestamp(self.timestamp() + value)
    }
}

impl core::ops::Sub<f64> for Time {
    type Output = Time;
    fn sub(self, value: f64) -> Time {
        Time::from_timestamp(self.timestamp() - value)
    }
}

impl core::ops::Add<u64> for Time {
    type Output = Time;
    fn add(self, value: u64) -> Time {
        Time {
            sec: self.sec + value,
            nsec: self.nsec,
        }
    }
}

impl core::ops::Sub<u64> for Time {
    type Output = Time;
    fn sub(self, value: u64) -> Time {
        Time {
            sec: self.sec - value,
            nsec: self.nsec,
        }
    }
}

impl core::ops::Add<u32> for Time {
    type Output = Time;
    fn add(self, value: u32) -> Time {
        Time {
            sec: self.sec + u64::from(value),
            nsec: self.nsec,
        }
    }
}

impl core::ops::Sub<u32> for Time {
    type Output = Time;
    fn sub(self, value: u32) -> Time {
        Time {
            sec: self.sec - u64::from(value),
            nsec: self.nsec,
        }
    }
}

/// # Panics
///
/// Will panic if duration in nanoseconds > u64::MAX
impl core::ops::Sub<Duration> for Time {
    type Output = Time;
    fn sub(self, dur: Duration) -> Time {
        let t_ns = self.timestamp_ns() - u64::try_from(dur.as_nanos()).unwrap();
        Time::from_timestamp_ns(t_ns)
    }
}

mod convert_chrono {
    use super::Time;
    use crate::{EResult, Error};
    use chrono::{DateTime, Local, NaiveDateTime, Utc};

    impl TryFrom<Time> for NaiveDateTime {
        type Error = Error;
        #[inline]
        fn try_from(t: Time) -> EResult<Self> {
            let dt = DateTime::from_timestamp(i64::try_from(t.sec)?, u32::try_from(t.nsec)?)
                .ok_or_else(|| Error::invalid_data("unable to convert timestamp"))?;
            Ok(dt.naive_local())
        }
    }
    impl TryFrom<Time> for DateTime<Utc> {
        type Error = Error;
        fn try_from(t: Time) -> EResult<Self> {
            let nt = NaiveDateTime::try_from(t)?;
            let dt_utc = DateTime::<Utc>::from_naive_utc_and_offset(nt, Utc);
            Ok(dt_utc)
        }
    }
    impl TryFrom<Time> for DateTime<Local> {
        type Error = Error;
        fn try_from(t: Time) -> EResult<Self> {
            let dt_utc = DateTime::<Utc>::try_from(t)?;
            Ok(DateTime::from(dt_utc))
        }
    }

    impl From<NaiveDateTime> for Time {
        #[allow(deprecated)]
        fn from(datetime: NaiveDateTime) -> Self {
            Time {
                sec: u64::try_from(datetime.and_utc().timestamp()).unwrap_or_default(),
                nsec: u64::from(datetime.timestamp_subsec_nanos()),
            }
        }
    }

    impl From<DateTime<Utc>> for Time {
        fn from(datetime: DateTime<Utc>) -> Self {
            Time {
                sec: u64::try_from(datetime.timestamp()).unwrap_or_default(),
                nsec: u64::from(datetime.timestamp_subsec_nanos()),
            }
        }
    }

    impl From<DateTime<Local>> for Time {
        fn from(datetime: DateTime<Local>) -> Self {
            Time {
                sec: u64::try_from(datetime.timestamp()).unwrap_or_default(),
                nsec: u64::from(datetime.timestamp_subsec_nanos()),
            }
        }
    }

    impl Time {
        #[inline]
        pub fn try_into_naivedatetime(self) -> EResult<NaiveDateTime> {
            self.try_into()
        }
        #[inline]
        pub fn try_into_datetime_local(self) -> EResult<DateTime<Local>> {
            self.try_into()
        }
        #[inline]
        pub fn try_into_datetime_utc(self) -> EResult<DateTime<Utc>> {
            self.try_into()
        }
    }
}

/// Get monotonic time in seconds
///
/// # Panics
///
/// Will panic if the monotonic clock is not available
#[inline]
pub fn monotonic() -> u64 {
    Time::now_monotonic().timestamp_sec()
}

/// Get monotonic time in nanoseconds
///
/// # Panics
///
/// Will panic if the monotonic clock is not available
#[inline]
pub fn monotonic_ns() -> u64 {
    Time::now_monotonic().timestamp_ns()
}

/// Get current UNIX timestamp in seconds
///
/// # Panics
///
/// Will panic if the system clock is not available
#[allow(clippy::cast_sign_loss)]
pub fn now() -> u64 {
    Time::now().timestamp_sec()
}

/// Get current UNIX timestamp in seconds as a float
///
/// # Panics
///
/// Will panic if the system clock is not available
#[inline]
pub fn now_ns_float() -> f64 {
    Time::now().timestamp()
}

/// Get current UNIX timestamp in nanoseconds
///
/// # Panics
///
/// Will panic if the system clock is not available
pub fn now_ns() -> u64 {
    Time::now().timestamp_ns()
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
#[allow(clippy::float_cmp)]
mod tests {
    use super::Time;
    #[test]
    fn test_time() {
        let timestamp = 1_632_093_707.189_334_9;
        let time = Time::from_timestamp(timestamp);
        assert_eq!(time.timestamp(), timestamp);
        assert_eq!(time.timestamp_ns(), 1_632_093_707_189_334_869);
        let timestamp_nanos = 1_632_093_707_123_456_789;
        let time = Time::from_timestamp_ns(timestamp_nanos);
        assert_eq!(time.timestamp_ns(), timestamp_nanos);
        assert_eq!(time.timestamp(), 1_632_093_707.123_456_7);
        assert_eq!(time.timestamp_ms(), timestamp_nanos / 1_000_000);
        assert_eq!(time.timestamp_us(), timestamp_nanos / 1_000);
        let timestamp_micros = 1_632_093_707_123_456;
        let time = Time::from_timestamp_us(timestamp_micros);
        assert_eq!(time.timestamp(), 1_632_093_707.123_456);
        assert_eq!(time.timestamp_ms(), timestamp_micros / 1_000);
        assert_eq!(time.timestamp_us(), timestamp_micros);
        assert_eq!(time.timestamp_ns(), timestamp_micros * 1_000);
        let timestamp_millis = 1_632_093_707_123;
        let time = Time::from_timestamp_ms(timestamp_millis);
        assert_eq!(time.timestamp(), 1_632_093_707.123);
        assert_eq!(time.timestamp_ms(), timestamp_millis);
        assert_eq!(time.timestamp_us(), timestamp_millis * 1_000);
        assert_eq!(time.timestamp_ns(), timestamp_millis * 1_000_000);
    }
}
