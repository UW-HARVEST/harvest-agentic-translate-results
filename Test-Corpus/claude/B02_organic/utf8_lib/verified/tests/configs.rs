//! Phase B -- valid-path differential tests.
//!
//! One test (or one clearly-labelled block) per row of `CONFIGS.md`.  Every test
//! drives BOTH the C `.so` and the Rust `.so` through `libloading` and asserts
//! byte-identical results.

mod common;

use common::*;

// ===========================================================================
// row 1 -- w_utf8_drop, empty string
// ===========================================================================
#[test]
fn row01_drop_empty() {
    assert_eq!(cmp_drop(&cstr(b"")), 0);
}

// ===========================================================================
// row 2 -- w_utf8_drop, pure ASCII
// ===========================================================================
#[test]
fn row02_drop_ascii() {
    for b in 1u8..0x80 {
        assert_eq!(cmp_drop(&cstr(&[b])), 1);
    }
    let mut r = Rng::new(0x0202_0202_0000_0001);
    for _ in 0..4000 {
        let n = r.below(65) as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            push_valid1(&mut v, &mut r);
        }
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), n);
    }
}

// ===========================================================================
// row 3 -- w_utf8_drop, exhaustive all 256 single-byte strings
// ===========================================================================
#[test]
fn row03_drop_all_1byte() {
    for b in 0u8..=0xFF {
        cmp_drop(&cstr(&[b]));
    }
}

// ===========================================================================
// row 4 -- w_utf8_drop, exhaustive all 65536 two-byte strings
// ===========================================================================
#[test]
fn row04_drop_all_2byte() {
    let mut buf = [0u8; 3];
    for b0 in 0u8..=0xFF {
        for b1 in 0u8..=0xFF {
            buf[0] = b0;
            buf[1] = b1;
            buf[2] = 0;
            cmp_drop(&buf);
        }
    }
}

// ===========================================================================
// row 5 -- w_utf8_drop, exhaustive 3-byte strings, leads 0xE0..0xEF
// ===========================================================================
#[test]
fn row05_drop_all_3byte_e0_ef() {
    let mut buf = [0u8; 4];
    for lead in 0xE0u8..=0xEF {
        for b1 in 0u8..=0xFF {
            for b2 in 0u8..=0xFF {
                buf[0] = lead;
                buf[1] = b1;
                buf[2] = b2;
                buf[3] = 0;
                cmp_drop(&buf);
            }
        }
    }
}

// ===========================================================================
// row 6 -- w_utf8_drop, exhaustive 4-byte strings, leads 0xF0..0xF7
// ===========================================================================
#[test]
fn row06_drop_4byte_f0_f7() {
    let sample: [u8; 16] = [
        0x00, 0x01, 0x41, 0x7F, 0x80, 0x81, 0x8F, 0x90, 0x9F, 0xA0, 0xBE, 0xBF, 0xC0, 0xE0, 0xF0,
        0xFF,
    ];
    let mut buf = [0u8; 5];
    for lead in 0xF0u8..=0xF7 {
        for b1 in 0u8..=0xFF {
            for &b2 in &sample {
                for &b3 in &sample {
                    buf[0] = lead;
                    buf[1] = b1;
                    buf[2] = b2;
                    buf[3] = b3;
                    buf[4] = 0;
                    cmp_drop(&buf);
                }
            }
        }
    }
}

// ===========================================================================
// row 7 -- w_utf8_drop, well-formed 2-byte sequences only
// ===========================================================================
#[test]
fn row07_drop_valid2_only() {
    // exhaustive over every well-formed 2-byte character
    for lead in 0xC2u8..=0xDF {
        for cont in 0x80u8..=0xBF {
            assert_eq!(cmp_drop(&cstr(&[lead, cont])), 2);
        }
    }
    let mut r = Rng::new(0x0707_0707_0000_0007);
    for _ in 0..3000 {
        let n = r.below(33) as usize;
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid2(&mut v, &mut r);
        }
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), 2 * n);
    }
}

// ===========================================================================
// row 8 -- w_utf8_drop, well-formed 3-byte sequences only + boundaries
// ===========================================================================
#[test]
fn row08_drop_valid3_only() {
    for seq in [
        [0xE0u8, 0xA0, 0x80],
        [0xE0, 0xBF, 0xBF],
        [0xE1, 0x80, 0x80],
        [0xEC, 0xBF, 0xBF],
        [0xED, 0x80, 0x80],
        [0xED, 0x9F, 0xBF],
        [0xEE, 0x80, 0x80],
        [0xEF, 0xBF, 0xBF],
        [0xEF, 0xBF, 0xBD],
    ] {
        assert_eq!(cmp_drop(&cstr(&seq)), 3, "seq {}", hex(&seq));
    }
    let mut r = Rng::new(0x0808_0808_0000_0008);
    for _ in 0..3000 {
        let n = r.below(25) as usize;
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid3(&mut v, &mut r);
        }
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), 3 * n);
    }
}

// ===========================================================================
// row 9 -- w_utf8_drop, well-formed 4-byte sequences only + boundaries
// ===========================================================================
#[test]
fn row09_drop_valid4_only() {
    for seq in [
        [0xF0u8, 0x90, 0x80, 0x80],
        [0xF0, 0xBF, 0xBF, 0xBF],
        [0xF1, 0x80, 0x80, 0x80],
        [0xF3, 0xBF, 0xBF, 0xBF],
        [0xF4, 0x80, 0x80, 0x80],
        [0xF4, 0x8F, 0xBF, 0xBF],
    ] {
        assert_eq!(cmp_drop(&cstr(&seq)), 4, "seq {}", hex(&seq));
    }
    let mut r = Rng::new(0x0909_0909_0000_0009);
    for _ in 0..3000 {
        let n = r.below(17) as usize;
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid4(&mut v, &mut r);
        }
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), 4 * n);
    }
}

// ===========================================================================
// row 10 -- w_utf8_drop, mixed well-formed 1/2/3/4-byte
// ===========================================================================
#[test]
fn row10_drop_mixed_valid() {
    let mut r = Rng::new(0x1010_1010_0000_0010);
    for _ in 0..8000 {
        let n = r.below(65) as usize;
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid_any(&mut v, &mut r);
        }
        let want = v.len();
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), want, "input {}", hex(&buf));
    }
}

// ===========================================================================
// row 11 -- w_utf8_drop, uniformly random bytes
// ===========================================================================
#[test]
fn row11_drop_uniform_random() {
    let mut r = Rng::new(0x1111_1111_0000_0011);
    for _ in 0..20000 {
        let n = r.below(257) as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(r.nonzero_byte());
        }
        cmp_drop(&cstr(&v));
    }
}

// ===========================================================================
// row 12 -- w_utf8_drop, biased random (lead/continuation heavy)
// ===========================================================================
fn biased_byte(r: &mut Rng) -> u8 {
    match r.below(10) {
        0 => 0x80 + r.below(0x40) as u8,      // continuation
        1 => 0xC0 + r.below(0x20) as u8,      // 2-byte lead (incl. overlong)
        2 => 0xE0 + r.below(0x10) as u8,      // 3-byte lead
        3 => 0xF0 + r.below(0x10) as u8,      // 4-byte lead (incl. F5..FF)
        4 => 0xF8 + r.below(0x08) as u8,      // never-valid lead
        5 => 1 + r.below(0x7F) as u8,         // ascii
        6 => [0xE0, 0xED, 0xEF, 0xF0, 0xF4, 0xC0, 0xC1, 0xC2][r.below(8) as usize],
        7 => [0x8F, 0x90, 0x9F, 0xA0, 0xBF, 0xC0, 0x7F, 0x01][r.below(8) as usize],
        _ => r.nonzero_byte(),
    }
}

#[test]
fn row12_drop_biased_random() {
    let mut r = Rng::new(0x1212_1212_0000_0012);
    for _ in 0..20000 {
        let n = r.below(257) as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(biased_byte(&mut r));
        }
        cmp_drop(&cstr(&v));
    }
}

// ===========================================================================
// row 13 -- w_utf8_drop, truncated trailing sequence
// ===========================================================================
#[test]
fn row13_drop_truncated_tail() {
    let mut r = Rng::new(0x1313_1313_0000_0013);
    for _ in 0..6000 {
        let prefix_chars = r.below(9) as usize;
        let mut v = Vec::new();
        for _ in 0..prefix_chars {
            push_valid_any(&mut v, &mut r);
        }
        let prefix_len = v.len();

        // append one well-formed char, then chop 1..width-1 of its bytes
        let mut tail = Vec::new();
        let width = 2 + r.below(3) as usize; // 2, 3 or 4
        match width {
            2 => push_valid2(&mut tail, &mut r),
            3 => push_valid3(&mut tail, &mut r),
            _ => push_valid4(&mut tail, &mut r),
        }
        let keep = 1 + r.below(width as u32 - 1) as usize; // 1..width-1
        v.extend_from_slice(&tail[..keep]);

        let buf = cstr(&v);
        assert_eq!(
            cmp_drop(&buf),
            prefix_len,
            "truncated tail should be rejected at the lead byte: {}",
            hex(&buf)
        );
    }
}

// ===========================================================================
// row 14 -- w_utf8_drop, bytes after the NUL terminator
// ===========================================================================
#[test]
fn row14_drop_after_nul() {
    let mut r = Rng::new(0x1414_1414_0000_0014);
    for _ in 0..5000 {
        let n = r.below(33) as usize;
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid_any(&mut v, &mut r);
        }
        let visible = v.len();
        v.push(0); // terminator
        let junk = r.below(17) as usize;
        for _ in 0..junk {
            v.push(biased_byte(&mut r));
        }
        v.push(0); // keep the whole buffer NUL-terminated for cmp_drop's sanity check
        assert_eq!(cmp_drop(&v), visible, "buffer {}", hex(&v));
    }
}

// ===========================================================================
// row 15 -- w_utf8_drop, long all-valid input
// ===========================================================================
#[test]
fn row15_drop_long_valid() {
    let mut r = Rng::new(0x1515_1515_0000_0015);
    for _ in 0..20 {
        let mut v = Vec::new();
        while v.len() < 40_000 {
            push_valid_any(&mut v, &mut r);
        }
        let want = v.len();
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), want);
    }
}

// ===========================================================================
// rows 16..19 -- w_utf8_filter, strdup path (input already fully valid)
// ===========================================================================
#[test]
fn row16_17_filter_empty_both_flags() {
    let buf = cstr(b"");
    for repl in [0u32, 1] {
        let out = cmp_filter(&buf, repl);
        assert!(!out.null);
        assert!(out.bytes.is_empty());
    }
}

#[test]
fn row18_19_filter_valid_strdup_path() {
    let mut r = Rng::new(0x1819_1819_0000_0018);
    for _ in 0..4000 {
        let n = r.below(49) as usize;
        let mut v = Vec::new();
        for _ in 0..n {
            push_valid_any(&mut v, &mut r);
        }
        let buf = cstr(&v);
        for repl in [0u32, 1] {
            let out = cmp_filter(&buf, repl);
            assert!(!out.null);
            assert_eq!(out.bytes, v, "strdup path must copy verbatim");
        }
    }
}

// ===========================================================================
// rows 20..25 -- w_utf8_filter, invalid byte at offset 0 / middle / end
// ===========================================================================

/// Build `prefix_chars` well-formed chars, an invalid byte, then `suffix_chars`
/// well-formed chars.
fn with_invalid_at(r: &mut Rng, prefix_chars: usize, suffix_chars: usize) -> (Vec<u8>, usize) {
    let mut v = Vec::new();
    for _ in 0..prefix_chars {
        push_valid_any(&mut v, r);
    }
    let at = v.len();
    v.push(invalid_byte(r));
    for _ in 0..suffix_chars {
        push_valid_any(&mut v, r);
    }
    (v, at)
}

#[test]
fn row20_21_filter_invalid_at_offset_zero() {
    let mut r = Rng::new(0x2021_2021_0000_0020);
    for _ in 0..4000 {
        let suf = r.below(17) as usize;
        let (v, at) = with_invalid_at(&mut r, 0, suf);
        assert_eq!(at, 0);
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), 0);
        for repl in [0u32, 1] {
            cmp_filter(&buf, repl);
        }
    }
    // every "always invalid" byte, at offset 0, alone
    for &b in ALWAYS_INVALID_LEADS {
        let buf = cstr(&[b]);
        assert_eq!(cmp_drop(&buf), 0);
        for repl in [0u32, 1] {
            cmp_filter(&buf, repl);
        }
    }
}

#[test]
fn row22_23_filter_invalid_in_middle() {
    let mut r = Rng::new(0x2223_2223_0000_0022);
    for _ in 0..6000 {
        let pre = 1 + r.below(16) as usize;
        let suf = 1 + r.below(16) as usize;
        let (v, at) = with_invalid_at(&mut r, pre, suf);
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), at);
        for repl in [0u32, 1] {
            cmp_filter(&buf, repl);
        }
    }
}

#[test]
fn row24_25_filter_invalid_at_end() {
    let mut r = Rng::new(0x2425_2425_0000_0024);
    for _ in 0..6000 {
        // invalid byte is the last byte
        let pre = 1 + r.below(16) as usize;
        let (v, at) = with_invalid_at(&mut r, pre, 0);
        let buf = cstr(&v);
        assert_eq!(cmp_drop(&buf), at);
        for repl in [0u32, 1] {
            cmp_filter(&buf, repl);
        }

        // truncated multi-byte char at the end
        let mut w = Vec::new();
        for _ in 0..(1 + r.below(8)) {
            push_valid_any(&mut w, &mut r);
        }
        let mut tail = Vec::new();
        let width = 2 + r.below(3) as usize;
        match width {
            2 => push_valid2(&mut tail, &mut r),
            3 => push_valid3(&mut tail, &mut r),
            _ => push_valid4(&mut tail, &mut r),
        }
        let keep = 1 + r.below(width as u32 - 1) as usize;
        w.extend_from_slice(&tail[..keep]);
        let buf = cstr(&w);
        for repl in [0u32, 1] {
            cmp_filter(&buf, repl);
        }
    }
}

// ===========================================================================
// rows 26..27 -- w_utf8_filter, exhaustive all-256 1-byte strings
// ===========================================================================
#[test]
fn row26_27_filter_all_1byte() {
    for b in 0u8..=0xFF {
        let buf = cstr(&[b]);
        for repl in [0u32, 1] {
            cmp_filter(&buf, repl);
        }
    }
}

// ===========================================================================
// rows 28..29 -- w_utf8_filter, exhaustive all-65536 2-byte strings
// ===========================================================================
#[test]
fn row28_29_filter_all_2byte() {
    let mut buf = [0u8; 3];
    for b0 in 0u8..=0xFF {
        for b1 in 0u8..=0xFF {
            buf[0] = b0;
            buf[1] = b1;
            buf[2] = 0;
            cmp_filter(&buf, 0);
            cmp_filter(&buf, 1);
        }
    }
}

// ===========================================================================
// rows 30..31 -- w_utf8_filter, 3-byte strings over every multi-byte lead
// ===========================================================================
#[test]
fn row30_31_filter_3byte_leads() {
    let sample: [u8; 12] = [
        0x00, 0x01, 0x41, 0x7F, 0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF, 0xC0, 0xFF,
    ];
    let mut buf = [0u8; 4];
    for lead in 0xC0u8..=0xFF {
        for b1 in 0u8..=0xFF {
            for &b2 in &sample {
                buf[0] = lead;
                buf[1] = b1;
                buf[2] = b2;
                buf[3] = 0;
                cmp_drop(&buf);
                cmp_filter(&buf, 0);
                cmp_filter(&buf, 1);
            }
        }
    }
}

// ===========================================================================
// rows 32..33 -- w_utf8_filter, uniform random bytes
// ===========================================================================
#[test]
fn row32_33_filter_uniform_random() {
    let mut r = Rng::new(0x3233_3233_0000_0032);
    for _ in 0..12000 {
        let n = r.below(257) as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(r.nonzero_byte());
        }
        let buf = cstr(&v);
        cmp_drop(&buf);
        cmp_filter(&buf, 0);
        cmp_filter(&buf, 1);
    }
}

// ===========================================================================
// rows 34..35 -- w_utf8_filter, biased random (valid chars XOR invalid bytes)
// ===========================================================================
#[test]
fn row34_35_filter_biased_random() {
    let mut r = Rng::new(0x3435_3435_0000_0034);
    for _ in 0..6000 {
        let target = r.below(1025) as usize;
        let mut v = Vec::new();
        while v.len() < target {
            if r.below(3) == 0 {
                v.push(biased_byte(&mut r));
            } else {
                push_valid_any(&mut v, &mut r);
            }
        }
        let buf = cstr(&v);
        cmp_drop(&buf);
        cmp_filter(&buf, 0);
        cmp_filter(&buf, 1);
    }
}

// ===========================================================================
// rows 36..37 -- REPLACEMENT_INC / `repl < 3` accounting boundary
// ===========================================================================
#[test]
fn row36_37_repl_threshold_boundary() {
    // n invalid bytes => n replacements; realloc is triggered on replacement
    // #1, #1366, #2731, ... (4096 / 3 = 1365.33)
    let counts: [usize; 17] = [
        0, 1, 2, 3, 4, 1364, 1365, 1366, 1367, 2729, 2730, 2731, 2732, 4094, 4095, 4096, 4097,
    ];
    for &n in &counts {
        let mut r = Rng::new(0x3637_0000_0000_0000 ^ n as u64);
        // pure invalid run
        let mut v = vec![0x80u8; n];
        for b in v.iter_mut() {
            *b = invalid_byte(&mut r);
        }
        let buf = cstr(&v);
        let o0 = cmp_filter(&buf, 0);
        let o1 = cmp_filter(&buf, 1);
        assert!(!o0.null && !o1.null);
        assert_eq!(o0.bytes.len(), 0, "n={n}: all bytes dropped");
        assert_eq!(o1.bytes.len(), 3 * n, "n={n}: 3 bytes per replacement");
        for c in o1.bytes.chunks(3) {
            assert_eq!(c, [0xEF, 0xBF, 0xBD]);
        }
        // exact differential check of the internal `size` / `repl` accounting
        cmp_filter_alloc_size(&buf, 0);
        cmp_filter_alloc_size(&buf, 1);

        // invalid bytes interleaved with well-formed characters
        let mut w = Vec::new();
        for _ in 0..n {
            push_valid_any(&mut w, &mut r);
            w.push(invalid_byte(&mut r));
        }
        push_valid_any(&mut w, &mut r);
        let buf = cstr(&w);
        cmp_drop(&buf);
        cmp_filter(&buf, 0);
        cmp_filter(&buf, 1);
        cmp_filter_alloc_size(&buf, 0);
        cmp_filter_alloc_size(&buf, 1);
    }
}

// ===========================================================================
// row 38 -- many realloc rounds
// ===========================================================================
#[test]
fn row38_filter_many_realloc_rounds() {
    let mut r = Rng::new(0x3838_3838_0000_0038);
    for &n in &[10_000usize, 20_000, 50_000] {
        let mut v = Vec::new();
        for i in 0..n {
            if i % 4 == 0 {
                push_valid_any(&mut v, &mut r);
            }
            v.push(invalid_byte(&mut r));
        }
        let buf = cstr(&v);
        cmp_drop(&buf);
        cmp_filter(&buf, 0);
        cmp_filter(&buf, 1);
        // dozens of realloc rounds: the accumulated `size` is a sharp
        // fingerprint of the REPLACEMENT_INC / `repl < 3` schedule
        cmp_filter_alloc_size(&buf, 1);
    }
}

// ===========================================================================
// row 46 -- internal allocation schedule (`size`, `repl`, REPLACEMENT_INC)
// ===========================================================================
#[test]
fn row46_allocation_schedule() {
    let mut r = Rng::new(0x4646_4646_0000_0046);
    // sweep the number of replacements across several REPLACEMENT_INC periods
    let mut counts: Vec<usize> = vec![0, 1, 2, 3, 1365, 1366, 2730, 2731, 4095, 4096];
    for k in 1..=8usize {
        counts.push(k * 1365);
        counts.push(k * 1365 + 1);
        counts.push(k * 4096);
    }
    for &n in &counts {
        let mut v = vec![0u8; n];
        for b in v.iter_mut() {
            *b = invalid_byte(&mut r);
        }
        let buf = cstr(&v);
        for repl in [0u32, 1] {
            cmp_filter(&buf, repl);
            cmp_filter_alloc_size(&buf, repl);
        }
    }
    // strdup path: the allocation is exactly strlen+1 in both implementations
    for _ in 0..40 {
        let mut v = Vec::new();
        for _ in 0..r.below(200) {
            push_valid_any(&mut v, &mut r);
        }
        let buf = cstr(&v);
        cmp_filter_alloc_size(&buf, 0);
        cmp_filter_alloc_size(&buf, 1);
    }
}

// ===========================================================================
// rows 39..40 -- non-normalized `_Bool` across the FFI boundary
// ===========================================================================
#[test]
fn row39_40_non_normalized_bool() {
    let mut r = Rng::new(0x3940_3940_0000_0039);
    // low byte != 0 => true ; low byte == 0 => false
    let truthy: [u32; 8] = [1, 2, 3, 0x7F, 0x80, 0xFF, 0x1234_5601, 0xFFFF_FFFF];
    let falsy: [u32; 6] = [0, 0x100, 0xFF00, 0x1234_5600, 0xFFFF_FF00, 0x8000_0000];

    for _ in 0..600 {
        let mut v = Vec::new();
        for _ in 0..(1 + r.below(12)) {
            push_valid_any(&mut v, &mut r);
            v.push(invalid_byte(&mut r));
        }
        let buf = cstr(&v);

        let reference_true = cmp_filter(&buf, 1).bytes;
        let reference_false = cmp_filter(&buf, 0).bytes;
        assert_ne!(reference_true, reference_false);

        for &t in &truthy {
            let out = cmp_filter(&buf, t);
            assert_eq!(
                out.bytes, reference_true,
                "replacement={t:#x} (low byte {:#x}) must behave as true",
                t & 0xFF
            );
        }
        for &f in &falsy {
            let out = cmp_filter(&buf, f);
            assert_eq!(
                out.bytes, reference_false,
                "replacement={f:#x} (low byte 0) must behave as false"
            );
        }
    }
}

// ===========================================================================
// row 41 -- w_utf8_filter, bytes after the NUL terminator
// ===========================================================================
#[test]
fn row41_filter_after_nul() {
    let mut r = Rng::new(0x4141_4141_0000_0041);
    for _ in 0..4000 {
        let mut v = Vec::new();
        for _ in 0..r.below(17) {
            push_valid_any(&mut v, &mut r);
            if r.below(2) == 0 {
                v.push(invalid_byte(&mut r));
            }
        }
        let visible: Vec<u8> = v.clone();
        v.push(0);
        for _ in 0..r.below(17) {
            v.push(biased_byte(&mut r));
        }
        v.push(0);

        // the reference result computed from the visible prefix only
        let vis_buf = cstr(&visible);
        for repl in [0u32, 1] {
            let want = cmp_filter(&vis_buf, repl).bytes;
            let got = cmp_filter(&v, repl).bytes;
            assert_eq!(got, want, "bytes past the NUL must be ignored");
        }
    }
}

// ===========================================================================
// rows 42..43 -- long inputs
// ===========================================================================
#[test]
fn row42_filter_long_all_valid() {
    let mut r = Rng::new(0x4242_4242_0000_0042);
    for _ in 0..10 {
        let mut v = Vec::new();
        while v.len() < 40_000 {
            push_valid_any(&mut v, &mut r);
        }
        let buf = cstr(&v);
        for repl in [0u32, 1] {
            let out = cmp_filter(&buf, repl);
            assert_eq!(out.bytes, v);
        }
    }
}

#[test]
fn row43_filter_long_scattered_invalid() {
    let mut r = Rng::new(0x4343_4343_0000_0043);
    for _ in 0..10 {
        let mut v = Vec::new();
        while v.len() < 40_000 {
            if r.below(20) == 0 {
                v.push(biased_byte(&mut r));
            } else {
                push_valid_any(&mut v, &mut r);
            }
        }
        let buf = cstr(&v);
        cmp_drop(&buf);
        cmp_filter(&buf, 0);
        cmp_filter(&buf, 1);
    }
}

// ===========================================================================
// row 44 -- composed pipeline: drop(filter(s))
// ===========================================================================
#[test]
fn row44_composed_drop_of_filter() {
    let p = pair();
    let mut r = Rng::new(0x4444_4444_0000_0044);
    for _ in 0..4000 {
        let n = r.below(129) as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(biased_byte(&mut r));
        }
        let buf = cstr(&v);
        for repl in [0u32, 1] {
            let filtered = cmp_filter(&buf, repl);
            assert!(!filtered.null);
            // feed the filtered output back through both scanners: the result
            // must be fully valid, i.e. drop() reports the terminator.
            let again = cstr(&filtered.bytes);
            let off = cmp_drop(&again);
            assert_eq!(
                off,
                filtered.bytes.len(),
                "filter output should be fully valid: {}",
                hex(&filtered.bytes)
            );
            // sanity: the raw C/Rust scanners agree on the exact same pointer
            let base = again.as_ptr() as *const std::ffi::c_char;
            let a = unsafe { (p.c.utf8_drop)(base) };
            let b = unsafe { (p.rs.utf8_drop)(base) };
            assert_eq!(a, b as *const _);
        }
    }
}

// ===========================================================================
// row 45 -- idempotence of filter, compared C vs Rust
// ===========================================================================
#[test]
fn row45_filter_idempotent() {
    let mut r = Rng::new(0x4545_4545_0000_0045);
    for _ in 0..4000 {
        let n = r.below(129) as usize;
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(biased_byte(&mut r));
        }
        let buf = cstr(&v);
        for repl in [0u32, 1] {
            let once = cmp_filter(&buf, repl);
            let b2 = cstr(&once.bytes);
            let twice = cmp_filter(&b2, repl);
            assert_eq!(once.bytes, twice.bytes, "filter must be idempotent");
        }
    }
}

// ===========================================================================
// row 47 -- no read past the NUL terminator (guard page)
// ===========================================================================
//
// Every input is placed so that its terminating NUL is the last readable byte
// before a PROT_NONE page.  A single byte of over-read (e.g. evaluating a
// continuation-byte check that C short-circuits away) is turned into a SIGSEGV
// that kills the test binary, instead of silently returning a plausible answer.
#[test]
fn row47_guard_page_no_overread() {
    let g = GuardedBuf::new();

    // all 1-byte inputs
    for b in 0u8..=0xFF {
        let s = g.place(&[b]);
        cmp_drop(s);
        cmp_filter(s, 0);
        cmp_filter(s, 1);
    }
    // all 2-byte inputs
    for b0 in 0u8..=0xFF {
        for b1 in 0u8..=0xFF {
            let s = g.place(&[b0, b1]);
            cmp_drop(s);
            cmp_filter(s, 0);
            cmp_filter(s, 1);
        }
    }
    // 3-byte inputs over every multi-byte lead
    let sample: [u8; 10] = [0x00, 0x01, 0x7F, 0x80, 0x8F, 0x90, 0x9F, 0xA0, 0xBF, 0xFF];
    for lead in 0xC0u8..=0xFF {
        for b1 in 0u8..=0xFF {
            for &b2 in &sample {
                let s = g.place(&[lead, b1, b2]);
                cmp_drop(s);
                cmp_filter(s, 1);
            }
        }
    }
    // 4-byte inputs over every 4-byte-ish lead
    for lead in 0xF0u8..=0xF7 {
        for &b1 in &sample {
            for &b2 in &sample {
                for &b3 in &sample {
                    let s = g.place(&[lead, b1, b2, b3]);
                    cmp_drop(s);
                    cmp_filter(s, 1);
                }
            }
        }
    }
    // truncated well-formed sequences pressed right against the guard page
    let mut r = Rng::new(0x4747_4747_0000_0047);
    for _ in 0..20000 {
        let mut v = Vec::new();
        for _ in 0..r.below(12) {
            push_valid_any(&mut v, &mut r);
        }
        let mut tail = Vec::new();
        match r.below(3) {
            0 => push_valid2(&mut tail, &mut r),
            1 => push_valid3(&mut tail, &mut r),
            _ => push_valid4(&mut tail, &mut r),
        }
        let keep = 1 + r.below(tail.len() as u32) as usize;
        v.extend_from_slice(&tail[..keep]);
        let s = g.place(&v);
        cmp_drop(s);
        cmp_filter(s, 0);
        cmp_filter(s, 1);
    }
}
