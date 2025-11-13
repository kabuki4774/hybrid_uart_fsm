//! UART packet parser implementation (spec-correct with optional CRC modes)
//!
//! Frame layout:
//!   [SYNC=0xAA][LEN][TYPE][PAYLOAD...][CHECKSUM]
//!
//! Spec notes:
//! - LEN is the number of bytes from TYPE through CHECKSUM (inclusive). Valid: 2..=32.
//! - 8-bit mode: CHECKSUM = ~((LEN + TYPE + sum(payload)) & 0xFF).  (SYNC and CHECKSUM itself excluded)
//! - CRC16 one-byte mode (`crc16` only): validate **LSB only** of CRC16-CCITT over [LEN, TYPE, PAYLOAD...]
//! - CRC16 two-byte mode (`crc16` + `crc16_2b`): expect **MSB then LSB** (two bytes total) over [LEN, TYPE, PAYLOAD...]
//!
//! Byte-stuffing (`byte_stuff`) unescapes 0x7D ^ 0x20 for any byte AFTER SYNC.

use heapless::Vec as HVec;

#[cfg(feature = "crc16")]
use crate::crc::crc16_ccitt;

/// Command types as per the protocol spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmdType {
    Start,
    Stop,
    Ping,
    Reset,
    Unknown(u8),
}

impl From<u8> for CmdType {
    fn from(v: u8) -> Self {
        match v {
            0x01 => Self::Start,
            0x02 => Self::Stop,
            0x03 => Self::Ping,
            0xFF => Self::Reset,
            _ => Self::Unknown(v),
        }
    }
}

/// Parsed packet presented to the application.
#[derive(Debug, Clone)]
pub struct Packet {
    pub typ: CmdType,
    pub payload: HVec<u8, 24>,
}

/// Internal parser states.
#[derive(Clone, Copy)]
enum St {
    WaitSync,
    ReadLen,
    ReadType {
        len: u8,
    },
    ReadPayload {
        len: u8,
        typ: u8,
        remain: u8,
        sum: u16, // running sum of payload bytes for 8-bit mode
    },
}

pub struct Parser {
    st: St,
    payload: HVec<u8, 24>,
    invalid_consec: u32,
    #[cfg(feature = "byte_stuff")]
    escape_next: bool,
    #[cfg(all(feature = "crc16", feature = "crc16_2b"))]
    crc_hi: Option<u8>, // first CRC byte (MSB) when two-byte mode is enabled
}

impl Parser {
    pub const SYNC: u8 = 0xAA;

    pub fn new() -> Self {
        Self {
            st: St::WaitSync,
            payload: HVec::new(),
            invalid_consec: 0,
            #[cfg(feature = "byte_stuff")]
            escape_next: false,
            #[cfg(all(feature = "crc16", feature = "crc16_2b"))]
            crc_hi: None,
        }
    }

    // ---------------- checksum helpers ----------------

    /// Spec 8‑bit checksum helper:
    ///   chk = ~((LEN + TYPE + sum(payload)) & 0xFF)
    #[cfg(not(feature = "crc16"))]
    #[inline]
    fn cs(len: u8, typ: u8, pay_sum: u16) -> u8 {
        let s = ((len as u16) + (typ as u16) + pay_sum) & 0xFF;
        !(s as u8)
    }

    /// Number of consecutive invalid bytes/frames seen.
    pub fn invalid_consecutive(&self) -> u32 {
        self.invalid_consec
    }

    /// Feed a single byte. Returns `Some(Packet)` when a full, valid frame is parsed.
    pub fn step<F>(&mut self, b: u8, mut on_invalid: F) -> Option<Packet>
    where
        F: FnMut(),
    {
        // Rebind 'b' conditionally to avoid `unused_mut` warnings when byte_stuff is off.
        #[cfg(feature = "byte_stuff")]
        let mut b = b;
        #[cfg(not(feature = "byte_stuff"))]
        let b = b;

        // --- Optional byte unstuffing ---
        #[cfg(feature = "byte_stuff")]
        {
            if self.escape_next {
                b ^= 0x20;
                self.escape_next = false;
            } else if b == 0x7D {
                self.escape_next = true;
                return None;
            }
        }

        // Determine checksum length at compile time
        #[cfg(all(feature = "crc16", feature = "crc16_2b"))]
        const CHK_LEN: u8 = 2;
        #[cfg(all(feature = "crc16", not(feature = "crc16_2b")))]
        const CHK_LEN: u8 = 1;
        #[cfg(not(feature = "crc16"))]
        const CHK_LEN: u8 = 1;

        use St::*;
        match self.st {
            // --- SYNC gate ---
            WaitSync => {
                if b == Self::SYNC {
                    self.st = ReadLen;
                } else {
                    self.invalid_consec = self.invalid_consec.saturating_add(1);
                    on_invalid();
                }
                None
            }

            // --- LEN ---
            ReadLen => {
                if !(2..=32).contains(&b) {
                    #[cfg(feature = "std")]
                    eprintln!("[parser] invalid LEN {}", b);
                    self.invalid_consec = self.invalid_consec.saturating_add(1);
                    on_invalid();
                    self.st = WaitSync;
                    self.payload.clear();
                    return None;
                }
                self.st = ReadType { len: b };
                None
            }

            // --- TYPE ---
            ReadType { len } => {
                let typ = b;
                // PAYLOAD length = LEN - (TYPE(1) + CHK_LEN)
                let pay_len = len.saturating_sub(1 + CHK_LEN);
                self.payload.clear();
                self.st = ReadPayload {
                    len,
                    typ,
                    remain: pay_len,
                    sum: 0,
                };
                None
            }

            // --- PAYLOAD then CHECKSUM ---
            ReadPayload {
                len,
                typ,
                mut remain,
                mut sum,
            } => {
                if remain > 0 {
                    // Collect payload
                    if self.payload.push(b).is_err() {
                        #[cfg(feature = "std")]
                        eprintln!("[parser] payload overflow");
                        self.invalid_consec = self.invalid_consec.saturating_add(1);
                        on_invalid();
                        self.st = WaitSync;
                        self.payload.clear();
                        return None;
                    }
                    #[cfg(not(feature = "crc16"))]
                    {
                        sum = sum.wrapping_add(b as u16);
                    }

                    remain -= 1;
                    self.st = ReadPayload {
                        len,
                        typ,
                        remain,
                        sum,
                    };
                    None
                } else {
                    // We are now receiving checksum byte(s)
                    //  - 8-bit mode: 'b' is the only checksum byte
                    //  - CRC16 LSB mode: 'b' is the LSB; compare against CRC LSB
                    //  - CRC16 two-byte mode: we expect MSB first, then LSB

                    // ---------- CRC16 two-byte mode ----------
                    #[cfg(all(feature = "crc16", feature = "crc16_2b"))]
                    {
                        if self.crc_hi.is_none() {
                            // First CRC byte (MSB)
                            self.crc_hi = Some(b);
                            return None;
                        } else {
                            // Second CRC byte (LSB) arrived → validate CRC
                            let hi = self.crc_hi.take().unwrap();
                            let mut tmp = heapless::Vec::<u8, 64>::new();
                            tmp.push(len).ok();
                            tmp.push(typ).ok();
                            tmp.extend_from_slice(&self.payload).ok();
                            let crc = crc16_ccitt(&tmp);
                            let exp_hi = ((crc >> 8) & 0xFF) as u8;
                            let exp_lo = (crc & 0xFF) as u8;

                            if hi != exp_hi || b != exp_lo {
                                #[cfg(feature = "std")]
                                eprintln!(
                                    "[parser] CRC16 mismatch typ=0x{:02X} len={} got={:02X} {:02X} exp={:02X} {:02X}",
                                    typ, len, hi, b, exp_hi, exp_lo
                                );
                                self.invalid_consec = self.invalid_consec.saturating_add(1);
                                on_invalid();
                                self.st = WaitSync;
                                self.payload.clear();
                                return None;
                            }

                            // ✅ Valid frame
                            let pkt = Packet {
                                typ: CmdType::from(typ),
                                payload: self.payload.clone(),
                            };
                            #[cfg(feature = "std")]
                            eprintln!(
                                "[parser] ✅ valid packet typ={:?} payload_len={}",
                                pkt.typ,
                                pkt.payload.len()
                            );

                            self.payload.clear();
                            self.invalid_consec = 0;
                            self.st = WaitSync;
                            return Some(pkt);
                        }
                    }

                    // ---------- CRC16 one-byte LSB mode ----------
                    #[cfg(all(feature = "crc16", not(feature = "crc16_2b")))]
                    {
                        let mut tmp = heapless::Vec::<u8, 64>::new();
                        tmp.push(len).ok();
                        tmp.push(typ).ok();
                        tmp.extend_from_slice(&self.payload).ok();
                        let crc = crc16_ccitt(&tmp);
                        let lsb = (crc & 0xFF) as u8;

                        if b != lsb {
                            #[cfg(feature = "std")]
                            eprintln!(
                                "[parser] CRC16-LSB mismatch typ=0x{:02X} len={} got={:02X} exp={:02X}",
                                typ, len, b, lsb
                            );
                            self.invalid_consec = self.invalid_consec.saturating_add(1);
                            on_invalid();
                            self.st = WaitSync;
                            self.payload.clear();
                            return None;
                        }

                        // ✅ Valid frame
                        let pkt = Packet {
                            typ: CmdType::from(typ),
                            payload: self.payload.clone(),
                        };
                        #[cfg(feature = "std")]
                        eprintln!(
                            "[parser] ✅ valid packet typ={:?} payload_len={}",
                            pkt.typ,
                            pkt.payload.len()
                        );

                        self.payload.clear();
                        self.invalid_consec = 0;
                        self.st = WaitSync;
                        return Some(pkt);
                    }

                    // ---------- 8-bit checksum mode ----------
                    #[cfg(not(feature = "crc16"))]
                    {
                        let expected = Self::cs(len, typ, sum);
                        if b != expected {
                            #[cfg(feature = "std")]
                            eprintln!(
                                "[parser] checksum mismatch typ=0x{:02X} len={} got={:02X} exp={:02X}",
                                typ, len, b, expected
                            );
                            self.invalid_consec = self.invalid_consec.saturating_add(1);
                            on_invalid();
                            self.st = WaitSync;
                            self.payload.clear();
                            return None;
                        }

                        // ✅ Valid frame
                        let pkt = Packet {
                            typ: CmdType::from(typ),
                            payload: self.payload.clone(),
                        };
                        #[cfg(feature = "std")]
                        eprintln!(
                            "[parser] ✅ valid packet typ={:?} payload_len={}",
                            pkt.typ,
                            pkt.payload.len()
                        );

                        self.payload.clear();
                        self.invalid_consec = 0;
                        self.st = WaitSync;
                        return Some(pkt);
                    }
                }
            }
        }
    }
}
