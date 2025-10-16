use image::{RgbImage, Rgb, ImageBuffer, Luma, GrayImage};
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