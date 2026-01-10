/// Database functions (sqlx-based) and dynamic wrapper
/// Currently supported sqlx 0.6 only
///
/// Supported databases: Sqlite, PostgresSQL
///
/// For Value type use JSONB only
/// For OID use VARCHAR(1024)
///
/// For Time (feature "time" enabled) type use INTEGER for Sqlite and TIMESTAMP/TIMESTAMPTZ for
/// Postgres
#[cfg(feature = "acl")]
use crate::acl::OIDMask;
use crate::{OID, value::Value};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres;
use sqlx::postgres::PgValueRef;
use sqlx::sqlite;
use sqlx::sqlite::SqliteValueRef;
use sqlx::{Decode, Encode};
use sqlx::{Postgres, Sqlite, Type};
use std::borrow::Cow;

type ResultIsNull = Result<IsNull, Box<dyn std::error::Error + Send + Sync + 'static>>;

impl Type<Sqlite> for OID {
    fn type_info() -> sqlite::SqliteTypeInfo {
        <str as Type<Sqlite>>::type_info()
    }
}

impl Type<Postgres> for OID {
    fn type_info() -> postgres::PgTypeInfo {
        <str as Type<Postgres>>::type_info()
    }
    fn compatible(ty: &postgres::PgTypeInfo) -> bool {
        *ty == postgres::PgTypeInfo::with_name("VARCHAR")
            || *ty == postgres::PgTypeInfo::with_name("TEXT")
    }
}

impl postgres::PgHasArrayType for OID {
    fn array_type_info() -> postgres::PgTypeInfo {
        postgres::PgTypeInfo::with_name("_TEXT")
    }

    fn array_compatible(ty: &postgres::PgTypeInfo) -> bool {
        *ty == postgres::PgTypeInfo::with_name("_TEXT")
            || *ty == postgres::PgTypeInfo::with_name("_VARCHAR")
    }
}

impl<'r> Decode<'r, Sqlite> for OID {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <&str as Decode<Sqlite>>::decode(value)?;
        value.parse().map_err(Into::into)
    }
}

impl<'r> Decode<'r, Postgres> for OID {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <&str as Decode<Postgres>>::decode(value)?;
        value.parse().map_err(Into::into)
    }
}

impl<'q> Encode<'q, Sqlite> for OID {
    fn encode(self, args: &mut Vec<sqlite::SqliteArgumentValue<'q>>) -> ResultIsNull {
        args.push(sqlite::SqliteArgumentValue::Text(Cow::Owned(
            self.to_string(),
        )));

        Ok(IsNull::No)
    }
    fn encode_by_ref(&self, args: &mut Vec<sqlite::SqliteArgumentValue<'q>>) -> ResultIsNull {
        args.push(sqlite::SqliteArgumentValue::Text(Cow::Owned(
            self.to_string(),
        )));
        Ok(IsNull::No)
    }

    fn size_hint(&self) -> usize {
        self.as_str().len()
    }
}

impl Encode<'_, Postgres> for OID {
    fn encode_by_ref(&self, buf: &mut postgres::PgArgumentBuffer) -> ResultIsNull {
        <&str as Encode<Postgres>>::encode(self.as_str(), buf)
    }
    fn size_hint(&self) -> usize {
        self.as_str().len()
    }
}

#[cfg(feature = "acl")]
impl Type<Sqlite> for OIDMask {
    fn type_info() -> sqlite::SqliteTypeInfo {
        <str as Type<Sqlite>>::type_info()
    }
}

#[cfg(feature = "acl")]
impl Type<Postgres> for OIDMask {
    fn type_info() -> postgres::PgTypeInfo {
        <str as Type<Postgres>>::type_info()
    }
    fn compatible(ty: &postgres::PgTypeInfo) -> bool {
        *ty == postgres::PgTypeInfo::with_name("VARCHAR")
            || *ty == postgres::PgTypeInfo::with_name("TEXT")
    }
}

#[cfg(feature = "acl")]
impl postgres::PgHasArrayType for OIDMask {
    fn array_type_info() -> postgres::PgTypeInfo {
        postgres::PgTypeInfo::with_name("_TEXT")
    }

    fn array_compatible(ty: &postgres::PgTypeInfo) -> bool {
        *ty == postgres::PgTypeInfo::with_name("_TEXT")
            || *ty == postgres::PgTypeInfo::with_name("_VARCHAR")
    }
}

#[cfg(feature = "acl")]
impl<'r> Decode<'r, Sqlite> for OIDMask {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <&str as Decode<Sqlite>>::decode(value)?;
        value.parse().map_err(Into::into)
    }
}

#[cfg(feature = "acl")]
impl<'r> Decode<'r, Postgres> for OIDMask {
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let value = <&str as Decode<Postgres>>::decode(value)?;
        value.parse().map_err(Into::into)
    }
}

#[cfg(feature = "acl")]
impl<'q> Encode<'q, Sqlite> for OIDMask {
    fn encode(self, args: &mut Vec<sqlite::SqliteArgumentValue<'q>>) -> ResultIsNull {
        args.push(sqlite::SqliteArgumentValue::Text(Cow::Owned(
            self.to_string(),
        )));

        Ok(IsNull::No)
    }
    fn encode_by_ref(&self, args: &mut Vec<sqlite::SqliteArgumentValue<'q>>) -> ResultIsNull {
        args.push(sqlite::SqliteArgumentValue::Text(Cow::Owned(
            self.to_string(),
        )));
        Ok(IsNull::No)
    }
}

#[cfg(feature = "acl")]
impl Encode<'_, Postgres> for OIDMask {
    #[allow(clippy::needless_borrows_for_generic_args)]
    fn encode_by_ref(&self, buf: &mut postgres::PgArgumentBuffer) -> ResultIsNull {
        <&str as Encode<Postgres>>::encode(&self.to_string(), buf)
    }
}

impl Type<Sqlite> for Value {
    fn type_info() -> sqlite::SqliteTypeInfo {
        <str as Type<Sqlite>>::type_info()
    }

    fn compatible(ty: &sqlite::SqliteTypeInfo) -> bool {
        <&str as Type<Sqlite>>::compatible(ty)
    }
}

impl Type<Postgres> for Value {
    fn type_info() -> postgres::PgTypeInfo {
        postgres::PgTypeInfo::with_name("JSONB")
    }
}

impl Encode<'_, Sqlite> for Value {
    fn encode_by_ref(&self, buf: &mut Vec<sqlite::SqliteArgumentValue<'_>>) -> ResultIsNull {
        let json_string_value =
            serde_json::to_string(self).expect("serde_json failed to convert to string");
        Encode::<Sqlite>::encode(json_string_value, buf)
    }
}

impl<'r> Decode<'r, Sqlite> for Value {
    fn decode(value: sqlite::SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let string_value = <&str as Decode<Sqlite>>::decode(value)?;

        serde_json::from_str(string_value).map_err(Into::into)
    }
}

impl Encode<'_, Postgres> for Value {
    fn encode_by_ref(&self, buf: &mut postgres::PgArgumentBuffer) -> ResultIsNull {
        buf.push(1);
        serde_json::to_writer(&mut **buf, &self)
            .expect("failed to serialize to JSON for encoding on transmission to the database");
        Ok(IsNull::No)
    }
}

impl<'r> Decode<'r, Postgres> for Value {
    fn decode(value: postgres::PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let buf = value.as_bytes()?;
        assert_eq!(buf[0], 1, "unsupported JSONB format version {}", buf[0]);
        serde_json::from_slice(&buf[1..]).map_err(Into::into)
    }
}

#[cfg(feature = "time")]
mod time_impl {
    use super::ResultIsNull;
    use crate::time::Time;
    use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
    use sqlx::sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};
    use sqlx::{Decode, Encode, Postgres, Sqlite, Type, encode::IsNull, error::BoxDynError};

    const J2000_EPOCH_US: i64 = 946_684_800_000_000;

    impl Type<Sqlite> for Time {
        fn type_info() -> SqliteTypeInfo {
            <i64 as Type<Sqlite>>::type_info()
        }

        fn compatible(ty: &SqliteTypeInfo) -> bool {
            *ty == <i64 as Type<Sqlite>>::type_info()
                || *ty == <i32 as Type<Sqlite>>::type_info()
                || *ty == <i16 as Type<Sqlite>>::type_info()
                || *ty == <i8 as Type<Sqlite>>::type_info()
        }
    }

    impl<'q> Encode<'q, Sqlite> for Time {
        fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> ResultIsNull {
            args.push(SqliteArgumentValue::Int64(
                i64::try_from(self.timestamp_ns()).expect("timestamp too large"),
            ));

            Ok(IsNull::No)
        }
    }

    impl<'r> Decode<'r, Sqlite> for Time {
        fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
            let value = <i64 as Decode<Sqlite>>::decode(value)?;
            Ok(Time::from_timestamp_ns(
                value.try_into().unwrap_or_default(),
            ))
        }
    }

    impl Type<Postgres> for Time {
        fn type_info() -> PgTypeInfo {
            PgTypeInfo::with_name("TIMESTAMPTZ")
        }
        fn compatible(ty: &PgTypeInfo) -> bool {
            *ty == PgTypeInfo::with_name("TIMESTAMPTZ") || *ty == PgTypeInfo::with_name("TIMESTAMP")
        }
    }

    impl Encode<'_, Postgres> for Time {
        fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> ResultIsNull {
            let us =
                i64::try_from(self.timestamp_us()).expect("timestamp too large") - J2000_EPOCH_US;
            Encode::<Postgres>::encode(us, buf)
        }

        fn size_hint(&self) -> usize {
            std::mem::size_of::<i64>()
        }
    }

    impl<'r> Decode<'r, Postgres> for Time {
        fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
            let us: i64 = Decode::<Postgres>::decode(value)?;
            Ok(Time::from_timestamp_us(
                (us + J2000_EPOCH_US).try_into().unwrap_or_default(),
            ))
        }
    }
}
