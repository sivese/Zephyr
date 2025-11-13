use base64::{Engine, engine::general_purpose};
use bytes::Bytes;

use serde_json::json;
use tracing::info;

pub struct MeshyClient {
    api_key : String,
}

impl MeshyClient {
    pub fn new() -> Self {
        let api_res = std::env::var("MESHY_API_KEY");

        match api_res {
            Ok(key) => MeshyClient { api_key: key },
            Err(_) => panic!("MESHY_API_KEY environment variable not set"),
        }
    }

    pub async fn gen_3d(
        &self, images: Vec<Bytes>
    ) -> Result<Bytes, Box<dyn std::error::Error>> {
    
        Err("Failed to generate 3D model from images".into())
    }
}