//! Differential tests for CONFIGS.md rows C001-C055:
//!   * C001-C018 — the exported DATA TABLES / context statics
//!   * C019-C055 — the exported `PRIV()` leaf functions
//!
//! Everything goes through the two loaded `.so`s (see `tests/common/mod.rs`).

mod common;
use common::*;

use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

// ============================================================ C constants
// (all values read out of c_src/src/*.h — see the comment on each)

/// LINK_SIZE in the 8-bit build (pcre2_internal.h)
const LINK_SIZE: usize = 2;

// opcode numbers (pcre2_internal.h, the numbered comments in the opcode enum)
const OP_XCLASS: u8 = 112;
const OP_ECLASS: u8 = 113;
const OP_KET: u8 = 122;
const OP_BRA: u8 = 137;

// offsetof(pcre2_real_code, ...) for the LP64 layout in pcre2_intmodedep.h:660
//   memctl 0..24, tables 24, executable_jit 32, start_bitmap 40..72,
//   blocksize 72, code_start 80, magic_number 88, ...
const RC_BLOCKSIZE: usize = 72;
const RC_CODESTART: usize = 80;

// offsetof(pcre2_real_compile_context, ...) — pcre2_intmodedep.h:601
const CC_MEMCTL_MALLOC: usize = 0;
const CC_MEMCTL_FREE: usize = 8;
const CC_MEMCTL_DATA: usize = 16;
const CC_STACK_GUARD: usize = 24;
const CC_STACK_GUARD_DATA: usize = 32;
const CC_TABLES: usize = 40;
const CC_MAX_PATTERN_LENGTH: usize = 48;
const CC_MAX_PATTERN_COMPILED_LENGTH: usize = 56;
const CC_BSR: usize = 64;
const CC_NEWLINE: usize = 66;
const CC_PARENS_NEST_LIMIT: usize = 68;
const CC_EXTRA_OPTIONS: usize = 72;
const CC_MAX_VARLOOKBEHIND: usize = 76;
const CC_OPTIMIZATION_FLAGS: usize = 80;
const CC_SIZE: usize = 88;

// offsetof(pcre2_real_match_context, ...) — pcre2_intmodedep.h:618 (no SUPPORT_JIT)
const MC_CALLOUT: usize = 24;
const MC_CALLOUT_DATA: usize = 32;
const MC_SUBSTITUTE_CALLOUT: usize = 40;
const MC_SUBSTITUTE_CALLOUT_DATA: usize = 48;
const MC_SUBSTITUTE_CASE_CALLOUT: usize = 56;
const MC_SUBSTITUTE_CASE_CALLOUT_DATA: usize = 64;
const MC_OFFSET_LIMIT: usize = 72;
const MC_HEAP_LIMIT: usize = 80;
const MC_MATCH_LIMIT: usize = 84;
const MC_DEPTH_LIMIT: usize = 88;
const MC_SIZE: usize = 96;

// offsetof(pcre2_real_convert_context, ...) — pcre2_intmodedep.h:639
const VC_GLOB_SEPARATOR: usize = 24;
const VC_GLOB_ESCAPE: usize = 28;
const VC_SIZE: usize = 32;

// pcre2_internal.h:467
const NLTYPE_FIXED: u32 = 0;
const NLTYPE_ANY: u32 = 1;
const NLTYPE_ANYCRLF: u32 = 2;

// pcre2_internal.h:1481
const XCL_MAP: u8 = 0x02;
const XCL_PROP: u8 = 3;
const XCL_NOTPROP: u8 = 4;
const XCL_LIST_LO: u8 = 0x10; // XCL_LIST in the 8-bit build

// pcre2_internal.h:1569
const ECL_MAP: u8 = 0x01;
const ECL_AND: u8 = 1;
const ECL_OR: u8 = 2;
const ECL_XOR: u8 = 3;
const ECL_NOT: u8 = 4;
const ECL_XCLASS: u8 = 5;

// pcre2_internal.h:1445
const PT_LAMP: u8 = 0;
const PT_GC: u8 = 1;
const PT_PC: u8 = 2;
const PT_SC: u8 = 3;
const PT_SCX: u8 = 4;
const PT_ALNUM: u8 = 5;
const PT_SPACE: u8 = 6;
const PT_PXSPACE: u8 = 7;
const PT_WORD: u8 = 8;
const PT_UCNC: u8 = 10;
const PT_BIDICL: u8 = 11;
const PT_BOOL: u8 = 12;
const PT_ANY: u8 = 13;
const PT_PXGRAPH: u8 = 14;
const PT_PXPRINT: u8 = 15;
const PT_PXPUNCT: u8 = 16;
const PT_PXXDIGIT: u8 = 17;

// pcre2_internal.h:204 / 2122
const MAX_UTF_CODE_POINT: u32 = 0x10ffff;
const UCD_BLOCK_SIZE: usize = 128;
// pcre2_internal.h:598
const TABLES_LENGTH: usize = 1088;
// pcre2_internal.h:1908 — OP_TABLE_LENGTH; confirmed by the ELF symbol size of
// _pcre2_OP_lengths_8 (0xad = 173) in both libraries.
const OP_TABLE_LENGTH: usize = 173;

const NOTACHAR: u32 = 0xffff_ffff;

/// PCRE2_ERROR_UTF8_ERRn (pcre2.h:355) == -(2 + n)
fn utf8_err(n: i32) -> i32 {
    -(2 + n)
}

// ============================================================ data helpers

unsafe fn vec_u8(api: &Api, name: &str, n: usize) -> Vec<u8> {
    let p = api.data(name);
    (0..n).map(|i| *p.add(i)).collect()
}
unsafe fn vec_u16(api: &Api, name: &str, n: usize) -> Vec<u16> {
    let p = api.data(name) as *const u16;
    (0..n).map(|i| ptr::read_unaligned(p.add(i))).collect()
}
unsafe fn vec_u32(api: &Api, name: &str, n: usize) -> Vec<u32> {
    let p = api.data(name) as *const u32;
    (0..n).map(|i| ptr::read_unaligned(p.add(i))).collect()
}
unsafe fn vec_i32(api: &Api, name: &str, n: usize) -> Vec<i32> {
    let p = api.data(name) as *const i32;
    (0..n).map(|i| ptr::read_unaligned(p.add(i))).collect()
}
unsafe fn one_u32(api: &Api, name: &str) -> u32 {
    ptr::read_unaligned(api.data(name) as *const u32)
}
unsafe fn one_usize(api: &Api, name: &str) -> usize {
    ptr::read_unaligned(api.data(name) as *const usize)
}
unsafe fn rd_ptr(base: *const u8, off: usize) -> *const u8 {
    ptr::read_unaligned(base.add(off) as *const *const u8)
}
unsafe fn rd_usize(base: *const u8, off: usize) -> usize {
    ptr::read_unaligned(base.add(off) as *const usize)
}
unsafe fn rd_u32(base: *const u8, off: usize) -> u32 {
    ptr::read_unaligned(base.add(off) as *const u32)
}
unsafe fn rd_u16(base: *const u8, off: usize) -> u16 {
    ptr::read_unaligned(base.add(off) as *const u16)
}
unsafe fn cstr(p: *const u8) -> Vec<u8> {
    let mut v = vec![];
    let mut i = 0;
    while *p.add(i) != 0 {
        v.push(*p.add(i));
        i += 1;
    }
    v
}

/// Compare two byte blobs and report the first differing index.
fn cmp_blob(id: &str, what: &str, a: &[u8], b: &[u8]) {
    assert_eq!(a.len(), b.len(), "{} {}: length", id, what);
    for i in 0..a.len() {
        if a[i] != b[i] {
            panic!(
                "{} {}: byte {} differs: C=0x{:02x} RUST=0x{:02x}\n  C   [{}..]={:?}\n  RUST[{}..]={:?}",
                id,
                what,
                i,
                a[i],
                b[i],
                i,
                &a[i..(i + 16).min(a.len())],
                i,
                &b[i..(i + 16).min(b.len())]
            );
        }
    }
}

fn cmp_words<T: PartialEq + std::fmt::Debug>(id: &str, what: &str, a: &[T], b: &[T]) {
    assert_eq!(a.len(), b.len(), "{} {}: length", id, what);
    for i in 0..a.len() {
        if a[i] != b[i] {
            panic!(
                "{} {}: element {} differs: C={:?} RUST={:?}",
                id, what, i, a[i], b[i]
            );
        }
    }
}

fn utf8(cp: u32) -> Vec<u8> {
    if cp < 0x80 {
        vec![cp as u8]
    } else if cp < 0x800 {
        vec![0xc0 | (cp >> 6) as u8, 0x80 | (cp & 0x3f) as u8]
    } else if cp < 0x10000 {
        vec![
            0xe0 | (cp >> 12) as u8,
            0x80 | ((cp >> 6) & 0x3f) as u8,
            0x80 | (cp & 0x3f) as u8,
        ]
    } else {
        vec![
            0xf0 | (cp >> 18) as u8,
            0x80 | ((cp >> 12) & 0x3f) as u8,
            0x80 | ((cp >> 6) & 0x3f) as u8,
            0x80 | (cp & 0x3f) as u8,
        ]
    }
}

fn utf8_str(cps: &[u32]) -> Vec<u8> {
    let mut v = vec![];
    for &c in cps {
        v.extend_from_slice(&utf8(c));
    }
    v
}

/// Byte offsets in `s` that start a UTF-8 character (`s` must be valid UTF-8).
fn char_starts(s: &[u8]) -> Vec<usize> {
    let mut v = vec![];
    let mut i = 0;
    while i < s.len() {
        v.push(i);
        let b = s[i];
        i += if b < 0x80 {
            1
        } else if b < 0xe0 {
            2
        } else if b < 0xf0 {
            3
        } else {
            4
        };
    }
    v
}

// ============================================================ compile helpers

unsafe fn compile_c(c: &Api, pat: &str, opts: u32) -> Ptr {
    let mut ec: i32 = 0;
    let mut eo: usize = 0;
    let code = (c.pcre2_compile_8)(
        pat.as_ptr(),
        pat.len(),
        opts,
        &mut ec,
        &mut eo,
        ptr::null_mut(),
    );
    assert!(
        !code.is_null(),
        "C compile of {:?} (opts=0x{:08x}) failed: errorcode {} at offset {}",
        pat,
        opts,
        ec,
        eo
    );
    code
}

/// A located OP_XCLASS / OP_ECLASS block inside a C-compiled pattern.
struct ClassBlk {
    label: String,
    op: u8,
    /// `code + 1 + LINK_SIZE` — the flag code unit, i.e. the `data` argument
    data: *const u8,
    /// one past the last byte of the block (the `data_end` argument of eclass)
    dend: *const u8,
    /// `mb->start_code` — the `char_lists_end` argument
    cle: *const u8,
    flags: u8,
}

/// Locate the class opcode in a pattern of the form `[...]`.
///
/// The compiler always emits `OP_BRA <link> <class...> OP_KET <link> OP_END`
/// for such a pattern, so the opcode sits at `code_start + 1 + LINK_SIZE`.
/// The located offset is verified three ways: OP_BRA at code_start, a
/// plausible block length from `GET(code,1)`, and OP_KET exactly at the end of
/// the block.
unsafe fn locate_class(code: Ptr, label: &str) -> ClassBlk {
    let base = code as *const u8;
    let bs = rd_usize(base, RC_BLOCKSIZE);
    let cs = rd_usize(base, RC_CODESTART);
    assert!(cs + 8 < bs, "{}: implausible code_start/blocksize", label);
    let s = std::slice::from_raw_parts(base, bs);
    assert_eq!(
        s[cs], OP_BRA,
        "{}: expected OP_BRA at code_start, found {}",
        label, s[cs]
    );
    let at = cs + 1 + LINK_SIZE;
    let op = s[at];
    assert!(
        op == OP_XCLASS || op == OP_ECLASS,
        "{}: expected OP_XCLASS/OP_ECLASS at code_start+3, found opcode {}",
        label,
        op
    );
    let len = ((s[at + 1] as usize) << 8) | s[at + 2] as usize;
    assert!(
        len >= 1 + LINK_SIZE + 1 && at + len < bs,
        "{}: implausible block length {} (blocksize {})",
        label,
        len,
        bs
    );
    assert_eq!(
        s[at + len],
        OP_KET,
        "{}: block length {} does not land on OP_KET",
        label,
        len
    );
    ClassBlk {
        label: label.to_string(),
        op,
        data: base.add(at + 1 + LINK_SIZE),
        dend: base.add(at + len),
        cle: base.add(cs),
        flags: s[at + 1 + LINK_SIZE],
    }
}

/// The property items (`XCL_PROP` / `XCL_NOTPROP`) at the head of an XCLASS.
unsafe fn xclass_props(blk: &ClassBlk) -> Vec<(u8, u8, u8)> {
    assert_eq!(blk.op, OP_XCLASS);
    let mut d = blk.data;
    let flags = *d;
    d = d.add(1);
    if flags & XCL_MAP != 0 {
        d = d.add(32);
    }
    let mut out = vec![];
    while *d == XCL_PROP || *d == XCL_NOTPROP {
        out.push((*d, *d.add(1), *d.add(2)));
        d = d.add(3);
    }
    out
}

/// TRUE if the XCLASS uses the character-list encoding.
unsafe fn xclass_is_list(blk: &ClassBlk) -> bool {
    assert_eq!(blk.op, OP_XCLASS);
    let mut d = blk.data;
    let flags = *d;
    d = d.add(1);
    if flags & XCL_MAP != 0 {
        d = d.add(32);
    }
    while *d == XCL_PROP || *d == XCL_NOTPROP {
        d = d.add(3);
    }
    *d >= XCL_LIST_LO
}

/// The RPN opcodes of an ECLASS body (ECL_XCLASS items reported as ECL_XCLASS).
unsafe fn eclass_ops(blk: &ClassBlk) -> Vec<u8> {
    assert_eq!(blk.op, OP_ECLASS);
    let mut d = blk.data;
    let flags = *d;
    d = d.add(1);
    if flags & ECL_MAP != 0 {
        d = d.add(32);
    }
    let mut out = vec![];
    while (d as usize) < blk.dend as usize {
        let o = *d;
        out.push(o);
        if o == ECL_XCLASS {
            let l = ((*d.add(1) as usize) << 8) | *d.add(2) as usize;
            assert!(l >= 1 + LINK_SIZE, "{}: bad ECL_XCLASS length", blk.label);
            d = d.add(l);
        } else {
            d = d.add(1);
        }
    }
    out
}

/// The byte range of the i'th `ECL_XCLASS` item of an ECLASS body.
unsafe fn eclass_item(blk: &ClassBlk, want: usize) -> Vec<u8> {
    let mut d = blk.data;
    let flags = *d;
    d = d.add(1);
    if flags & ECL_MAP != 0 {
        d = d.add(32);
    }
    let mut n = 0;
    while (d as usize) < blk.dend as usize {
        if *d == ECL_XCLASS {
            let l = ((*d.add(1) as usize) << 8) | *d.add(2) as usize;
            if n == want {
                return std::slice::from_raw_parts(d, l).to_vec();
            }
            n += 1;
            d = d.add(l);
        } else {
            d = d.add(1);
        }
    }
    panic!("{}: no ECL_XCLASS item #{}", blk.label, want);
}

/// The code points swept through `_pcre2_xclass` / `_pcre2_eclass`: EVERY valid
/// code point, plus a fixed-seed random tail (repeats, so the binary searches are
/// re-entered in a different order).
fn sweep_points(seed: u64) -> Vec<u32> {
    let mut v: Vec<u32> = (0u32..=MAX_UTF_CODE_POINT).collect();
    let mut rng = Rng::new(seed);
    for _ in 0..20000 {
        v.push(rng.below(MAX_UTF_CODE_POINT + 1));
    }
    v
}

unsafe fn sweep_xclass(id: &str, c: &Api, r: &Api, blk: &ClassBlk, pts: &[u32]) {
    for &utf in &[0i32, 1i32] {
        for &cp in pts {
            let a = (c._pcre2_xclass_8)(cp, blk.data, blk.cle, utf);
            let b = (r._pcre2_xclass_8)(cp, blk.data, blk.cle, utf);
            if a != b {
                panic!(
                    "{} [{}]: _pcre2_xclass_8(c=0x{:x}, utf={}) C={} RUST={} (flags=0x{:02x})",
                    id, blk.label, cp, utf, a, b, blk.flags
                );
            }
        }
    }
}

unsafe fn sweep_eclass(id: &str, c: &Api, r: &Api, blk: &ClassBlk, pts: &[u32]) {
    for &utf in &[0i32, 1i32] {
        for &cp in pts {
            let a = (c._pcre2_eclass_8)(cp, blk.data, blk.dend, blk.cle, utf);
            let b = (r._pcre2_eclass_8)(cp, blk.data, blk.dend, blk.cle, utf);
            if a != b {
                panic!(
                    "{} [{}]: _pcre2_eclass_8(c=0x{:x}, utf={}) C={} RUST={}",
                    id, blk.label, cp, utf, a, b
                );
            }
        }
    }
}

// =========================================================================
// C001  _pcre2_OP_lengths_8
// =========================================================================
#[test]
fn c001_op_lengths() {
    println!("C001 _pcre2_OP_lengths_8");
    let (c, r) = both();
    unsafe {
        let a = vec_u8(c, "_pcre2_OP_lengths_8", OP_TABLE_LENGTH);
        let b = vec_u8(r, "_pcre2_OP_lengths_8", OP_TABLE_LENGTH);
        cmp_blob("C001", "_pcre2_OP_lengths_8", &a, &b);
        // the documented zero placeholders (length is stored in the code instead)
        for (name, op) in [("OP_XCLASS", 112usize), ("OP_ECLASS", 113), ("OP_CALLOUT_STR", 120)] {
            assert_eq!(a[op], 0, "C001 OP_lengths[{}] should be 0", name);
        }
        // every entry must be a plausible compiled-item size (the largest is
        // OP_CLASS/OP_NCLASS = 1 + 32 bitmap bytes)
        for i in 0..OP_TABLE_LENGTH {
            assert!(a[i] <= 33, "C001 OP_lengths[{}]={} implausible", i, a[i]);
        }
        // LINK_SIZE=2 / IMM2_SIZE=2 consequences: OP_KET is 1+LINK_SIZE,
        // OP_CBRA is 1+LINK_SIZE+IMM2_SIZE
        assert_eq!(a[122], 3, "C001 OP_lengths[OP_KET] with LINK_SIZE=2");
        assert_eq!(a[139], 5, "C001 OP_lengths[OP_CBRA] with LINK_SIZE=2, IMM2_SIZE=2");
        assert_eq!(a[110], 33, "C001 OP_lengths[OP_CLASS]");
        println!("C001 ok: {} bytes identical", OP_TABLE_LENGTH);
    }
}

// =========================================================================
// C002  _pcre2_hspace_list_8 / _pcre2_vspace_list_8
// =========================================================================
#[test]
fn c002_space_lists() {
    println!("C002 _pcre2_hspace_list_8 / _pcre2_vspace_list_8");
    let (c, r) = both();
    unsafe {
        for (name, n) in [("_pcre2_hspace_list_8", 20usize), ("_pcre2_vspace_list_8", 8)] {
            let a = vec_u32(c, name, n);
            let b = vec_u32(r, name, n);
            cmp_words("C002", name, &a, &b);
            assert_eq!(a[n - 1], NOTACHAR, "C002 {}: missing NOTACHAR terminator", name);
            for i in 0..n - 2 {
                assert!(a[i] < a[i + 1], "C002 {}: not ascending at {}", name, i);
            }
        }
        println!("C002 ok");
    }
}

// =========================================================================
// C003  _pcre2_utf8_table1..4 + size
// =========================================================================
#[test]
fn c003_utf8_tables() {
    println!("C003 _pcre2_utf8_table1/2/3/4");
    let (c, r) = both();
    unsafe {
        let sz_c = one_u32(c, "_pcre2_utf8_table1_size");
        let sz_r = one_u32(r, "_pcre2_utf8_table1_size");
        assert_eq!(sz_c, sz_r, "C003 _pcre2_utf8_table1_size");
        assert_eq!(sz_c, 6, "C003 _pcre2_utf8_table1_size expected 6");
        for name in ["_pcre2_utf8_table1", "_pcre2_utf8_table2", "_pcre2_utf8_table3"] {
            let a = vec_i32(c, name, 6);
            let b = vec_i32(r, name, 6);
            cmp_words("C003", name, &a, &b);
        }
        let a4 = vec_u8(c, "_pcre2_utf8_table4", 64);
        let b4 = vec_u8(r, "_pcre2_utf8_table4", 64);
        // index with every value 0x00..0x3f explicitly
        for i in 0..64usize {
            assert_eq!(
                a4[i], b4[i],
                "C003 _pcre2_utf8_table4[0x{:02x}] C={} RUST={}",
                i, a4[i], b4[i]
            );
            assert!(a4[i] >= 1 && a4[i] <= 5, "C003 table4[{}]={}", i, a4[i]);
        }
        println!("C003 ok");
    }
}

// =========================================================================
// C004  _pcre2_ucp_gentype_8
// =========================================================================
#[test]
fn c004_ucp_gentype() {
    println!("C004 _pcre2_ucp_gentype_8");
    let (c, r) = both();
    unsafe {
        let a = vec_u32(c, "_pcre2_ucp_gentype_8", 30);
        let b = vec_u32(r, "_pcre2_ucp_gentype_8", 30);
        cmp_words("C004", "_pcre2_ucp_gentype_8", &a, &b);
        // ucp_Cc..ucp_Cs are ucp_C(0); ucp_Ll(5) precedes ucp_Lu(9), both ucp_L(1)
        for i in 0..5 {
            assert_eq!(a[i], 0, "C004 gentype[{}]", i);
        }
        for i in 5..10 {
            assert_eq!(a[i], 1, "C004 gentype[{}]", i);
        }
        assert!(a.iter().all(|&v| v <= 6), "C004 gentype out of range");
        println!("C004 ok");
    }
}

// =========================================================================
// C005  _pcre2_ucp_gbtable_8
// =========================================================================
#[test]
fn c005_ucp_gbtable() {
    println!("C005 _pcre2_ucp_gbtable_8");
    let (c, r) = both();
    unsafe {
        let a = vec_u32(c, "_pcre2_ucp_gbtable_8", 15);
        let b = vec_u32(r, "_pcre2_ucp_gbtable_8", 15);
        cmp_words("C005", "_pcre2_ucp_gbtable_8", &a, &b);
        assert_eq!(a[0], 1u32 << 1, "C005 row gbCR must be only 1<<gbLF");
        assert_eq!(a[1], 0, "C005 row gbLF must be 0");
        assert_eq!(a[2], 0, "C005 row gbControl must be 0");
        assert_eq!(a[11], 1u32 << 11, "C005 row gbRI must be only 1<<gbRI");
        println!("C005 ok");
    }
}

// =========================================================================
// C006  _pcre2_callout_start_delims_8 / _pcre2_callout_end_delims_8
// =========================================================================
#[test]
fn c006_callout_delims() {
    println!("C006 _pcre2_callout_(start|end)_delims_8");
    let (c, r) = both();
    unsafe {
        let sa = vec_u32(c, "_pcre2_callout_start_delims_8", 9);
        let sb = vec_u32(r, "_pcre2_callout_start_delims_8", 9);
        let ea = vec_u32(c, "_pcre2_callout_end_delims_8", 9);
        let eb = vec_u32(r, "_pcre2_callout_end_delims_8", 9);
        cmp_words("C006", "_pcre2_callout_start_delims_8", &sa, &sb);
        cmp_words("C006", "_pcre2_callout_end_delims_8", &ea, &eb);
        // positional pairing: identical except the `{`/`}` slot
        for i in 0..9 {
            if sa[i] != ea[i] {
                assert_eq!(
                    (sa[i], ea[i]),
                    (b'{' as u32, b'}' as u32),
                    "C006 slot {} differs but is not the brace pair",
                    i
                );
            }
        }
        println!("C006 ok start={:?} end={:?}", sa, ea);
    }
}

// =========================================================================
// C007  _pcre2_posix_class_maps8
// =========================================================================
#[test]
fn c007_posix_class_maps() {
    println!("C007 _pcre2_posix_class_maps8");
    let (c, r) = both();
    unsafe {
        let a = vec_i32(c, "_pcre2_posix_class_maps8", 42);
        let b = vec_i32(r, "_pcre2_posix_class_maps8", 42);
        cmp_words("C007", "_pcre2_posix_class_maps8", &a, &b);
        let names = [
            "alpha", "lower", "upper", "alnum", "ascii", "blank", "cntrl", "digit", "graph",
            "print", "punct", "space", "word", "xdigit",
        ];
        for k in 0..14 {
            let t = (a[3 * k], a[3 * k + 1], a[3 * k + 2]);
            // only alpha and ascii have a non-(-1) second field
            if names[k] != "alpha" && names[k] != "ascii" {
                assert_eq!(t.1, -1, "C007 {} second field", names[k]);
            }
            // only alpha(-2), alnum(2), blank(1) have a non-zero third field
            match names[k] {
                "alpha" => assert_eq!(t.2, -2, "C007 alpha tweak"),
                "alnum" => assert_eq!(t.2, 2, "C007 alnum tweak"),
                "blank" => assert_eq!(t.2, 1, "C007 blank tweak"),
                _ => assert_eq!(t.2, 0, "C007 {} tweak", names[k]),
            }
        }
        println!("C007 ok");
    }
}

// =========================================================================
// C008  _pcre2_default_tables_8 (+ cross-check against pcre2_maketables)
// =========================================================================
#[test]
fn c008_default_tables() {
    println!("C008 _pcre2_default_tables_8");
    let (c, r) = both();
    unsafe {
        let a = vec_u8(c, "_pcre2_default_tables_8", TABLES_LENGTH);
        let b = vec_u8(r, "_pcre2_default_tables_8", TABLES_LENGTH);
        cmp_blob("C008", "_pcre2_default_tables_8", &a, &b);

        // sub-tables at the fixed offsets (pcre2_internal.h:592-598)
        cmp_blob("C008", "lcc", &a[0..256], &b[0..256]);
        cmp_blob("C008", "fcc", &a[256..512], &b[256..512]);
        for (i, off) in (512..832).step_by(32).enumerate() {
            cmp_blob(
                "C008",
                &format!("cbits map #{}", i),
                &a[off..off + 32],
                &b[off..off + 32],
            );
        }
        cmp_blob("C008", "ctypes", &a[832..1088], &b[832..1088]);

        // pcre2_config reports the same length
        for (tag, api) in [("C", c), ("RUST", r)] {
            let mut v: u32 = 0;
            let rc = (api.pcre2_config_8)(PCRE2_CONFIG_TABLES_LENGTH, &mut v as *mut u32 as Ptr);
            assert_eq!(rc, 0, "C008 {} config TABLES_LENGTH rc", tag);
            assert_eq!(v as usize, TABLES_LENGTH, "C008 {} TABLES_LENGTH", tag);
        }

        // maketables in the C locale must reproduce the compiled-in defaults
        for (tag, api) in [("C", c), ("RUST", r)] {
            let t = (api.pcre2_maketables_8)(ptr::null_mut());
            assert!(!t.is_null(), "C008 {} maketables returned NULL", tag);
            let made = std::slice::from_raw_parts(t, TABLES_LENGTH).to_vec();
            (api.pcre2_maketables_free_8)(ptr::null_mut(), t);
            cmp_blob("C008", &format!("{} maketables vs default_tables", tag), &a, &made);
        }
        println!("C008 ok: {} bytes identical, maketables agrees", TABLES_LENGTH);
    }
}

// =========================================================================
// C009  _pcre2_unicode_version_8
// =========================================================================
#[test]
fn c009_unicode_version() {
    println!("C009 _pcre2_unicode_version_8");
    let (c, r) = both();
    unsafe {
        // the symbol is a `const char *` variable: dereference it
        let pa = ptr::read_unaligned(c.data("_pcre2_unicode_version_8") as *const *const u8);
        let pb = ptr::read_unaligned(r.data("_pcre2_unicode_version_8") as *const *const u8);
        assert!(!pa.is_null() && !pb.is_null(), "C009 NULL version pointer");
        let sa = cstr(pa);
        let sb = cstr(pb);
        assert_eq!(sa, sb, "C009 unicode version: C={:?} RUST={:?}",
                   String::from_utf8_lossy(&sa), String::from_utf8_lossy(&sb));
        assert!(!sa.is_empty(), "C009 empty version string");
        assert_ne!(
            String::from_utf8_lossy(&sa),
            "Unicode not supported",
            "C009 SUPPORT_UNICODE build expected"
        );
        // pcre2_config must report the same string
        for (tag, api) in [("C", c), ("RUST", r)] {
            let mut buf = [0u8; 64];
            let rc = (api.pcre2_config_8)(PCRE2_CONFIG_UNICODE_VERSION, buf.as_mut_ptr() as Ptr);
            assert!(rc > 0, "C009 {} config rc {}", tag, rc);
            let s = cstr(buf.as_ptr());
            assert_eq!(s, sa, "C009 {} config vs symbol", tag);
        }
        println!("C009 ok: {:?}", String::from_utf8_lossy(&sa));
    }
}

// =========================================================================
// C010  _pcre2_ucd_records_8
// =========================================================================
const UCD_RECORD_SIZE: usize = 12; // uint8*4 + int32 + uint16*2, LP64
const UCD_RECORD_COUNT: usize = 1563; // ELF symbol size 0x4944 / 12

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
struct Ucd {
    script: u8,
    chartype: u8,
    gbprop: u8,
    caseset: u8,
    other_case: i32,
    scriptx_bidiclass: u16,
    bprops: u16,
}

unsafe fn ucd_at(base: *const u8, i: usize) -> Ucd {
    let p = base.add(i * UCD_RECORD_SIZE);
    Ucd {
        script: *p,
        chartype: *p.add(1),
        gbprop: *p.add(2),
        caseset: *p.add(3),
        other_case: ptr::read_unaligned(p.add(4) as *const i32),
        scriptx_bidiclass: rd_u16(p, 8),
        bprops: rd_u16(p, 10),
    }
}

#[test]
fn c010_ucd_records() {
    println!("C010 _pcre2_ucd_records_8");
    let (c, r) = both();
    unsafe {
        let pa = c.data("_pcre2_ucd_records_8");
        let pb = r.data("_pcre2_ucd_records_8");
        // byte-exact first
        let a = std::slice::from_raw_parts(pa, UCD_RECORD_COUNT * UCD_RECORD_SIZE).to_vec();
        let b = std::slice::from_raw_parts(pb, UCD_RECORD_COUNT * UCD_RECORD_SIZE).to_vec();
        cmp_blob("C010", "_pcre2_ucd_records_8", &a, &b);
        // then field-wise, which also proves the struct layout matches
        for i in 0..UCD_RECORD_COUNT {
            let ra = ucd_at(pa, i);
            let rb = ucd_at(pb, i);
            if ra != rb {
                panic!("C010 record {} differs: C={:?} RUST={:?}", i, ra, rb);
            }
        }
        // record 0 is the unassigned fallback
        let z = ucd_at(pa, 0);
        assert_eq!(z.other_case, 0, "C010 record 0 other_case");
        println!("C010 ok: {} records identical", UCD_RECORD_COUNT);
    }
}

// =========================================================================
// C011  _pcre2_ucd_stage1_8 / _pcre2_ucd_stage2_8 + full GET_UCD sweep
// =========================================================================
#[test]
fn c011_ucd_stages() {
    println!("C011 _pcre2_ucd_stage1_8 / _pcre2_ucd_stage2_8");
    let (c, r) = both();
    unsafe {
        // (MAX_UTF_CODE_POINT+1)/UCD_BLOCK_SIZE
        let n1 = (MAX_UTF_CODE_POINT as usize + 1) / UCD_BLOCK_SIZE;
        assert_eq!(n1, 8704, "C011 derived stage1 length");
        let s1a = vec_u16(c, "_pcre2_ucd_stage1_8", n1);
        let s1b = vec_u16(r, "_pcre2_ucd_stage1_8", n1);
        cmp_words("C011", "_pcre2_ucd_stage1_8", &s1a, &s1b);

        // stage1_max_index * UCD_BLOCK_SIZE
        let max1 = *s1a.iter().max().unwrap() as usize;
        let n2 = (max1 + 1) * UCD_BLOCK_SIZE;
        assert_eq!(n2, 40192, "C011 derived stage2 length ({} blocks)", max1 + 1);
        let s2a = vec_u16(c, "_pcre2_ucd_stage2_8", n2);
        let s2b = vec_u16(r, "_pcre2_ucd_stage2_8", n2);
        cmp_words("C011", "_pcre2_ucd_stage2_8", &s2a, &s2b);

        // Exercise REAL_GET_UCD's arithmetic for EVERY code point and compare the
        // record each library's tables select, field by field.
        let reca = c.data("_pcre2_ucd_records_8");
        let recb = r.data("_pcre2_ucd_records_8");
        let gta = vec_u32(c, "_pcre2_ucp_gentype_8", 30);
        let gtb = vec_u32(r, "_pcre2_ucp_gentype_8", 30);
        let mut max_rec = 0usize;
        for ch in 0..=MAX_UTF_CODE_POINT as usize {
            let ia = s1a[ch / UCD_BLOCK_SIZE] as usize * UCD_BLOCK_SIZE + ch % UCD_BLOCK_SIZE;
            let ib = s1b[ch / UCD_BLOCK_SIZE] as usize * UCD_BLOCK_SIZE + ch % UCD_BLOCK_SIZE;
            if ia != ib {
                panic!("C011 stage2 index for U+{:04X}: C={} RUST={}", ch, ia, ib);
            }
            let ka = s2a[ia] as usize;
            let kb = s2b[ib] as usize;
            if ka != kb {
                panic!("C011 record index for U+{:04X}: C={} RUST={}", ch, ka, kb);
            }
            assert!(ka < UCD_RECORD_COUNT, "C011 U+{:04X} record index {} OOB", ch, ka);
            if ka > max_rec {
                max_rec = ka;
            }
            let ra = ucd_at(reca, ka);
            let rb = ucd_at(recb, kb);
            if ra != rb {
                panic!("C011 U+{:04X} record: C={:?} RUST={:?}", ch, ra, rb);
            }
            // UCD_CHARTYPE / UCD_SCRIPT / UCD_GRAPHBREAK / UCD_OTHERCASE /
            // UCD_CASESET / UCD_CATEGORY / UCD_SCRIPTX / UCD_BIDICLASS / UCD_BPROPS
            let ca = gta[ra.chartype as usize];
            let cb = gtb[rb.chartype as usize];
            if ca != cb {
                panic!("C011 UCD_CATEGORY(U+{:04X}) C={} RUST={}", ch, ca, cb);
            }
            let oa = (ch as i64 + ra.other_case as i64) as u32;
            let ob = (ch as i64 + rb.other_case as i64) as u32;
            if oa != ob {
                panic!("C011 UCD_OTHERCASE(U+{:04X}) C={} RUST={}", ch, oa, ob);
            }
            if (ra.scriptx_bidiclass & 0x3ff, ra.scriptx_bidiclass >> 11, ra.bprops & 0xfff)
                != (rb.scriptx_bidiclass & 0x3ff, rb.scriptx_bidiclass >> 11, rb.bprops & 0xfff)
            {
                panic!("C011 UCD_SCRIPTX/BIDICLASS/BPROPS(U+{:04X}) differ", ch);
            }
        }
        assert!(
            max_rec + 1 <= UCD_RECORD_COUNT,
            "C011 highest record index {} vs count {}",
            max_rec,
            UCD_RECORD_COUNT
        );
        println!(
            "C011 ok: stage1={} stage2={} entries, all {} code points agree (max record {})",
            n1,
            n2,
            MAX_UTF_CODE_POINT + 1,
            max_rec
        );
    }
}

// =========================================================================
// C012  _pcre2_ucd_caseless_sets_8 / _pcre2_ucd_turkish_dotted_i_caseset_8
// =========================================================================
#[test]
fn c012_caseless_sets() {
    println!("C012 _pcre2_ucd_caseless_sets_8 / turkish caseset");
    let (c, r) = both();
    unsafe {
        let n = 118; // ELF symbol size 0x1d8 / 4
        let a = vec_u32(c, "_pcre2_ucd_caseless_sets_8", n);
        let b = vec_u32(r, "_pcre2_ucd_caseless_sets_8", n);
        cmp_words("C012", "_pcre2_ucd_caseless_sets_8", &a, &b);
        assert_eq!(a[0], NOTACHAR, "C012 index 0 must be the lone sentinel");

        let ta = one_u32(c, "_pcre2_ucd_turkish_dotted_i_caseset_8");
        let tb = one_u32(r, "_pcre2_ucd_turkish_dotted_i_caseset_8");
        assert_eq!(ta, tb, "C012 turkish caseset offset");
        assert_eq!(ta, 112, "C012 turkish caseset offset expected 112");
        // both sets: at the offset and at offset+3
        for off in [ta as usize, ta as usize + 3] {
            let mut i = off;
            let mut run_a = vec![];
            let mut run_b = vec![];
            while a[i] != NOTACHAR {
                run_a.push(a[i]);
                run_b.push(b[i]);
                i += 1;
                assert!(i < n, "C012 unterminated run at {}", off);
            }
            assert_eq!(run_a, run_b, "C012 run at offset {}", off);
            assert!(!run_a.is_empty(), "C012 empty run at offset {}", off);
        }
        println!("C012 ok");
    }
}

// =========================================================================
// C013  _pcre2_ucd_digit_sets_8
// =========================================================================
#[test]
fn c013_digit_sets() {
    println!("C013 _pcre2_ucd_digit_sets_8");
    let (c, r) = both();
    unsafe {
        let n = 78; // ELF symbol size 0x138 / 4
        let a = vec_u32(c, "_pcre2_ucd_digit_sets_8", n);
        let b = vec_u32(r, "_pcre2_ucd_digit_sets_8", n);
        cmp_words("C013", "_pcre2_ucd_digit_sets_8", &a, &b);
        assert_eq!(a[0], 77, "C013 [0] is the count");
        assert_eq!(a[1], 0x39, "C013 [1] is the ASCII fast-path value");
        for i in 1..(a[0] as usize) {
            assert!(a[i] < a[i + 1], "C013 not ascending at {}", i);
        }
        // exercise the binary chop the same way _pcre2_script_run does
        let chop = |v: &Vec<u32>, cp: u32| -> u32 {
            if cp <= v[1] {
                return 1;
            }
            let mut bot = 1usize;
            let mut top = v[0] as usize;
            loop {
                if top <= bot + 1 {
                    return top as u32;
                }
                let mid = (top + bot) / 2;
                if cp <= v[mid] {
                    top = mid;
                } else {
                    bot = mid;
                }
            }
        };
        let mut rng = Rng::new(0x13);
        for k in 0..5000 {
            let cp = if k < 200 {
                a[(k % (a[0] as usize)) + 1].wrapping_sub((k % 3) as u32)
            } else {
                rng.below(MAX_UTF_CODE_POINT + 1)
            };
            assert_eq!(chop(&a, cp), chop(&b, cp), "C013 digitset chop for 0x{:x}", cp);
        }
        println!("C013 ok");
    }
}

// =========================================================================
// C014  _pcre2_ucd_script_sets_8
// =========================================================================
#[test]
fn c014_script_sets() {
    println!("C014 _pcre2_ucd_script_sets_8");
    let (c, r) = both();
    unsafe {
        let rows = 119; // 476 u32 / 4 words per row (ELF size 0x770)
        let n = rows * 4;
        let a = vec_u32(c, "_pcre2_ucd_script_sets_8", n);
        let b = vec_u32(r, "_pcre2_ucd_script_sets_8", n);
        cmp_words("C014", "_pcre2_ucd_script_sets_8", &a, &b);
        for k in 0..4 {
            assert_eq!(a[k], 0, "C014 row 0 word {} must be zero", k);
        }
        // every UCD_SCRIPTX_PROP value that actually occurs must address a full row
        let reca = c.data("_pcre2_ucd_records_8");
        for i in 0..UCD_RECORD_COUNT {
            let sx = (ucd_at(reca, i).scriptx_bidiclass & 0x3ff) as usize;
            assert!(sx + 4 <= n, "C014 record {} scriptx offset {} OOB", i, sx);
            assert_eq!(sx % 4, 0, "C014 record {} scriptx offset {} unaligned", i, sx);
        }
        println!("C014 ok: {} bitsets", rows);
    }
}

// =========================================================================
// C015  _pcre2_ucd_boolprop_sets_8
// =========================================================================
#[test]
fn c015_boolprop_sets() {
    println!("C015 _pcre2_ucd_boolprop_sets_8");
    let (c, r) = both();
    unsafe {
        let rows = 191; // 382 u32 / 2 words per row (ELF size 0x5f8)
        let n = rows * 2;
        let a = vec_u32(c, "_pcre2_ucd_boolprop_sets_8", n);
        let b = vec_u32(r, "_pcre2_ucd_boolprop_sets_8", n);
        cmp_words("C015", "_pcre2_ucd_boolprop_sets_8", &a, &b);
        for k in 0..2 {
            assert_eq!(a[k], 0, "C015 row 0 word {} must be zero", k);
        }
        let reca = c.data("_pcre2_ucd_records_8");
        for i in 0..UCD_RECORD_COUNT {
            let bp = (ucd_at(reca, i).bprops & 0xfff) as usize;
            assert!(bp + 2 <= n, "C015 record {} bprops offset {} OOB", i, bp);
            assert_eq!(bp % 2, 0, "C015 record {} bprops offset {} unaligned", i, bp);
        }
        println!("C015 ok: {} bitsets", rows);
    }
}

// =========================================================================
// C016  _pcre2_ucd_nocase_ranges_8 (+ size)
// =========================================================================
#[test]
fn c016_nocase_ranges() {
    println!("C016 _pcre2_ucd_nocase_ranges_8");
    let (c, r) = both();
    unsafe {
        let n = 84; // ELF symbol size 0x150 / 4
        let a = vec_u32(c, "_pcre2_ucd_nocase_ranges_8", n);
        let b = vec_u32(r, "_pcre2_ucd_nocase_ranges_8", n);
        cmp_words("C016", "_pcre2_ucd_nocase_ranges_8", &a, &b);
        let sa = one_u32(c, "_pcre2_ucd_nocase_ranges_size_8");
        let sb = one_u32(r, "_pcre2_ucd_nocase_ranges_size_8");
        assert_eq!(sa, sb, "C016 nocase_ranges_size");
        assert_eq!(sa, 82, "C016 nocase_ranges_size expected 82 (excludes sentinel)");
        assert_eq!(a[n - 1], NOTACHAR, "C016 missing 0xffffffff sentinel");
        assert_eq!(a[n - 2], NOTACHAR, "C016 sentinel pair");
        // ascending exclusive bounds (adjacent ranges may share an endpoint, e.g.
        // 0x007a,0x00b5 followed by 0x00b5,0x00c0)
        for i in 0..(sa as usize - 1) {
            assert!(a[i] <= a[i + 1], "C016 not ascending at {}", i);
        }
        // reproduce get_nocase_range() from pcre2_compile_class.c:128-146 exactly
        let find = |v: &Vec<u32>, cp: u32| -> usize {
            if cp > MAX_UTF_CODE_POINT {
                return sa as usize;
            }
            let mut left = 0u32;
            let mut right = sa;
            loop {
                let middle = ((left + right) >> 1) | 0x1;
                if v[middle as usize] <= cp {
                    left = middle + 1;
                } else if middle > 1 && v[middle as usize - 2] > cp {
                    right = middle - 1;
                } else {
                    return middle as usize - 1;
                }
            }
        };
        let mut rng = Rng::new(0x16);
        for k in 0..5000u32 {
            let cp = if k < 200 { a[(k as usize) % (n - 2)] } else { rng.below(MAX_UTF_CODE_POINT + 1) };
            assert_eq!(find(&a, cp), find(&b, cp), "C016 search for 0x{:x}", cp);
        }
        println!("C016 ok");
    }
}

// =========================================================================
// C017  _pcre2_utt_8 / _pcre2_utt_names_8 / _pcre2_utt_size_8
// =========================================================================
#[test]
fn c017_utt() {
    println!("C017 _pcre2_utt_8 / _pcre2_utt_names_8 / _pcre2_utt_size_8");
    let (c, r) = both();
    unsafe {
        let na = one_usize(c, "_pcre2_utt_size_8");
        let nb = one_usize(r, "_pcre2_utt_size_8");
        assert_eq!(na, nb, "C017 _pcre2_utt_size_8: C={} RUST={}", na, nb);
        assert!(na > 400 && na < 1000, "C017 implausible utt_size {}", na);

        let ta = c.data("_pcre2_utt_8");
        let tb = r.data("_pcre2_utt_8");
        let sa = c.data("_pcre2_utt_names_8");
        let sb = r.data("_pcre2_utt_names_8");

        // ucp_type_table { uint16 name_offset; uint16 type; uint16 value; }
        let mut names_end = 0usize;
        let mut prev: Vec<u8> = vec![];
        for i in 0..na {
            let ea = (rd_u16(ta, i * 6), rd_u16(ta, i * 6 + 2), rd_u16(ta, i * 6 + 4));
            let eb = (rd_u16(tb, i * 6), rd_u16(tb, i * 6 + 2), rd_u16(tb, i * 6 + 4));
            if ea != eb {
                panic!(
                    "C017 utt[{}] differs: C={{off {}, type {}, value {}}} RUST={{off {}, type {}, value {}}}",
                    i, ea.0, ea.1, ea.2, eb.0, eb.1, eb.2
                );
            }
            // dereference the name offset in both blobs
            let n1 = cstr(sa.add(ea.0 as usize));
            let n2 = cstr(sb.add(eb.0 as usize));
            if n1 != n2 {
                panic!(
                    "C017 utt[{}] name at offset {}: C={:?} RUST={:?}",
                    i,
                    ea.0,
                    String::from_utf8_lossy(&n1),
                    String::from_utf8_lossy(&n2)
                );
            }
            assert!(!n1.is_empty(), "C017 utt[{}] empty name", i);
            names_end = names_end.max(ea.0 as usize + n1.len() + 1);
            // the compile-time \p{...} lookup is a binary search: must be sorted
            assert!(
                prev.as_slice() < n1.as_slice(),
                "C017 utt not sorted at {}: {:?} then {:?}",
                i,
                String::from_utf8_lossy(&prev),
                String::from_utf8_lossy(&n1)
            );
            prev = n1;
        }
        // the whole names blob
        let ba = std::slice::from_raw_parts(sa, names_end).to_vec();
        let bb = std::slice::from_raw_parts(sb, names_end).to_vec();
        cmp_blob("C017", "_pcre2_utt_names_8", &ba, &bb);

        // Alias pairs share type+value. Script names carry PT_SC or PT_SCX
        // depending on whether the script appears in any script-extension list
        // (never both for the same script value), so both property types must be
        // present in the table and the split must be identical in both libraries.
        let mut alias_pairs = 0;
        let mut n_sc = (0usize, 0usize);
        let mut n_scx = (0usize, 0usize);
        for i in 0..na {
            let a1 = (rd_u16(ta, i * 6 + 2), rd_u16(ta, i * 6 + 4));
            let b1 = (rd_u16(tb, i * 6 + 2), rd_u16(tb, i * 6 + 4));
            if a1.0 as u8 == PT_SC {
                n_sc.0 += 1;
            }
            if b1.0 as u8 == PT_SC {
                n_sc.1 += 1;
            }
            if a1.0 as u8 == PT_SCX {
                n_scx.0 += 1;
            }
            if b1.0 as u8 == PT_SCX {
                n_scx.1 += 1;
            }
            for j in (i + 1)..na {
                let a2 = (rd_u16(ta, j * 6 + 2), rd_u16(ta, j * 6 + 4));
                if a1 == a2 {
                    alias_pairs += 1;
                }
            }
        }
        assert!(alias_pairs > 0, "C017 no alias pairs found");
        assert_eq!(n_sc.0, n_sc.1, "C017 PT_SC entry count");
        assert_eq!(n_scx.0, n_scx.1, "C017 PT_SCX entry count");
        assert!(n_sc.0 > 0 && n_scx.0 > 0, "C017 PT_SC={} PT_SCX={}", n_sc.0, n_scx.0);
        println!(
            "C017 ok: {} entries, {} names bytes, {} alias pairs, {} PT_SC + {} PT_SCX entries",
            na, names_end, alias_pairs, n_sc.0, n_scx.0
        );
    }
}

// =========================================================================
// C018  the three exported default context statics
// =========================================================================
#[test]
fn c018_default_contexts() {
    println!("C018 _pcre2_default_(compile|match|convert)_context_8");
    let (c, r) = both();
    unsafe {
        // --- the library's own default_malloc/default_free, for reference
        let defaults = |api: &Api| -> (*const u8, *const u8) {
            let gc = (api.pcre2_general_context_create_8)(None, None, ptr::null_mut());
            assert!(!gc.is_null());
            let p = gc as *const u8;
            let d = (rd_ptr(p, 0), rd_ptr(p, 8));
            (api.pcre2_general_context_free_8)(gc);
            d
        };
        let (cm, cf) = defaults(c);
        let (rm, rf) = defaults(r);

        // ---------------- compile context
        for (tag, api, dm, df) in [("C", c, cm, cf), ("RUST", r, rm, rf)] {
            let p = api.data("_pcre2_default_compile_context_8");
            assert_eq!(rd_ptr(p, CC_MEMCTL_MALLOC), dm, "C018 {} ccontext memctl.malloc", tag);
            assert_eq!(rd_ptr(p, CC_MEMCTL_FREE), df, "C018 {} ccontext memctl.free", tag);
            assert!(rd_ptr(p, CC_MEMCTL_DATA).is_null(), "C018 {} ccontext memory_data", tag);
            assert!(rd_ptr(p, CC_STACK_GUARD).is_null(), "C018 {} stack_guard", tag);
            assert!(rd_ptr(p, CC_STACK_GUARD_DATA).is_null(), "C018 {} stack_guard_data", tag);
            assert_eq!(
                rd_ptr(p, CC_TABLES),
                api.data("_pcre2_default_tables_8"),
                "C018 {} ccontext tables must point at that library's _pcre2_default_tables_8",
                tag
            );
        }
        let pa = c.data("_pcre2_default_compile_context_8");
        let pb = r.data("_pcre2_default_compile_context_8");
        let scal_cc: &[(&str, usize, usize)] = &[
            ("max_pattern_length", CC_MAX_PATTERN_LENGTH, 8),
            ("max_pattern_compiled_length", CC_MAX_PATTERN_COMPILED_LENGTH, 8),
            ("bsr_convention", CC_BSR, 2),
            ("newline_convention", CC_NEWLINE, 2),
            ("parens_nest_limit", CC_PARENS_NEST_LIMIT, 4),
            ("extra_options", CC_EXTRA_OPTIONS, 4),
            ("max_varlookbehind", CC_MAX_VARLOOKBEHIND, 4),
            ("optimization_flags", CC_OPTIMIZATION_FLAGS, 4),
        ];
        for &(nm, off, w) in scal_cc {
            let (va, vb) = match w {
                8 => (rd_usize(pa, off) as u64, rd_usize(pb, off) as u64),
                4 => (rd_u32(pa, off) as u64, rd_u32(pb, off) as u64),
                _ => (rd_u16(pa, off) as u64, rd_u16(pb, off) as u64),
            };
            assert_eq!(va, vb, "C018 compile_context.{}: C={} RUST={}", nm, va, vb);
        }
        assert_eq!(rd_usize(pa, CC_MAX_PATTERN_LENGTH), PCRE2_UNSET, "C018 max_pattern_length");
        assert_eq!(
            rd_usize(pa, CC_MAX_PATTERN_COMPILED_LENGTH),
            PCRE2_UNSET,
            "C018 max_pattern_compiled_length"
        );
        assert_eq!(rd_u16(pa, CC_BSR) as u32, PCRE2_BSR_UNICODE, "C018 BSR_DEFAULT");
        assert_eq!(rd_u16(pa, CC_NEWLINE) as u32, PCRE2_NEWLINE_LF, "C018 NEWLINE_DEFAULT");
        assert_eq!(rd_u32(pa, CC_PARENS_NEST_LIMIT), 250, "C018 PARENS_NEST_LIMIT");
        assert_eq!(rd_u32(pa, CC_EXTRA_OPTIONS), 0, "C018 extra_options");
        assert_eq!(rd_u32(pa, CC_MAX_VARLOOKBEHIND), 255, "C018 MAX_VARLOOKBEHIND");
        assert_eq!(rd_u32(pa, CC_OPTIMIZATION_FLAGS), 0x7, "C018 PCRE2_OPTIMIZATION_ALL");
        // no unaccounted-for tail bytes
        assert_eq!(CC_SIZE, 88);

        // ---------------- match context
        for (tag, api, dm, df) in [("C", c, cm, cf), ("RUST", r, rm, rf)] {
            let p = api.data("_pcre2_default_match_context_8");
            assert_eq!(rd_ptr(p, CC_MEMCTL_MALLOC), dm, "C018 {} mcontext memctl.malloc", tag);
            assert_eq!(rd_ptr(p, CC_MEMCTL_FREE), df, "C018 {} mcontext memctl.free", tag);
            for (nm, off) in [
                ("memory_data", CC_MEMCTL_DATA),
                ("callout", MC_CALLOUT),
                ("callout_data", MC_CALLOUT_DATA),
                ("substitute_callout", MC_SUBSTITUTE_CALLOUT),
                ("substitute_callout_data", MC_SUBSTITUTE_CALLOUT_DATA),
                ("substitute_case_callout", MC_SUBSTITUTE_CASE_CALLOUT),
                ("substitute_case_callout_data", MC_SUBSTITUTE_CASE_CALLOUT_DATA),
            ] {
                assert!(rd_ptr(p, off).is_null(), "C018 {} mcontext.{}", tag, nm);
            }
        }
        let ma = c.data("_pcre2_default_match_context_8");
        let mb = r.data("_pcre2_default_match_context_8");
        assert_eq!(
            rd_usize(ma, MC_OFFSET_LIMIT),
            rd_usize(mb, MC_OFFSET_LIMIT),
            "C018 match_context.offset_limit"
        );
        assert_eq!(rd_usize(ma, MC_OFFSET_LIMIT), PCRE2_UNSET, "C018 offset_limit");
        for (nm, off, want) in [
            ("heap_limit", MC_HEAP_LIMIT, 20_000_000u32),
            ("match_limit", MC_MATCH_LIMIT, 10_000_000),
            ("depth_limit", MC_DEPTH_LIMIT, 10_000_000),
        ] {
            let (va, vb) = (rd_u32(ma, off), rd_u32(mb, off));
            assert_eq!(va, vb, "C018 match_context.{}: C={} RUST={}", nm, va, vb);
            assert_eq!(va, want, "C018 match_context.{}", nm);
        }
        assert_eq!(MC_SIZE, 96);

        // ---------------- convert context
        for (tag, api, dm, df) in [("C", c, cm, cf), ("RUST", r, rm, rf)] {
            let p = api.data("_pcre2_default_convert_context_8");
            assert_eq!(rd_ptr(p, CC_MEMCTL_MALLOC), dm, "C018 {} vcontext memctl.malloc", tag);
            assert_eq!(rd_ptr(p, CC_MEMCTL_FREE), df, "C018 {} vcontext memctl.free", tag);
            assert!(rd_ptr(p, CC_MEMCTL_DATA).is_null(), "C018 {} vcontext memory_data", tag);
        }
        let va = c.data("_pcre2_default_convert_context_8");
        let vb = r.data("_pcre2_default_convert_context_8");
        assert_eq!(
            (rd_u32(va, VC_GLOB_SEPARATOR), rd_u32(va, VC_GLOB_ESCAPE)),
            (rd_u32(vb, VC_GLOB_SEPARATOR), rd_u32(vb, VC_GLOB_ESCAPE)),
            "C018 convert_context glob separator/escape"
        );
        assert_eq!(rd_u32(va, VC_GLOB_SEPARATOR), b'/' as u32, "C018 glob separator");
        assert_eq!(rd_u32(va, VC_GLOB_ESCAPE), b'\\' as u32, "C018 glob escape");
        assert_eq!(VC_SIZE, 32);
        println!("C018 ok (pointer fields compared structurally, scalars byte-exact)");
    }
}

// =========================================================================
// C019  _pcre2_strlen_8
// =========================================================================
#[test]
fn c019_strlen() {
    println!("C019 _pcre2_strlen_8");
    let (c, r) = both();
    unsafe {
        let mut cases: Vec<Vec<u8>> = vec![
            b"\0".to_vec(),                                 // ""
            b"a\0".to_vec(),                                // "a"
            {
                let mut v = vec![b'x'; 255];
                v.push(0);
                v
            },
            {
                let mut v: Vec<u8> = (0x80u32..0x100).map(|b| b as u8).collect();
                v.push(0);
                v
            },
            {
                let mut v = vec![0u8];
                v.extend_from_slice(b"abc\0");
                v
            }, // NUL at index 0
        ];
        let mut rng = Rng::new(0x19);
        for _ in 0..400 {
            let n = rng.below(64) as usize;
            let mut v: Vec<u8> = (0..n).map(|_| 1 + rng.below(255) as u8).collect();
            v.push(0);
            v.extend_from_slice(&[7u8; 4]); // junk past the terminator
            cases.push(v);
        }
        for (i, s) in cases.iter().enumerate() {
            let a = (c._pcre2_strlen_8)(s.as_ptr());
            let b = (r._pcre2_strlen_8)(s.as_ptr());
            assert_eq!(a, b, "C019 case {}: C={} RUST={}", i, a, b);
        }
        println!("C019 ok: {} cases", cases.len());
    }
}

// =========================================================================
// C020 / C021  _pcre2_strcmp_8 / _pcre2_strcmp_c8_8
// =========================================================================
fn nul(s: &[u8]) -> Vec<u8> {
    let mut v = s.to_vec();
    v.push(0);
    v
}

fn strcmp_shapes() -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut v: Vec<(Vec<u8>, Vec<u8>)> = vec![
        (nul(b""), nul(b"")),                     // both empty
        (nul(b"abc"), nul(b"abc")),               // equal non-empty
        (nul(b"abc"), nul(b"abd")),               // differ at k, c1<c2
        (nul(b"abd"), nul(b"abc")),               // differ at k, c1>c2
        (nul(b"ab"), nul(b"abc")),                // str1 strict prefix of str2
        (nul(b"abc"), nul(b"ab")),                // str2 strict prefix of str1
        (nul(b""), nul(b"a")),
        (nul(b"a"), nul(b"")),
        (nul(b"\x7f"), nul(b"\x80")),
        (nul(b"\x80"), nul(b"\x7f")),
        (nul(b"\xff\xfe"), nul(b"\xff\xff")),
        (nul(b"\xff"), nul(b"\xff")),
    ];
    let mut rng = Rng::new(0x20);
    for _ in 0..400 {
        let n1 = rng.below(12) as usize;
        let n2 = rng.below(12) as usize;
        let a: Vec<u8> = (0..n1).map(|_| 1 + rng.below(255) as u8).collect();
        let mut b: Vec<u8> = (0..n2).map(|_| 1 + rng.below(255) as u8).collect();
        // often make b a mutation of a so differences land at interesting places
        if rng.below(2) == 0 && !a.is_empty() {
            b = a.clone();
            let k = rng.below(a.len() as u32) as usize;
            b[k] = 1 + rng.below(255) as u8;
        }
        v.push((nul(&a), nul(&b)));
    }
    v
}

#[test]
fn c020_strcmp() {
    println!("C020 _pcre2_strcmp_8");
    let (c, r) = both();
    unsafe {
        let cases = strcmp_shapes();
        for (i, (s1, s2)) in cases.iter().enumerate() {
            let a = (c._pcre2_strcmp_8)(s1.as_ptr(), s2.as_ptr());
            let b = (r._pcre2_strcmp_8)(s1.as_ptr(), s2.as_ptr());
            assert_eq!(a, b, "C020 case {} ({:?},{:?}): C={} RUST={}", i, s1, s2, a, b);
        }
        println!("C020 ok: {} cases", cases.len());
    }
}

#[test]
fn c021_strcmp_c8() {
    println!("C021 _pcre2_strcmp_c8_8");
    let (c, r) = both();
    unsafe {
        let mut cases = strcmp_shapes();
        // str2 bytes above 0x7f (const char* may be signed)
        for hi in [0x80u8, 0x81, 0xa0, 0xfe, 0xff] {
            cases.push((nul(&[hi]), nul(&[hi])));
            cases.push((nul(&[0x41]), nul(&[hi])));
            cases.push((nul(&[hi]), nul(&[0x41])));
            cases.push((nul(&[hi, 0x41]), nul(&[hi, 0x42])));
        }
        for (i, (s1, s2)) in cases.iter().enumerate() {
            let a = (c._pcre2_strcmp_c8_8)(s1.as_ptr(), s2.as_ptr() as *const i8);
            let b = (r._pcre2_strcmp_c8_8)(s1.as_ptr(), s2.as_ptr() as *const i8);
            assert_eq!(a, b, "C021 case {} ({:?},{:?}): C={} RUST={}", i, s1, s2, a, b);
        }
        println!("C021 ok: {} cases", cases.len());
    }
}

// =========================================================================
// C022 / C023  _pcre2_strncmp_8 / _pcre2_strncmp_c8_8
// =========================================================================
fn strncmp_shapes() -> Vec<(Vec<u8>, Vec<u8>, usize)> {
    let mut v: Vec<(Vec<u8>, Vec<u8>, usize)> = vec![
        (vec![], vec![], 0),                                    // len == 0: must not deref
        (nul(b"abc"), nul(b"xyz"), 0),
        (nul(b"abcd"), nul(b"abcd"), 4),                        // all len units equal
        (nul(b"abcd"), nul(b"abzd"), 4),                        // first difference at k<len
        (nul(b"abcd"), nul(b"abcz"), 4),                        // difference exactly at len-1
        (nul(b"abcd"), nul(b"abcz"), 3),                        // difference at index >= len
        (nul(b"ab\0cd"), nul(b"ab\0cd"), 5),                    // embedded NUL inside the span
        (nul(b"ab\0cd"), nul(b"ab\0ce"), 5),
        (nul(b"ab\0cd"), nul(b"ab\0"), 5),                      // NUL is not a terminator here
        (nul(b"\xff\xff"), nul(b"\xff\xfe"), 2),
    ];
    let mut rng = Rng::new(0x22);
    for _ in 0..400 {
        let n = 1 + rng.below(12) as usize;
        let a: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
        let mut b = a.clone();
        if rng.below(3) != 0 {
            let k = rng.below(n as u32) as usize;
            b[k] = rng.byte();
        }
        let len = rng.below(n as u32 + 1) as usize;
        v.push((nul(&a), nul(&b), len));
    }
    v
}

#[test]
fn c022_strncmp() {
    println!("C022 _pcre2_strncmp_8");
    let (c, r) = both();
    unsafe {
        let cases = strncmp_shapes();
        for (i, (s1, s2, len)) in cases.iter().enumerate() {
            let p1 = if s1.is_empty() { ptr::null() } else { s1.as_ptr() };
            let p2 = if s2.is_empty() { ptr::null() } else { s2.as_ptr() };
            let a = (c._pcre2_strncmp_8)(p1, p2, *len);
            let b = (r._pcre2_strncmp_8)(p1, p2, *len);
            assert_eq!(a, b, "C022 case {} len={}: C={} RUST={}", i, len, a, b);
        }
        println!("C022 ok: {} cases (including len==0 with NULL pointers)", cases.len());
    }
}

#[test]
fn c023_strncmp_c8() {
    println!("C023 _pcre2_strncmp_c8_8");
    let (c, r) = both();
    unsafe {
        let mut cases = strncmp_shapes();
        for hi in [0x80u8, 0xa0, 0xfe, 0xff] {
            cases.push((nul(&[hi, hi, hi]), nul(&[hi, hi, hi]), 3));
            cases.push((nul(&[0x41, hi]), nul(&[0x41, 0x42]), 2));
            cases.push((nul(&[0x41, 0x42]), nul(&[0x41, hi]), 2));
        }
        // the (*VERB) / pso_list prefix shapes the caller actually uses
        for lit in [
            &b"UTF)"[..], &b"UCP)"[..], &b"NO_START_OPT)"[..], &b"LIMIT_MATCH="[..],
        ] {
            cases.push((nul(b"UTF)abc"), nul(lit), lit.len()));
            cases.push((nul(lit), nul(lit), lit.len()));
        }
        for (i, (s1, s2, len)) in cases.iter().enumerate() {
            let p1 = if s1.is_empty() { ptr::null() } else { s1.as_ptr() };
            let p2 = if s2.is_empty() { ptr::null() } else { s2.as_ptr() as *const i8 };
            let a = (c._pcre2_strncmp_c8_8)(p1, p2, *len);
            let b = (r._pcre2_strncmp_c8_8)(p1, p2, *len);
            assert_eq!(a, b, "C023 case {} len={}: C={} RUST={}", i, len, a, b);
        }
        println!("C023 ok: {} cases", cases.len());
    }
}

// =========================================================================
// C024  _pcre2_strcpy_c8_8
// =========================================================================
#[test]
fn c024_strcpy_c8() {
    println!("C024 _pcre2_strcpy_c8_8");
    let (c, r) = both();
    unsafe {
        let mut cases: Vec<Vec<u8>> = vec![
            nul(b""),
            nul(b"a"),
            nul(b"abcdef"),
            nul(b"\x80\xff\xa0"),
            nul(b"mixed\x80ascii\xff"),
        ];
        let mut rng = Rng::new(0x24);
        for _ in 0..400 {
            let n = rng.below(40) as usize;
            let v: Vec<u8> = (0..n).map(|_| 1 + rng.below(255) as u8).collect();
            cases.push(nul(&v));
        }
        for (i, src) in cases.iter().enumerate() {
            let mut ba = vec![0xAAu8; 80];
            let mut bb = vec![0xAAu8; 80];
            let la = (c._pcre2_strcpy_c8_8)(ba.as_mut_ptr(), src.as_ptr() as *const i8);
            let lb = (r._pcre2_strcpy_c8_8)(bb.as_mut_ptr(), src.as_ptr() as *const i8);
            assert_eq!(la, lb, "C024 case {}: returned length C={} RUST={}", i, la, lb);
            cmp_blob("C024", &format!("case {} buffer", i), &ba, &bb);
            assert_eq!(la, src.len() - 1, "C024 case {}: length", i);
            assert_eq!(ba[la], 0, "C024 case {}: NUL terminator", i);
        }
        println!("C024 ok: {} cases", cases.len());
    }
}

// =========================================================================
// C025  _pcre2_ord2utf_8
// =========================================================================
#[test]
fn c025_ord2utf() {
    println!("C025 _pcre2_ord2utf_8");
    let (c, r) = both();
    unsafe {
        let mut vals: Vec<u32> = vec![
            0x0, 0x7f, 0x80, 0x7ff, 0x800, 0xffff, 0x10000, 0x10ffff, 0x1fffff, 0x200000,
            0x3ffffff, 0x4000000, 0x7fffffff, 0x80000000, 0xffffffff, 0xd800, 0xdfff, 0xd7ff,
            0xe000, 0x110000, 0x7ffffffe, 0x80000001,
        ];
        // every code point 0..=0x10FFFF
        vals.extend(0u32..=MAX_UTF_CODE_POINT);
        let mut rng = Rng::new(0x25);
        for _ in 0..2000 {
            vals.push(rng.next_u64() as u32);
        }
        for &cp in &vals {
            let mut ba = [0xAAu8; 16];
            let mut bb = [0xAAu8; 16];
            let la = (c._pcre2_ord2utf_8)(cp, ba.as_mut_ptr());
            let lb = (r._pcre2_ord2utf_8)(cp, bb.as_mut_ptr());
            if la != lb || ba != bb {
                panic!(
                    "C025 _pcre2_ord2utf_8(0x{:x}): C len={} bytes={:?}  RUST len={} bytes={:?}",
                    cp, la, &ba[..], lb, &bb[..]
                );
            }
        }
        println!("C025 ok: {} values (all of 0..=0x10FFFF plus boundaries)", vals.len());
    }
}

// =========================================================================
// C026-C029  _pcre2_valid_utf_8
// =========================================================================
unsafe fn valid_utf_case(c: &Api, r: &Api, id: &str, s: &[u8], len: usize, want: Option<(i32, usize)>) {
    let mut ea: usize = usize::MAX - 7;
    let mut eb: usize = usize::MAX - 7;
    let p = if s.is_empty() { ptr::null() } else { s.as_ptr() };
    let ra = (c._pcre2_valid_utf_8)(p, len, &mut ea);
    let rb = (r._pcre2_valid_utf_8)(p, len, &mut eb);
    if (ra, ea) != (rb, eb) {
        panic!(
            "{} _pcre2_valid_utf_8({:02x?}, len={}): C=(rc {}, erroroffset {}) RUST=(rc {}, erroroffset {})",
            id, s, len, ra, ea, rb, eb
        );
    }
    if let Some((wrc, woff)) = want {
        assert_eq!(ra, wrc, "{} {:02x?}: expected rc {} got {}", id, s, wrc, ra);
        if wrc != 0 {
            assert_eq!(ea, woff, "{} {:02x?}: expected erroroffset {} got {}", id, s, woff, ea);
        }
    }
}

#[test]
fn c026_valid_utf_valid() {
    println!("C026 _pcre2_valid_utf_8 — valid inputs");
    let (c, r) = both();
    unsafe {
        let cases: &[(&[u8], usize)] = &[
            (b"", 0),
            (b"abcdefghijklmnopqrstuvwxyz0123456789", 36),
            (b"\x00\x01\x7f", 3),
            (b"\xC2\x80", 2),
            (b"\xDF\xBF", 2),
            (b"\xE0\xA0\x80", 3),
            (b"\xEF\xBF\xBF", 3),
            (b"\xF0\x90\x80\x80", 4),
            (b"\xF4\x8F\xBF\xBF", 4),
            (b"a\xC2\x80b\xE0\xA0\x80c\xF0\x90\x80\x80d", 13),
            // a length shorter than the buffer
            (b"abc\xff\xff\xff", 3),
            (b"\xC2\x80\xff", 2),
            (b"\xF4\x8F\xBF\xBF\x80", 4),
        ];
        for &(s, len) in cases {
            valid_utf_case(c, r, "C026", s, len, Some((0, 0)));
        }
        // every valid code point, encoded on its own
        let mut n = 0;
        for cp in 0u32..=MAX_UTF_CODE_POINT {
            if (0xd800..=0xdfff).contains(&cp) {
                continue;
            }
            let e = utf8(cp);
            valid_utf_case(c, r, "C026", &e, e.len(), Some((0, 0)));
            n += 1;
        }
        println!("C026 ok: {} fixed shapes + {} single code points", cases.len(), n);
    }
}

#[test]
fn c027_valid_utf_truncated() {
    println!("C027 _pcre2_valid_utf_8 — truncation errors");
    let (c, r) = both();
    unsafe {
        let cases: &[(&[u8], i32, usize)] = &[
            (b"\xC2", 1, 0),
            (b"\xE1", 2, 0),
            (b"\xF1", 3, 0),
            (b"\xF9", 4, 0),
            (b"\xFD", 5, 0),
            (b"\xE1\x80", 1, 0), // still ERR1: missing 1 byte
            (b"\xF1\x80", 2, 0),
            (b"\xF1\x80\x80", 1, 0),
            (b"\xF9\x80", 3, 0),
            (b"\xF9\x80\x80", 2, 0),
            (b"\xF9\x80\x80\x80", 1, 0),
            (b"\xFD\x80", 4, 0),
            (b"\xFD\x80\x80", 3, 0),
            (b"\xFD\x80\x80\x80", 2, 0),
            (b"\xFD\x80\x80\x80\x80", 1, 0),
            // with a prefix so the erroroffset is non-zero
            (b"abc\xC2", 1, 3),
            (b"ab\xE1\x80", 1, 2),
        ];
        for &(s, n, off) in cases {
            valid_utf_case(c, r, "C027", s, s.len(), Some((utf8_err(n), off)));
        }
        println!("C027 ok: {} shapes covering ERR1..ERR5", cases.len());
    }
}

#[test]
fn c028_valid_utf_bad_continuation() {
    println!("C028 _pcre2_valid_utf_8 — bad continuation bytes");
    let (c, r) = both();
    unsafe {
        let cases: &[(&[u8], i32, usize)] = &[
            (b"\xC2\x41", 6, 0),
            (b"\xE1\x80\x41", 7, 0),
            (b"\xE1\x41\x80", 6, 0),
            (b"\xF1\x80\x80\x41", 8, 0),
            (b"\xF1\x80\x41\x80", 7, 0), // ERR7 for ab=3
            (b"\xF9\x80\x80\x80\x41", 9, 0),
            (b"\xF9\x80\x41\x80\x80", 7, 0), // ERR7 for ab=4
            (b"\xF9\x80\x80\x41\x80", 8, 0),
            (b"\xFD\x80\x80\x80\x80\x41", 10, 0),
            (b"\xFD\x80\x41\x80\x80\x80", 7, 0), // ERR7 for ab=5
            (b"\xFD\x80\x80\x41\x80\x80", 8, 0),
            (b"\xFD\x80\x80\x80\x41\x80", 9, 0),
            // with prefixes so every erroroffset subtraction (-1..-5) is checked
            (b"xy\xC2\x41", 6, 2),
            (b"xy\xE1\x80\x41", 7, 2),
            (b"xy\xF1\x80\x80\x41", 8, 2),
            (b"xy\xF9\x80\x80\x80\x41", 9, 2),
            (b"xy\xFD\x80\x80\x80\x80\x41", 10, 2),
        ];
        for &(s, n, off) in cases {
            valid_utf_case(c, r, "C028", s, s.len(), Some((utf8_err(n), off)));
        }
        println!("C028 ok: {} shapes covering ERR6..ERR10", cases.len());
    }
}

#[test]
fn c029_valid_utf_range_overlong_and_fuzz() {
    println!("C029 _pcre2_valid_utf_8 — range/overlong/surrogate + random fuzz");
    let (c, r) = both();
    unsafe {
        let cases: &[(&[u8], i32, usize)] = &[
            (b"\xF8\x88\x80\x80\x80", 11, 0),         // 5-byte char
            (b"\xFC\x84\x80\x80\x80\x80", 12, 0),     // 6-byte char
            (b"\xF5\x80\x80\x80", 13, 0),             // > 0x10ffff (c > 0xf4)
            (b"\xF4\x90\x80\x80", 13, 0),             // > 0x10ffff (c == 0xf4, d > 0x8f)
            (b"\xED\xA0\x80", 14, 0),                 // 0xd800
            (b"\xED\xBF\xBF", 14, 0),                 // 0xdfff
            (b"\xC0\x80", 15, 0),                     // overlong 2-byte
            (b"\xC1\xBF", 15, 0),
            (b"\xE0\x80\x80", 16, 0),                 // overlong 3-byte
            (b"\xE0\x9F\xBF", 16, 0),
            (b"\xF0\x80\x80\x80", 17, 0),             // overlong 4-byte
            (b"\xF0\x8F\xBF\xBF", 17, 0),
            (b"\xF8\x80\x80\x80\x80", 18, 0),         // overlong 5-byte
            (b"\xF8\x87\xBF\xBF\xBF", 18, 0),
            (b"\xFC\x80\x80\x80\x80\x80", 19, 0),     // overlong 6-byte
            (b"\xFC\x83\xBF\xBF\xBF\xBF", 19, 0),
            (b"\x80", 20, 0),                         // isolated 10xxxxxx
            (b"\xBF", 20, 0),
            (b"\xFE", 21, 0),
            (b"\xFF", 21, 0),
            (b"ab\xFE", 21, 2),
            (b"ab\x80", 20, 2),
            (b"ab\xF8\x88\x80\x80\x80", 11, 2),
            (b"ab\xFC\x84\x80\x80\x80\x80", 12, 2),
        ];
        for &(s, n, off) in cases {
            valid_utf_case(c, r, "C029", s, s.len(), Some((utf8_err(n), off)));
        }
        // confirm all 21 error numbers are reachable through the fixed shapes above
        // plus C027/C028 (ERR1..ERR10 are covered there).
        let mut seen = std::collections::BTreeSet::new();
        for &(_, n, _) in cases {
            seen.insert(n);
        }
        for n in 11..=21 {
            assert!(seen.contains(&n), "C029 ERR{} not covered", n);
        }

        // ---- large randomized corpus of raw byte strings
        let mut rng = Rng::new(0xC029);
        let mut counts = std::collections::BTreeMap::<i32, usize>::new();
        let iters = 100_000;
        for _ in 0..iters {
            let n = rng.below(41) as usize;
            let mut s: Vec<u8> = Vec::with_capacity(n);
            for _ in 0..n {
                // bias towards lead/continuation bytes so errors are dense
                let b = match rng.below(4) {
                    0 => rng.byte(),
                    1 => 0x80 | (rng.byte() & 0x3f),
                    2 => 0xc0 | (rng.byte() & 0x3f),
                    _ => rng.byte() & 0x7f,
                };
                s.push(b);
            }
            let mut ea: usize = usize::MAX - 7;
            let mut eb: usize = usize::MAX - 7;
            let p = if s.is_empty() { ptr::null() } else { s.as_ptr() };
            let ra = (c._pcre2_valid_utf_8)(p, s.len(), &mut ea);
            let rb = (r._pcre2_valid_utf_8)(p, s.len(), &mut eb);
            if (ra, ea) != (rb, eb) {
                panic!(
                    "C029 fuzz _pcre2_valid_utf_8({:02x?}): C=(rc {}, off {}) RUST=(rc {}, off {})",
                    s, ra, ea, rb, eb
                );
            }
            *counts.entry(ra).or_insert(0) += 1;
        }
        println!(
            "C029 ok: {} fixed shapes + {} random byte strings; rc histogram {:?}",
            cases.len(),
            iters,
            counts
        );
    }
}

// =========================================================================
// C030-C032  _pcre2_is_newline_8
// =========================================================================
unsafe fn drive_is_newline(id: &str, c: &Api, r: &Api, subj: &[u8], utf: i32, ty: u32, at: &[usize]) {
    let e = subj.as_ptr().add(subj.len());
    for &i in at {
        assert!(i < subj.len(), "{}: position {} out of range", id, i);
        let p = subj.as_ptr().add(i);
        let mut la: u32 = 0xDEAD_BEEF;
        let mut lb: u32 = 0xDEAD_BEEF;
        let ra = (c._pcre2_is_newline_8)(p, ty, e, &mut la, utf);
        let rb = (r._pcre2_is_newline_8)(p, ty, e, &mut lb, utf);
        if (ra, la) != (rb, lb) {
            panic!(
                "{} _pcre2_is_newline_8(subj={:02x?}, pos={}, type={}, utf={}): C=(rc {}, nllen {}) RUST=(rc {}, nllen {})",
                id, subj, i, ty, utf, ra, la, rb, lb
            );
        }
    }
}

#[test]
fn c030_is_newline_anycrlf() {
    println!("C030 _pcre2_is_newline_8 type=NLTYPE_ANYCRLF");
    let (c, r) = both();
    unsafe {
        // ---- non-UTF: any bytes, every position
        let subjects: Vec<Vec<u8>> = vec![
            b"\n".to_vec(),
            b"\r\n".to_vec(),
            b"a\r\nb".to_vec(),
            b"\r".to_vec(),
            b"a\r".to_vec(),   // CR as the last unit
            b"\rx".to_vec(),   // CR followed by non-LF
            b"\r\r\n".to_vec(),
            b"\x0b\x0c\x85\x00z".to_vec(),
            b"\x85".to_vec(),
            b"abc".to_vec(),
            b"\n\r\n\r".to_vec(),
        ];
        for s in &subjects {
            let at: Vec<usize> = (0..s.len()).collect();
            drive_is_newline("C030", c, r, s, 0, NLTYPE_ANYCRLF, &at);
        }
        // ---- UTF: valid UTF-8 subjects, character boundaries only
        let usubjects: Vec<Vec<u8>> = vec![
            utf8_str(&[0x0a]),
            utf8_str(&[0x0d, 0x0a]),
            utf8_str(&[0x61, 0x0d, 0x0a, 0x62]),
            utf8_str(&[0x0d]),
            utf8_str(&[0x61, 0x0d]),
            utf8_str(&[0x0d, 0x78]),
            utf8_str(&[0x85, 0x0a]),       // NEL then LF
            utf8_str(&[0x2028, 0x2029, 0x0a]),
            utf8_str(&[0x0b, 0x0c, 0x85, 0x2028, 0x2029, 0x41]),
        ];
        for s in &usubjects {
            let at = char_starts(s);
            drive_is_newline("C030", c, r, s, 1, NLTYPE_ANYCRLF, &at);
        }
        // ---- randomized
        let mut rng = Rng::new(0x30);
        let pool_b: [u8; 10] = [b'\n', b'\r', 0x0b, 0x0c, 0x85, b'a', 0x00, 0xe2, 0x80, 0xa8];
        for _ in 0..400 {
            let n = 1 + rng.below(10) as usize;
            let s: Vec<u8> = (0..n).map(|_| *rng.pick(&pool_b)).collect();
            let at: Vec<usize> = (0..s.len()).collect();
            drive_is_newline("C030", c, r, &s, 0, NLTYPE_ANYCRLF, &at);
        }
        let pool_c: [u32; 9] = [0x0a, 0x0d, 0x0b, 0x0c, 0x85, 0x2028, 0x2029, 0x61, 0x1F600];
        for _ in 0..400 {
            let n = 1 + rng.below(8) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool_c)).collect();
            let s = utf8_str(&cps);
            let at = char_starts(&s);
            drive_is_newline("C030", c, r, &s, 1, NLTYPE_ANYCRLF, &at);
        }
        println!("C030 ok");
    }
}

#[test]
fn c031_is_newline_any_nonutf() {
    println!("C031 _pcre2_is_newline_8 type=NLTYPE_ANY, utf=FALSE");
    let (c, r) = both();
    unsafe {
        let subjects: Vec<Vec<u8>> = vec![
            b"\n".to_vec(),
            b"\x0b".to_vec(),
            b"\x0c".to_vec(),
            b"\r\n".to_vec(),
            b"\r".to_vec(),
            b"a\r".to_vec(),
            b"\rz".to_vec(),
            b"\x85".to_vec(),
            b"\xe2".to_vec(),
            b"\xe2\x80\xa8".to_vec(),
            b"\xe2\x80\xa9".to_vec(),
            b"Q".to_vec(),
            b"\x00".to_vec(),
            b"\n\x0b\x0c\r\x85\xe2\x80\xa8Q".to_vec(),
        ];
        for s in &subjects {
            let at: Vec<usize> = (0..s.len()).collect();
            // NLTYPE_ANY plus every other non-ANYCRLF value takes the same branch
            for ty in [NLTYPE_ANY, NLTYPE_FIXED, 3, 4, 5, 6] {
                if ty == NLTYPE_ANYCRLF {
                    continue;
                }
                drive_is_newline("C031", c, r, s, 0, ty, &at);
            }
        }
        let mut rng = Rng::new(0x31);
        for _ in 0..600 {
            let n = 1 + rng.below(12) as usize;
            let s: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            let at: Vec<usize> = (0..s.len()).collect();
            drive_is_newline("C031", c, r, &s, 0, NLTYPE_ANY, &at);
        }
        println!("C031 ok");
    }
}

#[test]
fn c032_is_newline_any_utf() {
    println!("C032 _pcre2_is_newline_8 type=NLTYPE_ANY, utf=TRUE");
    let (c, r) = both();
    unsafe {
        let subjects: Vec<Vec<u8>> = vec![
            utf8_str(&[0x85]),          // NEL, nllen 2
            utf8_str(&[0x2028]),        // LS, nllen 3
            utf8_str(&[0x2029]),        // PS, nllen 3
            utf8_str(&[0x0a]),
            utf8_str(&[0x0b]),
            utf8_str(&[0x0c]),
            utf8_str(&[0x0d, 0x0a]),
            utf8_str(&[0x0d]),
            utf8_str(&[0x61, 0x0d]),
            utf8_str(&[0x0d, 0x7a]),
            utf8_str(&[0x51]),
            utf8_str(&[0x0a, 0x0b, 0x0c, 0x0d, 0x85, 0x2028, 0x2029, 0x51, 0x1F600]),
        ];
        for s in &subjects {
            let at = char_starts(s);
            for ty in [NLTYPE_ANY, NLTYPE_FIXED, 3, 4, 5, 6] {
                drive_is_newline("C032", c, r, s, 1, ty, &at);
            }
        }
        let mut rng = Rng::new(0x32);
        let pool: [u32; 12] = [
            0x0a, 0x0b, 0x0c, 0x0d, 0x85, 0x2028, 0x2029, 0x61, 0xff, 0x100, 0x2027, 0x1F600,
        ];
        for _ in 0..600 {
            let n = 1 + rng.below(8) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool)).collect();
            let s = utf8_str(&cps);
            let at = char_starts(&s);
            drive_is_newline("C032", c, r, &s, 1, NLTYPE_ANY, &at);
        }
        println!("C032 ok");
    }
}

// =========================================================================
// C033 / C034  _pcre2_was_newline_8
// =========================================================================
unsafe fn drive_was_newline(id: &str, c: &Api, r: &Api, subj: &[u8], utf: i32, ty: u32, at: &[usize]) {
    let st = subj.as_ptr();
    for &i in at {
        assert!(i > 0 && i <= subj.len(), "{}: position {} invalid", id, i);
        let p = st.add(i);
        let mut la: u32 = 0xDEAD_BEEF;
        let mut lb: u32 = 0xDEAD_BEEF;
        let ra = (c._pcre2_was_newline_8)(p, ty, st, &mut la, utf);
        let rb = (r._pcre2_was_newline_8)(p, ty, st, &mut lb, utf);
        if (ra, la) != (rb, lb) {
            panic!(
                "{} _pcre2_was_newline_8(subj={:02x?}, pos={}, type={}, utf={}): C=(rc {}, nllen {}) RUST=(rc {}, nllen {})",
                id, subj, i, ty, utf, ra, la, rb, lb
            );
        }
    }
}

/// Positions just after each character (1..=len at char boundaries).
fn char_ends(s: &[u8]) -> Vec<usize> {
    let mut v: Vec<usize> = char_starts(s).into_iter().skip(1).collect();
    v.push(s.len());
    v
}

#[test]
fn c033_was_newline_anycrlf() {
    println!("C033 _pcre2_was_newline_8 type=NLTYPE_ANYCRLF");
    let (c, r) = both();
    unsafe {
        let subjects: Vec<Vec<u8>> = vec![
            b"\r\n".to_vec(),   // LF preceded by CR, ptr>startptr
            b"\n".to_vec(),     // LF as the very first unit
            b"a\n".to_vec(),    // LF preceded by non-CR
            b"\r".to_vec(),
            b"a\r".to_vec(),
            b"z".to_vec(),
            b"\r\n\r\n".to_vec(),
            b"\x0b\x0c\x85Q".to_vec(),
        ];
        for s in &subjects {
            let at: Vec<usize> = (1..=s.len()).collect();
            drive_was_newline("C033", c, r, s, 0, NLTYPE_ANYCRLF, &at);
        }
        let usubjects: Vec<Vec<u8>> = vec![
            utf8_str(&[0x0d, 0x0a]),
            utf8_str(&[0x0a]),
            utf8_str(&[0x61, 0x0a]),
            utf8_str(&[0x0d]),
            utf8_str(&[0x85, 0x0a]),
            utf8_str(&[0x2028, 0x0d, 0x0a, 0x2029, 0x41]),
        ];
        for s in &usubjects {
            let at = char_ends(s);
            drive_was_newline("C033", c, r, s, 1, NLTYPE_ANYCRLF, &at);
        }
        let mut rng = Rng::new(0x33);
        let pool_b: [u8; 7] = [b'\n', b'\r', 0x0b, 0x0c, 0x85, b'a', 0x00];
        for _ in 0..400 {
            let n = 1 + rng.below(10) as usize;
            let s: Vec<u8> = (0..n).map(|_| *rng.pick(&pool_b)).collect();
            let at: Vec<usize> = (1..=s.len()).collect();
            drive_was_newline("C033", c, r, &s, 0, NLTYPE_ANYCRLF, &at);
        }
        let pool_c: [u32; 8] = [0x0a, 0x0d, 0x0b, 0x0c, 0x85, 0x2028, 0x2029, 0x61];
        for _ in 0..400 {
            let n = 1 + rng.below(8) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool_c)).collect();
            let s = utf8_str(&cps);
            let at = char_ends(&s);
            drive_was_newline("C033", c, r, &s, 1, NLTYPE_ANYCRLF, &at);
        }
        println!("C033 ok");
    }
}

#[test]
fn c034_was_newline_any() {
    println!("C034 _pcre2_was_newline_8 type=NLTYPE_ANY");
    let (c, r) = both();
    unsafe {
        let subjects: Vec<Vec<u8>> = vec![
            b"\r\n".to_vec(),
            b"a\n".to_vec(),
            b"\x0b".to_vec(),
            b"\x0c".to_vec(),
            b"\r".to_vec(),
            b"\x85".to_vec(),          // non-utf: nllen 1
            b"\xe2\x80\xa8".to_vec(),
            b"Q".to_vec(),
            b"\n\x0b\x0c\r\x85Q".to_vec(),
        ];
        for s in &subjects {
            let at: Vec<usize> = (1..=s.len()).collect();
            for ty in [NLTYPE_ANY, NLTYPE_FIXED, 3, 4, 5, 6] {
                drive_was_newline("C034", c, r, s, 0, ty, &at);
            }
        }
        let usubjects: Vec<Vec<u8>> = vec![
            utf8_str(&[0x0d, 0x0a]),
            utf8_str(&[0x61, 0x0a]),
            utf8_str(&[0x0b]),
            utf8_str(&[0x0c]),
            utf8_str(&[0x0d]),
            utf8_str(&[0x85]),          // utf: nllen 2
            utf8_str(&[0x2028]),        // nllen 3
            utf8_str(&[0x2029]),        // nllen 3
            utf8_str(&[0x51]),
            utf8_str(&[0x0a, 0x0b, 0x0c, 0x0d, 0x85, 0x2028, 0x2029, 0x51, 0x1F600]),
        ];
        for s in &usubjects {
            let at = char_ends(s);
            for ty in [NLTYPE_ANY, NLTYPE_FIXED, 3, 4, 5, 6] {
                drive_was_newline("C034", c, r, s, 1, ty, &at);
            }
        }
        let mut rng = Rng::new(0x34);
        for _ in 0..600 {
            let n = 1 + rng.below(12) as usize;
            let s: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            let at: Vec<usize> = (1..=s.len()).collect();
            drive_was_newline("C034", c, r, &s, 0, NLTYPE_ANY, &at);
        }
        let pool: [u32; 11] = [
            0x0a, 0x0b, 0x0c, 0x0d, 0x85, 0x2028, 0x2029, 0x61, 0xff, 0x100, 0x1F600,
        ];
        for _ in 0..600 {
            let n = 1 + rng.below(8) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool)).collect();
            let s = utf8_str(&cps);
            let at = char_ends(&s);
            drive_was_newline("C034", c, r, &s, 1, NLTYPE_ANY, &at);
        }
        println!("C034 ok");
    }
}

// =========================================================================
// C035-C038  _pcre2_extuni_8
// =========================================================================
/// Drive `_pcre2_extuni_8` starting at every character of `subj`.
unsafe fn drive_extuni(id: &str, c: &Api, r: &Api, subj: &[u8], utf: i32) {
    let st = subj.as_ptr();
    let end = st.add(subj.len());
    let starts: Vec<usize> = if utf != 0 {
        char_starts(subj)
    } else {
        (0..subj.len()).collect()
    };
    for (k, &i) in starts.iter().enumerate() {
        // first character value + the pointer just after it
        let (cp, adv) = if utf != 0 {
            let nxt = starts.get(k + 1).copied().unwrap_or(subj.len());
            let b = subj[i];
            let v = if b < 0x80 {
                b as u32
            } else {
                let n = nxt - i;
                let mut v = (b as u32) & (0x7f >> n);
                for j in 1..n {
                    v = (v << 6) | (subj[i + j] as u32 & 0x3f);
                }
                v
            };
            (v, nxt)
        } else {
            (subj[i] as u32, i + 1)
        };
        for &with_xcount in &[false, true] {
            let mut xa: i32 = 7;
            let mut xb: i32 = 7;
            let (pa, pb) = if with_xcount {
                (&mut xa as *mut i32, &mut xb as *mut i32)
            } else {
                (ptr::null_mut(), ptr::null_mut())
            };
            let ra = (c._pcre2_extuni_8)(cp, st.add(adv), st, end, utf, pa);
            let rb = (r._pcre2_extuni_8)(cp, st.add(adv), st, end, utf, pb);
            let oa = ra as usize - st as usize;
            let ob = rb as usize - st as usize;
            if oa != ob || xa != xb {
                panic!(
                    "{} _pcre2_extuni_8(c=0x{:x}, at={}, subj={:02x?}, utf={}, xcount={}): C=(end {}, xcount {}) RUST=(end {}, xcount {})",
                    id, cp, i, subj, utf, with_xcount, oa, xa, ob, xb
                );
            }
        }
    }
}

#[test]
fn c035_extuni_basic() {
    println!("C035 _pcre2_extuni_8 — gbtable cells, utf TRUE/FALSE, xcount NULL/non-NULL");
    let (c, r) = both();
    unsafe {
        // one representative code point per grapheme-break class (pcre2_ucp.h)
        let seqs: Vec<Vec<u32>> = vec![
            vec![0x61],                       // single character, eptr == end at entry
            vec![0x61, 0x62],                 // Other + Other -> break
            vec![0x61, 0x0301],               // Other + Extend -> join
            vec![0x61, 0x0903],               // Other + SpacingMark -> join
            vec![0x0d, 0x0a],                 // CR + LF -> join
            vec![0x0d, 0x61],                 // CR + non-LF -> break
            vec![0x01, 0x61],                 // Control + anything -> break
            vec![0x0a, 0x61],                 // LF + anything -> break
            vec![0x0600, 0x61],               // Prepend + Other -> join
            vec![0x0600, 0x0600, 0x61],
            vec![0x61, 0x0301, 0x0301, 0x0903, 0x62],
            vec![0x200d, 0x61],               // ZWJ + Other
            vec![0x61, 0x200d, 0x62],
            vec![0x1100, 0x61],
            vec![0x61, 0x0d, 0x0a, 0x62],
        ];
        for s in &seqs {
            let b = utf8_str(s);
            drive_extuni("C035", c, r, &b, 1);
        }
        // non-UTF: single bytes only
        let byte_subjects: Vec<Vec<u8>> = vec![
            b"a".to_vec(),
            b"ab".to_vec(),
            b"\r\n".to_vec(),
            b"\ra".to_vec(),
            b"\x01a".to_vec(),
            b"\nx".to_vec(),
            (0u8..=255).collect(),
        ];
        for s in &byte_subjects {
            drive_extuni("C035", c, r, s, 0);
        }
        // randomized, from a pool spanning grapheme-break classes
        let pool: [u32; 20] = [
            0x0d, 0x0a, 0x01, 0x0300, 0x0600, 0x0903, 0x1100, 0x1160, 0x11a8, 0xac00, 0xac01,
            0x1F1E6, 0x1F1E7, 0x61, 0x200d, 0x26a1, 0x1F468, 0x1F469, 0xfe0f, 0x0378,
        ];
        let mut rng = Rng::new(0x35);
        for _ in 0..600 {
            let n = 1 + rng.below(8) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool)).collect();
            drive_extuni("C035", c, r, &utf8_str(&cps), 1);
        }
        for _ in 0..300 {
            let n = 1 + rng.below(12) as usize;
            let s: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            drive_extuni("C035", c, r, &s, 0);
        }
        println!("C035 ok");
    }
}

#[test]
fn c036_extuni_zwj_pictographic() {
    println!("C036 _pcre2_extuni_8 — ZWJ / Extended_Pictographic latch");
    let (c, r) = both();
    unsafe {
        let seqs: Vec<Vec<u32>> = vec![
            vec![0x61, 0x200d, 0x26a1],                        // no preceding EP -> break
            vec![0x1F468, 0x200d, 0x1F469],                    // join
            vec![0x1F468, 0xfe0f, 0x200d, 0x1F469],            // one intervening Extend
            vec![0x1F468, 0xfe0f, 0xfe0f, 0x200d, 0x1F469],    // two
            vec![0x1F468, 0xfe0f, 0xfe0f, 0xfe0f, 0x200d, 0x1F469],
            vec![0x1F468, 0x0300, 0x200d, 0x1F469, 0x200d, 0x1F466],
            vec![0x26a1, 0x200d, 0x26a1, 0x200d, 0x26a1],
            vec![0x1F468, 0x200d, 0x61],
            vec![0x61, 0xfe0f, 0x200d, 0x26a1],
            vec![0x1F468, 0x200d, 0x200d, 0x1F469],
        ];
        for s in &seqs {
            drive_extuni("C036", c, r, &utf8_str(s), 1);
        }
        let pool: [u32; 8] = [0x1F468, 0x1F469, 0x26a1, 0x200d, 0xfe0f, 0x0300, 0x61, 0x1F3FB];
        let mut rng = Rng::new(0x36);
        for _ in 0..800 {
            let n = 1 + rng.below(9) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool)).collect();
            drive_extuni("C036", c, r, &utf8_str(&cps), 1);
        }
        println!("C036 ok");
    }
}

#[test]
fn c037_extuni_regional_indicators() {
    println!("C037 _pcre2_extuni_8 — Regional_Indicator back-scan");
    let (c, r) = both();
    unsafe {
        let ri = [0x1F1E6u32, 0x1F1E7, 0x1F1E8, 0x1F1E9];
        let seqs: Vec<Vec<u32>> = vec![
            vec![ri[0], ri[1]],
            vec![ri[0], ri[1], ri[2]],
            vec![ri[0], ri[1], ri[2], ri[3]],
            vec![ri[0], ri[1], ri[2], ri[3], ri[0]],
            vec![0x61, ri[0], ri[1], ri[2]],
            vec![0x61, ri[0], ri[1], ri[2], ri[3]],
            vec![ri[0]],
            vec![ri[0], 0x61],
            vec![ri[0], 0x0300, ri[1]],
            vec![0x1F468, ri[0], ri[1]],
        ];
        for s in &seqs {
            drive_extuni("C037", c, r, &utf8_str(s), 1);
        }
        let pool: [u32; 7] = [ri[0], ri[1], ri[2], ri[3], 0x61, 0x0300, 0x200d];
        let mut rng = Rng::new(0x37);
        for _ in 0..800 {
            let n = 1 + rng.below(10) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool)).collect();
            drive_extuni("C037", c, r, &utf8_str(&cps), 1);
        }
        println!("C037 ok");
    }
}

#[test]
fn c038_extuni_hangul() {
    println!("C038 _pcre2_extuni_8 — Hangul gbtable rows");
    let (c, r) = both();
    unsafe {
        // L=U+1100, V=U+1160, T=U+11A8, LV=U+AC00, LVT=U+AC01
        let (l, v, t, lv, lvt) = (0x1100u32, 0x1160u32, 0x11a8u32, 0xac00u32, 0xac01u32);
        let pairs: Vec<Vec<u32>> = vec![
            vec![l, l],
            vec![l, v],
            vec![l, lv],
            vec![l, lvt],
            vec![v, v],
            vec![v, t],
            vec![t, t],
            vec![lv, v],
            vec![lv, t],
            vec![lvt, t],
            vec![v, l],     // breaking pair
            vec![t, v],     // breaking pair
            vec![t, l],
            vec![lv, l],
            vec![lvt, v],
            vec![l, v, t, l, v, t],
            vec![l, l, v, v, t, t],
        ];
        for s in &pairs {
            drive_extuni("C038", c, r, &utf8_str(s), 1);
        }
        let pool = [l, v, t, lv, lvt, 0x61, 0x0300];
        let mut rng = Rng::new(0x38);
        for _ in 0..800 {
            let n = 1 + rng.below(8) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool)).collect();
            drive_extuni("C038", c, r, &utf8_str(&cps), 1);
        }
        println!("C038 ok");
    }
}

// =========================================================================
// C039-C042  _pcre2_script_run_8
// =========================================================================
unsafe fn drive_script_run(id: &str, c: &Api, r: &Api, subj: &[u8], utf: i32) {
    let st = subj.as_ptr();
    let bounds: Vec<usize> = if utf != 0 {
        let mut b = char_starts(subj);
        b.push(subj.len());
        b
    } else {
        (0..=subj.len()).collect()
    };
    for &i in &bounds {
        for &j in &bounds {
            if j < i {
                continue;
            }
            let ra = (c._pcre2_script_run_8)(st.add(i), st.add(j), utf);
            let rb = (r._pcre2_script_run_8)(st.add(i), st.add(j), utf);
            if ra != rb {
                panic!(
                    "{} _pcre2_script_run_8(subj={:02x?}, {}..{}, utf={}): C={} RUST={}",
                    id, subj, i, j, utf, ra, rb
                );
            }
        }
    }
}

#[test]
fn c039_script_run_shortcircuits() {
    println!("C039 _pcre2_script_run_8 — empty/one-char/Unknown/Common/Inherited");
    let (c, r) = both();
    unsafe {
        let seqs: Vec<Vec<u32>> = vec![
            vec![],
            vec![0x61],
            vec![0x0378],            // Unknown alone -> TRUE
            vec![0x61, 0x0378],      // Unknown at position >= 2 -> FALSE
            vec![0x0378, 0x61],
            vec![0x61, 0x21],        // Common, script check skipped
            vec![0x61, 0x0301],      // Inherited, skipped
            vec![0x21, 0x21, 0x21],
            vec![0x0301, 0x0301],
            vec![0x61, 0x21, 0x0301, 0x62],
        ];
        for s in &seqs {
            let b = utf8_str(s);
            drive_script_run("C039", c, r, &b, 1);
        }
        // non-UTF (single-byte code points)
        for s in [&b""[..], &b"a"[..], &b"ab"[..], &b"a!"[..], &b"AZaz09"[..], &b"\x80\x81"[..]] {
            drive_script_run("C039", c, r, s, 0);
        }
        println!("C039 ok");
    }
}

#[test]
fn c040_script_run_han_states() {
    println!("C040 _pcre2_script_run_8 — Han state machine");
    let (c, r) = both();
    unsafe {
        let han = 0x4E00u32;
        let han2 = 0x4E01u32;
        let hira = 0x3042u32;
        let kata = 0x30A2u32;
        let bopo = 0x3105u32;
        let hang = 0xAC00u32;
        let latin = 0x61u32;
        let seqs: Vec<Vec<u32>> = vec![
            vec![han, han2],
            vec![han, latin],
            vec![han, bopo],
            vec![han, hira],
            vec![han, kata],
            vec![han, hang],
            vec![han, hira, kata],
            vec![han, hira, bopo],
            vec![han, bopo, hira],
            vec![han, hang, hira],
            vec![hira, kata, han],
            vec![hira, bopo],
            vec![kata, hang],
            vec![bopo, han, bopo],
            vec![bopo, hira],
            vec![hang, han, hang],
            vec![hang, hira],
            vec![han, han, han, hira, kata, han],
            vec![0x3006, han],   // Common-with-extensions ideographic closing mark
            vec![0x3001, han, hira],
            vec![0x30FC, hira],  // prolonged sound mark: Hira+Kata extensions
        ];
        for s in &seqs {
            drive_script_run("C040", c, r, &utf8_str(s), 1);
        }
        let pool = [han, han2, hira, kata, bopo, hang, latin, 0x3006, 0x3001, 0x30FC, 0x0301, 0x21];
        let mut rng = Rng::new(0x40);
        for _ in 0..500 {
            let n = 1 + rng.below(6) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool)).collect();
            drive_script_run("C040", c, r, &utf8_str(&cps), 1);
        }
        println!("C040 ok");
    }
}

#[test]
fn c041_script_run_map_intersection() {
    println!("C041 _pcre2_script_run_8 — SCRIPT_MAP intersection");
    let (c, r) = both();
    unsafe {
        let seqs: Vec<Vec<u32>> = vec![
            vec![0x61, 0x62],                   // Latin + Latin
            vec![0x61, 0x0430],                 // Latin + Cyrillic -> FALSE
            vec![0x03B1, 0x03B2],               // Greek + Greek
            vec![0x03B1, 0x0430],               // Greek + Cyrillic -> FALSE
            vec![0x0964, 0x0905],               // Devanagari danda (multi-script) + Devanagari
            vec![0x0964, 0x0985],               // danda + Bengali
            vec![0x0964, 0x0905, 0x0985],       // narrows require_map twice -> FALSE
            vec![0x0966, 0x0905],
            vec![0x060C, 0x0627],               // Arabic comma (multi-script) + Arabic
            vec![0x060C, 0x0627, 0x05D0],       // + Hebrew -> FALSE
            vec![0x0483, 0x0430],               // Cyrillic combining + Cyrillic
            vec![0x061C, 0x0627],
        ];
        for s in &seqs {
            drive_script_run("C041", c, r, &utf8_str(s), 1);
        }
        let pool = [
            0x61u32, 0x0430, 0x03B1, 0x05D0, 0x0627, 0x0905, 0x0985, 0x0964, 0x0966, 0x060C,
            0x0483, 0x0E01, 0x10A0, 0x0531,
        ];
        let mut rng = Rng::new(0x41);
        for _ in 0..500 {
            let n = 1 + rng.below(5) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool)).collect();
            drive_script_run("C041", c, r, &utf8_str(&cps), 1);
        }
        println!("C041 ok");
    }
}

#[test]
fn c042_script_run_digitsets() {
    println!("C042 _pcre2_script_run_8 — digit-set consistency");
    let (c, r) = both();
    unsafe {
        let seqs: Vec<Vec<u32>> = vec![
            vec![0x31, 0x32],                     // "12": ASCII fast path
            vec![0x31, 0x0669],                   // different digit sets -> FALSE
            vec![0x0660, 0x0661],                 // same non-ASCII set
            vec![0x0660, 0x06F0],                 // two different non-ASCII sets
            vec![0x0966, 0x0967],
            vec![0x0966, 0x09E6],
            vec![0x1FBF9, 0x1FBF8],               // high digits: many chop iterations
            vec![0x1FBF9, 0x31],
            vec![0x31, 0x1FBF9],
            vec![0xFF11, 0xFF12],
            vec![0x0660, 0x0627, 0x0661],         // digits + Arabic letter
        ];
        for s in &seqs {
            drive_script_run("C042", c, r, &utf8_str(s), 1);
        }
        for s in [&b"12"[..], &b"1a2"[..], &b"09"[..]] {
            drive_script_run("C042", c, r, s, 0);
        }
        let pool = [
            0x30u32, 0x31, 0x39, 0x0660, 0x0669, 0x06F0, 0x0966, 0x09E6, 0x0A66, 0xFF10, 0x1FBF0,
            0x1FBF9, 0x0627, 0x61,
        ];
        let mut rng = Rng::new(0x42);
        for _ in 0..500 {
            let n = 1 + rng.below(5) as usize;
            let cps: Vec<u32> = (0..n).map(|_| *rng.pick(&pool)).collect();
            drive_script_run("C042", c, r, &utf8_str(&cps), 1);
        }
        println!("C042 ok");
    }
}

// =========================================================================
// C043-C047  _pcre2_xclass_8
// =========================================================================
/// Compile every pattern in `pats` with the C library, locate its class block
/// and run the sweep. Returns `(label, flags)` for each located block.
///
/// NOTE: the compiled blocks are freed before returning, so only the copied
/// metadata is handed back — never the pointers.
unsafe fn run_xclass_patterns(id: &str, c: &Api, r: &Api, pats: &[(&str, u32)]) -> Vec<(String, u8)> {
    let pts = sweep_points(0xC043 + id.len() as u64);
    let mut codes = vec![];
    let mut meta = vec![];
    for &(p, o) in pats {
        let code = compile_c(c, p, o);
        let blk = locate_class(code, p);
        assert_eq!(blk.op, OP_XCLASS, "{} [{}]: expected OP_XCLASS", id, p);
        sweep_xclass(id, c, r, &blk, &pts);
        meta.push((blk.label.clone(), blk.flags));
        codes.push(code);
    }
    for code in codes {
        (c.pcre2_code_free_8)(code);
    }
    meta
}

/// TRUE if a copied `ECL_XCLASS` item's nested XCLASS uses the legacy (non
/// character-list) encoding, i.e. it does not depend on `char_lists_end`.
fn item_is_legacy(item: &[u8]) -> bool {
    let mut i = 1 + LINK_SIZE;
    let flags = item[i];
    i += 1;
    if flags & XCL_MAP != 0 {
        i += 32;
    }
    while item[i] == XCL_PROP || item[i] == XCL_NOTPROP {
        i += 3;
    }
    item[i] < XCL_LIST_LO
}

fn rpn(parts: &[&[u8]]) -> Vec<u8> {
    let mut v = vec![0u8]; // flags: no ECL_MAP
    for p in parts {
        v.extend_from_slice(p);
    }
    v
}

#[test]
fn c043_xclass_bitmap() {
    println!("C043 _pcre2_xclass_8 — XCL_MAP present/absent, both polarities");
    let (c, r) = both();
    unsafe {
        let pats: &[(&str, u32)] = &[
            // XCL_MAP present, bits set and clear for c<256
            ("[a\\x{100}]", PCRE2_UTF),
            ("[^a\\x{100}]", PCRE2_UTF),
            ("[a-zA-Z\\x{100}-\\x{200}]", PCRE2_UTF),
            ("[^a-zA-Z\\x{100}-\\x{200}]", PCRE2_UTF),
            ("[\\p{L}a-c\\x{100}-\\x{200}\\x{5000}]", PCRE2_UTF),
            ("[^\\p{L}a-c\\x{100}-\\x{200}\\x{5000}]", PCRE2_UTF),
            // XCL_MAP absent
            ("[\\x{100}-\\x{200}]", PCRE2_UTF),
            ("[^\\x{100}-\\x{200}]", PCRE2_UTF),
            ("[\\p{L}]", PCRE2_UTF),
            ("[^\\p{L}]", PCRE2_UTF),
        ];
        let blks = run_xclass_patterns("C043", c, r, pats);
        let with_map = blks.iter().filter(|b| b.1 & XCL_MAP != 0).count();
        let without = blks.len() - with_map;
        let negated = blks.iter().filter(|b| b.1 & 1 != 0).count();
        assert!(with_map >= 3 && without >= 3, "C043 map coverage");
        assert!(negated >= 4, "C043 XCL_NOT coverage");
        println!(
            "C043 ok: {} blocks ({} with XCL_MAP, {} negated)",
            blks.len(),
            with_map,
            negated
        );
    }
}

#[test]
fn c044_xclass_properties() {
    println!("C044 _pcre2_xclass_8 — every PT_* switch arm, both polarities");
    let (c, r) = both();
    unsafe {
        let u = PCRE2_UTF;
        let uc = PCRE2_UTF | PCRE2_UCP;
        let pats: &[(&str, u32)] = &[
            ("[\\p{L&}]", u),
            ("[^\\p{L&}]", u),
            ("[\\P{L&}]", u),
            ("[\\p{L}]", u),
            ("[^\\p{L}]", u),
            ("[\\P{L}]", u),
            ("[^\\P{L}]", u),
            ("[\\p{Lu}]", u),
            ("[\\P{Lu}]", u),
            ("[^\\p{Lu}]", u),
            ("[\\p{sc=Han}]", u),
            ("[\\P{sc=Han}]", u),
            ("[^\\p{sc=Han}]", u),
            ("[\\p{scx=Han}]", u),
            ("[\\p{scx=Latin}]", u),
            ("[\\P{scx=Latin}]", u),
            ("[^\\p{scx=Latin}]", u),
            ("[\\p{Xan}]", u),
            ("[\\P{Xan}]", u),
            ("[\\p{Xsp}]", u),
            ("[\\P{Xsp}]", u),
            ("[\\p{Xps}]", u),
            ("[\\P{Xps}]", u),
            ("[\\p{Xwd}]", u),
            ("[\\P{Xwd}]", u),
            ("[\\p{Xuc}]", u),
            ("[\\P{Xuc}]", u),
            ("[^\\p{Xuc}]", u),
            ("[\\p{bc=AL}]", u),
            ("[\\P{bc=AL}]", u),
            ("[\\p{Bidi_Control}]", u),
            ("[\\P{Bidi_Control}]", u),
            ("[\\p{Zl}\\p{Zp}]", u),
            ("[\\p{Cf}]", u),
            ("[[:graph:]]", uc),
            ("[^[:graph:]]", uc),
            ("[[:print:]]", uc),
            ("[^[:print:]]", uc),
            ("[[:punct:]]", uc),
            ("[^[:punct:]]", uc),
            ("[[:xdigit:]]", uc),
            ("[^[:xdigit:]]", uc),
            // several properties in one list, plus a list where nothing matches so
            // control falls through to the range code
            ("[\\p{L}\\p{N}\\p{Lu}\\p{Xuc}]", u),
            ("[\\p{Zl}\\x{100}-\\x{200}\\x{5000}]", u),
            ("[^\\p{Zl}\\x{100}-\\x{200}\\x{5000}]", u),
        ];
        // check up front that all PT_* arms are represented
        let mut seen = std::collections::BTreeSet::new();
        let mut polarity = std::collections::BTreeSet::new();
        let mut codes = vec![];
        for &(p, o) in pats {
            let code = compile_c(c, p, o);
            let blk = locate_class(code, p);
            for (tag, pt, _pd) in xclass_props(&blk) {
                seen.insert(pt);
                polarity.insert((pt, tag));
            }
            codes.push(code);
        }
        for pt in [
            PT_LAMP, PT_GC, PT_PC, PT_SC, PT_SCX, PT_ALNUM, PT_SPACE, PT_PXSPACE, PT_WORD,
            PT_UCNC, PT_BIDICL, PT_BOOL, PT_PXGRAPH, PT_PXPRINT, PT_PXPUNCT, PT_PXXDIGIT,
        ] {
            assert!(seen.contains(&pt), "C044 PT_{} not produced by any pattern", pt);
        }
        assert!(
            polarity.iter().any(|&(_, t)| t == XCL_PROP)
                && polarity.iter().any(|&(_, t)| t == XCL_NOTPROP),
            "C044 need both XCL_PROP and XCL_NOTPROP"
        );
        for code in codes {
            (c.pcre2_code_free_8)(code);
        }

        let blks = run_xclass_patterns("C044", c, r, pats);
        println!(
            "C044 ok: {} blocks, PT_* arms exercised {:?}",
            blks.len(),
            seen
        );
    }
}

#[test]
fn c045_xclass_legacy_items() {
    println!("C045 _pcre2_xclass_8 — legacy XCL_SINGLE / XCL_RANGE item list");
    let (c, r) = both();
    unsafe {
        let u = PCRE2_UTF;
        let pats: &[(&str, u32)] = &[
            // NB: a lone `[\x{100}]` compiles to OP_CHAR, not OP_XCLASS
            ("[\\x{100}\\x{102}]", u),
            ("[^\\x{100}\\x{102}]", u),
            ("[\\x{100}-\\x{200}]", u),
            ("[^\\x{100}-\\x{200}]", u),
            ("[\\x{100}\\x{300}-\\x{400}\\x{500}]", u),
            ("[^\\x{100}\\x{300}-\\x{400}\\x{500}]", u),
            ("[\\x{8000}-\\x{8001}\\x{9000}-\\x{9001}\\x{a000}-\\x{a001}\\x{b000}]", u),
            ("[\\x{10000}-\\x{10001}\\x{20000}-\\x{20001}\\x{30000}\\x{40000}\\x{50000}]", u),
            ("[^\\x{10000}-\\x{10001}\\x{20000}-\\x{20001}\\x{30000}]", u),
            ("[\\x{100}-\\x{12000}]", u),
            ("[^\\x{100}-\\x{12000}]", u),
            ("[\\x{500}-\\x{9000}]", u),
            ("[\\x{100}-\\x{7ffe}\\x{8000}-\\x{fffe}\\x{10000}-\\x{10fffe}]", u),
            ("[\\p{L}\\x{100}]", u),
        ];
        // must not be list-encoded, or we would not be testing the legacy arm
        let mut legacy = 0;
        let mut codes = vec![];
        for &(p, o) in pats {
            let code = compile_c(c, p, o);
            let blk = locate_class(code, p);
            if !xclass_is_list(&blk) {
                legacy += 1;
            }
            codes.push(code);
        }
        for code in codes {
            (c.pcre2_code_free_8)(code);
        }
        assert!(legacy >= 10, "C045 only {} legacy-encoded patterns", legacy);
        let blks = run_xclass_patterns("C045", c, r, pats);
        println!("C045 ok: {} blocks ({} legacy-encoded)", blks.len(), legacy);
    }
}

#[test]
fn c046_xclass_list_16() {
    println!("C046 _pcre2_xclass_8 — XCL_LIST 16-bit search");
    let (c, r) = both();
    unsafe {
        let u = PCRE2_UTF;
        let pats: &[(&str, u32)] = &[
            // L16 count 3 (item-count escape hatch), BEGIN_WITH_RANGE etc.
            ("[\\x{100}-\\x{101}\\x{200}-\\x{201}\\x{300}-\\x{301}\\x{400}-\\x{401}\\x{500}-\\x{501}\\x{700}-\\x{701}]", u),
            ("[^\\x{100}-\\x{101}\\x{200}-\\x{201}\\x{300}-\\x{301}\\x{400}-\\x{401}\\x{500}-\\x{501}\\x{700}-\\x{701}]", u),
            ("[\\x{100}\\x{200}\\x{300}\\x{400}\\x{500}\\x{600}\\x{700}\\x{800}]", u),
            ("[^\\x{100}\\x{200}\\x{300}\\x{400}\\x{500}\\x{600}\\x{700}\\x{800}]", u),
            // H16 only: the low-16 block is skipped for c >= 0x8000
            ("[\\x{8000}\\x{8100}\\x{8200}\\x{8300}\\x{8400}\\x{8500}\\x{8600}\\x{8700}]", u),
            ("[^\\x{8000}\\x{8100}\\x{8200}\\x{8300}\\x{8400}\\x{8500}\\x{8600}\\x{8700}]", u),
            ("[\\x{8000}\\x{8002}\\x{8004}\\x{8006}\\x{8008}\\x{800a}\\x{800c}\\x{800e}\\x{8010}\\x{8012}\\x{8014}\\x{8016}]", u),
            // 20 singletons: a long 16-bit list, exercising all three binary-search arms
            ("[\\x{100}\\x{102}\\x{104}\\x{106}\\x{108}\\x{10a}\\x{10c}\\x{10e}\\x{110}\\x{112}\\x{114}\\x{116}\\x{118}\\x{11a}\\x{11c}\\x{11e}\\x{120}\\x{122}\\x{124}\\x{126}]", u),
            ("[^\\x{100}\\x{102}\\x{104}\\x{106}\\x{108}\\x{10a}\\x{10c}\\x{10e}\\x{110}\\x{112}\\x{114}\\x{116}\\x{118}\\x{11a}\\x{11c}\\x{11e}\\x{120}\\x{122}\\x{124}\\x{126}]", u),
            // mixed: L16 max_index 2, H16 max_index 2, L32 escape
            ("[\\x{100}\\x{200}\\x{8000}\\x{8100}\\x{10000}\\x{20000}\\x{30000}\\x{40000}]", u),
            ("[^\\x{100}\\x{200}\\x{8000}\\x{8100}\\x{10000}\\x{20000}\\x{30000}\\x{40000}]", u),
            // with an XCL_MAP and with properties in front of the list
            ("[a\\x{100}\\x{200}\\x{8000}\\x{8100}\\x{10000}\\x{20000}\\x{30000}\\x{40000}]", u),
            ("[\\p{L}\\x{100}\\x{200}\\x{8000}\\x{8100}\\x{10000}\\x{20000}\\x{30000}\\x{40000}]", u),
            // L16 max_index 0 (only 32-bit chars present)
            ("[\\x{10000}\\x{20000}\\x{30000}\\x{40000}\\x{50000}\\x{60000}\\x{70000}\\x{80000}]", u),
            // BEGIN_WITH_RANGE in both 16-bit halves
            ("[\\x{100}-\\x{12000}\\x{20000}\\x{30000}\\x{40000}\\x{50000}\\x{60000}]", u),
        ];
        let mut list = 0;
        let mut codes = vec![];
        for &(p, o) in pats {
            let code = compile_c(c, p, o);
            let blk = locate_class(code, p);
            if xclass_is_list(&blk) {
                list += 1;
            }
            codes.push(code);
        }
        for code in codes {
            (c.pcre2_code_free_8)(code);
        }
        assert_eq!(list, pats.len(), "C046 all patterns must be list-encoded");
        let blks = run_xclass_patterns("C046", c, r, pats);
        println!("C046 ok: {} list-encoded blocks", blks.len());
    }
}

#[test]
fn c047_xclass_list_32() {
    println!("C047 _pcre2_xclass_8 — XCL_LIST 32-bit search");
    let (c, r) = both();
    unsafe {
        let u = PCRE2_UTF;
        let pats: &[(&str, u32)] = &[
            ("[\\x{10000}\\x{20000}\\x{30000}\\x{40000}\\x{50000}\\x{60000}\\x{70000}\\x{80000}]", u),
            ("[^\\x{10000}\\x{20000}\\x{30000}\\x{40000}\\x{50000}\\x{60000}\\x{70000}\\x{80000}]", u),
            ("[\\x{10000}\\x{10002}\\x{10004}\\x{10006}\\x{10008}\\x{1000a}\\x{1000c}\\x{1000e}\\x{10010}\\x{10012}\\x{10014}]", u),
            ("[^\\x{10000}\\x{10002}\\x{10004}\\x{10006}\\x{10008}\\x{1000a}\\x{1000c}\\x{1000e}\\x{10010}\\x{10012}\\x{10014}]", u),
            ("[\\x{100}\\x{200}\\x{8000}\\x{8100}\\x{10000}\\x{20000}\\x{30000}\\x{40000}]", u),
            ("[\\x{100}-\\x{12000}\\x{20000}\\x{30000}\\x{40000}\\x{50000}\\x{60000}]", u),
            ("[^\\x{100}-\\x{12000}\\x{20000}\\x{30000}\\x{40000}\\x{50000}\\x{60000}]", u),
            ("[\\x{10000}-\\x{10001}\\x{20000}-\\x{20001}\\x{30000}-\\x{30001}\\x{40000}-\\x{40001}\\x{50000}-\\x{50001}\\x{60000}-\\x{60001}]", u),
            ("[\\x{fff00}\\x{fff02}\\x{fff04}\\x{fff06}\\x{fff08}\\x{fff0a}\\x{fff0c}\\x{fff0e}\\x{fff10}]", u),
            ("[a\\x{100}\\x{8000}\\x{10000}\\x{20000}\\x{30000}\\x{40000}\\x{50000}\\x{60000}]", u),
        ];
        let mut codes = vec![];
        for &(p, o) in pats {
            let code = compile_c(c, p, o);
            let blk = locate_class(code, p);
            assert!(xclass_is_list(&blk), "C047 [{}] is not list-encoded", p);
            codes.push(code);
        }
        for code in codes {
            (c.pcre2_code_free_8)(code);
        }
        let blks = run_xclass_patterns("C047", c, r, pats);
        println!("C047 ok: {} list-encoded blocks", blks.len());
    }
}

// =========================================================================
// C048  _pcre2_eclass_8
// =========================================================================
#[test]
fn c048_eclass() {
    println!("C048 _pcre2_eclass_8 — ECL_MAP + RPN programs");
    let (c, r) = both();
    unsafe {
        let u = PCRE2_UTF;
        let ux = PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS;
        let pats: &[(&str, u32)] = &[
            ("(?[[\\p{L}]|[\\x{100}]])", u),                                   // OR
            ("(?[[\\p{L}]&[\\p{Ll}\\x{100}]])", u),                            // AND
            ("(?[[\\p{L}]^[\\p{Ll}\\x{100}]])", u),                            // XOR
            ("(?[[\\p{L}]-[\\p{Ll}\\x{100}]])", u),
            ("(?[![\\p{L}\\x{100}]|[\\x{200}]])", u),
            ("(?[[\\p{L}]|[\\p{N}]|[\\x{2000}-\\x{3000}]])", u),
            ("(?[([\\p{L}]|[\\x{100}])&([\\p{N}]|[\\x{200}])])", u),
            ("(?[[a\\x{100}]|[b\\x{200}]])", u),                               // ECL_MAP present
            ("[[\\p{L}]&&[^q\\x{100}]]", ux),                                  // ECL_MAP present
            ("[[\\p{L}]--[\\p{Ll}\\x{100}]]", ux),
            ("[[\\p{L}]~~[\\p{Ll}\\x{100}]]", ux),
            // pushes the operand stack past depth 2
            ("(?[([\\p{L}]|[\\x{100}])|(([\\p{N}]|[\\x{200}])|([\\p{S}]|[\\x{300}]))])", u),
            ("(?[([\\p{L}]&[\\p{Ll}])|(([\\p{N}]&[\\x{200}-\\x{300}])^([\\p{S}]|[\\x{400}]))])", u),
            ("(?[!(!([\\p{L}]|[\\x{100}])|[\\x{200}])])", u),
        ];
        let pts = sweep_points(0xC048);
        let mut ops_seen = std::collections::BTreeSet::new();
        let mut maps = 0;
        for &(p, o) in pats {
            let code = compile_c(c, p, o);
            let blk = locate_class(code, p);
            assert_eq!(blk.op, OP_ECLASS, "C048 [{}] is not an OP_ECLASS", p);
            for op in eclass_ops(&blk) {
                ops_seen.insert(op);
            }
            if blk.flags & ECL_MAP != 0 {
                maps += 1;
            }
            sweep_eclass("C048", c, r, &blk, &pts);
            (c.pcre2_code_free_8)(code);
        }
        assert!(ops_seen.contains(&ECL_AND), "C048 no ECL_AND");
        assert!(ops_seen.contains(&ECL_OR), "C048 no ECL_OR");
        assert!(ops_seen.contains(&ECL_XOR), "C048 no ECL_XOR");
        assert!(ops_seen.contains(&ECL_XCLASS), "C048 no ECL_XCLASS");
        assert!(maps >= 2, "C048 need ECL_MAP-present blocks");

        // ------------------------------------------------------------------
        // The compiler never emits ECL_NOT (it folds negation into the nested
        // XCLASS's XCL_NOT flag) and never emits a body consisting of a single
        // ECL_XCLASS. To reach those arms, and to force the operand stack past
        // depth 2 with a known truth table, reassemble RPN programs out of
        // ECL_XCLASS items copied VERBATIM out of C-compiled blocks. The
        // `char_lists_end` argument is left pointing at the original block, so
        // any list-encoded nested XCLASS still resolves correctly.
        // ------------------------------------------------------------------
        // The three operand sets overlap pairwise but are not nested, so the sweep
        // reaches all four rows of every binary truth table.
        let mut keep = vec![];
        let mut items: Vec<Vec<u8>> = vec![];
        for &(p, o) in &[
            ("(?[[\\p{L}\\x{660}-\\x{669}]|[\\x{100}]])", u),
            ("(?[[\\p{N}\\x{100}-\\x{17f}]|[\\x{300}]])", u),
            ("(?[[\\p{S}\\x{100}-\\x{10f}]|[\\x{400}]])", u),
        ] {
            let code = compile_c(c, p, o);
            let blk = locate_class(code, p);
            assert_eq!(blk.op, OP_ECLASS, "C048 [{}] is not an OP_ECLASS", p);
            let it = eclass_item(&blk, 0);
            assert!(
                item_is_legacy(&it),
                "C048 harvested item from [{}] is list-encoded; copying it would \
                 make char_lists_end wrong",
                p
            );
            items.push(it);
            keep.push((code, blk.cle));
        }
        let cle = keep[0].1;
        let a = items[0].clone();
        let b = items[1].clone();
        let d = items[2].clone();
        let progs: Vec<(String, Vec<u8>)> = vec![
            ("single XCLASS".into(), rpn(&[&a[..]])),
            ("A B AND".into(), rpn(&[&a[..], &b[..], &[ECL_AND]])),
            ("A B OR".into(), rpn(&[&a[..], &b[..], &[ECL_OR]])),
            ("A B XOR".into(), rpn(&[&a[..], &b[..], &[ECL_XOR]])),
            ("A NOT".into(), rpn(&[&a[..], &[ECL_NOT]])),
            ("A NOT NOT".into(), rpn(&[&a[..], &[ECL_NOT], &[ECL_NOT]])),
            ("A B AND NOT".into(), rpn(&[&a[..], &b[..], &[ECL_AND], &[ECL_NOT]])),
            ("A B C OR AND".into(), rpn(&[&a[..], &b[..], &d[..], &[ECL_OR], &[ECL_AND]])),
            (
                "A B C NOT XOR OR".into(),
                rpn(&[&a[..], &b[..], &d[..], &[ECL_NOT], &[ECL_XOR], &[ECL_OR]]),
            ),
            (
                "A B C AND AND (depth 3)".into(),
                rpn(&[&a[..], &b[..], &d[..], &[ECL_AND], &[ECL_AND]]),
            ),
        ];
        // truth-table coverage of the binary operators
        let mut tt = std::collections::BTreeSet::new();
        let xa = a.as_ptr().add(1 + LINK_SIZE);
        let xb = b.as_ptr().add(1 + LINK_SIZE);
        let xd = d.as_ptr().add(1 + LINK_SIZE);
        let mut tt2 = std::collections::BTreeSet::new();
        for &cp in &pts {
            let va = (c._pcre2_xclass_8)(cp, xa, cle, 1) != 0;
            let vb = (c._pcre2_xclass_8)(cp, xb, cle, 1) != 0;
            let vd = (c._pcre2_xclass_8)(cp, xd, cle, 1) != 0;
            tt.insert((va, vb));
            tt2.insert((vb, vd));
        }
        assert_eq!(tt.len(), 4, "C048 A/B truth table only covered {:?}", tt);
        assert_eq!(tt2.len(), 4, "C048 B/C truth table only covered {:?}", tt2);

        for (name, prog) in &progs {
            let ds = prog.as_ptr();
            let de = ds.add(prog.len());
            for &utf in &[0i32, 1i32] {
                for &cp in &pts {
                    let ra = (c._pcre2_eclass_8)(cp, ds, de, cle, utf);
                    let rb = (r._pcre2_eclass_8)(cp, ds, de, cle, utf);
                    if ra != rb {
                        panic!(
                            "C048 synthesized program {:?}: _pcre2_eclass_8(c=0x{:x}, utf={}) C={} RUST={}",
                            name, cp, utf, ra, rb
                        );
                    }
                }
            }
        }
        // also with an ECL_MAP prefix copied from a real block
        let mapped = {
            let code = compile_c(c, "(?[[a\\x{100}]|[b\\x{200}]])", u);
            let blk = locate_class(code, "map source");
            let m = std::slice::from_raw_parts(blk.data.add(1), 32).to_vec();
            (c.pcre2_code_free_8)(code);
            m
        };
        let mut prog = vec![ECL_MAP];
        prog.extend_from_slice(&mapped);
        prog.extend_from_slice(&a);
        prog.extend_from_slice(&b);
        prog.push(ECL_OR);
        {
            let ds = prog.as_ptr();
            let de = ds.add(prog.len());
            for &utf in &[0i32, 1i32] {
                for &cp in &pts {
                    let ra = (c._pcre2_eclass_8)(cp, ds, de, cle, utf);
                    let rb = (r._pcre2_eclass_8)(cp, ds, de, cle, utf);
                    if ra != rb {
                        panic!(
                            "C048 synthesized ECL_MAP program: _pcre2_eclass_8(c=0x{:x}, utf={}) C={} RUST={}",
                            cp, utf, ra, rb
                        );
                    }
                }
            }
        }
        for (code, _) in keep {
            (c.pcre2_code_free_8)(code);
        }
        println!(
            "C048 ok: {} compiled blocks (ops {:?}, {} with ECL_MAP) + {} synthesized RPN programs incl. ECL_NOT",
            pats.len(),
            ops_seen,
            maps,
            progs.len() + 1
        );
    }
}

// =========================================================================
// C049  _pcre2_update_classbits_8
// =========================================================================
#[test]
fn c049_update_classbits() {
    println!("C049 _pcre2_update_classbits_8");
    let (c, r) = both();
    unsafe {
        // pdata is only meaningful for some properties; for PT_SCX and PT_BOOL it
        // indexes a fixed-width bitset row (4 and 2 uint32 words respectively -
        // UCD_MAPSIZE=4 from ucp_Unknown=99, boolprop rows are 2 words), so it
        // must stay inside that row or the C would read past the table.
        let cases: Vec<(u8, Vec<u32>)> = vec![
            (PT_ANY, vec![0]),
            (PT_LAMP, vec![0, 1, 255]),
            (PT_GC, (0..8u32).collect()),
            (PT_PC, (0..32u32).collect()),
            (PT_SC, (0..176u32).collect()),
            (PT_SCX, (0..128u32).collect()),
            (PT_ALNUM, vec![0, 1, 255]),
            (PT_SPACE, vec![0, 1, 255]),
            (PT_PXSPACE, vec![0, 1, 255]),
            (PT_WORD, vec![0, 1, 255]),
            (9, vec![0, 1]), // PT_CLIST — hits the `default:` arm
            (PT_UCNC, vec![0, 1, 255]),
            (PT_BIDICL, (0..32u32).collect()),
            (PT_BOOL, (0..64u32).collect()),
            (PT_PXGRAPH, vec![0, 1, 255]),
            (PT_PXPRINT, vec![0, 1, 255]),
            (PT_PXPUNCT, vec![0, 1, 255]),
            (PT_PXXDIGIT, vec![0, 1, 255]),
        ];
        let seeds: Vec<[u8; 32]> = vec![
            [0u8; 32],
            [0xffu8; 32],
            [0x0fu8; 32],
            {
                let mut v = [0u8; 32];
                for i in 0..32 {
                    v[i] = (i as u8) * 7;
                }
                v
            },
        ];
        let mut n = 0usize;
        for (ptype, pdatas) in &cases {
            for &pdata in pdatas {
                for &negated in &[0i32, 1i32] {
                    for seed in &seeds {
                        let mut ba = *seed;
                        let mut bb = *seed;
                        (c._pcre2_update_classbits_8)(*ptype as u32, pdata, negated, ba.as_mut_ptr());
                        (r._pcre2_update_classbits_8)(*ptype as u32, pdata, negated, bb.as_mut_ptr());
                        if ba != bb {
                            panic!(
                                "C049 _pcre2_update_classbits_8(ptype={}, pdata={}, negated={}, seed={:02x?}):\n  C   ={:02x?}\n  RUST={:02x?}",
                                ptype, pdata, negated, seed, ba, bb
                            );
                        }
                        n += 1;
                    }
                }
            }
        }
        // the documented PT_ANY early return
        {
            let seed = [0x5au8; 32];
            let mut b0 = seed;
            (c._pcre2_update_classbits_8)(PT_ANY as u32, 0, 0, b0.as_mut_ptr());
            assert_eq!(b0, [0xffu8; 32], "C049 PT_ANY negated=FALSE must memset 0xff");
            let mut b1 = seed;
            (c._pcre2_update_classbits_8)(PT_ANY as u32, 0, 1, b1.as_mut_ptr());
            assert_eq!(b1, seed, "C049 PT_ANY negated=TRUE must leave the buffer alone");
            let mut b2 = seed;
            (r._pcre2_update_classbits_8)(PT_ANY as u32, 0, 1, b2.as_mut_ptr());
            assert_eq!(b2, seed, "C049 RUST PT_ANY negated=TRUE must leave the buffer alone");
        }
        println!("C049 ok: {} calls", n);
    }
}

// =========================================================================
// C050  _pcre2_ckd_smul_8
// =========================================================================
#[test]
fn c050_ckd_smul() {
    println!("C050 _pcre2_ckd_smul_8");
    let (c, r) = both();
    unsafe {
        let mut pairs: Vec<(i32, i32)> = vec![
            (0, 0),
            (1, 1),
            (2, 3),
            (0, i32::MAX),
            (i32::MAX, 0),
            (65536, 65536),
            (100000, 100000),
            (i32::MAX, i32::MAX),
            (1, -1),
            (-1, 1),
            (-1, -1),
            (i32::MIN, 1),
            (1, i32::MIN),
            (i32::MIN, i32::MIN),
            (i32::MIN, i32::MAX),
            (i32::MAX, i32::MIN),
            (i32::MIN, -1),
            (-1, i32::MAX),
            (46341, 46341),
            (46340, 46340),
        ];
        let mut rng = Rng::new(0x50);
        for _ in 0..5000 {
            pairs.push((rng.next_u64() as i32, rng.next_u64() as i32));
        }
        for &(a, b) in &pairs {
            let mut ra: usize = 0xDEAD_BEEF_DEAD_BEEF;
            let mut rb: usize = 0xDEAD_BEEF_DEAD_BEEF;
            let x = (c._pcre2_ckd_smul_8)(&mut ra, a, b);
            let y = (r._pcre2_ckd_smul_8)(&mut rb, a, b);
            if (x, ra) != (y, rb) {
                panic!(
                    "C050 _pcre2_ckd_smul_8(a={}, b={}): C=(rc {}, r 0x{:x}) RUST=(rc {}, r 0x{:x})",
                    a, b, x, ra, y, rb
                );
            }
        }
        println!("C050 ok: {} pairs", pairs.len());
    }
}

// =========================================================================
// C051-C053  _pcre2_find_bracket_8
// =========================================================================
unsafe fn drive_find_bracket(
    id: &str,
    c: &Api,
    r: &Api,
    pat: &str,
    opts: u32,
    utfs: &[i32],
    numbers: &[i32],
) {
    let code = compile_c(c, pat, opts);
    let base = code as *const u8;
    let cs = rd_usize(base, RC_CODESTART);
    let bs = rd_usize(base, RC_BLOCKSIZE);
    let start = base.add(cs);
    let mut ncap: u32 = 0;
    (c.pcre2_pattern_info_8)(code, PCRE2_INFO_CAPTURECOUNT, &mut ncap as *mut u32 as Ptr);
    let mut nums: Vec<i32> = numbers.to_vec();
    for n in 1..=(ncap as i32 + 2) {
        if !nums.contains(&n) {
            nums.push(n);
        }
    }
    for &utf in utfs {
        for &n in &nums {
            let a = (c._pcre2_find_bracket_8)(start, utf, n);
            let b = (r._pcre2_find_bracket_8)(start, utf, n);
            let oa = if a.is_null() { None } else { Some(a as usize - base as usize) };
            let ob = if b.is_null() { None } else { Some(b as usize - base as usize) };
            if oa != ob {
                panic!(
                    "{} _pcre2_find_bracket_8({:?}, utf={}, number={}): C={:?} RUST={:?}",
                    id, pat, utf, n, oa, ob
                );
            }
            if let Some(o) = oa {
                assert!(
                    o >= cs && o < bs,
                    "{} {:?}: returned offset {} outside the code block [{},{})",
                    id, pat, o, cs, bs
                );
            }
        }
    }
    (c.pcre2_code_free_8)(code);
}

#[test]
fn c051_find_bracket_capture_groups() {
    println!("C051 _pcre2_find_bracket_8 — OP_CBRA/SCBRA/CBRAPOS/SCBRAPOS");
    let (c, r) = both();
    unsafe {
        let cases: &[(&str, u32)] = &[
            ("(a)", 0),
            ("(a)(b)(c)", 0),
            ("(a*)", 0),
            ("(a)++", 0),
            ("(?>(a))", 0),
            ("(a)(?<n>b)(c)(d)(e)(f)(g)(h)(i)(j)(k)", 0),
            ("(a(b(c)))", 0),
            ("(?:x)(a)", 0),
            ("((a)|(b))*", 0),
            ("(a)?(b)+(c){2,3}", 0),
        ];
        for &(p, o) in cases {
            drive_find_bracket("C051", c, r, p, o, &[0, 1], &[0, -1, 1, 2, 3, 4, 99]);
        }
        println!("C051 ok: {} patterns", cases.len());
    }
}

#[test]
fn c052_find_bracket_lookbehind() {
    println!("C052 _pcre2_find_bracket_8 — OP_REVERSE / OP_VREVERSE with number<0");
    let (c, r) = both();
    unsafe {
        let cases: &[(&str, u32)] = &[
            ("(?<=abc)x", 0),
            ("(?<=ab|abc)x", 0),
            ("(?<=abc)(x)", 0),
            ("(?<=ab|abc)(x)", 0),
            ("(?<!abc)x", 0),
            ("(?<=a(b)c)x", 0),
            ("(?<=a|bb|ccc)(x)(y)", 0),
            ("x(?<=abc)", 0),
        ];
        for &(p, o) in cases {
            drive_find_bracket("C052", c, r, p, o, &[0, 1], &[-1, -2, -99, 1, 2]);
        }
        println!("C052 ok: {} patterns", cases.len());
    }
}

#[test]
fn c053_find_bracket_variable_length_opcodes() {
    println!("C053 _pcre2_find_bracket_8 — variable-length opcodes");
    let (c, r) = both();
    unsafe {
        // ASCII-only compiled code: both utf flags are safe (no code unit >= 0xc0
        // sits in a character position, so the MAYBE_UTF_MULTI block is a no-op).
        let ascii: &[(&str, u32)] = &[
            ("(?C{s})(x)", 0),
            ("(?C{some longer string})(x)(y)", 0),
            ("\\d+(x)", 0),
            ("\\d{2,5}(x)", 0),
            ("\\d{3}(x)", 0),
            ("(*MARK:abc)(x)", 0),
            ("(*COMMIT:a)(x)", 0),
            ("(*PRUNE:a)(x)", 0),
            ("(*SKIP:a)(x)", 0),
            ("(*THEN:a)(x)", 0),
            ("(*MARK:aaaaaaaaaaaaaaaa)(x)(y)", 0),
        ];
        for &(p, o) in ascii {
            drive_find_bracket("C053", c, r, p, o, &[0, 1], &[-1, 1, 2, 3]);
        }
        // UTF-compiled code must be walked with utf=TRUE (that is how every real
        // caller invokes it); walking UTF code with utf=FALSE would desynchronise
        // the scan and read outside the block.
        let utf_pats: &[(&str, u32)] = &[
            ("[\\x{100}-\\x{200}](x)", PCRE2_UTF),
            ("[^\\x{100}-\\x{200}](x)", PCRE2_UTF),
            ("[[\\p{L}]&&[^q\\x{100}]](x)", PCRE2_UTF | PCRE2_ALT_EXTENDED_CLASS),
            ("\\p{L}+(x)", PCRE2_UTF),
            ("\\P{L}+(x)", PCRE2_UTF),
            ("\\p{L}{2,5}(x)", PCRE2_UTF),
            ("\\p{L}{3}(x)", PCRE2_UTF),
            ("\\x{e9}(x)", PCRE2_UTF),
            ("\\x{e9}+(x)", PCRE2_UTF),
            ("\\x{e9}{2,3}(x)", PCRE2_UTF),
            ("[^\\x{e9}]*(x)", PCRE2_UTF),
            ("\\x{1F600}(x)(y)", PCRE2_UTF),
            ("\\x{1F600}{2,4}(x)", PCRE2_UTF),
            ("(?i)\\x{e9}+(x)", PCRE2_UTF),
            ("\\x{e9}?+(x)", PCRE2_UTF),
        ];
        for &(p, o) in utf_pats {
            drive_find_bracket("C053", c, r, p, o, &[1], &[-1, 1, 2, 3]);
        }
        println!("C053 ok: {} ASCII + {} UTF patterns", ascii.len(), utf_pats.len());
    }
}

// =========================================================================
// C054  _pcre2_compile_get_hash_from_name8
// =========================================================================
#[test]
fn c054_get_hash_from_name() {
    println!("C054 _pcre2_compile_get_hash_from_name8");
    let (c, r) = both();
    unsafe {
        // exact prototype: uint16_t f(PCRE2_SPTR, uint32_t)
        type HashFn = unsafe extern "C" fn(Sptr, u32) -> u16;
        let hc: HashFn = c.raw("_pcre2_compile_get_hash_from_name8");
        let hr: HashFn = r.raw("_pcre2_compile_get_hash_from_name8");

        let mut names: Vec<Vec<u8>> = vec![
            b"a".to_vec(),                       // length 1: first == last
            b"\x00".to_vec(),
            b"\xff".to_vec(),                    // masked-off high bit / max hash inputs
            b"ab".to_vec(),                      // length 2
            b"abc".to_vec(),
            b"axc".to_vec(),                     // must collide with "abc"
            b"a_very_long_capture_group_name_c".to_vec(),
            b"\x80bc".to_vec(),                  // first byte has 0x80 set (masked off)
            b"\x00bc".to_vec(),
            b"ab\xff".to_vec(),                  // last byte 0xff -> maximum hash
            b"\x7f\x00\xff".to_vec(),
        ];
        let mut rng = Rng::new(0x54);
        for _ in 0..1000 {
            let n = 1 + rng.below(40) as usize;
            names.push((0..n).map(|_| rng.byte()).collect());
        }
        for nm in &names {
            let a = hc(nm.as_ptr(), nm.len() as u32);
            let b = hr(nm.as_ptr(), nm.len() as u32);
            assert_eq!(
                a, b,
                "C054 hash({:02x?}, {}): C=0x{:04x} RUST=0x{:04x}",
                nm, nm.len(), a, b
            );
            // the documented formula, and the NAMED_GROUP_HASH_MASK bound
            let want = ((nm[0] & 0x7f) as u16) | (((nm[nm.len() - 1] & 0xff) as u16) << 7);
            assert_eq!(a, want, "C054 hash({:02x?}) formula", nm);
            assert!(a <= 0x7fff, "C054 hash 0x{:04x} exceeds 0x7fff", a);
        }
        // the collision the row calls for
        assert_eq!(hc(b"abc".as_ptr(), 3), hc(b"axc".as_ptr(), 3), "C054 abc/axc collision (C)");
        assert_eq!(hr(b"abc".as_ptr(), 3), hr(b"axc".as_ptr(), 3), "C054 abc/axc collision (RUST)");
        assert_eq!(hc(b"\xff".as_ptr(), 1), 0x7fff, "C054 maximum hash");
        println!("C054 ok: {} names", names.len());
    }
}

// =========================================================================
// C055  _pcre2_memctl_malloc_8
// =========================================================================
#[repr(C)]
#[derive(Clone, Copy)]
struct Memctl {
    malloc: Option<unsafe extern "C" fn(usize, Ptr) -> Ptr>,
    free: Option<unsafe extern "C" fn(Ptr, Ptr)>,
    memory_data: Ptr,
}

static MY_MALLOC_CALLS: AtomicUsize = AtomicUsize::new(0);
static MY_FREE_CALLS: AtomicUsize = AtomicUsize::new(0);
static MY_NULL_CALLS: AtomicUsize = AtomicUsize::new(0);

/// Over-allocates so the library's unconditional 24-byte header write is always
/// in bounds even for the "size smaller than sizeof(pcre2_memctl)" shape.
unsafe extern "C" fn my_malloc(size: usize, _data: Ptr) -> Ptr {
    MY_MALLOC_CALLS.fetch_add(1, Ordering::SeqCst);
    let n = if size < 64 { 64 } else { size };
    let layout = std::alloc::Layout::from_size_align(n + 16, 16).unwrap();
    let p = std::alloc::alloc(layout);
    assert!(!p.is_null());
    *(p as *mut usize) = n + 16;
    p.add(16) as Ptr
}

unsafe extern "C" fn my_free(block: Ptr, _data: Ptr) {
    MY_FREE_CALLS.fetch_add(1, Ordering::SeqCst);
    if block.is_null() {
        return;
    }
    let p = (block as *mut u8).sub(16);
    let n = *(p as *const usize);
    let layout = std::alloc::Layout::from_size_align(n, 16).unwrap();
    std::alloc::dealloc(p, layout);
}

unsafe extern "C" fn null_malloc(_size: usize, _data: Ptr) -> Ptr {
    MY_NULL_CALLS.fetch_add(1, Ordering::SeqCst);
    ptr::null_mut()
}

unsafe extern "C" fn never_free(_block: Ptr, _data: Ptr) {
    panic!("C055 never_free must not be called");
}

#[test]
fn c055_memctl_malloc() {
    println!("C055 _pcre2_memctl_malloc_8");
    let (c, r) = both();
    unsafe {
        let memctl_size = std::mem::size_of::<Memctl>();
        assert_eq!(memctl_size, 24, "C055 sizeof(pcre2_memctl)");

        // each library's own default_malloc / default_free
        let defaults = |api: &Api| -> (*const u8, *const u8) {
            let gc = (api.pcre2_general_context_create_8)(None, None, ptr::null_mut());
            assert!(!gc.is_null());
            let p = gc as *const u8;
            let d = (rd_ptr(p, 0), rd_ptr(p, 8));
            (api.pcre2_general_context_free_8)(gc);
            d
        };
        let dflt = [defaults(c), defaults(r)];

        // ---- 1. memctl == NULL: plain malloc + the library's own defaults.
        // Sizes are kept >= sizeof(pcre2_memctl) here because the C writes 24
        // bytes into the block unconditionally; a smaller request would be a
        // heap overflow in the *C* reference, not something the Rust can differ on.
        for &size in &[24usize, 25, 32, 64, 1000, 65536] {
            for (i, api) in [c, r].iter().enumerate() {
                let p = (api._pcre2_memctl_malloc_8)(size, ptr::null_mut());
                assert!(!p.is_null(), "C055 {} NULL-memctl size {} returned NULL", api.tag, size);
                let h = p as *const u8;
                assert_eq!(
                    rd_ptr(h, 0), dflt[i].0,
                    "C055 {} header malloc must be that library's default_malloc",
                    api.tag
                );
                assert_eq!(
                    rd_ptr(h, 8), dflt[i].1,
                    "C055 {} header free must be that library's default_free",
                    api.tag
                );
                assert!(rd_ptr(h, 16).is_null(), "C055 {} header memory_data", api.tag);
                // free through the free function the library itself installed
                let f: unsafe extern "C" fn(Ptr, Ptr) =
                    std::mem::transmute(rd_ptr(h, 8) as *const ());
                f(p, ptr::null_mut());
            }
        }

        // ---- 2. memctl with the library's own default malloc/free
        for &size in &[1usize, 8, 16, 23, 24, 25, 64, 4096] {
            for (i, api) in [c, r].iter().enumerate() {
                let mut mc = Memctl {
                    malloc: Some(std::mem::transmute(dflt[i].0 as *const ())),
                    free: Some(std::mem::transmute(dflt[i].1 as *const ())),
                    memory_data: ptr::null_mut(),
                };
                if size < 24 {
                    continue; // default malloc gives no slack for the header write
                }
                let p = (api._pcre2_memctl_malloc_8)(size, &mut mc as *mut Memctl as Ptr);
                assert!(!p.is_null(), "C055 {} default-memctl size {}", api.tag, size);
                let h = p as *const u8;
                assert_eq!(rd_ptr(h, 0), dflt[i].0, "C055 {} copied malloc", api.tag);
                assert_eq!(rd_ptr(h, 8), dflt[i].1, "C055 {} copied free", api.tag);
                assert!(rd_ptr(h, 16).is_null(), "C055 {} copied memory_data", api.tag);
                (mc.free.unwrap())(p, mc.memory_data);
            }
        }

        // ---- 3. custom malloc/free with non-NULL memory_data, all three fields copied.
        // Both libraries must produce a block whose hidden prefix holds the same
        // memctl copy.
        let cookie = 0x1234_5678usize as Ptr;
        let mm: unsafe extern "C" fn(usize, Ptr) -> Ptr = my_malloc;
        let mf: unsafe extern "C" fn(Ptr, Ptr) = my_free;
        let mm_addr = mm as usize as *const u8;
        let mf_addr = mf as usize as *const u8;
        for &size in &[1usize, 8, 16, 23, 24, 25, 40, 4096, 100000] {
            let mc0 = Memctl {
                malloc: Some(my_malloc),
                free: Some(my_free),
                memory_data: cookie,
            };
            let mut hdr: Vec<[u8; 24]> = vec![];
            let mut blocks: Vec<Ptr> = vec![];
            for api in [c, r] {
                let mut mc = mc0;
                let before = MY_MALLOC_CALLS.load(Ordering::SeqCst);
                let p = (api._pcre2_memctl_malloc_8)(size, &mut mc as *mut Memctl as Ptr);
                assert_eq!(
                    MY_MALLOC_CALLS.load(Ordering::SeqCst),
                    before + 1,
                    "C055 {} must call the custom malloc exactly once",
                    api.tag
                );
                assert!(!p.is_null(), "C055 {} custom-memctl size {}", api.tag, size);
                let mut h = [0u8; 24];
                ptr::copy_nonoverlapping(p as *const u8, h.as_mut_ptr(), 24);
                assert_eq!(rd_ptr(p as *const u8, 0), mm_addr, "C055 {} copied malloc", api.tag);
                assert_eq!(rd_ptr(p as *const u8, 8), mf_addr, "C055 {} copied free", api.tag);
                assert_eq!(
                    rd_ptr(p as *const u8, 16),
                    cookie as *const u8,
                    "C055 {} copied memory_data",
                    api.tag
                );
                hdr.push(h);
                blocks.push(p);
            }
            cmp_blob("C055", &format!("memctl header (size {})", size), &hdr[0], &hdr[1]);
            for p in blocks {
                my_free(p, cookie);
            }
        }

        // ---- 4. custom malloc returning NULL: the early return skips the header write
        for &size in &[0usize, 1, 24, 4096] {
            let mut results = vec![];
            for api in [c, r] {
                let mut mc = Memctl {
                    malloc: Some(null_malloc),
                    free: Some(never_free),
                    memory_data: ptr::null_mut(),
                };
                let before = MY_NULL_CALLS.load(Ordering::SeqCst);
                let p = (api._pcre2_memctl_malloc_8)(size, &mut mc as *mut Memctl as Ptr);
                assert_eq!(
                    MY_NULL_CALLS.load(Ordering::SeqCst),
                    before + 1,
                    "C055 {} must call the custom malloc",
                    api.tag
                );
                results.push(p);
            }
            assert!(
                results[0].is_null() && results[1].is_null(),
                "C055 size {}: both libraries must return NULL, got {:?}",
                size,
                results
            );
        }
        println!(
            "C055 ok (custom malloc calls {}, frees {}, NULL-returns {})",
            MY_MALLOC_CALLS.load(Ordering::SeqCst),
            MY_FREE_CALLS.load(Ordering::SeqCst),
            MY_NULL_CALLS.load(Ordering::SeqCst)
        );
    }
}
