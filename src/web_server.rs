use axum::{
    extract::{WebSocketUpgrade, State},
    response::{Json, Response},
    routing::get,
    Router,
};
use axum::extract::ws::{WebSocket, Message};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tower_http::cors::{CorsLayer, AllowOrigin};
use std::env;
use futures_util::{SinkExt, StreamExt};

use crate::OutputData;

pub type SharedData = Arc<RwLock<OutputData>>;
pub type Broadcaster = broadcast::Sender<OutputData>;

pub async fn create_server(shared_data: SharedData) -> (Router, Broadcaster) {
    // Create broadcast channel for WebSocket updates with reasonable buffer
    let (tx, _rx) = broadcast::channel(32);
    let broadcaster = tx.clone();

    // Configure CORS more specifically for security
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::any()) // Consider restricting this in production
        .allow_methods([axum::http::Method::GET])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/", get(root))
        .route("/api/represence", get(get_presence))
        .route("/ws/represence", get(websocket_handler))
        .route("/health", get(health_check))
        .with_state((shared_data, tx))
        .layer(cors);

    (app, broadcaster)
}

async fn root() -> &'static str {
    "Represence API Server - Use /api/represence to get current presence data"
}

async fn get_presence(
    State((shared_data, _)): State<(SharedData, Broadcaster)>
) -> Json<OutputData> {
    let data = shared_data.read().await;
    Json(data.clone())
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State((shared_data, broadcaster)): State<(SharedData, Broadcaster)>
) -> Response {
    ws.on_upgrade(move |socket| websocket_connection(socket, shared_data, broadcaster))
}

async fn websocket_connection(socket: WebSocket, shared_data: SharedData, broadcaster: Broadcaster) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = broadcaster.subscribe();

    // Send current data immediately upon connection
    {
        let current_data = shared_data.read().await;
        if let Ok(json) = serde_json::to_string(&*current_data) {
            if sender.send(Message::Text(json.into())).await.is_err() {
                return;
            }
        }
    }

    // Handle incoming messages and broadcast updates
    let send_task = tokio::spawn(async move {
        while let Ok(data) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&data) {
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        }
    });

    // Simplified receive task - just handle close and ping/pong
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Close(_)) => break,
                Ok(Message::Ping(data)) => {
                    // Respond to ping with pong (handled automatically by axum)
                    let _ = data;
                }
                Ok(_) => {
                    // Ignore other message types
                }
                Err(_) => break,
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

async fn health_check() -> Json<Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().timestamp(),
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": {
            "presence": "/api/represence",
            "websocket": "/ws/represence",
            "health": "/health"
        }
    }))
} 