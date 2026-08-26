//! Phase C — differential error-path tests for `ERRORS.md` rows 1..99.
//!
//! Every test constructs the exact invalid input of its row, calls BOTH
//! libraries through their `.so` exports and requires the same rejection
//! (same sentinel / same error code / same error pointer), not merely
//! "both failed somehow".
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int};
use std::fmt::Write as _;
use std::ptr::null_mut;

unsafe fn err_off(api: &Api, base: *const c_char) -> String {
    let e = (api.cJSON_GetErrorPtr)();
    if e.is_null() {
        "NULL".to_string()
    } else {
        format!("+{}", e as isize - base as isize)
    }
}

/* ================= rows 1..7: accessors and strdup ================= */

#[test]
fn rows_1_to_7_accessor_rejections() {
    diff("ERRORS 1-7", |api| unsafe {
        let mut log = String::new();

        // rows 1 + 3: NULL item
        let _ = writeln!(
            log,
            "row1 GetStringValue(NULL) null={}",
            (api.cJSON_GetStringValue)(null_mut()).is_null()
        );
        let _ = writeln!(
            log,
            "row3 GetNumberValue(NULL) bits=0x{:016x}",
            (api.cJSON_GetNumberValue)(null_mut()).to_bits()
        );

        // rows 2 + 4: wrong type
        let raw = cs("raw");
        let items: [(&str, *mut CJson); 8] = [
            ("null", (api.cJSON_CreateNull)()),
            ("true", (api.cJSON_CreateTrue)()),
            ("false", (api.cJSON_CreateFalse)()),
            ("number", (api.cJSON_CreateNumber)(1.0)),
            ("array", (api.cJSON_CreateArray)()),
            ("object", (api.cJSON_CreateObject)()),
            ("raw", (api.cJSON_CreateRaw)(raw.as_ptr())),
            ("invalid", {
                let it = (api.cJSON_CreateNumber)(1.0);
                (*it).type_ = CJSON_INVALID;
                it
            }),
        ];
        for (n, it) in items {
            let _ = writeln!(
                log,
                "row2/4 {n}: str_null={} num_bits=0x{:016x}",
                (api.cJSON_GetStringValue)(it).is_null(),
                (api.cJSON_GetNumberValue)(it).to_bits()
            );
            (api.cJSON_Delete)(it);
        }

        // row 5: a child whose `string` is NULL makes lookups fail
        let obj = (api.cJSON_CreateObject)();
        (api.cJSON_AddItemToArray)(obj, (api.cJSON_CreateNumber)(1.0)); // no key
        let after = cs("after");
        (api.cJSON_AddItemToObject)(obj, after.as_ptr(), (api.cJSON_CreateNumber)(2.0));
        for probe in ["after", "AFTER", "nothing"] {
            let p = cs(probe);
            let _ = writeln!(
                log,
                "row5 {probe:?}: insens_null={} sens_null={} has={}",
                (api.cJSON_GetObjectItem)(obj, p.as_ptr()).is_null(),
                (api.cJSON_GetObjectItemCaseSensitive)(obj, p.as_ptr()).is_null(),
                (api.cJSON_HasObjectItem)(obj, p.as_ptr())
            );
        }
        (api.cJSON_Delete)(obj);

        // rows 6 + 7: NULL string / raw
        let _ = writeln!(
            log,
            "row6 CreateString(NULL) null={}",
            (api.cJSON_CreateString)(null_mut()).is_null()
        );
        let _ = writeln!(
            log,
            "row7 CreateRaw(NULL) null={}",
            (api.cJSON_CreateRaw)(null_mut()).is_null()
        );
        log
    });
}

/* ================= rows 10..12: parse_number ================= */

#[test]
fn rows_10_to_12_parse_number() {
    diff("ERRORS 10-12", |api| unsafe {
        let mut log = String::new();
        // row 11: strtod consumes nothing.  row 10 (NULL buffer/content) is
        // unreachable from the public API - `cJSON_ParseWithLengthOpts` rejects
        // `value == NULL` first (row 50) and always sets `content`.
        for src in [
            "-", "-e5", "-.", "-e", "-E", "-+", "--", "-.e", "- 1", "-,", "-]",
        ] {
            let b = CBuf::new(src.as_bytes());
            let root = (api.cJSON_Parse)(b.ptr());
            let _ = writeln!(
                log,
                "row11 {src:?}: null={} err={}",
                root.is_null(),
                err_off(api, b.ptr())
            );
            (api.cJSON_Delete)(root);
            // also nested, so the failure propagates through parse_array
            let nested = format!("[{src}]");
            let b2 = CBuf::new(nested.as_bytes());
            let root = (api.cJSON_Parse)(b2.ptr());
            let _ = writeln!(
                log,
                "row11 {nested:?}: null={} err={}",
                root.is_null(),
                err_off(api, b2.ptr())
            );
            (api.cJSON_Delete)(root);
        }
        log
    });
}

/* ================= rows 13..18: cJSON_SetValuestring ============ */

#[test]
fn rows_13_to_18_set_valuestring() {
    diff("ERRORS 13-18", |api| unsafe {
        let mut log = String::new();
        let val = cs("value");

        // row 13: NULL object
        let _ = writeln!(
            log,
            "row13 null_object={}",
            (api.cJSON_SetValuestring)(null_mut(), val.as_ptr()).is_null()
        );

        // row 14: not a string
        let raw = cs("raw");
        for (n, it) in [
            ("null", (api.cJSON_CreateNull)()),
            ("true", (api.cJSON_CreateTrue)()),
            ("number", (api.cJSON_CreateNumber)(1.0)),
            ("array", (api.cJSON_CreateArray)()),
            ("object", (api.cJSON_CreateObject)()),
            ("raw", (api.cJSON_CreateRaw)(raw.as_ptr())),
        ] {
            let _ = writeln!(
                log,
                "row14 {n}: null={}",
                (api.cJSON_SetValuestring)(it, val.as_ptr()).is_null()
            );
            (api.cJSON_Delete)(it);
        }

        // row 15: string reference
        let refstr = cs("referenced");
        let it = (api.cJSON_CreateStringReference)(refstr.as_ptr());
        let _ = writeln!(
            log,
            "row15 reference: null={} valuestring_unchanged={}",
            (api.cJSON_SetValuestring)(it, val.as_ptr()).is_null(),
            (*it).valuestring as *const c_char == refstr.as_ptr()
        );
        (api.cJSON_Delete)(it);

        // row 16: string item with a NULL valuestring
        let it = (api.cJSON_CreateNull)();
        (*it).type_ = CJSON_STRING;
        let _ = writeln!(
            log,
            "row16 null_valuestring: null={}",
            (api.cJSON_SetValuestring)(it, val.as_ptr()).is_null()
        );
        (api.cJSON_Delete)(it);

        // row 17: NULL new value
        let it = (api.cJSON_CreateString)(val.as_ptr());
        let _ = writeln!(
            log,
            "row17 null_value: null={} still={:?}",
            (api.cJSON_SetValuestring)(it, null_mut()).is_null(),
            read_cstr((*it).valuestring).map(|v| show(&v))
        );

        // row 18: overlapping buffers (same or interior pointer, shorter/equal)
        let same = (api.cJSON_SetValuestring)(it, (*it).valuestring);
        let _ = writeln!(
            log,
            "row18 self: null={} still={:?}",
            same.is_null(),
            read_cstr((*it).valuestring).map(|v| show(&v))
        );
        let inner = (*it).valuestring.add(1);
        let ret = (api.cJSON_SetValuestring)(it, inner);
        let _ = writeln!(
            log,
            "row18 interior: null={} still={:?}",
            ret.is_null(),
            read_cstr((*it).valuestring).map(|v| show(&v))
        );
        (api.cJSON_Delete)(it);
        log
    });
}

/* ============ rows 20..30, 44..46, 78..99: ensure()/print internals ===== */

/// Sweeping *every* buffer length from 0 to the exact required size drives every
/// `ensure()` failure inside `print_number`, `print_string_ptr`, `print_array`
/// and `print_object` (rows 24, 30, 45, 46, 79..82, 93..99) and compares both
/// the return code and the partially written bytes.
#[test]
fn rows_24_30_45_46_79_99_ensure_failures() {
    diff("ERRORS 24/30/45/46/79-82/93-99", |api| unsafe {
        let mut log = String::new();
        let mut targets: Vec<(String, *mut CJson)> = Vec::new();

        let s = Box::leak(Box::new(cs("string \"needing\"\tescapes\x01")));
        let plain = Box::leak(Box::new(cs("plain")));
        let raw = Box::leak(Box::new(cs("[0,1]")));

        targets.push(("number".into(), (api.cJSON_CreateNumber)(1.0 / 3.0)));
        targets.push(("number_int".into(), (api.cJSON_CreateNumber)(-1234.0)));
        targets.push(("nan".into(), (api.cJSON_CreateNumber)(f64::NAN)));
        targets.push(("null".into(), (api.cJSON_CreateNull)()));
        targets.push(("true".into(), (api.cJSON_CreateTrue)()));
        targets.push(("false".into(), (api.cJSON_CreateFalse)()));
        targets.push(("string".into(), (api.cJSON_CreateString)(s.as_ptr())));
        targets.push(("raw".into(), (api.cJSON_CreateRaw)(raw.as_ptr())));
        targets.push(("empty_array".into(), (api.cJSON_CreateArray)()));
        targets.push(("empty_object".into(), (api.cJSON_CreateObject)()));

        let arr = (api.cJSON_CreateArray)();
        (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateNumber)(1.0));
        (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateString)(s.as_ptr()));
        (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateNull)());
        targets.push(("array3".into(), arr));

        let obj = (api.cJSON_CreateObject)();
        (api.cJSON_AddItemToObject)(obj, plain.as_ptr(), (api.cJSON_CreateNumber)(2.0));
        (api.cJSON_AddItemToObject)(obj, s.as_ptr(), (api.cJSON_CreateString)(s.as_ptr()));
        let inner = (api.cJSON_CreateObject)();
        (api.cJSON_AddItemToObject)(inner, plain.as_ptr(), (api.cJSON_CreateArray)());
        (api.cJSON_AddItemToObject)(obj, plain.as_ptr(), inner);
        targets.push(("object_nested".into(), obj));

        // row 45: object member with a NULL key -> print_string_ptr(NULL)
        let obj = (api.cJSON_CreateObject)();
        (api.cJSON_AddItemToArray)(obj, (api.cJSON_CreateNumber)(5.0));
        targets.push(("object_null_key".into(), obj));

        // string item with NULL valuestring -> print_string_ptr(NULL)
        let it = (api.cJSON_CreateNull)();
        (*it).type_ = CJSON_STRING;
        targets.push(("string_null_value".into(), it));

        for (name, item) in &targets {
            for format in [0i32, 1] {
                let full = if format == 1 {
                    take_print(api, (api.cJSON_Print)(*item))
                } else {
                    take_print(api, (api.cJSON_PrintUnformatted)(*item))
                };
                let need = full.as_ref().map(|v| v.len()).unwrap_or(0);
                for len in 0..=(need + 6) {
                    let mut buf = vec![0x7Eu8; len + 4];
                    let rc = (api.cJSON_PrintPreallocated)(
                        *item,
                        buf.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        format,
                    );
                    let _ = writeln!(
                        log,
                        "{name} format={format} len={len} rc={rc} buf={}",
                        show(&buf)
                    );
                }
                // and cJSON_PrintBuffered with every prebuffer size
                for pre in 0..=(need + 4) {
                    let p = take_print(api, (api.cJSON_PrintBuffered)(*item, pre as c_int, format));
                    let _ = writeln!(
                        log,
                        "{name} format={format} prebuffer={pre} -> {}",
                        p.map(|v| show(&v)).unwrap_or("NULL".into())
                    );
                }
            }
        }
        for (_, item) in &targets {
            (api.cJSON_Delete)(*item);
        }
        log
    });
}

/* ============ rows 31..43: parse_string / utf16 rejections ============ */

#[test]
fn rows_31_to_43_string_rejections() {
    diff("ERRORS 31-43", |api| unsafe {
        let mut log = String::new();
        let cases: [(&str, &[u8]); 30] = [
            ("row31 non-hex", br#""\uZZZZ""#),
            ("row31 non-hex partial", br#""\u00 0""#),
            ("row31 zeros", br#""\u0000""#),
            ("row32 short", br#""\u12""#),
            ("row32 shortest", br#""\u""#),
            ("row32 no digits", br#""\u"#),
            ("row33 low surrogate", br#""\udc00""#),
            ("row33 low surrogate hi", br#""\udfff""#),
            ("row34 pair truncated", br#""\ud800\u12""#),
            ("row34 pair missing", br#""\ud800""#),
            ("row35 no backslash-u", br#""\ud800abcdef""#),
            ("row35 backslash-x", br#""\ud800\xabcd""#),
            ("row36 bad second", br#""\ud800A""#),
            ("row36 second high", br#""\ud800\ud800""#),
            ("row39 trailing backslash", br#""abc\"#),
            ("row39 only backslash", br#""\"#),
            ("row40 unterminated", br#""abc"#),
            ("row40 empty unterminated", br#"""#),
            ("row42 unknown escape x", br#""\x""#),
            ("row42 unknown escape space", br#""\ ""#),
            ("row42 unknown escape a", br#""\a""#),
            ("row42 unknown escape digit", br#""\1""#),
            ("row42 escape quote only", br#""\""#),
            ("valid escapes", br#""\"\\\/\b\f\n\r\t""#),
            ("valid bmp", "\"é€\"".as_bytes()),
            ("valid pair", "\"😀\"".as_bytes()),
            ("row38 key not a string", b"{a:1}"),
            ("row38 key number", b"{1:2}"),
            ("row38 key unterminated", b"{\"a:1}"),
            ("row43 in object key", br#"{"\ud800":1}"#),
        ];
        for (label, src) in cases {
            let b = CBuf::new(src);
            let root = (api.cJSON_Parse)(b.ptr());
            let _ = writeln!(
                log,
                "{label}: src={} null={} err={}",
                show(src),
                root.is_null(),
                err_off(api, b.ptr())
            );
            let _ = write!(log, "  {}", dump(root));
            let p = take_print(api, (api.cJSON_PrintUnformatted)(root));
            let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
            (api.cJSON_Delete)(root);

            // the same input truncated everywhere (drives rows 32/39/40 harder)
            for len in 0..=b.0.len() {
                let root = (api.cJSON_ParseWithLength)(b.ptr(), len);
                let _ = writeln!(
                    log,
                    "  len={len} null={} err={}",
                    root.is_null(),
                    err_off(api, b.ptr())
                );
                (api.cJSON_Delete)(root);
            }
        }
        log
    });
}

/* ============ rows 49..54: parse entry points ============ */

#[test]
fn rows_49_to_54_parse_entry_points() {
    diff("ERRORS 49-54", |api| unsafe {
        let mut log = String::new();

        // row 49/50: NULL value
        for rnt in [0i32, 1] {
            let root = (api.cJSON_Parse)(null_mut());
            let _ = writeln!(
                log,
                "row49 Parse(NULL) null={} errptr_null={}",
                root.is_null(),
                (api.cJSON_GetErrorPtr)().is_null()
            );
            let mut end: *const c_char = 0x1 as *const c_char;
            let root = (api.cJSON_ParseWithOpts)(null_mut(), &mut end, rnt);
            let _ = writeln!(
                log,
                "row49 ParseWithOpts(NULL,rnt={rnt}) null={} end_untouched={} errptr_null={}",
                root.is_null(),
                end == 0x1 as *const c_char,
                (api.cJSON_GetErrorPtr)().is_null()
            );
            let root = (api.cJSON_ParseWithLength)(null_mut(), 10);
            let _ = writeln!(
                log,
                "row50 ParseWithLength(NULL,10) null={} errptr_null={}",
                root.is_null(),
                (api.cJSON_GetErrorPtr)().is_null()
            );
            let mut end: *const c_char = 0x1 as *const c_char;
            let root = (api.cJSON_ParseWithLengthOpts)(null_mut(), 10, &mut end, rnt);
            let _ = writeln!(
                log,
                "row50 ParseWithLengthOpts(NULL,10) null={} end_untouched={} errptr_null={}",
                root.is_null(),
                end == 0x1 as *const c_char,
                (api.cJSON_GetErrorPtr)().is_null()
            );
        }

        // row 51: buffer_length == 0
        for src in ["1", "", "[1]", "null"] {
            let b = CBuf::new(src.as_bytes());
            let mut end: *const c_char = 0x1 as *const c_char;
            let root = (api.cJSON_ParseWithLengthOpts)(b.ptr(), 0, &mut end, 0);
            let _ = writeln!(
                log,
                "row51 {src:?} len=0: null={} end={} err={}",
                root.is_null(),
                if end == 0x1 as *const c_char {
                    "untouched".into()
                } else {
                    format!("+{}", end as isize - b.ptr() as isize)
                },
                err_off(api, b.ptr())
            );
            let root2 = (api.cJSON_ParseWithLength)(b.ptr(), 0);
            let _ = writeln!(
                log,
                "row51 {src:?} ParseWithLength len=0: null={} err={}",
                root2.is_null(),
                err_off(api, b.ptr())
            );
            (api.cJSON_Delete)(root);
            (api.cJSON_Delete)(root2);
        }

        // row 53: parse_value fails -> error position bookkeeping
        for src in [
            "x", "", " ", "[", "{", "[1,", "{\"a\"", "nul", "1x", "\"unterminated",
            "[1,2", "{\"a\":", "@", "\t\t\t", "[[[",
        ] {
            let b = CBuf::new(src.as_bytes());
            let mut end: *const c_char = 0x1 as *const c_char;
            let root = (api.cJSON_ParseWithOpts)(b.ptr(), &mut end, 0);
            let _ = writeln!(
                log,
                "row53 {src:?}: null={} end={} err={}",
                root.is_null(),
                if end == 0x1 as *const c_char {
                    "untouched".into()
                } else {
                    format!("+{}", end as isize - b.ptr() as isize)
                },
                err_off(api, b.ptr())
            );
            (api.cJSON_Delete)(root);
        }

        // row 54: require_null_terminated with trailing garbage
        for src in [
            "1 x", "{} ", "[1] [2]", "1 ", "1\t", "null null", "\"a\" b", "1",
            "  1  ", "[1]x",
        ] {
            let b = CBuf::new(src.as_bytes());
            for rnt in [0i32, 1, 2, -1] {
                let mut end: *const c_char = 0x1 as *const c_char;
                let root = (api.cJSON_ParseWithOpts)(b.ptr(), &mut end, rnt);
                let _ = writeln!(
                    log,
                    "row54 {src:?} rnt={rnt}: null={} end={} err={}",
                    root.is_null(),
                    if end == 0x1 as *const c_char {
                        "untouched".into()
                    } else {
                        format!("+{}", end as isize - b.ptr() as isize)
                    },
                    err_off(api, b.ptr())
                );
                (api.cJSON_Delete)(root);
                // and without the trailing NUL in the length
                let root = (api.cJSON_ParseWithLengthOpts)(b.ptr(), b.len(), null_mut(), rnt);
                let _ = writeln!(
                    log,
                    "row54 {src:?} rnt={rnt} len=strlen: null={} err={}",
                    root.is_null(),
                    err_off(api, b.ptr())
                );
                (api.cJSON_Delete)(root);
            }
        }
        log
    });
}

/* ============ rows 56..71: print entry points and print_value ======== */

#[test]
fn rows_56_to_71_print_entry_points() {
    diff("ERRORS 56-71", |api| unsafe {
        let mut log = String::new();

        // rows 58/61/65/68: NULL item
        let _ = writeln!(
            log,
            "row58 Print(NULL) null={} PrintUnformatted(NULL) null={}",
            (api.cJSON_Print)(null_mut()).is_null(),
            (api.cJSON_PrintUnformatted)(null_mut()).is_null()
        );
        for pre in [0i32, 1, 256] {
            for fmt in [0i32, 1] {
                let _ = writeln!(
                    log,
                    "row61 PrintBuffered(NULL,{pre},{fmt}) null={}",
                    (api.cJSON_PrintBuffered)(null_mut(), pre, fmt).is_null()
                );
            }
        }
        let mut buf = [0u8; 64];
        for fmt in [0i32, 1] {
            let _ = writeln!(
                log,
                "row65 PrintPreallocated(NULL,buf,64,{fmt}) rc={}",
                (api.cJSON_PrintPreallocated)(null_mut(), buf.as_mut_ptr() as *mut c_char, 64, fmt)
            );
        }

        // row 59: prebuffer < 0
        let it = (api.cJSON_CreateNumber)(1.0);
        for pre in [-1i32, -2, c_int::MIN] {
            for fmt in [0i32, 1] {
                let _ = writeln!(
                    log,
                    "row59 PrintBuffered(item,{pre},{fmt}) null={}",
                    (api.cJSON_PrintBuffered)(it, pre, fmt).is_null()
                );
            }
        }

        // rows 62/63: length < 0 / NULL buffer
        for len in [-1i32, -100, c_int::MIN] {
            let _ = writeln!(
                log,
                "row62 PrintPreallocated(item,buf,{len},1) rc={}",
                (api.cJSON_PrintPreallocated)(it, buf.as_mut_ptr() as *mut c_char, len, 1)
            );
        }
        for len in [0i32, 1, 64] {
            let _ = writeln!(
                log,
                "row63 PrintPreallocated(item,NULL,{len},1) rc={}",
                (api.cJSON_PrintPreallocated)(it, null_mut(), len, 1)
            );
        }
        (api.cJSON_Delete)(it);

        // rows 56/70/71: print_value rejects invalid items
        let raw_null = (api.cJSON_CreateNull)();
        (*raw_null).type_ = CJSON_RAW; // valuestring == NULL
        let mut invalid: Vec<(String, *mut CJson)> = vec![("raw_null".into(), raw_null)];
        for t in [
            CJSON_INVALID,
            3,
            5,
            6,
            9,
            0x0F,
            0x10 | 0x20,
            0xFF,
            0x100,
            0x1FF,
            0x200,
            c_int::MIN,
            c_int::MAX,
            -1,
        ] {
            let it = (api.cJSON_CreateNumber)(3.0);
            (*it).type_ = t;
            invalid.push((format!("type_0x{t:x}"), it));
        }
        for (n, it) in &invalid {
            let mut b = [0x33u8; 64];
            let _ = writeln!(
                log,
                "row56/70/71 {n}: print_null={} unfmt_null={} buffered_null={} prealloc_rc={}",
                (api.cJSON_Print)(*it).is_null(),
                (api.cJSON_PrintUnformatted)(*it).is_null(),
                (api.cJSON_PrintBuffered)(*it, 64, 1).is_null(),
                (api.cJSON_PrintPreallocated)(*it, b.as_mut_ptr() as *mut c_char, 64, 1)
            );
            let _ = writeln!(log, "  buffer_after={}", show(&b));
        }
        // an invalid item nested inside a container: the failure must propagate
        for (n, it) in &invalid {
            let arr = (api.cJSON_CreateArray)();
            (api.cJSON_AddItemToArray)(arr, (api.cJSON_CreateNumber)(1.0));
            (api.cJSON_AddItemReferenceToArray)(arr, *it);
            let obj = (api.cJSON_CreateObject)();
            let k = cs("k");
            (api.cJSON_AddItemReferenceToObject)(obj, k.as_ptr(), *it);
            let _ = writeln!(
                log,
                "row71 nested {n}: array_fmt_null={} array_unfmt_null={} object_fmt_null={}",
                (api.cJSON_Print)(arr).is_null(),
                (api.cJSON_PrintUnformatted)(arr).is_null(),
                (api.cJSON_Print)(obj).is_null()
            );
            (api.cJSON_Delete)(arr);
            (api.cJSON_Delete)(obj);
        }
        for (_, it) in &invalid {
            (*(*it)).type_ = CJSON_NULL;
            (api.cJSON_Delete)(*it);
        }
        log
    });
}

/* ============ rows 72..91: parse_array / parse_object ============ */

#[test]
fn rows_72_to_91_container_parse_rejections() {
    diff("ERRORS 72-91", |api| unsafe {
        let mut log = String::new();
        let cases: [(&str, &[u8]); 34] = [
            ("row74 open only", b"["),
            ("row74 open space", b"[ "),
            ("row74 open ws", b"[\t\r\n"),
            ("row76 empty element", b"[,]"),
            ("row76 bad element", b"[x]"),
            ("row76 trailing comma", b"[1,]"),
            ("row76 double comma", b"[1,,2]"),
            ("row76 leading comma", b"[,1]"),
            ("row77 missing close", b"[1"),
            ("row77 missing comma", b"[1 2]"),
            ("row77 wrong close", b"[1}"),
            ("row77 nested missing", b"[[1]"),
            ("valid empty", b"[]"),
            ("valid ws empty", b"[  ]"),
            ("row85 open only", b"{"),
            ("row85 open space", b"{ "),
            ("row87 nothing after comma", b"{\"a\":1,"),
            ("row87 nothing after brace", b"{\"a\":1,  "),
            ("row88 unquoted key", b"{a:1}"),
            ("row88 number key", b"{1:2}"),
            ("row88 unterminated key", b"{\"a:1}"),
            ("row88 key after comma", b"{\"a\":1,b:2}"),
            ("row89 missing colon", b"{\"a\" 1}"),
            ("row89 no colon at all", b"{\"a\"}"),
            ("row89 comma instead", b"{\"a\",1}"),
            ("row90 missing value", b"{\"a\":}"),
            ("row90 bad value", b"{\"a\":x}"),
            ("row90 value is comma", b"{\"a\":,}"),
            ("row91 missing close", b"{\"a\":1"),
            ("row91 missing comma", b"{\"a\":1 \"b\":2}"),
            ("row91 wrong close", b"{\"a\":1]"),
            ("row91 trailing comma", b"{\"a\":1,}"),
            ("valid empty object", b"{}"),
            ("valid ws empty object", b"{ \t }"),
        ];
        for (label, src) in cases {
            for len_mode in 0..2 {
                let b = CBuf::new(src);
                let len = if len_mode == 0 { b.len() + 1 } else { b.len() };
                let root = (api.cJSON_ParseWithLength)(b.ptr(), len);
                let _ = writeln!(
                    log,
                    "{label} len={len}: src={} null={} err={}",
                    show(src),
                    root.is_null(),
                    err_off(api, b.ptr())
                );
                let _ = write!(log, "  {}", dump(root));
                (api.cJSON_Delete)(root);
            }
        }

        // rows 72/83: the nesting limit for both container kinds
        for depth in [999usize, 1000, 1001] {
            for (open, close) in [(b'[', b']'), (b'{', b'}')] {
                let mut src = Vec::new();
                if open == b'{' {
                    for _ in 0..depth {
                        src.extend_from_slice(b"{\"k\":");
                    }
                    src.push(b'1');
                    for _ in 0..depth {
                        src.push(b'}');
                    }
                } else {
                    for _ in 0..depth {
                        src.push(open);
                    }
                    for _ in 0..depth {
                        src.push(close);
                    }
                }
                let b = CBuf::new(&src);
                let root = (api.cJSON_Parse)(b.ptr());
                let _ = writeln!(
                    log,
                    "row72/83 depth={depth} kind={}: null={} err={}",
                    open as char,
                    root.is_null(),
                    err_off(api, b.ptr())
                );
                (api.cJSON_Delete)(root);
            }
        }
        // mixed nesting right at the limit
        for depth in [499usize, 500, 501] {
            let mut src = Vec::new();
            for _ in 0..depth {
                src.extend_from_slice(b"[{\"k\":");
            }
            src.push(b'1');
            for _ in 0..depth {
                src.extend_from_slice(b"}]");
            }
            let b = CBuf::new(&src);
            let root = (api.cJSON_Parse)(b.ptr());
            let _ = writeln!(
                log,
                "row72/83 mixed depth={depth}: null={} err={}",
                root.is_null(),
                err_off(api, b.ptr())
            );
            (api.cJSON_Delete)(root);
        }
        log
    });
}
