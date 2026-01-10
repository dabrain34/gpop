# gpop-rs

GStreamer Prince of Parser - A pipeline management daemon with WebSocket and DBus interfaces.

## Overview

`gpop-rs` is a Rust implementation of a GStreamer pipeline manager that allows you to create, control, and monitor GStreamer pipelines through WebSocket and DBus interfaces.

## Features

- **WebSocket API**: JSON-RPC 2.0 based protocol for pipeline management
- **DBus Interface** (Linux only): Native DBus integration for desktop applications
- **Real-time Events**: Receive pipeline state changes, errors, and EOS notifications
- **Pipeline Introspection**: Get DOT graph representations of pipelines

## Building

```bash
cd gpop-rs
cargo build --release
```

## Running the Server

```bash
# Default: bind to 127.0.0.1:9000
./target/release/gpop-rs

# Custom bind address and port
./target/release/gpop-rs --bind 0.0.0.0 --port 8080

# Enable debug logging
RUST_LOG=debug ./target/release/gpop-rs
```

### Command Line Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--bind` | `-b` | `127.0.0.1` | IP address to bind to |
| `--port` | `-p` | `9000` | Port to listen on |

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
        "id": "pipeline-0",
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
    "pipeline_id": "pipeline-0"
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
    "pipeline_id": "pipeline-0"
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

#### `get_pipeline`

Get information about a specific pipeline.

**Request:**
```json
{
  "id": "4",
  "method": "get_pipeline",
  "params": {
    "pipeline_id": "pipeline-0"
  }
}
```

**Response:**
```json
{
  "id": "4",
  "result": {
    "id": "pipeline-0",
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
    "pipeline_id": "pipeline-0",
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

Convenience methods for state changes.

**Request:**
```json
{
  "id": "6",
  "method": "play",
  "params": {
    "pipeline_id": "pipeline-0"
  }
}
```

#### `get_dot`

Get the DOT graph representation of a pipeline.

**Request:**
```json
{
  "id": "7",
  "method": "get_dot",
  "params": {
    "pipeline_id": "pipeline-0",
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
    "pipeline_id": "pipeline-0",
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
    "pipeline_id": "pipeline-0",
    "message": "Error description"
  }
}
```

#### `eos`
```json
{
  "event": "eos",
  "data": {
    "pipeline_id": "pipeline-0"
  }
}
```

#### `pipeline_added`
```json
{
  "event": "pipeline_added",
  "data": {
    "pipeline_id": "pipeline-0",
    "description": "videotestsrc ! autovideosink"
  }
}
```

#### `pipeline_removed`
```json
{
  "event": "pipeline_removed",
  "data": {
    "pipeline_id": "pipeline-0"
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
list                     - List all pipelines
create <description>     - Create a new pipeline
remove <id>              - Remove a pipeline
info <id>                - Get pipeline info
play <id>                - Play a pipeline
pause <id>               - Pause a pipeline
stop <id>                - Stop a pipeline
state <id> <state>       - Set pipeline state
dot <id> [details]       - Get DOT graph
quit                     - Exit
```

### Example Session

```
$ cargo run --example ws_client
Connecting to ws://127.0.0.1:9000...
Connected!

> create videotestsrc ! autovideosink
Sending: {"id":"...","method":"create_pipeline","params":{"description":"videotestsrc ! autovideosink"}}

[RESPONSE] id=...: {
  "pipeline_id": "pipeline-0"
}

> play pipeline-0
Sending: {"id":"...","method":"play","params":{"pipeline_id":"pipeline-0"}}

[EVENT] state_changed: {"new_state":"ready","old_state":"null","pipeline_id":"pipeline-0"}
[EVENT] state_changed: {"new_state":"paused","old_state":"ready","pipeline_id":"pipeline-0"}
[RESPONSE] id=...: {
  "success": true
}
[EVENT] state_changed: {"new_state":"playing","old_state":"paused","pipeline_id":"pipeline-0"}

> list
Sending: {"id":"...","method":"list_pipelines","params":{}}

[RESPONSE] id=...: {
  "pipelines": [
    {
      "description": "videotestsrc ! autovideosink",
      "id": "pipeline-0",
      "state": "playing",
      "streaming": true
    }
  ]
}

> stop pipeline-0
> remove pipeline-0
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

LGPL-2.1-or-later
