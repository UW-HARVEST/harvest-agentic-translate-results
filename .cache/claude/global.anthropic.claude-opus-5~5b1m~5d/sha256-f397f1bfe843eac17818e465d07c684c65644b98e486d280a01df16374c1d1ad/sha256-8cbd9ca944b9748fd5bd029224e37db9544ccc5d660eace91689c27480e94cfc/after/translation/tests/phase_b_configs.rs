//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Every test calls BOTH the C `.so` and the Rust `.so` through their exported
//! `decode_base64` symbol and compares the full heap allocation byte-for-byte.
//! All randomized rows use a fixed seed (reproducible).

mod common;

use common::*;

const ITERS: usize = 400;

/// Row 1 — l % 4 == 0, no '=', lengths 4..64.
#[test]
fn row01_len_multiple_of_4() {
    let mut rng = Rng::new(0x0001);
    for n in (4..=64).step_by(4) {
        for _ in 0..ITERS {
            let s = rand_from(&mut rng, B64_ALPHABET, n);
            assert_same("row01", &s);
        }
    }
}

/// Row 2 — l % 4 == 1 (c2 = c3 = c4 default to 'A').
#[test]
fn row02_len_mod4_is_1() {
    let mut rng = Rng::new(0x0002);
    for n in [1, 5, 9, 13, 17, 33, 65, 101] {
        for _ in 0..ITERS {
            let s = rand_from(&mut rng, B64_ALPHABET, n);
            assert_same("row02", &s);
        }
    }
}

/// Row 3 — l % 4 == 2 (c3 = c4 default to 'A').
#[test]
fn row03_len_mod4_is_2() {
    let mut rng = Rng::new(0x0003);
    for n in [2, 6, 10, 14, 18, 34, 66, 102] {
        for _ in 0..ITERS {
            let s = rand_from(&mut rng, B64_ALPHABET, n);
            assert_same("row03", &s);
        }
    }
}

/// Row 4 — l % 4 == 3 (c4 defaults to 'A').
#[test]
fn row04_len_mod4_is_3() {
    let mut rng = Rng::new(0x0004);
    for n in [3, 7, 11, 15, 19, 35, 67, 103] {
        for _ in 0..ITERS {
            let s = rand_from(&mut rng, B64_ALPHABET, n);
            assert_same("row04", &s);
        }
    }
}

/// Row 5 — exhaustive single base64 character, plus '='.
#[test]
fn row05_single_char_exhaustive() {
    for &c in B64_ALPHABET {
        assert_same("row05", &[c]);
    }
    assert_same("row05:eq", b"=");
}

/// Row 6 — exhaustive 2-char and randomized 3-char inputs.
#[test]
fn row06_two_and_three_chars() {
    let set: Vec<u8> = B64_ALPHABET.iter().copied().chain([b'=']).collect();
    for &a in &set {
        for &b in &set {
            assert_same("row06:pair", &[a, b]);
        }
    }
    let mut rng = Rng::new(0x0006);
    for _ in 0..4000 {
        let s = rand_from(&mut rng, &set, 3);
        assert_same("row06:triple", &s);
    }
}

/// Row 7 — canonical single padding `xxx=`.
#[test]
fn row07_single_padding() {
    let mut rng = Rng::new(0x0007);
    for groups in 1..=6 {
        for _ in 0..ITERS {
            let mut s = rand_from(&mut rng, B64_ALPHABET, groups * 4);
            *s.last_mut().unwrap() = b'=';
            assert_same("row07", &s);
        }
    }
}

/// Row 8 — canonical double padding `xx==`.
#[test]
fn row08_double_padding() {
    let mut rng = Rng::new(0x0008);
    for groups in 1..=6 {
        for _ in 0..ITERS {
            let mut s = rand_from(&mut rng, B64_ALPHABET, groups * 4);
            let n = s.len();
            s[n - 1] = b'=';
            s[n - 2] = b'=';
            assert_same("row08", &s);
        }
    }
}

/// Row 9 — '=' in the c1 slot of a group (interior padding, decodes to 63).
#[test]
fn row09_eq_in_c1_slot() {
    let mut rng = Rng::new(0x0009);
    for groups in 1..=6 {
        for g in 0..groups {
            for _ in 0..60 {
                let mut s = rand_from(&mut rng, B64_ALPHABET, groups * 4);
                s[g * 4] = b'=';
                assert_same("row09", &s);
            }
        }
    }
}

/// Row 10 — '=' in the c2 slot of a group.
#[test]
fn row10_eq_in_c2_slot() {
    let mut rng = Rng::new(0x000a);
    for groups in 1..=6 {
        for g in 0..groups {
            for _ in 0..60 {
                let mut s = rand_from(&mut rng, B64_ALPHABET, groups * 4);
                s[g * 4 + 1] = b'=';
                assert_same("row10", &s);
            }
        }
    }
}

/// Row 11 — '=' in the c3 slot of a NON-FINAL group: suppresses byte 2 of that
/// group while decoding continues into later groups.
#[test]
fn row11_eq_in_c3_slot_nonfinal() {
    let mut rng = Rng::new(0x000b);
    for groups in 2..=6 {
        for g in 0..groups - 1 {
            for _ in 0..60 {
                let mut s = rand_from(&mut rng, B64_ALPHABET, groups * 4);
                s[g * 4 + 2] = b'=';
                assert_same("row11:c3", &s);
                let mut t = rand_from(&mut rng, B64_ALPHABET, groups * 4);
                t[g * 4 + 3] = b'=';
                assert_same("row11:c4", &t);
                let mut u = rand_from(&mut rng, B64_ALPHABET, groups * 4);
                u[g * 4 + 2] = b'=';
                u[g * 4 + 3] = b'=';
                assert_same("row11:c3c4", &u);
            }
        }
    }
}

/// Row 12 — multiple '=' at random positions and random counts.
#[test]
fn row12_scattered_padding() {
    let mut rng = Rng::new(0x000c);
    for _ in 0..6000 {
        let n = rng.range(1, 40);
        let mut s = rand_from(&mut rng, B64_ALPHABET, n);
        let eqs = rng.range(1, n);
        for _ in 0..eqs {
            let i = rng.below(n);
            s[i] = b'=';
        }
        assert_same("row12", &s);
    }
}

/// Row 13 — alphabet restricted to A-Z (decode class 0..25).
#[test]
fn row13_upper_only() {
    let mut rng = Rng::new(0x000d);
    for n in 1..=40 {
        for _ in 0..60 {
            assert_same("row13", &rand_from(&mut rng, UPPER, n));
        }
    }
}

/// Row 14 — alphabet restricted to a-z (decode class 26..51).
#[test]
fn row14_lower_only() {
    let mut rng = Rng::new(0x000e);
    for n in 1..=40 {
        for _ in 0..60 {
            assert_same("row14", &rand_from(&mut rng, LOWER, n));
        }
    }
}

/// Row 15 — alphabet restricted to 0-9 (decode class 52..61).
#[test]
fn row15_digits_only() {
    let mut rng = Rng::new(0x000f);
    for n in 1..=40 {
        for _ in 0..60 {
            assert_same("row15", &rand_from(&mut rng, DIGITS, n));
        }
    }
}

/// Row 16 — alphabet restricted to '+' and '/' (decode classes 62 and 63).
#[test]
fn row16_specials_only() {
    let mut rng = Rng::new(0x0010);
    for n in 1..=40 {
        for _ in 0..40 {
            assert_same("row16", &rand_from(&mut rng, SPECIALS, n));
        }
    }
    for n in 1..=12 {
        assert_same("row16:plus", &vec![b'+'; n]);
        assert_same("row16:slash", &vec![b'/'; n]);
    }
}

/// Row 17 — every decode-class boundary char at every slot of a 4-group.
#[test]
fn row17_boundary_chars_every_slot() {
    let boundaries = b"AZaz09+/=";
    for &c in boundaries {
        for slot in 0..4 {
            for filler in [b'A', b'Z', b'z', b'9', b'/', b'='] {
                let mut s = vec![filler; 4];
                s[slot] = c;
                assert_same("row17", &s);
                // and inside a two-group input
                let mut t = vec![filler; 8];
                t[slot] = c;
                assert_same("row17:g0", &t);
                let mut u = vec![filler; 8];
                u[4 + slot] = c;
                assert_same("row17:g1", &u);
            }
        }
    }
}

/// Row 18 — valid base64 with rejected ASCII bytes interleaved.
#[test]
fn row18_ascii_junk_interleaved() {
    let junk = ascii_junk();
    assert!(!junk.is_empty());
    let mut rng = Rng::new(0x0012);
    for _ in 0..6000 {
        let n = rng.range(1, 60);
        let mut s = Vec::with_capacity(n);
        for _ in 0..n {
            if rng.below(3) == 0 {
                s.push(rng.pick(&junk));
            } else {
                s.push(rng.pick(B64_ALPHABET));
            }
        }
        assert_same("row18", &s);
    }
}

/// Row 19 — high-bit bytes 0x80..0xFF interleaved (negative `char`).
#[test]
fn row19_high_bit_bytes_interleaved() {
    let high: Vec<u8> = (0x80u8..=0xff).collect();
    let mut rng = Rng::new(0x0013);
    for _ in 0..6000 {
        let n = rng.range(1, 60);
        let mut s = Vec::with_capacity(n);
        for _ in 0..n {
            if rng.below(3) == 0 {
                s.push(rng.pick(&high));
            } else {
                s.push(rng.pick(B64_ALPHABET));
            }
        }
        assert_same("row19", &s);
    }
    // every single high byte alone, and prefixed to a valid group
    for b in 0x80u8..=0xff {
        assert_same("row19:solo", &[b]);
        assert_same("row19:pre", &[b, b'A', b'B', b'C', b'D']);
        assert_same("row19:post", &[b'A', b'B', b'C', b'D', b]);
    }
}

/// Row 20 — leading-only / interior-only / trailing-only junk.
#[test]
fn row20_junk_placement() {
    let junk = ascii_junk();
    let mut rng = Rng::new(0x0014);
    for _ in 0..2000 {
        let bn = rng.range(1, 30);
        let body = rand_from(&mut rng, B64_ALPHABET, bn);
        let jn = rng.range(1, 8);
        let j = rand_from(&mut rng, &junk, jn);

        let mut lead = j.clone();
        lead.extend_from_slice(&body);
        assert_same("row20:lead", &lead);

        let mut trail = body.clone();
        trail.extend_from_slice(&j);
        assert_same("row20:trail", &trail);

        let mid = body.len() / 2;
        let mut inter = body[..mid].to_vec();
        inter.extend_from_slice(&j);
        inter.extend_from_slice(&body[mid..]);
        assert_same("row20:interior", &inter);
    }
}

/// Row 21 — non-empty input containing ZERO base64 chars: `l == 0`, the decode
/// loop never executes and an all-zero buffer is returned (non-NULL).
#[test]
fn row21_only_junk_nonempty() {
    let junk = ascii_junk();
    let high: Vec<u8> = (0x80u8..=0xff).collect();
    let all_junk: Vec<u8> = junk.iter().chain(high.iter()).copied().collect();

    // sanity: the C really does return non-NULL here
    match c_call(b"!!!") {
        Outcome::Buf(v) => assert!(v.iter().all(|&b| b == 0), "expected all-zero buffer"),
        Outcome::Null => panic!("C returned NULL for \"!!!\""),
    }

    for &b in &all_junk {
        assert_same("row21:solo", &[b]);
    }
    let mut rng = Rng::new(0x0015);
    for _ in 0..3000 {
        let n = rng.range(1, 80);
        assert_same("row21", &rand_from(&mut rng, &all_junk, n));
    }
}

/// Row 22 — unconstrained fuzz: random bytes 0x01..0xFF, length 1..300.
#[test]
fn row22_full_fuzz() {
    let mut rng = Rng::new(0x0016);
    for _ in 0..12000 {
        let n = rng.range(1, 300);
        let s: Vec<u8> = (0..n)
            .map(|_| {
                let mut b = rng.byte();
                if b == 0 {
                    b = 1;
                }
                b
            })
            .collect();
        assert_same("row22", &s);
    }
}

/// Row 23 — ~70 % base64 chars, length 1..1024.
#[test]
fn row23_biased_fuzz() {
    let junk: Vec<u8> = ascii_junk().into_iter().chain(0x80u8..=0xff).collect();
    let mut rng = Rng::new(0x0017);
    for _ in 0..2500 {
        let n = rng.range(1, 1024);
        let s: Vec<u8> = (0..n)
            .map(|_| {
                if rng.below(10) < 7 {
                    let mut set = B64_ALPHABET.to_vec();
                    set.push(b'=');
                    rng.pick(&set)
                } else {
                    rng.pick(&junk)
                }
            })
            .collect();
        assert_same("row23", &s);
    }
}

/// Row 24 — decoded payloads containing embedded NUL bytes.
#[test]
fn row24_embedded_nuls() {
    for lit in [
        &b"AAAA"[..],
        b"AA==",
        b"AAA=",
        b"AAAAAAAA",
        b"AAAAAA==",
        b"A",
        b"AA",
        b"AAA",
        b"AAAAA",
    ] {
        assert_same("row24:lit", lit);
    }
    let mut rng = Rng::new(0x0018);
    for _ in 0..3000 {
        let n = rng.range(1, 64);
        // zero-heavy payload -> lots of embedded NULs in the output
        let payload: Vec<u8> = (0..n)
            .map(|_| if rng.below(4) == 0 { rng.byte() } else { 0 })
            .collect();
        let enc = b64_encode(&payload);
        assert_same("row24:enc", &enc);
    }
}

/// Row 25 — payload spanning all 256 output byte values.
#[test]
fn row25_all_output_bytes() {
    let payload: Vec<u8> = (0..=255u8).collect();
    assert_same("row25:full", &b64_encode(&payload));
    for shift in 0..3 {
        let p: Vec<u8> = payload[shift..].to_vec();
        assert_same("row25:shift", &b64_encode(&p));
    }
    for b in 0..=255u8 {
        assert_same("row25:one", &b64_encode(&[b]));
        assert_same("row25:two", &b64_encode(&[b, b]));
        assert_same("row25:three", &b64_encode(&[b, b, b]));
    }
}

/// Row 26 — large inputs: 4 KiB, 16 KiB, 64 KiB.
#[test]
fn row26_large_inputs() {
    let mut rng = Rng::new(0x0019);
    for n in [4096usize, 16384, 65536] {
        assert_same("row26:pure", &rand_from(&mut rng, B64_ALPHABET, n));
        let mut set = B64_ALPHABET.to_vec();
        set.push(b'=');
        assert_same("row26:pad", &rand_from(&mut rng, &set, n));
        // with junk mixed in so the filter shrinks l substantially
        let junk: Vec<u8> = ascii_junk().into_iter().chain(0x80u8..=0xff).collect();
        let s: Vec<u8> = (0..n)
            .map(|_| {
                if rng.below(2) == 0 {
                    rng.pick(&set)
                } else {
                    rng.pick(&junk)
                }
            })
            .collect();
        assert_same("row26:mixed", &s);
        // odd lengths around the group boundary
        for extra in 1..=3 {
            assert_same("row26:odd", &rand_from(&mut rng, B64_ALPHABET, n + extra));
        }
    }
}

/// Row 27 — 64 KiB of '=' only.
#[test]
fn row27_all_equals_large() {
    for n in [1usize, 2, 3, 4, 5, 7, 8, 1024, 65536] {
        assert_same("row27", &vec![b'='; n]);
    }
}

/// Row 28 — every single-byte input 0x01..=0xFF through the pointer parameter.
#[test]
fn row28_single_byte_sweep() {
    for b in 1u8..=255 {
        assert_same("row28", &[b]);
    }
}

/// Row 29 — exhaustive 2-byte sweep over a representative alphabet ∪ junk.
#[test]
fn row29_exhaustive_pairs_mixed() {
    let set: Vec<u8> = b"AZaz09+/=!\x20\x7f\x80\xff\x01"
        .iter()
        .copied()
        .collect();
    for &a in &set {
        for &b in &set {
            assert_same("row29:2", &[a, b]);
            for &c in &set {
                assert_same("row29:3", &[a, b, c]);
                assert_same("row29:4", &[a, b, c, a]);
                assert_same("row29:5", &[a, b, c, a, b]);
            }
        }
    }
}

/// Row 30 — lengths at group boundaries ±1.
#[test]
fn row30_group_boundary_lengths() {
    let mut rng = Rng::new(0x001e);
    let mut set = B64_ALPHABET.to_vec();
    set.push(b'=');
    for n in [1usize, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15, 16, 17] {
        for _ in 0..800 {
            assert_same("row30", &rand_from(&mut rng, &set, n));
        }
    }
}
