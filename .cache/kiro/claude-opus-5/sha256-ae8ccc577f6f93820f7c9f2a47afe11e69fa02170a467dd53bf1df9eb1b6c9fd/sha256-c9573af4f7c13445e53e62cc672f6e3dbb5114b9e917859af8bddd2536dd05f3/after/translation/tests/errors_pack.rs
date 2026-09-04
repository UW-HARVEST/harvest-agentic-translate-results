//! Phase C — ERRORS.md rows 206–251 (`pack_unpack.c`).
//!
//! Each row constructs its exact trigger, calls BOTH libraries, and asserts the
//! same return value AND the same error struct (code, line, column, position,
//! source, text).
mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_void};

/* ================= rows 206..224: pack ================= */

#[test]
fn e_rows_206_212_pack_format_errors() {
    let _g = lock();
    let p = pair();
    let key = cstr("k");
    let sv = cstr("v");
    // (row, fmt-or-NULL, expected code)
    let cases: Vec<(u32, Option<&str>, u8)> = vec![
        (206, None, E_INVALID_ARGUMENT),
        (207, Some(""), E_INVALID_ARGUMENT),
        (208, Some("[]x"), E_INVALID_FORMAT),
        (208, Some("{}{}"), E_INVALID_FORMAT),
        (209, Some("q"), E_INVALID_FORMAT),
        (209, Some("]"), E_INVALID_FORMAT),
        (209, Some("}"), E_INVALID_FORMAT),
        (209, Some("*"), E_INVALID_FORMAT),
        (209, Some("#"), E_INVALID_FORMAT),
        (209, Some("%"), E_INVALID_FORMAT),
        (209, Some("+"), E_INVALID_FORMAT),
        (209, Some("!"), E_INVALID_FORMAT),
        (209, Some("?"), E_INVALID_FORMAT),
        (209, Some("[q]"), E_INVALID_FORMAT),
        (210, Some("{"), E_INVALID_FORMAT),
        (211, Some("{i:i}"), E_INVALID_FORMAT),
        (211, Some("{n:n}"), E_INVALID_FORMAT),
        (211, Some("{[]:i}"), E_INVALID_FORMAT),
        (212, Some("["), E_INVALID_FORMAT),
        (212, Some("[[["), E_INVALID_FORMAT),
    ];
    unsafe {
        for (row, fmt, expect) in cases {
            let z = fmt.map(cstr);
            let fp: *const c_char = z.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let a = (p.c.json_pack_ex)(&mut ec, 0usize, fp, key.as_ptr(), sv.as_ptr(), 1i32);
            let b = (p.r.json_pack_ex)(&mut er, 0usize, fp, key.as_ptr(), sv.as_ptr(), 1i32);
            assert!(a.is_null(), "row {row} fmt={fmt:?}: C unexpectedly succeeded");
            assert!(b.is_null(), "row {row} fmt={fmt:?}: Rust unexpectedly succeeded");
            assert_eq!(
                ec.code(),
                expect,
                "row {row} fmt={fmt:?}: C code {} != documented {expect} (text={:?})",
                ec.code(),
                ec.text_str()
            );
            assert_eq!(
                ec.snapshot(),
                er.snapshot(),
                "row {row} fmt={fmt:?}: error struct differs\n C   text={:?} src={:?} l={} c={} p={}\n Rust text={:?} src={:?} l={} c={} p={}",
                ec.text_str(),
                ec.source_str(),
                ec.line,
                ec.column,
                ec.position,
                er.text_str(),
                er.source_str(),
                er.line,
                er.column,
                er.position
            );
        }
        // NULL error struct on every failing form must not crash
        for fmt in ["", "q", "{", "["] {
            let z = cstr(fmt);
            assert!((p.c.json_pack)(z.as_ptr()).is_null());
            assert!((p.r.json_pack)(z.as_ptr()).is_null());
        }
        assert!((p.c.json_pack)(std::ptr::null()).is_null());
        assert!((p.r.json_pack)(std::ptr::null()).is_null());
    }
}

#[test]
fn e_rows_213_218_pack_string_errors() {
    let _g = lock();
    let p = pair();
    unsafe {
        let good = cstr("good");
        // row 213: NULL string arg, non-optional
        for fmt in ["s", "[s]", "{s:i}", "[s,s]"] {
            let z = cstr(fmt);
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let (a, b) = if fmt == "{s:i}" {
                (
                    (p.c.json_pack_ex)(&mut ec, 0usize, z.as_ptr(), std::ptr::null::<c_char>(), 1i32),
                    (p.r.json_pack_ex)(&mut er, 0usize, z.as_ptr(), std::ptr::null::<c_char>(), 1i32),
                )
            } else {
                (
                    (p.c.json_pack_ex)(
                        &mut ec,
                        0usize,
                        z.as_ptr(),
                        std::ptr::null::<c_char>(),
                        good.as_ptr(),
                    ),
                    (p.r.json_pack_ex)(
                        &mut er,
                        0usize,
                        z.as_ptr(),
                        std::ptr::null::<c_char>(),
                        good.as_ptr(),
                    ),
                )
            };
            assert!(a.is_null() && b.is_null(), "row 213 fmt={fmt}");
            assert_eq!(ec.code(), E_NULL_VALUE, "row 213 fmt={fmt} text={:?}", ec.text_str());
            assert_eq!(ec.snapshot(), er.snapshot(), "row 213 fmt={fmt}");
        }
        // row 214: invalid UTF-8 string arg
        for badb in [&b"\xff"[..], &b"\xc2"[..], &b"\xed\xa0\x80"[..], &b"a\xffb"[..]] {
            let z = nul_terminated(badb);
            for fmt in ["s", "[s]", "{s:i}"] {
                let zf = cstr(fmt);
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let a = (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), z.as_ptr(), 1i32);
                let b = (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), z.as_ptr(), 1i32);
                assert!(a.is_null() && b.is_null(), "row 214 {badb:?} fmt={fmt}");
                assert_eq!(ec.code(), E_INVALID_UTF8, "row 214 {badb:?} fmt={fmt}");
                assert_eq!(ec.snapshot(), er.snapshot(), "row 214 {badb:?} fmt={fmt}");
            }
        }
        // row 215: '#'/'%'/'+' on an optional string
        for fmt in ["s?#", "s?%", "s?+", "s*#", "s*%", "s*+"] {
            let zf = cstr(fmt);
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let a = (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), good.as_ptr(), 2i32);
            let b = (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), good.as_ptr(), 2i32);
            assert!(a.is_null() && b.is_null(), "row 215 fmt={fmt}");
            assert_eq!(
                ec.code(),
                E_INVALID_FORMAT,
                "row 215 fmt={fmt} text={:?}",
                ec.text_str()
            );
            assert_eq!(ec.snapshot(), er.snapshot(), "row 215 fmt={fmt}");
        }
        // row 216: NULL in a concatenation chain (first / second position)
        for (a1, a2) in [
            (std::ptr::null::<c_char>(), good.as_ptr()),
            (good.as_ptr(), std::ptr::null::<c_char>()),
            (std::ptr::null::<c_char>(), std::ptr::null::<c_char>()),
        ] {
            let zf = cstr("s+");
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let a = (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), a1, a2);
            let b = (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), a1, a2);
            assert!(a.is_null() && b.is_null(), "row 216");
            assert_eq!(ec.code(), E_NULL_VALUE, "row 216 text={:?}", ec.text_str());
            assert_eq!(ec.snapshot(), er.snapshot(), "row 216");
        }
        // row 217: concatenation producing invalid UTF-8 (split a 2-byte char)
        let half1 = nul_terminated(b"\xc3");
        let half2 = nul_terminated(b"\xa9");
        let zf = cstr("s+");
        let mut ec = JsonError::zeroed();
        let mut er = JsonError::zeroed();
        let a = (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), half1.as_ptr(), half2.as_ptr());
        let b = (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), half1.as_ptr(), half2.as_ptr());
        // the concatenation is actually valid UTF-8 ("é") — both must agree
        assert_eq!(dumps(p.c, a, JSON_ENCODE_ANY), dumps(p.r, b, JSON_ENCODE_ANY));
        assert_eq!(ec.snapshot(), er.snapshot());
        decref(p.c, a);
        decref(p.r, b);
        // now make the concatenation genuinely invalid
        let bad2 = nul_terminated(b"\x41");
        let mut ec = JsonError::zeroed();
        let mut er = JsonError::zeroed();
        let a = (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), half1.as_ptr(), bad2.as_ptr());
        let b = (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), half1.as_ptr(), bad2.as_ptr());
        assert!(a.is_null() && b.is_null(), "row 217");
        assert_eq!(ec.code(), E_INVALID_UTF8, "row 217 text={:?}", ec.text_str());
        assert_eq!(ec.snapshot(), er.snapshot(), "row 217");
    }
}

#[test]
fn e_rows_219_221_pack_value_errors() {
    let _g = lock();
    let p = pair();
    unsafe {
        let key = cstr("k");
        // row 219: NULL json_t for o / O without ? or *
        for fmt in ["o", "O", "[o]", "[O]", "{s:o}", "{s:O}"] {
            let zf = cstr(fmt);
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let (a, b) = if fmt.starts_with('{') {
                (
                    (p.c.json_pack_ex)(
                        &mut ec,
                        0usize,
                        zf.as_ptr(),
                        key.as_ptr(),
                        std::ptr::null_mut::<JsonT>(),
                    ),
                    (p.r.json_pack_ex)(
                        &mut er,
                        0usize,
                        zf.as_ptr(),
                        key.as_ptr(),
                        std::ptr::null_mut::<JsonT>(),
                    ),
                )
            } else {
                (
                    (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), std::ptr::null_mut::<JsonT>()),
                    (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), std::ptr::null_mut::<JsonT>()),
                )
            };
            assert!(a.is_null() && b.is_null(), "row 219 fmt={fmt}");
            assert_eq!(ec.code(), E_NULL_VALUE, "row 219 fmt={fmt} text={:?}", ec.text_str());
            assert_eq!(ec.snapshot(), er.snapshot(), "row 219 fmt={fmt}");
        }
        // row 220: NULL object value in {s:o} where o has no '*'
        let zf = cstr("{s:o,s:i}");
        let mut ec = JsonError::zeroed();
        let mut er = JsonError::zeroed();
        let a = (p.c.json_pack_ex)(
            &mut ec,
            0usize,
            zf.as_ptr(),
            key.as_ptr(),
            std::ptr::null_mut::<JsonT>(),
            cstr("k2").as_ptr(),
            5i32,
        );
        let b = (p.r.json_pack_ex)(
            &mut er,
            0usize,
            zf.as_ptr(),
            key.as_ptr(),
            std::ptr::null_mut::<JsonT>(),
            cstr("k2").as_ptr(),
            5i32,
        );
        assert!(a.is_null() && b.is_null(), "row 220");
        assert_eq!(ec.snapshot(), er.snapshot(), "row 220");
        // row 221: non-finite double
        for v in [f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for fmt in ["f", "[f]", "{s:f}"] {
                let zf = cstr(fmt);
                let mut ec = JsonError::zeroed();
                let mut er = JsonError::zeroed();
                let (a, b) = if fmt.starts_with('{') {
                    (
                        (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), key.as_ptr(), v),
                        (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), key.as_ptr(), v),
                    )
                } else {
                    (
                        (p.c.json_pack_ex)(&mut ec, 0usize, zf.as_ptr(), v),
                        (p.r.json_pack_ex)(&mut er, 0usize, zf.as_ptr(), v),
                    )
                };
                assert!(a.is_null() && b.is_null(), "row 221 {v:?} fmt={fmt}");
                assert_eq!(
                    ec.code(),
                    E_NUMERIC_OVERFLOW,
                    "row 221 {v:?} fmt={fmt} text={:?}",
                    ec.text_str()
                );
                assert_eq!(ec.snapshot(), er.snapshot(), "row 221 {v:?} fmt={fmt}");
            }
        }
    }
}

/* ================= rows 225..251: unpack ================= */

#[test]
fn e_rows_225_229_unpack_argument_errors() {
    let _g = lock();
    let p = pair();
    unsafe {
        // row 225: NULL root
        for flags in [0usize, JSON_VALIDATE_ONLY, JSON_STRICT] {
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let a = (p.c.json_unpack_ex)(std::ptr::null_mut(), &mut ec, flags, cstr("n").as_ptr());
            let b = (p.r.json_unpack_ex)(std::ptr::null_mut(), &mut er, flags, cstr("n").as_ptr());
            assert_eq!(a, -1);
            assert_eq!(b, -1);
            assert_eq!(ec.code(), E_NULL_VALUE);
            assert_eq!(ec.snapshot(), er.snapshot(), "row 225 flags={flags}");
            assert_eq!(ec.source_str(), "<root>");
        }
        // rows 226,227: NULL / empty format
        let roots = ["null", "[]", "{}", "1"];
        for root in roots {
            for fmt in [None, Some("")] {
                let z = fmt.map(cstr);
                let fp: *const c_char = z.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
                let mut res = Vec::new();
                for api in [p.c, p.r] {
                    let r = (api.json_loads)(
                        cstr(root).as_ptr(),
                        JSON_DECODE_ANY,
                        std::ptr::null_mut(),
                    );
                    let mut e = JsonError::zeroed();
                    let ret = (api.json_unpack_ex)(r, &mut e, 0usize, fp);
                    res.push((ret, e.snapshot(), e.code()));
                    decref(api, r);
                }
                assert_eq!(res[0].0, -1);
                assert_eq!(res[0].2, E_INVALID_ARGUMENT);
                assert_eq!(res[0], res[1], "rows 226/227 root={root} fmt={fmt:?}");
            }
        }
        // row 228: garbage after format
        for (root, fmt) in [("[]", "[]x"), ("{}", "{}}"), ("null", "nn"), ("[1]", "[i]!")] {
            let mut res = Vec::new();
            for api in [p.c, p.r] {
                let r = (api.json_loads)(cstr(root).as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
                let mut e = JsonError::zeroed();
                let mut iv: c_int = 0;
                let ret = (api.json_unpack_ex)(r, &mut e, 0usize, cstr(fmt).as_ptr(), &mut iv);
                res.push((ret, e.snapshot(), e.code()));
                decref(api, r);
            }
            assert_eq!(res[0].0, -1, "row 228 root={root} fmt={fmt}");
            assert_eq!(res[0], res[1], "row 228 root={root} fmt={fmt}");
        }
        // row 229: unknown format char at top level
        for fmt in ["q", "x", "#", "%", "+", "?", "!", "]", "}"] {
            let mut res = Vec::new();
            for api in [p.c, p.r] {
                let r = (api.json_loads)(cstr("null").as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
                let mut e = JsonError::zeroed();
                let ret = (api.json_unpack_ex)(r, &mut e, 0usize, cstr(fmt).as_ptr());
                res.push((ret, e.snapshot(), e.code()));
                decref(api, r);
            }
            assert_eq!(res[0].0, -1, "row 229 fmt={fmt}");
            assert_eq!(res[0].2, E_INVALID_FORMAT, "row 229 fmt={fmt}");
            assert_eq!(res[0], res[1], "row 229 fmt={fmt}");
        }
    }
}

#[test]
fn e_rows_230_243_unpack_container_errors() {
    let _g = lock();
    let p = pair();
    // (row, root, fmt, n_keys, expected code)
    let cases: Vec<(u32, &str, &str, usize, u8)> = vec![
        (230, "[]", "{}", 0, E_WRONG_TYPE),
        (230, "1", "{}", 0, E_WRONG_TYPE),
        (230, "\"s\"", "{}", 0, E_WRONG_TYPE),
        (230, "null", "{}", 0, E_WRONG_TYPE),
        (230, "true", "{}", 0, E_WRONG_TYPE),
        (230, "1.5", "{}", 0, E_WRONG_TYPE),
        (231, "{\"a\":1}", "{!s}", 1, E_INVALID_FORMAT),
        (231, "{\"a\":1}", "{*s}", 1, E_INVALID_FORMAT),
        (232, "{}", "{", 0, E_INVALID_FORMAT),
        (232, "{\"a\":1}", "{s", 1, E_INVALID_FORMAT),
        (233, "{}", "{i}", 0, E_INVALID_FORMAT),
        (233, "{}", "{n}", 0, E_INVALID_FORMAT),
        (235, "{}", "{s:i}", 1, E_ITEM_NOT_FOUND),
        (235, "{\"b\":1}", "{s:i}", 1, E_ITEM_NOT_FOUND),
        (236, "{\"a\":1,\"b\":2}", "{s:i!}", 1, E_END_OF_INPUT_EXPECTED),
        (238, "{}", "[]", 0, E_WRONG_TYPE),
        (238, "1", "[]", 0, E_WRONG_TYPE),
        (238, "\"s\"", "[]", 0, E_WRONG_TYPE),
        (238, "null", "[]", 0, E_WRONG_TYPE),
        (239, "[1]", "[!i]", 0, E_INVALID_FORMAT),
        (239, "[1]", "[*i]", 0, E_INVALID_FORMAT),
        (240, "[]", "[", 0, E_INVALID_FORMAT),
        (241, "[1]", "[q]", 0, E_INVALID_FORMAT),
        (241, "[1]", "[#]", 0, E_INVALID_FORMAT),
        (241, "[1]", "[%]", 0, E_INVALID_FORMAT),
        (241, "[1]", "[?]", 0, E_INVALID_FORMAT),
        (242, "[]", "[i]", 0, E_INDEX_OUT_OF_RANGE),
        (242, "[1]", "[i,i]", 0, E_INDEX_OUT_OF_RANGE),
        (243, "[1,2]", "[i!]", 0, E_END_OF_INPUT_EXPECTED),
    ];
    unsafe {
        for (row, root, fmt, nkeys, expect) in cases {
            let mut res = Vec::new();
            for api in [p.c, p.r] {
                let r = (api.json_loads)(cstr(root).as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
                let zf = cstr(fmt);
                let ka = cstr("a");
                let kb = cstr("b");
                let mut e = JsonError::zeroed();
                let mut i1: c_int = 0;
                let mut i2: c_int = 0;
                let ret = match nkeys {
                    0 => (api.json_unpack_ex)(r, &mut e, 0usize, zf.as_ptr(), &mut i1, &mut i2),
                    1 => (api.json_unpack_ex)(
                        r,
                        &mut e,
                        0usize,
                        zf.as_ptr(),
                        ka.as_ptr(),
                        &mut i1,
                    ),
                    _ => (api.json_unpack_ex)(
                        r,
                        &mut e,
                        0usize,
                        zf.as_ptr(),
                        ka.as_ptr(),
                        &mut i1,
                        kb.as_ptr(),
                        &mut i2,
                    ),
                };
                res.push((ret, e.snapshot(), e.code(), e.text_str()));
                decref(api, r);
            }
            assert_eq!(res[0].0, -1, "row {row} root={root} fmt={fmt}: C succeeded");
            assert_eq!(
                res[0].2, expect,
                "row {row} root={root} fmt={fmt}: C code {} != documented {expect} (text={:?})",
                res[0].2, res[0].3
            );
            assert_eq!(
                res[0], res[1],
                "row {row} root={root} fmt={fmt}: C text={:?} Rust text={:?}",
                res[0].3, res[1].3
            );
        }
        // row 234: NULL object key
        for fmt in ["{s:i}", "{s?i}", "{s:i,s:i}"] {
            let mut res = Vec::new();
            for api in [p.c, p.r] {
                let r = (api.json_loads)(
                    cstr("{\"a\":1,\"b\":2}").as_ptr(),
                    0,
                    std::ptr::null_mut(),
                );
                let mut e = JsonError::zeroed();
                let mut i1: c_int = 0;
                let ret = (api.json_unpack_ex)(
                    r,
                    &mut e,
                    0usize,
                    cstr(fmt).as_ptr(),
                    std::ptr::null::<c_char>(),
                    &mut i1,
                );
                res.push((ret, e.snapshot(), e.code()));
                decref(api, r);
            }
            assert_eq!(res[0].0, -1);
            assert_eq!(res[0].2, E_NULL_VALUE, "row 234 fmt={fmt}");
            assert_eq!(res[0], res[1], "row 234 fmt={fmt}");
        }
        // row 236/243 via JSON_STRICT rather than '!'
        for (root, fmt, keys) in [
            ("{\"a\":1,\"b\":2}", "{s:i}", 1usize),
            ("{\"a\":1,\"b\":2,\"c\":3}", "{s:i}", 1),
            ("[1,2]", "[i]", 0),
            ("[1,2,3]", "[i,i]", 0),
        ] {
            let mut res = Vec::new();
            for api in [p.c, p.r] {
                let r = (api.json_loads)(cstr(root).as_ptr(), 0, std::ptr::null_mut());
                let ka = cstr("a");
                let mut e = JsonError::zeroed();
                let mut i1: c_int = 0;
                let mut i2: c_int = 0;
                let ret = if keys == 1 {
                    (api.json_unpack_ex)(
                        r,
                        &mut e,
                        JSON_STRICT,
                        cstr(fmt).as_ptr(),
                        ka.as_ptr(),
                        &mut i1,
                    )
                } else {
                    (api.json_unpack_ex)(
                        r,
                        &mut e,
                        JSON_STRICT,
                        cstr(fmt).as_ptr(),
                        &mut i1,
                        &mut i2,
                    )
                };
                res.push((ret, e.snapshot(), e.code(), e.text_str()));
                decref(api, r);
            }
            assert_eq!(res[0].0, -1, "STRICT root={root} fmt={fmt}");
            assert_eq!(res[0].2, E_END_OF_INPUT_EXPECTED, "STRICT root={root} fmt={fmt}");
            assert_eq!(
                res[0], res[1],
                "STRICT root={root} fmt={fmt}: C={:?} Rust={:?}",
                res[0].3, res[1].3
            );
        }
        // the "unrecognized keys" message with many extra keys and with s? present
        for (root, fmt) in [
            ("{\"a\":1,\"zz\":2,\"yy\":3,\"xx\":4}", "{s:i}"),
            ("{\"a\":1,\"b\":2}", "{s?i}"),
            ("{\"a\":1,\"b\":2,\"c\":3}", "{s?i}"),
        ] {
            let mut res = Vec::new();
            for api in [p.c, p.r] {
                let r = (api.json_loads)(cstr(root).as_ptr(), 0, std::ptr::null_mut());
                let ka = cstr("a");
                let mut e = JsonError::zeroed();
                let mut i1: c_int = 0;
                let ret = (api.json_unpack_ex)(
                    r,
                    &mut e,
                    JSON_STRICT,
                    cstr(fmt).as_ptr(),
                    ka.as_ptr(),
                    &mut i1,
                );
                res.push((ret, e.snapshot(), e.text_str()));
                decref(api, r);
            }
            assert_eq!(
                res[0], res[1],
                "unrecognized-keys message root={root} fmt={fmt}: C={:?} Rust={:?}",
                res[0].2, res[1].2
            );
        }
    }
}

#[test]
fn e_rows_244_251_unpack_type_errors() {
    let _g = lock();
    let p = pair();
    let roots = [
        "null", "true", "false", "0", "42", "-1", "1.5", "\"str\"", "[]", "[1]", "{}",
        "{\"a\":1}",
    ];
    let specs = ["s", "i", "I", "b", "f", "F", "n", "o", "O", "s%"];
    unsafe {
        for root in roots {
            for spec in specs {
                let mut res = Vec::new();
                for api in [p.c, p.r] {
                    let r = (api.json_loads)(
                        cstr(root).as_ptr(),
                        JSON_DECODE_ANY,
                        std::ptr::null_mut(),
                    );
                    let mut e = JsonError::zeroed();
                    let mut sp: *const c_char = std::ptr::null();
                    let mut sl: usize = 0xdead;
                    let mut iv: c_int = -1;
                    let mut ll: i64 = -1;
                    let mut dv: f64 = -1.0;
                    let mut ov: Jt = std::ptr::null_mut();
                    let ret = match spec {
                        "s" => (api.json_unpack_ex)(r, &mut e, 0usize, cstr(spec).as_ptr(), &mut sp),
                        "s%" => (api.json_unpack_ex)(
                            r,
                            &mut e,
                            0usize,
                            cstr(spec).as_ptr(),
                            &mut sp,
                            &mut sl,
                        ),
                        "i" | "b" => {
                            (api.json_unpack_ex)(r, &mut e, 0usize, cstr(spec).as_ptr(), &mut iv)
                        }
                        "I" => (api.json_unpack_ex)(r, &mut e, 0usize, cstr(spec).as_ptr(), &mut ll),
                        "f" | "F" => {
                            (api.json_unpack_ex)(r, &mut e, 0usize, cstr(spec).as_ptr(), &mut dv)
                        }
                        "n" => (api.json_unpack_ex)(r, &mut e, 0usize, cstr(spec).as_ptr()),
                        _ => (api.json_unpack_ex)(r, &mut e, 0usize, cstr(spec).as_ptr(), &mut ov),
                    };
                    let sval = if sp.is_null() {
                        None
                    } else {
                        Some(std::ffi::CStr::from_ptr(sp).to_bytes().to_vec())
                    };
                    res.push((ret, e.snapshot(), sval, sl, iv, ll, dv.to_bits(), !ov.is_null()));
                    decref(api, r);
                }
                assert_eq!(
                    res[0], res[1],
                    "unpack root={root} spec={spec}: C text={:?} Rust text={:?}",
                    JsonError {
                        line: 0,
                        column: 0,
                        position: 0,
                        source: [0; 80],
                        text: [0; 160]
                    }
                    .text_str(),
                    ""
                );
            }
        }
        // row 245: NULL string target; row 246: NULL length target
        let mut res = Vec::new();
        for api in [p.c, p.r] {
            let r = (api.json_loads)(cstr("\"s\"").as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
            let mut e1 = JsonError::zeroed();
            let a = (api.json_unpack_ex)(
                r,
                &mut e1,
                0usize,
                cstr("s").as_ptr(),
                std::ptr::null_mut::<*const c_char>(),
            );
            let mut e2 = JsonError::zeroed();
            let mut sp: *const c_char = std::ptr::null();
            let b = (api.json_unpack_ex)(
                r,
                &mut e2,
                0usize,
                cstr("s%").as_ptr(),
                &mut sp,
                std::ptr::null_mut::<usize>(),
            );
            res.push((a, e1.snapshot(), e1.code(), b, e2.snapshot(), e2.code()));
            decref(api, r);
        }
        assert_eq!(res[0].0, -1);
        assert_eq!(res[0].2, E_NULL_VALUE, "row 245");
        assert_eq!(res[0].3, -1);
        assert_eq!(res[0].5, E_NULL_VALUE, "row 246");
        assert_eq!(res[0], res[1], "rows 245/246");
    }
}

#[allow(unused)]
fn _u(_: *mut c_void) {}
