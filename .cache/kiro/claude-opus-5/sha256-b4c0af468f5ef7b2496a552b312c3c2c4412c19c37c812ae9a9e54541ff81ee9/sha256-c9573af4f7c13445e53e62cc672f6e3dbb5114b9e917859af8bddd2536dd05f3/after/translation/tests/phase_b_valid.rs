//! Phase B — valid-path differential tests, part 1: parse + print pipeline.
//! Covers CONFIGS.md rows 1-35, 93.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

/// Offset of `cJSON_GetErrorPtr()` relative to the buffer we handed in.
unsafe fn error_offset(api: &Api, base: *const c_char) -> Option<isize> {
    let p = (api.cJSON_GetErrorPtr)();
    if p.is_null() {
        None
    } else {
        Some(p.offset_from(base))
    }
}

/// Render with every print entry point and return all outputs.
unsafe fn render_all(api: &Api, item: *mut CJson) -> Vec<(String, Option<Vec<u8>>)> {
    let mut out = Vec::new();
    out.push((
        "Print".to_string(),
        take_printed(api, (api.cJSON_Print)(item)),
    ));
    out.push((
        "PrintUnformatted".to_string(),
        take_printed(api, (api.cJSON_PrintUnformatted)(item)),
    ));
    for &pre in &[0i32, 1, 2, 8, 255, 256, 257, 4096] {
        for &fmt in &[0i32, 1, 2, -1] {
            out.push((
                format!("PrintBuffered(pre={pre},fmt={fmt})"),
                take_printed(api, (api.cJSON_PrintBuffered)(item, pre, fmt)),
            ));
        }
    }
    out
}

/// Sweep the preallocated-buffer length and record the success/failure pattern
/// plus the bytes written on success.
unsafe fn preallocated_profile(api: &Api, item: *mut CJson, fmt: c_int) -> Vec<(c_int, Option<Vec<u8>>)> {
    let reference = take_printed(api, (api.cJSON_PrintBuffered)(item, 4096, fmt));
    let len = reference.as_ref().map(|v| v.len()).unwrap_or(0) as c_int;
    let mut res = Vec::new();
    let hi = len + 4;
    let lo = if len > 8 { len - 8 } else { 0 };
    for l in lo..=hi {
        let mut buf = vec![0u8; (l as usize) + 64];
        let ok = (api.cJSON_PrintPreallocated)(item, buf.as_mut_ptr() as *mut c_char, l, fmt);
        if ok != 0 {
            let s = std::ffi::CStr::from_ptr(buf.as_ptr() as *const c_char)
                .to_bytes()
                .to_vec();
            res.push((l, Some(s)));
        } else {
            res.push((l, None));
        }
    }
    res
}

/// Full differential check of one JSON input through every parse/print path.
unsafe fn check_json(p: &Pair, input: &[u8], label: &str) {
    let buf = cbytes(input);
    let base = buf.as_ptr();

    // ---- cJSON_Parse -------------------------------------------------
    let ci = (p.c.cJSON_Parse)(base);
    let ri = (p.r.cJSON_Parse)(base);
    assert_eq!(
        ci.is_null(),
        ri.is_null(),
        "[{label}] cJSON_Parse null-ness differs for {:?}",
        String::from_utf8_lossy(input)
    );
    assert_eq!(
        error_offset(p.c, base),
        error_offset(p.r, base),
        "[{label}] cJSON_GetErrorPtr offset differs for {:?}",
        String::from_utf8_lossy(input)
    );

    let cs_ = snapshot(ci);
    let rs_ = snapshot(ri);
    assert!(
        cs_ == rs_,
        "[{label}] parsed tree differs for {:?}\n C: {:?}\n R: {:?}",
        String::from_utf8_lossy(input),
        cs_,
        rs_
    );

    if !ci.is_null() {
        // ---- all print variants -------------------------------------
        let co = render_all(p.c, ci);
        let ro = render_all(p.r, ri);
        assert_eq!(co.len(), ro.len());
        for ((cn, cv), (rn, rv)) in co.iter().zip(ro.iter()) {
            assert_eq!(cn, rn);
            assert!(
                cv == rv,
                "[{label}] {cn} differs for {:?}\n C: {}\n R: {}",
                String::from_utf8_lossy(input),
                show(cv),
                show(rv)
            );
        }

        // ---- cJSON_PrintPreallocated length sweep --------------------
        for &fmt in &[0i32, 1] {
            let cp = preallocated_profile(p.c, ci, fmt);
            let rp = preallocated_profile(p.r, ri, fmt);
            assert!(
                cp == rp,
                "[{label}] PrintPreallocated(fmt={fmt}) sweep differs for {:?}\n C: {:?}\n R: {:?}",
                String::from_utf8_lossy(input),
                cp,
                rp
            );
        }

        // ---- queries ------------------------------------------------
        assert_eq!(
            (p.c.cJSON_GetArraySize)(ci),
            (p.r.cJSON_GetArraySize)(ri),
            "[{label}] GetArraySize differs for {:?}",
            String::from_utf8_lossy(input)
        );
        for pred in [
            ("IsInvalid", p.c.cJSON_IsInvalid, p.r.cJSON_IsInvalid),
            ("IsFalse", p.c.cJSON_IsFalse, p.r.cJSON_IsFalse),
            ("IsTrue", p.c.cJSON_IsTrue, p.r.cJSON_IsTrue),
            ("IsBool", p.c.cJSON_IsBool, p.r.cJSON_IsBool),
            ("IsNull", p.c.cJSON_IsNull, p.r.cJSON_IsNull),
            ("IsNumber", p.c.cJSON_IsNumber, p.r.cJSON_IsNumber),
            ("IsString", p.c.cJSON_IsString, p.r.cJSON_IsString),
            ("IsArray", p.c.cJSON_IsArray, p.r.cJSON_IsArray),
            ("IsObject", p.c.cJSON_IsObject, p.r.cJSON_IsObject),
            ("IsRaw", p.c.cJSON_IsRaw, p.r.cJSON_IsRaw),
        ] {
            assert_eq!(
                (pred.1)(ci),
                (pred.2)(ri),
                "[{label}] {} differs for {:?}",
                pred.0,
                String::from_utf8_lossy(input)
            );
        }
        assert_eq!(
            (p.c.cJSON_GetNumberValue)(ci).to_bits(),
            (p.r.cJSON_GetNumberValue)(ri).to_bits(),
            "[{label}] GetNumberValue bits differ"
        );
        assert_eq!(
            take_cstr((p.c.cJSON_GetStringValue)(ci)),
            take_cstr((p.r.cJSON_GetStringValue)(ri)),
            "[{label}] GetStringValue differs"
        );

        // ---- Duplicate (recurse 0/1/2/-1) + Compare ------------------
        for &rec in &[0i32, 1, 2, -1] {
            let cd = (p.c.cJSON_Duplicate)(ci, rec);
            let rd = (p.r.cJSON_Duplicate)(ri, rec);
            assert_eq!(cd.is_null(), rd.is_null(), "[{label}] Duplicate({rec}) nullness");
            assert!(
                snapshot(cd) == snapshot(rd),
                "[{label}] Duplicate({rec}) differs for {:?}",
                String::from_utf8_lossy(input)
            );
            for &cse in &[0i32, 1, 2, -1] {
                assert_eq!(
                    (p.c.cJSON_Compare)(ci, cd, cse),
                    (p.r.cJSON_Compare)(ri, rd, cse),
                    "[{label}] Compare(orig,dup,{cse}) differs for {:?}",
                    String::from_utf8_lossy(input)
                );
            }
            (p.c.cJSON_Delete)(cd);
            (p.r.cJSON_Delete)(rd);
        }
        for &cse in &[0i32, 1] {
            assert_eq!(
                (p.c.cJSON_Compare)(ci, ci, cse),
                (p.r.cJSON_Compare)(ri, ri, cse),
                "[{label}] Compare(self,self,{cse})"
            );
        }
    }

    (p.c.cJSON_Delete)(ci);
    (p.r.cJSON_Delete)(ri);
}

/* ================================================================== */
/* CONFIGS rows 1, 2, 15-19, 21, 22, 23, 24, 25, 26, 27, 28, 30-35     */
/* ================================================================== */

#[test]
fn row_1_2_15_35_randomized_documents() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0x5EED_0001);
    unsafe {
        for i in 0..600 {
            let json = random_json(&mut rng, 5);
            check_json(p, json.as_bytes(), &format!("rand#{i}"));
        }
    }
}

#[test]
fn row_12_whitespace_sprinkled_documents() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0x5EED_0002);
    unsafe {
        for i in 0..300 {
            let json = random_json(&mut rng, 4);
            let ws = sprinkle_ws(&mut rng, &json);
            check_json(p, ws.as_bytes(), &format!("ws#{i}"));
        }
    }
}

#[test]
fn row_15_16_scalars_and_numbers() {
    let _g = lock();
    let p = pair();
    let mut cases: Vec<String> = vec![
        "null".into(),
        "true".into(),
        "false".into(),
        "\"\"".into(),
        "[]".into(),
        "{}".into(),
    ];
    for n in [
        "0", "-0", "1", "-1", "42", "2147483646", "2147483647", "2147483648", "2147483649",
        "-2147483647", "-2147483648", "-2147483649", "4294967295", "4294967296", "0.0", "-0.0",
        "0.5", "-0.5", "1.5", "1e0", "1e1", "1e10", "1e100", "1e308", "1e309", "-1e309",
        "1e-10", "1e-100", "1e-308", "5e-324", "1e-320", "1e-400", "3.141592653589793",
        "2.718281828459045", "0.1", "0.2", "0.3", "1.0000000000000002", "9007199254740992",
        "9007199254740993", "1.7976931348623157e308", "-1.7976931348623157e308",
        "123456789012345678901234567890", "1E+3", "1E-3", "-1E+3", "12345678901234567",
        "100000000000000000000", "0e0", "-0e0", "0.000000000000000000001",
    ] {
        cases.push(n.to_string());
        cases.push(format!("[{n}]"));
        cases.push(format!("{{\"k\":{n}}}"));
    }
    unsafe {
        for (i, c) in cases.iter().enumerate() {
            check_json(p, c.as_bytes(), &format!("num#{i}:{c}"));
        }
    }
}

#[test]
fn row_17_strings_and_escapes() {
    let _g = lock();
    let p = pair();
    let mut cases: Vec<Vec<u8>> = Vec::new();
    let literals: &[&str] = &[
        r#""""#,
        r#""a""#,
        r#""ab\ncd""#,
        r#""\b\f\n\r\t\"\\\/""#,
        r#""\u0000""#,
        r#""\u0001""#,
        r#""\u001f""#,
        r#""\u0020""#,
        r#""\u007f""#,
        r#""\u0080""#,
        r#""\u07ff""#,
        r#""\u0800""#,
        r#""\uffff""#,
        r#""\ufffe""#,
        r#""\ud800\udc00""#,
        r#""\udbff\udfff""#,
        r#""\uD83D\uDE00""#,
        r#""\u0041\u00e9\u20ac""#,
        r#""mixed \t and \u0007 and plain""#,
        r#""\/\/not a comment""#,
        r#""0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789""#,
    ];
    for l in literals {
        cases.push(l.as_bytes().to_vec());
        cases.push(format!("[{l},{l}]").into_bytes());
        cases.push(format!("{{{l}:{l}}}").into_bytes());
    }
    // raw high bytes inside a string (valid for cJSON: copied verbatim)
    for b in [0x80u8, 0xC3, 0xA9, 0xE2, 0xFF, 0x7F] {
        let mut v = vec![b'"', b'x', b];
        v.extend_from_slice(b"y\"");
        cases.push(v);
    }
    // \u sweep over the BMP (step chosen to hit every 1/2/3-byte UTF-8 class)
    for cp in (0u32..0x10000).step_by(97) {
        if (0xD800..0xE000).contains(&cp) {
            continue;
        }
        cases.push(format!("\"\\u{cp:04x}\"").into_bytes());
    }
    // surrogate pairs
    for hi in (0xD800u32..0xDC00).step_by(151) {
        for lo in (0xDC00u32..0xE000).step_by(157) {
            cases.push(format!("\"\\u{hi:04x}\\u{lo:04x}\"").into_bytes());
        }
    }
    unsafe {
        for (i, c) in cases.iter().enumerate() {
            check_json(p, c, &format!("str#{i}"));
        }
    }
}

#[test]
fn row_18_19_containers() {
    let _g = lock();
    let p = pair();
    let mut cases: Vec<String> = vec![
        "[]".into(),
        "[ ]".into(),
        "[1]".into(),
        "[1,2]".into(),
        "[1,2,3,4,5,6,7,8,9,10]".into(),
        "[null,true,false,0,\"\",[],{}]".into(),
        "{}".into(),
        "{ }".into(),
        "{\"a\":1}".into(),
        "{\"a\":1,\"b\":2}".into(),
        "{\"\":1}".into(),
        "{\"a\":1,\"a\":2}".into(),
        "{\"a\":1,\"A\":2}".into(),
        "{\"A\":1,\"a\":2}".into(),
        "{\"key\":{\"key\":{\"key\":[1,[2,[3]]]}}}".into(),
        "[[[[[[[[[[1]]]]]]]]]]".into(),
        "{\"a\":[{\"b\":[{\"c\":1}]}]}".into(),
    ];
    // wide containers force several `ensure` growth steps
    let wide_arr: String = format!(
        "[{}]",
        (0..200).map(|i| i.to_string()).collect::<Vec<_>>().join(",")
    );
    let wide_obj: String = format!(
        "{{{}}}",
        (0..200)
            .map(|i| format!("\"k{i}\":{i}"))
            .collect::<Vec<_>>()
            .join(",")
    );
    cases.push(wide_arr);
    cases.push(wide_obj);
    unsafe {
        for (i, c) in cases.iter().enumerate() {
            check_json(p, c.as_bytes(), &format!("cont#{i}"));
        }
    }
}

#[test]
fn row_20_nesting_limit() {
    let _g = lock();
    let p = pair();
    unsafe {
        for &open in &['[', '{'] {
            for &depth in &[1usize, 2, 3, 500, 997, 998, 999, 1000, 1001, 1002, 2000] {
                let mut s = String::new();
                for _ in 0..depth {
                    if open == '[' {
                        s.push('[');
                    } else {
                        s.push_str("{\"a\":");
                    }
                }
                s.push('1');
                for _ in 0..depth {
                    if open == '[' {
                        s.push(']');
                    } else {
                        s.push('}');
                    }
                }
                let buf = cbytes(s.as_bytes());
                let ci = (p.c.cJSON_Parse)(buf.as_ptr());
                let ri = (p.r.cJSON_Parse)(buf.as_ptr());
                assert_eq!(
                    ci.is_null(),
                    ri.is_null(),
                    "nesting depth {depth} ('{open}') nullness differs (C null={}, R null={})",
                    ci.is_null(),
                    ri.is_null()
                );
                assert_eq!(
                    error_offset(p.c, buf.as_ptr()),
                    error_offset(p.r, buf.as_ptr()),
                    "nesting depth {depth} ('{open}') error offset differs"
                );
                if !ci.is_null() {
                    let co = take_printed(p.c, (p.c.cJSON_PrintUnformatted)(ci));
                    let ro = take_printed(p.r, (p.r.cJSON_PrintUnformatted)(ri));
                    assert!(co == ro, "nesting depth {depth} print differs");
                }
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
    }
}

#[test]
fn row_13_utf8_bom() {
    let _g = lock();
    let p = pair();
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for tail in ["", "1", "[1,2]", "{\"a\":1}", "null", " [1] ", "\"x\""] {
        let mut v = vec![0xEFu8, 0xBB, 0xBF];
        v.extend_from_slice(tail.as_bytes());
        cases.push(v);
    }
    // partial / malformed BOMs
    cases.push(vec![0xEF, 0xBB, 0xBF]);
    cases.push(vec![0xEF, 0xBB]);
    cases.push(vec![0xEF]);
    cases.push(vec![0xEF, 0xBB, 0xBF, b'1']);
    cases.push(vec![0xEF, 0xBB, 0xBE, b'1']);
    cases.push(vec![0xFE, 0xFF, b'1']);
    unsafe {
        for (i, c) in cases.iter().enumerate() {
            check_json(p, c, &format!("bom#{i}"));
        }
    }
}

/* ================================================================== */
/* CONFIGS rows 3-11: the low-level parse entry points                 */
/* ================================================================== */

unsafe fn check_parse_with_length_opts(
    p: &Pair,
    input: &[u8],
    len: usize,
    want_end: bool,
    rnt: c_int,
    label: &str,
) {
    let buf = cbytes(input);
    let base = buf.as_ptr();
    let mut cend: *const c_char = std::ptr::null();
    let mut rend: *const c_char = std::ptr::null();
    let cend_p = if want_end { &mut cend as *mut _ } else { std::ptr::null_mut() };
    let rend_p = if want_end { &mut rend as *mut _ } else { std::ptr::null_mut() };

    let ci = (p.c.cJSON_ParseWithLengthOpts)(base, len, cend_p, rnt);
    let ri = (p.r.cJSON_ParseWithLengthOpts)(base, len, rend_p, rnt);

    assert_eq!(
        ci.is_null(),
        ri.is_null(),
        "[{label}] nullness differs (len={len}, end={want_end}, rnt={rnt}) for {:?}",
        String::from_utf8_lossy(input)
    );
    assert!(
        snapshot(ci) == snapshot(ri),
        "[{label}] tree differs (len={len}, rnt={rnt}) for {:?}",
        String::from_utf8_lossy(input)
    );
    if want_end {
        let co = if cend.is_null() { None } else { Some(cend.offset_from(base)) };
        let ro = if rend.is_null() { None } else { Some(rend.offset_from(base)) };
        assert_eq!(
            co, ro,
            "[{label}] return_parse_end differs (len={len}, rnt={rnt}) for {:?}",
            String::from_utf8_lossy(input)
        );
    }
    assert_eq!(
        error_offset(p.c, base),
        error_offset(p.r, base),
        "[{label}] error offset differs (len={len}, rnt={rnt}) for {:?}",
        String::from_utf8_lossy(input)
    );
    if !ci.is_null() {
        let co = take_printed(p.c, (p.c.cJSON_Print)(ci));
        let ro = take_printed(p.r, (p.r.cJSON_Print)(ri));
        assert!(co == ro, "[{label}] print differs");
    }
    (p.c.cJSON_Delete)(ci);
    (p.r.cJSON_Delete)(ri);
}

#[test]
fn rows_3_11_parse_entry_points_matrix() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0x5EED_0003);
    let mut corpus: Vec<String> = vec![
        "1".into(),
        "1 ".into(),
        "1 x".into(),
        "1x".into(),
        "[1,2]".into(),
        "[1,2] trailing".into(),
        "{\"a\":1} junk".into(),
        "\"str\"".into(),
        "null".into(),
        "  ".into(),
        "".into(),
        "[".into(),
        "{".into(),
        "[1,".into(),
        "\"unterminated".into(),
    ];
    for _ in 0..40 {
        corpus.push(random_json(&mut rng, 3));
    }
    unsafe {
        for (i, c) in corpus.iter().enumerate() {
            let bytes = c.as_bytes();
            // ParseWithOpts: end-ptr x require_null_terminated (incl. out-of-range)
            for &rnt in &[0i32, 1, 2, -1, i32::MAX, i32::MIN] {
                for &want_end in &[false, true] {
                    let buf = cbytes(bytes);
                    let base = buf.as_ptr();
                    let mut cend: *const c_char = std::ptr::null();
                    let mut rend: *const c_char = std::ptr::null();
                    let cp = if want_end { &mut cend as *mut _ } else { std::ptr::null_mut() };
                    let rp = if want_end { &mut rend as *mut _ } else { std::ptr::null_mut() };
                    let ci = (p.c.cJSON_ParseWithOpts)(base, cp, rnt);
                    let ri = (p.r.cJSON_ParseWithOpts)(base, rp, rnt);
                    assert_eq!(
                        ci.is_null(),
                        ri.is_null(),
                        "opts#{i} nullness (rnt={rnt}, end={want_end}) for {c:?}"
                    );
                    assert!(snapshot(ci) == snapshot(ri), "opts#{i} tree for {c:?}");
                    if want_end {
                        let co = if cend.is_null() { None } else { Some(cend.offset_from(base)) };
                        let ro = if rend.is_null() { None } else { Some(rend.offset_from(base)) };
                        assert_eq!(co, ro, "opts#{i} end ptr (rnt={rnt}) for {c:?}");
                    }
                    assert_eq!(
                        error_offset(p.c, base),
                        error_offset(p.r, base),
                        "opts#{i} error offset (rnt={rnt}) for {c:?}"
                    );
                    (p.c.cJSON_Delete)(ci);
                    (p.r.cJSON_Delete)(ri);
                }
            }

            // ParseWithLength / ParseWithLengthOpts: every truncation point plus
            // exact, exact+1 and over-long lengths.
            let n = bytes.len();
            let mut lens: Vec<usize> = (0..=n + 2).collect();
            lens.dedup();
            for &len in &lens {
                let buf = cbytes(bytes);
                let ci = (p.c.cJSON_ParseWithLength)(buf.as_ptr(), len);
                let ri = (p.r.cJSON_ParseWithLength)(buf.as_ptr(), len);
                assert_eq!(
                    ci.is_null(),
                    ri.is_null(),
                    "len#{i} nullness (len={len}) for {c:?}"
                );
                assert!(snapshot(ci) == snapshot(ri), "len#{i} tree (len={len}) for {c:?}");
                assert_eq!(
                    error_offset(p.c, buf.as_ptr()),
                    error_offset(p.r, buf.as_ptr()),
                    "len#{i} error offset (len={len}) for {c:?}"
                );
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);

                for &rnt in &[0i32, 1, 2] {
                    for &want_end in &[false, true] {
                        check_parse_with_length_opts(
                            p, bytes, len, want_end, rnt, &format!("lopts#{i}"),
                        );
                    }
                }
            }
        }
    }
}

/* ================================================================== */
/* CONFIGS row 14: error pointer for every prefix of a corpus          */
/* ================================================================== */

#[test]
fn row_14_error_pointer_for_every_prefix() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(0x5EED_0004);
    let mut corpus: Vec<String> = vec![
        "{\"a\":[1,2,{\"b\":\"c\"}],\"d\":null}".into(),
        "[1,2,3]".into(),
        "\"abc\\u0041def\"".into(),
        "{\"x\":1e10,\"y\":-2}".into(),
    ];
    for _ in 0..20 {
        corpus.push(random_json(&mut rng, 3));
    }
    unsafe {
        for (i, doc) in corpus.iter().enumerate() {
            let b = doc.as_bytes();
            for cut in 0..=b.len() {
                let buf = cbytes(&b[..cut]);
                let base = buf.as_ptr();
                let ci = (p.c.cJSON_Parse)(base);
                let ri = (p.r.cJSON_Parse)(base);
                assert_eq!(ci.is_null(), ri.is_null(), "prefix#{i}/{cut} nullness");
                assert_eq!(
                    error_offset(p.c, base),
                    error_offset(p.r, base),
                    "prefix#{i}/{cut} error offset for {:?}",
                    String::from_utf8_lossy(&b[..cut])
                );
                assert!(snapshot(ci) == snapshot(ri), "prefix#{i}/{cut} tree");
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
    }
}

/* ================================================================== */
/* CONFIGS rows 28, 29, 31, 33, 34, 83: printing hand-built items      */
/* ================================================================== */

#[test]
fn rows_28_34_print_handbuilt_items() {
    let _g = lock();
    let p = pair();
    unsafe {
        // every type value, including invalid ones, on a Number-shaped node
        for ty in [
            0i32, 1, 2, 3, 4, 5, 8, 9, 16, 32, 64, 128, 255, 256, 257, 512, 0x1FF, 0x100 | 8,
            0x200 | 16, -1, i32::MIN, i32::MAX, 7, 15, 31, 63, 127, 129, 130,
        ] {
            let cn = (p.c.cJSON_CreateNumber)(1.5);
            let rn = (p.r.cJSON_CreateNumber)(1.5);
            (*cn).type_ = ty;
            (*rn).type_ = ty;
            let co = render_all(p.c, cn);
            let ro = render_all(p.r, rn);
            for ((n, cv), (_, rv)) in co.iter().zip(ro.iter()) {
                assert!(cv == rv, "type={ty} {n} differs: C={} R={}", show(cv), show(rv));
            }
            for pred in [
                ("IsInvalid", p.c.cJSON_IsInvalid, p.r.cJSON_IsInvalid),
                ("IsFalse", p.c.cJSON_IsFalse, p.r.cJSON_IsFalse),
                ("IsTrue", p.c.cJSON_IsTrue, p.r.cJSON_IsTrue),
                ("IsBool", p.c.cJSON_IsBool, p.r.cJSON_IsBool),
                ("IsNull", p.c.cJSON_IsNull, p.r.cJSON_IsNull),
                ("IsNumber", p.c.cJSON_IsNumber, p.r.cJSON_IsNumber),
                ("IsString", p.c.cJSON_IsString, p.r.cJSON_IsString),
                ("IsArray", p.c.cJSON_IsArray, p.r.cJSON_IsArray),
                ("IsObject", p.c.cJSON_IsObject, p.r.cJSON_IsObject),
                ("IsRaw", p.c.cJSON_IsRaw, p.r.cJSON_IsRaw),
            ] {
                assert_eq!((pred.1)(cn), (pred.2)(rn), "type={ty} {}", pred.0);
            }
            assert_eq!(
                (p.c.cJSON_GetNumberValue)(cn).to_bits(),
                (p.r.cJSON_GetNumberValue)(rn).to_bits(),
                "type={ty} GetNumberValue"
            );
            assert_eq!(
                (p.c.cJSON_Compare)(cn, cn, 1),
                (p.r.cJSON_Compare)(rn, rn, 1),
                "type={ty} Compare(self)"
            );
            // reset type so Delete frees consistently
            (*cn).type_ = cJSON_Number;
            (*rn).type_ = cJSON_Number;
            (p.c.cJSON_Delete)(cn);
            (p.r.cJSON_Delete)(rn);
        }

        // Raw items
        for raw in ["", "1", "not json at all", "{\"a\":1}", "\u{1}\u{2}", "[[["] {
            let s = cs(raw);
            let cn = (p.c.cJSON_CreateRaw)(s.as_ptr());
            let rn = (p.r.cJSON_CreateRaw)(s.as_ptr());
            let co = render_all(p.c, cn);
            let ro = render_all(p.r, rn);
            for ((n, cv), (_, rv)) in co.iter().zip(ro.iter()) {
                assert!(cv == rv, "raw={raw:?} {n}: C={} R={}", show(cv), show(rv));
            }
            (p.c.cJSON_Delete)(cn);
            (p.r.cJSON_Delete)(rn);
        }

        // String item with valuestring == NULL  -> prints ""
        let cn = (p.c.cJSON_CreateStringReference)(std::ptr::null());
        let rn = (p.r.cJSON_CreateStringReference)(std::ptr::null());
        let co = render_all(p.c, cn);
        let ro = render_all(p.r, rn);
        for ((n, cv), (_, rv)) in co.iter().zip(ro.iter()) {
            assert!(cv == rv, "null strref {n}: C={} R={}", show(cv), show(rv));
        }
        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);

        // Object whose child has string == NULL -> prints "" key
        let cobj = (p.c.cJSON_CreateObject)();
        let robj = (p.r.cJSON_CreateObject)();
        let k = cs("k");
        (p.c.cJSON_AddItemToObject)(cobj, k.as_ptr(), (p.c.cJSON_CreateNumber)(1.0));
        (p.r.cJSON_AddItemToObject)(robj, k.as_ptr(), (p.r.cJSON_CreateNumber)(1.0));
        let cchild = (*cobj).child;
        let rchild = (*robj).child;
        let ckey = (*cchild).string;
        let rkey = (*rchild).string;
        (*cchild).string = std::ptr::null_mut();
        (*rchild).string = std::ptr::null_mut();
        let co = render_all(p.c, cobj);
        let ro = render_all(p.r, robj);
        for ((n, cv), (_, rv)) in co.iter().zip(ro.iter()) {
            assert!(cv == rv, "null key {n}: C={} R={}", show(cv), show(rv));
        }
        (*cchild).string = ckey;
        (*rchild).string = rkey;
        (p.c.cJSON_Delete)(cobj);
        (p.r.cJSON_Delete)(robj);
    }
}

/* ================================================================== */
/* CONFIGS rows 36, 82: CreateNumber / SetNumberHelper value sweeps     */
/* ================================================================== */

fn number_pool() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        1.5,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        f64::EPSILON,
        5e-324,
        i32::MAX as f64,
        i32::MAX as f64 - 1.0,
        i32::MAX as f64 + 1.0,
        i32::MIN as f64,
        i32::MIN as f64 + 1.0,
        i32::MIN as f64 - 1.0,
        2147483647.5,
        -2147483648.5,
        1e300,
        -1e300,
        1e-300,
        3.141592653589793,
        9007199254740993.0,
        1.0000000000000002,
        123456789.125,
    ];
    let mut rng = Rng::new(0x5EED_0005);
    for _ in 0..300 {
        v.push(rng.f64());
    }
    for _ in 0..100 {
        v.push(rng.i32() as f64);
    }
    v
}

#[test]
fn row_36_create_number_sweep() {
    let _g = lock();
    let p = pair();
    unsafe {
        for (i, &d) in number_pool().iter().enumerate() {
            let cn = (p.c.cJSON_CreateNumber)(d);
            let rn = (p.r.cJSON_CreateNumber)(d);
            assert!(!cn.is_null() && !rn.is_null());
            assert_eq!(
                (*cn).valuedouble.to_bits(),
                (*rn).valuedouble.to_bits(),
                "#{i} CreateNumber({d:?}) valuedouble bits"
            );
            assert_eq!(
                (*cn).valueint,
                (*rn).valueint,
                "#{i} CreateNumber({d:?}) valueint (C={}, R={})",
                (*cn).valueint,
                (*rn).valueint
            );
            assert_eq!((*cn).type_, (*rn).type_, "#{i} type");
            let co = render_all(p.c, cn);
            let ro = render_all(p.r, rn);
            for ((n, cv), (_, rv)) in co.iter().zip(ro.iter()) {
                assert!(cv == rv, "#{i} {d:?} {n}: C={} R={}", show(cv), show(rv));
            }
            (p.c.cJSON_Delete)(cn);
            (p.r.cJSON_Delete)(rn);
        }
    }
}

#[test]
fn row_82_set_number_helper_sweep() {
    let _g = lock();
    let p = pair();
    unsafe {
        for (i, &d) in number_pool().iter().enumerate() {
            let cn = (p.c.cJSON_CreateNumber)(7.0);
            let rn = (p.r.cJSON_CreateNumber)(7.0);
            let cr = (p.c.cJSON_SetNumberHelper)(cn, d);
            let rr = (p.r.cJSON_SetNumberHelper)(rn, d);
            assert_eq!(cr.to_bits(), rr.to_bits(), "#{i} SetNumberHelper({d:?}) return");
            assert_eq!(
                (*cn).valueint,
                (*rn).valueint,
                "#{i} SetNumberHelper({d:?}) valueint"
            );
            assert_eq!(
                (*cn).valuedouble.to_bits(),
                (*rn).valuedouble.to_bits(),
                "#{i} SetNumberHelper({d:?}) valuedouble"
            );
            let co = take_printed(p.c, (p.c.cJSON_Print)(cn));
            let ro = take_printed(p.r, (p.r.cJSON_Print)(rn));
            assert!(co == ro, "#{i} print after SetNumberHelper({d:?})");
            (p.c.cJSON_Delete)(cn);
            (p.r.cJSON_Delete)(rn);
        }
    }
}

/* ================================================================== */
/* CONFIGS rows 83, 84: version / malloc / free                        */
/* ================================================================== */

#[test]
fn rows_83_84_version_malloc_free() {
    let _g = lock();
    let p = pair();
    unsafe {
        assert_eq!(
            take_cstr((p.c.cJSON_Version)()),
            take_cstr((p.r.cJSON_Version)())
        );
        for &sz in &[0usize, 1, 7, 4096, 1 << 20] {
            let cm = (p.c.cJSON_malloc)(sz);
            let rm = (p.r.cJSON_malloc)(sz);
            assert_eq!(cm.is_null(), rm.is_null(), "cJSON_malloc({sz}) nullness");
            (p.c.cJSON_free)(cm);
            (p.r.cJSON_free)(rm);
        }
        // cJSON_free(NULL) must be a no-op in both
        (p.c.cJSON_free)(std::ptr::null_mut::<c_void>());
        (p.r.cJSON_free)(std::ptr::null_mut::<c_void>());
    }
}
