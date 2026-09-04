//! Phase C — `pcre2_substitute_8` differential tests (C `.so` vs Rust `.so`).
//!
//! Every case runs the SAME call against both libraries, re-using one single
//! output buffer (so that the `input`/`output` pointers seen by a
//! substitute-callout are identical in both runs and can be compared directly),
//! and asserts equality of
//!   * the returned `int`,
//!   * `*outlengthptr` (written on success AND on failure),
//!   * every byte of the output buffer plus an 8-byte overrun guard.
//!
//! The buffer is pre-filled with a 0xAA sentinel before each run, so bytes that
//! the library never touches are *defined* and can safely be compared.

mod common;

use common::diff::*;
use common::*;

use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

// ---------------------------------------------------------------- error codes
// (verified against c_src/include/pcre2.h)
pub const ERR_BADREPLACEMENT: i32 = -35;
pub const ERR_DFA_UFUNC: i32 = -41;
pub const ERR_BADREPESCAPE: i32 = -57;
pub const ERR_REPMISSINGBRACE: i32 = -58;
pub const ERR_BADSUBSTITUTION: i32 = -59;
pub const ERR_BADSUBSPATTERN: i32 = -60;
pub const ERR_TOOMANYREPLACE: i32 = -61;
pub const ERR_REPLACECASE: i32 = -69;
pub const ERR_TOOLARGEREPLACE: i32 = -70;
pub const ERR_DIFFSUBSPATTERN: i32 = -71;
pub const ERR_DIFFSUBSSUBJECT: i32 = -72;
pub const ERR_DIFFSUBSOFFSET: i32 = -73;
pub const ERR_DIFFSUBSOPTIONS: i32 = -74;
pub const ERR_PARTIALSUBS: i32 = -76;

/// `PCRE2_SUBSTITUTE_CASE_*` values handed to a substitute-case callout.
pub const CASE_LOWER: i32 = 1;
pub const CASE_UPPER: i32 = 2;
pub const CASE_TITLE_FIRST: i32 = 3;

const SEED: u64 = 0x5B57_1717_0BED_5EED; // arbitrary fixed seed
const SENTINEL: u8 = 0xAA;
/// Bytes appended past the declared buffer size to detect a buffer overrun.
const GUARD: usize = 8;

// ======================================================================= core

/// Everything observable from one `pcre2_substitute` call.
#[derive(Clone)]
struct SubOut {
    rc: i32,
    outlen: usize,
    /// the whole buffer (declared size + GUARD), sentinel-initialised
    buf: Vec<u8>,
}

/// One `pcre2_substitute` invocation, minus the library.
#[derive(Clone, Debug)]
struct SubArgs<'a> {
    subject: &'a [u8],
    sublen: usize,
    startoffset: usize,
    replacement: &'a [u8],
    rlen: usize,
    options: u32,
    bufsize: usize,
}

impl<'a> SubArgs<'a> {
    fn new(subject: &'a [u8], replacement: &'a [u8], options: u32, bufsize: usize) -> Self {
        SubArgs {
            subject,
            sublen: subject.len(),
            startoffset: 0,
            replacement,
            rlen: replacement.len(),
            options,
            bufsize,
        }
    }
    fn start(mut self, s: usize) -> Self {
        self.startoffset = s;
        self
    }
    fn sublen(mut self, n: usize) -> Self {
        self.sublen = n;
        self
    }
    fn rlen(mut self, n: usize) -> Self {
        self.rlen = n;
        self
    }
}

fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &c in b {
        if c == b'\\' {
            s.push_str("\\\\");
        } else if (0x20..0x7f).contains(&c) {
            s.push(c as char);
        } else {
            s.push_str(&format!("\\x{:02x}", c));
        }
    }
    s
}

/// Human-readable list of the substitute option bits that are set.
fn show_opts(o: u32) -> String {
    let names: [(&str, u32); 12] = [
        ("GLOBAL", PCRE2_SUBSTITUTE_GLOBAL),
        ("EXTENDED", PCRE2_SUBSTITUTE_EXTENDED),
        ("UNSET_EMPTY", PCRE2_SUBSTITUTE_UNSET_EMPTY),
        ("UNKNOWN_UNSET", PCRE2_SUBSTITUTE_UNKNOWN_UNSET),
        ("OVERFLOW_LENGTH", PCRE2_SUBSTITUTE_OVERFLOW_LENGTH),
        ("LITERAL", PCRE2_SUBSTITUTE_LITERAL),
        ("MATCHED", PCRE2_SUBSTITUTE_MATCHED),
        ("REPLACEMENT_ONLY", PCRE2_SUBSTITUTE_REPLACEMENT_ONLY),
        ("NOTBOL", PCRE2_NOTBOL),
        ("NOTEMPTY", PCRE2_NOTEMPTY),
        ("ANCHORED", PCRE2_ANCHORED),
        ("NO_UTF_CHECK", PCRE2_NO_UTF_CHECK),
    ];
    let mut v: Vec<&str> = Vec::new();
    for (n, b) in names {
        if o & b != 0 {
            v.push(n);
        }
    }
    if v.is_empty() {
        format!("{:#x}", o)
    } else {
        format!("{:#x}[{}]", o, v.join("|"))
    }
}

/// Per-library setup performed immediately before the `pcre2_substitute` call.
/// Returns `(mcontext, match_data)`; both are freed by the driver afterwards.
type Prep<'p> = &'p mut dyn FnMut(&'static Api, *mut c_void) -> (*mut c_void, *mut c_void);

/// Run the same call in both libraries and assert full agreement.
///
/// `code_c` / `code_r` are the compiled patterns; `prep` builds the optional
/// match context / match data for the library it is handed.
unsafe fn diff_sub_with(
    cc: &Compiled,
    rr: &Compiled,
    a: &SubArgs,
    label: &str,
    prep: Prep,
) -> (SubOut, SubOut) {
    let total = a.bufsize.saturating_add(GUARD);
    // One shared buffer: identical `output` pointer in both runs.
    let mut buf = vec![SENTINEL; total];
    let mut outs: Vec<SubOut> = Vec::with_capacity(2);

    for (api, code) in [(cc.api, cc.code), (rr.api, rr.code)] {
        for b in buf.iter_mut() {
            *b = SENTINEL;
        }
        let (mcx, md) = prep(api, code);
        if std::env::var_os("SUBTRACE").is_some() {
            eprintln!(
                "TRACE {} {} subject={:?} sublen={} start={} repl={:?} rlen={} opts={} bs={}",
                label,
                api.name,
                show(a.subject),
                a.sublen,
                a.startoffset,
                show(a.replacement),
                a.rlen,
                show_opts(a.options),
                a.bufsize
            );
        }
        let mut outlen = a.bufsize;
        let rc = (api.substitute)(
            code,
            a.subject.as_ptr(),
            a.sublen,
            a.startoffset,
            a.options,
            md,
            mcx,
            a.replacement.as_ptr(),
            a.rlen,
            buf.as_mut_ptr(),
            &mut outlen,
        );
        if !md.is_null() {
            (api.match_data_free)(md);
        }
        if !mcx.is_null() {
            (api.match_context_free)(mcx);
        }
        // Buffer-overrun guard.
        assert!(
            buf[a.bufsize..].iter().all(|&x| x == SENTINEL),
            "{}: {} wrote past the end of the output buffer \
             (bufsize={} guard={:02x?}) subject={:?} repl={:?} opts={}",
            label,
            api.name,
            a.bufsize,
            &buf[a.bufsize..],
            show(a.subject),
            show(a.replacement),
            show_opts(a.options),
        );
        outs.push(SubOut { rc, outlen, buf: buf.clone() });
    }

    let (co, ro) = (outs[0].clone(), outs[1].clone());
    let ctx = || {
        format!(
            "subject={:?} sublen={} start={} repl={:?} rlen={} opts={} bufsize={}",
            show(a.subject),
            a.sublen,
            a.startoffset,
            show(a.replacement),
            a.rlen,
            show_opts(a.options),
            a.bufsize
        )
    };

    assert_eq!(
        co.rc,
        ro.rc,
        "{}: rc differs (C={} Rust={}) {}",
        label,
        co.rc,
        ro.rc,
        ctx()
    );
    assert_eq!(
        co.outlen,
        ro.outlen,
        "{}: *outlengthptr differs (rc={} C={:#x} Rust={:#x}) {}",
        label,
        co.rc,
        co.outlen,
        ro.outlen,
        ctx()
    );
    if co.buf != ro.buf {
        let i = co
            .buf
            .iter()
            .zip(ro.buf.iter())
            .position(|(x, y)| x != y)
            .unwrap();
        panic!(
            "{}: output buffer differs at byte {} (C={:#04x} Rust={:#04x}) rc={} outlen={}\n \
             {}\n C={:?}\n R={:?}",
            label,
            i,
            co.buf[i],
            ro.buf[i],
            co.rc,
            co.outlen,
            ctx(),
            show(&co.buf),
            show(&ro.buf),
        );
    }
    // The error messages for the returned code must agree too.
    if co.rc < 0 {
        let (c, r) = both();
        let mut mb1 = [0u8; 256];
        let mut mb2 = [0u8; 256];
        let n1 = (c.get_error_message)(co.rc, mb1.as_mut_ptr(), 256);
        let n2 = (r.get_error_message)(ro.rc, mb2.as_mut_ptr(), 256);
        assert_eq!(n1, n2, "{}: get_error_message({}) length", label, co.rc);
        assert_eq!(mb1, mb2, "{}: get_error_message({}) text", label, co.rc);
    }
    (co, ro)
}

/// The common case: no match context, no match data.
unsafe fn diff_sub(cc: &Compiled, rr: &Compiled, a: &SubArgs, label: &str) -> (SubOut, SubOut) {
    let mut nothing =
        |_: &'static Api, _: *mut c_void| (std::ptr::null_mut(), std::ptr::null_mut());
    diff_sub_with(cc, rr, a, label, &mut nothing)
}

/// Ask the C library how big the buffer needs to be, via
/// `PCRE2_SUBSTITUTE_OVERFLOW_LENGTH` with a 0-size buffer.
/// Returns `Some(needed_including_nul)` or `None` if the call did not overflow.
unsafe fn probe_needed(cc: &Compiled, a: &SubArgs) -> Option<usize> {
    let mut guard = [SENTINEL; GUARD];
    let mut outlen = 0usize;
    let rc = (cc.api.substitute)(
        cc.code,
        a.subject.as_ptr(),
        a.sublen,
        a.startoffset,
        a.options | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        a.replacement.as_ptr(),
        a.rlen,
        guard.as_mut_ptr(),
        &mut outlen,
    );
    if rc == ERR_NOMEMORY && outlen != PCRE2_UNSET {
        Some(outlen)
    } else {
        None
    }
}

// ============================================================ shared corpora

/// Patterns with capture groups / names / marks that the replacement corpus
/// refers to.  Every one of these compiles cleanly under default options.
const PATTERNS: &[&str] = &[
    "(a)",
    "(a)(b)",
    "(a+)(b*)",
    "(?<name>a+)",
    "(?<name>a)(?<other>b)",
    "(?<name>a)|(?<other>b)",
    "(a)|(b)",
    "(a)(b)?",
    "a",
    "a*",
    "b+",
    ".",
    "\\b",
    "(?=x)",
    "(*MARK:mk)(a)",
    "(?<name>\\w+)\\s+(?<other>\\w+)",
    "()",
    "(?:a)",
    "((a)(b))",
    "x",
];

const SUBJECTS: &[&str] = &[
    "",
    "a",
    "b",
    "ab",
    "aab",
    "abab",
    "xaby",
    "aaa",
    "ABC",
    "hello world",
    "a b",
    "xxx",
    "abc abc abc",
    "\u{00e9}",
    "caf\u{00e9}",
    "\u{20ac}1",
];

/// Replacement strings: every `$`/`\` form the C code accepts, plus degenerate
/// and outright invalid ones (which must produce identical error codes).
const REPS: &[&str] = &[
    // plain
    "", "X", "XY", "hello", "-", "[]",
    // dollar forms
    "$", "$$", "$$$", "$$1", "$0", "$1", "$2", "$9", "$10", "$00", "$01", "$99",
    "${0}", "${1}", "${2}", "${10}", "${}", "${1", "${",
    "$&", "$`", "$'", "$_", "$+", "$+{name}", "$+{nope}",
    "$name", "${name}", "$<name>", "$<name", "$<>", "$<nope>", "$other",
    "$*MARK", "${*MARK}", "$*OTHER", "${*MARK", "$*",
    "a$1b$2c", "$1$1$1", "[$0]", "<$&>", "$`|$'", "$_$_",
    // extended conditional forms
    "${name:+set}",
    "${name:+set:unset}",
    "${name:-def}",
    "${1:+yes:no}",
    "${1:-fallback}",
    "${2:+a$1b:c$1d}",
    "${name:+${1}:${2}}",
    "${1:+${2:+deep:x}:y}",
    "${name:}",
    "${name:*}",
    "${name:+}",
    "${name:-}",
    "${name:+a",
    "${*MARK:+m:n}",
    "${1:+\\Uup\\E:\\Ldown\\E}",
    // extended backslash escapes
    "\\n", "\\r", "\\t", "\\f", "\\a", "\\e", "\\0", "\\b", "\\v",
    "\\x41", "\\x{41}", "\\x{1F600}", "\\x{}", "\\o{101}", "\\o{}", "\\101",
    "\\cA", "\\c", "\\8",
    "\\U", "\\L", "\\u", "\\l", "\\E",
    "\\Uabc", "\\Labc", "\\Uabc\\Edef", "\\uabc", "\\labc",
    "\\u\\Labc", "\\l\\Uabc", "\\U$1\\E$2", "\\LABC\\E", "\\UAbC\\LdEf\\E",
    "\\uA\\lB\\UC\\LD\\E", "\\U\\L\\U\\Labc", "\\u\\u\\uabc", "\\E\\E\\E",
    "\\Qa$1b\\E", "\\Q$1", "\\Q\\E", "\\Qabc", "\\Q\\\\E",
    "\\g<name>", "\\g{1}", "\\g1", "\\g{-1}", "\\g<>", "\\g<nope>", "\\g",
    "\\1", "\\2", "\\9", "\\10",
    "\\$", "\\\\", "\\{", "\\}", "\\:", "\\<", "\\>",
    "\\z", "\\A", "\\d", "\\w", "\\R", "\\X", "\\K", "\\N", "\\G",
    "\\",
    // mixtures
    "\\U$1$2\\E-$0", "$1\\n$2", "pre\\t${name:-none}\\tpost",
];

/// Interesting combinations of the substitute-only option bits.
fn option_sets() -> Vec<u32> {
    use PCRE2_SUBSTITUTE_EXTENDED as EXT;
    use PCRE2_SUBSTITUTE_GLOBAL as G;
    use PCRE2_SUBSTITUTE_LITERAL as LIT;
    use PCRE2_SUBSTITUTE_OVERFLOW_LENGTH as OVF;
    use PCRE2_SUBSTITUTE_REPLACEMENT_ONLY as RO;
    use PCRE2_SUBSTITUTE_UNKNOWN_UNSET as UU;
    use PCRE2_SUBSTITUTE_UNSET_EMPTY as UE;
    let singles = [0, G, EXT, UE, UU, OVF, LIT, RO];
    let mut v: Vec<u32> = singles.to_vec();
    // every single flag combined with GLOBAL
    for s in singles {
        v.push(s | G);
    }
    // pairs and triples that interact
    v.extend_from_slice(&[
        EXT | UE,
        EXT | UU,
        UE | UU,
        EXT | UE | UU,
        EXT | OVF,
        G | EXT | OVF,
        G | EXT | UE,
        G | EXT | UU,
        G | EXT | UE | UU,
        G | EXT | UE | UU | OVF,
        G | EXT | UE | UU | OVF | RO,
        LIT | G | OVF,
        LIT | EXT,
        LIT | RO,
        RO | OVF,
        RO | EXT | G,
        G | UE | OVF | RO,
        // MATCHED with no match_data must fail with PCRE2_ERROR_NULL
        PCRE2_SUBSTITUTE_MATCHED,
        PCRE2_SUBSTITUTE_MATCHED | G | EXT,
    ]);
    v.sort_unstable();
    v.dedup();
    v
}

// ================================================================== 1. flags

#[test]
fn substitute_flag_matrix() {
    let opts = option_sets();
    unsafe {
        for pat in PATTERNS {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for subj in SUBJECTS {
                for rep in ["X", "$0-$1", "\\U$1\\E", "${1:-d}", "$&$&"] {
                    for &o in &opts {
                        for &bs in &[0usize, 1, 4, 64] {
                            let a = SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                            diff_sub(&cc, &rr, &a, &format!("flags pat={:?}", pat));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn substitute_flag_matrix_with_match_options() {
    // Non-substitute option bits are forwarded to pcre2_match().
    let extra = [
        0,
        PCRE2_NOTBOL,
        PCRE2_NOTEOL,
        PCRE2_NOTEMPTY,
        PCRE2_NOTEMPTY_ATSTART,
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_NO_UTF_CHECK,
        PCRE2_NOTBOL | PCRE2_NOTEOL,
    ];
    let subopts = [
        0,
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY | PCRE2_SUBSTITUTE_GLOBAL,
    ];
    unsafe {
        for pat in ["^a", "a$", "a*", "(a)", "\\b", "(?m)^a"] {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for subj in ["", "a", "aa", "a\na", "ba", "ab"] {
                for &e in &extra {
                    for &s in &subopts {
                        for &bs in &[0usize, 2, 64] {
                            let a = SubArgs::new(subj.as_bytes(), b"<$0>", e | s, bs);
                            diff_sub(&cc, &rr, &a, &format!("matchopts pat={:?}", pat));
                        }
                    }
                }
            }
        }
        // PARTIAL_* is only legal together with REPLACEMENT_ONLY.
        for pat in ["abcd", "a(b)c"] {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            for subj in ["", "ab", "abc", "abcd", "abcdx"] {
                for &p in &[PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
                    for &ro in &[0, PCRE2_SUBSTITUTE_REPLACEMENT_ONLY] {
                        for rep in ["Z", "$0", "$`", "$'", "$_"] {
                            let a = SubArgs::new(subj.as_bytes(), rep.as_bytes(), p | ro, 32);
                            diff_sub(&cc, &rr, &a, &format!("partial pat={:?}", pat));
                        }
                    }
                }
            }
        }
    }
}

// ==================================================== 2. replacement syntax

#[test]
fn substitute_replacement_syntax() {
    let opts = [
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY,
        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_UNSET_EMPTY
            | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_LITERAL,
        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_UNSET_EMPTY
            | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY | PCRE2_SUBSTITUTE_EXTENDED,
    ];
    // Patterns chosen so that group 1/2 and the names `name`/`other` exist
    // (or deliberately don't).
    let pats = [
        "(?<name>a+)(?<other>b*)",
        "(?<name>a)|(?<other>b)",
        "(a)(b)?",
        "(a)",
        "a",
        "(*MARK:mk)(a)",
    ];
    unsafe {
        for pat in pats {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for subj in ["", "a", "ab", "b", "xaby", "aabb", "abcabc", "a b"] {
                for rep in REPS {
                    for &o in &opts {
                        for &bs in &[64usize, 256] {
                            let a = SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                            diff_sub(&cc, &rr, &a, &format!("repsyntax pat={:?}", pat));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn substitute_replacement_syntax_tight_buffers() {
    // Same replacement corpus, but with buffer sizes that force the overflow
    // and NOROOM paths through every replacement item.
    let opts = [
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
    ];
    unsafe {
        for pat in [
            "(?<name>a+)(?<other>b*)",
            "(a)(b)?",
            "(?<name>a)|(?<other>b)",
            "(*MARK:mk)(a)",
            "a",
        ] {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for subj in ["aab", "ab", "xxaabbxx", "", "b"] {
                for rep in REPS {
                    for &o in &opts {
                        for &bs in &[0usize, 1, 2, 3, 4, 5, 7, 9, 13] {
                            let a = SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                            diff_sub(&cc, &rr, &a, &format!("tight pat={:?}", pat));
                        }
                    }
                }
            }
        }
    }
}

// ================================================= 3. subject / repl shapes

#[test]
fn substitute_shapes_and_offsets() {
    let long_rep = "Y".repeat(40);
    let reps: Vec<&[u8]> = vec![
        b"",
        b"Z",
        b"$0$0$0$0",
        long_rep.as_bytes(),
        b"$`<>$'",
        b"$_",
    ];
    let opts = [
        0,
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
    ];
    // Includes the zero-length-match patterns which drive the empty-match
    // advance logic of pcre2_next_match().
    let pats = [
        "a*", "a*?", "(?=x)", "\\b", "\\B", "b*", "", "(?:)", "x?", "$", "^",
        "a", "ab", "(a|ab)", "\\G",
    ];
    unsafe {
        for pat in pats {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for subj in ["", "a", "x", "ab", "aXbXc", "aaa", "abcabc", "  a  "] {
                let sb = subj.as_bytes();
                let starts: Vec<usize> = vec![0, sb.len() / 2, sb.len(), sb.len() + 1];
                for &st in &starts {
                    for rep in &reps {
                        for &o in &opts {
                            for &bs in &[0usize, 4, 128] {
                                let a = SubArgs::new(sb, rep, o, bs).start(st);
                                diff_sub(&cc, &rr, &a, &format!("shape pat={:?}", pat));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn substitute_zero_terminated_lengths() {
    unsafe {
        for pat in ["a", "a*", "(a)(b)", "\\b"] {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            for base in ["", "a", "ab", "abab", "xay"] {
                // NUL-terminated copies so PCRE2_ZERO_TERMINATED is legal.
                let mut sz = base.as_bytes().to_vec();
                sz.push(0);
                for rep in ["", "Z", "$0!", "\\U$0"] {
                    let mut rz = rep.as_bytes().to_vec();
                    rz.push(0);
                    for &o in &[
                        0,
                        PCRE2_SUBSTITUTE_GLOBAL,
                        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
                        PCRE2_SUBSTITUTE_GLOBAL
                            | PCRE2_SUBSTITUTE_EXTENDED
                            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                    ] {
                        for &bs in &[0usize, 3, 64] {
                            // subject zero-terminated
                            let a = SubArgs::new(&sz, rep.as_bytes(), o, bs)
                                .sublen(PCRE2_ZERO_TERMINATED);
                            diff_sub(&cc, &rr, &a, "zt-subject");
                            // replacement zero-terminated
                            let a = SubArgs::new(base.as_bytes(), &rz, o, bs)
                                .rlen(PCRE2_ZERO_TERMINATED);
                            diff_sub(&cc, &rr, &a, "zt-replacement");
                            // both
                            let a = SubArgs::new(&sz, &rz, o, bs)
                                .sublen(PCRE2_ZERO_TERMINATED)
                                .rlen(PCRE2_ZERO_TERMINATED);
                            diff_sub(&cc, &rr, &a, "zt-both");
                        }
                    }
                }
            }
        }
    }
}

// ================================================== 4. output buffer sizing

#[test]
fn substitute_buffer_sizing_and_two_pass() {
    let cases: &[(&str, &str, &str)] = &[
        ("(a)", "aaa", "[$1]"),
        ("a", "banana", "XY"),
        ("a*", "bab", "<$0>"),
        ("(?<n>\\w+)", "one two three", "${n}!"),
        ("x", "no match here", "Q"),
        ("(a)(b)", "abab", "$2$1$2$1"),
        ("", "abc", "-"),
        ("(a)", "aaaaaaaaaaaaaaaaaaaa", "$1$1$1"),
    ];
    let bases = [
        0u32,
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY | PCRE2_SUBSTITUTE_GLOBAL,
    ];
    unsafe {
        for &(pat, subj, rep) in cases {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for &base in &bases {
                let probe = SubArgs::new(subj.as_bytes(), rep.as_bytes(), base, 0);
                let needed = probe_needed(&cc, &probe);

                let mut sizes: Vec<usize> = vec![0, 1, 2, 4096];
                if let Some(n) = needed {
                    // n includes the trailing NUL.
                    sizes.push(n);
                    if n > 0 {
                        sizes.push(n - 1);
                    }
                    if n > 1 {
                        sizes.push(n - 2);
                    }
                    sizes.push(n + 1);
                }
                sizes.sort_unstable();
                sizes.dedup();

                for &bs in &sizes {
                    for &ovf in &[0, PCRE2_SUBSTITUTE_OVERFLOW_LENGTH] {
                        let a =
                            SubArgs::new(subj.as_bytes(), rep.as_bytes(), base | ovf, bs);
                        let (co, _) = diff_sub(
                            &cc,
                            &rr,
                            &a,
                            &format!("sizing pat={:?} needed={:?}", pat, needed),
                        );
                        // Cross-check the reported length against the probe.
                        if let Some(n) = needed {
                            if bs >= n {
                                assert!(
                                    co.rc >= 0,
                                    "sizing pat={:?} bufsize {} >= needed {} but rc={}",
                                    pat,
                                    bs,
                                    n,
                                    co.rc
                                );
                                assert_eq!(
                                    co.outlen,
                                    n - 1,
                                    "sizing pat={:?}: success length should be needed-1",
                                    pat
                                );
                            } else {
                                assert_eq!(
                                    co.rc, ERR_NOMEMORY,
                                    "sizing pat={:?} bufsize {} < needed {}",
                                    pat, bs, n
                                );
                                if ovf != 0 {
                                    assert_eq!(
                                        co.outlen, n,
                                        "sizing pat={:?}: OVERFLOW_LENGTH must report needed",
                                        pat
                                    );
                                }
                            }
                        }
                    }
                }

                // The documented two-pass workflow: learn the length with a
                // tiny buffer, then call again with exactly that size.
                let mut small = SubArgs::new(subj.as_bytes(), rep.as_bytes(), base, 1);
                small.options |= PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
                let (c1, _) = diff_sub(&cc, &rr, &small, "twopass-1");
                if c1.rc == ERR_NOMEMORY {
                    let big = SubArgs::new(
                        subj.as_bytes(),
                        rep.as_bytes(),
                        base | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                        c1.outlen,
                    );
                    let (c2, _) = diff_sub(&cc, &rr, &big, "twopass-2");
                    assert!(
                        c2.rc >= 0,
                        "twopass pat={:?}: second pass with reported size {} failed rc={}",
                        pat,
                        c1.outlen,
                        c2.rc
                    );
                }
            }
        }
    }
}

// ============================================================== 5./6. UTF

#[test]
fn substitute_utf() {
    let cfgs = [
        CompileCfg::new(PCRE2_UTF),
        CompileCfg::new(PCRE2_UTF | PCRE2_UCP),
        CompileCfg::new(PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS),
    ];
    let pats = [
        "(\\w+)",
        "(.)",
        "(\\X)",
        "(\u{00e9})",
        "([\u{00e0}-\u{00ff}]+)",
        "(?<n>\\p{L}+)",
        "(\\p{Lu})",
        ".*",
        "(a)",
        "\\b",
    ];
    let subjs = [
        "",
        "a",
        "caf\u{00e9}",
        "\u{00e9}\u{00e8}\u{00ea}",
        "\u{20ac}100",
        "\u{1F600}\u{1F601}",
        "\u{0130}\u{0131}",
        "\u{03B1}\u{03B2}\u{03B3}",
        "\u{4E00}\u{4E8C}\u{4E09}",
        "Stra\u{00df}e",
        "MIXED case \u{00c9}\u{00e9}",
    ];
    let reps = [
        "",
        "\u{00e9}",
        "$0",
        "$1",
        "<$1>",
        "\\U$1\\E",
        "\\L$1\\E",
        "\\u$1",
        "\\l$1",
        "\\u\\L$1\\E",
        "\\l\\U$1\\E",
        "\\x{1F600}",
        "\\x{e9}",
        "\\x{10FFFF}",
        "\\x{110000}",
        "${n:-\u{00e9}}",
        "\u{20ac}$1\u{20ac}",
        "\\U\u{00e9}\u{00e8}\\E",
        "\\L\u{00c9}\u{00c8}\\E",
        "\\u\u{00e9}x",
    ];
    let opts = [
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_UNSET_EMPTY
            | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
    ];
    unsafe {
        for cfg in &cfgs {
            for pat in pats {
                let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), cfg, pat);
                if cc.code.is_null() {
                    continue;
                }
                for subj in subjs {
                    for rep in reps {
                        for &o in &opts {
                            for &bs in &[0usize, 3, 7, 256] {
                                let a =
                                    SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                                diff_sub(
                                    &cc,
                                    &rr,
                                    &a,
                                    &format!("utf pat={:?} cfg={:#x}", pat, cfg.options),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn substitute_invalid_utf() {
    // Invalid UTF-8 in the subject and/or the replacement, with and without
    // PCRE2_NO_UTF_CHECK.
    let bad: &[&[u8]] = &[
        b"\xff",
        b"\x80",
        b"\xc3",
        b"\xc3\x28",
        b"\xe0\x80\x80",
        b"\xed\xa0\x80",
        b"\xf4\x90\x80\x80",
        b"a\xffb",
        b"caf\xe9",
        b"\xf0\x9f\x98",
    ];
    let good: &[&[u8]] = &[
        b"",
        b"a",
        b"ab",
        "caf\u{00e9}".as_bytes(),
        "\u{20ac}".as_bytes(),
        "\u{1F600}".as_bytes(),
    ];
    let opts = [
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
    ];
    let reps_valid: &[&[u8]] = &[b"", b"X", b"$0", b"<$1>", b"\\U$0\\E", b"\\x{e9}"];
    unsafe {
        for pat in ["(.)", "(a)", ".*", "(\\w)", "(\\X)"] {
            for &ucp in &[0, PCRE2_UCP] {
                let cfg = CompileCfg::new(PCRE2_UTF | ucp);
                let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &cfg, pat);
                if cc.code.is_null() {
                    continue;
                }
                // (a) Invalid UTF-8 WITHOUT PCRE2_NO_UTF_CHECK: both libraries
                // must reject it with exactly the same PCRE2_ERROR_UTF8_ERR*.
                //
                // NOTE: PCRE2 documents that passing invalid UTF together with
                // PCRE2_NO_UTF_CHECK has undefined behaviour (the C library
                // does in fact segfault on e.g. subject "\xff"), so that
                // combination is deliberately NOT exercised here.
                for subj in bad.iter().chain(good.iter()) {
                    for rep in bad.iter().chain(good.iter()) {
                        for &o in &opts {
                            for &bs in &[0usize, 4, 64] {
                                let a = SubArgs::new(subj, rep, o, bs);
                                diff_sub(&cc, &rr, &a, &format!("badutf pat={:?}", pat));
                            }
                        }
                    }
                }
                // (b) VALID UTF-8 with PCRE2_NO_UTF_CHECK: skips both the
                // replacement check and the subject check.
                for subj in good.iter() {
                    for rep in reps_valid.iter() {
                        for &o in &opts {
                            for &bs in &[0usize, 4, 64] {
                                let a =
                                    SubArgs::new(subj, rep, o | PCRE2_NO_UTF_CHECK, bs);
                                diff_sub(&cc, &rr, &a, &format!("noutfchk pat={:?}", pat));
                            }
                        }
                    }
                }
                // (c) PCRE2_MATCH_INVALID_UTF makes matching an invalid subject
                // well-defined; the replacement itself stays valid UTF-8.
                let cfg2 = CompileCfg::new(PCRE2_UTF | ucp | PCRE2_MATCH_INVALID_UTF);
                let (c2, r2) = compile_both(pat.as_bytes(), pat.len(), &cfg2, pat);
                if c2.code.is_null() {
                    continue;
                }
                for subj in bad.iter().chain(good.iter()) {
                    for rep in reps_valid.iter() {
                        for &o in &opts {
                            for &bs in &[0usize, 4, 64] {
                                let a = SubArgs::new(subj, rep, o, bs);
                                diff_sub(
                                    &c2,
                                    &r2,
                                    &a,
                                    &format!("matchinvalidutf pat={:?}", pat),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================= 6. named / duplicate / unset

#[test]
fn substitute_named_and_unset_groups() {
    let pats = [
        // unset group 2
        "(a)(b)?",
        "(a)|(b)",
        // named, one branch unset
        "(?<n>a)|(?<m>b)",
        // duplicate names (needs DUPNAMES)
        "(?<n>a)|(?<n>b)",
        "(?<n>a)(?<n>b)?",
        "(?<n>x)|(?<n>y)|(?<n>z)",
        // deeply nested names
        "((?<n>a)(?<m>b)?)",
        // no groups at all
        "a",
        "",
    ];
    let reps = [
        "$1", "$2", "$3", "${1}", "${2}", "${3}", "$n", "$m", "${n}", "${m}",
        "$<n>", "$<m>", "$nope", "${nope}", "$+", "$&",
        "${n:-D}", "${m:-D}", "${nope:-D}",
        "${n:+S:U}", "${m:+S:U}", "${nope:+S:U}",
        "\\g<n>", "\\g<m>", "\\g<nope>", "\\g{2}", "\\2", "\\3",
        "[$1|$2|$n|$m]",
    ];
    let opts = [
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_UNSET_EMPTY,
        PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY,
        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_UNSET_EMPTY
            | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_UNSET_EMPTY
            | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
    ];
    unsafe {
        for &dup in &[0u32, PCRE2_DUPNAMES] {
            for pat in pats {
                let cfg = CompileCfg::new(dup);
                let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &cfg, pat);
                if cc.code.is_null() {
                    continue;
                }
                for subj in ["", "a", "b", "ab", "x", "y", "z", "ba"] {
                    for rep in reps {
                        for &o in &opts {
                            for &bs in &[0usize, 2, 64] {
                                let a =
                                    SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                                diff_sub(
                                    &cc,
                                    &rr,
                                    &a,
                                    &format!("named pat={:?} dup={:#x}", pat, dup),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn substitute_small_ovector() {
    // A match_data whose ovector is too small for all the groups exercises the
    // PCRE2_ERROR_UNAVAILABLE / "rc == 0" paths.
    let pats = ["(a)(b)(c)(d)", "(?<n>a)(?<m>b)(?<o>c)"];
    let reps = ["$1$2$3$4", "$+", "${m}", "$&", "$4", "\\4"];
    unsafe {
        for pat in pats {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for ovecsize in [1u32, 2, 3, 4, 5, 8] {
                for subj in ["abcd", "xabcdy", ""] {
                    for rep in reps {
                        for &o in &[
                            0,
                            PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                            PCRE2_SUBSTITUTE_UNSET_EMPTY
                                | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
                        ] {
                            let mut mk = |api: &'static Api, _code: *mut c_void| {
                                let md = (api.match_data_create)(
                                    ovecsize,
                                    std::ptr::null_mut(),
                                );
                                assert!(!md.is_null());
                                (std::ptr::null_mut(), md)
                            };
                            let a = SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, 64);
                            diff_sub_with(
                                &cc,
                                &rr,
                                &a,
                                &format!("ovec{} pat={:?}", ovecsize, pat),
                                &mut mk,
                            );
                        }
                    }
                }
            }
        }
    }
}

// ============================================== 7. PCRE2_SUBSTITUTE_MATCHED

#[test]
fn substitute_matched_option() {
    let cases: &[(&str, &str)] = &[
        ("(a)", "aaa"),
        ("(a)(b)", "abab"),
        ("a*", "bab"),
        ("x", "abc"),
        ("(?<n>\\w+)", "one two"),
        ("(a)", ""),
    ];
    let reps = ["Z", "[$0]", "$1-$1", "\\U$0\\E"];
    unsafe {
        for &(pat, subj) in cases {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            let sb = subj.as_bytes();
            for rep in reps {
                for &o in &[
                    PCRE2_SUBSTITUTE_MATCHED,
                    PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_MATCHED
                        | PCRE2_SUBSTITUTE_GLOBAL
                        | PCRE2_SUBSTITUTE_EXTENDED
                        | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                    PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                    // MATCHED without match_data at all -> PCRE2_ERROR_NULL
                    PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_EXTENDED,
                ] {
                    for &bs in &[0usize, 3, 64] {
                        // (a) match_data holding a real prior match
                        let mut prematched = |api: &'static Api, code: *mut c_void| {
                            let md = (api.match_data_create_from_pattern)(
                                code,
                                std::ptr::null_mut(),
                            );
                            assert!(!md.is_null());
                            (api.do_match)(
                                code,
                                sb.as_ptr(),
                                sb.len(),
                                0,
                                0,
                                md,
                                std::ptr::null_mut(),
                            );
                            (std::ptr::null_mut(), md)
                        };
                        let a = SubArgs::new(sb, rep.as_bytes(), o, bs);
                        diff_sub_with(
                            &cc,
                            &rr,
                            &a,
                            &format!("matched-prior pat={:?}", pat),
                            &mut prematched,
                        );

                        // (b) A *virgin* match_data (created but never used for
                        // a match) is deliberately NOT tested: pcre2_match_data
                        // _create() leaves `rc`, `code`, `subject`,
                        // `subject_length`, `start_offset` and `options`
                        // uninitialised (see c_src/src/pcre2_match_data.c:53),
                        // so PCRE2_SUBSTITUTE_MATCHED on such a block reads
                        // indeterminate memory and cannot be compared.

                        // (c) match_data that recorded a NO-match
                        let mut nomatched = |api: &'static Api, code: *mut c_void| {
                            let md = (api.match_data_create_from_pattern)(
                                code,
                                std::ptr::null_mut(),
                            );
                            assert!(!md.is_null());
                            let never = b"\x01\x02\x03";
                            (api.do_match)(
                                code,
                                never.as_ptr(),
                                never.len(),
                                0,
                                0,
                                md,
                                std::ptr::null_mut(),
                            );
                            (std::ptr::null_mut(), md)
                        };
                        let a = SubArgs::new(sb, rep.as_bytes(), o, bs);
                        diff_sub_with(
                            &cc,
                            &rr,
                            &a,
                            &format!("matched-nomatch pat={:?}", pat),
                            &mut nomatched,
                        );

                        // (d) match_data from the DFA matcher -> DFA_UFUNC
                        let mut dfa = |api: &'static Api, code: *mut c_void| {
                            let md = (api.match_data_create_from_pattern)(
                                code,
                                std::ptr::null_mut(),
                            );
                            assert!(!md.is_null());
                            let mut ws = [0i32; 256];
                            (api.dfa_match)(
                                code,
                                sb.as_ptr(),
                                sb.len(),
                                0,
                                0,
                                md,
                                std::ptr::null_mut(),
                                ws.as_mut_ptr(),
                                ws.len(),
                            );
                            (std::ptr::null_mut(), md)
                        };
                        let a = SubArgs::new(sb, rep.as_bytes(), o, bs);
                        diff_sub_with(
                            &cc,
                            &rr,
                            &a,
                            &format!("matched-dfa pat={:?}", pat),
                            &mut dfa,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn substitute_matched_mismatch_errors() {
    let _guard = global_lock();
    // A pre-existing match whose subject / start offset / options / pattern do
    // not agree with the substitute call: PCRE2_ERROR_DIFFSUBS*.
    unsafe {
        let pat = "(a)";
        let other_pat = "(b)";
        let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
        let (oc, or) =
            compile_both(other_pat.as_bytes(), other_pat.len(), &CompileCfg::new(0), other_pat);
        let subj = b"xaax";
        let other_subj = b"yaay";

        // wrong subject contents (different pointer)
        let mut wrong_subject = |api: &'static Api, code: *mut c_void| {
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            (api.do_match)(
                code,
                other_subj.as_ptr(),
                other_subj.len(),
                0,
                0,
                md,
                std::ptr::null_mut(),
            );
            (std::ptr::null_mut(), md)
        };
        let a = SubArgs::new(subj, b"Z", PCRE2_SUBSTITUTE_MATCHED, 64);
        let (co, _) = diff_sub_with(&cc, &rr, &a, "diffsubj", &mut wrong_subject);
        assert_eq!(co.rc, ERR_DIFFSUBSSUBJECT, "expected DIFFSUBSSUBJECT");

        // wrong start offset
        let mut wrong_offset = |api: &'static Api, code: *mut c_void| {
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            (api.do_match)(
                code,
                subj.as_ptr(),
                subj.len(),
                1,
                0,
                md,
                std::ptr::null_mut(),
            );
            (std::ptr::null_mut(), md)
        };
        let a = SubArgs::new(subj, b"Z", PCRE2_SUBSTITUTE_MATCHED, 64);
        let (co, _) = diff_sub_with(&cc, &rr, &a, "diffoffset", &mut wrong_offset);
        assert_eq!(co.rc, ERR_DIFFSUBSOFFSET, "expected DIFFSUBSOFFSET");

        // wrong match options
        let mut wrong_options = |api: &'static Api, code: *mut c_void| {
            let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
            (api.do_match)(
                code,
                subj.as_ptr(),
                subj.len(),
                0,
                PCRE2_NOTBOL,
                md,
                std::ptr::null_mut(),
            );
            (std::ptr::null_mut(), md)
        };
        let a = SubArgs::new(subj, b"Z", PCRE2_SUBSTITUTE_MATCHED, 64);
        let (co, _) = diff_sub_with(&cc, &rr, &a, "diffoptions", &mut wrong_options);
        assert_eq!(co.rc, ERR_DIFFSUBSOPTIONS, "expected DIFFSUBSOPTIONS");

        // match performed with the *other* pattern
        if !oc.code.is_null() {
            let ocode_c = oc.code;
            let ocode_r = or.code;
            let mut wrong_pattern = |api: &'static Api, _code: *mut c_void| {
                let ocode = if api.name == "C" { ocode_c } else { ocode_r };
                let md =
                    (api.match_data_create_from_pattern)(ocode, std::ptr::null_mut());
                (api.do_match)(
                    ocode,
                    subj.as_ptr(),
                    subj.len(),
                    0,
                    0,
                    md,
                    std::ptr::null_mut(),
                );
                (std::ptr::null_mut(), md)
            };
            let a = SubArgs::new(subj, b"Z", PCRE2_SUBSTITUTE_MATCHED, 64);
            let (co, _) = diff_sub_with(&cc, &rr, &a, "diffpattern", &mut wrong_pattern);
            assert_eq!(co.rc, ERR_DIFFSUBSPATTERN, "expected DIFFSUBSPATTERN");
        }

        // MATCHED with a NULL match_data -> PCRE2_ERROR_NULL
        let a = SubArgs::new(subj, b"Z", PCRE2_SUBSTITUTE_MATCHED, 64);
        let (co, _) = diff_sub(&cc, &rr, &a, "matched-null-md");
        assert_eq!(co.rc, ERR_NULL, "expected PCRE2_ERROR_NULL");
    }
}

// ================================================== 8. substitute callout

/// Mirror of `pcre2_substitute_callout_block` from c_src/include/pcre2.h:616.
#[repr(C)]
struct SubstituteCalloutBlock {
    version: u32,
    input: *const u8,
    output: *const u8,
    output_offsets: [usize; 2],
    ovector: *mut usize,
    oveccount: u32,
    subscount: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CalloutRec {
    version: u32,
    /// raw pointers — comparable because both runs share one subject and one
    /// output buffer
    input: usize,
    output: usize,
    off0: usize,
    off1: usize,
    oveccount: u32,
    subscount: u32,
    ovector: Vec<usize>,
    /// the newly written output fragment
    fragment: Vec<u8>,
}

static REC_C: Mutex<Vec<CalloutRec>> = Mutex::new(Vec::new());
static REC_R: Mutex<Vec<CalloutRec>> = Mutex::new(Vec::new());
/// what the substitute callout returns
static CALLOUT_RET: AtomicI32 = AtomicI32::new(0);
/// return this on the Nth (1-based) callout only; 0 = always
static CALLOUT_RET_ON: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn record_callout(
    scb: *mut SubstituteCalloutBlock,
    data: *mut c_void,
) -> i32 {
    let b = &*scb;
    let off0 = b.output_offsets[0];
    let off1 = b.output_offsets[1];
    let fragment = if off1 >= off0 && !b.output.is_null() {
        std::slice::from_raw_parts(b.output.add(off0), off1 - off0).to_vec()
    } else {
        Vec::new()
    };
    let ovector = if b.ovector.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(b.ovector, 2 * b.oveccount as usize).to_vec()
    };
    let rec = CalloutRec {
        version: b.version,
        input: b.input as usize,
        output: b.output as usize,
        off0,
        off1,
        oveccount: b.oveccount,
        subscount: b.subscount,
        ovector,
        fragment,
    };
    let which = data as usize;
    let mut g = if which == 0 {
        REC_C.lock().unwrap()
    } else {
        REC_R.lock().unwrap()
    };
    g.push(rec);
    let n = g.len();
    drop(g);

    let on = CALLOUT_RET_ON.load(Ordering::SeqCst);
    if on == 0 || on == n {
        CALLOUT_RET.load(Ordering::SeqCst)
    } else {
        0
    }
}

fn clear_recs() {
    REC_C.lock().unwrap().clear();
    REC_R.lock().unwrap().clear();
}

/// Build a `prep` closure that installs `record_callout` in the given library.
fn callout_prep() -> impl FnMut(&'static Api, *mut c_void) -> (*mut c_void, *mut c_void) {
    move |api: &'static Api, _code: *mut c_void| unsafe {
        let mcx = (api.match_context_create)(std::ptr::null_mut());
        assert!(!mcx.is_null());
        let which: usize = if api.name == "C" { 0 } else { 1 };
        (api.set_substitute_callout)(
            mcx,
            record_callout as *const () as *mut c_void,
            which as *mut c_void,
        );
        (mcx, std::ptr::null_mut())
    }
}

#[test]
fn substitute_callout_sequence() {
    let _guard = global_lock();
    let cases: &[(&str, &str, &str)] = &[
        ("(a)", "aaa", "[$1]"),
        ("a", "banana", "X"),
        ("a*", "bab", "<$0>"),
        ("(?<n>\\w+)", "one two three", "${n}!"),
        ("(a)(b)", "abab", "$2$1"),
        ("x", "no match", "Q"),
        ("", "abc", "-"),
        ("(*MARK:mk)(a)", "aa", "$*MARK$1"),
        ("\\b", "ab cd", "|"),
        ("(a)|(b)", "ab", "$1$2"),
    ];
    let opts = [
        0,
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_UNSET_EMPTY
            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
    ];
    CALLOUT_RET.store(0, Ordering::SeqCst);
    CALLOUT_RET_ON.store(0, Ordering::SeqCst);
    unsafe {
        for &(pat, subj, rep) in cases {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for &o in &opts {
                for &bs in &[0usize, 1, 4, 128] {
                    clear_recs();
                    let mut prep = callout_prep();
                    let a = SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                    diff_sub_with(
                        &cc,
                        &rr,
                        &a,
                        &format!("callout pat={:?}", pat),
                        &mut prep,
                    );
                    let rc_ = REC_C.lock().unwrap().clone();
                    let rr_ = REC_R.lock().unwrap().clone();
                    assert_eq!(
                        rc_.len(),
                        rr_.len(),
                        "callout count differs pat={:?} subj={:?} rep={:?} opts={} bs={} \
                         (C={} Rust={})",
                        pat,
                        subj,
                        rep,
                        show_opts(o),
                        bs,
                        rc_.len(),
                        rr_.len()
                    );
                    for (i, (x, y)) in rc_.iter().zip(rr_.iter()).enumerate() {
                        assert_eq!(
                            x, y,
                            "callout block #{} differs pat={:?} subj={:?} rep={:?} \
                             opts={} bs={}\n C={:?}\n R={:?}",
                            i,
                            pat,
                            subj,
                            rep,
                            show_opts(o),
                            bs,
                            x,
                            y
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn substitute_callout_return_values() {
    let _guard = global_lock();
    let cases: &[(&str, &str, &str)] = &[
        ("(a)", "aaa", "[$1]"),
        ("a", "banana", "XYZ"),
        ("a*", "bab", "<$0>"),
        ("(a)(b)", "ababab", "$2$1"),
        ("\\w+", "one two three", "W"),
        ("x", "abc", "Q"),
    ];
    let opts = [
        0,
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY
            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
    ];
    unsafe {
        for &(pat, subj, rep) in cases {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for ret in [0i32, 1, 7, -1, -9] {
                for on in [0usize, 1, 2, 3] {
                    CALLOUT_RET.store(ret, Ordering::SeqCst);
                    CALLOUT_RET_ON.store(on, Ordering::SeqCst);
                    for &o in &opts {
                        for &bs in &[0usize, 2, 5, 128] {
                            clear_recs();
                            let mut prep = callout_prep();
                            let a = SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                            diff_sub_with(
                                &cc,
                                &rr,
                                &a,
                                &format!(
                                    "calloutret pat={:?} ret={} on={}",
                                    pat, ret, on
                                ),
                                &mut prep,
                            );
                            let x = REC_C.lock().unwrap().clone();
                            let y = REC_R.lock().unwrap().clone();
                            assert_eq!(
                                x, y,
                                "callout sequence differs pat={:?} ret={} on={} opts={} bs={}",
                                pat,
                                ret,
                                on,
                                show_opts(o),
                                bs
                            );
                        }
                    }
                }
            }
        }
    }
    CALLOUT_RET.store(0, Ordering::SeqCst);
    CALLOUT_RET_ON.store(0, Ordering::SeqCst);
}

// ============================================= 9. substitute case callout

/// Modes for `case_callout`, selected via the callout data pointer.
const CASE_MODE_ASCII: usize = 0;
/// always returns 0 (produces no output at all)
const CASE_MODE_ZERO: usize = 1;
/// doubles every code unit — forces the buffer-growth loop in do_case_copy()
const CASE_MODE_EXPAND: usize = 2;
/// signals an error (`~(PCRE2_SIZE)0`) -> PCRE2_ERROR_REPLACECASE
const CASE_MODE_ERROR: usize = 3;
/// identity copy
const CASE_MODE_IDENTITY: usize = 4;

unsafe extern "C" fn case_callout(
    input: *const u8,
    input_len: usize,
    output: *mut u8,
    output_cap: usize,
    to_case: i32,
    data: *mut c_void,
) -> usize {
    let mode = data as usize;
    let inp = if input_len == 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(input, input_len)
    };
    match mode {
        CASE_MODE_ZERO => 0,
        CASE_MODE_ERROR => usize::MAX,
        CASE_MODE_EXPAND => {
            let need = input_len * 2;
            if need <= output_cap {
                for (i, &b) in inp.iter().enumerate() {
                    *output.add(2 * i) = b;
                    *output.add(2 * i + 1) = b;
                }
            }
            need
        }
        CASE_MODE_IDENTITY => {
            if input_len <= output_cap {
                std::ptr::copy(input, output, input_len);
            }
            input_len
        }
        // CASE_MODE_ASCII: byte-wise ASCII casing (UTF-8 safe: only touches
        // bytes < 0x80).
        _ => {
            let mut out = Vec::with_capacity(input_len);
            for (i, &b) in inp.iter().enumerate() {
                let up = match to_case {
                    CASE_UPPER => true,
                    CASE_TITLE_FIRST => i == 0,
                    _ => false,
                };
                out.push(if up {
                    b.to_ascii_uppercase()
                } else {
                    b.to_ascii_lowercase()
                });
            }
            if out.len() <= output_cap {
                std::ptr::copy_nonoverlapping(out.as_ptr(), output, out.len());
            }
            out.len()
        }
    }
}

fn case_prep(mode: usize) -> impl FnMut(&'static Api, *mut c_void) -> (*mut c_void, *mut c_void)
{
    move |api: &'static Api, _code: *mut c_void| unsafe {
        let mcx = (api.match_context_create)(std::ptr::null_mut());
        assert!(!mcx.is_null());
        (api.set_substitute_case_callout)(
            mcx,
            case_callout as *const () as *mut c_void,
            mode as *mut c_void,
        );
        (mcx, std::ptr::null_mut())
    }
}

#[test]
fn substitute_case_callout() {
    let _guard = global_lock();
    let cases: &[(&str, &str)] = &[
        ("(\\w+)", "hello WORLD"),
        ("(a+)(b+)", "aabbaabb"),
        ("(.)", "abc"),
        ("(?<n>\\w+)", "MiXeD case"),
        ("x", "abc"),
        ("(\\w+)", ""),
    ];
    let reps = [
        "\\U$1\\E",
        "\\L$1\\E",
        "\\u$1",
        "\\l$1",
        "\\u\\L$1\\E",
        "\\l\\U$1\\E",
        "\\U$1",
        "\\U$0-$0\\E-$0",
        "\\Uab\\Lcd\\Uef\\E",
        "\\U",
        "\\U\\E",
        "\\u\\u$1",
        "\\U${n:-none}\\E",
        "pre\\Umid\\Epost",
        "\\U\\x{e9}\\E",
        "\\l\\U$0\\E",
    ];
    let modes = [
        CASE_MODE_ASCII,
        CASE_MODE_ZERO,
        CASE_MODE_EXPAND,
        CASE_MODE_ERROR,
        CASE_MODE_IDENTITY,
    ];
    let opts = [
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL
            | PCRE2_SUBSTITUTE_EXTENDED
            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
    ];
    unsafe {
        for &utf in &[0u32, PCRE2_UTF] {
            for &(pat, subj) in cases {
                let cfg = CompileCfg::new(utf);
                let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &cfg, pat);
                if cc.code.is_null() {
                    continue;
                }
                for rep in reps {
                    for &mode in &modes {
                        for &o in &opts {
                            for &bs in &[0usize, 1, 4, 12, 256] {
                                let mut prep = case_prep(mode);
                                let a =
                                    SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                                diff_sub_with(
                                    &cc,
                                    &rr,
                                    &a,
                                    &format!(
                                        "casecallout pat={:?} mode={} utf={:#x}",
                                        pat, mode, utf
                                    ),
                                    &mut prep,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn substitute_case_callout_utf_and_marks() {
    let _guard = global_lock();
    // The case callout also handles $*MARK and multi-byte input.
    let cases: &[(&str, &str, &str)] = &[
        ("(*MARK:mArK)(a)", "aa", "\\U$*MARK\\E"),
        ("(*MARK:mArK)(a)", "aa", "\\u$*MARK"),
        ("(\\X)", "caf\u{00e9}", "\\U$1\\E"),
        ("(\\X+)", "\u{00e9}\u{00e8}\u{00ea}", "\\U$1\\E"),
        ("(.)", "\u{1F600}", "\\U$1\\E"),
        ("(\\w+)", "Stra\u{00df}e", "\\U$1\\E"),
        ("(\\w+)", "\u{0130}\u{0131}x", "\\L$1\\E"),
    ];
    unsafe {
        for &(pat, subj, rep) in cases {
            for &utf in &[0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP] {
                let cfg = CompileCfg::new(utf);
                let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &cfg, pat);
                if cc.code.is_null() {
                    continue;
                }
                for &mode in &[
                    CASE_MODE_ASCII,
                    CASE_MODE_EXPAND,
                    CASE_MODE_ZERO,
                    CASE_MODE_IDENTITY,
                ] {
                    for &o in &[
                        PCRE2_SUBSTITUTE_EXTENDED,
                        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
                        PCRE2_SUBSTITUTE_GLOBAL
                            | PCRE2_SUBSTITUTE_EXTENDED
                            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                    ] {
                        for &bs in &[0usize, 2, 6, 256] {
                            let mut prep = case_prep(mode);
                            let a = SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                            diff_sub_with(
                                &cc,
                                &rr,
                                &a,
                                &format!(
                                    "casemark pat={:?} mode={} utf={:#x}",
                                    pat, mode, utf
                                ),
                                &mut prep,
                            );
                        }
                    }
                }
                // ... and with BOTH callouts installed at once.
                for &o in &[
                    PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_GLOBAL
                        | PCRE2_SUBSTITUTE_EXTENDED
                        | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                ] {
                    clear_recs();
                    CALLOUT_RET.store(0, Ordering::SeqCst);
                    CALLOUT_RET_ON.store(0, Ordering::SeqCst);
                    let mut prep = |api: &'static Api, _code: *mut c_void| {
                        let mcx = (api.match_context_create)(std::ptr::null_mut());
                        let which: usize = if api.name == "C" { 0 } else { 1 };
                        (api.set_substitute_callout)(
                            mcx,
                            record_callout as *const () as *mut c_void,
                            which as *mut c_void,
                        );
                        (api.set_substitute_case_callout)(
                            mcx,
                            case_callout as *const () as *mut c_void,
                            CASE_MODE_ASCII as *mut c_void,
                        );
                        (mcx, std::ptr::null_mut())
                    };
                    let a = SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, 256);
                    diff_sub_with(&cc, &rr, &a, "both-callouts", &mut prep);
                    assert_eq!(
                        *REC_C.lock().unwrap(),
                        *REC_R.lock().unwrap(),
                        "both-callouts: callout sequence differs pat={:?} subj={:?}",
                        pat,
                        subj
                    );
                }
            }
        }
    }
}

// ================================== default (built-in) case transformation

#[test]
fn substitute_default_case_forcing() {
    // No case callout: exercises default_substitute_case_callout(), including
    // the non-ASCII path that consults the case tables.
    let pats = ["(\\w+)", "(.)", "(\\X+)", "(?<n>\\w+)", "(a)(b)", "(*MARK:mK)(a)"];
    let subjs = [
        "",
        "abc",
        "ABC",
        "MiXeD",
        "caf\u{00e9}",
        "\u{00c9}\u{00e9}\u{00df}",
        "\u{0130}\u{0131}",
        "\u{03A3}\u{03C3}\u{03C2}",
        "\u{1F600}",
        "a1b2",
    ];
    let reps = [
        "\\U$1\\E",
        "\\L$1\\E",
        "\\u$1",
        "\\l$1",
        "\\u\\L$1\\E",
        "\\l\\U$1\\E",
        "\\U$1",
        "\\L$1",
        "\\U$0\\E-\\L$0\\E",
        "\\Uab\\Lcd\\E",
        "\\u\\u$1",
        "\\l\\l$1",
        "\\U\\x{e9}\\x{df}\\E",
        "\\L\\x{c9}\\E",
        "\\U$*MARK\\E",
        "\\u$*MARK",
        "\\U${n:-dEf}\\E",
        "\\U\\Q$1\\E\\E",
    ];
    unsafe {
        for &utf in &[0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_UCP] {
            for pat in pats {
                let cfg = CompileCfg::new(utf);
                let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &cfg, pat);
                if cc.code.is_null() {
                    continue;
                }
                for subj in subjs {
                    for rep in reps {
                        for &o in &[
                            PCRE2_SUBSTITUTE_EXTENDED,
                            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
                            PCRE2_SUBSTITUTE_GLOBAL
                                | PCRE2_SUBSTITUTE_EXTENDED
                                | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                            PCRE2_SUBSTITUTE_EXTENDED
                                | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                        ] {
                            for &bs in &[0usize, 1, 5, 256] {
                                let a =
                                    SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                                diff_sub(
                                    &cc,
                                    &rr,
                                    &a,
                                    &format!(
                                        "defaultcase pat={:?} utf={:#x}",
                                        pat, utf
                                    ),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

// ============================================ conditional / nested ${...}

#[test]
fn substitute_conditional_and_nested_forms() {
    let reps = [
        "${1:+set:unset}",
        "${1:-default}",
        "${2:+set:unset}",
        "${2:-default}",
        "${n:+has:hasnt}",
        "${n:-fb}",
        "${1:+${2:+both:only1}:none}",
        "${1:+${2:-d2}:x}",
        "${1:-${2:-deep}}",
        "${1:+a\\Ub\\Ec:d}",
        "${1:+\\Qa:b\\E:c}",
        "${1:+a\\}b:c}",
        "${1:+a${2}b:c${1}d}",
        "${1:+:}",
        "${1:-}",
        "${1:+}",
        "${1:++:-}",
        "${1:+:::}",
        "${1:+\\:}",
        "${1:-\\}}",
        "${1:+x",
        "${1:+x:y",
        "${1:*x}",
        "${1:}",
        "${1:",
        // deep nesting: the PTR_STACK_SIZE (20) limit
        "${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+\
          ${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+${1:+deep:x}:x}:x}:x}:x}\
          :x}:x}:x}:x}:x}:x}:x}:x}:x}:x}:x}:x}:x}:x}:x}:x}:x}:x}:x}",
        // \Q inside conditionals and unbalanced \E
        "${1:+\\Q}\\E:z}",
        "${n:+\\U$n\\E:\\L$n\\E}",
    ];
    unsafe {
        for pat in ["(?<n>a)(b)?", "(a)|(b)", "(?<n>a+)(?<m>b*)", "a"] {
            let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);
            if cc.code.is_null() {
                continue;
            }
            for subj in ["", "a", "b", "ab", "aab"] {
                for rep in reps {
                    for &o in &[
                        0,
                        PCRE2_SUBSTITUTE_EXTENDED,
                        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY,
                        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                        PCRE2_SUBSTITUTE_GLOBAL
                            | PCRE2_SUBSTITUTE_EXTENDED
                            | PCRE2_SUBSTITUTE_UNSET_EMPTY
                            | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                        PCRE2_SUBSTITUTE_GLOBAL
                            | PCRE2_SUBSTITUTE_EXTENDED
                            | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                    ] {
                        for &bs in &[0usize, 3, 256] {
                            let a =
                                SubArgs::new(subj.as_bytes(), rep.as_bytes(), o, bs);
                            diff_sub(
                                &cc,
                                &rr,
                                &a,
                                &format!("cond pat={:?}", pat),
                            );
                        }
                    }
                }
            }
        }
    }
}

// ============================================================ error paths

#[test]
fn substitute_error_paths() {
    unsafe {
        let (c, r) = both();
        let pat = "(a)";
        let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), &CompileCfg::new(0), pat);

        // NULL subject with non-zero length, NULL replacement with non-zero
        // rlength -> PCRE2_ERROR_NULL. Also the legal NULL/0 forms.
        for &(slen, rlen) in &[(0usize, 0usize), (0, 3), (3, 0), (3, 3)] {
            for &(snull, rnull) in
                &[(true, true), (true, false), (false, true), (false, false)]
            {
                let mut outs: Vec<(i32, usize, [u8; 32])> = Vec::new();
                for (api, code) in [(c, cc.code), (r, rr.code)] {
                    let mut buf = [SENTINEL; 32];
                    let subj: *const u8 = if snull {
                        std::ptr::null()
                    } else {
                        b"aaa".as_ptr()
                    };
                    let rep: *const u8 = if rnull {
                        std::ptr::null()
                    } else {
                        b"XYZ".as_ptr()
                    };
                    let mut outlen = 24usize;
                    let rc = (api.substitute)(
                        code,
                        subj,
                        slen,
                        0,
                        0,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        rep,
                        rlen,
                        buf.as_mut_ptr(),
                        &mut outlen,
                    );
                    outs.push((rc, outlen, buf));
                }
                assert_eq!(
                    outs[0].0, outs[1].0,
                    "null-args rc differs slen={} rlen={} snull={} rnull={}",
                    slen, rlen, snull, rnull
                );
                assert_eq!(
                    outs[0].1, outs[1].1,
                    "null-args outlen differs slen={} rlen={} snull={} rnull={}",
                    slen, rlen, snull, rnull
                );
                assert_eq!(
                    outs[0].2, outs[1].2,
                    "null-args buffer differs slen={} rlen={} snull={} rnull={}",
                    slen, rlen, snull, rnull
                );
            }
        }

        // start_offset > length -> PCRE2_ERROR_BADOFFSET
        for &st in &[4usize, 5, 100, usize::MAX / 2] {
            let a = SubArgs::new(b"aaa", b"X", 0, 32).start(st);
            let (co, _) = diff_sub(&cc, &rr, &a, "badoffset");
            assert_eq!(co.rc, ERR_BADOFFSET, "expected BADOFFSET for start={}", st);
        }

        // \K that makes the match end before it starts -> BADSUBSPATTERN
        for kpat in ["(?=(a))\\Kb", "a\\Kb", "(?<=a)\\Kb", "ab\\K", "\\Ka"] {
            let (kc, kr) =
                compile_both(kpat.as_bytes(), kpat.len(), &CompileCfg::new(0), kpat);
            if kc.code.is_null() {
                continue;
            }
            for subj in ["", "a", "ab", "abab", "xaby"] {
                for &o in &[
                    0,
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                ] {
                    for &bs in &[0usize, 4, 64] {
                        let a = SubArgs::new(subj.as_bytes(), b"[$0]", o, bs);
                        diff_sub(&kc, &kr, &a, &format!("K pat={:?}", kpat));
                    }
                }
            }
        }

        // match limits reached inside the substitution loop
        for lpat in ["(a+)+b", "(a|aa)+c"] {
            let (lc, lr) =
                compile_both(lpat.as_bytes(), lpat.len(), &CompileCfg::new(0), lpat);
            if lc.code.is_null() {
                continue;
            }
            for &(ml, dl, hl) in &[(1u32, 0u32, 0u32), (10, 0, 0), (0, 1, 0), (0, 0, 1)] {
                let mut prep = move |api: &'static Api, _code: *mut c_void| {
                    let mcx = (api.match_context_create)(std::ptr::null_mut());
                    if ml != 0 {
                        (api.set_match_limit)(mcx, ml);
                    }
                    if dl != 0 {
                        (api.set_depth_limit)(mcx, dl);
                    }
                    if hl != 0 {
                        (api.set_heap_limit)(mcx, hl);
                    }
                    (mcx, std::ptr::null_mut())
                };
                let subj = "aaaaaaaaaaaaaaaaaaaaaaaa";
                let a = SubArgs::new(
                    subj.as_bytes(),
                    b"X",
                    PCRE2_SUBSTITUTE_GLOBAL,
                    64,
                );
                diff_sub_with(&lc, &lr, &a, &format!("limits pat={:?}", lpat), &mut prep);
            }
        }
    }
}

// ==================================================== 10. randomized fuzz

/// Alphabet heavy in the metacharacters of the replacement language.
const REP_ALPHABET: &[u8] = b"$${}\\\\0123456789ABCDEFabcdefULEQlu:+-<>&`'_*nrtxo{}$\\gMARKk";
const SUBJ_ALPHABET: &[u8] = b"aabbcXY \n\t01\xc3\xa9\xff";
const PAT_ALPHABET: &[u8] = b"ab().*+?|[]^$\\dws{}<>?:=!0123456789PnKQE";

#[test]
fn substitute_fuzz_random() {
    let mut rng = Rng::new(SEED);
    // A fixed set of compiled patterns; the fuzzing happens on the subject,
    // the replacement, the options and the buffer size.
    let pat_src: &[&str] = &[
        "(a)",
        "(a)(b)",
        "(?<n>a+)(?<m>b*)",
        "(?<n>a)|(?<n>b)",
        "a*",
        "\\b",
        "(?=a)",
        "(.)",
        "(\\w+)",
        "(a)(b)?(c)?",
        "(*MARK:mk)(a)",
        "(a|ab|abc)",
        "()",
        "x",
        "(?<n>.)(?<m>.)?",
        "[ab]+",
        "(?:(a)|(b))+",
        "(a)\\1?",
        ".",
        "(?<n>\\d+)",
    ];
    let cfgs = [
        CompileCfg::new(PCRE2_DUPNAMES),
        CompileCfg::new(PCRE2_DUPNAMES | PCRE2_UTF),
        CompileCfg::new(PCRE2_DUPNAMES | PCRE2_CASELESS),
    ];
    // `bool` records whether the pattern was compiled in UTF mode.
    let mut compiled: Vec<(Compiled, Compiled, bool)> = Vec::new();
    unsafe {
        for cfg in &cfgs {
            for p in pat_src {
                let (cc, rr) = compile_both(p.as_bytes(), p.len(), cfg, p);
                if !cc.code.is_null() {
                    compiled.push((cc, rr, cfg.options & PCRE2_UTF != 0));
                }
            }
        }
        assert!(!compiled.is_empty());

        let optbits = [
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_EXTENDED,
            PCRE2_SUBSTITUTE_UNSET_EMPTY,
            PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
            PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
            PCRE2_SUBSTITUTE_LITERAL,
            PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
            PCRE2_NOTBOL,
            PCRE2_NOTEMPTY,
            PCRE2_NO_UTF_CHECK,
        ];

        for iter in 0..3000u32 {
            let idx = rng.below(compiled.len() as u32) as usize;
            let (cc, rr, is_utf) = &compiled[idx];

            let slen = rng.below(20) as usize;
            let subject = if rng.below(8) == 0 {
                rng.raw_bytes(slen)
            } else {
                rng.bytes_from(slen, SUBJ_ALPHABET)
            };
            let rlen = rng.below(18) as usize;
            let replacement = rng.bytes_from(rlen, REP_ALPHABET);

            let mut options = 0u32;
            for &b in &optbits {
                if rng.below(3) == 0 {
                    options |= b;
                }
            }
            // Invalid UTF + PCRE2_NO_UTF_CHECK is documented undefined
            // behaviour (and really does crash the C library), so never combine
            // the two.
            if *is_utf {
                options &= !PCRE2_NO_UTF_CHECK;
            }
            let bufsize = *rng.pick(&[0usize, 1, 2, 3, 5, 8, 16, 40, 200]);
            let start = if subject.is_empty() {
                0
            } else {
                rng.below(subject.len() as u32 + 1) as usize
            };

            let a = SubArgs::new(&subject, &replacement, options, bufsize).start(start);
            diff_sub(cc, rr, &a, &format!("fuzz iter={}", iter));
        }
    }
}

#[test]
fn substitute_fuzz_random_patterns() {
    let mut rng = Rng::new(SEED ^ 0x1234_5678);
    let mut done = 0u32;
    let mut tries = 0u32;
    unsafe {
        while done < 400 && tries < 4000 {
            tries += 1;
            let plen = rng.range(1, 12) as usize;
            let pat = rng.bytes_from(plen, PAT_ALPHABET);
            let cfg = CompileCfg::new(if rng.bool() { PCRE2_DUPNAMES } else { 0 });
            let (cc, rr) = compile_both(&pat, pat.len(), &cfg, "fuzzpat");
            if cc.code.is_null() {
                continue;
            }
            done += 1;
            for _ in 0..4 {
                let slen = rng.below(14) as usize;
                let subject = rng.bytes_from(slen, SUBJ_ALPHABET);
                let rlen = rng.below(14) as usize;
                let replacement = rng.bytes_from(rlen, REP_ALPHABET);
                let mut options = 0u32;
                for &b in &[
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_UNSET_EMPTY,
                    PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                    PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                    PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                ] {
                    if rng.below(3) == 0 {
                        options |= b;
                    }
                }
                let bufsize = *rng.pick(&[0usize, 1, 4, 16, 128]);
                let a = SubArgs::new(&subject, &replacement, options, bufsize);
                diff_sub(
                    &cc,
                    &rr,
                    &a,
                    &format!("fuzzpat pat={:?}", show(&pat)),
                );
            }
        }
    }
    assert!(done > 100, "fuzz_random_patterns compiled only {} patterns", done);
}
