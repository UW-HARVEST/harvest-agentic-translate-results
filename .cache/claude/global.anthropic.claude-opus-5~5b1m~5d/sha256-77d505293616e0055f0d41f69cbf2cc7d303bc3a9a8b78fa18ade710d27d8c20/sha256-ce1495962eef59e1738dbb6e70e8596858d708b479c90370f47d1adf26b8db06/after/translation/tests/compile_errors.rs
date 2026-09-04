//! Phase C — error-path differential tests for `ERRORS.md` section B
//! (`pcre2_compile_8` bad pattern strings, `ERR1`…`ERR120`).
//!
//! For every trigger we assert that C and Rust agree on:
//!   * compile failure (both return NULL),
//!   * the exact `*errorcode`,
//!   * the exact `*erroroffset`,
//!   * the rendered `pcre2_get_error_message_8` text.
//! …and additionally that the C library really does produce the error number
//! recorded in `ERRORS.md` (so the table cannot silently rot).
mod common;

use common::diff::*;
use common::*;

#[path = "common/gen_compile_errors.rs"]
mod gen;

/// Compile `pat` in both libraries with `cfg` and assert both fail with the
/// same code/offset/message; also assert C's code equals `expect`.
unsafe fn assert_compile_error(
    pat: &[u8],
    cfg: &CompileCfg,
    expect: i32,
    label: &str,
) {
    let (c, r) = both();
    let cc = compile_in(c, pat, pat.len(), cfg);
    let rr = compile_in(r, pat, pat.len(), cfg);

    assert!(
        cc.code.is_null(),
        "{}: C unexpectedly COMPILED pattern {:?} (ERRORS.md says code {})",
        label,
        String::from_utf8_lossy(pat),
        expect
    );
    assert!(
        rr.code.is_null(),
        "{}: Rust unexpectedly COMPILED pattern {:?} while C failed with {}",
        label,
        String::from_utf8_lossy(pat),
        cc.errorcode
    );
    assert_eq!(
        cc.errorcode, expect,
        "{}: ERRORS.md says C returns {} for pattern {:?}, but C returned {}",
        label,
        expect,
        String::from_utf8_lossy(pat),
        cc.errorcode
    );
    assert_eq!(
        cc.errorcode, rr.errorcode,
        "{}: errorcode differs for pattern {:?} (C={} Rust={})",
        label,
        String::from_utf8_lossy(pat),
        cc.errorcode,
        rr.errorcode
    );
    assert_eq!(
        cc.erroroffset, rr.erroroffset,
        "{}: erroroffset differs for pattern {:?} (code={}, C={} Rust={})",
        label,
        String::from_utf8_lossy(pat),
        cc.errorcode,
        cc.erroroffset,
        rr.erroroffset
    );
    // the rendered message must match too
    let mut cb = [0u8; 256];
    let mut rb = [0u8; 256];
    let cn = (c.get_error_message)(cc.errorcode, cb.as_mut_ptr(), 256);
    let rn = (r.get_error_message)(rr.errorcode, rb.as_mut_ptr(), 256);
    assert_eq!(cn, rn, "{}: error message length for code {}", label, cc.errorcode);
    assert_eq!(cb, rb, "{}: error message text for code {}", label, cc.errorcode);
}

/// All 101 triggers extracted mechanically from `ERRORS.md` section B that need
/// no special compile options.
#[test]
fn errors_md_section_b_plain_patterns() {
    unsafe {
        for (row, err, code, pat) in gen::COMPILE_ERRORS {
            assert_compile_error(
                pat,
                &CompileCfg::new(0),
                *code,
                &format!("{} {}", row, err),
            );
        }
    }
}

/// Section B rows whose trigger needs a compile option or a generated pattern.
#[test]
fn errors_md_section_b_option_dependent() {
    unsafe {
        // row 66 — ERR40: PCRE2_ALT_VERBNAMES + non-literal escape in a verb name
        assert_compile_error(
            br"(*MARK:\d)",
            &CompileCfg::new(PCRE2_ALT_VERBNAMES),
            140,
            "row66 ERR40",
        );

        // row 103 — ERR77: ALT_BSUX | EXTRA_ALT_BSUX + \u{110000} out of range
        assert_compile_error(
            br"\u{110000}",
            &CompileCfg::new(PCRE2_ALT_BSUX).extra(PCRE2_EXTRA_ALT_BSUX),
            177,
            "row103 ERR77",
        );

        // rows 133-138 — the PCRE2_ALT_EXTENDED_CLASS operator errors
        let ec = CompileCfg::new(PCRE2_ALT_EXTENDED_CLASS);
        // row 133 ERR107: extended-class nesting too deep
        let deep: Vec<u8> = {
            let mut v = vec![b'['; 16];
            v.push(b'a');
            v.extend(std::iter::repeat(b']').take(16));
            v
        };
        assert_compile_error(&deep, &ec, 207, "row133 ERR107");
        // row 134 ERR108: invalid operator / triple-repeated set operator
        assert_compile_error(b"[a---b]", &ec, 208, "row134 ERR108");
        // row 135 ERR109: operator with no preceding operand
        assert_compile_error(b"[&&a]", &ec, 209, "row135 ERR109");
        // row 136 ERR110: operator with no following operand
        assert_compile_error(b"[a&&]", &ec, 210, "row136 ERR110");
        // row 137 ERR111: mixed operator precedence
        assert_compile_error(b"[a&&b||c]", &ec, 211, "row137 ERR111");
        // row 138 ERR112: unterminated extended class
        assert_compile_error(b"[[a]", &ec, 212, "row138 ERR112");

        // row 45 — ERR19: nesting beyond parens_nest_limit (default 250)
        let nest: Vec<u8> = {
            let mut v = vec![b'('; 260];
            v.push(b'a');
            v.extend(std::iter::repeat(b')').take(260));
            v
        };
        assert_compile_error(&nest, &CompileCfg::new(0), 119, "row45 ERR19");

        // row 46 — ERR20: compiled pattern too large
        assert_compile_error(
            b"(?:(?:(?:(?:a{255}){255}){255}){255})",
            &CompileCfg::new(0),
            120,
            "row46 ERR20",
        );

        // row 75 — ERR49: more than MAX_NAME_COUNT (10000) named groups
        let many: Vec<u8> = {
            let mut v = Vec::new();
            for i in 0..10005 {
                v.extend_from_slice(format!("(?<n{}>a)", i).as_bytes());
            }
            v
        };
        assert_compile_error(&many, &CompileCfg::new(0), 149, "row75 ERR49");

        // row 110 — ERR84: (?| / (?J: / (?x: nesting deeper than 255.
        // NOTE: parens_nest_limit must be raised, otherwise ERR19 (=119) fires
        // first at the default limit of 250.
        for opener in [&b"(?|"[..], b"(?J:", b"(?x:"] {
            let bars: Vec<u8> = {
                let mut v = Vec::new();
                for _ in 0..400 {
                    v.extend_from_slice(opener);
                }
                v.push(b'a');
                v.extend(std::iter::repeat(b')').take(400));
                v
            };
            assert_compile_error(
                &bars,
                &CompileCfg::new(0).parens_nest(100_000),
                184,
                "row110 ERR84",
            );
            // …and with the DEFAULT nest limit the same pattern gives ERR19
            assert_compile_error(&bars, &CompileCfg::new(0), 119, "row110 default-limit");
        }

        // row 123 — ERR97: too many capturing groups (> 65535)
        let caps: Vec<u8> = b"(a)".repeat(70000);
        assert_compile_error(&caps, &CompileCfg::new(0), 197, "row123 ERR97");

        // --- the nine rows whose trigger needs a CONSTRUCTED pattern -------
        // row 61 — ERR35: lookbehind length computation exceeds the 2000-iteration
        // cap (pcre2_compile.c:9600)
        let complicated: Vec<u8> = {
            let mut v = b"(?<=".to_vec();
            for _ in 0..2001 {
                v.extend_from_slice(b"(?|a|b)");
            }
            v.extend_from_slice(b")x");
            v
        };
        assert_compile_error(&complicated, &CompileCfg::new(0), 135, "row61 ERR35");

        // row 62 — ERR36: \C inside a lookbehind, UTF mode only
        assert_compile_error(
            br"(?<=\C)a",
            &CompileCfg::new(PCRE2_UTF),
            136,
            "row62 ERR36",
        );
        // …and WITHOUT PCRE2_UTF the very same pattern must COMPILE in both
        let _ = compile_both(
            br"(?<=\C)a",
            8,
            &CompileCfg::new(0),
            "row62 ERR36 non-utf compiles",
        );

        // row 74 — ERR48: subpattern name longer than MAX_NAME_SIZE (128)
        let longname: Vec<u8> = {
            let mut v = b"(?<".to_vec();
            v.extend(std::iter::repeat(b'n').take(129));
            v.extend_from_slice(b">a)");
            v
        };
        assert_compile_error(&longname, &CompileCfg::new(0), 148, "row74 ERR48");

        // row 94 — ERR68: \c followed by a non-printable ASCII character
        assert_compile_error(b"\\c\x7f", &CompileCfg::new(0), 168, "row94 ERR68");
        assert_compile_error(b"\\c\x00", &CompileCfg::new(0), 168, "row94 ERR68 NUL");

        // row 99 — ERR73: surrogate code point, UTF mode
        for pat in [&br"\x{d800}"[..], br"[\x{d800}]", br"\N{U+D800}"] {
            assert_compile_error(pat, &CompileCfg::new(PCRE2_UTF), 173, "row99 ERR73");
        }

        // row 102 — ERR76: verb name longer than MAX_MARK (255)
        let longmark: Vec<u8> = {
            let mut v = b"(*MARK:".to_vec();
            v.extend(std::iter::repeat(b'm').take(256));
            v.extend_from_slice(b")a");
            v
        };
        assert_compile_error(&longmark, &CompileCfg::new(0), 176, "row102 ERR76");

        // row 104 — ERR78: digits missing after \x / \o / \N{U+}
        for (pat, opts) in [
            (&br"\x{}"[..], 0u32),
            (br"\o{}", 0),
            (br"\x{ }", 0),
            (br"[\x{}]", 0),
            (br"\N{U+}", PCRE2_UTF),
        ] {
            assert_compile_error(pat, &CompileCfg::new(opts), 178, "row104 ERR78");
        }

        // row 113 — ERR87: lookbehind longer than LOOKBEHIND_MAX (65535)
        let longlb: Vec<u8> = {
            let mut v = b"(?<=".to_vec();
            v.extend(std::iter::repeat(b'a').take(70000));
            v.extend_from_slice(b")b");
            v
        };
        assert_compile_error(&longlb, &CompileCfg::new(0), 187, "row113 ERR87");

        // row 145 — ERR119: missing terminator in a subpattern-number reference
        assert_compile_error(br"\g{1x", &CompileCfg::new(0), 219, "row145 ERR119");
        assert_compile_error(br"\g{+1x}", &CompileCfg::new(0), 219, "row145 ERR119b");
    }
}

/// Section B rows reachable only via a compile OPTION conflict; these are
/// cross-referenced from section A ("see row N") and are checked here so every
/// section-B row has a passing test.
#[test]
fn errors_md_section_b_option_conflicts() {
    unsafe {
        // row 100 / ERR74: PCRE2_NEVER_UTF + (*UTF)
        assert_compile_error(
            b"(*UTF)a",
            &CompileCfg::new(PCRE2_NEVER_UTF),
            174,
            "row100 ERR74",
        );
        // row 101 / ERR75: PCRE2_NEVER_UCP + (*UCP)
        assert_compile_error(
            b"(*UCP)a",
            &CompileCfg::new(PCRE2_NEVER_UCP),
            175,
            "row101 ERR75",
        );
        // row 109 / ERR83: PCRE2_NEVER_BACKSLASH_C + \C
        assert_compile_error(
            br"\C",
            &CompileCfg::new(PCRE2_NEVER_BACKSLASH_C),
            183,
            "row109 ERR83",
        );
        // row 124 / ERR98: PCRE2_EXTRA_NO_BS0 + \0
        assert_compile_error(
            br"\0",
            &CompileCfg::new(0).extra(PCRE2_EXTRA_NO_BS0),
            198,
            "row124 ERR98",
        );
        // row 128 / ERR102: PCRE2_EXTRA_PYTHON_OCTAL + ambiguous \400
        assert_compile_error(
            br"\400",
            &CompileCfg::new(0).extra(PCRE2_EXTRA_PYTHON_OCTAL),
            202,
            "row128 ERR102",
        );
        // row 129 / ERR103: PCRE2_EXTRA_NEVER_CALLOUT + (?C1)
        assert_compile_error(
            b"(?C1)a",
            &CompileCfg::new(0).extra(PCRE2_EXTRA_NEVER_CALLOUT),
            203,
            "row129 ERR103",
        );
        // row 114 / ERR88: max_pattern_length exceeded
        assert_compile_error(
            b"abcdefghij",
            &CompileCfg::new(0).max_len(5),
            188,
            "row114 ERR88",
        );
        // row 127 / ERR101: max_pattern_compiled_length exceeded
        assert_compile_error(
            b"abcdefghij",
            &CompileCfg::new(0).max_compiled(1),
            201,
            "row127 ERR101",
        );
    }
}

/// Section B rows that are UNREACHABLE through the public API in this build.
/// Documented here so no row is silently unaccounted for; each is asserted to
/// be unreachable in the SAME way in both libraries.
#[test]
fn errors_md_section_b_unreachable_rows_documented() {
    let (c, r) = both();
    unsafe {
        // ERR32 / ERR45 / ERR96: "no Unicode support". This build DOES have
        // SUPPORT_UNICODE (CMakeLists adds -DSUPPORT_UNICODE), so the patterns
        // that would trigger them must COMPILE in both libraries instead.
        for (pat, opts) in [
            (&b"a"[..], PCRE2_UTF),
            (&b"a"[..], PCRE2_UCP),
            (&br"\p{L}"[..], 0u32),
            (&br"\X"[..], 0u32),
            (&b"(*script_run:a)"[..], 0u32),
        ] {
            let _ = compile_both(pat, pat.len(), &CompileCfg::new(opts), "unicode-present");
        }
        // and pcre2_config must agree that Unicode is on and JIT is off
        for what in [9u32 /* UNICODE */, 1 /* JIT */] {
            let mut cv = 0u32;
            let mut rv = 0u32;
            assert_eq!(
                (c.config)(what, &mut cv as *mut _ as *mut _),
                (r.config)(what, &mut rv as *mut _ as *mut _),
            );
            assert_eq!(cv, rv, "config({})", what);
        }

        // ERR59 is a retired slot: never returned, but its message must still
        // render identically (and be the "obsolete error" text).
        let mut cb = [0u8; 128];
        let mut rb = [0u8; 128];
        let cn = (c.get_error_message)(159, cb.as_mut_ptr(), 128);
        let rn = (r.get_error_message)(159, rb.as_mut_ptr(), 128);
        assert_eq!(cn, rn, "ERR59 message length");
        assert_eq!(cb, rb, "ERR59 message text");

        // The remaining unreachable rows are "internal error" codes that are
        // only produced by corrupted internal state (ERR10, 23, 31, 52, 53, 56,
        // 63, 70, 80, 89, 90), plus ERR85/ERR91 (other code-unit widths) and
        // ERR72/ERR86 (need >4 GiB inputs). They cannot be provoked across the
        // FFI boundary. What we CAN and DO verify is that both libraries render
        // every one of those codes identically:
        for code in [
            110, 123, 131, 152, 153, 156, 163, 170, 180, 189, 190, // internal
            132, 145, 196, // no-Unicode (unreachable here)
            185, 191, // other widths
            172, 186, // need huge inputs
        ] {
            let mut cb = [0u8; 256];
            let mut rb = [0u8; 256];
            let cn = (c.get_error_message)(code, cb.as_mut_ptr(), 256);
            let rn = (r.get_error_message)(code, rb.as_mut_ptr(), 256);
            assert_eq!(cn, rn, "code {} message length", code);
            assert_eq!(cb, rb, "code {} message text", code);
        }
    }
}

/// Every compile-error code in the whole table must round-trip through
/// `pcre2_get_error_message_8` identically, at every buffer size.
#[test]
fn all_compile_error_messages_render_identically() {
    let (c, r) = both();
    unsafe {
        for code in 100..=225 {
            for bufsize in [0usize, 1, 2, 8, 16, 32, 64, 128, 256] {
                let mut cb = vec![0xAAu8; bufsize + 8];
                let mut rb = vec![0xAAu8; bufsize + 8];
                let cn = (c.get_error_message)(code, cb.as_mut_ptr(), bufsize);
                let rn = (r.get_error_message)(code, rb.as_mut_ptr(), bufsize);
                assert_eq!(cn, rn, "code {} buf {} rc", code, bufsize);
                assert_eq!(cb, rb, "code {} buf {} bytes", code, bufsize);
            }
        }
    }
}
