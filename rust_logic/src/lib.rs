use std::os::raw::c_int;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::sync::{mpsc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

// Expose selected C-side parser symbols for internal testing (unchanged)
extern "C" {
    fn parser_init(ptr: *mut std::ffi::c_void);
    fn parser_feed_byte(ptr: *mut std::ffi::c_void, byte: u8, out: *mut std::ffi::c_void) -> c_int;
}

// ---- Bring in the Rust UART implementation (feature-driven) ----
use uart_fsm_rs::harness::make_frame;
use uart_fsm_rs::parser::{CmdType, Parser as RsParser};

/// Friendly hello for smoke tests
#[no_mangle]
pub extern "C" fn rust_hello() {
    println!("Hello from Rust! 🦀");
}

/// CRC16-CCITT helper (unchanged signature)
#[no_mangle]
pub extern "C" fn rust_crc16(buf: *const u8, len: usize) -> u16 {
    if buf.is_null() {
        return 0;
    }
    let data = unsafe { std::slice::from_raw_parts(buf, len) };
    // Same polynomial/seed as uart_fsm_rs::crc::crc16_ccitt
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            if (crc & 0x8000) != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

/// New: build a frame using uart_fsm_rs::harness::make_frame (honors features automatically)
///
/// Returns the number of bytes written to `out` (0 on error/overflow).
#[no_mangle]
pub extern "C" fn rust_build_frame(
    typ: u8,
    payload: *const u8,
    len: usize,
    out: *mut u8,
    cap: usize,
) -> usize {
    if out.is_null() || cap == 0 {
        return 0;
    }
    if len > 0 && payload.is_null() {
        return 0;
    }

    let pay = if len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(payload, len) }
    };

    let v = make_frame(typ, pay); // feature-aware builder
    if v.len() > cap {
        return 0;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(v.as_ptr(), out, v.len());
    }
    v.len()
}

#[no_mangle]
pub extern "C" fn rust_make_frame(
    typ: u8,
    payload: *const u8,
    len: usize,
    out: *mut u8,
    cap: usize,
) -> usize {
    use uart_fsm_rs::harness::make_frame;
    if out.is_null() || cap == 0 {
        return 0;
    }
    let pay = if !payload.is_null() && len > 0 {
        unsafe { std::slice::from_raw_parts(payload, len) }
    } else {
        &[]
    };
    let frame_h = make_frame(typ, pay); // heapless or std Vec, feature-aware
    let slice = frame_h.as_slice(); // works for both
    let n = slice.len().min(cap);
    unsafe {
        std::ptr::copy_nonoverlapping(slice.as_ptr(), out, n);
    }
    n
}

/// Parse and act on a single frame using the *feature-aware* Rust parser.
/// Prints START/STOP/PONG/RESET messages on success, returns 0 if a valid
/// packet is seen, otherwise -1.
fn process_frame(bytes: &[u8]) -> c_int {
    let mut p = RsParser::new();
    let mut ok = false;

    for &b in bytes {
        if let Some(pkt) = p.step(b, || {}) {
            match pkt.typ {
                CmdType::Start => {
                    println!("Rust(worker): START");
                }
                CmdType::Stop => {
                    println!("Rust(worker): STOP");
                }
                CmdType::Ping => {
                    // Best-effort UTF-8 print
                    match core::str::from_utf8(&pkt.payload) {
                        Ok(s) => println!("Rust(worker): PONG {s}"),
                        Err(_) => println!("Rust(worker): PONG <bin>"),
                    }
                }
                CmdType::Reset => {
                    println!("Rust(worker): RESET");
                }
                CmdType::Unknown(x) => {
                    println!("Rust(worker): Unknown TYPE 0x{x:02X}");
                }
            }
            ok = true;
            // In this harness we expect one packet per submitted frame; keep consuming to be robust.
        }
    }

    if ok {
        0
    } else {
        -1
    }
}

/// FFI entry for fuzzers/differential tests: >=0 valid, <0 invalid
#[no_mangle]
pub extern "C" fn rust_process_frame(buf: *const u8, len: usize) -> c_int {
    if buf.is_null() || len == 0 {
        eprintln!("Rust: invalid frame pointer/length.");
        return -1;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buf, len) };
    process_frame(bytes)
}

// ====================== Worker thread state (unchanged) ======================

static SENDER: OnceLock<mpsc::Sender<Vec<u8>>> = OnceLock::new();
static STOP: AtomicBool = AtomicBool::new(false);
static HANDLE: OnceLock<Mutex<Option<thread::JoinHandle<()>>>> = OnceLock::new();

#[no_mangle]
pub extern "C" fn rust_worker_start() -> c_int {
    if SENDER.get().is_some() {
        return 0;
    } // already started
    STOP.store(false, Ordering::SeqCst);
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    let handle = thread::spawn(move || {
        let mut last_hb = Instant::now();
        loop {
            if STOP.load(Ordering::SeqCst) {
                break;
            }
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(frame) => {
                    let _ = process_frame(&frame);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if last_hb.elapsed() >= Duration::from_millis(1000) {
                        println!("Rust(worker): HEARTBEAT");
                        last_hb = Instant::now();
                    }
                }
                Err(_) => break, // channel closed
            }
        }
    });

    let _ = SENDER.set(tx);
    let _ = HANDLE.set(Mutex::new(Some(handle)));
    0
}

#[no_mangle]
pub extern "C" fn rust_worker_submit(buf: *const u8, len: usize) -> c_int {
    let Some(tx) = SENDER.get() else {
        return -1;
    };
    if buf.is_null() || len == 0 {
        return -2;
    }
    let bytes = unsafe { std::slice::from_raw_parts(buf, len) };
    match tx.send(bytes.to_vec()) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

#[no_mangle]
pub extern "C" fn rust_worker_send_cmd(typ: u8, payload: *const u8, len: usize) -> c_int {
    use uart_fsm_rs::harness::make_frame;
    let Some(tx) = SENDER.get() else {
        return -1;
    };
    let pay = if !payload.is_null() && len > 0 {
        unsafe { std::slice::from_raw_parts(payload, len) }
    } else {
        &[]
    };
    let frame_h = make_frame(typ, pay);
    let frame: std::vec::Vec<u8> = frame_h.as_slice().to_vec(); // avoid heapless/std mismatch
    match tx.send(frame) {
        Ok(_) => 0,
        Err(_) => -3,
    }
}

#[no_mangle]
pub extern "C" fn rust_worker_stop() -> c_int {
    STOP.store(true, Ordering::SeqCst);
    if let Some(tx) = SENDER.get() {
        drop(tx.clone());
    } // close channel

    if let Some(h_mutex) = HANDLE.get() {
        if let Some(handle) = h_mutex.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
    0
}

// ---------------- Test helpers (unchanged) ----------------

fn run_c_parser(input: &[u8]) -> i32 {
    #[repr(C)]
    struct Parser {
        _buf: [u8; 256],
    }
    let mut parser = Parser { _buf: [0; 256] };
    unsafe {
        parser_init((&mut parser) as *mut _ as *mut std::ffi::c_void);
    }
    let mut total = 0;
    for &b in input {
        unsafe {
            total += parser_feed_byte(
                (&mut parser) as *mut _ as *mut std::ffi::c_void,
                b,
                std::ptr::null_mut(),
            );
        }
    }
    total
}

fn run_rust_parser(input: &[u8]) -> i32 {
    unsafe { rust_process_frame(input.as_ptr(), input.len()) }
}
