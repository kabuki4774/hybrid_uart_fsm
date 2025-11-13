#![no_main]

use libfuzzer_sys::fuzz_target;

extern "C" {
    fn parser_init(ptr: *mut core::ffi::c_void);
    fn parser_feed_byte(ptr: *mut core::ffi::c_void, b: u8, out: *mut core::ffi::c_void) -> i32;
}

#[repr(C)]
struct Parser {
    // Opaque buffer large enough to hold `parser_t` from C.
    _buf: [u8; 256],
}

fn eval_c(bytes: &[u8]) -> i32 {
    let mut p = Parser { _buf: [0; 256] };
    unsafe {
        parser_init((&mut p) as *mut _ as *mut core::ffi::c_void);
    }
    let mut acc = 0;
    for &b in bytes {
        unsafe {
            // We intentionally pass a null `out` pointer here.
            acc += parser_feed_byte(
                (&mut p) as *mut _ as *mut core::ffi::c_void,
                b,
                core::ptr::null_mut(),
            );
        }
    }
    acc
}

use rust_logic::rust_process_frame;

fn eval_rust(bytes: &[u8]) -> i32 {
    unsafe { rust_process_frame(bytes.as_ptr(), bytes.len()) }
}

fuzz_target!(|data: &[u8]| {
    let _ = (eval_c(data) >= 0, eval_rust(data) >= 0);
});
