use crate::geometry::BBox;
use crate::output::FieldType;

use super::preprocessing::TransformInfo;

/// A single raw detection from the model (in pixel coordinates of the original image).
#[derive(Debug, Clone)]
pub struct Detection {
    pub bbox: BBox,
    pub confidence: f32,
    pub class_id: i32,
}

impl Detection {
    pub fn field_type(&self) -> FieldType {
        FieldType::from_class_id(self.class_id)
    }
}

/// Collection of detections from a single inference run.
#[derive(Debug, Clone, Default)]
pub struct Detections {
    pub items: Vec<Detection>,
}

impl Detections {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn extend(&mut self, other: Detections) {
        self.items.extend(other.items);
    }
}

/// Postprocess V2 model outputs: filter by confidence, unpad/unscale coordinates.
pub fn postprocess_v2(
    boxes: &ndarray::ArrayView2<f32>,   // [N, 4] xyxy
    labels: &ndarray::ArrayView1<i64>,  // [N]
    scores: &ndarray::ArrayView1<f32>,  // [N]
    transform: &TransformInfo,
    offset_x: f32,
    offset_y: f32,
    confidence: f32,
) -> Detections {
    let n = scores.len();
    let mut detections = Detections::new();

    for i in 0..n {
        let score = scores[i];
        if score <= confidence {
            continue;
        }

        let x1 = (boxes[[i, 0]] - transform.pad_w as f32) / transform.ratio + offset_x;
        let y1 = (boxes[[i, 1]] - transform.pad_h as f32) / transform.ratio + offset_y;
        let x2 = (boxes[[i, 2]] - transform.pad_w as f32) / transform.ratio + offset_x;
        let y2 = (boxes[[i, 3]] - transform.pad_h as f32) / transform.ratio + offset_y;

        detections.items.push(Detection {
            bbox: BBox::from_xyxy(x1, y1, x2, y2),
            confidence: score,
            class_id: labels[i] as i32,
        });
    }

    detections
}

/// Normalize detection coordinates to [0, 1] relative to image dimensions.
pub fn normalize_detections(detections: &mut Detections, image_width: f32, image_height: f32) {
    for det in &mut detections.items {
        let x0 = det.bbox.x / image_width;
        let y0 = det.bbox.y / image_height;
        let x1 = det.bbox.endx() / image_width;
        let y1 = det.bbox.endy() / image_height;

        let x1 = x1.min(1.0);
        let y1 = y1.min(1.0);

        if x0 < 0.0 || x0 > 1.0 || y0 < 0.0 || y0 > 1.0 {
            det.confidence = 0.0; // mark for removal
            continue;
        }

        det.bbox = BBox::new(x0, y0, x1 - x0, y1 - y0);
    }

    detections.items.retain(|d| d.confidence > 0.0);
}
