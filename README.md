# Hybrid UART FSM (C + Rust)

**What is this?**  
A small, testable UART protocol stack with a finite state machine (FSM) and a streaming parser implemented in both **C** and **Rust**. The repo demonstrates parity, fuzzing, and a hybrid mode where C programs call into a Rust worker thread and/or use Rust to build frames, showing how to evolve legacy C firmware toward Rust safely and incrementally.


## Highlights
- **Protocol**: `SYNC=0xAA`, `LEN`, `TYPE`, `PAYLOAD…`, `CHECKSUM/CRC`
- **Features (compile-time)**
  - `crc16`: CRC16‑CCITT over `[LEN, TYPE, PAYLOAD]`
  - `crc16_2b`: emit/expect **two CRC bytes** (MSB, LSB). Implies `crc16`
  - `byte_stuff`: escape `0xAA`/`0x7D` after SYNC (Rust un-stuffs accordingly)
  - `tickless`: low‑power scheduling for the FSM
- **Parity**: C and Rust parsers fuzzed and tested for behavioral equivalence.
- **Demos**: Valid start→ping→stop, noise→error→reset, stdin/tty piping.

## Layout
c_firmware/   # simple C app that spawns Rust worker and uses Rust to build frames
uart_fsm_c/   # C reference (parser, FSM, harness, demo)
uart_fsm_rs/  # Rust port (parser, FSM, harness, examples)
rust_logic/   # Rust FFI library used by C (worker thread, rust_build_frame, etc.)
fuzz/         # libFuzzer targets (diff & parser)
docs/         # design, API, protocol
tools/        # scripts (e.g., system diagnostics)
logs/, reports/  # runtime artifacts (ignored by git)

## Quick Start
```bash
# 1) Build the C demo (C parses; Rust builds frames):
make -C c_firmware RUST_FEATURES="std,crc16,crc16_2b,byte_stuff" run

# 2) Run C reference demo directly:
make -C uart_fsm_c clean all crc16=1 byte_stuff=1 && ./uart_fsm_c/uart_fsm_demo --test

# 3) Generate a Rust frame and pipe into C demo:
cargo run --quiet --no-default-features \
  --features "std,crc16,crc16_2b,byte_stuff" \
  --manifest-path uart_fsm_rs/Cargo.toml \
  --example make_frame -- --raw RESET | ./uart_fsm_c/uart_fsm_demo --stdin

  ### Feature Matrix (build-time)

| Mode                | Checksum bytes | Stuffing  | Notes                            |
|----------------------|----------------|------------|----------------------------------|
| legacy               | 1 (8-bit)      | optional   | `--features "std"`               |
| CRC16 (LSB only)     | 1 (LSB)        | optional   | `--features "std,crc16"`         |
| CRC16 (2B MSB, LSB)  | 2              | optional   | `--features "std,crc16,crc16_2b"` |

See docs/PROTOCOL.md for more details.

