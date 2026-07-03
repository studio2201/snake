# Snake - Traditional Arcade Game

[![CI](https://github.com/UberMetroid/snake/actions/workflows/ci.yml/badge.svg)](https://github.com/UberMetroid/snake/actions/workflows/ci.yml)

<p align="center">
  <img src="https://raw.githubusercontent.com/UberMetroid/snake/main/assets/logo.png?v=1.0.34" alt="Snake Logo" width="128" height="128">
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

1. Create a `docker-compose.yml` file:

```yaml
version: '3'
services:
  snake:
    image: ubermetroid/snake:latest
    container_name: snake
    restart: unless-stopped
    ports:
      - 4501:4501
    volumes:
      - ./data:/app/data
    environment:
      - PORT=4501
      - SITE_TITLE=Snake
      - BASE_URL=http://localhost:4501
      - ALLOWED_ORIGINS=*
      - SNAKE_PIN=1234
      - TZ=UTC
      - ENABLE_TRANSLATION=false
      - ENABLE_THEMES=true
      - ENABLE_PRINT=false
```

2. Run the container:

```bash
docker compose up -d
```

3. Open your browser and navigate to `http://localhost:4501`.

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
| `ENABLE_TRANSLATION` | Enable the multi-language / translation selector in the navigation header (true/false). | `false` |
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
