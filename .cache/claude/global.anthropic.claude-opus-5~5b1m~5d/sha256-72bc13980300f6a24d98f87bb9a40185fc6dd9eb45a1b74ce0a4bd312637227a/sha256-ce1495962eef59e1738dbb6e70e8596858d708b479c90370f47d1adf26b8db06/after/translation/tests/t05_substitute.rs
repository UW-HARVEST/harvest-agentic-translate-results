//! Phase B — `pcre2_substitute` across the whole option cross-product, plus
//! `pcre2_set_substitute_callout` and `pcre2_set_substitute_case_callout`.
//!
//! CONFIGS.md rows 143-158 · ERRORS.md rows 148-175.
//!
//! Every observation goes through `diff()`, which runs the identical closure
//! against the C `libpcre2.so` and the Rust `libpcre2.so` and compares the
//! resulting byte logs. Pointers are *never* logged; only pointee bytes,
//! lengths, offsets and null-ness.
#![allow(non_snake_case)]

mod common;
use common::corpus::*;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

/// Extra bytes appended past the declared buffer capacity. They are pre-filled
/// with a sentinel and included in the log, so any write past `*blength` shows
/// up as a divergence (or as a change from the sentinel in both libraries).
const GUARD: usize = 16;

// ============================================================ local corpora

/// Patterns chosen for substitution: numbered groups, named groups, duplicate
/// names, optional/unset groups, marks, `\K`, empty-matching, anchors.
const SUB_PATTERNS: &[&str] = &[
    "",
    "a",
    "b",
    "abc",
    "a+",
    "a*",
    ".",
    "\\w+",
    "\\d+",
    "[aeiou]",
    "(a)",
    "(a)(b)",
    "(a)|(b)",
    "(a)?b",
    "(a)?(b)?(c)?",
    "(.)(.)",
    "(?<name>a)",
    "(?<name>a)(?<other>b)",
    "(?<name>\\w)(\\d)?",
    "(?J)(?<name>a)|(?<name>b)",
    "(?<year>\\d{4})-(?<mon>\\d{2})",
    "(?i)a",
    "^a",
    "a$",
    "a\\Kb",
    "(*MARK:m1)a",
    "a(?=b)",
    "(?:x)?",
    "((((a))))",
    "\\b\\w",
];

/// Hand-picked replacement strings covering every `$`/`\` form the substitute
/// scanner recognizes, plus a number of malformed ones.
const REPLACEMENTS: &[&str] = &[
    "",
    "X",
    "xyz",
    "-",
    " ",
    "\n",
    "$0",
    "$1",
    "${1}",
    "$2",
    "${2}",
    "$3",
    "$&",
    "$$",
    "$",
    "$_",
    "$`",
    "$'",
    "$+",
    "$name",
    "${name}",
    "$<name>",
    "${*MARK}",
    "$*MARK",
    "[$0]",
    "$1$2",
    "a$1b$0c",
    "$01",
    "$99",
    "${99}",
    "${zz}",
    "${1",
    "$1}",
    "${}",
    "${*}",
    "$<name",
    "$<>",
    "\\U$1\\E",
    "\\L$1\\E",
    "\\u$1",
    "\\l$1",
    "\\u\\L$1\\E",
    "\\l\\U$1\\E",
    "\\U$0",
    "\\Uabc\\Edef",
    "\\a\\e\\f\\n\\r\\t\\0",
    "\\x41",
    "\\x{41}",
    "\\x{263A}",
    "\\o{101}",
    "\\cA",
    "\\Qa$1b\\E",
    "\\Qabc",
    "\\g<name>",
    "\\g1",
    "\\g{1}",
    "\\g{-1}",
    "\\q",
    "\\",
    "\\E",
    "${1:-def}",
    "${1:+yes:no}",
    "${name:-def}",
    "${name:+yes:no}",
    "${1:x}",
    "${1:-${2:-z}}",
    "${1:+${2}:${3:-q}}",
    "${1:-\\Ua\\E}",
    "é",
    "😀",
    "\\U😀\\E",
];

/// Subjects used for the deterministic sweeps.
const SUB_SUBJECTS: &[&str] = &[
    "",
    "a",
    "b",
    "ab",
    "abc",
    "abcabc",
    "aaa",
    "a1b2c3",
    "xayaza",
    " a a ",
    "2024-01-31",
    "hello world",
    "A",
    "aéb",
    "日本語",
    "😀a😀",
    "\0a\0",
    "abababababab",
];

/// A random replacement built from the pieces the scanner special-cases.
fn gen_replacement(rng: &mut Rng) -> Vec<u8> {
    const PIECES: &[&str] = &[
        "", "X", "y", "ab", "-", " ", "\n", "0", "$0", "$1", "$2", "$3", "${1}", "${2}", "$&",
        "$$", "$`", "$'", "$_", "$+", "$name", "${name}", "$<name>", "${*MARK}", "\\U", "\\L",
        "\\u", "\\l", "\\E", "\\Q", "\\n", "\\t", "\\x{41}", "\\a", "\\e", "\\f", "\\r", "\\0",
        "${1:-d}", "${1:+y:n}", "${name:-d}", "${name:+y:n}", "\\g<name>", "\\g1", "\\g{2}", "$",
        "{", "}", ":", "\\", "\\q", "é", "😀", "$9", "${zz}",
    ];
    let n = rng.below(5);
    let mut s = Vec::new();
    for _ in 0..n {
        s.extend_from_slice(rng.pick(PIECES).as_bytes());
    }
    s
}

// ============================================================ call wrappers

/// Calls `pcre2_substitute` and logs rc, `*blength`, and the *whole* output
/// buffer including the guard region.
#[allow(clippy::too_many_arguments)]
unsafe fn call_sub(
    api: &Api,
    code: Code,
    subject: *const u8,
    length: Sz,
    start_offset: Sz,
    options: u32,
    md: MatchData,
    mc: MContext,
    repl: *const u8,
    rlength: Sz,
    bufcap: usize,
    l: &mut Log,
) -> c_int {
    let mut buf = vec![0xAAu8; bufcap + GUARD];
    let mut blen: Sz = bufcap;
    let rc = (api.substitute)(
        code,
        subject,
        length,
        start_offset,
        options,
        md,
        mc,
        repl,
        rlength,
        buf.as_mut_ptr(),
        &mut blen,
    );
    l.tag("s").i(rc as i64).u(blen as u64).b(&buf);
    rc
}

/// Logs only the parts of a match data block whose contents the API actually
/// defines: the rc, the ovector count, the match-data size, and — for a
/// *successful* match — the ovector pairs the match set, the start char and the
/// mark. After a no-match the ovector contents are explicitly unspecified (the
/// matcher simply leaves the block as `pcre2_match_data_create` left it), so
/// they must never be logged.
unsafe fn log_md(api: &Api, md: MatchData, rc: c_int, l: &mut Log) {
    l.tag("md").i(rc as i64);
    if md.is_null() {
        return;
    }
    let n = (api.get_ovector_count)(md);
    l.u(n as u64);
    // Which ovector pairs are defined: `rc` pairs on success, *all* of them
    // when rc == 0 (the ovector was too small for every capture), and just the
    // first pair after a partial match. Nothing at all otherwise.
    let pairs = if rc > 0 {
        (rc as usize).min(n as usize)
    } else if rc == 0 {
        n as usize
    } else if rc == ERR_PARTIAL {
        1usize.min(n as usize)
    } else {
        0
    };
    if pairs > 0 {
        let ov = (api.get_ovector_pointer)(md);
        if !ov.is_null() {
            for i in 0..(2 * pairs) {
                l.u(*ov.add(i) as u64);
            }
        }
        l.u((api.get_startchar)(md) as u64);
        let mk = (api.get_mark)(md);
        l.i(mk.is_null() as i64);
        if !mk.is_null() {
            l.b(&cstr(mk));
        }
    }
    l.u((api.get_match_data_size)(md) as u64);
}

/// Logs only the *shape* of a match data block (ovector count and block size),
/// for use after a call that leaves the block's contents unspecified.
unsafe fn log_md_shape(api: &Api, md: MatchData, l: &mut Log) {
    l.tag("mdshape");
    if md.is_null() {
        return;
    }
    l.u((api.get_ovector_count)(md) as u64);
    l.u((api.get_match_data_size)(md) as u64);
}

/// Snapshot of the raw ovector. Used only for *equality* comparisons inside one
/// library (the values themselves may be unspecified, but "did the call change
/// them?" is a well-defined, comparable observation).
unsafe fn ovec_snapshot(api: &Api, md: MatchData) -> Vec<Sz> {
    if md.is_null() {
        return Vec::new();
    }
    let n = (api.get_ovector_count)(md) as usize;
    let ov = (api.get_ovector_pointer)(md);
    if ov.is_null() {
        return Vec::new();
    }
    (0..2 * n).map(|i| *ov.add(i)).collect()
}

/// Compile + substitute + free, for the common case (no contexts, offset 0).
fn diff_sub(
    label: &str,
    pat: &[u8],
    patopts: u32,
    subj: &[u8],
    repl: &[u8],
    options: u32,
    caps: &[usize],
) {
    diff(label, |api| {
        let mut l = Log::new();
        unsafe {
            let code = compile_logged(api, pat, pat.len(), patopts, std::ptr::null_mut(), &mut l);
            if code.is_null() {
                return l;
            }
            for &cap in caps {
                call_sub(
                    api,
                    code,
                    subj.as_ptr(),
                    subj.len(),
                    0,
                    options,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    repl.as_ptr(),
                    repl.len(),
                    cap,
                    &mut l,
                );
            }
            (api.code_free)(code);
        }
        l
    });
}

// ============================================================ row 143

/// Randomized pattern × subject × replacement with no options at all.
#[test]
fn sub_random_no_options() {
    let mut rng = Rng::new(0x5AB5_0001);
    for iter in 0..260000 {
        let pat = SUB_PATTERNS[rng.below(SUB_PATTERNS.len())];
        let subj = SUB_SUBJECTS[rng.below(SUB_SUBJECTS.len())];
        let repl = gen_replacement(&mut rng);
        diff_sub(
            &format!("plain iter={iter} pat={pat:?} subj={subj:?} repl={repl:?}"),
            pat.as_bytes(),
            0,
            subj.as_bytes(),
            &repl,
            0,
            &[256],
        );
    }
}

// ============================================================ rows 143-148, 150

/// The full grid: every hand-picked pattern × subject × replacement, each run
/// under no options and under every single substitute option bit plus a few
/// interesting pairs. The pattern is compiled once per grid point and the
/// option masks are looped inside the same `diff` closure.
#[test]
fn sub_grid_all_option_bits() {
    const OPTS: &[u32] = &[
        0,
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_LITERAL,
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_UNSET_EMPTY,
        PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY | PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
            | PCRE2_SUBSTITUTE_UNSET_EMPTY,
        PCRE2_SUBSTITUTE_LITERAL | PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
            | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY
            | PCRE2_SUBSTITUTE_UNSET_EMPTY,
    ];
    for (pi, pat) in SUB_PATTERNS.iter().enumerate() {
        for (si, subj) in SUB_SUBJECTS.iter().enumerate() {
            for (ri, repl) in REPLACEMENTS.iter().enumerate() {
                diff(
                    &format!("grid p[{pi}]={pat:?} s[{si}]={subj:?} r[{ri}]={repl:?}"),
                    |api| {
                        let mut l = Log::new();
                        unsafe {
                            let code = compile_logged(
                                api,
                                pat.as_bytes(),
                                pat.len(),
                                0,
                                std::ptr::null_mut(),
                                &mut l,
                            );
                            if code.is_null() {
                                return l;
                            }
                            for &o in OPTS {
                                call_sub(
                                    api,
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    0,
                                    o,
                                    std::ptr::null_mut(),
                                    std::ptr::null_mut(),
                                    repl.as_ptr(),
                                    repl.len(),
                                    256,
                                    &mut l,
                                );
                            }
                            (api.code_free)(code);
                        }
                        l
                    },
                );
            }
        }
    }
}

// ============================================================ row 146

/// `PCRE2_SUBSTITUTE_EXTENDED`: case escapes, `${n:-default}`, `${n:+yes:no}`,
/// and every control/hex escape, including nested and malformed forms.
#[test]
fn sub_extended_replacement_syntax() {
    let reps: &[&str] = &[
        "\\U$1\\E",
        "\\L$1\\E",
        "\\u$1",
        "\\l$1",
        "\\u\\L$1\\E",
        "\\l\\U$1\\E",
        "\\U$1\\L$2\\E\\E",
        "\\U",
        "\\L",
        "\\u",
        "\\l",
        "\\E",
        "\\E\\E",
        "\\Uabc",
        "a\\Ub\\Ec",
        "\\l\\u$0",
        "\\u\\u$0",
        "\\U${1:-def}\\E",
        "${1:-\\Uzz\\E}",
        "${1:+\\Uy\\E:\\Ln\\E}",
        "${1:-}",
        "${1:+}",
        "${1:+:}",
        "${1:+a:}",
        "${1:+:b}",
        "${name:-d}",
        "${name:+y:n}",
        "${99:-fallback}",
        "${zz:-fallback}",
        "${1:-${2:-${3:-deep}}}",
        "${1:+${2:+a:b}:c}",
        "${1:-\\}}",
        "${1:-\\:}",
        "${1:-a:b}",
        "${1:x}",
        "${1:",
        "${1:-",
        "${1:+a",
        "\\a",
        "\\e",
        "\\f",
        "\\n",
        "\\r",
        "\\t",
        "\\0",
        "\\00",
        "\\000",
        "\\1",
        "\\2",
        "\\x{0}",
        "\\x{41}",
        "\\x{7f}",
        "\\x{80}",
        "\\x{ff}",
        "\\x{100}",
        "\\x{263A}",
        "\\x{10FFFF}",
        "\\x{110000}",
        "\\x{}",
        "\\x",
        "\\xzz",
        "\\o{0}",
        "\\o{377}",
        "\\o{}",
        "\\cA",
        "\\c",
        "\\Qa$1\\Eb",
        "\\Q\\U$1\\E\\E",
        "\\g<name>",
        "\\g<zz>",
        "\\g<>",
        "\\g1",
        "\\g{1}",
        "\\g{-1}",
        "\\g{99}",
        "\\g",
        "\\b",
        "\\v",
        "\\q",
        "\\z",
        "\\",
        "\\$1",
        "\\\\$1",
    ];
    for (pi, pat) in ["(a)(b)?", "(?<name>a)(b)", "a", "(a)|(b)", "(?<name>\\w+)"]
        .iter()
        .enumerate()
    {
        for subj in ["", "a", "ab", "abab", "b"] {
            for (ri, repl) in reps.iter().enumerate() {
                for extra in [0u32, PCRE2_SUBSTITUTE_GLOBAL, PCRE2_SUBSTITUTE_UNSET_EMPTY] {
                    diff_sub(
                        &format!("ext p[{pi}] s={subj:?} r[{ri}]={repl:?} x={extra:#x}"),
                        pat.as_bytes(),
                        0,
                        subj.as_bytes(),
                        repl.as_bytes(),
                        PCRE2_SUBSTITUTE_EXTENDED | extra,
                        &[256],
                    );
                }
            }
        }
    }
}

// ============================================================ rows 147, 148

/// Unset groups (`UNSET_EMPTY`) and unknown names/numbers (`UNKNOWN_UNSET`).
#[test]
fn sub_unset_and_unknown() {
    let pats = [
        "(a)?(b)?",
        "(a)|(b)",
        "(?<x>a)?(?<y>b)?",
        "(?J)(?<n>a)|(?<n>b)",
        "(a)(?:(b))?",
        "(?<x>a)(?<y>b)?",
    ];
    let reps = [
        "$1", "$2", "${1}", "${2}", "$x", "$y", "$n", "${x}", "${y}", "${n}", "$3", "${3}", "$9",
        "${zz}", "$zz", "$<zz>", "$<x>", "$+", "$1$2$3", "${1:-U1}", "${2:-U2}", "${1:+S1:N1}",
        "${zz:-Z}", "${zz:+A:B}", "${9:-N}", "\\g<zz>", "\\g<x>",
    ];
    for (pi, pat) in pats.iter().enumerate() {
        for subj in ["", "a", "b", "ab", "ba"] {
            for (ri, repl) in reps.iter().enumerate() {
                for bits in [
                    0u32,
                    PCRE2_SUBSTITUTE_UNSET_EMPTY,
                    PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                    PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                    PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY,
                    PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                    PCRE2_SUBSTITUTE_EXTENDED
                        | PCRE2_SUBSTITUTE_UNSET_EMPTY
                        | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                ] {
                    diff_sub(
                        &format!("unset p[{pi}]={pat:?} s={subj:?} r[{ri}]={repl:?} b={bits:#x}"),
                        pat.as_bytes(),
                        0,
                        subj.as_bytes(),
                        repl.as_bytes(),
                        bits,
                        &[256],
                    );
                }
            }
        }
    }
}

// ============================================================ row 149

/// `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH`: sweep every buffer size from 0 up past
/// the exact required length and verify the value written to `*blength`, then
/// re-run with exactly the reported size (the documented two-call protocol).
#[test]
fn sub_overflow_length() {
    let cases: &[(&str, &str, &str, u32)] = &[
        ("a", "aaa", "XY", 0),
        ("a", "aaa", "XY", PCRE2_SUBSTITUTE_GLOBAL),
        ("(a)", "abcabc", "[$1]", PCRE2_SUBSTITUTE_GLOBAL),
        ("(a)", "abcabc", "[$1]", PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY),
        ("\\w+", "hello world", "<$0>", PCRE2_SUBSTITUTE_GLOBAL),
        ("", "abc", "-", PCRE2_SUBSTITUTE_GLOBAL),
        ("b", "abc", "very long replacement text indeed", 0),
        ("(?<n>a)", "aaa", "\\U${n}\\E!", PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED),
        ("x", "abc", "Y", 0),
        ("a", "", "Y", 0),
    ];
    for (i, (pat, subj, repl, bits)) in cases.iter().enumerate() {
        diff(&format!("ovf[{i}] pat={pat:?} subj={subj:?}"), |api| {
            let mut l = Log::new();
            unsafe {
                let code = compile_logged(
                    api,
                    pat.as_bytes(),
                    pat.len(),
                    0,
                    std::ptr::null_mut(),
                    &mut l,
                );
                if code.is_null() {
                    return l;
                }
                for cap in 0..64usize {
                    // Without OVERFLOW_LENGTH: immediate NOMEMORY.
                    call_sub(
                        api,
                        code,
                        subj.as_ptr(),
                        subj.len(),
                        0,
                        *bits,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        repl.as_ptr(),
                        repl.len(),
                        cap,
                        &mut l,
                    );
                    // With OVERFLOW_LENGTH: the required length in *blength.
                    let mut buf = vec![0xAAu8; cap + GUARD];
                    let mut blen: Sz = cap;
                    let rc = (api.substitute)(
                        code,
                        subj.as_ptr(),
                        subj.len(),
                        0,
                        *bits | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        repl.as_ptr(),
                        repl.len(),
                        buf.as_mut_ptr(),
                        &mut blen,
                    );
                    l.tag("o").i(rc as i64).u(blen as u64).b(&buf);
                    // Two-call protocol: retry with exactly the reported size.
                    if rc == ERR_NOMEMORY && blen != PCRE2_UNSET && blen < 4096 {
                        call_sub(
                            api,
                            code,
                            subj.as_ptr(),
                            subj.len(),
                            0,
                            *bits | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            repl.as_ptr(),
                            repl.len(),
                            blen,
                            &mut l,
                        );
                        call_sub(
                            api,
                            code,
                            subj.as_ptr(),
                            subj.len(),
                            0,
                            *bits,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            repl.as_ptr(),
                            repl.len(),
                            blen,
                            &mut l,
                        );
                    }
                }
                (api.code_free)(code);
            }
            l
        });
    }
}

// ============================================================ row 152

/// All 32 combinations of GLOBAL × EXTENDED × REPLACEMENT_ONLY ×
/// OVERFLOW_LENGTH × UNSET_EMPTY.
#[test]
fn sub_option_combinations_32() {
    let bits = [
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_UNSET_EMPTY,
    ];
    let cases: &[(&str, &str, &str)] = &[
        ("(a)(b)?", "abcab", "[$1|$2]"),
        ("(?<n>\\w)", "a1b2", "\\U${n}\\E"),
        ("a", "aaaa", "XY"),
        ("", "abc", "."),
        ("(a)|(b)", "ab", "${1:-N1}${2:-N2}"),
        ("x", "abc", "Q"),
        ("(a)?b", "b", "<$1>"),
        ("\\w+", "hello world", "\\u$0"),
    ];
    for mask in 0u32..32 {
        let mut o = 0u32;
        for (k, b) in bits.iter().enumerate() {
            if mask & (1 << k) != 0 {
                o |= b;
            }
        }
        for (i, (pat, subj, repl)) in cases.iter().enumerate() {
            diff_sub(
                &format!("combo mask={mask} case={i} o={o:#x}"),
                pat.as_bytes(),
                0,
                subj.as_bytes(),
                repl.as_bytes(),
                o,
                &[256, 8, 1, 0],
            );
        }
    }
}

// ============================================================ row 153

/// The `$1`, `${1}`, `$name`, `${name}`, `$0`, `$$`, `$<name>`, `$&`, `` $` ``,
/// `$'`, `$_`, `$+`, `${*MARK}` and bare-trailing-`$` forms.
#[test]
fn sub_replacement_forms() {
    let forms: &[&str] = &[
        "$0", "${0}", "$1", "${1}", "$2", "${2}", "$10", "${10}", "$00", "$name", "${name}",
        "$<name>", "$$", "$", "a$", "$&", "$`", "$'", "$_", "$+", "${*MARK}", "${*mark}",
        "${*MARKX}", "$*MARK", "${*name}", "$+{name}", "${1}${2}$0", "$1$$1", "${name}$name",
        "$<name>x", "$<name", "${name", "$ ", "$\n", "${ }", "$-", "${-1}",
    ];
    let pats = [
        "(?<name>a)(b)?",
        "(a)(b)",
        "(*MARK:mk)(a)",
        "a(*MARK:m2)b",
        "(?<name>\\w+)",
        "a",
    ];
    for (pi, pat) in pats.iter().enumerate() {
        for subj in ["", "a", "ab", "abab", "xaby"] {
            for (ri, repl) in forms.iter().enumerate() {
                for bits in [
                    0u32,
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_UNKNOWN_UNSET | PCRE2_SUBSTITUTE_UNSET_EMPTY,
                ] {
                    diff_sub(
                        &format!("form p[{pi}] s={subj:?} r[{ri}]={repl:?} b={bits:#x}"),
                        pat.as_bytes(),
                        0,
                        subj.as_bytes(),
                        repl.as_bytes(),
                        bits,
                        &[256],
                    );
                }
            }
        }
    }
    // A group name longer than MAX_NAME_SIZE (128) must be rejected identically.
    let long_name: String = std::iter::repeat('n').take(200).collect();
    for form in [
        format!("${{{long_name}}}"),
        format!("${long_name}"),
        format!("\\g<{long_name}>"),
    ] {
        diff_sub(
            &format!("longname {}", form.len()),
            b"(?<name>a)",
            0,
            b"a",
            form.as_bytes(),
            PCRE2_SUBSTITUTE_EXTENDED,
            &[256],
        );
    }
}

// ============================================================ row 151

/// `PCRE2_SUBSTITUTE_MATCHED` with a valid pre-existing match produced by
/// `pcre2_match`, including the global continuation from that match.
#[test]
fn sub_matched_existing_match() {
    let cases: &[(&str, &str, &str)] = &[
        ("a", "aaa", "X"),
        ("(a)", "abcabc", "[$1]"),
        ("(a)(b)?", "abab", "<$1$2>"),
        ("\\w+", "hello world", "($0)"),
        ("", "abc", "-"),
        ("x", "abc", "Q"),
        ("(?<n>a)", "aXa", "\\U${n}\\E"),
        ("b", "abc", "long-ish replacement"),
    ];
    for (i, (pat, subj, repl)) in cases.iter().enumerate() {
        for so in [0usize, 1] {
            for matchopts in [0u32, PCRE2_NOTBOL, PCRE2_NOTEMPTY, PCRE2_ANCHORED] {
                for subbits in [
                    0u32,
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                    PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                ] {
                    diff(
                        &format!("matched[{i}] so={so} mo={matchopts:#x} sb={subbits:#x}"),
                        |api| {
                            let mut l = Log::new();
                            unsafe {
                                let code = compile_logged(
                                    api,
                                    pat.as_bytes(),
                                    pat.len(),
                                    0,
                                    std::ptr::null_mut(),
                                    &mut l,
                                );
                                if code.is_null() {
                                    return l;
                                }
                                let md =
                                    (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                                if so > subj.len() {
                                    (api.match_data_free)(md);
                                    (api.code_free)(code);
                                    return l;
                                }
                                let rc = (api.do_match)(
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    so,
                                    matchopts,
                                    md,
                                    std::ptr::null_mut(),
                                );
                                log_md(api, md, rc, &mut l);
                                let before = ovec_snapshot(api, md);
                                call_sub(
                                    api,
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    so,
                                    matchopts | PCRE2_SUBSTITUTE_MATCHED | subbits,
                                    md,
                                    std::ptr::null_mut(),
                                    repl.as_ptr(),
                                    repl.len(),
                                    256,
                                    &mut l,
                                );
                                // The external match data must be untouched:
                                // compare the ovector against its own snapshot
                                // (values may be unspecified, "unchanged" is
                                // not).
                                let after = ovec_snapshot(api, md);
                                l.tag("untouched").i((before == after) as i64);
                                log_md(api, md, rc, &mut l);
                                // And the same substitution without MATCHED.
                                call_sub(
                                    api,
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    so,
                                    matchopts | subbits,
                                    md,
                                    std::ptr::null_mut(),
                                    repl.as_ptr(),
                                    repl.len(),
                                    256,
                                    &mut l,
                                );
                                // After a substitution *without* MATCHED the
                                // block holds the last internal (no-)match, so
                                // only its shape is well defined.
                                log_md_shape(api, md, &mut l);
                                (api.match_data_free)(md);
                                (api.code_free)(code);
                            }
                            l
                        },
                    );
                }
            }
        }
    }
}

/// `PCRE2_SUBSTITUTE_MATCHED` combined with `PCRE2_COPY_MATCHED_SUBJECT`,
/// where the subject pointer differs but the contents match.
#[test]
fn sub_matched_copied_subject() {
    for (i, (pat, subj, repl)) in [
        ("(a)", "abcabc", "[$1]"),
        ("a", "aaa", "X"),
        ("", "ab", "-"),
    ]
    .iter()
    .enumerate()
    {
        for gbl in [0u32, PCRE2_SUBSTITUTE_GLOBAL] {
            diff(&format!("copysubj[{i}] g={gbl:#x}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile_logged(
                        api,
                        pat.as_bytes(),
                        pat.len(),
                        0,
                        std::ptr::null_mut(),
                        &mut l,
                    );
                    if code.is_null() {
                        return l;
                    }
                    let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                    let owned = subj.as_bytes().to_vec();
                    let rc = (api.do_match)(
                        code,
                        owned.as_ptr(),
                        owned.len(),
                        0,
                        PCRE2_COPY_MATCHED_SUBJECT,
                        md,
                        std::ptr::null_mut(),
                    );
                    log_md(api, md, rc, &mut l);
                    // A *different* buffer with identical content: allowed only
                    // because the match data owns a copy.
                    let other = subj.as_bytes().to_vec();
                    call_sub(
                        api,
                        code,
                        other.as_ptr(),
                        other.len(),
                        0,
                        PCRE2_COPY_MATCHED_SUBJECT | PCRE2_SUBSTITUTE_MATCHED | gbl,
                        md,
                        std::ptr::null_mut(),
                        repl.as_ptr(),
                        repl.len(),
                        256,
                        &mut l,
                    );
                    // Different content of the same length => DIFFSUBSSUBJECT.
                    let mut wrong = subj.as_bytes().to_vec();
                    if let Some(b) = wrong.last_mut() {
                        *b ^= 0x20;
                    }
                    call_sub(
                        api,
                        code,
                        wrong.as_ptr(),
                        wrong.len(),
                        0,
                        PCRE2_COPY_MATCHED_SUBJECT | PCRE2_SUBSTITUTE_MATCHED | gbl,
                        md,
                        std::ptr::null_mut(),
                        repl.as_ptr(),
                        repl.len(),
                        256,
                        &mut l,
                    );
                    (api.match_data_free)(md);
                    (api.code_free)(code);
                }
                l
            });
        }
    }
}

// ============================================================ row 154

/// UTF patterns / subjects / replacements, with and without
/// `PCRE2_NO_UTF_CHECK`. Invalid UTF is only ever combined with the check
/// *enabled*, since `NO_UTF_CHECK` on invalid input is undefined behaviour.
#[test]
fn sub_utf() {
    let pats = ["(.)", "(\\X)", "\\w", "(?<n>.)", "é", "(😀)", ".", "(\\p{L})"];
    let good_subjects: &[&str] = &["", "a", "aéb", "日本語", "😀😀", "aéb😀", "A\u{85}B"];
    let good_repls: &[&str] = &[
        "", "X", "é", "😀", "[$0]", "[$1]", "\\U$0\\E", "\\u$0", "\\L$0\\E", "\\x{263A}",
        "\\x{10FFFF}", "${n}", "${n:-é}",
    ];
    let bad_utf: &[&[u8]] = &[
        &[0x80],
        &[0xFF],
        &[0xC2],
        &[0xC2, 0x41],
        &[0xE0, 0xA0],
        &[0xED, 0xA0, 0x80],
        &[0xF4, 0x90, 0x80, 0x80],
        &[b'a', 0x80, b'b'],
    ];
    for (pi, pat) in pats.iter().enumerate() {
        for popts in [PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_UTF | PCRE2_CASELESS] {
            for subj in good_subjects {
                for (ri, repl) in good_repls.iter().enumerate() {
                    for nuc in [0u32, PCRE2_NO_UTF_CHECK] {
                        for ext in [0u32, PCRE2_SUBSTITUTE_EXTENDED] {
                            diff_sub(
                                &format!(
                                    "utf p[{pi}] po={popts:#x} s={subj:?} r[{ri}] n={nuc:#x} e={ext:#x}"
                                ),
                                pat.as_bytes(),
                                popts,
                                subj.as_bytes(),
                                repl.as_bytes(),
                                nuc | ext | PCRE2_SUBSTITUTE_GLOBAL,
                                &[256],
                            );
                        }
                    }
                }
            }
        }
    }
    // Invalid UTF in the subject and in the replacement, check ENABLED.
    for (pi, pat) in ["(.)", "\\w", "a"].iter().enumerate() {
        for (bi, bad) in bad_utf.iter().enumerate() {
            diff_sub(
                &format!("utfbadsubj p[{pi}] b[{bi}]"),
                pat.as_bytes(),
                PCRE2_UTF,
                bad,
                b"[$0]",
                PCRE2_SUBSTITUTE_GLOBAL,
                &[256],
            );
            diff_sub(
                &format!("utfbadrepl p[{pi}] b[{bi}]"),
                pat.as_bytes(),
                PCRE2_UTF,
                b"a",
                bad,
                PCRE2_SUBSTITUTE_GLOBAL,
                &[256],
            );
            diff_sub(
                &format!("utfbadrepl_ext p[{pi}] b[{bi}]"),
                pat.as_bytes(),
                PCRE2_UTF,
                b"a",
                bad,
                PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
                &[256],
            );
        }
    }
    // Non-UTF pattern with raw bytes everywhere (always legal).
    for (bi, bad) in bad_utf.iter().enumerate() {
        for repl in [b"X".as_ref(), b"[$0]".as_ref()] {
            diff_sub(
                &format!("rawbytes b[{bi}] r={repl:?}"),
                b".",
                0,
                bad,
                repl,
                PCRE2_SUBSTITUTE_GLOBAL,
                &[256],
            );
        }
    }
    // start_offset in the middle of a UTF character.
    for so in 0..4usize {
        diff_sub_offset(
            &format!("utfoffset so={so}"),
            b"(.)",
            PCRE2_UTF,
            "😀a".as_bytes(),
            b"[$1]",
            PCRE2_SUBSTITUTE_GLOBAL,
            so,
        );
        diff_sub_offset(
            &format!("utfoffset_nuc so={so}"),
            b"(.)",
            PCRE2_UTF,
            "😀a".as_bytes(),
            b"[$1]",
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_NO_UTF_CHECK,
            so,
        );
    }
}

fn diff_sub_offset(
    label: &str,
    pat: &[u8],
    patopts: u32,
    subj: &[u8],
    repl: &[u8],
    options: u32,
    so: usize,
) {
    diff(label, |api| {
        let mut l = Log::new();
        unsafe {
            let code = compile_logged(api, pat, pat.len(), patopts, std::ptr::null_mut(), &mut l);
            if code.is_null() {
                return l;
            }
            call_sub(
                api,
                code,
                subj.as_ptr(),
                subj.len(),
                so,
                options,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                repl.as_ptr(),
                repl.len(),
                256,
                &mut l,
            );
            (api.code_free)(code);
        }
        l
    });
}

// ============================================================ row 155

/// State handed to the substitute callout. Only *contents* are logged, never
/// addresses; pointer identity is reduced to a boolean against a known base.
struct ScbState {
    subject: *const u8,
    subject_len: usize,
    buffer: *const u8,
    /// 0 => always 0, 1 => always 1, 2 => always -7,
    /// 3 => 1 when subscount is odd, 4 => -7 on the second call.
    policy: u32,
    calls: u32,
    log: Vec<u8>,
}

unsafe extern "C" fn scb_cb(b: *mut SubstituteCalloutBlock, data: *mut c_void) -> c_int {
    let st = &mut *(data as *mut ScbState);
    let b = &*b;
    st.calls += 1;
    let mut v: Vec<u8> = Vec::new();
    v.extend_from_slice(b"SCB");
    v.extend_from_slice(&st.calls.to_le_bytes());
    v.extend_from_slice(&b.version.to_le_bytes());
    // Pointer identity as booleans (addresses differ between libraries).
    v.push((b.input == st.subject) as u8);
    v.push((b.output == st.buffer) as u8);
    v.push(b.input.is_null() as u8);
    v.push(b.output.is_null() as u8);
    v.push(b.ovector.is_null() as u8);
    // Contents of the input (== the subject we passed in).
    if !b.input.is_null() {
        v.extend_from_slice(&(st.subject_len as u64).to_le_bytes());
        v.extend_from_slice(std::slice::from_raw_parts(b.input, st.subject_len));
    }
    v.extend_from_slice(&(b.output_offsets[0] as u64).to_le_bytes());
    v.extend_from_slice(&(b.output_offsets[1] as u64).to_le_bytes());
    // The freshly-written replacement text.
    if !b.output.is_null() && b.output_offsets[1] >= b.output_offsets[0] {
        let n = b.output_offsets[1] - b.output_offsets[0];
        v.extend_from_slice(&(n as u64).to_le_bytes());
        v.extend_from_slice(std::slice::from_raw_parts(
            b.output.add(b.output_offsets[0]),
            n,
        ));
    }
    v.extend_from_slice(&b.oveccount.to_le_bytes());
    v.extend_from_slice(&b.subscount.to_le_bytes());
    if !b.ovector.is_null() {
        for i in 0..(2 * b.oveccount as usize) {
            v.extend_from_slice(&(*b.ovector.add(i) as u64).to_le_bytes());
        }
    }
    let rc = match st.policy {
        0 => 0,
        1 => 1,
        2 => -7,
        3 => {
            if b.subscount % 2 == 1 {
                1
            } else {
                0
            }
        }
        _ => {
            if st.calls >= 2 {
                -7
            } else {
                0
            }
        }
    };
    v.extend_from_slice(&(rc as i64).to_le_bytes());
    st.log.extend_from_slice(&v);
    rc
}

#[test]
fn sub_substitute_callout() {
    let cases: &[(&str, &str, &str)] = &[
        ("a", "aaa", "X"),
        ("(a)", "abcabc", "[$1]"),
        ("(a)(b)?", "abab", "<$1$2>"),
        ("\\w+", "hello world x", "($0)"),
        ("", "abc", "-"),
        ("x", "abc", "Q"),
        ("(?<n>a)", "aXa", "\\U${n}\\E"),
        ("(*MARK:mk)a", "aa", "${*MARK}"),
        ("b", "abcb", "much longer replacement"),
        ("a*", "baaab", "<$0>"),
    ];
    for (i, (pat, subj, repl)) in cases.iter().enumerate() {
        for policy in 0u32..5 {
            for bits in [
                0u32,
                PCRE2_SUBSTITUTE_GLOBAL,
                PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
                PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_LITERAL,
            ] {
                for cap in [256usize, 6, 0] {
                    diff(
                        &format!("scb[{i}] pol={policy} b={bits:#x} cap={cap}"),
                        |api| {
                            let mut l = Log::new();
                            unsafe {
                                let code = compile_logged(
                                    api,
                                    pat.as_bytes(),
                                    pat.len(),
                                    0,
                                    std::ptr::null_mut(),
                                    &mut l,
                                );
                                if code.is_null() {
                                    return l;
                                }
                                let mc = (api.match_context_create)(std::ptr::null_mut());
                                assert!(!mc.is_null());
                                let mut buf = vec![0xAAu8; cap + GUARD];
                                let mut st = ScbState {
                                    subject: subj.as_ptr(),
                                    subject_len: subj.len(),
                                    buffer: buf.as_ptr(),
                                    policy,
                                    calls: 0,
                                    log: Vec::new(),
                                };
                                l.i((api.set_substitute_callout)(
                                    mc,
                                    Some(scb_cb),
                                    &mut st as *mut ScbState as *mut c_void,
                                ) as i64);
                                let mut blen: Sz = cap;
                                let rc = (api.substitute)(
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    0,
                                    bits,
                                    std::ptr::null_mut(),
                                    mc,
                                    repl.as_ptr(),
                                    repl.len(),
                                    buf.as_mut_ptr(),
                                    &mut blen,
                                );
                                l.tag("r")
                                    .i(rc as i64)
                                    .u(blen as u64)
                                    .b(&buf)
                                    .u(st.calls as u64)
                                    .b(&st.log);
                                // Clearing the callout must restore plain behaviour.
                                l.i((api.set_substitute_callout)(
                                    mc,
                                    None,
                                    std::ptr::null_mut(),
                                ) as i64);
                                call_sub(
                                    api,
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    0,
                                    bits,
                                    std::ptr::null_mut(),
                                    mc,
                                    repl.as_ptr(),
                                    repl.len(),
                                    cap,
                                    &mut l,
                                );
                                (api.match_context_free)(mc);
                                (api.code_free)(code);
                            }
                            l
                        },
                    );
                }
            }
        }
    }
}

// ============================================================ row 156

/// State for the substitute *case* callout.
struct CaseState {
    /// 0 => faithful ASCII transform, 1 => return `PCRE2_SIZE` max (error),
    /// 2 => claim double the length, 3 => claim len+1 without writing,
    /// 4 => claim 0 and write nothing.
    mode: u32,
    calls: u32,
    log: Vec<u8>,
}

/// Deterministic ASCII-only case transform, byte-wise so that the length is
/// preserved and in-place operation is safe.
fn ascii_case(b: u8, to_upper: bool) -> u8 {
    if to_upper {
        if b.is_ascii_lowercase() {
            b - 32
        } else {
            b
        }
    } else if b.is_ascii_uppercase() {
        b + 32
    } else {
        b
    }
}

unsafe extern "C" fn case_cb(
    input: *const u8,
    ilen: Sz,
    output: *mut u8,
    ocap: Sz,
    to_case: c_int,
    data: *mut c_void,
) -> Sz {
    let st = &mut *(data as *mut CaseState);
    st.calls += 1;
    // Snapshot the input *before* touching the (possibly aliasing) output.
    let src: Vec<u8> = if input.is_null() || ilen == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(input, ilen).to_vec()
    };
    st.log.extend_from_slice(b"CC");
    st.log.extend_from_slice(&st.calls.to_le_bytes());
    st.log.extend_from_slice(&(ilen as u64).to_le_bytes());
    st.log.extend_from_slice(&(ocap as u64).to_le_bytes());
    st.log.extend_from_slice(&(to_case as i64).to_le_bytes());
    st.log.extend_from_slice(&(input.is_null() as u8).to_le_bytes());
    st.log.extend_from_slice(&(output.is_null() as u8).to_le_bytes());
    st.log.extend_from_slice(&(src.len() as u64).to_le_bytes());
    st.log.extend_from_slice(&src);

    let ret: Sz = match st.mode {
        1 => !0usize,
        2 => {
            let needed = src.len() * 2;
            if needed <= ocap && !output.is_null() {
                for (i, &b) in src.iter().enumerate() {
                    let c = ascii_case(b, to_case == PCRE2_SUBSTITUTE_CASE_UPPER);
                    *output.add(2 * i) = c;
                    *output.add(2 * i + 1) = c;
                }
            }
            needed
        }
        3 => src.len() + 1,
        4 => 0,
        _ => {
            let needed = src.len();
            if needed <= ocap && !output.is_null() {
                for (i, &b) in src.iter().enumerate() {
                    let up = match to_case {
                        PCRE2_SUBSTITUTE_CASE_UPPER => true,
                        PCRE2_SUBSTITUTE_CASE_LOWER => false,
                        // TITLE_FIRST: upper-case the first character only.
                        _ => i == 0,
                    };
                    *output.add(i) = ascii_case(b, up);
                }
            }
            needed
        }
    };
    st.log.extend_from_slice(&(ret as u64).to_le_bytes());
    ret
}

#[test]
fn sub_substitute_case_callout() {
    let reps: &[&str] = &[
        "\\U$0\\E",
        "\\L$0\\E",
        "\\u$0",
        "\\l$0",
        "\\u\\L$0\\E",
        "\\l\\U$0\\E",
        "\\Uabc\\Edef",
        "x\\Uy$1z\\Ew",
        "\\L$1$2\\E",
        "\\U${*MARK}\\E",
        "\\u",
        "\\U",
        "\\U\\L$0\\E\\E",
        "\\U$1\\Emid\\L$2\\E",
        "$0",
        "\\Ué\\E",
        "\\U😀a\\E",
    ];
    let cases: &[(&str, &str)] = &[
        ("(a)(B)?", "aBaB"),
        ("(?i)(hello)", "Hello World"),
        ("(*MARK:mk)(a)", "aa"),
        ("(\\w)(\\w)?", "aB cD"),
        ("(.)", "aÉb"),
        ("x", "abc"),
    ];
    for (ci, (pat, subj)) in cases.iter().enumerate() {
        for popts in [0u32, PCRE2_UTF] {
            for mode in 0u32..5 {
                for (ri, repl) in reps.iter().enumerate() {
                    for cap in [256usize, 4] {
                        diff(
                            &format!("casecb[{ci}] po={popts:#x} m={mode} r[{ri}] cap={cap}"),
                            |api| {
                                let mut l = Log::new();
                                unsafe {
                                    let code = compile_logged(
                                        api,
                                        pat.as_bytes(),
                                        pat.len(),
                                        popts,
                                        std::ptr::null_mut(),
                                        &mut l,
                                    );
                                    if code.is_null() {
                                        return l;
                                    }
                                    let mc = (api.match_context_create)(std::ptr::null_mut());
                                    let mut st = CaseState {
                                        mode,
                                        calls: 0,
                                        log: Vec::new(),
                                    };
                                    l.i((api.set_substitute_case_callout)(
                                        mc,
                                        Some(case_cb),
                                        &mut st as *mut CaseState as *mut c_void,
                                    ) as i64);
                                    for bits in [
                                        PCRE2_SUBSTITUTE_EXTENDED,
                                        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                                        PCRE2_SUBSTITUTE_EXTENDED
                                            | PCRE2_SUBSTITUTE_GLOBAL
                                            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                                    ] {
                                        st.calls = 0;
                                        st.log.clear();
                                        call_sub(
                                            api,
                                            code,
                                            subj.as_ptr(),
                                            subj.len(),
                                            0,
                                            bits,
                                            std::ptr::null_mut(),
                                            mc,
                                            repl.as_ptr(),
                                            repl.len(),
                                            cap,
                                            &mut l,
                                        );
                                        l.u(st.calls as u64).b(&st.log);
                                    }
                                    // Without a callout: the library's own
                                    // default case transform.
                                    l.i((api.set_substitute_case_callout)(
                                        mc,
                                        None,
                                        std::ptr::null_mut(),
                                    ) as i64);
                                    for bits in [
                                        PCRE2_SUBSTITUTE_EXTENDED,
                                        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                                        PCRE2_SUBSTITUTE_EXTENDED
                                            | PCRE2_SUBSTITUTE_GLOBAL
                                            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                                    ] {
                                        call_sub(
                                            api,
                                            code,
                                            subj.as_ptr(),
                                            subj.len(),
                                            0,
                                            bits,
                                            std::ptr::null_mut(),
                                            mc,
                                            repl.as_ptr(),
                                            repl.len(),
                                            cap,
                                            &mut l,
                                        );
                                    }
                                    (api.match_context_free)(mc);
                                    (api.code_free)(code);
                                }
                                l
                            },
                        );
                    }
                }
            }
        }
    }
}

/// The default (no-callout) case transform on UTF/UCP/caseless patterns.
#[test]
fn sub_default_case_transform() {
    let reps = [
        "\\U$0\\E",
        "\\L$0\\E",
        "\\u$0",
        "\\l$0",
        "\\u\\L$0\\E",
        "\\l\\U$0\\E",
        "\\U$0",
        "\\L\\u$0",
        "\\U${*MARK}\\E",
        "\\U\\x{e9}\\E",
        "\\L\\x{c9}\\E",
        "\\Uß\\E",
        "\\Uİ\\E",
    ];
    for pat in ["(.+)", "(\\w+)", "(*MARK:mk)(.)", "(.)"] {
        for popts in [
            0u32,
            PCRE2_UTF,
            PCRE2_UCP,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_CASELESS,
        ] {
            for subj in ["", "aBcD", "ÉéÀà", "日本", "😀", "straße", "İi"] {
                for (ri, repl) in reps.iter().enumerate() {
                    diff_sub(
                        &format!("defcase pat={pat:?} po={popts:#x} s={subj:?} r[{ri}]"),
                        pat.as_bytes(),
                        popts,
                        subj.as_bytes(),
                        repl.as_bytes(),
                        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                        &[256, 3],
                    );
                }
            }
        }
    }
    // Compile-time extra options that affect casing tables.
    for extra in [
        0u32,
        PCRE2_EXTRA_TURKISH_CASING,
        PCRE2_EXTRA_CASELESS_RESTRICT,
        PCRE2_EXTRA_TURKISH_CASING | PCRE2_EXTRA_CASELESS_RESTRICT,
    ] {
        for popts in [PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_UTF | PCRE2_CASELESS] {
            for subj in ["İi", "Iı", "aBcD", "ÉéÀà", "ß", "ﬁ", "ǅǄǆ"] {
                for (ri, repl) in reps.iter().enumerate() {
                    diff(
                        &format!("turk x={extra:#x} po={popts:#x} s={subj:?} r[{ri}]"),
                        |api| {
                            let mut l = Log::new();
                            unsafe {
                                let cc = (api.compile_context_create)(std::ptr::null_mut());
                                l.i((api.set_compile_extra_options)(cc, extra) as i64);
                                let code =
                                    compile_logged(api, b"(.+)", 4, popts, cc, &mut l);
                                if !code.is_null() {
                                    for cap in [256usize, 3] {
                                        call_sub(
                                            api,
                                            code,
                                            subj.as_ptr(),
                                            subj.len(),
                                            0,
                                            PCRE2_SUBSTITUTE_EXTENDED
                                                | PCRE2_SUBSTITUTE_GLOBAL,
                                            std::ptr::null_mut(),
                                            std::ptr::null_mut(),
                                            repl.as_ptr(),
                                            repl.len(),
                                            cap,
                                            &mut l,
                                        );
                                    }
                                    (api.code_free)(code);
                                }
                                (api.compile_context_free)(cc);
                            }
                            l
                        },
                    );
                }
            }
        }
    }
    // Custom character tables (locale-independent, generated by the library
    // under test) drive the non-UTF casing path through `fcc_offset`.
    let raw_subjects: &[&[u8]] = &[b"aBcD", b"", b"xYz", &[0x80, 0xFF, b'a', b'B']];
    for subj in raw_subjects {
        for (ri, repl) in reps.iter().enumerate() {
            diff(&format!("tabcase s={subj:?} r[{ri}]"), |api| {
                let mut l = Log::new();
                unsafe {
                    let tables = (api.maketables)(std::ptr::null_mut());
                    let cc = (api.compile_context_create)(std::ptr::null_mut());
                    l.i((api.set_character_tables)(cc, tables) as i64);
                    let code = compile_logged(api, b"(.+)", 4, 0, cc, &mut l);
                    if !code.is_null() {
                        call_sub(
                            api,
                            code,
                            subj.as_ptr(),
                            subj.len(),
                            0,
                            PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            repl.as_ptr(),
                            repl.len(),
                            256,
                            &mut l,
                        );
                        (api.code_free)(code);
                    }
                    (api.compile_context_free)(cc);
                    (api.maketables_free)(std::ptr::null_mut(), tables);
                }
                l
            });
        }
    }
}

// ============================================================ row 157

/// `start_offset` 0 / middle / == length, and `PCRE2_ZERO_TERMINATED` for both
/// the subject and the replacement length.
#[test]
fn sub_start_offset_and_zero_terminated() {
    let cases: &[(&str, &str, &str)] = &[
        ("a", "aaa", "X"),
        ("(a)", "abcabc", "[$1]"),
        ("\\w+", "one two three", "<$0>"),
        ("", "abc", "-"),
        ("b", "ab", "Y"),
        ("(?<n>.)", "abc", "${n}${n}"),
    ];
    for (i, (pat, subj, repl)) in cases.iter().enumerate() {
        for bits in [
            0u32,
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        ] {
            diff(&format!("offzt[{i}] b={bits:#x}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile_logged(
                        api,
                        pat.as_bytes(),
                        pat.len(),
                        0,
                        std::ptr::null_mut(),
                        &mut l,
                    );
                    if code.is_null() {
                        return l;
                    }
                    // Every start offset 0..=len, plus len+1 (BADOFFSET).
                    for so in 0..=(subj.len() + 1) {
                        call_sub(
                            api,
                            code,
                            subj.as_ptr(),
                            subj.len(),
                            so,
                            bits,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            repl.as_ptr(),
                            repl.len(),
                            256,
                            &mut l,
                        );
                    }
                    // Zero-terminated subject and/or replacement.
                    let mut zs = subj.as_bytes().to_vec();
                    zs.push(0);
                    let mut zr = repl.as_bytes().to_vec();
                    zr.push(0);
                    for (slen, rlen) in [
                        (PCRE2_ZERO_TERMINATED, repl.len()),
                        (subj.len(), PCRE2_ZERO_TERMINATED),
                        (PCRE2_ZERO_TERMINATED, PCRE2_ZERO_TERMINATED),
                    ] {
                        call_sub(
                            api,
                            code,
                            zs.as_ptr(),
                            slen,
                            0,
                            bits,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            zr.as_ptr(),
                            rlen,
                            256,
                            &mut l,
                        );
                    }
                    // NULL subject / replacement with length 0 (legal: treated
                    // as empty strings).
                    call_sub(
                        api,
                        code,
                        std::ptr::null(),
                        0,
                        0,
                        bits,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        repl.as_ptr(),
                        repl.len(),
                        256,
                        &mut l,
                    );
                    call_sub(
                        api,
                        code,
                        subj.as_ptr(),
                        subj.len(),
                        0,
                        bits,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null(),
                        0,
                        256,
                        &mut l,
                    );
                    call_sub(
                        api,
                        code,
                        std::ptr::null(),
                        0,
                        0,
                        bits,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null(),
                        0,
                        256,
                        &mut l,
                    );
                    (api.code_free)(code);
                }
                l
            });
        }
    }
    // A subject with an embedded NUL: explicit length sees all of it,
    // ZERO_TERMINATED stops at the NUL.
    for (i, subj) in ["a\0b", "\0ab", "ab\0"].iter().enumerate() {
        let mut z = subj.as_bytes().to_vec();
        z.push(0);
        diff(&format!("embnul[{i}]"), |api| {
            let mut l = Log::new();
            unsafe {
                let code =
                    compile_logged(api, b".", 1, 0, std::ptr::null_mut(), &mut l);
                for slen in [z.len() - 1, PCRE2_ZERO_TERMINATED] {
                    call_sub(
                        api,
                        code,
                        z.as_ptr(),
                        slen,
                        0,
                        PCRE2_SUBSTITUTE_GLOBAL,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        b"<$0>".as_ptr(),
                        4,
                        256,
                        &mut l,
                    );
                }
                (api.code_free)(code);
            }
            l
        });
    }
}

// ============================================================ row 158

/// Empty replacement, empty subject, and a replacement far longer than the
/// subject (including repeated global expansion).
#[test]
fn sub_empty_and_oversized() {
    let long: String = std::iter::repeat("LONG-").take(40).collect();
    let cases: &[(&str, &str, &str)] = &[
        ("a", "", ""),
        ("", "", ""),
        ("", "", "X"),
        ("a", "aaa", ""),
        ("", "abc", ""),
        ("a*", "", "X"),
        ("x?", "", "Y"),
        ("a", "a", long.as_str()),
        (".", "abcdef", long.as_str()),
        ("", "abc", long.as_str()),
    ];
    for (i, (pat, subj, repl)) in cases.iter().enumerate() {
        for bits in [
            0u32,
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
            PCRE2_SUBSTITUTE_LITERAL | PCRE2_SUBSTITUTE_GLOBAL,
        ] {
            diff_sub(
                &format!("emptylong[{i}] b={bits:#x}"),
                pat.as_bytes(),
                0,
                subj.as_bytes(),
                repl.as_bytes(),
                bits,
                &[4096, 300, 64, 8, 1, 0],
            );
        }
    }
}

// ============================================================ random sweep

/// Big randomized sweep over pattern × subject bytes × replacement × option
/// mask × buffer size.
#[test]
fn sub_random_everything() {
    let mut rng = Rng::new(0x5AB5_0002);
    let subjects = byte_subjects();
    let masks = [
        0u32,
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_LITERAL,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
            | PCRE2_SUBSTITUTE_UNSET_EMPTY,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY | PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_OVERFLOW_LENGTH | PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
            | PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
    ];
    for iter in 0..260000 {
        let pat = SUB_PATTERNS[rng.below(SUB_PATTERNS.len())];
        let subj = &subjects[rng.below(subjects.len())];
        let repl = gen_replacement(&mut rng);
        let bits = masks[rng.below(masks.len())];
        let cap = *rng.pick(&[0usize, 1, 2, 5, 16, 64, 256]);
        let so = rng.below(subj.len() + 2);
        diff(
            &format!(
                "rand iter={iter} pat={pat:?} subj={subj:?} repl={repl:?} b={bits:#x} cap={cap} so={so}"
            ),
            |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile_logged(
                        api,
                        pat.as_bytes(),
                        pat.len(),
                        0,
                        std::ptr::null_mut(),
                        &mut l,
                    );
                    if code.is_null() {
                        return l;
                    }
                    call_sub(
                        api,
                        code,
                        subj.as_ptr(),
                        subj.len(),
                        so,
                        bits,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        repl.as_ptr(),
                        repl.len(),
                        cap,
                        &mut l,
                    );
                    (api.code_free)(code);
                }
                l
            },
        );
    }
}

/// Randomized *valid* generated patterns fed through substitute.
#[test]
fn sub_random_generated_patterns() {
    let mut rng = Rng::new(0x5AB5_0003);
    for iter in 0..120000 {
        let pat = PatternGen::gen(&mut rng);
        let utf = rng.bool();
        let subj = gen_subject(&mut rng, utf);
        let repl = gen_replacement(&mut rng);
        let bits = *rng.pick(&[
            0u32,
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
            PCRE2_SUBSTITUTE_GLOBAL
                | PCRE2_SUBSTITUTE_EXTENDED
                | PCRE2_SUBSTITUTE_UNSET_EMPTY
                | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
            PCRE2_SUBSTITUTE_OVERFLOW_LENGTH | PCRE2_SUBSTITUTE_GLOBAL,
        ]);
        diff(
            &format!("gen iter={iter} pat={pat:?} subj={subj:?} repl={repl:?} b={bits:#x}"),
            |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile_logged(
                        api,
                        pat.as_bytes(),
                        pat.len(),
                        0,
                        std::ptr::null_mut(),
                        &mut l,
                    );
                    if code.is_null() {
                        return l;
                    }
                    for cap in [256usize, 8] {
                        call_sub(
                            api,
                            code,
                            subj.as_ptr(),
                            subj.len(),
                            0,
                            bits,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            repl.as_ptr(),
                            repl.len(),
                            cap,
                            &mut l,
                        );
                    }
                    (api.code_free)(code);
                }
                l
            },
        );
    }
}

/// Random *bytes* as the replacement string. Most are malformed, which
/// exercises the whole error surface of the replacement scanner (`$`, `${...}`,
/// `$<...>`, `${n:+…:…}`, `\Q…\E`, `\U\L\u\l\E`, `check_escape`) and requires
/// identical error codes *and* identical `*blength` offsets.
#[test]
fn sub_random_replacement_bytes() {
    let mut rng = Rng::new(0x5AB5_0004);
    // An alphabet biased towards the characters the scanner special-cases.
    let alphabet: &[u8] =
        b"$${}<>:+-*&'`_\\\\0123456789abnmeklsuUELQqxXogGcMARK.,;!?()[]|^ \t\n\xc3\xa9\x80\xff";
    let pats: &[&str] = &[
        "(?<name>a)(b)?",
        "(a)(b)(c)",
        "a",
        "",
        "(?J)(?<n>a)|(?<n>b)",
        "(*MARK:MK)(a)",
        "\\w+",
        "(?<name>\\w)(\\w)?",
    ];
    let subjects: &[&str] = &["", "a", "ab", "abc", "abcabc", "a1b2", "MARK"];
    let masks = [
        0u32,
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_UNSET_EMPTY
            | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_LITERAL,
        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
    ];
    for iter in 0..650000 {
        let n = rng.below(12);
        let repl: Vec<u8> = (0..n).map(|_| *rng.pick(alphabet)).collect();
        let pat = pats[rng.below(pats.len())];
        let subj = subjects[rng.below(subjects.len())];
        let bits = masks[rng.below(masks.len())];
        diff(
            &format!("replbytes iter={iter} pat={pat:?} subj={subj:?} repl={repl:?} b={bits:#x}"),
            |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile_logged(
                        api,
                        pat.as_bytes(),
                        pat.len(),
                        0,
                        std::ptr::null_mut(),
                        &mut l,
                    );
                    if code.is_null() {
                        return l;
                    }
                    call_sub(
                        api,
                        code,
                        subj.as_ptr(),
                        subj.len(),
                        0,
                        bits,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        repl.as_ptr(),
                        repl.len(),
                        128,
                        &mut l,
                    );
                    (api.code_free)(code);
                }
                l
            },
        );
    }
}

/// Randomly generated `${n:+set:unset}` / `${n:-default}` trees, which is the
/// only part of the replacement grammar that recurses (`find_text_end` plus the
/// `ptrstack`). Leaf texts deliberately include `}`/`:`/`\Q`/`\E`/`$` so that
/// the "super lenient" scanning rules in `find_text_end` are stressed.
#[test]
fn sub_random_extended_constructs() {
    const LEAVES: &[&str] = &[
        "", "x", "yy", "$0", "$1", "$2", "$name", "${name}", "$<name>", "\\U", "\\L", "\\u",
        "\\l", "\\E", "\\Q", "\\q", "\\}", "\\:", "\\\\", ":", "}", "{", "$", "$$", "\\g<name>",
        "\\1", "\\2", "\\x{41}", "\\n", "${*MARK}", "\\Qa}b\\E", "\\Qa:b\\E", "${zz}", "${9}",
    ];
    const GROUPS: &[&str] = &["0", "1", "2", "3", "9", "name", "zz", "*MARK"];

    fn build(rng: &mut Rng, depth: u32, out: &mut String) {
        if depth == 0 || rng.below(3) == 0 {
            out.push_str(rng.pick(LEAVES));
            return;
        }
        let g = *rng.pick(GROUPS);
        match rng.below(3) {
            0 => {
                out.push_str("${");
                out.push_str(g);
                out.push_str(":-");
                build(rng, depth - 1, out);
                out.push('}');
            }
            1 => {
                out.push_str("${");
                out.push_str(g);
                out.push_str(":+");
                build(rng, depth - 1, out);
                out.push(':');
                build(rng, depth - 1, out);
                out.push('}');
            }
            _ => {
                build(rng, depth - 1, out);
                out.push_str(rng.pick(LEAVES));
                build(rng, depth - 1, out);
            }
        }
    }

    let mut rng = Rng::new(0x5AB5_0005);
    let pats: &[&str] = &[
        "(?<name>a)(b)?(c)?",
        "(a)(b)(c)",
        "(*MARK:MK)(?<name>a)",
        "(?<name>\\w)(\\w)?",
        "a",
    ];
    for iter in 0..260000 {
        let mut repl = String::new();
        let depth = 1 + rng.below(4) as u32;
        build(&mut rng, depth, &mut repl);
        let pat = pats[rng.below(pats.len())];
        let subj = *rng.pick(&["", "a", "ab", "abc", "a1", "xay"]);
        let bits = PCRE2_SUBSTITUTE_EXTENDED
            | *rng.pick(&[
                0u32,
                PCRE2_SUBSTITUTE_GLOBAL,
                PCRE2_SUBSTITUTE_UNSET_EMPTY,
                PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
            ]);
        diff_sub(
            &format!("extfuzz iter={iter} pat={pat:?} subj={subj:?} repl={repl:?} b={bits:#x}"),
            pat.as_bytes(),
            0,
            subj.as_bytes(),
            repl.as_bytes(),
            bits,
            &[256],
        );
    }
    // Exactly at, and past, PTR_STACK_SIZE / 2 = 10 levels of nesting.
    for levels in 1..=14usize {
        let mut r = String::new();
        for _ in 0..levels {
            r.push_str("${1:+");
        }
        r.push('x');
        for _ in 0..levels {
            r.push_str(":y}");
        }
        diff_sub(
            &format!("ptrstack levels={levels}"),
            b"(a)",
            0,
            b"a",
            r.as_bytes(),
            PCRE2_SUBSTITUTE_EXTENDED,
            &[256],
        );
        let mut r2 = String::new();
        for _ in 0..levels {
            r2.push_str("${1:-");
        }
        r2.push('x');
        for _ in 0..levels {
            r2.push('}');
        }
        diff_sub(
            &format!("ptrstack_minus levels={levels}"),
            b"(a)?b",
            0,
            b"b",
            r2.as_bytes(),
            PCRE2_SUBSTITUTE_EXTENDED,
            &[256],
        );
    }
}

/// `${*MARK}` against every kind of mark-setting verb, including long mark
/// names and marks set on failing branches.
#[test]
fn sub_mark_substitution() {
    let long_mark: String = std::iter::repeat('M').take(200).collect();
    let pats: Vec<String> = vec![
        "(*MARK:m1)a".into(),
        "(*:m2)a".into(),
        "a(*MARK:m3)b".into(),
        "a(*MARK:m1)b|c(*MARK:m2)d".into(),
        "(*MARK:m1)a(*FAIL)|b".into(),
        "(a(*MARK:in))?b".into(),
        "(*MARK:x)(?=a)a".into(),
        "a".into(),
        format!("(*MARK:{long_mark})a"),
        "(*MARK:ünïcødé)a".into(),
        "(*PRUNE:p1)a".into(),
        "(*THEN:t1)a|b".into(),
        "(*SKIP:s1)a".into(),
    ];
    let reps = [
        "${*MARK}",
        "[${*MARK}]",
        "\\U${*MARK}\\E",
        "\\L${*MARK}\\E",
        "\\u${*MARK}",
        "${*mark}",
        "${*Mark}",
        "$*MARK",
        "${*MARK}${*MARK}",
        "$0${*MARK}$0",
    ];
    for (pi, pat) in pats.iter().enumerate() {
        for subj in ["", "a", "ab", "b", "cd", "aa"] {
            for (ri, repl) in reps.iter().enumerate() {
                for bits in [
                    0u32,
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                ] {
                    diff_sub(
                        &format!("mark p[{pi}] s={subj:?} r[{ri}]={repl:?} b={bits:#x}"),
                        pat.as_bytes(),
                        0,
                        subj.as_bytes(),
                        repl.as_bytes(),
                        bits,
                        &[512, 6],
                    );
                }
            }
        }
    }
}

/// `\Q ... \E` literal mode inside a replacement, including the case where the
/// `\Q` is never terminated (the flag persists to the end of the call) and the
/// interaction with case forcing.
#[test]
fn sub_escaped_literal_mode() {
    let reps = [
        "\\Qabc\\E",
        "\\Qa$1b\\E",
        "\\Qa\\Ub\\E",
        "\\Qabc",
        "\\Q",
        "\\Q\\E",
        "\\E\\Qx\\E",
        "\\U\\Qa$1\\E\\E",
        "\\Q$0\\E$0",
        "x\\Qy\\Ez\\Qw",
        "\\Q\\\\E",
        "\\Q}\\E",
        "\\Q:\\E",
        "${1:-\\Qa}b\\E}",
        "${1:+\\Qa:b\\E:c}",
        "\\l\\Qab\\E",
        "\\u\\Qab\\E",
    ];
    for (pi, pat) in ["(a)", "(a)(b)", "a", "\\w"].iter().enumerate() {
        for subj in ["", "a", "ab", "abab"] {
            for (ri, repl) in reps.iter().enumerate() {
                for bits in [
                    PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_EXTENDED
                        | PCRE2_SUBSTITUTE_GLOBAL
                        | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                    0,
                    PCRE2_SUBSTITUTE_LITERAL,
                ] {
                    diff_sub(
                        &format!("qq p[{pi}] s={subj:?} r[{ri}]={repl:?} b={bits:#x}"),
                        pat.as_bytes(),
                        0,
                        subj.as_bytes(),
                        repl.as_bytes(),
                        bits,
                        &[256, 5],
                    );
                }
            }
        }
    }
}

// ============================================================ ERRORS 148

/// Row 148: every single option bit, and wholesale out-of-range option words,
/// crossing the FFI boundary. Undefined bits must yield `BADOPTION` (-34).
#[test]
fn error_option_bits() {
    let mut words: Vec<u32> = Vec::new();
    for b in 0..32u32 {
        words.push(1u32 << b);
    }
    words.extend([
        0u32,
        0xFFFF_FFFF,
        0xFFFF_FFFE,
        0x8000_0000,
        0x7FFF_FFFF,
        0x0008_0000, // first undefined bit above DISABLE_RECURSELOOP_CHECK
        0x0010_0000,
        0x0100_0000,
        0x1000_0000,
        PCRE2_SUBSTITUTE_GLOBAL | 0x0008_0000,
        PCRE2_SUBSTITUTE_EXTENDED | 0x1000_0000,
        PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD,
        PCRE2_DFA_RESTART,
        PCRE2_DFA_SHORTEST,
        PCRE2_DFA_RESTART | PCRE2_DFA_SHORTEST,
    ]);
    for (i, w) in words.iter().enumerate() {
        for (pi, pat) in ["a", "(a)", ""].iter().enumerate() {
            diff(&format!("optbits[{i}]={w:#x} p[{pi}]"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile_logged(
                        api,
                        pat.as_bytes(),
                        pat.len(),
                        0,
                        std::ptr::null_mut(),
                        &mut l,
                    );
                    if code.is_null() {
                        return l;
                    }
                    // No match data (SUBSTITUTE_MATCHED then gives NULL).
                    call_sub(
                        api,
                        code,
                        b"aaa".as_ptr(),
                        3,
                        0,
                        *w,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        b"X".as_ptr(),
                        1,
                        64,
                        &mut l,
                    );
                    // And with a real match data, so SUBSTITUTE_MATCHED gets
                    // past the NULL check.
                    let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                    let rc = (api.do_match)(
                        code,
                        b"aaa".as_ptr(),
                        3,
                        0,
                        0,
                        md,
                        std::ptr::null_mut(),
                    );
                    l.i(rc as i64);
                    call_sub(
                        api,
                        code,
                        b"aaa".as_ptr(),
                        3,
                        0,
                        *w,
                        md,
                        std::ptr::null_mut(),
                        b"X".as_ptr(),
                        1,
                        64,
                        &mut l,
                    );
                    (api.match_data_free)(md);
                    (api.code_free)(code);
                }
                l
            });
        }
    }
}

// ============================================================ ERRORS 149-151

/// Rows 149, 150, 151: the three `PCRE2_ERROR_NULL` conditions.
#[test]
fn error_null_arguments() {
    diff("null_args", |api| {
        let mut l = Log::new();
        unsafe {
            let code = compile_logged(api, b"a", 1, 0, std::ptr::null_mut(), &mut l);
            assert!(!code.is_null());
            let subj = b"aaa";
            // replacement == NULL with rlength != 0  =>  -51
            for rlen in [1usize, 5, PCRE2_ZERO_TERMINATED] {
                call_sub(
                    api,
                    code,
                    subj.as_ptr(),
                    3,
                    0,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null(),
                    rlen,
                    64,
                    &mut l,
                );
            }
            // replacement == NULL with rlength == 0  =>  legal, empty string
            call_sub(
                api,
                code,
                subj.as_ptr(),
                3,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
                0,
                64,
                &mut l,
            );
            // subject == NULL with length != 0  =>  -51
            for len in [1usize, 5, PCRE2_ZERO_TERMINATED] {
                call_sub(
                    api,
                    code,
                    std::ptr::null(),
                    len,
                    0,
                    0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    b"X".as_ptr(),
                    1,
                    64,
                    &mut l,
                );
            }
            // subject == NULL with length == 0  =>  legal, empty string
            call_sub(
                api,
                code,
                std::ptr::null(),
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                b"X".as_ptr(),
                1,
                64,
                &mut l,
            );
            // SUBSTITUTE_MATCHED with match_data == NULL  =>  -51
            for extra in [
                0u32,
                PCRE2_SUBSTITUTE_GLOBAL,
                PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
            ] {
                call_sub(
                    api,
                    code,
                    subj.as_ptr(),
                    3,
                    0,
                    PCRE2_SUBSTITUTE_MATCHED | extra,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    b"X".as_ptr(),
                    1,
                    64,
                    &mut l,
                );
            }
            (api.code_free)(code);
        }
        l
    });
}

// ============================================================ ERRORS 152

/// Row 152: `SUBSTITUTE_MATCHED` with match data produced by
/// `pcre2_dfa_match` must give `PCRE2_ERROR_DFA_UFUNC` (-41).
#[test]
fn error_dfa_ufunc() {
    for (i, (pat, subj)) in [("a", "aaa"), ("(a)", "abc"), ("\\w+", "hi")]
        .iter()
        .enumerate()
    {
        diff(&format!("dfa_ufunc[{i}]"), |api| {
            let mut l = Log::new();
            unsafe {
                let code = compile_logged(
                    api,
                    pat.as_bytes(),
                    pat.len(),
                    0,
                    std::ptr::null_mut(),
                    &mut l,
                );
                if code.is_null() {
                    return l;
                }
                let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                let mut ws = [0i32; 128];
                let rc = (api.dfa_match)(
                    code,
                    subj.as_ptr(),
                    subj.len(),
                    0,
                    0,
                    md,
                    std::ptr::null_mut(),
                    ws.as_mut_ptr(),
                    ws.len(),
                );
                log_md(api, md, rc, &mut l);
                for bits in [0u32, PCRE2_SUBSTITUTE_GLOBAL, PCRE2_SUBSTITUTE_EXTENDED] {
                    call_sub(
                        api,
                        code,
                        subj.as_ptr(),
                        subj.len(),
                        0,
                        PCRE2_SUBSTITUTE_MATCHED | bits,
                        md,
                        std::ptr::null_mut(),
                        b"X".as_ptr(),
                        1,
                        64,
                        &mut l,
                    );
                }
                // Without SUBSTITUTE_MATCHED the DFA provenance is irrelevant.
                call_sub(
                    api,
                    code,
                    subj.as_ptr(),
                    subj.len(),
                    0,
                    0,
                    md,
                    std::ptr::null_mut(),
                    b"X".as_ptr(),
                    1,
                    64,
                    &mut l,
                );
                (api.match_data_free)(md);
                (api.code_free)(code);
            }
            l
        });
    }
}

// ============================================================ ERRORS 153-156

/// Rows 153-156: DIFFSUBSPATTERN / DIFFSUBSSUBJECT / DIFFSUBSOFFSET /
/// DIFFSUBSOPTIONS.
#[test]
fn error_matched_mismatches() {
    diff("diffsubs", |api| {
        let mut l = Log::new();
        unsafe {
            let codea = compile_logged(api, b"(a)", 3, 0, std::ptr::null_mut(), &mut l);
            let codeb = compile_logged(api, b"(b)", 3, 0, std::ptr::null_mut(), &mut l);
            assert!(!codea.is_null() && !codeb.is_null());
            let subj = b"aabaa".to_vec();
            let md = (api.match_data_create_from_pattern)(codea, std::ptr::null_mut());
            let rc = (api.do_match)(
                codea,
                subj.as_ptr(),
                subj.len(),
                1,
                PCRE2_NOTBOL,
                md,
                std::ptr::null_mut(),
            );
            log_md(api, md, rc, &mut l);

            // Baseline: everything agrees.
            call_sub(
                api,
                codea,
                subj.as_ptr(),
                subj.len(),
                1,
                PCRE2_NOTBOL | PCRE2_SUBSTITUTE_MATCHED,
                md,
                std::ptr::null_mut(),
                b"[$1]".as_ptr(),
                4,
                64,
                &mut l,
            );
            // Row 153: different compiled pattern  =>  -71
            call_sub(
                api,
                codeb,
                subj.as_ptr(),
                subj.len(),
                1,
                PCRE2_NOTBOL | PCRE2_SUBSTITUTE_MATCHED,
                md,
                std::ptr::null_mut(),
                b"[$1]".as_ptr(),
                4,
                64,
                &mut l,
            );
            // Row 154: different subject pointer, same content  =>  -72
            let other = subj.clone();
            call_sub(
                api,
                codea,
                other.as_ptr(),
                other.len(),
                1,
                PCRE2_NOTBOL | PCRE2_SUBSTITUTE_MATCHED,
                md,
                std::ptr::null_mut(),
                b"[$1]".as_ptr(),
                4,
                64,
                &mut l,
            );
            // Row 154 variant: same pointer, different length  =>  -72
            for len in [subj.len() - 1, subj.len() - 2, 0] {
                call_sub(
                    api,
                    codea,
                    subj.as_ptr(),
                    len,
                    1,
                    PCRE2_NOTBOL | PCRE2_SUBSTITUTE_MATCHED,
                    md,
                    std::ptr::null_mut(),
                    b"[$1]".as_ptr(),
                    4,
                    64,
                    &mut l,
                );
            }
            // Row 154 variant: NULL subject with length 0 vs a real one  => -72
            call_sub(
                api,
                codea,
                std::ptr::null(),
                0,
                1,
                PCRE2_NOTBOL | PCRE2_SUBSTITUTE_MATCHED,
                md,
                std::ptr::null_mut(),
                b"[$1]".as_ptr(),
                4,
                64,
                &mut l,
            );
            // Row 155: different start_offset  =>  -73
            for so in [0usize, 2, 3, subj.len()] {
                call_sub(
                    api,
                    codea,
                    subj.as_ptr(),
                    subj.len(),
                    so,
                    PCRE2_NOTBOL | PCRE2_SUBSTITUTE_MATCHED,
                    md,
                    std::ptr::null_mut(),
                    b"[$1]".as_ptr(),
                    4,
                    64,
                    &mut l,
                );
            }
            // Row 156: different match options  =>  -74
            for mo in [
                0u32,
                PCRE2_NOTEOL,
                PCRE2_NOTBOL | PCRE2_NOTEOL,
                PCRE2_ANCHORED,
                PCRE2_NOTEMPTY,
            ] {
                call_sub(
                    api,
                    codea,
                    subj.as_ptr(),
                    subj.len(),
                    1,
                    mo | PCRE2_SUBSTITUTE_MATCHED,
                    md,
                    std::ptr::null_mut(),
                    b"[$1]".as_ptr(),
                    4,
                    64,
                    &mut l,
                );
            }
            // NO_UTF_CHECK is explicitly exempt from the options comparison.
            for extra in [
                PCRE2_NO_UTF_CHECK,
                PCRE2_SUBSTITUTE_GLOBAL,
                PCRE2_SUBSTITUTE_EXTENDED,
                PCRE2_SUBSTITUTE_UNSET_EMPTY,
                PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                PCRE2_SUBSTITUTE_LITERAL,
            ] {
                call_sub(
                    api,
                    codea,
                    subj.as_ptr(),
                    subj.len(),
                    1,
                    PCRE2_NOTBOL | PCRE2_SUBSTITUTE_MATCHED | extra,
                    md,
                    std::ptr::null_mut(),
                    b"[$1]".as_ptr(),
                    4,
                    64,
                    &mut l,
                );
            }
            // A match data that ended in NOMATCH, and one whose match failed.
            let md2 = (api.match_data_create_from_pattern)(codea, std::ptr::null_mut());
            let nrc = (api.do_match)(
                codea,
                b"zzz".as_ptr(),
                3,
                0,
                0,
                md2,
                std::ptr::null_mut(),
            );
            l.tag("nomatch").i(nrc as i64);
            call_sub(
                api,
                codea,
                b"zzz".as_ptr(),
                3,
                0,
                PCRE2_SUBSTITUTE_MATCHED,
                md2,
                std::ptr::null_mut(),
                b"[$1]".as_ptr(),
                4,
                64,
                &mut l,
            );
            (api.match_data_free)(md2);
            (api.match_data_free)(md);
            (api.code_free)(codea);
            (api.code_free)(codeb);
        }
        l
    });
}

/// A match data whose last `pcre2_match` failed with a real error must have
/// that error propagated straight back out of `pcre2_substitute`.
#[test]
fn error_matched_propagates_stored_rc() {
    diff("matched_stored_rc", |api| {
        let mut l = Log::new();
        unsafe {
            // `(*NO_START_OPT)` defeats the required-code-unit optimization so
            // that the matcher really does run and really does hit the limits.
            let code = compile_logged(
                api,
                b"(*NO_START_OPT)(a+)+b",
                21,
                0,
                std::ptr::null_mut(),
                &mut l,
            );
            assert!(!code.is_null());
            let subj = b"aaaaaaaaaaaaaaaaaaaac".to_vec();
            // (a) match limit exceeded  =>  -47, both directly and stored.
            let mc = (api.match_context_create)(std::ptr::null_mut());
            l.i((api.set_match_limit)(mc, 1) as i64);
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            let rc = (api.do_match)(code, subj.as_ptr(), subj.len(), 0, 0, md, mc);
            l.tag("mlimit").i(rc as i64);
            // Row 172: an unexpected negative rc from pcre2_match propagates.
            call_sub(
                api,
                code,
                subj.as_ptr(),
                subj.len(),
                0,
                0,
                md,
                mc,
                b"X".as_ptr(),
                1,
                256,
                &mut l,
            );
            // With SUBSTITUTE_MATCHED the stored rc is returned immediately.
            call_sub(
                api,
                code,
                subj.as_ptr(),
                subj.len(),
                0,
                PCRE2_SUBSTITUTE_MATCHED,
                md,
                mc,
                b"X".as_ptr(),
                1,
                256,
                &mut l,
            );
            (api.match_data_free)(md);
            (api.match_context_free)(mc);

            // (b) depth limit exceeded  =>  -53 (fails at depth 1, so fast).
            let mc2 = (api.match_context_create)(std::ptr::null_mut());
            l.i((api.set_depth_limit)(mc2, 1) as i64);
            call_sub(
                api,
                code,
                subj.as_ptr(),
                subj.len(),
                0,
                0,
                std::ptr::null_mut(),
                mc2,
                b"X".as_ptr(),
                1,
                256,
                &mut l,
            );
            (api.match_context_free)(mc2);

            // (c) heap limit 0 with a freshly created (internal) match data:
            // the very first heap-frame allocation fails  =>  -63.
            let mc3 = (api.match_context_create)(std::ptr::null_mut());
            l.i((api.set_heap_limit)(mc3, 0) as i64);
            call_sub(
                api,
                code,
                subj.as_ptr(),
                subj.len(),
                0,
                0,
                std::ptr::null_mut(),
                mc3,
                b"X".as_ptr(),
                1,
                256,
                &mut l,
            );
            (api.match_context_free)(mc3);
            (api.code_free)(code);
        }
        l
    });
}

// ============================================================ ERRORS 157

/// Row 157: `start_offset > length` gives `PCRE2_ERROR_BADOFFSET` (-33).
#[test]
fn error_bad_offset() {
    diff("bad_offset", |api| {
        let mut l = Log::new();
        unsafe {
            for pat in ["a", "(a)", ""] {
                let code = compile_logged(
                    api,
                    pat.as_bytes(),
                    pat.len(),
                    0,
                    std::ptr::null_mut(),
                    &mut l,
                );
                if code.is_null() {
                    continue;
                }
                for len in [0usize, 1, 3] {
                    for so in [0usize, 1, 3, 4, 100, usize::MAX / 2] {
                        call_sub(
                            api,
                            code,
                            b"aaa".as_ptr(),
                            len,
                            so,
                            0,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            b"X".as_ptr(),
                            1,
                            64,
                            &mut l,
                        );
                        // With REPLACEMENT_ONLY the copy-up-to-offset is
                        // skipped, so the offset check is the only guard.
                        call_sub(
                            api,
                            code,
                            b"aaa".as_ptr(),
                            len,
                            so,
                            PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            b"X".as_ptr(),
                            1,
                            64,
                            &mut l,
                        );
                    }
                }
                (api.code_free)(code);
            }
        }
        l
    });
}

// ============================================================ ERRORS 158-160

/// Rows 158, 159, 160: `NOMEMORY` with and without `OVERFLOW_LENGTH`, and with
/// `*outlengthptr == 0`.
#[test]
fn error_nomemory() {
    let cases: &[(&str, &str, &str)] = &[
        ("a", "aaa", "XYZ"),
        ("(a)", "abc", "[$1]"),
        ("x", "abc", "Q"),
        ("", "abc", "-"),
        ("a", "", "Y"),
    ];
    for (i, (pat, subj, repl)) in cases.iter().enumerate() {
        for bits in [
            0u32,
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
            PCRE2_SUBSTITUTE_LITERAL,
        ] {
            diff(&format!("nomem[{i}] b={bits:#x}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile_logged(
                        api,
                        pat.as_bytes(),
                        pat.len(),
                        0,
                        std::ptr::null_mut(),
                        &mut l,
                    );
                    if code.is_null() {
                        return l;
                    }
                    // *blength == 0 always fails: even a no-match needs the
                    // trailing NUL.
                    for cap in [0usize, 1, 2, 3] {
                        for ovf in [0u32, PCRE2_SUBSTITUTE_OVERFLOW_LENGTH] {
                            call_sub(
                                api,
                                code,
                                subj.as_ptr(),
                                subj.len(),
                                0,
                                bits | ovf,
                                std::ptr::null_mut(),
                                std::ptr::null_mut(),
                                repl.as_ptr(),
                                repl.len(),
                                cap,
                                &mut l,
                            );
                        }
                    }
                    (api.code_free)(code);
                }
                l
            });
        }
    }
}

// ============================================================ ERRORS 161-168

/// Rows 161-168: replacement-syntax errors. Each must yield the same code from
/// both libraries, and `*blength` carries the offset of the offending
/// character within the replacement.
#[test]
fn error_replacement_syntax() {
    // (replacement, suboptions) pairs targeting one documented error each.
    let cases: &[(&str, u32)] = &[
        // 161 BADREPLACEMENT (-35)
        ("$", 0),
        ("a$", 0),
        ("$", PCRE2_SUBSTITUTE_EXTENDED),
        ("${", 0),
        ("$<", 0),
        ("$*", 0),
        ("$<x", 0),
        ("$<>", 0),
        ("${}", 0),
        ("${*}", 0),
        ("${*FOO}", 0),
        ("$*FOO", 0),
        ("$-", 0),
        ("$ ", 0),
        // 162 REPMISSINGBRACE (-58)
        ("${1", 0),
        ("${1 ", 0),
        ("${name", 0),
        ("${1:-x", PCRE2_SUBSTITUTE_EXTENDED),
        ("${1:+a:b", PCRE2_SUBSTITUTE_EXTENDED),
        ("${1:-${2", PCRE2_SUBSTITUTE_EXTENDED),
        ("${*MARK", 0),
        // 163 NOSUBSTRING (-49) for a too-large number
        ("$9", 0),
        ("${9}", 0),
        ("$99", 0),
        ("$1000000", 0),
        ("$+", 0),
        // 164 NOSUBSTRING for an unknown name
        ("${zz}", 0),
        ("$zz", 0),
        ("$<zz>", 0),
        ("\\g<zz>", PCRE2_SUBSTITUTE_EXTENDED),
        ("${zz:-d}", PCRE2_SUBSTITUTE_EXTENDED),
        // 167 BADREPESCAPE (-57)
        ("\\q", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\z", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\A", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\Z", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\B", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\R", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\X", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\d", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\w", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\p{L}", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\x{110000}", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\c", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\g", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\g<", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\g<x", PCRE2_SUBSTITUTE_EXTENDED),
        ("\\g<>", PCRE2_SUBSTITUTE_EXTENDED),
        ("${1:-\\q}", PCRE2_SUBSTITUTE_EXTENDED),
        ("${1:-\\}", PCRE2_SUBSTITUTE_EXTENDED),
        // 168 BADSUBSTITUTION (-59)
        ("${1:x}", PCRE2_SUBSTITUTE_EXTENDED),
        ("${1:?abc}", PCRE2_SUBSTITUTE_EXTENDED),
        ("${name:x}", PCRE2_SUBSTITUTE_EXTENDED),
        ("${1::}", PCRE2_SUBSTITUTE_EXTENDED),
        ("${1:1}", PCRE2_SUBSTITUTE_EXTENDED),
        // deep ${} nesting past PTR_STACK_SIZE (20)
        (
            "${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+x:y}:y}:y}:y}:y}:y}:y}:y}:y}:y}:y}:y}",
            PCRE2_SUBSTITUTE_EXTENDED,
        ),
    ];
    for (i, (repl, bits)) in cases.iter().enumerate() {
        for (pi, pat) in ["(?<name>a)(b)?", "a", "(a)(b)(c)"].iter().enumerate() {
            for subj in ["a", "ab", "abc", ""] {
                for extra in [
                    0u32,
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                    PCRE2_SUBSTITUTE_UNSET_EMPTY,
                    PCRE2_SUBSTITUTE_UNKNOWN_UNSET | PCRE2_SUBSTITUTE_UNSET_EMPTY,
                ] {
                    diff_sub(
                        &format!("repsyn[{i}]={repl:?} p[{pi}] s={subj:?} x={extra:#x}"),
                        pat.as_bytes(),
                        0,
                        subj.as_bytes(),
                        repl.as_bytes(),
                        *bits | extra,
                        &[256],
                    );
                }
            }
        }
    }
}

// ============================================================ ERRORS 165, 166

/// Row 165: `PCRE2_ERROR_UNSET` (-55) for an unset group without
/// `UNSET_EMPTY`. Row 166: `PCRE2_ERROR_UNAVAILABLE` (-54) for a group outside
/// a deliberately tiny ovector.
#[test]
fn error_unset_and_unavailable() {
    diff("unset_unavailable", |api| {
        let mut l = Log::new();
        unsafe {
            // Row 165: an optional group that did not participate.
            for (pat, subj, repl) in [
                ("(a)|(b)", "a", "$2"),
                ("(a)|(b)", "b", "$1"),
                ("(a)?(b)", "b", "$1"),
                ("(?<x>a)?(b)", "b", "${x}"),
                ("(a)(?:(b))?", "a", "$2"),
                ("(a)|(b)", "a", "${2:-D}"),
            ] {
                let code = compile_logged(
                    api,
                    pat.as_bytes(),
                    pat.len(),
                    0,
                    std::ptr::null_mut(),
                    &mut l,
                );
                if code.is_null() {
                    continue;
                }
                for bits in [
                    0u32,
                    PCRE2_SUBSTITUTE_UNSET_EMPTY,
                    PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                    PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY,
                ] {
                    call_sub(
                        api,
                        code,
                        subj.as_ptr(),
                        subj.len(),
                        0,
                        bits,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        repl.as_ptr(),
                        repl.len(),
                        64,
                        &mut l,
                    );
                }
                (api.code_free)(code);
            }

            // Row 166: undersized ovector. `pcre2_match` only fills what fits,
            // and `$2`/`$+` then report UNAVAILABLE.
            let code = compile_logged(api, b"(a)(b)(c)", 9, 0, std::ptr::null_mut(), &mut l);
            assert!(!code.is_null());
            for oveccount in [1u32, 2, 3, 4] {
                for repl in ["$1", "$2", "$3", "$+", "${*MARK}", "$0"] {
                    let md = (api.match_data_create)(oveccount, std::ptr::null_mut());
                    l.u((api.get_ovector_count)(md) as u64);
                    for bits in [
                        0u32,
                        PCRE2_SUBSTITUTE_UNSET_EMPTY,
                        PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                    ] {
                        call_sub(
                            api,
                            code,
                            b"abc".as_ptr(),
                            3,
                            0,
                            bits,
                            md,
                            std::ptr::null_mut(),
                            repl.as_ptr(),
                            repl.len(),
                            64,
                            &mut l,
                        );
                    }
                    // Same thing through SUBSTITUTE_MATCHED.
                    let rc = (api.do_match)(
                        code,
                        b"abc".as_ptr(),
                        3,
                        0,
                        0,
                        md,
                        std::ptr::null_mut(),
                    );
                    log_md(api, md, rc, &mut l);
                    call_sub(
                        api,
                        code,
                        b"abc".as_ptr(),
                        3,
                        0,
                        PCRE2_SUBSTITUTE_MATCHED,
                        md,
                        std::ptr::null_mut(),
                        repl.as_ptr(),
                        repl.len(),
                        64,
                        &mut l,
                    );
                    (api.match_data_free)(md);
                }
            }
            (api.code_free)(code);

            // `$+` on a pattern with no capture groups at all.
            let code0 = compile_logged(api, b"a", 1, 0, std::ptr::null_mut(), &mut l);
            for bits in [
                0u32,
                PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                PCRE2_SUBSTITUTE_UNSET_EMPTY,
                PCRE2_SUBSTITUTE_UNKNOWN_UNSET | PCRE2_SUBSTITUTE_UNSET_EMPTY,
            ] {
                call_sub(
                    api,
                    code0,
                    b"a".as_ptr(),
                    1,
                    0,
                    bits,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    b"$+".as_ptr(),
                    2,
                    64,
                    &mut l,
                );
            }
            (api.code_free)(code0);
        }
        l
    });
}

// ============================================================ ERRORS 169, 170

/// Row 169: patterns that repeatedly match empty under `GLOBAL`. Row 170:
/// `TOOMANYREPLACE` needs `INT_MAX` substitutions, which is not reachable in a
/// test, so we instead exercise a *large* number of substitutions and require
/// the two libraries to agree exactly on the count and the output.
#[test]
fn sub_many_and_empty_matches() {
    let pats = ["", "a*", "x?", "\\b", "(?=a)", "$", "^", "\\B", "()", "a??"];
    for (pi, pat) in pats.iter().enumerate() {
        for subj in ["", "a", "aa", "abc", "aaaaaaaa", "a\nb"] {
            for bits in [
                PCRE2_SUBSTITUTE_GLOBAL,
                PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
            ] {
                for repl in ["", "-", "<$0>"] {
                    diff_sub(
                        &format!("empty p[{pi}]={pat:?} s={subj:?} b={bits:#x} r={repl:?}"),
                        pat.as_bytes(),
                        0,
                        subj.as_bytes(),
                        repl.as_bytes(),
                        bits,
                        &[512, 4],
                    );
                }
            }
        }
    }
    // A few thousand substitutions in one call.
    let big: String = std::iter::repeat('a').take(3000).collect();
    for bits in [
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
    ] {
        for pat in ["a", "", "a?"] {
            diff_sub(
                &format!("manysubs pat={pat:?} b={bits:#x}"),
                pat.as_bytes(),
                0,
                big.as_bytes(),
                b"bb",
                bits,
                &[16384, 100],
            );
        }
    }
}

// ============================================================ ERRORS 171

/// Row 171: `PCRE2_ERROR_PARTIALSUBS` (-76) and the `BADOPTION` that guards
/// partial matching without `REPLACEMENT_ONLY`.
#[test]
fn error_partial() {
    let pats = ["abc", "(a)(b)", "\\d{4}", "a+", "(?<n>ab)"];
    let subjs = ["abc", "ab", "a", "123", "abcabc", ""];
    let reps = ["X", "$0", "$'", "$`", "$_", "[$1]", "${n}", "\\U$0\\E"];
    for (pi, pat) in pats.iter().enumerate() {
        for subj in subjs {
            for (ri, repl) in reps.iter().enumerate() {
                for part in [PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
                    for ro in [0u32, PCRE2_SUBSTITUTE_REPLACEMENT_ONLY] {
                        for ext in [0u32, PCRE2_SUBSTITUTE_EXTENDED] {
                            diff_sub(
                                &format!(
                                    "partial p[{pi}] s={subj:?} r[{ri}]={repl:?} p={part:#x} ro={ro:#x} e={ext:#x}"
                                ),
                                pat.as_bytes(),
                                0,
                                subj.as_bytes(),
                                repl.as_bytes(),
                                part | ro | ext,
                                &[256],
                            );
                        }
                    }
                }
            }
        }
    }
}

// ============================================================ ERRORS 173

/// Row 173: a substitute case callout that returns a bad length must produce
/// `PCRE2_ERROR_REPLACECASE` (-69).
#[test]
fn error_replacecase() {
    let reps = [
        "\\U$0\\E",
        "\\L$0\\E",
        "\\u$0",
        "\\l$0",
        "\\u\\L$0\\E",
        "\\l\\U$0\\E",
        "\\Uabc\\E",
        "$0",
    ];
    for (ri, repl) in reps.iter().enumerate() {
        for mode in [1u32, 2, 3, 4] {
            for cap in [256usize, 8, 2, 0] {
                diff(&format!("replcase r[{ri}] m={mode} cap={cap}"), |api| {
                    let mut l = Log::new();
                    unsafe {
                        let code =
                            compile_logged(api, b"(\\w+)", 5, 0, std::ptr::null_mut(), &mut l);
                        assert!(!code.is_null());
                        let mc = (api.match_context_create)(std::ptr::null_mut());
                        let mut st = CaseState {
                            mode,
                            calls: 0,
                            log: Vec::new(),
                        };
                        l.i((api.set_substitute_case_callout)(
                            mc,
                            Some(case_cb),
                            &mut st as *mut CaseState as *mut c_void,
                        ) as i64);
                        for bits in [
                            PCRE2_SUBSTITUTE_EXTENDED,
                            PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                            PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                            PCRE2_SUBSTITUTE_EXTENDED
                                | PCRE2_SUBSTITUTE_GLOBAL
                                | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                        ] {
                            st.calls = 0;
                            st.log.clear();
                            call_sub(
                                api,
                                code,
                                b"hello world".as_ptr(),
                                11,
                                0,
                                bits,
                                std::ptr::null_mut(),
                                mc,
                                repl.as_ptr(),
                                repl.len(),
                                cap,
                                &mut l,
                            );
                            l.u(st.calls as u64).b(&st.log);
                        }
                        (api.match_context_free)(mc);
                        (api.code_free)(code);
                    }
                    l
                });
            }
        }
    }
}

// ============================================================ ERRORS 175

/// Row 175: `PCRE2_ERROR_BADSUBSPATTERN` (-60) when the incoming match has
/// `ovector[1] < ovector[0]` or `ovector[0] < start_offset`. The ovector of a
/// match data block is publicly writable, so we can construct exactly that.
#[test]
fn error_badsubspattern() {
    let pokes: &[(Sz, Sz, usize)] = &[
        (3, 1, 0), // ovector[1] < ovector[0]
        (2, 0, 0),
        (5, 5, 0),
        (0, 1, 2), // ovector[0] < start_offset
        (1, 3, 2),
        (2, 3, 2), // legal
        (2, 2, 2), // legal (empty match at the start offset)
    ];
    for (i, (o0, o1, so)) in pokes.iter().enumerate() {
        for bits in [
            0u32,
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        ] {
            diff(&format!("badsubspat[{i}] b={bits:#x}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile_logged(api, b"a", 1, 0, std::ptr::null_mut(), &mut l);
                    assert!(!code.is_null());
                    let subj = b"aaaaaa";
                    let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                    let rc = (api.do_match)(
                        code,
                        subj.as_ptr(),
                        6,
                        *so,
                        0,
                        md,
                        std::ptr::null_mut(),
                    );
                    l.i(rc as i64);
                    if rc > 0 {
                        let ov = (api.get_ovector_pointer)(md);
                        *ov.add(0) = *o0;
                        *ov.add(1) = *o1;
                        call_sub(
                            api,
                            code,
                            subj.as_ptr(),
                            6,
                            *so,
                            PCRE2_SUBSTITUTE_MATCHED | bits,
                            md,
                            std::ptr::null_mut(),
                            b"[$0]".as_ptr(),
                            4,
                            64,
                            &mut l,
                        );
                    }
                    (api.match_data_free)(md);
                    (api.code_free)(code);
                }
                l
            });
        }
    }
    // `\K` patterns that move the match start around, with and without
    // EXTRA_ALLOW_LOOKAROUND_BSK. Whatever the C library decides, the Rust one
    // must decide identically.
    let kpats = [
        "a\\Kb",
        "ab\\K",
        "\\Ka",
        "(?<=a)\\Kb",
        "(?<=a\\K)b",
        "(?=b\\K)ab",
        "a(?=b\\K)",
        "a\\Kb*",
        "(a\\K)b",
        "\\Kab|b",
    ];
    for (pi, pat) in kpats.iter().enumerate() {
        for extra in [0u32, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK] {
            for so in [0usize, 1, 2] {
                for bits in [0u32, PCRE2_SUBSTITUTE_GLOBAL] {
                    diff(
                        &format!("kpat p[{pi}]={pat:?} x={extra:#x} so={so} b={bits:#x}"),
                        |api| {
                            let mut l = Log::new();
                            unsafe {
                                let cc = (api.compile_context_create)(std::ptr::null_mut());
                                l.i((api.set_compile_extra_options)(cc, extra) as i64);
                                let code =
                                    compile_logged(api, pat.as_bytes(), pat.len(), 0, cc, &mut l);
                                if !code.is_null() {
                                    call_sub(
                                        api,
                                        code,
                                        b"abab".as_ptr(),
                                        4,
                                        so,
                                        bits,
                                        std::ptr::null_mut(),
                                        std::ptr::null_mut(),
                                        b"<$0>".as_ptr(),
                                        4,
                                        64,
                                        &mut l,
                                    );
                                    (api.code_free)(code);
                                }
                                (api.compile_context_free)(cc);
                            }
                            l
                        },
                    );
                }
            }
        }
    }
}

// ============================================================ both callouts

/// Both callouts installed at once, plus a match callout, so that the
/// interaction between the delayed case transformation and the substitute
/// callout's rewind logic is exercised.
#[test]
fn sub_both_callouts_together() {
    let reps = [
        "\\U$0\\E",
        "\\u$1\\L$2\\E",
        "$0",
        "\\L${*MARK}\\E",
        "x\\Uy\\Ez",
        "\\U$1\\E-\\L$1\\E",
    ];
    for (ri, repl) in reps.iter().enumerate() {
        for policy in 0u32..5 {
            for mode in [0u32, 2, 3] {
                for cap in [256usize, 10] {
                    diff(
                        &format!("bothcb r[{ri}] pol={policy} m={mode} cap={cap}"),
                        |api| {
                            let mut l = Log::new();
                            unsafe {
                                let code = compile_logged(
                                    api,
                                    b"(*MARK:Mk)(\\w)(\\w)?",
                                    19,
                                    0,
                                    std::ptr::null_mut(),
                                    &mut l,
                                );
                                assert!(!code.is_null());
                                let mc = (api.match_context_create)(std::ptr::null_mut());
                                let subj = b"aB cD eF";
                                let mut buf = vec![0xAAu8; cap + GUARD];
                                let mut scb = ScbState {
                                    subject: subj.as_ptr(),
                                    subject_len: subj.len(),
                                    buffer: buf.as_ptr(),
                                    policy,
                                    calls: 0,
                                    log: Vec::new(),
                                };
                                let mut cst = CaseState {
                                    mode,
                                    calls: 0,
                                    log: Vec::new(),
                                };
                                l.i((api.set_substitute_callout)(
                                    mc,
                                    Some(scb_cb),
                                    &mut scb as *mut ScbState as *mut c_void,
                                ) as i64);
                                l.i((api.set_substitute_case_callout)(
                                    mc,
                                    Some(case_cb),
                                    &mut cst as *mut CaseState as *mut c_void,
                                ) as i64);
                                let mut blen: Sz = cap;
                                let rc = (api.substitute)(
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    0,
                                    PCRE2_SUBSTITUTE_EXTENDED
                                        | PCRE2_SUBSTITUTE_GLOBAL
                                        | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                                    std::ptr::null_mut(),
                                    mc,
                                    repl.as_ptr(),
                                    repl.len(),
                                    buf.as_mut_ptr(),
                                    &mut blen,
                                );
                                l.tag("both")
                                    .i(rc as i64)
                                    .u(blen as u64)
                                    .b(&buf)
                                    .u(scb.calls as u64)
                                    .b(&scb.log)
                                    .u(cst.calls as u64)
                                    .b(&cst.log);
                                (api.match_context_free)(mc);
                                (api.code_free)(code);
                            }
                            l
                        },
                    );
                }
            }
        }
    }
}

/// Randomized fuzz over the two callouts together with random case-forcing
/// replacements and random buffer sizes. This is the densest part of
/// `pcre2_substitute` (`do_case_copy`'s grow loop, `DELAYEDFORCECASE`'s rewind,
/// and the substitute callout's cancel-and-rewind), so it gets the most
/// randomized iterations.
#[test]
fn sub_random_callout_fuzz() {
    const PIECES: &[&str] = &[
        "", "a", "Bc", "-", "\\U", "\\L", "\\u", "\\l", "\\E", "\\Q", "$0", "$1", "$2", "${n}",
        "${*MARK}", "\\x{41}", "\\x{e9}", "\\n", "é", "😀", "${1:-\\Ud\\E}", "${1:+\\Uy\\E:n}",
        "\\g<n>", "$&", "$`", "$_",
    ];
    let pats: &[&str] = &[
        "(*MARK:MK)(?<n>\\w)(\\w)?",
        "(?<n>a)(B)?",
        "(?i)(?<n>hello)",
        "(?<n>.)",
        "x",
        "(?<n>\\w+)",
    ];
    let subjects: &[&str] = &["", "aB cD", "Hello World", "aÉb", "abc", "😀a", "MARK"];
    let mut rng = Rng::new(0x5AB5_0006);
    for iter in 0..160000 {
        let n = rng.below(5);
        let mut repl = String::new();
        for _ in 0..n {
            repl.push_str(rng.pick(PIECES));
        }
        let pat = pats[rng.below(pats.len())];
        let popts = *rng.pick(&[0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS]);
        let subj = subjects[rng.below(subjects.len())];
        let policy = rng.below(5) as u32;
        let mode = rng.below(5) as u32;
        let use_scb = rng.bool();
        let use_ccb = rng.bool();
        let cap = *rng.pick(&[0usize, 1, 3, 8, 20, 64, 256]);
        let bits = PCRE2_SUBSTITUTE_EXTENDED
            | *rng.pick(&[
                0u32,
                PCRE2_SUBSTITUTE_GLOBAL,
                PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                PCRE2_SUBSTITUTE_GLOBAL
                    | PCRE2_SUBSTITUTE_UNSET_EMPTY
                    | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
            ]);
        diff(
            &format!(
                "cbfuzz iter={iter} pat={pat:?} po={popts:#x} subj={subj:?} repl={repl:?} \
                 pol={policy} mode={mode} scb={use_scb} ccb={use_ccb} cap={cap} b={bits:#x}"
            ),
            |api| {
                let mut l = Log::new();
                unsafe {
                    let code = compile_logged(
                        api,
                        pat.as_bytes(),
                        pat.len(),
                        popts,
                        std::ptr::null_mut(),
                        &mut l,
                    );
                    if code.is_null() {
                        return l;
                    }
                    let mc = (api.match_context_create)(std::ptr::null_mut());
                    let mut buf = vec![0xAAu8; cap + GUARD];
                    let mut scb = ScbState {
                        subject: subj.as_ptr(),
                        subject_len: subj.len(),
                        buffer: buf.as_ptr(),
                        policy,
                        calls: 0,
                        log: Vec::new(),
                    };
                    let mut cst = CaseState {
                        mode,
                        calls: 0,
                        log: Vec::new(),
                    };
                    if use_scb {
                        l.i((api.set_substitute_callout)(
                            mc,
                            Some(scb_cb),
                            &mut scb as *mut ScbState as *mut c_void,
                        ) as i64);
                    }
                    if use_ccb {
                        l.i((api.set_substitute_case_callout)(
                            mc,
                            Some(case_cb),
                            &mut cst as *mut CaseState as *mut c_void,
                        ) as i64);
                    }
                    let mut blen: Sz = cap;
                    let rc = (api.substitute)(
                        code,
                        subj.as_ptr(),
                        subj.len(),
                        0,
                        bits,
                        std::ptr::null_mut(),
                        mc,
                        repl.as_ptr(),
                        repl.len(),
                        buf.as_mut_ptr(),
                        &mut blen,
                    );
                    l.tag("f")
                        .i(rc as i64)
                        .u(blen as u64)
                        .b(&buf)
                        .u(scb.calls as u64)
                        .b(&scb.log)
                        .u(cst.calls as u64)
                        .b(&cst.log);
                    (api.match_context_free)(mc);
                    (api.code_free)(code);
                }
                l
            },
        );
    }
}

/// A match context that carries limits, offset limits and a `NULL` gcontext,
/// combined with substitution — makes sure the context plumbing agrees.
#[test]
fn sub_with_match_context_settings() {
    for (i, (pat, subj, repl)) in [
        ("(a)", "abcabc", "[$1]"),
        ("\\w+", "hello world", "<$0>"),
        ("a", "aaa", "X"),
    ]
    .iter()
    .enumerate()
    {
        for ml in [0u32, 1, 100, 1_000_000] {
            for dl in [0u32, 1, 100, 1_000_000] {
                diff(&format!("mctx[{i}] ml={ml} dl={dl}"), |api| {
                    let mut l = Log::new();
                    unsafe {
                        let code = compile_logged(
                            api,
                            pat.as_bytes(),
                            pat.len(),
                            0,
                            std::ptr::null_mut(),
                            &mut l,
                        );
                        if code.is_null() {
                            return l;
                        }
                        let mc = (api.match_context_create)(std::ptr::null_mut());
                        l.i((api.set_match_limit)(mc, ml) as i64);
                        l.i((api.set_depth_limit)(mc, dl) as i64);
                        for bits in [0u32, PCRE2_SUBSTITUTE_GLOBAL] {
                            call_sub(
                                api,
                                code,
                                subj.as_ptr(),
                                subj.len(),
                                0,
                                bits,
                                std::ptr::null_mut(),
                                mc,
                                repl.as_ptr(),
                                repl.len(),
                                256,
                                &mut l,
                            );
                        }
                        // Copy of the context must behave the same.
                        let mc2 = (api.match_context_copy)(mc);
                        call_sub(
                            api,
                            code,
                            subj.as_ptr(),
                            subj.len(),
                            0,
                            PCRE2_SUBSTITUTE_GLOBAL,
                            std::ptr::null_mut(),
                            mc2,
                            repl.as_ptr(),
                            repl.len(),
                            256,
                            &mut l,
                        );
                        (api.match_context_free)(mc2);
                        (api.match_context_free)(mc);
                        (api.code_free)(code);
                    }
                    l
                });
            }
        }
    }
}
