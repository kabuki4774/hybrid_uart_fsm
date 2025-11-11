//! Simple ring buffer implementation
//! Supports no_std environments.
//! Provides fixed-capacity storage for bytes.
//! Uses generic const parameters for buffer size.
//! Implements push and pop operations.
//! Handles full and empty conditions.
//! Uses modular arithmetic for index management.
//! Employs basic error handling.
//! Intended for use in UART FSM and parser.

pub struct RingBuf<const N: usize> {
    buf: [u8; N],
    head: usize,
    tail: usize,
    full: bool,
}

impl<const N: usize> RingBuf<N> {
    pub fn new() -> Self {
        Self {
            buf: [0; N],
            head: 0,
            tail: 0,
            full: false,
        }
    }
    fn adv(i: usize) -> usize {
        (i + 1) % N
    }
    pub fn is_empty(&self) -> bool {
        !self.full && self.head == self.tail
    }
    pub fn is_full(&self) -> bool {
        self.full
    }
    pub fn len(&self) -> usize {
        if self.full {
            N
        } else if self.head >= self.tail {
            self.head - self.tail
        } else {
            N - (self.tail - self.head)
        }
    }
    pub fn push(&mut self, b: u8) -> Result<(), ()> {
        if self.full {
            return Err(());
        }
        self.buf[self.head] = b;
        self.head = Self::adv(self.head);
        self.full = self.head == self.tail;
        Ok(())
    }
    pub fn pop(&mut self) -> Option<u8> {
        if self.is_empty() {
            return None;
        }
        let b = self.buf[self.tail];
        self.tail = Self::adv(self.tail);
        self.full = false;
        Some(b)
    }
}
