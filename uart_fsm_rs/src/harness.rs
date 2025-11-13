//! Test/demo harness for the UART FSM + parser
//! -------------------------------------------
//! - `make_frame(typ, payload)`: builds a single frame matching the current feature set
//!   (8-bit checksum OR CRC16; one-byte or two-byte CRC; with optional byte-stuffing).
//! - `run_demo(...)`: std-mode demo runner that prints to stdout.
//! - `run_capture(...)`: headless runner that captures logs for assertions.

#[cfg(feature = "crc16")]
use crate::crc::crc16_ccitt;

use crate::fsm::Device;
use crate::log::CaptureLogger;
use crate::parser::Parser;
use crate::ringbuf::RingBuf;

#[cfg(feature = "std")]
use crate::log::StdoutLogger;

/// Build one frame according to spec + feature flags.
/// - 8-bit sum mode (no `crc16`): CHECKSUM = ~((LEN + TYPE + sum(payload)) & 0xFF)
/// - CRC16 one-byte mode (`crc16` only): use **LSB only** (legacy compatibility)
/// - CRC16 two-byte mode (`crc16` + `crc16_2b`): emit **MSB then LSB** (C-side parity)
/// - byte_stuff: escape 0xAA and 0x7D in *all* bytes after SYNC (LEN/TYPE/PAYLOAD/CHECKSUM)
pub fn make_frame(typ: u8, payload: &[u8]) -> heapless::Vec<u8, 64> {
    use heapless::Vec;

    // Determine checksum length
    #[cfg(all(feature = "crc16", feature = "crc16_2b"))]
    const CHK_LEN: u8 = 2;
    #[cfg(all(feature = "crc16", not(feature = "crc16_2b")))]
    const CHK_LEN: u8 = 1;
    #[cfg(not(feature = "crc16"))]
    const CHK_LEN: u8 = 1;

    // LEN = TYPE(1) + PAYLOAD(N) + CHECKSUM(CHK_LEN)
    let len: u8 = (1 + payload.len() as u8 + CHK_LEN) as u8;

    // === compute checksum material: [LEN, TYPE, PAYLOAD...] ===
    let mut tmp: [u8; 66] = [0; 66];
    let mut t = 0usize;
    tmp[t] = len;
    t += 1;
    tmp[t] = typ;
    t += 1;
    for &b in payload {
        tmp[t] = b;
        t += 1;
    }

    #[cfg(all(feature = "crc16", feature = "crc16_2b"))]
    let (chk_hi, chk_lo) = {
        let crc = crc16_ccitt(&tmp[..t]);
        (((crc >> 8) & 0xFF) as u8, (crc & 0xFF) as u8)
    };

    #[cfg(all(feature = "crc16", not(feature = "crc16_2b")))]
    let chk_lsb: u8 = {
        // LSB-only legacy mode
        (crc16_ccitt(&tmp[..t]) & 0xFF) as u8
    };

    #[cfg(not(feature = "crc16"))]
    let chk8: u8 = {
        let mut s: u16 = 0;
        for &b in &tmp[..t] {
            s = s.wrapping_add(b as u16);
        }
        !(s as u8)
    };

    // Helper that escapes bytes (after SYNC) if `byte_stuff` is enabled.
    #[inline]
    fn push_stuffed<const CAP: usize>(v: &mut heapless::Vec<u8, CAP>, b: u8) {
        #[cfg(feature = "byte_stuff")]
        {
            if b == 0xAA || b == 0x7D {
                v.push(0x7D).ok();
                v.push(b ^ 0x20).ok();
            } else {
                v.push(b).ok();
            }
        }
        #[cfg(not(feature = "byte_stuff"))]
        {
            v.push(b).ok();
        }
    }

    let mut v: Vec<u8, 64> = Vec::new();
    v.push(0xAA).ok(); // SYNC (never escaped)
    push_stuffed(&mut v, len); // LEN
    push_stuffed(&mut v, typ); // TYPE
    for &b in payload {
        push_stuffed(&mut v, b);
    }

    // Append checksum according to mode
    #[cfg(all(feature = "crc16", feature = "crc16_2b"))]
    {
        // Two-byte CRC: MSB then LSB (parity with C)
        push_stuffed(&mut v, chk_hi);
        push_stuffed(&mut v, chk_lo);
    }

    #[cfg(all(feature = "crc16", not(feature = "crc16_2b")))]
    {
        // One-byte CRC: LSB only
        push_stuffed(&mut v, chk_lsb);
    }

    #[cfg(not(feature = "crc16"))]
    {
        // 8-bit sum mode
        push_stuffed(&mut v, chk8);
    }

    // Host-friendly debug prints (std only)
    #[cfg(feature = "std")]
    {
        #[cfg(all(feature = "crc16", feature = "crc16_2b"))]
        println!(
            "[make_frame] typ=0x{typ:02X} len={len} payload_len={} crc={:02X} {:02X}",
            payload.len(),
            chk_hi,
            chk_lo
        );

        #[cfg(all(feature = "crc16", not(feature = "crc16_2b")))]
        println!(
            "[make_frame] typ=0x{typ:02X} len={len} payload_len={} crc_lsb={:02X}",
            payload.len(),
            chk_lsb
        );

        #[cfg(not(feature = "crc16"))]
        println!(
            "[make_frame] typ=0x{typ:02X} len={len} payload_len={} chk8={:02X}",
            payload.len(),
            chk8
        );

        print!("[make_frame] bytes:");
        for b in &v {
            print!(" {:02X}", b);
        }
        println!();
    }

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

    while now <= max_ms {
        // Simulate ISR pushing variable-length chunks.
        if cur < input.len() {
            let n = sizes.next().unwrap_or(8).min(input.len() - cur);
            for i in 0..n {
                let _ = rb.push(input[cur + i]);
            }
            cur += n;
        }

        // Pump RB → parser → FSM
        while let Some(b) = rb.pop() {
            if let Some(pkt) = parser.step(b, || {
                #[cfg(feature = "std")]
                eprint!("[parser] invalid byte {b:02X}\n");
            }) {
                dev.handle_packet(pkt);
            }
            dev.on_invalid_consecutive(parser.invalid_consecutive());
        }

        dev.tick(now);
        now += 1;
    }
}

pub fn run_capture(input: &[u8], chunk_sizes: &[usize], max_ms: u32) -> heapless::String<1024> {
    let mut rb: RingBuf<32> = RingBuf::new(); // small to exercise overflow behavior
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
            if let Some(pkt) = parser.step(b, || {
                #[cfg(feature = "std")]
                eprint!("[parser] invalid byte {b:02X}\n");
            }) {
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
