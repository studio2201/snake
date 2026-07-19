# <img src="assets/icon.png" width="32" height="32" valign="middle"> Snake

Traditional arcade snake game.

## Configuration

This application can be configured using the following environment variables:

| Environment Variable | Description | Default |
|---|---|---|
| `PORT` | The port number the backend HTTP server will bind to. | `4501` |
| `SNAKE_PIN` | Optional PIN to restrict access to the application interface. | (None) |
| `SNAKE_DATA_DIR` / `DATA_DIR` | Path to the directory where application data and database files are stored. | `data` / `/config` |
| `SNAKE_ALLOWED_ORIGINS` / `ALLOWED_ORIGINS` | Comma-separated list of allowed CORS origins, or `*` to allow all. | `*` |
| `TZ` | Timezone configuration used for logs and local container events. | `UTC` |
| `TRUST_PROXY` | Set to `true` if the backend is hosted behind a reverse proxy (e.g. Nginx, Cloudflare). | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated list of trusted upstream proxy IP addresses or CIDR blocks. | (None) |
| `LOG_DIR` | Directory path where backend application logs will be written. | (None) |
| `LOG_LEVEL` | Logging verbosity level (`trace`, `debug`, `info`, `warn`, `error`). | `info` |

## Administration Console (CLI & TUI)

The application includes an administration console that can be run interactively as a Terminal User Interface (TUI) or non-interactively via command-line arguments.

### Launching the interactive TUI
To start the interactive TUI dashboard, run the shell helper with no arguments:
```bash
/app/sh
```
Inside the TUI menu, you can navigate options using the **Up/Down Arrow Keys** and select options with **Enter**.

### CLI Commands & Aliases
You can also run commands directly from the command line:
```bash
/app/sh [command]
```

The supported commands and their aliases include:
- `doctor` (aliases: `check`, `diagnose`): Perform system health check and write permission checks on data folders.
- `start` (aliases: `up`, `run`): Start the application backend server process.
- `stop` (aliases: `down`, `end`, `close`): Gracefully stop the application backend server.
- `restart` (alias: `reload`): Restart the application backend server.
- `data stats` (alias: `data size`): Display current storage and database space usage statistics.
- `data list` (alias: `data show`): View list of records stored in the database.
- `data clear` (aliases: `data prune`, `data reset`): Completely delete application database/storage to reset state.
- `version`: Show the current version of the application.
- `status` (alias: `info`): Show running status of background processes.
- `env`: List all configured environment variables.

## Quick Start

### Self-Hosting (Docker)
Pull and run the official Docker container:
```bash
docker run -d -p 4501:4501 -v /path/to/appdata:/app/data ghcr.io/studio2201/snake:latest
```

### Local Development
To run this application locally from source:

1. **Prerequisites**: Ensure you have Rust and `trunk` (for frontend building) installed:
   ```bash
   cargo install --locked trunk
   rustup target add wasm32-unknown-unknown
   ```
2. **Build and Run Frontend**:
   ```bash
   cd frontend
   trunk build
   ```
3. **Build and Run Backend**:
   ```bash
   cd backend
   cargo run
   ```
