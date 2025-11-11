# Testing and Fuzzing Guide

## 1. Unit Tests

| Component | Command |
|------------|----------|
| Rust parser | `cargo test` |
| C parser | `make test` |
| Hybrid integration | `cargo run` or `./uart_fsm_demo` | [coming soon]

---

## 2. Fuzz Testing [coming soon]

### Rust

```bash
cargo install cargo-fuzz
cargo fuzz run fuzz_parser
cargo fuzz run fuzz_diff    # Differential fuzzer

### C
```bash
clang -fsanitize=fuzzer,address,undefined -I./src \
      fuzz_parser.c src/parser.c src/ringbuf.c -o fuzz_parser
./fuzz_parser