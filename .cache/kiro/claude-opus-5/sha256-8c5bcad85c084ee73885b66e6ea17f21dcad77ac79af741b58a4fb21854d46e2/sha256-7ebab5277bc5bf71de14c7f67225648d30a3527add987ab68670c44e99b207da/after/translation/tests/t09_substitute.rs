//! `pcre2_substitute.c`.
mod common;

use common::*;
use std::ffi::c_void;

type Substitute = unsafe extern "C" fn(
    *const c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    PCRE2_SIZE,
    u32,
    *mut c_void,
    *mut c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    *mut PCRE2_UCHAR,
    *mut PCRE2_SIZE,
) -> i32;
type MdCreate = unsafe extern "C" fn(u32, *mut c_void) -> *mut c_void;
type MdFree = unsafe extern "C" fn(*mut c_void);

const PATTERNS: &[&[u8]] = &[
    b"a",
    b"abc",
    b"(a)(b)",
    b"(?<n>a)",
    b"(?<one>a)(?<two>b)?",
    b"\\d+",
    b"a*",
    b"",
    b"(a)|(b)",
    b"[aeiou]",
    b"(.)",
    b"^",
    b"$",
    b"\\b",
    b"(?i)abc",
    b"x(?=y)",
    b"(\\w+)\\s(\\w+)",
];

const SUBJECTS: &[&[u8]] = &[
    b"",
    b"a",
    b"abc",
    b"abcabc",
    b"aaa",
    b"xyz",
    b"hello world",
    b"a1b22c333",
    b"ab",
    b"AeIoU",
    b"xy",
    b"line1\nline2",
];

const REPLACEMENTS: &[&[u8]] = &[
    b"",
    b"X",
    b"[$0]",
    b"$1-$2",
    b"${1}${2}",
    b"$name",
    b"${n}",
    b"${one}/${two}",
    b"$99",
    b"$",
    b"$$",
    b"\\$",
    b"\\n",
    b"a\\Ub\\Ec",
    b"\\U$0\\E",
    b"\\L$0\\E",
    b"\\u$0",
    b"\\l$0",
    b"${1:-default}",
    b"${1:+set:unset}",
    b"${99:-d}",
    b"\\x41",
    b"\\{",
    b"${",
    b"${1",
    b"$(1)",
    b"\\",
    b"a\\0b",
    b"<$0>",
];

const OPTION_SETS: &[u32] = &[
    0,
    PCRE2_SUBSTITUTE_GLOBAL,
    PCRE2_SUBSTITUTE_EXTENDED,
    PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED,
    PCRE2_SUBSTITUTE_UNSET_EMPTY,
    PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
    PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
    PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
    PCRE2_SUBSTITUTE_LITERAL,
    PCRE2_SUBSTITUTE_LITERAL | PCRE2_SUBSTITUTE_GLOBAL,
    PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
    PCRE2_SUBSTITUTE_REPLACEMENT_ONLY | PCRE2_SUBSTITUTE_GLOBAL,
    PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNSET_EMPTY,
    PCRE2_NOTBOL,
    PCRE2_NOTEOL,
    PCRE2_NOTEMPTY,
    PCRE2_NOTEMPTY_ATSTART,
    PCRE2_ANCHORED,
    PCRE2_SUBSTITUTE_GLOBAL | PCRE2_SUBSTITUTE_EXTENDED | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
];

fn md_pair(n: u32) -> (*mut c_void, *mut c_void) {
    let (cc, rc) = both::<MdCreate>("pcre2_match_data_create_8");
    unsafe { (cc(n, std::ptr::null_mut()), rc(n, std::ptr::null_mut())) }
}

#[test]
fn substitute_matches() {
    let (sc, sr) = both::<Substitute>("pcre2_substitute_8");
    let (mdfc, mdfr) = both::<MdFree>("pcre2_match_data_free_8");
    let (mdc, mdr) = md_pair(16);
    for pat in PATTERNS {
        for &copts in &[0u32, PCRE2_CASELESS, PCRE2_UTF, PCRE2_DUPNAMES] {
            let Some(pair) = compile_both(pat, copts) else {
                continue;
            };
            for subj in SUBJECTS {
                for rep in REPLACEMENTS {
                    for &opts in OPTION_SETS {
                        for outsz in [0usize, 1, 4, 16, 256] {
                            let mut bc = vec![0xAAu8; 512];
                            let mut br = vec![0xAAu8; 512];
                            let mut la = outsz;
                            let mut lb = outsz;
                            unsafe {
                                let x = sc(
                                    pair.c,
                                    subj.as_ptr(),
                                    subj.len(),
                                    0,
                                    opts,
                                    mdc,
                                    std::ptr::null_mut(),
                                    rep.as_ptr(),
                                    rep.len(),
                                    bc.as_mut_ptr(),
                                    &mut la,
                                );
                                let y = sr(
                                    pair.r,
                                    subj.as_ptr(),
                                    subj.len(),
                                    0,
                                    opts,
                                    mdr,
                                    std::ptr::null_mut(),
                                    rep.as_ptr(),
                                    rep.len(),
                                    br.as_mut_ptr(),
                                    &mut lb,
                                );
                                let label = format!(
                                    "substitute pat={pat:02x?} subj={subj:02x?} rep={rep:02x?} opts={opts:#x} copts={copts:#x} outsz={outsz}"
                                );
                                assert_eq!(x, y, "{label}: rc");
                                assert_eq!(la, lb, "{label}: length");
                                assert_bytes_eq(&format!("{label}: buffer"), &bc, &br);
                            }
                        }
                    }
                }
            }
        }
    }
    unsafe {
        mdfc(mdc);
        mdfr(mdr);
    }
}

#[test]
fn substitute_zero_terminated_and_null_matchdata() {
    let (sc, sr) = both::<Substitute>("pcre2_substitute_8");
    for pat in PATTERNS {
        let Some(pair) = compile_both(pat, 0) else {
            continue;
        };
        for subj in SUBJECTS {
            let mut zs = subj.to_vec();
            zs.push(0);
            for rep in REPLACEMENTS {
                let mut zr = rep.to_vec();
                zr.push(0);
                for &opts in &[0u32, PCRE2_SUBSTITUTE_GLOBAL, PCRE2_SUBSTITUTE_EXTENDED] {
                    let mut bc = vec![0xAAu8; 512];
                    let mut br = vec![0xAAu8; 512];
                    let mut la = 256usize;
                    let mut lb = 256usize;
                    unsafe {
                        let x = sc(
                            pair.c,
                            zs.as_ptr(),
                            PCRE2_ZERO_TERMINATED,
                            0,
                            opts,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            zr.as_ptr(),
                            PCRE2_ZERO_TERMINATED,
                            bc.as_mut_ptr(),
                            &mut la,
                        );
                        let y = sr(
                            pair.r,
                            zs.as_ptr(),
                            PCRE2_ZERO_TERMINATED,
                            0,
                            opts,
                            std::ptr::null_mut(),
                            std::ptr::null_mut(),
                            zr.as_ptr(),
                            PCRE2_ZERO_TERMINATED,
                            br.as_mut_ptr(),
                            &mut lb,
                        );
                        let label = format!(
                            "zt substitute pat={pat:02x?} subj={subj:02x?} rep={rep:02x?} opts={opts:#x}"
                        );
                        assert_eq!(x, y, "{label}: rc");
                        assert_eq!(la, lb, "{label}: length");
                        assert_bytes_eq(&format!("{label}: buffer"), &bc, &br);
                    }
                }
            }
        }
    }
}

#[test]
fn substitute_bad_arguments() {
    let (sc, sr) = both::<Substitute>("pcre2_substitute_8");
    let Some(pair) = compile_both(b"(a)", 0) else {
        panic!()
    };
    let subj = b"aaa";
    let rep = b"X";
    unsafe {
        // NULL replacement (the C code substitutes an internal empty string).
        let mut la = 64usize;
        let mut lb = 64usize;
        let mut bc = vec![0xAAu8; 64];
        let mut br = vec![0xAAu8; 64];
        assert_eq!(
            sc(pair.c, subj.as_ptr(), 3, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(),
               std::ptr::null(), 0, bc.as_mut_ptr(), &mut la),
            sr(pair.r, subj.as_ptr(), 3, 0, 0, std::ptr::null_mut(), std::ptr::null_mut(),
               std::ptr::null(), 0, br.as_mut_ptr(), &mut lb),
            "NULL replacement"
        );
        assert_eq!(la, lb);
        // Bad start offset and bad options.
        for start in [4usize, 100] {
            let mut la = 64usize;
            let mut lb = 64usize;
            assert_eq!(
                sc(pair.c, subj.as_ptr(), 3, start, 0, std::ptr::null_mut(), std::ptr::null_mut(),
                   rep.as_ptr(), 1, bc.as_mut_ptr(), &mut la),
                sr(pair.r, subj.as_ptr(), 3, start, 0, std::ptr::null_mut(), std::ptr::null_mut(),
                   rep.as_ptr(), 1, br.as_mut_ptr(), &mut lb),
                "start {start}"
            );
        }
        for bad in [PCRE2_DFA_RESTART, 0xffff_ffff, PCRE2_PARTIAL_HARD] {
            let mut la = 64usize;
            let mut lb = 64usize;
            assert_eq!(
                sc(pair.c, subj.as_ptr(), 3, 0, bad, std::ptr::null_mut(), std::ptr::null_mut(),
                   rep.as_ptr(), 1, bc.as_mut_ptr(), &mut la),
                sr(pair.r, subj.as_ptr(), 3, 0, bad, std::ptr::null_mut(), std::ptr::null_mut(),
                   rep.as_ptr(), 1, br.as_mut_ptr(), &mut lb),
                "bad options {bad:#x}"
            );
        }
    }
}

#[test]
fn substitute_matched_option() {
    // PCRE2_SUBSTITUTE_MATCHED reuses an existing match in the match data.
    type Match = unsafe extern "C" fn(
        *const c_void,
        PCRE2_SPTR,
        PCRE2_SIZE,
        PCRE2_SIZE,
        u32,
        *mut c_void,
        *mut c_void,
    ) -> i32;
    let (mc, mr) = both::<Match>("pcre2_match_8");
    let (sc, sr) = both::<Substitute>("pcre2_substitute_8");
    let (mdfc, mdfr) = both::<MdFree>("pcre2_match_data_free_8");
    let (mdc, mdr) = md_pair(16);
    for pat in PATTERNS {
        let Some(pair) = compile_both(pat, 0) else {
            continue;
        };
        for subj in SUBJECTS {
            for rep in REPLACEMENTS {
                unsafe {
                    let a = mc(pair.c, subj.as_ptr(), subj.len(), 0, 0, mdc, std::ptr::null_mut());
                    let b = mr(pair.r, subj.as_ptr(), subj.len(), 0, 0, mdr, std::ptr::null_mut());
                    assert_eq!(a, b);
                    for &opts in &[
                        PCRE2_SUBSTITUTE_MATCHED,
                        PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_GLOBAL,
                        PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_EXTENDED,
                        PCRE2_SUBSTITUTE_MATCHED | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                    ] {
                        let mut bc = vec![0xAAu8; 512];
                        let mut br = vec![0xAAu8; 512];
                        let mut la = 256usize;
                        let mut lb = 256usize;
                        let x = sc(
                            pair.c, subj.as_ptr(), subj.len(), 0, opts, mdc,
                            std::ptr::null_mut(), rep.as_ptr(), rep.len(),
                            bc.as_mut_ptr(), &mut la,
                        );
                        let y = sr(
                            pair.r, subj.as_ptr(), subj.len(), 0, opts, mdr,
                            std::ptr::null_mut(), rep.as_ptr(), rep.len(),
                            br.as_mut_ptr(), &mut lb,
                        );
                        let label = format!(
                            "matched substitute pat={pat:02x?} subj={subj:02x?} rep={rep:02x?} opts={opts:#x}"
                        );
                        assert_eq!(x, y, "{label}: rc");
                        assert_eq!(la, lb, "{label}: length");
                        assert_bytes_eq(&format!("{label}: buffer"), &bc, &br);
                    }
                }
            }
        }
    }
    unsafe {
        mdfc(mdc);
        mdfr(mdr);
    }
}
