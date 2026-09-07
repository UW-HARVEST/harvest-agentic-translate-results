//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every case calls the `searchAndReplace` export of the C `.so` and of the
//! Rust `.so` through `libloading` and compares the returned buffers
//! byte-for-byte. Inputs are randomized with a fixed seed per row.

mod common;

use common::*;

/// Filler alphabet, disjoint from the needle alphabet, so the *only* matches in
/// a constructed haystack are the ones we placed deliberately.
const FILLER: &[u8] = b"abcdefg";
const NEEDLE_ALPHA: &[u8] = b"XYZ";
/// Value alphabet, disjoint from the needle alphabet, so a replacement can
/// never introduce a new match (the C never rescans its output anyway).
const VALUE_ALPHA: &[u8] = b"12345";

fn needle(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let n = rng.range(lo, hi);
    rng.bytes(n, NEEDLE_ALPHA)
}

fn filler(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let n = rng.range(lo, hi);
    if n == 0 {
        Vec::new()
    } else {
        rng.bytes(n, FILLER)
    }
}

fn value(rng: &mut Rng, lo: usize, hi: usize) -> Vec<u8> {
    let n = rng.range(lo, hi);
    if n == 0 {
        Vec::new()
    } else {
        rng.bytes(n, VALUE_ALPHA)
    }
}

// ---------------------------------------------------------------- C1 ---------
#[test]
fn c1_no_match_empty_orig() {
    let mut rng = Rng::new(0xC001);
    for _ in 0..2000 {
        let s = needle(&mut rng, 1, 8);
        let v = value(&mut rng, 0, 8);
        let r = diff("C1", b"", &s, &v);
        assert_eq!(r, Ret::Str(Vec::new()), "C1 expected strdup(\"\")");
    }
}

// ---------------------------------------------------------------- C2 ---------
#[test]
fn c2_no_match_nonempty_orig() {
    let mut rng = Rng::new(0xC002);
    for _ in 0..20_000 {
        let o = filler(&mut rng, 1, 64);
        let s = needle(&mut rng, 1, 4);
        let v = value(&mut rng, 0, 8);
        let r = diff("C2", &o, &s, &v);
        assert_eq!(r, Ret::Str(o.clone()), "C2 expected strdup(orig)");
    }
}

// ---------------------------------------------------------------- C3 ---------
#[test]
fn c3_needle_longer_than_haystack() {
    let mut rng = Rng::new(0xC003);
    for _ in 0..20_000 {
        let olen = rng.range(0, 6);
        let o = if olen == 0 {
            Vec::new()
        } else {
            rng.bytes(olen, ALPHA_SMALL)
        };
        let slen = olen + rng.range(1, 5);
        let s = rng.bytes(slen, ALPHA_SMALL);
        let v = value(&mut rng, 0, 6);
        assert!(s.len() > o.len());
        diff("C3", &o, &s, &v);
    }
}

// ---------------------------------------------------------------- C4 ---------
#[test]
fn c4_match_at_zero_single_no_tail() {
    let mut rng = Rng::new(0xC004);
    for _ in 0..20_000 {
        let s = needle(&mut rng, 1, 8);
        let v = value(&mut rng, 0, 10);
        // orig == search: inx_start == 0 (no leading malloc), one match, no tail.
        diff("C4", &s, &s, &v);
    }
}

// ---------------------------------------------------------------- C5 ---------
#[test]
fn c5_match_at_zero_single_with_tail() {
    let mut rng = Rng::new(0xC005);
    for _ in 0..20_000 {
        let s = needle(&mut rng, 1, 6);
        let tail = filler(&mut rng, 1, 20);
        let v = value(&mut rng, 0, 10);
        let mut o = s.clone();
        o.extend_from_slice(&tail);
        diff("C5", &o, &s, &v);
    }
}

// ---------------------------------------------------------------- C6 ---------
#[test]
fn c6_match_after_lead_no_tail() {
    let mut rng = Rng::new(0xC006);
    for _ in 0..20_000 {
        let lead = filler(&mut rng, 1, 20);
        let s = needle(&mut rng, 1, 6);
        let v = value(&mut rng, 0, 10);
        let mut o = lead.clone();
        o.extend_from_slice(&s);
        diff("C6", &o, &s, &v);
    }
}

// ---------------------------------------------------------------- C7 ---------
#[test]
fn c7_match_after_lead_with_tail() {
    let mut rng = Rng::new(0xC007);
    for _ in 0..20_000 {
        let lead = filler(&mut rng, 1, 20);
        let s = needle(&mut rng, 1, 6);
        let tail = filler(&mut rng, 1, 20);
        let v = value(&mut rng, 0, 10);
        let mut o = lead.clone();
        o.extend_from_slice(&s);
        o.extend_from_slice(&tail);
        diff("C7", &o, &s, &v);
    }
}

// ---------------------------------------------------------------- C8 ---------
#[test]
fn c8_two_matches_with_gap() {
    let mut rng = Rng::new(0xC008);
    for _ in 0..20_000 {
        let lead = filler(&mut rng, 0, 12);
        let s = needle(&mut rng, 1, 5);
        let gap = filler(&mut rng, 1, 12);
        let tail = filler(&mut rng, 0, 12);
        let v = value(&mut rng, 0, 10);
        let mut o = lead.clone();
        o.extend_from_slice(&s);
        o.extend_from_slice(&gap);
        o.extend_from_slice(&s);
        o.extend_from_slice(&tail);
        diff("C8", &o, &s, &v);
    }
}

// ---------------------------------------------------------------- C9 ---------
#[test]
fn c9_two_adjacent_matches() {
    let mut rng = Rng::new(0xC009);
    for _ in 0..20_000 {
        let lead = filler(&mut rng, 0, 12);
        let s = needle(&mut rng, 1, 5);
        let tail = filler(&mut rng, 0, 12);
        let v = value(&mut rng, 0, 10);
        let mut o = lead.clone();
        o.extend_from_slice(&s);
        o.extend_from_slice(&s);
        o.extend_from_slice(&tail);
        diff("C9", &o, &s, &v);
    }
}

// --------------------------------------------------------------- C10 ---------
#[test]
fn c10_many_matches_mixed_gaps() {
    let mut rng = Rng::new(0xC010);
    for _ in 0..20_000 {
        let s = needle(&mut rng, 1, 5);
        let v = value(&mut rng, 0, 10);
        let k = rng.range(3, 12);
        let mut o = filler(&mut rng, 0, 10);
        for i in 0..k {
            o.extend_from_slice(&s);
            if i + 1 < k {
                // 50 % adjacent (gap == 0), 50 % separated (gap > 0)
                let g = if rng.below(2) == 0 { 0 } else { rng.range(1, 8) };
                if g > 0 {
                    o.extend_from_slice(&rng.bytes(g, FILLER));
                }
            }
        }
        o.extend_from_slice(&filler(&mut rng, 0, 10));
        diff("C10", &o, &s, &v);
    }
}

// --------------------------------------------------------------- C11 ---------
#[test]
fn c11_empty_value_deletes() {
    let mut rng = Rng::new(0xC011);
    for _ in 0..20_000 {
        let s = needle(&mut rng, 1, 5);
        let k = rng.range(2, 8);
        let mut o = filler(&mut rng, 1, 10);
        for i in 0..k {
            o.extend_from_slice(&s);
            if i + 1 < k {
                o.extend_from_slice(&filler(&mut rng, 0, 6));
            }
        }
        o.extend_from_slice(&filler(&mut rng, 1, 10));
        diff("C11", &o, &s, b"");
    }
}

// ------------------------------------------------------- C12 / C13 / C14 -----
fn value_len_relation_row(row: &str, seed: u64, rel: std::cmp::Ordering) {
    let mut rng = Rng::new(seed);
    for _ in 0..20_000 {
        let slen = rng.range(2, 6);
        let s = rng.bytes(slen, NEEDLE_ALPHA);
        let vlen = match rel {
            std::cmp::Ordering::Less => rng.range(0, slen - 1),
            std::cmp::Ordering::Equal => slen,
            std::cmp::Ordering::Greater => rng.range(slen + 1, slen + 12),
        };
        let v = if vlen == 0 {
            Vec::new()
        } else {
            rng.bytes(vlen, VALUE_ALPHA)
        };
        let k = rng.range(1, 8);
        let mut o = filler(&mut rng, 0, 8);
        for i in 0..k {
            o.extend_from_slice(&s);
            if i + 1 < k {
                o.extend_from_slice(&filler(&mut rng, 0, 6));
            }
        }
        o.extend_from_slice(&filler(&mut rng, 0, 8));
        diff(row, &o, &s, &v);
    }
}

#[test]
fn c12_value_shorter_than_search() {
    value_len_relation_row("C12", 0xC012, std::cmp::Ordering::Less);
}

#[test]
fn c13_value_same_len_as_search() {
    value_len_relation_row("C13", 0xC013, std::cmp::Ordering::Equal);
}

#[test]
fn c14_value_longer_than_search() {
    value_len_relation_row("C14", 0xC014, std::cmp::Ordering::Greater);
}

// --------------------------------------------------------------- C15 ---------
#[test]
fn c15_orig_is_only_matches() {
    let mut rng = Rng::new(0xC015);
    for _ in 0..20_000 {
        let s = vec![NEEDLE_ALPHA[rng.below(NEEDLE_ALPHA.len())]];
        let k = rng.range(1, 24);
        let o: Vec<u8> = s.iter().cycle().take(k).copied().collect();
        let v = value(&mut rng, 0, 6);
        diff("C15", &o, &s, &v);
    }
}

// --------------------------------------------------------------- C16 ---------
#[test]
fn c16_self_overlapping_needle() {
    // Hand-picked structural cases first.
    for n in 0..24usize {
        let o = vec![b'a'; n];
        diff("C16", &o, b"aa", b"Q");
        diff("C16", &o, b"aa", b"");
        diff("C16", &o, b"aaa", b"QQQQ");
        diff("C16", &o, b"a", b"aa");
    }
    // Then randomized: needles that are runs of one byte, haystacks that are
    // runs of the same byte with occasional foreign bytes.
    let mut rng = Rng::new(0xC016);
    for _ in 0..20_000 {
        let nlen = rng.range(1, 5);
        let s = vec![b'a'; nlen];
        let olen = rng.range(0, 40);
        let mut o: Vec<u8> = (0..olen)
            .map(|_| if rng.below(8) == 0 { b'b' } else { b'a' })
            .collect();
        if o.is_empty() {
            o.push(b'a');
        }
        let v = value(&mut rng, 0, 6);
        diff("C16", &o, &s, &v);
    }
}

// --------------------------------------------------------------- C17 ---------
#[test]
fn c17_partial_prefix_decoys() {
    for (o, s) in [
        (&b"abaabab"[..], &b"abab"[..]),
        (&b"aabaabaaabaab"[..], &b"aabaab"[..]),
        (&b"ababab"[..], &b"abab"[..]),
        (&b"abababab"[..], &b"abab"[..]),
        (&b"xabxabxaby"[..], &b"abxaby"[..]),
        (&b"aaXaaXXaaX"[..], &b"aaXX"[..]),
    ] {
        for v in [&b""[..], &b"Q"[..], &b"QQQQQQ"[..], &b"ab"[..]] {
            diff("C17", o, s, v);
        }
    }
    let mut rng = Rng::new(0xC017);
    for _ in 0..50_000 {
        // Dense small alphabet => many partial-prefix decoys.
        let o = rng.bytes_range(1, 24, ALPHA_TINY);
        let s = rng.bytes_range(2, 5, ALPHA_TINY);
        let v = value(&mut rng, 0, 5);
        diff("C17", &o, &s, &v);
    }
}

// --------------------------------------------------------------- C18 ---------
#[test]
fn c18_value_contains_search() {
    let mut rng = Rng::new(0xC018);
    for _ in 0..20_000 {
        let s = needle(&mut rng, 1, 4);
        // value = pre + search + post, so a naive re-scanning implementation
        // would loop or double-replace.
        let mut v = filler(&mut rng, 0, 4);
        v.extend_from_slice(&s);
        v.extend_from_slice(&filler(&mut rng, 0, 4));
        let k = rng.range(1, 6);
        let mut o = filler(&mut rng, 0, 6);
        for i in 0..k {
            o.extend_from_slice(&s);
            if i + 1 < k {
                o.extend_from_slice(&filler(&mut rng, 0, 4));
            }
        }
        o.extend_from_slice(&filler(&mut rng, 0, 6));
        diff("C18", &o, &s, &v);
    }
}

// --------------------------------------------------------------- C19 ---------
#[test]
fn c19_single_byte_orig() {
    // Exhaustive over a small byte set, both matching and non-matching.
    for ob in [b'a', b'X', 0x00u8 + 1, 0x7f, 0x80, 0xff] {
        for sb in [b'a', b'X', 1u8, 0x7f, 0x80, 0xff] {
            for v in [&b""[..], &b"Q"[..], &b"QQQ"[..]] {
                diff("C19", &[ob], &[sb], v);
            }
        }
    }
}

// --------------------------------------------------------------- C20 ---------
#[test]
fn c20_large_inputs_many_reallocs() {
    let mut rng = Rng::new(0xC020);
    for _ in 0..60 {
        let s = needle(&mut rng, 1, 4);
        let v = value(&mut rng, 0, 8);
        let target = rng.range(4 * 1024, 64 * 1024);
        let mut o: Vec<u8> = Vec::with_capacity(target + 64);
        while o.len() < target {
            if rng.below(4) == 0 {
                o.extend_from_slice(&s);
            } else {
                o.extend_from_slice(&rng.bytes_range(1, 12, FILLER));
            }
        }
        diff("C20", &o, &s, &v);
    }
}

// --------------------------------------------------------------- C21 ---------
#[test]
fn c21_high_bytes() {
    let alpha = alpha_high();
    let mut rng = Rng::new(0xC021);
    for _ in 0..50_000 {
        let o = rng.bytes_range(1, 40, &alpha);
        let s = rng.bytes_range(1, 4, &alpha);
        let vlen = rng.range(0, 6);
        let v = if vlen == 0 {
            Vec::new()
        } else {
            rng.bytes(vlen, &alpha)
        };
        diff("C21", &o, &s, &v);
    }
    // Guarantee some matches with multi-byte high needles.
    for _ in 0..20_000 {
        let s = rng.bytes_range(1, 3, &alpha);
        let k = rng.range(1, 5);
        let mut o = rng.bytes_range(1, 6, &alpha);
        for _ in 0..k {
            o.extend_from_slice(&s);
            o.extend_from_slice(&rng.bytes_range(1, 5, &alpha));
        }
        let v = rng.bytes_range(1, 5, &alpha);
        diff("C21", &o, &s, &v);
    }
}

// --------------------------------------------------------------- C22 ---------
#[test]
fn c22_property_fuzz_dense_alphabet() {
    let mut rng = Rng::new(0xC022);
    // Tiny alphabet => matches are dense and appear at unpredictable offsets,
    // hitting every combination of the leading/gap/tail branches by accident.
    for _ in 0..200_000 {
        let o = rng.bytes_range(0, 20, ALPHA_TINY);
        let s = rng.bytes_range(1, 4, ALPHA_TINY);
        let vlen = rng.range(0, 4);
        let v = if vlen == 0 {
            Vec::new()
        } else {
            rng.bytes(vlen, ALPHA_TINY)
        };
        diff("C22", &o, &s, &v);
    }
    // Same again over the full non-NUL byte range with a 3-symbol alphabet.
    for _ in 0..100_000 {
        let alpha: Vec<u8> = (0..3)
            .map(|_| (rng.range(1, 255)) as u8)
            .collect::<Vec<u8>>();
        let o = rng.bytes_range(0, 16, &alpha);
        let s = rng.bytes_range(1, 3, &alpha);
        let vlen = rng.range(0, 3);
        let v = if vlen == 0 {
            Vec::new()
        } else {
            rng.bytes(vlen, &alpha)
        };
        diff("C22", &o, &s, &v);
    }
}

// --------------------------------------------------------------- C23 ---------
#[test]
fn c23_last_match_ends_at_end_of_orig() {
    let mut rng = Rng::new(0xC023);
    for _ in 0..20_000 {
        let lead = filler(&mut rng, 1, 12);
        let s = needle(&mut rng, 1, 5);
        let gap = filler(&mut rng, 0, 10);
        let v = value(&mut rng, 0, 8);
        let mut o = lead.clone();
        o.extend_from_slice(&s);
        o.extend_from_slice(&gap);
        o.extend_from_slice(&s); // ends exactly at orig_len => tail branch skipped
        diff("C23", &o, &s, &v);
    }
}

// --------------------------------------------------------------- C24 ---------
#[test]
fn c24_identity_and_erase_whole_string() {
    let mut rng = Rng::new(0xC024);
    for _ in 0..20_000 {
        let s = rng.bytes_range(1, 12, ALPHA_ASCII);
        diff("C24", &s, &s, &s); // identity rewrite
        diff("C24", &s, &s, b""); // erase everything
    }
}
