// Phase B -- valid-path differential tests for `overunder`, the composed
// pipeline (CONFIGS.md rows 26..=45). Every case compares BOTH the returned
// `int` AND the byte-exact stdout the library produced.
mod common;
use common::*;

const N: usize = 300;

/// Pick an `a` whose C remainder `a % 6` equals `want` (which may be negative).
fn pick_a_with_residue(r: &mut Rng, want: i32) -> i32 {
    loop {
        let a = r.range_i32(-1_000_000, 1_000_000);
        if a.wrapping_rem(6) == want {
            return a;
        }
    }
}

fn residue_row(row: &str, want: i32, seed: u64) {
    let mut r = Rng::new(seed);
    for _ in 0..N {
        let a = pick_a_with_residue(&mut r, want);
        let b = r.range_i32(-100_000, 100_000);
        let c = r.range_i32(-100_000, 100_000);
        let d = r.range_i32(-40_000, 40_000);
        cmp_overunder(row, a, b, c, d);
    }
    // also with large b/c/d so clamping interacts with this switch path
    for _ in 0..N {
        let a = pick_a_with_residue(&mut r, want);
        cmp_overunder(row, a, r.next_i32(), r.next_i32(), r.next_i32());
    }
}

// --- rows 26..32: each switch path reached through `a % 6` -----------------

#[test]
fn cfg_26_ou_residue_5() {
    residue_row("cfg26", 5, SEED ^ 26);
}

#[test]
fn cfg_27_ou_residue_4() {
    residue_row("cfg27", 4, SEED ^ 27);
}

#[test]
fn cfg_28_ou_residue_3() {
    residue_row("cfg28", 3, SEED ^ 28);
}

#[test]
fn cfg_29_ou_residue_2() {
    residue_row("cfg29", 2, SEED ^ 29);
}

#[test]
fn cfg_30_ou_residue_1() {
    residue_row("cfg30", 1, SEED ^ 30);
}

#[test]
fn cfg_31_ou_residue_0() {
    residue_row("cfg31", 0, SEED ^ 31);
}

#[test]
fn cfg_32_ou_negative_residue_hits_default() {
    for want in [-1, -2, -3, -4, -5] {
        residue_row("cfg32", want, SEED ^ 0x3200u64.wrapping_add(want as u64));
    }
}

// --- rows 33..36: magnitude classes ---------------------------------------

#[test]
fn cfg_33_ou_all_zero() {
    cmp_overunder("cfg33", 0, 0, 0, 0);
}

#[test]
fn cfg_34_ou_small_magnitudes() {
    let mut r = Rng::new(SEED ^ 34);
    for _ in 0..(N * 6) {
        cmp_overunder(
            "cfg34",
            r.range_i32(-100, 100),
            r.range_i32(-100, 100),
            r.range_i32(-100, 100),
            r.range_i32(-100, 100),
        );
    }
    // exhaustive-ish tiny sweep
    for a in -3..=3 {
        for b in -3..=3 {
            for c in -3..=3 {
                for d in -3..=3 {
                    cmp_overunder("cfg34", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn cfg_35_ou_medium_no_sqrt_overflow() {
    // |a|,|d| <= 46340 keeps d*d + a*a within int => a real (non-NaN) sqrt.
    let mut r = Rng::new(SEED ^ 35);
    for _ in 0..(N * 6) {
        let a = r.range_i32(-32767, 32767);
        let d = r.range_i32(-32767, 32767);
        assert!(d.checked_mul(d).and_then(|x| x.checked_add(a * a)).is_some());
        cmp_overunder("cfg35", a, r.range_i32(-46340, 46340), r.range_i32(-46340, 46340), d);
    }
    // right at the non-overflow edge
    for &v in &[46340, -46340, 46341, -46341, 32768, 65535, 65536] {
        cmp_overunder("cfg35", v, 1, 1, 0);
        cmp_overunder("cfg35", 0, 1, 1, v);
        cmp_overunder("cfg35", v, 1, 1, v);
    }
}

#[test]
fn cfg_36_ou_large_magnitudes() {
    let mut r = Rng::new(SEED ^ 36);
    for _ in 0..(N * 6) {
        cmp_overunder("cfg36", r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
}

// --- rows 37..40: one parameter at an extreme, the rest random ------------

fn extreme_param(row: &str, which: usize, seed: u64) {
    let mut r = Rng::new(seed);
    let extremes = [
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN / 2,
        i32::MAX / 2,
        1 << 30,
        -(1 << 30),
    ];
    for &e in &extremes {
        for _ in 0..N {
            let mut p = [r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32()];
            p[which] = e;
            cmp_overunder(row, p[0], p[1], p[2], p[3]);
        }
        for _ in 0..N {
            let mut p = [
                r.range_i32(-1000, 1000),
                r.range_i32(-1000, 1000),
                r.range_i32(-1000, 1000),
                r.range_i32(-1000, 1000),
            ];
            p[which] = e;
            cmp_overunder(row, p[0], p[1], p[2], p[3]);
        }
    }
}

#[test]
fn cfg_37_ou_a_extreme() {
    extreme_param("cfg37", 0, SEED ^ 37);
}

#[test]
fn cfg_38_ou_b_extreme() {
    extreme_param("cfg38", 1, SEED ^ 38);
}

#[test]
fn cfg_39_ou_c_extreme() {
    extreme_param("cfg39", 2, SEED ^ 39);
}

#[test]
fn cfg_40_ou_d_extreme() {
    extreme_param("cfg40", 3, SEED ^ 40);
}

// --- row 41: full corner cross-product ------------------------------------

#[test]
fn cfg_41_ou_corner_cross_product() {
    let corners = [i32::MIN, -1, 0, 1, i32::MAX];
    for &a in &corners {
        for &b in &corners {
            for &c in &corners {
                for &d in &corners {
                    cmp_overunder("cfg41", a, b, c, d);
                }
            }
        }
    }
}

// --- row 42: powers of two swept through each parameter -------------------

#[test]
fn cfg_42_ou_powers_of_two() {
    let mut pows: Vec<i32> = Vec::new();
    for k in 0..31 {
        pows.push(1i32 << k);
        pows.push(-(1i32 << k));
        pows.push((1i32 << k) - 1);
    }
    pows.push(i32::MIN);
    for &v in &pows {
        cmp_overunder("cfg42", v, 3, 5, 7);
        cmp_overunder("cfg42", 3, v, 5, 7);
        cmp_overunder("cfg42", 3, 5, v, 7);
        cmp_overunder("cfg42", 3, 5, 7, v);
        cmp_overunder("cfg42", v, v, v, v);
    }
}

// --- row 43: rounding-sensitive values for `a*1.5`, `b*2.7`, `%.2f` -------

#[test]
fn cfg_43_ou_rounding_sensitive() {
    // a*1.5 is exactly x.5 when a is odd, exactly integral when a is even.
    // b*2.7 exercises binary/decimal rounding in both the cast and "%.2f".
    let mut vals: Vec<i32> = Vec::new();
    for a in -40..=40 {
        vals.push(a);
    }
    for base in [1_000_001i32, 999_999, 2_000_001, 1 << 23, (1 << 23) + 1, 1 << 24, (1 << 24) + 1] {
        vals.push(base);
        vals.push(-base);
        vals.push(base + 1);
        vals.push(-base - 1);
    }
    for &a in &vals {
        for &b in &vals {
            if (a.wrapping_abs() % 7) != (b.wrapping_abs() % 7) {
                continue; // prune the cross-product but keep it varied
            }
            cmp_overunder("cfg43", a, b, 3, 5);
        }
    }
    // c/3.3 rounding: values around multiples of 3.3
    let mut r = Rng::new(SEED ^ 43);
    for _ in 0..(N * 4) {
        let k = r.range_i32(-100_000, 100_000);
        for delta in -1..=1 {
            cmp_overunder("cfg43", 1, 1, k.wrapping_mul(33).wrapping_add(delta), 1);
        }
    }
}

// --- row 44: high-volume fuzz (return values; stdout on a sample) ---------

#[test]
fn cfg_44_ou_high_volume_fuzz() {
    let mut r = Rng::new(SEED ^ 44);
    for i in 0..20_000u32 {
        let (a, b, c, d) = (r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
        if i % 16 == 0 {
            cmp_overunder("cfg44", a, b, c, d);
        } else {
            cmp_overunder_ret("cfg44", a, b, c, d);
        }
    }
    // mixed magnitude classes
    for i in 0..20_000u32 {
        let pick = |r: &mut Rng| match r.next_u64() % 5 {
            0 => 0,
            1 => r.range_i32(-10, 10),
            2 => r.range_i32(-46340, 46340),
            3 => r.next_i32(),
            _ => {
                if r.next_u64() % 2 == 0 {
                    i32::MIN
                } else {
                    i32::MAX
                }
            }
        };
        let (a, b, c, d) = (pick(&mut r), pick(&mut r), pick(&mut r), pick(&mut r));
        if i % 16 == 0 {
            cmp_overunder("cfg44", a, b, c, d);
        } else {
            cmp_overunder_ret("cfg44", a, b, c, d);
        }
    }
}

// --- row 45: cross-check the COMPOSITION against the low-level calls ------
//
// Recompute `overunder`'s return value out of the library's OWN low-level
// exports (per library), so a bug in how the Rust composes the pipeline shows
// up even if each individual wrapper is correct.
#[test]
fn cfg_45_ou_composition_matches_lowlevel_exports() {
    let l = libs();
    let mut r = Rng::new(SEED ^ 45);

    let mut cases: Vec<(i32, i32, i32, i32)> = vec![
        (0, 0, 0, 0),
        (7, 11, 13, 17),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MAX, i32::MIN, i32::MAX),
        (-6, 5, -7, 8),
    ];
    for _ in 0..3000 {
        cases.push((r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32()));
    }
    for _ in 0..3000 {
        cases.push((
            r.range_i32(-5000, 5000),
            r.range_i32(-5000, 5000),
            r.range_i32(-5000, 5000),
            r.range_i32(-5000, 5000),
        ));
    }

    for (a, b, c, d) in cases {
        for lib in [&l.c, &l.rs] {
            let sdti = lib.safe_double_to_int;
            let pwf = lib.process_with_fallthrough;
            let hpo = lib.handle_pointer_operations;

            let temp1 = a as f64 * 1.5;
            let temp2 = b as f64 * 2.7;
            let temp3 = c as f64 / 3.3;
            let temp4 = (d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a)) as f64).sqrt();

            let (conv1, conv2, conv3, conv4) = unsafe {
                (sdti(temp1), sdti(temp2), sdti(temp3), sdti(temp4))
            };
            let switch_result = unsafe { pwf(a.wrapping_rem(6), b) };
            let ptr_result = unsafe { hpo(c) };

            // The DataBlock round-trip through the library's own memcpy.
            let mut src = DataBlock::zeroed();
            src.id = a;
            src.value = temp1;
            src.label[..6].copy_from_slice(&[
                b'S' as _, b'o' as _, b'u' as _, b'r' as _, b'c' as _, b'e' as _,
            ]);
            let mut dst = DataBlock::from_bytes(&[0xAA; DATABLOCK_SIZE]);
            unsafe { (lib.copy_data_block)(&mut dst, &src) };

            let mut expect = conv1
                .wrapping_add(conv2)
                .wrapping_add(conv3)
                .wrapping_add(conv4)
                .wrapping_add(switch_result)
                .wrapping_add(ptr_result)
                .wrapping_add(dst.id);
            for v in [a, b, c, d, a.wrapping_add(b)] {
                expect = expect.wrapping_add(v);
            }

            let (got, _) = capture_stdout(|| unsafe { (lib.overunder)(a, b, c, d) });
            assert_eq!(
                got, expect,
                "[cfg45/{}] overunder({a},{b},{c},{d}) = {got} but composing that \
                 library's own low-level exports gives {expect}",
                lib.name
            );
        }
    }
}
