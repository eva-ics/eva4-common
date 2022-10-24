use crate::payload::{pack, unpack};
use crate::{EResult, Error};
use log::{error, trace};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteSynchronous},
    ConnectOptions, Pool, Sqlite,
};
use std::str::FromStr;
use std::time::Duration;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio::task::JoinHandle;

#[inline]
fn now() -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
}

#[allow(clippy::module_name_repetitions)]
pub struct TtlCache {
    path: String,
    ttl: Duration,
    pool: Pool<Sqlite>,
    fut_cleaner: JoinHandle<()>,
}

impl Drop for TtlCache {
    fn drop(&mut self) {
        self.fut_cleaner.abort();
    }
}

const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

impl TtlCache {
    #[allow(clippy::cast_possible_wrap)]
    pub async fn create(
        path: &str,
        ttl: Duration,
        timeout: Duration,
        pool_size: u32,
    ) -> EResult<Self> {
        let mut connection_options = SqliteConnectOptions::from_str(&format!("sqlite://{path}"))?
            .create_if_missing(true)
            .synchronous(SqliteSynchronous::Extra)
            .busy_timeout(timeout);
        connection_options
            .log_statements(log::LevelFilter::Trace)
            .log_slow_statements(log::LevelFilter::Warn, Duration::from_secs(2));
        let pool = SqlitePoolOptions::new()
            .max_connections(pool_size)
            .connect_timeout(timeout)
            .connect_with(connection_options)
            .await?;
        sqlx::query("CREATE TABLE IF NOT EXISTS kv(k VARCHAR(256), v BLOB, t INT, PRIMARY KEY(k))")
            .execute(&pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS kv_t ON kv(t)")
            .execute(&pool)
            .await?;
        let p = pool.clone();
        let db_path = path.to_owned();
        let fut_cleaner = tokio::spawn(async move {
            let mut next = Instant::now() + CLEANUP_INTERVAL;
            loop {
                trace!("cleaning up {} cache", db_path);
                if let Err(e) = sqlx::query("DELETE FROM kv WHERE t < ?")
                    .bind((now() - ttl).as_secs() as i64)
                    .execute(&p)
                    .await
                {
                    error!("cache {} error: {}", db_path, e);
                }
                let t = Instant::now();
                if next > t {
                    tokio::time::sleep(next - t).await;
                }
                next += CLEANUP_INTERVAL;
            }
        });
        Ok(Self {
            path: path.to_owned(),
            ttl,
            pool,
            fut_cleaner,
        })
    }
    #[allow(clippy::cast_possible_wrap)]
    pub async fn set<V: Serialize>(&self, key: &str, value: &V) -> EResult<()> {
        trace!("setting {} key {}", self.path, key);
        if key.len() > 256 {
            return Err(Error::invalid_data("key too long"));
        }
        sqlx::query("INSERT OR REPLACE INTO kv (k, v, t) VALUES (?, ?, ?)")
            .bind(key)
            .bind(pack(value)?)
            .bind(now().as_secs() as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn get<V: DeserializeOwned>(&self, key: &str) -> EResult<Option<V>> {
        trace!("getting {} key {}", self.path, key);
        let val: Option<(Vec<u8>,)> = sqlx::query_as("SELECT v FROM kv WHERE k = ? AND t > ?")
            .bind(key)
            .bind((now() - self.ttl).as_secs_f64())
            .fetch_optional(&self.pool)
            .await?;
        if let Some(v) = val {
            Ok(Some(unpack(&v.0)?))
        } else {
            Ok(None)
        }
    }
    pub async fn delete(&self, key: &str) -> EResult<()> {
        trace!("deleting {} key {}", self.path, key);
        sqlx::query("DELETE FROM kv WHERE k = ?")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub async fn purge(&self) -> EResult<()> {
        trace!("deleting all keys in {}", self.path);
        sqlx::query("DELETE FROM kv").execute(&self.pool).await?;
        Ok(())
    }
}
