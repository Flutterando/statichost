# Brand assets

The logo represents what statichost does at a glance: a central node (the server) with rays radiating out to dynamic subdomain endpoints — the wildcard `*.your-domain.com` pattern that defines the product.

## Files

| File | Use |
|---|---|
| `logo.svg` | Source of truth. Vector, scales to anything. Embed in docs, slides, web pages. |
| `logo-512.png` | Square PNG for app stores, Docker Hub, Coolify catalog, GitHub social preview. |
| `logo-256.png` | README header. |
| `logo-128.png` | Smaller catalog cards, sidebar icons. |
| `logo-64.png` | Inline list icons. |
| `favicon-32.png` | Browser tab. |

## Regenerating PNGs

If you edit `logo.svg`, regenerate the rasters with `librsvg`:

```bash
cd assets
for size in 512 256 128 64; do
  rsvg-convert -w $size -h $size logo.svg -o logo-$size.png
done
rsvg-convert -w 32 -h 32 logo.svg -o favicon-32.png
```

## Color reference

| Role | Color | Hex |
|---|---|---|
| Background gradient | Slate-900 → Slate-800 | `#0f172a` → `#1e293b` |
| Rays | Cyan-400 → Blue-500 | `#22d3ee` → `#3b82f6` |
| Subdomain dots | Amber-300 → Amber-600 | `#fde68a` → `#d97706` |
| Center node | White → Slate-300 | `#ffffff` → `#cbd5e1` |

## Usage

Free to use in any context that talks about, links to, or integrates with statichost. Don't combine with another product's identity in a way that suggests endorsement.
