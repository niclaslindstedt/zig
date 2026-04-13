// Wire-format TypeScript types for the zig-serve REST API.
//
// These mirror the `#[derive(Serialize/Deserialize)]` structs under
// `zig-serve/src/handlers/*` and `zig-serve/src/types.rs`. Keep them in sync
// by adjusting both sides together when endpoints change.

// ---------- Health ----------

export interface HealthResponse {
  status: "ok";
  version: string;
}

// ---------- Auth ----------

export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponse {
  token: string;
  username: string;
  home_dir: string;
}

export interface LogoutResponse {
  message: string;
}

// ---------- Workflows ----------

/**
 * Listing entry returned by `GET /api/v1/workflows`. Mirrors
 * `zig_core::manage::WorkflowInfo` — additional fields may appear over time,
 * so callers should tolerate extras.
 */
export interface WorkflowInfo {
  name: string;
  path: string;
  description?: string | null;
  tier?: string | null;
  [key: string]: unknown;
}

/**
 * Full workflow returned by `GET /api/v1/workflows/{name}`. The structure
 * matches `zig_core::workflow::model::Workflow`; it's complex and versioned,
 * so we expose it as an opaque record by default.
 */
export type Workflow = Record<string, unknown>;

export interface ValidateRequest {
  content: string;
}

export interface ValidateResponse {
  valid: boolean;
  errors?: string[];
  name?: string;
  step_count?: number;
}

export interface RunRequest {
  workflow: string;
  prompt?: string;
}

export interface RunResponse {
  zig_session_id: string;
}

export interface CreateRequest {
  name?: string;
  output?: string;
  pattern?: string;
}

/** Response body of `POST /api/v1/workflows/create` — matches `CreateParams`. */
export interface CreateParams {
  system_prompt: string;
  initial_prompt: string;
  output_path: string;
  session_name: string;
  session_tag: string;
}

// ---------- Sessions ----------

export interface SessionLogIndexEntry {
  zig_session_id: string;
  zag_session_id?: string | null;
  workflow?: string | null;
  started_at?: string | null;
  ended_at?: string | null;
  log_path: string;
  [key: string]: unknown;
}

/**
 * Event line emitted into a session log. The log format is JSONL and the
 * `type` tag discriminates variants such as `zig_session_started`,
 * `zig_session_ended`, `step_started`, etc. We expose it as an opaque record
 * so consumers can switch on whatever fields they need.
 */
export type SessionLogEvent = Record<string, unknown>;

export interface SessionDetail extends SessionLogIndexEntry {
  events: SessionLogEvent[];
}

// ---------- Manpages ----------

export interface TopicEntry {
  topic: string;
  description: string;
}

export interface TopicContent {
  topic: string;
  content: string;
}

// ---------- Users ----------

export interface UserListEntry {
  username: string;
  home_dir: string;
  enabled: boolean;
  created_at: string;
}

export interface UserAddRequest {
  username: string;
  password: string;
  home_dir: string;
}

export interface UserRemoveRequest {
  username: string;
}

export interface UserPasswdRequest {
  username: string;
  password: string;
}

export interface UserResponse {
  message: string;
}

// ---------- Web chat (only when zig serve --web) ----------

export interface StartChatRequest {
  initial_prompt: string;
  name?: string;
}

export interface StartChatResponse {
  session_id: string;
  output_path: string;
}

export interface SendMessageRequest {
  message: string;
}

export interface SendMessageResponse {
  ok: boolean;
}

/** Event emitted by the chat SSE stream. */
export interface ChatEvent {
  role: "user" | "agent" | "system";
  text: string;
}

// ---------- Errors ----------

/**
 * Error body returned by the server. All handlers use a consistent envelope
 * via `ServeError::into_response`.
 */
export interface ServerErrorBody {
  error: {
    code: string;
    message: string;
  };
}
