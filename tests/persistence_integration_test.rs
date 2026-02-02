//! Persistence Integration Tests
//! 
//! This module contains integration tests for SQLite-based persistence
//! in the A2A server.

use a2a_rust::a2a::{
    models::*,
    core_types::*,
    server::tasks::{SqliteTaskStore, SqlitePushNotificationConfigStore, TaskStore, PushNotificationConfigStore},
};
use url::Url;
use tokio;

#[tokio::test]
async fn test_full_persistence_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Task Store
    let task_store = SqliteTaskStore::connect("sqlite::memory:").await?;
    task_store.initialize().await?;

    // 2. Create multiple tasks
    for i in 1..=5 {
        let task = Task::new(
            "context-1".to_string(),
            TaskStatus::new(TaskState::Submitted),
        ).with_task_id(format!("task-{}", i));
        task_store.save(task).await?;
    }

    // 3. Verify count and retrieval
    let tasks = task_store.list().await?;
    assert_eq!(tasks.len(), 5);

    let task3 = task_store.get("task-3").await?.expect("Task 3 should exist");
    assert_eq!(task3.id, "task-3");
    assert_eq!(task3.status.state, TaskState::Submitted);

    // 4. Update a task and verify persistence
    let mut updated_task3 = task3;
    updated_task3.status.state = TaskState::Working;
    task_store.save(updated_task3).await?;

    let retrieved_task3 = task_store.get("task-3").await?.expect("Task 3 should exist");
    assert_eq!(retrieved_task3.status.state, TaskState::Working);

    // 5. Delete a task
    task_store.delete("task-1").await?;
    let tasks_after_delete = task_store.list().await?;
    assert_eq!(tasks_after_delete.len(), 4);
    assert!(task_store.get("task-1").await?.is_none());

    Ok(())
}

#[tokio::test]
async fn test_encrypted_push_config_persistence() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Push Config Store with encryption
    let encryption_key = [0u8; 32]; // Use a dummy key for testing
    let push_store = SqlitePushNotificationConfigStore::connect("sqlite::memory:", Some(encryption_key)).await?;
    push_store.initialize().await?;

    // 2. Create a push config
    let task_id = "task-push-123";
    let config = PushNotificationConfig {
        id: Some("config-1".to_string()),
        url: Url::parse("https://example.com/push")?,
        token: Some("secret-token-789".to_string()),
        authentication: None,
    };

    // 3. Save and retrieve
    push_store.set_info(task_id, config.clone()).await?;
    
    let retrieved_configs = push_store.get_info(task_id).await?;
    assert_eq!(retrieved_configs.len(), 1);
    assert_eq!(retrieved_configs[0].token, Some("secret-token-789".to_string()));
    assert_eq!(retrieved_configs[0].url.as_str(), "https://example.com/push");

    // 4. Verify deletion
    push_store.delete_info(task_id, Some("config-1")).await?;
    let configs_after_delete = push_store.get_info(task_id).await?;
    assert_eq!(configs_after_delete.len(), 0);

    Ok(())
}
