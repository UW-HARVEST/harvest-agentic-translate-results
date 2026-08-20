//! `driver` executable -- the translation of `c_src/src/container_of.c`.
//!
//! This binary reconstructs a C-style `argv` vector from the process arguments
//! and hands it to the very same [`container_of::c_main`] that the exported
//! `main` symbol of `libdriver.so` calls, so the executable and the shared
//! library cannot drift apart.
//!
//! The module is included by path rather than through the library crate because
//! the library is a `cdylib` that exports its own `main` symbol; linking it into
//! an executable would collide with the executable's entry point.
#[path = "container_of.rs"]
mod container_of;

use std::ffi::{c_char, c_int, CString, OsString};

fn main() {
    // Rust's runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, whereas a
    // C program inherits the default disposition. Restore the default so that a
    // vanishing stdout reader kills this process with `SIGPIPE`, exactly as it
    // kills the C `driver`.
    restore_default_sigpipe();

    // Rebuild `argv`: the real C `argv` is an array of NUL-terminated strings
    // terminated by a NULL pointer.
    let owned: Vec<CString> = std::env::args_os().map(os_string_to_cstring).collect();
    let argc = owned.len() as c_int;

    let mut argv: Vec<*mut c_char> = owned.iter().map(|s| s.as_ptr() as *mut c_char).collect();
    // The terminating NULL, plus one spare NULL slot. The C runtime places
    // `envp` immediately after the terminator, so `argv[2]` is always readable
    // memory there; the spare slot reproduces that readability without changing
    // any observable value, because the C code always faults on `argv[1]`
    // before it ever loads `argv[2]`.
    argv.push(std::ptr::null_mut());
    argv.push(std::ptr::null_mut());

    // `int main(int argc, char** argv)`; the C `main` returns 0 by falling off
    // the end of its body.
    let status = unsafe { container_of::c_main(argc, argv.as_mut_ptr()) };

    // Keep `owned` alive until after the call: `argv` only borrows its buffers.
    drop(owned);

    std::process::exit(status);
}

/// Converts one process argument into the NUL-terminated form C sees.
///
/// Arguments handed to a process can never contain an interior NUL byte, so the
/// conversion cannot fail in practice; should it somehow, truncating at the
/// first NUL is what a C program would observe anyway.
fn os_string_to_cstring(s: OsString) -> CString {
    let bytes = os_string_to_bytes(s);
    match CString::new(bytes.clone()) {
        Ok(c) => c,
        Err(_) => {
            let first_nul = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            CString::new(&bytes[..first_nul]).expect("truncated at first NUL")
        }
    }
}

#[cfg(unix)]
fn os_string_to_bytes(s: OsString) -> Vec<u8> {
    use std::os::unix::ffi::OsStringExt;
    s.into_vec()
}

#[cfg(not(unix))]
fn os_string_to_bytes(s: OsString) -> Vec<u8> {
    s.to_string_lossy().into_owned().into_bytes()
}

#[cfg(unix)]
fn restore_default_sigpipe() {
    const SIGPIPE: c_int = 13;
    const SIG_DFL: usize = 0;

    extern "C" {
        fn signal(signum: c_int, handler: usize) -> usize;
    }

    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

#[cfg(not(unix))]
fn restore_default_sigpipe() {}
