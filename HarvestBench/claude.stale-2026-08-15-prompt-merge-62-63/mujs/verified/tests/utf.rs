//! Phase B rows A1..A16 — `utf.c` differential tests.
//! Every call goes through the `.so` exports of both libraries.
#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int};

/// Decode one byte string with both libraries; returns (retlen, rune).
fn chartorune(api: &Api, bytes: &[u8]) -> (c_int, c_int) {
    // Always NUL-terminate: chartorune relies on the terminator.
    let mut buf: Vec<u8> = bytes.to_vec();
    buf.push(0);
    let mut rune: c_int = -12345;
    let n = unsafe { (api.jsU_chartorune)(&mut rune, buf.as_ptr() as *const c_char) };
    (n, rune)
}

fn diff_chartorune(label: &str, cases: impl Iterator<Item = Vec<u8>>) {
    let cases: Vec<Vec<u8>> = cases.collect();
    let (cr, rr) = both(|api, _| {
        cases.iter().map(|b| chartorune(api, b)).collect::<Vec<_>>()
    });
    for (i, (c, r)) in cr.iter().zip(rr.iter()).enumerate() {
        assert_eq!(
            c, r,
            "chartorune DIVERGENCE [{}] input={:02x?}: C={:?} Rust={:?}",
            label, cases[i], c, r
        );
    }
}

// ---------------------------------------------------------------- A1, A2, A6, A9
#[test]
fn a1_a2_a6_a9_chartorune_all_single_bytes() {
    diff_chartorune("single bytes 0x00..0xFF", (0u32..=0xFF).map(|b| vec![b as u8]));
}

// ---------------------------------------------------------------- A3
#[test]
fn a3_chartorune_all_two_byte_sequences() {
    let mut cases = Vec::new();
    for b0 in 0xC0u32..=0xDF {
        for b1 in 0x00u32..=0xFF {
            if b1 == 0 {
                cases.push(vec![b0 as u8]); // truncated
            } else {
                cases.push(vec![b0 as u8, b1 as u8]);
            }
        }
    }
    diff_chartorune("2-byte", cases.into_iter());
}

// ---------------------------------------------------------------- A4, A8
#[test]
fn a4_a8_chartorune_all_three_byte_sequences() {
    // Exhaustive over lead+first continuation, and a sweep of second bytes.
    let mut cases = Vec::new();
    for b0 in 0xE0u32..=0xEF {
        for b1 in 1u32..=0xFF {
            for b2 in [1u32, 0x40, 0x7F, 0x80, 0xA0, 0xBF, 0xC0, 0xFF] {
                cases.push(vec![b0 as u8, b1 as u8, b2 as u8]);
            }
            cases.push(vec![b0 as u8, b1 as u8]); // truncated
        }
    }
    diff_chartorune("3-byte", cases.into_iter());
}

#[test]
fn a4_chartorune_surrogates_and_overlong() {
    let mut cases: Vec<Vec<u8>> = vec![
        vec![0xC0, 0x80],             // overlong NUL
        vec![0xC1, 0xBF],             // overlong
        vec![0xE0, 0x80, 0x80],       // overlong
        vec![0xE0, 0x9F, 0xBF],       // overlong
        vec![0xF0, 0x80, 0x80, 0x80], // overlong
        vec![0xF0, 0x8F, 0xBF, 0xBF], // overlong
        vec![0xED, 0xA0, 0x80],       // U+D800 surrogate
        vec![0xED, 0xBF, 0xBF],       // U+DFFF surrogate
        vec![0xEF, 0xBF, 0xBD],       // U+FFFD
        vec![0xF4, 0x8F, 0xBF, 0xBF], // U+10FFFF = Runemax
        vec![0xF4, 0x90, 0x80, 0x80], // Runemax+1
        vec![0xF7, 0xBF, 0xBF, 0xBF], // 0x1FFFFF
        vec![0xF8, 0x88, 0x80, 0x80, 0x80],
        vec![0xFC, 0x84, 0x80, 0x80, 0x80, 0x80],
        vec![0xFE, 0x80],
        vec![0xFF, 0xFF],
    ];
    // every surrogate code point, encoded as 3-byte UTF-8 (CESU style)
    for cp in 0xD800u32..=0xDFFF {
        cases.push(vec![
            (0xE0 | (cp >> 12)) as u8,
            (0x80 | ((cp >> 6) & 0x3F)) as u8,
            (0x80 | (cp & 0x3F)) as u8,
        ]);
    }
    diff_chartorune("surrogates/overlong", cases.into_iter());
}

// ---------------------------------------------------------------- A5, A7
#[test]
fn a5_a7_chartorune_four_byte_and_truncated() {
    let mut cases = Vec::new();
    for b0 in 0xF0u32..=0xFF {
        for b1 in [1u32, 0x7F, 0x80, 0x8F, 0x90, 0xBF, 0xC0, 0xFF] {
            for b2 in [1u32, 0x80, 0xBF, 0xC0] {
                for b3 in [1u32, 0x80, 0xBF, 0xC0] {
                    cases.push(vec![b0 as u8, b1 as u8, b2 as u8, b3 as u8]);
                }
                cases.push(vec![b0 as u8, b1 as u8, b2 as u8]); // truncated
            }
            cases.push(vec![b0 as u8, b1 as u8]); // truncated
        }
        cases.push(vec![b0 as u8]); // truncated
    }
    diff_chartorune("4-byte", cases.into_iter());
}

// ---------------------------------------------------------------- A10
#[test]
fn a10_chartorune_randomized_byte_strings() {
    let mut rng = Rng::new(0xC0FFEE_1234);
    let mut cases = Vec::new();
    for _ in 0..40000 {
        let len = 1 + rng.below(6) as usize;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            // bias towards lead/continuation bytes
            let b = match rng.below(4) {
                0 => rng.next_u32() as u8,
                1 => 0x80 | (rng.next_u32() as u8 & 0x3F),
                2 => 0xC0 | (rng.next_u32() as u8 & 0x3F),
                _ => rng.next_u32() as u8 & 0x7F,
            };
            v.push(if b == 0 { 1 } else { b });
        }
        cases.push(v);
    }
    diff_chartorune("random", cases.into_iter());
}

// ---------------------------------------------------------------- A11
#[test]
fn a11_runetochar_all_runes() {
    let runes: Vec<c_int> = (-2000i64..0x110200i64)
        .step_by(1)
        .map(|x| x as c_int)
        .chain([
            i32::MIN,
            i32::MIN + 1,
            -1,
            0x7FFFFFFF,
            0x7FFFFFFE,
            0x10FFFF,
            0x110000,
            0xFFFD,
        ])
        .collect();
    let (cr, rr) = both(|api, _| {
        let mut out: Vec<(c_int, Vec<u8>)> = Vec::with_capacity(runes.len());
        for &r in &runes {
            // UTFmax is 4 but runetochar can emit more for out-of-range runes:
            // give it a generous buffer and a poison pattern so we detect writes.
            let mut buf = [0xAAu8; 32];
            let n = unsafe { (api.jsU_runetochar)(buf.as_mut_ptr() as *mut c_char, &r) };
            out.push((n, buf.to_vec()));
        }
        out
    });
    for (i, (c, r)) in cr.iter().zip(rr.iter()).enumerate() {
        assert_eq!(
            c, r,
            "runetochar DIVERGENCE rune={:#x}({}): C=({}, {:02x?}) Rust=({}, {:02x?})",
            runes[i], runes[i], c.0, c.1, r.0, r.1
        );
    }
}

// ---------------------------------------------------------------- A12
#[test]
fn a12_runelen_all_runes() {
    let runes: Vec<c_int> = (-2000i64..0x110200i64)
        .map(|x| x as c_int)
        .chain([i32::MIN, i32::MIN + 1, -1, 0x7FFFFFFF])
        .collect();
    let (cr, rr) = both(|api, _| {
        runes
            .iter()
            .map(|&r| unsafe { (api.jsU_runelen)(r) })
            .collect::<Vec<_>>()
    });
    for (i, (c, r)) in cr.iter().zip(rr.iter()).enumerate() {
        assert_eq!(c, r, "runelen DIVERGENCE rune={:#x}: C={} Rust={}", runes[i], c, r);
    }
}

// ---------------------------------------------------------------- A13
#[test]
fn a13_roundtrip_runetochar_chartorune() {
    let (cr, rr) = both(|api, _| {
        let mut out = Vec::new();
        for r in (0i64..0x110100).map(|x| x as c_int) {
            let mut buf = [0u8; 16];
            let n = unsafe { (api.jsU_runetochar)(buf.as_mut_ptr() as *mut c_char, &r) };
            buf[n.clamp(0, 15) as usize] = 0;
            let mut back: c_int = -1;
            let m = unsafe { (api.jsU_chartorune)(&mut back, buf.as_ptr() as *const c_char) };
            out.push((n, m, back));
        }
        out
    });
    assert_eq!(cr.len(), rr.len());
    for (i, (c, r)) in cr.iter().zip(rr.iter()).enumerate() {
        assert_eq!(c, r, "roundtrip DIVERGENCE rune={:#x}: C={:?} Rust={:?}", i, c, r);
    }
}

// ---------------------------------------------------------------- A14
#[test]
fn a14_rune_class_predicates_exhaustive() {
    for (name, get) in [
        ("isalpharune", 0usize),
        ("islowerrune", 1),
        ("isupperrune", 2),
    ] {
        let (cr, rr) = both(|api, _| {
            let f = match get {
                0 => api.jsU_isalpharune,
                1 => api.jsU_islowerrune,
                _ => api.jsU_isupperrune,
            };
            let mut v = Vec::with_capacity(0x110200);
            for c in (-256i64..0x110200i64).map(|x| x as c_int) {
                v.push(unsafe { f(c) });
            }
            v
        });
        let mut base = -256i64;
        for (c, r) in cr.iter().zip(rr.iter()) {
            assert_eq!(c, r, "{} DIVERGENCE rune={:#x}: C={} Rust={}", name, base, c, r);
            base += 1;
        }
    }
}

// ---------------------------------------------------------------- A15
#[test]
fn a15_case_conversion_exhaustive() {
    for (name, which) in [("tolowerrune", 0usize), ("toupperrune", 1)] {
        let (cr, rr) = both(|api, _| {
            let f = if which == 0 { api.jsU_tolowerrune } else { api.jsU_toupperrune };
            let mut v = Vec::with_capacity(0x110200);
            for c in (-256i64..0x110200i64).map(|x| x as c_int) {
                v.push(unsafe { f(c) });
            }
            v
        });
        let mut base = -256i64;
        for (c, r) in cr.iter().zip(rr.iter()) {
            assert_eq!(c, r, "{} DIVERGENCE rune={:#x}: C={:#x} Rust={:#x}", name, base, c, r);
            base += 1;
        }
    }
}

// ---------------------------------------------------------------- A16
#[test]
fn a16_case_conversion_full_exhaustive() {
    /// Read the NUL(0)-terminated Rune array returned by *_full.
    unsafe fn read_full(p: *const c_int) -> Option<Vec<c_int>> {
        if p.is_null() {
            return None;
        }
        let mut v = Vec::new();
        let mut i = 0isize;
        while *p.offset(i) != 0 {
            v.push(*p.offset(i));
            i += 1;
            assert!(i < 64, "unterminated *_full array");
        }
        Some(v)
    }
    for (name, which) in [("tolowerrune_full", 0usize), ("toupperrune_full", 1)] {
        let (cr, rr) = both(|api, _| {
            let f = if which == 0 {
                api.jsU_tolowerrune_full
            } else {
                api.jsU_toupperrune_full
            };
            let mut v = Vec::with_capacity(0x110200);
            for c in (-256i64..0x110200i64).map(|x| x as c_int) {
                v.push(unsafe { read_full(f(c)) });
            }
            v
        });
        let mut base = -256i64;
        for (c, r) in cr.iter().zip(rr.iter()) {
            assert_eq!(
                c, r,
                "{} DIVERGENCE rune={:#x}: C={:?} Rust={:?}",
                name, base, c, r
            );
            base += 1;
        }
    }
}

// ---------------------------------------------------------------- js_utflen / js_utfptrtoidx / js_runeat
#[test]
fn utflen_and_ptrtoidx_and_runeat() {
    let corpus: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"abc".to_vec(),
        "\u{e9}".as_bytes().to_vec(),
        "h\u{e9}llo w\u{f6}rld".as_bytes().to_vec(),
        "\u{10FFFF}".as_bytes().to_vec(),
        "\u{1F600}\u{1F601}".as_bytes().to_vec(),
        vec![0xFF, 0xFE, 0x41],
        vec![0x80, 0x80, 0x41],
        vec![0xC3],
        vec![0xE2, 0x82],
    ];
    let (cr, rr) = both(|api, _| {
        let mut out = Vec::new();
        for s in &corpus {
            let mut buf = s.clone();
            buf.push(0);
            let base = buf.as_ptr() as *const c_char;
            let len = unsafe { (api.js_utflen)(base) };
            let mut idxs = Vec::new();
            for off in 0..=buf.len() {
                idxs.push(unsafe { (api.js_utfptrtoidx)(base, base.add(off)) });
            }
            // js_runeat needs a state (only for the error path, which cannot
            // trigger here) — pass a real state.
            let J = unsafe { (api.js_newstate)(None, std::ptr::null_mut(), 0) };
            let mut runes = Vec::new();
            for i in -2..(len + 3) {
                runes.push(unsafe { (api.js_runeat)(J, base, i) });
            }
            unsafe { (api.js_freestate)(J) };
            out.push((len, idxs, runes));
        }
        out
    });
    for (i, (c, r)) in cr.iter().zip(rr.iter()).enumerate() {
        assert_eq!(
            c, r,
            "utflen/ptrtoidx/runeat DIVERGENCE for {:02x?}: C={:?} Rust={:?}",
            corpus[i], c, r
        );
    }
}
