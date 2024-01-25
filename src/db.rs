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
use crate::{value::Value, EResult, Error, OID};
use once_cell::sync::OnceCell;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{self, PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::sqlite::{self, SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::{database, ConnectOptions, Database, Decode, Encode};
use sqlx::{Postgres, Sqlite, Type};
use std::borrow::Cow;
use std::str::FromStr;
use std::time::Duration;

pub mod prelude {
    pub use super::{db_init, db_pool, DbKind, DbPool, Transaction};
}

static DB_POOL: OnceCell<DbPool> = OnceCell::new();

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

impl<'r, DB: Database> Decode<'r, DB> for OID
where
    &'r str: Decode<'r, DB>,
{
    fn decode(value: <DB as database::HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
        let value = <&str as Decode<DB>>::decode(value)?;
        value.parse().map_err(Into::into)
    }
}

impl<'q> Encode<'q, Sqlite> for OID {
    fn encode(self, args: &mut Vec<sqlite::SqliteArgumentValue<'q>>) -> IsNull {
        args.push(sqlite::SqliteArgumentValue::Text(Cow::Owned(
            self.to_string(),
        )));

        IsNull::No
    }
    fn encode_by_ref(&self, args: &mut Vec<sqlite::SqliteArgumentValue<'q>>) -> IsNull {
        args.push(sqlite::SqliteArgumentValue::Text(Cow::Owned(
            self.to_string(),
        )));
        IsNull::No
    }

    fn size_hint(&self) -> usize {
        self.as_str().len()
    }
}

impl Encode<'_, Postgres> for OID {
    fn encode_by_ref(&self, buf: &mut postgres::PgArgumentBuffer) -> IsNull {
        <&str as Encode<Postgres>>::encode(self.as_str(), buf)
    }
    fn size_hint(&self) -> usize {
        self.as_str().len()
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
    fn encode_by_ref(&self, buf: &mut Vec<sqlite::SqliteArgumentValue<'_>>) -> IsNull {
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

impl<'q> Encode<'q, Postgres> for Value {
    fn encode_by_ref(&self, buf: &mut postgres::PgArgumentBuffer) -> IsNull {
        buf.push(1);
        serde_json::to_writer(&mut **buf, &self)
            .expect("failed to serialize to JSON for encoding on transmission to the database");
        IsNull::No
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
    use crate::time::Time;
    use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef};
    use sqlx::sqlite::{SqliteArgumentValue, SqliteTypeInfo, SqliteValueRef};
    use sqlx::{encode::IsNull, error::BoxDynError, Decode, Encode, Postgres, Sqlite, Type};

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
        fn encode_by_ref(&self, args: &mut Vec<SqliteArgumentValue<'q>>) -> IsNull {
            args.push(SqliteArgumentValue::Int64(
                i64::try_from(self.timestamp_ns()).expect("timestamp too large"),
            ));

            IsNull::No
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
        fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> IsNull {
            let us =
                i64::try_from(self.timestamp_us()).expect("timestamp too large") - J2000_EPOCH_US;
            Encode::<Postgres>::encode(&us, buf)
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

/// # Panics
///
/// Will panic if not initialized
#[allow(clippy::module_name_repetitions)]
#[inline]
pub fn db_pool() -> &'static DbPool {
    DB_POOL.get().unwrap()
}

#[allow(clippy::module_name_repetitions)]
pub enum DbPool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum DbKind {
    Sqlite,
    Postgres,
}

impl DbPool {
    pub async fn begin(&self) -> Result<Transaction<'_>, sqlx::Error> {
        match self {
            DbPool::Sqlite(p) => Ok(Transaction::Sqlite(p.begin().await?)),
            DbPool::Postgres(p) => Ok(Transaction::Postgres(p.begin().await?)),
        }
    }
    pub fn kind(&self) -> DbKind {
        match self {
            DbPool::Sqlite(_) => DbKind::Sqlite,
            DbPool::Postgres(_) => DbKind::Postgres,
        }
    }
    pub async fn execute(&self, q: &str) -> EResult<()> {
        match self {
            DbPool::Sqlite(ref p) => {
                sqlx::query(q).execute(p).await?;
            }
            DbPool::Postgres(ref p) => {
                sqlx::query(q).execute(p).await?;
            }
        }
        Ok(())
    }
}

pub enum Transaction<'c> {
    Sqlite(sqlx::Transaction<'c, sqlx::sqlite::Sqlite>),
    Postgres(sqlx::Transaction<'c, sqlx::postgres::Postgres>),
}

impl<'c> Transaction<'c> {
    pub async fn commit(self) -> Result<(), sqlx::Error> {
        match self {
            Transaction::Sqlite(tx) => tx.commit().await,
            Transaction::Postgres(tx) => tx.commit().await,
        }
    }
    pub fn kind(&self) -> DbKind {
        match self {
            Transaction::Sqlite(_) => DbKind::Sqlite,
            Transaction::Postgres(_) => DbKind::Postgres,
        }
    }
    pub async fn execute(&mut self, q: &str) -> EResult<()> {
        match self {
            Transaction::Sqlite(ref mut p) => {
                sqlx::query(q).execute(p).await?;
            }
            Transaction::Postgres(ref mut p) => {
                sqlx::query(q).execute(p).await?;
            }
        }
        Ok(())
    }
}

/// Initialize database, must be called first and only once,
/// enables module-wide pool
#[allow(clippy::module_name_repetitions)]
pub async fn db_init(conn: &str, pool_size: u32, timeout: Duration) -> EResult<()> {
    DB_POOL
        .set(create_pool(conn, pool_size, timeout).await?)
        .map_err(|_| Error::core("unable to set DB_POOL"))?;
    Ok(())
}

/// Creates a pool to use it without the module
pub async fn create_pool(conn: &str, pool_size: u32, timeout: Duration) -> EResult<DbPool> {
    if conn.starts_with("sqlite://") {
        let mut opts = SqliteConnectOptions::from_str(conn)?
            .create_if_missing(true)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Extra)
            .busy_timeout(timeout);
        opts.log_statements(log::LevelFilter::Trace)
            .log_slow_statements(log::LevelFilter::Warn, timeout);
        Ok(DbPool::Sqlite(
            SqlitePoolOptions::new()
                .max_connections(pool_size)
                .acquire_timeout(timeout)
                .connect_with(opts)
                .await?,
        ))
    } else if conn.starts_with("postgres://") {
        let mut opts = PgConnectOptions::from_str(conn)?;
        opts.log_statements(log::LevelFilter::Trace)
            .log_slow_statements(log::LevelFilter::Warn, timeout);
        Ok(DbPool::Postgres(
            PgPoolOptions::new()
                .max_connections(pool_size)
                .acquire_timeout(timeout)
                .connect_with(opts)
                .await?,
        ))
    } else {
        Err(Error::unsupported("Unsupported database kind"))
    }
}
