//! Phase B/C — `pcre2_pattern_convert_8` differential tests across every
//! conversion type, glob option, and context setting.
mod common;

use common::diff::*;
use common::*;
use std::ffi::c_void;

const SEED: u64 = 0x5EED_C0DE_1111;

/// Convert `pattern` in one library. Returns `(rc, converted_bytes, blen)`.
/// On success the buffer is freed with `converted_pattern_free`.
unsafe fn convert_in(
    api: &Api,
    pattern: &[u8],
    plen: usize,
    options: u32,
    cctx: *mut c_void,
) -> (i32, Option<Vec<u8>>, usize) {
    let mut buf: *mut u8 = std::ptr::null_mut();
    let mut blen: usize = 0xDEAD_BEEF;
    let rc = (api.pattern_convert)(
        pattern.as_ptr(),
        plen,
        options,
        &mut buf,
        &mut blen,
        cctx,
    );
    if rc == 0 && !buf.is_null() {
        let v = std::slice::from_raw_parts(buf, blen).to_vec();
        (api.converted_pattern_free)(buf);
        (rc, Some(v), blen)
    } else {
        (rc, None, blen)
    }
}

/// Compare a conversion between C and Rust.
unsafe fn assert_convert_eq(
    pattern: &[u8],
    plen: usize,
    options: u32,
    glob_sep: Option<u32>,
    glob_esc: Option<u32>,
    label: &str,
) {
    let (c, r) = both();
    let mut ctxs = [std::ptr::null_mut(), std::ptr::null_mut()];
    let need_ctx = glob_sep.is_some() || glob_esc.is_some();
    let mut out = Vec::new();
    for (i, api) in [c, r].iter().enumerate() {
        let cctx = if need_ctx {
            let cx = (api.convert_context_create)(std::ptr::null_mut());
            assert!(!cx.is_null());
            if let Some(s) = glob_sep {
                (api.set_glob_separator)(cx, s);
            }
            if let Some(e) = glob_esc {
                (api.set_glob_escape)(cx, e);
            }
            cx
        } else {
            std::ptr::null_mut()
        };
        ctxs[i] = cctx;
        out.push(convert_in(api, pattern, plen, options, cctx));
    }
    let (crc, cbuf, cblen) = &out[0];
    let (rrc, rbuf, rblen) = &out[1];
    assert_eq!(
        crc, rrc,
        "{}: convert rc differs (C={} Rust={}) pattern={:?} options={:#x}",
        label, crc, rrc, String::from_utf8_lossy(pattern), options
    );
    // *bufflenptr is written on success (length) AND on error (error offset)
    assert_eq!(
        cblen, rblen,
        "{}: bufflenptr differs (rc={}) pattern={:?} options={:#x}",
        label, crc, String::from_utf8_lossy(pattern), options
    );
    assert_eq!(
        cbuf, rbuf,
        "{}: converted output differs pattern={:?} options={:#x}\n C={:?}\n R={:?}",
        label,
        String::from_utf8_lossy(pattern),
        options,
        cbuf.as_ref().map(|v| String::from_utf8_lossy(v).to_string()),
        rbuf.as_ref().map(|v| String::from_utf8_lossy(v).to_string()),
    );
    for (i, api) in [c, r].iter().enumerate() {
        if !ctxs[i].is_null() {
            (api.convert_context_free)(ctxs[i]);
        }
    }
    // If the conversion succeeded, the RESULT must itself compile identically.
    if let Some(v) = cbuf {
        let copts = if options & PCRE2_CONVERT_UTF != 0 { PCRE2_UTF } else { 0 };
        let _ = compile_both(v, v.len(), &CompileCfg::new(copts), label);
    }
}

const TYPES: [(&str, u32); 3] = [
    ("POSIX_BASIC", PCRE2_CONVERT_POSIX_BASIC),
    ("POSIX_EXTENDED", PCRE2_CONVERT_POSIX_EXTENDED),
    ("GLOB", PCRE2_CONVERT_GLOB),
];

/// Patterns for the POSIX BRE/ERE converters.
const POSIX_PATTERNS: &[&str] = &[
    "", "a", "abc", "a.c", "a*", "a\\*", "^a", "a$", "^a$", "[abc]", "[^abc]",
    "[a-z]", "[]]", "[^]]", "[[:alpha:]]", "[[:digit:][:space:]]", "[[.a.]]",
    "[[=a=]]", "a\\{2,3\\}", "a{2,3}", "\\(a\\)", "(a)", "a\\|b", "a|b",
    "a\\+", "a+", "a\\?", "a?", "\\.", "\\[", "\\\\", ".*", "..", "a**",
    "\\(a\\)\\1", "(a)\\1", "[a", "[", "]", "*a", "\\", "a\\", "{", "}",
    "((a))", "a{", "a}", "[[:foo:]]", "[[:alpha]]", "\\(", "\\)",
];

/// Patterns for the glob converter.
const GLOB_PATTERNS: &[&str] = &[
    "", "a", "abc", "*", "?", "*.txt", "a?c", "[abc]", "[!abc]", "[a-z]",
    "**", "**/", "a/**/b", "/**", "**/*", "a**b", "***", "[", "]", "[]",
    "[!]", "a[", "\\*", "\\?", "\\[", "\\\\", "\\", "a\\", "/", "//", "a/b",
    "/a/b/c", "*/", "/*", "?/?", "[/]", "[a/b]", ".*", "..", ".", "a.b",
    "{a,b}", "a\nb", "\u{00e9}*", "*\u{20ac}",
];

// =================================================================== tests
#[test]
fn convert_posix_basic_and_extended() {
    unsafe {
        for (name, ty) in TYPES {
            if ty == PCRE2_CONVERT_GLOB {
                continue;
            }
            for pat in POSIX_PATTERNS {
                for extra in [0u32, PCRE2_CONVERT_UTF, PCRE2_CONVERT_NO_UTF_CHECK,
                              PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK] {
                    assert_convert_eq(
                        pat.as_bytes(), pat.len(), ty | extra, None, None,
                        &format!("{} extra={:#x} {:?}", name, extra, pat),
                    );
                }
            }
        }
    }
}

#[test]
fn convert_glob_all_options() {
    unsafe {
        // GLOB, GLOB_NO_WILD_SEPARATOR and GLOB_NO_STARSTAR are supersets of
        // PCRE2_CONVERT_GLOB (0x10 | extra bits), so test each spelling.
        let globs: [(&str, u32); 4] = [
            ("GLOB", PCRE2_CONVERT_GLOB),
            ("GLOB_NO_WILD_SEPARATOR", PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR),
            ("GLOB_NO_STARSTAR", PCRE2_CONVERT_GLOB_NO_STARSTAR),
            (
                "GLOB_NO_WILD_SEPARATOR|GLOB_NO_STARSTAR",
                PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR | PCRE2_CONVERT_GLOB_NO_STARSTAR,
            ),
        ];
        for (name, ty) in globs {
            for pat in GLOB_PATTERNS {
                for extra in [0u32, PCRE2_CONVERT_UTF, PCRE2_CONVERT_NO_UTF_CHECK] {
                    assert_convert_eq(
                        pat.as_bytes(), pat.len(), ty | extra, None, None,
                        &format!("{} extra={:#x} {:?}", name, extra, pat),
                    );
                }
            }
        }
    }
}

/// The glob separator and escape characters are context settings that change
/// the converter's output; sweep every legal value (and the illegal ones).
#[test]
fn convert_glob_separator_and_escape() {
    unsafe {
        // pcre2_set_glob_separator accepts only '/', '\\' and '.'
        // pcre2_set_glob_escape accepts 0, '\\' and '.'  (see pcre2_context.c)
        let seps = [
            b'/' as u32, b'\\' as u32, b'.' as u32,
            // invalid values must be rejected identically
            0, b'a' as u32, b'*' as u32, 0xFFFF_FFFF,
        ];
        let escs = [
            0u32, b'\\' as u32, b'.' as u32,
            b'a' as u32, b'/' as u32, 0xFFFF_FFFF,
        ];
        let (c, r) = both();
        // first: the setters themselves must agree
        for s in seps {
            let cx = (c.convert_context_create)(std::ptr::null_mut());
            let rx = (r.convert_context_create)(std::ptr::null_mut());
            assert_eq!(
                (c.set_glob_separator)(cx, s),
                (r.set_glob_separator)(rx, s),
                "set_glob_separator({}) rc",
                s
            );
            (c.convert_context_free)(cx);
            (r.convert_context_free)(rx);
        }
        for e in escs {
            let cx = (c.convert_context_create)(std::ptr::null_mut());
            let rx = (r.convert_context_create)(std::ptr::null_mut());
            assert_eq!(
                (c.set_glob_escape)(cx, e),
                (r.set_glob_escape)(rx, e),
                "set_glob_escape({}) rc",
                e
            );
            (c.convert_context_free)(cx);
            (r.convert_context_free)(rx);
        }
        // then: the conversions produced under every accepted combination
        for s in [b'/' as u32, b'\\' as u32, b'.' as u32] {
            for e in [0u32, b'\\' as u32, b'.' as u32] {
                for pat in GLOB_PATTERNS {
                    for ty in [
                        PCRE2_CONVERT_GLOB,
                        PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
                        PCRE2_CONVERT_GLOB_NO_STARSTAR,
                    ] {
                        assert_convert_eq(
                            pat.as_bytes(), pat.len(), ty, Some(s), Some(e),
                            &format!("glob sep={} esc={} ty={:#x} {:?}", s, e, ty, pat),
                        );
                    }
                }
            }
        }
    }
}

/// PCRE2_ZERO_TERMINATED and explicit lengths, including truncating lengths.
#[test]
fn convert_lengths() {
    unsafe {
        for pat in GLOB_PATTERNS.iter().chain(POSIX_PATTERNS.iter()) {
            let mut z = pat.as_bytes().to_vec();
            z.push(0);
            for ty in [
                PCRE2_CONVERT_GLOB,
                PCRE2_CONVERT_POSIX_BASIC,
                PCRE2_CONVERT_POSIX_EXTENDED,
            ] {
                assert_convert_eq(
                    &z, PCRE2_ZERO_TERMINATED, ty, None, None,
                    &format!("zeroterm ty={:#x} {:?}", ty, pat),
                );
                // every truncating length, including 0
                for l in 0..=pat.len() {
                    assert_convert_eq(
                        pat.as_bytes(), l, ty, None, None,
                        &format!("len={} ty={:#x} {:?}", l, ty, pat),
                    );
                }
            }
        }
    }
}

/// Randomized fuzz of all three converters.
#[test]
fn convert_randomized() {
    let mut g = Rng::new(SEED);
    let alpha: &[u8] =
        b"ab*?[]!-^$.\\/{},()|+:=0123456789 \t\n\xc3\xa9\xff[:alpha:]";
    unsafe {
        for i in 0..6000 {
            let n = g.below(16) as usize;
            let pat = g.bytes_from(n, alpha);
            let ty = *g.pick(&[
                PCRE2_CONVERT_GLOB,
                PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
                PCRE2_CONVERT_GLOB_NO_STARSTAR,
                PCRE2_CONVERT_POSIX_BASIC,
                PCRE2_CONVERT_POSIX_EXTENDED,
            ]);
            let utf = if g.bool() { PCRE2_CONVERT_UTF } else { 0 };
            let sep = if g.bool() {
                Some(*g.pick(&[b'/' as u32, b'\\' as u32, b'.' as u32]))
            } else {
                None
            };
            let esc = if g.bool() {
                Some(*g.pick(&[0u32, b'\\' as u32, b'.' as u32]))
            } else {
                None
            };
            assert_convert_eq(
                &pat, pat.len(), ty | utf, sep, esc,
                &format!("rand#{} ty={:#x} {:02x?}", i, ty, pat),
            );
        }
    }
}

// ======================================================= Phase C: error paths
/// Every rejection `pcre2_pattern_convert` performs (pcre2_convert.c:1129-1145).
#[test]
fn convert_error_paths() {
    let (c, r) = both();
    unsafe {
        // --- NULL pattern with plength != 0 -> PCRE2_ERROR_NULL
        for plen in [1usize, 5, PCRE2_ZERO_TERMINATED] {
            let mut cb: *mut u8 = std::ptr::null_mut();
            let mut rb: *mut u8 = std::ptr::null_mut();
            let mut cl = 0xAAusize;
            let mut rl = 0xAAusize;
            let crc = (c.pattern_convert)(
                std::ptr::null(), plen, PCRE2_CONVERT_GLOB,
                &mut cb, &mut cl, std::ptr::null_mut(),
            );
            let rrc = (r.pattern_convert)(
                std::ptr::null(), plen, PCRE2_CONVERT_GLOB,
                &mut rb, &mut rl, std::ptr::null_mut(),
            );
            assert_eq!(crc, rrc, "NULL pattern plen={} rc", plen);
            assert_eq!(crc, ERR_NULL, "NULL pattern must give PCRE2_ERROR_NULL");
            assert_eq!(cl, rl, "NULL pattern plen={} error offset", plen);
        }

        // --- NULL pattern with plength == 0 is LEGAL (C substitutes null_str)
        {
            let mut cb: *mut u8 = std::ptr::null_mut();
            let mut rb: *mut u8 = std::ptr::null_mut();
            let mut cl = 0usize;
            let mut rl = 0usize;
            let crc = (c.pattern_convert)(
                std::ptr::null(), 0, PCRE2_CONVERT_GLOB,
                &mut cb, &mut cl, std::ptr::null_mut(),
            );
            let rrc = (r.pattern_convert)(
                std::ptr::null(), 0, PCRE2_CONVERT_GLOB,
                &mut rb, &mut rl, std::ptr::null_mut(),
            );
            assert_eq!(crc, rrc, "NULL pattern plen=0 rc");
            assert_eq!(cl, rl, "NULL pattern plen=0 length");
            if crc == 0 {
                assert_eq!(
                    std::slice::from_raw_parts(cb, cl),
                    std::slice::from_raw_parts(rb, rl),
                    "NULL pattern plen=0 output"
                );
                (c.converted_pattern_free)(cb);
                (r.converted_pattern_free)(rb);
            }
        }

        // --- NULL bufflenptr -> PCRE2_ERROR_NULL
        {
            let pat = b"a";
            let mut cb: *mut u8 = std::ptr::null_mut();
            let mut rb: *mut u8 = std::ptr::null_mut();
            let crc = (c.pattern_convert)(
                pat.as_ptr(), 1, PCRE2_CONVERT_GLOB, &mut cb,
                std::ptr::null_mut(), std::ptr::null_mut(),
            );
            let rrc = (r.pattern_convert)(
                pat.as_ptr(), 1, PCRE2_CONVERT_GLOB, &mut rb,
                std::ptr::null_mut(), std::ptr::null_mut(),
            );
            assert_eq!(crc, rrc, "NULL bufflenptr rc");
            assert_eq!(crc, ERR_NULL);
        }

        // --- bad option words: undefined bits, no type, more than one type
        let bad_opts: [u32; 14] = [
            0,                                      // no type set
            PCRE2_CONVERT_UTF,                      // no type, only UTF
            PCRE2_CONVERT_NO_UTF_CHECK,             // no type
            PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_POSIX_EXTENDED, // two types
            PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_GLOB,           // two types
            PCRE2_CONVERT_POSIX_EXTENDED | PCRE2_CONVERT_GLOB,        // two types
            PCRE2_CONVERT_GLOB | 0x8000_0000,       // undefined bit
            0x8000_0000,
            0x0000_0100,
            0xFFFF_FFFF,
            0x0000_0040,                            // NO_STARSTAR bit without GLOB
            0x0000_0020,                            // NO_WILD_SEPARATOR bit alone
            0x0000_00FF,
            0x1234_5678,
        ];
        for opts in bad_opts {
            let pat = b"abc";
            let mut cb: *mut u8 = std::ptr::null_mut();
            let mut rb: *mut u8 = std::ptr::null_mut();
            let mut cl = 0xAAusize;
            let mut rl = 0xAAusize;
            let crc = (c.pattern_convert)(
                pat.as_ptr(), 3, opts, &mut cb, &mut cl, std::ptr::null_mut(),
            );
            let rrc = (r.pattern_convert)(
                pat.as_ptr(), 3, opts, &mut rb, &mut rl, std::ptr::null_mut(),
            );
            assert_eq!(crc, rrc, "options={:#x} rc differs", opts);
            assert_eq!(cl, rl, "options={:#x} error offset differs", opts);
            if crc == 0 {
                (c.converted_pattern_free)(cb);
                (r.converted_pattern_free)(rb);
            }
        }

        // --- invalid UTF-8 with PCRE2_CONVERT_UTF (and suppressed by NO_UTF_CHECK)
        let bad_utf: [&[u8]; 8] = [
            b"\xff", b"\xc3", b"\xc3\x28", b"\xe2\x80", b"\xf0\x9f\x98",
            b"\x80", b"a\xffb", b"\xed\xa0\x80",
        ];
        for pat in bad_utf {
            for ty in [
                PCRE2_CONVERT_GLOB,
                PCRE2_CONVERT_POSIX_BASIC,
                PCRE2_CONVERT_POSIX_EXTENDED,
            ] {
                // NOTE: PCRE2_CONVERT_NO_UTF_CHECK is deliberately NOT combined
                // with invalid UTF-8 here. PCRE2 documents that suppressing the
                // check on invalid input is undefined behaviour, and the C
                // library itself reads past the end of the buffer and
                // segfaults. Verified: the C `.so` and the Rust `.so` crash
                // identically, so this is invalid usage, not a divergence.
                assert_convert_eq(
                    pat, pat.len(), ty | PCRE2_CONVERT_UTF, None, None,
                    &format!("badutf ty={:#x} {:02x?}", ty, pat),
                );
            }
        }
    }
}

/// `pcre2_converted_pattern_free(NULL)` must be a no-op in both.
#[test]
fn converted_pattern_free_null() {
    let (c, r) = both();
    unsafe {
        (c.converted_pattern_free)(std::ptr::null_mut());
        (r.converted_pattern_free)(std::ptr::null_mut());
    }
}
