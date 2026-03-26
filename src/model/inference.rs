use std::path::Path;

use image::DynamicImage;
use ndarray::{Array2, ArrayView1, ArrayView2};
use ort::session::Session;
use ort::value::{Value, ValueType};

use crate::error::{Error, Result};

use super::nmm::nmm;
use super::nms::nms;
use super::postprocessing::{normalize_detections, postprocess_v2, Detections};
use super::preprocessing::preprocess_image_v2;

pub struct InferenceOptions {
    pub confidence: f32,
    pub nms_threshold: f32,
    pub nmm_threshold: f32,
}

pub struct FieldDetector {
    session: Session,
    resolution: u32,
}

impl FieldDetector {
    pub fn load(model_path: &Path) -> Result<Self> {
        let session = Session::builder()
            .map_err(|e| Error::ModelLoad(e.to_string()))?
            .with_intra_threads(num_cpus::get())
            .map_err(|e| Error::ModelLoad(e.to_string()))?
            .with_inter_threads(1)
            .map_err(|e| Error::ModelLoad(e.to_string()))?
            .commit_from_file(model_path)
            .map_err(|e| Error::ModelLoad(format!("{model_path:?}: {e}")))?;

        // Read resolution from model input shape (images input: [1, 3, res, res])
        let resolution = session
            .inputs()
            .iter()
            .find(|i| i.name() == "images")
            .and_then(|i| {
                if let ValueType::Tensor { shape, .. } = i.dtype() {
                    shape.get(2).map(|&d| d as u32)
                } else {
                    None
                }
            })
            .ok_or_else(|| Error::ModelLoad("Could not determine model resolution from input shape".into()))?;

        tracing::info!("Loaded model from {model_path:?}, resolution={resolution}");

        Ok(Self {
            session,
            resolution,
        })
    }

    pub fn resolution(&self) -> u32 {
        self.resolution
    }

    pub fn detect(
        &mut self,
        image: &DynamicImage,
        options: &InferenceOptions,
    ) -> Result<Detections> {
        let (orig_w, orig_h) = (image.width() as f32, image.height() as f32);

        let (input_tensor, transform) =
            preprocess_image_v2(image, self.resolution)?;

        // Build orig_target_sizes: [[resolution, resolution]]
        let orig_sizes = Array2::<i64>::from_shape_vec(
            (1, 2),
            vec![self.resolution as i64, self.resolution as i64],
        )
        .map_err(|e| Error::Inference(e.to_string()))?;

        // Convert to ort Values (needs owned arrays)
        let images_value = Value::from_array(input_tensor)
            .map_err(|e| Error::Inference(e.to_string()))?;
        let sizes_value = Value::from_array(orig_sizes)
            .map_err(|e| Error::Inference(e.to_string()))?;

        let inputs = ort::inputs![
            "images" => images_value,
            "orig_target_sizes" => sizes_value,
        ];

        let outputs = self
            .session
            .run(inputs)
            .map_err(|e| Error::Inference(e.to_string()))?;

        // Extract outputs as ndarray views
        let boxes_array = outputs["boxes"]
            .try_extract_array::<f32>()
            .map_err(|e| Error::Inference(format!("boxes: {e}")))?;
        let labels_array = outputs["labels"]
            .try_extract_array::<i64>()
            .map_err(|e| Error::Inference(format!("labels: {e}")))?;
        let scores_array = outputs["scores"]
            .try_extract_array::<f32>()
            .map_err(|e| Error::Inference(format!("scores: {e}")))?;

        // Slice off batch dimension: [1, N, 4] -> [N, 4], [1, N] -> [N]
        let boxes: ArrayView2<f32> = boxes_array
            .slice(ndarray::s![0, .., ..]);
        let labels: ArrayView1<i64> = labels_array
            .slice(ndarray::s![0, ..]);
        let scores: ArrayView1<f32> = scores_array
            .slice(ndarray::s![0, ..]);

        let mut detections = postprocess_v2(
            &boxes,
            &labels,
            &scores,
            &transform,
            0.0,
            0.0,
            options.confidence,
        );

        // Normalize to [0, 1]
        normalize_detections(&mut detections, orig_w, orig_h);

        // Apply NMS then NMM
        let detections = nms(&detections, options.nms_threshold);
        let detections = nmm(&detections, options.nmm_threshold, options.confidence);

        Ok(detections)
    }
}
