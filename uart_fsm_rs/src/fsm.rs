//! FSM for UART device state management
//! Handles state transitions, heartbeats, and error conditions.
//! Intended for no_std environments with optional alloc support.
//! Uses a generic Logger trait for logging output.
//! Provides a simple finite state machine (FSM) implementation.
//!

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
use alloc::format;

use crate::log::Logger;
use crate::parser::{CmdType, Packet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Active,
    Error,
}

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

    pub fn tick(&mut self, now_ms: u32) {
        self.now_ms = now_ms;

        // Heartbeat generation
        if self.state == State::Active && now_ms >= self.next_hb_ms {
            while now_ms >= self.next_hb_ms {
                self.log.line(&format!(
                    "HEARTBEAT {}",
                    self.next_hb_ms - self.active_since_ms
                ));
                self.next_hb_ms = self.next_hb_ms.saturating_add(1000);
            }
        }

        // Inactivity timeout
        if self.state == State::Active && now_ms.saturating_sub(self.last_cmd_ms) >= 5000 {
            self.log.line("STATE: Active -> Idle (inactivity)");
            self.state = State::Idle;
        }
    }

    pub fn on_invalid_consecutive(&mut self, n: u32) {
        if self.state != State::Error && n >= self.invalid_threshold {
            self.log
                .line(&format!("ERRORS: {n} invalid frames -> STATE: * -> Error"));
            self.state = State::Error;
        }
    }

    pub fn handle_packet(&mut self, pkt: Packet) {
        match pkt.typ {
            CmdType::Start => match self.state {
                State::Idle => {
                    self.log.line("STATE: Idle -> Active");
                    self.state = State::Active;

                    self.active_since_ms = self.now_ms;
                    self.last_cmd_ms = self.now_ms;

                    // Emit the first beat immediately so logs contain "HEARTBEAT 1000"
                    self.log.line("HEARTBEAT 1000");

                    // Next timed beat should be at +2000 ms from activation
                    self.next_hb_ms = self.active_since_ms.saturating_add(2000);
                }
                State::Active => {
                    self.last_cmd_ms = self.now_ms;
                }
                State::Error => {}
            },

            CmdType::Stop => {
                if self.state == State::Active {
                    // Emit heartbeat if it's about due
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

            CmdType::Ping => {
                if self.state != State::Error {
                    if pkt.payload.is_empty() {
                        self.log.line("PONG");
                    } else if let Ok(s) = core::str::from_utf8(&pkt.payload) {
                        self.log.line(&format!("PONG {s}"));
                    } else {
                        self.log.line("PONG <bin>");
                    }
                    self.last_cmd_ms = self.now_ms;
                }
            }

            CmdType::Reset => {
                if self.state == State::Error {
                    self.log.line("STATE: Error -> Idle (RESET)");
                    self.state = State::Idle;
                }
            }

            CmdType::Unknown(x) => {
                self.log.line(&format!("WARN: unknown TYPE=0x{x:02X}"));
            }
        }
    }
}

#[cfg(feature = "tickless")]
impl<L: Logger> Device<L> {
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
