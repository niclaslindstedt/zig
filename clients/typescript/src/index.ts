export { ZigApiClient } from "./client.js";
export type { FetchLike, ZigApiClientOptions } from "./client.js";

export { ZigApiError } from "./errors.js";

export { openSseStream } from "./sse.js";
export type {
  EventSourceCtor,
  EventSourceLike,
  SseOptions,
  SseStreamHandle,
} from "./sse.js";

export type {
  // Health
  HealthResponse,
  // Auth
  LoginRequest,
  LoginResponse,
  LogoutResponse,
  // Workflows
  WorkflowInfo,
  Workflow,
  ValidateRequest,
  ValidateResponse,
  RunRequest,
  RunResponse,
  CreateRequest,
  CreateParams,
  // Sessions
  SessionLogIndexEntry,
  SessionLogEvent,
  SessionDetail,
  // Manpages
  TopicEntry,
  TopicContent,
  // Users
  UserListEntry,
  UserAddRequest,
  UserRemoveRequest,
  UserPasswdRequest,
  UserResponse,
  // Web chat
  StartChatRequest,
  StartChatResponse,
  SendMessageRequest,
  SendMessageResponse,
  ChatEvent,
  // Errors
  ServerErrorBody,
} from "./types.js";
