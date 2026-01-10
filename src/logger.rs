use crate::events::{LOG_CALL_TRACE_TOPIC, LOG_INPUT_TOPIC};
use crate::payload::pack;
use crate::{EResult, Error};
use busrt::QoS;
use busrt::client::AsyncClient;
use log::{Level, LevelFilter, Log};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, OnceLock};
use std::time::Duration;
use std::time::Instant;
use uuid::Uuid;

const MSG_MAX_REPEAT_DELAY: Duration = Duration::from_millis(100);

tokio::task_local! {
    pub static CALL_TRACE_ID: Option<Uuid>;
}

#[derive(Serialize)]
pub struct TraceMessage {
    l: u8,
    msg: Arc<String>,
}

static LOG_TOPICS: LazyLock<HashMap<Level, String>> = LazyLock::new(|| {
    let mut topics = HashMap::new();
    topics.insert(Level::Trace, format!("{}{}", LOG_INPUT_TOPIC, "trace"));
    topics.insert(Level::Debug, format!("{}{}", LOG_INPUT_TOPIC, "debug"));
    topics.insert(Level::Info, format!("{}{}", LOG_INPUT_TOPIC, "info"));
    topics.insert(Level::Warn, format!("{}{}", LOG_INPUT_TOPIC, "warn"));
    topics.insert(Level::Error, format!("{}{}", LOG_INPUT_TOPIC, "error"));
    topics
});
static LOG_TX: OnceLock<async_channel::Sender<(log::Level, Arc<String>)>> = OnceLock::new();
static TRACE_TX: OnceLock<async_channel::Sender<(TraceMessage, Uuid)>> = OnceLock::new();

static BUS_LOGGER: BusLogger = BusLogger {
    log_filter: OnceLock::new(),
    prev_message: parking_lot::Mutex::new(None),
};

struct LogMessage {
    level: log::Level,
    message: Arc<String>,
    t: Instant,
}

struct BusLogger {
    log_filter: OnceLock<LevelFilter>,
    prev_message: parking_lot::Mutex<Option<LogMessage>>,
}

impl Log for BusLogger {
    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        !metadata.target().starts_with("busrt::") && !metadata.target().starts_with("mio::")
    }
    #[inline]
    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let mut message: Option<Arc<String>> = None;
            macro_rules! format_msg {
                () => {{
                    if message.is_none() {
                        message.replace(Arc::new(record.args().to_string()));
                    }
                    message.as_ref().unwrap().clone()
                }};
            }
            let trid: Option<Uuid> = CALL_TRACE_ID.try_with(Clone::clone).unwrap_or_default();
            if let Some(trace_id) = trid
                && let Some(tx) = TRACE_TX.get()
            {
                let _r = tx.try_send((
                    TraceMessage {
                        l: crate::log_level_code(record.level()),
                        msg: format_msg!(),
                    },
                    trace_id,
                ));
            }
            if let Some(tx) = LOG_TX.get() {
                let level = record.level();
                if level <= *self.log_filter.get().unwrap() {
                    let msg: Arc<String> = format_msg!();
                    {
                        let mut prev = self.prev_message.lock();
                        // ignore messages wich repeat too fast
                        if let Some(p) = prev.as_mut()
                            && p.level == level
                            && p.message == msg
                            && p.t.elapsed() < MSG_MAX_REPEAT_DELAY
                        {
                            return;
                        }
                        prev.replace(LogMessage {
                            level,
                            message: msg.clone(),
                            t: Instant::now(),
                        });
                    }
                    let _r = tx.try_send((level, msg));
                }
            }
        }
    }
    #[inline]
    fn flush(&self) {}
}

async fn handle_logs<C>(
    client: Arc<tokio::sync::Mutex<C>>,
    rx: async_channel::Receiver<(Level, Arc<String>)>,
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

async fn handle_traces<C>(
    client: Arc<tokio::sync::Mutex<C>>,
    rx: async_channel::Receiver<(TraceMessage, Uuid)>,
) where
    C: ?Sized + AsyncClient,
{
    while let Ok((trace_message, trace_id)) = rx.recv().await {
        let trace_topic = format!("{}{}", LOG_CALL_TRACE_TOPIC, trace_id);
        match pack(&trace_message) {
            Ok(payload) => {
                if let Err(e) = client
                    .lock()
                    .await
                    .publish(&trace_topic, payload.into(), QoS::No)
                    .await
                {
                    eprintln!("{}", e);
                }
            }
            Err(e) => eprintln!("{}", e),
        }
    }
}

/// Must not be called twice
///
pub fn init_bus<C>(
    client: Arc<tokio::sync::Mutex<C>>,
    queue_size: usize,
    filter: LevelFilter,
    call_tracing: bool,
) -> EResult<()>
where
    C: ?Sized + AsyncClient + 'static,
{
    let (tx, rx) = async_channel::bounded(queue_size);
    LOG_TX
        .set(tx)
        .map_err(|_| Error::failed("Unable to set LOG_TX"))?;
    let cl = client.clone();
    tokio::spawn(async move {
        handle_logs(cl, rx).await;
    });
    if call_tracing {
        let (tx, rx) = async_channel::bounded(queue_size);
        TRACE_TX
            .set(tx)
            .map_err(|_| Error::failed("Unable to set TRACE_TX"))?;
        tokio::spawn(async move {
            handle_traces(client, rx).await;
        });
    }
    BUS_LOGGER
        .log_filter
        .set(filter)
        .map_err(|_| Error::failed("Unable to set BUS_LOGGER filter"))?;
    log::set_logger(&BUS_LOGGER)
        .map(|()| {
            log::set_max_level(if call_tracing {
                LevelFilter::Trace
            } else {
                filter
            });
        })
        .map_err(Error::failed)?;
    Ok(())
}
