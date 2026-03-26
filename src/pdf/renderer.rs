use image::DynamicImage;
use pdfium_render::prelude::*;

use crate::error::{Error, Result};

/// Render a PDF page to an image.
///
/// `target_size`: the pixel dimension for rendering.
pub fn render_page(page: &PdfPage, target_size: u32) -> Result<DynamicImage> {
    let config = PdfRenderConfig::new()
        .set_target_width(target_size as i32)
        .set_maximum_width(target_size as i32)
        .set_maximum_height(target_size as i32);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| Error::PdfRender(format!("render: {e}")))?;

    Ok(bitmap.as_image())
}
