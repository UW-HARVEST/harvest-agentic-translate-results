// Level 1: utf.c -- pure leaf functions, no js_State involved.
mod common;

use common::both;
use std::os::raw::c_char;

type Rune = i32;

#[test]
fn chartorune_matches() {
    let (c, r) = unsafe { both::<unsafe extern "C-unwind" fn(*mut Rune, *const c_char) -> i32>("jsU_chartorune") };

    // Build a corpus of byte sequences: all 1-byte, plus structured multi-byte,
    // plus exhaustive 2-byte prefixes and a sampling of 3/4-byte sequences.
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for b in 1u16..=255 {
        cases.push(vec![b as u8, 0]);
    }
    for b0 in 0x80u16..=0xFFu16 {
        for b1 in 1u16..=255 {
            cases.push(vec![b0 as u8, b1 as u8, 0]);
        }
    }
    // 3-byte / 4-byte sequences, sampled deterministically.
    let tails: [u8; 10] = [0x01, 0x20, 0x7F, 0x80, 0x81, 0xA5, 0xBF, 0xC0, 0xEF, 0xFF];
    for b0 in [0xC0u8, 0xC1, 0xC2, 0xDF, 0xE0, 0xE1, 0xED, 0xEF, 0xF0, 0xF1, 0xF4, 0xF5, 0xF7, 0xF8, 0xFC, 0xFE, 0xFF] {
        for &b1 in tails.iter() {
            for &b2 in tails.iter() {
                cases.push(vec![b0, b1, b2, 0]);
                for &b3 in tails.iter() {
                    cases.push(vec![b0, b1, b2, b3, 0]);
                }
            }
        }
    }
    // Known-good encodings.
    for s in [
        "a", "é", "€", "𝄞", "\u{7f}", "\u{80}", "\u{7ff}", "\u{800}", "\u{ffff}",
        "\u{10000}", "\u{10ffff}", "\u{fffd}", "ключ", "日本語",
    ] {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        cases.push(v);
    }
    cases.push(vec![0]); // empty string

    for bytes in &cases {
        let mut rc: Rune = -12345;
        let mut rr: Rune = -12345;
        let nc = unsafe { c(&mut rc, bytes.as_ptr() as *const c_char) };
        let nr = unsafe { r(&mut rr, bytes.as_ptr() as *const c_char) };
        assert_eq!(
            (nc, rc),
            (nr, rr),
            "chartorune mismatch for {:02X?}: C=({},{:#x}) Rust=({},{:#x})",
            bytes,
            nc,
            rc,
            nr,
            rr
        );
    }
}

#[test]
fn runetochar_matches() {
    let (c, r) = unsafe { both::<unsafe extern "C-unwind" fn(*mut c_char, *const Rune) -> i32>("jsU_runetochar") };
    let mut runes: Vec<Rune> = Vec::new();
    for v in -5..0x2200 {
        runes.push(v);
    }
    for v in [
        0xD7FF, 0xD800, 0xDBFF, 0xDC00, 0xDFFF, 0xE000, 0xFFFD, 0xFFFE, 0xFFFF, 0x10000, 0x1FFFF,
        0x10FFFE, 0x10FFFF, 0x110000, 0x1FFFFF, 0x200000, 0x7FFFFFFF, -1, -0x10000, i32::MIN,
        i32::MAX,
    ] {
        runes.push(v);
    }
    for v in (0x2200..0x110100).step_by(97) {
        runes.push(v);
    }

    for &rune in &runes {
        let mut bc = [0u8; 32];
        let mut br = [0u8; 32];
        let nc = unsafe { c(bc.as_mut_ptr() as *mut c_char, &rune) };
        let nr = unsafe { r(br.as_mut_ptr() as *mut c_char, &rune) };
        assert_eq!(nc, nr, "runetochar length mismatch for {:#x}", rune);
        assert_eq!(
            &bc[..nc.max(0) as usize],
            &br[..nr.max(0) as usize],
            "runetochar bytes mismatch for {:#x}",
            rune
        );
    }
}

#[test]
fn runelen_matches() {
    let (c, r) = unsafe { both::<unsafe extern "C-unwind" fn(i32) -> i32>("jsU_runelen") };
    for v in -3..0x2000 {
        assert_eq!(unsafe { c(v) }, unsafe { r(v) }, "runelen({:#x})", v);
    }
    for v in (0..0x110100i32).step_by(31) {
        assert_eq!(unsafe { c(v) }, unsafe { r(v) }, "runelen({:#x})", v);
    }
    for v in [i32::MIN, -1, 0x10FFFF, 0x110000, i32::MAX] {
        assert_eq!(unsafe { c(v) }, unsafe { r(v) }, "runelen({:#x})", v);
    }
}

fn rune_corpus() -> Vec<Rune> {
    let mut v: Vec<Rune> = (-4..0x3200).collect();
    v.extend((0x3200..0x110200i32).step_by(7));
    v.extend([i32::MIN, i32::MAX, -1, 0x1E900, 0x1E921, 0x10400, 0x10428]);
    v
}

#[test]
fn rune_class_and_case_match() {
    for name in [
        "jsU_isalpharune",
        "jsU_islowerrune",
        "jsU_isupperrune",
        "jsU_tolowerrune",
        "jsU_toupperrune",
    ] {
        let (c, r) = unsafe { both::<unsafe extern "C-unwind" fn(Rune) -> i32>(name) };
        for &x in rune_corpus().iter() {
            assert_eq!(unsafe { c(x) }, unsafe { r(x) }, "{}({:#x})", name, x);
        }
    }
}

#[test]
fn rune_case_full_match() {
    for name in ["jsU_tolowerrune_full", "jsU_toupperrune_full"] {
        let (c, r) = unsafe { both::<unsafe extern "C-unwind" fn(Rune) -> *const Rune>(name) };
        for &x in rune_corpus().iter() {
            let pc = unsafe { c(x) };
            let pr = unsafe { r(x) };
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "{}({:#x}) nullness differs (C null={}, Rust null={})",
                name,
                x,
                pc.is_null(),
                pr.is_null()
            );
            if !pc.is_null() {
                // The returned array is NUL(0)-terminated in the tables.
                let mut vc = Vec::new();
                let mut vr = Vec::new();
                for i in 0..4isize {
                    let a = unsafe { *pc.offset(i) };
                    let b = unsafe { *pr.offset(i) };
                    vc.push(a);
                    vr.push(b);
                    if a == 0 {
                        break;
                    }
                }
                assert_eq!(vc, vr, "{}({:#x}) expansion differs", name, x);
            }
        }
    }
}
