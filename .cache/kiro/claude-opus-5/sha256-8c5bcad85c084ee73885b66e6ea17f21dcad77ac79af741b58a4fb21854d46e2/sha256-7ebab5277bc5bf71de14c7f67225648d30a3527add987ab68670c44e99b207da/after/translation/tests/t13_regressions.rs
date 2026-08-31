//! Focused regression tests for the two divergences found during verification.
//!
//! 1. `pcre2_dfa_match`: the caseless repeat opcodes were detected with
//!    `codevalue >= OP_STARI`, but the opcode numbering puts `OP_NOTSTAR ..
//!    OP_NOTPOSUPTO` (59..=71) *above* `OP_STARI` (46), so caseful negated
//!    repeats such as `[^a]*` were treated as caseless and had their
//!    `codevalue` shifted into the wrong opcode range. That made the DFA add a
//!    spurious "repeat continues" state and skip the possessive
//!    `active_count--`.
//!
//! 2. `convert_posix`: the UTF flag was ignored and the input was always decoded
//!    as UTF-8, so in non-UTF mode multi-byte sequences were consumed as one
//!    character instead of byte by byte.
mod common;

use common::*;
use std::ffi::{c_int, c_void};

type DfaMatch = unsafe extern "C" fn(
    *const c_void,
    PCRE2_SPTR,
    PCRE2_SIZE,
    PCRE2_SIZE,
    u32,
    *mut c_void,
    *mut c_void,
    *mut c_int,
    PCRE2_SIZE,
) -> i32;
type MdCreate = unsafe extern "C" fn(u32, *mut c_void) -> *mut c_void;
type MdFree = unsafe extern "C" fn(*mut c_void);
type PatternConvert = unsafe extern "C" fn(
    PCRE2_SPTR,
    PCRE2_SIZE,
    u32,
    *mut *mut PCRE2_UCHAR,
    *mut PCRE2_SIZE,
    *mut c_void,
) -> i32;

/// The DFA state machine must evolve identically, not merely reach the same
/// answer: the workspace holds the live active/new state lists, so comparing it
/// after every call checks each step of the automaton.
#[test]
fn dfa_negated_repeat_state_lists_match() {
    let (dc, dr) = both::<DfaMatch>("pcre2_dfa_match_8");
    let (mdc, mdr) = both::<MdCreate>("pcre2_match_data_create_8");
    let (mdfc, mdfr) = both::<MdFree>("pcre2_match_data_free_8");

    // One pattern per affected opcode group (STAR, PLUS, QUERY, EXACT, UPTO),
    // caseful and caseless, greedy / lazy / possessive.
    let pats: &[&[u8]] = &[
        b"[^a]*a",
        b"[^a]+a",
        b"[^a]?a",
        b"[^a]{2}a",
        b"[^a]{1,3}a",
        b"[^a]*+a",
        b"[^a]++a",
        b"[^a]?+a",
        b"[^a]{1,3}+a",
        b"[^a]*?a",
        b"[^a]+?a",
        b"[^a]??a",
        b"[^a]{1,3}?a",
        b"(?i)[^a]*a",
        b"(?i)[^a]+a",
        b"(?i)[^a]?a",
        b"(?i)[^a]{2}a",
        b"(?i)[^a]{1,3}a",
        b"(?i)[^a]*+a",
        b"(?i)[^a]{1,3}+a",
        b"(?i)[^K]+x",
        b"[^\\x{100}]*a",
    ];
    let subs: &[&[u8]] = &[
        b"",
        b"a",
        b"A",
        b"aa",
        b"abc",
        b"bbba",
        b"BBBA",
        b"xyza",
        b"aaaa",
        b"kkkx",
        b"KKKx",
        b"\xc4\x80a",
    ];
    unsafe {
        let md_c = mdc(16, std::ptr::null_mut());
        let md_r = mdr(16, std::ptr::null_mut());
        for p in pats {
            for co in [0u32, PCRE2_UTF, PCRE2_UCP, PCRE2_UTF | PCRE2_UCP, PCRE2_CASELESS] {
                let Some(pair) = compile_both(p, co) else {
                    continue;
                };
                for s in subs {
                    for mo in [0u32, PCRE2_DFA_SHORTEST, PCRE2_PARTIAL_SOFT, PCRE2_ANCHORED] {
                        let mut ws_c = vec![0x5A5A_5A5Ai32; 400];
                        let mut ws_r = vec![0x5A5A_5A5Ai32; 400];
                        md_poison(md_c, 16);
                        md_poison(md_r, 16);
                        let x = dc(
                            pair.c, s.as_ptr(), s.len(), 0, mo, md_c,
                            std::ptr::null_mut(), ws_c.as_mut_ptr(), 400,
                        );
                        let y = dr(
                            pair.r, s.as_ptr(), s.len(), 0, mo, md_r,
                            std::ptr::null_mut(), ws_r.as_mut_ptr(), 400,
                        );
                        let label =
                            format!("dfa {p:02x?} / {s:02x?} co={co:#x} mo={mo:#x}");
                        assert_eq!(x, y, "{label}: rc");
                        assert_eq!(
                            md_snapshot(md_c, s.as_ptr(), s.len()),
                            md_snapshot(md_r, s.as_ptr(), s.len()),
                            "{label}: match data"
                        );
                        assert_eq!(ws_c, ws_r, "{label}: DFA state lists");
                    }
                }
            }
        }
        mdfc(md_c);
        mdfr(md_r);
    }
}

/// In non-UTF mode the POSIX converter must consume the input one code unit at a
/// time; the partial output left in a too-small caller-supplied buffer reveals
/// any difference in how far it got before reporting overflow.
#[test]
fn convert_posix_non_utf_is_bytewise() {
    let (pc, pr) = both::<PatternConvert>("pcre2_pattern_convert_8");
    let (fc, fr) = both::<ConvertedFree>("pcre2_converted_pattern_free_8");
    let pats: &[&[u8]] = &[
        b"\xc3\xa9*",
        b"\xc3\xa9",
        b"\xc3*",
        b"\xc3(",
        b"\xc3\\",
        b"\xc3.",
        b"\xc3[",
        b"\xe6\x97\xa5*",
        b"\xf0\x9f\x98\x80{2}",
        b"a\xc3\xa9b",
        b"[\xc3\xa9]",
        b"\xff\xfe",
    ];
    // POSIX_BASIC and POSIX_EXTENDED, with and without CONVERT_UTF.
    let opt_sets: &[u32] = &[0x4, 0x8, 0x4 | 0x1, 0x8 | 0x1];
    unsafe {
        for p in pats {
            for &opts in opt_sets {
                for cap in 1usize..=24 {
                    let mut outa = vec![0xAAu8; 64];
                    let mut outb = vec![0xAAu8; 64];
                    let mut pa = outa.as_mut_ptr();
                    let mut pb = outb.as_mut_ptr();
                    let mut la: PCRE2_SIZE = cap;
                    let mut lb: PCRE2_SIZE = cap;
                    let x = pc(p.as_ptr(), p.len(), opts, &mut pa, &mut la, std::ptr::null_mut());
                    let y = pr(p.as_ptr(), p.len(), opts, &mut pb, &mut lb, std::ptr::null_mut());
                    let label = format!("convert {p:02x?} opts={opts:#x} cap={cap}");
                    assert_eq!(x, y, "{label}: rc");
                    assert_eq!(la, lb, "{label}: length/error offset");
                    assert_bytes_eq(&format!("{label}: partial output"), &outa, &outb);
                }
                // Allocating form, for the successful cases.
                let mut pa: *mut PCRE2_UCHAR = std::ptr::null_mut();
                let mut pb: *mut PCRE2_UCHAR = std::ptr::null_mut();
                let mut la: PCRE2_SIZE = 0;
                let mut lb: PCRE2_SIZE = 0;
                let x = pc(p.as_ptr(), p.len(), opts, &mut pa, &mut la, std::ptr::null_mut());
                let y = pr(p.as_ptr(), p.len(), opts, &mut pb, &mut lb, std::ptr::null_mut());
                assert_eq!(x, y, "convert {p:02x?} opts={opts:#x}: rc (allocating)");
                assert_eq!(la, lb, "convert {p:02x?} opts={opts:#x}: length (allocating)");
                if x == 0 {
                    assert_bytes_eq(
                        &format!("convert {p:02x?} opts={opts:#x}: output"),
                        slice_at(pa, la + 1),
                        slice_at(pb, lb + 1),
                    );
                    fc(pa);
                    fr(pb);
                }
            }
        }
    }
}

type ConvertedFree = unsafe extern "C" fn(*mut PCRE2_UCHAR);
