//! Logger trait and implementations for UART FSM
//! ---------------------------------------------
//! - `Logger` trait: simple `.line(&str)` sink.
//! - `StdoutLogger` (std-only): prints to stdout.
//! - `CaptureLogger`: appends lines to a fixed heapless buffer (used by tests).
//!
//! `no_std`: Works without `alloc`. We format into a `heapless::String`.

#![cfg_attr(not(feature = "std"), no_std)]

pub trait Logger {
    /// Emit one log line (the caller provides the newline, or the impl may add it).
    fn line(&mut self, s: &str);
}

/// Standard output logger (only exists under `std`).
#[cfg(feature = "std")]
pub struct StdoutLogger;

#[cfg(feature = "std")]
impl Logger for StdoutLogger {
    fn line(&mut self, s: &str) {
        println!("{s}");
    }
}

/// Capture logger used by testing/harness to assert output.
/// Stores lines in a fixed-capacity heapless string buffer.
pub struct CaptureLogger {
    buf: heapless::String<1024>,
}

impl CaptureLogger {
    /// Create a fresh capture logger.
    pub fn new() -> Self {
        Self {
            buf: heapless::String::new(),
        }
    }

    /// Consume and return the captured buffer.
    pub fn take(self) -> heapless::String<1024> {
        self.buf
    }
}

impl Logger for CaptureLogger {
    fn line(&mut self, s: &str) {
        use core::fmt::Write;
        let _ = writeln!(self.buf, "{s}");
    }
}
