use crate::events::LOG_INPUT_TOPIC;
use crate::{EResult, Error};
use busrt::client::AsyncClient;
use busrt::QoS;
use lazy_static::lazy_static;
use log::{Level, LevelFilter, Log};
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::Arc;

lazy_static! {
    static ref LOG_TOPICS: HashMap<Level, String> = {
        let mut topics = HashMap::new();
        topics.insert(Level::Trace, format!("{}{}", LOG_INPUT_TOPIC, "trace"));
        topics.insert(Level::Debug, format!("{}{}", LOG_INPUT_TOPIC, "debug"));
        topics.insert(Level::Info, format!("{}{}", LOG_INPUT_TOPIC, "info"));
        topics.insert(Level::Warn, format!("{}{}", LOG_INPUT_TOPIC, "warn"));
        topics.insert(Level::Error, format!("{}{}", LOG_INPUT_TOPIC, "error"));
        topics
    };
    static ref LOG_TX: OnceCell<async_channel::Sender<(log::Level, String)>> = <_>::default();
}

static BUS_LOGGER: BusLogger = BusLogger {};

struct BusLogger {}

impl Log for BusLogger {
    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        !metadata.target().starts_with("busrt::") && !metadata.target().starts_with("mio::")
    }
    #[inline]
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            if let Some(tx) = LOG_TX.get() {
                let _r = tx.try_send((record.level(), format!("{}", record.args())));
            }
        }
    }
    #[inline]
    fn flush(&self) {}
}

async fn handle_logs<C>(
    client: Arc<tokio::sync::Mutex<C>>,
    rx: async_channel::Receiver<(Level, String)>,
) where
    C: ?Sized + AsyncClient,
{
    while let Ok((level, message)) = rx.recv().await {
        if let Err(e) = client
            .lock()
            .await
            .publish(
                LOG_TOPICS.get(&level).unwrap(),
                message.as_bytes().into(),
                QoS::No,
            )
            .await
        {
            eprintln!("{}", e);
        }
    }
}

/// Must not be called twice
///
pub fn init_bus<C>(
    client: Arc<tokio::sync::Mutex<C>>,
    queue_size: usize,
    filter: LevelFilter,
) -> EResult<()>
where
    C: ?Sized + AsyncClient + 'static,
{
    let (tx, rx) = async_channel::bounded(queue_size);
    LOG_TX
        .set(tx)
        .map_err(|_| Error::failed("Unable to set LOG_TX"))?;
    tokio::spawn(async move {
        handle_logs(client, rx).await;
    });
    log::set_logger(&BUS_LOGGER)
        .map(|()| log::set_max_level(filter))
        .map_err(Error::failed)?;
    Ok(())
}
