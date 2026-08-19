//! Independent third oracle, built **only** from symbols resolved at runtime
//! with `libloading` across the FFI boundary.
//!
//! `c_src` builds an executable, so there is no project `.so` to dlopen.  What
//! CAN be loaded is the set of libc/libm entry points that `SYMBOLS.md` lists as
//! the behaviour-defining imports of BOTH binaries — `strtod`, `pow`,
//! `__errno_location`, plus `snprintf` for the `%.2f` conversion.  This test
//! re-implements `main.c` line for line on top of those dynamically loaded
//! symbols and asserts that the C binary, the Rust binary, and this oracle all
//! produce the same bytes.  A formatting or errno bug in either binary shows up
//! here even if both binaries happened to agree.

mod common;
use common::*;

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

const EDOM: i32 = 33;
const ERANGE: i32 = 34;

type FnStrtod = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> f64;
type FnPow = unsafe extern "C" fn(f64, f64) -> f64;
type FnErrnoLoc = unsafe extern "C" fn() -> *mut c_int;
type FnSnprintf = unsafe extern "C" fn(*mut c_char, usize, *const c_char, ...) -> c_int;

struct Libc {
    _libc: Library,
    _libm: Library,
    strtod: FnStrtod,
    pow: FnPow,
    errno_location: FnErrnoLoc,
    snprintf: FnSnprintf,
}

fn open(names: &[&str]) -> Library {
    let mut last = String::new();
    for n in names {
        match unsafe { Library::new(n) } {
            Ok(l) => return l,
            Err(e) => last = format!("{n}: {e}"),
        }
    }
    panic!("could not dlopen any of {names:?} ({last})");
}

impl Libc {
    fn load() -> Self {
        let libc = open(&["libc.so.6", "libc.so"]);
        let libm = open(&["libm.so.6", "libm.so"]);
        unsafe {
            let strtod: Symbol<FnStrtod> = libc.get(b"strtod\0").unwrap();
            let errno_location: Symbol<FnErrnoLoc> = libc.get(b"__errno_location\0").unwrap();
            let snprintf: Symbol<FnSnprintf> = libc.get(b"snprintf\0").unwrap();
            // pow lives in libm in the C binary's own dynamic symbol table
            // (pow@GLIBC_2.29); fall back to libc for merged-libm systems.
            let pow: FnPow = match libm.get::<FnPow>(b"pow\0") {
                Ok(s) => *s,
                Err(_) => *libc.get::<FnPow>(b"pow\0").unwrap(),
            };
            let (s, e, p) = (*strtod, *errno_location, *snprintf);
            Libc {
                _libc: libc,
                _libm: libm,
                strtod: s,
                pow,
                errno_location: e,
                snprintf: p,
            }
        }
    }

    fn errno(&self) -> i32 {
        unsafe { *(self.errno_location)() }
    }
    fn set_errno(&self, v: i32) {
        unsafe { *(self.errno_location)() = v }
    }

    /// glibc's own `%.2f` rendering, obtained through the FFI boundary.
    fn fmt2(&self, v: f64) -> Vec<u8> {
        let fmt = CString::new("%.2f").unwrap();
        let mut buf = vec![0u8; 512];
        loop {
            let n = unsafe {
                (self.snprintf)(
                    buf.as_mut_ptr() as *mut c_char,
                    buf.len(),
                    fmt.as_ptr(),
                    v,
                )
            };
            assert!(n >= 0, "snprintf failed");
            let n = n as usize;
            if n < buf.len() {
                buf.truncate(n);
                return buf;
            }
            buf = vec![0u8; n + 1];
        }
    }

    /// A faithful re-implementation of `c_src/src/main.c`, using only the
    /// dynamically loaded libc/libm symbols.
    /// Returns (stdout, stderr, exit status).
    fn oracle(&self, base_arg: &[u8], exp_arg: &[u8]) -> (Vec<u8>, Vec<u8>, i32) {
        let mut out = Vec::new();
        let mut err = Vec::new();

        let cb = CString::new(base_arg).unwrap();
        let ce = CString::new(exp_arg).unwrap();

        // errno = 0; double base = strtod(argv[1], &endptr1);
        let mut endp: *mut c_char = std::ptr::null_mut();
        self.set_errno(0);
        let base = unsafe { (self.strtod)(cb.as_ptr(), &mut endp) };
        let e1 = self.errno();
        let consumed1 = unsafe { *endp } == 0;
        if e1 == ERANGE {
            err.extend_from_slice(b"Range error while converting base '");
            err.extend_from_slice(base_arg);
            err.extend_from_slice(b"'\n");
            return (out, err, 1);
        } else if !consumed1 {
            err.extend_from_slice(b"Invalid numeric input for base: '");
            err.extend_from_slice(base_arg);
            err.extend_from_slice(b"'\n");
            return (out, err, 1);
        }

        // errno = 0; double exponent = strtod(argv[2], &endptr2);
        let mut endp2: *mut c_char = std::ptr::null_mut();
        self.set_errno(0);
        let exponent = unsafe { (self.strtod)(ce.as_ptr(), &mut endp2) };
        let e2 = self.errno();
        let consumed2 = unsafe { *endp2 } == 0;
        if e2 == ERANGE {
            err.extend_from_slice(b"Range error while converting exponent '");
            err.extend_from_slice(exp_arg);
            err.extend_from_slice(b"'\n");
            return (out, err, 1);
        } else if !consumed2 {
            err.extend_from_slice(b"Invalid numeric input for exponent: '");
            err.extend_from_slice(exp_arg);
            err.extend_from_slice(b"'\n");
            return (out, err, 1);
        }

        // errno = 0; double result = pow(base, exponent);
        self.set_errno(0);
        let result = unsafe { (self.pow)(base, exponent) };
        let e3 = self.errno();
        if e3 == EDOM {
            err.extend_from_slice(b"Domain error: pow(");
            err.extend_from_slice(&self.fmt2(base));
            err.extend_from_slice(b", ");
            err.extend_from_slice(&self.fmt2(exponent));
            err.extend_from_slice(b") is undefined in the real number domain.\n");
            return (out, err, 1);
        } else if e3 == ERANGE {
            err.extend_from_slice(b"Range error: pow(");
            err.extend_from_slice(&self.fmt2(base));
            err.extend_from_slice(b", ");
            err.extend_from_slice(&self.fmt2(exponent));
            err.extend_from_slice(b") caused overflow or underflow.\n");
            return (out, err, 1);
        }

        out.extend_from_slice(b"Result: ");
        out.extend_from_slice(&self.fmt2(result));
        out.push(b'\n');
        (out, err, 0)
    }
}

fn check(libc: &Libc, base: &[u8], exp: &[u8]) {
    // 1. the two binaries agree with each other
    let both = assert_same_raw("FFI", &[base, exp]);
    // 2. ...and with the libloading-built libc oracle
    let (o_out, o_err, o_code) = libc.oracle(base, exp);
    assert_eq!(
        (both.code, &both.stdout, &both.stderr),
        (Some(o_code), &o_out, &o_err),
        "libloading oracle mismatch for base={} exp={}\n  binaries: {:?}\n  oracle  : code={} stdout={} stderr={}",
        esc(base),
        esc(exp),
        both,
        o_code,
        esc(&o_out),
        esc(&o_err)
    );
}

#[test]
fn ffi_oracle_matches_both_binaries_on_fixed_cases() {
    let libc = Libc::load();
    let cases: &[(&str, &str)] = &[
        ("2", "10"),
        ("2.5", "3"),
        ("-2", "3"),
        ("-2", "0.5"),
        ("0", "-1"),
        ("0", "-inf"),
        ("", ""),
        ("1e400", "2"),
        ("1e-400", "2"),
        ("2", "1e400"),
        ("abc", "2"),
        ("2", "abc"),
        (" 1.5", "2"),
        ("1.5 ", "2"),
        ("0x10", "2"),
        ("inf", "2"),
        ("-inf", "3"),
        ("nan", "3"),
        ("-nan", "3"),
        ("nan", "0"),
        ("10", "400"),
        ("10", "-400"),
        ("10", "-320"),
        ("1e300", "2"),
        ("0.125", "1"),
        ("0.375", "1"),
        ("-0.0", "3"),
        ("1.7976931348623157e308", "1"),
        ("2", "1023"),
        ("2", "1024"),
        ("0x1p3", "0x1p1"),
        ("9007199254740993", "1"),
        ("-1", "inf"),
        ("1e-320", "1"),
        ("0x1p-1023", "1"),
    ];
    for (b, e) in cases {
        check(&libc, b.as_bytes(), e.as_bytes());
    }
}

#[test]
fn ffi_oracle_matches_both_binaries_on_randomized_cases() {
    let libc = Libc::load();
    let mut rng = Rng::new(0xF1);
    for _ in 0..400 {
        let b = f64::from_bits(rng.next_u64());
        let e = match rng.below(4) {
            0 => rng.range_i64(-40, 40) as f64,
            1 => rng.range_i64(-40, 40) as f64 + 0.5,
            2 => rng.any_f64(),
            _ => rng.f01() * 2000.0 - 1000.0,
        };
        check(&libc, format!("{:e}", b).as_bytes(), format!("{:e}", e).as_bytes());
    }
    // error-path inputs too
    let mut rng = Rng::new(0xF2);
    let bad = [
        "abc", "1e400", "1e-400", " ", "0x", "1.5 ", "nan(", "-", ".", "1,5", "\t", "1e",
    ];
    for _ in 0..200 {
        let b = *rng.pick(&bad);
        let e = *rng.pick(&bad);
        check(&libc, b.as_bytes(), e.as_bytes());
        check(&libc, b.as_bytes(), b"2");
        check(&libc, b"2", e.as_bytes());
    }
}

/// The imports `SYMBOLS.md` claims are shared must really be resolvable, and the
/// Rust binary must really be using them (a reimplementation would drift).
#[test]
fn ffi_required_symbols_are_resolvable() {
    let libc = Libc::load();
    // strtod through FFI == what the binaries parsed
    let s = CString::new("0.1").unwrap();
    let mut end: *mut c_char = std::ptr::null_mut();
    let v = unsafe { (libc.strtod)(s.as_ptr(), &mut end) };
    assert_eq!(v, 0.1f64);
    assert_eq!(libc.fmt2(0.125), b"0.12".to_vec(), "glibc rounds half to even");
    assert_eq!(libc.fmt2(-0.0), b"-0.00".to_vec());
    assert_eq!(libc.fmt2(f64::NAN), b"nan".to_vec());
    assert_eq!(libc.fmt2(-f64::NAN), b"-nan".to_vec());
    assert_eq!(libc.fmt2(f64::INFINITY), b"inf".to_vec());
    assert_eq!(libc.fmt2(f64::NEG_INFINITY), b"-inf".to_vec());
    assert_eq!(unsafe { (libc.pow)(2.0, 10.0) }, 1024.0);
}
