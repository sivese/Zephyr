mod aws;
mod gemini;
mod custom;
mod util;

use axum::{
    extract::{ConnectInfo, Multipart, Request, State},
    http::{header, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use std::{net::SocketAddr, sync::Arc};
use tracing::{info, Level};
use tracing_subscriber;
use tower_http::cors::{CorsLayer, Any};
use serde_json::json;

#[tokio::main]
async fn main() {
    // tracing initialization
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let cors = CorsLayer::new()
        .allow_origin(Any)  // should modify for production
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/test", post(test))
        .route("/", post(handler))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    info!("Server running on http://127.0.0.1:8080");

    axum::serve(listener, app)
        .await
        .unwrap();
}

async fn test(mut multipart: Multipart) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Received multipart request");
    
    let mut saved_files = Vec::new();
    
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("unknown").to_string();
        let filename = field.file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}.png", name));
        
        let data = field.bytes().await.unwrap();
        
        // 파일 저장
        let filepath = format!("./uploads/{}", filename);
        let mut file = File::create(&filepath).await.unwrap();
        file.write_all(&data).await.unwrap();
        
        info!("Saved {} ({} bytes) to {}", name, data.len(), filepath);
        saved_files.push(filename);
    }
    
    let response = json!({
        "message": "Images uploaded successfully!",
        "files": saved_files
    });
    
    Ok(Json(response))
}

async fn handler(mut multipart: Multipart) -> Json<serde_json::Value> {
    let response = json!({
        "message": "Hello, World!"
    });

    Json(response)
}