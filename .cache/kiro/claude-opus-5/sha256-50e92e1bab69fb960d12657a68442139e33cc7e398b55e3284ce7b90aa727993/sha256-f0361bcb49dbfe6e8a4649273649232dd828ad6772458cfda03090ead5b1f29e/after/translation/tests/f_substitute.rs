//! CONFIGS.md rows 123–135 — `pcre2_substitute` under every option combination,
//! including the substitute callout and the substitute *case* callout.
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
fn pad(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.extend_from_slice(&[0u8; 16]);
    v
}

const SUB_PATTERNS: &[&str] = &[
    "a", "a+", "(a)", "(a)(b)", "(?<x>a)", "(?<x>a)(?<y>b)", "\\b\\w+\\b", "", "a*",
    "(a)?(b)?", "x|y", "\\d+", "(\\d)(\\d)", "[aeiou]", "(?i)abc", "(?<n>.)\\k<n>",
    "(?J)(?<n>a)|(?<n>b)", "^", "$", "\\s+", "(.)", "(.)(.)(.)", "(?:a)", "\\R",
    "(*UTF)\\X", "(?<A>a)(?<B>b)(?<C>c)",
];

const REPLACEMENTS: &[&str] = &[
    "", "X", "[$0]", "[$1]", "[${1}]", "[$2]", "[${2}]", "$1$2", "${x}", "${y}", "${n}",
    "$x", "$*", "$", "$$", "${1", "${", "$99", "${99}", "$1$", "\\n", "\\t", "\\x41",
    "\\U$1\\E", "\\L$1\\E", "\\u$1", "\\l$1", "\\U$0", "${1:-def}", "${1:+yes:no}",
    "${x:-d}", "${x:+a:b}", "${1:?}", "\\q", "\\", "\\\\", "a\\Ub\\Ec", "$0$0$0",
    "\u{00e9}", "\u{4e2d}$0",
];

const SUB_OPTS: &[(&str, u32)] = &[
    ("none", 0),
    ("GLOBAL", PCRE2_SUBSTITUTE_GLOBAL),
    ("EXTENDED", PCRE2_SUBSTITUTE_EXTENDED),
    ("UNSET_EMPTY", PCRE2_SUBSTITUTE_UNSET_EMPTY),
    ("UNKNOWN_UNSET", PCRE2_SUBSTITUTE_UNKNOWN_UNSET),
    ("OVERFLOW_LENGTH", PCRE2_SUBSTITUTE_OVERFLOW_LENGTH),
    ("LITERAL", PCRE2_SUBSTITUTE_LITERAL),
    ("REPLACEMENT_ONLY", PCRE2_SUBSTITUTE_REPLACEMENT_ONLY),
    ("NOTBOL", PCRE2_NOTBOL),
    ("NOTEOL", PCRE2_NOTEOL),
    ("NOTEMPTY", PCRE2_NOTEMPTY),
    ("NOTEMPTY_ATSTART", PCRE2_NOTEMPTY_ATSTART),
    ("ANCHORED", PCRE2_ANCHORED),
    ("ENDANCHORED", PCRE2_ENDANCHORED),
    ("NO_JIT", PCRE2_NO_JIT),
];

/// Run substitute in both libraries and compare rc, output length and bytes.
fn cmp_sub(
    cc: *const Code,
    rc_: *const Code,
    subj: &[u8],
    start: Sz,
    opts: u32,
    repl: &[u8],
    outcap: usize,
    mctx: (*mut Ctx, *mut Ctx),
    note: &str,
) {
    let (c, r) = pair();
    if !unsafe { subject_domain_is_defined(c, cc, subj, start, opts) } {
        return;
    }
    let sp = pad(subj);
    let rp = pad(repl);
    let mut cbuf = vec![0xAAu8; outcap + 8];
    let mut rbuf = vec![0xAAu8; outcap + 8];
    let mut clen: Sz = outcap;
    let mut rlen: Sz = outcap;
    let crc = unsafe {
        (c.substitute)(
            cc, sp.as_ptr(), subj.len(), start, opts, std::ptr::null_mut(), mctx.0,
            rp.as_ptr(), repl.len(), cbuf.as_mut_ptr(), &mut clen,
        )
    };
    let rrc = unsafe {
        (r.substitute)(
            rc_, sp.as_ptr(), subj.len(), start, opts, std::ptr::null_mut(), mctx.1,
            rp.as_ptr(), repl.len(), rbuf.as_mut_ptr(), &mut rlen,
        )
    };
    assert_eq!(
        (crc, clen),
        (rrc, rlen),
        "substitute rc/len divergence [{note}]\n  subj={subj:02x?} repl={:?} opts=0x{opts:08x} outcap={outcap}",
        String::from_utf8_lossy(repl)
    );
    if crc >= 0 {
        assert_eq!(
            &cbuf[..clen.min(cbuf.len())],
            &rbuf[..rlen.min(rbuf.len())],
            "substitute output divergence [{note}]\n  subj={subj:02x?} repl={:?} opts=0x{opts:08x}",
            String::from_utf8_lossy(repl)
        );
    }
}

fn compile2(p: &str, copts: u32) -> Option<(*mut Code, *mut Code)> {
    let (c, r) = pair();
    let v = zpat(p);
    let mut ce: c_int = 0;
    let mut co: Sz = 0;
    let mut re: c_int = 0;
    let mut ro: Sz = 0;
    let cc = unsafe {
        (c.compile)(v.as_ptr(), p.len(), copts, &mut ce, &mut co, std::ptr::null_mut())
    };
    let rr = unsafe {
        (r.compile)(v.as_ptr(), p.len(), copts, &mut re, &mut ro, std::ptr::null_mut())
    };
    assert_eq!((ce, co, cc.is_null()), (re, ro, rr.is_null()), "compile {p:?}");
    if cc.is_null() { None } else { Some((cc, rr)) }
}

#[test]
fn rows123_131_each_substitute_option() {
    let (c, r) = pair();
    let mut subs: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
    subs.extend(RAW_SUBJECTS.iter().map(|s| s.to_vec()));
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    for &(name, opts) in SUB_OPTS {
        for copts in [0u32, PCRE2_UTF, PCRE2_CASELESS] {
            for p in SUB_PATTERNS {
                let Some((cc, rr)) = compile2(p, copts) else { continue };
                for s in &subs {
                    for repl in REPLACEMENTS {
                        for outcap in [0usize, 1, 4, 32, 256] {
                            cmp_sub(
                                cc, rr, s, 0, opts, repl.as_bytes(), outcap, null_ctx,
                                &format!("{name} copts=0x{copts:x} pat={p:?}"),
                            );
                        }
                    }
                }
                unsafe { (c.code_free)(cc) };
                unsafe { (r.code_free)(rr) };
            }
        }
    }
}

#[test]
fn row134_random_substitute_combinations() {
    let (c, r) = pair();
    let mut rng = Rng::new(SEED ^ 134);
    let mut subs: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
    subs.extend(RAW_SUBJECTS.iter().map(|s| s.to_vec()));
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    for _ in 0..iters(60000) {
        let mut opts = 0u32;
        for _ in 0..rng.below(4) {
            opts |= rng.pick(SUB_OPTS).1;
        }
        let copts = *rng.pick(&[0u32, PCRE2_UTF, PCRE2_CASELESS, PCRE2_MULTILINE, PCRE2_DUPNAMES, PCRE2_UTF | PCRE2_UCP]);
        let p = if rng.bool() {
            rng.pick(SUB_PATTERNS).to_string()
        } else {
            random_pattern(&mut rng)
        };
        let Some((cc, rr)) = compile2(&p, copts) else { continue };
        let s = if rng.bool() {
            rng.pick(&subs).clone()
        } else {
            random_subject(&mut rng)
        };
        let repl = rng.pick(REPLACEMENTS);
        let outcap = *rng.pick(&[0usize, 1, 2, 8, 64, 512]);
        let start = rng.below(s.len() + 3);
        cmp_sub(
            cc, rr, &s, start, opts, repl.as_bytes(), outcap, null_ctx,
            &format!("random opts=0x{opts:x} copts=0x{copts:x} pat={p:?}"),
        );
        unsafe { (c.code_free)(cc) };
        unsafe { (r.code_free)(rr) };
    }
}

#[test]
fn row135_output_length_probe() {
    let (c, r) = pair();
    let subs: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
    let null_ctx = (std::ptr::null_mut(), std::ptr::null_mut());
    for p in SUB_PATTERNS {
        let Some((cc, rr)) = compile2(p, 0) else { continue };
        for s in &subs {
            for repl in REPLACEMENTS.iter().take(20) {
                // first learn the needed length via OVERFLOW_LENGTH + tiny buffer
                let sp = pad(s);
                let rp = pad(repl.as_bytes());
                let mut buf = [0u8; 1];
                let mut need: Sz = 0;
                let rcx = unsafe {
                    (c.substitute)(
                        cc, sp.as_ptr(), s.len(), 0,
                        PCRE2_SUBSTITUTE_OVERFLOW_LENGTH | PCRE2_SUBSTITUTE_GLOBAL,
                        std::ptr::null_mut(), std::ptr::null_mut(), rp.as_ptr(),
                        repl.len(), buf.as_mut_ptr(), &mut need,
                    )
                };
                let _ = rcx;
                for cap in [
                    0usize,
                    need.saturating_sub(1),
                    need,
                    need + 1,
                    need + 100,
                ] {
                    for og in [0u32, PCRE2_SUBSTITUTE_OVERFLOW_LENGTH] {
                        cmp_sub(
                            cc, rr, s, 0, PCRE2_SUBSTITUTE_GLOBAL | og, repl.as_bytes(),
                            cap, null_ctx, &format!("len probe pat={p:?} need={need}"),
                        );
                    }
                }
            }
        }
        unsafe { (c.code_free)(cc) };
        unsafe { (r.code_free)(rr) };
    }
}

// -------------------------------------------- row 132: substitute callout ----

#[repr(C)]
struct SubCalloutBlock {
    version: u32,
    _pad: u32,
    input: *const u8,
    output: *const u8,
    output_offsets: [Sz; 2],
    ovector: *mut Sz,
    oveccount: u32,
    subscount: u32,
}

static SUB_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
static SUB_RET: Mutex<c_int> = Mutex::new(0);

extern "C" fn sub_cb(b: *mut c_void, _u: *mut c_void) -> c_int {
    let b = unsafe { &*(b as *const SubCalloutBlock) };
    let ov = unsafe { std::slice::from_raw_parts(b.ovector, (b.oveccount as usize) * 2) };
    let out = unsafe {
        std::slice::from_raw_parts(b.output.add(b.output_offsets[0]), b.output_offsets[1] - b.output_offsets[0])
    };
    SUB_LOG.lock().unwrap().push(format!(
        "v={} off={:?} n={} sub={} out={out:02x?} ov={ov:?}",
        b.version, b.output_offsets, b.oveccount, b.subscount
    ));
    *SUB_RET.lock().unwrap()
}

#[test]
fn row132_substitute_callout() {
    let (c, r) = pair();
    let cm = unsafe { (c.match_context_create)(std::ptr::null_mut()) };
    let rm = unsafe { (r.match_context_create)(std::ptr::null_mut()) };
    unsafe { (c.set_substitute_callout)(cm, Some(sub_cb), std::ptr::null_mut()) };
    unsafe { (r.set_substitute_callout)(rm, Some(sub_cb), std::ptr::null_mut()) };
    let subs: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
    for ret in [0 as c_int, 1, -1, 2] {
        *SUB_RET.lock().unwrap() = ret;
        for p in SUB_PATTERNS {
            let Some((cc, rr)) = compile2(p, 0) else { continue };
            for s in &subs {
                for repl in ["X", "[$0]", "$1", ""] {
                    for opts in [
                        0u32,
                        PCRE2_SUBSTITUTE_GLOBAL,
                        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                    ] {
                        let sp = pad(s);
                        let rp = pad(repl.as_bytes());
                        SUB_LOG.lock().unwrap().clear();
                        let mut cbuf = vec![0xAAu8; 264];
                        let mut clen: Sz = 256;
                        let crc = unsafe {
                            (c.substitute)(
                                cc, sp.as_ptr(), s.len(), 0, opts, std::ptr::null_mut(), cm,
                                rp.as_ptr(), repl.len(), cbuf.as_mut_ptr(), &mut clen,
                            )
                        };
                        let clog = std::mem::take(&mut *SUB_LOG.lock().unwrap());
                        let mut rbuf = vec![0xAAu8; 264];
                        let mut rlen: Sz = 256;
                        let rrc = unsafe {
                            (r.substitute)(
                                rr, sp.as_ptr(), s.len(), 0, opts, std::ptr::null_mut(), rm,
                                rp.as_ptr(), repl.len(), rbuf.as_mut_ptr(), &mut rlen,
                            )
                        };
                        let rlog = std::mem::take(&mut *SUB_LOG.lock().unwrap());
                        assert_eq!(
                            (crc, clen),
                            (rrc, rlen),
                            "sub callout rc ret={ret} pat={p:?} repl={repl:?} opts=0x{opts:x} subj={s:02x?}"
                        );
                        if crc >= 0 {
                            assert_eq!(
                                &cbuf[..clen], &rbuf[..rlen],
                                "sub callout out ret={ret} pat={p:?} repl={repl:?}"
                            );
                        }
                        assert_eq!(
                            clog, rlog,
                            "sub callout log ret={ret} pat={p:?} repl={repl:?} opts=0x{opts:x} subj={s:02x?}"
                        );
                    }
                }
            }
            unsafe { (c.code_free)(cc) };
            unsafe { (r.code_free)(rr) };
        }
    }
    unsafe { (c.match_context_free)(cm) };
    unsafe { (r.match_context_free)(rm) };
}

// --------------------------------------- row 133: substitute case callout ----

static CASE_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// 0 = behave like a simple ASCII case mapper, 1 = return 0, 2 = return a huge
/// length (probing PCRE2_ERROR_REPLACECASE), 3 = report a "needs bigger buffer".
static CASE_MODE: Mutex<u32> = Mutex::new(0);

extern "C" fn case_cb(
    input: *const u8,
    inlen: Sz,
    output: *mut u8,
    outlen: Sz,
    to_case: c_int,
    _data: *mut c_void,
) -> Sz {
    let inp = unsafe { std::slice::from_raw_parts(input, inlen) };
    CASE_LOG
        .lock()
        .unwrap()
        .push(format!("in={inp:02x?} outlen={outlen} case={to_case}"));
    match *CASE_MODE.lock().unwrap() {
        1 => 0,
        2 => Sz::MAX,
        3 => outlen + 1,
        _ => {
            let n = inlen.min(outlen);
            for i in 0..n {
                let b = inp[i];
                let m = match to_case {
                    1 => b.to_ascii_lowercase(),
                    2 => b.to_ascii_uppercase(),
                    3 => {
                        if i == 0 {
                            b.to_ascii_uppercase()
                        } else {
                            b
                        }
                    }
                    _ => b,
                };
                unsafe { *output.add(i) = m };
            }
            inlen
        }
    }
}

#[test]
fn row133_substitute_case_callout() {
    let (c, r) = pair();
    let cm = unsafe { (c.match_context_create)(std::ptr::null_mut()) };
    let rm = unsafe { (r.match_context_create)(std::ptr::null_mut()) };
    unsafe { (c.set_substitute_case_callout)(cm, Some(case_cb), std::ptr::null_mut()) };
    unsafe { (r.set_substitute_case_callout)(rm, Some(case_cb), std::ptr::null_mut()) };
    let subs: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
    let repls = [
        "\\U$0\\E", "\\L$0\\E", "\\u$0", "\\l$0", "\\U$1", "\\L$1", "\\u\\L$0",
        "a\\Ub\\Lc\\Ed", "\\U", "\\E",
    ];
    for mode in [0u32, 1, 2, 3] {
        *CASE_MODE.lock().unwrap() = mode;
        for p in SUB_PATTERNS {
            let Some((cc, rr)) = compile2(p, 0) else { continue };
            for s in &subs {
                for repl in repls {
                    for opts in [
                        PCRE2_SUBSTITUTE_EXTENDED,
                        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                    ] {
                        let sp = pad(s);
                        let rp = pad(repl.as_bytes());
                        CASE_LOG.lock().unwrap().clear();
                        let mut cbuf = vec![0xAAu8; 264];
                        let mut clen: Sz = 256;
                        let crc = unsafe {
                            (c.substitute)(
                                cc, sp.as_ptr(), s.len(), 0, opts, std::ptr::null_mut(), cm,
                                rp.as_ptr(), repl.len(), cbuf.as_mut_ptr(), &mut clen,
                            )
                        };
                        let clog = std::mem::take(&mut *CASE_LOG.lock().unwrap());
                        let mut rbuf = vec![0xAAu8; 264];
                        let mut rlen: Sz = 256;
                        let rrc = unsafe {
                            (r.substitute)(
                                rr, sp.as_ptr(), s.len(), 0, opts, std::ptr::null_mut(), rm,
                                rp.as_ptr(), repl.len(), rbuf.as_mut_ptr(), &mut rlen,
                            )
                        };
                        let rlog = std::mem::take(&mut *CASE_LOG.lock().unwrap());
                        assert_eq!(
                            (crc, clen),
                            (rrc, rlen),
                            "case callout rc mode={mode} pat={p:?} repl={repl:?} subj={s:02x?}"
                        );
                        if crc >= 0 {
                            assert_eq!(
                                &cbuf[..clen], &rbuf[..rlen],
                                "case callout out mode={mode} pat={p:?} repl={repl:?}"
                            );
                        }
                        assert_eq!(
                            clog, rlog,
                            "case callout log mode={mode} pat={p:?} repl={repl:?} subj={s:02x?}"
                        );
                    }
                }
            }
            unsafe { (c.code_free)(cc) };
            unsafe { (r.code_free)(rr) };
        }
    }
    // Turkish casing + UTF, exercising the built-in case handling too
    unsafe { (c.set_substitute_case_callout)(cm, None, std::ptr::null_mut()) };
    unsafe { (r.set_substitute_case_callout)(rm, None, std::ptr::null_mut()) };
    let cc_ctx = unsafe { (c.compile_context_create)(std::ptr::null_mut()) };
    let rc_ctx = unsafe { (r.compile_context_create)(std::ptr::null_mut()) };
    for xopt in [0u32, PCRE2_EXTRA_TURKISH_CASING, PCRE2_EXTRA_CASELESS_RESTRICT] {
        unsafe { (c.set_compile_extra_options)(cc_ctx, xopt) };
        unsafe { (r.set_compile_extra_options)(rc_ctx, xopt) };
        for p in ["(.)", "(.+)", "(\\w+)"] {
            let v = zpat(p);
            let mut e: c_int = 0;
            let mut o: Sz = 0;
            let cc = unsafe {
                (c.compile)(v.as_ptr(), p.len(), PCRE2_UTF | PCRE2_UCP, &mut e, &mut o, cc_ctx)
            };
            let rr = unsafe {
                (r.compile)(v.as_ptr(), p.len(), PCRE2_UTF | PCRE2_UCP, &mut e, &mut o, rc_ctx)
            };
            if cc.is_null() {
                continue;
            }
            for s in &subs {
                for repl in repls {
                    cmp_sub(
                        cc, rr, s, 0,
                        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                        repl.as_bytes(), 256, (cm, rm),
                        &format!("builtin case xopt=0x{xopt:x} pat={p:?}"),
                    );
                }
            }
            unsafe { (c.code_free)(cc) };
            unsafe { (r.code_free)(rr) };
        }
    }
    unsafe { (c.compile_context_free)(cc_ctx) };
    unsafe { (r.compile_context_free)(rc_ctx) };
    unsafe { (c.match_context_free)(cm) };
    unsafe { (r.match_context_free)(rm) };
}

// ------------------------------------------- row 130: SUBSTITUTE_MATCHED ------

#[test]
fn row130_substitute_matched() {
    let (c, r) = pair();
    let subs: Vec<Vec<u8>> = SUBJECTS.iter().map(|s| s.as_bytes().to_vec()).collect();
    for p in SUB_PATTERNS {
        let Some((cc, rr)) = compile2(p, 0) else { continue };
        for s in &subs {
            let sp = pad(s);
            for mopts in [0u32, PCRE2_ANCHORED, PCRE2_NOTBOL] {
                let cmd = unsafe { (c.match_data_create_from_pattern)(cc, std::ptr::null_mut()) };
                let rmd = unsafe { (r.match_data_create_from_pattern)(rr, std::ptr::null_mut()) };
                let cm0 = unsafe {
                    (c.pcre2_match)(cc, sp.as_ptr(), s.len(), 0, mopts, cmd, std::ptr::null_mut())
                };
                let rm0 = unsafe {
                    (r.pcre2_match)(rr, sp.as_ptr(), s.len(), 0, mopts, rmd, std::ptr::null_mut())
                };
                assert_eq!(
                    unsafe { md_dump(c, cmd, cm0) },
                    unsafe { md_dump(r, rmd, rm0) },
                    "pre-match {p:?} {s:02x?}"
                );
                for repl in ["X", "[$0]", "$1"] {
                    let rp = pad(repl.as_bytes());
                    for extra in [0u32, PCRE2_SUBSTITUTE_REPLACEMENT_ONLY] {
                        let opts = PCRE2_SUBSTITUTE_MATCHED | mopts | extra;
                        let mut cbuf = vec![0xAAu8; 264];
                        let mut rbuf = vec![0xAAu8; 264];
                        let mut clen: Sz = 256;
                        let mut rlen: Sz = 256;
                        let crc = unsafe {
                            (c.substitute)(
                                cc, sp.as_ptr(), s.len(), 0, opts, cmd, std::ptr::null_mut(),
                                rp.as_ptr(), repl.len(), cbuf.as_mut_ptr(), &mut clen,
                            )
                        };
                        let rrc = unsafe {
                            (r.substitute)(
                                rr, sp.as_ptr(), s.len(), 0, opts, rmd, std::ptr::null_mut(),
                                rp.as_ptr(), repl.len(), rbuf.as_mut_ptr(), &mut rlen,
                            )
                        };
                        assert_eq!(
                            (crc, clen), (rrc, rlen),
                            "SUBSTITUTE_MATCHED rc {p:?} {s:02x?} repl={repl:?} opts=0x{opts:x}"
                        );
                        if crc >= 0 {
                            assert_eq!(&cbuf[..clen], &rbuf[..rlen], "SUBSTITUTE_MATCHED out");
                        }
                    }
                }
                unsafe { (c.match_data_free)(cmd) };
                unsafe { (r.match_data_free)(rmd) };
            }
        }
        unsafe { (c.code_free)(cc) };
        unsafe { (r.code_free)(rr) };
    }
}
