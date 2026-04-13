import { ZigApiError } from "./errors.js";
import { openSseStream, SseOptions, SseStreamHandle } from "./sse.js";
import type {
  ChatEvent,
  CreateParams,
  CreateRequest,
  HealthResponse,
  LoginRequest,
  LoginResponse,
  LogoutResponse,
  RunRequest,
  RunResponse,
  SendMessageResponse,
  SessionDetail,
  SessionLogEvent,
  SessionLogIndexEntry,
  StartChatResponse,
  TopicContent,
  TopicEntry,
  UserAddRequest,
  UserListEntry,
  UserPasswdRequest,
  UserRemoveRequest,
  UserResponse,
  ValidateResponse,
  Workflow,
  WorkflowInfo,
} from "./types.js";

/**
 * Ambient `fetch` type. Declared locally so the package compiles cleanly in
 * environments that don't ship a `lib.dom` — Node 18+ exposes a compatible
 * global `fetch` we pick up at runtime.
 */
export type FetchLike = (
  input: string,
  init?: {
    method?: string;
    headers?: Record<string, string>;
    body?: string;
    signal?: unknown;
  },
) => Promise<Response>;

export interface ZigApiClientOptions {
  /** Base URL of the zig-serve instance, e.g. `http://localhost:3000`. */
  baseUrl: string;
  /** Bearer token used in the `Authorization` header for every request. */
  token?: string;
  /** Override the global `fetch`. */
  fetch?: FetchLike;
  /**
   * Default SSE options (e.g. an injected `EventSourceImpl` polyfill for Node).
   * Can be overridden per-call on the streaming methods.
   */
  sse?: SseOptions;
}

/**
 * HTTP client for zig-serve. Each method maps to a single REST endpoint and
 * throws a [`ZigApiError`] on non-2xx responses.
 *
 * Example:
 * ```ts
 * const client = new ZigApiClient({
 *   baseUrl: "http://localhost:3000",
 *   token: "my-token",
 * });
 * const { session_id } = await client.startChat({ initial_prompt: "build a CI workflow" });
 * const stream = client.streamChat(session_id, (evt) => console.log(evt.role, evt.text));
 * await client.sendChatMessage(session_id, "use cargo test");
 * stream.close();
 * ```
 */
export class ZigApiClient {
  readonly baseUrl: string;
  private token: string | undefined;
  private readonly fetchImpl: FetchLike;
  private readonly sseDefaults: SseOptions;

  constructor(options: ZigApiClientOptions) {
    this.baseUrl = options.baseUrl.replace(/\/+$/, "");
    this.token = options.token;
    const fetchRef =
      options.fetch ?? (globalThis as { fetch?: FetchLike }).fetch;
    if (!fetchRef) {
      throw new Error(
        "No `fetch` available — pass `options.fetch` (e.g. node-fetch) or run on Node 18+ / a browser.",
      );
    }
    this.fetchImpl = fetchRef;
    this.sseDefaults = options.sse ?? {};
  }

  /** Update the bearer token used for all subsequent requests. */
  setToken(token: string | undefined): void {
    this.token = token;
  }

  /** Return the currently active bearer token, if any. */
  getToken(): string | undefined {
    return this.token;
  }

  // -- Low-level ------------------------------------------------------------

  private async request<T>(
    method: string,
    path: string,
    body?: unknown,
  ): Promise<T> {
    const headers: Record<string, string> = {
      accept: "application/json",
    };
    if (body !== undefined) {
      headers["content-type"] = "application/json";
    }
    if (this.token) {
      headers.authorization = `Bearer ${this.token}`;
    }

    const res = await this.fetchImpl(`${this.baseUrl}${path}`, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
    });

    if (!res.ok) {
      throw await ZigApiError.fromResponse(res);
    }

    // 204 No Content
    if (res.status === 204) return undefined as T;

    const text = await res.text();
    if (!text) return undefined as T;
    return JSON.parse(text) as T;
  }

  private authQuery(): string {
    return this.token ? `?token=${encodeURIComponent(this.token)}` : "";
  }

  // -- Health ---------------------------------------------------------------

  health(): Promise<HealthResponse> {
    return this.request<HealthResponse>("GET", "/api/v1/health");
  }

  // -- Auth -----------------------------------------------------------------

  async login(req: LoginRequest): Promise<LoginResponse> {
    const res = await this.request<LoginResponse>(
      "POST",
      "/api/v1/login",
      req,
    );
    // Auto-promote the session token so subsequent calls authenticate.
    this.token = res.token;
    return res;
  }

  logout(): Promise<LogoutResponse> {
    return this.request<LogoutResponse>("POST", "/api/v1/logout");
  }

  // -- Workflows ------------------------------------------------------------

  listWorkflows(): Promise<WorkflowInfo[]> {
    return this.request<WorkflowInfo[]>("GET", "/api/v1/workflows");
  }

  showWorkflow(name: string): Promise<Workflow> {
    return this.request<Workflow>(
      "GET",
      `/api/v1/workflows/${encodeURIComponent(name)}`,
    );
  }

  deleteWorkflow(name: string): Promise<void> {
    return this.request<void>(
      "DELETE",
      `/api/v1/workflows/${encodeURIComponent(name)}`,
    );
  }

  validateWorkflow(content: string): Promise<ValidateResponse> {
    return this.request<ValidateResponse>(
      "POST",
      "/api/v1/workflows/validate",
      { content },
    );
  }

  runWorkflow(req: RunRequest): Promise<RunResponse> {
    return this.request<RunResponse>("POST", "/api/v1/workflows/run", req);
  }

  createWorkflow(req: CreateRequest = {}): Promise<CreateParams> {
    return this.request<CreateParams>(
      "POST",
      "/api/v1/workflows/create",
      req,
    );
  }

  // -- Sessions -------------------------------------------------------------

  listSessions(): Promise<SessionLogIndexEntry[]> {
    return this.request<SessionLogIndexEntry[]>("GET", "/api/v1/sessions");
  }

  sessionDetail(id: string): Promise<SessionDetail> {
    return this.request<SessionDetail>(
      "GET",
      `/api/v1/sessions/${encodeURIComponent(id)}`,
    );
  }

  /**
   * Subscribe to a session's live event log via Server-Sent Events. Returns a
   * handle whose `close()` tears down the underlying connection.
   */
  streamSession(
    id: string,
    onEvent: (event: SessionLogEvent) => void,
    options: SseOptions = {},
  ): SseStreamHandle {
    const url = `${this.baseUrl}/api/v1/sessions/${encodeURIComponent(id)}/events/stream`;
    return openSseStream<SessionLogEvent>(url, onEvent, {
      ...this.sseDefaults,
      ...options,
    });
  }

  // -- Manpages -------------------------------------------------------------

  listManpages(): Promise<TopicEntry[]> {
    return this.request<TopicEntry[]>("GET", "/api/v1/man");
  }

  showManpage(topic: string): Promise<TopicContent> {
    return this.request<TopicContent>(
      "GET",
      `/api/v1/man/${encodeURIComponent(topic)}`,
    );
  }

  // -- Users ----------------------------------------------------------------

  listUsers(): Promise<UserListEntry[]> {
    return this.request<UserListEntry[]>("GET", "/api/v1/users");
  }

  addUser(req: UserAddRequest): Promise<UserResponse> {
    return this.request<UserResponse>("POST", "/api/v1/users/add", req);
  }

  removeUser(req: UserRemoveRequest): Promise<UserResponse> {
    return this.request<UserResponse>("POST", "/api/v1/users/remove", req);
  }

  changeUserPassword(req: UserPasswdRequest): Promise<UserResponse> {
    return this.request<UserResponse>("POST", "/api/v1/users/passwd", req);
  }

  // -- Web chat (requires `zig serve --web`) --------------------------------

  startChat(initialPrompt: string, name?: string): Promise<StartChatResponse> {
    return this.request<StartChatResponse>("POST", "/api/v1/web/chat", {
      initial_prompt: initialPrompt,
      name,
    });
  }

  sendChatMessage(
    sessionId: string,
    message: string,
  ): Promise<SendMessageResponse> {
    return this.request<SendMessageResponse>(
      "POST",
      `/api/v1/web/chat/${encodeURIComponent(sessionId)}`,
      { message },
    );
  }

  /**
   * Subscribe to a web-chat session's SSE event stream. Because `EventSource`
   * cannot set request headers, the token is passed as a `?token=` query
   * parameter — `zig-serve`'s auth middleware accepts that for this endpoint.
   */
  streamChat(
    sessionId: string,
    onEvent: (event: ChatEvent) => void,
    options: SseOptions = {},
  ): SseStreamHandle {
    const url = `${this.baseUrl}/api/v1/web/chat/${encodeURIComponent(sessionId)}/stream${this.authQuery()}`;
    return openSseStream<ChatEvent>(url, onEvent, {
      ...this.sseDefaults,
      ...options,
    });
  }
}
