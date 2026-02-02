//! Push Notification Configuration Store interface and implementations
//! 
//! This module defines the interface for persisting and retrieving 
//! push notification configurations.

use crate::{PushNotificationConfig, A2AError};
use async_trait::async_trait;

/// Push Notification Config Store interface
#[async_trait]
pub trait PushNotificationConfigStore: Send + Sync {
    /// Sets or updates the push notification configuration for a task
    async fn set_info(&self, task_id: &str, config: PushNotificationConfig) -> Result<(), A2AError>;
    
    /// Retrieves all push notification configurations for a task
    async fn get_info(&self, task_id: &str) -> Result<Vec<PushNotificationConfig>, A2AError>;
    
    /// Deletes push notification configurations for a task
    /// 
    /// If config_id is provided, only that specific configuration is deleted.
    /// If config_id is None, all configurations for the task are deleted.
    async fn delete_info(&self, task_id: &str, config_id: Option<&str>) -> Result<(), A2AError>;
}

/// In-memory implementation of PushNotificationConfigStore
pub struct InMemoryPushNotificationConfigStore {
    configs: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, Vec<PushNotificationConfig>>>>,
}

impl InMemoryPushNotificationConfigStore {
    pub fn new() -> Self {
        Self {
            configs: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

impl Default for InMemoryPushNotificationConfigStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PushNotificationConfigStore for InMemoryPushNotificationConfigStore {
    async fn set_info(&self, task_id: &str, config: PushNotificationConfig) -> Result<(), A2AError> {
        let mut configs = self.configs.write().await;
        let task_configs = configs.entry(task_id.to_string()).or_insert_with(Vec::new);
        
        // If config has an ID, check if it already exists and update it
        if let Some(ref config_id) = config.id {
            if let Some(pos) = task_configs.iter().position(|c| c.id.as_ref() == Some(config_id)) {
                task_configs[pos] = config;
                return Ok(());
            }
        }
        
        task_configs.push(config);
        Ok(())
    }
    
    async fn get_info(&self, task_id: &str) -> Result<Vec<PushNotificationConfig>, A2AError> {
        let configs = self.configs.read().await;
        Ok(configs.get(task_id).cloned().unwrap_or_default())
    }
    
    async fn delete_info(&self, task_id: &str, config_id: Option<&str>) -> Result<(), A2AError> {
        let mut configs = self.configs.write().await;
        if let Some(config_id) = config_id {
            if let Some(task_configs) = configs.get_mut(task_id) {
                task_configs.retain(|c| c.id.as_ref().map(|s| s.as_str()) != Some(config_id));
            }
        } else {
            configs.remove(task_id);
        }
        Ok(())
    }
}
