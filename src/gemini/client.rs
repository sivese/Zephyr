use base64::{Engine, engine::general_purpose};
use bytes::Bytes;

use serde_json::json;
use tracing::info;

pub struct GeminiClient {
    api_key : String,
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
        
        // 이미지들을 base64로 인코딩
        let mut __parts__ = vec![
            json!({
                "text": prompt
            })
        ];
        
        for (idx, image_bytes) in images.iter().enumerate() {
            info!("Processing image {}: {} bytes", idx, image_bytes.len());
            
            // 이미지 타입 감지
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
            __parts__.push(json!({
                "inline_data": {
                    "mime_type": mime_type,
                    "data": img_base64
                }
            }));
        }
        
        let body = json!({
            "contents": [{
                "parts": __parts__
            }]
        });
        
        info!("Sending request to Gemini API...");
        
        // API 호출
        let client = reqwest::Client::new();
        let response = client
            .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image:generateContent")
            .header("x-goog-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        
        let status = response.status();
        info!("Gemini API response status: {}", status);
        
        // 응답 텍스트를 먼저 가져오기
        let response_text = response.text().await?;
        //info!("Gemini API response length: {} bytes", response_text.len());
        
        // 텍스트를 JSON으로 파싱
        let result: serde_json::Value = serde_json::from_str(&response_text)
            .map_err(|e| format!("Failed to parse JSON."))?;
        
        // 에러 체크
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
        
        // 생성된 이미지 추출
        let parts = result["candidates"][0]["content"]["parts"].as_array()
            .ok_or("Failed to get parts array")?;

        for part in parts {
            // inlineData로 변경!
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