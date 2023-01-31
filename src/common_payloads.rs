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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum IdOrList<'a> {
    Single(&'a str),
    Multi(Vec<&'a str>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum IdOrListOwned {
    Single(String),
    Multi(Vec<String>),
}

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

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ValueOrList<T> {
    Single(T),
    Multiple(Vec<T>),
}

impl<T> Default for ValueOrList<T> {
    #[inline]
    fn default() -> Self {
        ValueOrList::Multiple(Vec::new())
    }
}

impl<T> ValueOrList<T> {
    pub fn is_empty(&self) -> bool {
        match self {
            ValueOrList::Single(_) => false,
            ValueOrList::Multiple(v) => v.is_empty(),
        }
    }
    pub fn iter(&self) -> ValueOrListIter<T> {
        ValueOrListIter {
            path: self,
            curr: 0,
        }
    }
    pub fn shuffle(&mut self) {
        if let ValueOrList::Multiple(ref mut v) = self {
            v.shuffle(&mut thread_rng());
        }
    }
}

pub struct ValueOrListIter<'a, T> {
    path: &'a ValueOrList<T>,
    curr: usize,
}

impl<'a, T> Iterator for ValueOrListIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        let res = match self.path {
            ValueOrList::Single(v) => {
                if self.curr == 0 {
                    Some(v)
                } else {
                    None
                }
            }
            ValueOrList::Multiple(v) => v.get(self.curr),
        };
        self.curr += 1;
        res
    }
}
