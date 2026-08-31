//! The library must write through the process-wide libc `stdout` buffer, so its
//! output interleaves with a caller's own stdio writes identically to the C
//! version. A Rust-side `std::io::stdout` would buffer separately and reorder.

mod common;

use common::{capture_stdout, libs, show, Impl};
use libloading::Symbol;
use std::ffi::{c_char, c_int, c_void, CString};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fwrite(ptr: *const c_void, size: usize, n: usize, stream: *mut c_void) -> usize;
    /// glibc exposes `stdout` as a `FILE *` global.
    static stdout: *mut c_void;
}

type DriverFn = unsafe extern "C" fn(*const c_char);

/// Caller writes before, between and after two `driver` calls, without flushing.
fn interleaved(which: Impl, input: &str) -> Vec<u8> {
    let l = libs();
    let lib = match which {
        Impl::C => &l.c,
        Impl::Rust => &l.rust,
    };
    let sym: Symbol<DriverFn> = unsafe { lib.get(b"driver\0").expect("driver symbol") };
    let arg = CString::new(input).unwrap();
    capture_stdout(|| unsafe {
        printf(b"<before>\n\0".as_ptr() as *const c_char);
        printf(b"<partial line no newline> \0".as_ptr() as *const c_char);
        sym(arg.as_ptr());
        printf(b"<between>\n\0".as_ptr() as *const c_char);
        sym(arg.as_ptr());
        let tail = b"<after, via fwrite>\n";
        fwrite(tail.as_ptr() as *const c_void, 1, tail.len(), stdout);
        printf(b"<end>\n\0".as_ptr() as *const c_char);
    })
}

#[test]
fn output_interleaves_with_caller_stdio_identically() {
    for input in ["4", "-2", "not-a-number", "2147483648", "0"] {
        let c = interleaved(Impl::C, input);
        let r = interleaved(Impl::Rust, input);
        if c != r {
            panic!(
                "interleaved output for driver({input:?}) differs\n--- C ---\n{}\n--- Rust ---\n{}",
                show(&c),
                show(&r)
            );
        }
        // The library's first write must continue the caller's unterminated
        // line, which only happens if both share one stdio buffer.
        let expected_continuation = if input.parse::<i32>().is_ok() {
            "<partial line no newline> The house has"
        } else {
            "<partial line no newline> An error occurred"
        };
        assert!(
            show(&c).contains(expected_continuation),
            "expected {expected_continuation:?} in the output, got:\n{}",
            show(&c)
        );
    }
}
