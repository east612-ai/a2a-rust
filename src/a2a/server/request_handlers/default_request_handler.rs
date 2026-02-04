//! Default request handler implementation
//! 
//! This module provides the DefaultRequestHandler which coordinates between
//! TaskStore, PushNotificationSender, and other components, mirroring the
//! Python implementation.

use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use std::sync::Arc;
use tracing::error;

use crate::a2a::models::*;
use crate::a2a::core_types::{TaskStatus, TaskState};
use crate::a2a::server::context::ServerCallContext;
use crate::a2a::server::request_handlers::request_handler::{RequestHandler, MessageSendResult, Event};
use crate::a2a::server::tasks::{TaskStore, PushNotificationConfigStore, PushNotificationSender, TaskManager};
use crate::a2a::error::A2AError;

/// Default Request Handler
pub struct DefaultRequestHandler {
    task_store: Arc<dyn TaskStore>,
    push_config_store: Option<Arc<dyn PushNotificationConfigStore>>,
    push_sender: Option<Arc<dyn PushNotificationSender>>,
}

impl DefaultRequestHandler {
    /// Create a new DefaultRequestHandler
    pub fn new(
        task_store: Arc<dyn TaskStore>,
        push_config_store: Option<Arc<dyn PushNotificationConfigStore>>,
        push_sender: Option<Arc<dyn PushNotificationSender>>,
    ) -> Self {
        Self {
            task_store,
            push_config_store,
            push_sender,
        }
    }

    async fn send_push_notification_if_needed(&self, task: &Task) {
        if let Some(ref sender) = self.push_sender {
            if let Err(e) = sender.send_notification(task).await {
                error!("Failed to send push notification: {}", e);
            }
        }
    }
}

#[async_trait]
impl RequestHandler for DefaultRequestHandler {
    async fn on_get_task(
        &self,
        params: TaskQueryParams,
        _context: Option<&ServerCallContext>,
    ) -> Result<Option<Task>, A2AError> {
        self.task_store.get(&params.id).await
    }

    async fn on_cancel_task(
        &self,
        params: TaskIdParams,
        _context: Option<&ServerCallContext>,
    ) -> Result<Option<Task>, A2AError> {
        let task = self.task_store.get(&params.id).await?;
        if let Some(mut task) = task {
            task.status.state = TaskState::Canceled;
            task.status.timestamp = Some(chrono::Utc::now().to_string());
            self.task_store.save(task.clone()).await?;
            
            // Trigger push notification on cancellation
            self.send_push_notification_if_needed(&task).await;
            
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    async fn on_message_send(
        &self,
        params: MessageSendParams,
        _context: Option<&ServerCallContext>,
    ) -> Result<MessageSendResult, A2AError> {
        let task_id = params.message.task_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let context_id = params.message.context_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let mut task_manager = TaskManager::new(
            Some(task_id.clone()),
            Some(context_id.clone()),
            self.task_store.clone(),
            Some(params.message.clone()),
            None,
        )?;

        // Handle push config if provided in params
        if let Some(ref config_store) = self.push_config_store {
            if let Some(config) = params.configuration.as_ref().and_then(|c| c.push_notification_config.clone()) {
                config_store.set_info(&task_id, config).await?;
            }
        }

        // Mock execution: just return a task in Working state
        let task = task_manager.save_task_event(crate::a2a::server::tasks::TaskEvent::Task(Task {
            id: task_id,
            context_id,
            status: TaskStatus::new(TaskState::Working),
            artifacts: None,
            history: Some(vec![params.message.clone()]),
            metadata: None,
            kind: "task".to_string(),
        })).await?;

        // Trigger push notification
        self.send_push_notification_if_needed(&task).await;

        Ok(MessageSendResult::Task(task))
    }

    async fn on_message_send_stream(
        &self,
        params: MessageSendParams,
        _context: Option<&ServerCallContext>,
    ) -> Result<BoxStream<'static, Result<Event, A2AError>>, A2AError> {
        let task_id = params.message.task_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let context_id = params.message.context_id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        // Handle push config
        if let Some(ref config_store) = self.push_config_store {
            if let Some(config) = params.configuration.as_ref().and_then(|c| c.push_notification_config.clone()) {
                config_store.set_info(&task_id, config).await?;
            }
        }

        let task = Task {
            id: task_id.clone(),
            context_id: context_id.clone(),
            status: TaskStatus::new(TaskState::Working),
            artifacts: None,
            history: Some(vec![params.message.clone()]),
            metadata: None,
            kind: "task".to_string(),
        };

        // In a real implementation, we would wrap the stream to trigger push notifications
        // on each event. For now, we'll just return a mock stream.
        let sender = self.push_sender.clone();
        let task_clone = task.clone();

        let stream = futures::stream::iter(vec![
            Ok(Event::Task(task.clone())),
            Ok(Event::TaskStatusUpdate(TaskStatusUpdateEvent::new(
                task_id.clone(),
                context_id.clone(),
                TaskStatus::new(TaskState::Completed),
                true,
            ))),
        ]).then(move |res| {
            let sender = sender.clone();
            let task = task_clone.clone();
            async move {
                if let Ok(_) = res {
                    if let Some(ref s) = sender {
                        let _ = s.send_notification(&task).await;
                    }
                }
                res
            }
        });

        Ok(Box::pin(stream))
    }

    async fn on_set_task_push_notification_config(
        &self,
        params: TaskPushNotificationConfig,
        _context: Option<&ServerCallContext>,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        if let Some(ref store) = self.push_config_store {
            store.set_info(&params.task_id, params.push_notification_config.clone()).await?;
            Ok(params)
        } else {
            Err(A2AError::unsupported_operation("Push notification config store not configured"))
        }
    }

    async fn on_get_task_push_notification_config(
        &self,
        params: crate::a2a::server::request_handlers::request_handler::TaskPushNotificationConfigQueryParams,
        _context: Option<&ServerCallContext>,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        if let Some(ref store) = self.push_config_store {
            let configs = store.get_info(&params.task_id).await?;
            if let Some(config) = configs.into_iter().next() {
                Ok(TaskPushNotificationConfig::new(params.task_id, config))
            } else {
                Err(A2AError::internal("Push notification config not found"))
            }
        } else {
            Err(A2AError::unsupported_operation("Push notification config store not configured"))
        }
    }

    async fn on_list_task_push_notification_config(
        &self,
        params: TaskIdParams,
        _context: Option<&ServerCallContext>,
    ) -> Result<Vec<TaskPushNotificationConfig>, A2AError> {
        if let Some(ref store) = self.push_config_store {
            let configs = store.get_info(&params.id).await?;
            Ok(configs.into_iter().map(|c| TaskPushNotificationConfig::new(params.id.clone(), c)).collect())
        } else {
            Err(A2AError::unsupported_operation("Push notification config store not configured"))
        }
    }

    async fn on_delete_task_push_notification_config(
        &self,
        params: DeleteTaskPushNotificationConfigParams,
        _context: Option<&ServerCallContext>,
    ) -> Result<(), A2AError> {
        if let Some(ref store) = self.push_config_store {
            store.delete_info(&params.id, Some(&params.push_notification_config_id)).await
        } else {
            Err(A2AError::unsupported_operation("Push notification config store not configured"))
        }
    }
}
