use image::DynamicImage;
use ndarray::Array4;

use crate::error::Result;

pub struct TransformInfo {
    pub ratio: f32,
    pub pad_w: u32,
    pub pad_h: u32,
}

/// Preprocess an image for the V2 model.
///
/// 1. Convert to RGB
/// 2. Resize preserving aspect ratio to fit within `resolution x resolution`
/// 3. Center-pad to `resolution x resolution` with white
/// 4. Normalize to [0, 1] by dividing by 255
/// 5. Transpose HWC -> CHW, reshape to [1, 3, resolution, resolution]
pub fn preprocess_image_v2(
    image: &DynamicImage,
    resolution: u32,
) -> Result<(Array4<f32>, TransformInfo)> {
    let rgb = image.to_rgb8();
    let (orig_w, orig_h) = (rgb.width(), rgb.height());

    let ratio_w = resolution as f32 / orig_w as f32;
    let ratio_h = resolution as f32 / orig_h as f32;
    let ratio = ratio_w.min(ratio_h);

    let new_w = (orig_w as f32 * ratio) as u32;
    let new_h = (orig_h as f32 * ratio) as u32;

    let resized = image::imageops::resize(&rgb, new_w, new_h, image::imageops::FilterType::Triangle);

    let pad_w = (resolution - new_w) / 2;
    let pad_h = (resolution - new_h) / 2;

    // Create white canvas and paste resized image
    let mut padded = image::RgbImage::from_pixel(resolution, resolution, image::Rgb([255, 255, 255]));
    image::imageops::overlay(&mut padded, &resized, pad_w as i64, pad_h as i64);

    // Build CHW tensor [1, 3, resolution, resolution]
    let res = resolution as usize;
    let mut tensor = Array4::<f32>::zeros((1, 3, res, res));

    for y in 0..res {
        for x in 0..res {
            let pixel = padded.get_pixel(x as u32, y as u32);
            tensor[[0, 0, y, x]] = pixel[0] as f32 / 255.0;
            tensor[[0, 1, y, x]] = pixel[1] as f32 / 255.0;
            tensor[[0, 2, y, x]] = pixel[2] as f32 / 255.0;
        }
    }

    let transform = TransformInfo { ratio, pad_w, pad_h };

    Ok((tensor, transform))
}
