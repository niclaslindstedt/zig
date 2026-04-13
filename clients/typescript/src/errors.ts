import type { ServerErrorBody } from "./types.js";

/**
 * Thrown by every `ZigApiClient` method when the server responds with a
 * non-2xx status. `code` is the server-provided machine-readable code (e.g.
 * `"not_found"`, `"validation_error"`, `"zag_error"`), or `"http_error"` if
 * the response body could not be parsed as a `ServerErrorBody`.
 */
export class ZigApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
    public readonly body?: unknown,
  ) {
    super(message);
    this.name = "ZigApiError";
  }

  static async fromResponse(res: Response): Promise<ZigApiError> {
    const text = await res.text().catch(() => "");
    let body: unknown = text;
    let code = "http_error";
    let message = `${res.status} ${res.statusText}`.trim();

    if (text) {
      try {
        const parsed = JSON.parse(text) as Partial<ServerErrorBody>;
        body = parsed;
        if (parsed.error && typeof parsed.error === "object") {
          if (typeof parsed.error.code === "string") code = parsed.error.code;
          if (typeof parsed.error.message === "string") message = parsed.error.message;
        }
      } catch {
        // Non-JSON body; keep the text as-is.
        message = `${message}: ${text}`;
      }
    }

    return new ZigApiError(res.status, code, message, body);
  }
}
