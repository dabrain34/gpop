# gpop-rs

GStreamer Prince of Parser - A pipeline management daemon with WebSocket and DBus interfaces.

## Table of Contents

- [Overview](#overview)
- [Features](#features)
- [Building](#building)
  - [With Cargo (standalone)](#with-cargo-standalone)
  - [With Meson (full project)](#with-meson-full-project)
- [Running the Server](#running-the-server)
  - [Command Line Options](#command-line-options)
  - [Environment Variables](#environment-variables)
- [Authentication](#authentication)
  - [Authentication Responses](#authentication-responses)
  - [Client Examples](#client-examples)
- [WebSocket API](#websocket-api)
  - [Protocol](#protocol)
  - [Methods](#methods)
  - [Events](#events)
  - [Error Codes](#error-codes)
- [Example Client](#example-client)
  - [Client Commands](#client-commands)
  - [Example Session](#example-session)
- [DBus Interface (Linux only)](#dbus-interface-linux-only)
  - [Manager Interface](#manager-interface)
  - [Pipeline Interface](#pipeline-interface)
  - [DBus Example](#dbus-example)
- [License](#license)

## Overview

`gpop-rs` is a Rust implementation of a GStreamer pipeline manager that allows you to create, control, and monitor GStreamer pipelines through WebSocket and DBus interfaces.

## Features

- **WebSocket API**: JSON-RPC 2.0 based protocol for pipeline management
- **DBus Interface** (Linux only): Native DBus integration for desktop applications
- **Real-time Events**: Receive pipeline state changes, errors, EOS, and lifecycle notifications
- **Pipeline Introspection**: Get DOT graph representations of pipelines

## Building

### With Cargo (standalone)

```bash
cd daemon
cargo build --release
```

The binary will be at `target/release/gpop-daemon`.

### With Meson (full project)

From the project root:

```bash
meson setup builddir
ninja -C builddir
```

The binary will be at `builddir/release/gpop-daemon`.

To build only the daemon (without clients):

```bash
meson setup builddir -Dclient=false -Dc_client=false
ninja -C builddir
```

## Running the Server

```bash
# Default: bind to 127.0.0.1:9000
gpop-daemon

# Custom bind address and port
gpop-daemon --bind 0.0.0.0 --port 8080

# With initial pipeline
gpop-daemon -P "videotestsrc ! autovideosink"

# With authentication
gpop-daemon --api-key mysecretkey

# Enable debug logging
RUST_LOG=debug gpop-daemon
```

### Command Line Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--bind` | `-b` | `127.0.0.1` | IP address to bind to |
| `--port` | `-p` | `9000` | Port to listen on |
| `--pipeline` | `-P` | - | Initial pipeline(s) to create |
| `--api-key` | - | - | API key for WebSocket authentication |
| `--no-websocket` | - | - | Disable WebSocket interface |
| `--no-dbus` | - | - | Disable DBus interface (Linux only) |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `GPOP_API_KEY` | API key for WebSocket authentication (alternative to `--api-key`) |
| `RUST_LOG` | Log level (e.g., `debug`, `info`, `warn`, `error`) |

## Authentication

By default, the WebSocket server accepts connections from any client that can reach it. When binding to `127.0.0.1` (the default), only local processes can connect.

For deployments where the server is exposed on a network or in multi-user environments, you can enable API key authentication:

```bash
# Via command line
gpop-daemon --api-key mysecretkey

# Via environment variable
GPOP_API_KEY=mysecretkey gpop-daemon

# Combined with network binding
gpop-daemon --bind 0.0.0.0 --api-key mysecretkey
```

When authentication is enabled, clients must include the API key in the `Authorization` header during the WebSocket handshake:

```
Authorization: mysecretkey
```

### Authentication Responses

| Scenario | HTTP Status |
|----------|-------------|
| No `--api-key` configured | Connection accepted (no auth required) |
| Correct API key provided | Connection accepted |
| Missing `Authorization` header | `401 Unauthorized` |
| Invalid API key | `403 Forbidden` |

### Client Examples

**JavaScript (Browser/Node.js):**
```javascript
const ws = new WebSocket('ws://localhost:9000', {
  headers: {
    'Authorization': 'mysecretkey'
  }
});
```

**Python (websockets library):**
```python
import websockets

async def connect():
    async with websockets.connect(
        'ws://localhost:9000',
        extra_headers={'Authorization': 'mysecretkey'}
    ) as ws:
        await ws.send('{"id":"1","method":"list_pipelines","params":{}}')
        print(await ws.recv())
```

**Rust (tokio-tungstenite):**
```rust
use tokio_tungstenite::tungstenite::http::Request;

let request = Request::builder()
    .uri("ws://localhost:9000")
    .header("Authorization", "mysecretkey")
    .body(())?;

let (ws_stream, _) = connect_async(request).await?;
```

**curl (for testing handshake):**
```bash
# This will fail with 401 if auth is enabled and no key provided
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Authorization: mysecretkey" \
  http://localhost:9000/
```

## WebSocket API

Connect to `ws://<host>:<port>` to interact with the server.

### Protocol

All messages use JSON-RPC 2.0 format:

**Request:**
```json
{
  "id": "unique-request-id",
  "method": "method_name",
  "params": { ... }
}
```

**Success Response:**
```json
{
  "id": "unique-request-id",
  "result": { ... }
}
```

**Error Response:**
```json
{
  "id": "unique-request-id",
  "error": {
    "code": -32000,
    "message": "Error description"
  }
}
```

### Methods

#### `list_pipelines`

List all managed pipelines.

**Request:**
```json
{
  "id": "1",
  "method": "list_pipelines",
  "params": {}
}
```

**Response:**
```json
{
  "id": "1",
  "result": {
    "pipelines": [
      {
        "id": "0",
        "description": "videotestsrc ! autovideosink",
        "state": "playing",
        "streaming": true
      }
    ]
  }
}
```

#### `create_pipeline`

Create a new pipeline from a GStreamer pipeline description.

**Request:**
```json
{
  "id": "2",
  "method": "create_pipeline",
  "params": {
    "description": "videotestsrc ! autovideosink"
  }
}
```

**Response:**
```json
{
  "id": "2",
  "result": {
    "pipeline_id": "0"
  }
}
```

#### `remove_pipeline`

Remove and destroy a pipeline.

**Request:**
```json
{
  "id": "3",
  "method": "remove_pipeline",
  "params": {
    "pipeline_id": "0"
  }
}
```

**Response:**
```json
{
  "id": "3",
  "result": {}
}
```

#### `get_pipeline_info`

Get information about a specific pipeline.

**Request:**
```json
{
  "id": "4",
  "method": "get_pipeline_info",
  "params": {
    "pipeline_id": "0"
  }
}
```

**Response:**
```json
{
  "id": "4",
  "result": {
    "id": "0",
    "description": "videotestsrc ! autovideosink",
    "state": "playing",
    "streaming": true
  }
}
```

#### `set_state`

Set the pipeline state.

**Request:**
```json
{
  "id": "5",
  "method": "set_state",
  "params": {
    "pipeline_id": "0",
    "state": "playing"
  }
}
```

Valid states: `null`, `ready`, `paused`, `playing`

**Response:**
```json
{
  "id": "5",
  "result": {
    "success": true
  }
}
```

#### `play`, `pause`, `stop`

Convenience methods for state changes. The `pipeline_id` parameter is optional and defaults to `"0"`.

**Request:**
```json
{
  "id": "6",
  "method": "play",
  "params": {}
}
```

Or with explicit pipeline ID:
```json
{
  "id": "6",
  "method": "play",
  "params": {
    "pipeline_id": "0"
  }
}
```

#### `update_pipeline`

Update an existing pipeline with a new description. The pipeline is stopped and replaced atomically.

**Request:**
```json
{
  "id": "7",
  "method": "update_pipeline",
  "params": {
    "pipeline_id": "0",
    "description": "videotestsrc pattern=ball ! autovideosink"
  }
}
```

**Response:**
```json
{
  "id": "7",
  "result": {
    "success": true
  }
}
```

#### `get_position`

Get the current position and duration of a pipeline. The `pipeline_id` parameter is optional and defaults to `"0"`.

**Request:**
```json
{
  "id": "8",
  "method": "get_position",
  "params": {}
}
```

**Response:**
```json
{
  "id": "8",
  "result": {
    "position_ns": 1500000000,
    "duration_ns": 10000000000,
    "progress": 0.15
  }
}
```

Note: `position_ns` and `duration_ns` are in nanoseconds. `progress` is a value between 0.0 and 1.0. Any of these fields may be `null` if not available (e.g., for live streams).

#### `get_version`

Get the daemon version.

**Request:**
```json
{
  "id": "9",
  "method": "get_version",
  "params": {}
}
```

**Response:**
```json
{
  "id": "9",
  "result": {
    "version": "1.0.0"
  }
}
```

#### `get_pipeline_count`

Get the number of managed pipelines.

**Request:**
```json
{
  "id": "10",
  "method": "get_pipeline_count",
  "params": {}
}
```

**Response:**
```json
{
  "id": "10",
  "result": {
    "count": 3
  }
}
```

#### `snapshot`

Get the DOT graph representation of a pipeline. The `pipeline_id` parameter is optional and defaults to `"0"`.

**Request:**
```json
{
  "id": "7",
  "method": "snapshot",
  "params": {}
}
```

Or with explicit pipeline ID and detail level:
```json
{
  "id": "7",
  "method": "snapshot",
  "params": {
    "pipeline_id": "0",
    "details": "all"
  }
}
```

Valid detail levels: `media`, `caps`, `non-default`, `states`, `all` (default)

**Response:**
```json
{
  "id": "7",
  "result": {
    "dot": "digraph pipeline { ... }"
  }
}
```

### Events

The server broadcasts events to all connected clients:

#### `state_changed`
```json
{
  "event": "state_changed",
  "data": {
    "pipeline_id": "0",
    "old_state": "paused",
    "new_state": "playing"
  }
}
```

#### `error`
```json
{
  "event": "error",
  "data": {
    "pipeline_id": "0",
    "message": "Error description"
  }
}
```

#### `eos`
```json
{
  "event": "eos",
  "data": {
    "pipeline_id": "0"
  }
}
```

#### `pipeline_added`
```json
{
  "event": "pipeline_added",
  "data": {
    "pipeline_id": "0",
    "description": "videotestsrc ! autovideosink"
  }
}
```

#### `pipeline_updated`
```json
{
  "event": "pipeline_updated",
  "data": {
    "pipeline_id": "0",
    "description": "videotestsrc ! autovideosink"
  }
}
```

#### `pipeline_removed`
```json
{
  "event": "pipeline_removed",
  "data": {
    "pipeline_id": "0"
  }
}
```

### Error Codes

| Code | Description |
|------|-------------|
| `-32700` | Parse error - Invalid JSON |
| `-32601` | Method not found |
| `-32602` | Invalid params |
| `-32603` | Internal error |
| `-32000` | Pipeline not found |
| `-32001` | Pipeline creation failed |
| `-32002` | State change failed |
| `-32003` | GStreamer error |

## Example Client

An interactive WebSocket client is included:

```bash
cargo run --example ws_client

# Or connect to a different server
cargo run --example ws_client -- ws://192.168.1.100:9000
```

### Client Commands

```
list                        - List all pipelines
create <description>        - Create a new pipeline
update <id> <description>   - Update pipeline description
remove <id>                 - Remove a pipeline
info <id>                   - Get pipeline info
play [id]                   - Play a pipeline (default: 0)
pause [id]                  - Pause a pipeline (default: 0)
stop [id]                   - Stop a pipeline (default: 0)
state <id> <state>          - Set pipeline state
position [id]               - Get pipeline position/duration (default: 0)
snapshot [id] [details]     - Get DOT graph (default: 0)
quit                        - Exit
```

### Example Session

```
$ cargo run --example ws_client
Connecting to ws://127.0.0.1:9000...
Connected!

> create videotestsrc ! autovideosink
Sending: {"id":"...","method":"create_pipeline","params":{"description":"videotestsrc ! autovideosink"}}

[RESPONSE] id=...: {
  "pipeline_id": "0"
}

> play
Sending: {"id":"...","method":"play","params":{}}

[EVENT] state_changed: {"new_state":"ready","old_state":"null","pipeline_id":"0"}
[EVENT] state_changed: {"new_state":"paused","old_state":"ready","pipeline_id":"0"}
[RESPONSE] id=...: {
  "success": true
}
[EVENT] state_changed: {"new_state":"playing","old_state":"paused","pipeline_id":"0"}

> snapshot
Sending: {"id":"...","method":"snapshot","params":{}}

[RESPONSE] id=...: {
  "dot": "digraph pipeline { ... }"
}

> list
Sending: {"id":"...","method":"list_pipelines","params":{}}

[RESPONSE] id=...: {
  "pipelines": [
    {
      "description": "videotestsrc ! autovideosink",
      "id": "0",
      "state": "playing",
      "streaming": true
    }
  ]
}

> stop
> remove 0
> quit
Goodbye!
```

## DBus Interface (Linux only)

On Linux, gpop-rs also exposes a DBus interface on the session bus.

### Service Name

`org.gpop`

### Manager Interface

**Path:** `/org/gpop/Manager`
**Interface:** `org.gpop.Manager`

#### Methods

- `AddPipeline(description: string) -> string` - Create a pipeline, returns ID
- `RemovePipeline(id: string)` - Remove a pipeline
- `GetPipelineDesc(id: string) -> string` - Get pipeline description
- `UpdatePipeline(id: string, description: string)` - Update pipeline description

#### Properties

- `Pipelines: u32` - Number of active pipelines
- `Version: string` - Server version

#### Signals

- `PipelineAdded(id: string, description: string)`
- `PipelineRemoved(id: string)`

### Pipeline Interface

**Path:** `/org/gpop/Pipeline{N}` (e.g., `/org/gpop/Pipeline0`)
**Interface:** `org.gpop.Pipeline`

#### Methods

- `SetState(state: string) -> bool`
- `Play() -> bool`
- `Pause() -> bool`
- `Stop() -> bool`

#### Properties

- `Id: string` - Pipeline ID
- `Description: string` - Pipeline description
- `State: string` - Current state
- `Streaming: bool` - Whether pipeline is streaming

#### Signals

- `StateChanged(old_state: string, new_state: string)`
- `Error(message: string)`
- `Eos()`

### DBus Example

```bash
# List pipelines count
dbus-send --session --print-reply --dest=org.gpop \
  /org/gpop/Manager org.freedesktop.DBus.Properties.Get \
  string:org.gpop.Manager string:Pipelines

# Create a pipeline
dbus-send --session --print-reply --dest=org.gpop \
  /org/gpop/Manager org.gpop.Manager.AddPipeline \
  string:"videotestsrc ! fakesink"

# Play a pipeline
dbus-send --session --print-reply --dest=org.gpop \
  /org/gpop/Pipeline0 org.gpop.Pipeline.Play
```

## License

GPL-3.0-only
