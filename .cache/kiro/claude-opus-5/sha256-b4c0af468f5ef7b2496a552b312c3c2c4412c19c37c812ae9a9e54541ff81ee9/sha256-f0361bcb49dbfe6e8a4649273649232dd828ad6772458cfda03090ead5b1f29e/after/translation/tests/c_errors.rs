//! Phase C — one differential test per row of `ERRORS.md`, plus the generic
//! FFI boundary cases (G1-G7).
mod harness;
use harness::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr::{null, null_mut};

/* =================================================================== */
/* Helper: assert both libraries reject an input the same way           */
/* =================================================================== */

macro_rules! same {
    ($what:expr, $c:expr, $r:expr) => {{
        let a = $c;
        let b = $r;
        assert_eq!(a, b, "{}", $what);
    }};
}

/// Both libraries must return the same nullness for a pointer-returning call.
#[allow(unused_macros)]
macro_rules! same_null {
    ($what:expr, $c:expr, $r:expr) => {{
        let a = $c;
        let b = $r;
        assert_eq!(a.is_null(), b.is_null(), "{} (nullness)", $what);
        a.is_null()
    }};
}

/* =================================================================== */
/* Rows 1-2: GetStringValue / GetNumberValue                            */
/* =================================================================== */

#[test]
fn rows01_02_get_value_wrong_type() {
    let _g = lock();
    let p = pair();
    let s = cstr("s");
    unsafe {
        // row 1 + 2 with NULL
        same!(
            "row1 GetStringValue(NULL)",
            (p.c.cJSON_GetStringValue)(null()).is_null(),
            (p.r.cJSON_GetStringValue)(null()).is_null()
        );
        same!(
            "row2 GetNumberValue(NULL) bits",
            (p.c.cJSON_GetNumberValue)(null()).to_bits(),
            (p.r.cJSON_GetNumberValue)(null()).to_bits()
        );
        // and on every non-matching type
        let makers: Vec<Box<dyn Fn(&Api) -> *mut cJSON>> = vec![
            Box::new(|a: &Api| (a.cJSON_CreateNull)()),
            Box::new(|a: &Api| (a.cJSON_CreateTrue)()),
            Box::new(|a: &Api| (a.cJSON_CreateFalse)()),
            Box::new(|a: &Api| (a.cJSON_CreateNumber)(3.5)),
            Box::new(|a: &Api| (a.cJSON_CreateArray)()),
            Box::new(|a: &Api| (a.cJSON_CreateObject)()),
        ];
        for (i, m) in makers.iter().enumerate() {
            let ci = m(&p.c);
            let ri = m(&p.r);
            same!(
                format!("row1 GetStringValue type {i}"),
                read_cstr((p.c.cJSON_GetStringValue)(ci)),
                read_cstr((p.r.cJSON_GetStringValue)(ri))
            );
            same!(
                format!("row2 GetNumberValue type {i}"),
                (p.c.cJSON_GetNumberValue)(ci).to_bits(),
                (p.r.cJSON_GetNumberValue)(ri).to_bits()
            );
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // raw item is not a string for GetStringValue purposes
        let ci = (p.c.cJSON_CreateRaw)(sp(&s));
        let ri = (p.r.cJSON_CreateRaw)(sp(&s));
        same!(
            "row1 GetStringValue(raw)",
            read_cstr((p.c.cJSON_GetStringValue)(ci)),
            read_cstr((p.r.cJSON_GetStringValue)(ri))
        );
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);
    }
}

/* =================================================================== */
/* Rows 3-8: cJSON_SetValuestring rejections                            */
/* =================================================================== */

#[test]
fn rows03_08_set_valuestring_rejections() {
    let _g = lock();
    let p = pair();
    let v = cstr("newvalue");
    let orig = cstr("original");
    unsafe {
        // row 3: object == NULL
        same!(
            "row3 SetValuestring(NULL, s)",
            (p.c.cJSON_SetValuestring)(null_mut(), sp(&v)).is_null(),
            (p.r.cJSON_SetValuestring)(null_mut(), sp(&v)).is_null()
        );
        // row 4: not a string
        for k in 0..5 {
            let mk = |a: &Api| match k {
                0 => (a.cJSON_CreateNumber)(1.0),
                1 => (a.cJSON_CreateNull)(),
                2 => (a.cJSON_CreateArray)(),
                3 => (a.cJSON_CreateObject)(),
                _ => (a.cJSON_CreateRaw)(sp(&orig)),
            };
            let ci = mk(&p.c);
            let ri = mk(&p.r);
            same!(
                format!("row4 SetValuestring wrong type {k}"),
                read_cstr((p.c.cJSON_SetValuestring)(ci, sp(&v))),
                read_cstr((p.r.cJSON_SetValuestring)(ri, sp(&v)))
            );
            assert_snap_eq(ci, ri, "row4 unchanged");
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // row 5: string REFERENCE (cJSON_IsReference set)
        let ci = (p.c.cJSON_CreateStringReference)(sp(&orig));
        let ri = (p.r.cJSON_CreateStringReference)(sp(&orig));
        same!(
            "row5 SetValuestring on reference",
            read_cstr((p.c.cJSON_SetValuestring)(ci, sp(&v))),
            read_cstr((p.r.cJSON_SetValuestring)(ri, sp(&v)))
        );
        assert_snap_eq(ci, ri, "row5 unchanged");
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);

        // row 6: object->valuestring == NULL on a String-typed item
        let ci = (p.c.cJSON_CreateString)(sp(&orig));
        let ri = (p.r.cJSON_CreateString)(sp(&orig));
        let csaved = (*ci).valuestring;
        let rsaved = (*ri).valuestring;
        (*ci).valuestring = null_mut();
        (*ri).valuestring = null_mut();
        same!(
            "row6 SetValuestring with NULL valuestring",
            (p.c.cJSON_SetValuestring)(ci, sp(&v)).is_null(),
            (p.r.cJSON_SetValuestring)(ri, sp(&v)).is_null()
        );
        (*ci).valuestring = csaved;
        (*ri).valuestring = rsaved;
        // row 7: valuestring argument == NULL
        same!(
            "row7 SetValuestring(item, NULL)",
            (p.c.cJSON_SetValuestring)(ci, null()).is_null(),
            (p.r.cJSON_SetValuestring)(ri, null()).is_null()
        );
        assert_snap_eq(ci, ri, "row7 unchanged");
        // row 8: overlapping buffers (pass the item's own valuestring back in,
        // and an interior pointer of it)
        same!(
            "row8 SetValuestring(item, item->valuestring)",
            read_cstr((p.c.cJSON_SetValuestring)(ci, (*ci).valuestring)),
            read_cstr((p.r.cJSON_SetValuestring)(ri, (*ri).valuestring))
        );
        same!(
            "row8 SetValuestring(item, item->valuestring+2)",
            read_cstr((p.c.cJSON_SetValuestring)(ci, (*ci).valuestring.add(2))),
            read_cstr((p.r.cJSON_SetValuestring)(ri, (*ri).valuestring.add(2)))
        );
        assert_snap_eq(ci, ri, "row8 result");
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);
    }
}

/* =================================================================== */
/* Rows 10-49, 18-49: parse rejections                                  */
/* =================================================================== */

/// Compare every parse entry point's rejection for `bytes`.
fn assert_parse_rejects_same(bytes: &[u8], what: &str) {
    let p = pair();
    let buf = cbytes(bytes);
    let base = buf.as_ptr() as isize;
    let off = |q: *const c_char| -> isize {
        if q.is_null() {
            -1
        } else {
            let d = q as isize - base;
            if d >= 0 && d <= buf.len() as isize {
                d
            } else {
                -2
            }
        }
    };
    unsafe {
        // Parse
        let ci = (p.c.cJSON_Parse)(sp(&buf));
        let ce = off((p.c.cJSON_GetErrorPtr)());
        let ri = (p.r.cJSON_Parse)(sp(&buf));
        let re = off((p.r.cJSON_GetErrorPtr)());
        assert_eq!(ci.is_null(), ri.is_null(), "{what}: Parse nullness");
        assert_eq!(ce, re, "{what}: Parse GetErrorPtr offset");
        if !ci.is_null() {
            assert_snap_eq(ci, ri, what);
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // ParseWithOpts, both require_null_terminated values, with end pointer
        for req in [0i32, 1] {
            let mut cend: *const c_char = null();
            let ci = (p.c.cJSON_ParseWithOpts)(sp(&buf), &mut cend, req);
            let coff = off(cend);
            let cerr = off((p.c.cJSON_GetErrorPtr)());
            let mut rend: *const c_char = null();
            let ri = (p.r.cJSON_ParseWithOpts)(sp(&buf), &mut rend, req);
            let roff = off(rend);
            let rerr = off((p.r.cJSON_GetErrorPtr)());
            assert_eq!(ci.is_null(), ri.is_null(), "{what}: PWO({req}) nullness");
            assert_eq!(coff, roff, "{what}: PWO({req}) return_parse_end");
            assert_eq!(cerr, rerr, "{what}: PWO({req}) GetErrorPtr");
            if !ci.is_null() {
                assert_snap_eq(ci, ri, what);
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
        // ParseWithLengthOpts across lengths, incl. 0 (row 12)
        for len in [0usize, 1, bytes.len(), buf.len()] {
            for req in [0i32, 1] {
                let mut cend: *const c_char = null();
                let ci = (p.c.cJSON_ParseWithLengthOpts)(sp(&buf), len, &mut cend, req);
                let coff = off(cend);
                let cerr = off((p.c.cJSON_GetErrorPtr)());
                let mut rend: *const c_char = null();
                let ri = (p.r.cJSON_ParseWithLengthOpts)(sp(&buf), len, &mut rend, req);
                let roff = off(rend);
                let rerr = off((p.r.cJSON_GetErrorPtr)());
                assert_eq!(
                    ci.is_null(),
                    ri.is_null(),
                    "{what}: PWLO({len},{req}) nullness"
                );
                assert_eq!(coff, roff, "{what}: PWLO({len},{req}) end");
                assert_eq!(cerr, rerr, "{what}: PWLO({len},{req}) errptr");
                if !ci.is_null() {
                    assert_snap_eq(ci, ri, what);
                    (p.c.cJSON_Delete)(ci);
                    (p.r.cJSON_Delete)(ri);
                }
            }
        }
    }
}

#[test]
fn rows10_12_parse_null_and_zero_length() {
    let _g = lock();
    let p = pair();
    let ok = cstr("[1]");
    unsafe {
        // row 10 / 16: cJSON_Parse(NULL), cJSON_ParseWithOpts(NULL, ...)
        same!(
            "row16 Parse(NULL)",
            (p.c.cJSON_Parse)(null()).is_null(),
            (p.r.cJSON_Parse)(null()).is_null()
        );
        for req in [0i32, 1] {
            let mut ce: *const c_char = null();
            let mut re: *const c_char = null();
            same!(
                format!("row10 ParseWithOpts(NULL,end,{req})"),
                (p.c.cJSON_ParseWithOpts)(null(), &mut ce, req).is_null(),
                (p.r.cJSON_ParseWithOpts)(null(), &mut re, req).is_null()
            );
            same!(
                format!("row10 ParseWithOpts(NULL) end nullness"),
                ce.is_null(),
                re.is_null()
            );
            // row 11
            same!(
                format!("row11 ParseWithLengthOpts(NULL,4,end,{req})"),
                (p.c.cJSON_ParseWithLengthOpts)(null(), 4, &mut ce, req).is_null(),
                (p.r.cJSON_ParseWithLengthOpts)(null(), 4, &mut re, req).is_null()
            );
        }
        // row 17
        same!(
            "row17 ParseWithLength(NULL, 4)",
            (p.c.cJSON_ParseWithLength)(null(), 4).is_null(),
            (p.r.cJSON_ParseWithLength)(null(), 4).is_null()
        );
        // row 12: buffer_length == 0 on a perfectly valid document
        same!(
            "row12 ParseWithLength(valid, 0)",
            (p.c.cJSON_ParseWithLength)(sp(&ok), 0).is_null(),
            (p.r.cJSON_ParseWithLength)(sp(&ok), 0).is_null()
        );
        for req in [0i32, 1] {
            let mut ce: *const c_char = null();
            let mut re: *const c_char = null();
            same!(
                format!("row12 PWLO(valid,0,end,{req})"),
                (p.c.cJSON_ParseWithLengthOpts)(sp(&ok), 0, &mut ce, req).is_null(),
                (p.r.cJSON_ParseWithLengthOpts)(sp(&ok), 0, &mut re, req).is_null()
            );
        }
    }
}

#[test]
fn rows13_14_18_19_number_and_trailing_rejections() {
    let _g = lock();
    // row 19: strtod consumes nothing
    for bad in [
        "-", ".", "e5", "E5", "+", "-.", "-e", "- 1", "--1", "-.e", ".e5", "e", "-e5",
    ] {
        assert_parse_rejects_same(bad.as_bytes(), &format!("row19 number {bad:?}"));
    }
    // row 13/14: malformed and trailing garbage
    for bad in [
        "", " ", "[1,2,3]x", "[1,2,3] x", "1 2", "nullx", "truex", "falsex", "{}[]",
        "\"a\"\"b\"", "[1]]", "{}}",
    ] {
        assert_parse_rejects_same(bad.as_bytes(), &format!("row13/14 {bad:?}"));
    }
}

#[test]
fn rows21_27_unicode_escape_rejections() {
    let _g = lock();
    let cases: Vec<&[u8]> = vec![
        br#""\uZZZZ""#,      // row 21: non-hex digit
        br#""\uD83G""#,      // row 21
        br#""\u00g0""#,      // row 21
        br#""\u12""#,        // row 22: input ends inside \u
        br#""\u1""#,         // row 22
        br#""\u""#,          // row 22
        br#""\uDC00""#,      // row 23: lone low surrogate
        br#""\uDFFF""#,      // row 23
        br#""\uD800""#,      // row 24: high surrogate, nothing follows
        br#""\uD800\u""#,    // row 24
        br#""\uD800\u12""#,  // row 24
        br#""\uD800x1234""#, // row 25: second escape not \u
        br#""\uD800\n""#,    // row 25
        br#""\uD800\\u1234""#, // row 25
        br#""\uD800\u0041""#, // row 26: second code not a low surrogate
        br#""\uD800\uD800""#, // row 26
        br#""\uD800\uE000""#, // row 26
    ];
    for c in cases {
        assert_parse_rejects_same(c, &format!("unicode {:?}", String::from_utf8_lossy(c)));
    }
}

#[test]
fn rows28_32_string_parse_rejections() {
    let _g = lock();
    let cases: Vec<&[u8]> = vec![
        b"'a'",         // row 28: not a string where a string is required
        b"{a:1}",       // row 45 (key not a string)
        b"{1:2}",       // row 45
        br#""abc"#,     // row 30: unterminated
        br#"""#,        // row 30
        br#""a\"#,      // row 29/31: trailing backslash
        br#""\"#,       // row 29
        br#""\x""#,     // row 32: unknown escape
        br#""\a""#,     // row 32
        br#""\q1234""#, // row 32
        br#""\0""#,     // row 32
    ];
    for c in cases {
        assert_parse_rejects_same(c, &format!("string {:?}", String::from_utf8_lossy(c)));
    }
}

#[test]
fn rows34_35_value_dispatch_rejections() {
    let _g = lock();
    for bad in [
        "x", "'", "+1", "nul", "tru", "fals", "NULL", "True", "None", "undefined", "]", "}",
        ",", ":", "*", "\x01", "\x7f",
    ] {
        assert_parse_rejects_same(bad.as_bytes(), &format!("row35 {bad:?}"));
    }
}

#[test]
fn rows36_41_array_parse_rejections() {
    let _g = lock();
    let mut cases: Vec<Vec<u8>> = vec![
        b"[".to_vec(),        // row 38/40
        b"[1".to_vec(),       // row 40
        b"[1,".to_vec(),      // row 38
        b"[1,]".to_vec(),     // row 39
        b"[,1]".to_vec(),     // row 39
        b"[1 2]".to_vec(),    // row 40
        b"[1}".to_vec(),      // row 40
        b"[x]".to_vec(),      // row 39
        b"[[]".to_vec(),      // row 40
        b"[1,2,".to_vec(),    // row 38
        b"[1,,2]".to_vec(),   // row 39
        b"]".to_vec(),        // row 37
    ];
    // row 36: exceed CJSON_NESTING_LIMIT
    for depth in [1000usize, 1001, 1200] {
        let mut s = vec![b'['; depth];
        s.push(b'1');
        s.extend(std::iter::repeat(b']').take(depth));
        cases.push(s);
    }
    for c in &cases {
        assert_parse_rejects_same(c, &format!("array {:?}", String::from_utf8_lossy(&c[..c.len().min(40)])));
    }
}

#[test]
fn rows42_49_object_parse_rejections() {
    let _g = lock();
    let mut cases: Vec<Vec<u8>> = vec![
        b"{".to_vec(),               // row 44/48
        b"{\"a\"".to_vec(),          // row 46
        b"{\"a\":".to_vec(),         // row 47
        b"{\"a\":1".to_vec(),        // row 48
        b"{\"a\":1,".to_vec(),       // row 44
        b"{\"a\":1,}".to_vec(),      // row 45
        b"{,}".to_vec(),             // row 45
        b"{\"a\" 1}".to_vec(),       // row 46
        b"{\"a\"=1}".to_vec(),       // row 46
        b"{\"a\":x}".to_vec(),       // row 47
        b"{\"a\":1]".to_vec(),       // row 48
        b"{1:2}".to_vec(),           // row 45
        b"{\"a\":1 \"b\":2}".to_vec(), // row 48
        b"}".to_vec(),               // row 43
    ];
    // row 42: exceed CJSON_NESTING_LIMIT with objects
    for depth in [1000usize, 1001, 1200] {
        let mut s = Vec::new();
        for _ in 0..depth {
            s.extend_from_slice(b"{\"a\":");
        }
        s.push(b'1');
        s.extend(std::iter::repeat(b'}').take(depth));
        cases.push(s);
    }
    for c in &cases {
        assert_parse_rejects_same(
            c,
            &format!("object {:?}", String::from_utf8_lossy(&c[..c.len().min(40)])),
        );
    }
}

/* =================================================================== */
/* Rows 50-67: print rejections                                         */
/* =================================================================== */

#[test]
fn rows50_58_print_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        // rows 50/51/54/65: NULL item
        same!(
            "row65 Print(NULL)",
            (p.c.cJSON_Print)(null()).is_null(),
            (p.r.cJSON_Print)(null()).is_null()
        );
        same!(
            "row65 PrintUnformatted(NULL)",
            (p.c.cJSON_PrintUnformatted)(null()).is_null(),
            (p.r.cJSON_PrintUnformatted)(null()).is_null()
        );
        for pre in [0i32, 16] {
            for fmt in [0i32, 1] {
                same!(
                    format!("row54 PrintBuffered(NULL,{pre},{fmt})"),
                    (p.c.cJSON_PrintBuffered)(null(), pre, fmt).is_null(),
                    (p.r.cJSON_PrintBuffered)(null(), pre, fmt).is_null()
                );
            }
        }
        // row 53: prebuffer < 0
        let item_c = (p.c.cJSON_CreateNumber)(1.0);
        let item_r = (p.r.cJSON_CreateNumber)(1.0);
        for pre in [-1i32, -2, i32::MIN, -100000] {
            for fmt in [0i32, 1] {
                same!(
                    format!("row53 PrintBuffered(item,{pre},{fmt})"),
                    (p.c.cJSON_PrintBuffered)(item_c, pre, fmt).is_null(),
                    (p.r.cJSON_PrintBuffered)(item_r, pre, fmt).is_null()
                );
            }
        }
        // rows 56/57/58: PrintPreallocated
        let mut cbuf = [0u8; 64];
        let mut rbuf = [0u8; 64];
        for len in [-1i32, -2, i32::MIN] {
            for fmt in [0i32, 1] {
                same!(
                    format!("row56 PrintPreallocated(len={len})"),
                    (p.c.cJSON_PrintPreallocated)(
                        item_c,
                        cbuf.as_mut_ptr() as *mut c_char,
                        len,
                        fmt
                    ),
                    (p.r.cJSON_PrintPreallocated)(
                        item_r,
                        rbuf.as_mut_ptr() as *mut c_char,
                        len,
                        fmt
                    )
                );
            }
        }
        for len in [0i32, 1, 16] {
            for fmt in [0i32, 1] {
                // row 57: buffer == NULL
                same!(
                    format!("row57 PrintPreallocated(NULL buffer, {len})"),
                    (p.c.cJSON_PrintPreallocated)(item_c, null_mut(), len, fmt),
                    (p.r.cJSON_PrintPreallocated)(item_r, null_mut(), len, fmt)
                );
            }
        }
        // row 65: NULL item to PrintPreallocated
        for fmt in [0i32, 1] {
            same!(
                format!("row65 PrintPreallocated(NULL item, fmt={fmt})"),
                (p.c.cJSON_PrintPreallocated)(
                    null_mut(),
                    cbuf.as_mut_ptr() as *mut c_char,
                    64,
                    fmt
                ),
                (p.r.cJSON_PrintPreallocated)(
                    null_mut(),
                    rbuf.as_mut_ptr() as *mut c_char,
                    64,
                    fmt
                )
            );
        }
        // rows 58/62: buffer too small -> ensure() hits noalloc
        let big = format!("[{}]", (0..200).map(|i| i.to_string()).collect::<Vec<_>>().join(","));
        let bs = cstr(&big);
        let ci = (p.c.cJSON_Parse)(sp(&bs));
        let ri = (p.r.cJSON_Parse)(sp(&bs));
        for len in [1i32, 2, 5, 10, 100, 500] {
            for fmt in [0i32, 1] {
                let mut cb = vec![0x5Au8; 4096];
                let mut rb = vec![0x5Au8; 4096];
                same!(
                    format!("row58 PrintPreallocated small len={len} fmt={fmt}"),
                    (p.c.cJSON_PrintPreallocated)(ci, cb.as_mut_ptr() as *mut c_char, len, fmt),
                    (p.r.cJSON_PrintPreallocated)(ri, rb.as_mut_ptr() as *mut c_char, len, fmt)
                );
                assert_eq!(cb, rb, "row58 partial buffer contents len={len} fmt={fmt}");
            }
        }
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);
        (p.c.cJSON_Delete)(item_c);
        (p.r.cJSON_Delete)(item_r);
    }
}

#[test]
fn rows66_67_print_raw_null_and_invalid_type() {
    let _g = lock();
    let p = pair();
    unsafe {
        // row 66: cJSON_Raw with NULL valuestring
        let raw = cstr("[1]");
        let ci = (p.c.cJSON_CreateRaw)(sp(&raw));
        let ri = (p.r.cJSON_CreateRaw)(sp(&raw));
        let cs = (*ci).valuestring;
        let rs = (*ri).valuestring;
        (*ci).valuestring = null_mut();
        (*ri).valuestring = null_mut();
        assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri), "row66 raw NULL");
        // nested in a container too
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        (p.c.cJSON_AddItemToArray)(ca, ci);
        (p.r.cJSON_AddItemToArray)(ra, ri);
        assert_eq!(print_all(&p.c, ca), print_all(&p.r, ra), "row66 raw NULL nested");
        (*ci).valuestring = cs;
        (*ri).valuestring = rs;
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);

        // row 67: cJSON_Invalid / unknown type byte
        for ty in [cJSON_Invalid, 7, 0x40 | 0x20, 0x0f] {
            let ci = (p.c.cJSON_CreateNumber)(1.0);
            let ri = (p.r.cJSON_CreateNumber)(1.0);
            (*ci).type_ = ty;
            (*ri).type_ = ty;
            assert_eq!(
                print_all(&p.c, ci),
                print_all(&p.r, ri),
                "row67 print type {ty:#x}"
            );
            (*ci).type_ = cJSON_Number;
            (*ri).type_ = cJSON_Number;
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
    }
}

/* =================================================================== */
/* Rows 68-74: accessor rejections                                      */
/* =================================================================== */

#[test]
fn rows68_74_accessor_rejections() {
    let _g = lock();
    let p = pair();
    let key = cstr("k");
    unsafe {
        // row 68
        same!(
            "row68 GetArraySize(NULL)",
            (p.c.cJSON_GetArraySize)(null()),
            (p.r.cJSON_GetArraySize)(null())
        );
        // rows 69/70/71
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        for i in 0..3 {
            (p.c.cJSON_AddItemToArray)(ca, (p.c.cJSON_CreateNumber)(i as f64));
            (p.r.cJSON_AddItemToArray)(ra, (p.r.cJSON_CreateNumber)(i as f64));
        }
        for idx in [-1i32, -2, i32::MIN, 3, 4, 100, i32::MAX] {
            same!(
                format!("row69/71 GetArrayItem({idx})"),
                (p.c.cJSON_GetArrayItem)(ca, idx).is_null(),
                (p.r.cJSON_GetArrayItem)(ra, idx).is_null()
            );
            // row 70: NULL array
            same!(
                format!("row70 GetArrayItem(NULL,{idx})"),
                (p.c.cJSON_GetArrayItem)(null(), idx).is_null(),
                (p.r.cJSON_GetArrayItem)(null(), idx).is_null()
            );
        }
        // rows 72/73/74
        for (obj_null, key_null) in [(true, false), (false, true), (true, true)] {
            let co = if obj_null { null() } else { ca as *const cJSON };
            let ro = if obj_null { null() } else { ra as *const cJSON };
            let ck = if key_null { null() } else { sp(&key) };
            same!(
                format!("row72 GetObjectItem({obj_null},{key_null})"),
                (p.c.cJSON_GetObjectItem)(co, ck).is_null(),
                (p.r.cJSON_GetObjectItem)(ro, ck).is_null()
            );
            same!(
                format!("row72 GetObjectItemCS({obj_null},{key_null})"),
                (p.c.cJSON_GetObjectItemCaseSensitive)(co, ck).is_null(),
                (p.r.cJSON_GetObjectItemCaseSensitive)(ro, ck).is_null()
            );
            same!(
                format!("row74 HasObjectItem({obj_null},{key_null})"),
                (p.c.cJSON_HasObjectItem)(co, ck),
                (p.r.cJSON_HasObjectItem)(ro, ck)
            );
        }
        // row 73: absent key
        let absent = cstr("nope");
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        (p.c.cJSON_AddNumberToObject)(co, sp(&key), 1.0);
        (p.r.cJSON_AddNumberToObject)(ro, sp(&key), 1.0);
        same!(
            "row73 GetObjectItem absent",
            (p.c.cJSON_GetObjectItem)(co, sp(&absent)).is_null(),
            (p.r.cJSON_GetObjectItem)(ro, sp(&absent)).is_null()
        );
        same!(
            "row73 GetObjectItemCS absent",
            (p.c.cJSON_GetObjectItemCaseSensitive)(co, sp(&absent)).is_null(),
            (p.r.cJSON_GetObjectItemCaseSensitive)(ro, sp(&absent)).is_null()
        );
        same!(
            "row74 HasObjectItem absent",
            (p.c.cJSON_HasObjectItem)(co, sp(&absent)),
            (p.r.cJSON_HasObjectItem)(ro, sp(&absent))
        );
        // objects whose members have NULL ->string (row 73 second clause)
        let ca2 = (p.c.cJSON_CreateArray)();
        let ra2 = (p.r.cJSON_CreateArray)();
        (p.c.cJSON_AddItemToArray)(ca2, (p.c.cJSON_CreateNumber)(1.0));
        (p.r.cJSON_AddItemToArray)(ra2, (p.r.cJSON_CreateNumber)(1.0));
        same!(
            "row73 GetObjectItem on array (members have no ->string)",
            (p.c.cJSON_GetObjectItem)(ca2, sp(&key)).is_null(),
            (p.r.cJSON_GetObjectItem)(ra2, sp(&key)).is_null()
        );
        (p.c.cJSON_Delete)(ca2);
        (p.r.cJSON_Delete)(ra2);
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);
    }
}

/* =================================================================== */
/* Rows 75-98: add* rejections                                          */
/* =================================================================== */

#[test]
fn rows75_87_add_rejections() {
    let _g = lock();
    let p = pair();
    let key = cstr("k");
    unsafe {
        let ca = (p.c.cJSON_CreateArray)();
        let ra = (p.r.cJSON_CreateArray)();
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        // row 75: item == NULL
        same!(
            "row75 AddItemToArray(arr,NULL)",
            (p.c.cJSON_AddItemToArray)(ca, null_mut()),
            (p.r.cJSON_AddItemToArray)(ra, null_mut())
        );
        // row 76: array == NULL
        let cn = (p.c.cJSON_CreateNumber)(1.0);
        let rn = (p.r.cJSON_CreateNumber)(1.0);
        same!(
            "row76 AddItemToArray(NULL,item)",
            (p.c.cJSON_AddItemToArray)(null_mut(), cn),
            (p.r.cJSON_AddItemToArray)(null_mut(), rn)
        );
        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);
        // row 77: self reference
        same!(
            "row77 AddItemToArray(arr,arr)",
            (p.c.cJSON_AddItemToArray)(ca, ca),
            (p.r.cJSON_AddItemToArray)(ra, ra)
        );
        // row 78: create_reference(NULL) via AddItemReference*
        same!(
            "row78 AddItemReferenceToArray(arr,NULL)",
            (p.c.cJSON_AddItemReferenceToArray)(ca, null_mut()),
            (p.r.cJSON_AddItemReferenceToArray)(ra, null_mut())
        );
        same!(
            "row78 AddItemReferenceToObject(obj,k,NULL)",
            (p.c.cJSON_AddItemReferenceToObject)(co, sp(&key), null_mut()),
            (p.r.cJSON_AddItemReferenceToObject)(ro, sp(&key), null_mut())
        );
        // row 85/86/87
        let cn = (p.c.cJSON_CreateNumber)(1.0);
        let rn = (p.r.cJSON_CreateNumber)(1.0);
        same!(
            "row85 AddItemReferenceToArray(NULL,item)",
            (p.c.cJSON_AddItemReferenceToArray)(null_mut(), cn),
            (p.r.cJSON_AddItemReferenceToArray)(null_mut(), rn)
        );
        same!(
            "row86 AddItemReferenceToObject(NULL,k,item)",
            (p.c.cJSON_AddItemReferenceToObject)(null_mut(), sp(&key), cn),
            (p.r.cJSON_AddItemReferenceToObject)(null_mut(), sp(&key), rn)
        );
        same!(
            "row87 AddItemReferenceToObject(obj,NULL,item)",
            (p.c.cJSON_AddItemReferenceToObject)(co, null(), cn),
            (p.r.cJSON_AddItemReferenceToObject)(ro, null(), rn)
        );
        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);
        // rows 80-83: add_item_to_object rejections
        for (obj_null, key_null, item_null, selfref) in [
            (true, false, false, false),
            (false, true, false, false),
            (false, false, true, false),
            (false, false, false, true),
        ] {
            let cobj = if obj_null { null_mut() } else { co };
            let robj = if obj_null { null_mut() } else { ro };
            let ck = if key_null { null() } else { sp(&key) };
            let (citem, ritem) = if item_null {
                (null_mut(), null_mut())
            } else if selfref {
                (co, ro)
            } else {
                ((p.c.cJSON_CreateNumber)(2.0), (p.r.cJSON_CreateNumber)(2.0))
            };
            let cres = (p.c.cJSON_AddItemToObject)(cobj, ck, citem);
            let rres = (p.r.cJSON_AddItemToObject)(robj, ck, ritem);
            assert_eq!(
                cres, rres,
                "rows80-83 AddItemToObject({obj_null},{key_null},{item_null},{selfref})"
            );
            let cres = (p.c.cJSON_AddItemToObjectCS)(cobj, ck, citem);
            let rres = (p.r.cJSON_AddItemToObjectCS)(robj, ck, ritem);
            assert_eq!(
                cres, rres,
                "rows80-83 AddItemToObjectCS({obj_null},{key_null},{item_null},{selfref})"
            );
            if !item_null && !selfref && cres == 0 {
                (p.c.cJSON_Delete)(citem);
                (p.r.cJSON_Delete)(ritem);
            }
        }
        assert_snap_eq(ca, ra, "array after rejected adds");
        assert_snap_eq(co, ro, "object after rejected adds");
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);
    }
}

#[test]
fn rows88_98_add_wrapper_rejections() {
    let _g = lock();
    let p = pair();
    let key = cstr("k");
    let val = cstr("v");
    unsafe {
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        for (label, obj_null, key_null) in [
            ("NULL object", true, false),
            ("NULL name", false, true),
            ("both NULL", true, true),
        ] {
            let cobj = if obj_null { null_mut() } else { co };
            let robj = if obj_null { null_mut() } else { ro };
            let ck = if key_null { null() } else { sp(&key) };
            same!(
                format!("row88 AddNullToObject {label}"),
                (p.c.cJSON_AddNullToObject)(cobj, ck).is_null(),
                (p.r.cJSON_AddNullToObject)(robj, ck).is_null()
            );
            same!(
                format!("row89 AddTrueToObject {label}"),
                (p.c.cJSON_AddTrueToObject)(cobj, ck).is_null(),
                (p.r.cJSON_AddTrueToObject)(robj, ck).is_null()
            );
            same!(
                format!("row90 AddFalseToObject {label}"),
                (p.c.cJSON_AddFalseToObject)(cobj, ck).is_null(),
                (p.r.cJSON_AddFalseToObject)(robj, ck).is_null()
            );
            same!(
                format!("row91 AddBoolToObject {label}"),
                (p.c.cJSON_AddBoolToObject)(cobj, ck, 1).is_null(),
                (p.r.cJSON_AddBoolToObject)(robj, ck, 1).is_null()
            );
            same!(
                format!("row92 AddNumberToObject {label}"),
                (p.c.cJSON_AddNumberToObject)(cobj, ck, 1.0).is_null(),
                (p.r.cJSON_AddNumberToObject)(robj, ck, 1.0).is_null()
            );
            same!(
                format!("row93 AddStringToObject {label}"),
                (p.c.cJSON_AddStringToObject)(cobj, ck, sp(&val)).is_null(),
                (p.r.cJSON_AddStringToObject)(robj, ck, sp(&val)).is_null()
            );
            same!(
                format!("row95 AddRawToObject {label}"),
                (p.c.cJSON_AddRawToObject)(cobj, ck, sp(&val)).is_null(),
                (p.r.cJSON_AddRawToObject)(robj, ck, sp(&val)).is_null()
            );
            same!(
                format!("row97 AddObjectToObject {label}"),
                (p.c.cJSON_AddObjectToObject)(cobj, ck).is_null(),
                (p.r.cJSON_AddObjectToObject)(robj, ck).is_null()
            );
            same!(
                format!("row98 AddArrayToObject {label}"),
                (p.c.cJSON_AddArrayToObject)(cobj, ck).is_null(),
                (p.r.cJSON_AddArrayToObject)(robj, ck).is_null()
            );
        }
        // rows 94 / 96 / 122 / 123: NULL string payloads
        same!(
            "row94 AddStringToObject(obj,k,NULL)",
            (p.c.cJSON_AddStringToObject)(co, sp(&key), null()).is_null(),
            (p.r.cJSON_AddStringToObject)(ro, sp(&key), null()).is_null()
        );
        same!(
            "row96 AddRawToObject(obj,k,NULL)",
            (p.c.cJSON_AddRawToObject)(co, sp(&key), null()).is_null(),
            (p.r.cJSON_AddRawToObject)(ro, sp(&key), null()).is_null()
        );
        assert_snap_eq(co, ro, "object after rejected wrapper adds");
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);
    }
}

/* =================================================================== */
/* Rows 99-121: detach / delete / insert / replace rejections            */
/* =================================================================== */

#[test]
fn rows99_121_mutation_rejections() {
    let _g = lock();
    let p = pair();
    let key = cstr("k0");
    let absent = cstr("zzz");
    unsafe {
        let build = |a: &Api| {
            let arr = (a.cJSON_CreateArray)();
            for i in 0..3 {
                (a.cJSON_AddItemToArray)(arr, (a.cJSON_CreateNumber)(i as f64));
            }
            arr
        };
        let build_obj = |a: &Api| {
            let o = (a.cJSON_CreateObject)();
            for i in 0..3 {
                let k = cbytes(format!("k{i}").as_bytes());
                (a.cJSON_AddItemToObject)(o, sp(&k), (a.cJSON_CreateNumber)(i as f64));
            }
            o
        };
        let ca = build(&p.c);
        let ra = build(&p.r);

        // row 99: parent NULL
        let ct = (p.c.cJSON_GetArrayItem)(ca, 0);
        let rt = (p.r.cJSON_GetArrayItem)(ra, 0);
        same!(
            "row99 DetachItemViaPointer(NULL,item)",
            (p.c.cJSON_DetachItemViaPointer)(null_mut(), ct).is_null(),
            (p.r.cJSON_DetachItemViaPointer)(null_mut(), rt).is_null()
        );
        // row 100: item NULL
        same!(
            "row100 DetachItemViaPointer(parent,NULL)",
            (p.c.cJSON_DetachItemViaPointer)(ca, null_mut()).is_null(),
            (p.r.cJSON_DetachItemViaPointer)(ra, null_mut()).is_null()
        );
        // row 101: item not a member (prev == NULL and != parent->child)
        let cfree = (p.c.cJSON_CreateNumber)(99.0);
        let rfree = (p.r.cJSON_CreateNumber)(99.0);
        same!(
            "row101 DetachItemViaPointer(parent, non-member)",
            (p.c.cJSON_DetachItemViaPointer)(ca, cfree).is_null(),
            (p.r.cJSON_DetachItemViaPointer)(ra, rfree).is_null()
        );
        (p.c.cJSON_Delete)(cfree);
        (p.r.cJSON_Delete)(rfree);

        // rows 102/103
        for which in [-1i32, -2, i32::MIN, 3, 4, i32::MAX] {
            same!(
                format!("row102/103 DetachItemFromArray({which})"),
                (p.c.cJSON_DetachItemFromArray)(ca, which).is_null(),
                (p.r.cJSON_DetachItemFromArray)(ra, which).is_null()
            );
            // row 105: DeleteItemFromArray out of range is a no-op
            (p.c.cJSON_DeleteItemFromArray)(ca, which);
            (p.r.cJSON_DeleteItemFromArray)(ra, which);
            assert_snap_eq(ca, ra, &format!("row105 after DeleteItemFromArray({which})"));
            // row 116/117
            let cn = (p.c.cJSON_CreateNumber)(7.0);
            let rn = (p.r.cJSON_CreateNumber)(7.0);
            same!(
                format!("row116/117 ReplaceItemInArray({which})"),
                (p.c.cJSON_ReplaceItemInArray)(ca, which, cn),
                (p.r.cJSON_ReplaceItemInArray)(ra, which, rn)
            );
            // row 107: which < 0 for insert
            let cn2 = (p.c.cJSON_CreateNumber)(8.0);
            let rn2 = (p.r.cJSON_CreateNumber)(8.0);
            let cres = (p.c.cJSON_InsertItemInArray)(ca, which, cn2);
            let rres = (p.r.cJSON_InsertItemInArray)(ra, which, rn2);
            assert_eq!(cres, rres, "row107/109 InsertItemInArray({which})");
            if cres == 0 {
                (p.c.cJSON_Delete)(cn2);
                (p.r.cJSON_Delete)(rn2);
            }
            assert_snap_eq(ca, ra, &format!("after ops which={which}"));
        }
        // row 108: newitem == NULL
        same!(
            "row108 InsertItemInArray(arr,0,NULL)",
            (p.c.cJSON_InsertItemInArray)(ca, 0, null_mut()),
            (p.r.cJSON_InsertItemInArray)(ra, 0, null_mut())
        );
        // NULL array to insert / replace
        let cn = (p.c.cJSON_CreateNumber)(1.0);
        let rn = (p.r.cJSON_CreateNumber)(1.0);
        same!(
            "InsertItemInArray(NULL,0,item)",
            (p.c.cJSON_InsertItemInArray)(null_mut(), 0, cn),
            (p.r.cJSON_InsertItemInArray)(null_mut(), 0, rn)
        );
        same!(
            "ReplaceItemInArray(NULL,0,item)",
            (p.c.cJSON_ReplaceItemInArray)(null_mut(), 0, cn),
            (p.r.cJSON_ReplaceItemInArray)(null_mut(), 0, rn)
        );
        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);

        // rows 111-115: ReplaceItemViaPointer
        let ca = build(&p.c);
        let ra = build(&p.r);
        let cempty = (p.c.cJSON_CreateArray)();
        let rempty = (p.r.cJSON_CreateArray)();
        let ct = (p.c.cJSON_GetArrayItem)(ca, 1);
        let rt = (p.r.cJSON_GetArrayItem)(ra, 1);
        let crep = (p.c.cJSON_CreateNumber)(42.0);
        let rrep = (p.r.cJSON_CreateNumber)(42.0);
        same!(
            "row111 ReplaceItemViaPointer(NULL,item,rep)",
            (p.c.cJSON_ReplaceItemViaPointer)(null_mut(), ct, crep),
            (p.r.cJSON_ReplaceItemViaPointer)(null_mut(), rt, rrep)
        );
        same!(
            "row112 ReplaceItemViaPointer(empty,item,rep)",
            (p.c.cJSON_ReplaceItemViaPointer)(cempty, ct, crep),
            (p.r.cJSON_ReplaceItemViaPointer)(rempty, rt, rrep)
        );
        same!(
            "row113 ReplaceItemViaPointer(parent,item,NULL)",
            (p.c.cJSON_ReplaceItemViaPointer)(ca, ct, null_mut()),
            (p.r.cJSON_ReplaceItemViaPointer)(ra, rt, null_mut())
        );
        same!(
            "row114 ReplaceItemViaPointer(parent,NULL,rep)",
            (p.c.cJSON_ReplaceItemViaPointer)(ca, null_mut(), crep),
            (p.r.cJSON_ReplaceItemViaPointer)(ra, null_mut(), rrep)
        );
        // row 115: replacement == item -> true
        same!(
            "row115 ReplaceItemViaPointer(parent,item,item)",
            (p.c.cJSON_ReplaceItemViaPointer)(ca, ct, ct),
            (p.r.cJSON_ReplaceItemViaPointer)(ra, rt, rt)
        );
        assert_snap_eq(ca, ra, "rows111-115 unchanged");
        (p.c.cJSON_Delete)(crep);
        (p.r.cJSON_Delete)(rrep);
        (p.c.cJSON_Delete)(cempty);
        (p.r.cJSON_Delete)(rempty);
        (p.c.cJSON_Delete)(ca);
        (p.r.cJSON_Delete)(ra);

        // rows 104/106/118/119/120
        let co = build_obj(&p.c);
        let ro = build_obj(&p.r);
        for (obj_null, key_use) in [(true, 0), (false, 1), (false, 2), (true, 1)] {
            let cobj = if obj_null { null_mut() } else { co };
            let robj = if obj_null { null_mut() } else { ro };
            let ck = match key_use {
                0 => sp(&key),
                1 => null(),
                _ => sp(&absent),
            };
            same!(
                format!("row104 DetachItemFromObject({obj_null},{key_use})"),
                (p.c.cJSON_DetachItemFromObject)(cobj, ck).is_null(),
                (p.r.cJSON_DetachItemFromObject)(robj, ck).is_null()
            );
            same!(
                format!("row104 DetachItemFromObjectCS({obj_null},{key_use})"),
                (p.c.cJSON_DetachItemFromObjectCaseSensitive)(cobj, ck).is_null(),
                (p.r.cJSON_DetachItemFromObjectCaseSensitive)(robj, ck).is_null()
            );
            // row 106: void no-ops
            (p.c.cJSON_DeleteItemFromObject)(cobj, ck);
            (p.r.cJSON_DeleteItemFromObject)(robj, ck);
            (p.c.cJSON_DeleteItemFromObjectCaseSensitive)(cobj, ck);
            (p.r.cJSON_DeleteItemFromObjectCaseSensitive)(robj, ck);
            // rows 118/119/120
            let cn = (p.c.cJSON_CreateNumber)(5.0);
            let rn = (p.r.cJSON_CreateNumber)(5.0);
            let cres = (p.c.cJSON_ReplaceItemInObject)(cobj, ck, cn);
            let rres = (p.r.cJSON_ReplaceItemInObject)(robj, ck, rn);
            assert_eq!(
                cres, rres,
                "rows118-120 ReplaceItemInObject({obj_null},{key_use})"
            );
            if cres == 0 {
                (p.c.cJSON_Delete)(cn);
                (p.r.cJSON_Delete)(rn);
            }
            let cn = (p.c.cJSON_CreateNumber)(6.0);
            let rn = (p.r.cJSON_CreateNumber)(6.0);
            let cres = (p.c.cJSON_ReplaceItemInObjectCaseSensitive)(cobj, ck, cn);
            let rres = (p.r.cJSON_ReplaceItemInObjectCaseSensitive)(robj, ck, rn);
            assert_eq!(cres, rres, "rows118-120 ReplaceItemInObjectCS");
            if cres == 0 {
                (p.c.cJSON_Delete)(cn);
                (p.r.cJSON_Delete)(rn);
            }
            assert_snap_eq(co, ro, "rows104-120 object state");
        }
        // row 118: replacement NULL
        same!(
            "row118 ReplaceItemInObject(obj,k,NULL)",
            (p.c.cJSON_ReplaceItemInObject)(co, sp(&key), null_mut()),
            (p.r.cJSON_ReplaceItemInObject)(ro, sp(&key), null_mut())
        );
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);
    }
}

/* =================================================================== */
/* Rows 122-134: constructor rejections                                 */
/* =================================================================== */

#[test]
fn rows122_134_constructor_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        // rows 122/123
        same!(
            "row122 CreateString(NULL)",
            (p.c.cJSON_CreateString)(null()).is_null(),
            (p.r.cJSON_CreateString)(null()).is_null()
        );
        same!(
            "row123 CreateRaw(NULL)",
            (p.c.cJSON_CreateRaw)(null()).is_null(),
            (p.r.cJSON_CreateRaw)(null()).is_null()
        );
        // CreateStringReference(NULL) does NOT strdup: compare snapshot
        let ci = (p.c.cJSON_CreateStringReference)(null());
        let ri = (p.r.cJSON_CreateStringReference)(null());
        assert_eq!(ci.is_null(), ri.is_null(), "CreateStringReference(NULL)");
        if !ci.is_null() {
            assert_snap_eq(ci, ri, "CreateStringReference(NULL)");
            assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        for f in 0..2 {
            let (ci, ri) = if f == 0 {
                (
                    (p.c.cJSON_CreateObjectReference)(null()),
                    (p.r.cJSON_CreateObjectReference)(null()),
                )
            } else {
                (
                    (p.c.cJSON_CreateArrayReference)(null()),
                    (p.r.cJSON_CreateArrayReference)(null()),
                )
            };
            assert_eq!(ci.is_null(), ri.is_null(), "Create*Reference(NULL) {f}");
            if !ci.is_null() {
                assert_snap_eq(ci, ri, "Create*Reference(NULL)");
                assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri));
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
        // rows 125-132: count < 0 / numbers == NULL
        let ints = [1i32, 2, 3];
        let floats = [1.0f32, 2.0];
        let doubles = [1.0f64, 2.0];
        let sbuf = [cstr("a"), cstr("b")];
        let sptrs: Vec<*const c_char> = sbuf.iter().map(|b| sp(b)).collect();
        for count in [-1i32, -2, i32::MIN] {
            same!(
                format!("row125 CreateIntArray(ptr,{count})"),
                (p.c.cJSON_CreateIntArray)(ints.as_ptr(), count).is_null(),
                (p.r.cJSON_CreateIntArray)(ints.as_ptr(), count).is_null()
            );
            same!(
                format!("row127 CreateFloatArray(ptr,{count})"),
                (p.c.cJSON_CreateFloatArray)(floats.as_ptr(), count).is_null(),
                (p.r.cJSON_CreateFloatArray)(floats.as_ptr(), count).is_null()
            );
            same!(
                format!("row129 CreateDoubleArray(ptr,{count})"),
                (p.c.cJSON_CreateDoubleArray)(doubles.as_ptr(), count).is_null(),
                (p.r.cJSON_CreateDoubleArray)(doubles.as_ptr(), count).is_null()
            );
            same!(
                format!("row131 CreateStringArray(ptr,{count})"),
                (p.c.cJSON_CreateStringArray)(sptrs.as_ptr(), count).is_null(),
                (p.r.cJSON_CreateStringArray)(sptrs.as_ptr(), count).is_null()
            );
        }
        for count in [0i32, 1, 2] {
            same!(
                format!("row126 CreateIntArray(NULL,{count})"),
                (p.c.cJSON_CreateIntArray)(null(), count).is_null(),
                (p.r.cJSON_CreateIntArray)(null(), count).is_null()
            );
            same!(
                format!("row128 CreateFloatArray(NULL,{count})"),
                (p.c.cJSON_CreateFloatArray)(null(), count).is_null(),
                (p.r.cJSON_CreateFloatArray)(null(), count).is_null()
            );
            same!(
                format!("row130 CreateDoubleArray(NULL,{count})"),
                (p.c.cJSON_CreateDoubleArray)(null(), count).is_null(),
                (p.r.cJSON_CreateDoubleArray)(null(), count).is_null()
            );
            same!(
                format!("row132 CreateStringArray(NULL,{count})"),
                (p.c.cJSON_CreateStringArray)(null(), count).is_null(),
                (p.r.cJSON_CreateStringArray)(null(), count).is_null()
            );
        }
        // row 133: an element string is NULL
        for bad_at in 0..3usize {
            let mut ptrs: Vec<*const c_char> = vec![sp(&sbuf[0]), sp(&sbuf[1]), sp(&sbuf[0])];
            ptrs[bad_at] = null();
            same!(
                format!("row133 CreateStringArray with NULL at {bad_at}"),
                (p.c.cJSON_CreateStringArray)(ptrs.as_ptr(), 3).is_null(),
                (p.r.cJSON_CreateStringArray)(ptrs.as_ptr(), 3).is_null()
            );
        }
    }
}

/* =================================================================== */
/* Rows 135-140: Duplicate rejections                                   */
/* =================================================================== */

#[test]
fn rows135_140_duplicate_rejections() {
    let _g = lock();
    let p = pair();
    unsafe {
        for recurse in [0i32, 1, 2, -1] {
            same!(
                format!("row135 Duplicate(NULL,{recurse})"),
                (p.c.cJSON_Duplicate)(null(), recurse).is_null(),
                (p.r.cJSON_Duplicate)(null(), recurse).is_null()
            );
        }
        // rows 139/140: sibling chain at / beyond CJSON_CIRCULAR_LIMIT
        for n in [9_998usize, 9_999, 10_000, 10_001, 10_050] {
            let ca = (p.c.cJSON_CreateArray)();
            let ra = (p.r.cJSON_CreateArray)();
            for i in 0..n {
                (p.c.cJSON_AddItemToArray)(ca, (p.c.cJSON_CreateNumber)(i as f64));
                (p.r.cJSON_AddItemToArray)(ra, (p.r.cJSON_CreateNumber)(i as f64));
            }
            let cd = (p.c.cJSON_Duplicate)(ca, 1);
            let rd = (p.r.cJSON_Duplicate)(ra, 1);
            assert_eq!(cd.is_null(), rd.is_null(), "row139 Duplicate chain n={n}");
            if !cd.is_null() {
                assert_eq!(
                    (p.c.cJSON_GetArraySize)(cd),
                    (p.r.cJSON_GetArraySize)(rd),
                    "row139 dup size n={n}"
                );
                (p.c.cJSON_Delete)(cd);
                (p.r.cJSON_Delete)(rd);
            }
            (p.c.cJSON_Delete)(ca);
            (p.r.cJSON_Delete)(ra);
        }
        // String item whose valuestring is NULL
        let s = cstr("s");
        let ci = (p.c.cJSON_CreateString)(sp(&s));
        let ri = (p.r.cJSON_CreateString)(sp(&s));
        let cs = (*ci).valuestring;
        let rs = (*ri).valuestring;
        (*ci).valuestring = null_mut();
        (*ri).valuestring = null_mut();
        for recurse in [0i32, 1] {
            let cd = (p.c.cJSON_Duplicate)(ci, recurse);
            let rd = (p.r.cJSON_Duplicate)(ri, recurse);
            assert_eq!(cd.is_null(), rd.is_null(), "Duplicate string/NULL valuestring");
            if !cd.is_null() {
                assert_snap_eq(cd, rd, "dup NULL valuestring");
                (p.c.cJSON_Delete)(cd);
                (p.r.cJSON_Delete)(rd);
            }
        }
        (*ci).valuestring = cs;
        (*ri).valuestring = rs;
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);
    }
}

/* =================================================================== */
/* Rows 141-151: Minify / predicates on NULL                            */
/* =================================================================== */

#[test]
fn rows141_151_minify_and_predicates_on_null() {
    let _g = lock();
    let p = pair();
    unsafe {
        // row 141: Minify(NULL) must be a no-op, not a crash
        (p.c.cJSON_Minify)(null_mut());
        (p.r.cJSON_Minify)(null_mut());
        // rows 142-151
        let preds: [(&str, fn(&Api) -> unsafe extern "C" fn(*const cJSON) -> c_int); 10] = [
            ("row142 IsInvalid", |a| a.cJSON_IsInvalid),
            ("row143 IsFalse", |a| a.cJSON_IsFalse),
            ("row144 IsTrue", |a| a.cJSON_IsTrue),
            ("row145 IsBool", |a| a.cJSON_IsBool),
            ("row146 IsNull", |a| a.cJSON_IsNull),
            ("row147 IsNumber", |a| a.cJSON_IsNumber),
            ("row148 IsString", |a| a.cJSON_IsString),
            ("row149 IsArray", |a| a.cJSON_IsArray),
            ("row150 IsObject", |a| a.cJSON_IsObject),
            ("row151 IsRaw", |a| a.cJSON_IsRaw),
        ];
        for (name, f) in preds {
            same!(format!("{name}(NULL)"), f(&p.c)(null()), f(&p.r)(null()));
        }
    }
}

/* =================================================================== */
/* Rows 152-165: Compare rejections                                     */
/* =================================================================== */

#[test]
fn rows152_165_compare_rejections() {
    let _g = lock();
    let p = pair();
    let s = cstr("s");
    let t = cstr("t");
    unsafe {
        let cn = (p.c.cJSON_CreateNumber)(1.0);
        let rn = (p.r.cJSON_CreateNumber)(1.0);
        for cs in [0i32, 1] {
            // rows 152/153
            same!(
                format!("row152 Compare(NULL,b,{cs})"),
                (p.c.cJSON_Compare)(null(), cn, cs),
                (p.r.cJSON_Compare)(null(), rn, cs)
            );
            same!(
                format!("row153 Compare(a,NULL,{cs})"),
                (p.c.cJSON_Compare)(cn, null(), cs),
                (p.r.cJSON_Compare)(rn, null(), cs)
            );
            same!(
                format!("Compare(NULL,NULL,{cs})"),
                (p.c.cJSON_Compare)(null(), null(), cs),
                (p.r.cJSON_Compare)(null(), null(), cs)
            );
        }
        // row 154: mismatched types
        let cstrn = (p.c.cJSON_CreateString)(sp(&s));
        let rstrn = (p.r.cJSON_CreateString)(sp(&s));
        for cs in [0i32, 1] {
            same!(
                format!("row154 Compare(number,string,{cs})"),
                (p.c.cJSON_Compare)(cn, cstrn, cs),
                (p.r.cJSON_Compare)(rn, rstrn, cs)
            );
        }
        // row 155/165: invalid type bytes (out-of-range enum across FFI)
        for ty in [
            cJSON_Invalid,
            3,
            5,
            6,
            7,
            0x0f,
            0x30,
            0x60,
            0xc0,
            0xff,
            0x100,
            0x1000,
            -1,
            i32::MIN,
            i32::MAX,
        ] {
            let ca = (p.c.cJSON_CreateNumber)(1.0);
            let cb = (p.c.cJSON_CreateNumber)(1.0);
            let ra = (p.r.cJSON_CreateNumber)(1.0);
            let rb = (p.r.cJSON_CreateNumber)(1.0);
            (*ca).type_ = ty;
            (*cb).type_ = ty;
            (*ra).type_ = ty;
            (*rb).type_ = ty;
            for cs in [0i32, 1] {
                same!(
                    format!("row155 Compare same invalid type {ty:#x} cs={cs}"),
                    (p.c.cJSON_Compare)(ca, cb, cs),
                    (p.r.cJSON_Compare)(ra, rb, cs)
                );
                // identical-pointer shortcut with an invalid type
                same!(
                    format!("row155 Compare(a,a) invalid type {ty:#x} cs={cs}"),
                    (p.c.cJSON_Compare)(ca, ca, cs),
                    (p.r.cJSON_Compare)(ra, ra, cs)
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
        // row 157: String/Raw with NULL valuestring
        for kind in 0..2 {
            let (ca, cb, ra, rb) = if kind == 0 {
                (
                    (p.c.cJSON_CreateString)(sp(&s)),
                    (p.c.cJSON_CreateString)(sp(&s)),
                    (p.r.cJSON_CreateString)(sp(&s)),
                    (p.r.cJSON_CreateString)(sp(&s)),
                )
            } else {
                (
                    (p.c.cJSON_CreateRaw)(sp(&s)),
                    (p.c.cJSON_CreateRaw)(sp(&s)),
                    (p.r.cJSON_CreateRaw)(sp(&s)),
                    (p.r.cJSON_CreateRaw)(sp(&s)),
                )
            };
            let saved = [
                (*ca).valuestring,
                (*cb).valuestring,
                (*ra).valuestring,
                (*rb).valuestring,
            ];
            for (na, nb) in [(true, false), (false, true), (true, true)] {
                if na {
                    (*ca).valuestring = null_mut();
                    (*ra).valuestring = null_mut();
                }
                if nb {
                    (*cb).valuestring = null_mut();
                    (*rb).valuestring = null_mut();
                }
                for cs in [0i32, 1] {
                    same!(
                        format!("row157 Compare kind={kind} na={na} nb={nb} cs={cs}"),
                        (p.c.cJSON_Compare)(ca, cb, cs),
                        (p.r.cJSON_Compare)(ra, rb, cs)
                    );
                }
                (*ca).valuestring = saved[0];
                (*cb).valuestring = saved[1];
                (*ra).valuestring = saved[2];
                (*rb).valuestring = saved[3];
            }
            (p.c.cJSON_Delete)(ca);
            (p.c.cJSON_Delete)(cb);
            (p.r.cJSON_Delete)(ra);
            (p.r.cJSON_Delete)(rb);
        }
        // row 158: strcmp mismatch
        let cb2 = (p.c.cJSON_CreateString)(sp(&t));
        let rb2 = (p.r.cJSON_CreateString)(sp(&t));
        for cs in [0i32, 1] {
            same!(
                format!("row158 Compare(\"s\",\"t\",{cs})"),
                (p.c.cJSON_Compare)(cstrn, cb2, cs),
                (p.r.cJSON_Compare)(rstrn, rb2, cs)
            );
        }
        (p.c.cJSON_Delete)(cb2);
        (p.r.cJSON_Delete)(rb2);
        (p.c.cJSON_Delete)(cstrn);
        (p.r.cJSON_Delete)(rstrn);
        (p.c.cJSON_Delete)(cn);
        (p.r.cJSON_Delete)(rn);

        // rows 156, 159-164 through documents
        let doc_pairs = [
            ("1", "2"),                             // 156
            ("[1,2]", "[1,3]"),                     // 159
            ("[1,2]", "[1,2,3]"),                   // 160
            ("[1,2,3]", "[1,2]"),                   // 160
            ("{\"a\":1}", "{\"b\":1}"),             // 161
            ("{\"a\":1}", "{\"a\":2}"),             // 162
            ("{\"a\":1}", "{\"a\":1,\"b\":2}"),     // 163
            ("{\"a\":1,\"b\":2}", "{\"a\":1}"),     // 163
            ("{\"a\":[1]}", "{\"a\":[2]}"),         // 164
            ("{\"A\":1}", "{\"a\":1}"),             // case
        ];
        for (a, b) in doc_pairs {
            let ab = cstr(a);
            let bb = cstr(b);
            let ca = (p.c.cJSON_Parse)(sp(&ab));
            let cb = (p.c.cJSON_Parse)(sp(&bb));
            let ra = (p.r.cJSON_Parse)(sp(&ab));
            let rb = (p.r.cJSON_Parse)(sp(&bb));
            for cs in [0i32, 1] {
                same!(
                    format!("rows156/159-164 Compare({a},{b},{cs})"),
                    (p.c.cJSON_Compare)(ca, cb, cs),
                    (p.r.cJSON_Compare)(ra, rb, cs)
                );
            }
            (p.c.cJSON_Delete)(ca);
            (p.c.cJSON_Delete)(cb);
            (p.r.cJSON_Delete)(ra);
            (p.r.cJSON_Delete)(rb);
        }
    }
}

/* =================================================================== */
/* Rows 167-170: void / trivial APIs on degenerate input                */
/* =================================================================== */

#[test]
fn rows167_170_void_and_alloc_apis() {
    let _g = lock();
    let p = pair();
    unsafe {
        // row 167
        (p.c.cJSON_Delete)(null_mut());
        (p.r.cJSON_Delete)(null_mut());
        // row 168
        (p.c.cJSON_InitHooks)(null_mut());
        (p.r.cJSON_InitHooks)(null_mut());
        // still functional afterwards
        let s = cstr("[1,2]");
        let ci = (p.c.cJSON_Parse)(sp(&s));
        let ri = (p.r.cJSON_Parse)(sp(&s));
        assert_snap_eq(ci, ri, "row168 after InitHooks(NULL)");
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);
        // row 169
        (p.c.cJSON_free)(null_mut());
        (p.r.cJSON_free)(null_mut());
        // row 170
        let a = (p.c.cJSON_malloc)(0);
        let b = (p.r.cJSON_malloc)(0);
        assert_eq!(a.is_null(), b.is_null(), "row170 cJSON_malloc(0)");
        (p.c.cJSON_free)(a);
        (p.r.cJSON_free)(b);
    }
}

/* =================================================================== */
/* G1: NULL to every pointer-taking public function                     */
/* =================================================================== */

#[test]
fn g1_null_pointers_everywhere() {
    let _g = lock();
    let p = pair();
    unsafe {
        macro_rules! nullchk {
            ($name:ident, ptr, $($arg:expr),*) => {
                assert_eq!(
                    (p.c.$name)($($arg),*).is_null(),
                    (p.r.$name)($($arg),*).is_null(),
                    concat!("G1 ", stringify!($name))
                );
            };
            ($name:ident, val, $($arg:expr),*) => {
                assert_eq!(
                    (p.c.$name)($($arg),*),
                    (p.r.$name)($($arg),*),
                    concat!("G1 ", stringify!($name))
                );
            };
        }
        nullchk!(cJSON_Parse, ptr, null());
        nullchk!(cJSON_ParseWithLength, ptr, null(), 0);
        nullchk!(cJSON_ParseWithLength, ptr, null(), 10);
        nullchk!(cJSON_ParseWithOpts, ptr, null(), null_mut(), 0);
        nullchk!(cJSON_ParseWithLengthOpts, ptr, null(), 0, null_mut(), 0);
        nullchk!(cJSON_Print, ptr, null());
        nullchk!(cJSON_PrintUnformatted, ptr, null());
        nullchk!(cJSON_PrintBuffered, ptr, null(), 0, 0);
        nullchk!(cJSON_PrintPreallocated, val, null_mut(), null_mut(), 0, 0);
        nullchk!(cJSON_GetArraySize, val, null());
        nullchk!(cJSON_GetArrayItem, ptr, null(), 0);
        nullchk!(cJSON_GetObjectItem, ptr, null(), null());
        nullchk!(cJSON_GetObjectItemCaseSensitive, ptr, null(), null());
        nullchk!(cJSON_HasObjectItem, val, null(), null());
        nullchk!(cJSON_GetStringValue, ptr, null());
        nullchk!(cJSON_IsInvalid, val, null());
        nullchk!(cJSON_IsFalse, val, null());
        nullchk!(cJSON_IsTrue, val, null());
        nullchk!(cJSON_IsBool, val, null());
        nullchk!(cJSON_IsNull, val, null());
        nullchk!(cJSON_IsNumber, val, null());
        nullchk!(cJSON_IsString, val, null());
        nullchk!(cJSON_IsArray, val, null());
        nullchk!(cJSON_IsObject, val, null());
        nullchk!(cJSON_IsRaw, val, null());
        nullchk!(cJSON_CreateString, ptr, null());
        nullchk!(cJSON_CreateRaw, ptr, null());
        nullchk!(cJSON_CreateIntArray, ptr, null(), 0);
        nullchk!(cJSON_CreateFloatArray, ptr, null(), 0);
        nullchk!(cJSON_CreateDoubleArray, ptr, null(), 0);
        nullchk!(cJSON_CreateStringArray, ptr, null(), 0);
        nullchk!(cJSON_AddItemToArray, val, null_mut(), null_mut());
        nullchk!(cJSON_AddItemToObject, val, null_mut(), null(), null_mut());
        nullchk!(cJSON_AddItemToObjectCS, val, null_mut(), null(), null_mut());
        nullchk!(cJSON_AddItemReferenceToArray, val, null_mut(), null_mut());
        nullchk!(
            cJSON_AddItemReferenceToObject,
            val,
            null_mut(),
            null(),
            null_mut()
        );
        nullchk!(cJSON_AddNullToObject, ptr, null_mut(), null());
        nullchk!(cJSON_AddTrueToObject, ptr, null_mut(), null());
        nullchk!(cJSON_AddFalseToObject, ptr, null_mut(), null());
        nullchk!(cJSON_AddBoolToObject, ptr, null_mut(), null(), 0);
        nullchk!(cJSON_AddNumberToObject, ptr, null_mut(), null(), 0.0);
        nullchk!(cJSON_AddStringToObject, ptr, null_mut(), null(), null());
        nullchk!(cJSON_AddRawToObject, ptr, null_mut(), null(), null());
        nullchk!(cJSON_AddObjectToObject, ptr, null_mut(), null());
        nullchk!(cJSON_AddArrayToObject, ptr, null_mut(), null());
        nullchk!(cJSON_DetachItemViaPointer, ptr, null_mut(), null_mut());
        nullchk!(cJSON_DetachItemFromArray, ptr, null_mut(), 0);
        nullchk!(cJSON_DetachItemFromObject, ptr, null_mut(), null());
        nullchk!(
            cJSON_DetachItemFromObjectCaseSensitive,
            ptr,
            null_mut(),
            null()
        );
        nullchk!(cJSON_InsertItemInArray, val, null_mut(), 0, null_mut());
        nullchk!(
            cJSON_ReplaceItemViaPointer,
            val,
            null_mut(),
            null_mut(),
            null_mut()
        );
        nullchk!(cJSON_ReplaceItemInArray, val, null_mut(), 0, null_mut());
        nullchk!(cJSON_ReplaceItemInObject, val, null_mut(), null(), null_mut());
        nullchk!(
            cJSON_ReplaceItemInObjectCaseSensitive,
            val,
            null_mut(),
            null(),
            null_mut()
        );
        nullchk!(cJSON_Duplicate, ptr, null(), 0);
        nullchk!(cJSON_Duplicate, ptr, null(), 1);
        nullchk!(cJSON_Compare, val, null(), null(), 0);
        nullchk!(cJSON_SetValuestring, ptr, null_mut(), null());
        // void functions: must not crash
        (p.c.cJSON_Delete)(null_mut());
        (p.r.cJSON_Delete)(null_mut());
        (p.c.cJSON_DeleteItemFromArray)(null_mut(), 0);
        (p.r.cJSON_DeleteItemFromArray)(null_mut(), 0);
        (p.c.cJSON_DeleteItemFromObject)(null_mut(), null());
        (p.r.cJSON_DeleteItemFromObject)(null_mut(), null());
        (p.c.cJSON_DeleteItemFromObjectCaseSensitive)(null_mut(), null());
        (p.r.cJSON_DeleteItemFromObjectCaseSensitive)(null_mut(), null());
        (p.c.cJSON_Minify)(null_mut());
        (p.r.cJSON_Minify)(null_mut());
        (p.c.cJSON_InitHooks)(null_mut());
        (p.r.cJSON_InitHooks)(null_mut());
        (p.c.cJSON_free)(null_mut());
        (p.r.cJSON_free)(null_mut());
        // GetNumberValue returns NAN; compare bit patterns
        assert_eq!(
            (p.c.cJSON_GetNumberValue)(null()).to_bits(),
            (p.r.cJSON_GetNumberValue)(null()).to_bits(),
            "G1 cJSON_GetNumberValue(NULL)"
        );
    }
}

/* =================================================================== */
/* G5: out-of-range type enum values across the FFI boundary            */
/* =================================================================== */

#[test]
fn g5_out_of_range_type_enum() {
    let _g = lock();
    let p = pair();
    let types: [c_int; 22] = [
        0,      // cJSON_Invalid
        3,      // False|True
        5,      // False|NULL
        6,
        7,
        9,
        0x0f,
        0x11,
        0x18,
        0x30,
        0x60,
        0x80,
        0xa0,
        0xc0,
        0xff,
        0x100,  // IsReference alone
        0x200,  // StringIsConst alone
        0x300,
        0x1000,
        0x7fff_ffff,
        -1,
        i32::MIN,
    ];
    unsafe {
        for ty in types {
            // a NUMBER item (valuestring NULL) is safe to re-type and delete
            let ci = (p.c.cJSON_CreateNumber)(2.5);
            let ri = (p.r.cJSON_CreateNumber)(2.5);
            (*ci).type_ = ty;
            (*ri).type_ = ty;
            // every predicate
            let preds: [(&str, fn(&Api) -> unsafe extern "C" fn(*const cJSON) -> c_int); 10] = [
                ("IsInvalid", |a| a.cJSON_IsInvalid),
                ("IsFalse", |a| a.cJSON_IsFalse),
                ("IsTrue", |a| a.cJSON_IsTrue),
                ("IsBool", |a| a.cJSON_IsBool),
                ("IsNull", |a| a.cJSON_IsNull),
                ("IsNumber", |a| a.cJSON_IsNumber),
                ("IsString", |a| a.cJSON_IsString),
                ("IsArray", |a| a.cJSON_IsArray),
                ("IsObject", |a| a.cJSON_IsObject),
                ("IsRaw", |a| a.cJSON_IsRaw),
            ];
            for (name, f) in preds {
                assert_eq!(
                    f(&p.c)(ci),
                    f(&p.r)(ri),
                    "G5 {name} with type {ty:#x}"
                );
            }
            // printing
            assert_eq!(
                print_all(&p.c, ci),
                print_all(&p.r, ri),
                "G5 print with type {ty:#x}"
            );
            // accessors
            assert_eq!(
                read_cstr((p.c.cJSON_GetStringValue)(ci)),
                read_cstr((p.r.cJSON_GetStringValue)(ri)),
                "G5 GetStringValue type {ty:#x}"
            );
            assert_eq!(
                (p.c.cJSON_GetNumberValue)(ci).to_bits(),
                (p.r.cJSON_GetNumberValue)(ri).to_bits(),
                "G5 GetNumberValue type {ty:#x}"
            );
            assert_eq!(
                (p.c.cJSON_GetArraySize)(ci),
                (p.r.cJSON_GetArraySize)(ri),
                "G5 GetArraySize type {ty:#x}"
            );
            // duplicate
            for recurse in [0i32, 1] {
                let cd = (p.c.cJSON_Duplicate)(ci, recurse);
                let rd = (p.r.cJSON_Duplicate)(ri, recurse);
                assert_eq!(
                    cd.is_null(),
                    rd.is_null(),
                    "G5 Duplicate({recurse}) type {ty:#x}"
                );
                if !cd.is_null() {
                    assert_snap_eq(cd, rd, &format!("G5 dup type {ty:#x}"));
                    // the duplicate may carry a weird type: restore before free
                    (*cd).type_ = cJSON_Number;
                    (*rd).type_ = cJSON_Number;
                    (p.c.cJSON_Delete)(cd);
                    (p.r.cJSON_Delete)(rd);
                }
            }
            // SetValuestring
            let v = cstr("zz");
            assert_eq!(
                (p.c.cJSON_SetValuestring)(ci, sp(&v)).is_null(),
                (p.r.cJSON_SetValuestring)(ri, sp(&v)).is_null(),
                "G5 SetValuestring type {ty:#x}"
            );
            // SetNumberHelper works on any type
            assert_eq!(
                (p.c.cJSON_SetNumberHelper)(ci, 7.5).to_bits(),
                (p.r.cJSON_SetNumberHelper)(ri, 7.5).to_bits(),
                "G5 SetNumberHelper type {ty:#x}"
            );
            assert_snap_eq(ci, ri, &format!("G5 state after ops, type {ty:#x}"));
            (*ci).type_ = cJSON_Number;
            (*ri).type_ = cJSON_Number;
            (*ci).valuestring = null_mut();
            (*ri).valuestring = null_mut();
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
    }
}

/* =================================================================== */
/* G6: non-boolean cJSON_bool arguments                                 */
/* =================================================================== */

#[test]
fn g6_nonboolean_bool_args() {
    let _g = lock();
    let p = pair();
    let doc = cstr("{\"a\":[1,{\"B\":2}]}");
    let odd: [c_int; 7] = [0, 1, 2, -1, 0x100, i32::MIN, i32::MAX];
    unsafe {
        for b in odd {
            // CreateBool
            let ci = (p.c.cJSON_CreateBool)(b);
            let ri = (p.r.cJSON_CreateBool)(b);
            assert_snap_eq(ci, ri, &format!("G6 CreateBool({b})"));
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
            // AddBoolToObject
            let co = (p.c.cJSON_CreateObject)();
            let ro = (p.r.cJSON_CreateObject)();
            let k = cstr("k");
            (p.c.cJSON_AddBoolToObject)(co, sp(&k), b);
            (p.r.cJSON_AddBoolToObject)(ro, sp(&k), b);
            assert_snap_eq(co, ro, &format!("G6 AddBoolToObject({b})"));
            (p.c.cJSON_Delete)(co);
            (p.r.cJSON_Delete)(ro);
            // require_null_terminated
            let mut ce: *const c_char = null();
            let mut re: *const c_char = null();
            let ci = (p.c.cJSON_ParseWithOpts)(sp(&doc), &mut ce, b);
            let ri = (p.r.cJSON_ParseWithOpts)(sp(&doc), &mut re, b);
            assert_eq!(ci.is_null(), ri.is_null(), "G6 ParseWithOpts req={b}");
            if !ci.is_null() {
                assert_snap_eq(ci, ri, &format!("G6 parse req={b}"));
                // fmt argument of the printers
                for pre in [0i32, 32] {
                    let a = take_print(&p.c, (p.c.cJSON_PrintBuffered)(ci, pre, b));
                    let bb = take_print(&p.r, (p.r.cJSON_PrintBuffered)(ri, pre, b));
                    assert_eq!(a, bb, "G6 PrintBuffered fmt={b}");
                }
                let mut cb = vec![0u8; 512];
                let mut rb = vec![0u8; 512];
                assert_eq!(
                    (p.c.cJSON_PrintPreallocated)(ci, cb.as_mut_ptr() as *mut c_char, 512, b),
                    (p.r.cJSON_PrintPreallocated)(ri, rb.as_mut_ptr() as *mut c_char, 512, b),
                    "G6 PrintPreallocated fmt={b}"
                );
                assert_eq!(cb, rb, "G6 PrintPreallocated buffer fmt={b}");
                // recurse / case_sensitive
                let cd = (p.c.cJSON_Duplicate)(ci, b);
                let rd = (p.r.cJSON_Duplicate)(ri, b);
                assert_eq!(cd.is_null(), rd.is_null(), "G6 Duplicate recurse={b}");
                if !cd.is_null() {
                    assert_snap_eq(cd, rd, &format!("G6 Duplicate recurse={b}"));
                    assert_eq!(
                        (p.c.cJSON_Compare)(ci, cd, b),
                        (p.r.cJSON_Compare)(ri, rd, b),
                        "G6 Compare case_sensitive={b}"
                    );
                    (p.c.cJSON_Delete)(cd);
                    (p.r.cJSON_Delete)(rd);
                }
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
    }
}

/* =================================================================== */
/* G7 + alloc-failure rows: a hook allocator that fails the k-th malloc  */
/* =================================================================== */

static FAIL_AT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(-1);
static COUNTER: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

unsafe extern "C" fn failing_malloc(sz: usize) -> *mut c_void {
    use std::sync::atomic::Ordering::SeqCst;
    let n = COUNTER.fetch_add(1, SeqCst);
    let fail_at = FAIL_AT.load(SeqCst);
    if fail_at >= 0 && n == fail_at {
        return null_mut();
    }
    real_malloc(sz)
}
unsafe extern "C" fn passthrough_free(ptr: *mut c_void) {
    real_free(ptr)
}
extern "C" {
    #[link_name = "malloc"]
    fn real_malloc(sz: usize) -> *mut c_void;
    #[link_name = "free"]
    fn real_free(p: *mut c_void);
}

/// Run `op` on one library with the k-th internal allocation forced to fail,
/// returning (allocations attempted, opaque result description).
unsafe fn under_failing_alloc<R, F: FnOnce(&Api) -> R>(
    api: &Api,
    k: i64,
    op: F,
) -> (i64, R) {
    use std::sync::atomic::Ordering::SeqCst;
    let mut hooks = cJSON_Hooks {
        malloc_fn: Some(failing_malloc),
        free_fn: Some(passthrough_free),
    };
    (api.cJSON_InitHooks)(&mut hooks);
    COUNTER.store(0, SeqCst);
    FAIL_AT.store(k, SeqCst);
    let r = op(api);
    let used = COUNTER.load(SeqCst);
    FAIL_AT.store(-1, SeqCst);
    (api.cJSON_InitHooks)(null_mut());
    (used, r)
}

/// Rows 9/15/20/33/41/49/52/55/79/84/121/124/134/136-138: allocation-failure
/// paths, driven by a hook allocator that fails the k-th request.  Both
/// libraries must make the same number of allocations and fail identically.
#[test]
fn alloc_failure_paths_match() {
    let _g = lock();
    let p = pair();
    let doc = cstr("{\"a\":[1,2,\"three\",{\"b\":null}],\"c\":true}");
    let s = cstr("some string");
    unsafe {
        // First measure how many allocations each operation performs.
        #[allow(clippy::type_complexity)]
        let ops: Vec<(&str, Box<dyn Fn(&Api) -> i64>)> = vec![
            (
                "Parse",
                Box::new(|a: &Api| {
                    let it = (a.cJSON_Parse)(sp(&doc));
                    let r = if it.is_null() { 0 } else { 1 };
                    if !it.is_null() {
                        (a.cJSON_Delete)(it);
                    }
                    r
                }),
            ),
            (
                "CreateString",
                Box::new(|a: &Api| {
                    let it = (a.cJSON_CreateString)(sp(&s));
                    let r = if it.is_null() { 0 } else { 1 };
                    if !it.is_null() {
                        (a.cJSON_Delete)(it);
                    }
                    r
                }),
            ),
            (
                "CreateRaw",
                Box::new(|a: &Api| {
                    let it = (a.cJSON_CreateRaw)(sp(&s));
                    let r = if it.is_null() { 0 } else { 1 };
                    if !it.is_null() {
                        (a.cJSON_Delete)(it);
                    }
                    r
                }),
            ),
            (
                "CreateIntArray",
                Box::new(|a: &Api| {
                    let nums = [1i32, 2, 3, 4];
                    let it = (a.cJSON_CreateIntArray)(nums.as_ptr(), 4);
                    let r = if it.is_null() { 0 } else { 1 };
                    if !it.is_null() {
                        (a.cJSON_Delete)(it);
                    }
                    r
                }),
            ),
            (
                "CreateStringArray",
                Box::new(|a: &Api| {
                    let b1 = cstr("aa");
                    let b2 = cstr("bb");
                    let ptrs = [sp(&b1), sp(&b2)];
                    let it = (a.cJSON_CreateStringArray)(ptrs.as_ptr(), 2);
                    let r = if it.is_null() { 0 } else { 1 };
                    if !it.is_null() {
                        (a.cJSON_Delete)(it);
                    }
                    r
                }),
            ),
            (
                "AddItemToObject",
                Box::new(|a: &Api| {
                    let o = (a.cJSON_CreateObject)();
                    let k = cstr("key");
                    let v = (a.cJSON_CreateNumber)(1.0);
                    let ok = (a.cJSON_AddItemToObject)(o, sp(&k), v);
                    if ok == 0 && !v.is_null() {
                        (a.cJSON_Delete)(v);
                    }
                    let r = ok as i64;
                    if !o.is_null() {
                        (a.cJSON_Delete)(o);
                    }
                    r
                }),
            ),
            (
                "Duplicate",
                Box::new(|a: &Api| {
                    let it = (a.cJSON_Parse)(sp(&doc));
                    if it.is_null() {
                        return -1;
                    }
                    let d = (a.cJSON_Duplicate)(it, 1);
                    let r = if d.is_null() { 0 } else { 1 };
                    if !d.is_null() {
                        (a.cJSON_Delete)(d);
                    }
                    (a.cJSON_Delete)(it);
                    r
                }),
            ),
            (
                "SetValuestring-longer",
                Box::new(|a: &Api| {
                    let short = cstr("ab");
                    let long = cstr("abcdefghijklmnop");
                    let it = (a.cJSON_CreateString)(sp(&short));
                    if it.is_null() {
                        return -1;
                    }
                    let res = (a.cJSON_SetValuestring)(it, sp(&long));
                    let r = if res.is_null() { 0 } else { 1 };
                    (a.cJSON_Delete)(it);
                    r
                }),
            ),
            (
                "ReplaceItemInObject",
                Box::new(|a: &Api| {
                    let o = (a.cJSON_CreateObject)();
                    let k = cstr("key");
                    (a.cJSON_AddNumberToObject)(o, sp(&k), 1.0);
                    let rep = (a.cJSON_CreateNumber)(2.0);
                    let ok = (a.cJSON_ReplaceItemInObject)(o, sp(&k), rep);
                    if ok == 0 && !rep.is_null() {
                        (a.cJSON_Delete)(rep);
                    }
                    let r = ok as i64;
                    if !o.is_null() {
                        (a.cJSON_Delete)(o);
                    }
                    r
                }),
            ),
            (
                "AddItemReferenceToArray",
                Box::new(|a: &Api| {
                    let arr = (a.cJSON_CreateArray)();
                    let tgt = (a.cJSON_CreateNumber)(3.0);
                    let ok = (a.cJSON_AddItemReferenceToArray)(arr, tgt);
                    let r = ok as i64;
                    if !arr.is_null() {
                        (a.cJSON_Delete)(arr);
                    }
                    if !tgt.is_null() {
                        (a.cJSON_Delete)(tgt);
                    }
                    r
                }),
            ),
            (
                "AddItemReferenceToObject",
                Box::new(|a: &Api| {
                    let o = (a.cJSON_CreateObject)();
                    let k = cstr("rk");
                    let tgt = (a.cJSON_CreateNumber)(3.0);
                    let ok = (a.cJSON_AddItemReferenceToObject)(o, sp(&k), tgt);
                    let r = ok as i64;
                    if !o.is_null() {
                        (a.cJSON_Delete)(o);
                    }
                    if !tgt.is_null() {
                        (a.cJSON_Delete)(tgt);
                    }
                    r
                }),
            ),
            (
                "CreateFloatArray",
                Box::new(|a: &Api| {
                    let nums = [1.5f32, 2.5, 3.5];
                    let it = (a.cJSON_CreateFloatArray)(nums.as_ptr(), 3);
                    let r = if it.is_null() { 0 } else { 1 };
                    if !it.is_null() {
                        (a.cJSON_Delete)(it);
                    }
                    r
                }),
            ),
            (
                "CreateDoubleArray",
                Box::new(|a: &Api| {
                    let nums = [1.5f64, 2.5, 3.5];
                    let it = (a.cJSON_CreateDoubleArray)(nums.as_ptr(), 3);
                    let r = if it.is_null() { 0 } else { 1 };
                    if !it.is_null() {
                        (a.cJSON_Delete)(it);
                    }
                    r
                }),
            ),
            (
                "scalar-constructors",
                Box::new(|a: &Api| {
                    let mut n = 0i64;
                    for it in [
                        (a.cJSON_CreateNull)(),
                        (a.cJSON_CreateTrue)(),
                        (a.cJSON_CreateFalse)(),
                        (a.cJSON_CreateBool)(1),
                        (a.cJSON_CreateNumber)(1.0),
                        (a.cJSON_CreateArray)(),
                        (a.cJSON_CreateObject)(),
                        (a.cJSON_CreateStringReference)(sp(&cstr("x"))),
                        (a.cJSON_CreateObjectReference)(std::ptr::null()),
                        (a.cJSON_CreateArrayReference)(std::ptr::null()),
                    ] {
                        if !it.is_null() {
                            n += 1;
                            (a.cJSON_Delete)(it);
                        }
                    }
                    n
                }),
            ),
            (
                "PrintUnformatted",
                Box::new(|a: &Api| {
                    let d = cstr("{\"a\":[1,2,\"three\"]}");
                    let it = (a.cJSON_Parse)(sp(&d));
                    if it.is_null() {
                        return -1;
                    }
                    let out = (a.cJSON_PrintUnformatted)(it);
                    let r = if out.is_null() { 0 } else { 1 };
                    if !out.is_null() {
                        (a.cJSON_free)(out as *mut c_void);
                    }
                    (a.cJSON_Delete)(it);
                    r
                }),
            ),
            (
                "Print",
                Box::new(|a: &Api| {
                    let it = (a.cJSON_Parse)(sp(&doc));
                    if it.is_null() {
                        return -1;
                    }
                    let out = (a.cJSON_Print)(it);
                    let r = if out.is_null() { 0 } else { 1 };
                    if !out.is_null() {
                        (a.cJSON_free)(out as *mut c_void);
                    }
                    (a.cJSON_Delete)(it);
                    r
                }),
            ),
            (
                "PrintBuffered",
                Box::new(|a: &Api| {
                    let it = (a.cJSON_Parse)(sp(&doc));
                    if it.is_null() {
                        return -1;
                    }
                    let out = (a.cJSON_PrintBuffered)(it, 2, 1);
                    let r = if out.is_null() { 0 } else { 1 };
                    if !out.is_null() {
                        (a.cJSON_free)(out as *mut c_void);
                    }
                    (a.cJSON_Delete)(it);
                    r
                }),
            ),
        ];

        for (name, op) in &ops {
            // baseline: no failure, learn the allocation count
            let (cn, cr) = under_failing_alloc(&p.c, -1, |a| op(a));
            let (rn, rr) = under_failing_alloc(&p.r, -1, |a| op(a));
            assert_eq!(cr, rr, "{name}: baseline result");
            assert!(cn > 0, "{name}: no allocations observed — test is vacuous");
            eprintln!("alloc-failure: {name}: {cn} allocations");
            assert_eq!(
                cn, rn,
                "{name}: allocation COUNT differs (C={cn}, Rust={rn}) — the \
                 translation performs a different number of allocations"
            );
            // now fail each allocation in turn
            for k in 0..cn.min(60) {
                let (_, cres) = under_failing_alloc(&p.c, k, |a| op(a));
                let (_, rres) = under_failing_alloc(&p.r, k, |a| op(a));
                assert_eq!(cres, rres, "{name}: failing alloc #{k} gives different result");
            }
        }
    }
}
