use base64::{Engine, engine::general_purpose};
use bytes::Bytes;

use serde_json::json;
use tracing::info;

const GEMINI_API_URL: &str = "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image:generateContent";

pub struct GeminiClient {
    api_key: String,
}

impl GeminiClient {
    pub fn new() -> Self {
        let api_res = std::env::var("GEMINI_API_KEY");

        match api_res {
            Ok(key) => GeminiClient { api_key: key },
            Err(_) => panic!("GEMINI_API_KEY environment variable not set"),
        }
    }

    pub async fn gen_image_nanobanana(
        &self,
        prompt: String,
        images: Vec<Bytes>
    ) -> Result<Bytes, Box<dyn std::error::Error>> {
        info!("Starting image generation with {} images", images.len());

        // Encode images to base64 and build request parts
        let mut parts = vec![
            json!({
                "text": prompt
            })
        ];
        
        for (idx, image_bytes) in images.iter().enumerate() {
            info!("Processing image {}: {} bytes", idx, image_bytes.len());

            // Detect image type from magic bytes
            let mime_type = if image_bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
                "image/jpeg"
            } else if image_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                "image/png"
            } else if image_bytes.starts_with(&[0x47, 0x49, 0x46]) {
                "image/gif"
            } else if image_bytes.starts_with(&[0x52, 0x49, 0x46, 0x46]) {
                "image/webp"
            } else {
                info!("Unknown image format, defaulting to image/jpeg");
                "image/jpeg"
            };
            
            info!("Detected MIME type: {}", mime_type);
            
            let img_base64 = general_purpose::STANDARD.encode(&image_bytes);
            parts.push(json!({
                "inline_data": {
                    "mime_type": mime_type,
                    "data": img_base64
                }
            }));
        }

        let body = json!({
            "contents": [{
                "parts": parts
            }]
        });
        
        info!("Sending request to Gemini API...");

        // Call Gemini API
        let client = reqwest::Client::new();
        let response = client
            .post(GEMINI_API_URL)
            .header("x-goog-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        info!("Gemini API response status: {}", status);

        // Get response text first
        let response_text = response.text().await?;

        // Parse text as JSON
        let result: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse JSON."))?;

        // Check for errors in response
        if let Some(error) = result.get("error") {
            let error_message = error.get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error");
            let error_code = error.get("code")
                .and_then(|c| c.as_i64())
                .unwrap_or(0);

            info!("Gemini API error ({}): {}", error_code, error_message);

            return Err(format!("Gemini API error ({}): {}", error_code, error_message).into());
        }

        // Extract generated image from response
        let parts = result["candidates"][0]["content"]["parts"].as_array()
            .ok_or("Failed to get parts array")?;

        for part in parts {
            // Check for inline data in response
            if let Some(data) = part["inlineData"]["data"].as_str() {
                info!("Successfully extracted image data");
                let decoded = general_purpose::STANDARD.decode(data)?;
                info!("Decoded image size: {} bytes", decoded.len());
                return Ok(Bytes::from(decoded));
            }
        }
                
        info!("No image data found in response. Response structure: {}", 
            serde_json::to_string_pretty(&result["candidates"][0]["content"]).unwrap_or_else(|_| "Unable to serialize".to_string())
        );
        Err("Failed to extract image data from response".into())
    }
}