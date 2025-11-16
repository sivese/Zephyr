use base64::{Engine, engine::general_purpose};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;
use reqwest::Client;

#[derive(Debug, Serialize)]
pub struct TaskCreatedResponse {
    pub(crate) task_id: String,
}

#[derive(Debug, Serialize)]
pub struct TaskStatusResponse {
    pub id: String,
    pub status: String,
    pub progress: Option<i32>,
    pub model_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MeshyTaskResponse {
    result: String,
}

#[derive(Debug, Deserialize)]
struct MeshyTaskStatus {
    id: String,
    status: String,
    #[serde(default)]
    model_urls: Option<ModelUrls>,
    #[serde(default)]
    progress: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ModelUrls {
    glb: Option<String>,
    fbx: Option<String>,
    usdz: Option<String>,
}

pub struct MeshyClient {
    api_key: String,
    client: Client,
}

impl MeshyClient {
    const MESHY_API_BASE: &str = "https://api.meshy.ai";
    
    pub fn new() -> Self {
        let api_res = std::env::var("MESHY_API_KEY");
        match api_res {
            Ok(key) => MeshyClient { 
                api_key: key,
                client: Client::new(),
            },
            Err(_) => panic!("MESHY_API_KEY environment variable not set"),
        }
    }
    
    pub async fn create_3d_task(
        &self,
        images: Vec<Bytes>
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let request_url = format!("{}/v2/image-to-3d", Self::MESHY_API_BASE);
        
        let mut image_urls = Vec::new();
        
        for (idx, image_bytes) in images.iter().enumerate() {
            info!("Processing image {}: {} bytes", idx, image_bytes.len());
            
            let mime_type = if image_bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
                "image/jpeg"
            } else if image_bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
                "image/png"
            } else if image_bytes.starts_with(&[0x47, 0x49, 0x46]) {
                "image/gif"
            } else if image_bytes.starts_with(&[0x52, 0x49, 0x46, 0x46]) {
                "image/webp"
            } else {
                "image/jpeg"
            };
            
            let img_base64 = general_purpose::STANDARD.encode(&image_bytes);
            image_urls.push(format!("data:{};base64,{}", mime_type, img_base64));
        }
        
        let payload = json!({
            "image_urls": image_urls,
            "enable_pbr": true,
            "should_remesh": true,
        });
        
        let response = self.client
            .post(&request_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Failed to create task: {}", error_text).into());
        }
        
        let task_response: MeshyTaskResponse = response.json().await?;
        Ok(task_response.result)
    }
    
    pub async fn get_task_status(
        &self,
        task_id: &str
    ) -> Result<TaskStatusResponse, Box<dyn std::error::Error + Send + Sync>> {
        let status_url = format!("{}/v2/image-to-3d/{}", Self::MESHY_API_BASE, task_id);
        
        let response = self.client
            .get(&status_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Failed to check status: {}", error_text).into());
        }
        
        let status: MeshyTaskStatus = response.json().await?;
        
        let model_url = status.model_urls
            .and_then(|urls| urls.glb);
        
        Ok(TaskStatusResponse {
            id: status.id,
            status: status.status,
            progress: status.progress,
            model_url,
        })
    }
}