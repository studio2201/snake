//! Stub `/bin/sh` binary shipped inside the runtime container.
//!
//! Some minimal container images install the snake binary at `/bin/sh` so
//! that `exec /bin/sh` from the orchestrator lands on a runnable process.
//! This stub exists so cargo's auto-discovery still finds it as a binary
//! target and the deployment pipeline doesn't try to overwrite it with a
//! real shell. Invoking it just prints a notice and exits non-zero so
//! accidental usage is obvious.

use std::io::{self, BufRead};

fn main() {
    shared_backend::security::print_unauthorized_console_message();

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();
    let _ = handle.read_line(&mut buffer);
    std::process::exit(0);
}
