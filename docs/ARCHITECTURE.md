# System Architecture Overview

This document summarizes the module relationships, data flow, and build integration
for the UART Stream Parser + FSM system across both C and Rust components.

---

## 1. ASCII Block Diagram

             ┌──────────────────────────────┐
             │          UART Line           │
             │  (Incoming asynchronous bytes)│
             └──────────────┬───────────────┘
                            │
                    [ISR / Driver]
                            │
                            ▼
                  ┌─────────────────┐
                  │   Ring Buffer   │
                  │ (128B, SPSC)    │
                  └─────────────────┘
                            │
                            ▼
                ┌────────────────────┐
                │     Parser FSM     │
                │  WaitSync→ReadLen  │
                │  →ReadType→Payload │
                │  Valid/Invalid Out │
                └────────────────────┘
                            │
                            ▼
            ┌────────────────────────────────┐
            │         Device FSM             │
            │ Idle ↔ Active ↔ Error states   │
            │ Heartbeat / Timeout / Commands │
            └────────────────────────────────┘
                            │
              ┌─────────────┴─────────────┐
              ▼                           ▼
      ┌─────────────────┐          ┌─────────────────┐
      │    Logger / UI  │          │  Application    │
      │  (printf/logs)  │          │   Logic Layer   │
      └─────────────────┘          └─────────────────┘


---

## 2. Module Relationships

| Layer | Module | Language | Responsibility |
|-------|---------|-----------|----------------|
| I/O | `ringbuf.[ch]` | C | Single-producer/single-consumer buffer for UART RX |
| Protocol | `parser.[ch]` | C / Rust | Converts byte stream into frames; verifies integrity |
| Logic | `fsm.[ch]` | C / Rust | Executes command semantics; manages timeouts & heartbeats |
| Test | `harness.c` | C | Simulated UART driver and demo |
| Integration | `rust_logic/lib.rs` | Rust | Safe high-level interface; worker threads; FFI exports |
| Hybrid bridge | `uart_fsm_rs` | Rust | Calls into C library; fuzzing & regression |
| Validation | `fuzz_targets/` | Rust / C | Automated fuzzers and differential equivalence testing |

---

## 3. Hybrid Build Integration

[ C side ]
┌────────────────────────────┐
│ uart_fsm_c (libuartfsm.a)  │
└────────────┬───────────────┘
             │  static link
    [ Rust side ]
┌────────────────────────────┐
│ rust_logic (librustlogic.a)│
│ uart_fsm_rs (cargo run)    │
└────────────────────────────┘

- *Rust→C mode:* Rust binary (`uart_fsm_rs`) calls `libuartfsm.a`.
- *C→Rust mode:* C firmware links `librustlogic.a`.

Both can coexist and call each other safely.

---

## 4. Timing Model

           +---------------------+
           | UART RX Interrupt   |
           +----------+----------+
                      |
                      ▼
           [ rb_push(byte) ]  →  Ring Buffer full? drop oldest
                      |
                      ▼
     Main loop / Rust worker thread:
           rb_pop() → parser_feed_byte()
                 ├── valid → FSM.handle_packet()
                 └── invalid → invalid_consec++
                      |
                      ▼
           FSM.tick(now_ms)
               ├── heartbeat every 1 s
               └── inactivity → Idle

---

## 5. Verification Pipeline [coming soon]

             +--------------------------+
             | cargo test / make test   |
             +------------+-------------+
                          |
             +------------v-------------+
             |    Fuzzers (libFuzzer)   |
             |  - Rust fuzz_parser      |
             |  - C fuzz_parser.c       |
             |  - fuzz_diff (both)      |
             +------------+-------------+
                          |
             +------------v-------------+
             | Regression Runner        |
             |  (replays corpus cases)  |
             +--------------------------+

---

## 6. Data Flow Summary

| Stage | Input | Output | Notes |
|--------|--------|--------|-------|
| ISR → Ring Buffer | UART bytes | Stored bytes | Non-blocking push |
| Parser FSM | Bytes | Valid/invalid packets | Checks LEN, TYPE, CHK |
| Device FSM | Packets | State transitions, heartbeats | 3-state logic |
| Logger | Events | Text logs | For test harness only |
| Fuzzers | Random data | Code coverage, crash corpus | Verifies stability | [coming soon]

---

## 7. Interfaces Overview [coming soon]

### From C
```c
void parser_init(parser_t *p);
int parser_feed_byte(parser_t *p, uint8_t byte, packet_t *out);
void fsm_handle_packet(fsm_t *f, const packet_t *pkt);
void fsm_tick(fsm_t *f, uint32_t now_ms);

### From Rust
extern "C" {
    fn rust_process_frame(buf: *const u8, len: usize) -> i32;
    fn rust_worker_start() -> i32;
    fn rust_worker_submit(buf: *const u8, len: usize) -> i32;
    fn rust_worker_stop() -> i32;
}


