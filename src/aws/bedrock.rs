use aws_config::{meta::region::RegionProviderChain, BehaviorVersion, Region};
use aws_sdk_bedrockruntime::{Client, primitives::Blob};
use serde::{Deserialize, Serialize};
use base64::{Engine as _, engine::general_purpose};
use anyhow::Result;
use std::fs;

// Configuration constants
const DEFAULT_REGION: &str = "us-west-2";
const MODEL_ID: &str = "stability.stable-diffusion-xl-v1";
const DEFAULT_CFG_SCALE: f32 = 7.0;
const INPAINT_CFG_SCALE: f32 = 8.0;
const DEFAULT_STEPS: u32 = 50;
const STYLE_PRESET: &str = "photographic";

// Stable Diffusion XL request structure
#[derive(Serialize, Debug)]
struct StableDiffusionRequest {
    text_prompts: Vec<TextPrompt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    init_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mask_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    mask_image: Option<String>,
    cfg_scale: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_strength: Option<f32>,
    steps: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    style_preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u32>,
}

#[derive(Serialize, Debug)]
struct TextPrompt {
    text: String,
    weight: f32,
}

// Stable Diffusion XL response structure
#[derive(Deserialize, Debug)]
struct StableDiffusionResponse {
    artifacts: Vec<ImageArtifact>,
}

#[derive(Deserialize, Debug)]
struct ImageArtifact {
    base64: String,
    #[serde(rename = "finishReason")]
    finish_reason: String,
}

pub struct BedrockImageGenerator {
    client: Client,
}

impl BedrockImageGenerator {
    // Initialize the Bedrock client
    pub async fn new() -> Result<Self> {
        let region_provider = RegionProviderChain::default_provider()
            .or_else(Region::new(DEFAULT_REGION));

        let config = aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;
        
        let client = Client::new(&config);
        
        Ok(Self { client })
    }

    // Encode image to base64
    fn encode_image(&self, image_path: &str) -> Result<String> {
        let image_data = fs::read(image_path)?;
        Ok(general_purpose::STANDARD.encode(image_data))
    }

    /// Generate image from text (Text-to-Image)
    pub async fn generate_from_text(
        &self,
        prompt: &str,
        negative_prompt: Option<&str>,
    ) -> Result<Vec<u8>> {
        let mut text_prompts = vec![
            TextPrompt {
                text: prompt.to_string(),
                weight: 1.0,
            }
        ];
        
        if let Some(neg_prompt) = negative_prompt {
            text_prompts.push(TextPrompt {
                text: neg_prompt.to_string(),
                weight: -1.0,
            });
        }
        
        let request = StableDiffusionRequest {
            text_prompts,
            init_image: None,
            mask_source: None,
            mask_image: None,
            cfg_scale: DEFAULT_CFG_SCALE,
            image_strength: None,
            steps: DEFAULT_STEPS,
            style_preset: Some(STYLE_PRESET.to_string()),
            seed: None,
        };
        
        self.invoke_model(request).await
    }

    // Generate image from image (Image-to-Image)
    pub async fn generate_from_image(
        &self,
        base_image_path: &str,
        prompt: &str,
        image_strength: f32,
    ) -> Result<Vec<u8>> {
        let base_image = self.encode_image(base_image_path)?;
        
        let request = StableDiffusionRequest {
            text_prompts: vec![
                TextPrompt {
                    text: prompt.to_string(),
                    weight: 1.0,
                }
            ],
            init_image: Some(base_image),
            mask_source: None,
            mask_image: None,
            cfg_scale: DEFAULT_CFG_SCALE,
            image_strength: Some(image_strength),
            steps: DEFAULT_STEPS,
            style_preset: Some(STYLE_PRESET.to_string()),
            seed: None,
        };
        
        self.invoke_model(request).await
    }

    // Inpainting (Modify part of an image)
    pub async fn inpaint(
        &self,
        base_image_path: &str,
        mask_image_path: &str,
        prompt: &str,
        negative_prompt: Option<&str>,
    ) -> Result<Vec<u8>> {
        let base_image = self.encode_image(base_image_path)?;
        let mask_image = self.encode_image(mask_image_path)?;
        
        let mut text_prompts = vec![
            TextPrompt {
                text: prompt.to_string(),
                weight: 1.0,
            }
        ];
        
        if let Some(neg_prompt) = negative_prompt {
            text_prompts.push(TextPrompt {
                text: neg_prompt.to_string(),
                weight: -1.0,
            });
        }
        
        let request = StableDiffusionRequest {
            text_prompts,
            init_image: Some(base_image),
            mask_source: Some("MASK_IMAGE_BLACK".to_string()),
            mask_image: Some(mask_image),
            cfg_scale: INPAINT_CFG_SCALE,
            image_strength: None,
            steps: DEFAULT_STEPS,
            style_preset: Some(STYLE_PRESET.to_string()),
            seed: None,
        };
        
        self.invoke_model(request).await
    }

    // Call Bedrock API
    async fn invoke_model(&self, request: StableDiffusionRequest) -> Result<Vec<u8>> {
        let body_json = serde_json::to_string(&request)?;
        let body_blob = Blob::new(body_json.as_bytes());
        
        let response = self.client
            .invoke_model()
            .model_id(MODEL_ID)
            .content_type("application/json")
            .accept("application/json")
            .body(body_blob)
            .send()
            .await?;
        
        let body_bytes = response.body.as_ref();
        let response_body: StableDiffusionResponse = 
            serde_json::from_slice(body_bytes)?;
        
        if let Some(artifact) = response_body.artifacts.first() {
            let image_bytes = general_purpose::STANDARD.decode(&artifact.base64)?;
            Ok(image_bytes)
        } else {
            anyhow::bail!("No image generated")
        }
    }
}