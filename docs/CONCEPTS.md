# Key Concepts

## Ring Buffer
A fixed-size circular array storing bytes between interrupt context (producer)
and main loop (consumer). Constant-time O(1) push/pop, no heap, no fragmentation.

## Parser FSM
Incremental state machine that rebuilds packets from arbitrary byte streams.
It maintains state between calls and validates sync, length, and checksum.

## Finite-State Machine
Tracks device modes (Idle, Active, Error) and transitions deterministically
based on parsed packets or timeouts.

## Checksum vs CRC16
Checksum: fast, low-cost detection of single-bit errors.
CRC16-CCITT: polynomial remainder method detecting most burst errors.

## Byte-Stuffing
Escapes reserved bytes (0xAA, 0x7D) to allow arbitrary payload data.

## Tickless Scheduling
Calculates next deadline to sleep until, minimizing CPU wake-ups and power draw.

## Fuzzing
Automated randomized testing exploring parser state space to find crashes,
hangs, or behavioral differences.

## Differential Fuzzing
Compares two implementations’ outputs under identical random input streams to
ensure functional equivalence.

## Regression Runner
Replays all previously failing inputs to guarantee fixes remain effective.

## Hybrid C↔Rust Integration
Shared boundary via FFI (`extern "C"`) using static libraries.  Enables gradual
migration or safe augmentation without rewriting entire codebases.