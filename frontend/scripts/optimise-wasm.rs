use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let dist_dir = Path::new("dist");
    if !dist_dir.is_dir() {
        eprintln!("dist/ directory not found. Run 'trunk build --release' first.");
        std::process::exit(1);
    }

    // Verify wasm-opt is installed
    if Command::new("wasm-opt").arg("--version").output().is_err() {
        eprintln!("wasm-opt not found on PATH. Install it via cargo or your package manager.");
        std::process::exit(1);
    }

    let mut total_saved = 0;

    if let Ok(entries) = fs::read_dir(dist_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("wasm") {
                if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                    if file_name.starts_with("frontend-") {
                        let orig_size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        if orig_size == 0 {
                            continue;
                        }

                        let tmp_path = path.with_extension("opt");
                        
                        println!("Optimizing {}...", file_name);
                        let status = Command::new("wasm-opt")
                            .args(&[
                                "-Oz", 
                                "--strip-debug", 
                                "--strip-producers", 
                                "--output", 
                                tmp_path.to_str().unwrap(), 
                                path.to_str().unwrap()
                            ])
                            .status();

                        if let Ok(status) = status {
                            if status.success() {
                                let opt_size = fs::metadata(&tmp_path).map(|m| m.len()).unwrap_or(0);
                                if opt_size > 0 && opt_size < orig_size {
                                    let saved = orig_size - opt_size;
                                    total_saved += saved;
                                    
                                    if fs::remove_file(&path).is_ok() && fs::rename(&tmp_path, &path).is_ok() {
                                        println!("  {}: {} -> {} bytes (-{} bytes)", file_name, orig_size, opt_size, saved);
                                    }
                                } else {
                                    let _ = fs::remove_file(&tmp_path);
                                    println!("  {}: no improvement; kept original", file_name);
                                }
                            } else {
                                eprintln!("  Error: wasm-opt failed for {}", file_name);
                            }
                        }
                    }
                }
            }
        }
    }

    if total_saved > 0 {
        println!("Total saved: {} bytes across all bundles", total_saved);
    }
}
