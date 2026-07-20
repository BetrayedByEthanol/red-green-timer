use crate::{
    error::PersistenceError, mapping::*, model::PersistedTimer, repository::TimerRepository,
};
use sqlx::Row;
use std::time::{Duration, SystemTime};
use timer_core::TimerDefinition;
use uuid::Uuid;

impl TimerRepository {
    pub async fn create_timer(&self, definition: &TimerDefinition) -> Result<(), PersistenceError> {
        let now = system_time_to_unix_ns(SystemTime::now())?;
        sqlx::query("INSERT INTO timers (id,name,green_duration_ns,red_duration_ns,created_at_ns,updated_at_ns) VALUES (?,?,?,?,?,?)")
            .bind(definition.id.to_string()).bind(definition.name.trim()).bind(duration_to_ns(definition.green_duration)?).bind(duration_to_ns(definition.red_duration)?).bind(now).bind(now).execute(&self.pool).await?;
        Ok(())
    }
    pub async fn timer_count(&self) -> Result<i64, PersistenceError> {
        Ok(sqlx::query("SELECT COUNT(*) c FROM timers")
            .fetch_one(&self.pool)
            .await?
            .get("c"))
    }
    pub async fn seed_default_timer_if_empty(
        &self,
    ) -> Result<Option<PersistedTimer>, PersistenceError> {
        if self.timer_count().await? != 0 {
            return Ok(None);
        }
        let def = TimerDefinition::new(
            Uuid::new_v4(),
            "Red-Green Light",
            Duration::from_secs(40),
            Duration::from_secs(20),
        )
        .map_err(|e| PersistenceError::InvalidStoredValue(e.to_string()))?;
        self.create_timer(&def).await?;
        self.get_timer(def.id).await
    }
    pub async fn get_timer(
        &self,
        timer_id: Uuid,
    ) -> Result<Option<PersistedTimer>, PersistenceError> {
        let row = sqlx::query("SELECT * FROM timers WHERE id=?")
            .bind(timer_id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_timer).transpose()
    }
    pub async fn list_active_timers(&self) -> Result<Vec<PersistedTimer>, PersistenceError> {
        let rows = sqlx::query(
            "SELECT * FROM timers WHERE archived_at_ns IS NULL ORDER BY created_at_ns ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_timer).collect()
    }
    pub async fn update_timer(&self, definition: &TimerDefinition) -> Result<(), PersistenceError> {
        let old = self
            .get_timer(definition.id)
            .await?
            .ok_or(PersistenceError::TimerNotFound(definition.id))?;
        if old.archived_at.is_some() {
            return Err(PersistenceError::TimerArchived(definition.id));
        }
        let now = system_time_to_unix_ns(SystemTime::now())?;
        let res=sqlx::query("UPDATE timers SET name=?, green_duration_ns=?, red_duration_ns=?, updated_at_ns=? WHERE id=? AND archived_at_ns IS NULL").bind(definition.name.trim()).bind(duration_to_ns(definition.green_duration)?).bind(duration_to_ns(definition.red_duration)?).bind(now).bind(definition.id.to_string()).execute(&self.pool).await?;
        if res.rows_affected() == 0 {
            Err(PersistenceError::TimerNotFound(definition.id))
        } else {
            Ok(())
        }
    }
    pub async fn archive_timer(
        &self,
        timer_id: Uuid,
        archived_at: SystemTime,
    ) -> Result<(), PersistenceError> {
        let old = self
            .get_timer(timer_id)
            .await?
            .ok_or(PersistenceError::TimerNotFound(timer_id))?;
        if old.archived_at.is_some() {
            return Err(PersistenceError::TimerArchived(timer_id));
        }
        sqlx::query("UPDATE timers SET archived_at_ns=?, updated_at_ns=? WHERE id=?")
            .bind(system_time_to_unix_ns(archived_at)?)
            .bind(system_time_to_unix_ns(archived_at)?)
            .bind(timer_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn row_timer(row: sqlx::sqlite::SqliteRow) -> Result<PersistedTimer, PersistenceError> {
    let id: String = row.get("id");
    let name: String = row.get("name");
    let archived: Option<i64> = row.get("archived_at_ns");
    Ok(PersistedTimer {
        definition: TimerDefinition::new(
            uuid_from_db(id)?,
            name,
            ns_to_duration(row.get("green_duration_ns"))?,
            ns_to_duration(row.get("red_duration_ns"))?,
        )
        .map_err(|e| PersistenceError::InvalidStoredValue(e.to_string()))?,
        created_at: unix_ns_to_system_time(row.get("created_at_ns"))?,
        updated_at: unix_ns_to_system_time(row.get("updated_at_ns"))?,
        archived_at: archived.map(unix_ns_to_system_time).transpose()?,
    })
}
