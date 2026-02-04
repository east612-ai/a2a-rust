use a2a_rust::a2a::models::*;
use a2a_rust::a2a::core_types::{Message, Role, Part};
use a2a_rust::a2a::server::request_handlers::{DefaultRequestHandler, RequestHandler};
use a2a_rust::a2a::server::tasks::{InMemoryTaskStore, InMemoryPushNotificationConfigStore, HttpPushNotificationSender};
use std::sync::Arc;
use mockito::Server;

#[tokio::test]
async fn test_default_handler_auto_push() {
    // 1. Setup Mock Push Server
    let mut server = Server::new_async().await;
    let url_str = server.url();
    let url = url_str.parse().unwrap();
    
    let mock = server.mock("POST", "/")
        .with_status(200)
        .create_async()
        .await;

    // 2. Setup Handler with Stores and Sender
    let task_store = Arc::new(InMemoryTaskStore::new());
    let push_config_store = Arc::new(InMemoryPushNotificationConfigStore::new());
    let push_sender = Arc::new(HttpPushNotificationSender::new(push_config_store.clone()));
    
    let handler = DefaultRequestHandler::new(
        task_store,
        Some(push_config_store),
        Some(push_sender),
    );

    // 3. Send Message with Push Config
    let message = Message::new(Role::User, vec![Part::text("Hello".to_string())]);
    let config = PushNotificationConfig {
        id: Some("cfg1".to_string()),
        url,
        token: Some("test-token".to_string()),
        authentication: None,
    };
    
    let params = MessageSendParams::new(message)
        .with_configuration(MessageSendConfiguration::new().with_push_notification_config(config));

    // 4. Execute on_message_send
    let result = handler.on_message_send(params, None).await.unwrap();
    
    // 5. Verify result and auto-push
    if let a2a_rust::a2a::server::request_handlers::MessageSendResult::Task(task) = result {
        assert_eq!(task.status.state, a2a_rust::a2a::core_types::TaskState::Working);
    } else {
        panic!("Expected Task result");
    }

    // Wait a bit for async push to complete if necessary (though HttpPushNotificationSender is awaited in DefaultRequestHandler)
    mock.assert_async().await;
}
