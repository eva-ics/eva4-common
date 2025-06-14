use crate::acl::OIDMaskList;
use crate::value::{Value, ValueOption, ValueOptionOwned};
use crate::{EResult, Error};
use crate::{ItemStatus, IEID, OID};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::time::Duration;

pub const RAW_STATE_TOPIC: &str = "RAW/";
pub const RAW_STATE_BULK_TOPIC: &str = "RAW";
pub const LOCAL_STATE_TOPIC: &str = "ST/LOC/";
pub const REMOTE_STATE_TOPIC: &str = "ST/REM/";
pub const REMOTE_ARCHIVE_STATE_TOPIC: &str = "ST/RAR/";
pub const ANY_STATE_TOPIC: &str = "ST/+/";
pub const REPLICATION_STATE_TOPIC: &str = "RPL/ST/";
pub const REPLICATION_INVENTORY_TOPIC: &str = "RPL/INVENTORY/";
pub const REPLICATION_NODE_STATE_TOPIC: &str = "RPL/NODE/";
pub const LOG_INPUT_TOPIC: &str = "LOG/IN/";
pub const LOG_EVENT_TOPIC: &str = "LOG/EV/";
pub const LOG_CALL_TRACE_TOPIC: &str = "LOG/TR/";
pub const SERVICE_STATUS_TOPIC: &str = "SVC/ST";
pub const AAA_ACL_TOPIC: &str = "AAA/ACL/";
pub const AAA_KEY_TOPIC: &str = "AAA/KEY/";
pub const AAA_USER_TOPIC: &str = "AAA/USER/";

#[derive(Debug, Copy, Clone)]
#[repr(i8)]
pub enum NodeStatus {
    Online = 1,
    Offline = 0,
    Removed = -1,
}

impl NodeStatus {
    fn as_str(&self) -> &str {
        match self {
            NodeStatus::Online => "online",
            NodeStatus::Offline => "offline",
            NodeStatus::Removed => "removed",
        }
    }
}

impl FromStr for NodeStatus {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "online" => Ok(NodeStatus::Online),
            "offline" => Ok(NodeStatus::Offline),
            "removed" => Ok(NodeStatus::Removed),
            _ => Err(Error::invalid_data(format!("Invalid node status: {}", s))),
        }
    }
}

/// submitted to RPL/NODE/<name>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStateEvent {
    pub status: NodeStatus,
    #[serde(default)]
    pub info: Option<NodeInfo>,
    #[serde(
        default,
        serialize_with = "crate::tools::serialize_opt_duration_as_f64",
        deserialize_with = "crate::tools::de_opt_float_as_duration"
    )]
    pub timeout: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub build: u64,
    pub version: String,
}

impl Serialize for NodeStatus {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for NodeStatus {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<NodeStatus, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default, Ord, PartialOrd)]
pub enum Force {
    #[default]
    None,
    // Update force behavior: always updates item state even if the previous is the same, updates
    // lvar state even if its status is 0
    Update,
    /// Full force behavior: does the same as Update, but also updates the item state even if the
    /// the item is disabled
    Full,
}

impl Force {
    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, Force::None)
    }
    #[inline]
    pub fn is_weak(&self) -> bool {
        matches!(self, Force::Update)
    }
    #[inline]
    pub fn is_full(&self) -> bool {
        matches!(self, Force::Full)
    }
    #[inline]
    pub fn is_any(&self) -> bool {
        !self.is_none()
    }
}

impl Serialize for Force {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Force::None => serializer.serialize_bool(false),
            Force::Full => serializer.serialize_bool(true),
            Force::Update => serializer.serialize_str("update"),
        }
    }
}

impl FromStr for Force {
    type Err = Error;

    fn from_str(input: &str) -> Result<Force, Self::Err> {
        match input.to_lowercase().as_str() {
            "none" => Ok(Force::None),
            "full" => Ok(Force::Full),
            "update" | "weak" => Ok(Force::Update),
            _ => Err(Error::invalid_data(format!(
                "Invalid force value: {}",
                input
            ))),
        }
    }
}

impl<'de> Deserialize<'de> for Force {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ForceVisitor;

        impl serde::de::Visitor<'_> for ForceVisitor {
            type Value = Force;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a boolean or a string representing a force")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Force, E>
            where
                E: serde::de::Error,
            {
                Ok(if value { Force::Full } else { Force::None })
            }

            fn visit_borrowed_str<E>(self, value: &str) -> Result<Force, E>
            where
                E: serde::de::Error,
            {
                value.parse().map_err(serde::de::Error::custom)
            }

            fn visit_str<E>(self, value: &str) -> Result<Force, E>
            where
                E: serde::de::Error,
            {
                value.parse().map_err(serde::de::Error::custom)
            }

            fn visit_string<E>(self, value: String) -> Result<Force, E>
            where
                E: serde::de::Error,
            {
                value.parse().map_err(serde::de::Error::custom)
            }
        }

        deserializer.deserialize_any(ForceVisitor)
    }
}

/// On modified rules
#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnModified<'a> {
    SetOther(OnModifiedSet<'a>),
    SetOtherValueDelta(OnModifiedValueDelta<'a>),
}

/// On modified rules (owned)
#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnModifiedOwned {
    SetOther(OnModifiedSetOwned),
    SetOtherValueDelta(OnModifiedValueDeltaOwned),
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum OnModifiedError {
    /// Skip the operation
    Skip,
    /// Reset item to status = 1, value = 0
    Reset,
    /// Process the operation
    #[default]
    Process,
}

#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OnModifiedSet<'a> {
    /// For the selected OID mask list
    pub oid: &'a OIDMaskList,
    /// The new status
    pub status: ItemStatus,
    /// The new value (optional)
    #[serde(default, skip_serializing_if = "ValueOption::is_none")]
    pub value: ValueOption<'a>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnNegativeDelta {
    /// Skip the operation
    Skip,
    /// Reset item to value = 0
    Reset,
    /// Process the operation
    #[default]
    Process,
    /// Respect the overflow
    Overflow { floor: f64, ceil: f64 },
}

impl Eq for OnNegativeDelta {}

#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OnModifiedValueDelta<'a> {
    /// For the selected OID mask list
    pub oid: &'a OID,
    #[serde(default)]
    /// On item status error
    pub on_error: OnModifiedError,
    /// On negative delta
    #[serde(default)]
    pub on_negative: OnNegativeDelta,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OnModifiedValueDeltaOwned {
    /// For the selected OID
    pub oid: OID,
    /// Calculate delta per given period in seconds (e.g. 1.0 for delta per second, 3600.0 for
    /// delta per hour etc.)
    pub period: Option<f64>,
    #[serde(default)]
    pub on_error: OnModifiedError,
    #[serde(default)]
    pub on_negative: OnNegativeDelta,
}

impl Eq for OnModifiedValueDeltaOwned {}

#[derive(Debug, Clone, Serialize, Eq, PartialEq, Deserialize)]
pub struct OnModifiedSetOwned {
    /// For the selected OID mask list
    pub oid: OIDMaskList,
    /// The new status
    pub status: ItemStatus,
    /// The new value (optional)
    #[serde(default, skip_serializing_if = "ValueOptionOwned::is_none")]
    pub value: ValueOptionOwned,
}

/// Submitted by services via the bus for local items
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct RawStateEvent<'a> {
    pub status: ItemStatus,
    #[serde(default, skip_serializing_if = "ValueOption::is_none")]
    pub value: ValueOption<'a>,
    #[serde(default, skip_serializing_if = "Force::is_none")]
    pub force: Force,
    /// Override the time of the event
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<f64>,
    /// Compare the status with the current status (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_compare: Option<ItemStatus>,
    /// Compare the value with the current value (optional)
    #[serde(default, skip_serializing_if = "ValueOption::is_none")]
    pub value_compare: ValueOption<'a>,
    /// if comparison is used and unequal, set item status. In case if status is not specified,
    /// `crate::ITEM_STATUS_ERROR` is used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_else: Option<ItemStatus>,
    /// if comparison is used and unequal, set item value (optional)
    #[serde(default, skip_serializing_if = "ValueOption::is_none")]
    pub value_else: ValueOption<'a>,
    /// If the item is modified, OnModified rules are applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_modified: Option<OnModified<'a>>,
}

impl Eq for RawStateEvent<'_> {}

impl<'a> RawStateEvent<'a> {
    #[inline]
    pub fn new(status: ItemStatus, value: &'a Value) -> Self {
        Self {
            status,
            value: ValueOption::Value(value),
            force: Force::None,
            t: None,
            on_modified: None,
            status_compare: None,
            value_compare: ValueOption::No,
            status_else: None,
            value_else: ValueOption::No,
        }
    }
    #[inline]
    pub fn new0(status: ItemStatus) -> Self {
        Self {
            status,
            value: ValueOption::No,
            force: Force::None,
            t: None,
            on_modified: None,
            status_compare: None,
            value_compare: ValueOption::No,
            status_else: None,
            value_else: ValueOption::No,
        }
    }
    pub fn force(mut self) -> Self {
        self.force = Force::Full;
        self
    }
    pub fn force_update(mut self) -> Self {
        self.force = Force::Update;
        self
    }
    pub fn at(mut self, t: f64) -> Self {
        self.t = Some(t);
        self
    }
}

/// Submitted by services via the bus for local items
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct RawStateEventOwned {
    pub status: ItemStatus,
    #[serde(default, skip_serializing_if = "ValueOptionOwned::is_none")]
    pub value: ValueOptionOwned,
    #[serde(default, skip_serializing_if = "Force::is_none")]
    pub force: Force,
    /// Override the time of the event
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t: Option<f64>,
    /// Compare the status with the current status (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_compare: Option<ItemStatus>,
    /// Compare the value with the current value (optional)
    #[serde(default, skip_serializing_if = "ValueOptionOwned::is_none")]
    pub value_compare: ValueOptionOwned,
    /// if comparison is used and unequal, set item status. In case if status is not specified,
    /// `crate::ITEM_STATUS_ERROR` is used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_else: Option<ItemStatus>,
    /// if comparison is used and unequal, set item value (optional)
    #[serde(default, skip_serializing_if = "ValueOptionOwned::is_none")]
    pub value_else: ValueOptionOwned,
    /// If the item is modified, OnModified rules are applied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_modified: Option<OnModifiedOwned>,
}

impl Eq for RawStateEventOwned {}

impl RawStateEventOwned {
    #[inline]
    pub fn new(status: ItemStatus, value: Value) -> Self {
        Self {
            status,
            value: ValueOptionOwned::Value(value),
            force: Force::None,
            t: None,
            status_compare: None,
            value_compare: ValueOptionOwned::No,
            status_else: None,
            value_else: ValueOptionOwned::No,
            on_modified: None,
        }
    }
    #[inline]
    pub fn new0(status: ItemStatus) -> Self {
        Self {
            status,
            value: ValueOptionOwned::No,
            force: Force::None,
            t: None,
            status_compare: None,
            value_compare: ValueOptionOwned::No,
            status_else: None,
            value_else: ValueOptionOwned::No,
            on_modified: None,
        }
    }
    pub fn force(mut self) -> Self {
        self.force = Force::Full;
        self
    }
    pub fn force_update(mut self) -> Self {
        self.force = Force::Update;
        self
    }
    pub fn at(mut self, t: f64) -> Self {
        self.t = Some(t);
        self
    }
}

#[derive(Serialize)]
pub struct RawStateBulkEvent<'a> {
    #[serde(alias = "i")]
    pub oid: &'a OID,
    #[serde(flatten)]
    pub raw: RawStateEvent<'a>,
}

impl<'a> RawStateBulkEvent<'a> {
    #[inline]
    pub fn new(oid: &'a OID, rse: RawStateEvent<'a>) -> Self {
        Self { oid, raw: rse }
    }
    #[inline]
    pub fn split_into_oid_and_rse(self) -> (&'a OID, RawStateEvent<'a>) {
        (self.oid, self.raw)
    }
}

impl<'a> From<RawStateBulkEvent<'a>> for RawStateEvent<'a> {
    #[inline]
    fn from(r: RawStateBulkEvent<'a>) -> Self {
        r.raw
    }
}

#[derive(Serialize, Deserialize)]
pub struct RawStateBulkEventOwned {
    #[serde(alias = "i")]
    pub oid: OID,
    #[serde(flatten)]
    pub raw: RawStateEventOwned,
}

impl RawStateBulkEventOwned {
    #[inline]
    pub fn new(oid: OID, rseo: RawStateEventOwned) -> Self {
        Self { oid, raw: rseo }
    }
    #[inline]
    pub fn split_into_oid_and_rseo(self) -> (OID, RawStateEventOwned) {
        (self.oid, self.raw)
    }
}

impl From<RawStateBulkEventOwned> for RawStateEventOwned {
    #[inline]
    fn from(r: RawStateBulkEventOwned) -> Self {
        r.raw
    }
}

/// Submitted by the core via the bus for procesed local events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalStateEvent {
    pub status: ItemStatus,
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<usize>,
    pub ieid: IEID,
    pub t: f64,
}

/// Submitted by the core via the bus for processed remote events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteStateEvent {
    pub status: ItemStatus,
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<usize>,
    pub ieid: IEID,
    pub t: f64,
    pub node: String,
    pub connected: bool,
}

impl RemoteStateEvent {
    pub fn from_local_state_event(
        event: LocalStateEvent,
        system_name: &str,
        connected: bool,
    ) -> Self {
        Self {
            status: event.status,
            value: event.value,
            act: event.act,
            ieid: event.ieid,
            t: event.t,
            node: system_name.to_owned(),
            connected,
        }
    }
}

/// Stored by the core
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DbState {
    pub status: ItemStatus,
    pub value: Value,
    pub ieid: IEID,
    pub t: f64,
}

/// Processed by the core and some additional services
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplicationState {
    pub status: ItemStatus,
    pub value: Value,
    pub act: Option<usize>,
    pub ieid: IEID,
    pub t: f64,
}

/// Submitted by replication services for remote items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStateEvent {
    pub status: ItemStatus,
    pub value: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act: Option<usize>,
    pub ieid: IEID,
    pub t: f64,
    pub node: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub force_accept: bool,
}

impl From<ReplicationStateEvent> for ReplicationState {
    fn from(d: ReplicationStateEvent) -> Self {
        Self {
            status: d.status,
            value: d.value,
            act: d.act,
            ieid: d.ieid,
            t: d.t,
        }
    }
}

impl TryFrom<ReplicationInventoryItem> for ReplicationState {
    type Error = Error;
    fn try_from(item: ReplicationInventoryItem) -> Result<Self, Self::Error> {
        let v: Option<Value> = item.value.into();
        Ok(Self {
            status: item.status.unwrap_or_default(),
            value: v.unwrap_or_default(),
            act: item.act,
            ieid: item
                .ieid
                .ok_or_else(|| Error::invalid_data(format!("IEID missing ({})", item.oid)))?,
            t: item
                .t
                .ok_or_else(|| Error::invalid_data(format!("Set time missing ({})", item.oid)))?,
        })
    }
}

#[allow(clippy::similar_names)]
impl ReplicationStateEvent {
    #[inline]
    pub fn new(
        status: ItemStatus,
        value: Value,
        act: Option<usize>,
        ieid: IEID,
        t: f64,
        node: &str,
    ) -> Self {
        Self {
            status,
            value,
            act,
            ieid,
            t,
            node: node.to_owned(),
            force_accept: false,
        }
    }
}

impl From<ReplicationStateEvent> for RemoteStateEvent {
    fn from(d: ReplicationStateEvent) -> Self {
        Self {
            status: d.status,
            value: d.value,
            act: d.act,
            ieid: d.ieid,
            t: d.t,
            node: d.node,
            connected: true,
        }
    }
}

/// Submitted by replication services to RPL/INVENTORY/<name> (as a list of)
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ReplicationInventoryItem {
    pub oid: OID,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ItemStatus>,
    #[serde(default, skip_serializing_if = "ValueOptionOwned::is_none")]
    pub value: ValueOptionOwned,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<usize>,
    pub ieid: Option<IEID>,
    pub t: Option<f64>,
    pub meta: Option<Value>,
    pub enabled: bool,
}

impl Hash for ReplicationInventoryItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.oid.hash(state);
    }
}

impl Eq for ReplicationInventoryItem {}

impl PartialEq for ReplicationInventoryItem {
    fn eq(&self, other: &Self) -> bool {
        self.oid == other.oid
    }
}

/// full state with info, returned by item.state RPC functions, used in HMI and other apps
#[derive(Debug, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct FullItemStateAndInfo<'a> {
    #[serde(flatten)]
    pub si: ItemStateAndInfo<'a>,
    // full
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<&'a Value>,
    pub enabled: bool,
}

/// short state with info, returned by item.state RPC functions, used in HMI and other apps
#[derive(Debug, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ItemStateAndInfo<'a> {
    pub oid: &'a OID,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ItemStatus>,
    // the value is always owned as states are usually hold under mutexes
    #[serde(default, skip_serializing_if = "ValueOptionOwned::is_none")]
    pub value: ValueOptionOwned,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<usize>,
    pub ieid: Option<IEID>,
    pub t: Option<f64>,
    pub node: &'a str,
    pub connected: bool,
}

/// full state with info, returned by item.state RPC functions, used in HMI and other apps
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct FullItemStateAndInfoOwned {
    #[serde(flatten)]
    pub si: ItemStateAndInfoOwned,
    // full
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<Value>,
    pub enabled: bool,
}

/// short state with info, returned by item.state RPC functions, used in HMI and other apps
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ItemStateAndInfoOwned {
    pub oid: OID,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ItemStatus>,
    #[serde(default, skip_serializing_if = "ValueOptionOwned::is_none")]
    pub value: ValueOptionOwned,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act: Option<usize>,
    pub ieid: Option<IEID>,
    pub t: Option<f64>,
    pub node: String,
    pub connected: bool,
}

impl From<FullItemStateAndInfoOwned> for ReplicationInventoryItem {
    fn from(s: FullItemStateAndInfoOwned) -> ReplicationInventoryItem {
        ReplicationInventoryItem {
            oid: s.si.oid,
            status: s.si.status,
            value: s.si.value,
            act: s.si.act,
            ieid: s.si.ieid,
            t: s.si.t,
            meta: s.meta,
            enabled: s.enabled,
        }
    }
}

pub struct EventBuffer<T> {
    data: parking_lot::Mutex<Vec<T>>,
    size: usize,
}

#[allow(dead_code)]
impl<T> EventBuffer<T> {
    #[inline]
    pub fn bounded(size: usize) -> Self {
        Self {
            data: <_>::default(),
            size,
        }
    }
    #[inline]
    pub fn unbounded() -> Self {
        Self {
            data: <_>::default(),
            size: 0,
        }
    }
    pub fn push(&self, value: T) -> EResult<()> {
        let mut buf = self.data.lock();
        if self.size > 0 && buf.len() >= self.size {
            return Err(Error::failed("buffer overflow, event dropped"));
        }
        buf.push(value);
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.data.lock().len()
    }
    pub fn is_empty(&self) -> bool {
        self.data.lock().is_empty()
    }
    pub fn take(&self) -> Vec<T> {
        std::mem::take(&mut *self.data.lock())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ReplicationStateEventExtended {
    Inventory(ReplicationNodeInventoryItem),
    Basic(ReplicationStateEvent),
}

impl ReplicationStateEventExtended {
    pub fn node(&self) -> &str {
        match self {
            ReplicationStateEventExtended::Basic(v) => &v.node,
            ReplicationStateEventExtended::Inventory(v) => &v.node,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ReplicationNodeInventoryItem {
    pub node: String,
    #[serde(flatten)]
    pub item: ReplicationInventoryItem,
}
