# Snake — Traditional Arcade Game <img src="https://raw.githubusercontent.com/UberMetroid/snake/main/assets/logo.png?v=1.0.37" width="48" height="48" alt="snake logo" align="right">

Snake is a self-hosted traditional arcade-style snake game application designed for home servers and NAS systems. Built with a high-performance Rust (Axum/Tokio) backend and a WebAssembly (Yew) frontend.

---

## 🏛️ Architecture & Stack
*   **Frontend**: Yew (WASM)
*   **Backend**: Axum (Rust) / Tokio
*   **Deployment**: Nix-built Container / Unraid native / Docker Compose

---

## 🟢 Key Features
*   **Traditional Arcade Loop**: Classic gameplay with grid rendering, score tracking, and persistent high scores.
*   **Gold Food Mode**: Flashing Gold Food that expires in 5 seconds (with a dynamic visual countdown bar) and awards +30 points.
*   **High Score Leaderboard**: Persists the Top 10 player scores using simple file-based JSON storage (`leaderboard.json`).
*   **Sleek Neon Theme**: Dark retro-futuristic styling matching the Super Metroid theme design system.
*   **Mobile-Friendly D-Pad**: Integrated touch/D-Pad controls overlay for easy play on mobile and tablets.
*   **Access PIN Security**: Lock down the interface with an optional numerical PIN for absolute privacy.

---

## 💾 Deployment & Installation

### Docker Compose
Create a `docker-compose.yml` file with the following service definition:

```yaml
services:
  snake:
    image: ubermetroid/snake:latest
    container_name: snake
    restart: unless-stopped
    volumes:
      - ${SNAKE_DATA_PATH:-./data}:/app/data
    ports:
      - ${PORT:-4501}:4501
    environment:
      PORT: 4501
      BASE_URL: ${SNAKE_BASE_URL:-http://localhost:4501}
      SNAKE_PIN: ${SNAKE_PIN:-}
      ALLOWED_ORIGINS: ${SNAKE_ALLOWED_ORIGINS:-*}
      MAX_ATTEMPTS: ${MAX_ATTEMPTS:-5}
      SITE_TITLE: ${SNAKE_SITE_TITLE:-Snake}
      ENABLE_TRANSLATION: ${ENABLE_TRANSLATION:-true}
      ENABLE_THEMES: ${ENABLE_THEMES:-true}
      ENABLE_PRINT: ${ENABLE_PRINT:-true}
      TZ: ${TZ:-UTC}
```

---

## ⚙️ Configuration Options

| Environment Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | The port number the backend HTTP server will bind to inside the container. | `4501` |
| `SITE_TITLE` | Custom website title rendered in navigation headers, browser tabs, and PWA manifest. | `Snake` |
| `BASE_URL` | Application base URL. | `http://localhost:4501` |
| `ALLOWED_ORIGINS` | Comma-separated list of allowed HTTP request origins (CORS filter). | `*` |
| `SNAKE_PIN` | Optional numerical PIN to lock access to the interface. | None |
| `TZ` | Timezone for the container processes and logs. | `UTC` |
| `ENABLE_TRANSLATION` | Enable the multi-language / translation selector in the navigation header. | `true` |
| `ENABLE_THEMES` | Enable the Super Metroid theme selector in the navigation header. | `true` |
| `ENABLE_PRINT` | Enable the print button in the navigation header. | `true` |
| `MAX_ATTEMPTS` | Maximum PIN auth attempts allowed before rate lockout. | `5` |
| `SNAKE_DATA_DIR` | Directory where runtime state is persisted (`leaderboard.json`). | `./data` |
| `SNAKE_FRONTEND_DIR` | Path to the prebuilt Trunk SPA bundle. | `./frontend/dist` |

---

## 🛠️ Local Development

Ensure you have the Rust toolchain and Trunk installed.

```bash
# 1. Run workspace tests
cargo test

# 2. Run clippy workspace checks
cargo clippy --workspace --all-targets

# 3. Start frontend Yew dev server (from frontend/)
cd frontend && trunk serve

# 4. Start backend Axum server (from backend/)
cd backend && cargo run
```

### Nix Flake Run
You can also run or install Snake directly using Nix flakes:
```bash
# Run the application directly
nix run github:UberMetroid/snake --impure
```

---

## 📄 License
Licensed under the [Apache License, Version 2.0](LICENSE). Copyright 2026 UberMetroid.
