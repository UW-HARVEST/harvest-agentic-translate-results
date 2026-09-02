//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. Rows that assert a fatal signal run the
//! call inside a forked child and compare the exact termination signal, not
//! merely "both failed".

mod common;

use common::*;
use std::ffi::{c_char, c_int};

const SIGSEGV: c_int = 11;
const SIGBUS: c_int = 7;

fn foo_both(p: *const c_char, c: u8) -> (i32, i32) {
    let (c_foo, r_foo) = foo_pair();
    let cv = c as c_char;
    (unsafe { c_foo(p, cv) }, unsafe { r_foo(p, cv) })
}

// ---------------------------------------------------------------------------
// Row 1 — NULL sentinel on the first iteration: c absent from a non-empty in
// ---------------------------------------------------------------------------
#[test]
fn err01_no_match_first_iteration_returns_zero() {
    let buf = CStrBuf::new(b"bcdefghij");
    for c in 1u8..=255 {
        if b"bcdefghij".contains(&c) {
            continue;
        }
        let (a, b) = foo_both(buf.as_ptr(), c);
        assert_eq!(a, b, "c=0x{c:02x}");
        assert_eq!(a, 0, "c=0x{c:02x} must be rejected with 0");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — empty string
// ---------------------------------------------------------------------------
#[test]
fn err02_empty_string_returns_zero() {
    let buf = CStrBuf::new(b"");
    for c in 1u8..=255 {
        let (a, b) = foo_both(buf.as_ptr(), c);
        assert_eq!((a, b), (0, 0), "c=0x{c:02x}");
    }
}

// ---------------------------------------------------------------------------
// Row 3 — NULL sentinel on a later iteration: exact count, no off-by-one
// ---------------------------------------------------------------------------
#[test]
fn err03_later_termination_exact_count() {
    let mut rng = Rng::new(SEED ^ 103);
    for n in 0..64usize {
        for _ in 0..20 {
            let tail = rng.below(40);
            let mut bytes = vec![b'Z'; n + tail];
            for i in 0..n {
                bytes[i] = b'q';
            }
            // shuffle so matches are not only a prefix
            for i in (1..bytes.len()).rev() {
                let j = rng.below(i + 1);
                bytes.swap(i, j);
            }
            let buf = CStrBuf::new(&bytes);
            let (a, b) = foo_both(buf.as_ptr(), b'q');
            assert_eq!(a, b);
            assert_eq!(a, n as i32, "expected {n} matches");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — match at the last byte: must read the NUL but not past it
// ---------------------------------------------------------------------------
#[test]
fn err04_match_at_last_byte_boundary() {
    let mut rng = Rng::new(SEED ^ 104);
    for len in 1..=200usize {
        let mut bytes = vec![b'.'; len];
        bytes[len - 1] = b'q';
        let off = rng.below(64);
        let buf = CStrBuf::with_alignment(&bytes, off);
        let (a, b) = foo_both(buf.as_ptr(), b'q');
        assert_eq!(a, b, "len={len} off={off}");
        assert_eq!(a, 1, "len={len}");
    }
    // Also the degenerate 1-byte string that is entirely the match.
    let buf = CStrBuf::new(b"q");
    assert_eq!(foo_both(buf.as_ptr(), b'q'), (1, 1));
}

// ---------------------------------------------------------------------------
// Row 5 — c == '\0' is undefined behaviour (non-terminating OOB scan).
// Documented, deliberately not asserted. This test records the reasoning and
// verifies the *reachable* part: that `driver` never passes 0, and that both
// libraries agree on `strchr`-style terminator handling for every other byte.
// ---------------------------------------------------------------------------
#[test]
fn err05_nul_search_byte_is_documented_ub_not_asserted() {
    // `driver` only ever passes 'A' (0x41) and 'x' (0x78) — never 0 — so the
    // UB is unreachable through the public header API. Confirm by exercising
    // driver on an input made only of NUL-adjacent low bytes.
    let bytes: Vec<u8> = (1u8..=32).collect();
    let buf = CStrBuf::new(&bytes);
    let (c_drv, r_drv) = driver_pair();
    let p = buf.as_ptr();
    let oc = capture_stdout(|| unsafe { c_drv(p) });
    let or = capture_stdout(|| unsafe { r_drv(p) });
    assert_eq!(oc, or);
    assert_eq!(oc, b"A: 0\nx: 0\n");
    // See ERRORS.md row 5 for why foo(in, 0) is not called.
}

// ---------------------------------------------------------------------------
// Rows 6 & 7 — NULL input pointer faults identically, with the same signal
// ---------------------------------------------------------------------------
#[test]
fn err06_foo_null_input_faults_identically() {
    let (c_foo, r_foo) = foo_pair();
    let cf = (*c_foo) as usize;
    let rf = (*r_foo) as usize;

    for &c in &[b'A', b'x', 1u8, 0x7F, 0x80, 0xFF] {
        let oc = run_in_child(|| {
            let f: FooFn = unsafe { std::mem::transmute(cf) };
            let n = unsafe { f(std::ptr::null(), c as c_char) };
            // Keep the result observable so the call cannot be optimised out.
            std::hint::black_box(n);
        });
        let or = run_in_child(|| {
            let f: FooFn = unsafe { std::mem::transmute(rf) };
            let n = unsafe { f(std::ptr::null(), c as c_char) };
            std::hint::black_box(n);
        });
        assert_eq!(oc, or, "foo(NULL, 0x{c:02x}) outcome differs");
        assert!(
            matches!(oc, Outcome::Signalled(SIGSEGV) | Outcome::Signalled(SIGBUS)),
            "expected a memory fault, got {oc:?}"
        );
    }
}

#[test]
fn err07_driver_null_input_faults_identically_and_prints_nothing() {
    let (c_drv, r_drv) = driver_pair();
    let cf = (*c_drv) as usize;
    let rf = (*r_drv) as usize;

    let mut oc = None;
    let out_c = capture_stdout(|| {
        oc = Some(run_in_child(|| {
            let f: DriverFn = unsafe { std::mem::transmute(cf) };
            unsafe { f(std::ptr::null()) };
        }));
    });
    let mut or = None;
    let out_r = capture_stdout(|| {
        or = Some(run_in_child(|| {
            let f: DriverFn = unsafe { std::mem::transmute(rf) };
            unsafe { f(std::ptr::null()) };
        }));
    });
    let (oc, or) = (oc.unwrap(), or.unwrap());

    assert_eq!(oc, or, "driver(NULL) outcome differs");
    assert!(
        matches!(oc, Outcome::Signalled(SIGSEGV) | Outcome::Signalled(SIGBUS)),
        "expected a memory fault, got {oc:?}"
    );
    assert_eq!(out_c, out_r, "driver(NULL) stdout differs");
    assert!(
        out_c.is_empty(),
        "driver(NULL) must print nothing, got {:?}",
        String::from_utf8_lossy(&out_c)
    );
}

// ---------------------------------------------------------------------------
// Row 8 — garbage in the upper bits of the `char` argument register
// ---------------------------------------------------------------------------
#[test]
fn err08_upper_argument_bits_are_ignored_identically() {
    let (c_foo, r_foo) = foo_int_pair();
    let mut rng = Rng::new(SEED ^ 108);

    let bytes: Vec<u8> = (0..512).map(|i| (i % 255) as u8 + 1).collect();
    let buf = CStrBuf::new(&bytes);
    let p = buf.as_ptr();

    for low in 1u8..=255 {
        for _ in 0..4 {
            // Random garbage above the low byte, including the sign bit.
            let garbage = (rng.next_u64() as u32) & 0xFFFF_FF00;
            let arg = (garbage | low as u32) as c_int;
            let a = unsafe { c_foo(p, arg) };
            let b = unsafe { r_foo(p, arg) };
            assert_eq!(a, b, "arg=0x{arg:08x} low=0x{low:02x}");
            // And it must equal the result of passing the clean low byte.
            let clean = unsafe { c_foo(p, low as c_int) };
            assert_eq!(a, clean, "upper bits changed the C result");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9 — high-bit search bytes (negative signed char)
// ---------------------------------------------------------------------------
#[test]
fn err09_high_bit_search_bytes_not_confused() {
    let bytes: Vec<u8> = (1u8..=255).collect();
    let buf = CStrBuf::new(&bytes);
    for c in 0x80u8..=0xFF {
        let (a, b) = foo_both(buf.as_ptr(), c);
        assert_eq!(a, b, "c=0x{c:02x}");
        assert_eq!(a, 1, "0x{c:02x} occurs exactly once in 0x01..0xFF");
    }
    // The complementary check: a low byte must not match its high-bit twin.
    let only_high = CStrBuf::new(&[0x80u8, 0xC1, 0xFF]);
    for &c in &[0x00u8 | 0x41, 0x7F, 0x01] {
        let (a, b) = foo_both(only_high.as_ptr(), c);
        assert_eq!((a, b), (0, 0), "c=0x{c:02x} must not match high-bit bytes");
    }
}

// ---------------------------------------------------------------------------
// Row 10 — extreme non-zero signed-char boundaries
// ---------------------------------------------------------------------------
#[test]
fn err10_signed_char_extremes() {
    for &c in &[0x01u8, 0x7Fu8, 0x80u8, 0xFFu8] {
        // Present exactly three times, plus adjacent neighbour values.
        let neighbours = [c.wrapping_sub(1), c.wrapping_add(1)];
        let mut bytes = vec![b'.'; 32];
        bytes[0] = c;
        bytes[15] = c;
        bytes[31] = c;
        for (i, &n) in neighbours.iter().enumerate() {
            if n != 0 {
                bytes[5 + i] = n;
            }
        }
        let buf = CStrBuf::new(&bytes);
        let (a, b) = foo_both(buf.as_ptr(), c);
        assert_eq!(a, b, "c=0x{c:02x}");
        assert_eq!(a, 3, "c=0x{c:02x}");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — every byte matches
// ---------------------------------------------------------------------------
#[test]
fn err11_all_bytes_match_returns_strlen() {
    for &len in &[1usize, 2, 16, 17, 63, 64, 65, 1000, 4096] {
        for &c in &[b'A', b'x', 0x01u8, 0xFFu8] {
            let bytes = vec![c; len];
            let buf = CStrBuf::new(&bytes);
            let (a, b) = foo_both(buf.as_ptr(), c);
            assert_eq!(a, b, "len={len} c=0x{c:02x}");
            assert_eq!(a, len as i32);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12 — int overflow of `res` is unreachable (>2 GiB input). Verify that
// counts far above 16 bits are still exact, i.e. `res` is a full 32-bit int
// and not silently truncated to 8 or 16 bits anywhere.
// ---------------------------------------------------------------------------
#[test]
fn err12_counts_beyond_8_and_16_bit_ranges() {
    for &n in &[255usize, 256, 257, 65535, 65536, 65537, 200_000] {
        let bytes = vec![b'A'; n];
        let buf = CStrBuf::new(&bytes);
        let (a, b) = foo_both(buf.as_ptr(), b'A');
        assert_eq!(a, b, "n={n}");
        assert_eq!(a, n as i32, "n={n} truncated?");
    }
}

// ---------------------------------------------------------------------------
// Rows 13/14/15 — driver printf formatting boundaries
// ---------------------------------------------------------------------------
#[test]
fn err13_driver_zero_and_mixed_counts() {
    let (c_drv, r_drv) = driver_pair();
    for (input, expect) in [
        (b"".to_vec(), "A: 0\nx: 0\n"),
        (b"zzz".to_vec(), "A: 0\nx: 0\n"),
        (b"A".to_vec(), "A: 1\nx: 0\n"),
        (b"x".to_vec(), "A: 0\nx: 1\n"),
        (b"Ax".to_vec(), "A: 1\nx: 1\n"),
        (b"AAAAAAAAAAx".to_vec(), "A: 10\nx: 1\n"),
        (b"AxxxxxxxxxxA".to_vec(), "A: 2\nx: 10\n"),
    ] {
        let buf = CStrBuf::new(&input);
        let p = buf.as_ptr();
        let oc = capture_stdout(|| unsafe { c_drv(p) });
        let or = capture_stdout(|| unsafe { r_drv(p) });
        assert_eq!(oc, or, "input={:?}", String::from_utf8_lossy(&input));
        assert_eq!(oc, expect.as_bytes());
    }
}

#[test]
fn err14_driver_digit_width_boundaries() {
    let (c_drv, r_drv) = driver_pair();
    for &(na, nx) in &[
        (0usize, 0usize),
        (1, 0),
        (9, 9),
        (10, 10),
        (99, 99),
        (100, 100),
        (999, 999),
        (1000, 1000),
        (9999, 9999),
        (10000, 10000),
        (99999, 1),
    ] {
        let mut bytes = vec![b'A'; na];
        bytes.extend(std::iter::repeat(b'x').take(nx));
        let buf = CStrBuf::new(&bytes);
        let p = buf.as_ptr();
        let oc = capture_stdout(|| unsafe { c_drv(p) });
        let or = capture_stdout(|| unsafe { r_drv(p) });
        assert_eq!(oc, or, "na={na} nx={nx}");
        assert_eq!(oc, format!("A: {na}\nx: {nx}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 15 — zero-length input to `driver`
// ---------------------------------------------------------------------------
#[test]
fn err15_driver_empty_input() {
    let (c_drv, r_drv) = driver_pair();
    // Both an empty heap string and an empty string at every alignment inside
    // a 64-byte-aligned allocation.
    for off in 0..64usize {
        let buf = CStrBuf::with_alignment(b"", off);
        let p = buf.as_ptr();
        let oc = capture_stdout(|| unsafe { c_drv(p) });
        let or = capture_stdout(|| unsafe { r_drv(p) });
        assert_eq!(oc, or, "off={off}");
        assert_eq!(oc, b"A: 0\nx: 0\n", "off={off}");
    }
    // And through the low-level entry point for every non-zero search byte.
    let buf = CStrBuf::new(b"");
    for c in 1u8..=255 {
        assert_eq!(foo_both(buf.as_ptr(), c), (0, 0), "c=0x{c:02x}");
    }
}

// ---------------------------------------------------------------------------
// Row 16 — arbitrary non-UTF-8 bytes are not validated or rejected
// ---------------------------------------------------------------------------
#[test]
fn err16_non_utf8_bytes_not_rejected() {
    let mut rng = Rng::new(SEED ^ 116);
    let (c_drv, r_drv) = driver_pair();
    for _ in 0..200 {
        let len = rng.below(300);
        // Deliberately invalid UTF-8: lone continuation bytes and 0xFF/0xFE.
        let bytes: Vec<u8> = (0..len)
            .map(|_| match rng.below(3) {
                0 => 0x80 + (rng.next_u64() % 0x40) as u8,
                1 => 0xFE + (rng.next_u64() % 2) as u8,
                _ => rng.nonzero_byte(),
            })
            .collect();
        let buf = CStrBuf::new(&bytes);
        let p = buf.as_ptr();
        for c in [b'A', b'x', 0x80u8, 0xFFu8] {
            let (a, b) = foo_both(p, c);
            assert_eq!(a, b);
            assert_eq!(a, bytes.iter().filter(|&&x| x == c).count() as i32);
        }
        let oc = capture_stdout(|| unsafe { c_drv(p) });
        let or = capture_stdout(|| unsafe { r_drv(p) });
        assert_eq!(oc, or);
    }
}

// ---------------------------------------------------------------------------
// Generic boundaries beyond the table: out-of-range "enum-like" ints passed
// across the FFI boundary for the `char` parameter. The C API has no enums,
// but the second parameter is a narrow type reached through an `int` register,
// so the whole `int` range is a real input the C handles.
// ---------------------------------------------------------------------------
#[test]
fn generic_out_of_range_int_values_for_char_param() {
    let (c_foo, r_foo) = foo_int_pair();
    let bytes: Vec<u8> = (1u8..=255).collect();
    let buf = CStrBuf::new(&bytes);
    let p = buf.as_ptr();

    let mut cases: Vec<c_int> = vec![
        0, 1, -1, 127, 128, -128, -129, 255, 256, 257, 65535, 65536, 0x1_0041, 0x7FFF_FFFF,
        -0x8000_0000, -256, -257, -32768, 0x0000_FF41, 0x00FF_FF41,
    ];
    let mut rng = Rng::new(SEED ^ 200);
    for _ in 0..2000 {
        cases.push(rng.next_u64() as c_int);
    }

    for &v in &cases {
        // Skip values whose low byte is 0: that is the documented UB of
        // ERRORS.md row 5 (non-terminating out-of-bounds scan).
        if (v as u8) == 0 {
            continue;
        }
        let a = unsafe { c_foo(p, v) };
        let b = unsafe { r_foo(p, v) };
        assert_eq!(a, b, "int arg {v} (0x{:08x})", v as u32);
    }
}

// ---------------------------------------------------------------------------
// Generic boundaries: oversized / long inputs, and a page-boundary-adjacent
// string (the last bytes of a mapped page, so any over-read would fault).
// ---------------------------------------------------------------------------
#[test]
fn generic_string_ending_exactly_at_page_boundary() {
    use std::ffi::c_void;
    unsafe extern "C" {
        fn mmap(
            addr: *mut c_void,
            len: usize,
            prot: c_int,
            flags: c_int,
            fd: c_int,
            off: i64,
        ) -> *mut c_void;
        fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> c_int;
        fn munmap(addr: *mut c_void, len: usize) -> c_int;
    }
    const PROT_NONE: c_int = 0;
    const PROT_READ: c_int = 1;
    const PROT_WRITE: c_int = 2;
    const MAP_PRIVATE: c_int = 2;
    const MAP_ANONYMOUS: c_int = 0x20;
    const PAGE: usize = 4096;

    let base = unsafe {
        mmap(
            std::ptr::null_mut(),
            2 * PAGE,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert!(base as isize != -1, "mmap failed");
    // Make the second page unreadable: any over-read past the NUL faults.
    assert_eq!(
        unsafe { mprotect((base as *mut u8).add(PAGE) as *mut c_void, PAGE, PROT_NONE) },
        0
    );

    let (c_foo, r_foo) = foo_pair();
    for len in [1usize, 2, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 100, 4095] {
        let start = unsafe { (base as *mut u8).add(PAGE - 1 - len) };
        for i in 0..len {
            unsafe { *start.add(i) = if i % 3 == 0 { b'A' } else { b'.' } };
        }
        unsafe { *start.add(len) = 0 };
        let expect = (0..len).filter(|i| i % 3 == 0).count() as i32;
        let p = start as *const c_char;
        let a = unsafe { c_foo(p, b'A' as c_char) };
        let b = unsafe { r_foo(p, b'A' as c_char) };
        assert_eq!(a, b, "len={len}");
        assert_eq!(a, expect, "len={len}");
        // A search byte that never occurs: the scan must stop at the NUL.
        let a = unsafe { c_foo(p, b'Z' as c_char) };
        let b = unsafe { r_foo(p, b'Z' as c_char) };
        assert_eq!((a, b), (0, 0), "len={len} no-match");
    }

    unsafe { munmap(base, 2 * PAGE) };
}
