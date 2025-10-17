use anyhow::Result;
use std::fs;

use crate::aws::bedrock::BedrockImageGenerator;
use crate::util::image_mask::{MaskGenerator, PartType, MaskIntensity};

/// 모터사이클 커스텀 시각화 파이프라인
pub struct MotorcycleCustomizer {
    generator: BedrockImageGenerator,
}

impl MotorcycleCustomizer {
    pub async fn new() -> Result<Self> {
        let generator = BedrockImageGenerator::new().await?;
        Ok(Self { generator })
    }

    pub async fn visualize_customization(
            &self,
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

        self.generator.inpaint(
            base_motorcycle_path,
            mask_path,
            &prompt,
            Some(negative_prompt),
        ).await
    }

    pub async fn visualize_custom_part(
        &self,
        base_motorcycle_path: &str,
        part_type: PartType,
        bike_description: &str,
        part_description: &str,
        intensity: MaskIntensity,
    ) -> Result<Vec<u8>> {
        println!("🎨 Generating custom visualization...");
        
        // 1. 마스크 생성
        println!("  📍 Creating mask for {:?}...", part_type);
        let gray_mask = MaskGenerator::generate_mask_from_image(
            base_motorcycle_path,
            part_type,
            intensity,
        )?;
        
        let rgb_mask = MaskGenerator::to_rgb_mask(&gray_mask);
        let mask_path = format!("temp_mask_{:?}.png", part_type);
        rgb_mask.save(&mask_path)?;
        
        // 2. 프롬프트 구성
        let part_name = match part_type {
            PartType::Exhaust => "exhaust system",
            PartType::Seat => "seat",
            PartType::Handlebar => "handlebars",
        };
        
        let prompt = format!(
            "{} style motorcycle with custom {} installed, \
            {}, seamlessly integrated aftermarket part, \
            maintaining original frame geometry and proportions, \
            professional product photography, photorealistic, \
            high detail, studio lighting, 8k",
            bike_description, part_name, part_description
        );
        
        let negative_prompt = 
            "different motorcycle model, changed body style, \
            distorted proportions, unrealistic, blurry, low quality, \
            cartoon, 3d render, wrong bike type, illustration";
        
        // 3. Bedrock으로 이미지 생성
        println!("  🚀 Generating image with Bedrock...");
        let result = self.generator.inpaint(
            base_motorcycle_path,
            &mask_path,
            &prompt,
            Some(negative_prompt),
        ).await?;
        
        // 4. 임시 마스크 파일 삭제
        let _ = fs::remove_file(&mask_path);
        
        println!("  ✅ Generation complete!");
        Ok(result)
    }

    // 여러 강도로 생성하여 옵션 제공
    pub async fn generate_options(
        &self,
        base_motorcycle_path: &str,
        part_type: PartType,
        bike_description: &str,
        part_description: &str,
    ) -> Result<Vec<(MaskIntensity, Vec<u8>)>> {
        let intensities = vec![
            MaskIntensity::Minimal,
            MaskIntensity::Medium,
            MaskIntensity::Aggressive,
        ];
        
        let mut results = Vec::new();
        
        for intensity in intensities {
            println!("\n🔄 Generating with {:?} intensity...", intensity);
            
            match self.visualize_custom_part(
                base_motorcycle_path,
                part_type,
                bike_description,
                part_description,
                intensity,
            ).await {
                Ok(image_data) => {
                    results.push((intensity, image_data));
                }
                Err(e) => {
                    eprintln!("⚠️  Failed with {:?} intensity: {}", intensity, e);
                }
            }
        }
        
        Ok(results)
    }
}

#[tokio::test]
async fn main() -> Result<()> {
    println!("🏍️  Motorcycle Custom Visualizer\n");
    
    // 초기화
    let customizer = MotorcycleCustomizer::new().await?;
    
    // 예시 1: 단일 배기 파츠 커스텀
    println!("═══════════════════════════════════════");
    println!("Example 1: Custom Exhaust Visualization");
    println!("═══════════════════════════════════════\n");
    
    let exhaust_result = customizer.visualize_custom_part(
        "base_motorcycle.jpg",
        PartType::Exhaust,
        "sport bike with red and black fairings",
        "polished chrome dual slip-on exhaust with carbon fiber tips, \
        aggressive sound, high-flow design",
        MaskIntensity::Medium,
    ).await?;
    
    fs::write("custom_exhaust.png", &exhaust_result)?;
    println!("💾 Saved: custom_exhaust.png\n");
    
    // 예시 2: 시트 커스텀
    println!("═══════════════════════════════════════");
    println!("Example 2: Custom Seat Visualization");
    println!("═══════════════════════════════════════\n");
    
    let seat_result = customizer.visualize_custom_part(
        "base_motorcycle.jpg",
        PartType::Seat,
        "cruiser style motorcycle",
        "brown vintage leather seat with diamond stitching pattern, \
        comfortable padding, classic styling",
        MaskIntensity::Medium,
    ).await?;
    
    fs::write("custom_seat.png", &seat_result)?;
    println!("💾 Saved: custom_seat.png\n");
    
    // 예시 3: 핸들바 커스텀 (여러 옵션)
    println!("═══════════════════════════════════════");
    println!("Example 3: Handlebar Options (Multiple Intensities)");
    println!("═══════════════════════════════════════\n");
    
    let handlebar_options = customizer.generate_options(
        "base_motorcycle.jpg",
        PartType::Handlebar,
        "naked bike style",
        "black aluminum clip-on handlebars, racing position, \
        anodized finish with integrated bar-end mirrors",
    ).await?;
    
    for (intensity, image_data) in handlebar_options {
        let filename = format!("handlebar_{:?}.png", intensity);
        fs::write(&filename, &image_data)?;
        println!("💾 Saved: {}", filename);
    }
    
    // 예시 4: 마이너 바이크 모델 (모델명 없이)
    println!("\n═══════════════════════════════════════");
    println!("Example 4: Minor Bike Model (No Model Name)");
    println!("═══════════════════════════════════════\n");
    
    let minor_bike_result = customizer.visualize_custom_part(
        "hyosung_gt250r.jpg",
        PartType::Exhaust,
        "lightweight sport bike with inline twin engine, \
        red bodywork with white graphics, \
        17 inch wheels",
        "titanium racing exhaust system with removable baffle, \
        blue heat gradient finish, lightweight construction",
        MaskIntensity::Medium,
    ).await?;
    
    fs::write("minor_bike_custom.png", &minor_bike_result)?;
    println!("💾 Saved: minor_bike_custom.png\n");
    
    println!("✨ All visualizations complete!");
    
    Ok(())
}

// 실제 사용 시 CLI 인터페이스 추가 예시
#[cfg(feature = "cli")]
mod cli {
    use super::*;
    use clap::Parser;
    
    #[derive(Parser)]
    #[command(author, version, about, long_about = None)]
    struct Cli {
        /// Base motorcycle image path
        #[arg(short, long)]
        base: String,
        
        /// Part type (exhaust, seat, handlebar)
        #[arg(short, long)]
        part: String,
        
        /// Bike description
        #[arg(short = 'd', long)]
        bike_desc: String,
        
        /// Part description
        #[arg(short = 'p', long)]
        part_desc: String,
        
        /// Intensity (minimal, medium, aggressive)
        #[arg(short, long, default_value = "medium")]
        intensity: String,
        
        /// Output path
        #[arg(short, long, default_value = "output.png")]
        output: String,
    }
    
    pub async fn run_cli() -> Result<()> {
        let cli = Cli::parse();
        
        let part_type = match cli.part.as_str() {
            "exhaust" => PartType::Exhaust,
            "seat" => PartType::Seat,
            "handlebar" => PartType::Handlebar,
            _ => anyhow::bail!("Invalid part type"),
        };
        
        let intensity = match cli.intensity.as_str() {
            "minimal" => MaskIntensity::Minimal,
            "medium" => MaskIntensity::Medium,
            "aggressive" => MaskIntensity::Aggressive,
            _ => anyhow::bail!("Invalid intensity"),
        };
        
        let customizer = MotorcycleCustomizer::new().await?;
        
        let result = customizer.visualize_custom_part(
            &cli.base,
            part_type,
            &cli.bike_desc,
            &cli.part_desc,
            intensity,
        ).await?;
        
        fs::write(&cli.output, &result)?;
        println!("✅ Saved to: {}", cli.output);
        
        Ok(())
    }
}

// CLI 사용 예시:
// cargo run --features cli -- \
//   --base motorcycle.jpg \
//   --part exhaust \
//   --bike-desc "sport bike with red fairings" \
//   --part-desc "chrome dual exhaust with carbon tips" \
//   --intensity medium \
//   --output custom_result.png