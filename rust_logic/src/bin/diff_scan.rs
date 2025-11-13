//! diff_scan: compare Rust vs C parser decisions over given paths.
//! Usage: cargo run -p rust_logic --bin diff_scan -- <dir-or-file>...
use std::{env, fs, path::Path};
// Pull the symbol from our library crate so Cargo links the lib
use rust_logic::rust_process_frame;

extern "C" {
    fn parser_init(ptr: *mut std::ffi::c_void);
    fn parser_feed_byte(ptr: *mut std::ffi::c_void, b: u8, out: *mut std::ffi::c_void) -> i32;
}

#[repr(C)]
struct Parser {
    _buf: [u8; 256],
}

fn eval_c(bytes: &[u8]) -> i32 {
    let mut p = Parser { _buf: [0; 256] };
    unsafe {
        parser_init((&mut p) as *mut _ as *mut std::ffi::c_void);
    }
    let mut acc = 0;
    for &b in bytes {
        unsafe {
            acc += parser_feed_byte(
                (&mut p) as *mut _ as *mut std::ffi::c_void,
                b,
                std::ptr::null_mut(),
            );
        }
    }
    acc
}
fn eval_rust(bytes: &[u8]) -> i32 {
    // This now calls the item from our lib crate, not an unresolved extern.
    unsafe { rust_process_frame(bytes.as_ptr(), bytes.len()) }
}

fn visit(path: &Path, files: &mut Vec<String>) {
    if path.is_file() {
        files.push(path.display().to_string());
    } else if path.is_dir() {
        if let Ok(rd) = fs::read_dir(path) {
            for e in rd.flatten() {
                visit(&e.path(), files);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: diff_scan <dir-or-file>...");
        std::process::exit(2);
    }
    let mut files = Vec::new();
    for a in &args {
        visit(Path::new(a), &mut files);
    }

    let mut ok = 0usize;
    let mut mis = 0usize;
    let mut err = 0usize;

    for f in &files {
        let data = match fs::read(f) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("ERR READ {f}: {e}");
                err += 1;
                continue;
            }
        };
        let r = eval_rust(&data);
        let c = eval_c(&data);
        let r_ok = r >= 0;
        let c_ok = c >= 0;
        if r_ok == c_ok {
            println!("OK {f} rust={r} c={c}");
            ok += 1;
        } else {
            println!("MISMATCH {f} rust={r} c={c}");
            mis += 1;
        }
    }
    println!(
        "TOTAL files={} ok={} mismatch={} err={}",
        files.len(),
        ok,
        mis,
        err
    );
    if mis > 0 {
        std::process::exit(1);
    }
}
