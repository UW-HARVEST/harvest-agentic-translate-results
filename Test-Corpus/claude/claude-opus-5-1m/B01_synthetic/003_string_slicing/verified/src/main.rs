// Copyright 2025 MIT Lincoln Laboratory
// Permission is hereby granted, free of charge,
// to any person obtaining a copy of this software
// and associated documentation files (the "Software"),
// to deal in the Software without restriction,
// including without limitation the rights to use, copy,
// modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software,
// and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
// EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
// IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE
// FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT,
// TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE
// OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Executable view of the translated program: the exact equivalent of running
//! the C `driver` binary.

mod imp;

use core::ffi::{c_char, c_int};

/// The `(argc, argv)` pair the process was started with.
///
/// The C `main` receives the raw vector the kernel placed on the stack, and it
/// happily indexes `argv[1]` even when `argc == 0`. To be able to reproduce
/// that byte for byte, grab the real vector instead of rebuilding one from
/// `std::env::args_os()`: on glibc every `.init_array` entry is called with
/// `(argc, argv, envp)`.
static mut REAL_ARGC: c_int = 0;
static mut REAL_ARGV: *mut *mut c_char = core::ptr::null_mut();

#[cfg(all(target_os = "linux", target_env = "gnu"))]
#[used]
#[link_section = ".init_array"]
static CAPTURE_ARGV: extern "C" fn(c_int, *mut *mut c_char, *mut *mut c_char) = capture_argv;

#[cfg(all(target_os = "linux", target_env = "gnu"))]
extern "C" fn capture_argv(argc: c_int, argv: *mut *mut c_char, _envp: *mut *mut c_char) {
    unsafe {
        REAL_ARGC = argc;
        REAL_ARGV = argv;
    }
}

/// Fallback for platforms where the real vector cannot be captured: rebuild a
/// C-style `argv` from the arguments the runtime reports.
fn run_from_env() -> c_int {
    #[cfg(unix)]
    use std::os::unix::ffi::OsStrExt;

    let mut owned: Vec<Vec<u8>> = Vec::new();
    for arg in std::env::args_os() {
        #[cfg(unix)]
        let mut bytes = arg.as_bytes().to_vec();
        #[cfg(not(unix))]
        let mut bytes = arg.to_string_lossy().into_owned().into_bytes();
        bytes.push(0);
        owned.push(bytes);
    }

    let mut argv: Vec<*mut c_char> = owned
        .iter_mut()
        .map(|b| b.as_mut_ptr() as *mut c_char)
        .collect();
    argv.push(core::ptr::null_mut());

    let argc = owned.len() as c_int;
    unsafe { imp::c_main(argc, argv.as_mut_ptr()) }
}

extern "C" {
    /// `signal(2)`; the handler is passed as a plain address so no function
    /// pointer type gymnastics are needed for `SIG_DFL`.
    fn signal(signum: c_int, handler: usize) -> usize;
}

const SIGPIPE: c_int = 13;
const SIG_DFL: usize = 0;

/// The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, so a failed
/// write would merely return `EPIPE` and the program would exit 0. A C program
/// keeps the default disposition and is killed by `SIGPIPE` instead (e.g.
/// `driver <long string> | head -c 5`). Restore the C behavior.
fn restore_default_sigpipe() {
    unsafe {
        signal(SIGPIPE, SIG_DFL);
    }
}

fn main() {
    restore_default_sigpipe();

    let (argc, argv) = unsafe { (REAL_ARGC, REAL_ARGV) };

    let status = if argv.is_null() {
        run_from_env()
    } else {
        unsafe { imp::c_main(argc, argv) }
    };

    // `return status;` from C's main.
    std::process::exit(status);
}
