//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row drives BOTH the C `.so` and the
//! Rust `.so` through their exported `tool_basename` symbol with MANY
//! randomized inputs (fixed seed, so failures reproduce) and asserts the
//! returned pointers are byte-identical.

mod common;

use common::*;

const N: usize = 400;

/// Independent, straight-from-the-C reference used as a cross-check that the
/// randomized generators really do land on the branch each row claims.
fn expected_offset(s: &[u8]) -> usize {
    let s1 = s.iter().rposition(|&b| b == b'/');
    let s2 = s.iter().rposition(|&b| b == b'\\');
    match (s1, s2) {
        (Some(i1), Some(i2)) => (if i1 > i2 { i1 } else { i2 }) + 1,
        (Some(i1), None) => i1 + 1,
        (None, Some(i2)) => i2 + 1,
        (None, None) => 0,
    }
}

fn check(bytes: &[u8], ctx: &str) {
    let got = compare(bytes, ctx);
    assert_eq!(
        got.offset as usize,
        expected_offset(bytes),
        "both libs agreed but disagree with the C semantics [{ctx}] input={:?}",
        Bytes(bytes)
    );
}

// ---------------------------------------------------------------- rows 1..3
// B5: no separator present at all -> `path` returned unchanged.

#[test]
fn cfg_01_b5_no_separator_random_ascii() {
    let mut rng = Rng::new(Rng::SEED);
    for _ in 0..N {
        let len = rng.range(1, 64);
        let s: Vec<u8> = (0..len).map(|_| rng.pick(b"abcXYZ.0-_ ")).collect();
        check(&s, "row1 b5 ascii");
    }
}

#[test]
fn cfg_02_b5_empty_string() {
    check(b"", "row2 empty");
    // and the empty string reached through many repeated calls
    for _ in 0..8 {
        check(b"", "row2 empty repeat");
    }
}

#[test]
fn cfg_03_b5_near_miss_alphabet() {
    let mut rng = Rng::new(Rng::SEED ^ 3);
    let alphabet: &[u8] = &[b'.', b'0', b'[', b']', b':', 0x2e, 0x30, 0x5b, 0x5d, 0xa6];
    for _ in 0..N {
        let len = rng.range(1, 40);
        let s: Vec<u8> = (0..len).map(|_| rng.pick(alphabet)).collect();
        check(&s, "row3 near-miss alphabet");
    }
    check(b"./0[]:", "row3 fixed");
}

// ---------------------------------------------------------------- rows 4..8
// B3: only '/' present.

#[test]
fn cfg_04_b3_single_slash_interior() {
    let mut rng = Rng::new(Rng::SEED ^ 4);
    for _ in 0..N {
        let len = rng.range(3, 48);
        let mut s = rand_non_sep(&mut rng, len);
        let pos = rng.range(1, len - 2); // interior
        s[pos] = b'/';
        check(&s, "row4 single '/' interior");
    }
}

#[test]
fn cfg_05_b3_slash_at_first_byte() {
    let mut rng = Rng::new(Rng::SEED ^ 5);
    for _ in 0..N {
        let len = rng.range(1, 40);
        let mut s = rand_non_sep(&mut rng, len);
        s[0] = b'/';
        check(&s, "row5 '/' first");
    }
    check(b"/x", "row5 fixed");
}

#[test]
fn cfg_06_b3_slash_at_last_byte() {
    let mut rng = Rng::new(Rng::SEED ^ 6);
    for _ in 0..N {
        let len = rng.range(1, 40);
        let mut s = rand_non_sep(&mut rng, len);
        *s.last_mut().unwrap() = b'/';
        let r = compare(&s, "row6 '/' last");
        assert!(r.tail.is_empty(), "trailing separator must yield empty tail");
        assert_eq!(r.offset as usize, s.len());
    }
    check(b"a/b/", "row6 fixed");
}

#[test]
fn cfg_07_b3_many_slashes() {
    let mut rng = Rng::new(Rng::SEED ^ 7);
    for _ in 0..N {
        let len = rng.range(4, 64);
        let mut s = rand_non_sep(&mut rng, len);
        let count = rng.range(2, 16.min(len));
        for _ in 0..count {
            let p = rng.below(len);
            s[p] = b'/';
        }
        check(&s, "row7 many '/'");
    }
}

#[test]
fn cfg_08_b3_slash_only_string() {
    check(b"/", "row8 \"/\"");
    check(b"//", "row8 \"//\"");
    check(b"///", "row8 \"///\"");
}

// -------------------------------------------------------------- rows 9..13
// B4: only '\\' present.

#[test]
fn cfg_09_b4_single_backslash_interior() {
    let mut rng = Rng::new(Rng::SEED ^ 9);
    for _ in 0..N {
        let len = rng.range(3, 48);
        let mut s = rand_non_sep(&mut rng, len);
        let pos = rng.range(1, len - 2);
        s[pos] = b'\\';
        check(&s, "row9 single '\\' interior");
    }
}

#[test]
fn cfg_10_b4_backslash_at_first_byte() {
    let mut rng = Rng::new(Rng::SEED ^ 10);
    for _ in 0..N {
        let len = rng.range(1, 40);
        let mut s = rand_non_sep(&mut rng, len);
        s[0] = b'\\';
        check(&s, "row10 '\\' first");
    }
}

#[test]
fn cfg_11_b4_backslash_at_last_byte() {
    let mut rng = Rng::new(Rng::SEED ^ 11);
    for _ in 0..N {
        let len = rng.range(1, 40);
        let mut s = rand_non_sep(&mut rng, len);
        *s.last_mut().unwrap() = b'\\';
        let r = compare(&s, "row11 '\\' last");
        assert!(r.tail.is_empty());
        assert_eq!(r.offset as usize, s.len());
    }
}

#[test]
fn cfg_12_b4_many_backslashes() {
    let mut rng = Rng::new(Rng::SEED ^ 12);
    for _ in 0..N {
        let len = rng.range(4, 64);
        let mut s = rand_non_sep(&mut rng, len);
        let count = rng.range(2, 16.min(len));
        for _ in 0..count {
            let p = rng.below(len);
            s[p] = b'\\';
        }
        check(&s, "row12 many '\\'");
    }
}

#[test]
fn cfg_13_b4_backslash_only_string() {
    check(b"\\", "row13");
    check(b"\\\\", "row13");
    check(b"\\\\\\", "row13");
}

// -------------------------------------------------------------- rows 14..19
// B1 / B2: both separators present; the LATER one wins.

#[test]
fn cfg_14_b1_last_slash_after_last_backslash() {
    let mut rng = Rng::new(Rng::SEED ^ 14);
    for _ in 0..N {
        let len = rng.range(4, 56);
        let mut s = rand_non_sep(&mut rng, len);
        let i2 = rng.below(len / 2); // backslash in the first half
        let i1 = rng.range(len / 2 + 1, len - 1); // slash strictly later
        s[i2] = b'\\';
        s[i1] = b'/';
        let e = expected_offset(&s);
        assert_eq!(e, i1 + 1, "generator failed to hit B1");
        check(&s, "row14 B1 s1>s2");
    }
}

#[test]
fn cfg_15_b2_last_backslash_after_last_slash() {
    let mut rng = Rng::new(Rng::SEED ^ 15);
    for _ in 0..N {
        let len = rng.range(4, 56);
        let mut s = rand_non_sep(&mut rng, len);
        let i1 = rng.below(len / 2);
        let i2 = rng.range(len / 2 + 1, len - 1);
        s[i1] = b'/';
        s[i2] = b'\\';
        let e = expected_offset(&s);
        assert_eq!(e, i2 + 1, "generator failed to hit B2");
        check(&s, "row15 B2 s2>s1");
    }
}

#[test]
fn cfg_16_adjacent_separators() {
    for fixed in [
        &b"a/\\b"[..],
        &b"a\\/b"[..],
        &b"/\\"[..],
        &b"\\/"[..],
        &b"x/\\"[..],
        &b"x\\/"[..],
        &b"/\\x"[..],
        &b"\\/x"[..],
    ] {
        check(fixed, "row16 adjacent");
    }
    let mut rng = Rng::new(Rng::SEED ^ 16);
    for _ in 0..N {
        let len = rng.range(2, 40);
        let mut s = rand_non_sep(&mut rng, len);
        let p = rng.below(len - 1);
        if rng.below(2) == 0 {
            s[p] = b'/';
            s[p + 1] = b'\\';
        } else {
            s[p] = b'\\';
            s[p + 1] = b'/';
        }
        check(&s, "row16 adjacent random");
    }
}

#[test]
fn cfg_17_many_of_each_interleaved() {
    let mut rng = Rng::new(Rng::SEED ^ 17);
    for _ in 0..(N * 2) {
        let len = rng.range(2, 72);
        let mut s = rand_non_sep(&mut rng, len);
        for _ in 0..rng.range(1, 12) {
            let p = rng.below(len);
            s[p] = b'/';
        }
        for _ in 0..rng.range(1, 12) {
            let p = rng.below(len);
            s[p] = b'\\';
        }
        check(&s, "row17 interleaved many");
    }
}

#[test]
fn cfg_18_both_present_one_trailing() {
    let mut rng = Rng::new(Rng::SEED ^ 18);
    for _ in 0..N {
        let len = rng.range(3, 48);
        let mut s = rand_non_sep(&mut rng, len);
        let interior = rng.below(len - 1);
        if rng.below(2) == 0 {
            s[interior] = b'/';
            *s.last_mut().unwrap() = b'\\';
        } else {
            s[interior] = b'\\';
            *s.last_mut().unwrap() = b'/';
        }
        let r = compare(&s, "row18 trailing + interior");
        assert!(r.tail.is_empty());
        assert_eq!(r.offset as usize, s.len());
    }
}

#[test]
fn cfg_19_only_separators() {
    let mut rng = Rng::new(Rng::SEED ^ 19);
    for _ in 0..N {
        let len = rng.range(1, 16);
        let s: Vec<u8> = (0..len).map(|_| rng.pick(b"/\\")).collect();
        let r = compare(&s, "row19 only separators");
        assert_eq!(r.offset as usize, len, "must point at the NUL terminator");
        assert!(r.tail.is_empty());
    }
}

// ------------------------------------------------------------------- row 20
#[test]
fn cfg_20_full_byte_alphabet_non_utf8() {
    let mut rng = Rng::new(Rng::SEED ^ 20);
    for _ in 0..(N * 2) {
        let len = rng.range(1, 64);
        // Any byte except NUL, so separators appear naturally too.
        let s: Vec<u8> = (0..len).map(|_| (rng.range(1, 255)) as u8).collect();
        check(&s, "row20 full byte alphabet");
    }
    // Deliberately invalid UTF-8 next to separators.
    check(&[0x80, b'/', 0xff, 0xfe], "row20 fixed");
    check(&[0xc3, b'\\', 0x28, 0x80], "row20 fixed");
    check(&[0xed, 0xa0, 0x80, b'/', 0xf5], "row20 surrogate/oob");
}

// ---------------------------------------------------------------- rows 21..23
const BIG: usize = 64 * 1024;

#[test]
fn cfg_21_oversized_no_separator() {
    let mut rng = Rng::new(Rng::SEED ^ 21);
    let s = rand_non_sep(&mut rng, BIG);
    let r = compare(&s, "row21 64KiB no separator");
    assert_eq!(r.offset, 0);
    assert_eq!(r.tail.len(), BIG);
}

#[test]
fn cfg_22_oversized_separator_at_end() {
    let mut rng = Rng::new(Rng::SEED ^ 22);
    for sep in [b'/', b'\\'] {
        let mut s = rand_non_sep(&mut rng, BIG);
        *s.last_mut().unwrap() = sep;
        let r = compare(&s, "row22 64KiB separator at end");
        assert_eq!(r.offset as usize, BIG);
        assert!(r.tail.is_empty());
    }
    // separator one byte before the end
    let mut s = rand_non_sep(&mut rng, BIG);
    s[BIG - 2] = b'/';
    let r = compare(&s, "row22 64KiB separator at end-1");
    assert_eq!(r.offset as usize, BIG - 1);
}

#[test]
fn cfg_23_oversized_random_separators() {
    let mut rng = Rng::new(Rng::SEED ^ 23);
    for _ in 0..12 {
        let mut s = rand_non_sep(&mut rng, BIG);
        for _ in 0..rng.range(1, 500) {
            let p = rng.below(BIG);
            s[p] = if rng.below(2) == 0 { b'/' } else { b'\\' };
        }
        check(&s, "row23 64KiB random separators");
    }
}

// ------------------------------------------------------------------- row 24
#[test]
fn cfg_24_interior_nul_stops_the_search() {
    // `strrchr` stops at the first NUL, so the trailing "/d" is invisible.
    let raw = b"a/b\0c/d\0";
    let r = compare_raw(raw, "row24 interior NUL");
    assert_eq!(r.offset, 2, "search must stop at the first NUL");
    assert_eq!(r.tail, b"b");

    let raw2 = b"abc\0/x\0";
    let r2 = compare_raw(raw2, "row24 separator only after NUL");
    assert_eq!(r2.offset, 0);
    assert_eq!(r2.tail, b"abc");

    let mut rng = Rng::new(Rng::SEED ^ 24);
    for _ in 0..N {
        let head_len = rng.range(0, 24);
        let tail_len = rng.range(1, 24);
        let mut raw = rand_non_sep(&mut rng, head_len);
        if head_len > 0 && rng.below(2) == 0 {
            let p = rng.below(head_len);
            raw[p] = if rng.below(2) == 0 { b'/' } else { b'\\' };
        }
        let visible = raw.clone();
        raw.push(0);
        for _ in 0..tail_len {
            raw.push(rng.pick(b"ab/\\x"));
        }
        raw.push(0);
        let r = compare_raw(&raw, "row24 random interior NUL");
        assert_eq!(r.offset as usize, expected_offset(&visible));
    }
}

// ------------------------------------------------------------------- row 25
#[test]
fn cfg_25_basename_of_basename() {
    let l = libs();
    let mut rng = Rng::new(Rng::SEED ^ 25);
    for _ in 0..N {
        let len = rng.range(1, 48);
        let mut s: Vec<u8> = (0..len).map(|_| rng.pick(b"ab/\\.x")).collect();
        s.push(0);

        let mut bc = s.clone();
        let mut br = s.clone();

        let (oc, or) = unsafe {
            let base_c = bc.as_mut_ptr() as *mut std::ffi::c_char;
            let base_r = br.as_mut_ptr() as *mut std::ffi::c_char;
            let c1 = (l.c)(base_c);
            let c2 = (l.c)(c1);
            let r1 = (l.rust)(base_r);
            let r2 = (l.rust)(r1);
            (
                c2 as isize - base_c as isize,
                r2 as isize - base_r as isize,
            )
        };
        assert_eq!(
            oc,
            or,
            "DIVERGENCE row25 nested call input={:?}",
            Bytes(&s)
        );
        // A basename contains no separator, so it is a fixed point.
        let first = expected_offset(&s[..s.len() - 1]);
        assert_eq!(oc as usize, first, "basename must be idempotent");
    }
}

// ------------------------------------------------------------------- row 26
#[test]
fn cfg_26_repeat_call_stability_and_no_mutation() {
    let l = libs();
    let mut rng = Rng::new(Rng::SEED ^ 26);
    for _ in 0..N {
        let len = rng.range(0, 40);
        let mut s: Vec<u8> = (0..len).map(|_| rng.pick(b"ab/\\.0")).collect();
        let original = s.clone();
        s.push(0);
        let mut buf = s.clone();
        let base = buf.as_mut_ptr() as *mut std::ffi::c_char;

        let mut offsets = Vec::new();
        for _ in 0..5 {
            let c = unsafe { (l.c)(base) } as isize - base as isize;
            let r = unsafe { (l.rust)(base) } as isize - base as isize;
            assert_eq!(c, r, "DIVERGENCE row26 input={:?}", Bytes(&original));
            offsets.push(c);
        }
        assert!(
            offsets.windows(2).all(|w| w[0] == w[1]),
            "result not stable across repeated calls"
        );
        assert_eq!(buf, s, "buffer was mutated");
    }
}

// ------------------------------------------------------------------- row 27
#[test]
fn cfg_27_fuzz_sweep() {
    let mut rng = Rng::new(Rng::SEED ^ 27);
    let alphabet: &[u8] = &[b'a', b'b', b'/', b'\\', b'.', b'0', b'[', b']', 0x80, 0xff];
    for _ in 0..20_000 {
        let len = rng.below(81);
        let s: Vec<u8> = (0..len).map(|_| rng.pick(alphabet)).collect();
        check(&s, "row27 fuzz");
    }
}
