//! Phase B/C: pcre2_substitute.
//! CONFIGS.md rows 65-75; ERRORS.md rows 58-78.
mod harness;
use harness::*;
use std::ffi::c_void;
use std::os::raw::c_int;

#[derive(Debug, PartialEq, Eq)]
struct SubOut {
    compile_err: c_int,
    rc: c_int,
    outlen: Sz,
    out: Vec<u8>,
    callouts: Vec<(u32, u32, Sz, Sz, Vec<Sz>)>,
    case_calls: Vec<(Vec<u8>, c_int)>,
}

thread_local! {
    static CALLOUTS: std::cell::RefCell<Vec<(u32, u32, Sz, Sz, Vec<Sz>)>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static CASECALLS: std::cell::RefCell<Vec<(Vec<u8>, c_int)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

unsafe extern "C" fn sub_callout(b: *mut SubstituteCalloutBlock, _d: *mut c_void) -> c_int {
    let b = unsafe { &*b };
    let ovec = unsafe { std::slice::from_raw_parts(b.ovector, (b.oveccount as usize) * 2) }.to_vec();
    CALLOUTS.with(|c| {
        c.borrow_mut().push((
            b.version,
            b.subscount,
            b.output_offsets[0],
            b.output_offsets[1],
            ovec,
        ))
    });
    0
}

/// Upper-cases ASCII; mirrors what a real caller would do, and records inputs.
unsafe extern "C" fn case_callout(
    input: *const u8,
    inlen: Sz,
    output: *mut u8,
    outcap: Sz,
    to_case: c_int,
    _d: *mut c_void,
) -> Sz {
    let src = unsafe { std::slice::from_raw_parts(input, inlen) };
    CASECALLS.with(|c| c.borrow_mut().push((src.to_vec(), to_case)));
    let mapped: Vec<u8> = src
        .iter()
        .map(|&b| match to_case {
            0 | 2 => b.to_ascii_lowercase(),
            _ => b.to_ascii_uppercase(),
        })
        .collect();
    if mapped.len() > outcap {
        return Sz::MAX; // signal "too large" like the doc's example
    }
    unsafe { std::ptr::copy_nonoverlapping(mapped.as_ptr(), output, mapped.len()) };
    mapped.len()
}

#[allow(clippy::too_many_arguments)]
fn run_sub(
    api: &Api,
    pat: &[u8],
    subject: &[u8],
    repl: &[u8],
    copts: u32,
    sopts: u32,
    start_offset: Sz,
    bufsize: usize,
    use_matched: bool,
    with_callout: bool,
    with_case_callout: bool,
    ovecsize: Option<u32>,
) -> SubOut {
    CALLOUTS.with(|c| c.borrow_mut().clear());
    CASECALLS.with(|c| c.borrow_mut().clear());
    unsafe {
        let mut err: c_int = 0;
        let mut off: Sz = 0;
        let code = (api.compile)(
            pat.as_ptr(),
            pat.len(),
            copts,
            &mut err,
            &mut off,
            std::ptr::null_mut(),
        );
        if code.is_null() {
            return SubOut {
                compile_err: err,
                rc: 0,
                outlen: 0,
                out: Vec::new(),
                callouts: Vec::new(),
                case_calls: Vec::new(),
            };
        }
        let mc = (api.match_context_create)(std::ptr::null_mut());
        if with_callout {
            (api.set_substitute_callout)(mc, Some(sub_callout), std::ptr::null_mut());
        }
        if with_case_callout {
            (api.set_substitute_case_callout)(mc, Some(case_callout), std::ptr::null_mut());
        }
        let md = if use_matched || ovecsize.is_some() {
            match ovecsize {
                Some(n) => (api.match_data_create)(n, std::ptr::null_mut()),
                None => (api.match_data_create_from_pattern)(code, std::ptr::null_mut()),
            }
        } else {
            std::ptr::null_mut()
        };
        if use_matched && !md.is_null() {
            (api.do_match)(
                code,
                subject.as_ptr(),
                subject.len(),
                start_offset,
                0,
                md,
                std::ptr::null_mut(),
            );
        }
        let mut buf = vec![0xaau8; bufsize.max(1)];
        let mut outlen: Sz = bufsize;
        let rc = (api.substitute)(
            code,
            subject.as_ptr(),
            subject.len(),
            start_offset,
            sopts,
            md,
            mc,
            repl.as_ptr(),
            repl.len(),
            buf.as_mut_ptr(),
            &mut outlen,
        );
        // Only the region the API says it wrote is observable.
        let written = if rc >= 0 { outlen.min(bufsize) } else { 0 };
        let out = buf[..written].to_vec();
        if !md.is_null() {
            (api.match_data_free)(md);
        }
        (api.match_context_free)(mc);
        (api.code_free)(code);
        SubOut {
            compile_err: err,
            rc,
            outlen,
            out,
            callouts: CALLOUTS.with(|c| c.borrow().clone()),
            case_calls: CASECALLS.with(|c| c.borrow().clone()),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn diff_sub(
    pat: &str,
    subject: &str,
    repl: &str,
    copts: u32,
    sopts: u32,
    start_offset: Sz,
    bufsize: usize,
    use_matched: bool,
    with_callout: bool,
    with_case_callout: bool,
    ovecsize: Option<u32>,
) {
    let co = run_sub(
        c(),
        pat.as_bytes(),
        subject.as_bytes(),
        repl.as_bytes(),
        copts,
        sopts,
        start_offset,
        bufsize,
        use_matched,
        with_callout,
        with_case_callout,
        ovecsize,
    );
    let ro = run_sub(
        r(),
        pat.as_bytes(),
        subject.as_bytes(),
        repl.as_bytes(),
        copts,
        sopts,
        start_offset,
        bufsize,
        use_matched,
        with_callout,
        with_case_callout,
        ovecsize,
    );
    if co != ro {
        panic!(
            "SUBSTITUTE DIVERGENCE\n pat={pat:?} subj={subject:?} repl={repl:?}\n copts={copts:#x} sopts={sopts:#x} so={start_offset} buf={bufsize} matched={use_matched} co={with_callout} cc={with_case_callout} ovec={ovecsize:?}\n C    = {co:?}\n Rust = {ro:?}"
        );
    }
}

const PATS: &[&str] = &[
    "a", "a+", "(a)(b)", "(?<x>a)(?<y>b)", "\\d+", "b*", "", "(a)|(b)", "x?",
    "(?<n>a)(?<n>b)?", "\\b\\w+\\b", "(a)(b)?(c)?",
];
const SUBJECTS: &[&str] = &[
    "", "a", "abc", "aaa", "abcabc", "xxx", "a1b22c333", "hello world", "ab", "b",
];
const REPLS: &[&str] = &[
    "", "X", "[$0]", "[$1]", "[$2]", "[${1}]", "[${x}]", "$", "$$", "\\$", "${", "${1",
    "${nope}", "$99", "\\U$0\\E", "\\L$0\\E", "\\u$0", "\\l$0", "${1:-def}", "${1:+yes:no}",
    "${x:+A:B}", "\\n", "\\x41", "\\Q$1\\E", "a\\", "$1$2$3", "\\U${1}x\\E", "${1:-$2}",
    "\\z", "\\g", "${9999}",
];

// ------------------------------------------------------- rows 65-71, 75; errors
#[test]
fn substitute_option_matrix() {
    let sopt_sets: &[u32] = &[
        0,
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_LITERAL,
        PCRE2_SUBSTITUTE_LITERAL | PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_UNSET_EMPTY,
        PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_OVERFLOW_LENGTH | PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY | PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_NOTEMPTY,
        PCRE2_NOTEMPTY_ATSTART,
        PCRE2_ANCHORED,
        PCRE2_NO_UTF_CHECK,
    ];
    for p in PATS {
        for s in SUBJECTS {
            for rp in REPLS {
                for &so in sopt_sets {
                    for buf in [0usize, 1, 4, 64] {
                        diff_sub(p, s, rp, 0, so, 0, buf, false, false, false, None);
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------- row 72
#[test]
fn substitute_matched_and_offsets() {
    for p in PATS {
        for s in SUBJECTS {
            for rp in ["X", "[$0]", "[$1]"] {
                for so in 0..=s.len().min(3) {
                    for sopts in [
                        0,
                        PCRE2_SUBSTITUTE_MATCHED,
                        PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_GLOBAL,
                        PCRE2_SUBSTITUTE_GLOBAL,
                    ] {
                        for ovec in [None, Some(1u32), Some(2), Some(8)] {
                            diff_sub(p, s, rp, 0, sopts, so, 64, true, false, false, ovec);
                            diff_sub(p, s, rp, 0, sopts, so, 64, false, false, false, ovec);
                        }
                    }
                }
            }
        }
    }
    // start_offset past the end (ERRORS.md row 67)
    for p in PATS {
        for s in SUBJECTS {
            for so in [s.len() + 1, s.len() + 5, usize::MAX] {
                diff_sub(p, s, "X", 0, 0, so, 64, false, false, false, None);
            }
        }
    }
}

// -------------------------------------------------------------- rows 73 and 74
#[test]
fn substitute_callouts() {
    for p in PATS {
        for s in SUBJECTS {
            for rp in ["X", "[$0]", "\\U$0\\E", "\\L$0\\E", "\\u$0\\l$0"] {
                for sopts in [
                    0,
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_GLOBAL,
                ] {
                    for (co, cc) in [(true, false), (false, true), (true, true)] {
                        diff_sub(p, s, rp, 0, sopts, 0, 64, false, co, cc, None);
                        diff_sub(p, s, rp, 0, sopts, 0, 4, false, co, cc, None);
                    }
                }
            }
        }
    }
}

// ------------------------------------------------------------------ row 68-70
#[test]
fn substitute_buffer_sizes() {
    for p in PATS {
        for s in SUBJECTS {
            for rp in ["LONGREPLACEMENT", "[$0][$0]", "X"] {
                for sopts in [
                    0,
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                    PCRE2_SUBSTITUTE_OVERFLOW_LENGTH | PCRE2_SUBSTITUTE_GLOBAL,
                ] {
                    for buf in 0..24usize {
                        diff_sub(p, s, rp, 0, sopts, 0, buf, false, false, false, None);
                    }
                }
            }
        }
    }
}

// --------------------------------------------------- ERRORS.md rows 58-66, 77
#[test]
fn substitute_error_paths() {
    let (c, r) = (c(), r());
    // row 58: undefined option bits
    for bad in [
        0x0000_0040u32, // DFA_RESTART
        0x0000_0080,    // DFA_SHORTEST
        0x0008_0000,    // undefined
        0x0400_0000,    // undefined
        0x1000_0000,    // undefined
        0x8000_0000,    // ANCHORED (valid)
        0xffff_ffff,
    ] {
        let co = run_sub(c, b"a", b"aaa", b"X", 0, bad, 0, 64, false, false, false, None);
        let ro = run_sub(r, b"a", b"aaa", b"X", 0, bad, 0, 64, false, false, false, None);
        assert_eq!(co, ro, "substitute bad options {bad:#x}");
    }

    unsafe {
        for (api, name) in [(c, "C"), (r, "Rust")] {
            let mut err = 0;
            let mut off = 0;
            let code =
                (api.compile)(b"a".as_ptr(), 1, 0, &mut err, &mut off, std::ptr::null_mut());
            let mut buf = [0u8; 32];
            let mut outlen: Sz = 32;
            // rows 59/60: NULL replacement / subject with non-zero length
            let rc1 = (api.substitute)(
                code,
                b"aaa".as_ptr(),
                3,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
                1,
                buf.as_mut_ptr(),
                &mut outlen,
            );
            let rc2 = (api.substitute)(
                code,
                std::ptr::null(),
                3,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                b"X".as_ptr(),
                1,
                buf.as_mut_ptr(),
                &mut outlen,
            );
            // NULL with zero length is legal
            let mut outlen3: Sz = 32;
            let rc3 = (api.substitute)(
                code,
                std::ptr::null(),
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
                0,
                buf.as_mut_ptr(),
                &mut outlen3,
            );
            // row 61: SUBSTITUTE_MATCHED with NULL match data
            let mut outlen4: Sz = 32;
            let rc4 = (api.substitute)(
                code,
                b"aaa".as_ptr(),
                3,
                0,
                PCRE2_SUBSTITUTE_MATCHED,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                b"X".as_ptr(),
                1,
                buf.as_mut_ptr(),
                &mut outlen4,
            );
            eprintln!("{name}: {rc1} {rc2} {rc3} {rc4}");
            RESULTS.with(|v| v.borrow_mut().push((rc1, rc2, rc3, outlen3, rc4)));
            (api.code_free)(code);
        }
    }
    let got = RESULTS.with(|v| v.borrow().clone());
    assert_eq!(got[0], got[1], "substitute NULL-argument handling differs");
    assert_eq!(got[0].0, PCRE2_ERROR_NULL);
    assert_eq!(got[0].1, PCRE2_ERROR_NULL);
    assert_eq!(got[0].4, PCRE2_ERROR_NULL);

    // rows 62-66: SUBSTITUTE_MATCHED consistency checks
    unsafe {
        let mut out = Vec::new();
        for api in [c, r] {
            let mut err = 0;
            let mut off = 0;
            let code1 =
                (api.compile)(b"a".as_ptr(), 1, 0, &mut err, &mut off, std::ptr::null_mut());
            let code2 =
                (api.compile)(b"b".as_ptr(), 1, 0, &mut err, &mut off, std::ptr::null_mut());
            let subj1 = b"aaa".to_vec();
            let subj2 = b"aaa".to_vec(); // different pointer, same content
            let md = (api.match_data_create_from_pattern)(code1, std::ptr::null_mut());
            let mut buf = [0u8; 32];

            // run a match with code1/subj1/offset 0/options 0
            let mrc = (api.do_match)(
                code1,
                subj1.as_ptr(),
                3,
                0,
                0,
                md,
                std::ptr::null_mut(),
            );
            let mut res = Vec::new();
            // (a) different pattern
            let mut ol: Sz = 32;
            res.push((api.substitute)(
                code2, subj1.as_ptr(), 3, 0, PCRE2_SUBSTITUTE_MATCHED, md,
                std::ptr::null_mut(), b"X".as_ptr(), 1, buf.as_mut_ptr(), &mut ol,
            ));
            // (b) different subject pointer
            let mut ol: Sz = 32;
            res.push((api.substitute)(
                code1, subj2.as_ptr(), 3, 0, PCRE2_SUBSTITUTE_MATCHED, md,
                std::ptr::null_mut(), b"X".as_ptr(), 1, buf.as_mut_ptr(), &mut ol,
            ));
            // (c) different start offset
            let mut ol: Sz = 32;
            res.push((api.substitute)(
                code1, subj1.as_ptr(), 3, 1, PCRE2_SUBSTITUTE_MATCHED, md,
                std::ptr::null_mut(), b"X".as_ptr(), 1, buf.as_mut_ptr(), &mut ol,
            ));
            // (d) different match options
            let mut ol: Sz = 32;
            res.push((api.substitute)(
                code1, subj1.as_ptr(), 3, 0,
                PCRE2_SUBSTITUTE_MATCHED | PCRE2_NOTBOL, md,
                std::ptr::null_mut(), b"X".as_ptr(), 1, buf.as_mut_ptr(), &mut ol,
            ));
            // (e) match data produced by pcre2_dfa_match
            let md2 = (api.match_data_create_from_pattern)(code1, std::ptr::null_mut());
            let mut ws = [0i32; 64];
            (api.dfa_match)(
                code1, subj1.as_ptr(), 3, 0, 0, md2, std::ptr::null_mut(),
                ws.as_mut_ptr(), ws.len(),
            );
            let mut ol: Sz = 32;
            res.push((api.substitute)(
                code1, subj1.as_ptr(), 3, 0, PCRE2_SUBSTITUTE_MATCHED, md2,
                std::ptr::null_mut(), b"X".as_ptr(), 1, buf.as_mut_ptr(), &mut ol,
            ));
            // (f) the consistent case must succeed
            let mut ol: Sz = 32;
            let okrc = (api.substitute)(
                code1, subj1.as_ptr(), 3, 0, PCRE2_SUBSTITUTE_MATCHED, md,
                std::ptr::null_mut(), b"X".as_ptr(), 1, buf.as_mut_ptr(), &mut ol,
            );
            res.push(okrc);
            res.push(mrc);
            out.push(res);
            (api.match_data_free)(md);
            (api.match_data_free)(md2);
            (api.code_free)(code1);
            (api.code_free)(code2);
        }
        assert_eq!(out[0], out[1], "SUBSTITUTE_MATCHED consistency codes differ");
        eprintln!("SUBSTITUTE_MATCHED codes: {:?}", out[0]);
        // Distinct, specific error codes (not merely "both failed").
        assert!(out[0][0] < 0 && out[0][1] < 0 && out[0][2] < 0 && out[0][3] < 0);
        assert_eq!(out[0][4], PCRE2_ERROR_DFA_UFUNC);
        assert!(out[0][5] >= 0);
    }

    // row 77: partial match during substitute
    for sopts in [
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_PARTIAL_SOFT | PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_PARTIAL_HARD | PCRE2_SUBSTITUTE_GLOBAL,
    ] {
        for (p, s) in [("abcd", "abc"), ("\\d{4}", "12"), ("abc", "xxabc")] {
            diff_sub(p, s, "X", 0, sopts, 0, 64, false, false, false, None);
        }
    }

    // row 76: too many replacements / empty-match loops
    for (p, s) in [("", "aaaa"), ("x*", "aaaa"), ("(?=a)", "aaaa"), ("\\b", "a b c")] {
        for sopts in [
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        ] {
            diff_sub(p, s, "-", 0, sopts, 0, 4096, false, false, false, None);
        }
    }
}

thread_local! {
    static RESULTS: std::cell::RefCell<Vec<(c_int, c_int, c_int, Sz, c_int)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

// ------------------------------------------------------- randomized (property)
#[test]
fn substitute_randomized() {
    let mut rng = Rng::new(0x5EED_5B0B);
    let sopt_pool: &[u32] = &[
        0,
        PCRE2_SUBSTITUTE_GLOBAL,
        PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_LITERAL,
        PCRE2_SUBSTITUTE_UNSET_EMPTY,
        PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
        PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
        PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        PCRE2_NOTEMPTY,
        PCRE2_ANCHORED,
    ];
    let copt_pool: &[u32] = &[
        0,
        PCRE2_CASELESS,
        PCRE2_MULTILINE,
        PCRE2_UTF,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_DUPNAMES,
        PCRE2_NO_AUTO_CAPTURE,
    ];
    for i in 0..6000u32 {
        let p = if rng.bool() {
            (*rng.pick(PATS)).to_string()
        } else {
            let d = rng.range(1, 2) as u32;
            random_pattern(&mut rng, d)
        };
        let copts = *rng.pick(copt_pool);
        let s = if rng.bool() {
            (*rng.pick(SUBJECTS)).as_bytes().to_vec()
        } else {
            random_subject(&mut rng, copts & PCRE2_UTF != 0)
        };
        let rp = (*rng.pick(REPLS)).to_string();
        let sopts = *rng.pick(sopt_pool);
        let buf = rng.below(40);
        let so = if s.is_empty() { 0 } else { rng.below(s.len() + 1) };
        let with_co = rng.below(4) == 0;
        let with_cc = rng.below(4) == 0;
        let co = run_sub(
            c(), p.as_bytes(), &s, rp.as_bytes(), copts, sopts, so, buf, false,
            with_co, with_cc, None,
        );
        let ro = run_sub(
            r(), p.as_bytes(), &s, rp.as_bytes(), copts, sopts, so, buf, false,
            with_co, with_cc, None,
        );
        if co != ro {
            panic!(
                "SUBSTITUTE DIVERGENCE iter={i}\n pat={p:?} subj={:?} repl={rp:?} copts={copts:#x} sopts={sopts:#x} buf={buf} so={so}\n C    = {co:?}\n Rust = {ro:?}",
                String::from_utf8_lossy(&s)
            );
        }
    }
}
