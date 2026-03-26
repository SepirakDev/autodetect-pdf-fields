# autodetect-pdf-fields

Detect and label fillable fields in PDF documents. Uses an ONNX object detection model to find fields, then optionally sends them to Claude's vision API for semantic labeling and field ID mapping.

## How It Works

1. **ML Detection** -- Renders each PDF page and runs it through an ONNX object detection model (RT-DETR) that locates text fields and checkboxes
2. **Structural Analysis** -- Extracts underscores and horizontal lines from the PDF to confirm ML detections and boost confidence
3. **Type Classification** -- Uses regex patterns on surrounding text to classify fields as date, signature, number, or text
4. **Labeling** -- Sends each page to Claude's vision API with detected bounding boxes. Claude returns semantic labels like "Account Number" or "Member 1 Signature"
5. **Field Mapping** -- Matches detected fields against a provided list of available fields and returns their IDs

## Quick Start

Download a release from [Releases](https://github.com/SepirakDev/autodetect-pdf-fields/releases), extract, and run:

```bash
# Detect fields
./autodetect-pdf-fields document.pdf --pretty

# Detect and label with Claude
ANTHROPIC_API_KEY=sk-ant-... ./autodetect-pdf-fields document.pdf --pretty --label

# Label and map to available fields
ANTHROPIC_API_KEY=sk-ant-... ./autodetect-pdf-fields document.pdf --pretty --label --fields-file fields.json

# Generate a debug PDF with bounding boxes
./autodetect-pdf-fields document.pdf --debug debug.pdf
```

Release archives are self-contained -- they include the binary, pdfium library, and ONNX model.

---

## HTTP API

The `server/` directory contains a Bun server that wraps the CLI as a REST API with OpenAPI documentation.

### Running the Server

```bash
cd server
bun install
ANTHROPIC_API_KEY=sk-ant-... bun run start
```

The server starts on port 3000 (configurable via `PORT` env var).

| URL | Description |
|-----|-------------|
| `GET /docs` | Interactive API docs (Scalar UI) |
| `GET /openapi.json` | OpenAPI 3.1 specification |
| `GET /api/health` | Health check |
| `POST /api/detect` | Detect fields in a PDF |

### `POST /api/detect`

Upload a PDF and receive detected fields as JSON.

**Request (multipart/form-data):**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `file` | binary | yes | PDF document |
| `label` | boolean | no | Enable Claude vision labeling (default: `false`) |
| `confidence` | number | no | Confidence threshold 0-1 (default: `0.3`) |
| `debug` | boolean | no | Include base64 debug PDF in response (default: `false`) |
| `availableFields` | JSON array | no | Fields to match against (see below) |

**`availableFields` format:**

```json
[
  { "type": "text", "name": "Full Name", "id": "full_name" },
  { "type": "date", "name": "Date of Birth", "id": "dob" },
  { "type": "signature", "name": "Customer Signature", "id": "sig_1" }
]
```

Each object requires:
- `type` -- one of `text`, `checkbox`, `date`, `signature`, `number`
- `name` -- descriptive label
- `id` -- unique identifier returned as `field_id` in the response

**Example with curl:**

```bash
# Basic detection
curl -X POST http://localhost:3000/api/detect \
  -F file=@document.pdf

# With labeling
curl -X POST http://localhost:3000/api/detect \
  -F file=@document.pdf \
  -F label=true

# With labeling and field mapping
curl -X POST http://localhost:3000/api/detect \
  -F file=@document.pdf \
  -F label=true \
  -F 'availableFields=[{"type":"text","name":"Full Name","id":"full_name"},{"type":"date","name":"Date of Birth","id":"dob"}]'
```

**Response:**

```json
{
  "fields": [
    {
      "type": "text",
      "name": "Full Name",
      "field_id": "full_name",
      "page": 0,
      "confidence": 0.83,
      "x": 0.22,
      "y": 0.27,
      "w": 0.14,
      "h": 0.06
    },
    {
      "type": "date",
      "name": "Date of Birth",
      "field_id": "dob",
      "page": 0,
      "confidence": 0.79,
      "x": 0.08,
      "y": 0.35,
      "w": 0.14,
      "h": 0.06
    }
  ]
}
```

With `debug=true`, the response includes a `debug_pdf` field containing the base64-encoded annotated PDF.

**Response fields:**

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `text`, `checkbox`, `date`, `signature`, or `number` |
| `name` | string? | Semantic label (present when `label=true`) |
| `field_id` | string? | Matched available field ID (present when `availableFields` provided) |
| `page` | integer | Page number (0-indexed) |
| `confidence` | number | Detection confidence (0-1) |
| `x`, `y` | number | Top-left corner, normalized to [0, 1] |
| `w`, `h` | number | Width and height, normalized to [0, 1] |

### `GET /api/health`

Returns `{"status": "ok"}`.

### Server Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Server port |
| `BINARY_PATH` | `../target/release/autodetect-pdf-fields` | Path to CLI binary |
| `MODEL_PATH` | `../models/model_704_int8.onnx` | Path to ONNX model |
| `ANTHROPIC_API_KEY` | -- | Required for `label=true` |

---

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
  --fields-file <PATH>     JSON file with available fields for ID mapping
```

### Available Fields File Format

```json
{
  "availableFields": [
    { "type": "text", "name": "Full Name", "id": "full_name" },
    { "type": "date", "name": "Date of Birth", "id": "dob" },
    { "type": "signature", "name": "Member Signature", "id": "member_sig" }
  ]
}
```

When provided with `--label`, Claude matches each detected field to the best available field and returns its `id` as `field_id` in the output.

### CLI Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | Only with `--label` | Anthropic API key for Claude vision labeling |

---

## Rust Library

The core detection engine is available as a Rust library crate (`autodetect_pdf_fields`).

### Add to Cargo.toml

```toml
[dependencies]
autodetect-pdf-fields = { git = "https://github.com/SepirakDev/autodetect-pdf-fields" }
```

### Detection Only

```rust
use autodetect_pdf_fields::{detect_fields_in_pdf, DetectOptions};
use autodetect_pdf_fields::model::inference::FieldDetector;
use autodetect_pdf_fields::pdf::document::PdfDoc;

let mut detector = FieldDetector::load("models/model_704_int8.onnx".as_ref())?;
let pdf = PdfDoc::open("document.pdf".as_ref())?;

let options = DetectOptions::default();
let fields = detect_fields_in_pdf(&pdf, &mut detector, &options)?;

for field in &fields {
    println!("{:?} at page {} ({:.0}%)",
        field.field_type, field.page, field.confidence * 100.0);
}
```

### With Claude Labeling

```rust
use autodetect_pdf_fields::labeler::label_fields;

let mut fields = detect_fields_in_pdf(&pdf, &mut detector, &options)?;

// Label with Claude (requires ANTHROPIC_API_KEY env var)
label_fields(&pdf, &mut fields, None, None)?;

for field in &fields {
    println!("{}: {:?}",
        field.name.as_deref().unwrap_or("unlabeled"),
        field.field_type);
}
```

### With Available Field Mapping

```rust
use autodetect_pdf_fields::output::{AvailableField, AvailableFieldsFile};
use autodetect_pdf_fields::labeler::label_fields;

let available = vec![
    AvailableField { field_type: "text".into(), name: "Full Name".into(), id: "full_name".into() },
    AvailableField { field_type: "date".into(), name: "Date of Birth".into(), id: "dob".into() },
];

let mut fields = detect_fields_in_pdf(&pdf, &mut detector, &options)?;
label_fields(&pdf, &mut fields, None, Some(&available))?;

for field in &fields {
    if let Some(id) = &field.field_id {
        println!("{} -> {}", field.name.as_deref().unwrap_or("?"), id);
    }
}
```

### Key Types

```rust
pub struct DetectedField {
    pub field_type: FieldType,       // text, checkbox, date, signature, number
    pub name: Option<String>,        // semantic label (from heuristic or Claude)
    pub field_id: Option<String>,    // matched available field ID
    pub page: usize,                 // 0-indexed page number
    pub confidence: f32,             // 0.0 - 1.0
    pub bbox: BBox,                  // x, y, w, h normalized to [0, 1]
}

pub struct DetectOptions {
    pub confidence: f32,             // default: 0.3
    pub nms_threshold: f32,          // default: 0.1
    pub nmm_threshold: f32,          // default: 0.5
    pub classify_types: bool,        // default: true
    pub padding: Option<u32>,        // default: Some(20)
    pub page: Option<usize>,         // default: None (all pages)
}

pub struct AvailableField {
    pub field_type: String,          // "text", "date", etc.
    pub name: String,                // "Full Name"
    pub id: String,                  // "full_name"
}
```

### Debug PDF Output

```rust
use autodetect_pdf_fields::debug::write_debug_pdf;

let fields = detect_fields_in_pdf(&pdf, &mut detector, &options)?;
write_debug_pdf(&pdf, &fields, "debug.pdf".as_ref())?;
```

---

## Building from Source

```bash
# Download the ONNX model
./scripts/download_model.sh

# Build the Rust binary
cargo build --release

# Run the server
cd server && bun install && bun run start
```

Requires pdfium at runtime. The binary searches for `libpdfium` in the current directory first, then the system library path. Download from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries/releases).

## Model

The ONNX model is from [docusealco/fields-detection](https://github.com/docusealco/fields-detection). RT-DETR architecture, INT8 quantized, 704x704 input resolution. Detects two classes: text fields and checkboxes.
