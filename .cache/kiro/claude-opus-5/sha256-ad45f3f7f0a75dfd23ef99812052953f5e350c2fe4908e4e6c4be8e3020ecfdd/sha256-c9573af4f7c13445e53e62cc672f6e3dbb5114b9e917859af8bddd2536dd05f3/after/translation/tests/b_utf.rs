//! Phase B rows 1-11: UTF-8 / rune primitives.
mod common;
use common::*;
use std::os::raw::{c_char, c_int};

fn chartorune_case(bytes: &[u8]) {
    let p = pair();
    let buf = cbuf(bytes);
    let mut ra: Rune = -12345;
    let mut rb: Rune = -12345;
    let na = unsafe { (p.c.jsU_chartorune)(&mut ra, buf.as_ptr()) };
    let nb = unsafe { (p.r.jsU_chartorune)(&mut rb, buf.as_ptr()) };
    assert_eq!((na, ra), (nb, rb), "chartorune on {bytes:02x?}");
}

#[test]
fn row01_chartorune_ascii() {
    for b in 0u8..=0x7f {
        chartorune_case(&[b, b'x', b'y', b'z']);
    }
    // lone NUL terminator
    chartorune_case(&[]);
}

#[test]
fn row02_chartorune_two_byte() {
    for b0 in 0xc0u8..=0xdf {
        for b1 in 0u16..=0xff {
            chartorune_case(&[b0, b1 as u8, b'z']);
        }
        chartorune_case(&[b0]); // truncated
    }
}

#[test]
fn row03_chartorune_three_byte() {
    let mut rng = Rng::new(0x3333);
    for b0 in 0xe0u8..=0xef {
        for _ in 0..2000 {
            let b1 = rng.next_u32() as u8;
            let b2 = rng.next_u32() as u8;
            chartorune_case(&[b0, b1, b2, b'z']);
        }
        // exhaustive over the interesting continuation edges
        for b1 in [0x00u8, 0x7f, 0x80, 0x9f, 0xa0, 0xbf, 0xc0, 0xff] {
            for b2 in [0x00u8, 0x7f, 0x80, 0xbf, 0xc0, 0xff] {
                chartorune_case(&[b0, b1, b2, b'z']);
                chartorune_case(&[b0, b1]); // truncated
            }
        }
        chartorune_case(&[b0]);
    }
    // surrogates and overlongs explicitly
    for cp in [0xd800u32, 0xdbff, 0xdc00, 0xdfff, 0x0800, 0x07ff, 0xfffd, 0xffff] {
        let s = char::from_u32(cp)
            .map(|c| c.to_string().into_bytes())
            .unwrap_or_else(|| {
                vec![
                    (0xe0 | (cp >> 12)) as u8,
                    (0x80 | ((cp >> 6) & 0x3f)) as u8,
                    (0x80 | (cp & 0x3f)) as u8,
                ]
            });
        chartorune_case(&s);
    }
}

#[test]
fn row04_chartorune_four_byte() {
    let mut rng = Rng::new(0x4444);
    for b0 in 0xf0u8..=0xff {
        for _ in 0..3000 {
            let b1 = rng.next_u32() as u8;
            let b2 = rng.next_u32() as u8;
            let b3 = rng.next_u32() as u8;
            chartorune_case(&[b0, b1, b2, b3, b'z']);
        }
        for b1 in [0x00u8, 0x80, 0x8f, 0x90, 0xbf, 0xc0, 0xff] {
            for b2 in [0x00u8, 0x80, 0xbf, 0xff] {
                for b3 in [0x00u8, 0x80, 0xbf, 0xff] {
                    chartorune_case(&[b0, b1, b2, b3, b'z']);
                }
                chartorune_case(&[b0, b1, b2]);
            }
            chartorune_case(&[b0, b1]);
        }
        chartorune_case(&[b0]);
    }
    // exact boundaries around Runemax
    for cp in [0x10000u32, 0x10ffff, 0x110000, 0x1fffff, 0x200000] {
        chartorune_case(&[
            (0xf0 | (cp >> 18)) as u8,
            (0x80 | ((cp >> 12) & 0x3f)) as u8,
            (0x80 | ((cp >> 6) & 0x3f)) as u8,
            (0x80 | (cp & 0x3f)) as u8,
        ]);
    }
}

#[test]
fn row05_chartorune_random_soup() {
    let mut rng = Rng::new(0x5555);
    for _ in 0..40000 {
        let n = 1 + rng.below(6) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
        chartorune_case(&bytes);
    }
    // continuation bytes / 0x80..0xbf leading
    for b in 0x80u8..=0xbf {
        chartorune_case(&[b, b'z']);
    }
}

#[test]
fn row06_runetochar() {
    let p = pair();
    let mut cases: Vec<Rune> = Vec::new();
    for r in 0..0x800 {
        cases.push(r);
    }
    for r in [
        0x800, 0xfff, 0x1000, 0xd7ff, 0xd800, 0xdfff, 0xe000, 0xfffd, 0xffff, 0x10000, 0x10ffff,
        0x110000, 0x1fffff, 0x7fffffff, -1, -2, -0x80, i32::MIN, i32::MIN + 1,
    ] {
        cases.push(r);
    }
    let mut rng = Rng::new(0x6666);
    for _ in 0..20000 {
        cases.push(rng.i32());
    }
    for r in cases {
        let mut a: [c_char; 16] = [0x7f; 16];
        let mut b: [c_char; 16] = [0x7f; 16];
        let na = unsafe { (p.c.jsU_runetochar)(a.as_mut_ptr(), &r) };
        let nb = unsafe { (p.r.jsU_runetochar)(b.as_mut_ptr(), &r) };
        assert_eq!(na, nb, "runetochar len for rune {r:#x}");
        assert_eq!(
            &a[..na.max(0) as usize],
            &b[..nb.max(0) as usize],
            "runetochar bytes for rune {r:#x}"
        );
    }
}

#[test]
fn row07_runelen() {
    let p = pair();
    let mut rng = Rng::new(0x7777);
    let mut cases: Vec<c_int> = (0..0x2000).collect();
    for extra in [
        0x7ff, 0x800, 0xffff, 0x10000, 0x10ffff, 0x110000, i32::MAX, i32::MIN, -1,
    ] {
        cases.push(extra);
    }
    for _ in 0..20000 {
        cases.push(rng.i32());
    }
    for c in cases {
        let a = unsafe { (p.c.jsU_runelen)(c) };
        let b = unsafe { (p.r.jsU_runelen)(c) };
        assert_eq!(a, b, "runelen({c:#x})");
    }
}

#[test]
fn row08_rune_class_predicates() {
    let p = pair();
    for r in -0x200i32..0x11_2000 {
        let a = unsafe {
            (
                (p.c.jsU_isalpharune)(r),
                (p.c.jsU_islowerrune)(r),
                (p.c.jsU_isupperrune)(r),
            )
        };
        let b = unsafe {
            (
                (p.r.jsU_isalpharune)(r),
                (p.r.jsU_islowerrune)(r),
                (p.r.jsU_isupperrune)(r),
            )
        };
        assert_eq!(a, b, "rune class predicates for {r:#x}");
    }
    for r in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        let a = unsafe {
            (
                (p.c.jsU_isalpharune)(r),
                (p.c.jsU_islowerrune)(r),
                (p.c.jsU_isupperrune)(r),
            )
        };
        let b = unsafe {
            (
                (p.r.jsU_isalpharune)(r),
                (p.r.jsU_islowerrune)(r),
                (p.r.jsU_isupperrune)(r),
            )
        };
        assert_eq!(a, b, "rune class predicates for {r:#x}");
    }
}

#[test]
fn row09_rune_case_simple() {
    let p = pair();
    for r in -0x200i32..0x11_2000 {
        let a = unsafe { ((p.c.jsU_tolowerrune)(r), (p.c.jsU_toupperrune)(r)) };
        let b = unsafe { ((p.r.jsU_tolowerrune)(r), (p.r.jsU_toupperrune)(r)) };
        assert_eq!(a, b, "case mapping for {r:#x}");
    }
    for r in [i32::MIN, i32::MAX, -1, 0] {
        let a = unsafe { ((p.c.jsU_tolowerrune)(r), (p.c.jsU_toupperrune)(r)) };
        let b = unsafe { ((p.r.jsU_tolowerrune)(r), (p.r.jsU_toupperrune)(r)) };
        assert_eq!(a, b, "case mapping for {r:#x}");
    }
}

unsafe fn read_rune_seq(p: *const Rune) -> Option<Vec<Rune>> {
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    let mut i = 0;
    loop {
        let r = unsafe { *p.add(i) };
        if r == 0 || i > 8 {
            break;
        }
        v.push(r);
        i += 1;
    }
    Some(v)
}

#[test]
fn row10_rune_case_full() {
    let p = pair();
    for r in -0x100i32..0x11_1000 {
        let (a1, a2) = unsafe {
            (
                read_rune_seq((p.c.jsU_tolowerrune_full)(r)),
                read_rune_seq((p.c.jsU_toupperrune_full)(r)),
            )
        };
        let (b1, b2) = unsafe {
            (
                read_rune_seq((p.r.jsU_tolowerrune_full)(r)),
                read_rune_seq((p.r.jsU_toupperrune_full)(r)),
            )
        };
        assert_eq!((a1, a2), (b1, b2), "full case mapping for {r:#x}");
    }
}

#[test]
fn row11_utflen_and_ptrtoidx() {
    let p = pair();
    let mut rng = Rng::new(0xbbbb);
    let mut samples: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"abc".to_vec(),
        "héllo wörld".as_bytes().to_vec(),
        "\u{1f600}\u{1f601}".as_bytes().to_vec(),
        vec![0xff, 0xfe, 0x80, 0xc0],
        vec![0xc3],
        vec![0xe2, 0x82],
    ];
    for _ in 0..3000 {
        let n = rng.below(24) as usize;
        samples.push((0..n).map(|_| 1 + (rng.next_u32() as u8 % 255)).collect());
    }
    for s in samples {
        let buf = cbuf(&s);
        let la = unsafe { (p.c.js_utflen)(buf.as_ptr()) };
        let lb = unsafe { (p.r.js_utflen)(buf.as_ptr()) };
        assert_eq!(la, lb, "utflen {s:02x?}");
        for off in 0..=buf.len() - 1 {
            let ptr = unsafe { buf.as_ptr().add(off) };
            let ia = unsafe { (p.c.js_utfptrtoidx)(buf.as_ptr(), ptr) };
            let ib = unsafe { (p.r.js_utfptrtoidx)(buf.as_ptr(), ptr) };
            assert_eq!(ia, ib, "utfptrtoidx off={off} {s:02x?}");
        }
    }
}
