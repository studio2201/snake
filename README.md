# Snake - Traditional Arcade Game

<p align="center">
  <img src="https://raw.githubusercontent.com/UberMetroid/snake/main/assets/logo.png?v=1.0.36" alt="Snake Logo" width="128" height="128">
</p>

## Overview

Snake is a self-hosted traditional arcade-style snake game application designed for home servers and NAS systems. Built with a high-performance Rust (Axum/Tokio) backend and a WebAssembly (Yew) frontend.

---

## Key Features

*   **Traditional Arcade Loop**: Classic gameplay with grid rendering, score tracking, and persistent high scores.
*   **Gold Food Mode**: 15% spawn chance of a flashing Gold Food that expires in 5 seconds (with a dynamic visual countdown bar) and awards +30 points.
*   **High Score Leaderboard**: Persists the Top 10 player scores using simple file-based JSON storage (`leaderboard.json`).
*   **Sleek Neon Theme**: Dark retro-futuristic styling matching the Super Metroid theme design system.
*   **Mobile-Friendly D-Pad**: Integrated touch/D-Pad controls overlay for easy play on mobile and tablets.
*   **Access PIN Security**: Lock down the interface with an optional numerical PIN for absolute privacy.

---

## Container Registry

The Docker image is built with **Nix** (no Alpine, fully reproducible) and published to Docker Hub:

*   **Docker Hub**: [ubermetroid/snake](https://hub.docker.com/r/ubermetroid/snake)

---

## Container Installation

The shipped [`docker-compose.yml`](./docker-compose.yml) is the full recipe; copy it (or rename it to `compose.yaml`) and adjust the host-side `${VAR}` overrides as needed. Three things are **mandatory**:

| Setting | Why |
| :--- | :--- |
| `image: ubermetroid/snake:latest` | Container can't run without an image. Pull from Docker Hub or replace with a `nix build .#dockerImage && docker load < result` output. |
| `volumes: ["./data:/app/data", ...]` | Persists the leaderboard (`${...}/leaderboard.json`) and log files. **Without this mount, scores vanish on every container restart.** |
| `ports: ["4501:4501"]` | Exposes the SPA + API to the host. Change the host side to e.g. `"8080:4501"` to publish on a different port. |

### Minimum viable recipe

If you want the absolute minimum to come up, the line-noise YAML above already is the smallest sensible version. Equivalent bare-bones form, for reference:

```yaml
services:
  snake:
    image: ubermetroid/snake:latest
    volumes:
      - ./data:/app/data
    ports:
      - "4501:4501"
    environment:
      BASE_URL: http://localhost:4501   # change for non-localhost deploys
      SNAKE_PIN: ""                     # leave empty for no auth, or set a 4-64 char PIN
```

### What you usually need to change for production

| Setting | When |
| :--- | :--- |
| `BASE_URL` | Must match the *public* URL (with scheme + host, no trailing slash). Wrong value breaks `Secure` cookie flagging, the PWA install prompt, and the `Origin`-header CSRF defense. |
| `SNAKE_PIN` | Any deployment that isn't `localhost` demo. 4-64 chars. |
| Reverse-proxy forwarding | TLS should be terminated upstream and `x-forwarded-proto: https` passed through; the backend only marks cookies Secure when both (a) the header says HTTPS or (b) `BASE_URL` starts with `https`. See "Production Deployment" below. |

### Run

```bash
docker compose up -d
```

Open `http://localhost:4501` (or whatever host port you chose) in a browser.

## Local Development (Trunk)

For frontend iteration outside Nix:

```bash
cd frontend

### Local Development (Trunk)

For frontend iteration outside Nix:

```bash
cd frontend
trunk build --release                  # 520 KB WASM
./scripts/optimise-wasm.sh             # 355 KB WASM (-32% raw, -19% gzipped)
```

### Building the Image Locally

To build the Docker container locally from the source files using Nix:

```bash
nix build .#dockerImage
docker load < result
docker tag snake-nix:latest ubermetroid/snake:latest
```

The image is Nix-built (no Alpine, no Docker daemon dependency for the build).

For development iteration, use the devShell:

```bash
nix develop
```

---

## Flake Installation

You can also run or install Snake directly using Nix flakes:

```bash
# Run the application directly
nix run github:UberMetroid/snake --impure

# Install to your user profile
nix profile install github:UberMetroid/snake --impure
```

---

## Production Deployment

Snake assumes TLS termination happens at a reverse proxy. The cookie's `Secure` flag is set when either:
1. The request's `x-forwarded-proto` header equals `https` (case-insensitive)
2. The `BASE_URL` config starts with `https://`

### nginx example

```nginx
server {
    listen 443 ssl http2;
    server_name snake.example.com;

    ssl_certificate     /etc/ssl/certs/snake.crt;
    ssl_certificate_key /etc/ssl/private/snake.key;

    location / {
        proxy_pass http://127.0.0.1:4501;
        proxy_set_header X-Forwarded-Proto  $scheme;
        proxy_set_header X-Forwarded-For    $proxy_add_x_forwarded_for;
        proxy_set_header Host                $host;
        proxy_http_version 1.1;
        proxy_set_header Connection         "";
    }
}
```

### caddy example

```caddyfile
snake.example.com {
    reverse_proxy 127.0.0.1:4501 {
        header_up X-Forwarded-Proto {scheme}
    }
}
```

### Required env

When fronting with a reverse proxy, ensure:
- `BASE_URL` matches the public-facing URL (e.g., `https://snake.example.com`)
- `TRUST_PROXY=true` so the `x-forwarded-for` chain is honoured for rate limiting and PIN lockout (otherwise every request looks like it came from `127.0.0.1` and the lockout becomes global)
- `ALLOWED_ORIGINS` lists the public URL; leaving it at the default `*` is acceptable because auth cookies are `SameSite=Strict`

### Service-worker and HTTPS

Service workers only register on `https://` (or `http://localhost`). Operators behind a non-HTTPS reverse proxy will see no PWA install prompt and no offline cache; the game still works as a regular web app.

---

## Configuration Options

Configure these settings inside your Docker Compose environment or container environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | The port number the backend HTTP server will bind to inside the container. | `4501` |
| `SITE_TITLE` | Custom website title rendered in navigation headers, browser tabs, and PWA manifest. | `Snake` |
| `BASE_URL` | Application base URL. | `http://localhost:4501` |
| `ALLOWED_ORIGINS` | Comma-separated list of allowed HTTP request origins (CORS filter). Use `*` to allow all origins. | `*` |
| `SNAKE_PIN` | Optional PIN to lock access to the interface. Frontend accepts 4–10 ASCII digits; backend accepts 4-64 characters of any kind. Leave empty for public mode. *(Supports fallback `PIN`)* | None |
| `TZ` | Timezone for the container processes and logs. | `UTC` |
| `ENABLE_TRANSLATION` | Enable the multi-language / translation selector in the navigation header (true/false). | `true` |
| `ENABLE_THEMES` | Enable the Super Metroid theme selector in the navigation header (true/false). | `true` |
| `ENABLE_PRINT` | Enable the print button in the navigation header (true/false). | `false` |
| `MAX_ATTEMPTS` | Maximum PIN auth attempts allowed before rate lockout. | `5` |
| `SNAKE_DATA_DIR` | Directory where runtime state is persisted. The high-score leaderboard is written atomically to `${SNAKE_DATA_DIR}/leaderboard.json` on every successful submission. Back this up; deleting it wipes the leaderboard. *(Supports fallback `DATA_DIR`)* | `./data` |
| `SNAKE_FRONTEND_DIR` | Path to the prebuilt Trunk SPA bundle (must contain `index.html`, `service-worker.js`, `Assets/manifest.json`). Override for custom builds. *(Supports fallback `FRONTEND_DIR`)* | `./frontend/dist` |

### Where scores live

Each successful `POST /api/leaderboard` writes the new top-10 list to `${SNAKE_DATA_DIR}/leaderboard.json` via an atomic temp-file + rename, so:

- A crash mid-write never leaves a half-written file
- Concurrent submitters can't lose data (a per-process mutex serialises the read-modify-write critical section)
- `SNAKE_DATA_DIR` is the only thing operators need to back up

On container startup Snake logs the resolved `leaderboard_file` path at `INFO` so the operator doesn't have to dig through code to find it.
