const API_KEYS: Set<string> | null = process.env.API_KEYS
  ? new Set(
      process.env.API_KEYS.split(",")
        .map((k) => k.trim())
        .filter(Boolean)
    )
  : null;

export function isAuthEnabled(): boolean {
  return API_KEYS !== null && API_KEYS.size > 0;
}

export function isValidKey(key: string): boolean {
  if (!API_KEYS) return true;
  return API_KEYS.has(key);
}

export function extractBearerToken(request: Request): string | null {
  const header = request.headers.get("authorization");
  if (!header?.startsWith("Bearer ")) return null;
  return header.slice(7);
}
