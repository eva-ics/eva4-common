//#![cfg_attr(feature = "nostd", no_std)]

//#[cfg(feature = "ext")]
//#[macro_use]
//extern crate lazy_static;

use crate::value::{to_value, Value};
#[cfg(feature = "axum")]
use axum::http::StatusCode;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::hash::{BuildHasher, Hash, Hasher};
use std::str::FromStr;
use std::time::Duration;

pub const LOG_LEVEL_TRACE: u8 = 0;
pub const LOG_LEVEL_DEBUG: u8 = 10;
pub const LOG_LEVEL_INFO: u8 = 20;
pub const LOG_LEVEL_WARN: u8 = 30;
pub const LOG_LEVEL_ERROR: u8 = 40;
pub const LOG_LEVEL_OFF: u8 = 100;

#[inline]
pub fn log_level_code(level: log::Level) -> u8 {
    match level {
        log::Level::Trace => LOG_LEVEL_TRACE,
        log::Level::Debug => LOG_LEVEL_DEBUG,
        log::Level::Info => LOG_LEVEL_INFO,
        log::Level::Warn => LOG_LEVEL_WARN,
        log::Level::Error => LOG_LEVEL_ERROR,
    }
}

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

pub mod op;
mod runtime_tests;
pub mod tools;

#[allow(unused_imports)]
pub use runtime_tests::self_test;

#[cfg(feature = "acl")]
pub mod acl;
#[cfg(feature = "actions")]
pub mod actions;
#[cfg(feature = "cache")]
pub mod cache;
#[cfg(feature = "common-payloads")]
pub mod common_payloads;
#[cfg(feature = "console-logger")]
pub mod console_logger;
#[cfg(feature = "db")]
pub mod db;
#[cfg(feature = "data-objects")]
pub mod dobj;
#[cfg(any(feature = "events", feature = "common-payloads", feature = "logger"))]
pub mod events;
//#[cfg(feature = "ext")]
//pub mod ext;
#[cfg(feature = "hyper")]
pub mod hyper_tools;
#[cfg(feature = "logger")]
pub mod logger;
#[cfg(feature = "logic")]
pub mod logic;
#[cfg(feature = "payload")]
pub mod payload;
#[cfg(feature = "registry")]
pub mod registry;
#[cfg(feature = "serde-keyvalue")]
pub mod serde_keyvalue;
#[cfg(feature = "services")]
pub mod services;
#[cfg(feature = "time")]
pub mod time;
pub mod transform;
#[cfg(feature = "workers")]
pub mod workers;

pub mod value;

pub mod prelude {
    pub use crate::value::to_value;
    pub use crate::value::Value;
    pub use crate::value::ValueOption;
    pub use crate::value::ValueOptionOwned;
    pub use crate::EResult;
    pub use crate::Error;
    pub use crate::ErrorKind;
    pub use crate::ItemKind;
    pub use crate::ItemStatus;
    pub use crate::IEID;
    pub use crate::OID;
}

static ERR_INVALID_OID: &str = "Invalid OID format";
static ERR_OID_TOO_LONG: &str = "OID too long";

pub const SLEEP_STEP: Duration = Duration::from_millis(100);

#[inline]
pub fn get_default_sleep_step() -> Duration {
    SLEEP_STEP
}

pub type EResult<T> = std::result::Result<T, Error>;

pub type ItemStatus = i16;

pub const ITEM_STATUS_ERROR: i16 = -1;

pub const ERR_CODE_NOT_FOUND: i16 = -32001;
pub const ERR_CODE_ACCESS_DENIED: i16 = -32002;
pub const ERR_CODE_SYSTEM_ERROR: i16 = -32003;
pub const ERR_CODE_OTHER: i16 = -32004;
pub const ERR_CODE_NOT_READY: i16 = -32005;
pub const ERR_CODE_UNSUPPORTED: i16 = -32006;
pub const ERR_CODE_CORE_ERROR: i16 = -32007;
pub const ERR_CODE_TIMEOUT: i16 = -32008;
pub const ERR_CODE_INVALID_DATA: i16 = -32009;
pub const ERR_CODE_FUNC_FAILED: i16 = -32010;
pub const ERR_CODE_ABORTED: i16 = -32011;
pub const ERR_CODE_ALREADY_EXISTS: i16 = -32012;
pub const ERR_CODE_BUSY: i16 = -32013;
pub const ERR_CODE_METHOD_NOT_IMPLEMENTED: i16 = -32014;
pub const ERR_CODE_TOKEN_RESTRICTED: i16 = -32015;
pub const ERR_CODE_IO: i16 = -32016;
pub const ERR_CODE_REGISTRY: i16 = -32017;
pub const ERR_CODE_EVAHI_AUTH_REQUIRED: i16 = -32018;

pub const ERR_CODE_ACCESS_DENIED_MORE_DATA_REQUIRED: i16 = -32022;

pub const ERR_CODE_PARSE: i16 = -32700;
pub const ERR_CODE_INVALID_REQUEST: i16 = -32600;
pub const ERR_CODE_METHOD_NOT_FOUND: i16 = -32601;
pub const ERR_CODE_INVALID_PARAMS: i16 = -32602;
pub const ERR_CODE_INTERNAL_RPC: i16 = -32603;

pub const ERR_CODE_BUS_CLIENT_NOT_REGISTERED: i16 = -32113;
pub const ERR_CODE_BUS_DATA: i16 = -32114;
pub const ERR_CODE_BUS_IO: i16 = -32115;
pub const ERR_CODE_BUS_OTHER: i16 = -32116;
pub const ERR_CODE_BUS_NOT_SUPPORTED: i16 = -32117;
pub const ERR_CODE_BUS_BUSY: i16 = -32118;
pub const ERR_CODE_BUS_NOT_DELIVERED: i16 = -32119;
pub const ERR_CODE_BUS_TIMEOUT: i16 = -32120;
pub const ERR_CODE_BUS_ACCESS: i16 = -32121;

pub const WILDCARD: &[&str] = &["#", "*"];
pub const MATCH_ANY: &[&str] = &["+", "?"];

#[inline]
pub fn is_str_wildcard(s: &str) -> bool {
    WILDCARD.contains(&s)
}
#[inline]
pub fn is_str_any(s: &str) -> bool {
    MATCH_ANY.contains(&s)
}

#[derive(Serialize_repr, Deserialize_repr, Eq, PartialEq, Debug, Copy, Clone)]
#[repr(i16)]
pub enum ErrorKind {
    CoreError = ERR_CODE_CORE_ERROR,
    Unsupported = ERR_CODE_UNSUPPORTED,
    NotReady = ERR_CODE_NOT_READY,
    IOError = ERR_CODE_IO,
    RegistryError = ERR_CODE_REGISTRY,
    InvalidData = ERR_CODE_INVALID_DATA,
    FunctionFailed = ERR_CODE_FUNC_FAILED,
    ResourceNotFound = ERR_CODE_NOT_FOUND,
    ResourceBusy = ERR_CODE_BUSY,
    ResourceAlreadyExists = ERR_CODE_ALREADY_EXISTS,
    AccessDenied = ERR_CODE_ACCESS_DENIED,
    AccessDeniedMoreDataRequired = ERR_CODE_ACCESS_DENIED_MORE_DATA_REQUIRED,
    MethodNotImplemented = ERR_CODE_METHOD_NOT_IMPLEMENTED,
    MethodNotFound = ERR_CODE_METHOD_NOT_FOUND,
    InvalidParameter = ERR_CODE_INVALID_PARAMS,
    Timeout = ERR_CODE_TIMEOUT,
    Aborted = ERR_CODE_ABORTED,
    EvaHIAuthenticationRequired = ERR_CODE_EVAHI_AUTH_REQUIRED,
    TokenRestricted = ERR_CODE_TOKEN_RESTRICTED,
    BusClientNotRegistered = ERR_CODE_BUS_CLIENT_NOT_REGISTERED,
    BusData = ERR_CODE_BUS_DATA,
    BusIo = ERR_CODE_BUS_IO,
    BusOther = ERR_CODE_BUS_OTHER,
    BusNotSupported = ERR_CODE_BUS_NOT_SUPPORTED,
    BusBusy = ERR_CODE_BUS_BUSY,
    BusNotDelivered = ERR_CODE_BUS_NOT_DELIVERED,
    BusTimeout = ERR_CODE_BUS_TIMEOUT,
    BusAccess = ERR_CODE_BUS_ACCESS,
    Other = ERR_CODE_OTHER,
}

impl From<i16> for ErrorKind {
    fn from(code: i16) -> ErrorKind {
        match code {
            x if x == ErrorKind::CoreError as i16 => ErrorKind::CoreError,
            x if x == ErrorKind::Unsupported as i16 => ErrorKind::Unsupported,
            x if x == ErrorKind::IOError as i16 => ErrorKind::IOError,
            x if x == ErrorKind::RegistryError as i16 => ErrorKind::RegistryError,
            x if x == ErrorKind::InvalidData as i16 => ErrorKind::InvalidData,
            x if x == ErrorKind::FunctionFailed as i16 => ErrorKind::FunctionFailed,
            x if x == ErrorKind::ResourceNotFound as i16 => ErrorKind::ResourceNotFound,
            x if x == ErrorKind::ResourceBusy as i16 => ErrorKind::ResourceBusy,
            x if x == ErrorKind::ResourceAlreadyExists as i16 => ErrorKind::ResourceAlreadyExists,
            x if x == ErrorKind::AccessDenied as i16 => ErrorKind::AccessDenied,
            x if x == ErrorKind::AccessDeniedMoreDataRequired as i16 => {
                ErrorKind::AccessDeniedMoreDataRequired
            }
            x if x == ErrorKind::MethodNotImplemented as i16 => ErrorKind::MethodNotImplemented,
            x if x == ErrorKind::MethodNotFound as i16 => ErrorKind::MethodNotFound,
            x if x == ErrorKind::InvalidParameter as i16 => ErrorKind::InvalidParameter,
            x if x == ErrorKind::Timeout as i16 => ErrorKind::Timeout,
            x if x == ErrorKind::Aborted as i16 => ErrorKind::Aborted,
            x if x == ErrorKind::EvaHIAuthenticationRequired as i16 => {
                ErrorKind::EvaHIAuthenticationRequired
            }
            x if x == ErrorKind::TokenRestricted as i16 => ErrorKind::TokenRestricted,
            x if x == ErrorKind::Other as i16 => ErrorKind::Other,
            x if x == ErrorKind::NotReady as i16 => ErrorKind::NotReady,
            x if x == ErrorKind::BusClientNotRegistered as i16 => ErrorKind::BusClientNotRegistered,
            x if x == ErrorKind::BusData as i16 => ErrorKind::BusData,
            x if x == ErrorKind::BusIo as i16 => ErrorKind::BusIo,
            x if x == ErrorKind::BusOther as i16 => ErrorKind::BusOther,
            x if x == ErrorKind::BusNotSupported as i16 => ErrorKind::BusNotSupported,
            x if x == ErrorKind::BusBusy as i16 => ErrorKind::BusBusy,
            x if x == ErrorKind::BusNotDelivered as i16 => ErrorKind::BusNotDelivered,
            x if x == ErrorKind::BusTimeout as i16 => ErrorKind::BusTimeout,
            x if x == ErrorKind::BusAccess as i16 => ErrorKind::BusAccess,
            _ => ErrorKind::Other,
        }
    }
}

impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ErrorKind::CoreError => "Core error",
                ErrorKind::Unsupported => "Unsupported",
                ErrorKind::IOError => "IO error",
                ErrorKind::RegistryError => "Registry error",
                ErrorKind::InvalidData => "Invalid data",
                ErrorKind::FunctionFailed => "Function failed",
                ErrorKind::ResourceNotFound => "Resource not found",
                ErrorKind::ResourceBusy => "Resource busy",
                ErrorKind::ResourceAlreadyExists => "Resource already exists",
                ErrorKind::AccessDenied => "Access denied",
                ErrorKind::AccessDeniedMoreDataRequired => "Access denied, more data required",
                ErrorKind::MethodNotImplemented => "Method not implemented",
                ErrorKind::MethodNotFound => "Method not found",
                ErrorKind::InvalidParameter => "Invalid parameter",
                ErrorKind::Timeout => "Timed out",
                ErrorKind::Aborted => "Aborted",
                ErrorKind::EvaHIAuthenticationRequired => "EvaHI authentication required",
                ErrorKind::TokenRestricted => "Token restricted",
                ErrorKind::Other => "Other",
                ErrorKind::NotReady => "Not ready",
                ErrorKind::BusClientNotRegistered => "Bus client not registered",
                ErrorKind::BusData => "Bus data error",
                ErrorKind::BusIo => "Bus IO error",
                ErrorKind::BusOther => "Bus error",
                ErrorKind::BusNotSupported => "Bus feature not supported",
                ErrorKind::BusBusy => "Bus busy",
                ErrorKind::BusNotDelivered => "Bus not delivered",
                ErrorKind::BusTimeout => "Bus timed out",
                ErrorKind::BusAccess => "Bus op access denied",
            }
        )
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Error {
    kind: ErrorKind,
    message: Option<Cow<'static, str>>,
}

impl std::error::Error for Error {}

macro_rules! impl_err_error {
    ($src: ty, $f: path) => {
        impl From<$src> for Error {
            fn from(err: $src) -> Error {
                $f(err)
            }
        }
    };
}

impl_err_error!(std::string::FromUtf8Error, Error::invalid_data);
impl_err_error!(std::fmt::Error, Error::failed);
impl_err_error!(std::str::Utf8Error, Error::invalid_data);
impl_err_error!(std::num::ParseIntError, Error::invalid_data);
impl_err_error!(std::num::ParseFloatError, Error::invalid_data);
impl_err_error!(std::num::TryFromIntError, Error::invalid_data);
impl_err_error!(ipnetwork::IpNetworkError, Error::invalid_data);
impl_err_error!(serde_json::Error, Error::invalid_data);
impl_err_error!(std::io::Error, Error::io);
#[cfg(feature = "bus-rpc")]
impl_err_error!(busrt::Error, Error::io);
#[cfg(any(feature = "services", feature = "workers"))]
impl_err_error!(tokio::sync::oneshot::error::RecvError, Error::io);
#[cfg(any(feature = "services", feature = "workers"))]
impl_err_error!(tokio::sync::TryLockError, Error::core);
#[cfg(feature = "payload")]
impl_err_error!(rmp_serde::encode::Error, Error::invalid_data);
#[cfg(feature = "payload")]
impl_err_error!(rmp_serde::decode::Error, Error::invalid_data);
impl_err_error!(std::array::TryFromSliceError, Error::invalid_data);
#[cfg(feature = "db")]
impl_err_error!(yedb::Error, Error::registry);
#[cfg(any(feature = "db", feature = "cache"))]
impl_err_error!(sqlx::Error, Error::io);
#[cfg(feature = "dataconv")]
impl_err_error!(hex::FromHexError, Error::invalid_data);
#[cfg(feature = "dataconv")]
impl_err_error!(regex::Error, Error::invalid_data);
#[cfg(any(feature = "actions", feature = "dataconv"))]
impl_err_error!(uuid::Error, Error::invalid_data);
#[cfg(feature = "services")]
impl_err_error!(openssl::error::ErrorStack, Error::core);

#[cfg(feature = "bus-rpc")]
impl From<busrt::rpc::RpcError> for Error {
    fn from(err: busrt::rpc::RpcError) -> Self {
        Error {
            kind: err.code().into(),
            message: err
                .data()
                .map(|v| Cow::Owned(std::str::from_utf8(v).unwrap_or_default().to_owned())),
        }
    }
}

#[cfg(feature = "bus-rpc")]
impl From<Error> for busrt::rpc::RpcError {
    fn from(err: Error) -> Self {
        busrt::rpc::RpcError::new(
            err.kind() as i16,
            busrt::rpc::rpc_err_str(err.message().unwrap_or_default()),
        )
    }
}

#[cfg(feature = "bus-rpc")]
impl From<crate::value::SerializerError> for busrt::rpc::RpcError {
    fn from(err: crate::value::SerializerError) -> Self {
        busrt::rpc::RpcError::new(
            ErrorKind::InvalidData as i16,
            busrt::rpc::rpc_err_str(err.to_string()),
        )
    }
}

#[cfg(feature = "bus-rpc")]
impl From<crate::value::DeserializerError> for busrt::rpc::RpcError {
    fn from(err: crate::value::DeserializerError) -> Self {
        busrt::rpc::RpcError::new(
            ErrorKind::InvalidData as i16,
            busrt::rpc::rpc_err_str(err.to_string()),
        )
    }
}

#[cfg(any(feature = "services", feature = "workers", feature = "extended-value"))]
impl From<tokio::time::error::Elapsed> for Error {
    fn from(_e: tokio::time::error::Elapsed) -> Error {
        Error::timeout()
    }
}

#[cfg(any(feature = "services", feature = "workers"))]
impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Error {
        Error::failed(e)
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(_err: std::convert::Infallible) -> Error {
        panic!();
    }
}

impl Error {
    #[allow(clippy::must_use_candidate)]
    pub fn new<T: fmt::Display>(kind: ErrorKind, message: T) -> Self {
        Self {
            kind,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn new0(kind: ErrorKind) -> Self {
        Self {
            kind,
            message: None,
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn newc(kind: ErrorKind, message: Option<impl fmt::Display>) -> Self {
        Self {
            kind,
            message: message.map(|v| Cow::Owned(v.to_string())),
        }
    }

    pub fn code(&self) -> i16 {
        self.kind as i16
    }

    #[allow(clippy::must_use_candidate)]
    pub fn e<T: fmt::Display>(kind: ErrorKind, message: T) -> Self {
        Self {
            kind,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn not_found<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::ResourceNotFound,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn not_ready<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::NotReady,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn unsupported<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::Unsupported,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn registry<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::RegistryError,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn busy<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::ResourceBusy,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn core<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::CoreError,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn io<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::IOError,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn duplicate<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::ResourceAlreadyExists,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn failed<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::FunctionFailed,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn access<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::AccessDenied,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn access_more_data_required<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::AccessDeniedMoreDataRequired,
            message: Some(Cow::Owned(message.to_string())),
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn timeout() -> Self {
        Self {
            kind: ErrorKind::Timeout,
            message: None,
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn aborted() -> Self {
        Self {
            kind: ErrorKind::Aborted,
            message: None,
        }
    }

    #[allow(clippy::must_use_candidate)]
    pub fn invalid_data<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::InvalidData,
            message: Some(Cow::Owned(message.to_string())),
        }
    }
    fn invalid_data_static(message: &'static str) -> Self {
        Self {
            kind: ErrorKind::InvalidData,
            message: Some(Cow::Borrowed(message)),
        }
    }
    pub fn invalid_params<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::InvalidParameter,
            message: Some(Cow::Owned(message.to_string())),
        }
    }
    pub fn not_implemented<T: fmt::Display>(message: T) -> Self {
        Self {
            kind: ErrorKind::MethodNotImplemented,
            message: Some(Cow::Owned(message.to_string())),
        }
    }
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref().map(AsRef::as_ref)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(msg) = self.message.as_ref() {
            write!(f, "{}: {}", self.kind, msg)
        } else {
            write!(f, "{}", self.kind)
        }
    }
}

#[cfg(feature = "axum")]
impl From<Error> for (StatusCode, String) {
    fn from(e: Error) -> Self {
        let code = match e.kind() {
            ErrorKind::NotReady => StatusCode::SERVICE_UNAVAILABLE,
            ErrorKind::ResourceNotFound => StatusCode::NOT_FOUND,
            ErrorKind::ResourceBusy => StatusCode::LOCKED,
            ErrorKind::ResourceAlreadyExists => StatusCode::CONFLICT,
            ErrorKind::AccessDenied
            | ErrorKind::AccessDeniedMoreDataRequired
            | ErrorKind::EvaHIAuthenticationRequired
            | ErrorKind::TokenRestricted => StatusCode::FORBIDDEN,
            ErrorKind::MethodNotFound
            | ErrorKind::MethodNotImplemented
            | ErrorKind::InvalidParameter => StatusCode::BAD_REQUEST,
            ErrorKind::Timeout => StatusCode::REQUEST_TIMEOUT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (code, e.message.map(|v| v.to_string()).unwrap_or_default())
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Serialize, Deserialize)]
pub struct IEID(u64, u64);

impl IEID {
    #[allow(clippy::must_use_candidate)]
    #[inline]
    pub fn new(b: u64, i: u64) -> Self {
        Self(b, i)
    }
    #[inline]
    pub fn boot_id(&self) -> u64 {
        self.0
    }
    #[inline]
    pub fn is_phantom(&self) -> bool {
        self.0 == 0
    }
    #[inline]
    pub fn mark_phantom(&mut self) {
        self.0 = 0;
        self.1 = 0;
    }
    /// # Panics
    ///
    /// Will panic if the serializer has gone mad
    #[allow(clippy::must_use_candidate)]
    #[inline]
    pub fn to_value(&self) -> Value {
        let value_b: Value = self.0.into();
        let value_i: Value = self.1.into();
        to_value(vec![value_b, value_i]).unwrap()
    }

    /// Other IEID is newer than current
    #[inline]
    pub fn other_is_newer(&self, other: &IEID) -> bool {
        other.0 > self.0 || (other.0 == self.0 && other.1 > self.1)
    }

    /// Other IEID is less or equal to the current
    #[inline]
    pub fn other_is_less_or_equal(&self, other: &IEID) -> bool {
        other.0 < self.0 || (other.0 == self.0 && other.1 <= self.1)
    }
}

impl TryFrom<&Value> for IEID {
    type Error = Error;
    fn try_from(v: &Value) -> EResult<Self> {
        if let Value::Seq(s) = v {
            let mut ix = s.iter();
            let ieid_b = if let Some(b) = ix.next() {
                b.try_into()?
            } else {
                return Err(Error::invalid_data("First IEID element mismatch"));
            };
            let ieid_i = if let Some(i) = ix.next() {
                i.try_into()?
            } else {
                return Err(Error::invalid_data("Second IEID element mismatch"));
            };
            if ix.next().is_some() {
                return Err(Error::invalid_data(
                    "Incompatible IEID (more than 2 elements)",
                ));
            }
            Ok(Self(ieid_b, ieid_i))
        } else {
            Err(Error::invalid_data("invalid value for IEID"))
        }
    }
}

impl PartialOrd for IEID {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.0.cmp(&other.0) {
            Ordering::Less => Some(Ordering::Less),
            Ordering::Greater => Some(Ordering::Greater),
            Ordering::Equal => self.1.partial_cmp(&other.1),
        }
    }
}

#[derive(Clone, Eq)]
pub struct OID {
    kind: ItemKind,
    oid_str: String,
    path_str: String,
    tpos: u16,
    grp_pos: Option<u16>,
}

impl PartialEq for OID {
    fn eq(&self, other: &Self) -> bool {
        self.oid_str == other.oid_str
    }
}

impl Ord for OID {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.kind == other.kind {
            self.full_id().cmp(other.full_id())
        } else {
            self.kind.cmp(&other.kind)
        }
    }
}

impl PartialOrd for OID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub const OID_ALLOWED_SYMBOLS: &str = "_.()[]-\\";
pub const OID_MASK_ALLOWED_SYMBOLS: &str = "~_.()[]-+?#*\\";

pub const OID_MASK_PREFIX_FORMULA: &str = "f~";
pub const OID_MASK_PREFIX_REGEX: &str = "r~";

impl OID {
    #[inline]
    fn check(s: &str, is_path: bool) -> EResult<()> {
        if s.len() > 65000 {
            return Err(Error::invalid_data("OID too long"));
        }
        for c in s.chars() {
            if !(c.is_alphanumeric() || OID_ALLOWED_SYMBOLS.contains(c) || (is_path && c == '/')) {
                return Err(Error::invalid_data(format!("Invalid symbol in OID: {}", c)));
            }
        }
        Ok(())
    }
    #[allow(clippy::cast_possible_truncation)]
    pub fn new(kind: ItemKind, group: &str, id: &str) -> EResult<Self> {
        OID::check(group, true)?;
        OID::check(id, false)?;
        if group == "+" || id == "+" {
            return Err(Error::invalid_data("OID group or id can not be equal to +"));
        }
        let tp_str = kind.to_string();
        if id.is_empty() || group.is_empty() {
            Err(Error::invalid_data(ERR_INVALID_OID))
        } else if tp_str.len() + id.len() + group.len() + 2 > std::u16::MAX as usize {
            Err(Error::invalid_data(ERR_OID_TOO_LONG))
        } else {
            let oid_str = format!("{}:{}/{}", kind, group, id);
            let path_str = format!("{}/{}/{}", kind, group, id);
            let grp_pos = Some((group.len() as u16) + (tp_str.len() as u16) + 1);
            Ok(Self {
                kind,
                oid_str,
                path_str,
                grp_pos,
                tpos: tp_str.len() as u16 + 1,
            })
        }
    }
    #[inline]
    pub fn new0(kind: ItemKind, id: &str) -> EResult<Self> {
        Self::_new0(kind, id, true)
    }
    #[inline]
    pub fn new0_unchecked(kind: ItemKind, id: &str) -> EResult<Self> {
        Self::_new0(kind, id, false)
    }
    #[allow(clippy::cast_possible_truncation)]
    fn _new0(kind: ItemKind, id: &str, need_check: bool) -> EResult<Self> {
        if need_check {
            OID::check(id, true)?;
        }
        let tp_str = kind.to_string();
        if id.is_empty() {
            Err(Error::invalid_data(ERR_INVALID_OID))
        } else if id.len() + tp_str.len() >= std::u16::MAX as usize {
            Err(Error::invalid_data(ERR_OID_TOO_LONG))
        } else {
            let grp_pos = id.rfind('/').map(|p| p as u16 + tp_str.len() as u16 + 1);
            let oid_str = format!("{}:{}", kind, id);
            let path_str = format!("{}/{}", kind, id);
            Ok(Self {
                kind,
                oid_str,
                path_str,
                grp_pos,
                tpos: tp_str.len() as u16 + 1,
            })
        }
    }
    #[inline]
    pub fn id(&self) -> &str {
        self.grp_pos.map_or_else(
            || &self.oid_str[self.tpos as usize..],
            |g| &self.oid_str[(g + 1) as usize..],
        )
    }
    #[inline]
    pub fn full_id(&self) -> &str {
        &self.oid_str[self.tpos as usize..]
    }
    #[inline]
    pub fn group(&self) -> Option<&str> {
        self.grp_pos
            .map(|g| &self.oid_str[self.tpos as usize..g as usize])
    }
    #[inline]
    pub fn kind(&self) -> ItemKind {
        self.kind
    }
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.oid_str
    }
    #[inline]
    pub fn as_path(&self) -> &str {
        &self.path_str
    }
    #[inline]
    pub fn is_wildcard(&self) -> bool {
        is_str_wildcard(self.id())
    }
    #[inline]
    pub fn to_wildcard_str(&self, wildcard_suffix: &str) -> String {
        let mut s = format!("{}:", self.kind);
        if let Some(group) = self.group() {
            s = s + group + "/";
        }
        s + wildcard_suffix
    }
    pub fn serialize_into(&self, target: &mut BTreeMap<Value, Value>) {
        target.insert("oid".into(), self.as_str().into());
        //COMPAT, deprecated, remove in 4.2
        target.insert("full_id".into(), self.full_id().into());
        target.insert("id".into(), self.id().into());
        target.insert("group".into(), self.group().map_or(Value::Unit, Into::into));
        target.insert("type".into(), self.kind.into());
    }
    pub fn from_str_type(tp: ItemKind, s: &str) -> EResult<Self> {
        if let Some(tpos) = s.find(':') {
            let otp: ItemKind = s[..tpos].parse()?;
            if otp == tp {
                Self::new0(tp, &s[tpos + 1..])
            } else {
                Err(Error::invalid_data(format!(
                    "OID type mismatch, expected: {}, found: {}",
                    tp, otp
                )))
            }
        } else {
            OID::new0(tp, s)
        }
    }
    #[inline]
    pub fn from_path(s: &str) -> EResult<Self> {
        Self::parse_oid(s, '/')
    }
    #[inline]
    fn parse_oid(s: &str, c: char) -> EResult<Self> {
        s.find(c).map_or(
            Err(Error::invalid_data(format!("{}: {}", ERR_INVALID_OID, s))),
            |tpos| {
                let tp: ItemKind = s[..tpos].parse()?;
                Self::new0(tp, &s[tpos + 1..])
            },
        )
    }
}

impl AsRef<str> for OID {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<OID> for OID {
    fn as_ref(&self) -> &OID {
        self
    }
}

impl FromStr for OID {
    type Err = Error;
    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_oid(s, ':')
    }
}

impl TryFrom<&Value> for OID {
    type Error = Error;
    fn try_from(value: &Value) -> Result<OID, Self::Error> {
        let s: &str = value.try_into()?;
        s.parse()
    }
}

impl Serialize for OID {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

// in case of problems with Deserializer
#[inline]
pub fn deserialize_oid<'de, D>(deserializer: D) -> Result<OID, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = String::deserialize(deserializer)?;
    buf.parse().map_err(serde::de::Error::custom)
}

impl<'de> Deserialize<'de> for OID {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<OID, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for OID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.oid_str)
    }
}

impl fmt::Debug for OID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.oid_str)
    }
}

impl Hash for OID {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        (self.kind as u16).hash(hasher);
        self.full_id().hash(hasher);
    }
}

impl From<OID> for Value {
    fn from(oid: OID) -> Value {
        oid.as_str().into()
    }
}

impl From<&OID> for Value {
    fn from(oid: &OID) -> Value {
        oid.as_str().into()
    }
}

impl TryFrom<Value> for OID {
    type Error = Error;
    fn try_from(value: Value) -> EResult<OID> {
        match value {
            Value::String(s) => Ok(s.parse()?),
            _ => Err(Error::invalid_data("Expected string")),
        }
    }
}

impl<S: BuildHasher + Default> TryFrom<Value> for HashSet<OID, S> {
    type Error = Error;
    fn try_from(value: Value) -> EResult<HashSet<OID, S>> {
        match value {
            Value::Seq(vec) => {
                let mut result = HashSet::default();
                for v in vec {
                    result.insert(v.try_into()?);
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

impl<S: BuildHasher> From<HashSet<OID, S>> for Value {
    fn from(v: HashSet<OID, S>) -> Value {
        Value::Seq(v.iter().map(|oid| to_value(oid).unwrap()).collect())
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Ord, PartialOrd, Hash)]
#[repr(u16)]
pub enum ItemKind {
    Unit = 100,
    Sensor = 101,
    Lvar = 200,
    Lmacro = 300,
}

impl ItemKind {
    pub fn as_str(&self) -> &str {
        match self {
            ItemKind::Unit => "unit",
            ItemKind::Sensor => "sensor",
            ItemKind::Lvar => "lvar",
            ItemKind::Lmacro => "lmacro",
        }
    }
}

impl fmt::Display for ItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<ItemKind> for Value {
    fn from(src: ItemKind) -> Value {
        src.to_string().into()
    }
}

impl FromStr for ItemKind {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unit" | "U" => Ok(ItemKind::Unit),
            "sensor" | "S" => Ok(ItemKind::Sensor),
            "lvar" | "LV" => Ok(ItemKind::Lvar),
            "lmacro" | "K" => Ok(ItemKind::Lmacro),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Invalid item type: {}", s),
            )),
        }
    }
}

impl TryFrom<&Value> for ItemKind {
    type Error = Error;
    fn try_from(value: &Value) -> Result<ItemKind, Self::Error> {
        TryInto::<&str>::try_into(value)?.parse()
    }
}

impl TryFrom<&Value> for Vec<ItemKind> {
    type Error = Error;
    fn try_from(value: &Value) -> Result<Vec<ItemKind>, Self::Error> {
        let data: Vec<&str> = value.try_into()?;
        let mut result = Vec::new();
        for d in data {
            result.push(TryInto::<&str>::try_into(d)?.parse()?);
        }
        Ok(result)
    }
}

impl TryFrom<Value> for ItemKind {
    type Error = Error;
    fn try_from(value: Value) -> Result<ItemKind, Self::Error> {
        TryInto::<String>::try_into(value)?.parse()
    }
}

impl TryFrom<Value> for Vec<ItemKind> {
    type Error = Error;
    fn try_from(value: Value) -> Result<Vec<ItemKind>, Self::Error> {
        let data: Vec<String> = value.try_into()?;
        let mut result = Vec::new();
        for d in data {
            result.push(TryInto::<String>::try_into(d)?.parse()?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::to_value;
    use super::{Error, ItemKind, Value, IEID, OID};
    use std::convert::TryInto;

    #[test]
    fn test_oid() {
        let oid: OID = "sensor:env/room1/temp1".parse().unwrap();
        assert_eq!(oid.id(), "temp1");
        assert_eq!(oid.full_id(), "env/room1/temp1");
        assert_eq!(oid.group().unwrap(), "env/room1");
        assert_eq!(oid.kind, ItemKind::Sensor);
        assert!("sensorx:env/temp1".parse::<OID>().is_err());
        assert!("sensorxenv/temp1".parse::<OID>().is_err());
        assert!("sensorxenv/:temp1".parse::<OID>().is_err());
        assert!("sensor|temp1".parse::<OID>().is_err());
        assert!("sensor:".parse::<OID>().is_err());
        let oid = OID::new0(ItemKind::Sensor, "tests/test1").unwrap();
        assert_eq!(oid.id(), "test1");
        assert_eq!(oid.group().unwrap(), "tests");
        assert_eq!(oid.kind(), ItemKind::Sensor);
        let oid = OID::new0(ItemKind::Sensor, "tests/room1/test1").unwrap();
        assert_eq!(oid.id(), "test1");
        assert_eq!(oid.group().unwrap(), "tests/room1");
        assert_eq!(oid.kind(), ItemKind::Sensor);
    }

    #[test]
    fn test_ieid() {
        assert!(IEID::new(1, 1) == IEID::new(1, 1));
        assert!(IEID::new(2, 1) > IEID::new(1, 9));
        assert!(IEID::new(2, 2) < IEID::new(3, 1));
        assert!(IEID::new(2, 4) > IEID::new(2, 2));
        assert!(IEID::new(2, 4) < IEID::new(2, 5));
    }

    #[test]
    fn test_try_into_vec() {
        let v = vec!["1", "2", "3"];
        let value = to_value(v.clone()).unwrap();
        let result: Vec<&str> = (&value).try_into().unwrap();
        let value2: Value = "1,2,3".into();
        assert_eq!(result, v);
        let result: Vec<&str> = (&value2).try_into().unwrap();
        assert_eq!(result, v);
    }

    #[test]
    fn test_try_into_bool() {
        assert!(TryInto::<bool>::try_into(Value::String("True".to_owned())).unwrap());
        assert!(TryInto::<bool>::try_into(Value::String("Trux".to_owned())).is_err());
        assert!(!TryInto::<bool>::try_into(Value::U64(0)).unwrap());
        assert!(TryInto::<bool>::try_into(Value::F64(1.0)).unwrap());
        assert!(TryInto::<bool>::try_into(Value::F64(2.0)).is_err());
    }

    #[test]
    fn test_err() {
        assert_eq!(format!("{}", Error::timeout()), "Timed out");
        assert_eq!(
            format!("{}", Error::not_found("test")),
            "Resource not found: test"
        );
    }
}
