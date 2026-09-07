//! Phase C — one differential test per ERRORS.md row.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

const NULLC: *const c_char = ptr::null();

fn nullitem() -> Item {
    ptr::null_mut()
}

/// Rows 1, 2 — cJSON_GetStringValue
#[test]
fn rows01_02_get_string_value() {
    unsafe {
        let (c, r) = both();
        assert_eq!(
            (c.cJSON_GetStringValue)(nullitem()).is_null(),
            (r.cJSON_GetStringValue)(nullitem()).is_null(),
            "row1 NULL item"
        );
        assert!((c.cJSON_GetStringValue)(nullitem()).is_null());
        for n in [
            Node::Null,
            Node::True,
            Node::Number(1.0),
            Node::Array(vec![]),
            Node::Object(vec![]),
            Node::Raw("1".into()),
            Node::Retyped(Box::new(Node::Str("s".into())), cJSON_String | 0x100),
        ] {
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let a = (c.cJSON_GetStringValue)(ci);
            let b = (r.cJSON_GetStringValue)(ri);
            assert_eq!(a.is_null(), b.is_null(), "row2 non-string");
            assert_eq!(read_cstr(a), read_cstr(b), "row2 value");
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Rows 3, 4 — cJSON_GetNumberValue returns NaN
#[test]
fn rows03_04_get_number_value() {
    unsafe {
        let (c, r) = both();
        let a = (c.cJSON_GetNumberValue)(nullitem());
        let b = (r.cJSON_GetNumberValue)(nullitem());
        assert!(a.is_nan() && b.is_nan(), "row3 NULL item must give NaN");
        for n in [
            Node::Null,
            Node::True,
            Node::Str("s".into()),
            Node::Array(vec![]),
            Node::Object(vec![]),
            Node::Raw("1".into()),
        ] {
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let a = (c.cJSON_GetNumberValue)(ci);
            let b = (r.cJSON_GetNumberValue)(ri);
            assert_eq!(a.is_nan(), b.is_nan(), "row4 NaN-ness");
            assert!(a.is_nan(), "row4 C must return NaN for non-number");
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Rows 5..9 — cJSON_SetValuestring rejections
#[test]
fn rows05_09_set_valuestring() {
    unsafe {
        let (c, r) = both();
        let s = cbytes(b"new value");
        let sp = s.as_ptr() as *const c_char;

        // row 5: NULL object
        assert_eq!(
            (c.cJSON_SetValuestring)(nullitem(), sp).is_null(),
            (r.cJSON_SetValuestring)(nullitem(), sp).is_null(),
            "row5"
        );
        assert!((c.cJSON_SetValuestring)(nullitem(), sp).is_null());

        // row 6: not a string
        for n in [
            Node::Null,
            Node::True,
            Node::Number(1.0),
            Node::Array(vec![]),
            Node::Object(vec![]),
            Node::Raw("1".into()),
        ] {
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let a = (c.cJSON_SetValuestring)(ci, sp);
            let b = (r.cJSON_SetValuestring)(ri, sp);
            assert_eq!(a.is_null(), b.is_null(), "row6 non-string");
            assert!(a.is_null(), "row6 C must reject");
            assert_eq!(
                show(&print_and_free(&c, ci)),
                show(&print_and_free(&r, ri)),
                "row6 unchanged"
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }

        // row 7: cJSON_IsReference set
        let n = Node::StrRef("literal".into());
        let ci = build(&c, &n);
        let ri = build(&r, &n);
        let a = (c.cJSON_SetValuestring)(ci, sp);
        let b = (r.cJSON_SetValuestring)(ri, sp);
        assert_eq!(a.is_null(), b.is_null(), "row7 reference");
        assert!(a.is_null(), "row7 C must reject references");
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);

        // row 8: valuestring == NULL (corrupted string item)
        let ci = (c.cJSON_CreateString)(cbytes(b"x").as_ptr() as *const c_char);
        let ri = (r.cJSON_CreateString)(cbytes(b"x").as_ptr() as *const c_char);
        let saved_c = (*ci).valuestring;
        let saved_r = (*ri).valuestring;
        (*ci).valuestring = ptr::null_mut();
        (*ri).valuestring = ptr::null_mut();
        let a = (c.cJSON_SetValuestring)(ci, sp);
        let b = (r.cJSON_SetValuestring)(ri, sp);
        assert_eq!(a.is_null(), b.is_null(), "row8");
        assert!(a.is_null(), "row8 C must reject");
        (*ci).valuestring = saved_c;
        (*ri).valuestring = saved_r;
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);

        // row 9: NULL valuestring argument
        let ci = (c.cJSON_CreateString)(cbytes(b"x").as_ptr() as *const c_char);
        let ri = (r.cJSON_CreateString)(cbytes(b"x").as_ptr() as *const c_char);
        let a = (c.cJSON_SetValuestring)(ci, NULLC);
        let b = (r.cJSON_SetValuestring)(ri, NULLC);
        assert_eq!(a.is_null(), b.is_null(), "row9");
        assert!(a.is_null(), "row9 C must reject");
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);
    }
}

/// Rows 10..14 — NULL / zero-length parse inputs
#[test]
fn rows10_14_parse_null_inputs() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        // row 10 / 14
        for (label, f) in [("row14 cJSON_Parse", 0), ("row10 cJSON_ParseWithOpts", 1)] {
            let (a, b) = if f == 0 {
                ((c.cJSON_Parse)(NULLC), (r.cJSON_Parse)(NULLC))
            } else {
                (
                    (c.cJSON_ParseWithOpts)(NULLC, ptr::null_mut(), 0),
                    (r.cJSON_ParseWithOpts)(NULLC, ptr::null_mut(), 0),
                )
            };
            assert_eq!(a.is_null(), b.is_null(), "{}", label);
            assert!(a.is_null(), "{} C must return NULL", label);
        }
        // row 11 / 12 / 13
        for len in [0usize, 1, 16, usize::MAX] {
            let a = (c.cJSON_ParseWithLength)(NULLC, len);
            let b = (r.cJSON_ParseWithLength)(NULLC, len);
            assert_eq!(a.is_null(), b.is_null(), "row11 len={}", len);
            assert!(a.is_null());
            let a = (c.cJSON_ParseWithLengthOpts)(NULLC, len, ptr::null_mut(), 1);
            let b = (r.cJSON_ParseWithLengthOpts)(NULLC, len, ptr::null_mut(), 1);
            assert_eq!(a.is_null(), b.is_null(), "row11b len={}", len);
        }
        let text = cbytes(b"{}");
        for rnt in [0i32, 1] {
            let a = (c.cJSON_ParseWithLengthOpts)(text.as_ptr() as *const c_char, 0, ptr::null_mut(), rnt);
            let b = (r.cJSON_ParseWithLengthOpts)(text.as_ptr() as *const c_char, 0, ptr::null_mut(), rnt);
            assert_eq!(a.is_null(), b.is_null(), "row12 zero length");
            assert!(a.is_null(), "row12 C must return NULL");
            let a = (c.cJSON_ParseWithLength)(text.as_ptr() as *const c_char, 0);
            let b = (r.cJSON_ParseWithLength)(text.as_ptr() as *const c_char, 0);
            assert_eq!(a.is_null(), b.is_null(), "row13 zero length");
        }
        // error pointer must agree in all of the above
        assert_eq!(
            (c.cJSON_GetErrorPtr)().is_null(),
            (r.cJSON_GetErrorPtr)().is_null(),
            "rows10_14 GetErrorPtr NULL-ness"
        );
    }
}

/// Rows 15..20 — malformed JSON of every documented flavour
#[test]
fn rows15_20_malformed_json() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let bad: Vec<&[u8]> = vec![
            // row 15 — structural
            b"{", b"[", b"tru", b"nul", b"fals", b"@", b"", b" ", b"'x'", b"{\"a\"}",
            b"[1,]", b"{,}", b"[,]", b"{\"a\":}", b"{:1}", b"]", b"}", b"[}", b"{]",
            b"[1 2]", b"{\"a\":1,}", b"{\"a\" 1}", b"truex", b"-", b"+1", b".", b"e5",
            b"[[1]", b"{\"a\":{\"b\":1}", b"\"", b"nan", b"inf", b"Infinity", b"NaN",
            b"01x", b"--1", b"1e", b"1e+", b"1.", b"[1,,2]",
            // row 16 — trailing garbage (require_null_terminated is 0 for cJSON_Parse,
            // so these are handled by row 21 in Phase B; kept here for the error ptr)
            b"{}x", b"1 2",
            // row 19 — strings
            b"\"abc", b"\"\\q\"", b"\"\\uZZZZ\"", b"\"\\ud800\"", b"\"\\u00\"",
            b"\"\\u\"", b"\"\\\"", b"\"\\ud800\\ud800\"", b"\"\\udc00\"",
            b"\"\\ud83d\\u0041\"", b"\"a\\u12\"",
            // row 20 — bad hex in \u
            b"\"\\u12g4\"", b"\"\\u-123\"", b"\"\\u 123\"",
        ];
        for t in bad {
            let buf = cbytes(t);
            let a = (c.cJSON_Parse)(buf.as_ptr() as *const c_char);
            let b = (r.cJSON_Parse)(buf.as_ptr() as *const c_char);
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "rows15_20 {:?}: NULL-ness (C null={})",
                String::from_utf8_lossy(t),
                a.is_null()
            );
            let oc = (c.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize;
            let or = (r.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize;
            assert_eq!(
                oc,
                or,
                "rows15_20 {:?}: error offset",
                String::from_utf8_lossy(t)
            );
            if !a.is_null() {
                assert_eq!(
                    show(&print_unformatted_and_free(&c, a)),
                    show(&print_unformatted_and_free(&r, b)),
                    "rows15_20 {:?}: accepted, printed",
                    String::from_utf8_lossy(t)
                );
            }
            (c.cJSON_Delete)(a);
            (r.cJSON_Delete)(b);
        }
    }
}

/// Row 17 — nesting limit
#[test]
fn row17_nesting_limit() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        for (depth, opener, closer) in [
            (1000usize, '[', ']'),
            (1001, '[', ']'),
            (1002, '[', ']'),
            (2000, '[', ']'),
            (1000, '{', '}'),
            (1001, '{', '}'),
        ] {
            let mut t = String::new();
            for _ in 0..depth {
                if opener == '{' {
                    t.push_str("{\"a\":");
                } else {
                    t.push('[');
                }
            }
            t.push('1');
            for _ in 0..depth {
                t.push(closer);
            }
            let buf = cbytes(t.as_bytes());
            let a = (c.cJSON_Parse)(buf.as_ptr() as *const c_char);
            let b = (r.cJSON_Parse)(buf.as_ptr() as *const c_char);
            assert_eq!(
                a.is_null(),
                b.is_null(),
                "row17 depth={} {}: NULL-ness (C null={})",
                depth,
                opener,
                a.is_null()
            );
            assert_eq!(
                (c.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize,
                (r.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize,
                "row17 depth={} error offset",
                depth
            );
            (c.cJSON_Delete)(a);
            (r.cJSON_Delete)(b);
        }
    }
}

/// Rows 21..24 — print failures
#[test]
fn rows21_24_print_failures() {
    unsafe {
        let (c, r) = both();
        // row 21: NULL item
        assert!((c.cJSON_Print)(ptr::null()).is_null(), "row21 C");
        assert_eq!(
            (c.cJSON_Print)(ptr::null()).is_null(),
            (r.cJSON_Print)(ptr::null()).is_null(),
            "row21"
        );
        assert_eq!(
            (c.cJSON_PrintUnformatted)(ptr::null()).is_null(),
            (r.cJSON_PrintUnformatted)(ptr::null()).is_null(),
            "row24 NULL"
        );

        // row 22: invalid type values
        for t in [0i32, 0x4000, -1, 0x10000, 3, 5, 9, 0xFF, 0x1FF] {
            let n = Node::Retyped(Box::new(Node::Number(1.0)), t);
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let a = print_and_free(&c, ci);
            let b = print_and_free(&r, ri);
            assert_eq!(show(&a), show(&b), "row22 type={:#x} Print", t);
            let a = print_unformatted_and_free(&c, ci);
            let b = print_unformatted_and_free(&r, ri);
            assert_eq!(show(&a), show(&b), "row24 type={:#x} PrintUnformatted", t);
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }

        // row 23: string / raw item with NULL valuestring
        for base_type in [cJSON_String, cJSON_Raw] {
            let ci = (c.cJSON_CreateNumber)(0.0);
            let ri = (r.cJSON_CreateNumber)(0.0);
            (*ci).type_ = base_type;
            (*ri).type_ = base_type;
            let a = print_and_free(&c, ci);
            let b = print_and_free(&r, ri);
            assert_eq!(show(&a), show(&b), "row23 type={} Print", base_type);
            // The C code distinguishes the two: print_string_ptr(NULL) emits `""`
            // for cJSON_String, while print_value returns false for cJSON_Raw.
            if base_type == cJSON_Raw {
                assert_eq!(show(&a), "<NULL>", "row23 Raw+NULL must fail in C");
            } else {
                assert_eq!(show(&a), "\"\"", "row23 String+NULL prints empty string in C");
            }
            let a = print_unformatted_and_free(&c, ci);
            let b = print_unformatted_and_free(&r, ri);
            assert_eq!(show(&a), show(&b), "row23 type={} unformatted", base_type);
            // nested inside a container, too
            let ca = (c.cJSON_CreateArray)();
            let ra = (r.cJSON_CreateArray)();
            (c.cJSON_AddItemToArray)(ca, ci);
            (r.cJSON_AddItemToArray)(ra, ri);
            assert_eq!(
                show(&print_and_free(&c, ca)),
                show(&print_and_free(&r, ra)),
                "row23 nested"
            );
            (*ci).type_ = cJSON_Number;
            (*ri).type_ = cJSON_Number;
            (c.cJSON_Delete)(ca);
            (r.cJSON_Delete)(ra);
        }
    }
}

/// Rows 25, 26 — cJSON_PrintBuffered rejections
#[test]
fn rows25_26_print_buffered() {
    unsafe {
        let (c, r) = both();
        let n = Node::Object(vec![("a".into(), Node::Number(1.0))]);
        let ci = build(&c, &n);
        let ri = build(&r, &n);
        for prebuffer in [-1i32, -2, c_int::MIN] {
            for fmt in [0i32, 1] {
                let a = print_buffered_and_free(&c, ci, prebuffer, fmt);
                let b = print_buffered_and_free(&r, ri, prebuffer, fmt);
                assert_eq!(show(&a), show(&b), "row25 prebuffer={}", prebuffer);
                assert_eq!(show(&a), "<NULL>", "row25 C must reject");
            }
        }
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);

        // row 26: NULL item / invalid type
        for prebuffer in [0i32, 1, 256] {
            let a = print_buffered_and_free(&c, ptr::null(), prebuffer, 1);
            let b = print_buffered_and_free(&r, ptr::null(), prebuffer, 1);
            assert_eq!(show(&a), show(&b), "row26 NULL item");
            assert_eq!(show(&a), "<NULL>");
            let n = Node::Retyped(Box::new(Node::Number(1.0)), 0);
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let a = print_buffered_and_free(&c, ci, prebuffer, 1);
            let b = print_buffered_and_free(&r, ri, prebuffer, 1);
            assert_eq!(show(&a), show(&b), "row26 invalid type");
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Rows 27..31 — cJSON_PrintPreallocated rejections
#[test]
fn rows27_31_print_preallocated() {
    unsafe {
        let (c, r) = both();
        let n = Node::Object(vec![
            ("a".into(), Node::Number(1.0)),
            ("b".into(), Node::Str("some longer string".into())),
        ]);
        let ci = build(&c, &n);
        let ri = build(&r, &n);
        let mut cbuf = vec![0u8; 4096];
        let mut rbuf = vec![0u8; 4096];

        // row 27: negative length
        for len in [-1i32, -100, c_int::MIN] {
            for format in [0i32, 1] {
                let a = (c.cJSON_PrintPreallocated)(ci, cbuf.as_mut_ptr() as *mut c_char, len, format);
                let b = (r.cJSON_PrintPreallocated)(ri, rbuf.as_mut_ptr() as *mut c_char, len, format);
                assert_eq!(a, b, "row27 len={}", len);
                assert_eq!(a, 0, "row27 C must return false");
            }
        }
        // row 28: NULL buffer
        for len in [0i32, 1, 4096] {
            let a = (c.cJSON_PrintPreallocated)(ci, ptr::null_mut(), len, 1);
            let b = (r.cJSON_PrintPreallocated)(ri, ptr::null_mut(), len, 1);
            assert_eq!(a, b, "row28 len={}", len);
            assert_eq!(a, 0, "row28 C must return false");
        }
        // rows 29 & 31: buffer too small / zero length
        let full = print_unformatted_and_free(&c, ci).unwrap();
        for len in 0..(full.len() + 2) {
            let mut cb = vec![0u8; full.len() + 8];
            let mut rb = vec![0u8; full.len() + 8];
            let a = (c.cJSON_PrintPreallocated)(ci, cb.as_mut_ptr() as *mut c_char, len as c_int, 0);
            let b = (r.cJSON_PrintPreallocated)(ri, rb.as_mut_ptr() as *mut c_char, len as c_int, 0);
            assert_eq!(a, b, "row29 len={} return", len);
            assert_eq!(cb, rb, "row29 len={} buffer bytes", len);
        }
        // row 30: NULL item
        for format in [0i32, 1] {
            let a = (c.cJSON_PrintPreallocated)(
                ptr::null_mut(),
                cbuf.as_mut_ptr() as *mut c_char,
                4096,
                format,
            );
            let b = (r.cJSON_PrintPreallocated)(
                ptr::null_mut(),
                rbuf.as_mut_ptr() as *mut c_char,
                4096,
                format,
            );
            assert_eq!(a, b, "row30 NULL item");
            assert_eq!(a, 0, "row30 C must return false");
        }
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);
    }
}

/// Rows 33..37 — array accessors
#[test]
fn rows33_37_array_access() {
    unsafe {
        let (c, r) = both();
        // rows 33, 34
        assert_eq!(
            (c.cJSON_GetArraySize)(ptr::null()),
            (r.cJSON_GetArraySize)(ptr::null()),
            "row33"
        );
        assert_eq!((c.cJSON_GetArraySize)(ptr::null()), 0, "row33 C == 0");
        for n in [Node::Number(1.0), Node::Str("s".into()), Node::Null] {
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            assert_eq!(
                (c.cJSON_GetArraySize)(ci),
                (r.cJSON_GetArraySize)(ri),
                "row34"
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
        // rows 35, 36, 37
        for idx in [-1i32, -2, c_int::MIN, 0, 1, 5, 6, 100, c_int::MAX] {
            assert_eq!(
                (c.cJSON_GetArrayItem)(ptr::null(), idx).is_null(),
                (r.cJSON_GetArrayItem)(ptr::null(), idx).is_null(),
                "row36 idx={}",
                idx
            );
            let n = Node::Array((0..5).map(|i| Node::Number(i as f64)).collect());
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let a = (c.cJSON_GetArrayItem)(ci, idx);
            let b = (r.cJSON_GetArrayItem)(ri, idx);
            assert_eq!(a.is_null(), b.is_null(), "row35/37 idx={}", idx);
            if !a.is_null() {
                assert_eq!(
                    show(&print_unformatted_and_free(&c, a)),
                    show(&print_unformatted_and_free(&r, b)),
                    "row37 idx={} value",
                    idx
                );
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Rows 38..42 — object accessors
#[test]
fn rows38_42_object_access() {
    unsafe {
        let (c, r) = both();
        let key = cbytes(b"a");
        let kp = key.as_ptr() as *const c_char;
        // rows 38, 39, 41, 42 with NULL arguments
        assert_eq!(
            (c.cJSON_GetObjectItem)(ptr::null(), kp).is_null(),
            (r.cJSON_GetObjectItem)(ptr::null(), kp).is_null(),
            "row38"
        );
        assert_eq!(
            (c.cJSON_GetObjectItemCaseSensitive)(ptr::null(), kp).is_null(),
            (r.cJSON_GetObjectItemCaseSensitive)(ptr::null(), kp).is_null(),
            "row41 NULL object"
        );
        assert_eq!(
            (c.cJSON_HasObjectItem)(ptr::null(), kp),
            (r.cJSON_HasObjectItem)(ptr::null(), kp),
            "row42 NULL object"
        );
        let n = Node::Object(vec![("a".into(), Node::Number(1.0))]);
        let ci = build(&c, &n);
        let ri = build(&r, &n);
        assert_eq!(
            (c.cJSON_GetObjectItem)(ci, NULLC).is_null(),
            (r.cJSON_GetObjectItem)(ri, NULLC).is_null(),
            "row39 NULL key"
        );
        assert_eq!(
            (c.cJSON_GetObjectItemCaseSensitive)(ci, NULLC).is_null(),
            (r.cJSON_GetObjectItemCaseSensitive)(ri, NULLC).is_null(),
            "row41 NULL key"
        );
        assert_eq!(
            (c.cJSON_HasObjectItem)(ci, NULLC),
            (r.cJSON_HasObjectItem)(ri, NULLC),
            "row42 NULL key"
        );
        // row 40: absent key
        for probe in ["b", "", "A", "aa"] {
            let kb = cbytes(probe.as_bytes());
            let p = kb.as_ptr() as *const c_char;
            assert_eq!(
                (c.cJSON_GetObjectItem)(ci, p).is_null(),
                (r.cJSON_GetObjectItem)(ri, p).is_null(),
                "row40 probe={:?}",
                probe
            );
            assert_eq!(
                (c.cJSON_GetObjectItemCaseSensitive)(ci, p).is_null(),
                (r.cJSON_GetObjectItemCaseSensitive)(ri, p).is_null(),
                "row41 probe={:?}",
                probe
            );
            assert_eq!(
                (c.cJSON_HasObjectItem)(ci, p),
                (r.cJSON_HasObjectItem)(ri, p),
                "row42 probe={:?}",
                probe
            );
        }
        // an object whose child has a NULL key (the `current_element->string == NULL`
        // guard in get_object_item)
        let extra = (c.cJSON_CreateNumber)(2.0);
        let extra_r = (r.cJSON_CreateNumber)(2.0);
        (c.cJSON_AddItemToArray)(ci, extra);
        (r.cJSON_AddItemToArray)(ri, extra_r);
        for probe in ["a", "b", ""] {
            let kb = cbytes(probe.as_bytes());
            let p = kb.as_ptr() as *const c_char;
            assert_eq!(
                (c.cJSON_GetObjectItem)(ci, p).is_null(),
                (r.cJSON_GetObjectItem)(ri, p).is_null(),
                "row40 NULL-keyed child, probe={:?}",
                probe
            );
            assert_eq!(
                (c.cJSON_GetObjectItemCaseSensitive)(ci, p).is_null(),
                (r.cJSON_GetObjectItemCaseSensitive)(ri, p).is_null(),
                "row41 NULL-keyed child, probe={:?}",
                probe
            );
        }
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);
    }
}

/// Rows 43..56 — Add* rejections
#[test]
fn rows43_56_add_rejections() {
    unsafe {
        let (c, r) = both();
        let key = cbytes(b"k");
        let kp = key.as_ptr() as *const c_char;

        // rows 43, 44, 45
        let ca = (c.cJSON_CreateArray)();
        let ra = (r.cJSON_CreateArray)();
        assert_eq!(
            (c.cJSON_AddItemToArray)(ca, nullitem()),
            (r.cJSON_AddItemToArray)(ra, nullitem()),
            "row43"
        );
        assert_eq!((c.cJSON_AddItemToArray)(ca, nullitem()), 0, "row43 C false");
        let ct = (c.cJSON_CreateNumber)(1.0);
        let rt = (r.cJSON_CreateNumber)(1.0);
        assert_eq!(
            (c.cJSON_AddItemToArray)(nullitem(), ct),
            (r.cJSON_AddItemToArray)(nullitem(), rt),
            "row44"
        );
        assert_eq!((c.cJSON_AddItemToArray)(nullitem(), ct), 0, "row44 C false");
        assert_eq!(
            (c.cJSON_AddItemToArray)(ca, ca),
            (r.cJSON_AddItemToArray)(ra, ra),
            "row45 self insert"
        );
        assert_eq!((c.cJSON_AddItemToArray)(ca, ca), 0, "row45 C false");
        (c.cJSON_Delete)(ct);
        (r.cJSON_Delete)(rt);

        // rows 46..50
        let co = (c.cJSON_CreateObject)();
        let ro = (r.cJSON_CreateObject)();
        for cs_variant in [false, true] {
            let add_c = |o: Item, k: *const c_char, it: Item| {
                if cs_variant {
                    (c.cJSON_AddItemToObjectCS)(o, k, it)
                } else {
                    (c.cJSON_AddItemToObject)(o, k, it)
                }
            };
            let add_r = |o: Item, k: *const c_char, it: Item| {
                if cs_variant {
                    (r.cJSON_AddItemToObjectCS)(o, k, it)
                } else {
                    (r.cJSON_AddItemToObject)(o, k, it)
                }
            };
            let ct = (c.cJSON_CreateNumber)(1.0);
            let rt = (r.cJSON_CreateNumber)(1.0);
            assert_eq!(add_c(nullitem(), kp, ct), add_r(nullitem(), kp, rt), "row46");
            assert_eq!(add_c(nullitem(), kp, ct), 0, "row46 C false");
            assert_eq!(add_c(co, NULLC, ct), add_r(ro, NULLC, rt), "row47");
            assert_eq!(add_c(co, NULLC, ct), 0, "row47 C false");
            assert_eq!(
                add_c(co, kp, nullitem()),
                add_r(ro, kp, nullitem()),
                "row48"
            );
            assert_eq!(add_c(co, kp, nullitem()), 0, "row48 C false");
            assert_eq!(add_c(co, kp, co), add_r(ro, kp, ro), "row49 self");
            assert_eq!(add_c(co, kp, co), 0, "row49 C false");
            (c.cJSON_Delete)(ct);
            (r.cJSON_Delete)(rt);
        }

        // rows 51, 52
        assert_eq!(
            (c.cJSON_AddItemReferenceToArray)(nullitem(), ca),
            (r.cJSON_AddItemReferenceToArray)(nullitem(), ra),
            "row51"
        );
        assert_eq!(
            (c.cJSON_AddItemReferenceToArray)(nullitem(), ca),
            0,
            "row51 C false"
        );
        assert_eq!(
            (c.cJSON_AddItemReferenceToArray)(ca, nullitem()),
            (r.cJSON_AddItemReferenceToArray)(ra, nullitem()),
            "row52"
        );
        assert_eq!(
            (c.cJSON_AddItemReferenceToArray)(ca, nullitem()),
            0,
            "row52 C false"
        );

        // rows 53, 54, 55
        assert_eq!(
            (c.cJSON_AddItemReferenceToObject)(nullitem(), kp, ca),
            (r.cJSON_AddItemReferenceToObject)(nullitem(), kp, ra),
            "row53"
        );
        assert_eq!(
            (c.cJSON_AddItemReferenceToObject)(co, NULLC, ca),
            (r.cJSON_AddItemReferenceToObject)(ro, NULLC, ra),
            "row54"
        );
        assert_eq!(
            (c.cJSON_AddItemReferenceToObject)(co, kp, nullitem()),
            (r.cJSON_AddItemReferenceToObject)(ro, kp, nullitem()),
            "row55"
        );
        assert_eq!(
            (c.cJSON_AddItemReferenceToObject)(co, kp, nullitem()),
            0,
            "row55 C false"
        );

        assert_eq!(
            show(&print_and_free(&c, ca)),
            show(&print_and_free(&r, ra)),
            "rows43_56 array unchanged"
        );
        assert_eq!(
            show(&print_and_free(&c, co)),
            show(&print_and_free(&r, ro)),
            "rows43_56 object unchanged"
        );
        (c.cJSON_Delete)(ca);
        (r.cJSON_Delete)(ra);
        (c.cJSON_Delete)(co);
        (r.cJSON_Delete)(ro);
    }
}

/// Rows 57..60 — Add*ToObject helpers with NULL object / name / value
#[test]
fn rows57_60_add_helpers_null() {
    unsafe {
        let (c, r) = both();
        let name = cbytes(b"n");
        let np = name.as_ptr() as *const c_char;
        let val = cbytes(b"v");
        let vp = val.as_ptr() as *const c_char;
        let co = (c.cJSON_CreateObject)();
        let ro = (r.cJSON_CreateObject)();

        macro_rules! simple {
            ($f:ident) => {{
                // row 57: NULL object
                assert_eq!(
                    (c.$f)(nullitem(), np).is_null(),
                    (r.$f)(nullitem(), np).is_null(),
                    "row57 {}",
                    stringify!($f)
                );
                assert!((c.$f)(nullitem(), np).is_null(), "row57 {} C", stringify!($f));
                // row 58: NULL name
                assert_eq!(
                    (c.$f)(co, NULLC).is_null(),
                    (r.$f)(ro, NULLC).is_null(),
                    "row58 {}",
                    stringify!($f)
                );
                assert!((c.$f)(co, NULLC).is_null(), "row58 {} C", stringify!($f));
            }};
        }
        simple!(cJSON_AddNullToObject);
        simple!(cJSON_AddTrueToObject);
        simple!(cJSON_AddFalseToObject);
        simple!(cJSON_AddObjectToObject);
        simple!(cJSON_AddArrayToObject);

        for bv in [0i32, 1, 2, -1] {
            assert_eq!(
                (c.cJSON_AddBoolToObject)(nullitem(), np, bv).is_null(),
                (r.cJSON_AddBoolToObject)(nullitem(), np, bv).is_null(),
                "row57 AddBool"
            );
            assert_eq!(
                (c.cJSON_AddBoolToObject)(co, NULLC, bv).is_null(),
                (r.cJSON_AddBoolToObject)(ro, NULLC, bv).is_null(),
                "row58 AddBool"
            );
        }
        for d in [0.0f64, f64::NAN, f64::INFINITY] {
            assert_eq!(
                (c.cJSON_AddNumberToObject)(nullitem(), np, d).is_null(),
                (r.cJSON_AddNumberToObject)(nullitem(), np, d).is_null(),
                "row57 AddNumber"
            );
            assert_eq!(
                (c.cJSON_AddNumberToObject)(co, NULLC, d).is_null(),
                (r.cJSON_AddNumberToObject)(ro, NULLC, d).is_null(),
                "row58 AddNumber"
            );
        }
        // rows 57/58/59/60 for the string-valued helpers
        for (label, use_raw) in [("AddStringToObject", false), ("AddRawToObject", true)] {
            let f_c = |o: Item, n: *const c_char, v: *const c_char| {
                if use_raw {
                    (c.cJSON_AddRawToObject)(o, n, v)
                } else {
                    (c.cJSON_AddStringToObject)(o, n, v)
                }
            };
            let f_r = |o: Item, n: *const c_char, v: *const c_char| {
                if use_raw {
                    (r.cJSON_AddRawToObject)(o, n, v)
                } else {
                    (r.cJSON_AddStringToObject)(o, n, v)
                }
            };
            assert_eq!(
                f_c(nullitem(), np, vp).is_null(),
                f_r(nullitem(), np, vp).is_null(),
                "row57 {}",
                label
            );
            assert_eq!(
                f_c(co, NULLC, vp).is_null(),
                f_r(ro, NULLC, vp).is_null(),
                "row58 {}",
                label
            );
            // rows 59, 60: NULL value string
            assert_eq!(
                f_c(co, np, NULLC).is_null(),
                f_r(ro, np, NULLC).is_null(),
                "rows59_60 {}",
                label
            );
            assert!(f_c(co, np, NULLC).is_null(), "rows59_60 {} C", label);
        }
        assert_eq!(
            show(&print_and_free(&c, co)),
            show(&print_and_free(&r, ro)),
            "rows57_60 object state"
        );
        (c.cJSON_Delete)(co);
        (r.cJSON_Delete)(ro);
    }
}

/// Rows 61..69 — Detach / Delete rejections
#[test]
fn rows61_69_detach_delete() {
    unsafe {
        let (c, r) = both();
        let key = cbytes(b"a");
        let kp = key.as_ptr() as *const c_char;

        // rows 61, 62
        let n = Node::Array(vec![Node::Number(1.0), Node::Number(2.0)]);
        let ci = build(&c, &n);
        let ri = build(&r, &n);
        let c0 = (c.cJSON_GetArrayItem)(ci, 0);
        let r0 = (r.cJSON_GetArrayItem)(ri, 0);
        assert_eq!(
            (c.cJSON_DetachItemViaPointer)(nullitem(), c0).is_null(),
            (r.cJSON_DetachItemViaPointer)(nullitem(), r0).is_null(),
            "row61"
        );
        assert!(
            (c.cJSON_DetachItemViaPointer)(nullitem(), c0).is_null(),
            "row61 C"
        );
        assert_eq!(
            (c.cJSON_DetachItemViaPointer)(ci, nullitem()).is_null(),
            (r.cJSON_DetachItemViaPointer)(ri, nullitem()).is_null(),
            "row62"
        );
        // row 63: item not in parent
        let orphan_c = (c.cJSON_CreateNumber)(9.0);
        let orphan_r = (r.cJSON_CreateNumber)(9.0);
        assert_eq!(
            (c.cJSON_DetachItemViaPointer)(ci, orphan_c).is_null(),
            (r.cJSON_DetachItemViaPointer)(ri, orphan_r).is_null(),
            "row63"
        );
        assert!(
            (c.cJSON_DetachItemViaPointer)(ci, orphan_c).is_null(),
            "row63 C"
        );
        (c.cJSON_Delete)(orphan_c);
        (r.cJSON_Delete)(orphan_r);
        assert_eq!(
            show(&print_and_free(&c, ci)),
            show(&print_and_free(&r, ri)),
            "rows61_63 unchanged"
        );

        // rows 64, 65
        for which in [-1i32, -5, c_int::MIN, 2, 3, c_int::MAX] {
            let a = (c.cJSON_DetachItemFromArray)(ci, which);
            let b = (r.cJSON_DetachItemFromArray)(ri, which);
            assert_eq!(a.is_null(), b.is_null(), "row64/65 which={}", which);
            assert!(a.is_null(), "row64/65 C which={}", which);
            let a = (c.cJSON_DetachItemFromArray)(nullitem(), which);
            let b = (r.cJSON_DetachItemFromArray)(nullitem(), which);
            assert_eq!(a.is_null(), b.is_null(), "row65 NULL array which={}", which);
        }
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);

        // rows 66, 67
        let n = Node::Object(vec![("a".into(), Node::Number(1.0))]);
        let ci = build(&c, &n);
        let ri = build(&r, &n);
        assert_eq!(
            (c.cJSON_DetachItemFromObject)(nullitem(), kp).is_null(),
            (r.cJSON_DetachItemFromObject)(nullitem(), kp).is_null(),
            "row66 NULL object"
        );
        assert_eq!(
            (c.cJSON_DetachItemFromObject)(ci, NULLC).is_null(),
            (r.cJSON_DetachItemFromObject)(ri, NULLC).is_null(),
            "row66 NULL key"
        );
        assert_eq!(
            (c.cJSON_DetachItemFromObjectCaseSensitive)(nullitem(), kp).is_null(),
            (r.cJSON_DetachItemFromObjectCaseSensitive)(nullitem(), kp).is_null(),
            "row67 NULL object"
        );
        assert_eq!(
            (c.cJSON_DetachItemFromObjectCaseSensitive)(ci, NULLC).is_null(),
            (r.cJSON_DetachItemFromObjectCaseSensitive)(ri, NULLC).is_null(),
            "row67 NULL key"
        );
        for probe in ["b", "", "A"] {
            let kb = cbytes(probe.as_bytes());
            let p = kb.as_ptr() as *const c_char;
            assert_eq!(
                (c.cJSON_DetachItemFromObject)(ci, p).is_null(),
                (r.cJSON_DetachItemFromObject)(ri, p).is_null(),
                "row66 absent {:?}",
                probe
            );
            assert_eq!(
                (c.cJSON_DetachItemFromObjectCaseSensitive)(ci, p).is_null(),
                (r.cJSON_DetachItemFromObjectCaseSensitive)(ri, p).is_null(),
                "row67 absent {:?}",
                probe
            );
        }

        // rows 68, 69: the void-returning Delete* variants must not crash
        for which in [-1i32, 0, 1, 99, c_int::MIN, c_int::MAX] {
            (c.cJSON_DeleteItemFromArray)(nullitem(), which);
            (r.cJSON_DeleteItemFromArray)(nullitem(), which);
        }
        (c.cJSON_DeleteItemFromObject)(nullitem(), kp);
        (r.cJSON_DeleteItemFromObject)(nullitem(), kp);
        (c.cJSON_DeleteItemFromObject)(ci, NULLC);
        (r.cJSON_DeleteItemFromObject)(ri, NULLC);
        (c.cJSON_DeleteItemFromObjectCaseSensitive)(nullitem(), kp);
        (r.cJSON_DeleteItemFromObjectCaseSensitive)(nullitem(), kp);
        (c.cJSON_DeleteItemFromObjectCaseSensitive)(ci, NULLC);
        (r.cJSON_DeleteItemFromObjectCaseSensitive)(ri, NULLC);
        for probe in ["b", ""] {
            let kb = cbytes(probe.as_bytes());
            let p = kb.as_ptr() as *const c_char;
            (c.cJSON_DeleteItemFromObject)(ci, p);
            (r.cJSON_DeleteItemFromObject)(ri, p);
            (c.cJSON_DeleteItemFromObjectCaseSensitive)(ci, p);
            (r.cJSON_DeleteItemFromObjectCaseSensitive)(ri, p);
        }
        assert_eq!(
            show(&print_and_free(&c, ci)),
            show(&print_and_free(&r, ri)),
            "rows68_69 object state"
        );
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);

        // cJSON_Delete(NULL) is a no-op in both
        (c.cJSON_Delete)(nullitem());
        (r.cJSON_Delete)(nullitem());
    }
}

/// Rows 70..73 — cJSON_InsertItemInArray rejections
#[test]
fn rows70_73_insert() {
    unsafe {
        let (c, r) = both();
        let n = Node::Array(vec![Node::Number(1.0), Node::Number(2.0)]);
        // row 70: negative index
        for which in [-1i32, -9, c_int::MIN] {
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let ct = (c.cJSON_CreateNumber)(7.0);
            let rt = (r.cJSON_CreateNumber)(7.0);
            let a = (c.cJSON_InsertItemInArray)(ci, which, ct);
            let b = (r.cJSON_InsertItemInArray)(ri, which, rt);
            assert_eq!(a, b, "row70 which={}", which);
            assert_eq!(a, 0, "row70 C false");
            (c.cJSON_Delete)(ct);
            (r.cJSON_Delete)(rt);
            assert_eq!(
                show(&print_and_free(&c, ci)),
                show(&print_and_free(&r, ri)),
                "row70 unchanged"
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
        // row 71: NULL newitem
        let ci = build(&c, &n);
        let ri = build(&r, &n);
        for which in [0i32, 1, 5] {
            assert_eq!(
                (c.cJSON_InsertItemInArray)(ci, which, nullitem()),
                (r.cJSON_InsertItemInArray)(ri, which, nullitem()),
                "row71 which={}",
                which
            );
            assert_eq!(
                (c.cJSON_InsertItemInArray)(ci, which, nullitem()),
                0,
                "row71 C false"
            );
        }
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);
        // rows 72, 73: NULL array
        for which in [0i32, 1, 5, c_int::MAX] {
            let ct = (c.cJSON_CreateNumber)(7.0);
            let rt = (r.cJSON_CreateNumber)(7.0);
            let a = (c.cJSON_InsertItemInArray)(nullitem(), which, ct);
            let b = (r.cJSON_InsertItemInArray)(nullitem(), which, rt);
            assert_eq!(a, b, "row72 which={}", which);
            assert_eq!(a, 0, "row72 C false");
            (c.cJSON_Delete)(ct);
            (r.cJSON_Delete)(rt);
        }
        // row 73: index past the end appends
        for size in [0usize, 1, 3] {
            for which in [size as c_int, size as c_int + 1, c_int::MAX] {
                let node = Node::Array((0..size).map(|i| Node::Number(i as f64)).collect());
                let ci = build(&c, &node);
                let ri = build(&r, &node);
                let ct = (c.cJSON_CreateNumber)(7.0);
                let rt = (r.cJSON_CreateNumber)(7.0);
                let a = (c.cJSON_InsertItemInArray)(ci, which, ct);
                let b = (r.cJSON_InsertItemInArray)(ri, which, rt);
                assert_eq!(a, b, "row73 size={} which={}", size, which);
                assert_eq!(
                    show(&print_and_free(&c, ci)),
                    show(&print_and_free(&r, ri)),
                    "row73 size={} which={} state",
                    size,
                    which
                );
                assert_eq!(link_shape(ci), link_shape(ri), "row73 links");
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);
            }
        }
    }
}

/// Rows 74..84 — Replace rejections
#[test]
fn rows74_84_replace() {
    unsafe {
        let (c, r) = both();
        let key = cbytes(b"a");
        let kp = key.as_ptr() as *const c_char;
        let n = Node::Array(vec![Node::Number(1.0), Node::Number(2.0)]);

        // row 74: NULL parent
        {
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let c0 = (c.cJSON_GetArrayItem)(ci, 0);
            let r0 = (r.cJSON_GetArrayItem)(ri, 0);
            let ct = (c.cJSON_CreateNumber)(7.0);
            let rt = (r.cJSON_CreateNumber)(7.0);
            let a = (c.cJSON_ReplaceItemViaPointer)(nullitem(), c0, ct);
            let b = (r.cJSON_ReplaceItemViaPointer)(nullitem(), r0, rt);
            assert_eq!(a, b, "row74");
            assert_eq!(a, 0, "row74 C false");
            (c.cJSON_Delete)(ct);
            (r.cJSON_Delete)(rt);
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
        // row 75: empty parent
        {
            let ci = (c.cJSON_CreateArray)();
            let ri = (r.cJSON_CreateArray)();
            let ct = (c.cJSON_CreateNumber)(7.0);
            let rt = (r.cJSON_CreateNumber)(7.0);
            let co = (c.cJSON_CreateNumber)(8.0);
            let ro = (r.cJSON_CreateNumber)(8.0);
            let a = (c.cJSON_ReplaceItemViaPointer)(ci, co, ct);
            let b = (r.cJSON_ReplaceItemViaPointer)(ri, ro, rt);
            assert_eq!(a, b, "row75");
            assert_eq!(a, 0, "row75 C false");
            (c.cJSON_Delete)(ct);
            (r.cJSON_Delete)(rt);
            (c.cJSON_Delete)(co);
            (r.cJSON_Delete)(ro);
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
        // rows 76, 77, 78
        {
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let c0 = (c.cJSON_GetArrayItem)(ci, 0);
            let r0 = (r.cJSON_GetArrayItem)(ri, 0);
            assert_eq!(
                (c.cJSON_ReplaceItemViaPointer)(ci, c0, nullitem()),
                (r.cJSON_ReplaceItemViaPointer)(ri, r0, nullitem()),
                "row76"
            );
            assert_eq!(
                (c.cJSON_ReplaceItemViaPointer)(ci, c0, nullitem()),
                0,
                "row76 C false"
            );
            let ct = (c.cJSON_CreateNumber)(7.0);
            let rt = (r.cJSON_CreateNumber)(7.0);
            assert_eq!(
                (c.cJSON_ReplaceItemViaPointer)(ci, nullitem(), ct),
                (r.cJSON_ReplaceItemViaPointer)(ri, nullitem(), rt),
                "row77"
            );
            assert_eq!(
                (c.cJSON_ReplaceItemViaPointer)(ci, nullitem(), ct),
                0,
                "row77 C false"
            );
            (c.cJSON_Delete)(ct);
            (r.cJSON_Delete)(rt);
            // row 78: replacement == item -> true, unchanged
            let a = (c.cJSON_ReplaceItemViaPointer)(ci, c0, c0);
            let b = (r.cJSON_ReplaceItemViaPointer)(ri, r0, r0);
            assert_eq!(a, b, "row78");
            assert_eq!(a, 1, "row78 C true");
            assert_eq!(
                show(&print_and_free(&c, ci)),
                show(&print_and_free(&r, ri)),
                "row78 unchanged"
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
        // rows 79, 80
        for which in [-1i32, c_int::MIN, 2, 3, c_int::MAX] {
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let ct = (c.cJSON_CreateNumber)(7.0);
            let rt = (r.cJSON_CreateNumber)(7.0);
            let a = (c.cJSON_ReplaceItemInArray)(ci, which, ct);
            let b = (r.cJSON_ReplaceItemInArray)(ri, which, rt);
            assert_eq!(a, b, "rows79_80 which={}", which);
            assert_eq!(a, 0, "rows79_80 C false");
            (c.cJSON_Delete)(ct);
            (r.cJSON_Delete)(rt);
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
        // rows 81..84
        let obj = Node::Object(vec![("a".into(), Node::Number(1.0))]);
        for case_sensitive in [false, true] {
            let repl_c = |o: Item, k: *const c_char, it: Item| {
                if case_sensitive {
                    (c.cJSON_ReplaceItemInObjectCaseSensitive)(o, k, it)
                } else {
                    (c.cJSON_ReplaceItemInObject)(o, k, it)
                }
            };
            let repl_r = |o: Item, k: *const c_char, it: Item| {
                if case_sensitive {
                    (r.cJSON_ReplaceItemInObjectCaseSensitive)(o, k, it)
                } else {
                    (r.cJSON_ReplaceItemInObject)(o, k, it)
                }
            };
            let ci = build(&c, &obj);
            let ri = build(&r, &obj);
            // row 81: NULL newitem
            assert_eq!(
                repl_c(ci, kp, nullitem()),
                repl_r(ri, kp, nullitem()),
                "row81 cs={}",
                case_sensitive
            );
            assert_eq!(repl_c(ci, kp, nullitem()), 0, "row81 C false");
            // row 82: NULL key
            let ct = (c.cJSON_CreateNumber)(7.0);
            let rt = (r.cJSON_CreateNumber)(7.0);
            assert_eq!(
                repl_c(ci, NULLC, ct),
                repl_r(ri, NULLC, rt),
                "row82 cs={}",
                case_sensitive
            );
            assert_eq!(repl_c(ci, NULLC, ct), 0, "row82 C false");
            // row 83: absent key (the replacement's own key IS rewritten)
            let missing = cbytes(b"zzz");
            let mp = missing.as_ptr() as *const c_char;
            let a = repl_c(ci, mp, ct);
            let b = repl_r(ri, mp, rt);
            assert_eq!(a, b, "row83 cs={}", case_sensitive);
            assert_eq!(a, 0, "row83 C false");
            assert_eq!(
                read_cstr((*ct).string),
                read_cstr((*rt).string),
                "row83 replacement key rewritten"
            );
            assert_eq!((*ct).type_, (*rt).type_, "row83 replacement type");
            // row 84: NULL object
            let a = repl_c(nullitem(), kp, ct);
            let b = repl_r(nullitem(), kp, rt);
            assert_eq!(a, b, "row84 cs={}", case_sensitive);
            assert_eq!(a, 0, "row84 C false");
            assert_eq!(
                show(&print_and_free(&c, ci)),
                show(&print_and_free(&r, ri)),
                "rows81_84 object unchanged"
            );
            (c.cJSON_Delete)(ct);
            (r.cJSON_Delete)(rt);
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Rows 85..96 — Create* rejections
#[test]
fn rows85_96_create_rejections() {
    unsafe {
        let (c, r) = both();
        // rows 85, 86
        assert_eq!(
            (c.cJSON_CreateString)(NULLC).is_null(),
            (r.cJSON_CreateString)(NULLC).is_null(),
            "row85"
        );
        assert!((c.cJSON_CreateString)(NULLC).is_null(), "row85 C");
        assert_eq!(
            (c.cJSON_CreateRaw)(NULLC).is_null(),
            (r.cJSON_CreateRaw)(NULLC).is_null(),
            "row86"
        );
        assert!((c.cJSON_CreateRaw)(NULLC).is_null(), "row86 C");

        // row 87: CreateStringReference(NULL) yields a non-NULL item
        let a = (c.cJSON_CreateStringReference)(NULLC);
        let b = (r.cJSON_CreateStringReference)(NULLC);
        assert_eq!(a.is_null(), b.is_null(), "row87 NULL-ness");
        assert!(!a.is_null(), "row87 C returns an item");
        assert_eq!((*a).type_, (*b).type_, "row87 type");
        assert_eq!(
            (*a).valuestring.is_null(),
            (*b).valuestring.is_null(),
            "row87 valuestring"
        );
        assert_eq!(
            show(&print_and_free(&c, a)),
            show(&print_and_free(&r, b)),
            "row87 print"
        );
        (c.cJSON_Delete)(a);
        (r.cJSON_Delete)(b);

        // rows 88, 89
        for is_object in [true, false] {
            let a = if is_object {
                (c.cJSON_CreateObjectReference)(ptr::null())
            } else {
                (c.cJSON_CreateArrayReference)(ptr::null())
            };
            let b = if is_object {
                (r.cJSON_CreateObjectReference)(ptr::null())
            } else {
                (r.cJSON_CreateArrayReference)(ptr::null())
            };
            assert_eq!(a.is_null(), b.is_null(), "rows88_89 NULL-ness");
            assert!(!a.is_null(), "rows88_89 C returns an item");
            assert_eq!((*a).type_, (*b).type_, "rows88_89 type");
            assert_eq!((*a).child.is_null(), (*b).child.is_null(), "rows88_89 child");
            assert_eq!(
                show(&print_and_free(&c, a)),
                show(&print_and_free(&r, b)),
                "rows88_89 print"
            );
            (c.cJSON_Delete)(a);
            (r.cJSON_Delete)(b);
        }

        // rows 90..95: negative count / NULL data pointer
        let ints = [1i32, 2, 3];
        let floats = [1.0f32, 2.0, 3.0];
        let doubles = [1.0f64, 2.0, 3.0];
        let s0 = cbytes(b"a");
        let s1 = cbytes(b"b");
        let strs: [*const c_char; 2] = [
            s0.as_ptr() as *const c_char,
            s1.as_ptr() as *const c_char,
        ];
        for count in [-1i32, -100, c_int::MIN] {
            assert_eq!(
                (c.cJSON_CreateIntArray)(ints.as_ptr(), count).is_null(),
                (r.cJSON_CreateIntArray)(ints.as_ptr(), count).is_null(),
                "row90 count={}",
                count
            );
            assert!(
                (c.cJSON_CreateIntArray)(ints.as_ptr(), count).is_null(),
                "row90 C"
            );
            assert_eq!(
                (c.cJSON_CreateFloatArray)(floats.as_ptr(), count).is_null(),
                (r.cJSON_CreateFloatArray)(floats.as_ptr(), count).is_null(),
                "row93 count={}",
                count
            );
            assert_eq!(
                (c.cJSON_CreateDoubleArray)(doubles.as_ptr(), count).is_null(),
                (r.cJSON_CreateDoubleArray)(doubles.as_ptr(), count).is_null(),
                "row94 count={}",
                count
            );
            assert_eq!(
                (c.cJSON_CreateStringArray)(strs.as_ptr(), count).is_null(),
                (r.cJSON_CreateStringArray)(strs.as_ptr(), count).is_null(),
                "row95 count={}",
                count
            );
        }
        for count in [0i32, 1, 3] {
            assert_eq!(
                (c.cJSON_CreateIntArray)(ptr::null(), count).is_null(),
                (r.cJSON_CreateIntArray)(ptr::null(), count).is_null(),
                "row91 count={}",
                count
            );
            assert!(
                (c.cJSON_CreateIntArray)(ptr::null(), count).is_null(),
                "row91 C"
            );
            assert_eq!(
                (c.cJSON_CreateFloatArray)(ptr::null(), count).is_null(),
                (r.cJSON_CreateFloatArray)(ptr::null(), count).is_null(),
                "row93b count={}",
                count
            );
            assert_eq!(
                (c.cJSON_CreateDoubleArray)(ptr::null(), count).is_null(),
                (r.cJSON_CreateDoubleArray)(ptr::null(), count).is_null(),
                "row94b count={}",
                count
            );
            assert_eq!(
                (c.cJSON_CreateStringArray)(ptr::null(), count).is_null(),
                (r.cJSON_CreateStringArray)(ptr::null(), count).is_null(),
                "row95b count={}",
                count
            );
        }
        // row 92: count == 0 is valid and yields an empty array
        let a = (c.cJSON_CreateIntArray)(ints.as_ptr(), 0);
        let b = (r.cJSON_CreateIntArray)(ints.as_ptr(), 0);
        assert!(!a.is_null() && !b.is_null(), "row92 non-NULL");
        assert_eq!((*a).child.is_null(), (*b).child.is_null(), "row92 empty");
        assert_eq!(
            show(&print_and_free(&c, a)),
            show(&print_and_free(&r, b)),
            "row92 print"
        );
        (c.cJSON_Delete)(a);
        (r.cJSON_Delete)(b);

        // row 96: NULL element inside the strings array
        for pos in 0..3usize {
            let mut arr: [*const c_char; 3] = [
                s0.as_ptr() as *const c_char,
                s1.as_ptr() as *const c_char,
                s0.as_ptr() as *const c_char,
            ];
            arr[pos] = ptr::null();
            let a = (c.cJSON_CreateStringArray)(arr.as_ptr(), 3);
            let b = (r.cJSON_CreateStringArray)(arr.as_ptr(), 3);
            assert_eq!(a.is_null(), b.is_null(), "row96 pos={}", pos);
            assert!(a.is_null(), "row96 C must reject");
            (c.cJSON_Delete)(a);
            (r.cJSON_Delete)(b);
        }
    }
}

/// Rows 97, 99 — cJSON_Duplicate rejections (row 98 has its own test)
#[test]
fn rows97_99_duplicate() {
    unsafe {
        let (c, r) = both();
        for recurse in [0i32, 1, 2, -1] {
            assert_eq!(
                (c.cJSON_Duplicate)(ptr::null(), recurse).is_null(),
                (r.cJSON_Duplicate)(ptr::null(), recurse).is_null(),
                "row97 recurse={}",
                recurse
            );
            assert!(
                (c.cJSON_Duplicate)(ptr::null(), recurse).is_null(),
                "row97 C"
            );
        }
        // row 99: string item whose valuestring is NULL
        for t in [cJSON_String, cJSON_Raw] {
            let ci = (c.cJSON_CreateNumber)(0.0);
            let ri = (r.cJSON_CreateNumber)(0.0);
            (*ci).type_ = t;
            (*ri).type_ = t;
            for recurse in [0i32, 1] {
                let a = (c.cJSON_Duplicate)(ci, recurse);
                let b = (r.cJSON_Duplicate)(ri, recurse);
                assert_eq!(a.is_null(), b.is_null(), "row99 t={} NULL-ness", t);
                if !a.is_null() {
                    assert_eq!((*a).type_, (*b).type_, "row99 type");
                    assert_eq!(
                        (*a).valuestring.is_null(),
                        (*b).valuestring.is_null(),
                        "row99 valuestring"
                    );
                    (c.cJSON_Delete)(a);
                    (r.cJSON_Delete)(b);
                }
            }
            (*ci).type_ = cJSON_Number;
            (*ri).type_ = cJSON_Number;
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Row 98 — cJSON_Duplicate depth limit (`CJSON_CIRCULAR_LIMIT`).
/// Runs on a thread with a large stack because the C implementation recurses.
#[test]
fn row98_duplicate_depth_limit() {
    std::thread::Builder::new()
        .stack_size(256 * 1024 * 1024)
        .spawn(|| unsafe {
            let (c, r) = both();
            for depth in [10usize, 9999, 10000, 10001, 10050] {
                // build `depth` nested arrays without recursion
                let croot = (c.cJSON_CreateArray)();
                let rroot = (r.cJSON_CreateArray)();
                let mut ccur = croot;
                let mut rcur = rroot;
                for _ in 1..depth {
                    let cn = (c.cJSON_CreateArray)();
                    let rn = (r.cJSON_CreateArray)();
                    (c.cJSON_AddItemToArray)(ccur, cn);
                    (r.cJSON_AddItemToArray)(rcur, rn);
                    ccur = cn;
                    rcur = rn;
                }
                for recurse in [0i32, 1] {
                    let a = (c.cJSON_Duplicate)(croot, recurse);
                    let b = (r.cJSON_Duplicate)(rroot, recurse);
                    assert_eq!(
                        a.is_null(),
                        b.is_null(),
                        "row98 depth={} recurse={} NULL-ness (C null={})",
                        depth,
                        recurse,
                        a.is_null()
                    );
                    (c.cJSON_Delete)(a);
                    (r.cJSON_Delete)(b);
                }
                (c.cJSON_Delete)(croot);
                (r.cJSON_Delete)(rroot);
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

/// Rows 100..106 — cJSON_Compare rejections
#[test]
fn rows100_106_compare() {
    unsafe {
        let (c, r) = both();
        let n = Node::Number(1.0);
        let ci = build(&c, &n);
        let ri = build(&r, &n);
        for cs_flag in [0i32, 1, 2, -1] {
            // rows 100, 101
            assert_eq!(
                (c.cJSON_Compare)(ptr::null(), ci, cs_flag),
                (r.cJSON_Compare)(ptr::null(), ri, cs_flag),
                "row100 cs={}",
                cs_flag
            );
            assert_eq!(
                (c.cJSON_Compare)(ptr::null(), ci, cs_flag),
                0,
                "row100 C false"
            );
            assert_eq!(
                (c.cJSON_Compare)(ci, ptr::null(), cs_flag),
                (r.cJSON_Compare)(ri, ptr::null(), cs_flag),
                "row101 cs={}",
                cs_flag
            );
            assert_eq!(
                (c.cJSON_Compare)(ptr::null(), ptr::null(), cs_flag),
                (r.cJSON_Compare)(ptr::null(), ptr::null(), cs_flag),
                "row100/101 both NULL"
            );
        }
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);

        // row 102: differing types
        let types = [
            Node::Null,
            Node::True,
            Node::False,
            Node::Number(1.0),
            Node::Str("s".into()),
            Node::Raw("1".into()),
            Node::Array(vec![]),
            Node::Object(vec![]),
        ];
        for a in &types {
            for b in &types {
                let ca = build(&c, a);
                let cb = build(&c, b);
                let ra = build(&r, a);
                let rb = build(&r, b);
                for cs_flag in [0i32, 1] {
                    assert_eq!(
                        (c.cJSON_Compare)(ca, cb, cs_flag),
                        (r.cJSON_Compare)(ra, rb, cs_flag),
                        "row102"
                    );
                }
                (c.cJSON_Delete)(ca);
                (c.cJSON_Delete)(cb);
                (r.cJSON_Delete)(ra);
                (r.cJSON_Delete)(rb);
            }
        }

        // rows 103, 105: invalid type values, incl. identity shortcut
        for t in [0i32, 3, 5, 6, 9, 0x0A, 0xFF, 0x1FF, -1, 0x10000] {
            let n = Node::Retyped(Box::new(Node::Number(1.0)), t);
            let ca = build(&c, &n);
            let cb = build(&c, &n);
            let ra = build(&r, &n);
            let rb = build(&r, &n);
            for cs_flag in [0i32, 1] {
                assert_eq!(
                    (c.cJSON_Compare)(ca, cb, cs_flag),
                    (r.cJSON_Compare)(ra, rb, cs_flag),
                    "row103 type={:#x}",
                    t
                );
                assert_eq!(
                    (c.cJSON_Compare)(ca, ca, cs_flag),
                    (r.cJSON_Compare)(ra, ra, cs_flag),
                    "row105 identity type={:#x}",
                    t
                );
            }
            (c.cJSON_Delete)(ca);
            (c.cJSON_Delete)(cb);
            (r.cJSON_Delete)(ra);
            (r.cJSON_Delete)(rb);
        }

        // row 104: string/raw item with NULL valuestring on either side
        for t in [cJSON_String, cJSON_Raw] {
            let ca = (c.cJSON_CreateNumber)(0.0);
            let cb = (c.cJSON_CreateString)(cbytes(b"x").as_ptr() as *const c_char);
            let ra = (r.cJSON_CreateNumber)(0.0);
            let rb = (r.cJSON_CreateString)(cbytes(b"x").as_ptr() as *const c_char);
            (*ca).type_ = t;
            (*ra).type_ = t;
            (*cb).type_ = t;
            (*rb).type_ = t;
            for cs_flag in [0i32, 1] {
                assert_eq!(
                    (c.cJSON_Compare)(ca, cb, cs_flag),
                    (r.cJSON_Compare)(ra, rb, cs_flag),
                    "row104 a-null t={}",
                    t
                );
                assert_eq!(
                    (c.cJSON_Compare)(cb, ca, cs_flag),
                    (r.cJSON_Compare)(rb, ra, cs_flag),
                    "row104 b-null t={}",
                    t
                );
                assert_eq!(
                    (c.cJSON_Compare)(ca, ca, cs_flag),
                    (r.cJSON_Compare)(ra, ra, cs_flag),
                    "row104 identity t={}",
                    t
                );
            }
            (*ca).type_ = cJSON_Number;
            (*ra).type_ = cJSON_Number;
            (*cb).type_ = cJSON_String;
            (*rb).type_ = cJSON_String;
            (c.cJSON_Delete)(ca);
            (c.cJSON_Delete)(cb);
            (r.cJSON_Delete)(ra);
            (r.cJSON_Delete)(rb);
        }

        // row 106: key present in a but not in b, and vice versa
        let pairs = [
            (
                Node::Object(vec![("a".into(), Node::Number(1.0))]),
                Node::Object(vec![("b".into(), Node::Number(1.0))]),
            ),
            (
                Node::Object(vec![
                    ("a".into(), Node::Number(1.0)),
                    ("b".into(), Node::Number(2.0)),
                ]),
                Node::Object(vec![("a".into(), Node::Number(1.0))]),
            ),
            (
                Node::Object(vec![("a".into(), Node::Number(1.0))]),
                Node::Object(vec![
                    ("a".into(), Node::Number(1.0)),
                    ("b".into(), Node::Number(2.0)),
                ]),
            ),
        ];
        for (na, nb) in &pairs {
            let ca = build(&c, na);
            let cb = build(&c, nb);
            let ra = build(&r, na);
            let rb = build(&r, nb);
            for cs_flag in [0i32, 1] {
                assert_eq!(
                    (c.cJSON_Compare)(ca, cb, cs_flag),
                    (r.cJSON_Compare)(ra, rb, cs_flag),
                    "row106"
                );
                assert_eq!(
                    (c.cJSON_Compare)(cb, ca, cs_flag),
                    (r.cJSON_Compare)(rb, ra, cs_flag),
                    "row106 reversed"
                );
            }
            (c.cJSON_Delete)(ca);
            (c.cJSON_Delete)(cb);
            (r.cJSON_Delete)(ra);
            (r.cJSON_Delete)(rb);
        }
    }
}

/// Rows 107, 108 — cJSON_Minify edge cases
#[test]
fn rows107_108_minify() {
    unsafe {
        let (c, r) = both();
        // row 107: NULL input must be a no-op
        (c.cJSON_Minify)(ptr::null_mut());
        (r.cJSON_Minify)(ptr::null_mut());
        // row 108: unterminated comment / string
        for t in [
            &b"/* unterminated"[..],
            b"/*",
            b"/",
            b"//",
            b"\"unterminated",
            b"\"esc\\",
            b"{\"a\":\"b",
            b"[1,2/*",
            b"",
            b"\"",
        ] {
            let mut a = cbytes(t);
            let mut b = cbytes(t);
            (c.cJSON_Minify)(a.as_mut_ptr() as *mut c_char);
            (r.cJSON_Minify)(b.as_mut_ptr() as *mut c_char);
            assert_eq!(
                a,
                b,
                "row108 {:?}: C={:?} RUST={:?}",
                String::from_utf8_lossy(t),
                String::from_utf8_lossy(&a),
                String::from_utf8_lossy(&b)
            );
        }
    }
}

/// Rows 109, 110 — predicates with NULL / invalid types
#[test]
fn rows109_110_predicates() {
    unsafe {
        let (c, r) = both();
        macro_rules! p {
            ($f:ident) => {{
                assert_eq!(
                    (c.$f)(ptr::null()),
                    (r.$f)(ptr::null()),
                    "row109 {} NULL",
                    stringify!($f)
                );
                assert_eq!((c.$f)(ptr::null()), 0, "row109 {} C false", stringify!($f));
            }};
        }
        p!(cJSON_IsInvalid);
        p!(cJSON_IsFalse);
        p!(cJSON_IsTrue);
        p!(cJSON_IsBool);
        p!(cJSON_IsNull);
        p!(cJSON_IsNumber);
        p!(cJSON_IsString);
        p!(cJSON_IsArray);
        p!(cJSON_IsObject);
        p!(cJSON_IsRaw);

        // row 110: cJSON_IsInvalid only for type & 0xFF == 0
        for t in [0i32, 0x100, 0x200, 0x300, 1, 2, 0xFF, 0x1FF, -1, 0x10000] {
            let n = Node::Retyped(Box::new(Node::Number(1.0)), t);
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            assert_eq!(
                (c.cJSON_IsInvalid)(ci),
                (r.cJSON_IsInvalid)(ri),
                "row110 type={:#x}",
                t
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Rows 111..114 — hooks and allocator edge cases
#[test]
fn rows111_114_hooks_and_alloc() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        // row 111
        (c.cJSON_InitHooks)(ptr::null_mut());
        (r.cJSON_InitHooks)(ptr::null_mut());
        let n = Node::Object(vec![("a".into(), Node::Number(1.0))]);
        let ci = build(&c, &n);
        let ri = build(&r, &n);
        assert_eq!(
            show(&print_and_free(&c, ci)),
            show(&print_and_free(&r, ri)),
            "row111 after reset"
        );
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);

        // row 112: hooks struct with NULL members
        let mut h = CJsonHooks {
            malloc_fn: None,
            free_fn: None,
        };
        (c.cJSON_InitHooks)(&mut h);
        (r.cJSON_InitHooks)(&mut h);
        let ci = build(&c, &n);
        let ri = build(&r, &n);
        assert_eq!(
            show(&print_and_free(&c, ci)),
            show(&print_and_free(&r, ri)),
            "row112 NULL members"
        );
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);
        (c.cJSON_InitHooks)(ptr::null_mut());
        (r.cJSON_InitHooks)(ptr::null_mut());

        // rows 113, 114
        (c.cJSON_free)(ptr::null_mut::<c_void>());
        (r.cJSON_free)(ptr::null_mut::<c_void>());
        for size in [0usize, 1] {
            let pc = (c.cJSON_malloc)(size);
            let pr = (r.cJSON_malloc)(size);
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "row114 cJSON_malloc({}) NULL-ness",
                size
            );
            (c.cJSON_free)(pc);
            (r.cJSON_free)(pr);
        }
    }
}

/// Rows 115..118 — number clamping
#[test]
fn rows115_118_number_clamping() {
    unsafe {
        let (c, r) = both();
        let vals: Vec<f64> = vec![
            2147483647.0,
            2147483647.5,
            2147483648.0,
            2147483649.0,
            1e300,
            f64::INFINITY,
            -2147483647.0,
            -2147483648.0,
            -2147483648.5,
            -2147483649.0,
            -1e300,
            f64::NEG_INFINITY,
            f64::NAN,
            -f64::NAN,
            0.0,
            -0.0,
            0.5,
            -0.5,
            f64::MIN,
            f64::MAX,
        ];
        for v in vals {
            // rows 115..117 through cJSON_SetNumberHelper
            let ci = (c.cJSON_CreateNumber)(0.0);
            let ri = (r.cJSON_CreateNumber)(0.0);
            let a = (c.cJSON_SetNumberHelper)(ci, v);
            let b = (r.cJSON_SetNumberHelper)(ri, v);
            assert_eq!(a.to_bits(), b.to_bits(), "rows115_117 return {:?}", v);
            assert_eq!((*ci).valueint, (*ri).valueint, "rows115_117 valueint {:?}", v);
            assert_eq!(
                (*ci).valuedouble.to_bits(),
                (*ri).valuedouble.to_bits(),
                "rows115_117 valuedouble {:?}",
                v
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
            // row 118 through cJSON_CreateNumber
            let ci = (c.cJSON_CreateNumber)(v);
            let ri = (r.cJSON_CreateNumber)(v);
            assert_eq!((*ci).valueint, (*ri).valueint, "row118 valueint {:?}", v);
            assert_eq!(
                (*ci).valuedouble.to_bits(),
                (*ri).valuedouble.to_bits(),
                "row118 valuedouble {:?}",
                v
            );
            assert_eq!(
                show(&print_and_free(&c, ci)),
                show(&print_and_free(&r, ri)),
                "row118 print {:?}",
                v
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Row 119 — cJSON_GetErrorPtr after a successful parse
#[test]
fn row119_error_ptr_after_success() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let buf = cbytes(b"{\"a\":1}");
        let ci = (c.cJSON_Parse)(buf.as_ptr() as *const c_char);
        let ri = (r.cJSON_Parse)(buf.as_ptr() as *const c_char);
        assert!(!ci.is_null() && !ri.is_null(), "row119 parse must succeed");
        let ec = (c.cJSON_GetErrorPtr)();
        let er = (r.cJSON_GetErrorPtr)();
        assert_eq!(ec.is_null(), er.is_null(), "row119 NULL-ness");
        assert_eq!(
            ec as isize - buf.as_ptr() as isize,
            er as isize - buf.as_ptr() as isize,
            "row119 offset"
        );
        (c.cJSON_Delete)(ci);
        (r.cJSON_Delete)(ri);
    }
}

/// Row 120 — out-of-range `cJSON_bool` values crossing the FFI boundary
#[test]
fn row120_out_of_range_bools() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let flags = [0i32, 1, 2, 42, -1, c_int::MAX, c_int::MIN, 256, 0x10000];
        let n = Node::Object(vec![
            ("a".into(), Node::Number(1.0)),
            ("B".into(), Node::Str("s".into())),
        ]);
        for f in flags {
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            // Compare
            assert_eq!(
                (c.cJSON_Compare)(ci, ci, f),
                (r.cJSON_Compare)(ri, ri, f),
                "row120 Compare flag={}",
                f
            );
            // Duplicate
            let dc = (c.cJSON_Duplicate)(ci, f);
            let dr = (r.cJSON_Duplicate)(ri, f);
            assert_eq!(dc.is_null(), dr.is_null(), "row120 Duplicate flag={}", f);
            assert_eq!(
                show(&print_and_free(&c, dc)),
                show(&print_and_free(&r, dr)),
                "row120 Duplicate print flag={}",
                f
            );
            (c.cJSON_Delete)(dc);
            (r.cJSON_Delete)(dr);
            // CreateBool
            let bc = (c.cJSON_CreateBool)(f);
            let br = (r.cJSON_CreateBool)(f);
            assert_eq!((*bc).type_, (*br).type_, "row120 CreateBool flag={}", f);
            assert_eq!(
                show(&print_and_free(&c, bc)),
                show(&print_and_free(&r, br)),
                "row120 CreateBool print flag={}",
                f
            );
            (c.cJSON_Delete)(bc);
            (r.cJSON_Delete)(br);
            // PrintBuffered / PrintPreallocated format flag
            assert_eq!(
                show(&print_buffered_and_free(&c, ci, 8, f)),
                show(&print_buffered_and_free(&r, ri, 8, f)),
                "row120 PrintBuffered flag={}",
                f
            );
            let mut cb = vec![0u8; 4096];
            let mut rb = vec![0u8; 4096];
            assert_eq!(
                (c.cJSON_PrintPreallocated)(ci, cb.as_mut_ptr() as *mut c_char, 4096, f),
                (r.cJSON_PrintPreallocated)(ri, rb.as_mut_ptr() as *mut c_char, 4096, f),
                "row120 PrintPreallocated flag={}",
                f
            );
            assert_eq!(cb, rb, "row120 PrintPreallocated bytes flag={}", f);
            // AddBoolToObject
            let co = (c.cJSON_CreateObject)();
            let ro = (r.cJSON_CreateObject)();
            let kb = cbytes(b"flag");
            (c.cJSON_AddBoolToObject)(co, kb.as_ptr() as *const c_char, f);
            (r.cJSON_AddBoolToObject)(ro, kb.as_ptr() as *const c_char, f);
            assert_eq!(
                show(&print_and_free(&c, co)),
                show(&print_and_free(&r, ro)),
                "row120 AddBoolToObject flag={}",
                f
            );
            (c.cJSON_Delete)(co);
            (r.cJSON_Delete)(ro);
            // ParseWithOpts require_null_terminated
            for text in ["{}", "{} ", "{}x"] {
                let buf = cbytes(text.as_bytes());
                let a = (c.cJSON_ParseWithOpts)(buf.as_ptr() as *const c_char, ptr::null_mut(), f);
                let b = (r.cJSON_ParseWithOpts)(buf.as_ptr() as *const c_char, ptr::null_mut(), f);
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "row120 ParseWithOpts {:?} flag={}",
                    text,
                    f
                );
                assert_eq!(
                    (c.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize,
                    (r.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize,
                    "row120 error offset {:?} flag={}",
                    text,
                    f
                );
                (c.cJSON_Delete)(a);
                (r.cJSON_Delete)(b);
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Row 121 — out-of-range `type` values crossing the FFI boundary
#[test]
fn row121_out_of_range_types() {
    unsafe {
        let (c, r) = both();
        let types = [
            -1i32,
            0,
            1,
            2,
            3,
            4,
            8,
            0x1FF,
            0x10000,
            0x7FFFFFFF,
            c_int::MIN,
            0xFF,
            cJSON_Array | cJSON_IsReference,
            cJSON_Object | cJSON_StringIsConst,
            cJSON_Number | cJSON_IsReference | cJSON_StringIsConst,
        ];
        for t in types {
            for base in [
                Node::Number(1.5),
                Node::Str("s".into()),
                Node::Array(vec![Node::Number(1.0)]),
                Node::Object(vec![("k".into(), Node::Number(1.0))]),
            ] {
                let n = Node::Retyped(Box::new(base), t);
                let ci = build(&c, &n);
                let ri = build(&r, &n);
                assert_eq!(
                    show(&print_and_free(&c, ci)),
                    show(&print_and_free(&r, ri)),
                    "row121 Print type={:#x}",
                    t
                );
                assert_eq!(
                    show(&print_unformatted_and_free(&c, ci)),
                    show(&print_unformatted_and_free(&r, ri)),
                    "row121 PrintUnformatted type={:#x}",
                    t
                );
                assert_eq!(
                    (c.cJSON_Compare)(ci, ci, 1),
                    (r.cJSON_Compare)(ri, ri, 1),
                    "row121 Compare type={:#x}",
                    t
                );
                let dc = (c.cJSON_Duplicate)(ci, 1);
                let dr = (r.cJSON_Duplicate)(ri, 1);
                assert_eq!(dc.is_null(), dr.is_null(), "row121 Duplicate type={:#x}", t);
                if !dc.is_null() {
                    assert_eq!((*dc).type_, (*dr).type_, "row121 duplicate type");
                    assert_eq!(
                        show(&print_and_free(&c, dc)),
                        show(&print_and_free(&r, dr)),
                        "row121 duplicate print type={:#x}",
                        t
                    );
                }
                (c.cJSON_Delete)(dc);
                (r.cJSON_Delete)(dr);
                assert_eq!(
                    (c.cJSON_GetArraySize)(ci),
                    (r.cJSON_GetArraySize)(ri),
                    "row121 GetArraySize type={:#x}",
                    t
                );
                macro_rules! p {
                    ($f:ident) => {
                        assert_eq!(
                            (c.$f)(ci),
                            (r.$f)(ri),
                            "row121 {} type={:#x}",
                            stringify!($f),
                            t
                        );
                    };
                }
                p!(cJSON_IsInvalid);
                p!(cJSON_IsFalse);
                p!(cJSON_IsTrue);
                p!(cJSON_IsBool);
                p!(cJSON_IsNull);
                p!(cJSON_IsNumber);
                p!(cJSON_IsString);
                p!(cJSON_IsArray);
                p!(cJSON_IsObject);
                p!(cJSON_IsRaw);
                // cJSON_Delete must handle the reference / const-key bits identically
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);
            }
        }
    }
}
