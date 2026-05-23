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
      "source_id": "macbook-vuong-a1b2c3d4:8a9ab86c464f0dd11965d22b835e74f22f29bbed363576ce9f8161d7adb2f5a6",
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
<machine_id>:<sha256(machine_id + local_id + row_content)>
```

The row content includes timestamp, original command, RTK command, token counts, savings percentage, execution time, and project path. This makes retries safe when an upload succeeds on the server but the client fails before recording the checkpoint, while tying the idempotency key to the uploaded row content.
