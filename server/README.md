# autodetect-pdf-fields HTTP Server

Bun server that wraps the `autodetect-pdf-fields` binary as an HTTP API.

## Setup

1. Build the Rust binary (from the project root):
   ```bash
   cargo build --release
   ```

2. Ensure the ONNX model is at `models/model_704_int8.onnx` (from the project root):
   ```bash
   ./scripts/download_model.sh
   ```

3. Start the server:
   ```bash
   cd server
   bun run start
   ```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3000` | Server port |
| `BINARY_PATH` | `../target/release/autodetect-pdf-fields` | Path to the CLI binary |
| `MODEL_PATH` | `../models/model_704_int8.onnx` | Path to ONNX model |
| `ANTHROPIC_API_KEY` | _(none)_ | Required for `?label=true` |

## API

### `POST /detect`

Upload a PDF and receive detected fields as JSON.

**Request** (multipart/form-data):
```bash
curl -X POST http://localhost:3000/detect \
  -F file=@document.pdf
```

**Request** (raw PDF body):
```bash
curl -X POST http://localhost:3000/detect \
  -H "Content-Type: application/pdf" \
  --data-binary @document.pdf
```

**Query Parameters:**

| Param | Default | Description |
|-------|---------|-------------|
| `label` | `false` | Enable Claude vision labeling |
| `confidence` | `0.3` | Confidence threshold |
| `pretty` | `false` | Pretty-print JSON |
| `debug` | `false` | Include debug PDF (base64) in response |

**Response:**
```json
{
  "fields": [
    {
      "type": "text",
      "name": "Account Number",
      "page": 0,
      "confidence": 0.83,
      "x": 0.22,
      "y": 0.27,
      "w": 0.14,
      "h": 0.06
    }
  ]
}
```

With `?debug=true`, the response includes a `debug_pdf` field containing the base64-encoded debug PDF.

### `GET /health`

Returns `{"status": "ok"}`.
