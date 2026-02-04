//! gRPC request handler adapter
//!
//! This module mirrors the JSONRPCHandler but is intended to be used by a
//! future gRPC server implementation. It delegates protocol-specific handling
//! to the core `RequestHandler` trait so that business logic remains shared.
//!
//! Semantics aligned with Python GrpcHandler:
//! - message/stream + tasks/resubscribe require streaming capability
//! - set push_notification requires push_notifications capability
//! - get push_notification DOES NOT gate on push capability
//! - tasks/get + tasks/cancel return Option<Task>; transport maps None -> TaskNotFound

use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;

use crate::a2a::error::A2AError;
use crate::a2a::models::*;
use crate::a2a::server::context::ServerCallContext;
use crate::a2a::server::request_handlers::{
    Event, MessageSendResult, RequestHandler, TaskPushNotificationConfigQueryParams,
};

/// gRPC Handler
///
/// Provides thin async adapters around the core `RequestHandler` trait for a
/// gRPC transport. The transport layer (generated service) should call these
/// helpers to keep protocol handling minimal.
pub struct GRPCHandler {
    agent_card: AgentCard,
    request_handler: Arc<dyn RequestHandler>,
}

impl GRPCHandler {
    /// Create a new gRPC handler adapter
    pub fn new(agent_card: AgentCard, request_handler: Arc<dyn RequestHandler>) -> Self {
        Self {
            agent_card,
            request_handler,
        }
    }

    /// Handle a unary message/send request
    pub async fn handle_message_send(
        &self,
        params: MessageSendParams,
        context: &ServerCallContext,
    ) -> Result<MessageSendResult, A2AError> {
        self.request_handler
            .on_message_send(params, Some(context))
            .await
    }

    /// Handle a server-streaming message/stream request with capability check
    pub async fn handle_message_stream(
        &self,
        params: MessageSendParams,
        context: &ServerCallContext,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Event, A2AError>> + Send>>, A2AError> {
        self.ensure_streaming_supported()?;

        self.request_handler
            .on_message_send_stream(params, Some(context))
            .await
    }

    /// Handle tasks/get
    pub async fn handle_get_task(
        &self,
        params: TaskQueryParams,
        context: &ServerCallContext,
    ) -> Result<Option<Task>, A2AError> {
        self.request_handler
            .on_get_task(params, Some(context))
            .await
    }

    /// Handle tasks/cancel
    pub async fn handle_cancel_task(
        &self,
        params: TaskIdParams,
        context: &ServerCallContext,
    ) -> Result<Option<Task>, A2AError> {
        self.request_handler
            .on_cancel_task(params, Some(context))
            .await
    }

    /// Handle tasks/pushNotificationConfig/set with capability check
    pub async fn handle_set_push_notification_config(
        &self,
        params: TaskPushNotificationConfig,
        context: &ServerCallContext,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        self.ensure_push_supported()?;

        self.request_handler
            .on_set_task_push_notification_config(params, Some(context))
            .await
    }

    /// Handle tasks/pushNotificationConfig/get
    ///
    /// IMPORTANT: Python does NOT gate this endpoint on push_notifications capability.
    pub async fn handle_get_push_notification_config(
        &self,
        params: TaskPushNotificationConfigQueryParams,
        context: &ServerCallContext,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        self.request_handler
            .on_get_task_push_notification_config(params, Some(context))
            .await
    }

    /// Handle tasks/resubscribe (streaming) with capability check
    pub async fn handle_resubscribe_task(
        &self,
        params: TaskIdParams,
        context: &ServerCallContext,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Event, A2AError>> + Send>>, A2AError> {
        self.ensure_streaming_supported()?;

        self.request_handler
            .on_resubscribe_to_task(params, Some(context))
            .await
    }

    /// Handle agent/authenticatedExtendedCard requests (your extension)
    pub async fn handle_get_authenticated_extended_card(
        &self,
        _context: &ServerCallContext,
    ) -> Result<AgentCard, A2AError> {
        if !self
            .agent_card
            .supports_authenticated_extended_card
            .unwrap_or(false)
        {
            return Err(A2AError::unsupported_operation(
                "Authenticated extended card is not supported by this agent",
            ));
        }

        Ok(self.agent_card.clone())
    }

    /// Get the agent card (non-authenticated version)
    pub async fn get_agent_card(
        &self,
        _context: &ServerCallContext,
    ) -> Result<AgentCard, A2AError> {
        Ok(self.agent_card.clone())
    }

    // -------------------------
    // Capability helpers
    // -------------------------

    fn ensure_streaming_supported(&self) -> Result<(), A2AError> {
        if !self.agent_card.capabilities.streaming.unwrap_or(false) {
            // Match Python validate message as closely as possible
            return Err(A2AError::unsupported_operation(
                "Streaming is not supported by the agent",
            ));
        }
        Ok(())
    }

    fn ensure_push_supported(&self) -> Result<(), A2AError> {
        if !self.agent_card.capabilities.push_notifications.unwrap_or(false) {
            return Err(A2AError::push_notification_not_supported());
        }
        Ok(())
    }
}
