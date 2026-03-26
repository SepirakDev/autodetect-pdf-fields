# autodetect-pdf-fields

Detect and label fillable fields in PDF documents. Uses an ONNX object detection model to find fields, then optionally sends them to Claude's vision API for semantic labeling.

## How It Works

1. **ML Detection** -- Renders each PDF page to an image and runs it through an ONNX object detection model (RT-DETR) that locates text fields and checkboxes
2. **Structural Analysis** -- Extracts underscores (`___`) and horizontal lines from the PDF to confirm ML detections and boost confidence
3. **Type Classification** -- Uses regex patterns on surrounding text to classify fields as date, signature, number, or text
4. **Labeling** (with `--label`) -- Sends each page to Claude's vision API with the detected bounding boxes. Claude reads the document and returns semantic labels like "Account Number", "Date of Birth", "Member 1 Signature"

## Quick Start

Download a release from [Releases](https://github.com/SepirakDev/autodetect-pdf-fields/releases), extract, and run:

```bash
# Detect fields
./autodetect-pdf-fields document.pdf --pretty

# Detect and label fields with Claude
ANTHROPIC_API_KEY=sk-ant-... ./autodetect-pdf-fields document.pdf --pretty --label

# Generate a debug PDF with bounding box visualization
./autodetect-pdf-fields document.pdf --debug debug.pdf
```

Release archives are self-contained -- they include the binary, pdfium library, and ONNX model.

## Output

### Without `--label`

```json
[
  {
    "type": "date",
    "name": "ACCOUNT OPEN DATE",
    "page": 0,
    "confidence": 0.83,
    "x": 0.08,
    "y": 0.27,
    "w": 0.14,
    "h": 0.06
  }
]
```

The `name` field contains a heuristic label extracted from the preceding text in the PDF. It's often noisy for complex layouts like tables.

### With `--label`

```json
[
  {
    "type": "date",
    "name": "Account Open Date",
    "page": 0,
    "confidence": 0.83,
    "x": 0.08,
    "y": 0.27,
    "w": 0.14,
    "h": 0.06
  },
  {
    "type": "text",
    "name": "Member 1 Signature",
    "page": 5,
    "confidence": 0.51,
    "x": 0.63,
    "y": 0.22,
    "w": 0.27,
    "h": 0.02
  }
]
```

Claude refines the heuristic labels into clean, human-readable names. One API call per page.

Coordinates (`x`, `y`, `w`, `h`) are normalized to [0, 1] relative to the page dimensions.

## Field Types

| Type | Description |
|------|-------------|
| `text` | General text input |
| `checkbox` | Checkbox |
| `date` | Date field (e.g., preceded by "Date:", "Datum:") |
| `signature` | Signature field (e.g., preceded by "Signature:", "Sign here:") |
| `number` | Number/currency field (e.g., preceded by "Price:", "$", "Total:") |

## CLI Reference

```
autodetect-pdf-fields [OPTIONS] <INPUT>

Arguments:
  <INPUT>                  Path to PDF file

Options:
  -m, --model <PATH>       Path to ONNX model [default: models/model_704_int8.onnx]
  -c, --confidence <F>     Confidence threshold [default: 0.3]
  --nms <F>                NMS IoU threshold [default: 0.1]
  --nmm <F>                NMM overlap threshold [default: 0.5]
  --no-classify            Disable regex-based type classification
  --page <N>               Process only this page (0-indexed)
  --pretty                 Pretty-print JSON output
  -o, --output <PATH>      Write JSON output to file instead of stdout
  --debug <PATH>           Write a debug PDF with bounding boxes
  --label                  Label fields using Claude's vision API
  --label-model <MODEL>    Claude model for labeling [default: claude-sonnet-4-20250514]
```

## Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | Only with `--label` | Anthropic API key for Claude vision labeling |

## Building from Source

```bash
# Install dependencies
./scripts/download_model.sh

# Build
cargo build --release
```

Requires pdfium at runtime. The binary searches for `libpdfium` in the current directory first, then the system library path. Download from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases).

## Library Usage

```rust
use autodetect_pdf_fields::{detect_fields_in_pdf, DetectOptions};
use autodetect_pdf_fields::model::inference::FieldDetector;
use autodetect_pdf_fields::pdf::document::PdfDoc;
use autodetect_pdf_fields::labeler::label_fields;

let mut detector = FieldDetector::load("models/model_704_int8.onnx".as_ref())?;
let pdf = PdfDoc::open("document.pdf".as_ref())?;

let options = DetectOptions::default();
let mut fields = detect_fields_in_pdf(&pdf, &mut detector, &options)?;

// Optional: label with Claude
label_fields(&pdf, &mut fields, None)?;

for field in &fields {
    println!("{}: {:?} ({:.0}%)",
        field.name.as_deref().unwrap_or("unlabeled"),
        field.field_type,
        field.confidence * 100.0);
}
```

## Model

The ONNX model is from [docusealco/fields-detection](https://github.com/docusealco/fields-detection). RT-DETR architecture, INT8 quantized, 704x704 input resolution. Detects two classes: text fields and checkboxes.
