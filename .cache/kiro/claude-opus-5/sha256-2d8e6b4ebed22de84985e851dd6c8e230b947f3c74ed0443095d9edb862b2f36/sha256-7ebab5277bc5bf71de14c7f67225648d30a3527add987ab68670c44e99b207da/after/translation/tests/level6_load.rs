//! Level 6: load.c
//!
//! For every input and flag combination the parsed value (dumped byte-for-byte)
//! *and* the whole `json_error_t` (all 160 + 80 buffer bytes, including the
//! error code stored in `text[159]`) must match.

mod common;

use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_int, c_void};

const SEED: usize = 0x5eed_1234;

const JSON_REJECT_DUPLICATES: usize = 0x1;
const JSON_DISABLE_EOF_CHECK: usize = 0x2;
const JSON_DECODE_ANY: usize = 0x4;
const JSON_DECODE_INT_AS_REAL: usize = 0x8;
const JSON_ALLOW_NUL: usize = 0x10;

const DUMP_FLAGS: usize = 0x200 /* ENCODE_ANY */ | 0x400 /* ESCAPE_SLASH */;

fn seed_both() -> (&'static Lib, &'static Lib) {
    let (c, r) = libs();
    for l in [c, r] {
        let f: Symbol<FnJsonObjectSeed> = l.sym("json_object_seed");
        unsafe { f(SEED) };
    }
    (c, r)
}

fn load_flags() -> Vec<usize> {
    let mut v = Vec::new();
    for base in [
        0usize,
        JSON_DECODE_ANY,
        JSON_REJECT_DUPLICATES,
        JSON_DISABLE_EOF_CHECK,
        JSON_DECODE_INT_AS_REAL,
        JSON_ALLOW_NUL,
        JSON_DECODE_ANY | JSON_REJECT_DUPLICATES,
        JSON_DECODE_ANY | JSON_DISABLE_EOF_CHECK,
        JSON_DECODE_ANY | JSON_DECODE_INT_AS_REAL,
        JSON_DECODE_ANY | JSON_ALLOW_NUL,
        JSON_DECODE_ANY | JSON_ALLOW_NUL | JSON_DECODE_INT_AS_REAL,
        0x1f,
    ] {
        v.push(base);
    }
    v
}

/// Inputs: valid JSON, edge cases and a broad set of malformed documents.
fn inputs() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    let text: &[&str] = &[
        // --- well formed containers
        "{}",
        "[]",
        "[1]",
        "[1,2,3]",
        "{\"a\":1}",
        "{\"a\":1,\"b\":2}",
        "{\"a\":{\"b\":{\"c\":[1,2,{\"d\":null}]}}}",
        "[[[[[[[[[[1]]]]]]]]]]",
        "{\"a\":[],\"b\":{},\"c\":null,\"d\":true,\"e\":false}",
        "  \t\r\n {\"a\" : 1 }  \n ",
        // --- duplicates
        "{\"a\":1,\"a\":2}",
        "{\"a\":1,\"a\":2,\"a\":3}",
        // --- scalars (need DECODE_ANY)
        "null",
        "true",
        "false",
        "0",
        "-0",
        "1",
        "-1",
        "123",
        "-123",
        "0.0",
        "-0.0",
        "1.5",
        "1e5",
        "1E5",
        "1e+5",
        "1e-5",
        "1.5e10",
        "1.5E-10",
        "0e0",
        "\"\"",
        "\"a\"",
        "\"hello world\"",
        // --- number edge cases
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551616",
        "1e309",
        "-1e309",
        "1e-400",
        "1e1000",
        "0.00000000000000000000001",
        "123456789012345678901234567890",
        "1.7976931348623157e308",
        "1.7976931348623159e308",
        "2.2250738585072014e-308",
        "5e-324",
        "1e",
        "1e+",
        "1e-",
        "1.",
        ".1",
        "-",
        "+1",
        "01",
        "00",
        "-01",
        "1.2.3",
        "1..2",
        "--1",
        "1e5e5",
        "0x10",
        "Infinity",
        "-Infinity",
        "NaN",
        "inf",
        "nan",
        "1 ",
        " 1",
        // --- string escapes
        r#""\"""#,
        r#""\\""#,
        r#""\/""#,
        r#""\b\f\n\r\t""#,
        r#""\u0041""#,
        r#""\u00e9""#,
        r#""\u20ac""#,
        r#""\uD834\uDD1E""#,   // valid surrogate pair
        r#""\uD834""#,          // lone high surrogate
        r#""\uDD1E""#,          // lone low surrogate
        r#""\uD834\u0041""#,    // high surrogate + non-surrogate
        r#""\uD834\uD834""#,    // two high surrogates
        r#""\u0000""#,          // NUL escape (needs ALLOW_NUL)
        r#""a\u0000b""#,
        r#""\u001f""#,
        r#""\uFFFF""#,
        r#""\uffff""#,
        r#""\uFFFf""#,
        r#""\x41""#,            // invalid escape
        r#""\q""#,
        r#""\""#,
        r#""\u""#,
        r#""\u00""#,
        r#""\u00G1""#,
        r#""\u 041""#,
        "\"unterminated",
        "\"raw\nnewline\"",
        "\"raw\ttab\"",
        "\"raw\x01control\"",
        "\"del\x7f\"",
        // --- syntax errors
        "",
        " ",
        "\t\n\r ",
        "{",
        "}",
        "[",
        "]",
        "[,]",
        "[1,]",
        "[,1]",
        "[1 2]",
        "{\"a\"}",
        "{\"a\":}",
        "{:1}",
        "{\"a\":1,}",
        "{,\"a\":1}",
        "{\"a\":1 \"b\":2}",
        "{a:1}",
        "{'a':1}",
        "[1}",
        "{]",
        "[[1,2]",
        "{\"a\":[1}",
        "nul",
        "tru",
        "fals",
        "truex",
        "nullx",
        "{}{}",
        "[][]",
        "1 2",
        "\"a\" \"b\"",
        "{} garbage",
        "[] garbage",
        "1 garbage",
        "/* comment */ {}",
        "// comment\n{}",
        "#comment\n{}",
        // --- keys
        "{\"\":1}",
        "{\"\\u0000\":1}",
        "{\"a\\u0000b\":1}",
        "{1:2}",
        "{true:2}",
        "{null:2}",
        // --- UTF-8 in strings and keys
        "\"héllo\"",
        "\"日本語\"",
        "\"𝄞\"",
        "{\"ünïcödé\":\"vàlüé\"}",
        "\"\u{7f}\"",
        "\"\u{80}\"",
        "\"\u{10ffff}\"",
        // --- BOM
        "\u{feff}{}",
        "\u{feff}1",
    ];
    for t in text {
        v.push(t.as_bytes().to_vec());
    }

    // invalid UTF-8 byte sequences inside strings and keys
    for bad in [
        &[0x80u8][..],
        &[0xffu8, 0xfe][..],
        &[0xc0u8, 0x80][..],
        &[0xc1u8, 0xbf][..],
        &[0xedu8, 0xa0, 0x80][..],
        &[0xf5u8, 0x80, 0x80, 0x80][..],
        &[0xe2u8, 0x82][..],
        &[0xf0u8, 0x9f][..],
        &[0xf4u8, 0x90, 0x80, 0x80][..],
    ] {
        let mut s = b"\"".to_vec();
        s.extend_from_slice(bad);
        s.push(b'"');
        v.push(s);
        let mut s = b"{\"".to_vec();
        s.extend_from_slice(bad);
        s.extend_from_slice(b"\":1}");
        v.push(s);
        // and outside of a string
        v.push(bad.to_vec());
        let mut s = b"[".to_vec();
        s.extend_from_slice(bad);
        s.push(b']');
        v.push(s);
    }

    // embedded raw NUL bytes
    v.push(b"{\"a\":\"b\0c\"}".to_vec());
    v.push(b"[1,\0 2]".to_vec());
    v.push(b"\0".to_vec());
    v.push(b"{}\0".to_vec());
    v.push(b"{}\0garbage".to_vec());

    // depth: JSON_PARSER_MAX_DEPTH == 2048
    for depth in [1usize, 2, 100, 2046, 2047, 2048, 2049, 2100, 5000] {
        let mut s = Vec::new();
        s.extend(std::iter::repeat(b'[').take(depth));
        s.extend(std::iter::repeat(b']').take(depth));
        v.push(s);
        let mut s = Vec::new();
        for _ in 0..depth {
            s.extend_from_slice(b"{\"a\":");
        }
        s.extend_from_slice(b"1");
        for _ in 0..depth {
            s.push(b'}');
        }
        v.push(s);
    }

    // long documents (cross the 1024-byte stream buffer used by load.c)
    let big: Vec<u8> = {
        let mut s = b"[".to_vec();
        for i in 0..500 {
            if i > 0 {
                s.push(b',');
            }
            s.extend_from_slice(format!("{{\"key{i}\":{i}.5}}").as_bytes());
        }
        s.push(b']');
        s
    };
    v.push(big);
    v.push({
        let mut s = b"\"".to_vec();
        s.extend(std::iter::repeat(b'x').take(5000));
        s.push(b'"');
        s
    });
    v.push({
        let mut s = b"\"".to_vec();
        for _ in 0..800 {
            s.extend_from_slice(b"\\u00e9");
        }
        s.push(b'"');
        s
    });
    // a long error message to exercise error text truncation
    v.push({
        let mut s = b"{\"".to_vec();
        s.extend(std::iter::repeat(b'k').take(300));
        s.extend_from_slice(b"\" 1}");
        s
    });
    // errors deep inside a long document (line/column/position accounting)
    v.push({
        let mut s = Vec::new();
        for i in 0..50 {
            s.extend_from_slice(format!("\n// line {i}\n").as_bytes());
        }
        s.extend_from_slice(b"[1,2,\n\n3,]");
        s
    });
    v.push(b"[1,\n2,\n3,\n4,\n]".to_vec());
    v.push(b"\n\n\n   {\"a\" 1}".to_vec());

    v
}

#[derive(PartialEq)]
struct LoadOut {
    ok: bool,
    dump: Option<Vec<u8>>,
    err: Vec<u8>,
    err_dbg: String,
}

impl std::fmt::Debug for LoadOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadOut")
            .field("ok", &self.ok)
            .field(
                "dump",
                &self.dump.as_ref().map(|d| String::from_utf8_lossy(d).into_owned()),
            )
            .field("err", &self.err_dbg)
            .finish()
    }
}

unsafe fn finish(l: &Lib, v: *mut JsonT, e: &JsonError) -> LoadOut {
    let out = LoadOut {
        ok: !v.is_null(),
        dump: if v.is_null() { None } else { dump(l, v, DUMP_FLAGS) },
        err: e.raw(),
        err_dbg: format!("{:?} code={}", e, e.text[159] as u8),
    };
    if !v.is_null() {
        let del: Symbol<FnJsonDelete> = l.sym("json_delete");
        del(v);
    }
    out
}

#[test]
fn json_loadb_matches() {
    let (c, r) = seed_both();
    let fc: Symbol<FnJsonLoadb> = c.sym("json_loadb");
    let fr: Symbol<FnJsonLoadb> = r.sym("json_loadb");
    unsafe {
        for input in inputs() {
            for flags in load_flags() {
                let mut ec = JsonError {
                    line: 111,
                    column: 222,
                    position: 333,
                    source: [0x41; JSON_ERROR_SOURCE_LENGTH],
                    text: [0x42; JSON_ERROR_TEXT_LENGTH],
                };
                let mut er = ec;
                let a = fc(input.as_ptr() as *const c_char, input.len(), flags, &mut ec);
                let b = fr(input.as_ptr() as *const c_char, input.len(), flags, &mut er);
                assert_eq!(
                    finish(c, a, &ec),
                    finish(r, b, &er),
                    "json_loadb({:?}, {flags:#x})",
                    String::from_utf8_lossy(&input)
                );
            }
        }
        // NULL error pointer must be tolerated
        for flags in load_flags() {
            let s = b"{\"a\":1}";
            let a = fc(s.as_ptr() as *const c_char, s.len(), flags, std::ptr::null_mut());
            let b = fr(s.as_ptr() as *const c_char, s.len(), flags, std::ptr::null_mut());
            assert_eq!(a.is_null(), b.is_null(), "loadb NULL error, {flags:#x}");
            let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
            let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
            if !a.is_null() {
                dc(a);
            }
            if !b.is_null() {
                dr(b);
            }
        }
        // NULL buffer
        for flags in load_flags() {
            let mut ec = JsonError::default();
            let mut er = JsonError::default();
            let a = fc(std::ptr::null(), 0, flags, &mut ec);
            let b = fr(std::ptr::null(), 0, flags, &mut er);
            assert_eq!(
                finish(c, a, &ec),
                finish(r, b, &er),
                "json_loadb(NULL, 0, {flags:#x})"
            );
        }
    }
}

#[test]
fn json_loads_matches() {
    let (c, r) = seed_both();
    let fc: Symbol<FnJsonLoads> = c.sym("json_loads");
    let fr: Symbol<FnJsonLoads> = r.sym("json_loads");
    unsafe {
        for input in inputs() {
            // json_loads needs a NUL-terminated string
            let z = match std::ffi::CString::new(input.clone()) {
                Ok(z) => z,
                Err(_) => continue,
            };
            for flags in load_flags() {
                let mut ec = JsonError {
                    line: 7,
                    column: 8,
                    position: 9,
                    source: [0x31; JSON_ERROR_SOURCE_LENGTH],
                    text: [0x32; JSON_ERROR_TEXT_LENGTH],
                };
                let mut er = ec;
                let a = fc(z.as_ptr(), flags, &mut ec);
                let b = fr(z.as_ptr(), flags, &mut er);
                assert_eq!(
                    finish(c, a, &ec),
                    finish(r, b, &er),
                    "json_loads({:?}, {flags:#x})",
                    String::from_utf8_lossy(&input)
                );
            }
        }
        for flags in load_flags() {
            let mut ec = JsonError::default();
            let mut er = JsonError::default();
            let a = fc(std::ptr::null(), flags, &mut ec);
            let b = fr(std::ptr::null(), flags, &mut er);
            assert_eq!(
                finish(c, a, &ec),
                finish(r, b, &er),
                "json_loads(NULL, {flags:#x})"
            );
        }
    }
}

// ------------------------------------------------------------ load_callback

struct Src {
    data: Vec<u8>,
    pos: usize,
    /// return -1 on the nth call
    fail_on: usize,
    calls: usize,
    /// hand out at most this many bytes per call
    chunk: usize,
}

unsafe extern "C" fn src_cb(buf: *mut c_char, buflen: usize, data: *mut c_void) -> usize {
    let s = &mut *(data as *mut Src);
    s.calls += 1;
    if s.calls == s.fail_on {
        return usize::MAX; // (size_t)-1 signals an error to load.c
    }
    let n = buflen.min(s.chunk).min(s.data.len() - s.pos);
    if n > 0 {
        std::ptr::copy_nonoverlapping(s.data[s.pos..].as_ptr(), buf as *mut u8, n);
        s.pos += n;
    }
    n
}

#[test]
fn json_load_callback_matches() {
    let (c, r) = seed_both();
    let fc: Symbol<FnJsonLoadCallback> = c.sym("json_load_callback");
    let fr: Symbol<FnJsonLoadCallback> = r.sym("json_load_callback");
    unsafe {
        for input in inputs() {
            for &chunk in &[1usize, 2, 3, 7, 1024, usize::MAX] {
                for flags in [0usize, JSON_DECODE_ANY, JSON_DECODE_ANY | JSON_ALLOW_NUL, 0x1f] {
                    let mut sa = Src {
                        data: input.clone(),
                        pos: 0,
                        fail_on: 0,
                        calls: 0,
                        chunk,
                    };
                    let mut sb = Src {
                        data: input.clone(),
                        pos: 0,
                        fail_on: 0,
                        calls: 0,
                        chunk,
                    };
                    let mut ec = JsonError::default();
                    let mut er = JsonError::default();
                    let a = fc(
                        src_cb as *mut c_void,
                        &mut sa as *mut Src as *mut c_void,
                        flags,
                        &mut ec,
                    );
                    let b = fr(
                        src_cb as *mut c_void,
                        &mut sb as *mut Src as *mut c_void,
                        flags,
                        &mut er,
                    );
                    assert_eq!(
                        finish(c, a, &ec),
                        finish(r, b, &er),
                        "json_load_callback({:?}, chunk {chunk}, {flags:#x})",
                        String::from_utf8_lossy(&input)
                    );
                    assert_eq!(
                        sa.calls, sb.calls,
                        "json_load_callback({:?}, chunk {chunk}, {flags:#x}) call count",
                        String::from_utf8_lossy(&input)
                    );
                }
            }
        }
        // failing source callback
        for fail_on in [1usize, 2, 3] {
            for input in [
                &b"{\"a\":1}"[..],
                b"[1,2,3,4,5,6,7,8,9,10]",
                b"\"a string that is long enough to need several reads\"",
            ] {
                let mut sa = Src {
                    data: input.to_vec(),
                    pos: 0,
                    fail_on,
                    calls: 0,
                    chunk: 4,
                };
                let mut sb = Src {
                    data: input.to_vec(),
                    pos: 0,
                    fail_on,
                    calls: 0,
                    chunk: 4,
                };
                let mut ec = JsonError::default();
                let mut er = JsonError::default();
                let a = fc(
                    src_cb as *mut c_void,
                    &mut sa as *mut Src as *mut c_void,
                    JSON_DECODE_ANY,
                    &mut ec,
                );
                let b = fr(
                    src_cb as *mut c_void,
                    &mut sb as *mut Src as *mut c_void,
                    JSON_DECODE_ANY,
                    &mut er,
                );
                assert_eq!(
                    finish(c, a, &ec),
                    finish(r, b, &er),
                    "failing source (call {fail_on}) on {:?}",
                    String::from_utf8_lossy(input)
                );
                assert_eq!(sa.calls, sb.calls, "failing source call count");
            }
        }
        // NULL callback
        let mut ec = JsonError::default();
        let mut er = JsonError::default();
        let a = fc(std::ptr::null_mut(), std::ptr::null_mut(), 0, &mut ec);
        let b = fr(std::ptr::null_mut(), std::ptr::null_mut(), 0, &mut er);
        assert_eq!(
            finish(c, a, &ec),
            finish(r, b, &er),
            "json_load_callback(NULL)"
        );
    }
}

// ----------------------------------------------------------------- fd, file

#[test]
fn json_loadfd_matches() {
    let (c, r) = seed_both();
    let fc: Symbol<FnJsonLoadfd> = c.sym("json_loadfd");
    let fr: Symbol<FnJsonLoadfd> = r.sym("json_loadfd");
    let dir = std::env::temp_dir();
    let p = dir.join(format!("jansson_loadfd_{}.json", std::process::id()));
    unsafe {
        for input in inputs() {
            std::fs::write(&p, &input).unwrap();
            for flags in [0usize, JSON_DECODE_ANY, JSON_DECODE_ANY | JSON_ALLOW_NUL, 0x1f] {
                let mut ec = JsonError::default();
                let mut er = JsonError::default();
                let f1 = std::fs::File::open(&p).unwrap();
                let a = fc(std::os::fd::AsRawFd::as_raw_fd(&f1), flags, &mut ec);
                drop(f1);
                let f2 = std::fs::File::open(&p).unwrap();
                let b = fr(std::os::fd::AsRawFd::as_raw_fd(&f2), flags, &mut er);
                drop(f2);
                assert_eq!(
                    finish(c, a, &ec),
                    finish(r, b, &er),
                    "json_loadfd({:?}, {flags:#x})",
                    String::from_utf8_lossy(&input)
                );
            }
        }
        // bad fd
        for flags in [0usize, JSON_DECODE_ANY] {
            let mut ec = JsonError::default();
            let mut er = JsonError::default();
            let a = fc(-1, flags, &mut ec);
            let b = fr(-1, flags, &mut er);
            assert_eq!(finish(c, a, &ec), finish(r, b, &er), "json_loadfd(-1)");
        }
    }
    let _ = std::fs::remove_file(&p);
}

#[test]
fn json_load_file_matches() {
    let (c, r) = seed_both();
    let fc: Symbol<FnJsonLoadFile> = c.sym("json_load_file");
    let fr: Symbol<FnJsonLoadFile> = r.sym("json_load_file");
    let dir = std::env::temp_dir();
    // The file name ends up in error->source, so use the *same* path for both.
    let p = dir.join(format!("jansson_loadfile_{}.json", std::process::id()));
    let z = std::ffi::CString::new(p.to_str().unwrap()).unwrap();
    unsafe {
        for input in inputs() {
            std::fs::write(&p, &input).unwrap();
            for flags in [0usize, JSON_DECODE_ANY, JSON_DECODE_ANY | JSON_ALLOW_NUL, 0x1f] {
                let mut ec = JsonError::default();
                let mut er = JsonError::default();
                let a = fc(z.as_ptr(), flags, &mut ec);
                let b = fr(z.as_ptr(), flags, &mut er);
                assert_eq!(
                    finish(c, a, &ec),
                    finish(r, b, &er),
                    "json_load_file({:?}, {flags:#x})",
                    String::from_utf8_lossy(&input)
                );
            }
        }
        // missing file (and a very long path, to hit error source truncation)
        for path in [
            "/definitely/not/here.json".to_string(),
            format!("/tmp/{}.json", "n".repeat(200)),
        ] {
            let z = std::ffi::CString::new(path.clone()).unwrap();
            let mut ec = JsonError::default();
            let mut er = JsonError::default();
            let a = fc(z.as_ptr(), 0, &mut ec);
            let b = fr(z.as_ptr(), 0, &mut er);
            assert_eq!(
                finish(c, a, &ec),
                finish(r, b, &er),
                "json_load_file({path:?})"
            );
        }
    }
    let _ = std::fs::remove_file(&p);
}

#[test]
fn json_loadf_matches() {
    let (c, r) = seed_both();
    type FnLoadf = unsafe extern "C" fn(*mut c_void, usize, *mut JsonError) -> *mut JsonT;
    extern "C" {
        fn fopen(p: *const c_char, m: *const c_char) -> *mut c_void;
        fn fclose(f: *mut c_void) -> c_int;
    }
    let fc: Symbol<FnLoadf> = c.sym("json_loadf");
    let fr: Symbol<FnLoadf> = r.sym("json_loadf");
    let dir = std::env::temp_dir();
    let p = dir.join(format!("jansson_loadf_{}.json", std::process::id()));
    let z = std::ffi::CString::new(p.to_str().unwrap()).unwrap();
    let mode = cs("rb");
    unsafe {
        for input in inputs() {
            std::fs::write(&p, &input).unwrap();
            for flags in [0usize, JSON_DECODE_ANY, JSON_DECODE_ANY | JSON_ALLOW_NUL, 0x1f] {
                let mut ec = JsonError::default();
                let mut er = JsonError::default();
                let f1 = fopen(z.as_ptr(), mode.as_ptr());
                let a = fc(f1, flags, &mut ec);
                fclose(f1);
                let f2 = fopen(z.as_ptr(), mode.as_ptr());
                let b = fr(f2, flags, &mut er);
                fclose(f2);
                assert_eq!(
                    finish(c, a, &ec),
                    finish(r, b, &er),
                    "json_loadf({:?}, {flags:#x})",
                    String::from_utf8_lossy(&input)
                );
            }
        }
    }
    let _ = std::fs::remove_file(&p);
}

#[test]
fn round_trip_dump_then_load_matches() {
    // Dump each library's value and re-load it in *both* libraries; the results
    // must be identical, which cross-checks the encoder against the decoder.
    let (c, r) = seed_both();
    unsafe {
        let lc: Symbol<FnJsonLoads> = c.sym("json_loads");
        let lr: Symbol<FnJsonLoads> = r.sym("json_loads");
        for input in inputs() {
            let z = match std::ffi::CString::new(input.clone()) {
                Ok(z) => z,
                Err(_) => continue,
            };
            let mut ec = JsonError::default();
            let mut er = JsonError::default();
            let a = lc(z.as_ptr(), JSON_DECODE_ANY, &mut ec);
            let b = lr(z.as_ptr(), JSON_DECODE_ANY, &mut er);
            if a.is_null() {
                assert!(b.is_null());
                continue;
            }
            let da = dump(c, a, DUMP_FLAGS).unwrap();
            let db = dump(r, b, DUMP_FLAGS).unwrap();
            assert_eq!(da, db, "first dump of {:?}", String::from_utf8_lossy(&input));

            // re-load the dumped text on both sides and dump again
            let z2 = std::ffi::CString::new(da.clone()).unwrap();
            let mut ec2 = JsonError::default();
            let mut er2 = JsonError::default();
            let a2 = lc(z2.as_ptr(), JSON_DECODE_ANY, &mut ec2);
            let b2 = lr(z2.as_ptr(), JSON_DECODE_ANY, &mut er2);
            assert_eq!(
                finish(c, a2, &ec2),
                finish(r, b2, &er2),
                "reload of {:?}",
                String::from_utf8_lossy(&da)
            );
            let dc_: Symbol<FnJsonDelete> = c.sym("json_delete");
            let dr_: Symbol<FnJsonDelete> = r.sym("json_delete");
            dc_(a);
            dr_(b);
        }
    }
}
