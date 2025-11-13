//! UART FSM Demo Binaries (feature-chosen)
//!
//! Exactly one of the following demo modes should be enabled at a time:
//! - `demo_sim`  : run a built-in simulation sequence
//! - `demo_file` : read frames from `stream.bin`
//! - `demo_tty`  : read frames from a TTY/PTY path passed on the CLI
//! - `demo_c`    : run the C hybrid demo via the linked static lib / external binary
//!
//! If none is selected (but `std` is), we show a help message.

#[cfg(all(
    feature = "std",
    any(
        all(feature = "demo_sim", feature = "demo_file"),
        all(feature = "demo_sim", feature = "demo_tty"),
        all(feature = "demo_file", feature = "demo_tty"),
        all(feature = "demo_c", feature = "demo_sim"),
        all(feature = "demo_c", feature = "demo_file"),
        all(feature = "demo_c", feature = "demo_tty")
    )
))]
compile_error!("Enable exactly ONE of: demo_sim, demo_file, demo_tty, demo_c.");

#[cfg(not(feature = "std"))]
fn main() {}

/// ------------------------------------------------------------
/// demo_sim: Run two built-in demos (valid flow + noise/reset)
/// ------------------------------------------------------------
#[cfg(all(feature = "std", feature = "demo_sim"))]
fn main() {
    use uart_fsm_rs::harness::{make_frame, run_demo};

    // DEMO 1: START -> PING("hi") -> STOP
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&make_frame(0x01, &[]));
    bytes.extend_from_slice(&make_frame(0x03, b"hi"));
    bytes.extend_from_slice(&make_frame(0x02, &[]));

    println!("--- DEMO 1 ---");
    run_demo(&bytes, &[3, 1, 2, 5], 3000);

    // DEMO 2: noise + invalids -> Error, then valid RESET to recover
    let mut demo2: Vec<u8> = vec![0x13, 0x00, 0xAA, 0x01, 0xFF];
    demo2.extend_from_slice(&make_frame(0xFF, &[]));
    println!("--- DEMO 2 ---");
    run_demo(&demo2, &[2, 2, 5], 200);
}

/// ------------------------------------------------------------
/// demo_file: Read frames from `stream.bin`
/// ------------------------------------------------------------
#[cfg(all(feature = "std", feature = "demo_file"))]
fn main() {
    use std::fs;
    use uart_fsm_rs::harness::run_demo;

    let bytes = fs::read("stream.bin").expect("missing stream.bin");
    println!("--- Running from stream.bin ({} bytes) ---", bytes.len());
    run_demo(&bytes, &[3, 1, 2, 5], 3000);
}

/// ------------------------------------------------------------
/// demo_tty: Read frames from a TTY/PTY (path on CLI)
/// ------------------------------------------------------------
#[cfg(all(feature = "std", feature = "demo_tty"))]
fn main() {
    use std::{env, fs::File, io::Read, thread, time::Duration};
    use uart_fsm_rs::harness::run_demo;

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
            Ok(0) => thread::sleep(Duration::from_millis(10)),
            Ok(n) => run_demo(&tmp[..n], &[4, 4, 4], 1000),
            Err(e) => {
                eprintln!("read error: {:?}", e);
                break;
            }
        }
    }
}

/// ------------------------------------------------------------
/// demo_c: Run C hybrid demo (linked static lib) or stdin mode
/// ------------------------------------------------------------
#[cfg(all(feature = "std", feature = "demo_c"))]
fn main() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use uart_fsm_rs::{run_c_demos, run_c_stdin};

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--stdin" {
        run_c_stdin();
        return;
    }

    println!("🦀 Running UART-FSM hybrid demo (Rust + C)...");
    run_c_demos();

    println!("\n🦀 Sending a custom raw frame from Rust...");
    // START frame (LEN=2, TYPE=0x01, CHK=~(2+1)=0xFC)
    let frame: [u8; 4] = [0xAA, 0x02, 0x01, 0xFC];

    let mut child = Command::new("../uart_fsm_c/uart_fsm_demo")
        .arg("--stdin")
        .stdin(Stdio::piped())
        .spawn()
        .expect("failed to spawn C demo");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&frame).expect("write to C stdin");
    }

    let _ = child.wait();
}

/// ------------------------------------------------------------
/// default (std, no demo_* enabled): print usage
/// ------------------------------------------------------------
#[cfg(all(
    feature = "std",
    not(any(
        feature = "demo_sim",
        feature = "demo_file",
        feature = "demo_tty",
        feature = "demo_c"
    ))
))]
fn main() {
    println!("UART-FSM: no demo mode selected. Use one of:");
    println!("  --features \"demo_sim\"   # run simulated frames");
    println!("  --features \"demo_file\"  # read stream.bin");
    println!("  --features \"demo_tty\"   # attach to serial port");
    println!("  --features \"demo_c\"     # run C hybrid demo");
    println!("Example:");
    println!("  cargo run --features \"crc16 byte_stuff demo_sim\"");
}
