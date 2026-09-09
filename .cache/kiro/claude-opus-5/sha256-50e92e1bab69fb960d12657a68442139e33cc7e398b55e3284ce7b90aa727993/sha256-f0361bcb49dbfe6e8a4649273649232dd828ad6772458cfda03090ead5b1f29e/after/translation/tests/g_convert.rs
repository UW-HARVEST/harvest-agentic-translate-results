//! CONFIGS.md rows 136–146 — `pcre2_pattern_convert` for POSIX BRE/ERE and glob
//! conversion, across every option and convert-context setting.
mod common;
use common::corpus::*;
use common::*;
use std::ffi::c_int;

fn z(p: &[u8]) -> Vec<u8> {
    let mut v = p.to_vec();
    v.push(0);
    v
}

const GLOBS: &[&str] = &[
    "", "*", "?", "**", "***", "a", "a*", "*a", "a?b", "[abc]", "[!abc]", "[^abc]",
    "[a-z]", "[]a]", "[!]a]", "[", "]", "[a", "[a-", "a/b", "a/*/b", "a/**/b", "**/a",
    "a/**", "/**/", "a\\*b", "\\", "\\\\", "\\*", "a.b", "a\\b", ".*", "..", ".", "/",
    "//", "a//b", "*.txt", "**.txt", "src/**/*.rs", "[[:alpha:]]", "{a,b}", "a{b}c",
    "~", "!", "#", "$", "%", "&", "'", "(", ")", "+", ",", "-", ":", ";", "<", "=", ">",
    "@", "^", "_", "`", "|", "}", "{", "\u{00e9}", "\u{4e2d}*", "a\u{0301}b",
];

const POSIX: &[&str] = &[
    "", "a", "abc", "a*", "a\\*", "^a", "a$", "^a$", ".", ".*", "[abc]", "[^abc]",
    "[a-z]", "[]a]", "[^]a]", "[[:alpha:]]", "[[:foo:]]", "[", "[a", "a\\", "\\",
    "\\(a\\)", "(a)", "a\\{2\\}", "a{2}", "a\\{2,3\\}", "a{2,3}", "a{", "a{2", "a{2,",
    "a{,2}", "a{2,3", "a|b", "a\\|b", "a+", "a\\+", "a?", "a\\?", "\\1", "\\9",
    "\\<word\\>", "\\bword\\b", "\\w", "\\W", "\\s", "\\S", "a\\nb", "\\.", "]", "*",
    "^*", "\\{", "\\}", "a**", "\\(\\)", "((a))", "a{1,2}{3}", "\u{00e9}", "\u{4e2d}+",
    "[\u{00e9}-\u{4e2d}]", "a\\(b", "a\\)b", "[a-", "[-a]",
];

const CONV_TYPES: &[(&str, u32)] = &[
    ("POSIX_BASIC", PCRE2_CONVERT_POSIX_BASIC),
    ("POSIX_EXTENDED", PCRE2_CONVERT_POSIX_EXTENDED),
    ("GLOB", PCRE2_CONVERT_GLOB),
    ("GLOB_NO_WILD_SEPARATOR", PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR),
    ("GLOB_NO_STARSTAR", PCRE2_CONVERT_GLOB_NO_STARSTAR),
];

/// Compare `pcre2_pattern_convert` between both libraries.
fn cmp_conv(pat: &[u8], plen: Sz, opts: u32, ctx: (*mut Ctx, *mut Ctx), note: &str) {
    let (c, r) = pair();
    let p = z(pat);
    // (a) library-allocated buffer
    let mut cb: *mut u8 = std::ptr::null_mut();
    let mut rb: *mut u8 = std::ptr::null_mut();
    let mut cl: Sz = 0;
    let mut rl: Sz = 0;
    let crc = unsafe { (c.pattern_convert)(p.as_ptr(), plen, opts, &mut cb, &mut cl, ctx.0) };
    let rrc = unsafe { (r.pattern_convert)(p.as_ptr(), plen, opts, &mut rb, &mut rl, ctx.1) };
    assert_eq!(
        (crc, cl),
        (rrc, rl),
        "pattern_convert rc/len [{note}]\n  pattern={:?} raw={pat:02x?} plen={plen} opts=0x{opts:08x}",
        String::from_utf8_lossy(pat)
    );
    if crc == 0 {
        let cs = unsafe { std::slice::from_raw_parts(cb, cl + 1) };
        let rs = unsafe { std::slice::from_raw_parts(rb, rl + 1) };
        assert_eq!(
            cs, rs,
            "pattern_convert output [{note}]\n  pattern={:?} opts=0x{opts:08x}\n  C   ={:?}\n  RUST={:?}",
            String::from_utf8_lossy(pat),
            String::from_utf8_lossy(&cs[..cl]),
            String::from_utf8_lossy(&rs[..rl])
        );
        // the converted pattern must compile identically too
        let mut ce: c_int = 0;
        let mut co: Sz = 0;
        let mut re: c_int = 0;
        let mut ro: Sz = 0;
        let ccode = unsafe {
            (c.compile)(cb, cl, 0, &mut ce, &mut co, std::ptr::null_mut())
        };
        let rcode = unsafe {
            (r.compile)(rb, rl, 0, &mut re, &mut ro, std::ptr::null_mut())
        };
        assert_eq!(
            (ce, co, ccode.is_null()),
            (re, ro, rcode.is_null()),
            "converted pattern compile [{note}]"
        );
        if !ccode.is_null() {
            assert_eq!(
                unsafe { serialize_bytes(c, ccode) },
                unsafe { serialize_bytes(r, rcode) },
                "converted pattern serialized [{note}]"
            );
            unsafe { (c.code_free)(ccode) };
            unsafe { (r.code_free)(rcode) };
        }
        // (b) caller-supplied buffer of exactly the needed size, one less, one more
        for extra in [0usize, 1, 8] {
            let cap = cl + extra;
            let mut cbuf = vec![0xAAu8; cap + 8];
            let mut rbuf = vec![0xAAu8; cap + 8];
            let mut cbp = cbuf.as_mut_ptr();
            let mut rbp = rbuf.as_mut_ptr();
            let mut ccap: Sz = cap;
            let mut rcap: Sz = cap;
            let ci = unsafe {
                (c.pattern_convert)(p.as_ptr(), plen, opts, &mut cbp, &mut ccap, ctx.0)
            };
            let ri = unsafe {
                (r.pattern_convert)(p.as_ptr(), plen, opts, &mut rbp, &mut rcap, ctx.1)
            };
            assert_eq!((ci, ccap), (ri, rcap), "convert user buffer(cap={cap}) [{note}]");
            assert_eq!(cbuf, rbuf, "convert user buffer bytes(cap={cap}) [{note}]");
        }
        if cl > 0 {
            let cap = cl - 1;
            let mut cbuf = vec![0xAAu8; cap + 8];
            let mut rbuf = vec![0xAAu8; cap + 8];
            let mut cbp = cbuf.as_mut_ptr();
            let mut rbp = rbuf.as_mut_ptr();
            let mut ccap: Sz = cap;
            let mut rcap: Sz = cap;
            let ci = unsafe {
                (c.pattern_convert)(p.as_ptr(), plen, opts, &mut cbp, &mut ccap, ctx.0)
            };
            let ri = unsafe {
                (r.pattern_convert)(p.as_ptr(), plen, opts, &mut rbp, &mut rcap, ctx.1)
            };
            assert_eq!((ci, ccap), (ri, rcap), "convert short buffer [{note}]");
        }
        unsafe { (c.converted_pattern_free)(cb) };
        unsafe { (r.converted_pattern_free)(rb) };
    }
}

#[test]
fn rows136_142_each_convert_type() {
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    for &(name, ty) in CONV_TYPES {
        for extra in [
            0u32,
            PCRE2_CONVERT_UTF,
            PCRE2_CONVERT_NO_UTF_CHECK,
            PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK,
        ] {
            let opts = ty | extra;
            for pat in GLOBS.iter().chain(POSIX.iter()).chain(PATTERNS.iter()) {
                cmp_conv(
                    pat.as_bytes(),
                    pat.len(),
                    opts,
                    null_ctx,
                    &format!("{name} extra=0x{extra:x}"),
                );
                cmp_conv(
                    pat.as_bytes(),
                    PCRE2_ZERO_TERMINATED,
                    opts,
                    null_ctx,
                    &format!("{name} extra=0x{extra:x} zero-term"),
                );
            }
            // invalid UTF-8 inputs (checked path only; NO_UTF_CHECK + invalid is UB)
            if extra & PCRE2_CONVERT_NO_UTF_CHECK == 0 {
                for pat in RAW_SUBJECTS {
                    cmp_conv(pat, pat.len(), opts, null_ctx, &format!("{name} raw"));
                }
            }
        }
    }
}

#[test]
fn rows143_144_glob_separator_and_escape() {
    let (c, r) = pair();
    let cc = unsafe { (c.convert_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.convert_context_create)(std::ptr::null_mut()) };
    assert!(!cc.is_null() && !rc.is_null());
    let seps: [u32; 3] = [b'/' as u32, b'\\' as u32, b'.' as u32];
    // The escape must be 0 or a member of the C `globpunct` set.
    let escs: [u32; 8] = [
        0,
        b'\\' as u32,
        b'!' as u32,
        b'~' as u32,
        b'#' as u32,
        b'^' as u32,
        b'|' as u32,
        b'-' as u32,
    ];
    for &sep in &seps {
        for &esc in &escs {
            assert_eq!(
                unsafe { (c.set_glob_separator)(cc, sep) },
                unsafe { (r.set_glob_separator)(rc, sep) },
                "set_glob_separator({sep})"
            );
            assert_eq!(
                unsafe { (c.set_glob_escape)(cc, esc) },
                unsafe { (r.set_glob_escape)(rc, esc) },
                "set_glob_escape({esc})"
            );
            for ty in [
                PCRE2_CONVERT_GLOB,
                PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
                PCRE2_CONVERT_GLOB_NO_STARSTAR,
            ] {
                for extra in [0u32, PCRE2_CONVERT_UTF] {
                    for pat in GLOBS {
                        cmp_conv(
                            pat.as_bytes(),
                            pat.len(),
                            ty | extra,
                            (cc, rc),
                            &format!("glob sep={sep} esc={esc} ty=0x{ty:x}"),
                        );
                    }
                }
            }
        }
    }
    // a copy of the configured context must behave identically
    let cc2 = unsafe { (c.convert_context_copy)(cc) };
    let rc2 = unsafe { (r.convert_context_copy)(rc) };
    for pat in GLOBS {
        cmp_conv(pat.as_bytes(), pat.len(), PCRE2_CONVERT_GLOB, (cc2, rc2), "glob ctx copy");
    }
    unsafe { (c.convert_context_free)(cc2) };
    unsafe { (r.convert_context_free)(rc2) };
    unsafe { (c.convert_context_free)(cc) };
    unsafe { (r.convert_context_free)(rc) };
    unsafe { (c.converted_pattern_free)(std::ptr::null_mut()) };
    unsafe { (r.converted_pattern_free)(std::ptr::null_mut()) };
}

#[test]
fn row_convert_random() {
    let mut rng = Rng::new(SEED ^ 0xC0);
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    let alphabet: &[u8] = b"ab*?[]!^-\\/.{},|()+$@~ \t\n\x00\xff\xc3\xa9";
    for _ in 0..iters(60000) {
        let n = rng.below(14);
        let pat: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
        let ty = rng.pick(CONV_TYPES).1;
        let extra = *rng.pick(&[
            0u32,
            PCRE2_CONVERT_UTF,
            PCRE2_CONVERT_NO_UTF_CHECK,
            PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK,
        ]);
        if extra & PCRE2_CONVERT_NO_UTF_CHECK != 0 && std::str::from_utf8(&pat).is_err() {
            continue; // NO_UTF_CHECK + invalid UTF is undefined in the C
        }
        cmp_conv(&pat, pat.len(), ty | extra, null_ctx, "convert random");
    }
}
