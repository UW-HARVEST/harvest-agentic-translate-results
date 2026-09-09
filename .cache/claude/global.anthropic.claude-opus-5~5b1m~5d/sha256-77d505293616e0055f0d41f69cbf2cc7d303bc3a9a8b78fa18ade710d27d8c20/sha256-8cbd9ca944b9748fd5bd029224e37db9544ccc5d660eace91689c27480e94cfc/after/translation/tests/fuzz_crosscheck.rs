//! Cross-cutting randomized differential fuzz, independent of the per-row CONFIGS tests.
//!
//! One fixed-seed generator produces random patterns, random option words, random newline /
//! BSR / limit settings and random subjects, and every case is pushed through the WHOLE
//! pipeline in both libraries: compile -> pattern_info -> serialize -> match -> dfa_match ->
//! next_match -> substitute -> substring extraction. Everything observable is compared.
mod common;
use common::*;
use std::ptr;

const COMPILE_OPTS: &[u32] = &[
    PCRE2_ANCHORED,
    PCRE2_NO_UTF_CHECK,
    PCRE2_ENDANCHORED,
    PCRE2_ALLOW_EMPTY_CLASS,
    PCRE2_ALT_BSUX,
    PCRE2_AUTO_CALLOUT,
    PCRE2_CASELESS,
    PCRE2_DOLLAR_ENDONLY,
    PCRE2_DOTALL,
    PCRE2_DUPNAMES,
    PCRE2_EXTENDED,
    PCRE2_FIRSTLINE,
    PCRE2_MATCH_UNSET_BACKREF,
    PCRE2_MULTILINE,
    PCRE2_NEVER_UCP,
    PCRE2_NEVER_UTF,
    PCRE2_NO_AUTO_CAPTURE,
    PCRE2_NO_AUTO_POSSESS,
    PCRE2_NO_DOTSTAR_ANCHOR,
    PCRE2_NO_START_OPTIMIZE,
    PCRE2_UCP,
    PCRE2_UNGREEDY,
    PCRE2_UTF,
    PCRE2_NEVER_BACKSLASH_C,
    PCRE2_ALT_CIRCUMFLEX,
    PCRE2_ALT_VERBNAMES,
    PCRE2_USE_OFFSET_LIMIT,
    PCRE2_EXTENDED_MORE,
    PCRE2_LITERAL,
    PCRE2_MATCH_INVALID_UTF,
    PCRE2_ALT_EXTENDED_CLASS,
];

const EXTRA_OPTS: &[u32] = &[
    PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
    PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
    PCRE2_EXTRA_MATCH_WORD,
    PCRE2_EXTRA_MATCH_LINE,
    PCRE2_EXTRA_ESCAPED_CR_IS_LF,
    PCRE2_EXTRA_ALT_BSUX,
    PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
    PCRE2_EXTRA_CASELESS_RESTRICT,
    PCRE2_EXTRA_ASCII_BSD,
    PCRE2_EXTRA_ASCII_BSS,
    PCRE2_EXTRA_ASCII_BSW,
    PCRE2_EXTRA_ASCII_POSIX,
    PCRE2_EXTRA_ASCII_DIGIT,
    PCRE2_EXTRA_PYTHON_OCTAL,
    PCRE2_EXTRA_NO_BS0,
    PCRE2_EXTRA_NEVER_CALLOUT,
    PCRE2_EXTRA_TURKISH_CASING,
];

const MATCH_OPTS: &[u32] = &[
    PCRE2_NOTBOL,
    PCRE2_NOTEOL,
    PCRE2_NOTEMPTY,
    PCRE2_NOTEMPTY_ATSTART,
    PCRE2_PARTIAL_SOFT,
    PCRE2_PARTIAL_HARD,
    PCRE2_ANCHORED,
    PCRE2_ENDANCHORED,
    PCRE2_NO_UTF_CHECK,
    PCRE2_COPY_MATCHED_SUBJECT,
    PCRE2_DISABLE_RECURSELOOP_CHECK,
];

/// Pattern fragments: literals, escapes, classes, groups, quantifiers, verbs, callouts.
const FRAGS: &[&str] = &[
    "a", "b", "Z", "9", "_", " ", "\\n", "\\r", "\\x{e9}", "\\x{10437}", "\\xff", "\\0", ".",
    "\\d", "\\D", "\\w", "\\W", "\\s", "\\S", "\\h", "\\v", "\\R", "\\X", "\\N", "\\b", "\\B",
    "\\A", "\\Z", "\\z", "\\G", "\\K", "\\C", "\\Qa.b\\E", "\\p{L}", "\\p{Greek}", "\\P{Nd}",
    "\\p{Any}", "\\p{Lu}", "[a-z]", "[^a-z]", "[[:alpha:]]", "[[:^digit:]]", "[\\p{L}\\d]",
    "[]a]", "[^]]", "[\\x{100}-\\x{200}]", "[a-\\x{ff}]", "()", "(a)", "(?:a)", "(?|(a)|(b))",
    "(?<n>a)", "(?'m'b)", "(?P<p>c)", "(?>a+)", "(?=a)", "(?!a)", "(?<=ab)", "(?<!ab)",
    "(?#note)", "(*atomic:a)", "(*script_run:\\p{Greek}+)", "(*sr:a)", "(*plb:ab)", "(*naplb:a)",
    "a*", "a+", "a?", "a*?", "a+?", "a??", "a*+", "a++", "a?+", "a{0,3}", "a{2,}", "a{3}",
    "a{0,0}", "a{1,2}?", "a|b", "|", "\\1", "\\g1", "\\g{-1}", "\\g{+1}", "\\k<n>", "(?P=n)",
    "(?R)", "(?1)", "(?&n)", "(?+1)", "(?-1)", "(?(1)a|b)", "(?(<n>)a)", "(?(R)a)",
    "(?(DEFINE)(?<x>a))", "(?(VERSION>=10.0)a|b)", "(*MARK:m)", "(*SKIP)", "(*SKIP:s)",
    "(*THEN)", "(*PRUNE)", "(*COMMIT)", "(*ACCEPT)", "(*FAIL)", "(*F)", "(*:m)", "(?C)", "(?C1)",
    "(?C255)", "(?C\"str\")", "(?i)", "(?-i)", "(?x)", "(?xx)", "(?s)", "(?m)", "(?J)", "(?U)",
    "(?n)", "(?aT)", "(?^i)", "(?i:a)", "\\x{d800}", "\\o{101}", "\\N{U+41}", "\\c@", "\\e",
    "\\a", "\\f", "\\777", "\\8", "^", "$", "-", "]", "}", "{", "*", "+", "?",
    "[[a-z]&&[^q]]", "(?[[a]|[b]])", "(*scs:(1)a)", "(*LIMIT_MATCH=10)", "(*LIMIT_DEPTH=10)",
    "(*LIMIT_HEAP=100)", "(*CR)", "(*LF)", "(*CRLF)", "(*ANY)", "(*ANYCRLF)", "(*NUL)",
    "(*BSR_ANYCRLF)", "(*BSR_UNICODE)", "(*UTF)", "(*UCP)", "(*NO_AUTO_POSSESS)",
    "(*NO_START_OPT)", "(*NOTEMPTY)", "(*NOTEMPTY_ATSTART)", "(*NO_DOTSTAR_ANCHOR)",
    "(*NO_JIT)",
];

const REPL_FRAGS: &[&str] = &[
    "x", "$0", "$1", "$2", "${1}", "$name", "${name}", "$$", "$&", "\\$", "\\\\", "\\n", "\\U",
    "\\L", "\\u", "\\l", "\\E", "${name:-def}", "${1:+a:b}", "${1:-$2}", "${", "$", "$99", "",
    "\\x{e9}", "\\a",
];

fn hex(v: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in v {
        if b >= 0x20 && b < 0x7f && b != b'"' && b != b'\\' {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{:02x}", b));
        }
    }
    s.push('"');
    s
}

fn gen_pattern(rng: &mut Rng) -> Vec<u8> {
    let n = 1 + rng.below(12);
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(rng.pick(FRAGS));
    }
    s.into_bytes()
}

fn gen_subject(rng: &mut Rng) -> Vec<u8> {
    let len = if rng.below(4) == 0 { rng.below(64) as usize } else { rng.below(24) as usize };
    let mut v = Vec::with_capacity(len + 4);
    while v.len() < len {
        match rng.below(10) {
            0..=3 => v.push(*rng.pick(b"abcABC019 _-.")),
            4 => v.push(rng.byte()),
            5 => v.extend_from_slice(*rng.pick(&[
                &b"\n"[..],
                &b"\r"[..],
                &b"\r\n"[..],
                &b"\xc2\x85"[..],
                &b"\xe2\x80\xa8"[..],
                &b"\xe2\x80\xa9"[..],
            ])),
            6 => v.extend_from_slice(*rng.pick(&[
                &b"\xc3\xa9"[..],
                &b"\xce\xb1"[..],
                &b"\xe4\xb8\x80"[..],
                &b"\xf0\x90\x90\xb7"[..],
            ])),
            7 => v.extend_from_slice(*rng.pick(&[
                &b"\x80"[..],
                &b"\xc3"[..],
                &b"\xed\xa0\x80"[..],
                &b"\xf5\x80\x80\x80"[..],
                &b"\xfe"[..],
            ])),
            8 => v.push(0),
            _ => v.push(*rng.pick(b"$^*+?|()[]{}\\/")),
        }
    }
    v
}

fn rand_opts(rng: &mut Rng, pool: &[u32]) -> u32 {
    let mut o = 0;
    let k = rng.below(4);
    for _ in 0..k {
        o |= *rng.pick(pool);
    }
    // occasionally throw in an undefined bit
    if rng.below(32) == 0 {
        o |= 1 << rng.below(32);
    }
    o
}

struct Case {
    pat: Vec<u8>,
    opts: u32,
    xopts: u32,
    newline: u32,
    bsr: u32,
    varlb: u32,
    subj: Vec<u8>,
    start: usize,
    mopts: u32,
    mlimit: u32,
    dlimit: u32,
    hlimit: u32,
    olimit: usize,
    repl: Vec<u8>,
    sopts: u32,
    ovec: u32,
}

fn gen_case(rng: &mut Rng) -> Case {
    let subj = gen_subject(rng);
    let start = if subj.is_empty() {
        0
    } else {
        rng.below(subj.len() as u32 + 2) as usize
    };
    let mut repl = String::new();
    for _ in 0..rng.below(4) {
        repl.push_str(rng.pick(REPL_FRAGS));
    }
    // PCRE2_NO_UTF_CHECK on a subject that is NOT valid UTF-8, or with a start offset that
    // is not on a character boundary, is DOCUMENTED UNDEFINED BEHAVIOUR in PCRE2 (the C
    // library itself crashes), so it must never be generated for such a subject.
    let utf_safe = std::str::from_utf8(&subj)
        .map(|s| s.is_char_boundary(start.min(s.len())))
        .unwrap_or(false)
        && start <= subj.len();
    let mut c = Case {
        pat: gen_pattern(rng),
        opts: rand_opts(rng, COMPILE_OPTS),
        xopts: if rng.below(3) == 0 {
            rand_opts(rng, EXTRA_OPTS)
        } else {
            0
        },
        newline: *rng.pick(&[0, 1, 2, 3, 4, 5, 6, 7, 99]),
        bsr: *rng.pick(&[0, 1, 2, 3]),
        varlb: *rng.pick(&[0, 1, 2, 255, 65535]),
        subj,
        start,
        mopts: rand_opts(rng, MATCH_OPTS),
        mlimit: *rng.pick(&[0, 1, 5, 40, 1000, 10_000_000]),
        dlimit: *rng.pick(&[0, 1, 5, 40, 1000, 10_000_000]),
        hlimit: *rng.pick(&[0, 1, 20, 1000, 20_000_000]),
        olimit: *rng.pick(&[0, 1, 3, usize::MAX]),
        repl: repl.into_bytes(),
        sopts: rand_opts(
            rng,
            &[
                PCRE2_SUBSTITUTE_GLOBAL,
                PCRE2_SUBSTITUTE_EXTENDED,
                PCRE2_SUBSTITUTE_UNSET_EMPTY,
                PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                PCRE2_SUBSTITUTE_LITERAL,
                PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                PCRE2_NOTBOL,
                PCRE2_NOTEOL,
                PCRE2_NOTEMPTY,
                PCRE2_ANCHORED,
            ],
        ),
        ovec: *rng.pick(&[0, 1, 2, 3, 8, 40]),
    };
    if !utf_safe {
        c.mopts &= !PCRE2_NO_UTF_CHECK;
        c.sopts &= !PCRE2_NO_UTF_CHECK;
    }
    c
}

/// Everything one library observes for one case.
#[derive(Debug, PartialEq, Eq)]
struct Obs {
    compile: CompileOut,
    m: Option<MatchOut>,
    m_frames: usize,
    dfa: Option<MatchOut>,
    dfa_ws_small: i32,
    next: Vec<(i32, usize, u32, Vec<usize>)>,
    subst: (i32, usize, Vec<u8>),
    subst_small: (i32, usize, Vec<u8>),
    substrings: Vec<(i32, Vec<u8>)>,
    info_ptr_contents: Vec<u8>,
}

macro_rules! stage {
    ($api:expr, $($t:tt)*) => {
        if std::env::var_os("FUZZ_TRACE").is_some() {
            eprintln!("    [{}] {}", $api.tag, format!($($t)*));
        }
    };
}

unsafe fn observe(api: &Api, cs: &Case) -> Obs {
    let cc = (api.pcre2_compile_context_create_8)(ptr::null_mut());
    if cs.xopts != 0 {
        (api.pcre2_set_compile_extra_options_8)(cc, cs.xopts);
    }
    if cs.newline != 0 {
        (api.pcre2_set_newline_8)(cc, cs.newline);
    }
    if cs.bsr != 0 {
        (api.pcre2_set_bsr_8)(cc, cs.bsr);
    }
    (api.pcre2_set_max_varlookbehind_8)(cc, cs.varlb);

    stage!(api, "compile_probe");
    let compile = api.compile_probe(&cs.pat, cs.pat.len(), cs.opts, cc);

    // Recompile for the matching stages (compile_probe frees its code object).
    stage!(api, "recompile");
    let mut ec = 0;
    let mut eo = 0;
    let code = (api.pcre2_compile_8)(cs.pat.as_ptr(), cs.pat.len(), cs.opts, &mut ec, &mut eo, cc);
    let mut obs = Obs {
        compile,
        m: None,
        m_frames: 0,
        dfa: None,
        dfa_ws_small: 0,
        next: vec![],
        subst: (0, 0, vec![]),
        subst_small: (0, 0, vec![]),
        substrings: vec![],
        info_ptr_contents: vec![],
    };
    if code.is_null() {
        (api.pcre2_compile_context_free_8)(cc);
        return obs;
    }

    stage!(api, "info-pointers");
    // name table + start bitmap contents (pointer-valued info keys)
    let mut ncount: u32 = 0;
    let mut nsize: u32 = 0;
    (api.pcre2_pattern_info_8)(code, PCRE2_INFO_NAMECOUNT, &mut ncount as *mut u32 as Ptr);
    (api.pcre2_pattern_info_8)(code, PCRE2_INFO_NAMEENTRYSIZE, &mut nsize as *mut u32 as Ptr);
    let mut ntab: *const u8 = ptr::null();
    if (api.pcre2_pattern_info_8)(code, PCRE2_INFO_NAMETABLE, &mut ntab as *mut *const u8 as Ptr)
        == 0
        && !ntab.is_null()
    {
        obs.info_ptr_contents
            .extend_from_slice(std::slice::from_raw_parts(ntab, (ncount * nsize) as usize));
    }

    stage!(api, "match-context");
    let mc = (api.pcre2_match_context_create_8)(ptr::null_mut());
    (api.pcre2_set_match_limit_8)(mc, cs.mlimit);
    (api.pcre2_set_depth_limit_8)(mc, cs.dlimit);
    (api.pcre2_set_heap_limit_8)(mc, cs.hlimit);
    (api.pcre2_set_offset_limit_8)(mc, cs.olimit);

    let gz = zeroing_context(api);
    let md = (api.pcre2_match_data_create_8)(cs.ovec, gz);
    let sp = if cs.subj.is_empty() {
        ptr::null()
    } else {
        cs.subj.as_ptr()
    };

    stage!(api, "match");
    // ---- pcre2_match
    let rc = (api.pcre2_match_8)(code, sp, cs.subj.len(), cs.start, cs.mopts, md, mc);
    obs.m = Some(read_match(api, md, rc));
    obs.m_frames = (api.pcre2_get_match_data_heapframes_size_8)(md);

    stage!(api, "substring");
    // ---- substring extraction from whatever the match left behind
    if rc > 0 {
        for gi in 0..(rc as u32 + 2) {
            let mut len: usize = 0;
            let lrc = (api.pcre2_substring_length_bynumber_8)(md, gi, &mut len);
            let mut buf = vec![0xAAu8; 64];
            let mut blen = buf.len();
            let crc = (api.pcre2_substring_copy_bynumber_8)(md, gi, buf.as_mut_ptr(), &mut blen);
            obs.substrings.push((lrc * 1000 + crc, buf));
            obs.substrings.push((len as i32, blen.to_ne_bytes().to_vec()));
        }
    }

    stage!(api, "next_match");
    // ---- pcre2_next_match iteration (bounded)
    {
        let md2 = (api.pcre2_match_data_create_8)(cs.ovec.max(1), gz);
        let mut off = cs.start;
        let mut o = cs.mopts;
        let r0 = (api.pcre2_match_8)(code, sp, cs.subj.len(), off, o, md2, mc);
        let n = (api.pcre2_get_ovector_count_8)(md2);
        let ov = (api.pcre2_get_ovector_pointer_8)(md2);
        obs.next.push((
            r0,
            off,
            o,
            std::slice::from_raw_parts(ov, (n * 2) as usize).to_vec(),
        ));
        let mut guard = 0;
        while r0 > 0 && guard < 8 {
            let nr = (api.pcre2_next_match_8)(md2, &mut off, &mut o);
            let n = (api.pcre2_get_ovector_count_8)(md2);
            let ov = (api.pcre2_get_ovector_pointer_8)(md2);
            obs.next.push((
                nr,
                off,
                o,
                std::slice::from_raw_parts(ov, (n * 2) as usize).to_vec(),
            ));
            if nr < 0 {
                break;
            }
            let r = (api.pcre2_match_8)(code, sp, cs.subj.len(), off, o, md2, mc);
            let n = (api.pcre2_get_ovector_count_8)(md2);
            let ov = (api.pcre2_get_ovector_pointer_8)(md2);
            obs.next.push((
                r,
                off,
                o,
                std::slice::from_raw_parts(ov, (n * 2) as usize).to_vec(),
            ));
            if r < 0 {
                break;
            }
            guard += 1;
        }
        (api.pcre2_match_data_free_8)(md2);
    }

    stage!(api, "dfa");
    // ---- pcre2_dfa_match
    {
        let md3 = (api.pcre2_match_data_create_8)(cs.ovec.max(1), gz);
        let mut ws = vec![0i32; 200];
        let drc = (api.pcre2_dfa_match_8)(
            code,
            sp,
            cs.subj.len(),
            cs.start,
            cs.mopts & !(PCRE2_COPY_MATCHED_SUBJECT | PCRE2_DISABLE_RECURSELOOP_CHECK),
            md3,
            mc,
            ws.as_mut_ptr(),
            ws.len(),
        );
        obs.dfa = Some(read_match(api, md3, drc));
        let mut ws2 = vec![0i32; 20];
        obs.dfa_ws_small = (api.pcre2_dfa_match_8)(
            code,
            sp,
            cs.subj.len(),
            cs.start,
            cs.mopts & !(PCRE2_COPY_MATCHED_SUBJECT | PCRE2_DISABLE_RECURSELOOP_CHECK),
            md3,
            mc,
            ws2.as_mut_ptr(),
            ws2.len(),
        );
        (api.pcre2_match_data_free_8)(md3);
    }

    stage!(api, "substitute");
    // ---- pcre2_substitute, with a generous and a tight output buffer
    for (slot, cap) in [(0usize, 256usize), (1, 4)] {
        let md4 = (api.pcre2_match_data_create_8)(cs.ovec.max(1), gz);
        let mut out = vec![0xAAu8; cap + 8];
        let mut olen = cap;
        let src = (api.pcre2_substitute_8)(
            code,
            sp,
            cs.subj.len(),
            cs.start,
            cs.sopts,
            md4,
            mc,
            cs.repl.as_ptr(),
            cs.repl.len(),
            out.as_mut_ptr(),
            &mut olen,
        );
        let t = (src, olen, out);
        if slot == 0 {
            obs.subst = t;
        } else {
            obs.subst_small = t;
        }
        (api.pcre2_match_data_free_8)(md4);
    }

    (api.pcre2_match_data_free_8)(md);
    (api.pcre2_general_context_free_8)(gz);
    (api.pcre2_match_context_free_8)(mc);
    (api.pcre2_code_free_8)(code);
    (api.pcre2_compile_context_free_8)(cc);
    obs
}

fn run_fuzz(seed: u64, iters: usize, label: &str) {
    let (c, r) = both();
    let mut rng = Rng::new(seed);
    let mut compiled = 0usize;
    for i in 0..iters {
        let cs = gen_case(&mut rng);
        let trace = std::env::var_os("FUZZ_TRACE").is_some();
        if trace {
            eprintln!(
                "[{} {}] pat={} opts={:#x} x={:#x} nl={} bsr={} vlb={} subj={} start={} mopts={:#x} ovec={} lim={}/{}/{}/{:x} repl={:?} sopts={:#x} -> C",
                label, i, hex(&cs.pat), cs.opts, cs.xopts, cs.newline, cs.bsr,
                cs.varlb, hex(&cs.subj), cs.start, cs.mopts, cs.ovec,
                cs.mlimit, cs.dlimit, cs.hlimit, cs.olimit, hex(&cs.repl), cs.sopts);
        }
        let a = unsafe { observe(c, &cs) };
        if trace {
            eprintln!("[{} {}] C ok -> RUST", label, i);
        }
        let b = unsafe { observe(r, &cs) };
        if trace {
            eprintln!("[{} {}] RUST ok", label, i);
        }
        if a.compile.ok {
            compiled += 1;
        }
        if a != b {
            let which = if a.compile != b.compile {
                "compile"
            } else if a.m != b.m {
                "match"
            } else if a.m_frames != b.m_frames {
                "heapframes_size"
            } else if a.dfa != b.dfa {
                "dfa_match"
            } else if a.dfa_ws_small != b.dfa_ws_small {
                "dfa_match(small workspace)"
            } else if a.next != b.next {
                "next_match"
            } else if a.subst != b.subst {
                "substitute"
            } else if a.subst_small != b.subst_small {
                "substitute(small buffer)"
            } else if a.substrings != b.substrings {
                "substring"
            } else {
                "info pointers"
            };
            panic!(
                "\n{} iteration {} diverged in {}\n  pattern  = {:?}\n  options  = {:#x} extra={:#x} newline={} bsr={} varlb={}\n  subject  = {:?}\n  start    = {} mopts={:#x} ovec={}\n  limits   = match={} depth={} heap={} offset={}\n  repl     = {:?} sopts={:#x}\n  C    = {:?}\n  RUST = {:?}\n",
                label,
                i,
                which,
                String::from_utf8_lossy(&cs.pat),
                cs.opts,
                cs.xopts,
                cs.newline,
                cs.bsr,
                cs.varlb,
                String::from_utf8_lossy(&cs.subj),
                cs.start,
                cs.mopts,
                cs.ovec,
                cs.mlimit,
                cs.dlimit,
                cs.hlimit,
                cs.olimit as i64,
                String::from_utf8_lossy(&cs.repl),
                cs.sopts,
                a,
                b
            );
        }
    }
    println!(
        "{}: {} random cases identical ({} compiled successfully)",
        label, iters, compiled
    );
}

#[test]
fn fuzz_seed_1() {
    run_fuzz(0x1234_5678_9abc_def1, 200000, "seed1");
}
#[test]
fn fuzz_seed_2() {
    run_fuzz(0xdead_beef_cafe_0001, 200000, "seed2");
}
#[test]
fn fuzz_seed_3() {
    run_fuzz(0x0000_0000_0000_0003, 200000, "seed3");
}
#[test]
fn fuzz_seed_4() {
    run_fuzz(0xffff_ffff_ffff_fff5, 200000, "seed4");
}
