<p align="center">
  <a href="https://github.com/studio2201">
    <img src="assets/header.jpg" alt="studio2201 banner" width="100%">
  </a>
</p>

# <img src="assets/icon.png" width="32" height="32" valign="middle"> Snake

Self-hosted terminal-classic snake game with global highscore leaderboards.

## Quick Start (Docker)

Pull and run the official Docker container:
```bash
docker run -d \
  -p 4501:4501 \
  -v /path/to/appdata:/config \
  -e SNAKE_PIN=your_secret_pin \
  ghcr.io/studio2201/snake:latest
```

## Configuration

The service can be customized using the following container environment variables:

| Variable | Description | Default |
| :--- | :--- | :--- |
| `PORT` | The network port the web server binds to | `4501` |
| `SNAKE_PIN` | Security PIN code required for client authentication | (None) |
| `SNAKE_DATA_DIR` | Directory path where persistent data is stored | `/config` |
| `SNAKE_ALLOWED_ORIGINS` | CORS allowed origins list (comma-separated) | `*` |
| `TZ` | System timezone | `UTC` |
| `TRUST_PROXY` | Whether to honor upstream reverse proxy headers | `false` |
| `TRUSTED_PROXY_IPS` | Comma-separated CIDR/IP list of trusted reverse proxies | (None) |
| `LOG_DIR` | Directory where diagnostic log files are written | (Disabled) |
| `LOG_LEVEL` | Logging verbosity filter (`error`, `warn`, `info`, `debug`) | `info` |

## Administration Console (CLI & TUI)

Each container includes a built-in admin tool located in the system path as `snake`. To open the console, execute a shell inside the container:
```bash
docker exec -it <container-name> sh
```
Then, run `snake` to manage the application:
```bash
snake [command]
```
Running `snake` without arguments or running `snake tui` launches the interactive terminal user interface.

### CLI Commands

| Command | Aliases | Description |
| :--- | :--- | :--- |
| `tui` | (Default) | Launch the interactive arrow-key TUI panel dashboard |
| `doctor` | `check`, `diagnose` | Perform health diagnostics on directories, port, and databases |
| `start` | `up`, `run` | Launch the main web server process if stopped |
| `stop` | `down`, `terminate`, `close` | Gracefully shut down the web server (stops the container) |
| `restart` | `reload` | Perform a stop and start cycle on the server process |
| `status` | `info` | Display current network and settings configurations |
| `env` | | List the loaded environment variables for the service |
| `version` | `-v`, `--version` | Display the compiled version of the application |
| `data stats` | `data size`, `data info` | View storage file sizes and entry counts |
| `data list` | `data show`, `data view` | Show records (tasks, high scores, etc.) stored in the database |
| `data clear` | `data prune`, `data reset` | Reset the database to a clean, empty state (interactive) |
| `help` | `-h`, `--help` | Show the help information page |
