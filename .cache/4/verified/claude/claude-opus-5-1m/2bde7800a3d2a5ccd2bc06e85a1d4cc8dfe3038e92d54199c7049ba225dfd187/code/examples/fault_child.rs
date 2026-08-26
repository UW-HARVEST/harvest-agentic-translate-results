//! Helper process for the differential rows that need a *modified process
//! environment*, which cannot be arranged from inside a running test:
//!
//! * ERRORS.md rows 1, 2, 9, 11 — reachable only when `malloc()` returns NULL,
//!   so the process is started with `LD_PRELOAD=fail_malloc_preload.so` (see
//!   `tests/fixtures/fail_malloc.c`) and one exact allocation size is made to
//!   fail *after* both shared objects are loaded.
//! * CONFIGS.md row 44 — a heap pre-filled with a non-zero pattern
//!   (`MALLOC_PERTURB_`), which is what makes the *untouched tail* of
//!   `create_result_string`'s 64-byte block and of `Result.operation[32]`
//!   observable.  On a fresh, zero-filled heap a missing NUL terminator or a
//!   short `strcpy` is invisible.
//!
//! It loads the C `.so` and the Rust `.so` with `libloading` — exactly like the
//! in-process differential tests — runs the requested scenario against each,
//! and writes a delimited report on stdout that the parent test compares
//! section-by-section.
//!
//! usage: fault_child <c.so> <rust.so> <scenario> <fail_size>

use libloading::os::unix::Library as UnixLibrary;
use libloading::Library;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

type FnCreateResultString = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_char;
type FnCheckPermissions = unsafe extern "C" fn(c_int, c_int) -> c_int;
type FnSafeAdd = unsafe extern "C" fn(c_int, c_int, c_int) -> c_int;
type FnMultiplyWithLog = unsafe extern "C" fn(c_int, c_int, *mut *mut c_char) -> c_int;
type FnCopyAndSum = unsafe extern "C" fn(*mut c_int, c_int) -> c_int;
type FnCompareOperations = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type FnComplexMode = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type FnArm = unsafe extern "C" fn(u64);
type FnFill = unsafe extern "C" fn(u64);
type FnLogStart = unsafe extern "C" fn();
type FnLogStop = unsafe extern "C" fn();
type FnLogCount = unsafe extern "C" fn() -> c_int;
type FnLogGet = unsafe extern "C" fn(c_int) -> u64;

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn fflush(f: *mut c_void) -> c_int;
    fn free(p: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

/// Print through the C runtime so our markers share the buffer (and therefore
/// the ordering) of the printf() calls made inside the libraries.
fn out(s: &str) {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    unsafe {
        printf(b"%s\0".as_ptr() as *const c_char, v.as_ptr() as *const c_char);
        fflush(std::ptr::null_mut());
    }
}

unsafe fn read_cstr(p: *const c_char) -> String {
    if p.is_null() {
        return "<NULL>".to_string();
    }
    let n = strlen(p);
    let b = std::slice::from_raw_parts(p as *const u8, n);
    String::from_utf8_lossy(b).escape_debug().to_string()
}

/// Hex dump of the *whole* 64-byte block handed back by
/// `create_result_string`, including every byte past the NUL terminator.  With
/// `MALLOC_PERTURB_` set these bytes are a deterministic non-zero pattern, so a
/// short write or a missing terminator shows up as a differing dump.
unsafe fn hex_block(p: *const u8, len: usize) -> String {
    if p.is_null() {
        return "<NULL>".to_string();
    }
    let b = std::slice::from_raw_parts(p, len);
    let mut s = String::with_capacity(len * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

const EDGE: [c_int; 7] = [0, 1, -1, 42, i32::MAX, i32::MIN, 65536];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!("usage: fault_child <c.so> <rust.so> <scenario> <fail_size>");
        std::process::exit(2);
    }
    let scenario = args[3].clone();
    let fail_size: u64 = args[4].parse().expect("fail_size");

    // Make sure stdout's FILE buffer (and everything else lazily allocated) is
    // in place *before* the interposer is armed.
    out("PRIME\n");

    let libs: Vec<(&str, Library)> = vec![
        ("C", unsafe { Library::new(&args[1]) }.expect("dlopen C")),
        ("RUST", unsafe { Library::new(&args[2]) }.expect("dlopen Rust")),
    ];

    // Resolve the interposer's arming hook from the global scope (LD_PRELOAD
    // objects live there).  Only required when a failure size was requested.
    let this = UnixLibrary::this();
    let arm: Box<dyn Fn(u64)> = unsafe {
        match this.get::<FnArm>(b"fail_malloc_arm\0") {
            Ok(s) => {
                let f = *s;
                Box::new(move |n| f(n))
            }
            Err(e) => {
                if fail_size != 0 {
                    eprintln!("fail_malloc_arm not found (LD_PRELOAD missing?): {e}");
                    std::process::exit(3);
                }
                Box::new(|_| {})
            }
        }
    };

    // CONFIGS.md row 44: pre-fill every freshly malloc'ed block with a non-zero
    // byte so the *untouched tails* of the library's partially written buffers
    // are deterministic and part of the comparison.
    let fill_byte: u64 = std::env::var("CDIFF_FILL_BYTE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let fill: Box<dyn Fn(u64)> = unsafe {
        match this.get::<FnFill>(b"fail_malloc_fill\0") {
            Ok(s) => {
                let f = *s;
                Box::new(move |n| f(n))
            }
            Err(e) => {
                if fill_byte != 0 {
                    eprintln!("fail_malloc_fill not found (LD_PRELOAD missing?): {e}");
                    std::process::exit(3);
                }
                Box::new(|_| {})
            }
        }
    };

    for (tag, lib) in &libs {
        out(&format!("===={tag}====\n"));
        fill(fill_byte);
        unsafe {
            match scenario.as_str() {
                // ERRORS.md row 1 — create_result_string, malloc(64) fails
                "crs" => {
                    let f: FnCreateResultString = *lib.get(b"create_result_string\0").unwrap();
                    let op = b"multiply\0";
                    arm(fail_size);
                    let p = f(op.as_ptr() as *const c_char, 42);
                    arm(0);
                    out(&format!("RET_PTR={}\n", read_cstr(p)));
                    if !p.is_null() {
                        free(p as *mut c_void);
                    }
                }
                // ERRORS.md row 2 — multiply_with_log, inner malloc(64) fails
                "mwl" => {
                    let f: FnMultiplyWithLog = *lib.get(b"multiply_with_log\0").unwrap();
                    let mut log: *mut c_char = 0x1 as *mut c_char;
                    arm(fail_size);
                    let r = f(6, 7, &mut log);
                    arm(0);
                    out(&format!("RET={r} LOG={}\n", read_cstr(log)));
                    if !log.is_null() && log as usize != 1 {
                        free(log as *mut c_void);
                    }
                }
                // ERRORS.md row 5 — copy_and_sum, malloc(count*4) fails
                "cas" => {
                    let f: FnCopyAndSum = *lib.get(b"copy_and_sum\0").unwrap();
                    let mut buf: [c_int; 3] = [10, 20, 30];
                    arm(fail_size);
                    let r = f(buf.as_mut_ptr(), 3);
                    arm(0);
                    out(&format!("RET={r}\n"));
                }
                // ERRORS.md rows 9 & 11 — complexmode with the tracker malloc
                // (40 bytes) or the log-string malloc (64 bytes) failing
                "cm1" | "cm2" | "cm3" | "cm4" | "cm9" => {
                    let f: FnComplexMode = *lib.get(b"complexmode\0").unwrap();
                    let mode: c_int = match scenario.as_str() {
                        "cm1" => 1,
                        "cm2" => 2,
                        "cm3" => 3,
                        "cm4" => 4,
                        _ => 9,
                    };
                    arm(fail_size);
                    let r = f(mode, 6, 7, 8);
                    arm(0);
                    out(&format!("RET={r}\n"));
                }
                // CONFIGS.md row 44 — full-surface sweep, meant to be run with
                // MALLOC_PERTURB_ so that every uninitialised heap byte the
                // library leaves behind is a deterministic non-zero pattern.
                "sweep" => {
                    let crs: FnCreateResultString =
                        *lib.get(b"create_result_string\0").unwrap();
                    let chk: FnCheckPermissions = *lib.get(b"check_permissions\0").unwrap();
                    let add: FnSafeAdd = *lib.get(b"safe_add\0").unwrap();
                    let mwl: FnMultiplyWithLog = *lib.get(b"multiply_with_log\0").unwrap();
                    let cas: FnCopyAndSum = *lib.get(b"copy_and_sum\0").unwrap();
                    let cmp: FnCompareOperations = *lib.get(b"compare_operations\0").unwrap();
                    let cm: FnComplexMode = *lib.get(b"complexmode\0").unwrap();

                    // create_result_string: dump all 64 bytes, so the untouched
                    // tail after the NUL is part of the comparison.
                    for len in [0usize, 1, 5, 8, 20, 30, 40, 50, 52, 53, 54, 60, 70] {
                        let mut op = vec![b'x'; len];
                        op.push(0);
                        for val in EDGE {
                            let p = crs(op.as_ptr() as *const c_char, val);
                            out(&format!(
                                "CRS len={len} val={val}: {}\n",
                                hex_block(p as *const u8, 64)
                            ));
                            if !p.is_null() {
                                free(p as *mut c_void);
                            }
                        }
                    }
                    let p = crs(std::ptr::null(), 7);
                    out(&format!("CRS NULL: {}\n", hex_block(p as *const u8, 64)));
                    if !p.is_null() {
                        free(p as *mut c_void);
                    }

                    // multiply_with_log: same full-block dump of the out-param.
                    for (a, b) in [(6, 7), (0, 0), (i32::MAX, i32::MAX), (i32::MIN, -1)] {
                        let mut log: *mut c_char = std::ptr::null_mut();
                        let r = mwl(a, b, &mut log);
                        out(&format!(
                            "MWL {a} {b}: RET={r} {}\n",
                            hex_block(log as *const u8, 64)
                        ));
                        if !log.is_null() {
                            free(log as *mut c_void);
                        }
                    }

                    // complexmode: the printed "Operation performed: %s" line
                    // walks Result.operation[32], which is only partially
                    // written — a short strcpy shows up here under perturbing.
                    for mode in -1..=6 {
                        for (v1, v2, v3) in [(6, 7, 8), (0, 0, 0), (i32::MAX, 2, i32::MIN)] {
                            let r = cm(mode, v1, v2, v3);
                            out(&format!("CM {mode} {v1} {v2} {v3}: RET={r}\n"));
                        }
                    }

                    // the remaining entry points, for completeness
                    for perms in [0, 0o100, 0o400, 0o600, 0o644, -1] {
                        for req in [0, 0o100, 0o600, 0o644, -1] {
                            out(&format!("CHK {perms} {req}: {}\n", chk(perms, req)));
                        }
                        out(&format!("ADD {perms}: {}\n", add(7, -9, perms)));
                    }
                    for count in [0, 1, 3, 17, -1] {
                        let mut buf: Vec<c_int> = (0..32).map(|i| i * 7 - 11).collect();
                        out(&format!("CAS {count}: {}\n", cas(buf.as_mut_ptr(), count)));
                    }
                    for (a, b) in [
                        (&b"none\0"[..], &b"none\0"[..]),
                        (&b"none\0"[..], &b"nonf\0"[..]),
                        (&b"\0"[..], &b"none\0"[..]),
                    ] {
                        out(&format!(
                            "CMP: {}\n",
                            cmp(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char)
                        ));
                    }
                }
                // CONFIGS.md row 45 — the exact sequence of malloc() request
                // sizes each library issues.  Catches size-computation
                // divergences that produce the same *outcome* (e.g. a
                // zero-extended instead of sign-extended `count * sizeof(int)`:
                // both requests fail, but they ask for different amounts).
                "sizes" => {
                    let crs: FnCreateResultString =
                        *lib.get(b"create_result_string\0").unwrap();
                    let mwl: FnMultiplyWithLog = *lib.get(b"multiply_with_log\0").unwrap();
                    let cas: FnCopyAndSum = *lib.get(b"copy_and_sum\0").unwrap();
                    let cm: FnComplexMode = *lib.get(b"complexmode\0").unwrap();

                    let start: FnLogStart = *this
                        .get(b"fail_malloc_log_start\0")
                        .expect("LD_PRELOAD missing");
                    let stop: FnLogStop = *this.get(b"fail_malloc_log_stop\0").unwrap();
                    let count: FnLogCount = *this.get(b"fail_malloc_log_count\0").unwrap();
                    let get: FnLogGet = *this.get(b"fail_malloc_log_get\0").unwrap();

                    // NOTE: nothing may allocate between start() and stop(),
                    // so the report is built only after stop().
                    let dump = |label: &str| {
                        let n = count();
                        let mut s = String::new();
                        for i in 0..n {
                            if i > 0 {
                                s.push(',');
                            }
                            s.push_str(&format!("{}", get(i)));
                        }
                        out(&format!("SIZES {label}: [{s}]\n"));
                    };

                    for len in [0usize, 8, 40, 70] {
                        let mut op = vec![b'x'; len];
                        op.push(0);
                        start();
                        let p = crs(op.as_ptr() as *const c_char, -12345);
                        stop();
                        dump(&format!("crs len={len}"));
                        if !p.is_null() {
                            free(p as *mut c_void);
                        }
                    }

                    for (a, b) in [(6, 7), (i32::MIN, -1)] {
                        let mut log: *mut c_char = std::ptr::null_mut();
                        start();
                        let _ = mwl(a, b, &mut log);
                        stop();
                        dump(&format!("mwl {a} {b}"));
                        if !log.is_null() {
                            free(log as *mut c_void);
                        }
                    }

                    // Negative / boundary counts: malloc fails for all of them,
                    // so only the logged request size distinguishes a wrong
                    // int -> size_t conversion.
                    let mut buf: Vec<c_int> = (0..64).map(|i| i - 7).collect();
                    for c in [
                        0,
                        1,
                        3,
                        17,
                        64,
                        -1,
                        -2,
                        -3,
                        -17,
                        -1024,
                        -65536,
                        -(1 << 30),
                        i32::MIN,
                        i32::MIN + 1,
                    ] {
                        start();
                        let _ = cas(buf.as_mut_ptr(), c);
                        stop();
                        dump(&format!("cas {c}"));
                    }

                    for mode in [-1, 0, 1, 2, 3, 4, 5] {
                        start();
                        let _ = cm(mode, 6, 7, 8);
                        stop();
                        dump(&format!("cm {mode}"));
                    }
                }
                other => {
                    eprintln!("unknown scenario {other}");
                    std::process::exit(4);
                }
            }
        }
    }
    out("====END====\n");
}
