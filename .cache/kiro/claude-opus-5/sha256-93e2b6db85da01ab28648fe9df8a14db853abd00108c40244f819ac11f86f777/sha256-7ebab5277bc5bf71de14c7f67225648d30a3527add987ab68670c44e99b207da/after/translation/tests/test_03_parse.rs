//! Parsing: `cJSON_Parse`, `cJSON_ParseWithOpts`, `cJSON_ParseWithLength`,
//! `cJSON_ParseWithLengthOpts`, plus `cJSON_GetErrorPtr` and the
//! `return_parse_end` out-parameter.
mod common;

use common::*;
use std::os::raw::c_char;

/// A broad corpus of inputs: valid documents, malformed documents, and every
/// interesting edge case in the scanner.
pub fn corpus() -> Vec<&'static str> {
    vec![
        // --- trivial / literals
        "", " ", "\t\r\n ", "null", "true", "false", "NULL", "nul", "tru", "fals",
        "nulll", "truex", "falsey", "None", "undefined",
        // --- numbers
        "0", "-0", "1", "-1", "42", "0.0", "-0.0", "3.14159", "1e3", "1E3", "1e+3",
        "1e-3", "1.5e10", "-1.5e-10", "0e0", "1e", "1e+", "1e-", ".5", "-.5", "5.",
        "00", "01", "-01", "1.2.3", "1e2e3", "+1", "--1", "1-", "0x10", "1_000",
        "2147483647", "2147483648", "-2147483648", "-2147483649", "4294967296",
        "9007199254740993", "1e308", "1e309", "-1e309", "1e-308", "1e-400",
        "123456789012345678901234567890", "1.7976931348623157e308",
        "0.1", "0.2", "0.30000000000000004", "1.0000000000000002",
        "100000000000000000000", "1e21", "1e-7", "0.000001", "1e15", "1e16", "1e17",
        "-0.0000000000000000000001", "6.02e23", "1.23456789012345678",
        "Infinity", "-Infinity", "NaN", "1e1000", "-1e1000",
        // --- strings
        "\"\"", "\"a\"", "\"hello\"", "\"with space\"", "\"tab\\there\"",
        "\"newline\\nhere\"", "\"quote\\\"inside\"", "\"back\\\\slash\"",
        "\"slash\\/fwd\"", "\"\\b\\f\\n\\r\\t\"", "\"\\u0041\"", "\"\\u00e9\"",
        "\"\\u20ac\"", "\"\\ud83d\\ude00\"", "\"\\ud83d\"", "\"\\ude00\"",
        "\"\\ud83dx\"", "\"\\ud83d\\u0041\"", "\"\\uD834\\uDD1E\"", "\"\\uZZZZ\"",
        "\"\\u12\"", "\"\\u123\"", "\"\\x41\"", "\"\\q\"", "\"\\\"", "\"unclosed",
        "\"nested \\\" quote\"", "\"\\u0000\"", "\"\\u0001\"", "\"\\u007f\"",
        "\"\\ud800\\udc00\"", "\"\\udbff\\udfff\"", "\"\\uffff\"", "\"\\ufffe\"",
        "'single'", "\"\u{00e9}\u{00e8}\"", "\"\u{4f60}\u{597d}\"", "\"\u{1f600}\"",
        "\"a\u{0000}b\"",
        "\"very long string with lots of characters to force buffer growth 0123456789 0123456789 0123456789 0123456789\"",
        // --- arrays
        "[]", "[ ]", "[1]", "[1,2,3]", "[ 1 , 2 , 3 ]", "[null,true,false]",
        "[[]]", "[[[]]]", "[[1,2],[3,4]]", "[1,]", "[,1]", "[1 2]", "[", "]",
        "[1,2", "[\"a\",\"b\"]", "[{}]", "[{\"a\":1}]", "[[[[[[[[[[]]]]]]]]]]",
        "[1,[2,[3,[4,[5]]]]]", "[ , ]", "[1,2,3,]",
        // --- objects
        "{}", "{ }", "{\"a\":1}", "{ \"a\" : 1 }", "{\"a\":1,\"b\":2}",
        "{\"a\":{\"b\":{\"c\":1}}}", "{\"a\":[1,2]}", "{\"a\"}", "{\"a\":}",
        "{:1}", "{\"a\":1,}", "{,}", "{", "}", "{\"a\":1", "{\"a\" 1}",
        "{\"\":1}", "{\"a\":1,\"a\":2}", "{\"A\":1,\"a\":2}",
        "{\"a\":null,\"b\":true,\"c\":false,\"d\":1.5,\"e\":\"s\",\"f\":[],\"g\":{}}",
        // --- BOM / whitespace / garbage
        "\u{feff}{}", "\u{feff}", "\u{feff}[1]", "\u{feff}\u{feff}{}",
        "{} garbage", "[1] trailing", "null null", " \t\n{}\r\n ",
        "\u{000b}{}", "\u{0000}", "\u{001f}[]",
        // --- deep nesting around CJSON_NESTING_LIMIT is generated separately
        // --- realistic documents
        r#"{"name":"Jack (\"Bee\") Nimble","format":{"type":"rect","width":1920,"height":1080,"interlace":false,"frame rate":24}}"#,
        r#"[{"precision":"zip","Latitude":37.7668,"Longitude":-122.3959,"Address":"","City":"SAN FRANCISCO","State":"CA","Zip":"94107","Country":"US"}]"#,
        r#"{"glossary":{"title":"example glossary","GlossDiv":{"title":"S","GlossList":{"GlossEntry":{"ID":"SGML","SortAs":"SGML","GlossTerm":"Standard Generalized Markup Language","Acronym":"SGML","Abbrev":"ISO 8879:1986","GlossDef":{"para":"A meta-markup language","GlossSeeAlso":["GML","XML"]},"GlossSee":"markup"}}}}}"#,
    ]
}

fn nested(depth: usize, open: &str, close: &str) -> String {
    let mut s = String::new();
    for _ in 0..depth {
        s.push_str(open);
    }
    for _ in 0..depth {
        s.push_str(close);
    }
    s
}

fn nested_object(depth: usize) -> String {
    let mut s = String::new();
    for _ in 0..depth {
        s.push_str("{\"a\":");
    }
    s.push('1');
    for _ in 0..depth {
        s.push('}');
    }
    s
}

fn all_inputs() -> Vec<String> {
    let mut v: Vec<String> = corpus().into_iter().map(|s| s.to_string()).collect();
    for d in [1usize, 2, 100, 998, 999, 1000, 1001, 1002, 1100] {
        v.push(nested(d, "[", "]"));
        v.push(nested_object(d));
    }
    // every single byte as a one-character document
    for b in 0u8..=127 {
        if b == 0 {
            continue;
        }
        v.push((b as char).to_string());
    }
    v
}

/// Compare a full parse: tree, parse-end offset and error offset.
unsafe fn check_parse(input: &str) {
    let a = apis();
    let bytes = CVec::new(input);
    let p = bytes.ptr();

    unsafe {
        for require_null in [0, 1] {
            let mut c_end: *const c_char = std::ptr::null();
            let mut r_end: *const c_char = std::ptr::null();
            let ct = a.c.cJSON_ParseWithOpts(p, &mut c_end, require_null);
            let c_err = a.c.cJSON_GetErrorPtr();
            let rt = a.rust.cJSON_ParseWithOpts(p, &mut r_end, require_null);
            let r_err = a.rust.cJSON_GetErrorPtr();

            let ctx = format!("ParseWithOpts({input:?}, require_null={require_null})");
            assert_eq!(ct.is_null(), rt.is_null(), "{ctx}: nullness");
            if !ct.is_null() {
                assert_tree_eq(&ctx, ct, rt);
            }
            assert_eq!(
                offset_of(p, c_end),
                offset_of(p, r_end),
                "{ctx}: return_parse_end"
            );
            assert_eq!(offset_of(p, c_err), offset_of(p, r_err), "{ctx}: GetErrorPtr");
            a.c.cJSON_Delete(ct);
            a.rust.cJSON_Delete(rt);
        }

        // cJSON_Parse
        let ct = a.c.cJSON_Parse(p);
        let c_err = a.c.cJSON_GetErrorPtr();
        let rt = a.rust.cJSON_Parse(p);
        let r_err = a.rust.cJSON_GetErrorPtr();
        let ctx = format!("Parse({input:?})");
        assert_eq!(ct.is_null(), rt.is_null(), "{ctx}: nullness");
        if !ct.is_null() {
            assert_tree_eq(&ctx, ct, rt);
        }
        assert_eq!(offset_of(p, c_err), offset_of(p, r_err), "{ctx}: GetErrorPtr");
        a.c.cJSON_Delete(ct);
        a.rust.cJSON_Delete(rt);

        // Length based variants: exact length, length without the NUL, truncated.
        // Never pass a length larger than the real buffer - the API contract
        // allows the parser to read every byte up to `buffer_length`.
        let full = bytes.len_with_nul();
        let mut lengths = vec![0usize, full, full - 1];
        if full >= 2 {
            lengths.push(full - 2);
        }
        if full >= 3 {
            lengths.push(full / 2);
        }
        for len in lengths {
            let mut c_end: *const c_char = std::ptr::null();
            let mut r_end: *const c_char = std::ptr::null();
            for require_null in [0, 1] {
                let ct = a.c.cJSON_ParseWithLengthOpts(p, len, &mut c_end, require_null);
                let c_err = a.c.cJSON_GetErrorPtr();
                let rt = a
                    .rust
                    .cJSON_ParseWithLengthOpts(p, len, &mut r_end, require_null);
                let r_err = a.rust.cJSON_GetErrorPtr();
                let ctx =
                    format!("ParseWithLengthOpts({input:?}, len={len}, rn={require_null})");
                assert_eq!(ct.is_null(), rt.is_null(), "{ctx}: nullness");
                if !ct.is_null() {
                    assert_tree_eq(&ctx, ct, rt);
                }
                assert_eq!(
                    offset_of(p, c_end),
                    offset_of(p, r_end),
                    "{ctx}: return_parse_end"
                );
                assert_eq!(offset_of(p, c_err), offset_of(p, r_err), "{ctx}: GetErrorPtr");
                a.c.cJSON_Delete(ct);
                a.rust.cJSON_Delete(rt);
            }

            let ct = a.c.cJSON_ParseWithLength(p, len);
            let rt = a.rust.cJSON_ParseWithLength(p, len);
            let ctx = format!("ParseWithLength({input:?}, len={len})");
            assert_eq!(ct.is_null(), rt.is_null(), "{ctx}: nullness");
            if !ct.is_null() {
                assert_tree_eq(&ctx, ct, rt);
            }
            a.c.cJSON_Delete(ct);
            a.rust.cJSON_Delete(rt);
        }
    }
}

/// Owns a NUL terminated byte buffer (the input may contain arbitrary bytes).
pub struct CVec(Vec<u8>);

impl CVec {
    pub fn new(s: &str) -> CVec {
        let mut v = s.as_bytes().to_vec();
        v.push(0);
        CVec(v)
    }
    pub fn ptr(&self) -> *const c_char {
        self.0.as_ptr() as *const c_char
    }
    /// length including the trailing NUL, as `strlen(value) + 1` would give for
    /// a string without embedded NULs
    pub fn len_with_nul(&self) -> usize {
        self.0
            .iter()
            .position(|&b| b == 0)
            .map(|i| i + 1)
            .unwrap_or(self.0.len())
    }
}

fn offset_of(base: *const c_char, p: *const c_char) -> Option<isize> {
    if p.is_null() {
        None
    } else {
        Some(p as isize - base as isize)
    }
}

#[test]
fn parse_corpus() {
    let _guard = serial();
    for input in all_inputs() {
        unsafe { check_parse(&input) };
    }
}

#[test]
fn parse_null_input() {
    let _guard = serial();
    let a = apis();
    unsafe {
        assert_eq!(
            a.c.cJSON_Parse(std::ptr::null()).is_null(),
            a.rust.cJSON_Parse(std::ptr::null()).is_null()
        );
        assert_eq!(
            a.c.cJSON_GetErrorPtr().is_null(),
            a.rust.cJSON_GetErrorPtr().is_null(),
            "GetErrorPtr after Parse(NULL)"
        );
        assert_eq!(
            a.c.cJSON_ParseWithLength(std::ptr::null(), 5).is_null(),
            a.rust.cJSON_ParseWithLength(std::ptr::null(), 5).is_null()
        );
        let mut ce: *const c_char = std::ptr::null();
        let mut re: *const c_char = std::ptr::null();
        assert_eq!(
            a.c.cJSON_ParseWithOpts(std::ptr::null(), &mut ce, 1).is_null(),
            a.rust
                .cJSON_ParseWithOpts(std::ptr::null(), &mut re, 1)
                .is_null()
        );
        assert_eq!(ce.is_null(), re.is_null());
        assert_eq!(
            a.c.cJSON_ParseWithLengthOpts(std::ptr::null(), 0, &mut ce, 0)
                .is_null(),
            a.rust
                .cJSON_ParseWithLengthOpts(std::ptr::null(), 0, &mut re, 0)
                .is_null()
        );
    }
}

/// Inputs that are *not* NUL terminated at all: the length variants must stay
/// inside the buffer.  Any read past the end would be caught by ASAN, but at
/// minimum the two implementations must agree.
#[test]
fn parse_without_nul_terminator() {
    let _guard = serial();
    let a = apis();
    let raws: &[&[u8]] = &[
        b"1234", b"[1,2", b"{\"a\":1", b"\"abc", b"tru", b"nul", b"fals",
        b"[", b"{", b"1.5e", b"\\u00", b"[[[[", b"12345678901234567890",
    ];
    unsafe {
        for raw in raws {
            for len in 0..=raw.len() {
                let ct = a.c.cJSON_ParseWithLength(raw.as_ptr() as *const c_char, len);
                let rt = a
                    .rust
                    .cJSON_ParseWithLength(raw.as_ptr() as *const c_char, len);
                let ctx = format!("ParseWithLength({:?}, {len})", String::from_utf8_lossy(raw));
                assert_eq!(ct.is_null(), rt.is_null(), "{ctx}: nullness");
                if !ct.is_null() {
                    assert_tree_eq(&ctx, ct, rt);
                }
                a.c.cJSON_Delete(ct);
                a.rust.cJSON_Delete(rt);
            }
        }
    }
}

/// Round-trip: parse then print then parse again must be stable and identical.
#[test]
fn parse_print_roundtrip() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for input in all_inputs() {
            let bytes = CVec::new(&input);
            let ct = a.c.cJSON_Parse(bytes.ptr());
            let rt = a.rust.cJSON_Parse(bytes.ptr());
            if ct.is_null() {
                assert!(rt.is_null());
                continue;
            }
            let (cf, cu) = print_both(&a.c, ct);
            let (rf, ru) = print_both(&a.rust, rt);
            assert_eq!(cf, rf, "print(parse({input:?})) formatted");
            assert_eq!(cu, ru, "print(parse({input:?})) unformatted");
            a.c.cJSON_Delete(ct);
            a.rust.cJSON_Delete(rt);

            // second generation
            if let Some(mut text) = cu {
                text.push(0);
                let p = text.as_ptr() as *const c_char;
                let ct = a.c.cJSON_Parse(p);
                let rt = a.rust.cJSON_Parse(p);
                assert_eq!(ct.is_null(), rt.is_null());
                if !ct.is_null() {
                    assert_tree_eq(&format!("reparse of {input:?}"), ct, rt);
                }
                a.c.cJSON_Delete(ct);
                a.rust.cJSON_Delete(rt);
            }
        }
    }
}
