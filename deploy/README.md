# Deployment recipes

statichost dynamically creates subdomains, which makes it tricky to deploy with reverse proxies that expect a static list of hostnames (Coolify, Caprover, most one-click PaaS). Pick the recipe that matches your setup:

| Recipe | When to use | TLS | DNS |
|---|---|---|---|
| [`docker-compose.caddy.yml`](./docker-compose.caddy.yml) | Self-hosted on a VPS with public ports 80/443 | Caddy on-demand (Let's Encrypt) | Wildcard A record at your registrar |
| [`docker-compose.cloudflared.yml`](./docker-compose.cloudflared.yml) | Anywhere — VPS, Coolify, behind NAT | Cloudflare edge | Cloudflare Zero Trust tunnel auto-creates DNS |
| [Coolify with Cloudflare Tunnel](#coolify-with-cloudflared-bypass) | Already on Coolify, want to keep it | Cloudflare edge | Wildcard via cloudflared |
| [Coolify native](#coolify-native-fixed-subdomains) | Coolify, no wildcards needed | Coolify-managed (Let's Encrypt) | Each subdomain registered in Coolify UI |

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

## Coolify with cloudflared bypass

You already have Coolify running other apps and don't want to disrupt it. Deploy statichost as a Coolify resource but route around Coolify's Traefik using cloudflared.

1. **In Coolify**, deploy the `docker-compose.cloudflared.yml` from this repo as a new resource (Service → Docker Compose Stack).
2. **Leave the Domain field blank** — this is critical. Coolify's domain field assumes specific subdomains; for wildcards you must skip it.
3. Set environment variables `STATICHOST_TOKEN`, `STATICHOST_DOMAIN`, `CLOUDFLARED_TOKEN` in the resource's UI.
4. Deploy.

Coolify will run both containers but won't try to add Traefik labels for them, since there's no domain registered. The cloudflared sidecar handles all routing.

Other apps you have on Coolify continue working normally — they live behind Coolify's Traefik on a different network path.

## Coolify native (fixed subdomains)

If you only need a small set of pre-known subdomains (e.g., one preview site, no tunnels with random names), you can deploy statichost as a regular Coolify resource and register each subdomain manually in the Domains field, separated by commas:

```
http://app.your-domain.com,http://blog.your-domain.com,http://staging.your-domain.com
```

Drawback: the subdomain has to be registered before you can deploy a site to it. Defeats most of statichost's value. Use Cloudflare Tunnel above instead.
