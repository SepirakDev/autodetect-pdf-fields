use pdfium_render::prelude::*;

use crate::geometry::BBox;

/// A single character with its position on the page (normalized 0-1 coordinates).
#[derive(Debug, Clone)]
pub struct TextNode {
    pub content: char,
    pub bbox: BBox,
}

impl TextNode {
    pub fn endx(&self) -> f32 {
        self.bbox.endx()
    }

    pub fn endy(&self) -> f32 {
        self.bbox.endy()
    }
}

/// Extract character-level text nodes from a PDF page.
///
/// Mirrors the Ruby `text_nodes` method from `lib/pdfium.rb`:
/// - Gets each character's bounding box from pdfium
/// - Transforms coordinates (PDF origin at bottom-left) to top-left normalized coords
/// - Sorts by reading order (endy, x) with y_threshold tolerance
pub fn extract_text_nodes(page: &PdfPage) -> Vec<TextNode> {
    let page_width = page.width().value as f32;
    let page_height = page.height().value as f32;

    if page_width == 0.0 || page_height == 0.0 {
        return Vec::new();
    }

    let text = match page.text() {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let chars = text.chars();

    let mut nodes = Vec::new();

    for char_ref in chars.iter() {
        let content = match char_ref.unicode_char() {
            Some(c) => c,
            None => continue,
        };

        // Get the character's bounding box from pdfium
        let bounds = match char_ref.tight_bounds() {
            Ok(b) => b,
            Err(_) => continue,
        };

        // pdfium-render provides bounds in PDF coordinate space (bottom-left origin)
        let left = bounds.left().value as f32;
        let bottom = bounds.bottom().value as f32;
        let right = bounds.right().value as f32;
        let top = bounds.top().value as f32;

        let abs_width = right - left;
        let abs_height = top - bottom;

        if abs_width <= 0.0 || abs_height <= 0.0 {
            continue;
        }

        // Transform to top-left origin, normalized
        let x = left / page_width;
        let y = (page_height - top) / page_height;
        let w = abs_width / page_width;
        let h = abs_height / page_height;

        nodes.push(TextNode {
            content,
            bbox: BBox::new(x, y, w, h),
        });
    }

    // Sort by reading order with y_threshold tolerance
    let y_threshold = 4.0 / page_width;
    nodes.sort_by(|a, b| {
        if (a.endy() - b.endy()).abs() < y_threshold {
            a.bbox
                .x
                .partial_cmp(&b.bbox.x)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            a.endy()
                .partial_cmp(&b.endy())
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    nodes
}
