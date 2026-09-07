//! Phase C — error/rejection-path differential tests, one per `ERRORS.md` row.
//!
//! The public ABI is `int memchra2(int,int,int,int)` — no pointers, no lengths,
//! no enums — so the internal rejection paths of the eight `static` helpers are
//! witnessed through the single `int` return value. Each test constructs the
//! input class that drives the guard in question and asserts the C and Rust
//! `.so` exports agree exactly (same value, not merely "both failed").
//!
//! For guards that are statically unreachable from `memchra2` (the helper is
//! always called with a non-NULL stack buffer and a literal non-zero length),
//! the test instead pins down the OBSERVABLE consequence of the guard NOT
//! firing, over randomized inputs — which is the only externally visible
//! evidence an FFI caller can obtain, and which would break immediately if the
//! Rust mirrored the guard differently (e.g. returned `-1` instead of a sum,
//! or `0` instead of `-1`).

mod common;
use common::{check, pair, Rng, EXTREMES};

/// Reference model of the exact contribution of each guarded helper, so the
/// rows below can assert the *specific* value/sentinel, not just C == Rust.
fn expected(a: i32, b: i32, c: i32, d: i32) -> i32 {
    let s = format!("test{}-{}-{}-{}", a, b, c, d);
    let buf = s.as_bytes();
    assert!(buf.len() < 64, "snprintf would truncate: {} bytes", buf.len());

    let mut result: i32 = 0;

    // count_occurrences(buffer, '-') -> memchra
    let dashes = buf.iter().filter(|&&x| x == b'-').count() as i32;
    result = result.wrapping_add(dashes.wrapping_mul(10));

    // safe_sum_array(values, 4)
    let sum = [a, b, c, d]
        .iter()
        .fold(0i32, |acc, &v| acc.wrapping_add(v));
    result = result.wrapping_add(sum);

    // process_strings(..., 4, "test") == 3
    result = result.wrapping_add(3 * 5);

    // int_to_float_bits(a)
    let f = f32::from_bits(a as u32);
    if f > 0.0 && f < 1000.0 {
        result = result.wrapping_add(f as i32);
    }

    // process_buffer(buffer, strlen(buffer))
    let buf_sum = buf.iter().fold(0i32, |acc, &x| acc.wrapping_add(x as i8 as i32));
    if buf_sum > 0 {
        result = result.wrapping_add(buf_sum % 256);
    }

    // interpret_as_int({b,c,d,0}, 4) little-endian
    let interpreted =
        i32::from_le_bytes([(b & 0xFF) as u8, (c & 0xFF) as u8, (d & 0xFF) as u8, 0]);
    result ^= interpreted;

    // complex_iteration(values, 4)
    let complex = [a, b, c, d]
        .iter()
        .fold(0i32, |acc, &v| acc ^ ((v as u32 & 0xFF) as i32));
    result.wrapping_add(complex)
}

fn c_of(a: i32, b: i32, c: i32, d: i32) -> i32 {
    unsafe { (pair().c)(a, b, c, d) }
}
fn rs_of(a: i32, b: i32, c: i32, d: i32) -> i32 {
    unsafe { (pair().rs)(a, b, c, d) }
}

#[track_caller]
fn check_all(a: i32, b: i32, c: i32, d: i32) {
    check(a, b, c, d);
    assert_eq!(
        c_of(a, b, c, d),
        expected(a, b, c, d),
        "reference model disagrees with C at ({a},{b},{c},{d})"
    );
}

/// Rows 1-4 — `process_buffer` guards: NULL buffer (-1), empty buffer (-1),
/// `len == 0` (0, suppressing the caller's `+=`), and early NUL break.
///
/// From `memchra2` the buffer is always the on-stack `buffer[64]` holding at
/// least `"test"`, and `len == strlen(buffer)`, so NONE of the -1 / 0 paths can
/// fire: `buf_sum` is always a positive sum of printable ASCII and
/// `result += buf_sum % 256` always happens. The assertion below pins exactly
/// that: had Rust returned the `-1` or `0` sentinel, the model would diverge.
#[test]
fn row01_04_process_buffer_guards() {
    let mut r = Rng::new(0x0101);
    for _ in 0..20_000 {
        let (a, b, c, d) = (r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
        check_all(a, b, c, d);
        // The guard's TRUE-path contribution must be a positive ASCII sum.
        let s = format!("test{}-{}-{}-{}", a, b, c, d);
        let buf_sum: i32 = s.bytes().map(|x| x as i8 as i32).sum();
        assert!(buf_sum > 0, "buf_sum must be > 0 for {s:?}");
    }
    for &v in EXTREMES {
        check_all(v, v, v, v);
        check_all(v, 0, 0, 0);
    }
}

/// Rows 5-9 — `process_strings` guards: NULL array (0), `count <= 0` (0),
/// NULL element (skip), empty element (skip), non-matching prefix (not counted).
///
/// `memchra2` always passes the 4-entry literal table and `count == 4`, so the
/// result is invariably `matches == 3` (`"other"` rejected by `strncmp`), i.e. a
/// constant `+15`. Verified against C for every input below.
#[test]
fn row05_09_process_strings_guards() {
    let mut r = Rng::new(0x0501);
    for _ in 0..20_000 {
        check_all(r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
    // Independent confirmation that the constant really is 3 matches (+15):
    // C(a,b,c,d) - model_without_strings == 15 for a sample of inputs.
    for &(a, b, c, d) in &[(1, 2, 3, 4), (0, 0, 0, 0), (-1, -1, -1, -1), (i32::MIN, i32::MAX, 0, 1)]
    {
        let with = expected(a, b, c, d);
        assert_eq!(c_of(a, b, c, d), with);
        assert_eq!(rs_of(a, b, c, d), with);
    }
}

/// Rows 10-11 — `safe_sum_array` guards: NULL array (0) and `size == 0` (0).
/// Always called with `values[4]`, `size = 4`; the sum (with wraparound) must
/// always be added.
#[test]
fn row10_11_safe_sum_array_guards() {
    // Inputs whose sum is 0, positive-overflowing, negative-overflowing.
    check_all(0, 0, 0, 0);
    check_all(1, -1, 2, -2);
    check_all(i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    check_all(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    check_all(i32::MAX, 1, 0, 0);
    check_all(i32::MIN, -1, 0, 0);
    let mut r = Rng::new(0x0A01);
    for _ in 0..20_000 {
        check_all(r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
}

/// Rows 12-13 — `interpret_as_int` guards: NULL bytes (0) and
/// `len < sizeof(int)` (0). Always called with `bytes[4]`, `len = 4`, so the
/// little-endian reinterpretation always happens and is XORed in.
#[test]
fn row12_13_interpret_as_int_guards() {
    // low bytes all zero -> interpreted == 0 -> `result ^= 0` (same value the
    // *rejected* path would have produced; distinguished by the rows above).
    check_all(0, 0, 0, 0);
    check_all(7, 256, 512, 768);
    // maximal interpreted value
    check_all(7, 0xFF, 0xFF, 0xFF);
    // one byte at a time
    for i in 0..3 {
        for v in [1i32, 0x7F, 0x80, 0xFF] {
            let mut args = [0i32; 3];
            args[i] = v;
            check_all(3, args[0], args[1], args[2]);
        }
    }
    let mut r = Rng::new(0x0C01);
    for _ in 0..20_000 {
        check_all(r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
}

/// Rows 14-15, 18 — `count_occurrences` guards: NULL text (0), empty text (0);
/// and `memchra` with `n == 0` (0). `buffer` always starts with `'t'` and
/// `strlen >= 4`, so `dash_count` is always `3 + #negatives` and
/// `result += dash_count * 10` always happens.
#[test]
fn row14_15_18_count_occurrences_guards() {
    // 0..4 negative arguments => dash_count 3..7
    let combos: [(i32, i32, i32, i32); 5] = [
        (1, 2, 3, 4),
        (-1, 2, 3, 4),
        (-1, -2, 3, 4),
        (-1, -2, -3, 4),
        (-1, -2, -3, -4),
    ];
    for &(a, b, c, d) in &combos {
        check_all(a, b, c, d);
        let dashes = format!("test{}-{}-{}-{}", a, b, c, d)
            .bytes()
            .filter(|&x| x == b'-')
            .count();
        assert!((3..=7).contains(&dashes));
    }
    // Every sign pattern of the four arguments.
    let mut r = Rng::new(0x0E01);
    for mask in 0u32..16 {
        for _ in 0..1_500 {
            let mut v = [0i32; 4];
            for (i, slot) in v.iter_mut().enumerate() {
                let m = r.i32_in(1, 2_000_000_000);
                *slot = if mask >> i & 1 == 1 { -m } else { m };
            }
            check_all(v[0], v[1], v[2], v[3]);
        }
    }
}

/// Rows 16-17 — `complex_iteration` guards: NULL data and `count == 0` both
/// return `-1`, which the caller then ADDS to `result`. Always called with
/// `values[4]`/`count = 4`, so the XOR fold (never `-1`) is what is added.
#[test]
fn row16_17_complex_iteration_guards() {
    // XOR fold == 0 (all equal) — distinguishable from the -1 sentinel.
    let mut r = Rng::new(0x1001);
    for _ in 0..10_000 {
        let v = r.next_i32();
        check_all(v, v, v, v);
    }
    // XOR fold == 0xFF (max)
    check_all(0xFF, 0, 0, 0);
    check_all(0, 0, 0, 0xFF);
    for _ in 0..10_000 {
        check_all(r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
}

/// Row 19 — the `if (f > 0.0f && f < 1000.0f)` REJECTION path, in every way it
/// can be reached. This is the only *reachable-from-the-ABI* rejection.
#[test]
fn row19_float_guard_rejections() {
    let rejected: [(i32, &str); 12] = [
        (0, "+0.0 — `f > 0` false"),
        (i32::MIN, "-0.0 — `f > 0` false"),
        (-1, "NaN (0xFFFFFFFF)"),
        (0x8000_0001u32 as i32, "negative subnormal"),
        ((-1.0f32).to_bits() as i32, "-1.0"),
        ((-1234.5f32).to_bits() as i32, "-1234.5"),
        (1000.0f32.to_bits() as i32, "exactly 1000.0 — `f < 1000` false"),
        (1000.000_1f32.to_bits() as i32, "just above 1000.0"),
        (f32::MAX.to_bits() as i32, "f32::MAX"),
        (f32::INFINITY.to_bits() as i32, "+inf"),
        (f32::NEG_INFINITY.to_bits() as i32, "-inf"),
        (0x7FC0_0000u32 as i32, "quiet NaN"),
    ];
    let mut r = Rng::new(0x1301);
    for &(a, why) in &rejected {
        let f = f32::from_bits(a as u32);
        assert!(!(f > 0.0 && f < 1000.0), "{why}: {f} should be rejected");
        check_all(a, 1, 2, 3);
        check_all(a, 0, 0, 0);
        check_all(a, -1, -1, -1);
        for _ in 0..500 {
            check_all(a, r.next_i32(), r.next_i32(), r.next_i32());
        }
    }
    // And the mirror ACCEPT boundary, one step apart.
    let just_below_1000 = (1000.0f32.to_bits() - 1) as i32;
    assert!(f32::from_bits(just_below_1000 as u32) < 1000.0);
    check_all(just_below_1000, 1, 2, 3);
    let smallest_positive = 1i32; // subnormal, `f > 0` TRUE
    assert!(f32::from_bits(smallest_positive as u32) > 0.0);
    check_all(smallest_positive, 1, 2, 3);
}

/// Row 20 — the `if (buf_sum > 0)` rejection. Unreachable (printable ASCII sum
/// is always positive) — asserted, and the TRUE branch verified everywhere.
#[test]
fn row20_buf_sum_guard() {
    let mut r = Rng::new(0x1401);
    for _ in 0..20_000 {
        let (a, b, c, d) = (r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
        let s = format!("test{}-{}-{}-{}", a, b, c, d);
        let buf_sum: i32 = s.bytes().map(|x| x as i8 as i32).sum();
        assert!(buf_sum > 0);
        assert!(s.bytes().all(|x| x.is_ascii_graphic()));
        check_all(a, b, c, d);
    }
}

/// Row 21 — the `snprintf` truncation boundary. Maximum output is 51 bytes
/// (`INT_MIN` four times) which is < 63, so truncation cannot occur; asserted
/// and probed at the maximum.
#[test]
fn row21_snprintf_truncation_boundary() {
    let worst = format!("test{}-{}-{}-{}", i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    assert_eq!(worst.len(), 51);
    assert!(worst.len() < 64, "would truncate");
    check_all(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    for &a in &[i32::MIN, i32::MIN + 1, i32::MAX, -1_000_000_000] {
        for &b in &[i32::MIN, i32::MAX, 1_000_000_000] {
            check_all(a, b, a, b);
            check_all(b, a, b, a);
        }
    }
}

/// Generic FFI-boundary cases: extreme and one-past-boundary `int` values in
/// every position. (No pointer/length/enum parameters exist in this ABI.)
#[test]
fn errors_boundary_ints() {
    let bounds = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in &bounds {
        for &b in &bounds {
            for &c in &bounds {
                for &d in &bounds {
                    check_all(a, b, c, d);
                }
            }
        }
    }
}

/// Generic: inputs engineered to overflow the internal signed accumulations.
#[test]
fn errors_overflow_wrap() {
    let big = [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX / 2 + 1,
        1 << 30,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -(1 << 30),
    ];
    for &a in &big {
        for &b in &big {
            for &c in &big {
                for &d in &big {
                    check_all(a, b, c, d);
                }
            }
        }
    }
}

/// Generic: the full IEEE-754 class taxonomy for the `int`->`float` type pun.
#[test]
fn errors_float_pun_classes() {
    let mut cases: Vec<i32> = vec![
        0,
        i32::MIN,                              // -0.0
        1,                                     // smallest positive subnormal
        0x007F_FFFF,                           // largest subnormal
        0x0080_0000,                           // smallest normal
        0x3F7F_FFFF,                           // largest < 1.0
        0x3F80_0000,                           // 1.0
        0x447A_0000,                           // 1000.0
        0x447A_0000 - 1,                       // largest < 1000.0
        0x7F7F_FFFF,                           // f32::MAX
        0x7F80_0000,                           // +inf
        0x7F80_0001,                           // sNaN
        0x7FC0_0000,                           // qNaN
        0x7FFF_FFFF,                           // NaN
    ];
    // Negative counterparts.
    let neg: Vec<i32> = cases.iter().map(|&v| (v as u32 | 0x8000_0000) as i32).collect();
    cases.extend(neg);

    let mut r = Rng::new(0x1F01);
    for &a in &cases {
        check_all(a, 0, 0, 0);
        check_all(a, 1, 2, 3);
        check_all(a, -1, -2, -3);
        check_all(a, i32::MIN, i32::MAX, 0);
        for _ in 0..400 {
            check_all(a, r.next_i32(), r.next_i32(), r.next_i32());
        }
    }
}
