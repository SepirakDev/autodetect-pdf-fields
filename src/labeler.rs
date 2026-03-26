use std::collections::HashMap;
use std::env;
use std::io::Cursor;

use base64::Engine;
use image::{DynamicImage, Rgba};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::output::DetectedField;
use crate::pdf::document::PdfDoc;
use crate::pdf::renderer::render_page;

const LABEL_RENDER_SIZE: u32 = 1500;
const BOX_THICKNESS: i32 = 3;
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ResponseContent>,
}

#[derive(Deserialize)]
struct ResponseContent {
    text: Option<String>,
}

#[derive(Deserialize)]
struct FieldLabel {
    field_number: usize,
    name: String,
}

/// Draw a numbered bounding box on an image.
fn draw_numbered_box(img: &mut image::RgbaImage, x: i32, y: i32, w: i32, h: i32, color: Rgba<u8>, thickness: i32) {
    let (img_w, img_h) = (img.width() as i32, img.height() as i32);

    for t in 0..thickness {
        for px in x..=(x + w) {
            let py = y + t;
            if px >= 0 && px < img_w && py >= 0 && py < img_h {
                img.put_pixel(px as u32, py as u32, color);
            }
            let py = y + h - t;
            if px >= 0 && px < img_w && py >= 0 && py < img_h {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
        for py in y..=(y + h) {
            let px_val = x + t;
            if px_val >= 0 && px_val < img_w && py >= 0 && py < img_h {
                img.put_pixel(px_val as u32, py as u32, color);
            }
            let px_val = x + w - t;
            if px_val >= 0 && px_val < img_w && py >= 0 && py < img_h {
                img.put_pixel(px_val as u32, py as u32, color);
            }
        }
    }

    // Draw number tag background
    let tag_w = 20;
    let tag_h = 16;
    let tag_x = x;
    let tag_y = (y - tag_h - 1).max(0);
    for py in tag_y.max(0)..((tag_y + tag_h).min(img_h)) {
        for px in tag_x.max(0)..((tag_x + tag_w).min(img_w)) {
            img.put_pixel(px as u32, py as u32, color);
        }
    }
}

/// Render a page with numbered bounding boxes and encode as base64 PNG.
fn render_annotated_page(
    pdf: &PdfDoc,
    page_idx: usize,
    fields: &[&DetectedField],
) -> Result<String> {
    let pages = pdf.document().pages();
    let page = pages
        .get(page_idx as u16)
        .map_err(|e| Error::PdfRender(format!("page {page_idx}: {e}")))?;

    let rendered = render_page(&page, LABEL_RENDER_SIZE)?;
    let mut img = rendered.to_rgba8();
    let (img_w, img_h) = (img.width() as f32, img.height() as f32);

    let color = Rgba([255, 0, 0, 255]); // red for visibility

    for (i, field) in fields.iter().enumerate() {
        let px_x = (field.bbox.x * img_w) as i32;
        let px_y = (field.bbox.y * img_h) as i32;
        let px_w = (field.bbox.w * img_w) as i32;
        let px_h = (field.bbox.h * img_h) as i32;

        draw_numbered_box(&mut img, px_x, px_y, px_w, px_h, color, BOX_THICKNESS);
        let _ = i; // number is conveyed via the JSON, not drawn as text
    }

    // Encode as PNG -> base64
    let dynamic = DynamicImage::ImageRgba8(img);
    let mut buf = Cursor::new(Vec::new());
    dynamic
        .write_to(&mut buf, image::ImageFormat::Png)
        .map_err(|e| Error::ImageProcess(format!("PNG encode: {e}")))?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
    Ok(b64)
}

/// Build the prompt for Claude with field information.
fn build_prompt(fields: &[&DetectedField]) -> String {
    let mut field_list = String::from("Here are the detected fields with their numbers, types, and heuristic labels:\n\n");

    for (i, field) in fields.iter().enumerate() {
        let heuristic = field.name.as_deref().unwrap_or("(unknown)");
        field_list.push_str(&format!(
            "Field {}: type={:?}, heuristic_label=\"{}\", position=({:.2}, {:.2})\n",
            i + 1,
            field.field_type,
            heuristic,
            field.bbox.x,
            field.bbox.y,
        ));
    }

    format!(
        r#"You are analyzing a document page with detected form fields marked by red bounding boxes.

{field_list}
For each numbered field, determine the correct semantic label — what data should be entered in that field (e.g., "Full Name", "Date of Birth", "Account Number", "Signature", "City", "State", "ZIP Code").

Use the heuristic labels as hints but correct them based on what you see in the document. If a heuristic label is "(unknown)", determine the label from the document context.

Respond with ONLY a JSON array, no other text:
[{{"field_number": 1, "name": "..."}}, {{"field_number": 2, "name": "..."}}, ...]"#
    )
}

/// Call Claude's vision API to label fields on a single page.
fn call_claude_api(
    api_key: &str,
    model: &str,
    image_b64: &str,
    fields: &[&DetectedField],
) -> Result<Vec<FieldLabel>> {
    let prompt = build_prompt(fields);

    let request = ApiRequest {
        model: model.to_string(),
        max_tokens: 4096,
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![
                ContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: "image/png".to_string(),
                        data: image_b64.to_string(),
                    },
                },
                ContentBlock::Text { text: prompt },
            ],
        }],
    };

    let response: ApiResponse = ureq::post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .send_json(&request)
        .map_err(|e| Error::Inference(format!("Claude API request failed: {e}")))?
        .body_mut()
        .read_json()
        .map_err(|e| Error::Inference(format!("Claude API response parse failed: {e}")))?;

    // Extract JSON from the response text
    let text = response
        .content
        .iter()
        .filter_map(|c| c.text.as_deref())
        .collect::<String>();

    // Find JSON array in the response (Claude might wrap it in markdown)
    let json_str = if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            &text[start..=end]
        } else {
            &text
        }
    } else {
        &text
    };

    let labels: Vec<FieldLabel> = serde_json::from_str(json_str)
        .map_err(|e| Error::Inference(format!("Failed to parse Claude labels: {e}\nResponse: {text}")))?;

    Ok(labels)
}

/// Label detected fields using Claude's vision API.
///
/// Groups fields by page, renders each page with numbered bounding boxes,
/// sends to Claude for labeling, and updates the field names.
pub fn label_fields(
    pdf: &PdfDoc,
    fields: &mut [DetectedField],
    model: Option<&str>,
) -> Result<()> {
    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|_| Error::Inference("ANTHROPIC_API_KEY environment variable not set".into()))?;

    let model = model.unwrap_or(DEFAULT_MODEL);

    // Group field indices by page
    let mut pages: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, field) in fields.iter().enumerate() {
        pages.entry(field.page).or_default().push(i);
    }

    for (page_idx, field_indices) in &pages {
        let page_fields: Vec<&DetectedField> = field_indices.iter().map(|&i| &fields[i]).collect();

        eprintln!(
            "Labeling {} fields on page {} via Claude...",
            page_fields.len(),
            page_idx
        );

        // Render annotated page
        let image_b64 = render_annotated_page(pdf, *page_idx, &page_fields)?;

        // Call Claude API
        match call_claude_api(&api_key, model, &image_b64, &page_fields) {
            Ok(labels) => {
                for label in labels {
                    // field_number is 1-indexed, map back to the original index
                    if label.field_number >= 1 && label.field_number <= field_indices.len() {
                        let original_idx = field_indices[label.field_number - 1];
                        fields[original_idx].name = Some(label.name);
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: labeling failed for page {page_idx}: {e}");
                // Graceful degradation — fields keep heuristic labels
            }
        }
    }

    Ok(())
}
