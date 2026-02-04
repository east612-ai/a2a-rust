//! Task Aware A2A Server Example
//! 
//! This example demonstrates how to create a basic A2A server with task management
//! using the a2a-rust library, following the same pattern as the rust_server example.

use a2a_rust::a2a::{
    models::*,
    server::{
        apps::jsonrpc::{A2AServerBuilder, ServerConfig},
        context::DefaultServerCallContextBuilder,
        request_handlers::{RequestHandler, MessageSendResult, TaskPushNotificationConfigQueryParams},
    },
    core_types::{Message, Part, Role, TaskState, TaskStatus},
    error::A2AError,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// Simple task-aware request handler
struct TaskAwareHandler {
    // In-memory task storage
    tasks: Arc<Mutex<HashMap<String, Task>>>,
}

impl TaskAwareHandler {
    fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a unique task ID
    fn generate_task_id() -> String {
        format!("task-{}", Uuid::new_v4())
    }
}

#[async_trait::async_trait]
impl RequestHandler for TaskAwareHandler {
    async fn on_get_task(
        &self,
        params: TaskQueryParams,
        _context: Option<&a2a_rust::a2a::server::context::ServerCallContext>,
    ) -> Result<Option<Task>, A2AError> {
        let tasks = self.tasks.lock().unwrap();
        Ok(tasks.get(&params.id).cloned())
    }

    async fn on_cancel_task(
        &self,
        params: TaskIdParams,
        _context: Option<&a2a_rust::a2a::server::context::ServerCallContext>,
    ) -> Result<Option<Task>, A2AError> {
        let mut tasks = self.tasks.lock().unwrap();
        
        if let Some(mut task) = tasks.get(&params.id).cloned() {
            // Update task status to canceled
            task.status = TaskStatus {
                state: TaskState::Canceled,
                message: None,
                timestamp: None,
            };
            
            // Store updated task
            tasks.insert(params.id.clone(), task.clone());
            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    async fn on_message_send(
        &self,
        params: MessageSendParams,
        _context: Option<&a2a_rust::a2a::server::context::ServerCallContext>,
    ) -> Result<MessageSendResult, A2AError> {
        let task_id = Self::generate_task_id();
        let context_id = params.message.context_id.clone().unwrap_or_else(|| "default-context".to_string());
        
        // Create a simple task
        let task = Task {
            id: task_id.clone(),
            context_id: context_id.clone(),
            status: TaskStatus::new(TaskState::Completed),
            artifacts: None,
            history: None,
            metadata: None,
            kind: "task".to_string(),
        };

        // Store the task
        {
            let mut tasks = self.tasks.lock().unwrap();
            tasks.insert(task_id.clone(), task.clone());
        }

        // Create response message
        let response_text = format!("Task {} processed successfully: received {} parts", 
            task_id, params.message.parts.len());
        
        let response_message = Message::new(Role::Agent, vec![
            Part::text(response_text.clone())
        ])
        .with_context_id(context_id.clone())
        .with_task_id(task_id.clone());

        Ok(MessageSendResult::Task(task))
    }

    async fn on_set_task_push_notification_config(
        &self,
        _params: TaskPushNotificationConfig,
        _context: Option<&a2a_rust::a2a::server::context::ServerCallContext>,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        Err(A2AError::unsupported_operation("Push notifications not supported"))
    }

    async fn on_get_task_push_notification_config(
        &self,
        _params: TaskPushNotificationConfigQueryParams,
        _context: Option<&a2a_rust::a2a::server::context::ServerCallContext>,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        Err(A2AError::unsupported_operation("Push notifications not supported"))
    }

    async fn on_list_task_push_notification_config(
        &self,
        _params: TaskIdParams,
        _context: Option<&a2a_rust::a2a::server::context::ServerCallContext>,
    ) -> Result<Vec<TaskPushNotificationConfig>, A2AError> {
        Ok(vec![])
    }

    async fn on_delete_task_push_notification_config(
        &self,
        _params: DeleteTaskPushNotificationConfigParams,
        _context: Option<&a2a_rust::a2a::server::context::ServerCallContext>,
    ) -> Result<(), A2AError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create agent card with basic capabilities
    let agent_card = AgentCard::new(
        "Task Aware Server".to_string(),
        "A simple task-aware server implemented in Rust".to_string(),
        "http://localhost:8081".to_string(),
        "1.0.0".to_string(),
        vec!["text/plain".to_string(), "application/json".to_string()],
        vec!["text/plain".to_string(), "application/json".to_string()],
        AgentCapabilities::new(),
        vec![],
    );

    // Create request handler
    let request_handler = Arc::new(TaskAwareHandler::new());

    // Create context builder
    let context_builder = Arc::new(DefaultServerCallContextBuilder);

    // Configure server
    let config = ServerConfig {
        bind_addr: "127.0.0.1:8081".parse::<SocketAddr>()?,
        ..Default::default()
    };

    // Build and start server
    let server = A2AServerBuilder::new()
        .with_agent_card(agent_card)
        .with_request_handler(request_handler)
        .with_context_builder(context_builder)
        .with_config(config)
        .build()?;

    println!("🚀 Starting Task Aware A2A Server on http://127.0.0.1:8081");
    println!("📋 Agent Card available at: http://127.0.0.1:8081/.well-known/agent.json");
    println!("🔌 JSON-RPC endpoint at: http://127.0.0.1:8081/rpc");
    println!("✨ Server is ready to accept connections!");
    println!();
    println!("✅ Features:");
    println!("   • Task management (create, get, cancel)");
    println!("   • Basic message processing");
    println!("   • JSON-RPC protocol support");

    // Start the server
    server.serve().await?;

    Ok(())
}