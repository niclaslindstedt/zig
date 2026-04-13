// Thin wrapper around the published `@nlindstedt/zig-api-client` package.
//
// The bearer token is sourced from the `?token=` query parameter (set by
// `zig serve --web`'s printed URL) and persisted to localStorage so
// subsequent reloads stay authenticated. We instantiate a single module-level
// `ZigApiClient` that the rest of the app talks to through a couple of tiny
// re-exported helpers.

import { ZigApiClient } from "@nlindstedt/zig-api-client";
import type {
  ChatEvent,
  SendMessageResponse,
  StartChatResponse,
} from "@nlindstedt/zig-api-client";

export type { ChatEvent, SendMessageResponse, StartChatResponse };

const TOKEN_KEY = "zig-token";

function readToken(): string {
  if (typeof window === "undefined") return "";
  const url = new URL(window.location.href);
  const fromUrl = url.searchParams.get("token");
  if (fromUrl) {
    localStorage.setItem(TOKEN_KEY, fromUrl);
    url.searchParams.delete("token");
    window.history.replaceState({}, "", url.toString());
  }
  return localStorage.getItem(TOKEN_KEY) ?? "";
}

// Same-origin base URL: Vite's dev server proxies `/api` to the backend, and
// the production build is served directly by `zig-serve`.
const client = new ZigApiClient({
  baseUrl:
    typeof window === "undefined" ? "http://localhost:3000" : window.location.origin,
  token: readToken(),
});

export function getClient(): ZigApiClient {
  return client;
}

export function startChat(
  initialPrompt: string,
  name?: string,
): Promise<StartChatResponse> {
  return client.startChat(initialPrompt, name);
}

export function sendChatMessage(
  sessionId: string,
  message: string,
): Promise<SendMessageResponse> {
  return client.sendChatMessage(sessionId, message);
}

export function streamChat(
  sessionId: string,
  onEvent: (event: ChatEvent) => void,
  onError?: (err: unknown) => void,
): () => void {
  const handle = client.streamChat(sessionId, onEvent, {
    onError: onError ?? undefined,
  });
  return () => handle.close();
}
