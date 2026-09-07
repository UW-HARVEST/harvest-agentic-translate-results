//! Phase C — error / rejection-path differential tests.
//! One test per row of `ERRORS.md`, plus the generic FFI boundary conditions.
//! Every assertion compares the *specific* value (return code / buffer bytes)
//! from both `.so`s, never merely "both failed".

mod common;

use common::{Pair, Rng};

// ---------------------------------------------------------------------------
// Harness sanity: each `Pair` really does get pristine statics. Without this,
// every other test could pass vacuously on shared state.
// ---------------------------------------------------------------------------

#[test]
fn harness_state_isolation() {
    for _ in 0..3 {
        let p = Pair::fresh();
        // accumulator starts at 0 -> add(1, 1) == 2
        assert_eq!(p.c_add(1, 1), 2, "C accumulator not pristine");
        assert_eq!(p.r_add(1, 1), 2, "Rust accumulator not pristine");
        // multiplier starts at 1 -> mul(3, 5) == 15
        assert_eq!(p.c_mul(3, 5), 15, "C multiplier not pristine");
        assert_eq!(p.r_mul(3, 5), 15, "Rust multiplier not pristine");
    }
}

// ---------------------------------------------------------------------------
// Rows 1-4 — divide_multiplier divisor guard
// ---------------------------------------------------------------------------

#[test]
fn err01_divide_by_zero_skips_division() {
    let p = Pair::fresh();
    // Seed multiplier = 1000 on both sides.
    assert_eq!(p.c_mul(100, 10), p.r_mul(100, 10));
    let c = p.c_div(0, 0);
    let r = p.r_div(0, 0);
    assert_eq!(c, r, "divide_multiplier(_, 0) return");
    assert_eq!(c, 1000, "multiplier must be unchanged by b == 0");
    // operation_count still advanced: observable via findrep's `count * 010`.
    assert_eq!(p.c_findrep(0, 0, 0, 0), p.r_findrep(0, 0, 0, 0));
}

#[test]
fn err02_divide_by_one_is_noop() {
    let p = Pair::fresh();
    assert_eq!(p.c_mul(7, 13), p.r_mul(7, 13)); // multiplier = 91
    let c = p.c_div(0, 1);
    let r = p.r_div(0, 1);
    assert_eq!(c, r);
    assert_eq!(c, 91);
}

#[test]
fn err03_divide_by_negative_truncates_toward_zero() {
    let p = Pair::fresh();
    // multiplier == 1 initially; 1 / -3 == 0 in C (truncation, not floor).
    let c = p.c_div(0, -3);
    let r = p.r_div(0, -3);
    assert_eq!(c, r, "divide_multiplier(_, -3) from multiplier == 1");
    assert_eq!(c, 0, "C truncates toward zero, so 1 / -3 == 0");

    // Larger magnitudes, several negative divisors, fresh state each time.
    for b in [-2, -3, -7, -100, -511, -1000, i32::MIN + 1] {
        let p = Pair::fresh();
        assert_eq!(p.c_mul(1234, 5), p.r_mul(1234, 5)); // multiplier = 6170
        assert_eq!(
            p.c_div(0, b),
            p.r_div(0, b),
            "divide_multiplier(_, {b}) with multiplier == 6170"
        );
    }
    // And negative multiplier / negative divisor.
    for b in [-2, -3, -7, -100] {
        let p = Pair::fresh();
        assert_eq!(p.c_mul(-1234, 5), p.r_mul(-1234, 5)); // multiplier = -6170
        assert_eq!(
            p.c_div(0, b),
            p.r_div(0, b),
            "divide_multiplier(_, {b}) with multiplier == -6170"
        );
    }
}

#[test]
fn err04_divide_by_int_min() {
    let p = Pair::fresh();
    // multiplier == 1; 1 / INT_MIN == 0.
    assert_eq!(p.c_div(0, i32::MIN), p.r_div(0, i32::MIN));
    let p = Pair::fresh();
    assert_eq!(p.c_mul(60_000, 30_000), p.r_mul(60_000, 30_000)); // wraps
    assert_eq!(
        p.c_div(0, i32::MIN),
        p.r_div(0, i32::MIN),
        "divide by INT_MIN after overflowing multiplier"
    );
}

// ---------------------------------------------------------------------------
// Rows 5-13 — validate_and_normalize clamp rejections
// ---------------------------------------------------------------------------

#[test]
fn err05_validate_zero_is_not_clamped() {
    let p = Pair::fresh();
    let (c, r) = (p.c_validate(0), p.r_validate(0));
    assert_eq!(c, r);
    assert_eq!(c, 0, "zero must pass through, NOT be raised to 0100");
}

#[test]
fn err06_validate_negatives_are_not_clamped() {
    let p = Pair::fresh();
    for v in [-1, -2, -63, -64, -65, -511, -512, -100_000] {
        let (c, r) = (p.c_validate(v), p.r_validate(v));
        assert_eq!(c, r, "validate_and_normalize({v})");
        assert_eq!(c, v, "negative {v} must pass through unclamped");
    }
}

#[test]
fn err07_validate_int_min() {
    let p = Pair::fresh();
    let (c, r) = (p.c_validate(i32::MIN), p.r_validate(i32::MIN));
    assert_eq!(c, r);
    assert_eq!(c, i32::MIN);
}

#[test]
fn err08_validate_below_lower_threshold() {
    let p = Pair::fresh();
    for v in 1..64 {
        let (c, r) = (p.c_validate(v), p.r_validate(v));
        assert_eq!(c, r, "validate_and_normalize({v})");
        assert_eq!(c, 64, "0 < {v} < 0100 must clamp up to 64");
    }
}

#[test]
fn err09_validate_one_below_lower_bound() {
    let p = Pair::fresh();
    let (c, r) = (p.c_validate(63), p.r_validate(63));
    assert_eq!(c, r);
    assert_eq!(c, 64);
}

#[test]
fn err10_validate_exactly_lower_bound() {
    let p = Pair::fresh();
    let (c, r) = (p.c_validate(64), p.r_validate(64));
    assert_eq!(c, r);
    assert_eq!(c, 64, "0100 itself is in range (`<` is strict)");
}

#[test]
fn err11_validate_exactly_upper_bound() {
    let p = Pair::fresh();
    let (c, r) = (p.c_validate(511), p.r_validate(511));
    assert_eq!(c, r);
    assert_eq!(c, 511, "0777 itself is in range (`>` is strict)");
}

#[test]
fn err12_validate_one_past_upper_bound() {
    let p = Pair::fresh();
    let (c, r) = (p.c_validate(512), p.r_validate(512));
    assert_eq!(c, r);
    assert_eq!(c, 511);
}

#[test]
fn err13_validate_int_max() {
    let p = Pair::fresh();
    for v in [513, 1024, 1_000_000, i32::MAX - 1, i32::MAX] {
        let (c, r) = (p.c_validate(v), p.r_validate(v));
        assert_eq!(c, r, "validate_and_normalize({v})");
        assert_eq!(c, 511, "oversized {v} must clamp down to 511");
    }
}

// ---------------------------------------------------------------------------
// Rows 14-19 — find_and_replace_char rejections
// ---------------------------------------------------------------------------

#[test]
fn err14_needle_absent_leaves_string_untouched() {
    let p = Pair::fresh();
    let s = b"abcdefghij";
    let c = p.c_find_replace(s, b'z' as i32);
    let r = p.r_find_replace(s, b'z' as i32);
    assert_eq!(c, r);
    assert_eq!(&c[..s.len()], s, "string must be unmodified");
    assert_eq!(c[s.len()], 0, "terminator preserved");
}

#[test]
fn err15_empty_string_zero_length() {
    let p = Pair::fresh();
    for needle in [0, 1, b'X' as i32, b'a' as i32, -1, 255, 256, 1000] {
        let c = p.c_find_replace(b"", needle);
        let r = p.r_find_replace(b"", needle);
        assert_eq!(c, r, "empty string, needle {needle}");
        assert_eq!(c[0], 0, "empty string must stay empty");
    }
}

#[test]
fn err16_nul_needle_never_matches() {
    let p = Pair::fresh();
    let s = b"hello";
    let c = p.c_find_replace(s, 0);
    let r = p.r_find_replace(s, 0);
    assert_eq!(c, r);
    assert_eq!(&c[..s.len()], s, "NUL is outside the strlen span");
    assert_eq!(c[s.len()], 0, "terminator must not become 'X'");
}

#[test]
fn err17_needle_above_255_truncates_to_unsigned_char() {
    let p = Pair::fresh();
    let s = b"zzAzz";
    // 0x141 == 321; (unsigned char)321 == 0x41 == 'A'
    let c = p.c_find_replace(s, 0x141);
    let r = p.r_find_replace(s, 0x141);
    assert_eq!(c, r, "needle 0x141 must behave like 'A'");
    assert_eq!(&c[..5], b"zzXzz", "the 'A' must have been replaced");

    // Every multiple-of-256 offset for a range of needles.
    for base in [b'a' as i32, b'z' as i32, b'0' as i32, 0x20] {
        for k in 1..5 {
            let n = base + 256 * k;
            let s2 = b"a mix: az0 ~!";
            assert_eq!(
                p.c_find_replace(s2, n),
                p.r_find_replace(s2, n),
                "needle {n} (base {base}, +{k}*256)"
            );
        }
    }
}

#[test]
fn err18_negative_needle_truncates_to_unsigned_char() {
    let p = Pair::fresh();
    let s = b"zzAzz";
    // -191 as unsigned char == 256 - 191 == 65 == 'A'
    let c = p.c_find_replace(s, -191);
    let r = p.r_find_replace(s, -191);
    assert_eq!(c, r, "needle -191 must behave like 'A'");
    assert_eq!(&c[..5], b"zzXzz");

    for n in [-1, -2, -65, -128, -191, -255, -256, -257, -1000, i32::MIN] {
        let s2 = b"boundary \x7f~ test";
        assert_eq!(
            p.c_find_replace(s2, n),
            p.r_find_replace(s2, n),
            "negative needle {n}"
        );
    }
}

#[test]
fn err19_needle_equals_replacement_char() {
    let p = Pair::fresh();
    let s = b"abXcdXe";
    let c = p.c_find_replace(s, b'X' as i32);
    let r = p.r_find_replace(s, b'X' as i32);
    assert_eq!(c, r);
    assert_eq!(&c[..s.len()], s, "'X' -> 'X' leaves the value unchanged");
}

// ---------------------------------------------------------------------------
// Rows 20-25 — findrep branch rejections
// ---------------------------------------------------------------------------

#[test]
fn err20_findrep_no_active_params_skips_add_and_multiply() {
    let p = Pair::fresh();
    let c = p.c_findrep(0, 0, 0, 0);
    let r = p.r_findrep(0, 0, 0, 0);
    assert_eq!(c, r, "findrep(0,0,0,0)");
    // Neither operations[0] nor [1] ran, so operation_count is still 0 and
    // accumulator is still 0 -> `both_active` false. Confirm through a
    // follow-up low-level call: accumulator must still be pristine.
    assert_eq!(p.c_add(1, 1), 2, "C accumulator untouched by findrep(0,0,0,0)");
    assert_eq!(p.r_add(1, 1), 2, "Rust accumulator untouched");
}

#[test]
fn err21_findrep_one_active_param_skips_multiply() {
    for v in [1i32, -1, 64, 511, 512, i32::MIN, i32::MAX] {
        for slot in 0..4 {
            let p = Pair::fresh();
            let mut a = [0i32; 4];
            a[slot] = v;
            assert_eq!(
                p.c_findrep(a[0], a[1], a[2], a[3]),
                p.r_findrep(a[0], a[1], a[2], a[3]),
                "findrep{a:?} (active_params == 1)"
            );
        }
    }
}

#[test]
fn err22_findrep_subtract_branch_not_taken() {
    // add(64, 0) -> accumulator = 64, which is <= 0150 (104).
    let p = Pair::fresh();
    let c = p.c_findrep(1, 0, 0, 0);
    let r = p.r_findrep(1, 0, 0, 0);
    assert_eq!(c, r);
    // operation_count == 1 (only add ran), accumulator == 64: the subtract
    // branch was skipped. Verify accumulator is exactly 64 on both sides.
    assert_eq!(p.c_sub(64, 0), 0, "C accumulator was exactly 64");
    assert_eq!(p.r_sub(64, 0), 0, "Rust accumulator was exactly 64");
}

#[test]
fn err23_findrep_both_active_false() {
    // multiplier forced to 0 -> `both_active` false regardless of accumulator.
    let p = Pair::fresh();
    assert_eq!(p.c_mul(0, 0), 0);
    assert_eq!(p.r_mul(0, 0), 0);
    for args in [(0, 0, 0, 0), (1, 1, 1, 1), (600, 600, 600, 600), (-5, 0, 0, 0)] {
        let (a, b, c, d) = args;
        assert_eq!(
            p.c_findrep(a, b, c, d),
            p.r_findrep(a, b, c, d),
            "findrep{args:?} with multiplier == 0"
        );
    }
}

#[test]
fn err24_findrep_divide_branch_not_taken() {
    // multiplier stays at 1 (<= 0100) when the multiply branch never fires,
    // so operations[3] must not run.
    let p = Pair::fresh();
    let c = p.c_findrep(5, 0, 0, 0);
    let r = p.r_findrep(5, 0, 0, 0);
    assert_eq!(c, r);
    // multiplier must be exactly 1 on both sides.
    assert_eq!(p.c_mul(1, 1), 1, "C multiplier was exactly 1");
    assert_eq!(p.r_mul(1, 1), 1, "Rust multiplier was exactly 1");
}

#[test]
fn err25_findrep_zero_result_becomes_sentinel_0777() {
    // Derivation of an input that makes the computed `result` exactly 0.
    //
    // With all four params 0: active_params == 0, so neither operations[0] nor
    // operations[1] runs. `result` is then
    //     9                        (memchr offset of 'p' in the search buffer)
    //   + accumulator + multiplier (if both_active)
    //   + operation_count * 010
    // Pre-seeding with a single `add_to_accumulator(-18, 0)` gives
    // accumulator = -18 (so the `> 0150` subtract branch stays off),
    // multiplier = 1, operation_count = 1:
    //     9 + (-18 + 1) + 1*8 == 0   ->  substituted with 0777 == 511.
    let p = Pair::fresh();
    assert_eq!(p.c_add(-18, 0), p.r_add(-18, 0));
    let cv = p.c_findrep(0, 0, 0, 0);
    let rv = p.r_findrep(0, 0, 0, 0);
    assert_eq!(cv, rv, "findrep(0,0,0,0) after add(-18,0)");
    assert_eq!(
        cv, 0o777,
        "computed result was 0, so the 0777 sentinel must be returned"
    );

    // Same branch reached by a different route: two seeding ops
    // (operation_count == 2) need accumulator == -26.
    let p = Pair::fresh();
    assert_eq!(p.c_add(-10, 0), p.r_add(-10, 0));
    assert_eq!(p.c_add(-16, 0), p.r_add(-16, 0));
    let cv = p.c_findrep(0, 0, 0, 0);
    let rv = p.r_findrep(0, 0, 0, 0);
    assert_eq!(cv, rv, "findrep(0,0,0,0) after add(-10,0), add(-16,0)");
    assert_eq!(cv, 0o777, "second route to the sentinel");

    // A third route that also drives the multiplier away from 1:
    // mul(4, 4) -> multiplier = 16, count = 1; add(A, 0) -> count = 2.
    // 9 + (A + 16) + 2*8 == 0  ->  A == -41.
    let p = Pair::fresh();
    assert_eq!(p.c_mul(4, 4), p.r_mul(4, 4));
    assert_eq!(p.c_add(-41, 0), p.r_add(-41, 0));
    let cv = p.c_findrep(0, 0, 0, 0);
    let rv = p.r_findrep(0, 0, 0, 0);
    assert_eq!(cv, rv, "findrep(0,0,0,0) after mul(4,4), add(-41,0)");
    assert_eq!(cv, 0o777, "third route to the sentinel");

    // Systematic scan over negative accumulator seeds: every value must agree,
    // and the sentinel must be observed at least once.
    let mut sentinel_hits = 0;
    for a in -200..=0i32 {
        let p = Pair::fresh();
        assert_eq!(p.c_add(a, 0), p.r_add(a, 0), "seed add({a},0)");
        let cv = p.c_findrep(0, 0, 0, 0);
        let rv = p.r_findrep(0, 0, 0, 0);
        assert_eq!(cv, rv, "scan: findrep(0,0,0,0) after add({a},0)");
        if cv == 0o777 {
            sentinel_hits += 1;
        }
    }
    assert!(
        sentinel_hits > 0,
        "the 0777 sentinel branch was never exercised by the scan"
    );

    // Broad randomized coverage of the same comparison.
    let mut rng = Rng::new(0xF00D_5EED);
    for i in 0..4_000 {
        let p = Pair::fresh();
        let (a, b, c, d) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        assert_eq!(
            p.c_findrep(a, b, c, d),
            p.r_findrep(a, b, c, d),
            "err25 iter {i}: findrep({a}, {b}, {c}, {d})"
        );
    }
    eprintln!("err25: 0777 sentinel branch exercised {sentinel_hits} times");
}

// ---------------------------------------------------------------------------
// Rows 26-28 — process_octal_string formatting edge cases
// ---------------------------------------------------------------------------

#[test]
fn err26_octal_zero() {
    let p = Pair::fresh();
    let c = p.c_octal(0);
    let r = p.r_octal(0);
    assert_eq!(c, r);
    let nul = c.iter().position(|&b| b == 0).unwrap();
    assert_eq!(&c[..nul], b"Octal: 00, Decimal: 0");
}

#[test]
fn err27_octal_negative_prints_as_unsigned() {
    let p = Pair::fresh();
    let c = p.c_octal(-1);
    let r = p.r_octal(-1);
    assert_eq!(c, r);
    let nul = c.iter().position(|&b| b == 0).unwrap();
    assert_eq!(&c[..nul], b"Octal: 037777777777, Decimal: -1");

    for v in [-2, -7, -8, -64, -511, -512, -1_000_000, i32::MIN + 1] {
        assert_eq!(p.c_octal(v), p.r_octal(v), "process_octal_string({v})");
    }
}

#[test]
fn err28_octal_int_min_widest_output() {
    let p = Pair::fresh();
    let c = p.c_octal(i32::MIN);
    let r = p.r_octal(i32::MIN);
    assert_eq!(c, r);
    let nul = c.iter().position(|&b| b == 0).unwrap();
    assert_eq!(&c[..nul], b"Octal: 020000000000, Decimal: -2147483648");
    assert_eq!(nul, 41, "widest output is 41 bytes, still inside buffer[50]");

    let c = p.c_octal(i32::MAX);
    let r = p.r_octal(i32::MAX);
    assert_eq!(c, r);
    let nul = c.iter().position(|&b| b == 0).unwrap();
    assert_eq!(&c[..nul], b"Octal: 017777777777, Decimal: 2147483647");
}

// ---------------------------------------------------------------------------
// Generic FFI boundary sweep (out-of-domain ints across the boundary).
// The C API declares no enum type, so "out-of-range enum value" degenerates to
// "any int the caller can pass"; sweep the full width on every int parameter.
// ---------------------------------------------------------------------------

#[test]
fn generic_out_of_domain_int_sweep() {
    let extremes: [i32; 14] = [
        i32::MIN,
        i32::MIN + 1,
        -2_000_000_000,
        -65_537,
        -256,
        -1,
        0,
        1,
        255,
        256,
        65_536,
        2_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];

    // validate_and_normalize + process_octal_string: stateless / write-only.
    let p = Pair::fresh();
    for &v in &extremes {
        assert_eq!(p.c_validate(v), p.r_validate(v), "validate({v})");
        assert_eq!(p.c_octal(v), p.r_octal(v), "octal({v})");
        assert_eq!(
            p.c_find_replace(b"probe string", v),
            p.r_find_replace(b"probe string", v),
            "find_replace(_, {v})"
        );
    }

    // The stateful pairs, each on a pristine instance.
    for &a in &extremes {
        for &b in &extremes {
            let p = Pair::fresh();
            assert_eq!(p.c_add(a, b), p.r_add(a, b), "add({a},{b})");
            let p = Pair::fresh();
            assert_eq!(p.c_mul(a, b), p.r_mul(a, b), "mul({a},{b})");
            let p = Pair::fresh();
            assert_eq!(p.c_sub(a, b), p.r_sub(a, b), "sub({a},{b})");
            // Skip C's UB case: multiplier is 1 here so INT_MIN/-1 cannot
            // occur, but b == 0 is the guard row and is included.
            let p = Pair::fresh();
            assert_eq!(p.c_div(a, b), p.r_div(a, b), "div({a},{b})");
        }
    }

    // findrep with extremes in each slot, other slots at a fixed non-zero.
    for slot in 0..4 {
        for &v in &extremes {
            let p = Pair::fresh();
            let mut args = [7i32, 7, 7, 7];
            args[slot] = v;
            assert_eq!(
                p.c_findrep(args[0], args[1], args[2], args[3]),
                p.r_findrep(args[0], args[1], args[2], args[3]),
                "findrep{args:?}"
            );
        }
    }
}

#[test]
fn generic_zero_and_oversized_lengths() {
    let p = Pair::fresh();
    // Zero length.
    assert_eq!(p.c_find_replace(b"", b'a' as i32), p.r_find_replace(b"", b'a' as i32));
    // One byte.
    for ch in [b'a', b'X', 0x7f, 0x20] {
        let s = [ch];
        for needle in [ch as i32, b'q' as i32, 0, ch as i32 + 256] {
            assert_eq!(
                p.c_find_replace(&s, needle),
                p.r_find_replace(&s, needle),
                "1-byte {ch:#x}, needle {needle}"
            );
        }
    }
    // Long string right up against the harness buffer.
    let long: Vec<u8> = (0..120).map(|i| b'a' + (i % 26) as u8).collect();
    assert_eq!(
        p.c_find_replace(&long, b'z' as i32),
        p.r_find_replace(&long, b'z' as i32),
        "120-byte string"
    );
    assert_eq!(
        p.c_find_replace(&long, b'!' as i32),
        p.r_find_replace(&long, b'!' as i32),
        "120-byte string, absent needle"
    );
}

#[test]
fn generic_all_byte_values_as_needle() {
    let p = Pair::fresh();
    // Haystack containing every non-NUL byte value exactly once.
    let hay: Vec<u8> = (1u16..=255).map(|b| b as u8).collect();
    for needle in 0..=255i32 {
        assert_eq!(
            p.c_find_replace(&hay, needle),
            p.r_find_replace(&hay, needle),
            "full-byte haystack, needle {needle}"
        );
    }
    // And the same needles offset by +256 / -256.
    for needle in 0..=255i32 {
        assert_eq!(
            p.c_find_replace(&hay, needle + 256),
            p.r_find_replace(&hay, needle + 256),
            "needle {needle}+256"
        );
        assert_eq!(
            p.c_find_replace(&hay, needle - 256),
            p.r_find_replace(&hay, needle - 256),
            "needle {needle}-256"
        );
    }
}
