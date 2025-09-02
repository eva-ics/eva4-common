use crate::registry;
use crate::Value;
use crate::{EResult, Error};
use busrt::rpc::{self, RpcClient, RpcHandlers};
#[cfg(all(feature = "openssl3", feature = "fips"))]
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::ffi::CString;
use std::fmt;
#[cfg(feature = "extended-value")]
use std::path::Path;
use std::sync::atomic;
use std::sync::Arc;
use std::time::Duration;

pub const SERVICE_CONFIG_VERSION: u16 = 4;

pub const SERVICE_PAYLOAD_PING: u8 = 0;
pub const SERVICE_PAYLOAD_INITIAL: u8 = 1;

#[cfg(all(feature = "openssl3", feature = "fips"))]
#[allow(dead_code)]
static FIPS_LOADED: OnceCell<()> = OnceCell::new();

#[cfg(any(
    feature = "openssl-vendored",
    feature = "openssl-no-fips",
    not(feature = "fips")
))]
pub fn enable_fips() -> EResult<()> {
    Err(Error::failed(
        "FIPS can not be enabled, consider using a native OS distribution",
    ))
}

#[cfg(not(any(feature = "openssl-vendored", feature = "openssl-no-fips")))]
#[cfg(feature = "fips")]
pub fn enable_fips() -> EResult<()> {
    #[cfg(feature = "openssl3")]
    {
        FIPS_LOADED
            .set(())
            .map_err(|_| Error::core("FIPS provided already loaded"))?;
        std::mem::forget(openssl::provider::Provider::load(None, "fips")?);
    }
    #[cfg(not(feature = "openssl3"))]
    openssl::fips::enable(true)?;
    Ok(())
}

pub struct Registry {
    id: String,
    rpc: Arc<RpcClient>,
}

impl Registry {
    #[inline]
    pub async fn key_set<V>(&self, key: &str, value: V) -> EResult<Value>
    where
        V: Serialize,
    {
        registry::key_set(
            &registry::format_svc_data_subkey(&self.id),
            key,
            value,
            &self.rpc,
        )
        .await
    }
    #[inline]
    pub async fn key_get(&self, key: &str) -> EResult<Value> {
        registry::key_get(&registry::format_svc_data_subkey(&self.id), key, &self.rpc).await
    }
    #[inline]
    pub async fn key_userdata_get(&self, key: &str) -> EResult<Value> {
        registry::key_get(registry::R_USER_DATA, key, &self.rpc).await
    }
    #[inline]
    pub async fn key_increment(&self, key: &str) -> EResult<i64> {
        registry::key_increment(&registry::format_svc_data_subkey(&self.id), key, &self.rpc).await
    }

    #[inline]
    pub async fn key_decrement(&self, key: &str) -> EResult<i64> {
        registry::key_decrement(&registry::format_svc_data_subkey(&self.id), key, &self.rpc).await
    }
    #[inline]
    pub async fn key_get_recursive(&self, key: &str) -> EResult<Vec<(String, Value)>> {
        registry::key_get_recursive(&registry::format_svc_data_subkey(&self.id), key, &self.rpc)
            .await
    }
    #[inline]
    pub async fn key_delete(&self, key: &str) -> EResult<Value> {
        registry::key_delete(&registry::format_svc_data_subkey(&self.id), key, &self.rpc).await
    }
    #[inline]
    pub async fn key_delete_recursive(&self, key: &str) -> EResult<Value> {
        registry::key_delete_recursive(&registry::format_svc_data_subkey(&self.id), key, &self.rpc)
            .await
    }
}

#[inline]
fn default_workers() -> u32 {
    1
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct RealtimeConfig {
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub cpu_ids: Vec<usize>,
    #[serde(default)]
    pub prealloc_heap: Option<usize>,
}

fn default_restart_delay() -> Duration {
    Duration::from_secs(2)
}

/// Initial properties for services
#[derive(Debug, Serialize, Deserialize)]
pub struct Initial {
    #[serde(rename = "version")]
    config_version: u16,
    system_name: String,
    id: String,
    command: String,
    #[serde(default)]
    prepare_command: Option<String>,
    data_path: String,
    timeout: Timeout,
    core: CoreInfo,
    bus: BusConfig,
    #[serde(default)]
    realtime: RealtimeConfig,
    #[serde(default)]
    config: Option<Value>,
    #[serde(default = "default_workers")]
    workers: u32,
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    react_to_fail: bool,
    #[serde(
        serialize_with = "crate::tools::serialize_atomic_bool",
        deserialize_with = "crate::tools::deserialize_atomic_bool"
    )]
    fail_mode: atomic::AtomicBool,
    #[serde(default)]
    fips: bool,
    #[serde(default)]
    call_tracing: bool,
    #[serde(
        default = "default_restart_delay",
        deserialize_with = "crate::tools::de_float_as_duration",
        serialize_with = "crate::tools::serialize_duration_as_f64"
    )]
    restart_delay: Duration,
}

impl Initial {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        system_name: &str,
        command: &str,
        prepare_command: Option<&str>,
        data_path: &str,
        timeout: &Timeout,
        core_info: CoreInfo,
        bus: BusConfig,
        config: Option<&Value>,
        workers: u32,
        user: Option<&str>,
        react_to_fail: bool,
        fips: bool,
        call_tracing: bool,
    ) -> Self {
        Self {
            config_version: SERVICE_CONFIG_VERSION,
            system_name: system_name.to_owned(),
            id: id.to_owned(),
            command: command.to_owned(),
            prepare_command: prepare_command.map(ToOwned::to_owned),
            data_path: data_path.to_owned(),
            timeout: timeout.clone(),
            core: core_info,
            bus,
            realtime: <_>::default(),
            config: config.cloned(),
            workers,
            user: user.map(ToOwned::to_owned),
            react_to_fail,
            fail_mode: atomic::AtomicBool::new(false),
            fips,
            call_tracing,
            restart_delay: default_restart_delay(),
        }
    }
    pub fn with_realtime(mut self, realtime: RealtimeConfig) -> Self {
        self.realtime = realtime;
        self
    }
    pub fn with_restart_delay(mut self, delay: Duration) -> Self {
        self.restart_delay = delay;
        self
    }
    #[inline]
    pub fn init(&self) -> EResult<()> {
        #[cfg(feature = "openssl-no-fips")]
        if self.fips {
            return Err(Error::not_implemented(
                "no FIPS 140 support, disable FIPS or switch to native package",
            ));
        }
        if self.fips {
            enable_fips()?;
        }
        Ok(())
    }
    #[inline]
    pub fn config_version(&self) -> u16 {
        self.config_version
    }
    #[inline]
    pub fn system_name(&self) -> &str {
        &self.system_name
    }
    #[inline]
    pub fn id(&self) -> &str {
        &self.id
    }
    #[inline]
    pub fn command(&self) -> &str {
        &self.command
    }
    pub fn realtime(&self) -> &RealtimeConfig {
        &self.realtime
    }
    #[inline]
    pub fn prepare_command(&self) -> Option<&str> {
        self.prepare_command.as_deref()
    }
    #[inline]
    pub fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }
    pub fn set_user(&mut self, user: Option<&str>) {
        self.user = user.map(ToOwned::to_owned);
    }
    pub fn set_id(&mut self, id: &str) {
        id.clone_into(&mut self.id);
    }
    #[inline]
    pub fn data_path(&self) -> Option<&str> {
        if let Some(ref user) = self.user {
            if user == "nobody" {
                return None;
            }
        }
        Some(&self.data_path)
    }
    #[inline]
    pub fn planned_data_path(&self) -> &str {
        &self.data_path
    }
    pub fn set_data_path(&mut self, path: &str) {
        path.clone_into(&mut self.data_path);
    }
    #[inline]
    pub fn timeout(&self) -> Duration {
        self.timeout
            .default
            .map_or(crate::DEFAULT_TIMEOUT, Duration::from_secs_f64)
    }
    #[inline]
    pub fn startup_timeout(&self) -> Duration {
        self.timeout
            .startup
            .map_or_else(|| self.timeout(), Duration::from_secs_f64)
    }
    #[inline]
    pub fn shutdown_timeout(&self) -> Duration {
        self.timeout
            .shutdown
            .map_or_else(|| self.timeout(), Duration::from_secs_f64)
    }
    #[inline]
    pub fn bus_timeout(&self) -> Duration {
        self.bus
            .timeout
            .map_or_else(|| self.timeout(), Duration::from_secs_f64)
    }
    #[inline]
    pub fn eva_build(&self) -> u64 {
        self.core.build
    }
    #[inline]
    pub fn eva_version(&self) -> &str {
        &self.core.version
    }
    #[inline]
    pub fn eapi_version(&self) -> u16 {
        self.core.eapi_verion
    }
    #[inline]
    pub fn eva_dir(&self) -> &str {
        &self.core.path
    }
    #[inline]
    pub fn eva_log_level(&self) -> u8 {
        self.core.log_level
    }
    #[inline]
    pub fn core_active(&self) -> bool {
        self.core.active
    }
    #[inline]
    pub fn call_tracing(&self) -> bool {
        self.call_tracing
    }
    #[inline]
    pub fn restart_delay(&self) -> Duration {
        self.restart_delay
    }
    #[inline]
    pub fn eva_log_level_filter(&self) -> log::LevelFilter {
        match self.core.log_level {
            crate::LOG_LEVEL_TRACE => log::LevelFilter::Trace,
            crate::LOG_LEVEL_DEBUG => log::LevelFilter::Debug,
            crate::LOG_LEVEL_WARN => log::LevelFilter::Warn,
            crate::LOG_LEVEL_ERROR => log::LevelFilter::Error,
            crate::LOG_LEVEL_OFF => log::LevelFilter::Off,
            _ => log::LevelFilter::Info,
        }
    }
    #[inline]
    pub fn bus_config(&self) -> EResult<busrt::ipc::Config> {
        if self.bus.tp == "native" {
            Ok(busrt::ipc::Config::new(&self.bus.path, &self.id)
                .buf_size(self.bus.buf_size)
                .buf_ttl(Duration::from_micros(self.bus.buf_ttl))
                .queue_size(self.bus.queue_size)
                .timeout(self.bus_timeout()))
        } else {
            Err(Error::not_implemented(format!(
                "bus type {} is not supported",
                self.bus.tp
            )))
        }
    }
    #[inline]
    pub fn bus_config_for_sub(&self, sub_id: &str) -> EResult<busrt::ipc::Config> {
        if self.bus.tp == "native" {
            Ok(
                busrt::ipc::Config::new(&self.bus.path, &format!("{}::{}", self.id, sub_id))
                    .buf_size(self.bus.buf_size)
                    .buf_ttl(Duration::from_micros(self.bus.buf_ttl))
                    .queue_size(self.bus.queue_size)
                    .timeout(self.bus_timeout()),
            )
        } else {
            Err(Error::not_implemented(format!(
                "bus type {} is not supported",
                self.bus.tp
            )))
        }
    }
    pub fn set_bus_path(&mut self, path: &str) {
        path.clone_into(&mut self.bus.path);
    }
    #[inline]
    pub fn bus_path(&self) -> &str {
        &self.bus.path
    }
    #[inline]
    pub fn config(&self) -> Option<&Value> {
        self.config.as_ref()
    }
    #[cfg(feature = "extended-value")]
    #[inline]
    pub async fn extend_config(&mut self, timeout: Duration, base: &Path) -> EResult<()> {
        self.config = if let Some(config) = self.config.take() {
            Some(config.extend(timeout, base).await?)
        } else {
            None
        };
        Ok(())
    }
    #[inline]
    pub fn workers(&self) -> u32 {
        self.workers
    }
    #[inline]
    pub fn bus_queue_size(&self) -> usize {
        self.bus.queue_size
    }
    #[inline]
    pub fn take_config(&mut self) -> Option<Value> {
        self.config.take()
    }
    #[inline]
    pub async fn init_rpc<R>(&self, handlers: R) -> EResult<Arc<RpcClient>>
    where
        R: RpcHandlers + Send + Sync + 'static,
    {
        self.init_rpc_opts(handlers, rpc::Options::default()).await
    }
    #[inline]
    pub async fn init_rpc_blocking<R>(&self, handlers: R) -> EResult<Arc<RpcClient>>
    where
        R: RpcHandlers + Send + Sync + 'static,
    {
        self.init_rpc_opts(
            handlers,
            rpc::Options::new()
                .blocking_notifications()
                .blocking_frames(),
        )
        .await
    }
    #[inline]
    pub async fn init_rpc_blocking_with_secondary<R>(
        &self,
        handlers: R,
    ) -> EResult<(Arc<RpcClient>, Arc<RpcClient>)>
    where
        R: RpcHandlers + Send + Sync + 'static,
    {
        let bus = self.init_bus_client().await?;
        let bus_secondary = bus.register_secondary().await?;
        let opts = rpc::Options::new()
            .blocking_notifications()
            .blocking_frames();
        let rpc = Arc::new(RpcClient::create(bus, handlers, opts.clone()));
        let rpc_secondary = Arc::new(RpcClient::create0(bus_secondary, opts));
        Ok((rpc, rpc_secondary))
    }
    pub async fn init_rpc_opts<R>(&self, handlers: R, opts: rpc::Options) -> EResult<Arc<RpcClient>>
    where
        R: RpcHandlers + Send + Sync + 'static,
    {
        let bus = self.init_bus_client().await?;
        let rpc = RpcClient::create(bus, handlers, opts);
        Ok(Arc::new(rpc))
    }
    pub async fn init_bus_client(&self) -> EResult<busrt::ipc::Client> {
        let bus = tokio::time::timeout(
            self.bus_timeout(),
            busrt::ipc::Client::connect(&self.bus_config()?),
        )
        .await??;
        Ok(bus)
    }
    pub async fn init_bus_client_sub(&self, sub_id: &str) -> EResult<busrt::ipc::Client> {
        let bus = tokio::time::timeout(
            self.bus_timeout(),
            busrt::ipc::Client::connect(&self.bus_config_for_sub(sub_id)?),
        )
        .await??;
        Ok(bus)
    }
    #[inline]
    pub fn init_registry(&self, rpc: &Arc<RpcClient>) -> Registry {
        Registry {
            id: self.id.clone(),
            rpc: rpc.clone(),
        }
    }
    #[inline]
    pub fn can_rtf(&self) -> bool {
        self.react_to_fail
    }
    #[inline]
    pub fn is_mode_normal(&self) -> bool {
        !self.fail_mode.load(atomic::Ordering::SeqCst)
    }
    #[inline]
    pub fn is_mode_rtf(&self) -> bool {
        self.fail_mode.load(atomic::Ordering::SeqCst)
    }
    #[inline]
    pub fn set_fail_mode(&self, mode: bool) {
        self.fail_mode.store(mode, atomic::Ordering::SeqCst);
    }
    #[cfg(target_os = "linux")]
    #[inline]
    pub fn drop_privileges(&self) -> EResult<()> {
        if let Some(ref user) = self.user {
            if !user.is_empty() {
                let u = get_system_user(user)?;
                if nix::unistd::getuid() != u.uid {
                    let c_user = CString::new(user.as_str()).map_err(|e| {
                        Error::failed(format!("Failed to parse user {}: {}", user, e))
                    })?;

                    let groups = nix::unistd::getgrouplist(&c_user, u.gid).map_err(|e| {
                        Error::failed(format!("Failed to get groups for user {}: {}", user, e))
                    })?;
                    nix::unistd::setgroups(&groups).map_err(|e| {
                        Error::failed(format!(
                            "Failed to switch the process groups for user {}: {}",
                            user, e
                        ))
                    })?;
                    nix::unistd::setgid(u.gid).map_err(|e| {
                        Error::failed(format!(
                            "Failed to switch the process group for user {}: {}",
                            user, e
                        ))
                    })?;
                    nix::unistd::setuid(u.uid).map_err(|e| {
                        Error::failed(format!(
                            "Failed to switch the process user to {}: {}",
                            user, e
                        ))
                    })?;
                }
            }
        }
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    #[inline]
    pub fn drop_privileges(&self) -> EResult<()> {
        eprintln(!"WARNING privileges not dropped");
        Ok(())
    }
    pub fn into_legacy_compat(mut self) -> Self {
        self.data_path = self.data_path().unwrap_or_default().to_owned();
        let user = self.user.take().unwrap_or_default();
        self.user.replace(user);
        let timeout = self
            .timeout
            .default
            .unwrap_or(crate::DEFAULT_TIMEOUT.as_secs_f64());
        self.timeout.default.replace(timeout);
        if self.timeout.startup.is_none() {
            self.timeout.startup.replace(timeout);
        }
        if self.timeout.shutdown.is_none() {
            self.timeout.shutdown.replace(timeout);
        }
        let config = self
            .take_config()
            .unwrap_or_else(|| Value::Map(<_>::default()));
        self.config.replace(config);
        self
    }
}

#[cfg(target_os = "linux")]
pub fn get_system_user(user: &str) -> EResult<nix::unistd::User> {
    let u = nix::unistd::User::from_name(user)
        .map_err(|e| Error::failed(format!("failed to get the system user {}: {}", user, e)))?
        .ok_or_else(|| Error::failed(format!("Failed to locate the system user {}", user)))?;
    Ok(u)
}

#[cfg(target_os = "linux")]
pub fn get_system_group(group: &str) -> EResult<nix::unistd::Group> {
    let g = nix::unistd::Group::from_name(group)
        .map_err(|e| Error::failed(format!("failed to get the system group {}: {}", group, e)))?
        .ok_or_else(|| Error::failed(format!("Failed to locate the system group {}", group)))?;
    Ok(g)
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Timeout {
    startup: Option<f64>,
    shutdown: Option<f64>,
    default: Option<f64>,
}

impl Timeout {
    pub fn offer(&mut self, timeout: f64) {
        if self.startup.is_none() {
            self.startup.replace(timeout);
        }
        if self.shutdown.is_none() {
            self.shutdown.replace(timeout);
        }
        if self.default.is_none() {
            self.default.replace(timeout);
        }
    }
    pub fn get(&self) -> Option<Duration> {
        self.default.map(Duration::from_secs_f64)
    }
    pub fn startup(&self) -> Option<Duration> {
        self.startup.map(Duration::from_secs_f64)
    }
    pub fn shutdown(&self) -> Option<Duration> {
        self.shutdown.map(Duration::from_secs_f64)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoreInfo {
    build: u64,
    version: String,
    eapi_verion: u16,
    path: String,
    log_level: u8,
    active: bool,
}

impl CoreInfo {
    pub fn new(
        build: u64,
        version: &str,
        eapi_verion: u16,
        path: &str,
        log_level: u8,
        active: bool,
    ) -> Self {
        Self {
            build,
            version: version.to_owned(),
            eapi_verion,
            path: path.to_owned(),
            log_level,
            active,
        }
    }
}

#[inline]
fn default_bus_type() -> String {
    "native".to_owned()
}

#[inline]
fn default_bus_buf_size() -> usize {
    busrt::DEFAULT_BUF_SIZE
}

#[allow(clippy::cast_possible_truncation)]
#[inline]
fn default_bus_buf_ttl() -> u64 {
    busrt::DEFAULT_BUF_TTL.as_micros() as u64
}

#[inline]
fn default_bus_queue_size() -> usize {
    busrt::DEFAULT_QUEUE_SIZE
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BusConfig {
    #[serde(rename = "type", default = "default_bus_type")]
    tp: String,
    path: String,
    timeout: Option<f64>,
    #[serde(default = "default_bus_buf_size")]
    buf_size: usize,
    #[serde(default = "default_bus_buf_ttl")]
    buf_ttl: u64, // microseconds
    #[serde(default = "default_bus_queue_size")]
    queue_size: usize,
    // deprecated field, as BUS/RT RPC uses timeout as a ping interval
    #[serde(rename = "ping_interval", skip_serializing, default)]
    _ping_interval: f64,
}

impl BusConfig {
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn set_path(&mut self, path: &str) {
        path.clone_into(&mut self.path);
    }
    pub fn offer_timeout(&mut self, timeout: f64) {
        if self.timeout.is_none() {
            self.timeout.replace(timeout);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodParamInfo {
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    #[serde(default)]
    pub description: String,
    pub params: HashMap<String, MethodParamInfo>,
}

/// info-structure only, can be used by clients for auto-completion
pub struct ServiceMethod {
    pub name: String,
    pub description: String,
    pub params: HashMap<String, MethodParamInfo>,
}

impl ServiceMethod {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            description: String::new(),
            params: <_>::default(),
        }
    }
    pub fn description(mut self, desc: &str) -> Self {
        desc.clone_into(&mut self.description);
        self
    }
    pub fn required(mut self, name: &str) -> Self {
        self.params
            .insert(name.to_owned(), MethodParamInfo { required: true });
        self
    }
    pub fn optional(mut self, name: &str) -> Self {
        self.params
            .insert(name.to_owned(), MethodParamInfo { required: false });
        self
    }
}

/// Returned by all services on "info" RPC command
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceInfo {
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub methods: HashMap<String, MethodInfo>,
}

impl ServiceInfo {
    pub fn new(author: &str, version: &str, description: &str) -> Self {
        Self {
            author: author.to_owned(),
            version: version.to_owned(),
            description: description.to_owned(),
            methods: <_>::default(),
        }
    }
    #[inline]
    pub fn add_method(&mut self, method: ServiceMethod) {
        self.methods.insert(
            method.name,
            MethodInfo {
                description: method.description,
                params: method.params,
            },
        );
    }
}

/// Used by services to announce their status (for "*")
#[derive(Serialize, Deserialize)]
pub struct ServiceStatusBroadcastEvent {
    pub status: ServiceStatusBroadcast,
}

impl ServiceStatusBroadcastEvent {
    #[inline]
    pub fn ready() -> Self {
        Self {
            status: ServiceStatusBroadcast::Ready,
        }
    }
    #[inline]
    pub fn terminating() -> Self {
        Self {
            status: ServiceStatusBroadcast::Terminating,
        }
    }
}

/// Used by services and the core to notify about its state
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum ServiceStatusBroadcast {
    Starting = 0,
    Ready = 1,
    Terminating = 0xef,
    Unknown = 0xff,
}

impl fmt::Display for ServiceStatusBroadcast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ServiceStatusBroadcast::Starting => "starting",
                ServiceStatusBroadcast::Ready => "ready",
                ServiceStatusBroadcast::Terminating => "terminating",
                ServiceStatusBroadcast::Unknown => "unknown",
            }
        )
    }
}
