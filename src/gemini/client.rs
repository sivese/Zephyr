use base64::{Engine, engine::general_purpose};
use bytes::Bytes;

use serde_json::json;

pub struct GeminiClient {
    api_key : String,
}

impl GeminiClient {
    pub fn new(api_key: String) -> Self {
        let api_res = std::env::var("GEMINI_API_KEY");

        match api_res {
            Ok(key) => GeminiClient { api_key: key },
            Err(_) => panic!("GEMINI_API_KEY environment variable not set"),
        }
    }

    pub async fn gen_image_nanobanana(
        &self,
        prompt : String,
        images : Vec<Bytes>
    ) -> Result<Bytes, Box<dyn std::error::Error>> {
        // 이미지들을 base64로 인코딩
        let mut parts = vec![
            json!({
                "text": prompt
            })
        ];

        for image_bytes in images {
            let img_base64 = general_purpose::STANDARD.encode(&image_bytes);
            parts.push(json!({
                "inline_data": {
                    "mime_type": "image/jpeg",
                    "data": img_base64
                }
            }));
        }

        let body = json!({
            "contents": [{
                "parts": parts
            }]
        });

        // API 호출
        let client = reqwest::Client::new();
        let response = client
            .post("https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-image:generateContent")
            .header("x-goog-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;
        
        Err("Failed to extract image data from response".into())
    }
}