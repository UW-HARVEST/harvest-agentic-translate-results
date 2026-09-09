//! CONFIGS.md rows 23–82 and ERRORS.md Part 2 — `pcre2_compile` under every
//! option / context configuration, compared through the deterministic
//! `pcre2_serialize_encode` image of the compiled pattern plus all 27
//! `pcre2_pattern_info` results, and through `(errorcode, erroroffset)` when the
//! compile is rejected.
mod common;
use common::corpus::*;
use common::*;
use std::ffi::{c_int, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Result of compiling one pattern in one library, in a comparable form.
#[derive(PartialEq, Eq)]
struct CompileResult {
    err: c_int,
    off: Sz,
    ser: Option<Vec<u8>>,
    info: Option<String>,
    ser_rc: i32,
}

impl std::fmt::Debug for CompileResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "err={} off={} ser_rc={} ", self.err, self.off, self.ser_rc)?;
        match &self.ser {
            None => write!(f, "ser=none ")?,
            Some(v) => write!(f, "ser[{}] ", v.len())?,
        }
        match &self.info {
            None => write!(f, "info=none"),
            Some(s) => write!(f, "info={s}"),
        }
    }
}

unsafe fn compile_and_snapshot(
    a: &Api,
    pat: &[u8],
    patlen: Sz,
    options: u32,
    cc: *mut Ctx,
) -> CompileResult {
    let mut err: c_int = 0x5A5A;
    let mut off: Sz = 0x5A5A;
    let code =
        unsafe { (a.compile)(pat.as_ptr(), patlen, options, &mut err, &mut off, cc) };
    if code.is_null() {
        return CompileResult { err, off, ser: None, info: None, ser_rc: 0 };
    }
    let (ser, ser_rc) = match unsafe { serialize_bytes(a, code) } {
        Ok(v) => (Some(v), 1),
        Err(rc) => (None, rc),
    };
    let info = Some(unsafe { info_dump(a, code) });
    unsafe { (a.code_free)(code) };
    CompileResult { err, off, ser, info, ser_rc }
}

/// The serialized image starts with `pcre2_serialized_data` (magic, version,
/// config, number_of_codes) followed by the 1088-byte character tables and then
/// the compiled code blocks. Strip nothing — both must be byte-identical.
fn diff_report(pat: &[u8], patlen: Sz, opts: u32, note: &str, c: &CompileResult, r: &CompileResult) -> String {
    let mut s = format!(
        "compile divergence\n  pattern = {:?} (raw {:02x?})\n  patlen  = {patlen}\n  options = 0x{opts:08x}\n  {note}\n  C    : {c:?}\n  RUST : {r:?}\n",
        String::from_utf8_lossy(pat),
        pat
    );
    if let (Some(cv), Some(rv)) = (&c.ser, &r.ser) {
        if cv != rv {
            let at = cv
                .iter()
                .zip(rv.iter())
                .position(|(x, y)| x != y)
                .unwrap_or(cv.len().min(rv.len()));
            s.push_str(&format!(
                "  serialized differs: C len={} RUST len={} first diff at {at}\n    C   : {:02x?}\n    RUST: {:02x?}\n",
                cv.len(),
                rv.len(),
                &cv[at.saturating_sub(8)..(at + 24).min(cv.len())],
                &rv[at.saturating_sub(8)..(at + 24).min(rv.len())],
            ));
        }
    }
    if let (Some(ci), Some(ri)) = (&c.info, &r.info) {
        if ci != ri {
            for (cf, rf) in ci.split(';').zip(ri.split(';')) {
                if cf != rf {
                    s.push_str(&format!("  info differs: C[{cf}] RUST[{rf}]\n"));
                }
            }
        }
    }
    s
}

static FAILURES: AtomicUsize = AtomicUsize::new(0);

/// Compile the pattern in both libraries and assert equality.
fn check(pat: &[u8], patlen: Sz, opts: u32, cc: (*mut Ctx, *mut Ctx), note: &str) {
    let (c, r) = pair();
    let cres = unsafe { compile_and_snapshot(c, pat, patlen, opts, cc.0) };
    let rres = unsafe { compile_and_snapshot(r, pat, patlen, opts, cc.1) };
    if cres != rres {
        FAILURES.fetch_add(1, Ordering::Relaxed);
        panic!("{}", diff_report(pat, patlen, opts, note, &cres, &rres));
    }
}

fn zpat(p: &str) -> Vec<u8> {
    let mut v = p.as_bytes().to_vec();
    v.push(0);
    v
}

fn run_corpus(opts: u32, note: &str) {
    let (c, r) = pair();
    let _ = (c, r);
    for p in PATTERNS {
        let v = zpat(p);
        check(&v, p.len(), opts, (std::ptr::null_mut(), std::ptr::null_mut()), note);
    }
}

// ----------------------------------------------- rows 23-53: single options ---

macro_rules! opt_row {
    ($name:ident, $opts:expr) => {
        #[test]
        fn $name() {
            run_corpus($opts, stringify!($name));
        }
    };
}

opt_row!(row23_baseline, 0);
opt_row!(row24_anchored, PCRE2_ANCHORED);
opt_row!(row25_endanchored, PCRE2_ENDANCHORED);
opt_row!(row26_allow_empty_class, PCRE2_ALLOW_EMPTY_CLASS);
opt_row!(row27_alt_bsux, PCRE2_ALT_BSUX);
opt_row!(row28_auto_callout, PCRE2_AUTO_CALLOUT);
opt_row!(row29_caseless, PCRE2_CASELESS);
opt_row!(row30_dollar_endonly, PCRE2_DOLLAR_ENDONLY);
opt_row!(row31_dotall, PCRE2_DOTALL);
opt_row!(row32_dupnames, PCRE2_DUPNAMES);
opt_row!(row33_extended, PCRE2_EXTENDED);
opt_row!(row34_extended_more, PCRE2_EXTENDED_MORE);
opt_row!(row35_firstline, PCRE2_FIRSTLINE);
opt_row!(row36_literal, PCRE2_LITERAL);
opt_row!(row37_match_unset_backref, PCRE2_MATCH_UNSET_BACKREF);
opt_row!(row38_multiline, PCRE2_MULTILINE);
opt_row!(row39_no_auto_capture, PCRE2_NO_AUTO_CAPTURE);
opt_row!(row40_no_auto_possess, PCRE2_NO_AUTO_POSSESS);
opt_row!(row41_no_dotstar_anchor, PCRE2_NO_DOTSTAR_ANCHOR);
opt_row!(row42_no_start_optimize, PCRE2_NO_START_OPTIMIZE);
opt_row!(row43_ucp, PCRE2_UCP);
opt_row!(row44_ungreedy, PCRE2_UNGREEDY);
opt_row!(row45_utf, PCRE2_UTF);
opt_row!(row46_utf_ucp, PCRE2_UTF | PCRE2_UCP);
opt_row!(row47_utf_no_utf_check, PCRE2_UTF | PCRE2_NO_UTF_CHECK);
opt_row!(row48_match_invalid_utf, PCRE2_MATCH_INVALID_UTF);
opt_row!(row49_alt_circumflex, PCRE2_ALT_CIRCUMFLEX);
opt_row!(row50_alt_verbnames, PCRE2_ALT_VERBNAMES);
opt_row!(row51_alt_extended_class, PCRE2_ALT_EXTENDED_CLASS);
opt_row!(row52_use_offset_limit, PCRE2_USE_OFFSET_LIMIT);
opt_row!(row53a_never_ucp, PCRE2_NEVER_UCP);
opt_row!(row53b_never_utf, PCRE2_NEVER_UTF);
opt_row!(row53c_never_backslash_c, PCRE2_NEVER_BACKSLASH_C);

const ALL_COMPILE_OPTS: &[u32] = &[
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

#[test]
fn row54_random_option_combinations() {
    let mut rng = Rng::new(SEED ^ 54);
    for _ in 0..iters(4000) {
        let mut opts = 0u32;
        for _ in 0..(1 + rng.below(3)) {
            opts |= *rng.pick(ALL_COMPILE_OPTS);
        }
        let p = if rng.bool() {
            rng.pick(PATTERNS).to_string()
        } else {
            random_pattern(&mut rng)
        };
        let v = zpat(&p);
        check(
            &v,
            p.len(),
            opts,
            (std::ptr::null_mut(), std::ptr::null_mut()),
            "row54 random combo",
        );
    }
}

#[test]
fn row_random_patterns_baseline() {
    let mut rng = Rng::new(SEED ^ 0x5A);
    for _ in 0..iters(20000) {
        let p = random_pattern(&mut rng);
        let v = zpat(&p);
        check(&v, p.len(), 0, (std::ptr::null_mut(), std::ptr::null_mut()), "random baseline");
    }
}

// ------------------------------------------- rows 55-68: extra options --------

const ALL_EXTRA_OPTS: &[u32] = &[
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

/// Run the corpus with a compile context configured by `setup`.
fn run_with_ctx(opts: u32, note: &str, setup: impl Fn(&Api, *mut Ctx)) {
    let (c, r) = pair();
    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    assert!(!cc.is_null() && !rc.is_null());
    setup(c, cc);
    setup(r, rc);
    for p in PATTERNS {
        let v = zpat(p);
        check(&v, p.len(), opts, (cc, rc), note);
    }
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };
}

#[test]
fn rows55_67_each_extra_option() {
    for &x in ALL_EXTRA_OPTS {
        for base in [0u32, PCRE2_UTF, PCRE2_UCP, PCRE2_CASELESS, PCRE2_UTF | PCRE2_CASELESS]
        {
            run_with_ctx(base, &format!("extra=0x{x:08x} base=0x{base:08x}"), |a, ctx| {
                assert_eq!(unsafe { (a.set_compile_extra_options)(ctx, x) }, 0);
            });
        }
    }
}

#[test]
fn row68_random_extra_option_combinations() {
    let (c, r) = pair();
    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    let mut rng = Rng::new(SEED ^ 68);
    for _ in 0..iters(4000) {
        let mut x = 0u32;
        for _ in 0..(1 + rng.below(3)) {
            x |= *rng.pick(ALL_EXTRA_OPTS);
        }
        unsafe { (c.set_compile_extra_options)(cc, x) };
        unsafe { (r.set_compile_extra_options)(rc, x) };
        let mut opts = 0u32;
        for _ in 0..rng.below(3) {
            opts |= *rng.pick(ALL_COMPILE_OPTS);
        }
        let p = if rng.bool() {
            rng.pick(PATTERNS).to_string()
        } else {
            random_pattern(&mut rng)
        };
        let v = zpat(&p);
        check(&v, p.len(), opts, (cc, rc), &format!("extra=0x{x:08x}"));
    }
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };
}

// ----------------------------------------- rows 69-77: context settings ------

#[test]
fn row69_newline_conventions() {
    for nl in 1u32..=6 {
        for base in [0u32, PCRE2_MULTILINE, PCRE2_DOTALL, PCRE2_UTF] {
            run_with_ctx(base, &format!("newline={nl}"), |a, ctx| {
                assert_eq!(unsafe { (a.set_newline)(ctx, nl) }, 0);
            });
        }
    }
}

#[test]
fn row70_bsr_conventions() {
    for bsr in 1u32..=2 {
        for base in [0u32, PCRE2_UTF] {
            run_with_ctx(base, &format!("bsr={bsr}"), |a, ctx| {
                assert_eq!(unsafe { (a.set_bsr)(ctx, bsr) }, 0);
            });
        }
    }
}

#[test]
fn row71_max_varlookbehind() {
    for lim in [0u32, 1, 2, 3, 20, 255, 65535, u32::MAX] {
        run_with_ctx(0, &format!("max_varlookbehind={lim}"), |a, ctx| {
            assert_eq!(unsafe { (a.set_max_varlookbehind)(ctx, lim) }, 0);
        });
    }
}

#[test]
fn row72_parens_nest_limit() {
    for lim in [0u32, 1, 2, 5, 10, 250, 65535, u32::MAX] {
        run_with_ctx(0, &format!("parens_nest_limit={lim}"), |a, ctx| {
            assert_eq!(unsafe { (a.set_parens_nest_limit)(ctx, lim) }, 0);
        });
    }
}

#[test]
fn row73_max_pattern_length() {
    let (c, r) = pair();
    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    for p in PATTERNS {
        let n = p.len();
        for lim in [0usize, 1, n.saturating_sub(1), n, n + 1, Sz::MAX] {
            unsafe { (c.set_max_pattern_length)(cc, lim) };
            unsafe { (r.set_max_pattern_length)(rc, lim) };
            let v = zpat(p);
            check(&v, n, 0, (cc, rc), &format!("max_pattern_length={lim}"));
        }
    }
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };
}

#[test]
fn row74_max_pattern_compiled_length() {
    let (c, r) = pair();
    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    for lim in [0usize, 1, 8, 32, 64, 128, 200, 1000, 100000, Sz::MAX] {
        unsafe { (c.set_max_pattern_compiled_length)(cc, lim) };
        unsafe { (r.set_max_pattern_compiled_length)(rc, lim) };
        for p in PATTERNS {
            let v = zpat(p);
            check(&v, p.len(), 0, (cc, rc), &format!("max_compiled_length={lim}"));
        }
    }
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };
}

#[test]
fn row75_character_tables() {
    let (c, r) = pair();
    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    let ct = unsafe { (c.maketables)(std::ptr::null_mut()) };
    let rt = unsafe { (r.maketables)(std::ptr::null_mut()) };
    unsafe { (c.set_character_tables)(cc, ct) };
    unsafe { (r.set_character_tables)(rc, rt) };
    for p in PATTERNS {
        let v = zpat(p);
        check(&v, p.len(), PCRE2_CASELESS, (cc, rc), "maketables");
    }
    // explicitly re-set to the built-in default tables
    unsafe { (c.set_character_tables)(cc, c.data("_pcre2_default_tables_8")) };
    unsafe { (r.set_character_tables)(rc, r.data("_pcre2_default_tables_8")) };
    for p in PATTERNS {
        let v = zpat(p);
        check(&v, p.len(), PCRE2_CASELESS, (cc, rc), "default tables");
    }
    unsafe { (c.maketables_free)(std::ptr::null_mut(), ct) };
    unsafe { (r.maketables_free)(std::ptr::null_mut(), rt) };
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };
}

#[test]
fn row76_optimize_directives() {
    // PCRE2_OPTIMIZATION_NONE/FULL plus each on/off directive 64..=67.
    for d in [0u32, 1, 64, 65, 66, 67] {
        run_with_ctx(0, &format!("optimize={d}"), |a, ctx| {
            assert_eq!(unsafe { (a.set_optimize)(ctx, d) }, 0, "set_optimize({d})");
        });
    }
    // sequences of directives
    let (c, r) = pair();
    let cc = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    let mut rng = Rng::new(SEED ^ 76);
    for _ in 0..iters(300) {
        let d1 = *rng.pick(&[0u32, 1, 64, 65, 66, 67]);
        let d2 = *rng.pick(&[0u32, 1, 64, 65, 66, 67]);
        assert_eq!(unsafe { (c.set_optimize)(cc, d1) }, unsafe {
            (r.set_optimize)(rc, d1)
        });
        assert_eq!(unsafe { (c.set_optimize)(cc, d2) }, unsafe {
            (r.set_optimize)(rc, d2)
        });
        let p = rng.pick(PATTERNS);
        let v = zpat(p);
        check(&v, p.len(), 0, (cc, rc), &format!("optimize {d1} then {d2}"));
    }
    unsafe { (c.compile_context_free)(cc) };
    unsafe { (r.compile_context_free)(rc) };
}

static GUARD_DEPTH: AtomicUsize = AtomicUsize::new(0);
extern "C" fn guard_ok(_d: u32, _u: *mut c_void) -> c_int {
    0
}
extern "C" fn guard_deny(d: u32, _u: *mut c_void) -> c_int {
    if d as usize >= GUARD_DEPTH.load(Ordering::Relaxed) { 1 } else { 0 }
}

#[test]
fn row77_compile_recursion_guard() {
    run_with_ctx(0, "guard always 0", |a, ctx| {
        assert_eq!(
            unsafe { (a.set_compile_recursion_guard)(ctx, Some(guard_ok), std::ptr::null_mut()) },
            0
        );
    });
    for depth in [0usize, 1, 2, 3, 5] {
        GUARD_DEPTH.store(depth, Ordering::Relaxed);
        run_with_ctx(0, &format!("guard deny at {depth}"), |a, ctx| {
            unsafe {
                (a.set_compile_recursion_guard)(ctx, Some(guard_deny), std::ptr::null_mut())
            };
        });
    }
}

// ------------------------------------------ row 78: pattern length shapes ----

#[test]
fn row78_length_shapes() {
    for p in PATTERNS {
        let v = zpat(p);
        // explicit length, zero-terminated, and a truncated length
        check(&v, p.len(), 0, (std::ptr::null_mut(), std::ptr::null_mut()), "explicit len");
        check(&v, PCRE2_ZERO_TERMINATED, 0, (std::ptr::null_mut(), std::ptr::null_mut()), "zero-terminated");
        for cut in [0usize, 1, 2, p.len() / 2] {
            if cut <= p.len() {
                check(&v, cut, 0, (std::ptr::null_mut(), std::ptr::null_mut()), "truncated len");
            }
        }
    }
    // embedded NUL
    let with_nul: &[u8] = b"a\0b\0";
    check(with_nul, 3, 0, (std::ptr::null_mut(), std::ptr::null_mut()), "embedded NUL");
    check(with_nul, PCRE2_ZERO_TERMINATED, 0, (std::ptr::null_mut(), std::ptr::null_mut()), "NUL-terminated at 1");
    // long pattern
    let long = "a".repeat(4096);
    let v = zpat(&long);
    check(&v, long.len(), 0, (std::ptr::null_mut(), std::ptr::null_mut()), "4KiB literal");
    let long2 = "(a)".repeat(1000);
    let v = zpat(&long2);
    check(&v, long2.len(), 0, (std::ptr::null_mut(), std::ptr::null_mut()), "1000 groups");
    let long3 = "(?:".repeat(300) + &")".repeat(300);
    let v = zpat(&long3);
    check(&v, long3.len(), 0, (std::ptr::null_mut(), std::ptr::null_mut()), "300 deep");
    let long4 = "[".to_string() + &"a-z".repeat(500) + "]";
    let v = zpat(&long4);
    check(&v, long4.len(), 0, (std::ptr::null_mut(), std::ptr::null_mut()), "big class");
    // NULL pattern with length 0 (ERRORS row 5)
    let empty: [u8; 1] = [0];
    let _ = empty;
    let (c, r) = pair();
    let mut ce: c_int = 0;
    let mut co: Sz = 0;
    let mut re: c_int = 0;
    let mut ro: Sz = 0;
    let cc = unsafe {
        (c.compile)(std::ptr::null(), 0, 0, &mut ce, &mut co, std::ptr::null_mut())
    };
    let rr = unsafe {
        (r.compile)(std::ptr::null(), 0, 0, &mut re, &mut ro, std::ptr::null_mut())
    };
    assert_eq!((ce, co, cc.is_null()), (re, ro, rr.is_null()), "NULL pattern len 0");
    if !cc.is_null() {
        assert_eq!(
            unsafe { serialize_bytes(c, cc) },
            unsafe { serialize_bytes(r, rr) },
            "NULL pattern serialized"
        );
        unsafe { (c.code_free)(cc) };
        unsafe { (r.code_free)(rr) };
    }
}

// -------------------------------------- row 82: code_copy / with_tables ------

#[test]
fn row82_code_copy() {
    let (c, r) = pair();
    for p in PATTERNS {
        let v = zpat(p);
        let mut ce: c_int = 0;
        let mut co: Sz = 0;
        let mut re: c_int = 0;
        let mut ro: Sz = 0;
        let cc = unsafe {
            (c.compile)(v.as_ptr(), p.len(), 0, &mut ce, &mut co, std::ptr::null_mut())
        };
        let rr = unsafe {
            (r.compile)(v.as_ptr(), p.len(), 0, &mut re, &mut ro, std::ptr::null_mut())
        };
        assert_eq!(cc.is_null(), rr.is_null(), "{p:?}");
        if cc.is_null() {
            continue;
        }
        for with_tables in [false, true] {
            let cp = if with_tables {
                unsafe { (c.code_copy_with_tables)(cc) }
            } else {
                unsafe { (c.code_copy)(cc) }
            };
            let rp = if with_tables {
                unsafe { (r.code_copy_with_tables)(rr) }
            } else {
                unsafe { (r.code_copy)(rr) }
            };
            assert!(!cp.is_null() && !rp.is_null());
            assert_eq!(
                unsafe { serialize_bytes(c, cp) },
                unsafe { serialize_bytes(r, rp) },
                "code_copy(with_tables={with_tables}) {p:?}"
            );
            assert_eq!(
                unsafe { info_dump(c, cp) },
                unsafe { info_dump(r, rp) },
                "code_copy info {p:?}"
            );
            // NOTE: `pcre2_code_copy_with_tables` sets PCRE2_DEREF_TABLES in
            // `flags`, so the copy is deliberately NOT byte-identical to the
            // original in the C code. Only C-vs-Rust equality is asserted.
            unsafe { (c.code_free)(cp) };
            unsafe { (r.code_free)(rp) };
        }
        unsafe { (c.code_free)(cc) };
        unsafe { (r.code_free)(rr) };
    }
}

// ------------------------------------------------ row 150: _pcre2_study ------

#[test]
fn row150_study_reentrant() {
    let (c, r) = pair();
    for p in PATTERNS {
        let v = zpat(p);
        let mut ce: c_int = 0;
        let mut co: Sz = 0;
        let mut re: c_int = 0;
        let mut ro: Sz = 0;
        let cc = unsafe {
            (c.compile)(v.as_ptr(), p.len(), 0, &mut ce, &mut co, std::ptr::null_mut())
        };
        let rr = unsafe {
            (r.compile)(v.as_ptr(), p.len(), 0, &mut re, &mut ro, std::ptr::null_mut())
        };
        if cc.is_null() {
            assert!(rr.is_null());
            continue;
        }
        let crc = unsafe { (c.priv_study)(cc) };
        let rrc = unsafe { (r.priv_study)(rr) };
        assert_eq!(crc, rrc, "_pcre2_study rc for {p:?}");
        assert_eq!(
            unsafe { serialize_bytes(c, cc) },
            unsafe { serialize_bytes(r, rr) },
            "state after _pcre2_study for {p:?}"
        );
        unsafe { (c.code_free)(cc) };
        unsafe { (r.code_free)(rr) };
    }
}

// -------------------------------- row 81: pcre2_callout_enumerate ------------

static ENUM_LOG: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

#[repr(C)]
struct EnumBlock {
    version: u32,
    _pad: u32,
    pattern_position: Sz,
    next_item_length: Sz,
    callout_number: u32,
    _pad2: u32,
    callout_string_offset: Sz,
    callout_string_length: Sz,
    callout_string: *const u8,
}

extern "C" fn enum_cb(b: *mut c_void, _u: *mut c_void) -> c_int {
    let b = unsafe { &*(b as *const EnumBlock) };
    let s = if b.callout_string.is_null() {
        "null".to_string()
    } else {
        let sl = unsafe {
            std::slice::from_raw_parts(b.callout_string, b.callout_string_length)
        };
        format!("{sl:02x?}")
    };
    ENUM_LOG.lock().unwrap().push(format!(
        "v={} pos={} len={} num={} soff={} slen={} s={}",
        b.version,
        b.pattern_position,
        b.next_item_length,
        b.callout_number,
        b.callout_string_offset,
        b.callout_string_length,
        s
    ));
    0
}

#[test]
fn row81_callout_enumerate() {
    let (c, r) = pair();
    for opts in [0u32, PCRE2_AUTO_CALLOUT, PCRE2_AUTO_CALLOUT | PCRE2_UTF] {
        for p in PATTERNS {
            let v = zpat(p);
            let mut ce: c_int = 0;
            let mut co: Sz = 0;
            let mut re: c_int = 0;
            let mut ro: Sz = 0;
            let cc = unsafe {
                (c.compile)(v.as_ptr(), p.len(), opts, &mut ce, &mut co, std::ptr::null_mut())
            };
            let rr = unsafe {
                (r.compile)(v.as_ptr(), p.len(), opts, &mut re, &mut ro, std::ptr::null_mut())
            };
            if cc.is_null() {
                assert!(rr.is_null());
                continue;
            }
            ENUM_LOG.lock().unwrap().clear();
            let crc = unsafe { (c.callout_enumerate)(cc, Some(enum_cb), std::ptr::null_mut()) };
            let clog = std::mem::take(&mut *ENUM_LOG.lock().unwrap());
            let rrc = unsafe { (r.callout_enumerate)(rr, Some(enum_cb), std::ptr::null_mut()) };
            let rlog = std::mem::take(&mut *ENUM_LOG.lock().unwrap());
            assert_eq!(crc, rrc, "callout_enumerate rc for {p:?} opts=0x{opts:x}");
            assert_eq!(clog, rlog, "callout_enumerate log for {p:?} opts=0x{opts:x}");
            unsafe { (c.code_free)(cc) };
            unsafe { (r.code_free)(rr) };
        }
    }
}
