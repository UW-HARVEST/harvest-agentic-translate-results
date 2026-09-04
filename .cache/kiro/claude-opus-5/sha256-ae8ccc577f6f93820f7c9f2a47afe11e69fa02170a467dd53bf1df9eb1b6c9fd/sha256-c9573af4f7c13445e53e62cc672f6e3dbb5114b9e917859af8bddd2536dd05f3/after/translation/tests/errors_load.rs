//! Phase C — ERRORS.md rows 163–205 (`load.c`).
//!
//! Every row constructs its exact trigger, calls BOTH libraries, and asserts the
//! same error CODE and the same error struct (line/column/position/source/text),
//! not merely "both returned NULL".  The expected code from ERRORS.md is also
//! asserted against the C, which validates the table itself.
mod common;

use common::*;
use std::io::Write;
use std::os::raw::{c_char, c_void};
use std::os::unix::io::AsRawFd;

/// (row, description, input bytes, flags, expected error code)
fn rows() -> Vec<(u32, &'static str, Vec<u8>, usize, u8)> {
    let mut v: Vec<(u32, &'static str, Vec<u8>, usize, u8)> = vec![
        // row 172: top level not [ or {
        (172, "scalar at top level", b"1".to_vec(), 0, E_INVALID_SYNTAX),
        (172, "string at top level", b"\"s\"".to_vec(), 0, E_INVALID_SYNTAX),
        (172, "true at top level", b"true".to_vec(), 0, E_INVALID_SYNTAX),
        (172, "null at top level", b"null".to_vec(), 0, E_INVALID_SYNTAX),
        // row 173: trailing garbage
        (173, "trailing garbage", b"[] x".to_vec(), 0, E_END_OF_INPUT_EXPECTED),
        (173, "two documents", b"{}{}".to_vec(), 0, E_END_OF_INPUT_EXPECTED),
        (173, "trailing char", b"[1]x".to_vec(), 0, E_END_OF_INPUT_EXPECTED),
        // row 175: invalid token
        (175, "bad identifier", b"[tru]".to_vec(), 0, E_INVALID_SYNTAX),
        (175, "at sign", b"[@]".to_vec(), 0, E_INVALID_SYNTAX),
        (175, "TRUE", b"[TRUE]".to_vec(), 0, E_INVALID_SYNTAX),
        // row 176: unexpected token
        (176, "leading comma", b"[,]".to_vec(), 0, E_INVALID_SYNTAX),
        (176, "colon in array", b"[:]".to_vec(), 0, E_INVALID_SYNTAX),
        (176, "brace in array", b"[}]".to_vec(), 0, E_INVALID_SYNTAX),
        // row 177: \u0000 without JSON_ALLOW_NUL
        (177, "nul in string", b"[\"a\\u0000b\"]".to_vec(), 0, E_NULL_CHARACTER),
        (177, "bare nul escape", b"[\"\\u0000\"]".to_vec(), 0, E_NULL_CHARACTER),
        // row 178: string or '}' expected
        (178, "int key", b"{1:2}".to_vec(), 0, E_INVALID_SYNTAX),
        (178, "bare key", b"{a:2}".to_vec(), 0, E_INVALID_SYNTAX),
        // row 179: NUL byte in key
        (179, "nul in key", b"{\"a\\u0000b\":1}".to_vec(), 0, E_NULL_BYTE_IN_KEY),
        (
            179,
            "nul in key with ALLOW_NUL",
            b"{\"a\\u0000b\":1}".to_vec(),
            JSON_ALLOW_NUL,
            E_NULL_BYTE_IN_KEY,
        ),
        // row 180: duplicate key with JSON_REJECT_DUPLICATES
        (
            180,
            "duplicate key",
            b"{\"a\":1,\"a\":2}".to_vec(),
            JSON_REJECT_DUPLICATES,
            E_DUPLICATE_KEY,
        ),
        (
            180,
            "duplicate key later",
            b"{\"a\":1,\"b\":2,\"a\":3}".to_vec(),
            JSON_REJECT_DUPLICATES,
            E_DUPLICATE_KEY,
        ),
        // row 181: ':' expected
        (181, "missing colon", b"{\"a\" 1}".to_vec(), 0, E_INVALID_SYNTAX),
        (181, "comma instead of colon", b"{\"a\",1}".to_vec(), 0, E_INVALID_SYNTAX),
        // row 182: '}' expected / premature end
        (182, "unterminated object", b"{\"a\":1".to_vec(), 0, E_PREMATURE_END),
        (182, "trailing comma in object", b"{\"a\":1,".to_vec(), 0, E_PREMATURE_END),
        (182, "trailing comma + brace", b"{\"a\":1,}".to_vec(), 0, E_INVALID_SYNTAX),
        // row 183: ']' expected / premature end
        (183, "unterminated array", b"[1".to_vec(), 0, E_PREMATURE_END),
        (183, "trailing comma in array", b"[1,".to_vec(), 0, E_PREMATURE_END),
        (183, "trailing comma + bracket", b"[1,]".to_vec(), 0, E_INVALID_SYNTAX),
        // row 184: EOF inside a string
        (184, "unterminated string", b"[\"abc".to_vec(), 0, E_PREMATURE_END),
        (
            187,
            "unterminated after escape (EOF -> invalid escape, not premature end)",
            b"[\"abc\\".to_vec(),
            0,
            E_INVALID_SYNTAX,
        ),
        // row 185: raw control char
        (185, "control 0x01", b"[\"a\x01b\"]".to_vec(), 0, E_INVALID_SYNTAX),
        (185, "control 0x1f", b"[\"\x1f\"]".to_vec(), 0, E_INVALID_SYNTAX),
        (185, "tab in string", b"[\"a\tb\"]".to_vec(), 0, E_INVALID_SYNTAX),
        // row 186: raw newline
        (186, "newline in string", b"[\"a\nb\"]".to_vec(), 0, E_INVALID_SYNTAX),
        // row 187: bad escape
        (187, "unknown escape", b"[\"\\x\"]".to_vec(), 0, E_INVALID_SYNTAX),
        (187, "escape a", b"[\"\\a\"]".to_vec(), 0, E_INVALID_SYNTAX),
        // row 188: \u with non-hex
        (188, "non-hex escape", b"[\"\\uZZZZ\"]".to_vec(), 0, E_INVALID_SYNTAX),
        (188, "short escape", b"[\"\\u12\"]".to_vec(), 0, E_INVALID_SYNTAX),
        (188, "escape with space", b"[\"\\u 123\"]".to_vec(), 0, E_INVALID_SYNTAX),
        // rows 189,190,191: surrogates
        (189, "lone high surrogate", b"[\"\\ud834\"]".to_vec(), 0, E_INVALID_SYNTAX),
        (190, "high + non-low", b"[\"\\ud834\\u0041\"]".to_vec(), 0, E_INVALID_SYNTAX),
        (190, "high + high", b"[\"\\ud834\\ud834\"]".to_vec(), 0, E_INVALID_SYNTAX),
        (191, "lone low surrogate", b"[\"\\udd1e\"]".to_vec(), 0, E_INVALID_SYNTAX),
        // row 193: leading zeros
        (193, "leading zero", b"[01]".to_vec(), 0, E_INVALID_SYNTAX),
        (193, "negative leading zero", b"[-01]".to_vec(), 0, E_INVALID_SYNTAX),
        // row 194: lone minus
        (194, "lone minus", b"[-]".to_vec(), 0, E_INVALID_SYNTAX),
        (194, "minus letter", b"[-a]".to_vec(), 0, E_INVALID_SYNTAX),
        // row 195: dot without digits
        (195, "trailing dot", b"[1.]".to_vec(), 0, E_INVALID_SYNTAX),
        (195, "dot then exponent", b"[1.e2]".to_vec(), 0, E_INVALID_SYNTAX),
        (195, "leading dot", b"[.1]".to_vec(), 0, E_INVALID_SYNTAX),
        // row 196: exponent without digits
        (196, "bare exponent", b"[1e]".to_vec(), 0, E_INVALID_SYNTAX),
        (196, "exponent plus", b"[1e+]".to_vec(), 0, E_INVALID_SYNTAX),
        (196, "exponent minus", b"[1e-]".to_vec(), 0, E_INVALID_SYNTAX),
        // rows 197,198: integer overflow
        (197, "big positive int", b"[9223372036854775808]".to_vec(), 0, E_NUMERIC_OVERFLOW),
        (
            197,
            "huge positive int",
            b"[123456789012345678901234567890]".to_vec(),
            0,
            E_NUMERIC_OVERFLOW,
        ),
        (198, "big negative int", b"[-9223372036854775809]".to_vec(), 0, E_NUMERIC_OVERFLOW),
        (
            198,
            "huge negative int",
            b"[-123456789012345678901234567890]".to_vec(),
            0,
            E_NUMERIC_OVERFLOW,
        ),
        // row 199: real overflow
        (199, "real overflow", b"[1e999]".to_vec(), 0, E_NUMERIC_OVERFLOW),
        (199, "negative real overflow", b"[-1e999]".to_vec(), 0, E_NUMERIC_OVERFLOW),
        (199, "real overflow 400", b"[1e400]".to_vec(), 0, E_NUMERIC_OVERFLOW),
        // row 200: invalid UTF-8 lead byte
        (200, "0xff lead", b"[\xff]".to_vec(), 0, E_INVALID_UTF8),
        (200, "0x80 lead", b"[\x80]".to_vec(), 0, E_INVALID_UTF8),
        (200, "0xc0 lead", b"[\xc0\x80]".to_vec(), 0, E_INVALID_UTF8),
        (200, "0xf5 lead", b"[\xf5\x80\x80\x80]".to_vec(), 0, E_INVALID_UTF8),
        // row 201: valid lead, bad continuation
        (201, "bad continuation", b"[\"\xc2\x41\"]".to_vec(), 0, E_INVALID_UTF8),
        (201, "surrogate encoded", b"[\"\xed\xa0\x80\"]".to_vec(), 0, E_INVALID_UTF8),
        (201, "overlong 3-byte", b"[\"\xe0\x80\x80\"]".to_vec(), 0, E_INVALID_UTF8),
        // row 205: empty input -> premature end of input
        (205, "empty input", b"".to_vec(), 0, E_PREMATURE_END),
        (205, "whitespace only", b"   ".to_vec(), 0, E_PREMATURE_END),
        (205, "empty with DECODE_ANY", b"".to_vec(), JSON_DECODE_ANY, E_PREMATURE_END),
        // row 204: saved_text longer than 20 chars -> no " near '...'" context
        (
            204,
            "long token",
            b"[abcdefghijklmnopqrstuvwxyz]".to_vec(),
            0,
            E_INVALID_SYNTAX,
        ),
        (
            204,
            "long number",
            b"[1234567890123456789012345.5e]".to_vec(),
            0,
            E_INVALID_SYNTAX,
        ),
    ];
    // row 174: depth limit
    v.push((
        174,
        "depth 2049",
        format!("{}{}", "[".repeat(2049), "]".repeat(2049)).into_bytes(),
        0,
        E_STACK_OVERFLOW,
    ));
    v.push((
        174,
        "depth 2049 objects",
        format!(
            "{}1{}",
            (0..2049).map(|_| "{\"k\":").collect::<String>(),
            "}".repeat(2049)
        )
        .into_bytes(),
        0,
        E_STACK_OVERFLOW,
    ));
    v
}

#[test]
fn e_rows_172_205_json_loads_error_codes() {
    let _g = lock();
    let p = pair();
    unsafe {
        for (row, what, input, flags, expect) in rows() {
            let z = nul_terminated(&input);
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let jc = (p.c.json_loads)(z.as_ptr(), flags, &mut ec);
            let jr = (p.r.json_loads)(z.as_ptr(), flags, &mut er);
            assert!(jc.is_null(), "row {row} ({what}): C unexpectedly succeeded");
            assert!(jr.is_null(), "row {row} ({what}): Rust unexpectedly succeeded");
            assert_eq!(
                ec.code(),
                expect,
                "row {row} ({what}): C code {} != documented {expect}; text={:?}",
                ec.code(),
                ec.text_str()
            );
            assert_eq!(
                ec.snapshot(),
                er.snapshot(),
                "row {row} ({what}): error struct differs\n  C   : code={} line={} col={} pos={} text={:?}\n  Rust: code={} line={} col={} pos={} text={:?}",
                ec.code(),
                ec.line,
                ec.column,
                ec.position,
                ec.text_str(),
                er.code(),
                er.line,
                er.column,
                er.position,
                er.text_str()
            );
            decref(p.c, jc);
            decref(p.r, jr);
        }
    }
}

/// The same rows through every other decode entry point: the error code must be
/// the same and only the `source` field may differ (per entry point).
#[test]
fn e_rows_172_205_across_all_entry_points() {
    let _g = lock();
    let p = pair();
    let libc = libc();
    unsafe {
        for (row, what, input, flags, _expect) in rows() {
            if input.len() > 8192 {
                continue;
            }
            let z = nul_terminated(&input);
            // json_loadb
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let jc = (p.c.json_loadb)(z.as_ptr(), input.len(), flags, &mut ec);
            let jr = (p.r.json_loadb)(z.as_ptr(), input.len(), flags, &mut er);
            assert_eq!(ec.snapshot(), er.snapshot(), "row {row} ({what}) loadb");
            decref(p.c, jc);
            decref(p.r, jr);

            let path = temp_path("errload");
            std::fs::File::create(&path).unwrap().write_all(&input).unwrap();
            let zp = cstr(path.to_str().unwrap());

            // json_loadf
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let fc = (libc.fopen)(zp.as_ptr(), cstr("rb").as_ptr());
            let jc = (p.c.json_loadf)(fc, flags, &mut ec);
            (libc.fclose)(fc);
            let fr = (libc.fopen)(zp.as_ptr(), cstr("rb").as_ptr());
            let jr = (p.r.json_loadf)(fr, flags, &mut er);
            (libc.fclose)(fr);
            assert_eq!(ec.snapshot(), er.snapshot(), "row {row} ({what}) loadf");
            decref(p.c, jc);
            decref(p.r, jr);

            // json_loadfd
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let hc = std::fs::File::open(&path).unwrap();
            let jc = (p.c.json_loadfd)(hc.as_raw_fd(), flags, &mut ec);
            drop(hc);
            let hr = std::fs::File::open(&path).unwrap();
            let jr = (p.r.json_loadfd)(hr.as_raw_fd(), flags, &mut er);
            drop(hr);
            assert_eq!(ec.snapshot(), er.snapshot(), "row {row} ({what}) loadfd");
            decref(p.c, jc);
            decref(p.r, jr);

            // json_load_file
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let jc = (p.c.json_load_file)(zp.as_ptr(), flags, &mut ec);
            let jr = (p.r.json_load_file)(zp.as_ptr(), flags, &mut er);
            assert_eq!(ec.snapshot(), er.snapshot(), "row {row} ({what}) load_file");
            decref(p.c, jc);
            decref(p.r, jr);
            std::fs::remove_file(&path).ok();
        }
    }
}

/* rows 163..169: NULL / invalid arguments to each entry point */

#[test]
fn e_rows_163_169_invalid_arguments() {
    let _g = lock();
    let p = pair();
    unsafe {
        // row 163
        let mut ec = JsonError::zeroed();
        let mut er = JsonError::zeroed();
        let jc = (p.c.json_loads)(std::ptr::null(), 0, &mut ec);
        let jr = (p.r.json_loads)(std::ptr::null(), 0, &mut er);
        assert!(jc.is_null() && jr.is_null());
        assert_eq!(ec.code(), E_INVALID_ARGUMENT);
        assert_eq!(ec.snapshot(), er.snapshot(), "row 163 json_loads(NULL)");
        assert_eq!(ec.source_str(), "<string>");

        // row 164
        let mut ec = JsonError::zeroed();
        let mut er = JsonError::zeroed();
        let jc = (p.c.json_loadb)(std::ptr::null(), 10, 0, &mut ec);
        let jr = (p.r.json_loadb)(std::ptr::null(), 10, 0, &mut er);
        assert!(jc.is_null() && jr.is_null());
        assert_eq!(ec.code(), E_INVALID_ARGUMENT);
        assert_eq!(ec.snapshot(), er.snapshot(), "row 164 json_loadb(NULL)");
        assert_eq!(ec.source_str(), "<buffer>");

        // row 165
        let mut ec = JsonError::zeroed();
        let mut er = JsonError::zeroed();
        let jc = (p.c.json_loadf)(std::ptr::null_mut(), 0, &mut ec);
        let jr = (p.r.json_loadf)(std::ptr::null_mut(), 0, &mut er);
        assert!(jc.is_null() && jr.is_null());
        assert_eq!(ec.code(), E_INVALID_ARGUMENT);
        assert_eq!(ec.snapshot(), er.snapshot(), "row 165 json_loadf(NULL)");

        // row 166
        for fd in [-1i32, -2, i32::MIN] {
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let jc = (p.c.json_loadfd)(fd, 0, &mut ec);
            let jr = (p.r.json_loadfd)(fd, 0, &mut er);
            assert!(jc.is_null() && jr.is_null());
            assert_eq!(ec.code(), E_INVALID_ARGUMENT);
            assert_eq!(ec.snapshot(), er.snapshot(), "row 166 json_loadfd({fd})");
        }
        // a valid but closed / non-readable fd behaves as EOF, not invalid arg
        let mut ec = JsonError::zeroed();
        let mut er = JsonError::zeroed();
        let jc = (p.c.json_loadfd)(99999, 0, &mut ec);
        let jr = (p.r.json_loadfd)(99999, 0, &mut er);
        assert!(jc.is_null() && jr.is_null());
        assert_eq!(ec.snapshot(), er.snapshot(), "json_loadfd(bad fd)");

        // row 167
        let mut ec = JsonError::zeroed();
        let mut er = JsonError::zeroed();
        let jc = (p.c.json_load_file)(std::ptr::null(), 0, &mut ec);
        let jr = (p.r.json_load_file)(std::ptr::null(), 0, &mut er);
        assert!(jc.is_null() && jr.is_null());
        assert_eq!(ec.code(), E_INVALID_ARGUMENT);
        assert_eq!(ec.snapshot(), er.snapshot(), "row 167 json_load_file(NULL)");

        // row 168: fopen failure — the text embeds strerror(errno)
        for path in [
            "/definitely/does/not/exist/at/all.json",
            "",
            "/proc/self/nonexistent",
            "/tmp",
        ] {
            let zp = cstr(path);
            let mut ec = JsonError::zeroed();
            let mut er = JsonError::zeroed();
            let jc = (p.c.json_load_file)(zp.as_ptr(), 0, &mut ec);
            let jr = (p.r.json_load_file)(zp.as_ptr(), 0, &mut er);
            assert_eq!(jc.is_null(), jr.is_null(), "load_file({path:?}) null-ness");
            assert_eq!(
                ec.snapshot(),
                er.snapshot(),
                "row 168 json_load_file({path:?}): C text={:?} Rust text={:?}",
                ec.text_str(),
                er.text_str()
            );
            decref(p.c, jc);
            decref(p.r, jr);
        }

        // row 169
        let mut ec = JsonError::zeroed();
        let mut er = JsonError::zeroed();
        let jc = (p.c.json_load_callback)(None, std::ptr::null_mut(), 0, &mut ec);
        let jr = (p.r.json_load_callback)(None, std::ptr::null_mut(), 0, &mut er);
        assert!(jc.is_null() && jr.is_null());
        assert_eq!(ec.code(), E_INVALID_ARGUMENT);
        assert_eq!(ec.snapshot(), er.snapshot(), "row 169 json_load_callback(NULL)");
        assert_eq!(ec.source_str(), "<callback>");

        // NULL error struct on every entry point must not crash
        assert!((p.c.json_loads)(std::ptr::null(), 0, std::ptr::null_mut()).is_null());
        assert!((p.r.json_loads)(std::ptr::null(), 0, std::ptr::null_mut()).is_null());
        assert!((p.c.json_loadb)(std::ptr::null(), 0, 0, std::ptr::null_mut()).is_null());
        assert!((p.r.json_loadb)(std::ptr::null(), 0, 0, std::ptr::null_mut()).is_null());
        assert!((p.c.json_loadf)(std::ptr::null_mut(), 0, std::ptr::null_mut()).is_null());
        assert!((p.r.json_loadf)(std::ptr::null_mut(), 0, std::ptr::null_mut()).is_null());
        assert!((p.c.json_loadfd)(-1, 0, std::ptr::null_mut()).is_null());
        assert!((p.r.json_loadfd)(-1, 0, std::ptr::null_mut()).is_null());
        assert!((p.c.json_load_file)(std::ptr::null(), 0, std::ptr::null_mut()).is_null());
        assert!((p.r.json_load_file)(std::ptr::null(), 0, std::ptr::null_mut()).is_null());
        assert!(
            (p.c.json_load_callback)(None, std::ptr::null_mut(), 0, std::ptr::null_mut())
                .is_null()
        );
        assert!(
            (p.r.json_load_callback)(None, std::ptr::null_mut(), 0, std::ptr::null_mut())
                .is_null()
        );
    }
}

/* rows 170,171: callback returning 0 / (size_t)-1 */

static mut MODE: u32 = 0;
static mut CALLS: usize = 0;

unsafe extern "C" fn cb(buf: *mut c_void, buflen: usize, _d: *mut c_void) -> usize {
    unsafe {
        CALLS += 1;
        match MODE {
            0 => 0,                 // immediate EOF
            1 => usize::MAX,        // (size_t)-1
            2 => {
                // one good chunk, then (size_t)-1
                if CALLS == 1 {
                    let s = b"[1,2";
                    let n = s.len().min(buflen);
                    std::ptr::copy_nonoverlapping(s.as_ptr(), buf as *mut u8, n);
                    n
                } else {
                    usize::MAX
                }
            }
            _ => {
                // one good chunk, then 0
                if CALLS == 1 {
                    let s = b"{\"a\":";
                    let n = s.len().min(buflen);
                    std::ptr::copy_nonoverlapping(s.as_ptr(), buf as *mut u8, n);
                    n
                } else {
                    0
                }
            }
        }
    }
}

#[test]
fn e_rows_170_171_callback_eof_variants() {
    let _g = lock();
    let p = pair();
    unsafe {
        for mode in 0..4u32 {
            for flags in [0usize, JSON_DECODE_ANY, JSON_DISABLE_EOF_CHECK] {
                let mut res = Vec::new();
                for api in [p.c, p.r] {
                    MODE = mode;
                    CALLS = 0;
                    let mut e = JsonError::zeroed();
                    let j = (api.json_load_callback)(
                        Some(cb),
                        std::ptr::null_mut(),
                        flags,
                        &mut e,
                    );
                    res.push((j.is_null(), e.snapshot(), CALLS, dumps(api, j, JSON_ENCODE_ANY)));
                    decref(api, j);
                }
                assert_eq!(res[0], res[1], "callback mode={mode} flags={flags:#x}");
                if mode <= 1 {
                    // rows 170/171: immediate EOF is a premature end of input
                    assert_eq!(res[0].1 .4[JSON_ERROR_TEXT_LENGTH - 1], E_PREMATURE_END);
                }
            }
        }
        MODE = 0;
    }
}

#[allow(unused)]
fn _u(_: *const c_char) {}
