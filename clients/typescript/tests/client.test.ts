import { strict as assert } from "node:assert";
import { test } from "node:test";

import { ZigApiClient } from "../src/client.js";
import { ZigApiError } from "../src/errors.js";

/**
 * Minimal fake of the global `fetch` Response class — enough surface for the
 * client's `request()` helper. Records each call so tests can assert on the
 * request URL, method, headers, and body.
 */
interface FakeCall {
  url: string;
  method: string;
  headers: Record<string, string>;
  body?: string;
}

interface FakeResponseSpec {
  status?: number;
  statusText?: string;
  body?: string;
}

function makeFakeFetch(calls: FakeCall[], responses: FakeResponseSpec[]) {
  return async (
    url: string,
    init?: {
      method?: string;
      headers?: Record<string, string>;
      body?: string;
    },
  ): Promise<Response> => {
    calls.push({
      url,
      method: init?.method ?? "GET",
      headers: init?.headers ?? {},
      body: init?.body,
    });
    const spec = responses.shift() ?? { status: 200, body: "{}" };
    const status = spec.status ?? 200;
    const statusText = spec.statusText ?? "OK";
    const bodyText = spec.body ?? "";
    return new Response(bodyText, {
      status,
      statusText,
      headers: { "content-type": "application/json" },
    });
  };
}

test("health() hits /api/v1/health with bearer token", async () => {
  const calls: FakeCall[] = [];
  const client = new ZigApiClient({
    baseUrl: "http://zig.test",
    token: "secret",
    fetch: makeFakeFetch(calls, [
      { body: '{"status":"ok","version":"0.5.7"}' },
    ]),
  });

  const res = await client.health();
  assert.deepEqual(res, { status: "ok", version: "0.5.7" });

  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, "http://zig.test/api/v1/health");
  assert.equal(calls[0].method, "GET");
  assert.equal(calls[0].headers.authorization, "Bearer secret");
});

test("baseUrl trailing slashes are trimmed", async () => {
  const calls: FakeCall[] = [];
  const client = new ZigApiClient({
    baseUrl: "http://zig.test///",
    fetch: makeFakeFetch(calls, [
      { body: '{"status":"ok","version":"0.5.7"}' },
    ]),
  });

  await client.health();
  assert.equal(calls[0].url, "http://zig.test/api/v1/health");
});

test("startChat posts JSON with initial_prompt", async () => {
  const calls: FakeCall[] = [];
  const client = new ZigApiClient({
    baseUrl: "http://zig.test",
    token: "t",
    fetch: makeFakeFetch(calls, [
      { body: '{"session_id":"abc","output_path":"/tmp/x.zwf"}' },
    ]),
  });

  const res = await client.startChat("build a CI workflow", "ci");
  assert.deepEqual(res, { session_id: "abc", output_path: "/tmp/x.zwf" });

  assert.equal(calls[0].url, "http://zig.test/api/v1/web/chat");
  assert.equal(calls[0].method, "POST");
  assert.equal(calls[0].headers["content-type"], "application/json");
  assert.deepEqual(JSON.parse(calls[0].body!), {
    initial_prompt: "build a CI workflow",
    name: "ci",
  });
});

test("sendChatMessage URL-encodes the session id", async () => {
  const calls: FakeCall[] = [];
  const client = new ZigApiClient({
    baseUrl: "http://zig.test",
    token: "t",
    fetch: makeFakeFetch(calls, [{ body: '{"ok":true}' }]),
  });

  const res = await client.sendChatMessage("id with space", "hello");
  assert.deepEqual(res, { ok: true });
  assert.equal(
    calls[0].url,
    "http://zig.test/api/v1/web/chat/id%20with%20space",
  );
});

test("login() auto-promotes the session token", async () => {
  const calls: FakeCall[] = [];
  const client = new ZigApiClient({
    baseUrl: "http://zig.test",
    fetch: makeFakeFetch(calls, [
      { body: '{"token":"new","username":"alice","home_dir":"/home/alice"}' },
      { body: '{"status":"ok","version":"0.5.7"}' },
    ]),
  });

  const login = await client.login({ username: "alice", password: "pw" });
  assert.equal(login.token, "new");
  assert.equal(client.getToken(), "new");

  await client.health();
  assert.equal(calls[1].headers.authorization, "Bearer new");
});

test("error responses raise ZigApiError with parsed code/message", async () => {
  const calls: FakeCall[] = [];
  const client = new ZigApiClient({
    baseUrl: "http://zig.test",
    token: "t",
    fetch: makeFakeFetch(calls, [
      {
        status: 404,
        statusText: "Not Found",
        body: '{"error":{"code":"not_found","message":"chat session xyz not found"}}',
      },
    ]),
  });

  await assert.rejects(
    () => client.sendChatMessage("xyz", "hi"),
    (err: unknown) => {
      assert.ok(err instanceof ZigApiError);
      assert.equal(err.status, 404);
      assert.equal(err.code, "not_found");
      assert.equal(err.message, "chat session xyz not found");
      return true;
    },
  );
});

test("streamChat appends token as query parameter", () => {
  let capturedUrl = "";
  class FakeEventSource {
    onmessage: ((ev: { data: string }) => void) | null = null;
    onerror: ((ev: unknown) => void) | null = null;
    constructor(url: string) {
      capturedUrl = url;
    }
    close(): void {}
  }

  const client = new ZigApiClient({
    baseUrl: "http://zig.test",
    token: "tok/with+weird=chars",
    fetch: makeFakeFetch([], []),
    sse: { EventSourceImpl: FakeEventSource },
  });

  const handle = client.streamChat("sess-1", () => {});
  assert.equal(
    capturedUrl,
    "http://zig.test/api/v1/web/chat/sess-1/stream?token=tok%2Fwith%2Bweird%3Dchars",
  );
  handle.close();
});

test("streamChat forwards parsed JSON events to the callback", () => {
  const received: unknown[] = [];
  let instance: { onmessage: ((ev: { data: string }) => void) | null } | null =
    null;
  class FakeEventSource {
    onmessage: ((ev: { data: string }) => void) | null = null;
    onerror: ((ev: unknown) => void) | null = null;
    constructor(_: string) {
      instance = this;
    }
    close(): void {}
  }

  const client = new ZigApiClient({
    baseUrl: "http://zig.test",
    token: "t",
    fetch: makeFakeFetch([], []),
    sse: { EventSourceImpl: FakeEventSource },
  });

  client.streamChat("sess-1", (ev) => received.push(ev));
  instance!.onmessage!({ data: '{"role":"agent","text":"hello"}' });
  instance!.onmessage!({ data: '{"role":"user","text":"world"}' });

  assert.deepEqual(received, [
    { role: "agent", text: "hello" },
    { role: "user", text: "world" },
  ]);
});
