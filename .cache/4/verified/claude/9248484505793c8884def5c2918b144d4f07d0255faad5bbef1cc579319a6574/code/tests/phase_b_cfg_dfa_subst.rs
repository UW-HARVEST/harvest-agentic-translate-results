// Phase B sign-off for CONFIGS.md rows 294-351:
//
//   * section 3 — `dfa_match`  / `pcre2_dfa_match_8`   (rows 294-321)
//   * section 4 — `substitute` / `pcre2_substitute_8`  (rows 322-351)
//
// Every row sets up its exact named configuration, calls the entry point in
// BOTH shared libraries and compares every observable the C defines for that
// return code.  `read_match_out_of(.., Engine::Dfa)` is used for the matcher so
// that only the fields `pcre2_dfa_match_8` actually assigns are read (it never
// touches `startchar` on NOMATCH, for instance), and the substitute rows compare
// the return code, the value written through `outlengthptr`, AND the whole
// output buffer including the bytes past the written region, so that a buffer
// overrun in either library is caught.
//
// Each row is driven with many randomized inputs from a fixed seed, on top of
// the hand-picked configuration the row names.

mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

pub const COVERAGE: &[CfgCov] = &[
    // ------------------------------------------------- 3. dfa_match
    CfgCov { cfg_rows: &[294], note: "dfa baseline /abc/ ws=20 ovec=1, matchedby=DFA_INTERPRETER, mark always NULL" },
    CfgCov { cfg_rows: &[295], note: "dfa subject NULL/len 0, ZERO_TERMINATED, startoffset == length" },
    CfgCov { cfg_rows: &[296], note: "dfa wscount sizing sweep around the (wscount-2)/6 state capacity" },
    CfgCov { cfg_rows: &[297], note: "dfa one-start/many-ends ovector, longest first, rc latch to 0" },
    CfgCov { cfg_rows: &[298], note: "dfa oveccount 1 vs exactly-the-number-of-ends, plus pcre2_next_match to exhaustion" },
    CfgCov { cfg_rows: &[299], note: "dfa PCRE2_DFA_SHORTEST: first acceptance wins, ENDANCHORED post-check bypassed" },
    CfgCov { cfg_rows: &[300], note: "dfa PCRE2_ENDANCHORED post-check tests the scan position, not each end" },
    CfgCov { cfg_rows: &[301], note: "dfa PCRE2_DFA_RESTART two-call sequence after a real partial match" },
    CfgCov { cfg_rows: &[302], note: "dfa PCRE2_DFA_RESTART workspace[0]/[1] sanity bounds, both memcpy halves" },
    CfgCov { cfg_rows: &[303], note: "dfa NOTBOL / NOTEOL / NOTEMPTY / NOTEMPTY_ATSTART" },
    CfgCov { cfg_rows: &[304], note: "dfa PARTIAL_SOFT vs PARTIAL_HARD, could_continue / partial_newline / \\z" },
    CfgCov { cfg_rows: &[305], note: "dfa PCRE2_ERROR_PARTIAL ovector = {start_match, end_subject}" },
    CfgCov { cfg_rows: &[306], note: "dfa PCRE2_COPY_MATCHED_SUBJECT only on rc >= 0, length 0, match_data reuse" },
    CfgCov { cfg_rows: &[307], note: "dfa PCRE2_UTF with and without NO_UTF_CHECK, MATCH_INVALID_UTF" },
    CfgCov { cfg_rows: &[308], note: "dfa PCRE2_FIRSTLINE end_subject fudge, restore, newline bumpalong terminator" },
    CfgCov { cfg_rows: &[309], note: "dfa PCRE2_USE_OFFSET_LIMIT at, below and above the match start" },
    CfgCov { cfg_rows: &[310], note: "dfa start-optimization arms: first-CU, dual memchr, startline, bitmap, minlength, req_cu window" },
    CfgCov { cfg_rows: &[311], note: "dfa PCRE2_NO_START_OPTIMIZE and (*NO_START_OPT), and DFA_RESTART skipping it" },
    CfgCov { cfg_rows: &[312], note: "dfa-supported constructs: atomic, possessive, lookaround, recursion, \\X, props, xclass, eclass, callout" },
    CfgCov { cfg_rows: &[313], note: "dfa lookbehind setup: multi-branch max_back, UTF step-back, gone_back clamp" },
    CfgCov { cfg_rows: &[314], note: "dfa callout block fields and return values 0 / >0 / <0" },
    CfgCov { cfg_rows: &[315], note: "dfa match_limit and depth_limit sweeps across the crossover, plus (*LIMIT_MATCH=)" },
    CfgCov { cfg_rows: &[316], note: "dfa RWS growth and heap_limit clamp for assertions and recursion" },
    CfgCov { cfg_rows: &[317], note: "dfa nested calls always get wscount 1000 regardless of the caller's" },
    CfgCov { cfg_rows: &[318], note: "dfa duplicate-state suppression terminates /(a*)*b/" },
    CfgCov { cfg_rows: &[319], note: "dfa newline handling under all 6 conventions" },
    CfgCov { cfg_rows: &[320], note: "dfa \\R under BSR_UNICODE / BSR_ANYCRLF, quantified, independent of newline" },
    CfgCov { cfg_rows: &[321], note: "dfa leftchar/rightchar from start_used_ptr/last_used_ptr; NOMATCH leaves them" },
    // ------------------------------------------------ 4. substitute
    CfgCov { cfg_rows: &[322], note: "substitute baseline /b/ over abc, internal match_data, trailing NUL" },
    CfgCov { cfg_rows: &[323], note: "substitute NULL subject/replacement, ZERO_TERMINATED, deletion, embedded NULs" },
    CfgCov { cfg_rows: &[324], note: "substitute startoffset 0 / mid / == length, prefix copy" },
    CfgCov { cfg_rows: &[325], note: "substitute GLOBAL incl. the empty-match NOTEMPTY_ATSTART retry" },
    CfgCov { cfg_rows: &[326], note: "substitute GLOBAL where the unanchored retry matches later: gap copy" },
    CfgCov { cfg_rows: &[327], note: "substitute GLOBAL|ANCHORED: no ANCHORED is added internally" },
    CfgCov { cfg_rows: &[328], note: "substitute LITERAL takes precedence over EXTENDED" },
    CfgCov { cfg_rows: &[329], note: "substitute non-EXTENDED $ forms, all 14 shapes" },
    CfgCov { cfg_rows: &[330], note: "substitute $*MARK with no mark and with an embedded NUL" },
    CfgCov { cfg_rows: &[331], note: "substitute $+ matrix: top_bracket 0, small ovector, last-set scan" },
    CfgCov { cfg_rows: &[332], note: "substitute UNSET_EMPTY vs PCRE2_ERROR_UNSET with the replacement offset" },
    CfgCov { cfg_rows: &[333], note: "substitute UNKNOWN_UNSET at all 4 sites, alone and with UNSET_EMPTY" },
    CfgCov { cfg_rows: &[334], note: "substitute EXTENDED ${name:-default}, nested to the PTR_STACK_SIZE limit" },
    CfgCov { cfg_rows: &[335], note: "substitute EXTENDED ${name:+set:unset}, empty text2, eager validation" },
    CfgCov { cfg_rows: &[336], note: "substitute EXTENDED backslash escapes, the whole set" },
    CfgCov { cfg_rows: &[337], note: "substitute EXTENDED|GLOBAL unterminated \\Q persists across iterations" },
    CfgCov { cfg_rows: &[338], note: "substitute case forcing with no case callout, table path and UCD path" },
    CfgCov { cfg_rows: &[339], note: "substitute case callout: fast path, split path, to_case values, in-place, retry loop" },
    CfgCov { cfg_rows: &[340], note: "substitute case callout SIZE_MAX => REPLACECASE; inflation vs (len>>3)+10 with OVERFLOW_LENGTH" },
    CfgCov { cfg_rows: &[341], note: "substitute callout block: every field of pcre2_substitute_callout_block" },
    CfgCov { cfg_rows: &[342], note: "substitute callout returning 0 / >0 / <0 and the subs count" },
    CfgCov { cfg_rows: &[343], note: "substitute callout not invoked in an overflowed sizing pass" },
    CfgCov { cfg_rows: &[344], note: "substitute buffer sizing matrix and the two-call sizing protocol" },
    CfgCov { cfg_rows: &[345], note: "substitute REPLACEMENT_ONLY suppresses all 4 verbatim copies" },
    CfgCov { cfg_rows: &[346], note: "substitute PARTIAL_SOFT|REPLACEMENT_ONLY, full match and partial" },
    CfgCov { cfg_rows: &[347], note: "substitute SUBSTITUTE_MATCHED happy paths incl. rc 0 and MATCHED|GLOBAL" },
    CfgCov { cfg_rows: &[348], note: "substitute SUBSTITUTE_MATCHED with COPY_MATCHED_SUBJECT" },
    CfgCov { cfg_rows: &[349], note: "substitute UTF replacement validation, once, and NO_UTF_CHECK" },
    CfgCov { cfg_rows: &[350], note: "substitute external match_data: match_data->rc overwritten at EXIT" },
    CfgCov { cfg_rows: &[351], note: "substitute \\K with ALLOW_LOOKAROUND_BSK, progress assertion" },
];

#[test]
fn coverage_declaration_is_sane() {
    check_coverage_decl(COVERAGE);
}

// ==================================================================== helpers

/// `pcre2_real_match_data` from `c_src/src/pcre2_intmodedep.h`.  Only used for
/// the three fields that have no public accessor (`matchedby`, `leftchar`,
/// `rightchar`); `md_head` self-checks the layout against the public accessors
/// before any of them is read.
#[repr(C)]
struct MdHead {
    memctl_malloc: *mut c_void,
    memctl_free: *mut c_void,
    memctl_data: *mut c_void,
    code: *const c_void,
    subject: Sptr,
    mark: Sptr,
    heapframes: *mut c_void,
    heapframes_size: Sz,
    subject_length: Sz,
    start_offset: Sz,
    leftchar: Sz,
    rightchar: Sz,
    startchar: Sz,
    matchedby: u8,
    flags: u8,
    oveccount: u16,
    options: u32,
    rc: c_int,
}

const MATCHEDBY_DFA_INTERPRETER: u8 = 1;

unsafe fn md_head(api: &Api, md: Ptr, rc: c_int) -> &'static MdHead {
    let h = &*(md as *const MdHead);
    assert_eq!(
        h.rc, rc,
        "[{}] MdHead layout is wrong: rc read as {} but the call returned {}",
        api.name, h.rc, rc
    );
    assert_eq!(
        h.oveccount as u32,
        (api.get_ovector_count)(md),
        "[{}] MdHead layout is wrong: oveccount",
        api.name
    );
    h
}

/// One compile configuration.
#[derive(Clone, Copy, Debug)]
struct Cfg {
    name: &'static str,
    opts: u32,
    xopts: u32,
    newline: u32,
    bsr: u32,
}

impl Cfg {
    const fn new(name: &'static str, opts: u32) -> Cfg {
        Cfg { name, opts, xopts: 0, newline: 0, bsr: 0 }
    }
    const fn x(name: &'static str, opts: u32, xopts: u32) -> Cfg {
        Cfg { name, opts, xopts, newline: 0, bsr: 0 }
    }
    const fn nl(name: &'static str, opts: u32, newline: u32) -> Cfg {
        Cfg { name, opts, xopts: 0, newline, bsr: 0 }
    }
}

unsafe fn make_ctx(api: &Api, cfg: &Cfg) -> Ptr {
    let cc = (api.compile_context_create)(ptr::null_mut());
    assert!(!cc.is_null(), "[{}] compile_context_create failed", api.name);
    if cfg.newline != 0 {
        assert_eq!((api.set_newline)(cc, cfg.newline), 0);
    }
    if cfg.bsr != 0 {
        assert_eq!((api.set_bsr)(cc, cfg.bsr), 0);
    }
    if cfg.xopts != 0 {
        assert_eq!((api.set_compile_extra_options)(cc, cfg.xopts), 0);
    }
    cc
}

/// Compile `pat` under `cfg` in both libraries; assert identical bytecode.
unsafe fn compile_both(p: &Pair, pat: &[u8], cfg: &Cfg, d: &mut Diffs) -> Option<(Ptr, Ptr)> {
    let cca = make_ctx(&p.c, cfg);
    let ccb = make_ctx(&p.r, cfg);
    let (mut eca, mut ecb) = (0 as c_int, 0 as c_int);
    let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
    let a = (p.c.compile)(pat.as_ptr(), pat.len(), cfg.opts, &mut eca, &mut eoa, cca);
    let b = (p.r.compile)(pat.as_ptr(), pat.len(), cfg.opts, &mut ecb, &mut eob, ccb);
    (p.c.compile_context_free)(cca);
    (p.r.compile_context_free)(ccb);
    let tag = format!("compile({}) cfg[{}]", show(pat), cfg.name);
    d.eq(&format!("{tag} null?"), a.is_null(), b.is_null());
    d.eq(&format!("{tag} errorcode"), eca, ecb);
    d.eq(&format!("{tag} erroroffset"), eoa, eob);
    if a.is_null() || b.is_null() {
        if !a.is_null() {
            (p.c.code_free)(a);
        }
        if !b.is_null() {
            (p.r.code_free)(b);
        }
        return None;
    }
    assert_code_eq(a, b, &tag);
    d.checked += 1;
    Some((a, b))
}

/// Same, but panics if the pattern does not compile — for the hand-picked
/// patterns a row names explicitly.
unsafe fn compile_ok(p: &Pair, pat: &[u8], cfg: &Cfg) -> (Ptr, Ptr) {
    let mut d = Diffs::new();
    let r = compile_both(p, pat, cfg, &mut d);
    d.finish(&format!("compile {} cfg[{}]", show(pat), cfg.name));
    r.unwrap_or_else(|| panic!("pattern {} cfg[{}] must compile", show(pat), cfg.name))
}

unsafe fn compile_one(api: &Api, pat: &[u8], cfg: &Cfg) -> Ptr {
    let cc = make_ctx(api, cfg);
    let (mut ec, mut eo) = (0 as c_int, 0usize);
    let c = (api.compile)(pat.as_ptr(), pat.len(), cfg.opts, &mut ec, &mut eo, cc);
    (api.compile_context_free)(cc);
    assert!(!c.is_null(), "[{}] compile {} failed ec={ec} eo={eo}", api.name, show(pat));
    c
}

/// A single `pcre2_dfa_match_8` invocation shape.
#[derive(Clone, Copy)]
struct Dfa {
    start: Sz,
    opts: u32,
    ovec: u32,
    wsn: usize,
}

impl Dfa {
    const fn new() -> Dfa {
        Dfa { start: 0, opts: 0, ovec: 4, wsn: 1000 }
    }
}

/// Runs one DFA match in both libraries and compares every field the C defines
/// for the returned code, plus `matchedby` and the restart words the DFA leaves
/// in the caller's workspace.
unsafe fn dfa_cmp(
    p: &Pair,
    a: Ptr,
    b: Ptr,
    subj: Sptr,
    len: Sz,
    g: &Dfa,
    mctx: (Ptr, Ptr),
    tag: &str,
    d: &mut Diffs,
) -> (c_int, c_int) {
    let mda = (p.c.match_data_create)(g.ovec, ptr::null_mut());
    let mdb = (p.r.match_data_create)(g.ovec, ptr::null_mut());
    let mut wa = vec![0 as c_int; g.wsn];
    let mut wb = vec![0 as c_int; g.wsn];
    let ra = (p.c.dfa_match)(a, subj, len, g.start, g.opts, mda, mctx.0, wa.as_mut_ptr(), g.wsn);
    let rb = (p.r.dfa_match)(b, subj, len, g.start, g.opts, mdb, mctx.1, wb.as_mut_ptr(), g.wsn);
    d.eq(
        tag,
        read_match_out_of(&p.c, mda, ra, Engine::Dfa),
        read_match_out_of(&p.r, mdb, rb, Engine::Dfa),
    );
    if ra == rb && (ra >= 0 || ra == PCRE2_ERROR_NOMATCH || ra == PCRE2_ERROR_PARTIAL) {
        d.eq(
            &format!("{tag} :: matchedby"),
            md_head(&p.c, mda, ra).matchedby,
            md_head(&p.r, mdb, rb).matchedby,
        );
    }
    d.eq(
        &format!("{tag} :: workspace[0],[1] left behind"),
        (wa[0], wa[1]),
        (wb[0], wb[1]),
    );
    (p.c.match_data_free)(mda);
    (p.r.match_data_free)(mdb);
    (ra, rb)
}

/// `pcre2_next_match_8` driven to exhaustion over whatever `pcre2_dfa_match_8`
/// recorded, in both libraries.
unsafe fn next_match_seq(api: &Api, md: Ptr) -> Vec<(c_int, Sz, u32)> {
    let mut steps = Vec::new();
    loop {
        let (mut o, mut f) = (usize::MAX, 0xDEAD_BEEFu32);
        let rc = (api.next_match)(md, &mut o, &mut f);
        steps.push((rc, o, f));
        if rc <= 0 || steps.len() > 64 {
            break;
        }
    }
    steps
}

/// A match context with deterministic limits, so no pattern can run away.
unsafe fn bounded_mctx(p: &Pair) -> (Ptr, Ptr) {
    let a = (p.c.match_context_create)(ptr::null_mut());
    let b = (p.r.match_context_create)(ptr::null_mut());
    for (m, v) in [(&p.c, a), (&p.r, b)] {
        assert_eq!((m.set_match_limit)(v, 200_000), 0);
        assert_eq!((m.set_depth_limit)(v, 2_000), 0);
        assert_eq!((m.set_heap_limit)(v, 4_000), 0);
    }
    (a, b)
}

unsafe fn free_mctx(p: &Pair, m: (Ptr, Ptr)) {
    (p.c.match_context_free)(m.0);
    (p.r.match_context_free)(m.1);
}

/// Patterns used for the randomized DFA sweeps.  Everything here is supported
/// by the DFA engine (no back references, no \C, no (*ACCEPT) inside groups).
const DFA_PATS: &[&str] = &[
    "abc", "a|ab|abc", "a*", "a+", "a?", "a{2,4}", "(a|b)+", "^abc$", "\\babc\\b",
    "[a-c]+", "[^a-c]+", "\\d+", "\\w+", "\\s+", "a.c", ".*", ".+", "(?:ab)+",
    "(?>a+)b", "a++", "(?=a)b?", "(?<=ab)c", "(?<!x)a", "a(?!b)", "\\R", "\\R+",
    "\\N+", "(?i)abc", "(?m)^a$", "(?s).+", "x*", "", "a|", "|a", "(a*)*b",
    "\\((?:[^()]++|(?R))*\\)", "(?:a|b|c|d){1,3}", "a\\Kb", "(*MARK:m1)a",
];

/// Subjects used for the randomized DFA sweeps.
const DFA_SUBJ: &[&str] = &[
    "", "a", "ab", "abc", "abcd", "xxabcxx", "aaa", "aaaa", "aaab", "aaac",
    "\n", "a\nb", "a\r\nb", "a\rb", "a\0b", "abc\ndef", "\r\naa\r\n",
    "()", "(a(b)c)", "(((())))", "hello world", "ABC", "a\u{e9}b", "\u{100}\u{200}",
    "\u{3b1}\u{3b2}", "\u{1f600}", "a\u{85}b", "a\u{2028}b", "12345",
];

// ================================================ row 294: dfa baseline

#[test]
fn cfg_294_dfa_baseline() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(29_401);
    unsafe {
        let cfg = Cfg::new("default", 0);
        // --- the exact configuration the row names, with the C's stated outcome.
        let (a, b) = compile_ok(p, b"abc", &cfg);
        let subj = b"xxabcxx";
        let mda = (p.c.match_data_create)(1, ptr::null_mut());
        let mdb = (p.r.match_data_create)(1, ptr::null_mut());
        let mut wa = [0 as c_int; 20];
        let mut wb = [0 as c_int; 20];
        let ra = (p.c.dfa_match)(a, subj.as_ptr(), 7, 0, 0, mda, ptr::null_mut(), wa.as_mut_ptr(), 20);
        let rb = (p.r.dfa_match)(b, subj.as_ptr(), 7, 0, 0, mdb, ptr::null_mut(), wb.as_mut_ptr(), 20);
        assert_eq!(ra, 1, "row 294: the C must return rc 1 for /abc/ over xxabcxx");
        assert_eq!(
            md_head(&p.c, mda, ra).matchedby,
            MATCHEDBY_DFA_INTERPRETER,
            "row 294: the C must set matchedby = PCRE2_MATCHEDBY_DFA_INTERPRETER"
        );
        assert!(
            (p.c.get_mark)(mda).is_null(),
            "row 294: the DFA has no (*MARK) support, mark must be NULL"
        );
        let oa = read_match_out_of(&p.c, mda, ra, Engine::Dfa);
        assert_eq!(oa.ovector, vec![2usize, 5], "row 294: /abc/ must match at 2..5");
        d.eq("row 294 baseline", oa, read_match_out_of(&p.r, mdb, rb, Engine::Dfa));
        d.eq(
            "row 294 baseline matchedby",
            md_head(&p.c, mda, ra).matchedby,
            md_head(&p.r, mdb, rb).matchedby,
        );
        (p.c.match_data_free)(mda);
        (p.r.match_data_free)(mdb);
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // --- randomized: minimum workspace, oveccount 1, mcontext NULL. `mark`
        // must be NULL for EVERY pattern, including ones containing (*MARK).
        for pat in DFA_PATS {
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for _ in 0..24 {
                let sv = if rng.chance(2) {
                    rng.pick(DFA_SUBJ).as_bytes().to_vec()
                } else {
                    gen_ascii(&mut rng, 12)
                };
                let g = Dfa { start: 0, opts: 0, ovec: 1, wsn: 20 };
                let tag = format!("row294 {} vs {} ws=20 ovec=1", show(pb), show(&sv));
                dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, (ptr::null_mut(), ptr::null_mut()), &tag, &mut d);
                // mark is never set by the DFA engine, in either library
                for (api, code) in [(&p.c, a), (&p.r, b)] {
                    let md = (api.match_data_create)(1, ptr::null_mut());
                    let mut w = [0 as c_int; 20];
                    let rc = (api.dfa_match)(code, sv.as_ptr(), sv.len(), 0, 0, md, ptr::null_mut(), w.as_mut_ptr(), 20);
                    if rc >= 0 || rc == PCRE2_ERROR_NOMATCH || rc == PCRE2_ERROR_PARTIAL {
                        assert!(
                            (api.get_mark)(md).is_null(),
                            "[{}] row 294: DFA must never set mark ({} vs {})",
                            api.name, show(pb), show(&sv)
                        );
                        assert_eq!(
                            md_head(api, md, rc).matchedby,
                            MATCHEDBY_DFA_INTERPRETER,
                            "[{}] row 294: matchedby must be DFA_INTERPRETER",
                            api.name
                        );
                    }
                    (api.match_data_free)(md);
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("CONFIGS 294: dfa baseline /abc/ over xxabcxx, ws=20, ovec=1, mcontext NULL; matchedby and mark over the whole corpus");
}

// ============================ row 295: NULL subject / ZT / startoffset == length

#[test]
fn cfg_295_dfa_subject_shapes() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(29_501);
    unsafe {
        let m = bounded_mctx(p);
        for cfg in [Cfg::new("default", 0), Cfg::new("UTF", PCRE2_UTF)] {
            for pat in DFA_PATS {
                let pb = pat.as_bytes();
                let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
                // subject == NULL, length == 0  =>  the internal null_str
                for ovec in [0u32, 1, 4] {
                    let g = Dfa { ovec, ..Dfa::new() };
                    let tag = format!("row295 {} cfg[{}] subject=NULL len=0 ovec={ovec}", show(pb), cfg.name);
                    dfa_cmp(p, a, b, ptr::null(), 0, &g, m, &tag, &mut d);
                }
                for _ in 0..10 {
                    let base = if rng.chance(2) {
                        rng.pick(DFA_SUBJ).as_bytes().to_vec()
                    } else if cfg.opts & PCRE2_UTF != 0 {
                        gen_utf8(&mut rng, 8)
                    } else {
                        gen_ascii(&mut rng, 12)
                    };
                    if cfg.opts & PCRE2_UTF != 0 && std::str::from_utf8(&base).is_err() {
                        continue;
                    }
                    // explicit length
                    let g = Dfa { start: 0, ..Dfa::new() };
                    let tag = format!("row295 {} cfg[{}] subj={} explicit len", show(pb), cfg.name, show(&base));
                    dfa_cmp(p, a, b, base.as_ptr(), base.len(), &g, m, &tag, &mut d);
                    // startoffset == length
                    let g = Dfa { start: base.len(), ..Dfa::new() };
                    let tag = format!("row295 {} cfg[{}] subj={} start==len", show(pb), cfg.name, show(&base));
                    dfa_cmp(p, a, b, base.as_ptr(), base.len(), &g, m, &tag, &mut d);
                    // length == PCRE2_ZERO_TERMINATED needs a real NUL terminator
                    if !base.contains(&0) {
                        let mut zt = base.clone();
                        zt.push(0);
                        let g = Dfa::new();
                        let tag = format!("row295 {} cfg[{}] subj={} ZERO_TERMINATED", show(pb), cfg.name, show(&base));
                        dfa_cmp(p, a, b, zt.as_ptr(), PCRE2_ZERO_TERMINATED, &g, m, &tag, &mut d);
                        // ... and with startoffset == the ZT length
                        let g = Dfa { start: base.len(), ..Dfa::new() };
                        let tag = format!("row295 {} cfg[{}] subj={} ZT start==len", show(pb), cfg.name, show(&base));
                        dfa_cmp(p, a, b, zt.as_ptr(), PCRE2_ZERO_TERMINATED, &g, m, &tag, &mut d);
                    }
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 295: dfa subject == NULL with length 0, PCRE2_ZERO_TERMINATED, startoffset == length");
}

// ================================================= row 296: wscount sizing

#[test]
fn cfg_296_dfa_wscount() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let m = bounded_mctx(p);
        // A 5-group pattern, as the row names, plus patterns whose simultaneous
        // state count is large enough that the (wscount-2)/6 capacity actually
        // bites, so the exact crossover is compared.
        let pats: &[&str] = &[
            "(a)(b)(c)(d)(e)",
            "abc",
            "a|ab|abc|abcd|abcde",
            "(?:a|b|c|d|e|f|g|h|i|j|k|l)+",
            "a*b*c*d*e*f*",
            "\\((?:[^()]++|(?R))*\\)",
            "(?=(?:a|aa|aaa|aaaa|aaaaa))a*",
        ];
        for pat in pats {
            let pb = pat.as_bytes();
            let cfg = Cfg::new("default", 0);
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            let mut tb = 0u32;
            (p.c.pattern_info)(a, PCRE2_INFO_CAPTURECOUNT, &mut tb as *mut u32 as Ptr);
            // the three sizes the row names, plus a full sweep across the
            // capacity crossover (each extra 6 ints buys one more state)
            let mut sizes: Vec<usize> = vec![20, 1000, 20 * (1 + tb as usize)];
            sizes.extend(19..=80);
            sizes.extend([100usize, 122, 128, 200, 500, 2000, 7676, 7680]);
            sizes.sort_unstable();
            sizes.dedup();
            for subj in ["", "a", "abcde", "aaaaaaaa", "(((a)))", "abcdefghijkl"] {
                let sv = subj.as_bytes();
                for &wsn in &sizes {
                    if wsn == 0 {
                        continue;
                    }
                    let g = Dfa { wsn, ovec: 8, ..Dfa::new() };
                    let tag = format!("row296 {} subj={} wscount={wsn} (cap={})", show(pb), show(sv), wsn.saturating_sub(2) / 6);
                    dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
        // wscount below the API floor of 20 is a documented hard error; assert the
        // C's value and that both libraries agree for every size at the boundary.
        let cfg = Cfg::new("default", 0);
        let (a, b) = compile_ok(p, b"abc", &cfg);
        for wsn in 0usize..=21 {
            let mut wa = vec![0 as c_int; wsn.max(1)];
            let mut wb = vec![0 as c_int; wsn.max(1)];
            let mda = (p.c.match_data_create)(2, ptr::null_mut());
            let mdb = (p.r.match_data_create)(2, ptr::null_mut());
            let ra = (p.c.dfa_match)(a, b"xxabcxx".as_ptr(), 7, 0, 0, mda, m.0, wa.as_mut_ptr(), wsn);
            let rb = (p.r.dfa_match)(b, b"xxabcxx".as_ptr(), 7, 0, 0, mdb, m.1, wb.as_mut_ptr(), wsn);
            if wsn < 20 {
                assert_eq!(ra, PCRE2_ERROR_DFA_WSSIZE, "row 296: wscount {wsn} < 20 must be DFA_WSSIZE");
            }
            d.eq(&format!("row296 wscount floor {wsn}"), ra, rb);
            (p.c.match_data_free)(mda);
            (p.r.match_data_free)(mdb);
        }
        (p.c.code_free)(a);
        (p.r.code_free)(b);
        free_mctx(p, m);
    }
    d.finish("CONFIGS 296: dfa wscount 20 / 1000 / 20*(1+top_bracket) and a full sweep over the (wscount-2)/6 state capacity");
}

// ======================= rows 297-298: one start, many ends

/// `/a|ab|abc/` over `abc` records three ends for the single start 0.
#[test]
fn cfg_297_298_many_ends() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(29_701);
    unsafe {
        let m = bounded_mctx(p);
        let cfg = Cfg::new("default", 0);

        // --- the exact configuration rows 297/298 name, with the C's outcome.
        let (a, b) = compile_ok(p, b"a|ab|abc", &cfg);
        for (ovec, want_rc, want_ov) in [
            (1u32, 0 as c_int, vec![0usize, 3]),
            (2, 0, vec![0, 3, 0, 2]),
            (3, 3, vec![0, 3, 0, 2, 0, 1]),
            (5, 3, vec![0, 3, 0, 2, 0, 1]),
        ] {
            let mda = (p.c.match_data_create)(ovec, ptr::null_mut());
            let mut wa = vec![0 as c_int; 1000];
            let ra = (p.c.dfa_match)(a, b"abc".as_ptr(), 3, 0, 0, mda, m.0, wa.as_mut_ptr(), 1000);
            assert_eq!(ra, want_rc, "row 297: /a|ab|abc/ over abc with oveccount {ovec}");
            let oa = read_match_out_of(&p.c, mda, ra, Engine::Dfa);
            assert_eq!(oa.ovector, want_ov, "row 297: ends longest-first, oveccount {ovec}");
            // every recorded pair starts at the same place
            for k in 0..oa.ovector.len() / 2 {
                assert_eq!(oa.ovector[2 * k], 0, "row 297: ovector[2k] must all be the one start");
            }
            (p.c.match_data_free)(mda);
        }
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // --- randomized sweep: alternations with k distinct ends, oveccount
        // swept right across the rc-latches-to-0 boundary, and pcre2_next_match
        // driven to exhaustion over whatever was recorded.
        let end_pats: &[&str] = &[
            "a|ab|abc",
            "a|ab",
            "ab|a",
            "a|aa|aaa|aaaa",
            "a{1,4}",
            "a{1,4}?",
            "(?:a|ab|abc|abcd|abcde|abcdef)",
            "x|xy|xyz|xyzw",
            "\\w|\\w\\w|\\w\\w\\w",
            "a*",
            "(a|b)*",
            "abc",
        ];
        for pat in end_pats {
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for subj in ["", "a", "ab", "abc", "abcd", "abcde", "abcdef", "xyzw", "aaaa", "zzz"] {
                let sv = subj.as_bytes();
                for ovec in 0u32..=8 {
                    let g = Dfa { ovec, ..Dfa::new() };
                    let tag = format!("row297 {} subj={} ovec={ovec}", show(pb), show(sv));
                    let (ra, _) = dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                    // Same call again, this time keeping the two match_data so
                    // pcre2_next_match can walk the recorded ends to exhaustion.
                    let mda = (p.c.match_data_create)(ovec, ptr::null_mut());
                    let mdb = (p.r.match_data_create)(ovec, ptr::null_mut());
                    let mut wa = vec![0 as c_int; 1000];
                    let mut wb = vec![0 as c_int; 1000];
                    let r1 = (p.c.dfa_match)(a, sv.as_ptr(), sv.len(), 0, 0, mda, m.0, wa.as_mut_ptr(), 1000);
                    let r2 = (p.r.dfa_match)(b, sv.as_ptr(), sv.len(), 0, 0, mdb, m.1, wb.as_mut_ptr(), 1000);
                    d.eq(&format!("{tag} :: rc for next_match"), r1, r2);
                    d.eq(
                        &format!("{tag} :: next_match to exhaustion"),
                        next_match_seq(&p.c, mda),
                        next_match_seq(&p.r, mdb),
                    );
                    // row 298: with rc == 0 the longest end must still be kept
                    if ra == 0 && ovec > 0 {
                        let oa = read_match_out_of(&p.c, mda, r1, Engine::Dfa);
                        let ob = read_match_out_of(&p.r, mdb, r2, Engine::Dfa);
                        d.eq(&format!("{tag} :: rc==0 ovector"), oa, ob);
                    }
                    (p.c.match_data_free)(mda);
                    (p.r.match_data_free)(mdb);
                }
                // a couple of random start offsets too
                for _ in 0..3 {
                    let start = if sv.is_empty() { 0 } else { rng.below(sv.len() + 1) };
                    let ovec = rng.below(7) as u32;
                    let g = Dfa { start, ovec, ..Dfa::new() };
                    let tag = format!("row298 {} subj={} start={start} ovec={ovec}", show(pb), show(sv));
                    dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 297-298: dfa one-start/many-ends ovector, longest-first order, rc latch to 0, and pcre2_next_match to exhaustion");
}

// ================================================ row 299: DFA_SHORTEST

#[test]
fn cfg_299_dfa_shortest() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let m = bounded_mctx(p);
        let cfg = Cfg::new("default", 0);

        // the exact configuration the row names
        let (a, b) = compile_ok(p, b"a|ab|abc", &cfg);
        let mda = (p.c.match_data_create)(4, ptr::null_mut());
        let mut wa = vec![0 as c_int; 1000];
        let ra = (p.c.dfa_match)(
            a, b"abc".as_ptr(), 3, 0, PCRE2_DFA_SHORTEST, mda, m.0, wa.as_mut_ptr(), 1000,
        );
        assert_eq!(ra, 1, "row 299: DFA_SHORTEST must return exactly one pair");
        assert_eq!(
            read_match_out_of(&p.c, mda, ra, Engine::Dfa).ovector,
            vec![0usize, 1],
            "row 299: DFA_SHORTEST must yield the shortest end"
        );
        (p.c.match_data_free)(mda);
        // and the ENDANCHORED post-check is bypassed: /a|ab/ over "ab" with
        // ENDANCHORED|DFA_SHORTEST still reports the short match.
        let mda = (p.c.match_data_create)(4, ptr::null_mut());
        let ra = (p.c.dfa_match)(
            a, b"abc".as_ptr(), 3, 0, PCRE2_DFA_SHORTEST | PCRE2_ENDANCHORED, mda,
            m.0, wa.as_mut_ptr(), 1000,
        );
        assert_eq!(ra, 1, "row 299: DFA_SHORTEST bypasses the ENDANCHORED post-check");
        assert_eq!(
            read_match_out_of(&p.c, mda, ra, Engine::Dfa).ovector,
            vec![0usize, 1],
            "row 299: DFA_SHORTEST + ENDANCHORED keeps the shortest end"
        );
        (p.c.match_data_free)(mda);
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // randomized: DFA_SHORTEST alone and crossed with the other option bits
        let opts: &[(u32, &str)] = &[
            (PCRE2_DFA_SHORTEST, "SHORTEST"),
            (PCRE2_DFA_SHORTEST | PCRE2_ENDANCHORED, "SHORTEST|ENDANCHORED"),
            (PCRE2_DFA_SHORTEST | PCRE2_ANCHORED, "SHORTEST|ANCHORED"),
            (PCRE2_DFA_SHORTEST | PCRE2_NOTEMPTY, "SHORTEST|NOTEMPTY"),
            (PCRE2_DFA_SHORTEST | PCRE2_NOTEMPTY_ATSTART, "SHORTEST|NOTEMPTY_ATSTART"),
            (PCRE2_DFA_SHORTEST | PCRE2_PARTIAL_SOFT, "SHORTEST|PARTIAL_SOFT"),
            (PCRE2_DFA_SHORTEST | PCRE2_PARTIAL_HARD, "SHORTEST|PARTIAL_HARD"),
            (PCRE2_DFA_SHORTEST | PCRE2_NO_START_OPTIMIZE, "SHORTEST|NO_START_OPT"),
            (0, "none"),
        ];
        for pat in DFA_PATS {
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for subj in DFA_SUBJ {
                let sv = subj.as_bytes();
                for &(o, on) in opts {
                    for ovec in [1u32, 4] {
                        let g = Dfa { opts: o, ovec, ..Dfa::new() };
                        let tag = format!("row299 {} subj={} {on} ovec={ovec}", show(pb), show(sv));
                        dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                    }
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 299: PCRE2_DFA_SHORTEST returns at the first acceptance and bypasses the ENDANCHORED post-check");
}

// ============================================== row 300: ENDANCHORED

#[test]
fn cfg_300_dfa_endanchored() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let m = bounded_mctx(p);
        let cfg = Cfg::new("default", 0);

        // The exact configuration the row names: /a|ab/ over "ab".
        let (a, b) = compile_ok(p, b"a|ab", &cfg);
        let mda = (p.c.match_data_create)(4, ptr::null_mut());
        let mut wa = vec![0 as c_int; 1000];
        let ra = (p.c.dfa_match)(
            a, b"ab".as_ptr(), 2, 0, PCRE2_ENDANCHORED, mda, m.0, wa.as_mut_ptr(), 1000,
        );
        let oa = read_match_out_of(&p.c, mda, ra, Engine::Dfa);
        println!("row 300: /a|ab/ over ab with ENDANCHORED => rc {ra} ovector {:?}", oa.ovector);
        assert!(ra > 0, "row 300: the scan reached the end, so the match must stand");
        assert_eq!(
            oa.ovector,
            vec![0usize, 2, 0, 1],
            "row 300: the shorter, non-end-anchored end must still appear in ovector[2..]"
        );
        (p.c.match_data_free)(mda);
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // randomized: ENDANCHORED at compile time and at match time, over
        // patterns with several possible ends and over every start offset.
        for cfg in [
            Cfg::new("default", 0),
            Cfg::new("ENDANCHORED@compile", PCRE2_ENDANCHORED),
            Cfg::new("ANCHORED@compile", PCRE2_ANCHORED),
        ] {
            for pat in DFA_PATS {
                let pb = pat.as_bytes();
                let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
                for subj in DFA_SUBJ {
                    let sv = subj.as_bytes();
                    for &(o, on) in &[
                        (0u32, "none"),
                        (PCRE2_ENDANCHORED, "ENDANCHORED"),
                        (PCRE2_ANCHORED, "ANCHORED"),
                        (PCRE2_ANCHORED | PCRE2_ENDANCHORED, "ANCHORED|ENDANCHORED"),
                    ] {
                        for start in 0..=sv.len().min(3) {
                            let g = Dfa { start, opts: o, ovec: 6, ..Dfa::new() };
                            let tag = format!(
                                "row300 {} cfg[{}] subj={} {on} start={start}",
                                show(pb), cfg.name, show(sv),
                            );
                            dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                        }
                    }
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 300: PCRE2_ENDANCHORED post-check tests the DFA scan position, shorter ends stay in ovector[2..]");
}

// ================================== rows 301-302: PCRE2_DFA_RESTART

#[test]
fn cfg_301_302_dfa_restart() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(30_101);
    unsafe {
        let m = bounded_mctx(p);
        let cfg = Cfg::new("default", 0);

        // ---- row 301: the exact two-call sequence the row names.
        let (a, b) = compile_ok(p, b"abcd", &cfg);
        for (api, code) in [(&p.c, a), (&p.r, b)] {
            let md = (api.match_data_create)(4, ptr::null_mut());
            let mut ws = vec![0 as c_int; 100];
            let r1 = (api.dfa_match)(
                code, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, md, m.0, ws.as_mut_ptr(), 100,
            );
            assert_eq!(
                r1, PCRE2_ERROR_PARTIAL,
                "[{}] row 301: /abcd/ over ab with PARTIAL_SOFT must be PCRE2_ERROR_PARTIAL",
                api.name
            );
            let ov = (api.get_ovector_pointer)(md);
            assert_eq!((*ov, *ov.add(1)), (0usize, 2usize), "[{}] row 301 partial ovector", api.name);
            // call 2: same workspace pointer, same wscount
            let r2 = (api.dfa_match)(
                code, b"cd".as_ptr(), 2, 0, PCRE2_DFA_RESTART, md, m.0, ws.as_mut_ptr(), 100,
            );
            assert_eq!(r2, 1, "[{}] row 301: restart on cd must complete the match", api.name);
            (api.match_data_free)(md);
        }
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // ---- rows 301+302: drive a genuine partial match, then restart with the
        // SAME and with a DIFFERENT workspace size, and with workspace[0] and
        // workspace[1] moved across their sanity bounds.
        let restart_pats: &[&str] = &[
            "abcd", "a+b", "(?:ab)+c", "\\d{4}", "abc|abd", "^abcd$", "a.c.e",
            "(?<=ab)cd", "x*yz", "[a-z]{3,6}",
        ];
        for pat in restart_pats {
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for (head, tail) in [
                ("ab", "cd"), ("a", "bcd"), ("abc", "d"), ("", "abcd"),
                ("aa", "b"), ("xa", "bcd"), ("12", "34"), ("x", "yz"),
            ] {
                for &wsn in &[20usize, 26, 32, 44, 100, 1000] {
                    // --- call 1: a real partial match (or whatever the C does)
                    let mda = (p.c.match_data_create)(4, ptr::null_mut());
                    let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                    let mut wa = vec![0 as c_int; wsn];
                    let mut wb = vec![0 as c_int; wsn];
                    let hv = head.as_bytes();
                    let r1a = (p.c.dfa_match)(
                        a, hv.as_ptr(), hv.len(), 0, PCRE2_PARTIAL_SOFT, mda, m.0, wa.as_mut_ptr(), wsn,
                    );
                    let r1b = (p.r.dfa_match)(
                        b, hv.as_ptr(), hv.len(), 0, PCRE2_PARTIAL_SOFT, mdb, m.1, wb.as_mut_ptr(), wsn,
                    );
                    let tag = format!("row301 {} head={} ws={wsn}", show(pb), show(hv));
                    d.eq(&format!("{tag} call1"),
                        read_match_out_of(&p.c, mda, r1a, Engine::Dfa),
                        read_match_out_of(&p.r, mdb, r1b, Engine::Dfa));
                    d.eq(&format!("{tag} call1 workspace"), (wa[0], wa[1]), (wb[0], wb[1]));

                    let tv = tail.as_bytes();
                    // --- call 2a: restart with the IDENTICAL workspace size
                    let mut wa2 = wa.clone();
                    let mut wb2 = wb.clone();
                    let r2a = (p.c.dfa_match)(
                        a, tv.as_ptr(), tv.len(), 0, PCRE2_DFA_RESTART, mda, m.0, wa2.as_mut_ptr(), wsn,
                    );
                    let r2b = (p.r.dfa_match)(
                        b, tv.as_ptr(), tv.len(), 0, PCRE2_DFA_RESTART, mdb, m.1, wb2.as_mut_ptr(), wsn,
                    );
                    d.eq(&format!("{tag} restart same wscount, tail={}", show(tv)),
                        read_match_out_of(&p.c, mda, r2a, Engine::Dfa),
                        read_match_out_of(&p.r, mdb, r2b, Engine::Dfa));

                    // --- call 2b: restart with a DIFFERENT workspace size.  The C
                    // requires the identical wscount; whatever it does with a
                    // different one, both libraries must do the same.
                    for wsn2 in [20usize, wsn / 2, wsn + 6, wsn * 2, 1000] {
                        if wsn2 < 20 {
                            continue;
                        }
                        let mut xa = vec![0 as c_int; wsn2];
                        let mut xb = vec![0 as c_int; wsn2];
                        let n = wsn.min(wsn2);
                        xa[..n].copy_from_slice(&wa[..n]);
                        xb[..n].copy_from_slice(&wb[..n]);
                        let ra = (p.c.dfa_match)(
                            a, tv.as_ptr(), tv.len(), 0, PCRE2_DFA_RESTART, mda, m.0, xa.as_mut_ptr(), wsn2,
                        );
                        let rb = (p.r.dfa_match)(
                            b, tv.as_ptr(), tv.len(), 0, PCRE2_DFA_RESTART, mdb, m.1, xb.as_mut_ptr(), wsn2,
                        );
                        d.eq(
                            &format!("{tag} restart wscount {wsn}->{wsn2}, tail={}", show(tv)),
                            read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                            read_match_out_of(&p.r, mdb, rb, Engine::Dfa),
                        );
                    }

                    // --- row 302: workspace[0] in {0,1} (0 takes the memcpy
                    // back-fill path) and workspace[1] at the lower bound 1 and
                    // the upper bound (wscount-2)/3.
                    let bound = ((wsn - 2) / 3) as c_int;
                    for w0 in [0 as c_int, 1, 2, -1, -2] {
                        for w1 in [0 as c_int, 1, 2, bound - 1, bound, bound + 1] {
                            let mut xa = wa.clone();
                            let mut xb = wb.clone();
                            xa[0] = w0;
                            xa[1] = w1;
                            xb[0] = w0;
                            xb[1] = w1;
                            let ra = (p.c.dfa_match)(
                                a, tv.as_ptr(), tv.len(), 0, PCRE2_DFA_RESTART, mda, m.0, xa.as_mut_ptr(), wsn,
                            );
                            let rb = (p.r.dfa_match)(
                                b, tv.as_ptr(), tv.len(), 0, PCRE2_DFA_RESTART, mdb, m.1, xb.as_mut_ptr(), wsn,
                            );
                            d.eq(
                                &format!("{tag} restart ws0={w0} ws1={w1} (bound={bound}) tail={}", show(tv)),
                                read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                                read_match_out_of(&p.r, mdb, rb, Engine::Dfa),
                            );
                        }
                    }
                    (p.c.match_data_free)(mda);
                    (p.r.match_data_free)(mdb);
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // ---- restart forces `anchored`, kills firstline and every start
        // optimization: the same restart against a subject where the pattern
        // would only match later must NOT bump along.
        for cfg in [
            Cfg::new("default", 0),
            Cfg::new("FIRSTLINE", PCRE2_FIRSTLINE),
            Cfg::new("NO_START_OPTIMIZE", PCRE2_NO_START_OPTIMIZE),
        ] {
            let (a, b) = compile_ok(p, b"abcd", &cfg);
            for tail in ["cd", "xcd", "cdx", "\ncd", "cd\ncd"] {
                let tv = tail.as_bytes();
                let mda = (p.c.match_data_create)(4, ptr::null_mut());
                let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                let mut wa = vec![0 as c_int; 100];
                let mut wb = vec![0 as c_int; 100];
                (p.c.dfa_match)(a, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, mda, m.0, wa.as_mut_ptr(), 100);
                (p.r.dfa_match)(b, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, mdb, m.1, wb.as_mut_ptr(), 100);
                let ra = (p.c.dfa_match)(a, tv.as_ptr(), tv.len(), 0, PCRE2_DFA_RESTART, mda, m.0, wa.as_mut_ptr(), 100);
                let rb = (p.r.dfa_match)(b, tv.as_ptr(), tv.len(), 0, PCRE2_DFA_RESTART, mdb, m.1, wb.as_mut_ptr(), 100);
                if tail == "xcd" {
                    assert_eq!(
                        ra, PCRE2_ERROR_NOMATCH,
                        "row 301: DFA_RESTART forces anchored, so a later match must not be found"
                    );
                }
                d.eq(
                    &format!("row301 restart-anchored cfg[{}] tail={}", cfg.name, show(tv)),
                    read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                    read_match_out_of(&p.r, mdb, rb, Engine::Dfa),
                );
                (p.c.match_data_free)(mda);
                (p.r.match_data_free)(mdb);
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // ---- randomized fuzz over the restart workspace, still valid per the
        // documented sanity check, so both libraries must agree exactly.
        let (a, b) = compile_ok(p, b"a(?:bc|bd)e", &cfg);
        for _ in 0..400 {
            let wsn = *rng.pick(&[20usize, 32, 50, 200]);
            let bound = ((wsn - 2) / 3) as c_int;
            let mda = (p.c.match_data_create)(4, ptr::null_mut());
            let mdb = (p.r.match_data_create)(4, ptr::null_mut());
            let mut wa = vec![0 as c_int; wsn];
            let mut wb = vec![0 as c_int; wsn];
            let head = gen_ascii(&mut rng, 6);
            (p.c.dfa_match)(a, head.as_ptr(), head.len(), 0, PCRE2_PARTIAL_SOFT, mda, m.0, wa.as_mut_ptr(), wsn);
            (p.r.dfa_match)(b, head.as_ptr(), head.len(), 0, PCRE2_PARTIAL_SOFT, mdb, m.1, wb.as_mut_ptr(), wsn);
            let w0 = (rng.below(2)) as c_int;
            let w1 = rng.range(1, bound.max(1) as usize) as c_int;
            wa[0] = w0;
            wa[1] = w1;
            wb[0] = w0;
            wb[1] = w1;
            let tail = gen_ascii(&mut rng, 6);
            let ra = (p.c.dfa_match)(a, tail.as_ptr(), tail.len(), 0, PCRE2_DFA_RESTART, mda, m.0, wa.as_mut_ptr(), wsn);
            let rb = (p.r.dfa_match)(b, tail.as_ptr(), tail.len(), 0, PCRE2_DFA_RESTART, mdb, m.1, wb.as_mut_ptr(), wsn);
            d.eq(
                &format!("row302 fuzz ws={wsn} ws0={w0} ws1={w1} head={} tail={}", show(&head), show(&tail)),
                read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                read_match_out_of(&p.r, mdb, rb, Engine::Dfa),
            );
            (p.c.match_data_free)(mda);
            (p.r.match_data_free)(mdb);
        }
        (p.c.code_free)(a);
        (p.r.code_free)(b);
        free_mctx(p, m);
    }
    d.finish("CONFIGS 301-302: PCRE2_DFA_RESTART after a real partial match, same and different wscount, workspace[0]/[1] bounds");
}

// ============================== row 303: NOTBOL / NOTEOL / NOTEMPTY*

#[test]
fn cfg_303_dfa_not_options() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let m = bounded_mctx(p);
        let opts: &[(u32, &str)] = &[
            (0, "none"),
            (PCRE2_NOTBOL, "NOTBOL"),
            (PCRE2_NOTEOL, "NOTEOL"),
            (PCRE2_NOTBOL | PCRE2_NOTEOL, "NOTBOL|NOTEOL"),
            (PCRE2_NOTEMPTY, "NOTEMPTY"),
            (PCRE2_NOTEMPTY_ATSTART, "NOTEMPTY_ATSTART"),
            (PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART, "NOTEMPTY|NOTEMPTY_ATSTART"),
            (PCRE2_NOTBOL | PCRE2_NOTEMPTY, "NOTBOL|NOTEMPTY"),
        ];
        // The exact pattern/subject the row names, plus the anchors and the
        // in-pattern (*NOTEMPTY) / (*NOTEMPTY_ATSTART) verb forms.
        let pats: &[&str] = &[
            "^a*$", "^a*", "a*$", "^", "$", "\\A", "\\z", "\\Z", "a*", "", "^$",
            "(*NOTEMPTY)a*", "(*NOTEMPTY_ATSTART)a*", "\\ba*", "^.*$",
        ];
        for cfg in [
            Cfg::new("MULTILINE", PCRE2_MULTILINE),
            Cfg::new("default", 0),
            Cfg::new("MULTILINE|DOLLAR_ENDONLY", PCRE2_MULTILINE | PCRE2_DOLLAR_ENDONLY),
            Cfg::new("MULTILINE|ALT_CIRCUMFLEX", PCRE2_MULTILINE | PCRE2_ALT_CIRCUMFLEX),
        ] {
            for pat in pats {
                let pb = pat.as_bytes();
                let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
                for subj in ["\na\n", "a", "", "\n", "aa\n", "\n\n", "a\nb\n", "\r\na\r\n"] {
                    let sv = subj.as_bytes();
                    for &(o, on) in opts {
                        for start in 0..=sv.len() {
                            let g = Dfa { start, opts: o, ovec: 4, ..Dfa::new() };
                            let tag = format!(
                                "row303 {} cfg[{}] subj={} {on} start={start}",
                                show(pb), cfg.name, show(sv)
                            );
                            dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                        }
                    }
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 303: dfa NOTBOL / NOTEOL / NOTEMPTY / NOTEMPTY_ATSTART over /^a*$/m and friends, every start offset");
}

// ============================ rows 304-305: partial matching

#[test]
fn cfg_304_305_dfa_partial() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(30_401);
    unsafe {
        let m = bounded_mctx(p);
        let cfg = Cfg::new("default", 0);

        // ---- the exact configuration row 304 names.
        let (a, b) = compile_ok(p, b"abcd", &cfg);
        for (o, name) in [(PCRE2_PARTIAL_SOFT, "SOFT"), (PCRE2_PARTIAL_HARD, "HARD")] {
            let mda = (p.c.match_data_create)(4, ptr::null_mut());
            let mut wa = vec![0 as c_int; 1000];
            let ra = (p.c.dfa_match)(a, b"ab".as_ptr(), 2, 0, o, mda, m.0, wa.as_mut_ptr(), 1000);
            assert_eq!(ra, PCRE2_ERROR_PARTIAL, "row 304: /abcd/ over ab PARTIAL_{name}");
            // row 305: the ovector is {start_match, end_subject}
            assert_eq!(
                read_match_out_of(&p.c, mda, ra, Engine::Dfa).ovector,
                vec![0usize, 2],
                "row 305: PARTIAL ovector must be {{start_match, end_subject}}"
            );
            (p.c.match_data_free)(mda);
        }
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // ---- PARTIAL_HARD makes \z / \Z return PARTIAL even after a complete
        // match was recorded.
        for pat in ["abc\\z", "abc\\Z", "abc$", "abc"] {
            let (a, b) = compile_ok(p, pat.as_bytes(), &cfg);
            for (o, name) in [
                (PCRE2_PARTIAL_SOFT, "PARTIAL_SOFT"),
                (PCRE2_PARTIAL_HARD, "PARTIAL_HARD"),
                (0, "none"),
            ] {
                let mda = (p.c.match_data_create)(4, ptr::null_mut());
                let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                let mut wa = vec![0 as c_int; 1000];
                let mut wb = vec![0 as c_int; 1000];
                let ra = (p.c.dfa_match)(a, b"abc".as_ptr(), 3, 0, o, mda, m.0, wa.as_mut_ptr(), 1000);
                let rb = (p.r.dfa_match)(b, b"abc".as_ptr(), 3, 0, o, mdb, m.1, wb.as_mut_ptr(), 1000);
                println!("row 304: /{pat}/ over abc with {name} => rc {ra}");
                if pat.ends_with("\\z") && o == PCRE2_PARTIAL_HARD {
                    assert_eq!(
                        ra, PCRE2_ERROR_PARTIAL,
                        "row 304: PARTIAL_HARD makes \\z report PARTIAL even with a complete match"
                    );
                }
                d.eq(
                    &format!("row304 /{pat}/ abc {name}"),
                    read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                    read_match_out_of(&p.r, mdb, rb, Engine::Dfa),
                );
                (p.c.match_data_free)(mda);
                (p.r.match_data_free)(mdb);
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // ---- randomized: partial matching over patterns that exercise
        // could_continue, partial_newline, start_used_ptr and allowemptypartial.
        let part_pats: &[&str] = &[
            "abcd", "ab+cd", "(?<=ab)cd", "a\\R", "a\\Rb", "abc$", "abc\\z", "abc\\Z",
            "^abc", "a*", "", "\\bxyz", "a.{3}z", "\\d{4}-\\d{2}", "(?:abc|abd)e",
            "a(?=bc)", "x{2,5}y", "\\R+z",
        ];
        let part_opts: &[(u32, &str)] = &[
            (PCRE2_PARTIAL_SOFT, "SOFT"),
            (PCRE2_PARTIAL_HARD, "HARD"),
            (PCRE2_PARTIAL_SOFT | PCRE2_ANCHORED, "SOFT|ANCHORED"),
            (PCRE2_PARTIAL_HARD | PCRE2_ANCHORED, "HARD|ANCHORED"),
            (PCRE2_PARTIAL_SOFT | PCRE2_NOTEOL, "SOFT|NOTEOL"),
            (PCRE2_PARTIAL_HARD | PCRE2_NOTBOL, "HARD|NOTBOL"),
            (PCRE2_PARTIAL_SOFT | PCRE2_NOTEMPTY, "SOFT|NOTEMPTY"),
            (PCRE2_PARTIAL_SOFT | PCRE2_DFA_SHORTEST, "SOFT|SHORTEST"),
            (PCRE2_PARTIAL_HARD | PCRE2_DFA_SHORTEST, "HARD|SHORTEST"),
            (PCRE2_PARTIAL_SOFT | PCRE2_NO_START_OPTIMIZE, "SOFT|NO_START_OPT"),
        ];
        for nl in [0u32, PCRE2_NEWLINE_CR, PCRE2_NEWLINE_LF, PCRE2_NEWLINE_CRLF, PCRE2_NEWLINE_ANYCRLF] {
            let cfg = Cfg::nl("newline", 0, nl);
            for pat in part_pats {
                let pb = pat.as_bytes();
                let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
                for subj in [
                    "", "a", "ab", "abc", "abcd", "abcde", "xab", "a\r", "a\n", "a\r\n",
                    "12", "1234", "1234-", "1234-5", "abcz", "xyz", "aaaa",
                ] {
                    let sv = subj.as_bytes();
                    for &(o, on) in part_opts {
                        for ovec in [0u32, 1, 4] {
                            let g = Dfa { opts: o, ovec, ..Dfa::new() };
                            let tag = format!(
                                "row304 {} nl={nl} subj={} {on} ovec={ovec}",
                                show(pb), show(sv)
                            );
                            dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                        }
                    }
                }
                // row 305: random subjects, checking the PARTIAL ovector shape
                for _ in 0..8 {
                    let sv = gen_ascii(&mut rng, 10);
                    let o = part_opts[rng.below(part_opts.len())].0;
                    let ovec = rng.below(5) as u32;
                    let start = if sv.is_empty() { 0 } else { rng.below(sv.len() + 1) };
                    let g = Dfa { start, opts: o, ovec, ..Dfa::new() };
                    let tag = format!("row305 {} subj={} opts={o:#x} ovec={ovec} start={start}", show(pb), show(&sv));
                    dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 304-305: dfa PARTIAL_SOFT vs PARTIAL_HARD, \\z/\\Z under HARD, and the PARTIAL ovector contents");
}

// =================================== row 306: COPY_MATCHED_SUBJECT

#[test]
fn cfg_306_dfa_copy_matched_subject() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let m = bounded_mctx(p);
        let cfg = Cfg::new("default", 0);
        // The C applies the copy only on rc >= 0.  Assert that, then compare.
        for pat in ["abc", "a*", "abcd", "", "x"] {
            let (a, b) = compile_ok(p, pat.as_bytes(), &cfg);
            for subj in ["", "abc", "xxabcxx", "ab", "zzz"] {
                let sv = subj.as_bytes();
                for &(o, on) in &[
                    (PCRE2_COPY_MATCHED_SUBJECT, "COPY"),
                    (PCRE2_COPY_MATCHED_SUBJECT | PCRE2_PARTIAL_SOFT, "COPY|PARTIAL_SOFT"),
                    (PCRE2_COPY_MATCHED_SUBJECT | PCRE2_PARTIAL_HARD, "COPY|PARTIAL_HARD"),
                    (0, "none"),
                ] {
                    let mda = (p.c.match_data_create)(4, ptr::null_mut());
                    let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                    let mut wa = vec![0 as c_int; 1000];
                    let mut wb = vec![0 as c_int; 1000];
                    // Two calls on the SAME match_data, so the second frees the
                    // copy the first one made.
                    for round in 0..3 {
                        let ra = (p.c.dfa_match)(a, sv.as_ptr(), sv.len(), 0, o, mda, m.0, wa.as_mut_ptr(), 1000);
                        let rb = (p.r.dfa_match)(b, sv.as_ptr(), sv.len(), 0, o, mdb, m.1, wb.as_mut_ptr(), 1000);
                        let tag = format!("row306 /{pat}/ subj={} {on} round={round}", show(sv));
                        d.eq(&tag,
                            read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                            read_match_out_of(&p.r, mdb, rb, Engine::Dfa));
                        let ha = md_head(&p.c, mda, ra);
                        let hb = md_head(&p.r, mdb, rb);
                        // `flags & PCRE2_MD_COPIED_SUBJECT` (bit 0) and whether the
                        // stored subject pointer is the caller's or a copy.
                        let copied_a = ha.flags & 1;
                        let copied_b = hb.flags & 1;
                        d.eq(&format!("{tag} :: MD_COPIED_SUBJECT flag"), copied_a, copied_b);
                        d.eq(
                            &format!("{tag} :: subject pointer is the caller's?"),
                            ha.subject == sv.as_ptr(),
                            hb.subject == sv.as_ptr(),
                        );
                        if o & PCRE2_COPY_MATCHED_SUBJECT != 0 && ra >= 0 {
                            if sv.is_empty() {
                                assert!(
                                    ha.subject.is_null(),
                                    "row 306: length == 0 must store subject = NULL"
                                );
                            } else {
                                assert_ne!(
                                    ha.subject, sv.as_ptr(),
                                    "row 306: COPY_MATCHED_SUBJECT must store a copy on rc >= 0"
                                );
                                assert_eq!(
                                    std::slice::from_raw_parts(ha.subject, sv.len()),
                                    sv,
                                    "row 306: the copy must have the same contents"
                                );
                            }
                            assert_eq!(copied_a, 1, "row 306: MD_COPIED_SUBJECT must be set");
                        }
                        if o & PCRE2_COPY_MATCHED_SUBJECT != 0 && ra == PCRE2_ERROR_PARTIAL {
                            assert_eq!(
                                copied_a, 0,
                                "row 306: PCRE2_ERROR_PARTIAL must NOT take the copy"
                            );
                        }
                        if copied_a == 1 && !sv.is_empty() {
                            d.eq(
                                &format!("{tag} :: copied subject bytes"),
                                std::slice::from_raw_parts(ha.subject, sv.len()).to_vec(),
                                std::slice::from_raw_parts(hb.subject, sv.len()).to_vec(),
                            );
                        }
                    }
                    (p.c.match_data_free)(mda);
                    (p.r.match_data_free)(mdb);
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 306: PCRE2_COPY_MATCHED_SUBJECT only on rc >= 0, length 0 => NULL, and match_data reuse freeing the old copy");
}

// ================================================= row 307: UTF subjects

#[test]
fn cfg_307_dfa_utf() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(30_701);
    unsafe {
        let m = bounded_mctx(p);
        let utf_pats: &[&str] = &[
            ".", ".+", "\\X", "\\X+", "\\p{L}+", "\\P{L}", "[\\x{100}-\\x{200}]",
            "\\w+", "[^a]", "a", "\\x{1f600}", "(?i)\\x{3b1}", "\\R", "^.$",
            "[\\p{Greek}]+", "(*sr:\\w+)", "\\b\\w\\b",
        ];
        // PCRE2_MATCH_INVALID_UTF is unsupported by the DFA: assert the C's code.
        {
            let cfg = Cfg::new("UTF|MATCH_INVALID_UTF", PCRE2_UTF | PCRE2_MATCH_INVALID_UTF);
            let (a, b) = compile_ok(p, b".+", &cfg);
            for subj in ["abc", "\u{e9}", "\u{1f600}"] {
                let sv = subj.as_bytes();
                for &o in &[0u32, PCRE2_NO_UTF_CHECK] {
                    let g = Dfa { opts: o, ..Dfa::new() };
                    let tag = format!("row307 MATCH_INVALID_UTF subj={} opts={o:#x}", show(sv));
                    let (ra, _) = dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                    assert_eq!(
                        ra, PCRE2_ERROR_DFA_UINVALID_UTF,
                        "row 307: the DFA rejects PCRE2_MATCH_INVALID_UTF patterns"
                    );
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
        for cfg in [
            Cfg::new("UTF", PCRE2_UTF),
            Cfg::new("UTF|UCP", PCRE2_UTF | PCRE2_UCP),
            Cfg::new("UTF|CASELESS", PCRE2_UTF | PCRE2_CASELESS),
            Cfg::new("UCP", PCRE2_UCP),
        ] {
            for pat in utf_pats {
                let pb = pat.as_bytes();
                let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
                for _ in 0..24 {
                    let sv = if rng.chance(3) {
                        rng.pick(DFA_SUBJ).as_bytes().to_vec()
                    } else {
                        gen_utf8(&mut rng, 8)
                    };
                    // startoffset only on character boundaries, as the row names
                    let mut bounds = vec![0usize];
                    let mut i = 0;
                    while i < sv.len() {
                        i += if sv[i] < 0x80 {
                            1
                        } else if sv[i] < 0xe0 {
                            2
                        } else if sv[i] < 0xf0 {
                            3
                        } else {
                            4
                        };
                        if i <= sv.len() {
                            bounds.push(i);
                        }
                    }
                    for &start in &bounds {
                        for &o in &[0u32, PCRE2_NO_UTF_CHECK, PCRE2_PARTIAL_SOFT, PCRE2_ANCHORED] {
                            let g = Dfa { start, opts: o, ovec: 4, ..Dfa::new() };
                            let tag = format!(
                                "row307 {} cfg[{}] subj={} start={start} opts={o:#x}",
                                show(pb), cfg.name, show(&sv)
                            );
                            dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                        }
                    }
                    // and raw bytes, which the UTF check must reject identically
                    let raw = gen_raw(&mut rng, 8);
                    for &o in &[0u32, PCRE2_NO_UTF_CHECK] {
                        // NO_UTF_CHECK on invalid UTF is undefined behaviour in
                        // the C, so only the checked form is comparable.
                        if o == PCRE2_NO_UTF_CHECK && std::str::from_utf8(&raw).is_err() {
                            continue;
                        }
                        let g = Dfa { opts: o, ..Dfa::new() };
                        let tag = format!("row307 {} cfg[{}] raw={} opts={o:#x}", show(pb), cfg.name, show(&raw));
                        dfa_cmp(p, a, b, raw.as_ptr(), raw.len(), &g, m, &tag, &mut d);
                    }
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 307: dfa PCRE2_UTF with and without NO_UTF_CHECK, character-boundary start offsets, MATCH_INVALID_UTF rejection");
}

// ================================================ row 308: FIRSTLINE

#[test]
fn cfg_308_dfa_firstline() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let m = bounded_mctx(p);
        let pats: &[&str] = &[
            "abc", "a", "b", "\\d+", "^abc", "abc$", "a*", "[bc]+", "x", "\\R", ".",
        ];
        for nl in [
            PCRE2_NEWLINE_CR, PCRE2_NEWLINE_LF, PCRE2_NEWLINE_CRLF,
            PCRE2_NEWLINE_ANY, PCRE2_NEWLINE_ANYCRLF, PCRE2_NEWLINE_NUL,
        ] {
            for extra in [0u32, PCRE2_UTF, PCRE2_MULTILINE] {
                let cfg = Cfg::nl("FIRSTLINE", PCRE2_FIRSTLINE | extra, nl);
                let plain = Cfg::nl("plain", extra, nl);
                for pat in pats {
                    let pb = pat.as_bytes();
                    let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
                    let Some((a2, b2)) = compile_both(p, pb, &plain, &mut d) else {
                        (p.c.code_free)(a);
                        (p.r.code_free)(b);
                        continue;
                    };
                    for subj in [
                        "abc", "x\nabc", "x\rabc", "x\r\nabc", "x\0abc", "abc\nabc",
                        "\nabc", "\rabc", "\r\nabc", "a\u{85}abc", "abc\u{2028}abc",
                        "", "\n", "\r\n\r\n", "12\n34",
                    ] {
                        let sv = subj.as_bytes();
                        for &o in &[0u32, PCRE2_PARTIAL_SOFT, PCRE2_ANCHORED, PCRE2_NO_START_OPTIMIZE] {
                            for start in 0..=sv.len().min(4) {
                                let g = Dfa { start, opts: o, ovec: 4, ..Dfa::new() };
                                let tag = format!(
                                    "row308 {} FIRSTLINE nl={nl} extra={extra:#x} subj={} start={start} opts={o:#x}",
                                    show(pb), show(sv)
                                );
                                dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                                let tag = format!(
                                    "row308 {} plain nl={nl} extra={extra:#x} subj={} start={start} opts={o:#x}",
                                    show(pb), show(sv)
                                );
                                dfa_cmp(p, a2, b2, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                            }
                        }
                    }
                    (p.c.code_free)(a);
                    (p.r.code_free)(b);
                    (p.c.code_free)(a2);
                    (p.r.code_free)(b2);
                }
            }
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 308: dfa PCRE2_FIRSTLINE end_subject fudge and restore, and the IS_NEWLINE bumpalong terminator, all 6 conventions");
}

// ============================================== row 309: offset limit

#[test]
fn cfg_309_dfa_offset_limit() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let pats: &[&str] = &["abc", "b+", "\\d+", "a*", "", "x|abc", "(?<=x)abc"];
        for pat in pats {
            for use_lim in [true, false] {
                let cfg = Cfg::new(
                    if use_lim { "USE_OFFSET_LIMIT" } else { "plain" },
                    if use_lim { PCRE2_USE_OFFSET_LIMIT } else { 0 },
                );
                let pb = pat.as_bytes();
                let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
                for subj in ["xxabcxx", "abcabc", "", "abc", "12345", "xabc"] {
                    let sv = subj.as_bytes();
                    // sweep the limit right across the match start (the C uses a
                    // strict `>` so the limit AT the match start is allowed)
                    let mut lims: Vec<Sz> = (0..=sv.len() + 1).collect();
                    lims.push(PCRE2_UNSET);
                    for &lim in &lims {
                        let mca = (p.c.match_context_create)(ptr::null_mut());
                        let mcb = (p.r.match_context_create)(ptr::null_mut());
                        d.eq(
                            &format!("row309 set_offset_limit({lim})"),
                            (p.c.set_offset_limit)(mca, lim),
                            (p.r.set_offset_limit)(mcb, lim),
                        );
                        for (api, v) in [(&p.c, mca), (&p.r, mcb)] {
                            assert_eq!((api.set_match_limit)(v, 200_000), 0);
                            assert_eq!((api.set_depth_limit)(v, 2_000), 0);
                        }
                        let g = Dfa { ovec: 4, ..Dfa::new() };
                        let tag = format!(
                            "row309 {} cfg[{}] subj={} offset_limit={}",
                            show(pb), cfg.name, show(sv),
                            if lim == PCRE2_UNSET { "UNSET".to_string() } else { lim.to_string() }
                        );
                        dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, (mca, mcb), &tag, &mut d);
                        (p.c.match_context_free)(mca);
                        (p.r.match_context_free)(mcb);
                    }
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
    }
    d.finish("CONFIGS 309: dfa PCRE2_USE_OFFSET_LIMIT swept across the match start, and the BADOFFSETLIMIT arm without the compile flag");
}

// ============================== rows 310-311: start optimizations

#[test]
fn cfg_310_311_dfa_start_optimizations() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(31_001);
    unsafe {
        let m = bounded_mctx(p);
        // One pattern per documented arm of the optimization block.
        let arms: &[(&str, u32, &str)] = &[
            ("abc", 0, "first-CU caseful memchr + req_cu"),
            ("abc", PCRE2_ANCHORED, "anchored first-CU pre-check"),
            ("(?i)abc", 0, "caseless dual memchr"),
            ("(?i)abc", PCRE2_ANCHORED, "anchored caseless first-CU"),
            ("[abc]xyz", 0, "start_bits bitmap"),
            ("[abc]xyz", PCRE2_ANCHORED, "anchored bitmap pre-check"),
            ("^abc", PCRE2_MULTILINE, "startline scan"),
            ("^[abc]", PCRE2_MULTILINE, "startline scan, bitmap suppressed"),
            ("abcdefghij", 0, "minlength cut"),
            ("a.{20}z", 0, "minlength cut + req_cu"),
            ("\\d+C", 0, "req_cu window"),
            ("^\\d+C", 0, "anchored req_cu window"),
            ("a", 0, "single first-CU, no req_cu"),
            (".*", 0, "no first-CU, no bitmap"),
        ];
        // Subjects short and LONG: the req_cu window is REQ_CU_MAX (5000) for
        // anchored patterns and 5 000 000 otherwise, so a >5000-byte subject
        // takes the other side of that branch.
        let long_a: Vec<u8> = std::iter::repeat(b'a').take(6000).collect();
        let mut long_ab: Vec<u8> = std::iter::repeat(b'a').take(6000).collect();
        long_ab.extend_from_slice(b"bc");
        let mut long_digits: Vec<u8> = std::iter::repeat(b'1').take(6000).collect();
        long_digits.push(b'C');
        let subjects: Vec<Vec<u8>> = vec![
            b"".to_vec(), b"abc".to_vec(), b"xxabcxx".to_vec(), b"ABC".to_vec(),
            b"aBc".to_vec(), b"xyz".to_vec(), b"axyz".to_vec(), b"zzabcxyz".to_vec(),
            b"\nabc".to_vec(), b"x\nabc".to_vec(), b"x\r\nabc".to_vec(),
            b"abcdefghij".to_vec(), b"123C".to_vec(), b"123".to_vec(),
            long_a, long_ab, long_digits,
        ];
        for &(pat, copts, arm) in arms {
            for &(extra, xname) in &[
                (0u32, "opt-on"),
                (PCRE2_NO_START_OPTIMIZE, "NO_START_OPTIMIZE"),
            ] {
                let cfg = Cfg::new("start-opt", copts | extra);
                let pb = pat.as_bytes();
                let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
                for sv in &subjects {
                    for &o in &[0u32, PCRE2_PARTIAL_SOFT, PCRE2_ANCHORED, PCRE2_NO_START_OPTIMIZE] {
                        let g = Dfa { opts: o, ovec: 4, ..Dfa::new() };
                        let tag = format!(
                            "row310 [{arm}] /{pat}/ {xname} subj(len={}) opts={o:#x}",
                            sv.len()
                        );
                        dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                    }
                    // several start offsets exercise the memchr result caches
                    for _ in 0..3 {
                        let start = if sv.is_empty() { 0 } else { rng.below(sv.len() + 1) };
                        let g = Dfa { start, ovec: 4, ..Dfa::new() };
                        let tag = format!("row310 [{arm}] /{pat}/ {xname} subj(len={}) start={start}", sv.len());
                        dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                    }
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }

        // ---- row 311: the (*NO_START_OPT) verb, the compile option, the
        // pcre2_set_optimize() route, and DFA_RESTART skipping the block
        // independently of all of them.
        for (pat, copts, name) in [
            ("(*NO_START_OPT)abcd", 0u32, "verb"),
            ("abcd", PCRE2_NO_START_OPTIMIZE, "compile option"),
            ("abcd", 0, "optimizations on"),
        ] {
            let cfg = Cfg::new(name, copts);
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for subj in ["", "abcd", "xxabcd", "ab", "zzz", "abcdabcd"] {
                let sv = subj.as_bytes();
                for &o in &[0u32, PCRE2_NO_START_OPTIMIZE, PCRE2_PARTIAL_SOFT] {
                    let g = Dfa { opts: o, ovec: 4, ..Dfa::new() };
                    let tag = format!("row311 /{pat}/ [{name}] subj={} opts={o:#x}", show(sv));
                    dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                }
                // and the restart route, which skips the block regardless
                let mda = (p.c.match_data_create)(4, ptr::null_mut());
                let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                let mut wa = vec![0 as c_int; 100];
                let mut wb = vec![0 as c_int; 100];
                (p.c.dfa_match)(a, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, mda, m.0, wa.as_mut_ptr(), 100);
                (p.r.dfa_match)(b, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, mdb, m.1, wb.as_mut_ptr(), 100);
                let ra = (p.c.dfa_match)(a, sv.as_ptr(), sv.len(), 0, PCRE2_DFA_RESTART, mda, m.0, wa.as_mut_ptr(), 100);
                let rb = (p.r.dfa_match)(b, sv.as_ptr(), sv.len(), 0, PCRE2_DFA_RESTART, mdb, m.1, wb.as_mut_ptr(), 100);
                d.eq(
                    &format!("row311 restart /{pat}/ [{name}] subj={}", show(sv)),
                    read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                    read_match_out_of(&p.r, mdb, rb, Engine::Dfa),
                );
                (p.c.match_data_free)(mda);
                (p.r.match_data_free)(mdb);
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // the pcre2_set_optimize() route as well
        for opt in [
            PCRE2_OPTIMIZATION_NONE,
            PCRE2_OPTIMIZATION_FULL,
            PCRE2_START_OPTIMIZE_OFF,
            PCRE2_START_OPTIMIZE,
            PCRE2_AUTO_POSSESS_OFF,
        ] {
            for pat in ["abcd", "(?i)abcd", "[ab]cd", "^ab", ".*x"] {
                let pb = pat.as_bytes();
                let (mut ec, mut eo) = (0 as c_int, 0usize);
                let cca = (p.c.compile_context_create)(ptr::null_mut());
                let ccb = (p.r.compile_context_create)(ptr::null_mut());
                assert_eq!((p.c.set_optimize)(cca, opt), 0);
                assert_eq!((p.r.set_optimize)(ccb, opt), 0);
                let a = (p.c.compile)(pb.as_ptr(), pb.len(), 0, &mut ec, &mut eo, cca);
                let b = (p.r.compile)(pb.as_ptr(), pb.len(), 0, &mut ec, &mut eo, ccb);
                (p.c.compile_context_free)(cca);
                (p.r.compile_context_free)(ccb);
                assert!(!a.is_null() && !b.is_null());
                assert_code_eq(a, b, &format!("row311 optimize={opt} /{pat}/"));
                for subj in ["", "abcd", "xxabcdxx", "ABCD", "\nab", "zzzx"] {
                    let sv = subj.as_bytes();
                    let g = Dfa { ovec: 4, ..Dfa::new() };
                    let tag = format!("row311 optimize={opt} /{pat}/ subj={}", show(sv));
                    dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 310-311: every dfa start-optimization arm incl. the 5000-code-unit req_cu window, and all three routes to disabling them");
}

// ============================ rows 312-313: supported constructs

#[test]
fn cfg_312_313_dfa_constructs() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(31_201);
    unsafe {
        let m = bounded_mctx(p);
        // One entry per DFA-supported construct family the row names.
        let cons: &[(&str, u32)] = &[
            ("(?>a+)b", 0),
            ("(?:a)++b", 0),
            ("(a)++", 0),
            ("a++", 0),
            ("a*+b", 0),
            ("[a-c]{2,}+", 0),
            ("(?=a)", 0),
            ("(?=ab)a", 0),
            ("(?<=ab)c", 0),
            ("(?<=ab|c)d", 0),
            ("(?<!ab)c", 0),
            ("a(?!b)", 0),
            ("(?<=a{2,4})x", 0),
            ("\\((?:[^()]++|(?R))*\\)", 0),
            ("(a|b(?1))", 0),
            ("(?(DEFINE)(?<w>\\w+))(?&w)", 0),
            ("a(*FAIL)|b", 0),
            ("\\X", PCRE2_UTF),
            ("\\X+", PCRE2_UTF),
            ("\\p{L}+", PCRE2_UTF),
            ("[\\x{100}-\\x{200}]", PCRE2_UTF),
            ("[\\x{100}\\p{L}a-c]", PCRE2_UTF),
            ("(?[ [\\p{L}] - [a-z] ])", PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS),
            ("[a&&b]", PCRE2_ALT_EXTENDED_CLASS),
            ("\\b\\w+\\b", PCRE2_UTF | PCRE2_UCP),
            ("\\B\\w", PCRE2_UTF | PCRE2_UCP),
            ("a(?C1)b", 0),
            ("(*script_run:\\w+)", PCRE2_UTF | PCRE2_UCP),
            ("(?i)(?:ab|cd)", 0),
            ("(a)?(?(1)b|c)", 0),
            ("(?(?=a)ab|cd)", 0),
            ("a\\Kb", 0),
            ("(?:a|b|c){2,3}", 0),
        ];
        let subs: &[&str] = &[
            "", "a", "ab", "abc", "abcd", "aab", "b", "c", "cd", "d", "x",
            "(a)", "((a))", "(((a)))", "abab", "\u{e9}", "e\u{301}",
            "\u{3b1}\u{3b2}", "\u{1f600}", "\u{100}", "ABCD", "aaaa",
            "\u{4e00}\u{3042}", "hello", "12ab",
        ];
        for &(pat, copts) in cons {
            let cfg = Cfg::new("constructs", copts);
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for subj in subs {
                let sv = subj.as_bytes();
                if copts & PCRE2_UTF != 0 && std::str::from_utf8(sv).is_err() {
                    continue;
                }
                for &o in &[0u32, PCRE2_ANCHORED, PCRE2_DFA_SHORTEST, PCRE2_PARTIAL_SOFT] {
                    // row 313: gone_back clamps at the subject start, so sweep
                    // startoffset from 0 up past the longest lookbehind.
                    for start in 0..=sv.len().min(4) {
                        let g = Dfa { start, opts: o, ovec: 6, ..Dfa::new() };
                        let tag = format!("row312 /{pat}/ subj={} start={start} opts={o:#x}", show(sv));
                        dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                    }
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // ---- row 313 explicitly: multi-branch lookbehind, startoffset 0 vs 3,
        // and UTF stepping back character by character.
        let lb: &[(&str, u32)] = &[
            ("(?<=ab|c)d", 0),
            ("(?<=abc|x)d", 0),
            ("(?<=a{2,4})x", 0),
            ("(?<=\\x{100}\\x{200})x", PCRE2_UTF),
            ("(?<=\\X)a", PCRE2_UTF),
            ("(?<=..)c", PCRE2_UTF),
            ("(?<=^ab)c", 0),
            ("(?<!ab|c)d", 0),
        ];
        for &(pat, copts) in lb {
            let cfg = Cfg::new("lookbehind", copts);
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for subj in [
                "abd", "cd", "xd", "abcd", "aaax", "aax", "ax", "d",
                "\u{100}\u{200}x", "\u{1f600}a", "\u{e9}\u{e9}c",
            ] {
                let sv = subj.as_bytes();
                if copts & PCRE2_UTF != 0 && std::str::from_utf8(sv).is_err() {
                    continue;
                }
                for start in 0..=sv.len() {
                    if copts & PCRE2_UTF != 0 && start < sv.len() && (sv[start] & 0xc0) == 0x80 {
                        continue; // not a character boundary
                    }
                    let g = Dfa { start, ovec: 4, ..Dfa::new() };
                    let tag = format!("row313 /{pat}/ subj={} start={start}", show(sv));
                    dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // ---- randomized: the whole construct corpus against random subjects
        for &(pat, copts) in cons {
            let cfg = Cfg::new("constructs", copts);
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for _ in 0..12 {
                let sv = if copts & PCRE2_UTF != 0 {
                    gen_utf8(&mut rng, 8)
                } else {
                    gen_ascii(&mut rng, 12)
                };
                let start = if sv.is_empty() { 0 } else { rng.below(sv.len() + 1) };
                if copts & PCRE2_UTF != 0 && start < sv.len() && (sv[start] & 0xc0) == 0x80 {
                    continue;
                }
                let g = Dfa { start, ovec: rng.range(1, 6) as u32, ..Dfa::new() };
                let tag = format!("row312 fuzz /{pat}/ subj={} start={start}", show(&sv));
                dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 312-313: every DFA-supported construct family, and the lookbehind max_back / UTF step-back / gone_back clamp");
}

// ============================================== row 314: DFA callouts

static mut DFA_CALLOUT_LOG: Vec<String> = Vec::new();
static mut DFA_CALLOUT_RET: c_int = 0;

/// `pcre2_callout_block` — exact field order from `c_src/include/pcre2.h`.
#[repr(C)]
struct CalloutBlock {
    version: u32,
    callout_number: u32,
    capture_top: u32,
    capture_last: u32,
    offset_vector: *const Sz,
    mark: Sptr,
    subject: Sptr,
    subject_length: Sz,
    start_match: Sz,
    current_position: Sz,
    pattern_position: Sz,
    next_item_length: Sz,
    callout_string_offset: Sz,
    callout_string_length: Sz,
    callout_string: Sptr,
    callout_flags: u32,
}

unsafe extern "C" fn dfa_callout(blk: *mut c_void, _d: *mut c_void) -> c_int {
    let b = &*(blk as *const CalloutBlock);
    let s = if b.callout_string.is_null() {
        String::from("-")
    } else {
        show(std::slice::from_raw_parts(b.callout_string, b.callout_string_length))
    };
    (*ptr::addr_of_mut!(DFA_CALLOUT_LOG)).push(format!(
        "v={} n={} ct={} cl={} sublen={} sm={} cp={} pp={} nil={} cso={} csl={} cf={} mark_null={} str={s}",
        b.version, b.callout_number, b.capture_top, b.capture_last, b.subject_length,
        b.start_match, b.current_position, b.pattern_position, b.next_item_length,
        b.callout_string_offset, b.callout_string_length, b.callout_flags,
        b.mark.is_null()
    ));
    *ptr::addr_of!(DFA_CALLOUT_RET)
}

#[test]
fn cfg_314_dfa_callout() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // The exact assertions the row names, on the C.
        let cfg = Cfg::new("default", 0);
        let (a, b) = compile_ok(p, b"a(?C1)b", &cfg);
        let mca = (p.c.match_context_create)(ptr::null_mut());
        assert_eq!((p.c.set_callout)(mca, Some(dfa_callout), ptr::null_mut()), 0);
        DFA_CALLOUT_RET = 0;
        DFA_CALLOUT_LOG.clear();
        let mda = (p.c.match_data_create)(4, ptr::null_mut());
        let mut wa = vec![0 as c_int; 1000];
        (p.c.dfa_match)(a, b"zab".as_ptr(), 3, 0, 0, mda, mca, wa.as_mut_ptr(), 1000);
        let log = DFA_CALLOUT_LOG.clone();
        assert!(!log.is_empty(), "row 314: the callout must fire");
        for l in &log {
            assert!(l.contains("v=2 "), "row 314: DFA callout block version must be 2: {l}");
            assert!(l.contains(" ct=1 "), "row 314: capture_top must be 1: {l}");
            assert!(l.contains(" cl=0 "), "row 314: capture_last must be 0: {l}");
            assert!(l.contains("mark_null=true"), "row 314: mark must be NULL: {l}");
        }
        (p.c.match_data_free)(mda);
        (p.c.match_context_free)(mca);
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // Full comparison of the whole callout sequence, for every return value.
        let cal_pats: &[(&str, u32)] = &[
            ("a(?C1)b", 0),
            ("a(?C)b", 0),
            ("a(?C255)b", 0),
            ("a(?C{txt})b", 0),
            ("(?C0)a(?C1)b(?C2)c", 0),
            ("a(?C1)b|a(?C2)c", 0),
            ("(?:a(?C1)|b(?C2))+", 0),
            ("^(?C1)a*(?C2)$", 0),
            ("\\d+(?C9)\\w*", 0),
            ("abc", PCRE2_AUTO_CALLOUT),
            ("a|ab|abc", PCRE2_AUTO_CALLOUT),
            ("(?<=ab)(?C1)c", 0),
            ("(?>a(?C1)+)b", 0),
            ("\\X(?C7)", PCRE2_UTF | PCRE2_AUTO_CALLOUT),
        ];
        for &(pat, copts) in cal_pats {
            let cfg = Cfg::new("callout", copts);
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            let mca = (p.c.match_context_create)(ptr::null_mut());
            let mcb = (p.r.match_context_create)(ptr::null_mut());
            for (api, v) in [(&p.c, mca), (&p.r, mcb)] {
                assert_eq!((api.set_callout)(v, Some(dfa_callout), ptr::null_mut()), 0);
                assert_eq!((api.set_match_limit)(v, 200_000), 0);
                assert_eq!((api.set_depth_limit)(v, 2_000), 0);
            }
            for subj in ["", "a", "ab", "zab", "abc", "abcabc", "ac", "\u{e9}a", "123x"] {
                let sv = subj.as_bytes();
                if copts & PCRE2_UTF != 0 && std::str::from_utf8(sv).is_err() {
                    continue;
                }
                for ret in [0 as c_int, 1, 2, 255, -1, -2, -44, i32::MIN / 2] {
                    for &o in &[0u32, PCRE2_NO_START_OPTIMIZE, PCRE2_DFA_SHORTEST] {
                        let mda = (p.c.match_data_create)(4, ptr::null_mut());
                        let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                        let mut wa = vec![0 as c_int; 1000];
                        let mut wb = vec![0 as c_int; 1000];
                        DFA_CALLOUT_RET = ret;
                        DFA_CALLOUT_LOG.clear();
                        let ra = (p.c.dfa_match)(a, sv.as_ptr(), sv.len(), 0, o, mda, mca, wa.as_mut_ptr(), 1000);
                        let la = DFA_CALLOUT_LOG.clone();
                        DFA_CALLOUT_LOG.clear();
                        let rb = (p.r.dfa_match)(b, sv.as_ptr(), sv.len(), 0, o, mdb, mcb, wb.as_mut_ptr(), 1000);
                        let lb = DFA_CALLOUT_LOG.clone();
                        let tag = format!("row314 /{pat}/ subj={} ret={ret} opts={o:#x}", show(sv));
                        d.eq(&tag,
                            read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                            read_match_out_of(&p.r, mdb, rb, Engine::Dfa));
                        let la_empty = la.is_empty();
                        d.eq(&format!("{tag} :: callout sequence"), la, lb);
                        // a negative return abandons the match with that code
                        if ret < 0 && !la_empty {
                            assert_eq!(ra, ret, "row 314: a negative callout return must be the rc");
                        }
                        (p.c.match_data_free)(mda);
                        (p.r.match_data_free)(mdb);
                    }
                }
            }
            (p.c.match_context_free)(mca);
            (p.r.match_context_free)(mcb);
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("CONFIGS 314: dfa callout block fields (version 2, capture_top 1, capture_last 0, mark NULL) and returns 0 / >0 / <0");
}

// ================================== row 315: match and depth limits

#[test]
fn cfg_315_dfa_limits() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let cfg = Cfg::new("default", 0);
        // ---- match_limit counts total internal_dfa_match invocations, reset
        // once per pcre2_dfa_match call rather than per bumpalong.  /a/ over a
        // 10-byte subject with no match has 11 start positions.
        let (a, b) = compile_ok(p, b"a", &cfg);
        let subj = b"bbbbbbbbbb"; // 10 bytes, no 'a'
        let mut first_ok = None;
        for lim in 1u32..=16 {
            let mca = (p.c.match_context_create)(ptr::null_mut());
            let mcb = (p.r.match_context_create)(ptr::null_mut());
            assert_eq!((p.c.set_match_limit)(mca, lim), 0);
            assert_eq!((p.r.set_match_limit)(mcb, lim), 0);
            let g = Dfa { opts: PCRE2_NO_START_OPTIMIZE, ovec: 2, ..Dfa::new() };
            let tag = format!("row315 /a/ over 10 bytes match_limit={lim}");
            let (ra, _) = dfa_cmp(p, a, b, subj.as_ptr(), 10, &g, (mca, mcb), &tag, &mut d);
            if ra != PCRE2_ERROR_MATCHLIMIT && first_ok.is_none() {
                first_ok = Some(lim);
            }
            (p.c.match_context_free)(mca);
            (p.r.match_context_free)(mcb);
        }
        println!("row 315: /a/ over 10 non-matching bytes first succeeds at match_limit {first_ok:?}");
        assert_eq!(
            first_ok,
            Some(11),
            "row 315: match_limit counts total internal_dfa_match calls (11 start positions)"
        );
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // ---- depth_limit bounds the nesting depth at limit+1.
        let (a, b) = compile_ok(p, b"(?>(?>(?>a)))", &cfg);
        for lim in 0u32..=6 {
            let mca = (p.c.match_context_create)(ptr::null_mut());
            let mcb = (p.r.match_context_create)(ptr::null_mut());
            assert_eq!((p.c.set_depth_limit)(mca, lim), 0);
            assert_eq!((p.r.set_depth_limit)(mcb, lim), 0);
            assert_eq!((p.c.set_match_limit)(mca, 200_000), 0);
            assert_eq!((p.r.set_match_limit)(mcb, 200_000), 0);
            let g = Dfa { ovec: 2, ..Dfa::new() };
            let tag = format!("row315 /(?>(?>(?>a)))/ depth_limit={lim}");
            let (ra, _) = dfa_cmp(p, a, b, b"a".as_ptr(), 1, &g, (mca, mcb), &tag, &mut d);
            println!("row 315: depth_limit={lim} => rc {ra}");
            (p.c.match_context_free)(mca);
            (p.r.match_context_free)(mcb);
        }
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // ---- sweeps across the crossover for a spread of patterns, plus the
        // in-pattern (*LIMIT_MATCH=) / (*LIMIT_DEPTH=) forms, which can only
        // lower the context value.
        let lim_pats: &[&str] = &[
            "a", "abc", "a+", "(a*)*b", "\\((?:[^()]++|(?R))*\\)",
            "(?>(?>(?>(?>a))))", "(?=(?=(?=a)))a", "(?<=(?<=ab))c",
            "(*LIMIT_MATCH=5)a", "(*LIMIT_MATCH=1000)a", "(*LIMIT_DEPTH=2)(?>(?>a))",
            "(*LIMIT_DEPTH=1000)(?>(?>a))",
        ];
        for pat in lim_pats {
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for subj in ["", "a", "abc", "aaab", "aaac", "(((a)))", "bbbb"] {
                let sv = subj.as_bytes();
                for ml in [1u32, 2, 3, 5, 8, 11, 20, 100, 100_000] {
                    for dl in [1u32, 2, 3, 4, 8, 2_000] {
                        let mca = (p.c.match_context_create)(ptr::null_mut());
                        let mcb = (p.r.match_context_create)(ptr::null_mut());
                        for (api, v) in [(&p.c, mca), (&p.r, mcb)] {
                            assert_eq!((api.set_match_limit)(v, ml), 0);
                            assert_eq!((api.set_depth_limit)(v, dl), 0);
                            assert_eq!((api.set_heap_limit)(v, 4_000), 0);
                        }
                        let g = Dfa { ovec: 4, ..Dfa::new() };
                        let tag = format!("row315 /{pat}/ subj={} ml={ml} dl={dl}", show(sv));
                        dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, (mca, mcb), &tag, &mut d);
                        (p.c.match_context_free)(mca);
                        (p.r.match_context_free)(mcb);
                    }
                }
                // and with a NULL match context, i.e. the built-in defaults
                let g = Dfa { ovec: 4, ..Dfa::new() };
                let tag = format!("row315 /{pat}/ subj={} default limits", show(sv));
                dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, (ptr::null_mut(), ptr::null_mut()), &tag, &mut d);
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("CONFIGS 315: dfa match_limit counted over all bumpalongs and depth_limit at limit+1, swept across the crossover, incl. (*LIMIT_*)");
}

// ======================== rows 316-317: RWS growth and nested wscount

#[test]
fn cfg_316_317_dfa_rws() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let cfg = Cfg::new("default", 0);
        // ---- row 316: 7 nested assertions fit in the 7676-int base block; the
        // 8th needs more_workspace, which mallocs a doubled 60 KiB block unless
        // the heap limit clamps it.  Each assertion frame costs
        // RWS_RSIZE + RWS_OVEC_OSIZE = 1004 ints.
        let mut nested = Vec::new();
        for n in 1..=10usize {
            let mut s = String::new();
            for _ in 0..n {
                s.push_str("(?=");
            }
            s.push('a');
            for _ in 0..n {
                s.push(')');
            }
            nested.push((n, s));
        }
        for (n, pat) in &nested {
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            let mut first_ok = None;
            for hl in [0u32, 1, 2, 3, 4, 5, 8, 16, 30, 59, 60, 61, 120, 4_000] {
                let mca = (p.c.match_context_create)(ptr::null_mut());
                let mcb = (p.r.match_context_create)(ptr::null_mut());
                for (api, v) in [(&p.c, mca), (&p.r, mcb)] {
                    assert_eq!((api.set_heap_limit)(v, hl), 0);
                    assert_eq!((api.set_match_limit)(v, 200_000), 0);
                    assert_eq!((api.set_depth_limit)(v, 2_000), 0);
                }
                let g = Dfa { ovec: 4, ..Dfa::new() };
                let tag = format!("row316 {n} nested assertions heap_limit={hl}");
                let (ra, _) = dfa_cmp(p, a, b, b"a".as_ptr(), 1, &g, (mca, mcb), &tag, &mut d);
                if ra >= 0 && first_ok.is_none() {
                    first_ok = Some(hl);
                }
                // second run on the same context reuses the cached rws->next
                let tag = format!("row316 {n} nested assertions heap_limit={hl} (rerun, cached rws)");
                dfa_cmp(p, a, b, b"a".as_ptr(), 1, &g, (mca, mcb), &tag, &mut d);
                (p.c.match_context_free)(mca);
                (p.r.match_context_free)(mcb);
            }
            println!("row 316: {n} nested assertions first succeed at heap_limit {first_ok:?} KiB");
            if *n <= 7 {
                assert_eq!(
                    first_ok, Some(0),
                    "row 316: {n} assertion frames must fit in the base RWS block"
                );
            }
            if *n == 8 {
                assert_eq!(
                    first_ok, Some(4),
                    "row 316: the 8th assertion frame needs a heap block, clamped at 4 KiB"
                );
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // ---- row 316, recursion arm: each recursion frame costs
        // RWS_RSIZE + RWS_OVEC_RSIZE = 3000 ints, so only 2 fit in the base
        // block and the clamp threshold is higher.
        let rec = "\\((?:[^()]++|(?R))*\\)";
        let (a, b) = compile_ok(p, rec.as_bytes(), &cfg);
        for depth in 1..=5usize {
            let mut sv = Vec::new();
            for _ in 0..depth {
                sv.push(b'(');
            }
            sv.push(b'a');
            for _ in 0..depth {
                sv.push(b')');
            }
            let mut first_ok = None;
            for hl in [0u32, 4, 8, 11, 12, 13, 16, 24, 30, 36, 48, 59, 60, 61, 120, 4_000] {
                let mca = (p.c.match_context_create)(ptr::null_mut());
                let mcb = (p.r.match_context_create)(ptr::null_mut());
                for (api, v) in [(&p.c, mca), (&p.r, mcb)] {
                    assert_eq!((api.set_heap_limit)(v, hl), 0);
                    assert_eq!((api.set_match_limit)(v, 200_000), 0);
                    assert_eq!((api.set_depth_limit)(v, 2_000), 0);
                }
                let g = Dfa { ovec: 4, ..Dfa::new() };
                let tag = format!("row316 recursion depth={depth} heap_limit={hl}");
                let (ra, _) = dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, (mca, mcb), &tag, &mut d);
                if ra >= 0 && first_ok.is_none() {
                    first_ok = Some(hl);
                }
                let tag = format!("row316 recursion depth={depth} heap_limit={hl} (rerun)");
                dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, (mca, mcb), &tag, &mut d);
                (p.c.match_context_free)(mca);
                (p.r.match_context_free)(mcb);
            }
            println!("row 316: recursion depth {depth} first succeeds at heap_limit {first_ok:?} KiB");
        }
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // ---- row 317: nested calls always get wscount = 1000 ints, i.e. 166
        // states, whatever the caller passed.  An assertion body with more than
        // 166 simultaneous states must therefore fail with DFA_WSSIZE even with
        // an enormous outer workspace.
        for nbranch in [8usize, 100, 166, 167, 200, 400] {
            let branches: Vec<String> = (1..=nbranch).map(|i| "a".repeat(i)).collect();
            let pat = format!("(?=(?:{}))a*", branches.join("|"));
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for wsn in [20usize, 1000, 100_000] {
                let mca = (p.c.match_context_create)(ptr::null_mut());
                let mcb = (p.r.match_context_create)(ptr::null_mut());
                for (api, v) in [(&p.c, mca), (&p.r, mcb)] {
                    assert_eq!((api.set_match_limit)(v, 200_000), 0);
                    assert_eq!((api.set_depth_limit)(v, 2_000), 0);
                    assert_eq!((api.set_heap_limit)(v, 4_000), 0);
                }
                let sv = "a".repeat(nbranch.min(200));
                let g = Dfa { wsn, ovec: 4, ..Dfa::new() };
                let tag = format!("row317 {nbranch} assertion branches, outer wscount={wsn}");
                let (ra, _) = dfa_cmp(p, a, b, sv.as_bytes().as_ptr(), sv.len(), &g, (mca, mcb), &tag, &mut d);
                println!("row 317: {nbranch} branches, outer wscount {wsn} => rc {ra}");
                if nbranch > 166 && wsn >= 1000 {
                    assert_eq!(
                        ra, PCRE2_ERROR_DFA_WSSIZE,
                        "row 317: the nested call is capped at 166 states regardless of wscount={wsn}"
                    );
                }
                (p.c.match_context_free)(mca);
                (p.r.match_context_free)(mcb);
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("CONFIGS 316-317: dfa RWS growth and heap_limit clamp for assertion and recursion frames, and the fixed 1000-int nested wscount");
}

// ============================ row 318: duplicate-state suppression

#[test]
fn cfg_318_dfa_duplicate_states() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(31_801);
    unsafe {
        let m = bounded_mctx(p);
        let cfg = Cfg::new("default", 0);
        // The exact configuration the row names must terminate, not loop.
        let (a, b) = compile_ok(p, b"(a*)*b", &cfg);
        let g = Dfa { ovec: 4, ..Dfa::new() };
        let (ra, _) = dfa_cmp(p, a, b, b"aaac".as_ptr(), 4, &g, m, "row318 /(a*)*b/ over aaac", &mut d);
        assert_eq!(ra, PCRE2_ERROR_NOMATCH, "row 318: /(a*)*b/ over aaac must terminate with NOMATCH");
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // Nested-empty-repeat shapes, all of which rely on the duplicate-state
        // suppression to terminate.
        let dup_pats: &[&str] = &[
            "(a*)*b", "(a*)+b", "(a?)*b", "(|a)*b", "(a|)*b", "((a*)*)*b",
            "(?:a*)*b", "(?:|a)+b", "(\\d*)*x", "([a-c]*)*z", "(a*|b*)*c",
            "(?:(?:a*)*)*b", "()*b", "(?:)*b", "(a{0,3})*b",
        ];
        for pat in dup_pats {
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for subj in ["", "a", "aa", "aaa", "aaac", "aaab", "b", "c", "z", "x", "abc", "12345x"] {
                let sv = subj.as_bytes();
                for &o in &[0u32, PCRE2_DFA_SHORTEST, PCRE2_ANCHORED, PCRE2_PARTIAL_SOFT] {
                    for wsn in [20usize, 100, 1000] {
                        let g = Dfa { opts: o, wsn, ovec: 4, ..Dfa::new() };
                        let tag = format!("row318 /{pat}/ subj={} opts={o:#x} ws={wsn}", show(sv));
                        dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                    }
                }
            }
            for _ in 0..10 {
                let sv = gen_ascii(&mut rng, 14);
                let g = Dfa { ovec: 4, ..Dfa::new() };
                let tag = format!("row318 fuzz /{pat}/ subj={}", show(&sv));
                dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 318: dfa duplicate-state suppression makes nested empty repeats terminate");
}

// ======================================= rows 319-320: newlines and \R

#[test]
fn cfg_319_320_dfa_newlines() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let m = bounded_mctx(p);
        const NLS: &[(u32, &str)] = &[
            (PCRE2_NEWLINE_CR, "CR"),
            (PCRE2_NEWLINE_LF, "LF"),
            (PCRE2_NEWLINE_CRLF, "CRLF"),
            (PCRE2_NEWLINE_ANY, "ANY"),
            (PCRE2_NEWLINE_ANYCRLF, "ANYCRLF"),
            (PCRE2_NEWLINE_NUL, "NUL"),
        ];
        // row 319: the constructs the row names.
        let nl_pats: &[(&str, u32)] = &[
            (".", 0),
            (".+", 0),
            (".*", 0),
            ("a.b", 0),
            ("a$", 0),
            ("a$", PCRE2_MULTILINE),
            ("^a", PCRE2_MULTILINE),
            ("a\\Z", 0),
            ("a\\z", 0),
            ("^.*$", PCRE2_MULTILINE),
            ("a$", PCRE2_DOLLAR_ENDONLY),
            ("\\N", 0),
            ("\\N+", 0),
        ];
        let subj = "a\rb\nc\r\nd\x0be";
        for &(nl, nlname) in NLS {
            for &(pat, copts) in nl_pats {
                let cfg = Cfg::nl("nl", copts, nl);
                let pb = pat.as_bytes();
                let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
                for sv in [
                    subj.as_bytes(), b"a\rb", b"a\nb", b"a\r\nb", b"a\0b", b"a\x0bb",
                    b"a", b"", b"\r\n", b"\r", b"\n", b"ab\r\ncd",
                ] {
                    for start in 0..=sv.len() {
                        for &o in &[0u32, PCRE2_NOTEOL, PCRE2_NOTBOL, PCRE2_ANCHORED] {
                            let g = Dfa { start, opts: o, ovec: 4, ..Dfa::new() };
                            let tag = format!(
                                "row319 /{pat}/ copts={copts:#x} nl={nlname} subj={} start={start} opts={o:#x}",
                                show(sv)
                            );
                            dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                        }
                    }
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
        // The row's explicit claim: under CRLF, `.` excludes only a CR that is
        // FOLLOWED by an LF.
        {
            let cfg = Cfg::nl("CRLF", 0, PCRE2_NEWLINE_CRLF);
            let (a, b) = compile_ok(p, b".", &cfg);
            let mda = (p.c.match_data_create)(2, ptr::null_mut());
            let mut wa = vec![0 as c_int; 1000];
            // a lone CR IS matched by `.` under CRLF
            let ra = (p.c.dfa_match)(a, b"\rx".as_ptr(), 2, 0, PCRE2_ANCHORED, mda, m.0, wa.as_mut_ptr(), 1000);
            assert_eq!(ra, 1, "row 319: under CRLF a lone CR is an ordinary character for `.`");
            (p.c.match_data_free)(mda);
            let mda = (p.c.match_data_create)(2, ptr::null_mut());
            let ra = (p.c.dfa_match)(a, b"\r\n".as_ptr(), 2, 0, PCRE2_ANCHORED, mda, m.0, wa.as_mut_ptr(), 1000);
            assert_eq!(ra, PCRE2_ERROR_NOMATCH, "row 319: under CRLF a CR followed by LF is excluded from `.`");
            (p.c.match_data_free)(mda);
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }

        // ---- row 320: \R under both \R conventions, single and quantified,
        // independent of the newline convention.
        let r_pats: &[&str] = &[
            "\\R", "\\R+", "\\R*", "\\R?", "\\R{2}", "\\R{1,3}", "\\R+?", "\\R++",
            "a\\Rb", "\\R\\R", "(*BSR_ANYCRLF)\\R", "(*BSR_UNICODE)\\R", "[\\R]?a",
        ];
        for &(nl, nlname) in NLS {
            for bsr in [PCRE2_BSR_UNICODE, PCRE2_BSR_ANYCRLF] {
                for pat in r_pats {
                    let cfg = Cfg { name: "bsr", opts: 0, xopts: 0, newline: nl, bsr };
                    let pb = pat.as_bytes();
                    let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
                    let cfgu = Cfg { name: "bsr|UTF", opts: PCRE2_UTF, xopts: 0, newline: nl, bsr };
                    let Some((au, bu)) = compile_both(p, pb, &cfgu, &mut d) else {
                        (p.c.code_free)(a);
                        (p.r.code_free)(b);
                        continue;
                    };
                    for sv in [
                        &b"\r\n"[..], b"\n", b"\r", b"\x0b", b"\x0c", b"\r\r", b"\n\n",
                        b"\r\n\r\n", b"a\r\nb", b"", b"a", b"\xc2\x85", b"\xe2\x80\xa8",
                        b"\r\n\n\r", b"\x0b\x0c\n",
                    ] {
                        for start in 0..=sv.len() {
                            let g = Dfa { start, ovec: 4, ..Dfa::new() };
                            let tag = format!(
                                "row320 /{pat}/ bsr={bsr} nl={nlname} subj={} start={start}",
                                show(sv)
                            );
                            dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                            if std::str::from_utf8(sv).is_ok() {
                                let tag = format!(
                                    "row320 UTF /{pat}/ bsr={bsr} nl={nlname} subj={} start={start}",
                                    show(sv)
                                );
                                if start == sv.len() || (sv[start] & 0xc0) != 0x80 {
                                    dfa_cmp(p, au, bu, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
                                }
                            }
                        }
                    }
                    (p.c.code_free)(a);
                    (p.r.code_free)(b);
                    (p.c.code_free)(au);
                    (p.r.code_free)(bu);
                }
            }
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 319-320: dfa newline handling under all 6 conventions, and \\R under both conventions independent of them");
}

// ===================================== row 321: leftchar / rightchar

#[test]
fn cfg_321_dfa_leftchar_rightchar() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(32_101);
    unsafe {
        let m = bounded_mctx(p);
        let cfg = Cfg::new("default", 0);
        // The exact configuration the row names.
        let (a, b) = compile_ok(p, b"(?<=ab)\\bc", &cfg);
        for (api, code) in [(&p.c, a), (&p.r, b)] {
            let md = (api.match_data_create)(4, ptr::null_mut());
            let mut w = vec![0 as c_int; 1000];
            let rc = (api.dfa_match)(code, b"abcd".as_ptr(), 4, 2, 0, md, m.0, w.as_mut_ptr(), 1000);
            assert_eq!(rc, 1, "[{}] row 321: /(?<=ab)\\bc/ at offset 2 must match", api.name);
            let h = md_head(api, md, rc);
            println!(
                "[{}] row 321: leftchar={} rightchar={} startchar={}",
                api.name, h.leftchar, h.rightchar, h.startchar
            );
            assert_eq!(h.leftchar, 0, "[{}] row 321: the lookbehind pushes leftchar back to 0", api.name);
            assert_eq!(h.startchar, 2, "[{}] row 321: startchar is the match start", api.name);
            (api.match_data_free)(md);
        }
        (p.c.code_free)(a);
        (p.r.code_free)(b);

        // Compare leftchar/rightchar wherever the C defines them (rc >= 0 or
        // PARTIAL), over patterns that push start_used_ptr / last_used_ptr
        // around.  For NOMATCH, prove they are LEFT ALONE: prime the block with
        // a successful match first, then re-run into a NOMATCH and assert the
        // values are unchanged in both libraries.
        let lc_pats: &[(&str, u32)] = &[
            ("(?<=ab)\\bc", 0),
            ("(?<=ab)c", 0),
            ("\\bc", 0),
            ("\\Bc", 0),
            ("c", 0),
            ("(?<=a{1,3})c", 0),
            ("(?=cd)c", 0),
            ("c(?=d)", 0),
            ("(?<!x)c", 0),
            ("\\w+", 0),
            ("(?<=\\X)c", PCRE2_UTF),
            ("(?<=ab|xy)c", 0),
        ];
        for &(pat, copts) in lc_pats {
            let cfg = Cfg::new("leftright", copts);
            let pb = pat.as_bytes();
            let Some((a, b)) = compile_both(p, pb, &cfg, &mut d) else { continue };
            for subj in ["abcd", "xycd", "abc", "cd", "c", "zzc", "aaac", "\u{e9}cd", "abcabc"] {
                let sv = subj.as_bytes();
                if copts & PCRE2_UTF != 0 && std::str::from_utf8(sv).is_err() {
                    continue;
                }
                for start in 0..=sv.len() {
                    if copts & PCRE2_UTF != 0 && start < sv.len() && (sv[start] & 0xc0) == 0x80 {
                        continue;
                    }
                    for &o in &[0u32, PCRE2_PARTIAL_SOFT, PCRE2_ANCHORED] {
                        let mda = (p.c.match_data_create)(4, ptr::null_mut());
                        let mdb = (p.r.match_data_create)(4, ptr::null_mut());
                        let mut wa = vec![0 as c_int; 1000];
                        let mut wb = vec![0 as c_int; 1000];
                        let ra = (p.c.dfa_match)(a, sv.as_ptr(), sv.len(), start, o, mda, m.0, wa.as_mut_ptr(), 1000);
                        let rb = (p.r.dfa_match)(b, sv.as_ptr(), sv.len(), start, o, mdb, m.1, wb.as_mut_ptr(), 1000);
                        let tag = format!("row321 /{pat}/ subj={} start={start} opts={o:#x}", show(sv));
                        d.eq(&tag,
                            read_match_out_of(&p.c, mda, ra, Engine::Dfa),
                            read_match_out_of(&p.r, mdb, rb, Engine::Dfa));
                        if ra == rb && (ra >= 0 || ra == PCRE2_ERROR_PARTIAL) {
                            let (ha, hb) = (md_head(&p.c, mda, ra), md_head(&p.r, mdb, rb));
                            d.eq(
                                &format!("{tag} :: leftchar/rightchar"),
                                (ha.leftchar, ha.rightchar),
                                (hb.leftchar, hb.rightchar),
                            );
                            d.eq(
                                &format!("{tag} :: subject_length/start_offset"),
                                (ha.subject_length, ha.start_offset),
                                (hb.subject_length, hb.start_offset),
                            );
                            // NOMATCH must leave leftchar/rightchar untouched
                            let la = (ha.leftchar, ha.rightchar);
                            let lb = (hb.leftchar, hb.rightchar);
                            let na = (p.c.dfa_match)(a, b"QQQQ".as_ptr(), 4, 0, PCRE2_ANCHORED, mda, m.0, wa.as_mut_ptr(), 1000);
                            let nb = (p.r.dfa_match)(b, b"QQQQ".as_ptr(), 4, 0, PCRE2_ANCHORED, mdb, m.1, wb.as_mut_ptr(), 1000);
                            if na == PCRE2_ERROR_NOMATCH && nb == PCRE2_ERROR_NOMATCH {
                                let (ha2, hb2) = (md_head(&p.c, mda, na), md_head(&p.r, mdb, nb));
                                assert_eq!(
                                    (ha2.leftchar, ha2.rightchar), la,
                                    "row 321: NOMATCH must not touch leftchar/rightchar (C)"
                                );
                                d.eq(
                                    &format!("{tag} :: leftchar/rightchar after NOMATCH"),
                                    (ha2.leftchar, ha2.rightchar),
                                    (hb2.leftchar, hb2.rightchar),
                                );
                                assert_eq!((hb2.leftchar, hb2.rightchar), lb, "row 321: rust NOMATCH must not touch them either");
                            }
                        }
                        (p.c.match_data_free)(mda);
                        (p.r.match_data_free)(mdb);
                    }
                }
            }
            for _ in 0..8 {
                let sv = gen_ascii(&mut rng, 10);
                let start = if sv.is_empty() { 0 } else { rng.below(sv.len() + 1) };
                let g = Dfa { start, ovec: 4, ..Dfa::new() };
                let tag = format!("row321 fuzz /{pat}/ subj={} start={start}", show(&sv));
                dfa_cmp(p, a, b, sv.as_ptr(), sv.len(), &g, m, &tag, &mut d);
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
        free_mctx(p, m);
    }
    d.finish("CONFIGS 321: dfa leftchar/rightchar from start_used_ptr/last_used_ptr, pushed back by lookbehind and \\b, untouched by NOMATCH");
}
