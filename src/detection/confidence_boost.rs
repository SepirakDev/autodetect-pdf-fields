use crate::geometry::BBox;
use crate::model::postprocessing::Detections;

/// Boost confidence of ML-detected text fields that overlap with structural indicators.
///
/// For each ML detection where:
/// - confidence < threshold AND type == "text" (class_id 0)
/// - Overlaps with a structural field (underscore or line) with IoU >= 0.4
///
/// Adds `boost` (default 1.0) to the detection's confidence.
pub fn boost_confidence(
    detections: &mut Detections,
    structural_fields: &[BBox],
    confidence_threshold: f32,
    boost: f32,
) {
    if structural_fields.is_empty() {
        return;
    }

    for detection in &mut detections.items {
        if detection.confidence >= confidence_threshold {
            continue;
        }
        if detection.class_id != 0 {
            // Only boost text fields
            continue;
        }

        for field in structural_fields {
            if field.y > detection.bbox.endy() {
                break;
            }
            if field.endy() < detection.bbox.y {
                continue;
            }

            if !detection.bbox.overlaps(field) {
                continue;
            }

            if detection.bbox.iou(field) >= 0.4 {
                detection.confidence += boost;
                break;
            }
        }
    }
}
