//! Logger trait and implementations for UART FSM
//! Provides a simple logging interface for capturing log messages.
//! Intended for no_std environments with optional alloc support.

#![cfg_attr(not(feature = "std"), no_std)]

/// Generic logger trait for portable builds.

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::format;

#[cfg(feature = "std")]
use std::format;

pub trait Logger {
    fn line(&mut self, s: &str);
}

/// Standard output logger (only in std builds)
#[cfg(feature = "std")]
pub struct StdoutLogger;

#[cfg(feature = "std")]
impl Logger for StdoutLogger {
    fn line(&mut self, s: &str) {
        println!("{}", s);
    }
}

/// Capture logger used for testing (works in no_std too)
pub struct CaptureLogger {
    buf: heapless::String<1024>,
}

impl CaptureLogger {
    pub fn new() -> Self {
        Self {
            buf: heapless::String::new(),
        }
    }

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
