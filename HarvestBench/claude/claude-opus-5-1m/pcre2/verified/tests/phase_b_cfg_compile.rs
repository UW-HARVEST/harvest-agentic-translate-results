// Phase B sign-off for CONFIGS.md rows 1-153 — the whole
// `pcre2_compile_8` (+ `pcre2_code_copy_8`, `pcre2_code_copy_with_tables_8`,
// `pcre2_code_free_8`) configuration surface.
//
// Every row is driven in BOTH libraries through their `.so` and the compiled
// pattern is compared BYTE FOR BYTE (`assert_code_eq`), which transitively
// validates the parser, `_pcre2_study_8`, `_pcre2_auto_possessify_8`, the class
// compilers and the name-table helpers.  On top of that, every
// `pcre2_pattern_info_8` item is compared, both copy functions are exercised,
// and — where a row states an expected value — the C is checked against the
// row's claim (`Claims`), because the C is ground truth: a claim that fails is a
// mis-derived row, not a bug.
//
// Each row group additionally re-runs its configuration over a randomized
// pattern population with a fixed seed, so value-dependent divergences are
// reachable rather than relying on the hand-picked pattern alone.

mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;
use std::sync::OnceLock;

pub const COVERAGE: &[CfgCov] = &[
    CfgCov { cfg_rows: &[1, 2, 3, 4, 5, 6], note: "literal/length/NUL/ZERO_TERMINATED pattern shapes" },
    CfgCov { cfg_rows: &[7, 8], note: "PCRE2_ANCHORED and the REQ_VARY gate on LASTSET" },
    CfgCov { cfg_rows: &[9, 10, 11, 12], note: "auto-anchor (^, .*) and STARTLINE" },
    CfgCov { cfg_rows: &[13, 14, 15], note: "MULTILINE circumflex/dollar, inline (?m)" },
    CfgCov { cfg_rows: &[16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26], note: "caseless: CHARI/PROP-CLIST/class folding/REFI flags" },
    CfgCov { cfg_rows: &[27, 28, 29, 30, 31, 32, 33, 34, 35], note: "named groups, DUPNAMES, the name table" },
    CfgCov { cfg_rows: &[36, 37, 38, 39, 40], note: "EXTENDED / EXTENDED_MORE and #-comment newline handling" },
    CfgCov { cfg_rows: &[41, 42], note: "UNGREEDY, option and inline form" },
    CfgCov { cfg_rows: &[43, 44], note: "NO_AUTO_CAPTURE, option and inline (?n)" },
    CfgCov { cfg_rows: &[45, 46, 47, 48, 49], note: "auto-possessify and pcre2_set_optimize_8" },
    CfgCov { cfg_rows: &[50, 51], note: "NO_START_OPTIMIZE, option and (*NO_START_OPT)" },
    CfgCov { cfg_rows: &[52, 53], note: "ALLOW_EMPTY_CLASS vs the literal ] rule" },
    CfgCov { cfg_rows: &[54, 55, 56], note: "ALT_BSUX, EXTRA_ALT_BSUX, ALLOW_SURROGATE_ESCAPES" },
    CfgCov { cfg_rows: &[57, 58, 59, 60, 61, 62, 63], note: "AUTO_CALLOUT and explicit numeric/string callouts" },
    CfgCov { cfg_rows: &[64, 65, 66, 67, 68], note: "PCRE2_LITERAL and its MATCH_WORD/MATCH_LINE wrappers" },
    CfgCov { cfg_rows: &[69, 70, 71, 72, 73], note: "ESCAPED_CR_IS_LF, BAD_ESCAPE_IS_LITERAL, PYTHON_OCTAL, NO_BS0" },
    CfgCov { cfg_rows: &[74, 75, 76, 77, 78, 79, 80, 81], note: "UCP and the EXTRA_ASCII_* reversions, POSIX classes" },
    CfgCov { cfg_rows: &[82, 83], note: "every PT_* property type, and the caseless \\p{Lu} rewrite" },
    CfgCov { cfg_rows: &[84, 85, 86, 87, 88, 89, 90, 91, 92, 93], note: "CLASS/NCLASS/XCLASS/ECLASS selection and folding" },
    CfgCov { cfg_rows: &[94, 95, 96, 97], note: "the quantifier grid, per previous-item family, possessive wraps" },
    CfgCov { cfg_rows: &[98, 99, 100, 101, 102], note: "group kinds, backreference syntax, conditions" },
    CfgCov { cfg_rows: &[103, 104, 105, 106, 107, 108], note: "lookarounds, alpha assertions, max_varlookbehind" },
    CfgCov { cfg_rows: &[109, 110, 111, 112], note: "recursion forms, capture-argument lists, script runs" },
    CfgCov { cfg_rows: &[113, 114, 115, 116, 117, 118, 119], note: "verbs, (*ACCEPT) side effects, (*scs:), \\K" },
    CfgCov { cfg_rows: &[120, 121], note: "every escape and every numeric escape form" },
    CfgCov { cfg_rows: &[122, 123, 124, 125, 126], note: "newline/BSR contexts, the pso_list verbs and limits" },
    CfgCov { cfg_rows: &[127, 128, 129], note: "max_pattern_length, max_pattern_compiled_length, parens_nest_limit" },
    CfgCov { cfg_rows: &[130, 131], note: "pcre2_maketables tables and the compile recursion guard" },
    CfgCov { cfg_rows: &[132, 133, 134], note: "parsed_pattern / groupinfo heap growth, workspace margin" },
    CfgCov { cfg_rows: &[135, 136, 137, 138], note: "NEVER_UTF/UCP, MATCH_INVALID_UTF, NO_UTF_CHECK, store-only bits" },
    CfgCov { cfg_rows: &[139, 140], note: "the inline option grid and option scoping" },
    CfgCov { cfg_rows: &[141, 142, 143, 144, 145], note: "_pcre2_study_8: first asserted cu and the start bitmap" },
    CfgCov { cfg_rows: &[146, 147, 148, 149], note: "MINLENGTH, HASCRORLF, MATCHEMPTY, MAXLOOKBEHIND" },
    CfgCov { cfg_rows: &[150, 151, 152, 153], note: "pcre2_code_copy_8 / _with_tables_8 / pcre2_code_free_8(NULL)" },
];

#[test]
fn coverage_declaration_is_sane() {
    check_coverage_decl(COVERAGE);
}

// ===================================================================== opcodes

#[allow(dead_code)]
mod op {
    pub const END: u8 = 0;
    pub const SOD: u8 = 1;
    pub const SOM: u8 = 2;
    pub const SET_SOM: u8 = 3;
    pub const NOT_WORD_BOUNDARY: u8 = 4;
    pub const WORD_BOUNDARY: u8 = 5;
    pub const NOT_DIGIT: u8 = 6;
    pub const DIGIT: u8 = 7;
    pub const NOT_WHITESPACE: u8 = 8;
    pub const WHITESPACE: u8 = 9;
    pub const NOT_WORDCHAR: u8 = 10;
    pub const WORDCHAR: u8 = 11;
    pub const ANY: u8 = 12;
    pub const ALLANY: u8 = 13;
    pub const ANYBYTE: u8 = 14;
    pub const NOTPROP: u8 = 15;
    pub const PROP: u8 = 16;
    pub const ANYNL: u8 = 17;
    pub const NOT_HSPACE: u8 = 18;
    pub const HSPACE: u8 = 19;
    pub const NOT_VSPACE: u8 = 20;
    pub const VSPACE: u8 = 21;
    pub const EXTUNI: u8 = 22;
    pub const EODN: u8 = 23;
    pub const EOD: u8 = 24;
    pub const DOLL: u8 = 25;
    pub const DOLLM: u8 = 26;
    pub const CIRC: u8 = 27;
    pub const CIRCM: u8 = 28;
    pub const CHAR: u8 = 29;
    pub const CHARI: u8 = 30;
    pub const NOT: u8 = 31;
    pub const NOTI: u8 = 32;
    pub const STAR: u8 = 33;
    pub const MINSTAR: u8 = 34;
    pub const PLUS: u8 = 35;
    pub const MINPLUS: u8 = 36;
    pub const QUERY: u8 = 37;
    pub const MINQUERY: u8 = 38;
    pub const UPTO: u8 = 39;
    pub const MINUPTO: u8 = 40;
    pub const EXACT: u8 = 41;
    pub const POSSTAR: u8 = 42;
    pub const POSPLUS: u8 = 43;
    pub const POSQUERY: u8 = 44;
    pub const POSUPTO: u8 = 45;
    pub const STARI: u8 = 46;
    pub const PLUSI: u8 = 48;
    pub const QUERYI: u8 = 50;
    pub const UPTOI: u8 = 52;
    pub const EXACTI: u8 = 54;
    pub const NOTSTAR: u8 = 59;
    pub const NOTPLUS: u8 = 61;
    pub const NOTQUERY: u8 = 63;
    pub const NOTUPTO: u8 = 65;
    pub const NOTEXACT: u8 = 67;
    pub const NOTSTARI: u8 = 72;
    pub const TYPESTAR: u8 = 85;
    pub const TYPEMINSTAR: u8 = 86;
    pub const TYPEPLUS: u8 = 87;
    pub const TYPEMINPLUS: u8 = 88;
    pub const TYPEQUERY: u8 = 89;
    pub const TYPEMINQUERY: u8 = 90;
    pub const TYPEUPTO: u8 = 91;
    pub const TYPEMINUPTO: u8 = 92;
    pub const TYPEEXACT: u8 = 93;
    pub const TYPEPOSSTAR: u8 = 94;
    pub const TYPEPOSPLUS: u8 = 95;
    pub const TYPEPOSQUERY: u8 = 96;
    pub const TYPEPOSUPTO: u8 = 97;
    pub const CRSTAR: u8 = 98;
    pub const CRMINSTAR: u8 = 99;
    pub const CRPLUS: u8 = 100;
    pub const CRMINPLUS: u8 = 101;
    pub const CRQUERY: u8 = 102;
    pub const CRMINQUERY: u8 = 103;
    pub const CRRANGE: u8 = 104;
    pub const CRMINRANGE: u8 = 105;
    pub const CRPOSSTAR: u8 = 106;
    pub const CRPOSPLUS: u8 = 107;
    pub const CRPOSQUERY: u8 = 108;
    pub const CRPOSRANGE: u8 = 109;
    pub const CLASS: u8 = 110;
    pub const NCLASS: u8 = 111;
    pub const XCLASS: u8 = 112;
    pub const ECLASS: u8 = 113;
    pub const REF: u8 = 114;
    pub const REFI: u8 = 115;
    pub const DNREF: u8 = 116;
    pub const DNREFI: u8 = 117;
    pub const RECURSE: u8 = 118;
    pub const CALLOUT: u8 = 119;
    pub const CALLOUT_STR: u8 = 120;
    pub const ALT: u8 = 121;
    pub const KET: u8 = 122;
    pub const KETRMAX: u8 = 123;
    pub const KETRMIN: u8 = 124;
    pub const KETRPOS: u8 = 125;
    pub const REVERSE: u8 = 126;
    pub const VREVERSE: u8 = 127;
    pub const ASSERT: u8 = 128;
    pub const ASSERT_NOT: u8 = 129;
    pub const ASSERTBACK: u8 = 130;
    pub const ASSERTBACK_NOT: u8 = 131;
    pub const ASSERT_NA: u8 = 132;
    pub const ASSERTBACK_NA: u8 = 133;
    pub const ASSERT_SCS: u8 = 134;
    pub const ONCE: u8 = 135;
    pub const SCRIPT_RUN: u8 = 136;
    pub const BRA: u8 = 137;
    pub const BRAPOS: u8 = 138;
    pub const CBRA: u8 = 139;
    pub const CBRAPOS: u8 = 140;
    pub const COND: u8 = 141;
    pub const SBRA: u8 = 142;
    pub const SBRAPOS: u8 = 143;
    pub const SCBRA: u8 = 144;
    pub const SCBRAPOS: u8 = 145;
    pub const SCOND: u8 = 146;
    pub const CREF: u8 = 147;
    pub const DNCREF: u8 = 148;
    pub const RREF: u8 = 149;
    pub const DNRREF: u8 = 150;
    pub const FALSE: u8 = 151;
    pub const TRUE: u8 = 152;
    pub const BRAZERO: u8 = 153;
    pub const BRAMINZERO: u8 = 154;
    pub const BRAPOSZERO: u8 = 155;
    pub const MARK: u8 = 156;
    pub const PRUNE: u8 = 157;
    pub const PRUNE_ARG: u8 = 158;
    pub const SKIP: u8 = 159;
    pub const SKIP_ARG: u8 = 160;
    pub const THEN: u8 = 161;
    pub const THEN_ARG: u8 = 162;
    pub const COMMIT: u8 = 163;
    pub const COMMIT_ARG: u8 = 164;
    pub const FAIL: u8 = 165;
    pub const ACCEPT: u8 = 166;
    pub const ASSERT_ACCEPT: u8 = 167;
    pub const CLOSE: u8 = 168;
    pub const SKIPZERO: u8 = 169;
    pub const DEFINE: u8 = 170;
    pub const NOT_UCP_WORD_BOUNDARY: u8 = 171;
    pub const UCP_WORD_BOUNDARY: u8 = 172;
}

// property types (`pcre2_internal.h`)
const PT_LAMP: u8 = 0;
const PT_GC: u8 = 1;
const PT_PC: u8 = 2;
const PT_SC: u8 = 3;
const PT_SCX: u8 = 4;
const PT_ALNUM: u8 = 5;
const PT_SPACE: u8 = 6;
const PT_PXSPACE: u8 = 7;
const PT_WORD: u8 = 8;
const PT_CLIST: u8 = 9;
const PT_UCNC: u8 = 10;
const PT_BIDICL: u8 = 11;
const PT_BOOL: u8 = 12;
const PT_ANY: u8 = 13;

// XCLASS / ECLASS flag bits
const XCL_NOT: u8 = 0x01;
const XCL_MAP: u8 = 0x02;
const XCL_HASPROP: u8 = 0x04;
const XCL_LIST: u8 = 0x10;
const ECL_MAP: u8 = 0x01;

// private `flags` bits (`pcre2_internal.h`)
const F_FIRSTSET: u32 = 0x0000_0010;
const F_FIRSTCASELESS: u32 = 0x0000_0020;
const F_FIRSTMAPSET: u32 = 0x0000_0040;
const F_LASTSET: u32 = 0x0000_0080;
const F_LASTCASELESS: u32 = 0x0000_0100;
const F_STARTLINE: u32 = 0x0000_0200;
const F_JCHANGED: u32 = 0x0000_0400;
const F_HASCRORLF: u32 = 0x0000_0800;
const F_HASTHEN: u32 = 0x0000_1000;
const F_MATCH_EMPTY: u32 = 0x0000_2000;
const F_BSR_SET: u32 = 0x0000_4000;
const F_NL_SET: u32 = 0x0000_8000;
const F_NOTEMPTY_SET: u32 = 0x0001_0000;
const F_NE_ATST_SET: u32 = 0x0002_0000;
const F_DEREF_TABLES: u32 = 0x0004_0000;
const F_NOJIT: u32 = 0x0008_0000;
const F_HASBKPORX: u32 = 0x0010_0000;
const F_DUPCAPUSED: u32 = 0x0020_0000;
const F_HASBKC: u32 = 0x0040_0000;
const F_HASACCEPT: u32 = 0x0080_0000;
const F_HASBSK: u32 = 0x0100_0000;

const LINK_SIZE: usize = 2;
const IMM2_SIZE: usize = 2;

// REFI flag byte bits (`pcre2_internal.h`)
const REFI_CASELESS_RESTRICT: u8 = 0x1;
const REFI_TURKISH_CASING: u8 = 0x2;

// ============================================================ row-claim checks

/// Collects the places where the C disagrees with what a CONFIGS row claims.
/// The C is ground truth, so a non-empty report means the row text is wrong.
struct Claims {
    bad: Vec<String>,
    n: usize,
}

impl Claims {
    fn new() -> Claims {
        Claims { bad: Vec::new(), n: 0 }
    }
    fn is<T: PartialEq + std::fmt::Debug>(&mut self, what: &str, c_says: T, row_says: T) {
        self.n += 1;
        if c_says != row_says {
            self.bad.push(format!(
                "{what}\n     C  produces = {c_says:?}\n     row claims  = {row_says:?}"
            ));
        }
    }
    fn ok(&mut self, what: &str, cond: bool) {
        self.is(what, cond, true);
    }
    fn finish(self, rows: &str) {
        assert!(self.n > 0, "rows [{rows}] checked no row claim");
        if !self.bad.is_empty() {
            panic!(
                "CONFIGS rows [{}]: {} of {} row claims disagree with the C:\n\n{}",
                rows,
                self.bad.len(),
                self.n,
                self.bad.join("\n\n")
            );
        }
        println!("rows [{rows}]: {} row claims confirmed against the C", self.n);
    }
}

// ================================================================ compile cfgs

/// One compile configuration: option bits plus compile-context state.
#[derive(Clone, Copy, Debug)]
struct Cfg {
    name: &'static str,
    opts: u32,
    xopts: u32,
    newline: u32,
    bsr: u32,
    varlookbehind: Option<u32>,
    parens_limit: Option<u32>,
    optimize: &'static [u32],
    own_tables: bool,
    max_pat_len: Option<Sz>,
    max_compiled_len: Option<Sz>,
    guard: bool,
    null_ctx: bool,
}

impl Cfg {
    const fn new(name: &'static str, opts: u32, xopts: u32) -> Cfg {
        Cfg {
            name,
            opts,
            xopts,
            newline: 0,
            bsr: 0,
            varlookbehind: None,
            parens_limit: None,
            optimize: &[],
            own_tables: false,
            max_pat_len: None,
            max_compiled_len: None,
            guard: false,
            null_ctx: false,
        }
    }
    fn nl(mut self, v: u32) -> Cfg {
        self.newline = v;
        self
    }
    fn bsr(mut self, v: u32) -> Cfg {
        self.bsr = v;
        self
    }
    fn vlb(mut self, v: u32) -> Cfg {
        self.varlookbehind = Some(v);
        self
    }
    fn parens(mut self, v: u32) -> Cfg {
        self.parens_limit = Some(v);
        self
    }
    fn optim(mut self, v: &'static [u32]) -> Cfg {
        self.optimize = v;
        self
    }
    fn tables(mut self) -> Cfg {
        self.own_tables = true;
        self
    }
    fn maxlen(mut self, v: Sz) -> Cfg {
        self.max_pat_len = Some(v);
        self
    }
    fn maxcomp(mut self, v: Sz) -> Cfg {
        self.max_compiled_len = Some(v);
        self
    }
    fn guarded(mut self) -> Cfg {
        self.guard = true;
        self
    }
    fn nullctx(mut self) -> Cfg {
        self.null_ctx = true;
        self
    }
}

/// The tables produced by `pcre2_maketables` are BORROWED by every pattern
/// compiled against them (`re->tables = tables`), so they are built once and
/// kept for the lifetime of the process.
fn locale_tables(api: &Api) -> *const u8 {
    static C_T: OnceLock<usize> = OnceLock::new();
    static R_T: OnceLock<usize> = OnceLock::new();
    let cell = if api.name == "C" { &C_T } else { &R_T };
    *cell.get_or_init(|| {
        let t = unsafe { (api.maketables)(ptr::null_mut()) };
        assert!(!t.is_null(), "[{}] pcre2_maketables_8 failed", api.name);
        t as usize
    }) as *const u8
}

// row 131: the compile recursion guard must be called with the same depths, in
// the same order, by both libraries.  One log per library, keyed by user data.
static mut GUARD_C: Vec<u32> = Vec::new();
static mut GUARD_R: Vec<u32> = Vec::new();

unsafe extern "C" fn guard_ok(depth: u32, data: *mut c_void) -> c_int {
    let log = if data.is_null() {
        &mut *ptr::addr_of_mut!(GUARD_C)
    } else {
        &mut *ptr::addr_of_mut!(GUARD_R)
    };
    if log.len() < 4096 {
        log.push(depth);
    }
    0
}

unsafe fn make_ctx(api: &Api, cfg: &Cfg) -> Ptr {
    if cfg.null_ctx {
        return ptr::null_mut();
    }
    let cc = (api.compile_context_create)(ptr::null_mut());
    assert!(!cc.is_null(), "[{}] compile_context_create failed", api.name);
    if cfg.newline != 0 {
        assert_eq!((api.set_newline)(cc, cfg.newline), 0);
    }
    if cfg.bsr != 0 {
        assert_eq!((api.set_bsr)(cc, cfg.bsr), 0);
    }
    if let Some(v) = cfg.varlookbehind {
        assert_eq!((api.set_max_varlookbehind)(cc, v), 0);
    }
    if let Some(v) = cfg.parens_limit {
        assert_eq!((api.set_parens_nest_limit)(cc, v), 0);
    }
    if cfg.xopts != 0 {
        assert_eq!((api.set_compile_extra_options)(cc, cfg.xopts), 0);
    }
    for &o in cfg.optimize {
        assert_eq!((api.set_optimize)(cc, o), 0, "set_optimize({o})");
    }
    if cfg.own_tables {
        assert_eq!((api.set_character_tables)(cc, locale_tables(api)), 0);
    }
    if let Some(v) = cfg.max_pat_len {
        assert_eq!((api.set_max_pattern_length)(cc, v), 0);
    }
    if let Some(v) = cfg.max_compiled_len {
        assert_eq!((api.set_max_pattern_compiled_length)(cc, v), 0);
    }
    if cfg.guard {
        let data = if api.name == "C" { ptr::null_mut() } else { 1usize as *mut c_void };
        assert_eq!((api.set_compile_recursion_guard)(cc, Some(guard_ok), data), 0);
    }
    cc
}

// ======================================================= the compiled-pair type

/// A pattern compiled in both libraries, already proven byte-identical.
/// The accessors read the C side (the ground truth).
struct Both<'a> {
    p: &'a Pair,
    a: Ptr,
    b: Ptr,
}

impl Drop for Both<'_> {
    fn drop(&mut self) {
        unsafe {
            (self.p.c.code_free)(self.a);
            (self.p.r.code_free)(self.b);
        }
    }
}

/// `_pcre2_OP_lengths_8`, read out of the C library once (row 445 proves the
/// two libraries' tables are identical).
fn op_lengths(p: &Pair) -> &'static [u8] {
    static T: OnceLock<Vec<u8>> = OnceLock::new();
    T.get_or_init(|| unsafe { std::slice::from_raw_parts(p.c.data("_pcre2_OP_lengths_8"), 173).to_vec() })
}

fn get2(b: &[u8], o: usize) -> usize {
    ((b[o] as usize) << 8) | b[o + 1] as usize
}

fn utf8_extra(lead: u8) -> usize {
    match lead {
        0xc0..=0xdf => 1,
        0xe0..=0xef => 2,
        0xf0..=0xf7 => 3,
        0xf8..=0xfb => 4,
        _ => 5,
    }
}

/// Walks the compiled bytecode exactly the way `_pcre2_find_bracket_8` does,
/// yielding (offset, opcode) for every item.
fn walk(by: &[u8], oplen: &[u8], utf: bool) -> Vec<(usize, u8)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < by.len() {
        let c = by[i];
        out.push((i, c));
        if c == op::END || out.len() > 100_000 {
            break;
        }
        if c == op::XCLASS || c == op::ECLASS {
            let l = get2(by, i + 1);
            if l == 0 {
                break;
            }
            i += l;
            continue;
        }
        if c == op::CALLOUT_STR {
            let l = get2(by, i + 1 + 2 * LINK_SIZE);
            if l == 0 {
                break;
            }
            i += l;
            continue;
        }
        let mut adv = 0usize;
        match c {
            op::TYPESTAR | op::TYPEMINSTAR | op::TYPEPLUS | op::TYPEMINPLUS | op::TYPEQUERY
            | op::TYPEMINQUERY | op::TYPEPOSSTAR | op::TYPEPOSPLUS | op::TYPEPOSQUERY => {
                if by[i + 1] == op::PROP || by[i + 1] == op::NOTPROP {
                    adv += 2;
                }
            }
            op::TYPEUPTO | op::TYPEMINUPTO | op::TYPEEXACT | op::TYPEPOSUPTO => {
                if by[i + 1 + IMM2_SIZE] == op::PROP || by[i + 1 + IMM2_SIZE] == op::NOTPROP {
                    adv += 2;
                }
            }
            op::MARK | op::COMMIT_ARG | op::PRUNE_ARG | op::SKIP_ARG | op::THEN_ARG => {
                adv += by[i + 1] as usize;
            }
            _ => {}
        }
        adv += oplen[c as usize] as usize;
        // opcodes followed by a character may be followed by a multi-byte one
        if utf && (29..=84).contains(&c) {
            let lead = by[i + adv - 1];
            if lead >= 0xc0 {
                adv += utf8_extra(lead);
            }
        }
        if adv == 0 || i + adv > by.len() {
            break;
        }
        i += adv;
    }
    out
}

impl<'a> Both<'a> {
    fn u32info(&self, what: u32) -> u32 {
        let mut v = 0xDEAD_BEEFu32;
        let rc = unsafe { (self.p.c.pattern_info)(self.a, what, &mut v as *mut u32 as Ptr) };
        assert_eq!(rc, 0, "pattern_info({what}) failed: {rc}");
        v
    }
    fn info_rc(&self, what: u32) -> c_int {
        let mut v = 0u32;
        unsafe { (self.p.c.pattern_info)(self.a, what, &mut v as *mut u32 as Ptr) }
    }
    fn szinfo(&self, what: u32) -> usize {
        let mut v = usize::MAX;
        let rc = unsafe { (self.p.c.pattern_info)(self.a, what, &mut v as *mut usize as Ptr) };
        assert_eq!(rc, 0, "pattern_info({what}) failed: {rc}");
        v
    }
    fn head(&self) -> &RealCodeHead {
        unsafe { &*(self.a as *const RealCodeHead) }
    }
    fn flags(&self) -> u32 {
        self.head().flags
    }
    fn utf(&self) -> bool {
        self.head().overall_options & PCRE2_UTF != 0
    }
    /// The whole name-table + bytecode region.
    fn block(&self) -> Vec<u8> {
        unsafe {
            let h = self.head();
            std::slice::from_raw_parts((self.a as *const u8).add(h.code_start), h.blocksize - h.code_start)
                .to_vec()
        }
    }
    /// The bytecode only (after the name table).
    fn code(&self) -> Vec<u8> {
        unsafe {
            let s = bytecode_ptr(self.a);
            let n = code_blocksize(self.a) - (s as usize - self.a as usize);
            std::slice::from_raw_parts(s, n).to_vec()
        }
    }
    fn ops(&self) -> Vec<(usize, u8)> {
        walk(&self.code(), op_lengths(self.p), self.utf())
    }
    /// Opcode of the first item inside the outermost bracket.
    fn first_op(&self) -> u8 {
        let o = self.ops();
        assert!(o.len() >= 2, "bytecode too short");
        o[1].1
    }
    fn opseq(&self) -> Vec<u8> {
        self.ops().iter().map(|x| x.1).collect()
    }
    fn count_op(&self, want: u8) -> usize {
        self.ops().iter().filter(|x| x.1 == want).count()
    }
    fn has(&self, want: u8) -> bool {
        self.count_op(want) > 0
    }
    fn find_op(&self, want: u8) -> Option<usize> {
        self.ops().iter().find(|x| x.1 == want).map(|x| x.0)
    }
    /// The opcode that follows the first occurrence of `want`.
    fn op_after(&self, want: u8) -> Option<u8> {
        let o = self.ops();
        o.iter().position(|x| x.1 == want).and_then(|i| o.get(i + 1).map(|x| x.1))
    }
    /// The single-byte operands of every OP_CHAR / OP_CHARI, in order.
    fn char_ops(&self) -> Vec<u8> {
        let by = self.code();
        self.ops()
            .iter()
            .filter(|x| x.1 == op::CHAR || x.1 == op::CHARI)
            .map(|x| by[x.0 + 1])
            .collect()
    }
    /// The callout numbers of every OP_CALLOUT, in order.
    fn callout_numbers(&self) -> Vec<u8> {
        let by = self.code();
        self.ops()
            .iter()
            .filter(|x| x.1 == op::CALLOUT)
            .map(|x| by[x.0 + 1 + 2 * LINK_SIZE])
            .collect()
    }
    /// The byte at `off` of the bytecode.
    fn at(&self, off: usize) -> u8 {
        self.code()[off]
    }
    fn nametable(&self) -> Vec<u8> {
        let n = self.u32info(PCRE2_INFO_NAMECOUNT) as usize;
        let sz = self.u32info(PCRE2_INFO_NAMEENTRYSIZE) as usize;
        if n == 0 {
            return Vec::new();
        }
        let mut t = ptr::null::<u8>();
        unsafe {
            (self.p.c.pattern_info)(self.a, PCRE2_INFO_NAMETABLE, &mut t as *mut _ as Ptr);
            std::slice::from_raw_parts(t, n * sz).to_vec()
        }
    }
    fn firstbitmap(&self) -> Option<[u8; 32]> {
        let mut t = ptr::null::<u8>();
        unsafe {
            (self.p.c.pattern_info)(self.a, PCRE2_INFO_FIRSTBITMAP, &mut t as *mut _ as Ptr);
            if t.is_null() {
                None
            } else {
                let mut o = [0u8; 32];
                o.copy_from_slice(std::slice::from_raw_parts(t, 32));
                Some(o)
            }
        }
    }
    /// FIRSTSET as (codetype, codeunit).
    fn first(&self) -> (u32, u32) {
        (self.u32info(PCRE2_INFO_FIRSTCODETYPE), self.u32info(PCRE2_INFO_FIRSTCODEUNIT))
    }
    fn last(&self) -> (u32, u32) {
        (self.u32info(PCRE2_INFO_LASTCODETYPE), self.u32info(PCRE2_INFO_LASTCODEUNIT))
    }
    fn minlen(&self) -> u32 {
        self.u32info(PCRE2_INFO_MINLENGTH)
    }
    /// The 32-byte bitmap of the OP_CLASS/OP_NCLASS at `off`.
    fn class_map(&self, off: usize) -> [u8; 32] {
        let by = self.code();
        let mut o = [0u8; 32];
        o.copy_from_slice(&by[off + 1..off + 33]);
        o
    }
}

fn bit(map: &[u8; 32], c: u8) -> bool {
    map[(c >> 3) as usize] & (1 << (c & 7)) != 0
}

fn map_of(cs: &[u8]) -> [u8; 32] {
    let mut m = [0u8; 32];
    for &c in cs {
        m[(c >> 3) as usize] |= 1 << (c & 7);
    }
    m
}

// ============================================================ the row drivers

/// Compiles `pat` (as a raw pointer, so `NULL` patterns are expressible) in both
/// libraries, asserts they agree on success/failure and are byte-identical, then
/// compares every `pcre2_pattern_info_8` item and both copy functions.
unsafe fn compile_ptr<'a>(
    p: &'a Pair,
    pat: Sptr,
    len: Sz,
    cfg: &Cfg,
    d: &mut Diffs,
    copies: bool,
) -> Option<Both<'a>> {
    let cca = make_ctx(&p.c, cfg);
    let ccb = make_ctx(&p.r, cfg);
    let (mut eca, mut ecb) = (0 as c_int, 0 as c_int);
    let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
    let a = (p.c.compile)(pat, len, cfg.opts, &mut eca, &mut eoa, cca);
    let b = (p.r.compile)(pat, len, cfg.opts, &mut ecb, &mut eob, ccb);
    if !cca.is_null() {
        (p.c.compile_context_free)(cca);
    }
    if !ccb.is_null() {
        (p.r.compile_context_free)(ccb);
    }
    let shown = if pat.is_null() {
        "<NULL>".to_string()
    } else if len == PCRE2_ZERO_TERMINATED {
        show(std::slice::from_raw_parts(pat, (p.c.p_strlen)(pat)))
    } else {
        show(std::slice::from_raw_parts(pat, len))
    };
    let tag = format!("compile({shown}) cfg[{}]", cfg.name);
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
    let bo = Both { p, a, b };
    compare_info(&bo, &tag, d);
    // the op walk, using each library's own bytecode
    {
        let t = op_lengths(p);
        let utf = bo.utf();
        let sa = bo.code();
        let s = bytecode_ptr(b);
        let n = code_blocksize(b) - (s as usize - b as usize);
        let sb = std::slice::from_raw_parts(s, n).to_vec();
        d.eq(&format!("{tag} op walk"), walk(&sa, t, utf), walk(&sb, t, utf));
    }
    if copies {
        compare_copies(&bo, &tag, d);
    }
    Some(bo)
}

unsafe fn compile_both<'a>(p: &'a Pair, pat: &[u8], cfg: &Cfg, d: &mut Diffs) -> Option<Both<'a>> {
    compile_ptr(p, pat.as_ptr(), pat.len(), cfg, d, true)
}

/// Row 1's `abc`-shaped helper: compile a `&str` pattern under `cfg`.
unsafe fn c1<'a>(p: &'a Pair, pat: &str, cfg: &Cfg, d: &mut Diffs) -> Both<'a> {
    match compile_both(p, pat.as_bytes(), cfg, d) {
        Some(b) => b,
        None => panic!("cfg[{}] failed to compile {}", cfg.name, show(pat.as_bytes())),
    }
}

/// Every `pcre2_pattern_info_8` item, using the right result width.
unsafe fn compare_info(bo: &Both, tag: &str, d: &mut Diffs) {
    let (p, a, b) = (bo.p, bo.a, bo.b);
    for what in [
        PCRE2_INFO_ARGOPTIONS,
        PCRE2_INFO_ALLOPTIONS,
        PCRE2_INFO_EXTRAOPTIONS,
        PCRE2_INFO_BACKREFMAX,
        PCRE2_INFO_BSR,
        PCRE2_INFO_CAPTURECOUNT,
        PCRE2_INFO_FIRSTCODEUNIT,
        PCRE2_INFO_FIRSTCODETYPE,
        PCRE2_INFO_HASCRORLF,
        PCRE2_INFO_JCHANGED,
        PCRE2_INFO_LASTCODEUNIT,
        PCRE2_INFO_LASTCODETYPE,
        PCRE2_INFO_MATCHEMPTY,
        PCRE2_INFO_MATCHLIMIT,
        PCRE2_INFO_MAXLOOKBEHIND,
        PCRE2_INFO_MINLENGTH,
        PCRE2_INFO_NAMECOUNT,
        PCRE2_INFO_NAMEENTRYSIZE,
        PCRE2_INFO_NEWLINE,
        PCRE2_INFO_DEPTHLIMIT,
        PCRE2_INFO_HASBACKSLASHC,
        PCRE2_INFO_HEAPLIMIT,
    ] {
        let (mut va, mut vb) = (0xDEAD_BEEFu32, 0xDEAD_BEEFu32);
        let ra = (p.c.pattern_info)(a, what, &mut va as *mut u32 as Ptr);
        let rb = (p.r.pattern_info)(b, what, &mut vb as *mut u32 as Ptr);
        d.eq(&format!("info[{what}] rc {tag}"), ra, rb);
        d.eq(&format!("info[{what}] val {tag}"), va, vb);
    }
    for what in [PCRE2_INFO_SIZE, PCRE2_INFO_FRAMESIZE, PCRE2_INFO_JITSIZE] {
        let (mut va, mut vb) = (usize::MAX, usize::MAX);
        let ra = (p.c.pattern_info)(a, what, &mut va as *mut usize as Ptr);
        let rb = (p.r.pattern_info)(b, what, &mut vb as *mut usize as Ptr);
        d.eq(&format!("info[{what}] rc {tag}"), ra, rb);
        d.eq(&format!("info[{what}] val {tag}"), va, vb);
    }
    {
        let (mut pa, mut pb) = (ptr::null::<u8>(), ptr::null::<u8>());
        let ra = (p.c.pattern_info)(a, PCRE2_INFO_FIRSTBITMAP, &mut pa as *mut _ as Ptr);
        let rb = (p.r.pattern_info)(b, PCRE2_INFO_FIRSTBITMAP, &mut pb as *mut _ as Ptr);
        d.eq(&format!("info[FIRSTBITMAP] rc {tag}"), ra, rb);
        d.eq(&format!("info[FIRSTBITMAP] null {tag}"), pa.is_null(), pb.is_null());
        if !pa.is_null() && !pb.is_null() {
            d.eq(
                &format!("info[FIRSTBITMAP] bytes {tag}"),
                std::slice::from_raw_parts(pa, 32).to_vec(),
                std::slice::from_raw_parts(pb, 32).to_vec(),
            );
        }
    }
    {
        let (mut na, mut nb) = (0u32, 0u32);
        (p.c.pattern_info)(a, PCRE2_INFO_NAMECOUNT, &mut na as *mut u32 as Ptr);
        (p.r.pattern_info)(b, PCRE2_INFO_NAMECOUNT, &mut nb as *mut u32 as Ptr);
        let (mut sa, mut sb) = (0u32, 0u32);
        (p.c.pattern_info)(a, PCRE2_INFO_NAMEENTRYSIZE, &mut sa as *mut u32 as Ptr);
        (p.r.pattern_info)(b, PCRE2_INFO_NAMEENTRYSIZE, &mut sb as *mut u32 as Ptr);
        if na == nb && sa == sb && na > 0 {
            let (mut ta, mut tb) = (ptr::null::<u8>(), ptr::null::<u8>());
            (p.c.pattern_info)(a, PCRE2_INFO_NAMETABLE, &mut ta as *mut _ as Ptr);
            (p.r.pattern_info)(b, PCRE2_INFO_NAMETABLE, &mut tb as *mut _ as Ptr);
            let n = (na * sa) as usize;
            d.eq(
                &format!("info[NAMETABLE] {tag}"),
                std::slice::from_raw_parts(ta, n).to_vec(),
                std::slice::from_raw_parts(tb, n).to_vec(),
            );
            for i in 0..na {
                let ent = ta.add((i * sa) as usize);
                let name = ent.add(2);
                d.eq(
                    &format!("substring_number_from_name #{i} {tag}"),
                    (p.c.substring_number_from_name)(a, name),
                    (p.r.substring_number_from_name)(b, name),
                );
            }
        }
    }
}

/// `pcre2_code_copy_8` and `pcre2_code_copy_with_tables_8` on the pair.
unsafe fn compare_copies(bo: &Both, tag: &str, d: &mut Diffs) {
    let p = bo.p;
    let ca = (p.c.code_copy)(bo.a);
    let cb = (p.r.code_copy)(bo.b);
    assert!(!ca.is_null() && !cb.is_null(), "code_copy failed {tag}");
    assert_code_eq(ca, cb, &format!("code_copy {tag}"));
    assert_code_eq(bo.a, ca, &format!("code_copy vs original (C) {tag}"));
    assert_code_eq(bo.b, cb, &format!("code_copy vs original (rust) {tag}"));
    let ta = (p.c.code_copy_with_tables)(bo.a);
    let tb = (p.r.code_copy_with_tables)(bo.b);
    assert!(!ta.is_null() && !tb.is_null(), "code_copy_with_tables failed {tag}");
    assert_code_eq(ta, tb, &format!("code_copy_with_tables {tag}"));
    // the clone owns its tables, so DEREF_TABLES is the one legitimate delta
    assert_code_eq_masked(bo.a, ta, F_DEREF_TABLES, &format!("copy_with_tables vs original {tag}"));
    d.checked += 3;
    (p.c.code_free)(ca);
    (p.r.code_free)(cb);
    (p.c.code_free)(ta);
    (p.r.code_free)(tb);
}

// ====================================================== randomized populations

/// Pattern fragments that reach the distinct parser/codegen arms.  Concatenating
/// them at random produces both valid and invalid patterns; the two libraries
/// must agree either way.
const FRAGS: &[&str] = &[
    "a", "b", "Z", "0", "_", ".", "\\d", "\\w", "\\s", "\\D", "\\S", "\\W", "\\h", "\\v", "\\R",
    "\\N", "\\X", "[abc]", "[^a-c]", "[[:alpha:]]", "[\\d\\s]", "[a-\\xff]", "[]a]", "[\\Q]\\E]",
    "\\p{L}", "\\P{Lu}", "\\p{Greek}", "\\x{100}", "\\x{ff}", "(a)", "(?:b)", "(?>c)", "(?<n1>x)",
    "(?'n2'y)", "(?|(a)|(b))", "*", "+", "?", "{2,4}", "{0,2}", "{3}", "*?", "+?", "??", "*+",
    "++", "?+", "{2,4}+", "|", "^", "$", "\\b", "\\B", "\\A", "\\z", "\\Z", "\\G", "\\K", "\\1",
    "\\k<n1>", "\\g{-1}", "(?=a)", "(?!b)", "(?<=ab)", "(?<!c)", "(?*a)", "(?i)", "(?-i)", "(?s)",
    "(?m)", "(?x)", "(?xx)", "(?U)", "(?J)", "(?n)", "(?aD)", "(?^i)", "(*MARK:m)", "(*SKIP)",
    "(*PRUNE)", "(*THEN)", "(*COMMIT)", "(*ACCEPT)", "(*FAIL)", "(?C)", "(?C1)", "(?C{s})",
    "(?#cmt)", "(?(1)a|b)", "(?(R)x|y)", "(?(DEFINE)(?<w>\\w))", "(?&n1)", "(?R)", "(?1)",
    "\\x41", "\\x{41}", "\\101", "\\o{101}", "\\cA", "\\Qa.b\\E", "\\e", "\\n", "\\r", "\\t",
    "(*script_run:\\w+)", "(*sr:a)", "(*atomic:a)", "(*scs:1)a", "(*pla:a)", "(*naplb:b)", " ",
    "\t", "\n", "#c\n", "\u{e9}", "\u{100}", "(?[a&&b])", "(?[\\p{L}--[a-z]])", "[a&&b]",
];

fn gen_pattern(rng: &mut Rng) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    match rng.below(10) {
        0..=4 => {
            let n = rng.range(1, 6);
            for _ in 0..n {
                out.extend_from_slice(rng.pick(FRAGS).as_bytes());
            }
        }
        5..=6 => {
            out.extend_from_slice(rng.pick(PATTERNS).as_bytes());
            if rng.chance(2) {
                out.extend_from_slice(rng.pick(FRAGS).as_bytes());
            }
        }
        7 => out = gen_ascii(rng, 14),
        8 => out = gen_utf8(rng, 8),
        _ => out = gen_raw(rng, 10),
    }
    if rng.chance(5) && !out.is_empty() {
        let i = rng.below(out.len());
        out[i] = rng.byte();
    }
    out
}

/// Drives `cfgs` over `n` randomized patterns with a fixed seed.  Both the
/// explicit-length and the `PCRE2_ZERO_TERMINATED` call shapes are used.
unsafe fn fuzz(p: &Pair, cfgs: &[Cfg], seed: u64, n: usize, d: &mut Diffs) {
    let mut rng = Rng::new(seed);
    let mut ok = 0usize;
    for i in 0..n {
        let cfg = &cfgs[i % cfgs.len()];
        let pat = gen_pattern(&mut rng);
        let zt = rng.chance(3) && !pat.contains(&0);
        let mut buf = pat.clone();
        if zt {
            buf.push(0);
        }
        let len = if zt { PCRE2_ZERO_TERMINATED } else { pat.len() };
        if compile_ptr(p, buf.as_ptr(), len, cfg, d, i % 4 == 0).is_some() {
            ok += 1;
        }
    }
    println!("  fuzz seed={seed}: {n} randomized patterns, {ok} compiled");
}

// ============================================== rows 1-6: pattern input shapes

#[test]
fn cfg_001_006_literal_shapes() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let plain = Cfg::new("options 0, ccontext NULL", 0, 0).nullctx();

        // row 1 — options 0, xoptions 0, ccontext NULL, pattern `abc` len 3
        let r1 = c1(p, "abc", &plain, &mut d);
        k.is("row 1: /abc/ FIRSTSET", r1.first(), (1, b'a' as u32));
        k.is("row 1: /abc/ LASTSET", r1.last(), (1, b'c' as u32));
        k.is("row 1: /abc/ MINLENGTH", r1.minlen(), 3);
        k.is("row 1: /abc/ FIRSTBITMAP absent", r1.firstbitmap().is_none(), true);
        k.is("row 1: /abc/ top_bracket", r1.head().top_bracket, 0);
        // a NULL ccontext must behave exactly like a default context
        let dflt = c1(p, "abc", &Cfg::new("default ccontext", 0, 0), &mut d);
        assert_code_eq(r1.a, dflt.a, "row 1: NULL vs default ccontext (C)");
        assert_code_eq(r1.b, dflt.b, "row 1: NULL vs default ccontext (rust)");

        // row 2 — `abc\0` with PCRE2_ZERO_TERMINATED
        let zt = b"abc\0";
        let r2 = compile_ptr(p, zt.as_ptr(), PCRE2_ZERO_TERMINATED, &plain, &mut d, true).unwrap();
        k.is("row 2: ZERO_TERMINATED identical to row 1", r2.block() == r1.block(), true);
        assert_code_eq(r1.a, r2.a, "row 2 vs row 1 (C)");
        assert_code_eq(r1.b, r2.b, "row 2 vs row 1 (rust)");

        // row 3 — pattern == NULL, patlen == 0 (internal null_str)
        let r3 = compile_ptr(p, ptr::null(), 0, &plain, &mut d, true).unwrap();
        k.is("row 3: NULL/0 MATCHEMPTY", r3.u32info(PCRE2_INFO_MATCHEMPTY), 1);
        k.is("row 3: NULL/0 MINLENGTH", r3.minlen(), 0);
        k.is("row 3: NULL/0 top_bracket", r3.head().top_bracket, 0);

        // row 4 — non-NULL pointer, patlen == 0
        let r4 = compile_ptr(p, b"x".as_ptr(), 0, &plain, &mut d, true).unwrap();
        k.is("row 4: empty pattern identical to row 3", r4.block() == r3.block(), true);
        assert_code_eq(r3.a, r4.a, "row 4 vs row 3 (C)");
        assert_code_eq(r3.b, r4.b, "row 4 vs row 3 (rust)");

        // row 5 — `a\x00b` with explicit patlen 3
        let r5 = compile_ptr(p, b"a\0b".as_ptr(), 3, &plain, &mut d, true).unwrap();
        k.is(
            "row 5: a\\0b opcode sequence",
            r5.opseq(),
            vec![op::BRA, op::CHAR, op::CHAR, op::CHAR, op::KET, op::END],
        );
        let o5 = r5.ops();
        k.is("row 5: the middle OP_CHAR operand is NUL", r5.at(o5[2].0 + 1), 0);
        k.is("row 5: MINLENGTH", r5.minlen(), 3);

        // row 6 — 40-character literal run
        let long = "abcdefghij".repeat(4);
        let r6 = c1(p, &long, &plain, &mut d);
        k.is("row 6: 40 x OP_CHAR", r6.count_op(op::CHAR), 40);
        k.is("row 6: FIRSTSET 'a'", r6.first(), (1, b'a' as u32));
        k.is("row 6: LASTSET last char", r6.last(), (1, b'j' as u32));
        k.is("row 6: MINLENGTH 40", r6.minlen(), 40);

        fuzz(p, &[plain, Cfg::new("default", 0, 0)], 1001, 500, &mut d);
    }
    k.finish("1-6");
    d.finish("CONFIGS 1-6: literal / zero-length / embedded-NUL / ZERO_TERMINATED pattern shapes");
}

// ================================== rows 7-8: PCRE2_ANCHORED and the REQ_VARY gate

#[test]
fn cfg_007_008_anchored_lastset() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let anch = Cfg::new("ANCHORED", PCRE2_ANCHORED, 0);
        // row 7 — the required code unit needs REQ_VARY when anchored
        let r7 = c1(p, "abc", &anch, &mut d);
        k.is("row 7: ANCHORED /abc/ LASTCODETYPE", r7.last().0, 0);
        k.ok("row 7: ANCHORED /abc/ flags LASTSET clear", r7.flags() & F_LASTSET == 0);
        // row 8 — reqcu after a variable item does survive
        let r8 = c1(p, "a.*c", &anch, &mut d);
        k.is("row 8: ANCHORED /a.*c/ LASTSET", r8.last(), (1, b'c' as u32));
        k.ok("row 8: flags LASTSET set", r8.flags() & F_LASTSET != 0);
        fuzz(p, &[anch, Cfg::new("ANCHORED|ENDANCHORED", PCRE2_ANCHORED | PCRE2_ENDANCHORED, 0)], 1002, 500, &mut d);
    }
    k.finish("7-8");
    d.finish("CONFIGS 7-8: PCRE2_ANCHORED with and without a variable item before the required code unit");
}

// ===================== rows 9-12: auto-anchoring, dotstar anchor, STARTLINE

#[test]
fn cfg_009_012_anchor_startline() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let plain = Cfg::new("default", 0, 0);
        let dotall = Cfg::new("DOTALL", PCRE2_DOTALL, 0);
        let nodsa = Cfg::new(
            "DOTALL|NO_DOTSTAR_ANCHOR",
            PCRE2_DOTALL | PCRE2_NO_DOTSTAR_ANCHOR,
            0,
        );
        // row 9 — `^abc` without MULTILINE is auto-anchored.  The C then still
        // records the first code unit (FIRSTSET 'a'): being anchored is what
        // stops the STARTLINE gate, not the first-code-unit search.
        let r9 = c1(p, "^abc", &plain, &mut d);
        k.ok("row 9: ^abc ALLOPTIONS has ANCHORED", r9.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_ANCHORED != 0);
        k.is("row 9: ^abc FIRSTSET", r9.first(), (1, b'a' as u32));
        k.is("row 9: ^abc flags STARTLINE clear", r9.flags() & F_STARTLINE, 0);
        // row 10 — `.*abc` with DOTALL (OP_ALLANY) is dotstar-anchored
        let r10 = c1(p, ".*abc", &dotall, &mut d);
        let s10 = r10.find_op(op::TYPESTAR).expect("DOTALL .* is OP_TYPESTAR");
        k.is("row 10: DOTALL .* is OP_TYPESTAR OP_ALLANY", r10.at(s10 + 1), op::ALLANY);
        k.ok(
            "row 10: DOTALL .*abc ALLOPTIONS has ANCHORED",
            r10.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_ANCHORED != 0,
        );
        // row 11 — NO_DOTSTAR_ANCHOR clears PCRE2_OPTIM_DOTSTAR_ANCHOR
        let r11 = c1(p, ".*abc", &nodsa, &mut d);
        k.is(
            "row 11: NO_DOTSTAR_ANCHOR .*abc ANCHORED bit",
            r11.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_ANCHORED,
            0,
        );
        // row 12 — `.*abc` without DOTALL (OP_ANY) is a STARTLINE pattern
        let r12 = c1(p, ".*abc", &plain, &mut d);
        let s12 = r12.find_op(op::TYPESTAR).expect(".* is OP_TYPESTAR");
        k.is("row 12: .* is OP_TYPESTAR OP_ANY", r12.at(s12 + 1), op::ANY);
        k.is("row 12: .*abc FIRSTCODETYPE", r12.first().0, 2);
        k.is("row 12: .*abc FIRSTBITMAP absent", r12.firstbitmap().is_none(), true);
        k.ok("row 12: .*abc flags STARTLINE", r12.flags() & F_STARTLINE != 0);
        fuzz(p, &[plain, dotall, nodsa], 1003, 500, &mut d);
    }
    k.finish("9-12");
    d.finish("CONFIGS 9-12: ^-anchoring, .*-anchoring, NO_DOTSTAR_ANCHOR and STARTLINE");
}

// ============================= rows 13-15: MULTILINE and the inline (?m) form

#[test]
fn cfg_013_015_multiline() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let plain = Cfg::new("default", 0, 0);
        let ml = Cfg::new("MULTILINE", PCRE2_MULTILINE, 0);
        // row 13 — OP_CIRCM, and firstcuflags forced to REQ_NONE
        let r13 = c1(p, "^a", &ml, &mut d);
        k.is("row 13: MULTILINE ^a first opcode", r13.first_op(), op::CIRCM);
        k.is("row 13: MULTILINE ^a FIRSTCODETYPE", r13.first().0, 2);
        // row 14 — OP_DOLLM vs OP_DOLL
        let r14 = c1(p, "a$", &ml, &mut d);
        k.ok("row 14: MULTILINE a$ has OP_DOLLM", r14.has(op::DOLLM));
        let r14n = c1(p, "a$", &plain, &mut d);
        k.ok("row 14: plain a$ has OP_DOLL", r14n.has(op::DOLL));
        // row 15 — in-pattern (?m) must produce the same codegen
        let r15 = c1(p, "(?m)^a$", &plain, &mut d);
        let r15b = c1(p, "^a$", &ml, &mut d);
        k.is("row 15: (?m)^a$ bytecode == MULTILINE ^a$", r15.code(), r15b.code());
        fuzz(p, &[ml, plain], 1004, 500, &mut d);
    }
    k.finish("13-15");
    d.finish("CONFIGS 13-15: MULTILINE circumflex/dollar and the inline (?m) equivalent");
}

// ==================================== rows 16-26: the caseless matrix

#[test]
fn cfg_016_026_caseless() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let ci = Cfg::new("CASELESS", PCRE2_CASELESS, 0);
        let ci_utf = Cfg::new("CASELESS|UTF", PCRE2_CASELESS | PCRE2_UTF, 0);
        let ci_utf_r = Cfg::new(
            "CASELESS|UTF + X:CASELESS_RESTRICT",
            PCRE2_CASELESS | PCRE2_UTF,
            PCRE2_EXTRA_CASELESS_RESTRICT,
        );
        let ci_ucp = Cfg::new("CASELESS|UCP", PCRE2_CASELESS | PCRE2_UCP, 0);
        let ci_ucp_r = Cfg::new(
            "CASELESS|UCP + X:CASELESS_RESTRICT",
            PCRE2_CASELESS | PCRE2_UCP,
            PCRE2_EXTRA_CASELESS_RESTRICT,
        );
        let turk = Cfg::new(
            "UTF + X:TURKISH_CASING",
            PCRE2_UTF,
            PCRE2_EXTRA_TURKISH_CASING,
        );
        let turk_ci = Cfg::new(
            "UTF|CASELESS + X:TURKISH_CASING",
            PCRE2_UTF | PCRE2_CASELESS,
            PCRE2_EXTRA_TURKISH_CASING,
        );
        let plain = Cfg::new("default", 0, 0);

        // row 16 — OP_CHARI plus PCRE2_FIRSTCASELESS
        let r16 = c1(p, "a", &ci, &mut d);
        k.is("row 16: CASELESS /a/ first opcode", r16.first_op(), op::CHARI);
        k.ok("row 16: CASELESS /a/ flags FIRSTCASELESS", r16.flags() & F_FIRSTCASELESS != 0);

        // row 17 — multi-case set becomes OP_PROP PT_CLIST
        let r17 = c1(p, "k", &ci_utf, &mut d);
        k.is("row 17: CASELESS|UTF /k/ first opcode", r17.first_op(), op::PROP);
        k.is("row 17: CASELESS|UTF /k/ property type", r17.at(r17.ops()[1].0 + 1), PT_CLIST);

        // row 18 — CASELESS_RESTRICT collapses it back to OP_CHARI
        let r18 = c1(p, "k", &ci_utf_r, &mut d);
        k.is("row 18: + CASELESS_RESTRICT /k/ first opcode", r18.first_op(), op::CHARI);

        // row 19 — UCP without UTF, literal byte 0xFF
        let r19 = compile_both(p, b"\xff", &ci_ucp, &mut d).expect("CASELESS|UCP 0xFF compiles");
        k.is("row 19: CASELESS|UCP byte 0xFF first opcode", r19.first_op(), op::CHARI);

        // row 20 — one-character negated class
        let r20 = c1(p, "[^k]", &ci_utf, &mut d);
        k.is("row 20: CASELESS|UTF /[^k]/ first opcode", r20.first_op(), op::NOTPROP);
        k.is("row 20: CASELESS|UTF /[^k]/ property type", r20.at(r20.ops()[1].0 + 1), PT_CLIST);

        // row 21 — two-character case-partner class folds to OP_CHARI
        let r21 = c1(p, "[Aa]", &plain, &mut d);
        k.is("row 21: /[Aa]/ first opcode", r21.first_op(), op::CHARI);

        // row 22 — not a case pair, so it stays a bitmap class
        let r22 = c1(p, "[Ab]", &plain, &mut d);
        k.is("row 22: /[Ab]/ first opcode", r22.first_op(), op::CLASS);
        let m22 = r22.class_map(r22.ops()[1].0);
        k.ok("row 22: /[Ab]/ bitmap has A and b only", {
            (0..=255u32).all(|c| bit(&m22, c as u8) == (c == b'A' as u32 || c == b'b' as u32))
        });

        // row 23 — CASELESS_RESTRICT suppresses the caseset so the fold happens
        let r23 = c1(p, "[Kk]", &ci_ucp_r, &mut d);
        k.is("row 23: CASELESS|UCP+RESTRICT /[Kk]/ first opcode", r23.first_op(), op::CHARI);

        // row 24 — the Turkish dotted-I caseset
        let r24 = c1(p, "(?i)i", &turk, &mut d);
        k.is("row 24: TURKISH_CASING (?i)i first opcode", r24.first_op(), op::PROP);
        k.is("row 24: TURKISH_CASING (?i)i property type", r24.at(r24.ops()[1].0 + 1), PT_CLIST);

        // row 25 — [Ii] must NOT fold under Turkish casing
        let r25 = c1(p, "[Ii]", &turk_ci, &mut d);
        k.ok("row 25: TURKISH_CASING /[Ii]/ is not OP_CHARI", r25.first_op() != op::CHARI);

        // row 26 — the three distinct OP_REFI flag bytes
        let r26a = c1(p, "(a)\\1", &ci, &mut d);
        let off = r26a.find_op(op::REFI).expect("CASELESS backref is OP_REFI");
        k.is("row 26: CASELESS (a)\\1 REFI flags byte", r26a.at(off + 1 + IMM2_SIZE), 0);
        let r26b = c1(
            p,
            "(a)\\1",
            &Cfg::new("CASELESS + X:CASELESS_RESTRICT", PCRE2_CASELESS, PCRE2_EXTRA_CASELESS_RESTRICT),
            &mut d,
        );
        let off = r26b.find_op(op::REFI).expect("OP_REFI");
        k.is(
            "row 26: + CASELESS_RESTRICT REFI flags byte",
            r26b.at(off + 1 + IMM2_SIZE),
            REFI_CASELESS_RESTRICT,
        );
        let r26c = c1(p, "(a)\\1", &turk_ci, &mut d);
        let off = r26c.find_op(op::REFI).expect("OP_REFI");
        k.is(
            "row 26: + TURKISH_CASING REFI flags byte",
            r26c.at(off + 1 + IMM2_SIZE),
            REFI_TURKISH_CASING,
        );

        fuzz(
            p,
            &[ci, ci_utf, ci_utf_r, ci_ucp, ci_ucp_r, turk, turk_ci],
            1016,
            700,
            &mut d,
        );
    }
    k.finish("16-26");
    d.finish("CONFIGS 16-26: caseless single characters, classes, casesets, CASELESS_RESTRICT, TURKISH_CASING, OP_REFI flags");
}

// ================================= rows 27-35: named groups and the name table

#[test]
fn cfg_027_035_names() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let plain = Cfg::new("default", 0, 0);
        let dup = Cfg::new("DUPNAMES", PCRE2_DUPNAMES, 0);
        let dup_ci = Cfg::new("DUPNAMES|CASELESS", PCRE2_DUPNAMES | PCRE2_CASELESS, 0);
        let utf = Cfg::new("UTF", PCRE2_UTF, 0);

        // row 27 — OP_DNREF, two slots, NAMEENTRYSIZE = IMM2_SIZE + len + 1
        let r27 = c1(p, "(?<a>x)(?<a>y)\\k<a>", &dup, &mut d);
        k.ok("row 27: DUPNAMES has OP_DNREF", r27.has(op::DNREF));
        k.is("row 27: NAMECOUNT", r27.u32info(PCRE2_INFO_NAMECOUNT), 2);
        k.is("row 27: NAMEENTRYSIZE", r27.u32info(PCRE2_INFO_NAMEENTRYSIZE), 4);
        k.is(
            "row 27: name table (both slots)",
            r27.nametable(),
            vec![0, 1, b'a', 0, 0, 2, b'a', 0],
        );

        // row 28 — OP_DNREFI plus its flags byte
        let r28 = c1(p, "(?<a>x)(?<a>y)\\k<a>", &dup_ci, &mut d);
        let off = r28.find_op(op::DNREFI).expect("OP_DNREFI");
        k.is("row 28: DNREFI flags byte", r28.at(off + 1 + 2 * IMM2_SIZE), 0);

        // row 29 — (?J) sets JCHANGED, the API option does not
        let r29 = c1(p, "(?J)(?<a>x)(?<a>y)", &plain, &mut d);
        k.is("row 29: (?J) JCHANGED", r29.u32info(PCRE2_INFO_JCHANGED), 1);
        k.ok("row 29: (?J) flags JCHANGED", r29.flags() & F_JCHANGED != 0);
        let r29b = c1(p, "(?<a>x)(?<a>y)", &dup, &mut d);
        k.is("row 29: API DUPNAMES JCHANGED", r29b.u32info(PCRE2_INFO_JCHANGED), 0);

        // row 30 — (?| gives the same group number, so no DUPNAMES needed
        let r30 = c1(p, "(?|(?<a>x)|(?<a>y))", &plain, &mut d);
        k.ok("row 30: (?| flags DUPCAPUSED", r30.flags() & F_DUPCAPUSED != 0);
        k.is("row 30: (?| NAMECOUNT", r30.u32info(PCRE2_INFO_NAMECOUNT), 1);

        // row 31 — out-of-order insert (the memmove path)
        let r31 = c1(p, "(?<b>x)(?<a>y)", &plain, &mut d);
        k.is(
            "row 31: name table sorted",
            r31.nametable(),
            vec![0, 2, b'a', 0, 0, 1, b'b', 0],
        );

        // row 32 — one name a prefix of the other
        let r32 = c1(p, "(?<ab>x)(?<a>y)", &plain, &mut d);
        k.is("row 32: NAMECOUNT", r32.u32info(PCRE2_INFO_NAMECOUNT), 2);
        k.is("row 32: NAMEENTRYSIZE", r32.u32info(PCRE2_INFO_NAMEENTRYSIZE), 5);
        k.is(
            "row 32: name table",
            r32.nametable(),
            vec![0, 2, b'a', 0, 0, 0, 1, b'a', b'b', 0],
        );

        // row 33 — 21 named groups forces the named_groups heap realloc
        let mut big = String::new();
        for i in 1..=21 {
            big.push_str(&format!("(?<n{i:02}>a)"));
        }
        let r33 = c1(p, &big, &plain, &mut d);
        k.is("row 33: NAMECOUNT", r33.u32info(PCRE2_INFO_NAMECOUNT), 21);
        k.is("row 33: CAPTURECOUNT", r33.u32info(PCRE2_INFO_CAPTURECOUNT), 21);

        // row 34 — name lengths 1 and MAX_NAME_SIZE (128)
        let r34a = c1(p, "(?<a>x)", &plain, &mut d);
        k.is("row 34: 1-char name NAMEENTRYSIZE", r34a.u32info(PCRE2_INFO_NAMEENTRYSIZE), 4);
        let name128: String = std::iter::once('n').chain(std::iter::repeat('a').take(127)).collect();
        let r34b = c1(p, &format!("(?<{name128}>x)"), &plain, &mut d);
        k.is("row 34: 128-char name NAMEENTRYSIZE", r34b.u32info(PCRE2_INFO_NAMEENTRYSIZE), 131);

        // row 35 — a UTF group name (the ucp_L path in read_name)
        let r35 = c1(p, "(?<\u{e9}>a)\\k<\u{e9}>", &utf, &mut d);
        k.is("row 35: UTF name NAMECOUNT", r35.u32info(PCRE2_INFO_NAMECOUNT), 1);
        k.is(
            "row 35: UTF name table",
            r35.nametable(),
            vec![0, 1, 0xc3, 0xa9, 0],
        );

        fuzz(p, &[dup, dup_ci, plain, utf], 1027, 700, &mut d);
    }
    k.finish("27-35");
    d.finish("CONFIGS 27-35: named groups, DUPNAMES/(?J), the sorted name table, long and UTF names");
}

// ============================ rows 36-40: EXTENDED and EXTENDED_MORE

#[test]
fn cfg_036_040_extended() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let x_lf = Cfg::new("EXTENDED + newline LF", PCRE2_EXTENDED, 0).nl(PCRE2_NEWLINE_LF);
        // row 36 — whitespace dropped, # comment ends at the newline
        let r36 = c1(p, "a b\t# comment\nc", &x_lf, &mut d);
        k.is("row 36: EXTENDED opcode sequence", r36.opseq(), vec![op::BRA, op::CHAR, op::CHAR, op::CHAR, op::KET, op::END]);
        k.is("row 36: EXTENDED MINLENGTH", r36.minlen(), 3);

        // row 37 — four distinct IS_NEWLINE outcomes for #-comment termination
        for (nl, name, pat) in [
            (PCRE2_NEWLINE_CR, "CR", b"a# c\rb".to_vec()),
            (PCRE2_NEWLINE_CRLF, "CRLF", b"a# c\r\nb".to_vec()),
            (PCRE2_NEWLINE_ANY, "ANY", b"a# c\x0bb".to_vec()),
            (PCRE2_NEWLINE_NUL, "NUL", b"a# c\x00b".to_vec()),
        ] {
            let cfg = Cfg::new("EXTENDED + newline", PCRE2_EXTENDED, 0).nl(nl);
            let r = compile_both(p, &pat, &cfg, &mut d).expect("compiles");
            k.is(&format!("row 37: EXTENDED newline {name} MINLENGTH"), r.minlen(), 2);
            k.is(
                &format!("row 37: EXTENDED newline {name} opcodes"),
                r.opseq(),
                vec![op::BRA, op::CHAR, op::CHAR, op::KET, op::END],
            );
        }

        // row 38 — the Unicode /x whitespace arm.  The C's list is the isspace()
        // characters plus U+0085, U+200E, U+200F, U+2028 and U+2029; U+00A0 is
        // NOT pattern white space and stays a literal.
        let xu = Cfg::new("EXTENDED|UTF", PCRE2_EXTENDED | PCRE2_UTF, 0);
        for (cp, skipped) in [
            (0x85u32, true),
            (0x200e, true),
            (0x200f, true),
            (0x2028, true),
            (0x2029, true),
            (0xa0, false),
        ] {
            let mut pat = Vec::from(b"a".as_slice());
            let mut buf = [0u8; 4];
            pat.extend_from_slice(char::from_u32(cp).unwrap().encode_utf8(&mut buf).as_bytes());
            pat.extend_from_slice(b"b");
            let r = compile_both(p, &pat, &xu, &mut d).expect("compiles");
            k.is(
                &format!("row 38: EXTENDED|UTF U+{cp:04X} treated as /x whitespace"),
                r.count_op(op::CHAR) == 2,
                skipped,
            );
        }

        // row 39 — EXTENDED_MORE also drops space/HT inside a class
        let xx = Cfg::new(
            "EXTENDED|EXTENDED_MORE",
            PCRE2_EXTENDED | PCRE2_EXTENDED_MORE,
            0,
        );
        let r39 = c1(p, "[a b]", &xx, &mut d);
        k.is("row 39: EXTENDED_MORE [a b] first opcode", r39.first_op(), op::CLASS);
        let m39 = r39.class_map(r39.ops()[1].0);
        k.ok("row 39: EXTENDED_MORE [a b] bitmap = {a,b}", {
            (0..=255u32).all(|c| bit(&m39, c as u8) == (c == b'a' as u32 || c == b'b' as u32))
        });
        // `[ ^a]` becomes `[^a]`, and a one-character negated class is OP_NOT
        let r39b = c1(p, "[ ^a]", &xx, &mut d);
        k.is("row 39: EXTENDED_MORE [ ^a] first opcode", r39b.first_op(), op::NOT);
        k.is("row 39: EXTENDED_MORE [ ^a] operand", r39b.at(r39b.ops()[1].0 + 1), b'a');

        // row 40 — (?x) inside (?xx) clears EXTENDED_MORE again
        for (pat, space_in_class) in [
            ("(?xx)[a b]", false),
            ("(?x)[a b]", true),
            ("(?xx)(?x:[a b])", true),
        ] {
            let r = c1(p, pat, &Cfg::new("default", 0, 0), &mut d);
            let off = r.find_op(op::CLASS).expect("OP_CLASS present");
            let m = r.class_map(off);
            k.is(
                &format!("row 40: {pat} class contains a space"),
                bit(&m, b' '),
                space_in_class,
            );
        }

        fuzz(p, &[x_lf, xx, xu], 1036, 600, &mut d);
    }
    k.finish("36-40");
    d.finish("CONFIGS 36-40: EXTENDED whitespace/comments per newline convention, EXTENDED_MORE, (?x) scoping");
}

// ==================================== rows 41-42: UNGREEDY

#[test]
fn cfg_041_042_ungreedy() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let ug = Cfg::new("UNGREEDY", PCRE2_UNGREEDY, 0);
        let plain = Cfg::new("default", 0, 0);
        let nap = Cfg::new("NO_AUTO_POSSESS", PCRE2_NO_AUTO_POSSESS, 0);
        let ug_nap = Cfg::new(
            "UNGREEDY|NO_AUTO_POSSESS",
            PCRE2_UNGREEDY | PCRE2_NO_AUTO_POSSESS,
            0,
        );
        // row 41 — greedy/minimal swap.  With the default optimizations a
        // *greedy* quantifier at the end of the pattern is auto-possessified,
        // so the raw opcode is only observable with NO_AUTO_POSSESS.
        for (pat, want) in [
            ("a*", op::MINSTAR),
            ("a*?", op::STAR),
            ("a*+", op::POSSTAR),
            ("a{2,4}", op::MINUPTO),
            ("a{2,4}?", op::UPTO),
            ("a{2,4}+", op::POSUPTO),
        ] {
            let r = c1(p, pat, &ug_nap, &mut d);
            k.ok(&format!("row 41: UNGREEDY|NO_AUTO_POSSESS {pat} has opcode {want}"), r.has(want));
        }
        for (pat, want) in [
            ("a*", op::STAR),
            ("a*?", op::MINSTAR),
            ("a*+", op::POSSTAR),
            ("a{2,4}", op::UPTO),
            ("a{2,4}?", op::MINUPTO),
            ("a{2,4}+", op::POSUPTO),
        ] {
            let r = c1(p, pat, &nap, &mut d);
            k.ok(&format!("row 41: NO_AUTO_POSSESS {pat} has opcode {want}"), r.has(want));
        }
        // the same patterns with the default optimizations on
        for (pat, want) in [
            ("a*", op::MINSTAR),
            ("a*?", op::POSSTAR),
            ("a*+", op::POSSTAR),
            ("a{2,4}", op::MINUPTO),
            ("a{2,4}?", op::POSUPTO),
            ("a{2,4}+", op::POSUPTO),
        ] {
            let r = c1(p, pat, &ug, &mut d);
            k.ok(&format!("row 41: UNGREEDY {pat} has opcode {want}"), r.has(want));
        }
        // row 42 — inline (?U) and (?-U)
        let r42a = c1(p, "(?U)a*", &plain, &mut d);
        k.is("row 42: (?U)a* first opcode", r42a.first_op(), op::MINSTAR);
        let r42b = c1(p, "(?U)(?-U)a*", &plain, &mut d);
        k.is("row 42: (?U)(?-U)a* first opcode (auto-possessified)", r42b.first_op(), op::POSSTAR);
        let r42c = c1(p, "(?U)(?-U)a*", &nap, &mut d);
        k.is("row 42: (?U)(?-U)a* with NO_AUTO_POSSESS", r42c.first_op(), op::STAR);
        fuzz(p, &[ug, plain, nap, ug_nap], 1041, 600, &mut d);
    }
    k.finish("41-42");
    d.finish("CONFIGS 41-42: PCRE2_UNGREEDY and the inline (?U)/(?-U) forms over every quantifier");
}

// ============================== rows 43-44: NO_AUTO_CAPTURE

#[test]
fn cfg_043_044_no_auto_capture() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let nac = Cfg::new("NO_AUTO_CAPTURE", PCRE2_NO_AUTO_CAPTURE, 0);
        let plain = Cfg::new("default", 0, 0);
        // row 43 — plain ( becomes non-capturing, named groups still capture
        let r43 = c1(p, "(a)(?<n>b)", &nac, &mut d);
        k.is("row 43: NO_AUTO_CAPTURE CAPTURECOUNT", r43.u32info(PCRE2_INFO_CAPTURECOUNT), 1);
        k.is("row 43: NO_AUTO_CAPTURE top_bracket", r43.head().top_bracket, 1);
        k.ok("row 43: still has a capturing bracket", r43.has(op::CBRA));
        k.ok("row 43: also has a plain bracket", r43.has(op::BRA));
        // row 44 — inline (?n) plus a named backreference
        let r44 = c1(p, "(?n)(?<n>a)\\k<n>", &plain, &mut d);
        k.is("row 44: (?n) CAPTURECOUNT", r44.u32info(PCRE2_INFO_CAPTURECOUNT), 1);
        k.is("row 44: (?n) NAMECOUNT", r44.u32info(PCRE2_INFO_NAMECOUNT), 1);
        k.ok("row 44: (?n) has OP_REF", r44.has(op::REF));
        fuzz(p, &[nac, plain], 1043, 500, &mut d);
    }
    k.finish("43-44");
    d.finish("CONFIGS 43-44: PCRE2_NO_AUTO_CAPTURE and the inline (?n) form");
}

// ================= rows 45-49: auto-possessify and pcre2_set_optimize_8

#[test]
fn cfg_045_049_possess_optimize() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let plain = Cfg::new("default", 0, 0);
        let nap = Cfg::new("NO_AUTO_POSSESS", PCRE2_NO_AUTO_POSSESS, 0);
        // row 45 — possessified vs left alone
        for (pat, poss, plainop) in [
            ("a+b", op::POSPLUS, op::PLUS),
            ("\\d+\\D", op::TYPEPOSPLUS, op::TYPEPLUS),
            ("[a-z]+[0-9]", op::CRPOSPLUS, op::CRPLUS),
            ("\\w+\\s", op::TYPEPOSPLUS, op::TYPEPLUS),
            ("x+\\z", op::POSPLUS, op::PLUS),
        ] {
            let r = c1(p, pat, &plain, &mut d);
            k.ok(&format!("row 45: {pat} possessified to {poss}"), r.has(poss));
            let rn = c1(p, pat, &nap, &mut d);
            k.ok(&format!("row 45: NO_AUTO_POSSESS {pat} keeps {plainop}"), rn.has(plainop));
        }
        // row 46 — the verb form sets both the optimization flag and the option
        let r46 = c1(p, "(*NO_AUTO_POSSESS)a+b", &plain, &mut d);
        k.ok(
            "row 46: (*NO_AUTO_POSSESS) ALLOPTIONS bit",
            r46.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_NO_AUTO_POSSESS != 0,
        );
        k.ok("row 46: (*NO_AUTO_POSSESS) keeps OP_PLUS", r46.has(op::PLUS));

        // row 47 — PCRE2_OPTIMIZATION_NONE turns everything off
        let none = Cfg::new("OPTIMIZATION_NONE", 0, 0).optim(&[PCRE2_OPTIMIZATION_NONE]);
        let r = c1(p, "a+b", &none, &mut d);
        k.ok("row 47: NONE a+b keeps OP_PLUS", r.has(op::PLUS));
        let r = c1(p, "(?s).*x", &none, &mut d);
        k.is("row 47: NONE (?s).*x ANCHORED bit", r.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_ANCHORED, 0);
        let r = c1(p, "abc", &none, &mut d);
        k.is("row 47: NONE abc FIRSTCODETYPE", r.first().0, 0);
        k.is("row 47: NONE abc LASTCODETYPE", r.last().0, 0);
        k.is("row 47: NONE abc MINLENGTH", r.minlen(), 0);
        k.is("row 47: NONE abc FIRSTBITMAP absent", r.firstbitmap().is_none(), true);
        let r = c1(p, "[Ww]ord", &none, &mut d);
        k.is("row 47: NONE [Ww]ord FIRSTBITMAP absent", r.firstbitmap().is_none(), true);
        k.is("row 47: NONE [Ww]ord FIRSTCODETYPE", r.first().0, 0);

        // row 48 — NONE then AUTO_POSSESS turns bit 0 back on only
        let none_ap = Cfg::new("OPTIMIZATION_NONE + AUTO_POSSESS", 0, 0)
            .optim(&[PCRE2_OPTIMIZATION_NONE, PCRE2_AUTO_POSSESS]);
        let r = c1(p, "a+b", &none_ap, &mut d);
        k.ok("row 48: NONE+AUTO_POSSESS a+b is possessified", r.has(op::POSPLUS));
        let r = c1(p, "abc", &none_ap, &mut d);
        k.is("row 48: NONE+AUTO_POSSESS abc FIRSTCODETYPE", r.first().0, 0);
        let r = c1(p, "(?s).*x", &none_ap, &mut d);
        k.is(
            "row 48: NONE+AUTO_POSSESS (?s).*x ANCHORED bit",
            r.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_ANCHORED,
            0,
        );

        // row 49 — each single switch-off from the FULL default, and the restore
        let ap_off = Cfg::new("AUTO_POSSESS_OFF", 0, 0).optim(&[PCRE2_AUTO_POSSESS_OFF]);
        let r = c1(p, "a+b", &ap_off, &mut d);
        k.ok("row 49: AUTO_POSSESS_OFF a+b keeps OP_PLUS", r.has(op::PLUS));
        let r = c1(p, "abc", &ap_off, &mut d);
        k.is("row 49: AUTO_POSSESS_OFF abc FIRSTSET", r.first(), (1, b'a' as u32));
        let ds_off = Cfg::new("DOTSTAR_ANCHOR_OFF", 0, 0).optim(&[67]);
        let r = c1(p, "(?s).*x", &ds_off, &mut d);
        k.is(
            "row 49: DOTSTAR_ANCHOR_OFF (?s).*x ANCHORED bit",
            r.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_ANCHORED,
            0,
        );
        let so_off = Cfg::new("START_OPTIMIZE_OFF", 0, 0).optim(&[PCRE2_START_OPTIMIZE_OFF]);
        let r = c1(p, "abc", &so_off, &mut d);
        k.is("row 49: START_OPTIMIZE_OFF abc FIRSTCODETYPE", r.first().0, 0);
        let r = c1(p, "a+b", &so_off, &mut d);
        k.ok("row 49: START_OPTIMIZE_OFF a+b is still possessified", r.has(op::POSPLUS));
        let full = Cfg::new("NONE then FULL", 0, 0)
            .optim(&[PCRE2_OPTIMIZATION_NONE, PCRE2_OPTIMIZATION_FULL]);
        let r = c1(p, "a+b", &full, &mut d);
        k.ok("row 49: FULL restores auto-possessify", r.has(op::POSPLUS));
        let r = c1(p, "abc", &full, &mut d);
        k.is("row 49: FULL restores FIRSTSET", r.first(), (1, b'a' as u32));
        let r = c1(p, "(?s).*x", &full, &mut d);
        k.ok(
            "row 49: FULL restores the dotstar anchor",
            r.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_ANCHORED != 0,
        );

        fuzz(p, &[plain, nap, none, none_ap, ap_off, ds_off, so_off, full], 1045, 800, &mut d);
    }
    k.finish("45-49");
    d.finish("CONFIGS 45-49: auto-possessification and every pcre2_set_optimize_8 directive");
}

// ============================= rows 50-51: NO_START_OPTIMIZE

#[test]
fn cfg_050_051_no_start_optimize() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let nso = Cfg::new("NO_START_OPTIMIZE", PCRE2_NO_START_OPTIMIZE, 0);
        let plain = Cfg::new("default", 0, 0);
        // row 50
        let r = c1(p, "abc", &nso, &mut d);
        k.is("row 50: NO_START_OPTIMIZE abc FIRSTCODETYPE", r.first().0, 0);
        k.is("row 50: NO_START_OPTIMIZE abc LASTCODETYPE", r.last().0, 0);
        k.is("row 50: NO_START_OPTIMIZE abc FIRSTBITMAP absent", r.firstbitmap().is_none(), true);
        k.is("row 50: NO_START_OPTIMIZE abc MINLENGTH", r.minlen(), 0);
        let r = c1(p, "^abc", &nso, &mut d);
        k.ok(
            "row 50: NO_START_OPTIMIZE ^abc is still auto-anchored",
            r.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_ANCHORED != 0,
        );
        // row 51 — the verb form
        let r51 = c1(p, "(*NO_START_OPT)abc", &plain, &mut d);
        k.ok(
            "row 51: (*NO_START_OPT) ALLOPTIONS bit",
            r51.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_NO_START_OPTIMIZE != 0,
        );
        k.is("row 51: (*NO_START_OPT) FIRSTCODETYPE", r51.first().0, 0);
        k.is("row 51: (*NO_START_OPT) MINLENGTH", r51.minlen(), 0);
        fuzz(p, &[nso, plain], 1050, 500, &mut d);
    }
    k.finish("50-51");
    d.finish("CONFIGS 50-51: PCRE2_NO_START_OPTIMIZE and the (*NO_START_OPT) verb");
}

// ====================== rows 52-53: ALLOW_EMPTY_CLASS and the literal ]

#[test]
fn cfg_052_053_empty_class() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let aec = Cfg::new("ALLOW_EMPTY_CLASS", PCRE2_ALLOW_EMPTY_CLASS, 0);
        let plain = Cfg::new("default", 0, 0);
        // row 52
        let r = c1(p, "[]", &aec, &mut d);
        k.is("row 52: [] first opcode", r.first_op(), op::CLASS);
        k.is("row 52: [] bitmap all zero", r.class_map(r.ops()[1].0), [0u8; 32]);
        let r = c1(p, "[^]", &aec, &mut d);
        k.is("row 52: [^] first opcode", r.first_op(), op::ALLANY);
        let r = c1(p, "[]a]", &aec, &mut d);
        k.is("row 52: []a] MINLENGTH", r.minlen(), 3);
        k.is(
            "row 52: []a] opcodes",
            r.opseq(),
            vec![op::BRA, op::CLASS, op::CHAR, op::CHAR, op::KET, op::END],
        );
        // `[]]*` is the empty class followed by a quantified literal `]`, which
        // the default optimizations then auto-possessify
        let r = c1(p, "[]]*", &aec, &mut d);
        k.ok("row 52: []]* has a quantified ]", r.has(op::POSSTAR));
        k.is("row 52: []]* quantified operand", r.at(r.find_op(op::POSSTAR).unwrap() + 1), b']');
        // row 53 — without the option, ] is a literal class member
        let r53 = c1(p, "[]a]", &plain, &mut d);
        k.is("row 53: []a] first opcode", r53.first_op(), op::CLASS);
        k.is("row 53: []a] MINLENGTH", r53.minlen(), 1);
        let m = r53.class_map(r53.ops()[1].0);
        k.ok("row 53: []a] bitmap = {],a}", {
            (0..=255u32).all(|c| bit(&m, c as u8) == (c == b']' as u32 || c == b'a' as u32))
        });
        fuzz(p, &[aec, plain], 1052, 500, &mut d);
    }
    k.finish("52-53");
    d.finish("CONFIGS 52-53: PCRE2_ALLOW_EMPTY_CLASS versus the literal-] rule");
}

// ============================ rows 54-56: ALT_BSUX and surrogate escapes

#[test]
fn cfg_054_056_bsux() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let bsux = Cfg::new("ALT_BSUX", PCRE2_ALT_BSUX, 0);
        let xbsux = Cfg::new("X:ALT_BSUX", 0, PCRE2_EXTRA_ALT_BSUX);
        // row 54 — 4-hex \u, 2-hex \x, literal U
        for (pat, want) in [("\\u0041", b'A'), ("\\x41", b'A'), ("\\U", b'U')] {
            let r = c1(p, pat, &bsux, &mut d);
            k.is(&format!("row 54: ALT_BSUX {pat} char operand"), r.char_ops(), vec![want]);
        }
        // row 55 — \u{...} is EXTRA_ALT_BSUX only
        let r = c1(p, "\\u{41}", &xbsux, &mut d);
        k.is("row 55: X:ALT_BSUX \\u{41}", r.char_ops(), vec![b'A']);
        let r = c1(p, "\\u{ 12}", &xbsux, &mut d);
        k.is(
            "row 55: X:ALT_BSUX \\u{ 12} is literal",
            r.char_ops(),
            vec![b'u', b'{', b' ', b'1', b'2', b'}'],
        );
        let r = c1(p, "\\u{}", &xbsux, &mut d);
        k.is("row 55: X:ALT_BSUX \\u{} is literal", r.char_ops(), vec![b'u', b'{', b'}']);
        let r = c1(p, "[\\u{}]", &xbsux, &mut d);
        k.is("row 55: X:ALT_BSUX [\\u{}] first opcode", r.first_op(), op::CLASS);
        let m = r.class_map(r.ops()[1].0);
        k.ok("row 55: [\\u{}] bitmap = {u,{,}}", {
            (0..=255u32).all(|c| {
                bit(&m, c as u8) == (c == b'u' as u32 || c == b'{' as u32 || c == b'}' as u32)
            })
        });
        // row 56 — surrogate escapes
        let surr = Cfg::new(
            "UTF|ALT_BSUX + X:ALLOW_SURROGATE_ESCAPES",
            PCRE2_UTF | PCRE2_ALT_BSUX,
            PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES,
        );
        let a = c1(p, "\\ud800", &surr, &mut d);
        let b = c1(p, "\\o{155000}", &surr, &mut d);
        let c = c1(p, "\\x{d800}", &surr, &mut d);
        k.is("row 56: \\ud800 encodes U+D800", a.code()[3..6].to_vec(), vec![0xed, 0xa0, 0x80]);
        k.is("row 56: \\o{155000} == \\ud800", b.code(), a.code());
        k.is("row 56: \\x{d800} == \\ud800", c.code(), a.code());
        fuzz(p, &[bsux, xbsux, surr], 1054, 600, &mut d);
    }
    k.finish("54-56");
    d.finish("CONFIGS 54-56: ALT_BSUX, EXTRA_ALT_BSUX \\u{...}, ALLOW_SURROGATE_ESCAPES");
}

// ================================= rows 57-63: callouts

#[test]
fn cfg_057_063_callouts() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let ac = Cfg::new("AUTO_CALLOUT", PCRE2_AUTO_CALLOUT, 0);
        let plain = Cfg::new("default", 0, 0);
        // row 57 — a callout before every item plus a trailing one
        let r57 = c1(p, "abc", &ac, &mut d);
        k.is(
            "row 57: AUTO_CALLOUT abc opcodes",
            r57.opseq(),
            vec![
                op::BRA,
                op::CALLOUT,
                op::CHAR,
                op::CALLOUT,
                op::CHAR,
                op::CALLOUT,
                op::CHAR,
                op::CALLOUT,
                op::KET,
                op::END,
            ],
        );
        k.is("row 57: AUTO_CALLOUT abc callout numbers", r57.callout_numbers(), vec![255, 255, 255, 255]);
        // row 58 — an explicit callout abolishes the preceding auto callout
        let r58 = c1(p, "a(?C1)b", &ac, &mut d);
        k.is("row 58: a(?C1)b callout numbers", r58.callout_numbers(), vec![255, 1, 255]);
        // row 59 — the LITERAL fast path still manages callouts
        let r59 = c1(
            p,
            "a.b",
            &Cfg::new("AUTO_CALLOUT|LITERAL", PCRE2_AUTO_CALLOUT | PCRE2_LITERAL, 0),
            &mut d,
        );
        k.is("row 59: LITERAL a.b callout numbers", r59.callout_numbers(), vec![255, 255, 255, 255]);
        k.is("row 59: LITERAL a.b chars", r59.char_ops(), vec![b'a', b'.', b'b']);
        // row 60 — a callout between OP_COND and the condition assertion
        let r60 = c1(p, "(?(?=a)b|c)", &ac, &mut d);
        k.is("row 60: opcode after OP_COND", r60.op_after(op::COND), Some(op::CALLOUT));
        k.ok("row 60: the assertion is still there", r60.has(op::ASSERT));
        // row 61 — EXTRA_NEVER_CALLOUT does not suppress auto callouts
        let r61 = c1(
            p,
            "abc",
            &Cfg::new(
                "AUTO_CALLOUT + X:NEVER_CALLOUT",
                PCRE2_AUTO_CALLOUT,
                PCRE2_EXTRA_NEVER_CALLOUT,
            ),
            &mut d,
        );
        k.is("row 61: X:NEVER_CALLOUT still emits auto callouts", r61.callout_numbers().len(), 4);
        k.is("row 61: identical to plain AUTO_CALLOUT", r61.code(), r57.code());
        // row 62 — every callout string delimiter
        for delim in ["`", "'", "\"", "^", "%", "#", "$", "{"] {
            let close = if delim == "{" { "}" } else { delim };
            let pat = format!("a(?C{delim}x{close})b");
            let r = c1(p, &pat, &plain, &mut d);
            let off = r.find_op(op::CALLOUT_STR).expect("OP_CALLOUT_STR");
            k.is(
                &format!("row 62: {pat} stores the opening delimiter"),
                r.at(off + 1 + 4 * LINK_SIZE),
                delim.as_bytes()[0],
            );
            k.is(
                &format!("row 62: {pat} stores the string"),
                r.code()[off + 2 + 4 * LINK_SIZE..off + 4 + 4 * LINK_SIZE].to_vec(),
                vec![b'x', 0],
            );
        }
        let r = c1(p, "a(?C\"a\"\"b\")b", &plain, &mut d);
        let off = r.find_op(op::CALLOUT_STR).expect("OP_CALLOUT_STR");
        k.is(
            "row 62: doubled delimiter (?C\"a\"\"b\")",
            r.code()[off + 1 + 4 * LINK_SIZE..off + 6 + 4 * LINK_SIZE].to_vec(),
            vec![b'"', b'a', b'"', b'b', 0],
        );
        // row 63 — numeric callouts
        for (pat, num) in [("a(?C)b", 0u8), ("a(?C0)b", 0), ("a(?C1)b", 1), ("a(?C255)b", 255)] {
            let r = c1(p, pat, &plain, &mut d);
            k.is(&format!("row 63: {pat} callout number"), r.callout_numbers(), vec![num]);
        }
        fuzz(p, &[ac, plain], 1057, 600, &mut d);
    }
    k.finish("57-63");
    d.finish("CONFIGS 57-63: AUTO_CALLOUT, explicit numeric callouts and every string-callout delimiter");
}

// ================================= rows 64-68: PCRE2_LITERAL

#[test]
fn cfg_064_068_literal() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        let lit = Cfg::new("LITERAL", PCRE2_LITERAL, 0);
        // row 64 — every metacharacter is a literal
        let r64 = c1(p, "a.b*c[", &lit, &mut d);
        k.is("row 64: LITERAL a.b*c[ chars", r64.char_ops(), vec![b'a', b'.', b'b', b'*', b'c', b'[']);
        k.is("row 64: LITERAL MINLENGTH", r64.minlen(), 6);
        // row 65 — EXTRA_MATCH_WORD wraps the literal in \b(?:...)\b
        let r65 = c1(
            p,
            "a.b",
            &Cfg::new(
                "LITERAL|CASELESS + X:MATCH_WORD",
                PCRE2_LITERAL | PCRE2_CASELESS,
                PCRE2_EXTRA_MATCH_WORD,
            ),
            &mut d,
        );
        k.is("row 65: MATCH_WORD first opcode", r65.first_op(), op::WORD_BOUNDARY);
        k.is("row 65: MATCH_WORD two \\b", r65.count_op(op::WORD_BOUNDARY), 2);
        k.is("row 65: MATCH_WORD wraps a group", r65.count_op(op::BRA), 2);
        k.is("row 65: MATCH_WORD caseless chars", r65.char_ops(), vec![b'a', b'.', b'b']);
        k.is("row 65: MATCH_WORD uses OP_CHARI", r65.count_op(op::CHARI), 3);
        // row 66 — EXTRA_MATCH_LINE wraps it in ^(?:...)$
        let r66 = c1(
            p,
            "a.b",
            &Cfg::new(
                "LITERAL|MULTILINE + X:MATCH_LINE",
                PCRE2_LITERAL | PCRE2_MULTILINE,
                PCRE2_EXTRA_MATCH_LINE,
            ),
            &mut d,
        );
        k.is("row 66: MATCH_LINE first opcode", r66.first_op(), op::CIRCM);
        k.ok("row 66: MATCH_LINE has OP_DOLLM", r66.has(op::DOLLM));
        k.is("row 66: MATCH_LINE chars", r66.char_ops(), vec![b'a', b'.', b'b']);
        // row 67 — the pso_list scan is skipped for LITERAL
        let r67 = c1(p, "(*UTF)x", &lit, &mut d);
        k.is(
            "row 67: LITERAL (*UTF)x chars",
            r67.char_ops(),
            vec![b'(', b'*', b'U', b'T', b'F', b')', b'x'],
        );
        k.is("row 67: LITERAL (*UTF)x MINLENGTH", r67.minlen(), 7);
        k.is(
            "row 67: LITERAL (*UTF)x has no UTF option",
            r67.u32info(PCRE2_INFO_ALLOPTIONS) & PCRE2_UTF,
            0,
        );
        // row 68 — LITERAL with an embedded NUL and a legal LITERAL extra bit
        let r68 = compile_both(
            p,
            b"a\0b",
            &Cfg::new(
                "LITERAL + X:CASELESS_RESTRICT",
                PCRE2_LITERAL,
                PCRE2_EXTRA_CASELESS_RESTRICT,
            ),
            &mut d,
        )
        .expect("compiles");
        k.is("row 68: LITERAL a\\0b chars", r68.char_ops(), vec![b'a', 0, b'b']);
        k.is("row 68: LITERAL a\\0b MINLENGTH", r68.minlen(), 3);
        fuzz(p, &[lit, Cfg::new("LITERAL|CASELESS", PCRE2_LITERAL | PCRE2_CASELESS, 0)], 1064, 600, &mut d);
    }
    k.finish("64-68");
    d.finish("CONFIGS 64-68: PCRE2_LITERAL, MATCH_WORD/MATCH_LINE wrappers, verbs and NULs as literals");
}

// ================== rows 69-73: escape-related EXTRA options

#[test]
fn cfg_069_073_escape_extras() {
    let p = pair();
    let mut d = Diffs::new();
    let mut k = Claims::new();
    unsafe {
        // row 69 — EXTRA_ESCAPED_CR_IS_LF
        let cr_lf = Cfg::new("X:ESCAPED_CR_IS_LF", 0, PCRE2_EXTRA_ESCAPED_CR_IS_LF);
        let r = c1(p, "a\\rb", &cr_lf, &mut d);
        k.is("row 69: \\r becomes LF", r.char_ops(), vec![b'a', 0x0a, b'b']);
        k.is("row 69: HASCRORLF", r.u32info(PCRE2_INFO_HASCRORLF), 1);
        let r = c1(p, "a\\x0db", &cr_lf, &mut d);
        k.is("row 69: \\x0d is unaffected", r.char_ops(), vec![b'a', 0x0d, b'b']);
        k.is("row 69: \\x0d HASCRORLF", r.u32info(PCRE2_INFO_HASCRORLF), 1);
        // row 70 — EXTRA_BAD_ESCAPE_IS_LITERAL
        let bad = Cfg::new("X:BAD_ESCAPE_IS_LITERAL", 0, PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL);
        for (pat, want) in [
            ("\\q", vec![b'q']),
            ("\\y", vec![b'y']),
            ("\\F", vec![b'F']),
            ("\\L", vec![b'L']),
            ("\\x{", vec![b'x', b'{']),
        ] {
            let r = c1(p, pat, &bad, &mut d);
            k.is(&format!("row 70: BAD_ESCAPE_IS_LITERAL {pat}"), r.char_ops(), want);
        }
        let r = c1(p, "[\\q]", &bad, &mut d);
        k.is("row 70: [\\q] first opcode", r.first_op(), op::CHAR);
        k.is("row 70: [\\q] operand", r.char_ops(), vec![b'q']);
        let r = c1(
            p,
            "\\C",
            &Cfg::new(
                "NEVER_BACKSLASH_C + X:BAD_ESCAPE_IS_LITERAL",
                PCRE2_NEVER_BACKSLASH_C,
                PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
            ),
            &mut d,
        );
        k.is("row 70: \\C rescued to a literal C", r.char_ops(), vec![b'C']);
        let r = c1(
            p,
            "\\0",
            &Cfg::new(
                "X:NO_BS0 + X:BAD_ESCAPE_IS_LITERAL",
                0,
                PCRE2_EXTRA_NO_BS0 | PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL,
            ),
            &mut d,
        );
        k.is("row 70: \\0 rescued to a literal 0", r.char_ops(), vec![b'0']);
        // row 71 — EXTRA_PYTHON_OCTAL
        let py = Cfg::new("X:PYTHON_OCTAL", 0, PCRE2_EXTRA_PYTHON_OCTAL);
        let r = c1(p, "(a)(b)\\123", &py, &mut d);
        k.is("row 71: PYTHON_OCTAL \\123 is octal", r.char_ops(), vec![b'a', b'b', 0o123]);
        let r = c1(p, "(a)(b)\\1", &py, &mut d);
        k.ok("row 71: PYTHON_OCTAL \\1 is a backreference", r.has(op::REF));
        let r = c1(p, "(a)(b)\\377", &py, &mut d);
        k.is("row 71: PYTHON_OCTAL \\377", r.char_ops(), vec![b'a', b'b', 0xff]);
        // row 72 — the default (Perl) disambiguation
        let plain = Cfg::new("default", 0, 0);
        let r = c1(p, "(a)(b)\\12", &plain, &mut d);
        k.is("row 72: Perl \\12 is octal", r.char_ops(), vec![b'a', b'b', 0o12]);
        let r = c1(p, "(a)(b)\\1", &plain, &mut d);
        k.ok("row 72: Perl \\1 is a backreference", r.has(op::REF));
        let r = c1(p, "(a)(b)\\377", &plain, &mut d);
        k.is("row 72: Perl \\377", r.char_ops(), vec![b'a', b'b', 0xff]);
        // row 73 — EXTRA_NO_BS0 rejects only the bare \0
        let nobs0 = Cfg::new("X:NO_BS0", 0, PCRE2_EXTRA_NO_BS0);
        for pat in ["\\00", "\\000", "\\x00", "\\o{0}"] {
            let r = c1(p, pat, &nobs0, &mut d);
            k.is(&format!("row 73: NO_BS0 {pat} is still legal"), r.char_ops(), vec![0]);
        }
        fuzz(p, &[cr_lf, bad, py, nobs0, plain], 1069, 700, &mut d);
    }
    k.finish("69-73");
    d.finish("CONFIGS 69-73: ESCAPED_CR_IS_LF, BAD_ESCAPE_IS_LITERAL, PYTHON_OCTAL, NO_BS0 and the Perl octal rule");
}
