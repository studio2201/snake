# Snake - Traditional Arcade Game

<p align="center">
  <img src="https://raw.githubusercontent.com/UberMetroid/unraid-templates/main/icons/snake.png?v=1.0.3" alt="Snake Logo" width="128" height="128">
</p>

<p align="center">
  <a href="https://www.buymeacoffee.com/ubermetroid" target="_blank"><img src="https://img.buymeacoffee.com/button-api/?text=Buy%20me%20a%20coffee&emoji=☕&slug=ubermetroid&button_colour=FFDD00&font_colour=000000&font_family=Cookie&outline_colour=000000&coffee_colour=ffffff" alt="Buy Me A Coffee" height="36"></a>
  &nbsp;&nbsp;&nbsp;&nbsp;
  <a href="bitcoin:3MbRFvcCJKBCjLDd2JhZcody29H2xHxv6T"><img src="https://img.shields.io/badge/Donate-Bitcoin-orange?logo=bitcoin&style=for-the-badge&logoColor=white" alt="Donate Bitcoin" height="36"></a>
</p>

Snake is a self-hosted traditional arcade-style snake game application designed for home servers and NAS systems. Built with a high-performance Rust (Axum/Tokio) backend and a WebAssembly (Yew) frontend.

---

## Key Features

*   **Traditional Arcade Loop**: Classic gameplay with grid rendering, score tracking, and persistent high scores.
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
      - 4407:4407
    volumes:
      - ./data:/app/data
    environment:
      - PORT=4407
      - SITE_TITLE=Snake
      - BASE_URL=http://localhost:4407
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

3. Open your browser and navigate to `http://localhost:4407`.

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

## Configuration Options

Configure these settings inside your Docker Compose environment or container environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | The port number the backend HTTP server will bind to inside the container. | `4407` |
| `SITE_TITLE` | Custom website title rendered in navigation headers, browser tabs, and PWA manifest. | `Snake` |
| `BASE_URL` | Application base URL. | `http://localhost:4407` |
| `ALLOWED_ORIGINS` | Comma-separated list of allowed HTTP request origins (CORS filter). Use `*` to allow all origins. | `*` |
| `SNAKE_PIN` | Optional 4–10 digit PIN (numerical only) to lock access to the interface. Leave empty for public mode. *(Supports fallback `PIN`)* | None |
| `TZ` | Timezone for the container processes and logs. | `UTC` |
| `ENABLE_TRANSLATION` | Enable the multi-language / translation selector in the navigation header (true/false). | `false` |
| `ENABLE_THEMES` | Enable the Super Metroid theme selector in the navigation header (true/false). | `true` |
| `ENABLE_PRINT` | Enable the print button in the navigation header (true/false). | `false` |
| `MAX_ATTEMPTS` | Maximum PIN auth attempts allowed before rate lockout. | `5` |
