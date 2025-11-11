//! Test harness for UART FSM and parser
//! Provides functions to run demo and capture logs.
//! Supports both std and no_std environments.
//! Uses a ring buffer for input byte management.
//! Utilizes the FSM and parser modules for processing.

use crate::fsm::Device;
use crate::log::CaptureLogger;
use crate::parser::Parser;
use crate::ringbuf::RingBuf;

#[cfg(feature = "std")]
use crate::log::StdoutLogger;

pub fn make_frame(typ: u8, payload: &[u8]) -> heapless::Vec<u8, 64> {
    let len: u8 = (2 + payload.len()) as u8;
    let mut v: heapless::Vec<u8, 64> = heapless::Vec::new();
    v.push(0xAA).ok();
    v.push(len).ok();
    v.push(typ).ok();
    let mut sum: u16 = (len as u16) + (typ as u16);
    for &b in payload {
        #[cfg(feature = "byte_stuff")]
        {
            if b == 0xAA || b == 0x7D {
                v.push(0x7D).ok();
                v.push(b ^ 0x20).ok();
                continue;
            }
        }
        v.push(b).ok();
        sum = sum.wrapping_add(b as u16);
    }
    v.push(!(sum as u8)).ok();
    v
}

#[cfg(feature = "std")]
pub fn run_demo(input: &[u8], chunk_sizes: &[usize], max_ms: u32) {
    let mut rb: RingBuf<128> = RingBuf::new();
    let mut parser = Parser::new();
    let mut dev = Device::new(StdoutLogger);
    let mut now = 0u32;
    let mut cur = 0usize;
    let mut sizes = chunk_sizes.iter().copied().cycle();

    #[cfg(not(feature = "tickless"))]
    while now <= max_ms {
        if cur < input.len() {
            let n = sizes.next().unwrap_or(8).min(input.len() - cur);
            for i in 0..n {
                let _ = rb.push(input[cur + i]);
            }
            cur += n;
        }
        while let Some(b) = rb.pop() {
            if let Some(pkt) = parser.step(b, || {}) {
                dev.handle_packet(pkt);
            }
            dev.on_invalid_consecutive(parser.invalid_consecutive());
        }
        dev.tick(now);
        now += 1;
    }

    #[cfg(feature = "tickless")]
    while now <= max_ms {
        if cur < input.len() {
            let n = sizes.next().unwrap_or(8).min(input.len() - cur);
            for i in 0..n {
                let _ = rb.push(input[cur + i]);
            }
            cur += n;
        }
        while let Some(b) = rb.pop() {
            if let Some(pkt) = parser.step(b, || {}) {
                dev.handle_packet(pkt);
            }
            dev.on_invalid_consecutive(parser.invalid_consecutive());
        }
        dev.tick(now);
        if let Some(next) = dev.next_deadline_ms() {
            now = core::cmp::min(next, max_ms);
        } else {
            now += 1;
        }
    }
}

pub fn run_capture(input: &[u8], chunk_sizes: &[usize], max_ms: u32) -> heapless::String<1024> {
    let mut rb: RingBuf<32> = RingBuf::new();
    let mut parser = Parser::new();
    let mut log = CaptureLogger::new();
    let mut dev = Device::new(log);
    let mut now = 0u32;
    let mut cur = 0usize;
    let mut sizes = chunk_sizes.iter().copied().cycle();

    while now <= max_ms {
        if cur < input.len() {
            let n = sizes.next().unwrap_or(8).min(input.len() - cur);
            for i in 0..n {
                let _ = rb.push(input[cur + i]);
            }
            cur += n;
        }
        while let Some(b) = rb.pop() {
            if let Some(pkt) = parser.step(b, || {}) {
                dev.handle_packet(pkt);
            }
            dev.on_invalid_consecutive(parser.invalid_consecutive());
        }
        dev.tick(now);
        now += 1;
    }

    log = dev.log;
    log.take()
}
