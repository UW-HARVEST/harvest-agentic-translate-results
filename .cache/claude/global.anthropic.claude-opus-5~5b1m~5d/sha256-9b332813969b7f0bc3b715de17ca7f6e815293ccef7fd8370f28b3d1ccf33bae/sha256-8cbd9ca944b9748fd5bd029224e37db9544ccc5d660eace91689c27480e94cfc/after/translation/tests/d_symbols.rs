//! Phase D — symbol parity, plus the handful of exported symbols not reached by
//! the Phase B/C suites (`gethex`, `dtoa_r`, `strtod__unused`, `jsonp_error_vset`).

mod common;
use common::*;
use libloading::Symbol;
use std::collections::BTreeSet;
use std::os::raw::{c_char, c_double, c_int};
use std::process::Command;

fn exported_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only", path.to_str().unwrap()])
        .output()
        .expect("failed to run `nm`");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
        .collect()
}

/// The completion gate: every symbol the C `.so` exports must also be exported
/// by the Rust `.so`, with the exact same name.
#[test]
fn d_symbol_parity_is_exact() {
    let c = exported_symbols(&c_so_path());
    let r = exported_symbols(&rust_so_path());
    assert!(!c.is_empty(), "nm found no symbols in the C .so");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "{} symbol(s) exported by the C .so are MISSING from the Rust .so: {:?}",
        missing.len(),
        missing
    );
    // Report (but do not fail on) extras: the Rust cdylib may legitimately also
    // export Rust-runtime symbols.
    let extra: Vec<&String> = r.difference(&c).collect();
    assert!(
        extra.is_empty(),
        "the Rust .so exports {} symbol(s) the C .so does not: {:?}",
        extra.len(),
        extra
    );
    assert_eq!(c.len(), 130, "the C .so's public surface changed unexpectedly");
}

/// Every symbol must additionally be *resolvable* through `dlsym`, which is what
/// a real consumer does — a symbol present in `nm` but with a bad type would
/// still be found here, so this is a weaker but independent check.
#[test]
fn d_every_symbol_is_dlsym_resolvable() {
    let p = pair();
    let c = exported_symbols(&c_so_path());
    for name in &c {
        let mut z = name.clone().into_bytes();
        z.push(0);
        unsafe {
            let cs_: Result<Symbol<*mut ()>, _> = p.c.lib.get(&z);
            let rs_: Result<Symbol<*mut ()>, _> = p.r.lib.get(&z);
            assert!(cs_.is_ok(), "dlsym failed on the C .so for {name}");
            assert!(
                rs_.is_ok(),
                "dlsym failed on the Rust .so for {name} — present in nm but not resolvable"
            );
        }
    }
}

// ===========================================================================
// dtoa_r — the reentrant form, which can write into a caller-supplied buffer.
// ===========================================================================

type DtoaR = unsafe extern "C" fn(
    c_double,
    c_int,
    c_int,
    *mut c_int,
    *mut c_int,
    *mut *mut c_char,
    *mut c_char,
    usize,
) -> *mut c_char;

#[test]
fn d_dtoa_r_direct() {
    let p = pair();
    unsafe {
        let cf: Symbol<DtoaR> = p.c.lib.get(b"dtoa_r\0").unwrap();
        let rf: Symbol<DtoaR> = p.r.lib.get(b"dtoa_r\0").unwrap();

        let mut vals: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 0.5, 2.5, 0.125, 1.0 / 3.0, 1e-5, 1e15, 1e16, 1e17, 1e300,
            1e-300, f64::MAX, f64::MIN, f64::MIN_POSITIVE, 5e-324, f64::EPSILON,
            9007199254740992.0, 1234.5678, 123456789.123456789,
        ];
        let mut rng = Rng::new(0xD704_5217);
        for _ in 0..1500 {
            vals.push(rng.finite_f64());
        }

        for &v in &vals {
            for mode in [-1i32, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10] {
                for nd in [-1i32, 0, 1, 2, 5, 15, 17, 18, 24, 25, 30] {
                    // (a) with the internal allocator (buf == NULL)
                    for &(buf_mode, blen) in &[
                        (0usize, 0usize),   // NULL buf
                        (1, 0),             // buf given, blen 0
                        (1, 1),
                        (1, 8),
                        (1, 18),
                        (1, 25),
                        (1, 26),
                        (1, 64),
                    ] {
                        let mut cbuf = [0x5Au8; 128];
                        let mut rbuf = [0x5Au8; 128];
                        let (cb, rb) = if buf_mode == 0 {
                            (std::ptr::null_mut(), std::ptr::null_mut())
                        } else {
                            (
                                cbuf.as_mut_ptr() as *mut c_char,
                                rbuf.as_mut_ptr() as *mut c_char,
                            )
                        };
                        let mut cdec: c_int = -9999;
                        let mut csign: c_int = -9999;
                        let mut cend: *mut c_char = std::ptr::null_mut();
                        let mut rdec: c_int = -9999;
                        let mut rsign: c_int = -9999;
                        let mut rend: *mut c_char = std::ptr::null_mut();
                        let cr = cf(v, mode, nd, &mut cdec, &mut csign, &mut cend, cb, blen);
                        let rr = rf(v, mode, nd, &mut rdec, &mut rsign, &mut rend, rb, blen);
                        let ctx = format!(
                            "dtoa_r({v:?}, mode {mode}, nd {nd}, buf_mode {buf_mode}, blen {blen})"
                        );
                        assert_eq!(cr.is_null(), rr.is_null(), "{ctx}: NULL-ness");
                        if cr.is_null() {
                            continue;
                        }
                        let cd = cstr_bytes(cr).unwrap();
                        let rd = cstr_bytes(rr).unwrap();
                        let coff = if cend.is_null() {
                            None
                        } else {
                            Some(cend as usize - cr as usize)
                        };
                        let roff = if rend.is_null() {
                            None
                        } else {
                            Some(rend as usize - rr as usize)
                        };
                        assert_eq!(
                            (&cd, cdec, csign, coff),
                            (&rd, rdec, rsign, roff),
                            "{ctx}: C=({:?},{cdec},{csign},{coff:?}) RUST=({:?},{rdec},{rsign},{roff:?})",
                            String::from_utf8_lossy(&cd),
                            String::from_utf8_lossy(&rd)
                        );
                        // Only free when the library allocated the buffer.
                        if buf_mode == 0 {
                            p.c.freedtoa(cr);
                            p.r.freedtoa(rr);
                        }
                    }
                }
            }
        }
    }
}

// ===========================================================================
// strtod__unused — dtoa's own strtod, exported but unused by jansson itself.
// ===========================================================================

type StrtodU = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_double;

#[test]
fn d_strtod__unused_direct() {
    let p = pair();
    unsafe {
        let cf: Symbol<StrtodU> = p.c.lib.get(b"strtod__unused\0").unwrap();
        let rf: Symbol<StrtodU> = p.r.lib.get(b"strtod__unused\0").unwrap();

        let mut texts: Vec<String> = vec![
            "".into(),
            " ".into(),
            "0".into(),
            "-0".into(),
            "+0".into(),
            "1".into(),
            "-1".into(),
            "1.5".into(),
            ".5".into(),
            "5.".into(),
            "1e5".into(),
            "1E5".into(),
            "1e+5".into(),
            "1e-5".into(),
            "1e309".into(),
            "1e-400".into(),
            "1e999999".into(),
            "-1e999999".into(),
            "3.141592653589793".into(),
            "1.7976931348623157e308".into(),
            "2.2250738585072014e-308".into(),
            "4.9406564584124654e-324".into(),
            "9007199254740993".into(),
            "0.1".into(),
            "0.30000000000000004".into(),
            "  42abc".into(),
            "42abc".into(),
            "abc".into(),
            "-".into(),
            "+".into(),
            "e5".into(),
            ".".into(),
            "inf".into(),
            "Infinity".into(),
            "nan".into(),
            "NaN".into(),
            "NAN(123)".into(),
            // hex floats, which route through gethex
            "0x1p3".into(),
            "0X1P3".into(),
            "0x1.8p1".into(),
            "0x1.fffffffffffffp1023".into(),
            "0x0p0".into(),
            "-0x1p-1".into(),
            "0x".into(),
            "0xg".into(),
            "0x1p".into(),
            "0x1p+".into(),
            "0x.8p1".into(),
            "0x10".into(),
            "000000000000000000001".into(),
            "0.000000000000000000001".into(),
            "1000000000000000000000000000000".into(),
        ];
        let mut rng = Rng::new(0x5710_D_2);
        for _ in 0..3000 {
            let m = rng.next_u64() as i64;
            let e = rng.range(-340, 340);
            texts.push(format!("{m}e{e}"));
            texts.push(format!("{m}.{:06}e{e}", rng.below(1_000_000)));
            texts.push(format!("0x{:x}p{}", rng.next_u64(), rng.range(-60, 60)));
        }

        for t in &texts {
            // Pad with NULs so a scanner that reads one byte past the end stays
            // inside the allocation.
            let mut z = t.clone().into_bytes();
            z.extend_from_slice(&[0u8; 8]);
            let mut cend: *mut c_char = std::ptr::null_mut();
            let mut rend: *mut c_char = std::ptr::null_mut();
            let base = z.as_ptr() as *const c_char;
            let cv = cf(base, &mut cend);
            let rv = rf(base, &mut rend);
            let coff = if cend.is_null() {
                None
            } else {
                Some(cend as usize - base as usize)
            };
            let roff = if rend.is_null() {
                None
            } else {
                Some(rend as usize - base as usize)
            };
            assert_eq!(
                (cv.to_bits(), coff),
                (rv.to_bits(), roff),
                "strtod__unused({t:?}): C=({cv:?}, end+{coff:?}) RUST=({rv:?}, end+{roff:?})"
            );
            // A NULL `se` out-param must also be accepted.
            let cv2 = cf(base, std::ptr::null_mut());
            let rv2 = rf(base, std::ptr::null_mut());
            assert_eq!(
                cv2.to_bits(),
                rv2.to_bits(),
                "strtod__unused({t:?}, NULL se)"
            );
        }
    }
}

// ===========================================================================
// gethex — `void gethex(const char **sp, U *rvp, int rounding, int sign)`
// (`U` is `union { double d; ULong L[2]; }`, and MULTIPLE_THREADS is not set,
//  so there is no trailing ThInfo** parameter.)
// ===========================================================================

#[repr(C)]
#[derive(Clone, Copy)]
union U {
    d: c_double,
    l: [u32; 2],
}

type GetHex = unsafe extern "C" fn(*mut *const c_char, *mut U, c_int, c_int);

#[test]
fn d_gethex_direct() {
    let p = pair();
    unsafe {
        let cf: Symbol<GetHex> = p.c.lib.get(b"gethex\0").unwrap();
        let rf: Symbol<GetHex> = p.r.lib.get(b"gethex\0").unwrap();

        // `gethex` is entered with `*sp` pointing at the 'x'/'X' of a "0x..."
        // literal (see the call site in strtod__unused), so every input starts
        // with that.
        let mut texts: Vec<String> = vec![
            "x0".into(),
            "X0".into(),
            "x1".into(),
            "x1p0".into(),
            "x1p1".into(),
            "x1p3".into(),
            "x1p-1".into(),
            "x1p+1".into(),
            "X1P3".into(),
            "x1.8p1".into(),
            "x.8p1".into(),
            "x8.p1".into(),
            "x1.fffffffffffffp1023".into(),
            "x1p1024".into(),
            "x1p-1075".into(),
            "x1p-1074".into(),
            "x1p-1022".into(),
            "x1p-1023".into(),
            "xffffffffffffffff".into(),
            "x0000000000000001".into(),
            "x123456789abcdef".into(),
            "xABCDEF".into(),
            "x10".into(),
            "x".into(),
            "xg".into(),
            "x1p".into(),
            "x1p+".into(),
            "x1p-".into(),
            "x0p0".into(),
            "x0.0p0".into(),
            "x00000".into(),
            "x1.0000000000000000000000001p0".into(),
        ];
        let mut rng = Rng::new(0x6E74_4E58);
        for _ in 0..2000 {
            texts.push(format!("x{:x}p{}", rng.next_u64(), rng.range(-1100, 1100)));
            texts.push(format!(
                "x{:x}.{:x}p{}",
                rng.next_u64() >> 32,
                rng.next_u64() >> 32,
                rng.range(-80, 80)
            ));
        }

        // Round_zero = 0, Round_near = 1, Round_up = 2, Round_down = 3.
        for t in &texts {
            for rounding in [0i32, 1, 2, 3] {
                for sign in [0i32, 1] {
                    let mut z = t.clone().into_bytes();
                    z.extend_from_slice(&[0u8; 8]);
                    let base = z.as_ptr() as *const c_char;

                    let mut csp: *const c_char = base;
                    let mut rsp: *const c_char = base;
                    let mut cu = U { l: [0xDEAD_BEEF, 0xDEAD_BEEF] };
                    let mut ru = U { l: [0xDEAD_BEEF, 0xDEAD_BEEF] };
                    cf(&mut csp, &mut cu, rounding, sign);
                    rf(&mut rsp, &mut ru, rounding, sign);

                    let ctx = format!("gethex({t:?}, rounding {rounding}, sign {sign})");
                    assert_eq!(
                        csp as usize - base as usize,
                        rsp as usize - base as usize,
                        "{ctx}: advanced *sp"
                    );
                    assert_eq!(cu.l, ru.l, "{ctx}: U bits (C {:?} RUST {:?})", cu.l, ru.l);
                    assert_eq!(
                        cu.d.to_bits(),
                        ru.d.to_bits(),
                        "{ctx}: double value (C {:?} RUST {:?})",
                        cu.d,
                        ru.d
                    );
                }
            }
        }
    }
}

// ===========================================================================
// jsonp_error_vset — takes a `va_list`, which cannot be portably synthesised
// from Rust. It is the implementation `jsonp_error_set` forwards to, so it is
// covered transitively by `c_errors::e1_e7_error_api_rejections` and by every
// decoder/pack error test. This test only pins its presence and that the
// variadic wrapper reaches it identically.
// ===========================================================================

#[test]
fn d_jsonp_error_vset_present_and_reached_via_wrapper() {
    let p = pair();
    unsafe {
        let c: Result<Symbol<*mut ()>, _> = p.c.lib.get(b"jsonp_error_vset\0");
        let r: Result<Symbol<*mut ()>, _> = p.r.lib.get(b"jsonp_error_vset\0");
        assert!(c.is_ok() && r.is_ok(), "jsonp_error_vset must be exported");

        // Reached through the variadic wrapper, with every code and several
        // format shapes.
        let cset = p.c.jsonp_error_set_sym();
        let rset = p.r.jsonp_error_set_sym();
        for code in 0..=17i32 {
            for (fmt, arg) in [("%s", "text"), ("%.6s", "abcdefghij"), ("[%s]", "")] {
                let mut ce = json_error_t::default();
                let mut re = json_error_t::default();
                p.c.jsonp_error_init(&mut ce, cs("src").as_ptr());
                p.r.jsonp_error_init(&mut re, cs("src").as_ptr());
                let f = cs(fmt);
                let a = cs(arg);
                cset(&mut ce, 11, 22, 33, code, f.as_ptr(), a.as_ptr());
                rset(&mut re, 11, 22, 33, code, f.as_ptr(), a.as_ptr());
                assert_eq!(
                    ce.snap(),
                    re.snap(),
                    "jsonp_error_set -> vset (code {code}, fmt {fmt:?})"
                );
            }
        }
    }
}
