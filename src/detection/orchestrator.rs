use crate::error::{Error, Result};
use crate::model::inference::{FieldDetector, InferenceOptions};
use crate::model::postprocessing::Detections;
use crate::output::{DetectedField, FieldType};
use crate::pdf::document::PdfDoc;
use crate::pdf::line_extraction::extract_line_nodes;
use crate::pdf::renderer::render_page;
use crate::pdf::text_extraction::{extract_text_nodes, TextNode};

use super::confidence_boost::boost_confidence;
use super::line_filter::filter_line_fields;
use super::type_classifier::classify_field_type;
use super::underscore_fields::detect_underscore_fields;

pub struct DetectOptions {
    pub confidence: f32,
    pub nms_threshold: f32,
    pub nmm_threshold: f32,
    pub classify_types: bool,
    pub padding: Option<u32>,
    pub page: Option<usize>,
}

impl Default for DetectOptions {
    fn default() -> Self {
        Self {
            confidence: 0.3,
            nms_threshold: 0.1,
            nmm_threshold: 0.5,
            classify_types: true,
            padding: Some(20),
            page: None,
        }
    }
}

/// Detect fields in a PDF document.
///
/// For each page:
/// 1. Render to image at resolution * 1.5 (if padding)
/// 2. Run ML inference at confidence / 3.0
/// 3. Extract structural indicators (underscores, lines) from PDF
/// 4. Boost ML confidence for overlapping structural indicators
/// 5. Filter by confidence threshold
/// 6. Classify field types via regex on preceding text
pub fn detect_fields_in_pdf(
    pdf: &PdfDoc,
    detector: &mut FieldDetector,
    options: &DetectOptions,
) -> Result<Vec<DetectedField>> {
    let page_count = pdf.page_count();
    let pages = pdf.document().pages();

    let page_range: Vec<usize> = match options.page {
        Some(p) => {
            if p < page_count {
                vec![p]
            } else {
                return Ok(Vec::new());
            }
        }
        None => (0..page_count).collect(),
    };

    let resolution = detector.resolution();
    let render_size = if options.padding.is_some() {
        (resolution as f32 * 1.5) as u32
    } else {
        resolution
    };

    let mut all_fields = Vec::new();

    for page_idx in page_range {
        let page = pages
            .get(page_idx as u16)
            .map_err(|e| Error::PdfRender(format!("page {page_idx}: {e}")))?;

        let page_height_pts = page.height().value as f32;

        // Render page to image
        let image = render_page(&page, render_size)?;

        // Run ML inference with reduced confidence
        let inference_opts = InferenceOptions {
            confidence: options.confidence / 3.0,
            nms_threshold: options.nms_threshold,
            nmm_threshold: options.nmm_threshold,
        };
        let mut detections = detector.detect(&image, &inference_opts)?;

        // Extract structural indicators from PDF
        let text_nodes = extract_text_nodes(&page);
        let line_nodes = extract_line_nodes(&page);

        let underscore_fields = detect_underscore_fields(&text_nodes);
        let line_fields = filter_line_fields(&line_nodes, &text_nodes, page_height_pts);

        // Sort detections by reading order
        sort_detections(&mut detections);

        // Boost confidence for overlapping structural indicators
        boost_confidence(
            &mut detections,
            &underscore_fields,
            options.confidence,
            1.0,
        );
        boost_confidence(&mut detections, &line_fields, options.confidence, 1.0);

        // Filter by original confidence threshold
        detections
            .items
            .retain(|d| d.confidence >= options.confidence);

        // Build detected fields with type classification
        let page_fields = build_fields(
            &detections,
            &text_nodes,
            page_idx,
            options.classify_types,
        );

        all_fields.extend(page_fields);
    }

    Ok(all_fields)
}

fn sort_detections(detections: &mut Detections) {
    let y_threshold = 0.01;
    detections.items.sort_by(|a, b| {
        if (a.bbox.endy() - b.bbox.endy()).abs() < y_threshold {
            a.bbox
                .x
                .partial_cmp(&b.bbox.x)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            a.bbox
                .endy()
                .partial_cmp(&b.bbox.endy())
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });
}

/// Build DetectedField results with optional regex type classification.
///
/// For type classification, finds the text preceding each field in reading order
/// and checks it against regex patterns.
fn build_fields(
    detections: &Detections,
    text_nodes: &[TextNode],
    page: usize,
    classify_types: bool,
) -> Vec<DetectedField> {
    let y_threshold = 0.01;

    detections
        .items
        .iter()
        .map(|det| {
            let base_type = det.field_type();

            let field_type = if classify_types && base_type == FieldType::Text {
                // Find preceding text by collecting text nodes that come before this field
                let preceding = collect_preceding_text(text_nodes, &det.bbox, y_threshold);
                classify_field_type(&preceding, base_type)
            } else {
                base_type
            };

            // Extract heuristic label from preceding text
            let name = if classify_types {
                let preceding = collect_preceding_text(text_nodes, &det.bbox, y_threshold);
                extract_heuristic_label(&preceding)
            } else {
                None
            };

            DetectedField {
                field_type,
                name,
                field_id: None,
                page,
                confidence: det.confidence,
                bbox: det.bbox,
            }
        })
        .collect()
}

/// Collect text that precedes a field in reading order.
///
/// Gathers characters from text nodes that are either:
/// - On the same line (within y_threshold) and to the left of the field
/// - On the line immediately above the field
fn collect_preceding_text(
    text_nodes: &[TextNode],
    field_bbox: &crate::geometry::BBox,
    y_threshold: f32,
) -> String {
    let mut text = String::new();

    for node in text_nodes.iter().rev() {
        // Stop if we've gone too far above
        if node.endy() < field_bbox.y - y_threshold * 3.0 {
            break;
        }

        // Same line or line above
        let on_same_line = (node.endy() - field_bbox.endy()).abs() < y_threshold;
        let on_line_above =
            node.endy() < field_bbox.y && node.endy() > field_bbox.y - y_threshold * 3.0;

        if on_same_line && node.bbox.x < field_bbox.x {
            text.insert(0, node.content);
        } else if on_line_above {
            text.insert(0, node.content);
        }
    }

    text
}

/// Extract a heuristic label from the preceding text.
///
/// Trims trailing colons, underscores, dashes, whitespace and returns
/// the cleaned text as a label. Returns None if the result is empty or
/// too short to be meaningful.
fn extract_heuristic_label(preceding_text: &str) -> Option<String> {
    let trimmed = preceding_text
        .trim_end_matches(|c: char| c == ':' || c == '_' || c == '-' || c.is_whitespace())
        .trim();

    if trimmed.len() < 2 {
        return None;
    }

    // Take only the last line/phrase (after last newline or tab)
    let label = trimmed
        .rsplit(|c: char| c == '\n' || c == '\t')
        .next()
        .unwrap_or(trimmed)
        .trim();

    if label.len() < 2 {
        return None;
    }

    Some(label.to_string())
}
