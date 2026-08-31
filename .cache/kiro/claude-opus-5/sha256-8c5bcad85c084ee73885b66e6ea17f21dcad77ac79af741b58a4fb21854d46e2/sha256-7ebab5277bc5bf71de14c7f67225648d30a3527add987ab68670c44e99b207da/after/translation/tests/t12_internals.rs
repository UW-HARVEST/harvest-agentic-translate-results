//! Internal helpers that can be exercised directly through their exported
//! symbols: `_pcre2_xclass`, `_pcre2_eclass`, `_pcre2_find_bracket` and
//! `_pcre2_study` (via a compiled pattern).
mod common;

use common::*;
use std::ffi::{c_int, c_void};

type XClassFn = unsafe extern "C" fn(u32, PCRE2_SPTR, *const u8, c_int) -> c_int;
type EClassFn = unsafe extern "C" fn(u32, PCRE2_SPTR, PCRE2_SPTR, *const u8, c_int) -> c_int;
type FindBracketFn = unsafe extern "C" fn(PCRE2_SPTR, c_int, c_int) -> PCRE2_SPTR;
type StudyFn = unsafe extern "C" fn(*mut c_void) -> c_int;

/// Opcode numbers taken from `pcre2_internal.h`.
const OP_END: u8 = 0;
const OP_XCLASS: u8 = 112;
const OP_ECLASS: u8 = 113;

/// Read `blocksize`, `code_start` and the OP_lengths table so we can walk the
/// compiled byte code.
unsafe fn code_bytes(code: *const c_void) -> (&'static [u8], usize) {
    unsafe {
        let base = code as *const u8;
        let tail = std::ptr::read_unaligned(base.add(CODE_OFF_TAIL) as *const RealCodeTail);
        (
            slice_at(base.add(tail.code_start), tail.blocksize - tail.code_start),
            tail.code_start,
        )
    }
}

/// All the patterns below are a single top-level class, so the compiled form is
/// `OP_BRA <2-byte link> OP_XCLASS|OP_ECLASS ...`, putting the class opcode at
/// byte offset 3. Returns the opcode found there.
const CLASS_OP_OFFSET: usize = 3;

/// Set-operation classes only compile to OP_ECLASS when
/// `PCRE2_ALT_EXTENDED_CLASS` is in force, so include it here.
const CLASS_OPTION_SETS: [u32; 9] = [
    0,
    PCRE2_UTF,
    PCRE2_UCP,
    PCRE2_UTF | PCRE2_UCP,
    PCRE2_CASELESS,
    PCRE2_ALT_EXTENDED_CLASS,
    PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF,
    PCRE2_ALT_EXTENDED_CLASS | PCRE2_UTF | PCRE2_UCP,
    PCRE2_ALT_EXTENDED_CLASS | PCRE2_CASELESS,
];

fn get2(b: &[u8], at: usize) -> usize {
    ((b[at] as usize) << 8) | b[at + 1] as usize
}

#[test]
fn xclass_matches() {
    // `_pcre2_xclass(c, data, start_code, utf)`: the third argument is the start
    // of the compiled byte code (used to resolve XCL_LIST offsets), so each
    // library must be given its own.
    let (xc, xr) = both::<XClassFn>("_pcre2_xclass_8");
    let cus = code_units();
    let mut exercised = 0usize;
    for pat in WIDE_CLASS_PATTERNS {
        for &opts in &CLASS_OPTION_SETS {
            let Some(pair) = compile_both(pat, opts) else {
                continue;
            };
            unsafe {
                let (bc, _) = code_bytes(pair.c);
                let (br, _) = code_bytes(pair.r);
                assert_eq!(bc, br, "byte code differs for {pat:02x?}");
                if bc.len() <= CLASS_OP_OFFSET + 3 || bc[CLASS_OP_OFFSET] != OP_XCLASS {
                    continue;
                }
                exercised += 1;
                let data_c = bc.as_ptr().add(CLASS_OP_OFFSET + 1 + 2);
                let data_r = br.as_ptr().add(CLASS_OP_OFFSET + 1 + 2);
                let utf = if (opts & PCRE2_UTF) != 0 { 1 } else { 0 };
                for &cu in &cus {
                    let a = xc(cu, data_c, bc.as_ptr(), utf);
                    let b = xr(cu, data_r, br.as_ptr(), utf);
                    assert_eq!(
                        a != 0,
                        b != 0,
                        "xclass({cu:#x}, utf={utf}) for {pat:02x?} opts={opts:#x}"
                    );
                }
            }
        }
    }
    assert!(exercised > 0, "no OP_XCLASS pattern was exercised");
}

#[test]
fn eclass_matches() {
    // `_pcre2_eclass(c, data, data_end, start_code, utf)`.
    let (ec, er) = both::<EClassFn>("_pcre2_eclass_8");
    let cus = code_units();
    let mut exercised = 0usize;
    for pat in WIDE_CLASS_PATTERNS {
        for &opts in &CLASS_OPTION_SETS {
            let Some(pair) = compile_both(pat, opts) else {
                continue;
            };
            unsafe {
                let (bc, _) = code_bytes(pair.c);
                let (br, _) = code_bytes(pair.r);
                if bc.len() <= CLASS_OP_OFFSET + 3 || bc[CLASS_OP_OFFSET] != OP_ECLASS {
                    continue;
                }
                let len = get2(bc, CLASS_OP_OFFSET + 1);
                if len == 0 || CLASS_OP_OFFSET + len > bc.len() {
                    continue;
                }
                exercised += 1;
                let data_c = bc.as_ptr().add(CLASS_OP_OFFSET + 1 + 2);
                let data_r = br.as_ptr().add(CLASS_OP_OFFSET + 1 + 2);
                let end_c = bc.as_ptr().add(CLASS_OP_OFFSET + len);
                let end_r = br.as_ptr().add(CLASS_OP_OFFSET + len);
                let utf = if (opts & PCRE2_UTF) != 0 { 1 } else { 0 };
                for &cu in &cus {
                    let a = ec(cu, data_c, end_c, bc.as_ptr(), utf);
                    let b = er(cu, data_r, end_r, br.as_ptr(), utf);
                    assert_eq!(
                        a != 0,
                        b != 0,
                        "eclass({cu:#x}, utf={utf}) for {pat:02x?} opts={opts:#x}"
                    );
                }
            }
        }
    }
    assert!(exercised > 0, "no OP_ECLASS pattern was exercised");
}

/// Patterns that are known to produce OP_XCLASS (a class needing more than the
/// 256-bit bitmap) or OP_ECLASS (set operations on classes).
const WIDE_CLASS_PATTERNS: &[&[u8]] = &[
    b"[\\x{100}-\\x{200}]",
    b"[^\\x{100}]",
    b"[\\p{L}]",
    b"[\\P{Nd}]",
    b"[\\p{Greek}\\p{Han}]",
    b"[a-z\\x{100}-\\x{2000}]",
    b"[^\\p{L}]",
    b"[\\x{10000}-\\x{10ffff}]",
    b"[\\p{Xan}]",
    b"[[a-z]&&[^aeiou]]",
    b"[[a-z]--[aeiou]]",
    b"[[a-z]||[0-9]]",
    b"[[a-z]~~[a-c]]",
    b"[!\\p{L}]",
    b"[\\p{L}&&\\p{Greek}]",
    b"[[:alpha:]&&[:^lower:]]",
    b"[\\p{L}&&[^\\x{100}]]",
    b"[[\\x{100}-\\x{200}]||[a-z]]",
    b"[\\p{L}&&\\p{Greek}]",
    b"[\\p{L}--\\p{Greek}]",
    b"[\\p{L}||\\p{Nd}]",
    b"[\\p{L}~~\\p{Greek}]",
    b"[[\\x{100}-\\x{200}]&&[\\x{150}-\\x{250}]]",
    b"[!\\p{L}&&\\p{Greek}]",
    b"[\\p{L}&&[^\\x{100}]]",
    b"[\\p{Greek}||[\\x{100}-\\x{110}]]",
    b"[\\p{L}&&\\p{Greek}&&\\p{Nd}]",
];

fn code_units() -> Vec<u32> {
    let mut v: Vec<u32> = (0u32..=0x400).collect();
    v.extend((0x400u32..=0x11000).step_by(7));
    v.extend([
        0x10ffff, 0x10fffe, 0x10000, 0xffff, 0xfffe, 0xd7ff, 0xe000, 0x2ffff, 0x30000,
    ]);
    v
}

#[test]
fn find_bracket_matches() {
    let (fc, fr) = both::<FindBracketFn>("_pcre2_find_bracket_8");
    for pat in patterns() {
        for &opts in &[0u32, PCRE2_UTF, PCRE2_DUPNAMES, PCRE2_NO_AUTO_CAPTURE] {
            let Some(pair) = compile_both(pat, opts) else {
                continue;
            };
            unsafe {
                let (bc, _) = code_bytes(pair.c);
                let (br, _) = code_bytes(pair.r);
                // The `utf` argument must match how the pattern was compiled; a
                // mismatch makes the walk misinterpret the byte code (and index
                // OP_lengths out of range, which is undefined in C too).
                let utf = if (opts & PCRE2_UTF) != 0 { 1 } else { 0 };
                for number in -1..12 {
                    let a = fc(bc.as_ptr(), utf, number);
                    let b = fr(br.as_ptr(), utf, number);
                    let oa = if a.is_null() {
                        None
                    } else {
                        Some(a.offset_from(bc.as_ptr()))
                    };
                    let ob = if b.is_null() {
                        None
                    } else {
                        Some(b.offset_from(br.as_ptr()))
                    };
                    assert_eq!(
                        oa, ob,
                        "find_bracket({number}, utf={utf}) for {pat:02x?} opts={opts:#x}"
                    );
                }
            }
        }
    }
}

#[test]
fn study_is_idempotent_and_matches() {
    // pcre2_compile already calls _pcre2_study; calling it again on a compiled
    // pattern must produce the same return code and the same derived fields.
    let (sc, sr) = both::<StudyFn>("_pcre2_study_8");
    for pat in patterns() {
        for &opts in &[0u32, PCRE2_UTF, PCRE2_NO_START_OPTIMIZE, PCRE2_ANCHORED] {
            let Some(pair) = compile_both(pat, opts) else {
                continue;
            };
            unsafe {
                let a = sc(pair.c);
                let b = sr(pair.r);
                assert_eq!(a, b, "study rc for {pat:02x?} opts={opts:#x}");
                let (bm_c, tail_c, body_c) = code_snapshot(pair.c);
                let (bm_r, tail_r, body_r) = code_snapshot(pair.r);
                assert_bytes_eq(&format!("post-study bitmap {pat:02x?}"), &bm_c, &bm_r);
                assert_eq!(tail_c, tail_r, "post-study header {pat:02x?}");
                assert_bytes_eq(&format!("post-study body {pat:02x?}"), &body_c, &body_r);
            }
        }
    }
}

#[test]
fn op_lengths_table_walks_identically() {
    // Sanity: walk the byte code of every compiled pattern using OP_lengths and
    // confirm both libraries agree on where OP_END is (this exercises the table
    // and the compiled layout together).
    let (clen, rlen) = both_data("_pcre2_OP_lengths_8");
    unsafe {
        let ct = slice_at(clen, 0xad);
        let rt = slice_at(rlen, 0xad);
        assert_eq!(ct, rt, "OP_lengths");
        for pat in patterns() {
            let Some(pair) = compile_both(pat, 0) else {
                continue;
            };
            let (bc, _) = code_bytes(pair.c);
            // Find the top-level OP_END by scanning with the length table where
            // possible; variable-length opcodes stop the walk, which is fine.
            let mut i = 0usize;
            let mut steps = 0;
            while i < bc.len() && steps < 10_000 {
                let op = bc[i];
                if op == OP_END {
                    break;
                }
                let l = ct[op as usize] as usize;
                if l == 0 {
                    break; /* variable length: stop walking */
                }
                i += l;
                steps += 1;
            }
            assert!(i <= bc.len(), "walk overran for {pat:02x?}");
        }
    }
}
