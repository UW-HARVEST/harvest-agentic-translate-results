//! Phase B/C — `pcre2_dfa_match` across the DFA option cross-product.
//!
//! CONFIGS.md rows 119-128 · ERRORS.md rows 126-147.
//!
//! Note on the ovector: `pcre2_dfa_match` fills `2 * rc` entries when `rc > 0`
//! (start/end pairs of the alternative match lengths), all `2 * oveccount`
//! entries when `rc == 0`, and only pair 0 on `PCRE2_ERROR_PARTIAL`
//! (`pcre2_dfa_match.c:4048`). Everything else is untouched, so the harness's
//! `log_match_result` compares exactly the defined prefix.
#![allow(non_snake_case)]

mod common;
use common::corpus::*;
use common::*;
use std::os::raw::c_int;

const WS_DEFAULT: usize = 1000;

fn dfa_probe(
    api: &Api,
    pat: &[u8],
    copts: u32,
    subj: &[u8],
    slen: Sz,
    start: Sz,
    mopts: u32,
    wscount: usize,
    ovec: Option<u32>,
    mctx_setup: Option<&dyn Fn(&Api, MContext)>,
    l: &mut Log,
) {
    unsafe {
        let code = compile_logged(api, pat, pat.len(), copts, std::ptr::null_mut(), l);
        if code.is_null() {
            return;
        }
        let md = match ovec {
            None => (api.match_data_create_from_pattern)(code, std::ptr::null_mut()),
            Some(n) => (api.match_data_create)(n, std::ptr::null_mut()),
        };
        let mctx = match mctx_setup {
            None => std::ptr::null_mut(),
            Some(f) => {
                let m = (api.match_context_create)(std::ptr::null_mut());
                f(api, m);
                m
            }
        };
        let mut ws: Vec<c_int> = vec![0; wscount.max(1)];
        let sp = if subj.is_empty() {
            b"".as_ptr()
        } else {
            subj.as_ptr()
        };
        let rc = (api.dfa_match)(
            code,
            sp,
            slen,
            start,
            mopts,
            md,
            mctx,
            ws.as_mut_ptr(),
            wscount,
        );
        log_match_result(api, md, rc, l);
        if !mctx.is_null() {
            (api.match_context_free)(mctx);
        }
        (api.match_data_free)(md);
        (api.code_free)(code);
    }
}

// ------------------------------------------------- row 119: baseline

#[test]
fn dfa_all_patterns_all_subjects() {
    for (pi, p) in PATTERNS.iter().enumerate() {
        for ws in [20usize, 100, WS_DEFAULT] {
            diff(&format!("dfa pat[{pi}]={p:?} ws={ws}"), |api| {
                let mut l = Log::new();
                for s in SUBJECTS {
                    dfa_probe(
                        api,
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        s.len(),
                        0,
                        0,
                        ws,
                        None,
                        None,
                        &mut l,
                    );
                }
                l
            });
        }
    }
}

#[test]
fn dfa_subject_length_forms_and_start_offsets() {
    for (pi, p) in PATTERNS.iter().enumerate() {
        diff(&format!("dfa_lenforms pat[{pi}]"), |api| {
            let mut l = Log::new();
            for s in SUBJECTS {
                let mut z = s.as_bytes().to_vec();
                z.push(0);
                for &slen in &[s.len(), PCRE2_ZERO_TERMINATED] {
                    for start in 0..=(s.len() + 1) {
                        dfa_probe(
                            api,
                            p.as_bytes(),
                            0,
                            &z,
                            slen,
                            start,
                            0,
                            WS_DEFAULT,
                            None,
                            None,
                            &mut l,
                        );
                    }
                }
            }
            // NULL subject
            for len in [0usize, 1, 5] {
                dfa_probe_null_subject(api, p.as_bytes(), len, &mut l);
            }
            l
        });
    }
}

fn dfa_probe_null_subject(api: &Api, pat: &[u8], len: Sz, l: &mut Log) {
    unsafe {
        let code = compile_logged(api, pat, pat.len(), 0, std::ptr::null_mut(), l);
        if code.is_null() {
            return;
        }
        let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
        let mut ws: Vec<c_int> = vec![0; WS_DEFAULT];
        let rc = (api.dfa_match)(
            code,
            std::ptr::null(),
            len,
            0,
            0,
            md,
            std::ptr::null_mut(),
            ws.as_mut_ptr(),
            WS_DEFAULT,
        );
        log_match_result(api, md, rc, l);
        (api.match_data_free)(md);
        (api.code_free)(code);
    }
}

// ------------------------------------------------- rows 120-123, 128: options

#[test]
fn dfa_each_option_bit() {
    let opts: &[(&str, u32)] = &[
        ("ANCHORED", PCRE2_ANCHORED),
        ("ENDANCHORED", PCRE2_ENDANCHORED),
        ("NOTBOL", PCRE2_NOTBOL),
        ("NOTEOL", PCRE2_NOTEOL),
        ("NOTEMPTY", PCRE2_NOTEMPTY),
        ("NOTEMPTY_ATSTART", PCRE2_NOTEMPTY_ATSTART),
        ("NO_UTF_CHECK", PCRE2_NO_UTF_CHECK),
        ("PARTIAL_SOFT", PCRE2_PARTIAL_SOFT),
        ("PARTIAL_HARD", PCRE2_PARTIAL_HARD),
        ("DFA_SHORTEST", PCRE2_DFA_SHORTEST),
        ("COPY_MATCHED_SUBJECT", PCRE2_COPY_MATCHED_SUBJECT),
    ];
    for (name, bit) in opts {
        for (pi, p) in PATTERNS.iter().enumerate() {
            diff(&format!("dfaopt={name} pat[{pi}]"), |api| {
                let mut l = Log::new();
                for s in SUBJECTS {
                    dfa_probe(
                        api,
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        s.len(),
                        0,
                        *bit,
                        WS_DEFAULT,
                        None,
                        None,
                        &mut l,
                    );
                }
                l
            });
        }
    }
}

#[test]
fn dfa_option_pairs() {
    let bits = [
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_NOTBOL,
        PCRE2_NOTEOL,
        PCRE2_NOTEMPTY,
        PCRE2_NOTEMPTY_ATSTART,
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_DFA_SHORTEST,
        PCRE2_COPY_MATCHED_SUBJECT,
    ];
    let pats = [
        "a*", "^a", "a$", ".*", "a|b", "(a)(b)?", "a(?=b)", "(?<=a)b", "a+b", r"\ba\b",
        "(*MARK:m)a", "a(*ACCEPT)b", "(?i)A", r"\R", r"\X", "[a-z]+",
    ];
    for i in 0..bits.len() {
        for j in i..bits.len() {
            let o = bits[i] | bits[j];
            for (pi, p) in pats.iter().enumerate() {
                diff(&format!("dfapair {o:#x} pat[{pi}]"), |api| {
                    let mut l = Log::new();
                    for s in SUBJECTS {
                        dfa_probe(
                            api,
                            p.as_bytes(),
                            0,
                            s.as_bytes(),
                            s.len(),
                            0,
                            o,
                            WS_DEFAULT,
                            None,
                            None,
                            &mut l,
                        );
                    }
                    l
                });
            }
        }
    }
}

// ------------------------------------------------- row 121: DFA_RESTART

/// Drives a partial match and then continues it with `PCRE2_DFA_RESTART`,
/// reusing the same workspace — the documented restart protocol.
#[test]
fn dfa_restart_protocol() {
    let pats = [
        "abcd", "ab+cd", "a.*z", "^abc$", r"\d{4}", "(ab)+c", "xyz", "a(?:bc)?d",
    ];
    let subjects = ["ab", "abc", "abcd", "a", "", "abcdabcd", "12", "1234"];
    for (pi, p) in pats.iter().enumerate() {
        for split in 0..4usize {
            diff(&format!("dfarestart pat[{pi}] split={split}"), |api| {
                let mut l = Log::new();
                unsafe {
                    let code =
                        compile_logged(api, p.as_bytes(), p.len(), 0, std::ptr::null_mut(), &mut l);
                    if code.is_null() {
                        return l;
                    }
                    for s in subjects {
                        let b = s.as_bytes();
                        let cut = split.min(b.len());
                        let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                        let mut ws: Vec<c_int> = vec![0; WS_DEFAULT];
                        // first half, asking for a partial match
                        let rc1 = (api.dfa_match)(
                            code,
                            b.as_ptr(),
                            cut,
                            0,
                            PCRE2_PARTIAL_HARD,
                            md,
                            std::ptr::null_mut(),
                            ws.as_mut_ptr(),
                            WS_DEFAULT,
                        );
                        log_match_result(api, md, rc1, &mut l);
                        // restart over the whole subject with the same workspace
                        let rc2 = (api.dfa_match)(
                            code,
                            b.as_ptr(),
                            b.len(),
                            cut,
                            PCRE2_DFA_RESTART | PCRE2_PARTIAL_HARD,
                            md,
                            std::ptr::null_mut(),
                            ws.as_mut_ptr(),
                            WS_DEFAULT,
                        );
                        log_match_result(api, md, rc2, &mut l);
                        // the workspace is an output too: compare its first
                        // few slots, which hold the restart state
                        l.b(&ws[..8]
                            .iter()
                            .flat_map(|v| v.to_le_bytes())
                            .collect::<Vec<u8>>());
                        (api.match_data_free)(md);
                    }
                    (api.code_free)(code);
                }
                l
            });
        }
    }
}

// ------------------------------------------------- row 124: UTF

#[test]
fn dfa_utf_subjects() {
    let byte_subjs = byte_subjects();
    for (pi, p) in PATTERNS.iter().enumerate() {
        for copts in [PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_UTF | PCRE2_CASELESS] {
            diff(&format!("dfautf pat[{pi}] copts={copts:#x}"), |api| {
                let mut l = Log::new();
                for s in &byte_subjs {
                    let valid = std::str::from_utf8(s).is_ok();
                    let mopt_set: &[u32] = if valid {
                        &[0, PCRE2_NO_UTF_CHECK]
                    } else {
                        &[0]
                    };
                    for &mopts in mopt_set {
                        dfa_probe(
                            api,
                            p.as_bytes(),
                            copts,
                            s,
                            s.len(),
                            0,
                            mopts,
                            WS_DEFAULT,
                            None,
                            None,
                            &mut l,
                        );
                    }
                }
                l
            });
        }
    }
}

// ------------------------------------------------- row 125: ovector sizes

#[test]
fn dfa_ovector_sizes() {
    let pats = [
        "a*", "(a)(b)(c)", "a|ab|abc", "a+", ".*", "(a|ab)+", "x", "[a-z]*",
    ];
    for (pi, p) in pats.iter().enumerate() {
        diff(&format!("dfaovec pat[{pi}]"), |api| {
            let mut l = Log::new();
            for n in [0u32, 1, 2, 3, 4, 16, 100] {
                for s in ["", "a", "ab", "abc", "aaa", "xyz"] {
                    dfa_probe(
                        api,
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        s.len(),
                        0,
                        0,
                        WS_DEFAULT,
                        Some(n),
                        None,
                        &mut l,
                    );
                }
            }
            l
        });
    }
}

// ------------------------------------------------- row 126/127

#[test]
fn dfa_limits_and_offset_limit() {
    let pats = ["(a+)+b", "a*a*a*b", "abc", ".*", "(a(?1)?b)"];
    for (pi, p) in pats.iter().enumerate() {
        for (ml, dl, hl) in [
            (0u32, 0u32, 0u32),
            (1, 1, 0),
            (10, 10, 1),
            (1000, 1000, 100),
            (u32::MAX, u32::MAX, 1000),
        ] {
            let f = move |api: &Api, m: MContext| unsafe {
                (api.set_match_limit)(m, ml);
                (api.set_depth_limit)(m, dl);
                (api.set_heap_limit)(m, hl);
            };
            diff(&format!("dfalimits pat[{pi}] {ml}/{dl}/{hl}"), |api| {
                let mut l = Log::new();
                for s in ["aaaaaaaaaaaaaaaaaaaac", "abc", "", "aaab"] {
                    dfa_probe(
                        api,
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        s.len(),
                        0,
                        0,
                        WS_DEFAULT,
                        None,
                        Some(&f),
                        &mut l,
                    );
                }
                l
            });
        }
    }

    // offset_limit: valid values are 0..=length (per the API contract) plus
    // PCRE2_UNSET; anything larger would let the engine run past the subject.
    let subj = "aaa b aaa 123 x";
    for (pi, p) in ["a", "b", ".*", "x", r"\d"].iter().enumerate() {
        for lim in [0usize, 1, 3, 5, 14, 15, PCRE2_UNSET] {
            for copts in [0u32, PCRE2_USE_OFFSET_LIMIT] {
                let f = move |api: &Api, m: MContext| unsafe {
                    (api.set_offset_limit)(m, lim);
                };
                diff(
                    &format!("dfaofflimit pat[{pi}] lim={lim:#x} copts={copts:#x}"),
                    |api| {
                        let mut l = Log::new();
                        dfa_probe(
                            api,
                            p.as_bytes(),
                            copts,
                            subj.as_bytes(),
                            subj.len(),
                            0,
                            0,
                            WS_DEFAULT,
                            None,
                            Some(&f),
                            &mut l,
                        );
                        l
                    },
                );
            }
        }
    }
}

#[test]
fn dfa_newline_and_bsr() {
    let pats = [r"\R", r"a\Rb", r"^.$", r"$", r"^", ".", r"\R+"];
    let subjects = [
        "\r", "\n", "\r\n", "a\rb", "a\nb", "a\r\nb", "a\u{85}b", "a\u{2028}b", "\0", "a\0b",
    ];
    for nl in 1..=6u32 {
        for bsr in 1..=2u32 {
            for (pi, p) in pats.iter().enumerate() {
                for copts in [0u32, PCRE2_MULTILINE, PCRE2_DOTALL] {
                    diff(
                        &format!("dfanl nl={nl} bsr={bsr} pat[{pi}] copts={copts:#x}"),
                        |api| {
                            let mut l = Log::new();
                            unsafe {
                                let cc = (api.compile_context_create)(std::ptr::null_mut());
                                l.i((api.set_newline)(cc, nl) as i64);
                                l.i((api.set_bsr)(cc, bsr) as i64);
                                let code =
                                    compile_logged(api, p.as_bytes(), p.len(), copts, cc, &mut l);
                                if !code.is_null() {
                                    for s in subjects {
                                        let md = (api.match_data_create_from_pattern)(
                                            code,
                                            std::ptr::null_mut(),
                                        );
                                        let mut ws: Vec<c_int> = vec![0; WS_DEFAULT];
                                        let rc = (api.dfa_match)(
                                            code,
                                            s.as_ptr(),
                                            s.len(),
                                            0,
                                            0,
                                            md,
                                            std::ptr::null_mut(),
                                            ws.as_mut_ptr(),
                                            WS_DEFAULT,
                                        );
                                        log_match_result(api, md, rc, &mut l);
                                        (api.match_data_free)(md);
                                    }
                                    (api.code_free)(code);
                                }
                                (api.compile_context_free)(cc);
                            }
                            l
                        },
                    );
                }
            }
        }
    }
}

// ------------------------------------------------- row 126: callouts

static mut DFA_CO_RET: c_int = 0;

unsafe extern "C" fn dfa_callout(b: *mut CalloutBlock, data: *mut std::ffi::c_void) -> c_int {
    let b = &*b;
    let v = &mut *(data as *mut Vec<u8>);
    v.extend_from_slice(&b.version.to_le_bytes());
    v.extend_from_slice(&b.callout_number.to_le_bytes());
    v.extend_from_slice(&b.capture_top.to_le_bytes());
    v.extend_from_slice(&b.capture_last.to_le_bytes());
    v.extend_from_slice(&b.subject_length.to_le_bytes());
    v.extend_from_slice(&b.start_match.to_le_bytes());
    v.extend_from_slice(&b.current_position.to_le_bytes());
    v.extend_from_slice(&b.pattern_position.to_le_bytes());
    v.extend_from_slice(&b.next_item_length.to_le_bytes());
    v.extend_from_slice(&b.callout_string_offset.to_le_bytes());
    v.extend_from_slice(&b.callout_string_length.to_le_bytes());
    v.extend_from_slice(&b.callout_flags.to_le_bytes());
    v.push(b.callout_string.is_null() as u8);
    if !b.callout_string.is_null() {
        v.extend_from_slice(std::slice::from_raw_parts(
            b.callout_string,
            b.callout_string_length,
        ));
    }
    v.push(b'\n');
    DFA_CO_RET
}

#[test]
fn dfa_callouts() {
    let pats = ["(?C1)a(?C2)b", "(?C)a*", "(?C{s})x", "a(?C1)|b(?C2)", "abc"];
    for (pi, p) in pats.iter().enumerate() {
        for copts in [0u32, PCRE2_AUTO_CALLOUT] {
            for ret in [0i32, 1, -1, -37, -99] {
                diff(
                    &format!("dfacallout pat[{pi}] copts={copts:#x} ret={ret}"),
                    |api| {
                        let mut l = Log::new();
                        unsafe {
                            DFA_CO_RET = ret;
                            let code = compile_logged(
                                api,
                                p.as_bytes(),
                                p.len(),
                                copts,
                                std::ptr::null_mut(),
                                &mut l,
                            );
                            if code.is_null() {
                                return l;
                            }
                            for s in ["ab", "a", "xab", "", "abcabc", "b"] {
                                let mut buf: Vec<u8> = Vec::new();
                                let md = (api.match_data_create_from_pattern)(
                                    code,
                                    std::ptr::null_mut(),
                                );
                                let m = (api.match_context_create)(std::ptr::null_mut());
                                l.i((api.set_callout)(
                                    m,
                                    Some(dfa_callout),
                                    &mut buf as *mut Vec<u8> as *mut std::ffi::c_void,
                                ) as i64);
                                let mut ws: Vec<c_int> = vec![0; WS_DEFAULT];
                                let rc = (api.dfa_match)(
                                    code,
                                    s.as_ptr(),
                                    s.len(),
                                    0,
                                    0,
                                    md,
                                    m,
                                    ws.as_mut_ptr(),
                                    WS_DEFAULT,
                                );
                                log_match_result(api, md, rc, &mut l);
                                l.b(&buf);
                                (api.match_context_free)(m);
                                (api.match_data_free)(md);
                            }
                            (api.code_free)(code);
                        }
                        l
                    },
                );
            }
        }
    }
}

// ------------------------------------------------- randomized

#[test]
fn dfa_random_patterns_random_subjects() {
    let mut rng = Rng::new(0xD0FA_0001);
    for iter in 0..2500 {
        let pat = PatternGen::gen(&mut rng);
        let utf = rng.bool();
        let copts = if utf { PCRE2_UTF } else { 0 };
        let subs: Vec<Vec<u8>> = (0..6).map(|_| gen_subject(&mut rng, utf)).collect();
        let bits = [
            0u32,
            PCRE2_ANCHORED,
            PCRE2_ENDANCHORED,
            PCRE2_NOTBOL,
            PCRE2_NOTEOL,
            PCRE2_NOTEMPTY,
            PCRE2_NOTEMPTY_ATSTART,
            PCRE2_PARTIAL_SOFT,
            PCRE2_PARTIAL_HARD,
            PCRE2_DFA_SHORTEST,
            PCRE2_COPY_MATCHED_SUBJECT,
        ];
        let mopts = *rng.pick(&bits);
        let ovec = *rng.pick(&[0u32, 1, 2, 4, 32]);
        let ws = *rng.pick(&[20usize, 40, 200, 1000]);
        let starts: Vec<usize> = subs.iter().map(|s| rng.below(s.len() + 1)).collect();
        diff(
            &format!("dfarand iter={iter} pat={pat:?} copts={copts:#x} mopts={mopts:#x} ws={ws}"),
            |api| {
                let mut l = Log::new();
                for (k, s) in subs.iter().enumerate() {
                    dfa_probe(
                        api,
                        pat.as_bytes(),
                        copts,
                        s,
                        s.len(),
                        starts[k],
                        mopts,
                        ws,
                        Some(ovec),
                        None,
                        &mut l,
                    );
                }
                l
            },
        );
    }
}

// ------------------------------------------------- ERRORS rows 126-147

#[test]
fn dfa_error_paths() {
    diff("dfa_errors", |api| {
        let mut l = Log::new();
        unsafe {
            let code = compile_logged(api, b"abc", 3, 0, std::ptr::null_mut(), &mut l);
            assert!(!code.is_null());
            let md = (api.match_data_create)(4, std::ptr::null_mut());
            let mut ws: Vec<c_int> = vec![0; WS_DEFAULT];

            // match_data == NULL
            let rc = (api.dfa_match)(
                code,
                b"abc".as_ptr(),
                3,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                ws.as_mut_ptr(),
                WS_DEFAULT,
            );
            l.tag("md_null").i(rc as i64);

            // code == NULL
            let rc = (api.dfa_match)(
                std::ptr::null_mut(),
                b"abc".as_ptr(),
                3,
                0,
                0,
                md,
                std::ptr::null_mut(),
                ws.as_mut_ptr(),
                WS_DEFAULT,
            );
            l.tag("code_null").i(rc as i64);

            // subject == NULL
            for len in [0usize, 1, 3] {
                let rc = (api.dfa_match)(
                    code,
                    std::ptr::null(),
                    len,
                    0,
                    0,
                    md,
                    std::ptr::null_mut(),
                    ws.as_mut_ptr(),
                    WS_DEFAULT,
                );
                l.tag("subj_null").i(rc as i64);
            }

            // workspace == NULL, and wscount < 20 (checked before the
            // workspace is ever dereferenced)
            for wsc in [0usize, 1, 19] {
                let rc = (api.dfa_match)(
                    code,
                    b"abc".as_ptr(),
                    3,
                    0,
                    0,
                    md,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    wsc,
                );
                l.tag("ws_small").u(wsc as u64).i(rc as i64);
            }

            // undefined / out-of-range option bits
            for o in [
                PCRE2_NO_JIT,
                PCRE2_DISABLE_RECURSELOOP_CHECK,
                PCRE2_SUBSTITUTE_GLOBAL,
                0x0002_0000u32,
                0x1000_0000,
                0xFFFF_FFFF,
                PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD,
                PCRE2_DFA_RESTART | PCRE2_DFA_SHORTEST,
            ] {
                let rc = (api.dfa_match)(
                    code,
                    b"abc".as_ptr(),
                    3,
                    0,
                    o,
                    md,
                    std::ptr::null_mut(),
                    ws.as_mut_ptr(),
                    WS_DEFAULT,
                );
                l.tag("badopt").u(o as u64).i(rc as i64);
            }

            // start_offset > length
            for start in [3usize, 4, 100, usize::MAX] {
                let rc = (api.dfa_match)(
                    code,
                    b"abc".as_ptr(),
                    3,
                    start,
                    0,
                    md,
                    std::ptr::null_mut(),
                    ws.as_mut_ptr(),
                    WS_DEFAULT,
                );
                l.tag("badoffset").i(rc as i64);
            }

            // bad magic number (row 134) and bad mode (row 135)
            let cp = (api.code_copy)(code);
            with_bad_magic(cp, || {
                let rc = (api.dfa_match)(
                    cp,
                    b"abc".as_ptr(),
                    3,
                    0,
                    0,
                    md,
                    std::ptr::null_mut(),
                    ws.as_mut_ptr(),
                    WS_DEFAULT,
                );
                l.tag("badmagic").i(rc as i64);
            });
            with_bad_mode(cp, || {
                let rc = (api.dfa_match)(
                    cp,
                    b"abc".as_ptr(),
                    3,
                    0,
                    0,
                    md,
                    std::ptr::null_mut(),
                    ws.as_mut_ptr(),
                    WS_DEFAULT,
                );
                l.tag("badmode").i(rc as i64);
            });
            (api.code_free)(cp);

            // DFA_RESTART with a fresh (never-partial) workspace
            let mut ws2: Vec<c_int> = vec![0; WS_DEFAULT];
            let rc = (api.dfa_match)(
                code,
                b"abc".as_ptr(),
                3,
                0,
                PCRE2_DFA_RESTART,
                md,
                std::ptr::null_mut(),
                ws2.as_mut_ptr(),
                WS_DEFAULT,
            );
            l.tag("badrestart").i(rc as i64);

            // offset limit without USE_OFFSET_LIMIT
            let m = (api.match_context_create)(std::ptr::null_mut());
            (api.set_offset_limit)(m, 1);
            let rc = (api.dfa_match)(
                code,
                b"abc".as_ptr(),
                3,
                0,
                0,
                md,
                m,
                ws.as_mut_ptr(),
                WS_DEFAULT,
            );
            l.tag("badoffsetlimit").i(rc as i64);
            (api.match_context_free)(m);

            (api.match_data_free)(md);
            (api.code_free)(code);

            // MATCH_INVALID_UTF pattern -> DFA_UINVALID_UTF
            let code2 = compile_logged(
                api,
                b"abc",
                3,
                PCRE2_MATCH_INVALID_UTF,
                std::ptr::null_mut(),
                &mut l,
            );
            if !code2.is_null() {
                let md2 = (api.match_data_create)(4, std::ptr::null_mut());
                let rc = (api.dfa_match)(
                    code2,
                    b"abc".as_ptr(),
                    3,
                    0,
                    0,
                    md2,
                    std::ptr::null_mut(),
                    ws.as_mut_ptr(),
                    WS_DEFAULT,
                );
                l.tag("uinvalidutf").i(rc as i64);
                (api.match_data_free)(md2);
                (api.code_free)(code2);
            }
        }
        l
    });
}

/// Patterns the DFA engine cannot handle: `\C` in UTF mode (`DFA_UITEM`),
/// conditions it rejects (`DFA_UCOND`), and zero-length recursion
/// (`DFA_RECURSE`).
#[test]
fn dfa_unsupported_items() {
    let pats = [
        r"\C",
        r"a\Cb",
        r"(?(R1)a|b)",
        r"(?(R)a|b)",
        r"(a)(?(1)b|c)",
        r"(?<n>a)(?(<n>)b|c)",
        r"()(?1)",
        r"(a?)(?1)",
        r"(|a)(?1)",
        r"(a(?1)?b)",
        r"(?(?=a)b|c)",
        r"(?(DEFINE)(?<x>a))(?&x)",
        r"\X",
        r"(*script_run:a+)",
    ];
    for (pi, p) in pats.iter().enumerate() {
        for copts in [0u32, PCRE2_UTF, PCRE2_UCP, PCRE2_UTF | PCRE2_UCP] {
            diff(&format!("dfaunsup pat[{pi}] copts={copts:#x}"), |api| {
                let mut l = Log::new();
                for s in ["", "a", "b", "ab", "abc", "aaa", "é"] {
                    if copts & PCRE2_UTF != 0 && std::str::from_utf8(s.as_bytes()).is_err() {
                        continue;
                    }
                    dfa_probe(
                        api,
                        p.as_bytes(),
                        copts,
                        s.as_bytes(),
                        s.len(),
                        0,
                        0,
                        WS_DEFAULT,
                        None,
                        None,
                        &mut l,
                    );
                }
                l
            });
        }
    }
}

/// Small workspaces so the `DFA_WSSIZE` overflow path is reached.
#[test]
fn dfa_workspace_overflow() {
    let pats = [
        "(a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p)+",
        "(?:a|aa|aaa|aaaa|aaaaa)+b",
        "[a-z]*[a-z]*[a-z]*x",
        "(a(?1)?b)",
        r"(\w+\s?)+$",
    ];
    let subj = "abcdefghijklmnopabcdefghijklmnop";
    for (pi, p) in pats.iter().enumerate() {
        for ws in [20usize, 21, 25, 30, 40, 60, 100, 300] {
            diff(&format!("dfaws pat[{pi}] ws={ws}"), |api| {
                let mut l = Log::new();
                for s in [subj, "aaaaaaaaaaab", "abc", ""] {
                    dfa_probe(
                        api,
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        s.len(),
                        0,
                        0,
                        ws,
                        None,
                        None,
                        &mut l,
                    );
                }
                l
            });
        }
    }
}

/// UTF-validity error paths for the DFA engine.
#[test]
fn dfa_utf_error_paths() {
    let bad: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xC0, 0x80],
        vec![0xC2],
        vec![0xC2, 0x41],
        vec![0xE0, 0x80, 0x80],
        vec![0xED, 0xA0, 0x80],
        vec![0xF4, 0x90, 0x80, 0x80],
        vec![0xFF],
        vec![b'a', 0x80, b'b'],
        vec![0xC2, 0xA9],
        vec![0xE2, 0x82, 0xAC],
        vec![0xF0, 0x9F, 0x98, 0x80],
    ];
    for (bi, s) in bad.iter().enumerate() {
        for copts in [PCRE2_UTF, PCRE2_UTF | PCRE2_UCP] {
            for start in 0..=s.len() {
                diff(&format!("dfautferr[{bi}] copts={copts:#x} start={start}"), |api| {
                    let mut l = Log::new();
                    for pat in [b".*".as_ref(), b"a".as_ref()] {
                        dfa_probe(
                            api,
                            pat,
                            copts,
                            s,
                            s.len(),
                            start,
                            0,
                            WS_DEFAULT,
                            None,
                            None,
                            &mut l,
                        );
                    }
                    l
                });
            }
        }
    }
}
