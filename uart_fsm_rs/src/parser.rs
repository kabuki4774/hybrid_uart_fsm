//! UART packet parser implementation
//! Supports no_std environments with optional alloc support.
//! Implements byte-stuffing and CRC16 checksum as optional features.
//! Provides a state machine for parsing incoming bytes into packets.
//! Uses heapless data structures for fixed-capacity storage.

use heapless::Vec as HVec;

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

#[derive(Debug, Clone)]
pub struct Packet {
    pub typ: CmdType,
    pub payload: HVec<u8, 24>,
}

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
        sum: u16,
    },
}

#[cfg(feature = "crc16")]
fn crc16_ccitt(buf: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in buf {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if (crc & 0x8000) != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

pub struct Parser {
    st: St,
    payload: HVec<u8, 24>,
    invalid_consec: u32,
    #[cfg(feature = "byte_stuff")]
    escape_next: bool,
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
        }
    }

    #[inline]
    fn cs(len: u8, typ: u8, pay_sum: u16) -> u8 {
        let s = ((len as u16) + (typ as u16) + pay_sum) & 0xFF;
        !(s as u8)
    }
    pub fn invalid_consecutive(&self) -> u32 {
        self.invalid_consec
    }

    pub fn step<F>(&mut self, mut b: u8, mut on_invalid: F) -> Option<Packet>
    where
        F: FnMut(),
    {
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

        use St::*;
        match self.st {
            WaitSync => {
                if b == Self::SYNC {
                    self.st = ReadLen;
                } else {
                    self.invalid_consec += 1;
                    on_invalid();
                }
                None
            }
            ReadLen => {
                if !(2..=32).contains(&b) {
                    self.invalid_consec += 1;
                    on_invalid();
                    self.st = WaitSync;
                    self.payload.clear();
                    return None;
                }
                self.st = ReadType { len: b };
                None
            }
            ReadType { len } => {
                let typ = b;
                let pay_len = len.saturating_sub(2);
                self.payload.clear();
                self.st = ReadPayload {
                    len,
                    typ,
                    remain: pay_len,
                    sum: 0,
                };
                None
            }
            ReadPayload {
                len,
                typ,
                mut remain,
                mut sum,
            } => {
                if remain > 0 {
                    if self.payload.push(b).is_err() {
                        self.invalid_consec += 1;
                        on_invalid();
                        self.st = WaitSync;
                        self.payload.clear();
                        return None;
                    }
                    sum = sum.wrapping_add(b as u16);
                    remain -= 1;
                    self.st = ReadPayload {
                        len,
                        typ,
                        remain,
                        sum,
                    };
                    None
                } else {
                    // Checksum verification
                    #[cfg(not(feature = "crc16"))]
                    let valid = b == Self::cs(len, typ, sum);
                    #[cfg(feature = "crc16")]
                    let valid = {
                        let mut tmp = heapless::Vec::<u8, 64>::new();
                        tmp.push(len).ok();
                        tmp.push(typ).ok();
                        tmp.extend_from_slice(&self.payload).ok();
                        crc16_ccitt(&tmp) == (b as u16)
                    };

                    if !valid {
                        self.invalid_consec += 1;
                        on_invalid();
                        self.st = WaitSync;
                        self.payload.clear();
                        return None;
                    }
                    let pkt = Packet {
                        typ: CmdType::from(typ),
                        payload: self.payload.clone(),
                    };
                    self.payload.clear();
                    self.st = WaitSync;
                    self.invalid_consec = 0;
                    Some(pkt)
                }
            }
        }
    }
}
