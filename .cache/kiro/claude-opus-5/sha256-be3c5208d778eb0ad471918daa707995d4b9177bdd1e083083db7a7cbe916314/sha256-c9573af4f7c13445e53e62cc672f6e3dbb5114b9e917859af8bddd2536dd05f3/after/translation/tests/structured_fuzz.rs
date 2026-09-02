//! Phase B — structured (grammar-driven) pattern fuzzing.
//!
//! `cfg_random_patterns_fuzz` in `match_diff.rs` generates raw byte soup, which
//! mostly exercises the *parser*. This file generates syntactically valid regexes
//! from a small grammar so that the *compiler, optimiser and matchers* are the
//! things under stress, then compares the compiled byte code and every match
//! result between the two shared objects.
//!
//! `PCRE2_FUZZ_ITERS` / `PCRE2_FUZZ_SEED` override the defaults.

mod common;
use common::*;
use std::ffi::c_void;

/// Recursively build a syntactically plausible regex.
fn gen_pat(rng: &mut Rng, depth: usize, ngroups: &mut u32, out: &mut Vec<u8>) {
    let leafy = depth == 0 || rng.below(100) < 35;
    if leafy {
        match rng.below(26) {
            0 => out.push(*rng.pick(b"abcxyzABZ019_ -")),
            1 => out.extend_from_slice(b"."),
            2 => out.extend_from_slice(*rng.pick(&[
                &b"\\d"[..], b"\\D", b"\\s", b"\\S", b"\\w", b"\\W", b"\\h", b"\\H", b"\\v",
                b"\\V", b"\\R", b"\\N", b"\\X",
            ])),
            3 => out.extend_from_slice(*rng.pick(&[
                &b"\\b"[..], b"\\B", b"\\A", b"\\Z", b"\\z", b"\\G", b"^", b"$",
            ])),
            4 => out.extend_from_slice(*rng.pick(&[
                &b"[abc]"[..], b"[^abc]", b"[a-z]", b"[^a-z]", b"[[:alpha:]]", b"[[:^digit:]]",
                b"[\\d\\s]", b"[\\x20-\\x7e]", b"[]a]", b"[a-]", b"[\\x{100}-\\x{200}]",
            ])),
            5 => out.extend_from_slice(*rng.pick(&[
                &b"\\p{L}"[..], b"\\P{L}", b"\\p{Lu}", b"\\p{Nd}", b"\\p{Greek}", b"\\p{Xan}",
                b"\\p{Xwd}", b"\\p{Any}",
            ])),
            6 => out.extend_from_slice(*rng.pick(&[
                &b"\\x41"[..], b"\\x{41}", b"\\101", b"\\o{101}", b"\\cA", b"\\n", b"\\r", b"\\t",
                b"\\f", b"\\e", b"\\0",
            ])),
            7 => out.extend_from_slice(b"\\Qa.b\\E"),
            8 => {
                if *ngroups > 0 {
                    let g = 1 + rng.below(*ngroups as usize);
                    out.extend_from_slice(format!("\\{}", g).as_bytes());
                } else {
                    out.push(b'a');
                }
            }
            9 => out.extend_from_slice(*rng.pick(&[
                &b"(*FAIL)"[..], b"(*ACCEPT)", b"(*COMMIT)", b"(*PRUNE)", b"(*SKIP)", b"(*THEN)",
                b"(*MARK:m)", b"(*:n)", b"(*PRUNE:p)", b"(*SKIP:s)", b"(*THEN:t)",
            ])),
            10 => out.extend_from_slice(*rng.pick(&[
                &b"(?C)"[..], b"(?C1)", b"(?C255)", b"(?C{s})", b"(?C`s`)",
            ])),
            11 => out.extend_from_slice(b"\\K"),
            _ => out.push(*rng.pick(b"abcABC012 .")),
        }
    } else {
        match rng.below(14) {
            0 => {
                // capture group
                *ngroups += 1;
                out.push(b'(');
                gen_pat(rng, depth - 1, ngroups, out);
                out.push(b')');
            }
            1 => {
                out.extend_from_slice(b"(?:");
                gen_pat(rng, depth - 1, ngroups, out);
                out.push(b')');
            }
            2 => {
                // named group
                *ngroups += 1;
                let n = *ngroups;
                out.extend_from_slice(format!("(?<g{}>", n).as_bytes());
                gen_pat(rng, depth - 1, ngroups, out);
                out.push(b')');
            }
            3 => {
                out.extend_from_slice(*rng.pick(&[
                    &b"(?="[..], b"(?!", b"(?<=", b"(?<!", b"(?>", b"(?*", b"(?<*",
                ]));
                gen_pat(rng, depth - 1, ngroups, out);
                out.push(b')');
            }
            4 => {
                // alternation
                out.extend_from_slice(b"(?:");
                let n = 1 + rng.below(3);
                for i in 0..n {
                    if i > 0 {
                        out.push(b'|');
                    }
                    gen_pat(rng, depth - 1, ngroups, out);
                }
                out.push(b')');
            }
            5 => {
                // quantified group
                out.extend_from_slice(b"(?:");
                gen_pat(rng, depth - 1, ngroups, out);
                out.push(b')');
                out.extend_from_slice(*rng.pick(&[
                    &b"*"[..], b"+", b"?", b"{2}", b"{0,3}", b"{1,}", b"*?", b"+?", b"??",
                    b"{2,4}?", b"*+", b"++", b"?+", b"{2,4}+",
                ]));
            }
            6 => {
                // inline options
                out.extend_from_slice(*rng.pick(&[
                    &b"(?i:"[..], b"(?-i:", b"(?s:", b"(?m:", b"(?x:", b"(?xx:", b"(?U:", b"(?n:",
                    b"(?J:", b"(?^i:", b"(?ims:",
                ]));
                gen_pat(rng, depth - 1, ngroups, out);
                out.push(b')');
            }
            7 => {
                // conditional on a group that exists
                if *ngroups > 0 {
                    let g = 1 + rng.below(*ngroups as usize);
                    out.extend_from_slice(format!("(?({})", g).as_bytes());
                    gen_pat(rng, depth - 1, ngroups, out);
                    out.push(b'|');
                    gen_pat(rng, depth - 1, ngroups, out);
                    out.push(b')');
                } else {
                    out.extend_from_slice(b"(?(?=a)b|c)");
                }
            }
            8 => {
                // atomic / script run / alpha assertions
                out.extend_from_slice(*rng.pick(&[
                    &b"(*atomic:"[..], b"(*script_run:", b"(*asr:", b"(*positive_lookahead:",
                    b"(*negative_lookahead:", b"(*positive_lookbehind:",
                    b"(*negative_lookbehind:",
                ]));
                gen_pat(rng, depth - 1, ngroups, out);
                out.push(b')');
            }
            9 => {
                // subroutine call to an existing group
                if *ngroups > 0 {
                    let g = 1 + rng.below(*ngroups as usize);
                    out.extend_from_slice(format!("(?{})", g).as_bytes());
                } else {
                    gen_pat(rng, depth - 1, ngroups, out);
                }
            }
            10 => {
                // extended class
                out.extend_from_slice(b"(?[");
                out.extend_from_slice(*rng.pick(&[
                    &b"[a-z]&&[b-y]"[..], b"[a-z]--[m-p]", b"[a-z]||[0-9]", b"![a-z]",
                    b"[\\p{L}]&&[a-f]", b"[a-c]",
                ]));
                out.extend_from_slice(b"])");
            }
            11 => {
                // concatenation
                let n = 2 + rng.below(3);
                for _ in 0..n {
                    gen_pat(rng, depth - 1, ngroups, out);
                }
            }
            12 => {
                // leaf with a quantifier
                gen_pat(rng, 0, ngroups, out);
                out.extend_from_slice(*rng.pick(&[
                    &b"*"[..], b"+", b"?", b"{2}", b"{0,3}", b"{1,}", b"*?", b"+?", b"*+", b"++",
                ]));
            }
            _ => {
                gen_pat(rng, depth - 1, ngroups, out);
                out.push(b'|');
                gen_pat(rng, depth - 1, ngroups, out);
            }
        }
    }
}

fn gen_subj(rng: &mut Rng) -> Vec<u8> {
    let alphabet: &[u8] =
        b"aabbccxyzABC019 _-.\n\r\t\x00\xC3\xA9\xE2\x82\xAC\xF0\x9F\x98\x80\x80\xFF";
    let n = rng.below(20);
    (0..n).map(|_| *rng.pick(alphabet)).collect()
}

#[test]
fn structured_pattern_fuzz() {
    let iters: usize = std::env::var("PCRE2_FUZZ_ITERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000);
    let seed: u64 = std::env::var("PCRE2_FUZZ_SEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0x5EED_5730_0000_0001);
    let p = libs();
    let mut rng = Rng::new(seed);

    let optsets: &[u32] = &[
        0,
        o::CASELESS,
        o::MULTILINE,
        o::DOTALL,
        o::EXTENDED,
        o::EXTENDED_MORE,
        o::UNGREEDY,
        o::DUPNAMES,
        o::NO_AUTO_CAPTURE,
        o::NO_AUTO_POSSESS,
        o::NO_START_OPTIMIZE,
        o::NO_DOTSTAR_ANCHOR,
        o::ANCHORED,
        o::ENDANCHORED,
        o::DOLLAR_ENDONLY,
        o::FIRSTLINE,
        o::MATCH_UNSET_BACKREF,
        o::ALT_CIRCUMFLEX,
        o::ALT_VERBNAMES,
        o::ALT_BSUX,
        o::ALT_EXTENDED_CLASS,
        o::ALLOW_EMPTY_CLASS,
        o::AUTO_CALLOUT,
        o::UTF,
        o::UCP,
        o::UTF | o::UCP,
        o::UTF | o::CASELESS,
        o::UTF | o::UCP | o::CASELESS | o::MULTILINE,
        o::UTF | o::MATCH_INVALID_UTF,
        o::CASELESS | o::MULTILINE | o::DOTALL | o::EXTENDED,
    ];
    let xoptsets: &[u32] = &[
        0,
        o::X_CASELESS_RESTRICT,
        o::X_ASCII_BSD | o::X_ASCII_BSS | o::X_ASCII_BSW,
        o::X_ASCII_POSIX | o::X_ASCII_DIGIT,
        o::X_MATCH_WORD,
        o::X_MATCH_LINE,
        o::X_ESCAPED_CR_IS_LF,
        o::X_ALLOW_LOOKAROUND_BSK,
        o::X_BAD_ESCAPE_IS_LITERAL,
        o::X_ALT_BSUX,
        o::X_PYTHON_OCTAL,
        o::X_NO_BS0,
    ];
    let newlines: &[u32] = &[0, 1, 2, 3, 4, 5, 6];
    let bsrs: &[u32] = &[0, 1, 2];
    let optimizes: &[&[u32]] = &[
        &[],
        &[o::OPTIMIZATION_NONE],
        &[o::OPTIMIZATION_FULL],
        &[o::AUTO_POSSESS_OFF],
        &[o::START_OPTIMIZE_OFF],
        &[o::DOTSTAR_ANCHOR_OFF],
    ];
    let mopts: &[u32] = &[
        0,
        o::NOTBOL,
        o::NOTEOL,
        o::NOTEMPTY,
        o::NOTEMPTY_ATSTART,
        o::PARTIAL_SOFT,
        o::PARTIAL_HARD,
        o::ANCHORED,
        o::ENDANCHORED,
        o::COPY_MATCHED_SUBJECT,
        o::DISABLE_RECURSELOOP_CHECK,
        o::NOTBOL | o::NOTEOL | o::NOTEMPTY,
    ];

    let mut compiled = 0usize;
    for _ in 0..iters {
        let mut pat = Vec::new();
        let mut ngroups = 0u32;
        let depth = 2 + rng.below(3);
        gen_pat(&mut rng, depth, &mut ngroups, &mut pat);
        if pat.is_empty() {
            continue;
        }
        let opts = *rng.pick(optsets);
        let xopts = *rng.pick(xoptsets);
        let nl = *rng.pick(newlines);
        let bsr = *rng.pick(bsrs);
        let optz = *rng.pick(optimizes);

        unsafe {
            let cc = (p.c.compile_context_create)(std::ptr::null_mut());
            let cr = (p.r.compile_context_create)(std::ptr::null_mut());
            if xopts != 0 {
                assert_eq!(
                    (p.c.set_compile_extra_options)(cc, xopts),
                    (p.r.set_compile_extra_options)(cr, xopts)
                );
            }
            if nl != 0 {
                assert_eq!((p.c.set_newline)(cc, nl), (p.r.set_newline)(cr, nl));
            }
            if bsr != 0 {
                assert_eq!((p.c.set_bsr)(cc, bsr), (p.r.set_bsr)(cr, bsr));
            }
            for &d in optz {
                assert_eq!((p.c.set_optimize)(cc, d), (p.r.set_optimize)(cr, d));
            }

            let mut ec = 0i32;
            let mut eo = 0usize;
            let mut ec2 = 0i32;
            let mut eo2 = 0usize;
            let trace = std::env::var_os("PCRE2_TRACE").is_some();
            if trace {
                eprintln!(
                    "[trace] pat={:?} opts={:#x} xopts={:#x} nl={} bsr={} optz={:?}",
                    String::from_utf8_lossy(&pat), opts, xopts, nl, bsr, optz
                );
            }
            let a = (p.c.compile)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, cc);
            let b = (p.r.compile)(pat.as_ptr(), pat.len(), opts, &mut ec2, &mut eo2, cr);
            let label = format!(
                "sfuzz|{:?}|opts={:#x}|xopts={:#x}|nl={}|bsr={}|optz={:?}",
                String::from_utf8_lossy(&pat), opts, xopts, nl, bsr, optz
            );
            assert_eq!((a.is_null(), ec, eo), (b.is_null(), ec2, eo2), "compile [{}]", label);
            if !a.is_null() {
                compiled += 1;
                let cp = CodePair { c: a, r: b };
                cmp_all_pattern_info(p, &cp, &label);
                cmp_compiled_bytes(p, &cp, &label);

                // Matching. Skip the documented-UB combinations (invalid UTF with
                // NO_UTF_CHECK is excluded because NO_UTF_CHECK is not in `mopts`).
                let mut allopts: u32 = 0;
                (p.c.pattern_info)(cp.c, info::ALLOPTIONS, &mut allopts as *mut _ as *mut c_void);
                // Bounded heap limit: keeps pathological recursive patterns from
                // asking for gigabytes (both libraries hold a match_data at the
                // same time, so an unbounded limit makes the *second* allocation
                // fail for machine-dependent reasons). The unbounded default is
                // covered by `match_limits_produce_same_error`.
                let mctx_c = (p.c.match_context_create)(std::ptr::null_mut());
                let mctx_r = (p.r.match_context_create)(std::ptr::null_mut());
                assert_eq!(
                    (p.c.set_heap_limit)(mctx_c, 65536),
                    (p.r.set_heap_limit)(mctx_r, 65536)
                );
                // Bounded match/depth limits keep any single generated pattern from
                // dominating the run (the generator readily produces recursive
                // patterns such as `(?<g1>(?:x|(?1)))`). Both libraries get the
                // same limits, so the comparison is unaffected; the unbounded
                // defaults are covered by `cfg_default` and
                // `match_limits_produce_same_error`.
                assert_eq!(
                    (p.c.set_match_limit)(mctx_c, 200_000),
                    (p.r.set_match_limit)(mctx_r, 200_000)
                );
                assert_eq!(
                    (p.c.set_depth_limit)(mctx_c, 10_000),
                    (p.r.set_depth_limit)(mctx_r, 10_000)
                );
                for _ in 0..3 {
                    let subj = gen_subj(&mut rng);
                    let mo = *rng.pick(mopts);
                    let start = if subj.is_empty() { 0 } else { rng.below(subj.len() + 1) };
                    let mdc = (p.c.match_data_create_from_pattern)(cp.c, std::ptr::null_mut());
                    let mdr = (p.r.match_data_create_from_pattern)(cp.r, std::ptr::null_mut());
                    let sp = if subj.is_empty() { std::ptr::null() } else { subj.as_ptr() };
                    if trace {
                        eprintln!("[trace]   subj={:02x?} start={} mo={:#x}", subj, start, mo);
                    }
                    if c_crashes_on_invalid_utf(p, &cp, &subj, subj.len(), start) {
                        (p.c.match_data_free)(mdc);
                        (p.r.match_data_free)(mdr);
                        continue;
                    }
                    let x = (p.c.pcre2_match)(cp.c, sp, subj.len(), start, mo, mdc, mctx_c);
                    let y = (p.r.pcre2_match)(cp.r, sp, subj.len(), start, mo, mdr, mctx_r);
                    assert_eq!(
                        x, y,
                        "match rc [{}] subj={:02x?} start={} mo={:#x}",
                        label, subj, start, mo
                    );
                    let cnt = (p.c.get_ovector_count)(mdc);
                    assert_eq!(cnt, (p.r.get_ovector_count)(mdr), "ovector count [{}]", label);
                    let defined = if x > 0 {
                        x as usize
                    } else if x == 0 {
                        cnt as usize
                    } else if x == err::PARTIAL {
                        1
                    } else {
                        0
                    };
                    let oc = std::slice::from_raw_parts((p.c.get_ovector_pointer)(mdc), defined * 2);
                    let or = std::slice::from_raw_parts((p.r.get_ovector_pointer)(mdr), defined * 2);
                    assert_eq!(
                        oc, or,
                        "ovector [{}] subj={:02x?} start={} mo={:#x}",
                        label, subj, start, mo
                    );
                    if x >= 0 || x == err::PARTIAL || x == err::NOMATCH {
                        let mc = (p.c.get_mark)(mdc);
                        let mr = (p.r.get_mark)(mdr);
                        assert_eq!(mc.is_null(), mr.is_null(), "mark [{}]", label);
                        if !mc.is_null() {
                            let l1 = (p.c.priv_strlen)(mc);
                            let l2 = (p.r.priv_strlen)(mr);
                            assert_eq!(l1, l2, "mark len [{}]", label);
                            assert_eq!(
                                std::slice::from_raw_parts(mc, l1),
                                std::slice::from_raw_parts(mr, l2),
                                "mark bytes [{}]",
                                label
                            );
                        }
                    }
                    if x >= 0 || x == err::PARTIAL {
                        assert_eq!(
                            (p.c.get_startchar)(mdc),
                            (p.r.get_startchar)(mdr),
                            "startchar [{}]",
                            label
                        );
                    }
                    // DFA engine on the same input.
                    let mut ws = [0i32; 512];
                    let mut ws2 = [0i32; 512];
                    for extra in [0u32, o::DFA_SHORTEST] {
                        let x = (p.c.dfa_match)(
                            cp.c, sp, subj.len(), start, mo | extra, mdc, mctx_c,
                            ws.as_mut_ptr(), 512,
                        );
                        let y = (p.r.dfa_match)(
                            cp.r, sp, subj.len(), start, mo | extra, mdr, mctx_r,
                            ws2.as_mut_ptr(), 512,
                        );
                        assert_eq!(
                            x, y,
                            "dfa rc [{}] subj={:02x?} start={} mo={:#x} extra={:#x}",
                            label, subj, start, mo, extra
                        );
                    }
                    // Substitution on the same input.
                    for sopt in [0u32, o::SUBSTITUTE_GLOBAL, o::SUBSTITUTE_EXTENDED, o::SUBSTITUTE_LITERAL] {
                        let repl: &[u8] = rng.pick(&[&b"X"[..], b"$0", b"$1", b"[$1|$2]", b""]);
                        let mut b1 = vec![0xCDu8; 512];
                        let mut b2 = vec![0xCDu8; 512];
                        let mut l1: Sz = 256;
                        let mut l2: Sz = 256;
                        let x = (p.c.substitute)(
                            cp.c, sp, subj.len(), start, sopt, std::ptr::null_mut(),
                            mctx_c, repl.as_ptr(), repl.len(), b1.as_mut_ptr(), &mut l1,
                        );
                        let y = (p.r.substitute)(
                            cp.r, sp, subj.len(), start, sopt, std::ptr::null_mut(),
                            mctx_r, repl.as_ptr(), repl.len(), b2.as_mut_ptr(), &mut l2,
                        );
                        assert_eq!(
                            (x, l1),
                            (y, l2),
                            "substitute [{}] subj={:02x?} sopt={:#x} repl={:?}",
                            label, subj, sopt, String::from_utf8_lossy(repl)
                        );
                        assert_eq!(b1, b2, "substitute buffer [{}]", label);
                    }
                    (p.c.match_data_free)(mdc);
                    (p.r.match_data_free)(mdr);
                }
                (p.c.match_context_free)(mctx_c);
                (p.r.match_context_free)(mctx_r);
                free_code_pair(p, cp);
            }
            (p.c.compile_context_free)(cc);
            (p.r.compile_context_free)(cr);
        }
    }
    eprintln!("structured fuzz: {} of {} generated patterns compiled", compiled, iters);
    assert!(
        compiled * 3 > iters,
        "structured generator produced only {} valid patterns out of {}",
        compiled,
        iters
    );
}
