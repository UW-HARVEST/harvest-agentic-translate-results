//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH `.so` files via `libloading` and compares their
//! observable output byte-for-byte. Randomized rows use a fixed seed so a
//! failure is always reproducible.

mod common;

use common::{pair, reference, Outcome, Rng};

/// Build a `len`-byte buffer and compare C vs Rust in explicit-length mode.
fn check_explicit(bytes: &[u8], ctx: &str) {
    pair().assert_same(bytes.len() as i32, Some(bytes), ctx);
}

// ---------------------------------------------------------------------------
// Rows 1-3: size == len, one row per `len % 3` class (padding branches).
// ---------------------------------------------------------------------------

fn multigroup_class(residue: usize, seed: u64, ctx: &str) {
    let mut rng = Rng::new(seed);
    for group_count in 1..=40usize {
        let len = group_count * 3 + residue;
        for _ in 0..8 {
            let bytes = rng.ascii(len);
            check_explicit(&bytes, ctx);
        }
    }
}

#[test]
fn cfg_row1_explicit_len_mod3_is_0_ascii() {
    multigroup_class(0, 0x1111_1111, "row1 len%3==0 ascii");
}

#[test]
fn cfg_row2_explicit_len_mod3_is_1_double_padding() {
    multigroup_class(1, 0x2222_2222, "row2 len%3==1 ==");
    // Assert the padding actually appears, so the row is meaningful.
    let out = pair().rs.call(4, Some(b"abcd"));
    match out {
        Outcome::Str(s) => assert!(
            s.ends_with(b"=="),
            "len%3==1 must produce '==' padding, got {:?}",
            String::from_utf8_lossy(&s)
        ),
        Outcome::Null => panic!("unexpected NULL"),
    }
}

#[test]
fn cfg_row3_explicit_len_mod3_is_2_single_padding() {
    multigroup_class(2, 0x3333_3333, "row3 len%3==2 =");
    let out = pair().rs.call(5, Some(b"abcde"));
    match out {
        Outcome::Str(s) => {
            assert!(
                s.ends_with(b"=") && !s.ends_with(b"=="),
                "len%3==2 must produce a single '=' pad, got {:?}",
                String::from_utf8_lossy(&s)
            );
        }
        Outcome::Null => panic!("unexpected NULL"),
    }
}

// ---------------------------------------------------------------------------
// Rows 4-6: the three smallest lengths, exhaustive over byte values.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row4_len_1_all_byte_values() {
    for b in 0u8..=255 {
        check_explicit(&[b], "row4 len=1 exhaustive");
    }
}

#[test]
fn cfg_row5_len_2_randomized_full_range() {
    let mut rng = Rng::new(0x5555_5555);
    // Exhaustive on the first byte, randomized on the second, plus corners.
    for b0 in 0u8..=255 {
        check_explicit(&[b0, rng.byte()], "row5 len=2");
        check_explicit(&[b0, 0x00], "row5 len=2 lo");
        check_explicit(&[b0, 0xFF], "row5 len=2 hi");
    }
}

#[test]
fn cfg_row6_len_3_one_complete_group() {
    let mut rng = Rng::new(0x6666_6666);
    for _ in 0..4000 {
        let bytes = rng.bytes(3);
        check_explicit(&bytes, "row6 len=3 random");
    }
    for corner in [[0u8, 0, 0], [0xFF, 0xFF, 0xFF], [0, 0xFF, 0], [0xFF, 0, 0xFF]] {
        check_explicit(&corner, "row6 len=3 corner");
    }
}

// ---------------------------------------------------------------------------
// Row 7: full byte range 0x00..=0xFF (signed-char / high-bit axis).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row7_explicit_full_byte_range_all_residues() {
    let mut rng = Rng::new(0x7777_7777);
    for len in 1..=64usize {
        for _ in 0..40 {
            let bytes = rng.bytes(len);
            check_explicit(&bytes, "row7 full byte range");
        }
        // Guarantee high-bit bytes are present at least once per length.
        let mut hi = rng.bytes(len);
        for b in hi.iter_mut() {
            *b |= 0x80;
        }
        check_explicit(&hi, "row7 all high-bit");
    }
}

// ---------------------------------------------------------------------------
// Row 8: sweep every 6-bit index through every position, hitting all five
// `encode()` branches ('A'-'Z', 'a'-'z', '0'-'9', '+', '/').
// ---------------------------------------------------------------------------

/// Pack four 6-bit values into the three bytes that produce them.
fn pack(v: [u8; 4]) -> [u8; 3] {
    [
        (v[0] << 2) | (v[1] >> 4),
        ((v[1] & 0x0F) << 4) | (v[2] >> 2),
        ((v[2] & 0x03) << 6) | v[3],
    ]
}

#[test]
fn cfg_row8_all_encode_branches_every_position() {
    let mut seen = [false; 64];
    for v in 0u8..64 {
        for pos in 0..4usize {
            let mut idx = [0u8; 4];
            idx[pos] = v;
            let bytes = pack(idx);
            check_explicit(&bytes, "row8 6-bit sweep");
        }
        // Same value in all four slots.
        let bytes = pack([v, v, v, v]);
        check_explicit(&bytes, "row8 6-bit uniform");

        if let Outcome::Str(s) = pair().c.call(3, Some(&bytes)) {
            assert_eq!(s.len(), 4);
            seen[v as usize] = true;
            // Cross-check against the reference transliteration of the C.
            assert_eq!(s, reference(3, &bytes), "reference mismatch for v={v}");
        } else {
            panic!("unexpected NULL for v={v}");
        }
    }
    assert!(seen.iter().all(|&x| x), "not every 6-bit index was exercised");
    // Confirm the alphabet's four distinct branches really were produced.
    let alphabet: Vec<u8> = (0u8..64)
        .map(|v| match pair().c.call(3, Some(&pack([v, 0, 0, 0]))) {
            Outcome::Str(s) => s[0],
            Outcome::Null => panic!("NULL"),
        })
        .collect();
    assert!(alphabet.contains(&b'A') && alphabet.contains(&b'Z'), "A-Z branch");
    assert!(alphabet.contains(&b'a') && alphabet.contains(&b'z'), "a-z branch");
    assert!(alphabet.contains(&b'0') && alphabet.contains(&b'9'), "0-9 branch");
    assert!(alphabet.contains(&b'+'), "'+' branch (u == 62)");
    assert!(alphabet.contains(&b'/'), "'/' branch (u == 63)");
}

// ---------------------------------------------------------------------------
// Rows 9-11: embedded NULs, and the min/max uniform buffers.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row9_embedded_nul_bytes_explicit_length() {
    let mut rng = Rng::new(0x9999_9999);
    for len in 1..=64usize {
        for _ in 0..20 {
            let mut bytes = rng.bytes(len);
            // Sprinkle NULs, guaranteeing at least one.
            bytes[rng.below(len)] = 0;
            for b in bytes.iter_mut() {
                if rng.byte() < 64 {
                    *b = 0;
                }
            }
            check_explicit(&bytes, "row9 embedded NULs");
        }
    }
    // A NUL in the very first position must still encode `len` bytes.
    let bytes = [0u8, b'A', b'B', b'C'];
    check_explicit(&bytes, "row9 leading NUL");
    match pair().c.call(4, Some(&bytes)) {
        Outcome::Str(s) => assert_eq!(s.len(), 8, "explicit length must ignore the NUL"),
        Outcome::Null => panic!("unexpected NULL"),
    }
}

#[test]
fn cfg_row10_all_zero_buffer() {
    for len in 1..=128usize {
        let bytes = vec![0u8; len];
        check_explicit(&bytes, "row10 all 0x00");
    }
    match pair().c.call(3, Some(&[0, 0, 0])) {
        Outcome::Str(s) => assert_eq!(&s, b"AAAA", "all-zero input encodes to 'AAAA'"),
        Outcome::Null => panic!("unexpected NULL"),
    }
}

#[test]
fn cfg_row11_all_ff_buffer() {
    for len in 1..=128usize {
        let bytes = vec![0xFFu8; len];
        check_explicit(&bytes, "row11 all 0xFF");
    }
    match pair().c.call(3, Some(&[0xFF, 0xFF, 0xFF])) {
        Outcome::Str(s) => assert_eq!(&s, b"////", "all-0xFF input encodes to '////'"),
        Outcome::Null => panic!("unexpected NULL"),
    }
}

// ---------------------------------------------------------------------------
// Row 12: size < real buffer length (the C never validates).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row12_size_smaller_than_buffer() {
    let mut rng = Rng::new(0xC0DE_C0DE);
    for _ in 0..3000 {
        let len = 1 + rng.below(96);
        let bytes = rng.bytes(len);
        let size = 1 + rng.below(len);
        pair().assert_same(size as i32, Some(&bytes), "row12 truncated size");
    }
}

// ---------------------------------------------------------------------------
// Rows 13-16: implicit `strlen` mode (size == 0).
// ---------------------------------------------------------------------------

#[test]
fn cfg_row13_strlen_mode_ascii() {
    let mut rng = Rng::new(0x1313_1313);
    for len in 1..=128usize {
        for _ in 0..12 {
            let mut bytes = rng.ascii(len);
            bytes.push(0); // NUL terminator
            pair().assert_same(0, Some(&bytes), "row13 strlen ascii");
        }
    }
}

#[test]
fn cfg_row14_strlen_mode_full_nonzero_byte_range() {
    let mut rng = Rng::new(0x1414_1414);
    for len in 1..=128usize {
        for _ in 0..12 {
            let mut bytes = rng.nonzero_bytes(len);
            bytes.push(0);
            pair().assert_same(0, Some(&bytes), "row14 strlen high-bit");
        }
        // All high-bit, at least once per length.
        let mut hi = rng.nonzero_bytes(len);
        for b in hi.iter_mut() {
            *b |= 0x80;
        }
        hi.push(0);
        pair().assert_same(0, Some(&hi), "row14 strlen all high-bit");
    }
}

#[test]
fn cfg_row15_strlen_mode_empty_string() {
    let empty = [0u8];
    pair().assert_same(0, Some(&empty), "row15 empty string");
    let c = pair().c.call(0, Some(&empty));
    let r = pair().rs.call(0, Some(&empty));
    assert_eq!(c, Outcome::Str(Vec::new()), "empty input => empty output");
    assert_eq!(r, Outcome::Str(Vec::new()), "empty input => empty output");
}

#[test]
fn cfg_row16_strlen_mode_ignores_bytes_after_nul() {
    let mut rng = Rng::new(0x1616_1616);
    for len in 1..=64usize {
        for _ in 0..12 {
            let mut bytes = rng.nonzero_bytes(len);
            bytes.push(0);
            let garbage_len = 1 + rng.below(16);
            bytes.extend(rng.bytes(garbage_len)); // garbage past the NUL
            bytes.push(0);
            pair().assert_same(0, Some(&bytes), "row16 trailing garbage");
        }
    }
    // Explicitly: "AB\0CD" in strlen mode must encode only "AB".
    let buf = b"AB\0CD\0";
    pair().assert_same(0, Some(buf), "row16 AB\\0CD");
    let only_ab = pair().c.call(2, Some(b"AB"));
    assert_eq!(pair().c.call(0, Some(buf)), only_ab, "strlen mode stops at NUL");
}

// ---------------------------------------------------------------------------
// Row 17: long inputs (multi-KiB), capacity edges at scale.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row17_long_inputs() {
    let mut rng = Rng::new(0x1717_1717);
    for _ in 0..60 {
        let len = 1024 + rng.below(7 * 1024);
        let bytes = rng.bytes(len);
        check_explicit(&bytes, "row17 long explicit");
    }
    for _ in 0..30 {
        let len = 1024 + rng.below(7 * 1024);
        let mut bytes = rng.nonzero_bytes(len);
        bytes.push(0);
        pair().assert_same(0, Some(&bytes), "row17 long strlen");
    }
    // Exact multi-KiB boundaries in every residue class.
    for len in [1023usize, 1024, 1025, 4095, 4096, 4097, 8191, 8192, 8193] {
        let bytes = rng.bytes(len);
        check_explicit(&bytes, "row17 boundary length");
    }
}

// ---------------------------------------------------------------------------
// Row 18: exhaustive length sweep 1..=200.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row18_exhaustive_length_sweep() {
    let mut rng = Rng::new(0x1818_1818);
    for len in 1..=200usize {
        for _ in 0..10 {
            let bytes = rng.bytes(len);
            check_explicit(&bytes, "row18 length sweep");
        }
        // strlen mode at the same length.
        let mut nz = rng.nonzero_bytes(len);
        nz.push(0);
        pair().assert_same(0, Some(&nz), "row18 length sweep strlen");
    }
}

// ---------------------------------------------------------------------------
// Rows 19-20 live in the error-path suite (they are `ERRORS.md` rows 4-6 and
// 10-11); they are re-asserted here so Phase B's table is self-contained.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row19_negative_size_valid_src() {
    let src = b"hello world\0";
    for size in [-1i32, -2, -3, -4, -100, -1_000_000, i32::MIN, i32::MIN + 1] {
        pair().assert_same(size, Some(src), "row19 negative size");
    }
}

#[test]
fn cfg_row20_overflow_threshold_allocation_parity() {
    let src = b"hello world\0";
    // Only the band [0x2000_0000, 0x3FFF_FFFC] is safely observable: there
    // `size*4` wraps to a value <= -13, so `cap` is negative, the huge
    // `calloc` fails, and the read loop never runs. Outside that band (e.g.
    // 0x3FFF_FFFF, 0x4000_0000, INT_MAX) `cap` wraps to a small POSITIVE
    // number, `calloc` succeeds, and the C loop writes gigabytes past a
    // 3-byte buffer -- ERRORS.md row 7, C-side UB with no ground truth.
    for size in [
        0x2000_0000i32,
        0x2000_0001,
        0x2AAA_AAAA,
        0x3000_0000,
        0x3FFF_FFF0,
        0x3FFF_FFFC,
    ] {
        pair().assert_same_nullness(size, Some(src), "row20 calloc-failure band");
    }

    // Just BELOW the threshold `cap` is a large positive number and the loop
    // really runs, so a genuinely `size`-byte source buffer is required.
    // 0x1FFF_FFFF bytes in, ~683 MiB out.
    let big = vec![0u8; 0x1FFF_FFFF];
    pair().assert_same_nullness(0x1FFF_FFFF, Some(&big), "row20 one below threshold");
    drop(big);
}

/// A large but fully comparable input, bridging row 17 (KiB) and row 20 (the
/// 512 MiB threshold): 64 MiB of random bytes, output compared byte-for-byte.
#[test]
fn cfg_row20b_large_input_full_byte_comparison() {
    let mut rng = Rng::new(0x2020_2020);
    let len = 64 * 1024 * 1024 + 1; // len % 3 == 2 -> single-'=' padding
    let bytes = rng.bytes(len);
    pair().assert_same(len as i32, Some(&bytes), "row20b 64 MiB explicit");
}

// ---------------------------------------------------------------------------
// Sanity: the differential harness really is exercising the codec, and both
// `.so` files agree with an independent transliteration of the C.
// ---------------------------------------------------------------------------

#[test]
fn harness_agrees_with_reference_encoder() {
    let mut rng = Rng::new(0xFEED_FACE);
    for len in 1..=90usize {
        for _ in 0..20 {
            let bytes = rng.bytes(len);
            let expect = Outcome::Str(reference(len as i32, &bytes));
            assert_eq!(
                pair().c.call(len as i32, Some(&bytes)),
                expect,
                "C vs reference, len={len}"
            );
            assert_eq!(
                pair().rs.call(len as i32, Some(&bytes)),
                expect,
                "Rust vs reference, len={len}"
            );
        }
    }
    // Known-answer vectors (RFC 4648 test vectors, with this library's
    // non-standard trailing-'=' behaviour for the partial groups).
    for (input, expect) in [
        (&b"f"[..], &b"Zg=="[..]),
        (&b"fo"[..], &b"Zm8="[..]),
        (&b"foo"[..], &b"Zm9v"[..]),
        (&b"foob"[..], &b"Zm9vYg=="[..]),
        (&b"fooba"[..], &b"Zm9vYmE="[..]),
        (&b"foobar"[..], &b"Zm9vYmFy"[..]),
    ] {
        assert_eq!(
            pair().c.call(input.len() as i32, Some(input)),
            Outcome::Str(expect.to_vec()),
            "C known-answer for {input:?}"
        );
        assert_eq!(
            pair().rs.call(input.len() as i32, Some(input)),
            Outcome::Str(expect.to_vec()),
            "Rust known-answer for {input:?}"
        );
    }
}
