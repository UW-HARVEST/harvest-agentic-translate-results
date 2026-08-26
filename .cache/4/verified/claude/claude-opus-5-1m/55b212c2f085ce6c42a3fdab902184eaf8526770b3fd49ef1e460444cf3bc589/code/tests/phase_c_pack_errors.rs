//! Phase C — error-path differential tests for `pack_unpack.c`
//! (ERRORS.md rows 185..234).

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_longlong, c_void};
use std::ptr;

macro_rules! pack_err {
    ($api:expr, $rec:expr, $row:expr, $want:expr, $fmt:expr $(, $arg:expr)*) => {{
        let f = cs($fmt);
        let mut e = JsonError::patterned();
        let j = ($api.json_pack_ex)(&mut e, 0, f.as_ptr() $(, $arg)*);
        assert!(j.is_null(), "[{}] row {}: {:?} should not pack", $api.tag, $row, $fmt);
        expect_code($api, $row, &e, $want);
        $rec.line(&format!("row{} {:?}", $row, $fmt));
        $rec.error(&format!("row{}", $row), &e);
        decref($api, j);
    }};
}

macro_rules! pack_ok {
    ($api:expr, $rec:expr, $tag:expr, $fmt:expr $(, $arg:expr)*) => {{
        let f = cs($fmt);
        let mut e = JsonError::patterned();
        let j = ($api.json_pack_ex)(&mut e, 0, f.as_ptr() $(, $arg)*);
        $rec.json($tag, j);
        $rec.error(&format!("{}.err", $tag), &e);
        rec_dump_all($api, $rec, $tag, j);
        decref($api, j);
    }};
}

/* --------------------------- rows 185..188: format string validation ---- */

#[test]
fn err185to188_format_validation() {
    diff("ERRORS 185-188 pack format", |api, rec| unsafe {
        // row 185: fmt == NULL
        let mut e = JsonError::patterned();
        let j = (api.json_pack_ex)(&mut e, 0, ptr::null::<c_char>());
        assert!(j.is_null());
        expect_code(api, 185, &e, E_INVALID_ARGUMENT);
        rec.error("row185", &e);
        // json_pack() has no error struct
        rec.json("row185_plain", (api.json_pack)(ptr::null::<c_char>()));

        // row 186: fmt == ""
        pack_err!(api, rec, 186, E_INVALID_ARGUMENT, "");

        // row 187: garbage after a complete format
        pack_err!(api, rec, 187, E_INVALID_FORMAT, "[]x");
        pack_err!(api, rec, 187, E_INVALID_FORMAT, "{}i", 1i32);
        pack_err!(api, rec, 187, E_INVALID_FORMAT, "ii", 1i32, 2i32);
        pack_err!(api, rec, 187, E_INVALID_FORMAT, "n]");
        pack_err!(api, rec, 187, E_INVALID_FORMAT, "nn");
        pack_err!(api, rec, 187, E_INVALID_FORMAT, "[] []");

        // row 188: unknown format characters
        for fmt in [
            "q", "1", "]", "}", "#", "%", "+", "?", "*", "!", "~", "S", "d", "x", " q ", "\t!",
        ] {
            let f = cs(fmt);
            let mut e = JsonError::patterned();
            let j = (api.json_pack_ex)(&mut e, 0, f.as_ptr());
            assert!(j.is_null(), "[{}] row 188 {fmt:?}", api.tag);
            expect_code(api, 188, &e, E_INVALID_FORMAT);
            rec.error(&format!("row188.{fmt}"), &e);
        }
        // and nested inside containers
        for fmt in ["[q]", "{s:q}", "[[q]]"] {
            let f = cs(fmt);
            let k = cs("k");
            let mut e = JsonError::patterned();
            let j = (api.json_pack_ex)(&mut e, 0, f.as_ptr(), k.as_ptr());
            assert!(j.is_null(), "[{}] row 188 {fmt:?}", api.tag);
            expect_code(api, 188, &e, E_INVALID_FORMAT);
            rec.error(&format!("row188n.{fmt}"), &e);
        }
    });
}

/* --------------------- rows 189..193: container format errors ----------- */

#[test]
fn err189to193_container_formats() {
    diff("ERRORS 189-193 containers", |api, rec| unsafe {
        let k = cs("key");
        // row 189: object format ends before '}'
        pack_err!(api, rec, 189, E_INVALID_FORMAT, "{");
        pack_err!(api, rec, 189, E_INVALID_FORMAT, "{s", k.as_ptr());
        pack_err!(api, rec, 189, E_INVALID_FORMAT, "{s:i", k.as_ptr(), 1i32);
        pack_err!(
            api,
            rec,
            189,
            E_INVALID_FORMAT,
            "{s:i,s:i",
            k.as_ptr(),
            1i32,
            k.as_ptr(),
            2i32
        );
        // row 190: key format is not 's'
        pack_err!(api, rec, 190, E_INVALID_FORMAT, "{i:i}", 1i32, 2i32);
        pack_err!(api, rec, 190, E_INVALID_FORMAT, "{n}");
        pack_err!(api, rec, 190, E_INVALID_FORMAT, "{[i]:i}", 1i32, 2i32);
        pack_err!(api, rec, 190, E_INVALID_FORMAT, "{b:i}", 1i32, 2i32);
        // row 193: array format ends before ']'
        pack_err!(api, rec, 193, E_INVALID_FORMAT, "[");
        pack_err!(api, rec, 193, E_INVALID_FORMAT, "[i", 1i32);
        pack_err!(api, rec, 193, E_INVALID_FORMAT, "[i,i", 1i32, 2i32);
        pack_err!(api, rec, 193, E_INVALID_FORMAT, "[[i]", 1i32);
        pack_err!(api, rec, 193, E_INVALID_FORMAT, "{s:[i}", k.as_ptr(), 1i32);
    });
}

/* --------------------- rows 194..199: read_string rejections ------------ */

#[test]
fn err194to199_read_string() {
    diff("ERRORS 194-199 read_string", |api, rec| unsafe {
        let k = cs("key");
        let good = cs("good");
        // row 194: NULL string / NULL object key
        pack_err!(api, rec, 194, E_NULL_VALUE, "s", ptr::null::<c_char>());
        pack_err!(api, rec, 194, E_NULL_VALUE, "[s]", ptr::null::<c_char>());
        pack_err!(
            api,
            rec,
            194,
            E_NULL_VALUE,
            "{s:i}",
            ptr::null::<c_char>(),
            1i32
        );
        pack_err!(
            api,
            rec,
            194,
            E_NULL_VALUE,
            "{s:s}",
            k.as_ptr(),
            ptr::null::<c_char>()
        );
        // row 195: invalid UTF-8 arguments
        for bad in [
            &b"\xff"[..],
            &b"\xc2"[..],
            &b"\xc0\x80"[..],
            &b"\xed\xa0\x80"[..],
            &b"ok\xffbad"[..],
        ] {
            let z = cbuf(bad);
            let p = z.as_ptr() as *const c_char;
            pack_err!(api, rec, 195, E_INVALID_UTF8, "s", p);
            pack_err!(api, rec, 195, E_INVALID_UTF8, "[s]", p);
            pack_err!(api, rec, 195, E_INVALID_UTF8, "{s:i}", p, 1i32);
            pack_err!(api, rec, 195, E_INVALID_UTF8, "{s:s}", k.as_ptr(), p);
            // with an explicit length, too
            pack_err!(api, rec, 195, E_INVALID_UTF8, "s#", p, bad.len() as c_int);
            pack_err!(api, rec, 195, E_INVALID_UTF8, "s%", p, bad.len());
        }
        // row 196: '#', '%', '+' on an optional string
        for fmt in ["s?#", "s?%", "s?+", "s*#", "s*%", "s*+"] {
            let f = cs(fmt);
            let mut e = JsonError::patterned();
            let j = (api.json_pack_ex)(&mut e, 0, f.as_ptr(), good.as_ptr(), 2i32);
            assert!(j.is_null(), "[{}] row 196 {fmt:?}", api.tag);
            expect_code(api, 196, &e, E_INVALID_FORMAT);
            rec.error(&format!("row196.{fmt}"), &e);
        }
        for fmt in ["{s:s?#}", "{s:s*%}"] {
            let f = cs(fmt);
            let mut e = JsonError::patterned();
            let j = (api.json_pack_ex)(&mut e, 0, f.as_ptr(), k.as_ptr(), good.as_ptr(), 2i32);
            assert!(j.is_null(), "[{}] row 196 {fmt:?}", api.tag);
            expect_code(api, 196, &e, E_INVALID_FORMAT);
            rec.error(&format!("row196o.{fmt}"), &e);
        }
        // row 198: the *concatenated* result is invalid UTF-8
        let half1 = cbuf(b"\xc2");
        let half2 = cbuf(b"A");
        pack_err!(
            api,
            rec,
            198,
            E_INVALID_UTF8,
            "s+",
            half1.as_ptr() as *const c_char,
            half2.as_ptr() as *const c_char
        );
        // ... while a split of a *valid* sequence is accepted
        let lo = cbuf(b"\x80");
        pack_ok!(
            api,
            rec,
            "concat_valid",
            "s+",
            half1.as_ptr() as *const c_char,
            lo.as_ptr() as *const c_char
        );
        // row 199: NULL argument inside a '+' chain
        pack_err!(
            api,
            rec,
            199,
            E_NULL_VALUE,
            "s+",
            good.as_ptr(),
            ptr::null::<c_char>()
        );
        pack_err!(
            api,
            rec,
            199,
            E_NULL_VALUE,
            "s+",
            ptr::null::<c_char>(),
            good.as_ptr()
        );
        pack_err!(
            api,
            rec,
            199,
            E_NULL_VALUE,
            "s+#",
            good.as_ptr(),
            ptr::null::<c_char>(),
            1i32
        );
    });
}

/* -------------------- rows 200/201/205: object refs and optionals ------- */

#[test]
fn err200_201_205_object_refs() {
    diff("ERRORS 200/201/205 O/o optionals", |api, rec| unsafe {
        let k = cs("k");
        // row 200: NULL json for 'O'/'o' with no modifier
        pack_err!(api, rec, 200, E_NULL_VALUE, "O", ptr::null::<c_void>());
        pack_err!(api, rec, 200, E_NULL_VALUE, "o", ptr::null::<c_void>());
        pack_err!(api, rec, 200, E_NULL_VALUE, "[O]", ptr::null::<c_void>());
        pack_err!(api, rec, 200, E_NULL_VALUE, "[o]", ptr::null::<c_void>());
        pack_err!(
            api,
            rec,
            200,
            E_NULL_VALUE,
            "{s:O}",
            k.as_ptr(),
            ptr::null::<c_void>()
        );

        // row 201: '*' skips silently — NULL result, error struct untouched
        for fmt in ["o*", "O*"] {
            let f = cs(fmt);
            let mut e = JsonError::patterned();
            let j = (api.json_pack_ex)(&mut e, 0, f.as_ptr(), ptr::null::<c_void>());
            rec.json(&format!("row201.{fmt}"), j);
            rec.error(&format!("row201.{fmt}.err"), &e);
            assert!(j.is_null());
            // `jsonp_error_init` only clears text[0]; when no error is set the
            // code byte at text[159] keeps the caller's pattern.
            assert_eq!(
                e.text[0], 0,
                "[{}] row 201: no error message must be set",
                api.tag
            );
            decref(api, j);
        }
        for fmt in ["[o*]", "[O*]", "{s:o*}", "{s:O*}"] {
            let f = cs(fmt);
            let mut e = JsonError::patterned();
            let j = if fmt.starts_with('{') {
                (api.json_pack_ex)(&mut e, 0, f.as_ptr(), k.as_ptr(), ptr::null::<c_void>())
            } else {
                (api.json_pack_ex)(&mut e, 0, f.as_ptr(), ptr::null::<c_void>())
            };
            rec.json(&format!("row201c.{fmt}"), j);
            rec.error(&format!("row201c.{fmt}.err"), &e);
            rec_dump_all(api, rec, &format!("row201c.{fmt}"), j);
            decref(api, j);
        }
        // '?' substitutes json_null()
        for fmt in ["o?", "O?", "[o?]", "[O?]", "{s:o?}", "{s:O?}"] {
            let f = cs(fmt);
            let mut e = JsonError::patterned();
            let j = if fmt.starts_with('{') {
                (api.json_pack_ex)(&mut e, 0, f.as_ptr(), k.as_ptr(), ptr::null::<c_void>())
            } else {
                (api.json_pack_ex)(&mut e, 0, f.as_ptr(), ptr::null::<c_void>())
            };
            rec.json(&format!("qmark.{fmt}"), j);
            rec.error(&format!("qmark.{fmt}.err"), &e);
            rec_dump_all(api, rec, &format!("qmark.{fmt}"), j);
            decref(api, j);
        }
        // row 205: pack_string with '?' and a NULL argument yields json_null()
        for fmt in ["s?", "[s?]", "{s:s?}", "s*", "[s*]", "{s:s*}"] {
            let f = cs(fmt);
            let mut e = JsonError::patterned();
            let j = if fmt.starts_with('{') {
                (api.json_pack_ex)(&mut e, 0, f.as_ptr(), k.as_ptr(), ptr::null::<c_char>())
            } else {
                (api.json_pack_ex)(&mut e, 0, f.as_ptr(), ptr::null::<c_char>())
            };
            rec.json(&format!("row205.{fmt}"), j);
            rec.error(&format!("row205.{fmt}.err"), &e);
            rec_dump_all(api, rec, &format!("row205.{fmt}"), j);
            decref(api, j);
        }
    });
}

/* ------------------------------- row 204: non-finite reals -------------- */

#[test]
fn err204_pack_real_non_finite() {
    diff("ERRORS 204 pack real", |api, rec| unsafe {
        let k = cs("k");
        for v in [
            f64::NAN,
            -f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::from_bits(0x7FF8_0000_0000_0000),
        ] {
            pack_err!(api, rec, 204, E_NUMERIC_OVERFLOW, "f", v);
            pack_err!(api, rec, 204, E_NUMERIC_OVERFLOW, "[f]", v);
            pack_err!(api, rec, 204, E_NUMERIC_OVERFLOW, "{s:f}", k.as_ptr(), v);
        }
        // finite extremes pack fine
        for v in [f64::MAX, -f64::MAX, 5e-324, 0.0, -0.0] {
            pack_ok!(api, rec, "finite", "f", v);
        }
    });
}

/* --------------- rows 192/197/202/203: allocation failures in pack ------ */

#[test]
fn err192_197_202_203_pack_oom() {
    diff("ERRORS 192/197/202/203 pack OOM", |api, rec| unsafe {
        // row 202: pack_integer
        oom_sweep(api, rec, "pack_i", 10, |api, rec| {
            let f = cs("i");
            let mut e = JsonError::patterned();
            let j = (api.json_pack_ex)(&mut e, 0, f.as_ptr(), 5i32);
            rec.json("j", j);
            rec.error("err", &e);
            decref(api, j);
        });
        // row 203: pack_real
        oom_sweep(api, rec, "pack_f", 10, |api, rec| {
            let f = cs("f");
            let mut e = JsonError::patterned();
            let j = (api.json_pack_ex)(&mut e, 0, f.as_ptr(), 1.5f64);
            rec.json("j", j);
            rec.error("err", &e);
            decref(api, j);
        });
        // row 192: json_object_setn_new_nocheck failure
        oom_sweep(api, rec, "pack_obj", 32, |api, rec| {
            let f = cs("{s:i,s:i,s:i}");
            let k1 = cs("a");
            let k2 = cs("b");
            let k3 = cs("c");
            let mut e = JsonError::patterned();
            let j = (api.json_pack_ex)(
                &mut e,
                0,
                f.as_ptr(),
                k1.as_ptr(),
                1i32,
                k2.as_ptr(),
                2i32,
                k3.as_ptr(),
                3i32,
            );
            rec.json("j", j);
            rec.error("err", &e);
            rec_dump_all(api, rec, "j", j);
            decref(api, j);
        });
        // row 197: strbuffer_init failure in the '+' path
        oom_sweep(api, rec, "pack_concat", 24, |api, rec| {
            let f = cs("s+");
            let a = cs("hello ");
            let b = cs("world");
            let mut e = JsonError::patterned();
            let j = (api.json_pack_ex)(&mut e, 0, f.as_ptr(), a.as_ptr(), b.as_ptr());
            rec.json("j", j);
            rec.error("err", &e);
            rec.cstring("v", (api.json_string_value)(j));
            decref(api, j);
        });
        // arrays too
        oom_sweep(api, rec, "pack_arr", 32, |api, rec| {
            let f = cs("[i,i,i,s,f]");
            let s = cs("txt");
            let mut e = JsonError::patterned();
            let j = (api.json_pack_ex)(
                &mut e, 0, f.as_ptr(), 1i32, 2i32, 3i32, s.as_ptr(), 1.5f64,
            );
            rec.json("j", j);
            rec.error("err", &e);
            rec_dump_all(api, rec, "j", j);
            decref(api, j);
        });
    });
}

/* -------------------- rows 206..210: unpack argument validation --------- */

#[test]
fn err206to210_unpack_validation() {
    diff("ERRORS 206-210 unpack args", |api, rec| unsafe {
        let doc = cs(r#"{"a":1}"#);
        let root = (api.json_loads)(doc.as_ptr(), 0, ptr::null_mut());
        let f = cs("{s:i}");
        let ka = cs("a");

        // row 206: root == NULL
        let mut e = JsonError::patterned();
        let mut i: c_int = 0;
        let r = (api.json_unpack_ex)(
            ptr::null_mut(),
            &mut e,
            0,
            f.as_ptr(),
            ka.as_ptr(),
            &mut i,
        );
        assert_eq!(r, -1);
        expect_code(api, 206, &e, E_NULL_VALUE);
        rec.error("row206", &e);
        rec.tag_i(
            "row206_plain",
            (api.json_unpack)(ptr::null_mut(), f.as_ptr(), ka.as_ptr(), &mut i) as i64,
        );

        // rows 207/208: fmt NULL / empty
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(root, &mut e, 0, ptr::null::<c_char>());
        assert_eq!(r, -1);
        expect_code(api, 207, &e, E_INVALID_ARGUMENT);
        rec.error("row207", &e);

        let empty = cs("");
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(root, &mut e, 0, empty.as_ptr());
        assert_eq!(r, -1);
        expect_code(api, 208, &e, E_INVALID_ARGUMENT);
        rec.error("row208", &e);

        // both NULL
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(ptr::null_mut(), &mut e, 0, ptr::null::<c_char>());
        assert_eq!(r, -1);
        expect_code(api, 206, &e, E_NULL_VALUE);
        rec.error("row206b", &e);

        // row 209: garbage after the format
        for fmt in ["{s:i}x", "{s:i}i", "{s:i} {}"] {
            let cf = cs(fmt);
            let mut e = JsonError::patterned();
            let mut v: c_int = 0;
            let r = (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), ka.as_ptr(), &mut v);
            assert_eq!(r, -1, "[{}] row 209 {fmt:?}", api.tag);
            expect_code(api, 209, &e, E_INVALID_FORMAT);
            rec.error(&format!("row209.{fmt}"), &e);
        }

        // row 210: unknown format characters at the top level
        for fmt in ["q", "1", "]", "}", "#", "%", "+", "?", "*", "!", "~", "S"] {
            let cf = cs(fmt);
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr());
            assert_eq!(r, -1, "[{}] row 210 {fmt:?}", api.tag);
            expect_code(api, 210, &e, E_INVALID_FORMAT);
            rec.error(&format!("row210.{fmt}"), &e);
        }
        decref(api, root);
    });
}

/* ------------------- rows 211..219: unpack type validation ------------- */

#[test]
fn err211to219_wrong_types() {
    diff("ERRORS 211-219 wrong type", |api, rec| unsafe {
        let docs = [
            ("object", r#"{"x":1}"#),
            ("array", r#"[1]"#),
            ("string", r#""s""#),
            ("integer", r#"7"#),
            ("real", r#"1.5"#),
            ("true", r#"true"#),
            ("false", r#"false"#),
            ("null", r#"null"#),
        ];
        for (dname, d) in docs {
            let cd = cs(d);
            let root = (api.json_loads)(cd.as_ptr(), JSON_DECODE_ANY, ptr::null_mut());
            assert!(!root.is_null());
            // every scalar format against every root type
            let mut sstr: *const c_char = ptr::null();
            let mut slen: usize = 999;
            let mut si: c_int = -1;
            let mut sl: c_longlong = -1;
            let mut sf: f64 = -1.0;
            let mut so: *mut Json = ptr::null_mut();
            for (row, fmt) in [
                (211u32, "s"),
                (214, "i"),
                (215, "I"),
                (216, "b"),
                (217, "f"),
                (218, "F"),
                (219, "n"),
                (220, "{}"),
                (229, "[]"),
                (0, "o"),
                (0, "O"),
                (0, "s%"),
            ] {
                let cf = cs(fmt);
                let mut e = JsonError::patterned();
                let r = match fmt {
                    "s" => (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), &mut sstr),
                    "s%" => (api.json_unpack_ex)(
                        root, &mut e, 0, cf.as_ptr(), &mut sstr, &mut slen,
                    ),
                    "i" | "b" => (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), &mut si),
                    "I" => (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), &mut sl),
                    "f" | "F" => (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), &mut sf),
                    "o" | "O" => (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), &mut so),
                    _ => (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr()),
                };
                rec.tag_i(&format!("{dname}.{fmt}.ret"), r as i64);
                rec.error(&format!("{dname}.{fmt}.err"), &e);
                if r == -1 && row != 0 {
                    // rows 211/214..220/229 all report json_error_wrong_type
                    expect_code(api, row, &e, E_WRONG_TYPE);
                }
                if fmt == "O" && r == 0 && !so.is_null() {
                    decref(api, so);
                    so = ptr::null_mut();
                }
            }
            decref(api, root);
        }

        // rows 212/213: NULL out-parameters
        let cd = cs(r#""text""#);
        let root = (api.json_loads)(cd.as_ptr(), JSON_DECODE_ANY, ptr::null_mut());
        let f = cs("s");
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(root, &mut e, 0, f.as_ptr(), ptr::null_mut::<*const c_char>());
        assert_eq!(r, -1);
        expect_code(api, 212, &e, E_NULL_VALUE);
        rec.error("row212", &e);

        let f = cs("s%");
        let mut sstr: *const c_char = ptr::null();
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(
            root,
            &mut e,
            0,
            f.as_ptr(),
            &mut sstr,
            ptr::null_mut::<usize>(),
        );
        assert_eq!(r, -1);
        expect_code(api, 213, &e, E_NULL_VALUE);
        rec.error("row213", &e);

        // inside containers as well
        let cd2 = cs(r#"["text"]"#);
        let arr = (api.json_loads)(cd2.as_ptr(), 0, ptr::null_mut());
        let f = cs("[s]");
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(arr, &mut e, 0, f.as_ptr(), ptr::null_mut::<*const c_char>());
        assert_eq!(r, -1);
        expect_code(api, 212, &e, E_NULL_VALUE);
        rec.error("row212arr", &e);
        decref(api, arr);
        decref(api, root);
    });
}

/* --------------- rows 220..228: unpack_object specific errors ----------- */

#[test]
fn err220to228_unpack_object() {
    diff("ERRORS 220-228 unpack_object", |api, rec| unsafe {
        let ka = cs("a");
        let kb = cs("b");
        let kmiss = cs("missing");

        // row 220: root is not an object
        for d in [r#"[1]"#, r#""s""#, r#"1"#, r#"1.5"#, r#"true"#, r#"null"#] {
            let cd = cs(d);
            let root = (api.json_loads)(cd.as_ptr(), JSON_DECODE_ANY, ptr::null_mut());
            let f = cs("{s:i}");
            let mut i: c_int = 0;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(root, &mut e, 0, f.as_ptr(), ka.as_ptr(), &mut i);
            assert_eq!(r, -1);
            expect_code(api, 220, &e, E_WRONG_TYPE);
            rec.error(&format!("row220.{d}"), &e);
            decref(api, root);
        }

        let cd = cs(r#"{"a":1,"b":2,"c":3}"#);
        let root = (api.json_loads)(cd.as_ptr(), 0, ptr::null_mut());

        // row 222: a token after '!' / '*'
        for fmt in ["{!s:i}", "{*s:i}", "{s:i!s:i}", "{s:i*s:i}"] {
            let cf = cs(fmt);
            let mut i1: c_int = 0;
            let mut i2: c_int = 0;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(
                root,
                &mut e,
                0,
                cf.as_ptr(),
                ka.as_ptr(),
                &mut i1,
                kb.as_ptr(),
                &mut i2,
            );
            assert_eq!(r, -1, "[{}] row 222 {fmt:?}", api.tag);
            expect_code(api, 222, &e, E_INVALID_FORMAT);
            rec.error(&format!("row222.{fmt}"), &e);
        }

        // row 223: format ends before '}'
        for fmt in ["{", "{s", "{s:i", "{s:i,"] {
            let cf = cs(fmt);
            let mut i1: c_int = 0;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), ka.as_ptr(), &mut i1);
            assert_eq!(r, -1, "[{}] row 223 {fmt:?}", api.tag);
            expect_code(api, 223, &e, E_INVALID_FORMAT);
            rec.error(&format!("row223.{fmt}"), &e);
        }

        // row 224: key format is not 's'
        for fmt in ["{i:i}", "{n}", "{b:i}", "{[i]:i}", "{{s:i}}"] {
            let cf = cs(fmt);
            let mut i1: c_int = 0;
            let mut i2: c_int = 0;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), &mut i1, &mut i2);
            assert_eq!(r, -1, "[{}] row 224 {fmt:?}", api.tag);
            expect_code(api, 224, &e, E_INVALID_FORMAT);
            rec.error(&format!("row224.{fmt}"), &e);
        }

        // row 225: NULL key argument
        let f = cs("{s:i}");
        let mut i1: c_int = 0;
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(
            root,
            &mut e,
            0,
            f.as_ptr(),
            ptr::null::<c_char>(),
            &mut i1,
        );
        assert_eq!(r, -1);
        expect_code(api, 225, &e, E_NULL_VALUE);
        rec.error("row225", &e);

        // row 226: required key missing
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(root, &mut e, 0, f.as_ptr(), kmiss.as_ptr(), &mut i1);
        assert_eq!(r, -1);
        expect_code(api, 226, &e, E_ITEM_NOT_FOUND);
        rec.error("row226", &e);
        // ... but an optional key is fine
        let fopt = cs("{s?i}");
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(root, &mut e, 0, fopt.as_ptr(), kmiss.as_ptr(), &mut i1);
        rec.tag_i("row226_opt", r as i64);
        rec.error("row226_opt.err", &e);

        // rows 227/228: leftover keys under '!' or JSON_STRICT
        for (fmt, flags, nkeys) in [
            ("{s:i!}", 0usize, 1),
            ("{s:i}", JSON_STRICT, 1),
            ("{s:i,s:i!}", 0, 2),
            ("{s:i,s:i}", JSON_STRICT, 2),
            ("{s?i!}", 0, 1),
            ("{s?i}", JSON_STRICT, 1),
        ] {
            let cf = cs(fmt);
            let mut a1: c_int = 0;
            let mut a2: c_int = 0;
            let mut e = JsonError::patterned();
            let r = if nkeys == 1 {
                (api.json_unpack_ex)(root, &mut e, flags, cf.as_ptr(), ka.as_ptr(), &mut a1)
            } else {
                (api.json_unpack_ex)(
                    root,
                    &mut e,
                    flags,
                    cf.as_ptr(),
                    ka.as_ptr(),
                    &mut a1,
                    kb.as_ptr(),
                    &mut a2,
                )
            };
            assert_eq!(r, -1, "[{}] rows 227/228 {fmt:?} flags {flags}", api.tag);
            expect_code(api, 227, &e, E_END_OF_INPUT_EXPECTED);
            rec.error(&format!("row227.{fmt}.{flags}"), &e);
        }
        // row 228: gotopt path — an optional key that *is* present
        let cd2 = cs(r#"{"a":1,"b":2}"#);
        let r2 = (api.json_loads)(cd2.as_ptr(), 0, ptr::null_mut());
        let cf = cs("{s?i!}");
        let mut a1: c_int = 0;
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(r2, &mut e, 0, cf.as_ptr(), ka.as_ptr(), &mut a1);
        assert_eq!(r, -1);
        expect_code(api, 228, &e, E_END_OF_INPUT_EXPECTED);
        rec.error("row228", &e);
        // all keys consumed -> success even with '!'
        let cf = cs("{s?i,s?i!}");
        let mut a2: c_int = 0;
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(
            r2,
            &mut e,
            0,
            cf.as_ptr(),
            ka.as_ptr(),
            &mut a1,
            kb.as_ptr(),
            &mut a2,
        );
        rec.tag_i("row228_full", r as i64);
        rec.error("row228_full.err", &e);
        decref(api, r2);
        decref(api, root);
    });
}

/* --------------- rows 229..234: unpack_array specific errors ------------ */

#[test]
fn err229to234_unpack_array() {
    diff("ERRORS 229-234 unpack_array", |api, rec| unsafe {
        // row 229: root is not an array
        for d in [r#"{"a":1}"#, r#""s""#, r#"1"#, r#"1.5"#, r#"true"#, r#"null"#] {
            let cd = cs(d);
            let root = (api.json_loads)(cd.as_ptr(), JSON_DECODE_ANY, ptr::null_mut());
            let f = cs("[i]");
            let mut i: c_int = 0;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(root, &mut e, 0, f.as_ptr(), &mut i);
            assert_eq!(r, -1);
            expect_code(api, 229, &e, E_WRONG_TYPE);
            rec.error(&format!("row229.{d}"), &e);
            decref(api, root);
        }

        let cd = cs(r#"[1,2,3]"#);
        let root = (api.json_loads)(cd.as_ptr(), 0, ptr::null_mut());

        // row 230: a token after '!' / '*'
        for fmt in ["[!i]", "[*i]", "[i!i]", "[i*i]"] {
            let cf = cs(fmt);
            let mut a1: c_int = 0;
            let mut a2: c_int = 0;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), &mut a1, &mut a2);
            assert_eq!(r, -1, "[{}] row 230 {fmt:?}", api.tag);
            expect_code(api, 230, &e, E_INVALID_FORMAT);
            rec.error(&format!("row230.{fmt}"), &e);
        }

        // row 231: format ends before ']'
        for fmt in ["[", "[i", "[i,", "[i,i"] {
            let cf = cs(fmt);
            let mut a1: c_int = 0;
            let mut a2: c_int = 0;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), &mut a1, &mut a2);
            assert_eq!(r, -1, "[{}] row 231 {fmt:?}", api.tag);
            expect_code(api, 231, &e, E_INVALID_FORMAT);
            rec.error(&format!("row231.{fmt}"), &e);
        }

        // row 232: format char outside `unpack_value_starters`.  None of these
        // consume a vararg, so no out-parameter is passed at all.
        for fmt in ["[%]", "[#]", "[?]", "[+]", "[q]", "[1]", "[}]", "[S]", "[!q]", "[i,%]"] {
            let cf = cs(fmt);
            let mut a1: c_int = 0;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), &mut a1);
            assert_eq!(r, -1, "[{}] row 232 {fmt:?}", api.tag);
            expect_code(api, 232, &e, E_INVALID_FORMAT);
            rec.error(&format!("row232.{fmt}"), &e);
        }
        // 'F' *is* in the starter set: it takes a `double *` and accepts both
        // integers and reals, so it must succeed on `[1,2,3]`.
        {
            let cf = cs("[F]");
            let mut d: f64 = -1.0;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(root, &mut e, 0, cf.as_ptr(), &mut d);
            rec.tag_i("F_ret", r as i64);
            rec.tag_f("F_val", d);
            rec.error("F_err", &e);
        }

        // row 233: more format items than elements
        for (fmt, n) in [("[i,i,i,i]", 4), ("[i,i,i,i,i]", 5)] {
            let cf = cs(fmt);
            let mut v1: c_int = 0;
            let mut v2: c_int = 0;
            let mut v3: c_int = 0;
            let mut v4: c_int = 0;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(
                root,
                &mut e,
                0,
                cf.as_ptr(),
                &mut v1,
                &mut v2,
                &mut v3,
                &mut v4,
            );
            assert_eq!(r, -1, "[{}] row 233 {fmt:?} ({n})", api.tag);
            expect_code(api, 233, &e, E_INDEX_OUT_OF_RANGE);
            rec.error(&format!("row233.{fmt}"), &e);
        }
        // empty array with any item format
        let ce = cs("[]");
        let empty = (api.json_loads)(ce.as_ptr(), 0, ptr::null_mut());
        let cf = cs("[i]");
        let mut v1: c_int = 0;
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(empty, &mut e, 0, cf.as_ptr(), &mut v1);
        assert_eq!(r, -1);
        expect_code(api, 233, &e, E_INDEX_OUT_OF_RANGE);
        rec.error("row233_empty", &e);
        decref(api, empty);

        // row 234: leftover elements under '!' or JSON_STRICT
        for (fmt, flags, n) in [
            ("[i!]", 0usize, 1),
            ("[i]", JSON_STRICT, 1),
            ("[i,i!]", 0, 2),
            ("[i,i]", JSON_STRICT, 2),
        ] {
            let cf = cs(fmt);
            let mut v1: c_int = 0;
            let mut v2: c_int = 0;
            let mut e = JsonError::patterned();
            let r = if n == 1 {
                (api.json_unpack_ex)(root, &mut e, flags, cf.as_ptr(), &mut v1)
            } else {
                (api.json_unpack_ex)(root, &mut e, flags, cf.as_ptr(), &mut v1, &mut v2)
            };
            assert_eq!(r, -1, "[{}] row 234 {fmt:?} flags {flags}", api.tag);
            expect_code(api, 234, &e, E_END_OF_INPUT_EXPECTED);
            rec.error(&format!("row234.{fmt}.{flags}"), &e);
        }
        // '*' suppresses the leftover check
        let cf = cs("[i*]");
        let mut v1: c_int = 0;
        let mut e = JsonError::patterned();
        let r = (api.json_unpack_ex)(root, &mut e, JSON_STRICT, cf.as_ptr(), &mut v1);
        rec.tag_i("row234_star", r as i64);
        rec.error("row234_star.err", &e);
        decref(api, root);
    });
}

/* --------------------- row 221: unpack_object hashtable_init OOM ------- */

#[test]
fn err221_unpack_object_oom() {
    diff("ERRORS 221 unpack_object OOM", |api, rec| unsafe {
        oom_sweep(api, rec, "unpack_obj", 32, |api, rec| {
            let cd = cs(r#"{"a":1,"b":2}"#);
            let root = (api.json_loads)(cd.as_ptr(), 0, ptr::null_mut());
            if root.is_null() {
                rec.line("root=NULL");
                return;
            }
            let ka = cs("a");
            let kb = cs("b");
            let f = cs("{s:i,s:i}");
            let mut a1: c_int = -1;
            let mut a2: c_int = -1;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(
                root,
                &mut e,
                JSON_STRICT,
                f.as_ptr(),
                ka.as_ptr(),
                &mut a1,
                kb.as_ptr(),
                &mut a2,
            );
            rec.tag_i("ret", r as i64);
            rec.tag_i("a1", a1 as i64);
            rec.tag_i("a2", a2 as i64);
            rec.error("err", &e);
            decref(api, root);
        });
        // the strbuffer used to build the "left unpacked" message can fail too
        oom_sweep(api, rec, "unpack_leftover", 40, |api, rec| {
            let cd = cs(r#"{"a":1,"b":2,"c":3}"#);
            let root = (api.json_loads)(cd.as_ptr(), 0, ptr::null_mut());
            if root.is_null() {
                rec.line("root=NULL");
                return;
            }
            let ka = cs("a");
            let f = cs("{s:i!}");
            let mut a1: c_int = -1;
            let mut e = JsonError::patterned();
            let r = (api.json_unpack_ex)(root, &mut e, 0, f.as_ptr(), ka.as_ptr(), &mut a1);
            rec.tag_i("ret", r as i64);
            rec.error("err", &e);
            decref(api, root);
        });
    });
}
