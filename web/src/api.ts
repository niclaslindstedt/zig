// Thin client for the zig-serve API. The bearer token is sourced from the
// `?token=` query parameter (set by `zig serve --web`'s printed URL) and
// persisted to localStorage so subsequent reloads stay authenticated.

const TOKEN_KEY = "zig-token";

export function getToken(): string {
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

async function api<T>(path: string, init: RequestInit = {}): Promise<T> {
  const res = await fetch(`/api/v1${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      authorization: `Bearer ${getToken()}`,
      ...(init.headers ?? {}),
    },
  });
  if (!res.ok) {
    const body = await res.text().catch(() => "");
    throw new Error(`${res.status} ${res.statusText}: ${body}`);
  }
  return res.json() as Promise<T>;
}

export interface StartChatResponse {
  session_id: string;
  output_path: string;
}

export interface ChatEvent {
  role: "user" | "agent" | "system";
  text: string;
}

export function startChat(
  initialPrompt: string,
  name?: string,
): Promise<StartChatResponse> {
  return api<StartChatResponse>("/web/chat", {
    method: "POST",
    body: JSON.stringify({ initial_prompt: initialPrompt, name }),
  });
}

export function sendChatMessage(
  sessionId: string,
  message: string,
): Promise<{ ok: boolean }> {
  return api<{ ok: boolean }>(`/web/chat/${sessionId}`, {
    method: "POST",
    body: JSON.stringify({ message }),
  });
}

export function streamChat(
  sessionId: string,
  onEvent: (event: ChatEvent) => void,
  onError?: (err: Event) => void,
): () => void {
  const token = encodeURIComponent(getToken());
  const es = new EventSource(
    `/api/v1/web/chat/${sessionId}/stream?token=${token}`,
  );
  es.onmessage = (e) => {
    try {
      onEvent(JSON.parse(e.data) as ChatEvent);
    } catch {
      onEvent({ role: "agent", text: e.data });
    }
  };
  if (onError) es.onerror = onError;
  return () => es.close();
}
