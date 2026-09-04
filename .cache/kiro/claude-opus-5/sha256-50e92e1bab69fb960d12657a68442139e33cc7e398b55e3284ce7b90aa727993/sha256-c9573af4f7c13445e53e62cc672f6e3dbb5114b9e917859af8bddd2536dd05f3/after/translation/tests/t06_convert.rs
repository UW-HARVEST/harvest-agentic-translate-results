//! Differential tests for `pcre2_pattern_convert_8` (pcre2_convert.c).
//!
//! Every case runs the SAME inputs through the C and the Rust `libpcre2.so`
//! and requires bit-identical observable output: the integer return code,
//! `*bufflenptr`, and the converted bytes (only when the API defines them,
//! i.e. rc == 0). When a conversion succeeds we additionally compile the
//! converted pattern with `pcre2_compile_8` in both libraries and match it
//! against a handful of subjects, comparing compile error, ovector count and
//! ovector contents.
//!
//! The C source branches on:
//!   * option validation: undefined bits, >1 type bit, 0 type bits -> BADOPTION
//!   * NULL pattern / NULL bufflenptr -> NULL   (NULL pattern + len 0 is legal)
//!   * PCRE2_ZERO_TERMINATED length
//!   * UTF validation unless PCRE2_CONVERT_NO_UTF_CHECK
//!   * caller-supplied buffer vs library-allocated buffer (dummy-run + alloc)
//!   * NOMEMORY when a caller buffer is too small
//!   * POSIX BRE/ERE conversion and glob conversion with separator/escape
mod harness;
use harness::*;
use std::os::raw::c_int;

// -------------------------------------------------------------------- helpers

/// The full observable result of one `pcre2_pattern_convert` call in the
/// library-allocated-buffer mode (`*buffptr == NULL`).
#[derive(Debug, PartialEq, Eq, Clone)]
struct ConvOut {
    rc: c_int,
    /// `*bufflenptr` after the call. On success this is the converted length
    /// (code units, excluding the trailing NUL); on error it is the error
    /// offset into the pattern. Both are defined by the API, so both compared.
    blength: Sz,
    /// The converted bytes, INCLUDING the trailing NUL, captured only when
    /// rc == 0 (the only case where the output buffer is defined).
    out: Option<Vec<u8>>,
}

/// Run a conversion in library-allocated mode through one library and capture
/// everything observable, freeing the library-allocated buffer afterwards.
fn conv_alloc(
    api: &Api,
    pattern: *const u8,
    plength: Sz,
    options: u32,
    ctx: Ctx,
) -> ConvOut {
    unsafe {
        let mut buf: *mut u8 = std::ptr::null_mut();
        let mut blen: Sz = 0;
        let rc = (api.pattern_convert)(pattern, plength, options, &mut buf, &mut blen, ctx);
        let out = if rc == 0 {
            // The library allocated a buffer of blen+1 code units and NUL
            // terminated it; capture blen+1 bytes so the terminator is checked.
            let v = if buf.is_null() {
                Vec::new()
            } else {
                std::slice::from_raw_parts(buf, blen + 1).to_vec()
            };
            Some(v)
        } else {
            // rc != 0: the output buffer is undefined -> DO NOT read it.
            None
        };
        if !buf.is_null() {
            // converted_pattern_free is only valid for library-allocated
            // buffers, which is exactly this mode.
            (api.converted_pattern_free)(buf);
        }
        ConvOut { rc, blength: blen, out }
    }
}

/// Result of a caller-supplied-buffer conversion. Here `*buffptr` is pre-set to
/// a caller-owned buffer and `*bufflenptr` to its length. The library performs
/// a single run (no dummy run, no allocation) and must NOT be freed with
/// `converted_pattern_free`.
#[derive(Debug, PartialEq, Eq, Clone)]
struct ConvCaller {
    rc: c_int,
    blength: Sz,
    /// Bytes actually written into the caller buffer, captured only on success
    /// (blen+1 code units, to include the terminator the converter writes).
    written: Option<Vec<u8>>,
}

fn conv_caller(
    api: &Api,
    pattern: *const u8,
    plength: Sz,
    options: u32,
    ctx: Ctx,
    bufsize: Sz,
) -> ConvCaller {
    unsafe {
        // Fill the caller buffer with a marker so we can see exactly what the
        // converter wrote. Allocate one extra guard byte we never hand over.
        let mut backing = vec![0xEEu8; bufsize + 1];
        let mut buf: *mut u8 = backing.as_mut_ptr();
        let mut blen: Sz = bufsize;
        let rc = (api.pattern_convert)(pattern, plength, options, &mut buf, &mut blen, ctx);
        let written = if rc == 0 {
            // On success the converter wrote blen code units plus a trailing
            // NUL, i.e. blen+1 bytes, all within the caller buffer.
            let n = (blen + 1).min(bufsize);
            Some(backing[..n].to_vec())
        } else {
            None
        };
        // NOTE: caller-supplied buffer -> must NOT call converted_pattern_free.
        ConvCaller { rc, blength: blen, written }
    }
}

/// A tiny set of subjects used to check that a *successfully converted* pattern
/// behaves identically once compiled in both libraries.
const CHECK_SUBJECTS: &[&[u8]] = &[
    b"", b"a", b"ab", b"abc", b"a/b", b"a.b", b"foo/bar", b"x", b"/", b".",
    b"a*b", b"aXb", b"path/to/file", b"[a]", b"AAA", b"123", b"z9", b"\\",
];

/// Compile `converted` (a NUL-terminated converted pattern) in `api` and match
/// against a few subjects. Returns (compile_err, compile_off, per-subject
/// (rc, ovector)). Only called when both libraries reported the SAME converted
/// bytes, so any divergence here is a genuine compile/match divergence.
fn compile_and_probe(api: &Api, converted: &[u8]) -> (c_int, Sz, Vec<(c_int, Vec<Sz>)>) {
    unsafe {
        // converted includes a trailing NUL; use PCRE2_ZERO_TERMINATED so the
        // exact length is irrelevant and matches how converted patterns are
        // normally consumed.
        let mut err: c_int = 0;
        let mut off: Sz = 0;
        let code = (api.compile)(
            converted.as_ptr(),
            PCRE2_ZERO_TERMINATED,
            0,
            &mut err,
            &mut off,
            std::ptr::null_mut(),
        );
        if code.is_null() {
            return (err, off, Vec::new());
        }
        let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
        let mut capcount: u32 = 0;
        (api.pattern_info)(code, 4, &mut capcount as *mut u32 as *mut std::ffi::c_void);
        let mut probes = Vec::new();
        for subj in CHECK_SUBJECTS {
            let rc = (api.do_match)(
                code,
                subj.as_ptr(),
                subj.len(),
                0,
                0,
                md,
                std::ptr::null_mut(),
            );
            let n = (api.get_ovector_count)(md);
            let ov = if rc > 0 {
                let pairs = ((capcount + 1) as usize).min(n as usize);
                std::slice::from_raw_parts((api.get_ovector_pointer)(md), pairs * 2).to_vec()
            } else {
                Vec::new()
            };
            probes.push((rc, ov));
        }
        (api.match_data_free)(md);
        (api.code_free)(code);
        (err, off, probes)
    }
}

/// Build a fresh convert context for `api` with an optional glob separator and
/// escape. Returns NULL context when both are `None` (the default-context path
/// in the C source).
fn make_ctx(api: &Api, sep: Option<u32>, esc: Option<u32>) -> Ctx {
    if sep.is_none() && esc.is_none() {
        return std::ptr::null_mut();
    }
    unsafe {
        let vc = (api.convert_context_create)(std::ptr::null_mut());
        assert!(!vc.is_null());
        if let Some(s) = sep {
            (api.set_glob_separator)(vc, s);
        }
        if let Some(e) = esc {
            (api.set_glob_escape)(vc, e);
        }
        vc
    }
}

fn free_ctx(api: &Api, ctx: Ctx) {
    if !ctx.is_null() {
        unsafe { (api.convert_context_free)(ctx) };
    }
}

/// The core differential check for one (pattern, plength, options, context)
/// tuple. Runs BOTH the library-allocated path and (derived from the C length)
/// a set of caller-buffer paths, and, on success, cross-compiles the output.
///
/// `sep`/`esc` describe the convert-context to build FOR EACH library
/// independently (contexts are not shareable across the two .so files).
fn check(
    pattern: &[u8],
    use_zero_terminated: bool,
    options: u32,
    sep: Option<u32>,
    esc: Option<u32>,
    label: &str,
) {
    // For the PCRE2_ZERO_TERMINATED path the library calls strlen() on the
    // pointer, so the buffer MUST be NUL terminated. Build a terminated copy
    // when the caller wants that path; otherwise use the raw slice + length.
    let zt_buf: Vec<u8>;
    let (pptr, plength) = if use_zero_terminated {
        zt_buf = cb(pattern); // appends a trailing NUL
        (zt_buf.as_ptr(), PCRE2_ZERO_TERMINATED)
    } else {
        (pattern.as_ptr(), pattern.len())
    };

    // ---- library-allocated buffer path -----------------------------------
    let cc = c();
    let rr = r();
    let cctx = make_ctx(cc, sep, esc);
    let rctx = make_ctx(rr, sep, esc);

    let co = conv_alloc(cc, pptr, plength, options, cctx);
    let ro = conv_alloc(rr, pptr, plength, options, rctx);

    if co != ro {
        free_ctx(cc, cctx);
        free_ctx(rr, rctx);
        panic!(
            "CONVERT DIVERGENCE (alloc) [{label}]\n  pattern = {:?} (zt={use_zero_terminated})\n  options = {options:#x} sep={sep:?} esc={esc:?}\n  C    = {:?}\n  Rust = {:?}",
            String::from_utf8_lossy(pattern),
            co,
            ro,
        );
    }

    // ---- caller-supplied buffer paths -------------------------------------
    // Use the length the (agreed) allocated run reported to pick buffer sizes
    // that are exact, too-large, and too-small (to trigger NOMEMORY).
    if co.rc == 0 {
        let need = co.blength; // code units, excluding NUL
        // The converter writes need+1 code units (incl NUL). Sizes below the
        // full requirement must yield PCRE2_ERROR_NOMEMORY in both libs.
        let sizes: Vec<Sz> = {
            let mut v = vec![need + 1, need + 8, need + 100];
            if need + 1 > 0 {
                v.push(need); // one short of the terminator -> NOMEMORY
            }
            if need > 0 {
                v.push(need / 2);
                v.push(1);
            }
            v.push(0);
            v
        };
        for &bs in &sizes {
            let cco = conv_caller(cc, pptr, plength, options, cctx, bs);
            let rco = conv_caller(rr, pptr, plength, options, rctx, bs);
            if cco != rco {
                free_ctx(cc, cctx);
                free_ctx(rr, rctx);
                panic!(
                    "CONVERT DIVERGENCE (caller buf, size={bs}) [{label}]\n  pattern = {:?}\n  options = {options:#x} sep={sep:?} esc={esc:?}\n  C    = {:?}\n  Rust = {:?}",
                    String::from_utf8_lossy(pattern),
                    cco,
                    rco,
                );
            }
        }

        // ---- cross-compile the converted output ---------------------------
        // co.out includes the trailing NUL; both libs produced identical bytes
        // (asserted above), so feed the C output to both compilers.
        if let Some(bytes) = &co.out {
            let cprobe = compile_and_probe(cc, bytes);
            let rprobe = compile_and_probe(rr, bytes);
            if cprobe != rprobe {
                free_ctx(cc, cctx);
                free_ctx(rr, rctx);
                panic!(
                    "CONVERTED-PATTERN COMPILE/MATCH DIVERGENCE [{label}]\n  pattern   = {:?}\n  converted = {:?}\n  options   = {options:#x} sep={sep:?} esc={esc:?}\n  C    = {:?}\n  Rust = {:?}",
                    String::from_utf8_lossy(pattern),
                    String::from_utf8_lossy(bytes),
                    cprobe,
                    rprobe,
                );
            }
        }
    }

    free_ctx(cc, cctx);
    free_ctx(rr, rctx);
}

// The five conversion type selectors. NOTE (from the harness):
//   GLOB_NO_WILD_SEPARATOR == 0x30 == GLOB | 0x20
//   GLOB_NO_STARSTAR       == 0x50 == GLOB | 0x40
// so each already carries the GLOB type bit; they are single-type options.
const TYPES: &[(u32, &str)] = &[
    (PCRE2_CONVERT_POSIX_BASIC, "POSIX_BASIC"),
    (PCRE2_CONVERT_POSIX_EXTENDED, "POSIX_EXTENDED"),
    (PCRE2_CONVERT_GLOB, "GLOB"),
    (PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR, "GLOB_NO_WILD_SEPARATOR"),
    (PCRE2_CONVERT_GLOB_NO_STARSTAR, "GLOB_NO_STARSTAR"),
];

// UTF flag combinations layered on top of a type.
const UTF_FLAGS: &[(u32, &str)] = &[
    (0, "noutf"),
    (PCRE2_CONVERT_UTF, "utf"),
    (PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK, "utf|noutfcheck"),
    (PCRE2_CONVERT_NO_UTF_CHECK, "noutfcheck"),
];

// ============================================================================
//  1. Every conversion type x UTF flag over a curated corpus.
// ============================================================================

/// POSIX BRE/ERE patterns plus generic patterns that exercise the POSIX and
/// glob converters' branches.
fn posix_glob_corpus() -> Vec<&'static str> {
    vec![
        // --- trivial / literal ---
        "", "a", "abc", "hello world", "a.b.c", "A-Z",
        // --- POSIX BRE metacharacters ---
        "a*", ".*", "^a", "a$", "^a$", "^.*$", "[abc]", "[^abc]", "[a-z]",
        "[0-9]", "a\\{2,3\\}", "\\(a\\)", "\\(ab\\)*", "a\\|b", "\\(a\\|b\\)",
        "\\.", "\\*", "\\[", "\\]", "\\\\", "a\\.b", "x\\+y", "a\\?b",
        // --- POSIX ERE metacharacters ---
        "a+", "a?", "a{2,3}", "(a)", "(ab)*", "a|b", "(a|b)", "a{2,}", "a{,3}",
        "a{3}", "(a)(b)(c)", "((a))", "a.b", "^(ab|cd)+$", "[[:alpha:]]",
        "[[:digit:]]+", "[[:space:]]*", "[[:alnum:]_]+", "[^[:space:]]",
        "[[:upper:][:lower:]]", "colou?r", "gr[ae]y",
        // --- POSIX character-class edge cases ---
        "[]a]", "[^]a]", "[a-]", "[-a]", "[]]", "[^]]", "[a^]", "[[]", "[a[]",
        "[[:alpha:]a-z]", "[abc", "[[:alpha:]",
        // --- glob patterns ---
        "*", "?", "*.txt", "foo*", "*bar", "a?c", "[abc]def", "[!abc]", "[a-z]*",
        "**", "**/*.c", "a/**/b", "src/**", "**foo", "foo**", "a/*/b", "*/*/*",
        "[[:digit:]]", "\\*literal", "a\\?b", "foo[!/]bar", "a[/]b", "[a/b]",
        "{a,b}", "file.{c,h}", "x!y", "a-b", "路径/*.rs", "*.日本",
        // --- error triggers ---
        "\\", "a\\", "abc\\", "[", "a[", "[[:unknown:]]", "[[.collating.]]",
        "[[=equiv=]]", "[[:alpha]", "[z-a]", "[a-\\]",
    ]
}

#[test]
fn types_x_utf_corpus() {
    for &(ty, tyname) in TYPES {
        for &(uf, ufname) in UTF_FLAGS {
            let options = ty | uf;
            for pat in posix_glob_corpus() {
                let label = format!("{tyname}/{ufname}");
                // Explicit length.
                check(pat.as_bytes(), false, options, None, None, &label);
                // Zero-terminated length path.
                check(pat.as_bytes(), true, options, None, None, &label);
            }
        }
    }
}

// ============================================================================
//  2. Glob separator x escape matrix.
// ============================================================================

#[test]
fn glob_separator_escape_matrix() {
    let seps: &[Option<u32>] = &[
        None,
        Some(b'/' as u32),
        Some(b'\\' as u32),
        Some(b'.' as u32),
    ];
    let escs: &[Option<u32>] = &[
        None,
        Some(0),
        Some(b'\\' as u32),
        Some(b'!' as u32),
    ];
    let globs = [
        "*", "?", "**", "a/b", "a\\b", "a.b", "*.txt", "**/*.c", "src/**/x",
        "[a-z]/*", "foo/bar/**", "a!b", "!x", "[!a]", "path/to/*.rs", "a?b/c",
        "*/*", "[abc]/def", "**foo**", "x\\*y", "a.b.c.*",
    ];
    for &sep in seps {
        for &esc in escs {
            for g in globs {
                for &(ty, tyname) in &[
                    (PCRE2_CONVERT_GLOB, "GLOB"),
                    (PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR, "GLOB_NWS"),
                    (PCRE2_CONVERT_GLOB_NO_STARSTAR, "GLOB_NSS"),
                ] {
                    let label = format!("{tyname} sep={sep:?} esc={esc:?}");
                    check(g.as_bytes(), false, ty, sep, esc, &label);
                }
            }
        }
    }
}

// ============================================================================
//  3. Option-validation and NULL / length edge cases (byte-exact rc/blength).
// ============================================================================

/// Directly compare rc + `*bufflenptr` for edge cases where the pattern/buffer
/// pointers matter (NULL pattern, NULL buffptr, NULL bufflenptr), which the
/// higher-level `check` helper cannot express.
#[test]
fn null_and_option_validation() {
    let mut outs: Vec<Vec<(c_int, i128)>> = Vec::new();
    for api in [c(), r()] {
        unsafe {
            let mut v: Vec<(c_int, i128)> = Vec::new();

            // ---- invalid option bits -> BADOPTION, blength(error offset)=0 --
            let pat = b"abc\0";
            for opt in [
                0u32,                                   // no type
                0xffff_ffff,                            // all bits incl. undefined
                0x8000_0000,                            // single undefined bit
                PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_POSIX_EXTENDED, // two types
                PCRE2_CONVERT_POSIX_BASIC | PCRE2_CONVERT_GLOB,           // two types
                PCRE2_CONVERT_UTF,                      // only UTF, no type
                PCRE2_CONVERT_NO_UTF_CHECK,             // only NO_UTF_CHECK
                // NO_WILD_SEPARATOR bit (0x20) WITHOUT the GLOB type bit:
                0x0000_0020,
                0x0000_0040,
            ] {
                let mut buf: *mut u8 = std::ptr::null_mut();
                let mut blen: Sz = 12345;
                let rc =
                    (api.pattern_convert)(pat.as_ptr(), 3, opt, &mut buf, &mut blen, std::ptr::null_mut());
                if rc == 0 && !buf.is_null() {
                    (api.converted_pattern_free)(buf);
                }
                v.push((rc, blen as i128));
            }

            // ---- NULL bufflenptr -> NULL error -----------------------------
            {
                let mut buf: *mut u8 = std::ptr::null_mut();
                let rc = (api.pattern_convert)(
                    pat.as_ptr(),
                    3,
                    PCRE2_CONVERT_GLOB,
                    &mut buf,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                v.push((rc, -1));
            }

            // ---- NULL buffptr but length required only (buffptr==NULL) -----
            // C: buffptr==NULL means "just the length is required"; a dummy run
            // computes the length and returns 0 with *bufflenptr set.
            {
                let mut blen: Sz = 0;
                let rc = (api.pattern_convert)(
                    pat.as_ptr(),
                    3,
                    PCRE2_CONVERT_GLOB,
                    std::ptr::null_mut(),
                    &mut blen,
                    std::ptr::null_mut(),
                );
                v.push((rc, blen as i128));
            }

            // ---- NULL pattern with zero length -> uses internal null string -
            {
                let mut buf: *mut u8 = std::ptr::null_mut();
                let mut blen: Sz = 0;
                let rc = (api.pattern_convert)(
                    std::ptr::null(),
                    0,
                    PCRE2_CONVERT_GLOB,
                    &mut buf,
                    &mut blen,
                    std::ptr::null_mut(),
                );
                let tail = if rc == 0 && !buf.is_null() {
                    let out = std::slice::from_raw_parts(buf, blen + 1).to_vec();
                    (api.converted_pattern_free)(buf);
                    // fold the bytes into the comparison via a hash-ish sum
                    out.iter().map(|&b| b as i128).sum::<i128>()
                } else {
                    -999
                };
                v.push((rc, blen as i128));
                v.push((rc, tail));
            }

            // ---- NULL pattern with NON-zero length -> NULL error -----------
            {
                let mut buf: *mut u8 = std::ptr::null_mut();
                let mut blen: Sz = 42;
                let rc = (api.pattern_convert)(
                    std::ptr::null(),
                    5,
                    PCRE2_CONVERT_GLOB,
                    &mut buf,
                    &mut blen,
                    std::ptr::null_mut(),
                );
                if rc == 0 && !buf.is_null() {
                    (api.converted_pattern_free)(buf);
                }
                v.push((rc, blen as i128));
            }

            // ---- both pattern NULL and bufflenptr NULL ---------------------
            {
                let mut buf: *mut u8 = std::ptr::null_mut();
                let rc = (api.pattern_convert)(
                    std::ptr::null(),
                    5,
                    PCRE2_CONVERT_GLOB,
                    &mut buf,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                );
                v.push((rc, -1));
            }

            outs.push(v);
        }
    }
    assert_eq!(outs[0], outs[1], "NULL / option-validation rc/blength diverge");
    // Sanity: the invalid-option cases must all be BADOPTION with offset 0.
    for i in 0..9 {
        assert_eq!(outs[0][i].0, PCRE2_ERROR_BADOPTION, "case {i} expected BADOPTION");
        assert_eq!(outs[0][i].1, 0, "case {i} expected error offset 0");
    }
}

// ============================================================================
//  4. Explicit error-trigger corpus (rc + error offset compared exactly).
// ============================================================================

#[test]
fn error_trigger_corpus() {
    // (pattern, type) pairs chosen to hit each rejection branch.
    let cases: &[(&[u8], u32, &str)] = &[
        // POSIX: trailing backslash -> END_BACKSLASH
        (b"a\\", PCRE2_CONVERT_POSIX_BASIC, "bre trailing backslash"),
        (b"\\", PCRE2_CONVERT_POSIX_BASIC, "bre lone backslash"),
        (b"abc\\", PCRE2_CONVERT_POSIX_EXTENDED, "ere trailing backslash"),
        (b"a\\", PCRE2_CONVERT_POSIX_EXTENDED, "ere trailing backslash 2"),
        // POSIX: unterminated class -> MISSING_SQUARE_BRACKET
        (b"[", PCRE2_CONVERT_POSIX_BASIC, "bre open class"),
        (b"[abc", PCRE2_CONVERT_POSIX_EXTENDED, "ere open class"),
        (b"[[:alpha:]", PCRE2_CONVERT_POSIX_BASIC, "bre class no close"),
        (b"[[:alpha:]]", PCRE2_CONVERT_POSIX_BASIC, "bre known posix class ok"),
        (b"[[:unknown:]]", PCRE2_CONVERT_POSIX_BASIC, "bre unknown posix class"),
        (b"[[.collating.]]", PCRE2_CONVERT_POSIX_EXTENDED, "ere collating"),
        (b"[[=equiv=]]", PCRE2_CONVERT_POSIX_EXTENDED, "ere equiv"),
        // glob: trailing escape -> CONVERT_SYNTAX (with default escape '\\')
        (b"a\\", PCRE2_CONVERT_GLOB, "glob trailing escape"),
        (b"\\", PCRE2_CONVERT_GLOB, "glob lone escape"),
        // glob: unterminated '[' -> MISSING_SQUARE_BRACKET
        (b"[", PCRE2_CONVERT_GLOB, "glob open class"),
        (b"[abc", PCRE2_CONVERT_GLOB, "glob class no close"),
        (b"[!", PCRE2_CONVERT_GLOB, "glob negated open"),
        (b"[a-", PCRE2_CONVERT_GLOB, "glob open range"),
        // glob: reversed range -> CONVERT_SYNTAX
        (b"[z-a]", PCRE2_CONVERT_GLOB, "glob reversed range"),
        // glob: class inside a range -> CONVERT_SYNTAX
        (b"[a-[:digit:]]", PCRE2_CONVERT_GLOB, "glob class in range"),
        // glob: '**' handling with / without NO_STARSTAR
        (b"**", PCRE2_CONVERT_GLOB, "glob starstar"),
        (b"**", PCRE2_CONVERT_GLOB_NO_STARSTAR, "glob starstar no_starstar"),
        (b"a/**/b", PCRE2_CONVERT_GLOB, "glob mid starstar"),
        (b"a/**/b", PCRE2_CONVERT_GLOB_NO_STARSTAR, "glob mid starstar nss"),
        // separators inside wildcards
        (b"[a/b]", PCRE2_CONVERT_GLOB, "glob sep in class"),
        (b"[a/b]", PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR, "glob sep in class nws"),
        (b"*/*", PCRE2_CONVERT_GLOB, "glob sep between stars"),
        // empty pattern
        (b"", PCRE2_CONVERT_GLOB, "glob empty"),
        (b"", PCRE2_CONVERT_POSIX_BASIC, "bre empty"),
        (b"", PCRE2_CONVERT_POSIX_EXTENDED, "ere empty"),
    ];
    for &(pat, opt, label) in cases {
        // explicit length
        check(pat, false, opt, None, None, label);
        // zero-terminated (only meaningful when there is no embedded NUL)
        check(pat, true, opt, None, None, label);
    }
}

// ============================================================================
//  5. Invalid UTF-8 with / without NO_UTF_CHECK.
// ============================================================================

#[test]
fn invalid_utf8_paths() {
    // Byte sequences that are NOT valid UTF-8. Passing these with
    // PCRE2_CONVERT_NO_UTF_CHECK violates the API contract (which asserts the
    // input IS valid UTF-8) and lets the converter's GETCHARLENTEST read past
    // the buffer end -> that is test misuse, not a divergence, so for invalid
    // input we only exercise (a) the UTF path WITH the check (must be rejected)
    // and (b) the non-UTF raw-byte path (GETCHARLENTEST does not decode when
    // utf is false, so it is in-bounds).
    let invalid: &[&[u8]] = &[
        b"\xff",                     // lone 0xff
        b"\x80",                     // stray continuation
        b"a\xc3\x28b",               // bad 2-byte
        b"\xe2\x28\xa1",             // bad 3-byte
        b"\xf0\x28\x8c\x28",         // bad 4-byte
        b"abc\xc0\x80",             // overlong NUL
        b"\xed\xa0\x80",             // UTF-16 surrogate
    ];
    // Valid UTF-8 sequences: safe to run through ALL flag combinations,
    // including NO_UTF_CHECK (the contract is satisfied).
    let valid: &[&[u8]] = &[
        b"valid\xf0\x9f\x98\x80",   // valid emoji
        "café/*".as_bytes(),        // valid multibyte glob
        "日本語*".as_bytes(),        // valid multibyte glob
        b"plain",
    ];
    for &(ty, tyname) in TYPES {
        for seq in invalid {
            // UTF with the check -> invalid input must be rejected identically.
            check(seq, false, ty | PCRE2_CONVERT_UTF, None, None,
                  &format!("{tyname} invalid utf+check"));
            // Non-UTF raw byte handling (in-bounds, no decoding).
            check(seq, false, ty, None, None, &format!("{tyname} invalid raw"));
        }
        for seq in valid {
            check(seq, false, ty | PCRE2_CONVERT_UTF | PCRE2_CONVERT_NO_UTF_CHECK, None, None,
                  &format!("{tyname} valid utf+nocheck"));
            check(seq, false, ty | PCRE2_CONVERT_UTF, None, None,
                  &format!("{tyname} valid utf+check"));
            check(seq, false, ty, None, None, &format!("{tyname} valid raw"));
        }
    }
}

// ============================================================================
//  6. Large seeded-random corpus.
// ============================================================================

/// Build a random glob/POSIX pattern from a pool that includes glob and POSIX
/// metacharacters plus multi-byte UTF-8.
fn random_conv_pattern(rng: &mut Rng) -> Vec<u8> {
    // Byte/​string fragments; some are multi-byte UTF-8.
    let pool: &[&[u8]] = &[
        b"*", b"?", b"[", b"]", b"^", b"$", b".", b"\\", b"/", b"{", b"}", b",",
        b"!", b"-", b"a", b"b", b"z", b"0", b"9", b"(", b")", b"+", b"|",
        b"[:alpha:]", b"[:digit:]", b":", b"**", b"[!", b"[^", b"a-z",
        "é".as_bytes(), "日".as_bytes(), "本".as_bytes(), "\u{1F600}".as_bytes(),
        b"\xff", b"\x80", b"\xc3\x28",
    ];
    let n = rng.range(0, 14);
    let mut out = Vec::new();
    for _ in 0..n {
        out.extend_from_slice(rng.pick(pool));
    }
    out
}

/// Random single-type option, optionally OR'd with UTF flags. `valid_utf`
/// indicates whether the pattern is valid UTF-8; NO_UTF_CHECK is only ever
/// combined with PCRE2_CONVERT_UTF when the input really is valid UTF-8,
/// because NO_UTF_CHECK on invalid input is an API-contract violation that
/// lets the converter read out of bounds (test misuse, not a divergence).
fn random_options(rng: &mut Rng, valid_utf: bool) -> u32 {
    let ty = TYPES[rng.below(TYPES.len())].0;
    let mut o = ty;
    let want_utf = rng.bool();
    if want_utf {
        o |= PCRE2_CONVERT_UTF;
    }
    if rng.bool() {
        // Safe to skip the check only when either UTF isn't requested (the flag
        // is then inert) or the input is valid UTF-8.
        if !want_utf || valid_utf {
            o |= PCRE2_CONVERT_NO_UTF_CHECK;
        }
    }
    o
}

#[test]
fn seeded_random_corpus() {
    let mut rng = Rng::new(0x00C0_FFEE_1234_5678);
    let seps: &[Option<u32>] = &[None, Some(b'/' as u32), Some(b'\\' as u32), Some(b'.' as u32)];
    let escs: &[Option<u32>] = &[None, Some(0), Some(b'\\' as u32), Some(b'!' as u32)];

    let iters = 3200; // > 3000 as required
    for i in 0..iters {
        let pat = random_conv_pattern(&mut rng);
        let valid_utf = std::str::from_utf8(&pat).is_ok();
        let opts = random_options(&mut rng, valid_utf);
        let sep = *rng.pick(seps);
        let esc = *rng.pick(escs);
        let zt = rng.bool() && !pat.contains(&0);
        check(&pat, zt, opts, sep, esc, &format!("rand#{i}"));
    }
}
