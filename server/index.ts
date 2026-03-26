import { OpenAPIHandler } from "@orpc/openapi/fetch";
import { OpenAPIGenerator } from "@orpc/openapi";
import { ZodToJsonSchemaConverter } from "@orpc/zod";
import { CORSPlugin } from "@orpc/server/plugins";
import { onError } from "@orpc/server";
import { router } from "./router";
import { isAuthEnabled, isValidKey, extractBearerToken } from "./auth";

const PORT = parseInt(process.env.PORT || "3000", 10);

const handler = new OpenAPIHandler(router, {
  plugins: [new CORSPlugin()],
  interceptors: [
    onError((error) => {
      console.error("oRPC error:", error);
    }),
  ],
});

const generator = new OpenAPIGenerator({
  schemaConverters: [new ZodToJsonSchemaConverter()],
});

let specCache: object | null = null;

function fixFileSchemas(obj: any): void {
  if (obj && typeof obj === "object") {
    // Fix file fields: contentMediaType means it's a file upload
    if (obj.type === "string" && obj.contentMediaType) {
      obj.type = "string";
      obj.format = "binary";
      delete obj.contentMediaType;
    }
    for (const value of Object.values(obj)) {
      fixFileSchemas(value);
    }
  }
}

async function getSpec() {
  if (!specCache) {
    const spec: any = await generator.generate(router, {
      info: {
        title: "autodetect-pdf-fields",
        version: "0.1.0",
        description:
          "Detect and label fillable fields in PDF documents using ONNX object detection and Claude vision AI.",
      },
      servers: [{ url: "/api" }],
    });
    // Fix file schemas to use format: binary instead of contentMediaType
    fixFileSchemas(spec);

    // Add security scheme when auth is enabled
    if (isAuthEnabled()) {
      spec.components = spec.components || {};
      spec.components.securitySchemes = {
        bearerAuth: {
          type: "http",
          scheme: "bearer",
          description: "API key passed as Bearer token",
        },
      };
      spec.security = [{ bearerAuth: [] }];
    }

    specCache = spec;
  }
  return specCache;
}

const DOCS_HTML = `<!DOCTYPE html>
<html>
<head>
  <title>autodetect-pdf-fields API</title>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
</head>
<body>
  <script id="api-reference" data-url="/openapi.json"></script>
  <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>`;

const server = Bun.serve({
  port: PORT,
  async fetch(request) {
    const url = new URL(request.url);

    // Serve OpenAPI spec (outside /api prefix)
    if (url.pathname === "/openapi.json" && request.method === "GET") {
      const spec = await getSpec();
      return new Response(JSON.stringify(spec, null, 2), {
        headers: { "content-type": "application/json" },
      });
    }

    // Serve Scalar API docs UI
    if (url.pathname === "/docs" && request.method === "GET") {
      return new Response(DOCS_HTML, {
        headers: { "content-type": "text/html" },
      });
    }

    // Root redirect to docs
    if (url.pathname === "/" && request.method === "GET") {
      return Response.redirect("/docs", 302);
    }

    // Auth check for API routes (except health)
    if (url.pathname.startsWith("/api/") && url.pathname !== "/api/health") {
      if (isAuthEnabled()) {
        const token = extractBearerToken(request);
        if (!token || !isValidKey(token)) {
          return Response.json(
            { error: "Unauthorized", message: "Invalid or missing API key" },
            { status: 401, headers: { "WWW-Authenticate": "Bearer" } }
          );
        }
      }
    }

    // Handle API routes via oRPC
    const { matched, response } = await handler.handle(request, {
      prefix: "/api",
      context: {},
    });

    if (matched) {
      return response;
    }

    return Response.json({ error: "Not found" }, { status: 404 });
  },
});

console.log(`autodetect-pdf-fields server listening on http://localhost:${server.port}`);
console.log(`  API:    http://localhost:${server.port}/api`);
console.log(`  Docs:   http://localhost:${server.port}/docs`);
console.log(`  Spec:   http://localhost:${server.port}/openapi.json`);
