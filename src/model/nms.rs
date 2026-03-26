use super::postprocessing::Detections;

/// Non-Maximum Suppression.
///
/// Sorts detections by descending confidence.
/// Greedily keeps the highest-confidence box, suppressing all remaining boxes
/// whose IoU with the kept box exceeds `iou_threshold`.
pub fn nms(detections: &Detections, iou_threshold: f32) -> Detections {
    if detections.items.is_empty() {
        return Detections::new();
    }

    // Sort by descending confidence
    let mut indices: Vec<usize> = (0..detections.items.len()).collect();
    indices.sort_by(|&a, &b| {
        detections.items[b]
            .confidence
            .partial_cmp(&detections.items[a].confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut keep = Vec::new();

    while !indices.is_empty() {
        let i = indices[0];
        keep.push(i);

        if indices.len() == 1 {
            break;
        }

        let current = &detections.items[i].bbox;

        indices = indices[1..]
            .iter()
            .filter(|&&j| current.iou(&detections.items[j].bbox) <= iou_threshold)
            .copied()
            .collect();
    }

    Detections {
        items: keep.into_iter().map(|i| detections.items[i].clone()).collect(),
    }
}
