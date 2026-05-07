# Deployment recipes

statichost dynamically creates subdomains, which makes it tricky to deploy with reverse proxies that expect a static list of hostnames (Coolify, Caprover, most one-click PaaS). Pick the recipe that matches your setup:

| Recipe | When to use | TLS | DNS |
|---|---|---|---|
| [`docker-compose.caddy.yml`](./docker-compose.caddy.yml) | Self-hosted on a VPS with public ports 80/443 | Caddy on-demand (Let's Encrypt) | Wildcard A record at your registrar |
| [`docker-compose.cloudflared.yml`](./docker-compose.cloudflared.yml) | Anywhere — VPS, Coolify, behind NAT | Cloudflare edge | Cloudflare Zero Trust tunnel auto-creates DNS |
| [Coolify with a subdomain list](#coolify-with-a-subdomain-list-recommended-for-coolify-users) | You already use Coolify and want it to manage TLS | Coolify-managed (Let's Encrypt per subdomain) | Each subdomain registered in Coolify's Domain field |
| [Coolify with cloudflared bypass](#coolify-with-cloudflared-bypass) | Coolify + you need unbounded wildcards | Cloudflare edge | Wildcard via cloudflared |

## Caddy with on-demand TLS (recommended for VPS)

The simplest fully-self-hosted option. Caddy ships in front of statichost, requests Let's Encrypt certs the first time each subdomain is hit, and proxies everything to statichost. No DNS API access, no Cloudflare account.

Requirements:
- A VPS with public IPv4 + ports 80 and 443 free
- Wildcard DNS `*.your-domain.com` pointing to that VPS
- (Optional) apex `your-domain.com` pointing to the same IP

```bash
cd deploy
STATICHOST_TOKEN=$(openssl rand -hex 32) \
STATICHOST_DOMAIN=your-domain.com \
docker compose -f docker-compose.caddy.yml up -d
```

Limitations:
- Let's Encrypt rate-limits 50 certs/week per registered domain. Plenty for static deploys; tight if you spawn many ephemeral tunnel names.
- Any hostname that resolves to your IP triggers a cert request. Restrict your DNS wildcard scope if abuse is a concern.

## Cloudflare Tunnel (recommended for Coolify / behind NAT)

statichost runs in its own container, with a `cloudflared` sidecar that opens an outbound tunnel to Cloudflare. Cloudflare terminates TLS on its edge and forwards all `*.your-domain.com` traffic through the tunnel. Works on any host with outbound internet — no public IP, no port forwarding.

Setup:
1. In [Cloudflare Zero Trust](https://one.dash.cloudflare.com) → **Networks → Tunnels** → **Create a tunnel** → **Cloudflared** → name it `statichost`.
2. Skip the "Install cloudflared" step (the docker container does this); copy the **token** shown after creation.
3. Click **Next** → **Add a public hostname**:
   - Subdomain: `*`
   - Domain: your-domain.com
   - Service: HTTP — `http://statichost:3000`
4. Save.

```bash
cd deploy
STATICHOST_TOKEN=$(openssl rand -hex 32) \
STATICHOST_DOMAIN=your-domain.com \
CLOUDFLARED_TOKEN=eyJhIj... \
docker compose -f docker-compose.cloudflared.yml up -d
```

Cloudflare automatically creates the wildcard DNS for you.

## Coolify with a subdomain list (recommended for Coolify users)

The pragmatic Coolify path. Coolify's reverse proxy can't do wildcard routing — its Domain field generates one literal `Host(...)` Traefik rule per item. Wildcards like `*.example.com` either get treated as a literal `*` or do nothing. So you maintain a **comma-separated list of every subdomain you want**, and Coolify routes each one to statichost.

Counter-intuitive but it works: every subdomain in the list points to the same statichost container, which then routes by Host header internally. New subdomain? Edit the field, click save, redeploy — Coolify applies new Traefik rules in seconds.

### Setup

1. **New resource → Docker Image**, image: `jacobmoura7/statichost:latest`
2. **Domains** field: paste your subdomain list as `http://...` URLs separated by commas. A starter set covering common cases:
   ```
   http://statichost.your-domain.com,http://www.your-domain.com,http://app.your-domain.com,http://blog.your-domain.com,http://docs.your-domain.com,http://demo.your-domain.com,http://preview.your-domain.com,http://staging.your-domain.com
   ```
   The `statichost.your-domain.com` entry is the API host (CLI talks to it). The rest are slots for future deploys/tunnels.
3. **Environment variables**:
   - `STATICHOST_TOKEN` — `openssl rand -hex 32` (mark as secret)
   - `STATICHOST_DOMAIN` — your domain, no scheme, no `*.`
4. **Persistent storage**: `/data/statichost/sites` → `/sites`
5. **Network port exposed**: `3000`
6. Save and deploy.

Coolify will issue a Let's Encrypt cert for each subdomain (HTTP-01 challenge per hostname).

### Adding subdomains later

Edit the Domains field, append `http://newname.your-domain.com`, click save. Coolify regenerates Traefik rules and provisions the new cert. Then `statichost deploy ./folder --name newname` works.

### About the Coolify warning

When you save a comma-separated list, Coolify shows a yellow warning saying "not all services support multiple subdomains". This is a generic warning aimed at apps that embed their own canonical URL in HTML/emails. **statichost handles multiple subdomains by design** — every request is routed by its `Host` header, no global "self URL" concept. Ignore the warning.

If you want to silence it permanently, the only path today is contributing a `accepts_wildcards` flag to the Coolify project so service templates can opt out — see [coollabsio/coolify](https://github.com/coollabsio/coolify) on GitHub.

### Limitations

- **Each subdomain has to exist in the Coolify list before you deploy to it.** No truly dynamic creation. If you need ephemeral / random names (e.g., per-PR previews with auto-generated names), use the cloudflared bypass below instead.
- **Coolify charges no Let's Encrypt rate budget**, but Let's Encrypt itself caps at 50 issuances/week per registered domain. Long enough lists or frequent recreations can hit that.

## Coolify with cloudflared bypass

You already have Coolify running other apps and want unlimited dynamic subdomains for statichost (no list maintenance). Deploy statichost as a Coolify resource but route around Coolify's Traefik using cloudflared.

1. **In Coolify**, deploy [`docker-compose.cloudflared.yml`](./docker-compose.cloudflared.yml) as a new resource (Service → Docker Compose Stack).
2. **Leave the Domain field blank** — critical. With Domain empty, Coolify doesn't generate Traefik labels and the cloudflared sidecar handles routing instead.
3. Set environment variables `STATICHOST_TOKEN`, `STATICHOST_DOMAIN`, `CLOUDFLARED_TOKEN` in the resource's UI.
4. Deploy.

Coolify runs both containers without touching their routing. cloudflared opens an outbound tunnel to Cloudflare and forwards `*.your-domain.com` traffic directly to the statichost container.

Other apps on Coolify continue working — they live behind Coolify's Traefik on a different routing path.

### Tradeoff vs the subdomain list

| | Subdomain list | cloudflared bypass |
|---|---|---|
| Subdomain creation | Edit Coolify field + redeploy | Just `statichost deploy --name X` |
| TLS | Coolify Let's Encrypt per name | Cloudflare edge |
| Unbounded names | ❌ | ✅ |
| Cloudflare account required | No | Yes |
| Coolify warning on save | Yes (harmless) | None |
| Best for | Stable list of predictable subdomains | Per-PR previews, random tunnel names |
