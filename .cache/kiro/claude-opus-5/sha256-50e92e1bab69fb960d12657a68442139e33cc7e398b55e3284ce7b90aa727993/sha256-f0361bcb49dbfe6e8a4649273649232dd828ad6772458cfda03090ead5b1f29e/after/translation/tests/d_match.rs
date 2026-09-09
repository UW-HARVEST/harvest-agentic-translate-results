//! CONFIGS.md rows 83–114 — `pcre2_match`, `pcre2_dfa_match`, `pcre2_jit_match`
//! and the match-data accessors, driven through both `.so` files.
mod common;
use common::corpus::*;
use common::*;
use std::ffi::{c_int, c_void};
use std::sync::Mutex;

fn zpat(p: &str) -> Vec<u8> {
    let mut v = p.as_bytes().to_vec();
    v.push(0);
    v
}

/// A compiled pattern in both libraries (skipped when the pattern is invalid).
struct Pair2 {
    cc: *mut Code,
    rc: *mut Code,
}

fn compile_pair(copts: u32, pat: &str) -> Option<Pair2> {
    let (c, r) = pair();
    let v = zpat(pat);
    let mut ce: c_int = 0;
    let mut co: Sz = 0;
    let mut re: c_int = 0;
    let mut ro: Sz = 0;
    let cc = unsafe {
        (c.compile)(v.as_ptr(), pat.len(), copts, &mut ce, &mut co, std::ptr::null_mut())
    };
    let rc = unsafe {
        (r.compile)(v.as_ptr(), pat.len(), copts, &mut re, &mut ro, std::ptr::null_mut())
    };
    assert_eq!(
        (ce, co, cc.is_null()),
        (re, ro, rc.is_null()),
        "compile mismatch for {pat:?} opts=0x{copts:08x}"
    );
    if cc.is_null() {
        return None;
    }
    Some(Pair2 { cc, rc })
}

impl Drop for Pair2 {
    fn drop(&mut self) {
        let (c, r) = pair();
        unsafe { (c.code_free)(self.cc) };
        unsafe { (r.code_free)(self.rc) };
    }
}

/// Run `pcre2_match` on both and compare everything observable.
fn cmp_match(
    p: &Pair2,
    subj: &[u8],
    len: Sz,
    start: Sz,
    mopts: u32,
    mctx: (*mut Ctx, *mut Ctx),
    oveccount: u32,
    note: &str,
) {
    let (c, r) = pair();
    if !unsafe { subject_domain_is_defined(c, p.cc, &subj[..len.min(subj.len())], start, mopts) } {
        return; // outside the defined domain of PCRE2_NO_UTF_CHECK
    }
    if let Some(f) = std::env::var_os("DIFF_TRACE") {
        use std::io::Write;
        let mut fh = std::fs::OpenOptions::new()
            .create(true).write(true).truncate(true).open(f).unwrap();
        writeln!(fh, "match [{note}] subj={subj:02x?} len={len} start={start} mopts=0x{mopts:08x} ovec={oveccount}").unwrap();
        fh.sync_all().unwrap();
    }
    // Always hand PCRE2 a heap buffer with readable padding: the C library may
    // touch bytes at/after `end_subject`, so a zero-length static slice
    // (`b"".as_ptr()`) is not a legal subject pointer.
    let mut pad = subj.to_vec();
    pad.extend_from_slice(&[0u8; 16]);
    let subj_ptr = pad.as_ptr();
    let cmd = unsafe { (c.match_data_create)(oveccount, std::ptr::null_mut()) };
    let rmd = unsafe { (r.match_data_create)(oveccount, std::ptr::null_mut()) };
    assert!(!cmd.is_null() && !rmd.is_null());
    let crc = unsafe { (c.pcre2_match)(p.cc, subj_ptr, len, start, mopts, cmd, mctx.0) };
    let rrc = unsafe { (r.pcre2_match)(p.rc, subj_ptr, len, start, mopts, rmd, mctx.1) };
    let cd = unsafe { md_dump(c, cmd, crc) };
    let rd = unsafe { md_dump(r, rmd, rrc) };
    assert_eq!(
        cd, rd,
        "pcre2_match divergence [{note}]\n  subject = {:?} raw {subj:02x?}\n  len={len} start={start} mopts=0x{mopts:08x} ovec={oveccount}",
        String::from_utf8_lossy(subj)
    );
    // pcre2_next_match is a pure function of match_data: call it once and
    // compare (the proper iteration loop is row106_global_iteration).
    {
        let mut cs: Sz = 0xDEAD;
        let mut ce: u32 = 0xDEAD;
        let mut rs: Sz = 0xDEAD;
        let mut re: u32 = 0xDEAD;
        let ci = unsafe { (c.next_match)(cmd, &mut cs, &mut ce) };
        let ri = unsafe { (r.next_match)(rmd, &mut rs, &mut re) };
        assert_eq!(ci, ri, "next_match rc divergence [{note}]");
        if ci != 0 {
            assert_eq!((cs, ce), (rs, re), "next_match out divergence [{note}]");
        }
    }
    unsafe { (c.match_data_free)(cmd) };
    unsafe { (r.match_data_free)(rmd) };
}

fn cmp_dfa(
    p: &Pair2,
    subj: &[u8],
    len: Sz,
    start: Sz,
    mopts: u32,
    mctx: (*mut Ctx, *mut Ctx),
    oveccount: u32,
    wscount: usize,
    note: &str,
) {
    let (c, r) = pair();
    if !unsafe { subject_domain_is_defined(c, p.cc, &subj[..len.min(subj.len())], start, mopts) } {
        return; // outside the defined domain of PCRE2_NO_UTF_CHECK
    }
    let mut pad = subj.to_vec();
    pad.extend_from_slice(&[0u8; 16]);
    let subj_ptr = pad.as_ptr();
    let cmd = unsafe { (c.match_data_create)(oveccount, std::ptr::null_mut()) };
    let rmd = unsafe { (r.match_data_create)(oveccount, std::ptr::null_mut()) };
    let mut cws = vec![0 as c_int; wscount.max(1)];
    let mut rws = vec![0 as c_int; wscount.max(1)];
    let crc = unsafe {
        (c.dfa_match)(
            p.cc, subj_ptr, len, start, mopts, cmd, mctx.0, cws.as_mut_ptr(), wscount,
        )
    };
    let rrc = unsafe {
        (r.dfa_match)(
            p.rc, subj_ptr, len, start, mopts, rmd, mctx.1, rws.as_mut_ptr(), wscount,
        )
    };
    let cd = unsafe { md_dump(c, cmd, crc) };
    let rd = unsafe { md_dump(r, rmd, rrc) };
    assert_eq!(
        cd, rd,
        "pcre2_dfa_match divergence [{note}]\n  subject = {:?} raw {subj:02x?}\n  len={len} start={start} mopts=0x{mopts:08x} ws={wscount}",
        String::from_utf8_lossy(subj)
    );
    if crc == PCRE2_ERROR_PARTIAL {
        // Only after a partial match is the workspace a defined restart state.
        assert_eq!(cws, rws, "dfa workspace divergence [{note}]");
    }
    unsafe { (c.match_data_free)(cmd) };
    unsafe { (r.match_data_free)(rmd) };
}

/// Bound the runtime of pathological patterns without changing semantics: the
/// same limits are applied to both libraries, so results must still agree.
fn bounded_ctx() -> (*mut Ctx, *mut Ctx) {
    let (c, r) = pair();
    let cm = unsafe { (c.match_context_create)(std::ptr::null_mut()) };
    let rm = unsafe { (r.match_context_create)(std::ptr::null_mut()) };
    unsafe { (c.set_match_limit)(cm, 20000) };
    unsafe { (r.set_match_limit)(rm, 20000) };
    unsafe { (c.set_depth_limit)(cm, 2000) };
    unsafe { (r.set_depth_limit)(rm, 2000) };
    (cm, rm)
}

const MATCH_OPTS: &[(&str, u32)] = &[
    ("baseline", 0),
    ("ANCHORED", PCRE2_ANCHORED),
    ("ENDANCHORED", PCRE2_ENDANCHORED),
    ("NOTBOL", PCRE2_NOTBOL),
    ("NOTEOL", PCRE2_NOTEOL),
    ("NOTEMPTY", PCRE2_NOTEMPTY),
    ("NOTEMPTY_ATSTART", PCRE2_NOTEMPTY_ATSTART),
    ("PARTIAL_SOFT", PCRE2_PARTIAL_SOFT),
    ("PARTIAL_HARD", PCRE2_PARTIAL_HARD),
    ("NO_UTF_CHECK", PCRE2_NO_UTF_CHECK),
    ("COPY_MATCHED_SUBJECT", PCRE2_COPY_MATCHED_SUBJECT),
    ("NO_JIT", PCRE2_NO_JIT),
];

fn all_subjects() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
    v.extend(RAW_SUBJECTS.iter().map(|s| s.to_vec()));
    v
}

// ------------------------------------------- rows 85-96: each match option ---

#[test]
fn rows85_96_each_match_option() {
    let ctx = bounded_ctx();
    let subs = all_subjects();
    for &(name, mopts) in MATCH_OPTS {
        for copts in [0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS] {
            for p in PATTERNS {
                let Some(cp) = compile_pair(copts, p) else { continue };
                for s in &subs {
                    cmp_match(&cp, s, s.len(), 0, mopts, ctx, 8, &format!("{name} copts=0x{copts:x} pat={p:?}"));
                }
            }
        }
    }
}

// ---------------------------------------- row 97: random option combinations --

#[test]
fn row97_random_match_option_combinations() {
    let ctx = bounded_ctx();
    let mut rng = Rng::new(SEED ^ 97);
    let subs = all_subjects();
    for _ in 0..iters(250_000) {
        let mut mopts = 0u32;
        for _ in 0..rng.below(4) {
            mopts |= rng.pick(MATCH_OPTS).1;
        }
        let copts = *rng.pick(&[0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS, PCRE2_MULTILINE, PCRE2_DOTALL, PCRE2_UNGREEDY, PCRE2_MATCH_INVALID_UTF]);
        let p = if rng.bool() {
            rng.pick(PATTERNS).to_string()
        } else {
            random_pattern(&mut rng)
        };
        let Some(cp) = compile_pair(copts, &p) else { continue };
        let s = if rng.bool() {
            rng.pick(&subs).clone()
        } else {
            random_subject(&mut rng)
        };
        let ovec = *rng.pick(&[1u32, 2, 3, 8, 32]);
        cmp_match(&cp, &s, s.len(), 0, mopts, ctx, ovec, &format!("random mopts=0x{mopts:x} copts=0x{copts:x} pat={p:?}"));
    }
}

// ------------------------------------------------ row 98: start_offset sweep --

#[test]
fn row98_start_offset_sweep() {
    let ctx = bounded_ctx();
    let subs = all_subjects();
    for p in PATTERNS.iter().take(200) {
        for copts in [0u32, PCRE2_UTF] {
            let Some(cp) = compile_pair(copts, p) else { continue };
            for s in &subs {
                for start in 0..=s.len() {
                    cmp_match(&cp, s, s.len(), start, 0, ctx, 8, &format!("start sweep pat={p:?} copts=0x{copts:x}"));
                }
                // one past the end (ERRORS row 48 / B18)
                cmp_match(&cp, s, s.len(), s.len() + 1, 0, ctx, 8, "start > length");
                cmp_match(&cp, s, s.len(), Sz::MAX, 0, ctx, 8, "start = SIZE_MAX");
            }
        }
    }
}

// ---------------------------------------------- rows 99-101: context limits ---

#[test]
fn rows99_101_limits() {
    let (c, r) = pair();
    let cm = unsafe { (c.match_context_create)(std::ptr::null_mut()) };
    let rm = unsafe { (r.match_context_create)(std::ptr::null_mut()) };
    let subs = all_subjects();
    let hard: &[&str] = &[
        "(a+)+b", "(a|aa)+b", "(?:a?){20}b", "a*a*a*a*b", "(?R)?a", "(a)(?1)*b",
        "\\b(\\w+)\\s+\\1\\b", "(?:(?:(?:a)*)*)*b",
    ];
    for lim in [0u32, 1, 2, 10, 100, 1000, 10000] {
        for which in 0..3 {
            unsafe {
                (c.set_match_limit)(cm, u32::MAX);
                (r.set_match_limit)(rm, u32::MAX);
                (c.set_depth_limit)(cm, u32::MAX);
                (r.set_depth_limit)(rm, u32::MAX);
                (c.set_heap_limit)(cm, u32::MAX);
                (r.set_heap_limit)(rm, u32::MAX);
            }
            match which {
                0 => unsafe {
                    (c.set_match_limit)(cm, lim);
                    (r.set_match_limit)(rm, lim);
                },
                1 => unsafe {
                    (c.set_depth_limit)(cm, lim);
                    (r.set_depth_limit)(rm, lim);
                },
                _ => unsafe {
                    (c.set_heap_limit)(cm, lim);
                    (r.set_heap_limit)(rm, lim);
                },
            }
            for p in hard.iter().chain(PATTERNS.iter().take(60)) {
                let Some(cp) = compile_pair(0, p) else { continue };
                for s in subs.iter().take(20) {
                    cmp_match(&cp, s, s.len(), 0, 0, (cm, rm), 8, &format!("limit which={which} lim={lim} pat={p:?}"));
                }
            }
        }
    }
    unsafe { (c.match_context_free)(cm) };
    unsafe { (r.match_context_free)(rm) };
}

// ------------------------------------------------- row 102: offset limit ------

#[test]
fn row102_offset_limit() {
    let (c, r) = pair();
    let cm = unsafe { (c.match_context_create)(std::ptr::null_mut()) };
    let rm = unsafe { (r.match_context_create)(std::ptr::null_mut()) };
    let subs = all_subjects();
    for p in PATTERNS.iter().take(120) {
        for copts in [PCRE2_USE_OFFSET_LIMIT, 0] {
            let Some(cp) = compile_pair(copts, p) else { continue };
            for s in &subs {
                for lim in [0usize, 1, s.len() / 2, s.len(), s.len() + 1, PCRE2_UNSET] {
                    unsafe { (c.set_offset_limit)(cm, lim) };
                    unsafe { (r.set_offset_limit)(rm, lim) };
                    for mopts in [0u32, PCRE2_USE_OFFSET_LIMIT] {
                        cmp_match(&cp, s, s.len(), 0, mopts, (cm, rm), 8, &format!("offset_limit={lim} copts=0x{copts:x} mopts=0x{mopts:x} pat={p:?}"));
                    }
                }
            }
        }
    }
    unsafe { (c.match_context_free)(cm) };
    unsafe { (r.match_context_free)(rm) };
}

// ------------------------------------------------- row 103: match callouts ----

#[repr(C)]
struct CalloutBlock {
    version: u32,
    callout_number: u32,
    capture_top: u32,
    capture_last: u32,
    offset_vector: *mut Sz,
    mark: *const u8,
    subject: *const u8,
    subject_length: Sz,
    start_match: Sz,
    current_position: Sz,
    pattern_position: Sz,
    next_item_length: Sz,
    callout_string_offset: Sz,
    callout_string_length: Sz,
    callout_string: *const u8,
    callout_flags: u32,
}

static CALLOUT_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CALLOUT_RET: Mutex<c_int> = Mutex::new(0);

extern "C" fn callout_cb(b: *mut c_void, _u: *mut c_void) -> c_int {
    let b = unsafe { &*(b as *const CalloutBlock) };
    let ov = unsafe { std::slice::from_raw_parts(b.offset_vector, (b.capture_top as usize) * 2) };
    let mark = if b.mark.is_null() {
        "null".into()
    } else {
        let (c, _) = pair();
        let n = unsafe { (c.priv_strlen)(b.mark) };
        format!("{:02x?}", unsafe { std::slice::from_raw_parts(b.mark, n) })
    };
    let cs = if b.callout_string.is_null() {
        "null".into()
    } else {
        format!("{:02x?}", unsafe {
            std::slice::from_raw_parts(b.callout_string, b.callout_string_length)
        })
    };
    CALLOUT_LOG.lock().unwrap().push(format!(
        "v={} n={} ct={} cl={} sl={} sm={} cp={} pp={} nil={} cso={} csl={} cs={} fl={} mark={mark} ov={ov:?}",
        b.version, b.callout_number, b.capture_top, b.capture_last, b.subject_length,
        b.start_match, b.current_position, b.pattern_position, b.next_item_length,
        b.callout_string_offset, b.callout_string_length, cs, b.callout_flags
    ));
    *CALLOUT_RET.lock().unwrap()
}

#[test]
fn row103_match_callouts() {
    let (c, r) = pair();
    let cm = unsafe { (c.match_context_create)(std::ptr::null_mut()) };
    let rm = unsafe { (r.match_context_create)(std::ptr::null_mut()) };
    unsafe { (c.set_callout)(cm, Some(callout_cb), std::ptr::null_mut()) };
    unsafe { (r.set_callout)(rm, Some(callout_cb), std::ptr::null_mut()) };
    let pats: &[&str] = &[
        "(?C1)a", "a(?C1)b(?C2)c", "(?C)a+", "(?C`s`)a", "(?C'str')x", "a(?C255)b",
        "(?C1)(a)(b)", "(?C1)a|(?C2)b", "(*MARK:m)(?C1)a", "(?C1)(?<n>a)",
    ];
    let subs = all_subjects();
    for ret in [0 as c_int, 1, -1, 2, -2] {
        *CALLOUT_RET.lock().unwrap() = ret;
        for copts in [0u32, PCRE2_AUTO_CALLOUT, PCRE2_UTF] {
            for p in pats.iter().chain(PATTERNS.iter().take(80)) {
                let Some(cp) = compile_pair(copts, p) else { continue };
                for s in subs.iter().take(24) {
                    if !unsafe { subject_domain_is_defined(c, cp.cc, s, 0, 0) } {
                        continue;
                    }
                    let mut sp = s.clone();
                    sp.extend_from_slice(&[0u8; 16]);
                    CALLOUT_LOG.lock().unwrap().clear();
                    let cmd = unsafe { (c.match_data_create)(8, std::ptr::null_mut()) };
                    let crc = unsafe {
                        (c.pcre2_match)(cp.cc, sp.as_ptr(), s.len(), 0, 0, cmd, cm)
                    };
                    let cd = unsafe { md_dump(c, cmd, crc) };
                    let clog = std::mem::take(&mut *CALLOUT_LOG.lock().unwrap());
                    let rmd = unsafe { (r.match_data_create)(8, std::ptr::null_mut()) };
                    let rrc = unsafe {
                        (r.pcre2_match)(cp.rc, sp.as_ptr(), s.len(), 0, 0, rmd, rm)
                    };
                    let rd = unsafe { md_dump(r, rmd, rrc) };
                    let rlog = std::mem::take(&mut *CALLOUT_LOG.lock().unwrap());
                    assert_eq!(cd, rd, "callout match result ret={ret} pat={p:?} subj={s:02x?}");
                    assert_eq!(
                        clog, rlog,
                        "callout log ret={ret} copts=0x{copts:x} pat={p:?} subj={s:02x?}"
                    );
                    unsafe { (c.match_data_free)(cmd) };
                    unsafe { (r.match_data_free)(rmd) };
                }
            }
        }
    }
    unsafe { (c.match_context_free)(cm) };
    unsafe { (r.match_context_free)(rm) };
}

// -------------------------------------------------- row 104: marks ------------

#[test]
fn row104_marks() {
    let ctx = bounded_ctx();
    let pats: &[&str] = &[
        "(*MARK:one)a", "(*MARK:one)a|(*MARK:two)b", "a(*MARK:x)(*FAIL)|ab",
        "(*:m)a", "(*MARK:m)(*SKIP)a|b", "(*MARK:m)(*PRUNE)a|b",
        "(*MARK:m)(*THEN)a|b", "(*MARK:m)(*COMMIT)a|b", "(*MARK:\u{00e9})a",
        "a(*MARK:1)b(*MARK:2)c",
    ];
    let subs = all_subjects();
    for p in pats {
        for copts in [0u32, PCRE2_UTF, PCRE2_ALT_VERBNAMES] {
            let Some(cp) = compile_pair(copts, p) else { continue };
            for s in &subs {
                for mopts in [0u32, PCRE2_PARTIAL_SOFT, PCRE2_ANCHORED] {
                    cmp_match(&cp, s, s.len(), 0, mopts, ctx, 8, &format!("mark pat={p:?}"));
                }
            }
        }
    }
}

// --------------------------------------- rows 83-84: match data accessors -----

#[test]
fn rows83_84_match_data_accessors() {
    let (c, r) = pair();
    for n in [0u32, 1, 2, 3, 16, 100, 65535] {
        let cmd = unsafe { (c.match_data_create)(n, std::ptr::null_mut()) };
        let rmd = unsafe { (r.match_data_create)(n, std::ptr::null_mut()) };
        assert!(!cmd.is_null() && !rmd.is_null(), "match_data_create({n})");
        assert_eq!(
            unsafe { (c.get_ovector_count)(cmd) },
            unsafe { (r.get_ovector_count)(rmd) },
            "ovector_count({n})"
        );
        assert_eq!(
            unsafe { (c.get_match_data_size)(cmd) },
            unsafe { (r.get_match_data_size)(rmd) },
            "match_data_size({n})"
        );
        assert_eq!(
            unsafe { (c.get_match_data_heapframes_size)(cmd) },
            unsafe { (r.get_match_data_heapframes_size)(rmd) },
            "heapframes_size({n})"
        );
        // NOTE: `mark` is not initialised by pcre2_match_data_create in the C
        // code, so it is not compared on a fresh match_data.
        unsafe { (c.match_data_free)(cmd) };
        unsafe { (r.match_data_free)(rmd) };
    }
    for p in PATTERNS {
        for copts in [0u32, PCRE2_UTF, PCRE2_NO_AUTO_CAPTURE] {
            let Some(cp) = compile_pair(copts, p) else { continue };
            let cmd = unsafe { (c.match_data_create_from_pattern)(cp.cc, std::ptr::null_mut()) };
            let rmd = unsafe { (r.match_data_create_from_pattern)(cp.rc, std::ptr::null_mut()) };
            assert!(!cmd.is_null() && !rmd.is_null());
            assert_eq!(
                unsafe { (c.get_ovector_count)(cmd) },
                unsafe { (r.get_ovector_count)(rmd) },
                "from_pattern ovector_count {p:?}"
            );
            assert_eq!(
                unsafe { (c.get_match_data_size)(cmd) },
                unsafe { (r.get_match_data_size)(rmd) },
                "from_pattern size {p:?}"
            );
            unsafe { (c.match_data_free)(cmd) };
            unsafe { (r.match_data_free)(rmd) };
        }
    }
}

// ------------------------------------------------- rows 107-114: DFA ---------

const DFA_OPTS: &[(&str, u32)] = &[
    ("baseline", 0),
    ("SHORTEST", PCRE2_DFA_SHORTEST),
    ("PARTIAL_SOFT", PCRE2_PARTIAL_SOFT),
    ("PARTIAL_HARD", PCRE2_PARTIAL_HARD),
    ("ANCHORED", PCRE2_ANCHORED),
    ("ENDANCHORED", PCRE2_ENDANCHORED),
    ("NOTBOL", PCRE2_NOTBOL),
    ("NOTEOL", PCRE2_NOTEOL),
    ("NOTEMPTY", PCRE2_NOTEMPTY),
    ("NOTEMPTY_ATSTART", PCRE2_NOTEMPTY_ATSTART),
    ("COPY_MATCHED_SUBJECT", PCRE2_COPY_MATCHED_SUBJECT),
    ("NO_UTF_CHECK", PCRE2_NO_UTF_CHECK),
];

#[test]
fn rows107_112_dfa_options() {
    let ctx = bounded_ctx();
    let subs = all_subjects();
    for &(name, mopts) in DFA_OPTS {
        for copts in [0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS] {
            for p in PATTERNS {
                let Some(cp) = compile_pair(copts, p) else { continue };
                for s in &subs {
                    for ws in [20usize, 64, 1000] {
                        cmp_dfa(&cp, s, s.len(), 0, mopts, ctx, 8, ws, &format!("dfa {name} copts=0x{copts:x} pat={p:?}"));
                    }
                }
            }
        }
    }
}

#[test]
fn row109_dfa_restart() {
    // Proper two-call sequence: PARTIAL_HARD then DFA_RESTART on the rest.
    let (c, r) = pair();
    let ctx = bounded_ctx();
    let pats: &[&str] = &["abcd", "ab+cd", "^abc", "(a)(b)(c)", "a.c", "\\d{4}", "x|abc"];
    for p in pats {
        let Some(cp) = compile_pair(0, p) else { continue };
        for split in 1..4usize {
            let full: Vec<u8> = b"abcd".to_vec();
            let (h0, t0) = full.split_at(split.min(full.len()));
            let mut h = h0.to_vec();
            let hlen = h.len();
            h.extend_from_slice(&[0u8; 16]);
            let mut t = t0.to_vec();
            let tlen = t.len();
            t.extend_from_slice(&[0u8; 16]);
            let mut cws = vec![0 as c_int; 1000];
            let mut rws = vec![0 as c_int; 1000];
            let cmd = unsafe { (c.match_data_create)(8, std::ptr::null_mut()) };
            let rmd = unsafe { (r.match_data_create)(8, std::ptr::null_mut()) };
            let c1 = unsafe {
                (c.dfa_match)(cp.cc, h.as_ptr(), hlen, 0, PCRE2_PARTIAL_HARD, cmd, ctx.0, cws.as_mut_ptr(), 1000)
            };
            let r1 = unsafe {
                (r.dfa_match)(cp.rc, h.as_ptr(), hlen, 0, PCRE2_PARTIAL_HARD, rmd, ctx.1, rws.as_mut_ptr(), 1000)
            };
            assert_eq!(unsafe { md_dump(c, cmd, c1) }, unsafe { md_dump(r, rmd, r1) }, "dfa part1 {p:?}");
            if c1 == PCRE2_ERROR_PARTIAL {
                assert_eq!(cws, rws, "dfa workspace after part1 {p:?}");
                let c2 = unsafe {
                    (c.dfa_match)(cp.cc, t.as_ptr(), tlen, 0, PCRE2_DFA_RESTART, cmd, ctx.0, cws.as_mut_ptr(), 1000)
                };
                let r2 = unsafe {
                    (r.dfa_match)(cp.rc, t.as_ptr(), tlen, 0, PCRE2_DFA_RESTART, rmd, ctx.1, rws.as_mut_ptr(), 1000)
                };
                assert_eq!(unsafe { md_dump(c, cmd, c2) }, unsafe { md_dump(r, rmd, r2) }, "dfa restart {p:?}");
                if c2 == PCRE2_ERROR_PARTIAL {
                    assert_eq!(cws, rws, "dfa workspace after restart {p:?}");
                }
            }
            unsafe { (c.match_data_free)(cmd) };
            unsafe { (r.match_data_free)(rmd) };
        }
    }
}

#[test]
fn row113_dfa_limits_and_offset_limit() {
    let (c, r) = pair();
    let cm = unsafe { (c.match_context_create)(std::ptr::null_mut()) };
    let rm = unsafe { (r.match_context_create)(std::ptr::null_mut()) };
    let subs = all_subjects();
    for lim in [0u32, 1, 10, 1000] {
        unsafe {
            (c.set_match_limit)(cm, lim);
            (r.set_match_limit)(rm, lim);
            (c.set_depth_limit)(cm, lim);
            (r.set_depth_limit)(rm, lim);
            (c.set_heap_limit)(cm, lim);
            (r.set_heap_limit)(rm, lim);
        }
        for p in PATTERNS.iter().take(80) {
            let Some(cp) = compile_pair(0, p) else { continue };
            for s in subs.iter().take(16) {
                cmp_dfa(&cp, s, s.len(), 0, 0, (cm, rm), 8, 64, &format!("dfa lim={lim} pat={p:?}"));
            }
        }
    }
    // offset limit
    unsafe {
        (c.set_match_limit)(cm, u32::MAX);
        (r.set_match_limit)(rm, u32::MAX);
        (c.set_depth_limit)(cm, u32::MAX);
        (r.set_depth_limit)(rm, u32::MAX);
        (c.set_heap_limit)(cm, u32::MAX);
        (r.set_heap_limit)(rm, u32::MAX);
    }
    for p in PATTERNS.iter().take(80) {
        for copts in [PCRE2_USE_OFFSET_LIMIT, 0] {
            let Some(cp) = compile_pair(copts, p) else { continue };
            for s in subs.iter().take(16) {
                for lim in [0usize, 1, s.len(), PCRE2_UNSET] {
                    unsafe { (c.set_offset_limit)(cm, lim) };
                    unsafe { (r.set_offset_limit)(rm, lim) };
                    for mopts in [0u32, PCRE2_USE_OFFSET_LIMIT] {
                        cmp_dfa(&cp, s, s.len(), 0, mopts, (cm, rm), 8, 64, &format!("dfa offlim={lim} pat={p:?}"));
                    }
                }
            }
        }
    }
    unsafe { (c.match_context_free)(cm) };
    unsafe { (r.match_context_free)(rm) };
}

#[test]
fn row114_dfa_random() {
    let ctx = bounded_ctx();
    let mut rng = Rng::new(SEED ^ 114);
    let subs = all_subjects();
    for _ in 0..iters(150_000) {
        let mut mopts = 0u32;
        for _ in 0..rng.below(3) {
            mopts |= rng.pick(DFA_OPTS).1;
        }
        let copts = *rng.pick(&[0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS, PCRE2_MULTILINE, PCRE2_DOTALL]);
        let p = if rng.bool() {
            rng.pick(PATTERNS).to_string()
        } else {
            random_pattern(&mut rng)
        };
        let Some(cp) = compile_pair(copts, &p) else { continue };
        let s = if rng.bool() {
            rng.pick(&subs).clone()
        } else {
            random_subject(&mut rng)
        };
        let ws = *rng.pick(&[20usize, 21, 40, 100, 1000]);
        let ovec = *rng.pick(&[1u32, 2, 8, 32]);
        cmp_dfa(&cp, &s, s.len(), 0, mopts, ctx, ovec, ws, &format!("dfa random mopts=0x{mopts:x} pat={p:?}"));
    }
}

// ------------------------------- row 106: full global-match iteration loop ----

/// Drive the documented `pcre2_match` + `pcre2_next_match` iteration to
/// exhaustion in both libraries and compare the whole sequence of matches.
#[test]
fn row106_global_iteration() {
    let (c, r) = pair();
    let ctx = bounded_ctx();
    let subs = all_subjects();
    for p in PATTERNS.iter() {
        for copts in [0u32, PCRE2_UTF, PCRE2_MULTILINE, PCRE2_CASELESS] {
            let Some(cp) = compile_pair(copts, p) else { continue };
            for s in &subs {
                if !unsafe { subject_domain_is_defined(c, cp.cc, s, 0, 0) } {
                    continue;
                }
                let mut sp = s.clone();
                sp.extend_from_slice(&[0u8; 16]);
                let mut clog = String::new();
                let mut rlog = String::new();
                for (a, code, log) in
                    [(c, cp.cc, &mut clog), (r, cp.rc, &mut rlog)]
                {
                    let md = unsafe { (a.match_data_create)(8, std::ptr::null_mut()) };
                    let mctx = if std::ptr::eq(a, c) { ctx.0 } else { ctx.1 };
                    let mut start: Sz = 0;
                    let mut opts: u32 = 0;
                    for _ in 0..64 {
                        let rc = unsafe {
                            (a.pcre2_match)(code, sp.as_ptr(), s.len(), start, opts, md, mctx)
                        };
                        log.push_str(&unsafe { md_dump(a, md, rc) });
                        log.push('|');
                        let mut ns: Sz = 0;
                        let mut no: u32 = 0;
                        if unsafe { (a.next_match)(md, &mut ns, &mut no) } == 0 {
                            break;
                        }
                        log.push_str(&format!("next({ns},{no})|"));
                        start = ns;
                        opts = no;
                    }
                    unsafe { (a.match_data_free)(md) };
                }
                assert_eq!(
                    clog, rlog,
                    "global iteration divergence pat={p:?} copts=0x{copts:x} subj={s:02x?}"
                );
            }
        }
    }
}

// -------------------------------------------- row 149: jit stub entry points --

#[test]
fn row149_jit_stubs() {
    let (c, r) = pair();
    for p in PATTERNS.iter().take(60) {
        let Some(cp) = compile_pair(0, p) else { continue };
        for opts in [0u32, 1, 2, 4, 6, 7, 8, 0x10, u32::MAX] {
            assert_eq!(
                unsafe { (c.jit_compile)(cp.cc, opts) },
                unsafe { (r.jit_compile)(cp.rc, opts) },
                "jit_compile(0x{opts:x}) {p:?}"
            );
        }
        let cmd = unsafe { (c.match_data_create)(8, std::ptr::null_mut()) };
        let rmd = unsafe { (r.match_data_create)(8, std::ptr::null_mut()) };
        let s: Vec<u8> = b"abc\0\0\0\0\0\0\0\0".to_vec();
        assert_eq!(
            unsafe { (c.jit_match)(cp.cc, s.as_ptr(), 3, 0, 0, cmd, std::ptr::null_mut()) },
            unsafe { (r.jit_match)(cp.rc, s.as_ptr(), 3, 0, 0, rmd, std::ptr::null_mut()) },
            "jit_match {p:?}"
        );
        unsafe { (c.match_data_free)(cmd) };
        unsafe { (r.match_data_free)(rmd) };
    }
    // stack functions
    for (a, b) in [(1usize, 1usize), (0, 0), (1024, 1024 * 1024), (1 << 20, 1 << 10)] {
        let cs = unsafe { (c.jit_stack_create)(a, b, std::ptr::null_mut()) };
        let rs = unsafe { (r.jit_stack_create)(a, b, std::ptr::null_mut()) };
        assert_eq!(cs.is_null(), rs.is_null(), "jit_stack_create({a},{b})");
        unsafe { (c.jit_stack_free)(cs) };
        unsafe { (r.jit_stack_free)(rs) };
    }
    unsafe { (c.jit_stack_free)(std::ptr::null_mut()) };
    unsafe { (r.jit_stack_free)(std::ptr::null_mut()) };
    unsafe { (c.jit_free_unused_memory)(std::ptr::null_mut()) };
    unsafe { (r.jit_free_unused_memory)(std::ptr::null_mut()) };
    let cm = unsafe { (c.match_context_create)(std::ptr::null_mut()) };
    let rm = unsafe { (r.match_context_create)(std::ptr::null_mut()) };
    unsafe { (c.jit_stack_assign)(cm, None, std::ptr::null_mut()) };
    unsafe { (r.jit_stack_assign)(rm, None, std::ptr::null_mut()) };
    unsafe { (c.match_context_free)(cm) };
    unsafe { (r.match_context_free)(rm) };
}
