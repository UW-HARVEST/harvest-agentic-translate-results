//! Phase B — rows C38..C57: every parse entry point × option × input shape.
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int};
use std::fmt::Write as _;
use std::ptr::null_mut;

/// `cJSON_GetErrorPtr()` expressed as an offset into the input buffer, so the
/// two libraries are directly comparable.
unsafe fn err_off(api: &Api, base: *const c_char) -> String {
    let e = (api.cJSON_GetErrorPtr)();
    if e.is_null() {
        "NULL".to_string()
    } else {
        format!("+{}", e as isize - base as isize)
    }
}

fn documents() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        // C38: the value types
        b"null".to_vec(),
        b"true".to_vec(),
        b"false".to_vec(),
        b"0".to_vec(),
        b"\"str\"".to_vec(),
        b"[]".to_vec(),
        b"{}".to_vec(),
        // C39: numbers
        b"-0".to_vec(),
        b"1e5".to_vec(),
        b"1E+5".to_vec(),
        b"1e-5".to_vec(),
        b"0.5".to_vec(),
        b"-0.5".to_vec(),
        b"1.7976931348623157e308".to_vec(),
        b"1e309".to_vec(),
        b"-1e309".to_vec(),
        b"1e-320".to_vec(),
        b"1e-400".to_vec(),
        b"2147483647".to_vec(),
        b"2147483648".to_vec(),
        b"-2147483648".to_vec(),
        b"-2147483649".to_vec(),
        b"12345678901234567890".to_vec(),
        b"0.0000000000000000000001".to_vec(),
        b"123456789012345678901234567890.12345".to_vec(),
        b"9007199254740993".to_vec(),
        // C40: numbers with prefixes strtod accepts / stops on
        b"0x10".to_vec(),
        b"01".to_vec(),
        b"1.2.3".to_vec(),
        b"1e".to_vec(),
        b"1e+".to_vec(),
        b"--1".to_vec(),
        b"-+1".to_vec(),
        b"1-2".to_vec(),
        b"-".to_vec(),
        b"-e5".to_vec(),
        b"-.".to_vec(),
        b".5".to_vec(),
        b"+1".to_vec(),
        b"1.".to_vec(),
        b"0e0".to_vec(),
        b"00".to_vec(),
        b"1E".to_vec(),
        b"infinity".to_vec(),
        b"nan".to_vec(),
        // C41: strings
        b"\"\"".to_vec(),
        b"\"\\\"\"".to_vec(),
        b"\"\\\\\"".to_vec(),
        b"\"\\/\"".to_vec(),
        b"\"\\b\\f\\n\\r\\t\"".to_vec(),
        b"\"\\u0041\"".to_vec(),
        b"\"\\u00e9\"".to_vec(),
        b"\"\\u20ac\"".to_vec(),
        b"\"\\ud83d\\ude00\"".to_vec(),
        b"\"\\u0000\"".to_vec(),
        b"\"\\u0001\"".to_vec(),
        b"\"\\uD800\\uDC00\"".to_vec(),
        b"\"\\udbff\\udfff\"".to_vec(),
        b"\"\\uFFFF\"".to_vec(),
        b"\"\\uzzzz\"".to_vec(),
        b"\"\\u12\"".to_vec(),
        b"\"\\ud800\\u0041\"".to_vec(),
        b"\"\\ud800abcdef\"".to_vec(),
        b"\"\\udc00\"".to_vec(),
        b"\"\\x\"".to_vec(),
        b"\"unterminated".to_vec(),
        b"\"trailing backslash\\".to_vec(),
        "\"raw utf8 é€😀\"".as_bytes().to_vec(),
        vec![b'"', 0x80, 0xff, b'"'],
        b"\"tab\there\"".to_vec(),
        // C42: arrays
        b"[1]".to_vec(),
        b"[1,2,3]".to_vec(),
        b"[ 1 , 2 ]".to_vec(),
        b"[[[]]]".to_vec(),
        b"[null,true,false,0,\"s\",[],{}]".to_vec(),
        b"[,]".to_vec(),
        b"[1,]".to_vec(),
        b"[1 2]".to_vec(),
        b"[1".to_vec(),
        b"[".to_vec(),
        b"[ ".to_vec(),
        b"]".to_vec(),
        b"[x]".to_vec(),
        b"[[1],[2]]".to_vec(),
        // C43: objects
        b"{\"a\":1}".to_vec(),
        b"{\"a\":1,\"b\":2}".to_vec(),
        b"{ \"a\" : 1 }".to_vec(),
        b"{\"a\":{\"b\":{\"c\":1}}}".to_vec(),
        b"{\"a\":1,\"a\":2}".to_vec(),
        b"{\"A\":1,\"a\":2}".to_vec(),
        b"{\"\":1}".to_vec(),
        b"{\"a\\n\":1}".to_vec(),
        b"{a:1}".to_vec(),
        b"{1:2}".to_vec(),
        b"{\"a\" 1}".to_vec(),
        b"{\"a\"}".to_vec(),
        b"{\"a\":}".to_vec(),
        b"{\"a\":x}".to_vec(),
        b"{\"a\":1".to_vec(),
        b"{\"a\":1,".to_vec(),
        b"{\"a\":1 \"b\":2}".to_vec(),
        b"{".to_vec(),
        b"{ ".to_vec(),
        b"}".to_vec(),
        b"{\"a\":1,}".to_vec(),
        // C44: whitespace
        b" \t\r\n1".to_vec(),
        b"1 \t\r\n".to_vec(),
        b"\n\n[\n1\n,\n2\n]\n".to_vec(),
        b"".to_vec(),
        b" ".to_vec(),
        b"\t".to_vec(),
        // C45: BOM
        b"\xEF\xBB\xBF1".to_vec(),
        b"\xEF\xBB\xBF[1,2]".to_vec(),
        b"\xEF\xBB\xBF".to_vec(),
        b"\xEF\xBB".to_vec(),
        b"\xEF\xBB\xBF ".to_vec(),
        b"\xEF\xBB\xBFx".to_vec(),
        // C46: trailing garbage
        b"1 garbage".to_vec(),
        b"{} x".to_vec(),
        b"[1] [2]".to_vec(),
        b"nullnull".to_vec(),
        b"truex".to_vec(),
        b"falsey".to_vec(),
        // partial keywords
        b"nul".to_vec(),
        b"tru".to_vec(),
        b"fals".to_vec(),
        b"NULL".to_vec(),
        b"True".to_vec(),
        // misc rejections
        b"x".to_vec(),
        b"'a'".to_vec(),
        b"\0".to_vec(),
        vec![0x01],
        vec![0xff],
    ];
    // a few generated documents
    let mut rng = Rng::new(0x4444_5555_6666_7777);
    for _ in 0..60 {
        v.push(gen_json(&mut rng));
    }
    v
}

#[test]
fn c38_c46_parse_documents() {
    diff("C38..C46 cJSON_Parse over every input shape", |api| unsafe {
        let mut log = String::new();
        for d in documents() {
            let buf = CBuf::new(&d);
            let root = (api.cJSON_Parse)(buf.ptr());
            let _ = writeln!(
                log,
                "src={} null={} err={}",
                show(&d),
                root.is_null(),
                err_off(api, buf.ptr())
            );
            let _ = write!(log, "{}", dump(root));
            let pf = take_print(api, (api.cJSON_Print)(root));
            let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
            let _ = writeln!(
                log,
                "  fmt={} unfmt={}",
                pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                pu.map(|v| show(&v)).unwrap_or("NULL".into())
            );
            (api.cJSON_Delete)(root);
        }
        log
    });
}

#[test]
fn c47_c48_parse_with_length() {
    diff("C47/C48 cJSON_ParseWithLength strlen vs strlen+1", |api| unsafe {
        let mut log = String::new();
        for d in documents() {
            let buf = CBuf::new(&d);
            for len in [buf.len(), buf.len() + 1] {
                let root = (api.cJSON_ParseWithLength)(buf.ptr(), len);
                let _ = writeln!(
                    log,
                    "src={} len={len} null={} err={}",
                    show(&d),
                    root.is_null(),
                    err_off(api, buf.ptr())
                );
                let _ = write!(log, "{}", dump(root));
                let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
                let _ = writeln!(log, "  unfmt={}", pu.map(|v| show(&v)).unwrap_or("NULL".into()));
                (api.cJSON_Delete)(root);
            }
        }
        log
    });
}

#[test]
fn c49_parse_with_length_truncated() {
    diff("C49 cJSON_ParseWithLength truncated at every offset", |api| unsafe {
        let mut log = String::new();
        let docs: Vec<&[u8]> = vec![
            b"{\"key\":[1,2.5,\"s\\u0041\",null,true,false,{}]}",
            b"[[[[1]]]]",
            b"\xEF\xBB\xBF{\"a\":\"\\ud83d\\ude00\"}",
            b"  \t[ 1 , 2 ]  ",
            b"1234.5678e-9",
        ];
        for d in docs {
            let buf = CBuf::new(d);
            for len in 0..=buf.0.len() {
                let root = (api.cJSON_ParseWithLength)(buf.ptr(), len);
                let _ = writeln!(
                    log,
                    "src={} len={len} null={} err={}",
                    show(d),
                    root.is_null(),
                    err_off(api, buf.ptr())
                );
                let _ = write!(log, "{}", dump(root));
                (api.cJSON_Delete)(root);
            }
        }
        log
    });
}

#[test]
fn c50_parse_with_length_longer() {
    diff("C50 cJSON_ParseWithLength longer than the string", |api| unsafe {
        let mut log = String::new();
        for d in [
            &b"1"[..],
            b"[1,2]",
            b"{\"a\":1}",
            b"null",
            b"true",
            b"\"s\"",
            b"[1",
            b"{",
        ] {
            // buffer = document + NUL + 8 readable padding bytes
            let mut bytes = d.to_vec();
            bytes.push(0);
            bytes.extend_from_slice(b"XYZ\0[1]\0");
            let buf = CBuf::new(&bytes);
            for extra in 0..=8usize {
                let len = d.len() + 1 + extra;
                let root = (api.cJSON_ParseWithLength)(buf.ptr(), len);
                let _ = writeln!(
                    log,
                    "src={} len={len} null={} err={}",
                    show(d),
                    root.is_null(),
                    err_off(api, buf.ptr())
                );
                let _ = write!(log, "{}", dump(root));
                (api.cJSON_Delete)(root);
            }
        }
        log
    });
}

#[test]
fn c51_c54_parse_with_opts() {
    diff("C51..C54 cJSON_ParseWith(Length)Opts option cross product", |api| unsafe {
        let mut log = String::new();
        for d in documents() {
            let buf = CBuf::new(&d);
            for rnt in [0i32, 1, 2, -1, c_int::MIN] {
                // ParseWithOpts, return_parse_end = NULL
                let root = (api.cJSON_ParseWithOpts)(buf.ptr(), null_mut(), rnt);
                let _ = writeln!(
                    log,
                    "opts src={} rnt={rnt} null={} err={}",
                    show(&d),
                    root.is_null(),
                    err_off(api, buf.ptr())
                );
                let _ = write!(log, "{}", dump(root));
                (api.cJSON_Delete)(root);

                // ParseWithOpts, return_parse_end set
                let mut end: *const c_char = 0x1 as *const c_char;
                let root = (api.cJSON_ParseWithOpts)(buf.ptr(), &mut end, rnt);
                let _ = writeln!(
                    log,
                    "opts+end src={} rnt={rnt} null={} end={} err={}",
                    show(&d),
                    root.is_null(),
                    if end == 0x1 as *const c_char {
                        "untouched".to_string()
                    } else {
                        format!("+{}", end as isize - buf.ptr() as isize)
                    },
                    err_off(api, buf.ptr())
                );
                (api.cJSON_Delete)(root);

                // ParseWithLengthOpts over the interesting lengths
                for len in [0usize, 1, buf.len(), buf.len() + 1] {
                    if len > buf.len() + 1 {
                        continue;
                    }
                    let mut end: *const c_char = 0x1 as *const c_char;
                    let root = (api.cJSON_ParseWithLengthOpts)(buf.ptr(), len, &mut end, rnt);
                    let _ = writeln!(
                        log,
                        "lenopts src={} len={len} rnt={rnt} null={} end={} err={}",
                        show(&d),
                        root.is_null(),
                        if end == 0x1 as *const c_char {
                            "untouched".to_string()
                        } else {
                            format!("+{}", end as isize - buf.ptr() as isize)
                        },
                        err_off(api, buf.ptr())
                    );
                    let _ = write!(log, "{}", dump(root));
                    (api.cJSON_Delete)(root);

                    let root = (api.cJSON_ParseWithLengthOpts)(buf.ptr(), len, null_mut(), rnt);
                    let _ = writeln!(
                        log,
                        "lenopts-noend src={} len={len} rnt={rnt} null={} err={}",
                        show(&d),
                        root.is_null(),
                        err_off(api, buf.ptr())
                    );
                    (api.cJSON_Delete)(root);
                }
            }
        }
        log
    });
}

fn nested(open: u8, close: u8, depth: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(depth * 2);
    for _ in 0..depth {
        v.push(open);
    }
    if open == b'{' {
        // objects need a key/value at the innermost level
        v.pop();
        v.extend_from_slice(b"{\"k\":1}");
        for _ in 1..depth {
            v.push(close);
        }
        // rebuild properly: {"k":{"k":...{"k":1}...}}
        v.clear();
        for _ in 0..depth {
            v.extend_from_slice(b"{\"k\":");
        }
        v.push(b'1');
        for _ in 0..depth {
            v.push(b'}');
        }
    } else {
        for _ in 0..depth {
            v.push(close);
        }
    }
    v
}

#[test]
fn c55_c56_nesting_limit() {
    diff("C55/C56 CJSON_NESTING_LIMIT boundary", |api| unsafe {
        let mut log = String::new();
        for depth in [1usize, 2, 3, 500, 998, 999, 1000, 1001, 1002, 2000] {
            for (o, c) in [(b'[', b']'), (b'{', b'}')] {
                let d = nested(o, c, depth);
                let buf = CBuf::new(&d);
                let root = (api.cJSON_Parse)(buf.ptr());
                let _ = writeln!(
                    log,
                    "depth={depth} kind={} null={} err={} size={}",
                    o as char,
                    root.is_null(),
                    err_off(api, buf.ptr()),
                    (api.cJSON_GetArraySize)(root)
                );
                if !root.is_null() {
                    // walk down and count the actual depth
                    let mut n = 0usize;
                    let mut p = root;
                    while !(*p).child.is_null() {
                        p = (*p).child;
                        n += 1;
                    }
                    let _ = writeln!(log, "  measured depth={n} leaf_type=0x{:x}", (*p).type_);
                }
                (api.cJSON_Delete)(root);
            }
        }
        log
    });
}

#[test]
fn c57_error_pointer() {
    diff("C57 cJSON_GetErrorPtr reset/set behaviour", |api| unsafe {
        let mut log = String::new();
        // 1. successful parse resets the error
        let good = cs("[1,2,3]");
        let bad = cs("[1,2,");
        for step in 0..3 {
            let root = (api.cJSON_Parse)(bad.as_ptr());
            let _ = writeln!(
                log,
                "step{step} bad: null={} err={}",
                root.is_null(),
                err_off(api, bad.as_ptr())
            );
            (api.cJSON_Delete)(root);
            let root = (api.cJSON_Parse)(good.as_ptr());
            let _ = writeln!(
                log,
                "step{step} good: null={} err_is_base={} err={}",
                root.is_null(),
                (api.cJSON_GetErrorPtr)() == good.as_ptr(),
                err_off(api, good.as_ptr())
            );
            (api.cJSON_Delete)(root);
        }
        // 2. NULL input leaves the error pointer NULL
        let root = (api.cJSON_Parse)(null_mut());
        let _ = writeln!(
            log,
            "null input: null={} errptr_null={}",
            root.is_null(),
            (api.cJSON_GetErrorPtr)().is_null()
        );
        // 3. every failing document's error offset
        for d in documents() {
            let buf = CBuf::new(&d);
            let root = (api.cJSON_Parse)(buf.ptr());
            if root.is_null() {
                let _ = writeln!(log, "fail src={} err={}", show(&d), err_off(api, buf.ptr()));
            }
            (api.cJSON_Delete)(root);
        }
        log
    });
}
