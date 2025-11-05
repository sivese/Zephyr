mod aws;
mod gemini;
mod custom;
mod util;

use axum::{
    extract::Multipart,
    http::{header, StatusCode},
    response::{Json, Response},
    routing::post,
    Router,
};

use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber;
use tower_http::cors::{CorsLayer, Any};
use serde_json::json;
use dotenv::dotenv;

use crate::gemini::client::GeminiClient;

#[tokio::main]
async fn main() {
    dotenv().ok();

    // tracing initialization
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Verify API key exists
    match std::env::var("GEMINI_API_KEY") {
        Ok(_) => info!("GEMINI_API_KEY loaded successfully"),
        Err(_) => panic!("GEMINI_API_KEY not found in environment"),
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)  // TODO: restrict origins for production
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/test", post(test))
        .route("/gen_image", post(generate_image))
        .route("/", post(handler))
        .layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind to address");

    info!("Server running on http://127.0.0.1:8080");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
}

async fn test(mut multipart: Multipart) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    info!("Received multipart request");

    let mut saved_files = Vec::new();

    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read field: {}", e)))?
    {
        let name = field.name().unwrap_or("unknown").to_string();
        let filename = field.file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{}.png", name));

        let data = field.bytes().await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to read bytes: {}", e)))?;

        // Save file to disk
        let filepath = format!("./uploads/{}", filename);
        let mut file = File::create(&filepath).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create file: {}", e)))?;
        file.write_all(&data).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to write file: {}", e)))?;

        info!("Saved {} ({} bytes) to {}", name, data.len(), filepath);
        saved_files.push(filename);
    }

    let response = json!({
        "message": "Images uploaded successfully!",
        "files": saved_files
    });

    Ok(Json(response))
}

// Image generation endpoint using Gemini API
async fn generate_image(mut multipart: Multipart) -> Result<Response, (StatusCode, String)> {
    info!("Received image generation request");

    let mut images = Vec::new();
    // Default prompt for motorcycle customization
    let prompt = String::from(
        "Generate a photorealistic image of the base motorcycle with the custom exhaust system installed. \
        The exhaust should replace the original exhaust, maintaining the same lighting conditions, shadows, and perspective as the base image. \
        Ensure the exhaust pipe diameter, mounting position, and finish match realistic installation standards. \
        The image should look like a professional product photograph."
    );

    // Parse multipart form data
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

    // Call Gemini API
    match gemini_client.gen_image_nanobanana(prompt, images).await {
        Ok(result_image) => {
            info!("Successfully generated image: {} bytes", result_image.len());

            // Return image as PNG
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "image/png")
                .body(axum::body::Body::from(result_image))
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to build response: {}", e)))?)
        }
        Err(e) => {
            let error_msg = format!("Failed to generate image: {}", e);
            error!("{}", error_msg);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_msg))
        }
    }
}


async fn handler() -> Json<serde_json::Value> {
    let response = json!({
        "message": "Hello, World!"
    });

    Json(response)
}