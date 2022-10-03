use crate::EResult;
use serde::{Deserialize, Serialize};

#[inline]
pub fn pack<T>(val: &T) -> EResult<Vec<u8>>
where
    T: Serialize + ?Sized,
{
    rmp_serde::to_vec_named(val).map_err(Into::into)
}

#[inline]
pub fn unpack<'a, T>(input: &'a [u8]) -> EResult<T>
where
    T: Deserialize<'a>,
{
    rmp_serde::from_slice(input).map_err(Into::into)
}
