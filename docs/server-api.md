# Server API

`rtk-sync` uploads RTK tracking events to a central HTTP API.

## Request

```http
POST /api/rtk/events
Authorization: Bearer <token>
Content-Type: application/json
```

Request body shape:

```json
{
  "machine_id": "macbook-vuong-a1b2c3d4",
  "events": [
    {
      "source_id": "macbook-vuong-a1b2c3d4:12345",
      "machine_id": "macbook-vuong-a1b2c3d4",
      "local_id": 12345,
      "command": "rtk git status",
      "original_cmd": "git status",
      "input_tokens": 1200,
      "output_tokens": 180,
      "saved_tokens": 1020,
      "savings_pct": 85.0,
      "exec_time_ms": 42,
      "project_path": "/path/to/project",
      "created_at": "2026-05-23T10:30:00Z"
    }
  ]
}
```

## Response

Expected response:

```json
{
  "accepted": 100,
  "duplicates": 3,
  "max_local_id": 12445
}
```

`rtk-sync` updates its local checkpoint to `max_local_id` only after a successful response.

## Idempotency

The server should enforce uniqueness on `source_id`.

`source_id` is generated as:

```text
<machine_id>:<local_row_id>
```

This makes retries safe when an upload succeeds on the server but the client fails before recording the checkpoint.
