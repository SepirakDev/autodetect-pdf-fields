use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Model loading failed: {0}")]
    ModelLoad(String),

    #[error("Inference failed: {0}")]
    Inference(String),

    #[error("PDF open failed: {0}")]
    PdfOpen(String),

    #[error("PDF render failed: {0}")]
    PdfRender(String),

    #[error("Image processing failed: {0}")]
    ImageProcess(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
