# Snake - Traditional Arcade Game

[![CI](https://github.com/UberMetroid/snake/actions/workflows/ci.yml/badge.svg)](https://github.com/UberMetroid/snake/actions/workflows/ci.yml)

<p align="center">
  <img src="https://raw.githubusercontent.com/UberMetroid/snake/main/assets/logo.png?v=1.0.30" alt="Snake Logo" width="128" height="128">
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
