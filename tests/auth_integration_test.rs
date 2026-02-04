//! Authentication Integration Tests
//! 
//! This module contains integration tests for the authentication system,
//! demonstrating how to configure and use credentials with interceptors.

use a2a_rust::a2a::{
    models::*,
    core_types::*,
    client::auth::{AuthInterceptor, InMemoryContextCredentialStore},
    client::client_trait::ClientCallInterceptor,
};
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::json;

#[tokio::test]
async fn test_auth_flow_with_multiple_schemes() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Agent Card with multiple security schemes
    let mut security_schemes = HashMap::new();
    
    // Scheme A: Bearer Token
    security_schemes.insert(
        "bearerAuth".to_string(),
        SecurityScheme::HTTPAuth(HTTPAuthSecurityScheme {
            scheme: "bearer".to_string(),
            bearer_format: Some("JWT".to_string()),
            description: Some("Main authentication".to_string()),
        }),
    );
    
    // Scheme B: API Key in Header
    security_schemes.insert(
        "apiKeyAuth".to_string(),
        SecurityScheme::APIKey(APIKeySecurityScheme {
            name: "X-Custom-Key".to_string(),
            in_: In::Header,
            description: Some("Secondary authentication".to_string()),
        }),
    );

    let agent_card = AgentCard::new(
        "Secure Agent".to_string(),
        "An agent requiring authentication".to_string(),
        "http://localhost:8080".to_string(),
        "1.0.0".to_string(),
        vec![],
        vec![],
        AgentCapabilities::new(),
        vec![],
    )
    .with_security_schemes(security_schemes)
    .with_security(vec![
        // Requirement 1: bearerAuth
        HashMap::from([("bearerAuth".to_string(), vec![])]),
    ]);

    // 2. Setup Credential Store
    let mut store = InMemoryContextCredentialStore::new();
    store.add_credential("bearerAuth", "my-secret-jwt-token");
    store.add_credential("apiKeyAuth", "my-api-key");

    // 3. Create Interceptor
    let interceptor = AuthInterceptor::new(Arc::new(store));

    // 4. Simulate a request
    let method = "message/send";
    let payload = json!({"message": "hello"});
    let http_kwargs = HashMap::new();

    // 5. Run interception
    let (_new_payload, updated_kwargs) = interceptor
        .intercept(method, payload, http_kwargs, &agent_card, None)
        .await?;

    // 6. Verify results
    let headers = updated_kwargs.get("headers").expect("Headers should exist");
    let auth_header = headers.get("Authorization").expect("Authorization header should exist");
    assert_eq!(auth_header, "Bearer my-secret-jwt-token");

    // 7. Test fallback/alternative requirement
    let mut agent_card_alt = agent_card.clone();
    agent_card_alt.security = Some(vec![
        // Requirement: apiKeyAuth
        HashMap::from([("apiKeyAuth".to_string(), vec![])]),
    ]);

    let (_new_payload, updated_kwargs_alt) = interceptor
        .intercept(method, json!({}), HashMap::new(), &agent_card_alt, None)
        .await?;

    let headers_alt = updated_kwargs_alt.get("headers").expect("Headers should exist");
    let api_key_header = headers_alt.get("X-Custom-Key").expect("API Key header should exist");
    assert_eq!(api_key_header, "my-api-key");

    Ok(())
}
