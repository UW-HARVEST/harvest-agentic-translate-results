//! Phase B/C — pack_unpack.c: `json_pack*` / `json_unpack*`.
//! CONFIGS rows 91-104 · ERRORS rows 201-246.
mod common;
use common::*;
use std::ffi::{c_char, c_int, c_longlong};

/// Result of a pack attempt, as an external caller sees it.
type PackObs = (bool, String, (c_int, c_int, c_int, String, String, i32, Vec<u8>));

unsafe fn pobs(api: &'static Api, j: *mut JsonT, e: &JsonError) -> PackObs {
    unsafe {
        let ok = !j.is_null();
        let sh = if ok { shape(api, j) } else { String::new() };
        (ok, sh, e.snapshot())
    }
}

/// Runs the same `json_pack_ex` call against both libraries. `f` receives the
/// api plus a fresh error struct and performs the (variadic) call.
#[track_caller]
fn diff_pack<F>(what: &str, mut f: F)
where
    F: FnMut(&'static Api, *mut JsonError) -> *mut JsonT,
{
    unsafe {
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        let cj = f(c(), &mut ce);
        let rj = f(r(), &mut re);
        let co = pobs(c(), cj, &ce);
        let ro = pobs(r(), rj, &re);
        assert_eq!(co, ro, "{what}\n  C   = {co:?}\n  RUST= {ro:?}");
        decref(c(), cj);
        decref(r(), rj);
    }
}

/* ================= CONFIGS 91: every pack format char ================= */

#[test]
fn pack_every_format_char() {
    let _g = dtoa_guard();
    unsafe {
        // no-arg formats
        for fmt in ["n", "{}", "[]", "[n]", "{s:n}", "[[]]", "[{}]", "{s:{}}", "{s:[]}"] {
            let f = cs(fmt);
            let key = cs("k");
            diff_pack(&format!("pack({fmt:?})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), key.as_ptr())
            });
        }
        // b (int)
        for v in [0i32, 1, -1, 2, i32::MIN, i32::MAX] {
            let f = cs("b");
            diff_pack(&format!("pack(\"b\", {v})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), v)
            });
            let f2 = cs("[b,b]");
            diff_pack(&format!("pack(\"[b,b]\", {v})"), |api, e| {
                (api.json_pack_ex)(e, 0, f2.as_ptr(), v, !v)
            });
        }
        // i (int) and I (json_int_t)
        let mut rng = Rng::new(0x0AC1_0001);
        let mut ivals: Vec<i32> = vec![0, 1, -1, i32::MIN, i32::MAX];
        let mut lvals: Vec<i64> = vec![0, 1, -1, i64::MIN, i64::MAX];
        for _ in 0..600 {
            ivals.push(rng.next_u32() as i32);
            lvals.push(rng.next_u64() as i64);
        }
        for &v in &ivals {
            let f = cs("i");
            diff_pack(&format!("pack(\"i\", {v})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), v)
            });
        }
        for &v in &lvals {
            let f = cs("I");
            diff_pack(&format!("pack(\"I\", {v})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), v as c_longlong)
            });
        }
        // f (double) — including NaN/Inf (ERRORS 219)
        let mut fvals: Vec<f64> = vec![
            0.0, -0.0, 1.0, -1.0, 0.5, 1e300, 1e-300,
            f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
            f64::MAX, f64::MIN, 5e-324,
        ];
        for _ in 0..600 {
            fvals.push(rng.f64_smallish());
        }
        for &v in &fvals {
            let f = cs("f");
            diff_pack(&format!("pack(\"f\", {v:?})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), v)
            });
            let f2 = cs("[f]");
            diff_pack(&format!("pack(\"[f]\", {v:?})"), |api, e| {
                (api.json_pack_ex)(e, 0, f2.as_ptr(), v)
            });
            let f3 = cs("{s:f}");
            let k = cs("d");
            diff_pack(&format!("pack(\"{{s:f}}\", {v:?})"), |api, e| {
                (api.json_pack_ex)(e, 0, f3.as_ptr(), k.as_ptr(), v)
            });
        }
        // s (string)
        let mut svals: Vec<String> = vec![
            "".into(), "a".into(), "hello".into(), "héllo".into(), "€uro".into(),
            "😀".into(), "with \"quotes\" and \\slashes/".into(),
            "\u{1}\u{1f}".into(),
        ];
        for _ in 0..600 {
            svals.push(rng.utf8(12));
        }
        for v in &svals {
            let f = cs("s");
            let a = cs(v);
            diff_pack(&format!("pack(\"s\", {v:?})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), a.as_ptr())
            });
        }
        // invalid UTF-8 string arg (ERRORS 211)
        for bad in [
            vec![0xC2u8, 0x00],
            vec![0xE2, 0x82, 0x00],
            vec![0xFF, 0x00],
            vec![0xED, 0xA0, 0x80, 0x00],
        ] {
            let f = cs("s");
            let p = bad.as_ptr() as *const c_char;
            diff_pack(&format!("pack(\"s\", {bad:02x?})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), p)
            });
        }
        // o / O (json_t*): each library needs its OWN object
        for fmt in ["o", "O", "[o]", "[O]", "{s:o}", "{s:O}"] {
            let f = cs(fmt);
            let k = cs("k");
            unsafe {
                let mut ce = JsonError::default();
                let mut re = JsonError::default();
                let cv = (c().json_integer)(77);
                let rv = (r().json_integer)(77);
                let (cj, rj) = if fmt.starts_with('{') {
                    (
                        (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), k.as_ptr(), cv),
                        (r().json_pack_ex)(&mut re, 0, f.as_ptr(), k.as_ptr(), rv),
                    )
                } else {
                    (
                        (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), cv),
                        (r().json_pack_ex)(&mut re, 0, f.as_ptr(), rv),
                    )
                };
                assert_eq!(
                    pobs(c(), cj, &ce),
                    pobs(r(), rj, &re),
                    "pack({fmt:?}, json_t*)"
                );
                decref(c(), cj);
                decref(r(), rj);
            }
        }
    }
}

/* ================= CONFIGS 92/93: s#, s%, s+ ================= */

#[test]
fn pack_string_length_and_concat_specs() {
    unsafe {
        let mut rng = Rng::new(0xC0FFEE01);
        let mut cases: Vec<(String, usize)> = vec![
            ("hello".into(), 5),
            ("hello".into(), 0),
            ("hello".into(), 3),
            ("héllo".into(), 2),   // cuts a multi-byte sequence: must be rejected
            ("héllo".into(), 3),
            ("€".into(), 1),
            ("€".into(), 3),
            ("".into(), 0),
        ];
        for _ in 0..400 {
            let s = rng.utf8(10);
            let n = rng.below(s.len() + 1);
            cases.push((s, n));
        }
        for (s, n) in &cases {
            let a = cs(s);
            // s#  (length is an int)
            let f = cs("s#");
            let ni = *n as c_int;
            diff_pack(&format!("pack(\"s#\", {s:?}, {n})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), a.as_ptr(), ni)
            });
            // s%  (length is a size_t)
            let f = cs("s%");
            let nz = *n;
            diff_pack(&format!("pack(\"s%\", {s:?}, {n})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), a.as_ptr(), nz)
            });
            // inside containers
            let f = cs("[s#]");
            diff_pack(&format!("pack(\"[s#]\", {s:?}, {n})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), a.as_ptr(), ni)
            });
            let f = cs("{s#:i}");
            diff_pack(&format!("pack(\"{{s#:i}}\", {s:?}, {n})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), a.as_ptr(), ni, 1i32)
            });
        }

        // s+  concatenation chains
        let parts: Vec<(&str, &str, &str)> = vec![
            ("a", "b", "c"),
            ("", "", ""),
            ("héllo ", "wörld", "!"),
            ("😀", "😀", "😀"),
            ("x", "", "y"),
        ];
        for (p1, p2, p3) in parts {
            let a = cs(p1);
            let b = cs(p2);
            let d = cs(p3);
            let f = cs("s+");
            diff_pack(&format!("pack(\"s+\", {p1:?}, {p2:?})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), a.as_ptr(), b.as_ptr())
            });
            let f = cs("s++");
            diff_pack(&format!("pack(\"s++\", 3 parts)"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), a.as_ptr(), b.as_ptr(), d.as_ptr())
            });
            let f = cs("s#+");
            diff_pack(&format!("pack(\"s#+\", {p1:?}/1, {p2:?})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), a.as_ptr(), 1i32, b.as_ptr())
            });
            let f = cs("s%+%");
            diff_pack(&format!("pack(\"s%+%\")"), |api, e| {
                (api.json_pack_ex)(
                    e, 0, f.as_ptr(), a.as_ptr(), 1usize, b.as_ptr(), 1usize,
                )
            });
            let f = cs("{s:s+}");
            let k = cs("k");
            diff_pack("pack(\"{s:s+}\")", |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), k.as_ptr(), a.as_ptr(), b.as_ptr())
            });
        }
        // ERRORS 213: concatenation that produces invalid UTF-8
        let half1 = [0xC3u8, 0x00];
        let half2 = [0xA9u8, 0x00];
        let f = cs("s+");
        diff_pack("ERRORS 213: split multi-byte across s+", |api, e| {
            (api.json_pack_ex)(
                e,
                0,
                f.as_ptr(),
                half1.as_ptr() as *const c_char,
                half2.as_ptr() as *const c_char,
            )
        });
        let bad = [0xFFu8, 0x00];
        diff_pack("ERRORS 213: invalid UTF-8 via s+", |api, e| {
            (api.json_pack_ex)(
                e,
                0,
                f.as_ptr(),
                bad.as_ptr() as *const c_char,
                bad.as_ptr() as *const c_char,
            )
        });
        // ERRORS 210: NULL string in the s+ chain
        diff_pack("ERRORS 210: NULL in s+ chain", |api, e| {
            (api.json_pack_ex)(e, 0, f.as_ptr(), std::ptr::null::<c_char>(), half2.as_ptr() as *const c_char)
        });
    }
}

/* ================= CONFIGS 94 · ERRORS 212, 214-218 ================= */

#[test]
fn pack_optional_specs() {
    unsafe {
        let good = cs("val");
        let nul = std::ptr::null::<c_char>();
        // s? / s* with non-NULL and NULL
        for fmt in ["s?", "s*", "[s?]", "[s*]", "{s:s?}", "{s:s*}", "[s?,i]", "[s*,i]"] {
            let f = cs(fmt);
            let k = cs("k");
            for &arg in &[good.as_ptr(), nul] {
                let has_key = fmt.starts_with('{');
                let has_i = fmt.contains(",i");
                diff_pack(&format!("pack({fmt:?}, {})", if arg.is_null() { "NULL" } else { "\"val\"" }), |api, e| {
                    if has_key {
                        (api.json_pack_ex)(e, 0, f.as_ptr(), k.as_ptr(), arg)
                    } else if has_i {
                        (api.json_pack_ex)(e, 0, f.as_ptr(), arg, 5i32)
                    } else {
                        (api.json_pack_ex)(e, 0, f.as_ptr(), arg)
                    }
                });
            }
        }
        // ERRORS 212: optional combined with #/%/+
        for fmt in ["s?#", "s?%", "s?+", "s*#", "s*%", "s*+"] {
            let f = cs(fmt);
            diff_pack(&format!("ERRORS 212: pack({fmt:?})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), good.as_ptr(), 1i32)
            });
        }
        // o?/o*/O?/O* with non-NULL and NULL json_t*
        for fmt in ["o?", "o*", "O?", "O*", "[o?]", "[o*]", "[O?]", "[O*]", "{s:o?}", "{s:o*}"] {
            let f = cs(fmt);
            let k = cs("k");
            for use_null in [false, true] {
                let mut ce = JsonError::default();
                let mut re = JsonError::default();
                let cv = if use_null {
                    std::ptr::null_mut()
                } else {
                    (c().json_integer)(5)
                };
                let rv = if use_null {
                    std::ptr::null_mut()
                } else {
                    (r().json_integer)(5)
                };
                let (cj, rj) = if fmt.starts_with('{') {
                    (
                        (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), k.as_ptr(), cv),
                        (r().json_pack_ex)(&mut re, 0, f.as_ptr(), k.as_ptr(), rv),
                    )
                } else {
                    (
                        (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), cv),
                        (r().json_pack_ex)(&mut re, 0, f.as_ptr(), rv),
                    )
                };
                assert_eq!(
                    pobs(c(), cj, &ce),
                    pobs(r(), rj, &re),
                    "pack({fmt:?}, null={use_null})"
                );
                decref(c(), cj);
                decref(r(), rj);
            }
        }
        // ERRORS 216: NULL json_t* with no ?/*
        for fmt in ["o", "O", "[o]", "{s:O}"] {
            let f = cs(fmt);
            let k = cs("k");
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let (cj, rj) = if fmt.starts_with('{') {
                (
                    (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), k.as_ptr(), std::ptr::null_mut::<JsonT>()),
                    (r().json_pack_ex)(&mut re, 0, f.as_ptr(), k.as_ptr(), std::ptr::null_mut::<JsonT>()),
                )
            } else {
                (
                    (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), std::ptr::null_mut::<JsonT>()),
                    (r().json_pack_ex)(&mut re, 0, f.as_ptr(), std::ptr::null_mut::<JsonT>()),
                )
            };
            assert_eq!(
                pobs(c(), cj, &ce),
                pobs(r(), rj, &re),
                "ERRORS 216: pack({fmt:?}, NULL)"
            );
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* ================= CONFIGS 95/96/97 ================= */

#[test]
fn pack_nested_and_whitespace_and_flags() {
    let _g = dtoa_guard();
    unsafe {
        let k1 = cs("k1");
        let k2 = cs("k2");
        let k3 = cs("k3");
        let sv = cs("sv");
        let nested: Vec<&str> = vec![
            "{s:{s:[i,i]}}",
            "{ s : { s : [ i , i ] } }",
            "{s:{s:{s:{s:i}}}}",
            "[[[[i]]]]",
            "{s:[{s:i},{s:i}]}",
            "[i,s,f,b,n]",
            "{s:i,s:s,s:f}",
            "\n{\n  s : i ,\n  s : s\n}\n",
            "{s:i}{",  // ERRORS 203: garbage after
            "{s:i}x",
            "[i]]",
            "[i] ",
        ];
        for fmt in nested {
            let f = cs(fmt);
            for &flags in &[0usize, JSON_VALIDATE_ONLY, JSON_STRICT, JSON_VALIDATE_ONLY | JSON_STRICT] {
                diff_pack(&format!("pack({fmt:?}, flags={flags:#x})"), |api, e| {
                    match fmt {
                        "{s:{s:[i,i]}}" | "{ s : { s : [ i , i ] } }" => (api.json_pack_ex)(
                            e, flags, f.as_ptr(), k1.as_ptr(), k2.as_ptr(), 1i32, 2i32,
                        ),
                        "{s:{s:{s:{s:i}}}}" => (api.json_pack_ex)(
                            e, flags, f.as_ptr(), k1.as_ptr(), k2.as_ptr(), k3.as_ptr(),
                            k1.as_ptr(), 9i32,
                        ),
                        "[[[[i]]]]" => (api.json_pack_ex)(e, flags, f.as_ptr(), 4i32),
                        "{s:[{s:i},{s:i}]}" => (api.json_pack_ex)(
                            e, flags, f.as_ptr(), k1.as_ptr(), k2.as_ptr(), 1i32,
                            k3.as_ptr(), 2i32,
                        ),
                        "[i,s,f,b,n]" => (api.json_pack_ex)(
                            e, flags, f.as_ptr(), 1i32, sv.as_ptr(), 2.5f64, 1i32,
                        ),
                        "{s:i,s:s,s:f}" => (api.json_pack_ex)(
                            e, flags, f.as_ptr(), k1.as_ptr(), 1i32, k2.as_ptr(),
                            sv.as_ptr(), k3.as_ptr(), 2.5f64,
                        ),
                        "\n{\n  s : i ,\n  s : s\n}\n" => (api.json_pack_ex)(
                            e, flags, f.as_ptr(), k1.as_ptr(), 1i32, k2.as_ptr(),
                            sv.as_ptr(),
                        ),
                        "{s:i}{" | "{s:i}x" => (api.json_pack_ex)(
                            e, flags, f.as_ptr(), k1.as_ptr(), 1i32,
                        ),
                        _ => (api.json_pack_ex)(e, flags, f.as_ptr(), 1i32),
                    }
                });
            }
        }
    }
}

/* ================= ERRORS 201-209, 220 ================= */

#[test]
fn pack_format_errors() {
    unsafe {
        // ERRORS 201/202: NULL and empty format
        for api in both() {
            let mut e = JsonError::default();
            assert!((api.json_pack_ex)(&mut e, 0, std::ptr::null::<c_char>()).is_null());
            assert_eq!(e.code(), E_INVALID_ARGUMENT, "{}", api.tag);
            assert_eq!(e.text_str(), "NULL or empty format string");
            assert_eq!(e.source_str(), "<format>");
        }
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        (c().json_pack_ex)(&mut ce, 0, std::ptr::null::<c_char>());
        (r().json_pack_ex)(&mut re, 0, std::ptr::null::<c_char>());
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 201");
        let empty = cs("");
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        (c().json_pack_ex)(&mut ce, 0, empty.as_ptr());
        (r().json_pack_ex)(&mut re, 0, empty.as_ptr());
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 202");

        // ERRORS 204/205/206/208 and various malformed formats
        let bad_no_args: Vec<&str> = vec![
            "x", "q", "Q", "1", "?", "*", "+", "#", "%", "!", ":", ",", " ", "  ",
            "}", "]", "{", "[", "{}}", "[]]", "{[", "[{", "{s", "[i", "{:", "{,",
            "[,", "{s:", "{s:}", "{i:i}", "{n:i}", "{{}}", "[[]", "{s:i", "z{}",
            "n n", "nn", "[n]n", "\t", "\n", ",,,", "::",
        ];
        for fmt in bad_no_args {
            let f = cs(fmt);
            // Supply a couple of harmless args in case the format consumes some.
            let k = cs("k");
            diff_pack(&format!("pack({fmt:?})"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), k.as_ptr(), 1i32, k.as_ptr(), 2i32)
            });
        }

        // ERRORS 207: object value packs to NULL and the spec is not '*'
        for fmt in ["{s:o}", "{s:s}", "{s:o?}"] {
            let f = cs(fmt);
            let k = cs("k");
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let cj = (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), k.as_ptr(), std::ptr::null_mut::<JsonT>());
            let rj = (r().json_pack_ex)(&mut re, 0, f.as_ptr(), k.as_ptr(), std::ptr::null_mut::<JsonT>());
            assert_eq!(
                pobs(c(), cj, &ce),
                pobs(r(), rj, &re),
                "ERRORS 207: pack({fmt:?}, NULL value)"
            );
            decref(c(), cj);
            decref(r(), rj);
        }
        // ERRORS 209: array element packs to NULL, spec not '*'
        for fmt in ["[o]", "[s]", "[o,i]", "[s,i]"] {
            let f = cs(fmt);
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let cj = (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), std::ptr::null_mut::<JsonT>(), 1i32);
            let rj = (r().json_pack_ex)(&mut re, 0, f.as_ptr(), std::ptr::null_mut::<JsonT>(), 1i32);
            assert_eq!(
                pobs(c(), cj, &ce),
                pobs(r(), rj, &re),
                "ERRORS 209: pack({fmt:?}, NULL element)"
            );
            decref(c(), cj);
            decref(r(), rj);
        }
        // ERRORS 210: NULL object key
        for fmt in ["{s:i}", "{s#:i}"] {
            let f = cs(fmt);
            diff_pack(&format!("ERRORS 210: pack({fmt:?}, NULL key)"), |api, e| {
                (api.json_pack_ex)(e, 0, f.as_ptr(), std::ptr::null::<c_char>(), 1i32, 1i32)
            });
        }
        // json_pack (no error struct) must agree too
        for fmt in ["{s:i}", "x", "", "[i]"] {
            let f = cs(fmt);
            let k = cs("k");
            let cj = (c().json_pack)(f.as_ptr(), k.as_ptr(), 1i32);
            let rj = (r().json_pack)(f.as_ptr(), k.as_ptr(), 1i32);
            assert_eq!(cj.is_null(), rj.is_null(), "json_pack({fmt:?})");
            if !cj.is_null() {
                assert_eq!(shape(c(), cj), shape(r(), rj), "json_pack({fmt:?})");
            }
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* ===================== UNPACK ===================== */

/// The document corpus used for unpack tests, as JSON text.
fn unpack_docs() -> Vec<&'static str> {
    vec![
        r#"{"i":1,"s":"str","f":1.5,"b":true,"n":null,"a":[1,2],"o":{"x":1}}"#,
        r#"{"i":1}"#,
        r#"{}"#,
        r#"[]"#,
        r#"[1,2,3]"#,
        r#"["a","b"]"#,
        r#"[1,"a",1.5,true,null]"#,
        r#"{"a":{"b":[1,2]}}"#,
        r#"1"#,
        r#""str""#,
        r#"1.5"#,
        r#"true"#,
        r#"false"#,
        r#"null"#,
    ]
}

unsafe fn load_both(text: &str) -> (*mut JsonT, *mut JsonT) {
    unsafe {
        let t = cs(text);
        let cj = (c().json_loads)(t.as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
        let rj = (r().json_loads)(t.as_ptr(), JSON_DECODE_ANY, std::ptr::null_mut());
        assert!(!cj.is_null() && !rj.is_null(), "setup load {text:?}");
        (cj, rj)
    }
}

/* ---- CONFIGS 98/99/100/101/102/103/104 · ERRORS 224-246 ---- */

#[test]
fn unpack_every_format_and_error() {
    let _g = dtoa_guard();
    unsafe {
        let key_i = cs("i");
        let key_s = cs("s");
        let key_f = cs("f");
        let key_b = cs("b");
        let key_n = cs("n");
        let key_a = cs("a");
        let key_o = cs("o");
        let key_miss = cs("missing");

        // Formats that take no output args (or only ones we can supply blindly).
        let simple: Vec<&str> = vec![
            "{}", "[]", "n", "{s:n}", "[n]", "{!}", "{*}", "[!]", "[*]",
            "{s:i}", "{s:I}", "{s:s}", "{s:f}", "{s:F}", "{s:b}", "{s:o}", "{s:O}",
            "{s?i}", "{s?s}", "{s?o}",
            "[i]", "[i,i]", "[i,i,i]", "[s]", "[f]", "[b]", "[o]", "[O]", "[n]",
            "{s:i,s:s}", "{s:i!}", "{s:i*}", "[i!]", "[i*]",
            "{s:[i,i]}", "{s:{s:i}}",
            // format errors
            "x", "", "{", "[", "{s", "[i", "{s:", "{i:i}", "{s:x}", "[x]", "{s:i}x",
            "[i]]", "{!i}", "{*i}", "[!i]", "[*i]", "{s:i!x}", "[i!x]",
            "{s:i}{", "%", "#", "+", "?",
        ];
        for text in unpack_docs() {
            for fmt in &simple {
                for &flags in &[
                    0usize,
                    JSON_VALIDATE_ONLY,
                    JSON_STRICT,
                    JSON_VALIDATE_ONLY | JSON_STRICT,
                ] {
                    // With JSON_VALIDATE_ONLY the C consumes NO output
                    // arguments, so a format with two or more value slots
                    // would read the next output pointer as its key. That is
                    // caller error, not a translation difference, so restrict
                    // VALIDATE_ONLY to formats with at most one value slot.
                    if flags & JSON_VALIDATE_ONLY != 0
                        && fmt.matches(|ch| "iIsfFbBoO".contains(ch)).count() > 1
                    {
                        continue;
                    }
                    let (cj, rj) = load_both(text);
                    let f = cs(fmt);
                    // Output slots (unused when the format doesn't consume them).
                    let mut ci: c_int = -1;
                    let mut ri: c_int = -1;
                    let mut cii: c_longlong = -1;
                    let mut rii: c_longlong = -1;
                    let mut cd: f64 = -1.0;
                    let mut rd: f64 = -1.0;
                    let mut cp: *const c_char = std::ptr::null();
                    let mut rp: *const c_char = std::ptr::null();
                    let mut cjp: *mut JsonT = std::ptr::null_mut();
                    let mut rjp: *mut JsonT = std::ptr::null_mut();
                    let mut ce = JsonError::default();
                    let mut re = JsonError::default();
                    // Raw pointers so the same slot can be passed more than
                    // once in a single variadic call (as C callers do).
                    let cip = &mut ci as *mut c_int;
                    let rip = &mut ri as *mut c_int;

                    // Pick the argument list from the format's shape. Only the
                    // arguments the C actually consumes matter; extras are
                    // harmless because C variadics ignore them.
                    let (crc, rrc) = match *fmt {
                        "{s:i}" | "{s:i!}" | "{s:i*}" | "{s?i}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_i.as_ptr(), cip),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_i.as_ptr(), rip),
                        ),
                        "{s:I}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_i.as_ptr(), &mut cii),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_i.as_ptr(), &mut rii),
                        ),
                        "{s:s}" | "{s?s}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_s.as_ptr(), &mut cp),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_s.as_ptr(), &mut rp),
                        ),
                        "{s:f}" | "{s:F}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_f.as_ptr(), &mut cd),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_f.as_ptr(), &mut rd),
                        ),
                        "{s:b}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_b.as_ptr(), cip),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_b.as_ptr(), rip),
                        ),
                        "{s:n}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_n.as_ptr()),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_n.as_ptr()),
                        ),
                        "{s:o}" | "{s:O}" | "{s?o}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_o.as_ptr(), &mut cjp),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_o.as_ptr(), &mut rjp),
                        ),
                        "{s:[i,i]}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_a.as_ptr(), cip, cip),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_a.as_ptr(), rip, rip),
                        ),
                        "{s:{s:i}}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_o.as_ptr(), cs("x").as_ptr(), cip),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_o.as_ptr(), cs("x").as_ptr(), rip),
                        ),
                        "{s:i,s:s}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_i.as_ptr(), cip, key_s.as_ptr(), &mut cp),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_i.as_ptr(), rip, key_s.as_ptr(), &mut rp),
                        ),
                        "[i]" | "[i!]" | "[i*]" | "[!i]" | "[*i]" | "[i!x]" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), cip),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), rip),
                        ),
                        "[i,i]" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), cip, cip),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), rip, rip),
                        ),
                        "[i,i,i]" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), cip, cip, cip),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), rip, rip, rip),
                        ),
                        "[s]" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), &mut cp),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), &mut rp),
                        ),
                        "[f]" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), &mut cd),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), &mut rd),
                        ),
                        "[b]" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), cip),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), rip),
                        ),
                        "[o]" | "[O]" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), &mut cjp),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), &mut rjp),
                        ),
                        "{s:x}" | "{!i}" | "{*i}" | "{s:i}x" | "{s:i}{" | "{s:i!x}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_i.as_ptr(), cip),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_i.as_ptr(), rip),
                        ),
                        _ => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), key_miss.as_ptr(), cip),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), key_miss.as_ptr(), rip),
                        ),
                    };
                    let tag = format!("unpack({text:?}, {fmt:?}, {flags:#x})");
                    assert_eq!(crc, rrc, "{tag} rc");
                    assert_eq!(ce.snapshot(), re.snapshot(), "{tag} error");
                    assert_eq!((ci, cii), (ri, rii), "{tag} int outputs");
                    assert_eq!(cd.to_bits(), rd.to_bits(), "{tag} double output");
                    assert_eq!(cp.is_null(), rp.is_null(), "{tag} string out null-ness");
                    if !cp.is_null() {
                        assert_eq!(
                            std::ffi::CStr::from_ptr(cp).to_bytes(),
                            std::ffi::CStr::from_ptr(rp).to_bytes(),
                            "{tag} string output"
                        );
                    }
                    assert_eq!(cjp.is_null(), rjp.is_null(), "{tag} json out null-ness");
                    if !cjp.is_null() {
                        assert_eq!(shape(c(), cjp), shape(r(), rjp), "{tag} json output");
                    }
                    // and the root must be unchanged in the same way
                    assert_eq!(shape(c(), cj), shape(r(), rj), "{tag} root");
                    decref(c(), cj);
                    decref(r(), rj);
                }
            }
        }
    }
}

/* ---- CONFIGS 99: s% length out-param · ERRORS 226, 227 ---- */

#[test]
fn unpack_string_with_length() {
    unsafe {
        for text in [
            r#"{"s":"hello"}"#,
            r#"{"s":""}"#,
            r#"{"s":"héllo"}"#,
            r#"{"s":"😀"}"#,
        ] {
            for &flags in &[0usize, JSON_VALIDATE_ONLY] {
                let (cj, rj) = load_both(text);
                let f = cs("{s:s%}");
                let k = cs("s");
                let mut cp: *const c_char = std::ptr::null();
                let mut rp: *const c_char = std::ptr::null();
                let mut cl: usize = 0xdead;
                let mut rl: usize = 0xdead;
                let mut ce = JsonError::default();
                let mut re = JsonError::default();
                let crc = (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), k.as_ptr(), &mut cp, &mut cl);
                let rrc = (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), k.as_ptr(), &mut rp, &mut rl);
                assert_eq!(crc, rrc, "unpack s% rc ({text:?}, {flags:#x})");
                assert_eq!(ce.snapshot(), re.snapshot(), "unpack s% error");
                assert_eq!(cl, rl, "unpack s% length ({text:?}, {flags:#x})");
                assert_eq!(cp.is_null(), rp.is_null());
                decref(c(), cj);
                decref(r(), rj);
            }
        }
        // ERRORS 226: NULL string target
        for fmt in ["{s:s}", "{s:s%}", "[s]"] {
            let (cj, rj) = load_both(r#"{"s":"x"}"#);
            let f = cs(fmt);
            let k = cs("s");
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let crc = (c().json_unpack_ex)(cj, &mut ce, 0, f.as_ptr(), k.as_ptr(), std::ptr::null_mut::<*const c_char>(), std::ptr::null_mut::<usize>());
            let rrc = (r().json_unpack_ex)(rj, &mut re, 0, f.as_ptr(), k.as_ptr(), std::ptr::null_mut::<*const c_char>(), std::ptr::null_mut::<usize>());
            assert_eq!(crc, rrc, "ERRORS 226/227: unpack({fmt:?}, NULL target) rc");
            assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 226/227 error");
            decref(c(), cj);
            decref(r(), rj);
        }
        // ERRORS 227: NULL length target only
        let (cj, rj) = load_both(r#"{"s":"x"}"#);
        let f = cs("{s:s%}");
        let k = cs("s");
        let mut cp: *const c_char = std::ptr::null();
        let mut rp: *const c_char = std::ptr::null();
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        let crc = (c().json_unpack_ex)(cj, &mut ce, 0, f.as_ptr(), k.as_ptr(), &mut cp, std::ptr::null_mut::<usize>());
        let rrc = (r().json_unpack_ex)(rj, &mut re, 0, f.as_ptr(), k.as_ptr(), &mut rp, std::ptr::null_mut::<usize>());
        assert_eq!(crc, rrc, "ERRORS 227 rc");
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 227 error");
        decref(c(), cj);
        decref(r(), rj);
    }
}

/* ---- CONFIGS 101/102 · ERRORS 235, 240, 242, 245, 246: strictness ---- */

#[test]
fn unpack_strictness_and_range() {
    let _g = dtoa_guard();
    unsafe {
        let docs = [
            r#"{"a":1,"b":2,"c":3}"#,
            r#"{"a":1}"#,
            r#"{}"#,
            r#"[1,2,3]"#,
            r#"[1]"#,
            r#"[]"#,
        ];
        let fmts: Vec<(&str, usize)> = vec![
            ("{s:i}", 1),
            ("{s:i,s:i}", 2),
            ("{s:i,s:i,s:i}", 3),
            ("{s:i!}", 1),
            ("{s:i*}", 1),
            ("{s:i,s:i!}", 2),
            ("{s?i}", 1),
            ("{s?i,s?i}", 2),
            ("{s?i!}", 1),
            ("[i]", 0),
            ("[i,i]", 0),
            ("[i,i,i]", 0),
            ("[i,i,i,i]", 0),
            ("[i!]", 0),
            ("[i*]", 0),
            ("[i,i!]", 0),
            ("[!]", 0),
            ("[*]", 0),
            ("{!}", 0),
            ("{*}", 0),
        ];
        let ka = cs("a");
        let kb = cs("b");
        let kc = cs("c");
        for doc in docs {
            for (fmt, nkeys) in &fmts {
                for &flags in &[0usize, JSON_STRICT, JSON_VALIDATE_ONLY, JSON_STRICT | JSON_VALIDATE_ONLY] {
                    let (cj, rj) = load_both(doc);
                    let f = cs(fmt);
                    let mut c1: c_int = 0;
                    let mut c2: c_int = 0;
                    let mut c3: c_int = 0;
                    let mut c4: c_int = 0;
                    let mut r1: c_int = 0;
                    let mut r2: c_int = 0;
                    let mut r3: c_int = 0;
                    let mut r4: c_int = 0;
                    let mut ce = JsonError::default();
                    let mut re = JsonError::default();
                    let (crc, rrc) = match nkeys {
                        1 => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), ka.as_ptr(), &mut c1),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), ka.as_ptr(), &mut r1),
                        ),
                        2 => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), ka.as_ptr(), &mut c1, kb.as_ptr(), &mut c2),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), ka.as_ptr(), &mut r1, kb.as_ptr(), &mut r2),
                        ),
                        3 => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), ka.as_ptr(), &mut c1, kb.as_ptr(), &mut c2, kc.as_ptr(), &mut c3),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), ka.as_ptr(), &mut r1, kb.as_ptr(), &mut r2, kc.as_ptr(), &mut r3),
                        ),
                        _ => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), &mut c1, &mut c2, &mut c3, &mut c4),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), &mut r1, &mut r2, &mut r3, &mut r4),
                        ),
                    };
                    let tag = format!("unpack({doc:?}, {fmt:?}, {flags:#x})");
                    assert_eq!(crc, rrc, "{tag} rc");
                    assert_eq!(ce.snapshot(), re.snapshot(), "{tag} error");
                    assert_eq!((c1, c2, c3, c4), (r1, r2, r3, r4), "{tag} outputs");
                    decref(c(), cj);
                    decref(r(), rj);
                }
            }
        }
    }
}

/* ---- ERRORS 221, 222, 223, 238 ---- */

#[test]
fn unpack_argument_errors() {
    unsafe {
        // ERRORS 221: NULL root
        for api in both() {
            let mut e = JsonError::default();
            let f = cs("{}");
            assert_eq!(
                (api.json_unpack_ex)(std::ptr::null_mut(), &mut e, 0, f.as_ptr()),
                -1
            );
            assert_eq!(e.code(), E_NULL_VALUE, "{}", api.tag);
            assert_eq!(e.text_str(), "NULL root value");
            assert_eq!(e.source_str(), "<root>");
        }
        let f = cs("{}");
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        (c().json_unpack_ex)(std::ptr::null_mut(), &mut ce, 0, f.as_ptr());
        (r().json_unpack_ex)(std::ptr::null_mut(), &mut re, 0, f.as_ptr());
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 221");

        // ERRORS 222: NULL / empty format
        let (cj, rj) = load_both("{}");
        for fp in [std::ptr::null::<c_char>(), cs("").as_ptr()] {
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let crc = (c().json_unpack_ex)(cj, &mut ce, 0, fp);
            let rrc = (r().json_unpack_ex)(rj, &mut re, 0, fp);
            assert_eq!(crc, rrc, "ERRORS 222 rc");
            assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 222 error");
            assert_eq!(ce.code(), E_INVALID_ARGUMENT);
        }
        decref(c(), cj);
        decref(r(), rj);

        // ERRORS 238: NULL object key
        let (cj, rj) = load_both(r#"{"a":1}"#);
        let f = cs("{s:i}");
        let mut ci: c_int = 0;
        let mut ri: c_int = 0;
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        let crc = (c().json_unpack_ex)(cj, &mut ce, 0, f.as_ptr(), std::ptr::null::<c_char>(), &mut ci);
        let rrc = (r().json_unpack_ex)(rj, &mut re, 0, f.as_ptr(), std::ptr::null::<c_char>(), &mut ri);
        assert_eq!(crc, rrc, "ERRORS 238 rc");
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 238 error");
        assert_eq!(ce.code(), E_NULL_VALUE);
        decref(c(), cj);
        decref(r(), rj);

        // json_unpack (no error struct)
        for (doc, fmt) in [
            (r#"{"a":1}"#, "{s:i}"),
            (r#"{"a":1}"#, "{s:s}"),
            (r#"[1]"#, "[i]"),
            (r#"[1]"#, "[i,i]"),
        ] {
            let (cj, rj) = load_both(doc);
            let f = cs(fmt);
            let k = cs("a");
            let mut ci: c_int = 0;
            let mut ri: c_int = 0;
            let mut cp: *const c_char = std::ptr::null();
            let mut rp: *const c_char = std::ptr::null();
            let (crc, rrc) = if fmt.contains('s') && fmt.contains(':') && fmt.ends_with("s}") {
                (
                    (c().json_unpack)(cj, f.as_ptr(), k.as_ptr(), &mut cp),
                    (r().json_unpack)(rj, f.as_ptr(), k.as_ptr(), &mut rp),
                )
            } else if fmt.starts_with('{') {
                (
                    (c().json_unpack)(cj, f.as_ptr(), k.as_ptr(), &mut ci),
                    (r().json_unpack)(rj, f.as_ptr(), k.as_ptr(), &mut ri),
                )
            } else {
                (
                    (c().json_unpack)(cj, f.as_ptr(), &raw mut ci, &raw mut ci),
                    (r().json_unpack)(rj, f.as_ptr(), &raw mut ri, &raw mut ri),
                )
            };
            assert_eq!(crc, rrc, "json_unpack({doc:?}, {fmt:?})");
            assert_eq!(ci, ri);
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}

/* ---- ERRORS 225, 228-234, 241: wrong-type messages for every type ---- */

#[test]
fn unpack_wrong_type_messages() {
    let _g = dtoa_guard();
    unsafe {
        let values: Vec<&str> = vec![
            r#"{"v":{}}"#,
            r#"{"v":[]}"#,
            r#"{"v":"s"}"#,
            r#"{"v":1}"#,
            r#"{"v":1.5}"#,
            r#"{"v":true}"#,
            r#"{"v":false}"#,
            r#"{"v":null}"#,
        ];
        // one row per unpack format char that type-checks
        let specs: Vec<&str> = vec![
            "{s:s}", "{s:i}", "{s:I}", "{s:b}", "{s:f}", "{s:F}", "{s:n}",
            "{s:{}}", "{s:[]}", "{s:o}", "{s:O}",
        ];
        let k = cs("v");
        for doc in &values {
            for spec in &specs {
                for &flags in &[0usize, JSON_VALIDATE_ONLY] {
                    let (cj, rj) = load_both(doc);
                    let f = cs(spec);
                    let mut ci: c_int = 0;
                    let mut ri: c_int = 0;
                    let mut cii: c_longlong = 0;
                    let mut rii: c_longlong = 0;
                    let mut cd = 0.0f64;
                    let mut rd = 0.0f64;
                    let mut cp: *const c_char = std::ptr::null();
                    let mut rp: *const c_char = std::ptr::null();
                    let mut cjp: *mut JsonT = std::ptr::null_mut();
                    let mut rjp: *mut JsonT = std::ptr::null_mut();
                    let mut ce = JsonError::default();
                    let mut re = JsonError::default();
                    let (crc, rrc) = match *spec {
                        "{s:s}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), k.as_ptr(), &mut cp),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), k.as_ptr(), &mut rp),
                        ),
                        "{s:i}" | "{s:b}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), k.as_ptr(), &mut ci),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), k.as_ptr(), &mut ri),
                        ),
                        "{s:I}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), k.as_ptr(), &mut cii),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), k.as_ptr(), &mut rii),
                        ),
                        "{s:f}" | "{s:F}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), k.as_ptr(), &mut cd),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), k.as_ptr(), &mut rd),
                        ),
                        "{s:o}" | "{s:O}" => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), k.as_ptr(), &mut cjp),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), k.as_ptr(), &mut rjp),
                        ),
                        _ => (
                            (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), k.as_ptr()),
                            (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), k.as_ptr()),
                        ),
                    };
                    let tag = format!("unpack({doc:?}, {spec:?}, {flags:#x})");
                    assert_eq!(crc, rrc, "{tag} rc");
                    assert_eq!(ce.snapshot(), re.snapshot(), "{tag} error");
                    assert_eq!((ci, cii), (ri, rii), "{tag} ints");
                    assert_eq!(cd.to_bits(), rd.to_bits(), "{tag} double");
                    decref(c(), cj);
                    decref(r(), rj);
                }
            }
        }
        // ERRORS 234/241: root type mismatch at the top level
        for doc in &values {
            for spec in ["[i]", "{s:i}", "s", "i", "b", "f", "n", "o"] {
                let (cj, rj) = load_both(doc);
                let f = cs(spec);
                let mut ci: c_int = 0;
                let mut ri: c_int = 0;
                let mut ce = JsonError::default();
                let mut re = JsonError::default();
                let crc = (c().json_unpack_ex)(cj, &mut ce, 0, f.as_ptr(), k.as_ptr(), &mut ci);
                let rrc = (r().json_unpack_ex)(rj, &mut re, 0, f.as_ptr(), k.as_ptr(), &mut ri);
                assert_eq!(crc, rrc, "root unpack({doc:?}, {spec:?}) rc");
                assert_eq!(ce.snapshot(), re.snapshot(), "root unpack({doc:?}, {spec:?})");
                decref(c(), cj);
                decref(r(), rj);
            }
        }
    }
}

/* ---- ERRORS 245: array index out of range with long formats ---- */

#[test]
fn unpack_array_index_out_of_range() {
    unsafe {
        for n in 0usize..6 {
            let doc = format!("[{}]", (0..n).map(|i| i.to_string()).collect::<Vec<_>>().join(","));
            for m in 0usize..8 {
                let fmt = format!("[{}]", vec!["i"; m].join(","));
                for &flags in &[0usize, JSON_STRICT, JSON_VALIDATE_ONLY] {
                    let (cj, rj) = load_both(&doc);
                    let f = cs(&fmt);
                    let mut cv = [c_int_zero(); 8];
                    let mut rv = [c_int_zero(); 8];
                    let mut ce = JsonError::default();
                    let mut re = JsonError::default();
                    let cp0 = cv.as_mut_ptr();
                    let rp0 = rv.as_mut_ptr();
                    let crc = (c().json_unpack_ex)(
                        cj, &mut ce, flags, f.as_ptr(),
                        cp0, cp0.add(1), cp0.add(2), cp0.add(3),
                        cp0.add(4), cp0.add(5), cp0.add(6), cp0.add(7),
                    );
                    let rrc = (r().json_unpack_ex)(
                        rj, &mut re, flags, f.as_ptr(),
                        rp0, rp0.add(1), rp0.add(2), rp0.add(3),
                        rp0.add(4), rp0.add(5), rp0.add(6), rp0.add(7),
                    );
                    let tag = format!("unpack({doc:?}, {fmt:?}, {flags:#x})");
                    assert_eq!(crc, rrc, "{tag} rc");
                    assert_eq!(ce.snapshot(), re.snapshot(), "{tag} error");
                    assert_eq!(cv, rv, "{tag} outputs");
                    decref(c(), cj);
                    decref(r(), rj);
                }
            }
        }
    }
}

fn c_int_zero() -> c_int {
    0
}

/* ---- ERRORS 240: "N object item(s) left unpacked: keys" message ---- */

#[test]
fn unpack_strict_unrecognized_keys_message() {
    unsafe {
        let docs = [
            r#"{"a":1,"b":2}"#,
            r#"{"a":1,"b":2,"c":3}"#,
            r#"{"a":1,"zzzzzzzzzzzzzzzzzzzzzzzzzzzzz":2}"#,
            // many extra keys => a long, comma-joined list
            &format!(
                "{{{}}}",
                (0..30)
                    .map(|i| format!("\"k{i}\":{i}"))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        ];
        for doc in docs {
            for fmt in ["{s:i!}", "{s:i}", "{s?i!}"] {
                for &flags in &[0usize, JSON_STRICT] {
                    let (cj, rj) = load_both(doc);
                    let f = cs(fmt);
                    let k = cs(if doc.contains("\"a\"") { "a" } else { "k0" });
                    let mut ci: c_int = 0;
                    let mut ri: c_int = 0;
                    let mut ce = JsonError::default();
                    let mut re = JsonError::default();
                    let crc = (c().json_unpack_ex)(cj, &mut ce, flags, f.as_ptr(), k.as_ptr(), &mut ci);
                    let rrc = (r().json_unpack_ex)(rj, &mut re, flags, f.as_ptr(), k.as_ptr(), &mut ri);
                    let tag = format!("unpack({doc:?}, {fmt:?}, {flags:#x})");
                    assert_eq!(crc, rrc, "{tag} rc");
                    assert_eq!(ce.snapshot(), re.snapshot(), "{tag} error");
                    decref(c(), cj);
                    decref(r(), rj);
                }
            }
        }
    }
}

/* ---- pack -> unpack round trip on randomized shapes ---- */

#[test]
fn pack_unpack_roundtrip_randomized() {
    let _g = dtoa_guard();
    unsafe {
        let mut rng = Rng::new(0xBEEF_0007);
        for trial in 0..2000 {
            let n = 1 + rng.below(5);
            let fmt = format!("[{}]", vec!["i"; n].join(","));
            let vals: Vec<c_int> = (0..n).map(|_| rng.next_u32() as c_int).collect();
            let f = cs(&fmt);
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let (cj, rj) = match n {
                1 => (
                    (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), vals[0]),
                    (r().json_pack_ex)(&mut re, 0, f.as_ptr(), vals[0]),
                ),
                2 => (
                    (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), vals[0], vals[1]),
                    (r().json_pack_ex)(&mut re, 0, f.as_ptr(), vals[0], vals[1]),
                ),
                3 => (
                    (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), vals[0], vals[1], vals[2]),
                    (r().json_pack_ex)(&mut re, 0, f.as_ptr(), vals[0], vals[1], vals[2]),
                ),
                4 => (
                    (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), vals[0], vals[1], vals[2], vals[3]),
                    (r().json_pack_ex)(&mut re, 0, f.as_ptr(), vals[0], vals[1], vals[2], vals[3]),
                ),
                _ => (
                    (c().json_pack_ex)(&mut ce, 0, f.as_ptr(), vals[0], vals[1], vals[2], vals[3], vals[4]),
                    (r().json_pack_ex)(&mut re, 0, f.as_ptr(), vals[0], vals[1], vals[2], vals[3], vals[4]),
                ),
            };
            assert_eq!(
                pobs(c(), cj, &ce),
                pobs(r(), rj, &re),
                "trial {trial}: pack({fmt:?})"
            );
            if cj.is_null() {
                continue;
            }
            assert_bytes_eq(
                &format!("trial {trial}: dumps of packed"),
                &dumps(c(), cj, JSON_COMPACT),
                &dumps(r(), rj, JSON_COMPACT),
            );
            // unpack it back
            let mut cout = [c_int_zero(); 8];
            let mut rout = [c_int_zero(); 8];
            let mut ce2 = JsonError::default();
            let mut re2 = JsonError::default();
            let co0 = cout.as_mut_ptr();
            let ro0 = rout.as_mut_ptr();
            let crc = (c().json_unpack_ex)(
                cj, &mut ce2, JSON_STRICT, f.as_ptr(),
                co0, co0.add(1), co0.add(2), co0.add(3), co0.add(4),
            );
            let rrc = (r().json_unpack_ex)(
                rj, &mut re2, JSON_STRICT, f.as_ptr(),
                ro0, ro0.add(1), ro0.add(2), ro0.add(3), ro0.add(4),
            );
            assert_eq!(crc, rrc, "trial {trial}: unpack rc");
            assert_eq!(ce2.snapshot(), re2.snapshot(), "trial {trial}: unpack error");
            assert_eq!(cout, rout, "trial {trial}: unpack outputs");
            assert_eq!(&cout[..n], &vals[..], "trial {trial}: round-trip values");
            decref(c(), cj);
            decref(r(), rj);
        }
    }
}
