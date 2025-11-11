//! Simulation tests for UART FSM and parser
//! Uses the test harness to run scenarios and capture logs.
//! Intended for std builds only.
//! Includes tests for valid frames, errors, boundaries, overflow, and timeouts.
//! Utilizes the harness module for frame creation and log capturing.
//! Employs assertions to verify expected log outputs.
//! Tests various aspects of the UART FSM and parser functionality.

use uart_fsm_demo::harness::{make_frame, run_capture};

#[test]
fn valid_start_ping_stop() {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice(&make_frame(0x01, &[]));
    bytes.extend_from_slice(&make_frame(0x03, b"hi"));
    bytes.extend_from_slice(&make_frame(0x02, &[]));

    let log = run_capture(&bytes, &[1, 2, 1, 3, 5], 2500);
    assert!(log.contains("STATE: Idle -> Active"));
    assert!(log.contains("PONG hi"));
    assert!(log.contains("HEARTBEAT 1000"));
    assert!(log.contains("STATE: Active -> Idle (STOP)"));
}

#[test]
fn noise_error_reset() {
    let bytes: [u8; 9] = [0x13, 0x00, 0xAA, 0x01, 0xFF, 0xAA, 0x02, 0xFF, 0xFE];
    let log = run_capture(&bytes, &[2, 2, 5], 150);
    assert!(log.contains("ERRORS: 3 invalid frames"));
    assert!(log.contains("-> Error"));
    assert!(log.contains("STATE: Error -> Idle (RESET)"));
}

#[test]
fn len_boundaries() {
    let mut bytes: Vec<u8> = Vec::new();
    let payload24 = [b'A'; 24];
    bytes.extend_from_slice(&make_frame(0x01, &[]));
    bytes.extend_from_slice(&make_frame(0x03, &payload24));
    bytes.extend_from_slice(&make_frame(0x02, &[]));
    let log = run_capture(&bytes, &[7], 3000);
    assert!(log.contains("PONG AAAAAAAAAAAAAAAAAAAAAAAA"));
}

#[test]
fn overflow_still_runs() {
    let mut bytes: Vec<u8> = Vec::new();
    for _ in 0..10 {
        bytes.extend_from_slice(&make_frame(0x03, b"x"));
    }
    let mut input: Vec<u8> = make_frame(0x01, &[]).to_vec();
    input.extend_from_slice(&bytes);
    let log = run_capture(&input, &[100], 1200);
    assert!(log.contains("STATE: Idle -> Active"));
}

#[test]
fn inactivity_timeout() {
    let bytes = make_frame(0x01, &[]);
    let log = run_capture(&bytes, &[8], 5200);
    assert!(log.contains("STATE: Active -> Idle (inactivity)"));
}
