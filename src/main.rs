mod aws;
mod gemini;
mod custom;
mod util;
mod meshy;

use base64::{Engine, engine::general_purpose};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

use reqwest::Client;
use axum::{
    Router, 
    extract::{ConnectInfo, Multipart, Request, Path, ws::{Message, WebSocket, WebSocketUpgrade}, State}, 
    http::{StatusCode, header}, 
    response::{IntoResponse, Json, Response}, 
    routing::{get, post},
    body::Body
};

use futures::sink::SinkExt;

use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;

use std::{net::SocketAddr, sync::Arc};
use tracing::{info, error, Level};
use tracing_subscriber;
use tower_http::cors::{CorsLayer, Any};
use dotenv::dotenv;

use crate::{gemini::client::GeminiClient, meshy::client::TaskCreatedResponse};
use crate::meshy::client::MeshyClient;

#[derive(Clone)]
pub struct AppState {
    meshy_client: Arc<MeshyClient>,
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    // tracing initialization
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // API 키 확인
    match std::env::var("GEMINI_API_KEY") {
        Ok(_) => info!("GEMINI_API_KEY loaded successfully"),
        Err(_) => panic!("GEMINI_API_KEY not found in environment"),
    }

    match std::env::var("MESHY_API_KEY") {
        Ok(_) => info!("MESHY_API_KEY loaded successfully"),
        Err(_) => panic!("MESHY_API_KEY not found in environment"),
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let meshy_client = Arc::new(MeshyClient::new());

    let app = Router::new()
        .route("/test", post(test))
        .route("/gen_image", post(generate_image))
        // Consider to integrate these three into one with different prompts
        .route("/extract_exhaust", post(extract_exhaust_image))
        .route("/extract_seat", post(extract_seat_image))
        .route("/extract_frame", post(extract_frame_image))
        .route("/", post(handler))
        .merge(create_router(meshy_client))
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

async fn generate_image(mut multipart: Multipart) -> Result<Response, (StatusCode, String)> {
    info!("Received image generation request");
    
    let mut images = Vec::new();
    let prompt = String::from(
        "Generate a photorealistic image of the base motorcycle with the custom exhaust system installed.
        The exhaust should replace the original exhaust, maintaining the same lighting conditions, shadows, and perspective as the base image. 
        Ensure the exhaust pipe diameter, mounting position, and finish match realistic installation standards. 
        The image should look like a professional product photograph."
    );
    
    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read field: {}", e)))? 
    {
        let name = field.name().unwrap_or("unknown").to_string();
        info!("Processing field: {}", name);
        
        if name.starts_with("image") || name == "file" {
            let data = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read bytes: {}", e)))?;
            info!("Received image field '{}': {} bytes", name, data.len());
            images.push(data);
        }
    }
    
    if images.is_empty() {
        info!("No images received");
        return Err((StatusCode::BAD_REQUEST, "No images provided".to_string()));
    }

    let gemini_client = GeminiClient::new();

    match gemini_client.gen_image_nanobanana(prompt, images).await {
        Ok(result_image) => {
            info!("Successfully generated image: {} bytes", result_image.len());
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/png")
                .body(axum::body::Body::from(result_image))
                .unwrap())
        }
        Err(e) => {
            let error_msg = format!("Failed to generate image: {}", e);
            info!("{}", error_msg);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg))
        }
    }
}

async fn extract_exhaust_image(
    mut multipart: Multipart,
) -> Result<Response, (StatusCode, String)> {
    let prompt = String::from("
        Extract only the muffler and exhaust pipe from this motorcycle image. 
        Show the exhaust system as an isolated part on a clean white background. 
        Remove the motorcycle body and all other components.
    ");

    let mut img = Bytes::new();

    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read field: {}", e)))? 
    {
        let name = field.name().unwrap_or("unknown").to_string();

        if name == "image_motorcycle" {
            img = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read bytes: {}", e)))?;

            info!("Extracted frame image: {} bytes", img.len());
        }
    }

    if img.is_empty() {
        info!("No images received");
        return Err((StatusCode::BAD_REQUEST, "No images provided".to_string()));
    }

    let gemini_client = GeminiClient::new();

    match gemini_client.extract_image_nanobanana(prompt, img).await {
        Ok(result_image) => {
            info!("Successfully generated image: {} bytes", result_image.len());
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/png")
                .body(axum::body::Body::from(result_image))
                .unwrap())
        }
        Err(e) => {
            let error_msg = format!("Failed to generate image: {}", e);
            info!("{}", error_msg);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg))
        }
    }
}

async fn extract_seat_image(
    mut multipart: Multipart,
) -> Result<Response, (StatusCode, String)> {
    let prompt = String::from("
        Extract only the seat (saddle) from this motorcycle image.
        Show the seat as an isolated part on a clean white background.
        Remove the motorcycle body and all other components.
    ");

    let mut img = Bytes::new();

    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read field: {}", e)))? 
    {
        let name = field.name().unwrap_or("unknown").to_string();

        if name == "image_motorcycle" {
            img = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read bytes: {}", e)))?;

            info!("Extracted frame image: {} bytes", img.len());
        }
    }

    if img.is_empty() {
        info!("No images received");
        return Err((StatusCode::BAD_REQUEST, "No images provided".to_string()));
    }

    let gemini_client = GeminiClient::new();

    match gemini_client.extract_image_nanobanana(prompt, img).await {
        Ok(result_image) => {
            info!("Successfully generated image: {} bytes", result_image.len());
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/png")
                .body(axum::body::Body::from(result_image))
                .unwrap())
        }
        Err(e) => {
            let error_msg = format!("Failed to generate image: {}", e);
            info!("{}", error_msg);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg))
        }
    }
}

async fn extract_frame_image(
    mut multipart: Multipart,
) -> Result<Response, (StatusCode, String)> {
    let prompt = String::from("
        Remove the exhaust pipe, muffler, and seat from the motorcycle. 
        Show only the bare frame and engine where these parts were located. 
        Keep the rest of the motorcycle intact and unchanged. Clean, realistic result.
    ");

    let mut img = Bytes::new();

    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read field: {}", e)))? 
    {
        let name = field.name().unwrap_or("unknown").to_string();

        if name == "image_motorcycle" {
            img = field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read bytes: {}", e)))?;

            info!("Extracted frame image: {} bytes", img.len());
        }
    }

    if img.is_empty() {
        info!("No images received");
        return Err((StatusCode::BAD_REQUEST, "No images provided".to_string()));
    }

    let gemini_client = GeminiClient::new();

    match gemini_client.extract_image_nanobanana(prompt, img).await {
        Ok(result_image) => {
            info!("Successfully generated image: {} bytes", result_image.len());
            
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/png")
                .body(axum::body::Body::from(result_image))
                .unwrap())
        }
        Err(e) => {
            let error_msg = format!("Failed to generate image: {}", e);
            info!("{}", error_msg);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg))
        }
    }
}

pub async fn create_3d_handler(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<TaskCreatedResponse>, StatusCode> {
    info!("Received 3D creation request");
    
    let mut images: Vec<Bytes> = Vec::new();
    
    // multipart에서 이미지 추출
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or("unknown").to_string();
        info!("Processing field: {}", name);
        
        if name.starts_with("image") || name == "file" {
            let data = field.bytes().await
                .map_err(|_| StatusCode::BAD_REQUEST)?;
            info!("Received image field '{}': {} bytes", name, data.len());
            images.push(data);
        }
    }
    
    if images.is_empty() {
        info!("No images received");
        return Err(StatusCode::BAD_REQUEST);
    }
    
    match state.meshy_client.create_3d_task(images).await {
        Ok(task_id) => Ok(Json(TaskCreatedResponse { task_id })),
        Err(e) => {
            error!("Failed to create 3D task: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn handler(mut multipart: Multipart) -> Json<serde_json::Value> {
    let response = json!({
        "message": "Hello, World!"
    });

    Json(response)
}

// WebSocket 핸들러
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(task_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, task_id, state))
}

async fn handle_socket(
    mut socket: WebSocket, 
    task_id: String, 
    state: AppState,
) {
    info!("WebSocket connected - task: {}", task_id);
    
    loop {
        match state.meshy_client.get_task_status(&task_id).await {
            Ok(status) => {
                let status_json = match serde_json::to_string(&status) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize status: {}", e);
                        break;
                    }
                };
                
                info!("Sending status update: {} - progress: {}", 
                    status.status, 
                    status.progress.unwrap_or(0)
                );
                
                if socket.send(Message::Text(status_json.into())).await.is_err() {
                    info!("Client disconnected");
                    break;
                }
                
                // Check if task completed
                if status.status == "SUCCEEDED" || status.status == "FAILED" {
                    info!("Task {} finished with status: {}", task_id, status.status);
                    let _ = socket.close().await;
                    break;
                }
                
                // Poll every 5 seconds
                sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                error!("Failed to get task status: {}", e);
                let error_msg = json!({
                    "error": "Failed to get status",
                    "details": e.to_string()
                }).to_string();
                
                if socket.send(Message::Text(error_msg.into())).await.is_err() {
                    break;
                }
                break;
            }
        }
    }
    
    info!("WebSocket closed for task: {}", task_id);
}

// Router configuration with proper state management
pub fn create_router(meshy_client: Arc<MeshyClient>) -> Router {
    let state = AppState {
        meshy_client,
    };
    
    Router::new()
        .route("/api/3d/create", post(create_3d_handler))
        .route("/api/3d/ws/{task_id}", get(ws_handler))
        .route("/api/3d/model/{task_id}", get(proxy_model_handler))  // 새 라우트
        .with_state(state)
}

pub async fn proxy_model_handler(
    Path(task_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    info!("Proxying 3D model for task: {}", task_id);
    
    match state.meshy_client.get_task_status(&task_id).await {
        Ok(status) => {
            if let Some(model_url) = status.model_url {
                info!("Fetching model from: {}", model_url);
                
                let client = Client::new();
                match client.get(&model_url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.bytes().await {
                                Ok(bytes) => {
                                    info!("Successfully fetched model: {} bytes", bytes.len());
                                    
                                    Ok(Response::builder()
                                        .status(StatusCode::OK)
                                        .header(header::CONTENT_TYPE, "application/octet-stream")
                                        .header(
                                            header::CONTENT_DISPOSITION,
                                            format!("attachment; filename=\"motorcycle-3d-{}.glb\"", task_id)
                                        )
                                        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                                        .body(Body::from(bytes))
                                        .unwrap())
                                }
                                Err(e) => {
                                    error!("Failed to read model bytes: {}", e);
                                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                                }
                            }
                        } else {
                            error!("Failed to fetch model: {}", response.status());
                            Err(StatusCode::BAD_GATEWAY)
                        }
                    }
                    Err(e) => {
                        error!("Failed to download model: {}", e);
                        Err(StatusCode::BAD_GATEWAY)
                    }
                }
            } else {
                error!("No model URL available for task: {}", task_id);
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            error!("Failed to get task status: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}