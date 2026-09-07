//! Phase B — valid-path differential tests. One test per row of `CONFIGS.md`.
//!
//! Both implementations are driven exclusively through their `.so` exports.

mod harness;

use harness::{buf_from, same, same_bytes, Pair, Rng, EDGE_I32};

const N: usize = 400; // randomized inputs per row

// ===========================================================================
// validate_and_normalize — rows 1..6
// ===========================================================================

fn check_normalize(tag: &str, values: impl IntoIterator<Item = i32>) {
    let p = Pair::fresh();
    for v in values {
        same(
            &format!("{tag}: validate_and_normalize({v})"),
            p.c.normalize(v),
            p.rust.normalize(v),
        );
    }
}

#[test]
fn cfg_01_normalize_bucket_z() {
    check_normalize("row1/Z", [0]);
}

#[test]
fn cfg_02_normalize_bucket_n() {
    let mut r = Rng::new(0x0201);
    let mut vals = vec![-1, -2, -63, -64, -511, -512, i32::MIN, i32::MIN + 1];
    for _ in 0..N {
        vals.push(r.i32_in(i32::MIN, -1));
    }
    check_normalize("row2/N", vals);
}

#[test]
fn cfg_03_normalize_bucket_l() {
    check_normalize("row3/L", 1..=63); // exhaustive
}

#[test]
fn cfg_04_normalize_bucket_p() {
    check_normalize("row4/P", 64..=511); // exhaustive
}

#[test]
fn cfg_05_normalize_bucket_u() {
    let mut r = Rng::new(0x0205);
    let mut vals = vec![512, 513, 1000, i32::MAX - 1, i32::MAX];
    for _ in 0..N {
        vals.push(r.i32_in(512, i32::MAX));
    }
    check_normalize("row5/U", vals);
}

#[test]
fn cfg_06_normalize_random_sweep() {
    let mut r = Rng::new(0x0206);
    let mut vals: Vec<i32> = EDGE_I32.to_vec();
    for _ in 0..(N * 10) {
        vals.push(r.i32_any());
    }
    check_normalize("row6/sweep", vals);
}

// ===========================================================================
// add_to_accumulator / subtract_from_accumulator — rows 7..11
// ===========================================================================

#[test]
fn cfg_07_add_single_fresh() {
    // One call per FRESH library so `accumulator` starts at 0 every time.
    let mut r = Rng::new(0x0207);
    let mut cases: Vec<(i32, i32)> = Vec::new();
    for &a in EDGE_I32.iter().take(12) {
        for &b in EDGE_I32.iter().rev().take(6) {
            cases.push((a, b));
        }
    }
    for _ in 0..60 {
        cases.push((r.i32_any(), r.i32_any()));
    }
    for (a, b) in cases {
        let p = Pair::fresh();
        same(
            &format!("row7: add({a},{b}) on fresh state"),
            p.c.add(a, b),
            p.rust.add(a, b),
        );
        // The follow-up findrep observes the state the single call left behind.
        same(
            &format!("row7: findrep after add({a},{b})"),
            p.c.findrep(1, 1, 1, 1),
            p.rust.findrep(1, 1, 1, 1),
        );
    }
}

#[test]
fn cfg_08_add_sequence() {
    let p = Pair::fresh();
    let mut r = Rng::new(0x0208);
    for i in 0..(N * 5) {
        let (a, b) = if i % 4 == 0 {
            (r.i32_in(-200, 200), r.i32_in(-200, 200))
        } else {
            (r.i32_any(), r.i32_any())
        };
        same(
            &format!("row8: add #{i} ({a},{b})"),
            p.c.add(a, b),
            p.rust.add(a, b),
        );
    }
}

#[test]
fn cfg_09_sub_single_fresh() {
    let mut r = Rng::new(0x0209);
    for _ in 0..80 {
        let (a, b) = (r.i32_any(), r.i32_any());
        let p = Pair::fresh();
        same(
            &format!("row9: sub({a},{b}) fresh"),
            p.c.sub(a, b),
            p.rust.sub(a, b),
        );
    }
    for &a in EDGE_I32 {
        for &b in &[i32::MIN, -1, 0, 1, i32::MAX] {
            let p = Pair::fresh();
            same(
                &format!("row9: sub({a},{b}) fresh/edge"),
                p.c.sub(a, b),
                p.rust.sub(a, b),
            );
        }
    }
}

#[test]
fn cfg_10_sub_sequence() {
    let p = Pair::fresh();
    let mut r = Rng::new(0x0210);
    for i in 0..(N * 5) {
        let (a, b) = (r.i32_any(), r.i32_any());
        same(
            &format!("row10: sub #{i} ({a},{b})"),
            p.c.sub(a, b),
            p.rust.sub(a, b),
        );
    }
}

#[test]
fn cfg_11_add_sub_interleaved() {
    let p = Pair::fresh();
    let mut r = Rng::new(0x0211);
    for i in 0..(N * 5) {
        let (a, b) = (r.i32_any(), r.i32_any());
        let (cv, rv) = if r.bool() {
            (p.c.add(a, b), p.rust.add(a, b))
        } else {
            (p.c.sub(a, b), p.rust.sub(a, b))
        };
        same(&format!("row11: interleaved #{i} ({a},{b})"), cv, rv);
    }
}

// ===========================================================================
// multiply_with_multiplier / divide_multiplier — rows 12..18
// ===========================================================================

#[test]
fn cfg_12_mul_single_fresh() {
    let mut r = Rng::new(0x0212);
    for _ in 0..80 {
        let (a, b) = (r.i32_any(), r.i32_any());
        let p = Pair::fresh();
        same(
            &format!("row12: mul({a},{b}) fresh"),
            p.c.mul(a, b),
            p.rust.mul(a, b),
        );
    }
    for &a in EDGE_I32 {
        for &b in &[i32::MIN, -2, -1, 0, 1, 2, i32::MAX] {
            let p = Pair::fresh();
            same(
                &format!("row12: mul({a},{b}) fresh/edge"),
                p.c.mul(a, b),
                p.rust.mul(a, b),
            );
        }
    }
}

#[test]
fn cfg_13_mul_sequence() {
    let p = Pair::fresh();
    let mut r = Rng::new(0x0213);
    for i in 0..(N * 5) {
        let (a, b) = (r.i32_any(), r.i32_any());
        same(
            &format!("row13: mul #{i} ({a},{b})"),
            p.c.mul(a, b),
            p.rust.mul(a, b),
        );
    }
}

#[test]
fn cfg_14_mul_small_operands() {
    // Small operands keep `multiplier` from immediately exploding, so the
    // `multiplier > 0100` guard inside findrep toggles both ways.
    let p = Pair::fresh();
    let mut r = Rng::new(0x0214);
    for i in 0..(N * 2) {
        let (a, b) = (r.i32_in(-3, 3), r.i32_in(-3, 3));
        same(
            &format!("row14: mul #{i} ({a},{b})"),
            p.c.mul(a, b),
            p.rust.mul(a, b),
        );
        same(
            &format!("row14: findrep after mul #{i}"),
            p.c.findrep(1, 2, 3, 4),
            p.rust.findrep(1, 2, 3, 4),
        );
    }
}

#[test]
fn cfg_15_div_single_fresh() {
    let mut r = Rng::new(0x0215);
    // `a` is ignored by the C; vary it anyway to prove that.
    for _ in 0..80 {
        let a = r.i32_any();
        let mut b = r.i32_any();
        if b == 0 {
            b = 1;
        }
        let p = Pair::fresh();
        same(
            &format!("row15: div({a},{b}) fresh"),
            p.c.div(a, b),
            p.rust.div(a, b),
        );
    }
    for &b in EDGE_I32 {
        if b == 0 {
            continue;
        }
        let p = Pair::fresh();
        same(
            &format!("row15: div(_,{b}) fresh/edge"),
            p.c.div(12345, b),
            p.rust.div(12345, b),
        );
    }
}

#[test]
fn cfg_16_div_sequence() {
    let p = Pair::fresh();
    let mut r = Rng::new(0x0216);
    for i in 0..(N * 5) {
        let a = r.i32_any();
        // Never (INT_MIN / -1): that traps in C (see ERRORS.md row U4).
        let mut b = r.i32_in(-1000, 1000);
        if b == 0 {
            b = 3;
        }
        same(
            &format!("row16: div #{i} (_,{b}) a={a}"),
            p.c.div(a, b),
            p.rust.div(a, b),
        );
    }
}

#[test]
fn cfg_17_mul_div_interleaved() {
    let p = Pair::fresh();
    let mut r = Rng::new(0x0217);
    for i in 0..(N * 5) {
        let (cv, rv) = if r.bool() {
            let (a, b) = (r.i32_in(-50, 50), r.i32_in(-50, 50));
            (p.c.mul(a, b), p.rust.mul(a, b))
        } else {
            let mut b = r.i32_in(-20, 20);
            if b == 0 {
                b = 7;
            }
            (p.c.div(0, b), p.rust.div(0, b))
        };
        same(&format!("row17: mul/div interleaved #{i}"), cv, rv);
    }
}

#[test]
fn cfg_18_all_ops_interleaved() {
    // The exact composed pipeline `findrep` builds, driven at the lowest level:
    // random interleaving of all four `operation_func` targets.
    let p = Pair::fresh();
    let mut r = Rng::new(0x0218);
    for i in 0..(N * 8) {
        let a = if r.bool() { r.i32_any() } else { r.i32_in(-64, 64) };
        let b = if r.bool() { r.i32_any() } else { r.i32_in(-64, 64) };
        let (cv, rv) = match r.next_u32() % 4 {
            0 => (p.c.add(a, b), p.rust.add(a, b)),
            1 => {
                let b = r.i32_in(-4, 4);
                (p.c.mul(a, b), p.rust.mul(a, b))
            }
            2 => (p.c.sub(a, b), p.rust.sub(a, b)),
            _ => {
                let mut d = r.i32_in(-100, 100);
                if d == 0 {
                    d = 0; // exercise the b == 0 guard on the valid path too
                }
                (p.c.div(a, d), p.rust.div(a, d))
            }
        };
        same(&format!("row18: op #{i} (a={a},b={b})"), cv, rv);
        if i % 16 == 0 {
            same(
                &format!("row18: findrep probe #{i}"),
                p.c.findrep(a, b, 1, 0),
                p.rust.findrep(a, b, 1, 0),
            );
        }
    }
}

// ===========================================================================
// process_octal_string — rows 19..22
// ===========================================================================

fn check_octal(tag: &str, values: impl IntoIterator<Item = i32>) {
    let p = Pair::fresh();
    for v in values {
        same_bytes(
            &format!("{tag}: process_octal_string(_, {v})"),
            &p.c.octal(v),
            &p.rust.octal(v),
        );
    }
}

#[test]
fn cfg_19_octal_small_values() {
    check_octal("row19", [0, 1, 7, 8, 0o123, 0o777, 0o150, 63, 64, 511, 512]);
}

#[test]
fn cfg_20_octal_random_positive() {
    let mut r = Rng::new(0x0220);
    check_octal("row20", (0..N * 5).map(|_| r.i32_in(0, i32::MAX)));
}

#[test]
fn cfg_21_octal_random_negative() {
    let mut r = Rng::new(0x0221);
    check_octal("row21", (0..N * 5).map(|_| r.i32_in(i32::MIN, -1)));
}

#[test]
fn cfg_22_octal_length_boundaries() {
    let mut vals: Vec<i32> = vec![i32::MIN, i32::MIN + 1, -1, 0, i32::MAX, i32::MAX - 1];
    // Every point where the octal digit count changes, from both sides.
    for shift in 0..31 {
        let v = 1i32 << shift;
        vals.push(v);
        vals.push(v - 1);
        vals.push(v.wrapping_neg());
        vals.push(v.wrapping_neg().wrapping_sub(1));
    }
    vals.extend(EDGE_I32.iter().copied());
    check_octal("row22", vals);
}

// ===========================================================================
// find_and_replace_char — rows 23..28
// ===========================================================================

fn check_replace(tag: &str, cases: &[(Vec<u8>, i32)]) {
    let p = Pair::fresh();
    for (buf, ch) in cases {
        same_bytes(
            &format!(
                "{tag}: find_and_replace_char({:?}, {ch})",
                String::from_utf8_lossy(buf)
            ),
            &p.c.replace(buf, *ch),
            &p.rust.replace(buf, *ch),
        );
    }
}

#[test]
fn cfg_23_replace_at_start() {
    let cases = vec![
        (buf_from(b"Octal: 0123", 64), b'O' as i32),
        (buf_from(b"aaaa", 16), b'a' as i32),
        (buf_from(b"z", 8), b'z' as i32),
    ];
    check_replace("row23", &cases);
}

#[test]
fn cfg_24_replace_at_end() {
    let cases = vec![
        (buf_from(b"hello world", 64), b'd' as i32),
        (buf_from(b"abcQ", 16), b'Q' as i32),
        (buf_from(b"Function pointer example with static vars", 100), b's' as i32),
    ];
    check_replace("row24", &cases);
}

#[test]
fn cfg_25_replace_middle_and_findrep_case() {
    let cases = vec![
        (buf_from(b"Octal: 0123, Decimal: 83", 64), b'O' as i32),
        (buf_from(b"Octal: 00, Decimal: 0", 64), b'O' as i32),
        (buf_from(b"Function pointer example with static vars", 100), b'p' as i32),
        (buf_from(b"abcdefghij", 32), b'e' as i32),
    ];
    check_replace("row25", &cases);
}

#[test]
fn cfg_26_replace_single_byte() {
    let mut cases = Vec::new();
    for b in 1u16..=255 {
        cases.push((buf_from(&[b as u8], 8), b as i32)); // match
        cases.push((buf_from(&[b as u8], 8), (b as i32) ^ 1)); // usually no match
    }
    check_replace("row26", &cases);
}

#[test]
fn cfg_27_replace_randomized() {
    let mut r = Rng::new(0x0227);
    let mut cases = Vec::new();
    for _ in 0..(N * 5) {
        let len = (r.next_u32() % 24) as usize;
        let s: Vec<u8> = (0..len)
            .map(|_| {
                let b = r.byte();
                if b == 0 { 1 } else { b }
            })
            .collect();
        let ch = if r.bool() {
            // Bias toward bytes that are actually present.
            if s.is_empty() {
                r.i32_any()
            } else {
                s[(r.next_u32() as usize) % s.len()] as i32
            }
        } else {
            r.i32_any()
        };
        cases.push((buf_from(&s, 32), ch));
    }
    check_replace("row27", &cases);
}

#[test]
fn cfg_28_replace_high_bit_bytes() {
    let mut cases = Vec::new();
    for b in 0x80u16..=0xFF {
        let s = vec![b'a', b as u8, b'z', b as u8];
        // Same byte reached as a positive int, as a sign-extended negative
        // int, and offset by 256 -- memchr narrows to unsigned char.
        cases.push((buf_from(&s, 24), b as i32));
        cases.push((buf_from(&s, 24), (b as u8 as i8) as i32));
        cases.push((buf_from(&s, 24), b as i32 + 256));
        cases.push((buf_from(&s, 24), b as i32 - 256));
    }
    check_replace("row28", &cases);
}

// ===========================================================================
// findrep — rows 29..44
// ===========================================================================

/// Run `findrep` once on a FRESH pair of libraries and compare.
#[track_caller]
fn findrep_fresh(tag: &str, a: i32, b: i32, c: i32, d: i32) {
    let p = Pair::fresh();
    same(
        &format!("{tag}: findrep({a},{b},{c},{d}) fresh"),
        p.c.findrep(a, b, c, d),
        p.rust.findrep(a, b, c, d),
    );
}

#[test]
fn cfg_29_findrep_active_0() {
    findrep_fresh("row29", 0, 0, 0, 0);
    // ...and again on the same library, twice, since state now differs.
    let p = Pair::fresh();
    for i in 0..5 {
        same(
            &format!("row29: repeat #{i}"),
            p.c.findrep(0, 0, 0, 0),
            p.rust.findrep(0, 0, 0, 0),
        );
    }
}

#[test]
fn cfg_30_findrep_active_1_each_position() {
    let mut r = Rng::new(0x0230);
    for pos in 0..4 {
        for _ in 0..40 {
            let mut v = [0i32; 4];
            v[pos] = {
                let mut x = r.i32_any();
                if x == 0 {
                    x = 1;
                }
                x
            };
            findrep_fresh(&format!("row30/pos{pos}"), v[0], v[1], v[2], v[3]);
        }
    }
}

#[test]
fn cfg_31_findrep_active_2_all_pairs() {
    let mut r = Rng::new(0x0231);
    for i in 0..4 {
        for j in (i + 1)..4 {
            for _ in 0..20 {
                let mut v = [0i32; 4];
                for &k in &[i, j] {
                    v[k] = {
                        let mut x = r.i32_any();
                        if x == 0 {
                            x = 1;
                        }
                        x
                    };
                }
                findrep_fresh(&format!("row31/pair({i},{j})"), v[0], v[1], v[2], v[3]);
            }
        }
    }
}

#[test]
fn cfg_32_findrep_active_3_all_triples() {
    let mut r = Rng::new(0x0232);
    for zero in 0..4 {
        for _ in 0..25 {
            let mut v = [0i32; 4];
            for k in 0..4 {
                if k != zero {
                    v[k] = {
                        let mut x = r.i32_any();
                        if x == 0 {
                            x = 1;
                        }
                        x
                    };
                }
            }
            findrep_fresh(&format!("row32/zero@{zero}"), v[0], v[1], v[2], v[3]);
        }
    }
}

#[test]
fn cfg_33_findrep_active_4() {
    let mut r = Rng::new(0x0233);
    for _ in 0..100 {
        let mut v = [0i32; 4];
        for k in 0..4 {
            v[k] = {
                let mut x = r.i32_any();
                if x == 0 {
                    x = 1;
                }
                x
            };
        }
        findrep_fresh("row33", v[0], v[1], v[2], v[3]);
    }
}

#[test]
fn cfg_34_findrep_bucket_cross_product() {
    // Full 5^4 = 625 cross-product of the validate_and_normalize buckets.
    // Each combination gets several randomized representatives; combinations
    // are run on a shared library so the state axis is exercised too, and a
    // fresh library is used every 25 combinations to also cover from-scratch.
    let mut r = Rng::new(0x0234);
    let mut p = Pair::fresh();
    let mut n = 0usize;
    for b1 in 0..5 {
        for b2 in 0..5 {
            for b3 in 0..5 {
                for b4 in 0..5 {
                    if n % 25 == 0 {
                        p = Pair::fresh();
                    }
                    n += 1;
                    for rep in 0..3 {
                        let (v1, v2, v3, v4) =
                            (r.bucket(b1), r.bucket(b2), r.bucket(b3), r.bucket(b4));
                        same(
                            &format!(
                                "row34: buckets({b1},{b2},{b3},{b4}) rep{rep} \
                                 findrep({v1},{v2},{v3},{v4})"
                            ),
                            p.c.findrep(v1, v2, v3, v4),
                            p.rust.findrep(v1, v2, v3, v4),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn cfg_35_findrep_accumulator_guard_true() {
    // Push `accumulator` past 0150 (104) before / via findrep so the
    // `subtract_from_accumulator` dispatch at lib.c:142 fires.
    let mut r = Rng::new(0x0235);
    for _ in 0..60 {
        let p = Pair::fresh();
        let seed = r.i32_in(200, 100000);
        same("row35: priming add", p.c.add(seed, seed), p.rust.add(seed, seed));
        let (a, b, c, d) = (
            r.i32_in(1, 2000),
            r.i32_in(1, 2000),
            r.i32_in(1, 2000),
            r.i32_in(1, 2000),
        );
        same(
            &format!("row35: findrep({a},{b},{c},{d}) with accumulator>104"),
            p.c.findrep(a, b, c, d),
            p.rust.findrep(a, b, c, d),
        );
    }
    // Also reach the guard purely through findrep's own add dispatch.
    for _ in 0..60 {
        let p = Pair::fresh();
        let (a, b) = (r.i32_in(64, 511), r.i32_in(64, 511));
        same(
            &format!("row35: self-primed findrep({a},{b},1,1)"),
            p.c.findrep(a, b, 1, 1),
            p.rust.findrep(a, b, 1, 1),
        );
    }
}

#[test]
fn cfg_36_findrep_multiplier_guard_true() {
    let mut r = Rng::new(0x0236);
    for _ in 0..80 {
        let p = Pair::fresh();
        let m = r.i32_in(65, 5000);
        same("row36: priming mul", p.c.mul(m, 1), p.rust.mul(m, 1));
        let (a, b, c, d) = (r.i32_in(0, 600), r.i32_in(0, 600), r.i32_in(0, 600), r.i32_in(0, 600));
        same(
            &format!("row36: findrep({a},{b},{c},{d}) with multiplier>64"),
            p.c.findrep(a, b, c, d),
            p.rust.findrep(a, b, c, d),
        );
        // Second call: multiplier has now been halved by the divide dispatch.
        same(
            "row36: follow-up findrep",
            p.c.findrep(a, b, c, d),
            p.rust.findrep(a, b, c, d),
        );
    }
}

#[test]
fn cfg_37_findrep_both_active() {
    let mut r = Rng::new(0x0237);
    // both_active FALSE via multiplier == 0 (latched by a zero-operand mul).
    for _ in 0..40 {
        let p = Pair::fresh();
        same("row37: zero mul", p.c.mul(0, 0), p.rust.mul(0, 0));
        let (a, b, c, d) = (r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any());
        same(
            "row37: findrep with multiplier==0",
            p.c.findrep(a, b, c, d),
            p.rust.findrep(a, b, c, d),
        );
    }
    // both_active FALSE via accumulator == 0 (fresh library, zero params).
    for _ in 0..40 {
        let p = Pair::fresh();
        same(
            "row37: findrep with accumulator==0",
            p.c.findrep(0, 0, 0, 0),
            p.rust.findrep(0, 0, 0, 0),
        );
    }
    // both_active TRUE.
    for _ in 0..40 {
        let p = Pair::fresh();
        let v = r.i32_in(1, 500);
        same("row37: prime add", p.c.add(v, v), p.rust.add(v, v));
        same(
            "row37: findrep with both nonzero",
            p.c.findrep(v, v, v, v),
            p.rust.findrep(v, v, v, v),
        );
    }
}

#[test]
fn cfg_38_findrep_repeated_same_args() {
    let mut r = Rng::new(0x0238);
    for case in 0..40 {
        let p = Pair::fresh();
        let (a, b, c, d) = (r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any());
        for i in 0..10 {
            same(
                &format!("row38: case{case} iter{i} findrep({a},{b},{c},{d})"),
                p.c.findrep(a, b, c, d),
                p.rust.findrep(a, b, c, d),
            );
        }
    }
}

#[test]
fn cfg_39_findrep_long_random_sequence() {
    let p = Pair::fresh();
    let mut r = Rng::new(0x0239);
    for i in 0..600 {
        let (a, b, c, d) = (r.i32_any(), r.i32_any(), r.i32_any(), r.i32_any());
        same(
            &format!("row39: #{i} findrep({a},{b},{c},{d})"),
            p.c.findrep(a, b, c, d),
            p.rust.findrep(a, b, c, d),
        );
    }
}

#[test]
fn cfg_40_findrep_small_params_sequence() {
    for seed in [0x0240u64, 0x0241, 0x0242] {
        let p = Pair::fresh();
        let mut r = Rng::new(seed);
        for i in 0..300 {
            let (a, b, c, d) = (
                r.i32_in(-4, 4),
                r.i32_in(-4, 4),
                r.i32_in(-4, 4),
                r.i32_in(-4, 4),
            );
            same(
                &format!("row40/{seed:#x}: #{i} findrep({a},{b},{c},{d})"),
                p.c.findrep(a, b, c, d),
                p.rust.findrep(a, b, c, d),
            );
        }
    }
}

#[test]
fn cfg_41_mixed_findrep_and_direct_ops() {
    let p = Pair::fresh();
    let mut r = Rng::new(0x0241);
    for i in 0..800 {
        let a = if r.bool() { r.i32_in(-600, 600) } else { r.i32_any() };
        let b = if r.bool() { r.i32_in(-600, 600) } else { r.i32_any() };
        let (cv, rv) = match r.next_u32() % 6 {
            0 => (p.c.add(a, b), p.rust.add(a, b)),
            1 => {
                let m = r.i32_in(-3, 3);
                (p.c.mul(a, m), p.rust.mul(a, m))
            }
            2 => (p.c.sub(a, b), p.rust.sub(a, b)),
            3 => {
                let d = r.i32_in(-9, 9);
                (p.c.div(a, d), p.rust.div(a, d))
            }
            _ => {
                let (c, d) = (r.i32_in(-600, 600), r.i32_in(-600, 600));
                (p.c.findrep(a, b, c, d), p.rust.findrep(a, b, c, d))
            }
        };
        same(&format!("row41: #{i} (a={a},b={b})"), cv, rv);
    }
}

#[test]
fn cfg_42_stateless_helpers_do_not_perturb_state() {
    let p = Pair::fresh();
    let mut r = Rng::new(0x0242);
    for i in 0..400 {
        // Stateless helpers interleaved with stateful calls.
        let v = r.i32_any();
        same(
            &format!("row42: normalize({v}) #{i}"),
            p.c.normalize(v),
            p.rust.normalize(v),
        );
        same_bytes(
            &format!("row42: octal({v}) #{i}"),
            &p.c.octal(v),
            &p.rust.octal(v),
        );
        let s = buf_from(b"Octal: 0123, Decimal: 83", 64);
        let ch = r.i32_any();
        same_bytes(
            &format!("row42: replace #{i} ch={ch}"),
            &p.c.replace(&s, ch),
            &p.rust.replace(&s, ch),
        );
        let (a, b, c, d) = (
            r.i32_in(-300, 300),
            r.i32_in(-300, 300),
            r.i32_in(-300, 300),
            r.i32_in(-300, 300),
        );
        same(
            &format!("row42: findrep #{i}"),
            p.c.findrep(a, b, c, d),
            p.rust.findrep(a, b, c, d),
        );
    }
}

#[test]
fn cfg_43_findrep_boundary_params() {
    const B: &[i32] = &[0, 1, 63, 64, 511, 512, i32::MIN, i32::MAX];
    let mut r = Rng::new(0x0243);
    let mut n = 0usize;
    let mut p = Pair::fresh();
    for &a in B {
        for &b in B {
            for &c in B {
                for &d in B {
                    if n % 16 == 0 {
                        p = Pair::fresh();
                    }
                    n += 1;
                    same(
                        &format!("row43: findrep({a},{b},{c},{d})"),
                        p.c.findrep(a, b, c, d),
                        p.rust.findrep(a, b, c, d),
                    );
                    // A second call from a different angle.
                    let x = r.i32_in(-2, 2);
                    same(
                        &format!("row43: follow-up findrep({x},{a},{b},{c})"),
                        p.c.findrep(x, a, b, c),
                        p.rust.findrep(x, a, b, c),
                    );
                }
            }
        }
    }
}

#[test]
fn cfg_44_findrep_overflow_params() {
    const X: &[i32] = &[i32::MIN, i32::MIN + 1, -1, 1, i32::MAX - 1, i32::MAX];
    for &a in X {
        for &b in X {
            for &c in X {
                for &d in X {
                    let p = Pair::fresh();
                    same(
                        &format!("row44: findrep({a},{b},{c},{d})"),
                        p.c.findrep(a, b, c, d),
                        p.rust.findrep(a, b, c, d),
                    );
                    same(
                        &format!("row44: repeat findrep({a},{b},{c},{d})"),
                        p.c.findrep(a, b, c, d),
                        p.rust.findrep(a, b, c, d),
                    );
                }
            }
        }
    }
}
