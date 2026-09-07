//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! `tool_basename` has no error return, so each row is one of the implicit
//! rejection / boundary conditions the C reaches by *not* checking. Each test
//! asserts the two libraries agree on the exact outcome (same offset, or the
//! same fatal signal), not merely that "both failed".

mod common;

use common::*;

// ------------------------------------------------------------------ ERRORS #1
// path == NULL: the C code hands it straight to strrchr() with no null check.
// Asserted out-of-process so the fault does not take the test runner with it,
// and compared by exact wait-status (i.e. the same signal).

const CRASH_ENV: &str = "DRIVER_DIFFTEST_CRASH_TARGET";

#[test]
fn err_01_null_pointer_faults_identically() {
    // Child mode: actually perform the NULL call and die.
    if let Ok(which) = std::env::var(CRASH_ENV) {
        let path = match which.as_str() {
            "c" => c_so_path(),
            "rust" => rust_so_path(),
            other => panic!("bad {CRASH_ENV}={other}"),
        };
        let (_lib, f) = load_one(&path);
        let r = unsafe { f(std::ptr::null_mut()) };
        // Reaching here means no fault occurred; report it distinguishably.
        println!("NO_FAULT ret={r:?}");
        std::process::exit(41);
    }

    let exe = std::env::current_exe().expect("current_exe");
    let run = |which: &str| -> (Option<i32>, Option<i32>) {
        let out = std::process::Command::new(&exe)
            .args([
                "--exact",
                "err_01_null_pointer_faults_identically",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CRASH_ENV, which)
            .env("DRIVER_C_SO", c_so_path())
            .env("DRIVER_RUST_SO", rust_so_path())
            .output()
            .expect("spawn child");
        #[cfg(unix)]
        let sig = {
            use std::os::unix::process::ExitStatusExt;
            out.status.signal()
        };
        #[cfg(not(unix))]
        let sig = None;
        (out.status.code(), sig)
    };

    let c = run("c");
    let r = run("rust");
    assert_eq!(
        c, r,
        "NULL-pointer behaviour differs: C (code, signal) = {c:?}, Rust = {r:?}"
    );
    // And it must genuinely be a fatal memory fault in both, not a quiet return.
    #[cfg(unix)]
    assert_eq!(
        c.1,
        Some(libc_sigsegv()),
        "expected SIGSEGV from a NULL dereference, got {c:?}"
    );
}

#[cfg(unix)]
fn libc_sigsegv() -> i32 {
    11 // SIGSEGV on Linux
}

// ------------------------------------------------------------------ ERRORS #2
#[test]
fn err_02_empty_string() {
    let r = compare(b"", "err2 empty string");
    assert_eq!(r.offset, 0, "empty string must return `path` unchanged");
    assert!(r.tail.is_empty());
}

// --------------------------------------------------------------- ERRORS #3,#4
#[test]
fn err_03_separator_only_returns_empty_tail() {
    for s in [&b"/"[..], &b"\\"[..]] {
        let r = compare(s, "err3/4 separator-only");
        assert_eq!(
            r.offset, 1,
            "single separator must return path+1 (the NUL byte)"
        );
        assert!(r.tail.is_empty(), "basename must be the empty string");
    }
}

// ------------------------------------------------------------------ ERRORS #5
#[test]
fn err_04_trailing_separator() {
    for s in [
        &b"a/b/"[..],
        &b"a\\b\\"[..],
        &b"a/b\\"[..],
        &b"a\\b/"[..],
        &b"dir/"[..],
        &b"//"[..],
    ] {
        let r = compare(s, "err5 trailing separator");
        assert_eq!(r.offset as usize, s.len(), "must point at the NUL");
        assert!(r.tail.is_empty());
    }
}

// ------------------------------------------------------------------ ERRORS #6
#[test]
fn err_05_both_separators_ordering() {
    // s1 > s2 -> s1 + 1
    let r = compare(b"a\\b/c", "err6 s1>s2");
    assert_eq!(r.offset, 4);
    assert_eq!(r.tail, b"c");
    // s2 > s1 -> s2 + 1
    let r = compare(b"a/b\\c", "err6 s2>s1");
    assert_eq!(r.offset, 4);
    assert_eq!(r.tail, b"c");
    // s1 and s2 can never be equal (different bytes); both arms above are hit.
    let r = compare(b"/\\", "err6 adjacent s2 later");
    assert_eq!(r.offset, 2);
    let r = compare(b"\\/", "err6 adjacent s1 later");
    assert_eq!(r.offset, 2);
    // Many of each, later one wins regardless of counts.
    let r = compare(b"////x\\y", "err6 many slashes then backslash");
    assert_eq!(r.offset, 6);
    let r = compare(b"\\\\\\\\x/y", "err6 many backslashes then slash");
    assert_eq!(r.offset, 6);
}

// ------------------------------------------------------------------ ERRORS #7
#[test]
fn err_06_oversized_length() {
    let n = 64 * 1024;
    let s = vec![b'x'; n];
    let r = compare(&s, "err7 64KiB no separator");
    assert_eq!(r.offset, 0);

    let mut s = vec![b'x'; n];
    s[n - 1] = b'/';
    let r = compare(&s, "err7 64KiB separator at end");
    assert_eq!(r.offset as usize, n);
    assert!(r.tail.is_empty());

    let mut s = vec![b'y'; n];
    s[0] = b'\\';
    let r = compare(&s, "err7 64KiB separator at start");
    assert_eq!(r.offset, 1);
    assert_eq!(r.tail.len(), n - 1);
}

// ------------------------------------------------------------------ ERRORS #8
#[test]
fn err_07_non_utf8_bytes() {
    for s in [
        &[0x80u8, 0xff, 0xfe][..],
        &[0x80, b'/', 0xff][..],
        &[0xff, b'\\', 0x80, 0x80][..],
        &[0xc3][..],           // truncated 2-byte sequence
        &[0xe0, 0x80][..],     // truncated 3-byte sequence
        &[0xed, 0xa0, 0x80][..], // encoded surrogate
        &[0xf5, 0xff, b'/', 0xf6][..],
        &[0xfe, 0xff][..],
    ] {
        let r = compare(s, "err8 non-utf8");
        let expect = s
            .iter()
            .rposition(|&b| b == b'/' || b == b'\\')
            .map(|i| i + 1)
            .unwrap_or(0);
        assert_eq!(r.offset as usize, expect);
    }
    // Every single non-NUL byte value on its own.
    for b in 1u8..=255 {
        let r = compare(&[b], "err8 single byte sweep");
        let expect = if b == b'/' || b == b'\\' { 1 } else { 0 };
        assert_eq!(r.offset as usize, expect, "byte 0x{b:02x}");
    }
}

// ------------------------------------------------------------------ ERRORS #9
#[test]
fn err_08_near_miss_separator_bytes() {
    // One step either side of '/' (0x2f) and '\\' (0x5c) must NOT match.
    for &b in &[0x2eu8, 0x30, 0x5b, 0x5d] {
        let s = [b'a', b, b'b'];
        let r = compare(&s, "err9 near-miss");
        assert_eq!(r.offset, 0, "byte 0x{b:02x} must not be a separator");
    }
    // Sanity: the real separators DO match at the same positions.
    for &b in &[b'/', b'\\'] {
        let s = [b'a', b, b'b'];
        let r = compare(&s, "err9 real separator");
        assert_eq!(r.offset, 2);
    }
    let r = compare(b".[]0:", "err9 all near-miss bytes");
    assert_eq!(r.offset, 0);
}

// ----------------------------------------------------------------- ERRORS #10
#[test]
fn err_09_interior_nul_truncates_search() {
    let r = compare_raw(b"a/b\0c/d\0", "err10 interior NUL");
    assert_eq!(r.offset, 2);
    assert_eq!(r.tail, b"b");

    // A buffer whose FIRST byte is the NUL: zero-length string with garbage
    // (including separators) behind it.
    let r = compare_raw(b"\0/x\\y\0", "err10 leading NUL");
    assert_eq!(r.offset, 0);
    assert!(r.tail.is_empty());

    // Separator immediately before the NUL, more separators after it.
    let r = compare_raw(b"ab\\\0//\0", "err10 separator then NUL");
    assert_eq!(r.offset, 3);
    assert!(r.tail.is_empty());
}

// ----------------------------------------------------------------- ERRORS #11
#[test]
fn err_10_returns_interior_pointer_no_copy() {
    let l = libs();
    let mut rng = Rng::new(Rng::SEED ^ 0xA0);
    for _ in 0..500 {
        let len = rng.range(0, 40);
        let mut s: Vec<u8> = (0..len).map(|_| rng.pick(b"ab/\\.")).collect();
        s.push(0);
        let snapshot = s.clone();

        let mut buf = s.clone();
        let base = buf.as_mut_ptr() as *mut std::ffi::c_char;
        let lo = base as usize;
        let hi = lo + len; // pointer to the NUL is the maximum legal result

        let pc = unsafe { (l.c)(base) } as usize;
        let pr = unsafe { (l.rust)(base) } as usize;

        assert_eq!(pc, pr, "returned pointers differ for {:?}", Bytes(&snapshot));
        assert!(
            (lo..=hi).contains(&pc),
            "returned pointer is not interior to the caller's buffer \
             (a copy/allocation would leak): ptr={pc:#x} buf=[{lo:#x},{hi:#x}]"
        );
        assert_eq!(buf, snapshot, "input buffer was modified");
    }
}

// ----------------------------------------------------------------- ERRORS #12
/// Discharges the "out-of-range enum across the FFI boundary" class explicitly.
/// The API has no enum, flag, mode or integer parameter at all — the single
/// parameter is `char *` — so there is no invalid-variant input to construct.
/// This test pins that fact to the actual exported signature: the symbol takes
/// exactly one pointer argument, verified by calling it through a
/// one-pointer-argument function type, and pins the remaining generic
/// boundaries (zero length, one-past-a-separator values) already covered above.
#[test]
fn configs_and_errors_coverage() {
    // No enum/flag/mode in the C source.
    let c_src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");
    let hdr = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../c_src/include/lib.h"),
    )
    .expect("read c_src/include/lib.h");
    for needle in ["enum", "#define", "#ifdef", "typedef"] {
        assert!(
            !c_src.contains(needle) && !hdr.contains(needle),
            "the C source gained a `{needle}`: ERRORS.md/CONFIGS.md row 12 \
             (out-of-range enum values) is no longer vacuous and needs a real test"
        );
    }
    // The header declares exactly one public function.
    assert_eq!(
        hdr.matches('(').count(),
        1,
        "the C header declares more than one function; SYMBOLS.md must be regenerated"
    );

    // Zero length and one-past-range boundaries, once more through the exports.
    assert_eq!(compare(b"", "coverage empty").offset, 0);
    assert_eq!(compare(&[0x2e], "coverage 0x2e").offset, 0);
    assert_eq!(compare(&[0x2f], "coverage 0x2f").offset, 1);
    assert_eq!(compare(&[0x30], "coverage 0x30").offset, 0);
    assert_eq!(compare(&[0x5b], "coverage 0x5b").offset, 0);
    assert_eq!(compare(&[0x5c], "coverage 0x5c").offset, 1);
    assert_eq!(compare(&[0x5d], "coverage 0x5d").offset, 0);
}
