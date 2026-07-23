<h1 align="center">
  <img src="assets/icon.png?v=1.0.31" width="48" height="48" valign="middle"> Snake
</h1>

<p align="center">
  <b>Classic retro snake arcade game with multi-language support, global leaderboards, and custom themes written in Rust.</b>
</p>

---

### Instant One-Line Install (Docker Container)

Run the official zero-dependency container on port 4504:

```bash
docker run -d --name snake -p 4504:4504 -v /mnt/user/appdata/snake:/config ghcr.io/studio2201/snake:latest
```

Open your browser to `http://localhost:4504` to start slithering immediately.

---

### Environment Configuration

The backend service can be customized using the following environment variables:

| Variable | Description | Default |
| :--- | :--- | :---: |
| `PORT` | Network port the web server binds to | `4504` |
| `SNAKE_PIN` | Security PIN required for application access | *(Disabled)* |
| `SNAKE_DATA_DIR` | Directory path for persistent data and high scores | `/config` |
| `SNAKE_ALLOWED_ORIGINS` | CORS allowed origins list (comma-separated) | `*` |
| `TRUST_PROXY` | Honor reverse proxy headers (`X-Forwarded-For`) | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated CIDR list of trusted reverse proxies | *(None)* |
| `LOG_LEVEL` | Tracing filter (`error`, `warn`, `info`, `debug`) | `info` |

---

### Administration CLI & TUI Dashboard

Every container and package includes a built-in administration utility (`snake`).

Launch interactive TUI dashboard:
```bash
docker exec -it snake snake tui
```

System diagnostics and self-healing check:
```bash
docker exec -it snake snake doctor
```

CLI Command Reference:
- `snake tui` — Interactive terminal user interface.
- `snake doctor` — Diagnoses storage permissions, ports, and database health.
- `snake status` — Displays network configuration and security parameters.
- `snake data stats` — Shows storage utilization and leaderboard metrics.

---

### Architecture & Security

- **Axum Web Backend**: High-concurrency async HTTP runtime built on Tokio.
- **Yew WebAssembly Frontend**: Type-safe client bundle running natively in browser WASM runtime.
- **Strict Input & Path Sanitization**: Path canonicalization guards preventing directory traversal escapes.
- **Fail-Closed Security PIN Authentication**: Rate-limited brute force protection with automatic lockout timers.

---

### License

Distributed under the Apache 2.0 License. See [LICENSE](LICENSE) for details.

---

<p align="center">
  <a href="https://github.com/studio2201/snake">
    <img src="assets/corgi-footer.jpg" alt="studio2201 banner" width="100%">
  </a>
</p>
