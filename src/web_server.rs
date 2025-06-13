use axum::{
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{CorsLayer, AllowOrigin};
use std::env;

use crate::OutputData;

pub type SharedData = Arc<RwLock<OutputData>>;

pub async fn create_server(shared_data: SharedData) -> Router {
    // Get allowed domains from environment variable (comma-separated)
    let allowed_domains_str = env::var("REPRESENCE_DOMAIN_ALLOWED")
        .unwrap_or_else(|_| "https://localhost:3000".to_string());

    // Parse comma-separated domains and trim whitespace
    let allowed_domains: Vec<String> = allowed_domains_str
        .split(',')
        .map(|domain| domain.trim().to_string())
        .filter(|domain| !domain.is_empty())
        .collect();

    println!("CORS configured for domains: {:?}", allowed_domains);

    // Configure CORS for multiple domains
    let cors = if allowed_domains.len() == 1 {
        // Single domain optimization
        CorsLayer::new()
            .allow_origin(AllowOrigin::exact(allowed_domains[0].parse().unwrap()))
            .allow_methods([axum::http::Method::GET])
            .allow_headers([axum::http::header::CONTENT_TYPE])
    } else {
        // Multiple domains
        let origins: Result<Vec<_>, _> = allowed_domains
            .iter()
            .map(|domain| domain.parse())
            .collect();
        
        match origins {
            Ok(parsed_origins) => {
                CorsLayer::new()
                    .allow_origin(AllowOrigin::list(parsed_origins))
                    .allow_methods([axum::http::Method::GET])
                    .allow_headers([axum::http::header::CONTENT_TYPE])
            }
            Err(_) => {
                eprintln!("Warning: Failed to parse some domains, falling back to first valid domain");
                CorsLayer::new()
                    .allow_origin(AllowOrigin::exact(allowed_domains[0].parse().unwrap()))
                    .allow_methods([axum::http::Method::GET])
                    .allow_headers([axum::http::header::CONTENT_TYPE])
            }
        }
    };

    Router::new()
        .route("/", get(root))
        .route("/api/presence", get(get_presence))
        .route("/health", get(health_check))
        .with_state(shared_data)
        .layer(cors)
}

async fn root() -> &'static str {
    "Represence API Server - Use /api/presence to get current presence data"
}

async fn get_presence(
    axum::extract::State(shared_data): axum::extract::State<SharedData>
) -> Json<OutputData> {
    let data = shared_data.read().await;
    Json(data.clone())
}

async fn health_check() -> Json<Value> {
    let allowed_domains_str = env::var("REPRESENCE_DOMAIN_ALLOWED")
        .unwrap_or_else(|_| "not_set".to_string());
    
    let allowed_domains: Vec<String> = allowed_domains_str
        .split(',')
        .map(|domain| domain.trim().to_string())
        .filter(|domain| !domain.is_empty())
        .collect();
    
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().timestamp(),
        "allowed_domains": allowed_domains
    }))
} 