//! SQL implementation of TaskStore using sqlx
//! 
//! This module provides a persistent task store implementation using sqlx
//! with support for SQLite.

use crate::{Task, A2AError};
use crate::a2a::server::tasks::task_store::TaskStore;
use async_trait::async_trait;
use sqlx::{SqlitePool, sqlite::SqliteConnectOptions};
use std::str::FromStr;

/// SQLite implementation of TaskStore
pub struct SqliteTaskStore {
    pool: SqlitePool,
    table_name: String,
}

impl SqliteTaskStore {
    /// Creates a new SqliteTaskStore with the given connection pool
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            table_name: "tasks".to_string(),
        }
    }

    /// Creates a new SqliteTaskStore with a custom table name
    pub fn with_table_name(pool: SqlitePool, table_name: String) -> Self {
        Self {
            pool,
            table_name,
        }
    }

    /// Connects to a SQLite database and initializes the store
    pub async fn connect(url: &str) -> Result<Self, A2AError> {
        let options = SqliteConnectOptions::from_str(url)
            .map_err(|e| A2AError::internal(&format!("Invalid database URL: {}", e)))?
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| A2AError::internal(&format!("Failed to connect to database: {}", e)))?;

        let store = Self::new(pool);
        store.initialize().await?;
        Ok(store)
    }

    /// Initializes the database schema
    pub async fn initialize(&self) -> Result<(), A2AError> {
        let query = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                context_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                status TEXT NOT NULL,
                artifacts TEXT,
                history TEXT,
                metadata TEXT
            )",
            self.table_name
        );

        sqlx::query(&query)
            .execute(&self.pool)
            .await
            .map_err(|e| A2AError::internal(&format!("Failed to initialize database: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl TaskStore for SqliteTaskStore {
    async fn save(&self, task: Task) -> Result<(), A2AError> {
        let query = format!(
            "INSERT OR REPLACE INTO {} (id, context_id, kind, status, artifacts, history, metadata)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            self.table_name
        );

        let status_json = serde_json::to_string(&task.status)
            .map_err(|e| A2AError::internal(&format!("Failed to serialize status: {}", e)))?;
        
        let artifacts_json = task.artifacts.as_ref().map(|a| serde_json::to_string(a))
            .transpose()
            .map_err(|e| A2AError::internal(&format!("Failed to serialize artifacts: {}", e)))?;
            
        let history_json = task.history.as_ref().map(|h| serde_json::to_string(h))
            .transpose()
            .map_err(|e| A2AError::internal(&format!("Failed to serialize history: {}", e)))?;
            
        let metadata_json = task.metadata.as_ref().map(|m| serde_json::to_string(m))
            .transpose()
            .map_err(|e| A2AError::internal(&format!("Failed to serialize metadata: {}", e)))?;

        sqlx::query(&query)
            .bind(&task.id)
            .bind(&task.context_id)
            .bind(task.kind)
            .bind(status_json)
            .bind(artifacts_json)
            .bind(history_json)
            .bind(metadata_json)
            .execute(&self.pool)
            .await
            .map_err(|e| A2AError::internal(&format!("Failed to save task: {}", e)))?;

        Ok(())
    }

    async fn get(&self, task_id: &str) -> Result<Option<Task>, A2AError> {
        let query = format!(
            "SELECT id, context_id, kind, status, artifacts, history, metadata FROM {} WHERE id = ?",
            self.table_name
        );

        let row = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, Option<String>)>(&query)
            .bind(task_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| A2AError::internal(&format!("Failed to get task: {}", e)))?;

        if let Some((id, context_id, kind, status_json, artifacts_json, history_json, metadata_json)) = row {
            let status = serde_json::from_str(&status_json)
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize status: {}", e)))?;
                
            let artifacts = artifacts_json.map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize artifacts: {}", e)))?;
                
            let history = history_json.map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize history: {}", e)))?;
                
            let metadata = metadata_json.map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize metadata: {}", e)))?;

            Ok(Some(Task {
                id,
                context_id,
                kind,
                status,
                artifacts,
                history,
                metadata,
            }))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, task_id: &str) -> Result<(), A2AError> {
        let query = format!("DELETE FROM {} WHERE id = ?", self.table_name);

        sqlx::query(&query)
            .bind(task_id)
            .execute(&self.pool)
            .await
            .map_err(|e| A2AError::internal(&format!("Failed to delete task: {}", e)))?;

        Ok(())
    }

    async fn list(&self) -> Result<Vec<Task>, A2AError> {
        let query = format!(
            "SELECT id, context_id, kind, status, artifacts, history, metadata FROM {}",
            self.table_name
        );

        let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, Option<String>)>(&query)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| A2AError::internal(&format!("Failed to list tasks: {}", e)))?;

        let mut tasks = Vec::new();
        for (id, context_id, kind, status_json, artifacts_json, history_json, metadata_json) in rows {
            let status = serde_json::from_str(&status_json)
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize status: {}", e)))?;
                
            let artifacts = artifacts_json.map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize artifacts: {}", e)))?;
                
            let history = history_json.map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize history: {}", e)))?;
                
            let metadata = metadata_json.map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize metadata: {}", e)))?;

            tasks.push(Task {
                id,
                context_id,
                kind,
                status,
                artifacts,
                history,
                metadata,
            });
        }
        Ok(tasks)
    }

    async fn list_by_context(&self, context_id: &str) -> Result<Vec<Task>, A2AError> {
        let query = format!(
            "SELECT id, context_id, kind, status, artifacts, history, metadata FROM {} WHERE context_id = ?",
            self.table_name
        );

        let rows = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<String>, Option<String>)>(&query)
            .bind(context_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| A2AError::internal(&format!("Failed to list tasks by context: {}", e)))?;

        let mut tasks = Vec::new();
        for (id, context_id, kind, status_json, artifacts_json, history_json, metadata_json) in rows {
            let status = serde_json::from_str(&status_json)
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize status: {}", e)))?;
                
            let artifacts = artifacts_json.map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize artifacts: {}", e)))?;
                
            let history = history_json.map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize history: {}", e)))?;
                
            let metadata = metadata_json.map(|s| serde_json::from_str(&s))
                .transpose()
                .map_err(|e| A2AError::internal(&format!("Failed to deserialize metadata: {}", e)))?;

            tasks.push(Task {
                id,
                context_id,
                kind,
                status,
                artifacts,
                history,
                metadata,
            });
        }
        Ok(tasks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TaskStatus, TaskState};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_sqlite_task_store() {
        let store = SqliteTaskStore::connect("sqlite::memory:").await.unwrap();
        
        let task_id = Uuid::new_v4().to_string();
        let context_id = Uuid::new_v4().to_string();
        let task = Task {
            id: task_id.clone(),
            context_id: context_id.clone(),
            status: TaskStatus {
                state: TaskState::Submitted,
                timestamp: Some(chrono::Utc::now().to_rfc3339()),
                message: None,
            },
            artifacts: None,
            history: None,
            metadata: None,
            kind: "task".to_string(),
        };

        // Test save
        store.save(task.clone()).await.unwrap();

        // Test get
        let retrieved = store.get(&task_id.to_string()).await.unwrap().unwrap();
        assert_eq!(retrieved.id, task_id);
        assert_eq!(retrieved.context_id, context_id);
        assert_eq!(retrieved.status.state, TaskState::Submitted);

        // Test update
        let mut updated_task = task.clone();
        updated_task.status.state = TaskState::Completed;
        store.save(updated_task).await.unwrap();
        
        let retrieved_updated = store.get(&task_id.to_string()).await.unwrap().unwrap();
        assert_eq!(retrieved_updated.status.state, TaskState::Completed);

        // Test list
        let tasks = store.list().await.unwrap();
        assert_eq!(tasks.len(), 1);

        // Test delete
        store.delete(&task_id.to_string()).await.unwrap();
        let deleted = store.get(&task_id.to_string()).await.unwrap();
        assert!(deleted.is_none());
    }
}
