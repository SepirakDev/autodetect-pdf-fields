import { spawn } from "bun";
import { join, dirname } from "path";
import { mkdtemp, rm } from "fs/promises";
import { tmpdir } from "os";

const PROJECT_ROOT = dirname(import.meta.dir);
const BINARY_PATH =
  process.env.BINARY_PATH ||
  join(PROJECT_ROOT, "target", "release", "autodetect-pdf-fields");
const MODEL_PATH =
  process.env.MODEL_PATH || join(PROJECT_ROOT, "models", "model_704_int8.onnx");
const ANTHROPIC_API_KEY = process.env.ANTHROPIC_API_KEY;

export interface AvailableField {
  type: string;
  name: string;
  id: string;
}

export interface DetectInput {
  file: File;
  label?: boolean;
  confidence?: number;
  debug?: boolean;
  availableFields?: AvailableField[];
}

export interface DetectedField {
  type: string;
  name?: string;
  field_id?: string;
  page: number;
  confidence: number;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface DetectResult {
  fields: DetectedField[];
  debug_pdf?: string;
}

export async function runDetection(input: DetectInput): Promise<DetectResult> {
  const { file, label = false, confidence = 0.3, debug = false, availableFields } = input;

  const pdfBuffer = await file.arrayBuffer();
  if (pdfBuffer.byteLength === 0) {
    throw new Error("Empty PDF");
  }

  const tmpDir = await mkdtemp(join(tmpdir(), "apf-"));
  const inputPath = join(tmpDir, "input.pdf");
  const debugPath = debug ? join(tmpDir, "debug.pdf") : null;

  try {
    await Bun.write(inputPath, pdfBuffer);

    const args: string[] = [inputPath, "-m", MODEL_PATH, "-c", confidence.toString(), "--pretty"];

    if (label) {
      if (!ANTHROPIC_API_KEY) {
        throw new Error("Labeling requested but ANTHROPIC_API_KEY is not set on server");
      }
      args.push("--label");
    }

    if (availableFields && availableFields.length > 0) {
      const fieldsPath = join(tmpDir, "fields.json");
      await Bun.write(fieldsPath, JSON.stringify({ availableFields }));
      args.push("--fields-file", fieldsPath);
    }

    if (debugPath) {
      args.push("--debug", debugPath);
    }

    // Run from project root (where libpdfium lives), or binary dir if BINARY_PATH is explicit
    const cwd = process.env.BINARY_PATH ? dirname(BINARY_PATH) : PROJECT_ROOT;
    const proc = spawn({
      cmd: [BINARY_PATH, ...args],
      cwd,
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
      throw new Error(`Detection failed: ${stderr.trim()}`);
    }

    const fields: DetectedField[] = JSON.parse(stdout);

    const result: DetectResult = { fields };

    if (debugPath) {
      const debugFile = Bun.file(debugPath);
      if (await debugFile.exists()) {
        const debugBuffer = await debugFile.arrayBuffer();
        result.debug_pdf = Buffer.from(debugBuffer).toString("base64");
      }
    }

    return result;
  } finally {
    await rm(tmpDir, { recursive: true, force: true }).catch(() => {});
  }
}
