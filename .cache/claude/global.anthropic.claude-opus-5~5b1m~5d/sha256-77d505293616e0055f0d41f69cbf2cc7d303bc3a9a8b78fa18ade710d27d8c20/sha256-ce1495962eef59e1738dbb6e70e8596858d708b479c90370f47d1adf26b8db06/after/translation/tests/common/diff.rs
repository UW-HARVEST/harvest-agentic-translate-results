//! Shared differential drivers: compile a pattern in BOTH libraries and compare
//! every observable — errorcode, erroroffset, all `pattern_info` fields, the
//! serialized bytecode byte-for-byte — then match and compare rc / ovector /
//! mark / startchar.
//!
//! Available to all test binaries as `common::diff`.
#![allow(dead_code)]

use super::*;
use std::ffi::c_void;

// ---------------------------------------------------------------- constants
pub const PCRE2_ANCHORED: u32 = 0x8000_0000;
pub const PCRE2_NO_UTF_CHECK: u32 = 0x4000_0000;
pub const PCRE2_ENDANCHORED: u32 = 0x2000_0000;

pub const PCRE2_ALLOW_EMPTY_CLASS: u32 = 0x0000_0001;
pub const PCRE2_ALT_BSUX: u32 = 0x0000_0002;
pub const PCRE2_AUTO_CALLOUT: u32 = 0x0000_0004;
pub const PCRE2_CASELESS: u32 = 0x0000_0008;
pub const PCRE2_DOLLAR_ENDONLY: u32 = 0x0000_0010;
pub const PCRE2_DOTALL: u32 = 0x0000_0020;
pub const PCRE2_DUPNAMES: u32 = 0x0000_0040;
pub const PCRE2_EXTENDED: u32 = 0x0000_0080;
pub const PCRE2_FIRSTLINE: u32 = 0x0000_0100;
pub const PCRE2_MATCH_UNSET_BACKREF: u32 = 0x0000_0200;
pub const PCRE2_MULTILINE: u32 = 0x0000_0400;
pub const PCRE2_NEVER_UCP: u32 = 0x0000_0800;
pub const PCRE2_NEVER_UTF: u32 = 0x0000_1000;
pub const PCRE2_NO_AUTO_CAPTURE: u32 = 0x0000_2000;
pub const PCRE2_NO_AUTO_POSSESS: u32 = 0x0000_4000;
pub const PCRE2_NO_DOTSTAR_ANCHOR: u32 = 0x0000_8000;
pub const PCRE2_NO_START_OPTIMIZE: u32 = 0x0001_0000;
pub const PCRE2_UCP: u32 = 0x0002_0000;
pub const PCRE2_UNGREEDY: u32 = 0x0004_0000;
pub const PCRE2_UTF: u32 = 0x0008_0000;
pub const PCRE2_NEVER_BACKSLASH_C: u32 = 0x0010_0000;
pub const PCRE2_ALT_CIRCUMFLEX: u32 = 0x0020_0000;
pub const PCRE2_ALT_VERBNAMES: u32 = 0x0040_0000;
pub const PCRE2_USE_OFFSET_LIMIT: u32 = 0x0080_0000;
pub const PCRE2_EXTENDED_MORE: u32 = 0x0100_0000;
pub const PCRE2_LITERAL: u32 = 0x0200_0000;
pub const PCRE2_MATCH_INVALID_UTF: u32 = 0x0400_0000;
pub const PCRE2_ALT_EXTENDED_CLASS: u32 = 0x0800_0000;

pub const PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES: u32 = 0x0000_0001;
pub const PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL: u32 = 0x0000_0002;
pub const PCRE2_EXTRA_MATCH_WORD: u32 = 0x0000_0004;
pub const PCRE2_EXTRA_MATCH_LINE: u32 = 0x0000_0008;
pub const PCRE2_EXTRA_ESCAPED_CR_IS_LF: u32 = 0x0000_0010;
pub const PCRE2_EXTRA_ALT_BSUX: u32 = 0x0000_0020;
pub const PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK: u32 = 0x0000_0040;
pub const PCRE2_EXTRA_CASELESS_RESTRICT: u32 = 0x0000_0080;
pub const PCRE2_EXTRA_ASCII_BSD: u32 = 0x0000_0100;
pub const PCRE2_EXTRA_ASCII_BSS: u32 = 0x0000_0200;
pub const PCRE2_EXTRA_ASCII_BSW: u32 = 0x0000_0400;
pub const PCRE2_EXTRA_ASCII_POSIX: u32 = 0x0000_0800;
pub const PCRE2_EXTRA_ASCII_DIGIT: u32 = 0x0000_1000;
pub const PCRE2_EXTRA_PYTHON_OCTAL: u32 = 0x0000_2000;
pub const PCRE2_EXTRA_NO_BS0: u32 = 0x0000_4000;
pub const PCRE2_EXTRA_NEVER_CALLOUT: u32 = 0x0000_8000;
pub const PCRE2_EXTRA_TURKISH_CASING: u32 = 0x0001_0000;

pub const PCRE2_NOTBOL: u32 = 0x0000_0001;
pub const PCRE2_NOTEOL: u32 = 0x0000_0002;
pub const PCRE2_NOTEMPTY: u32 = 0x0000_0004;
pub const PCRE2_NOTEMPTY_ATSTART: u32 = 0x0000_0008;
pub const PCRE2_PARTIAL_SOFT: u32 = 0x0000_0010;
pub const PCRE2_PARTIAL_HARD: u32 = 0x0000_0020;
pub const PCRE2_DFA_RESTART: u32 = 0x0000_0040;
pub const PCRE2_DFA_SHORTEST: u32 = 0x0000_0080;
pub const PCRE2_SUBSTITUTE_GLOBAL: u32 = 0x0000_0100;
pub const PCRE2_SUBSTITUTE_EXTENDED: u32 = 0x0000_0200;
pub const PCRE2_SUBSTITUTE_UNSET_EMPTY: u32 = 0x0000_0400;
pub const PCRE2_SUBSTITUTE_UNKNOWN_UNSET: u32 = 0x0000_0800;
pub const PCRE2_SUBSTITUTE_OVERFLOW_LENGTH: u32 = 0x0000_1000;
pub const PCRE2_NO_JIT: u32 = 0x0000_2000;
pub const PCRE2_COPY_MATCHED_SUBJECT: u32 = 0x0000_4000;
pub const PCRE2_SUBSTITUTE_LITERAL: u32 = 0x0000_8000;
pub const PCRE2_SUBSTITUTE_MATCHED: u32 = 0x0001_0000;
pub const PCRE2_SUBSTITUTE_REPLACEMENT_ONLY: u32 = 0x0002_0000;
pub const PCRE2_DISABLE_RECURSELOOP_CHECK: u32 = 0x0004_0000;

pub const PCRE2_CONVERT_UTF: u32 = 0x0000_0001;
pub const PCRE2_CONVERT_NO_UTF_CHECK: u32 = 0x0000_0002;
pub const PCRE2_CONVERT_POSIX_BASIC: u32 = 0x0000_0004;
pub const PCRE2_CONVERT_POSIX_EXTENDED: u32 = 0x0000_0008;
pub const PCRE2_CONVERT_GLOB: u32 = 0x0000_0010;
pub const PCRE2_CONVERT_GLOB_NO_WILD_SEPARATOR: u32 = 0x0000_0030;
pub const PCRE2_CONVERT_GLOB_NO_STARSTAR: u32 = 0x0000_0050;

pub const PCRE2_ZERO_TERMINATED: usize = usize::MAX;
pub const PCRE2_UNSET: usize = usize::MAX;

// newline conventions
pub const NL_CR: u32 = 1;
pub const NL_LF: u32 = 2;
pub const NL_CRLF: u32 = 3;
pub const NL_ANY: u32 = 4;
pub const NL_ANYCRLF: u32 = 5;
pub const NL_NUL: u32 = 6;
pub const ALL_NEWLINES: [u32; 6] = [NL_CR, NL_LF, NL_CRLF, NL_ANY, NL_ANYCRLF, NL_NUL];

// bsr conventions
pub const BSR_UNICODE: u32 = 1;
pub const BSR_ANYCRLF: u32 = 2;

// error codes used in assertions
pub const ERR_NOMATCH: i32 = -1;
pub const ERR_PARTIAL: i32 = -2;
pub const ERR_NULL: i32 = -51;
pub const ERR_BADDATA: i32 = -29;
pub const ERR_BADMAGIC: i32 = -31;
pub const ERR_BADOPTION: i32 = -34;
pub const ERR_BADOFFSET: i32 = -33;
pub const ERR_NOMEMORY: i32 = -48;
pub const ERR_NOSUBSTRING: i32 = -49;
pub const ERR_NOUNIQUESUBSTRING: i32 = -50;
pub const ERR_UNSET: i32 = -55;
pub const ERR_BADOFFSETLIMIT: i32 = -56;
pub const ERR_JIT_BADOPTION: i32 = -45;
pub const ERR_UNAVAILABLE: i32 = -54;

/// All `pattern_info` selectors that return a scalar, with the size of the
/// value they write. `NAMETABLE` (19) and `FIRSTBITMAP` (7) return pointers and
/// are handled separately.
pub const INFO_U32: [u32; 21] = [
    0, 1, 2, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13, 14, 15, 17, 18, 20, 21, 23, 26,
];
pub const INFO_SIZE_T: [u32; 3] = [16, 22, 24]; // MINLENGTH is u32 actually
pub const INFO_HEAPLIMIT: u32 = 25;

// ------------------------------------------------------- compile-side config
/// A full compile-time configuration: the option words plus every compile
/// context setting the C code branches on.
#[derive(Clone, Debug, Default)]
pub struct CompileCfg {
    pub options: u32,
    pub extra_options: u32,
    pub newline: Option<u32>,
    pub bsr: Option<u32>,
    pub max_pattern_length: Option<usize>,
    pub max_pattern_compiled_length: Option<usize>,
    pub max_varlookbehind: Option<u32>,
    pub parens_nest_limit: Option<u32>,
    pub optimize: Option<u32>,
    /// use `pcre2_maketables()` output instead of the built-in default tables
    pub own_tables: bool,
}

impl CompileCfg {
    pub fn new(options: u32) -> Self {
        CompileCfg { options, ..Default::default() }
    }
    pub fn extra(mut self, e: u32) -> Self {
        self.extra_options = e;
        self
    }
    pub fn newline(mut self, n: u32) -> Self {
        self.newline = Some(n);
        self
    }
    pub fn bsr(mut self, b: u32) -> Self {
        self.bsr = Some(b);
        self
    }
    pub fn varlookbehind(mut self, n: u32) -> Self {
        self.max_varlookbehind = Some(n);
        self
    }
    pub fn parens_nest(mut self, n: u32) -> Self {
        self.parens_nest_limit = Some(n);
        self
    }
    pub fn max_len(mut self, n: usize) -> Self {
        self.max_pattern_length = Some(n);
        self
    }
    pub fn max_compiled(mut self, n: usize) -> Self {
        self.max_pattern_compiled_length = Some(n);
        self
    }
    pub fn optimize(mut self, n: u32) -> Self {
        self.optimize = Some(n);
        self
    }
    pub fn own_tables(mut self) -> Self {
        self.own_tables = true;
        self
    }
    pub fn needs_context(&self) -> bool {
        self.extra_options != 0
            || self.newline.is_some()
            || self.bsr.is_some()
            || self.max_pattern_length.is_some()
            || self.max_pattern_compiled_length.is_some()
            || self.max_varlookbehind.is_some()
            || self.parens_nest_limit.is_some()
            || self.optimize.is_some()
            || self.own_tables
    }
}

/// Build the compile context for `cfg` in `api`'s library (NULL if not needed).
/// Returns `(ccontext, tables_to_free)`.
pub unsafe fn make_ccontext(
    api: &Api,
    cfg: &CompileCfg,
) -> (*mut c_void, *const u8) {
    if !cfg.needs_context() {
        return (std::ptr::null_mut(), std::ptr::null());
    }
    let cx = (api.compile_context_create)(std::ptr::null_mut());
    assert!(!cx.is_null(), "{}: compile_context_create failed", api.name);
    let mut tables = std::ptr::null();
    if cfg.extra_options != 0 {
        (api.set_compile_extra_options)(cx, cfg.extra_options);
    }
    if let Some(n) = cfg.newline {
        (api.set_newline)(cx, n);
    }
    if let Some(b) = cfg.bsr {
        (api.set_bsr)(cx, b);
    }
    if let Some(n) = cfg.max_pattern_length {
        (api.set_max_pattern_length)(cx, n);
    }
    if let Some(n) = cfg.max_pattern_compiled_length {
        (api.set_max_pattern_compiled_length)(cx, n);
    }
    if let Some(n) = cfg.max_varlookbehind {
        (api.set_max_varlookbehind)(cx, n);
    }
    if let Some(n) = cfg.parens_nest_limit {
        (api.set_parens_nest_limit)(cx, n);
    }
    if let Some(n) = cfg.optimize {
        (api.set_optimize)(cx, n);
    }
    if cfg.own_tables {
        tables = (api.maketables)(std::ptr::null_mut());
        assert!(!tables.is_null());
        (api.set_character_tables)(cx, tables);
    }
    (cx, tables)
}

/// Result of compiling in one library.
pub struct Compiled {
    pub api: &'static Api,
    pub code: *mut c_void,
    pub errorcode: i32,
    pub erroroffset: usize,
    ccontext: *mut c_void,
    tables: *const u8,
}

impl Drop for Compiled {
    fn drop(&mut self) {
        unsafe {
            if !self.code.is_null() {
                (self.api.code_free)(self.code);
            }
            if !self.ccontext.is_null() {
                (self.api.compile_context_free)(self.ccontext);
            }
            if !self.tables.is_null() {
                (self.api.maketables_free)(std::ptr::null_mut(), self.tables);
            }
        }
    }
}

pub unsafe fn compile_in(
    api: &'static Api,
    pattern: &[u8],
    patlen: usize,
    cfg: &CompileCfg,
) -> Compiled {
    let (cx, tables) = make_ccontext(api, cfg);
    let mut ec = 0i32;
    let mut eo = usize::MAX;
    let code = (api.compile)(
        pattern.as_ptr(),
        patlen,
        cfg.options,
        &mut ec,
        &mut eo,
        cx,
    );
    Compiled { api, code, errorcode: ec, erroroffset: eo, ccontext: cx, tables }
}

/// Serialize one compiled code and return its bytes — this exposes the whole
/// compiled bytecode block, so comparing it catches ANY codegen divergence.
pub unsafe fn serialized_bytes(api: &Api, code: *mut c_void) -> Option<Vec<u8>> {
    let codes = [code as *const c_void];
    let mut buf: *mut u8 = std::ptr::null_mut();
    let mut len = 0usize;
    let rc = (api.serialize_encode)(
        codes.as_ptr(),
        1,
        &mut buf,
        &mut len,
        std::ptr::null_mut(),
    );
    if rc != 1 {
        return None;
    }
    let v = std::slice::from_raw_parts(buf, len).to_vec();
    (api.serialize_free)(buf);
    Some(v)
}

/// Compare every `pattern_info` field between two compiled codes.
pub unsafe fn assert_pattern_info_eq(
    cc: *mut c_void,
    rc_: *mut c_void,
    label: &str,
) {
    let (c, r) = both();
    for what in INFO_U32 {
        let mut cv: u32 = 0xDEAD_BEEF;
        let mut rv: u32 = 0xDEAD_BEEF;
        let crc = (c.pattern_info)(cc, what, &mut cv as *mut _ as *mut c_void);
        let rrc = (r.pattern_info)(rc_, what, &mut rv as *mut _ as *mut c_void);
        assert_eq!(crc, rrc, "{}: pattern_info({}) rc", label, what);
        if crc == 0 {
            assert_eq!(cv, rv, "{}: pattern_info({}) value", label, what);
        }
    }
    for what in [16u32, 22, 24, 25] {
        // MINLENGTH(16)=u32, SIZE(22)=PCRE2_SIZE, FRAMESIZE(24)=PCRE2_SIZE,
        // HEAPLIMIT(25)=u32 — use a usize buffer, which is large enough for all
        let mut cv: usize = 0xDEAD_BEEF;
        let mut rv: usize = 0xDEAD_BEEF;
        let crc = (c.pattern_info)(cc, what, &mut cv as *mut _ as *mut c_void);
        let rrc = (r.pattern_info)(rc_, what, &mut rv as *mut _ as *mut c_void);
        assert_eq!(crc, rrc, "{}: pattern_info({}) rc", label, what);
        if crc == 0 {
            assert_eq!(cv, rv, "{}: pattern_info({}) value", label, what);
        }
    }
    // FIRSTBITMAP (7) returns a pointer to a 32-byte bitmap or NULL
    {
        let mut cp: *const u8 = std::ptr::null();
        let mut rp: *const u8 = std::ptr::null();
        let crc = (c.pattern_info)(cc, 7, &mut cp as *mut _ as *mut c_void);
        let rrc = (r.pattern_info)(rc_, 7, &mut rp as *mut _ as *mut c_void);
        assert_eq!(crc, rrc, "{}: pattern_info(FIRSTBITMAP) rc", label);
        assert_eq!(
            cp.is_null(),
            rp.is_null(),
            "{}: FIRSTBITMAP nullness",
            label
        );
        if !cp.is_null() {
            let cs = std::slice::from_raw_parts(cp, 32);
            let rs = std::slice::from_raw_parts(rp, 32);
            assert_eq!(cs, rs, "{}: FIRSTBITMAP bytes", label);
        }
    }
    // NAMETABLE (19) + NAMECOUNT (17) + NAMEENTRYSIZE (18)
    {
        let mut cnt: u32 = 0;
        let mut esz: u32 = 0;
        let mut cp: *const u8 = std::ptr::null();
        let mut rp: *const u8 = std::ptr::null();
        (c.pattern_info)(cc, 17, &mut cnt as *mut _ as *mut c_void);
        (c.pattern_info)(cc, 18, &mut esz as *mut _ as *mut c_void);
        let crc = (c.pattern_info)(cc, 19, &mut cp as *mut _ as *mut c_void);
        let rrc = (r.pattern_info)(rc_, 19, &mut rp as *mut _ as *mut c_void);
        assert_eq!(crc, rrc, "{}: pattern_info(NAMETABLE) rc", label);
        if cnt > 0 {
            let n = (cnt * esz) as usize;
            let cs = std::slice::from_raw_parts(cp, n);
            let rs = std::slice::from_raw_parts(rp, n);
            assert_eq!(cs, rs, "{}: NAMETABLE bytes", label);
        }
    }
}

/// Compile `pattern` in both libraries and assert full agreement.
/// Returns `(c_compiled, r_compiled)` so the caller can go on to match.
pub unsafe fn compile_both(
    pattern: &[u8],
    patlen: usize,
    cfg: &CompileCfg,
    label: &str,
) -> (Compiled, Compiled) {
    let (c, r) = both();
    let cc = compile_in(c, pattern, patlen, cfg);
    let rr = compile_in(r, pattern, patlen, cfg);

    assert_eq!(
        cc.code.is_null(),
        rr.code.is_null(),
        "{}: compile success differs (C ec={} eo={}, Rust ec={} eo={}) pattern={:?} cfg={:?}",
        label,
        cc.errorcode,
        cc.erroroffset,
        rr.errorcode,
        rr.erroroffset,
        String::from_utf8_lossy(pattern),
        cfg
    );

    if cc.code.is_null() {
        assert_eq!(
            cc.errorcode, rr.errorcode,
            "{}: errorcode differs for pattern={:?} cfg={:?}",
            label,
            String::from_utf8_lossy(pattern),
            cfg
        );
        assert_eq!(
            cc.erroroffset, rr.erroroffset,
            "{}: erroroffset differs (ec={}) for pattern={:?} cfg={:?}",
            label,
            cc.errorcode,
            String::from_utf8_lossy(pattern),
            cfg
        );
        // the human-readable message must match too
        let mut cb = [0u8; 256];
        let mut rb = [0u8; 256];
        let cn = (c.get_error_message)(cc.errorcode, cb.as_mut_ptr(), 256);
        let rn = (r.get_error_message)(rr.errorcode, rb.as_mut_ptr(), 256);
        assert_eq!(cn, rn, "{}: error message length", label);
        assert_eq!(cb, rb, "{}: error message text", label);
        return (cc, rr);
    }

    assert_pattern_info_eq(cc.code, rr.code, label);

    // Strongest check: the entire compiled bytecode block, byte for byte.
    let cb = serialized_bytes(c, cc.code);
    let rb = serialized_bytes(r, rr.code);
    match (&cb, &rb) {
        (Some(cv), Some(rv)) => {
            assert_eq!(
                cv.len(),
                rv.len(),
                "{}: serialized length differs for pattern={:?} cfg={:?}",
                label,
                String::from_utf8_lossy(pattern),
                cfg
            );
            if cv != rv {
                let i = cv.iter().zip(rv.iter()).position(|(a, b)| a != b).unwrap();
                panic!(
                    "{}: serialized bytecode differs at byte {} (C={:#04x} Rust={:#04x})\n\
                     pattern={:?} cfg={:?}\n C[{}..]={:02x?}\n R[{}..]={:02x?}",
                    label,
                    i,
                    cv[i],
                    rv[i],
                    String::from_utf8_lossy(pattern),
                    cfg,
                    i,
                    &cv[i..(i + 24).min(cv.len())],
                    i,
                    &rv[i..(i + 24).min(rv.len())],
                );
            }
        }
        (None, None) => {}
        _ => panic!("{}: serialize_encode succeeded in only one library", label),
    }

    (cc, rr)
}

// --------------------------------------------------------- match-side config
#[derive(Clone, Debug, Default)]
pub struct MatchCfg {
    pub options: u32,
    pub match_limit: Option<u32>,
    pub depth_limit: Option<u32>,
    pub heap_limit: Option<u32>,
    pub offset_limit: Option<usize>,
    /// ovector pair count for `match_data_create`; `None` = from pattern
    pub ovecsize: Option<u32>,
}

impl MatchCfg {
    pub fn new(options: u32) -> Self {
        MatchCfg { options, ..Default::default() }
    }
    pub fn ovec(mut self, n: u32) -> Self {
        self.ovecsize = Some(n);
        self
    }
    pub fn match_limit(mut self, n: u32) -> Self {
        self.match_limit = Some(n);
        self
    }
    pub fn depth_limit(mut self, n: u32) -> Self {
        self.depth_limit = Some(n);
        self
    }
    pub fn heap_limit(mut self, n: u32) -> Self {
        self.heap_limit = Some(n);
        self
    }
    pub fn offset_limit(mut self, n: usize) -> Self {
        self.offset_limit = Some(n);
        self
    }
    pub fn needs_context(&self) -> bool {
        self.match_limit.is_some()
            || self.depth_limit.is_some()
            || self.heap_limit.is_some()
            || self.offset_limit.is_some()
    }
}

pub unsafe fn make_mcontext(api: &Api, cfg: &MatchCfg) -> *mut c_void {
    if !cfg.needs_context() {
        return std::ptr::null_mut();
    }
    let cx = (api.match_context_create)(std::ptr::null_mut());
    assert!(!cx.is_null());
    if let Some(n) = cfg.match_limit {
        (api.set_match_limit)(cx, n);
    }
    if let Some(n) = cfg.depth_limit {
        (api.set_depth_limit)(cx, n);
    }
    if let Some(n) = cfg.heap_limit {
        (api.set_heap_limit)(cx, n);
    }
    if let Some(n) = cfg.offset_limit {
        (api.set_offset_limit)(cx, n);
    }
    cx
}

/// Everything observable after a match.
#[derive(Debug, PartialEq, Eq)]
pub struct MatchOut {
    pub rc: i32,
    pub ovector: Vec<usize>,
    pub ovec_count: u32,
    pub startchar: usize,
    pub mark: Option<Vec<u8>>,
}

/// Which matcher to drive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    Interpreter,
    Dfa,
    /// `pcre2_jit_match` — with JIT unsupported this must fail identically
    JitMatch,
}

pub unsafe fn run_match(
    api: &Api,
    code: *mut c_void,
    subject: &[u8],
    subjlen: usize,
    startoffset: usize,
    cfg: &MatchCfg,
    engine: Engine,
) -> MatchOut {
    let mcx = make_mcontext(api, cfg);
    let md = match cfg.ovecsize {
        Some(n) => (api.match_data_create)(n, std::ptr::null_mut()),
        None => (api.match_data_create_from_pattern)(code, std::ptr::null_mut()),
    };
    assert!(!md.is_null(), "{}: match_data_create failed", api.name);

    let rc = match engine {
        Engine::Interpreter => (api.do_match)(
            code,
            subject.as_ptr(),
            subjlen,
            startoffset,
            cfg.options,
            md,
            mcx,
        ),
        Engine::JitMatch => (api.jit_match)(
            code,
            subject.as_ptr(),
            subjlen,
            startoffset,
            cfg.options,
            md,
            mcx,
        ),
        Engine::Dfa => {
            let mut wspace = [0i32; 1000];
            (api.dfa_match)(
                code,
                subject.as_ptr(),
                subjlen,
                startoffset,
                cfg.options,
                md,
                mcx,
                wspace.as_mut_ptr(),
                wspace.len(),
            )
        }
    };

    let count = (api.get_ovector_count)(md);
    let ovp = (api.get_ovector_pointer)(md);
    // Only entries that PCRE2 actually DEFINES may be compared; the rest are
    // uninitialised heap and differ run-to-run even between two runs of the
    // same library.
    //   rc > 0  -> pairs 0..rc are set (unused capture slots are set to UNSET)
    //   rc == 0 -> the ovector was too small, so every pair was filled
    //   rc == PCRE2_ERROR_PARTIAL -> pair 0 holds the partial match
    //   any other rc -> nothing is defined
    let defined_pairs: usize = if rc > 0 {
        (rc as usize).min(count as usize)
    } else if rc == 0 {
        count as usize
    } else if rc == ERR_PARTIAL {
        1.min(count as usize)
    } else {
        0
    };
    let ovector = if ovp.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(ovp, defined_pairs * 2).to_vec()
    };
    // startchar is only meaningful after a match or a partial match.
    let startchar = if rc >= 0 || rc == ERR_PARTIAL {
        (api.get_startchar)(md)
    } else {
        0
    };
    let markp = (api.get_mark)(md);
    let mark = if markp.is_null() {
        None
    } else {
        // mark is a zero-terminated string in the subject/pattern
        let mut v = Vec::new();
        let mut p = markp;
        while *p != 0 {
            v.push(*p);
            p = p.add(1);
        }
        Some(v)
    };

    (api.match_data_free)(md);
    if !mcx.is_null() {
        (api.match_context_free)(mcx);
    }
    MatchOut { rc, ovector, ovec_count: count, startchar, mark }
}

/// Match in both libraries and assert full agreement.
pub unsafe fn assert_match_eq(
    cc: &Compiled,
    rr: &Compiled,
    subject: &[u8],
    subjlen: usize,
    startoffset: usize,
    cfg: &MatchCfg,
    engine: Engine,
    label: &str,
) {
    let co = run_match(cc.api, cc.code, subject, subjlen, startoffset, cfg, engine);
    let ro = run_match(rr.api, rr.code, subject, subjlen, startoffset, cfg, engine);
    assert_eq!(
        co.rc, ro.rc,
        "{}: {:?} rc differs (C={} Rust={}) subject={:?} start={} cfg={:?}",
        label, engine, co.rc, ro.rc, String::from_utf8_lossy(subject), startoffset, cfg
    );
    assert_eq!(
        co.ovec_count, ro.ovec_count,
        "{}: {:?} ovector count differs",
        label, engine
    );
    assert_eq!(
        co.ovector, ro.ovector,
        "{}: {:?} ovector differs (rc={}) subject={:?} start={} cfg={:?}",
        label, engine, co.rc, String::from_utf8_lossy(subject), startoffset, cfg
    );
    assert_eq!(
        co.startchar, ro.startchar,
        "{}: {:?} startchar differs subject={:?}",
        label, engine, String::from_utf8_lossy(subject)
    );
    assert_eq!(
        co.mark, ro.mark,
        "{}: {:?} mark differs subject={:?}",
        label, engine, String::from_utf8_lossy(subject)
    );
}

/// The whole pipeline: compile in both, then match every subject with both
/// engines and assert agreement throughout.
pub unsafe fn diff_compile_and_match(
    pattern: &[u8],
    cfg: &CompileCfg,
    subjects: &[&[u8]],
    mcfg: &MatchCfg,
    engines: &[Engine],
    label: &str,
) {
    let (cc, rr) = compile_both(pattern, pattern.len(), cfg, label);
    if cc.code.is_null() {
        return;
    }
    for subj in subjects {
        for &engine in engines {
            for &start in &[0usize, subj.len() / 2, subj.len()] {
                if start > subj.len() {
                    continue;
                }
                assert_match_eq(
                    &cc, &rr, subj, subj.len(), start, mcfg, engine, label,
                );
            }
        }
    }
}
