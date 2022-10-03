/// experimental shared lib extensions, not used yet
use crate::EvaError;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

const CLASS_PHI: u16 = 10;
const CLASS_GENERIC_PLUGIN: u16 = 20;
const CLASS_AUTH_MODULE: u16 = 30;

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Debug)]
#[repr(u16)]
pub enum EeExtensionClass {
    Phi = CLASS_PHI,
    GenericPlugin = CLASS_GENERIC_PLUGIN,
    AuthModule = CLASS_AUTH_MODULE,
}

pub mod prelude {
    pub use super::comm::ee_async_id;
    pub use super::comm::ee_decode;
    pub use super::comm::ee_encode;
    pub use super::comm::ee_send_async_result;
    pub use super::comm::EeFrame;
    pub use super::comm::EeFrameConv;
    pub use super::comm::EeResult;
    pub use super::comm::EeResultAsync;
    pub use super::EeExtensionClass;
    pub use super::EeMetadata;
    pub use crate::ee_commons;
    pub use crate::ee_get_option;
    pub use crate::ee_result;
    pub use crate::ee_unwrap_or;
}

impl From<libloading::Error> for EvaError {
    fn from(err: libloading::Error) -> EvaError {
        EvaError::failed(err)
    }
}

#[macro_export]
macro_rules! ee_unwrap_or {
    ($result: expr) => {
        match $result {
            Ok(v) => v,
            Err(e) => {
                return e.into();
            }
        }
    };
}

#[macro_export]
macro_rules! ee_get_option {
    ($data: expr, $key: expr) => {
        match $data.remove($key) {
            Some(v) => ee_unwrap_or!(v.try_into()),
            None => {
                return EvaError::invalid_data(format!("The parameter is missing: {}", $key))
                    .into();
            }
        }
    };
    ($data: expr, $key: expr, $v: expr) => {
        match $data.remove($key) {
            Some(v) => ee_unwrap_or!(v.try_into()),
            None => $v,
        }
    };
}

#[macro_export]
macro_rules! ee_commons {
    () => {
        #[no_mangle]
        pub unsafe fn set_logger_fn(func: fn($crate::ext::comm::EeFrame), level: u8) -> EeResult {
            $crate::ext::logger::set_logger_fn(func, level)
        }

        #[no_mangle]
        pub unsafe fn set_async_result_fn(
            func: fn(u32, u32, $crate::ext::comm::EeFrame),
        ) -> EeResult {
            $crate::ext::comm::set_async_result_fn(func)
        }

        #[no_mangle]
        pub unsafe fn free_result() -> EeResult {
            $crate::ext::free_result()
        }

        #[no_mangle]
        pub unsafe fn get_comm_version() -> u16 {
            $crate::ext::comm::COMM_VERSION
        }

        #[no_mangle]
        pub fn set_id(id: u32) -> EeResult {
            $crate::ext::comm::set_id(id);
            EeResult::ok()
        }

        #[no_mangle]
        pub unsafe fn set_name(data: EeFrame) -> EeResult {
            let name = ee_unwrap_or!(data.decode());
            $crate::ext::logger::set_name(name);
            EeResult::ok()
        }
    };
}

#[macro_export]
macro_rules! ee_result {
    ($result: expr) => {
        match $result {
            Ok(v) => v.to_eeframe(),
            Err(e) => e.into(),
        }
    };
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EeMetadata {
    pub class: EeExtensionClass,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub copyright: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: u32,
    pub api: u32,
}

pub unsafe fn free_result() -> comm::EeResult {
    comm::clear_data();
    comm::EeResult::ok()
}

pub mod comm {

    use crate::{EvaError, EvaErrorKind};
    use serde::{Deserialize, Serialize};
    use std::cell::RefCell;
    use std::sync::{atomic, Mutex};

    pub const RESULT_OK: i16 = 0;
    pub const COMM_VERSION: u16 = 1;

    const ERR_UNSUPPORTED_COMM_VERSION: &str = "Unsupported comm version";

    pub fn version_supported(ver: u16) -> Result<(), EvaError> {
        if ver > COMM_VERSION {
            Err(EvaError::unsupported(ERR_UNSUPPORTED_COMM_VERSION))
        } else {
            Ok(())
        }
    }

    #[repr(C)]
    pub struct EeFrame {
        code: i16,           // code (used for errors only)
        length: u32,         // data length
        data_ptr: *const u8, // raw pointer to the frame data
    }

    impl EeFrame {
        pub fn decode<'a, T: Deserialize<'a>>(self) -> Result<T, EvaError> {
            unsafe { ee_decode(COMM_VERSION, self) }
        }
        pub fn decode_ver<'a, T: Deserialize<'a>>(self, comm_ver: u16) -> Result<T, EvaError> {
            unsafe { ee_decode(comm_ver, self) }
        }
        pub fn ok() -> EeFrame {
            unsafe { ee_encode(COMM_VERSION, RESULT_OK, &None::<u32>).unwrap() }
        }
    }

    impl From<EvaError> for EeFrame {
        fn from(err: EvaError) -> EeFrame {
            err.to_eeframe()
        }
    }

    #[repr(C)]
    pub struct EeResult(i16);

    impl EeResult {
        pub fn decode_ver(&self, comm_ver: u16) -> Result<(), EvaError> {
            if comm_ver != 1 {
                return Err(EvaError::unsupported(ERR_UNSUPPORTED_COMM_VERSION));
            }
            if self.0 == RESULT_OK {
                Ok(())
            } else {
                Err(EvaError::new0(self.0.into()))
            }
        }
        pub fn decode(&self) -> Result<(), EvaError> {
            self.decode_ver(COMM_VERSION)
        }
        pub fn ok() -> Self {
            Self(RESULT_OK)
        }
    }

    impl From<EvaError> for EeResult {
        fn from(err: EvaError) -> EeResult {
            EeResult(err.kind as i16)
        }
    }

    impl From<EvaErrorKind> for EeResult {
        fn from(kind: EvaErrorKind) -> EeResult {
            EeResult(kind as i16)
        }
    }

    #[repr(C)]
    pub struct EeResultAsync(i16, u32);

    impl EeResultAsync {
        pub fn decode_ver(&self, comm_ver: u16) -> Result<u32, EvaError> {
            if comm_ver != 1 {
                return Err(EvaError::unsupported(ERR_UNSUPPORTED_COMM_VERSION));
            }
            if self.0 == RESULT_OK {
                Ok(self.1)
            } else {
                Err(EvaError::new0(self.0.into()))
            }
        }
        pub fn decode(&self) -> Result<u32, EvaError> {
            self.decode_ver(COMM_VERSION)
        }
        pub fn ok(async_id: u32) -> Self {
            Self(RESULT_OK, async_id)
        }
    }

    impl From<EvaError> for EeResultAsync {
        fn from(err: EvaError) -> EeResultAsync {
            EeResultAsync(err.kind as i16, 0)
        }
    }

    impl From<EvaErrorKind> for EeResultAsync {
        fn from(kind: EvaErrorKind) -> EeResultAsync {
            EeResultAsync(kind as i16, 0)
        }
    }

    #[repr(C)]
    pub struct DataFrame {
        data: Vec<u8>,
    }

    trait EeErrorConv {
        fn to_eeframe(&self) -> EeFrame;
        fn to_eeframe_ver(&self, comm_ver: u16) -> EeFrame;
    }

    impl EeErrorConv for EvaError {
        fn to_eeframe(&self) -> EeFrame {
            unsafe { ee_encode(COMM_VERSION, self.kind as i16, &self.message) }.unwrap()
        }
        fn to_eeframe_ver(&self, comm_ver: u16) -> EeFrame {
            unsafe { ee_encode(comm_ver, self.kind as i16, &self.message) }.unwrap()
        }
    }

    pub trait EeFrameConv {
        fn ee_encode(&self) -> Result<EeFrame, EvaError>;
        fn ee_encode_ver(&self, comm_ver: u16) -> Result<EeFrame, EvaError>;
        fn to_eeframe(&self) -> EeFrame;
        fn to_eeframe_ver(&self, comm_ver: u16) -> EeFrame;
    }

    macro_rules! unwrap_or_err_frame {
        ($comm_ver: expr, $result: expr) => {
            $result.unwrap_or_else(|e| e.to_eeframe_ver($comm_ver))
        };
    }

    impl<T> EeFrameConv for T
    where
        T: Serialize,
    {
        fn ee_encode(&self) -> Result<EeFrame, EvaError> {
            unsafe { ee_encode(COMM_VERSION, 0, self) }
        }
        fn ee_encode_ver(&self, comm_ver: u16) -> Result<EeFrame, EvaError> {
            unsafe { ee_encode(comm_ver, 0, self) }
        }
        fn to_eeframe(&self) -> EeFrame {
            unwrap_or_err_frame!(COMM_VERSION, unsafe { ee_encode(COMM_VERSION, 0, self) })
        }
        fn to_eeframe_ver(&self, comm_ver: u16) -> EeFrame {
            unwrap_or_err_frame!(comm_ver, unsafe { ee_encode(comm_ver, 0, self) })
        }
    }

    impl DataFrame {
        pub fn with_data(data: Vec<u8>) -> Self {
            Self { data }
        }
        pub fn set_data(&mut self, data: Vec<u8>) {
            self.data = data;
        }
        pub fn as_frame(&self, code: i16) -> EeFrame {
            EeFrame {
                code,
                length: self.data.len() as u32,
                data_ptr: self.data.as_ptr(),
            }
        }
        pub fn clear(&mut self) {
            self.data.clear();
        }
    }

    static mut DATA: DataFrame = DataFrame { data: Vec::new() };
    static mut ASYNC_ID: u32 = 0;

    static ID: atomic::AtomicU32 = atomic::AtomicU32::new(0);

    pub fn ee_async_id() -> u32 {
        unsafe {
            if ASYNC_ID == std::u32::MAX {
                ASYNC_ID = 0;
            }
            ASYNC_ID += 1;
            ASYNC_ID
        }
    }

    thread_local! {
        pub static TDATA: RefCell<DataFrame> = RefCell::new(DataFrame { data: Vec::new() });
    }

    lazy_static! {
        static ref ASYNC_RESULT: Mutex<Option<fn(u32, u32, EeFrame)>> = Mutex::new(None);
    }

    pub unsafe fn set_async_result_fn(func: fn(u32, u32, EeFrame)) -> EeResult {
        ASYNC_RESULT.lock().unwrap().replace(func);
        EeResult::ok()
    }

    pub fn ee_send_async_result<T: Serialize>(async_id: u32, result: Result<T, EvaError>) {
        if let Some(async_result) = ASYNC_RESULT.lock().unwrap().as_ref() {
            TDATA.with(|cell| {
                let id = ID.load(atomic::Ordering::SeqCst);
                let mut tdata = cell.borrow_mut();
                match result {
                    Ok(ref v) => {
                        tdata.data = rmp_serde::to_vec_named(v).unwrap();
                        async_result(id, async_id, tdata.as_frame(RESULT_OK));
                    }
                    Err(e) => {
                        tdata.data = rmp_serde::to_vec_named(&e.message).unwrap();
                        async_result(id, async_id, tdata.as_frame(e.kind as i16));
                    }
                }
                tdata.clear();
            });
        } else {
            panic!("async result function is not set");
        }
    }

    pub fn set_id(id: u32) {
        ID.store(id, atomic::Ordering::SeqCst);
    }

    pub unsafe fn ee_encode<T: Serialize>(
        comm_ver: u16,
        code: i16,
        data: &T,
    ) -> Result<EeFrame, EvaError> {
        if comm_ver != 1 {
            return Err(EvaError::unsupported(ERR_UNSUPPORTED_COMM_VERSION));
        }
        DATA = DataFrame::with_data(rmp_serde::to_vec_named(data)?);
        Ok(DATA.as_frame(code))
    }

    pub unsafe fn ee_decode<'de, T: Deserialize<'de>>(
        comm_ver: u16,
        frame: EeFrame,
    ) -> Result<T, EvaError> {
        if comm_ver != 1 {
            return Err(EvaError::unsupported(ERR_UNSUPPORTED_COMM_VERSION));
        }
        let data_slice = std::slice::from_raw_parts(frame.data_ptr, frame.length as usize);
        if frame.code == RESULT_OK {
            Ok(rmp_serde::from_slice(data_slice)?)
        } else {
            let message: Option<String> = rmp_serde::from_slice(data_slice)?;
            Err(EvaError::newc(frame.code.into(), message))
        }
    }

    pub unsafe fn clear_data() {
        DATA.clear();
    }
}

pub mod logger {
    use super::comm::{EeFrame, EeResult, RESULT_OK};
    use log::{debug, error, info, trace, warn};
    use log::{Level, LevelFilter, Log, Record};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct LogRecord {
        #[serde(rename = "l")]
        level: u8,
        #[serde(rename = "m")]
        message: String,
    }

    impl LogRecord {
        pub fn send(&self) {
            match self.level {
                crate::LOG_LEVEL_TRACE => trace!("{}", self.message),
                crate::LOG_LEVEL_DEBUG => debug!("{}", self.message),
                crate::LOG_LEVEL_WARN => warn!("{}", self.message),
                crate::LOG_LEVEL_ERROR => error!("{}", self.message),
                _ => info!("{}", self.message),
            }
        }
    }

    static mut LOGGER: Logger = Logger {
        name: String::new(),
        logger_function: None,
    };

    pub struct Logger {
        logger_function: Option<fn(EeFrame)>,
        name: String,
    }

    impl Logger {
        fn set_logger_function(&mut self, func: fn(EeFrame)) {
            self.logger_function = Some(func);
        }
        fn set_name(&mut self, name: &str) {
            self.name = name.to_owned();
        }
    }

    impl Log for Logger {
        fn enabled(&self, _metadata: &log::Metadata) -> bool {
            true
        }
        fn log(&self, record: &Record) {
            crate::ext::comm::TDATA.with(|cell| {
                let mut tdata = cell.borrow_mut();
                let log_record = LogRecord {
                    level: {
                        match record.level() {
                            Level::Trace => crate::LOG_LEVEL_TRACE,
                            Level::Debug => crate::LOG_LEVEL_DEBUG,
                            Level::Info => crate::LOG_LEVEL_INFO,
                            Level::Warn => crate::LOG_LEVEL_WARN,
                            Level::Error => crate::LOG_LEVEL_ERROR,
                        }
                    },
                    message: format!("{}: {}", self.name, record.args()),
                };
                match rmp_serde::to_vec_named(&log_record) {
                    Ok(data) => {
                        tdata.set_data(data);
                        (self.logger_function.unwrap())(tdata.as_frame(RESULT_OK));
                        tdata.clear();
                    }
                    Err(e) => eprintln!("{} unable to send log record frame: {}", self.name, e),
                }
            });
        }
        fn flush(&self) {}
    }

    fn get_level_filter(level: u8) -> LevelFilter {
        match level {
            crate::LOG_LEVEL_TRACE => LevelFilter::Trace,
            crate::LOG_LEVEL_DEBUG => LevelFilter::Debug,
            crate::LOG_LEVEL_WARN => LevelFilter::Warn,
            crate::LOG_LEVEL_ERROR => LevelFilter::Error,
            _ => LevelFilter::Info,
        }
    }

    pub unsafe fn set_logger_fn(func: fn(EeFrame), level: u8) -> EeResult {
        LOGGER.set_logger_function(func);
        LOGGER.set_name("noname_extension");
        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(get_level_filter(level)))
            .unwrap();
        EeResult::ok()
    }

    pub unsafe fn set_name(name: &str) {
        LOGGER.set_name(name);
    }
}

pub mod auth {

    use serde::{Deserialize, Serialize};
    use std::time::Duration;

    /// Authentication request
    ///
    /// Timeout is in nanoseconds
    #[derive(Debug, Serialize, Deserialize, Clone)]
    pub struct Request {
        #[serde(rename = "l")]
        login: Option<String>,
        #[serde(rename = "p")]
        password: Option<String>,
        #[serde(rename = "t")]
        timeout: u64, // nanoseconds
    }

    impl Request {
        pub fn new(login: String, password: String, timeout: Duration) -> Self {
            Self {
                login: Some(login),
                password: Some(password),
                timeout: timeout.as_nanos() as u64,
            }
        }
        pub fn take_login(&mut self) -> String {
            self.login.take().unwrap()
        }
        pub fn take_password(&mut self) -> String {
            self.password.take().unwrap()
        }
        pub fn timeout_as_duration(&self) -> Duration {
            Duration::from_nanos(self.timeout)
        }
        pub fn timeout_as_nanos(&self) -> u64 {
            self.timeout
        }
    }
}
