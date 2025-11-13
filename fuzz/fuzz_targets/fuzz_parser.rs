// fuzz/fuzz_targets/fuzz_parser.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use rust_logic::rust_process_frame;
fuzz_target!(|data: &[u8]| {
    unsafe {
        rust_process_frame(data.as_ptr(), data.len());
    }
});
