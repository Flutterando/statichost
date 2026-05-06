# statichost

Self-hosted alternative to Firebase Hosting + ngrok. Rust server + CLI, designed to run on Coolify behind Cloudflare Tunnel.

## Quick start

### Server

```bash
docker run -d --name statichost \
  -p 3000:3000 \
  -e STATICHOST_TOKEN=changeme \
  -e STATICHOST_DOMAIN=example.com \
  -v /data/sites:/sites \
  -v /data/dl:/dl \
  ghcr.io/<owner>/statichost:latest
```

Wildcard DNS `*.example.com` should point to the server.

### CLI

```bash
curl -fsSL https://example.com/install.sh | bash

statichost login --host https://example.com --token changeme
statichost deploy ./build/web --name myapp        # -> myapp.example.com
statichost tunnel 3000 --name myapi              # -> myapi.example.com -> localhost:3000
statichost list
statichost delete myapp
```

## Build from source

```bash
cargo build --release
./target/release/statichost-server   # server
./target/release/statichost          # cli
```

See [CLAUDE.md](CLAUDE.md) for architecture details.
