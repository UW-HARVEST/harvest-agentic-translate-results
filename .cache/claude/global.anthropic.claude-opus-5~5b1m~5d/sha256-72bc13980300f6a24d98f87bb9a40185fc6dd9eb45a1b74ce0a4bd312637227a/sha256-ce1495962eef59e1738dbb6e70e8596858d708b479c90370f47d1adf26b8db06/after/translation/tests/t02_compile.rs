//! Phase B — `pcre2_compile` across the option cross-product, plus
//! `pcre2_pattern_info`, `pcre2_code_copy*` and `pcre2_callout_enumerate`.
//!
//! CONFIGS.md rows 47-89 · ERRORS.md rows 176-185.
#![allow(non_snake_case)]

mod common;
use common::corpus::*;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

/// Compiles `pat` with `options`/`extra`/`newline`/`bsr` and logs *everything*
/// observable: error code + offset, every `PCRE2_INFO_*`, and the full
/// serialized bytecode.
fn compile_probe(
    api: &Api,
    pat: &[u8],
    patlen: Sz,
    options: u32,
    extra: u32,
    newline: u32,
    bsr: u32,
    l: &mut Log,
) {
    unsafe {
        let cc = (api.compile_context_create)(std::ptr::null_mut());
        assert!(!cc.is_null());
        if extra != 0 {
            l.i((api.set_compile_extra_options)(cc, extra) as i64);
        }
        if newline != 0 {
            l.i((api.set_newline)(cc, newline) as i64);
        }
        if bsr != 0 {
            l.i((api.set_bsr)(cc, bsr) as i64);
        }
        let code = compile_logged(api, pat, patlen, options, cc, l);
        if !code.is_null() {
            log_all_info(api, code, l);
            log_serialized(api, code, l);
            // code_copy and code_copy_with_tables must produce equivalent codes
            let cp = (api.code_copy)(code);
            l.tag("copy").i(cp.is_null() as i64);
            if !cp.is_null() {
                log_all_info(api, cp, l);
                (api.code_free)(cp);
            }
            let cpt = (api.code_copy_with_tables)(code);
            l.tag("copyt").i(cpt.is_null() as i64);
            if !cpt.is_null() {
                log_all_info(api, cpt, l);
                (api.code_free)(cpt);
            }
            (api.code_free)(code);
        }
        (api.compile_context_free)(cc);
    }
}

fn diff_compile(label: &str, pat: &[u8], options: u32, extra: u32, newline: u32, bsr: u32) {
    diff(label, |api| {
        let mut l = Log::new();
        compile_probe(api, pat, pat.len(), options, extra, newline, bsr, &mut l);
        l
    });
}

// -------------------------------------------------- row 47/48: baseline shapes

#[test]
fn compile_all_patterns_no_options() {
    for (i, p) in PATTERNS.iter().enumerate() {
        diff_compile(&format!("plain[{i}]={p:?}"), p.as_bytes(), 0, 0, 0, 0);
    }
}

#[test]
fn compile_zero_terminated_vs_explicit_length() {
    for (i, p) in PATTERNS.iter().enumerate() {
        // Zero-terminated form: identical result unless the pattern has a NUL.
        let mut z = p.as_bytes().to_vec();
        z.push(0);
        diff(&format!("zt[{i}]"), |api| {
            let mut l = Log::new();
            compile_probe(api, &z, PCRE2_ZERO_TERMINATED, 0, 0, 0, 0, &mut l);
            // explicit length that is shorter than the string
            for cut in [0usize, 1, p.len() / 2] {
                if cut <= p.len() {
                    compile_probe(api, &z, cut, 0, 0, 0, 0, &mut l);
                }
            }
            l
        });
    }
    // NULL pattern with zero and non-zero length (ERRORS rows 4/5).
    diff("null_pattern", |api| {
        let mut l = Log::new();
        unsafe {
            for len in [0usize, 1, 5, PCRE2_ZERO_TERMINATED] {
                let mut ec: c_int = 0x7FFF;
                let mut eo: Sz = 0xDEAD;
                let code = (api.compile)(
                    std::ptr::null(),
                    len,
                    0,
                    &mut ec,
                    &mut eo,
                    std::ptr::null_mut(),
                );
                l.i(code.is_null() as i64).i(ec as i64).u(eo as u64);
                if !code.is_null() {
                    log_all_info(api, code, &mut l);
                    (api.code_free)(code);
                }
            }
        }
        l
    });
}

// -------------------------------------------------- rows 49-80: single options

/// Every compile option bit, applied on its own to every pattern.
#[test]
fn compile_each_option_bit() {
    let opts: &[(&str, u32)] = &[
        ("ANCHORED", PCRE2_ANCHORED),
        ("NO_UTF_CHECK", PCRE2_NO_UTF_CHECK),
        ("ENDANCHORED", PCRE2_ENDANCHORED),
        ("ALLOW_EMPTY_CLASS", PCRE2_ALLOW_EMPTY_CLASS),
        ("ALT_BSUX", PCRE2_ALT_BSUX),
        ("AUTO_CALLOUT", PCRE2_AUTO_CALLOUT),
        ("CASELESS", PCRE2_CASELESS),
        ("DOLLAR_ENDONLY", PCRE2_DOLLAR_ENDONLY),
        ("DOTALL", PCRE2_DOTALL),
        ("DUPNAMES", PCRE2_DUPNAMES),
        ("EXTENDED", PCRE2_EXTENDED),
        ("FIRSTLINE", PCRE2_FIRSTLINE),
        ("MATCH_UNSET_BACKREF", PCRE2_MATCH_UNSET_BACKREF),
        ("MULTILINE", PCRE2_MULTILINE),
        ("NEVER_UCP", PCRE2_NEVER_UCP),
        ("NEVER_UTF", PCRE2_NEVER_UTF),
        ("NO_AUTO_CAPTURE", PCRE2_NO_AUTO_CAPTURE),
        ("NO_AUTO_POSSESS", PCRE2_NO_AUTO_POSSESS),
        ("NO_DOTSTAR_ANCHOR", PCRE2_NO_DOTSTAR_ANCHOR),
        ("NO_START_OPTIMIZE", PCRE2_NO_START_OPTIMIZE),
        ("UCP", PCRE2_UCP),
        ("UNGREEDY", PCRE2_UNGREEDY),
        ("UTF", PCRE2_UTF),
        ("NEVER_BACKSLASH_C", PCRE2_NEVER_BACKSLASH_C),
        ("ALT_CIRCUMFLEX", PCRE2_ALT_CIRCUMFLEX),
        ("ALT_VERBNAMES", PCRE2_ALT_VERBNAMES),
        ("USE_OFFSET_LIMIT", PCRE2_USE_OFFSET_LIMIT),
        ("EXTENDED_MORE", PCRE2_EXTENDED_MORE),
        ("LITERAL", PCRE2_LITERAL),
        ("MATCH_INVALID_UTF", PCRE2_MATCH_INVALID_UTF),
        ("ALT_EXTENDED_CLASS", PCRE2_ALT_EXTENDED_CLASS),
    ];
    for (name, bit) in opts {
        for (i, p) in PATTERNS.iter().enumerate() {
            diff_compile(&format!("opt={name} pat[{i}]={p:?}"), p.as_bytes(), *bit, 0, 0, 0);
        }
    }
}

/// Every `PCRE2_EXTRA_*` option bit on its own (rows 39, 59, 72-80).
#[test]
fn compile_each_extra_option_bit() {
    let extras: &[(&str, u32)] = &[
        ("ALLOW_SURROGATE_ESCAPES", PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES),
        ("BAD_ESCAPE_IS_LITERAL", PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL),
        ("MATCH_WORD", PCRE2_EXTRA_MATCH_WORD),
        ("MATCH_LINE", PCRE2_EXTRA_MATCH_LINE),
        ("ESCAPED_CR_IS_LF", PCRE2_EXTRA_ESCAPED_CR_IS_LF),
        ("ALT_BSUX", PCRE2_EXTRA_ALT_BSUX),
        ("ALLOW_LOOKAROUND_BSK", PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
        ("CASELESS_RESTRICT", PCRE2_EXTRA_CASELESS_RESTRICT),
        ("ASCII_BSD", PCRE2_EXTRA_ASCII_BSD),
        ("ASCII_BSS", PCRE2_EXTRA_ASCII_BSS),
        ("ASCII_BSW", PCRE2_EXTRA_ASCII_BSW),
        ("ASCII_POSIX", PCRE2_EXTRA_ASCII_POSIX),
        ("ASCII_DIGIT", PCRE2_EXTRA_ASCII_DIGIT),
        ("PYTHON_OCTAL", PCRE2_EXTRA_PYTHON_OCTAL),
        ("NO_BS0", PCRE2_EXTRA_NO_BS0),
        ("NEVER_CALLOUT", PCRE2_EXTRA_NEVER_CALLOUT),
        ("TURKISH_CASING", PCRE2_EXTRA_TURKISH_CASING),
    ];
    for (name, bit) in extras {
        for base in [0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS] {
            for (i, p) in PATTERNS.iter().enumerate() {
                diff_compile(
                    &format!("extra={name} base={base:#x} pat[{i}]={p:?}"),
                    p.as_bytes(),
                    base,
                    *bit,
                    0,
                    0,
                );
            }
        }
    }
}

/// Newline conventions × BSR conventions (rows 34, 50, 63).
#[test]
fn compile_newline_and_bsr_conventions() {
    for nl in 1..=6u32 {
        for bsr in 1..=2u32 {
            for base in [0u32, PCRE2_MULTILINE, PCRE2_FIRSTLINE, PCRE2_DOTALL] {
                for (i, p) in PATTERNS.iter().enumerate() {
                    diff_compile(
                        &format!("nl={nl} bsr={bsr} base={base:#x} pat[{i}]"),
                        p.as_bytes(),
                        base,
                        0,
                        nl,
                        bsr,
                    );
                }
            }
        }
    }
}

/// Extended-class patterns need `PCRE2_ALT_EXTENDED_CLASS` (row 68).
#[test]
fn compile_extended_classes() {
    for (i, p) in ECLASS_PATTERNS.iter().enumerate() {
        for opt in [
            0u32,
            PCRE2_ALT_EXTENDED_CLASS,
            PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF,
            PCRE2_ALT_EXTENDED_CLASS | PCRE2_UCP,
            PCRE2_ALT_EXTENDED_CLASS | PCRE2_CASELESS,
            PCRE2_ALT_EXTENDED_CLASS | PCRE2_EXTENDED,
        ] {
            diff_compile(&format!("eclass[{i}] opt={opt:#x}"), p.as_bytes(), opt, 0, 0, 0);
        }
    }
}

/// Empty classes need `PCRE2_ALLOW_EMPTY_CLASS` (row 67).
#[test]
fn compile_empty_classes() {
    for (i, p) in EMPTY_CLASS_PATTERNS.iter().enumerate() {
        for opt in [0u32, PCRE2_ALLOW_EMPTY_CLASS, PCRE2_ALLOW_EMPTY_CLASS | PCRE2_UTF] {
            diff_compile(&format!("emptycls[{i}] opt={opt:#x}"), p.as_bytes(), opt, 0, 0, 0);
        }
    }
}

// -------------------------------------------------- rows 35-40: limits

#[test]
fn compile_limits() {
    let pats: Vec<Vec<u8>> = vec![
        b"abc".to_vec(),
        b"(((((((((((a)))))))))))".to_vec(),
        b"(?<=a{1,20}b)c".to_vec(),
        b"a{1,100}".to_vec(),
        // 300-deep nesting to cross the default parens_nest_limit of 250
        {
            let mut v = Vec::new();
            for _ in 0..300 {
                v.push(b'(');
            }
            v.push(b'a');
            for _ in 0..300 {
                v.push(b')');
            }
            v
        },
    ];
    for (i, p) in pats.iter().enumerate() {
        diff(&format!("limits[{i}]"), |api| {
            let mut l = Log::new();
            unsafe {
                for maxpatlen in [0usize, 1, 2, p.len(), p.len() + 1, usize::MAX] {
                    for maxcompiled in [0usize, 1, 16, 1024, usize::MAX] {
                        for parens in [0u32, 1, 5, 250, 1000] {
                            for varlb in [0u32, 1, 20, 255] {
                                let cc = (api.compile_context_create)(std::ptr::null_mut());
                                l.i((api.set_max_pattern_length)(cc, maxpatlen) as i64);
                                l.i((api.set_max_pattern_compiled_length)(cc, maxcompiled) as i64);
                                l.i((api.set_parens_nest_limit)(cc, parens) as i64);
                                l.i((api.set_max_varlookbehind)(cc, varlb) as i64);
                                let code = compile_logged(api, p, p.len(), 0, cc, &mut l);
                                if !code.is_null() {
                                    let mut v: Sz = 0;
                                    (api.pattern_info)(
                                        code,
                                        INFO_SIZE,
                                        &mut v as *mut Sz as *mut c_void,
                                    );
                                    l.u(v as u64);
                                    (api.code_free)(code);
                                }
                                (api.compile_context_free)(cc);
                            }
                        }
                    }
                }
            }
            l
        });
    }
}

// -------------------------------------------------- row 40: set_optimize

#[test]
fn compile_optimize_directives() {
    let directives = [
        PCRE2_OPTIMIZATION_NONE,
        PCRE2_OPTIMIZATION_FULL,
        PCRE2_AUTO_POSSESS,
        PCRE2_AUTO_POSSESS_OFF,
        PCRE2_DOTSTAR_ANCHOR,
        PCRE2_DOTSTAR_ANCHOR_OFF,
        PCRE2_START_OPTIMIZE,
        PCRE2_START_OPTIMIZE_OFF,
    ];
    for (i, p) in PATTERNS.iter().enumerate() {
        diff(&format!("optimize pat[{i}]"), |api| {
            let mut l = Log::new();
            unsafe {
                for d in directives {
                    let cc = (api.compile_context_create)(std::ptr::null_mut());
                    l.i((api.set_optimize)(cc, d) as i64);
                    let code = compile_logged(api, p.as_bytes(), p.len(), 0, cc, &mut l);
                    if !code.is_null() {
                        log_all_info(api, code, &mut l);
                        log_serialized(api, code, &mut l);
                        (api.code_free)(code);
                    }
                    (api.compile_context_free)(cc);
                }
                // Sequences of directives (the flags word accumulates).
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                for d in directives {
                    l.i((api.set_optimize)(cc, d) as i64);
                }
                let code = compile_logged(api, p.as_bytes(), p.len(), 0, cc, &mut l);
                if !code.is_null() {
                    log_all_info(api, code, &mut l);
                    (api.code_free)(code);
                }
                (api.compile_context_free)(cc);
            }
            l
        });
    }
}

// -------------------------------------------------- row 7: custom tables

#[test]
fn compile_with_generated_character_tables() {
    for (i, p) in PATTERNS.iter().enumerate() {
        diff(&format!("tables pat[{i}]"), |api| {
            let mut l = Log::new();
            unsafe {
                let tables = (api.maketables)(std::ptr::null_mut());
                let cc = (api.compile_context_create)(std::ptr::null_mut());
                l.i((api.set_character_tables)(cc, tables) as i64);
                let code = compile_logged(api, p.as_bytes(), p.len(), PCRE2_CASELESS, cc, &mut l);
                if !code.is_null() {
                    log_all_info(api, code, &mut l);
                    log_serialized(api, code, &mut l);
                    (api.code_free)(code);
                }
                // and explicitly NULL tables (falls back to defaults)
                l.i((api.set_character_tables)(cc, std::ptr::null()) as i64);
                let code2 = compile_logged(api, p.as_bytes(), p.len(), PCRE2_CASELESS, cc, &mut l);
                if !code2.is_null() {
                    log_all_info(api, code2, &mut l);
                    (api.code_free)(code2);
                }
                (api.compile_context_free)(cc);
                (api.maketables_free)(std::ptr::null_mut(), tables);
            }
            l
        });
    }
}

// -------------------------------------------------- row 45: recursion guard

static mut GUARD_CALLS: u32 = 0;

unsafe extern "C" fn guard_ok(_depth: u32, _d: *mut c_void) -> c_int {
    GUARD_CALLS += 1;
    0
}
unsafe extern "C" fn guard_fail_deep(depth: u32, _d: *mut c_void) -> c_int {
    GUARD_CALLS += 1;
    if depth > 3 {
        1
    } else {
        0
    }
}
unsafe extern "C" fn guard_always_fail(_depth: u32, _d: *mut c_void) -> c_int {
    GUARD_CALLS += 1;
    1
}

#[test]
fn compile_recursion_guard() {
    let pats: [&str; 6] = [
        "a",
        "(a)",
        "((a))",
        "((((a))))",
        "((((((((a))))))))",
        "(a(b(c(d(e)))))",
    ];
    for (i, p) in pats.iter().enumerate() {
        diff(&format!("guard pat[{i}]"), |api| {
            let mut l = Log::new();
            unsafe {
                for (k, g) in [
                    None,
                    Some(guard_ok as unsafe extern "C" fn(u32, *mut c_void) -> c_int),
                    Some(guard_fail_deep as unsafe extern "C" fn(u32, *mut c_void) -> c_int),
                    Some(guard_always_fail as unsafe extern "C" fn(u32, *mut c_void) -> c_int),
                ]
                .iter()
                .enumerate()
                {
                    GUARD_CALLS = 0;
                    let cc = (api.compile_context_create)(std::ptr::null_mut());
                    l.i((api.set_compile_recursion_guard)(cc, *g, std::ptr::null_mut()) as i64);
                    let code = compile_logged(api, p.as_bytes(), p.len(), 0, cc, &mut l);
                    l.u(k as u64).u(GUARD_CALLS as u64);
                    if !code.is_null() {
                        log_all_info(api, code, &mut l);
                        (api.code_free)(code);
                    }
                    (api.compile_context_free)(cc);
                }
            }
            l
        });
    }
}

// -------------------------------------------------- row 82: randomized patterns

#[test]
fn compile_random_patterns() {
    let mut rng = Rng::new(0xC0FFEE_01);
    let combos: [(u32, u32); 12] = [
        (0, 0),
        (PCRE2_CASELESS, 0),
        (PCRE2_UTF, 0),
        (PCRE2_UTF | PCRE2_UCP, 0),
        (PCRE2_MULTILINE | PCRE2_DOTALL, 0),
        (PCRE2_EXTENDED, 0),
        (PCRE2_UNGREEDY, 0),
        (PCRE2_NO_AUTO_CAPTURE, 0),
        (PCRE2_AUTO_CALLOUT, 0),
        (PCRE2_NO_AUTO_POSSESS | PCRE2_NO_START_OPTIMIZE, 0),
        (PCRE2_UTF | PCRE2_CASELESS, PCRE2_EXTRA_CASELESS_RESTRICT),
        (PCRE2_UCP, PCRE2_EXTRA_ASCII_BSD | PCRE2_EXTRA_ASCII_BSW),
    ];
    for iter in 0..1200 {
        let pat = PatternGen::gen(&mut rng);
        for (opts, extra) in combos {
            diff_compile(
                &format!("randpat iter={iter} pat={pat:?} opts={opts:#x} extra={extra:#x}"),
                pat.as_bytes(),
                opts,
                extra,
                0,
                0,
            );
        }
    }
}

/// Random *bytes* as a pattern: mostly invalid, which exercises the whole
/// error surface of the parser and must produce identical error codes/offsets.
#[test]
fn compile_random_bytes() {
    let mut rng = Rng::new(0xC0FFEE_02);
    let alphabet: &[u8] = b"ab()[]{}|*+?.^$\\-,:!<>=&~#'\"`0123456789PpQqEeCcRrNnKkGgXxUuLlWwSsDdHhVvBbAaZzTtMmFfIiJjOoYy \t\n";
    for iter in 0..8000 {
        let n = rng.below(14);
        let pat: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
        for opts in [
            0u32,
            PCRE2_UTF,
            PCRE2_UCP,
            PCRE2_CASELESS | PCRE2_MULTILINE,
            PCRE2_EXTENDED,
            PCRE2_ALT_EXTENDED_CLASS,
            PCRE2_ALT_BSUX,
            PCRE2_ALLOW_EMPTY_CLASS,
            PCRE2_LITERAL,
        ] {
            diff_compile(
                &format!("randbytes iter={iter} pat={pat:?} opts={opts:#x}"),
                &pat,
                opts,
                0,
                0,
                0,
            );
        }
    }
}

// -------------------------------------------------- row 89: callout_enumerate

static mut ENUM_LOG: Vec<u8> = Vec::new();

unsafe extern "C" fn enum_cb(b: *mut CalloutEnumerateBlock, data: *mut c_void) -> c_int {
    let b = &*b;
    let v = &mut *(data as *mut Vec<u8>);
    v.extend_from_slice(&b.version.to_le_bytes());
    v.extend_from_slice(&b.pattern_position.to_le_bytes());
    v.extend_from_slice(&b.next_item_length.to_le_bytes());
    v.extend_from_slice(&b.callout_number.to_le_bytes());
    v.extend_from_slice(&b.callout_string_offset.to_le_bytes());
    v.extend_from_slice(&b.callout_string_length.to_le_bytes());
    if b.callout_string.is_null() {
        v.push(0);
    } else {
        v.push(1);
        v.extend_from_slice(std::slice::from_raw_parts(
            b.callout_string,
            b.callout_string_length,
        ));
    }
    0
}

unsafe extern "C" fn enum_cb_stop(_b: *mut CalloutEnumerateBlock, _d: *mut c_void) -> c_int {
    -99
}

#[test]
fn callout_enumerate_all() {
    for (i, p) in PATTERNS.iter().enumerate() {
        for opts in [0u32, PCRE2_AUTO_CALLOUT, PCRE2_AUTO_CALLOUT | PCRE2_UTF] {
            diff(&format!("calloutenum pat[{i}] opts={opts:#x}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code =
                        compile_logged(api, p.as_bytes(), p.len(), opts, std::ptr::null_mut(), &mut l);
                    if code.is_null() {
                        return l;
                    }
                    let mut buf: Vec<u8> = Vec::new();
                    let rc = (api.callout_enumerate)(
                        code,
                        Some(enum_cb),
                        &mut buf as *mut Vec<u8> as *mut c_void,
                    );
                    l.i(rc as i64).b(&buf);
                    // A callback that aborts must propagate its return value.
                    let rc2 = (api.callout_enumerate)(
                        code,
                        Some(enum_cb_stop),
                        std::ptr::null_mut(),
                    );
                    l.i(rc2 as i64);
                    (api.code_free)(code);
                }
                l
            });
        }
    }
}

// -------------------------------------------------- ERRORS rows 176-185

#[test]
fn pattern_info_error_paths() {
    diff("pattern_info_errors", |api| {
        let mut l = Log::new();
        unsafe {
            // code == NULL
            let mut v: u32 = 0;
            for what in 0..=27u32 {
                let rc = (api.pattern_info)(
                    std::ptr::null_mut(),
                    what,
                    &mut v as *mut u32 as *mut c_void,
                );
                l.i(rc as i64);
            }
            // bad magic: corrupt a copy of a real compiled code
            let code = compile_logged(api, b"abc", 3, 0, std::ptr::null_mut(), &mut l);
            assert!(!code.is_null());
            let cp = (api.code_copy)(code);
            // ERRORS row 177: corrupt magic_number -> PCRE2_ERROR_BADMAGIC
            with_bad_magic(cp, || {
                for what in [INFO_SIZE, INFO_CAPTURECOUNT, INFO_NAMETABLE, 9999] {
                    let rc = (api.pattern_info)(cp, what, &mut v as *mut u32 as *mut c_void);
                    l.tag("badmagic").i(rc as i64);
                }
                let rc = (api.callout_enumerate)(cp, Some(enum_cb), std::ptr::null_mut());
                l.tag("badmagic_ce").i(rc as i64);
            });
            // ERRORS row 178: clear the code-unit-width bit -> PCRE2_ERROR_BADMODE
            with_bad_mode(cp, || {
                for what in [INFO_SIZE, INFO_CAPTURECOUNT, INFO_NAMETABLE, 9999] {
                    let rc = (api.pattern_info)(cp, what, &mut v as *mut u32 as *mut c_void);
                    l.tag("badmode").i(rc as i64);
                }
                let rc = (api.callout_enumerate)(cp, Some(enum_cb), std::ptr::null_mut());
                l.tag("badmode_ce").i(rc as i64);
            });
            (api.code_free)(cp);

            // Unset limits => PCRE2_ERROR_UNSET
            for what in [INFO_DEPTHLIMIT, INFO_HEAPLIMIT, INFO_MATCHLIMIT] {
                let rc = (api.pattern_info)(code, what, &mut v as *mut u32 as *mut c_void);
                l.tag("unset").i(rc as i64);
            }
            // where == NULL size-query form
            for what in 0..=27u32 {
                let rc = (api.pattern_info)(code, what, std::ptr::null_mut());
                l.tag("nullwhere").i(rc as i64);
            }
            (api.code_free)(code);

            // callout_enumerate with NULL code
            let rc = (api.callout_enumerate)(
                std::ptr::null_mut(),
                Some(enum_cb),
                std::ptr::null_mut(),
            );
            l.tag("ce_null").i(rc as i64);
        }
        l
    });
}

/// Limits set by inline `(*LIMIT_*)` directives must be reported identically.
#[test]
fn pattern_info_inline_limits() {
    let pats = [
        "(*LIMIT_MATCH=1000)a",
        "(*LIMIT_DEPTH=50)a",
        "(*LIMIT_HEAP=64)a",
        "(*LIMIT_MATCH=1)(*LIMIT_DEPTH=2)(*LIMIT_HEAP=3)a",
        "(*LIMIT_MATCH=0)a",
        "(*LIMIT_MATCH=4294967295)a",
        "(*LIMIT_MATCH=99999999999999999999)a",
    ];
    for (i, p) in pats.iter().enumerate() {
        diff(&format!("inline_limits[{i}]"), |api| {
            let mut l = Log::new();
            unsafe {
                let code =
                    compile_logged(api, p.as_bytes(), p.len(), 0, std::ptr::null_mut(), &mut l);
                if !code.is_null() {
                    log_all_info(api, code, &mut l);
                    (api.code_free)(code);
                }
            }
            l
        });
    }
}
