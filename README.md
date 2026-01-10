### Description

**gpop** (GstPrinceOfParser) is a GStreamer pipeline management system with WebSocket and DBus interfaces.

### Project Structure

```
GstPrinceOfParser/
├── daemon/           # Rust server (WebSocket + DBus)
├── client/
│   ├── rust/         # Rust WebSocket client
│   └── c/            # C client
├── lib/              # C library (libgpop)
├── Cargo.toml        # Rust workspace
└── meson.build       # Build system (C + Rust)
```

### Dependencies

#### Linux (Debian/Ubuntu)

```bash
sudo apt install meson ninja-build rustc cargo \
  libglib2.0-dev libgstreamer1.0-dev \
  libsoup-3.0-dev libjson-glib-dev libreadline-dev
```

#### Linux (Fedora)

```bash
sudo dnf install meson ninja-build rust cargo \
  glib2-devel gstreamer1-devel \
  libsoup3-devel json-glib-devel readline-devel
```

### Build

```
meson setup builddir
ninja -C builddir
```

This builds everything:
- Rust daemon and client → `builddir/release/`
- C library → `builddir/lib/`

### Usage

#### Running the Daemon

Start the WebSocket server:

```
./builddir/release/gpop-daemon
```

By default, the server binds to `ws://127.0.0.1:9000`.

Options:
- `--bind` / `-b`: IP address to bind to (default: `127.0.0.1`)
- `--port` / `-p`: Port to listen on (default: `9000`)

Example with custom settings:

```
./builddir/release/gpop-daemon --bind 0.0.0.0 --port 8080
```

#### Running the Rust Client

```
./builddir/release/gpop-client
```

Or connect to a specific server:

```
./builddir/release/gpop-client ws://192.168.1.100:9000
```

See [daemon/README.md](daemon/README.md) for full API documentation.
