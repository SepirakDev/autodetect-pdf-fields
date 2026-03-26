FROM oven/bun:1 AS base

WORKDIR /app

# Download the pre-built linux-x64 release artifact
RUN apt-get update && apt-get install -y curl tar && rm -rf /var/lib/apt/lists/*

ARG RELEASE_VERSION=v0.2.0
RUN curl -L -o /tmp/release.tar.gz \
    "https://github.com/SepirakDev/autodetect-pdf-fields/releases/download/${RELEASE_VERSION}/autodetect-pdf-fields-linux-x64.tar.gz" \
    && tar -xzf /tmp/release.tar.gz -C /tmp \
    && mv /tmp/autodetect-pdf-fields-linux-x64/autodetect-pdf-fields /app/autodetect-pdf-fields \
    && mv /tmp/autodetect-pdf-fields-linux-x64/libpdfium.so /app/libpdfium.so \
    && mkdir -p /app/models \
    && mv /tmp/autodetect-pdf-fields-linux-x64/models/model_704_int8.onnx /app/models/model_704_int8.onnx \
    && rm -rf /tmp/release.tar.gz /tmp/autodetect-pdf-fields-linux-x64

RUN chmod +x /app/autodetect-pdf-fields

# Copy server files and install deps
COPY server/package.json server/bun.lock ./server/
WORKDIR /app/server
RUN bun install --frozen-lockfile
COPY server/index.ts server/router.ts server/detect.ts server/auth.ts ./

# Set environment for the server
ENV BINARY_PATH=/app/autodetect-pdf-fields
ENV MODEL_PATH=/app/models/model_704_int8.onnx
ENV LD_LIBRARY_PATH=/app
ENV PORT=3000

EXPOSE 3000

CMD ["bun", "run", "index.ts"]
