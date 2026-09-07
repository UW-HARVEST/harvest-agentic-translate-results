//! Phase C — error/rejection-path differential tests. One test per ERRORS.md
//! row, plus the generic FFI boundary cases.
//!
//! Every public function in this library is `void`, so "the same error" is
//! observed as the exact stdout bytes: either a specific error string, or the
//! total absence of output. Each test therefore asserts BOTH that C and Rust
//! agree byte-for-byte AND that the produced bytes are the specific expected
//! sentinel — so "both failed somehow" cannot pass for the wrong reason.

#![allow(non_snake_case)] // test names mirror the ERRORS.md row ids (errG1, …)

mod harness;

use harness::*;
use std::ffi::{c_int, CString};

const NEGATIVE_MSG: &[u8] = b"ERROR: Array index is negative.\n";
const OOB_MSG: &[u8] = b"ERROR: Array index is out-of-bounds\n";

/// The ten lines `goodG2B()` always emits first (constant `data = 7`).
const GOOD_G2B_BLOCK: &[u8] = b"0\n0\n0\n0\n0\n0\n0\n1\n0\n0\n";

/// Ten printed lines with `1` at `idx` (or all zeros when `idx` is out of the
/// printed window).
fn block_with_one_at(idx: i64) -> Vec<u8> {
    let mut v = Vec::new();
    for i in 0..10i64 {
        v.extend_from_slice(if i == idx { b"1\n" } else { b"0\n" });
    }
    v
}

/// Runs one call against BOTH libraries, requires byte-identical output, and
/// additionally pins that output to `expected`.
fn diff_exact<F>(label: &str, expected: &[u8], op: F)
where
    F: Fn(&Api),
{
    let c_out = capture(|| op(c_api()));
    let rust_out = capture(|| op(rust_api()));
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
        "[{label}] C and Rust disagree"
    );
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(expected),
        "[{label}] C output is not the expected sentinel"
    );
}

// ===========================================================================
// Row 1-3 — printLine rejection / degenerate inputs
// ===========================================================================

/// ERRORS row 1: `line == NULL` is the library's only pointer check; it must
/// suppress ALL output (0 bytes) and must not crash.
#[test]
fn err01_print_line_null() {
    diff_exact("err01 printLine(NULL)", b"", |api| unsafe {
        (api.print_line)(std::ptr::null())
    });
    // Repeated NULL calls also stay silent (no residual state).
    diff_exact("err01 printLine(NULL) x5", b"", |api| unsafe {
        for _ in 0..5 {
            (api.print_line)(std::ptr::null());
        }
    });
}

/// ERRORS row 2: zero-length string — passes the NULL check, so exactly one
/// newline is emitted.
#[test]
fn err02_print_line_empty() {
    let s = CString::new("").unwrap();
    diff_exact("err02 printLine(\"\")", b"\n", |api| unsafe {
        (api.print_line)(s.as_ptr())
    });
}

/// ERRORS row 3: the argument must never be treated as a format string.
/// (`%n` would be an arbitrary-write primitive if it were.)
#[test]
fn err03_print_line_percent() {
    for raw in ["%s %d %n", "%n%n%n", "%s", "%p", "%99999999d", "%%"] {
        let s = CString::new(raw).unwrap();
        let mut expected = raw.as_bytes().to_vec();
        expected.push(b'\n');
        diff_exact(
            &format!("err03 printLine({raw:?})"),
            &expected,
            |api| unsafe { (api.print_line)(s.as_ptr()) },
        );
    }
}

// ===========================================================================
// Rows 4-9 — bad()
// ===========================================================================

/// ERRORS row 4: `data < 0` takes the `else` branch of `if (data >= 0)`.
#[test]
fn err04_bad_negative() {
    for v in [-1, -2, -9, -10, -1000] {
        diff_exact(&format!("err04 bad({v})"), NEGATIVE_MSG, |api| unsafe {
            (api.bad)(v)
        });
    }
}

/// ERRORS row 5: the extreme of the negative domain.
#[test]
fn err05_bad_int_min() {
    for v in [i32::MIN, i32::MIN + 1] {
        diff_exact(&format!("err05 bad({v})"), NEGATIVE_MSG, |api| unsafe {
            (api.bad)(v)
        });
    }
}

/// ERRORS row 6: `data == 9`, the last in-bounds index — must NOT be rejected.
#[test]
fn err06_bad_last_valid() {
    let expected = block_with_one_at(9);
    diff_exact("err06 bad(9)", &expected, |api| unsafe { (api.bad)(9) });
    // and index 0, the other boundary of the valid range
    let expected0 = block_with_one_at(0);
    diff_exact("err06 bad(0)", &expected0, |api| unsafe { (api.bad)(0) });
}

/// ERRORS row 7: one step past the valid range. There is **no** upper-bound
/// check in `bad`, so instead of an error message the C code writes 4 bytes
/// past `buffer` (into the frame padding at `-0x8(%rbp)`) and then prints ten
/// unchanged zeros.
#[test]
fn err07_bad_off_by_one() {
    let all_zero = block_with_one_at(-1);
    diff_exact("err07 bad(10)", &all_zero, |api| unsafe { (api.bad)(10) });
}

/// ERRORS row 8: two steps past. C writes into the loop-counter slot
/// `-0x4(%rbp)`, which `i = 0` immediately overwrites — again ten zeros, and
/// notably still NOT an error message.
#[test]
fn err08_bad_off_by_two() {
    let all_zero = block_with_one_at(-1);
    diff_exact("err08 bad(11)", &all_zero, |api| unsafe { (api.bad)(11) });
}

/// ERRORS row 9 — DOCUMENTATION ONLY, deliberately does not invoke the C code.
///
/// For `data >= 12` the C `bad()` writes through the saved `%rbp` (index 12/13)
/// and the return address (index 14/15) of its own frame:
///
/// ```text
/// 11a6:  sub  $0x40,%rsp
/// 11e0:  movl $0x1,-0x30(%rbp,%rax,4)   ; buffer base = -0x30(%rbp)
/// ```
///
/// so `buffer[12]` == `0x0(%rbp)` == saved `%rbp`, and `buffer[14]` ==
/// `0x8(%rbp)` == the return address. That is genuine undefined behaviour: the
/// C library corrupts its own control flow and the observable result is not a
/// defined value that any translation could be required to reproduce. It is
/// therefore excluded from differential assertion; `BAD_MAX_DEFINED` bounds
/// every randomized `bad`/`badData` sweep at 11.
#[test]
fn err09_bad_ub_documented() {
    assert_eq!(
        BAD_MAX_DEFINED, 11,
        "the defined domain of bad() is INT_MIN..=11; see the frame layout above"
    );
    // Prove the claim about the *defined* edge of that domain instead.
    let all_zero = block_with_one_at(-1);
    for v in 10..=BAD_MAX_DEFINED {
        diff_exact(&format!("err09 bad({v}) still defined"), &all_zero, |api| unsafe {
            (api.bad)(v)
        });
    }
}

// ===========================================================================
// Rows 10-14 — good() / goodB2G's two-conjunct range check
// ===========================================================================

fn good_expected(data: i64) -> Vec<u8> {
    let mut v = GOOD_G2B_BLOCK.to_vec();
    if (0..10).contains(&data) {
        v.extend_from_slice(&block_with_one_at(data));
    } else {
        v.extend_from_slice(OOB_MSG);
    }
    v
}

/// ERRORS row 10: fails the FIRST conjunct (`data >= 0`).
#[test]
fn err10_good_negative() {
    for v in [-1, -2, -10, -12345] {
        let expected = good_expected(v as i64);
        diff_exact(&format!("err10 good({v})"), &expected, |api| unsafe {
            (api.good)(v)
        });
    }
}

/// ERRORS row 11: extreme negative.
#[test]
fn err11_good_int_min() {
    for v in [i32::MIN, i32::MIN + 1] {
        let expected = good_expected(v as i64);
        diff_exact(&format!("err11 good({v})"), &expected, |api| unsafe {
            (api.good)(v)
        });
    }
}

/// ERRORS row 12: fails the SECOND conjunct (`data < 10`) — one step past the
/// valid range. Unlike `bad(10)`, this one IS rejected with a message.
#[test]
fn err12_good_off_by_one() {
    let expected = good_expected(10);
    diff_exact("err12 good(10)", &expected, |api| unsafe { (api.good)(10) });
    let expected11 = good_expected(11);
    diff_exact("err12 good(11)", &expected11, |api| unsafe { (api.good)(11) });
}

/// ERRORS row 13: extreme positive.
#[test]
fn err13_good_int_max() {
    for v in [i32::MAX, i32::MAX - 1, 1 << 30] {
        let expected = good_expected(v as i64);
        diff_exact(&format!("err13 good({v})"), &expected, |api| unsafe {
            (api.good)(v)
        });
    }
}

/// ERRORS row 14: `data == 9` must NOT be rejected (boundary, inclusive).
#[test]
fn err14_good_last_valid() {
    for v in [0, 9] {
        let expected = good_expected(v as i64);
        diff_exact(&format!("err14 good({v})"), &expected, |api| unsafe {
            (api.good)(v)
        });
    }
}

// ===========================================================================
// Rows 15-17 — driver()
// ===========================================================================

fn driver_expected(good_data: i64, bad_data: i64) -> Vec<u8> {
    let mut v = b"Calling good()...\n".to_vec();
    v.extend_from_slice(&good_expected(good_data));
    v.extend_from_slice(b"Finished good()\n");
    v.extend_from_slice(b"Calling bad()...\n");
    if bad_data < 0 {
        v.extend_from_slice(NEGATIVE_MSG);
    } else if bad_data < 10 {
        v.extend_from_slice(&block_with_one_at(bad_data));
    } else {
        // 10 / 11: unchecked write lands outside the printed window
        v.extend_from_slice(&block_with_one_at(-1));
    }
    v.extend_from_slice(b"Finished bad()\n");
    v
}

/// ERRORS row 15: invalid `goodData`, valid `badData`.
#[test]
fn err15_driver_bad_gooddata() {
    for g in [-1, i32::MIN, 10, 11, i32::MAX] {
        for b in [0, 7, 9] {
            let expected = driver_expected(g as i64, b as i64);
            diff_exact(&format!("err15 driver({g}, {b})"), &expected, |api| unsafe {
                (api.driver)(g, b)
            });
        }
    }
}

/// ERRORS row 16: negative `badData`, valid `goodData`.
#[test]
fn err16_driver_negative_baddata() {
    for g in [0, 5, 9] {
        for b in [-1, -100, i32::MIN, i32::MIN + 1] {
            let expected = driver_expected(g as i64, b as i64);
            diff_exact(&format!("err16 driver({g}, {b})"), &expected, |api| unsafe {
                (api.driver)(g, b)
            });
        }
    }
}

/// ERRORS row 17: both arguments invalid (but within the defined domain).
#[test]
fn err17_driver_both_invalid() {
    for g in [-1, i32::MIN, 10, i32::MAX] {
        for b in [-1, i32::MIN, 10, 11] {
            let expected = driver_expected(g as i64, b as i64);
            diff_exact(&format!("err17 driver({g}, {b})"), &expected, |api| unsafe {
                (api.driver)(g, b)
            });
        }
    }
}

// ===========================================================================
// Row 18 — printIntLine boundary formatting
// ===========================================================================

/// ERRORS row 18: `%d` rendering at the edges of the `int` domain. `INT_MIN`
/// has no positive counterpart, which is the classic negation-overflow trap.
#[test]
fn err18_print_int_line_extremes() {
    for v in [
        0i32,
        -1,
        1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        -2147483648,
        2147483647,
    ] {
        let expected = format!("{v}\n");
        diff_exact(
            &format!("err18 printIntLine({v})"),
            expected.as_bytes(),
            |api| unsafe { (api.print_int_line)(v) },
        );
    }
    // The exact glibc rendering of INT_MIN, pinned literally.
    diff_exact("err18 INT_MIN literal", b"-2147483648\n", |api| unsafe {
        (api.print_int_line)(i32::MIN)
    });
}

// ===========================================================================
// Generic FFI-boundary sweep (required even where not in the table)
// ===========================================================================

/// NULL is the only pointer this API takes, and `printLine` is the only
/// function taking it. Confirmed above; here we additionally confirm that no
/// *other* entry point can be reached with a pointer at all (the ABI is
/// `void f(int)` / `void f(int,int)`), by exercising the full int domain edge
/// set through every one of them.
#[test]
fn errG1_generic_integer_boundaries_all_entry_points() {
    let edges: [c_int; 12] = [
        i32::MIN,
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        8,
        9,
        10,
        11,
        i32::MAX - 1,
        i32::MAX,
    ];

    for v in edges {
        // printIntLine: total function, every int is valid.
        diff(&format!("errG1 printIntLine({v})"), |api| unsafe {
            (api.print_int_line)(v)
        });
        // good: total function, every int is valid.
        diff(&format!("errG1 good({v})"), |api| unsafe { (api.good)(v) });
        // bad: only defined for INT_MIN..=11.
        if v <= BAD_MAX_DEFINED {
            diff(&format!("errG1 bad({v})"), |api| unsafe { (api.bad)(v) });
        }
        // driver: goodData total, badData restricted to the defined domain.
        for b in [0, 9, 10, 11, -1, i32::MIN] {
            diff(&format!("errG1 driver({v}, {b})"), |api| unsafe {
                (api.driver)(v, b)
            });
        }
    }
}

/// "Out-of-range enum value" analogue. `driver.h` declares no enum and no
/// struct — the whole API is `void f(int …)` / `void f(const char *)`. A C
/// `enum` accepts any `int`, and here *every* parameter is already a raw `int`,
/// so the corresponding hostile input is an arbitrary bit pattern with no
/// meaningful interpretation. This pushes a large set of such patterns
/// (including values that would be invalid discriminants, sign-bit-only
/// values, and all-ones) through every total entry point.
#[test]
fn errG2_out_of_range_scalar_discriminants() {
    let mut patterns: Vec<c_int> = vec![
        0x0000_0000u32 as i32,
        0xFFFF_FFFFu32 as i32,
        0x8000_0000u32 as i32,
        0x7FFF_FFFFu32 as i32,
        0x0000_00FFu32 as i32,
        0x0000_FF00u32 as i32,
        0x00FF_0000u32 as i32,
        0xFF00_0000u32 as i32,
        0xAAAA_AAAAu32 as i32,
        0x5555_5555u32 as i32,
        0xDEAD_BEEFu32 as i32,
        0xCAFE_BABEu32 as i32,
        0x0000_000Au32 as i32, // 10 — first invalid index
        0x0000_000Bu32 as i32, // 11
        0x0000_000Cu32 as i32, // 12
        0x0000_0064u32 as i32, // 100
    ];
    let mut rng = Rng::new(SEED ^ 0xE2);
    for _ in 0..256 {
        patterns.push(rng.next_i32());
    }

    for v in patterns {
        diff(&format!("errG2 printIntLine({v:#010x})"), |api| unsafe {
            (api.print_int_line)(v)
        });
        diff(&format!("errG2 good({v:#010x})"), |api| unsafe {
            (api.good)(v)
        });
        if v <= BAD_MAX_DEFINED {
            diff(&format!("errG2 bad({v:#010x})"), |api| unsafe { (api.bad)(v) });
        }
        diff(&format!("errG2 driver({v:#010x}, -1)"), |api| unsafe {
            (api.driver)(v, -1)
        });
    }
}

/// Oversized length: `printLine` has no length limit and no internal buffer,
/// so a 64 KiB / 1 MiB string must stream through unchanged in both libraries.
#[test]
fn errG3_print_line_oversized() {
    for len in [4095usize, 4096, 4097, 65535, 65536, 65537, 1 << 20] {
        let s = CString::new(vec![b'q'; len]).unwrap();
        let mut expected = vec![b'q'; len];
        expected.push(b'\n');
        diff_exact(
            &format!("errG3 printLine(len {len})"),
            &expected,
            |api| unsafe { (api.print_line)(s.as_ptr()) },
        );
    }
}

/// A string whose only byte is the terminator, and a pointer to the last byte
/// of a large allocation (so any over-read past the NUL would be visible under
/// a sanitizer / page guard).
#[test]
fn errG4_print_line_degenerate_pointers() {
    // "\0" == empty string
    let buf = [0u8; 1];
    diff_exact("errG4 printLine(&\"\\0\")", b"\n", |api| unsafe {
        (api.print_line)(buf.as_ptr() as *const std::ffi::c_char)
    });

    // NUL at the very end of a heap block: reads must stop at the terminator.
    let mut heap = vec![b'x'; 4096];
    heap[4095] = 0;
    let mut expected = vec![b'x'; 4095];
    expected.push(b'\n');
    diff_exact(
        "errG4 printLine(NUL at end of 4 KiB block)",
        &expected,
        |api| unsafe { (api.print_line)(heap.as_ptr() as *const std::ffi::c_char) },
    );
}
