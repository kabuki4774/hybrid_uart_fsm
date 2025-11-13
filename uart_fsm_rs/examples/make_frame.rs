//! Example program to create UART frames for testing
//! Can output raw bytes or human-readable hex + binary file
//! Usage:
//!  cargo run --example make_frame -- --raw START | socat -u - /tmp/ttyV1
//!  cargo run --example make_frame -- START
//!  cargo run --example make_frame -- PING "hello world"
//! Outputs binary file: <cmd>.bin
//! Options:
//! --raw    Output only raw bytes to stdout (for piping to PTY)
//! <cmd>    One of: START, STOP, PING, RESET
//! [payload]  Optional payload for PING command
//! Example:
//! cargo run --example make_frame -- PING "hello world"
//!  Outputs: Hex bytes: AA 0E 03 68 65 6C 6C 6F 20 77 6F 72 6C 64 D2
//! Wrote ping.bin (14 bytes)
//! The file ping.bin will contain the corresponding binary frame.
//! To send the frame to a UART device, you can use socat or a similar tool.
//! For example:
//! socat -u ping.bin /tmp/ttyV1
//! This will write the contents of ping.bin to the specified UART device.
//! Make sure to replace /tmp/ttyV1 with the actual path to your UART device.
//! You can also use the --raw option to output only the raw bytes directly to stdout,
//! which can be piped directly to a UART device using socat or similar tools.

use std::env;
use std::io::{self, Write};
use uart_fsm_rs::harness::make_frame;

fn main() {
    // Usage:
    //   cargo run --quiet --example make_frame -- --raw START | socat -u - /tmp/ttyV1
    //   cargo run --quiet --example make_frame -- START
    //   cargo run --quiet --example make_frame -- PING "hello world"

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run --example make_frame -- [--raw] <CMD> [payload]");
        eprintln!("  CMD = START | STOP | PING | RESET");
        return;
    }

    // accept --raw anywhere
    let raw = args.iter().any(|a| a.eq_ignore_ascii_case("--raw"));

    // first non-flag is the command
    let mut non_flags = args.iter().skip(1).filter(|s| !s.starts_with("--"));
    let cmd = match non_flags.next() {
        Some(c) => c.to_uppercase(),
        None => {
            eprintln!("Missing CMD");
            return;
        }
    };

    // For PING: join remaining args with spaces (convert &String -> &str for join)
    let payload: Vec<u8> = if cmd == "PING" {
        let rest: Vec<&str> = non_flags.map(|s| s.as_str()).collect();
        if rest.is_empty() {
            Vec::new()
        } else {
            rest.join(" ").into_bytes()
        }
    } else {
        Vec::new()
    };

    // Map command to type byte
    let typ: u8 = match cmd.as_str() {
        "START" => 0x01,
        "STOP" => 0x02,
        "PING" => 0x03,
        "RESET" => 0xFF,
        _ => {
            eprintln!("Unknown command: {cmd}");
            return;
        }
    };

    // Build frame
    let frame = make_frame(typ, &payload);

    if raw {
        // emit only raw bytes to stdout (for piping to PTY)
        let mut out = io::stdout().lock();
        out.write_all(&frame).expect("stdout write failed");
        let _ = out.flush();
        return;
    }

    // human-readable + file output
    print!("Hex bytes: ");
    for b in &frame {
        print!("{:02X} ", b);
    }
    println!();

    let fname = format!("{}.bin", cmd.to_lowercase());
    std::fs::File::create(&fname)
        .and_then(|mut f| f.write_all(&frame))
        .expect("failed to write binary");
    println!("Wrote {} ({} bytes)", fname, frame.len());
}
