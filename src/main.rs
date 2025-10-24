mod aws;
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
use crate::aws::client::AwsClients;

#[derive(Clone)]
struct AppState {
    aws_clients: Arc<AwsClients>,
}

#[tokio::main]
async fn main() {
    // tracing initialization
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Initializing AWS clients...");
    let aws_clients = Arc::new(AwsClients::new().await);
    info!("AWS clients initialized");

    let state = AppState { aws_clients };

    let cors = CorsLayer::new()
        .allow_origin(Any)  // should modify for production
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/test", post(test))
        .route("/aws/credentials", get(test_aws_credentials))
        .route("/aws/s3", get(test_aws_s3))
        .route("/aws/generate-image", get(generate_sunset_motorcycle_image))
        .route("/aws/generate-image-base64", get(generate_image_base64))
        .route("/", post(handler))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    info!("Server running on http://127.0.0.1:8080");

    axum::serve(listener, app)
        .await
        .unwrap();
}

// AWS 자격 증명 테스트 엔드포인트
async fn test_aws_credentials(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Testing AWS credentials...");
    
    match state.aws_clients.test_credentials().await {
        Ok(message) => {
            Ok(Json(json!({
                "status": "success",
                "message": message,
                "region": "us-west-2"
            })))
        }
        Err(error) => {
            Ok(Json(json!({
                "status": "error",
                "message": error
            })))
        }
    }
}

// S3 연결 테스트 엔드포인트
async fn test_aws_s3(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Testing S3 connection...");
    
    match state.aws_clients.test_s3_connection().await {
        Ok(buckets) => {
            Ok(Json(json!({
                "status": "success",
                "message": "S3 connection successful",
                "buckets": buckets,
                "bucket_count": buckets.len(),
                "region": "us-west-2"
            })))
        }
        Err(error) => {
            Ok(Json(json!({
                "status": "error",
                "message": error
            })))
        }
    }
}

// 노을진 바닷가 오토바이 이미지 생성 (바이너리 반환)
async fn generate_sunset_motorcycle_image(
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    info!("Generating sunset motorcycle image...");
    
    let prompt = "A stunning cinematic scene of a person sitting on a sleek motorcycle, \
                  gazing at a beautiful sunset over the ocean. The sky is painted with \
                  vibrant orange, pink, and purple hues. The ocean waves gently lap at \
                  the shore. The motorcycle is a modern sport bike with chrome details. \
                  Photorealistic, highly detailed, golden hour lighting, peaceful atmosphere, \
                  8k quality";

    match state.aws_clients.generate_image(prompt).await {
        Ok(image_bytes) => {
            info!("Successfully generated image, returning {} bytes", image_bytes.len());
            
            Ok((
                [(header::CONTENT_TYPE, "image/png")],
                image_bytes,
            ).into_response())
        }
        Err(error) => {
            tracing::error!("Failed to generate image: {}", error);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// Base64 형식으로 이미지 반환 (JSON)
async fn generate_image_base64(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    info!("Generating sunset motorcycle image (base64)...");
    
    let prompt = "A stunning cinematic scene of a person sitting on a sleek motorcycle, \
                  gazing at a beautiful sunset over the ocean. The sky is painted with \
                  vibrant orange, pink, and purple hues. The ocean waves gently lap at \
                  the shore. The motorcycle is a modern sport bike with chrome details. \
                  Photorealistic, highly detailed, golden hour lighting, peaceful atmosphere, \
                  8k quality";

    match state.aws_clients.generate_image(prompt).await {
        Ok(image_bytes) => {
            let base64_image = base64::encode(&image_bytes);
            
            Ok(Json(json!({
                "status": "success",
                "message": "Image generated successfully",
                "image": format!("data:image/png;base64,{}", base64_image),
                "size_bytes": image_bytes.len()
            })))
        }
        Err(error) => {
            Ok(Json(json!({
                "status": "error",
                "message": error
            })))
        }
    }
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