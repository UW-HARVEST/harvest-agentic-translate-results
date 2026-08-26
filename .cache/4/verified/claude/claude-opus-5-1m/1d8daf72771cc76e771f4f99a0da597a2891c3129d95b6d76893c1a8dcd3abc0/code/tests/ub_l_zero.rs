//! Phase C — the one reachable input for which the C itself invokes undefined
//! behaviour (`ERRORS.md` row 5).
//!
//! `int l = strlen(src) + 1;` (`lib.c:49`) truncates to `int`, so for
//! `strlen(src) == 4294967295` the result is `l == 0`. Then
//!
//!   * `calloc(sizeof(char), l + 13)` == `calloc(1, 13)`  -> succeeds,
//!   * `malloc(l)` == `malloc(0)`                          -> succeeds,
//!
//! so neither NULL check fires, and the filter loop (`lib.c:67-71`) proceeds to
//! write `strlen(src)` bytes into the zero-byte scratch buffer. The C has no
//! defined behaviour here; what it does in practice is die from SIGSEGV, and the
//! translation must not turn that into something else (a silent success, a
//! different signal, or a Rust panic message on stderr).
//!
//! Each call is made in a forked child so the heap smashing cannot affect the
//! test process, and the two children are compared on exit status.

mod common;

use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut c_int, options: c_int) -> i32;
    fn _exit(code: c_int) -> !;
}

fn mem_available_bytes() -> u64 {
    let s = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            if let Some(kb) = rest.split_whitespace().next() {
                if let Ok(v) = kb.parse::<u64>() {
                    return v * 1024;
                }
            }
        }
    }
    0
}

/// Runs `f(src)` in a child; returns `(signal, exit_code, returned_null)`.
fn call_in_child(f: common::DecodeFn, src: *const c_char) -> (c_int, c_int) {
    let pid = unsafe { fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        let p = unsafe { f(src) };
        // 0 = survived and returned NULL, 1 = survived and returned a pointer
        let code = if p.is_null() { 0 } else { 1 };
        if !p.is_null() {
            unsafe { common::free(p as *mut c_void) };
        }
        unsafe { _exit(code) }
    }
    let mut status: c_int = 0;
    let r = unsafe { waitpid(pid, &mut status, 0) };
    assert_eq!(r, pid, "waitpid failed");
    (status & 0x7f, (status >> 8) & 0xff)
}

/// ERRORS.md row 5 — `strlen(src) == 4294967295` makes `l == 0`: both
/// allocations succeed and the C then overruns a zero-byte buffer. C and Rust
/// must fail in exactly the same way.
#[test]
fn e5_l_zero_undefined_behaviour_matches() {
    let n = 4_294_967_296usize; // 4 GiB buffer, NUL in the last byte
    let need = n as u64 + (1 << 30);
    assert!(
        mem_available_bytes() >= need,
        "not enough free memory ({} MiB) for the 4 GiB l==0 test",
        mem_available_bytes() >> 20
    );
    let mut buf = vec![b'A'; n];
    buf[n - 1] = 0;
    let src = buf.as_ptr() as *const c_char;
    assert_eq!(
        (((n - 1) as i32).wrapping_add(1)),
        0,
        "test bug: this input must produce l == 0"
    );

    let a = common::api();
    let (c_sig, c_exit) = call_in_child(a.c, src);
    let (r_sig, r_exit) = call_in_child(a.rust, src);

    eprintln!("l == 0: C -> signal {c_sig}/exit {c_exit}, Rust -> signal {r_sig}/exit {r_exit}");
    assert_eq!(
        (c_sig, c_exit),
        (r_sig, r_exit),
        "l == 0 behaviour differs: C died with signal {c_sig} (exit {c_exit}) but \
         Rust with signal {r_sig} (exit {r_exit})"
    );
    // Document what that behaviour is: the C really does crash here.
    assert_ne!(c_sig, 0, "the C unexpectedly survived l == 0");
}
