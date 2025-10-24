// src/aws.rs
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_sts::Client as StsClient;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_sdk_bedrockruntime::primitives::Blob;
use tracing::info;
use serde_json::json;

pub struct AwsClients {
    pub s3: S3Client,
    pub sts: StsClient,
    pub bedrock: BedrockClient,
}

impl AwsClients {
    pub async fn new() -> Self {
        let region_provider = RegionProviderChain::default_provider()
            .or_else("us-west-2");
        
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .load()
            .await;

        info!("AWS configured with region: {:?}", config.region());

        Self {
            s3: S3Client::new(&config),
            sts: StsClient::new(&config),
            bedrock: BedrockClient::new(&config),
        }
    }

    /// AWS 자격 증명 테스트
    pub async fn test_credentials(&self) -> Result<String, String> {
        match self.sts.get_caller_identity().send().await {
            Ok(response) => {
                let account = response.account().unwrap_or("unknown");
                let arn = response.arn().unwrap_or("unknown");
                let user_id = response.user_id().unwrap_or("unknown");
                
                info!("AWS Credentials Valid - Account: {}, ARN: {}", account, arn);
                
                Ok(format!(
                    "AWS Connection Successful!\nAccount: {}\nUser ID: {}\nARN: {}",
                    account, user_id, arn
                ))
            }
            Err(e) => {
                let error_msg = format!("AWS Credentials Error: {}", e);
                tracing::error!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// S3 버킷 리스트 테스트
    pub async fn test_s3_connection(&self) -> Result<Vec<String>, String> {
        match self.s3.list_buckets().send().await {
            Ok(response) => {
                let buckets: Vec<String> = response
                    .buckets()
                    .iter()
                    .filter_map(|b| b.name().map(|n| n.to_string()))
                    .collect();
                
                info!("S3 Connection Successful - Found {} buckets", buckets.len());
                Ok(buckets)
            }
            Err(e) => {
                let error_msg = format!("S3 Connection Error: {}", e);
                tracing::error!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// 특정 버킷 접근 테스트
    pub async fn test_bucket_access(&self, bucket_name: &str) -> Result<String, String> {
        match self.s3.head_bucket()
            .bucket(bucket_name)
            .send()
            .await
        {
            Ok(_) => {
                info!("Bucket '{}' is accessible", bucket_name);
                Ok(format!("Bucket '{}' is accessible", bucket_name))
            }
            Err(e) => {
                let error_msg = format!("Bucket '{}' access error: {}", bucket_name, e);
                tracing::error!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// Bedrock으로 이미지 생성 (Amazon Titan Image Generator 사용)
    pub async fn generate_image(&self, prompt: &str) -> Result<Vec<u8>, String> {
        info!("Generating image with prompt: {}", prompt);

        // Titan Image Generator v2 요청 페이로드
        let request_body = json!({
            "taskType": "TEXT_IMAGE",
            "textToImageParams": {
                "text": prompt,
            },
            "imageGenerationConfig": {
                "numberOfImages": 1,
                "quality": "standard",
                "height": 1024,
                "width": 1024,
                "cfgScale": 8.0,
                "seed": 0
            }
        });

        let body_string = serde_json::to_string(&request_body)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;

        info!("Sending request to Bedrock...");

        // Bedrock 모델 ID: Amazon Titan Image Generator G1
        let model_id = "amazon.titan-image-generator-v2:0";

        match self.bedrock
            .invoke_model()
            .model_id(model_id)
            .content_type("application/json")
            .body(Blob::new(body_string.as_bytes()))
            .send()
            .await
        {
            Ok(response) => {
                info!("Received response from Bedrock");
                
                let response_body = response.body().as_ref();
                let response_json: serde_json::Value = serde_json::from_slice(response_body)
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                // 이미지 데이터 추출
                let base64_image = response_json["images"][0]
                    .as_str()
                    .ok_or("No image in response")?;

                // Base64 디코딩
                let image_bytes = base64::decode(base64_image)
                    .map_err(|e| format!("Failed to decode base64: {}", e))?;

                info!("Image generated successfully, size: {} bytes", image_bytes.len());
                Ok(image_bytes)
            }
            Err(e) => {
                let error_msg = format!("Bedrock image generation error: {}", e);
                tracing::error!("{}", error_msg);
                Err(error_msg)
            }
        }
    }

    /// Stable Diffusion을 사용한 이미지 생성 (대안)
    pub async fn generate_image_stable_diffusion(&self, prompt: &str) -> Result<Vec<u8>, String> {
        info!("Generating image with Stable Diffusion, prompt: {}", prompt);

        let request_body = json!({
            "text_prompts": [
                {
                    "text": prompt,
                    "weight": 1.0
                }
            ],
            "cfg_scale": 7.0,
            "steps": 50,
            "seed": 0,
            "width": 1024,
            "height": 1024
        });

        let body_string = serde_json::to_string(&request_body)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;

        let model_id = "stability.stable-diffusion-xl-v1";

        match self.bedrock
            .invoke_model()
            .model_id(model_id)
            .content_type("application/json")
            .body(Blob::new(body_string.as_bytes()))
            .send()
            .await
        {
            Ok(response) => {
                let response_body = response.body().as_ref();
                let response_json: serde_json::Value = serde_json::from_slice(response_body)
                    .map_err(|e| format!("Failed to parse response: {}", e))?;

                let base64_image = response_json["artifacts"][0]["base64"]
                    .as_str()
                    .ok_or("No image in response")?;

                let image_bytes = base64::decode(base64_image)
                    .map_err(|e| format!("Failed to decode base64: {}", e))?;

                info!("Image generated successfully, size: {} bytes", image_bytes.len());
                Ok(image_bytes)
            }
            Err(e) => {
                let error_msg = format!("Bedrock image generation error: {}", e);
                tracing::error!("{}", error_msg);
                Err(error_msg)
            }
        }
    }
}