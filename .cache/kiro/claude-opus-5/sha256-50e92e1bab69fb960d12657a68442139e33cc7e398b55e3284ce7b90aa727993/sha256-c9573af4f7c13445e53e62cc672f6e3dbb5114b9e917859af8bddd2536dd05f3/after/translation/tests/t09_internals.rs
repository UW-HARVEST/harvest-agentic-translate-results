//! Phase B: internal helpers driven through real compiled patterns.
//! CONFIGS.md rows 98-106 (xclass/eclass, find_bracket, study, auto-possess,
//! class bitmaps, name-table helpers, nested class compilation).
mod harness;
use harness::*;
use std::ffi::c_void;
use std::os::raw::c_int;

// Field offsets in `pcre2_real_code` (c_src/src/pcre2_intmodedep.h):
//   0..24  memctl (2 fn pointers + data pointer)
//   24     tables pointer
//   32     executable_jit pointer
//   40..72 start_bitmap[32]
//   72     blocksize
//   80     code_start
//   88     magic_number
//   92.. compile_options, overall_options, extra_options, flags, limits,
//        first/last code unit, bsr, newline, max_lookbehind, minlength,
//        top_bracket, top_backref, name_entry_size, name_count,
//        optimization_flags, then the name table and the byte code.
const OFF_START_BITMAP: usize = 40;
const OFF_BLOCKSIZE: usize = 72;
const OFF_CODE_START: usize = 80;

unsafe fn blocksize(code: Code) -> usize {
    unsafe { *((code as *const u8).add(OFF_BLOCKSIZE) as *const usize) }
}
unsafe fn code_start(code: Code) -> usize {
    unsafe { *((code as *const u8).add(OFF_CODE_START) as *const usize) }
}

/// The whole compiled block minus the three host pointers at its head. This
/// covers the start bitmap, every scalar field, the name table and the complete
/// byte code, so it validates the compiler, `_pcre2_study_8`,
/// `_pcre2_auto_possessify_8`, `_pcre2_update_classbits_8`, the
/// `_pcre2_compile_class_*` encoders and the name-table helpers in one shot.
unsafe fn comparable_block(code: Code) -> Vec<u8> {
    let n = unsafe { blocksize(code) };
    assert!(n > OFF_START_BITMAP && n < 1 << 24, "implausible blocksize {n}");
    unsafe { std::slice::from_raw_parts((code as *const u8).add(OFF_START_BITMAP), n - OFF_START_BITMAP) }
        .to_vec()
}

fn compiled_blocks(pat: &[u8], options: u32, xoptions: u32) -> Option<(Vec<u8>, Vec<u8>, usize)> {
    let mut out = Vec::new();
    let mut cstart = 0usize;
    for api in [c(), r()] {
        unsafe {
            let cc = (api.compile_context_create)(std::ptr::null_mut());
            (api.set_compile_extra_options)(cc, xoptions);
            let mut err = 0;
            let mut off = 0;
            let code = (api.compile)(pat.as_ptr(), pat.len(), options, &mut err, &mut off, cc);
            if code.is_null() {
                (api.compile_context_free)(cc);
                return None;
            }
            cstart = code_start(code);
            out.push(comparable_block(code));
            (api.code_free)(code);
            (api.compile_context_free)(cc);
        }
    }
    Some((out.remove(0), out.remove(0), cstart))
}

// -------------------------------------- rows 100-106: byte-code level identity
#[test]
fn compiled_byte_code_identical() {
    let opts = [
        0u32,
        PCRE2_UTF,
        PCRE2_UTF | PCRE2_UCP,
        PCRE2_UCP,
        PCRE2_CASELESS,
        PCRE2_UTF | PCRE2_CASELESS,
        PCRE2_DUPNAMES,
        PCRE2_NO_AUTO_POSSESS,
        PCRE2_NO_DOTSTAR_ANCHOR,
        PCRE2_NO_START_OPTIMIZE,
        PCRE2_ALT_EXTENDED_CLASS,
        PCRE2_EXTENDED,
        PCRE2_MULTILINE | PCRE2_DOTALL,
        PCRE2_AUTO_CALLOUT,
        PCRE2_NO_AUTO_CAPTURE,
        PCRE2_LITERAL,
        PCRE2_ANCHORED,
        PCRE2_ENDANCHORED,
    ];
    let xopts = [
        0u32,
        PCRE2_EXTRA_ASCII_BSD | PCRE2_EXTRA_ASCII_BSW | PCRE2_EXTRA_ASCII_BSS,
        PCRE2_EXTRA_CASELESS_RESTRICT,
        PCRE2_EXTRA_MATCH_WORD,
        PCRE2_EXTRA_MATCH_LINE,
        PCRE2_EXTRA_ASCII_POSIX | PCRE2_EXTRA_ASCII_DIGIT,
    ];
    let mut compared = 0usize;
    for p in curated_patterns() {
        for &o in &opts {
            for &x in &xopts {
                if let Some((cb_, rb, _)) = compiled_blocks(p.as_bytes(), o, x) {
                    if cb_ != rb {
                        let i = cb_.iter().zip(&rb).position(|(a, b)| a != b);
                        panic!(
                            "BYTE CODE DIVERGENCE pat={p:?} options={o:#x} xoptions={x:#x}\n first differing byte index (from start_bitmap) = {i:?}\n C   len={} Rust len={}\n C   ={:02x?}\n Rust={:02x?}",
                            cb_.len(),
                            rb.len(),
                            &cb_[..cb_.len().min(160)],
                            &rb[..rb.len().min(160)],
                        );
                    }
                    compared += 1;
                }
            }
        }
    }
    // randomized patterns too
    let mut rng = Rng::new(0x5EED_B17E);
    for _ in 0..20000 {
        let d = rng.range(1, 3) as u32;
        let p = random_pattern(&mut rng, d);
        let o = *rng.pick(&opts);
        let x = *rng.pick(&xopts);
        if let Some((cb_, rb, _)) = compiled_blocks(p.as_bytes(), o, x) {
            if cb_ != rb {
                let i = cb_.iter().zip(&rb).position(|(a, b)| a != b);
                panic!(
                    "BYTE CODE DIVERGENCE pat={p:?} options={o:#x} xoptions={x:#x}\n first differing byte index = {i:?}\n C   ={:02x?}\n Rust={:02x?}",
                    cb_, rb
                );
            }
            compared += 1;
        }
    }
    eprintln!("compiled byte-code blocks compared: {compared}");
    assert!(compared > 4000);
}

// ------------------------------------------------------------------- row 99
#[test]
fn find_bracket() {
    let pats = [
        "(a)",
        "(a)(b)(c)",
        "(?<n>a)(?:b)(c)",
        "((((a))))",
        "(a)|(b)|(c)",
        "(?|(a)|(b))",
        "(a(b(c(d))))",
        "abc",
        "(?:a)",
        "(a)(?<x>b)(?'y'c)(?P<z>d)",
        "(a){0}(b)",
        "\\x{263a}(a)\\p{L}(b)",
        "(?<n>a)(?&n)",
        "[a-z](a)[^b](b)",
    ];
    for p in pats {
        // `utf` must agree with the mode the pattern was compiled in; otherwise
        // the scan mis-decodes character lengths and runs off the byte code.
        for options in [0u32, PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_DUPNAMES] {
            {
                let utf: c_int = if options & PCRE2_UTF != 0 { 1 } else { 0 };
                let mut results = Vec::new();
                for api in [c(), r()] {
                    unsafe {
                        let mut err = 0;
                        let mut off = 0;
                        let pb = p.as_bytes();
                        let code = (api.compile)(pb.as_ptr(), pb.len(), options, &mut err,
                                                 &mut off, std::ptr::null_mut());
                        if code.is_null() {
                            results.push(Vec::new());
                            continue;
                        }
                        let start = (code as *const u8).add(code_start(code));
                        let mut v = Vec::new();
                        for n in 0..12i32 {
                            let q = (api.priv_find_bracket)(start, utf, n);
                            v.push(if q.is_null() {
                                -1i64
                            } else {
                                q.offset_from(start) as i64
                            });
                        }
                        // negative / huge bracket numbers
                        for n in [-1i32, -5, 1000, i32::MAX] {
                            let q = (api.priv_find_bracket)(start, utf, n);
                            v.push(if q.is_null() {
                                -1i64
                            } else {
                                q.offset_from(start) as i64
                            });
                        }
                        results.push(v);
                        (api.code_free)(code);
                    }
                }
                assert_eq!(
                    results[0], results[1],
                    "find_bracket divergence for {p:?} utf={utf} options={options:#x}"
                );
            }
        }
    }
}

// ------------------------------------------------------------------- row 100
#[test]
fn study_recomputes_identically() {
    // _pcre2_study_8 is normally called once by pcre2_compile. Calling it again
    // on the finished code must recompute the same first/last code unit, start
    // bitmap, minimum length and flags in both libraries.
    let pats = [
        "abc", "a|b", ".*abc", "^abc", "(?i)abc", "\\d+", "[a-z]+", "(?:ab|cd)ef",
        "a{3,}b", "(a)(b)(c)", "\\p{L}+", "(?s).*", "x*y", "(?=a)b", "(?!a)b",
        "(?<=a)b", "(a+)+b", "(?R)?a", "\\b\\w+\\b", "", "a", "[^\\x00-\\x7f]+",
        "\\x{1000}abc", "(?m)^x$", "(*NO_START_OPT)abc", "\\Kabc", "(?|(a)|(b))c",
    ];
    for p in pats {
        for options in [
            0u32,
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_CASELESS,
            PCRE2_MULTILINE,
            PCRE2_DOTALL,
            PCRE2_ANCHORED,
            PCRE2_NO_START_OPTIMIZE,
        ] {
            let mut results = Vec::new();
            for api in [c(), r()] {
                unsafe {
                    let mut err = 0;
                    let mut off = 0;
                    let pb = p.as_bytes();
                    let code = (api.compile)(pb.as_ptr(), pb.len(), options, &mut err, &mut off,
                                             std::ptr::null_mut());
                    if code.is_null() {
                        results.push((0, Vec::new(), Vec::new()));
                        continue;
                    }
                    let before = comparable_block(code);
                    let rc = (api.priv_study)(code);
                    let after = comparable_block(code);
                    results.push((rc, before, after));
                    (api.code_free)(code);
                }
            }
            assert_eq!(
                results[0].0, results[1].0,
                "study rc differs for {p:?} options={options:#x}"
            );
            assert_eq!(
                results[0].1, results[1].1,
                "block before study differs for {p:?} options={options:#x}"
            );
            assert_eq!(
                results[0].2, results[1].2,
                "block after study differs for {p:?} options={options:#x}"
            );
        }
    }
}

// ------------------------------------------- rows 98, 102: wide/extended classes
/// Drive `_pcre2_xclass_8`, `_pcre2_eclass_8` and `_pcre2_update_classbits_8`
/// through the public API over the whole code-point range.
#[test]
fn wide_and_extended_classes() {
    let class_pats = [
        "^[\\x{100}-\\x{200}]$",
        "^[^\\x{100}-\\x{200}]$",
        "^[\\p{L}]$",
        "^[^\\p{L}]$",
        "^[\\p{Nd}\\p{Lu}]$",
        "^[\\p{Greek}\\p{Han}]$",
        "^[a-z\\x{400}-\\x{4ff}]$",
        "^[[:alpha:]\\x{2000}]$",
        "^[\\p{Xan}]$",
        "^[\\p{Xsp}\\p{Xps}]$",
        "^[\\p{Xwd}]$",
        "^[\\x{10000}-\\x{10fff}]$",
        "^[\\p{Any}]$",
        "^[\\p{L}&&\\p{Lu}]$",
        "^[[a-z]&&[b-d]]$",
        "^[[\\p{L}]--[a-z]]$",
        "^[[a-f]~~[d-k]]$",
        "^[[:digit:]||[a-c]]$",
        "^[^[a-z]&&[b-d]]$",
        "^\\X$",
        "^[\\p{Cased}]$",
        "^[\\p{Changes_When_Casefolded}]$",
        "^[\\p{Bidi_Class:L}]$",
    ];
    let mut cps: Vec<u32> = vec![
        0, 1, 0x40, 0x41, 0x5a, 0x61, 0x7a, 0x7f, 0x80, 0xff, 0x100, 0x1ff, 0x200, 0x201,
        0x3ff, 0x400, 0x4ff, 0x500, 0x660, 0x1000, 0x2000, 0x2028, 0x3000, 0xd7ff, 0xe000,
        0xffff, 0x10000, 0x10fff, 0x11000, 0x1f600, 0x10ffff,
    ];
    let mut rng = Rng::new(0x5EED_C1A5);
    for _ in 0..3000 {
        let cp = (rng.next_u64() % 0x110000) as u32;
        if !(0xd800..=0xdfff).contains(&cp) {
            cps.push(cp);
        }
    }
    for p in class_pats {
        for options in [
            PCRE2_UTF,
            PCRE2_UTF | PCRE2_UCP,
            PCRE2_UTF | PCRE2_CASELESS,
            PCRE2_UTF | PCRE2_UCP | PCRE2_CASELESS,
            PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS,
            PCRE2_UTF | PCRE2_UCP | PCRE2_ALT_EXTENDED_CLASS,
        ] {
            // compile once per library, then probe every code point
            let mut ok = true;
            let mut codes: Vec<Code> = Vec::new();
            let mut mds: Vec<MatchData> = Vec::new();
            for api in [c(), r()] {
                unsafe {
                    let mut err = 0;
                    let mut off = 0;
                    let pb = p.as_bytes();
                    let code = (api.compile)(pb.as_ptr(), pb.len(), options, &mut err, &mut off,
                                             std::ptr::null_mut());
                    if code.is_null() {
                        ok = false;
                    } else {
                        mds.push((api.match_data_create_from_pattern)(code, std::ptr::null_mut()));
                    }
                    codes.push(code);
                }
            }
            if !ok {
                for (api, code) in [c(), r()].into_iter().zip(&codes) {
                    if !code.is_null() {
                        unsafe { (api.code_free)(*code) };
                    }
                }
                continue;
            }
            for &cp in &cps {
                let ch = match char::from_u32(cp) {
                    Some(ch) => ch,
                    None => continue,
                };
                let mut buf = [0u8; 4];
                let s = ch.encode_utf8(&mut buf).as_bytes();
                let mut res = Vec::new();
                for (i, api) in [c(), r()].into_iter().enumerate() {
                    unsafe {
                        let rc = (api.do_match)(codes[i], s.as_ptr(), s.len(), 0, 0, mds[i],
                                                std::ptr::null_mut());
                        let n = (api.get_ovector_count)(mds[i]);
                        let ov = if rc > 0 {
                            std::slice::from_raw_parts(
                                (api.get_ovector_pointer)(mds[i]),
                                (rc as usize).min(n as usize) * 2,
                            )
                            .to_vec()
                        } else {
                            Vec::new()
                        };
                        res.push((rc, ov));
                    }
                }
                assert_eq!(
                    res[0], res[1],
                    "class divergence pat={p:?} options={options:#x} cp={cp:#x}"
                );
            }
            // Also probe with the interpreter's DFA sibling.
            for &cp in cps.iter().take(300) {
                let ch = match char::from_u32(cp) {
                    Some(ch) => ch,
                    None => continue,
                };
                let mut buf = [0u8; 4];
                let s = ch.encode_utf8(&mut buf).as_bytes();
                let mut res = Vec::new();
                for (i, api) in [c(), r()].into_iter().enumerate() {
                    unsafe {
                        let mut ws = [0i32; 128];
                        let rc = (api.dfa_match)(codes[i], s.as_ptr(), s.len(), 0, 0, mds[i],
                                                 std::ptr::null_mut(), ws.as_mut_ptr(), 128);
                        res.push(rc);
                    }
                }
                assert_eq!(
                    res[0], res[1],
                    "DFA class divergence pat={p:?} options={options:#x} cp={cp:#x}"
                );
            }
            for (i, api) in [c(), r()].into_iter().enumerate() {
                unsafe {
                    (api.match_data_free)(mds[i]);
                    (api.code_free)(codes[i]);
                }
            }
        }
    }
}

// ---------------------------------------------------- rows 104, 106: name tables
#[test]
fn name_table_helpers() {
    let pats = [
        "(?<a>x)",
        "(?<a>x)(?<b>y)",
        "(?<b>y)(?<a>x)",
        "(?<a>x)(?<ab>y)(?<abc>z)",
        "(?<abc>x)(?<ab>y)(?<a>z)",
        "(?<n>x)|(?<n>y)",
        "(?<n>x)|(?<n>y)|(?<n>z)",
        "(?<aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa>x)(?<aaaaaaaaaaaaaaaaaaaaaaaaaaaaab>y)",
        "(?<z0>a)(?<z1>b)(?<z2>c)(?<z3>d)(?<z4>e)(?<z5>f)(?<z6>g)(?<z7>h)(?<z8>i)(?<z9>j)",
        "(?<n>a)(?&n)(?P>n)\\k<n>\\g{n}",
        "(?<n>a)(?(<n>)x|y)",
        "(?'q'a)(?P<r>b)(?<s>c)",
        "(?|(?<n>a)|(?<n>b))",
    ];
    for p in pats {
        for options in [0u32, PCRE2_DUPNAMES, PCRE2_UTF, PCRE2_DUPNAMES | PCRE2_UTF] {
            let mut results = Vec::new();
            for api in [c(), r()] {
                unsafe {
                    let mut err = 0;
                    let mut off = 0;
                    let pb = p.as_bytes();
                    let code = (api.compile)(pb.as_ptr(), pb.len(), options, &mut err, &mut off,
                                             std::ptr::null_mut());
                    if code.is_null() {
                        results.push((err, off, Vec::new(), Vec::new()));
                        continue;
                    }
                    let mut ncount: u32 = 0;
                    let mut nsize: u32 = 0;
                    (api.pattern_info)(code, 17, &mut ncount as *mut u32 as *mut c_void);
                    (api.pattern_info)(code, 18, &mut nsize as *mut u32 as *mut c_void);
                    let mut nt: *const u8 = std::ptr::null();
                    (api.pattern_info)(code, 19, &mut nt as *mut *const u8 as *mut c_void);
                    let table = if nt.is_null() {
                        Vec::new()
                    } else {
                        std::slice::from_raw_parts(nt, (ncount * nsize) as usize).to_vec()
                    };
                    let mut lookups: Vec<(String, c_int, i64, i64)> = Vec::new();
                    for nm in [
                        "a", "b", "ab", "abc", "n", "q", "r", "s", "z0", "z9", "nope", "A",
                        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaab",
                    ] {
                        let cn = cs(nm);
                        let num = (api.substring_number_from_name)(code, cn.as_ptr());
                        let mut first: *const u8 = std::ptr::null();
                        let mut last: *const u8 = std::ptr::null();
                        let scan = (api.substring_nametable_scan)(code, cn.as_ptr(), &mut first,
                                                                 &mut last);
                        let (fo, lo) = if scan >= 0 && !nt.is_null() && !first.is_null() {
                            (first.offset_from(nt) as i64, last.offset_from(nt) as i64)
                        } else {
                            (-1, -1)
                        };
                        lookups.push((nm.to_string(), num, fo, lo));
                        let _ = scan;
                    }
                    results.push((err, off, table, lookups));
                    (api.code_free)(code);
                }
            }
            assert!(
                results[0] == results[1],
                "name-table divergence pat={p:?} options={options:#x}\n C   ={:?}\n Rust={:?}",
                results[0],
                results[1]
            );
        }
    }
}
