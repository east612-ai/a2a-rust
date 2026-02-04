//! A2A Auth Client Demo
//! 
//! This example demonstrates how to configure authentication credentials
//! and use the AuthInterceptor to automatically sign requests,
//! matching the usage pattern of the Python client.

use a2a_rust::a2a::client::auth::{AuthInterceptor, CredentialService};
use a2a_rust::a2a::models::{
    SecurityScheme, HTTPAuthSecurityScheme, APIKeySecurityScheme, 
    AgentCard, AgentCapabilities
};
use a2a_rust::a2a::core_types::In;
use a2a_rust::a2a::client::client_trait::{ClientCallContext, ClientCallInterceptor};
use a2a_rust::a2a::error::A2AError;
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde_json::json;

/// A simple mock credential service
struct MyCredentialService {
    credentials: HashMap<String, String>,
}

#[async_trait]
impl CredentialService for MyCredentialService {
    async fn get_credentials(
        &self, 
        scheme_id: &str, 
        _context: Option<&ClientCallContext>
    ) -> Result<Option<String>, A2AError> {
        Ok(self.credentials.get(scheme_id).cloned())
    }
}

#[tokio::main]
async fn main() {
    // 1. Setup Credentials
    let mut creds = HashMap::new();
    creds.insert("bearer_auth".to_string(), "my-secret-jwt-token".to_string());
    creds.insert("api_key_auth".to_string(), "my-api-key-123".to_string());

    let cred_service = Arc::new(MyCredentialService { credentials: creds });

    // 2. Create Auth Interceptor
    let interceptor = AuthInterceptor::new(cred_service);

    // 3. Case A: Server requires Bearer Token
    let mut security_schemes_a = HashMap::new();
    security_schemes_a.insert(
        "bearer_auth".to_string(),
        SecurityScheme::HTTPAuth(HTTPAuthSecurityScheme {
            scheme: "bearer".to_string(),
            description: Some("Main auth".to_string()),
            bearer_format: Some("JWT".to_string()),
        }),
    );

    let card_a = AgentCard::new(
        "Test Agent A".to_string(),
        "Requires Bearer".to_string(),
        "http://localhost:8080".to_string(),
        "1.0.0".to_string(),
        vec![],
        vec![],
        AgentCapabilities::new(),
        vec![],
    )
    .with_security_schemes(security_schemes_a)
    .with_security(vec![HashMap::from([("bearer_auth".to_string(), vec![])])]);

    let payload = json!({"message": "hello"});
    let http_kwargs = HashMap::new();
    
    let (_, new_kwargs_a) = interceptor
        .intercept("message/send", payload.clone(), http_kwargs, &card_a, None)
        .await
        .unwrap();
    
    println!("--- Case A: Bearer Token Required ---");
    if let Some(headers) = new_kwargs_a.get("headers") {
        println!("Authorization: {:?}", headers.get("Authorization"));
    }

    // 4. Case B: Server requires API Key
    let mut security_schemes_b = HashMap::new();
    security_schemes_b.insert(
        "api_key_auth".to_string(),
        SecurityScheme::APIKey(APIKeySecurityScheme {
            name: "X-API-Key".to_string(),
            in_: In::Header,
            description: Some("Backup auth".to_string()),
        }),
    );

    let card_b = AgentCard::new(
        "Test Agent B".to_string(),
        "Requires API Key".to_string(),
        "http://localhost:8080".to_string(),
        "1.0.0".to_string(),
        vec![],
        vec![],
        AgentCapabilities::new(),
        vec![],
    )
    .with_security_schemes(security_schemes_b)
    .with_security(vec![HashMap::from([("api_key_auth".to_string(), vec![])])]);

    let http_kwargs_b = HashMap::new();
    let (_, new_kwargs_b) = interceptor
        .intercept("message/send", payload, http_kwargs_b, &card_b, None)
        .await
        .unwrap();

    println!("\n--- Case B: API Key Required ---");
    if let Some(headers) = new_kwargs_b.get("headers") {
        println!("X-API-Key: {:?}", headers.get("X-API-Key"));
    }
    
    println!("\nUsage pattern matches a2a-python's AuthInterceptor.");
}
