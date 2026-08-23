//! Phase B/C: differential tests for the utf.c entry points.
//! CONFIGS.md rows 356-376; the utf.c ERRORS.md rows 911-933.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

fn boundary_runes() -> Vec<i32> {
    let mut v = vec![
        0, 1, 0x1f, 0x20, 0x7e, 0x7f, 0x80, 0x81, 0x7fe, 0x7ff, 0x800, 0x801, 0xd7ff, 0xd800,
        0xdbff, 0xdc00, 0xdfff, 0xe000, 0xfffd, 0xfffe, 0xffff, 0x10000, 0x10001, 0x10fffe,
        0x10ffff, 0x110000, 0x110001, 0x1fffff, 0x200000, 0x7fffffff, -1, -2, -0x80000000,
        0x41, 0x61, 0xc0, 0xe0, 0x131, 0x130, 0x1e9e, 0xdf, 0x3a3, 0x3c2, 0x3c3, 0xfb00,
        0x1f88, 0x2126, 0x212a, 0x212b, 0x10400, 0x10428,
    ];
    // sweep every rune in the interesting Unicode ranges
    for r in 0..0x300 {
        v.push(r);
    }
    for r in 0x1000..0x1200 {
        v.push(r);
    }
    for r in 0x2100..0x2200 {
        v.push(r);
    }
    for r in 0xff00..0xff80 {
        v.push(r);
    }
    for r in 0x10400..0x10500 {
        v.push(r);
    }
    for r in 0x1e900..0x1e980 {
        v.push(r);
    }
    v
}

#[test]
fn t_runelen() {
    let p = libs();
    unsafe {
        for c in boundary_runes() {
            let a = p.c.jsU_runelen(c);
            let b = p.rs.jsU_runelen(c);
            assert_eq!(a, b, "runelen({c:#x})");
        }
        let mut rng = Rng::new(11);
        for _ in 0..20000 {
            let c = rng.next_u32() as i32;
            assert_eq!(p.c.jsU_runelen(c), p.rs.jsU_runelen(c), "runelen({c:#x})");
        }
    }
}

#[test]
fn t_runetochar() {
    let p = libs();
    unsafe {
        let check = |c: i32| {
            let mut ba = [0i8; 32];
            let mut bb = [0i8; 32];
            let r = c as Rune;
            let na = p.c.jsU_runetochar(ba.as_mut_ptr(), &r);
            let nb = p.rs.jsU_runetochar(bb.as_mut_ptr(), &r);
            assert_eq!(na, nb, "runetochar({c:#x}) return");
            assert_eq!(ba, bb, "runetochar({c:#x}) bytes");
        };
        for c in boundary_runes() {
            check(c);
        }
        let mut rng = Rng::new(12);
        for _ in 0..20000 {
            check((rng.next_u32() & 0x1f_ffff) as i32);
        }
        for _ in 0..5000 {
            check(rng.next_u32() as i32);
        }
    }
}

#[test]
fn t_chartorune() {
    let p = libs();
    unsafe {
        let check = |bytes: &[u8], label: &str| {
            // NUL-terminate; chartorune reads until it has enough bytes
            let mut buf: Vec<u8> = bytes.to_vec();
            buf.push(0);
            buf.push(0);
            buf.push(0);
            buf.push(0);
            let mut ra: Rune = -12345;
            let mut rb: Rune = -12345;
            let na = p
                .c
                .jsU_chartorune(&mut ra, buf.as_ptr() as *const c_char);
            let nb = p
                .rs
                .jsU_chartorune(&mut rb, buf.as_ptr() as *const c_char);
            assert_eq!((na, ra), (nb, rb), "chartorune({label}) {bytes:02x?}");
        };
        // every single byte
        for b in 0u16..=255 {
            check(&[b as u8], "single");
        }
        // every 2-byte pair with a lead byte
        for lead in 0xc0u16..=0xffu16 {
            for second in [0u8, 0x1, 0x3f, 0x40, 0x7f, 0x80, 0x81, 0xbf, 0xc0, 0xff] {
                check(&[lead as u8, second], "pair");
            }
        }
        // canonical encodings of every boundary rune
        for c in boundary_runes() {
            let mut tmp = [0i8; 8];
            let r = c as Rune;
            let n = p.c.jsU_runetochar(tmp.as_mut_ptr(), &r);
            let bytes: Vec<u8> = (0..n as usize).map(|i| tmp[i] as u8).collect();
            check(&bytes, "canonical");
            // and truncations of it
            for k in 1..bytes.len() {
                check(&bytes[..k], "truncated");
            }
        }
        // overlong / random sequences
        let mut rng = Rng::new(13);
        for _ in 0..30000 {
            let n = 1 + rng.below(5) as usize;
            let bytes: Vec<u8> = (0..n).map(|_| (rng.next_u32() & 0xff) as u8).collect();
            check(&bytes, "random");
        }
        // sequences biased to be structurally plausible utf-8
        for _ in 0..30000 {
            let lead = match rng.below(4) {
                0 => 0xc0 | (rng.below(0x20) as u8),
                1 => 0xe0 | (rng.below(0x10) as u8),
                2 => 0xf0 | (rng.below(0x08) as u8),
                _ => rng.below(0x80) as u8,
            };
            let mut bytes = vec![lead];
            for _ in 0..3 {
                bytes.push(0x80 | (rng.below(0x40) as u8));
            }
            check(&bytes, "plausible");
        }
    }
}

#[test]
fn t_rune_predicates_and_case() {
    let p = libs();
    unsafe {
        for name in ["jsU_isalpharune", "jsU_islowerrune", "jsU_isupperrune"] {
            for c in boundary_runes() {
                assert_eq!(
                    p.c.rune_pred(name, c),
                    p.rs.rune_pred(name, c),
                    "{name}({c:#x})"
                );
            }
        }
        for name in ["jsU_tolowerrune", "jsU_toupperrune"] {
            for c in boundary_runes() {
                assert_eq!(
                    p.c.rune_pred(name, c),
                    p.rs.rune_pred(name, c),
                    "{name}({c:#x})"
                );
            }
        }
        // exhaustive sweep of the whole BMP + a chunk of astral for the tables
        for c in 0..0x11000 {
            for name in [
                "jsU_isalpharune",
                "jsU_islowerrune",
                "jsU_isupperrune",
                "jsU_tolowerrune",
                "jsU_toupperrune",
            ] {
                let a = p.c.rune_pred(name, c);
                let b = p.rs.rune_pred(name, c);
                assert_eq!(a, b, "{name}({c:#x})");
            }
        }
        let mut rng = Rng::new(14);
        for _ in 0..5000 {
            let c = rng.next_u32() as i32;
            for name in [
                "jsU_isalpharune",
                "jsU_islowerrune",
                "jsU_isupperrune",
                "jsU_tolowerrune",
                "jsU_toupperrune",
            ] {
                assert_eq!(
                    p.c.rune_pred(name, c),
                    p.rs.rune_pred(name, c),
                    "{name}({c:#x})"
                );
            }
        }
    }
}

#[test]
fn t_rune_full_case() {
    let p = libs();
    unsafe {
        let read = |lib: &Lib, name: &'static str, c: Rune| -> Option<Vec<Rune>> {
            let ptr = lib.rune_full(name, c);
            if ptr.is_null() {
                None
            } else {
                // the tables are NUL/0-terminated sequences of runes
                let mut v = vec![];
                let mut i = 0isize;
                loop {
                    let r = *ptr.offset(i);
                    if r == 0 || i > 8 {
                        break;
                    }
                    v.push(r);
                    i += 1;
                }
                Some(v)
            }
        };
        for name in ["jsU_tolowerrune_full", "jsU_toupperrune_full"] {
            for c in 0..0x11000 {
                let a = read(&p.c, name, c);
                let b = read(&p.rs, name, c);
                assert_eq!(a, b, "{name}({c:#x})");
            }
            for c in boundary_runes() {
                assert_eq!(
                    read(&p.c, name, c),
                    read(&p.rs, name, c),
                    "{name}({c:#x})"
                );
            }
        }
    }
}

#[test]
fn t_utflen_and_ptrtoidx() {
    let p = libs();
    unsafe {
        let mut rng = Rng::new(15);
        let mut cases: Vec<Vec<u8>> = vec![
            vec![],
            b"a".to_vec(),
            b"abc".to_vec(),
            "\u{80}".as_bytes().to_vec(),
            "\u{7ff}\u{800}\u{ffff}\u{10000}\u{10ffff}".as_bytes().to_vec(),
            vec![0x80],             // stray continuation
            vec![0xc0],             // truncated
            vec![0xf4, 0x8f],       // truncated
            vec![0xff, 0xfe, 0xfd], // invalid
        ];
        for _ in 0..2000 {
            cases.push(rng.unicode_string(12).into_bytes());
        }
        for _ in 0..2000 {
            cases.push(rng.raw_bytes(12));
        }
        for bytes in &cases {
            let mut buf = bytes.clone();
            buf.push(0);
            let sp = buf.as_ptr() as *const c_char;
            let a = p.c.js_utflen(sp);
            let b = p.rs.js_utflen(sp);
            assert_eq!(a, b, "js_utflen({bytes:02x?})");
            for off in 0..=bytes.len() {
                let pp = sp.add(off);
                let a = p.c.js_utfptrtoidx(sp, pp);
                let b = p.rs.js_utfptrtoidx(sp, pp);
                assert_eq!(a, b, "js_utfptrtoidx({bytes:02x?}, +{off})");
            }
        }
    }
}

#[test]
fn t_runeat() {
    let p = libs();
    unsafe {
        let jc = new_state(&p.c, 0);
        set_cur(&p.rs);
        let jr = new_state(&p.rs, 0);
        let mut rng = Rng::new(16);
        let mut cases: Vec<Vec<u8>> = vec![
            vec![],
            b"a".to_vec(),
            b"hello".to_vec(),
            "\u{80}\u{7ff}\u{800}\u{10000}".as_bytes().to_vec(),
            vec![0x80, 0x80],
            vec![0xc2],
        ];
        for _ in 0..800 {
            cases.push(rng.unicode_string(8).into_bytes());
        }
        for _ in 0..800 {
            cases.push(rng.raw_bytes(8));
        }
        for bytes in &cases {
            let mut buf = bytes.clone();
            buf.push(0);
            let sp = buf.as_ptr() as *const c_char;
            for i in -2..(bytes.len() as c_int + 3) {
                set_cur(&p.c);
                let a = p.c.js_runeat(jc, sp, i);
                set_cur(&p.rs);
                let b = p.rs.js_runeat(jr, sp, i);
                assert_eq!(a, b, "js_runeat({bytes:02x?}, {i})");
            }
        }
        set_cur(&p.c);
        p.c.js_freestate(jc);
        set_cur(&p.rs);
        p.rs.js_freestate(jr);
    }
}
