//! Based on https://github.com/arcnmx/serde-value

use crate::{EResult, Error};
use ordered_float::OrderedFloat;
use rust_decimal::prelude::*;
use serde::{Deserialize, Deserializer, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::convert::AsRef;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::hash::{BuildHasher, Hash, Hasher};
use std::iter::FromIterator;
#[cfg(feature = "extended-value")]
use std::path::Path;
#[cfg(feature = "extended-value")]
use std::time::Duration;

pub use de::*;
pub use ser::*;

//pub use ser::SerializerError;
//pub use de::DeserializerError;

mod de;
mod ser;

impl From<de::DeserializerError> for Error {
    fn from(err: de::DeserializerError) -> Error {
        Error::invalid_data(err)
    }
}

impl From<ser::SerializerError> for Error {
    fn from(err: ser::SerializerError) -> Error {
        Error::invalid_data(err)
    }
}

const ERR_INVALID_VALUE: &str = "Invalid value";

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum ValueOptionOwned {
    No,
    Value(Value),
}

impl Default for ValueOptionOwned {
    fn default() -> Self {
        ValueOptionOwned::No
    }
}

impl ValueOptionOwned {
    pub fn is_none(&self) -> bool {
        matches!(self, ValueOptionOwned::No)
    }

    pub fn is_some(&self) -> bool {
        !matches!(self, ValueOptionOwned::No)
    }

    pub fn as_ref(&self) -> Option<&Value> {
        match self {
            ValueOptionOwned::No => None,
            ValueOptionOwned::Value(ref v) => Some(v),
        }
    }
}

impl From<ValueOptionOwned> for Option<Value> {
    fn from(vo: ValueOptionOwned) -> Self {
        match vo {
            ValueOptionOwned::No => None,
            ValueOptionOwned::Value(v) => Some(v),
        }
    }
}

impl From<Option<Value>> for ValueOptionOwned {
    fn from(v: Option<Value>) -> Self {
        if let Some(val) = v {
            ValueOptionOwned::Value(val)
        } else {
            ValueOptionOwned::No
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
#[serde(untagged)]
pub enum ValueOption<'a> {
    No,
    Value(&'a Value),
}

impl<'a> Default for ValueOption<'a> {
    fn default() -> Self {
        ValueOption::No
    }
}

impl<'a> ValueOption<'a> {
    pub fn is_none(&self) -> bool {
        matches!(self, ValueOption::No)
    }

    pub fn is_some(&self) -> bool {
        !matches!(self, ValueOption::No)
    }
}

impl<'de> Deserialize<'de> for ValueOptionOwned {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(ValueOptionOwned::Value(Value::deserialize(deserializer)?))
    }
}

#[cfg(feature = "time")]
#[inline]
fn parse_time_frame(s: &str) -> Option<f64> {
    if s.len() < 2 {
        None
    } else if let Ok(v) = s[..s.len() - 1].parse::<f64>() {
        match &s[s.len() - 1..] {
            "S" => Some(v),
            "T" => Some(v * 60.0),
            "H" => Some(v * 3_600.0),
            "D" => Some(v * 86_400.0),
            "W" => Some(v * 604_800.0),
            _ => None,
        }
    } else {
        None
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Bool(bool),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),

    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),

    F32(f32),
    F64(f64),

    Char(char),
    String(String),

    Unit,
    Option(Option<Box<Value>>),
    Newtype(Box<Value>),
    Seq(Vec<Value>),
    Map(BTreeMap<Value, Value>),
    Bytes(Vec<u8>),
}

impl Default for Value {
    fn default() -> Self {
        Value::Unit
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Bool(v) => write!(f, "{}", v),
            Value::U8(v) => write!(f, "{}", v),
            Value::U16(v) => write!(f, "{}", v),
            Value::U32(v) => write!(f, "{}", v),
            Value::U64(v) => write!(f, "{}", v),
            Value::I8(v) => write!(f, "{}", v),
            Value::I16(v) => write!(f, "{}", v),
            Value::I32(v) => write!(f, "{}", v),
            Value::I64(v) => write!(f, "{}", v),
            Value::F32(v) => write!(f, "{}", v),
            Value::F64(v) => write!(f, "{}", v),
            Value::Char(v) => write!(f, "{}", v),
            Value::String(ref v) => write!(f, "{}", v),
            Value::Unit => write!(f, ""),
            Value::Option(ref v) => {
                if let Some(val) = v {
                    write!(f, "{}", val)
                } else {
                    write!(f, "")
                }
            }
            Value::Newtype(ref v) => write!(f, "{}", v),
            Value::Seq(ref v) => write!(f, "{:?}", v),
            Value::Map(ref v) => write!(f, "{:?}", v),
            Value::Bytes(ref v) => write!(f, "{:?}", v),
        }
    }
}

impl Hash for Value {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.discriminant().hash(hasher);
        match *self {
            Value::Bool(v) => v.hash(hasher),
            Value::U8(v) => v.hash(hasher),
            Value::U16(v) => v.hash(hasher),
            Value::U32(v) => v.hash(hasher),
            Value::U64(v) => v.hash(hasher),
            Value::I8(v) => v.hash(hasher),
            Value::I16(v) => v.hash(hasher),
            Value::I32(v) => v.hash(hasher),
            Value::I64(v) => v.hash(hasher),
            Value::F32(v) => OrderedFloat(v).hash(hasher),
            Value::F64(v) => OrderedFloat(v).hash(hasher),
            Value::Char(v) => v.hash(hasher),
            Value::String(ref v) => v.hash(hasher),
            Value::Unit => 0_u8.hash(hasher),
            Value::Option(ref v) => v.hash(hasher),
            Value::Newtype(ref v) => v.hash(hasher),
            Value::Seq(ref v) => v.hash(hasher),
            Value::Map(ref v) => v.hash(hasher),
            Value::Bytes(ref v) => v.hash(hasher),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (&Value::Bool(v0), &Value::Bool(v1)) if v0 == v1 => true,
            (&Value::U8(v0), &Value::U8(v1)) if v0 == v1 => true,
            (&Value::U16(v0), &Value::U16(v1)) if v0 == v1 => true,
            (&Value::U32(v0), &Value::U32(v1)) if v0 == v1 => true,
            (&Value::U64(v0), &Value::U64(v1)) if v0 == v1 => true,
            (&Value::I8(v0), &Value::I8(v1)) if v0 == v1 => true,
            (&Value::I16(v0), &Value::I16(v1)) if v0 == v1 => true,
            (&Value::I32(v0), &Value::I32(v1)) if v0 == v1 => true,
            (&Value::I64(v0), &Value::I64(v1)) if v0 == v1 => true,
            (&Value::F32(v0), &Value::F32(v1)) if OrderedFloat(v0) == OrderedFloat(v1) => true,
            (&Value::F64(v0), &Value::F64(v1)) if OrderedFloat(v0) == OrderedFloat(v1) => true,
            (&Value::Char(v0), &Value::Char(v1)) if v0 == v1 => true,
            (&Value::String(ref v0), &Value::String(ref v1)) if v0 == v1 => true,
            (&Value::Unit, &Value::Unit) => true,
            (&Value::Option(ref v0), &Value::Option(ref v1)) if v0 == v1 => true,
            (&Value::Newtype(ref v0), &Value::Newtype(ref v1)) if v0 == v1 => true,
            (&Value::Seq(ref v0), &Value::Seq(ref v1)) if v0 == v1 => true,
            (&Value::Map(ref v0), &Value::Map(ref v1)) if v0 == v1 => true,
            (&Value::Bytes(ref v0), &Value::Bytes(ref v1)) if v0 == v1 => true,
            _ => false,
        }
    }
}

impl Ord for Value {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match (self, rhs) {
            (&Value::Bool(v0), &Value::Bool(ref v1)) => v0.cmp(v1),
            (&Value::U8(v0), &Value::U8(ref v1)) => v0.cmp(v1),
            (&Value::U16(v0), &Value::U16(ref v1)) => v0.cmp(v1),
            (&Value::U32(v0), &Value::U32(ref v1)) => v0.cmp(v1),
            (&Value::U64(v0), &Value::U64(ref v1)) => v0.cmp(v1),
            (&Value::I8(v0), &Value::I8(ref v1)) => v0.cmp(v1),
            (&Value::I16(v0), &Value::I16(ref v1)) => v0.cmp(v1),
            (&Value::I32(v0), &Value::I32(ref v1)) => v0.cmp(v1),
            (&Value::I64(v0), &Value::I64(ref v1)) => v0.cmp(v1),
            (&Value::F32(v0), &Value::F32(v1)) => OrderedFloat(v0).cmp(&OrderedFloat(v1)),
            (&Value::F64(v0), &Value::F64(v1)) => OrderedFloat(v0).cmp(&OrderedFloat(v1)),
            (&Value::Char(v0), &Value::Char(ref v1)) => v0.cmp(v1),
            (&Value::String(ref v0), &Value::String(ref v1)) => v0.cmp(v1),
            (&Value::Unit, &Value::Unit) => Ordering::Equal,
            (&Value::Option(ref v0), &Value::Option(ref v1)) => v0.cmp(v1),
            (&Value::Newtype(ref v0), &Value::Newtype(ref v1)) => v0.cmp(v1),
            (&Value::Seq(ref v0), &Value::Seq(ref v1)) => v0.cmp(v1),
            (&Value::Map(ref v0), &Value::Map(ref v1)) => v0.cmp(v1),
            (&Value::Bytes(ref v0), &Value::Bytes(ref v1)) => v0.cmp(v1),
            (v0, v1) => v0.discriminant().cmp(&v1.discriminant()),
        }
    }
}

fn strip_bytes_rec(value: Value) -> Value {
    if let Value::Bytes(_) = value {
        Value::String("<binary>".to_owned())
    } else if let Value::Seq(s) = value {
        let v: Vec<Value> = s.into_iter().map(strip_bytes_rec).collect();
        Value::Seq(v)
    } else if let Value::Map(m) = value {
        let mut result = BTreeMap::new();
        for (k, v) in m {
            result.insert(k, strip_bytes_rec(v));
        }
        Value::Map(result)
    } else {
        value
    }
}

impl Value {
    /// Rounds value to digits after comma, if the value is float
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]

    pub fn rounded(self, precision: Option<u32>) -> EResult<Value> {
        if let Some(precs) = precision {
            if let Value::F64(vf) = self {
                if precs > 0 {
                    let d = Decimal::from_f64_retain(vf)
                        .ok_or_else(|| Error::invalid_data("unable to parse float"))?;
                    let rounded = d.round_dp(precs);
                    return Ok(Value::F64(
                        rounded
                            .to_f64()
                            .ok_or_else(|| Error::invalid_data("unable to convert float"))?,
                    ));
                }
                return Ok(Value::U64(vf.round() as u64));
            }
            if let Value::F32(vf) = self {
                if precs > 0 {
                    let d = Decimal::from_f32_retain(vf)
                        .ok_or_else(|| Error::invalid_data("unable to parse float"))?;
                    let rounded = d.round_dp(precs);
                    return Ok(Value::F32(
                        rounded
                            .to_f32()
                            .ok_or_else(|| Error::invalid_data("unable to convert float"))?,
                    ));
                }
                return Ok(Value::U32(vf.round() as u32));
            }
        }
        Ok(self)
    }

    pub fn to_no_bytes(self) -> Value {
        strip_bytes_rec(self)
    }

    #[cfg(feature = "time")]
    #[inline]
    /// Tries to convert Value to f64 timestamp
    ///
    /// Valid options are:
    ///
    /// number - timestamp as-is
    /// time frame as N<S|T|H|D|W>, e.g. 5T for 5 minutes: now - time frame
    /// other string - tries to parse the string into date/time
    pub fn as_timestamp(&self) -> EResult<f64> {
        self.as_ts(true)
    }

    #[cfg(feature = "time")]
    #[inline]
    /// Same as as_timestamp() but time frames are added to now
    pub fn as_future_timestamp(&self) -> EResult<f64> {
        self.as_ts(false)
    }

    #[cfg(feature = "time")]
    #[allow(clippy::cast_precision_loss)]
    fn as_ts(&self, tf_past: bool) -> EResult<f64> {
        if let Ok(v) = f64::try_from(self) {
            Ok(v)
        } else if let Value::String(s) = self {
            if let Some(v) = parse_time_frame(s) {
                let now = crate::time::now_ns_float();
                Ok(if tf_past { now - v } else { now + v })
            } else {
                let d = dateparser::parse(s).map_err(Error::invalid_data)?;
                let timestamp = d.timestamp() as f64
                    + f64::try_from(d.timestamp_subsec_nanos())? / 1_000_000_000.0;
                Ok(timestamp)
            }
        } else {
            Err(Error::invalid_data("unsupported date/time format"))
        }
    }

    pub fn to_alphanumeric_string(self) -> EResult<String> {
        match self {
            Value::Bool(v) => Ok(v.to_string()),
            Value::U8(v) => Ok(v.to_string()),
            Value::U16(v) => Ok(v.to_string()),
            Value::U32(v) => Ok(v.to_string()),
            Value::U64(v) => Ok(v.to_string()),
            Value::I8(v) => Ok(v.to_string()),
            Value::I16(v) => Ok(v.to_string()),
            Value::I32(v) => Ok(v.to_string()),
            Value::I64(v) => Ok(v.to_string()),
            Value::F32(v) => Ok(v.to_string()),
            Value::F64(v) => Ok(v.to_string()),
            Value::Char(v) => Ok(v.to_string()),
            Value::String(v) => {
                for c in v.chars() {
                    if !c.is_alphanumeric() {
                        return Err(Error::invalid_params(format!("invalid symbols in {}", v)));
                    }
                }
                Ok(v)
            }
            Value::Unit => Ok("null".to_owned()),
            _ => Err(Error::invalid_data(format!(
                "unable to get string from {:?}",
                self
            ))),
        }
    }

    pub fn to_string_or_pack(self) -> EResult<String> {
        match self {
            Value::U8(v) => Ok(v.to_string()),
            Value::U16(v) => Ok(v.to_string()),
            Value::U32(v) => Ok(v.to_string()),
            Value::U64(v) => Ok(v.to_string()),
            Value::I8(v) => Ok(v.to_string()),
            Value::I16(v) => Ok(v.to_string()),
            Value::I32(v) => Ok(v.to_string()),
            Value::I64(v) => Ok(v.to_string()),
            Value::F32(v) => Ok(v.to_string()),
            Value::F64(v) => Ok(v.to_string()),
            Value::Char(v) => Ok(v.to_string()),
            Value::String(v) => Ok(v),
            _ => Ok(format!("!!{}", serde_json::to_string(&self)?)),
        }
    }

    pub fn unpack(self) -> EResult<Self> {
        if let Value::String(ref v) = self {
            if let Some(s) = v.strip_prefix("!!") {
                return serde_json::from_str(s).map_err(Into::into);
            }
        }
        Ok(self)
    }

    fn discriminant(&self) -> usize {
        match *self {
            Value::Bool(..) => 0,
            Value::U8(..) => 1,
            Value::U16(..) => 2,
            Value::U32(..) => 3,
            Value::U64(..) => 4,
            Value::I8(..) => 5,
            Value::I16(..) => 6,
            Value::I32(..) => 7,
            Value::I64(..) => 8,
            Value::F32(..) => 9,
            Value::F64(..) => 10,
            Value::Char(..) => 11,
            Value::String(..) => 12,
            Value::Unit => 13,
            Value::Option(..) => 14,
            Value::Newtype(..) => 15,
            Value::Seq(..) => 16,
            Value::Map(..) => 17,
            Value::Bytes(..) => 18,
        }
    }

    fn unexpected(&self) -> serde::de::Unexpected {
        match *self {
            Value::Bool(b) => serde::de::Unexpected::Bool(b),
            Value::U8(n) => serde::de::Unexpected::Unsigned(u64::from(n)),
            Value::U16(n) => serde::de::Unexpected::Unsigned(u64::from(n)),
            Value::U32(n) => serde::de::Unexpected::Unsigned(u64::from(n)),
            Value::U64(n) => serde::de::Unexpected::Unsigned(n),
            Value::I8(n) => serde::de::Unexpected::Signed(i64::from(n)),
            Value::I16(n) => serde::de::Unexpected::Signed(i64::from(n)),
            Value::I32(n) => serde::de::Unexpected::Signed(i64::from(n)),
            Value::I64(n) => serde::de::Unexpected::Signed(n),
            Value::F32(n) => serde::de::Unexpected::Float(f64::from(n)),
            Value::F64(n) => serde::de::Unexpected::Float(n),
            Value::Char(c) => serde::de::Unexpected::Char(c),
            Value::String(ref s) => serde::de::Unexpected::Str(s),
            Value::Unit => serde::de::Unexpected::Unit,
            Value::Option(_) => serde::de::Unexpected::Option,
            Value::Newtype(_) => serde::de::Unexpected::NewtypeStruct,
            Value::Seq(_) => serde::de::Unexpected::Seq,
            Value::Map(_) => serde::de::Unexpected::Map,
            Value::Bytes(ref b) => serde::de::Unexpected::Bytes(b),
        }
    }

    pub fn deserialize_into<'de, T: Deserialize<'de>>(self) -> Result<T, DeserializerError> {
        T::deserialize(self)
    }
    pub fn is_empty(&self) -> bool {
        match self {
            Value::Unit => true,
            Value::Option(v) => v.is_none(),
            Value::String(s) => s.is_empty(),
            _ => false,
        }
    }
    pub fn is_numeric(&self) -> bool {
        match self {
            Value::U8(_)
            | Value::U16(_)
            | Value::U32(_)
            | Value::U64(_)
            | Value::I8(_)
            | Value::I16(_)
            | Value::I32(_)
            | Value::I64(_)
            | Value::F32(_)
            | Value::F64(_) => true,
            Value::String(v) => v.parse::<f64>().is_ok() || v.parse::<i128>().is_ok(),
            _ => false,
        }
    }

    #[cfg(feature = "extended-value")]
    pub async fn extend(self, timeout: Duration, base: &Path) -> EResult<Value> {
        let op = crate::op::Op::new(timeout);
        extend_value(self, &op, base).await
    }
}

#[cfg(feature = "extended-value")]
#[async_recursion::async_recursion]
async fn extend_value(value: Value, op: &crate::op::Op, base: &Path) -> EResult<Value> {
    match value {
        Value::String(s) => Ok(extend_string_value(s, op, base).await?),
        Value::Seq(s) => {
            let mut result = Vec::with_capacity(s.len());
            for val in s {
                result.push(extend_value(val, op, base).await?);
            }
            Ok(Value::Seq(result))
        }
        Value::Map(m) => {
            let mut result = BTreeMap::new();
            for (k, v) in m {
                result.insert(k, extend_value(v, op, base).await?);
            }
            Ok(Value::Map(result))
        }
        _ => Ok(value),
    }
}

impl FromStr for Value {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Ok(v) = s.parse::<u64>() {
            Value::U64(v)
        } else if let Ok(v) = s.parse::<i64>() {
            Value::I64(v)
        } else if let Ok(v) = s.parse::<f64>() {
            Value::F64(v)
        } else {
            serde_json::from_str(s).unwrap_or_else(|_| {
                let s_l = s.to_lowercase();
                match s_l.as_str() {
                    "true" => Value::Bool(true),
                    "false" => Value::Bool(false),
                    "none" | "null" => Value::Unit,
                    _ => Value::String(s.to_owned()),
                }
            })
        })
    }
}

#[cfg(feature = "extended-value")]
async fn extend_string_value(val: String, op: &crate::op::Op, base: &Path) -> EResult<Value> {
    if let Some(s) = val.strip_prefix('^') {
        let mut sp = s.splitn(2, ' ');
        let cmd = sp.next().unwrap();
        macro_rules! pipe {
            () => {{
                let cmd = sp
                    .next()
                    .ok_or_else(|| Error::invalid_params("xvalue pipe: command not specified"))?;
                let cd_cmd = format!("cd \"{}\" && {}", base.to_string_lossy(), cmd);
                let res = bmart::process::command(
                    "sh",
                    &["-c", &cd_cmd],
                    op.timeout()?,
                    bmart::process::Options::default(),
                )
                .await?;
                if res.ok() {
                    res.out.join("\n")
                } else {
                    return Err(Error::failed(format!(
                        "xvalue pipe command failed to execute: {}",
                        res.err.join("\n")
                    )));
                }
            }};
        }
        match cmd {
            "include" => {
                let fname = sp.next().ok_or_else(|| {
                    Error::invalid_params("xvalue include: file name not specified")
                })?;
                let mut path = base.to_path_buf();
                path.push(fname);
                let content = tokio::time::timeout(op.timeout()?, tokio::fs::read(path)).await??;
                let val: Value = serde_yaml::from_slice(&content).map_err(Error::invalid_data)?;
                Ok(val)
            }
            "include-text" => {
                let fname = sp.next().ok_or_else(|| {
                    Error::invalid_params("xvalue include: file name not specified")
                })?;
                let mut path = base.to_path_buf();
                path.push(fname);
                let content =
                    tokio::time::timeout(op.timeout()?, tokio::fs::read_to_string(path)).await??;
                Ok(Value::String(content.trim_end().to_string()))
            }
            "pipe" => {
                let s = pipe!();
                let val: Value = serde_yaml::from_str(&s).map_err(Error::invalid_data)?;
                Ok(val)
            }
            "pipe-text" => {
                let s = pipe!();
                Ok(Value::String(s.trim_end().to_string()))
            }
            _ => Ok(Value::String(if s.starts_with('^') {
                s.to_owned()
            } else {
                val
            })),
        }
    } else {
        Ok(Value::String(val))
    }
}

impl Eq for Value {}
impl PartialOrd for Value {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

macro_rules! impl_from {
    ($v: ty, $val: expr) => {
        impl From<$v> for Value {
            fn from(src: $v) -> Value {
                $val(src)
            }
        }
    };
}

impl_from!(bool, Value::Bool);
impl_from!(u8, Value::U8);
impl_from!(i8, Value::I8);
impl_from!(u16, Value::U16);
impl_from!(i16, Value::I16);
impl_from!(u32, Value::U32);
impl_from!(i32, Value::I32);
impl_from!(u64, Value::U64);
impl_from!(i64, Value::I64);
impl_from!(f32, Value::F32);
impl_from!(f64, Value::F64);
impl_from!(String, Value::String);

// comparing $from unsigned bigger
macro_rules! ngt {
    ($n: expr, $from: ident, $to: ident) => {
        if $n > $to::MAX as $from {
            return Err(Error::invalid_data(format!(
                "value too big: {} (max: {})",
                $n,
                $to::MAX
            )));
        } else {
            $n as $to
        }
    };
}
// comparing $from signed bigger $to signed/unsigned smaller
macro_rules! ngt_nlt {
    ($n: expr, $from: ident, $to: ident) => {
        if $n > $to::MAX as $from {
            return Err(Error::invalid_data(format!(
                "value too big: {} (max: {})",
                $n,
                $to::MAX
            )));
        } else if $n < $to::MIN as $from {
            return Err(Error::invalid_data(format!(
                "value too small: {} (min: {})",
                $n,
                $to::MIN
            )));
        } else {
            $n as $to
        }
    };
}
// comparing $from smaller signed with $to unsigned (check that $from is zero-positive)
macro_rules! nltz {
    ($n: expr, $from: ident, $to: ident) => {
        if $n < 0 as $from {
            return Err(Error::invalid_data(format!(
                "value too small: {} (min: 0)",
                $n,
            )));
        } else {
            $n as $to
        }
    };
}

impl TryFrom<Value> for u8 {
    type Error = Error;

    fn try_from(value: Value) -> EResult<u8> {
        match value {
            Value::U8(v) => Ok(v),
            Value::U16(v) => Ok(ngt!(v, u16, u8)),
            Value::U32(v) => Ok(ngt!(v, u32, u8)),
            Value::U64(v) => Ok(ngt!(v, u64, u8)),
            Value::I8(v) => Ok(nltz!(v, i8, u8)),
            Value::I16(v) => Ok(ngt_nlt!(v, i16, u8)),
            Value::I32(v) => Ok(ngt_nlt!(v, i32, u8)),
            Value::I64(v) => Ok(ngt_nlt!(v, i64, u8)),
            Value::F32(v) => Ok(ngt_nlt!(v, f32, u8)),
            Value::F64(v) => Ok(ngt_nlt!(v, f64, u8)),
            Value::String(v) => Ok(v.parse::<u8>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for i8 {
    type Error = Error;

    fn try_from(value: Value) -> EResult<i8> {
        match value {
            Value::U8(v) => Ok(ngt!(v, u8, i8)),
            Value::U16(v) => Ok(ngt!(v, u16, i8)),
            Value::U32(v) => Ok(ngt!(v, u32, i8)),
            Value::U64(v) => Ok(ngt!(v, u64, i8)),
            Value::I8(v) => Ok(v),
            Value::I16(v) => Ok(ngt_nlt!(v, i16, i8)),
            Value::I32(v) => Ok(ngt_nlt!(v, i32, i8)),
            Value::I64(v) => Ok(ngt_nlt!(v, i64, i8)),
            Value::F32(v) => Ok(ngt_nlt!(v, f32, i8)),
            Value::F64(v) => Ok(ngt_nlt!(v, f64, i8)),
            Value::String(v) => Ok(v.parse::<i8>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for u16 {
    type Error = Error;

    fn try_from(value: Value) -> EResult<u16> {
        match value {
            Value::U8(v) => Ok(u16::from(v)),
            Value::U16(v) => Ok(v),
            Value::U32(v) => Ok(ngt!(v, u32, u16)),
            Value::U64(v) => Ok(ngt!(v, u64, u16)),
            Value::I8(v) => Ok(nltz!(v, i8, u16)),
            Value::I16(v) => Ok(nltz!(v, i16, u16)),
            Value::I32(v) => Ok(ngt_nlt!(v, i32, u16)),
            Value::I64(v) => Ok(ngt_nlt!(v, i64, u16)),
            Value::F32(v) => Ok(ngt_nlt!(v, f32, u16)),
            Value::F64(v) => Ok(ngt_nlt!(v, f64, u16)),
            Value::String(v) => Ok(v.parse::<u16>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for i16 {
    type Error = Error;

    fn try_from(value: Value) -> EResult<i16> {
        match value {
            Value::U8(v) => Ok(i16::from(v)),
            Value::U16(v) => Ok(ngt!(v, u16, i16)),
            Value::U32(v) => Ok(ngt!(v, u32, i16)),
            Value::U64(v) => Ok(ngt!(v, u64, i16)),
            Value::I8(v) => Ok(i16::from(v)),
            Value::I16(v) => Ok(v),
            Value::I32(v) => Ok(ngt_nlt!(v, i32, i16)),
            Value::I64(v) => Ok(ngt_nlt!(v, i64, i16)),
            Value::F32(v) => Ok(ngt_nlt!(v, f32, i16)),
            Value::F64(v) => Ok(ngt_nlt!(v, f64, i16)),
            Value::String(v) => Ok(v.parse::<i16>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for u32 {
    type Error = Error;

    fn try_from(value: Value) -> EResult<u32> {
        match value {
            Value::U8(v) => Ok(u32::from(v)),
            Value::U16(v) => Ok(u32::from(v)),
            Value::U32(v) => Ok(v),
            Value::U64(v) => Ok(ngt!(v, u64, u32)),
            Value::I8(v) => Ok(nltz!(v, i8, u32)),
            Value::I16(v) => Ok(nltz!(v, i16, u32)),
            Value::I32(v) => Ok(nltz!(v, i32, u32)),
            Value::I64(v) => Ok(ngt_nlt!(v, i64, u32)),
            Value::F32(v) => Ok(nltz!(v, f32, u32)),
            Value::F64(v) => Ok(ngt_nlt!(v, f64, u32)),
            Value::String(v) => Ok(v.parse::<u32>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for i32 {
    type Error = Error;

    fn try_from(value: Value) -> EResult<i32> {
        match value {
            Value::U8(v) => Ok(i32::from(v)),
            Value::U16(v) => Ok(i32::from(v)),
            Value::U32(v) => Ok(ngt!(v, u32, i32)),
            Value::U64(v) => Ok(ngt!(v, u64, i32)),
            Value::I8(v) => Ok(i32::from(v)),
            Value::I16(v) => Ok(i32::from(v)),
            Value::I32(v) => Ok(v),
            Value::I64(v) => Ok(ngt_nlt!(v, i64, i32)),
            #[allow(clippy::cast_possible_truncation)]
            Value::F32(v) => Ok(v as i32),
            Value::F64(v) => Ok(ngt_nlt!(v, f64, i32)),
            Value::String(v) => Ok(v.parse::<i32>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<&Value> for u64 {
    type Error = Error;

    fn try_from(value: &Value) -> EResult<u64> {
        match value {
            Value::U8(v) => Ok(u64::from(*v)),
            Value::U16(v) => Ok(u64::from(*v)),
            Value::U32(v) => Ok(u64::from(*v)),
            Value::U64(v) => Ok(*v),
            Value::I8(v) => Ok(nltz!(*v, i8, u64)),
            Value::I16(v) => Ok(nltz!(*v, i16, u64)),
            Value::I32(v) => Ok(nltz!(*v, i32, u64)),
            Value::I64(v) => Ok(nltz!(*v, i64, u64)),
            Value::F32(v) => Ok(nltz!(*v, f32, u64)),
            Value::F64(v) => Ok(nltz!(*v, f64, u64)),
            Value::String(v) => Ok(v.parse::<u64>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for u64 {
    type Error = Error;

    fn try_from(value: Value) -> EResult<u64> {
        match value {
            Value::U8(v) => Ok(u64::from(v)),
            Value::U16(v) => Ok(u64::from(v)),
            Value::U32(v) => Ok(u64::from(v)),
            Value::U64(v) => Ok(v),
            Value::I8(v) => Ok(nltz!(v, i8, u64)),
            Value::I16(v) => Ok(nltz!(v, i16, u64)),
            Value::I32(v) => Ok(nltz!(v, i32, u64)),
            Value::I64(v) => Ok(nltz!(v, i64, u64)),
            Value::F32(v) => Ok(nltz!(v, f32, u64)),
            Value::F64(v) => Ok(nltz!(v, f64, u64)),
            Value::String(v) => Ok(v.parse::<u64>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<&Value> for i64 {
    type Error = Error;

    fn try_from(value: &Value) -> EResult<i64> {
        match value {
            Value::U8(v) => Ok(i64::from(*v)),
            Value::U16(v) => Ok(i64::from(*v)),
            Value::U32(v) => Ok(i64::from(*v)),
            Value::U64(v) => Ok(ngt!(*v, u64, i64)),
            Value::I8(v) => Ok(i64::from(*v)),
            Value::I16(v) => Ok(i64::from(*v)),
            Value::I32(v) => Ok(i64::from(*v)),
            Value::I64(v) => Ok(*v),
            #[allow(clippy::cast_possible_truncation)]
            Value::F32(v) => Ok(*v as i64),
            #[allow(clippy::cast_possible_truncation)]
            Value::F64(v) => Ok(*v as i64),
            Value::String(v) => Ok(v.parse::<i64>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for i64 {
    type Error = Error;

    fn try_from(value: Value) -> EResult<i64> {
        match value {
            Value::U8(v) => Ok(i64::from(v)),
            Value::U16(v) => Ok(i64::from(v)),
            Value::U32(v) => Ok(i64::from(v)),
            Value::U64(v) => Ok(ngt!(v, u64, i64)),
            Value::I8(v) => Ok(i64::from(v)),
            Value::I16(v) => Ok(i64::from(v)),
            Value::I32(v) => Ok(i64::from(v)),
            Value::I64(v) => Ok(v),
            #[allow(clippy::cast_possible_truncation)]
            Value::F32(v) => Ok(v as i64),
            #[allow(clippy::cast_possible_truncation)]
            Value::F64(v) => Ok(v as i64),
            Value::String(v) => Ok(v.parse::<i64>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for f32 {
    type Error = Error;

    fn try_from(value: Value) -> EResult<f32> {
        match value {
            Value::F32(v) => Ok(v),
            Value::F64(v) => Ok(ngt_nlt!(v, f64, f32)),
            Value::U8(v) => Ok(f32::from(v)),
            Value::U16(v) => Ok(f32::from(v)),
            Value::U32(v) => Ok(ngt!(v, u32, f32)),
            Value::U64(v) => Ok(ngt!(v, u64, f32)),
            Value::I8(v) => Ok(f32::from(v)),
            Value::I16(v) => Ok(f32::from(v)),
            Value::I32(v) => Ok(ngt_nlt!(v, i32, f32)),
            Value::I64(v) => Ok(ngt_nlt!(v, i64, f32)),
            Value::String(v) => Ok(v.parse::<f32>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<&Value> for f64 {
    type Error = Error;

    fn try_from(value: &Value) -> EResult<f64> {
        match value {
            Value::U8(v) => Ok(f64::from(*v)),
            Value::U16(v) => Ok(f64::from(*v)),
            Value::U32(v) => Ok(f64::from(*v)),
            Value::U64(v) => Ok(ngt!(*v, u64, f64)),
            Value::I8(v) => Ok(f64::from(*v)),
            Value::I16(v) => Ok(f64::from(*v)),
            Value::I32(v) => Ok(f64::from(*v)),
            Value::I64(v) => Ok(ngt_nlt!(*v, i64, f64)),
            Value::F32(v) => Ok(f64::from(*v)),
            Value::F64(v) => Ok(*v),
            Value::String(v) => Ok(v.parse::<f64>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for f64 {
    type Error = Error;

    fn try_from(value: Value) -> EResult<f64> {
        match value {
            Value::U8(v) => Ok(f64::from(v)),
            Value::U16(v) => Ok(f64::from(v)),
            Value::U32(v) => Ok(f64::from(v)),
            Value::U64(v) => Ok(ngt!(v, u64, f64)),
            Value::I8(v) => Ok(f64::from(v)),
            Value::I16(v) => Ok(f64::from(v)),
            Value::I32(v) => Ok(f64::from(v)),
            Value::I64(v) => Ok(ngt_nlt!(v, i64, f64)),
            Value::F32(v) => Ok(f64::from(v)),
            Value::F64(v) => Ok(v),
            Value::String(v) => Ok(v.parse::<f64>()?),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for Option<std::time::Duration> {
    type Error = Error;

    fn try_from(v: Value) -> EResult<Option<std::time::Duration>> {
        let t: f64 = v.try_into()?;
        if t > 0.0 {
            Ok(Some(std::time::Duration::from_secs_f64(t)))
        } else {
            Ok(None)
        }
    }
}

impl TryFrom<Value> for String {
    type Error = Error;

    fn try_from(v: Value) -> EResult<String> {
        match v {
            Value::Option(Some(s)) => Ok((*s).try_into()?),
            Value::String(s) => Ok(s),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<&Value> for String {
    type Error = Error;

    fn try_from(v: &Value) -> EResult<String> {
        match v {
            Value::Option(Some(s)) => Ok(s.as_ref().try_into()?),
            Value::String(s) => Ok(s.clone()),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl<'a> TryFrom<&'a Value> for &'a str {
    type Error = Error;

    fn try_from(v: &'a Value) -> EResult<&'a str> {
        match v {
            Value::Option(Some(s)) => Ok(s.as_ref().try_into()?),
            Value::String(s) => Ok(s),
            _ => Err(Error::invalid_data(ERR_INVALID_VALUE)),
        }
    }
}

impl TryFrom<Value> for Option<String> {
    type Error = Error;

    fn try_from(v: Value) -> EResult<Option<String>> {
        let s = match v {
            Value::Option(v) => match v {
                Some(s) => (*s).try_into()?,
                None => return Ok(None),
            },
            Value::Unit => return Ok(None),
            Value::String(s) => s,
            _ => {
                return Err(Error::invalid_data(ERR_INVALID_VALUE));
            }
        };
        Ok(if s.is_empty() { None } else { Some(s) })
    }
}

impl TryFrom<Value> for std::time::Duration {
    type Error = Error;

    fn try_from(v: Value) -> EResult<std::time::Duration> {
        Ok(std::time::Duration::from_secs_f64(v.try_into()?))
    }
}

impl TryFrom<Value> for Vec<Value> {
    type Error = Error;

    fn try_from(value: Value) -> EResult<Vec<Value>> {
        match value {
            Value::Seq(vec) => Ok(vec),
            Value::String(s) => Ok(s.split(',').map(|s| Value::String(s.to_owned())).collect()),
            _ => Err(Error::invalid_data("Expected vec or string")),
        }
    }
}

impl<S: BuildHasher + Default> TryFrom<Value> for HashSet<Value, S> {
    type Error = Error;

    fn try_from(value: Value) -> EResult<HashSet<Value, S>> {
        match value {
            Value::Seq(vec) => Ok(HashSet::from_iter(vec)),
            Value::String(s) => Ok(s.split(',').map(|s| Value::String(s.to_owned())).collect()),
            _ => Err(Error::invalid_data("Expected vec or string")),
        }
    }
}

impl From<HashSet<ipnetwork::IpNetwork>> for Value {
    fn from(v: HashSet<ipnetwork::IpNetwork>) -> Value {
        to_value(v).unwrap()
    }
}

impl<S: BuildHasher + Default> TryFrom<Value> for HashSet<ipnetwork::IpNetwork, S> {
    type Error = Error;

    fn try_from(value: Value) -> EResult<HashSet<ipnetwork::IpNetwork, S>> {
        match value {
            Value::Seq(vec) => {
                let mut result = HashSet::default();
                for v in vec {
                    result.insert(v.deserialize_into()?);
                }
                Ok(result)
            }
            Value::String(s) => {
                let mut result = HashSet::default();
                for v in s.split(',') {
                    result.insert(v.parse()?);
                }
                Ok(result)
            }
            _ => Err(Error::invalid_data("Expected vec or string")),
        }
    }
}

impl TryFrom<Value> for Vec<String> {
    type Error = Error;

    fn try_from(value: Value) -> EResult<Vec<String>> {
        match value {
            Value::Seq(vec) => {
                let mut result = Vec::new();
                for v in vec {
                    result.push(v.try_into()?);
                }
                Ok(result)
            }
            Value::String(s) => Ok(s.split(',').map(ToOwned::to_owned).collect()),
            _ => Err(Error::invalid_data("Expected vec or string")),
        }
    }
}

impl TryFrom<&Value> for Vec<String> {
    type Error = Error;

    fn try_from(value: &Value) -> EResult<Vec<String>> {
        match value {
            Value::Seq(vec) => {
                let mut result = Vec::new();
                for v in vec {
                    result.push(v.try_into()?);
                }
                Ok(result)
            }
            Value::String(s) => Ok(s.split(',').map(ToOwned::to_owned).collect()),
            _ => Err(Error::invalid_data("Expected vec or string")),
        }
    }
}

impl<'a> TryFrom<&'a Value> for Vec<&'a str> {
    type Error = Error;

    fn try_from(value: &'a Value) -> EResult<Vec<&'a str>> {
        match value {
            Value::Seq(vec) => {
                let mut result = Vec::new();
                for v in vec {
                    result.push(v.try_into()?);
                }
                Ok(result)
            }
            Value::String(s) => Ok(s.split(',').collect()),
            _ => Err(Error::invalid_data("Expected vec or string")),
        }
    }
}

impl TryFrom<Value> for bool {
    type Error = Error;

    fn try_from(value: Value) -> EResult<bool> {
        match value {
            Value::Bool(v) => Ok(v),
            Value::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(true),
                "false" | "0" | "no" => Ok(false),
                _ => Err(Error::invalid_data(format!(
                    "Can not convert {} to boolean",
                    s
                ))),
            },
            _ => {
                let n: u64 = value
                    .try_into()
                    .map_err(|_| Error::invalid_data("Expected boolean"))?;
                if n == 0 {
                    Ok(false)
                } else if n == 1 {
                    Ok(true)
                } else {
                    Err(Error::invalid_data(format!(
                        "Can not convert {} to boolean",
                        n
                    )))
                }
            }
        }
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Value {
        Value::String(s.to_owned())
    }
}

impl From<&String> for Value {
    fn from(s: &String) -> Value {
        Value::String(s.clone())
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Value {
        Value::Seq(v)
    }
}

impl From<HashSet<Value>> for Value {
    fn from(v: HashSet<Value>) -> Value {
        Value::Seq(Vec::from_iter(v))
    }
}

impl From<Vec<String>> for Value {
    fn from(v: Vec<String>) -> Value {
        Value::Seq(v.iter().map(Into::into).collect::<Vec<Value>>())
    }
}

impl From<BTreeMap<Value, Value>> for Value {
    fn from(v: BTreeMap<Value, Value>) -> Value {
        Value::Map(v)
    }
}

impl From<std::time::Duration> for Value {
    fn from(v: std::time::Duration) -> Value {
        v.as_secs_f64().into()
    }
}

impl From<Option<std::time::Duration>> for Value {
    fn from(v: Option<std::time::Duration>) -> Value {
        v.map_or(Value::Unit, |d| d.as_secs_f64().into())
    }
}

impl From<Option<f64>> for Value {
    fn from(v: Option<f64>) -> Value {
        v.map_or(Value::Unit, Value::F64)
    }
}

impl From<Option<String>> for Value {
    fn from(v: Option<String>) -> Value {
        v.map_or(Value::Unit, Into::into)
    }
}

impl TryFrom<Value> for serde_json::Value {
    type Error = Error;
    fn try_from(v: Value) -> EResult<Self> {
        serde_json::to_value(v).map_err(Into::into)
    }
}

impl TryFrom<serde_json::Value> for Value {
    type Error = Error;
    fn try_from(v: serde_json::Value) -> EResult<Self> {
        serde_json::from_value(v).map_err(Into::into)
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use serde::Serialize;

    #[test]
    fn test_val_pack() -> EResult<()> {
        #[derive(Serialize)]
        struct My {
            test: bool,
            abc: usize,
        }

        let my = My {
            test: true,
            abc: 123,
        };
        let mut valx: Value = to_value(my)?;
        valx = valx.unpack()?;
        let vlstr = valx.clone().to_string_or_pack()?;
        dbg!(&vlstr);
        let mut val = Value::String(vlstr);
        val = val.unpack()?;
        assert_eq!(val, valx);
        Ok(())
    }

    #[test]
    fn test_val_parse() {
        let val: Value = "12345.111".parse().unwrap();
        assert_eq!(val, Value::F64(12345.111));
        let val: Value = "12345".parse().unwrap();
        assert_eq!(val, Value::U64(12345));
        let val: Value = "-12345".parse().unwrap();
        assert_eq!(val, Value::I64(-12345));
        let val: Value = "True".parse().unwrap();
        assert_eq!(val, Value::Bool(true));
        let val: Value = "False".parse().unwrap();
        assert_eq!(val, Value::Bool(false));
        let val: Value = "None".parse().unwrap();
        assert_eq!(val, Value::Unit);
        let val: Value = "Null".parse().unwrap();
        assert_eq!(val, Value::Unit);
    }
}
