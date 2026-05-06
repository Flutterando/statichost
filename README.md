# statichost

Self-hosted alternative to Firebase Hosting + ngrok. Rust server + CLI.

- **Static hosting**: deploy a folder with one command, served on `{name}.your-domain.com`.
- **Tunnel**: expose any local port through a public subdomain (no third-party tunnel service).
- **Single binary CLI**: macOS, Linux, Windows, amd64 and arm64.
- **Single container server**: ~30 MB image, no nginx, runs on Coolify, Portainer or any Docker host.

Source: [github.com/Flutterando/statichost](https://github.com/Flutterando/statichost)
Image: [hub.docker.com/r/jacobmoura7/statichost](https://hub.docker.com/r/jacobmoura7/statichost)

---

## Table of contents

1. [How it works](#how-it-works)
2. [Server installation](#server-installation)
   - [docker run](#docker-run)
   - [docker-compose](#docker-compose)
   - [Coolify](#coolify)
   - [Portainer](#portainer)
3. [CLI installation](#cli-installation)
   - [macOS / Linux](#macos--linux)
   - [Windows](#windows)
4. [Hosting examples](#hosting-examples)
5. [Tunnel examples](#tunnel-examples)
6. [Best practices](#best-practices)
7. [Configuration reference](#configuration-reference)

---

## How it works

```
Internet → Reverse proxy (Traefik / Cloudflare Tunnel) → statichost
                                                           ├─ active tunnel for {name}? → WebSocket proxy
                                                           ├─ folder /sites/{name}/?     → static files (SPA fallback)
                                                           └─ none of the above           → 404
```

Each request is routed by the `Host` header. `foo.example.com` → tunnel `foo` if connected, otherwise the folder `/sites/foo/`, otherwise 404.

---

## Server installation

### Prerequisites

- A domain you control. **Subdomains are created dynamically**, so you must dedicate either a root zone (`example.com`) or a sub-zone (`apps.example.com`) to statichost — see [Best practices](#best-practices).
- A wildcard DNS record `*.your-domain.com` pointing to your server.
- A reverse proxy that terminates TLS for the wildcard domain (Coolify / Traefik / Cloudflare Tunnel).
- A strong token: `openssl rand -hex 32`.

### docker run

```bash
docker run -d --name statichost --restart unless-stopped \
  -p 3000:3000 \
  -e STATICHOST_TOKEN=devtoken \
  -e STATICHOST_DOMAIN=localtest.me \
  -v /data/statichost/sites:/sites \
  -v /data/statichost/dl:/dl \
  jacobmoura7/statichost:latest
```

### docker-compose

```yaml
services:
  statichost:
    image: jacobmoura7/statichost:latest
    restart: unless-stopped
    ports:
      - "3000:3000"
    environment:
      STATICHOST_TOKEN: devtoken
      STATICHOST_DOMAIN: localtest.me
    volumes:
      - sites:/sites
      - dl:/dl

volumes:
  sites:
  dl:
```

Run with `STATICHOST_TOKEN=$(openssl rand -hex 32) docker compose up -d`.

### Coolify

1. **New resource → Docker Image**, set image to `jacobmoura7/statichost:latest`.
2. **Domains**: enter `http://*.example.com` — the leading wildcard is the key part. Coolify generates a Traefik/Caddy rule that catches every subdomain of `example.com` and forwards to this service on port `3000`.
3. **Environment variables**:
   - `STATICHOST_TOKEN` — generate a long random value (mark as secret).
   - `STATICHOST_DOMAIN` — `example.com` (must match the wildcard above, without `*.`).
4. **Persistent storage**: add two volumes:
   - `/data/statichost/sites` → `/sites`
   - `/data/statichost/dl` → `/dl`
5. **Network port exposed**: `3000`.
6. Deploy. After the first deploy, Coolify provisions a wildcard certificate via Let's Encrypt (DNS challenge) — point your DNS `*.example.com` to the Coolify host.

### Portainer

1. **Stacks → Add stack**, paste the docker-compose snippet from above.
2. In **Environment variables**, set `STATICHOST_TOKEN` to a strong random value and confirm `STATICHOST_DOMAIN`.
3. Deploy.
4. Put a reverse proxy in front of port `3000` for the wildcard domain. The two common options:
   - **Traefik**: add labels to the stack —
     ```yaml
     labels:
       - traefik.enable=true
       - traefik.http.routers.statichost.rule=HostRegexp(`{sub:[a-z0-9-]+}.example.com`) || Host(`example.com`)
       - traefik.http.routers.statichost.entrypoints=websecure
       - traefik.http.routers.statichost.tls.certresolver=letsencrypt
       - traefik.http.routers.statichost.tls.domains[0].main=example.com
       - traefik.http.routers.statichost.tls.domains[0].sans=*.example.com
       - traefik.http.services.statichost.loadbalancer.server.port=3000
     ```
   - **Cloudflare Tunnel**: in your tunnel config, route `*.example.com` → `http://statichost:3000`. Cloudflare handles TLS, you can run statichost without certificates.

---

## CLI installation

### macOS / Linux

```bash
curl -fsSL https://example.com/install.sh | bash
```

The server hosts an installer that detects your OS (Linux / macOS) and arch (amd64 / arm64), downloads the right binary, and drops it in `/usr/local/bin/statichost` (uses `sudo` when needed).

Manual install:

```bash
# pick the right binary for your platform
curl -fsSL https://example.com/dl/statichost-darwin-arm64 -o statichost   # macOS Apple Silicon
curl -fsSL https://example.com/dl/statichost-darwin-amd64 -o statichost   # macOS Intel
curl -fsSL https://example.com/dl/statichost-linux-amd64  -o statichost   # Linux x86_64
curl -fsSL https://example.com/dl/statichost-linux-arm64  -o statichost   # Linux ARM (RPi, Graviton)

chmod +x statichost
sudo mv statichost /usr/local/bin/
```

### Windows

PowerShell:

```powershell
irm https://example.com/install.ps1 | iex
```

This downloads `statichost.exe` to `%USERPROFILE%\.statichost\` and adds it to your user `PATH` (open a new terminal afterwards).

Manual install:

```powershell
$dir = "$env:USERPROFILE\.statichost"
New-Item -ItemType Directory -Force -Path $dir | Out-Null
Invoke-WebRequest "https://example.com/dl/statichost-windows-amd64.exe" -OutFile "$dir\statichost.exe"
[Environment]::SetEnvironmentVariable('Path', "$env:Path;$dir", 'User')
```

### Verify

```bash
statichost --version
statichost --help
```

### Login

```bash
statichost login --host https://example.com --token <your-token>
```

Credentials are saved to `~/.statichost/config.toml` (mode `0600` on Unix).

---

## Hosting examples

Static hosting takes any folder with `index.html` (or `index.htm`) and serves it on `{name}.example.com` with SPA fallback (any 404 falls back to `/index.html`).

### Plain HTML / CSS / JS

```bash
statichost deploy ./public --name landing
# → https://landing.example.com
```

### Vite / React / Next.js (static export) / Vue / Svelte

```bash
npm run build               # outputs to ./dist (Vite) or ./out (Next export) or ./build (CRA)
statichost deploy ./dist --name app
# → https://app.example.com
```

### Flutter Web

```bash
statichost deploy-flutter --name preview
# runs `flutter build web` then deploys ./build/web
# → https://preview.example.com
```

### Other commands

```bash
statichost list                       # show deployed sites with size
statichost delete app                 # remove a site (asks confirmation)
statichost delete app --force         # without confirmation
```

### What happens on deploy

1. CLI tar-gzips the folder in memory.
2. Uploads via multipart `POST /api/deploy`.
3. Server extracts to a staging directory, then atomically swaps with the live folder — no half-served state, no downtime.
4. New version is live the moment the swap completes.

---

## Tunnel examples

Tunnels are like ngrok: a public subdomain proxies HTTP requests through a WebSocket back to a port on your laptop.

### Expose a local API

```bash
# in one terminal
node server.js                 # listening on localhost:3000

# in another terminal
statichost tunnel 3000 --name api
# → https://api.example.com → localhost:3000
```

### Share a Vite dev server with QA

```bash
npm run dev                    # vite on localhost:5173
statichost tunnel 5173 --name preview
# → https://preview.example.com
```

### Webhook receiver from Stripe / GitHub / etc

```bash
statichost tunnel 4242 --name hooks
# point https://hooks.example.com/webhook in the third-party dashboard
```

### Behavior

- The CLI reconnects with exponential backoff (1s → 30s) if the connection drops.
- Headers `X-Forwarded-For`, `X-Forwarded-Host`, `X-Forwarded-Proto` are added automatically.
- Hop-by-hop headers (`Connection`, `Upgrade`, `Transfer-Encoding`, …) are stripped on both directions.
- A live tunnel **shadows** any static deploy with the same name — you can stage a static replacement under the same subdomain while the tunnel is up, and it takes over the moment the tunnel closes.
- WebSocket upgrade from the public side is currently not supported (returns `501`). HTTP/1.1 with chunked bodies works fine.

---

## Best practices

### Use a dedicated root or sub-zone

statichost creates subdomains **dynamically** — you don't pre-register them. That only works if the **whole zone** is forwarded to it.

| Setup | Works? | Why |
|---|---|---|
| `*.example.com` → statichost | ✅ | Any new `{name}` becomes a subdomain instantly. |
| `*.apps.example.com` → statichost (and `example.com` used for other things) | ✅ | Set `STATICHOST_DOMAIN=apps.example.com`. Sub-zone is fully delegated. |
| `apps.example.com` → statichost (single host, no wildcard) | ❌ | Can't generate `foo.apps.example.com` without wildcard DNS + wildcard reverse-proxy rule. |
| `*.example.com` shared with another service | ⚠️ | Routing collisions. Pick a dedicated zone. |

In short: **put statichost at the root of whatever zone you give it, and dedicate that zone to it.**

### Generate strong tokens

```bash
openssl rand -hex 32
```

Treat the token like a password. It's a single shared secret in the MVP — don't paste it in CI logs or commit to git. Rotate by updating `STATICHOST_TOKEN` on the server and re-running `statichost login` on every machine that uses the CLI.

### Reserved subdomains

The server rejects deploys/tunnels named `www`, `api`, `admin`, `dl`, `install` — they collide with the server's own routes. Pick something else.

### Wildcard TLS

Use either:
- **Cloudflare Tunnel** — TLS terminated by Cloudflare, server runs plaintext on `:3000`. Simplest setup.
- **Traefik / Caddy with Let's Encrypt DNS challenge** — issues a real wildcard cert. Requires API access to your DNS provider.

HTTP-01 challenge does **not** work for wildcards — don't waste time on it.

### Persist `/sites` and `/dl`

`/sites` holds your deployed content. `/dl` holds CLI binaries served by the server. Mount both to durable storage so an image upgrade or container recreation doesn't wipe them.

### CLI binaries on the server

After you tag a release on GitHub, the CLI binaries are published as release assets. Drop them in your `/dl` volume so `https://example.com/dl/statichost-linux-amd64` and the install scripts work for your team. (The release workflow doesn't push to a running instance automatically — that's by design, to keep the CI free of production credentials.)

### Resource sizing

The server is mostly I/O. A `0.25 vCPU / 256 MB` container handles dozens of small static sites and a few active tunnels comfortably. Bump RAM if you serve large uploads (>50 MB single-file).

---

## Configuration reference

| Env var | Required | Default | Description |
|---|---|---|---|
| `STATICHOST_TOKEN` | yes | — | Bearer token for the API and tunnel WebSocket |
| `STATICHOST_DOMAIN` | yes | — | Base zone, e.g. `example.com` or `apps.example.com` |
| `STATICHOST_PORT` | no | `3000` | Listen port inside the container |
| `STATICHOST_SITES_DIR` | no | `/sites` | Directory where deployed sites live |
| `STATICHOST_BINARIES_DIR` | no | `/dl` | Directory served by `/dl/{filename}` |
| `RUST_LOG` | no | `info` | Log filter (e.g. `debug`, `statichost_server=trace`) |

CLI config is at `~/.statichost/config.toml` and contains `host` and `token` — managed by `statichost login`, no need to edit by hand.

---

See [CLAUDE.md](CLAUDE.md) for the full architecture spec.
