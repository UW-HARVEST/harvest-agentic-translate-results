//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Rows whose expected C result is a *process death* (`exit(30)`, `SIGSEGV`) are
//! run in a re-executed child process so the exact exit code / signal can be
//! compared between the two libraries.

mod common;

use common::*;
use std::os::raw::{c_char, c_int};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const SEED: u64 = 0x5EED_1234_ABCD_0002;

// =========================================================================
// Row 1 — extractFilename: separator absent -> returns `path` unchanged
// =========================================================================
#[test]
fn err_01_extract_separator_absent() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 1);
    let no_sep = b"abcdefgHIJKLM0123._-";
    for _ in 0..3000 {
        let path = rand_cstr(&mut rng, 0, 64, no_sep);
        let off = diff_extract(&c, &r, &path, b'/');
        assert_eq!(off, 0, "must return the *same* pointer, not a copy/offset");
    }
    // and for a separator that simply never occurs in a full-range buffer
    for _ in 0..2000 {
        let path = rand_cstr(&mut rng, 1, 40, b"aB1");
        for sep in [b'/', b'\\', b'z', 0xFF, 0x80] {
            let off = diff_extract(&c, &r, &path, sep);
            assert_eq!(off, 0);
        }
    }
}

// =========================================================================
// Row 2 — extractFilename: empty path
// =========================================================================
#[test]
fn err_02_extract_empty_path() {
    let (c, r) = both();
    let empty = b"\0";
    for sep in 1u16..=255 {
        let off = diff_extract(&c, &r, empty, sep as u8);
        assert_eq!(off, 0, "empty path + absent separator must return path, not path+1");
    }
}

// =========================================================================
// Row 3 — extractFilename: separator == 0 (the NUL byte)
//
// Per C, the terminator is part of the string, so strrchr SUCCEEDS at index
// strlen(path) and the function returns path + strlen(path) + 1.
// =========================================================================
#[test]
fn err_03_extract_nul_separator() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..2000 {
        let len = rng.range(0, 64);
        let path = rand_cstring(&mut rng, len, b"abc/XYZ.0");
        let off = diff_extract(&c, &r, &path, 0);
        assert_eq!(
            off as usize,
            cstr_len(&path) + 1,
            "strrchr(s,0) must match the terminator, so the result is one PAST it"
        );
    }
    // the empty string, explicitly
    let off = diff_extract(&c, &r, b"\0", 0);
    assert_eq!(off, 1);
}

// =========================================================================
// Row 4 — extractFilename: separator is the last byte -> empty tail
// =========================================================================
#[test]
fn err_04_extract_trailing_separator() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..2000 {
        let sep = rng.nonzero_byte();
        let len = rng.range(0, 48);
        let mut path: Vec<u8> = (0..len).map(|_| rng.byte_from(b"abcXYZ")).collect();
        path.push(sep);
        path.push(0);
        let off = diff_extract(&c, &r, &path, sep);
        assert_eq!(off as usize, len + 1);
        // the returned tail must be the empty string in both
        assert_eq!(path[off as usize], 0);
    }
}

// =========================================================================
// Row 5 — extractFilename: negative / sign-extended separator bytes
// =========================================================================
#[test]
fn err_05_extract_negative_separator() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 5);
    for sep in 0x80u16..=0xFF {
        for _ in 0..40 {
            let len = rng.range(0, 40);
            let path = rand_cstring_full(&mut rng, len);
            diff_extract(&c, &r, &path, sep as u8);
        }
        // guaranteed hit: the separator byte planted in the buffer
        let mut path = rand_cstring_full(&mut rng, 24);
        let n = cstr_len(&path);
        if n > 2 {
            path[n / 2] = sep as u8;
            let off = diff_extract(&c, &r, &path, sep as u8);
            assert!(off > 0, "planted high separator byte must be found");
        }
    }
}

// =========================================================================
// Row 6 — FIO_createFilename_fromOutDir: empty outDirName triggers the
//         out-of-bounds `outDirName[strlen("")-1]` == `outDirName[-1]` read.
//
// Both libraries are handed the SAME pointer, so they must read the SAME byte
// and therefore take the SAME branch. The byte before the string is placed
// deliberately so BOTH branches of lib.c:45 are exercised.
// =========================================================================
#[test]
fn err_06_empty_outdir_oob_read() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 6);

    // preceding_byte == '/'  -> lib.c:45 true branch (concatenate)
    // preceding_byte != '/'  -> lib.c:45 false branch (insert separator)
    for preceding in [b'/', b'X', 0u8, 0xFF, b'\\'] {
        for _ in 0..400 {
            // buffer layout: [ preceding | 0 ]   out_dir points at the 0
            let buf = vec![preceding, 0u8];
            let out_dir = unsafe { buf.as_ptr().add(1) } as *const c_char;

            let path = rand_cstr(&mut rng, 0, 32, b"/abcXY.0");
            let suffix = rng.range(0, 32);
            // dir_len == 0, so alloc = 0 + 1 + name_len + suffix + 1
            let n = 1 + filename_tail_len(&path) + suffix + 1;

            diff_create_raw(
                &c,
                &r,
                path.as_ptr() as *const c_char,
                out_dir,
                suffix,
                n,
                &format!("empty outDir, preceding byte 0x{preceding:02x}"),
            );
            std::hint::black_box(&buf);
        }
    }
}

// =========================================================================
// Row 7 — FIO_createFilename_fromOutDir: size_t wrap-around on suffixLen
//
// suffixLen == SIZE_MAX makes
//   dirLen + 1 + nameLen + SIZE_MAX + 1  ==  dirLen + nameLen + 1  (mod 2^64)
// which is EXACTLY the number of bytes written, so calloc succeeds, the write
// fits exactly, and the buffer is left WITHOUT a NUL terminator. No error.
// =========================================================================
#[test]
fn err_07_suffixlen_size_t_overflow() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..500 {
        // axis-E false branch (dir does not end in '/')
        let dir = rand_cstr(&mut rng, 1, 24, b"abcXYZ");
        // axis-E true branch (dir ends in '/')
        let mut dir2 = dir.clone();
        let n2 = cstr_len(&dir2);
        dir2[n2 - 1] = b'/';

        let path = rand_cstr(&mut rng, 0, 32, b"/abcXY.0");

        for d in [&dir, &dir2] {
            let expect = cstr_len(d)
                .wrapping_add(1)
                .wrapping_add(filename_tail_len(&path))
                .wrapping_add(usize::MAX)
                .wrapping_add(1);
            assert_eq!(
                expect,
                cstr_len(d) + filename_tail_len(&path) + 1,
                "test bug: wrap arithmetic"
            );
            diff_create_n(&c, &r, &path, d, usize::MAX, expect);
        }
    }
}

// =========================================================================
// Row 12 — FIO_createFilename_fromOutDir: suffixLen == 0 (minimum)
// =========================================================================
#[test]
fn err_12_zero_suffixlen() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..3000 {
        let mut dir = rand_cstr(&mut rng, 1, 32, b"abc/XYZ");
        if rng.bool() {
            let n = cstr_len(&dir);
            dir[n - 1] = b'/';
        }
        let path = rand_cstr(&mut rng, 0, 32, b"/abcXY.0");
        diff_create(&c, &r, &path, &dir, 0);
    }
}

// =========================================================================
// Row 11 — extractFilename: out-of-range value for the narrow `char`
//          parameter, passed across FFI as a full `int`.
//
// A C `char` parameter accepts any `int` in the argument register; only the low
// 8 bits are significant. This is the narrow-parameter analogue of passing an
// out-of-range enum value, and both libraries must truncate identically.
// =========================================================================
#[test]
fn err_11_out_of_range_separator_int() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 11);

    let wide: &[c_int] = &[
        0x100, 0x101, 0x12F, 0x17F, 0x180, 0x1FF, 0x2F, 0xFF, 0x7F, 0x80, 0xFFFF, 0x1_0000,
        0x1_002F, 0x7FFF_FFFF, -1, -47, -300, -256, -257, -0x1_0000, i32::MIN, 0x100 | 0x2F,
    ];

    for _ in 0..400 {
        let path = rand_cstr(&mut rng, 0, 40, b"/abcXY.0\\");
        for &w in wide {
            // C and Rust must agree with each other ...
            let got = diff_extract_int(&c, &r, &path, w);
            // ... and must agree with the low-byte-only call
            let low = (w as u32 & 0xFF) as u8;
            let want = diff_extract(&c, &r, &path, low);
            assert_eq!(
                got, want,
                "wide separator {w:#x} must behave exactly like its low byte {low:#04x}"
            );
        }
    }

    // full-byte-range buffers too
    for _ in 0..400 {
        let path = rand_cstr_full(&mut rng, 0, 32);
        for &w in wide {
            let got = diff_extract_int(&c, &r, &path, w);
            let want = diff_extract(&c, &r, &path, (w as u32 & 0xFF) as u8);
            assert_eq!(got, want, "wide separator {w:#x} vs low byte");
        }
    }
}

// =========================================================================
// Fatal rows (8, 9, 10) — run in a re-executed child process
// =========================================================================

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    zstd_msg: Option<String>,
}

/// Re-execute this very test binary, running only `child_worker`, with the
/// requested library + operation. Returns how the child died.
fn run_child(lib: &str, op: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(&exe)
        .args(["--exact", "child_worker", "--test-threads=1", "--nocapture"])
        .env("DIFF_CHILD_LIB", lib)
        .env("DIFF_CHILD_OP", op)
        .output()
        .expect("spawn child");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    // isolate the library's own diagnostic from harness chatter
    let zstd_msg = stderr.find("zstd: ").map(|i| {
        stderr[i..]
            .lines()
            .next()
            .unwrap_or("")
            .trim_end()
            .to_string()
    });
    Outcome { code: out.status.code(), signal: out.status.signal(), zstd_msg }
}

/// The child side. A no-op unless `DIFF_CHILD_OP` is set, so it is harmless
/// during a normal full-suite run.
#[test]
fn child_worker() {
    let Ok(op) = std::env::var("DIFF_CHILD_OP") else { return };
    let lib = std::env::var("DIFF_CHILD_LIB").expect("DIFF_CHILD_LIB");
    let api = match lib.as_str() {
        "c" => c_api(),
        "rust" => rust_api(),
        other => panic!("bad DIFF_CHILD_LIB={other}"),
    };

    let path = b"file.txt\0";
    let dir = b"outdir\0";

    // SAFETY: these calls deliberately reproduce the C library's unchecked
    // behaviour; the process is expected to die or exit(30).
    unsafe {
        match op.as_str() {
            "extract_null" => {
                let p = (api.extract)(std::ptr::null(), b'/' as i8);
                std::hint::black_box(p);
            }
            "create_null_path" => {
                let p = (api.create)(std::ptr::null(), dir.as_ptr() as *const c_char, 0);
                std::hint::black_box(p);
            }
            "create_null_outdir" => {
                let p = (api.create)(path.as_ptr() as *const c_char, std::ptr::null(), 0);
                std::hint::black_box(p);
            }
            "create_null_both" => {
                let p = (api.create)(std::ptr::null(), std::ptr::null(), 0);
                std::hint::black_box(p);
            }
            "alloc_fail_half" => {
                let p = (api.create)(
                    path.as_ptr() as *const c_char,
                    dir.as_ptr() as *const c_char,
                    usize::MAX / 2,
                );
                std::hint::black_box(p);
            }
            "alloc_fail_half_plus1" => {
                let p = (api.create)(
                    path.as_ptr() as *const c_char,
                    dir.as_ptr() as *const c_char,
                    usize::MAX / 2 + 1,
                );
                std::hint::black_box(p);
            }
            "alloc_fail_wrap_underflow" => {
                // wraps to a huge size_t -> calloc fails -> exit(30)
                let p = (api.create)(
                    path.as_ptr() as *const c_char,
                    dir.as_ptr() as *const c_char,
                    usize::MAX - 100,
                );
                std::hint::black_box(p);
            }
            other => panic!("bad DIFF_CHILD_OP={other}"),
        }
    }
    // If we get here the call unexpectedly returned. Use a distinctive code.
    std::process::exit(77);
}

/// Row 8 — allocation failure: `fprintf(stderr, ...)` then `exit(30)`.
#[test]
fn err_08_alloc_failure_exit_30() {
    for op in ["alloc_fail_half", "alloc_fail_half_plus1", "alloc_fail_wrap_underflow"] {
        let c = run_child("c", op);
        let r = run_child("rust", op);
        assert_eq!(
            c.code,
            Some(30),
            "[{op}] C must exit(30) on allocation failure, got {c:?}"
        );
        assert_eq!(c.code, r.code, "[{op}] exit code mismatch: C={c:?} Rust={r:?}");
        assert_eq!(c.signal, r.signal, "[{op}] signal mismatch: C={c:?} Rust={r:?}");
        assert!(
            c.zstd_msg.is_some(),
            "[{op}] C must print the zstd diagnostic, got {c:?}"
        );
        assert_eq!(
            c.zstd_msg, r.zstd_msg,
            "[{op}] stderr diagnostic mismatch:\n  C   ={:?}\n  Rust={:?}",
            c.zstd_msg, r.zstd_msg
        );
    }
}

/// Row 9 — `extractFilename(NULL, sep)`: no null check exists in C.
#[test]
fn err_09_null_path_extract() {
    let c = run_child("c", "extract_null");
    let r = run_child("rust", "extract_null");
    assert!(
        c.signal.is_some(),
        "C must die from a fatal signal on NULL path, got {c:?}"
    );
    assert_eq!(c.signal, r.signal, "signal mismatch: C={c:?} Rust={r:?}");
    assert_eq!(c.code, r.code, "exit code mismatch: C={c:?} Rust={r:?}");
    assert_ne!(c.code, Some(77), "the call must not return normally");
}

/// Row 10 — `FIO_createFilename_fromOutDir` with NULL pointers.
#[test]
fn err_10_null_args_create() {
    for op in ["create_null_path", "create_null_outdir", "create_null_both"] {
        let c = run_child("c", op);
        let r = run_child("rust", op);
        assert!(
            c.signal.is_some(),
            "[{op}] C must die from a fatal signal, got {c:?}"
        );
        assert_eq!(c.signal, r.signal, "[{op}] signal mismatch: C={c:?} Rust={r:?}");
        assert_eq!(c.code, r.code, "[{op}] exit code mismatch: C={c:?} Rust={r:?}");
        assert_ne!(c.code, Some(77), "[{op}] the call must not return normally");
    }
}
