use crate::EResult;
use crate::{Error, ErrorKind};
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tokio::sync::{Mutex, Notify};

#[macro_export]
macro_rules! periodic_worker {
    ($name: expr, $target: expr, $interval: expr) => {
        let (trigger, _fut) = bmart::worker!($target);
        $crate::workers::recreate_scheduler($name, trigger, $interval, false).await?;
    };
    ($name: expr, $target: expr, $interval: expr, $($arg:tt)+) => {
        let (trigger, _fut) = bmart::worker!($target, $($arg)+);
        $crate::workers::recreate_scheduler($name, trigger, $interval, false).await?;
    };
}

#[macro_export]
macro_rules! cleaner {
    ($name: expr, $target: expr, $interval: expr) => {
        $crate::periodic_worker!(&format!("cleaner::{}", $name), $target, $interval);
    };
    ($name: expr, $target: expr, $interval: expr, $($arg:tt)+) => {
        $crate::periodic_worker!(&format!("cleaner::{}", $name), $target, $interval, $($arg)+);
    };
}

impl From<bmart::Error> for Error {
    fn from(error: bmart::Error) -> Self {
        match error.kind {
            bmart::ErrorKind::Duplicate => {
                Error::newc(ErrorKind::ResourceAlreadyExists, error.message)
            }
            bmart::ErrorKind::NotFound => Error::newc(ErrorKind::ResourceNotFound, error.message),
            bmart::ErrorKind::Internal => Error::newc(ErrorKind::CoreError, error.message),
            bmart::ErrorKind::InvalidData => Error::newc(ErrorKind::InvalidData, error.message),
            bmart::ErrorKind::Timeout => Error::newc(ErrorKind::Timeout, error.message),
        }
    }
}

static WORKERS: LazyLock<Mutex<bmart::workers::WorkerFactory>> =
    LazyLock::new(|| Mutex::new(bmart::workers::WorkerFactory::new()));

/// # Errors
///
/// Will return `Err` if failed to recreate the worker
pub async fn recreate_scheduler(
    worker_id: &str,
    trigger: Arc<Notify>,
    interval: Duration,
    instant: bool,
) -> EResult<()> {
    WORKERS
        .lock()
        .await
        .recreate_scheduler(worker_id, trigger, interval, instant)
        .map_err(Into::into)
}

/// # Errors
///
/// Will return `Err` if the worker is not found
pub async fn destroy_scheduler(worker_id: &str) -> EResult<()> {
    WORKERS
        .lock()
        .await
        .destroy_scheduler(worker_id)
        .map_err(Into::into)
}
