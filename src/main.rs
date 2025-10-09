use axum::{
    extract::{multipart, Multipart}, http::StatusCode, response::Json, routing::{get, post}, Router
};

use serde_json::json;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", post(handler));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}

async fn handler(mut multipart: Multipart) -> Json<serde_json::Value> {
    let response = json!({
        "message": "Hello, World!"
    });

    Json(response)
}