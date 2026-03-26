use std::path::Path;

use pdfium_render::prelude::*;

use crate::error::{Error, Result};

pub struct PdfDoc {
    // Document must be dropped before pdfium - field order matters for drop
    document: Option<PdfDocument<'static>>,
    #[allow(dead_code)]
    pdfium: Pdfium,
}

unsafe impl Send for PdfDoc {}

impl PdfDoc {
    pub fn open(path: &Path) -> Result<Self> {
        // Try to bind pdfium from current directory first, then system
        let bindings = Pdfium::bind_to_library(
            Pdfium::pdfium_platform_library_name_at_path("./"),
        )
        .or_else(|_| Pdfium::bind_to_system_library())
        .map_err(|e| Error::PdfOpen(format!("Could not load pdfium library: {e}")))?;

        let pdfium = Pdfium::new(bindings);
        let document: PdfDocument<'static> = unsafe {
            std::mem::transmute(
                pdfium
                    .load_pdf_from_file(path, None)
                    .map_err(|e| Error::PdfOpen(format!("{path:?}: {e}")))?,
            )
        };

        Ok(Self {
            document: Some(document),
            pdfium,
        })
    }

    pub fn page_count(&self) -> usize {
        self.document.as_ref().unwrap().pages().len() as usize
    }

    pub fn document(&self) -> &PdfDocument<'static> {
        self.document.as_ref().unwrap()
    }
}

impl Drop for PdfDoc {
    fn drop(&mut self) {
        // Explicitly drop document before pdfium
        self.document.take();
    }
}
