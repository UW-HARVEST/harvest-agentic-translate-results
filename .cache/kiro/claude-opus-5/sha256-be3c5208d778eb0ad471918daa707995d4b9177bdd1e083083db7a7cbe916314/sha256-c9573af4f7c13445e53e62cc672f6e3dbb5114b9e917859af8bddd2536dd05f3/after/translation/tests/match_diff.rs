//! Phase B — valid-path differential tests over the configuration surface.
//!
//! Each `#[test]` here corresponds to a block of rows in `CONFIGS.md`. Every row
//! is driven through BOTH shared objects with MANY randomized inputs (fixed
//! seed) and compares:
//!   * `pcre2_compile` result (errorcode / erroroffset)
//!   * the whole compiled bytecode, byte for byte (via `pcre2_serialize_encode`)
//!   * every `pcre2_pattern_info` item
//!   * `pcre2_match`, `pcre2_dfa_match` and `pcre2_jit_match` return codes,
//!     ovectors, `pcre2_get_startchar` and `pcre2_get_mark`
//!   * the substring accessors and `pcre2_next_match` iteration

mod common;
use common::*;
use std::ffi::c_void;

// ---------------------------------------------------------------------------
// Pattern pool: one entry per distinct compiled construct in the C compiler.
// ---------------------------------------------------------------------------

pub static PATTERNS: &[&[u8]] = &[
    // literals / empties
    b"",
    b"a",
    b"abc",
    b"a\x00b",
    b"\\Qa.b\\E",
    // quantifiers, greedy / lazy / possessive
    b"a*",
    b"a+",
    b"a?",
    b"a{2,4}",
    b"a{0,}",
    b"a{3}",
    b"a*?",
    b"a+?",
    b"a??",
    b"a{2,4}?",
    b"a*+",
    b"a++",
    b"a?+",
    b"a{2,4}+",
    b"(?:ab)*",
    b"(?:ab)+?",
    b"(?:ab){2,3}+",
    // classes
    b"[abc]",
    b"[^abc]",
    b"[a-z0-9_]",
    b"[[:alpha:][:digit:]]",
    b"[^[:space:]]",
    b"[\\d\\s\\w]",
    b"[\\D\\S\\W]",
    b"[\\x00-\\xff]",
    b"[]]",
    b"[^]]",
    b"[a-]",
    b"[-a]",
    b"[\\b]",
    b"[\\Q-\\E]",
    // escapes and character types
    b"\\d\\D\\s\\S\\w\\W",
    b"\\h\\H\\v\\V",
    b"\\R",
    b"\\N",
    b".",
    b"\\C",
    b"\\X",
    b"\\b\\B",
    b"\\A\\Z\\z",
    b"\\G",
    b"^$",
    b"\\n\\r\\t\\f\\a\\e",
    b"\\0",
    b"\\101",
    b"\\o{101}",
    b"\\x41",
    b"\\x{41}",
    b"\\cA",
    // groups, alternation, backreferences
    b"(a)(b)(c)",
    b"(?:a|b|c)",
    b"(a)\\1",
    b"(?<n>a)\\k<n>",
    b"(?<n>a)\\k'n'",
    b"(?<n>a)\\k{n}",
    b"(?<n>a)(?P=n)",
    b"(?'n'a)\\g{n}",
    b"(a)(?1)",
    b"(a)\\g{-1}",
    b"(a)\\g<1>",
    b"(?|(a)|(b))",
    b"(?<a1>x)|(?<a1>y)",
    b"(a)(?(1)b|c)",
    b"(?(?=a)b|c)",
    b"(?(DEFINE)(?<d>x))(?&d)",
    b"(?(VERSION>=10.0)a|b)",
    // assertions
    b"(?=abc)",
    b"(?!abc)",
    b"(?<=abc)",
    b"(?<!abc)",
    b"(?<=a|bc)",
    b"(?<=a{1,3})",
    b"(?*a)",
    b"(?<*a)",
    b"(*positive_lookahead:a)",
    b"(*negative_lookahead:a)",
    b"(*positive_lookbehind:a)",
    b"(*negative_lookbehind:a)",
    b"(*atomic:a)",
    b"(*script_run:a)",
    b"(*asr:a)",
    // atomic / recursion / subroutines
    b"(?>a+)b",
    b"(?R)?a",
    b"a(?0)?",
    b"(?1)(a)",
    b"(?+1)(a)",
    b"(?-1)(a)",
    // options inside the pattern
    b"(?i)abc",
    b"(?i:abc)DEF",
    b"(?-i)abc",
    b"(?x) a b c",
    b"(?xx) a b c",
    b"(?s).",
    b"(?m)^a$",
    b"(?U)a+",
    b"(?J)(?<n>a)(?<n>b)",
    b"(?n)(a)(b)",
    b"(*UTF)a",
    b"(*UCP)\\w",
    b"(*CR)^a$",
    b"(*LF)^a$",
    b"(*CRLF)^a$",
    b"(*ANY)^a$",
    b"(*ANYCRLF)^a$",
    b"(*NUL)^a$",
    b"(*BSR_ANYCRLF)\\R",
    b"(*BSR_UNICODE)\\R",
    b"(*LIMIT_MATCH=100)a",
    b"(*LIMIT_DEPTH=100)a",
    b"(*LIMIT_HEAP=100)a",
    b"(*NO_AUTO_POSSESS)a+",
    b"(*NO_DOTSTAR_ANCHOR).*a",
    b"(*NO_START_OPT)a",
    b"(*NOTEMPTY)a*",
    b"(*NOTEMPTY_ATSTART)a*",
    b"(*NO_JIT)a",
    b"(*CASELESS)abc",
    // verbs
    b"a(*FAIL)",
    b"a(*ACCEPT)b",
    b"a(*COMMIT)b",
    b"a(*PRUNE)b",
    b"a(*SKIP)b",
    b"a(*THEN)b",
    b"a(*MARK:m1)b",
    b"a(*:m2)b",
    b"(a(*PRUNE:p)b|c)",
    b"(a(*SKIP:s)b|c)",
    b"(a(*THEN:t)b|c)",
    // callouts
    b"a(?C)b",
    b"a(?C1)b",
    b"a(?C255)b",
    b"a(?C{txt})b",
    b"a(?C\"txt\")b",
    b"a(?C'txt')b",
    b"a(?C`txt`)b",
    b"a(?C^txt^)b",
    b"a(?C%txt%)b",
    b"a(?C#txt#)b",
    b"a(?C$txt$)b",
    b"a(?C{te}}xt})b",
    // \K
    b"a\\Kb",
    b"(?:a\\Kb)+",
    // Unicode properties
    b"\\p{L}",
    b"\\P{L}",
    b"\\p{Lu}",
    b"\\p{^Lu}",
    b"\\pL",
    b"\\p{Greek}",
    b"\\p{Any}",
    b"\\p{Xan}",
    b"\\p{Xps}",
    b"\\p{Xsp}",
    b"\\p{Xuc}",
    b"\\p{Xwd}",
    b"\\p{Bidi_Control}",
    b"\\p{ASCII}",
    b"\\p{Cased}",
    b"[\\p{Nd}\\p{Lu}]",
    // extended classes
    b"(?[[a-z]&&[b-y]])",
    b"(?[[a-z]--[m-p]])",
    b"(?[[a-z]||[0-9]])",
    b"(?[![a-z]])",
    b"(?[[a-z]~~[b-y]])",
    b"(?[ [a] || [b] ])",
    // longer / composite
    b"^(?:[a-z]+)\\s*=\\s*(?<val>\"[^\"]*\"|\\d+)\\s*;?$",
    b"(\\w+)@(\\w+)\\.(\\w{2,4})",
    b"(?i)(?:https?|ftp)://[^\\s/$.?#].[^\\s]*",
    b"((a)|(b))+c",
    b"(a?){10}b",
    b"(?:a|ab|abc|abcd)+x",
    b"\\b\\w+\\b",
];

pub static SUBJECT_SEEDS: &[&[u8]] = &[
    b"",
    b"a",
    b"A",
    b"abc",
    b"ABC",
    b"aaa",
    b"abcabc",
    b"xyz",
    b"a\nb",
    b"a\r\nb",
    b"a\rb",
    b"a\x85b",
    b"a\x00b",
    b"a=1;",
    b"foo = \"bar\";",
    b"user@example.com",
    b"https://example.com/x?y=1",
    b"  a  b  c  ",
    b"\xC3\xA9\xC3\xA8",
    b"\xE2\x82\xAC",
    b"\xF0\x9F\x98\x80",
    b"\xC3\xA9abc\xE2\x82\xAC",
    b"0123456789",
    b"_-+=",
    b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
];

/// Every match-option combination worth distinguishing at match time.
pub static MATCH_OPTS: &[u32] = &[
    0,
    o::NOTBOL,
    o::NOTEOL,
    o::NOTBOL | o::NOTEOL,
    o::NOTEMPTY,
    o::NOTEMPTY_ATSTART,
    o::ANCHORED,
    o::ENDANCHORED,
    o::ANCHORED | o::ENDANCHORED,
    o::PARTIAL_SOFT,
    o::PARTIAL_HARD,
    o::NO_UTF_CHECK,
    o::COPY_MATCHED_SUBJECT,
    o::DISABLE_RECURSELOOP_CHECK,
    o::NO_JIT,
    o::NOTBOL | o::NOTEMPTY | o::ANCHORED,
    o::PARTIAL_SOFT | o::NOTEOL,
];

pub struct Cfg {
    pub label: &'static str,
    pub opts: u32,
    pub xopts: u32,
    pub newline: u32,
    pub bsr: u32,
    pub optimize: &'static [u32],
    pub custom_tables: bool,
    pub max_varlookbehind: Option<u32>,
}

impl Cfg {
    const fn new(label: &'static str) -> Cfg {
        Cfg {
            label,
            opts: 0,
            xopts: 0,
            newline: 0,
            bsr: 0,
            optimize: &[],
            custom_tables: false,
            max_varlookbehind: None,
        }
    }
}

struct Ctxs {
    cc: Ctx,
    cr: Ctx,
    tc: *const u8,
    tr: *const u8,
}

fn make_contexts(p: &Pair, cfg: &Cfg) -> Ctxs {
    unsafe {
        let cc = (p.c.compile_context_create)(std::ptr::null_mut());
        let cr = (p.r.compile_context_create)(std::ptr::null_mut());
        assert!(!cc.is_null() && !cr.is_null());
        if cfg.xopts != 0 {
            assert_eq!(
                (p.c.set_compile_extra_options)(cc, cfg.xopts),
                (p.r.set_compile_extra_options)(cr, cfg.xopts)
            );
        }
        if cfg.newline != 0 {
            assert_eq!((p.c.set_newline)(cc, cfg.newline), (p.r.set_newline)(cr, cfg.newline));
        }
        if cfg.bsr != 0 {
            assert_eq!((p.c.set_bsr)(cc, cfg.bsr), (p.r.set_bsr)(cr, cfg.bsr));
        }
        for &d in cfg.optimize {
            assert_eq!((p.c.set_optimize)(cc, d), (p.r.set_optimize)(cr, d));
        }
        if let Some(v) = cfg.max_varlookbehind {
            assert_eq!(
                (p.c.set_max_varlookbehind)(cc, v),
                (p.r.set_max_varlookbehind)(cr, v)
            );
        }
        let (tc, tr) = if cfg.custom_tables {
            let tc = (p.c.maketables)(std::ptr::null_mut());
            let tr = (p.r.maketables)(std::ptr::null_mut());
            assert!(!tc.is_null() && !tr.is_null());
            assert_eq!((p.c.set_character_tables)(cc, tc), (p.r.set_character_tables)(cr, tr));
            (tc, tr)
        } else {
            (std::ptr::null(), std::ptr::null())
        };
        Ctxs { cc, cr, tc, tr }
    }
}

fn free_contexts(p: &Pair, c: Ctxs) {
    unsafe {
        (p.c.compile_context_free)(c.cc);
        (p.r.compile_context_free)(c.cr);
        if !c.tc.is_null() {
            (p.c.maketables_free)(std::ptr::null_mut(), c.tc);
            (p.r.maketables_free)(std::ptr::null_mut(), c.tr);
        }
    }
}

/// Compare every observable output of one match attempt.
fn cmp_match_state(p: &Pair, mdc: MData, mdr: MData, rc: i32, label: &str) {
    unsafe {
        let cntc = (p.c.get_ovector_count)(mdc);
        let cntr = (p.r.get_ovector_count)(mdr);
        assert_eq!(cntc, cntr, "ovector count [{}]", label);
        // How many ovector PAIRS the C library actually defines:
        //   rc  > 0            -> rc pairs
        //   rc == 0            -> the whole (too-small) ovector was filled
        //   rc == PARTIAL      -> pair 0 only (start/end of the partial match)
        //   any other rc < 0   -> nothing is defined (untouched malloc memory)
        let defined = if rc > 0 {
            rc as usize
        } else if rc == 0 {
            cntc as usize
        } else if rc == err::PARTIAL {
            1
        } else {
            0
        };
        let n = defined * 2;
        let oc = std::slice::from_raw_parts((p.c.get_ovector_pointer)(mdc), n);
        let or = std::slice::from_raw_parts((p.r.get_ovector_pointer)(mdr), n);
        assert_eq!(oc, or, "ovector [{}] rc={}", label, rc);
        if rc >= 0 || rc == err::PARTIAL {
            assert_eq!(
                (p.c.get_startchar)(mdc),
                (p.r.get_startchar)(mdr),
                "startchar [{}]",
                label
            );
        }
        // `match_data->mark` is only assigned on the paths that reach the end of
        // pcre2_match (success, NOMATCH or PARTIAL). The early argument-validation
        // returns (NULL / BADOPTION / BADOFFSET / BADMAGIC / BADMODE) leave it as
        // untouched malloc memory in the C library, so it is not comparable there.
        if rc >= 0 || rc == err::PARTIAL || rc == err::NOMATCH {
            let mc = (p.c.get_mark)(mdc);
            let mr = (p.r.get_mark)(mdr);
            assert_eq!(mc.is_null(), mr.is_null(), "mark null-ness [{}]", label);
            if !mc.is_null() {
                let lc = (p.c.priv_strlen)(mc);
                let lr = (p.r.priv_strlen)(mr);
                assert_eq!(lc, lr, "mark length [{}]", label);
                assert_eq!(
                    std::slice::from_raw_parts(mc, lc),
                    std::slice::from_raw_parts(mr, lr),
                    "mark bytes [{}]",
                    label
                );
            }
        }
        assert_eq!(
            (p.c.get_match_data_size)(mdc),
            (p.r.get_match_data_size)(mdr),
            "match_data_size [{}]",
            label
        );
        // next_match iteration (only defined after a successful/failed match).
        let mut oc2: Sz = 0xAAAA;
        let mut or2: Sz = 0xAAAA;
        let mut lc2: u32 = 0xAAAA;
        let mut lr2: u32 = 0xAAAA;
        let a = (p.c.next_match)(mdc, &mut oc2, &mut lc2);
        let b = (p.r.next_match)(mdr, &mut or2, &mut lr2);
        assert_eq!(a, b, "next_match [{}]", label);
        assert_eq!((oc2, lc2), (or2, lr2), "next_match outputs [{}]", label);
    }
}

/// PCRE2 documents that passing `PCRE2_NO_UTF_CHECK` with a subject that is not
/// valid UTF (or a start offset that is not on a character boundary) is
/// *undefined behaviour* — the C library then indexes its Unicode tables with
/// out-of-range values. Such inputs are therefore not comparable and must be
/// skipped.
fn utf_unsafe(p: &Pair, cp: &CodePair, subj: &[u8], slen: Sz, start: Sz, mopts: u32) -> bool {
    let eff0 = if slen == PCRE2_ZERO_TERMINATED {
        subj.iter().position(|&b| b == 0).unwrap_or(subj.len())
    } else {
        slen.min(subj.len())
    };
    // Upstream C crash: MATCH_INVALID_UTF + a lookbehind + invalid UTF subject.
    if c_crashes_on_invalid_utf(p, cp, subj, eff0, start) {
        return true;
    }
    if mopts & o::NO_UTF_CHECK == 0 {
        return false;
    }
    let mut allopts: u32 = 0;
    unsafe {
        (p.c.pattern_info)(cp.c, info::ALLOPTIONS, &mut allopts as *mut _ as *mut c_void);
    }
    if allopts & o::UTF == 0 {
        return false;
    }
    let eff = if slen == PCRE2_ZERO_TERMINATED {
        subj.iter().position(|&b| b == 0).unwrap_or(subj.len())
    } else {
        slen.min(subj.len())
    };
    let mut off: Sz = 0;
    let valid = if eff == 0 {
        true
    } else {
        unsafe { (p.c.priv_valid_utf)(subj.as_ptr(), eff, &mut off) == 0 }
    };
    if !valid {
        return true;
    }
    // Start offset must be on a character boundary.
    if start > eff {
        return false; // BADOFFSET is still checked even with NO_UTF_CHECK
    }
    if start < eff && (subj[start] & 0xC0) == 0x80 {
        return true;
    }
    false
}

/// Run one subject through all three match entry points in both libraries.
fn run_subject(
    p: &Pair,
    cp: &CodePair,
    subj: &[u8],
    slen: Sz,
    start: Sz,
    mopts: u32,
    ovn: Option<u32>,
    label: &str,
) {
    if utf_unsafe(p, cp, subj, slen, start, mopts) {
        return;
    }
    // PCRE2_ZERO_TERMINATED makes the library call strlen() on the subject, so it
    // must really be NUL-terminated; build an owned copy in that case.
    let owned;
    let sp: *const u8 = if slen == PCRE2_ZERO_TERMINATED {
        owned = {
            let mut v = subj.to_vec();
            v.push(0);
            v
        };
        owned.as_ptr()
    } else if subj.is_empty() {
        std::ptr::null()
    } else {
        subj.as_ptr()
    };
    unsafe {
        // A bounded heap limit keeps the comparison deterministic: without it,
        // pathological patterns (e.g. `(?<g1>(?1))` with
        // PCRE2_DISABLE_RECURSELOOP_CHECK) make each library ask for ~2.7 GiB of
        // backtracking frames, and because both match_data objects are alive at
        // the same time it is the *second* allocation that fails, which depends on
        // the machine rather than on the code. With the limit set, both libraries
        // take exactly the same decision. The unbounded default is covered by
        // `match_limits_produce_same_error`.
        let mctx_c = (p.c.match_context_create)(std::ptr::null_mut());
        let mctx_r = (p.r.match_context_create)(std::ptr::null_mut());
        assert!(!mctx_c.is_null() && !mctx_r.is_null());
        assert_eq!(
            (p.c.set_heap_limit)(mctx_c, 262144),
            (p.r.set_heap_limit)(mctx_r, 262144)
        );
        let (mdc, mdr) = match ovn {
            Some(n) => (
                (p.c.match_data_create)(n, std::ptr::null_mut()),
                (p.r.match_data_create)(n, std::ptr::null_mut()),
            ),
            None => (
                (p.c.match_data_create_from_pattern)(cp.c, std::ptr::null_mut()),
                (p.r.match_data_create_from_pattern)(cp.r, std::ptr::null_mut()),
            ),
        };
        assert!(!mdc.is_null() && !mdr.is_null());

        // ---- pcre2_match -------------------------------------------------
        let a = (p.c.pcre2_match)(cp.c, sp, slen, start, mopts, mdc, mctx_c);
        let b = (p.r.pcre2_match)(cp.r, sp, slen, start, mopts, mdr, mctx_r);
        assert_eq!(a, b, "pcre2_match rc [{}]", label);
        cmp_match_state(p, mdc, mdr, a, &format!("{}|match", label));

        // ---- substring accessors on the result ---------------------------
        if a >= 0 || a == err::PARTIAL {
            for gi in 0..6u32 {
                let mut sc: Sz = 0xAAAA;
                let mut sr: Sz = 0xAAAA;
                let x = (p.c.substring_length_bynumber)(mdc, gi, &mut sc);
                let y = (p.r.substring_length_bynumber)(mdr, gi, &mut sr);
                assert_eq!((x, sc), (y, sr), "substring_length_bynumber({}) [{}]", gi, label);
                let mut bc = vec![0xCDu8; 64];
                let mut br = vec![0xCDu8; 64];
                let mut cc: Sz = 48;
                let mut cr: Sz = 48;
                let x = (p.c.substring_copy_bynumber)(mdc, gi, bc.as_mut_ptr(), &mut cc);
                let y = (p.r.substring_copy_bynumber)(mdr, gi, br.as_mut_ptr(), &mut cr);
                assert_eq!((x, cc), (y, cr), "substring_copy_bynumber({}) [{}]", gi, label);
                assert_eq!(bc, br, "substring_copy_bynumber({}) bytes [{}]", gi, label);
                let mut pc: *mut u8 = std::ptr::null_mut();
                let mut pr: *mut u8 = std::ptr::null_mut();
                let mut lc: Sz = 0;
                let mut lr: Sz = 0;
                let x = (p.c.substring_get_bynumber)(mdc, gi, &mut pc, &mut lc);
                let y = (p.r.substring_get_bynumber)(mdr, gi, &mut pr, &mut lr);
                assert_eq!((x, lc), (y, lr), "substring_get_bynumber({}) [{}]", gi, label);
                if x == 0 {
                    assert_eq!(
                        std::slice::from_raw_parts(pc, lc),
                        std::slice::from_raw_parts(pr, lr),
                        "substring_get_bynumber({}) bytes [{}]",
                        gi,
                        label
                    );
                    (p.c.substring_free)(pc);
                    (p.r.substring_free)(pr);
                }
            }
            let mut lc: *mut *mut u8 = std::ptr::null_mut();
            let mut lr: *mut *mut u8 = std::ptr::null_mut();
            let mut ocs: *mut Sz = std::ptr::null_mut();
            let mut ors: *mut Sz = std::ptr::null_mut();
            let x = (p.c.substring_list_get)(mdc, &mut lc, &mut ocs);
            let y = (p.r.substring_list_get)(mdr, &mut lr, &mut ors);
            assert_eq!(x, y, "substring_list_get [{}]", label);
            if x == 0 {
                let mut i = 0;
                loop {
                    let ec = *lc.add(i);
                    let er = *lr.add(i);
                    assert_eq!(ec.is_null(), er.is_null(), "list[{}] null [{}]", i, label);
                    if ec.is_null() {
                        break;
                    }
                    let nc = *ocs.add(i);
                    let nr = *ors.add(i);
                    assert_eq!(nc, nr, "list[{}] len [{}]", i, label);
                    assert_eq!(
                        std::slice::from_raw_parts(ec, nc),
                        std::slice::from_raw_parts(er, nr),
                        "list[{}] bytes [{}]",
                        i,
                        label
                    );
                    i += 1;
                }
                (p.c.substring_list_free)(lc);
                (p.r.substring_list_free)(lr);
            }
        }

        // ---- pcre2_jit_match (no JIT in this build: must agree anyway) ----
        let a = (p.c.jit_match)(cp.c, sp, slen, start, mopts, mdc, mctx_c);
        let b = (p.r.jit_match)(cp.r, sp, slen, start, mopts, mdr, mctx_r);
        assert_eq!(a, b, "pcre2_jit_match rc [{}]", label);

        // ---- pcre2_dfa_match --------------------------------------------
        for extra in [0u32, o::DFA_SHORTEST] {
            for wsn in [20usize, 64, 1000] {
                let mut ws = vec![0i32; wsn];
                let a = (p.c.dfa_match)(
                    cp.c, sp, slen, start, mopts | extra, mdc, mctx_c,
                    ws.as_mut_ptr(), wsn,
                );
                let mut ws2 = vec![0i32; wsn];
                let b = (p.r.dfa_match)(
                    cp.r, sp, slen, start, mopts | extra, mdr, mctx_r,
                    ws2.as_mut_ptr(), wsn,
                );
                assert_eq!(
                    a, b,
                    "pcre2_dfa_match rc [{}|extra={:#x}|ws={}]",
                    label, extra, wsn
                );
                cmp_match_state(p, mdc, mdr, a, &format!("{}|dfa{:#x}|{}", label, extra, wsn));
                // Restarting from a partial match must behave identically.
                if a == err::PARTIAL {
                    let x = (p.c.dfa_match)(
                        cp.c, sp, slen, start, mopts | extra | o::DFA_RESTART, mdc,
                        mctx_c, ws.as_mut_ptr(), wsn,
                    );
                    let y = (p.r.dfa_match)(
                        cp.r, sp, slen, start, mopts | extra | o::DFA_RESTART, mdr,
                        mctx_r, ws2.as_mut_ptr(), wsn,
                    );
                    assert_eq!(x, y, "dfa restart rc [{}]", label);
                }
            }
        }

        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
        (p.c.match_context_free)(mctx_c);
        (p.r.match_context_free)(mctx_r);
    }
}

/// Compile every pattern under `cfg` and run the full match matrix.
fn drive(cfg: &Cfg, seed: u64, subjects_per_pattern: usize, mopts: &[u32]) {
    let p = libs();
    let ctxs = make_contexts(p, cfg);
    let mut rng = Rng::new(seed);
    let alphabet: &[u8] = b"aAbBcCxXyYzZ019 \t\n\r\x00.=;@_-+/\\[]{}()|*?\xC3\xA9\xE2\x82\xAC\x80\xFF";

    for pat in PATTERNS {
        let label0 = format!("{}|{:?}", cfg.label, String::from_utf8_lossy(pat));
        if std::env::var_os("PCRE2_TRACE").is_some() {
            eprintln!("[trace] {}", label0);
        }
        let cp = match compile_both(p, pat, pat.len(), cfg.opts, ctxs.cc, ctxs.cr, &label0) {
            Ok(cp) => cp,
            Err(_) => continue,
        };
        cmp_all_pattern_info(p, &cp, &label0);
        cmp_compiled_bytes(p, &cp, &label0);

        // Also verify code_copy / code_copy_with_tables reproduce the same bytes.
        unsafe {
            let kc = (p.c.code_copy)(cp.c);
            let kr = (p.r.code_copy)(cp.r);
            assert_eq!(kc.is_null(), kr.is_null(), "code_copy null [{}]", label0);
            if !kc.is_null() {
                let kp = CodePair { c: kc, r: kr };
                cmp_compiled_bytes(p, &kp, &format!("{}|copy", label0));
                free_code_pair(p, kp);
            }
            let kc = (p.c.code_copy_with_tables)(cp.c);
            let kr = (p.r.code_copy_with_tables)(cp.r);
            assert_eq!(kc.is_null(), kr.is_null(), "code_copy_with_tables null [{}]", label0);
            if !kc.is_null() {
                let kp = CodePair { c: kc, r: kr };
                cmp_compiled_bytes(p, &kp, &format!("{}|copytab", label0));
                free_code_pair(p, kp);
            }
        }

        for &mo in mopts {
            for i in 0..subjects_per_pattern {
                // Half the subjects come from the curated seeds, half are random.
                let subj: Vec<u8> = if i % 2 == 0 {
                    rng.pick(SUBJECT_SEEDS).to_vec()
                } else {
                    let n = rng.below(18);
                    (0..n).map(|_| *rng.pick(alphabet)).collect()
                };
                let use_zt = rng.bool() && !subj.contains(&0);
                let slen = if use_zt { PCRE2_ZERO_TERMINATED } else { subj.len() };
                let start = if subj.is_empty() { 0 } else { rng.below(subj.len() + 1) };
                let ovn = match rng.below(4) {
                    0 => None,
                    1 => Some(0),
                    2 => Some(1),
                    _ => Some(8),
                };
                let label = format!(
                    "{}|mo={:#x}|subj={:02x?}|slen={}|start={}|ovn={:?}",
                    label0, mo, subj, slen as i64, start, ovn
                );
                if std::env::var_os("PCRE2_TRACE").is_some() {
                    eprintln!("[trace]   {}", label);
                }
                run_subject(p, &cp, &subj, slen, start, mo, ovn, &label);
            }
        }
        free_code_pair(p, cp);
    }
    free_contexts(p, ctxs);
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows
// ---------------------------------------------------------------------------

#[test]
fn cfg_default() {
    drive(&Cfg::new("default"), 1, 6, MATCH_OPTS);
}

#[test]
fn cfg_each_compile_option_bit_alone() {
    // Every bit of the pcre2_compile options word, including the undefined ones.
    for bit in 0..32u32 {
        let mut cfg = Cfg::new("optbit");
        cfg.opts = 1u32 << bit;
        drive(&cfg, 100 + bit as u64, 2, &[0, o::NOTBOL, o::ANCHORED, o::PARTIAL_HARD]);
    }
}

#[test]
fn cfg_each_extra_option_bit_alone() {
    for bit in 0..32u32 {
        let mut cfg = Cfg::new("xoptbit");
        cfg.xopts = 1u32 << bit;
        drive(&cfg, 200 + bit as u64, 2, &[0, o::NOTEOL, o::ENDANCHORED]);
    }
}

#[test]
fn cfg_utf_and_ucp_matrix() {
    for (label, opts) in [
        ("utf", o::UTF),
        ("ucp", o::UCP),
        ("utf+ucp", o::UTF | o::UCP),
        ("utf+caseless", o::UTF | o::CASELESS),
        ("ucp+caseless", o::UCP | o::CASELESS),
        ("utf+ucp+caseless", o::UTF | o::UCP | o::CASELESS),
        ("utf+invalidutf", o::UTF | o::MATCH_INVALID_UTF),
        ("utf+invalidutf+ucp", o::UTF | o::MATCH_INVALID_UTF | o::UCP),
    ] {
        let mut cfg = Cfg::new("utfmatrix");
        cfg.label = label;
        cfg.opts = opts;
        drive(&cfg, 300, 4, MATCH_OPTS);
    }
    // Caseless restrict / ASCII-restriction extra options interact with UTF/UCP.
    for x in [
        o::X_CASELESS_RESTRICT,
        o::X_ASCII_BSD,
        o::X_ASCII_BSS,
        o::X_ASCII_BSW,
        o::X_ASCII_POSIX,
        o::X_ASCII_DIGIT,
        o::X_ASCII_BSD | o::X_ASCII_BSS | o::X_ASCII_BSW | o::X_ASCII_POSIX | o::X_ASCII_DIGIT,
        o::X_TURKISH_CASING,
    ] {
        for opts in [o::UTF, o::UCP, o::UTF | o::UCP, o::UTF | o::CASELESS, o::UTF | o::UCP | o::CASELESS] {
            let mut cfg = Cfg::new("utf+xopt");
            cfg.opts = opts;
            cfg.xopts = x;
            drive(&cfg, 310, 2, &[0, o::NO_UTF_CHECK, o::PARTIAL_SOFT]);
        }
    }
}

#[test]
fn cfg_newline_and_bsr_matrix() {
    for nl in [
        o::NEWLINE_CR,
        o::NEWLINE_LF,
        o::NEWLINE_CRLF,
        o::NEWLINE_ANY,
        o::NEWLINE_ANYCRLF,
        o::NEWLINE_NUL,
    ] {
        for bsr in [o::BSR_UNICODE, o::BSR_ANYCRLF] {
            for opts in [0, o::MULTILINE, o::DOLLAR_ENDONLY, o::MULTILINE | o::DOLLAR_ENDONLY, o::FIRSTLINE, o::ALT_CIRCUMFLEX | o::MULTILINE] {
                let mut cfg = Cfg::new("nl+bsr");
                cfg.newline = nl;
                cfg.bsr = bsr;
                cfg.opts = opts;
                drive(&cfg, 400 + (nl * 10 + bsr) as u64, 3, &[0, o::NOTBOL, o::NOTEOL, o::NOTBOL | o::NOTEOL, o::PARTIAL_HARD]);
            }
        }
    }
}

#[test]
fn cfg_optimize_directives() {
    for dirs in [
        &[o::OPTIMIZATION_NONE][..],
        &[o::OPTIMIZATION_FULL][..],
        &[o::AUTO_POSSESS_OFF][..],
        &[o::AUTO_POSSESS][..],
        &[o::DOTSTAR_ANCHOR_OFF][..],
        &[o::DOTSTAR_ANCHOR][..],
        &[o::START_OPTIMIZE_OFF][..],
        &[o::START_OPTIMIZE][..],
        &[o::OPTIMIZATION_NONE, o::AUTO_POSSESS][..],
        &[o::OPTIMIZATION_FULL, o::AUTO_POSSESS_OFF, o::START_OPTIMIZE_OFF][..],
        &[o::OPTIMIZATION_NONE, o::START_OPTIMIZE, o::DOTSTAR_ANCHOR][..],
    ] {
        let mut cfg = Cfg::new("optimize");
        cfg.optimize = dirs;
        drive(&cfg, 500, 3, MATCH_OPTS);
    }
    // The equivalent compile options must behave the same way.
    for opts in [
        o::NO_AUTO_POSSESS,
        o::NO_DOTSTAR_ANCHOR,
        o::NO_START_OPTIMIZE,
        o::NO_AUTO_POSSESS | o::NO_DOTSTAR_ANCHOR | o::NO_START_OPTIMIZE,
    ] {
        let mut cfg = Cfg::new("nooptimize-opts");
        cfg.opts = opts;
        drive(&cfg, 510, 3, MATCH_OPTS);
    }
}

#[test]
fn cfg_custom_character_tables() {
    let mut cfg = Cfg::new("maketables");
    cfg.custom_tables = true;
    drive(&cfg, 600, 4, MATCH_OPTS);
    let mut cfg = Cfg::new("maketables+caseless+ucp");
    cfg.custom_tables = true;
    cfg.opts = o::CASELESS | o::UCP;
    drive(&cfg, 610, 3, MATCH_OPTS);
}

#[test]
fn cfg_extended_and_literal_modes() {
    for opts in [
        o::EXTENDED,
        o::EXTENDED_MORE,
        o::EXTENDED | o::EXTENDED_MORE,
        o::LITERAL,
        o::LITERAL | o::CASELESS,
        o::LITERAL | o::NO_START_OPTIMIZE,
        o::LITERAL | o::ANCHORED | o::ENDANCHORED,
        o::ALT_BSUX,
        o::ALT_VERBNAMES,
        o::ALT_CIRCUMFLEX,
        o::ALT_EXTENDED_CLASS,
        o::ALLOW_EMPTY_CLASS,
        o::AUTO_CALLOUT,
        o::NO_AUTO_CAPTURE,
        o::DUPNAMES,
        o::UNGREEDY,
        o::MATCH_UNSET_BACKREF,
        o::MATCH_UNSET_BACKREF | o::DUPNAMES,
        o::USE_OFFSET_LIMIT,
    ] {
        let mut cfg = Cfg::new("mode");
        cfg.opts = opts;
        drive(&cfg, 700, 3, MATCH_OPTS);
    }
    // ALT_BSUX plus its extra-option variant.
    let mut cfg = Cfg::new("altbsux");
    cfg.opts = o::ALT_BSUX;
    cfg.xopts = o::X_ALT_BSUX;
    drive(&cfg, 710, 3, MATCH_OPTS);
}

#[test]
fn cfg_max_varlookbehind_values() {
    for v in [0u32, 1, 2, 3, 255, 65535] {
        let mut cfg = Cfg::new("varlb");
        cfg.max_varlookbehind = Some(v);
        drive(&cfg, 800 + v as u64, 2, &[0, o::ANCHORED, o::PARTIAL_HARD]);
    }
}

#[test]
fn cfg_offset_limit_and_match_context_limits() {
    let p = libs();
    let pats: &[&[u8]] = &[b"a", b"a+", b"(a)(b)?", b".*b", b"\\bx\\b", b"(?:a|bb)+"];
    for lim in [0usize, 1, 2, 3, 5, 10, PCRE2_UNSET] {
        for compile_opts in [o::USE_OFFSET_LIMIT, 0] {
            for pat in pats {
                let cp = match compile_both(p, pat, pat.len(), compile_opts, std::ptr::null_mut(), std::ptr::null_mut(), "olim")
                {
                    Ok(cp) => cp,
                    Err(_) => continue,
                };
                unsafe {
                    let mc = (p.c.match_context_create)(std::ptr::null_mut());
                    let mr = (p.r.match_context_create)(std::ptr::null_mut());
                    assert_eq!((p.c.set_offset_limit)(mc, lim), (p.r.set_offset_limit)(mr, lim));
                    for subj in [&b"xxaxxbxx"[..], &b"ab"[..], &b""[..], &b"bbbbbbbbbb"[..]] {
                        let mdc = (p.c.match_data_create_from_pattern)(cp.c, std::ptr::null_mut());
                        let mdr = (p.r.match_data_create_from_pattern)(cp.r, std::ptr::null_mut());
                        let sp = if subj.is_empty() { std::ptr::null() } else { subj.as_ptr() };
                        let a = (p.c.pcre2_match)(cp.c, sp, subj.len(), 0, 0, mdc, mc);
                        let b = (p.r.pcre2_match)(cp.r, sp, subj.len(), 0, 0, mdr, mr);
                        let label = format!(
                            "olim={} copts={:#x} pat={:?} subj={:?}",
                            lim as i64,
                            compile_opts,
                            String::from_utf8_lossy(pat),
                            String::from_utf8_lossy(subj)
                        );
                        assert_eq!(a, b, "match rc [{}]", label);
                        cmp_match_state(p, mdc, mdr, a, &label);
                        let mut ws = [0i32; 256];
                        let mut ws2 = [0i32; 256];
                        let a = (p.c.dfa_match)(cp.c, sp, subj.len(), 0, 0, mdc, mc, ws.as_mut_ptr(), 256);
                        let b = (p.r.dfa_match)(cp.r, sp, subj.len(), 0, 0, mdr, mr, ws2.as_mut_ptr(), 256);
                        assert_eq!(a, b, "dfa rc [{}]", label);
                        cmp_match_state(p, mdc, mdr, a, &format!("{}|dfa", label));
                        (p.c.match_data_free)(mdc);
                        (p.r.match_data_free)(mdr);
                    }
                    (p.c.match_context_free)(mc);
                    (p.r.match_context_free)(mr);
                }
                free_code_pair(p, cp);
            }
        }
    }
}

#[test]
fn cfg_serialize_roundtrip_then_match() {
    // Compile -> serialize -> decode -> match must be identical, and the blob
    // produced by one library must decode in the other.
    let p = libs();
    let mut rng = Rng::new(900);
    for pat in PATTERNS {
        for opts in [0u32, o::UTF, o::CASELESS, o::MULTILINE | o::DOTALL] {
            let cp = match compile_both(p, pat, pat.len(), opts, std::ptr::null_mut(), std::ptr::null_mut(), "ser")
            {
                Ok(cp) => cp,
                Err(_) => continue,
            };
            unsafe {
                let cc = [cp.c];
                let rr = [cp.r];
                let mut bc: *mut u8 = std::ptr::null_mut();
                let mut br: *mut u8 = std::ptr::null_mut();
                let mut lc: Sz = 0;
                let mut lr: Sz = 0;
                let a = (p.c.serialize_encode)(cc.as_ptr(), 1, &mut bc, &mut lc, std::ptr::null_mut());
                let b = (p.r.serialize_encode)(rr.as_ptr(), 1, &mut br, &mut lr, std::ptr::null_mut());
                assert_eq!((a, lc), (b, lr));
                if a < 0 {
                    free_code_pair(p, cp);
                    continue;
                }
                assert_eq!(
                    std::slice::from_raw_parts(bc, lc),
                    std::slice::from_raw_parts(br, lr)
                );
                // Cross-decode.
                let mut oc: [Code; 2] = [std::ptr::null_mut(); 2];
                let mut or: [Code; 2] = [std::ptr::null_mut(); 2];
                let a = (p.c.serialize_decode)(oc.as_mut_ptr(), 1, br, std::ptr::null_mut());
                let b = (p.r.serialize_decode)(or.as_mut_ptr(), 1, bc, std::ptr::null_mut());
                assert_eq!(a, b, "cross decode");
                if a > 0 {
                    let dp = CodePair { c: oc[0], r: or[0] };
                    let label = format!("ser|{:?}|{:#x}", String::from_utf8_lossy(pat), opts);
                    cmp_all_pattern_info(p, &dp, &label);
                    cmp_compiled_bytes(p, &dp, &label);
                    for _ in 0..3 {
                        let subj = rng.pick(SUBJECT_SEEDS).to_vec();
                        run_subject(p, &dp, &subj, subj.len(), 0, 0, None, &label);
                    }
                    free_code_pair(p, dp);
                }
                (p.c.serialize_free)(bc);
                (p.r.serialize_free)(br);
            }
            free_code_pair(p, cp);
        }
    }
}

#[test]
fn cfg_long_and_pathological_subjects() {
    let p = libs();
    let pats: &[(&[u8], u32)] = &[
        (b"a+b", 0),
        (b".*", 0),
        (b".*", o::DOTALL),
        (b"(a|aa)+$", 0),
        (b"\\b\\w+\\b", 0),
        (b"^.*$", o::MULTILINE),
        (b"(?s)^.*$", 0),
        (b"\\R+", 0),
        (b"[^x]*x", 0),
        (b"\\p{L}+", o::UTF | o::UCP),
    ];
    let mut rng = Rng::new(1000);
    for (pat, opts) in pats {
        let cp = match compile_both(p, pat, pat.len(), *opts, std::ptr::null_mut(), std::ptr::null_mut(), "long")
        {
            Ok(cp) => cp,
            Err(_) => continue,
        };
        for n in [0usize, 1, 2, 255, 256, 1000, 5000] {
            for filler in [&b"a"[..], &b"ab"[..], &b"a\n"[..], &b"a\r\n"[..], &b"\xC3\xA9"[..], &b"x"[..]] {
                let mut subj = Vec::new();
                while subj.len() < n {
                    subj.extend_from_slice(filler);
                }
                subj.truncate(n);
                let label = format!(
                    "long|{:?}|{:#x}|n={}|f={:02x?}",
                    String::from_utf8_lossy(pat), opts, n, filler
                );
                let start = if subj.is_empty() { 0 } else { rng.below(subj.len() + 1) };
                run_subject(p, &cp, &subj, subj.len(), start, 0, None, &label);
            }
        }
        free_code_pair(p, cp);
    }
}

#[test]
fn cfg_callout_and_enumerate() {
    // The callout block contents must be identical field by field.
    use std::sync::Mutex;
    static LOG_C: Mutex<Vec<String>> = Mutex::new(Vec::new());
    static LOG_R: Mutex<Vec<String>> = Mutex::new(Vec::new());

    unsafe extern "C" fn cb(b: *mut CalloutBlock, data: *mut c_void) -> i32 {
        let b = unsafe { &*b };
        let s = format!(
            "v={} n={} ct={} cl={} sl={} sm={} cur={} pp={} nil={} cso={} csl={} cs={:?} fl={} mark={:?}",
            b.version,
            b.callout_number,
            b.capture_top,
            b.capture_last,
            b.subject_length,
            b.start_match,
            b.current_position,
            b.pattern_position,
            b.next_item_length,
            b.callout_string_offset,
            b.callout_string_length,
            if b.callout_string.is_null() {
                None
            } else {
                Some(unsafe {
                    std::slice::from_raw_parts(b.callout_string, b.callout_string_length).to_vec()
                })
            },
            b.callout_flags,
            if b.mark.is_null() { None } else { Some(unsafe { *b.mark }) },
        );
        let which = data as usize;
        if which == 0 { LOG_C.lock().unwrap().push(s) } else { LOG_R.lock().unwrap().push(s) }
        0
    }
    unsafe extern "C" fn cb_fail(_b: *mut CalloutBlock, _d: *mut c_void) -> i32 {
        1
    }
    unsafe extern "C" fn cb_abort(_b: *mut CalloutBlock, _d: *mut c_void) -> i32 {
        -99
    }
    unsafe extern "C" fn enum_cb(b: *mut CalloutEnumBlock, data: *mut c_void) -> i32 {
        let b = unsafe { &*b };
        let s = format!(
            "v={} pp={} nil={} n={} cso={} csl={} cs={:?}",
            b.version,
            b.pattern_position,
            b.next_item_length,
            b.callout_number,
            b.callout_string_offset,
            b.callout_string_length,
            if b.callout_string.is_null() {
                None
            } else {
                Some(unsafe {
                    std::slice::from_raw_parts(b.callout_string, b.callout_string_length).to_vec()
                })
            },
        );
        let which = data as usize;
        if which == 0 { LOG_C.lock().unwrap().push(s) } else { LOG_R.lock().unwrap().push(s) }
        0
    }

    let p = libs();
    let pats: &[&[u8]] = &[
        b"a(?C1)b",
        b"a(?C)b(?C2)c",
        b"(?C{str})a",
        b"a(?C`x`)b",
        b"(a)(?C3)(b)",
        b"a(?C1)b|c(?C2)d",
        b"a(*MARK:m)(?C1)b",
        b"(?:a(?C1))+",
    ];
    for &auto in &[0u32, o::AUTO_CALLOUT] {
        for pat in pats {
            let cp = match compile_both(p, pat, pat.len(), auto, std::ptr::null_mut(), std::ptr::null_mut(), "callout")
            {
                Ok(cp) => cp,
                Err(_) => continue,
            };
            let label = format!("callout|{:?}|auto={:#x}", String::from_utf8_lossy(pat), auto);
            // enumerate
            LOG_C.lock().unwrap().clear();
            LOG_R.lock().unwrap().clear();
            unsafe {
                let a = (p.c.callout_enumerate)(cp.c, Some(enum_cb), 0usize as *mut c_void);
                let b = (p.r.callout_enumerate)(cp.r, Some(enum_cb), 1usize as *mut c_void);
                assert_eq!(a, b, "callout_enumerate rc [{}]", label);
            }
            assert_eq!(
                *LOG_C.lock().unwrap(),
                *LOG_R.lock().unwrap(),
                "callout_enumerate blocks [{}]",
                label
            );
            // matching callouts
            for subj in [&b"ab"[..], &b"abc"[..], &b"cd"[..], &b"aab"[..], &b""[..], &b"xay"[..]] {
                for (cbn, f) in [
                    ("accept", cb as unsafe extern "C" fn(*mut CalloutBlock, *mut c_void) -> i32),
                    ("fail", cb_fail),
                    ("abort", cb_abort),
                ] {
                    unsafe {
                        let mc = (p.c.match_context_create)(std::ptr::null_mut());
                        let mr = (p.r.match_context_create)(std::ptr::null_mut());
                        assert_eq!(
                            (p.c.set_callout)(mc, Some(f), 0usize as *mut c_void),
                            (p.r.set_callout)(mr, Some(f), 1usize as *mut c_void)
                        );
                        LOG_C.lock().unwrap().clear();
                        LOG_R.lock().unwrap().clear();
                        let mdc = (p.c.match_data_create_from_pattern)(cp.c, std::ptr::null_mut());
                        let mdr = (p.r.match_data_create_from_pattern)(cp.r, std::ptr::null_mut());
                        let sp = if subj.is_empty() { std::ptr::null() } else { subj.as_ptr() };
                        let a = (p.c.pcre2_match)(cp.c, sp, subj.len(), 0, 0, mdc, mc);
                        let b = (p.r.pcre2_match)(cp.r, sp, subj.len(), 0, 0, mdr, mr);
                        let l2 = format!("{}|subj={:?}|cb={}", label, String::from_utf8_lossy(subj), cbn);
                        assert_eq!(a, b, "match rc [{}]", l2);
                        assert_eq!(
                            *LOG_C.lock().unwrap(),
                            *LOG_R.lock().unwrap(),
                            "callout blocks [{}]",
                            l2
                        );
                        cmp_match_state(p, mdc, mdr, a, &l2);
                        // and via the DFA engine
                        LOG_C.lock().unwrap().clear();
                        LOG_R.lock().unwrap().clear();
                        let mut ws = [0i32; 256];
                        let mut ws2 = [0i32; 256];
                        let a = (p.c.dfa_match)(cp.c, sp, subj.len(), 0, 0, mdc, mc, ws.as_mut_ptr(), 256);
                        let b = (p.r.dfa_match)(cp.r, sp, subj.len(), 0, 0, mdr, mr, ws2.as_mut_ptr(), 256);
                        assert_eq!(a, b, "dfa rc [{}]", l2);
                        assert_eq!(
                            *LOG_C.lock().unwrap(),
                            *LOG_R.lock().unwrap(),
                            "dfa callout blocks [{}]",
                            l2
                        );
                        (p.c.match_data_free)(mdc);
                        (p.r.match_data_free)(mdr);
                        (p.c.match_context_free)(mc);
                        (p.r.match_context_free)(mr);
                    }
                }
            }
            free_code_pair(p, cp);
        }
    }
}

#[test]
fn cfg_custom_allocator_paths() {
    // A counting allocator: the two libraries must request the same total number
    // of allocations and the same sizes, in the same order.
    use std::sync::Mutex;
    static SIZES_C: Mutex<Vec<usize>> = Mutex::new(Vec::new());
    static SIZES_R: Mutex<Vec<usize>> = Mutex::new(Vec::new());

    unsafe extern "C" fn my_malloc(n: usize, d: *mut c_void) -> *mut c_void {
        unsafe extern "C" {
            fn malloc(n: usize) -> *mut c_void;
        }
        if d as usize == 0 {
            SIZES_C.lock().unwrap().push(n)
        } else {
            SIZES_R.lock().unwrap().push(n)
        }
        unsafe { malloc(n) }
    }
    unsafe extern "C" fn my_free(p: *mut c_void, _d: *mut c_void) {
        unsafe extern "C" {
            fn free(p: *mut c_void);
        }
        unsafe { free(p) }
    }

    let p = libs();
    for pat in PATTERNS.iter().take(80) {
        SIZES_C.lock().unwrap().clear();
        SIZES_R.lock().unwrap().clear();
        unsafe {
            let gc = (p.c.general_context_create)(Some(my_malloc), Some(my_free), 0usize as *mut c_void);
            let gr = (p.r.general_context_create)(Some(my_malloc), Some(my_free), 1usize as *mut c_void);
            assert!(!gc.is_null() && !gr.is_null());
            let cc = (p.c.compile_context_create)(gc);
            let cr = (p.r.compile_context_create)(gr);
            let mut ec = 0;
            let mut eo = 0;
            let mut ec2 = 0;
            let mut eo2 = 0;
            let a = (p.c.compile)(pat.as_ptr(), pat.len(), 0, &mut ec, &mut eo, cc);
            let b = (p.r.compile)(pat.as_ptr(), pat.len(), 0, &mut ec2, &mut eo2, cr);
            assert_eq!((a.is_null(), ec, eo), (b.is_null(), ec2, eo2));
            if !a.is_null() {
                let mdc = (p.c.match_data_create_from_pattern)(a, gc);
                let mdr = (p.r.match_data_create_from_pattern)(b, gr);
                let subj = b"abcabc";
                let x = (p.c.pcre2_match)(a, subj.as_ptr(), 6, 0, 0, mdc, std::ptr::null_mut());
                let y = (p.r.pcre2_match)(b, subj.as_ptr(), 6, 0, 0, mdr, std::ptr::null_mut());
                assert_eq!(x, y);
                (p.c.match_data_free)(mdc);
                (p.r.match_data_free)(mdr);
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
            (p.c.compile_context_free)(cc);
            (p.r.compile_context_free)(cr);
            (p.c.general_context_free)(gc);
            (p.r.general_context_free)(gr);
        }
        assert_eq!(
            *SIZES_C.lock().unwrap(),
            *SIZES_R.lock().unwrap(),
            "allocation sequence for {:?}",
            String::from_utf8_lossy(pat)
        );
    }
}

#[test]
fn cfg_recursion_memory_management_setter() {
    // pcre2_set_recursion_memory_management is a no-op kept for ABI
    // compatibility; verify both accept and ignore it identically.
    unsafe extern "C" fn m(n: usize, _d: *mut c_void) -> *mut c_void {
        unsafe extern "C" {
            fn malloc(n: usize) -> *mut c_void;
        }
        unsafe { malloc(n) }
    }
    unsafe extern "C" fn f(p: *mut c_void, _d: *mut c_void) {
        unsafe extern "C" {
            fn free(p: *mut c_void);
        }
        unsafe { free(p) }
    }
    let p = libs();
    unsafe {
        let mc = (p.c.match_context_create)(std::ptr::null_mut());
        let mr = (p.r.match_context_create)(std::ptr::null_mut());
        assert_eq!(
            (p.c.set_recursion_memory_management)(mc, Some(m), Some(f), std::ptr::null_mut()),
            (p.r.set_recursion_memory_management)(mr, Some(m), Some(f), std::ptr::null_mut())
        );
        assert_eq!(
            (p.c.set_recursion_memory_management)(mc, None, None, std::ptr::null_mut()),
            (p.r.set_recursion_memory_management)(mr, None, None, std::ptr::null_mut())
        );
        // A context copy must still behave identically.
        let mc2 = (p.c.match_context_copy)(mc);
        let mr2 = (p.r.match_context_copy)(mr);
        assert!(!mc2.is_null() && !mr2.is_null());
        let cp = compile_both(p, b"a+", 2, 0, std::ptr::null_mut(), std::ptr::null_mut(), "rmm").unwrap();
        let mdc = (p.c.match_data_create)(4, std::ptr::null_mut());
        let mdr = (p.r.match_data_create)(4, std::ptr::null_mut());
        let a = (p.c.pcre2_match)(cp.c, b"aaa".as_ptr(), 3, 0, 0, mdc, mc2);
        let b = (p.r.pcre2_match)(cp.r, b"aaa".as_ptr(), 3, 0, 0, mdr, mr2);
        assert_eq!(a, b);
        cmp_match_state(p, mdc, mdr, a, "rmm");
        (p.c.match_data_free)(mdc);
        (p.r.match_data_free)(mdr);
        free_code_pair(p, cp);
        (p.c.match_context_free)(mc2);
        (p.r.match_context_free)(mr2);
        (p.c.match_context_free)(mc);
        (p.r.match_context_free)(mr);
    }
}

#[test]
fn cfg_compile_context_copy_is_faithful() {
    let p = libs();
    unsafe {
        let cc = (p.c.compile_context_create)(std::ptr::null_mut());
        let cr = (p.r.compile_context_create)(std::ptr::null_mut());
        (p.c.set_newline)(cc, o::NEWLINE_ANYCRLF);
        (p.r.set_newline)(cr, o::NEWLINE_ANYCRLF);
        (p.c.set_bsr)(cc, o::BSR_ANYCRLF);
        (p.r.set_bsr)(cr, o::BSR_ANYCRLF);
        (p.c.set_compile_extra_options)(cc, o::X_MATCH_WORD);
        (p.r.set_compile_extra_options)(cr, o::X_MATCH_WORD);
        (p.c.set_max_varlookbehind)(cc, 7);
        (p.r.set_max_varlookbehind)(cr, 7);
        (p.c.set_parens_nest_limit)(cc, 30);
        (p.r.set_parens_nest_limit)(cr, 30);
        (p.c.set_optimize)(cc, o::AUTO_POSSESS_OFF);
        (p.r.set_optimize)(cr, o::AUTO_POSSESS_OFF);
        let cc2 = (p.c.compile_context_copy)(cc);
        let cr2 = (p.r.compile_context_copy)(cr);
        assert!(!cc2.is_null() && !cr2.is_null());
        for pat in PATTERNS.iter().take(60) {
            let label = format!("ctxcopy|{:?}", String::from_utf8_lossy(pat));
            if let Ok(cp) = compile_both(p, pat, pat.len(), 0, cc2, cr2, &label) {
                cmp_all_pattern_info(p, &cp, &label);
                cmp_compiled_bytes(p, &cp, &label);
                free_code_pair(p, cp);
            }
        }
        (p.c.compile_context_free)(cc2);
        (p.r.compile_context_free)(cr2);
        (p.c.compile_context_free)(cc);
        (p.r.compile_context_free)(cr);
    }
}

#[test]
fn cfg_match_word_and_match_line_extra_options() {
    for x in [o::X_MATCH_WORD, o::X_MATCH_LINE, o::X_MATCH_WORD | o::X_MATCH_LINE] {
        for opts in [0u32, o::CASELESS, o::MULTILINE, o::UTF] {
            let mut cfg = Cfg::new("matchword/line");
            cfg.xopts = x;
            cfg.opts = opts;
            drive(&cfg, 1100, 3, MATCH_OPTS);
        }
    }
}

#[test]
fn cfg_random_patterns_fuzz() {
    // Randomly generated pattern text: most will fail to compile, and the point
    // is that C and Rust must agree on *which* ones fail and with what error, and
    // that the ones that compile behave identically.
    //
    // PCRE2_FUZZ_ITERS / PCRE2_FUZZ_SEED override the defaults for longer runs.
    let iters: usize = std::env::var("PCRE2_FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(400_000);
    let seed: u64 = std::env::var("PCRE2_FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0xF0F0_1234_5678_9ABC);
    let p = libs();
    let mut rng = Rng::new(seed);
    let bytes: &[u8] = b"ab019()[]{}|*+?.^$\\-,:!=<>&~#'\"`/%@_ \t\n\rPpQEKGXRNCcxouhHvVsSwWdDbBAZz";
    let optsets: &[u32] = &[
        0,
        o::UTF,
        o::UCP,
        o::CASELESS,
        o::EXTENDED,
        o::MULTILINE | o::DOTALL,
        o::ALT_BSUX,
        o::ALT_EXTENDED_CLASS,
        o::ALT_VERBNAMES,
        o::AUTO_CALLOUT,
        o::NO_AUTO_CAPTURE,
        o::DUPNAMES,
        o::UNGREEDY,
        o::LITERAL,
    ];
    let xoptsets: &[u32] = &[
        0,
        o::X_ALT_BSUX,
        o::X_BAD_ESCAPE_IS_LITERAL,
        o::X_ALLOW_SURROGATE_ESCAPES,
        o::X_ESCAPED_CR_IS_LF,
        o::X_PYTHON_OCTAL,
        o::X_NO_BS0,
        o::X_ALLOW_LOOKAROUND_BSK,
        o::X_CASELESS_RESTRICT,
        o::X_MATCH_WORD,
        o::X_MATCH_LINE,
    ];
    let mut compiled = 0usize;
    for _ in 0..iters {
        let n = 1 + rng.below(14);
        let pat: Vec<u8> = (0..n).map(|_| *rng.pick(bytes)).collect();
        let opts = *rng.pick(optsets);
        let xopts = *rng.pick(xoptsets);
        unsafe {
            let cc = (p.c.compile_context_create)(std::ptr::null_mut());
            let cr = (p.r.compile_context_create)(std::ptr::null_mut());
            if xopts != 0 {
                (p.c.set_compile_extra_options)(cc, xopts);
                (p.r.set_compile_extra_options)(cr, xopts);
            }
            let mut ec = 0i32;
            let mut eo = 0usize;
            let mut ec2 = 0i32;
            let mut eo2 = 0usize;
            let a = (p.c.compile)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, cc);
            let b = (p.r.compile)(pat.as_ptr(), pat.len(), opts, &mut ec2, &mut eo2, cr);
            assert_eq!(
                (a.is_null(), ec, eo),
                (b.is_null(), ec2, eo2),
                "fuzz compile {:?} opts={:#x} xopts={:#x}",
                String::from_utf8_lossy(&pat),
                opts,
                xopts
            );
            if !a.is_null() {
                compiled += 1;
                let cp = CodePair { c: a, r: b };
                let label = format!("fuzz|{:?}|{:#x}|{:#x}", String::from_utf8_lossy(&pat), opts, xopts);
                cmp_all_pattern_info(p, &cp, &label);
                cmp_compiled_bytes(p, &cp, &label);
                for _ in 0..2 {
                    let subj = rng.pick(SUBJECT_SEEDS).to_vec();
                    let mo = *rng.pick(MATCH_OPTS);
                    run_subject(p, &cp, &subj, subj.len(), 0, mo, None, &label);
                }
                free_code_pair(p, cp);
            }
            (p.c.compile_context_free)(cc);
            (p.r.compile_context_free)(cr);
        }
    }
    assert!(
        compiled * 60 > iters,
        "fuzzer only produced {} valid patterns out of {}",
        compiled,
        iters
    );
    eprintln!("fuzz: {} of {} random patterns compiled", compiled, iters);
}
