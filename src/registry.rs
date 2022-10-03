use crate::err_logger;
use crate::payload::{pack, unpack};
use crate::prelude::*;
use busrt::rpc::{Rpc, RpcClient};
use busrt::QoS;
use serde::{Deserialize, Serialize};

err_logger!();

pub const GLOBAL_KEY_PREFIX: &str = "eva";
pub const SERVICE_NAME: &str = "eva.registry";

pub const R_INVENTORY: &str = "inventory";
pub const R_STATE: &str = "state";
pub const R_SERVICE: &str = "svc";
pub const R_SERVICE_DATA: &str = "svc_data";
pub const R_USER_DATA: &str = "user_data";
pub const R_CONFIG: &str = "config";
pub const R_DATA: &str = "data";
pub const R_CACHE: &str = "cache";

// the below methods are pub as the core access the registry directly as db during startup
#[inline]
pub fn format_top_key(key: &str) -> String {
    format!("{}/{}", GLOBAL_KEY_PREFIX, key)
}

#[inline]
pub fn format_key(prefix: &str, key: &str) -> String {
    format!("{}/{}/{}", GLOBAL_KEY_PREFIX, prefix, key)
}

#[inline]
pub fn format_config_key(key: &str) -> String {
    format!("{}/{}/{}", GLOBAL_KEY_PREFIX, R_CONFIG, key)
}

#[inline]
pub fn format_data_key(key: &str) -> String {
    format!("{}/{}/{}", GLOBAL_KEY_PREFIX, R_DATA, key)
}

#[inline]
pub fn format_svc_data_key(key: &str) -> String {
    format!("{}/{}/{}", GLOBAL_KEY_PREFIX, R_SERVICE_DATA, key)
}

#[inline]
pub fn format_svc_data_subkey(key: &str) -> String {
    format!("{}/{}", R_SERVICE_DATA, key)
}

#[inline]
async fn call<P>(method: &str, payload: P, rpc: &RpcClient) -> EResult<Value>
where
    P: Serialize,
{
    let result = rpc
        .call(
            SERVICE_NAME,
            method,
            pack(&payload).log_err()?.into(),
            QoS::Processed,
        )
        .await
        .map_err(|e| {
            Error::registry(std::str::from_utf8(e.data().unwrap_or(&[])).unwrap_or_default())
        })?;
    unpack(result.payload())
}

#[derive(Serialize)]
struct PayloadKeySet {
    key: String,
    value: Value,
}

#[derive(Serialize)]
struct PayloadKey {
    key: String,
}

#[inline]
pub async fn key_set<V>(prefix: &str, key: &str, value: V, rpc: &RpcClient) -> EResult<Value>
where
    V: Serialize,
{
    let payload = PayloadKeySet {
        key: format_key(prefix, key),
        value: to_value(value)?,
    };
    call("key_set", payload, rpc).await
}

#[inline]
pub async fn key_get(prefix: &str, key: &str, rpc: &RpcClient) -> EResult<Value> {
    let payload = PayloadKey {
        key: format_key(prefix, key),
    };
    call("key_get", payload, rpc).await
}

#[inline]
pub async fn key_increment(prefix: &str, key: &str, rpc: &RpcClient) -> EResult<i64> {
    let payload = PayloadKey {
        key: format_key(prefix, key),
    };
    TryInto::<i64>::try_into(call("key_increment", payload, rpc).await?).map_err(Into::into)
}

#[inline]
pub async fn key_decrement(prefix: &str, key: &str, rpc: &RpcClient) -> EResult<i64> {
    let payload = PayloadKey {
        key: format_key(prefix, key),
    };
    TryInto::<i64>::try_into(call("key_decrement", payload, rpc).await?).map_err(Into::into)
}

#[inline]
pub async fn key_get_recursive(
    prefix: &str,
    key: &str,
    rpc: &RpcClient,
) -> EResult<Vec<(String, Value)>> {
    let payload = PayloadKey {
        key: format_key(prefix, key),
    };
    let key_len = payload.key.len() + 1;
    let val = call("key_get_recursive", payload, rpc).await?;
    let res: Vec<(String, Value)> = Vec::deserialize(val)?;
    let mut result: Vec<(String, Value)> = Vec::new();
    for (k, v) in res {
        if k.len() < key_len {
            return Err(Error::invalid_data(format!(
                "invalid key name returned by the registry: {}",
                k
            )));
        }
        result.push((k[key_len..].to_string(), v));
    }
    Ok(result)
}

#[inline]
pub async fn key_delete(prefix: &str, key: &str, rpc: &RpcClient) -> EResult<Value> {
    let payload = PayloadKey {
        key: format_key(prefix, key),
    };
    call("key_delete", payload, rpc).await
}

#[inline]
pub async fn key_delete_recursive(prefix: &str, key: &str, rpc: &RpcClient) -> EResult<Value> {
    let payload = PayloadKey {
        key: format_key(prefix, key),
    };
    call("key_delete_recursive", payload, rpc).await
}
