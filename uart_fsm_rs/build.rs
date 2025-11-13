//! Build script for UART FSM (Rust + optional C bridge)
//!
//! When `--features demo_c` is set, this compiles the C reference implementation
//! as a static library (`libuartfsm.a`) to support hybrid mode testing.

fn main() {
    let demo_c = std::env::var("CARGO_FEATURE_DEMO_C").is_ok();

    if demo_c {
        println!("cargo:warning=uart_fsm_rs: Building uart_fsm_c static library (demo_c enabled)");
        println!("cargo:rerun-if-changed=../uart_fsm_c/src");

        // Mirror Rust feature flags into C preprocessor defines
        let crc16 = std::env::var("CARGO_FEATURE_CRC16").is_ok();
        let bytestuff = std::env::var("CARGO_FEATURE_BYTE_STUFF").is_ok();
        let tickless = std::env::var("CARGO_FEATURE_TICKLESS").is_ok();
        let demo_tty = std::env::var("CARGO_FEATURE_DEMO_TTY").is_ok(); // serial demo

        let mut b = cc::Build::new();
        b.include("../uart_fsm_c/src")
            .files([
                "../uart_fsm_c/src/harness.c",
                "../uart_fsm_c/src/fsm.c",
                "../uart_fsm_c/src/parser.c",
                "../uart_fsm_c/src/ringbuf.c",
                "../uart_fsm_c/src/main.c",
            ])
            .flag("-std=c11")
            .flag("-O2");

        if crc16 {
            b.define("USE_CRC16", Some("1"));
        }
        if bytestuff {
            b.define("USE_BYTESTUFF", Some("1"));
        }
        if tickless {
            b.define("USE_TICKLESS", Some("1"));
        }
        if demo_tty {
            b.define("USE_SERIAL", Some("1"));
        }

        b.compile("uartfsm");
    } else {
        println!(
            "cargo:warning=uart_fsm_rs: Skipping C static library build (demo_c feature not set)"
        );
    }
}
