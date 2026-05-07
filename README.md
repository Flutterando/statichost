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
   - [Local quick test](#local-quick-test-no-real-domain)
   - [Production recipes](#production-pick-a-recipe) — see [`deploy/`](deploy/)
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
- A way to terminate TLS for the wildcard. statichost itself doesn't speak TLS; you put **Caddy**, **Cloudflare Tunnel**, or another wildcard-aware proxy in front.
- A strong token: `openssl rand -hex 32`.

> **Note**: The reverse-proxy story is the trickiest part of deploying statichost — it dynamically creates subdomains, which clashes with PaaS platforms (Coolify, Caprover) that expect each hostname to be registered up-front. The [`deploy/`](deploy/) directory has ready-to-use compose recipes for the common scenarios.

### Local quick test (no real domain)

For a 2-minute test before committing to a real domain. Uses `localtest.me` (a public DNS that resolves `*.localtest.me` to `127.0.0.1`):

```bash
docker run -d --rm --name statichost-test \
  -p 3000:3000 \
  -e STATICHOST_TOKEN=devtoken \
  -e STATICHOST_DOMAIN=localtest.me \
  -v /tmp/statichost-sites:/sites \
  jacobmoura7/statichost:latest

curl http://localhost:3000/api/health -H "Authorization: Bearer devtoken"
# → ok

# Check landing page
curl http://localtest.me:3000/
```

### Production: pick a recipe

| Recipe | Best for | Public ports needed | Cloudflare account |
|---|---|---|---|
| [Caddy with on-demand TLS](deploy/docker-compose.caddy.yml) | VPS with public IP | 80 + 443 | no |
| [Cloudflare Tunnel sidecar](deploy/docker-compose.cloudflared.yml) | Anywhere — VPS, NAT, Coolify | none | yes |
| [Coolify with a subdomain list](deploy/README.md#coolify-with-a-subdomain-list-recommended-for-coolify-users) | Already running Coolify, predictable set of names | Coolify-managed | no |
| [Coolify + cloudflared bypass](deploy/README.md#coolify-with-cloudflared-bypass) | Coolify + unbounded random names (per-PR previews, ephemeral tunnels) | none | yes |

See [`deploy/README.md`](deploy/README.md) for full setup steps.

The shortest path for most people:

```bash
git clone https://github.com/Flutterando/statichost
cd statichost/deploy

# self-hosted on a VPS with public IP
STATICHOST_TOKEN=$(openssl rand -hex 32) \
STATICHOST_DOMAIN=your-domain.com \
docker compose -f docker-compose.caddy.yml up -d
```

Or with Cloudflare (works behind NAT, on Coolify, anywhere):

```bash
STATICHOST_TOKEN=$(openssl rand -hex 32) \
STATICHOST_DOMAIN=your-domain.com \
CLOUDFLARED_TOKEN=eyJh...   # from Cloudflare Zero Trust dashboard
docker compose -f docker-compose.cloudflared.yml up -d
```

### Why doesn't a wildcard in Coolify's Domain field "just work"?

Coolify (and similar PaaS) generate one literal `Host(...)` Traefik rule per item in the Domain field. They were designed for one app = one hostname. A `*.your-domain.com` either gets treated as a literal `*` character or generates no rule at all — Traefik then returns `404 page not found` for every actual subdomain.

The four recipes work around this differently:

- **Caddy** and **cloudflared** sidecars **replace** Coolify's reverse proxy for this stack. Other Coolify apps keep using Coolify's Traefik untouched.
- The **subdomain list** trick uses Coolify's native routing as designed (literal hostnames), but lists every subdomain you'll ever need up-front. statichost still does internal routing, so each entry just points to the same container.
- The **cloudflared bypass** keeps Coolify for other apps but skips its proxy for statichost specifically.

---

## CLI installation

The CLI binaries are published as **GitHub Release assets**. The install scripts below pull from `releases/latest`, so they always grab the newest version. To pin a specific version, set `STATICHOST_VERSION=v0.1.0` before running the installer.

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/Flutterando/statichost/main/scripts/install.sh | sh
```

Detects OS (Linux / macOS) and arch (amd64 / arm64), downloads the matching binary from the latest GitHub Release, and drops it in `/usr/local/bin/statichost` (uses `sudo` if the directory isn't writable).

Manual install:

```bash
# pick the right binary for your platform
BASE=https://github.com/Flutterando/statichost/releases/latest/download
curl -fsSL "$BASE/statichost-darwin-arm64" -o statichost   # macOS Apple Silicon
# curl -fsSL "$BASE/statichost-darwin-amd64" -o statichost   # macOS Intel
# curl -fsSL "$BASE/statichost-linux-amd64"  -o statichost   # Linux x86_64
# curl -fsSL "$BASE/statichost-linux-arm64"  -o statichost   # Linux ARM (RPi, Graviton)

chmod +x statichost
sudo mv statichost /usr/local/bin/
```

### Windows

PowerShell:

```powershell
irm https://raw.githubusercontent.com/Flutterando/statichost/main/scripts/install.ps1 | iex
```

This downloads `statichost.exe` to `%USERPROFILE%\.statichost\` and adds it to your user `PATH` (open a new terminal afterwards).

Manual install:

```powershell
$dir = "$env:USERPROFILE\.statichost"
New-Item -ItemType Directory -Force -Path $dir | Out-Null
Invoke-WebRequest "https://github.com/Flutterando/statichost/releases/latest/download/statichost-windows-amd64.exe" -OutFile "$dir\statichost.exe"
[Environment]::SetEnvironmentVariable('Path', "$env:Path;$dir", 'User')
```

### Verify

```bash
statichost --version
statichost --help
```

### Login

The CLI talks to the API subdomain (default `statichost.{your-domain}`):

```bash
statichost login --host https://statichost.example.com --token <your-token>
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

The server rejects deploys/tunnels named `www`, `api`, `admin`, `dl`, `install`, plus whatever you set as `STATICHOST_API_SUBDOMAIN` (default `statichost`) — they collide with the server's own routes. Pick something else.

### Wildcard TLS

Three options that work, in increasing order of complexity:

1. **Caddy with on-demand TLS** ([recipe](deploy/docker-compose.caddy.yml)) — issues per-subdomain certs lazily, no DNS API access. Best for self-hosting on a VPS.
2. **Cloudflare Tunnel** ([recipe](deploy/docker-compose.cloudflared.yml)) — TLS terminated at Cloudflare's edge, statichost runs plaintext. Works anywhere, including behind NAT.
3. **Traefik / Caddy with Let's Encrypt DNS-01 challenge** — issues a real `*.example.com` cert. Requires API access to your DNS provider.

HTTP-01 challenge does **not** issue wildcard certs — don't waste time on it.

Coolify's built-in cert/proxy doesn't handle wildcards in its Domain field. Use one of the recipes in [`deploy/`](deploy/) instead of trying to coerce Coolify's Traefik.

### Persist `/sites`

`/sites` holds your deployed content. Mount it to durable storage so an image upgrade or container recreation doesn't wipe deployments.

### CLI distribution

CLI binaries live on **GitHub Releases** — the server doesn't host them. The install scripts in this repo's `scripts/` directory point to `releases/latest/download/...`, so a `git tag v* && git push --tags` is enough to make a new version available to anyone via the curl-pipe install command. No production credentials in CI, no `/dl` volume to populate.

### Resource sizing

The server is mostly I/O. A `0.25 vCPU / 256 MB` container handles dozens of small static sites and a few active tunnels comfortably. Bump RAM if you serve large uploads (>50 MB single-file).

---

## Configuration reference

| Env var | Required | Default | Description |
|---|---|---|---|
| `STATICHOST_TOKEN` | yes | — | Bearer token for the API and tunnel WebSocket |
| `STATICHOST_DOMAIN` | yes | — | Base zone, e.g. `example.com` or `apps.example.com` |
| `STATICHOST_API_SUBDOMAIN` | no | `statichost` | Subdomain that exposes the API + tunnel WebSocket (point your CLI at `https://{this}.{domain}`) |
| `STATICHOST_PORT` | no | `3000` | Listen port inside the container |
| `STATICHOST_SITES_DIR` | no | `/sites` | Directory where deployed sites live |
| `RUST_LOG` | no | `info` | Log filter (e.g. `debug`, `statichost_server=trace`) |

CLI config is at `~/.statichost/config.toml` and contains `host` and `token` — managed by `statichost login`, no need to edit by hand.

---

See [CLAUDE.md](CLAUDE.md) for the full architecture spec.
