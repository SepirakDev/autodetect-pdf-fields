#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
MODEL_DIR="$SCRIPT_DIR/../models"
mkdir -p "$MODEL_DIR"

echo "Downloading ONNX model..."
curl -L -o "$MODEL_DIR/model_704_int8.onnx" \
  "https://github.com/docusealco/fields-detection/releases/download/2.0.0/model_704_int8.onnx"

echo "Done. Model saved to: $MODEL_DIR/model_704_int8.onnx"
