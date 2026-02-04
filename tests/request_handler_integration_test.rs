//! Integration tests for request handlers
//! 
//! This module tests the request handler implementations to ensure all interfaces
//! are working correctly, following the same pattern as existing tests in the project.

use a2a_rust::a2a::{
    core_types::{Message, Part, Role},
    models::*,
    server::{
        apps::jsonrpc::{A2AServerBuilder, ServerConfig},
        context::DefaultServerCallContextBuilder,
        request_handlers::request_handler::MockRequestHandler,
    },
    utils::constants::*,
};
use axum::{
    body::Body,
    http::{Request, StatusCode, Method},
    response::Response,
    Router,
};
use serde_json::json;
use std::sync::Arc;
use tower::util::ServiceExt;

/// Helper function to create a test agent card
fn create_test_agent_card() -> AgentCard {
    AgentCard::new(
        "Test Agent".to_string(),
        "A test agent for testing".to_string(),
        "http://localhost:8080".to_string(),
        "1.0.0".to_string(),
        vec!["text/plain".to_string()],
        vec!["text/plain".to_string()],
        AgentCapabilities::new(),
        vec![],
    )
}

#[tokio::test]
async fn test_jsonrpc_message_send() {
    let agent_card = create_test_agent_card();
    let request_handler = Arc::new(MockRequestHandler::new());
    let context_builder = Arc::new(DefaultServerCallContextBuilder);

    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };

    let server = A2AServerBuilder::new()
        .with_agent_card(agent_card)
        .with_request_handler(request_handler)
        .with_context_builder(context_builder)
        .with_config(config)
        .build()
        .unwrap();

    let router: Router = server.build_router().await;

    // Test JSON-RPC endpoint with a valid message/send request
    let jsonrpc_request = json!({
        "jsonrpc": "2.0",
        "method": "message/send",
        "params": {
            "message": {
                "kind": "message",
                "messageId": "test-msg-123",
                "role": "user",
                "parts": [
                    {
                        "kind": "text",
                        "text": "Hello, world!"
                    }
                ]
            }
        },
        "id": 1
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri(DEFAULT_RPC_URL)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&jsonrpc_request).unwrap()))
        .unwrap();

    let response: Response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response_json["jsonrpc"], "2.0");
    assert_eq!(response_json["id"], 1);
    assert!(response_json["result"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_invalid_method() {
    let agent_card = create_test_agent_card();
    let request_handler = Arc::new(MockRequestHandler::new());
    let context_builder = Arc::new(DefaultServerCallContextBuilder);

    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };

    let server = A2AServerBuilder::new()
        .with_agent_card(agent_card)
        .with_request_handler(request_handler)
        .with_context_builder(context_builder)
        .with_config(config)
        .build()
        .unwrap();

    let router: Router = server.build_router().await;

    // Test JSON-RPC endpoint with unknown method
    let jsonrpc_request = json!({
        "jsonrpc": "2.0",
        "method": "unknown/method",
        "params": {},
        "id": 1
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri(DEFAULT_RPC_URL)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&jsonrpc_request).unwrap()))
        .unwrap();

    let response: Response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response_json["jsonrpc"], "2.0");
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32601); // Method not found
}

#[tokio::test]
async fn test_jsonrpc_invalid_json() {
    let agent_card = create_test_agent_card();
    let request_handler = Arc::new(MockRequestHandler::new());
    let context_builder = Arc::new(DefaultServerCallContextBuilder);

    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };

    let server = A2AServerBuilder::new()
        .with_agent_card(agent_card)
        .with_request_handler(request_handler)
        .with_context_builder(context_builder)
        .with_config(config)
        .build()
        .unwrap();

    let router: Router = server.build_router().await;

    // Test JSON-RPC endpoint with invalid JSON
    let request = Request::builder()
        .method(Method::POST)
        .uri(DEFAULT_RPC_URL)
        .header("content-type", "application/json")
        .body(Body::from("{invalid json}"))
        .unwrap();

    let response: Response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response_json["jsonrpc"], "2.0");
    assert!(response_json["error"].is_object());
    assert_eq!(response_json["error"]["code"], -32700); // Parse error
}

#[tokio::test]
async fn test_agent_card_endpoint() {
    let agent_card = create_test_agent_card();
    let request_handler = Arc::new(MockRequestHandler::new());
    let context_builder = Arc::new(DefaultServerCallContextBuilder);

    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };

    let server = A2AServerBuilder::new()
        .with_agent_card(agent_card.clone())
        .with_request_handler(request_handler)
        .with_context_builder(context_builder)
        .with_config(config)
        .build()
        .unwrap();

    let router: Router = server.build_router().await;

    // Test agent card endpoint
    let request = Request::builder()
        .method(Method::GET)
        .uri(AGENT_CARD_WELL_KNOWN_PATH)
        .body(Body::empty())
        .unwrap();

    let response: Response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response_json["name"], agent_card.name);
    assert_eq!(response_json["description"], agent_card.description);
}

#[tokio::test]
async fn test_extended_agent_card_endpoint() {
    let mut agent_card = create_test_agent_card();
    agent_card.supports_authenticated_extended_card = Some(true);

    let extended_card = AgentCard::new(
        "Extended Test Agent".to_string(),
        "An extended test agent".to_string(),
        "http://localhost:8080".to_string(),
        "1.0.0".to_string(),
        vec!["text/plain".to_string()],
        vec!["text/plain".to_string()],
        AgentCapabilities::new(),
        vec![],
    );

    let request_handler = Arc::new(MockRequestHandler::new());
    let context_builder = Arc::new(DefaultServerCallContextBuilder);

    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };

    let server = A2AServerBuilder::new()
        .with_agent_card(agent_card)
        .with_request_handler(request_handler)
        .with_context_builder(context_builder)
        .with_extended_agent_card(extended_card.clone())
        .with_config(config)
        .build()
        .unwrap();

    let router: Router = server.build_router().await;

    // Test extended agent card endpoint
    let request = Request::builder()
        .method(Method::GET)
        .uri(EXTENDED_AGENT_CARD_PATH)
        .body(Body::empty())
        .unwrap();

    let response: Response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response_json["name"], extended_card.name);
    assert_eq!(response_json["description"], extended_card.description);
}

#[tokio::test]
async fn test_jsonrpc_message_send_with_configuration() {
    let agent_card = create_test_agent_card();
    let request_handler = Arc::new(MockRequestHandler::new());
    let context_builder = Arc::new(DefaultServerCallContextBuilder);

    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };

    let server = A2AServerBuilder::new()
        .with_agent_card(agent_card)
        .with_request_handler(request_handler)
        .with_context_builder(context_builder)
        .with_config(config)
        .build()
        .unwrap();

    let router: Router = server.build_router().await;

    // Test JSON-RPC endpoint with message configuration
    let jsonrpc_request = json!({
        "jsonrpc": "2.0",
        "method": "message/send",
        "params": {
            "message": {
                "kind": "message",
                "messageId": "test-msg-456",
                "role": "user",
                "parts": [
                    {
                        "kind": "text",
                        "text": "Test with configuration"
                    }
                ]
            },
            "configuration": {
                "blocking": true,
                "history_length": 10
            }
        },
        "id": 2
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri(DEFAULT_RPC_URL)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&jsonrpc_request).unwrap()))
        .unwrap();

    let response: Response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response_json["jsonrpc"], "2.0");
    assert_eq!(response_json["id"], 2);
    assert!(response_json["result"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_message_send_with_context() {
    let agent_card = create_test_agent_card();
    let request_handler = Arc::new(MockRequestHandler::new());
    let context_builder = Arc::new(DefaultServerCallContextBuilder);

    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };

    let server = A2AServerBuilder::new()
        .with_agent_card(agent_card)
        .with_request_handler(request_handler)
        .with_context_builder(context_builder)
        .with_config(config)
        .build()
        .unwrap();

    let router: Router = server.build_router().await;

    // Test JSON-RPC endpoint with context ID
    let jsonrpc_request = json!({
        "jsonrpc": "2.0",
        "method": "message/send",
        "params": {
            "message": {
                "kind": "message",
                "messageId": "test-msg-789",
                "contextId": "test-context-123",
                "role": "user",
                "parts": [
                    {
                        "kind": "text",
                        "text": "Test with context ID"
                    }
                ]
            }
        },
        "id": 3
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri(DEFAULT_RPC_URL)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&jsonrpc_request).unwrap()))
        .unwrap();

    let response: Response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response_json["jsonrpc"], "2.0");
    assert_eq!(response_json["id"], 3);
    assert!(response_json["result"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_task_get() {
    let agent_card = create_test_agent_card();
    let request_handler = Arc::new(MockRequestHandler::new());
    let context_builder = Arc::new(DefaultServerCallContextBuilder);

    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };

    let server = A2AServerBuilder::new()
        .with_agent_card(agent_card)
        .with_request_handler(request_handler)
        .with_context_builder(context_builder)
        .with_config(config)
        .build()
        .unwrap();

    let router: Router = server.build_router().await;

    // Test JSON-RPC endpoint with tasks/get method
    let jsonrpc_request = json!({
        "jsonrpc": "2.0",
        "method": "tasks/get",
        "params": {
            "id": "test-task-123",
            "history_length": 10
        },
        "id": 4
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri(DEFAULT_RPC_URL)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&jsonrpc_request).unwrap()))
        .unwrap();

    let response: Response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response_json["jsonrpc"], "2.0");
    assert_eq!(response_json["id"], 4);
    // Mock handler should return a valid JSON-RPC response
    // Result could be null, an object, or there could be an error field
    assert!(response_json.get("result").is_some() || response_json.get("error").is_some());
}

#[tokio::test]
async fn test_jsonrpc_task_cancel() {
    let agent_card = create_test_agent_card();
    let request_handler = Arc::new(MockRequestHandler::new());
    let context_builder = Arc::new(DefaultServerCallContextBuilder);

    let config = ServerConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        ..Default::default()
    };

    let server = A2AServerBuilder::new()
        .with_agent_card(agent_card)
        .with_request_handler(request_handler)
        .with_context_builder(context_builder)
        .with_config(config)
        .build()
        .unwrap();

    let router: Router = server.build_router().await;

    // Test JSON-RPC endpoint with tasks/cancel method
    let jsonrpc_request = json!({
        "jsonrpc": "2.0",
        "method": "tasks/cancel",
        "params": {
            "id": "test-task-123"
        },
        "id": 5
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri(DEFAULT_RPC_URL)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&jsonrpc_request).unwrap()))
        .unwrap();

    let response: Response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    assert_eq!(response_json["jsonrpc"], "2.0");
    assert_eq!(response_json["id"], 5);
    // Mock handler should return a valid JSON-RPC response
    // Result could be null, an object, or there could be an error field
    assert!(response_json.get("result").is_some() || response_json.get("error").is_some());
}
