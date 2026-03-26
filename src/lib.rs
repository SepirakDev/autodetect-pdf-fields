pub mod cli;
pub mod debug;
pub mod detection;
pub mod error;
pub mod geometry;
pub mod labeler;
pub mod model;
pub mod output;
pub mod pdf;

pub use detection::orchestrator::{detect_fields_in_pdf, DetectOptions};
pub use error::{Error, Result};
pub use output::{DetectedField, FieldType};
