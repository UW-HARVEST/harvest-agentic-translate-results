//! Phase C — error-path differential tests for `load.c`
//! (ERRORS.md rows 138..184).

mod common;
use common::*;
use std::os::raw::c_char;
use std::ptr;

/// (row, description, input, flags, expected `enum json_error_code`)
const CASES: &[(u32, &str, &str, usize, u8)] = &[
    // (rows 146/147 need raw bytes -> see err146and147_utf8_stream_errors)
    (
        148,
        "eof inside string",
        "[\"abc",
        0,
        E_PREMATURE_END_OF_INPUT,
    ),
    (149, "raw newline in string", "[\"a\nb\"]", 0, E_INVALID_SYNTAX),
    (150, "control char in string", "[\"a\u{1}b\"]", 0, E_INVALID_SYNTAX),
    (150, "control char 0x1f", "[\"\u{1f}\"]", 0, E_INVALID_SYNTAX),
    (151, "\\u with non-hex", "[\"\\uZZZZ\"]", 0, E_INVALID_SYNTAX),
    (151, "\\u truncated", "[\"\\u12\"]", 0, E_INVALID_SYNTAX),
    (152, "unknown escape \\x", "[\"\\x\"]", 0, E_INVALID_SYNTAX),
    (152, "unknown escape \\z", "[\"\\z\"]", 0, E_INVALID_SYNTAX),
    (152, "unknown escape \\U", "[\"\\U0041\"]", 0, E_INVALID_SYNTAX),
    (
        154,
        "lone high surrogate",
        "[\"\\ud800\"]",
        0,
        E_INVALID_SYNTAX,
    ),
    (
        154,
        "high surrogate then plain char",
        "[\"\\ud800A\"]",
        0,
        E_INVALID_SYNTAX,
    ),
    (
        155,
        "high surrogate + non-low",
        "[\"\\ud800\\u0041\"]",
        0,
        E_INVALID_SYNTAX,
    ),
    (
        155,
        "high surrogate + high surrogate",
        "[\"\\ud800\\ud800\"]",
        0,
        E_INVALID_SYNTAX,
    ),
    (
        156,
        "lone low surrogate",
        "[\"\\udc00\"]",
        0,
        E_INVALID_SYNTAX,
    ),
    (
        156,
        "lone low surrogate dfff",
        "[\"\\udfff\"]",
        0,
        E_INVALID_SYNTAX,
    ),
    // --- lex_scan_number ----------------------------------------------------
    (157, "leading zero", "[01]", 0, E_INVALID_SYNTAX),
    (157, "negative leading zero", "[-012]", 0, E_INVALID_SYNTAX),
    (158, "bare minus", "[-]", 0, E_INVALID_SYNTAX),
    (158, "minus then letter", "[-x]", 0, E_INVALID_SYNTAX),
    (158, "minus then dot", "[-.5]", 0, E_INVALID_SYNTAX),
    (
        159,
        "integer overflow positive",
        "[9223372036854775808]",
        0,
        E_NUMERIC_OVERFLOW,
    ),
    (
        159,
        "integer overflow huge",
        "[123456789012345678901234567890]",
        0,
        E_NUMERIC_OVERFLOW,
    ),
    (
        160,
        "integer overflow negative",
        "[-9223372036854775809]",
        0,
        E_NUMERIC_OVERFLOW,
    ),
    (161, "trailing dot", "[1.]", 0, E_INVALID_SYNTAX),
    (161, "dot then exponent", "[1.e5]", 0, E_INVALID_SYNTAX),
    (162, "bare exponent", "[1e]", 0, E_INVALID_SYNTAX),
    (162, "exponent plus", "[1e+]", 0, E_INVALID_SYNTAX),
    (162, "exponent minus", "[1e-]", 0, E_INVALID_SYNTAX),
    (162, "exponent letter", "[1ex]", 0, E_INVALID_SYNTAX),
    (163, "real overflow", "[1e400]", 0, E_NUMERIC_OVERFLOW),
    (163, "real overflow negative", "[-1e400]", 0, E_NUMERIC_OVERFLOW),
    // --- lex_scan identifiers / stray bytes ---------------------------------
    (164, "truncated true", "[tru]", 0, E_INVALID_SYNTAX),
    (164, "truncated null", "[nul]", 0, E_INVALID_SYNTAX),
    (164, "wrong case True", "[True]", 0, E_INVALID_SYNTAX),
    (164, "wrong case NULL", "[NULL]", 0, E_INVALID_SYNTAX),
    (165, "stray at sign", "[@]", 0, E_INVALID_SYNTAX),
    (165, "stray hash", "[#]", 0, E_INVALID_SYNTAX),
    (165, "single quote", "['x']", 0, E_INVALID_SYNTAX),
    (165, "multi-byte utf8 token", "[\u{263A}]", 0, E_INVALID_SYNTAX),
    // --- parse_object -------------------------------------------------------
    (166, "non-string key", "{1:2}", 0, E_INVALID_SYNTAX),
    (166, "bare identifier key", "{a:1}", 0, E_INVALID_SYNTAX),
    (166, "comma right after brace", "{,}", 0, E_INVALID_SYNTAX),
    (
        167,
        "NUL byte in key",
        "{\"a\\u0000b\":1}",
        0,
        E_NULL_BYTE_IN_KEY,
    ),
    (
        167,
        "NUL byte in key with ALLOW_NUL",
        "{\"a\\u0000b\":1}",
        JSON_ALLOW_NUL,
        E_NULL_BYTE_IN_KEY,
    ),
    (
        168,
        "duplicate key rejected",
        "{\"a\":1,\"a\":2}",
        JSON_REJECT_DUPLICATES,
        E_DUPLICATE_KEY,
    ),
    (
        168,
        "duplicate key later",
        "{\"a\":1,\"b\":2,\"a\":3}",
        JSON_REJECT_DUPLICATES,
        E_DUPLICATE_KEY,
    ),
    (169, "missing colon", "{\"a\" 1}", 0, E_INVALID_SYNTAX),
    (169, "colon replaced by comma", "{\"a\",1}", 0, E_INVALID_SYNTAX),
    (
        170,
        "nested value error propagates",
        "{\"a\":[1,]}",
        0,
        E_INVALID_SYNTAX,
    ),
    (
        170,
        "nested overflow propagates",
        "{\"a\":1e400}",
        0,
        E_NUMERIC_OVERFLOW,
    ),
    // at EOF `error_set` remaps invalid_syntax -> premature_end_of_input (row 182)
    (171, "missing closing brace (EOF)", "{\"a\":1", 0, E_PREMATURE_END_OF_INPUT),
    (171, "brace closed by bracket", "{\"a\":1]", 0, E_INVALID_SYNTAX),
    (166, "trailing comma in object", "{\"a\":1,}", 0, E_INVALID_SYNTAX),
    (171, "object closed by bracket after comma", "{\"a\":1,\"b\":2]", 0, E_INVALID_SYNTAX),
    // --- parse_array --------------------------------------------------------
    (173, "missing closing bracket (EOF)", "[1,2", 0, E_PREMATURE_END_OF_INPUT),
    (173, "array closed by brace", "[1,2}", 0, E_INVALID_SYNTAX),
    (173, "bracket closed by brace", "[1}", 0, E_INVALID_SYNTAX),
    (173, "trailing comma in array", "[1,]", 0, E_INVALID_SYNTAX),
    (174, "element error propagates", "[1e400]", 0, E_NUMERIC_OVERFLOW),
    (174, "element error propagates 2", "[[01]]", 0, E_INVALID_SYNTAX),
    // --- parse_value --------------------------------------------------------
    (
        176,
        "NUL in string value",
        "[\"a\\u0000b\"]",
        0,
        E_NULL_CHARACTER,
    ),
    (177, "invalid token (valid UTF-8, no token)", "[\u{ff}]", 0, E_INVALID_SYNTAX),
    (178, "unexpected close brace", "[}]", 0, E_INVALID_SYNTAX),
    (178, "unexpected close bracket", "{]}", 0, E_INVALID_SYNTAX),
    (178, "unexpected comma", "[,]", 0, E_INVALID_SYNTAX),
    (178, "unexpected colon", "[:]", 0, E_INVALID_SYNTAX),
    // --- parse_json ---------------------------------------------------------
    (180, "scalar at top level", "1", 0, E_INVALID_SYNTAX),
    (180, "string at top level", "\"x\"", 0, E_INVALID_SYNTAX),
    (180, "true at top level", "true", 0, E_INVALID_SYNTAX),
    (
        181,
        "trailing garbage",
        "[1] x",
        0,
        E_END_OF_INPUT_EXPECTED,
    ),
    (
        181,
        "two documents",
        "{} {}",
        0,
        E_END_OF_INPUT_EXPECTED,
    ),
    (
        181,
        "extra bracket",
        "[1]]",
        0,
        E_END_OF_INPUT_EXPECTED,
    ),
    (
        181,
        "trailing garbage with DECODE_ANY",
        "1 2",
        JSON_DECODE_ANY,
        E_END_OF_INPUT_EXPECTED,
    ),
    // --- error_set remapping ------------------------------------------------
    (182, "empty input", "", 0, E_PREMATURE_END_OF_INPUT),
    (182, "whitespace only", "  \t\n", 0, E_PREMATURE_END_OF_INPUT),
    (182, "open bracket only", "[", 0, E_PREMATURE_END_OF_INPUT),
    (182, "open brace only", "{", 0, E_PREMATURE_END_OF_INPUT),
    (
        183,
        "context longer than 20 bytes",
        "[aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa]",
        0,
        E_INVALID_SYNTAX,
    ),
    (
        183,
        "context exactly 20 bytes",
        "[aaaaaaaaaaaaaaaaaaaa]",
        0,
        E_INVALID_SYNTAX,
    ),
    (
        183,
        "context 21 bytes",
        "[aaaaaaaaaaaaaaaaaaaaa]",
        0,
        E_INVALID_SYNTAX,
    ),
];

#[test]
fn err138to183_table() {
    diff("ERRORS 138-183 load table", |api, rec| unsafe {
        for (row, desc, input, flags, want) in CASES {
            let z = cs(input);
            let mut e = JsonError::patterned();
            let j = (api.json_loads)(z.as_ptr(), *flags, &mut e);
            assert!(
                j.is_null(),
                "[{}] row {row} ({desc}): expected rejection of {input:?}",
                api.tag
            );
            expect_code(api, *row, &e, *want);
            rec.line(&format!("row{row} {desc}"));
            rec.error(&format!("row{row}"), &e);

            // the same input through json_loadb must fail identically
            let mut e2 = JsonError::patterned();
            let j2 = (api.json_loadb)(
                input.as_ptr() as *const c_char,
                input.len(),
                *flags,
                &mut e2,
            );
            assert!(j2.is_null(), "[{}] row {row} loadb", api.tag);
            rec.error(&format!("row{row}.loadb"), &e2);

            // and with error == NULL (must not crash, same NULL return)
            let j3 = (api.json_loads)(z.as_ptr(), *flags, ptr::null_mut());
            assert!(j3.is_null());
            rec.line("noerr_null_ok");
        }
    });
}

/* ---------------- rows 138..144: argument validation of every source ---- */

#[test]
fn err138to144_null_arguments() {
    diff("ERRORS 138-144 null args", |api, rec| unsafe {
        // 138 json_loads(NULL)
        let mut e = JsonError::patterned();
        let j = (api.json_loads)(ptr::null(), 0, &mut e);
        assert!(j.is_null());
        expect_code(api, 138, &e, E_INVALID_ARGUMENT);
        rec.error("loads_null", &e);
        assert!((api.json_loads)(ptr::null(), 0, ptr::null_mut()).is_null());

        // 139 json_loadb(NULL)
        for len in [0usize, 1, 16, usize::MAX] {
            let mut e = JsonError::patterned();
            let j = (api.json_loadb)(ptr::null(), len, 0, &mut e);
            assert!(j.is_null());
            expect_code(api, 139, &e, E_INVALID_ARGUMENT);
            rec.error(&format!("loadb_null{len}"), &e);
        }

        // 140 json_loadf(NULL)
        let mut e = JsonError::patterned();
        let j = (api.json_loadf)(ptr::null_mut(), 0, &mut e);
        assert!(j.is_null());
        expect_code(api, 140, &e, E_INVALID_ARGUMENT);
        rec.error("loadf_null", &e);

        // 141 json_loadfd(negative)
        for fd in [-1i32, -2, i32::MIN] {
            let mut e = JsonError::patterned();
            let j = (api.json_loadfd)(fd, 0, &mut e);
            assert!(j.is_null());
            expect_code(api, 141, &e, E_INVALID_ARGUMENT);
            rec.error(&format!("loadfd{fd}"), &e);
        }

        // 142 json_load_file(NULL)
        let mut e = JsonError::patterned();
        let j = (api.json_load_file)(ptr::null(), 0, &mut e);
        assert!(j.is_null());
        expect_code(api, 142, &e, E_INVALID_ARGUMENT);
        rec.error("loadfile_null", &e);

        // 143 json_load_file(unopenable)
        for p in [
            "/nonexistent-dir-xyz/none.json",
            "/proc/self/nonexistent",
            "",
        ] {
            let cp = cs(p);
            let mut e = JsonError::patterned();
            let j = (api.json_load_file)(cp.as_ptr(), 0, &mut e);
            assert!(j.is_null());
            expect_code(api, 143, &e, E_CANNOT_OPEN_FILE);
            rec.error(&format!("cannot_open{p}"), &e);
        }

        // 144 json_load_callback(NULL)
        let mut e = JsonError::patterned();
        let j = (api.json_load_callback)(None, ptr::null_mut(), 0, &mut e);
        assert!(j.is_null());
        expect_code(api, 144, &e, E_INVALID_ARGUMENT);
        rec.error("loadcb_null", &e);

        // a directory as the path (fopen succeeds on Linux, reads fail)
        let cp = cs("/tmp");
        let mut e = JsonError::patterned();
        let j = (api.json_load_file)(cp.as_ptr(), 0, &mut e);
        rec.json("dir", j);
        rec.error("dir_err", &e);
        decref(api, j);
    });
}

/* ------------------------------ row 145: callback signals an error ------ */

use std::sync::Mutex;
static FEED: Mutex<(Vec<u8>, usize, usize, bool)> = Mutex::new((Vec::new(), 0, 0, false));

unsafe extern "C" fn cb(
    buffer: *mut std::os::raw::c_void,
    buflen: usize,
    _arg: *mut std::os::raw::c_void,
) -> usize {
    let mut g = FEED.lock().unwrap();
    let (data, pos, chunk, err) = &mut *g;
    let remaining = data.len() - *pos;
    if remaining == 0 {
        return if *err { usize::MAX } else { 0 };
    }
    let n = remaining.min(*chunk).min(buflen);
    ptr::copy_nonoverlapping(data[*pos..].as_ptr(), buffer as *mut u8, n);
    *pos += n;
    n
}

fn feed_set(data: &[u8], chunk: usize, err: bool) {
    *FEED.lock().unwrap() = (data.to_vec(), 0, chunk, err);
}

#[test]
fn err145_callback_error_return() {
    diff("ERRORS 145 callback (size_t)-1", |api, rec| unsafe {
        for (data, chunk) in [
            (&b"[1,2,3]"[..], 1usize),
            (&b"[1,2,3"[..], 1),
            (&b""[..], 1),
            (&b"["[..], 8),
        ] {
            for err in [false, true] {
                feed_set(data, chunk, err);
                let mut e = JsonError::patterned();
                let j = (api.json_load_callback)(Some(cb), ptr::null_mut(), 0, &mut e);
                rec.json("j", j);
                rec.error("err", &e);
                decref(api, j);
            }
        }
    });
}

/* ------------------------- rows 146..147: UTF-8 stream errors ----------- */

#[test]
fn err146and147_utf8_stream_errors() {
    diff("ERRORS 146-147 UTF-8 stream", |api, rec| unsafe {
        let cases: &[(u32, &[u8])] = &[
            (146, b"[\x80]"),
            (146, b"[\xbf]"),
            (146, b"[\xc0\x80]"),
            (146, b"[\xc1\xbf]"),
            (146, b"[\xf5\x80\x80\x80]"),
            (146, b"[\xff]"),
            (146, b"\x80"),
            (147, b"[\xc2\x41]"),
            (147, b"[\xc2]"),
            (147, b"[\xe0\xa0\x41]"),
            (147, b"[\xed\xa0\x80]"),
            (147, b"[\xf0\x80\x80\x80]"),
            (147, b"[\xf4\x90\x80\x80]"),
            (147, b"[\"\xc2\x41\"]"),
            (147, b"[\"\xed\xbf\xbf\"]"),
        ];
        for (row, bytes) in cases {
            let mut e = JsonError::patterned();
            let j = (api.json_loadb)(
                bytes.as_ptr() as *const c_char,
                bytes.len(),
                0,
                &mut e,
            );
            assert!(j.is_null(), "[{}] row {row}", api.tag);
            expect_code(api, *row, &e, E_INVALID_UTF8);
            rec.error(&format!("row{row}"), &e);
        }
    });
}

/* --------------------------------- row 175: depth limit ---------------- */

#[test]
fn err175_stack_overflow() {
    diff("ERRORS 175 depth limit", |api, rec| unsafe {
        for depth in [2049usize, 2050, 3000] {
            for open in ['[', '{'] {
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
                let z = cs(&s);
                let mut e = JsonError::patterned();
                let j = (api.json_loads)(z.as_ptr(), 0, &mut e);
                assert!(j.is_null(), "[{}] depth {depth}", api.tag);
                expect_code(api, 175, &e, E_STACK_OVERFLOW);
                rec.error(&format!("d{depth}{open}"), &e);
            }
        }
        // exactly at the limit must succeed
        for open in ['[', '{'] {
            let mut s = String::new();
            for _ in 0..2048 {
                if open == '[' {
                    s.push('[');
                } else {
                    s.push_str("{\"a\":");
                }
            }
            s.push('1');
            for _ in 0..2048 {
                if open == '[' {
                    s.push(']');
                } else {
                    s.push('}');
                }
            }
            let z = cs(&s);
            let mut e = JsonError::patterned();
            let j = (api.json_loads)(z.as_ptr(), 0, &mut e);
            rec.json(&format!("limit{open}"), j);
            rec.error(&format!("limit{open}.err"), &e);
            decref(api, j);
        }
    });
}

/* ------------------- rows 153, 172, 179, 184: OOM during parsing -------- */

#[test]
fn err153_172_179_184_parse_oom() {
    diff("ERRORS 153/172/179/184 parse OOM", |api, rec| unsafe {
        for text in [
            r#"{"a":1}"#,
            r#"["string value"]"#,
            r#"[1.5]"#,
            r#"[1,2,3,4,5,6,7,8,9]"#,
            r#"{"a":{"b":["c",1,2.5,true,null]}}"#,
            r#"{"k0":0,"k1":1,"k2":2,"k3":3,"k4":4,"k5":5,"k6":6,"k7":7,"k8":8}"#,
        ] {
            let t = text.to_string();
            oom_sweep(api, rec, text, 60, move |api, rec| {
                let z = cs(&t);
                let mut e = JsonError::patterned();
                let j = (api.json_loads)(z.as_ptr(), 0, &mut e);
                rec.json("j", j);
                rec.error("err", &e);
                if !j.is_null() {
                    match dumps(api, j, JSON_SORT_KEYS) {
                        None => rec.line("dump=NULL"),
                        Some(d) => rec.tag_bytes("dump", &d),
                    }
                }
                decref(api, j);
            });
        }
    });
}
