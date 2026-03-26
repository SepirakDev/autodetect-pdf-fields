use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use image::{DynamicImage, Rgba, RgbaImage};
use printpdf::{
    ColorBits, ColorSpace, Image, ImageTransform, ImageXObject, Mm, PdfDocument, Px,
};

use crate::error::{Error, Result};
use crate::output::{DetectedField, FieldType};
use crate::pdf::document::PdfDoc;
use crate::pdf::renderer::render_page;

const DEBUG_RENDER_SIZE: u32 = 2000;
const BOX_THICKNESS: i32 = 3;

fn color_for_type(field_type: FieldType) -> Rgba<u8> {
    match field_type {
        FieldType::Text => Rgba([0, 120, 255, 255]),
        FieldType::Checkbox => Rgba([0, 200, 0, 255]),
        FieldType::Date => Rgba([200, 100, 0, 255]),
        FieldType::Signature => Rgba([200, 0, 200, 255]),
        FieldType::Number => Rgba([0, 180, 180, 255]),
    }
}

/// Draw a rectangle border on an RGBA image.
fn draw_rect(img: &mut RgbaImage, x: i32, y: i32, w: i32, h: i32, color: Rgba<u8>, thickness: i32) {
    let (img_w, img_h) = (img.width() as i32, img.height() as i32);

    for t in 0..thickness {
        // Top edge
        for px in x..=(x + w) {
            let py = y + t;
            if px >= 0 && px < img_w && py >= 0 && py < img_h {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
        // Bottom edge
        for px in x..=(x + w) {
            let py = y + h - t;
            if px >= 0 && px < img_w && py >= 0 && py < img_h {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
        // Left edge
        for py in y..=(y + h) {
            let px = x + t;
            if px >= 0 && px < img_w && py >= 0 && py < img_h {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
        // Right edge
        for py in y..=(y + h) {
            let px = x + w - t;
            if px >= 0 && px < img_w && py >= 0 && py < img_h {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
    }
}

/// Draw a filled label tag above the bounding box.
fn draw_label_tag(img: &mut RgbaImage, x: i32, y: i32, field: &DetectedField, color: Rgba<u8>) {
    let label = match field.field_type {
        FieldType::Text => "text",
        FieldType::Checkbox => "checkbox",
        FieldType::Date => "date",
        FieldType::Signature => "signature",
        FieldType::Number => "number",
    };
    let tag_w = (label.len() as i32 * 7 + 50).max(60);
    let tag_h = 18;
    let tag_y = (y - tag_h - 1).max(0);
    let (img_w, img_h) = (img.width() as i32, img.height() as i32);

    for py in tag_y.max(0)..((tag_y + tag_h).min(img_h)) {
        for px in x.max(0)..((x + tag_w).min(img_w)) {
            img.put_pixel(px as u32, py as u32, color);
        }
    }
}

/// Render a debug PDF with bounding boxes drawn around each detected field.
pub fn write_debug_pdf(
    pdf: &PdfDoc,
    fields: &[DetectedField],
    output_path: &Path,
) -> Result<()> {
    let page_count = pdf.page_count();
    let pages = pdf.document().pages();

    let first_page = pages
        .get(0)
        .map_err(|e| Error::PdfRender(format!("page 0: {e}")))?;
    let initial_w = Mm(first_page.width().value * 0.3528);
    let initial_h = Mm(first_page.height().value * 0.3528);

    let (doc, mut current_page, mut current_layer) =
        PdfDocument::new("Debug Output", initial_w, initial_h, "Page 1");

    for page_idx in 0..page_count {
        let page = pages
            .get(page_idx as u16)
            .map_err(|e| Error::PdfRender(format!("page {page_idx}: {e}")))?;

        // Render at high resolution for readability
        let rendered = render_page(&page, DEBUG_RENDER_SIZE)?;
        let mut img = rendered.to_rgba8();
        let (img_w, img_h) = (img.width() as f32, img.height() as f32);

        // Draw bounding boxes for fields on this page
        let page_fields: Vec<&DetectedField> = fields.iter().filter(|f| f.page == page_idx).collect();

        for field in &page_fields {
            let color = color_for_type(field.field_type);

            let px_x = (field.bbox.x * img_w) as i32;
            let px_y = (field.bbox.y * img_h) as i32;
            let px_w = (field.bbox.w * img_w) as i32;
            let px_h = (field.bbox.h * img_h) as i32;

            draw_rect(&mut img, px_x, px_y, px_w, px_h, color, BOX_THICKNESS);
            draw_label_tag(&mut img, px_x, px_y, field, color);
        }

        // Convert to RGB for PDF embedding
        let rgb_img = DynamicImage::ImageRgba8(img).to_rgb8();
        let (w, h) = (rgb_img.width(), rgb_img.height());

        let page_w_mm = page.width().value * 0.3528;
        let page_h_mm = page.height().value * 0.3528;

        let (target_page, target_layer) = if page_idx == 0 {
            (current_page, current_layer)
        } else {
            doc.add_page(
                Mm(page_w_mm),
                Mm(page_h_mm),
                format!("Page {}", page_idx + 1),
            )
        };

        let image_xobj = ImageXObject {
            width: Px(w as usize),
            height: Px(h as usize),
            color_space: ColorSpace::Rgb,
            bits_per_component: ColorBits::Bit8,
            interpolate: true,
            image_data: rgb_img.into_raw(),
            image_filter: None,
            clipping_bbox: None,
            smask: None,
        };

        let image_ref = Image::from(image_xobj);
        let layer = doc.get_page(target_page).get_layer(target_layer);

        let dpi = w as f32 / (page_w_mm / 25.4);

        image_ref.add_to_layer(
            layer,
            ImageTransform {
                translate_x: Some(Mm(0.0)),
                translate_y: Some(Mm(0.0)),
                dpi: Some(dpi),
                ..Default::default()
            },
        );

        current_page = target_page;
        current_layer = target_layer;
    }

    let file = File::create(output_path).map_err(Error::Io)?;
    let mut writer = BufWriter::new(file);
    doc.save(&mut writer)
        .map_err(|e| Error::ImageProcess(format!("Failed to write debug PDF: {e}")))?;

    Ok(())
}
