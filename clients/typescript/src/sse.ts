// SSE stream helper shared by chat and session event endpoints.
//
// The browser provides a global `EventSource`. Node 18+ does NOT ship one by
// default, so in Node we expect callers to inject a polyfill (e.g. the
// `eventsource` package). The injection point is `SseOptions.EventSourceImpl`.

export type EventSourceCtor = new (
  url: string,
  init?: { withCredentials?: boolean },
) => EventSourceLike;

/**
 * Structural subset of the DOM `EventSource` interface. Declared locally so
 * this module compiles in Node environments where `lib.dom` isn't present.
 */
export interface EventSourceLike {
  onmessage: ((ev: { data: string }) => void) | null;
  onerror: ((ev: unknown) => void) | null;
  close(): void;
}

export interface SseOptions {
  /**
   * Explicit `EventSource` implementation. Defaults to `globalThis.EventSource`
   * when available — supply a polyfill in Node environments.
   */
  EventSourceImpl?: EventSourceCtor;
  /** Called when the underlying connection errors. */
  onError?: (ev: unknown) => void;
}

export interface SseStreamHandle {
  /** Closes the underlying `EventSource`. */
  close(): void;
}

/**
 * Opens an SSE connection to `url` and parses each event's `data` payload as
 * JSON into `T`. Non-JSON lines are forwarded as `unknown` so callers can
 * decide how to handle them.
 *
 * The returned handle has a single `close()` method that tears down the
 * underlying `EventSource`.
 */
export function openSseStream<T = unknown>(
  url: string,
  onEvent: (event: T) => void,
  options: SseOptions = {},
): SseStreamHandle {
  const Impl =
    options.EventSourceImpl ??
    (globalThis as { EventSource?: EventSourceCtor }).EventSource;

  if (!Impl) {
    throw new Error(
      "No EventSource implementation available. In Node, pass `EventSourceImpl` " +
        "from the `eventsource` npm package; in browsers it should be defined globally.",
    );
  }

  const es = new Impl(url);
  es.onmessage = (ev) => {
    try {
      onEvent(JSON.parse(ev.data) as T);
    } catch {
      onEvent(ev.data as unknown as T);
    }
  };
  if (options.onError) es.onerror = options.onError;

  return { close: () => es.close() };
}
