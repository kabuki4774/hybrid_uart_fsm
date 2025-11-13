use std::{env, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // ---------- Compile C ----------
    cc::Build::new()
        .files([
            "../uart_fsm_c/src/parser.c",
            "../uart_fsm_c/src/fsm.c",
            "../uart_fsm_c/src/ringbuf.c",
            "../uart_fsm_c/src/harness.c",
        ])
        .include("../uart_fsm_c/src")
        .flag_if_supported("-O2")
        .flag_if_supported("-fvisibility=default")
        .compile("uartfsm_c");

    // ---------- Link so ASan/LTO always pulls it ----------
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=uartfsm_c");

    // Keep force-load of the C archive if you rely on its symbols in bins:
    let lib = out_dir.join("libuartfsm_c.a");
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-arg=-Wl,-force_load");
        println!("cargo:rustc-link-arg={}", lib.display());
    } else {
        println!("cargo:rustc-link-arg=-Wl,--whole-archive");
        println!("cargo:rustc-link-lib=static=uartfsm_c");
        println!("cargo:rustc-link-arg=-Wl,--no-whole-archive");
    }

    for f in [
        "../uart_fsm_c/src/parser.c",
        "../uart_fsm_c/src/fsm.c",
        "../uart_fsm_c/src/ringbuf.c",
        "../uart_fsm_c/src/harness.c",
    ] {
        println!("cargo:rerun-if-changed={}", f);
    }
}
