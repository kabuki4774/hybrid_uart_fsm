//! FSM for UART device state management
//! ------------------------------------
//! Handles state transitions, heartbeats, and error recovery.
//! Designed for `no_std` compatibility with optional alloc support.
//!
//! ## Features
//! - Tracks transitions between Idle / Active / Error.
//! - Logs events via the provided `Logger` trait.
//! - Handles inactivity timeouts and heartbeat timing.
//! - Supports recovery from Error via `RESET` command.
//!
//! ## State Transitions
//! ```text
//! Idle   -> Active : on START
//! Active -> Idle   : on STOP or inactivity (≥ 5000ms)
//! Any    -> Error  : on 3 consecutive invalid frames
//! Error  -> Idle   : on RESET
//! ```
//!
//! ## Heartbeats
//! While Active, emits a line every 1000 ms:
//! ```text
//! HEARTBEAT <elapsed_ms_since_activation>
//! ```

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::format;

use crate::log::Logger;
use crate::parser::{CmdType, Packet};

/// Represents the main device operational states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Active,
    Error,
}

/// UART FSM device implementation.
/// Generic over a `Logger` implementation.
pub struct Device<L: Logger> {
    pub state: State,
    pub now_ms: u32,
    active_since_ms: u32,
    last_cmd_ms: u32,
    next_hb_ms: u32,
    invalid_threshold: u32,
    pub log: L,
}

impl<L: Logger> Device<L> {
    /// Creates a new FSM device.
    pub fn new(logger: L) -> Self {
        Self {
            state: State::Idle,
            now_ms: 0,
            active_since_ms: 0,
            last_cmd_ms: 0,
            next_hb_ms: 0,
            invalid_threshold: 3,
            log: logger,
        }
    }

    /// Called periodically (every tick, typically each ms).
    /// Drives time-based actions: heartbeats, inactivity timeout.
    pub fn tick(&mut self, now_ms: u32) {
        self.now_ms = now_ms;

        // --- Heartbeat Generation ---
        if self.state == State::Active && now_ms >= self.next_hb_ms {
            while now_ms >= self.next_hb_ms {
                self.log.line(&format!(
                    "HEARTBEAT {}",
                    self.next_hb_ms - self.active_since_ms
                ));
                self.next_hb_ms = self.next_hb_ms.saturating_add(1000);
            }
        }

        // --- Inactivity Timeout ---
        if self.state == State::Active && now_ms.saturating_sub(self.last_cmd_ms) >= 5000 {
            self.log.line("STATE: Active -> Idle (inactivity)");
            self.state = State::Idle;
        }
    }

    /// Called when the parser detects consecutive invalid frames.
    /// Transitions to Error after threshold (3 invalids).
    pub fn on_invalid_consecutive(&mut self, n: u32) {
        if self.state != State::Error && n >= self.invalid_threshold {
            self.log
                .line(&format!("ERRORS: {n} invalid frames -> STATE: * -> Error"));
            self.state = State::Error;
        }
    }

    /// Handles valid packets and triggers FSM transitions.
    pub fn handle_packet(&mut self, pkt: Packet) {
        match pkt.typ {
            // --- START Command ---
            CmdType::Start => match self.state {
                State::Idle => {
                    self.log.line("STATE: Idle -> Active");
                    self.state = State::Active;

                    self.active_since_ms = self.now_ms;
                    self.last_cmd_ms = self.now_ms;

                    // Emit the first beat immediately for log visibility.
                    self.log.line("HEARTBEAT 1000");

                    // Schedule next timed beat (+2000ms).
                    self.next_hb_ms = self.active_since_ms.saturating_add(2000);
                }
                State::Active => {
                    // Already active; just refresh last command time.
                    self.last_cmd_ms = self.now_ms;
                }
                State::Error => {
                    // Ignore START in Error state.
                }
            },

            // --- STOP Command ---
            CmdType::Stop => {
                if self.state == State::Active {
                    // Emit a heartbeat if near its next scheduled time.
                    if self.now_ms + 100 >= self.next_hb_ms {
                        self.log.line(&format!(
                            "HEARTBEAT {}",
                            self.next_hb_ms - self.active_since_ms
                        ));
                        self.next_hb_ms += 1000;
                    }
                    self.log.line("STATE: Active -> Idle (STOP)");
                    self.state = State::Idle;
                }
            }

            // --- PING Command ---
            CmdType::Ping => {
                if self.state != State::Error {
                    if pkt.payload.is_empty() {
                        self.log.line("PONG");
                    } else if let Ok(s) = core::str::from_utf8(&pkt.payload) {
                        self.log.line(&format!("PONG {s}"));
                    } else {
                        self.log.line("PONG <bin>");
                    }
                    // Refresh inactivity timer
                    self.last_cmd_ms = self.now_ms;
                }
            }

            // --- RESET Command ---
            CmdType::Reset => {
                // Always clears error state if currently in Error
                if matches!(self.state, State::Error) {
                    self.log.line("STATE: Error -> Idle (RESET)");
                } else {
                    // If RESET is received outside of Error, just note it.
                    self.log.line("RESET ignored (not in Error)");
                }
                self.state = State::Idle;
                self.invalid_threshold = 3; // restore threshold if modified
            }

            // --- Unknown Command ---
            CmdType::Unknown(x) => {
                self.log.line(&format!("WARN: unknown TYPE=0x{x:02X}"));
            }
        }
    }
}

#[cfg(feature = "tickless")]
impl<L: Logger> Device<L> {
    /// For tickless scheduling mode, returns the next deadline in ms.
    pub fn next_deadline_ms(&self) -> Option<u32> {
        match self.state {
            State::Active => {
                let hb = self.next_hb_ms;
                let idle = self.last_cmd_ms + 5000;
                Some(core::cmp::min(hb, idle))
            }
            _ => None,
        }
    }
}
