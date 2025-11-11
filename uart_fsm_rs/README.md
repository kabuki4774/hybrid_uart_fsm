# UART Stream Parser + FSM (Rust, host-simulatable)

**Goal**: Parse `[AA][LEN][TYPE][PAY...][CHK]` from a byte stream, drive an `Idle/Active/Error` FSM, and emit heartbeats.

## Structure

- `ringbuf.rs` – SPSC ring buffer (128B demo / 32B in tests). O(1) push/pop, drop-on-overflow (returns `Err(())`).
- `parser.rs` – Stateful parser:
  - States: `WaitSync → ReadLen → ReadType → ReadPayload`.
  - `LEN` range 2–32 (inclusive). Payload for PING limited to 24 bytes (heapless `Vec`).
  - Checksum: `~((LEN + TYPE + sum(payload)) & 0xFF)`; excludes `SYNC` and `CHK`.
  - **Resync**: In `WaitSync`, each non-`0xAA` increments `invalid_consec`. Bad `LEN` or bad checksum resets to `WaitSync` and increments `invalid_consec`. This matches the spec’s Example 2 (“3 invalid frames”).
- `fsm.rs` – Device FSM (with generic `Logger`):
  - `Idle→Active` on `START`.
  - `Active→Idle` on `STOP` or 5 s inactivity.
  - `*→Error` when `invalid_consec ≥ 3`.
  - `Error→Idle` on `RESET`.
  - In `Active`, heartbeat every 1000 ms: `HEARTBEAT <ms_since_active_start>`.
  - `PING` yields `PONG <payload>` and resets inactivity timer.
- `log.rs` – Simple `Logger` trait; `StdoutLogger` + `CaptureLogger` for deterministic tests.
- `harness.rs` – Cooperative loop: feed bytes in arbitrary chunk sizes, pump RB→parser→FSM, advance ms ticks. Helper to build valid frames.

## Correctness highlights

- **Invalid frame detection**: wrong sync, LEN out of [2,32], payload length over 24, checksum mismatch → parsed as invalid and resynchronized.
- **Resynchronization**: We do not scan the ring with memmoves; instead the bytewise pump keeps constant time and relies on the SYNC gate to quickly realign. Counting non-SYNC bytes as invalid aligns with the provided “3 invalid frames → Error” example.
- **Timers**: Millisecond ticks; heartbeat at 1 s cadence from `Active` entry; 5 s inactivity → `Active→Idle`.

## Testing

- **Happy path**: `START → PING("hi") → STOP`, with heartbeats asserted.
- **Noise/desync**: Example 2 (noise then RESET) including *three invalid* to enter `Error` and recover.
- **Boundary LEN**: `LEN=2` (no payload) and `LEN=32` (`PING` with 24B payload).
- **Overflow**: Tiny ring with a huge ISR-like chunk; parsing still progresses (enter `Active`, heartbeat).
- **Inactivity**: Timeout back to `Idle`.

## Performance & Robustness

- **Hot path**: O(1) per byte, predictable branchy FSM; no dynamic allocation (`heapless::Vec`).
- **Memory**: 128B ring + ≤24B payload scratch + small struct fields; no heap.
- **Overflow**: ISR-like policy (drop on push `Err(())`); upper layers tolerate loss.

## Design choices & alternatives

- **Resync policy**:
  - *Chosen*: Count non-`SYNC` while waiting and reset on any invalid LEN/CHK. Pros: matches example behavior; simple; constant-time. Cons: may count noise bytes aggressively; acceptable given threshold of 3.
  - *Alt*: Only count “frames that start with `AA` but later fail” as invalid, ignoring pre-sync noise. Easy tweak (don’t increment in `WaitSync`).
- **Checksum vs CRC**:
  - *Chosen*: 8-bit sum + NOT per spec; fast on MCUs.
  - *Alt (stretch)*: CRC-16-CCITT improves burst error detect at small cost; can slot into parser with a feature flag.
- **Byte-stuffing**:
  - *Not required*: If payload may contain `0xAA`, add escape `0x7D` + XOR transform; parser state gains a simple “escape” substate.
- **Ticking**:
  - *Chosen*: periodic `tick(now_ms)`. 
  - *Alt (stretch)*: tickless (“next deadline” scheduling) reduces wakeups for low-power MCUs.

## Portability / `no_std`

- The core (ringbuf/parser/fsm) avoids `std`. Logging is abstracted; `StdoutLogger` is used on host, and an embedded logger can write to RTT/UART.