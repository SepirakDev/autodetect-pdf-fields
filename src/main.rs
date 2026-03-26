use anyhow::Result;
use clap::Parser;
use std::fs;

use autodetect_pdf_fields::cli::Args;
use autodetect_pdf_fields::debug::write_debug_pdf;
use autodetect_pdf_fields::detection::orchestrator::{detect_fields_in_pdf, DetectOptions};
use autodetect_pdf_fields::labeler::label_fields;
use autodetect_pdf_fields::model::inference::FieldDetector;
use autodetect_pdf_fields::pdf::document::PdfDoc;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    let mut detector = FieldDetector::load(&args.model)?;

    let pdf = PdfDoc::open(&args.input)?;

    let options = DetectOptions {
        confidence: args.confidence,
        nms_threshold: args.nms,
        nmm_threshold: args.nmm,
        classify_types: !args.no_classify,
        padding: Some(20),
        page: args.page,
    };

    let mut fields = detect_fields_in_pdf(&pdf, &mut detector, &options)?;

    // Label fields via Claude if requested
    if args.label {
        label_fields(&pdf, &mut fields, Some(&args.label_model))?;
    }

    // Write debug PDF if requested
    if let Some(debug_path) = &args.debug {
        write_debug_pdf(&pdf, &fields, debug_path)?;
        eprintln!("Debug PDF written to: {}", debug_path.display());
    }

    let json = if args.pretty {
        serde_json::to_string_pretty(&fields)?
    } else {
        serde_json::to_string(&fields)?
    };

    if let Some(output_path) = &args.output {
        fs::write(output_path, &json)?;
    } else {
        println!("{json}");
    }

    Ok(())
}
