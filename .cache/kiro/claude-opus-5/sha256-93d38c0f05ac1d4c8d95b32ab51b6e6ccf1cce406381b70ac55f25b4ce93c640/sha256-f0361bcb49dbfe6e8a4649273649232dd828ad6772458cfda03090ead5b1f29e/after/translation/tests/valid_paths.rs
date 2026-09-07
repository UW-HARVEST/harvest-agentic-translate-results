//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both libraries are loaded from their `.so` files with `libloading`; nothing
//! is called directly.

mod common;

use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;
/// Randomized iterations per row.
const ITERS: usize = 400;

fn assert_drop_eq(pair: &Pair, payload: &[u8], ctx: &str) {
    let buf = cstring(payload);
    let c = call_drop(&pair.c, &buf);
    let r = call_drop(&pair.rs, &buf);
    assert_eq!(
        c, r,
        "w_utf8_drop offset mismatch ({ctx}): C={c} Rust={r} input={:02X?}",
        payload
    );
}

fn assert_filter_eq(pair: &Pair, payload: &[u8], replacement: u8, ctx: &str) {
    let buf = cstring(payload);
    let c = call_filter(&pair.c, &buf, replacement);
    let r = call_filter(&pair.rs, &buf, replacement);
    assert_eq!(
        c.is_some(),
        r.is_some(),
        "w_utf8_filter NULL-ness mismatch ({ctx}, repl={replacement}) input={:02X?}",
        payload
    );
    match (c, r) {
        (Some(c), Some(r)) => assert_eq!(
            c, r,
            "w_utf8_filter output mismatch ({ctx}, repl={replacement})\n  input = {:02X?}\n  C     = {:02X?}\n  Rust  = {:02X?}",
            payload, c, r
        ),
        _ => {}
    }
}

/// Both entry points, both flag values, in one shot.
fn assert_all_eq(pair: &Pair, payload: &[u8], ctx: &str) {
    assert_drop_eq(pair, payload, ctx);
    assert_filter_eq(pair, payload, 0, ctx);
    assert_filter_eq(pair, payload, 1, ctx);
}

// --- Row 1 -----------------------------------------------------------------
#[test]
fn row01_drop_empty_string() {
    let pair = load_pair();
    assert_drop_eq(&pair, b"", "row01 empty");
}

// --- Row 2 -----------------------------------------------------------------
#[test]
fn row02_drop_random_ascii() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..ITERS {
        let n = rng.range(1, 64);
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid_1(&mut v, &mut rng);
        }
        assert_drop_eq(&pair, &v, "row02 ascii");
    }
}

// --- Row 3 -----------------------------------------------------------------
#[test]
fn row03_drop_valid_two_byte_only() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..ITERS {
        let n = rng.range(1, 32);
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid_2(&mut v, &mut rng);
        }
        assert_drop_eq(&pair, &v, "row03 valid2");
    }
    // explicit boundaries
    for lead in [0xC2u8, 0xDFu8] {
        for cont in [0x80u8, 0xBFu8] {
            assert_drop_eq(&pair, &[lead, cont], "row03 boundary");
        }
    }
}

// --- Row 4 -----------------------------------------------------------------
#[test]
fn row04_drop_valid_three_byte_only() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..ITERS {
        let n = rng.range(1, 32);
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid_3(&mut v, &mut rng);
        }
        assert_drop_eq(&pair, &v, "row04 valid3");
    }
    for seq in [
        [0xE0u8, 0xA0, 0x80],
        [0xE0, 0xBF, 0xBF],
        [0xED, 0x80, 0x80],
        [0xED, 0x9F, 0xBF],
        [0xEF, 0xBF, 0xBF],
        [0xEE, 0x80, 0x80],
    ] {
        assert_drop_eq(&pair, &seq, "row04 boundary");
    }
}

// --- Row 5 -----------------------------------------------------------------
#[test]
fn row05_drop_valid_four_byte_only() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..ITERS {
        let n = rng.range(1, 32);
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid_4(&mut v, &mut rng);
        }
        assert_drop_eq(&pair, &v, "row05 valid4");
    }
    for seq in [
        [0xF0u8, 0x90, 0x80, 0x80],
        [0xF0, 0xBF, 0xBF, 0xBF],
        [0xF4, 0x80, 0x80, 0x80],
        [0xF4, 0x8F, 0xBF, 0xBF],
        [0xF1, 0x80, 0x80, 0x80],
    ] {
        assert_drop_eq(&pair, &seq, "row05 boundary");
    }
}

// --- Row 6 -----------------------------------------------------------------
#[test]
fn row06_drop_valid_mixed_widths() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..ITERS {
        let v = gen_valid_mixed_range(&mut rng, 1, 40);
        assert_drop_eq(&pair, &v, "row06 mixed valid");
    }
}

// --- Row 7 -----------------------------------------------------------------
#[test]
fn row07_drop_uniform_random_bytes() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..(ITERS * 4) {
        let n = rng.range(1, 64);
        let v: Vec<u8> = (0..n).map(|_| rng.nonzero_byte()).collect();
        assert_drop_eq(&pair, &v, "row07 uniform random");
    }
}

// --- Row 8 -----------------------------------------------------------------
#[test]
fn row08_drop_valid_prefix_then_injected_invalid() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..ITERS {
        let mut v = gen_valid_mixed_range(&mut rng, 1, 20);
        let at = rng.below(v.len() + 1);
        v.insert(at, invalid_byte(&mut rng));
        assert_drop_eq(&pair, &v, "row08 injected invalid");
    }
}

// --- Row 9 -----------------------------------------------------------------
#[test]
fn row09_drop_invalid_at_offset_zero() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 9);
    for bad in ALWAYS_INVALID {
        for _ in 0..40 {
            let mut v = vec![bad];
            v.extend_from_slice(&gen_valid_mixed_range(&mut rng, 0, 10));
            assert_drop_eq(&pair, &v, "row09 invalid at 0");
        }
    }
}

// --- Row 10 ----------------------------------------------------------------
#[test]
fn row10_drop_truncated_sequences_at_end() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 10);
    // every truncation of every valid width, after a random valid prefix
    for _ in 0..ITERS {
        let prefix = gen_valid_mixed_range(&mut rng, 0, 8);
        let mut full = Vec::new();
        match rng.below(3) {
            0 => push_valid_2(&mut full, &mut rng),
            1 => push_valid_3(&mut full, &mut rng),
            _ => push_valid_4(&mut full, &mut rng),
        }
        let keep = rng.range(1, full.len().saturating_sub(1).max(1));
        let mut v = prefix;
        v.extend_from_slice(&full[..keep]);
        assert_drop_eq(&pair, &v, "row10 truncated");
    }
    // hand-picked truncations
    for t in [
        vec![0xC2u8],
        vec![0xDF],
        vec![0xE0],
        vec![0xE0, 0xA0],
        vec![0xED],
        vec![0xED, 0x9F],
        vec![0xEF, 0xBF],
        vec![0xF0],
        vec![0xF0, 0x90],
        vec![0xF0, 0x90, 0x80],
        vec![0xF4],
        vec![0xF4, 0x8F],
        vec![0xF4, 0x8F, 0xBF],
    ] {
        assert_drop_eq(&pair, &t, "row10 hand truncation");
    }
}

// --- Row 11 ----------------------------------------------------------------
#[test]
fn row11_drop_long_mixed_inputs() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..40 {
        let target = rng.range(1024, 16384);
        let mut v = Vec::with_capacity(target + 8);
        while v.len() < target {
            if rng.below(8) == 0 {
                v.push(invalid_byte(&mut rng));
            } else {
                let one = gen_valid_mixed(&mut rng, 1);
                v.extend_from_slice(&one);
            }
        }
        assert_drop_eq(&pair, &v, "row11 long mixed");
    }
}

// --- Rows 12 & 13 ----------------------------------------------------------
#[test]
fn row12_row13_filter_empty_string_both_flags() {
    let pair = load_pair();
    assert_filter_eq(&pair, b"", 0, "row12 empty repl=false");
    assert_filter_eq(&pair, b"", 1, "row13 empty repl=true");
}

// --- Rows 14 & 15 ----------------------------------------------------------
#[test]
fn row14_row15_filter_fully_valid_strdup_fastpath() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..ITERS {
        let v = gen_valid_mixed_range(&mut rng, 1, 40);
        assert_filter_eq(&pair, &v, 0, "row14 valid repl=false");
        assert_filter_eq(&pair, &v, 1, "row15 valid repl=true");
    }
}

// --- Rows 16 & 17 ----------------------------------------------------------
#[test]
fn row16_row17_filter_invalid_at_offset_zero() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 16);
    for bad in ALWAYS_INVALID {
        for _ in 0..40 {
            let mut v = vec![bad];
            v.extend_from_slice(&gen_valid_mixed_range(&mut rng, 0, 10));
            assert_filter_eq(&pair, &v, 0, "row16 invalid@0 repl=false");
            assert_filter_eq(&pair, &v, 1, "row17 invalid@0 repl=true");
        }
    }
}

// --- Rows 18 & 19 ----------------------------------------------------------
#[test]
fn row18_row19_filter_single_invalid_in_middle() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..(ITERS * 2) {
        let head = gen_valid_mixed_range(&mut rng, 1, 12);
        let tail = gen_valid_mixed_range(&mut rng, 1, 12);
        let mut v = head;
        v.push(invalid_byte(&mut rng));
        v.extend_from_slice(&tail);
        assert_filter_eq(&pair, &v, 0, "row18 mid repl=false");
        assert_filter_eq(&pair, &v, 1, "row19 mid repl=true");
    }
}

// --- Rows 20 & 21 ----------------------------------------------------------
#[test]
fn row20_row21_filter_invalid_as_last_byte() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..ITERS {
        let mut v = gen_valid_mixed_range(&mut rng, 1, 16);
        v.push(invalid_byte(&mut rng));
        assert_filter_eq(&pair, &v, 0, "row20 last repl=false");
        assert_filter_eq(&pair, &v, 1, "row21 last repl=true");
    }
}

// --- Rows 22 & 23 ----------------------------------------------------------
#[test]
fn row22_row23_filter_uniform_random_bytes() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..(ITERS * 4) {
        let n = rng.range(1, 64);
        let v: Vec<u8> = (0..n).map(|_| rng.nonzero_byte()).collect();
        assert_filter_eq(&pair, &v, 0, "row22 rand repl=false");
        assert_filter_eq(&pair, &v, 1, "row23 rand repl=true");
    }
}

// --- Rows 24 & 25 ----------------------------------------------------------
#[test]
fn row24_row25_filter_mixed_valid_and_invalid() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..(ITERS * 2) {
        let n = rng.range(1, 30);
        let mut v = Vec::new();
        for _ in 0..n {
            if rng.below(3) == 0 {
                v.push(invalid_byte(&mut rng));
            } else {
                let one = gen_valid_mixed(&mut rng, 1);
                v.extend_from_slice(&one);
            }
        }
        if v.is_empty() {
            v.push(0x41);
        }
        assert_filter_eq(&pair, &v, 0, "row24 mixed repl=false");
        assert_filter_eq(&pair, &v, 1, "row25 mixed repl=true");
    }
}

// --- Row 26 ----------------------------------------------------------------
#[test]
fn row26_filter_all_invalid_no_replacement() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..60 {
        let n = rng.range(1, 4096);
        let v: Vec<u8> = (0..n).map(|_| invalid_byte(&mut rng)).collect();
        assert_filter_eq(&pair, &v, 0, "row26 all invalid repl=false");
    }
}

// --- Row 27 ----------------------------------------------------------------
#[test]
fn row27_filter_second_realloc_boundary() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 27);
    // repl starts at 0 -> first invalid byte reallocs and sets repl = 4093.
    // 4093 = 3 * 1364 + 1, so the 1366th replacement sees repl == 1 < 3.
    for n in [1364usize, 1365, 1366, 1367, 1368, 2000] {
        let v: Vec<u8> = (0..n).map(|_| invalid_byte(&mut rng)).collect();
        assert_filter_eq(&pair, &v, 1, "row27 realloc boundary");
        assert_filter_eq(&pair, &v, 0, "row27 realloc boundary repl=false");
    }
}

// --- Row 28 ----------------------------------------------------------------
#[test]
fn row28_filter_many_realloc_rounds() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 28);
    for n in [5000usize, 6000, 9000, 12000] {
        let v: Vec<u8> = (0..n).map(|_| invalid_byte(&mut rng)).collect();
        assert_filter_eq(&pair, &v, 1, "row28 many reallocs");
    }
    // interleaved valid + invalid at the same scale
    for _ in 0..10 {
        let mut v = Vec::new();
        while v.len() < 12000 {
            if rng.below(2) == 0 {
                v.push(invalid_byte(&mut rng));
            } else {
                let one = gen_valid_mixed(&mut rng, 1);
                v.extend_from_slice(&one);
            }
        }
        assert_filter_eq(&pair, &v, 1, "row28 interleaved large");
    }
}

// --- Rows 29 & 30 ----------------------------------------------------------
#[test]
fn row29_row30_filter_over_replacement_inc_sparse_invalid() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 29);
    for _ in 0..20 {
        let target = rng.range(4097, 20000);
        let mut v = Vec::with_capacity(target + 8);
        while v.len() < target {
            if rng.below(200) == 0 {
                v.push(invalid_byte(&mut rng));
            } else {
                let one = gen_valid_mixed(&mut rng, 1);
                v.extend_from_slice(&one);
            }
        }
        // guarantee at least one invalid byte so the strdup fast path is skipped
        let at = rng.below(v.len());
        v[at] = 0xFF;
        assert_filter_eq(&pair, &v, 0, "row29 >4096 sparse repl=false");
        assert_filter_eq(&pair, &v, 1, "row30 >4096 sparse repl=true");
    }
}

// --- Rows 31 & 32 ----------------------------------------------------------
#[test]
fn row31_row32_filter_truncated_at_end() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 31);
    for t in [
        vec![0xC2u8],
        vec![0xE0],
        vec![0xE0, 0xA0],
        vec![0xED, 0x9F],
        vec![0xF0],
        vec![0xF0, 0x90],
        vec![0xF0, 0x90, 0x80],
        vec![0xF4, 0x8F, 0xBF],
    ] {
        for _ in 0..20 {
            let mut v = gen_valid_mixed_range(&mut rng, 0, 8);
            v.extend_from_slice(&t);
            if v.is_empty() {
                continue;
            }
            assert_filter_eq(&pair, &v, 0, "row31 truncated repl=false");
            assert_filter_eq(&pair, &v, 1, "row32 truncated repl=true");
        }
    }
}

// --- Row 33 ----------------------------------------------------------------
#[test]
fn row33_filter_out_of_range_bool_values() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 33);
    for flag in [0u8, 1, 2, 3, 0x7F, 0x80, 0xFE, 0xFF] {
        for _ in 0..60 {
            let n = rng.range(1, 40);
            let v: Vec<u8> = (0..n).map(|_| rng.nonzero_byte()).collect();
            assert_filter_eq(&pair, &v, flag, "row33 out-of-range bool");
        }
        // also on an all-invalid payload, where the flag decides everything
        let v: Vec<u8> = (0..64).map(|_| invalid_byte(&mut rng)).collect();
        assert_filter_eq(&pair, &v, flag, "row33 out-of-range bool all-invalid");
    }
}

// --- Row 34 ----------------------------------------------------------------
#[test]
fn row34_composed_drop_then_filter_pipeline() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 34);
    for _ in 0..(ITERS * 2) {
        let n = rng.range(1, 40);
        let mut v = Vec::new();
        for _ in 0..n {
            if rng.below(4) == 0 {
                v.push(invalid_byte(&mut rng));
            } else {
                let one = gen_valid_mixed(&mut rng, 1);
                v.extend_from_slice(&one);
            }
        }
        if v.is_empty() {
            v.push(0x41);
        }
        let buf = cstring(&v);
        let off_c = call_drop(&pair.c, &buf);
        let off_r = call_drop(&pair.rs, &buf);
        assert_eq!(off_c, off_r, "row34 drop offsets differ for {:02X?}", v);

        for flag in [0u8, 1] {
            let out_c = call_filter(&pair.c, &buf, flag).expect("C filter returned NULL");
            let out_r = call_filter(&pair.rs, &buf, flag).expect("Rust filter returned NULL");
            assert_eq!(out_c, out_r, "row34 filter differs for {:02X?}", v);
            // the memcpy'd prefix is exactly the bytes before the drop point
            if off_c < v.len() {
                assert!(
                    out_c.len() >= off_c && out_c[..off_c] == v[..off_c],
                    "row34 prefix invariant broken: off={off_c} in={:02X?} out={:02X?}",
                    v,
                    out_c
                );
            }
        }
    }
}

// --- Row 35 ----------------------------------------------------------------
#[test]
fn row35_filter_both_flags_same_corpus() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 35);
    for _ in 0..(ITERS * 2) {
        let n = rng.range(1, 80);
        let v: Vec<u8> = (0..n).map(|_| rng.nonzero_byte()).collect();
        let buf = cstring(&v);
        for flag in [0u8, 1] {
            let c = call_filter(&pair.c, &buf, flag);
            let r = call_filter(&pair.rs, &buf, flag);
            assert_eq!(c, r, "row35 mismatch flag={flag} input={:02X?}", v);
        }
    }
}

// --- Row 36 ----------------------------------------------------------------
#[test]
fn row36_drop_exhaustive_single_byte() {
    let pair = load_pair();
    for b in 1u16..=0xFF {
        assert_all_eq(&pair, &[b as u8], "row36 single byte");
    }
}

// --- Rows 37 & 38 ----------------------------------------------------------
#[test]
fn row37_row38_exhaustive_two_byte_sweep() {
    let pair = load_pair();
    for b0 in 1u16..=0xFF {
        for b1 in 1u16..=0xFF {
            let payload = [b0 as u8, b1 as u8];
            let buf = cstring(&payload);
            let c = call_drop(&pair.c, &buf);
            let r = call_drop(&pair.rs, &buf);
            assert_eq!(c, r, "row37 drop differs for {:02X?}", payload);
            for flag in [0u8, 1] {
                let oc = call_filter(&pair.c, &buf, flag);
                let or = call_filter(&pair.rs, &buf, flag);
                assert_eq!(oc, or, "row38 filter differs for {:02X?} flag={flag}", payload);
            }
        }
    }
}

// --- Row 39 ----------------------------------------------------------------
#[test]
fn row39_drop_exhaustive_three_byte_leads() {
    let pair = load_pair();
    for b0 in 0xE0u8..=0xEF {
        for b1 in 1u16..=0xFF {
            for b2 in 1u16..=0xFF {
                let payload = [b0, b1 as u8, b2 as u8];
                let buf = cstring(&payload);
                let c = call_drop(&pair.c, &buf);
                let r = call_drop(&pair.rs, &buf);
                assert_eq!(c, r, "row39 drop differs for {:02X?}", payload);
            }
        }
    }
}

// --- Row 39b: filter over the same space, boundary-focused ----------------
#[test]
fn row39b_filter_three_byte_leads_boundary_focused() {
    let pair = load_pair();
    for b0 in 0xE0u8..=0xEF {
        for &b1 in INTERESTING.iter() {
            for &b2 in INTERESTING.iter() {
                let payload = [b0, b1, b2];
                let buf = cstring(&payload);
                for flag in [0u8, 1] {
                    let oc = call_filter(&pair.c, &buf, flag);
                    let or = call_filter(&pair.rs, &buf, flag);
                    assert_eq!(
                        oc, or,
                        "row39b filter differs for {:02X?} flag={flag}",
                        payload
                    );
                }
            }
        }
    }
}

// --- Row 40 ----------------------------------------------------------------
#[test]
fn row40_drop_exhaustive_four_byte_leads() {
    let pair = load_pair();
    for b0 in 0xF0u8..=0xF7 {
        for b1 in 1u16..=0xFF {
            for &b2 in INTERESTING.iter() {
                for &b3 in INTERESTING.iter() {
                    let payload = [b0, b1 as u8, b2, b3];
                    let buf = cstring(&payload);
                    let c = call_drop(&pair.c, &buf);
                    let r = call_drop(&pair.rs, &buf);
                    assert_eq!(c, r, "row40 drop differs for {:02X?}", payload);
                }
            }
        }
    }
}

// --- Row 40b: filter over four-byte space --------------------------------
#[test]
fn row40b_filter_four_byte_leads() {
    let pair = load_pair();
    for b0 in 0xF0u8..=0xF7 {
        for &b1 in INTERESTING.iter() {
            for &b2 in INTERESTING.iter() {
                for &b3 in INTERESTING.iter() {
                    let payload = [b0, b1, b2, b3];
                    let buf = cstring(&payload);
                    for flag in [0u8, 1] {
                        let oc = call_filter(&pair.c, &buf, flag);
                        let or = call_filter(&pair.rs, &buf, flag);
                        assert_eq!(
                            oc, or,
                            "row40b filter differs for {:02X?} flag={flag}",
                            payload
                        );
                    }
                }
            }
        }
    }
}
