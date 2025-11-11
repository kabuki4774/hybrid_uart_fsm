# Rust API Reference

## Library Crates

| Crate | Purpose |
|-------|----------|
| `rust_logic` | Staticlib callable from C firmware |
| `uart_fsm_rs` | Binary harness calling into C parser |
| `fuzz_targets` | Fuzzing entry points |

---

## `rust_logic` Public Functions

```rust
#[no_mangle] pub extern "C" fn rust_hello();
#[no_mangle] pub extern "C" fn rust_process_frame(buf: *const u8, len: usize) -> i32;
#[no_mangle] pub extern "C" fn rust_crc16(buf: *const u8, len: usize) -> u16;

#[no_mangle] pub extern "C" fn rust_worker_start() -> i32;
#[no_mangle] pub extern "C" fn rust_worker_submit(buf: *const u8, len: usize) -> i32;
#[no_mangle] pub extern "C" fn rust_worker_stop() -> i32;

---

### Examples

## Worker Thread
```bash
rust_worker_start();
rust_worker_submit(frame, len);
rust_worker_stop();

Logs appear asynchronously from the Rust thread.

## Differential Fuzzing [coming soon]

fuzz/fuzz_targets/fuzz_diff.rs executes both C and Rust parsers on random data and asserts parity.