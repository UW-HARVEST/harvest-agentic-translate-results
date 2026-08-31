//! Phase B/C differential tests for the `utf.c` module:
//! `jsU_chartorune`, `jsU_runetochar`, `jsU_runelen`, `jsU_isalpharune`,
//! `jsU_islowerrune`, `jsU_isupperrune`, `jsU_tolowerrune`, `jsU_toupperrune`,
//! `jsU_tolowerrune_full`, `jsU_toupperrune_full`.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

const RUNEERROR: c_int = 0xFFFD;
const RUNEMAX: c_int = 0x10FFFF;

/// Decode with both impls. Input is padded with 8 NULs because `chartorune`
/// unconditionally reads up to `str[3]` (and `str[1]` on entry).
fn chartorune_both(bytes: &[u8]) -> ((c_int, c_int), (c_int, c_int)) {
    let (fc, fr) = pair::<FnChartorune>("jsU_chartorune");
    let mut buf = bytes.to_vec();
    buf.extend_from_slice(&[0u8; 8]);
    let mut rc: c_int = -12345;
    let mut rr: c_int = -12345;
    let nc = unsafe { fc(&mut rc, buf.as_ptr() as *const c_char) };
    let nr = unsafe { fr(&mut rr, buf.as_ptr() as *const c_char) };
    ((nc, rc), (nr, rr))
}

fn runetochar_both(r: c_int) -> ((c_int, Vec<u8>), (c_int, Vec<u8>)) {
    let (fc, fr) = pair::<FnRunetochar>("jsU_runetochar");
    // C uses `char str[10]` internally in runelen; give the same headroom.
    let mut bc = [0xAAu8; 16];
    let mut br = [0xAAu8; 16];
    let nc = unsafe { fc(bc.as_mut_ptr() as *mut c_char, &r) };
    let nr = unsafe { fr(br.as_mut_ptr() as *mut c_char, &r) };
    // Compare the whole scratch buffer: catches an impl writing extra bytes.
    ((nc, bc.to_vec()), (nr, br.to_vec()))
}

// ---------------------------------------------------------------------------
// CONFIGS rows: chartorune over every input shape
// ---------------------------------------------------------------------------

#[test]
fn chartorune_all_single_bytes() {
    // Shape: every possible lead byte, followed by NUL.
    let mut b = Batch::new();
    for lead in 0u16..=255 {
        let bytes = [lead as u8];
        let (c, r) = chartorune_both(&bytes);
        b.check(&format!("chartorune([{lead:#04x}])"), c, r);
    }
    b.finish("chartorune single lead bytes");
}

#[test]
fn chartorune_all_two_byte_pairs() {
    // Shape: exhaustive 2-byte space (65536 cases) -- covers the overlong-NUL
    // special case (0xC0 0x80), C0/C1 overlongs, truncated 3/4-byte sequences.
    let mut b = Batch::new();
    for lead in 0u16..=255 {
        for second in 0u16..=255 {
            let bytes = [lead as u8, second as u8];
            let (c, r) = chartorune_both(&bytes);
            b.check(&format!("chartorune([{lead:#04x},{second:#04x}])"), c, r);
        }
    }
    b.finish("chartorune exhaustive 2-byte");
}

#[test]
fn chartorune_exhaustive_three_byte_lead_ranges() {
    // Shape: exhaustive over 3-byte sequences whose lead is in the interesting
    // ranges (E0..EF for 3-byte, F0..F7 for 4-byte lead), all continuations.
    let mut b = Batch::new();
    for lead in 0xC0u16..=0xFF {
        for c1 in 0u16..=255 {
            for c2 in [0u16, 0x7F, 0x80, 0xBF, 0xC0, 0xFF, 0x9F, 0xA0] {
                let bytes = [lead as u8, c1 as u8, c2 as u8];
                let (c, r) = chartorune_both(&bytes);
                b.check(&format!("chartorune([{lead:#04x},{c1:#04x},{c2:#04x}])"), c, r);
            }
        }
    }
    b.finish("chartorune 3-byte sequences");
}

#[test]
fn chartorune_four_byte_sequences() {
    // Shape: 4-byte sequences including > Runemax rejections (F4 90.. and up)
    // and the F5..FF invalid leads.
    let mut b = Batch::new();
    for lead in 0xF0u16..=0xFF {
        for c1 in 0u16..=255 {
            for c2 in [0x80u16, 0xBF, 0x00, 0xC0] {
                for c3 in [0x80u16, 0xBF, 0x00, 0xC0] {
                    let bytes = [lead as u8, c1 as u8, c2 as u8, c3 as u8];
                    let (c, r) = chartorune_both(&bytes);
                    b.check(
                        &format!("chartorune([{lead:#04x},{c1:#04x},{c2:#04x},{c3:#04x}])"),
                        c,
                        r,
                    );
                }
            }
        }
    }
    b.finish("chartorune 4-byte sequences");
}

#[test]
fn chartorune_randomized_byte_soup() {
    // Property test: random byte strings, many of them not valid UTF-8.
    let mut b = Batch::new();
    let mut rng = Rng::new(0xC0FFEE_1234);
    for _ in 0..40_000 {
        let n = 1 + rng.below(6) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
        let (c, r) = chartorune_both(&bytes);
        b.check(&format!("chartorune({bytes:02x?})"), c, r);
    }
    b.finish("chartorune random bytes");
}

#[test]
fn chartorune_valid_encodings_roundtrip() {
    // Shape: every rune encoded by runetochar must decode back identically in
    // both impls (and to the same value).
    let mut b = Batch::new();
    let (rtc_c, _) = pair::<FnRunetochar>("jsU_runetochar");
    let mut rng = Rng::new(0xABCDEF01);
    let mut runes: Vec<c_int> = vec![
        0, 1, 0x7F, 0x80, 0x7FF, 0x800, 0xFFF, 0xD7FF, 0xD800, 0xDBFF, 0xDC00, 0xDFFF, 0xE000,
        0xFFFD, 0xFFFF, 0x10000, 0x10FFFF, 0x110000, 0x1FFFFF, 0x200000, RUNEMAX, RUNEMAX + 1,
    ];
    for _ in 0..4000 {
        runes.push(rng.below(0x110000) as c_int);
    }
    for &rune in &runes {
        let mut enc = [0u8; 16];
        let n = unsafe { rtc_c(enc.as_mut_ptr() as *mut c_char, &rune) };
        let (c, r) = chartorune_both(&enc[..n as usize]);
        b.check(&format!("roundtrip rune={rune:#x} enc={:02x?}", &enc[..n as usize]), c, r);
    }
    b.finish("chartorune roundtrip of encoded runes");
}

// ---------------------------------------------------------------------------
// runetochar / runelen
// ---------------------------------------------------------------------------

#[test]
fn runetochar_boundaries_and_invalid() {
    // Shape: boundary runes, negatives, > Runemax, i32 extremes.
    let mut b = Batch::new();
    let mut cases: Vec<c_int> = vec![
        c_int::MIN, c_int::MIN + 1, -0x110000, -1000, -128, -2, -1, 0, 1, 0x7E, 0x7F, 0x80, 0x81,
        0x7FE, 0x7FF, 0x800, 0x801, 0xD7FF, 0xD800, 0xDFFF, 0xE000, 0xFFFE, 0xFFFD, 0xFFFF,
        0x10000, 0x10FFFE, RUNEMAX, RUNEMAX + 1, 0x1FFFFF, 0x200000, 0x7FFFFFFE, c_int::MAX,
    ];
    let mut rng = Rng::new(0x5EED_0001);
    for _ in 0..20_000 {
        cases.push(rng.next_u32() as c_int);
    }
    for &r in &cases {
        let (a, bb) = runetochar_both(r);
        b.check(&format!("runetochar({r:#x})"), a, bb);
    }
    b.finish("runetochar");
}

#[test]
fn runetochar_exhaustive_low_and_boundary_windows() {
    // Exhaustive over every rune in the ranges the code branches on.
    let mut b = Batch::new();
    let windows: [(i64, i64); 7] = [
        (-8, 300),
        (0x7F0, 0x810),
        (0xFFF0, 0x10010),
        (0x10FFF0, 0x110010),
        (0xD7F0, 0xE010),
        (0x1FFFF0, 0x200010),
        (0x7FFFFFF0, 0x7FFFFFFF),
    ];
    for (lo, hi) in windows {
        let mut v = lo;
        while v <= hi {
            let r = v as c_int;
            let (a, bb) = runetochar_both(r);
            b.check(&format!("runetochar({r:#x})"), a, bb);
            v += 1;
        }
    }
    b.finish("runetochar boundary windows");
}

#[test]
fn runelen_exhaustive_and_random() {
    let (fc, fr) = pair::<FnRunelen>("jsU_runelen");
    let mut b = Batch::new();
    let mut cases: Vec<c_int> = Vec::new();
    for v in -300i64..=0x11_0010 {
        // sample densely near boundaries, sparsely elsewhere
        let near = |x: i64| {
            (v - x).abs() < 8
        };
        if v < 300
            || near(0x7F)
            || near(0x80)
            || near(0x7FF)
            || near(0x800)
            || near(0xFFFF)
            || near(0x10000)
            || near(RUNEMAX as i64)
            || v % 977 == 0
        {
            cases.push(v as c_int);
        }
    }
    cases.extend([c_int::MIN, c_int::MIN + 1, -1, 0, c_int::MAX, c_int::MAX - 1, RUNEMAX + 1]);
    let mut rng = Rng::new(0x5EED_0002);
    for _ in 0..20_000 {
        cases.push(rng.next_u32() as c_int);
    }
    for &r in &cases {
        let a = unsafe { fc(r) };
        let bb = unsafe { fr(r) };
        b.check(&format!("runelen({r:#x})"), a, bb);
    }
    b.finish("runelen");
}

// ---------------------------------------------------------------------------
// Unicode character-database predicates and case mappings
// ---------------------------------------------------------------------------

/// The full Unicode range plus out-of-range/negative inputs.
fn ucd_probe_runes() -> Vec<c_int> {
    let mut v: Vec<c_int> = Vec::new();
    // Every code point in the BMP + astral planes, strided, plus all of the BMP
    // exhaustively (the tables are dense there).
    for c in 0..0x11000 {
        v.push(c);
    }
    let mut c = 0x11000i64;
    while c <= 0x110010 {
        v.push(c as c_int);
        c += 1;
    }
    v.extend([
        c_int::MIN, c_int::MIN + 1, -0x10FFFF, -1000, -2, -1, RUNEMAX, RUNEMAX + 1, 0x1FFFFF,
        0x7FFFFFFF, c_int::MAX - 1,
    ]);
    let mut rng = Rng::new(0x5EED_0003);
    for _ in 0..20_000 {
        v.push(rng.next_u32() as c_int);
    }
    v
}

#[test]
fn ucd_predicates_match() {
    let runes = ucd_probe_runes();
    for name in ["jsU_isalpharune", "jsU_islowerrune", "jsU_isupperrune"] {
        let (fc, fr) = pair::<FnRunePred>(name);
        let mut b = Batch::new();
        for &r in &runes {
            let a = unsafe { fc(r) };
            let bb = unsafe { fr(r) };
            b.check(&format!("{name}({r:#x})"), a, bb);
        }
        b.finish(name);
    }
}

#[test]
fn ucd_case_maps_match() {
    let runes = ucd_probe_runes();
    for name in ["jsU_tolowerrune", "jsU_toupperrune"] {
        let (fc, fr) = pair::<FnRuneMap>(name);
        let mut b = Batch::new();
        for &r in &runes {
            let a = unsafe { fc(r) };
            let bb = unsafe { fr(r) };
            b.check(&format!("{name}({r:#x})"), a, bb);
        }
        b.finish(name);
    }
}

#[test]
fn ucd_full_case_maps_match() {
    // `tolowerrune_full` / `toupperrune_full` return a NUL(0)-terminated Rune*
    // (or NULL when there is no multi-character mapping).
    let runes = ucd_probe_runes();
    for name in ["jsU_tolowerrune_full", "jsU_toupperrune_full"] {
        let (fc, fr) = pair::<FnRuneMapFull>(name);
        let mut b = Batch::new();
        for &r in &runes {
            let a = unsafe { read_rune_seq(fc(r)) };
            let bb = unsafe { read_rune_seq(fr(r)) };
            b.check(&format!("{name}({r:#x})"), a, bb);
        }
        b.finish(name);
    }
}

unsafe fn read_rune_seq(p: *const c_int) -> Option<Vec<c_int>> {
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    let mut i = 0isize;
    while i < 8 {
        let r = *p.offset(i);
        if r == 0 {
            break;
        }
        v.push(r);
        i += 1;
    }
    Some(v)
}

// ---------------------------------------------------------------------------
// Phase C: error / edge conditions specific to utf.c
// ---------------------------------------------------------------------------

#[test]
fn chartorune_returns_runeerror_for_bad_sequences() {
    // ERRORS row: every `goto bad` path yields *rune = Runeerror(0xFFFD), ret 1.
    // Verified against C, then asserted equal in Rust.
    let bad: &[&[u8]] = &[
        &[0x80],             // bare continuation
        &[0xBF],             // bare continuation
        &[0xC0, 0x00],       // overlong (not the 0xC0 0x80 special case)
        &[0xC1, 0x80],       // overlong 2-byte
        &[0xC2, 0x00],       // bad continuation
        &[0xE0, 0x80, 0x80], // overlong 3-byte
        &[0xE0, 0x00, 0x00],
        &[0xF0, 0x80, 0x80, 0x80], // overlong 4-byte
        &[0xF4, 0x90, 0x80, 0x80], // > Runemax
        &[0xF5, 0x80, 0x80, 0x80], // > Runemax
        &[0xF7, 0xBF, 0xBF, 0xBF], // > Runemax
        &[0xF8, 0x80, 0x80, 0x80], // 5-byte lead: invalid
        &[0xFF, 0xFF, 0xFF, 0xFF],
    ];
    let mut b = Batch::new();
    for bytes in bad {
        let ((nc, rc), (nr, rr)) = chartorune_both(bytes);
        // Ground truth from C:
        assert_eq!((nc, rc), (1, RUNEERROR), "C did not reject {bytes:02x?} as expected");
        b.check(&format!("bad seq {bytes:02x?}"), (nc, rc), (nr, rr));
    }
    b.finish("chartorune bad-sequence rejection");
}

#[test]
fn chartorune_overlong_nul_special_case() {
    // ERRORS/CONFIGS row: 0xC0 0x80 decodes to rune 0 with length 2.
    let ((nc, rc), (nr, rr)) = chartorune_both(&[0xC0, 0x80]);
    assert_eq!((nc, rc), (2, 0), "C overlong-NUL contract changed");
    assert_eq!((nr, rr), (2, 0), "Rust overlong-NUL diverges");
}

#[test]
fn runetochar_zero_emits_overlong_nul() {
    // ERRORS/CONFIGS row: rune 0 encodes as the 2-byte overlong 0xC0 0x80.
    let ((nc, bc), (nr, br)) = runetochar_both(0);
    assert_eq!(nc, 2);
    assert_eq!(&bc[..2], &[0xC0, 0x80]);
    assert_eq!((nc, &bc[..2]), (nr, &br[..2]));
}

#[test]
fn runetochar_above_runemax_becomes_runeerror() {
    // ERRORS row: c > Runemax is replaced with Runeerror before encoding.
    let (fc, _) = pair::<FnRunetochar>("jsU_runetochar");
    let mut enc = [0u8; 16];
    let r: c_int = RUNEMAX + 1;
    let n = unsafe { fc(enc.as_mut_ptr() as *mut c_char, &r) };
    assert_eq!(n, 3, "C should encode Runeerror in 3 bytes");
    assert_eq!(&enc[..3], &[0xEF, 0xBF, 0xBD], "C should emit U+FFFD");
    let ((a1, a2), (b1, b2)) = runetochar_both(RUNEMAX + 1);
    assert_eq!((a1, &a2[..4]), (b1, &b2[..4]));
}
