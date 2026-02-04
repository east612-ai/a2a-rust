//! Push Notification Sender interface and implementations
//! 
//! This module defines the interface for sending push notifications
//! to external services when task events occur.

use crate::{Task, A2AError};
use crate::a2a::server::tasks::PushNotificationConfigStore;
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn, error};

/// Push Notification Sender interface
#[async_trait]
pub trait PushNotificationSender: Send + Sync {
    /// Sends a push notification for a task
    async fn send_notification(&self, task: &Task) -> Result<(), A2AError>;
}

/// HTTP implementation of PushNotificationSender
pub struct HttpPushNotificationSender {
    client: reqwest::Client,
    config_store: Arc<dyn PushNotificationConfigStore>,
}

impl HttpPushNotificationSender {
    /// Creates a new HttpPushNotificationSender
    pub fn new(config_store: Arc<dyn PushNotificationConfigStore>) -> Self {
        Self {
            client: reqwest::Client::new(),
            config_store,
        }
    }

    /// Creates a new HttpPushNotificationSender with a custom reqwest client
    pub fn with_client(client: reqwest::Client, config_store: Arc<dyn PushNotificationConfigStore>) -> Self {
        Self {
            client,
            config_store,
        }
    }

    async fn dispatch_notification(&self, task: &Task, url: String, token: Option<String>) -> bool {
        let mut request = self.client.post(&url).json(task);
        
        if let Some(ref token) = token {
            request = request.header("X-A2A-Notification-Token", token);
        }

        match request.send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("Push-notification sent for task_id={} to URL: {}", task.id, url);
                    true
                } else {
                    warn!("Push-notification failed for task_id={} to URL: {}. Status: {}", task.id, url, response.status());
                    false
                }
            }
            Err(e) => {
                error!("Error sending push-notification for task_id={} to URL: {}. Error: {}", task.id, url, e);
                false
            }
        }
    }
}

#[async_trait]
impl PushNotificationSender for HttpPushNotificationSender {
    async fn send_notification(&self, task: &Task) -> Result<(), A2AError> {
        let configs = self.config_store.get_info(&task.id).await?;
        if configs.is_empty() {
            return Ok(());
        }

        let mut futures = Vec::new();
        for config in configs {
            let url = config.url.to_string();
            let token = config.token.clone();
            futures.push(self.dispatch_notification(task, url, token));
        }

        let results = futures::future::join_all(futures).await;
        
        if results.iter().any(|&r| !r) {
            warn!("Some push notifications failed to send for task_id={}", task.id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TaskStatus, TaskState};
    use crate::a2a::server::tasks::InMemoryPushNotificationConfigStore;
    use crate::PushNotificationConfig;
    use mockito::Server;

    #[tokio::test]
    async fn test_http_push_sender_success() {
        let mut server = Server::new_async().await;
        let url_str = server.url();
        let url = url_str.parse().unwrap();
        
        let mock = server.mock("POST", "/")
            .with_status(200)
            .create_async()
            .await;

        let config_store = Arc::new(InMemoryPushNotificationConfigStore::new());
        let task_id = "test-task-123";
        
        config_store.set_info(task_id, PushNotificationConfig {
            id: Some("cfg1".to_string()),
            url,
            token: Some("secret-token".to_string()),
            authentication: None,
        }).await.unwrap();

        let sender = HttpPushNotificationSender::new(config_store);
        let task = Task {
            id: task_id.to_string(),
            context_id: "ctx-456".to_string(),
            status: TaskStatus {
                state: TaskState::Completed,
                timestamp: None,
                message: None,
            },
            artifacts: None,
            history: None,
            metadata: None,
            kind: "task".to_string(),
        };

        sender.send_notification(&task).await.unwrap();
        mock.assert_async().await;
    }
}
