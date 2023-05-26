use crate::events::NodeInfo;
use crate::value::Value;
use crate::OID;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::atomic;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

pub fn deserialize_uuid<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
    D: Deserializer<'de>,
{
    let val: Value = Deserialize::deserialize(deserializer)?;
    Uuid::deserialize(val).map_err(serde::de::Error::custom)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeData {
    svc: Option<String>,
    #[serde(
        deserialize_with = "crate::tools::deserialize_arc_atomic_bool",
        serialize_with = "crate::tools::serialize_atomic_bool"
    )]
    online: Arc<atomic::AtomicBool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<NodeInfo>,
    #[serde(
        default,
        serialize_with = "crate::tools::serialize_opt_duration_as_f64",
        deserialize_with = "crate::tools::de_opt_float_as_duration"
    )]
    timeout: Option<Duration>,
}

impl NodeData {
    #[inline]
    pub fn new(
        svc: Option<&str>,
        online: bool,
        info: Option<NodeInfo>,
        timeout: Option<Duration>,
    ) -> Self {
        Self {
            svc: svc.map(ToOwned::to_owned),
            online: Arc::new(atomic::AtomicBool::new(online)),
            info,
            timeout,
        }
    }
    #[inline]
    pub fn svc(&self) -> Option<&str> {
        self.svc.as_deref()
    }
    #[inline]
    pub fn online(&self) -> bool {
        self.online.load(atomic::Ordering::SeqCst)
    }
    #[inline]
    pub fn online_beacon(&self) -> Arc<atomic::AtomicBool> {
        self.online.clone()
    }
    #[inline]
    pub fn info(&self) -> Option<&NodeInfo> {
        self.info.as_ref()
    }
    #[inline]
    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
    #[inline]
    pub fn set_online(&self, online: bool) {
        self.online.store(online, atomic::Ordering::SeqCst);
    }
    #[inline]
    pub fn update_info(&mut self, info: NodeInfo) {
        self.info.replace(info);
    }
    #[inline]
    pub fn update_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParamsId<'a> {
    #[serde(borrow)]
    pub i: &'a str,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParamsIdOwned {
    pub i: String,
}

pub type IdOrList<'a> = ValueOrList<&'a str>;

pub type IdOrListOwned = ValueOrList<String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParamsIdOrList<'a> {
    #[serde(borrow)]
    pub i: IdOrList<'a>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParamsIdOrListOwned {
    pub i: IdOrListOwned,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParamsIdList<'a> {
    #[serde(borrow)]
    pub i: Vec<&'a str>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ParamsIdListOwned {
    pub i: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParamsOID {
    pub i: OID,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParamsUuid {
    pub u: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParamsUuidAny {
    #[serde(deserialize_with = "deserialize_uuid")]
    pub u: Uuid,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ValueOrList<T>
where
    T: Send + Sync,
{
    Single(T),
    Multiple(Vec<T>),
}

impl<T> Default for ValueOrList<T>
where
    T: Send + Sync,
{
    #[inline]
    fn default() -> Self {
        ValueOrList::Multiple(Vec::new())
    }
}

impl<T> ValueOrList<T>
where
    T: Send + Sync,
{
    pub fn is_empty(&self) -> bool {
        match self {
            ValueOrList::Single(_) => false,
            ValueOrList::Multiple(v) => v.is_empty(),
        }
    }
    pub fn len(&self) -> usize {
        match self {
            ValueOrList::Single(_) => 1,
            ValueOrList::Multiple(v) => v.len(),
        }
    }
    pub fn shuffle(&mut self) {
        if let ValueOrList::Multiple(ref mut v) = self {
            v.shuffle(&mut thread_rng());
        }
    }
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
        self.into_iter()
    }
}

impl<T: Send + Sync + 'static> IntoIterator for ValueOrList<T> {
    type Item = T;
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + Send + Sync + 'static>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            ValueOrList::Single(s) => Box::new(SingleIter(Some(s))),
            ValueOrList::Multiple(vals) => Box::new(vals.into_iter()),
        }
    }
}

impl<'a, T: Send + Sync> IntoIterator for &'a ValueOrList<T> {
    type Item = &'a T;
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + Send + Sync + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            ValueOrList::Single(s) => Box::new(SingleIter(Some(s))),
            ValueOrList::Multiple(vals) => Box::new(vals.iter()),
        }
    }
}

struct SingleIter<T>(Option<T>);

impl<T> Iterator for SingleIter<T> {
    type Item = T;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.take()
    }
}
