/// Contains the action manager
use crate::value::Value;
use crate::{EResult, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub const ACTION_CREATED: u8 = 0b0000_0000; // created by the core
pub const ACTION_ACCEPTED: u8 = 0b0000_0001; // accepted
pub const ACTION_PENDING: u8 = 0b0000_0010; // queued by the controller
pub const ACTION_RUNNING: u8 = 0b0000_1000; // running by the controller
pub const ACTION_COMPLETED: u8 = 0b0000_1111; // completed successfully
pub const ACTION_FAILED: u8 = 0b1000_0000; // failed to be completed
pub const ACTION_CANCELED: u8 = 0b1000_0001; // canceled in queue
pub const ACTION_TERMINATED: u8 = 0b1000_0010; // terminated while running

pub const ACTION_TOPIC: &str = "ACT/";

pub const DEFAULT_ACTION_PRIORITY: u8 = 100;

#[inline]
pub fn default_action_priority() -> u8 {
    DEFAULT_ACTION_PRIORITY
}

/// Action status enum
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Copy, Clone, Hash, PartialOrd)]
#[repr(u8)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Created = ACTION_CREATED,
    Accepted = ACTION_ACCEPTED,
    Pending = ACTION_PENDING,
    Running = ACTION_RUNNING,
    Completed = ACTION_COMPLETED,
    Failed = ACTION_FAILED,
    Canceled = ACTION_CANCELED,
    Terminated = ACTION_TERMINATED,
}

impl TryFrom<u8> for Status {
    type Error = Error;
    fn try_from(code: u8) -> EResult<Self> {
        match code {
            ACTION_CREATED => Ok(Status::Created),
            ACTION_ACCEPTED => Ok(Status::Accepted),
            ACTION_PENDING => Ok(Status::Pending),
            ACTION_RUNNING => Ok(Status::Running),
            ACTION_COMPLETED => Ok(Status::Completed),
            ACTION_FAILED => Ok(Status::Failed),
            ACTION_CANCELED => Ok(Status::Canceled),
            ACTION_TERMINATED => Ok(Status::Terminated),
            _ => Err(Error::invalid_data(format!(
                "invalid action code: {}",
                code
            ))),
        }
    }
}

/// Params for unit actions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UnitParams {
    pub value: Value,
}

/// Params for lmacro actions
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct LmacroParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kwargs: Option<HashMap<String, Value>>,
}

/// Params enum
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Params {
    Unit(UnitParams),
    Lmacro(LmacroParams),
}

/// Params view enum
#[derive(Serialize)]
#[serde(untagged)]
pub enum ParamsView<'a> {
    Unit(&'a UnitParams),
    Lmacro(LmacroParamsView<'a>),
}

#[derive(Serialize)]
pub struct LmacroParamsView<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kwargs: Option<HashMap<&'a str, Value>>,
}

impl Params {
    #[inline]
    pub fn new_unit(value: Value) -> Self {
        Self::Unit(UnitParams { value })
    }
    #[inline]
    pub fn new_lmacro(args: Option<Vec<Value>>, kwargs: Option<HashMap<String, Value>>) -> Self {
        Self::Lmacro(LmacroParams { args, kwargs })
    }
    pub fn as_view(&self) -> ParamsView<'_> {
        match self {
            Params::Unit(p) => ParamsView::Unit(p),
            Params::Lmacro(p) => {
                let args = p
                    .args
                    .as_ref()
                    .map(|args| args.iter().map(|v| v.clone().to_no_bytes()).collect());
                let kwargs = if let Some(ref kwargs) = p.kwargs {
                    let mut m: HashMap<&str, Value> = HashMap::new();
                    for (k, v) in kwargs {
                        m.insert(k, v.clone().to_no_bytes());
                    }
                    Some(m)
                } else {
                    None
                };
                ParamsView::Lmacro(LmacroParamsView { args, kwargs })
            }
        }
    }
}

/// Event payload, announced by services when an action changes its state
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ActionEvent {
    pub uuid: Uuid,
    pub status: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub err: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exitcode: Option<i16>,
}
