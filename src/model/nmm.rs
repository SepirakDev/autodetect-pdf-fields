use super::postprocessing::{Detection, Detections};
use crate::geometry::BBox;

/// Non-Maximum Merging.
///
/// Sorts detections by descending area. For each kept box, finds overlapping boxes
/// of the same class where `intersection / candidate_area > overlap_threshold`
/// and the kept box has confidence > `min_confidence`. Merges by expanding the
/// kept box to encompass the matched boxes and taking the max confidence.
pub fn nmm(detections: &Detections, overlap_threshold: f32, min_confidence: f32) -> Detections {
    if detections.items.is_empty() {
        return Detections::new();
    }

    // Clone items so we can mutate boxes/scores during merging
    let mut items: Vec<Detection> = detections.items.clone();

    // Sort by descending area
    let mut indices: Vec<usize> = (0..items.len()).collect();
    indices.sort_by(|&a, &b| {
        items[b]
            .bbox
            .area()
            .partial_cmp(&items[a].bbox.area())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut keep = Vec::new();

    while !indices.is_empty() {
        let i = indices[0];
        keep.push(i);

        if indices.len() == 1 {
            break;
        }

        let rest = &indices[1..];

        // Determine which candidates to merge (same class, high overlap)
        let should_merge: Vec<bool> = if items[i].confidence > min_confidence {
            rest.iter()
                .map(|&j| {
                    let overlap = items[i].bbox.overlap_ratio(&items[j].bbox);
                    overlap > overlap_threshold && items[j].class_id == items[i].class_id
                })
                .collect()
        } else {
            vec![false; rest.len()]
        };

        // Merge matched candidates into the kept box
        let merge_indices: Vec<usize> = rest
            .iter()
            .zip(should_merge.iter())
            .filter(|(_, &m)| m)
            .map(|(&j, _)| j)
            .collect();

        if !merge_indices.is_empty() {
            let mut max_conf = items[i].confidence;
            let mut min_x = items[i].bbox.x;
            let mut min_y = items[i].bbox.y;
            let mut max_x = items[i].bbox.endx();
            let mut max_y = items[i].bbox.endy();

            for &j in &merge_indices {
                max_conf = max_conf.max(items[j].confidence);
                min_x = min_x.min(items[j].bbox.x);
                min_y = min_y.min(items[j].bbox.y);
                max_x = max_x.max(items[j].bbox.endx());
                max_y = max_y.max(items[j].bbox.endy());
            }

            items[i].confidence = max_conf;
            items[i].bbox = BBox::from_xyxy(min_x, min_y, max_x, max_y);
        }

        // Keep only non-merged candidates
        indices = rest
            .iter()
            .zip(should_merge.iter())
            .filter(|(_, &m)| !m)
            .map(|(&j, _)| j)
            .collect();
    }

    Detections {
        items: keep.into_iter().map(|i| items[i].clone()).collect(),
    }
}
