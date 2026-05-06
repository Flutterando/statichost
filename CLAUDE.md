# statichost

Self-hosted static site hosting + tunnel proxy. Uma alternativa self-hosted ao Firebase Hosting + ngrok, rodando no Coolify com Cloudflare Tunnel.

## Visao Geral

Um servidor Rust + CLI Rust que permite:
- Deploy de sites estaticos via CLI com um comando
- Tunnel de portas locais estilo ngrok, expondo localhost via subdominio publico
- Wildcard routing automatico - deploy um site e ele ja esta acessivel em {name}.{domain}

## Arquitetura

```
Internet -> Cloudflare Tunnel -> Traefik (Coolify) -> statichost server
                                                          |
                                            +-------------+-------------+
                                            |             |             |
                                        Sites estaticos  Tunnel proxy  API
                                        /sites/{name}/   WebSocket     /api/*
```

### Decisao de roteamento (runtime)

Quando um request chega em {name}.{domain}:
1. Existe tunnel WebSocket ativo para {name}? -> Proxy via WebSocket
2. Existe pasta /sites/{name}/? -> Serve arquivos estaticos (SPA-ready com try_files)
3. Nenhum dos dois? -> 404

### Componentes

- Server (server/): Rust com axum + tokio-tungstenite + tower
- CLI (cli/): Rust com clap + reqwest + tokio-tungstenite
- Dockerfile: Multi-stage build, imagem final ~10MB (scratch/alpine)
- CI: GitHub Actions cross-compile para todos os targets

## Stack Tecnico

### Server (Rust)
- axum - HTTP framework async
- tokio-tungstenite - WebSocket para tunnels
- tower - Middleware (auth Bearer token, logging)
- tokio - Runtime async
- flate2 / tar - Extracao de uploads .tar.gz

### CLI (Rust)
- clap - Parser de argumentos
- reqwest - HTTP client
- tokio-tungstenite - WebSocket client para tunnels
- tar / flate2 - Compactacao de pastas para upload

## Estrutura do Repositorio

```
statichost/
+-- server/
|   +-- Cargo.toml
|   +-- src/
|       +-- main.rs
|       +-- routes/
|       |   +-- api.rs          # POST /api/deploy, GET /api/sites, DELETE /api/sites/:name
|       |   +-- tunnel.rs       # WebSocket /api/tunnel?name=x
|       |   +-- static_files.rs # Serve /sites/{subdomain}/
|       |   +-- downloads.rs    # GET /dl/{binary}, /install.sh, /install.ps1
|       +-- proxy.rs            # Tunnel proxy: request <-> WebSocket <-> CLI
|       +-- router.rs           # Decisao: tunnel vs static vs 404
|       +-- auth.rs             # Bearer token validation
+-- cli/
|   +-- Cargo.toml
|   +-- src/
|       +-- main.rs
|       +-- commands/
|       |   +-- login.rs        # statichost login --host x --token y
|       |   +-- deploy.rs       # statichost deploy ./pasta --name x
|       |   +-- tunnel.rs       # statichost tunnel 3000 --name x
|       |   +-- list.rs         # statichost list
|       |   +-- delete.rs       # statichost delete x
|       +-- config.rs           # ~/.statichost/config.toml
|       +-- upload.rs           # Compacta pasta -> tar.gz -> POST /api/deploy
+-- scripts/
|   +-- install.sh              # Instalador macOS/Linux
|   +-- install.ps1             # Instalador Windows
+-- .github/
|   +-- workflows/
|       +-- release.yml         # Cross-compile + upload binarios
+-- Dockerfile
+-- CLAUDE.md
+-- README.md
```

## Variaveis de Ambiente (Server)

| Variavel | Descricao | Exemplo |
|---|---|---|
| STATICHOST_TOKEN | Token de autenticacao | meu-token-secreto |
| STATICHOST_DOMAIN | Dominio base | jacobmoura.work |
| STATICHOST_PORT | Porta do server | 3000 |
| STATICHOST_SITES_DIR | Diretorio dos sites | /sites |
| STATICHOST_BINARIES_DIR | Diretorio dos binarios para download | /dl |

## API Endpoints

### Autenticacao
Todos os endpoints (exceto /dl/* e /install.*) exigem header:
```
Authorization: Bearer {STATICHOST_TOKEN}
```

### Endpoints

| Metodo | Path | Descricao |
|---|---|---|
| POST | /api/deploy | Upload de site (multipart: name + archive .tar.gz) |
| GET | /api/sites | Lista sites deployados |
| DELETE | /api/sites/:name | Remove um site |
| GET | /api/tunnel | WebSocket - registra tunnel (query: name, token) |
| GET | /dl/:filename | Download de binarios da CLI |
| GET | /install.sh | Script de instalacao Linux/macOS |
| GET | /install.ps1 | Script de instalacao Windows |

## CLI Comandos

```bash
# Configuracao inicial
statichost login --host https://statichost.jacobmoura.work --token meu-token

# Deploy de site estatico
statichost deploy ./build/web --name meuapp
# Resultado: meuapp.jacobmoura.work

# Deploy Flutter Web (atalho)
statichost deploy-flutter --name preview
# Equivale a: flutter build web && statichost deploy ./build/web --name preview

# Listar sites
statichost list

# Remover site
statichost delete meuapp

# Tunnel (expor porta local)
statichost tunnel 3000 --name meuapi
# Resultado: meuapi.jacobmoura.work -> localhost:3000

# Tunnel com porta especifica
statichost tunnel 8080 --name dashboard
```

## Config da CLI

Salva em ~/.statichost/config.toml:
```toml
host = "https://statichost.jacobmoura.work"
token = "meu-token-secreto"
```

## Deploy no Coolify

### Dominio
```
http://*.jacobmoura.work
```

### Env vars no Coolify
```
STATICHOST_TOKEN=<gerar-token-seguro>
STATICHOST_DOMAIN=jacobmoura.work
```

### Volume
```
/data/sites -> /sites (persistencia dos sites deployados)
/data/statichost/dl -> /dl (binarios da CLI)
```

## CI/CD - GitHub Actions

### Targets de compilacao
```
x86_64-unknown-linux-gnu
aarch64-unknown-linux-gnu
x86_64-apple-darwin
aarch64-apple-darwin
x86_64-pc-windows-msvc
```

### Binarios gerados
```
statichost-linux-amd64
statichost-linux-arm64
statichost-darwin-amd64
statichost-darwin-arm64
statichost-windows-amd64.exe
```

### Workflow
1. Tag v* -> trigger release
2. Cross-compile server + CLI para todos os targets
3. Build Docker image do server (linux/amd64 + linux/arm64)
4. Push image para GitHub Container Registry (ghcr.io)
5. Upload binarios da CLI como release assets
6. Os binarios ficam disponiveis em /dl/ no proprio server

## Instalacao da CLI

### macOS / Linux
```bash
curl -fsSL https://statichost.jacobmoura.work/install.sh | bash
```

### Windows
```powershell
irm https://statichost.jacobmoura.work/install.ps1 | iex
```

### O que o script faz
1. Detecta OS (linux/darwin/windows) + arch (amd64/arm64)
2. Baixa binario de https://statichost.jacobmoura.work/dl/statichost-{os}-{arch}
3. Move para /usr/local/bin/statichost (Unix) ou %USERPROFILE%\.statichost\statichost.exe (Windows)
4. Torna executavel

## Fluxo completo de uso

```bash
# 1. Instala
curl -fsSL https://statichost.jacobmoura.work/install.sh | bash

# 2. Configura
statichost login --host https://statichost.jacobmoura.work --token meu-token

# 3. Deploya site
cd meu-projeto
flutter build web
statichost deploy ./build/web --name meuapp
# -> meuapp.jacobmoura.work

# 4. Ou expoe API local
statichost tunnel 3000 --name meuapi
# -> meuapi.jacobmoura.work -> localhost:3000
```

## Notas de Design

- Sem nginx: o server Rust serve estaticos diretamente (performance equivalente)
- SPA support: todo site estatico usa try_files - se o arquivo nao existe, serve index.html
- Token fixo MVP: evolucao futura pode adicionar multiplas API keys com revogacao individual
- Tunnel via WebSocket: requests HTTP chegam no server, sao empacotados e enviados via WebSocket pro CLI, que faz o request local e devolve a response
- Binarios self-hosted: o proprio server serve o instalador e os binarios, zero dependencia externa
- Container minimo: imagem Docker baseada em scratch/alpine, ~10MB
