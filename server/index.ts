import { spawn } from "bun";
import { join, dirname } from "path";
import { mkdtemp, rm } from "fs/promises";
import { tmpdir } from "os";

const PORT = parseInt(process.env.PORT || "3000", 10);
const BINARY_PATH =
  process.env.BINARY_PATH ||
  join(dirname(import.meta.dir), "target", "release", "autodetect-pdf-fields");
const MODEL_PATH =
  process.env.MODEL_PATH || join(dirname(import.meta.dir), "models", "model_704_int8.onnx");
const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;

async function handleDetect(req: Request): Promise<Response> {
  const contentType = req.headers.get("content-type") || "";

  let pdfBuffer: ArrayBuffer;
  let label = false;
  let confidence = 0.3;
  let pretty = false;
  let debug = false;

  // Parse query params
  const url = new URL(req.url);
  if (url.searchParams.has("label")) label = url.searchParams.get("label") !== "false";
  if (url.searchParams.has("confidence"))
    confidence = parseFloat(url.searchParams.get("confidence")!);
  if (url.searchParams.has("pretty")) pretty = url.searchParams.get("pretty") !== "false";
  if (url.searchParams.has("debug")) debug = url.searchParams.get("debug") !== "false";

  // Accept multipart/form-data or raw application/pdf
  if (contentType.includes("multipart/form-data")) {
    const formData = await req.formData();
    const file = formData.get("file") as File | null;
    if (!file) {
      return Response.json({ error: "No 'file' field in form data" }, { status: 400 });
    }
    pdfBuffer = await file.arrayBuffer();
  } else if (
    contentType.includes("application/pdf") ||
    contentType.includes("application/octet-stream")
  ) {
    pdfBuffer = await req.arrayBuffer();
  } else {
    return Response.json(
      {
        error: "Send a PDF as multipart/form-data (field: file) or raw application/pdf body",
      },
      { status: 400 }
    );
  }

  if (pdfBuffer.byteLength === 0) {
    return Response.json({ error: "Empty PDF" }, { status: 400 });
  }

  // Write PDF to temp file
  const tmpDir = await mkdtemp(join(tmpdir(), "apf-"));
  const inputPath = join(tmpDir, "input.pdf");
  const debugPath = debug ? join(tmpDir, "debug.pdf") : null;

  try {
    await Bun.write(inputPath, pdfBuffer);

    // Build CLI args
    const args: string[] = [
      inputPath,
      "-m",
      MODEL_PATH,
      "-c",
      confidence.toString(),
      "--pretty",
    ];

    if (label) {
      if (!ANTHROPIC_API_KEY) {
        return Response.json(
          { error: "Labeling requested but ANTHROPIC_API_KEY is not set on server" },
          { status: 500 }
        );
      }
      args.push("--label");
    }

    if (debugPath) {
      args.push("--debug", debugPath);
    }

    // Run the binary from its directory so it finds libpdfium
    const binaryDir = dirname(BINARY_PATH);
    const proc = spawn({
      cmd: [BINARY_PATH, ...args],
      cwd: binaryDir,
      stdout: "pipe",
      stderr: "pipe",
      env: {
        ...process.env,
        ...(ANTHROPIC_API_KEY ? { ANTHROPIC_API_KEY } : {}),
      },
    });

    const [stdout, stderr] = await Promise.all([
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
    ]);

    const exitCode = await proc.exited;

    if (exitCode !== 0) {
      return Response.json(
        { error: "Detection failed", details: stderr.trim() },
        { status: 500 }
      );
    }

    // Parse the JSON output
    let fields: unknown;
    try {
      fields = JSON.parse(stdout);
    } catch {
      return Response.json(
        { error: "Failed to parse detector output", raw: stdout.trim() },
        { status: 500 }
      );
    }

    // If debug PDF was requested, return multipart or include base64
    if (debugPath) {
      const debugFile = Bun.file(debugPath);
      const debugExists = await debugFile.exists();

      if (debugExists) {
        const debugBuffer = await debugFile.arrayBuffer();
        const debugBase64 = Buffer.from(debugBuffer).toString("base64");

        const result = pretty
          ? JSON.stringify({ fields, debug_pdf: debugBase64 }, null, 2)
          : JSON.stringify({ fields, debug_pdf: debugBase64 });

        return new Response(result, {
          headers: { "content-type": "application/json" },
        });
      }
    }

    const result = pretty ? JSON.stringify({ fields }, null, 2) : JSON.stringify({ fields });

    return new Response(result, {
      headers: { "content-type": "application/json" },
    });
  } finally {
    await rm(tmpDir, { recursive: true, force: true }).catch(() => {});
  }
}

function handleHealth(): Response {
  return Response.json({ status: "ok" });
}

const server = Bun.serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);

    if (url.pathname === "/health" && req.method === "GET") {
      return handleHealth();
    }

    if (url.pathname === "/detect" && req.method === "POST") {
      return handleDetect(req);
    }

    return Response.json(
      {
        error: "Not found",
        endpoints: {
          "POST /detect": "Upload a PDF to detect fields",
          "GET /health": "Health check",
        },
      },
      { status: 404 }
    );
  },
});

console.log(`autodetect-pdf-fields server listening on http://localhost:${server.port}`);
