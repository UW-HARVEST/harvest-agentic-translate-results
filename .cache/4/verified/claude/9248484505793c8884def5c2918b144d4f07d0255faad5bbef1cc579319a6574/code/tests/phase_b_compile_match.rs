// Phase B — the compile pipeline and both matchers, driven end to end the way
// a real consumer does: build a compile context, set the options, compile, then
// run the full match with a match context.
//
// The compiled bytecode is compared BYTE FOR BYTE, which is a far stronger
// check than comparing match results alone: it transitively validates every
// internal that only runs during compilation (the parser, `_pcre2_study_8`,
// `_pcre2_auto_possessify_8`, `_pcre2_check_escape_8`, the class compilers, the
// capture-group/name-table helpers), none of which can be called directly
// without fabricating an internal `compile_block`.

mod common;
use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

/// One compile configuration: option bits plus compile-context state.
#[derive(Clone, Copy, Debug)]
pub struct Cfg {
    pub name: &'static str,
    pub opts: u32,
    pub xopts: u32,
    pub newline: u32,
    pub bsr: u32,
    pub varlookbehind: u32,
    pub parens_limit: u32,
    pub optimize: u32,
    pub own_tables: bool,
}

impl Cfg {
    const fn new(name: &'static str, opts: u32, xopts: u32) -> Cfg {
        Cfg {
            name,
            opts,
            xopts,
            newline: 0,
            bsr: 0,
            varlookbehind: 0,
            parens_limit: 0,
            optimize: u32::MAX,
            own_tables: false,
        }
    }
}

/// The configuration axes the C actually branches on, crossed.
fn configs() -> Vec<Cfg> {
    let mut v = vec![
        Cfg::new("default", 0, 0),
        Cfg::new("CASELESS", PCRE2_CASELESS, 0),
        Cfg::new("MULTILINE", PCRE2_MULTILINE, 0),
        Cfg::new("DOTALL", PCRE2_DOTALL, 0),
        Cfg::new("EXTENDED", PCRE2_EXTENDED, 0),
        Cfg::new("EXTENDED_MORE", PCRE2_EXTENDED | PCRE2_EXTENDED_MORE, 0),
        Cfg::new("UNGREEDY", PCRE2_UNGREEDY, 0),
        Cfg::new("DUPNAMES", PCRE2_DUPNAMES, 0),
        Cfg::new("NO_AUTO_CAPTURE", PCRE2_NO_AUTO_CAPTURE, 0),
        Cfg::new("NO_AUTO_POSSESS", PCRE2_NO_AUTO_POSSESS, 0),
        Cfg::new("NO_START_OPTIMIZE", PCRE2_NO_START_OPTIMIZE, 0),
        Cfg::new("NO_DOTSTAR_ANCHOR", PCRE2_NO_DOTSTAR_ANCHOR, 0),
        Cfg::new("ANCHORED", PCRE2_ANCHORED, 0),
        Cfg::new("ENDANCHORED", PCRE2_ENDANCHORED, 0),
        Cfg::new("ANCHORED|ENDANCHORED", PCRE2_ANCHORED | PCRE2_ENDANCHORED, 0),
        Cfg::new("DOLLAR_ENDONLY|MULTILINE", PCRE2_DOLLAR_ENDONLY | PCRE2_MULTILINE, 0),
        Cfg::new("FIRSTLINE", PCRE2_FIRSTLINE, 0),
        Cfg::new("ALT_CIRCUMFLEX|MULTILINE", PCRE2_ALT_CIRCUMFLEX | PCRE2_MULTILINE, 0),
        Cfg::new("ALT_BSUX", PCRE2_ALT_BSUX, 0),
        Cfg::new("ALT_VERBNAMES", PCRE2_ALT_VERBNAMES, 0),
        Cfg::new("ALLOW_EMPTY_CLASS", PCRE2_ALLOW_EMPTY_CLASS, 0),
        Cfg::new("MATCH_UNSET_BACKREF", PCRE2_MATCH_UNSET_BACKREF, 0),
        Cfg::new("LITERAL", PCRE2_LITERAL, 0),
        Cfg::new("LITERAL|CASELESS", PCRE2_LITERAL | PCRE2_CASELESS, 0),
        Cfg::new("AUTO_CALLOUT", PCRE2_AUTO_CALLOUT, 0),
        Cfg::new("ALT_EXTENDED_CLASS", PCRE2_ALT_EXTENDED_CLASS, 0),
        // --- UTF / UCP axis, the biggest behaviour switch in the library
        Cfg::new("UTF", PCRE2_UTF, 0),
        Cfg::new("UCP", PCRE2_UCP, 0),
        Cfg::new("UTF|UCP", PCRE2_UTF | PCRE2_UCP, 0),
        Cfg::new("UTF|CASELESS", PCRE2_UTF | PCRE2_CASELESS, 0),
        Cfg::new("UTF|UCP|CASELESS", PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS, 0),
        Cfg::new("UTF|MATCH_INVALID_UTF", PCRE2_UTF | PCRE2_MATCH_INVALID_UTF, 0),
        Cfg::new("UTF|UCP|DOTALL|MULTILINE", PCRE2_UTF | PCRE2_UCP | PCRE2_DOTALL | PCRE2_MULTILINE, 0),
        // --- EXTRA options
        Cfg::new("X:CASELESS_RESTRICT", PCRE2_CASELESS, PCRE2_EXTRA_CASELESS_RESTRICT),
        Cfg::new(
            "UTF|UCP|CASELESS + X:CASELESS_RESTRICT",
            PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS,
            PCRE2_EXTRA_CASELESS_RESTRICT,
        ),
        Cfg::new("X:TURKISH_CASING", PCRE2_UTF | PCRE2_CASELESS, PCRE2_EXTRA_TURKISH_CASING),
        Cfg::new("X:ASCII_BSD", PCRE2_UCP, PCRE2_EXTRA_ASCII_BSD),
        Cfg::new("X:ASCII_BSS", PCRE2_UCP, PCRE2_EXTRA_ASCII_BSS),
        Cfg::new("X:ASCII_BSW", PCRE2_UCP, PCRE2_EXTRA_ASCII_BSW),
        Cfg::new("X:ASCII_POSIX", PCRE2_UCP, PCRE2_EXTRA_ASCII_POSIX),
        Cfg::new("X:ASCII_DIGIT", PCRE2_UCP, PCRE2_EXTRA_ASCII_DIGIT),
        Cfg::new(
            "X:ASCII_all|UCP|UTF",
            PCRE2_UCP | PCRE2_UTF,
            PCRE2_EXTRA_ASCII_BSD | PCRE2_EXTRA_ASCII_BSS | PCRE2_EXTRA_ASCII_BSW
                | PCRE2_EXTRA_ASCII_POSIX | PCRE2_EXTRA_ASCII_DIGIT,
        ),
        Cfg::new("X:BAD_ESCAPE_IS_LITERAL", 0, PCRE2_EXTRA_BAD_ESCAPE_IS_LITERAL),
        Cfg::new("X:MATCH_WORD", 0, PCRE2_EXTRA_MATCH_WORD),
        Cfg::new("X:MATCH_LINE", 0, PCRE2_EXTRA_MATCH_LINE),
        Cfg::new("X:MATCH_LINE|MULTILINE", PCRE2_MULTILINE, PCRE2_EXTRA_MATCH_LINE),
        Cfg::new("X:ESCAPED_CR_IS_LF", 0, PCRE2_EXTRA_ESCAPED_CR_IS_LF),
        Cfg::new("X:ALT_BSUX", 0, PCRE2_EXTRA_ALT_BSUX | PCRE2_ALT_BSUX * 0),
        Cfg::new("ALT_BSUX + X:ALT_BSUX", PCRE2_ALT_BSUX, PCRE2_EXTRA_ALT_BSUX),
        Cfg::new("X:ALLOW_LOOKAROUND_BSK", 0, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK),
        Cfg::new("X:ALLOW_SURROGATE_ESCAPES|UTF", PCRE2_UTF, PCRE2_EXTRA_ALLOW_SURROGATE_ESCAPES),
        Cfg::new("X:PYTHON_OCTAL", 0, PCRE2_EXTRA_PYTHON_OCTAL),
        Cfg::new("X:NO_BS0", 0, PCRE2_EXTRA_NO_BS0),
        Cfg::new("X:NEVER_CALLOUT", 0, PCRE2_EXTRA_NEVER_CALLOUT),
    ];
    // --- newline convention axis (affects ., $, \R, \N and the CRLF scan)
    for nl in [
        PCRE2_NEWLINE_CR,
        PCRE2_NEWLINE_LF,
        PCRE2_NEWLINE_CRLF,
        PCRE2_NEWLINE_ANY,
        PCRE2_NEWLINE_ANYCRLF,
        PCRE2_NEWLINE_NUL,
    ] {
        let mut c = Cfg::new("MULTILINE + newline", PCRE2_MULTILINE, 0);
        c.newline = nl;
        v.push(c);
        let mut c = Cfg::new("UTF|MULTILINE + newline", PCRE2_UTF | PCRE2_MULTILINE, 0);
        c.newline = nl;
        v.push(c);
    }
    // --- \R convention axis
    for bsr in [PCRE2_BSR_UNICODE, PCRE2_BSR_ANYCRLF] {
        let mut c = Cfg::new("bsr", 0, 0);
        c.bsr = bsr;
        v.push(c);
        let mut c = Cfg::new("UTF + bsr", PCRE2_UTF, 0);
        c.bsr = bsr;
        v.push(c);
    }
    // --- optimization directives (auto-possessify / start optimize on+off)
    for opt in [
        PCRE2_OPTIMIZATION_NONE,
        PCRE2_OPTIMIZATION_FULL,
        PCRE2_AUTO_POSSESS_OFF,
        PCRE2_START_OPTIMIZE_OFF,
    ] {
        let mut c = Cfg::new("optimize", 0, 0);
        c.optimize = opt;
        v.push(c);
        let mut c = Cfg::new("UTF|UCP + optimize", PCRE2_UTF | PCRE2_UCP, 0);
        c.optimize = opt;
        v.push(c);
    }
    // --- variable lookbehind limit
    for vl in [1u32, 2, 255] {
        let mut c = Cfg::new("max_varlookbehind", 0, 0);
        c.varlookbehind = vl;
        v.push(c);
    }
    // --- parens nest limit (drives PARENTHESES_NEST_TOO_DEEP)
    for pl in [1u32, 3, 250] {
        let mut c = Cfg::new("parens_nest_limit", 0, 0);
        c.parens_limit = pl;
        v.push(c);
    }
    // --- locale tables built by pcre2_maketables (vs the built-in defaults)
    let mut c = Cfg::new("own tables (pcre2_maketables)", 0, 0);
    c.own_tables = true;
    v.push(c);
    let mut c = Cfg::new("own tables + CASELESS", PCRE2_CASELESS, 0);
    c.own_tables = true;
    v.push(c);
    v
}

/// `pcre2_compile` stores a BORROWED pointer to the character tables in the
/// compiled pattern (`re->tables = tables`) — that is exactly why
/// `pcre2_code_copy_with_tables` exists. So the locale tables must outlive every
/// `pcre2_code` compiled against them; they are built once and kept for the
/// lifetime of the process. (`pcre2_maketables_free` is covered separately.)
fn locale_tables(api: &Api) -> *const u8 {
    use std::sync::OnceLock;
    static C_T: OnceLock<usize> = OnceLock::new();
    static R_T: OnceLock<usize> = OnceLock::new();
    let cell = if api.name == "C" { &C_T } else { &R_T };
    *cell.get_or_init(|| {
        let t = unsafe { (api.maketables)(ptr::null_mut()) };
        assert!(!t.is_null(), "[{}] pcre2_maketables_8 failed", api.name);
        t as usize
    }) as *const u8
}

/// A compile context configured per `Cfg`.
struct Ctx {
    ccontext: Ptr,
}

unsafe fn make_ctx(api: &Api, cfg: &Cfg) -> Ctx {
    let cc = (api.compile_context_create)(ptr::null_mut());
    assert!(!cc.is_null(), "[{}] compile_context_create failed", api.name);
    if cfg.newline != 0 {
        assert_eq!((api.set_newline)(cc, cfg.newline), 0);
    }
    if cfg.bsr != 0 {
        assert_eq!((api.set_bsr)(cc, cfg.bsr), 0);
    }
    if cfg.varlookbehind != 0 {
        assert_eq!((api.set_max_varlookbehind)(cc, cfg.varlookbehind), 0);
    }
    if cfg.parens_limit != 0 {
        assert_eq!((api.set_parens_nest_limit)(cc, cfg.parens_limit), 0);
    }
    if cfg.xopts != 0 {
        assert_eq!((api.set_compile_extra_options)(cc, cfg.xopts), 0);
    }
    if cfg.optimize != u32::MAX {
        assert_eq!((api.set_optimize)(cc, cfg.optimize), 0);
    }
    if cfg.own_tables {
        assert_eq!((api.set_character_tables)(cc, locale_tables(api)), 0);
    }
    Ctx { ccontext: cc }
}

unsafe fn free_ctx(api: &Api, c: Ctx) {
    (api.compile_context_free)(c.ccontext);
}

/// Compile `pat` under `cfg` in both libraries.  Asserts the two agree on
/// success/failure, on the error code and error offset, and (on success) that
/// the produced bytecode is byte-identical.  Returns the two code objects.
unsafe fn compile_both(p: &Pair, pat: &[u8], len: Sz, cfg: &Cfg, d: &mut Diffs) -> Option<(Ptr, Ptr)> {
    let cc = make_ctx(&p.c, cfg);
    let rc = make_ctx(&p.r, cfg);
    let (mut eca, mut ecb) = (0 as c_int, 0 as c_int);
    let (mut eoa, mut eob) = (usize::MAX, usize::MAX);
    let a = (p.c.compile)(pat.as_ptr(), len, cfg.opts, &mut eca, &mut eoa, cc.ccontext);
    let b = (p.r.compile)(pat.as_ptr(), len, cfg.opts, &mut ecb, &mut eob, rc.ccontext);
    free_ctx(&p.c, cc);
    free_ctx(&p.r, rc);

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

// ============================================== compile: bytecode + info

// CONFIGS rows: every Cfg x every pattern -> identical bytecode and identical
// pattern_info for all 27 info ids.
#[test]
fn compile_bytecode_and_info_identical() {
    let p = pair();
    let cfgs = configs();
    let mut d = Diffs::new();
    for cfg in &cfgs {
        for pat in PATTERNS {
            let raw = pat.as_bytes();
            // both explicit length and PCRE2_ZERO_TERMINATED; the latter needs
            // a genuinely NUL-terminated buffer to read from
            let mut zt = raw.to_vec();
            zt.push(0);
            for &len in &[raw.len(), PCRE2_ZERO_TERMINATED] {
                if len == PCRE2_ZERO_TERMINATED && raw.contains(&0) {
                    continue;
                }
                let b: &[u8] = if len == PCRE2_ZERO_TERMINATED { &zt } else { raw };
                unsafe {
                    if let Some((a, bb)) = compile_both(p, b, len, cfg, &mut d) {
                        compare_all_info(p, a, bb, &format!("{} cfg[{}]", show(b), cfg.name), &mut d);
                        (p.c.code_free)(a);
                        (p.r.code_free)(bb);
                    }
                }
            }
        }
    }
    println!("compile configs={} patterns={}", cfgs.len(), PATTERNS.len());
    d.finish("compile: all Cfg x all PATTERNS x {explicit len, ZERO_TERMINATED}");
}

/// `pcre2_pattern_info` for every info id, using the right result width.
unsafe fn compare_all_info(p: &Pair, a: Ptr, b: Ptr, tag: &str, d: &mut Diffs) {
    // uint32_t results
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
    // PCRE2_SIZE results
    for what in [PCRE2_INFO_SIZE, PCRE2_INFO_FRAMESIZE, PCRE2_INFO_JITSIZE] {
        let (mut va, mut vb) = (usize::MAX, usize::MAX);
        let ra = (p.c.pattern_info)(a, what, &mut va as *mut usize as Ptr);
        let rb = (p.r.pattern_info)(b, what, &mut vb as *mut usize as Ptr);
        d.eq(&format!("info[{what}] rc {tag}"), ra, rb);
        d.eq(&format!("info[{what}] val {tag}"), va, vb);
    }
    // FIRSTBITMAP: pointer to a 32-byte table (or NULL)
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
    // NAMETABLE: name_count entries of name_entry_size bytes
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
            // and the by-name lookups over that table
            for i in 0..na {
                let ent = ta.add((i * sa) as usize);
                let name = ent.add(2); // 2-byte group number, then NUL-term name
                d.eq(
                    &format!("substring_number_from_name #{i} {tag}"),
                    (p.c.substring_number_from_name)(a, name),
                    (p.r.substring_number_from_name)(b, name),
                );
                let (mut f1, mut l1) = (ptr::null(), ptr::null());
                let (mut f2, mut l2) = (ptr::null(), ptr::null());
                let ra = (p.c.substring_nametable_scan)(a, name, &mut f1, &mut l1);
                let rb = (p.r.substring_nametable_scan)(b, name, &mut f2, &mut l2);
                d.eq(&format!("nametable_scan rc #{i} {tag}"), ra, rb);
                if ra >= 0 && rb >= 0 {
                    d.eq(
                        &format!("nametable_scan span #{i} {tag}"),
                        (f1 as usize - ta as usize, l1 as usize - ta as usize),
                        (f2 as usize - tb as usize, l2 as usize - tb as usize),
                    );
                }
            }
        }
    }
}

// ============================================================ code_copy

// CONFIGS row: pcre2_code_copy / pcre2_code_copy_with_tables must reproduce a
// pattern that is byte-identical to the original and to the other library's.
#[test]
fn code_copy_identical() {
    let p = pair();
    let cfgs = configs();
    let mut d = Diffs::new();
    for cfg in cfgs.iter().take(34) {
        for pat in PATTERNS.iter().step_by(3) {
            let b = pat.as_bytes();
            unsafe {
                if let Some((a, bb)) = compile_both(p, b, b.len(), cfg, &mut d) {
                    let tag = format!("{} cfg[{}]", show(b), cfg.name);
                    let ca = (p.c.code_copy)(a);
                    let cb = (p.r.code_copy)(bb);
                    assert!(!ca.is_null() && !cb.is_null(), "code_copy failed {tag}");
                    assert_code_eq(ca, cb, &format!("code_copy {tag}"));
                    assert_code_eq(a, ca, &format!("code_copy vs original (C) {tag}"));
                    let ta = (p.c.code_copy_with_tables)(a);
                    let tb = (p.r.code_copy_with_tables)(bb);
                    assert!(!ta.is_null() && !tb.is_null(), "code_copy_with_tables failed {tag}");
                    assert_code_eq(ta, tb, &format!("code_copy_with_tables {tag}"));
                    d.checked += 2;
                    (p.c.code_free)(ca);
                    (p.r.code_free)(cb);
                    (p.c.code_free)(ta);
                    (p.r.code_free)(tb);
                    (p.c.code_free)(a);
                    (p.r.code_free)(bb);
                }
            }
        }
    }
    d.finish("pcre2_code_copy_8 / pcre2_code_copy_with_tables_8 over Cfg x patterns");
}

// ================================================================= matching

/// Runs `pcre2_match` in both libraries and compares rc, ovector, startchar,
/// mark, and the derived match-data accessors.
unsafe fn cmp_match(
    p: &Pair,
    a: Ptr,
    b: Ptr,
    subj: &[u8],
    len: Sz,
    start: Sz,
    mopts: u32,
    ovecsize: u32,
    mctx: (Ptr, Ptr),
    tag: &str,
    d: &mut Diffs,
) {
    let mda = (p.c.match_data_create)(ovecsize, ptr::null_mut());
    let mdb = (p.r.match_data_create)(ovecsize, ptr::null_mut());
    let ra = (p.c.do_match)(a, subj.as_ptr(), len, start, mopts, mda, mctx.0);
    let rb = (p.r.do_match)(b, subj.as_ptr(), len, start, mopts, mdb, mctx.1);
    let oa = read_match_out(&p.c, mda, ra);
    let ob = read_match_out(&p.r, mdb, rb);
    d.eq(tag, oa, ob);
    d.eq(
        &format!("{tag} :: ovector_count"),
        (p.c.get_ovector_count)(mda),
        (p.r.get_ovector_count)(mdb),
    );
    d.eq(
        &format!("{tag} :: match_data_size"),
        (p.c.get_match_data_size)(mda),
        (p.r.get_match_data_size)(mdb),
    );
    d.eq(
        &format!("{tag} :: heapframes_size"),
        (p.c.get_match_data_heapframes_size)(mda),
        (p.r.get_match_data_heapframes_size)(mdb),
    );
    // substring accessors over the result
    if ra > 0 && rb > 0 {
        for i in 0..(ra.max(rb) as u32 + 1) {
            let (mut la, mut lb) = (usize::MAX, usize::MAX);
            d.eq(
                &format!("{tag} :: substring_length_bynumber({i}) rc"),
                (p.c.substring_length_bynumber)(mda, i, &mut la),
                (p.r.substring_length_bynumber)(mdb, i, &mut lb),
            );
            d.eq(&format!("{tag} :: substring_length_bynumber({i}) len"), la, lb);
            let mut ba = [0u8; 256];
            let mut bb = [0u8; 256];
            let (mut ca, mut cb) = (ba.len(), bb.len());
            d.eq(
                &format!("{tag} :: substring_copy_bynumber({i}) rc"),
                (p.c.substring_copy_bynumber)(mda, i, ba.as_mut_ptr(), &mut ca),
                (p.r.substring_copy_bynumber)(mdb, i, bb.as_mut_ptr(), &mut cb),
            );
            d.eq(&format!("{tag} :: substring_copy_bynumber({i}) out"), (ba, ca), (bb, cb));
            let (mut pa, mut pb) = (ptr::null_mut::<u8>(), ptr::null_mut::<u8>());
            let (mut na, mut nb) = (usize::MAX, usize::MAX);
            let ga = (p.c.substring_get_bynumber)(mda, i, &mut pa, &mut na);
            let gb = (p.r.substring_get_bynumber)(mdb, i, &mut pb, &mut nb);
            d.eq(&format!("{tag} :: substring_get_bynumber({i}) rc"), ga, gb);
            d.eq(&format!("{tag} :: substring_get_bynumber({i}) len"), na, nb);
            if ga == 0 && gb == 0 {
                d.eq(
                    &format!("{tag} :: substring_get_bynumber({i}) bytes"),
                    std::slice::from_raw_parts(pa, na + 1).to_vec(),
                    std::slice::from_raw_parts(pb, nb + 1).to_vec(),
                );
            }
            if !pa.is_null() {
                (p.c.substring_free)(pa);
            }
            if !pb.is_null() {
                (p.r.substring_free)(pb);
            }
        }
        // whole-list variant
        let (mut la, mut lb) = (ptr::null_mut(), ptr::null_mut());
        let (mut sa, mut sb) = (ptr::null_mut(), ptr::null_mut());
        let ra2 = (p.c.substring_list_get)(mda, &mut la, &mut sa);
        let rb2 = (p.r.substring_list_get)(mdb, &mut lb, &mut sb);
        d.eq(&format!("{tag} :: substring_list_get rc"), ra2, rb2);
        if ra2 == 0 && rb2 == 0 {
            // The C uses `count = match_data->rc`, or `oveccount` when rc == 0
            // (ovector too small), and stores exactly `count` entries plus a
            // NULL terminator.
            let n = if ra > 0 {
                ra as usize
            } else {
                (p.c.get_ovector_count)(mda) as usize
            };
            for i in 0..n {
                let (x, y) = (*la.add(i), *lb.add(i));
                let (nx, ny) = (*sa.add(i), *sb.add(i));
                d.eq(&format!("{tag} :: list[{i}] len"), nx, ny);
                if nx == ny {
                    d.eq(
                        &format!("{tag} :: list[{i}] bytes"),
                        std::slice::from_raw_parts(x, nx).to_vec(),
                        std::slice::from_raw_parts(y, ny).to_vec(),
                    );
                }
            }
            (p.c.substring_list_free)(la);
            (p.r.substring_list_free)(lb);
        }
    }
    (p.c.match_data_free)(mda);
    (p.r.match_data_free)(mdb);
}

/// Match options crossed over the axes `pcre2_match` branches on.
const MOPTS: &[(u32, &str)] = &[
    (0, "none"),
    (PCRE2_NOTBOL, "NOTBOL"),
    (PCRE2_NOTEOL, "NOTEOL"),
    (PCRE2_NOTBOL | PCRE2_NOTEOL, "NOTBOL|NOTEOL"),
    (PCRE2_NOTEMPTY, "NOTEMPTY"),
    (PCRE2_NOTEMPTY_ATSTART, "NOTEMPTY_ATSTART"),
    (PCRE2_ANCHORED, "ANCHORED"),
    (PCRE2_ENDANCHORED, "ENDANCHORED"),
    (PCRE2_ANCHORED | PCRE2_ENDANCHORED, "ANCHORED|ENDANCHORED"),
    (PCRE2_PARTIAL_SOFT, "PARTIAL_SOFT"),
    (PCRE2_PARTIAL_HARD, "PARTIAL_HARD"),
    (PCRE2_NO_START_OPTIMIZE, "NO_START_OPTIMIZE"),
    (PCRE2_NO_UTF_CHECK, "NO_UTF_CHECK"),
    (PCRE2_COPY_MATCHED_SUBJECT, "COPY_MATCHED_SUBJECT"),
    (PCRE2_DISABLE_RECURSELOOP_CHECK, "DISABLE_RECURSELOOP_CHECK"),
    (PCRE2_NO_JIT, "NO_JIT"),
];

/// Compile configurations used for matching (a representative spread; the full
/// set is covered by the bytecode test above).
fn match_cfgs() -> Vec<Cfg> {
    let all = configs();
    let names = [
        "default",
        "CASELESS",
        "MULTILINE",
        "DOTALL",
        "UNGREEDY",
        "NO_START_OPTIMIZE",
        "NO_AUTO_POSSESS",
        "ANCHORED",
        "ENDANCHORED",
        "FIRSTLINE",
        "MATCH_UNSET_BACKREF",
        "AUTO_CALLOUT",
        "UTF",
        "UCP",
        "UTF|UCP",
        "UTF|CASELESS",
        "UTF|UCP|CASELESS",
        "UTF|MATCH_INVALID_UTF",
        "UTF|UCP|DOTALL|MULTILINE",
        "X:CASELESS_RESTRICT",
        "X:TURKISH_CASING",
        "X:ASCII_all|UCP|UTF",
        "X:MATCH_WORD",
        "X:MATCH_LINE",
        "own tables (pcre2_maketables)",
    ];
    let mut v: Vec<Cfg> = all
        .iter()
        .filter(|c| names.contains(&c.name))
        .cloned()
        .collect();
    // plus each newline convention with MULTILINE, and both \R conventions
    for nl in [
        PCRE2_NEWLINE_CR,
        PCRE2_NEWLINE_LF,
        PCRE2_NEWLINE_CRLF,
        PCRE2_NEWLINE_ANY,
        PCRE2_NEWLINE_ANYCRLF,
        PCRE2_NEWLINE_NUL,
    ] {
        let mut c = Cfg::new("MULTILINE + newline", PCRE2_MULTILINE, 0);
        c.newline = nl;
        v.push(c);
    }
    for bsr in [PCRE2_BSR_UNICODE, PCRE2_BSR_ANYCRLF] {
        let mut c = Cfg::new("UTF + bsr", PCRE2_UTF, 0);
        c.bsr = bsr;
        v.push(c);
    }
    v
}

// CONFIGS rows: pcre2_match over (compile cfg) x (pattern) x (subject) x
// (match options) x (startoffset) x (ovector size).
#[test]
fn match_identical() {
    let p = pair();
    let mut rng = Rng::new(100);
    let mut d = Diffs::new();
    // Bound the run: deterministic limits so both libraries hit them alike.
    let mca = unsafe { (p.c.match_context_create)(ptr::null_mut()) };
    let mcb = unsafe { (p.r.match_context_create)(ptr::null_mut()) };
    unsafe {
        for (m, v) in [(&p.c, mca), (&p.r, mcb)] {
            assert_eq!((m.set_match_limit)(v, 20_000), 0);
            assert_eq!((m.set_depth_limit)(v, 2_000), 0);
            assert_eq!((m.set_heap_limit)(v, 200), 0);
        }
    }
    for cfg in &match_cfgs() {
        for pat in PATTERNS {
            let pb = pat.as_bytes();
            unsafe {
                let Some((a, b)) = compile_both(p, pb, pb.len(), cfg, &mut d) else {
                    continue;
                };
                for subj in SUBJECTS {
                    let raw = subj.as_bytes();
                    let mut zt = raw.to_vec();
                    zt.push(0);
                    // a random slice of the option/offset/ovector axes per
                    // (pattern, subject) keeps the run bounded while still
                    // covering every axis many thousands of times overall
                    for _ in 0..3 {
                        let (mo, mn) = MOPTS[rng.below(MOPTS.len())];
                        let start = if raw.is_empty() { 0 } else { rng.below(raw.len() + 1) };
                        let ovec = *rng.pick(&[0u32, 1, 2, 4, 16, 64]);
                        let zero_term = rng.chance(4) && !raw.contains(&0);
                        let len = if zero_term { PCRE2_ZERO_TERMINATED } else { raw.len() };
                        let sb: &[u8] = if zero_term { &zt } else { raw };
                        let tag = format!(
                            "match {} cfg[{}] subj={} start={} mopts={} ovec={} len={}",
                            show(pb),
                            cfg.name,
                            show(sb),
                            start,
                            mn,
                            ovec,
                            if len == PCRE2_ZERO_TERMINATED { "ZT".into() } else { len.to_string() }
                        );
                        cmp_match(p, a, b, sb, len, start, mo, ovec, (mca, mcb), &tag, &mut d);
                    }
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
    }
    unsafe {
        (p.c.match_context_free)(mca);
        (p.r.match_context_free)(mcb);
    }
    d.finish("pcre2_match_8: match_cfgs x PATTERNS x SUBJECTS x MOPTS x startoffset x ovector size");
}

// CONFIGS rows: pcre2_dfa_match over the same axes, plus DFA_SHORTEST,
// DFA_RESTART and workspace sizes.
#[test]
fn dfa_match_identical() {
    let p = pair();
    let mut rng = Rng::new(200);
    let mut d = Diffs::new();
    let mca = unsafe { (p.c.match_context_create)(ptr::null_mut()) };
    let mcb = unsafe { (p.r.match_context_create)(ptr::null_mut()) };
    unsafe {
        for (m, v) in [(&p.c, mca), (&p.r, mcb)] {
            assert_eq!((m.set_match_limit)(v, 20_000), 0);
            assert_eq!((m.set_depth_limit)(v, 2_000), 0);
        }
    }
    let dfa_opts: &[(u32, &str)] = &[
        (0, "none"),
        (PCRE2_DFA_SHORTEST, "DFA_SHORTEST"),
        (PCRE2_NOTBOL, "NOTBOL"),
        (PCRE2_NOTEOL, "NOTEOL"),
        (PCRE2_NOTEMPTY, "NOTEMPTY"),
        (PCRE2_NOTEMPTY_ATSTART, "NOTEMPTY_ATSTART"),
        (PCRE2_ANCHORED, "ANCHORED"),
        (PCRE2_ENDANCHORED, "ENDANCHORED"),
        (PCRE2_PARTIAL_SOFT, "PARTIAL_SOFT"),
        (PCRE2_PARTIAL_HARD, "PARTIAL_HARD"),
        (PCRE2_NO_START_OPTIMIZE, "NO_START_OPTIMIZE"),
        (PCRE2_NO_UTF_CHECK, "NO_UTF_CHECK"),
        (PCRE2_COPY_MATCHED_SUBJECT, "COPY_MATCHED_SUBJECT"),
    ];
    for cfg in &match_cfgs() {
        for pat in PATTERNS {
            let pb = pat.as_bytes();
            unsafe {
                let Some((a, b)) = compile_both(p, pb, pb.len(), cfg, &mut d) else {
                    continue;
                };
                for subj in SUBJECTS {
                    let sb = subj.as_bytes();
                    for _ in 0..3 {
                        let (mo, mn) = dfa_opts[rng.below(dfa_opts.len())];
                        let start = if sb.is_empty() { 0 } else { rng.below(sb.len() + 1) };
                        let ovec = *rng.pick(&[0u32, 1, 2, 4, 16]);
                        // include a deliberately tiny workspace to reach
                        // PCRE2_ERROR_DFA_WSSIZE identically
                        let wsn = *rng.pick(&[20usize, 100, 1000]);
                        let mut wa = vec![0 as c_int; wsn];
                        let mut wb = vec![0 as c_int; wsn];
                        let mda = (p.c.match_data_create)(ovec, ptr::null_mut());
                        let mdb = (p.r.match_data_create)(ovec, ptr::null_mut());
                        let ra = (p.c.dfa_match)(
                            a, sb.as_ptr(), sb.len(), start, mo, mda, mca, wa.as_mut_ptr(), wsn,
                        );
                        let rb = (p.r.dfa_match)(
                            b, sb.as_ptr(), sb.len(), start, mo, mdb, mcb, wb.as_mut_ptr(), wsn,
                        );
                        let tag = format!(
                            "dfa_match {} cfg[{}] subj={} start={} opts={} ovec={} ws={}",
                            show(pb), cfg.name, show(sb), start, mn, ovec, wsn
                        );
                        d.eq(&tag, read_match_out_of(&p.c, mda, ra, Engine::Dfa), read_match_out_of(&p.r, mdb, rb, Engine::Dfa));
                        (p.c.match_data_free)(mda);
                        (p.r.match_data_free)(mdb);
                    }
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
    }
    unsafe {
        (p.c.match_context_free)(mca);
        (p.r.match_context_free)(mcb);
    }
    d.finish("pcre2_dfa_match_8: match_cfgs x PATTERNS x SUBJECTS x dfa options x workspace sizes");
}

// CONFIGS row: pcre2_next_match_8 — iterating the alternative match ends that
// pcre2_dfa_match records in the ovector.
#[test]
fn next_match_identical() {
    let p = pair();
    let mut d = Diffs::new();
    let cfgs = match_cfgs();
    for cfg in cfgs.iter().take(20) {
        for pat in PATTERNS {
            let pb = pat.as_bytes();
            unsafe {
                let Some((a, b)) = compile_both(p, pb, pb.len(), cfg, &mut d) else {
                    continue;
                };
                for subj in SUBJECTS {
                    let sb = subj.as_bytes();
                    let mut ws_a = vec![0 as c_int; 1000];
                    let mut ws_b = vec![0 as c_int; 1000];
                    let mda = (p.c.match_data_create)(16, ptr::null_mut());
                    let mdb = (p.r.match_data_create)(16, ptr::null_mut());
                    let ra = (p.c.dfa_match)(
                        a, sb.as_ptr(), sb.len(), 0, 0, mda, ptr::null_mut(), ws_a.as_mut_ptr(), 1000,
                    );
                    let rb = (p.r.dfa_match)(
                        b, sb.as_ptr(), sb.len(), 0, 0, mdb, ptr::null_mut(), ws_b.as_mut_ptr(), 1000,
                    );
                    let tag = format!("next_match {} cfg[{}] subj={}", show(pb), cfg.name, show(sb));
                    d.eq(&format!("{tag} dfa rc"), ra, rb);
                    // walk the whole iterator to exhaustion in both
                    let mut steps_a = Vec::new();
                    let mut steps_b = Vec::new();
                    loop {
                        let (mut o, mut f) = (usize::MAX, 0u32);
                        let rc = (p.c.next_match)(mda, &mut o, &mut f);
                        steps_a.push((rc, o, f));
                        if rc <= 0 || steps_a.len() > 40 {
                            break;
                        }
                    }
                    loop {
                        let (mut o, mut f) = (usize::MAX, 0u32);
                        let rc = (p.r.next_match)(mdb, &mut o, &mut f);
                        steps_b.push((rc, o, f));
                        if rc <= 0 || steps_b.len() > 40 {
                            break;
                        }
                    }
                    d.eq(&format!("{tag} next_match sequence"), steps_a, steps_b);
                    (p.c.match_data_free)(mda);
                    (p.r.match_data_free)(mdb);
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
    }
    d.finish("pcre2_next_match_8: full iteration after pcre2_dfa_match over cfgs x patterns x subjects");
}

// =========================================================== callouts

static mut CALLOUT_LOG: Vec<u8> = Vec::new();

/// Records the fields of every callout block so the two libraries' callout
/// sequences can be compared exactly.  Layout mirrors `pcre2_callout_block`.
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

/// `pcre2_callout_enumerate_block` — a DIFFERENT layout from
/// `pcre2_callout_block`, so it needs its own recorder.
#[repr(C)]
struct CalloutEnumBlock {
    version: u32,
    pattern_position: Sz,
    next_item_length: Sz,
    callout_number: u32,
    callout_string_offset: Sz,
    callout_string_length: Sz,
    callout_string: Sptr,
}

unsafe extern "C" fn record_enum_callout(blk: *mut c_void, _data: *mut c_void) -> c_int {
    let b = &*(blk as *const CalloutEnumBlock);
    let log = &mut *ptr::addr_of_mut!(CALLOUT_LOG);
    for v in [
        b.version as u64,
        b.pattern_position as u64,
        b.next_item_length as u64,
        b.callout_number as u64,
        b.callout_string_offset as u64,
        b.callout_string_length as u64,
    ] {
        log.extend_from_slice(&v.to_le_bytes());
    }
    if !b.callout_string.is_null() {
        log.extend_from_slice(std::slice::from_raw_parts(
            b.callout_string,
            b.callout_string_length,
        ));
    }
    0
}

unsafe extern "C" fn record_callout(blk: *mut c_void, _data: *mut c_void) -> c_int {
    let b = &*(blk as *const CalloutBlock);
    let log = &mut *ptr::addr_of_mut!(CALLOUT_LOG);
    for v in [
        b.version as u64,
        b.callout_number as u64,
        b.capture_top as u64,
        b.capture_last as u64,
        b.subject_length as u64,
        b.start_match as u64,
        b.current_position as u64,
        b.pattern_position as u64,
        b.next_item_length as u64,
        b.callout_string_offset as u64,
        b.callout_string_length as u64,
        b.callout_flags as u64,
        b.mark.is_null() as u64,
    ] {
        log.extend_from_slice(&v.to_le_bytes());
    }
    if !b.callout_string.is_null() {
        log.extend_from_slice(std::slice::from_raw_parts(
            b.callout_string,
            b.callout_string_length,
        ));
    }
    0
}

// CONFIGS rows: match-time callouts, with AUTO_CALLOUT and with explicit
// (?C) / (?C1) / (?C{str}) callouts — the whole callout sequence must match.
#[test]
fn callouts_identical() {
    let p = pair();
    let mut d = Diffs::new();
    let cal_pats: &[&str] = &[
        "a(?C)b",
        "a(?C1)b",
        "a(?C255)b",
        "a(?C{txt})b",
        "(?C0)a(?C1)b(?C2)c",
        "(a)(?C)(b)",
        "\\d+(?C9)\\w*",
        "(?:a(?C)|b(?C))+",
        "^(?C)a*(?C)$",
    ];
    for cfg in [
        Cfg::new("default", 0, 0),
        Cfg::new("AUTO_CALLOUT", PCRE2_AUTO_CALLOUT, 0),
        Cfg::new("UTF|AUTO_CALLOUT", PCRE2_UTF | PCRE2_AUTO_CALLOUT, 0),
        Cfg::new("UTF|UCP", PCRE2_UTF | PCRE2_UCP, 0),
    ] {
        // AUTO_CALLOUT applies to every pattern, so use the full corpus there
        let pats: Vec<&str> = if cfg.opts & PCRE2_AUTO_CALLOUT != 0 {
            PATTERNS.iter().copied().collect()
        } else {
            cal_pats.iter().copied().collect()
        };
        for pat in pats {
            let pb = pat.as_bytes();
            unsafe {
                let Some((a, b)) = compile_both(p, pb, pb.len(), &cfg, &mut d) else {
                    continue;
                };
                // pcre2_callout_enumerate walks the compiled pattern
                {
                    CALLOUT_LOG.clear();
                    let ea = (p.c.callout_enumerate)(a, Some(record_enum_callout), ptr::null_mut());
                    let la = CALLOUT_LOG.clone();
                    CALLOUT_LOG.clear();
                    let eb = (p.r.callout_enumerate)(b, Some(record_enum_callout), ptr::null_mut());
                    let lb = CALLOUT_LOG.clone();
                    d.eq(&format!("callout_enumerate rc {}", show(pb)), ea, eb);
                    d.eq(&format!("callout_enumerate log {}", show(pb)), la, lb);
                }
                let mca = (p.c.match_context_create)(ptr::null_mut());
                let mcb = (p.r.match_context_create)(ptr::null_mut());
                assert_eq!((p.c.set_callout)(mca, Some(record_callout), ptr::null_mut()), 0);
                assert_eq!((p.r.set_callout)(mcb, Some(record_callout), ptr::null_mut()), 0);
                for subj in SUBJECTS {
                    let sb = subj.as_bytes();
                    for &(mo, mn) in &[(0u32, "none"), (PCRE2_NO_START_OPTIMIZE, "NO_START_OPT")] {
                        let mda = (p.c.match_data_create)(16, ptr::null_mut());
                        let mdb = (p.r.match_data_create)(16, ptr::null_mut());
                        CALLOUT_LOG.clear();
                        let ra = (p.c.do_match)(a, sb.as_ptr(), sb.len(), 0, mo, mda, mca);
                        let la = CALLOUT_LOG.clone();
                        CALLOUT_LOG.clear();
                        let rb = (p.r.do_match)(b, sb.as_ptr(), sb.len(), 0, mo, mdb, mcb);
                        let lb = CALLOUT_LOG.clone();
                        let tag =
                            format!("callout match {} cfg[{}] subj={} {}", show(pb), cfg.name, show(sb), mn);
                        d.eq(&tag, read_match_out_of(&p.c, mda, ra, Engine::Dfa), read_match_out_of(&p.r, mdb, rb, Engine::Dfa));
                        d.eq(&format!("{tag} :: callout log"), la, lb);
                        (p.c.match_data_free)(mda);
                        (p.r.match_data_free)(mdb);
                    }
                }
                (p.c.match_context_free)(mca);
                (p.r.match_context_free)(mcb);
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
    }
    d.finish("callouts: explicit (?C..) and AUTO_CALLOUT, pcre2_callout_enumerate_8 + match-time sequence");
}

// ================================================== bytecode-level internals

/// Finds the unique occurrence of `op` in a compiled pattern's bytecode.
/// Returns the byte offset from the bytecode start, or None if not unique.
unsafe fn unique_op(code: Ptr, op: u8) -> Option<usize> {
    let start = bytecode_ptr(code);
    let n = code_blocksize(code) - (start as usize - code as usize);
    let by = std::slice::from_raw_parts(start, n);
    let mut found = None;
    for i in 0..n.saturating_sub(3) {
        if by[i] != op {
            continue;
        }
        let link = ((by[i + 1] as usize) << 8) | by[i + 2] as usize; // LINK_SIZE == 2
        if link >= 4 && i + link <= n {
            if found.is_some() {
                return None; // ambiguous
            }
            found = Some(i);
        }
    }
    found
}

const OP_XCLASS: u8 = 112;
const OP_ECLASS: u8 = 113;

// CONFIGS row: _pcre2_xclass_8 and _pcre2_eclass_8 driven over every code point
// class boundary, using real XCLASS/ECLASS bytecode produced by the compiler.
#[test]
fn xclass_and_eclass_identical() {
    let p = pair();
    let mut d = Diffs::new();
    // patterns that produce exactly one wide/extended class
    let xpats: &[(&str, u32, u32)] = &[
        ("[\\x{100}-\\x{200}]", PCRE2_UTF, 0),
        ("[^\\x{100}-\\x{200}]", PCRE2_UTF, 0),
        ("[\\x{100}\\x{300}\\x{500}]", PCRE2_UTF, 0),
        ("[a-c\\x{100}-\\x{200}]", PCRE2_UTF, 0),
        ("[\\p{L}]", PCRE2_UTF | PCRE2_UCP, 0),
        ("[\\P{L}]", PCRE2_UTF | PCRE2_UCP, 0),
        ("[\\p{Greek}\\p{Nd}]", PCRE2_UTF | PCRE2_UCP, 0),
        ("[\\p{Lu}a-f]", PCRE2_UTF | PCRE2_UCP, 0),
        ("[\\x{100}-\\x{10ffff}]", PCRE2_UTF, 0),
        ("[[:alpha:]\\x{100}]", PCRE2_UTF, 0),
        ("(?[\\p{L} & \\p{Greek}])", PCRE2_UTF | PCRE2_UCP | PCRE2_ALT_EXTENDED_CLASS, 0),
        ("(?[\\p{L} -- [a-z]])", PCRE2_UTF | PCRE2_UCP | PCRE2_ALT_EXTENDED_CLASS, 0),
        ("(?[[a-z] | \\p{Nd}])", PCRE2_UTF | PCRE2_UCP | PCRE2_ALT_EXTENDED_CLASS, 0),
        ("(?[\\p{L} ^ \\p{Lu}])", PCRE2_UTF | PCRE2_UCP | PCRE2_ALT_EXTENDED_CLASS, 0),
        ("[\\p{Xan}\\x{2000}-\\x{2fff}]", PCRE2_UTF | PCRE2_UCP, 0),
    ];
    // code points at and around every interesting boundary, plus a sweep
    let mut cps: Vec<u32> = vec![
        0, 1, 0x40, 0x41, 0x5a, 0x61, 0x7a, 0x7f, 0x80, 0xff, 0x100, 0x101, 0x1ff, 0x200, 0x201,
        0x2ff, 0x300, 0x37f, 0x386, 0x3b1, 0x400, 0x2000, 0x2028, 0x2fff, 0xffff, 0x10000,
        0x10ffff,
    ];
    for i in 0..2000u32 {
        cps.push(i * 137 % 0x11_0000);
    }
    for &(pat, opts, xopts) in xpats {
        let mut cfg = Cfg::new("xclass", opts, xopts);
        cfg.name = "xclass probe";
        let pb = pat.as_bytes();
        unsafe {
            let Some((a, b)) = compile_both(p, pb, pb.len(), &cfg, &mut d) else {
                continue;
            };
            let sa = bytecode_ptr(a);
            let sb = bytecode_ptr(b);
            for (op, is_e) in [(OP_XCLASS, false), (OP_ECLASS, true)] {
                let Some(off) = unique_op(a, op) else { continue };
                assert_eq!(Some(off), unique_op(b, op), "opcode offset differs for {pat}");
                for utf in [0 as Bool, 1] {
                    for &c in &cps {
                        let da = sa.add(off + 3); // skip opcode + LINK_SIZE
                        let db = sb.add(off + 3);
                        if is_e {
                            let link = {
                                let by = std::slice::from_raw_parts(sa.add(off), 3);
                                ((by[1] as usize) << 8) | by[2] as usize
                            };
                            d.eq(
                                &format!("eclass({c:#x}, {pat}, utf={utf})"),
                                (p.c.p_eclass)(c, da, sa.add(off + link), sa, utf),
                                (p.r.p_eclass)(c, db, sb.add(off + link), sb, utf),
                            );
                        } else {
                            d.eq(
                                &format!("xclass({c:#x}, {pat}, utf={utf})"),
                                (p.c.p_xclass)(c, da, sa, utf),
                                (p.r.p_xclass)(c, db, sb, utf),
                            );
                        }
                    }
                }
            }
            (p.c.code_free)(a);
            (p.r.code_free)(b);
        }
    }
    d.finish("_pcre2_xclass_8 / _pcre2_eclass_8: real XCLASS/ECLASS bytecode x ~2000 code points x utf");
}

// CONFIGS row: _pcre2_find_bracket_8 over real bytecode, for every group
// number present plus out-of-range and negative numbers.
#[test]
fn find_bracket_identical() {
    let p = pair();
    let mut d = Diffs::new();
    for cfg in [
        Cfg::new("default", 0, 0),
        Cfg::new("UTF", PCRE2_UTF, 0),
        Cfg::new("DUPNAMES", PCRE2_DUPNAMES, 0),
        Cfg::new("NO_AUTO_CAPTURE", PCRE2_NO_AUTO_CAPTURE, 0),
    ] {
        for pat in PATTERNS {
            let pb = pat.as_bytes();
            unsafe {
                let Some((a, b)) = compile_both(p, pb, pb.len(), &cfg, &mut d) else {
                    continue;
                };
                let (sa, sb) = (bytecode_ptr(a), bytecode_ptr(b));
                let mut top = 0u32;
                (p.c.pattern_info)(a, PCRE2_INFO_CAPTURECOUNT, &mut top as *mut u32 as Ptr);
                for utf in [0 as Bool, 1] {
                    for num in -3i32..=(top as i32 + 3) {
                        let ra = (p.c.p_find_bracket)(sa, utf, num);
                        let rb = (p.r.p_find_bracket)(sb, utf, num);
                        d.eq(
                            &format!("find_bracket({} cfg[{}] n={num} utf={utf})", show(pb), cfg.name),
                            if ra.is_null() { None } else { Some(ra as usize - sa as usize) },
                            if rb.is_null() { None } else { Some(rb as usize - sb as usize) },
                        );
                    }
                }
                (p.c.code_free)(a);
                (p.r.code_free)(b);
            }
        }
    }
    d.finish("_pcre2_find_bracket_8: real bytecode x group numbers -3..top+3 x utf on/off");
}
