use crate::Error;
use serde::{Deserialize, Deserializer, Serializer};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic;
use std::time::Duration;

#[inline]
pub fn get_eva_dir() -> String {
    std::env::var("EVA_DIR").unwrap_or_else(|_| "/opt/eva4".to_owned())
}

#[inline]
pub fn atomic_true() -> atomic::AtomicBool {
    atomic::AtomicBool::new(true)
}

#[inline]
pub fn arc_atomic_true() -> Arc<atomic::AtomicBool> {
    Arc::new(atomic::AtomicBool::new(true))
}

#[derive(Debug)]
pub enum SocketPath {
    Tcp(String),
    Udp(String),
    Unix(String),
}

impl FromStr for SocketPath {
    type Err = Error;

    /// # Panics
    ///
    /// Will panic on internal errors
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if s.starts_with("tcp://") {
            SocketPath::Tcp(s.strip_prefix("tcp://").unwrap().to_owned())
        } else if s.starts_with("udp://") {
            SocketPath::Udp(s.strip_prefix("udp://").unwrap().to_owned())
        } else {
            SocketPath::Unix(s.to_owned())
        })
    }
}

/// # Panics
///
/// Will panic of neither path nor default specified
pub fn format_path(base: &str, path: Option<&str>, default: Option<&str>) -> String {
    if let Some(p) = path {
        if p.starts_with('/') {
            p.to_owned()
        } else {
            format!("{}/{}", base, p)
        }
    } else if let Some(d) = default {
        format!("{}/{}", base, d)
    } else {
        panic!("unable to format, neither path nor default specified");
    }
}

#[macro_export]
macro_rules! err_logger {
    () => {
        pub trait ErrLogger {
            /// log error and forget the result
            fn log_ef(self);
            /// log error as debug and forget the result
            fn log_efd(self);
            /// log error and keep the result
            fn log_err(self) -> Self;
            /// log error as debug and keep the result
            fn log_ed(self) -> Self;
            /// log error and forget the result with message
            fn log_ef_with(self, msg: impl ::std::fmt::Display);
            /// log error as debug and forget the result with message
            fn log_efd_with(self, msg: impl ::std::fmt::Display);
            /// log error and keep the result with message
            fn log_err_with(self, msg: impl ::std::fmt::Display) -> Self;
            /// log error as debug and keep the result with message
            fn log_ed_with(self, msg: impl ::std::fmt::Display) -> Self;
        }

        impl<R, E> ErrLogger for Result<R, E>
        where
            E: ::std::fmt::Display,
        {
            #[inline]
            fn log_ef(self) {
                if let Err(ref e) = self {
                    ::log::error!("{}", e);
                }
            }
            #[inline]
            fn log_efd(self) {
                if let Err(ref e) = self {
                    ::log::debug!("{}", e);
                }
            }
            #[inline]
            fn log_err(self) -> Self {
                if let Err(ref e) = self {
                    ::log::error!("{}", e);
                }
                self
            }
            #[inline]
            fn log_ed(self) -> Self {
                if let Err(ref e) = self {
                    ::log::debug!("{}", e);
                }
                self
            }
            #[inline]
            fn log_ef_with(self, msg: impl ::std::fmt::Display) {
                if let Err(ref e) = self {
                    ::log::error!("{}: {}", msg, e);
                }
            }
            #[inline]
            fn log_efd_with(self, msg: impl ::std::fmt::Display) {
                if let Err(ref e) = self {
                    ::log::debug!("{}: {}", msg, e);
                }
            }
            #[inline]
            fn log_err_with(self, msg: impl ::std::fmt::Display) -> Self {
                if let Err(ref e) = self {
                    ::log::error!("{}: {}", msg, e);
                }
                self
            }
            #[inline]
            fn log_ed_with(self, msg: impl ::std::fmt::Display) -> Self {
                if let Err(ref e) = self {
                    ::log::debug!("{}: {}", msg, e);
                }
                self
            }
        }
    };
}

// atomic functions (not implemented in serde for certain archs)
pub fn serialize_atomic_bool<S>(
    value: &atomic::AtomicBool,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bool(value.load(atomic::Ordering::SeqCst))
}

pub fn deserialize_atomic_bool<'de, D>(deserializer: D) -> Result<atomic::AtomicBool, D::Error>
where
    D: Deserializer<'de>,
{
    let val = bool::deserialize(deserializer)?;
    Ok(atomic::AtomicBool::new(val))
}

pub fn deserialize_arc_atomic_bool<'de, D>(
    deserializer: D,
) -> Result<Arc<atomic::AtomicBool>, D::Error>
where
    D: Deserializer<'de>,
{
    let val = bool::deserialize(deserializer)?;
    Ok(Arc::new(atomic::AtomicBool::new(val)))
}

pub fn serialize_atomic_u64<S>(value: &atomic::AtomicU64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(value.load(atomic::Ordering::SeqCst))
}

pub fn deserialize_atomic_u64<'de, D>(deserializer: D) -> Result<atomic::AtomicU64, D::Error>
where
    D: Deserializer<'de>,
{
    let val = u64::deserialize(deserializer)?;
    Ok(atomic::AtomicU64::new(val))
}

pub fn deserialize_arc_atomic_u64<'de, D>(
    deserializer: D,
) -> Result<Arc<atomic::AtomicU64>, D::Error>
where
    D: Deserializer<'de>,
{
    let val = u64::deserialize(deserializer)?;
    Ok(Arc::new(atomic::AtomicU64::new(val)))
}

pub fn serialize_duration_as_f64<S>(t: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_f64(t.as_secs_f64())
}

pub fn serialize_duration_as_u64<S>(t: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_u64(t.as_secs())
}

#[allow(clippy::cast_possible_truncation)]
pub fn serialize_duration_as_micros<S>(t: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_u64(t.as_micros() as u64)
}

#[allow(clippy::cast_possible_truncation)]
pub fn serialize_opt_duration_as_micros<S>(t: &Option<Duration>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(dur) = t {
        s.serialize_u64(dur.as_micros() as u64)
    } else {
        s.serialize_none()
    }
}

#[allow(clippy::cast_possible_truncation)]
pub fn serialize_duration_as_nanos<S>(t: &Duration, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_u64(t.as_nanos() as u64)
}

#[allow(clippy::cast_possible_truncation)]
pub fn serialize_opt_duration_as_nanos<S>(t: &Option<Duration>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(dur) = t {
        s.serialize_u64(dur.as_nanos() as u64)
    } else {
        s.serialize_none()
    }
}

pub fn serialize_opt_duration_as_f64<S>(t: &Option<Duration>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Some(dur) = t {
        s.serialize_f64(dur.as_secs_f64())
    } else {
        s.serialize_none()
    }
}

pub fn deserialize_duration_from_micros<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Duration::from_micros(u64::deserialize(deserializer)?))
}

pub fn deserialize_duration_from_nanos<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Duration::from_nanos(u64::deserialize(deserializer)?))
}

fn check_float_for_duration(t: f64) -> Result<(), String> {
    if t < 0.0 {
        return Err(format!("negative duration not allowed: {}", t));
    }
    if t.is_nan() {
        return Err(format!("nan duration not allowed: {}", t));
    }
    if t.is_infinite() {
        return Err(format!("infinite duration not allowed: {}", t));
    }
    Ok(())
}

pub fn de_float_as_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let t = f64::deserialize(deserializer)?;
    check_float_for_duration(t).map_err(serde::de::Error::custom)?;
    Ok(Duration::from_secs_f64(t))
}

pub fn de_opt_float_as_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let t: Option<f64> = Option::deserialize(deserializer)?;
    let Some(t) = t else {
        return Ok(None);
    };
    check_float_for_duration(t).map_err(serde::de::Error::custom)?;
    Ok(Some(Duration::from_secs_f64(t)))
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
pub fn de_float_as_duration_us<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Duration::from_nanos(
        (f64::deserialize(deserializer)? * 1000.0) as u64,
    ))
}

#[allow(clippy::cast_possible_truncation)]
#[allow(clippy::cast_sign_loss)]
pub fn de_opt_float_as_duration_us<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let t: Option<f64> = Option::deserialize(deserializer)?;
    Ok(t.map(|v| Duration::from_nanos((v * 1000.0) as u64)))
}

#[inline]
pub fn default_true() -> bool {
    true
}

#[inline]
pub fn is_true(b: &bool) -> bool {
    *b
}
