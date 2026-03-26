import { os } from "@orpc/server";
import { oz } from "@orpc/zod";
import { z } from "zod";
import { runDetection } from "./detect";

const AvailableFieldSchema = z.object({
  type: z
    .enum(["text", "checkbox", "date", "signature", "number"])
    .describe("Field type"),
  name: z.string().describe("Descriptive label for the field"),
  id: z.string().describe("Unique identifier for the field"),
});

const DetectedFieldSchema = z.object({
  type: z.enum(["text", "checkbox", "date", "signature", "number"]),
  name: z.string().optional().describe("Semantic label for the field"),
  field_id: z.string().optional().describe("Matched available field ID"),
  page: z.number().int().describe("Page number (0-indexed)"),
  confidence: z.number().describe("Detection confidence (0-1)"),
  x: z.number().describe("Normalized X coordinate (0-1)"),
  y: z.number().describe("Normalized Y coordinate (0-1)"),
  w: z.number().describe("Normalized width (0-1)"),
  h: z.number().describe("Normalized height (0-1)"),
});

const detect = os
  .route({ method: "POST", path: "/detect" })
  .input(
    z.object({
      file: oz.file().type("application/pdf").describe("PDF document to analyze"),
      label: z
        .union([z.boolean(), z.string().transform((s) => s === "true")])
        .optional()
        .default(false)
        .describe("Enable Claude vision labeling"),
      confidence: z
        .union([z.number(), z.string().transform((s) => parseFloat(s))])
        .pipe(z.number().min(0).max(1))
        .optional()
        .default(0.3)
        .describe("Confidence threshold"),
      debug: z
        .union([z.boolean(), z.string().transform((s) => s === "true")])
        .optional()
        .default(false)
        .describe("Include debug PDF with bounding boxes (base64)"),
      availableFields: z
        .union([
          z.array(AvailableFieldSchema),
          z.string().transform((s) => JSON.parse(s) as z.infer<typeof AvailableFieldSchema>[]),
        ])
        .optional()
        .describe("Available fields to match detected fields against (JSON array or string)"),
    })
  )
  .output(
    z.object({
      fields: z.array(DetectedFieldSchema),
      debug_pdf: z.string().optional().describe("Base64-encoded debug PDF"),
    })
  )
  .handler(async ({ input }) => {
    return runDetection(input);
  });

const health = os
  .route({ method: "GET", path: "/health" })
  .output(z.object({ status: z.string() }))
  .handler(async () => {
    return { status: "ok" };
  });

export const router = {
  detect,
  health,
};
