use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

// Global settings configured per app
const APP_NAME: &str = "Snake";
const ENV_PREFIX: &str = "SNAKE";
const DB_FILE_NAME: &str = "leaderboard.json";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        handle_cli_args(&args);
    } else {
        run_tui();
    }
}

struct RawMode;

impl RawMode {
    fn enable() -> Self {
        let mut cmd = Command::new("stty");
        cmd.arg("raw").arg("-echo");
        let _ = cmd.status();
        // Hide cursor
        print!("\x1B[?25l");
        let _ = io::stdout().flush();
        RawMode
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let mut cmd = Command::new("stty");
        cmd.arg("-raw").arg("echo");
        let _ = cmd.status();
        // Show cursor and reset style
        print!("\x1B[?25h\x1B[0m");
        let _ = io::stdout().flush();
    }
}

fn get_port() -> u16 {
    env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(match APP_NAME {
            "Beam" => 4401,
            "Defend" => 4504,
            "Grid" => 4405,
            "Pad" => 4402,
            "Pulse" => 4406,
            "Rustle" => 4502,
            "Scan" => 4503,
            "Snake" => 4501,
            "Todo" => 4403,
            "Trace" => 4404,
            _ => 8080,
        })
}

fn get_data_dir() -> PathBuf {
    let key1 = format!("{}_DATA_DIR", ENV_PREFIX);
    let key2 = format!("{}_DATA_PATH", ENV_PREFIX);
    let dir = env::var(&key1)
        .or_else(|_| env::var(&key2))
        .or_else(|_| env::var("DATA_DIR"))
        .unwrap_or_else(|_| "data".to_string());
    PathBuf::from(dir)
}

fn get_pin() -> Option<String> {
    let key = format!("{}_PIN", ENV_PREFIX);
    env::var(&key).or_else(|_| env::var("PIN")).ok()
}

fn get_allowed_origins() -> String {
    let key = format!("{}_ALLOWED_ORIGINS", ENV_PREFIX);
    env::var(&key).or_else(|_| env::var("ALLOWED_ORIGINS")).unwrap_or_else(|_| "*".to_string())
}

fn print_status() {
    println!("=== {} Status ===", APP_NAME);
    println!("Port: {}", get_port());
    println!("Data Directory: {:?}", get_data_dir());
    println!("PIN Enabled: {}", get_pin().is_some());
    println!("Allowed Origins: {}", get_allowed_origins());
    println!("Timezone: {}", env::var("TZ").unwrap_or_else(|_| "UTC".to_string()));
}

fn print_env() {
    println!("=== {} Environment Variables ===", APP_NAME);
    for (key, val) in env::vars() {
        if key.starts_with(ENV_PREFIX) || key == "PORT" || key == "DATA_DIR" || key == "TZ" || key == "ALLOWED_ORIGINS" {
            println!("{}={}", key, val);
        }
    }
}

fn print_help() {
    println!("Usage: sh [command]");
    println!();
    println!("Commands:");
    println!("  version        Show application version");
    println!("  status         Show service status and configurations");
    println!("  env            Print loaded configuration environment variables");
    println!("  doctor         Perform system health and permission diagnostics");
    println!("  start          Launch backend server process if stopped");
    println!("  end / close    Gracefully terminate the backend server process");
    println!("  data stats     Show database/storage file statistics");
    println!("  data list      List items stored in the database");
    println!("  data clear     Delete the database to reset application state");
    println!("  help           Show this help menu");
    println!();
    println!("Run without arguments to launch the interactive TUI console.");
}

fn get_db_file_path() -> Option<PathBuf> {
    if DB_FILE_NAME.is_empty() {
        None
    } else {
        Some(get_data_dir().join(DB_FILE_NAME))
    }
}

fn run_doctor() {
    println!("=== {} Doctor Diagnostics ===", APP_NAME);
    let mut failures = 0;

    // 1. Check data directory permissions
    let data_dir = get_data_dir();
    print!("Checking data directory {:?}... ", data_dir);
    let _ = io::stdout().flush();
    if let Err(e) = fs::create_dir_all(&data_dir) {
        println!("\x1B[1;31m[FAIL] (Cannot create directory: {})\x1B[0m", e);
        failures += 1;
    } else {
        // Test write permission
        let test_file = data_dir.join(".doctor_test");
        if fs::write(&test_file, b"test").is_err() {
            println!("\x1B[1;31m[FAIL] (Directory is not writable)\x1B[0m");
            failures += 1;
        } else {
            let _ = fs::remove_file(&test_file);
            println!("\x1B[1;32m[PASS]\x1B[0m");
        }
    }

    // 2. Check database integrity
    if let Some(path) = get_db_file_path() {
        print!("Checking database file {:?}... ", path);
        let _ = io::stdout().flush();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                        println!("\x1B[1;32m[PASS]\x1B[0m");
                    } else {
                        println!("\x1B[1;31m[FAIL] (Corrupted JSON content)\x1B[0m");
                        failures += 1;
                    }
                }
                Err(e) => {
                    println!("\x1B[1;31m[FAIL] (Cannot read file: {})\x1B[0m", e);
                    failures += 1;
                }
            }
        } else {
            println!("\x1B[1;33m[WARN] (File does not exist yet; will be created on start)\x1B[0m");
        }
    }

    // 3. Check port availability
    let port = get_port();
    print!("Checking web port {} availability... ", port);
    let _ = io::stdout().flush();
    if std::net::TcpListener::bind(format!("0.0.0.0:{}", port)).is_ok() {
        println!("\x1B[1;32m[PASS] (Port is free)\x1B[0m");
    } else {
        println!("\x1B[1;33m[WARN] (Port is already bound; server is likely running)\x1B[0m");
    }

    // 4. Check PIN setup
    print!("Checking Security PIN setup... ");
    let _ = io::stdout().flush();
    if get_pin().is_some() {
        println!("\x1B[1;32m[SET] (PIN is configured)\x1B[0m");
    } else {
        println!("\x1B[1;33m[WARN] (No PIN configured; authentication is disabled)\x1B[0m");
    }

    println!();
    if failures == 0 {
        println!("\x1B[1;32mDoctor report: System is healthy and configured correctly.\x1B[0m");
    } else {
        println!("\x1B[1;31mDoctor report: Found {} error(s). Please check logs or configurations.\x1B[0m", failures);
    }
}

fn run_start() {
    println!("Starting {} server...", APP_NAME);
    let port = get_port();
    
    // Check if server is already running
    if std::net::TcpListener::bind(format!("0.0.0.0:{}", port)).is_err() {
        println!("\x1B[1;33mWarning: Port {} is already in use. Server is likely already running.\x1B[0m", port);
        println!("If you are inside a container, the server runs automatically.");
        return;
    }

    // Attempt to execute the backend server binary
    let server_path = if Path::new("/app/server").exists() {
        "/app/server"
    } else if Path::new("./target/release/server").exists() {
        "./target/release/server"
    } else if Path::new("./server").exists() {
        "./server"
    } else {
        "server"
    };

    println!("Spawning server process: {}", server_path);
    let mut child = match Command::new(server_path).spawn() {
        Ok(c) => c,
        Err(e) => {
            println!("\x1B[1;31mError: Failed to spawn server binary: {}\x1B[0m", e);
            return;
        }
    };

    println!("Server process started with PID {}.", child.id());
    println!("Console will now exit.");
}

fn run_end() {
    println!("Stopping {} server process...", APP_NAME);
    
    let mut stopped = false;
    
    // Attempt graceful shutdown via pkill
    if Command::new("pkill").arg("-15").arg("server").status().is_ok() {
        println!("Sent SIGTERM shutdown signal to server processes.");
        stopped = true;
    } else {
        // Fallback: kill container init process (PID 1)
        if Command::new("kill").arg("-15").arg("1").status().is_ok() {
            println!("Sent SIGTERM to container init process (PID 1).");
            stopped = true;
        }
    }
    
    if !stopped {
        println!("\x1B[1;31mError: Could not stop server. Process command 'pkill' / 'kill' failed.\x1B[0m");
    }
}

fn print_data_stats() {
    println!("=== {} Data Statistics ===", APP_NAME);
    let data_dir = get_data_dir();
    println!("Database Directory: {:?}", data_dir);

    if APP_NAME == "Beam" {
        let uploads_dir = data_dir.parent().unwrap_or(&data_dir).join("uploads");
        println!("Uploads Directory: {:?}", uploads_dir);
        if let Ok(entries) = fs::read_dir(&uploads_dir) {
            let mut count = 0;
            let mut total_size = 0;
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        count += 1;
                        total_size += meta.len();
                    }
                }
            }
            println!("Uploaded Files: {}", count);
            println!("Total Space Used: {} bytes", total_size);
        } else {
            println!("Uploads Directory: not found or unreadable");
        }
    } else if let Some(path) = get_db_file_path() {
        if path.exists() {
            if let Ok(meta) = fs::metadata(&path) {
                println!("Database File: {:?}", path);
                println!("File Size: {} bytes", meta.len());
                // Try reading count of items
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                        if value.is_object() {
                            println!("Number of database keys: {}", value.as_object().unwrap().keys().len());
                        } else if value.is_array() {
                            println!("Number of database entries: {}", value.as_array().unwrap().len());
                        }
                    }
                }
            }
        } else {
            println!("Database file not found at {:?}", path);
        }
    } else {
        println!("This application is stateless and does not maintain a JSON database.");
    }
}

fn list_data_contents() {
    println!("=== {} Data Contents ===", APP_NAME);
    let data_dir = get_data_dir();
    if APP_NAME == "Beam" {
        let uploads_dir = data_dir.parent().unwrap_or(&data_dir).join("uploads");
        if let Ok(entries) = fs::read_dir(&uploads_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Ok(meta) = entry.metadata() {
                    if meta.is_file() {
                        println!(" - {} ({} bytes)", name, meta.len());
                    }
                }
            }
        } else {
            println!("No uploads found.");
        }
        return;
    }

    let path = match get_db_file_path() {
        Some(p) => p,
        None => {
            println!("This application is stateless and has no database contents to list.");
            return;
        }
    };

    if !path.exists() {
        println!("Database file not found.");
        return;
    }

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            println!("Failed to read database file.");
            return;
        }
    };

    match APP_NAME {
        "Todo" => {
            if let Ok(lists) = serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(&content) {
                for (list_name, items) in lists {
                    println!("List: {}", list_name);
                    if let Some(arr) = items.as_array() {
                        for item in arr {
                            let text = item["text"].as_str().unwrap_or("");
                            let completed = item["completed"].as_bool().unwrap_or(false);
                            let marker = if completed { "[x]" } else { "[ ]" };
                            println!("  {} {}", marker, text);
                        }
                    }
                }
            } else {
                println!("Failed to parse todos database.");
            }
        }
        "Grid" => {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(boards) = value["boards"].as_object() {
                    for (board_name, board) in boards {
                        println!("Board: {}", board_name);
                        if let Some(tasks) = board["tasks"].as_object() {
                            for (task_id, task) in tasks {
                                let title = task["title"].as_str().unwrap_or("");
                                let column = task["column"].as_str().unwrap_or("unknown");
                                println!("  - [{}] {} ({})", task_id, title, column);
                            }
                        }
                    }
                }
            } else {
                println!("Failed to parse grid tasks database.");
            }
        }
        "Pad" => {
            if let Ok(pads) = serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(&content) {
                for (pad_id, pad) in pads {
                    let text = pad["content"].as_str().unwrap_or("");
                    println!("Pad ID: {} ({} chars)", pad_id, text.len());
                }
            } else {
                println!("Failed to parse pads database.");
            }
        }
        "Defend" | "Snake" | "Scan" => {
            if let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&content) {
                println!("Top Leaderboard Scores:");
                for (idx, entry) in entries.iter().enumerate() {
                    let name = entry["name"].as_str().unwrap_or("Anonymous");
                    let score = entry["score"].as_u64().unwrap_or(0);
                    println!("  {}. {} - {} pts", idx + 1, name, score);
                }
            } else {
                println!("Failed to parse leaderboard.");
            }
        }
        _ => {
            println!("{}", content);
        }
    }
}

fn clear_data() {
    print!("Are you sure you want to clear/reset the database? (y/N): ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim().to_lowercase();
        if trimmed == "y" || trimmed == "yes" {
            if APP_NAME == "Beam" {
                let uploads_dir = get_data_dir().parent().unwrap_or(&get_data_dir()).join("uploads");
                let _ = fs::remove_dir_all(&uploads_dir);
                let _ = fs::create_dir_all(&uploads_dir);
                println!("Uploads directory cleared.");
            } else if let Some(path) = get_db_file_path() {
                let empty_content = match APP_NAME {
                    "Defend" | "Snake" | "Scan" => "[]",
                    _ => "{}",
                };
                if fs::write(&path, empty_content).is_ok() {
                    println!("Database cleared successfully.");
                } else {
                    println!("Failed to write empty database.");
                }
            } else {
                println!("This application is stateless; no database to clear.");
            }
        } else {
            println!("Clear cancelled.");
        }
    }
}

fn handle_cli_args(args: &[String]) {
    match args[1].as_str() {
        "version" | "--version" | "-v" => {
            println!("{} version: v{}", APP_NAME, env!("CARGO_PKG_VERSION"));
        }
        "status" | "--status" => {
            print_status();
        }
        "env" | "--env" => {
            print_env();
        }
        "doctor" => {
            run_doctor();
        }
        "start" => {
            run_start();
        }
        "end" | "close" => {
            run_end();
        }
        "data" => {
            if args.len() > 2 {
                match args[2].as_str() {
                    "stats" => print_data_stats(),
                    "list" => list_data_contents(),
                    "clear" => clear_data(),
                    _ => print_help(),
                }
            } else {
                print_help();
            }
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        _ => {
            println!("Unknown argument. Use --help for usage.");
            std::process::exit(1);
        }
    }
}

fn run_tui() {
    let _raw = RawMode::enable();
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    let mut menu_selection = 0;
    loop {
        // Draw TUI
        print!("\x1B[2J\x1B[1;1H"); // Clear screen
        println!("\x1B[1;36m========================================================\x1B[0m");
        println!("\x1B[1;37m   {:^48} \x1B[0m", format!("{} Administration Console", APP_NAME));
        println!("\x1B[1;36m========================================================\x1B[0m");
        println!("  \x1B[1mStatus\x1B[0m:          \x1B[32mRunning\x1B[0m");
        println!("  \x1B[1mWeb Port\x1B[0m:        {}", get_port());
        println!("  \x1B[1mData Directory\x1B[0m:  {:?}", get_data_dir());
        let pin_status = if get_pin().is_some() { "\x1B[32mEnabled (PIN Auth active)\x1B[0m" } else { "\x1B[33mDisabled (No authentication required)\x1B[0m" };
        println!("  \x1B[1mSecurity PIN\x1B[0m:    {}", pin_status);
        println!("\x1B[1;36m--------------------------------------------------------\x1B[0m");
        println!("  Select an option:");
        println!();
        
        let options = [
            "Show Full Configuration Settings",
            "Run System Diagnostics (Doctor)",
            "View Database Statistics",
            "List Database/Files Content",
            "Reset / Clear Application State",
            "Exit Console"
        ];
        
        for (i, opt) in options.iter().enumerate() {
            if i == menu_selection {
                println!("  \x1B[1;36m-> [ {} ] {}\x1B[0m", i + 1, opt);
            } else {
                println!("     [ {} ] {}", i + 1, opt);
            }
        }
        println!();
        println!("\x1B[1;36m--------------------------------------------------------\x1B[0m");
        println!("  Use \x1B[1m[Up/Down Arrow]\x1B[0m to navigate, \x1B[1m[Enter]\x1B[0m to select.");
        let _ = stdout.flush();

        // Read single key
        let mut key_buf = [0u8; 3];
        let bytes_read = match stdin.read(&mut key_buf) {
            Ok(n) => n,
            Err(_) => break,
        };

        if bytes_read == 1 {
            match key_buf[0] {
                13 | 10 => { // Enter
                    if menu_selection == 5 {
                        break;
                    }
                    execute_tui_option(menu_selection);
                }
                b'1' => execute_tui_option(0),
                b'2' => execute_tui_option(1),
                b'3' => execute_tui_option(2),
                b'4' => execute_tui_option(3),
                b'5' => execute_tui_option(4),
                b'6' => break,
                b'q' => break,
                _ => {}
            }
        } else if bytes_read == 3 && key_buf[0] == 27 && key_buf[1] == 91 { // ANSI escape sequence
            match key_buf[2] {
                65 => { // Up Arrow
                    if menu_selection > 0 {
                        menu_selection -= 1;
                    } else {
                        menu_selection = 5;
                    }
                }
                66 => { // Down Arrow
                    if menu_selection < 5 {
                        menu_selection += 1;
                    } else {
                        menu_selection = 0;
                    }
                }
                _ => {}
            }
        }
    }
}

fn execute_tui_option(index: usize) {
    // Temporarily restore cooked terminal mode to show output and read confirmations
    let mut cmd = Command::new("stty");
    cmd.arg("-raw").arg("echo");
    let _ = cmd.status();
    // Show cursor
    print!("\x1B[?25h\x1B[0m");
    let _ = io::stdout().flush();

    print!("\x1B[2J\x1B[1;1H"); // Clear screen
    match index {
        0 => {
            print_status();
            println!();
            print_env();
        }
        1 => {
            run_doctor();
        }
        2 => {
            print_data_stats();
        }
        3 => {
            list_data_contents();
        }
        4 => {
            clear_data();
        }
        _ => {}
    }

    println!("\nPress \x1B[1;36m[Enter]\x1B[0m to return to menu...");
    let mut discard = String::new();
    let _ = io::stdin().read_line(&mut discard);

    // Re-enable raw mode
    let mut cmd = Command::new("stty");
    cmd.arg("raw").arg("-echo");
    let _ = cmd.status();
    // Hide cursor
    print!("\x1B[?25l");
    let _ = io::stdout().flush();
}
