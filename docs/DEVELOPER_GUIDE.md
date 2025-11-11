# Developer Guide

## Directory Structure
uart_fsm_c/           # C implementation
uart_fsm_rs/          # Rust wrapper / driver
rust_logic/           # Rust staticlib for C firmware [coming soon]
docs/                 # Documentation

---

## Modules

| Module | Language | Responsibility |
|---------|-----------|----------------|
| `ringbuf` | C | Fixed-size circular byte buffer |
| `parser` | C/Rust | Stream → frames with checksum verification |
| `fsm` | C/Rust | Device state machine, heartbeat, timers |
| `harness` | C | Test scaffolding and serial simulation |
| `rust_logic` | Rust | Safe parser + worker thread implementation |
| `fuzz_targets` | Rust/C | Automated input fuzzers |
| `examples/` | Rust | Frame generator, regression runner |

---

## Build Instructions

### C

```bash
make            # build demo binary
make lib        # build static library libuartfsm.a
make run        # run demos

### Rust
```bash
cargo build
cargo run
cargo fuzz run fuzz_parser   # optional [coming soon]

### Hybrid [coming soon]
Rust driving C — use uart_fsm_rs/build.rs.
C driving Rust — link librustlogic.a in your Makefile.
