use crate::geometry::BBox;
use crate::pdf::line_extraction::LineNode;
use crate::pdf::text_extraction::TextNode;

/// Filter horizontal lines from PDF to find likely field indicators.
///
/// Mirrors `extract_line_fields_from_page` from `detect_fields.rb`:
/// 1. Separate vertical and horizontal lines
/// 2. Reject full-width lines (likely page borders)
/// 3. Reject lines that intersect vertical lines (table borders)
/// 4. Reject lines mostly covered by text (underlined text)
/// 5. Expand remaining lines vertically to overlap with ML detections
pub fn filter_line_fields(
    line_nodes: &[LineNode],
    text_nodes: &[TextNode],
    page_height: f32,
) -> Vec<BBox> {
    let line_thickness = 5.0 / page_height;

    let (vertical_lines, horizontal_lines): (Vec<&LineNode>, Vec<&LineNode>) =
        line_nodes.iter().partition(|l| l.tilt == 90);

    let mut filtered: Vec<BBox> = Vec::new();

    'outer: for h_line in &horizontal_lines {
        // Reject full-width lines
        if h_line.bbox.w > 0.7 && (h_line.bbox.h < 0.1 || h_line.bbox.h < 0.9) {
            continue;
        }

        // Reject if intersects with vertical lines (table border detection)
        if !vertical_lines.is_empty() {
            let h_x_min = h_line.bbox.x;
            let h_x_max = h_line.bbox.endx();
            let h_y_avg = h_line.bbox.y + h_line.bbox.h / 2.0;

            for v_line in &vertical_lines {
                let v_x_avg = v_line.bbox.x + v_line.bbox.w / 2.0;
                let v_y_min = v_line.bbox.y;
                let v_y_max = v_line.bbox.endy();

                let x_overlap = (v_x_avg - line_thickness) <= (h_x_max + line_thickness)
                    && (v_x_avg + line_thickness) >= (h_x_min - line_thickness);
                let y_overlap = (h_y_avg - line_thickness) <= (v_y_max + line_thickness)
                    && (h_y_avg + line_thickness) >= (v_y_min - line_thickness);

                if x_overlap && y_overlap {
                    continue 'outer;
                }
            }
        }

        // Reject if mostly covered by text
        let mut text_width_sum = 0.0f32;
        for text in text_nodes {
            // Check if text node overlaps with the line
            if text.bbox.endx() < h_line.bbox.x || h_line.bbox.endx() < text.bbox.x {
                continue;
            }
            if text.bbox.endy() < h_line.bbox.y - text.bbox.h
                || h_line.bbox.y < text.bbox.y
            {
                continue;
            }
            text_width_sum += text.bbox.w;
        }

        if text_width_sum > h_line.bbox.w / 2.0 {
            continue;
        }

        // Expand line vertically to overlap with ML detections
        let expanded = BBox::new(
            h_line.bbox.x,
            h_line.bbox.y - 4.0 * line_thickness,
            h_line.bbox.w,
            h_line.bbox.h + 4.0 * line_thickness,
        );

        filtered.push(expanded);
    }

    filtered
}
