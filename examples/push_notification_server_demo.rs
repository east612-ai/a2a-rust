//! A2A Push Notification Server Demo
//! 
//! This example demonstrates how to setup a DefaultRequestHandler with
//! PushNotificationSender and ConfigStore to enable automatic push notifications,
//! matching the architecture of the Python server.

use a2a_rust::a2a::models::*;
use a2a_rust::a2a::core_types::{Message, Role, Part};
use a2a_rust::a2a::server::request_handlers::{DefaultRequestHandler, RequestHandler};
use a2a_rust::a2a::server::tasks::{InMemoryTaskStore, InMemoryPushNotificationConfigStore, HttpPushNotificationSender};
use std::sync::Arc;
use url::Url;

#[tokio::main]
async fn main() {
    // 1. Initialize Stores
    let task_store = Arc::new(InMemoryTaskStore::new());
    let push_config_store = Arc::new(InMemoryPushNotificationConfigStore::new());
    
    // 2. Initialize Push Sender
    let push_sender = Arc::new(HttpPushNotificationSender::new(push_config_store.clone()));
    
    // 3. Create Default Request Handler (The core of automatic push)
    let handler = DefaultRequestHandler::new(
        task_store,
        Some(push_config_store),
        Some(push_sender),
    );

    println!("Server initialized with automatic push notification support.");

    // 4. Simulate receiving a message with push configuration
    let message = Message::new(Role::User, vec![Part::text("Start a long task".to_string())]);
    
    let push_config = PushNotificationConfig {
        id: Some("client-callback-1".to_string()),
        url: Url::parse("https://client.example.com/webhook").unwrap(),
        token: Some("client-secret-token".to_string()),
        authentication: None,
    };
    
    let params = MessageSendParams::new(message)
        .with_configuration(MessageSendConfiguration::new()
            .with_push_notification_config(push_config));

    println!("\nProcessing 'message/send' with push config...");
    
    // This call will automatically:
    // - Save the push config to the store
    // - Create a task
    // - Trigger the first push notification (async)
    let result = handler.on_message_send(params, None).await.unwrap();

    if let a2a_rust::a2a::server::request_handlers::MessageSendResult::Task(task) = result {
        println!("Task created: {}", task.id);
        println!("Status: {:?}", task.status.state);
        println!("Push notification has been automatically triggered in the background.");
    }

    println!("\nUsage pattern matches a2a-python's DefaultRequestHandler integration.");
}
