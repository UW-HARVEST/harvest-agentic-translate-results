//! Phase C — error-path differential tests, one per ERRORS.md row.
//!
//! Rows 1–2 run in-process. Rows 3–5 are only reachable when `calloc`/`malloc`/
//! `strlen` misbehave, so they run in a child process with the
//! `tests/fixtures/interpose.c` shim in `LD_PRELOAD`; the shim interposes those
//! symbols for BOTH libraries identically (both import them dynamically).

mod common;

use common::*;
use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::process::Command;

// ==========================================================================
// ERRORS.md row 1 — src == NULL
// ==========================================================================

#[test]
fn e01_null_pointer() {
    let cf = c_decode();
    let rf = rust_decode();
    unsafe {
        let c = cf(std::ptr::null());
        let r = rf(std::ptr::null());
        assert!(c.is_null(), "[E01] C must return NULL for a NULL src");
        assert!(r.is_null(), "[E01] Rust must return NULL for a NULL src");
    }
    // Repeat: a NULL argument must not be sticky or crash on re-entry.
    for _ in 0..1000 {
        unsafe {
            assert!(cf(std::ptr::null()).is_null());
            assert!(rf(std::ptr::null()).is_null());
        }
    }
}

// ==========================================================================
// ERRORS.md row 2 — *src == '\0' (empty string)
// ==========================================================================

#[test]
fn e02_empty_string() {
    assert_both_null("E02", b"");
    // An "empty" string that has trailing garbage after the NUL is still empty
    // as far as the C is concerned.
    let buf = b"\0ABCD\0";
    let cf = c_decode();
    let rf = rust_decode();
    unsafe {
        assert!(
            cf(buf.as_ptr() as *const c_char).is_null(),
            "[E02] C must reject a leading-NUL buffer"
        );
        assert!(
            rf(buf.as_ptr() as *const c_char).is_null(),
            "[E02] Rust must reject a leading-NUL buffer"
        );
    }
}

// ==========================================================================
// Generic FFI-boundary sweeps required by Phase C.
// ==========================================================================

#[test]
fn e_boundary_one_past_every_range() {
    // One step below and above each accepted range in is_base64 / decode.
    // '@'=0x40 ('A'-1), '['=0x5B ('Z'+1), '`'=0x60 ('a'-1), '{'=0x7B ('z'+1),
    // '/'=0x2F ('0'-1 is '/', which IS accepted -> fall-through 63),
    // ':'=0x3A ('9'+1), '*'=0x2A ('+'-1), ','=0x2C ('+'+1),
    // '<'=0x3C ('='-1), '>'=0x3E ('='+1).
    let edges: &[u8] = b"@[`{/:*,<>0-9AZaz+=.";
    for &e in edges {
        assert_same("E-bound", &[e]);
        for &b in B64 {
            assert_same("E-bound", &[e, b]);
            assert_same("E-bound", &[b, e]);
            assert_same("E-bound", &[e, b, e, b]);
            assert_same("E-bound", &[b, b, b, e]);
            assert_same("E-bound", &[b, b, e, b]);
        }
    }
    // Extremes of the byte range, incl. the sign boundary of `char`.
    for &b in &[0x01u8, 0x2f, 0x30, 0x39, 0x3a, 0x40, 0x41, 0x5a, 0x5b, 0x60, 0x61, 0x7a, 0x7b, 0x7e, 0x7f, 0x80, 0x81, 0xfe, 0xff] {
        assert_same("E-bound", &[b]);
        assert_same("E-bound", &[b, b]);
        assert_same("E-bound", &[b, b, b]);
        assert_same("E-bound", &[b, b, b, b]);
        assert_same("E-bound", &[b, b, b, b, b]);
    }
}

#[test]
fn e_boundary_zero_and_oversized_lengths() {
    // Zero length (row 2) and a range of lengths around each quartet boundary,
    // plus a deliberately oversized input.
    assert_both_null("E-len", b"");
    for n in 1..=40usize {
        let v = vec![b'A'; n];
        assert_both_ok("E-len", &v);
        let v = vec![b'='; n];
        assert_both_ok("E-len", &v);
        let v = vec![b'!'; n]; // all-ignored, any length
        assert_both_ok("E-len", &v);
    }
    // Oversized: 2 MiB of base64 plus 2 MiB of ignored bytes.
    let big = vec![b'Z'; 2 << 20];
    assert_both_ok("E-len", &big);
    let big_junk = vec![b'\n'; 2 << 20];
    assert_both_ok("E-len", &big_junk);
}

#[test]
fn e_out_of_range_enum_values_not_applicable_but_int_like_args_swept() {
    // `decode_base64` takes no enum and no flags, so there is no invalid
    // discriminant to smuggle across the boundary (documented in ERRORS.md).
    // The nearest equivalent is the full 8-bit value space of every `char` the
    // API consumes, including the values with no meaning to the decoder --
    // swept exhaustively here for 1..=3 byte inputs at every position.
    for b in 0x01u16..=0xff {
        let x = b as u8;
        assert_same("E-enum", &[x]);
        assert_same("E-enum", &[x, b'A']);
        assert_same("E-enum", &[b'A', x]);
        assert_same("E-enum", &[x, b'A', b'A']);
        assert_same("E-enum", &[b'A', x, b'A']);
        assert_same("E-enum", &[b'A', b'A', x]);
        assert_same("E-enum", &[x, x, x, x]);
        assert_same("E-enum", &[b'A', b'A', b'A', x]);
        assert_same("E-enum", &[b'A', b'A', x, b'A']);
    }
}

// ==========================================================================
// ERRORS.md rows 3, 4, 5 — driven in a child process under LD_PRELOAD.
// ==========================================================================

const CHILD_ENV: &str = "DRIVER_SHIM_CHILD";
const SHIM_ENV: &str = "DRIVER_SHIM_SO";

fn build_shim() -> PathBuf {
    let manifest = manifest_dir();
    let src = manifest.join("tests/fixtures/interpose.c");
    assert!(src.is_file(), "missing fixture {}", src.display());
    let out_dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let out = out_dir.join("libinterpose_shim.so");
    let status = Command::new("cc")
        .args(["-shared", "-fPIC", "-O1", "-o"])
        .arg(&out)
        .arg(&src)
        .status()
        .expect("failed to invoke cc to build the LD_PRELOAD shim");
    assert!(status.success(), "cc failed building {}", src.display());
    assert!(out.is_file(), "shim not produced at {}", out.display());
    out
}

/// Parent side: compile the shim, re-exec this test binary with it preloaded,
/// and require the child's assertions to pass.
#[test]
fn e03_e04_e05_allocation_failure_and_int_overflow() {
    if std::env::var(CHILD_ENV).is_ok() {
        return; // we ARE the child; the real work is in shim_child().
    }
    let shim = build_shim();
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(&exe)
        .args(["shim_child", "--exact", "--nocapture", "--test-threads=1"])
        .env("LD_PRELOAD", &shim)
        .env(CHILD_ENV, "1")
        .env(SHIM_ENV, &shim)
        .env("DRIVER_C_SO", c_lib_path())
        .env("DRIVER_RUST_SO", rust_lib_path())
        .output()
        .expect("failed to re-exec test binary as LD_PRELOAD child");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "LD_PRELOAD child failed ({:?})\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
        out.status
    );
    // Guard against the child silently not running the scenarios.
    assert!(
        stdout.contains("SHIM-CHILD-OK"),
        "child did not report completion\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
    assert!(
        stdout.contains("1 passed"),
        "child did not run shim_child\n--- stdout ---\n{stdout}"
    );
}

type ShimArm = unsafe extern "C" fn(usize, usize, usize);
type ShimVoid = unsafe extern "C" fn();
type ShimInt = unsafe extern "C" fn() -> c_int;
type ShimStrlenSet = unsafe extern "C" fn(c_int, *const c_char);

type ShimSize = unsafe extern "C" fn() -> usize;
type ShimULong = unsafe extern "C" fn() -> std::ffi::c_ulong;

struct Shim {
    _lib: Library,
    arm: ShimArm,
    disarm: ShimVoid,
    watch_seen: ShimInt,
    watch_freed: ShimInt,
    strlen_set: ShimStrlenSet,
    trace_reset: ShimVoid,
    last_calloc_total: ShimSize,
    last_malloc_size: ShimSize,
    calloc_calls: ShimULong,
    malloc_calls: ShimULong,
    free_calls: ShimULong,
}

/// Snapshot of the allocator traffic produced by exactly one library call.
#[derive(Debug, PartialEq, Eq)]
struct AllocTrace {
    calloc_total: usize,
    malloc_size: usize,
    calloc_calls: u64,
    malloc_calls: u64,
    free_calls: u64,
}

impl Shim {
    /// Reset the trace, run `f`, and report the allocator traffic it caused.
    /// Nothing else allocates between the reset and the read.
    unsafe fn trace<R>(&self, f: impl FnOnce() -> R) -> (R, AllocTrace) {
        (self.trace_reset)();
        let r = f();
        let t = AllocTrace {
            calloc_total: (self.last_calloc_total)(),
            malloc_size: (self.last_malloc_size)(),
            calloc_calls: (self.calloc_calls)() as u64,
            malloc_calls: (self.malloc_calls)() as u64,
            free_calls: (self.free_calls)() as u64,
        };
        (r, t)
    }
}

fn load_shim() -> Shim {
    let path = std::env::var(SHIM_ENV).expect("DRIVER_SHIM_SO not set in child");
    // The shim is already mapped via LD_PRELOAD; dlopen by path returns the
    // very same mapping, hence the same static state.
    let lib = unsafe { Library::new(&path) }.expect("dlopen shim");
    unsafe {
        let arm = *lib.get::<ShimArm>(b"shim_arm\0").expect("shim_arm");
        let disarm = *lib.get::<ShimVoid>(b"shim_disarm\0").expect("shim_disarm");
        let watch_seen = *lib.get::<ShimInt>(b"shim_watch_seen\0").expect("shim_watch_seen");
        let watch_freed = *lib.get::<ShimInt>(b"shim_watch_freed\0").expect("shim_watch_freed");
        let strlen_set = *lib
            .get::<ShimStrlenSet>(b"shim_strlen_set\0")
            .expect("shim_strlen_set");
        let trace_reset = *lib.get::<ShimVoid>(b"shim_trace_reset\0").expect("trace_reset");
        let last_calloc_total = *lib
            .get::<ShimSize>(b"shim_last_calloc_total\0")
            .expect("last_calloc_total");
        let last_malloc_size = *lib
            .get::<ShimSize>(b"shim_last_malloc_size\0")
            .expect("last_malloc_size");
        let calloc_calls = *lib.get::<ShimULong>(b"shim_calloc_calls\0").expect("calloc_calls");
        let malloc_calls = *lib.get::<ShimULong>(b"shim_malloc_calls\0").expect("malloc_calls");
        let free_calls = *lib.get::<ShimULong>(b"shim_free_calls\0").expect("free_calls");
        Shim {
            _lib: lib,
            arm,
            disarm,
            watch_seen,
            watch_freed,
            strlen_set,
            trace_reset,
            last_calloc_total,
            last_malloc_size,
            calloc_calls,
            malloc_calls,
            free_calls,
        }
    }
}

extern "C" {
    #[link_name = "free"]
    fn libc_free(p: *mut c_void);
}

/// Runs inside the LD_PRELOAD child. A no-op in a normal test run.
#[test]
fn shim_child() {
    if std::env::var(CHILD_ENV).is_err() {
        return;
    }
    let shim = load_shim();
    let cf = c_decode();
    let rf = rust_decode();

    // Sanity: with nothing armed both implementations still agree, which also
    // proves the shim itself is transparent.
    unsafe { (shim.disarm)() };
    assert_both_ok("shim-transparent", b"QUJDRA==");

    // Input sized so the two allocation requests have distinctive, unequal,
    // non-power-of-two byte counts that the test harness will never request.
    const L: usize = 100_003;
    let input = vec![b'Q'; L];
    let mut cinput = input.clone();
    cinput.push(0);
    let src = cinput.as_ptr() as *const c_char;
    let dest_total = L + 14; // calloc(sizeof(char), l + 13), l = strlen + 1
    let buf_total = L + 1; // malloc(l)

    // ---- ERRORS.md row 3: calloc fails -------------------------------
    unsafe {
        (shim.arm)(dest_total, 0, 0);
        let c = cf(src);
        (shim.arm)(dest_total, 0, 0);
        let r = rf(src);
        (shim.disarm)();
        assert!(
            c.is_null(),
            "[E03] C must return NULL when calloc(1, {dest_total}) fails"
        );
        assert!(
            r.is_null(),
            "[E03] Rust must return NULL when calloc(1, {dest_total}) fails"
        );
        println!("[E03] calloc-failure: C=NULL Rust=NULL  OK");
    }

    // ---- ERRORS.md row 4: malloc fails, dest must be freed -----------
    unsafe {
        (shim.arm)(0, buf_total, dest_total);
        let c = cf(src);
        let c_seen = (shim.watch_seen)();
        let c_freed = (shim.watch_freed)();

        (shim.arm)(0, buf_total, dest_total);
        let r = rf(src);
        let r_seen = (shim.watch_seen)();
        let r_freed = (shim.watch_freed)();
        (shim.disarm)();

        assert!(
            c.is_null(),
            "[E04] C must return NULL when malloc({buf_total}) fails"
        );
        assert!(
            r.is_null(),
            "[E04] Rust must return NULL when malloc({buf_total}) fails"
        );
        assert_eq!(c_seen, 1, "[E04] C never allocated dest — scenario invalid");
        assert_eq!(r_seen, 1, "[E04] Rust never allocated dest — scenario invalid");
        // The return value is NULL, so nobody outside could have freed dest:
        // this proves the `free(dest)` inside decode_base64 on both sides.
        assert_eq!(c_freed, 1, "[E04] C must free(dest) before returning NULL");
        assert_eq!(
            r_freed, 1,
            "[E04] Rust must free(dest) before returning NULL (leak vs. C)"
        );
        assert_eq!(c_seen, r_seen, "[E04] dest-allocation behaviour differs");
        assert_eq!(c_freed, r_freed, "[E04] dest-free behaviour differs");
        println!("[E04] malloc-failure: C=NULL(freed) Rust=NULL(freed)  OK");
    }

    // ---- ERRORS.md row 5: strlen(src) + 1 overflows int --------------
    // strlen reports INT_MAX, so l = INT_MAX + 1 truncates to INT_MIN and the
    // sign-extended calloc size is astronomically large => allocation fails.
    unsafe {
        let marker = b"QUJDRA==\0";
        let mp = marker.as_ptr() as *const c_char;
        (shim.disarm)();
        (shim.strlen_set)(1, mp);
        let c = cf(mp);
        let r = rf(mp);
        (shim.strlen_set)(0, std::ptr::null());
        assert!(
            c.is_null(),
            "[E05] C must return NULL when strlen+1 overflows int"
        );
        assert!(
            r.is_null(),
            "[E05] Rust must return NULL when strlen+1 overflows int"
        );
        println!("[E05] int-overflow of strlen+1: C=NULL Rust=NULL  OK");
    }

    // ---- ERRORS.md row 5b: benign int truncation ---------------------
    // strlen reports real + 2^32. `int l = strlen(src) + 1` truncates back to
    // real + 1, so both implementations must behave exactly as normal. This
    // pins down the truncation semantics without invoking UB.
    unsafe {
        let marker = b"QUJDRAAAbGlicmFyeQ==\0";
        let mp = marker.as_ptr() as *const c_char;
        let real = marker.len() - 1;
        (shim.disarm)();
        (shim.strlen_set)(2, mp);
        let c = cf(mp);
        let r = rf(mp);
        (shim.strlen_set)(0, std::ptr::null());
        assert!(!c.is_null(), "[E05b] C should succeed after truncation");
        assert!(!r.is_null(), "[E05b] Rust should succeed after truncation");
        let n = real + 14;
        let cb = std::slice::from_raw_parts(c as *const u8, n).to_vec();
        let rb = std::slice::from_raw_parts(r as *const u8, n).to_vec();
        libc_free(c as *mut c_void);
        libc_free(r as *mut c_void);
        assert_eq!(cb, rb, "[E05b] truncated-length decode differs");
        println!("[E05b] int truncation of strlen+1: byte-identical  OK");
    }

    // ---- rows 1 & 2 again, under the shim ---------------------------
    unsafe {
        (shim.disarm)();
        assert!(cf(std::ptr::null()).is_null(), "[E01/shim] C NULL");
        assert!(rf(std::ptr::null()).is_null(), "[E01/shim] Rust NULL");
        let empty = b"\0";
        assert!(
            cf(empty.as_ptr() as *const c_char).is_null(),
            "[E02/shim] C empty"
        );
        assert!(
            rf(empty.as_ptr() as *const c_char).is_null(),
            "[E02/shim] Rust empty"
        );
    }

    // ---- allocation-trace differential ------------------------------
    // The bytes returned only prove the *contents* match. Here we compare the
    // allocator traffic itself: the exact calloc/malloc byte counts and the
    // number of calloc/malloc/free calls. A wrong `l + 13` / `malloc(l)` size,
    // a missing free(buf), or a double free diverges here even when the visible
    // output happens to be identical.
    unsafe {
        (shim.disarm)();
        // Warm up both libraries so no one-time runtime init pollutes the trace.
        for warm in [b"QUJDRA==".as_ptr(), b"x".as_ptr()] {
            let p = cf(warm as *const c_char);
            if !p.is_null() {
                libc_free(p as *mut c_void);
            }
            let p = rf(warm as *const c_char);
            if !p.is_null() {
                libc_free(p as *mut c_void);
            }
        }

        let mut rng = Rng::new(0xA110C);
        let mut cases: Vec<Vec<u8>> = vec![
            b"A".to_vec(),
            b"QQ==".to_vec(),
            b"QUJDRA==".to_vec(),
            b"!!!!".to_vec(),          // l == 0 after filtering
            b"\x80\xff".to_vec(),      // negative chars only
            vec![b'Z'; 3],
            vec![b'Z'; 4],
            vec![b'Z'; 5],
            vec![b'Z'; 4096],
            vec![b'\n'; 1000],
        ];
        for _ in 0..200 {
            let n = rng.range(1, 300);
            cases.push(rng.nonnul_bytes(n));
        }

        for case in &cases {
            let mut z = case.clone();
            z.push(0);
            let p = z.as_ptr() as *const c_char;

            let (cp, ct) = shim.trace(|| cf(p));
            if !cp.is_null() {
                libc_free(cp as *mut c_void);
            }
            let (rp, rt) = shim.trace(|| rf(p));
            if !rp.is_null() {
                libc_free(rp as *mut c_void);
            }

            assert_eq!(
                ct, rt,
                "[E-trace] allocator traffic differs for {:02x?}\n  C   : {ct:?}\n  Rust: {rt:?}",
                &case[..case.len().min(24)]
            );
            // And pin the trace to what the C source literally says.
            let strlen = case.iter().position(|&b| b == 0).unwrap_or(case.len());
            assert_eq!(
                ct.calloc_total,
                strlen + 14,
                "[E-trace] calloc size must be strlen+1+13"
            );
            assert_eq!(
                ct.malloc_size,
                strlen + 1,
                "[E-trace] malloc size must be strlen+1"
            );
            assert_eq!(ct.calloc_calls, 1, "[E-trace] exactly one calloc");
            assert_eq!(ct.malloc_calls, 1, "[E-trace] exactly one malloc");
            assert_eq!(ct.free_calls, 1, "[E-trace] exactly one free (of buf)");
        }
        println!(
            "[E-trace] allocator traffic identical across {} inputs  OK",
            cases.len()
        );
    }

    // A last transparency check: the shim is disarmed, ordinary inputs agree.
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..500 {
        let n = rng.range(1, 128);
        assert_same("shim-transparent", &rng.nonnul_bytes(n));
    }

    println!("SHIM-CHILD-OK");
}
