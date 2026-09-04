//! Phase B — CONFIGS.md section E: pack / unpack / sprintf, including the
//! `va_list` entry points reached through a C shim.
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

unsafe fn d(api: &Api, j: Jt) -> Option<Vec<u8>> {
    unsafe { dumps(api, j, JSON_ENCODE_ANY | JSON_SORT_KEYS) }
}

/* ===================== E1: every scalar specifier ===================== */

#[test]
fn e1_pack_scalars() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0xE1);
    unsafe {
        // n / b
        for f in ["n", "[n]", "{s:n}"] {
            let z = cstr(f);
            let k = cstr("k");
            let (a, b) = if f.starts_with('{') {
                (
                    (p.c.json_pack)(z.as_ptr(), k.as_ptr()),
                    (p.r.json_pack)(z.as_ptr(), k.as_ptr()),
                )
            } else {
                ((p.c.json_pack)(z.as_ptr()), (p.r.json_pack)(z.as_ptr()))
            };
            assert_eq!(d(p.c, a), d(p.r, b), "pack {f}");
            decref(p.c, a);
            decref(p.r, b);
        }
        for v in [0i32, 1, -1, 42, i32::MIN, i32::MAX] {
            for f in ["b", "[b]", "i", "[i]", "[i,i]"] {
                let z = cstr(f);
                let a = (p.c.json_pack)(z.as_ptr(), v, v);
                let b = (p.r.json_pack)(z.as_ptr(), v, v);
                assert_eq!(d(p.c, a), d(p.r, b), "pack {f} v={v}");
                decref(p.c, a);
                decref(p.r, b);
            }
        }
        // I (json_int_t)
        let mut ints: Vec<i64> = vec![0, 1, -1, i64::MIN, i64::MAX, 1 << 40];
        for _ in 0..200 {
            ints.push(rng.i64());
        }
        for v in ints {
            let z = cstr("[I]");
            let a = (p.c.json_pack)(z.as_ptr(), v);
            let b = (p.r.json_pack)(z.as_ptr(), v);
            assert_eq!(d(p.c, a), d(p.r, b), "pack [I] v={v}");
            decref(p.c, a);
            decref(p.r, b);
        }
        // f (double), incl. non-finite -> error
        let mut reals: Vec<f64> = vec![
            0.0,
            -0.0,
            1.5,
            -1.5,
            f64::MAX,
            f64::MIN,
            5e-324,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ];
        for _ in 0..200 {
            reals.push(rng.tame_f64());
        }
        for v in reals {
            let z = cstr("[f]");
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let a = (p.c.json_pack_ex)(&mut ec, 0usize, z.as_ptr(), v);
            let b = (p.r.json_pack_ex)(&mut er, 0usize, z.as_ptr(), v);
            assert_eq!(d(p.c, a), d(p.r, b), "pack [f] v={v:?}");
            assert_eq!(ec.snapshot(), er.snapshot(), "pack [f] error v={v:?}");
            decref(p.c, a);
            decref(p.r, b);
        }
        // s
        for s in ["", "a", "hello", "é€中𝄞", "with \"quotes\" and \\ and /"] {
            let zs = cstr(s);
            for f in ["s", "[s]", "[s,s]"] {
                let z = cstr(f);
                let a = (p.c.json_pack)(z.as_ptr(), zs.as_ptr(), zs.as_ptr());
                let b = (p.r.json_pack)(z.as_ptr(), zs.as_ptr(), zs.as_ptr());
                assert_eq!(d(p.c, a), d(p.r, b), "pack {f} s={s:?}");
                decref(p.c, a);
                decref(p.r, b);
            }
        }
    }
}

/* ===================== E2: s#, s%, s+, s+#, s+% ===================== */

#[test]
fn e2_pack_string_modifiers() {
    let _g = lock();
    let p = pair();
    unsafe {
        let s1 = cstr("abcdefgh");
        let s2 = cstr("0123456789");
        // s#  (length as int)
        for l in [0i32, 1, 3, 8] {
            let z = cstr("s#");
            let a = (p.c.json_pack)(z.as_ptr(), s1.as_ptr(), l);
            let b = (p.r.json_pack)(z.as_ptr(), s1.as_ptr(), l);
            assert_eq!(d(p.c, a), d(p.r, b), "pack s# l={l}");
            decref(p.c, a);
            decref(p.r, b);
        }
        // s%  (length as size_t)
        for l in [0usize, 1, 3, 8] {
            let z = cstr("s%");
            let a = (p.c.json_pack)(z.as_ptr(), s1.as_ptr(), l);
            let b = (p.r.json_pack)(z.as_ptr(), s1.as_ptr(), l);
            assert_eq!(d(p.c, a), d(p.r, b), "pack s% l={l}");
            decref(p.c, a);
            decref(p.r, b);
        }
        // s+  (concatenation)
        let z = cstr("s+");
        let a = (p.c.json_pack)(z.as_ptr(), s1.as_ptr(), s2.as_ptr());
        let b = (p.r.json_pack)(z.as_ptr(), s1.as_ptr(), s2.as_ptr());
        assert_eq!(d(p.c, a), d(p.r, b), "pack s+");
        decref(p.c, a);
        decref(p.r, b);
        // s++ (three-way)
        let z = cstr("s++");
        let a = (p.c.json_pack)(z.as_ptr(), s1.as_ptr(), s2.as_ptr(), s1.as_ptr());
        let b = (p.r.json_pack)(z.as_ptr(), s1.as_ptr(), s2.as_ptr(), s1.as_ptr());
        assert_eq!(d(p.c, a), d(p.r, b), "pack s++");
        decref(p.c, a);
        decref(p.r, b);
        // s#+#  and s%+%
        for z in ["s#+#", "s%+%"] {
            let zf = cstr(z);
            if z.contains('#') {
                let a = (p.c.json_pack)(zf.as_ptr(), s1.as_ptr(), 3i32, s2.as_ptr(), 4i32);
                let b = (p.r.json_pack)(zf.as_ptr(), s1.as_ptr(), 3i32, s2.as_ptr(), 4i32);
                assert_eq!(d(p.c, a), d(p.r, b), "pack {z}");
                decref(p.c, a);
                decref(p.r, b);
            } else {
                let a = (p.c.json_pack)(zf.as_ptr(), s1.as_ptr(), 3usize, s2.as_ptr(), 4usize);
                let b = (p.r.json_pack)(zf.as_ptr(), s1.as_ptr(), 3usize, s2.as_ptr(), 4usize);
                assert_eq!(d(p.c, a), d(p.r, b), "pack {z}");
                decref(p.c, a);
                decref(p.r, b);
            }
        }
        // inside objects / arrays
        let key = cstr("k");
        for z in ["{s:s#}", "[s#,s%]", "{s:s+}"] {
            let zf = cstr(z);
            let (a, b) = match z {
                "{s:s#}" => (
                    (p.c.json_pack)(zf.as_ptr(), key.as_ptr(), s1.as_ptr(), 4i32),
                    (p.r.json_pack)(zf.as_ptr(), key.as_ptr(), s1.as_ptr(), 4i32),
                ),
                "[s#,s%]" => (
                    (p.c.json_pack)(zf.as_ptr(), s1.as_ptr(), 2i32, s2.as_ptr(), 3usize),
                    (p.r.json_pack)(zf.as_ptr(), s1.as_ptr(), 2i32, s2.as_ptr(), 3usize),
                ),
                _ => (
                    (p.c.json_pack)(zf.as_ptr(), key.as_ptr(), s1.as_ptr(), s2.as_ptr()),
                    (p.r.json_pack)(zf.as_ptr(), key.as_ptr(), s1.as_ptr(), s2.as_ptr()),
                ),
            };
            assert_eq!(d(p.c, a), d(p.r, b), "pack {z}");
            decref(p.c, a);
            decref(p.r, b);
        }
    }
}

/* ===================== E3: s? and s* ===================== */

#[test]
fn e3_pack_optional_strings() {
    let _g = lock();
    let p = pair();
    unsafe {
        let s = cstr("val");
        let key = cstr("k");
        for z in ["s?", "s*", "[s?]", "[s*]", "{s:s?}", "{s:s*}", "[s?,i]", "[s*,i]"] {
            let zf = cstr(z);
            for nonnull in [true, false] {
                let arg: *const c_char = if nonnull { s.as_ptr() } else { std::ptr::null() };
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let (a, b) = if z.starts_with('{') {
                    (
                        (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), key.as_ptr(), arg),
                        (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), key.as_ptr(), arg),
                    )
                } else if z.contains(",i") {
                    (
                        (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), arg, 7i32),
                        (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), arg, 7i32),
                    )
                } else {
                    (
                        (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), arg),
                        (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), arg),
                    )
                };
                assert_eq!(d(p.c, a), d(p.r, b), "pack {z} nonnull={nonnull}");
                assert_eq!(ec.snapshot(), er.snapshot(), "pack {z} nonnull={nonnull} error");
                decref(p.c, a);
                decref(p.r, b);
            }
        }
    }
}

/* ===================== E4: o and O ===================== */

#[test]
fn e4_pack_object_inter() {
    let _g = lock();
    let p = pair();
    unsafe {
        let key = cstr("k");
        for z in ["o", "O", "o?", "O?", "o*", "O*", "[o]", "[O]", "{s:o}", "{s:O}"] {
            let zf = cstr(z);
            for present in [true, false] {
                let vc: Jt = if present {
                    (p.c.json_loads)(cstr("[1,2]").as_ptr(), 0, std::ptr::null_mut())
                } else {
                    std::ptr::null_mut()
                };
                let vr: Jt = if present {
                    (p.r.json_loads)(cstr("[1,2]").as_ptr(), 0, std::ptr::null_mut())
                } else {
                    std::ptr::null_mut()
                };
                let rc_before_c = if vc.is_null() { 0 } else { (*vc).refcount };
                let rc_before_r = if vr.is_null() { 0 } else { (*vr).refcount };
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let (a, b) = if z.starts_with('{') {
                    (
                        (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), key.as_ptr(), vc),
                        (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), key.as_ptr(), vr),
                    )
                } else {
                    (
                        (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), vc),
                        (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), vr),
                    )
                };
                assert_eq!(d(p.c, a), d(p.r, b), "pack {z} present={present}");
                assert_eq!(ec.snapshot(), er.snapshot(), "pack {z} error");
                // refcount effect of 'O' vs 'o'
                let rc_after_c = if vc.is_null() { 0 } else { (*vc).refcount };
                let rc_after_r = if vr.is_null() { 0 } else { (*vr).refcount };
                assert_eq!(
                    rc_after_c.wrapping_sub(rc_before_c),
                    rc_after_r.wrapping_sub(rc_before_r),
                    "refcount delta for {z} present={present}"
                );
                decref(p.c, a);
                decref(p.r, b);
            }
        }
    }
}

/* ===================== E5: nesting and format whitespace ===================== */

#[test]
fn e5_pack_nested() {
    let _g = lock();
    let p = pair();
    unsafe {
        let k1 = cstr("alpha");
        let k2 = cstr("beta");
        let k3 = cstr("gamma");
        let sv = cstr("text");
        for z in [
            "{s:[i,i],s:{s:s}}",
            "{ s : [ i , i ] , s : { s : s } }",
            "{s:[i,i]\n,s:{s:s}}",
            "{s:[i,i],,,s:{s:s}}",
            "[[[[i]]]]",
            "{s:{s:{s:i}}}",
        ] {
            let zf = cstr(z);
            let (a, b) = match z {
                "[[[[i]]]]" => (
                    (p.c.json_pack)(zf.as_ptr(), 5i32),
                    (p.r.json_pack)(zf.as_ptr(), 5i32),
                ),
                "{s:{s:{s:i}}}" => (
                    (p.c.json_pack)(zf.as_ptr(), k1.as_ptr(), k2.as_ptr(), k3.as_ptr(), 9i32),
                    (p.r.json_pack)(zf.as_ptr(), k1.as_ptr(), k2.as_ptr(), k3.as_ptr(), 9i32),
                ),
                _ => (
                    (p.c.json_pack)(
                        zf.as_ptr(),
                        k1.as_ptr(),
                        1i32,
                        2i32,
                        k2.as_ptr(),
                        k3.as_ptr(),
                        sv.as_ptr(),
                    ),
                    (p.r.json_pack)(
                        zf.as_ptr(),
                        k1.as_ptr(),
                        1i32,
                        2i32,
                        k2.as_ptr(),
                        k3.as_ptr(),
                        sv.as_ptr(),
                    ),
                ),
            };
            assert_eq!(d(p.c, a), d(p.r, b), "pack {z}");
            decref(p.c, a);
            decref(p.r, b);
        }
    }
}

/* ===================== E6 / E14: *_ex and the va_list forms ================= */

#[test]
fn e6_e14_pack_ex_and_vpack_ex() {
    let _g = lock();
    let p = pair();
    let sh = vshim();
    unsafe {
        let ca = p.c.json_vpack_ex as usize as *mut c_void;
        let ra = p.r.json_vpack_ex as usize as *mut c_void;
        let key = cstr("k");
        let sv = cstr("v");
        for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT, JSON_VALIDATE_ONLY | JSON_STRICT] {
            // json_pack_ex
            for z in ["[i]", "{s:s}", "n", "bad", ""] {
                let zf = cstr(z);
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let a = (p.c.json_pack_ex)(&mut ec, flags, zf.as_ptr(), key.as_ptr(), sv.as_ptr());
                let b = (p.r.json_pack_ex)(&mut er, flags, zf.as_ptr(), key.as_ptr(), sv.as_ptr());
                assert_eq!(d(p.c, a), d(p.r, b), "json_pack_ex {z} flags={flags}");
                assert_eq!(ec.snapshot(), er.snapshot(), "json_pack_ex {z} error");
                decref(p.c, a);
                decref(p.r, b);
            }
            // json_vpack_ex through the shim: no args, one int, one string, string+int
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let zf = cstr("[n,n]");
            let a = (sh.vpack_0)(ca, &mut ec, flags, zf.as_ptr());
            let b = (sh.vpack_0)(ra, &mut er, flags, zf.as_ptr());
            assert_eq!(d(p.c, a), d(p.r, b), "vpack_ex [n,n]");
            assert_eq!(ec.snapshot(), er.snapshot());
            decref(p.c, a);
            decref(p.r, b);

            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let zf = cstr("[i]");
            let a = (sh.vpack_i)(ca, &mut ec, flags, zf.as_ptr(), 1234);
            let b = (sh.vpack_i)(ra, &mut er, flags, zf.as_ptr(), 1234);
            assert_eq!(d(p.c, a), d(p.r, b), "vpack_ex [i]");
            assert_eq!(ec.snapshot(), er.snapshot());
            decref(p.c, a);
            decref(p.r, b);

            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let zf = cstr("[s]");
            let a = (sh.vpack_s)(ca, &mut ec, flags, zf.as_ptr(), sv.as_ptr());
            let b = (sh.vpack_s)(ra, &mut er, flags, zf.as_ptr(), sv.as_ptr());
            assert_eq!(d(p.c, a), d(p.r, b), "vpack_ex [s]");
            assert_eq!(ec.snapshot(), er.snapshot());
            decref(p.c, a);
            decref(p.r, b);

            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let zf = cstr("{s:i}");
            let a = (sh.vpack_si)(ca, &mut ec, flags, zf.as_ptr(), key.as_ptr(), 77);
            let b = (sh.vpack_si)(ra, &mut er, flags, zf.as_ptr(), key.as_ptr(), 77);
            assert_eq!(d(p.c, a), d(p.r, b), "vpack_ex {{s:i}}");
            assert_eq!(ec.snapshot(), er.snapshot());
            decref(p.c, a);
            decref(p.r, b);

            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let zf = cstr("{s:i,s:i}");
            let a = (sh.vpack_sisi)(ca, &mut ec, flags, zf.as_ptr(), key.as_ptr(), 1, sv.as_ptr(), 2);
            let b = (sh.vpack_sisi)(ra, &mut er, flags, zf.as_ptr(), key.as_ptr(), 1, sv.as_ptr(), 2);
            assert_eq!(d(p.c, a), d(p.r, b), "vpack_ex two pairs");
            assert_eq!(ec.snapshot(), er.snapshot());
            decref(p.c, a);
            decref(p.r, b);

            for v in [0.0f64, 1.5, -2.25, f64::NAN, f64::INFINITY] {
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let zf = cstr("[f]");
                let a = (sh.vpack_d)(ca, &mut ec, flags, zf.as_ptr(), v);
                let b = (sh.vpack_d)(ra, &mut er, flags, zf.as_ptr(), v);
                assert_eq!(d(p.c, a), d(p.r, b), "vpack_ex [f] {v:?}");
                assert_eq!(ec.snapshot(), er.snapshot());
                decref(p.c, a);
                decref(p.r, b);
            }
            // NULL / empty format through the va_list entry point
            for zf in [std::ptr::null(), cstr("").as_ptr()] {
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let a = (sh.vpack_0)(ca, &mut ec, flags, zf);
                let b = (sh.vpack_0)(ra, &mut er, flags, zf);
                assert!(a.is_null() && b.is_null());
                assert_eq!(ec.snapshot(), er.snapshot());
            }
        }
    }
}

/* ===================== E7: json_sprintf / json_vsprintf ===================== */

#[test]
fn e7_sprintf() {
    let _g = lock();
    let p = pair();
    let sh = vshim();
    unsafe {
        let ca = p.c.json_vsprintf as usize as *mut c_void;
        let ra = p.r.json_vsprintf as usize as *mut c_void;
        // empty result
        for f in ["", "plain", "%%"] {
            let zf = cstr(f);
            let a = (p.c.json_sprintf)(zf.as_ptr());
            let b = (p.r.json_sprintf)(zf.as_ptr());
            assert_eq!(d(p.c, a), d(p.r, b), "json_sprintf {f:?}");
            decref(p.c, a);
            decref(p.r, b);
            let a = (sh.vsprintf_0)(ca, zf.as_ptr());
            let b = (sh.vsprintf_0)(ra, zf.as_ptr());
            assert_eq!(d(p.c, a), d(p.r, b), "json_vsprintf {f:?}");
            decref(p.c, a);
            decref(p.r, b);
        }
        // %d
        for v in [0i32, -1, 12345, i32::MIN, i32::MAX] {
            let zf = cstr("n=%d");
            let a = (p.c.json_sprintf)(zf.as_ptr(), v);
            let b = (p.r.json_sprintf)(zf.as_ptr(), v);
            assert_eq!(d(p.c, a), d(p.r, b), "sprintf %d {v}");
            decref(p.c, a);
            decref(p.r, b);
            let a = (sh.vsprintf_i)(ca, zf.as_ptr(), v);
            let b = (sh.vsprintf_i)(ra, zf.as_ptr(), v);
            assert_eq!(d(p.c, a), d(p.r, b), "vsprintf %d {v}");
            decref(p.c, a);
            decref(p.r, b);
        }
        // %s, incl. UTF-8, invalid UTF-8, and results over 160 bytes
        let long = "L".repeat(500);
        let inputs = ["", "x", "héllo €", "𝄞", &long, "a/b\\c\"d"];
        for s in inputs {
            let zs = cstr(s);
            let zf = cstr("[%s]");
            let a = (p.c.json_sprintf)(zf.as_ptr(), zs.as_ptr());
            let b = (p.r.json_sprintf)(zf.as_ptr(), zs.as_ptr());
            assert_eq!(d(p.c, a), d(p.r, b), "sprintf %s {s:?}");
            decref(p.c, a);
            decref(p.r, b);
            let a = (sh.vsprintf_s)(ca, zf.as_ptr(), zs.as_ptr());
            let b = (sh.vsprintf_s)(ra, zf.as_ptr(), zs.as_ptr());
            assert_eq!(d(p.c, a), d(p.r, b), "vsprintf %s {s:?}");
            decref(p.c, a);
            decref(p.r, b);
        }
        // invalid UTF-8 argument -> NULL on both
        let bad = nul_terminated(b"\xff\xfe");
        let zf = cstr("%s");
        let a = (p.c.json_sprintf)(zf.as_ptr(), bad.as_ptr());
        let b = (p.r.json_sprintf)(zf.as_ptr(), bad.as_ptr());
        assert!(a.is_null() && b.is_null(), "invalid UTF-8 sprintf must fail");
        // %f / %g
        for v in [0.0f64, 1.5, -2.25, 1e300, 1e-300] {
            for f in ["%f", "%.3f", "%g", "%e"] {
                let zf = cstr(f);
                let a = (p.c.json_sprintf)(zf.as_ptr(), v);
                let b = (p.r.json_sprintf)(zf.as_ptr(), v);
                assert_eq!(d(p.c, a), d(p.r, b), "sprintf {f} {v:?}");
                decref(p.c, a);
                decref(p.r, b);
                let a = (sh.vsprintf_d)(ca, zf.as_ptr(), v);
                let b = (sh.vsprintf_d)(ra, zf.as_ptr(), v);
                assert_eq!(d(p.c, a), d(p.r, b), "vsprintf {f} {v:?}");
                decref(p.c, a);
                decref(p.r, b);
            }
        }
        // combined
        let zs = cstr("str");
        let zf = cstr("%s-%d");
        let a = (sh.vsprintf_si)(ca, zf.as_ptr(), zs.as_ptr(), 42);
        let b = (sh.vsprintf_si)(ra, zf.as_ptr(), zs.as_ptr(), 42);
        assert_eq!(d(p.c, a), d(p.r, b), "vsprintf %s-%d");
        decref(p.c, a);
        decref(p.r, b);
    }
}

/* ===================== E8..E13: unpack ===================== */

/// One unpack observation: return code, error struct, and the out-params.
#[derive(PartialEq, Debug)]
struct U {
    ret: c_int,
    err: (i32, i32, i32, Vec<u8>, Vec<u8>),
    sptr: Option<Vec<u8>>,
    slen: usize,
    i: c_int,
    ii: i64,
    f: u64,
    obj: bool,
}

#[test]
fn e8_e13_unpack_matrix() {
    let _g = lock();
    let p = pair();
    let roots = [
        "null", "true", "false", "0", "42", "-1", "1.5", "0.0", "\"str\"", "\"\"", "[]",
        "[1,2,3]", "[\"a\",\"b\"]", "{}", "{\"a\":1}", "{\"a\":1,\"b\":2}",
        "{\"a\":\"s\",\"b\":[1,2],\"c\":{\"d\":null}}",
    ];
    let formats = [
        "n", "b", "i", "I", "f", "F", "s", "o", "O", "s%", "[i]", "[i,i]", "[i,i,i]",
        "[i!]", "[i*]", "[s]", "{s:i}", "{s:i,s:i}", "{s:i!}", "{s:i*}", "{s?i}",
        "{s:i,s?i}", "{s:s}", "{s:o}", "[]", "{}", "[n]", "q", "[q]", "{s:q}", "[i,",
        "{s:i", "[!]", "[*]", "[i!i]", "[i*i]", "{s:i!s:i}",
    ];
    let flag_sets = [
        0usize,
        JSON_VALIDATE_ONLY,
        JSON_STRICT,
        JSON_VALIDATE_ONLY | JSON_STRICT,
    ];
    unsafe {
        for root in roots {
            for fmt in formats {
                for flags in flag_sets {
                    let mut obs = Vec::new();
                    for api in [p.c, p.r] {
                        let r = (api.json_loads)(
                            cstr(root).as_ptr(),
                            JSON_DECODE_ANY,
                            std::ptr::null_mut(),
                        );
                        let zf = cstr(fmt);
                        let mut e = JsonError::zeroed();
                        let mut sptr: *const c_char = std::ptr::null();
                        let mut slen: usize = 0xdead;
                        let mut iv: c_int = -12345;
                        let mut iiv: i64 = -12345;
                        let mut fv: f64 = -12345.0;
                        let mut ov: Jt = std::ptr::null_mut();
                        let ka = cstr("a");
                        let kb = cstr("b");
                        // Which format strings take object keys as varargs?
                        let keyed1 = matches!(
                            fmt,
                            "{s:i}" | "{s:i!}" | "{s:i*}" | "{s?i}" | "{s:s}" | "{s:o}" | "{s:q}"
                        );
                        let keyed2 = matches!(fmt, "{s:i,s:i}" | "{s:i,s?i}" | "{s:i!s:i}");
                        // Under JSON_VALIDATE_ONLY the value specifiers consume
                        // NO varargs (see `unpack()` in pack_unpack.c), so only
                        // the keys may be passed — anything else would be a
                        // caller bug whose garbage differs between libraries.
                        let ret = if flags & JSON_VALIDATE_ONLY != 0 {
                            if keyed2 {
                                (api.json_unpack_ex)(
                                    r,
                                    &mut e,
                                    flags,
                                    zf.as_ptr(),
                                    ka.as_ptr(),
                                    kb.as_ptr(),
                                )
                            } else if keyed1 {
                                (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr(), ka.as_ptr())
                            } else {
                                (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr())
                            }
                        } else {
                        // supply enough out-params for any of the formats above
                        match fmt {
                            "n" | "[n]" | "[]" | "{}" | "[!]" | "[*]" | "q" | "[q]"
                            | "[i," | "{s:i" => (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr()),
                            "b" => (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr(), &mut iv),
                            "i" => (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr(), &mut iv),
                            "I" => (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr(), &mut iiv),
                            "f" | "F" => {
                                (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr(), &mut fv)
                            }
                            "s" => (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr(), &mut sptr),
                            "s%" => (api.json_unpack_ex)(
                                r,
                                &mut e,
                                flags,
                                zf.as_ptr(),
                                &mut sptr,
                                &mut slen,
                            ),
                            "o" | "O" => {
                                (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr(), &mut ov)
                            }
                            "[i]" | "[i!]" | "[i*]" => {
                                (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr(), &mut iv)
                            }
                            "[i,i]" | "[i!i]" | "[i*i]" => (api.json_unpack_ex)(
                                r,
                                &mut e,
                                flags,
                                zf.as_ptr(),
                                &mut iv,
                                &mut iiv,
                            ),
                            "[i,i,i]" => {
                                let mut i3: c_int = 0;
                                (api.json_unpack_ex)(
                                    r,
                                    &mut e,
                                    flags,
                                    zf.as_ptr(),
                                    &mut iv,
                                    &mut iiv,
                                    &mut i3,
                                )
                            }
                            "[s]" => (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr(), &mut sptr),
                            "{s:i}" | "{s:i!}" | "{s:i*}" | "{s?i}" => (api.json_unpack_ex)(
                                r,
                                &mut e,
                                flags,
                                zf.as_ptr(),
                                ka.as_ptr(),
                                &mut iv,
                            ),
                            "{s:i,s:i}" | "{s:i,s?i}" | "{s:i!s:i}" => (api.json_unpack_ex)(
                                r,
                                &mut e,
                                flags,
                                zf.as_ptr(),
                                ka.as_ptr(),
                                &mut iv,
                                kb.as_ptr(),
                                &mut iiv,
                            ),
                            "{s:s}" => (api.json_unpack_ex)(
                                r,
                                &mut e,
                                flags,
                                zf.as_ptr(),
                                ka.as_ptr(),
                                &mut sptr,
                            ),
                            "{s:o}" | "{s:q}" => (api.json_unpack_ex)(
                                r,
                                &mut e,
                                flags,
                                zf.as_ptr(),
                                ka.as_ptr(),
                                &mut ov,
                            ),
                            _ => (api.json_unpack_ex)(r, &mut e, flags, zf.as_ptr()),
                        }
                        };
                        obs.push(U {
                            ret,
                            err: e.snapshot(),
                            sptr: if sptr.is_null() {
                                None
                            } else {
                                Some(std::ffi::CStr::from_ptr(sptr).to_bytes().to_vec())
                            },
                            slen,
                            i: iv,
                            ii: iiv,
                            f: fv.to_bits(),
                            obj: !ov.is_null(),
                        });
                        decref(api, r);
                    }
                    assert_eq!(
                        obs[0], obs[1],
                        "json_unpack_ex root={root:?} fmt={fmt:?} flags={flags:#x}"
                    );
                }
            }
        }
    }
}

#[test]
fn e8_json_unpack_plain_and_null_args() {
    let _g = lock();
    let p = pair();
    unsafe {
        // json_unpack (no error struct, flags 0)
        for (root, fmt) in [
            ("[1]", "[i]"),
            ("{\"a\":1}", "{s:i}"),
            ("\"x\"", "s"),
            ("null", "n"),
            ("1.5", "f"),
            ("[1]", "{s:i}"),
        ] {
            let mut rets = Vec::new();
            for api in [p.c, p.r] {
                let r = (api.json_loads)(cstr(root).as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
                let zf = cstr(fmt);
                let ka = cstr("a");
                let mut iv: c_int = -1;
                let mut sp: *const c_char = std::ptr::null();
                let mut fv: f64 = -1.0;
                let ret = match fmt {
                    "[i]" => (api.json_unpack)(r, zf.as_ptr(), &mut iv),
                    "{s:i}" => (api.json_unpack)(r, zf.as_ptr(), ka.as_ptr(), &mut iv),
                    "s" => (api.json_unpack)(r, zf.as_ptr(), &mut sp),
                    "f" => (api.json_unpack)(r, zf.as_ptr(), &mut fv),
                    _ => (api.json_unpack)(r, zf.as_ptr()),
                };
                rets.push((ret, iv, fv.to_bits(), sp.is_null()));
                decref(api, r);
            }
            assert_eq!(rets[0], rets[1], "json_unpack {root} {fmt}");
        }
        // NULL string target / NULL length target / NULL key
        let mut rets = Vec::new();
        for api in [p.c, p.r] {
            let r = (api.json_loads)(cstr("\"s\"").as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
            let mut e = JsonError::zeroed();
            let a = (api.json_unpack_ex)(r, &mut e, 0usize, cstr("s").as_ptr(), std::ptr::null_mut::<*const c_char>());
            let e1 = e.snapshot();
            let mut e = JsonError::zeroed();
            let mut sp: *const c_char = std::ptr::null();
            let b = (api.json_unpack_ex)(
                r,
                &mut e,
                0usize,
                cstr("s%").as_ptr(),
                &mut sp,
                std::ptr::null_mut::<usize>(),
            );
            let e2 = e.snapshot();
            decref(api, r);
            let o = (api.json_loads)(cstr("{\"a\":1}").as_ptr(), 0, std::ptr::null_mut());
            let mut e = JsonError::zeroed();
            let mut iv: c_int = 0;
            let c = (api.json_unpack_ex)(
                o,
                &mut e,
                0usize,
                cstr("{s:i}").as_ptr(),
                std::ptr::null::<c_char>(),
                &mut iv,
            );
            let e3 = e.snapshot();
            decref(api, o);
            rets.push((a, e1, b, e2, c, e3));
        }
        assert_eq!(rets[0], rets[1], "unpack NULL-argument handling");
    }
}

#[test]
fn e14_vunpack_ex_through_shim() {
    let _g = lock();
    let p = pair();
    let sh = vshim();
    unsafe {
        let ca = p.c.json_vunpack_ex as usize as *mut c_void;
        let ra = p.r.json_vunpack_ex as usize as *mut c_void;
        for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT] {
            for (root, fmt, kind) in [
                ("null", "n", 0),
                ("[1]", "[i]", 1),
                ("[1,2]", "[i,i]", 2),
                ("{\"a\":1}", "{s:i}", 3),
                ("{\"a\":1,\"b\":2}", "{s:i,s:i}", 4),
                ("\"str\"", "s", 5),
                ("[1,2,3]", "[i,i]", 2),
                ("{\"a\":1}", "{s:i!}", 3),
                ("bad", "x", 0),
            ] {
                if flags & JSON_VALIDATE_ONLY != 0 && !matches!(kind, 0 | 3 | 4) {
                    // value specifiers consume no varargs under VALIDATE_ONLY
                    continue;
                }
                let mut res = Vec::new();
                for (api, addr) in [(p.c, ca), (p.r, ra)] {
                    let r =
                        (api.json_loads)(cstr(root).as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
                    let zf = cstr(fmt);
                    let ka = cstr("a");
                    let kb = cstr("b");
                    let mut e = JsonError::zeroed();
                    let mut a1: i64 = -1;
                    let mut a2: i64 = -1;
                    let mut sp: *const c_char = std::ptr::null();
                    let vo = flags & JSON_VALIDATE_ONLY != 0;
                    let ret = match kind {
                        0 => (sh.vunpack_0)(addr, r, &mut e, flags, zf.as_ptr()),
                        1 => (sh.vunpack_p)(
                            addr,
                            r,
                            &mut e,
                            flags,
                            zf.as_ptr(),
                            &mut a1 as *mut i64 as *mut c_void,
                        ),
                        2 => (sh.vunpack_pp)(
                            addr,
                            r,
                            &mut e,
                            flags,
                            zf.as_ptr(),
                            &mut a1 as *mut i64 as *mut c_void,
                            &mut a2 as *mut i64 as *mut c_void,
                        ),
                        3 => {
                            if vo {
                                (sh.vunpack_p)(
                                    addr,
                                    r,
                                    &mut e,
                                    flags,
                                    zf.as_ptr(),
                                    ka.as_ptr() as *mut c_void,
                                )
                            } else {
                                (sh.vunpack_sp)(
                                    addr,
                                    r,
                                    &mut e,
                                    flags,
                                    zf.as_ptr(),
                                    ka.as_ptr(),
                                    &mut a1 as *mut i64 as *mut c_void,
                                )
                            }
                        }
                        4 => {
                            if vo {
                                (sh.vunpack_pp)(
                                    addr,
                                    r,
                                    &mut e,
                                    flags,
                                    zf.as_ptr(),
                                    ka.as_ptr() as *mut c_void,
                                    kb.as_ptr() as *mut c_void,
                                )
                            } else {
                                (sh.vunpack_spsp)(
                                    addr,
                                    r,
                                    &mut e,
                                    flags,
                                    zf.as_ptr(),
                                    ka.as_ptr(),
                                    &mut a1 as *mut i64 as *mut c_void,
                                    kb.as_ptr(),
                                    &mut a2 as *mut i64 as *mut c_void,
                                )
                            }
                        }
                        // string out-param: compare CONTENT, not the address
                        _ => (sh.vunpack_p)(
                            addr,
                            r,
                            &mut e,
                            flags,
                            zf.as_ptr(),
                            &mut sp as *mut *const c_char as *mut c_void,
                        ),
                    };
                    let sval = if sp.is_null() {
                        None
                    } else {
                        Some(std::ffi::CStr::from_ptr(sp).to_bytes().to_vec())
                    };
                    res.push((ret, e.snapshot(), a1, a2, sval));
                    decref(api, r);
                }
                assert_eq!(
                    res[0], res[1],
                    "json_vunpack_ex root={root} fmt={fmt} flags={flags:#x}"
                );
            }
            // NULL root and NULL/empty format
            let mut res = Vec::new();
            for (_api, addr) in [(p.c, ca), (p.r, ra)] {
                let mut e1 = JsonError::zeroed();
                let r1 = (sh.vunpack_0)(addr, std::ptr::null_mut(), &mut e1, flags, cstr("n").as_ptr());
                let mut e2 = JsonError::zeroed();
                let r2 = (sh.vunpack_0)(addr, std::ptr::null_mut(), &mut e2, flags, std::ptr::null());
                res.push((r1, e1.snapshot(), r2, e2.snapshot()));
            }
            assert_eq!(res[0], res[1], "vunpack_ex NULL root/format");
        }
    }
}

/* ===================== E15: full round trip ===================== */

#[test]
fn e15_pack_dump_load_unpack_round_trip() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0xE15);
    unsafe {
        for _ in 0..400 {
            let i1 = rng.range(-1_000_000, 1_000_000) as c_int;
            let i2 = rng.i64();
            let s1 = rng.ascii_string(12);
            let s2 = rng.spicy_string(8);
            let dv = rng.tame_f64();
            let zk1 = cstr("num");
            let zk2 = cstr("big");
            let zk3 = cstr("txt");
            let zs1 = cstr(&s1);
            let zs2 = cstr(&s2);
            let zf = cstr("{s:i,s:I,s:[s,s,f]}");
            let mut outs = Vec::new();
            for api in [p.c, p.r] {
                let j = (api.json_pack)(
                    zf.as_ptr(),
                    zk1.as_ptr(),
                    i1,
                    zk2.as_ptr(),
                    i2,
                    zk3.as_ptr(),
                    zs1.as_ptr(),
                    zs2.as_ptr(),
                    dv,
                );
                let dumped = dumps(api, j, JSON_SORT_KEYS);
                let mut reloaded_dump = None;
                let mut unpacked = (0i32, -1i32, -1i64);
                if let Some(bytes) = &dumped {
                    let z = nul_terminated(bytes);
                    let j2 = (api.json_loads)(z.as_ptr(), 0, std::ptr::null_mut());
                    reloaded_dump = dumps(api, j2, JSON_SORT_KEYS);
                    let mut a: c_int = -1;
                    let mut b: i64 = -1;
                    let ret = (api.json_unpack)(
                        j2,
                        cstr("{s:i,s:I,s:[s,s,f]}").as_ptr(),
                        zk1.as_ptr(),
                        &mut a,
                        zk2.as_ptr(),
                        &mut b,
                        zk3.as_ptr(),
                        &mut std::ptr::null::<c_char>(),
                        &mut std::ptr::null::<c_char>(),
                        &mut 0f64,
                    );
                    unpacked = (ret, a, b);
                    decref(api, j2);
                }
                outs.push((dumped, reloaded_dump, unpacked));
                decref(api, j);
            }
            assert_eq!(outs[0], outs[1], "pack round trip i1={i1} s1={s1:?}");
        }
    }
}
