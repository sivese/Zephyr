use image::{GenericImageView, GrayImage, ImageBuffer, Luma, Rgb, RgbImage};
use imageproc::drawing::{draw_filled_ellipse_mut, draw_filled_rect_mut};
use imageproc::rect::Rect;
use imageproc::filter::gaussian_blur_f32;
use anyhow::Result;

pub struct MaskGenerator;

#[derive(Debug, Clone, Copy)]
pub enum PartType {
    Exhaust,
    Seat,
    Handlebar,
}

#[derive(Debug, Clone, Copy)]
pub enum MaskIntensity {
    Minimal,
    Medium,
    Aggressive,
}

impl MaskGenerator {
    // Create a mask for the specified motorcycle part
    pub fn create_part_mask(
        image_width: u32,
        image_height: u32,
        part_type: PartType,
        intensity: MaskIntensity,
    ) -> Result<GrayImage> {
        let mut mask = GrayImage::new(image_width, image_height);
        
        let scale = match intensity {
            MaskIntensity::Minimal => 0.8,
            MaskIntensity::Medium => 1.0,
            MaskIntensity::Aggressive => 1.2,
        };
        
        let white = Luma([255u8]);
        
        match part_type {
            PartType::Exhaust => {
                // 배기 파츠 영역 (우측 하단)
                let x = (image_width as f32 * 0.5) as i32;
                let y = (image_height as f32 * 0.65) as i32;
                let width = (image_width as f32 * 0.35 * scale) as i32;
                let height = (image_height as f32 * 0.25 * scale) as i32;
                
                draw_filled_ellipse_mut(
                    &mut mask,
                    (x, y),
                    width,
                    height,
                    white,
                );
            }
            PartType::Seat => {
                // 시트 영역 (중앙 상단)
                let x = (image_width as f32 * 0.5) as i32;
                let y = (image_height as f32 * 0.45) as i32;
                let width = (image_width as f32 * 0.15 * scale) as i32;
                let height = (image_height as f32 * 0.12 * scale) as i32;
                
                draw_filled_ellipse_mut(
                    &mut mask,
                    (x, y),
                    width,
                    height,
                    white,
                );
            }
            PartType::Handlebar => {
                // 핸들바 영역 (전면 상단)
                let x = (image_width as f32 * 0.4) as i32;
                let y = (image_height as f32 * 0.25) as i32;
                let width = (image_width as f32 * 0.2 * scale) as i32;
                let height = (image_height as f32 * 0.12 * scale) as i32;
                
                draw_filled_ellipse_mut(
                    &mut mask,
                    (x, y),
                    width,
                    height,
                    white,
                );
            }
        }

        // Soft border (Gaussian Blur)
        let blurred_mask = gaussian_blur_f32(&mask, 15.0);
        
        Ok(blurred_mask)
    }

    // Create mask from an existing image
    pub fn generate_mask_from_image(
        base_image_path: &str,
        part_type: PartType,
        intensity: MaskIntensity,
    ) -> Result<GrayImage> {
        let img = image::open(base_image_path)?;
        let (width, height) = img.dimensions();
        
        Self::create_part_mask(width, height, part_type, intensity)
    }
    
    // Convert GrayImage mask to RgbImage mask
    pub fn to_rgb_mask(gray_mask: &GrayImage) -> RgbImage {
        let (width, height) = gray_mask.dimensions();
        let mut rgb_mask = RgbImage::new(width, height);
        
        for (x, y, pixel) in gray_mask.enumerate_pixels() {
            let gray_value = pixel[0];
            rgb_mask.put_pixel(x, y, Rgb([gray_value, gray_value, gray_value]));
        }
        
        rgb_mask
    }

    // Gernerate a custom elliptical mask
    pub fn create_custom_mask(
        image_width: u32,
        image_height: u32,
        region_x: f32,      // 0.0 ~ 1.0
        region_y: f32,      // 0.0 ~ 1.0
        region_width: f32,  // 0.0 ~ 1.0
        region_height: f32, // 0.0 ~ 1.0
        feather_radius: f32, // 블러 강도
    ) -> Result<GrayImage> {
        let mut mask = GrayImage::new(image_width, image_height);
        
        let x = (image_width as f32 * region_x) as i32;
        let y = (image_height as f32 * region_y) as i32;
        let w = (image_width as f32 * region_width) as i32;
        let h = (image_height as f32 * region_height) as i32;
        
        let white = Luma([255u8]);
        
        draw_filled_ellipse_mut(
            &mut mask,
            (x, y),
            w,
            h,
            white,
        );
        
        if feather_radius > 0.0 {
            let blurred = gaussian_blur_f32(&mask, feather_radius);
            Ok(blurred)
        } else {
            Ok(mask)
        }
    }
}