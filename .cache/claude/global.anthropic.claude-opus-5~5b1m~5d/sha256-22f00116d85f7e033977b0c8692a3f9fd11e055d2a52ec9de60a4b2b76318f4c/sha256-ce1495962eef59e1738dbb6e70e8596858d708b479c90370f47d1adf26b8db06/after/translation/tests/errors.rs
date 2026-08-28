//! Phase C — one differential test per ERRORS.md row that is reachable with the
//! default allocator.  Rows needing a failing allocator live in `hooks.rs`;
//! row 25 lives in `bigalloc.rs`.
//!
//! Every test constructs the exact invalid input/condition, calls BOTH `.so`s
//! and asserts they return the SAME error code / sentinel — not merely that both
//! failed somehow.
#![allow(non_snake_case)]

mod harness;

use harness::*;
use std::ffi::{c_char, c_int, c_void};

/// The nine distinct item shapes, one per `print_value` switch arm.
fn one_of_each_type() -> Vec<(&'static str, Spec)> {
    vec![
        ("Invalid", Spec::Arr(vec![])), // retyped below where needed
        ("NULL", Spec::Null),
        ("True", Spec::True),
        ("False", Spec::False),
        ("Number", Spec::Num(1.5)),
        ("String", Spec::Str(b"s".to_vec())),
        ("StringRef", Spec::StrRef(b"s".to_vec())),
        ("Raw", Spec::Raw(b"1".to_vec())),
        ("Array", Spec::Arr(vec![Spec::Num(1.0)])),
        ("Object", Spec::Obj(vec![(b"k".to_vec(), Spec::Num(1.0))])),
    ]
}

/// Fabricated `type` values with no valid variant — a C `int`/enum accepts any
/// value, so these are real inputs that must be handled identically.
const BAD_TYPES: [c_int; 18] = [
    0, 3, 5, 6, 7, 9, 0x0A, 0x18, 0x30, 0x88, 0xFF, 0x100, 0x200, 0x300, 0x1FF, -1, i32::MIN,
    i32::MAX,
];

/// Out-of-range values for the `cJSON_bool` / `int` parameters.
const BAD_BOOLS: [c_int; 7] = [2, -1, 3, 0x10000, i32::MIN, i32::MAX, -0x10000];

// ===========================================================================
// rows 1–4 — value accessors
// ===========================================================================
#[test]
fn err_get_string_value() {
    let (c, r) = both();
    unsafe {
        assert_eq!(
            (c.cJSON_GetStringValue)(std::ptr::null()),
            std::ptr::null_mut(),
            "C: GetStringValue(NULL) must be NULL"
        );
        assert_eq!(
            (c.cJSON_GetStringValue)(std::ptr::null()).is_null(),
            (r.cJSON_GetStringValue)(std::ptr::null()).is_null(),
            "row 1: GetStringValue(NULL)"
        );
        for (name, spec) in one_of_each_type() {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            let a = (c.cJSON_GetStringValue)(bc.root);
            let b = (r.cJSON_GetStringValue)(br.root);
            assert_eq!(a.is_null(), b.is_null(), "row 2: GetStringValue on {name}");
            assert_eq!(cstr(a), cstr(b), "row 2: GetStringValue contents on {name}");
            bc.delete();
            br.delete();
        }
        for t in BAD_TYPES {
            let nc = (c.cJSON_CreateString)(cs("v").as_ptr());
            let nr = (r.cJSON_CreateString)(cs("v").as_ptr());
            (*nc).type_ = t;
            (*nr).type_ = t;
            assert_eq!(
                (c.cJSON_GetStringValue)(nc).is_null(),
                (r.cJSON_GetStringValue)(nr).is_null(),
                "row 2: GetStringValue with out-of-range type {t:#x}"
            );
            (*nc).type_ = cJSON_String;
            (*nr).type_ = cJSON_String;
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }
    }
}

#[test]
fn err_get_number_value() {
    let (c, r) = both();
    unsafe {
        let a = (c.cJSON_GetNumberValue)(std::ptr::null());
        let b = (r.cJSON_GetNumberValue)(std::ptr::null());
        assert!(a.is_nan(), "C: GetNumberValue(NULL) must be NaN");
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "row 3: GetNumberValue(NULL) NaN bit pattern"
        );
        for (name, spec) in one_of_each_type() {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            assert_eq!(
                (c.cJSON_GetNumberValue)(bc.root).to_bits(),
                (r.cJSON_GetNumberValue)(br.root).to_bits(),
                "row 4: GetNumberValue on {name}"
            );
            bc.delete();
            br.delete();
        }
        for t in BAD_TYPES {
            let nc = (c.cJSON_CreateNumber)(7.25);
            let nr = (r.cJSON_CreateNumber)(7.25);
            (*nc).type_ = t;
            (*nr).type_ = t;
            assert_eq!(
                (c.cJSON_GetNumberValue)(nc).to_bits(),
                (r.cJSON_GetNumberValue)(nr).to_bits(),
                "row 4: GetNumberValue with out-of-range type {t:#x}"
            );
            (*nc).type_ = cJSON_Number;
            (*nr).type_ = cJSON_Number;
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }
    }
}

// ===========================================================================
// rows 5, 97–101 — object lookup rejections
// ===========================================================================
#[test]
fn err_get_object_item() {
    let (c, r) = both();
    let key = Bytes::new(b"k");
    unsafe {
        // row 97: object == NULL
        for f in 0..2 {
            let a = if f == 0 {
                (c.cJSON_GetObjectItem)(std::ptr::null(), key.as_ptr())
            } else {
                (c.cJSON_GetObjectItemCaseSensitive)(std::ptr::null(), key.as_ptr())
            };
            let b = if f == 0 {
                (r.cJSON_GetObjectItem)(std::ptr::null(), key.as_ptr())
            } else {
                (r.cJSON_GetObjectItemCaseSensitive)(std::ptr::null(), key.as_ptr())
            };
            assert!(a.is_null(), "C: lookup on NULL object must be NULL");
            assert_eq!(a.is_null(), b.is_null(), "row 97 (variant {f})");
        }
        assert_eq!(
            (c.cJSON_HasObjectItem)(std::ptr::null(), key.as_ptr()),
            (r.cJSON_HasObjectItem)(std::ptr::null(), key.as_ptr()),
            "row 101: HasObjectItem(NULL, key)"
        );

        for (name, spec) in one_of_each_type() {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            // row 98: name == NULL
            assert_eq!(
                (c.cJSON_GetObjectItem)(bc.root, std::ptr::null()).is_null(),
                (r.cJSON_GetObjectItem)(br.root, std::ptr::null()).is_null(),
                "row 98: GetObjectItem(obj, NULL) on {name}"
            );
            assert!(
                (c.cJSON_GetObjectItem)(bc.root, std::ptr::null()).is_null(),
                "C: GetObjectItem(obj, NULL) must be NULL"
            );
            assert_eq!(
                (c.cJSON_GetObjectItemCaseSensitive)(bc.root, std::ptr::null()).is_null(),
                (r.cJSON_GetObjectItemCaseSensitive)(br.root, std::ptr::null()).is_null(),
                "row 98: …CaseSensitive(obj, NULL) on {name}"
            );
            assert_eq!(
                (c.cJSON_HasObjectItem)(bc.root, std::ptr::null()),
                (r.cJSON_HasObjectItem)(br.root, std::ptr::null()),
                "row 101: HasObjectItem(obj, NULL) on {name}"
            );
            // row 99: key absent
            let missing = Bytes::new(b"definitely_absent");
            assert_eq!(
                (c.cJSON_GetObjectItem)(bc.root, missing.as_ptr()).is_null(),
                (r.cJSON_GetObjectItem)(br.root, missing.as_ptr()).is_null(),
                "row 99: absent key on {name}"
            );
            assert_eq!(
                (c.cJSON_GetObjectItemCaseSensitive)(bc.root, missing.as_ptr()).is_null(),
                (r.cJSON_GetObjectItemCaseSensitive)(br.root, missing.as_ptr()).is_null(),
                "row 99: absent key CS on {name}"
            );
            bc.delete();
            br.delete();
        }

        // rows 5, 100: children whose `string` is NULL (array elements)
        let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0), Spec::Num(3.0)]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        for k in [&b"k"[..], &b""[..], &b"1"[..]] {
            let kb = Bytes::new(k);
            assert!(
                (c.cJSON_GetObjectItem)(bc.root, kb.as_ptr()).is_null(),
                "C: lookup in array-of-unnamed must be NULL"
            );
            assert_eq!(
                (c.cJSON_GetObjectItem)(bc.root, kb.as_ptr()).is_null(),
                (r.cJSON_GetObjectItem)(br.root, kb.as_ptr()).is_null(),
                "rows 5/100: case-insensitive walk over NULL keys"
            );
            assert_eq!(
                (c.cJSON_GetObjectItemCaseSensitive)(bc.root, kb.as_ptr()).is_null(),
                (r.cJSON_GetObjectItemCaseSensitive)(br.root, kb.as_ptr()).is_null(),
                "row 100: case-sensitive walk stops at a NULL key"
            );
        }
        // a MIXED list: named, unnamed, named — the case-sensitive loop stops at
        // the unnamed element and therefore cannot see the third entry.
        let mixed_c = (c.cJSON_CreateObject)();
        let mixed_r = (r.cJSON_CreateObject)();
        for (api, o) in [(&c, mixed_c), (&r, mixed_r)] {
            let n1 = (api.cJSON_CreateNumber)(1.0);
            (api.cJSON_AddItemToObject)(o, cs("first").as_ptr(), n1);
            let n2 = (api.cJSON_CreateNumber)(2.0);
            (api.cJSON_AddItemToArray)(o, n2); // no key at all
            let n3 = (api.cJSON_CreateNumber)(3.0);
            (api.cJSON_AddItemToObject)(o, cs("third").as_ptr(), n3);
        }
        for k in [&b"first"[..], &b"third"[..], &b"FIRST"[..], &b"THIRD"[..]] {
            let kb = Bytes::new(k);
            assert_eq!(
                snap((c.cJSON_GetObjectItem)(mixed_c, kb.as_ptr())),
                snap((r.cJSON_GetObjectItem)(mixed_r, kb.as_ptr())),
                "row 100: mixed list, case-insensitive, key {:?}",
                String::from_utf8_lossy(k)
            );
            assert_eq!(
                snap((c.cJSON_GetObjectItemCaseSensitive)(mixed_c, kb.as_ptr())),
                snap((r.cJSON_GetObjectItemCaseSensitive)(mixed_r, kb.as_ptr())),
                "row 100: mixed list, case-sensitive, key {:?}",
                String::from_utf8_lossy(k)
            );
        }
        (c.cJSON_Delete)(mixed_c);
        (r.cJSON_Delete)(mixed_r);
        bc.delete();
        br.delete();
    }
}

// ===========================================================================
// rows 6, 150, 151 — cJSON_CreateString/Raw with a NULL argument
// ===========================================================================
#[test]
fn err_create_string_null() {
    let (c, r) = both();
    unsafe {
        let a = (c.cJSON_CreateString)(std::ptr::null());
        let b = (r.cJSON_CreateString)(std::ptr::null());
        assert!(a.is_null(), "C: CreateString(NULL) must be NULL");
        assert_eq!(a.is_null(), b.is_null(), "rows 6/150: CreateString(NULL)");

        let a = (c.cJSON_CreateRaw)(std::ptr::null());
        let b = (r.cJSON_CreateRaw)(std::ptr::null());
        assert!(a.is_null(), "C: CreateRaw(NULL) must be NULL");
        assert_eq!(a.is_null(), b.is_null(), "row 151: CreateRaw(NULL)");

        // cJSON_CreateStringReference(NULL) is NOT an error: it yields an item
        // whose valuestring is NULL.
        let a = (c.cJSON_CreateStringReference)(std::ptr::null());
        let b = (r.cJSON_CreateStringReference)(std::ptr::null());
        assert_eq!(snap(a), snap(b), "CreateStringReference(NULL)");
        assert_eq!(
            print_and_take(&c, a),
            print_and_take(&r, b),
            "print of CreateStringReference(NULL)"
        );
        (c.cJSON_Delete)(a);
        (r.cJSON_Delete)(b);

        // CreateObjectReference/CreateArrayReference(NULL) likewise
        for as_object in [false, true] {
            let a = if as_object {
                (c.cJSON_CreateObjectReference)(std::ptr::null())
            } else {
                (c.cJSON_CreateArrayReference)(std::ptr::null())
            };
            let b = if as_object {
                (r.cJSON_CreateObjectReference)(std::ptr::null())
            } else {
                (r.cJSON_CreateArrayReference)(std::ptr::null())
            };
            assert_eq!(snap(a), snap(b), "Create*Reference(NULL) as_object={as_object}");
            assert_eq!(print_and_take(&c, a), print_and_take(&r, b));
            (c.cJSON_Delete)(a);
            (r.cJSON_Delete)(b);
        }
    }
}

// ===========================================================================
// rows 12–17 — cJSON_SetValuestring rejections
// ===========================================================================
#[test]
fn err_set_valuestring() {
    let (c, r) = both();
    let new = Bytes::new(b"new value");
    unsafe {
        // row 12: object == NULL
        let a = (c.cJSON_SetValuestring)(std::ptr::null_mut(), new.as_ptr());
        let b = (r.cJSON_SetValuestring)(std::ptr::null_mut(), new.as_ptr());
        assert!(a.is_null(), "C: SetValuestring(NULL, x) must be NULL");
        assert_eq!(a.is_null(), b.is_null(), "row 12");

        // rows 13, 14: wrong type / reference
        for (name, spec) in one_of_each_type() {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            let a = (c.cJSON_SetValuestring)(bc.root, new.as_ptr());
            let b = (r.cJSON_SetValuestring)(br.root, new.as_ptr());
            assert_eq!(a.is_null(), b.is_null(), "rows 13/14: on {name}");
            assert_eq!(cstr(a), cstr(b), "rows 13/14: contents on {name}");
            assert_eq!(snap(bc.root), snap(br.root), "rows 13/14: state on {name}");
            bc.delete();
            br.delete();
        }
        // every fabricated type
        for t in BAD_TYPES {
            let nc = (c.cJSON_CreateString)(cs("old").as_ptr());
            let nr = (r.cJSON_CreateString)(cs("old").as_ptr());
            (*nc).type_ = t;
            (*nr).type_ = t;
            let a = (c.cJSON_SetValuestring)(nc, new.as_ptr());
            let b = (r.cJSON_SetValuestring)(nr, new.as_ptr());
            assert_eq!(a.is_null(), b.is_null(), "rows 13/14: type {t:#x}");
            assert_eq!(cstr(a), cstr(b), "rows 13/14: contents type {t:#x}");
            (*nc).type_ = cJSON_String;
            (*nr).type_ = cJSON_String;
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }

        // row 15: object->valuestring == NULL
        let nc = (c.cJSON_CreateString)(cs("old").as_ptr());
        let nr = (r.cJSON_CreateString)(cs("old").as_ptr());
        (c.cJSON_free)((*nc).valuestring as *mut c_void);
        (r.cJSON_free)((*nr).valuestring as *mut c_void);
        (*nc).valuestring = std::ptr::null_mut();
        (*nr).valuestring = std::ptr::null_mut();
        let a = (c.cJSON_SetValuestring)(nc, new.as_ptr());
        let b = (r.cJSON_SetValuestring)(nr, new.as_ptr());
        assert!(a.is_null(), "C: SetValuestring with NULL valuestring must fail");
        assert_eq!(a.is_null(), b.is_null(), "row 15");
        assert_eq!(snap(nc), snap(nr), "row 15: state");
        (c.cJSON_Delete)(nc);
        (r.cJSON_Delete)(nr);

        // row 16: valuestring argument == NULL
        let nc = (c.cJSON_CreateString)(cs("old").as_ptr());
        let nr = (r.cJSON_CreateString)(cs("old").as_ptr());
        let a = (c.cJSON_SetValuestring)(nc, std::ptr::null());
        let b = (r.cJSON_SetValuestring)(nr, std::ptr::null());
        assert!(a.is_null(), "C: SetValuestring(_, NULL) must fail");
        assert_eq!(a.is_null(), b.is_null(), "row 16");
        assert_eq!(snap(nc), snap(nr), "row 16: state");
        (c.cJSON_Delete)(nc);
        (r.cJSON_Delete)(nr);
    }
}

#[test]
fn err_set_valuestring_overlap() {
    let (c, r) = both();
    unsafe {
        // row 17a: pass the item's own valuestring (fully overlapping)
        let nc = (c.cJSON_CreateString)(cs("abcdefgh").as_ptr());
        let nr = (r.cJSON_CreateString)(cs("abcdefgh").as_ptr());
        let a = (c.cJSON_SetValuestring)(nc, (*nc).valuestring);
        let b = (r.cJSON_SetValuestring)(nr, (*nr).valuestring);
        assert!(a.is_null(), "C: self-overlapping SetValuestring must be NULL");
        assert_eq!(a.is_null(), b.is_null(), "row 17a");
        assert_eq!(cstr((*nc).valuestring), cstr((*nr).valuestring), "row 17a: value");
        assert_eq!(cstr((*nc).valuestring), Some(b"abcdefgh".to_vec()));

        // row 17b: a proper suffix of the item's own valuestring
        for off in [1isize, 2, 4, 7] {
            let a = (c.cJSON_SetValuestring)(nc, (*nc).valuestring.offset(off));
            let b = (r.cJSON_SetValuestring)(nr, (*nr).valuestring.offset(off));
            assert!(a.is_null(), "C: suffix-overlapping SetValuestring must be NULL");
            assert_eq!(a.is_null(), b.is_null(), "row 17b off={off}");
            assert_eq!(
                cstr((*nc).valuestring),
                cstr((*nr).valuestring),
                "row 17b off={off}: value"
            );
        }
        // and the NUL terminator itself (v1_len == 0 <= v2_len, still overlapping)
        let a = (c.cJSON_SetValuestring)(nc, (*nc).valuestring.offset(8));
        let b = (r.cJSON_SetValuestring)(nr, (*nr).valuestring.offset(8));
        assert_eq!(a.is_null(), b.is_null(), "row 17b at the terminator");
        assert_eq!(cstr((*nc).valuestring), cstr((*nr).valuestring));

        (c.cJSON_Delete)(nc);
        (r.cJSON_Delete)(nr);

        // Contrast: a genuinely disjoint shorter value takes the strcpy path.
        let nc = (c.cJSON_CreateString)(cs("abcdefgh").as_ptr());
        let nr = (r.cJSON_CreateString)(cs("abcdefgh").as_ptr());
        let short = Bytes::new(b"xy");
        let a = (c.cJSON_SetValuestring)(nc, short.as_ptr());
        let b = (r.cJSON_SetValuestring)(nr, short.as_ptr());
        assert!(!a.is_null(), "C: disjoint shorter value must succeed");
        assert_eq!(cstr(a), cstr(b), "disjoint shorter value");
        assert_eq!(snap(nc), snap(nr));
        (c.cJSON_Delete)(nc);
        (r.cJSON_Delete)(nr);
    }
}

// ===========================================================================
// rows 21, 23, 29, 45, 46, 61, 62, 63, 70, 78, 90, 91 — print into a buffer
// ===========================================================================
#[test]
fn err_print_preallocated_negative() {
    let (c, r) = both();
    unsafe {
        let mut buf = vec![0u8; 256];
        let spec = Spec::Obj(vec![(b"a".to_vec(), Spec::Num(1.0))]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        for len in [-1i32, -2, -1000, i32::MIN] {
            for fmt in [0, 1] {
                let a = (c.cJSON_PrintPreallocated)(
                    bc.root,
                    buf.as_mut_ptr() as *mut c_char,
                    len,
                    fmt,
                );
                let b = (r.cJSON_PrintPreallocated)(
                    br.root,
                    buf.as_mut_ptr() as *mut c_char,
                    len,
                    fmt,
                );
                assert_eq!(a, 0, "C: PrintPreallocated(length={len}) must return 0");
                assert_eq!(a, b, "row 61: length={len} fmt={fmt}");
            }
        }
        bc.delete();
        br.delete();
    }
}

#[test]
fn err_print_preallocated_null_buffer() {
    let (c, r) = both();
    unsafe {
        let spec = Spec::Obj(vec![(b"a".to_vec(), Spec::Num(1.0))]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        for len in [-1i32, 0, 1, 1024] {
            for fmt in [0, 1] {
                let a = (c.cJSON_PrintPreallocated)(bc.root, std::ptr::null_mut(), len, fmt);
                let b = (r.cJSON_PrintPreallocated)(br.root, std::ptr::null_mut(), len, fmt);
                assert_eq!(a, 0, "C: PrintPreallocated(NULL buffer) must return 0");
                assert_eq!(a, b, "row 62: length={len} fmt={fmt}");
            }
        }
        // NULL item as well
        for len in [0i32, 16] {
            let a = (c.cJSON_PrintPreallocated)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                len,
                1,
            );
            let b = (r.cJSON_PrintPreallocated)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                len,
                1,
            );
            assert_eq!(a, b, "row 62/66: NULL item and NULL buffer, length={len}");
        }
        bc.delete();
        br.delete();
    }
}

#[test]
fn err_print_preallocated_small() {
    let (c, r) = both();
    // Every output position of a tree that touches every `ensure` call site:
    // null/false/true literals, numbers, strings needing escapes, Raw payloads,
    // nested arrays and objects (both formats).
    let specs = [
        Spec::Null,
        Spec::True,
        Spec::False,
        Spec::Num(1.5),
        Spec::Num(f64::NAN),
        Spec::Str(b"needs \"escapes\"\n\x01".to_vec()),
        Spec::Str(b"plain".to_vec()),
        Spec::Raw(b"{\"raw\":1}".to_vec()),
        Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0), Spec::Num(3.0)]),
        Spec::Obj(vec![
            (b"a".to_vec(), Spec::Num(1.0)),
            (b"b".to_vec(), Spec::Arr(vec![Spec::True, Spec::Null])),
            (b"c\n".to_vec(), Spec::Str(b"v\t".to_vec())),
        ]),
        Spec::Obj(vec![(
            b"outer".to_vec(),
            Spec::Obj(vec![(b"inner".to_vec(), Spec::Obj(vec![]))]),
        )]),
    ];
    unsafe {
        for (si, spec) in specs.iter().enumerate() {
            let bc = build(&c, spec);
            let br = build(&r, spec);
            for fmt in [0, 1] {
                let want = if fmt == 1 {
                    print_and_take(&c, bc.root)
                } else {
                    print_unformatted_and_take(&c, bc.root)
                };
                let exact = want.as_ref().map(|v| v.len()).unwrap_or(0);
                let mut first_success: Option<usize> = None;
                for len in 0..=(exact + 4) {
                    let mut buf_c = vec![0xCCu8; len + 32];
                    let mut buf_r = vec![0xCCu8; len + 32];
                    let a = (c.cJSON_PrintPreallocated)(
                        bc.root,
                        buf_c.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        fmt,
                    );
                    let b = (r.cJSON_PrintPreallocated)(
                        br.root,
                        buf_r.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        fmt,
                    );
                    assert_eq!(
                        a, b,
                        "rows 21/23/63/70/78/90/91: spec #{si} fmt={fmt} len={len} rc"
                    );
                    assert_eq!(
                        buf_c, buf_r,
                        "rows 21/23/63/70/78/90/91: spec #{si} fmt={fmt} len={len} buffer"
                    );
                    if a != 0 && first_success.is_none() {
                        first_success = Some(len);
                    }
                }
                // Sanity: there really is a rejection range (i.e. the row was hit)
                assert!(
                    first_success.map(|l| l > 0).unwrap_or(true),
                    "spec #{si} fmt={fmt}: expected small buffers to be rejected"
                );
            }
            bc.delete();
            br.delete();
        }

        // row 45: print_string_ptr's `input == NULL` branch with a tiny buffer
        let nc = (c.cJSON_CreateString)(cs("x").as_ptr());
        let nr = (r.cJSON_CreateString)(cs("x").as_ptr());
        (c.cJSON_free)((*nc).valuestring as *mut c_void);
        (r.cJSON_free)((*nr).valuestring as *mut c_void);
        (*nc).valuestring = std::ptr::null_mut();
        (*nr).valuestring = std::ptr::null_mut();
        for len in 0..6usize {
            let mut buf_c = vec![0xCCu8; len + 16];
            let mut buf_r = vec![0xCCu8; len + 16];
            let a = (c.cJSON_PrintPreallocated)(
                nc,
                buf_c.as_mut_ptr() as *mut c_char,
                len as c_int,
                1,
            );
            let b = (r.cJSON_PrintPreallocated)(
                nr,
                buf_r.as_mut_ptr() as *mut c_char,
                len as c_int,
                1,
            );
            assert_eq!(a, b, "row 45: NULL valuestring len={len} rc");
            assert_eq!(buf_c, buf_r, "row 45: NULL valuestring len={len} buffer");
        }
        (c.cJSON_Delete)(nc);
        (r.cJSON_Delete)(nr);
    }
}

#[test]
fn err_print_preallocated_tight() {
    let (c, r) = both();
    // row 21: `ensure` is entered again after the buffer is exactly filled, so
    // `p->offset >= p->length` triggers.  A one-element array does this: '['
    // fills a length-1 buffer, then the element's `ensure` sees offset == length.
    unsafe {
        for spec in [
            Spec::Arr(vec![Spec::Null]),
            Spec::Arr(vec![Spec::Num(1.0)]),
            Spec::Obj(vec![(b"k".to_vec(), Spec::Null)]),
            Spec::Arr(vec![Spec::Str(b"s".to_vec())]),
            Spec::Arr(vec![Spec::Raw(b"1".to_vec())]),
        ] {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            for fmt in [0, 1] {
                for len in 1..8usize {
                    let mut buf_c = vec![0xCCu8; len + 16];
                    let mut buf_r = vec![0xCCu8; len + 16];
                    let a = (c.cJSON_PrintPreallocated)(
                        bc.root,
                        buf_c.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        fmt,
                    );
                    let b = (r.cJSON_PrintPreallocated)(
                        br.root,
                        buf_r.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        fmt,
                    );
                    assert_eq!(a, b, "row 21: {spec:?} fmt={fmt} len={len} rc");
                    assert_eq!(buf_c, buf_r, "row 21: {spec:?} fmt={fmt} len={len} buffer");
                }
            }
            bc.delete();
            br.delete();
        }
    }
}

// ===========================================================================
// rows 55, 66, 68, 69, 79, 92 — print rejections driven by the item
// ===========================================================================
#[test]
fn err_print_null_and_bad_type() {
    let (c, r) = both();
    unsafe {
        // rows 55/66: NULL item through every print entry point
        assert!(
            (c.cJSON_Print)(std::ptr::null()).is_null(),
            "C: cJSON_Print(NULL) must be NULL"
        );
        assert_eq!(
            (c.cJSON_Print)(std::ptr::null()).is_null(),
            (r.cJSON_Print)(std::ptr::null()).is_null(),
            "rows 55/66: cJSON_Print(NULL)"
        );
        assert_eq!(
            (c.cJSON_PrintUnformatted)(std::ptr::null()).is_null(),
            (r.cJSON_PrintUnformatted)(std::ptr::null()).is_null(),
            "rows 55/66: cJSON_PrintUnformatted(NULL)"
        );
        for pb in [0, 1, 256] {
            for fmt in [0, 1] {
                assert_eq!(
                    (c.cJSON_PrintBuffered)(std::ptr::null(), pb, fmt).is_null(),
                    (r.cJSON_PrintBuffered)(std::ptr::null(), pb, fmt).is_null(),
                    "row 60: PrintBuffered(NULL, {pb}, {fmt})"
                );
            }
        }
        let mut buf = vec![0u8; 64];
        for fmt in [0, 1] {
            let a = (c.cJSON_PrintPreallocated)(
                std::ptr::null_mut(),
                buf.as_mut_ptr() as *mut c_char,
                64,
                fmt,
            );
            let b = (r.cJSON_PrintPreallocated)(
                std::ptr::null_mut(),
                buf.as_mut_ptr() as *mut c_char,
                64,
                fmt,
            );
            assert_eq!(a, 0, "C: PrintPreallocated(NULL item) must be 0");
            assert_eq!(a, b, "row 63/66: PrintPreallocated(NULL item) fmt={fmt}");
        }

        // row 68: unknown `type` values through every entry point
        for t in BAD_TYPES {
            let nc = (c.cJSON_CreateString)(cs("payload").as_ptr());
            let nr = (r.cJSON_CreateString)(cs("payload").as_ptr());
            (*nc).type_ = t;
            (*nr).type_ = t;
            assert_eq!(
                print_and_take(&c, nc),
                print_and_take(&r, nr),
                "row 68: cJSON_Print with type {t:#x}"
            );
            assert_eq!(
                print_unformatted_and_take(&c, nc),
                print_unformatted_and_take(&r, nr),
                "row 68: cJSON_PrintUnformatted with type {t:#x}"
            );
            for pb in [0, 1, 256] {
                for fmt in [0, 1] {
                    assert_eq!(
                        print_buffered_and_take(&c, nc, pb, fmt),
                        print_buffered_and_take(&r, nr, pb, fmt),
                        "row 68: PrintBuffered type {t:#x} pb={pb} fmt={fmt}"
                    );
                }
            }
            for fmt in [0, 1] {
                let mut bc2 = vec![0xEEu8; 128];
                let mut br2 = vec![0xEEu8; 128];
                let a = (c.cJSON_PrintPreallocated)(
                    nc,
                    bc2.as_mut_ptr() as *mut c_char,
                    128,
                    fmt,
                );
                let b = (r.cJSON_PrintPreallocated)(
                    nr,
                    br2.as_mut_ptr() as *mut c_char,
                    128,
                    fmt,
                );
                assert_eq!(a, b, "row 68: PrintPreallocated type {t:#x} fmt={fmt} rc");
                assert_eq!(bc2, br2, "row 68: PrintPreallocated type {t:#x} buffer");
            }
            (*nc).type_ = cJSON_String;
            (*nr).type_ = cJSON_String;
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }

        // rows 79/92: an unprintable child aborts print_array / print_object at
        // the exact same point.
        for t in BAD_TYPES {
            for as_object in [false, true] {
                for position in 0..3usize {
                    let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0), Spec::Num(3.0)]);
                    let bc = build(&c, &spec);
                    let br = build(&r, &spec);
                    let pc = (c.cJSON_GetArrayItem)(bc.root, position as c_int);
                    let pr = (r.cJSON_GetArrayItem)(br.root, position as c_int);
                    (*pc).type_ = t;
                    (*pr).type_ = t;
                    if as_object {
                        (*bc.root).type_ = cJSON_Object;
                        (*br.root).type_ = cJSON_Object;
                    }
                    assert_eq!(
                        print_and_take(&c, bc.root),
                        print_and_take(&r, br.root),
                        "rows 79/92: child type {t:#x} at {position}, as_object={as_object}"
                    );
                    assert_eq!(
                        print_unformatted_and_take(&c, bc.root),
                        print_unformatted_and_take(&r, br.root),
                        "rows 79/92 (unformatted): child type {t:#x} at {position}"
                    );
                    (*pc).type_ = cJSON_Number;
                    (*pr).type_ = cJSON_Number;
                    (*bc.root).type_ = cJSON_Array;
                    (*br.root).type_ = cJSON_Array;
                    bc.delete();
                    br.delete();
                }
            }
        }
    }
}

#[test]
fn err_print_raw_null_valuestring() {
    let (c, r) = both();
    unsafe {
        // row 69: Raw item with valuestring == NULL
        for extra in [0, cJSON_IsReference, cJSON_StringIsConst] {
            let nc = (c.cJSON_CreateRaw)(cs("1").as_ptr());
            let nr = (r.cJSON_CreateRaw)(cs("1").as_ptr());
            (c.cJSON_free)((*nc).valuestring as *mut c_void);
            (r.cJSON_free)((*nr).valuestring as *mut c_void);
            (*nc).valuestring = std::ptr::null_mut();
            (*nr).valuestring = std::ptr::null_mut();
            (*nc).type_ = cJSON_Raw | extra;
            (*nr).type_ = cJSON_Raw | extra;
            assert!(
                (c.cJSON_Print)(nc).is_null(),
                "C: printing a Raw item with a NULL payload must fail"
            );
            assert_eq!(
                print_and_take(&c, nc),
                print_and_take(&r, nr),
                "row 69: extra={extra:#x}"
            );
            let mut b1 = vec![0xDDu8; 64];
            let mut b2 = vec![0xDDu8; 64];
            let a = (c.cJSON_PrintPreallocated)(nc, b1.as_mut_ptr() as *mut c_char, 64, 1);
            let b = (r.cJSON_PrintPreallocated)(nr, b2.as_mut_ptr() as *mut c_char, 64, 1);
            assert_eq!(a, 0, "C: PrintPreallocated of Raw/NULL must be 0");
            assert_eq!(a, b, "row 69: PrintPreallocated rc");
            assert_eq!(b1, b2, "row 69: PrintPreallocated buffer");
            (*nc).type_ = cJSON_Raw;
            (*nr).type_ = cJSON_Raw;
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }
    }
}

#[test]
fn err_print_buffered_negative() {
    let (c, r) = both();
    unsafe {
        let spec = Spec::Arr(vec![Spec::Num(1.0)]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        for pb in [-1i32, -2, -1000, i32::MIN] {
            for fmt in [0, 1] {
                let a = (c.cJSON_PrintBuffered)(bc.root, pb, fmt);
                let b = (r.cJSON_PrintBuffered)(br.root, pb, fmt);
                assert!(a.is_null(), "C: PrintBuffered(prebuffer={pb}) must be NULL");
                assert_eq!(a.is_null(), b.is_null(), "row 58: prebuffer={pb} fmt={fmt}");
            }
        }
        bc.delete();
        br.delete();
    }
}

#[test]
fn err_print_buffered_bad_item() {
    let (c, r) = both();
    unsafe {
        for t in BAD_TYPES {
            let nc = (c.cJSON_CreateString)(cs("p").as_ptr());
            let nr = (r.cJSON_CreateString)(cs("p").as_ptr());
            (*nc).type_ = t;
            (*nr).type_ = t;
            for pb in [0, 1, 4, 256] {
                for fmt in [0, 1] {
                    assert_eq!(
                        print_buffered_and_take(&c, nc, pb, fmt),
                        print_buffered_and_take(&r, nr, pb, fmt),
                        "row 60: type {t:#x} pb={pb} fmt={fmt}"
                    );
                }
            }
            (*nc).type_ = cJSON_String;
            (*nr).type_ = cJSON_String;
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }
    }
}

// ===========================================================================
// parse rejections: rows 11, 30–35, 37–39, 43, 48–50, 52, 53, 65, 71, 73, 75,
// 76, 80, 82, 84–88
// ===========================================================================

/// Parses `text` with every entry point and requires identical results,
/// including the error position reported by `cJSON_GetErrorPtr`.
fn parse_reject(c: &Api, r: &Api, text: &[u8], row: &str) {
    let _guard = lock_global_state();
    let mut buf = text.to_vec();
    buf.push(0);
    buf.extend_from_slice(&[0u8; 8]);
    let p = buf.as_ptr() as *const c_char;
    unsafe {
        for variant in 0..6 {
            let mut end: *const c_char = std::ptr::null();
            let item = match variant {
                0 => (c.cJSON_Parse)(p),
                1 => (c.cJSON_ParseWithLength)(p, text.len() + 1),
                2 => (c.cJSON_ParseWithLength)(p, text.len()),
                3 => (c.cJSON_ParseWithOpts)(p, &mut end, 0),
                4 => (c.cJSON_ParseWithOpts)(p, &mut end, 1),
                _ => (c.cJSON_ParseWithLengthOpts)(p, text.len() + 1, &mut end, 1),
            };
            let a = (
                item.is_null(),
                snap(item),
                if end.is_null() {
                    None
                } else {
                    Some(end as isize - p as isize)
                },
                {
                    let e = (c.cJSON_GetErrorPtr)();
                    if e.is_null() {
                        None
                    } else {
                        Some(e as isize - p as isize)
                    }
                },
            );
            (c.cJSON_Delete)(item);

            let mut end: *const c_char = std::ptr::null();
            let item = match variant {
                0 => (r.cJSON_Parse)(p),
                1 => (r.cJSON_ParseWithLength)(p, text.len() + 1),
                2 => (r.cJSON_ParseWithLength)(p, text.len()),
                3 => (r.cJSON_ParseWithOpts)(p, &mut end, 0),
                4 => (r.cJSON_ParseWithOpts)(p, &mut end, 1),
                _ => (r.cJSON_ParseWithLengthOpts)(p, text.len() + 1, &mut end, 1),
            };
            let b = (
                item.is_null(),
                snap(item),
                if end.is_null() {
                    None
                } else {
                    Some(end as isize - p as isize)
                },
                {
                    let e = (r.cJSON_GetErrorPtr)();
                    if e.is_null() {
                        None
                    } else {
                        Some(e as isize - p as isize)
                    }
                },
            );
            (r.cJSON_Delete)(item);

            assert_eq!(
                a, b,
                "{row}: input {:?} variant {variant}\nC = {a:?}\nRust = {b:?}",
                String::from_utf8_lossy(text)
            );
        }
    }
}

/// Same as `parse_reject` but also asserts the C side really did reject.
fn parse_must_fail(c: &Api, r: &Api, text: &[u8], row: &str) {
    parse_reject(c, r, text, row);
    let _guard = lock_global_state();
    let mut buf = text.to_vec();
    buf.push(0);
    unsafe {
        let it = (c.cJSON_Parse)(buf.as_ptr() as *const c_char);
        assert!(
            it.is_null(),
            "{row}: expected the C parser to REJECT {:?}",
            String::from_utf8_lossy(text)
        );
        (c.cJSON_Delete)(it);
        let e = (c.cJSON_GetErrorPtr)();
        assert!(!e.is_null(), "{row}: C must set an error pointer");
    }
}

#[test]
fn err_parse_number_no_digits() {
    let (c, r) = both();
    for t in [
        &b"-"[..], &b"-e"[..], &b"-."[..], &b"-+"[..], &b"--1"[..], &b"-E"[..], &b"-e5"[..],
        &b"-."[..], &b"[-]"[..], &b"[-e]"[..], &b"{\"a\":-}"[..], &b"-,"[..],
    ] {
        parse_must_fail(&c, &r, t, "row 11");
    }
}

#[test]
fn err_parse_hex4_invalid() {
    let (c, r) = both();
    // row 30: parse_hex4 returns 0 for a non-hex digit, which is NOT treated as
    // an error — the codepoint becomes U+0000 and one NUL byte is emitted.
    for t in [
        &br#""\uZZZZ""#[..],
        &br#""\u00g0""#[..],
        &br#""\uG000""#[..],
        &br#""\u000/""#[..],
        &br#""\u:000""#[..],
        &br#""\u000:""#[..],
        &br#""\u@000""#[..],
        &br#""\u`000""#[..],
        &br#""x\uZZZZy""#[..],
    ] {
        parse_reject(&c, &r, t, "row 30");
    }
    // and the C side really does accept these
    let _guard = lock_global_state();
    unsafe {
        let b = Bytes::new(br#""\uZZZZ""#);
        let it = (c.cJSON_Parse)(b.as_ptr());
        assert!(
            !it.is_null(),
            "row 30: C accepts \\uZZZZ (parse_hex4 -> 0 -> U+0000)"
        );
        assert_eq!(cstr((*it).valuestring), Some(Vec::new()));
        (c.cJSON_Delete)(it);
    }
}

#[test]
fn err_parse_utf16_truncated() {
    let (c, r) = both();
    for t in [
        &br#""\u"#[..],
        &br#""\u1"#[..],
        &br#""\u12"#[..],
        &br#""\u123"#[..],
        &br#""\u""#[..],
        &br#""\u1""#[..],
        &br#""\u12""#[..],
        &br#""\u123""#[..],
        &br#""a\u12""#[..],
    ] {
        parse_must_fail(&c, &r, t, "row 31");
    }
}

#[test]
fn err_parse_utf16_lone_low_surrogate() {
    let (c, r) = both();
    for cp in [0xDC00u32, 0xDC01, 0xDD00, 0xDEAD, 0xDFFF] {
        let t = format!("\"\\u{cp:04x}\"").into_bytes();
        parse_must_fail(&c, &r, &t, "row 32");
        let t = format!("\"prefix\\u{cp:04X}suffix\"").into_bytes();
        parse_must_fail(&c, &r, &t, "row 32");
    }
}

#[test]
fn err_parse_utf16_truncated_pair() {
    let (c, r) = both();
    for t in [
        &br#""\ud800\u"#[..],
        &br#""\ud800\u1"#[..],
        &br#""\ud800\u12"#[..],
        &br#""\ud800\u123"#[..],
        &br#""\ud800\u12""#[..],
        &br#""\ud800\u123""#[..],
        &br#""\udbff\u1""#[..],
    ] {
        parse_must_fail(&c, &r, t, "row 33");
    }
}

#[test]
fn err_parse_utf16_missing_second() {
    let (c, r) = both();
    for t in [
        &br#""\ud800xxxxxx""#[..],
        &br#""\ud800\n1234""#[..],
        &br#""\ud800u12345""#[..],
        &br#""\ud800\\u1234""#[..],
        &br#""\ud800      ""#[..],
        &br#""\udbff123456""#[..],
    ] {
        parse_must_fail(&c, &r, t, "row 34");
    }
}

#[test]
fn err_parse_utf16_bad_second() {
    let (c, r) = both();
    for lo in [
        0x0000u32, 0x0041, 0x1234, 0xD7FF, 0xD800, 0xDBFF, 0xE000, 0xFFFF,
    ] {
        let t = format!("\"\\ud800\\u{lo:04x}\"").into_bytes();
        parse_must_fail(&c, &r, &t, "row 35");
    }
    // and a valid pair for contrast
    let _guard = lock_global_state();
    unsafe {
        let b = Bytes::new(br#""\ud800\udc00""#);
        let it = (c.cJSON_Parse)(b.as_ptr());
        assert!(!it.is_null(), "row 35 contrast: a valid pair must parse");
        (c.cJSON_Delete)(it);
    }
}

#[test]
fn err_parse_string_trailing_backslash() {
    let (c, r) = both();
    for t in [
        &b"\"abc\\"[..],
        &b"\"\\"[..],
        &b"[\"abc\\"[..],
        &b"{\"k\\"[..],
        &b"\"a\\\\\\"[..],
    ] {
        parse_must_fail(&c, &r, t, "row 38");
    }
}

#[test]
fn err_parse_string_unterminated() {
    let (c, r) = both();
    for t in [
        &b"\""[..],
        &b"\"abc"[..],
        &b"\"abc\\\"def"[..],
        &b"[\"abc]"[..],
        &b"{\"k:1}"[..],
        &b"\"\\n"[..],
    ] {
        parse_must_fail(&c, &r, t, "row 39");
    }
}

#[test]
fn err_parse_string_bad_escape() {
    let (c, r) = both();
    // every byte that is not a legal escape character
    let legal: &[u8] = b"bfnrt\"\\/u";
    for byte in 1u16..=255 {
        let byte = byte as u8;
        if legal.contains(&byte) {
            continue;
        }
        let mut t = vec![b'"', b'\\', byte, b'"'];
        parse_must_fail(&c, &r, &t, "row 43");
        t = vec![b'"', b'a', b'\\', byte, b'b', b'"'];
        parse_must_fail(&c, &r, &t, "row 43");
    }
}

#[test]
fn err_parse_with_opts_null() {
    let (c, r) = both();
    let _guard = lock_global_state();
    {
        // row 48: cJSON_ParseWithOpts(NULL, ...) returns before resetting
        // global_error, so a previously set error pointer survives.
        fn run(api: &Api) -> Vec<(bool, isize)> {
            let mut out = Vec::new();
            let bad = b"[1,x]\0";
            let p = bad.as_ptr() as *const c_char;
            unsafe {
                let it = (api.cJSON_Parse)(p);
                assert!(it.is_null());
                let e = (api.cJSON_GetErrorPtr)();
                out.push((e.is_null(), e as isize - p as isize));

                for rnt in [0, 1] {
                    let mut end: *const c_char = 0x1234 as *const c_char;
                    let it = (api.cJSON_ParseWithOpts)(std::ptr::null(), &mut end, rnt);
                    assert!(it.is_null(), "ParseWithOpts(NULL) must return NULL");
                    // `return_parse_end` must be left untouched
                    out.push((end == 0x1234 as *const c_char, 0));
                    let e = (api.cJSON_GetErrorPtr)();
                    out.push((e.is_null(), e as isize - p as isize));
                }
                let it = (api.cJSON_Parse)(std::ptr::null());
                assert!(it.is_null());
                let e = (api.cJSON_GetErrorPtr)();
                out.push((e.is_null(), e as isize - p as isize));
            }
            out
        }
        assert_eq!(run(&c), run(&r), "row 48: ParseWithOpts(NULL) state");
    }
}

#[test]
fn err_parse_with_length_opts_null() {
    let (c, r) = both();
    let _guard = lock_global_state();
    {
        // row 49: cJSON_ParseWithLengthOpts(NULL, ...) DOES reset global_error
        fn run(api: &Api) -> Vec<(bool, isize)> {
            let mut out = Vec::new();
            let bad = b"[1,x]\0";
            let p = bad.as_ptr() as *const c_char;
            unsafe {
                let it = (api.cJSON_Parse)(p);
                assert!(it.is_null());
                let e = (api.cJSON_GetErrorPtr)();
                out.push((e.is_null(), e as isize - p as isize));

                for len in [0usize, 1, 10, usize::MAX] {
                    for rnt in [0, 1] {
                        let mut end: *const c_char = 0x1234 as *const c_char;
                        let it =
                            (api.cJSON_ParseWithLengthOpts)(std::ptr::null(), len, &mut end, rnt);
                        assert!(it.is_null(), "ParseWithLengthOpts(NULL) must return NULL");
                        out.push((end == 0x1234 as *const c_char, 0));
                        let e = (api.cJSON_GetErrorPtr)();
                        out.push((e.is_null(), e as isize));
                    }
                }
            }
            out
        }
        assert_eq!(run(&c), run(&r), "row 49: ParseWithLengthOpts(NULL) state");
    }
}

#[test]
fn err_parse_zero_length() {
    let (c, r) = both();
    let _guard = lock_global_state();
    {
        // row 50: buffer_length == 0 with a non-NULL value
        fn run(api: &Api) -> Vec<(bool, isize, bool)> {
            let mut out = Vec::new();
            let doc = b"[1,2,3]\0";
            let p = doc.as_ptr() as *const c_char;
            unsafe {
                for rnt in [0, 1] {
                    for want_end in [false, true] {
                        let mut end: *const c_char = 0x1234 as *const c_char;
                        let it = (api.cJSON_ParseWithLengthOpts)(
                            p,
                            0,
                            if want_end { &mut end } else { std::ptr::null_mut() },
                            rnt,
                        );
                        let e = (api.cJSON_GetErrorPtr)();
                        out.push((
                            it.is_null(),
                            if e.is_null() { -1 } else { e as isize - p as isize },
                            end == p,
                        ));
                        (api.cJSON_Delete)(it);
                    }
                }
                let it = (api.cJSON_ParseWithLength)(p, 0);
                let e = (api.cJSON_GetErrorPtr)();
                out.push((
                    it.is_null(),
                    if e.is_null() { -1 } else { e as isize - p as isize },
                    true,
                ));
                (api.cJSON_Delete)(it);
            }
            out
        }
        let a = run(&c);
        let b = run(&r);
        assert!(a[0].0, "C: buffer_length == 0 must be rejected");
        assert_eq!(a[0].1, 0, "C: error position must be 0 for length 0");
        assert_eq!(a, b, "row 50: buffer_length == 0");
    }
}

#[test]
fn err_parse_error_ptr() {
    let (c, r) = both();
    // row 52: the error position is `offset` when `offset < length`, otherwise
    // `length - 1`.  Both branches, plus the reset on success.
    for t in [
        &b"[1,2,x]"[..],
        &b"[1,2"[..],
        &b"{"[..],
        &b"{\"a\""[..],
        &b"xyz"[..],
        &b""[..],
        &b" "[..],
        &b"-"[..],
        &b"\"abc"[..],
        &b"[[[[[[[[[[1"[..],
        &b"nul"[..],
    ] {
        parse_reject(&c, &r, t, "row 52");
    }
}

#[test]
fn err_parse_require_null_terminated() {
    let (c, r) = both();
    let _guard = lock_global_state();
    unsafe {
        // row 53: trailing garbage, and a length with no room for the NUL
        let cases: Vec<(&[u8], usize)> = vec![
            (b"[1] x", 6),
            (b"1 2", 4),
            (b"{} {}", 6),
            (b"null null", 10),
            (b"[1]", 3), // exactly the text: no NUL is visible
            (b"[1]", 4), // canonical: succeeds
            (b"[1] ", 5),
            (b"[1]\t\r\n", 7),
            (b"truex", 6),
            (b"1x", 3),
        ];
        for (text, len) in cases {
            let mut buf = text.to_vec();
            buf.push(0);
            buf.extend_from_slice(&[0u8; 8]);
            let p = buf.as_ptr() as *const c_char;
            for rnt in [0, 1, 2, -1] {
                let mut end_c: *const c_char = std::ptr::null();
                let ic = (c.cJSON_ParseWithLengthOpts)(p, len, &mut end_c, rnt);
                let ec = (c.cJSON_GetErrorPtr)();
                let a = (
                    ic.is_null(),
                    snap(ic),
                    end_c as isize - p as isize,
                    if ec.is_null() { -1 } else { ec as isize - p as isize },
                );
                (c.cJSON_Delete)(ic);

                let mut end_r: *const c_char = std::ptr::null();
                let ir = (r.cJSON_ParseWithLengthOpts)(p, len, &mut end_r, rnt);
                let er = (r.cJSON_GetErrorPtr)();
                let b = (
                    ir.is_null(),
                    snap(ir),
                    end_r as isize - p as isize,
                    if er.is_null() { -1 } else { er as isize - p as isize },
                );
                (r.cJSON_Delete)(ir);

                assert_eq!(
                    a, b,
                    "row 53: {:?} len={len} rnt={rnt}",
                    String::from_utf8_lossy(text)
                );
            }
        }
    }
}

#[test]
fn err_parse_value_no_match() {
    let (c, r) = both();
    for t in [
        &b""[..], &b"x"[..], &b"nul"[..], &b"tru"[..], &b"fals"[..], &b"NULL"[..],
        &b"TRUE"[..], &b"FALSE"[..], &b"None"[..], &b"+1"[..], &b".5"[..], &b"'a'"[..],
        &b"}"[..], &b"]"[..], &b","[..], &b":"[..], &b"@"[..], &b"#"[..], &b"\\"[..],
        &b"\x7f"[..], &b"\x80"[..], &b"\xff"[..], &b"undefined"[..], &b"NaN"[..], &b"Infinity"[..],
    ] {
        parse_must_fail(&c, &r, t, "row 65");
    }
    // "nulll" is NOT rejected: cJSON_Parse does not require a null terminator,
    // so it parses `null` and leaves the trailing "l" behind.
    for t in [&b"nulll"[..], &b"truex"[..], &b"falsey"[..], &b"1x"[..], &b"[1]x"[..]] {
        parse_reject(&c, &r, t, "row 65 (trailing garbage is accepted)");
    }
    // every single byte as a whole document
    for byte in 1u16..=255 {
        let t = vec![byte as u8];
        parse_reject(&c, &r, &t, "row 65 (single byte)");
    }
}

#[test]
fn err_parse_nesting_limit() {
    let (c, r) = both();
    // rows 71/80: exactly at, one below and one above CJSON_NESTING_LIMIT
    for depth in [998usize, 999, 1000, 1001, 1002] {
        let mut arr = vec![b'['; depth];
        arr.extend(std::iter::repeat(b']').take(depth));
        parse_reject(&c, &r, &arr, "row 71");

        let mut obj = Vec::new();
        for _ in 0..depth {
            obj.extend_from_slice(b"{\"a\":");
        }
        obj.push(b'1');
        for _ in 0..depth {
            obj.push(b'}');
        }
        parse_reject(&c, &r, &obj, "row 80");

        // mixed nesting reaching the limit
        let mut open = Vec::new();
        let mut close = Vec::new();
        for i in 0..depth {
            if i % 2 == 0 {
                open.push(b'[');
                close.insert(0, b']');
            } else {
                open.extend_from_slice(b"{\"k\":");
                close.insert(0, b'}');
            }
        }
        open.extend_from_slice(&close);
        parse_reject(&c, &r, &open, "rows 71/80 mixed");
    }
    // the C side must accept 1000 and reject 1001
    let _guard = lock_global_state();
    unsafe {
        for (depth, expect_ok) in [(1000usize, true), (1001usize, false)] {
            let mut arr = vec![b'['; depth];
            arr.extend(std::iter::repeat(b']').take(depth));
            arr.push(0);
            let it = (c.cJSON_Parse)(arr.as_ptr() as *const c_char);
            assert_eq!(
                !it.is_null(),
                expect_ok,
                "row 71: C nesting depth {depth} expectation"
            );
            (c.cJSON_Delete)(it);
        }
    }
}

/// Parses `text` with an explicit `buffer_length` that hides the NUL
/// terminator, then compares the result and the error offset.
fn parse_reject_with_length(c: &Api, r: &Api, text: &[u8], len: usize, row: &str) {
    let _guard = lock_global_state();
    let mut buf = text.to_vec();
    buf.push(0);
    buf.extend_from_slice(&[0u8; 8]);
    let p = buf.as_ptr() as *const c_char;
    unsafe {
        let ic = (c.cJSON_ParseWithLength)(p, len);
        let ec = (c.cJSON_GetErrorPtr)();
        let a = (
            ic.is_null(),
            snap(ic),
            if ec.is_null() { -1 } else { ec as isize - p as isize },
        );
        (c.cJSON_Delete)(ic);
        let ir = (r.cJSON_ParseWithLength)(p, len);
        let er = (r.cJSON_GetErrorPtr)();
        let b = (
            ir.is_null(),
            snap(ir),
            if er.is_null() { -1 } else { er as isize - p as isize },
        );
        (r.cJSON_Delete)(ir);
        assert!(
            a.0,
            "{row}: the C parser must reject {:?} with buffer_length {len}",
            String::from_utf8_lossy(text)
        );
        assert_eq!(
            a, b,
            "{row}: {:?} buffer_length {len}\nC = {a:?}\nRust = {b:?}",
            String::from_utf8_lossy(text)
        );
    }
}

#[test]
fn err_parse_array_truncated() {
    // row 73: the buffer is exhausted after '[' and the whitespace scan.
    let (c, r) = both();
    for (text, len) in [
        (&b"["[..], 1usize),
        (&b"[ "[..], 2),
        (&b"[  "[..], 3),
        (&b"[\t\r\n"[..], 4),
        (&b"[   \t"[..], 5),
    ] {
        parse_reject_with_length(&c, &r, text, len, "row 73");
    }
    // and with the NUL visible (a different path: parse_value sees '\0')
    for t in [&b"["[..], &b"[ "[..], &b"[\t"[..]] {
        parse_must_fail(&c, &r, t, "row 73");
    }
}

#[test]
fn err_parse_array_bad_element() {
    // row 75: the element's parse_value fails
    let (c, r) = both();
    for t in [
        &b"[,]"[..], &b"[1,]"[..], &b"[1,,2]"[..], &b"[x]"[..], &b"[,"[..], &b"[+1]"[..],
        &b"[.5]"[..], &b"[nul]"[..], &b"[1,\"a]"[..], &b"[[]"[..], &b"[[1,]]"[..],
        &b"[{\"a\":}]"[..],
    ] {
        parse_must_fail(&c, &r, t, "row 75");
    }
}

#[test]
fn err_parse_array_unclosed() {
    // row 76: no ']' after the last element
    let (c, r) = both();
    for t in [
        &b"[1"[..], &b"[1 "[..], &b"[1,2"[..], &b"[1}"[..], &b"[1 2]"[..], &b"[[1]"[..],
        &b"[1,2,3"[..], &b"[null"[..], &b"[\"a\""[..],
    ] {
        parse_must_fail(&c, &r, t, "row 76");
    }
}

#[test]
fn err_parse_object_truncated() {
    // row 82: the buffer is exhausted after '{' and the whitespace scan
    let (c, r) = both();
    for (text, len) in [
        (&b"{"[..], 1usize),
        (&b"{ "[..], 2),
        (&b"{  "[..], 3),
        (&b"{\t\r\n"[..], 4),
    ] {
        parse_reject_with_length(&c, &r, text, len, "row 82");
    }
    for t in [&b"{"[..], &b"{ "[..], &b"{\t"[..]] {
        parse_must_fail(&c, &r, t, "row 82");
    }
}

#[test]
fn err_parse_object_nothing_after_comma() {
    // row 84: cannot_access_at_index(input_buffer, 1)
    let (c, r) = both();
    for (text, len) in [
        (&b"{a"[..], 2usize),
        (&b"{\"a\":1,"[..], 7),
        (&b"{x"[..], 2),
        (&b"{\"a\":1,\"b\":2,"[..], 14),
    ] {
        parse_reject_with_length(&c, &r, text, len, "row 84");
    }
    for t in [&b"{\"a\":1,"[..], &b"{\"a\":1,\"b\":2,"[..]] {
        parse_must_fail(&c, &r, t, "row 84");
    }
}

#[test]
fn err_parse_object_bad_key() {
    // rows 37, 85: the key is not a string literal
    let (c, r) = both();
    for t in [
        &b"{x:1}"[..], &b"{1:2}"[..], &b"{'a':1}"[..], &b"{\"a:1}"[..], &b"{[]:1}"[..],
        &b"{null:1}"[..], &b"{true:1}"[..], &b"{:1}"[..], &b"{,}"[..],
        &b"{\"a\":1,x:2}"[..],
    ] {
        parse_must_fail(&c, &r, t, "rows 37/85");
    }
    // `{}}` is NOT a key error: the object closes and the extra `}` is trailing
    // garbage, which cJSON_Parse accepts.
    parse_reject(&c, &r, b"{}}", "row 85 (trailing garbage)");
}

#[test]
fn err_parse_object_missing_colon() {
    // row 86: no ':' after the key
    let (c, r) = both();
    for t in [
        &b"{\"a\" 1}"[..], &b"{\"a\"}"[..], &b"{\"a\",1}"[..], &b"{\"a\"=1}"[..],
        &b"{\"a\"\"b\"}"[..], &b"{\"a\":1,\"b\" 2}"[..], &b"{\"a\"["[..],
    ] {
        parse_must_fail(&c, &r, t, "row 86");
    }
}

#[test]
fn err_parse_object_bad_value() {
    // row 87: the value's parse_value fails
    let (c, r) = both();
    for t in [
        &b"{\"a\":}"[..], &b"{\"a\":x}"[..], &b"{\"a\":,}"[..], &b"{\"a\"::1}"[..],
        &b"{\"a\":+1}"[..], &b"{\"a\":-}"[..], &b"{\"a\":1,\"b\":}"[..],
        &b"{\"a\":\"unterminated}"[..],
    ] {
        parse_must_fail(&c, &r, t, "row 87");
    }
}

#[test]
fn err_parse_object_unclosed() {
    // row 88: no '}' after the last member
    let (c, r) = both();
    for t in [
        &b"{\"a\":1"[..], &b"{\"a\":1 "[..], &b"{\"a\":1,}"[..], &b"{\"a\":1]"[..],
        &b"{\"a\":1 \"b\":2}"[..], &b"{\"a\":{\"b\":1}"[..], &b"{\"a\":[1]"[..],
    ] {
        parse_must_fail(&c, &r, t, "row 88");
    }
}

// ===========================================================================
// rows 93–96 — array size / index accessors
// ===========================================================================
#[test]
fn err_get_array_size() {
    let (c, r) = both();
    unsafe {
        assert_eq!(
            (c.cJSON_GetArraySize)(std::ptr::null()),
            0,
            "C: GetArraySize(NULL) must be 0"
        );
        assert_eq!(
            (c.cJSON_GetArraySize)(std::ptr::null()),
            (r.cJSON_GetArraySize)(std::ptr::null()),
            "row 93: GetArraySize(NULL)"
        );
        // non-containers and empty containers
        for (name, spec) in one_of_each_type() {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            assert_eq!(
                (c.cJSON_GetArraySize)(bc.root),
                (r.cJSON_GetArraySize)(br.root),
                "row 93: GetArraySize on {name}"
            );
            bc.delete();
            br.delete();
        }
    }
}

#[test]
fn err_get_array_item() {
    let (c, r) = both();
    unsafe {
        // row 96: negative index
        for idx in [-1i32, -2, -100, i32::MIN] {
            let a = (c.cJSON_GetArrayItem)(std::ptr::null(), idx);
            let b = (r.cJSON_GetArrayItem)(std::ptr::null(), idx);
            assert!(a.is_null());
            assert_eq!(a.is_null(), b.is_null(), "rows 94/96: NULL array, idx={idx}");
        }
        // row 94: array == NULL with a valid index
        for idx in [0i32, 1, 100, i32::MAX] {
            let a = (c.cJSON_GetArrayItem)(std::ptr::null(), idx);
            let b = (r.cJSON_GetArrayItem)(std::ptr::null(), idx);
            assert!(a.is_null(), "C: GetArrayItem(NULL, {idx}) must be NULL");
            assert_eq!(a.is_null(), b.is_null(), "row 94: idx={idx}");
        }
        // rows 95, 96: out-of-range and negative indices on real containers
        for (name, spec) in one_of_each_type() {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            for idx in [-1i32, i32::MIN, 0, 1, 2, 5, 1000, i32::MAX] {
                let a = (c.cJSON_GetArrayItem)(bc.root, idx);
                let b = (r.cJSON_GetArrayItem)(br.root, idx);
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "rows 95/96: {name} idx={idx} nullness"
                );
                assert_eq!(snap(a), snap(b), "rows 95/96: {name} idx={idx} content");
            }
            bc.delete();
            br.delete();
        }
    }
}

// ===========================================================================
// rows 104–107 — add_item_to_array
// ===========================================================================
#[test]
fn err_add_item_to_array() {
    let (c, r) = both();
    unsafe {
        // row 105: array == NULL
        let nc = (c.cJSON_CreateNumber)(1.0);
        let nr = (r.cJSON_CreateNumber)(1.0);
        let a = (c.cJSON_AddItemToArray)(std::ptr::null_mut(), nc);
        let b = (r.cJSON_AddItemToArray)(std::ptr::null_mut(), nr);
        assert_eq!(a, 0, "C: AddItemToArray(NULL, item) must be 0");
        assert_eq!(a, b, "row 105");
        (c.cJSON_Delete)(nc);
        (r.cJSON_Delete)(nr);

        // row 104: item == NULL
        let ac = (c.cJSON_CreateArray)();
        let ar = (r.cJSON_CreateArray)();
        let a = (c.cJSON_AddItemToArray)(ac, std::ptr::null_mut());
        let b = (r.cJSON_AddItemToArray)(ar, std::ptr::null_mut());
        assert_eq!(a, 0, "C: AddItemToArray(array, NULL) must be 0");
        assert_eq!(a, b, "row 104");
        assert_eq!(snap(ac), snap(ar), "row 104: array unchanged");

        // row 106: array == item
        let a = (c.cJSON_AddItemToArray)(ac, ac);
        let b = (r.cJSON_AddItemToArray)(ar, ar);
        assert_eq!(a, 0, "C: self-append must be 0");
        assert_eq!(a, b, "row 106");
        assert_eq!(snap(ac), snap(ar), "row 106: array unchanged");

        // both NULL
        let a = (c.cJSON_AddItemToArray)(std::ptr::null_mut(), std::ptr::null_mut());
        let b = (r.cJSON_AddItemToArray)(std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(a, b, "rows 104/105: both NULL");

        (c.cJSON_Delete)(ac);
        (r.cJSON_Delete)(ar);
    }
}

#[test]
fn err_add_item_to_array_corrupt() {
    let (c, r) = both();
    unsafe {
        // row 107: array->child != NULL but child->prev == NULL — the C code
        // returns TRUE while silently NOT linking the new item.
        let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0)]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        (*(*bc.root).child).prev = std::ptr::null_mut();
        (*(*br.root).child).prev = std::ptr::null_mut();
        let nc = (c.cJSON_CreateNumber)(99.0);
        let nr = (r.cJSON_CreateNumber)(99.0);
        let a = (c.cJSON_AddItemToArray)(bc.root, nc);
        let b = (r.cJSON_AddItemToArray)(br.root, nr);
        assert_eq!(a, 1, "C: corrupted-list append still reports success");
        assert_eq!(a, b, "row 107: return value");
        assert_eq!(snap(bc.root), snap(br.root), "row 107: array unchanged");
        assert_eq!(snap(nc), snap(nr), "row 107: orphan item state");
        assert_eq!(
            print_and_take(&c, bc.root),
            print_and_take(&r, br.root),
            "row 107: printed array"
        );
        // the orphan was never linked, so it must be freed by hand
        (c.cJSON_Delete)(nc);
        (r.cJSON_Delete)(nr);
        // restore the back-pointer so Delete can walk the list
        let lastc = (c.cJSON_GetArrayItem)(bc.root, 1);
        let lastr = (r.cJSON_GetArrayItem)(br.root, 1);
        (*(*bc.root).child).prev = lastc;
        (*(*br.root).child).prev = lastr;
        bc.delete();
        br.delete();
    }
}

// ===========================================================================
// rows 108–111 — add_item_to_object
// ===========================================================================
#[test]
fn err_add_item_to_object() {
    let (c, r) = both();
    let key = Bytes::new(b"k");
    unsafe {
        for cs_variant in [false, true] {
            let add_c = if cs_variant {
                c.cJSON_AddItemToObjectCS
            } else {
                c.cJSON_AddItemToObject
            };
            let add_r = if cs_variant {
                r.cJSON_AddItemToObjectCS
            } else {
                r.cJSON_AddItemToObject
            };
            let tag = if cs_variant { "CS" } else { "" };

            // row 108: object == NULL
            let nc = (c.cJSON_CreateNumber)(1.0);
            let nr = (r.cJSON_CreateNumber)(1.0);
            let a = add_c(std::ptr::null_mut(), key.as_ptr(), nc);
            let b = add_r(std::ptr::null_mut(), key.as_ptr(), nr);
            assert_eq!(a, 0, "C: AddItemToObject{tag}(NULL, …) must be 0");
            assert_eq!(a, b, "row 108{tag}");
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);

            let oc = (c.cJSON_CreateObject)();
            let or = (r.cJSON_CreateObject)();

            // row 109: string == NULL
            let nc = (c.cJSON_CreateNumber)(1.0);
            let nr = (r.cJSON_CreateNumber)(1.0);
            let a = add_c(oc, std::ptr::null(), nc);
            let b = add_r(or, std::ptr::null(), nr);
            assert_eq!(a, 0, "C: AddItemToObject{tag}(obj, NULL, item) must be 0");
            assert_eq!(a, b, "row 109{tag}");
            assert_eq!(snap(oc), snap(or), "row 109{tag}: object unchanged");
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);

            // row 110: item == NULL
            let a = add_c(oc, key.as_ptr(), std::ptr::null_mut());
            let b = add_r(or, key.as_ptr(), std::ptr::null_mut());
            assert_eq!(a, 0, "C: AddItemToObject{tag}(obj, key, NULL) must be 0");
            assert_eq!(a, b, "row 110{tag}");
            assert_eq!(snap(oc), snap(or), "row 110{tag}: object unchanged");

            // row 111: object == item
            let a = add_c(oc, key.as_ptr(), oc);
            let b = add_r(or, key.as_ptr(), or);
            assert_eq!(a, 0, "C: AddItemToObject{tag}(obj, key, obj) must be 0");
            assert_eq!(a, b, "row 111{tag}");
            assert_eq!(snap(oc), snap(or), "row 111{tag}: object unchanged");

            (c.cJSON_Delete)(oc);
            (r.cJSON_Delete)(or);
        }
    }
}

// ===========================================================================
// rows 102, 113–116 — reference helpers
// ===========================================================================
#[test]
fn err_add_item_reference() {
    let (c, r) = both();
    let key = Bytes::new(b"ref");
    unsafe {
        // row 113: array == NULL
        let nc = (c.cJSON_CreateNumber)(1.0);
        let nr = (r.cJSON_CreateNumber)(1.0);
        let a = (c.cJSON_AddItemReferenceToArray)(std::ptr::null_mut(), nc);
        let b = (r.cJSON_AddItemReferenceToArray)(std::ptr::null_mut(), nr);
        assert_eq!(a, 0, "C: AddItemReferenceToArray(NULL, item) must be 0");
        assert_eq!(a, b, "row 113");

        // row 102: item == NULL ⇒ create_reference returns NULL
        let ac = (c.cJSON_CreateArray)();
        let ar = (r.cJSON_CreateArray)();
        let a = (c.cJSON_AddItemReferenceToArray)(ac, std::ptr::null_mut());
        let b = (r.cJSON_AddItemReferenceToArray)(ar, std::ptr::null_mut());
        assert_eq!(a, 0, "C: AddItemReferenceToArray(array, NULL) must be 0");
        assert_eq!(a, b, "row 102");
        assert_eq!(snap(ac), snap(ar), "row 102: array unchanged");

        // rows 114, 115, 116
        let oc = (c.cJSON_CreateObject)();
        let or = (r.cJSON_CreateObject)();
        let a = (c.cJSON_AddItemReferenceToObject)(std::ptr::null_mut(), key.as_ptr(), nc);
        let b = (r.cJSON_AddItemReferenceToObject)(std::ptr::null_mut(), key.as_ptr(), nr);
        assert_eq!(a, 0);
        assert_eq!(a, b, "row 114: object == NULL");

        let a = (c.cJSON_AddItemReferenceToObject)(oc, std::ptr::null(), nc);
        let b = (r.cJSON_AddItemReferenceToObject)(or, std::ptr::null(), nr);
        assert_eq!(a, 0);
        assert_eq!(a, b, "row 115: string == NULL");
        assert_eq!(snap(oc), snap(or), "row 115: object unchanged");

        let a = (c.cJSON_AddItemReferenceToObject)(oc, key.as_ptr(), std::ptr::null_mut());
        let b = (r.cJSON_AddItemReferenceToObject)(or, key.as_ptr(), std::ptr::null_mut());
        assert_eq!(a, 0);
        assert_eq!(a, b, "row 116: item == NULL");
        assert_eq!(snap(oc), snap(or), "row 116: object unchanged");

        (c.cJSON_Delete)(ac);
        (r.cJSON_Delete)(ar);
        (c.cJSON_Delete)(oc);
        (r.cJSON_Delete)(or);
        (c.cJSON_Delete)(nc);
        (r.cJSON_Delete)(nr);
    }
}

// ===========================================================================
// rows 117–127 — the nine cJSON_Add*ToObject helpers
// ===========================================================================
#[test]
fn err_add_x_to_object() {
    let (c, r) = both();
    let key = Bytes::new(b"name");
    let val = Bytes::new(b"value");
    unsafe {
        // helper index → (C fn, Rust fn) invoked with (object, name)
        macro_rules! probe {
            ($label:expr, $obj_c:expr, $obj_r:expr, $name:expr, $call_c:expr, $call_r:expr) => {{
                let pc = $call_c($obj_c, $name);
                let pr = $call_r($obj_r, $name);
                assert_eq!(
                    pc.is_null(),
                    pr.is_null(),
                    concat!("rows 117-127: ", $label, " nullness")
                );
                assert_eq!(snap($obj_c), snap($obj_r), concat!($label, ": object state"));
                (pc.is_null(), pr.is_null())
            }};
        }

        for (obj_null, name_null) in [(true, false), (false, true), (true, true)] {
            let oc = if obj_null {
                std::ptr::null_mut()
            } else {
                (c.cJSON_CreateObject)()
            };
            let or = if obj_null {
                std::ptr::null_mut()
            } else {
                (r.cJSON_CreateObject)()
            };
            let nm = if name_null {
                std::ptr::null()
            } else {
                key.as_ptr()
            };

            let (a, b) = probe!("AddNullToObject", oc, or, nm, c.cJSON_AddNullToObject, r.cJSON_AddNullToObject);
            assert!(a, "C: AddNullToObject must fail here");
            assert_eq!(a, b);
            probe!("AddTrueToObject", oc, or, nm, c.cJSON_AddTrueToObject, r.cJSON_AddTrueToObject);
            probe!("AddFalseToObject", oc, or, nm, c.cJSON_AddFalseToObject, r.cJSON_AddFalseToObject);
            probe!("AddObjectToObject", oc, or, nm, c.cJSON_AddObjectToObject, r.cJSON_AddObjectToObject);
            probe!("AddArrayToObject", oc, or, nm, c.cJSON_AddArrayToObject, r.cJSON_AddArrayToObject);

            for bl in [0, 1, 2, -1, i32::MIN] {
                let pc = (c.cJSON_AddBoolToObject)(oc, nm, bl);
                let pr = (r.cJSON_AddBoolToObject)(or, nm, bl);
                assert_eq!(pc.is_null(), pr.is_null(), "row 120: AddBoolToObject({bl})");
                assert_eq!(snap(oc), snap(or), "row 120: object state");
            }
            for d in [0.0, 1.5, f64::NAN, f64::INFINITY] {
                let pc = (c.cJSON_AddNumberToObject)(oc, nm, d);
                let pr = (r.cJSON_AddNumberToObject)(or, nm, d);
                assert_eq!(pc.is_null(), pr.is_null(), "row 121: AddNumberToObject({d:?})");
                assert_eq!(snap(oc), snap(or), "row 121: object state");
            }
            for sv in [val.as_ptr(), std::ptr::null()] {
                let pc = (c.cJSON_AddStringToObject)(oc, nm, sv);
                let pr = (r.cJSON_AddStringToObject)(or, nm, sv);
                assert_eq!(
                    pc.is_null(),
                    pr.is_null(),
                    "rows 122/123: AddStringToObject(value_null={})",
                    sv.is_null()
                );
                assert_eq!(snap(oc), snap(or), "rows 122/123: object state");
                let pc = (c.cJSON_AddRawToObject)(oc, nm, sv);
                let pr = (r.cJSON_AddRawToObject)(or, nm, sv);
                assert_eq!(
                    pc.is_null(),
                    pr.is_null(),
                    "rows 124/125: AddRawToObject(value_null={})",
                    sv.is_null()
                );
                assert_eq!(snap(oc), snap(or), "rows 124/125: object state");
            }
            (c.cJSON_Delete)(oc);
            (r.cJSON_Delete)(or);
        }

        // rows 123, 125 on a VALID object: only the payload is NULL
        let oc = (c.cJSON_CreateObject)();
        let or = (r.cJSON_CreateObject)();
        let pc = (c.cJSON_AddStringToObject)(oc, key.as_ptr(), std::ptr::null());
        let pr = (r.cJSON_AddStringToObject)(or, key.as_ptr(), std::ptr::null());
        assert!(pc.is_null(), "C: AddStringToObject(obj, name, NULL) must be NULL");
        assert_eq!(pc.is_null(), pr.is_null(), "row 123");
        assert_eq!(snap(oc), snap(or), "row 123: object unchanged");
        let pc = (c.cJSON_AddRawToObject)(oc, key.as_ptr(), std::ptr::null());
        let pr = (r.cJSON_AddRawToObject)(or, key.as_ptr(), std::ptr::null());
        assert!(pc.is_null(), "C: AddRawToObject(obj, name, NULL) must be NULL");
        assert_eq!(pc.is_null(), pr.is_null(), "row 125");
        assert_eq!(snap(oc), snap(or), "row 125: object unchanged");
        assert_eq!(print_and_take(&c, oc), print_and_take(&r, or));
        (c.cJSON_Delete)(oc);
        (r.cJSON_Delete)(or);
    }
}

// ===========================================================================
// rows 128–134 — detach / delete
// ===========================================================================
#[test]
fn err_detach_via_pointer() {
    let (c, r) = both();
    unsafe {
        let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0), Spec::Num(3.0)]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        let ic = (c.cJSON_GetArrayItem)(bc.root, 1);
        let ir = (r.cJSON_GetArrayItem)(br.root, 1);

        // row 128: parent == NULL
        let a = (c.cJSON_DetachItemViaPointer)(std::ptr::null_mut(), ic);
        let b = (r.cJSON_DetachItemViaPointer)(std::ptr::null_mut(), ir);
        assert!(a.is_null(), "C: DetachItemViaPointer(NULL, item) must be NULL");
        assert_eq!(a.is_null(), b.is_null(), "row 128");

        // row 129: item == NULL
        let a = (c.cJSON_DetachItemViaPointer)(bc.root, std::ptr::null_mut());
        let b = (r.cJSON_DetachItemViaPointer)(br.root, std::ptr::null_mut());
        assert!(a.is_null(), "C: DetachItemViaPointer(parent, NULL) must be NULL");
        assert_eq!(a.is_null(), b.is_null(), "row 129");
        assert_eq!(snap(bc.root), snap(br.root), "row 129: parent unchanged");

        // row 130: item not in this parent's list (prev == NULL and != child)
        let foreign_c = (c.cJSON_CreateNumber)(9.0);
        let foreign_r = (r.cJSON_CreateNumber)(9.0);
        let a = (c.cJSON_DetachItemViaPointer)(bc.root, foreign_c);
        let b = (r.cJSON_DetachItemViaPointer)(br.root, foreign_r);
        assert!(a.is_null(), "C: detaching a foreign item must be NULL");
        assert_eq!(a.is_null(), b.is_null(), "row 130");
        assert_eq!(snap(bc.root), snap(br.root), "row 130: parent unchanged");
        (c.cJSON_Delete)(foreign_c);
        (r.cJSON_Delete)(foreign_r);

        // both NULL
        let a = (c.cJSON_DetachItemViaPointer)(std::ptr::null_mut(), std::ptr::null_mut());
        let b = (r.cJSON_DetachItemViaPointer)(std::ptr::null_mut(), std::ptr::null_mut());
        assert_eq!(a.is_null(), b.is_null(), "rows 128/129: both NULL");

        // an empty parent
        let ec = (c.cJSON_CreateArray)();
        let er = (r.cJSON_CreateArray)();
        let fc = (c.cJSON_CreateNumber)(1.0);
        let fr = (r.cJSON_CreateNumber)(1.0);
        let a = (c.cJSON_DetachItemViaPointer)(ec, fc);
        let b = (r.cJSON_DetachItemViaPointer)(er, fr);
        assert!(a.is_null());
        assert_eq!(a.is_null(), b.is_null(), "row 130: empty parent");
        (c.cJSON_Delete)(ec);
        (r.cJSON_Delete)(er);
        (c.cJSON_Delete)(fc);
        (r.cJSON_Delete)(fr);

        bc.delete();
        br.delete();
    }
}

#[test]
fn err_detach_from_array() {
    let (c, r) = both();
    unsafe {
        let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0)]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        // row 131: negative index
        for which in [-1i32, -5, i32::MIN] {
            let a = (c.cJSON_DetachItemFromArray)(bc.root, which);
            let b = (r.cJSON_DetachItemFromArray)(br.root, which);
            assert!(a.is_null(), "C: DetachItemFromArray(_, {which}) must be NULL");
            assert_eq!(a.is_null(), b.is_null(), "row 131: which={which}");
            assert_eq!(snap(bc.root), snap(br.root), "row 131: unchanged");
        }
        // row 132: index >= size
        for which in [2i32, 3, 1000, i32::MAX] {
            let a = (c.cJSON_DetachItemFromArray)(bc.root, which);
            let b = (r.cJSON_DetachItemFromArray)(br.root, which);
            assert!(a.is_null(), "C: DetachItemFromArray(_, {which}) must be NULL");
            assert_eq!(a.is_null(), b.is_null(), "row 132: which={which}");
            assert_eq!(snap(bc.root), snap(br.root), "row 132: unchanged");
        }
        // NULL array
        for which in [-1i32, 0, 5] {
            let a = (c.cJSON_DetachItemFromArray)(std::ptr::null_mut(), which);
            let b = (r.cJSON_DetachItemFromArray)(std::ptr::null_mut(), which);
            assert_eq!(a.is_null(), b.is_null(), "rows 131/132: NULL array which={which}");
        }
        bc.delete();
        br.delete();
    }
}

#[test]
fn err_detach_from_object() {
    let (c, r) = both();
    let key = Bytes::new(b"absent");
    unsafe {
        for (name, spec) in one_of_each_type() {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            // row 133: absent key
            let a = (c.cJSON_DetachItemFromObject)(bc.root, key.as_ptr());
            let b = (r.cJSON_DetachItemFromObject)(br.root, key.as_ptr());
            assert!(a.is_null(), "C: detaching an absent key must be NULL ({name})");
            assert_eq!(a.is_null(), b.is_null(), "row 133: {name}");
            let a = (c.cJSON_DetachItemFromObjectCaseSensitive)(bc.root, key.as_ptr());
            let b = (r.cJSON_DetachItemFromObjectCaseSensitive)(br.root, key.as_ptr());
            assert_eq!(a.is_null(), b.is_null(), "row 133 CS: {name}");
            // NULL key
            let a = (c.cJSON_DetachItemFromObject)(bc.root, std::ptr::null());
            let b = (r.cJSON_DetachItemFromObject)(br.root, std::ptr::null());
            assert_eq!(a.is_null(), b.is_null(), "row 133: NULL key on {name}");
            let a = (c.cJSON_DetachItemFromObjectCaseSensitive)(bc.root, std::ptr::null());
            let b = (r.cJSON_DetachItemFromObjectCaseSensitive)(br.root, std::ptr::null());
            assert_eq!(a.is_null(), b.is_null(), "row 133 CS: NULL key on {name}");
            assert_eq!(snap(bc.root), snap(br.root), "row 133: unchanged ({name})");
            bc.delete();
            br.delete();
        }
        // NULL object
        for f in 0..2 {
            let a = if f == 0 {
                (c.cJSON_DetachItemFromObject)(std::ptr::null_mut(), key.as_ptr())
            } else {
                (c.cJSON_DetachItemFromObjectCaseSensitive)(std::ptr::null_mut(), key.as_ptr())
            };
            let b = if f == 0 {
                (r.cJSON_DetachItemFromObject)(std::ptr::null_mut(), key.as_ptr())
            } else {
                (r.cJSON_DetachItemFromObjectCaseSensitive)(std::ptr::null_mut(), key.as_ptr())
            };
            assert_eq!(a.is_null(), b.is_null(), "row 133: NULL object variant {f}");
        }
    }
}

#[test]
fn err_delete_item_from() {
    let (c, r) = both();
    let key = Bytes::new(b"absent");
    unsafe {
        // row 134: the detach returned NULL, so cJSON_Delete(NULL) is a no-op
        (c.cJSON_DeleteItemFromArray)(std::ptr::null_mut(), 0);
        (r.cJSON_DeleteItemFromArray)(std::ptr::null_mut(), 0);
        (c.cJSON_DeleteItemFromObject)(std::ptr::null_mut(), key.as_ptr());
        (r.cJSON_DeleteItemFromObject)(std::ptr::null_mut(), key.as_ptr());
        (c.cJSON_DeleteItemFromObjectCaseSensitive)(std::ptr::null_mut(), key.as_ptr());
        (r.cJSON_DeleteItemFromObjectCaseSensitive)(std::ptr::null_mut(), key.as_ptr());
        (c.cJSON_Delete)(std::ptr::null_mut());
        (r.cJSON_Delete)(std::ptr::null_mut());

        for (name, spec) in one_of_each_type() {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            for which in [-1i32, 0, 1, 100, i32::MIN, i32::MAX] {
                (c.cJSON_DeleteItemFromArray)(bc.root, which);
                (r.cJSON_DeleteItemFromArray)(br.root, which);
                assert_eq!(
                    snap(bc.root),
                    snap(br.root),
                    "row 134: DeleteItemFromArray({which}) on {name}"
                );
            }
            (c.cJSON_DeleteItemFromObject)(bc.root, key.as_ptr());
            (r.cJSON_DeleteItemFromObject)(br.root, key.as_ptr());
            (c.cJSON_DeleteItemFromObject)(bc.root, std::ptr::null());
            (r.cJSON_DeleteItemFromObject)(br.root, std::ptr::null());
            (c.cJSON_DeleteItemFromObjectCaseSensitive)(bc.root, key.as_ptr());
            (r.cJSON_DeleteItemFromObjectCaseSensitive)(br.root, key.as_ptr());
            (c.cJSON_DeleteItemFromObjectCaseSensitive)(bc.root, std::ptr::null());
            (r.cJSON_DeleteItemFromObjectCaseSensitive)(br.root, std::ptr::null());
            assert_eq!(snap(bc.root), snap(br.root), "row 134: object deletes on {name}");
            bc.delete();
            br.delete();
        }
    }
}

// ===========================================================================
// rows 135–138 — cJSON_InsertItemInArray
// ===========================================================================
#[test]
fn err_insert_item_in_array() {
    let (c, r) = both();
    unsafe {
        let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0)]);
        // row 135: which < 0
        for which in [-1i32, -9, i32::MIN] {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            let nc = (c.cJSON_CreateNumber)(7.0);
            let nr = (r.cJSON_CreateNumber)(7.0);
            let a = (c.cJSON_InsertItemInArray)(bc.root, which, nc);
            let b = (r.cJSON_InsertItemInArray)(br.root, which, nr);
            assert_eq!(a, 0, "C: InsertItemInArray(_, {which}, _) must be 0");
            assert_eq!(a, b, "row 135: which={which}");
            assert_eq!(snap(bc.root), snap(br.root), "row 135: unchanged");
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
            bc.delete();
            br.delete();
        }
        // row 136: newitem == NULL
        for which in [-1i32, 0, 1, 5] {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            let a = (c.cJSON_InsertItemInArray)(bc.root, which, std::ptr::null_mut());
            let b = (r.cJSON_InsertItemInArray)(br.root, which, std::ptr::null_mut());
            assert_eq!(a, 0, "C: InsertItemInArray(_, _, NULL) must be 0");
            assert_eq!(a, b, "row 136: which={which}");
            assert_eq!(snap(bc.root), snap(br.root), "row 136: unchanged");
            bc.delete();
            br.delete();
        }
        // row 137: which >= size falls back to add_item_to_array; with a NULL
        // array that is a rejection, otherwise it appends.
        for which in [2i32, 3, 100, i32::MAX] {
            let nc = (c.cJSON_CreateNumber)(7.0);
            let nr = (r.cJSON_CreateNumber)(7.0);
            let a = (c.cJSON_InsertItemInArray)(std::ptr::null_mut(), which, nc);
            let b = (r.cJSON_InsertItemInArray)(std::ptr::null_mut(), which, nr);
            assert_eq!(a, 0, "C: InsertItemInArray(NULL, {which}, item) must be 0");
            assert_eq!(a, b, "row 137: NULL array which={which}");
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);

            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            let nc = (c.cJSON_CreateNumber)(7.0);
            let nr = (r.cJSON_CreateNumber)(7.0);
            let a = (c.cJSON_InsertItemInArray)(bc.root, which, nc);
            let b = (r.cJSON_InsertItemInArray)(br.root, which, nr);
            assert_eq!(a, b, "row 137: append fallback which={which}");
            assert_eq!(snap(bc.root), snap(br.root), "row 137: append result");
            assert_eq!(print_and_take(&c, bc.root), print_and_take(&r, br.root));
            if a == 0 {
                (c.cJSON_Delete)(nc);
                (r.cJSON_Delete)(nr);
            }
            bc.delete();
            br.delete();
        }
    }
}

#[test]
fn err_insert_item_corrupt() {
    let (c, r) = both();
    unsafe {
        // row 138: after_inserted is not the child and has prev == NULL
        let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0), Spec::Num(3.0)]);
        for which in [1i32, 2] {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            let tc = (c.cJSON_GetArrayItem)(bc.root, which);
            let tr = (r.cJSON_GetArrayItem)(br.root, which);
            let saved_c = (*tc).prev;
            let saved_r = (*tr).prev;
            (*tc).prev = std::ptr::null_mut();
            (*tr).prev = std::ptr::null_mut();
            let nc = (c.cJSON_CreateNumber)(99.0);
            let nr = (r.cJSON_CreateNumber)(99.0);
            let a = (c.cJSON_InsertItemInArray)(bc.root, which, nc);
            let b = (r.cJSON_InsertItemInArray)(br.root, which, nr);
            assert_eq!(a, 0, "C: corrupted after_inserted must be rejected");
            assert_eq!(a, b, "row 138: which={which}");
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
            (*tc).prev = saved_c;
            (*tr).prev = saved_r;
            assert_eq!(snap(bc.root), snap(br.root), "row 138: unchanged");
            bc.delete();
            br.delete();
        }
    }
}

// ===========================================================================
// rows 139–149 — replace
// ===========================================================================
#[test]
fn err_replace_via_pointer() {
    let (c, r) = both();
    unsafe {
        let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0)]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        let ic = (c.cJSON_GetArrayItem)(bc.root, 0);
        let ir = (r.cJSON_GetArrayItem)(br.root, 0);

        // row 139: parent == NULL
        let nc = (c.cJSON_CreateNumber)(5.0);
        let nr = (r.cJSON_CreateNumber)(5.0);
        let a = (c.cJSON_ReplaceItemViaPointer)(std::ptr::null_mut(), ic, nc);
        let b = (r.cJSON_ReplaceItemViaPointer)(std::ptr::null_mut(), ir, nr);
        assert_eq!(a, 0, "C: ReplaceItemViaPointer(NULL, …) must be 0");
        assert_eq!(a, b, "row 139");

        // row 141: replacement == NULL
        let a = (c.cJSON_ReplaceItemViaPointer)(bc.root, ic, std::ptr::null_mut());
        let b = (r.cJSON_ReplaceItemViaPointer)(br.root, ir, std::ptr::null_mut());
        assert_eq!(a, 0, "C: replacement == NULL must be 0");
        assert_eq!(a, b, "row 141");
        assert_eq!(snap(bc.root), snap(br.root), "row 141: unchanged");

        // row 142: item == NULL
        let a = (c.cJSON_ReplaceItemViaPointer)(bc.root, std::ptr::null_mut(), nc);
        let b = (r.cJSON_ReplaceItemViaPointer)(br.root, std::ptr::null_mut(), nr);
        assert_eq!(a, 0, "C: item == NULL must be 0");
        assert_eq!(a, b, "row 142");
        assert_eq!(snap(bc.root), snap(br.root), "row 142: unchanged");

        // row 140: parent->child == NULL (empty container / a leaf as parent)
        for empty_spec in [
            Spec::Arr(vec![]),
            Spec::Obj(vec![]),
            Spec::Num(1.0),
            Spec::Str(b"s".to_vec()),
            Spec::Null,
        ] {
            let ec = build(&c, &empty_spec);
            let er = build(&r, &empty_spec);
            let a = (c.cJSON_ReplaceItemViaPointer)(ec.root, ic, nc);
            let b = (r.cJSON_ReplaceItemViaPointer)(er.root, ir, nr);
            assert_eq!(a, 0, "C: parent->child == NULL must be 0 ({empty_spec:?})");
            assert_eq!(a, b, "row 140: {empty_spec:?}");
            ec.delete();
            er.delete();
        }
        (c.cJSON_Delete)(nc);
        (r.cJSON_Delete)(nr);

        // row 143: replacement == item returns true without changing anything
        let a = (c.cJSON_ReplaceItemViaPointer)(bc.root, ic, ic);
        let b = (r.cJSON_ReplaceItemViaPointer)(br.root, ir, ir);
        assert_eq!(a, 1, "C: self-replace must return 1");
        assert_eq!(a, b, "row 143");
        assert_eq!(snap(bc.root), snap(br.root), "row 143: unchanged");
        assert_eq!(print_and_take(&c, bc.root), print_and_take(&r, br.root));

        bc.delete();
        br.delete();
    }
}

#[test]
fn err_replace_in_array() {
    let (c, r) = both();
    unsafe {
        let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0)]);
        for which in [-1i32, -7, i32::MIN, 2, 3, 100, i32::MAX] {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            let nc = (c.cJSON_CreateNumber)(5.0);
            let nr = (r.cJSON_CreateNumber)(5.0);
            let a = (c.cJSON_ReplaceItemInArray)(bc.root, which, nc);
            let b = (r.cJSON_ReplaceItemInArray)(br.root, which, nr);
            assert_eq!(a, 0, "C: ReplaceItemInArray(_, {which}, _) must be 0");
            assert_eq!(a, b, "rows 144/145: which={which}");
            assert_eq!(snap(bc.root), snap(br.root), "rows 144/145: unchanged");
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
            bc.delete();
            br.delete();
        }
        // NULL array
        for which in [-1i32, 0, 5] {
            let nc = (c.cJSON_CreateNumber)(5.0);
            let nr = (r.cJSON_CreateNumber)(5.0);
            let a = (c.cJSON_ReplaceItemInArray)(std::ptr::null_mut(), which, nc);
            let b = (r.cJSON_ReplaceItemInArray)(std::ptr::null_mut(), which, nr);
            assert_eq!(a, b, "rows 144/145: NULL array which={which}");
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }
    }
}

#[test]
fn err_replace_in_object() {
    let (c, r) = both();
    let key = Bytes::new(b"absent_key");
    unsafe {
        for cs_flag in [false, true] {
            let rep_c = if cs_flag {
                c.cJSON_ReplaceItemInObjectCaseSensitive
            } else {
                c.cJSON_ReplaceItemInObject
            };
            let rep_r = if cs_flag {
                r.cJSON_ReplaceItemInObjectCaseSensitive
            } else {
                r.cJSON_ReplaceItemInObject
            };
            let tag = if cs_flag { "CS" } else { "" };

            let spec = Spec::Obj(vec![(b"k".to_vec(), Spec::Num(1.0))]);
            let bc = build(&c, &spec);
            let br = build(&r, &spec);

            // row 146: replacement == NULL
            let a = rep_c(bc.root, key.as_ptr(), std::ptr::null_mut());
            let b = rep_r(br.root, key.as_ptr(), std::ptr::null_mut());
            assert_eq!(a, 0, "C: ReplaceItemInObject{tag} with NULL replacement must be 0");
            assert_eq!(a, b, "row 146{tag}");
            assert_eq!(snap(bc.root), snap(br.root), "row 146{tag}: unchanged");

            // row 147: string == NULL
            let nc = (c.cJSON_CreateNumber)(2.0);
            let nr = (r.cJSON_CreateNumber)(2.0);
            let a = rep_c(bc.root, std::ptr::null(), nc);
            let b = rep_r(br.root, std::ptr::null(), nr);
            assert_eq!(a, 0, "C: ReplaceItemInObject{tag} with NULL key must be 0");
            assert_eq!(a, b, "row 147{tag}");
            assert_eq!(snap(nc), snap(nr), "row 147{tag}: replacement untouched");
            assert_eq!(snap(bc.root), snap(br.root), "row 147{tag}: unchanged");

            // row 149: key absent — the replacement's `string` HAS already been
            // rewritten and `cJSON_StringIsConst` cleared, even though the call
            // fails.  That side effect must match.
            let a = rep_c(bc.root, key.as_ptr(), nc);
            let b = rep_r(br.root, key.as_ptr(), nr);
            assert_eq!(a, 0, "C: absent key must be 0");
            assert_eq!(a, b, "row 149{tag}");
            assert_eq!(
                snap(nc),
                snap(nr),
                "row 149{tag}: replacement's key side effect"
            );
            assert_eq!(cstr((*nc).string), Some(b"absent_key".to_vec()));
            assert_eq!(snap(bc.root), snap(br.root), "row 149{tag}: object unchanged");
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);

            // row 149 with a StringIsConst replacement: the flag must be cleared
            let oc2 = (c.cJSON_CreateObject)();
            let or2 = (r.cJSON_CreateObject)();
            let nc = (c.cJSON_CreateNumber)(3.0);
            let nr = (r.cJSON_CreateNumber)(3.0);
            let ck = Bytes::new(b"const");
            (c.cJSON_AddItemToObjectCS)(oc2, ck.as_ptr(), nc);
            (r.cJSON_AddItemToObjectCS)(or2, ck.as_ptr(), nr);
            let dc = (c.cJSON_DetachItemViaPointer)(oc2, nc);
            let dr = (r.cJSON_DetachItemViaPointer)(or2, nr);
            let a = rep_c(bc.root, key.as_ptr(), dc);
            let b = rep_r(br.root, key.as_ptr(), dr);
            assert_eq!(a, b, "row 149{tag}: const-key replacement rc");
            assert_eq!(snap(dc), snap(dr), "row 149{tag}: const-key side effect");
            (c.cJSON_Delete)(dc);
            (r.cJSON_Delete)(dr);
            (c.cJSON_Delete)(oc2);
            (r.cJSON_Delete)(or2);

            // NULL object
            let nc = (c.cJSON_CreateNumber)(4.0);
            let nr = (r.cJSON_CreateNumber)(4.0);
            let a = rep_c(std::ptr::null_mut(), key.as_ptr(), nc);
            let b = rep_r(std::ptr::null_mut(), key.as_ptr(), nr);
            assert_eq!(a, 0);
            assert_eq!(a, b, "row 149{tag}: NULL object");
            assert_eq!(snap(nc), snap(nr), "row 149{tag}: NULL object side effect");
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);

            bc.delete();
            br.delete();
        }
    }
}

// ===========================================================================
// rows 152–163 — cJSON_Create*Array rejections
// ===========================================================================
#[test]
fn err_create_arrays() {
    let (c, r) = both();
    let ints: Vec<c_int> = vec![1, 2, 3];
    let floats: Vec<f32> = vec![1.0, 2.0, 3.0];
    let doubles: Vec<f64> = vec![1.0, 2.0, 3.0];
    let sb = [Bytes::new(b"a"), Bytes::new(b"b"), Bytes::new(b"c")];
    let strs: Vec<*const c_char> = sb.iter().map(|b| b.as_ptr()).collect();
    unsafe {
        // rows 152, 155, 158, 161: count < 0
        for count in [-1i32, -2, -1000, i32::MIN] {
            for which in 0..4 {
                let a = match which {
                    0 => (c.cJSON_CreateIntArray)(ints.as_ptr(), count),
                    1 => (c.cJSON_CreateFloatArray)(floats.as_ptr(), count),
                    2 => (c.cJSON_CreateDoubleArray)(doubles.as_ptr(), count),
                    _ => (c.cJSON_CreateStringArray)(strs.as_ptr(), count),
                };
                let b = match which {
                    0 => (r.cJSON_CreateIntArray)(ints.as_ptr(), count),
                    1 => (r.cJSON_CreateFloatArray)(floats.as_ptr(), count),
                    2 => (r.cJSON_CreateDoubleArray)(doubles.as_ptr(), count),
                    _ => (r.cJSON_CreateStringArray)(strs.as_ptr(), count),
                };
                assert!(a.is_null(), "C: Create*Array(count={count}) must be NULL");
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "rows 152/155/158/161: which={which} count={count}"
                );
            }
        }
        // rows 153, 156, 159, 162: numbers/strings == NULL
        for count in [-1i32, 0, 1, 3, 1000] {
            let a = (c.cJSON_CreateIntArray)(std::ptr::null(), count);
            let b = (r.cJSON_CreateIntArray)(std::ptr::null(), count);
            assert!(a.is_null(), "C: CreateIntArray(NULL, {count}) must be NULL");
            assert_eq!(a.is_null(), b.is_null(), "row 153: count={count}");
            let a = (c.cJSON_CreateFloatArray)(std::ptr::null(), count);
            let b = (r.cJSON_CreateFloatArray)(std::ptr::null(), count);
            assert_eq!(a.is_null(), b.is_null(), "row 156: count={count}");
            let a = (c.cJSON_CreateDoubleArray)(std::ptr::null(), count);
            let b = (r.cJSON_CreateDoubleArray)(std::ptr::null(), count);
            assert_eq!(a.is_null(), b.is_null(), "row 159: count={count}");
            let a = (c.cJSON_CreateStringArray)(std::ptr::null(), count);
            let b = (r.cJSON_CreateStringArray)(std::ptr::null(), count);
            assert_eq!(a.is_null(), b.is_null(), "row 162: count={count}");
        }
        // row 163: some strings[i] == NULL ⇒ the whole array is discarded
        for null_at in 0..3usize {
            let mut ptrs = strs.clone();
            ptrs[null_at] = std::ptr::null();
            let a = (c.cJSON_CreateStringArray)(ptrs.as_ptr(), 3);
            let b = (r.cJSON_CreateStringArray)(ptrs.as_ptr(), 3);
            assert!(
                a.is_null(),
                "C: CreateStringArray with NULL at {null_at} must be NULL"
            );
            assert_eq!(a.is_null(), b.is_null(), "row 163: NULL at {null_at}");
        }
        // count == 0 with a valid pointer is NOT an error: an empty array
        for which in 0..4 {
            let a = match which {
                0 => (c.cJSON_CreateIntArray)(ints.as_ptr(), 0),
                1 => (c.cJSON_CreateFloatArray)(floats.as_ptr(), 0),
                2 => (c.cJSON_CreateDoubleArray)(doubles.as_ptr(), 0),
                _ => (c.cJSON_CreateStringArray)(strs.as_ptr(), 0),
            };
            let b = match which {
                0 => (r.cJSON_CreateIntArray)(ints.as_ptr(), 0),
                1 => (r.cJSON_CreateFloatArray)(floats.as_ptr(), 0),
                2 => (r.cJSON_CreateDoubleArray)(doubles.as_ptr(), 0),
                _ => (r.cJSON_CreateStringArray)(strs.as_ptr(), 0),
            };
            assert!(!a.is_null(), "C: count == 0 yields an empty array");
            assert_eq!(snap(a), snap(b), "count == 0, which={which}");
            assert_eq!(print_and_take(&c, a), print_and_take(&r, b));
            (c.cJSON_Delete)(a);
            (r.cJSON_Delete)(b);
        }
    }
}

// ===========================================================================
// rows 164, 168, 169 — cJSON_Duplicate
// ===========================================================================
#[test]
fn err_duplicate() {
    let (c, r) = both();
    unsafe {
        for recurse in [0i32, 1, 2, -1] {
            let a = (c.cJSON_Duplicate)(std::ptr::null(), recurse);
            let b = (r.cJSON_Duplicate)(std::ptr::null(), recurse);
            assert!(a.is_null(), "C: Duplicate(NULL, {recurse}) must be NULL");
            assert_eq!(a.is_null(), b.is_null(), "row 164: recurse={recurse}");
        }
    }
}

#[test]
fn err_duplicate_circular() {
    // `cJSON_Duplicate_rec` recurses once per `->child` level, so reaching
    // `CJSON_CIRCULAR_LIMIT` (10000) needs 10000 live frames on BOTH sides.
    // The required stack depends only on the per-frame size chosen by each
    // compiler at the optimisation level in use — 80 bytes for `cJSON.c` at
    // `-O0` (~800 KiB total), 64 bytes for the release Rust build (~640 KiB) and
    // 320 bytes for the unoptimised Rust build (~3.2 MiB), which exceeds the
    // 2 MiB default thread stack.  Stack size is the caller's responsibility, so
    // the probe runs on a thread with plenty of headroom for every build.
    let h = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(duplicate_circular_body)
        .expect("spawn deep-recursion thread");
    h.join().expect("deep-recursion probe panicked");
}

fn duplicate_circular_body() {
    let (c, r) = both();
    unsafe {
        // rows 168/169: a self-referential `child` makes cJSON_Duplicate_rec
        // recurse until `depth >= CJSON_CIRCULAR_LIMIT` (10000) and then fail
        // all the way back out.
        let ac = (c.cJSON_CreateArray)();
        let ar = (r.cJSON_CreateArray)();
        (*ac).child = ac;
        (*ar).child = ar;
        let a = (c.cJSON_Duplicate)(ac, 1);
        let b = (r.cJSON_Duplicate)(ar, 1);
        assert!(
            a.is_null(),
            "C: duplicating a self-referential tree must hit CJSON_CIRCULAR_LIMIT"
        );
        assert_eq!(a.is_null(), b.is_null(), "rows 168/169: self-referential child");
        // recurse == 0 stops before walking children, so it succeeds
        let a = (c.cJSON_Duplicate)(ac, 0);
        let b = (r.cJSON_Duplicate)(ar, 0);
        assert!(!a.is_null(), "C: recurse == 0 must succeed");
        assert_eq!(a.is_null(), b.is_null(), "row 168: recurse == 0");
        (c.cJSON_Delete)(a);
        (r.cJSON_Delete)(b);
        (*ac).child = std::ptr::null_mut();
        (*ar).child = std::ptr::null_mut();
        (c.cJSON_Delete)(ac);
        (r.cJSON_Delete)(ar);

        // A genuinely deep (not circular) chain: exactly at the boundary the
        // duplicate still succeeds; one deeper it fails.
        // The boundary: `cJSON_Duplicate_rec(node[k], k)` fails when some
        // node[k] with k >= CJSON_CIRCULAR_LIMIT still has a child, i.e. for a
        // chain of N nodes when N - 1 > 10000.  The exact position is asserted
        // by requiring that the sweep crosses from success to failure.
        let mut saw_ok = false;
        let mut saw_fail = false;
        for depth in [9998usize, 10000, 10001, 10002, 10003] {
            let mut roots_c = Vec::with_capacity(depth);
            let mut roots_r = Vec::with_capacity(depth);
            for _ in 0..depth {
                roots_c.push((c.cJSON_CreateArray)());
                roots_r.push((r.cJSON_CreateArray)());
            }
            for i in 0..depth - 1 {
                (c.cJSON_AddItemToArray)(roots_c[i], roots_c[i + 1]);
                (r.cJSON_AddItemToArray)(roots_r[i], roots_r[i + 1]);
            }
            let a = (c.cJSON_Duplicate)(roots_c[0], 1);
            let b = (r.cJSON_Duplicate)(roots_r[0], 1);
            if a.is_null() {
                saw_fail = true;
            } else {
                saw_ok = true;
            }
            assert_eq!(a.is_null(), b.is_null(), "row 168: chain depth {depth}");
            (c.cJSON_Delete)(a);
            (r.cJSON_Delete)(b);
            (c.cJSON_Delete)(roots_c[0]);
            (r.cJSON_Delete)(roots_r[0]);
        }
        assert!(
            saw_ok && saw_fail,
            "row 168: the sweep must cross CJSON_CIRCULAR_LIMIT (ok={saw_ok}, fail={saw_fail})"
        );
    }
}

// ===========================================================================
// row 170 — cJSON_Minify(NULL)
// ===========================================================================
#[test]
fn err_minify_null() {
    let (c, r) = both();
    unsafe {
        (c.cJSON_Minify)(std::ptr::null_mut());
        (r.cJSON_Minify)(std::ptr::null_mut());
        // an empty string is also a no-op except for the terminator
        let mut bc = Bytes::new(b"");
        let mut br = Bytes::new(b"");
        (c.cJSON_Minify)(bc.as_mut_ptr());
        (r.cJSON_Minify)(br.as_mut_ptr());
        assert_eq!(bc.0, br.0, "row 170: empty input");
    }
}

// ===========================================================================
// rows 171–180 — the type predicates with a NULL item
// ===========================================================================
#[test]
fn err_type_predicates_null() {
    let (c, r) = both();
    unsafe {
        let n = std::ptr::null();
        let pc = [
            (c.cJSON_IsInvalid)(n),
            (c.cJSON_IsFalse)(n),
            (c.cJSON_IsTrue)(n),
            (c.cJSON_IsBool)(n),
            (c.cJSON_IsNull)(n),
            (c.cJSON_IsNumber)(n),
            (c.cJSON_IsString)(n),
            (c.cJSON_IsArray)(n),
            (c.cJSON_IsObject)(n),
            (c.cJSON_IsRaw)(n),
        ];
        let pr = [
            (r.cJSON_IsInvalid)(n),
            (r.cJSON_IsFalse)(n),
            (r.cJSON_IsTrue)(n),
            (r.cJSON_IsBool)(n),
            (r.cJSON_IsNull)(n),
            (r.cJSON_IsNumber)(n),
            (r.cJSON_IsString)(n),
            (r.cJSON_IsArray)(n),
            (r.cJSON_IsObject)(n),
            (r.cJSON_IsRaw)(n),
        ];
        assert_eq!(pc, [0; 10], "C: every predicate on NULL must be 0");
        assert_eq!(pc, pr, "rows 171-180: predicates on NULL");
    }
}

// ===========================================================================
// rows 181–193 — cJSON_Compare rejections
// ===========================================================================
#[test]
fn err_compare_reject() {
    let (c, r) = both();
    unsafe {
        let spec = Spec::Num(1.0);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        for cs_flag in [0, 1, 2, -1] {
            // rows 181, 182
            let a = (c.cJSON_Compare)(std::ptr::null(), bc.root, cs_flag);
            let b = (r.cJSON_Compare)(std::ptr::null(), br.root, cs_flag);
            assert_eq!(a, 0, "C: Compare(NULL, x) must be 0");
            assert_eq!(a, b, "row 181: cs={cs_flag}");
            let a = (c.cJSON_Compare)(bc.root, std::ptr::null(), cs_flag);
            let b = (r.cJSON_Compare)(br.root, std::ptr::null(), cs_flag);
            assert_eq!(a, 0, "C: Compare(x, NULL) must be 0");
            assert_eq!(a, b, "row 182: cs={cs_flag}");
            let a = (c.cJSON_Compare)(std::ptr::null(), std::ptr::null(), cs_flag);
            let b = (r.cJSON_Compare)(std::ptr::null(), std::ptr::null(), cs_flag);
            assert_eq!(a, 0, "C: Compare(NULL, NULL) must be 0");
            assert_eq!(a, b, "rows 181/182: both NULL cs={cs_flag}");
        }
        bc.delete();
        br.delete();

        // row 183: mismatched types, over the full cross-product
        let all = one_of_each_type();
        for (na, sa) in &all {
            for (nb, sb) in &all {
                let ac = build(&c, sa);
                let bcx = build(&c, sb);
                let ar = build(&r, sa);
                let brx = build(&r, sb);
                for cs_flag in [0, 1] {
                    assert_eq!(
                        (c.cJSON_Compare)(ac.root, bcx.root, cs_flag),
                        (r.cJSON_Compare)(ar.root, brx.root, cs_flag),
                        "row 183: {na} vs {nb} cs={cs_flag}"
                    );
                }
                ac.delete();
                bcx.delete();
                ar.delete();
                brx.delete();
            }
        }

        // row 184: identical but invalid `type` on both sides
        for t in BAD_TYPES {
            let ac = (c.cJSON_CreateNumber)(1.0);
            let bcx = (c.cJSON_CreateNumber)(1.0);
            let ar = (r.cJSON_CreateNumber)(1.0);
            let brx = (r.cJSON_CreateNumber)(1.0);
            for x in [ac, bcx] {
                (*x).type_ = t;
            }
            for x in [ar, brx] {
                (*x).type_ = t;
            }
            for cs_flag in [0, 1] {
                assert_eq!(
                    (c.cJSON_Compare)(ac, bcx, cs_flag),
                    (r.cJSON_Compare)(ar, brx, cs_flag),
                    "row 184: type {t:#x} cs={cs_flag}"
                );
                // self-comparison short-circuits before the second switch
                assert_eq!(
                    (c.cJSON_Compare)(ac, ac, cs_flag),
                    (r.cJSON_Compare)(ar, ar, cs_flag),
                    "row 184: self with type {t:#x} cs={cs_flag}"
                );
            }
            for x in [ac, bcx] {
                (*x).type_ = cJSON_Number;
            }
            for x in [ar, brx] {
                (*x).type_ = cJSON_Number;
            }
            (c.cJSON_Delete)(ac);
            (c.cJSON_Delete)(bcx);
            (r.cJSON_Delete)(ar);
            (r.cJSON_Delete)(brx);
        }

        // row 186: String / Raw with a NULL valuestring on either side
        for base_type in [cJSON_String, cJSON_Raw] {
            for null_side in 0..3 {
                let mk = |api: &Api| {
                    let x = if base_type == cJSON_String {
                        (api.cJSON_CreateString)(cs("v").as_ptr())
                    } else {
                        (api.cJSON_CreateRaw)(cs("v").as_ptr())
                    };
                    x
                };
                let ac = mk(&c);
                let bcx = mk(&c);
                let ar = mk(&r);
                let brx = mk(&r);
                let clear = |api: &Api, x: *mut CJson| {
                    (api.cJSON_free)((*x).valuestring as *mut c_void);
                    (*x).valuestring = std::ptr::null_mut();
                };
                if null_side == 0 || null_side == 2 {
                    clear(&c, ac);
                    clear(&r, ar);
                }
                if null_side == 1 || null_side == 2 {
                    clear(&c, bcx);
                    clear(&r, brx);
                }
                for cs_flag in [0, 1] {
                    assert_eq!(
                        (c.cJSON_Compare)(ac, bcx, cs_flag),
                        (r.cJSON_Compare)(ar, brx, cs_flag),
                        "row 186: type {base_type:#x} null_side={null_side} cs={cs_flag}"
                    );
                    assert_eq!(
                        (c.cJSON_Compare)(bcx, ac, cs_flag),
                        (r.cJSON_Compare)(brx, ar, cs_flag),
                        "row 186 (reversed): type {base_type:#x} null_side={null_side}"
                    );
                }
                (c.cJSON_Delete)(ac);
                (c.cJSON_Delete)(bcx);
                (r.cJSON_Delete)(ar);
                (r.cJSON_Delete)(brx);
            }
        }
    }
}

#[test]
fn err_compare_numbers() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1857_1857);
    unsafe {
        // row 185: compare_double rejection, including NaN vs NaN
        let mut pairs: Vec<(f64, f64)> = vec![
            (f64::NAN, f64::NAN),
            (f64::NAN, 1.0),
            (1.0, f64::NAN),
            (f64::INFINITY, f64::INFINITY),
            (f64::INFINITY, f64::NEG_INFINITY),
            (f64::INFINITY, f64::MAX),
            (0.0, -0.0),
            (0.0, 5e-324),
            (1.0, 1.0 + f64::EPSILON),
            (1.0, 1.0 + 2.0 * f64::EPSILON),
            (1e308, 1e308),
            (1e-308, 2e-308),
            (1.0, 2.0),
            (-1.0, 1.0),
        ];
        for _ in 0..2000 {
            pairs.push((rng.json_f64(), rng.json_f64()));
        }
        for _ in 0..1000 {
            let x = rng.json_f64();
            pairs.push((x, x));
        }
        for (x, y) in pairs {
            let ac = (c.cJSON_CreateNumber)(x);
            let bcx = (c.cJSON_CreateNumber)(y);
            let ar = (r.cJSON_CreateNumber)(x);
            let brx = (r.cJSON_CreateNumber)(y);
            for cs_flag in [0, 1] {
                assert_eq!(
                    (c.cJSON_Compare)(ac, bcx, cs_flag),
                    (r.cJSON_Compare)(ar, brx, cs_flag),
                    "row 185: {:#018x} vs {:#018x} cs={cs_flag}",
                    x.to_bits(),
                    y.to_bits()
                );
            }
            (c.cJSON_Delete)(ac);
            (c.cJSON_Delete)(bcx);
            (r.cJSON_Delete)(ar);
            (r.cJSON_Delete)(brx);
        }
    }
}

#[test]
fn err_compare_strings() {
    let (c, r) = both();
    let pool = string_pool();
    unsafe {
        // row 187: strcmp != 0 for String and Raw
        for (i, a) in pool.iter().enumerate().take(40) {
            for b in pool.iter().take(40) {
                for base in [0u8, 1u8] {
                    let mk = |api: &Api, s: &Vec<u8>| {
                        let bs = Bytes::new(s);
                        if base == 0 {
                            (api.cJSON_CreateString)(bs.as_ptr())
                        } else {
                            (api.cJSON_CreateRaw)(bs.as_ptr())
                        }
                    };
                    let ac = mk(&c, a);
                    let bcx = mk(&c, b);
                    let ar = mk(&r, a);
                    let brx = mk(&r, b);
                    for cs_flag in [0, 1] {
                        assert_eq!(
                            (c.cJSON_Compare)(ac, bcx, cs_flag),
                            (r.cJSON_Compare)(ar, brx, cs_flag),
                            "row 187: #{i} base={base} cs={cs_flag}"
                        );
                    }
                    (c.cJSON_Delete)(ac);
                    (c.cJSON_Delete)(bcx);
                    (r.cJSON_Delete)(ar);
                    (r.cJSON_Delete)(brx);
                }
            }
        }
    }
}

#[test]
fn err_compare_containers() {
    let (c, r) = both();
    // rows 188-193
    let arrays: Vec<Spec> = vec![
        Spec::Arr(vec![]),
        Spec::Arr(vec![Spec::Num(1.0)]),
        Spec::Arr(vec![Spec::Num(2.0)]),
        Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0)]),
        Spec::Arr(vec![Spec::Num(2.0), Spec::Num(1.0)]),
        Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0), Spec::Num(3.0)]),
        Spec::Arr(vec![Spec::Str(b"a".to_vec())]),
        Spec::Arr(vec![Spec::Arr(vec![Spec::Num(1.0)])]),
        Spec::Arr(vec![Spec::Null, Spec::True, Spec::False]),
    ];
    let objects: Vec<Spec> = vec![
        Spec::Obj(vec![]),
        Spec::Obj(vec![(b"a".to_vec(), Spec::Num(1.0))]),
        Spec::Obj(vec![(b"A".to_vec(), Spec::Num(1.0))]),
        Spec::Obj(vec![(b"a".to_vec(), Spec::Num(2.0))]),
        Spec::Obj(vec![
            (b"a".to_vec(), Spec::Num(1.0)),
            (b"b".to_vec(), Spec::Num(2.0)),
        ]),
        Spec::Obj(vec![
            (b"b".to_vec(), Spec::Num(2.0)),
            (b"a".to_vec(), Spec::Num(1.0)),
        ]),
        Spec::Obj(vec![
            (b"a".to_vec(), Spec::Num(1.0)),
            (b"b".to_vec(), Spec::Num(2.0)),
            (b"c".to_vec(), Spec::Num(3.0)),
        ]),
        Spec::Obj(vec![(b"a".to_vec(), Spec::Obj(vec![]))]),
        Spec::ObjCS(vec![(b"a".to_vec(), Spec::Num(1.0))]),
        // an object whose child has string == NULL (added via AddItemToArray)
        Spec::Arr(vec![Spec::Num(1.0)]),
    ];
    unsafe {
        for group in [&arrays, &objects] {
            for (i, sa) in group.iter().enumerate() {
                for (j, sb) in group.iter().enumerate() {
                    let ac = build(&c, sa);
                    let bcx = build(&c, sb);
                    let ar = build(&r, sa);
                    let brx = build(&r, sb);
                    for cs_flag in [0, 1] {
                        assert_eq!(
                            (c.cJSON_Compare)(ac.root, bcx.root, cs_flag),
                            (r.cJSON_Compare)(ar.root, brx.root, cs_flag),
                            "rows 188-193: #{i} vs #{j} cs={cs_flag}\na={sa:?}\nb={sb:?}"
                        );
                    }
                    ac.delete();
                    bcx.delete();
                    ar.delete();
                    brx.delete();
                }
            }
        }

        // row 190 specifically: an element of `a` with string == NULL, compared
        // as objects (get_object_item(b, NULL, …) returns NULL).
        for cs_flag in [0, 1] {
            let ac = (c.cJSON_CreateObject)();
            let ar = (r.cJSON_CreateObject)();
            let bcx = (c.cJSON_CreateObject)();
            let brx = (r.cJSON_CreateObject)();
            for (api, o) in [(&c, ac), (&r, ar)] {
                let n = (api.cJSON_CreateNumber)(1.0);
                (api.cJSON_AddItemToArray)(o, n); // no key
            }
            for (api, o) in [(&c, bcx), (&r, brx)] {
                (api.cJSON_AddNumberToObject)(o, cs("x").as_ptr(), 1.0);
            }
            assert_eq!(
                (c.cJSON_Compare)(ac, bcx, cs_flag),
                (r.cJSON_Compare)(ar, brx, cs_flag),
                "row 190: NULL-keyed child cs={cs_flag}"
            );
            assert_eq!(
                (c.cJSON_Compare)(bcx, ac, cs_flag),
                (r.cJSON_Compare)(brx, ar, cs_flag),
                "row 192: NULL-keyed child reversed cs={cs_flag}"
            );
            (c.cJSON_Delete)(ac);
            (c.cJSON_Delete)(bcx);
            (r.cJSON_Delete)(ar);
            (r.cJSON_Delete)(brx);
        }
    }
}

// ===========================================================================
// rows 196, 197 — out-of-range values for `cJSON_bool` / `int` parameters
// ===========================================================================
#[test]
fn err_out_of_range_int_args() {
    let (c, r) = both();
    unsafe {
        // row 196: cJSON_CreateBool
        for b in BAD_BOOLS.iter().copied().chain([0, 1]) {
            let a = (c.cJSON_CreateBool)(b);
            let x = (r.cJSON_CreateBool)(b);
            assert_eq!(snap(a), snap(x), "row 196: CreateBool({b})");
            assert_eq!(print_and_take(&c, a), print_and_take(&r, x));
            (c.cJSON_Delete)(a);
            (r.cJSON_Delete)(x);
        }

        // row 197: the boolean-ish parameters of every function that takes one
        let spec = Spec::Obj(vec![
            (b"a".to_vec(), Spec::Num(1.0)),
            (b"A".to_vec(), Spec::Str(b"x".to_vec())),
            (b"arr".to_vec(), Spec::Arr(vec![Spec::True, Spec::Null])),
        ]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        let mut buf_c = vec![0xB0u8; 512];
        let mut buf_r = vec![0xB0u8; 512];
        for v in BAD_BOOLS.iter().copied().chain([0, 1]) {
            // cJSON_PrintBuffered fmt
            assert_eq!(
                print_buffered_and_take(&c, bc.root, 256, v),
                print_buffered_and_take(&r, br.root, 256, v),
                "row 197: PrintBuffered(fmt={v})"
            );
            // cJSON_PrintPreallocated format
            buf_c.iter_mut().for_each(|x| *x = 0xB0);
            buf_r.iter_mut().for_each(|x| *x = 0xB0);
            let a = (c.cJSON_PrintPreallocated)(
                bc.root,
                buf_c.as_mut_ptr() as *mut c_char,
                512,
                v,
            );
            let b = (r.cJSON_PrintPreallocated)(
                br.root,
                buf_r.as_mut_ptr() as *mut c_char,
                512,
                v,
            );
            assert_eq!(a, b, "row 197: PrintPreallocated(format={v}) rc");
            assert_eq!(buf_c, buf_r, "row 197: PrintPreallocated(format={v}) buffer");

            // cJSON_Compare case_sensitive
            assert_eq!(
                (c.cJSON_Compare)(bc.root, bc.root, v),
                (r.cJSON_Compare)(br.root, br.root, v),
                "row 197: Compare(case_sensitive={v})"
            );
            // cJSON_Duplicate recurse
            let dc = (c.cJSON_Duplicate)(bc.root, v);
            let dr = (r.cJSON_Duplicate)(br.root, v);
            assert_eq!(snap(dc), snap(dr), "row 197: Duplicate(recurse={v})");
            assert_eq!(print_and_take(&c, dc), print_and_take(&r, dr));
            (c.cJSON_Delete)(dc);
            (r.cJSON_Delete)(dr);

            // cJSON_AddBoolToObject boolean
            let oc = (c.cJSON_CreateObject)();
            let or = (r.cJSON_CreateObject)();
            let pc = (c.cJSON_AddBoolToObject)(oc, cs("b").as_ptr(), v);
            let pr = (r.cJSON_AddBoolToObject)(or, cs("b").as_ptr(), v);
            assert_eq!(pc.is_null(), pr.is_null());
            assert_eq!(snap(oc), snap(or), "row 197: AddBoolToObject({v})");
            (c.cJSON_Delete)(oc);
            (r.cJSON_Delete)(or);
        }
        bc.delete();
        br.delete();

        // require_null_terminated is covered by err_parse_require_null_terminated
    }
}
