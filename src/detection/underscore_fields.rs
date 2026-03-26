use crate::geometry::BBox;
use crate::pdf::text_extraction::TextNode;

/// Detect underscore sequences in text nodes that indicate field locations.
///
/// Scans for consecutive `_` characters where:
/// - Distance between adjacent underscores <= 0.02 (normalized)
/// - Vertical difference < 0.5 * character height
/// - Minimum 2 consecutive underscores
///
/// Returns bounding boxes spanning each underscore sequence.
pub fn detect_underscore_fields(text_nodes: &[TextNode]) -> Vec<BBox> {
    let mut fields = Vec::new();
    let mut i = 0;

    while i < text_nodes.len() {
        let node = &text_nodes[i];

        if node.content != '_' {
            i += 1;
            continue;
        }

        let x1 = node.bbox.x;
        let mut y1 = node.bbox.y;
        let mut x2 = node.bbox.endx();
        let mut y2 = node.bbox.endy();
        let mut count = 1;

        let mut j = i + 1;
        while j < text_nodes.len() {
            let next = &text_nodes[j];
            if next.content != '_' {
                break;
            }

            let distance = next.bbox.x - x2;
            let height_diff = (next.bbox.y - y1).abs();

            if distance > 0.02 || height_diff > node.bbox.h * 0.5 {
                break;
            }

            count += 1;
            x2 = next.bbox.endx();
            y2 = y2.max(next.bbox.endy());
            y1 = y1.min(next.bbox.y);

            j += 1;
        }

        if count >= 2 {
            fields.push(BBox::new(x1, y1, x2 - x1, y2 - y1));
        }

        i = j;
    }

    fields
}
