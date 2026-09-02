//! Phase B/C — error.c, version.c, and the generic cross-FFI boundaries.
//! CONFIGS rows 106, 107 · ERRORS rows 194-200, 247-252.
mod common;
use common::*;
use std::ffi::{c_char, c_int};

/* ---------- CONFIGS 106 · ERRORS 194-200 ---------- */

#[test]
fn jsonp_error_init_and_set_source() {
    unsafe {
        let mut sources: Vec<String> = vec![
            "".into(),
            "a".into(),
            "<string>".into(),
            "x".repeat(78),
            "x".repeat(79), // length == JSON_ERROR_SOURCE_LENGTH - 1
            "x".repeat(80), // length == JSON_ERROR_SOURCE_LENGTH  => truncation
            "x".repeat(81),
            "x".repeat(200),
            "/very/long/path/".repeat(30),
        ];
        let mut rng = Rng::new(0xE7_0001);
        for _ in 0..400 {
            let n = rng.below(220);
            sources.push(rng.key(1).repeat(n.max(1)));
        }
        for src in &sources {
            let sc = cs(src);
            // ERRORS 196: >= 80 chars => "..." + tail
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            (c().jsonp_error_init)(&mut ce, sc.as_ptr());
            (r().jsonp_error_init)(&mut re, sc.as_ptr());
            assert_eq!(
                ce.snapshot(),
                re.snapshot(),
                "jsonp_error_init(source len {})",
                src.len()
            );
            // jsonp_error_set_source on its own
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            (c().jsonp_error_set_source)(&mut ce, sc.as_ptr());
            (r().jsonp_error_set_source)(&mut re, sc.as_ptr());
            assert_eq!(
                ce.source, re.source,
                "jsonp_error_set_source(len {})",
                src.len()
            );
        }

        // ERRORS 194: NULL error struct is a no-op
        (c().jsonp_error_init)(std::ptr::null_mut(), cs("x").as_ptr());
        (r().jsonp_error_init)(std::ptr::null_mut(), cs("x").as_ptr());
        // ERRORS 195: NULL error or NULL source
        (c().jsonp_error_set_source)(std::ptr::null_mut(), cs("x").as_ptr());
        (r().jsonp_error_set_source)(std::ptr::null_mut(), cs("x").as_ptr());
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        (c().jsonp_error_set_source)(&mut ce, std::ptr::null());
        (r().jsonp_error_set_source)(&mut re, std::ptr::null());
        assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 195: NULL source");

        // NULL source in init => source[0] = '\0'
        let mut ce = JsonError::default();
        let mut re = JsonError::default();
        (c().jsonp_error_init)(&mut ce, std::ptr::null());
        (r().jsonp_error_init)(&mut re, std::ptr::null());
        assert_eq!(ce.snapshot(), re.snapshot(), "jsonp_error_init(NULL source)");
        assert_eq!(ce.line, -1);
        assert_eq!(ce.column, -1);
        assert_eq!(ce.position, 0);
        // ERRORS 200: json_error_code of a freshly initialised error
        assert_eq!(ce.code(), 0x5a, "text[159] untouched by jsonp_error_init");
    }
}

#[test]
fn jsonp_error_set_message_and_code() {
    unsafe {
        let mut msgs: Vec<String> = vec![
            "".into(),
            "short".into(),
            "x".repeat(100),
            "x".repeat(157),
            "x".repeat(158), // exactly fills the vsnprintf limit
            "x".repeat(159),
            "x".repeat(300),
            "with %% percent".into(),
        ];
        let mut rng = Rng::new(0xE7_0002);
        for _ in 0..300 {
            let n = rng.below(320);
            msgs.push("y".repeat(n));
        }
        // Every declared json_error_code, plus out-of-range values (ERRORS 248).
        let codes: Vec<c_int> = (0..18)
            .chain([18, 19, 100, 127, 128, 200, 255, 256, -1, -128, i32::MAX, i32::MIN])
            .collect();
        let pct_s = cs("%s");
        for msg in &msgs {
            let mc = cs(msg);
            for &code in &codes {
                for &(line, col, pos) in &[
                    (0i32, 0i32, 0usize),
                    (-1, -1, 0),
                    (1, 2, 3),
                    (i32::MAX, i32::MIN, usize::MAX),
                    (12345, 6789, 999_999),
                ] {
                    let mut ce = JsonError::zeroed();
                    let mut re = JsonError::zeroed();
                    (c().jsonp_error_set)(
                        &mut ce, line, col, pos, code, pct_s.as_ptr(), mc.as_ptr(),
                    );
                    (r().jsonp_error_set)(
                        &mut re, line, col, pos, code, pct_s.as_ptr(), mc.as_ptr(),
                    );
                    assert_eq!(
                        ce.snapshot(),
                        re.snapshot(),
                        "jsonp_error_set(len={}, code={code}, {line}/{col}/{pos})",
                        msg.len()
                    );
                    // ERRORS 198: a second set must NOT overwrite
                    let before_c = ce.snapshot();
                    let before_r = re.snapshot();
                    let other = cs("SECOND");
                    (c().jsonp_error_set)(&mut ce, 99, 99, 99, 1, pct_s.as_ptr(), other.as_ptr());
                    (r().jsonp_error_set)(&mut re, 99, 99, 99, 1, pct_s.as_ptr(), other.as_ptr());
                    if !msg.is_empty() {
                        assert_eq!(ce.snapshot(), before_c, "C: ERRORS 198");
                        assert_eq!(re.snapshot(), before_r, "RUST: ERRORS 198");
                    }
                    assert_eq!(ce.snapshot(), re.snapshot(), "ERRORS 198 differential");
                }
            }
        }
        // ERRORS 197: NULL error struct
        (c().jsonp_error_set)(
            std::ptr::null_mut(), 1, 1, 1, 0, pct_s.as_ptr(), cs("x").as_ptr(),
        );
        (r().jsonp_error_set)(
            std::ptr::null_mut(), 1, 1, 1, 0, pct_s.as_ptr(), cs("x").as_ptr(),
        );

        // Formats with several conversions, incl. %c of 0 (which writes an
        // embedded NUL that the C keeps writing past).
        let cases: Vec<(&str, Vec<i64>)> = vec![
            ("plain", vec![]),
            ("%d", vec![-5]),
            ("%d and %d", vec![1, 2]),
            ("char '%c'", vec![b'x' as i64]),
            ("char '%c'", vec![0]),
            ("char '%c' then '%c'", vec![0, b'z' as i64]),
            ("hex 0x%x", vec![0xdeadbeefi64]),
            ("%li items", vec![-1234567]),
        ];
        for (fmt, args) in cases {
            let fc = cs(fmt);
            let mut ce = JsonError::zeroed();
            let mut re = JsonError::zeroed();
            match args.len() {
                0 => {
                    (c().jsonp_error_set)(&mut ce, 1, 2, 3, 8, fc.as_ptr());
                    (r().jsonp_error_set)(&mut re, 1, 2, 3, 8, fc.as_ptr());
                }
                1 => {
                    if fmt.contains("%li") {
                        (c().jsonp_error_set)(&mut ce, 1, 2, 3, 8, fc.as_ptr(), args[0] as std::ffi::c_long);
                        (r().jsonp_error_set)(&mut re, 1, 2, 3, 8, fc.as_ptr(), args[0] as std::ffi::c_long);
                    } else {
                        (c().jsonp_error_set)(&mut ce, 1, 2, 3, 8, fc.as_ptr(), args[0] as c_int);
                        (r().jsonp_error_set)(&mut re, 1, 2, 3, 8, fc.as_ptr(), args[0] as c_int);
                    }
                }
                _ => {
                    (c().jsonp_error_set)(
                        &mut ce, 1, 2, 3, 8, fc.as_ptr(), args[0] as c_int, args[1] as c_int,
                    );
                    (r().jsonp_error_set)(
                        &mut re, 1, 2, 3, 8, fc.as_ptr(), args[0] as c_int, args[1] as c_int,
                    );
                }
            }
            assert_eq!(
                ce.snapshot(),
                re.snapshot(),
                "jsonp_error_set({fmt:?}, {args:?})"
            );
        }
    }
}

/* ---------- CONFIGS 107 · ERRORS 252 ---------- */

#[test]
fn version_functions() {
    unsafe {
        let cv = from_cstr((c().jansson_version_str)()).unwrap();
        let rv = from_cstr((r().jansson_version_str)()).unwrap();
        assert_eq!(cv, rv);
        assert_eq!(cv, "2.15.0");
        // the returned pointer must be stable across calls
        assert_eq!(
            (c().jansson_version_str)(),
            (c().jansson_version_str)()
        );
        assert_eq!(
            (r().jansson_version_str)(),
            (r().jansson_version_str)()
        );

        let mut rng = Rng::new(0xE7_0003);
        let mut triples: Vec<(c_int, c_int, c_int)> = vec![
            (2, 15, 0),
            (2, 15, 1),
            (2, 14, 0),
            (2, 16, 0),
            (1, 0, 0),
            (3, 0, 0),
            (0, 0, 0),
            (-1, -1, -1),
            (i32::MAX, i32::MAX, i32::MAX),
            (i32::MIN, i32::MIN, i32::MIN),
            (2, i32::MIN, 0),
            (2, 15, i32::MIN),
            (2, 15, i32::MAX),
        ];
        for _ in 0..3000 {
            triples.push((
                rng.next_u32() as c_int,
                rng.next_u32() as c_int,
                rng.next_u32() as c_int,
            ));
            triples.push((
                rng.range_i64(-5, 6) as c_int,
                rng.range_i64(10, 20) as c_int,
                rng.range_i64(-3, 4) as c_int,
            ));
        }
        for (maj, min, mic) in triples {
            assert_eq!(
                (c().jansson_version_cmp)(maj, min, mic),
                (r().jansson_version_cmp)(maj, min, mic),
                "jansson_version_cmp({maj}, {min}, {mic})"
            );
        }
    }
}

/* ---------- ERRORS 247: out-of-range json_type across the FFI ---------- */

#[test]
fn out_of_range_json_type_everywhere() {
    let _g = dtoa_guard();
    unsafe {
        let bad_types: Vec<c_int> = vec![8, 9, 10, 42, 127, 128, 255, 256, -1, -42, i32::MAX, i32::MIN];
        let mut results = Vec::new();
        for api in both() {
            let mut row: Vec<String> = Vec::new();
            for &bad in &bad_types {
                let p = (api.json_integer)(1);
                let q = (api.json_integer)(1);
                (*p).type_ = bad;
                (*q).type_ = bad;

                // json_equal
                row.push(format!("eq={}", (api.json_equal)(p, q)));
                let ok = (api.json_integer)(1);
                row.push(format!("eq_mixed={}", (api.json_equal)(p, ok)));
                row.push(format!("eq_mixed2={}", (api.json_equal)(ok, p)));
                // json_copy / json_deep_copy / do_deep_copy
                row.push(format!("copy_null={}", (api.json_copy)(p).is_null()));
                row.push(format!("deep_null={}", (api.json_deep_copy)(p).is_null()));
                // json_dumps with and without ENCODE_ANY
                row.push(format!("dump_any={:?}", dumps(api, p, JSON_ENCODE_ANY)));
                row.push(format!("dump_plain={:?}", dumps(api, p, 0)));
                // type predicates: every accessor must fall through
                row.push(format!("osize={}", (api.json_object_size)(p)));
                row.push(format!("asize={}", (api.json_array_size)(p)));
                row.push(format!("sval={}", (api.json_string_value)(p).is_null()));
                row.push(format!("slen={}", (api.json_string_length)(p)));
                row.push(format!("ival={}", (api.json_integer_value)(p)));
                row.push(format!("rval={}", (api.json_real_value)(p).to_bits()));
                row.push(format!("nval={}", (api.json_number_value)(p).to_bits()));
                row.push(format!("oiter={}", (api.json_object_iter)(p).is_null()));
                row.push(format!("aget={}", (api.json_array_get)(p, 0).is_null()));
                row.push(format!("oget={}", (api.json_object_get)(p, cs("k").as_ptr()).is_null()));
                row.push(format!("oclear={}", (api.json_object_clear)(p)));
                row.push(format!("aclear={}", (api.json_array_clear)(p)));
                row.push(format!("iset={}", (api.json_integer_set)(p, 1)));
                row.push(format!("rset={}", (api.json_real_set)(p, 1.0)));
                row.push(format!("sset={}", (api.json_string_set)(p, cs("x").as_ptr())));
                // json_delete must be a no-op for an unknown type
                (api.json_delete)(p);
                // nested inside a valid container
                let arr = (api.json_array)();
                (*p).type_ = JSON_INTEGER;
                (api.json_array_append_new)(arr, p);
                (*p).type_ = bad;
                row.push(format!("nested_dump={:?}", dumps(api, arr, 0)));
                row.push(format!("nested_deep={}", (api.json_deep_copy)(arr).is_null()));
                row.push(format!("nested_copy={}", (api.json_copy)(arr).is_null()));
                (*p).type_ = JSON_INTEGER;
                decref(api, arr);
                (*q).type_ = JSON_INTEGER;
                decref(api, q);
                decref(api, ok);
            }
            results.push(row);
        }
        assert_eq!(
            results[0], results[1],
            "ERRORS 247: out-of-range json_type must behave identically"
        );
    }
}

/* ---------- ERRORS 249, 250: unknown / oversized flag bits ---------- */

#[test]
fn unknown_and_oversized_flag_bits() {
    let _g = dtoa_guard();
    unsafe {
        let mut rng = Rng::new(0xE7_0004);
        let doc = cs(r#"{"b":2,"a":[1,2.5,"x"],"c":{"d":null}}"#);
        for trial in 0..3000 {
            // Fully random 64-bit flag words for BOTH the decoder and encoder.
            let dflags = rng.next_u64() as usize;
            let eflags = rng.next_u64() as usize;
            let mut ce = JsonError::default();
            let mut re = JsonError::default();
            let cj = (c().json_loads)(doc.as_ptr(), dflags, &mut ce);
            let rj = (r().json_loads)(doc.as_ptr(), dflags, &mut re);
            assert_eq!(cj.is_null(), rj.is_null(), "trial {trial} loads({dflags:#x})");
            assert_eq!(ce.snapshot(), re.snapshot(), "trial {trial} loads error");
            if cj.is_null() {
                continue;
            }
            assert_eq!(shape(c(), cj), shape(r(), rj), "trial {trial} shape");
            assert_bytes_eq(
                &format!("trial {trial} dumps({eflags:#x})"),
                &dumps(c(), cj, eflags),
                &dumps(r(), rj, eflags),
            );
            // pack / unpack flags too
            let pf = cs("{s:i}");
            let k = cs("k");
            let mut ce2 = JsonError::default();
            let mut re2 = JsonError::default();
            let cp = (c().json_pack_ex)(&mut ce2, eflags, pf.as_ptr(), k.as_ptr(), 1i32);
            let rp = (r().json_pack_ex)(&mut re2, eflags, pf.as_ptr(), k.as_ptr(), 1i32);
            assert_eq!(cp.is_null(), rp.is_null(), "trial {trial} pack({eflags:#x})");
            assert_eq!(ce2.snapshot(), re2.snapshot(), "trial {trial} pack error");
            if !cp.is_null() {
                assert_eq!(shape(c(), cp), shape(r(), rp));
            }
            decref(c(), cp);
            decref(r(), rp);
            decref(c(), cj);
            decref(r(), rj);
        }
        // ERRORS 250: indent / precision above their 5-bit masks
        let (cj, rj) = {
            let cj = (c().json_loads)(doc.as_ptr(), 0, std::ptr::null_mut());
            let rj = (r().json_loads)(doc.as_ptr(), 0, std::ptr::null_mut());
            (cj, rj)
        };
        for n in 0usize..=64 {
            for f in [json_indent(n), json_real_precision(n), n & 0x1F, (n & 0x1F) << 11] {
                assert_bytes_eq(
                    &format!("indent/precision n={n} f={f:#x}"),
                    &dumps(c(), cj, f),
                    &dumps(r(), rj, f),
                );
            }
            // raw (unmasked) values in the low bits
            assert_bytes_eq(
                &format!("raw low bits n={n}"),
                &dumps(c(), cj, n),
                &dumps(r(), rj, n),
            );
            assert_bytes_eq(
                &format!("raw precision bits n={n}"),
                &dumps(c(), cj, n << 11),
                &dumps(r(), rj, n << 11),
            );
        }
        decref(c(), cj);
        decref(r(), rj);
    }
}

/* ---------- ERRORS 251: utf8_encode extreme codepoints ---------- */

#[test]
fn utf8_encode_extremes() {
    unsafe {
        for cp in [
            i32::MIN,
            i32::MIN + 1,
            -1,
            0,
            0x10_FFFF,
            0x11_0000,
            0x7FFF_FFFF,
            i32::MAX,
        ] {
            let mut cb = [0i8; 8];
            let mut rb = [0i8; 8];
            let mut cn = 0xdeadusize;
            let mut rn = 0xdeadusize;
            let cv = (c().utf8_encode)(cp, cb.as_mut_ptr(), &mut cn);
            let rv = (r().utf8_encode)(cp, rb.as_mut_ptr(), &mut rn);
            assert_eq!(cv, rv, "utf8_encode({cp})");
            assert_eq!(cn, rn, "utf8_encode({cp}) size");
            assert_eq!(cb, rb, "utf8_encode({cp}) buffer");
        }
        let _ = std::mem::size_of::<*const c_char>();
    }
}
