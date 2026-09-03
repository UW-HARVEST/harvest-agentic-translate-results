//! Phase C — error-path differential tests. One test (or one loop iteration)
//! per row of ERRORS.md; rows marked **[alloc]** live in `phase_bc_hooks.rs`.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::{null, null_mut};

/// Every `cJSON_bool` value worth pushing across the FFI boundary, including
/// values that correspond to no valid "enum" variant.
const BOOLS: &[c_int] = &[0, 1, 2, -1, 3, 255, 256, 512, i32::MAX, i32::MIN];

/* ================================================================== */
/* rows 1-4, 181-182: accessors and predicates on NULL / wrong types    */
/* ================================================================== */

#[test]
fn rows_1_4_181_182_accessors_and_predicates() {
    let _g = lock();
    let p = pair();
    unsafe {
        // NULL item
        assert_eq!(
            take_cstr((p.c.cJSON_GetStringValue)(null())),
            take_cstr((p.r.cJSON_GetStringValue)(null())),
            "row 1: GetStringValue(NULL)"
        );
        assert!((p.c.cJSON_GetStringValue)(null()).is_null());
        assert_eq!(
            (p.c.cJSON_GetNumberValue)(null()).to_bits(),
            (p.r.cJSON_GetNumberValue)(null()).to_bits(),
            "row 3: GetNumberValue(NULL) NaN bits"
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
                (pred.1)(null()),
                (pred.2)(null()),
                "row 181: {}(NULL)",
                pred.0
            );
        }

        // wrong types, and out-of-range `type` values (rows 2, 4, 182, 197)
        for ty in [
            0i32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 15, 16, 17, 31, 32, 63, 64, 127, 128, 129, 130,
            255, 256, 257, 511, 512, 513, 0x1FF, 0x2FF, -1, -256, i32::MAX, i32::MIN,
            0x100 | 16, 0x200 | 16, 0x300 | 8,
        ] {
            let c = (p.c.cJSON_CreateNumber)(2.25);
            let r = (p.r.cJSON_CreateNumber)(2.25);
            (*c).type_ = ty;
            (*r).type_ = ty;
            assert_eq!(
                take_cstr((p.c.cJSON_GetStringValue)(c)),
                take_cstr((p.r.cJSON_GetStringValue)(r)),
                "row 2: GetStringValue(type={ty})"
            );
            assert_eq!(
                (p.c.cJSON_GetNumberValue)(c).to_bits(),
                (p.r.cJSON_GetNumberValue)(r).to_bits(),
                "row 4: GetNumberValue(type={ty})"
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
                assert_eq!((pred.1)(c), (pred.2)(r), "row 181/182: {}(type={ty})", pred.0);
            }
            // row 83/197: printing an unknown type must fail identically
            for &fmt in &[0i32, 1] {
                let mut cb = vec![0u8; 256];
                let mut rb = vec![0u8; 256];
                assert_eq!(
                    (p.c.cJSON_PrintPreallocated)(c, cb.as_mut_ptr() as *mut c_char, 256, fmt),
                    (p.r.cJSON_PrintPreallocated)(r, rb.as_mut_ptr() as *mut c_char, 256, fmt),
                    "row 83: PrintPreallocated(type={ty},fmt={fmt})"
                );
                assert_eq!(cb, rb, "row 83: buffer contents (type={ty},fmt={fmt})");
            }
            let cp = take_printed(p.c, (p.c.cJSON_Print)(c));
            let rp = take_printed(p.r, (p.r.cJSON_Print)(r));
            assert!(cp == rp, "row 83: Print(type={ty})");
            (*c).type_ = cJSON_Number;
            (*r).type_ = cJSON_Number;
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);
        }
    }
}

/* ================================================================== */
/* row 5: GetErrorPtr with no failed parse                              */
/* ================================================================== */

#[test]
fn row_5_60_error_ptr_state() {
    let _g = lock();
    let p = pair();
    unsafe {
        // A successful parse resets global_error to {NULL, 0}.
        let ok = cbytes(b"[1]");
        (p.c.cJSON_Delete)((p.c.cJSON_Parse)(ok.as_ptr()));
        (p.r.cJSON_Delete)((p.r.cJSON_Parse)(ok.as_ptr()));
        assert_eq!(
            (p.c.cJSON_GetErrorPtr)().is_null(),
            (p.r.cJSON_GetErrorPtr)().is_null(),
            "row 5: GetErrorPtr after success"
        );
        assert!((p.c.cJSON_GetErrorPtr)().is_null());

        // row 59/60: value == NULL leaves global_error untouched
        assert!((p.c.cJSON_Parse)(null()).is_null());
        assert!((p.r.cJSON_Parse)(null()).is_null());
        assert_eq!(
            (p.c.cJSON_GetErrorPtr)().is_null(),
            (p.r.cJSON_GetErrorPtr)().is_null(),
            "row 59: GetErrorPtr after Parse(NULL)"
        );
        assert!((p.c.cJSON_ParseWithLength)(null(), 10).is_null());
        assert!((p.r.cJSON_ParseWithLength)(null(), 10).is_null());
        assert_eq!(
            (p.c.cJSON_GetErrorPtr)().is_null(),
            (p.r.cJSON_GetErrorPtr)().is_null(),
            "row 60: GetErrorPtr after ParseWithLength(NULL)"
        );

        // ParseWithOpts / ParseWithLengthOpts with NULL value and every bool
        for &rnt in BOOLS {
            let mut ce: *const c_char = 0x1 as *const c_char;
            let mut re: *const c_char = 0x1 as *const c_char;
            assert!((p.c.cJSON_ParseWithOpts)(null(), &mut ce, rnt).is_null());
            assert!((p.r.cJSON_ParseWithOpts)(null(), &mut re, rnt).is_null());
            assert_eq!(ce, re, "row 59: return_parse_end untouched (rnt={rnt})");
            assert!((p.c.cJSON_ParseWithLengthOpts)(null(), 5, &mut ce, rnt).is_null());
            assert!((p.r.cJSON_ParseWithLengthOpts)(null(), 5, &mut re, rnt).is_null());
            assert_eq!(ce, re, "row 60: return_parse_end untouched (rnt={rnt})");
        }
    }
}

/* ================================================================== */
/* rows 16, 46-52, 61, 63-66, 78-79, 85-102: parse rejections            */
/* ================================================================== */

#[test]
fn parse_rejections() {
    let _g = lock();
    let p = pair();

    // (label, input) — every distinct rejection derived from the C source.
    let cases: &[(&str, &str)] = &[
        // row 16: strtod consumes nothing
        ("row16 lone minus", "-"),
        ("row16 minus e", "-e"),
        ("row16 minus dot", "-."),
        ("row16 minus plus", "-+"),
        ("row16 minus in array", "[-]"),
        ("row16 minus E", "-E5"),
        // row 46: not a string where a string is required
        ("row46/99 numeric key", "{1:2}"),
        ("row46/99 missing key", "{:1}"),
        ("row46/99 comma key", "{,}"),
        ("row46/99 bare key", "{a:1}"),
        // row 47: buffer ends with a backslash
        ("row47 trailing backslash", "\"ab\\"),
        ("row47 only backslash", "\"\\"),
        // row 48: unterminated string
        ("row48 unterminated", "\"abc"),
        ("row48 unterminated empty", "\""),
        ("row48 in array", "[\"abc]"),
        // row 51: unknown escapes
        ("row51 \\q", "\"\\q\""),
        ("row51 \\x", "\"\\x41\""),
        ("row51 \\a", "\"\\a\""),
        ("row51 \\0", "\"\\0\""),
        ("row51 \\v", "\"\\v\""),
        ("row51 \\U", "\"\\U0041\""),
        ("row51 \\1", "\"\\1\""),
        // rows 40-45 / 52: \u failures
        ("row40 truncated u", "\"\\u\""),
        ("row40 truncated u1", "\"\\u0\""),
        ("row40 truncated u2", "\"\\u00\""),
        ("row40 truncated u3", "\"\\u004\""),
        ("row39 non-hex", "\"\\u00g1\""),
        ("row39 non-hex2", "\"\\uzzzz\""),
        ("row41 lone low surrogate", "\"\\udc00\""),
        ("row41 lone low surrogate hi", "\"\\udfff\""),
        ("row42 high surrogate truncated", "\"\\ud800\\u\""),
        ("row42 high surrogate alone", "\"\\ud800\""),
        ("row43 second not \\u", "\"\\ud800AAAAAA\""),
        ("row43 second escape", "\"\\ud800\\n0000\""),
        ("row44 second out of range", "\"\\ud800\\u0041\""),
        ("row44 second is high", "\"\\ud800\\ud800\""),
        // row 79: no token matches
        ("row79 letter", "x"),
        ("row79 plus", "+1"),
        ("row79 dot", ".5"),
        ("row79 single quote", "'a'"),
        ("row79 nul", "nul"),
        ("row79 tru", "tru"),
        ("row79 fals", "fals"),
        ("row79 NULL upper", "NULL"),
        ("row79 True upper", "True"),
        ("row79 empty", ""),
        ("row79 ws only", "   "),
        ("row79 close bracket", "]"),
        ("row79 close brace", "}"),
        ("row79 colon", ":"),
        ("row79 comma", ","),
        // rows 86-89: array rejections
        ("row87 open only", "["),
        ("row87 open ws", "[ "),
        ("row87 open nl", "[\n\t"),
        ("row88 empty element", "[,]"),
        ("row88 trailing comma", "[1,]"),
        ("row88 leading comma", "[,1]"),
        ("row88 bad element", "[x]"),
        ("row89 unterminated", "[1,2"),
        ("row89 missing comma", "[1 2]"),
        ("row89 wrong close", "[1}"),
        ("row88 double comma", "[1,,2]"),
        // rows 96-102: object rejections
        ("row97 open only", "{"),
        ("row97 open ws", "{ "),
        ("row98 nothing after comma", "{\"a\":1,"),
        ("row98 comma then ws", "{\"a\":1, "),
        ("row100 missing colon", "{\"a\" 1}"),
        ("row100 no value", "{\"a\"}"),
        ("row100 comma instead", "{\"a\",1}"),
        ("row101 empty value", "{\"a\":}"),
        ("row101 bad value", "{\"a\":x}"),
        ("row102 unterminated", "{\"a\":1"),
        ("row102 missing comma", "{\"a\":1 \"b\":2}"),
        ("row102 wrong close", "{\"a\":1]"),
        ("row99 trailing comma", "{\"a\":1,}"),
        // mixed / nested failures
        ("nested array fail", "[[1,]]"),
        ("nested object fail", "{\"a\":{\"b\":}}"),
        ("deep fail", "[1,[2,[3,[4,\"x]]]]"),
    ];

    unsafe {
        for (label, input) in cases {
            let buf = cbytes(input.as_bytes());
            let base = buf.as_ptr();
            let ci = (p.c.cJSON_Parse)(base);
            let ri = (p.r.cJSON_Parse)(base);
            assert_eq!(
                ci.is_null(),
                ri.is_null(),
                "{label}: Parse({input:?}) nullness (C null={}, R null={})",
                ci.is_null(),
                ri.is_null()
            );
            let ce = (p.c.cJSON_GetErrorPtr)();
            let re = (p.r.cJSON_GetErrorPtr)();
            assert_eq!(ce.is_null(), re.is_null(), "{label}: GetErrorPtr nullness");
            if !ce.is_null() {
                assert_eq!(
                    ce.offset_from(base),
                    re.offset_from(base),
                    "{label}: GetErrorPtr offset for {input:?}"
                );
            }
            assert!(snapshot(ci) == snapshot(ri), "{label}: tree for {input:?}");
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);

            // same input through every other parse entry point
            for &rnt in BOOLS {
                let mut ce2: *const c_char = null();
                let mut re2: *const c_char = null();
                let c2 = (p.c.cJSON_ParseWithOpts)(base, &mut ce2, rnt);
                let r2 = (p.r.cJSON_ParseWithOpts)(base, &mut re2, rnt);
                assert_eq!(
                    c2.is_null(),
                    r2.is_null(),
                    "{label}: ParseWithOpts(rnt={rnt}) nullness for {input:?}"
                );
                let co = if ce2.is_null() { None } else { Some(ce2.offset_from(base)) };
                let ro = if re2.is_null() { None } else { Some(re2.offset_from(base)) };
                assert_eq!(co, ro, "{label}: ParseWithOpts(rnt={rnt}) end ptr for {input:?}");
                (p.c.cJSON_Delete)(c2);
                (p.r.cJSON_Delete)(r2);
            }
            for len in 0..=(input.len() + 2) {
                let c3 = (p.c.cJSON_ParseWithLength)(base, len);
                let r3 = (p.r.cJSON_ParseWithLength)(base, len);
                assert_eq!(
                    c3.is_null(),
                    r3.is_null(),
                    "{label}: ParseWithLength(len={len}) nullness for {input:?}"
                );
                assert!(
                    snapshot(c3) == snapshot(r3),
                    "{label}: ParseWithLength(len={len}) tree for {input:?}"
                );
                let ce3 = (p.c.cJSON_GetErrorPtr)();
                let re3 = (p.r.cJSON_GetErrorPtr)();
                assert_eq!(ce3.is_null(), re3.is_null(), "{label}: err nullness len={len}");
                if !ce3.is_null() {
                    assert_eq!(
                        ce3.offset_from(base),
                        re3.offset_from(base),
                        "{label}: err offset len={len} for {input:?}"
                    );
                }
                (p.c.cJSON_Delete)(c3);
                (p.r.cJSON_Delete)(r3);
            }
        }
    }
}

#[test]
fn rows_61_64_65_66_parse_length_and_null_termination() {
    let _g = lock();
    let p = pair();
    unsafe {
        // row 61: buffer_length == 0
        for input in ["", "1", "[1,2]"] {
            let buf = cbytes(input.as_bytes());
            let base = buf.as_ptr();
            assert!((p.c.cJSON_ParseWithLength)(base, 0).is_null());
            assert!((p.r.cJSON_ParseWithLength)(base, 0).is_null());
            let ce = (p.c.cJSON_GetErrorPtr)();
            let re = (p.r.cJSON_GetErrorPtr)();
            assert_eq!(ce.is_null(), re.is_null(), "row 61: err nullness");
            if !ce.is_null() {
                assert_eq!(ce.offset_from(base), re.offset_from(base), "row 61: err offset");
            }
            for &rnt in BOOLS {
                let mut ce2: *const c_char = null();
                let mut re2: *const c_char = null();
                assert!((p.c.cJSON_ParseWithLengthOpts)(base, 0, &mut ce2, rnt).is_null());
                assert!((p.r.cJSON_ParseWithLengthOpts)(base, 0, &mut re2, rnt).is_null());
                let co = if ce2.is_null() { None } else { Some(ce2.offset_from(base)) };
                let ro = if re2.is_null() { None } else { Some(re2.offset_from(base)) };
                assert_eq!(co, ro, "row 61: end ptr (rnt={rnt})");
            }
        }

        // rows 64-66: require_null_terminated with trailing garbage, and
        // buffer_length shorter than the value.
        let cases = [
            "1 x", "1x", "[1,2] y", "{\"a\":1}z", "null null", "true,", "\"a\" \"b\"",
            "1\t\n ", "1 \u{1}",
        ];
        for input in cases {
            let buf = cbytes(input.as_bytes());
            let base = buf.as_ptr();
            for &rnt in BOOLS {
                for len in 0..=(input.len() + 2) {
                    for want_end in [false, true] {
                        let mut ce: *const c_char = null();
                        let mut re: *const c_char = null();
                        let cp = if want_end { &mut ce as *mut _ } else { null_mut() };
                        let rp = if want_end { &mut re as *mut _ } else { null_mut() };
                        let ci = (p.c.cJSON_ParseWithLengthOpts)(base, len, cp, rnt);
                        let ri = (p.r.cJSON_ParseWithLengthOpts)(base, len, rp, rnt);
                        assert_eq!(
                            ci.is_null(),
                            ri.is_null(),
                            "rows 64-66: {input:?} len={len} rnt={rnt} nullness"
                        );
                        assert!(
                            snapshot(ci) == snapshot(ri),
                            "rows 64-66: {input:?} len={len} rnt={rnt} tree"
                        );
                        if want_end {
                            let co = if ce.is_null() { None } else { Some(ce.offset_from(base)) };
                            let ro = if re.is_null() { None } else { Some(re.offset_from(base)) };
                            assert_eq!(
                                co, ro,
                                "rows 64-66: {input:?} len={len} rnt={rnt} end ptr"
                            );
                        }
                        let ce3 = (p.c.cJSON_GetErrorPtr)();
                        let re3 = (p.r.cJSON_GetErrorPtr)();
                        assert_eq!(ce3.is_null(), re3.is_null(), "err nullness");
                        if !ce3.is_null() {
                            assert_eq!(
                                ce3.offset_from(base),
                                re3.offset_from(base),
                                "rows 64-66: {input:?} len={len} rnt={rnt} err offset"
                            );
                        }
                        (p.c.cJSON_Delete)(ci);
                        (p.r.cJSON_Delete)(ri);
                    }
                }
            }
        }
    }
}

#[test]
fn rows_85_95_nesting_limit_exact_boundary() {
    let _g = lock();
    let p = pair();
    unsafe {
        for &(open, close) in &[("[", "]"), ("{\"a\":", "}")] {
            for depth in [998usize, 999, 1000, 1001] {
                let s = format!("{}1{}", open.repeat(depth), close.repeat(depth));
                let buf = cbytes(s.as_bytes());
                let base = buf.as_ptr();
                let ci = (p.c.cJSON_Parse)(base);
                let ri = (p.r.cJSON_Parse)(base);
                assert_eq!(
                    ci.is_null(),
                    ri.is_null(),
                    "rows 85/95: depth {depth} '{open}' nullness (C null={}, R null={})",
                    ci.is_null(),
                    ri.is_null()
                );
                let ce = (p.c.cJSON_GetErrorPtr)();
                let re = (p.r.cJSON_GetErrorPtr)();
                assert_eq!(ce.is_null(), re.is_null(), "depth {depth} err nullness");
                if !ce.is_null() {
                    assert_eq!(
                        ce.offset_from(base),
                        re.offset_from(base),
                        "rows 85/95: depth {depth} err offset"
                    );
                }
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
        // exactly at the limit must succeed, one past must fail
        let ok = cbytes(format!("{}1{}", "[".repeat(1000), "]".repeat(1000)).as_bytes());
        let bad = cbytes(format!("{}1{}", "[".repeat(1001), "]".repeat(1001)).as_bytes());
        let a = (p.c.cJSON_Parse)(ok.as_ptr());
        let b = (p.r.cJSON_Parse)(ok.as_ptr());
        assert!(!a.is_null() && !b.is_null(), "depth 1000 must parse");
        (p.c.cJSON_Delete)(a);
        (p.r.cJSON_Delete)(b);
        let a = (p.c.cJSON_Parse)(bad.as_ptr());
        let b = (p.r.cJSON_Parse)(bad.as_ptr());
        assert!(a.is_null() && b.is_null(), "depth 1001 must fail");
        (p.c.cJSON_Delete)(a);
        (p.r.cJSON_Delete)(b);
    }
}

/* ================================================================== */
/* rows 32, 36, 53, 71-77, 80-84, 90-94, 103-104: print rejections       */
/* ================================================================== */

#[test]
fn rows_71_77_print_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        // rows 68/73/77/80: item == NULL
        assert_eq!(
            take_printed(p.c, (p.c.cJSON_Print)(null())),
            take_printed(p.r, (p.r.cJSON_Print)(null())),
            "row 80: Print(NULL)"
        );
        assert!((p.c.cJSON_Print)(null()).is_null());
        assert!((p.r.cJSON_Print)(null()).is_null());
        assert!((p.c.cJSON_PrintUnformatted)(null()).is_null());
        assert!((p.r.cJSON_PrintUnformatted)(null()).is_null());

        for &pre in &[-1i32, 0, 1, 256, i32::MIN, i32::MAX] {
            for &fmt in BOOLS {
                let c = (p.c.cJSON_PrintBuffered)(null(), pre, fmt);
                let r = (p.r.cJSON_PrintBuffered)(null(), pre, fmt);
                assert_eq!(
                    c.is_null(),
                    r.is_null(),
                    "rows 71/73: PrintBuffered(NULL,{pre},{fmt}) nullness"
                );
                take_printed(p.c, c);
                take_printed(p.r, r);
            }
        }

        // row 71: prebuffer < 0 on a valid item
        let ci = (p.c.cJSON_CreateNumber)(1.0);
        let ri = (p.r.cJSON_CreateNumber)(1.0);
        for &pre in &[-1i32, -2, i32::MIN] {
            for &fmt in BOOLS {
                let c = (p.c.cJSON_PrintBuffered)(ci, pre, fmt);
                let r = (p.r.cJSON_PrintBuffered)(ri, pre, fmt);
                assert!(c.is_null() && r.is_null(), "row 71: prebuffer={pre}");
            }
        }

        // rows 74-77: PrintPreallocated
        let mut buf = vec![0u8; 64];
        for &len in &[-1i32, -2, i32::MIN] {
            for &fmt in BOOLS {
                assert_eq!(
                    (p.c.cJSON_PrintPreallocated)(ci, buf.as_mut_ptr() as *mut c_char, len, fmt),
                    (p.r.cJSON_PrintPreallocated)(ri, buf.as_mut_ptr() as *mut c_char, len, fmt),
                    "row 74: PrintPreallocated(len={len},fmt={fmt})"
                );
            }
        }
        for &len in &[0i32, 1, 2, 8, 64] {
            for &fmt in BOOLS {
                assert_eq!(
                    (p.c.cJSON_PrintPreallocated)(ci, null_mut(), len, fmt),
                    (p.r.cJSON_PrintPreallocated)(ri, null_mut(), len, fmt),
                    "row 75: PrintPreallocated(buffer=NULL,len={len})"
                );
                assert_eq!(
                    (p.c.cJSON_PrintPreallocated)(null_mut(), buf.as_mut_ptr() as *mut c_char, len, fmt),
                    (p.r.cJSON_PrintPreallocated)(null_mut(), buf.as_mut_ptr() as *mut c_char, len, fmt),
                    "row 77: PrintPreallocated(item=NULL,len={len})"
                );
            }
        }
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);

        // rows 32/76/84/91-94/104: buffer too small at every `ensure` call site.
        let docs = [
            "null", "true", "false", "0", "1234567890", "1.5", "\"\"", "\"abc\"",
            "\"a\\nb\"", "\"\\u0001\"", "[]", "[1]", "[1,2]", "[[1],[2]]", "{}",
            "{\"a\":1}", "{\"a\":1,\"b\":2}", "{\"a\":{\"b\":[1,2,3]}}",
            "[{\"k\":\"v\"},{\"k\":\"v\"}]",
        ];
        for doc in docs {
            let b = cbytes(doc.as_bytes());
            let c = (p.c.cJSON_Parse)(b.as_ptr());
            let r = (p.r.cJSON_Parse)(b.as_ptr());
            assert!(!c.is_null() && !r.is_null(), "{doc} should parse");
            for &fmt in &[0i32, 1] {
                for len in 0i32..80 {
                    let mut cb = vec![0xAAu8; 128];
                    let mut rb = vec![0xAAu8; 128];
                    let cr =
                        (p.c.cJSON_PrintPreallocated)(c, cb.as_mut_ptr() as *mut c_char, len, fmt);
                    let rr =
                        (p.r.cJSON_PrintPreallocated)(r, rb.as_mut_ptr() as *mut c_char, len, fmt);
                    assert_eq!(
                        cr, rr,
                        "row 76: PrintPreallocated({doc:?},len={len},fmt={fmt}) return"
                    );
                    // The bytes the two libraries wrote into the caller's buffer
                    // must match exactly, success or failure.
                    assert_eq!(
                        cb, rb,
                        "row 76: PrintPreallocated({doc:?},len={len},fmt={fmt}) buffer bytes\n C: {:?}\n R: {:?}",
                        &cb[..len.max(0).min(128) as usize],
                        &rb[..len.max(0).min(128) as usize]
                    );
                }
            }
            // row 72/tiny prebuffer growth
            for pre in 0i32..40 {
                for &fmt in &[0i32, 1] {
                    let co = take_printed(p.c, (p.c.cJSON_PrintBuffered)(c, pre, fmt));
                    let ro = take_printed(p.r, (p.r.cJSON_PrintBuffered)(r, pre, fmt));
                    assert!(
                        co == ro,
                        "PrintBuffered({doc:?},pre={pre},fmt={fmt}) differs\n C: {}\n R: {}",
                        show(&co),
                        show(&ro)
                    );
                }
            }
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);
        }
    }
}

#[test]
fn row_82_raw_with_null_valuestring() {
    let _g = lock();
    let p = pair();
    unsafe {
        let c = (p.c.cJSON_CreateRaw)(cs("x").as_ptr());
        let r = (p.r.cJSON_CreateRaw)(cs("x").as_ptr());
        // Free and null out valuestring through the library's own allocator.
        (p.c.cJSON_free)((*c).valuestring as *mut c_void);
        (p.r.cJSON_free)((*r).valuestring as *mut c_void);
        (*c).valuestring = null_mut();
        (*r).valuestring = null_mut();
        assert_eq!(
            take_printed(p.c, (p.c.cJSON_Print)(c)),
            take_printed(p.r, (p.r.cJSON_Print)(r)),
            "row 82: Print(Raw with NULL valuestring)"
        );
        assert!((p.c.cJSON_Print)(c).is_null());
        for &fmt in &[0i32, 1] {
            let mut cb = vec![0u8; 64];
            let mut rb = vec![0u8; 64];
            assert_eq!(
                (p.c.cJSON_PrintPreallocated)(c, cb.as_mut_ptr() as *mut c_char, 64, fmt),
                (p.r.cJSON_PrintPreallocated)(r, rb.as_mut_ptr() as *mut c_char, 64, fmt),
                "row 82: PrintPreallocated(fmt={fmt})"
            );
            assert_eq!(cb, rb, "row 82: buffer bytes (fmt={fmt})");
        }
        // A Raw inside an array must make the whole print fail
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        (p.c.cJSON_AddItemToArray)(ca, c);
        (p.r.cJSON_AddItemToArray)(ra, r);
        assert_eq!(
            take_printed(p.c, (p.c.cJSON_Print)(ca)),
            take_printed(p.r, (p.r.cJSON_Print)(ra)),
            "row 92: Print(array containing broken Raw)"
        );
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);
    }
}

/* ================================================================== */
/* rows 19-25: cJSON_SetValuestring                                    */
/* ================================================================== */

#[test]
fn rows_19_25_set_valuestring_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        let newv = cs("new value");
        // row 19: object == NULL
        assert_eq!(
            (p.c.cJSON_SetValuestring)(null_mut(), newv.as_ptr()).is_null(),
            (p.r.cJSON_SetValuestring)(null_mut(), newv.as_ptr()).is_null(),
            "row 19"
        );
        assert!((p.c.cJSON_SetValuestring)(null_mut(), newv.as_ptr()).is_null());

        // row 20: not a String item / row 21: reference item
        let makers: [(&str, fn(&Api) -> *mut CJson); 8] = [
            ("null", |a| unsafe { (a.cJSON_CreateNull)() }),
            ("true", |a| unsafe { (a.cJSON_CreateTrue)() }),
            ("false", |a| unsafe { (a.cJSON_CreateFalse)() }),
            ("number", |a| unsafe { (a.cJSON_CreateNumber)(1.0) }),
            ("array", |a| unsafe { (a.cJSON_CreateArray)() }),
            ("object", |a| unsafe { (a.cJSON_CreateObject)() }),
            ("raw", |a| unsafe { (a.cJSON_CreateRaw)(b"r\0".as_ptr() as *const c_char) }),
            ("strref", |a| unsafe {
                (a.cJSON_CreateStringReference)(b"shared\0".as_ptr() as *const c_char)
            }),
        ];
        for (name, mk) in makers {
            let c = mk(p.c);
            let r = mk(p.r);
            let cres = (p.c.cJSON_SetValuestring)(c, newv.as_ptr());
            let rres = (p.r.cJSON_SetValuestring)(r, newv.as_ptr());
            assert_eq!(cres.is_null(), rres.is_null(), "rows 20/21: {name}");
            assert_eq!(take_cstr(cres), take_cstr(rres), "rows 20/21: {name} text");
            assert!(snapshot(c) == snapshot(r), "rows 20/21: {name} state");
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);
        }

        // row 22: valuestring == NULL on a String item
        let c = (p.c.cJSON_CreateStringReference)(null());
        let r = (p.r.cJSON_CreateStringReference)(null());
        (*c).type_ = cJSON_String; // drop IsReference so row 21 doesn't short-circuit
        (*r).type_ = cJSON_String;
        assert_eq!(
            (p.c.cJSON_SetValuestring)(c, newv.as_ptr()).is_null(),
            (p.r.cJSON_SetValuestring)(r, newv.as_ptr()).is_null(),
            "row 22"
        );
        assert!((p.c.cJSON_SetValuestring)(c, newv.as_ptr()).is_null());
        (p.c.cJSON_Delete)(c);
        (p.r.cJSON_Delete)(r);

        // row 23: valuestring argument == NULL
        let c = (p.c.cJSON_CreateString)(cs("abc").as_ptr());
        let r = (p.r.cJSON_CreateString)(cs("abc").as_ptr());
        assert_eq!(
            (p.c.cJSON_SetValuestring)(c, null()).is_null(),
            (p.r.cJSON_SetValuestring)(r, null()).is_null(),
            "row 23"
        );
        assert!((p.c.cJSON_SetValuestring)(c, null()).is_null());
        assert!(snapshot(c) == snapshot(r), "row 23 state");

        // row 24: overlapping buffers with v1_len <= v2_len
        for off in 0..3usize {
            let cres = (p.c.cJSON_SetValuestring)(c, (*c).valuestring.add(off));
            let rres = (p.r.cJSON_SetValuestring)(r, (*r).valuestring.add(off));
            assert_eq!(cres.is_null(), rres.is_null(), "row 24: overlap off={off}");
            assert_eq!(take_cstr(cres), take_cstr(rres), "row 24: overlap off={off} text");
            assert!(snapshot(c) == snapshot(r), "row 24: overlap off={off} state");
        }
        (p.c.cJSON_Delete)(c);
        (p.r.cJSON_Delete)(r);
    }
}

/* ================================================================== */
/* rows 105-133: query / add rejections                                 */
/* ================================================================== */

#[test]
fn rows_105_114_query_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        // rows 105/109: NULL container
        assert_eq!(
            (p.c.cJSON_GetArraySize)(null()),
            (p.r.cJSON_GetArraySize)(null()),
            "row 105"
        );
        for &idx in &[-1i32, 0, 1, i32::MAX, i32::MIN] {
            assert_eq!(
                (p.c.cJSON_GetArrayItem)(null(), idx).is_null(),
                (p.r.cJSON_GetArrayItem)(null(), idx).is_null(),
                "rows 107/109: GetArrayItem(NULL,{idx})"
            );
        }
        // row 106: non-container items have no children
        let makers: [(&str, fn(&Api) -> *mut CJson); 6] = [
            ("null", |a| unsafe { (a.cJSON_CreateNull)() }),
            ("true", |a| unsafe { (a.cJSON_CreateTrue)() }),
            ("number", |a| unsafe { (a.cJSON_CreateNumber)(1.0) }),
            ("string", |a| unsafe { (a.cJSON_CreateString)(b"s\0".as_ptr() as *const c_char) }),
            ("array", |a| unsafe { (a.cJSON_CreateArray)() }),
            ("object", |a| unsafe { (a.cJSON_CreateObject)() }),
        ];
        let key = cs("k");
        for (name, mk) in makers {
            let c = mk(p.c);
            let r = mk(p.r);
            assert_eq!(
                (p.c.cJSON_GetArraySize)(c),
                (p.r.cJSON_GetArraySize)(r),
                "row 106: GetArraySize({name})"
            );
            for &idx in &[-1i32, 0, 1, i32::MAX] {
                assert_eq!(
                    (p.c.cJSON_GetArrayItem)(c, idx).is_null(),
                    (p.r.cJSON_GetArrayItem)(r, idx).is_null(),
                    "rows 107/108: GetArrayItem({name},{idx})"
                );
            }
            // rows 110-114
            assert_eq!(
                (p.c.cJSON_GetObjectItem)(c, key.as_ptr()).is_null(),
                (p.r.cJSON_GetObjectItem)(r, key.as_ptr()).is_null(),
                "row 112: GetObjectItem({name})"
            );
            assert_eq!(
                (p.c.cJSON_GetObjectItem)(c, null()).is_null(),
                (p.r.cJSON_GetObjectItem)(r, null()).is_null(),
                "row 111: GetObjectItem({name},NULL)"
            );
            assert_eq!(
                (p.c.cJSON_GetObjectItemCaseSensitive)(c, null()).is_null(),
                (p.r.cJSON_GetObjectItemCaseSensitive)(r, null()).is_null(),
                "row 111: GetObjectItemCS({name},NULL)"
            );
            assert_eq!(
                (p.c.cJSON_HasObjectItem)(c, key.as_ptr()),
                (p.r.cJSON_HasObjectItem)(r, key.as_ptr()),
                "row 114: HasObjectItem({name})"
            );
            assert_eq!(
                (p.c.cJSON_HasObjectItem)(c, null()),
                (p.r.cJSON_HasObjectItem)(r, null()),
                "row 114: HasObjectItem({name},NULL)"
            );
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);
        }
        assert_eq!(
            (p.c.cJSON_GetObjectItem)(null(), key.as_ptr()).is_null(),
            (p.r.cJSON_GetObjectItem)(null(), key.as_ptr()).is_null(),
            "row 110"
        );
        assert_eq!(
            (p.c.cJSON_HasObjectItem)(null(), key.as_ptr()),
            (p.r.cJSON_HasObjectItem)(null(), key.as_ptr()),
            "row 114: HasObjectItem(NULL)"
        );

        // row 113: matched element whose `string` is NULL
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        (p.c.cJSON_AddNumberToObject)(co, key.as_ptr(), 1.0);
        (p.r.cJSON_AddNumberToObject)(ro, key.as_ptr(), 1.0);
        let ck = (*(*co).child).string;
        let rk = (*(*ro).child).string;
        (*(*co).child).string = null_mut();
        (*(*ro).child).string = null_mut();
        assert_eq!(
            (p.c.cJSON_GetObjectItem)(co, key.as_ptr()).is_null(),
            (p.r.cJSON_GetObjectItem)(ro, key.as_ptr()).is_null(),
            "row 113: GetObjectItem with NULL child key"
        );
        assert_eq!(
            (p.c.cJSON_GetObjectItemCaseSensitive)(co, key.as_ptr()).is_null(),
            (p.r.cJSON_GetObjectItemCaseSensitive)(ro, key.as_ptr()).is_null(),
            "row 113: GetObjectItemCS with NULL child key"
        );
        (*(*co).child).string = ck;
        (*(*ro).child).string = rk;
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);
    }
}

#[test]
fn rows_115_133_add_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        let key = cs("k");

        // rows 115-117: add_item_to_array
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        assert_eq!(
            (p.c.cJSON_AddItemToArray)(ca, null_mut()),
            (p.r.cJSON_AddItemToArray)(ra, null_mut()),
            "row 115"
        );
        let cn = (p.c.cJSON_CreateNumber)(1.0);
        let rn = (p.r.cJSON_CreateNumber)(1.0);
        assert_eq!(
            (p.c.cJSON_AddItemToArray)(null_mut(), cn),
            (p.r.cJSON_AddItemToArray)(null_mut(), rn),
            "row 116"
        );
        assert_eq!(
            (p.c.cJSON_AddItemToArray)(ca, ca),
            (p.r.cJSON_AddItemToArray)(ra, ra),
            "row 117"
        );
        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);

        // row 118: corrupted list (child->prev == NULL) returns true but appends nothing
        (p.c.cJSON_AddItemToArray)(ca, (p.c.cJSON_CreateNumber)(1.0));
        (p.r.cJSON_AddItemToArray)(ra, (p.r.cJSON_CreateNumber)(1.0));
        (*(*ca).child).prev = null_mut();
        (*(*ra).child).prev = null_mut();
        let cn = (p.c.cJSON_CreateNumber)(2.0);
        let rn = (p.r.cJSON_CreateNumber)(2.0);
        assert_eq!(
            (p.c.cJSON_AddItemToArray)(ca, cn),
            (p.r.cJSON_AddItemToArray)(ra, rn),
            "row 118: return value"
        );
        assert_eq!(
            (p.c.cJSON_GetArraySize)(ca),
            (p.r.cJSON_GetArraySize)(ra),
            "row 118: size after corrupted append"
        );
        assert!(
            take_printed(p.c, (p.c.cJSON_PrintUnformatted)(ca))
                == take_printed(p.r, (p.r.cJSON_PrintUnformatted)(ra)),
            "row 118: print after corrupted append"
        );
        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);

        // rows 119-122: add_item_to_object
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        for cs_flag in [false, true] {
            let add_c = if cs_flag { p.c.cJSON_AddItemToObjectCS } else { p.c.cJSON_AddItemToObject };
            let add_r = if cs_flag { p.r.cJSON_AddItemToObjectCS } else { p.r.cJSON_AddItemToObject };
            let cn = (p.c.cJSON_CreateNumber)(1.0);
            let rn = (p.r.cJSON_CreateNumber)(1.0);
            assert_eq!(
                add_c(null_mut(), key.as_ptr(), cn),
                add_r(null_mut(), key.as_ptr(), rn),
                "row 119 (cs={cs_flag})"
            );
            assert_eq!(
                add_c(co, null(), cn),
                add_r(ro, null(), rn),
                "row 120 (cs={cs_flag})"
            );
            assert_eq!(
                add_c(co, key.as_ptr(), null_mut()),
                add_r(ro, key.as_ptr(), null_mut()),
                "row 121 (cs={cs_flag})"
            );
            assert_eq!(
                add_c(co, key.as_ptr(), co),
                add_r(ro, key.as_ptr(), ro),
                "row 122 (cs={cs_flag})"
            );
            (p.c.cJSON_Delete)(cn);
            (p.r.cJSON_Delete)(rn);
        }

        // rows 124-129: reference adders
        let cn = (p.c.cJSON_CreateNumber)(1.0);
        let rn = (p.r.cJSON_CreateNumber)(1.0);
        assert_eq!(
            (p.c.cJSON_AddItemReferenceToArray)(null_mut(), cn),
            (p.r.cJSON_AddItemReferenceToArray)(null_mut(), rn),
            "row 124"
        );
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        assert_eq!(
            (p.c.cJSON_AddItemReferenceToArray)(ca, null_mut()),
            (p.r.cJSON_AddItemReferenceToArray)(ra, null_mut()),
            "row 125"
        );
        assert_eq!(
            (p.c.cJSON_AddItemReferenceToObject)(null_mut(), key.as_ptr(), cn),
            (p.r.cJSON_AddItemReferenceToObject)(null_mut(), key.as_ptr(), rn),
            "row 127"
        );
        assert_eq!(
            (p.c.cJSON_AddItemReferenceToObject)(co, null(), cn),
            (p.r.cJSON_AddItemReferenceToObject)(ro, null(), rn),
            "row 128"
        );
        assert_eq!(
            (p.c.cJSON_AddItemReferenceToObject)(co, key.as_ptr(), null_mut()),
            (p.r.cJSON_AddItemReferenceToObject)(ro, key.as_ptr(), null_mut()),
            "row 129"
        );
        assert!(
            take_printed(p.c, (p.c.cJSON_PrintUnformatted)(co))
                == take_printed(p.r, (p.r.cJSON_PrintUnformatted)(ro)),
            "rows 119-129: object unchanged"
        );
        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);

        // rows 130-133: the nine Add*ToObject helpers
        let sv = cs("v");
        for (name, cf, rf) in [
            ("AddNull", p.c.cJSON_AddNullToObject, p.r.cJSON_AddNullToObject),
            ("AddTrue", p.c.cJSON_AddTrueToObject, p.r.cJSON_AddTrueToObject),
            ("AddFalse", p.c.cJSON_AddFalseToObject, p.r.cJSON_AddFalseToObject),
            ("AddObject", p.c.cJSON_AddObjectToObject, p.r.cJSON_AddObjectToObject),
            ("AddArray", p.c.cJSON_AddArrayToObject, p.r.cJSON_AddArrayToObject),
        ] {
            assert_eq!(
                cf(null_mut(), key.as_ptr()).is_null(),
                rf(null_mut(), key.as_ptr()).is_null(),
                "row 130: {name}(NULL object)"
            );
            let co = (p.c.cJSON_CreateObject)();
            let ro = (p.r.cJSON_CreateObject)();
            assert_eq!(
                cf(co, null()).is_null(),
                rf(ro, null()).is_null(),
                "row 131: {name}(NULL name)"
            );
            (p.c.cJSON_Delete)(co);
            (p.r.cJSON_Delete)(ro);
        }
        for &b in BOOLS {
            assert_eq!(
                (p.c.cJSON_AddBoolToObject)(null_mut(), key.as_ptr(), b).is_null(),
                (p.r.cJSON_AddBoolToObject)(null_mut(), key.as_ptr(), b).is_null(),
                "row 130: AddBool(NULL,{b})"
            );
        }
        assert_eq!(
            (p.c.cJSON_AddNumberToObject)(null_mut(), key.as_ptr(), 1.0).is_null(),
            (p.r.cJSON_AddNumberToObject)(null_mut(), key.as_ptr(), 1.0).is_null(),
            "row 130: AddNumber(NULL)"
        );
        assert_eq!(
            (p.c.cJSON_AddStringToObject)(null_mut(), key.as_ptr(), sv.as_ptr()).is_null(),
            (p.r.cJSON_AddStringToObject)(null_mut(), key.as_ptr(), sv.as_ptr()).is_null(),
            "row 130: AddString(NULL)"
        );
        assert_eq!(
            (p.c.cJSON_AddRawToObject)(null_mut(), key.as_ptr(), sv.as_ptr()).is_null(),
            (p.r.cJSON_AddRawToObject)(null_mut(), key.as_ptr(), sv.as_ptr()).is_null(),
            "row 130: AddRaw(NULL)"
        );
        // rows 132-133: NULL payload string
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        assert_eq!(
            (p.c.cJSON_AddStringToObject)(co, key.as_ptr(), null()).is_null(),
            (p.r.cJSON_AddStringToObject)(ro, key.as_ptr(), null()).is_null(),
            "row 132: AddStringToObject(NULL string)"
        );
        assert_eq!(
            (p.c.cJSON_AddRawToObject)(co, key.as_ptr(), null()).is_null(),
            (p.r.cJSON_AddRawToObject)(ro, key.as_ptr(), null()).is_null(),
            "row 133: AddRawToObject(NULL raw)"
        );
        assert!(
            take_printed(p.c, (p.c.cJSON_PrintUnformatted)(co))
                == take_printed(p.r, (p.r.cJSON_PrintUnformatted)(ro)),
            "rows 132-133: object unchanged"
        );
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);
    }
}

/* ================================================================== */
/* rows 134-157: detach / delete / insert / replace rejections            */
/* ================================================================== */

#[test]
fn rows_134_157_mutation_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        let key = cs("k");

        // rows 134-136
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        (p.c.cJSON_AddItemToArray)(ca, (p.c.cJSON_CreateNumber)(1.0));
        (p.r.cJSON_AddItemToArray)(ra, (p.r.cJSON_CreateNumber)(1.0));
        let cchild = (*ca).child;
        let rchild = (*ra).child;
        assert_eq!(
            (p.c.cJSON_DetachItemViaPointer)(null_mut(), cchild).is_null(),
            (p.r.cJSON_DetachItemViaPointer)(null_mut(), rchild).is_null(),
            "row 134"
        );
        assert_eq!(
            (p.c.cJSON_DetachItemViaPointer)(ca, null_mut()).is_null(),
            (p.r.cJSON_DetachItemViaPointer)(ra, null_mut()).is_null(),
            "row 135"
        );
        // row 136: item not the child and prev == NULL
        let cfor = (p.c.cJSON_CreateNumber)(9.0);
        let rfor = (p.r.cJSON_CreateNumber)(9.0);
        assert_eq!(
            (p.c.cJSON_DetachItemViaPointer)(ca, cfor).is_null(),
            (p.r.cJSON_DetachItemViaPointer)(ra, rfor).is_null(),
            "row 136"
        );
        (p.c.cJSON_Delete)(cfor);
        (p.r.cJSON_Delete)(rfor);

        // rows 137-138, 141
        for &which in &[-1i32, -2, 1, 2, i32::MAX, i32::MIN] {
            assert_eq!(
                (p.c.cJSON_DetachItemFromArray)(ca, which).is_null(),
                (p.r.cJSON_DetachItemFromArray)(ra, which).is_null(),
                "rows 137/138: which={which}"
            );
            (p.c.cJSON_DeleteItemFromArray)(ca, which);
            (p.r.cJSON_DeleteItemFromArray)(ra, which);
            assert_eq!(
                (p.c.cJSON_GetArraySize)(ca),
                (p.r.cJSON_GetArraySize)(ra),
                "row 141: size after out-of-range delete which={which}"
            );
        }
        assert_eq!(
            (p.c.cJSON_DetachItemFromArray)(null_mut(), 0).is_null(),
            (p.r.cJSON_DetachItemFromArray)(null_mut(), 0).is_null(),
            "row 137: NULL array"
        );
        (p.c.cJSON_DeleteItemFromArray)(null_mut(), 0);
        (p.r.cJSON_DeleteItemFromArray)(null_mut(), 0);
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);

        // rows 139-140, 142
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        (p.c.cJSON_AddNumberToObject)(co, cs("Key").as_ptr(), 1.0);
        (p.r.cJSON_AddNumberToObject)(ro, cs("Key").as_ptr(), 1.0);
        for probe in ["key", "KEY", "missing", ""] {
            let k = cs(probe);
            assert_eq!(
                (p.c.cJSON_DetachItemFromObjectCaseSensitive)(co, k.as_ptr()).is_null(),
                (p.r.cJSON_DetachItemFromObjectCaseSensitive)(ro, k.as_ptr()).is_null(),
                "row 140: DetachCS({probe:?})"
            );
        }
        assert_eq!(
            (p.c.cJSON_DetachItemFromObject)(co, null()).is_null(),
            (p.r.cJSON_DetachItemFromObject)(ro, null()).is_null(),
            "row 139: Detach(NULL key)"
        );
        assert_eq!(
            (p.c.cJSON_DetachItemFromObject)(null_mut(), key.as_ptr()).is_null(),
            (p.r.cJSON_DetachItemFromObject)(null_mut(), key.as_ptr()).is_null(),
            "row 139: Detach(NULL object)"
        );
        (p.c.cJSON_DeleteItemFromObject)(co, cs("missing").as_ptr());
        (p.r.cJSON_DeleteItemFromObject)(ro, cs("missing").as_ptr());
        (p.c.cJSON_DeleteItemFromObjectCaseSensitive)(co, cs("key").as_ptr());
        (p.r.cJSON_DeleteItemFromObjectCaseSensitive)(ro, cs("key").as_ptr());
        (p.c.cJSON_DeleteItemFromObject)(null_mut(), key.as_ptr());
        (p.r.cJSON_DeleteItemFromObject)(null_mut(), key.as_ptr());
        (p.c.cJSON_DeleteItemFromObjectCaseSensitive)(null_mut(), key.as_ptr());
        (p.r.cJSON_DeleteItemFromObjectCaseSensitive)(null_mut(), key.as_ptr());
        assert!(
            take_printed(p.c, (p.c.cJSON_PrintUnformatted)(co))
                == take_printed(p.r, (p.r.cJSON_PrintUnformatted)(ro)),
            "row 142: object unchanged"
        );

        // rows 143-146
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        for &which in &[-1i32, -2, i32::MIN] {
            let cn = (p.c.cJSON_CreateNumber)(1.0);
            let rn = (p.r.cJSON_CreateNumber)(1.0);
            assert_eq!(
                (p.c.cJSON_InsertItemInArray)(ca, which, cn),
                (p.r.cJSON_InsertItemInArray)(ra, which, rn),
                "row 143: which={which}"
            );
            (p.c.cJSON_Delete)(cn);
            (p.r.cJSON_Delete)(rn);
        }
        for &which in &[0i32, 1, i32::MAX] {
            assert_eq!(
                (p.c.cJSON_InsertItemInArray)(ca, which, null_mut()),
                (p.r.cJSON_InsertItemInArray)(ra, which, null_mut()),
                "row 144: which={which}"
            );
        }
        // row 145: which >= size falls back to add_item_to_array; NULL array
        let cn = (p.c.cJSON_CreateNumber)(1.0);
        let rn = (p.r.cJSON_CreateNumber)(1.0);
        assert_eq!(
            (p.c.cJSON_InsertItemInArray)(null_mut(), 0, cn),
            (p.r.cJSON_InsertItemInArray)(null_mut(), 0, rn),
            "row 145: NULL array"
        );
        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);
        // row 146: after_inserted corrupted
        (p.c.cJSON_AddItemToArray)(ca, (p.c.cJSON_CreateNumber)(1.0));
        (p.r.cJSON_AddItemToArray)(ra, (p.r.cJSON_CreateNumber)(1.0));
        (p.c.cJSON_AddItemToArray)(ca, (p.c.cJSON_CreateNumber)(2.0));
        (p.r.cJSON_AddItemToArray)(ra, (p.r.cJSON_CreateNumber)(2.0));
        let csecond = (p.c.cJSON_GetArrayItem)(ca, 1);
        let rsecond = (p.r.cJSON_GetArrayItem)(ra, 1);
        (*csecond).prev = null_mut();
        (*rsecond).prev = null_mut();
        let cn = (p.c.cJSON_CreateNumber)(3.0);
        let rn = (p.r.cJSON_CreateNumber)(3.0);
        assert_eq!(
            (p.c.cJSON_InsertItemInArray)(ca, 1, cn),
            (p.r.cJSON_InsertItemInArray)(ra, 1, rn),
            "row 146: corrupted after_inserted"
        );
        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);
        // restore links before deleting
        (*csecond).prev = (*ca).child;
        (*rsecond).prev = (*ra).child;
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);

        // rows 147-153
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        let cn = (p.c.cJSON_CreateNumber)(1.0);
        let rn = (p.r.cJSON_CreateNumber)(1.0);
        assert_eq!(
            (p.c.cJSON_ReplaceItemViaPointer)(null_mut(), cn, cn),
            (p.r.cJSON_ReplaceItemViaPointer)(null_mut(), rn, rn),
            "row 147"
        );
        assert_eq!(
            (p.c.cJSON_ReplaceItemViaPointer)(ca, cn, cn),
            (p.r.cJSON_ReplaceItemViaPointer)(ra, rn, rn),
            "row 148: parent->child == NULL"
        );
        (p.c.cJSON_AddItemToArray)(ca, cn);
        (p.r.cJSON_AddItemToArray)(ra, rn);
        assert_eq!(
            (p.c.cJSON_ReplaceItemViaPointer)(ca, cn, null_mut()),
            (p.r.cJSON_ReplaceItemViaPointer)(ra, rn, null_mut()),
            "row 149"
        );
        let cr = (p.c.cJSON_CreateNumber)(2.0);
        let rr = (p.r.cJSON_CreateNumber)(2.0);
        assert_eq!(
            (p.c.cJSON_ReplaceItemViaPointer)(ca, null_mut(), cr),
            (p.r.cJSON_ReplaceItemViaPointer)(ra, null_mut(), rr),
            "row 150"
        );
        assert_eq!(
            (p.c.cJSON_ReplaceItemViaPointer)(ca, cn, cn),
            (p.r.cJSON_ReplaceItemViaPointer)(ra, rn, rn),
            "row 151: replacement == item"
        );
        assert!(
            take_printed(p.c, (p.c.cJSON_PrintUnformatted)(ca))
                == take_printed(p.r, (p.r.cJSON_PrintUnformatted)(ra)),
            "row 151: array unchanged"
        );
        for &which in &[-1i32, i32::MIN, 1, 2, i32::MAX] {
            assert_eq!(
                (p.c.cJSON_ReplaceItemInArray)(ca, which, cr),
                (p.r.cJSON_ReplaceItemInArray)(ra, which, rr),
                "rows 152/153: which={which}"
            );
        }
        (p.c.cJSON_Delete)(cr);
        (p.r.cJSON_Delete)(rr);
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);

        // rows 154-157
        for cs_flag in [false, true] {
            let rep_c = if cs_flag {
                p.c.cJSON_ReplaceItemInObjectCaseSensitive
            } else {
                p.c.cJSON_ReplaceItemInObject
            };
            let rep_r = if cs_flag {
                p.r.cJSON_ReplaceItemInObjectCaseSensitive
            } else {
                p.r.cJSON_ReplaceItemInObject
            };
            assert_eq!(
                rep_c(co, key.as_ptr(), null_mut()),
                rep_r(ro, key.as_ptr(), null_mut()),
                "row 154 (cs={cs_flag})"
            );
            let cn = (p.c.cJSON_CreateNumber)(5.0);
            let rn = (p.r.cJSON_CreateNumber)(5.0);
            assert_eq!(
                rep_c(co, null(), cn),
                rep_r(ro, null(), rn),
                "row 155 (cs={cs_flag})"
            );
            // row 157: key not found -> false, but replacement->string was rewritten
            let missing = cs("definitely-missing");
            assert_eq!(
                rep_c(co, missing.as_ptr(), cn),
                rep_r(ro, missing.as_ptr(), rn),
                "row 157 (cs={cs_flag})"
            );
            assert!(
                snapshot(cn) == snapshot(rn),
                "row 157: replacement state (cs={cs_flag})\n C: {:?}\n R: {:?}",
                snapshot(cn),
                snapshot(rn)
            );
            (p.c.cJSON_Delete)(cn);
            (p.r.cJSON_Delete)(rn);
        }
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);
    }
}

/* ================================================================== */
/* rows 159-172: constructor rejections                                 */
/* ================================================================== */

#[test]
fn rows_159_172_constructor_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        // rows 159/161: NULL string
        assert_eq!(
            (p.c.cJSON_CreateString)(null()).is_null(),
            (p.r.cJSON_CreateString)(null()).is_null(),
            "row 159"
        );
        assert!((p.c.cJSON_CreateString)(null()).is_null());
        assert_eq!(
            (p.c.cJSON_CreateRaw)(null()).is_null(),
            (p.r.cJSON_CreateRaw)(null()).is_null(),
            "row 161"
        );
        assert!((p.c.cJSON_CreateRaw)(null()).is_null());

        // rows 162-164: reference constructors accept NULL
        for kind in 0..3 {
            let (c, r) = match kind {
                0 => (
                    (p.c.cJSON_CreateStringReference)(null()),
                    (p.r.cJSON_CreateStringReference)(null()),
                ),
                1 => (
                    (p.c.cJSON_CreateObjectReference)(null()),
                    (p.r.cJSON_CreateObjectReference)(null()),
                ),
                _ => (
                    (p.c.cJSON_CreateArrayReference)(null()),
                    (p.r.cJSON_CreateArrayReference)(null()),
                ),
            };
            assert_eq!(c.is_null(), r.is_null(), "rows 162-164: kind={kind}");
            assert!(!c.is_null(), "rows 162-164: kind={kind} must succeed");
            assert_eq!((*c).type_, (*r).type_, "rows 162-164: kind={kind} type");
            assert!(snapshot(c) == snapshot(r), "rows 162-164: kind={kind} state");
            assert!(
                take_printed(p.c, (p.c.cJSON_Print)(c)) == take_printed(p.r, (p.r.cJSON_Print)(r)),
                "rows 162-164: kind={kind} print"
            );
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);
        }

        // rows 168-170: typed array constructors
        let ints = [1i32, 2, 3];
        let floats = [1.0f32, 2.0];
        let doubles = [1.0f64, 2.0];
        let owned: Vec<Vec<c_char>> = ["a", "b"].iter().map(|s| cbytes(s.as_bytes())).collect();
        let ptrs: Vec<*const c_char> = owned.iter().map(|v| v.as_ptr()).collect();
        for &count in &[-1i32, -2, i32::MIN] {
            assert_eq!(
                (p.c.cJSON_CreateIntArray)(ints.as_ptr(), count).is_null(),
                (p.r.cJSON_CreateIntArray)(ints.as_ptr(), count).is_null(),
                "row 168: Int count={count}"
            );
            assert_eq!(
                (p.c.cJSON_CreateFloatArray)(floats.as_ptr(), count).is_null(),
                (p.r.cJSON_CreateFloatArray)(floats.as_ptr(), count).is_null(),
                "row 168: Float count={count}"
            );
            assert_eq!(
                (p.c.cJSON_CreateDoubleArray)(doubles.as_ptr(), count).is_null(),
                (p.r.cJSON_CreateDoubleArray)(doubles.as_ptr(), count).is_null(),
                "row 168: Double count={count}"
            );
            assert_eq!(
                (p.c.cJSON_CreateStringArray)(ptrs.as_ptr(), count).is_null(),
                (p.r.cJSON_CreateStringArray)(ptrs.as_ptr(), count).is_null(),
                "row 168: String count={count}"
            );
        }
        for &count in &[0i32, 1, 2] {
            assert_eq!(
                (p.c.cJSON_CreateIntArray)(null(), count).is_null(),
                (p.r.cJSON_CreateIntArray)(null(), count).is_null(),
                "row 169: Int NULL count={count}"
            );
            assert_eq!(
                (p.c.cJSON_CreateFloatArray)(null(), count).is_null(),
                (p.r.cJSON_CreateFloatArray)(null(), count).is_null(),
                "row 169: Float NULL count={count}"
            );
            assert_eq!(
                (p.c.cJSON_CreateDoubleArray)(null(), count).is_null(),
                (p.r.cJSON_CreateDoubleArray)(null(), count).is_null(),
                "row 169: Double NULL count={count}"
            );
            assert_eq!(
                (p.c.cJSON_CreateStringArray)(null(), count).is_null(),
                (p.r.cJSON_CreateStringArray)(null(), count).is_null(),
                "row 169: String NULL count={count}"
            );
        }
        // row 170: a NULL element inside the string array
        for bad in 0..3usize {
            let mut ptrs2: Vec<*const c_char> = (0..3).map(|i| owned[i % 2].as_ptr()).collect();
            ptrs2[bad] = null();
            let c = (p.c.cJSON_CreateStringArray)(ptrs2.as_ptr(), 3);
            let r = (p.r.cJSON_CreateStringArray)(ptrs2.as_ptr(), 3);
            assert_eq!(c.is_null(), r.is_null(), "row 170: NULL element at {bad}");
            assert!(c.is_null(), "row 170: must fail");
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);
        }
    }
}

/* ================================================================== */
/* rows 173, 177: Duplicate rejections                                  */
/* ================================================================== */

#[test]
fn row_173_duplicate_null() {
    let _g = lock();
    let p = pair();
    unsafe {
        for &rec in BOOLS {
            assert_eq!(
                (p.c.cJSON_Duplicate)(null(), rec).is_null(),
                (p.r.cJSON_Duplicate)(null(), rec).is_null(),
                "row 173: Duplicate(NULL,{rec})"
            );
            assert!((p.c.cJSON_Duplicate)(null(), rec).is_null());
        }
    }
}

/* ================================================================== */
/* rows 178-180: Minify                                                */
/* ================================================================== */

#[test]
fn rows_178_180_minify_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        // row 178: NULL input must not crash and must not write
        (p.c.cJSON_Minify)(null_mut());
        (p.r.cJSON_Minify)(null_mut());

        // rows 179-180: unterminated comment / string
        for input in [
            "/* unterminated", "/*", "/", "//", "\"unterminated", "\"", "\"\\",
            "1 /* x", "[1,2 // x", "{\"a\": \"b",
        ] {
            let mut cb = cbytes(input.as_bytes());
            let mut rb = cbytes(input.as_bytes());
            (p.c.cJSON_Minify)(cb.as_mut_ptr());
            (p.r.cJSON_Minify)(rb.as_mut_ptr());
            assert_eq!(cb, rb, "rows 179/180: Minify({input:?}) differs");
        }
    }
}

/* ================================================================== */
/* rows 183-195: Compare rejections and cJSON_free(NULL)                */
/* ================================================================== */

#[test]
fn rows_183_195_compare_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        let cn = (p.c.cJSON_CreateNumber)(1.0);
        let rn = (p.r.cJSON_CreateNumber)(1.0);
        for &cse in BOOLS {
            assert_eq!(
                (p.c.cJSON_Compare)(null(), cn, cse),
                (p.r.cJSON_Compare)(null(), rn, cse),
                "row 183: Compare(NULL,x,{cse})"
            );
            assert_eq!(
                (p.c.cJSON_Compare)(cn, null(), cse),
                (p.r.cJSON_Compare)(rn, null(), cse),
                "row 184: Compare(x,NULL,{cse})"
            );
            assert_eq!(
                (p.c.cJSON_Compare)(null(), null(), cse),
                (p.r.cJSON_Compare)(null(), null(), cse),
                "rows 183/184: Compare(NULL,NULL,{cse})"
            );
        }

        // rows 185-186: mismatched and invalid types
        for ta in [0i32, 1, 2, 3, 4, 8, 9, 16, 32, 64, 127, 128, 255, 0x1FF, -1] {
            for tb in [0i32, 1, 2, 3, 4, 8, 9, 16, 32, 64, 127, 128, 255, 0x1FF, -1] {
                let ca = (p.c.cJSON_CreateNumber)(1.0);
                let cb = (p.c.cJSON_CreateNumber)(1.0);
                let ra = (p.r.cJSON_CreateNumber)(1.0);
                let rb = (p.r.cJSON_CreateNumber)(1.0);
                (*ca).type_ = ta;
                (*cb).type_ = tb;
                (*ra).type_ = ta;
                (*rb).type_ = tb;
                for &cse in &[0i32, 1] {
                    assert_eq!(
                        (p.c.cJSON_Compare)(ca, cb, cse),
                        (p.r.cJSON_Compare)(ra, rb, cse),
                        "rows 185/186: Compare(type {ta} vs {tb}, cs={cse})"
                    );
                    // row 187: aliasing with a possibly-invalid type
                    assert_eq!(
                        (p.c.cJSON_Compare)(ca, ca, cse),
                        (p.r.cJSON_Compare)(ra, ra, cse),
                        "row 187: Compare(self,self) type {ta} cs={cse}"
                    );
                }
                (*ca).type_ = cJSON_Number;
                (*cb).type_ = cJSON_Number;
                (*ra).type_ = cJSON_Number;
                (*rb).type_ = cJSON_Number;
                (p.c.cJSON_Delete)(ca);
                (p.c.cJSON_Delete)(cb);
                (p.r.cJSON_Delete)(ra);
                (p.r.cJSON_Delete)(rb);
            }
        }

        // row 189: String/Raw with NULL valuestring
        for &ty in &[cJSON_String, cJSON_Raw] {
            let ca = (p.c.cJSON_CreateStringReference)(null());
            let ra = (p.r.cJSON_CreateStringReference)(null());
            let cb = (p.c.cJSON_CreateStringReference)(null());
            let rb = (p.r.cJSON_CreateStringReference)(null());
            (*ca).type_ = ty;
            (*cb).type_ = ty;
            (*ra).type_ = ty;
            (*rb).type_ = ty;
            for &cse in &[0i32, 1] {
                assert_eq!(
                    (p.c.cJSON_Compare)(ca, cb, cse),
                    (p.r.cJSON_Compare)(ra, rb, cse),
                    "row 189: Compare(ty={ty}, both NULL valuestring, cs={cse})"
                );
            }
            let cgood = (p.c.cJSON_CreateString)(cs("x").as_ptr());
            let rgood = (p.r.cJSON_CreateString)(cs("x").as_ptr());
            (*cgood).type_ = ty;
            (*rgood).type_ = ty;
            for &cse in &[0i32, 1] {
                assert_eq!(
                    (p.c.cJSON_Compare)(ca, cgood, cse),
                    (p.r.cJSON_Compare)(ra, rgood, cse),
                    "row 189: Compare(NULL vs good, ty={ty}, cs={cse})"
                );
                assert_eq!(
                    (p.c.cJSON_Compare)(cgood, ca, cse),
                    (p.r.cJSON_Compare)(rgood, ra, cse),
                    "row 189: Compare(good vs NULL, ty={ty}, cs={cse})"
                );
            }
            (*cgood).type_ = cJSON_String;
            (*rgood).type_ = cJSON_String;
            (p.c.cJSON_Delete)(cgood);
            (p.r.cJSON_Delete)(rgood);
            for x in [ca, cb] {
                (*x).type_ = cJSON_String | cJSON_IsReference;
                (p.c.cJSON_Delete)(x);
            }
            for x in [ra, rb] {
                (*x).type_ = cJSON_String | cJSON_IsReference;
                (p.r.cJSON_Delete)(x);
            }
        }

        // rows 191-194: containers
        let pairs: &[(&str, &str)] = &[
            ("[1]", "[1,2]"),
            ("[1,2]", "[1]"),
            ("[]", "[1]"),
            ("{\"a\":1}", "{\"a\":1,\"b\":2}"),
            ("{\"a\":1,\"b\":2}", "{\"a\":1}"),
            ("{\"a\":1}", "{\"A\":1}"),
            ("{\"a\":1}", "{\"b\":1}"),
            ("{\"a\":[1,2]}", "{\"a\":[1,3]}"),
        ];
        for (a, b) in pairs {
            let ba = cbytes(a.as_bytes());
            let bb = cbytes(b.as_bytes());
            let cx = (p.c.cJSON_Parse)(ba.as_ptr());
            let cy = (p.c.cJSON_Parse)(bb.as_ptr());
            let rx = (p.r.cJSON_Parse)(ba.as_ptr());
            let ry = (p.r.cJSON_Parse)(bb.as_ptr());
            for &cse in BOOLS {
                assert_eq!(
                    (p.c.cJSON_Compare)(cx, cy, cse),
                    (p.r.cJSON_Compare)(rx, ry, cse),
                    "rows 191-194: Compare({a:?},{b:?},{cse})"
                );
            }
            (p.c.cJSON_Delete)(cx);
            (p.c.cJSON_Delete)(cy);
            (p.r.cJSON_Delete)(rx);
            (p.r.cJSON_Delete)(ry);
        }

        // Object child with NULL key (get_object_item -> NULL -> unequal)
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        let k = cs("k");
        (p.c.cJSON_AddNumberToObject)(co, k.as_ptr(), 1.0);
        (p.r.cJSON_AddNumberToObject)(ro, k.as_ptr(), 1.0);
        let co2 = (p.c.cJSON_Duplicate)(co, 1);
        let ro2 = (p.r.cJSON_Duplicate)(ro, 1);
        let ck = (*(*co).child).string;
        let rk = (*(*ro).child).string;
        (*(*co).child).string = null_mut();
        (*(*ro).child).string = null_mut();
        for &cse in &[0i32, 1] {
            assert_eq!(
                (p.c.cJSON_Compare)(co, co2, cse),
                (p.r.cJSON_Compare)(ro, ro2, cse),
                "row 192: Compare with NULL child key (cs={cse})"
            );
            assert_eq!(
                (p.c.cJSON_Compare)(co2, co, cse),
                (p.r.cJSON_Compare)(ro2, ro, cse),
                "row 193: Compare reversed with NULL child key (cs={cse})"
            );
        }
        (*(*co).child).string = ck;
        (*(*ro).child).string = rk;
        (p.c.cJSON_Delete)(co);
        (p.c.cJSON_Delete)(co2);
        (p.r.cJSON_Delete)(ro);
        (p.r.cJSON_Delete)(ro2);

        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);

        // row 195
        (p.c.cJSON_free)(null_mut());
        (p.r.cJSON_free)(null_mut());
    }
}

/* ================================================================== */
/* rows 17-18, 26-27, 165-167: numeric saturation boundaries             */
/* ================================================================== */

#[test]
fn rows_17_18_26_27_165_167_numeric_saturation() {
    let _g = lock();
    let p = pair();
    let boundary: &[f64] = &[
        i32::MAX as f64 - 1.0,
        i32::MAX as f64 - 0.5,
        i32::MAX as f64,
        i32::MAX as f64 + 1.0,
        i32::MAX as f64 + 1000.0,
        i32::MIN as f64 - 1000.0,
        i32::MIN as f64 - 1.0,
        i32::MIN as f64,
        i32::MIN as f64 + 0.5,
        i32::MIN as f64 + 1.0,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1e300,
        -1e300,
        0.0,
        -0.0,
    ];
    unsafe {
        for &d in boundary {
            // rows 165-167
            let c = (p.c.cJSON_CreateNumber)(d);
            let r = (p.r.cJSON_CreateNumber)(d);
            assert_eq!((*c).valueint, (*r).valueint, "rows 165-167: CreateNumber({d:?}).valueint");
            assert_eq!(
                (*c).valuedouble.to_bits(),
                (*r).valuedouble.to_bits(),
                "rows 165-167: CreateNumber({d:?}).valuedouble"
            );
            // rows 26-27
            let cres = (p.c.cJSON_SetNumberHelper)(c, d);
            let rres = (p.r.cJSON_SetNumberHelper)(r, d);
            assert_eq!(cres.to_bits(), rres.to_bits(), "rows 26-27: SetNumberHelper({d:?})");
            assert_eq!((*c).valueint, (*r).valueint, "rows 26-27: valueint({d:?})");
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);
        }
        // rows 17-18: via the parser
        for txt in [
            "2147483646", "2147483647", "2147483648", "2147483647.5", "-2147483647",
            "-2147483648", "-2147483649", "-2147483648.5", "1e300", "-1e300", "1e309",
            "-1e309", "0", "-0", "1e-320",
        ] {
            let b = cbytes(txt.as_bytes());
            let c = (p.c.cJSON_Parse)(b.as_ptr());
            let r = (p.r.cJSON_Parse)(b.as_ptr());
            assert!(!c.is_null() && !r.is_null(), "{txt} should parse");
            assert_eq!((*c).valueint, (*r).valueint, "rows 17-18: {txt} valueint");
            assert_eq!(
                (*c).valuedouble.to_bits(),
                (*r).valuedouble.to_bits(),
                "rows 17-18: {txt} valuedouble"
            );
            assert!(
                take_printed(p.c, (p.c.cJSON_Print)(c)) == take_printed(p.r, (p.r.cJSON_Print)(r)),
                "rows 17-18: {txt} print"
            );
            (p.c.cJSON_Delete)(c);
            (p.r.cJSON_Delete)(r);
        }
    }
}
