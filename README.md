# autodetect-pdf-fields

Detect fillable fields (text inputs, checkboxes, signatures, dates, numbers) in PDF documents using an ONNX object detection model.

A Rust reimplementation of [DocuSeal's](https://github.com/docusealco/docuseal) autodetect feature, combining ML inference with PDF structural analysis for accurate field detection.

## How It Works

1. **ML Inference**: Renders each PDF page to an image and runs it through an ONNX object detection model (RT-DETR based) that detects text fields and checkboxes
2. **Structural Analysis**: Extracts underscores (`___`) and horizontal lines from the PDF to confirm ML detections
3. **Confidence Boosting**: ML detections that overlap with structural indicators get a confidence boost
4. **Type Classification**: Uses regex patterns on text preceding each field to classify types (date, signature, number)

## Setup

### 1. Install pdfium

The tool requires the pdfium shared library:

**macOS (Homebrew):**
```bash
brew install pdfium
```

**Or download from:**
https://github.com/nicbarker/pdfium-binaries/releases

### 2. Download the ONNX model

```bash
./scripts/download_model.sh
```

Or manually download `model_704_int8.onnx` from:
https://github.com/docusealco/fields-detection/releases/download/2.0.0/model_704_int8.onnx

Place it at `models/model_704_int8.onnx`.

### 3. Build

```bash
cargo build --release
```

## Usage

```bash
# Basic usage
autodetect-pdf-fields document.pdf

# Pretty-print output
autodetect-pdf-fields document.pdf --pretty

# Custom model path and confidence threshold
autodetect-pdf-fields document.pdf -m /path/to/model.onnx -c 0.5

# Process a single page
autodetect-pdf-fields document.pdf --page 0

# Save output to file
autodetect-pdf-fields document.pdf -o fields.json --pretty
```

## Output Format

```json
[
  {
    "type": "text",
    "page": 0,
    "confidence": 0.87,
    "x": 0.123,
    "y": 0.456,
    "w": 0.234,
    "h": 0.034
  },
  {
    "type": "signature",
    "page": 1,
    "confidence": 0.92,
    "x": 0.08,
    "y": 0.61,
    "w": 0.35,
    "h": 0.05
  }
]
```

Coordinates are normalized to [0, 1] relative to the page dimensions.

## Field Types

| Type | Description |
|------|-------------|
| `text` | General text input field |
| `checkbox` | Checkbox (detected by ML model) |
| `date` | Date field (classified by preceding text like "Date:", "Datum:") |
| `signature` | Signature field (classified by preceding text like "Signature:", "Sign here:") |
| `number` | Number field (classified by preceding text like "Price:", "$", "Total:") |

## CLI Options

```
Arguments:
  <INPUT>              Path to PDF or image file

Options:
  -m, --model <PATH>   Path to ONNX model [default: models/model_704_int8.onnx]
  -c, --confidence <F> Confidence threshold [default: 0.3]
  --nms <F>            NMS IoU threshold [default: 0.1]
  --nmm <F>            NMM overlap threshold [default: 0.5]
  --no-classify        Disable regex-based type classification
  --page <N>           Process only this page (0-indexed)
  --pretty             Pretty-print JSON output
  -o, --output <PATH>  Write output to file instead of stdout
```

## Library Usage

```rust
use autodetect_pdf_fields::{detect_fields_in_pdf, DetectOptions};
use autodetect_pdf_fields::model::inference::FieldDetector;
use autodetect_pdf_fields::pdf::document::PdfDoc;

let detector = FieldDetector::load("models/model_704_int8.onnx".as_ref())?;
let pdf = PdfDoc::open("document.pdf".as_ref())?;

let options = DetectOptions::default();
let fields = detect_fields_in_pdf(&pdf, &detector, &options)?;

for field in &fields {
    println!("{:?} at page {} ({:.0}% confidence)",
        field.field_type, field.page, field.confidence * 100.0);
}
```

## Model

The ONNX model is from [docusealco/fields-detection](https://github.com/docusealco/fields-detection). It is a V2 object detection model (RT-DETR architecture) trained to detect text fields and checkboxes in document images. The INT8 quantized version (`model_704_int8.onnx`) is used for fast CPU inference at 704x704 resolution.
