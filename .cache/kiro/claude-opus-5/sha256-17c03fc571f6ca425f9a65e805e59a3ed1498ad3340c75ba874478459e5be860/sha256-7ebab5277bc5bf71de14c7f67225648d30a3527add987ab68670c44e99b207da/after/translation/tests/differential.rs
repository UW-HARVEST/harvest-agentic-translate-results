//! Differential tests: every call goes through `dlopen`/`dlsym` on both the C
//! reference `.so` and the Rust cdylib.
//!
//! `memchra2` is the only symbol either library exports; all the helper
//! functions in `c_src/src/lib.c` are `static`. The tests below are ordered
//! bottom-up by the helper they exercise, choosing inputs that isolate each
//! one's branches, then broaden to full randomized fuzzing.

mod common;

use common::{Harness, Rng, EDGE_VALUES};

// ---------------------------------------------------------------------------
// Level 0: the exported symbol exists and is callable in both libraries.
// ---------------------------------------------------------------------------

#[test]
fn loads_both_libraries_and_resolves_memchra2() {
    let h = Harness::load();
    // Resolving the symbols happens in `load()`; a single call proves both are
    // callable through the FFI boundary.
    let c = h.c_memchra2(0, 0, 0, 0);
    let r = h.rust_memchra2(0, 0, 0, 0);
    assert_eq!(c, r, "memchra2(0,0,0,0) mismatch");
}

// ---------------------------------------------------------------------------
// Level 1: `memchra` / `count_occurrences`.
//
// The dash count of the formatted buffer "test%d-%d-%d-%d" contributes
// `dash_count * 10`. There are always 3 separator dashes plus one per negative
// argument, so sign combinations drive this helper.
// ---------------------------------------------------------------------------

#[test]
fn memchra_dash_counting_over_all_sign_combinations() {
    let h = Harness::load();
    let magnitudes = [0i32, 1, 7, 42, 999, 123_456, i32::MAX];
    for &m in &magnitudes {
        for mask in 0u8..16 {
            let pick = |bit: u8| -> i32 {
                if mask & (1 << bit) != 0 {
                    m.wrapping_neg()
                } else {
                    m
                }
            };
            h.assert_match(pick(0), pick(1), pick(2), pick(3));
        }
    }
    // i32::MIN formats as "-2147483648" and cannot be negated; check it
    // separately in every argument position.
    for pos in 0..4 {
        let mut args = [1i32, 2, 3, 4];
        args[pos] = i32::MIN;
        h.assert_match(args[0], args[1], args[2], args[3]);
    }
}

// ---------------------------------------------------------------------------
// Level 2: `process_buffer`.
//
// Sums the signed `char` values of the formatted buffer up to the NUL, then
// `result += buf_sum % 256`. Digit content and buffer length drive it, so
// sweep decimal widths.
// ---------------------------------------------------------------------------

#[test]
fn process_buffer_across_decimal_widths() {
    let h = Harness::load();
    // One value per decimal width, positive and negative.
    let mut widths: Vec<i32> = Vec::new();
    let mut v: i64 = 1;
    while v <= 1_000_000_000 {
        widths.push(v as i32);
        widths.push(-(v as i32));
        widths.push((v * 9).min(i32::MAX as i64) as i32);
        v *= 10;
    }
    widths.push(0);
    widths.push(i32::MAX);
    widths.push(i32::MIN);

    for &a in &widths {
        for &b in &widths {
            h.assert_match(a, b, 0, 0);
            h.assert_match(0, 0, a, b);
            h.assert_match(a, b, b, a);
        }
    }
}

#[test]
fn process_buffer_longest_possible_formatted_string() {
    let h = Harness::load();
    // "test" + four 11-char values + 3 dashes = 51 bytes, the longest the
    // 64-byte snprintf buffer ever sees (so it never truncates).
    h.assert_match(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    h.assert_match(i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    h.assert_match(i32::MIN, i32::MAX, i32::MIN, i32::MAX);
}

// ---------------------------------------------------------------------------
// Level 3: `int_to_float_bits`.
//
// `a`'s bit pattern is reinterpreted as a float; if it lands strictly inside
// (0.0, 1000.0) the truncated value is added to the result.
// ---------------------------------------------------------------------------

#[test]
fn int_to_float_bits_in_range_window() {
    let h = Harness::load();
    // Sweep floats across the whole accepted window, including values whose
    // truncation is non-trivial.
    let mut probes: Vec<i32> = Vec::new();
    for f in [
        f32::from_bits(1),        // ~1.4e-45 subnormal
        1e-30,
        1e-10,
        0.0001,
        0.5,
        0.999_999_9,
        1.0,
        1.000_000_1,
        1.5,
        2.0,
        2.999_999_8,
        3.0,
        127.5,
        255.999_98,
        256.0,
        512.25,
        999.0,
        999.999_94,
        1000.0,
        1000.000_1,
        1024.0,
        f32::MAX,
        f32::INFINITY,
        -0.0,
        -1.0,
        -1000.0,
        f32::NEG_INFINITY,
        f32::NAN,
    ] {
        probes.push(f.to_bits() as i32);
    }
    // Exact boundary neighbourhoods in bit space.
    for base in [0.0f32.to_bits(), 1000.0f32.to_bits()] {
        for delta in -4i64..=4 {
            probes.push((base as i64 + delta) as u32 as i32);
        }
    }

    for &a in &probes {
        h.assert_match(a, 0, 0, 0);
        h.assert_match(a, 1, -2, 3);
        h.assert_match(a, i32::MIN, i32::MAX, -1);
    }
}

#[test]
fn int_to_float_bits_exponent_sweep() {
    let h = Harness::load();
    // Walk every biased exponent with a few mantissa patterns, both signs.
    for exp in 0u32..=255 {
        for mant in [0u32, 1, 0x0040_0000, 0x007F_FFFF] {
            for sign in [0u32, 1] {
                let bits = (sign << 31) | (exp << 23) | mant;
                h.assert_match(bits as i32, 5, -6, 7);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 4: `safe_sum_array` (wrapping `int` accumulation of a,b,c,d).
// ---------------------------------------------------------------------------

#[test]
fn safe_sum_array_overflow_patterns() {
    let h = Harness::load();
    let cases: &[[i32; 4]] = &[
        [i32::MAX, 1, 0, 0],
        [i32::MAX, i32::MAX, 0, 0],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, -1, 0, 0],
        [i32::MIN, i32::MIN, 0, 0],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, i32::MIN, i32::MAX, i32::MIN],
        [1_073_741_824, 1_073_741_824, 1_073_741_824, 1_073_741_824],
        [-1_073_741_824, -1_073_741_824, -1_073_741_824, -1_073_741_824],
    ];
    for c in cases {
        h.assert_match(c[0], c[1], c[2], c[3]);
    }
}

// ---------------------------------------------------------------------------
// Level 5: `interpret_as_int`.
//
// bytes = [b & 0xFF, c & 0xFF, d & 0xFF, 0] reinterpreted as a little-endian
// `int`, XORed into the result. Sweep the low bytes of b, c and d.
// ---------------------------------------------------------------------------

#[test]
fn interpret_as_int_low_byte_sweep() {
    let h = Harness::load();
    // Full sweep of one byte position at a time, plus high-bit-set carriers to
    // confirm only the low byte is used.
    for byte in 0i32..256 {
        for carrier in [0i32, 0x0100, -0x0100, 0x7FFF_FF00, i32::MIN] {
            let v = carrier | byte;
            h.assert_match(0, v, 0, 0);
            h.assert_match(0, 0, v, 0);
            h.assert_match(0, 0, 0, v);
            h.assert_match(0, v, v, v);
        }
    }
}

#[test]
fn interpret_as_int_combined_bytes() {
    let h = Harness::load();
    let bytes = [0i32, 1, 0x7F, 0x80, 0xFE, 0xFF];
    for &b in &bytes {
        for &c in &bytes {
            for &d in &bytes {
                h.assert_match(0, b, c, d);
                h.assert_match(0x3F80_0000, b, c, d);
                h.assert_match(-1, b, c, d);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 6: `complex_iteration` (XOR of the low bytes of a,b,c,d, or -1 when
// count == 0 — unreachable here since count is always 4).
// ---------------------------------------------------------------------------

#[test]
fn complex_iteration_low_byte_xor() {
    let h = Harness::load();
    let vals = [0i32, 0x01, 0xFF, 0x1234_5601, -1, i32::MIN, i32::MAX];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    h.assert_match(a, b, c, d);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Level 7: the full `memchra2` pipeline.
// ---------------------------------------------------------------------------

#[test]
fn memchra2_edge_value_cross_product() {
    let h = Harness::load();
    // Full cross product would be 39^4; pair up positions instead and rotate
    // so every edge value visits every argument slot.
    for (i, &x) in EDGE_VALUES.iter().enumerate() {
        for &y in EDGE_VALUES {
            h.assert_match(x, y, EDGE_VALUES[i % EDGE_VALUES.len()], y);
            h.assert_match(y, x, y, x);
            h.assert_match(y, y, x, x);
            h.assert_match(x, x, x, y);
        }
    }
}

#[test]
fn memchra2_fuzz_full_range() {
    let h = Harness::load();
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    for _ in 0..200_000 {
        let a = rng.next_i32();
        let b = rng.next_i32();
        let c = rng.next_i32();
        let d = rng.next_i32();
        h.assert_match(a, b, c, d);
    }
}

#[test]
fn memchra2_fuzz_biased_distributions() {
    let h = Harness::load();
    let mut rng = Rng::new(0x0123_4567_89AB_CDEF);
    for _ in 0..200_000 {
        let a = rng.next_interesting_i32();
        let b = rng.next_interesting_i32();
        let c = rng.next_interesting_i32();
        let d = rng.next_interesting_i32();
        h.assert_match(a, b, c, d);
    }
}

#[test]
fn memchra2_exhaustive_small_neighbourhood() {
    let h = Harness::load();
    // Exhaustive over a small signed window in all four arguments: 11^4.
    for a in -5i32..=5 {
        for b in -5i32..=5 {
            for c in -5i32..=5 {
                for d in -5i32..=5 {
                    h.assert_match(a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn memchra2_is_deterministic_and_side_effect_free() {
    let h = Harness::load();
    // Repeated calls must be stable, and interleaving the two libraries must
    // not change either one's answer (no shared/global state).
    let cases: &[[i32; 4]] = &[
        [0, 0, 0, 0],
        [1, 2, 3, 4],
        [-1, -2, -3, -4],
        [0x3F80_0000, 255, 128, 1],
        [i32::MIN, i32::MAX, i32::MIN, i32::MAX],
    ];
    for c in cases {
        let first_c = h.c_memchra2(c[0], c[1], c[2], c[3]);
        let first_r = h.rust_memchra2(c[0], c[1], c[2], c[3]);
        for _ in 0..50 {
            assert_eq!(h.c_memchra2(c[0], c[1], c[2], c[3]), first_c);
            assert_eq!(h.rust_memchra2(c[0], c[1], c[2], c[3]), first_r);
        }
        assert_eq!(first_c, first_r, "mismatch for {c:?}");
    }
}
