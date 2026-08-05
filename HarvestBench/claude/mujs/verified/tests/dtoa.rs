//! Phase B/C — differential tests for jsdtoa.c / jsvalue.c number helpers.
//! CONFIGS rows 6-8, ERRORS rows 15-17.
mod common;
use common::{Libs, Rng};
use std::os::raw::{c_char, c_int, c_double};

type StrtodFn = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_double;
type Grisu2Fn = unsafe extern "C" fn(c_double, *mut c_char, *mut c_int) -> c_int;
type FmtexpFn = unsafe extern "C" fn(*mut c_char, c_int);
type ItoaFn = unsafe extern "C" fn(*mut c_char, c_int) -> *const c_char;

fn cstr(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

#[test]
fn itoa_full_int_range() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<ItoaFn> = libs.c_sym(b"js_itoa");
        let r: libloading::Symbol<ItoaFn> = libs.rust_sym(b"js_itoa");
        let mut check = |v: c_int| {
            let mut cb = [0i8; 32];
            let mut rb = [0i8; 32];
            c(cb.as_mut_ptr(), v);
            r(rb.as_mut_ptr(), v);
            assert_eq!(cb, rb, "itoa({})", v);
        };
        for v in [0, 1, -1, i32::MIN, i32::MAX, 10, -10, 100000, -999999] {
            check(v);
        }
        let mut rng = Rng::new(10);
        for _ in 0..200_000 {
            check(rng.next_u32() as c_int);
        }
    }
}

#[test]
fn strtod_random_and_errors() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<StrtodFn> = libs.c_sym(b"js_strtod");
        let r: libloading::Symbol<StrtodFn> = libs.rust_sym(b"js_strtod");

        let mut check = |s: &str| {
            let cb = cstr(s);
            let rb = cstr(s);
            let mut cend: *mut c_char = std::ptr::null_mut();
            let mut rend: *mut c_char = std::ptr::null_mut();
            let cv = c(cb.as_ptr(), &mut cend);
            let rv = r(rb.as_ptr(), &mut rend);
            // consumed length must match
            let clen = cend as usize - cb.as_ptr() as usize;
            let rlen = rend as usize - rb.as_ptr() as usize;
            assert_eq!(clen, rlen, "strtod consumed len {:?}", s);
            assert!(
                (cv.is_nan() && rv.is_nan()) || cv.to_bits() == rv.to_bits(),
                "strtod({:?}) c={} r={}", s, cv, rv
            );
        };
        // fixed cases including error/overflow/underflow
        for s in ["abc", "", "1e400", "1e-400", "  12.5", "0x1p4", "inf", "-inf",
                  "1.7976931348623157e308", "5e-324", "nan", "+.5", "-0", "123abc",
                  "1e", "e5", ".", "-", "+", "0.0000001", "1234567890.12345678e-30"] {
            check(s);
        }
        // random numeric-ish strings
        let mut rng = Rng::new(11);
        for _ in 0..50_000 {
            let mut s = String::new();
            let n = 1 + rng.below(12);
            for _ in 0..n {
                let cls = rng.below(10);
                let ch = match cls {
                    0 => b'0' + (rng.below(10) as u8),
                    1 => b'.',
                    2 => b'e',
                    3 => b'E',
                    4 => b'+',
                    5 => b'-',
                    6 => b' ',
                    _ => b'0' + (rng.below(10) as u8),
                };
                s.push(ch as char);
            }
            check(&s);
        }
    }
}

#[test]
fn grisu2_random_finite() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<Grisu2Fn> = libs.c_sym(b"js_grisu2");
        let r: libloading::Symbol<Grisu2Fn> = libs.rust_sym(b"js_grisu2");
        let mut check = |v: c_double| {
            // grisu2 requires v != 0 and finite; skip otherwise
            if v == 0.0 || !v.is_finite() { return; }
            let mut cb = [0i8; 32];
            let mut rb = [0i8; 32];
            let mut ck = 0;
            let mut rk = 0;
            let cn = c(v, cb.as_mut_ptr(), &mut ck);
            let rn = r(v, rb.as_mut_ptr(), &mut rk);
            assert_eq!(cn, rn, "grisu2 len v={}", v);
            assert_eq!(ck, rk, "grisu2 K v={}", v);
            assert_eq!(&cb[..cn as usize], &rb[..rn as usize], "grisu2 digits v={}", v);
        };
        for v in [1.5, 0.1, 3.14159, 1e300, 1e-300, 123456.789, 2.5, 9.999999] {
            check(v);
        }
        let mut rng = Rng::new(12);
        for _ in 0..200_000 {
            let bits = rng.next_u64();
            let v = f64::from_bits(bits);
            check(v);
            // also feed "nice" random magnitudes
            let m = (rng.f64() - 0.5) * 2.0;
            let e = (rng.below(600) as i32) - 300;
            check(m * 10f64.powi(e));
        }
    }
}

#[test]
fn fmtexp_random() {
    let libs = Libs::load();
    unsafe {
        let c: libloading::Symbol<FmtexpFn> = libs.c_sym(b"js_fmtexp");
        let r: libloading::Symbol<FmtexpFn> = libs.rust_sym(b"js_fmtexp");
        let mut check = |e: c_int| {
            let mut cb = [0i8; 16];
            let mut rb = [0i8; 16];
            c(cb.as_mut_ptr(), e);
            r(rb.as_mut_ptr(), e);
            assert_eq!(cb, rb, "fmtexp({})", e);
        };
        for e in -400..=400 {
            check(e);
        }
        let mut rng = Rng::new(13);
        for _ in 0..50_000 {
            check((rng.next_u32() % 800) as c_int - 400);
        }
    }
}
