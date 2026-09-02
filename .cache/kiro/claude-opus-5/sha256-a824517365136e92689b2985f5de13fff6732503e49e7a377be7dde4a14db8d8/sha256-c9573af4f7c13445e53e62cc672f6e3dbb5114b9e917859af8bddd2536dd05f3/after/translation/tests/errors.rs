//! Phase C — error-path / rejection differential tests, one per row of
//! `ERRORS.md`, plus the generic FFI boundary classes G1–G7.
//!
//! Every call crosses the `.so` boundary via `libloading` for BOTH
//! implementations. The Rust crate is never called directly.
//!
//! `memchra2` has no pointer, length, or enum parameter, so most C guards
//! cannot be driven from outside. For those rows the test proves the guard's
//! *outcome* instead: an independent oracle (written fresh from the C source,
//! below) reconstructs the internal value the guard controls, asserts the guard
//! landed on the side `ERRORS.md` claims, and asserts C == Rust == oracle. If
//! either implementation ever took the other side of the guard, the oracle
//! comparison would fail.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Independent oracle, transcribed directly from c_src/src/lib.c.
// ---------------------------------------------------------------------------

struct Trace {
    result: c_int,
    /// `char buffer[64]` contents up to (excluding) the NUL.
    buf: Vec<u8>,
    /// return of `count_occurrences(buffer, '-')`
    dash_count: c_int,
    /// return of `safe_sum_array(values, 4)`
    sum: c_int,
    /// return of `process_strings(test_strings, 4, "test")`
    matches: c_int,
    /// whether `f > 0.0f && f < 1000.0f` held
    float_taken: bool,
    /// the `(int)f` contribution actually added (0 when the branch is skipped)
    float_contrib: c_int,
    /// return of `process_buffer(buffer, strlen(buffer))`
    buf_sum: c_int,
    /// return of `interpret_as_int(bytes, 4)`
    interpreted: c_int,
    /// return of `complex_iteration(values, 4)`
    complex_result: c_int,
}

fn oracle(a: c_int, b: c_int, c: c_int, d: c_int) -> Trace {
    let mut result: c_int = 0;

    // snprintf(buffer, 64, "test%d-%d-%d-%d", a, b, c, d)
    let s = format!("test{}-{}-{}-{}", a, b, c, d);
    let bytes_all = s.as_bytes();
    let keep = core::cmp::min(bytes_all.len(), 63); // sizeof(buffer) - 1
    let buf: Vec<u8> = bytes_all[..keep].to_vec();

    // count_occurrences(buffer, '-') -> memchra(text, ch, strlen(text))
    let dash_count: c_int = if buf.is_empty() {
        0
    } else {
        buf.iter().filter(|&&x| x == b'-').count() as c_int
    };
    result = result.wrapping_add(dash_count.wrapping_mul(10));

    // safe_sum_array(values, 4)
    let sum = a.wrapping_add(b).wrapping_add(c).wrapping_add(d);
    result = result.wrapping_add(sum);

    // process_strings({"test1","test2","testing","other"}, 4, "test")
    let matches: c_int = ["test1", "test2", "testing", "other"]
        .iter()
        .filter(|s| s.as_bytes().starts_with(b"test"))
        .count() as c_int;
    result = result.wrapping_add(matches.wrapping_mul(5));

    // int_to_float_bits(a) + range test
    let f = f32::from_bits(a as u32);
    let float_taken = f > 0.0f32 && f < 1000.0f32;
    let float_contrib: c_int = if float_taken { f as c_int } else { 0 };
    result = result.wrapping_add(float_contrib);

    // process_buffer(buffer, strlen(buffer))
    let buf_sum: c_int = if buf.is_empty() {
        -1 // *buffer == '\0'
    } else {
        buf.iter().fold(0i32, |acc, &x| acc.wrapping_add(x as i8 as c_int))
    };
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    // interpret_as_int({b&0xFF, c&0xFF, d&0xFF, 0}, 4) — little-endian
    let interpreted = c_int::from_le_bytes([
        (b & 0xFF) as u8,
        (c & 0xFF) as u8,
        (d & 0xFF) as u8,
        0,
    ]);
    result ^= interpreted;

    // complex_iteration(values, 4)
    let complex_result: c_int = [a, b, c, d]
        .iter()
        .fold(0i32, |acc, &v| acc ^ ((v as u32 & 0xFF) as c_int));
    result = result.wrapping_add(complex_result);

    Trace {
        result,
        buf,
        dash_count,
        sum,
        matches,
        float_taken,
        float_contrib,
        buf_sum,
        interpreted,
        complex_result,
    }
}

/// Asserts C == Rust == oracle and returns the trace for further assertions.
#[track_caller]
fn check(p: &Pair, label: &str, a: c_int, b: c_int, c: c_int, d: c_int) -> Trace {
    let t = oracle(a, b, c, d);
    let gc = p.c(a, b, c, d);
    let gr = p.rust(a, b, c, d);
    assert_eq!(gc, gr, "[{label}] C/Rust divergence at ({a},{b},{c},{d})");
    assert_eq!(
        gc, t.result,
        "[{label}] oracle divergence at ({a},{b},{c},{d}): impls={gc} oracle={}",
        t.result
    );
    t
}

/// The input tuples used to probe each guard: extremes, near-zero, low-byte
/// boundaries, plus a deterministic random sample.
fn probe_inputs() -> Vec<(c_int, c_int, c_int, c_int)> {
    let mut v = Vec::new();
    let fixed: [c_int; 12] = [
        0,
        1,
        -1,
        127,
        128,
        255,
        256,
        -128,
        -256,
        i32::MIN,
        i32::MAX,
        0x3F80_0000,
    ];
    for &a in fixed.iter() {
        for &b in fixed.iter() {
            v.push((a, b, 0, 0));
            v.push((a, 0, b, 0));
            v.push((a, 0, 0, b));
            v.push((a, b, b, b));
        }
    }
    let mut rng = Rng::new(0xE770_0000);
    for _ in 0..2000 {
        v.push((rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32()));
    }
    v
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 1–2, 11–12: process_buffer / count_occurrences NULL + empty.
// ---------------------------------------------------------------------------

#[test]
fn err_row01_process_buffer_null_unreachable() {
    // `process_buffer`'s only caller passes `char buffer[64]`, the address of a
    // local array, which can never be NULL. The guard's NULL disjunct is
    // therefore dead. Proof of the guard outcome: buf_sum is never the -1
    // sentinel and the full byte sum is always accumulated.
    let p = Pair::load();
    for (a, b, c, d) in probe_inputs() {
        let t = check(&p, "row01", a, b, c, d);
        assert_ne!(t.buf_sum, -1, "row01: NULL/empty guard must not fire");
    }
}

#[test]
fn err_row02_process_buffer_empty_unreachable() {
    // `*buffer == '\0'` requires the formatted string to be empty, but
    // `snprintf` always writes at least the literal prefix "test".
    let p = Pair::load();
    for (a, b, c, d) in probe_inputs() {
        let t = check(&p, "row02", a, b, c, d);
        assert!(!t.buf.is_empty(), "row02: buffer must be non-empty");
        assert_eq!(t.buf[0], b't', "row02: first byte is always 't'");
        assert_ne!(t.buf_sum, -1, "row02: -1 sentinel must not be returned");
    }
}

#[test]
fn err_row11_count_occurrences_null_unreachable() {
    let p = Pair::load();
    for (a, b, c, d) in probe_inputs() {
        let t = check(&p, "row11", a, b, c, d);
        // Guard not fired => dash_count reflects the real separator count,
        // which is >= 3 (three literal '-' separators), never the 0 sentinel.
        assert!(
            (3..=7).contains(&t.dash_count),
            "row11: dash_count {} outside 3..=7",
            t.dash_count
        );
    }
}

#[test]
fn err_row12_count_occurrences_empty_unreachable() {
    let p = Pair::load();
    for (a, b, c, d) in probe_inputs() {
        let t = check(&p, "row12", a, b, c, d);
        assert_eq!(t.buf[0], b't');
        assert_ne!(t.dash_count, 0, "row12: empty-string sentinel must not fire");
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 3–6: process_strings guards.
// ---------------------------------------------------------------------------

fn assert_process_strings_guard_clear(label: &str) {
    let p = Pair::load();
    for (a, b, c, d) in probe_inputs() {
        let t = check(&p, label, a, b, c, d);
        // The array literal is non-NULL, count is the literal 4, and all four
        // elements are non-NULL non-empty string literals; so neither the
        // whole-array guards (rows 3/4, sentinel 0) nor the per-element
        // `continue` guards (rows 5/6) can fire, and exactly the three
        // "test"-prefixed literals are counted.
        assert_eq!(t.matches, 3, "{label}: process_strings must return 3");
    }
}

#[test]
fn err_row03_process_strings_null_unreachable() {
    assert_process_strings_guard_clear("row03");
}

#[test]
fn err_row04_process_strings_count_le_zero_unreachable() {
    assert_process_strings_guard_clear("row04");
}

#[test]
fn err_row05_process_strings_null_element_unreachable() {
    assert_process_strings_guard_clear("row05");
}

#[test]
fn err_row06_process_strings_empty_element_unreachable() {
    assert_process_strings_guard_clear("row06");
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 7–8: safe_sum_array guards.
// ---------------------------------------------------------------------------

fn assert_safe_sum_guard_clear(label: &str) {
    let p = Pair::load();
    for (a, b, c, d) in probe_inputs() {
        let t = check(&p, label, a, b, c, d);
        // Guards not fired => the real wrapping sum is returned rather than the
        // 0 sentinel. (A genuine sum of 0 is indistinguishable by value, so the
        // assertion is on the computation, which is what the guard controls.)
        assert_eq!(
            t.sum,
            a.wrapping_add(b).wrapping_add(c).wrapping_add(d),
            "{label}: safe_sum_array must sum all 4 elements"
        );
    }
}

#[test]
fn err_row07_safe_sum_array_null_unreachable() {
    assert_safe_sum_guard_clear("row07");
}

#[test]
fn err_row08_safe_sum_array_zero_size_unreachable() {
    assert_safe_sum_guard_clear("row08");
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 9–10: interpret_as_int guards.
// ---------------------------------------------------------------------------

fn assert_interpret_guard_clear(label: &str) {
    let p = Pair::load();
    assert_eq!(
        core::mem::size_of::<c_int>(),
        4,
        "{label}: reference platform has sizeof(int)==4, so len==4 passes the guard"
    );
    for (a, b, c, d) in probe_inputs() {
        let t = check(&p, label, a, b, c, d);
        let expected = c_int::from_le_bytes([(b & 0xFF) as u8, (c & 0xFF) as u8, (d & 0xFF) as u8, 0]);
        assert_eq!(t.interpreted, expected, "{label}: real load, not 0 sentinel");
        // bytes[3] == 0, so the loaded value is always in 0..=0x00FFFFFF.
        assert!((0..=0x00FF_FFFF).contains(&t.interpreted), "{label}: range");
    }
}

#[test]
fn err_row09_interpret_as_int_null_unreachable() {
    assert_interpret_guard_clear("row09");
}

#[test]
fn err_row10_interpret_as_int_short_len_unreachable() {
    assert_interpret_guard_clear("row10");
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 13–14: complex_iteration guards (sentinel -1).
// ---------------------------------------------------------------------------

fn assert_complex_iteration_guard_clear(label: &str) {
    let p = Pair::load();
    for (a, b, c, d) in probe_inputs() {
        let t = check(&p, label, a, b, c, d);
        // The XOR fold of `u & 0xFF` terms always lands in 0..=255, so it can
        // never collide with the -1 error sentinel: observing a value in that
        // range proves neither guard fired.
        assert!(
            (0..=255).contains(&t.complex_result),
            "{label}: complex_iteration returned {} (would be -1 if the guard fired)",
            t.complex_result
        );
        assert_eq!(
            t.complex_result,
            (a & 0xFF) ^ (b & 0xFF) ^ (c & 0xFF) ^ (d & 0xFF),
            "{label}: XOR fold over all 4 elements"
        );
    }
}

#[test]
fn err_row13_complex_iteration_null_unreachable() {
    assert_complex_iteration_guard_clear("row13");
}

#[test]
fn err_row14_complex_iteration_zero_count_unreachable() {
    assert_complex_iteration_guard_clear("row14");
}

// ---------------------------------------------------------------------------
// ERRORS.md row 15: snprintf truncation is unreachable.
// ---------------------------------------------------------------------------

#[test]
fn err_row15_snprintf_never_truncates() {
    let p = Pair::load();
    // Worst case: "test" (4) + 4 x "-2147483648" (11 each) + 3 separators = 51.
    let worst = format!("test{}-{}-{}-{}", i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    assert_eq!(worst.len(), 51, "worst-case formatted length");
    assert!(worst.len() <= 63, "fits in sizeof(buffer)-1 == 63");

    // Exhaustively confirm no input can exceed 63 bytes, then check the
    // longest-output inputs differentially.
    let widest: [c_int; 6] = [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        -1_999_999_999,
        -1_000_000_000,
        1_999_999_999,
    ];
    for &a in widest.iter() {
        for &b in widest.iter() {
            for &c in widest.iter() {
                for &d in widest.iter() {
                    let t = check(&p, "row15", a, b, c, d);
                    assert!(
                        t.buf.len() <= 51,
                        "row15: buffer length {} exceeded the proven bound",
                        t.buf.len()
                    );
                    assert_eq!(
                        t.buf,
                        format!("test{}-{}-{}-{}", a, b, c, d).into_bytes(),
                        "row15: no truncation occurred"
                    );
                }
            }
        }
    }
    let mut rng = Rng::new(0xE770_0015);
    for _ in 0..5000 {
        let (a, b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        let t = check(&p, "row15", a, b, c, d);
        assert!(t.buf.len() <= 51);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 16: the float-range branch being REJECTED (reachable).
// ---------------------------------------------------------------------------

#[test]
fn err_row16_float_branch_rejected() {
    let p = Pair::load();
    let mut rng = Rng::new(0xE770_0016);
    // Every class of `a` for which `f > 0.0f && f < 1000.0f` is FALSE.
    for class in [
        AClass::Zero,
        AClass::PosGeThousand,
        AClass::PosInfNan,
        AClass::Negative,
    ] {
        let mut vals = class.boundaries();
        for _ in 0..500 {
            vals.push(class.sample(&mut rng));
        }
        for a in vals {
            for _ in 0..3 {
                let (b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
                let t = check(&p, "row16-reject", a, b, c, d);
                assert!(
                    !t.float_taken,
                    "row16: class {:?} value 0x{a:08x} unexpectedly took the branch",
                    class
                );
                assert_eq!(t.float_contrib, 0, "row16: no contribution when rejected");
            }
        }
    }
    // And the accepted classes, to confirm the two sides really are different
    // and that both implementations agree on which side is taken.
    for class in [
        AClass::PosSubnormal,
        AClass::PosNormLtOne,
        AClass::PosNormInRange,
    ] {
        let mut vals = class.boundaries();
        for _ in 0..500 {
            vals.push(class.sample(&mut rng));
        }
        for a in vals {
            let (b, c, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32());
            let t = check(&p, "row16-accept", a, b, c, d);
            assert!(t.float_taken, "row16: class {:?} must take the branch", class);
        }
    }
    // The exact cut points.
    for &a in [
        0x0000_0000u32,
        0x0000_0001,
        0x4479_FFFF,
        0x447A_0000,
        0x7F7F_FFFF,
        0x7F80_0000,
        0x8000_0000,
        0xFFFF_FFFF,
    ]
    .iter()
    {
        check(&p, "row16-cut", a as i32, 0, 0, 0);
    }
}

// ---------------------------------------------------------------------------
// ERRORS.md row 17: `buf_sum > 0` never fails.
// ---------------------------------------------------------------------------

#[test]
fn err_row17_buf_sum_always_positive() {
    let p = Pair::load();
    for (a, b, c, d) in probe_inputs() {
        let t = check(&p, "row17", a, b, c, d);
        assert!(
            t.buf_sum > 0,
            "row17: buf_sum {} was not positive for ({a},{b},{c},{d})",
            t.buf_sum
        );
        // Buffer holds only 't','e','s','t', ASCII digits and '-', all of which
        // have positive `(int)(char)` values, so no cancellation is possible.
        for &byte in t.buf.iter() {
            assert!(
                byte == b'-' || byte == b't' || byte == b'e' || byte == b's' || byte.is_ascii_digit(),
                "row17: unexpected buffer byte 0x{byte:02x}"
            );
            assert!((byte as i8) > 0, "row17: byte 0x{byte:02x} is not positive as signed char");
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary classes G1–G7.
// ---------------------------------------------------------------------------

#[test]
fn boundary_int_extremes_cross_product() {
    // G1: INT_MIN / INT_MAX in every argument position, all 4^4 combinations.
    let p = Pair::load();
    let ext: [c_int; 4] = [i32::MIN, i32::MAX, 0, -1];
    for &a in ext.iter() {
        for &b in ext.iter() {
            for &c in ext.iter() {
                for &d in ext.iter() {
                    check(&p, "G1", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn boundary_one_step_past() {
    // G2: one step past the extremes and around zero.
    let p = Pair::load();
    let v: [c_int; 8] = [i32::MIN, i32::MIN + 1, -2, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in v.iter() {
        for &b in v.iter() {
            for &c in v.iter() {
                for &d in v.iter() {
                    check(&p, "G2", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn boundary_sum_overflow() {
    // G3: signed overflow of a+b+c+d inside safe_sum_array.
    let p = Pair::load();
    let cases: [(c_int, c_int, c_int, c_int); 10] = [
        (i32::MAX, 1, 0, 0),
        (i32::MAX, i32::MAX, 0, 0),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, -1, 0, 0),
        (i32::MIN, i32::MIN, 0, 0),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
        (2_000_000_000, 2_000_000_000, 0, 0),
        (-2_000_000_000, -2_000_000_000, 0, 0),
        (1_073_741_824, 1_073_741_824, 1_073_741_824, 1_073_741_824),
    ];
    for &(a, b, c, d) in cases.iter() {
        check(&p, "G3", a, b, c, d);
    }
    let mut rng = Rng::new(0xE770_0003);
    for _ in 0..3000 {
        let a = (rng.range_u32(0x6000_0000, 0x7FFF_FFFF)) as i32;
        let b = (rng.range_u32(0x6000_0000, 0x7FFF_FFFF)) as i32;
        let c = (rng.range_u32(0x6000_0000, 0x7FFF_FFFF)) as i32;
        let d = (rng.range_u32(0x6000_0000, 0x7FFF_FFFF)) as i32;
        check(&p, "G3-rand", a, b, c, d);
    }
}

#[test]
fn boundary_low_byte_extremes() {
    // G4: x & 0xFF == 0x00 and 0xFF for every argument.
    let p = Pair::load();
    let mut rng = Rng::new(0xE770_0004);
    for pattern in 0u8..16 {
        for _ in 0..200 {
            let mk = |rng: &mut Rng, bit: u8| -> c_int {
                let hi = rng.next_u32() & 0xFFFF_FF00;
                let lo = if pattern & bit != 0 { 0xFFu32 } else { 0x00 };
                (hi | lo) as i32
            };
            let a = mk(&mut rng, 1);
            let b = mk(&mut rng, 2);
            let c = mk(&mut rng, 4);
            let d = mk(&mut rng, 8);
            check(&p, "G4", a, b, c, d);
        }
    }
}

#[test]
fn boundary_char_sign_extension() {
    // G5: (char)c sign boundary in memchra / count_occurrences and in
    // process_buffer's `(int)(*i)` conversion.
    let p = Pair::load();
    let bytes: [u8; 8] = [0x00, 0x01, 0x2D, 0x7E, 0x7F, 0x80, 0x81, 0xFF];
    let mut rng = Rng::new(0xE770_0005);
    for &ab in bytes.iter() {
        for &bb in bytes.iter() {
            for &cb in bytes.iter() {
                for &db in bytes.iter() {
                    let mk = |rng: &mut Rng, lo: u8| -> c_int {
                        ((rng.next_u32() & 0xFFFF_FF00) | lo as u32) as i32
                    };
                    let a = mk(&mut rng, ab);
                    let b = mk(&mut rng, bb);
                    let c = mk(&mut rng, cb);
                    let d = mk(&mut rng, db);
                    check(&p, "G5", a, b, c, d);
                    check(&p, "G5-lit", ab as i32, bb as i32, cb as i32, db as i32);
                }
            }
        }
    }
}

#[test]
fn boundary_ieee754_classes() {
    // G6: every IEEE-754 class of `a`, and the 1000.0f cut point +/- 1 ulp.
    let p = Pair::load();
    let named: [(&str, u32); 14] = [
        ("+0.0", 0x0000_0000),
        ("-0.0", 0x8000_0000),
        ("min subnormal", 0x0000_0001),
        ("max subnormal", 0x007F_FFFF),
        ("min normal", 0x0080_0000),
        ("1.0", 0x3F80_0000),
        ("just below 1000", 0x4479_FFFF),
        ("exactly 1000", 0x447A_0000),
        ("just above 1000", 0x447A_0001),
        ("max finite", 0x7F7F_FFFF),
        ("+inf", 0x7F80_0000),
        ("-inf", 0xFF80_0000),
        ("quiet NaN", 0x7FC0_0000),
        ("signalling NaN", 0x7F80_0001),
    ];
    let mut rng = Rng::new(0xE770_0006);
    for &(name, bits) in named.iter() {
        let a = bits as i32;
        check(&p, name, a, 0, 0, 0);
        check(&p, name, a, i32::MIN, i32::MAX, -1);
        for _ in 0..64 {
            check(&p, name, a, rng.next_i32(), rng.next_i32(), rng.next_i32());
        }
        // +/- 1 ulp on both sides of each landmark.
        for delta in [-1i64, 1] {
            let v = ((bits as i64 + delta) as u32) as i32;
            check(&p, name, v, 0, 0, 0);
        }
    }
}

#[test]
fn boundary_nan_payloads() {
    // G7: the analogue of "an out-of-range enum value with no valid variant" —
    // bit patterns of `a` whose float reinterpretation has no numeric value.
    // C accepts any int here, so the Rust must handle them identically.
    let p = Pair::load();
    let mut rng = Rng::new(0xE770_0007);
    for _ in 0..4000 {
        // Positive NaN payloads: 0x7F800001 ..= 0x7FFFFFFF.
        let a = rng.range_u32(0x7F80_0001, 0x7FFF_FFFF) as i32;
        check(&p, "G7-pos-nan", a, rng.next_i32(), rng.next_i32(), rng.next_i32());
        // Negative NaN payloads: 0xFF800001 ..= 0xFFFFFFFF.
        let a = rng.range_u32(0xFF80_0001, 0xFFFF_FFFF) as i32;
        check(&p, "G7-neg-nan", a, rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    // Both infinities and both zeros explicitly.
    for &bits in [0x7F80_0000u32, 0xFF80_0000, 0x0000_0000, 0x8000_0000].iter() {
        check(&p, "G7-special", bits as i32, 0, 0, 0);
    }
}
