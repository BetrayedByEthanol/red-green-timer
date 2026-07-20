use crate::error::PersistenceError;
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};

#[derive(Debug, Clone)]
pub struct TimerRepository {
    pub(crate) pool: SqlitePool,
}

impl TimerRepository {
    pub async fn open(database_url: &str) -> Result<Self, PersistenceError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA busy_timeout = 5000")
            .execute(&pool)
            .await?;
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await?;
        if !database_url.ends_with(":memory:") {
            let _ = sqlx::query("PRAGMA journal_mode = WAL")
                .execute(&pool)
                .await?;
        }
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
