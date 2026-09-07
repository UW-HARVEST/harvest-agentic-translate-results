//! Phase C — the `assert()`/`abort()` rows of `ERRORS.md`.
//!
//! An `assert` failure kills the process, so each case runs in a child process
//! (this same test binary re-invoked with `--exact abort_child`) once against
//! the C `.so` and once against the Rust `.so`. The child's **exit status /
//! terminating signal and its full stderr** must match byte-for-byte, which
//! also pins down the glibc `__assert_fail` diagnostic (file, line, function
//! and assertion text).

mod common;

use common::deflate::*;
use common::*;
use std::ffi::c_void;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

// --------------------------------------------------------------------------
// child side
// --------------------------------------------------------------------------

fn child_body(case: &str, which: &str) {
    let lib = match which {
        "c" => Lib::open("C", &c_so_path()),
        "rust" => Lib::open("RUST", &rust_so_path()),
        _ => panic!("bad CP_ABORT_LIB"),
    };
    let inflate = lib.cp_inflate();
    let mut out = vec![0u8; 4096];

    // helper closure: call cp_inflate with explicit in_bytes
    let mut call = |input: &[u8], align: usize, in_bytes: i32, out_bytes: i32| -> i32 {
        let mut buf = AlignedBuf::new(input, align);
        unsafe {
            inflate(
                buf.ptr() as *mut c_void,
                in_bytes,
                out.as_mut_ptr() as *mut c_void,
                out_bytes,
            )
        }
    };

    let ret = match case {
        // row 13 / 22 — empty input: assert(s->bits_left > 0)
        "in_bytes_zero" => call(&[0u8; 8], 0, 0, 4096),
        "in_bytes_zero_align3" => call(&[0u8; 8], 3, 0, 4096),
        "in_bytes_zero_null" => unsafe {
            inflate(std::ptr::null_mut(), 0, out.as_mut_ptr() as *mut c_void, 4096)
        },
        // row 23 — negative input length
        "in_bytes_neg1" => call(&[0u8; 8], 0, -1, 4096),
        "in_bytes_neg_min" => call(&[0u8; 8], 0, i32::MIN, 4096),
        // row 15 — assert(s->count >= num_bits_to_read) in cp_consume_bits:
        // a stored block header truncated to 2 bytes, so the 16-bit LEN read
        // has only 13 buffered bits and nothing left to load.
        "stored_truncated" => call(&[0x01, 0xAB, 0, 0], 0, 2, 4096),
        "stored_truncated_3" => call(&[0x01, 0xAB, 0xCD, 0], 0, 3, 4096),
        // row 14 — assert(!cp_would_overflow(...)) in cp_read_bits
        "would_overflow" => {
            let (s, in_bytes) = build_would_overflow_stream();
            call(&s, 0, in_bytes, 4096)
        }
        // row 16 — assert((search >> len) == (key >> len)) in cp_decode:
        // an all-zero code-length dynamic block yields an empty tree, so
        // cp_decode reads tree[-1].
        "empty_tree_decode" => {
            let s = build_all_zero_length_dynamic_block();
            let n = s.len() as i32;
            call(&s, 0, n, 4096)
        }
        // row 17 — assert(len < 16) in cp_build, via the exported (mutable)
        // cp_fixed_table
        "build_len_16" => {
            unsafe {
                *lib.data_ptr_u8(b"cp_fixed_table") = 16;
            }
            let s = fixed_literal_stream(b"abc");
            let n = s.len() as i32;
            call(&s, 0, n, 4096)
        }
        "build_len_255" => {
            unsafe {
                *lib.data_ptr_u8(b"cp_fixed_table").add(7) = 255;
            }
            let s = fixed_literal_stream(b"abc");
            let n = s.len() as i32;
            call(&s, 0, n, 4096)
        }
        // row 20 — assert(num_bits_to_read <= 32) in cp_read_bits, via the
        // exported (mutable) cp_dist_extra_bits
        "dist_extra_33" => {
            unsafe {
                *lib.data_ptr_u8(b"cp_dist_extra_bits") = 33;
            }
            let s = build_dist_symbol_zero_stream();
            let n = s.len() as i32;
            call(&s, 0, n, 4096)
        }
        "len_extra_64" => {
            unsafe {
                // cp_len_extra_bits[0] is the extra-bit count of length symbol
                // 257, which the stream below uses
                *lib.data_ptr_u8(b"cp_len_extra_bits") = 64;
            }
            let s = build_dist_symbol_zero_stream();
            let n = s.len() as i32;
            call(&s, 0, n, 4096)
        }
        other => panic!("unknown case {other}"),
    };
    // If we get here the library did *not* abort; report the return value so a
    // divergence is still visible.
    eprintln!("NO_ABORT ret={ret}");
}

#[test]
fn abort_child() {
    if let Ok(case) = std::env::var("CP_ABORT_CASE") {
        let which = std::env::var("CP_ABORT_LIB").expect("CP_ABORT_LIB");
        child_body(&case, &which);
        // flush and leave without letting libtest print anything else
        std::process::exit(0);
    }
}

// --------------------------------------------------------------------------
// parent side
// --------------------------------------------------------------------------

struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stderr: Vec<u8>,
}

fn spawn(case: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args(["--exact", "abort_child", "--nocapture", "--test-threads=1"])
        .env("CP_ABORT_CASE", case)
        .env("CP_ABORT_LIB", which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn child");
    Outcome {
        code: out.status.code(),
        signal: out.status.signal(),
        stderr: out.stderr,
    }
}

#[track_caller]
fn diff_abort(case: &str, expect_line: u32, expect_func: &str, expect_assertion: &str) {
    let c = spawn(case, "c");
    let r = spawn(case, "rust");
    let cs = String::from_utf8_lossy(&c.stderr).into_owned();
    let rs = String::from_utf8_lossy(&r.stderr).into_owned();

    assert_eq!(
        (c.signal, c.code),
        (r.signal, r.code),
        "[{case}] termination differs: C signal={:?} code={:?} / RUST signal={:?} code={:?}\n\
         C stderr:\n{cs}\nRUST stderr:\n{rs}",
        c.signal,
        c.code,
        r.signal,
        r.code
    );
    assert_eq!(
        c.signal,
        Some(libc_sigabrt()),
        "[{case}] expected SIGABRT from both, got signal {:?} / code {:?}\nC stderr:\n{cs}",
        c.signal,
        c.code
    );
    assert_eq!(c.stderr, r.stderr, "[{case}] stderr differs:\nC:\n{cs}\nRUST:\n{rs}");
    // and the diagnostic must be the real glibc assert message
    let needle = format!(":{expect_line}: {expect_func}: Assertion `{expect_assertion}' failed.");
    assert!(
        cs.contains(&needle),
        "[{case}] stderr does not contain {needle:?}:\n{cs}"
    );
    assert!(
        cs.contains("c_src/src/lib.c"),
        "[{case}] stderr does not name the C source file:\n{cs}"
    );
}

fn libc_sigabrt() -> i32 {
    6
}

// --------------------------------------------------------------------------
// rows
// --------------------------------------------------------------------------

#[test]
fn abort13_bits_left_zero() {
    diff_abort("in_bytes_zero", 119, "cp_read_bits", "s->bits_left > 0");
}

#[test]
fn abort13b_bits_left_zero_align3() {
    diff_abort("in_bytes_zero_align3", 119, "cp_read_bits", "s->bits_left > 0");
}

#[test]
fn abort22_bits_left_zero_null_input() {
    diff_abort("in_bytes_zero_null", 119, "cp_read_bits", "s->bits_left > 0");
}

#[test]
fn abort23_in_bytes_negative() {
    diff_abort("in_bytes_neg1", 119, "cp_read_bits", "s->bits_left > 0");
    diff_abort("in_bytes_neg_min", 119, "cp_read_bits", "s->bits_left > 0");
}

#[test]
fn abort15_consume_bits_count() {
    diff_abort("stored_truncated", 109, "cp_consume_bits", "s->count >= num_bits_to_read");
    // With 3 input bytes the alignment discard + LEN/NLEN reads exhaust the
    // stream first, so the *bits_left* assert wins instead — still a real,
    // deterministic rejection that both libraries must reproduce identically.
    diff_abort("stored_truncated_3", 119, "cp_read_bits", "s->bits_left > 0");
}

#[test]
fn abort14_would_overflow() {
    diff_abort(
        "would_overflow",
        121,
        "cp_read_bits",
        "!cp_would_overflow(s, num_bits_to_read)",
    );
}

#[test]
fn abort16_decode_mismatch() {
    diff_abort("empty_tree_decode", 211, "cp_decode", "(search >> len) == (key >> len)");
}

#[test]
fn abort17_build_len_ge_16() {
    diff_abort("build_len_16", 148, "cp_build", "len < 16");
    diff_abort("build_len_255", 148, "cp_build", "len < 16");
}

#[test]
fn abort20_read_bits_too_many() {
    diff_abort("dist_extra_33", 117, "cp_read_bits", "num_bits_to_read <= 32");
    diff_abort("len_extra_64", 117, "cp_read_bits", "num_bits_to_read <= 32");
}
