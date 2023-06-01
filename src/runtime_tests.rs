#[allow(unused_imports)]
use crate::{EResult, Error};

#[cfg(not(feature = "skip_self_test_serde"))]
#[allow(clippy::unreadable_literal)]
fn test_serde() -> EResult<()> {
    for json_val in [
        serde_json::json!({"int":1234567890}),
        serde_json::json!({"float":1234567890.123}),
    ] {
        let val: crate::value::Value = crate::value::to_value(json_val).unwrap();
        if let crate::value::Value::Map(_) = val {
            return Err(Error::core("serde_json arbitrary_precision MUST be off"));
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn failed(test: &str, e: Error) {
    panic!(
        "eva_common::self.test::{} failed: {}",
        test,
        e.message().unwrap_or_default()
    )
}

/// # Panics
///
/// Will panic if any test failed
pub fn self_test() {
    #[cfg(not(feature = "skip_self_test_serde"))]
    test_serde().map_err(|e| failed("serde", e)).unwrap();
}
