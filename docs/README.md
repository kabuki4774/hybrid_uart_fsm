# UART Stream Parser + FSM System

This system provides a reliable way to interpret framed UART data into
device commands and manage a deterministic finite-state machine (FSM)
for embedded or host-simulated environments.

---

## Contents

| File | Description |
|------|--------------|
| `PROTOCOL.md` | Packet framing, checksum, CRC, and escape rules |
| `DEVELOPER_GUIDE.md` | How the modules fit together and how to build them |
| `TESTING.md` | Testing, fuzzing, and regression pipelines |
| `C_API.md` | Public C function reference |
| `RUST_API.md` | Public Rust FFI and module reference |

---

## Features

* Deterministic **parser FSM**
* **Ring buffer** for UART ISR decoupling
* **Device FSM** (`Idle ↔ Active ↔ Error`)
* **Checksum or CRC16-CCITT** integrity
* **Byte-stuffing** escape handling
* **Tickless** heartbeat scheduling
* **Hybrid C↔Rust FFI integration**
* **Differential fuzzing** across implementations
* **Regression runner** for long-term equivalence

---

## Build Surfaces

•	C‑only: make -C uart_fsm_c ...
•	Hybrid: make -C c_firmware ... (builds rust_logic with features, then links C).
•	Rust‑only demos: cargo run -p uart_fsm_rs ...