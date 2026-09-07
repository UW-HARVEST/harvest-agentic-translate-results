//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row calls `driver` in BOTH the C
//! `.so` and the Rust `.so` through `libloading` and compares the stdout bytes.
//! Rows use randomized inputs from a fixed-seed SplitMix64 generator.

mod common;

use common::*;
use std::ffi::c_char;

/// Independently computed reference for `strcspn` + `printf("%zu\n")`, used as
/// a third opinion so a test cannot pass by both sides being equally wrong.
fn expected(s1: &[u8], s2: &[u8]) -> Vec<u8> {
    let n = s1
        .iter()
        .position(|c| s2.contains(c))
        .unwrap_or(s1.len());
    format!("{n}\n").into_bytes()
}

fn check(impls: &Impls, s1: &[u8], s2: &[u8], ctx: &str) {
    let got = same_bytes_out(impls, s1, s2, ctx);
    assert_eq!(
        got,
        expected(s1, s2),
        "both libs agreed but disagree with the reference for {ctx} \
         (s1 len {}, s2 len {})",
        s1.len(),
        s2.len()
    );
}

// --- Row 1 -----------------------------------------------------------------
#[test]
fn row_01_empty_s2_empty_s1() {
    let impls = Impls::load();
    check(&impls, b"", b"", "row1 both empty");
}

// --- Row 2 -----------------------------------------------------------------
#[test]
fn row_02_empty_s2_all_simd_block_sizes() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 2);
    for len in [1, 15, 16, 17, 31, 32, 33, 63, 64, 65] {
        for _ in 0..8 {
            let s1 = rng.bytes(len, 0x20, 0x7E);
            check(&impls, &s1, b"", &format!("row2 len={len}"));
        }
    }
}

// --- Row 3 -----------------------------------------------------------------
#[test]
fn row_03_empty_s2_page_sized_s1() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 3);
    for len in [PAGE - 1, PAGE, PAGE + 1] {
        let s1 = rng.bytes(len, 0x21, 0x7E);
        check(&impls, &s1, b"", &format!("row3 len={len}"));
    }
}

// --- Row 4 -----------------------------------------------------------------
#[test]
fn row_04_empty_s2_one_mib_s1() {
    let impls = Impls::load();
    let s1 = vec![b'x'; 1024 * 1024];
    let out = same_bytes_out(&impls, &s1, b"", "row4 1MiB");
    assert_eq!(out, b"1048576\n");
}

// --- Row 5 -----------------------------------------------------------------
#[test]
fn row_05_single_reject_byte_no_match() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..64 {
        let len = rng.range(1, 65);
        // Draw s1 from 'a'..'y' and reject on 'z' so no match is possible.
        let s1 = rng.bytes(len, b'a', b'y');
        check(&impls, &s1, b"z", &format!("row5 len={len}"));
    }
}

// --- Row 6 -----------------------------------------------------------------
#[test]
fn row_06_match_at_index_zero() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..64 {
        let len = rng.range(1, 80);
        let mut s1 = rng.bytes(len, b'a', b'y');
        s1[0] = b'z';
        let out = same_bytes_out(&impls, &s1, b"z", "row6 match at 0");
        assert_eq!(out, b"0\n");
    }
}

// --- Row 7 -----------------------------------------------------------------
#[test]
fn row_07_match_inside_first_chunk() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..64 {
        let len = rng.range(17, 96);
        let mut s1 = rng.bytes(len, b'a', b'y');
        let idx = rng.below(16); // strictly inside the first 16-byte vector
        s1[idx] = b'z';
        let out = same_bytes_out(&impls, &s1, b"z", &format!("row7 idx={idx}"));
        assert_eq!(out, format!("{idx}\n").into_bytes());
    }
}

// --- Row 8 -----------------------------------------------------------------
#[test]
fn row_08_match_beyond_first_chunk() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..64 {
        let len = rng.range(33, 200);
        let mut s1 = rng.bytes(len, b'a', b'y');
        let idx = rng.range(16, len - 1);
        s1[idx] = b'z';
        // Guarantee idx is the FIRST occurrence.
        for b in s1[..idx].iter_mut() {
            if *b == b'z' {
                *b = b'a';
            }
        }
        let out = same_bytes_out(&impls, &s1, b"z", &format!("row8 idx={idx}"));
        assert_eq!(out, format!("{idx}\n").into_bytes());
    }
}

// --- Row 9 -----------------------------------------------------------------
#[test]
fn row_09_match_at_last_byte() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..64 {
        let len = rng.range(1, 65);
        let mut s1 = rng.bytes(len, b'a', b'y');
        *s1.last_mut().unwrap() = b'z';
        let out = same_bytes_out(&impls, &s1, b"z", &format!("row9 len={len}"));
        assert_eq!(out, format!("{}\n", len - 1).into_bytes());
    }
}

// --- Row 10 ----------------------------------------------------------------
#[test]
fn row_10_small_reject_set() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..128 {
        let s2_len = rng.range(2, 16);
        let s2 = rng.bytes(s2_len, b'a', b'z');
        let s1_len = rng.below(81);
        let s1 = rng.bytes(s1_len, b'a', b'z');
        check(&impls, &s1, &s2, &format!("row10 |s1|={s1_len} |s2|={s2_len}"));
    }
}

// --- Row 11 ----------------------------------------------------------------
#[test]
fn row_11_large_reject_set() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..128 {
        let s2_len = rng.range(17, 90);
        let s2 = rng.bytes(s2_len, b'A', b'z');
        let s1_len = rng.below(81);
        let s1 = rng.bytes(s1_len, b'A', b'z');
        check(&impls, &s1, &s2, &format!("row11 |s1|={s1_len} |s2|={s2_len}"));
    }
}

// --- Rows 12 & 13 ----------------------------------------------------------
fn all_255_bytes() -> Vec<u8> {
    (1u8..=255).collect()
}

#[test]
fn row_12_s2_every_non_nul_byte_nonempty_s1() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 12);
    let s2 = all_255_bytes();
    for _ in 0..64 {
        let len = rng.range(1, 64);
        let s1 = rng.bytes(len, 1, 255);
        let out = same_bytes_out(&impls, &s1, &s2, "row12");
        assert_eq!(out, b"0\n", "every byte is rejected, so the answer is 0");
    }
}

#[test]
fn row_13_s2_every_non_nul_byte_empty_s1() {
    let impls = Impls::load();
    let s2 = all_255_bytes();
    let out = same_bytes_out(&impls, b"", &s2, "row13");
    assert_eq!(out, b"0\n");
}

// --- Row 14 ----------------------------------------------------------------
#[test]
fn row_14_high_bytes_both_with_match() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..128 {
        let len = rng.range(1, 70);
        let mut s1 = rng.bytes(len, 0x80, 0xFF);
        let s2_len = rng_len(&mut rng);
        let s2 = rng.bytes(s2_len, 0x80, 0xFF);
        // Force a match somewhere so the comparison path is exercised.
        let idx = rng.below(len);
        s1[idx] = s2[0];
        check(&impls, &s1, &s2, &format!("row14 len={len} idx={idx}"));
    }
}

fn rng_len(rng: &mut Rng) -> usize {
    rng.range(1, 20)
}

// --- Row 15 ----------------------------------------------------------------
#[test]
fn row_15_high_bytes_s1_ascii_s2_no_match() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..64 {
        let len = rng.range(1, 70);
        let s1 = rng.bytes(len, 0x80, 0xFF);
        let s2 = rng.bytes(8, 0x20, 0x7E);
        let out = same_bytes_out(&impls, &s1, &s2, &format!("row15 len={len}"));
        assert_eq!(
            out,
            format!("{len}\n").into_bytes(),
            "high bytes must not compare equal to ASCII (sign-extension bug)"
        );
    }
}

// --- Row 16 ----------------------------------------------------------------
#[test]
fn row_16_full_byte_domain_property_sweep() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..512 {
        let len1 = rng.below(96);
        let len2 = rng.below(40);
        let s1 = rng.bytes(len1, 1, 255);
        let s2 = rng.bytes(len2, 1, 255);
        check(&impls, &s1, &s2, &format!("row16 case {i}"));
    }
}

// --- Row 17 ----------------------------------------------------------------
#[test]
fn row_17_alignment_cross_product() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 17);
    // Over-allocate so each string can start at an arbitrary byte offset.
    for off1 in 0..17usize {
        for off2 in 0..17usize {
            let len1 = rng.range(1, 48);
            let len2 = rng.range(1, 12);
            let body1 = rng.bytes(len1, b'a', b'z');
            let body2 = rng.bytes(len2, b'a', b'z');

            let mut buf1 = vec![0u8; off1];
            buf1.extend_from_slice(&body1);
            buf1.push(0);
            let mut buf2 = vec![0u8; off2];
            buf2.extend_from_slice(&body2);
            buf2.push(0);

            let p1 = unsafe { buf1.as_ptr().add(off1) } as *const c_char;
            let p2 = unsafe { buf2.as_ptr().add(off2) } as *const c_char;
            let ctx = format!("row17 off1={off1} off2={off2}");
            let c_out = capture_stdout(|| unsafe { (impls.c)(p1, p2) });
            let r_out = capture_stdout(|| unsafe { (impls.rust)(p1, p2) });
            assert_eq!(c_out, r_out, "{ctx}");
            assert_eq!(c_out, expected(&body1, &body2), "{ctx} vs reference");
        }
    }
}

// --- Rows 18 & 19 ----------------------------------------------------------
#[test]
fn row_18_aliased_pointers_nonempty() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..32 {
        let len = rng.range(1, 50);
        let buf = cstr(&rng.bytes(len, b'a', b'z'));
        let p = buf.as_ptr() as *const c_char;
        let c_out = capture_stdout(|| unsafe { (impls.c)(p, p) });
        let r_out = capture_stdout(|| unsafe { (impls.rust)(p, p) });
        assert_eq!(c_out, r_out, "row18 aliased len={len}");
        assert_eq!(c_out, b"0\n", "s1[0] is always in s2 when s1 == s2");
    }
}

#[test]
fn row_19_aliased_pointers_empty() {
    let impls = Impls::load();
    let buf = [0u8; 1];
    let p = buf.as_ptr() as *const c_char;
    let c_out = capture_stdout(|| unsafe { (impls.c)(p, p) });
    let r_out = capture_stdout(|| unsafe { (impls.rust)(p, p) });
    assert_eq!(c_out, r_out, "row19 aliased empty");
    assert_eq!(c_out, b"0\n");
}

// --- Row 20 ----------------------------------------------------------------
#[test]
fn row_20_overlapping_buffers() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..32 {
        let len = rng.range(4, 60);
        let body = rng.bytes(len, b'a', b'z');
        let buf = cstr(&body);
        let split = rng.range(1, len - 1);
        // s2 = whole buffer, s1 = suffix pointing into the same allocation.
        let p2 = buf.as_ptr() as *const c_char;
        let p1 = unsafe { buf.as_ptr().add(split) } as *const c_char;
        let ctx = format!("row20 len={len} split={split}");
        let c_out = capture_stdout(|| unsafe { (impls.c)(p1, p2) });
        let r_out = capture_stdout(|| unsafe { (impls.rust)(p1, p2) });
        assert_eq!(c_out, r_out, "{ctx}");
        assert_eq!(c_out, expected(&body[split..], &body), "{ctx} vs reference");
    }
}

// --- Row 21 ----------------------------------------------------------------
#[test]
fn row_21_printf_digit_widths() {
    let impls = Impls::load();
    for n in [0usize, 1, 9, 10, 99, 100, 999, 1000, 65535, 65536, 1048576] {
        // No match anywhere, so the printed result is exactly strlen(s1) == n.
        let s1 = vec![b'a'; n];
        let out = same_bytes_out(&impls, &s1, b"Z", &format!("row21 n={n}"));
        assert_eq!(
            out,
            format!("{n}\n").into_bytes(),
            "%zu must print unpadded decimal with no separators"
        );
    }
}

// --- Rows 22 & 23 ----------------------------------------------------------
#[test]
fn row_22_nul_on_final_byte_of_page() {
    let impls = Impls::load();
    // One readable page followed by a PROT_NONE guard page: the string fills
    // the page exactly, so any over-read past its NUL would fault.
    let base = guarded_region(1);
    unsafe {
        for i in 0..PAGE - 1 {
            *base.add(i) = b'a';
        }
        *base.add(PAGE - 1) = 0;
        let s2 = cstr(b"b");
        assert_same(
            &impls,
            base as *const c_char,
            s2.as_ptr() as *const c_char,
            "row22 NUL at final page byte",
        );
        let out = capture_stdout(|| (impls.c)(base as *const c_char, s2.as_ptr() as *const c_char));
        assert_eq!(out, format!("{}\n", PAGE - 1).into_bytes());
    }
}

#[test]
fn row_23_match_byte_on_page_boundary() {
    let impls = Impls::load();
    // Two readable pages + guard. Match byte sits on the last byte of page 0.
    let base = guarded_region(2);
    unsafe {
        for i in 0..PAGE - 1 {
            *base.add(i) = b'a';
        }
        *base.add(PAGE - 1) = b'b'; // the match, exactly on the boundary
        *base.add(PAGE) = 0;
        let s2 = cstr(b"b");
        assert_same(
            &impls,
            base as *const c_char,
            s2.as_ptr() as *const c_char,
            "row23 match on page boundary",
        );
        let out = capture_stdout(|| (impls.c)(base as *const c_char, s2.as_ptr() as *const c_char));
        assert_eq!(out, format!("{}\n", PAGE - 1).into_bytes());
    }
}

// --- Rows 24-26 ------------------------------------------------------------
#[test]
fn row_24_duplicate_bytes_in_s2() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..64 {
        let dup = rng.byte_in(b'a', b'z');
        let s2_len = rng.range(1, 20);
        let s2 = vec![dup; s2_len];
        let s1_len = rng.range(0, 60);
        let s1 = rng.bytes(s1_len, b'a', b'z');
        check(&impls, &s1, &s2, "row24 duplicate reject bytes");
    }
}

#[test]
fn row_25_repeated_byte_s1_matching_s2() {
    let impls = Impls::load();
    for len in [1usize, 2, 15, 16, 17, 63, 64, 65, 1000] {
        let s1 = vec![b'q'; len];
        let out = same_bytes_out(&impls, &s1, b"q", &format!("row25 len={len}"));
        assert_eq!(out, b"0\n");
    }
}

#[test]
fn row_26_repeated_byte_s1_non_matching_s2() {
    let impls = Impls::load();
    for len in [1usize, 2, 15, 16, 17, 63, 64, 65, 1000] {
        let s1 = vec![b'q'; len];
        let out = same_bytes_out(&impls, &s1, b"r", &format!("row26 len={len}"));
        assert_eq!(out, format!("{len}\n").into_bytes());
    }
}

// --- Row 27 ----------------------------------------------------------------
#[test]
fn row_27_repeated_invocations_carry_no_state() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 27);
    // Interleave many calls in one process and additionally check that a long
    // run of C calls followed by a long run of Rust calls yields identical
    // concatenated stdout (i.e. neither buffers or accumulates anything).
    let cases: Vec<(Vec<u8>, Vec<u8>)> = (0..64)
        .map(|_| {
            let (l1, l2) = (rng.below(40), rng.below(8));
            (cstr(&rng.bytes(l1, b'a', b'z')), cstr(&rng.bytes(l2, b'a', b'z')))
        })
        .collect();

    let c_all = capture_stdout(|| {
        for (a, b) in &cases {
            unsafe { (impls.c)(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char) }
        }
    });
    let r_all = capture_stdout(|| {
        for (a, b) in &cases {
            unsafe { (impls.rust)(a.as_ptr() as *const c_char, b.as_ptr() as *const c_char) }
        }
    });
    assert_eq!(c_all, r_all, "row27 batched stdout differs");
    assert_eq!(
        c_all.iter().filter(|&&b| b == b'\n').count(),
        cases.len(),
        "one line per call, no extra or missing newlines"
    );
}

// --- Row 28 ----------------------------------------------------------------
#[test]
fn row_28_unrestricted_property_sweep() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED);
    for i in 0..2000 {
        // Random shape across every axis at once, including alignment offsets.
        let len1 = rng.below(100);
        let len2 = rng.below(24);
        let (lo, hi) = match rng.below(4) {
            0 => (0x20u8, 0x7Eu8),  // ASCII printable
            1 => (0x80, 0xFF),      // high bytes only
            2 => (0x01, 0xFF),      // full non-NUL domain
            _ => (b'a', b'e'),      // tiny alphabet -> frequent matches
        };
        let body1 = rng.bytes(len1, lo, hi);
        let body2 = rng.bytes(len2, lo, hi);
        let off1 = rng.below(9);
        let off2 = rng.below(9);

        let mut buf1 = vec![0xEEu8; off1];
        buf1.extend_from_slice(&body1);
        buf1.push(0);
        let mut buf2 = vec![0xEEu8; off2];
        buf2.extend_from_slice(&body2);
        buf2.push(0);

        let p1 = unsafe { buf1.as_ptr().add(off1) } as *const c_char;
        let p2 = unsafe { buf2.as_ptr().add(off2) } as *const c_char;
        let c_out = capture_stdout(|| unsafe { (impls.c)(p1, p2) });
        let r_out = capture_stdout(|| unsafe { (impls.rust)(p1, p2) });
        assert_eq!(
            c_out, r_out,
            "row28 case {i}: len1={len1} len2={len2} off1={off1} off2={off2} domain={lo:#x}..{hi:#x}"
        );
        assert_eq!(c_out, expected(&body1, &body2), "row28 case {i} vs reference");
    }
}
