//! Phase B/C — `pcre2_match` (the interpreter) across the match-option
//! cross-product, plus callouts, limits, marks and the match-data accessors.
//!
//! CONFIGS.md rows 90-118 · ERRORS.md rows 105-125.
#![allow(non_snake_case)]

mod common;
use common::corpus::*;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

/// Compile + match + log every observable. `md_ovec` of `None` means "create the
/// match data from the pattern".
fn match_probe(
    api: &Api,
    pat: &[u8],
    copts: u32,
    subj: &[u8],
    slen: Sz,
    start: Sz,
    mopts: u32,
    md_ovec: Option<u32>,
    mctx_setup: Option<&dyn Fn(&Api, MContext)>,
    l: &mut Log,
) {
    unsafe {
        let code = compile_logged(api, pat, pat.len(), copts, std::ptr::null_mut(), l);
        if code.is_null() {
            return;
        }
        let md = match md_ovec {
            None => (api.match_data_create_from_pattern)(code, std::ptr::null_mut()),
            Some(n) => (api.match_data_create)(n, std::ptr::null_mut()),
        };
        assert!(!md.is_null());
        let mctx = if mctx_setup.is_some() {
            let m = (api.match_context_create)(std::ptr::null_mut());
            (mctx_setup.unwrap())(api, m);
            m
        } else {
            std::ptr::null_mut()
        };
        let sp = if subj.is_empty() {
            b"".as_ptr()
        } else {
            subj.as_ptr()
        };
        let rc = (api.do_match)(code, sp, slen, start, mopts, md, mctx);
        log_match_result_full(api, code, md, rc, l);
        // The heapframes size is an internal allocation detail that both
        // implementations must agree on because it is observable via the API.
        l.u((api.get_match_data_heapframes_size)(md) as u64);
        if !mctx.is_null() {
            (api.match_context_free)(mctx);
        }
        (api.match_data_free)(md);
        (api.code_free)(code);
    }
}

fn diff_match(label: &str, pat: &[u8], copts: u32, subj: &[u8], start: Sz, mopts: u32) {
    diff(label, |api| {
        let mut l = Log::new();
        match_probe(
            api,
            pat,
            copts,
            subj,
            subj.len(),
            start,
            mopts,
            None,
            None,
            &mut l,
        );
        l
    });
}

// ------------------------------------------------- rows 95-97: baseline shapes

#[test]
fn match_all_patterns_all_subjects() {
    for (pi, p) in PATTERNS.iter().enumerate() {
        diff(&format!("match pat[{pi}]={p:?}"), |api| {
            let mut l = Log::new();
            for s in SUBJECTS {
                match_probe(
                    api,
                    p.as_bytes(),
                    0,
                    s.as_bytes(),
                    s.len(),
                    0,
                    0,
                    None,
                    None,
                    &mut l,
                );
            }
            l
        });
    }
}

#[test]
fn match_utf_patterns_all_subjects() {
    let byte_subjs = byte_subjects();
    for (pi, p) in PATTERNS.iter().enumerate() {
        for copts in [
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_UTF | PCRE2_CASELESS,
            PCRE2_MATCH_INVALID_UTF,
            PCRE2_UTF | PCRE2_NO_UTF_CHECK,
        ] {
            diff(&format!("matchutf pat[{pi}] copts={copts:#x}"), |api| {
                let mut l = Log::new();
                for s in &byte_subjs {
                    // PCRE2_NO_UTF_CHECK on a subject that is NOT valid UTF-8 is
                    // explicitly documented as undefined behaviour (the engine
                    // may read outside the subject), so it is only combined with
                    // valid-UTF-8 subjects.
                    let valid_utf = std::str::from_utf8(s).is_ok();
                    let mopt_set: &[u32] = if valid_utf {
                        &[0, PCRE2_NO_UTF_CHECK]
                    } else {
                        &[0]
                    };
                    for &mopts in mopt_set {
                        match_probe(
                            api,
                            p.as_bytes(),
                            copts,
                            s,
                            s.len(),
                            0,
                            mopts,
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

#[test]
fn match_subject_length_forms_and_start_offsets() {
    for (pi, p) in PATTERNS.iter().enumerate() {
        diff(&format!("lenforms pat[{pi}]"), |api| {
            let mut l = Log::new();
            for s in SUBJECTS {
                let mut z = s.as_bytes().to_vec();
                z.push(0);
                // explicit length, zero-terminated, and every start offset
                // including one past the end (which must be BADOFFSET).
                for &slen in &[s.len(), PCRE2_ZERO_TERMINATED] {
                    for start in 0..=(s.len() + 1) {
                        match_probe(
                            api,
                            p.as_bytes(),
                            0,
                            &z,
                            slen,
                            start,
                            0,
                            None,
                            None,
                            &mut l,
                        );
                    }
                }
            }
            // NULL subject with length 0 and with length != 0
            unsafe {
                let code =
                    compile_logged(api, p.as_bytes(), p.len(), 0, std::ptr::null_mut(), &mut l);
                if !code.is_null() {
                    let md = (api.match_data_create_from_pattern)(code, std::ptr::null_mut());
                    for len in [0usize, 1, 5] {
                        let rc = (api.do_match)(
                            code,
                            std::ptr::null(),
                            len,
                            0,
                            0,
                            md,
                            std::ptr::null_mut(),
                        );
                        log_match_result_full(api, code, md, rc, &mut l);
                    }
                    (api.match_data_free)(md);
                    (api.code_free)(code);
                }
            }
            l
        });
    }
}

// ------------------------------------------------- rows 98-104: match options

#[test]
fn match_each_option_bit() {
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
        ("NO_JIT", PCRE2_NO_JIT),
        ("COPY_MATCHED_SUBJECT", PCRE2_COPY_MATCHED_SUBJECT),
        ("DISABLE_RECURSELOOP_CHECK", PCRE2_DISABLE_RECURSELOOP_CHECK),
    ];
    for (name, bit) in opts {
        for (pi, p) in PATTERNS.iter().enumerate() {
            diff(&format!("mopt={name} pat[{pi}]"), |api| {
                let mut l = Log::new();
                for s in SUBJECTS {
                    match_probe(
                        api,
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        s.len(),
                        0,
                        *bit,
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

/// Pairwise combinations of the match options, on a smaller pattern set.
#[test]
fn match_option_pairs() {
    let bits = [
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
        PCRE2_NOTBOL,
        PCRE2_NOTEOL,
        PCRE2_NOTEMPTY,
        PCRE2_NOTEMPTY_ATSTART,
        PCRE2_PARTIAL_SOFT,
        PCRE2_PARTIAL_HARD,
        PCRE2_COPY_MATCHED_SUBJECT,
        PCRE2_DISABLE_RECURSELOOP_CHECK,
    ];
    let pats = [
        "a*", "^a", "a$", ".*", "a|b", "(a)(b)?", "a(?=b)", "(?<=a)b", "a+b", "(a(?R)?b)",
        r"\ba\b", "(*MARK:m)a", "a(*ACCEPT)b", "(?i)A", r"\R", r"\X",
    ];
    for i in 0..bits.len() {
        for j in i..bits.len() {
            let o = bits[i] | bits[j];
            for (pi, p) in pats.iter().enumerate() {
                diff(&format!("mpair {o:#x} pat[{pi}]"), |api| {
                    let mut l = Log::new();
                    for s in SUBJECTS {
                        match_probe(
                            api,
                            p.as_bytes(),
                            0,
                            s.as_bytes(),
                            s.len(),
                            0,
                            o,
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

// ------------------------------------------------- row 108: ovector sizes

#[test]
fn match_ovector_sizes() {
    let pats = [
        "(a)(b)(c)",
        "(a)(b)?(c)?",
        "(?<x>a)(?<y>b)",
        "((((a))))",
        "a",
        "(a)+",
        "(a|(b))+",
    ];
    for (pi, p) in pats.iter().enumerate() {
        diff(&format!("ovec pat[{pi}]"), |api| {
            let mut l = Log::new();
            for n in [0u32, 1, 2, 3, 4, 5, 16, 100] {
                for s in ["abc", "ab", "a", "", "bc", "aaa"] {
                    match_probe(
                        api,
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        s.len(),
                        0,
                        0,
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

// ------------------------------------------------- rows 110, 118: limits

fn set_limits(ml: u32, dl: u32, hl: u32) -> impl Fn(&Api, MContext) {
    move |api: &Api, m: MContext| unsafe {
        (api.set_match_limit)(m, ml);
        (api.set_depth_limit)(m, dl);
        (api.set_heap_limit)(m, hl);
    }
}

#[test]
fn match_limits() {
    let pats = [
        "(a+)+b",
        "(a|aa)+c",
        "a*a*a*a*a*b",
        "(?:a{1,10}){1,10}b",
        "abc",
        "(a(?R)?b)",
        r"(\w+\s?)*$",
    ];
    let subj = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaac";
    for (pi, p) in pats.iter().enumerate() {
        for (ml, dl, hl) in [
            (0u32, 0u32, 0u32),
            (1, 1, 0),
            (1, u32::MAX, u32::MAX),
            (u32::MAX, 1, u32::MAX),
            (u32::MAX, u32::MAX, 0),
            (10, 10, 1),
            (100, 100, 10),
            (10000, 10000, 100),
            (u32::MAX, u32::MAX, u32::MAX),
        ] {
            let f = set_limits(ml, dl, hl);
            diff(&format!("limits pat[{pi}] {ml}/{dl}/{hl}"), |api| {
                let mut l = Log::new();
                for s in [subj, "abc", "", "aaab"] {
                    match_probe(
                        api,
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        s.len(),
                        0,
                        0,
                        None,
                        Some(&f),
                        &mut l,
                    );
                }
                l
            });
        }
    }
}

#[test]
fn match_offset_limit() {
    let pats = ["a", "b", ".*", "x", "(a)", r"\d"];
    for (pi, p) in pats.iter().enumerate() {
        let subj = "aaa b aaa 123 x";
        for lim in [0usize, 1, 3, 5, 14, 15, 16, PCRE2_UNSET] {
            for copts in [0u32, PCRE2_USE_OFFSET_LIMIT] {
                let f = move |api: &Api, m: MContext| unsafe {
                    (api.set_offset_limit)(m, lim);
                };
                diff(
                    &format!("offlimit pat[{pi}] lim={lim:#x} copts={copts:#x}"),
                    |api| {
                        let mut l = Log::new();
                        match_probe(
                            api,
                            p.as_bytes(),
                            copts,
                            subj.as_bytes(),
                            subj.len(),
                            0,
                            0,
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

// ------------------------------------------------- row 109: callouts

/// Records every field of every callout block, plus the call order.
static mut CO_RET: c_int = 0;

unsafe extern "C" fn callout_cb(b: *mut CalloutBlock, data: *mut c_void) -> c_int {
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
    // ovector contents (capture_top pairs), not the pointer
    if !b.offset_vector.is_null() {
        for i in 0..(2 * b.capture_top as usize) {
            v.extend_from_slice(&(*b.offset_vector.add(i)).to_le_bytes());
        }
    }
    // mark / callout string contents, not the pointers
    v.push(b.mark.is_null() as u8);
    if !b.mark.is_null() {
        let s = cstr(b.mark);
        v.extend_from_slice(&(s.len() as u64).to_le_bytes());
        v.extend_from_slice(&s);
    }
    v.push(b.callout_string.is_null() as u8);
    if !b.callout_string.is_null() {
        v.extend_from_slice(std::slice::from_raw_parts(
            b.callout_string,
            b.callout_string_length,
        ));
    }
    // subject contents relative to the block, to prove the same buffer is seen
    v.push(b.subject.is_null() as u8);
    if !b.subject.is_null() {
        v.extend_from_slice(std::slice::from_raw_parts(b.subject, b.subject_length));
    }
    v.push(b'\n');
    CO_RET
}

#[test]
fn match_callouts() {
    let pats = [
        "(?C1)a(?C2)b(?C3)",
        "(?C)a*",
        "(?C{s})x",
        "a(?C1)|b(?C2)",
        "(?C1)(a)(?C2)(b)?",
        "(*MARK:m)(?C1)a",
        "(?C1)(?=a)(?C2)b",
        "abc",
    ];
    for (pi, p) in pats.iter().enumerate() {
        for copts in [0u32, PCRE2_AUTO_CALLOUT] {
            for ret in [0i32, 1, -1, -2, -37, -99] {
                diff(
                    &format!("callout pat[{pi}] copts={copts:#x} ret={ret}"),
                    |api| {
                        let mut l = Log::new();
                        unsafe {
                            CO_RET = ret;
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
                                    Some(callout_cb),
                                    &mut buf as *mut Vec<u8> as *mut c_void,
                                ) as i64);
                                let rc = (api.do_match)(
                                    code,
                                    s.as_ptr(),
                                    s.len(),
                                    0,
                                    0,
                                    md,
                                    m,
                                );
                                log_match_result_full(api, code, md, rc, &mut l);
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

// ------------------------------------------------- rows 111-116: constructs

#[test]
fn match_verbs_and_marks() {
    let pats = [
        "a(*ACCEPT)b",
        "a(*FAIL)|b",
        "a(*COMMIT)b|ab",
        "a(*PRUNE)b|ab",
        "a(*SKIP)b|ab",
        "a(*THEN)b|ac",
        "(*MARK:m1)a|(*MARK:m2)b",
        "a(*MARK:x)(*FAIL)|b",
        "(*PRUNE:p1)a|b",
        "(*SKIP:s1)a|b",
        "(*THEN:t1)a|b",
        "(a)(*ACCEPT)(b)",
        "(?:(*COMMIT)a|b)+",
        "^(?:(*THEN)a|b)",
    ];
    for (pi, p) in pats.iter().enumerate() {
        diff(&format!("verbs pat[{pi}]"), |api| {
            let mut l = Log::new();
            for s in SUBJECTS {
                for mopts in [0u32, PCRE2_ANCHORED, PCRE2_NOTEMPTY] {
                    match_probe(
                        api,
                        p.as_bytes(),
                        0,
                        s.as_bytes(),
                        s.len(),
                        0,
                        mopts,
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

#[test]
fn match_recursion_and_backrefs() {
    let pats = [
        r"(a)\1",
        r"(a)(b)\2\1",
        r"(?<n>a)\k<n>",
        r"(a)?\1b",
        r"(a(?R)?b)",
        r"\((?>[^()]|(?R))*\)",
        r"(?<n>a)(?&n)",
        r"(?(DEFINE)(?<x>a+))(?&x)b",
        r"(a)(?1)(?1)",
        r"^(\w+)\s+\1$",
        r"(a|b)+\1",
    ];
    for (pi, p) in pats.iter().enumerate() {
        for copts in [
            0u32,
            PCRE2_CASELESS,
            PCRE2_MATCH_UNSET_BACKREF,
            PCRE2_DUPNAMES,
        ] {
            diff(&format!("recur pat[{pi}] copts={copts:#x}"), |api| {
                let mut l = Log::new();
                for s in SUBJECTS {
                    for mopts in [0u32, PCRE2_DISABLE_RECURSELOOP_CHECK] {
                        match_probe(
                            api,
                            p.as_bytes(),
                            copts,
                            s.as_bytes(),
                            s.len(),
                            0,
                            mopts,
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

#[test]
fn match_backslash_R_all_conventions() {
    let pats = [r"\R", r"\R+", r"a\Rb", r"\R{2}", r"^.$", r"$", r"^", "."];
    let subjects: Vec<&str> = vec![
        "\r", "\n", "\r\n", "\n\r", "a\rb", "a\nb", "a\r\nb", "a\u{85}b", "a\u{2028}b",
        "a\u{2029}b", "a\u{b}b", "a\u{c}b", "\r\r", "\n\n", "\0", "a\0b",
    ];
    for nl in 1..=6u32 {
        for bsr in 1..=2u32 {
            for (pi, p) in pats.iter().enumerate() {
                for copts in [0u32, PCRE2_MULTILINE, PCRE2_UTF, PCRE2_DOTALL] {
                    diff(
                        &format!("bsR nl={nl} bsr={bsr} pat[{pi}] copts={copts:#x}"),
                        |api| {
                            let mut l = Log::new();
                            unsafe {
                                let cc = (api.compile_context_create)(std::ptr::null_mut());
                                l.i((api.set_newline)(cc, nl) as i64);
                                l.i((api.set_bsr)(cc, bsr) as i64);
                                let code =
                                    compile_logged(api, p.as_bytes(), p.len(), copts, cc, &mut l);
                                if !code.is_null() {
                                    log_all_info(api, code, &mut l);
                                    for s in &subjects {
                                        if copts & PCRE2_UTF != 0
                                            && std::str::from_utf8(s.as_bytes()).is_err()
                                        {
                                            continue;
                                        }
                                        let md = (api.match_data_create_from_pattern)(
                                            code,
                                            std::ptr::null_mut(),
                                        );
                                        let rc = (api.do_match)(
                                            code,
                                            s.as_ptr(),
                                            s.len(),
                                            0,
                                            0,
                                            md,
                                            std::ptr::null_mut(),
                                        );
                                        log_match_result_full(api, code, md, rc, &mut l);
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

#[test]
fn match_extended_grapheme_and_script_run() {
    let pats = [
        r"\X",
        r"\X+",
        r"\X{2}",
        r"a\Xb",
        "(*script_run:.+)",
        "(*sr:\\w+)",
        "(*asr:.+)",
        r"\p{L}\X",
    ];
    let subjects: Vec<String> = vec![
        "a".into(),
        "a\u{300}".into(),
        "a\u{300}\u{301}".into(),
        "\u{1F1E6}\u{1F1E7}".into(),
        "\u{1F1E6}\u{1F1E7}\u{1F1E8}".into(),
        "\u{1F600}".into(),
        "\u{1F468}\u{200D}\u{1F469}".into(),
        "\u{0D4E}\u{0D15}".into(),
        "\u{1100}\u{1160}\u{11A8}".into(),
        "\u{AC00}".into(),
        "\u{261D}\u{FE0F}".into(),
        "\r\n".into(),
        "abc".into(),
        "abcабв".into(),
        "\u{3042}\u{30A2}".into(),
    ];
    for (pi, p) in pats.iter().enumerate() {
        for copts in [PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, 0, PCRE2_UCP] {
            diff(&format!("grapheme pat[{pi}] copts={copts:#x}"), |api| {
                let mut l = Log::new();
                for s in &subjects {
                    match_probe(
                        api,
                        p.as_bytes(),
                        copts,
                        s.as_bytes(),
                        s.len(),
                        0,
                        0,
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

// ------------------------------------------------- row 82/95: randomized

#[test]
fn match_random_patterns_random_subjects() {
    let mut rng = Rng::new(0xBEEF_0001);
    for iter in 0..2500 {
        let pat = PatternGen::gen(&mut rng);
        let utf = rng.bool();
        let copts = if utf { PCRE2_UTF } else { 0 };
        let subs: Vec<Vec<u8>> = (0..6).map(|_| gen_subject(&mut rng, utf)).collect();
        let mopts = {
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
                PCRE2_COPY_MATCHED_SUBJECT,
            ];
            *rng.pick(&bits)
        };
        let ovec = *rng.pick(&[0u32, 1, 2, 4, 32]);
        let starts: Vec<usize> = subs.iter().map(|s| rng.below(s.len() + 1)).collect();
        diff(
            &format!("randmatch iter={iter} pat={pat:?} copts={copts:#x} mopts={mopts:#x}"),
            |api| {
                let mut l = Log::new();
                for (k, s) in subs.iter().enumerate() {
                    match_probe(
                        api,
                        pat.as_bytes(),
                        copts,
                        s,
                        s.len(),
                        starts[k],
                        mopts,
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

/// Long subjects, so the start-optimizations, bumpalong and heapframe growth
/// paths are all reached.
#[test]
fn match_long_subjects() {
    let mut rng = Rng::new(0xBEEF_0002);
    let pats = [
        "a", ".*b", "^x", "z$", r"\d+", "(a)(b)(c)", "(?i)ABC", r"\bword\b", "a{100}",
        "[^q]*q", ".*.*.*x",
    ];
    for (pi, p) in pats.iter().enumerate() {
        let mut subs: Vec<Vec<u8>> = Vec::new();
        for n in [0usize, 1, 2, 255, 256, 257, 1000, 4096] {
            subs.push(vec![b'a'; n]);
            let mut v: Vec<u8> = (0..n).map(|_| *rng.pick(b"abcxyz0123 \n")).collect();
            v.push(b'q');
            subs.push(v);
        }
        for copts in [0u32, PCRE2_NO_START_OPTIMIZE, PCRE2_UTF] {
            diff(&format!("long pat[{pi}] copts={copts:#x}"), |api| {
                let mut l = Log::new();
                for s in &subs {
                    match_probe(
                        api,
                        p.as_bytes(),
                        copts,
                        s,
                        s.len(),
                        0,
                        0,
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

// ------------------------------------------------- ERRORS rows 105-125

#[test]
fn match_error_paths() {
    diff("match_errors", |api| {
        let mut l = Log::new();
        unsafe {
            // ---- match_data == NULL
            let code = compile_logged(api, b"abc", 3, 0, std::ptr::null_mut(), &mut l);
            assert!(!code.is_null());
            let rc = (api.do_match)(
                code,
                b"abc".as_ptr(),
                3,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            );
            l.tag("md_null").i(rc as i64);

            let md = (api.match_data_create)(4, std::ptr::null_mut());

            // ---- code == NULL
            let rc = (api.do_match)(
                std::ptr::null_mut(),
                b"abc".as_ptr(),
                3,
                0,
                0,
                md,
                std::ptr::null_mut(),
            );
            l.tag("code_null").i(rc as i64);

            // ---- subject == NULL with length != 0 / == 0
            for len in [0usize, 1, 3] {
                let rc =
                    (api.do_match)(code, std::ptr::null(), len, 0, 0, md, std::ptr::null_mut());
                l.tag("subj_null").i(rc as i64);
            }

            // ---- undefined option bits, including out-of-range values
            for o in [
                0x0000_0040u32, // DFA_RESTART: not a match option
                0x0000_0080,    // DFA_SHORTEST
                0x0000_0100,    // SUBSTITUTE_GLOBAL
                0x0000_8000,
                0x0001_0000,
                0x0002_0000,
                0x0008_0000,
                0x1000_0000,
                0xFFFF_FFFF,
                PCRE2_PARTIAL_SOFT | PCRE2_PARTIAL_HARD,
            ] {
                let rc = (api.do_match)(code, b"abc".as_ptr(), 3, 0, o, md, std::ptr::null_mut());
                l.tag("badopt").u(o as u64).i(rc as i64);
            }

            // ---- start_offset > length
            for start in [0usize, 3, 4, 100, usize::MAX] {
                let rc = (api.do_match)(
                    code,
                    b"abc".as_ptr(),
                    3,
                    start,
                    0,
                    md,
                    std::ptr::null_mut(),
                );
                l.tag("badoffset").i(rc as i64);
            }

            // ---- bad magic number (row 111) and bad mode (row 112)
            let cp = (api.code_copy)(code);
            with_bad_magic(cp, || {
                let rc = (api.do_match)(cp, b"abc".as_ptr(), 3, 0, 0, md, std::ptr::null_mut());
                l.tag("badmagic").i(rc as i64);
            });
            with_bad_mode(cp, || {
                let rc = (api.do_match)(cp, b"abc".as_ptr(), 3, 0, 0, md, std::ptr::null_mut());
                l.tag("badmode").i(rc as i64);
            });
            (api.code_free)(cp);

            // ---- offset limit without USE_OFFSET_LIMIT
            let m = (api.match_context_create)(std::ptr::null_mut());
            (api.set_offset_limit)(m, 1);
            let rc = (api.do_match)(code, b"abc".as_ptr(), 3, 0, 0, md, m);
            l.tag("badoffsetlimit").i(rc as i64);
            (api.set_offset_limit)(m, PCRE2_UNSET);
            let rc = (api.do_match)(code, b"abc".as_ptr(), 3, 0, 0, md, m);
            l.tag("offsetlimit_unset").i(rc as i64);
            (api.match_context_free)(m);

            (api.match_data_free)(md);
            (api.code_free)(code);
        }
        l
    });
}

/// UTF-validity error paths: every invalid sequence, at every start offset.
#[test]
fn match_utf_error_paths() {
    let bad: Vec<Vec<u8>> = vec![
        vec![0x80],
        vec![0xBF],
        vec![0xC0, 0x80],
        vec![0xC2],
        vec![0xC2, 0x41],
        vec![0xE0, 0x80, 0x80],
        vec![0xE0, 0xA0],
        vec![0xED, 0xA0, 0x80],
        vec![0xF0, 0x80, 0x80, 0x80],
        vec![0xF4, 0x90, 0x80, 0x80],
        vec![0xF5, 0x80, 0x80, 0x80],
        vec![0xFE],
        vec![0xFF],
        vec![b'a', 0x80, b'b'],
        vec![b'a', 0xC2, b'b'],
        vec![0xE2, 0x82, 0xAC, 0x80],
        // valid, but with start offsets inside a character
        vec![0xC2, 0xA9],
        vec![0xE2, 0x82, 0xAC],
        vec![0xF0, 0x9F, 0x98, 0x80],
    ];
    for (bi, s) in bad.iter().enumerate() {
        for copts in [
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_MATCH_INVALID_UTF,
            PCRE2_MATCH_INVALID_UTF | PCRE2_UCP,
        ] {
            for mopts in [0u32, PCRE2_NO_UTF_CHECK, PCRE2_PARTIAL_HARD] {
                for start in 0..=s.len() {
                    // PCRE2_NO_UTF_CHECK on an invalid subject is undefined
                    // behaviour per the documentation, so skip that pairing.
                    if mopts & PCRE2_NO_UTF_CHECK != 0 {
                        continue;
                    }
                    diff(
                        &format!("utferr[{bi}] copts={copts:#x} mopts={mopts:#x} start={start}"),
                        |api| {
                            let mut l = Log::new();
                            match_probe(
                                api,
                                b".*",
                                copts,
                                s,
                                s.len(),
                                start,
                                mopts,
                                None,
                                None,
                                &mut l,
                            );
                            match_probe(
                                api,
                                b"a",
                                copts,
                                s,
                                s.len(),
                                start,
                                mopts,
                                None,
                                None,
                                &mut l,
                            );
                            l
                        },
                    );
                }
            }
        }
    }
}

/// `\K` moving the start backwards inside an assertion must give
/// `PCRE2_ERROR_BAD_BACKSLASH_K` from both, and `(?R)` loops must give
/// `PCRE2_ERROR_RECURSELOOP`.
#[test]
fn match_k_and_recurseloop() {
    let pats = [
        r"(?<=a\Kb)c",
        r"(?=a\K)b",
        r"a\Kb",
        r"(?:a\K)*b",
        r"(a\K)+",
        r"(?R)",
        r"(?:(?R))",
        r"(a?(?R))",
        r"(|(?R))",
        r"()(?1)*",
    ];
    for (pi, p) in pats.iter().enumerate() {
        for extra in [0u32, PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK] {
            for mopts in [0u32, PCRE2_DISABLE_RECURSELOOP_CHECK] {
                diff(
                    &format!("kloop pat[{pi}] extra={extra:#x} mopts={mopts:#x}"),
                    |api| {
                        let mut l = Log::new();
                        unsafe {
                            let cc = (api.compile_context_create)(std::ptr::null_mut());
                            (api.set_compile_extra_options)(cc, extra);
                            let code =
                                compile_logged(api, p.as_bytes(), p.len(), 0, cc, &mut l);
                            if !code.is_null() {
                                for s in ["abc", "ab", "a", "", "aaab"] {
                                    let md = (api.match_data_create_from_pattern)(
                                        code,
                                        std::ptr::null_mut(),
                                    );
                                    let m = (api.match_context_create)(std::ptr::null_mut());
                                    // keep the limits small so a runaway
                                    // recursion terminates quickly
                                    (api.set_match_limit)(m, 10_000);
                                    (api.set_depth_limit)(m, 1_000);
                                    let rc = (api.do_match)(
                                        code,
                                        s.as_ptr(),
                                        s.len(),
                                        0,
                                        mopts,
                                        md,
                                        m,
                                    );
                                    log_match_result_full(api, code, md, rc, &mut l);
                                    (api.match_context_free)(m);
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
