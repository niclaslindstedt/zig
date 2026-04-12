# zig serve

Start an HTTP API server that exposes zig's workflow orchestration as a REST API.

## Synopsis

```
zig serve [--port <PORT>] [--host <HOST>] [--token <TOKEN>]
```

## Description

`zig serve` starts an HTTP API server that provides programmatic access to zig's
workflow management and execution capabilities. This enables building frontends
(such as React web applications) that interact with zig remotely or locally.

The server exposes endpoints under `/api/v1/` for listing, inspecting, validating,
running, and creating workflows, as well as viewing session logs and manpages.

For interactive agent sessions (e.g., workflow creation or interactive workflow
steps), the frontend connects directly to a running `zag serve` instance using
the `zag_session_id` values returned by zig's API.

## Options

- `--port, -p <PORT>` — Port to listen on (default: `3000`)
- `--host <HOST>` — Host/IP to bind to (default: `127.0.0.1`)
- `--token <TOKEN>` — Bearer token for authentication. Can also be set via the
  `ZIG_SERVE_TOKEN` environment variable. If neither is provided, a random token
  is generated and printed to stderr on startup.

## Authentication

All endpoints except `/api/v1/health` require a bearer token in the
`Authorization` header:

```
Authorization: Bearer <token>
```

## API Endpoints

### Health (no auth required)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/health` | Server status and version |

### Workflows

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/workflows` | List all discovered workflows |
| GET | `/api/v1/workflows/{name}` | Get workflow details as JSON |
| DELETE | `/api/v1/workflows/{name}` | Delete a workflow file |
| POST | `/api/v1/workflows/validate` | Validate workflow content |
| POST | `/api/v1/workflows/run` | Run a workflow (returns session ID) |
| POST | `/api/v1/workflows/create` | Prepare workflow creation prompts |

### Sessions

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/sessions` | List zig sessions |
| GET | `/api/v1/sessions/{id}` | Get session events |
| GET | `/api/v1/sessions/{id}/stream` | WebSocket live event stream |

### Manpages

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/man` | List available manpage topics |
| GET | `/api/v1/man/{topic}` | Get manpage content |

## Examples

Start with default settings (auto-generated token):

```
zig serve
```

Start on a specific port with a fixed token:

```
zig serve --port 8080 --token my-secret-token
```

Use with environment variable:

```
ZIG_SERVE_TOKEN=my-secret-token zig serve
```

Query the health endpoint:

```
curl http://localhost:3000/api/v1/health
```

List workflows (authenticated):

```
curl -H "Authorization: Bearer <token>" http://localhost:3000/api/v1/workflows
```

Run a workflow:

```
curl -X POST -H "Authorization: Bearer <token>" \
     -H "Content-Type: application/json" \
     -d '{"workflow": "my-workflow", "prompt": "extra context"}' \
     http://localhost:3000/api/v1/workflows/run
```

## Architecture

The zig API server is designed to work alongside `zag serve`:

- **zig serve** handles workflow orchestration (list, run, validate, create workflows)
- **zag serve** handles agent interactions (streaming output, sending input)

When zig runs a workflow step, it spawns a zag session. The zig session events
include `zag_session_id` fields that a frontend can use to connect directly to
`zag serve` for real-time agent interaction.
