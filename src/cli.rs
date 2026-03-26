use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "autodetect-pdf-fields")]
#[command(about = "Detect fillable fields in PDF documents using ONNX object detection")]
pub struct Args {
    /// Path to PDF or image file
    pub input: PathBuf,

    /// Path to ONNX model file
    #[arg(short, long, default_value = "models/model_704_int8.onnx")]
    pub model: PathBuf,

    /// Confidence threshold
    #[arg(short, long, default_value_t = 0.3)]
    pub confidence: f32,

    /// NMS IoU threshold
    #[arg(long, default_value_t = 0.1)]
    pub nms: f32,

    /// NMM overlap threshold
    #[arg(long, default_value_t = 0.5)]
    pub nmm: f32,

    /// Disable regex-based type classification
    #[arg(long)]
    pub no_classify: bool,

    /// Process only this page (0-indexed)
    #[arg(long)]
    pub page: Option<usize>,

    /// Pretty-print JSON output
    #[arg(long)]
    pub pretty: bool,

    /// Write output to file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Write a debug PDF with bounding boxes to this path
    #[arg(long)]
    pub debug: Option<PathBuf>,

    /// Label fields using Claude's vision API (requires ANTHROPIC_API_KEY)
    #[arg(long)]
    pub label: bool,

    /// Model to use for labeling
    #[arg(long, default_value = "claude-sonnet-4-20250514")]
    pub label_model: String,

    /// Path to JSON file with available fields for mapping
    #[arg(long)]
    pub fields_file: Option<PathBuf>,
}
