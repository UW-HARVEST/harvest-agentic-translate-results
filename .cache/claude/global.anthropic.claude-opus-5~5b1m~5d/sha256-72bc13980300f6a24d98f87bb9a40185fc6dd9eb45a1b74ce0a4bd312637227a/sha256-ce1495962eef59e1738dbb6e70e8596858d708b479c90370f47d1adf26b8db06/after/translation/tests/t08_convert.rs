//! Phase B — `pcre2_pattern_convert`, `pcre2_converted_pattern_free` and the
//! convert-context accessors (`pcre2_set_glob_separator`,
//! `pcre2_set_glob_escape`, `pcre2_convert_context_create/_copy/_free`).
//!
//! CONFIGS.md rows 43, 44, 166-176 · ERRORS.md rows 218-228.
//!
//! Every observation crosses the FFI boundary twice (once into the C
//! `libpcre2.so`, once into the Rust one) and the two logs must be identical.
#![allow(non_snake_case)]

mod common;
use common::corpus::*;
use common::*;
use std::ffi::c_void;
#[allow(unused_imports)]
use std::os::raw::c_int;

// ------------------------------------------------------------------ constants

/// `ALL_OPTIONS` from pcre2_convert.c: UTF|NO_UTF_CHECK|GLOB_NO_WILD_SEPARATOR
/// |GLOB_NO_STARSTAR|GLOB|POSIX_BASIC|POSIX_EXTENDED.
const CONVERT_ALL_OPTIONS: u32 = PCRE2_CONVERT_UTF
    | PCRE2_CONVERT_NO_UTF_CHECK
    | PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR
    | PCRE2_CONVERT_GLOB_NO_STARSTAR
    | PCRE2_CONVERT_GLOB
    | PCRE2_CONVERT_POSIX_BASIC
    | PCRE2_CONVERT_POSIX_EXTENDED;

/// The three legal glob separators accepted by `pcre2_set_glob_separator`.
const SEPARATORS: [u32; 3] = [b'/' as u32, b'\\' as u32, b'.' as u32];

/// A representative slice of the escape values accepted by
/// `pcre2_set_glob_escape` (0 disables escaping).
const ESCAPES: [u32; 8] = [
    0,
    b'\\' as u32,
    b'!' as u32,
    b'^' as u32,
    b'%' as u32,
    b'~' as u32,
    b'@' as u32,
    b'`' as u32,
];

/// Every ASCII punctuation character, i.e. the `globpunct` string in
/// pcre2_context.c, plus values that must be rejected.
const ALL_PUNCT: &[u8] = b"!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

/// Subjects fed to the *converted* pattern so that the semantics of the
/// conversion — not just its bytes — are compared.
const MATCH_SUBJECTS: &[&str] = &[
    "",
    "a",
    "ab",
    "abc",
    "a/b",
    "a/b/c",
    "/a",
    "a/",
    "a.b",
    "a\\b",
    "x",
    "aaa",
    "[a]",
    "foo/bar/baz",
    "..",
    "a-b",
    "A",
    "0",
];

static EMPTY_PAT: [u8; 1] = [0];

unsafe fn pat_ptr(pat: &[u8]) -> *const u8 {
    if pat.is_empty() {
        // An empty Rust slice yields a dangling pointer (0x1); PCRE2 requires a
        // readable pointer even for a zero length, so hand it a real byte.
        EMPTY_PAT.as_ptr()
    } else {
        pat.as_ptr()
    }
}

// ------------------------------------------------------------------ the probe

/// Exercises `pcre2_pattern_convert` in *all three* buffer modes described in
/// pcre2_convert.c lines 1171-1229 and logs every byte it produces:
///
/// * mode A — `buffptr == NULL`: dummy run, only `*bufflenptr` is written.
/// * mode B — `*buffptr == NULL`: the library allocates (two internal passes).
/// * mode C — `*buffptr != NULL`: caller buffer of `*bufflenptr` code units,
///   swept from far-too-small up to comfortably large.
///
/// When `deep` is set the converted pattern is additionally compiled and
/// matched, so that a semantic difference is caught even if both libraries
/// happened to agree on nothing else.
unsafe fn conv_probe(
    api: &Api,
    pat: &[u8],
    plen: Sz,
    options: u32,
    sep: Option<u32>,
    esc: Option<u32>,
    deep: bool,
    l: &mut Log,
) {
    // A convert context is only created when a non-default separator/escape is
    // wanted; otherwise NULL is passed so that PRIV(default_convert_context)
    // is used (pcre2_convert.c line 1147).
    let cvc: CvContext = if sep.is_some() || esc.is_some() {
        let c = (api.convert_context_create)(std::ptr::null_mut());
        assert!(!c.is_null(), "{}: convert_context_create failed", api.name);
        if let Some(s) = sep {
            l.tag("gs").i((api.set_glob_separator)(c, s) as i64);
        }
        if let Some(e) = esc {
            l.tag("ge").i((api.set_glob_escape)(c, e) as i64);
        }
        c
    } else {
        std::ptr::null_mut()
    };

    let p = pat_ptr(pat);

    // ---- mode A: length query only.
    let mut blen: Sz = 0xDEAD_BEEF;
    let rca = (api.pattern_convert)(p, plen, options, std::ptr::null_mut(), &mut blen, cvc);
    l.tag("A").i(rca as i64).u(blen as u64);

    // ---- mode B: the library allocates the output buffer.
    let mut buf: *mut u8 = std::ptr::null_mut();
    let mut blenb: Sz = 0xDEAD_BEEF;
    let rcb = (api.pattern_convert)(p, plen, options, &mut buf, &mut blenb, cvc);
    l.tag("B").i(rcb as i64).u(blenb as u64).i(buf.is_null() as i64);
    let mut out: Vec<u8> = Vec::new();
    if !buf.is_null() {
        if rcb == 0 {
            // The converted pattern is always zero terminated; include the
            // terminator in the comparison.
            out = std::slice::from_raw_parts(buf, blenb + 1).to_vec();
            l.b(&out);
        }
        (api.converted_pattern_free)(buf);
    }

    // ---- mode C: caller-supplied buffer, sizes around the required length.
    let mut sizes: Vec<usize> = vec![0, 1, 2, 8];
    if rca == 0 {
        sizes.push(blen / 2);
        sizes.push(blen);
        sizes.push(blen + 1);
        sizes.push(blen + 2);
        sizes.push(blen + 9);
    }
    let cap = sizes.iter().copied().max().unwrap_or(0) + 16;
    let mut store: Vec<u8> = vec![0u8; cap];
    let base = store.as_mut_ptr();
    for sz in sizes {
        std::ptr::write_bytes(base, 0, cap);
        let mut bl: Sz = sz;
        let mut bp: *mut u8 = base;
        let rc = (api.pattern_convert)(p, plen, options, &mut bp, &mut bl, cvc);
        // Never log `bp` itself, only whether it still points at our buffer.
        l.tag("C")
            .u(sz as u64)
            .i(rc as i64)
            .u(bl as u64)
            .i((bp == base) as i64);
        let show = (sz + 4).min(cap);
        l.b(&store[..show]);
    }

    // ---- feed the conversion into the compiler and the matcher.
    if deep && rcb == 0 && !out.is_empty() {
        let copt = if (options & PCRE2_CONVERT_UTF) != 0 {
            PCRE2_UTF
        } else {
            0
        };
        let code = compile_logged(api, &out[..blenb], blenb, copt, std::ptr::null_mut(), l);
        if !code.is_null() {
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            for s in MATCH_SUBJECTS {
                let rc = (api.do_match)(
                    code,
                    pat_ptr(s.as_bytes()),
                    s.len(),
                    0,
                    0,
                    md,
                    std::ptr::null_mut(),
                );
                log_match_result(api, md, rc, l);
            }
            (api.match_data_free)(md);
            (api.code_free)(code);
        }
    }

    if !cvc.is_null() {
        (api.convert_context_free)(cvc);
    }
}

fn diff_conv(
    label: &str,
    pat: &[u8],
    plen: Sz,
    options: u32,
    sep: Option<u32>,
    esc: Option<u32>,
    deep: bool,
) {
    diff(label, |api| {
        let mut l = Log::new();
        unsafe { conv_probe(api, pat, plen, options, sep, esc, deep, &mut l) };
        l
    });
}

// ------------------------------------------------------------------ generators

/// Random POSIX basic/extended regular expressions. Deliberately includes
/// syntax that the converter must reject (trailing `\`, unterminated `[`).
fn gen_posix(rng: &mut Rng) -> Vec<u8> {
    let alphabet: &[u8] = b"abcz.*[]^$\\(){}|+?-:/019AZ,~!<>= \t";
    let n = if rng.below(4) == 0 { rng.below(30) } else { rng.below(13) };
    (0..n).map(|_| *rng.pick(alphabet)).collect()
}

/// Same, but with valid multi-byte UTF-8 characters mixed in, so that the
/// `GETCHARLENTEST` path inside `convert_posix` is exercised. Only *valid*
/// UTF-8 is produced, because `PCRE2_CONVERT_NO_UTF_CHECK` promises validity.
fn gen_posix_utf(rng: &mut Rng) -> Vec<u8> {
    const WIDE: &[char] = &[
        '\u{e9}', '\u{fc}', '\u{80}', '\u{7ff}', '\u{800}', '\u{4e00}', '\u{ffff}',
        '\u{10000}', '\u{1F600}', '\u{10FFFF}', '\u{85}', '\u{2028}',
    ];
    let alphabet: &[u8] = b"abc.*[]^$\\(){}|+?-:/0";
    let n = rng.below(10);
    let mut v: Vec<u8> = Vec::new();
    for _ in 0..n {
        if rng.below(3) == 0 {
            let mut b = [0u8; 4];
            v.extend_from_slice(rng.pick(WIDE).encode_utf8(&mut b).as_bytes());
        } else {
            v.push(*rng.pick(alphabet));
        }
    }
    v
}

/// Random glob patterns built from glob tokens, so that `**`, ranges and
/// POSIX classes appear reasonably often.
fn gen_glob(rng: &mut Rng) -> Vec<u8> {
    const TOKS: &[&str] = &[
        "a", "b", "z", "*", "**", "?", "[", "]", "!", "^", "-", "\\", "/", ".", "{", "}", "(",
        ")", ":", "[:alpha:]", "[:digit:]", "[:xdigit:]", "0", "1", "x", "**/", "/**", "[a-z]",
        "[!a]", "[^a]", "[]]", "[a", "\\*", "\\/", "//", "***", "a-", "]", "[a-", "[[:", " ",
    ];
    let n = if rng.below(4) == 0 { rng.below(14) } else { rng.below(7) };
    let mut v: Vec<u8> = Vec::new();
    for _ in 0..n {
        v.extend_from_slice(rng.pick(TOKS).as_bytes());
    }
    v
}

// ---------------------------------------------------- rows 166-168: POSIX BRE

#[test]
fn convert_posix_basic_random() {
    let mut rng = Rng::new(0x0801_0001);
    for iter in 0..120000 {
        let pat = if iter % 4 == 3 {
            gen_posix_utf(&mut rng)
        } else {
            gen_posix(&mut rng)
        };
        for extra in [0u32, PCRE2_CONVERT_UTF, PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK] {
            diff_conv(
                &format!("bre iter={iter} pat={pat:?} extra={extra:#x}"),
                &pat,
                pat.len(),
                PCRE2_CONVERT_POSIX_BASIC | extra,
                None,
                None,
                iter < 12000,
            );
        }
    }
}

#[test]
fn convert_posix_extended_random() {
    let mut rng = Rng::new(0x0801_0002);
    for iter in 0..120000 {
        let pat = if iter % 4 == 3 {
            gen_posix_utf(&mut rng)
        } else {
            gen_posix(&mut rng)
        };
        for extra in [0u32, PCRE2_CONVERT_UTF, PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK] {
            diff_conv(
                &format!("ere iter={iter} pat={pat:?} extra={extra:#x}"),
                &pat,
                pat.len(),
                PCRE2_CONVERT_POSIX_EXTENDED | extra,
                None,
                None,
                iter < 12000,
            );
        }
    }
}

/// The hand-written pattern corpus run through both POSIX converters, with the
/// conversion compiled and matched (rows 166-168).
#[test]
fn convert_posix_corpus() {
    for (i, p) in PATTERNS.iter().enumerate() {
        for ty in [PCRE2_CONVERT_POSIX_BASIC, PCRE2_CONVERT_POSIX_EXTENDED] {
            for extra in [
                0u32,
                PCRE2_CONVERT_UTF,
                PCRE2_CONVERT_NO_UTF_CHECK,
                PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK,
            ] {
                diff_conv(
                    &format!("posixcorpus[{i}]={p:?} ty={ty:#x} extra={extra:#x}"),
                    p.as_bytes(),
                    p.len(),
                    ty | extra,
                    None,
                    None,
                    true,
                );
            }
        }
    }
    // Handwritten POSIX shapes that exercise every branch of convert_posix().
    let extras: &[&str] = &[
        "",
        "a",
        "a*",
        "**",
        "***",
        "^a",
        "a$",
        "^a$",
        "(a)",
        "\\(a\\)",
        "a\\{2\\}",
        "a\\{2,3\\}",
        "a\\|b",
        "a|b",
        "a+b",
        "a?b",
        "[abc]",
        "[^abc]",
        "[]abc]",
        "[^]abc]",
        "[a-z]",
        "[[:alpha:]]",
        "[[:alpha:][:digit:]]",
        "[[:notaclass:]]",
        "[[:al",
        "[abc",
        "[",
        "[]",
        "[^]",
        "\\1",
        "\\9",
        "\\(a\\)\\1",
        "\\.",
        "\\*",
        "\\\\",
        "a.b",
        ".*",
        "$a",
        "a^b",
        "(*)",
        "\\(*\\)",
        "{2}",
        "a{2}",
        "[\\]",
        "[\\\\]",
        "[a\\]b]",
        "\u{e9}",
        "\u{65e5}\u{672c}",
        "[\u{e9}-\u{fc}]",
        "\u{1F600}*",
    ];
    for (i, p) in extras.iter().enumerate() {
        for ty in [PCRE2_CONVERT_POSIX_BASIC, PCRE2_CONVERT_POSIX_EXTENDED] {
            for extra in [
                0u32,
                PCRE2_CONVERT_UTF,
                PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK,
                PCRE2_CONVERT_NO_UTF_CHECK,
            ] {
                diff_conv(
                    &format!("posixextra[{i}]={p:?} ty={ty:#x} extra={extra:#x}"),
                    p.as_bytes(),
                    p.len(),
                    ty | extra,
                    None,
                    None,
                    true,
                );
            }
        }
    }
}

// ------------------------------------------------------- rows 169-173: globs

/// The three glob "type" option sets. `GLOB_NO_WILD_SEPARATOR` (0x30) and
/// `GLOB_NO_STARSTAR` (0x50) both include the `GLOB` bit.
const GLOB_TYPES: [(&str, u32); 4] = [
    ("GLOB", PCRE2_CONVERT_GLOB),
    ("NO_WILD_SEP", PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR),
    ("NO_STARSTAR", PCRE2_CONVERT_GLOB_NO_STARSTAR),
    (
        "NO_WILD_SEP|NO_STARSTAR",
        PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR | PCRE2_CONVERT_GLOB_NO_STARSTAR,
    ),
];

#[test]
fn convert_glob_random() {
    let mut rng = Rng::new(0x0801_0003);
    for iter in 0..90000 {
        let pat = gen_glob(&mut rng);
        for (name, ty) in GLOB_TYPES {
            for extra in [
                0u32,
                PCRE2_CONVERT_UTF,
                PCRE2_CONVERT_NO_UTF_CHECK,
                PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK,
            ] {
                diff_conv(
                    &format!("glob iter={iter} pat={pat:?} ty={name} extra={extra:#x}"),
                    &pat,
                    pat.len(),
                    ty | extra,
                    None,
                    None,
                    iter < 9000,
                );
            }
        }
    }
}

/// Globs crossed with every legal separator and a spread of escape characters
/// (rows 43, 44, 174-176).
#[test]
fn convert_glob_separator_escape_cross() {
    let mut rng = Rng::new(0x0801_0004);
    let pats: Vec<Vec<u8>> = {
        let mut v: Vec<Vec<u8>> = [
            "", "*", "**", "?", "a", "a*", "*a", "a**b", "**/a", "a/**", "a/**/b", "**", "*/*",
            "a?b", "[a-z]", "[!a-z]", "[^a]", "[]]", "[a/b]", "[[:alpha:]]", "[[:digit:]/]",
            "a\\*b", "a\\/b", "\\", "a\\", "*\\*", ".", "..", "a.b", "/", "//", "a//b",
            "[a-", "[", "]", "{a,b}", "(a)", "a+b", "a|b", "a^b", "a$b", "!a", "~a", "%a", "@a",
            "`a", "a**\\/b", "**\\.b", "[.-/]", "[/-9]",
        ]
        .iter()
        .map(|s| s.as_bytes().to_vec())
        .collect();
        for _ in 0..600 {
            v.push(gen_glob(&mut rng));
        }
        v
    };
    for (i, pat) in pats.iter().enumerate() {
        for (name, ty) in GLOB_TYPES {
            for sep in SEPARATORS {
                for esc in ESCAPES {
                    diff_conv(
                        &format!(
                            "globsep[{i}]={pat:?} ty={name} sep={} esc={esc}",
                            sep as u8 as char
                        ),
                        pat,
                        pat.len(),
                        ty,
                        Some(sep),
                        Some(esc),
                        i < 30,
                    );
                }
            }
        }
    }
}

/// Every ASCII punctuation escape value, on a fixed set of globs.
#[test]
fn convert_glob_every_punct_escape() {
    let pats: [&str; 10] = [
        "a*b", "a?b", "[a-z]", "a\\b", "!a!", "~a~", "%a%", "a@b", "`a`", "*a*b*",
    ];
    for (i, pat) in pats.iter().enumerate() {
        diff(&format!("globpunct[{i}]={pat:?}"), |api| {
            let mut l = Log::new();
            unsafe {
                for &e in ALL_PUNCT {
                    for sep in SEPARATORS {
                        for ty in [
                            PCRE2_CONVERT_GLOB,
                            PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
                            PCRE2_CONVERT_GLOB_NO_STARSTAR,
                        ] {
                            conv_probe(
                                api,
                                pat.as_bytes(),
                                pat.len(),
                                ty,
                                Some(sep),
                                Some(e as u32),
                                false,
                                &mut l,
                            );
                        }
                    }
                    // Also with 'a' and 256, which must be rejected and must
                    // therefore leave the previous escape in force.
                    conv_probe(
                        api,
                        pat.as_bytes(),
                        pat.len(),
                        PCRE2_CONVERT_GLOB,
                        None,
                        Some(b'a' as u32),
                        false,
                        &mut l,
                    );
                    conv_probe(
                        api,
                        pat.as_bytes(),
                        pat.len(),
                        PCRE2_CONVERT_GLOB,
                        None,
                        Some(256),
                        false,
                        &mut l,
                    );
                }
            }
            l
        });
    }
}

/// Glob corpus with the conversion compiled and matched.
#[test]
fn convert_glob_corpus_deep() {
    let pats: [&str; 26] = [
        "*", "**", "?", "a", "*.c", "**/*.c", "a/*/b", "a/**/b", "[a-z]*", "[!x]*", "*/", "/*",
        "**a", "a**", "a**b", "?*?", "[]]*", "[[:digit:]]*", "\\*", "a\\?b", "{a}", "a|b",
        "x**/**y", "**/**", "*/**/*", "a.b.c",
    ];
    for (i, p) in pats.iter().enumerate() {
        for (name, ty) in GLOB_TYPES {
            for sep in SEPARATORS {
                diff_conv(
                    &format!("globdeep[{i}]={p:?} ty={name} sep={}", sep as u8 as char),
                    p.as_bytes(),
                    p.len(),
                    ty,
                    Some(sep),
                    Some(b'\\' as u32),
                    true,
                );
            }
        }
    }
}

// ------------------------------- row 44: zero-terminated vs explicit length

#[test]
fn convert_zero_terminated_and_empty() {
    let pats: [&str; 14] = [
        "", "a", "a*b", "[abc]", "**/x", "a\\", "[abc", "a/b", "\u{e9}", "abcdef", "*", "?",
        "\\", "[[:alpha:]]",
    ];
    for (i, p) in pats.iter().enumerate() {
        let mut z = p.as_bytes().to_vec();
        z.push(0);
        for ty in [
            PCRE2_CONVERT_POSIX_BASIC,
            PCRE2_CONVERT_POSIX_EXTENDED,
            PCRE2_CONVERT_GLOB,
            PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
            PCRE2_CONVERT_GLOB_NO_STARSTAR,
        ] {
            // PCRE2_ZERO_TERMINATED must behave exactly like strlen().
            diff_conv(
                &format!("zt[{i}]={p:?} ty={ty:#x}"),
                &z,
                PCRE2_ZERO_TERMINATED,
                ty,
                None,
                None,
                true,
            );
            // Explicit lengths, including 0 (the empty pattern) and truncations.
            for cut in [0usize, 1, p.len() / 2, p.len()] {
                if cut <= p.len() {
                    diff_conv(
                        &format!("cut[{i}]={p:?} ty={ty:#x} cut={cut}"),
                        &z,
                        cut,
                        ty,
                        None,
                        None,
                        true,
                    );
                }
            }
        }
    }
    // The empty pattern in every type, with a genuinely zero-length slice.
    for ty in [
        PCRE2_CONVERT_POSIX_BASIC,
        PCRE2_CONVERT_POSIX_EXTENDED,
        PCRE2_CONVERT_GLOB,
        PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
        PCRE2_CONVERT_GLOB_NO_STARSTAR,
    ] {
        for extra in [0u32, PCRE2_CONVERT_UTF, PCRE2_CONVERT_NO_UTF_CHECK] {
            diff_conv(
                &format!("empty ty={ty:#x} extra={extra:#x}"),
                &[],
                0,
                ty | extra,
                None,
                None,
                true,
            );
        }
    }
}

// ------------------------------------------------ ERRORS 218-220: NULL / options

/// `pattern == NULL` and `bufflenptr == NULL` (pcre2_convert.c lines 1129-1136).
#[test]
fn convert_null_arguments() {
    diff("convert_null", |api| {
        let mut l = Log::new();
        unsafe {
            for ty in [
                PCRE2_CONVERT_POSIX_BASIC,
                PCRE2_CONVERT_POSIX_EXTENDED,
                PCRE2_CONVERT_GLOB,
                0,
                0xFFFF_FFFF,
            ] {
                // pattern == NULL with plength == 0 uses an internal 1-unit
                // buffer and therefore *succeeds* for valid type options.
                for plen in [0usize, 1, 5, PCRE2_ZERO_TERMINATED] {
                    let mut bl: Sz = 0xDEAD_BEEF;
                    let rc = (api.pattern_convert)(
                        std::ptr::null(),
                        plen,
                        ty,
                        std::ptr::null_mut(),
                        &mut bl,
                        std::ptr::null_mut(),
                    );
                    l.tag("np").i(rc as i64).u(bl as u64);

                    let mut buf: *mut u8 = std::ptr::null_mut();
                    let mut bl2: Sz = 0xDEAD_BEEF;
                    let rc2 = (api.pattern_convert)(
                        std::ptr::null(),
                        plen,
                        ty,
                        &mut buf,
                        &mut bl2,
                        std::ptr::null_mut(),
                    );
                    l.tag("npa").i(rc2 as i64).u(bl2 as u64).i(buf.is_null() as i64);
                    if !buf.is_null() {
                        if rc2 == 0 {
                            l.b(std::slice::from_raw_parts(buf, bl2 + 1));
                        }
                        (api.converted_pattern_free)(buf);
                    }
                }

                // bufflenptr == NULL is always PCRE2_ERROR_NULL (-51), even
                // with a perfectly good pattern.
                let rc = (api.pattern_convert)(
                    b"abc".as_ptr(),
                    3,
                    ty,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                l.tag("nb").i(rc as i64);
                let mut buf2: *mut u8 = std::ptr::null_mut();
                let rc = (api.pattern_convert)(
                    b"abc".as_ptr(),
                    3,
                    ty,
                    &mut buf2,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                l.tag("nb2").i(rc as i64).i(buf2.is_null() as i64);
                // Both NULL at once.
                let rc = (api.pattern_convert)(
                    std::ptr::null(),
                    0,
                    ty,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                l.tag("nn").i(rc as i64);
            }
        }
        l
    });
}

/// `PCRE2_ERROR_BADOPTION` (-34): undefined bits, no type bit, several type
/// bits (ERRORS rows 219-221).
#[test]
fn convert_bad_options() {
    diff("convert_badoptions", |api| {
        let mut l = Log::new();
        unsafe {
            let mut cases: Vec<u32> = Vec::new();
            // No conversion-type bit at all.
            cases.push(0);
            cases.push(PCRE2_CONVERT_UTF);
            cases.push(PCRE2_CONVERT_NO_UTF_CHECK);
            cases.push(PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK);
            // Every single undefined bit on its own, and combined with a
            // legal type.
            for b in 0..32u32 {
                let bit = 1u32 << b;
                if (bit & CONVERT_ALL_OPTIONS) == 0 {
                    cases.push(bit);
                    cases.push(bit | PCRE2_CONVERT_GLOB);
                    cases.push(bit | PCRE2_CONVERT_POSIX_BASIC);
                }
            }
            // More than one type bit.
            cases.push(PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_POSIX_EXTENDED);
            cases.push(PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_GLOB);
            cases.push(PCRE2_CONVERT_POSIX_EXTENDED | PCRE2_CONVERT_GLOB);
            cases.push(
                PCRE2_CONVERT_POSIX_BASIC
                    | PCRE2_CONVERT_POSIX_EXTENDED
                    | PCRE2_CONVERT_GLOB,
            );
            cases.push(PCRE2_CONVERT_POSIX_EXTENDED | PCRE2_CONVERT_GLOB_NO_STARSTAR);
            cases.push(PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR);
            // The everything-set value, and the exact legal mask (which still
            // has three type bits, so it is bad too).
            cases.push(0xFFFF_FFFF);
            cases.push(CONVERT_ALL_OPTIONS);
            // GLOB modifier bits without the GLOB bit are impossible because
            // NO_WILD_SEPARATOR/NO_STARSTAR both include it; the raw modifier
            // values 0x20 / 0x40 alone are undefined bits.
            cases.push(0x20);
            cases.push(0x40);
            cases.push(0x60);
            cases.push(0x20 | PCRE2_CONVERT_GLOB);
            cases.push(0x40 | PCRE2_CONVERT_GLOB);

            for opt in cases {
                for pat in [&b""[..], &b"abc"[..], &b"a*b"[..]] {
                    let mut bl: Sz = 0xDEAD_BEEF;
                    let rc = (api.pattern_convert)(
                        pat_ptr(pat),
                        pat.len(),
                        opt,
                        std::ptr::null_mut(),
                        &mut bl,
                        std::ptr::null_mut(),
                    );
                    l.tag("bo").u(opt as u64).i(rc as i64).u(bl as u64);

                    let mut buf: *mut u8 = std::ptr::null_mut();
                    let mut bl2: Sz = 0xDEAD_BEEF;
                    let rc2 = (api.pattern_convert)(
                        pat_ptr(pat),
                        pat.len(),
                        opt,
                        &mut buf,
                        &mut bl2,
                        std::ptr::null_mut(),
                    );
                    l.tag("bo2").i(rc2 as i64).u(bl2 as u64).i(buf.is_null() as i64);
                    if !buf.is_null() {
                        (api.converted_pattern_free)(buf);
                    }
                }
            }
        }
        l
    });
}

// ------------------------------------- ERRORS 222-225: converter-specific errors

/// POSIX: unterminated class -> 106, pattern ending in `\` -> 101.
#[test]
fn convert_posix_error_codes() {
    let cases: &[&str] = &[
        "[abc", "[", "[^", "[]", "[[:alpha:", "[[:", "a[b", "a\\", "\\", "abc\\", "[a]\\",
        "\\\\\\", "[a-", "x[", "[a\\", "[[:alpha:]", "a[[:digit:]",
    ];
    for (i, p) in cases.iter().enumerate() {
        for ty in [PCRE2_CONVERT_POSIX_BASIC, PCRE2_CONVERT_POSIX_EXTENDED] {
            for extra in [0u32, PCRE2_CONVERT_UTF] {
                diff_conv(
                    &format!("posixerr[{i}]={p:?} ty={ty:#x} extra={extra:#x}"),
                    p.as_bytes(),
                    p.len(),
                    ty | extra,
                    None,
                    None,
                    false,
                );
            }
        }
    }
}

/// Glob: `PCRE2_ERROR_CONVERT_SYNTAX` (-64) and missing `]` (106).
#[test]
fn convert_glob_syntax_errors() {
    let cases: &[&str] = &[
        "[", "[a", "[!", "[^", "[z-a]", "[b-a]", "[a-[:digit:]]", "\\", "a\\", "[\\", "[a\\",
        "[[:alpha:]", "[!]", "[^]", "[9-0]", "[z-a", "[a-b-c", "[[:bogus:]]", "[--", "[a--",
        "[/-.]", "[.-/]",
    ];
    for (i, p) in cases.iter().enumerate() {
        for (name, ty) in GLOB_TYPES {
            for sep in SEPARATORS {
                for esc in [0u32, b'\\' as u32, b'!' as u32] {
                    diff_conv(
                        &format!(
                            "globerr[{i}]={p:?} ty={name} sep={} esc={esc}",
                            sep as u8 as char
                        ),
                        p.as_bytes(),
                        p.len(),
                        ty,
                        Some(sep),
                        Some(esc),
                        false,
                    );
                }
            }
        }
    }
}

/// Invalid UTF input with `PCRE2_CONVERT_UTF` and *without* `NO_UTF_CHECK`
/// must return the matching UTF error code and the byte offset (ERRORS 224).
///
/// `NO_UTF_CHECK` is deliberately *not* combined with invalid input: the
/// converters then use the bounds-unaware `GETCHARLENTEST` macro, which is
/// documented undefined behaviour.
#[test]
fn convert_invalid_utf() {
    let mut bad: Vec<Vec<u8>> = byte_subjects();
    bad.extend([
        vec![0x80, 0x80],
        vec![0xC0, 0x80],
        vec![0xE0, 0x80, 0x80],
        vec![0xF8, 0x88, 0x80, 0x80, 0x80],
        vec![0xFC, 0x84, 0x80, 0x80, 0x80, 0x80],
        vec![b'a', b'*', 0xFF, b'b'],
        vec![b'[', 0x80, b']'],
        vec![0xED, 0xBF, 0xBF],
        vec![0xF4, 0x8F, 0xBF, 0xBF],
        vec![0xF5, 0x80, 0x80, 0x80],
    ]);
    for (i, pat) in bad.iter().enumerate() {
        for ty in [
            PCRE2_CONVERT_POSIX_BASIC,
            PCRE2_CONVERT_POSIX_EXTENDED,
            PCRE2_CONVERT_GLOB,
            PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
            PCRE2_CONVERT_GLOB_NO_STARSTAR,
        ] {
            diff_conv(
                &format!("badutf[{i}]={pat:?} ty={ty:#x}"),
                pat,
                pat.len(),
                ty | PCRE2_CONVERT_UTF,
                None,
                None,
                false,
            );
            // Valid-UTF inputs are also run with NO_UTF_CHECK to prove the
            // check is really skipped.
            if std::str::from_utf8(pat).is_ok() {
                diff_conv(
                    &format!("okutf[{i}]={pat:?} ty={ty:#x}"),
                    pat,
                    pat.len(),
                    ty | PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK,
                    None,
                    None,
                    false,
                );
            }
        }
    }
}

// ------------------------------------------------- ERRORS 226: NOMEMORY (-48)

// Thread-local, because libtest runs the tests in this file concurrently and
// they all share these two callback functions.
thread_local! {
    static T_FAIL_AFTER: std::cell::Cell<i64> = const { std::cell::Cell::new(-1) };
    static T_NALLOC: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static T_NFREE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

fn set_fail_after(v: i64) {
    T_FAIL_AFTER.with(|c| c.set(v));
}
fn nalloc() -> u64 {
    T_NALLOC.with(|c| c.get())
}
fn nfree() -> u64 {
    T_NFREE.with(|c| c.get())
}
fn reset_alloc() {
    T_FAIL_AFTER.with(|c| c.set(-1));
    T_NALLOC.with(|c| c.set(0));
    T_NFREE.with(|c| c.set(0));
}

/// A malloc that fails after a configurable number of successes, so that both
/// the `memctl_malloc` failure inside `pcre2_pattern_convert` and the context
/// allocation failure can be reached.
unsafe extern "C" fn counting_malloc(size: usize, _data: *mut c_void) -> *mut c_void {
    let n = T_NALLOC.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    let fa = T_FAIL_AFTER.with(|c| c.get());
    if fa >= 0 && n as i64 > fa {
        return std::ptr::null_mut();
    }
    let total = size + 16;
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    let p = std::alloc::alloc(layout);
    if p.is_null() {
        return std::ptr::null_mut();
    }
    *(p as *mut usize) = total;
    p.add(16) as *mut c_void
}

unsafe extern "C" fn counting_free(block: *mut c_void, _data: *mut c_void) {
    if block.is_null() {
        return;
    }
    T_NFREE.with(|c| c.set(c.get() + 1));
    let p = (block as *mut u8).sub(16);
    let total = *(p as *mut usize);
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    std::alloc::dealloc(p, layout);
}

#[test]
fn convert_out_of_memory() {
    let pats: [&str; 6] = ["a", "a*b", "[abc]", "**/x", "", "abcdefghij"];
    for (i, p) in pats.iter().enumerate() {
        for ty in [
            PCRE2_CONVERT_POSIX_BASIC,
            PCRE2_CONVERT_POSIX_EXTENDED,
            PCRE2_CONVERT_GLOB,
        ] {
            diff(&format!("nomem[{i}]={p:?} ty={ty:#x}"), |api| {
                let mut l = Log::new();
                unsafe {
                    // fail_after == 0 makes even the context allocation fail.
                    for fail_after in [0i64, 1, 2, 3, -1] {
                        reset_alloc();
                        let gc = (api.general_context_create)(
                            Some(counting_malloc),
                            Some(counting_free),
                            std::ptr::null_mut(),
                        );
                        assert!(!gc.is_null());
                        let cvc = (api.convert_context_create)(gc);
                        assert!(!cvc.is_null());
                        // Start counting only from here so the number of
                        // set-up allocations is irrelevant.
                        T_NALLOC.with(|c| c.set(0));
                        set_fail_after(fail_after);

                        let mut buf: *mut u8 = std::ptr::null_mut();
                        let mut bl: Sz = 0xDEAD_BEEF;
                        let rc = (api.pattern_convert)(
                            pat_ptr(p.as_bytes()),
                            p.len(),
                            ty,
                            &mut buf,
                            &mut bl,
                            cvc,
                        );
                        set_fail_after(-1);
                        l.tag("nm")
                            .i(fail_after)
                            .i(rc as i64)
                            .u(bl as u64)
                            .i(buf.is_null() as i64)
                            .u(nalloc());
                        if !buf.is_null() {
                            if rc == 0 {
                                l.b(std::slice::from_raw_parts(buf, bl + 1));
                            }
                            (api.converted_pattern_free)(buf);
                        }
                        (api.convert_context_free)(cvc);
                        (api.general_context_free)(gc);
                    }
                }
                l
            });
        }
    }
}

// ------------------------------- rows 43/44: convert context create/copy/free

#[test]
fn convert_context_lifecycle() {
    diff("convert_context_lifecycle", |api| {
        let mut l = Log::new();
        unsafe {
            // Default context, then a copy: the copy must convert identically.
            let a = (api.convert_context_create)(std::ptr::null_mut());
            l.tag("crt").i(a.is_null() as i64);
            assert!(!a.is_null());
            l.i((api.set_glob_separator)(a, b'.' as u32) as i64);
            l.i((api.set_glob_escape)(a, b'!' as u32) as i64);
            let b = (api.convert_context_copy)(a);
            l.tag("cpy").i(b.is_null() as i64);
            assert!(!b.is_null());

            for ctx in [a, b] {
                for p in ["a*b", "a!b", "x.y", "**/z", "[a-z]"] {
                    let mut buf: *mut u8 = std::ptr::null_mut();
                    let mut bl: Sz = 0;
                    let rc = (api.pattern_convert)(
                        p.as_bytes().as_ptr(),
                        p.len(),
                        PCRE2_CONVERT_GLOB,
                        &mut buf,
                        &mut bl,
                        ctx,
                    );
                    l.i(rc as i64).u(bl as u64);
                    if !buf.is_null() {
                        if rc == 0 {
                            l.b(std::slice::from_raw_parts(buf, bl + 1));
                        }
                        (api.converted_pattern_free)(buf);
                    }
                }
            }
            // Changing the original must not affect the copy.
            l.i((api.set_glob_separator)(a, b'/' as u32) as i64);
            for ctx in [a, b] {
                let mut buf: *mut u8 = std::ptr::null_mut();
                let mut bl: Sz = 0;
                let rc = (api.pattern_convert)(
                    b"a*b/c".as_ptr(),
                    5,
                    PCRE2_CONVERT_GLOB,
                    &mut buf,
                    &mut bl,
                    ctx,
                );
                l.tag("indep").i(rc as i64).u(bl as u64);
                if !buf.is_null() {
                    if rc == 0 {
                        l.b(std::slice::from_raw_parts(buf, bl + 1));
                    }
                    (api.converted_pattern_free)(buf);
                }
            }
            (api.convert_context_free)(b);
            (api.convert_context_free)(a);

            // A context built on a general context with custom allocators.
            reset_alloc();
            let gc = (api.general_context_create)(
                Some(counting_malloc),
                Some(counting_free),
                std::ptr::null_mut(),
            );
            l.tag("gc").i(gc.is_null() as i64);
            let cvc = (api.convert_context_create)(gc);
            l.tag("cvc").i(cvc.is_null() as i64);
            let mut buf: *mut u8 = std::ptr::null_mut();
            let mut bl: Sz = 0;
            let rc = (api.pattern_convert)(
                b"a*b".as_ptr(),
                3,
                PCRE2_CONVERT_GLOB,
                &mut buf,
                &mut bl,
                cvc,
            );
            l.i(rc as i64).u(bl as u64);
            if !buf.is_null() {
                if rc == 0 {
                    l.b(std::slice::from_raw_parts(buf, bl + 1));
                }
                // Must be released through the *same* library that made it.
                (api.converted_pattern_free)(buf);
            }
            (api.convert_context_free)(cvc);
            (api.general_context_free)(gc);
            // The custom allocator must have been used, and everything freed.
            l.tag("bal").u(nalloc()).u(nfree());

            // converted_pattern_free(NULL) is a documented no-op.
            (api.converted_pattern_free)(std::ptr::null_mut());
            (api.convert_context_free)(std::ptr::null_mut());
            l.tag("nullfree").i(0);
        }
        l
    });
}

/// Return codes of `pcre2_set_glob_separator` / `pcre2_set_glob_escape` for
/// the whole 0..=256 range plus a few far-out values, and the observable
/// effect of each accepted value.
#[test]
fn convert_glob_setters_return_codes() {
    diff("glob_setters", |api| {
        let mut l = Log::new();
        unsafe {
            let cvc = (api.convert_context_create)(std::ptr::null_mut());
            assert!(!cvc.is_null());
            for v in 0u32..=256 {
                l.u(v as u64)
                    .i((api.set_glob_separator)(cvc, v) as i64)
                    .i((api.set_glob_escape)(cvc, v) as i64);
            }
            for v in [
                257u32,
                300,
                1000,
                0x10FFFF,
                0xFFFF_FFFE,
                0xFFFF_FFFF,
                b'a' as u32,
                b'Z' as u32,
                b'0' as u32,
                b' ' as u32,
                b'\t' as u32,
            ] {
                l.u(v as u64)
                    .i((api.set_glob_separator)(cvc, v) as i64)
                    .i((api.set_glob_escape)(cvc, v) as i64);
            }
            // After all those calls the context must still hold the last
            // *accepted* values, which we observe through a conversion.
            let mut buf: *mut u8 = std::ptr::null_mut();
            let mut bl: Sz = 0;
            let rc = (api.pattern_convert)(
                b"a*b/c.d\\e".as_ptr(),
                9,
                PCRE2_CONVERT_GLOB,
                &mut buf,
                &mut bl,
                cvc,
            );
            l.tag("after").i(rc as i64).u(bl as u64);
            if !buf.is_null() {
                if rc == 0 {
                    l.b(std::slice::from_raw_parts(buf, bl + 1));
                }
                (api.converted_pattern_free)(buf);
            }
            (api.convert_context_free)(cvc);
        }
        l
    });
}

// ------------------------------------------------------------- caller buffers

/// A dedicated sweep of the caller-supplied-buffer path: for a known required
/// length `n`, buffer sizes `0..n+3` must switch from `PCRE2_ERROR_NOMEMORY`
/// to success at exactly the same point in both libraries, and must write
/// exactly the same bytes.
#[test]
fn convert_caller_buffer_sweep() {
    let cases: &[(&str, u32)] = &[
        ("", PCRE2_CONVERT_POSIX_BASIC),
        ("a", PCRE2_CONVERT_POSIX_BASIC),
        ("abc", PCRE2_CONVERT_POSIX_BASIC),
        ("a*b", PCRE2_CONVERT_POSIX_BASIC),
        ("[abc]", PCRE2_CONVERT_POSIX_BASIC),
        ("\\(a\\)\\1", PCRE2_CONVERT_POSIX_BASIC),
        ("^a$", PCRE2_CONVERT_POSIX_EXTENDED),
        ("(a|b)+", PCRE2_CONVERT_POSIX_EXTENDED),
        ("[[:alpha:]]x", PCRE2_CONVERT_POSIX_EXTENDED),
        ("", PCRE2_CONVERT_GLOB),
        ("a", PCRE2_CONVERT_GLOB),
        ("*", PCRE2_CONVERT_GLOB),
        ("**", PCRE2_CONVERT_GLOB),
        ("a*b", PCRE2_CONVERT_GLOB),
        ("**/*.c", PCRE2_CONVERT_GLOB),
        ("[a-z]*", PCRE2_CONVERT_GLOB),
        ("a?b", PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR),
        ("**x", PCRE2_CONVERT_GLOB_NO_STARSTAR),
        ("[abc", PCRE2_CONVERT_GLOB),
        ("a\\", PCRE2_CONVERT_POSIX_BASIC),
    ];
    for (i, (p, ty)) in cases.iter().enumerate() {
        diff(&format!("bufsweep[{i}]={p:?} ty={ty:#x}"), |api| {
            let mut l = Log::new();
            unsafe {
                // Required length first (dummy run).
                let mut need: Sz = 0;
                let rc0 = (api.pattern_convert)(
                    pat_ptr(p.as_bytes()),
                    p.len(),
                    *ty,
                    std::ptr::null_mut(),
                    &mut need,
                    std::ptr::null_mut(),
                );
                l.tag("need").i(rc0 as i64).u(need as u64);
                let top = if rc0 == 0 { need + 4 } else { 24 };
                let cap = top + 16;
                let mut store = vec![0u8; cap];
                let base = store.as_mut_ptr();
                for sz in 0..=top {
                    std::ptr::write_bytes(base, 0xAA, cap);
                    let mut bl: Sz = sz;
                    let mut bp: *mut u8 = base;
                    let rc = (api.pattern_convert)(
                        pat_ptr(p.as_bytes()),
                        p.len(),
                        *ty,
                        &mut bp,
                        &mut bl,
                        std::ptr::null_mut(),
                    );
                    l.u(sz as u64)
                        .i(rc as i64)
                        .u(bl as u64)
                        .i((bp == base) as i64)
                        .b(&store[..cap]);
                }
            }
            l
        });
    }
}

// --------------------------------------------- full byte range, non-UTF mode

/// Random byte strings over the *whole* 0..=255 range, converted in non-UTF
/// mode where every code unit is a character. This sweeps the `strchr`
/// look-ups in `convert_posix`/`convert_glob` (which are guarded by `c < 255`)
/// and the `convert_glob_char_in_class` table probe across all byte values.
#[test]
fn convert_random_bytes() {
    let mut rng = Rng::new(0x0801_0005);
    for iter in 0..40000 {
        let n = rng.below(10);
        let pat: Vec<u8> = (0..n).map(|_| (rng.next_u32() & 0xFF) as u8).collect();
        for ty in [
            PCRE2_CONVERT_POSIX_BASIC,
            PCRE2_CONVERT_POSIX_EXTENDED,
            PCRE2_CONVERT_GLOB,
            PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR,
            PCRE2_CONVERT_GLOB_NO_STARSTAR,
        ] {
            diff_conv(
                &format!("rbytes iter={iter} pat={pat:?} ty={ty:#x}"),
                &pat,
                pat.len(),
                ty,
                None,
                None,
                false,
            );
        }
    }
}

/// The same, but with every glob separator / escape combination, so that the
/// byte-value edge cases interact with the context settings.
#[test]
fn convert_random_bytes_with_context() {
    let mut rng = Rng::new(0x0801_0006);
    for iter in 0..4000 {
        let n = rng.below(8);
        let pat: Vec<u8> = (0..n).map(|_| (rng.next_u32() & 0xFF) as u8).collect();
        for sep in SEPARATORS {
            for esc in ESCAPES {
                for (name, ty) in GLOB_TYPES {
                    diff_conv(
                        &format!("rbytesctx iter={iter} pat={pat:?} ty={name}"),
                        &pat,
                        pat.len(),
                        ty,
                        Some(sep),
                        Some(esc),
                        false,
                    );
                }
            }
        }
    }
}

// ------------------------------------------- ERRORS 218-228: coverage census

/// Collects the *set* of return codes that the whole convert error surface can
/// produce and compares it between the two libraries. The census also proves
/// that each documented error row was actually reached: if a path stopped being
/// exercised, the corresponding code would drop out of the census and the
/// coverage assertion below would fire.
#[test]
fn convert_error_code_census() {
    /// Returns a sorted, de-duplicated list of every rc observed.
    unsafe fn census(api: &Api) -> Vec<i32> {
        let mut seen: Vec<i32> = Vec::new();
        let note = |rc: c_int, seen: &mut Vec<i32>| {
            if !seen.contains(&(rc as i32)) {
                seen.push(rc as i32);
            }
        };

        // -51 NULL
        let mut bl: Sz = 0;
        note(
            (api.pattern_convert)(
                std::ptr::null(),
                4,
                PCRE2_CONVERT_GLOB,
                std::ptr::null_mut(),
                &mut bl,
                std::ptr::null_mut(),
            ),
            &mut seen,
        );
        note(
            (api.pattern_convert)(
                b"a".as_ptr(),
                1,
                PCRE2_CONVERT_GLOB,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            ),
            &mut seen,
        );
        // -34 BADOPTION: undefined bit / no type / several types
        for opt in [
            0x8000_0000u32,
            0,
            PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_GLOB,
            0xFFFF_FFFF,
        ] {
            note(
                (api.pattern_convert)(
                    b"a".as_ptr(),
                    1,
                    opt,
                    std::ptr::null_mut(),
                    &mut bl,
                    std::ptr::null_mut(),
                ),
                &mut seen,
            );
        }
        // UTF errors (negative, from _pcre2_valid_utf)
        for bad in [&[0xFFu8][..], &[0xC2][..], &[0xED, 0xA0, 0x80][..], &[0x80][..]] {
            note(
                (api.pattern_convert)(
                    bad.as_ptr(),
                    bad.len(),
                    PCRE2_CONVERT_GLOB | PCRE2_CONVERT_UTF,
                    std::ptr::null_mut(),
                    &mut bl,
                    std::ptr::null_mut(),
                ),
                &mut seen,
            );
        }
        // 106 MISSING_SQUARE_BRACKET and 101 END_BACKSLASH from POSIX
        for (pat, ty) in [
            (&b"[abc"[..], PCRE2_CONVERT_POSIX_BASIC),
            (&b"a\\"[..], PCRE2_CONVERT_POSIX_BASIC),
            (&b"[abc"[..], PCRE2_CONVERT_POSIX_EXTENDED),
            (&b"a\\"[..], PCRE2_CONVERT_POSIX_EXTENDED),
            (&b"[abc"[..], PCRE2_CONVERT_GLOB),
        ] {
            note(
                (api.pattern_convert)(
                    pat.as_ptr(),
                    pat.len(),
                    ty,
                    std::ptr::null_mut(),
                    &mut bl,
                    std::ptr::null_mut(),
                ),
                &mut seen,
            );
        }
        // -64 CONVERT_SYNTAX from glob
        for pat in [&b"[z-a]"[..], &b"a\\"[..], &b"[a-[:digit:]]"[..]] {
            note(
                (api.pattern_convert)(
                    pat.as_ptr(),
                    pat.len(),
                    PCRE2_CONVERT_GLOB,
                    std::ptr::null_mut(),
                    &mut bl,
                    std::ptr::null_mut(),
                ),
                &mut seen,
            );
        }
        // -48 NOMEMORY, both from a too-small caller buffer and from a
        // failing allocator.
        let mut small = [0u8; 4];
        let mut bp: *mut u8 = small.as_mut_ptr();
        let mut bl2: Sz = 1;
        note(
            (api.pattern_convert)(
                b"abcdefgh".as_ptr(),
                8,
                PCRE2_CONVERT_GLOB,
                &mut bp,
                &mut bl2,
                std::ptr::null_mut(),
            ),
            &mut seen,
        );
        reset_alloc();
        let gc = (api.general_context_create)(
            Some(counting_malloc),
            Some(counting_free),
            std::ptr::null_mut(),
        );
        let cvc = (api.convert_context_create)(gc);
        T_NALLOC.with(|c| c.set(0));
        set_fail_after(0);
        let mut buf: *mut u8 = std::ptr::null_mut();
        let mut bl3: Sz = 0;
        note(
            (api.pattern_convert)(
                b"abc".as_ptr(),
                3,
                PCRE2_CONVERT_GLOB,
                &mut buf,
                &mut bl3,
                cvc,
            ),
            &mut seen,
        );
        set_fail_after(-1);
        if !buf.is_null() {
            (api.converted_pattern_free)(buf);
        }
        (api.convert_context_free)(cvc);
        (api.general_context_free)(gc);
        // 0 success
        note(
            (api.pattern_convert)(
                b"a*b".as_ptr(),
                3,
                PCRE2_CONVERT_GLOB,
                std::ptr::null_mut(),
                &mut bl,
                std::ptr::null_mut(),
            ),
            &mut seen,
        );
        seen.sort_unstable();
        seen
    }

    diff("convert_error_census", |api| {
        let mut l = Log::new();
        unsafe {
            for rc in census(api) {
                l.i(rc as i64);
            }
        }
        l
    });

    // Coverage self-check against the reference implementation: each of the
    // documented ERRORS.md rows must appear.
    let codes = unsafe { census(&apis().0) };
    if std::env::var("CENSUS").is_ok() {
        eprintln!("convert error codes covered: {codes:?}");
    }
    for want in [
        0,
        ERR_NULL,
        ERR_BADOPTION,
        ERR_NOMEMORY,
        ERR_CONVERT_SYNTAX,
        101, // PCRE2_ERROR_END_BACKSLASH
        106, // PCRE2_ERROR_MISSING_SQUARE_BRACKET
    ] {
        assert!(
            codes.contains(&want),
            "convert error census missing {want}; saw {codes:?}"
        );
    }
    // At least one UTF-8 validity error must have been produced.
    assert!(
        codes.iter().any(|&c| (-24..=-3).contains(&c)),
        "no UTF error in census: {codes:?}"
    );
}
