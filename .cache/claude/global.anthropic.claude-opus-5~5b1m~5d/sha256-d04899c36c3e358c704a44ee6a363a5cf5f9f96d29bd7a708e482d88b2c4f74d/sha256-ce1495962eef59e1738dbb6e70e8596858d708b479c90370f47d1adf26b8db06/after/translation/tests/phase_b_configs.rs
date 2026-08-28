//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every row drives BOTH shared objects through their exported C symbols and
//! compares the results byte for byte. Randomised rows use a fixed splitmix64
//! seed so failures are reproducible.

mod common;

use common::*;

/// Number of randomised inputs per randomised row.
const ITERS: usize = 2000;

// ===========================================================================
// C1..C12 — w_utf8_drop (the low-level entry point) on every input shape
// ===========================================================================

#[test]
fn c1_drop_empty() {
    let p = pair();
    diff_drop(&p, b"");
}

#[test]
fn c2_drop_ascii() {
    let p = pair();
    let mut rng = Rng::new(0xC002);
    for len in 1..=64 {
        diff_drop(&p, &gen_ascii(&mut rng, len));
    }
    for _ in 0..ITERS {
        let len = 1 + rng.below(200);
        diff_drop(&p, &gen_ascii(&mut rng, len));
    }
}

#[test]
fn c3_drop_valid2() {
    let p = pair();
    let mut rng = Rng::new(0xC003);
    for _ in 0..ITERS {
        let n = 1 + rng.below(60);
        diff_drop(&p, &gen_valid(&mut rng, n, &[2]));
    }
    // every legal 2-byte lead with the two extreme continuations
    for b0 in 0xC2u8..=0xDF {
        for b1 in [0x80u8, 0xBF] {
            diff_drop(&p, &[b0, b1]);
        }
    }
}

#[test]
fn c4_drop_valid3() {
    let p = pair();
    let mut rng = Rng::new(0xC004);
    for _ in 0..ITERS {
        let n = 1 + rng.below(60);
        diff_drop(&p, &gen_valid(&mut rng, n, &[3]));
    }
    for b0 in 0xE0u8..=0xEF {
        let lo = if b0 == 0xE0 { 0xA0 } else { 0x80 };
        let hi = if b0 == 0xED { 0x9F } else { 0xBF };
        for b1 in [lo, hi] {
            for b2 in [0x80u8, 0xBF] {
                diff_drop(&p, &[b0, b1, b2]);
            }
        }
    }
}

#[test]
fn c5_drop_valid4() {
    let p = pair();
    let mut rng = Rng::new(0xC005);
    for _ in 0..ITERS {
        let n = 1 + rng.below(60);
        diff_drop(&p, &gen_valid(&mut rng, n, &[4]));
    }
    for b0 in 0xF0u8..=0xF4 {
        let lo = if b0 == 0xF0 { 0x90 } else { 0x80 };
        let hi = if b0 == 0xF4 { 0x8F } else { 0xBF };
        for b1 in [lo, hi] {
            for b2 in [0x80u8, 0xBF] {
                for b3 in [0x80u8, 0xBF] {
                    diff_drop(&p, &[b0, b1, b2, b3]);
                }
            }
        }
    }
}

#[test]
fn c6_drop_valid_mixed() {
    let p = pair();
    let mut rng = Rng::new(0xC006);
    for _ in 0..ITERS {
        let n = 1 + rng.below(80);
        diff_drop(&p, &gen_valid(&mut rng, n, &[1, 2, 3, 4]));
    }
}

#[test]
fn c7_drop_boundary_codepoints() {
    let p = pair();
    // one at a time
    for &cp in BOUNDARY_CODEPOINTS {
        diff_drop(&p, &encode_utf8(cp));
    }
    // all of them concatenated, and every adjacent pair
    let mut all = Vec::new();
    for &cp in BOUNDARY_CODEPOINTS {
        all.extend_from_slice(&encode_utf8(cp));
    }
    diff_drop(&p, &all);
    for a in BOUNDARY_CODEPOINTS {
        for b in BOUNDARY_CODEPOINTS {
            let mut v = encode_utf8(*a);
            v.extend_from_slice(&encode_utf8(*b));
            diff_drop(&p, &v);
        }
    }
}

#[test]
fn c8_drop_uniform_random() {
    let p = pair();
    let mut rng = Rng::new(0xC008);
    for len in 1..=64 {
        for _ in 0..20 {
            diff_drop(&p, &gen_uniform(&mut rng, len));
        }
    }
}

#[test]
fn c9_drop_interesting_bytes() {
    let p = pair();
    let mut rng = Rng::new(0xC009);
    for len in 1..=40 {
        for _ in 0..40 {
            diff_drop(&p, &gen_interesting(&mut rng, len));
        }
    }
}

#[test]
fn c10_drop_valid_prefix_then_invalid() {
    let p = pair();
    let mut rng = Rng::new(0xC010);
    for _ in 0..ITERS {
        let mut v = gen_valid_n(&mut rng, 30);
        let cls = rng.below(INVALID_CLASSES);
        push_invalid(&mut v, &mut rng, cls);
        let junk = rng.below(20);
        v.extend_from_slice(&gen_uniform(&mut rng, junk));
        diff_drop(&p, &v);
    }
}

#[test]
fn c11_drop_truncated_tail() {
    let p = pair();
    let mut rng = Rng::new(0xC011);
    for _ in 0..ITERS {
        for width in 2u8..=4 {
            let mut seq = Vec::new();
            push_valid(&mut seq, &mut rng, width);
            // cut off 1..width-1 trailing bytes
            for cut in 1..width as usize {
                let prefix = gen_valid_n(&mut rng, 8);
                let mut v = prefix;
                v.extend_from_slice(&seq[..seq.len() - cut]);
                diff_drop(&p, &v);
            }
        }
    }
}

#[test]
fn c12_drop_long_mixed() {
    let p = pair();
    let mut rng = Rng::new(0xC012);
    for _ in 0..8 {
        let mut v = Vec::with_capacity(64 * 1024);
        while v.len() < 64 * 1024 {
            let chunk = gen_mixed(&mut rng, 64);
            v.extend_from_slice(&chunk);
        }
        diff_drop(&p, &v);
    }
}

// ===========================================================================
// C13..C34 — w_utf8_filter
// ===========================================================================

/// The shapes used by the "fully valid input" rows.
fn valid_shapes(rng: &mut Rng) -> Vec<Vec<u8>> {
    let mut out = vec![Vec::new()];
    for w in [1u8, 2, 3, 4] {
        for n in [1usize, 2, 3, 7, 33] {
            out.push(gen_valid(rng, n, &[w]));
        }
    }
    for n in [1usize, 5, 17, 64, 257] {
        out.push(gen_valid(rng, n, &[1, 2, 3, 4]));
    }
    for &cp in BOUNDARY_CODEPOINTS {
        out.push(encode_utf8(cp));
    }
    out
}

#[test]
fn c13_filter_valid_strdup_r0() {
    let p = pair();
    let mut rng = Rng::new(0xC013);
    for _ in 0..40 {
        for s in valid_shapes(&mut rng) {
            diff_filter(&p, &s, 0);
        }
    }
}

#[test]
fn c14_filter_valid_strdup_r1() {
    let p = pair();
    let mut rng = Rng::new(0xC014);
    for _ in 0..40 {
        for s in valid_shapes(&mut rng) {
            diff_filter(&p, &s, 1);
        }
    }
}

#[test]
fn c15_filter_empty() {
    let p = pair();
    for r in [0u8, 1, 2, 0xFF] {
        diff_filter(&p, b"", r);
    }
}

fn invalid_at_offset(rng: &mut Rng, prefix_seqs: usize) -> Vec<u8> {
    let mut v = gen_valid(rng, prefix_seqs, &[1, 2, 3, 4]);
    let cls = rng.below(INVALID_CLASSES);
    push_invalid(&mut v, rng, cls);
    v.extend_from_slice(&gen_valid_n(rng, 10));
    v
}

#[test]
fn c16_filter_invalid_at_0_r0() {
    let p = pair();
    let mut rng = Rng::new(0xC016);
    for _ in 0..ITERS {
        diff_filter(&p, &invalid_at_offset(&mut rng, 0), 0);
    }
}

#[test]
fn c17_filter_invalid_at_0_r1() {
    let p = pair();
    let mut rng = Rng::new(0xC017);
    for _ in 0..ITERS {
        diff_filter(&p, &invalid_at_offset(&mut rng, 0), 1);
    }
}

#[test]
fn c18_filter_invalid_mid_r0() {
    let p = pair();
    let mut rng = Rng::new(0xC018);
    for _ in 0..ITERS {
        let n = 1 + rng.below(20);
        diff_filter(&p, &invalid_at_offset(&mut rng, n), 0);
    }
}

#[test]
fn c19_filter_invalid_mid_r1() {
    let p = pair();
    let mut rng = Rng::new(0xC019);
    for _ in 0..ITERS {
        let n = 1 + rng.below(20);
        diff_filter(&p, &invalid_at_offset(&mut rng, n), 1);
    }
}

#[test]
fn c20_filter_invalid_last() {
    let p = pair();
    let mut rng = Rng::new(0xC020);
    for _ in 0..ITERS {
        let mut v = gen_valid_n(&mut rng, 20);
        // a single byte that can never start a valid sequence
        v.push(rng.pick(&[0x80u8, 0xBF, 0xC0, 0xC1, 0xF5, 0xF8, 0xFE, 0xFF]));
        for r in [0u8, 1] {
            diff_filter(&p, &v, r);
        }
    }
}

#[test]
fn c21_filter_uniform_r0() {
    let p = pair();
    let mut rng = Rng::new(0xC021);
    for len in 1..=64 {
        for _ in 0..20 {
            diff_filter(&p, &gen_uniform(&mut rng, len), 0);
        }
    }
}

#[test]
fn c22_filter_uniform_r1() {
    let p = pair();
    let mut rng = Rng::new(0xC022);
    for len in 1..=64 {
        for _ in 0..20 {
            diff_filter(&p, &gen_uniform(&mut rng, len), 1);
        }
    }
}

#[test]
fn c23_filter_interesting_r0() {
    let p = pair();
    let mut rng = Rng::new(0xC023);
    for len in 1..=40 {
        for _ in 0..40 {
            diff_filter(&p, &gen_interesting(&mut rng, len), 0);
        }
    }
}

#[test]
fn c24_filter_interesting_r1() {
    let p = pair();
    let mut rng = Rng::new(0xC024);
    for len in 1..=40 {
        for _ in 0..40 {
            diff_filter(&p, &gen_interesting(&mut rng, len), 1);
        }
    }
}

#[test]
fn c25_filter_mixed_classes_r0() {
    let p = pair();
    let mut rng = Rng::new(0xC025);
    for _ in 0..ITERS {
        let n = 1 + rng.below(60);
        diff_filter(&p, &gen_mixed(&mut rng, n), 0);
    }
    // and one row per invalid class, isolated, embedded in valid text
    for class in 0..INVALID_CLASSES {
        for _ in 0..20 {
            let mut v = gen_valid_n(&mut rng, 6);
            push_invalid(&mut v, &mut rng, class);
            v.extend_from_slice(&gen_valid_n(&mut rng, 6));
            diff_filter(&p, &v, 0);
        }
    }
}

#[test]
fn c26_filter_mixed_classes_r1() {
    let p = pair();
    let mut rng = Rng::new(0xC026);
    for _ in 0..ITERS {
        let n = 1 + rng.below(60);
        diff_filter(&p, &gen_mixed(&mut rng, n), 1);
    }
    for class in 0..INVALID_CLASSES {
        for _ in 0..20 {
            let mut v = gen_valid_n(&mut rng, 6);
            push_invalid(&mut v, &mut rng, class);
            v.extend_from_slice(&gen_valid_n(&mut rng, 6));
            diff_filter(&p, &v, 1);
        }
    }
}

#[test]
fn c27_filter_truncated_tail() {
    let p = pair();
    let mut rng = Rng::new(0xC027);
    for _ in 0..ITERS {
        for width in 2u8..=4 {
            let mut seq = Vec::new();
            push_valid(&mut seq, &mut rng, width);
            for cut in 1..width as usize {
                let mut v = gen_valid_n(&mut rng, 8);
                v.extend_from_slice(&seq[..seq.len() - cut]);
                for r in [0u8, 1] {
                    diff_filter(&p, &v, r);
                }
            }
        }
    }
}

/// Run lengths that straddle the `REPLACEMENT_INC`/`repl < 3` bookkeeping:
/// 4096 = 3·1365 + 1, so a realloc happens on replacement 1 and then again on
/// replacement 1366, 2731, …
const RUN_LENGTHS: &[usize] = &[
    1, 2, 3, 4, 5, 1363, 1364, 1365, 1366, 1367, 2729, 2730, 2731, 2732, 4095, 4096, 4097, 8191,
    8192,
];

#[test]
fn c28_filter_realloc_boundary_r1() {
    let p = pair();
    let mut rng = Rng::new(0xC028);
    for &n in RUN_LENGTHS {
        // pure run of invalid bytes
        let run: Vec<u8> = (0..n).map(|_| rng.pick(&[0x80u8, 0xC0, 0xF5, 0xFF])).collect();
        diff_filter(&p, &run, 1);
        // run preceded and followed by valid text
        let mut v = gen_valid(&mut rng, 5, &[1, 2, 3, 4]);
        v.extend_from_slice(&run);
        v.extend_from_slice(&gen_valid(&mut rng, 5, &[1, 2, 3, 4]));
        diff_filter(&p, &v, 1);
        // invalid bytes interleaved with single ASCII bytes
        let mut w = Vec::with_capacity(2 * n);
        for _ in 0..n {
            w.push(rng.range_u8(0x41, 0x5A));
            w.push(rng.pick(&[0x80u8, 0xC1, 0xF7, 0xF8]));
        }
        diff_filter(&p, &w, 1);
    }
}

#[test]
fn c29_filter_runs_no_realloc_r0() {
    let p = pair();
    let mut rng = Rng::new(0xC029);
    for &n in RUN_LENGTHS {
        let run: Vec<u8> = (0..n).map(|_| rng.pick(&[0x80u8, 0xC0, 0xF5, 0xFF])).collect();
        diff_filter(&p, &run, 0);
        let mut v = gen_valid(&mut rng, 5, &[1, 2, 3, 4]);
        v.extend_from_slice(&run);
        v.extend_from_slice(&gen_valid(&mut rng, 5, &[1, 2, 3, 4]));
        diff_filter(&p, &v, 0);
    }
}

#[test]
fn c30_filter_long_mixed() {
    let p = pair();
    let mut rng = Rng::new(0xC030);
    for _ in 0..6 {
        let mut v = Vec::with_capacity(64 * 1024);
        while v.len() < 64 * 1024 {
            let chunk = gen_mixed(&mut rng, 64);
            v.extend_from_slice(&chunk);
        }
        for r in [0u8, 1] {
            diff_filter(&p, &v, r);
        }
    }
}

#[test]
fn c31_filter_large_valid() {
    let p = pair();
    let mut rng = Rng::new(0xC031);
    let mut v = Vec::with_capacity(1 << 20);
    while v.len() < (1 << 20) {
        let chunk = gen_valid(&mut rng, 256, &[1, 2, 3, 4]);
        v.extend_from_slice(&chunk);
    }
    for r in [0u8, 1] {
        diff_filter(&p, &v, r);
    }
}

#[test]
fn c32_filter_large_invalid_r1() {
    let p = pair();
    let mut rng = Rng::new(0xC032);
    let v: Vec<u8> = (0..(1usize << 20))
        .map(|_| rng.pick(&[0x80u8, 0xC0, 0xC1, 0xF5, 0xF8, 0xFF]))
        .collect();
    diff_filter(&p, &v, 1);
    diff_filter(&p, &v, 0);
}

#[test]
fn c33_filter_noncanonical_bool() {
    let p = pair();
    let mut rng = Rng::new(0xC033);
    for r in [2u8, 3, 0x7F, 0x80, 0xFE, 0xFF] {
        for _ in 0..80 {
            let n = 1 + rng.below(40);
            diff_filter(&p, &gen_mixed(&mut rng, n), r);
        }
        for len in 1..=16 {
            diff_filter(&p, &gen_interesting(&mut rng, len), r);
        }
    }
}

#[test]
fn c34_filter_wide_bool_register() {
    let p = pair();
    let mut rng = Rng::new(0xC034);
    let wides: &[u64] = &[
        0x0000_0000_0000_0000,
        0x0000_0000_0000_0001,
        0x0000_0000_0000_0002,
        0x0000_0000_0000_0100,
        0x0000_0000_0000_01FF,
        0x0000_0000_FFFF_FF00,
        0x0000_0000_FFFF_FFFF,
        0x0000_00DE_ADBE_EF00,
        0x0000_00DE_ADBE_EF01,
        0xFFFF_FFFF_FFFF_FF00,
        0xFFFF_FFFF_FFFF_FFFF,
    ];
    for &r in wides {
        for _ in 0..60 {
            let n = 1 + rng.below(40);
            diff_filter_wide(&p, &gen_mixed(&mut rng, n), r);
        }
        for len in 1..=16 {
            diff_filter_wide(&p, &gen_interesting(&mut rng, len), r);
        }
    }
}

// ===========================================================================
// C35..C43 — composed pipeline, statelessness, exhaustive sweeps
// ===========================================================================

#[test]
fn c35_composed_pipeline() {
    let p = pair();
    let mut rng = Rng::new(0xC035);
    for _ in 0..ITERS {
        let n = 1 + rng.below(40);
        let bytes = gen_mixed(&mut rng, n);
        let buf = cstr_buf(&bytes);
        let base = buf.as_ptr() as *const std::os::raw::c_char;

        // step 1: low-level scanner, identical offsets
        let (co, ro) = unsafe { ((p.c.drop_)(base), (p.rs.drop_)(base)) };
        assert_eq!(co as usize - base as usize, ro as usize - base as usize);

        // step 2: filter the *suffix* the scanner stopped at (feeding one
        // library's output pointer into the other entry point)
        let suffix_off = co as usize - base as usize;
        let suffix = &bytes[suffix_off..];
        for r in [0u8, 1] {
            diff_filter(&p, suffix, r);
        }

        // step 3: filtering with replacement must produce a string that the
        // scanner accepts entirely; without replacement too.
        for r in [0u8, 1] {
            unsafe {
                let cp = (p.c.filter)(base, r);
                let rp = (p.rs.filter)(base, r);
                assert!(!cp.is_null() && !rp.is_null());
                // scan each library's output with the *other* library's scanner
                let c_end = (p.rs.drop_)(cp);
                let r_end = (p.c.drop_)(rp);
                assert_eq!(*c_end, 0, "C output not fully valid after filtering");
                assert_eq!(*r_end, 0, "RUST output not fully valid after filtering");
                assert_eq!(
                    c_end as usize - cp as usize,
                    r_end as usize - rp as usize,
                    "post-filter scan length mismatch"
                );
                libc_free(cp);
                libc_free(rp);
            }
        }
    }
}

unsafe extern "C" {
    #[link_name = "free"]
    fn c_free(p: *mut std::ffi::c_void);
}
unsafe fn libc_free(p: *mut std::os::raw::c_char) {
    unsafe { c_free(p.cast()) }
}

#[test]
fn c36_filter_twice() {
    let p = pair();
    let mut rng = Rng::new(0xC036);
    for _ in 0..ITERS {
        let n = 1 + rng.below(40);
        let bytes = gen_mixed(&mut rng, n);
        for r in [0u8, 1] {
            let buf = cstr_buf(&bytes);
            let base = buf.as_ptr() as *const std::os::raw::c_char;
            unsafe {
                let c1 = (p.c.filter)(base, r);
                let r1 = (p.rs.filter)(base, r);
                let c2 = (p.c.filter)(c1, r);
                let r2 = (p.rs.filter)(r1, r);
                let cs = std::ffi::CStr::from_ptr(c2).to_bytes().to_vec();
                let rs = std::ffi::CStr::from_ptr(r2).to_bytes().to_vec();
                let c1s = std::ffi::CStr::from_ptr(c1).to_bytes().to_vec();
                let r1s = std::ffi::CStr::from_ptr(r1).to_bytes().to_vec();
                libc_free(c1);
                libc_free(r1);
                libc_free(c2);
                libc_free(r2);
                assert_eq!(c1s, r1s, "first pass mismatch");
                assert_eq!(cs, rs, "second pass mismatch");
            }
        }
    }
}

#[test]
fn c37_exhaustive_len1() {
    let p = pair();
    for b in 1u8..=0xFF {
        let v = [b];
        diff_drop(&p, &v);
        for r in [0u8, 1] {
            diff_filter(&p, &v, r);
        }
    }
}

#[test]
fn c38_exhaustive_len2() {
    let p = pair();
    for b0 in 1u8..=0xFF {
        for b1 in 1u8..=0xFF {
            let v = [b0, b1];
            diff_drop(&p, &v);
            for r in [0u8, 1] {
                diff_filter(&p, &v, r);
            }
        }
    }
}

#[test]
fn c39_exhaustive_len3_interesting() {
    let p = pair();
    for &b0 in INTERESTING {
        for &b1 in INTERESTING {
            for &b2 in INTERESTING {
                let v = [b0, b1, b2];
                diff_drop(&p, &v);
                for r in [0u8, 1] {
                    diff_filter(&p, &v, r);
                }
            }
        }
    }
    let mut rng = Rng::new(0xC039);
    for _ in 0..5000 {
        let v = gen_uniform(&mut rng, 3);
        diff_drop(&p, &v);
        diff_filter(&p, &v, 1);
    }
}

#[test]
fn c40_exhaustive_len4_interesting() {
    let p = pair();
    for &b0 in INTERESTING {
        for &b1 in INTERESTING {
            for &b2 in INTERESTING {
                for &b3 in INTERESTING {
                    let v = [b0, b1, b2, b3];
                    diff_drop(&p, &v);
                    diff_filter(&p, &v, 1);
                }
            }
        }
    }
    // second sweep with replacement = 0 (separate loop keeps the runtime sane)
    for &b0 in INTERESTING {
        for &b1 in INTERESTING {
            for &b2 in INTERESTING {
                for &b3 in INTERESTING {
                    diff_filter(&p, &[b0, b1, b2, b3], 0);
                }
            }
        }
    }
}

#[test]
fn c41_ef_lead_sweep() {
    let p = pair();
    // 0xEF is the lead byte guarded by the (unreachable) `<= 0xBF` clause.
    for b1 in 1u8..=0xFF {
        for b2 in 1u8..=0xFF {
            let v = [0xEFu8, b1, b2];
            diff_drop(&p, &v);
            for r in [0u8, 1] {
                diff_filter(&p, &v, r);
            }
        }
    }
    // U+FFFD itself (EF BF BD) — the replacement character the filter emits
    for r in [0u8, 1] {
        diff_filter(&p, b"\xEF\xBF\xBD", r);
        diff_filter(&p, b"\xEF\xBF\xBD\x80\xEF\xBF\xBD", r);
    }
}

#[test]
fn c42_guarded_lead_all_second_bytes() {
    let p = pair();
    for lead in [0xC0u8, 0xC1, 0xC2, 0xDF, 0xE0, 0xED, 0xEE, 0xEF, 0xF0, 0xF4, 0xF5, 0xF8] {
        for b1 in 1u8..=0xFF {
            for tail in [
                [].as_slice(),
                [0x80u8].as_slice(),
                [0xBFu8, 0x80].as_slice(),
                [0x41u8].as_slice(),
            ] {
                let mut v = vec![lead, b1];
                v.extend_from_slice(tail);
                diff_drop(&p, &v);
                for r in [0u8, 1] {
                    diff_filter(&p, &v, r);
                }
            }
        }
    }
}

#[test]
fn c43_repeated_calls_stateless() {
    let p = pair();
    let mut rng = Rng::new(0xC043);
    for _ in 0..200 {
        let n = 1 + rng.below(30);
        let bytes = gen_mixed(&mut rng, n);
        // interleave: c, rust, c, rust … results must never drift
        for _ in 0..4 {
            diff_drop(&p, &bytes);
            for r in [0u8, 1] {
                diff_filter(&p, &bytes, r);
            }
        }
    }
}

// ===========================================================================
// C44 / C45 — strlen semantics and unaligned start pointers
// ===========================================================================

#[test]
fn c44_interior_nul_terminates() {
    let p = pair();
    let mut rng = Rng::new(0xC044);
    for _ in 0..ITERS {
        let n1 = 1 + rng.below(12);
        let head = gen_mixed(&mut rng, n1);
        let n2 = 1 + rng.below(12);
        let tail = gen_mixed(&mut rng, n2);
        // one interior NUL: everything from it on must be ignored by strlen
        let mut v = head.clone();
        v.push(0);
        v.extend_from_slice(&tail);
        for r in [0u8, 1] {
            diff_raw_at(&p, &v, 0, r);
        }
        // several interior NULs
        let mut w = head.clone();
        w.push(0);
        w.push(0);
        w.extend_from_slice(&tail);
        w.push(0);
        w.extend_from_slice(&tail);
        for r in [0u8, 1] {
            diff_raw_at(&p, &w, 0, r);
        }
    }
    // a buffer that starts with the terminator behaves like ""
    for r in [0u8, 1] {
        diff_raw_at(&p, &[0, 0x80, 0xFF, 0x41], 0, r);
    }
}

#[test]
fn c45_unaligned_start_pointer() {
    let p = pair();
    let mut rng = Rng::new(0xC045);
    for _ in 0..(ITERS / 4) {
        let n = 1 + rng.below(20);
        let v = gen_mixed(&mut rng, n);
        for offset in 0..16 {
            for r in [0u8, 1] {
                diff_raw_at(&p, &v, offset, r);
            }
        }
    }
    // long buffers at every offset within a cache line
    for _ in 0..8 {
        let v = gen_mixed(&mut rng, 400);
        for offset in 0..8 {
            diff_raw_at(&p, &v, offset, 1);
        }
    }
}
