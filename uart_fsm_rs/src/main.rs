//! UART FSM Demo Application
//! Demonstrates the UART FSM and parser in various modes.
//! Supports std and no_std builds with multiple demo modes.
//! Uses the harness, fsm, parser, and log modules.
//! Intended for testing and demonstration purposes.
//! Provides a main function for std builds only.
//! Includes demo modes: simulated input, file input, and tty input.
//! Uses conditional compilation for feature management.
//! Employs a ring buffer for input byte management.
//! Utilizes the FSM and parser modules for processing.
//! Captures logs using the log module.
//! Supports byte-stuffing via feature flag.
//! Includes mode sanity checks to ensure correct feature combinations.
//! Provides a no_std stub for non-std builds.

// Common modules
mod fsm;
mod harness;
mod log;
mod parser;
mod ringbuf;

// --- Mode sanity checks (std + exactly one mode) ----------------------------

#[cfg(all(
    feature = "std",
    any(
        all(feature = "demo_sim", feature = "demo_file"),
        all(feature = "demo_sim", feature = "demo_tty"),
        all(feature = "demo_file", feature = "demo_tty")
    )
))]
compile_error!("Enable exactly ONE of: demo_sim, demo_file, demo_tty.");

// --- no_std stub ------------------------------------------------------------

#[cfg(not(feature = "std"))]
fn main() {
    // No-std build: binary does nothing (library is still no_std-capable).
    // Alternatively, you could:
    // compile_error!("This binary requires the `std` feature.");
}

// --- demo_sim ---------------------------------------------------------------

#[cfg(all(feature = "std", feature = "demo_sim"))]
use harness::{make_frame, run_demo};

#[cfg(all(feature = "std", feature = "demo_sim"))]
fn main() {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&make_frame(0x01, &[]));
    bytes.extend_from_slice(&make_frame(0x03, b"hi"));
    bytes.extend_from_slice(&make_frame(0x02, &[]));

    println!("--- DEMO 1 ---");
    run_demo(&bytes, &[3, 1, 2, 5], 3000);

    let demo2: [u8; 9] = [0x13, 0x00, 0xAA, 0x01, 0xFF, 0xAA, 0x02, 0xFF, 0xFE];
    println!("--- DEMO 2 ---");
    run_demo(&demo2, &[2, 2, 5], 200);
}

// --- demo_file --------------------------------------------------------------

#[cfg(all(feature = "std", feature = "demo_file"))]
use std::fs;

#[cfg(all(feature = "std", feature = "demo_file"))]
use harness::run_demo;

#[cfg(all(feature = "std", feature = "demo_file"))]
fn main() {
    let bytes = fs::read("stream.bin").expect("missing stream.bin");
    println!("--- Running from stream.bin ({} bytes) ---", bytes.len());
    run_demo(&bytes, &[3, 1, 2, 5], 3000);
}

// --- demo_tty ---------------------------------------------------------------

#[cfg(all(feature = "std", feature = "demo_tty"))]
use std::{env, fs::File, io::Read, thread, time::Duration};

#[cfg(all(feature = "std", feature = "demo_tty"))]
use harness::run_demo;

#[cfg(all(feature = "std", feature = "demo_tty"))]
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --features std,demo_tty -- <tty_path>");
        eprintln!("Example: cargo run --features std,demo_tty -- /tmp/ttyV0");
        return;
    }
    let path = &args[1];
    let mut f = File::open(path).expect("failed to open tty path");
    eprintln!("Reading from device: {}", path);

    loop {
        let mut tmp = [0u8; 256];
        match f.read(&mut tmp) {
            Ok(0) => {
                // No data: brief sleep to avoid busy-looping
                thread::sleep(Duration::from_millis(10));
            }
            Ok(n) => {
                run_demo(&tmp[..n], &[4, 4, 4], 1000);
            }
            Err(e) => {
                eprintln!("read error: {:?}", e);
                break;
            }
        }
    }
}
