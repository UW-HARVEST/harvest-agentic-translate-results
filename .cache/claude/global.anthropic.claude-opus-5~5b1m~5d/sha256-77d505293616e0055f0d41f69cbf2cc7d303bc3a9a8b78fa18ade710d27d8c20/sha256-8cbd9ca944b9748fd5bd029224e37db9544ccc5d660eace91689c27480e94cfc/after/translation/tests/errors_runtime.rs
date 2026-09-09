//! Differential test for every REACHABLE row of `_v/err_runtime.tsv` (the runtime
//! error-surface table): 300 rows total, 33 marked UNREACHABLE (skipped with a
//! reason), 267 executed here.
//!
//! Each row's RECIPE is executed twice — once against the C reference `.so` and once
//! against the Rust `.so` — and three things are asserted per row:
//!
//!   1. the C library returns exactly the result tabulated in the TSV (if this fails
//!      the TABLE is stale, which is reported, never silently adjusted);
//!   2. the Rust library returns the identical result;
//!   3. every output argument the call touches holds identical values in both
//!      libraries (`*outlengthptr` after a failed `pcre2_substitute_8`, the ovector
//!      after a failed match, `*erroroffset`, the output buffers, ...).
//!
//! The recipes are ports of `_v/checkrt.c`, which implements the same 267 rows in the
//! same order against the C library and runs clean (`271 checks, 0 MISMATCH`).
//!
//! The TSV itself is `include_str!`d and parsed, so the expected result, the C source
//! location, the trigger condition and the UNREACHABLE marking used in the assertion
//! messages come straight from the table — no transcription drift is possible.

mod common;
use common::*;

use std::alloc::{alloc, dealloc, Layout};
use std::cell::Cell;
use std::ptr;
use std::sync::OnceLock;

// ===================================================================== the table

const TSV: &str = include_str!("../../_v/err_runtime.tsv");

#[derive(Debug)]
struct Row {
    n: usize,
    func: &'static str,
    expect: &'static str,
    loc: &'static str,
    trigger: &'static str,
    recipe: &'static str,
    notes: &'static str,
    unreachable: bool,
    /// The numeric result the row expects (the last integer literal in field 2).
    val: Option<i64>,
    /// True when field 2 is just `NULL` (a pointer-returning row).
    wants_null: bool,
}

/// Last integer literal appearing in `s` (`PCRE2_ERROR_UTF8_ERR20(-22)` -> -22,
/// `0 (never rejects)` -> 0, `... clamped to 65535` -> 65535).
fn last_int(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let mut i = 0usize;
    let mut found = None;
    while i < b.len() {
        if b[i].is_ascii_digit() {
            let start = if i > 0 && b[i - 1] == b'-' { i - 1 } else { i };
            let mut j = i;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            found = s[start..j].parse::<i64>().ok().or(found);
            i = j;
        } else {
            i += 1;
        }
    }
    found
}

fn table() -> &'static Vec<Row> {
    static T: OnceLock<Vec<Row>> = OnceLock::new();
    T.get_or_init(|| {
        let mut v = Vec::new();
        for (i, line) in TSV.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                f.len(),
                6,
                "err_runtime.tsv line {} has {} fields, expected 6",
                i + 1,
                f.len()
            );
            let expect = f[1];
            v.push(Row {
                n: i + 1,
                func: f[0],
                expect,
                loc: f[2],
                trigger: f[3],
                recipe: f[4],
                notes: f[5],
                unreachable: f[4].contains("UNREACHABLE"),
                val: last_int(expect),
                wants_null: expect.trim() == "NULL",
            });
        }
        v
    })
}

fn row(n: usize) -> &'static Row {
    let t = table();
    assert!(n >= 1 && n <= t.len(), "row {} out of range", n);
    &t[n - 1]
}

// ================================================================= result values

/// Everything observable about one recipe execution.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Res {
    /// integer return value only
    I(i64),
    /// integer return value + extra scalar outputs (ovector, `*outlengthptr`, ...)
    IX(i64, Vec<i64>),
    /// integer return value + extra scalars + an output buffer
    IB(i64, Vec<i64>, Vec<u8>),
    /// pointer-returning call; `true` == the call returned NULL
    P(bool),
}

impl Res {
    fn rc(&self) -> i64 {
        match self {
            Res::I(v) | Res::IX(v, _) | Res::IB(v, _, _) => *v,
            Res::P(n) => {
                if *n {
                    0
                } else {
                    1
                }
            }
        }
    }
}

/// A probe result: one `Res` per row plus corroborating (label, actual, expected)
/// values that are not TSV rows but prove the row really hit the tabulated site.
struct Probe {
    res: Vec<Res>,
    extra: Vec<(&'static str, i64, i64)>,
}

impl Probe {
    fn new() -> Probe {
        Probe {
            res: Vec::new(),
            extra: Vec::new(),
        }
    }
    fn push(&mut self, r: Res) {
        self.res.push(r)
    }
}

// ======================================================================= checker

struct Chk {
    func: &'static str,
    done: Vec<usize>,
}

impl Chk {
    fn new(func: &'static str) -> Chk {
        Chk {
            func,
            done: Vec::new(),
        }
    }

    fn one(&mut self, n: usize, c: &Res, r: &Res) {
        let row = row(n);
        assert_eq!(
            row.func, self.func,
            "internal: row {} belongs to {}, not {}",
            n, row.func, self.func
        );
        assert!(!row.unreachable, "internal: row {} is UNREACHABLE", n);
        println!("row {} {} {}", n, row.func, row.expect);
        let ctx = format!(
            "row {} {} expects {:?} at {}\n    trigger: {}\n    recipe:  {}",
            n, row.func, row.expect, row.loc, row.trigger, row.recipe
        );
        match (c, r) {
            (Res::P(cn), Res::P(rn)) => {
                assert!(
                    row.wants_null,
                    "{}\n  internal: recipe produced a pointer but the table expects an integer",
                    ctx
                );
                assert!(
                    *cn,
                    "{}\n  C LIBRARY DISAGREES WITH TABLE (stale table?): expected NULL, C returned non-NULL",
                    ctx
                );
                assert_eq!(
                    cn, rn,
                    "{}\n  RUST DIVERGES FROM C: C returned {}, Rust returned {}",
                    ctx,
                    if *cn { "NULL" } else { "non-NULL" },
                    if *rn { "NULL" } else { "non-NULL" }
                );
            }
            (Res::P(_), _) | (_, Res::P(_)) => {
                panic!("{}\n  internal: mismatched result kinds {:?} / {:?}", ctx, c, r)
            }
            _ => {
                let exp = row.val.unwrap_or_else(|| {
                    panic!("{}\n  internal: no expected integer parsed from {:?}", ctx, row.expect)
                });
                assert_eq!(
                    c.rc(),
                    exp,
                    "{}\n  C LIBRARY DISAGREES WITH TABLE (stale table?): C returned {}, table says {}\n    C result: {:?}",
                    ctx,
                    c.rc(),
                    exp,
                    c
                );
                assert_eq!(
                    c, r,
                    "{}\n  RUST DIVERGES FROM C\n    C   : {:?}\n    RUST: {:?}",
                    ctx, c, r
                );
            }
        }
        self.done.push(n);
    }

    /// Compare a whole probe pair against the row list it was built for.
    fn all(&mut self, rows: &[usize], c: &Probe, r: &Probe) {
        assert_eq!(
            c.res.len(),
            rows.len(),
            "{}: C probe produced {} results for {} rows",
            self.func,
            c.res.len(),
            rows.len()
        );
        assert_eq!(
            r.res.len(),
            rows.len(),
            "{}: Rust probe produced {} results for {} rows",
            self.func,
            r.res.len(),
            rows.len()
        );
        for (i, &n) in rows.iter().enumerate() {
            self.one(n, &c.res[i], &r.res[i]);
        }
        // corroborating (non-row) values: the C value must equal the documented
        // constant and the Rust value must equal the C value.
        assert_eq!(
            c.extra.len(),
            r.extra.len(),
            "{}: corroboration count differs",
            self.func
        );
        for (i, &(label, cval, want)) in c.extra.iter().enumerate() {
            let (rlabel, rval, _) = r.extra[i];
            println!("  corroboration {} = {} (want {})", label, cval, want);
            assert_eq!(
                cval, want,
                "{}: corroborating value {} is {} in the C library, expected {}",
                self.func, label, cval, want
            );
            assert_eq!(label, rlabel, "{}: corroboration label mismatch", self.func);
            assert_eq!(
                cval, rval,
                "{}: RUST DIVERGES FROM C on corroborating value {}: C {} vs Rust {}",
                self.func, label, cval, rval
            );
        }
    }

    fn finish(&self, rows: &[usize]) {
        assert_eq!(
            self.done, rows,
            "{}: executed rows {:?} but was assigned {:?}",
            self.func, self.done, rows
        );
        println!("{}: {} rows executed", self.func, self.done.len());
    }
}

/// Run the same probe against both libraries and check it against `rows`.
fn diff(func: &'static str, rows: &[usize], probe: unsafe fn(&Api) -> Probe) {
    let (c, r) = both();
    let cp = unsafe { probe(c) };
    let rp = unsafe { probe(r) };
    let mut chk = Chk::new(func);
    chk.all(rows, &cp, &rp);
    chk.finish(rows);
}

// ============================================================ counting allocator

thread_local! {
    /// Number of allocations the test allocator will still satisfy.
    static BUDGET: Cell<i64> = Cell::new(1 << 40);
}

fn set_budget(v: i64) {
    BUDGET.with(|b| b.set(v));
}
fn restore_budget() {
    set_budget(1 << 40);
}

/// `malloc` replacement with a budget; mirrors `cmalloc()` in `_v/checkrt.c`
/// (`if (budget-- <= 0) return NULL;`). The size is stored in a 16-byte header so
/// `cfree` can reconstruct the `Layout`.
unsafe extern "C" fn cmalloc(size: usize, _data: Ptr) -> Ptr {
    let b = BUDGET.with(|c| {
        let v = c.get();
        c.set(v - 1);
        v
    });
    if b <= 0 {
        return ptr::null_mut();
    }
    let total = size + 16;
    let layout = Layout::from_size_align(total, 16).unwrap();
    let p = alloc(layout);
    if p.is_null() {
        return ptr::null_mut();
    }
    (p as *mut usize).write(total);
    p.add(16) as Ptr
}

unsafe extern "C" fn cfree(p: Ptr, _data: Ptr) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(16);
    let total = (base as *mut usize).read();
    dealloc(base, Layout::from_size_align(total, 16).unwrap());
}

/// A general context using the counting allocator, created while the budget is high.
unsafe fn gctx_counting(a: &Api) -> Ptr {
    restore_budget();
    let g = (a.pcre2_general_context_create_8)(Some(cmalloc), Some(cfree), ptr::null_mut());
    assert!(!g.is_null(), "{}: counting general context creation failed", a.tag);
    g
}

/// A *valid* general context whose malloc always fails.
unsafe fn gctx_failing(a: &Api) -> Ptr {
    let g = gctx_counting(a);
    set_budget(0);
    g
}

// ==================================================================== primitives

const MAGIC: u32 = 0x5043_5245;
const SENTINEL: usize = 0x1234_5678_9abc_def0;

unsafe fn comp(a: &Api, pat: &[u8], opts: u32, xopts: u32) -> Ptr {
    let mut ec: i32 = 0;
    let mut eo: usize = 0;
    let cc = if xopts != 0 {
        let cc = (a.pcre2_compile_context_create_8)(ptr::null_mut());
        assert!(!cc.is_null());
        (a.pcre2_set_compile_extra_options_8)(cc, xopts);
        cc
    } else {
        ptr::null_mut()
    };
    let code = (a.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut ec, &mut eo, cc);
    if !cc.is_null() {
        (a.pcre2_compile_context_free_8)(cc);
    }
    assert!(
        !code.is_null(),
        "{}: compile of {:?} (opts {:#x} xopts {:#x}) failed: error {} at offset {}",
        a.tag,
        String::from_utf8_lossy(pat),
        opts,
        xopts,
        ec,
        eo
    );
    code
}

/// Offset of `pcre2_real_code.magic_number` inside a compiled block.
unsafe fn off_magic(a: &Api, code: Ptr) -> usize {
    let p = code as *const u8;
    let mut i = 64usize;
    while i < 256 {
        if (p.add(i) as *const u32).read_unaligned() == MAGIC {
            return i;
        }
        i += 4;
    }
    panic!("{}: magic_number not found in compiled block", a.tag);
}
fn off_flags(m: usize) -> usize {
    m + 16
}
fn off_name_entry_size(m: usize) -> usize {
    m + 52
}
fn off_name_count(m: usize) -> usize {
    m + 54
}
fn off_blocksize(m: usize) -> usize {
    m - 16
}

unsafe fn peek32(base: Ptr, off: usize) -> u32 {
    ((base as *const u8).add(off) as *const u32).read_unaligned()
}
unsafe fn poke32(base: Ptr, off: usize, v: u32) {
    ((base as *mut u8).add(off) as *mut u32).write_unaligned(v)
}
unsafe fn poke16(base: Ptr, off: usize, v: u16) {
    ((base as *mut u8).add(off) as *mut u16).write_unaligned(v)
}
unsafe fn pokesz(base: Ptr, off: usize, v: usize) {
    ((base as *mut u8).add(off) as *mut usize).write_unaligned(v)
}

/// Fill the ovector with a sentinel so that "the library did not touch this slot"
/// is distinguishable from "the library wrote something" — and so that comparing
/// ovectors between the two libraries never compares uninitialised heap.
unsafe fn fill_ovec(a: &Api, md: Ptr) {
    if md.is_null() {
        return;
    }
    let n = (a.pcre2_get_ovector_count_8)(md) as usize;
    let p = (a.pcre2_get_ovector_pointer_8)(md);
    for i in 0..2 * n {
        p.add(i).write(SENTINEL);
    }
}

unsafe fn ovec(a: &Api, md: Ptr) -> Vec<i64> {
    if md.is_null() {
        return vec![];
    }
    let n = (a.pcre2_get_ovector_count_8)(md) as usize;
    let p = (a.pcre2_get_ovector_pointer_8)(md);
    (0..2 * n).map(|i| p.add(i).read() as i64).collect()
}

/// `pcre2_match` with the ovector captured before/after.
unsafe fn m_res(
    a: &Api,
    code: Ptr,
    subj: *const u8,
    len: usize,
    so: usize,
    opts: u32,
    md: Ptr,
    mc: Ptr,
) -> Res {
    fill_ovec(a, md);
    let rc = (a.pcre2_match_8)(code, subj, len, so, opts, md, mc);
    if md.is_null() {
        Res::I(rc as i64)
    } else {
        Res::IX(rc as i64, ovec(a, md))
    }
}

/// `pcre2_dfa_match` with the ovector captured before/after.
#[allow(clippy::too_many_arguments)]
unsafe fn d_res(
    a: &Api,
    code: Ptr,
    subj: *const u8,
    len: usize,
    so: usize,
    opts: u32,
    md: Ptr,
    mc: Ptr,
    ws: *mut i32,
    wsc: usize,
) -> Res {
    fill_ovec(a, md);
    let rc = (a.pcre2_dfa_match_8)(code, subj, len, so, opts, md, mc, ws, wsc);
    if md.is_null() {
        Res::I(rc as i64)
    } else {
        Res::IX(rc as i64, ovec(a, md))
    }
}

// ===================================================================== callbacks

unsafe extern "C" fn enum_cb_99(_cb: Ptr, _d: Ptr) -> i32 {
    99
}
unsafe extern "C" fn callout_neg(_cb: Ptr, _d: Ptr) -> i32 {
    -37 // PCRE2_ERROR_CALLOUT
}
unsafe extern "C" fn bad_case_callout(
    _input: Sptr,
    _inlen: usize,
    _out: *mut u8,
    _outcap: usize,
    _tocase: i32,
    _data: Ptr,
) -> usize {
    usize::MAX
}

/// 98 empty groups + `(x)*[^x]`: 99 captures, a 1704-byte heapframe.
fn big_caps_pattern() -> Vec<u8> {
    let mut p = Vec::new();
    for _ in 0..98 {
        p.extend_from_slice(b"()");
    }
    p.extend_from_slice(b"(x)*[^x]");
    p
}

// ============================================================================
// pcre2_callout_enumerate_8 — rows 1..5
// ============================================================================
const R_CALLOUT_ENUMERATE: &[usize] = &[1, 2, 3, 4, 5];

unsafe fn probe_callout_enumerate(a: &Api) -> Probe {
    let mut p = Probe::new();
    // row 1: re == NULL
    p.push(Res::I(
        (a.pcre2_callout_enumerate_8)(ptr::null_mut(), Some(enum_cb_99), ptr::null_mut()) as i64,
    ));

    let c = comp(a, b"a", 0, 0);
    let m = off_magic(a, c);
    // row 2: corrupted magic_number
    poke32(c, m, 0);
    p.push(Res::I(
        (a.pcre2_callout_enumerate_8)(c, Some(enum_cb_99), ptr::null_mut()) as i64,
    ));
    poke32(c, m, MAGIC);
    // row 3: PCRE2_MODE8 bit cleared
    let fl = peek32(c, off_flags(m));
    poke32(c, off_flags(m), fl & !1u32);
    p.push(Res::I(
        (a.pcre2_callout_enumerate_8)(c, Some(enum_cb_99), ptr::null_mut()) as i64,
    ));
    poke32(c, off_flags(m), fl);
    (a.pcre2_code_free_8)(c);

    // row 4: numeric callout, callback returns 99
    let c = comp(a, b"a(?C1)b", 0, 0);
    p.push(Res::I(
        (a.pcre2_callout_enumerate_8)(c, Some(enum_cb_99), ptr::null_mut()) as i64,
    ));
    (a.pcre2_code_free_8)(c);

    // row 5: string callout, callback returns 99
    let c = comp(a, b"a(?C{X})b", 0, 0);
    p.push(Res::I(
        (a.pcre2_callout_enumerate_8)(c, Some(enum_cb_99), ptr::null_mut()) as i64,
    ));
    (a.pcre2_code_free_8)(c);
    p
}

#[test]
fn rt_pcre2_callout_enumerate_8() {
    diff(
        "pcre2_callout_enumerate_8",
        R_CALLOUT_ENUMERATE,
        probe_callout_enumerate,
    );
}

// ============================================================================
// context create/copy allocation failures — rows 6, 7, 12, 13, 47, 48, 116, 117
// ============================================================================
const R_COMPILE_CONTEXT_COPY: &[usize] = &[6];
const R_COMPILE_CONTEXT_CREATE: &[usize] = &[7];
const R_CONVERT_CONTEXT_COPY: &[usize] = &[12];
const R_CONVERT_CONTEXT_CREATE: &[usize] = &[13];
const R_GENERAL_CONTEXT_COPY: &[usize] = &[47];
const R_GENERAL_CONTEXT_CREATE: &[usize] = &[48];
const R_MATCH_CONTEXT_COPY: &[usize] = &[116];
const R_MATCH_CONTEXT_CREATE: &[usize] = &[117];

unsafe fn probe_compile_context_copy(a: &Api) -> Probe {
    let mut p = Probe::new();
    let g = gctx_counting(a);
    let cc = (a.pcre2_compile_context_create_8)(g);
    assert!(!cc.is_null());
    set_budget(0);
    let out = (a.pcre2_compile_context_copy_8)(cc);
    restore_budget();
    p.push(Res::P(out.is_null()));
    if !out.is_null() {
        (a.pcre2_compile_context_free_8)(out);
    }
    (a.pcre2_compile_context_free_8)(cc);
    (a.pcre2_general_context_free_8)(g);
    p
}

unsafe fn probe_compile_context_create(a: &Api) -> Probe {
    let mut p = Probe::new();
    let g = gctx_failing(a);
    let cc = (a.pcre2_compile_context_create_8)(g);
    restore_budget();
    p.push(Res::P(cc.is_null()));
    if !cc.is_null() {
        (a.pcre2_compile_context_free_8)(cc);
    }
    (a.pcre2_general_context_free_8)(g);
    p
}

unsafe fn probe_convert_context_copy(a: &Api) -> Probe {
    let mut p = Probe::new();
    let g = gctx_counting(a);
    let cc = (a.pcre2_convert_context_create_8)(g);
    assert!(!cc.is_null());
    set_budget(0);
    let out = (a.pcre2_convert_context_copy_8)(cc);
    restore_budget();
    p.push(Res::P(out.is_null()));
    if !out.is_null() {
        (a.pcre2_convert_context_free_8)(out);
    }
    (a.pcre2_convert_context_free_8)(cc);
    (a.pcre2_general_context_free_8)(g);
    p
}

unsafe fn probe_convert_context_create(a: &Api) -> Probe {
    let mut p = Probe::new();
    let g = gctx_failing(a);
    let cc = (a.pcre2_convert_context_create_8)(g);
    restore_budget();
    p.push(Res::P(cc.is_null()));
    if !cc.is_null() {
        (a.pcre2_convert_context_free_8)(cc);
    }
    (a.pcre2_general_context_free_8)(g);
    p
}

unsafe fn probe_general_context_copy(a: &Api) -> Probe {
    let mut p = Probe::new();
    let g = gctx_counting(a);
    set_budget(0);
    let out = (a.pcre2_general_context_copy_8)(g);
    restore_budget();
    p.push(Res::P(out.is_null()));
    if !out.is_null() {
        (a.pcre2_general_context_free_8)(out);
    }
    (a.pcre2_general_context_free_8)(g);
    p
}

unsafe fn probe_general_context_create(a: &Api) -> Probe {
    let mut p = Probe::new();
    set_budget(0);
    let g = (a.pcre2_general_context_create_8)(Some(cmalloc), Some(cfree), ptr::null_mut());
    restore_budget();
    p.push(Res::P(g.is_null()));
    if !g.is_null() {
        (a.pcre2_general_context_free_8)(g);
    }
    p
}

unsafe fn probe_match_context_copy(a: &Api) -> Probe {
    let mut p = Probe::new();
    let g = gctx_counting(a);
    let mc = (a.pcre2_match_context_create_8)(g);
    assert!(!mc.is_null());
    set_budget(0);
    let out = (a.pcre2_match_context_copy_8)(mc);
    restore_budget();
    p.push(Res::P(out.is_null()));
    if !out.is_null() {
        (a.pcre2_match_context_free_8)(out);
    }
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_general_context_free_8)(g);
    p
}

unsafe fn probe_match_context_create(a: &Api) -> Probe {
    let mut p = Probe::new();
    let g = gctx_failing(a);
    let mc = (a.pcre2_match_context_create_8)(g);
    restore_budget();
    p.push(Res::P(mc.is_null()));
    if !mc.is_null() {
        (a.pcre2_match_context_free_8)(mc);
    }
    (a.pcre2_general_context_free_8)(g);
    p
}

#[test]
fn rt_pcre2_compile_context_copy_8() {
    diff(
        "pcre2_compile_context_copy_8",
        R_COMPILE_CONTEXT_COPY,
        probe_compile_context_copy,
    );
}
#[test]
fn rt_pcre2_compile_context_create_8() {
    diff(
        "pcre2_compile_context_create_8",
        R_COMPILE_CONTEXT_CREATE,
        probe_compile_context_create,
    );
}
#[test]
fn rt_pcre2_convert_context_copy_8() {
    diff(
        "pcre2_convert_context_copy_8",
        R_CONVERT_CONTEXT_COPY,
        probe_convert_context_copy,
    );
}
#[test]
fn rt_pcre2_convert_context_create_8() {
    diff(
        "pcre2_convert_context_create_8",
        R_CONVERT_CONTEXT_CREATE,
        probe_convert_context_create,
    );
}
#[test]
fn rt_pcre2_general_context_copy_8() {
    diff(
        "pcre2_general_context_copy_8",
        R_GENERAL_CONTEXT_COPY,
        probe_general_context_copy,
    );
}
#[test]
fn rt_pcre2_general_context_create_8() {
    diff(
        "pcre2_general_context_create_8",
        R_GENERAL_CONTEXT_CREATE,
        probe_general_context_create,
    );
}
#[test]
fn rt_pcre2_match_context_copy_8() {
    diff(
        "pcre2_match_context_copy_8",
        R_MATCH_CONTEXT_COPY,
        probe_match_context_copy,
    );
}
#[test]
fn rt_pcre2_match_context_create_8() {
    diff(
        "pcre2_match_context_create_8",
        R_MATCH_CONTEXT_CREATE,
        probe_match_context_create,
    );
}

// ============================================================================
// pcre2_config_8 — rows 8..11
// ============================================================================
const R_CONFIG: &[usize] = &[8, 9, 10, 11];

unsafe fn probe_config(a: &Api) -> Probe {
    let mut p = Probe::new();
    // row 8: unknown key, where == NULL (length-request path)
    p.push(Res::I((a.pcre2_config_8)(999, ptr::null_mut()) as i64));
    // row 9: unknown key, where != NULL
    let mut v: u32 = 0xdead_beef;
    let rc = (a.pcre2_config_8)(999, &mut v as *mut u32 as Ptr);
    p.push(Res::IX(rc as i64, vec![v as i64]));
    // row 10: PCRE2_CONFIG_JITTARGET in a no-JIT build, with a buffer
    let mut buf = [0xAAu8; 256];
    let rc = (a.pcre2_config_8)(PCRE2_CONFIG_JITTARGET, buf.as_mut_ptr() as Ptr);
    p.push(Res::IB(rc as i64, vec![], buf.to_vec()));
    // row 11: PCRE2_CONFIG_JITTARGET with where == NULL
    p.push(Res::I(
        (a.pcre2_config_8)(PCRE2_CONFIG_JITTARGET, ptr::null_mut()) as i64,
    ));
    p
}

#[test]
fn rt_pcre2_config_8() {
    diff("pcre2_config_8", R_CONFIG, probe_config);
}

// ============================================================================
// pcre2_dfa_match_8 — rows 14..44
// ============================================================================
const R_DFA_MATCH: &[usize] = &[
    14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37,
    38, 39, 40, 41, 42, 43, 44,
];

unsafe fn probe_dfa_match(a: &Api) -> Probe {
    let mut p = Probe::new();
    let mut ws = vec![0i32; 1000];
    let mut wsbig = vec![0i32; 10000];
    let a600 = vec![b'a'; 600];
    let w = ws.as_mut_ptr();
    let wb = wsbig.as_mut_ptr();

    let c = comp(a, b"a", 0, 0);
    let md = (a.pcre2_match_data_create_8)(2, ptr::null_mut());
    assert!(!md.is_null());

    // 14: match_data == NULL
    p.push(d_res(a, c, b"a".as_ptr(), 1, 0, 0, ptr::null_mut(), ptr::null_mut(), w, 1000));
    // 15: re == NULL
    p.push(d_res(a, ptr::null_mut(), b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 1000));
    // 16: subject == NULL with length != 0
    p.push(d_res(a, c, ptr::null(), 1, 0, 0, md, ptr::null_mut(), w, 1000));
    // 17: workspace == NULL
    p.push(d_res(a, c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut(), ptr::null_mut(), 1000));
    // 18: option not in PUBLIC_DFA_MATCH_OPTIONS
    p.push(d_res(
        a, c, b"a".as_ptr(), 1, 0, PCRE2_DISABLE_RECURSELOOP_CHECK, md, ptr::null_mut(), w, 1000,
    ));
    // 19: wscount < 20
    p.push(d_res(a, c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 19));
    // 20: start_offset > length
    p.push(d_res(a, c, b"a".as_ptr(), 1, 2, 0, md, ptr::null_mut(), w, 1000));
    // 21: PARTIAL_SOFT together with ENDANCHORED
    p.push(d_res(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        PCRE2_PARTIAL_SOFT | PCRE2_ENDANCHORED,
        md,
        ptr::null_mut(),
        w,
        1000,
    ));
    (a.pcre2_code_free_8)(c);

    // 22: PCRE2_MATCH_INVALID_UTF pattern
    let c = comp(a, b"a", PCRE2_UTF | PCRE2_MATCH_INVALID_UTF, 0);
    p.push(d_res(a, c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 1000));
    (a.pcre2_code_free_8)(c);

    let c = comp(a, b"a", 0, 0);
    let m = off_magic(a, c);
    // 23: bad magic
    poke32(c, m, 0);
    p.push(d_res(a, c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 1000));
    poke32(c, m, MAGIC);
    // 24: bad mode
    let fl = peek32(c, off_flags(m));
    poke32(c, off_flags(m), fl & !1u32);
    p.push(d_res(a, c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 1000));
    poke32(c, off_flags(m), fl);

    // 25: invalid DFA_RESTART workspace
    ws[0] = 2;
    ws[1] = 1;
    p.push(d_res(
        a, c, b"a".as_ptr(), 1, 0, PCRE2_DFA_RESTART, md, ptr::null_mut(), w, 1000,
    ));
    ws[0] = 0;
    ws[1] = 0;

    // 26: offset limit without PCRE2_USE_OFFSET_LIMIT
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_offset_limit_8)(mc, 1);
    p.push(d_res(a, c, b"a".as_ptr(), 1, 0, 0, md, mc, w, 1000));
    (a.pcre2_match_context_free_8)(mc);

    // 27: match limit 0
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_match_limit_8)(mc, 0);
    p.push(d_res(a, c, b"a".as_ptr(), 1, 0, 0, md, mc, w, 1000));
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_code_free_8)(c);

    // 28: depth limit 0 with recursion
    let c = comp(a, b"(a(?1)?)", 0, 0);
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_depth_limit_8)(mc, 0);
    p.push(d_res(a, c, b"aa".as_ptr(), 2, 0, 0, md, mc, w, 1000));
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_code_free_8)(c);

    let c = comp(a, b"a", PCRE2_UTF, 0);
    // 29: start_offset inside a UTF character
    p.push(d_res(a, c, b"\xc3\xa9".as_ptr(), 2, 1, 0, md, ptr::null_mut(), w, 1000));
    // 30: isolated continuation byte
    p.push(d_res(a, c, b"\x80".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 1000));
    // 31: truncated 2-byte character
    p.push(d_res(a, c, b"\xc3".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 1000));
    (a.pcre2_code_free_8)(c);

    // 32: workspace exhausted by simultaneous states
    let c = comp(a, b"a|b|c|d|e|f|g|h", PCRE2_NO_START_OPTIMIZE, 0);
    p.push(d_res(a, c, b"zzzzz".as_ptr(), 5, 0, 0, md, ptr::null_mut(), w, 20));
    (a.pcre2_code_free_8)(c);

    // 33: quantified \C in UTF mode
    let c = comp(a, b"\\C*", PCRE2_UTF, 0);
    p.push(d_res(a, c, b"abc".as_ptr(), 3, 0, 0, md, ptr::null_mut(), w, 1000));
    (a.pcre2_code_free_8)(c);

    // 34: back reference
    let c = comp(a, b"(a)\\1", 0, 0);
    p.push(d_res(a, c, b"aa".as_ptr(), 2, 0, 0, md, ptr::null_mut(), w, 1000));
    (a.pcre2_code_free_8)(c);

    // 35: OP_CREF condition
    let c = comp(a, b"(a)(?(1)b|c)", 0, 0);
    p.push(d_res(a, c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut(), w, 1000));
    (a.pcre2_code_free_8)(c);

    // 36: OP_RREF condition with a group number
    let c = comp(a, b"(a)(?(R1)b|c)", 0, 0);
    p.push(d_res(a, c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut(), w, 1000));
    (a.pcre2_code_free_8)(c);

    // 37: too many distinct match ends for a recursion
    let c = comp(a, b"(a*)(?1)", 0, 0);
    p.push(d_res(a, c, a600.as_ptr(), 600, 0, 0, md, ptr::null_mut(), wb, 10000));
    (a.pcre2_code_free_8)(c);

    // 38: recursion loop
    let c = comp(a, b"((?1)|a)", 0, 0);
    p.push(d_res(a, c, b"b".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 1000));
    (a.pcre2_code_free_8)(c);

    // 39/40: more_workspace() heap limit / malloc failure
    let c = comp(a, b"(a(?1)?)", 0, 0);
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_heap_limit_8)(mc, 1);
    p.push(d_res(a, c, a600.as_ptr(), 600, 0, 0, md, mc, wb, 10000));
    (a.pcre2_match_context_free_8)(mc);

    let g = gctx_counting(a);
    let mc2 = (a.pcre2_match_context_create_8)(g);
    assert!(!mc2.is_null());
    set_budget(0);
    p.push(d_res(a, c, a600.as_ptr(), 600, 0, 0, md, mc2, wb, 10000));
    restore_budget();
    (a.pcre2_match_context_free_8)(mc2);
    (a.pcre2_general_context_free_8)(g);
    (a.pcre2_code_free_8)(c);
    (a.pcre2_match_data_free_8)(md);

    // 41: PCRE2_COPY_MATCHED_SUBJECT allocation failure
    let g = gctx_counting(a);
    let c = comp(a, b"a", 0, 0);
    let md2 = (a.pcre2_match_data_create_8)(2, g);
    assert!(!md2.is_null());
    set_budget(0);
    p.push(d_res(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        PCRE2_COPY_MATCHED_SUBJECT,
        md2,
        ptr::null_mut(),
        w,
        1000,
    ));
    restore_budget();
    (a.pcre2_match_data_free_8)(md2);
    (a.pcre2_general_context_free_8)(g);
    (a.pcre2_code_free_8)(c);

    // 42/43: PARTIAL and NOMATCH
    let c = comp(a, b"abc", 0, 0);
    let md = (a.pcre2_match_data_create_8)(2, ptr::null_mut());
    p.push(d_res(
        a, c, b"ab".as_ptr(), 2, 0, PCRE2_PARTIAL_SOFT, md, ptr::null_mut(), w, 1000,
    ));
    p.push(d_res(a, c, b"zzz".as_ptr(), 3, 0, 0, md, ptr::null_mut(), w, 1000));
    (a.pcre2_code_free_8)(c);
    (a.pcre2_match_data_free_8)(md);

    // 44: ovector too small -> 0
    let c = comp(a, b"a|aa|aaa", PCRE2_NO_START_OPTIMIZE, 0);
    let md = (a.pcre2_match_data_create_8)(1, ptr::null_mut());
    p.push(d_res(a, c, b"aaa".as_ptr(), 3, 0, 0, md, ptr::null_mut(), w, 1000));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    p
}

#[test]
fn rt_pcre2_dfa_match_8() {
    diff("pcre2_dfa_match_8", R_DFA_MATCH, probe_dfa_match);
}

// ============================================================================
// pcre2_get_error_message_8 — rows 49..54
// ============================================================================
const R_GET_ERROR_MESSAGE: &[usize] = &[49, 50, 51, 52, 53, 54];

unsafe fn probe_get_error_message(a: &Api) -> Probe {
    let mut p = Probe::new();
    let mut b = [0xAAu8; 256];
    let call = |errcode: i32, size: usize, b: &mut [u8; 256]| -> Res {
        for x in b.iter_mut() {
            *x = 0xAA;
        }
        let rc = (a.pcre2_get_error_message_8)(errcode, b.as_mut_ptr(), size);
        Res::IB(rc as i64, vec![], b.to_vec())
    };
    // 49: bufflen == 0
    p.push(call(PCRE2_ERROR_NOMATCH, 0, &mut b));
    // 50..53: error numbers with no message
    p.push(call(0, 256, &mut b));
    p.push(call(50, 256, &mut b));
    p.push(call(1000, 256, &mut b));
    p.push(call(-1000, 256, &mut b));
    // 54: buffer too small for the message
    p.push(call(PCRE2_ERROR_NOMATCH, 4, &mut b));
    p
}

#[test]
fn rt_pcre2_get_error_message_8() {
    diff(
        "pcre2_get_error_message_8",
        R_GET_ERROR_MESSAGE,
        probe_get_error_message,
    );
}

// ============================================================================
// JIT stubs — rows 55..61
// ============================================================================
const R_JIT_COMPILE: &[usize] = &[55, 56, 57, 58, 59];
const R_JIT_MATCH: &[usize] = &[60];
const R_JIT_STACK_CREATE: &[usize] = &[61];

unsafe fn probe_jit_compile(a: &Api) -> Probe {
    let mut p = Probe::new();
    let c = comp(a, b"a", 0, 0);
    // 55: PCRE2_JIT_TEST_ALLOC combined with another option
    p.push(Res::I(
        (a.pcre2_jit_compile_8)(c, PCRE2_JIT_TEST_ALLOC | PCRE2_JIT_COMPLETE) as i64,
    ));
    // 56: PCRE2_JIT_TEST_ALLOC alone in a no-JIT build
    p.push(Res::I((a.pcre2_jit_compile_8)(c, PCRE2_JIT_TEST_ALLOC) as i64));
    // 57: code == NULL
    p.push(Res::I(
        (a.pcre2_jit_compile_8)(ptr::null_mut(), PCRE2_JIT_COMPLETE) as i64,
    ));
    // 58: undefined option bit
    p.push(Res::I((a.pcre2_jit_compile_8)(c, 0x8000_0000u32) as i64));
    // 59: valid option, no JIT support
    p.push(Res::I((a.pcre2_jit_compile_8)(c, PCRE2_JIT_COMPLETE) as i64));
    (a.pcre2_code_free_8)(c);
    p
}

unsafe fn probe_jit_match(a: &Api) -> Probe {
    let mut p = Probe::new();
    let c = comp(a, b"a", 0, 0);
    let md = (a.pcre2_match_data_create_8)(1, ptr::null_mut());
    fill_ovec(a, md);
    let rc = (a.pcre2_jit_match_8)(c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut());
    p.push(Res::IX(rc as i64, ovec(a, md)));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    p
}

unsafe fn probe_jit_stack_create(a: &Api) -> Probe {
    let mut p = Probe::new();
    let s = (a.pcre2_jit_stack_create_8)(1024, 4096, ptr::null_mut());
    p.push(Res::P(s.is_null()));
    if !s.is_null() {
        (a.pcre2_jit_stack_free_8)(s);
    }
    p
}

#[test]
fn rt_pcre2_jit_compile_8() {
    diff("pcre2_jit_compile_8", R_JIT_COMPILE, probe_jit_compile);
}
#[test]
fn rt_pcre2_jit_match_8() {
    diff("pcre2_jit_match_8", R_JIT_MATCH, probe_jit_match);
}
#[test]
fn rt_pcre2_jit_stack_create_8() {
    diff(
        "pcre2_jit_stack_create_8",
        R_JIT_STACK_CREATE,
        probe_jit_stack_create,
    );
}

// ============================================================================
// pcre2_maketables_8 — row 62
// ============================================================================
const R_MAKETABLES: &[usize] = &[62];

unsafe fn probe_maketables(a: &Api) -> Probe {
    let mut p = Probe::new();
    let g = gctx_failing(a);
    let t = (a.pcre2_maketables_8)(g);
    restore_budget();
    p.push(Res::P(t.is_null()));
    if !t.is_null() {
        (a.pcre2_maketables_free_8)(g, t);
    }
    (a.pcre2_general_context_free_8)(g);
    p
}

#[test]
fn rt_pcre2_maketables_8() {
    diff("pcre2_maketables_8", R_MAKETABLES, probe_maketables);
}

// ============================================================================
// pcre2_match_8 — rows 63..107
// ============================================================================
const R_MATCH: &[usize] = &[
    63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86,
    87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107,
];

/// The 20 UTF-8 validity errors reachable through `pcre2_match` (rows 77..96);
/// ERR20 is row 76 and is done inline.
const UTF8_SUBJECTS: &[&[u8]] = &[
    b"\xc3",                     // ERR1
    b"\xe0",                     // ERR2
    b"\xf0",                     // ERR3
    b"\xf8",                     // ERR4
    b"\xfc",                     // ERR5
    b"\xc3\x41",                 // ERR6
    b"\xe1\x80\x41",             // ERR7
    b"\xf1\x80\x80\x41",         // ERR8
    b"\xf9\x80\x80\x80\x41",     // ERR9
    b"\xfd\x80\x80\x80\x80\x41", // ERR10
    b"\xf9\x80\x80\x80\x80",     // ERR11
    b"\xfd\x80\x80\x80\x80\x80", // ERR12
    b"\xf5\x80\x80\x80",         // ERR13
    b"\xed\xa0\x80",             // ERR14
    b"\xc1\x80",                 // ERR15
    b"\xe0\x80\x80",             // ERR16
    b"\xf0\x80\x80\x80",         // ERR17
    b"\xf8\x80\x80\x80\x80",     // ERR18
    b"\xfc\x80\x80\x80\x80\x80", // ERR19
    b"\xfe",                     // ERR21
];

unsafe fn probe_match(a: &Api) -> Probe {
    let mut p = Probe::new();
    let x600 = vec![b'x'; 600];

    let c = comp(a, b"a", 0, 0);
    let md = (a.pcre2_match_data_create_8)(2, ptr::null_mut());

    // 63: match_data == NULL
    p.push(m_res(a, c, b"a".as_ptr(), 1, 0, 0, ptr::null_mut(), ptr::null_mut()));
    // 64: re == NULL
    p.push(m_res(a, ptr::null_mut(), b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut()));
    // 65: subject == NULL with length != 0
    p.push(m_res(a, c, ptr::null(), 1, 0, 0, md, ptr::null_mut()));
    // 66: option not in PUBLIC_MATCH_OPTIONS
    p.push(m_res(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        PCRE2_SUBSTITUTE_GLOBAL,
        md,
        ptr::null_mut(),
    ));
    // 67: start_offset > length
    p.push(m_res(a, c, b"abc".as_ptr(), 3, 4, 0, md, ptr::null_mut()));
    // 68: bad magic
    let m = off_magic(a, c);
    poke32(c, m, 0);
    p.push(m_res(a, c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut()));
    poke32(c, m, MAGIC);
    // 69: bad mode
    let fl = peek32(c, off_flags(m));
    poke32(c, off_flags(m), fl & !1u32);
    p.push(m_res(a, c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut()));
    poke32(c, off_flags(m), fl);
    // 70: PARTIAL_HARD with ENDANCHORED
    p.push(m_res(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        PCRE2_PARTIAL_HARD | PCRE2_ENDANCHORED,
        md,
        ptr::null_mut(),
    ));
    // 71: offset limit without PCRE2_USE_OFFSET_LIMIT
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_offset_limit_8)(mc, 1);
    p.push(m_res(a, c, b"a".as_ptr(), 1, 0, 0, md, mc));
    (a.pcre2_match_context_free_8)(mc);
    // 72: match limit 0
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_match_limit_8)(mc, 0);
    p.push(m_res(a, c, b"a".as_ptr(), 1, 0, 0, md, mc));
    (a.pcre2_match_context_free_8)(mc);
    // 73: depth limit 0
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_depth_limit_8)(mc, 0);
    p.push(m_res(a, c, b"a".as_ptr(), 1, 0, 0, md, mc));
    (a.pcre2_match_context_free_8)(mc);
    // 74: heap limit 0
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_heap_limit_8)(mc, 0);
    p.push(m_res(a, c, b"a".as_ptr(), 1, 0, 0, md, mc));
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_code_free_8)(c);
    (a.pcre2_match_data_free_8)(md);

    // 75/76 + the 20 UTF-8 validity errors
    let c = comp(a, b"a", PCRE2_UTF, 0);
    let md = (a.pcre2_match_data_create_8)(2, ptr::null_mut());
    // 75: start_offset inside a character
    p.push(m_res(a, c, b"\xc3\xa9".as_ptr(), 2, 1, 0, md, ptr::null_mut()));
    // 76: isolated continuation byte (ERR20)
    p.push(m_res(a, c, b"\x80".as_ptr(), 1, 0, 0, md, ptr::null_mut()));
    for s in UTF8_SUBJECTS {
        p.push(m_res(a, c, s.as_ptr(), s.len(), 0, 0, md, ptr::null_mut()));
    }
    (a.pcre2_code_free_8)(c);
    (a.pcre2_match_data_free_8)(md);

    // 97: recursion loop
    let c = comp(a, b"((?1)|a)", 0, 0);
    let md = (a.pcre2_match_data_create_8)(4, ptr::null_mut());
    p.push(m_res(a, c, b"b".as_ptr(), 1, 0, 0, md, ptr::null_mut()));
    (a.pcre2_code_free_8)(c);
    // 98: \K inside a recursed group
    let c = comp(a, b"(a\\K)(?=(?1))", 0, 0);
    p.push(m_res(a, c, b"aa".as_ptr(), 2, 0, 0, md, ptr::null_mut()));
    (a.pcre2_code_free_8)(c);
    (a.pcre2_match_data_free_8)(md);

    // 99: negative callout return propagated
    let c = comp(a, b"a(?C1)b", 0, 0);
    let md = (a.pcre2_match_data_create_8)(2, ptr::null_mut());
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_callout_8)(mc, Some(callout_neg), ptr::null_mut());
    p.push(m_res(a, c, b"ab".as_ptr(), 2, 0, 0, md, mc));
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_code_free_8)(c);
    (a.pcre2_match_data_free_8)(md);

    // 100/101: frame-vector growth blocked by the heap limit at both sites
    let bigpat = big_caps_pattern();
    let c = comp(a, &bigpat, 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_heap_limit_8)(mc, 20);
    p.push(m_res(a, c, x600.as_ptr(), 600, 0, 0, md, mc));
    p.extra.push((
        "heapframes_size after HEAPLIMIT at pcre2_match.c:778",
        (a.pcre2_get_match_data_heapframes_size_8)(md) as i64,
        20480,
    ));
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_match_data_free_8)(md);

    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_heap_limit_8)(mc, 21);
    p.push(m_res(a, c, x600.as_ptr(), 600, 0, 0, md, mc));
    p.extra.push((
        "heapframes_size after HEAPLIMIT at pcre2_match.c:791",
        (a.pcre2_get_match_data_heapframes_size_8)(md) as i64,
        20480,
    ));
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_match_data_free_8)(md);

    // 102: malloc failure while growing the frame vector
    let g = gctx_counting(a);
    let md2 = (a.pcre2_match_data_create_from_pattern_8)(c, g);
    assert!(!md2.is_null());
    set_budget(1); // initial heapframes ok, growth fails
    p.push(m_res(a, c, x600.as_ptr(), 600, 0, 0, md2, ptr::null_mut()));
    restore_budget();
    (a.pcre2_match_data_free_8)(md2);
    (a.pcre2_general_context_free_8)(g);
    (a.pcre2_code_free_8)(c);

    // 103/104: initial frame vector / copied subject allocation failures
    let g = gctx_counting(a);
    let c = comp(a, b"a", 0, 0);
    let md2 = (a.pcre2_match_data_create_8)(2, g);
    assert!(!md2.is_null());
    set_budget(0);
    p.push(m_res(a, c, b"a".as_ptr(), 1, 0, 0, md2, ptr::null_mut()));
    set_budget(1);
    p.push(m_res(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        PCRE2_COPY_MATCHED_SUBJECT,
        md2,
        ptr::null_mut(),
    ));
    restore_budget();
    (a.pcre2_match_data_free_8)(md2);
    (a.pcre2_general_context_free_8)(g);
    (a.pcre2_code_free_8)(c);

    // 105/106: PARTIAL and NOMATCH
    let c = comp(a, b"abc", 0, 0);
    let md = (a.pcre2_match_data_create_8)(2, ptr::null_mut());
    p.push(m_res(
        a,
        c,
        b"ab".as_ptr(),
        2,
        0,
        PCRE2_PARTIAL_SOFT,
        md,
        ptr::null_mut(),
    ));
    p.push(m_res(a, c, b"zzz".as_ptr(), 3, 0, 0, md, ptr::null_mut()));
    (a.pcre2_code_free_8)(c);
    (a.pcre2_match_data_free_8)(md);

    // 107: ovector too small -> 0
    let c = comp(a, b"(a)(b)", 0, 0);
    let md = (a.pcre2_match_data_create_8)(1, ptr::null_mut());
    p.push(m_res(a, c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut()));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    p
}

#[test]
fn rt_pcre2_match_8() {
    diff("pcre2_match_8", R_MATCH, probe_match);
}

// ============================================================================
// pcre2_match_data_create_8 / _from_pattern_8 — rows 118..122
// ============================================================================
const R_MATCH_DATA_CREATE: &[usize] = &[118, 119, 120];
const R_MATCH_DATA_CREATE_FROM_PATTERN: &[usize] = &[121, 122];

unsafe fn probe_match_data_create(a: &Api) -> Probe {
    let mut p = Probe::new();
    // 118: oveccount 0 clamped to 1
    let md = (a.pcre2_match_data_create_8)(0, ptr::null_mut());
    assert!(!md.is_null());
    p.push(Res::I((a.pcre2_get_ovector_count_8)(md) as i64));
    (a.pcre2_match_data_free_8)(md);
    // 119: oveccount 70000 clamped to 65535
    let md = (a.pcre2_match_data_create_8)(70000, ptr::null_mut());
    assert!(!md.is_null());
    p.push(Res::I((a.pcre2_get_ovector_count_8)(md) as i64));
    (a.pcre2_match_data_free_8)(md);
    // 120: allocation failure
    let g = gctx_failing(a);
    let md = (a.pcre2_match_data_create_8)(1, g);
    restore_budget();
    p.push(Res::P(md.is_null()));
    if !md.is_null() {
        (a.pcre2_match_data_free_8)(md);
    }
    (a.pcre2_general_context_free_8)(g);
    p
}

unsafe fn probe_match_data_create_from_pattern(a: &Api) -> Probe {
    let mut p = Probe::new();
    // 121: code == NULL
    let md = (a.pcre2_match_data_create_from_pattern_8)(ptr::null_mut(), ptr::null_mut());
    p.push(Res::P(md.is_null()));
    if !md.is_null() {
        (a.pcre2_match_data_free_8)(md);
    }
    // 122: allocation failure
    let c = comp(a, b"a", 0, 0);
    let g = gctx_failing(a);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, g);
    restore_budget();
    p.push(Res::P(md.is_null()));
    if !md.is_null() {
        (a.pcre2_match_data_free_8)(md);
    }
    (a.pcre2_general_context_free_8)(g);
    (a.pcre2_code_free_8)(c);
    p
}

#[test]
fn rt_pcre2_match_data_create_8() {
    diff(
        "pcre2_match_data_create_8",
        R_MATCH_DATA_CREATE,
        probe_match_data_create,
    );
}
#[test]
fn rt_pcre2_match_data_create_from_pattern_8() {
    diff(
        "pcre2_match_data_create_from_pattern_8",
        R_MATCH_DATA_CREATE_FROM_PATTERN,
        probe_match_data_create_from_pattern,
    );
}

// ============================================================================
// pcre2_next_match_8 — rows 123..125
// ============================================================================
const R_NEXT_MATCH: &[usize] = &[123, 124, 125];

unsafe fn probe_next_match(a: &Api) -> Probe {
    let mut p = Probe::new();
    let md = (a.pcre2_match_data_create_8)(2, ptr::null_mut());

    let step = |pat: &[u8], subj: &[u8], so: usize| -> Res {
        let c = comp(a, pat, 0, 0);
        fill_ovec(a, md);
        (a.pcre2_match_8)(c, subj.as_ptr(), subj.len(), so, 0, md, ptr::null_mut());
        let mut off: usize = SENTINEL;
        let mut opt: u32 = 0xdead_beef;
        let rc = (a.pcre2_next_match_8)(md, &mut off, &mut opt);
        (a.pcre2_code_free_8)(c);
        let mut ex = vec![off as i64, opt as i64];
        ex.extend(ovec(a, md));
        Res::IX(rc as i64, ex)
    };

    // 123: the previous match failed
    p.push(step(b"z", b"a", 0));
    // 124: empty match at the end of the subject
    p.push(step(b"a*", b"b", 1));
    // 125: \K-moved empty match at the end of the subject
    p.push(step(b"(?<=a)\\Kb*", b"a", 1));

    (a.pcre2_match_data_free_8)(md);
    p
}

#[test]
fn rt_pcre2_next_match_8() {
    diff("pcre2_next_match_8", R_NEXT_MATCH, probe_next_match);
}

// ============================================================================
// pcre2_pattern_convert_8 — rows 126..143
// ============================================================================
const R_PATTERN_CONVERT: &[usize] = &[
    126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143,
];

/// One `pcre2_pattern_convert` call. `buf` is NULL to let the library allocate.
/// Captured: return code, `*blengthptr`, whether `*bufferptr` came back NULL, and
/// the caller-supplied buffer when there is one.
#[allow(clippy::too_many_arguments)]
unsafe fn cvt(
    a: &Api,
    pat: *const u8,
    plen: usize,
    opts: u32,
    buf: *mut u8,
    bufcap: usize,
    blen_in: usize,
    pass_blen: bool,
    cc: Ptr,
) -> Res {
    if !buf.is_null() {
        for i in 0..bufcap {
            buf.add(i).write(0xAA);
        }
    }
    let mut bp: *mut u8 = buf;
    let mut blen: usize = blen_in;
    let rc = (a.pcre2_pattern_convert_8)(
        pat,
        plen,
        opts,
        &mut bp,
        if pass_blen {
            &mut blen
        } else {
            ptr::null_mut()
        },
        cc,
    );
    let allocated = !bp.is_null() && bp != buf;
    let bytes = if buf.is_null() {
        vec![]
    } else {
        std::slice::from_raw_parts(buf, bufcap).to_vec()
    };
    let res = Res::IB(
        rc as i64,
        vec![
            if pass_blen { blen as i64 } else { -1 },
            bp.is_null() as i64,
            allocated as i64,
        ],
        bytes,
    );
    if allocated {
        (a.pcre2_converted_pattern_free_8)(bp);
    }
    res
}

unsafe fn probe_pattern_convert(a: &Api) -> Probe {
    let mut p = Probe::new();
    let mut sbuf = [0xAAu8; 512];
    let sb = sbuf.as_mut_ptr();
    let nul: *mut u8 = ptr::null_mut();

    // 126: pattern == NULL
    p.push(cvt(a, ptr::null(), 1, PCRE2_CONVERT_GLOB, nul, 0, 0, true, ptr::null_mut()));
    // 127: blengthptr == NULL
    p.push(cvt(a, b"a".as_ptr(), 1, PCRE2_CONVERT_GLOB, nul, 0, 0, false, ptr::null_mut()));
    // 128: unknown option bit
    p.push(cvt(
        a,
        b"a".as_ptr(),
        1,
        PCRE2_CONVERT_GLOB | 0x1000,
        nul,
        0,
        0,
        true,
        ptr::null_mut(),
    ));
    // 129: two conversion types at once
    p.push(cvt(
        a,
        b"a".as_ptr(),
        1,
        PCRE2_CONVERT_GLOB | PCRE2_CONVERT_POSIX_BASIC,
        nul,
        0,
        0,
        true,
        ptr::null_mut(),
    ));
    // 130: no conversion type
    p.push(cvt(a, b"a".as_ptr(), 1, 0, nul, 0, 0, true, ptr::null_mut()));
    // 131: invalid UTF pattern
    p.push(cvt(
        a,
        b"\x80".as_ptr(),
        1,
        PCRE2_CONVERT_GLOB | PCRE2_CONVERT_UTF,
        nul,
        0,
        0,
        true,
        ptr::null_mut(),
    ));
    // 132: POSIX pattern ending with a backslash
    p.push(cvt(
        a,
        b"a\\".as_ptr(),
        2,
        PCRE2_CONVERT_POSIX_BASIC,
        nul,
        0,
        0,
        true,
        ptr::null_mut(),
    ));
    // 133: unterminated POSIX class
    p.push(cvt(
        a,
        b"[a".as_ptr(),
        2,
        PCRE2_CONVERT_POSIX_BASIC,
        nul,
        0,
        0,
        true,
        ptr::null_mut(),
    ));
    // 134: POSIX output buffer too small (PUTCHARS)
    p.push(cvt(
        a,
        b"abcdefghij".as_ptr(),
        10,
        PCRE2_CONVERT_POSIX_BASIC,
        sb,
        512,
        1,
        true,
        ptr::null_mut(),
    ));
    // 135: POSIX output buffer too small inside a class
    p.push(cvt(
        a,
        b"[abcdefghij]".as_ptr(),
        12,
        PCRE2_CONVERT_POSIX_BASIC,
        sb,
        512,
        8,
        true,
        ptr::null_mut(),
    ));
    // 136/137/138: unterminated glob class
    p.push(cvt(a, b"[".as_ptr(), 1, PCRE2_CONVERT_GLOB, nul, 0, 0, true, ptr::null_mut()));
    p.push(cvt(a, b"[!".as_ptr(), 2, PCRE2_CONVERT_GLOB, nul, 0, 0, true, ptr::null_mut()));
    p.push(cvt(a, b"[a".as_ptr(), 2, PCRE2_CONVERT_GLOB, nul, 0, 0, true, ptr::null_mut()));
    // 139: reversed range
    p.push(cvt(
        a,
        b"[z-a]".as_ptr(),
        5,
        PCRE2_CONVERT_GLOB,
        nul,
        0,
        0,
        true,
        ptr::null_mut(),
    ));
    // 140: range ending with a POSIX class
    p.push(cvt(
        a,
        b"[a-[:alpha:]]".as_ptr(),
        13,
        PCRE2_CONVERT_GLOB,
        nul,
        0,
        0,
        true,
        ptr::null_mut(),
    ));
    // 141: glob ending with a backslash
    p.push(cvt(
        a,
        b"a\\".as_ptr(),
        2,
        PCRE2_CONVERT_GLOB,
        nul,
        0,
        0,
        true,
        ptr::null_mut(),
    ));
    // 142: glob output buffer too small
    p.push(cvt(
        a,
        b"abcdefghij".as_ptr(),
        10,
        PCRE2_CONVERT_GLOB,
        sb,
        512,
        2,
        true,
        ptr::null_mut(),
    ));
    // 143: internal buffer allocation failure
    let g = gctx_counting(a);
    let cc = (a.pcre2_convert_context_create_8)(g);
    assert!(!cc.is_null());
    set_budget(0);
    p.push(cvt(a, b"a".as_ptr(), 1, PCRE2_CONVERT_GLOB, nul, 0, 0, true, cc));
    restore_budget();
    (a.pcre2_convert_context_free_8)(cc);
    (a.pcre2_general_context_free_8)(g);
    p
}

#[test]
fn rt_pcre2_pattern_convert_8() {
    diff(
        "pcre2_pattern_convert_8",
        R_PATTERN_CONVERT,
        probe_pattern_convert,
    );
}

// ============================================================================
// pcre2_pattern_info_8 — rows 148..156
// ============================================================================
const R_PATTERN_INFO: &[usize] = &[148, 149, 150, 151, 152, 153, 154, 155, 156];

unsafe fn pinfo(a: &Api, code: Ptr, what: u32, pass_where: bool) -> Res {
    let mut v: u32 = 0xdead_beef;
    let rc = (a.pcre2_pattern_info_8)(
        code,
        what,
        if pass_where {
            &mut v as *mut u32 as Ptr
        } else {
            ptr::null_mut()
        },
    );
    Res::IX(rc as i64, vec![v as i64])
}

unsafe fn probe_pattern_info(a: &Api) -> Probe {
    let mut p = Probe::new();
    let c = comp(a, b"a", 0, 0);
    // 148/149: re == NULL
    p.push(pinfo(a, ptr::null_mut(), PCRE2_INFO_CAPTURECOUNT, true));
    p.push(pinfo(a, ptr::null_mut(), 999, false));
    // 150: bad magic
    let m = off_magic(a, c);
    poke32(c, m, 0);
    p.push(pinfo(a, c, PCRE2_INFO_CAPTURECOUNT, true));
    poke32(c, m, MAGIC);
    // 151: bad mode
    let fl = peek32(c, off_flags(m));
    poke32(c, off_flags(m), fl & !1u32);
    p.push(pinfo(a, c, PCRE2_INFO_CAPTURECOUNT, true));
    poke32(c, off_flags(m), fl);
    // 152/153/154: limits that the pattern does not set
    p.push(pinfo(a, c, PCRE2_INFO_DEPTHLIMIT, true));
    p.push(pinfo(a, c, PCRE2_INFO_HEAPLIMIT, true));
    p.push(pinfo(a, c, PCRE2_INFO_MATCHLIMIT, true));
    // 155/156: unknown key
    p.push(pinfo(a, c, 999, true));
    p.push(pinfo(a, c, 999, false));
    (a.pcre2_code_free_8)(c);
    p
}

#[test]
fn rt_pcre2_pattern_info_8() {
    diff("pcre2_pattern_info_8", R_PATTERN_INFO, probe_pattern_info);
}

// ============================================================================
// pcre2_serialize_* — rows 157..181
// ============================================================================
const R_SERIALIZE_DECODE: &[usize] = &[
    157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 169,
];
const R_SERIALIZE_ENCODE: &[usize] = &[170, 171, 172, 173, 174, 175, 176, 177];
const R_SERIALIZE_GET_NUMBER_OF_CODES: &[usize] = &[178, 179, 180, 181];

/// Compile the reference pattern and serialize it; returns (code, stream, size,
/// magic offset, code offset within the stream).
unsafe fn good_stream(a: &Api, p: &mut Probe) -> (Ptr, *mut u8, usize, usize, usize) {
    let c1 = comp(a, b"(?<nm>a)b", 0, 0);
    let mut tlen: u32 = 0;
    (a.pcre2_config_8)(PCRE2_CONFIG_TABLES_LENGTH, &mut tlen as *mut u32 as Ptr);
    let codes = [c1];
    let mut good: *mut u8 = ptr::null_mut();
    let mut gsize: usize = 0;
    let rc = (a.pcre2_serialize_encode_8)(
        codes.as_ptr() as *const Ptr,
        1,
        &mut good,
        &mut gsize,
        ptr::null_mut(),
    );
    p.extra
        .push(("pcre2_serialize_encode of the good stream", rc as i64, 1));
    assert!(!good.is_null(), "{}: serialize_encode produced no stream", a.tag);
    let m = off_magic(a, c1);
    (c1, good, gsize, m, 16 + tlen as usize)
}

unsafe fn probe_serialize_decode(a: &Api) -> Probe {
    let mut p = Probe::new();
    let (c1, good, gsize, m, codeoff) = good_stream(a, &mut p);
    let mut out: [Ptr; 2] = [ptr::null_mut(); 2];
    let mut bytes: Vec<u8> = std::slice::from_raw_parts(good, gsize).to_vec();
    let pristine = bytes.clone();

    let mut dec = |a: &Api, data: *const u8, count: i32, gc: Ptr, pass_out: bool| -> Res {
        out[0] = ptr::null_mut();
        out[1] = ptr::null_mut();
        let rc = (a.pcre2_serialize_decode_8)(
            if pass_out {
                out.as_mut_ptr()
            } else {
                ptr::null_mut()
            },
            count,
            data,
            gc,
        );
        let res = Res::IX(rc as i64, vec![out[0].is_null() as i64]);
        if rc > 0 {
            for i in 0..rc as usize {
                if !out[i].is_null() {
                    (a.pcre2_code_free_8)(out[i]);
                }
            }
        }
        res
    };

    // 157: data == NULL
    p.push(dec(a, ptr::null(), 1, ptr::null_mut(), true));
    // 158: codes == NULL
    p.push(dec(a, good, 1, ptr::null_mut(), false));
    // 159: number_of_codes <= 0
    p.push(dec(a, good, 0, ptr::null_mut(), true));
    // 160: byte-order / size sanity word corrupted
    bytes.copy_from_slice(&pristine);
    poke32(bytes.as_mut_ptr() as Ptr, 12, 0);
    p.push(dec(a, bytes.as_ptr(), 1, ptr::null_mut(), true));
    // 161: magic word corrupted
    bytes.copy_from_slice(&pristine);
    poke32(bytes.as_mut_ptr() as Ptr, 0, 0);
    p.push(dec(a, bytes.as_ptr(), 1, ptr::null_mut(), true));
    // 162/163: version / code-unit-width words corrupted
    bytes.copy_from_slice(&pristine);
    poke32(bytes.as_mut_ptr() as Ptr, 4, 0);
    p.push(dec(a, bytes.as_ptr(), 1, ptr::null_mut(), true));
    bytes.copy_from_slice(&pristine);
    poke32(bytes.as_mut_ptr() as Ptr, 8, 0);
    p.push(dec(a, bytes.as_ptr(), 1, ptr::null_mut(), true));
    // 164: tables allocation failure
    bytes.copy_from_slice(&pristine);
    let g = gctx_failing(a);
    p.push(dec(a, bytes.as_ptr(), 1, g, true));
    restore_budget();
    (a.pcre2_general_context_free_8)(g);
    // 165: blocksize inside the stream is nonsense
    pokesz(bytes.as_mut_ptr() as Ptr, codeoff + off_blocksize(m), 1);
    p.push(dec(a, bytes.as_ptr(), 1, ptr::null_mut(), true));
    // 166: code block allocation failure
    bytes.copy_from_slice(&pristine);
    let g = gctx_counting(a);
    set_budget(1); // tables malloc ok, code block malloc fails
    p.push(dec(a, bytes.as_ptr(), 1, g, true));
    restore_budget();
    (a.pcre2_general_context_free_8)(g);
    // 167: magic_number of the serialized code corrupted
    poke32(bytes.as_mut_ptr() as Ptr, codeoff + m, 0);
    p.push(dec(a, bytes.as_ptr(), 1, ptr::null_mut(), true));
    // 168: name_entry_size out of range
    bytes.copy_from_slice(&pristine);
    poke16(bytes.as_mut_ptr() as Ptr, codeoff + off_name_entry_size(m), 200);
    p.push(dec(a, bytes.as_ptr(), 1, ptr::null_mut(), true));
    // 169: name_count out of range
    bytes.copy_from_slice(&pristine);
    poke16(bytes.as_mut_ptr() as Ptr, codeoff + off_name_count(m), 20000);
    p.push(dec(a, bytes.as_ptr(), 1, ptr::null_mut(), true));

    (a.pcre2_serialize_free_8)(good);
    (a.pcre2_code_free_8)(c1);
    p
}

unsafe fn probe_serialize_encode(a: &Api) -> Probe {
    let mut p = Probe::new();
    let (c1, good, _gsize, m, _codeoff) = good_stream(a, &mut p);
    let mut codes: [Ptr; 2] = [c1, ptr::null_mut()];

    let enc = |a: &Api, cp: *const Ptr, n: i32, pass_buf: bool, pass_len: bool, gc: Ptr| -> Res {
        let mut bytes: *mut u8 = ptr::null_mut();
        let mut bsize: usize = SENTINEL;
        let rc = (a.pcre2_serialize_encode_8)(
            cp,
            n,
            if pass_buf {
                &mut bytes
            } else {
                ptr::null_mut()
            },
            if pass_len {
                &mut bsize
            } else {
                ptr::null_mut()
            },
            gc,
        );
        let res = Res::IX(
            rc as i64,
            vec![bytes.is_null() as i64, if pass_len { bsize as i64 } else { -1 }],
        );
        if !bytes.is_null() {
            (a.pcre2_serialize_free_8)(bytes);
        }
        res
    };

    // 170: codes == NULL
    p.push(enc(a, ptr::null(), 1, true, true, ptr::null_mut()));
    // 171: bufferptr == NULL
    p.push(enc(a, codes.as_ptr(), 1, false, true, ptr::null_mut()));
    // 172: bufflenptr == NULL
    p.push(enc(a, codes.as_ptr(), 1, true, false, ptr::null_mut()));
    // 173: number_of_codes <= 0
    p.push(enc(a, codes.as_ptr(), 0, true, true, ptr::null_mut()));
    // 174: codes[0] == NULL
    codes[0] = ptr::null_mut();
    p.push(enc(a, codes.as_ptr(), 1, true, true, ptr::null_mut()));
    codes[0] = c1;
    // 175: bad magic
    poke32(c1, m, 0);
    p.push(enc(a, codes.as_ptr(), 1, true, true, ptr::null_mut()));
    poke32(c1, m, MAGIC);
    // 176: two codes with different character tables
    let tables = (a.pcre2_maketables_8)(ptr::null_mut());
    assert!(!tables.is_null());
    let cc = (a.pcre2_compile_context_create_8)(ptr::null_mut());
    (a.pcre2_set_character_tables_8)(cc, tables);
    let mut ec: i32 = 0;
    let mut eo: usize = 0;
    let c2 = (a.pcre2_compile_8)(b"b".as_ptr(), 1, 0, &mut ec, &mut eo, cc);
    assert!(!c2.is_null());
    codes[1] = c2;
    p.push(enc(a, codes.as_ptr(), 2, true, true, ptr::null_mut()));
    (a.pcre2_code_free_8)(c2);
    (a.pcre2_compile_context_free_8)(cc);
    (a.pcre2_maketables_free_8)(ptr::null_mut(), tables);
    codes[1] = ptr::null_mut();
    // 177: output buffer allocation failure
    let g = gctx_failing(a);
    p.push(enc(a, codes.as_ptr(), 1, true, true, g));
    restore_budget();
    (a.pcre2_general_context_free_8)(g);

    (a.pcre2_serialize_free_8)(good);
    (a.pcre2_code_free_8)(c1);
    p
}

unsafe fn probe_serialize_get_number_of_codes(a: &Api) -> Probe {
    let mut p = Probe::new();
    let (c1, good, gsize, _m, _codeoff) = good_stream(a, &mut p);
    let mut bytes: Vec<u8> = std::slice::from_raw_parts(good, gsize).to_vec();
    let pristine = bytes.clone();

    // 178: data == NULL
    p.push(Res::I(
        (a.pcre2_serialize_get_number_of_codes_8)(ptr::null()) as i64,
    ));
    // 179: magic word corrupted
    bytes.copy_from_slice(&pristine);
    poke32(bytes.as_mut_ptr() as Ptr, 0, 0);
    p.push(Res::I(
        (a.pcre2_serialize_get_number_of_codes_8)(bytes.as_ptr()) as i64,
    ));
    // 180/181: version / code-unit-width words corrupted
    bytes.copy_from_slice(&pristine);
    poke32(bytes.as_mut_ptr() as Ptr, 4, 0);
    p.push(Res::I(
        (a.pcre2_serialize_get_number_of_codes_8)(bytes.as_ptr()) as i64,
    ));
    bytes.copy_from_slice(&pristine);
    poke32(bytes.as_mut_ptr() as Ptr, 8, 0);
    p.push(Res::I(
        (a.pcre2_serialize_get_number_of_codes_8)(bytes.as_ptr()) as i64,
    ));

    (a.pcre2_serialize_free_8)(good);
    (a.pcre2_code_free_8)(c1);
    p
}

#[test]
fn rt_pcre2_serialize_decode_8() {
    diff(
        "pcre2_serialize_decode_8",
        R_SERIALIZE_DECODE,
        probe_serialize_decode,
    );
}
#[test]
fn rt_pcre2_serialize_encode_8() {
    diff(
        "pcre2_serialize_encode_8",
        R_SERIALIZE_ENCODE,
        probe_serialize_encode,
    );
}
#[test]
fn rt_pcre2_serialize_get_number_of_codes_8() {
    diff(
        "pcre2_serialize_get_number_of_codes_8",
        R_SERIALIZE_GET_NUMBER_OF_CODES,
        probe_serialize_get_number_of_codes,
    );
}

// ============================================================================
// context setters — rows 182..208
// ============================================================================

/// Generates one test per setter: create the context, run the calls in row order,
/// free the context.  `$call` must be a non-capturing closure returning one `i64`
/// per row, in row order.
macro_rules! setter_test {
    ($test:ident, $name:literal, $rows:expr, $create:ident, $free:ident, $call:expr) => {
        #[test]
        fn $test() {
            const ROWS: &[usize] = $rows;
            unsafe fn probe(a: &Api) -> Probe {
                let mut p = Probe::new();
                let cx = (a.$create)(ptr::null_mut());
                assert!(!cx.is_null());
                let f: fn(&Api, Ptr) -> Vec<i64> = $call;
                for v in f(a, cx) {
                    p.push(Res::I(v));
                }
                (a.$free)(cx);
                p
            }
            diff($name, ROWS, probe);
        }
    };
}

const R_SET_BSR: &[usize] = &[182, 183];
const R_SET_GLOB_ESCAPE: &[usize] = &[184, 185];
const R_SET_GLOB_SEPARATOR: &[usize] = &[186];
const R_SET_NEWLINE: &[usize] = &[187, 188];
const R_SET_OPTIMIZE: &[usize] = &[189, 190, 191, 192];
const R_SET_CHARACTER_TABLES: &[usize] = &[193];
const R_SET_MAX_PATTERN_LENGTH: &[usize] = &[194];
const R_SET_MAX_PATTERN_COMPILED_LENGTH: &[usize] = &[195];
const R_SET_MAX_VARLOOKBEHIND: &[usize] = &[196];
const R_SET_PARENS_NEST_LIMIT: &[usize] = &[197];
const R_SET_COMPILE_EXTRA_OPTIONS: &[usize] = &[198];
const R_SET_COMPILE_RECURSION_GUARD: &[usize] = &[199];
const R_SET_CALLOUT: &[usize] = &[200];
const R_SET_SUBSTITUTE_CALLOUT: &[usize] = &[201];
const R_SET_SUBSTITUTE_CASE_CALLOUT: &[usize] = &[202];
const R_SET_HEAP_LIMIT: &[usize] = &[203];
const R_SET_MATCH_LIMIT: &[usize] = &[204];
const R_SET_DEPTH_LIMIT: &[usize] = &[205];
const R_SET_OFFSET_LIMIT: &[usize] = &[206];
const R_SET_RECURSION_LIMIT: &[usize] = &[207];
const R_SET_RECURSION_MEMORY_MANAGEMENT: &[usize] = &[208];

setter_test!(
    rt_pcre2_set_bsr_8,
    "pcre2_set_bsr_8",
    R_SET_BSR,
    pcre2_compile_context_create_8,
    pcre2_compile_context_free_8,
    |a, cx| unsafe {
        vec![
            (a.pcre2_set_bsr_8)(cx, 0) as i64, // below PCRE2_BSR_UNICODE
            (a.pcre2_set_bsr_8)(cx, 3) as i64, // above PCRE2_BSR_ANYCRLF
        ]
    }
);
setter_test!(
    rt_pcre2_set_glob_escape_8,
    "pcre2_set_glob_escape_8",
    R_SET_GLOB_ESCAPE,
    pcre2_convert_context_create_8,
    pcre2_convert_context_free_8,
    |a, cx| unsafe {
        vec![
            (a.pcre2_set_glob_escape_8)(cx, 256) as i64, // not an ASCII punctuation char
            (a.pcre2_set_glob_escape_8)(cx, b'a' as u32) as i64, // alphanumeric
        ]
    }
);
setter_test!(
    rt_pcre2_set_glob_separator_8,
    "pcre2_set_glob_separator_8",
    R_SET_GLOB_SEPARATOR,
    pcre2_convert_context_create_8,
    pcre2_convert_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_glob_separator_8)(cx, b':' as u32) as i64] }
);
setter_test!(
    rt_pcre2_set_newline_8,
    "pcre2_set_newline_8",
    R_SET_NEWLINE,
    pcre2_compile_context_create_8,
    pcre2_compile_context_free_8,
    |a, cx| unsafe {
        vec![
            (a.pcre2_set_newline_8)(cx, 0) as i64, // below PCRE2_NEWLINE_CR
            (a.pcre2_set_newline_8)(cx, 7) as i64, // above PCRE2_NEWLINE_NUL
        ]
    }
);
setter_test!(
    rt_pcre2_set_character_tables_8,
    "pcre2_set_character_tables_8",
    R_SET_CHARACTER_TABLES,
    pcre2_compile_context_create_8,
    pcre2_compile_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_character_tables_8)(cx, ptr::null()) as i64] }
);
setter_test!(
    rt_pcre2_set_max_pattern_length_8,
    "pcre2_set_max_pattern_length_8",
    R_SET_MAX_PATTERN_LENGTH,
    pcre2_compile_context_create_8,
    pcre2_compile_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_max_pattern_length_8)(cx, 0) as i64] }
);
setter_test!(
    rt_pcre2_set_max_pattern_compiled_length_8,
    "pcre2_set_max_pattern_compiled_length_8",
    R_SET_MAX_PATTERN_COMPILED_LENGTH,
    pcre2_compile_context_create_8,
    pcre2_compile_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_max_pattern_compiled_length_8)(cx, 0) as i64] }
);
setter_test!(
    rt_pcre2_set_max_varlookbehind_8,
    "pcre2_set_max_varlookbehind_8",
    R_SET_MAX_VARLOOKBEHIND,
    pcre2_compile_context_create_8,
    pcre2_compile_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_max_varlookbehind_8)(cx, 0) as i64] }
);
setter_test!(
    rt_pcre2_set_parens_nest_limit_8,
    "pcre2_set_parens_nest_limit_8",
    R_SET_PARENS_NEST_LIMIT,
    pcre2_compile_context_create_8,
    pcre2_compile_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_parens_nest_limit_8)(cx, 0) as i64] }
);
setter_test!(
    rt_pcre2_set_compile_extra_options_8,
    "pcre2_set_compile_extra_options_8",
    R_SET_COMPILE_EXTRA_OPTIONS,
    pcre2_compile_context_create_8,
    pcre2_compile_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_compile_extra_options_8)(cx, 0xffff_ffff) as i64] }
);
setter_test!(
    rt_pcre2_set_compile_recursion_guard_8,
    "pcre2_set_compile_recursion_guard_8",
    R_SET_COMPILE_RECURSION_GUARD,
    pcre2_compile_context_create_8,
    pcre2_compile_context_free_8,
    |a, cx| unsafe {
        vec![(a.pcre2_set_compile_recursion_guard_8)(cx, None, ptr::null_mut()) as i64]
    }
);
setter_test!(
    rt_pcre2_set_callout_8,
    "pcre2_set_callout_8",
    R_SET_CALLOUT,
    pcre2_match_context_create_8,
    pcre2_match_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_callout_8)(cx, None, ptr::null_mut()) as i64] }
);
setter_test!(
    rt_pcre2_set_substitute_callout_8,
    "pcre2_set_substitute_callout_8",
    R_SET_SUBSTITUTE_CALLOUT,
    pcre2_match_context_create_8,
    pcre2_match_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_substitute_callout_8)(cx, None, ptr::null_mut()) as i64] }
);
setter_test!(
    rt_pcre2_set_substitute_case_callout_8,
    "pcre2_set_substitute_case_callout_8",
    R_SET_SUBSTITUTE_CASE_CALLOUT,
    pcre2_match_context_create_8,
    pcre2_match_context_free_8,
    |a, cx| unsafe {
        vec![(a.pcre2_set_substitute_case_callout_8)(cx, None, ptr::null_mut()) as i64]
    }
);
setter_test!(
    rt_pcre2_set_heap_limit_8,
    "pcre2_set_heap_limit_8",
    R_SET_HEAP_LIMIT,
    pcre2_match_context_create_8,
    pcre2_match_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_heap_limit_8)(cx, 0) as i64] }
);
setter_test!(
    rt_pcre2_set_match_limit_8,
    "pcre2_set_match_limit_8",
    R_SET_MATCH_LIMIT,
    pcre2_match_context_create_8,
    pcre2_match_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_match_limit_8)(cx, 0) as i64] }
);
setter_test!(
    rt_pcre2_set_depth_limit_8,
    "pcre2_set_depth_limit_8",
    R_SET_DEPTH_LIMIT,
    pcre2_match_context_create_8,
    pcre2_match_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_depth_limit_8)(cx, 0) as i64] }
);
setter_test!(
    rt_pcre2_set_offset_limit_8,
    "pcre2_set_offset_limit_8",
    R_SET_OFFSET_LIMIT,
    pcre2_match_context_create_8,
    pcre2_match_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_offset_limit_8)(cx, 0) as i64] }
);
setter_test!(
    rt_pcre2_set_recursion_limit_8,
    "pcre2_set_recursion_limit_8",
    R_SET_RECURSION_LIMIT,
    pcre2_match_context_create_8,
    pcre2_match_context_free_8,
    |a, cx| unsafe { vec![(a.pcre2_set_recursion_limit_8)(cx, 0) as i64] }
);
setter_test!(
    rt_pcre2_set_recursion_memory_management_8,
    "pcre2_set_recursion_memory_management_8",
    R_SET_RECURSION_MEMORY_MANAGEMENT,
    pcre2_match_context_create_8,
    pcre2_match_context_free_8,
    |a, cx| unsafe {
        vec![(a.pcre2_set_recursion_memory_management_8)(cx, None, None, ptr::null_mut()) as i64]
    }
);

unsafe fn probe_set_optimize(a: &Api) -> Probe {
    let mut p = Probe::new();
    // 189: ccontext == NULL
    p.push(Res::I(
        (a.pcre2_set_optimize_8)(ptr::null_mut(), 1 /* PCRE2_OPTIMIZATION_FULL */) as i64,
    ));
    let cx = (a.pcre2_compile_context_create_8)(ptr::null_mut());
    // 190/191/192: unknown directives (raw out-of-range enum values)
    p.push(Res::I((a.pcre2_set_optimize_8)(cx, 2) as i64));
    p.push(Res::I((a.pcre2_set_optimize_8)(cx, 70) as i64));
    p.push(Res::I((a.pcre2_set_optimize_8)(cx, 63) as i64));
    (a.pcre2_compile_context_free_8)(cx);
    p
}

#[test]
fn rt_pcre2_set_optimize_8() {
    diff("pcre2_set_optimize_8", R_SET_OPTIMIZE, probe_set_optimize);
}

// ============================================================================
// pcre2_substitute_8 — rows 209..252
// ============================================================================
const R_SUBSTITUTE: &[usize] = &[
    209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 224, 225, 226, 227,
    228, 229, 230, 231, 232, 233, 234, 235, 236, 237, 238, 239, 240, 241, 242, 243, 244, 245, 246,
    247, 248, 249, 250, 251, 252,
];

const SUBBUF: usize = 1024;

/// One `pcre2_substitute` call. The output buffer is refilled with 0xAA first so
/// that comparing it between the two libraries never compares stale bytes.
#[allow(clippy::too_many_arguments)]
unsafe fn sub_call(
    a: &Api,
    code: Ptr,
    subj: *const u8,
    slen: usize,
    so: usize,
    opts: u32,
    md: Ptr,
    mc: Ptr,
    rep: *const u8,
    rlen: usize,
    buf: &mut [u8; SUBBUF],
    blen_in: usize,
) -> Res {
    for x in buf.iter_mut() {
        *x = 0xAA;
    }
    let mut blen = blen_in;
    let rc = (a.pcre2_substitute_8)(
        code,
        subj,
        slen,
        so,
        opts,
        md,
        mc,
        rep,
        rlen,
        buf.as_mut_ptr(),
        &mut blen,
    );
    Res::IB(rc as i64, vec![blen as i64], buf.to_vec())
}

/// The `SUB()` macro of `_v/checkrt.c`: compile, substitute with a full-size output
/// buffer, free.
#[allow(clippy::too_many_arguments)]
unsafe fn sub_pat(
    a: &Api,
    pat: &[u8],
    xopts: u32,
    subj: &[u8],
    so: usize,
    opts: u32,
    rep: &[u8],
    buf: &mut [u8; SUBBUF],
) -> Res {
    let c = comp(a, pat, 0, xopts);
    let r = sub_call(
        a,
        c,
        subj.as_ptr(),
        subj.len(),
        so,
        opts,
        ptr::null_mut(),
        ptr::null_mut(),
        rep.as_ptr(),
        rep.len(),
        buf,
        SUBBUF,
    );
    (a.pcre2_code_free_8)(c);
    r
}

unsafe fn probe_substitute(a: &Api) -> Probe {
    let mut p = Probe::new();
    let mut buf = [0xAAu8; SUBBUF];
    let mut ws = vec![0i32; 1000];
    let w = ws.as_mut_ptr();
    let nul: *const u8 = ptr::null();

    // 209: PARTIAL without SUBSTITUTE_REPLACEMENT_ONLY
    p.push(sub_pat(a, b"abc", 0, b"abc", 0, PCRE2_PARTIAL_SOFT, b"X", &mut buf));

    let c = comp(a, b"a", 0, 0);
    // 210: replacement == NULL with rlength != 0
    p.push(sub_call(
        a, c, b"a".as_ptr(), 1, 0, 0, ptr::null_mut(), ptr::null_mut(), nul, 1, &mut buf, SUBBUF,
    ));
    // 211: subject == NULL with length != 0
    p.push(sub_call(
        a,
        c,
        nul,
        1,
        0,
        0,
        ptr::null_mut(),
        ptr::null_mut(),
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    // 212: SUBSTITUTE_MATCHED without match data
    p.push(sub_call(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        PCRE2_SUBSTITUTE_MATCHED,
        ptr::null_mut(),
        ptr::null_mut(),
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));

    // 213: the passed-in match data holds an error
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(c, b"abc".as_ptr(), 3, 9, 0, md, ptr::null_mut()); // -> BADOFFSET
    p.push(sub_call(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        PCRE2_SUBSTITUTE_MATCHED,
        md,
        ptr::null_mut(),
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    // 214: DFA-produced match data
    (a.pcre2_dfa_match_8)(c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 1000);
    p.push(sub_call(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        PCRE2_SUBSTITUTE_MATCHED,
        md,
        ptr::null_mut(),
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    // 215..218: the match data must agree with pattern/subject/offset/options
    (a.pcre2_match_8)(c, b"aa".as_ptr(), 2, 0, 0, md, ptr::null_mut());
    let c2 = comp(a, b"a", 0, 0);
    p.push(sub_call(
        a,
        c2,
        b"aa".as_ptr(),
        2,
        0,
        PCRE2_SUBSTITUTE_MATCHED,
        md,
        ptr::null_mut(),
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    (a.pcre2_code_free_8)(c2);
    p.push(sub_call(
        a,
        c,
        b"bb".as_ptr(),
        2,
        0,
        PCRE2_SUBSTITUTE_MATCHED,
        md,
        ptr::null_mut(),
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    p.push(sub_call(
        a,
        c,
        b"aa".as_ptr(),
        2,
        1,
        PCRE2_SUBSTITUTE_MATCHED,
        md,
        ptr::null_mut(),
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    p.push(sub_call(
        a,
        c,
        b"aa".as_ptr(),
        2,
        0,
        PCRE2_SUBSTITUTE_MATCHED | PCRE2_NOTBOL,
        md,
        ptr::null_mut(),
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    (a.pcre2_match_data_free_8)(md);

    // 219/220: internal match-data allocation failures
    let g = gctx_counting(a);
    let mc = (a.pcre2_match_context_create_8)(g);
    assert!(!mc.is_null());
    set_budget(0);
    p.push(sub_call(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        0,
        ptr::null_mut(),
        mc,
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    restore_budget();
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut());
    set_budget(0);
    p.push(sub_call(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        PCRE2_SUBSTITUTE_MATCHED,
        md,
        mc,
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    restore_budget();
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_general_context_free_8)(g);
    (a.pcre2_code_free_8)(c);

    // 221: invalid UTF replacement
    let c = comp(a, b"a", PCRE2_UTF, 0);
    p.push(sub_call(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        0,
        ptr::null_mut(),
        ptr::null_mut(),
        b"\x80".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    (a.pcre2_code_free_8)(c);

    // 222: start_offset > length
    p.push(sub_pat(a, b"a", 0, b"abc", 9, 0, b"X", &mut buf));

    // 223: an error from the internal pcre2_match()
    let c = comp(a, b"a", 0, 0);
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_match_limit_8)(mc, 0);
    p.push(sub_call(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        0,
        ptr::null_mut(),
        mc,
        b"X".as_ptr(),
        1,
        &mut buf,
        SUBBUF,
    ));
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_code_free_8)(c);

    // 224/225: \K moved the match start before/after the ovector pair
    p.push(sub_pat(
        a,
        b"(?<=\\Kab)c",
        PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
        b"abc",
        2,
        0,
        b"X",
        &mut buf,
    ));
    p.push(sub_pat(
        a,
        b"a(?=b\\K)",
        PCRE2_EXTRA_ALLOW_LOOKAROUND_BSK,
        b"ab",
        0,
        0,
        b"X",
        &mut buf,
    ));
    // 226/227: $' and $_ are not available for a partial match
    p.push(sub_pat(
        a,
        b"abc",
        0,
        b"abcd",
        0,
        PCRE2_PARTIAL_SOFT | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        b"$'",
        &mut buf,
    ));
    p.push(sub_pat(
        a,
        b"abc",
        0,
        b"abcd",
        0,
        PCRE2_PARTIAL_SOFT | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
        b"$_",
        &mut buf,
    ));
    // 228: $+ with no capture groups
    p.push(sub_pat(a, b"abc", 0, b"abc", 0, 0, b"$+", &mut buf));

    // 229: $+ with an ovector too small for all groups
    let c = comp(a, b"(a)(b)", 0, 0);
    let md = (a.pcre2_match_data_create_8)(1, ptr::null_mut());
    p.push(sub_call(
        a,
        c,
        b"ab".as_ptr(),
        2,
        0,
        0,
        md,
        ptr::null_mut(),
        b"$+".as_ptr(),
        2,
        &mut buf,
        SUBBUF,
    ));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);

    // 230: $+ referring to an unset group
    p.push(sub_pat(a, b"(a)?b", 0, b"b", 0, 0, b"$+", &mut buf));
    // 231: $9 with only one group
    p.push(sub_pat(a, b"(a)", 0, b"a", 0, 0, b"$9", &mut buf));
    // 232: ${1:x} — unknown substitution operator
    p.push(sub_pat(
        a,
        b"(a)",
        0,
        b"a",
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        b"${1:x}",
        &mut buf,
    ));
    // 233/234: bad escapes inside ${..:+..}
    p.push(sub_pat(
        a,
        b"(a)",
        0,
        b"a",
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        b"${1:+\\j}",
        &mut buf,
    ));
    p.push(sub_pat(
        a,
        b"(a)",
        0,
        b"a",
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        b"${1:+\\d}",
        &mut buf,
    ));
    // 235/236: missing closing brace
    p.push(sub_pat(
        a,
        b"(a)",
        0,
        b"a",
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        b"${1:+abc",
        &mut buf,
    ));
    p.push(sub_pat(a, b"(a)", 0, b"a", 0, 0, b"${1", &mut buf));
    // 237..243: malformed replacement items
    p.push(sub_pat(a, b"(a)", 0, b"a", 0, 0, b"$", &mut buf));
    p.push(sub_pat(a, b"(a)", 0, b"a", 0, 0, b"${", &mut buf));
    p.push(sub_pat(a, b"(a)", 0, b"a", 0, 0, b"$<", &mut buf));
    p.push(sub_pat(a, b"(a)", 0, b"a", 0, 0, b"$*", &mut buf));
    p.push(sub_pat(a, b"(a)", 0, b"a", 0, 0, b"$!", &mut buf));
    p.push(sub_pat(a, b"(?<n>a)", 0, b"a", 0, 0, b"$<n", &mut buf));
    p.push(sub_pat(a, b"(a)", 0, b"a", 0, 0, b"${*foo}", &mut buf));
    // 244: too deeply nested ${..:+..}
    let mut rep: Vec<u8> = Vec::new();
    for _ in 0..11 {
        rep.extend_from_slice(b"${1:+");
    }
    rep.push(b'z');
    for _ in 0..11 {
        rep.push(b'}');
    }
    p.push(sub_pat(
        a,
        b"(a)",
        0,
        b"a",
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        &rep,
        &mut buf,
    ));
    // 245: $1 referring to an unset group
    p.push(sub_pat(a, b"(a)?b", 0, b"b", 0, 0, b"$1", &mut buf));
    // 246..249: bad escapes in the replacement
    p.push(sub_pat(
        a,
        b"(a)",
        0,
        b"a",
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        b"\\j",
        &mut buf,
    ));
    p.push(sub_pat(
        a,
        b"(a)",
        0,
        b"a",
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        b"\\d",
        &mut buf,
    ));
    p.push(sub_pat(
        a,
        b"(?<n>a)",
        0,
        b"a",
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        b"\\g<>",
        &mut buf,
    ));
    p.push(sub_pat(
        a,
        b"(?<n>a)",
        0,
        b"a",
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        b"\\g<n",
        &mut buf,
    ));

    // 250/251: output buffer overflow, with and without OVERFLOW_LENGTH
    let c = comp(a, b"abc", 0, 0);
    p.push(sub_call(
        a,
        c,
        b"abc".as_ptr(),
        3,
        0,
        0,
        ptr::null_mut(),
        ptr::null_mut(),
        b"XXXXXXXXXX".as_ptr(),
        10,
        &mut buf,
        2,
    ));
    let r = sub_call(
        a,
        c,
        b"abc".as_ptr(),
        3,
        0,
        PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
        ptr::null_mut(),
        ptr::null_mut(),
        b"XXXXXXXXXX".as_ptr(),
        10,
        &mut buf,
        2,
    );
    let needed = match &r {
        Res::IB(_, x, _) => x[0],
        _ => unreachable!(),
    };
    p.push(r);
    p.extra
        .push(("*blength set to the required length", needed, 11));
    (a.pcre2_code_free_8)(c);

    // 252: the case-transformation callout failed
    let c = comp(a, b"(a)", 0, 0);
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_substitute_case_callout_8)(mc, Some(bad_case_callout), ptr::null_mut());
    p.push(sub_call(
        a,
        c,
        b"a".as_ptr(),
        1,
        0,
        PCRE2_SUBSTITUTE_EXTENDED,
        ptr::null_mut(),
        mc,
        b"\\U$1".as_ptr(),
        4,
        &mut buf,
        SUBBUF,
    ));
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_code_free_8)(c);
    p
}

#[test]
fn rt_pcre2_substitute_8() {
    diff("pcre2_substitute_8", R_SUBSTITUTE, probe_substitute);
}

// ============================================================================
// pcre2_substring_* — rows 257..286
// ============================================================================
const R_SUBSTRING_COPY_BYNAME: &[usize] = &[257, 258, 259, 260];
const R_SUBSTRING_COPY_BYNUMBER: &[usize] = &[261, 262];
const R_SUBSTRING_GET_BYNAME: &[usize] = &[263, 264, 265, 266];
const R_SUBSTRING_GET_BYNUMBER: &[usize] = &[267, 268];
const R_SUBSTRING_LENGTH_BYNAME: &[usize] = &[269, 270, 271, 272];
const R_SUBSTRING_LENGTH_BYNUMBER: &[usize] = &[273, 274, 275, 276, 277, 278, 279];
const R_SUBSTRING_LIST_GET: &[usize] = &[281, 282];
const R_SUBSTRING_NAMETABLE_SCAN: &[usize] = &[283, 284];
const R_SUBSTRING_NUMBER_FROM_NAME: &[usize] = &[285, 286];

const SBUF: usize = 64;

unsafe fn cp_byname(a: &Api, md: Ptr, name: &[u8], cap: usize, buf: &mut [u8; SBUF]) -> Res {
    for x in buf.iter_mut() {
        *x = 0xAA;
    }
    let mut sz = cap;
    let rc = (a.pcre2_substring_copy_byname_8)(md, name.as_ptr(), buf.as_mut_ptr(), &mut sz);
    Res::IB(rc as i64, vec![sz as i64], buf.to_vec())
}

unsafe fn cp_bynumber(a: &Api, md: Ptr, n: u32, cap: usize, buf: &mut [u8; SBUF]) -> Res {
    for x in buf.iter_mut() {
        *x = 0xAA;
    }
    let mut sz = cap;
    let rc = (a.pcre2_substring_copy_bynumber_8)(md, n, buf.as_mut_ptr(), &mut sz);
    Res::IB(rc as i64, vec![sz as i64], buf.to_vec())
}

unsafe fn get_res(a: &Api, rc: i32, sp: *mut u8, sz: usize) -> Res {
    let bytes = if sp.is_null() || rc != 0 {
        vec![]
    } else {
        std::slice::from_raw_parts(sp, sz + 1).to_vec()
    };
    let res = Res::IB(rc as i64, vec![sz as i64, sp.is_null() as i64], bytes);
    if !sp.is_null() && rc == 0 {
        (a.pcre2_substring_free_8)(sp);
    }
    res
}

unsafe fn probe_substring_copy_byname(a: &Api) -> Probe {
    let mut p = Probe::new();
    let mut buf = [0xAAu8; SBUF];
    let mut ws = vec![0i32; 1000];
    let w = ws.as_mut_ptr();

    let c = comp(a, b"(?<n>a)(?<o>b)", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    // 257: the match came from pcre2_dfa_match()
    (a.pcre2_dfa_match_8)(c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut(), w, 1000);
    p.push(cp_byname(a, md, b"n\0", SBUF, &mut buf));
    // 258: no such name
    (a.pcre2_match_8)(c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut());
    p.push(cp_byname(a, md, b"zz\0", SBUF, &mut buf));
    (a.pcre2_match_data_free_8)(md);
    // 259: ovector too small for that group
    let md = (a.pcre2_match_data_create_8)(1, ptr::null_mut());
    (a.pcre2_match_8)(c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut());
    p.push(cp_byname(a, md, b"n\0", SBUF, &mut buf));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    // 260: the group did not participate in the match
    let c = comp(a, b"(?<n>a)|c", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(c, b"c".as_ptr(), 1, 0, 0, md, ptr::null_mut());
    p.push(cp_byname(a, md, b"n\0", SBUF, &mut buf));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    p
}

unsafe fn probe_substring_copy_bynumber(a: &Api) -> Probe {
    let mut p = Probe::new();
    let mut buf = [0xAAu8; SBUF];
    let c = comp(a, b"(abc)", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(c, b"abc".as_ptr(), 3, 0, 0, md, ptr::null_mut());
    // 261: no such group
    p.push(cp_bynumber(a, md, 5, SBUF, &mut buf));
    // 262: buffer too small
    p.push(cp_bynumber(a, md, 1, 1, &mut buf));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    p
}

unsafe fn probe_substring_get_byname(a: &Api) -> Probe {
    let mut p = Probe::new();
    let mut ws = vec![0i32; 1000];
    let w = ws.as_mut_ptr();
    let call = |a: &Api, md: Ptr, name: &[u8]| -> Res {
        let mut sp: *mut u8 = ptr::null_mut();
        let mut sz: usize = SENTINEL;
        let rc = (a.pcre2_substring_get_byname_8)(md, name.as_ptr(), &mut sp, &mut sz);
        get_res(a, rc, sp, sz)
    };

    let c = comp(a, b"(?<n>a)(?<o>b)", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    // 263: DFA match data
    (a.pcre2_dfa_match_8)(c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut(), w, 1000);
    p.push(call(a, md, b"n\0"));
    // 264: no such name
    (a.pcre2_match_8)(c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut());
    p.push(call(a, md, b"zz\0"));
    (a.pcre2_match_data_free_8)(md);
    // 265: ovector too small
    let md = (a.pcre2_match_data_create_8)(1, ptr::null_mut());
    (a.pcre2_match_8)(c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut());
    p.push(call(a, md, b"n\0"));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    // 266: unset group
    let c = comp(a, b"(?<n>a)|c", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(c, b"c".as_ptr(), 1, 0, 0, md, ptr::null_mut());
    p.push(call(a, md, b"n\0"));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    p
}

unsafe fn probe_substring_get_bynumber(a: &Api) -> Probe {
    let mut p = Probe::new();
    let call = |a: &Api, md: Ptr, n: u32| -> Res {
        let mut sp: *mut u8 = ptr::null_mut();
        let mut sz: usize = SENTINEL;
        let rc = (a.pcre2_substring_get_bynumber_8)(md, n, &mut sp, &mut sz);
        get_res(a, rc, sp, sz)
    };
    let c = comp(a, b"(abc)", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(c, b"abc".as_ptr(), 3, 0, 0, md, ptr::null_mut());
    // 267: no such group
    p.push(call(a, md, 5));
    (a.pcre2_match_data_free_8)(md);
    // 268: allocation failure
    let g = gctx_counting(a);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, g);
    (a.pcre2_match_8)(c, b"abc".as_ptr(), 3, 0, 0, md, ptr::null_mut());
    set_budget(0);
    p.push(call(a, md, 1));
    restore_budget();
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_general_context_free_8)(g);
    (a.pcre2_code_free_8)(c);
    p
}

unsafe fn probe_substring_length_byname(a: &Api) -> Probe {
    let mut p = Probe::new();
    let mut ws = vec![0i32; 1000];
    let w = ws.as_mut_ptr();
    let call = |a: &Api, md: Ptr, name: &[u8]| -> Res {
        let mut sz: usize = SENTINEL;
        let rc = (a.pcre2_substring_length_byname_8)(md, name.as_ptr(), &mut sz);
        Res::IX(rc as i64, vec![sz as i64])
    };
    let c = comp(a, b"(?<n>a)(?<o>b)", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    // 269: DFA match data
    (a.pcre2_dfa_match_8)(c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut(), w, 1000);
    p.push(call(a, md, b"n\0"));
    // 270: no such name
    (a.pcre2_match_8)(c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut());
    p.push(call(a, md, b"zz\0"));
    (a.pcre2_match_data_free_8)(md);
    // 271: ovector too small
    let md = (a.pcre2_match_data_create_8)(1, ptr::null_mut());
    (a.pcre2_match_8)(c, b"ab".as_ptr(), 2, 0, 0, md, ptr::null_mut());
    p.push(call(a, md, b"n\0"));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    // 272: unset group
    let c = comp(a, b"(?<n>a)|c", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(c, b"c".as_ptr(), 1, 0, 0, md, ptr::null_mut());
    p.push(call(a, md, b"n\0"));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    p
}

unsafe fn probe_substring_length_bynumber(a: &Api) -> Probe {
    let mut p = Probe::new();
    let mut ws = vec![0i32; 1000];
    let w = ws.as_mut_ptr();
    let call = |a: &Api, md: Ptr, n: u32| -> Res {
        let mut sz: usize = SENTINEL;
        let rc = (a.pcre2_substring_length_bynumber_8)(md, n, &mut sz);
        Res::IX(rc as i64, vec![sz as i64])
    };
    // 273/274: the stored match rc is propagated
    let c = comp(a, b"abc", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(
        c,
        b"ab".as_ptr(),
        2,
        0,
        PCRE2_PARTIAL_SOFT,
        md,
        ptr::null_mut(),
    );
    p.push(call(a, md, 1));
    (a.pcre2_match_8)(c, b"zzz".as_ptr(), 3, 0, 0, md, ptr::null_mut());
    p.push(call(a, md, 0));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    // 275: no such group
    let c = comp(a, b"(a)", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut());
    p.push(call(a, md, 5));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    // 276: group beyond the ovector
    let c = comp(a, b"(a)(b)(c)", 0, 0);
    let md = (a.pcre2_match_data_create_8)(2, ptr::null_mut());
    (a.pcre2_match_8)(c, b"abc".as_ptr(), 3, 0, 0, md, ptr::null_mut());
    p.push(call(a, md, 2));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    // 277: unset group
    let c = comp(a, b"(a)|(b)", 0, 0);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut());
    p.push(call(a, md, 2));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    // 278/279: after a DFA match
    let c = comp(a, b"a", 0, 0);
    let md = (a.pcre2_match_data_create_8)(2, ptr::null_mut());
    (a.pcre2_dfa_match_8)(c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 1000);
    p.push(call(a, md, 2));
    p.push(call(a, md, 1));
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_code_free_8)(c);
    p
}

unsafe fn probe_substring_list_get(a: &Api) -> Probe {
    let mut p = Probe::new();
    let call = |a: &Api, md: Ptr| -> Res {
        let mut list: *mut *mut u8 = ptr::null_mut();
        let mut lens: *mut usize = ptr::null_mut();
        let rc = (a.pcre2_substring_list_get_8)(md, &mut list, &mut lens);
        let res = Res::IX(rc as i64, vec![list.is_null() as i64, lens.is_null() as i64]);
        if rc == 0 && !list.is_null() {
            (a.pcre2_substring_list_free_8)(list);
        }
        res
    };
    let c = comp(a, b"(a)", 0, 0);
    // 281: the stored match rc is propagated
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, ptr::null_mut());
    (a.pcre2_match_8)(c, b"z".as_ptr(), 1, 0, 0, md, ptr::null_mut());
    p.push(call(a, md));
    (a.pcre2_match_data_free_8)(md);
    // 282: allocation failure
    let g = gctx_counting(a);
    let md = (a.pcre2_match_data_create_from_pattern_8)(c, g);
    (a.pcre2_match_8)(c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut());
    set_budget(0);
    p.push(call(a, md));
    restore_budget();
    (a.pcre2_match_data_free_8)(md);
    (a.pcre2_general_context_free_8)(g);
    (a.pcre2_code_free_8)(c);
    p
}

unsafe fn probe_substring_nametable_scan(a: &Api) -> Probe {
    let mut p = Probe::new();
    let cu = comp(a, b"(?<n>a)", 0, 0);
    let cd = comp(a, b"(?<n>a)|(?<n>b)", PCRE2_DUPNAMES, 0);
    // 283: no such name
    let mut first: *mut u8 = ptr::null_mut();
    let mut last: *mut u8 = ptr::null_mut();
    let rc = (a.pcre2_substring_nametable_scan_8)(cu, b"zz\0".as_ptr(), &mut first, &mut last);
    p.push(Res::IX(
        rc as i64,
        vec![first.is_null() as i64, last.is_null() as i64],
    ));
    // 284: duplicate name with firstptr == NULL
    let rc = (a.pcre2_substring_nametable_scan_8)(
        cd,
        b"n\0".as_ptr(),
        ptr::null_mut(),
        ptr::null_mut(),
    );
    p.push(Res::I(rc as i64));
    (a.pcre2_code_free_8)(cd);
    (a.pcre2_code_free_8)(cu);
    p
}

unsafe fn probe_substring_number_from_name(a: &Api) -> Probe {
    let mut p = Probe::new();
    let cu = comp(a, b"(?<n>a)", 0, 0);
    let cd = comp(a, b"(?<n>a)|(?<n>b)", PCRE2_DUPNAMES, 0);
    // 285: no such name
    p.push(Res::I(
        (a.pcre2_substring_number_from_name_8)(cu, b"zz\0".as_ptr()) as i64,
    ));
    // 286: duplicate name
    p.push(Res::I(
        (a.pcre2_substring_number_from_name_8)(cd, b"n\0".as_ptr()) as i64,
    ));
    (a.pcre2_code_free_8)(cd);
    (a.pcre2_code_free_8)(cu);
    p
}

#[test]
fn rt_pcre2_substring_copy_byname_8() {
    diff(
        "pcre2_substring_copy_byname_8",
        R_SUBSTRING_COPY_BYNAME,
        probe_substring_copy_byname,
    );
}
#[test]
fn rt_pcre2_substring_copy_bynumber_8() {
    diff(
        "pcre2_substring_copy_bynumber_8",
        R_SUBSTRING_COPY_BYNUMBER,
        probe_substring_copy_bynumber,
    );
}
#[test]
fn rt_pcre2_substring_get_byname_8() {
    diff(
        "pcre2_substring_get_byname_8",
        R_SUBSTRING_GET_BYNAME,
        probe_substring_get_byname,
    );
}
#[test]
fn rt_pcre2_substring_get_bynumber_8() {
    diff(
        "pcre2_substring_get_bynumber_8",
        R_SUBSTRING_GET_BYNUMBER,
        probe_substring_get_bynumber,
    );
}
#[test]
fn rt_pcre2_substring_length_byname_8() {
    diff(
        "pcre2_substring_length_byname_8",
        R_SUBSTRING_LENGTH_BYNAME,
        probe_substring_length_byname,
    );
}
#[test]
fn rt_pcre2_substring_length_bynumber_8() {
    diff(
        "pcre2_substring_length_bynumber_8",
        R_SUBSTRING_LENGTH_BYNUMBER,
        probe_substring_length_bynumber,
    );
}
#[test]
fn rt_pcre2_substring_list_get_8() {
    diff(
        "pcre2_substring_list_get_8",
        R_SUBSTRING_LIST_GET,
        probe_substring_list_get,
    );
}
#[test]
fn rt_pcre2_substring_nametable_scan_8() {
    diff(
        "pcre2_substring_nametable_scan_8",
        R_SUBSTRING_NAMETABLE_SCAN,
        probe_substring_nametable_scan,
    );
}
#[test]
fn rt_pcre2_substring_number_from_name_8() {
    diff(
        "pcre2_substring_number_from_name_8",
        R_SUBSTRING_NUMBER_FROM_NAME,
        probe_substring_number_from_name,
    );
}

// ============================================================================
// row accounting: every reachable row is executed exactly once
// ============================================================================

/// (public function, the rows its test executes). Must cover the TSV exactly.
const EXECUTED: &[(&str, &[usize])] = &[
    ("pcre2_callout_enumerate_8", R_CALLOUT_ENUMERATE),
    ("pcre2_compile_context_copy_8", R_COMPILE_CONTEXT_COPY),
    ("pcre2_compile_context_create_8", R_COMPILE_CONTEXT_CREATE),
    ("pcre2_config_8", R_CONFIG),
    ("pcre2_convert_context_copy_8", R_CONVERT_CONTEXT_COPY),
    ("pcre2_convert_context_create_8", R_CONVERT_CONTEXT_CREATE),
    ("pcre2_dfa_match_8", R_DFA_MATCH),
    ("pcre2_general_context_copy_8", R_GENERAL_CONTEXT_COPY),
    ("pcre2_general_context_create_8", R_GENERAL_CONTEXT_CREATE),
    ("pcre2_get_error_message_8", R_GET_ERROR_MESSAGE),
    ("pcre2_jit_compile_8", R_JIT_COMPILE),
    ("pcre2_jit_match_8", R_JIT_MATCH),
    ("pcre2_jit_stack_create_8", R_JIT_STACK_CREATE),
    ("pcre2_maketables_8", R_MAKETABLES),
    ("pcre2_match_8", R_MATCH),
    ("pcre2_match_context_copy_8", R_MATCH_CONTEXT_COPY),
    ("pcre2_match_context_create_8", R_MATCH_CONTEXT_CREATE),
    ("pcre2_match_data_create_8", R_MATCH_DATA_CREATE),
    (
        "pcre2_match_data_create_from_pattern_8",
        R_MATCH_DATA_CREATE_FROM_PATTERN,
    ),
    ("pcre2_next_match_8", R_NEXT_MATCH),
    ("pcre2_pattern_convert_8", R_PATTERN_CONVERT),
    ("pcre2_pattern_info_8", R_PATTERN_INFO),
    ("pcre2_serialize_decode_8", R_SERIALIZE_DECODE),
    ("pcre2_serialize_encode_8", R_SERIALIZE_ENCODE),
    (
        "pcre2_serialize_get_number_of_codes_8",
        R_SERIALIZE_GET_NUMBER_OF_CODES,
    ),
    ("pcre2_set_bsr_8", R_SET_BSR),
    ("pcre2_set_glob_escape_8", R_SET_GLOB_ESCAPE),
    ("pcre2_set_glob_separator_8", R_SET_GLOB_SEPARATOR),
    ("pcre2_set_newline_8", R_SET_NEWLINE),
    ("pcre2_set_optimize_8", R_SET_OPTIMIZE),
    ("pcre2_set_character_tables_8", R_SET_CHARACTER_TABLES),
    ("pcre2_set_max_pattern_length_8", R_SET_MAX_PATTERN_LENGTH),
    (
        "pcre2_set_max_pattern_compiled_length_8",
        R_SET_MAX_PATTERN_COMPILED_LENGTH,
    ),
    ("pcre2_set_max_varlookbehind_8", R_SET_MAX_VARLOOKBEHIND),
    ("pcre2_set_parens_nest_limit_8", R_SET_PARENS_NEST_LIMIT),
    (
        "pcre2_set_compile_extra_options_8",
        R_SET_COMPILE_EXTRA_OPTIONS,
    ),
    (
        "pcre2_set_compile_recursion_guard_8",
        R_SET_COMPILE_RECURSION_GUARD,
    ),
    ("pcre2_set_callout_8", R_SET_CALLOUT),
    ("pcre2_set_substitute_callout_8", R_SET_SUBSTITUTE_CALLOUT),
    (
        "pcre2_set_substitute_case_callout_8",
        R_SET_SUBSTITUTE_CASE_CALLOUT,
    ),
    ("pcre2_set_heap_limit_8", R_SET_HEAP_LIMIT),
    ("pcre2_set_match_limit_8", R_SET_MATCH_LIMIT),
    ("pcre2_set_depth_limit_8", R_SET_DEPTH_LIMIT),
    ("pcre2_set_offset_limit_8", R_SET_OFFSET_LIMIT),
    ("pcre2_set_recursion_limit_8", R_SET_RECURSION_LIMIT),
    (
        "pcre2_set_recursion_memory_management_8",
        R_SET_RECURSION_MEMORY_MANAGEMENT,
    ),
    ("pcre2_substitute_8", R_SUBSTITUTE),
    ("pcre2_substring_copy_byname_8", R_SUBSTRING_COPY_BYNAME),
    ("pcre2_substring_copy_bynumber_8", R_SUBSTRING_COPY_BYNUMBER),
    ("pcre2_substring_get_byname_8", R_SUBSTRING_GET_BYNAME),
    ("pcre2_substring_get_bynumber_8", R_SUBSTRING_GET_BYNUMBER),
    ("pcre2_substring_length_byname_8", R_SUBSTRING_LENGTH_BYNAME),
    (
        "pcre2_substring_length_bynumber_8",
        R_SUBSTRING_LENGTH_BYNUMBER,
    ),
    ("pcre2_substring_list_get_8", R_SUBSTRING_LIST_GET),
    (
        "pcre2_substring_nametable_scan_8",
        R_SUBSTRING_NAMETABLE_SCAN,
    ),
    (
        "pcre2_substring_number_from_name_8",
        R_SUBSTRING_NUMBER_FROM_NAME,
    ),
];

#[test]
fn rt_row_accounting() {
    let t = table();
    assert_eq!(t.len(), 300, "_v/err_runtime.tsv must have 300 rows");

    let reachable: Vec<usize> = t.iter().filter(|r| !r.unreachable).map(|r| r.n).collect();
    let unreachable: Vec<&Row> = t.iter().filter(|r| r.unreachable).collect();
    assert_eq!(reachable.len(), 267, "expected 267 reachable rows");
    assert_eq!(unreachable.len(), 33, "expected 33 UNREACHABLE rows");

    // every per-function list contains exactly that function's reachable rows,
    // in TSV order
    for (func, rows) in EXECUTED {
        let want: Vec<usize> = t
            .iter()
            .filter(|r| !r.unreachable && r.func == *func)
            .map(|r| r.n)
            .collect();
        assert_eq!(
            rows.to_vec(),
            want,
            "{}: test covers rows {:?} but the TSV has reachable rows {:?}",
            func,
            rows,
            want
        );
    }

    let mut exec: Vec<usize> = EXECUTED.iter().flat_map(|(_, v)| v.iter().copied()).collect();
    exec.sort_unstable();
    assert_eq!(
        exec.len(),
        267,
        "the per-function lists execute {} rows, expected 267",
        exec.len()
    );
    assert_eq!(
        exec, reachable,
        "the executed row numbers are not exactly the reachable row numbers"
    );

    println!(
        "rows parsed {} / reachable {} / executed {} / skipped(UNREACHABLE) {}",
        t.len(),
        reachable.len(),
        exec.len(),
        unreachable.len()
    );
}

#[test]
fn rt_unreachable_rows_skipped() {
    let t = table();
    let mut n = 0;
    for r in t.iter().filter(|r| r.unreachable) {
        n += 1;
        println!(
            "SKIPPED row {} {} {} [{}] - reason: {}",
            r.n, r.func, r.expect, r.loc, r.notes
        );
    }
    assert_eq!(n, 33, "expected 33 UNREACHABLE rows to skip");
}

// ============================================================================
// generic boundary tests (not TSV rows): NULL pointers, zero / ZERO_TERMINATED /
// oversized lengths, start_offset == length and length+1, ovector count 0, and one
// value one step past each documented valid range.  Every case is still C vs Rust.
// ============================================================================

type Cases = Vec<(String, Res)>;

fn diff_cases(name: &str, probe: unsafe fn(&Api) -> Cases) {
    let (c, r) = both();
    let cv = unsafe { probe(c) };
    let rv = unsafe { probe(r) };
    assert_eq!(
        cv.len(),
        rv.len(),
        "{}: case counts differ ({} vs {})",
        name,
        cv.len(),
        rv.len()
    );
    for i in 0..cv.len() {
        assert_eq!(cv[i].0, rv[i].0, "{}: case labels out of step", name);
        assert_eq!(
            cv[i].1, rv[i].1,
            "{}: RUST DIVERGES FROM C on boundary case {:?}\n    C   : {:?}\n    RUST: {:?}",
            name, cv[i].0, cv[i].1, rv[i].1
        );
    }
    println!("{}: {} boundary cases compared", name, cv.len());
}

unsafe fn raw_compile(a: &Api, pat: *const u8, plen: usize, pass_ec: bool, pass_eo: bool) -> Res {
    let mut ec: i32 = 0x7f7f_7f7f;
    let mut eo: usize = SENTINEL;
    let code = (a.pcre2_compile_8)(
        pat,
        plen,
        0,
        if pass_ec { &mut ec } else { ptr::null_mut() },
        if pass_eo { &mut eo } else { ptr::null_mut() },
        ptr::null_mut(),
    );
    let res = Res::IX(
        code.is_null() as i64,
        vec![ec as i64, eo as i64],
    );
    if !code.is_null() {
        (a.pcre2_code_free_8)(code);
    }
    res
}

unsafe fn bd_null_probe(a: &Api) -> Cases {
    let mut v: Cases = Vec::new();
    let mut push = |l: &str, r: Res| v.push((l.to_string(), r));

    // ---- pcre2_compile: documented NULL behaviour
    push("compile(NULL, 0)", raw_compile(a, ptr::null(), 0, true, true));
    push("compile(NULL, 1)", raw_compile(a, ptr::null(), 1, true, true));
    push(
        "compile(a, 1, erroroffset=NULL)",
        raw_compile(a, b"a".as_ptr(), 1, true, false),
    );
    push(
        "compile(a, 1, errorcode=NULL)",
        raw_compile(a, b"a".as_ptr(), 1, false, true),
    );
    push(
        "compile(a, PCRE2_ZERO_TERMINATED)",
        raw_compile(a, b"a\0".as_ptr(), PCRE2_ZERO_TERMINATED, true, true),
    );
    // pattern length that runs past the text into (legal, zero-filled) memory
    let padded = {
        let mut p = vec![0u8; 32];
        p[0] = b'a';
        p
    };
    push(
        "compile(a + 31 NULs, 32)",
        raw_compile(a, padded.as_ptr(), 32, true, true),
    );

    // ---- code copies of NULL
    let cc1 = (a.pcre2_code_copy_8)(ptr::null_mut());
    push("code_copy(NULL)", Res::P(cc1.is_null()));
    if !cc1.is_null() {
        (a.pcre2_code_free_8)(cc1);
    }
    let cc2 = (a.pcre2_code_copy_with_tables_8)(ptr::null_mut());
    push("code_copy_with_tables(NULL)", Res::P(cc2.is_null()));
    if !cc2.is_null() {
        (a.pcre2_code_free_8)(cc2);
    }

    // ---- matching with NULL / empty subjects
    let c = comp(a, b"a?", 0, 0);
    let md = (a.pcre2_match_data_create_8)(4, ptr::null_mut());
    let mut ws = vec![0i32; 1000];
    let w = ws.as_mut_ptr();
    push(
        "match(md=NULL)",
        m_res(a, c, b"a".as_ptr(), 1, 0, 0, ptr::null_mut(), ptr::null_mut()),
    );
    push(
        "match(code=NULL)",
        m_res(a, ptr::null_mut(), b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut()),
    );
    push(
        "match(subject=NULL, len=0)",
        m_res(a, c, ptr::null(), 0, 0, 0, md, ptr::null_mut()),
    );
    push(
        "match(subject=NULL, len=1)",
        m_res(a, c, ptr::null(), 1, 0, 0, md, ptr::null_mut()),
    );
    push(
        "dfa_match(md=NULL)",
        d_res(a, c, b"a".as_ptr(), 1, 0, 0, ptr::null_mut(), ptr::null_mut(), w, 1000),
    );
    push(
        "dfa_match(code=NULL)",
        d_res(a, ptr::null_mut(), b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut(), w, 1000),
    );
    push(
        "dfa_match(workspace=NULL)",
        d_res(a, c, b"a".as_ptr(), 1, 0, 0, md, ptr::null_mut(), ptr::null_mut(), 1000),
    );
    push(
        "dfa_match(subject=NULL, len=0)",
        d_res(a, c, ptr::null(), 0, 0, 0, md, ptr::null_mut(), w, 1000),
    );
    push(
        "dfa_match(subject=NULL, len=1)",
        d_res(a, c, ptr::null(), 1, 0, 0, md, ptr::null_mut(), w, 1000),
    );

    // ---- substitute with NULL subject / replacement of zero length
    let mut buf = [0xAAu8; SUBBUF];
    push(
        "substitute(subject=NULL, len=0)",
        sub_call(
            a,
            c,
            ptr::null(),
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            b"X".as_ptr(),
            1,
            &mut buf,
            SUBBUF,
        ),
    );
    push(
        "substitute(replacement=NULL, rlen=0)",
        sub_call(
            a,
            c,
            b"a".as_ptr(),
            1,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
            0,
            &mut buf,
            SUBBUF,
        ),
    );
    push(
        "substitute(subject=NULL/0, replacement=NULL/0)",
        sub_call(
            a,
            c,
            ptr::null(),
            0,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
            0,
            &mut buf,
            SUBBUF,
        ),
    );
    push(
        "substitute(subject ZERO_TERMINATED)",
        sub_call(
            a,
            c,
            b"aaa\0".as_ptr(),
            PCRE2_ZERO_TERMINATED,
            0,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
            b"X\0".as_ptr(),
            PCRE2_ZERO_TERMINATED,
            &mut buf,
            SUBBUF,
        ),
    );
    (a.pcre2_match_data_free_8)(md);

    // ---- pattern_info: `where == NULL` is the size-request path for every key
    for key in 0..=27u32 {
        let mut sz: usize = SENTINEL;
        let rc = (a.pcre2_pattern_info_8)(c, key, ptr::null_mut());
        let rc2 = (a.pcre2_pattern_info_8)(ptr::null_mut(), key, ptr::null_mut());
        let rc3 = (a.pcre2_pattern_info_8)(c, key, &mut sz as *mut usize as Ptr);
        v.push((
            format!("pattern_info(key={}) NULL-where / NULL-code / value", key),
            Res::IX(rc as i64, vec![rc2 as i64, rc3 as i64, sz as i64]),
        ));
    }
    (a.pcre2_code_free_8)(c);

    // ---- pcre2_config: `where == NULL` is the length-request path
    for key in 0..=20u32 {
        let mut val: u32 = 0xdead_beef;
        let mut cbuf = [0xAAu8; 256];
        let rc_len = (a.pcre2_config_8)(key, ptr::null_mut());
        let rc_val = if key == PCRE2_CONFIG_VERSION
            || key == PCRE2_CONFIG_UNICODE_VERSION
            || key == PCRE2_CONFIG_JITTARGET
        {
            (a.pcre2_config_8)(key, cbuf.as_mut_ptr() as Ptr)
        } else {
            (a.pcre2_config_8)(key, &mut val as *mut u32 as Ptr)
        };
        v.push((
            format!("config(key={})", key),
            Res::IB(
                rc_len as i64,
                vec![rc_val as i64, val as i64],
                cbuf.to_vec(),
            ),
        ));
    }
    let mut val: u32 = 0xdead_beef;
    v.push((
        "config(key=999)".to_string(),
        Res::IX(
            (a.pcre2_config_8)(999, ptr::null_mut()) as i64,
            vec![
                (a.pcre2_config_8)(999, &mut val as *mut u32 as Ptr) as i64,
                val as i64,
            ],
        ),
    ));

    // ---- serialize with NULL / non-positive counts
    let mut out: [Ptr; 2] = [ptr::null_mut(); 2];
    v.push((
        "serialize_decode(NULL, -1, NULL, NULL)".to_string(),
        Res::I((a.pcre2_serialize_decode_8)(out.as_mut_ptr(), -1, ptr::null(), ptr::null_mut()) as i64),
    ));
    v.push((
        "serialize_get_number_of_codes(NULL)".to_string(),
        Res::I((a.pcre2_serialize_get_number_of_codes_8)(ptr::null()) as i64),
    ));
    let mut bp: *mut u8 = ptr::null_mut();
    let mut bl: usize = SENTINEL;
    v.push((
        "serialize_encode(NULL, -1)".to_string(),
        Res::IX(
            (a.pcre2_serialize_encode_8)(ptr::null(), -1, &mut bp, &mut bl, ptr::null_mut()) as i64,
            vec![bp.is_null() as i64, bl as i64],
        ),
    ));

    // ---- jit stubs with NULL
    v.push((
        "jit_compile(NULL, 0)".to_string(),
        Res::I((a.pcre2_jit_compile_8)(ptr::null_mut(), 0) as i64),
    ));
    (a.pcre2_jit_free_unused_memory_8)(ptr::null_mut());
    (a.pcre2_jit_stack_free_8)(ptr::null_mut());

    // ---- pcre2_set_optimize with a NULL context, both valid directives
    v.push((
        "set_optimize(NULL, NONE/FULL)".to_string(),
        Res::IX(
            (a.pcre2_set_optimize_8)(ptr::null_mut(), 0) as i64,
            vec![(a.pcre2_set_optimize_8)(ptr::null_mut(), 1) as i64],
        ),
    ));

    // ---- every free function must tolerate NULL
    (a.pcre2_code_free_8)(ptr::null_mut());
    (a.pcre2_match_data_free_8)(ptr::null_mut());
    (a.pcre2_general_context_free_8)(ptr::null_mut());
    (a.pcre2_compile_context_free_8)(ptr::null_mut());
    (a.pcre2_match_context_free_8)(ptr::null_mut());
    (a.pcre2_convert_context_free_8)(ptr::null_mut());
    (a.pcre2_substring_free_8)(ptr::null_mut());
    (a.pcre2_substring_list_free_8)(ptr::null_mut());
    (a.pcre2_serialize_free_8)(ptr::null_mut());
    (a.pcre2_converted_pattern_free_8)(ptr::null_mut());
    v.push(("all free(NULL) calls survived".to_string(), Res::I(0)));

    // ---- maketables(NULL): full table image must be identical
    let mut tlen: u32 = 0;
    (a.pcre2_config_8)(PCRE2_CONFIG_TABLES_LENGTH, &mut tlen as *mut u32 as Ptr);
    let tables = (a.pcre2_maketables_8)(ptr::null_mut());
    assert!(!tables.is_null());
    v.push((
        "maketables(NULL) image".to_string(),
        Res::IB(
            tlen as i64,
            vec![],
            std::slice::from_raw_parts(tables, tlen as usize).to_vec(),
        ),
    ));
    (a.pcre2_maketables_free_8)(ptr::null_mut(), tables);
    v
}

#[test]
fn bd_null_pointers_and_documented_null_checks() {
    diff_cases("bd_null_pointers", bd_null_probe);
}

unsafe fn bd_lengths_probe(a: &Api) -> Cases {
    let mut v: Cases = Vec::new();
    let mut ws = vec![0i32; 1000];
    let w = ws.as_mut_ptr();
    let mut buf = [0xAAu8; SUBBUF];
    // A zero-filled tail so "oversized" lengths still address legal memory.
    let mut subj = vec![0u8; 64];
    subj[0] = b'a';
    subj[1] = b'b';
    subj[2] = b'c';

    for pat in [&b"abc"[..], &b"a?"[..], &b"^"[..], &b"(a)(b)"[..]] {
        let c = comp(a, pat, 0, 0);
        for &oveccount in &[0u32, 1, 2, 3] {
            let md = (a.pcre2_match_data_create_8)(oveccount, ptr::null_mut());
            assert!(!md.is_null());
            let count = (a.pcre2_get_ovector_count_8)(md);
            v.push((
                format!("ovector_count(create({}))", oveccount),
                Res::I(count as i64),
            ));
            for &len in &[0usize, 1, 3, 4, 64, PCRE2_ZERO_TERMINATED] {
                let real = if len == PCRE2_ZERO_TERMINATED { 3 } else { len };
                for &so in &[0usize, real, real + 1, real.wrapping_add(2)] {
                    let label = format!(
                        "pat={:?} ovec={} len={} start={}",
                        String::from_utf8_lossy(pat),
                        oveccount,
                        if len == PCRE2_ZERO_TERMINATED {
                            "ZERO_TERMINATED".to_string()
                        } else {
                            len.to_string()
                        },
                        so
                    );
                    v.push((
                        format!("match {}", label),
                        m_res(a, c, subj.as_ptr(), len, so, 0, md, ptr::null_mut()),
                    ));
                    v.push((
                        format!("dfa_match {}", label),
                        d_res(a, c, subj.as_ptr(), len, so, 0, md, ptr::null_mut(), w, 1000),
                    ));
                    v.push((
                        format!("substitute {}", label),
                        sub_call(
                            a,
                            c,
                            subj.as_ptr(),
                            len,
                            so,
                            0,
                            ptr::null_mut(),
                            ptr::null_mut(),
                            b"<$0>".as_ptr(),
                            4,
                            &mut buf,
                            SUBBUF,
                        ),
                    ));
                }
            }
            (a.pcre2_match_data_free_8)(md);
        }
        // dfa workspace count boundary: 19 is too small, 20 is the minimum
        let md = (a.pcre2_match_data_create_8)(2, ptr::null_mut());
        for &wsc in &[0usize, 1, 19, 20, 21] {
            v.push((
                format!(
                    "dfa wscount={} pat={:?}",
                    wsc,
                    String::from_utf8_lossy(pat)
                ),
                d_res(a, c, subj.as_ptr(), 3, 0, 0, md, ptr::null_mut(), w, wsc),
            ));
        }
        // substitute output buffer sizes around the required length
        for &cap in &[0usize, 1, 2, 5, 6, 7, 8, 64] {
            v.push((
                format!(
                    "substitute cap={} pat={:?}",
                    cap,
                    String::from_utf8_lossy(pat)
                ),
                sub_call(
                    a,
                    c,
                    subj.as_ptr(),
                    3,
                    0,
                    PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                    ptr::null_mut(),
                    ptr::null_mut(),
                    b"<$0>".as_ptr(),
                    4,
                    &mut buf,
                    cap,
                ),
            ));
        }
        (a.pcre2_match_data_free_8)(md);
        (a.pcre2_code_free_8)(c);
    }
    v
}

#[test]
fn bd_zero_and_oversized_lengths_and_offsets() {
    diff_cases("bd_lengths_offsets", bd_lengths_probe);
}

unsafe fn bd_ranges_probe(a: &Api) -> Cases {
    let mut v: Cases = Vec::new();

    // ---- compile-context setters: one step past each documented range
    let cc = (a.pcre2_compile_context_create_8)(ptr::null_mut());
    for x in 0..=8u32 {
        v.push((
            format!("set_bsr({})", x),
            Res::I((a.pcre2_set_bsr_8)(cc, x) as i64),
        ));
        v.push((
            format!("set_newline({})", x),
            Res::I((a.pcre2_set_newline_8)(cc, x) as i64),
        ));
    }
    for x in [0u32, 1, 2, 3, 62, 63, 64, 70, 0xffff_ffff] {
        v.push((
            format!("set_optimize({})", x),
            Res::I((a.pcre2_set_optimize_8)(cc, x) as i64),
        ));
    }
    for x in [0usize, 1, usize::MAX] {
        v.push((
            format!("set_max_pattern_length({})", x),
            Res::I((a.pcre2_set_max_pattern_length_8)(cc, x) as i64),
        ));
        v.push((
            format!("set_max_pattern_compiled_length({})", x),
            Res::I((a.pcre2_set_max_pattern_compiled_length_8)(cc, x) as i64),
        ));
    }
    for x in [0u32, 1, 255, 256, 65535, 0xffff_ffff] {
        v.push((
            format!("set_max_varlookbehind({})", x),
            Res::I((a.pcre2_set_max_varlookbehind_8)(cc, x) as i64),
        ));
        v.push((
            format!("set_parens_nest_limit({})", x),
            Res::I((a.pcre2_set_parens_nest_limit_8)(cc, x) as i64),
        ));
    }
    (a.pcre2_compile_context_free_8)(cc);

    // ---- convert-context setters
    let vc = (a.pcre2_convert_context_create_8)(ptr::null_mut());
    for x in [
        0u32, b'/' as u32, b'\\' as u32, b'.' as u32, b':' as u32, b'a' as u32, b'0' as u32, 127,
        128, 255, 256, 0xffff_ffff,
    ] {
        v.push((
            format!("set_glob_escape({})", x),
            Res::I((a.pcre2_set_glob_escape_8)(vc, x) as i64),
        ));
        v.push((
            format!("set_glob_separator({})", x),
            Res::I((a.pcre2_set_glob_separator_8)(vc, x) as i64),
        ));
    }
    (a.pcre2_convert_context_free_8)(vc);

    // ---- match-context setters (documented as never rejecting)
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    for x in [0u32, 1, 0xffff_ffff] {
        v.push((
            format!("set_heap_limit({})", x),
            Res::I((a.pcre2_set_heap_limit_8)(mc, x) as i64),
        ));
        v.push((
            format!("set_match_limit({})", x),
            Res::I((a.pcre2_set_match_limit_8)(mc, x) as i64),
        ));
        v.push((
            format!("set_depth_limit({})", x),
            Res::I((a.pcre2_set_depth_limit_8)(mc, x) as i64),
        ));
        v.push((
            format!("set_recursion_limit({})", x),
            Res::I((a.pcre2_set_recursion_limit_8)(mc, x) as i64),
        ));
    }
    for x in [0usize, 1, PCRE2_UNSET] {
        v.push((
            format!("set_offset_limit({})", x),
            Res::I((a.pcre2_set_offset_limit_8)(mc, x) as i64),
        ));
    }
    (a.pcre2_match_context_free_8)(mc);

    // ---- ovector counts at the clamping boundaries
    for x in [0u32, 1, 2, 65534, 65535, 65536, 70000, 0xffff_ffff] {
        let md = (a.pcre2_match_data_create_8)(x, ptr::null_mut());
        assert!(!md.is_null());
        v.push((
            format!("match_data_create({}) ovector_count", x),
            Res::IX(
                (a.pcre2_get_ovector_count_8)(md) as i64,
                vec![(a.pcre2_get_match_data_size_8)(md) as i64],
            ),
        ));
        (a.pcre2_match_data_free_8)(md);
    }

    // ---- pcre2_jit_compile option bits, valid and invalid
    let c = comp(a, b"a", 0, 0);
    for x in [
        0u32,
        PCRE2_JIT_COMPLETE,
        PCRE2_JIT_PARTIAL_SOFT,
        PCRE2_JIT_PARTIAL_HARD,
        PCRE2_JIT_INVALID_UTF,
        PCRE2_JIT_TEST_ALLOC,
        PCRE2_JIT_COMPLETE | PCRE2_JIT_PARTIAL_SOFT | PCRE2_JIT_PARTIAL_HARD,
        0x8,
        0x1000,
        0xffff_ffff,
    ] {
        v.push((
            format!("jit_compile({:#x})", x),
            Res::I((a.pcre2_jit_compile_8)(c, x) as i64),
        ));
    }
    (a.pcre2_code_free_8)(c);

    // ---- pcre2_get_error_message over the whole error-number range
    for code in -80i32..=200 {
        for &size in &[0usize, 1, 4, 64] {
            let mut b = [0xAAu8; 128];
            let rc = (a.pcre2_get_error_message_8)(code, b.as_mut_ptr(), size);
            v.push((
                format!("get_error_message({}, size={})", code, size),
                Res::IB(rc as i64, vec![], b.to_vec()),
            ));
        }
    }
    v
}

#[test]
fn bd_one_step_past_each_documented_range() {
    diff_cases("bd_ranges", bd_ranges_probe);
}

// ============================================================================
// randomized sweep of the same runtime error surface (fixed seed, so every run
// uses identical inputs).  Random subjects / lengths / start offsets / option
// masks (including undefined option bits) are driven through pcre2_match,
// pcre2_dfa_match and pcre2_substitute; the return code and every output value
// must agree between the two libraries.
// ============================================================================

const FUZZ_PATTERNS: &[(&[u8], u32)] = &[
    (b"abc", 0),
    (b"a?", 0),
    (b"(a)(b)?(c)", 0),
    (b"(a)\\1", 0),
    (b"((?1)|a)", 0),
    (b"(a(?1)?)", 0),
    (b"a(?C1)b", 0),
    (b"(?<n>a)|(?<m>b)", 0),
    (b"\\C", PCRE2_UTF),
    (b".", PCRE2_UTF),
    (b"(*UTF)\\X+", 0),
    (b"a\\Kb", 0),
    (b"^(?:a|ab|abc)$", PCRE2_NO_START_OPTIMIZE),
    (b"[^a]*", 0),
    (b"(?<=a)b", 0),
];

const FUZZ_MATCH_OPTS: &[u32] = &[
    0,
    PCRE2_ANCHORED,
    PCRE2_ENDANCHORED,
    PCRE2_NOTBOL,
    PCRE2_NOTEOL,
    PCRE2_NOTEMPTY,
    PCRE2_NOTEMPTY_ATSTART,
    PCRE2_PARTIAL_SOFT,
    PCRE2_PARTIAL_HARD,
    PCRE2_NO_UTF_CHECK,
    PCRE2_COPY_MATCHED_SUBJECT,
    PCRE2_DFA_SHORTEST,
    PCRE2_DISABLE_RECURSELOOP_CHECK, // illegal for dfa_match
    PCRE2_SUBSTITUTE_GLOBAL,         // illegal for match
    0x0800_0000,                     // undefined option bit
];

#[test]
fn rand_runtime_error_surface() {
    let (c, r) = both();
    let mut cases = 0usize;
    for pi in 0..FUZZ_PATTERNS.len() {
        let (pat, popts) = FUZZ_PATTERNS[pi];
        // One RNG per pattern, seeded from the pattern index: identical inputs for
        // both libraries and reproducible across runs.
        let mut rng_c = Rng::new(0x5eed_0000 + pi as u64);
        let mut rng_r = Rng::new(0x5eed_0000 + pi as u64);
        let cp = unsafe { fuzz_pattern(c, pat, popts, &mut rng_c) };
        let rp = unsafe { fuzz_pattern(r, pat, popts, &mut rng_r) };
        assert_eq!(cp.len(), rp.len());
        for i in 0..cp.len() {
            assert_eq!(cp[i].0, rp[i].0, "fuzz case labels out of step");
            assert_eq!(
                cp[i].1, rp[i].1,
                "RUST DIVERGES FROM C on random case {:?}\n    C   : {:?}\n    RUST: {:?}",
                cp[i].0, cp[i].1, rp[i].1
            );
            cases += 1;
        }
    }
    println!("rand_runtime_error_surface: {} random cases compared", cases);
}

unsafe fn fuzz_pattern(a: &Api, pat: &[u8], popts: u32, rng: &mut Rng) -> Cases {
    let mut v: Cases = Vec::new();
    let c = comp(a, pat, popts, 0);
    let mut ws = vec![0i32; 200];
    let w = ws.as_mut_ptr();
    let mut buf = [0xAAu8; SUBBUF];
    // bound the work so one pattern cannot dominate the runtime
    let mc = (a.pcre2_match_context_create_8)(ptr::null_mut());
    (a.pcre2_set_match_limit_8)(mc, 10_000);
    (a.pcre2_set_depth_limit_8)(mc, 200);
    (a.pcre2_set_heap_limit_8)(mc, 64);

    let alphabet: &[u8] = b"ab\xc3\xa9\x80\nc\r\0x";
    for it in 0..140 {
        let slen = rng.below(9) as usize;
        let subj: Vec<u8> = (0..slen).map(|_| *rng.pick(alphabet)).collect();
        let len = match rng.below(6) {
            0 => 0,
            1 => slen,
            2 => slen,
            3 => slen,
            4 => {
                if slen > 0 {
                    slen - 1
                } else {
                    0
                }
            }
            _ => slen,
        };
        let so = match rng.below(5) {
            0 => 0,
            1 => len,
            2 => len + 1,
            3 => rng.below((len + 2) as u32) as usize,
            _ => 0,
        };
        let opts = *rng.pick(FUZZ_MATCH_OPTS) | *rng.pick(FUZZ_MATCH_OPTS);
        let oveccount = rng.below(4);
        let wsc = *rng.pick(&[0usize, 19, 20, 21, 200]);
        let md = (a.pcre2_match_data_create_8)(oveccount, ptr::null_mut());
        let label = format!(
            "pat={:?} it={} subj={:?} len={} so={} opts={:#x} ovec={} wsc={}",
            String::from_utf8_lossy(pat),
            it,
            subj,
            len,
            so,
            opts,
            oveccount,
            wsc
        );
        v.push((
            format!("match {}", label),
            m_res(a, c, subj.as_ptr(), len, so, opts, md, mc),
        ));
        v.push((
            format!("dfa {}", label),
            d_res(a, c, subj.as_ptr(), len, so, opts, md, mc, w, wsc),
        ));
        let rep: &[u8] = rng.pick(&[
            &b"X"[..],
            &b"$0"[..],
            &b"$1"[..],
            &b"${1:+y:n}"[..],
            &b"\\U$0"[..],
            &b"$*"[..],
            &b"$"[..],
            &b"${9}"[..],
        ]);
        let sopts = opts
            | *rng.pick(&[
                0u32,
                PCRE2_SUBSTITUTE_GLOBAL,
                PCRE2_SUBSTITUTE_EXTENDED,
                PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
                PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
                PCRE2_SUBSTITUTE_UNSET_EMPTY,
                PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                PCRE2_SUBSTITUTE_LITERAL,
            ]);
        let cap = *rng.pick(&[0usize, 1, 4, SUBBUF]);
        v.push((
            format!("substitute {} rep={:?} sopts={:#x} cap={}", label, rep, sopts, cap),
            sub_call(
                a,
                c,
                subj.as_ptr(),
                len,
                so,
                sopts,
                ptr::null_mut(),
                mc,
                rep.as_ptr(),
                rep.len(),
                &mut buf,
                cap,
            ),
        ));
        (a.pcre2_match_data_free_8)(md);
    }
    (a.pcre2_match_context_free_8)(mc);
    (a.pcre2_code_free_8)(c);
    v
}

// APPEND_MARKER
