//! UART FSM and Parser Library

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod fsm;
pub mod harness;
pub mod log;
pub mod parser;
pub mod ringbuf;
