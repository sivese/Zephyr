use crate::aws::bedrock::{self, BedrockImageGenerator};

pub async fn visualize_customization(
        bedrock: &BedrockImageGenerator,
        base_motorcycle_path: &str,
        mask_path: &str,
        bike_style: &str,
        part_type: &str,
        part_description: &str,
    ) -> Result<Vec<u8>> {
    let prompt = format!(
        "{} style motorcycle with custom {} installed, \
        {}, seamlessly integrated aftermarket part, \
        professional product photography, high detail, photorealistic, \
        maintaining original frame geometry and proportions",
        bike_style, part_type, part_description
    );
    
    let negative_prompt = 
        "different motorcycle model, changed body style, \
        distorted proportions, unrealistic integration, \
        blurry, low quality, cartoon, 3d render";
    
    bedrock.inpaint(
        base_motorcycle_path,
        mask_path,
        &prompt,
        Some(negative_prompt),
    ).await
}