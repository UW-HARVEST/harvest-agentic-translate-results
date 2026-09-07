//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every call goes through the exported symbols of BOTH shared objects loaded
//! with `libloading`; stdout is captured for each and compared byte-for-byte.
//! Randomized rows use a fixed PRNG seed so failures are reproducible.

mod common;

use common::{cstr, diff, Rng};

/// Number of randomized inputs per property-style row.
const N: usize = 400;

// ===========================================================================
// printLine — the lowest-level entry point
// ===========================================================================

/// Row 1: non-NULL, length 1..64 random printable ASCII.
#[test]
fn row01_print_line_random_printable_ascii() {
    let mut rng = Rng::new(0x01_0000_0001);
    for i in 0..N {
        let len = rng.range(1, 65);
        let bytes: Vec<u8> = (0..len).map(|_| rng.range(0x20, 0x7f) as u8).collect();
        let s = cstr(&bytes);
        diff(&format!("row01 printLine #{i} len={len}"), |a| unsafe {
            (a.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 2: non-NULL, length 0 (empty string).
#[test]
fn row02_print_line_empty_string() {
    let s = cstr(b"");
    diff("row02 printLine empty", |a| unsafe {
        (a.print_line)(s.as_ptr().cast())
    });
}

/// Row 3: non-NULL, length 1 — every possible non-NUL byte value.
#[test]
fn row03_print_line_every_single_byte() {
    for b in 1u16..=0xff {
        let s = cstr(&[b as u8]);
        diff(&format!("row03 printLine byte 0x{b:02x}"), |a| unsafe {
            (a.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 4: non-NULL, random bytes 0x01..=0xFF (invalid UTF-8, control chars).
#[test]
fn row04_print_line_random_raw_bytes() {
    let mut rng = Rng::new(0x04_0000_0004);
    for i in 0..N {
        let len = rng.range(1, 257);
        let bytes: Vec<u8> = (0..len).map(|_| rng.range(0x01, 0x100) as u8).collect();
        let s = cstr(&bytes);
        diff(&format!("row04 printLine raw #{i} len={len}"), |a| unsafe {
            (a.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 5: printf format specifiers appearing in the DATA must not be
/// interpreted (the C uses `printf("%s\n", line)`, so `line` is an argument).
#[test]
fn row05_print_line_format_specifiers_as_data() {
    let cases: &[&[u8]] = &[
        b"%d",
        b"%s",
        b"%n",
        b"%%",
        b"%p %p %p %p %p %p %p %p",
        b"%s%s%s%s%s%s%s%s",
        b"100%% done",
        b"%1$s %2$d",
        b"%.*f",
        b"%hhn %hn %n %lln",
        b"a%db%sc%nd",
        b"%",
    ];
    for c in cases {
        let s = cstr(c);
        diff(
            &format!("row05 printLine fmt {:?}", String::from_utf8_lossy(c)),
            |a| unsafe { (a.print_line)(s.as_ptr().cast()) },
        );
    }
}

/// Row 6: large strings that cross the stdio buffer boundary.
#[test]
fn row06_print_line_large_strings() {
    for &len in &[1usize, 1023, 1024, 1025, 4096, 4097, 65535, 65536, 1 << 20] {
        let bytes: Vec<u8> = (0..len).map(|i| b'A' + (i % 26) as u8).collect();
        let s = cstr(&bytes);
        diff(&format!("row06 printLine len={len}"), |a| unsafe {
            (a.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 7: embedded newlines, CR and TAB.
#[test]
fn row07_print_line_embedded_control_chars() {
    let cases: &[&[u8]] = &[
        b"\n",
        b"\n\n\n",
        b"line1\nline2",
        b"a\r\nb",
        b"\tindented",
        b"trailing\n",
        b"\x0b\x0c\x1b[31m",
        b"mix\r\n\t\x07end",
    ];
    for c in cases {
        let s = cstr(c);
        diff(&format!("row07 printLine ctrl {c:?}"), |a| unsafe {
            (a.print_line)(s.as_ptr().cast())
        });
    }
}

/// Row 8: NULL argument — the `if(line != NULL)` guard. (Also ERRORS.md row 1.)
#[test]
fn row08_print_line_null() {
    diff("row08 printLine NULL", |a| unsafe {
        (a.print_line)(std::ptr::null())
    });
}

// ===========================================================================
// printIntLine — the other lowest-level entry point
// ===========================================================================

/// Row 9: uniformly random `int` over all 2^32 bit patterns.
#[test]
fn row09_print_int_line_random_full_range() {
    let mut rng = Rng::new(0x09_0000_0009);
    for i in 0..(N * 5) {
        let v = rng.next_i32();
        diff(&format!("row09 printIntLine #{i} v={v}"), |a| unsafe {
            (a.print_int_line)(v)
        });
    }
}

/// Row 10: boundary values of the `int` range.
#[test]
fn row10_print_int_line_boundaries() {
    let mut cases = vec![0i32, 1, -1, 2, -2, 9, 10, -9, -10, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1];
    let mut p: i64 = 1;
    while p <= i32::MAX as i64 {
        cases.push(p as i32);
        cases.push(-p as i32);
        cases.push((p - 1) as i32);
        p *= 10;
    }
    for v in cases {
        diff(&format!("row10 printIntLine v={v}"), |a| unsafe {
            (a.print_int_line)(v)
        });
    }
}

// ===========================================================================
// bad — mid-level entry point, `(int)(100.0 / data)` with no guard
// ===========================================================================

/// Row 11: normal positive floats, quotient inside the `int` range.
#[test]
fn row11_bad_normal_positive_in_range() {
    let mut rng = Rng::new(0x11_0000_0011);
    for i in 0..N {
        // 100/data in range => data > ~4.66e-8; stay well inside.
        let v = rng.finite_f32(1.0e-3, 1.0e6).abs();
        diff(&format!("row11 bad #{i} data={v:e}"), |a| unsafe {
            (a.bad)(v)
        });
    }
}

/// Row 12: normal negative floats — truncation must be toward zero, not floor.
#[test]
fn row12_bad_normal_negative_truncates_toward_zero() {
    let mut rng = Rng::new(0x12_0000_0012);
    for i in 0..N {
        let v = -rng.finite_f32(1.0e-3, 1.0e6).abs();
        diff(&format!("row12 bad #{i} data={v:e}"), |a| unsafe {
            (a.bad)(v)
        });
    }
    // Hand-picked truncation witnesses (floor would give -34 / -1).
    for v in [-3.0f32, 3.0, -7.0, 7.0, -101.0, 101.0, -100.0, 100.0, -99.0, 99.0] {
        diff(&format!("row12 bad exact data={v}"), |a| unsafe {
            (a.bad)(v)
        });
    }
}

/// Row 13: |100.0/data| < 1, i.e. the result truncates to 0 (or -0 -> "0").
#[test]
fn row13_bad_quotient_truncates_to_zero() {
    let mut rng = Rng::new(0x13_0000_0013);
    for i in 0..N {
        let v = rng.finite_f32(101.0, 1.0e30);
        diff(&format!("row13 bad #{i} data={v:e}"), |a| unsafe {
            (a.bad)(v)
        });
    }
    for v in [100.000_01f32, -100.000_01, 1.0e30, -1.0e30, f32::MAX, -f32::MAX] {
        diff(&format!("row13 bad exact data={v:e}"), |a| unsafe {
            (a.bad)(v)
        });
    }
}

/// Row 14: quotient within a few ULPs of `INT_MAX + 1` (data ~= 4.6566128e-8),
/// walking both sides of the conversion boundary, both signs.
#[test]
fn row14_bad_quotient_at_int_boundary() {
    // 100.0 / 2^31 exactly:
    let pivot = (100.0f64 / 2147483648.0f64) as f32;
    let mut vals = Vec::new();
    let mut v = pivot;
    for _ in 0..24 {
        vals.push(v);
        vals.push(-v);
        v = f32::from_bits(v.to_bits() + 1);
    }
    let mut v = pivot;
    for _ in 0..24 {
        v = f32::from_bits(v.to_bits() - 1);
        vals.push(v);
        vals.push(-v);
    }
    // Also around INT_MIN's magnitude.
    for extra in [4.656_612_9e-8f32, 4.656_612_5e-8, 4.66e-8, 4.65e-8] {
        vals.push(extra);
        vals.push(-extra);
    }
    for v in vals {
        diff(
            &format!("row14 bad data={v:e} bits=0x{:08x}", v.to_bits()),
            |a| unsafe { (a.bad)(v) },
        );
    }
}

/// Row 15: quotient overflows `int` (tiny normals and subnormals).
#[test]
fn row15_bad_quotient_overflows_int() {
    let mut rng = Rng::new(0x15_0000_0015);
    for i in 0..N {
        // subnormal / very small normal bit patterns
        let bits = rng.next_u32() & 0x007f_ffff; // exponent 0 => subnormal
        let sign = (rng.next_u64() as u32 & 1) << 31;
        let v = f32::from_bits(sign | bits);
        diff(
            &format!("row15 bad subnormal #{i} bits=0x{:08x}", v.to_bits()),
            |a| unsafe { (a.bad)(v) },
        );
    }
    for v in [
        1.0e-30f32,
        -1.0e-30,
        1.0e-38,
        -1.0e-38,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0e-45,
        -1.0e-45,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
    ] {
        diff(
            &format!("row15 bad exact bits=0x{:08x}", v.to_bits()),
            |a| unsafe { (a.bad)(v) },
        );
    }
}

/// Row 16: the special float values.
#[test]
fn row16_bad_special_values() {
    let specials: &[(u32, &str)] = &[
        (0x0000_0000, "+0.0"),
        (0x8000_0000, "-0.0"),
        (0x7f80_0000, "+inf"),
        (0xff80_0000, "-inf"),
        (0x7fc0_0000, "qNaN"),
        (0xffc0_0000, "-qNaN"),
        (0x7fa0_0000, "sNaN"),
        (0xffa0_0000, "-sNaN"),
        (0x7f80_0001, "sNaN min"),
        (0x7fff_ffff, "qNaN max"),
        (0xffff_ffff, "-qNaN max"),
        (0x3f80_0000, "1.0"),
        (0xbf80_0000, "-1.0"),
    ];
    for &(bits, name) in specials {
        let v = f32::from_bits(bits);
        diff(&format!("row16 bad {name} (0x{bits:08x})"), |a| unsafe {
            (a.bad)(v)
        });
    }
}

/// Row 17: fully random `f32` bit patterns — all classes at once.
#[test]
fn row17_bad_random_bit_patterns() {
    let mut rng = Rng::new(0x17_0000_0017);
    for i in 0..(N * 5) {
        let v = rng.any_f32();
        diff(
            &format!("row17 bad #{i} bits=0x{:08x}", v.to_bits()),
            |a| unsafe { (a.bad)(v) },
        );
    }
}

// ===========================================================================
// good — exercises static goodG2B() + static goodB2G(data)
// ===========================================================================

/// Row 18: `fabs(data) > 0.000001` → the division branch.
#[test]
fn row18_good_division_branch() {
    let mut rng = Rng::new(0x18_0000_0018);
    for i in 0..N {
        let v = rng.finite_f32(1.0e-5, 1.0e20);
        diff(&format!("row18 good #{i} data={v:e}"), |a| unsafe {
            (a.good)(v)
        });
    }
}

/// Row 19: `fabs(data) <= 0.000001` → the "divide by zero" message branch.
#[test]
fn row19_good_message_branch() {
    let mut vals: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0e-7,
        -1.0e-7,
        1.0e-20,
        -1.0e-20,
        1.0e-45,
        -1.0e-45,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7fa0_0000),
    ];
    let mut rng = Rng::new(0x19_0000_0019);
    for _ in 0..N {
        // subnormals and tiny normals, always under the threshold
        let bits = rng.next_u32() & 0x007f_ffff;
        let sign = (rng.next_u64() as u32 & 1) << 31;
        vals.push(f32::from_bits(sign | bits));
    }
    for v in vals {
        diff(
            &format!("row19 good bits=0x{:08x}", v.to_bits()),
            |a| unsafe { (a.good)(v) },
        );
    }
}

/// Row 20: exactly at the `0.000001` threshold and its ULP neighbours.
#[test]
fn row20_good_threshold_boundary() {
    let base = 1.0e-6f32;
    let mut vals = Vec::new();
    let mut up = base;
    for _ in 0..40 {
        vals.push(up);
        vals.push(-up);
        up = f32::from_bits(up.to_bits() + 1);
    }
    let mut dn = base;
    for _ in 0..40 {
        dn = f32::from_bits(dn.to_bits() - 1);
        vals.push(dn);
        vals.push(-dn);
    }
    for extra in [1.000_000_1e-6f32, 9.999_99e-7, 1.000_01e-6, 9.99e-7, 1.01e-6] {
        vals.push(extra);
        vals.push(-extra);
    }
    for v in vals {
        diff(
            &format!("row20 good data={v:e} bits=0x{:08x}", v.to_bits()),
            |a| unsafe { (a.good)(v) },
        );
    }
}

/// Row 21: the constant `goodG2B()` line must be emitted BEFORE the
/// `goodB2G()` line, for every shape. The diff of the whole capture covers
/// ordering; this test additionally pins the two-line structure.
#[test]
fn row21_good_emits_g2b_line_first() {
    let mut rng = Rng::new(0x21_0000_0021);
    let mut vals: Vec<f32> = vec![0.0, -0.0, 2.0, -2.0, 1.0e-7, f32::NAN, f32::INFINITY, 1.0e-6];
    for _ in 0..N {
        vals.push(rng.any_f32());
    }
    for v in vals {
        diff(
            &format!("row21 good ordering bits=0x{:08x}", v.to_bits()),
            |a| unsafe { (a.good)(v) },
        );
        // Structural check against the C source: first line is always "50".
        let out = common::capture(|| unsafe {
            (common::api(common::Impl::C).good)(v);
        });
        assert!(
            out.starts_with(b"50\n"),
            "row21: C good() first line was not the goodG2B constant for bits 0x{:08x}: {}",
            v.to_bits(),
            common::render(&out)
        );
    }
}

// ===========================================================================
// driver — the top-level composed pipeline
// ===========================================================================

/// Row 22: good division branch x bad in-range quotient.
#[test]
fn row22_driver_good_div_x_bad_in_range() {
    let mut rng = Rng::new(0x22_0000_0022);
    for i in 0..N {
        let g = rng.finite_f32(1.0e-5, 1.0e20);
        let b = rng.finite_f32(1.0e-3, 1.0e6);
        diff(
            &format!("row22 driver #{i} good={g:e} bad={b:e}"),
            |a| unsafe { (a.driver)(g, b) },
        );
    }
}

/// Row 23: good division branch x bad invalid.
#[test]
fn row23_driver_good_div_x_bad_invalid() {
    let bads: &[f32] = &[
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        1.0e-45,
        -1.0e-45,
        f32::from_bits(1),
    ];
    let mut rng = Rng::new(0x23_0000_0023);
    for i in 0..N {
        let g = rng.finite_f32(1.0e-5, 1.0e20);
        let b = bads[rng.below(bads.len())];
        diff(
            &format!("row23 driver #{i} good={g:e} bad bits=0x{:08x}", b.to_bits()),
            |a| unsafe { (a.driver)(g, b) },
        );
    }
}

/// Row 24: good message branch x bad in-range quotient.
#[test]
fn row24_driver_good_msg_x_bad_in_range() {
    let goods: &[f32] = &[0.0, -0.0, 1.0e-7, -1.0e-7, 1.0e-6, f32::NAN, 1.0e-45, -1.0e-45];
    let mut rng = Rng::new(0x24_0000_0024);
    for i in 0..N {
        let g = goods[rng.below(goods.len())];
        let b = rng.finite_f32(1.0e-3, 1.0e6);
        diff(
            &format!("row24 driver #{i} good bits=0x{:08x} bad={b:e}", g.to_bits()),
            |a| unsafe { (a.driver)(g, b) },
        );
    }
}

/// Row 25: good message branch x bad invalid — both defect paths at once.
#[test]
fn row25_driver_good_msg_x_bad_invalid() {
    let goods: &[f32] = &[0.0, -0.0, 1.0e-7, -1.0e-7, 1.0e-6, f32::NAN, 1.0e-45];
    let bads: &[f32] = &[
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        1.0e-45,
        f32::from_bits(0x7fa0_0000),
    ];
    for &g in goods {
        for &b in bads {
            diff(
                &format!(
                    "row25 driver good=0x{:08x} bad=0x{:08x}",
                    g.to_bits(),
                    b.to_bits()
                ),
                |a| unsafe { (a.driver)(g, b) },
            );
        }
    }
}

/// Row 26: fully random `(f32, f32)` bit-pattern pairs through the whole
/// composed pipeline, including the four fixed `printLine` labels.
#[test]
fn row26_driver_random_pairs() {
    let mut rng = Rng::new(0x26_0000_0026);
    for i in 0..(N * 5) {
        let g = rng.any_f32();
        let b = rng.any_f32();
        diff(
            &format!(
                "row26 driver #{i} good=0x{:08x} bad=0x{:08x}",
                g.to_bits(),
                b.to_bits()
            ),
            |a| unsafe { (a.driver)(g, b) },
        );
    }
}

/// Row 27: many mixed calls inside ONE captured stdout session — checks the
/// interleaving/buffering of the composed pipeline rather than each wrapper in
/// isolation.
#[test]
fn row27_mixed_call_sequences() {
    let mut rng = Rng::new(0x27_0000_0027);
    for session in 0..60 {
        // Build a random program of operations, then replay it against both.
        #[derive(Clone, Copy)]
        enum Op {
            Line(usize),
            Null,
            Int(i32),
            Bad(f32),
            Good(f32),
            Driver(f32, f32),
        }
        let strings: Vec<Vec<u8>> = vec![
            cstr(b""),
            cstr(b"x"),
            cstr(b"a longer label with spaces"),
            cstr(b"%d%s"),
            cstr(b"\ttab\rcr"),
        ];
        let n_ops = rng.range(1, 40);
        let ops: Vec<Op> = (0..n_ops)
            .map(|_| match rng.below(6) {
                0 => Op::Line(rng.below(strings.len())),
                1 => Op::Null,
                2 => Op::Int(rng.next_i32()),
                3 => Op::Bad(rng.any_f32()),
                4 => Op::Good(rng.any_f32()),
                _ => Op::Driver(rng.any_f32(), rng.any_f32()),
            })
            .collect();

        diff(&format!("row27 session #{session} ({n_ops} ops)"), |a| {
            for op in &ops {
                unsafe {
                    match *op {
                        Op::Line(i) => (a.print_line)(strings[i].as_ptr().cast()),
                        Op::Null => (a.print_line)(std::ptr::null()),
                        Op::Int(v) => (a.print_int_line)(v),
                        Op::Bad(v) => (a.bad)(v),
                        Op::Good(v) => (a.good)(v),
                        Op::Driver(g, b) => (a.driver)(g, b),
                    }
                }
            }
        });
    }
}
