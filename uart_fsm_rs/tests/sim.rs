//! Simulation tests for UART FSM + parser.
//!
//! These tests exercise:
//!  - Valid flow with START → PING("hi") → STOP
//!  - Noise and desync leading to Error, then recovery via RESET
//!  - Boundary LEN handling (PING with 24 bytes)
//!  - Overflow tolerance (ring too small for a burst; system still runs)
//!  - Inactivity timeout back to Idle
//!
//! The tests intentionally use `make_frame` so they are agnostic to feature flags
//! like `crc16` and `byte_stuff` (builder and parser always match).

use uart_fsm_rs::harness::{make_frame, run_capture};

#[test]
fn valid_start_ping_stop() {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&make_frame(0x01, &[])); // START
    bytes.extend_from_slice(&make_frame(0x03, b"hi")); // PING "hi"
    bytes.extend_from_slice(&make_frame(0x02, &[])); // STOP

    let log = run_capture(&bytes, &[1, 2, 1, 3, 5], 2500);
    assert!(log.contains("STATE: Idle -> Active"));
    assert!(log.contains("PONG hi"));
    assert!(log.contains("HEARTBEAT 1000"));
    assert!(log.contains("STATE: Active -> Idle (STOP)"));
}

#[test]
fn noise_error_reset() {
    // Noise (13 00), then a short/bad frame (AA 01 FF), then a valid RESET frame.
    // This produces at least three consecutive invalid events → enter Error,
    // then the valid RESET drives Error -> Idle.
    let mut bytes: Vec<u8> = vec![0x13, 0x00, 0xAA, 0x01, 0xFF];
    bytes.extend_from_slice(&make_frame(0xFF, &[])); // RESET (valid under any feature flags)

    let log = run_capture(&bytes, &[2, 2, 5], 150);
    assert!(log.contains("ERRORS: 3 invalid frames"));
    assert!(log.contains("-> Error"));
    assert!(log.contains("STATE: Error -> Idle (RESET)"));
}

#[test]
fn len_boundaries() {
    let mut bytes: Vec<u8> = Vec::new();
    let payload24 = [b'A'; 24]; // Max PING payload per spec
    bytes.extend_from_slice(&make_frame(0x01, &[])); // START
    bytes.extend_from_slice(&make_frame(0x03, &payload24)); // PING (24A)
    bytes.extend_from_slice(&make_frame(0x02, &[])); // STOP
    let log = run_capture(&bytes, &[7], 3000);
    assert!(log.contains("PONG AAAAAAAAAAAAAAAAAAAAAAAA"));
}

#[test]
fn overflow_still_runs() {
    // Generate many PING frames to flood a small ring, then START upfront so we go Active.
    let mut bytes: Vec<u8> = Vec::new();
    for _ in 0..10 {
        bytes.extend_from_slice(&make_frame(0x03, b"x"));
    }
    let mut input: Vec<u8> = make_frame(0x01, &[]).to_vec(); // START
    input.extend_from_slice(&bytes);
    let log = run_capture(&input, &[100], 1200);
    assert!(log.contains("STATE: Idle -> Active"));
}

#[test]
fn inactivity_timeout() {
    // START and then wait; should fall back to Idle after 5s of inactivity.
    let bytes = make_frame(0x01, &[]); // START
    let log = run_capture(&bytes, &[8], 5200);
    assert!(log.contains("STATE: Active -> Idle (inactivity)"));
}
