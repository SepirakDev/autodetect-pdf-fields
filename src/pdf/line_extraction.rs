use pdfium_render::prelude::*;

use crate::geometry::BBox;

/// A line object extracted from the PDF (normalized 0-1 coordinates).
#[derive(Debug, Clone)]
pub struct LineNode {
    pub bbox: BBox,
    pub tilt: u32, // 0 = horizontal, 90 = vertical
}

impl LineNode {
    pub fn endx(&self) -> f32 {
        self.bbox.endx()
    }

    pub fn endy(&self) -> f32 {
        self.bbox.endy()
    }
}

/// Extract line objects from PDF page path objects.
///
/// Mirrors the Ruby `line_nodes` method from `lib/pdfium.rb`:
/// - Iterates page objects, filters for path type
/// - Requires: both dims >= 1pt, 2-10 segments, one dim < 10pt
/// - Classifies horizontal (tilt=0) vs vertical (tilt=90)
/// - Normalizes coordinates to [0, 1]
pub fn extract_line_nodes(page: &PdfPage) -> Vec<LineNode> {
    let page_width = page.width().value as f32;
    let page_height = page.height().value as f32;

    if page_width == 0.0 || page_height == 0.0 {
        return Vec::new();
    }

    let mut nodes = Vec::new();

    for object in page.objects().iter() {
        if object.object_type() != PdfPageObjectType::Path {
            continue;
        }

        let bounds = match object.bounds() {
            Ok(b) => b,
            Err(_) => continue,
        };

        let obj_left = bounds.left().value as f32;
        let obj_bottom = bounds.bottom().value as f32;
        let obj_right = bounds.right().value as f32;
        let obj_top = bounds.top().value as f32;

        let obj_width = obj_right - obj_left;
        let obj_height = obj_top - obj_bottom;

        // Skip tiny objects (both dims < 1pt)
        if obj_width < 1.0 && obj_height < 1.0 {
            continue;
        }

        // Check segment count (2-10) via the path object
        let path_obj = match object.as_path_object() {
            Some(p) => p,
            None => continue,
        };

        let segment_count = path_obj.segments().len();
        if segment_count < 2 || segment_count > 10 {
            continue;
        }

        // Must be thin in one dimension (< 10pt)
        if obj_height >= 10.0 && obj_width >= 10.0 {
            continue;
        }

        let tilt = if obj_width > obj_height && obj_height < 10.0 {
            0 // horizontal
        } else if obj_height > obj_width && obj_width < 10.0 {
            90 // vertical
        } else {
            continue;
        };

        // Normalize coordinates (PDF origin at bottom-left)
        let norm_x = obj_left / page_width;
        let norm_y = (page_height - obj_top) / page_height;
        let norm_w = obj_width / page_width;
        let norm_h = obj_height / page_height;

        nodes.push(LineNode {
            bbox: BBox::new(norm_x, norm_y, norm_w, norm_h),
            tilt,
        });
    }

    // Sort by (endy, x)
    nodes.sort_by(|a, b| {
        if a.endy() == b.endy() {
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
