//! Phase B/C — differential tests for the UTF routines (utf.c).
//! CONFIGS rows 1-5, ERRORS rows 18-19.
mod common;
use common::{Libs, Rng};
use std::os::raw::{c_char, c_int};

type ChartoruneFn = unsafe extern "C" fn(*mut c_int, *const c_char) -> c_int;
type RunetocharFn = unsafe extern "C" fn(*mut c_char, *const c_int) -> c_int;
type RunelenFn = unsafe extern "C" fn(c_int) -> c_int;
type RunePredFn = unsafe extern "C" fn(c_int) -> c_int;
type RuneMapFn = unsafe extern "C" fn(c_int) -> c_int;

#[test]
fn runelen_boundaries() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<RunelenFn> = libs.c_sym(b"jsU_runelen");
        let r: libloading::Symbol<RunelenFn> = libs.rust_sym(b"jsU_runelen");
        let boundaries: &[c_int] = &[
            0, 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF, 0x110000, 0x1FFFFF,
            -1, i32::MAX, i32::MIN,
        ];
        for &b in boundaries {
            assert_eq!(c(b), r(b), "runelen({})", b);
        }
        // random
        let mut rng = Rng::new(1);
        for _ in 0..100_000 {
            let v = rng.next_u32() as c_int;
            assert_eq!(c(v), r(v), "runelen({})", v);
        }
    }
}

#[test]
fn runetochar_roundtrip() {
    let libs = Libs::load();
    unsafe {
        let cenc: libloading::Symbol<RunetocharFn> = libs.c_sym(b"jsU_runetochar");
        let renc: libloading::Symbol<RunetocharFn> = libs.rust_sym(b"jsU_runetochar");
        let cdec: libloading::Symbol<ChartoruneFn> = libs.c_sym(b"jsU_chartorune");
        let rdec: libloading::Symbol<ChartoruneFn> = libs.rust_sym(b"jsU_chartorune");

        let mut rng = Rng::new(2);
        let mut check = |rune: c_int| {
            let mut cb = [0i8; 8];
            let mut rb = [0i8; 8];
            let cn = cenc(cb.as_mut_ptr(), &rune);
            let rn = renc(rb.as_mut_ptr(), &rune);
            assert_eq!(cn, rn, "runetochar len rune={}", rune);
            assert_eq!(&cb[..cn as usize], &rb[..rn as usize], "runetochar bytes rune={}", rune);
            // now decode back from the C-encoded bytes with a NUL terminator
            let mut buf = [0i8; 9];
            buf[..cn as usize].copy_from_slice(&cb[..cn as usize]);
            let mut cr = 0;
            let mut rr = 0;
            let cdl = cdec(&mut cr, buf.as_ptr());
            let rdl = rdec(&mut rr, buf.as_ptr());
            assert_eq!(cdl, rdl, "chartorune len rune={}", rune);
            assert_eq!(cr, rr, "chartorune value rune={}", rune);
        };
        // boundaries
        for r in [0, 0x7F, 0x80, 0x7FF, 0x800, 0xFFFF, 0x10000, 0x10FFFF, 0x110000, 0x1FFFFF, 0x200000] {
            check(r);
        }
        for _ in 0..200_000 {
            let rune = (rng.next_u32() % 0x120000) as c_int;
            check(rune);
        }
    }
}

#[test]
fn chartorune_random_bytes() {
    let libs = Libs::load();
    unsafe {
        let cdec: libloading::Symbol<ChartoruneFn> = libs.c_sym(b"jsU_chartorune");
        let rdec: libloading::Symbol<ChartoruneFn> = libs.rust_sym(b"jsU_chartorune");
        let mut rng = Rng::new(3);
        for _ in 0..200_000 {
            // random 4-byte buffer + guaranteed NUL terminator
            let mut buf = [0i8; 8];
            for i in 0..4 {
                buf[i] = (rng.next_u32() & 0xFF) as i8;
            }
            let mut cr = 0;
            let mut rr = 0;
            let cn = cdec(&mut cr, buf.as_ptr());
            let rn = rdec(&mut rr, buf.as_ptr());
            assert_eq!(cn, rn, "chartorune len buf={:?}", &buf[..4]);
            assert_eq!(cr, rr, "chartorune rune buf={:?}", &buf[..4]);
        }
    }
}

#[test]
fn rune_predicates_and_maps() {
    let libs = Libs::load();
    unsafe {
        for name in [&b"jsU_isalpharune"[..], b"jsU_islowerrune", b"jsU_isupperrune"] {
            let c: libloading::Symbol<RunePredFn> = libs.c_sym(name);
            let r: libloading::Symbol<RunePredFn> = libs.rust_sym(name);
            let mut rng = Rng::new(4 + name.len() as u64);
            for rune in 0..0x11000 {
                assert_eq!(c(rune), r(rune), "{} ({})", String::from_utf8_lossy(name), rune);
            }
            for _ in 0..100_000 {
                let rune = (rng.next_u32() % 0x120000) as c_int;
                assert_eq!(c(rune), r(rune), "{} ({})", String::from_utf8_lossy(name), rune);
            }
        }
        for name in [&b"jsU_tolowerrune"[..], b"jsU_toupperrune"] {
            let c: libloading::Symbol<RuneMapFn> = libs.c_sym(name);
            let r: libloading::Symbol<RuneMapFn> = libs.rust_sym(name);
            let mut rng = Rng::new(400 + name.len() as u64);
            for rune in 0..0x11000 {
                assert_eq!(c(rune), r(rune), "{} ({})", String::from_utf8_lossy(name), rune);
            }
            for _ in 0..100_000 {
                let rune = (rng.next_u32() % 0x120000) as c_int;
                assert_eq!(c(rune), r(rune), "{} ({})", String::from_utf8_lossy(name), rune);
            }
        }
    }
}
