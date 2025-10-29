// src/aws.rs
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_sts::Client as StsClient;
use aws_sdk_bedrockruntime::Client as BedrockClient;
use aws_sdk_bedrockruntime::primitives::Blob;
use tracing::info;
use serde_json::json;

/*
AWS Legacy code
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

*/
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
