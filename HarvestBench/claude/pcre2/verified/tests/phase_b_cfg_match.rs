// Phase B sign-off for CONFIGS.md rows 154-293 — the whole of
//   "### 2. match — pcre2_match_8 (+ pcre2_match_data_create*_8,
//    pcre2_get_*_8, pcre2_next_match_8)".
//
// Every row drives its exact named configuration (compile options + compile
// context state + match options + match context state + subject shape) through
// BOTH `.so`s and compares:
//
//   * the compiled bytecode (`assert_code_eq`),
//   * everything `read_match_out` defines for the returned code,
//   * every derived accessor: `pcre2_get_ovector_count_8`,
//     `pcre2_get_ovector_pointer_8`, `pcre2_get_startchar_8`,
//     `pcre2_get_mark_8`, `pcre2_get_match_data_size_8`,
//     `pcre2_get_match_data_heapframes_size_8`, `pcre2_next_match_8`,
//   * and the whole of the `pcre2_real_match_data` block, which is made
//     deterministic by pre-filling every field the matcher may leave stale
//     with the same sentinel in both libraries (that is what makes rows such
//     as 219 "slots are left untouched" and 287 "stale fields" observable at
//     all).  The layout used to do that is self-checked against the public
//     accessors on every single use (see `mdh`).
//
// Where a row names an expected outcome explicitly it is also asserted
// against the C (`want(...)`), so a mis-derived row shows up as a failure
// rather than silently passing.

mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

pub const COVERAGE: &[CfgCov] = &[
    CfgCov { cfg_rows: &[154], note: "baseline /abc/ on xxabcxx: rc/ovector/startchar/leftchar/rightchar/mark/matchedby" },
    CfgCov { cfg_rows: &[155], note: "subject NULL + length 0 => null_str; subject restored to NULL" },
    CfgCov { cfg_rows: &[156], note: "PCRE2_ZERO_TERMINATED resolved before the startoffset check" },
    CfgCov { cfg_rows: &[157], note: "startoffset == length: empty match via lookbehind vs NOMATCH" },
    CfgCov { cfg_rows: &[158], note: "startoffset mid-subject with max_lookbehind 2 (check_subject rewind)" },
    CfgCov { cfg_rows: &[159], note: "(*NOTEMPTY)/(*NOTEMPTY_ATSTART) vs the option bits; match_data->options" },
    CfgCov { cfg_rows: &[160], note: "NOTEMPTY vs NOTEMPTY_ATSTART on /a*/ over bbb" },
    CfgCov { cfg_rows: &[161], note: "NOTEMPTY_ATSTART with startoffset 2 on /a*/ xxyy" },
    CfgCov { cfg_rows: &[162], note: "PCRE2_NOTBOL with /^abc/ and /^abc/m" },
    CfgCov { cfg_rows: &[163], note: "PCRE2_NOTEOL with /abc$/ and /abc$/m" },
    CfgCov { cfg_rows: &[164], note: "PCRE2_DOLLAR_ENDONLY: $ behaves as \\z" },
    CfgCov { cfg_rows: &[165], note: "PCRE2_ALT_CIRCUMFLEX: ^ at end_subject under MULTILINE" },
    CfgCov { cfg_rows: &[166], note: "MATCH_UNSET_BACKREF /(a)?\\1b/ on b" },
    CfgCov { cfg_rows: &[167], note: "MATCH_UNSET_BACKREF repeated ref /(a)?\\1{2,3}b/" },
    CfgCov { cfg_rows: &[168], note: "PCRE2_ANCHORED at match time vs compile time" },
    CfgCov { cfg_rows: &[169], note: "PCRE2_ENDANCHORED at match time; hard return after (*ACCEPT)" },
    CfgCov { cfg_rows: &[170], note: "mb->partial selection: SOFT / HARD / both (HARD wins)" },
    CfgCov { cfg_rows: &[171], note: "PARTIAL_SOFT: only pair 0 written, capture slots untouched" },
    CfgCov { cfg_rows: &[172], note: "PARTIAL_HARD beats a complete match at \\z" },
    CfgCov { cfg_rows: &[173], note: "PARTIAL_SOFT with max_lookbehind > 0 (allowemptypartial)" },
    CfgCov { cfg_rows: &[174], note: "PARTIAL_HARD + NEWLINE_CRLF, lone \\r in OP_ANY" },
    CfgCov { cfg_rows: &[175], note: "PARTIAL_HARD + NEWLINE_CRLF, OP_EODN CRLF-split arm" },
    CfgCov { cfg_rows: &[176], note: "partial disables the minlength / req_cu optimizations" },
    CfgCov { cfg_rows: &[177], note: "PCRE2_NO_JIT accepted, byte-identical to options 0" },
    CfgCov { cfg_rows: &[178], note: "COPY_MATCHED_SUBJECT on success; original subject freed" },
    CfgCov { cfg_rows: &[179], note: "COPY_MATCHED_SUBJECT with length 0: subject NULL, flag set" },
    CfgCov { cfg_rows: &[180], note: "COPY_MATCHED_SUBJECT twice on one match_data; then without" },
    CfgCov { cfg_rows: &[181], note: "COPY_MATCHED_SUBJECT on partial and on nomatch: no copy" },
    CfgCov { cfg_rows: &[182], note: "PCRE2_DISABLE_RECURSELOOP_CHECK vs PCRE2_ERROR_RECURSELOOP" },
    CfgCov { cfg_rows: &[183], note: "USE_OFFSET_LIMIT + offset_limit, strict > boundary sweep" },
    CfgCov { cfg_rows: &[184], note: "offset_limit still applies under NO_START_OPTIMIZE" },
    CfgCov { cfg_rows: &[185], note: "offset_limit == PCRE2_UNSET without USE_OFFSET_LIMIT is legal" },
    CfgCov { cfg_rows: &[186], note: "mcontext NULL vs default; which memctl the heapframes come from" },
    CfgCov { cfg_rows: &[187], note: "compile-time NO_START_OPTIMIZE: callout at every bumpalong" },
    CfgCov { cfg_rows: &[188], note: "anchored + has_first_cu single-position pre-check" },
    CfgCov { cfg_rows: &[189], note: "anchored + start_bits single-position pre-check" },
    CfgCov { cfg_rows: &[190], note: "unanchored caseful first-CU memchr over a 4 KiB subject" },
    CfgCov { cfg_rows: &[191], note: "unanchored caseless dual memchr, all 5 orderings" },
    CfgCov { cfg_rows: &[192], note: "caseless dual-memchr cache-hit arms across bumpalongs" },
    CfgCov { cfg_rows: &[193], note: "caseless first CU > 127 with UCP and no UTF (UCD_OTHERCASE)" },
    CfgCov { cfg_rows: &[194], note: "startline bump: WAS_NEWLINE scan + the CR/LF fudge" },
    CfgCov { cfg_rows: &[195], note: "start_bits bitmap scan (no first CU, no startline)" },
    CfgCov { cfg_rows: &[196], note: "strict precedence has_first_cu > startline > start_bits" },
    CfgCov { cfg_rows: &[197], note: "re->minlength cut, and minlength as code units under UTF" },
    CfgCov { cfg_rows: &[198], note: "req_cu caseful + the REQ_CU_MAX / *1000 / anchored windows" },
    CfgCov { cfg_rows: &[199], note: "req_cu caseless second memchr; has_first_cu skip of one unit" },
    CfgCov { cfg_rows: &[200], note: "req_cu_ptr monotonic cache across bumpalong iterations" },
    CfgCov { cfg_rows: &[201], note: "PCRE2_FIRSTLINE end_subject clamp, non-UTF and UTF loops" },
    CfgCov { cfg_rows: &[202], note: "FIRSTLINE + PARTIAL_SOFT: the attempt at the clamped end runs" },
    CfgCov { cfg_rows: &[203], note: "FIRSTLINE + IS_NEWLINE(start_match) stops the bumpalong" },
    CfgCov { cfg_rows: &[204], note: "CRLF bumpalong skip x HASCRORLF x all newline conventions" },
    CfgCov { cfg_rows: &[205], note: "UTF bumpalong advances whole characters (ACROSSCHAR)" },
    CfgCov { cfg_rows: &[206], note: "start_match == end_subject is still attempted" },
    CfgCov { cfg_rows: &[207], note: "UTF check runs over check_subject..end (NO_UTF_CHECK clear)" },
    CfgCov { cfg_rows: &[208], note: "PCRE2_NO_UTF_CHECK: check skipped, check_subject == subject" },
    CfgCov { cfg_rows: &[209], note: "MATCH_INVALID_UTF overrides NO_UTF_CHECK" },
    CfgCov { cfg_rows: &[210], note: "invalid-UTF fragment loop: NOTEOL on the first fragment" },
    CfgCov { cfg_rows: &[211], note: "two bad bytes: middle fragment NOTBOL|NOTEOL, FRAGMENT_RESTART" },
    CfgCov { cfg_rows: &[212], note: "startoffset mid-character: skipped_bad_start suppresses the rewind" },
    CfgCov { cfg_rows: &[213], note: "\\A / \\z / \\Z / \\G / \\b / lookbehind across fragments" },
    CfgCov { cfg_rows: &[214], note: "MATCH_INVALID_UTF + PARTIAL_SOFT: non-final partial discarded" },
    CfgCov { cfg_rows: &[215], note: "match_data_create clamping, ovector pointer/size formulas, free" },
    CfgCov { cfg_rows: &[216], note: "match_data_create_from_pattern: pairs and allocator source" },
    CfgCov { cfg_rows: &[217], note: "oveccount 1 on /(a)(b)/ => rc 0" },
    CfgCov { cfg_rows: &[218], note: "oveccount == top_bracket+1 => rc 3" },
    CfgCov { cfg_rows: &[219], note: "oversized oveccount: trailing slots left stale" },
    CfgCov { cfg_rows: &[220], note: "oveccount between: truncation and the rc 0 boundary" },
    CfgCov { cfg_rows: &[221], note: "non-participating group => PCRE2_UNSET from the 0xff memset" },
    CfgCov { cfg_rows: &[222], note: "groups above Foffset_top set UNSET by the while(--i) loop" },
    CfgCov { cfg_rows: &[223], note: "rc driven by end_offset_top, not top_bracket" },
    CfgCov { cfg_rows: &[224], note: "\\K moves ovector[0] but not startchar" },
    CfgCov { cfg_rows: &[225], note: "(*MARK) on success, on NOMATCH and on a hard error" },
    CfgCov { cfg_rows: &[226], note: "(*SKIP:x) does not set nomatch_mark; one case per verb" },
    CfgCov { cfg_rows: &[227], note: "heapframes growth across 0-capture / 200-capture reuse" },
    CfgCov { cfg_rows: &[228], note: "heap_limit clamps the initial heapframes vector" },
    CfgCov { cfg_rows: &[229], note: "heap_limit sweep across the in-match growth crossover" },
    CfgCov { cfg_rows: &[230], note: "match_limit: context / (*LIMIT_MATCH) / min-wins, swept" },
    CfgCov { cfg_rows: &[231], note: "depth_limit / (*LIMIT_DEPTH) / set_recursion_limit, swept" },
    CfgCov { cfg_rows: &[232], note: "heap_limit / (*LIMIT_HEAP) 5-way matrix, swept" },
    CfgCov { cfg_rows: &[233], note: "set_recursion_memory_management is a no-op" },
    CfgCov { cfg_rows: &[234], note: "match_ref caseless UTF/UCP, CASELESS_RESTRICT, TURKISH_CASING" },
    CfgCov { cfg_rows: &[235], note: "match_ref caseful memcmp vs the partial unit loop" },
    CfgCov { cfg_rows: &[236], note: "match_ref returning a partial, incl. the maximizing loop" },
    CfgCov { cfg_rows: &[237], note: "OP_DNREF / OP_DNREFI with DUPNAMES" },
    CfgCov { cfg_rows: &[238], note: "backref repeat forms CRSTAR..CRPOSSTAR" },
    CfgCov { cfg_rows: &[239], note: "zero-length set-group backref continue guard /()\\1*x/" },
    CfgCov { cfg_rows: &[240], note: "backref maximizing samelengths vs the caseless-UTF rescan" },
    CfgCov { cfg_rows: &[241], note: "OP_RECURSE whole pattern (?R)" },
    CfgCov { cfg_rows: &[242], note: "OP_RECURSE group recursion; captures not propagated out" },
    CfgCov { cfg_rows: &[243], note: "recurse-loop check: same/advanced position, other number" },
    CfgCov { cfg_rows: &[244], note: "recursion with a capture list (recurse_update_offsets)" },
    CfgCov { cfg_rows: &[245], note: "(*ACCEPT) inside a recursion walks back to GF_RECURSE" },
    CfgCov { cfg_rows: &[246], note: "verb containment in a recursion (verb_current_recurse)" },
    CfgCov { cfg_rows: &[247], note: "atomic group OP_ONCE discards alternatives" },
    CfgCov { cfg_rows: &[248], note: "possessive group family BRAPOS/CBRAPOS/SBRAPOS/KETRPOS" },
    CfgCov { cfg_rows: &[249], note: "possessive quantifiers with no backtracking frame" },
    CfgCov { cfg_rows: &[250], note: "fixed lookbehind OP_REVERSE, both floors" },
    CfgCov { cfg_rows: &[251], note: "variable lookbehind OP_VREVERSE retry loop and clamps" },
    CfgCov { cfg_rows: &[252], note: "variable-lookbehind end-point verification in all 4 sites" },
    CfgCov { cfg_rows: &[253], note: "\\X single (OP_EXTUNI) over the grapheme corpus" },
    CfgCov { cfg_rows: &[254], note: "\\X min/max repeat with backtracking" },
    CfgCov { cfg_rows: &[255], note: "(*script_run:) / (*sr:) / (*asr:)" },
    CfgCov { cfg_rows: &[256], note: "OP_XCLASS single / min / max repeat with \\C" },
    CfgCov { cfg_rows: &[257], note: "OP_ECLASS single / min / max repeat" },
    CfgCov { cfg_rows: &[258], note: "OP_CLASS vs OP_NCLASS >255 handling, UTF and 8-bit" },
    CfgCov { cfg_rows: &[259], note: "numeric callout block: every field, offset_vector UNSET inside" },
    CfgCov { cfg_rows: &[260], note: "string callout block: offset / length / pointer" },
    CfgCov { cfg_rows: &[261], note: "callout return 0 / >0 / <0" },
    CfgCov { cfg_rows: &[262], note: "callout_flags STARTMATCH and BACKTRACK" },
    CfgCov { cfg_rows: &[263], note: "callouts in the pattern with no callout function installed" },
    CfgCov { cfg_rows: &[264], note: "AUTO_CALLOUT full log; subject_length is the fragment length" },
    CfgCov { cfg_rows: &[265], note: "AUTO_CALLOUT conditional group (the Llength -= length fix-up)" },
    CfgCov { cfg_rows: &[266], note: "(*COMMIT) disables the bumpalong" },
    CfgCov { cfg_rows: &[267], note: "(*PRUNE) advances one character" },
    CfgCov { cfg_rows: &[268], note: "(*SKIP) verb_skip_ptr ahead vs not ahead" },
    CfgCov { cfg_rows: &[269], note: "(*SKIP:name) + (*MARK:name): MATCH_SKIP_ARG retry" },
    CfgCov { cfg_rows: &[270], note: "(*THEN) in a group, at top level, and inside an assertion" },
    CfgCov { cfg_rows: &[271], note: "(*ACCEPT) at top level and inside an assertion" },
    CfgCov { cfg_rows: &[272], note: "\\A \\G \\Z \\z with startoffset > 0" },
    CfgCov { cfg_rows: &[273], note: "\\K pushed before startoffset: BAD_BACKSLASH_K vs ALLOW_LOOKAROUND_BSK" },
    CfgCov { cfg_rows: &[274], note: "\\b / \\B, check_subject floor, and the UCP word-boundary opcode" },
    CfgCov { cfg_rows: &[275], note: ". under all 6 newline conventions x DOTALL" },
    CfgCov { cfg_rows: &[276], note: "^ and $ multiline under all 6 newline conventions" },
    CfgCov { cfg_rows: &[277], note: "\\R under both BSR conventions, all repeat forms" },
    CfgCov { cfg_rows: &[278], note: "8-bit non-UTF \\R max-repeat does not test 0x2028/0x2029" },
    CfgCov { cfg_rows: &[279], note: "\\C in UTF lands mid-character; quantified; non-UTF ALLANY" },
    CfgCov { cfg_rows: &[280], note: "\\h \\H \\v \\V single and repeat, UTF and 8-bit" },
    CfgCov { cfg_rows: &[281], note: "\\p / \\P for every PT_* type, single/min/max repeat" },
    CfgCov { cfg_rows: &[282], note: "assertions incl. (*napla:)/(*naplb:) and the negative 4-way switch" },
    CfgCov { cfg_rows: &[283], note: "(*scs:) group set / unset / DUPNAMES name list" },
    CfgCov { cfg_rows: &[284], note: "match-time conditions: RREF/DNRREF/CREF/DNCREF/FALSE/TRUE/assert" },
    CfgCov { cfg_rows: &[285], note: "OP_KET infinite-loop guard /(a*)*b/" },
    CfgCov { cfg_rows: &[286], note: "OP_BRA fast path vs mb->hasthen: match_limit crossover differs" },
    CfgCov { cfg_rows: &[287], note: "hard error leaves subject NULL and the other fields stale" },
    CfgCov { cfg_rows: &[288], note: "next_match after a non-empty match" },
    CfgCov { cfg_rows: &[289], note: "next_match after an empty match, not at end and at end" },
    CfgCov { cfg_rows: &[290], note: "next_match \\K-in-lookaround case: every do_bumpalong branch" },
    CfgCov { cfg_rows: &[291], note: "next_match with match_data->rc < 0 leaves the outputs untouched" },
    CfgCov { cfg_rows: &[292], note: "full global-iteration driver under CRLF and LF" },
    CfgCov { cfg_rows: &[293], note: "pcre2_jit_match_8 stub: JIT_BADOPTION, and match_data->rc set" },
];

#[test]
fn coverage_declaration_is_sane() {
    check_coverage_decl(COVERAGE);
}

// =========================================================== local constants

const PCRE2_ERROR_CALLOUT: c_int = -37;
const PCRE2_ERROR_JIT_BADOPTION: c_int = -45;
const PCRE2_ERROR_RECURSELOOP: c_int = -52;
const PCRE2_ERROR_BAD_BACKSLASH_K: c_int = -75;
const PCRE2_CALLOUT_STARTMATCH: u32 = 0x0000_0001;
const PCRE2_CALLOUT_BACKTRACK: u32 = 0x0000_0002;
const PCRE2_MD_COPIED_SUBJECT: u8 = 0x01;
const PCRE2_MATCHEDBY_INTERPRETER: u8 = 0;
const START_FRAMES_SIZE: Sz = 20480;

/// Sentinel written into every match_data field the matcher is allowed to
/// leave alone, so that "left untouched" / "stale" is a comparable observable.
const SENT: Sz = 0x5A5A_5A5A_5A5A_5A5A;

// ================================================== match_data introspection

/// Mirrors `pcre2_real_match_data` from `c_src/src/pcre2_intmodedep.h`.
/// Validated against the public accessors by `mdh` on every use.
#[repr(C)]
struct RealMd {
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
    // PCRE2_SIZE ovector[] follows here
}

unsafe fn mdh(api: &Api, m: Ptr) -> &'static mut RealMd {
    let h = &mut *(m as *mut RealMd);
    let off = std::mem::size_of::<RealMd>();
    assert_eq!(off, 120, "RealMd layout: unexpected size");
    assert_eq!(
        (api.get_ovector_pointer)(m) as usize - m as usize,
        off,
        "[{}] pcre2_get_ovector_pointer_8 is not at offsetof(match_data, ovector)",
        api.name
    );
    assert_eq!((api.get_ovector_count)(m), h.oveccount as u32, "[{}] oveccount", api.name);
    assert_eq!((api.get_startchar)(m), h.startchar, "[{}] startchar", api.name);
    assert_eq!((api.get_mark)(m), h.mark, "[{}] mark", api.name);
    assert_eq!(
        (api.get_match_data_heapframes_size)(m),
        h.heapframes_size,
        "[{}] heapframes_size",
        api.name
    );
    assert_eq!(
        (api.get_match_data_size)(m),
        off + 2 * h.oveccount as usize * std::mem::size_of::<Sz>(),
        "[{}] match_data_size formula",
        api.name
    );
    h
}

unsafe fn prefill(api: &Api, m: Ptr) {
    let n;
    {
        let h = mdh(api, m);
        // Never clear `subject` while a PCRE2_COPY_MATCHED_SUBJECT copy is
        // still owned by the block, or freeing it would leak.
        if h.flags & PCRE2_MD_COPIED_SUBJECT == 0 {
            h.subject = ptr::null();
        }
        h.mark = ptr::null();
        h.code = ptr::null();
        h.subject_length = SENT;
        h.start_offset = SENT;
        h.leftchar = SENT;
        h.rightchar = SENT;
        h.startchar = SENT;
        h.matchedby = 0xEE;
        h.options = 0xDEAD_BEEF;
        h.rc = 0x0BAD_0BAD;
        n = 2 * h.oveccount as usize;
    }
    let ov = (api.get_ovector_pointer)(m);
    for i in 0..n {
        *ov.add(i) = SENT;
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum Subj {
    Null,
    Original,
    Other,
}

/// Everything observable about one `pcre2_match_8` call.
#[derive(Debug, PartialEq, Eq, Clone)]
struct Full {
    out: MatchOut,
    ovec_all: Vec<Sz>,
    oveccount: u32,
    md_size: Sz,
    hf_size: Sz,
    rc_field: c_int,
    matchedby: u8,
    flags: u8,
    options: u32,
    subject: Subj,
    subject_length: Sz,
    start_offset: Sz,
    leftchar: Sz,
    rightchar: Sz,
    startchar_raw: Sz,
    code_ok: bool,
    mark_raw: Option<Vec<u8>>,
    next: (c_int, Sz, u32),
    copied: Option<Vec<u8>>,
}

unsafe fn cstr(p: Sptr) -> Vec<u8> {
    let mut v = Vec::new();
    let mut q = p;
    while *q != 0 && v.len() < 512 {
        v.push(*q);
        q = q.add(1);
    }
    v
}

unsafe fn read_full(api: &Api, m: Ptr, rc: c_int, code: Ptr, orig: Sptr) -> Full {
    let out = read_match_out(api, m, rc);
    let h = mdh(api, m);
    let n = 2 * h.oveccount as usize;
    let ov = (api.get_ovector_pointer)(m);
    let ovec_all = std::slice::from_raw_parts(ov, n).to_vec();
    let subject = if h.subject.is_null() {
        Subj::Null
    } else if h.subject == orig {
        Subj::Original
    } else {
        Subj::Other
    };
    let copied = if h.flags & PCRE2_MD_COPIED_SUBJECT != 0 && !h.subject.is_null() {
        Some(std::slice::from_raw_parts(h.subject, h.subject_length).to_vec())
    } else {
        None
    };
    let mark_raw = if h.mark.is_null() { None } else { Some(cstr(h.mark)) };
    // pcre2_next_match_8 returns FALSE immediately when rc < 0, so this is
    // safe (and row 291's observable) for every return code.
    let (mut o, mut f) = (SENT, 0xABCD_1234u32);
    let nrc = (api.next_match)(m, &mut o, &mut f);
    Full {
        out,
        ovec_all,
        oveccount: (api.get_ovector_count)(m),
        md_size: (api.get_match_data_size)(m),
        hf_size: (api.get_match_data_heapframes_size)(m),
        rc_field: h.rc,
        matchedby: h.matchedby,
        flags: h.flags,
        options: h.options,
        subject,
        subject_length: h.subject_length,
        start_offset: h.start_offset,
        leftchar: h.leftchar,
        rightchar: h.rightchar,
        startchar_raw: h.startchar,
        code_ok: h.code == code as *const c_void,
        mark_raw,
        next: (nrc, o, f),
        copied,
    }
}

// ============================================================ compile helpers

#[derive(Clone, Copy, Debug)]
struct Cc {
    opts: u32,
    xopts: u32,
    newline: u32,
    bsr: u32,
    optimize: u32,
}

impl Cc {
    fn n(opts: u32) -> Cc {
        Cc { opts, xopts: 0, newline: 0, bsr: 0, optimize: u32::MAX }
    }
    fn x(mut self, v: u32) -> Cc {
        self.xopts = v;
        self
    }
    fn nl(mut self, v: u32) -> Cc {
        self.newline = v;
        self
    }
    fn bsr(mut self, v: u32) -> Cc {
        self.bsr = v;
        self
    }
    fn optim(mut self, v: u32) -> Cc {
        self.optimize = v;
        self
    }
}

unsafe fn cc_make(api: &Api, c: &Cc) -> Ptr {
    let cc = (api.compile_context_create)(ptr::null_mut());
    assert!(!cc.is_null());
    if c.newline != 0 {
        assert_eq!((api.set_newline)(cc, c.newline), 0);
    }
    if c.bsr != 0 {
        assert_eq!((api.set_bsr)(cc, c.bsr), 0);
    }
    if c.xopts != 0 {
        assert_eq!((api.set_compile_extra_options)(cc, c.xopts), 0);
    }
    if c.optimize != u32::MAX {
        assert_eq!((api.set_optimize)(cc, c.optimize), 0);
    }
    cc
}

unsafe fn errmsg(api: &Api, code: c_int) -> String {
    let mut buf = [0u8; 256];
    let n = (api.get_error_message)(code, buf.as_mut_ptr(), buf.len());
    if n <= 0 {
        return format!("<no message for {code}>");
    }
    String::from_utf8_lossy(&buf[..n as usize]).into_owned()
}

/// Compile in both libraries, asserting the two agree completely (including
/// byte-identical bytecode).  `None` when the pattern is rejected by both.
unsafe fn try_compile(p: &Pair, pat: &[u8], c: &Cc) -> Option<(Ptr, Ptr)> {
    let (mut e1, mut e2) = (0 as c_int, 0 as c_int);
    let (mut o1, mut o2) = (usize::MAX, usize::MAX);
    let ca = cc_make(&p.c, c);
    let cb = cc_make(&p.r, c);
    let a = (p.c.compile)(pat.as_ptr(), pat.len(), c.opts, &mut e1, &mut o1, ca);
    let b = (p.r.compile)(pat.as_ptr(), pat.len(), c.opts, &mut e2, &mut o2, cb);
    (p.c.compile_context_free)(ca);
    (p.r.compile_context_free)(cb);
    let tag = format!("compile {} opts={:#x}/{:#x}", show(pat), c.opts, c.xopts);
    assert_eq!(a.is_null(), b.is_null(), "{tag}: nullness differs (C ec={e1} rust ec={e2})");
    assert_eq!(e1, e2, "{tag}: errorcode differs");
    assert_eq!(o1, o2, "{tag}: erroroffset differs");
    if a.is_null() {
        return None;
    }
    assert_code_eq(a, b, &tag);
    Some((a, b))
}

/// As `try_compile`, but a compile failure is a test bug: report it loudly.
unsafe fn compile2(p: &Pair, pat: &[u8], c: &Cc) -> (Ptr, Ptr) {
    match try_compile(p, pat, c) {
        Some(v) => v,
        None => {
            let (mut e, mut o) = (0 as c_int, usize::MAX);
            let cc = cc_make(&p.c, c);
            let _ = (p.c.compile)(pat.as_ptr(), pat.len(), c.opts, &mut e, &mut o, cc);
            (p.c.compile_context_free)(cc);
            panic!(
                "pattern {} (opts={:#x} xopts={:#x}) does not compile: {} at offset {}",
                show(pat),
                c.opts,
                c.xopts,
                errmsg(&p.c, e),
                o
            );
        }
    }
}

unsafe fn free2(p: &Pair, ab: (Ptr, Ptr)) {
    (p.c.code_free)(ab.0);
    (p.r.code_free)(ab.1);
}

unsafe fn info_u32(api: &Api, code: Ptr, what: u32) -> u32 {
    let mut v = 0xDEAD_BEEFu32;
    let rc = (api.pattern_info)(code, what, &mut v as *mut u32 as Ptr);
    assert_eq!(rc, 0, "pattern_info({what}) failed: {rc}");
    v
}

/// Subject buffers always carry 16 trailing zero bytes: `PCRE2_NO_UTF_CHECK`
/// over deliberately invalid UTF-8 lets the decoder read a few units past the
/// nominal end, and both libraries must do so within our allocation.
fn pad(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.extend_from_slice(&[0u8; 16]);
    v
}

// ============================================================ match runners

struct Mc {
    a: Ptr,
    b: Ptr,
}

impl Mc {
    fn t(&self) -> (Ptr, Ptr) {
        (self.a, self.b)
    }
}

/// A match context pair with deterministic limits so that no row can hang.
unsafe fn mc_new(p: &Pair) -> Mc {
    let a = (p.c.match_context_create)(ptr::null_mut());
    let b = (p.r.match_context_create)(ptr::null_mut());
    assert!(!a.is_null() && !b.is_null());
    for (api, v) in [(&p.c, a), (&p.r, b)] {
        assert_eq!((api.set_match_limit)(v, 200_000), 0);
        assert_eq!((api.set_depth_limit)(v, 4_000), 0);
        assert_eq!((api.set_heap_limit)(v, 4_000), 0);
    }
    Mc { a, b }
}

unsafe fn mc_free(p: &Pair, m: Mc) {
    (p.c.match_context_free)(m.a);
    (p.r.match_context_free)(m.b);
}

/// The core comparison: run one match in each library against pre-filled
/// match_data blocks and compare every observable.  Returns the C result.
#[allow(clippy::too_many_arguments)]
unsafe fn run_md(
    p: &Pair,
    code: (Ptr, Ptr),
    md: (Ptr, Ptr),
    s: Sptr,
    len: Sz,
    start: Sz,
    mopts: u32,
    mc: (Ptr, Ptr),
    tag: &str,
    d: &mut Diffs,
) -> (c_int, Full) {
    prefill(&p.c, md.0);
    prefill(&p.r, md.1);
    let ra = (p.c.do_match)(code.0, s, len, start, mopts, md.0, mc.0);
    let rb = (p.r.do_match)(code.1, s, len, start, mopts, md.1, mc.1);
    let fa = read_full(&p.c, md.0, ra, code.0, s);
    let fb = read_full(&p.r, md.1, rb, code.1, s);
    d.eq(tag, fa.clone(), fb);
    (ra, fa)
}

/// `run_md` with a freshly created match_data pair of `ovec` pairs.
#[allow(clippy::too_many_arguments)]
unsafe fn run(
    p: &Pair,
    code: (Ptr, Ptr),
    subj: &[u8],
    start: Sz,
    mopts: u32,
    ovec: u32,
    mc: (Ptr, Ptr),
    tag: &str,
    d: &mut Diffs,
) -> (c_int, Full) {
    let buf = pad(subj);
    let mda = (p.c.match_data_create)(ovec, ptr::null_mut());
    let mdb = (p.r.match_data_create)(ovec, ptr::null_mut());
    let r = run_md(p, code, (mda, mdb), buf.as_ptr(), subj.len(), start, mopts, mc, tag, d);
    (p.c.match_data_free)(mda);
    (p.r.match_data_free)(mdb);
    r
}

/// Assert an expectation stated by the CONFIGS.md row against the C library.
fn want<T: PartialEq + std::fmt::Debug>(d: &mut Diffs, tag: &str, expected: T, from_c: T) {
    d.eq(
        &format!("[CONFIGS expectation vs C] {tag}   (C = row says, rust = C actually did)"),
        expected,
        from_c,
    );
}

/// The ovector prefix as a comparable vector of `n` pairs.
fn ov(f: &Full, n: usize) -> Vec<Sz> {
    f.ovec_all.iter().copied().take(2 * n).collect()
}

// ============================================================ callout logging

/// Exact layout of `pcre2_callout_block` from `c_src/include/pcre2.h`.
#[repr(C)]
struct CBlock {
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

static mut CLOG: Vec<String> = Vec::new();
/// Value the logging callout returns; `CRET_AT` selects which invocation.
static mut CRET: c_int = 0;
static mut CRET_AT: u32 = u32::MAX;
static mut CSEEN: u32 = 0;

unsafe extern "C" fn log_callout(blk: *mut c_void, _d: *mut c_void) -> c_int {
    let b = &*(blk as *const CBlock);
    let log = &mut *ptr::addr_of_mut!(CLOG);
    let seen = &mut *ptr::addr_of_mut!(CSEEN);
    // The first two ovector slots are documented to be PCRE2_UNSET while a
    // callout is active (row 259); read only those, since capture_top may
    // exceed the caller's oveccount.
    let ov0 = *b.offset_vector;
    let ov1 = *b.offset_vector.add(1);
    log.push(format!(
        "v={} n={} ctop={} clast={} slen={} sm={} cp={} pp={} nil={} cso={} csl={} flags={:#x} \
         mark={:?} cstr={:?} ov0={} ov1={} subj_ok={}",
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
        b.callout_flags,
        if b.mark.is_null() { None } else { Some(String::from_utf8_lossy(&cstr(b.mark)).into_owned()) },
        if b.callout_string.is_null() {
            None
        } else {
            Some(String::from_utf8_lossy(std::slice::from_raw_parts(b.callout_string, b.callout_string_length)).into_owned())
        },
        ov0 as i64,
        ov1 as i64,
        !b.subject.is_null(),
    ));
    *seen += 1;
    if *seen - 1 == *ptr::addr_of!(CRET_AT) {
        return *ptr::addr_of!(CRET);
    }
    0
}

/// Only the bumpalong start positions, for the start-optimization rows.
static mut SLOG: Vec<(Sz, u32)> = Vec::new();

unsafe extern "C" fn log_start(blk: *mut c_void, _d: *mut c_void) -> c_int {
    let b = &*(blk as *const CBlock);
    (*ptr::addr_of_mut!(SLOG)).push((b.start_match, b.callout_flags));
    0
}

struct CalloutMc {
    mc: Mc,
}

unsafe fn callout_mc(p: &Pair, f: CalloutFn) -> CalloutMc {
    let mc = mc_new(p);
    assert_eq!((p.c.set_callout)(mc.a, Some(f), ptr::null_mut()), 0);
    assert_eq!((p.r.set_callout)(mc.b, Some(f), ptr::null_mut()), 0);
    CalloutMc { mc }
}

/// Run a match in both libraries with callout logging and compare the WHOLE
/// callout sequence as well as the match result.
#[allow(clippy::too_many_arguments)]
unsafe fn run_log(
    p: &Pair,
    code: (Ptr, Ptr),
    subj: &[u8],
    start: Sz,
    mopts: u32,
    ovec: u32,
    mc: (Ptr, Ptr),
    tag: &str,
    d: &mut Diffs,
) -> (c_int, Full, Vec<String>) {
    let buf = pad(subj);
    let mda = (p.c.match_data_create)(ovec, ptr::null_mut());
    let mdb = (p.r.match_data_create)(ovec, ptr::null_mut());
    prefill(&p.c, mda);
    prefill(&p.r, mdb);
    CLOG.clear();
    CSEEN = 0;
    let ra = (p.c.do_match)(code.0, buf.as_ptr(), subj.len(), start, mopts, mda, mc.0);
    let la = CLOG.clone();
    CLOG.clear();
    CSEEN = 0;
    let rb = (p.r.do_match)(code.1, buf.as_ptr(), subj.len(), start, mopts, mdb, mc.1);
    let lb = CLOG.clone();
    let fa = read_full(&p.c, mda, ra, code.0, buf.as_ptr());
    let fb = read_full(&p.r, mdb, rb, code.1, buf.as_ptr());
    d.eq(tag, fa.clone(), fb);
    d.eq(&format!("{tag} :: callout sequence"), la.clone(), lb);
    (p.c.match_data_free)(mda);
    (p.r.match_data_free)(mdb);
    (ra, fa, la)
}

/// Bumpalong start positions for one match, in both libraries.
#[allow(clippy::too_many_arguments)]
unsafe fn run_starts(
    p: &Pair,
    code: (Ptr, Ptr),
    subj: &[u8],
    start: Sz,
    mopts: u32,
    mc: (Ptr, Ptr),
    tag: &str,
    d: &mut Diffs,
) -> (c_int, Vec<Sz>) {
    let buf = pad(subj);
    let mda = (p.c.match_data_create)(4, ptr::null_mut());
    let mdb = (p.r.match_data_create)(4, ptr::null_mut());
    prefill(&p.c, mda);
    prefill(&p.r, mdb);
    SLOG.clear();
    let ra = (p.c.do_match)(code.0, buf.as_ptr(), subj.len(), start, mopts, mda, mc.0);
    let la = SLOG.clone();
    SLOG.clear();
    let rb = (p.r.do_match)(code.1, buf.as_ptr(), subj.len(), start, mopts, mdb, mc.1);
    let lb = SLOG.clone();
    let fa = read_full(&p.c, mda, ra, code.0, buf.as_ptr());
    let fb = read_full(&p.r, mdb, rb, code.1, buf.as_ptr());
    d.eq(tag, fa, fb);
    d.eq(&format!("{tag} :: callout positions"), la.clone(), lb);
    (p.c.match_data_free)(mda);
    (p.r.match_data_free)(mdb);
    // the distinct bumpalong positions, in order
    // `cb.callout_flags |= PCRE2_CALLOUT_STARTMATCH` is set once per bumpalong
    // and cleared by `do_callout`, so with a callout as the very first opcode
    // these are exactly the attempted start positions, in order.
    let out: Vec<Sz> = la
        .into_iter()
        .filter(|&(_, fl)| fl & PCRE2_CALLOUT_STARTMATCH != 0)
        .map(|(pos, _)| pos)
        .collect();
    (ra, out)
}

/// `run` with an explicit `length` (for `PCRE2_ZERO_TERMINATED`).
#[allow(clippy::too_many_arguments)]
unsafe fn run_len(
    p: &Pair,
    code: (Ptr, Ptr),
    subj: &[u8],
    len: Sz,
    start: Sz,
    mopts: u32,
    ovec: u32,
    mc: (Ptr, Ptr),
    tag: &str,
    d: &mut Diffs,
) -> (c_int, Full) {
    let buf = pad(subj);
    let mda = (p.c.match_data_create)(ovec, ptr::null_mut());
    let mdb = (p.r.match_data_create)(ovec, ptr::null_mut());
    let r = run_md(p, code, (mda, mdb), buf.as_ptr(), len, start, mopts, mc, tag, d);
    (p.c.match_data_free)(mda);
    (p.r.match_data_free)(mdb);
    r
}

const NOMC: (Ptr, Ptr) = (ptr::null_mut(), ptr::null_mut());

// ================================================= counting allocators (186, 215, 216)

static mut ACNT: [(usize, usize); 8] = [(0, 0); 8];

unsafe fn raw_alloc(n: usize) -> *mut c_void {
    let sz = n.max(1) + 16;
    let l = std::alloc::Layout::from_size_align(sz, 16).unwrap();
    let q = std::alloc::alloc(l);
    *(q as *mut usize) = sz;
    q.add(16) as *mut c_void
}

unsafe extern "C" fn raw_free(q: *mut c_void, _d: *mut c_void) {
    if q.is_null() {
        return;
    }
    let base = (q as *mut u8).sub(16);
    let sz = *(base as *mut usize);
    std::alloc::dealloc(base, std::alloc::Layout::from_size_align(sz, 16).unwrap());
}

macro_rules! counting {
    ($name:ident, $idx:expr) => {
        unsafe extern "C" fn $name(n: usize, _d: *mut c_void) -> *mut c_void {
            let a = &mut (*ptr::addr_of_mut!(ACNT))[$idx];
            a.0 += 1;
            a.1 += n;
            raw_alloc(n)
        }
    };
}
counting!(m0, 0);
counting!(m1, 1);
counting!(m2, 2);
counting!(m3, 3);
counting!(m4, 4);
counting!(m5, 5);
counting!(m6, 6);
counting!(m7, 7);

fn acnt(i: usize) -> (usize, usize) {
    unsafe { (*ptr::addr_of!(ACNT))[i] }
}
fn acnt_reset() {
    unsafe {
        for x in (*ptr::addr_of_mut!(ACNT)).iter_mut() {
            *x = (0, 0);
        }
    }
}

// ============================================================= rows 154-158

#[test]
fn cfg_154_158_entry_and_offsets() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(1540);
    unsafe {
        // ---- row 154: the documented baseline, mcontext NULL, oveccount 1
        let code = compile2(p, b"abc", &Cc::n(0));
        let (rc, f) = run(p, code, b"xxabcxx", 0, 0, 1, NOMC, "154 /abc/ xxabcxx", &mut d);
        want(&mut d, "154 rc", 1, rc);
        want(&mut d, "154 ovector", vec![2usize, 5], ov(&f, 1));
        want(&mut d, "154 startchar", 2, f.startchar_raw);
        want(&mut d, "154 leftchar", 2, f.leftchar);
        want(&mut d, "154 rightchar", 5, f.rightchar);
        want(&mut d, "154 mark", None, f.mark_raw.clone());
        want(&mut d, "154 matchedby", PCRE2_MATCHEDBY_INTERPRETER, f.matchedby);
        want(&mut d, "154 subject", Subj::Original, f.subject.clone());
        want(&mut d, "154 subject_length", 7, f.subject_length);
        want(&mut d, "154 start_offset", 0, f.start_offset);
        want(&mut d, "154 options", 0u32, f.options);
        want(&mut d, "154 flags", 0u8, f.flags);
        want(&mut d, "154 oveccount", 1u32, f.oveccount);
        want(&mut d, "154 code back-pointer", true, f.code_ok);
        // randomized: the same baseline shape over many literal/subject pairs
        for _ in 0..400 {
            let lit: Vec<u8> = (0..rng.range(1, 4)).map(|_| *rng.pick(b"abcxy")).collect();
            let mut subj = gen_ascii(&mut rng, 12);
            if rng.chance(2) {
                let at = rng.below(subj.len() + 1);
                for (k, c) in lit.iter().enumerate() {
                    subj.insert(at + k, *c);
                }
            }
            let Some(c2) = try_compile(p, &lit, &Cc::n(0)) else { continue };
            let start = rng.below(subj.len() + 1);
            run(
                p,
                c2,
                &subj,
                start,
                0,
                *rng.pick(&[1u32, 2, 4]),
                NOMC,
                &format!("154 rnd {} {} @{start}", show(&lit), show(&subj)),
                &mut d,
            );
            free2(p, c2);
        }
        free2(p, code);

        // ---- row 155: subject == NULL with length 0 (internal null_str)
        for pat in [&b"abc"[..], &b"a*"[..], &b""[..], &b"^$"[..], &b"\\A\\z"[..]] {
            let c2 = compile2(p, pat, &Cc::n(0));
            let mda = (p.c.match_data_create)(2, ptr::null_mut());
            let mdb = (p.r.match_data_create)(2, ptr::null_mut());
            let (rc, f) = run_md(
                p,
                c2,
                (mda, mdb),
                ptr::null(),
                0,
                0,
                0,
                NOMC,
                &format!("155 NULL subject {}", show(pat)),
                &mut d,
            );
            want(&mut d, &format!("155 {} subject stays NULL", show(pat)), Subj::Null, f.subject.clone());
            if pat == &b"abc"[..] {
                want(&mut d, "155 /abc/ on NULL,0 => NOMATCH", PCRE2_ERROR_NOMATCH, rc);
            }
            if pat == &b"a*"[..] {
                want(&mut d, "155 /a*/ on NULL,0 => empty match", 1, rc);
                want(&mut d, "155 /a*/ ovector", vec![0usize, 0], ov(&f, 1));
            }
            (p.c.match_data_free)(mda);
            (p.r.match_data_free)(mdb);
            free2(p, c2);
        }

        // ---- row 156: PCRE2_ZERO_TERMINATED resolved before the offset check
        let subj = b"abc\0def";
        for (pat, expect) in [(&b"abc"[..], 1), (&b"def"[..], PCRE2_ERROR_NOMATCH), (&b"abc\\z"[..], 1)] {
            let c2 = compile2(p, pat, &Cc::n(0));
            let (rc, _) = run_len(
                p,
                c2,
                subj,
                PCRE2_ZERO_TERMINATED,
                0,
                0,
                2,
                NOMC,
                &format!("156 ZT {}", show(pat)),
                &mut d,
            );
            want(&mut d, &format!("156 ZT {} rc", show(pat)), expect, rc);
            // startoffset checked against the RESOLVED length (3)
            for start in [0usize, 3, 4, 7] {
                let (rc2, _) = run_len(
                    p,
                    c2,
                    subj,
                    PCRE2_ZERO_TERMINATED,
                    start,
                    0,
                    2,
                    NOMC,
                    &format!("156 ZT {} start={start}", show(pat)),
                    &mut d,
                );
                if start > 3 {
                    want(
                        &mut d,
                        &format!("156 start={start} > resolved length => BADOFFSET"),
                        PCRE2_ERROR_BADOFFSET,
                        rc2,
                    );
                }
            }
            free2(p, c2);
        }

        // ---- row 157: startoffset == length
        let c2 = compile2(p, b"(?<=abc)", &Cc::n(0));
        let (rc, f) = run(p, c2, b"abc", 3, 0, 2, NOMC, "157 /(?<=abc)/ start=3", &mut d);
        want(&mut d, "157 empty match at the end", 1, rc);
        want(&mut d, "157 ovector", vec![3usize, 3], ov(&f, 1));
        free2(p, c2);
        let c2 = compile2(p, b"a", &Cc::n(0));
        let (rc, _) = run(p, c2, b"abc", 3, 0, 2, NOMC, "157 /a/ start=3", &mut d);
        want(&mut d, "157 /a/ at start==length => NOMATCH", PCRE2_ERROR_NOMATCH, rc);
        free2(p, c2);

        // ---- row 158: startoffset mid-subject, lookbehind reads before it
        let c2 = compile2(p, b"(?<=xx)abc", &Cc::n(0));
        want(&mut d, "158 max_lookbehind", 2u32, info_u32(&p.c, c2.0, PCRE2_INFO_MAXLOOKBEHIND));
        let (rc, f) = run(p, c2, b"xxabc", 2, 0, 2, NOMC, "158 start=2", &mut d);
        want(&mut d, "158 rc", 1, rc);
        want(&mut d, "158 ovector", vec![2usize, 5], ov(&f, 1));
        want(&mut d, "158 leftchar reaches before startoffset", 0, f.leftchar);
        let (rc, _) = run(p, c2, b"yyabc", 2, 0, 2, NOMC, "158 wrong lookbehind", &mut d);
        want(&mut d, "158 lookbehind not satisfied", PCRE2_ERROR_NOMATCH, rc);
        // every start offset, UTF and not, with a multi-byte lookbehind too
        for start in 0..=5 {
            run(p, c2, b"xxabc", start, 0, 2, NOMC, &format!("158 sweep start={start}"), &mut d);
        }
        free2(p, c2);
        let c2 = compile2(p, "(?<=\u{e9}\u{e9})abc".as_bytes(), &Cc::n(PCRE2_UTF));
        let subj = "\u{e9}\u{e9}abc".as_bytes();
        for start in 0..=subj.len() {
            run(p, c2, subj, start, 0, 2, NOMC, &format!("158 UTF start={start}"), &mut d);
        }
        free2(p, c2);
    }
    d.finish("CONFIGS 154-158: entry conditions, NULL subject, ZERO_TERMINATED, startoffset edges");
}

// ============================================================= rows 159-161

#[test]
fn cfg_159_161_notempty() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(1590);
    unsafe {
        // ---- row 159: (*NOTEMPTY) / (*NOTEMPTY_ATSTART) vs the option bits
        for (verb, bit) in [("(*NOTEMPTY)", PCRE2_NOTEMPTY), ("(*NOTEMPTY_ATSTART)", PCRE2_NOTEMPTY_ATSTART)] {
            for body in ["a*", "b?", "(?:)", "x|", "a*b*"] {
                let emb = format!("{verb}{body}");
                let ca = compile2(p, emb.as_bytes(), &Cc::n(0));
                let cb = compile2(p, body.as_bytes(), &Cc::n(0));
                for subj in ["", "a", "b", "bbb", "xa", "aab"] {
                    for start in 0..=subj.len() {
                        let t = format!("159 {emb} vs {body}+opt on {subj} @{start}");
                        let (r1, f1) = run(p, ca, subj.as_bytes(), start, 0, 4, NOMC, &format!("{t} embedded"), &mut d);
                        let (r2, f2) = run(p, cb, subj.as_bytes(), start, bit, 4, NOMC, &format!("{t} option"), &mut d);
                        want(&mut d, &format!("{t} same rc"), r2, r1);
                        want(&mut d, &format!("{t} same ovector"), ov(&f2, 2), ov(&f1, 2));
                        want(&mut d, &format!("{t} embedded options field"), 0u32, f1.options);
                        want(&mut d, &format!("{t} passed options field"), bit, f2.options);
                    }
                }
                free2(p, ca);
                free2(p, cb);
            }
        }

        // ---- row 160: NOTEMPTY vs NOTEMPTY_ATSTART on /a*/ over bbb
        let code = compile2(p, b"a*", &Cc::n(0));
        let (rc, _) = run(p, code, b"bbb", 0, PCRE2_NOTEMPTY, 2, NOMC, "160 NOTEMPTY /a*/ bbb", &mut d);
        want(&mut d, "160 NOTEMPTY rejects every empty match", PCRE2_ERROR_NOMATCH, rc);
        let (rc, f) = run(p, code, b"bbb", 0, PCRE2_NOTEMPTY_ATSTART, 2, NOMC, "160 ATSTART /a*/ bbb", &mut d);
        want(&mut d, "160 NOTEMPTY_ATSTART only forbids the start", 1, rc);
        want(&mut d, "160 NOTEMPTY_ATSTART ovector", vec![1usize, 1], ov(&f, 1));

        // ---- row 161: NOTEMPTY_ATSTART with startoffset 2
        let (rc, f) = run(p, code, b"xxyy", 2, PCRE2_NOTEMPTY_ATSTART, 2, NOMC, "161 start=2", &mut d);
        want(&mut d, "161 rc", 1, rc);
        want(&mut d, "161 empty match at 3", vec![3usize, 3], ov(&f, 1));
        for start in 0..=4 {
            for mo in [0, PCRE2_NOTEMPTY, PCRE2_NOTEMPTY_ATSTART, PCRE2_NOTEMPTY | PCRE2_NOTEMPTY_ATSTART] {
                run(p, code, b"xxyy", start, mo, 2, NOMC, &format!("161 sweep {start} {mo:#x}"), &mut d);
            }
        }
        free2(p, code);

        // randomized: empty-capable patterns x random subjects x both bits
        let pats: &[&str] = &["a*", "b*?", "(a)?", "(?:x|)", "\\b", "^", "$", "(?=a)", "a{0,2}", "()"];
        for _ in 0..300 {
            let pat = *rng.pick(pats);
            let Some(c2) = try_compile(p, pat.as_bytes(), &Cc::n(if rng.chance(3) { PCRE2_MULTILINE } else { 0 })) else {
                continue;
            };
            let subj = gen_ascii(&mut rng, 8);
            let start = rng.below(subj.len() + 1);
            let mo = *rng.pick(&[0u32, PCRE2_NOTEMPTY, PCRE2_NOTEMPTY_ATSTART]);
            run(p, c2, &subj, start, mo, 4, NOMC, &format!("159-161 rnd {pat} {} @{start} {mo:#x}", show(&subj)), &mut d);
            free2(p, c2);
        }
    }
    d.finish("CONFIGS 159-161: (*NOTEMPTY[_ATSTART]) verbs and the PCRE2_NOTEMPTY* option bits");
}

// ============================================================= rows 162-165

#[test]
fn cfg_162_165_bol_eol() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(1620);
    unsafe {
        // ---- row 162: NOTBOL
        let code = compile2(p, b"^abc", &Cc::n(0));
        let (rc, _) = run(p, code, b"abc", 0, PCRE2_NOTBOL, 2, NOMC, "162 /^abc/ NOTBOL", &mut d);
        want(&mut d, "162 NOTBOL blocks ^ at the start", PCRE2_ERROR_NOMATCH, rc);
        free2(p, code);
        let code = compile2(p, b"^abc", &Cc::n(PCRE2_MULTILINE));
        let (rc, f) = run(p, code, b"abc\nabc", 0, PCRE2_NOTBOL, 2, NOMC, "162 /^abc/m NOTBOL", &mut d);
        want(&mut d, "162 second line still matches", 1, rc);
        want(&mut d, "162 ovector", vec![4usize, 7], ov(&f, 1));
        run(p, code, b"abc\nabc", 0, 0, 2, NOMC, "162 /^abc/m plain", &mut d);
        free2(p, code);

        // ---- row 163: NOTEOL
        let code = compile2(p, b"abc$", &Cc::n(0));
        let (rc, _) = run(p, code, b"abc", 0, PCRE2_NOTEOL, 2, NOMC, "163 /abc$/ NOTEOL", &mut d);
        want(&mut d, "163 NOTEOL blocks $ at the end", PCRE2_ERROR_NOMATCH, rc);
        free2(p, code);
        let code = compile2(p, b"abc$", &Cc::n(PCRE2_MULTILINE));
        let (rc, f) = run(p, code, b"abc\nabc", 0, PCRE2_NOTEOL, 2, NOMC, "163 /abc$/m NOTEOL", &mut d);
        want(&mut d, "163 first line matches", 1, rc);
        want(&mut d, "163 ovector", vec![0usize, 3], ov(&f, 1));
        free2(p, code);

        // ---- row 164: DOLLAR_ENDONLY
        for (opts, expect) in [(PCRE2_DOLLAR_ENDONLY, PCRE2_ERROR_NOMATCH), (0, 1)] {
            let code = compile2(p, b"abc$", &Cc::n(opts));
            let (rc, _) = run(p, code, b"abc\n", 0, 0, 2, NOMC, &format!("164 opts={opts:#x}"), &mut d);
            want(&mut d, &format!("164 DOLLAR_ENDONLY={} on abc\\n", opts != 0), expect, rc);
            run(p, code, b"abc", 0, 0, 2, NOMC, &format!("164 no newline opts={opts:#x}"), &mut d);
            run(p, code, b"abc\n\n", 0, 0, 2, NOMC, &format!("164 two newlines opts={opts:#x}"), &mut d);
            free2(p, code);
        }

        // ---- row 165: ALT_CIRCUMFLEX lets ^ match at end_subject
        for (opts, expect_at_end) in [
            (PCRE2_MULTILINE, PCRE2_ERROR_NOMATCH),
            (PCRE2_MULTILINE | PCRE2_ALT_CIRCUMFLEX, 1),
        ] {
            let code = compile2(p, b"^", &Cc::n(opts));
            let (rc, f) = run(p, code, b"abc\n", 4, 0, 2, NOMC, &format!("165 /^/m at end opts={opts:#x}"), &mut d);
            want(
                &mut d,
                &format!("165 ^ at end_subject with ALT_CIRCUMFLEX={}", opts & PCRE2_ALT_CIRCUMFLEX != 0),
                expect_at_end,
                rc,
            );
            if expect_at_end == 1 {
                want(&mut d, "165 empty match at 4", vec![4usize, 4], ov(&f, 1));
            }
            for start in 0..=4 {
                run(p, code, b"abc\n", start, 0, 2, NOMC, &format!("165 sweep {start} opts={opts:#x}"), &mut d);
                run(p, code, b"a\nb\n", start, PCRE2_NOTBOL, 2, NOMC, &format!("165 notbol {start} opts={opts:#x}"), &mut d);
            }
            free2(p, code);
            // and the row's literal /^x/ shape
            let code = compile2(p, b"^x", &Cc::n(opts));
            run(p, code, b"abc\n", 0, 0, 2, NOMC, &format!("165 /^x/m abc\\n opts={opts:#x}"), &mut d);
            run(p, code, b"abc\nx", 0, 0, 2, NOMC, &format!("165 /^x/m abc\\nx opts={opts:#x}"), &mut d);
            free2(p, code);
        }

        // randomized: ^ and $ with every combination of the two option bits
        let pats: &[&str] = &["^a", "a$", "^a$", "^", "$", "^.*$", "(?m)^a", "(?m)a$", "\\Aa", "a\\z", "a\\Z"];
        let subs: &[&str] = &["", "a", "a\n", "\na", "a\nb", "abc\n", "\n", "a\r\nb", "aa\n\n"];
        for _ in 0..600 {
            let pat = *rng.pick(pats);
            let opts = [0u32, PCRE2_MULTILINE, PCRE2_DOLLAR_ENDONLY, PCRE2_MULTILINE | PCRE2_ALT_CIRCUMFLEX,
                        PCRE2_MULTILINE | PCRE2_DOLLAR_ENDONLY][rng.below(5)];
            let Some(c2) = try_compile(p, pat.as_bytes(), &Cc::n(opts)) else { continue };
            let s = *rng.pick(subs);
            let mo = *rng.pick(&[0u32, PCRE2_NOTBOL, PCRE2_NOTEOL, PCRE2_NOTBOL | PCRE2_NOTEOL]);
            let start = rng.below(s.len() + 1);
            run(p, c2, s.as_bytes(), start, mo, 4, NOMC, &format!("162-165 rnd {pat} {opts:#x} {} @{start} {mo:#x}", show(s.as_bytes())), &mut d);
            free2(p, c2);
        }
    }
    d.finish("CONFIGS 162-165: NOTBOL / NOTEOL / DOLLAR_ENDONLY / ALT_CIRCUMFLEX");
}

// ============================================================= rows 166-167

#[test]
fn cfg_166_167_unset_backref() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(1660);
    unsafe {
        for (pat, row) in [(&b"(a)?\\1b"[..], 166), (&b"(a)?\\1{2,3}b"[..], 167)] {
            for opts in [PCRE2_MATCH_UNSET_BACKREF, 0] {
                let code = compile2(p, pat, &Cc::n(opts));
                for subj in ["b", "ab", "aab", "aaab", "abb", ""] {
                    let (rc, f) = run(
                        p,
                        code,
                        subj.as_bytes(),
                        0,
                        0,
                        4,
                        NOMC,
                        &format!("{row} {} opts={opts:#x} subj={subj}", show(pat)),
                        &mut d,
                    );
                    if subj == "b" && opts == PCRE2_MATCH_UNSET_BACKREF {
                        want(&mut d, &format!("{row} MATCH_UNSET_BACKREF matches b"), 1, rc);
                        want(&mut d, &format!("{row} group 1 unset"), vec![0usize, 1, PCRE2_UNSET, PCRE2_UNSET], ov(&f, 2));
                    }
                    if subj == "b" && opts == 0 {
                        want(&mut d, &format!("{row} without the option => NOMATCH"), PCRE2_ERROR_NOMATCH, rc);
                    }
                }
                free2(p, code);
            }
        }
        // randomized: unset-backref shapes over random subjects
        let pats: &[&str] = &[
            "(a)?\\1b", "(a)?\\1{2,3}b", "(a)?\\1*b", "(a)?\\1+b", "(a)?\\1?b",
            "(?<n>a)?\\k<n>b", "(a)?(b)?\\2\\1c", "(a)|\\1b",
        ];
        for _ in 0..400 {
            let pat = *rng.pick(pats);
            let opts = if rng.chance(2) { PCRE2_MATCH_UNSET_BACKREF } else { 0 }
                | if rng.chance(3) { PCRE2_CASELESS } else { 0 };
            let Some(c2) = try_compile(p, pat.as_bytes(), &Cc::n(opts)) else { continue };
            let subj: Vec<u8> = (0..rng.below(7)).map(|_| *rng.pick(b"abcAB")).collect();
            run(p, c2, &subj, 0, 0, 8, NOMC, &format!("166-167 rnd {pat} {opts:#x} {}", show(&subj)), &mut d);
            free2(p, c2);
        }
    }
    d.finish("CONFIGS 166-167: PCRE2_MATCH_UNSET_BACKREF, plain and repeated references");
}

// ============================================================= rows 168-169

#[test]
fn cfg_168_169_anchored_endanchored() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(1680);
    unsafe {
        // ---- row 168: ANCHORED at match time vs at compile time
        let code = compile2(p, b"abc", &Cc::n(0));
        let (rc, _) = run(p, code, b"xabc", 0, PCRE2_ANCHORED, 2, NOMC, "168 match-time ANCHORED xabc", &mut d);
        want(&mut d, "168 ANCHORED at match time => no bumpalong", PCRE2_ERROR_NOMATCH, rc);
        let (rc, f) = run(p, code, b"abc", 0, PCRE2_ANCHORED, 2, NOMC, "168 match-time ANCHORED abc", &mut d);
        want(&mut d, "168 anchored match at 0", 1, rc);
        want(&mut d, "168 ovector", vec![0usize, 3], ov(&f, 1));
        let ccode = compile2(p, b"abc", &Cc::n(PCRE2_ANCHORED));
        for subj in ["xabc", "abc", "abcx", ""] {
            let (r1, f1) = run(p, code, subj.as_bytes(), 0, PCRE2_ANCHORED, 4, NOMC, &format!("168 mt {subj}"), &mut d);
            let (r2, f2) = run(p, ccode, subj.as_bytes(), 0, 0, 4, NOMC, &format!("168 ct {subj}"), &mut d);
            want(&mut d, &format!("168 compile-time == match-time ANCHORED rc {subj}"), r1, r2);
            want(&mut d, &format!("168 compile-time == match-time ANCHORED ovector {subj}"), ov(&f1, 2), ov(&f2, 2));
        }
        free2(p, ccode);
        free2(p, code);

        // ---- row 169: ENDANCHORED
        let code = compile2(p, b"ab", &Cc::n(0));
        let (rc, _) = run(p, code, b"abc", 0, PCRE2_ENDANCHORED, 2, NOMC, "169 /ab/ ENDANCHORED abc", &mut d);
        want(&mut d, "169 not at end => NOMATCH", PCRE2_ERROR_NOMATCH, rc);
        let (rc, f) = run(p, code, b"ab", 0, PCRE2_ENDANCHORED, 2, NOMC, "169 /ab/ ENDANCHORED ab", &mut d);
        want(&mut d, "169 at end => match", 1, rc);
        want(&mut d, "169 ovector", vec![0usize, 2], ov(&f, 1));
        free2(p, code);
        // (*ACCEPT) + ENDANCHORED hard-returns instead of backtracking: the
        // second alternative is never tried.
        let acc = compile2(p, b"a(*ACCEPT)b|ab", &Cc::n(0));
        let plain = compile2(p, b"a|ab", &Cc::n(0));
        let (r1, _) = run(p, acc, b"ab", 0, PCRE2_ENDANCHORED, 4, NOMC, "169 accept+ENDANCHORED", &mut d);
        let (r2, f2) = run(p, plain, b"ab", 0, PCRE2_ENDANCHORED, 4, NOMC, "169 plain+ENDANCHORED", &mut d);
        want(&mut d, "169 (*ACCEPT) not at end hard-returns", PCRE2_ERROR_NOMATCH, r1);
        want(&mut d, "169 /a|ab/ backtracks to the 2nd branch", 1, r2);
        want(&mut d, "169 /a|ab/ ovector", vec![0usize, 2], ov(&f2, 1));
        let (r3, _) = run(p, acc, b"ab", 0, 0, 4, NOMC, "169 accept no ENDANCHORED", &mut d);
        want(&mut d, "169 (*ACCEPT) without ENDANCHORED", 1, r3);
        free2(p, acc);
        free2(p, plain);
        // partial + ENDANCHORED is rejected
        let code = compile2(p, b"abcd", &Cc::n(0));
        for mo in [PCRE2_PARTIAL_SOFT | PCRE2_ENDANCHORED, PCRE2_PARTIAL_HARD | PCRE2_ENDANCHORED] {
            let (rc, _) = run(p, code, b"ab", 0, mo, 2, NOMC, &format!("169 partial+ENDANCHORED {mo:#x}"), &mut d);
            want(&mut d, "169 partial with ENDANCHORED => BADOPTION", PCRE2_ERROR_BADOPTION, rc);
        }
        free2(p, code);
        let ecode = compile2(p, b"abcd", &Cc::n(PCRE2_ENDANCHORED));
        let (rc, _) = run(p, ecode, b"ab", 0, PCRE2_PARTIAL_SOFT, 2, NOMC, "169 compile ENDANCHORED + partial", &mut d);
        want(&mut d, "169 compile-time ENDANCHORED + partial => BADOPTION", PCRE2_ERROR_BADOPTION, rc);
        free2(p, ecode);

        // randomized sweep over both bits from both sources
        for _ in 0..500 {
            let pat = *rng.pick(&["abc", "a.c", "a*", "(a)(b)?", "^a", "a$", "\\w+", "a|ab|abc"]);
            let ct = *rng.pick(&[0u32, PCRE2_ANCHORED, PCRE2_ENDANCHORED, PCRE2_ANCHORED | PCRE2_ENDANCHORED]);
            let Some(c2) = try_compile(p, pat.as_bytes(), &Cc::n(ct)) else { continue };
            let subj = gen_ascii(&mut rng, 8);
            let mt = *rng.pick(&[0u32, PCRE2_ANCHORED, PCRE2_ENDANCHORED, PCRE2_ANCHORED | PCRE2_ENDANCHORED]);
            let start = rng.below(subj.len() + 1);
            run(p, c2, &subj, start, mt, 4, NOMC, &format!("168-169 rnd {pat} ct={ct:#x} mt={mt:#x} {} @{start}", show(&subj)), &mut d);
            free2(p, c2);
        }
    }
    d.finish("CONFIGS 168-169: PCRE2_ANCHORED / PCRE2_ENDANCHORED at match time and compile time");
}

// ============================================================= rows 170-176

#[test]
fn cfg_170_176_partial() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(1700);
    unsafe {
        // ---- row 170: which partial mode wins
        let code = compile2(p, b"abcd", &Cc::n(0));
        for (mo, name) in [
            (PCRE2_PARTIAL_SOFT, "SOFT"),
            (PCRE2_PARTIAL_HARD, "HARD"),
            (PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD, "BOTH"),
        ] {
            let (r1, _) = run(p, code, b"ab", 0, mo, 2, NOMC, &format!("170 {name} on ab"), &mut d);
            want(&mut d, &format!("170 {name} on ab => PARTIAL"), PCRE2_ERROR_PARTIAL, r1);
            let (r2, _) = run(p, code, b"abcd", 0, mo, 2, NOMC, &format!("170 {name} on abcd"), &mut d);
            want(&mut d, &format!("170 {name} on abcd: /abcd/ needs nothing more => complete"), 1, r2);
        }
        free2(p, code);
        // HARD only differs from SOFT where an assertion inspects the end: the
        // OP_EOD arm sets hitend and returns immediately when partial > 1.
        let code = compile2(p, b"abcd\\z", &Cc::n(0));
        for (mo, name, expect) in [
            (PCRE2_PARTIAL_SOFT, "SOFT", 1),
            (PCRE2_PARTIAL_HARD, "HARD", PCRE2_ERROR_PARTIAL),
            (PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD, "BOTH", PCRE2_ERROR_PARTIAL),
            (0, "none", 1),
        ] {
            let (rc, _) = run(p, code, b"abcd", 0, mo, 2, NOMC, &format!("170 /abcd\\z/ {name} on abcd"), &mut d);
            want(&mut d, &format!("170 /abcd\\z/ {name} on abcd (HARD wins over SOFT)"), expect, rc);
        }
        free2(p, code);

        // ---- row 171: only pair 0 is written; capture slots untouched
        let code = compile2(p, b"a(b)cd", &Cc::n(0));
        let (rc, f) = run(p, code, b"ab", 0, PCRE2_PARTIAL_SOFT, 4, NOMC, "171 /a(b)cd/ PARTIAL_SOFT ab", &mut d);
        want(&mut d, "171 rc", PCRE2_ERROR_PARTIAL, rc);
        want(&mut d, "171 ovector pair 0", vec![0usize, 2], ov(&f, 1));
        want(&mut d, "171 capture slots untouched", vec![SENT, SENT, SENT, SENT, SENT, SENT], f.ovec_all[2..].to_vec());
        want(&mut d, "171 startchar == partial start", 0, f.startchar_raw);
        want(&mut d, "171 leftchar", 0, f.leftchar);
        want(&mut d, "171 rightchar == end_subject", 2, f.rightchar);
        free2(p, code);

        // ---- row 172: hard partial beats the complete match at \z.
        // NB `/abcd/` alone does NOT: nothing more could ever match, so no
        // OP_* handler sets hitend.  It takes an end-of-subject assertion.
        let code = compile2(p, b"abcd\\z", &Cc::n(0));
        let (rc, f) = run(p, code, b"abcd", 0, PCRE2_PARTIAL_HARD, 2, NOMC, "172 PARTIAL_HARD /abcd\\z/ abcd", &mut d);
        want(&mut d, "172 rc", PCRE2_ERROR_PARTIAL, rc);
        want(&mut d, "172 ovector", vec![0usize, 4], ov(&f, 1));
        free2(p, code);
        let code = compile2(p, b"abcd", &Cc::n(0));
        let (rc, f) = run(p, code, b"abcd", 0, PCRE2_PARTIAL_HARD, 2, NOMC, "172 PARTIAL_HARD /abcd/ abcd", &mut d);
        want(&mut d, "172 /abcd/ is a complete match even with PARTIAL_HARD", 1, rc);
        want(&mut d, "172 /abcd/ ovector", vec![0usize, 4], ov(&f, 1));
        free2(p, code);
        // the other end-inspecting arms behave the same way
        for pat in [&b"abcd$"[..], &b"abcd\\Z"[..], &b"abcd\\b"[..], &b"abc\\wd?"[..], &b"ab\\w*"[..]] {
            let code = compile2(p, pat, &Cc::n(0));
            for mo in [PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT, 0] {
                run(p, code, b"abcd", 0, mo, 2, NOMC, &format!("172 {} mo={mo:#x}", show(pat)), &mut d);
            }
            free2(p, code);
        }

        // ---- row 173: allowemptypartial via max_lookbehind > 0
        let code = compile2(p, b"(?<=abc)def", &Cc::n(0));
        want(&mut d, "173 max_lookbehind", 3u32, info_u32(&p.c, code.0, PCRE2_INFO_MAXLOOKBEHIND));
        let (rc, f) = run(p, code, b"abc", 0, PCRE2_PARTIAL_SOFT, 2, NOMC, "173 zero-length partial", &mut d);
        want(&mut d, "173 rc", PCRE2_ERROR_PARTIAL, rc);
        want(&mut d, "173 zero-length partial ovector", vec![3usize, 3], ov(&f, 1));
        free2(p, code);

        // ---- rows 174-175: the CRLF-split partial special cases
        for nl in [PCRE2_NEWLINE_CRLF, PCRE2_NEWLINE_CR, PCRE2_NEWLINE_LF, PCRE2_NEWLINE_ANY, PCRE2_NEWLINE_ANYCRLF, PCRE2_NEWLINE_NUL] {
            let code = compile2(p, b".", &Cc::n(0).nl(nl));
            for (subj, tag) in [(&b"\r"[..], "lone CR"), (&b"a\r"[..], "a then CR"), (&b"\r\n"[..], "CRLF")] {
                for mo in [PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT, 0] {
                    let (rc, _) = run(p, code, subj, 0, mo, 2, NOMC, &format!("174 /./ nl={nl} {tag} mo={mo:#x}"), &mut d);
                    if nl == PCRE2_NEWLINE_CRLF && subj == &b"\r"[..] && mo == PCRE2_PARTIAL_HARD {
                        want(&mut d, "174 lone CR under CRLF with PARTIAL_HARD => PARTIAL", PCRE2_ERROR_PARTIAL, rc);
                    }
                    if nl == PCRE2_NEWLINE_CRLF && subj == &b"\r"[..] && mo == PCRE2_PARTIAL_SOFT {
                        want(&mut d, "174 lone CR under CRLF with PARTIAL_SOFT => match", 1, rc);
                    }
                }
            }
            free2(p, code);
            let code = compile2(p, b"abc\\Z", &Cc::n(0).nl(nl));
            for mo in [PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT, 0] {
                let (rc, _) = run(p, code, b"abc\r", 0, mo, 2, NOMC, &format!("175 /abc\\Z/ nl={nl} mo={mo:#x}"), &mut d);
                if nl == PCRE2_NEWLINE_CRLF && mo == PCRE2_PARTIAL_HARD {
                    want(&mut d, "175 OP_EODN CRLF-split partial", PCRE2_ERROR_PARTIAL, rc);
                }
            }
            free2(p, code);
            let code = compile2(p, b"abc$", &Cc::n(0).nl(nl));
            for mo in [PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT, 0] {
                run(p, code, b"abc\r", 0, mo, 2, NOMC, &format!("175 /abc$/ nl={nl} mo={mo:#x}"), &mut d);
            }
            free2(p, code);
            let code = compile2(p, b"a.c", &Cc::n(PCRE2_MULTILINE).nl(nl));
            for mo in [PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT, 0] {
                run(p, code, b"a\r", 0, mo, 2, NOMC, &format!("174 /a.c/ nl={nl} mo={mo:#x}"), &mut d);
            }
            free2(p, code);
        }

        // ---- row 176: partial disables minlength and req_cu
        let code = compile2(p, b"abcdef", &Cc::n(0));
        want(&mut d, "176 minlength", 6u32, info_u32(&p.c, code.0, PCRE2_INFO_MINLENGTH));
        let (rc, f) = run(p, code, b"ab", 0, PCRE2_PARTIAL_SOFT, 2, NOMC, "176 short subject PARTIAL_SOFT", &mut d);
        want(&mut d, "176 the attempt still runs => PARTIAL", PCRE2_ERROR_PARTIAL, rc);
        want(&mut d, "176 ovector", vec![0usize, 2], ov(&f, 1));
        let (rc2, _) = run(p, code, b"ab", 0, 0, 2, NOMC, "176 short subject no partial", &mut d);
        want(&mut d, "176 without partial the minlength cut fires", PCRE2_ERROR_NOMATCH, rc2);
        free2(p, code);

        // randomized: every partial mode over truncated subjects
        let pats: &[&str] = &[
            "abcd", "a(b)cd", "\\d{4}", "a.*z", "(?<=ab)cd", "\\bxyz", "a\\R b", "(a|ab)(c|cd)",
            "^abc$", "\\X\\X", "a{2,4}b", "[a-c]{3}d",
        ];
        for _ in 0..800 {
            let pat = *rng.pick(pats);
            let utf = rng.chance(4);
            let Some(c2) = try_compile(p, pat.as_bytes(), &Cc::n(if utf { PCRE2_UTF } else { 0 })) else { continue };
            let full = if utf { gen_utf8(&mut rng, 6) } else { gen_ascii(&mut rng, 10) };
            let cut = rng.below(full.len() + 1);
            let subj = if utf {
                // keep the truncation on a character boundary for the UTF check
                let mut k = cut;
                while k > 0 && k < full.len() && (full[k] & 0xc0) == 0x80 {
                    k -= 1;
                }
                full[..k].to_vec()
            } else {
                full[..cut].to_vec()
            };
            let mo = *rng.pick(&[PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD, PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD, 0]);
            run(p, c2, &subj, 0, mo, 4, NOMC, &format!("170-176 rnd {pat} utf={utf} {} mo={mo:#x}", show(&subj)), &mut d);
            free2(p, c2);
        }
    }
    d.finish("CONFIGS 170-176: PARTIAL_SOFT / PARTIAL_HARD, the CRLF-split arms and the disabled optimizations");
}

// ============================================================= rows 177-181

#[test]
fn cfg_177_181_no_jit_and_copy_subject() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(1770);
    unsafe {
        // ---- row 177: PCRE2_NO_JIT is accepted and changes nothing
        for pat in ["abc", "(a)(b)", "a*", "\\d+"] {
            let code = compile2(p, pat.as_bytes(), &Cc::n(0));
            for subj in ["xxabcxx", "ab", "", "123"] {
                let (r1, f1) = run(p, code, subj.as_bytes(), 0, PCRE2_NO_JIT, 4, NOMC, &format!("177 NO_JIT {pat} {subj}"), &mut d);
                let (r2, f2) = run(p, code, subj.as_bytes(), 0, 0, 4, NOMC, &format!("177 plain {pat} {subj}"), &mut d);
                want(&mut d, &format!("177 NO_JIT rc == plain rc ({pat},{subj})"), r2, r1);
                want(&mut d, &format!("177 NO_JIT ovector == plain ({pat},{subj})"), ov(&f2, 2), ov(&f1, 2));
                want(&mut d, "177 only match_data->options records the bit", PCRE2_NO_JIT, f1.options);
            }
            free2(p, code);
        }

        // ---- row 178: COPY_MATCHED_SUBJECT survives freeing the subject
        let code = compile2(p, b"b(c)d", &Cc::n(0));
        {
            let want_bytes = b"abcde".to_vec();
            let lay = std::alloc::Layout::from_size_align(want_bytes.len() + 16, 16).unwrap();
            let buf = std::alloc::alloc_zeroed(lay);
            ptr::copy_nonoverlapping(want_bytes.as_ptr(), buf, want_bytes.len());
            let mda = (p.c.match_data_create)(4, ptr::null_mut());
            let mdb = (p.r.match_data_create)(4, ptr::null_mut());
            let (rc, f) = run_md(
                p,
                code,
                (mda, mdb),
                buf,
                want_bytes.len(),
                0,
                PCRE2_COPY_MATCHED_SUBJECT,
                NOMC,
                "178 COPY_MATCHED_SUBJECT",
                &mut d,
            );
            want(&mut d, "178 rc", 2, rc);
            want(&mut d, "178 flag set", PCRE2_MD_COPIED_SUBJECT, f.flags);
            want(&mut d, "178 subject is a private copy", Subj::Other, f.subject.clone());
            want(&mut d, "178 copy contents", Some(want_bytes.clone()), f.copied.clone());
            // free the caller's subject, then read the captures back
            std::alloc::dealloc(buf, lay);
            for i in 0..2u32 {
                let (mut pa, mut pb) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
                let (mut na, mut nb) = (usize::MAX, usize::MAX);
                let ga = (p.c.substring_get_bynumber)(mda, i, &mut pa, &mut na);
                let gb = (p.r.substring_get_bynumber)(mdb, i, &mut pb, &mut nb);
                d.eq(&format!("178 substring_get_bynumber({i}) rc"), ga, gb);
                d.eq(&format!("178 substring_get_bynumber({i}) len"), na, nb);
                if ga == 0 && gb == 0 {
                    let sa = std::slice::from_raw_parts(pa, na).to_vec();
                    let sb = std::slice::from_raw_parts(pb, nb).to_vec();
                    d.eq(&format!("178 substring_get_bynumber({i}) bytes"), sa.clone(), sb);
                    want(
                        &mut d,
                        &format!("178 substring {i} after freeing the subject"),
                        if i == 0 { b"bcd".to_vec() } else { b"c".to_vec() },
                        sa,
                    );
                }
                if !pa.is_null() {
                    (p.c.substring_free)(pa);
                }
                if !pb.is_null() {
                    (p.r.substring_free)(pb);
                }
            }
            (p.c.match_data_free)(mda);
            (p.r.match_data_free)(mdb);
        }
        free2(p, code);

        // ---- row 179: COPY_MATCHED_SUBJECT with length 0
        let code = compile2(p, b"", &Cc::n(0));
        let (rc, f) = run(p, code, b"", 0, PCRE2_COPY_MATCHED_SUBJECT, 2, NOMC, "179 empty subject", &mut d);
        want(&mut d, "179 rc", 1, rc);
        want(&mut d, "179 subject is NULL", Subj::Null, f.subject.clone());
        want(&mut d, "179 flag is still set", PCRE2_MD_COPIED_SUBJECT, f.flags);
        free2(p, code);

        // ---- row 180: reuse one match_data across two copying matches
        let code = compile2(p, b"(b+)", &Cc::n(0));
        {
            let mda = (p.c.match_data_create)(4, ptr::null_mut());
            let mdb = (p.r.match_data_create)(4, ptr::null_mut());
            for (i, s) in ["abbbc", "xbz", "bbbbbbbbbb"].iter().enumerate() {
                let buf = pad(s.as_bytes());
                let (rc, f) = run_md(
                    p,
                    code,
                    (mda, mdb),
                    buf.as_ptr(),
                    s.len(),
                    0,
                    PCRE2_COPY_MATCHED_SUBJECT,
                    NOMC,
                    &format!("180 copy #{i} {s}"),
                    &mut d,
                );
                want(&mut d, &format!("180 #{i} rc"), 2, rc);
                want(&mut d, &format!("180 #{i} flag"), PCRE2_MD_COPIED_SUBJECT, f.flags);
                want(&mut d, &format!("180 #{i} copy"), Some(s.as_bytes().to_vec()), f.copied.clone());
            }
            // ... and then a match WITHOUT the option: the flag must be cleared
            let buf = pad(b"zbz");
            let (rc, f) = run_md(p, code, (mda, mdb), buf.as_ptr(), 3, 0, 0, NOMC, "180 no-copy after copy", &mut d);
            want(&mut d, "180 rc", 2, rc);
            want(&mut d, "180 flag cleared", 0u8, f.flags);
            want(&mut d, "180 subject is the caller's again", Subj::Original, f.subject.clone());
            (p.c.match_data_free)(mda);
            (p.r.match_data_free)(mdb);
        }
        free2(p, code);

        // ---- row 181: no copy on partial or nomatch
        let code = compile2(p, b"abcd", &Cc::n(0));
        for (subj, mo, expect) in [
            (&b"ab"[..], PCRE2_COPY_MATCHED_SUBJECT | PCRE2_PARTIAL_SOFT, PCRE2_ERROR_PARTIAL),
            (&b"zzz"[..], PCRE2_COPY_MATCHED_SUBJECT, PCRE2_ERROR_NOMATCH),
        ] {
            let (rc, f) = run(p, code, subj, 0, mo, 2, NOMC, &format!("181 {} {mo:#x}", show(subj)), &mut d);
            want(&mut d, &format!("181 {} rc", show(subj)), expect, rc);
            want(&mut d, &format!("181 {} no copy made", show(subj)), 0u8, f.flags);
            want(&mut d, &format!("181 {} subject == original", show(subj)), Subj::Original, f.subject.clone());
        }
        free2(p, code);

        // randomized: COPY_MATCHED_SUBJECT over random patterns/subjects, with
        // the same match_data reused so the free-then-copy path is hammered
        let mda = (p.c.match_data_create)(8, ptr::null_mut());
        let mdb = (p.r.match_data_create)(8, ptr::null_mut());
        for _ in 0..500 {
            let pat = *rng.pick(&["a", "(a)(b)", "\\w+", "a*", "abcd", "(?<=a)b", "x"]);
            let Some(c2) = try_compile(p, pat.as_bytes(), &Cc::n(0)) else { continue };
            let subj = gen_ascii(&mut rng, 10);
            let buf = pad(&subj);
            let mo = *rng.pick(&[
                PCRE2_COPY_MATCHED_SUBJECT,
                PCRE2_COPY_MATCHED_SUBJECT | PCRE2_PARTIAL_SOFT,
                PCRE2_COPY_MATCHED_SUBJECT | PCRE2_NO_JIT,
                0,
            ]);
            run_md(
                p,
                c2,
                (mda, mdb),
                buf.as_ptr(),
                subj.len(),
                0,
                mo,
                NOMC,
                &format!("177-181 rnd {pat} {} {mo:#x}", show(&subj)),
                &mut d,
            );
            free2(p, c2);
        }
        (p.c.match_data_free)(mda);
        (p.r.match_data_free)(mdb);
    }
    d.finish("CONFIGS 177-181: PCRE2_NO_JIT and PCRE2_COPY_MATCHED_SUBJECT in every documented shape");
}

// ================================================================== row 182

#[test]
fn cfg_182_recurseloop() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        let mc = mc_new(p);
        // Patterns that can re-enter the same recursion at the same subject
        // position without consuming anything.
        let cands: &[&str] = &[
            "((?2))((?1))",
            "(?:(?1))*((?2))((?1))",
            "(a(?2))((?1))",
            "((?2)x)((?1)y)",
            "(?<a>(?&b))(?<b>(?&a))",
        ];
        let mut saw_loop = false;
        for pat in cands {
            let Some(code) = try_compile(p, pat.as_bytes(), &Cc::n(0)) else {
                continue;
            };
            for subj in ["", "a", "ab", "xy", "aaaa"] {
                let (rc, _) = run(p, code, subj.as_bytes(), 0, 0, 4, mc.t(), &format!("182 {pat} {subj}"), &mut d);
                if rc == PCRE2_ERROR_RECURSELOOP {
                    saw_loop = true;
                    // ... and with the check disabled the loop is bounded by
                    // the match/heap limits instead.
                    let (rc2, _) = run(
                        p,
                        code,
                        subj.as_bytes(),
                        0,
                        PCRE2_DISABLE_RECURSELOOP_CHECK,
                        4,
                        mc.t(),
                        &format!("182 {pat} {subj} DISABLE_RECURSELOOP_CHECK"),
                        &mut d,
                    );
                    want(
                        &mut d,
                        &format!("182 {pat}/{subj} with the check disabled is no longer RECURSELOOP"),
                        true,
                        rc2 != PCRE2_ERROR_RECURSELOOP,
                    );
                    want(
                        &mut d,
                        &format!("182 {pat}/{subj} disabled => bounded by a limit"),
                        true,
                        rc2 == PCRE2_ERROR_MATCHLIMIT
                            || rc2 == PCRE2_ERROR_DEPTHLIMIT
                            || rc2 == PCRE2_ERROR_HEAPLIMIT
                            || rc2 == PCRE2_ERROR_NOMATCH
                            || rc2 >= 0,
                    );
                }
            }
            free2(p, code);
        }
        want(&mut d, "182 at least one candidate reaches PCRE2_ERROR_RECURSELOOP", true, saw_loop);
        // Recursion that advances is not a loop, with and without the option.
        for pat in ["(a(?1)?b)", "\\((?:[^()]++|(?R))*\\)", "(?(R)a|(?R)b)"] {
            let Some(code) = try_compile(p, pat.as_bytes(), &Cc::n(0)) else { continue };
            for subj in ["aabb", "(a(b)c)", "ab", "aaa"] {
                for mo in [0, PCRE2_DISABLE_RECURSELOOP_CHECK] {
                    run(p, code, subj.as_bytes(), 0, mo, 4, mc.t(), &format!("182 ok {pat} {subj} {mo:#x}"), &mut d);
                }
            }
            free2(p, code);
        }
        mc_free(p, mc);
    }
    d.finish("CONFIGS 182: PCRE2_ERROR_RECURSELOOP and PCRE2_DISABLE_RECURSELOOP_CHECK");
}

// ============================================================= rows 183-185

#[test]
fn cfg_183_185_offset_limit() {
    let p = pair();
    let mut d = Diffs::new();
    let mut rng = Rng::new(1830);
    unsafe {
        let mc = mc_new(p);
        for extra in [0u32, PCRE2_NO_START_OPTIMIZE] {
            let row = if extra == 0 { 183 } else { 184 };
            let code = compile2(p, b"abc", &Cc::n(PCRE2_USE_OFFSET_LIMIT | extra));
            let subj = b"xxxxabc";
            // sweep the whole range so the strict `>` boundary is pinned down
            for lim in 0..=8usize {
                assert_eq!((p.c.set_offset_limit)(mc.a, lim), 0);
                assert_eq!((p.r.set_offset_limit)(mc.b, lim), 0);
                let (rc, f) = run(p, code, subj, 0, 0, 2, mc.t(), &format!("{row} offset_limit={lim}"), &mut d);
                let expect = if lim >= 4 { 1 } else { PCRE2_ERROR_NOMATCH };
                want(&mut d, &format!("{row} offset_limit={lim} rc"), expect, rc);
                if rc == 1 {
                    want(&mut d, &format!("{row} offset_limit={lim} ovector"), vec![4usize, 7], ov(&f, 1));
                }
            }
            assert_eq!((p.c.set_offset_limit)(mc.a, PCRE2_UNSET), 0);
            assert_eq!((p.r.set_offset_limit)(mc.b, PCRE2_UNSET), 0);
            run(p, code, subj, 0, 0, 2, mc.t(), &format!("{row} offset_limit UNSET"), &mut d);
            free2(p, code);
        }
        // ---- row 185: UNSET limit without the compile option is legal
        let code = compile2(p, b"abc", &Cc::n(0));
        assert_eq!((p.c.set_offset_limit)(mc.a, PCRE2_UNSET), 0);
        assert_eq!((p.r.set_offset_limit)(mc.b, PCRE2_UNSET), 0);
        let (rc, f) = run(p, code, b"xxxxabc", 0, 0, 2, mc.t(), "185 UNSET without USE_OFFSET_LIMIT", &mut d);
        want(&mut d, "185 legal", 1, rc);
        want(&mut d, "185 ovector", vec![4usize, 7], ov(&f, 1));
        // ... but any real limit is an error
        for lim in [0usize, 1, 4, 100] {
            assert_eq!((p.c.set_offset_limit)(mc.a, lim), 0);
            assert_eq!((p.r.set_offset_limit)(mc.b, lim), 0);
            let (rc, _) = run(p, code, b"xxxxabc", 0, 0, 2, mc.t(), &format!("185 limit={lim} no compile option"), &mut d);
            want(&mut d, &format!("185 limit={lim} => BADOFFSETLIMIT"), PCRE2_ERROR_BADOFFSETLIMIT, rc);
        }
        free2(p, code);

        // randomized: offset limits over many patterns/subjects/start offsets
        let pats: &[&str] = &["abc", "a", "[bc]d", "^x", "(?m)^x", "a*", "\\d+", "(?<=a)b"];
        for _ in 0..600 {
            let pat = *rng.pick(pats);
            let opts = PCRE2_USE_OFFSET_LIMIT
                | if rng.chance(3) { PCRE2_NO_START_OPTIMIZE } else { 0 }
                | if rng.chance(4) { PCRE2_MULTILINE } else { 0 };
            let Some(code) = try_compile(p, pat.as_bytes(), &Cc::n(opts)) else { continue };
            let subj = gen_ascii(&mut rng, 14);
            let lim = if rng.chance(6) { PCRE2_UNSET } else { rng.below(subj.len() + 3) };
            assert_eq!((p.c.set_offset_limit)(mc.a, lim), 0);
            assert_eq!((p.r.set_offset_limit)(mc.b, lim), 0);
            let start = rng.below(subj.len() + 1);
            run(
                p,
                code,
                &subj,
                start,
                0,
                4,
                mc.t(),
                &format!("183-185 rnd {pat} {opts:#x} {} @{start} lim={}", show(&subj), lim as i64),
                &mut d,
            );
            free2(p, code);
        }
        mc_free(p, mc);
    }
    d.finish("CONFIGS 183-185: pcre2_set_offset_limit_8 with and without PCRE2_USE_OFFSET_LIMIT");
}

// ================================================================== row 186

#[test]
fn cfg_186_memctl_sources() {
    let p = pair();
    let mut d = Diffs::new();
    unsafe {
        // Three independent counting allocators per library: one for the
        // pattern (via the compile context), one for the match context and one
        // for the match_data.  Slots: C 0/1/2, rust 3/4/5.
        let mallocs: [MallocFn; 6] = [m0, m1, m2, m3, m4, m5];
        for pat in ["a(b)c", "(a)(b)(c)(d)(e)", "(?:ab)*c"] {
            for use_mc in [false, true] {
                acnt_reset();
                let mut codes = [ptr::null_mut(); 2];
                let mut mcs = [ptr::null_mut(); 2];
                let mut mds = [ptr::null_mut(); 2];
                for (li, api) in [&p.c, &p.r].iter().enumerate() {
                    let base = li * 3;
                    let g_code = (api.general_context_create)(Some(mallocs[base]), Some(raw_free), ptr::null_mut());
                    let g_mc = (api.general_context_create)(Some(mallocs[base + 1]), Some(raw_free), ptr::null_mut());
                    let g_md = (api.general_context_create)(Some(mallocs[base + 2]), Some(raw_free), ptr::null_mut());
                    let cc = (api.compile_context_create)(g_code);
                    let (mut e, mut o) = (0 as c_int, 0usize);
                    let code = (api.compile)(pat.as_ptr(), pat.len(), 0, &mut e, &mut o, cc);
                    assert!(!code.is_null(), "[{}] compile {pat} failed: {e}", api.name);
                    (api.compile_context_free)(cc);
                    let m = if use_mc { (api.match_context_create)(g_mc) } else { ptr::null_mut() };
                    let md = (api.match_data_create)(8, g_md);
                    codes[li] = code;
                    mcs[li] = m;
                    mds[li] = md;
                    (api.general_context_free)(g_code);
                    (api.general_context_free)(g_mc);
                    (api.general_context_free)(g_md);
                }
                let before: Vec<(usize, usize)> = (0..6).map(acnt).collect();
                let buf = pad(b"xxabcxx");
                let (_rc, _f) = run_md(
                    p,
                    (codes[0], codes[1]),
                    (mds[0], mds[1]),
                    buf.as_ptr(),
                    7,
                    0,
                    0,
                    (mcs[0], mcs[1]),
                    &format!("186 {pat} use_mc={use_mc}"),
                    &mut d,
                );
                let after: Vec<(usize, usize)> = (0..6).map(acnt).collect();
                let delta: Vec<(usize, usize)> = (0..6)
                    .map(|i| (after[i].0 - before[i].0, after[i].1 - before[i].1))
                    .collect();
                // C and rust must make the identical sequence of requests
                d.eq(
                    &format!("186 {pat} use_mc={use_mc} allocator deltas"),
                    delta[0..3].to_vec(),
                    delta[3..6].to_vec(),
                );
                // and the heapframes come from the MATCH_DATA's allocator
                want(
                    &mut d,
                    &format!("186 {pat} use_mc={use_mc}: heapframes come from match_data->memctl"),
                    true,
                    delta[2].0 >= 1,
                );
                want(
                    &mut d,
                    &format!("186 {pat} use_mc={use_mc}: the code's allocator is not used at match time"),
                    0,
                    delta[0].0,
                );
                want(
                    &mut d,
                    &format!("186 {pat} use_mc={use_mc}: the match context's allocator is not used at match time"),
                    0,
                    delta[1].0,
                );
                for li in 0..2 {
                    let api = if li == 0 { &p.c } else { &p.r };
                    (api.match_data_free)(mds[li]);
                    if !mcs[li].is_null() {
                        (api.match_context_free)(mcs[li]);
                    }
                    (api.code_free)(codes[li]);
                }
            }
        }
    }
    d.finish("CONFIGS 186: which memctl the match-time allocations come from (mcontext NULL vs supplied)");
}

// __PART3__
