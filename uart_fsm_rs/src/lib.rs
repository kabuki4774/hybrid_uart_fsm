//! UART FSM + Parser Library
//! -------------------------
//! Modules:
//! - `ringbuf` : SPSC ring buffer
//! - `parser`  : byte-stream frame parser (spec-correct)
//! - `fsm`     : device state machine
//! - `harness` : helpers for building frames and running demos/tests
//! - `crc`     : CRC16-CCITT helper (shared)
//! - `log`     : logger trait and sinks
//!
//! C hybrid demo (feature `demo_c`):
//! - build.rs compiles a static C library and we link to it here.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod crc;
pub mod fsm;
pub mod harness;
pub mod log;
pub mod parser;
pub mod ringbuf;

#[cfg(feature = "demo_c")]
#[link(name = "uartfsm", kind = "static")]
extern "C" {
    fn run_demos();
    fn run_from_stdin();
}

#[cfg(feature = "demo_c")]
pub fn run_c_demos() {
    unsafe { run_demos() }
}

#[cfg(feature = "demo_c")]
pub fn run_c_stdin() {
    unsafe { run_from_stdin() }
}
