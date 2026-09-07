//! Phase C — error-path differential tests. One test per row of `ERRORS.md`.
//!
//! Every function in this library returns `void`, so the "error code" that must
//! match is the exact stdout byte stream the rejection branch emits (or the
//! absence of any output). Both `.so`s are driven through `dlsym`.

mod common;

use common::*;

/// The two diagnostic strings that appear in the C source. Byte-exact,
/// including the trailing period on one and its absence on the other.
const ERR_NEGATIVE: &[u8] = b"ERROR: Array index is negative.\n";
const ERR_OOB: &[u8] = b"ERROR: Array index is out-of-bounds\n";

fn slot_pattern(i: usize) -> Vec<u8> {
    (0..10)
        .map(|k| if k == i { "1\n" } else { "0\n" })
        .collect::<String>()
        .into_bytes()
}

fn all_zero_pattern() -> Vec<u8> {
    "0\n".repeat(10).into_bytes()
}

/// `good(d)` for invalid `d`: goodG2B's fixed pattern, then the OOB message.
fn good_rejected() -> Vec<u8> {
    let mut v = slot_pattern(7);
    v.extend_from_slice(ERR_OOB);
    v
}

// ---------------------------------------------------------------------------
// Row 1 — printLine(NULL)
// ---------------------------------------------------------------------------

#[test]
fn err_01_print_line_null() {
    let out = diff_print_line_null();
    assert!(
        out.is_empty(),
        "printLine(NULL) must emit ZERO bytes (not even a newline), got {out:?}"
    );

    // Also inside a longer stream, so a stray newline could not hide.
    let msg = b"x\0";
    let p = msg.as_ptr() as *const std::ffi::c_char;
    let out = diff(
        "printLine(x), printLine(NULL) x3, printLine(x)",
        || unsafe {
            let f = print_line::c();
            f(p);
            f(std::ptr::null());
            f(std::ptr::null());
            f(std::ptr::null());
            f(p);
        },
        || unsafe {
            let f = print_line::rs();
            f(p);
            f(std::ptr::null());
            f(std::ptr::null());
            f(std::ptr::null());
            f(p);
        },
    );
    assert_eq!(out, b"x\nx\n", "the three NULL calls must contribute nothing");
}

// ---------------------------------------------------------------------------
// Rows 2-4 — bad(data) with data < 0
// ---------------------------------------------------------------------------

#[test]
fn err_02_bad_negative_one() {
    let out = diff_bad(-1);
    assert_eq!(out, ERR_NEGATIVE, "bad(-1) rejection message");
}

#[test]
fn err_03_bad_int_min() {
    let out = diff_bad(i32::MIN);
    assert_eq!(out, ERR_NEGATIVE, "bad(INT_MIN) must take the same branch");
    let out = diff_bad(i32::MIN + 1);
    assert_eq!(out, ERR_NEGATIVE);
}

#[test]
fn err_04_bad_negative_random() {
    for d in [-1, -2, -3, -9, -10, -11, -100, -1000, -65536, -2_000_000_000] {
        let out = diff_bad(d);
        assert_eq!(out, ERR_NEGATIVE, "bad({d})");
    }
    let mut rng = Rng::new(SEED ^ 0xA001);
    for _ in 0..2048 {
        let d = rng.range_i32(i32::MIN, -1);
        let out = diff_bad(d);
        assert_eq!(out, ERR_NEGATIVE, "bad({d})");
    }
}

// ---------------------------------------------------------------------------
// Row 5 — bad(data >= 10): the guard that ISN'T there (CWE-787)
// ---------------------------------------------------------------------------

#[test]
fn err_05_bad_oob_write_not_rejected() {
    // The whole point of the test case: `bad` does NOT reject data >= 10, while
    // `goodB2G` does. Both implementations must agree on the non-rejection.
    // Restricted to `BAD_OOB_INFRAME`; beyond that the C corrupts its own return
    // address and crashes (see that constant's docs).
    for &d in BAD_OOB_INFRAME {
        let out = diff_bad(d);
        assert_eq!(out, all_zero_pattern(), "bad({d}) must NOT be rejected");
        assert!(!out.windows(6).any(|w| w == b"ERROR:"), "bad({d}) must emit no diagnostic");
    }

    // Contrast: the same value IS rejected by good()/goodB2G.
    let bad_out = diff_bad(10);
    let good_out = diff_good(10);
    assert_ne!(bad_out, good_out);
    assert!(good_out.ends_with(ERR_OOB), "good(10) must reject where bad(10) does not");
}

// ---------------------------------------------------------------------------
// Rows 6-9 — good(data) outside [0, 10)
// ---------------------------------------------------------------------------

#[test]
fn err_06_good_negative() {
    for d in [-1, -2, -10, -100] {
        let out = diff_good(d);
        assert_eq!(out, good_rejected(), "good({d})");
        // Must use the out-of-bounds wording, NOT bad()'s "negative." wording.
        assert!(out.ends_with(ERR_OOB));
        assert!(
            !out.windows(ERR_NEGATIVE.len()).any(|w| w == ERR_NEGATIVE),
            "good({d}) must not emit bad()'s message"
        );
    }
}

#[test]
fn err_07_good_ten_boundary() {
    // 9 is the last accepted index, 10 the first rejected: assert the boundary
    // is exactly there and not off by one.
    let accepted = diff_good(9);
    assert!(!accepted.ends_with(ERR_OOB), "good(9) must be accepted");
    assert_eq!(accepted[slot_pattern(7).len()..], slot_pattern(9)[..]);

    for d in [10, 11] {
        let out = diff_good(d);
        assert_eq!(out, good_rejected(), "good({d}) must be rejected");
    }
}

#[test]
fn err_08_good_int_extremes() {
    for d in [i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1] {
        let out = diff_good(d);
        assert_eq!(out, good_rejected(), "good({d})");
    }
}

#[test]
fn err_09_good_out_of_range_random() {
    let mut rng = Rng::new(SEED ^ 0xA002);
    let mut n_neg = 0;
    let mut n_high = 0;
    for _ in 0..2048 {
        let d = if rng.below(2) == 0 {
            n_neg += 1;
            rng.range_i32(i32::MIN, -1)
        } else {
            n_high += 1;
            rng.range_i32(10, i32::MAX)
        };
        let out = diff_good(d);
        assert_eq!(out, good_rejected(), "good({d})");
    }
    assert!(n_neg > 500 && n_high > 500, "both sides of the range must be sampled");
}

// ---------------------------------------------------------------------------
// Row 10 — goodG2B's else branch is dead code
// ---------------------------------------------------------------------------

#[test]
fn err_10_goodg2b_else_is_dead_code() {
    // goodG2B hardcodes `data = 7`, so `ERROR: Array index is negative.` can
    // never come out of good() for ANY input. Assert negatively over a wide set.
    let mut rng = Rng::new(SEED ^ 0xA003);
    let mut cases: Vec<i32> = vec![0, 1, 7, 9, 10, -1, i32::MIN, i32::MAX];
    for _ in 0..512 {
        cases.push(mixed_index(&mut rng));
    }
    for d in cases {
        let out = diff_good(d);
        assert!(
            !out.windows(ERR_NEGATIVE.len()).any(|w| w == ERR_NEGATIVE),
            "good({d}) emitted goodG2B's unreachable message: {:?}",
            String::from_utf8_lossy(&out)
        );
        // And goodG2B's half is always the fixed index-7 pattern.
        assert_eq!(&out[..slot_pattern(7).len()], &slot_pattern(7)[..]);
    }
}

// ---------------------------------------------------------------------------
// Rows 11-13 — rejections propagated through the composed `driver`
// ---------------------------------------------------------------------------

#[test]
fn err_11_driver_bad_negative() {
    for b in [-1, -7, i32::MIN] {
        let out = diff_driver(5, b);
        let want = {
            let mut v = b"Calling good()...\n".to_vec();
            v.extend_from_slice(&slot_pattern(7));
            v.extend_from_slice(&slot_pattern(5));
            v.extend_from_slice(b"Finished good()\n");
            v.extend_from_slice(b"Calling bad()...\n");
            v.extend_from_slice(ERR_NEGATIVE);
            v.extend_from_slice(b"Finished bad()\n");
            v
        };
        assert_eq!(out, want, "driver(5, {b})");
    }
}

#[test]
fn err_12_driver_good_out_of_range() {
    for g in [-1, 10, 11, i32::MIN, i32::MAX] {
        let out = diff_driver(g, 3);
        let want = {
            let mut v = b"Calling good()...\n".to_vec();
            v.extend_from_slice(&slot_pattern(7));
            v.extend_from_slice(ERR_OOB);
            v.extend_from_slice(b"Finished good()\n");
            v.extend_from_slice(b"Calling bad()...\n");
            v.extend_from_slice(&slot_pattern(3));
            v.extend_from_slice(b"Finished bad()\n");
            v
        };
        assert_eq!(out, want, "driver({g}, 3)");
    }
}

#[test]
fn err_13_driver_both_invalid() {
    // Both rejections in one call: the two messages are DIFFERENT and must
    // appear in the right sections in the right order.
    for (g, b) in [(-1, -1), (i32::MIN, i32::MIN), (10, -5), (-5, -10)] {
        let out = diff_driver(g, b);
        let want = {
            let mut v = b"Calling good()...\n".to_vec();
            v.extend_from_slice(&slot_pattern(7));
            v.extend_from_slice(ERR_OOB);
            v.extend_from_slice(b"Finished good()\n");
            v.extend_from_slice(b"Calling bad()...\n");
            v.extend_from_slice(ERR_NEGATIVE);
            v.extend_from_slice(b"Finished bad()\n");
            v
        };
        assert_eq!(out, want, "driver({g}, {b})");

        let oob_at = out.windows(ERR_OOB.len()).position(|w| w == ERR_OOB).unwrap();
        let neg_at = out.windows(ERR_NEGATIVE.len()).position(|w| w == ERR_NEGATIVE).unwrap();
        assert!(oob_at < neg_at, "good()'s rejection must precede bad()'s");
    }
}

// ---------------------------------------------------------------------------
// Row 14 — printIntLine has no rejection at all
// ---------------------------------------------------------------------------

#[test]
fn err_14_print_int_line_no_rejection() {
    for v in [0, -1, 1, i32::MIN, i32::MAX] {
        let out = diff_print_int_line(v);
        assert_eq!(out, format!("{v}\n").into_bytes());
        assert!(!out.windows(6).any(|w| w == b"ERROR:"), "printIntLine never rejects");
        assert!(!out.is_empty(), "printIntLine always prints, even for extremes");
    }
}

// ---------------------------------------------------------------------------
// G6 — no enum surface; the `int` equivalent is a full-range sweep
// ---------------------------------------------------------------------------

#[test]
fn err_15_no_enum_surface_full_int_sweep() {
    // driver.h declares no enum/typedef/struct, so there is no "invalid enum
    // variant" to pass. The analogous hostile input is an arbitrary `int`.
    // Sweep the range structurally through `good`, whose guard has both bounds.
    let mut cases: Vec<i32> = vec![i32::MIN, i32::MIN + 1, -1, 0, 9, 10, i32::MAX - 1, i32::MAX];
    for bit in 0..31 {
        let v = 1i32 << bit;
        cases.push(v);
        cases.push(-v);
        cases.push(v - 1);
        cases.push(v + 1);
    }
    let mut rng = Rng::new(SEED ^ 0xA004);
    for _ in 0..4096 {
        cases.push(rng.next_i32());
    }

    for d in cases {
        let out = diff_good(d);
        let want = if (0..10).contains(&d) {
            let mut v = slot_pattern(7);
            v.extend_from_slice(&slot_pattern(d as usize));
            v
        } else {
            good_rejected()
        };
        assert_eq!(out, want, "good({d}) across the FFI boundary");
    }
}

// ---------------------------------------------------------------------------
// G3 / G7 — oversized and hostile `printLine` payloads
// ---------------------------------------------------------------------------

#[test]
fn edge_print_line_huge() {
    for len in [1usize, 1024, 100_000, 1_000_000] {
        let s = vec![b'q'; len];
        let out = diff_print_line_bytes(&s);
        assert_eq!(out.len(), len + 1);
    }
}

#[test]
fn edge_print_line_percent() {
    // Repeated here as an *error-surface* case: `%n` through a format string
    // would be a write primitive, so this must stay literal data on both sides.
    for s in ["%n", "%n%n%n%n%n%n%n%n", "%s", "%999999999s", "%hhn", "%lln"] {
        let out = diff_print_line_bytes(s.as_bytes());
        assert_eq!(out, format!("{s}\n").into_bytes());
    }
}

#[test]
fn edge_print_line_non_utf8() {
    let mut rng = Rng::new(SEED ^ 0xA005);
    for _ in 0..512 {
        let len = rng.below(64) as usize;
        // Bias towards high bytes.
        let bytes: Vec<u8> = (0..len).map(|_| (0x80 + rng.below(0x80)) as u8).collect();
        let out = diff_print_line_bytes(&bytes);
        let mut want = bytes.clone();
        want.push(b'\n');
        assert_eq!(out, want);
    }
}

#[test]
fn aaa_00_harness_must_be_single_threaded() {
    assert_single_threaded();
}
