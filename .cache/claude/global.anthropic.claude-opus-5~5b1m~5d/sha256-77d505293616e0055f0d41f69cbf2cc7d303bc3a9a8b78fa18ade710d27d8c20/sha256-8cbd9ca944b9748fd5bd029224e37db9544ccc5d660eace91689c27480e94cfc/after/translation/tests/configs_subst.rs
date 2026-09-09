//! Differential tests for CONFIGS.md rows C170-C189:
//!   * C170-C183 — `pcre2_substitute_8` in all its configurations
//!   * C184-C187 — the eleven `pcre2_substring_*` functions
//!   * C188-C189 — `pcre2_serialize_encode/decode/get_number_of_codes`
//!
//! Everything goes through the two loaded `.so`s (see `tests/common/mod.rs`).
//! Nothing is ever mixed between the libraries: a code object, its match data,
//! its match context and the frees all come from one side.

mod common;
use common::*;

use std::cell::{Cell, RefCell};
use std::ptr;

// ======================================================= C constants
// pcre2.h:517-519
const CASE_LOWER: i32 = 1;
const CASE_UPPER: i32 = 2;
const CASE_TITLE_FIRST: i32 = 3;

// pcre2_substitute.c:48-52
const SUBSTITUTE_OPTIONS: u32 = PCRE2_SUBSTITUTE_EXTENDED
    | PCRE2_SUBSTITUTE_GLOBAL
    | PCRE2_SUBSTITUTE_LITERAL
    | PCRE2_SUBSTITUTE_MATCHED
    | PCRE2_SUBSTITUTE_OVERFLOW_LENGTH
    | PCRE2_SUBSTITUTE_REPLACEMENT_ONLY
    | PCRE2_SUBSTITUTE_UNKNOWN_UNSET
    | PCRE2_SUBSTITUTE_UNSET_EMPTY;

// offsetof(pcre2_real_code, ...) / sizeof, verified by compiling a probe against
// c_src/src/pcre2_intmodedep.h:660 with the project's own config.h.
const SIZEOF_REAL_CODE: usize = 152;
const RC_BLOCKSIZE: usize = 72;
const RC_MAGIC: usize = 88;
const RC_NAME_ENTRY_SIZE: usize = 140;
const RC_NAME_COUNT: usize = 142;

// sizeof(pcre2_serialized_data) — pcre2_internal.h:2169
const SERIALIZED_HDR: usize = 16;
// pcre2_internal.h:598
const TABLES_LENGTH: usize = 1088;
// pcre2_internal.h:542
const MAGIC_NUMBER: u32 = 0x5043_5245;

/// poison byte used to fill every output buffer before a call
const POISON: u8 = 0xAA;
/// number of poison bytes kept past the declared buffer length
const TAIL: usize = 8;

// ======================================================= observations

#[derive(PartialEq, Eq, Clone, Debug)]
struct MdProbe {
    lenrc: i32,
    len: usize,
    ov: Vec<usize>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
struct SubOut {
    rc: i32,
    outlen: usize,
    buf: Vec<u8>,
    md: Option<MdProbe>,
}

/// A mirror of `pcre2_substitute_callout_block` (pcre2.h:616-626).
/// size 56, input@8, output@16, output_offsets@24, ovector@40, oveccount@48,
/// subscount@52 — verified against the C headers.
#[repr(C)]
struct ScBlock {
    version: u32,
    input: *const u8,
    output: *const u8,
    output_offsets: [usize; 2],
    ovector: *mut usize,
    oveccount: u32,
    subscount: u32,
}

#[derive(PartialEq, Eq, Clone, Debug)]
struct ScLog {
    version: u32,
    input_is_subject: bool,
    output_is_buffer: bool,
    o0: usize,
    o1: usize,
    oveccount: u32,
    subscount: u32,
    ov: Vec<usize>,
    data: Option<u64>,
    ret: i32,
}

#[derive(PartialEq, Eq, Clone, Debug)]
struct CcLog {
    input: Vec<u8>,
    input_len: usize,
    out_off: Option<isize>,
    in_eq_out: bool,
    output_cap: usize,
    to_case: i32,
    data: Option<u64>,
    ret: usize,
}

// ======================================================= callout plumbing

thread_local! {
    static CB_SUBJ:   Cell<*const u8> = Cell::new(ptr::null());
    static CB_BUF:    Cell<*mut u8>   = Cell::new(ptr::null_mut());
    static CB_BUFLEN: Cell<usize>     = Cell::new(0);
    static SC_LOG:    RefCell<Vec<ScLog>> = RefCell::new(Vec::new());
    static CC_LOG:    RefCell<Vec<CcLog>> = RefCell::new(Vec::new());
    static SC_POLICY: Cell<u32> = Cell::new(0);
    static CC_POLICY: Cell<u32> = Cell::new(0);
    /// -1 = unlimited, otherwise the number of further successful allocations
    static MALLOC_BUDGET: Cell<i64> = Cell::new(-1);
    /// coverage bookkeeping: how often each callout fired, and which return
    /// codes pcre2_substitute produced (so a test cannot silently degenerate
    /// into "every case is the same error")
    static SC_CALLS: Cell<usize> = Cell::new(0);
    static CC_CALLS: Cell<usize> = Cell::new(0);
    static RC_SEEN: RefCell<std::collections::BTreeSet<i32>> =
        RefCell::new(std::collections::BTreeSet::new());
}

fn rc_seen() -> Vec<i32> {
    RC_SEEN.with(|s| s.borrow().iter().copied().collect())
}

static SENTINEL: u64 = 0xDEAD_BEEF_CAFE_0001;

fn sentinel_ptr() -> Ptr {
    &SENTINEL as *const u64 as Ptr
}

/// `pcre2_set_substitute_callout` callback. Logs the whole block and returns a
/// value determined by the (deterministic) policy, so both libraries see the
/// same decisions.
unsafe extern "C" fn sc_cb(blk: Ptr, data: Ptr) -> i32 {
    let b = &*(blk as *const ScBlock);
    let subj = CB_SUBJ.with(|c| c.get());
    let bufp = CB_BUF.with(|c| c.get());
    let n = (b.oveccount as usize) * 2;
    let ov = if b.ovector.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(b.ovector, n).to_vec()
    };
    let d = if data.is_null() {
        None
    } else {
        Some(*(data as *const u64))
    };
    let ret = match SC_POLICY.with(|c| c.get()) {
        0 => 0,
        1 => 1,
        2 => -1,
        3 => {
            if b.subscount % 2 == 0 {
                0
            } else {
                1
            }
        }
        4 => {
            if b.subscount >= 2 {
                -1
            } else {
                0
            }
        }
        5 => {
            if b.subscount == 1 {
                1
            } else {
                0
            }
        }
        _ => 0,
    };
    SC_CALLS.with(|x| x.set(x.get() + 1));
    SC_LOG.with(|l| {
        l.borrow_mut().push(ScLog {
            version: b.version,
            input_is_subject: b.input == subj,
            output_is_buffer: b.output == bufp as *const u8,
            o0: b.output_offsets[0],
            o1: b.output_offsets[1],
            oveccount: b.oveccount,
            subscount: b.subscount,
            ov,
            data: d,
            ret,
        })
    });
    ret
}

unsafe fn cc_write(output: *mut u8, cap: usize, data: &[u8]) {
    let n = data.len().min(cap);
    if n > 0 && !output.is_null() {
        ptr::copy(data.as_ptr(), output, n);
    }
}

/// `pcre2_set_substitute_case_callout` callback.
unsafe extern "C" fn cc_cb(
    input: Sptr,
    input_len: usize,
    output: *mut u8,
    output_cap: usize,
    to_case: i32,
    data: Ptr,
) -> usize {
    let src: Vec<u8> = if input.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(input, input_len).to_vec()
    };
    let bufp = CB_BUF.with(|c| c.get());
    let buflen = CB_BUFLEN.with(|c| c.get());
    let out_off = if !bufp.is_null()
        && output as usize >= bufp as usize
        && (output as usize) <= bufp as usize + buflen + TAIL
    {
        Some(output as isize - bufp as isize)
    } else {
        None
    };
    let d = if data.is_null() {
        None
    } else {
        Some(*(data as *const u64))
    };

    let xf: Vec<u8> = match to_case {
        CASE_LOWER => src.iter().map(|b| b.to_ascii_lowercase()).collect(),
        CASE_UPPER => src.iter().map(|b| b.to_ascii_uppercase()).collect(),
        CASE_TITLE_FIRST => src
            .iter()
            .enumerate()
            .map(|(i, b)| {
                if i == 0 {
                    b.to_ascii_uppercase()
                } else {
                    b.to_ascii_lowercase()
                }
            })
            .collect(),
        _ => src.clone(),
    };

    let pol = CC_POLICY.with(|c| c.get());
    let ret: usize = match pol {
        // 0: well behaved
        0 => {
            cc_write(output, output_cap, &xf);
            xf.len()
        }
        // 1: claims one code unit less than requested
        1 => {
            let n = xf.len().saturating_sub(1);
            cc_write(output, output_cap, &xf[..n]);
            n
        }
        // 2: claims three code units more than requested
        2 => {
            let mut v = xf.clone();
            v.extend_from_slice(b"!!!");
            cc_write(output, output_cap, &v);
            v.len()
        }
        // 3: returns zero
        3 => 0,
        // 4: signals an error
        4 => {
            CC_CALLS.with(|x| x.set(x.get() + 1));
            CC_LOG.with(|l| {
                l.borrow_mut().push(CcLog {
                    input: src,
                    input_len,
                    out_off,
                    in_eq_out: input as usize == output as usize,
                    output_cap,
                    to_case,
                    data: d,
                    ret: usize::MAX,
                })
            });
            return usize::MAX;
        }
        // 5: doubles every code unit (like sharp-s -> SS)
        _ => {
            let mut v = Vec::new();
            for &b in &xf {
                v.push(b);
                v.push(b);
            }
            cc_write(output, output_cap, &v);
            v.len()
        }
    };
    CC_CALLS.with(|x| x.set(x.get() + 1));
    CC_LOG.with(|l| {
        l.borrow_mut().push(CcLog {
            input: src,
            input_len,
            out_off,
            in_eq_out: input as usize == output as usize,
            output_cap,
            to_case,
            data: d,
            ret,
        })
    });
    ret
}

// ------------------------------------------------- failing allocator

unsafe extern "C" fn t_malloc(size: usize, _data: Ptr) -> Ptr {
    let b = MALLOC_BUDGET.with(|c| c.get());
    if b >= 0 {
        if b == 0 {
            return ptr::null_mut();
        }
        MALLOC_BUDGET.with(|c| c.set(b - 1));
    }
    let total = size + 16;
    let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
    let p = std::alloc::alloc(layout);
    if p.is_null() {
        return ptr::null_mut();
    }
    *(p as *mut usize) = total;
    p.add(16) as Ptr
}

unsafe extern "C" fn t_free(p: Ptr, _data: Ptr) {
    if p.is_null() {
        return;
    }
    let base = (p as *mut u8).sub(16);
    let total = *(base as *const usize);
    std::alloc::dealloc(base, std::alloc::Layout::from_size_align(total, 16).unwrap());
}

// ======================================================= call helpers

#[derive(Clone, Copy)]
struct SubArgs {
    subj: *const u8,
    slen: usize,
    start: usize,
    options: u32,
    rep: *const u8,
    rlen: usize,
    /// number of bytes actually allocated for the output buffer
    buflen: usize,
    /// what to put in `*blength` on entry; `None` means `buflen`
    declared: Option<usize>,
    null_buf: bool,
}

fn args(subj: &[u8], rep: &[u8]) -> SubArgs {
    SubArgs {
        subj: subj.as_ptr(),
        slen: subj.len(),
        start: 0,
        options: 0,
        rep: rep.as_ptr(),
        rlen: rep.len(),
        buflen: 96,
        declared: None,
        null_buf: false,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum MdMode {
    /// pass NULL, so pcre2_substitute creates an internal match data block
    Null,
    /// pass a caller-owned block, initialised deterministically
    Fresh,
    /// pass a caller-owned block that already holds a match of the same call
    Matched,
}

/// A dummy one-byte subject used only to initialise a match data block.
static DUMMY: [u8; 1] = [0];

/// Create a match data block whose every observable field is initialised
/// deterministically: run one match on an empty subject (which sets rc, subject,
/// subject_length, start_offset, options, code, mark, matchedby and startchar)
/// and then set the whole ovector to PCRE2_UNSET.
unsafe fn fresh_md(api: &Api, code: Ptr) -> Ptr {
    let md = (api.pcre2_match_data_create_from_pattern_8)(code, ptr::null_mut());
    assert!(!md.is_null(), "{}: match_data_create failed", api.tag);
    (api.pcre2_match_8)(code, DUMMY.as_ptr(), 0, 0, 0, md, ptr::null_mut());
    let n = (api.pcre2_get_ovector_count_8)(md) as usize;
    let ov = (api.pcre2_get_ovector_pointer_8)(md);
    for i in 0..n * 2 {
        *ov.add(i) = PCRE2_UNSET;
    }
    md
}

unsafe fn md_probe(api: &Api, md: Ptr) -> MdProbe {
    let mut len: usize = 0xdead_beef;
    let lenrc = (api.pcre2_substring_length_bynumber_8)(md, 0, &mut len);
    let n = (api.pcre2_get_ovector_count_8)(md) as usize;
    let ov = (api.pcre2_get_ovector_pointer_8)(md);
    MdProbe {
        lenrc,
        len: if lenrc == 0 { len } else { 0xdead_beef },
        ov: std::slice::from_raw_parts(ov, n * 2).to_vec(),
    }
}

/// Run one `pcre2_substitute` on one side, capturing everything observable.
unsafe fn call_side(
    api: &Api,
    code: Ptr,
    mc: Ptr,
    a: &SubArgs,
    mdm: MdMode,
    sc_pol: u32,
    cc_pol: u32,
) -> (SubOut, Vec<ScLog>, Vec<CcLog>) {
    let md = match mdm {
        MdMode::Null => ptr::null_mut(),
        MdMode::Fresh => fresh_md(api, code),
        MdMode::Matched => {
            let md = fresh_md(api, code);
            (api.pcre2_match_8)(
                code,
                a.subj,
                a.slen,
                a.start,
                a.options & !SUBSTITUTE_OPTIONS,
                md,
                mc,
            );
            md
        }
    };

    let mut buf = vec![POISON; a.buflen + TAIL];
    let bufptr = if a.null_buf {
        ptr::null_mut()
    } else {
        buf.as_mut_ptr()
    };
    let mut outlen = a.declared.unwrap_or(a.buflen);

    CB_SUBJ.with(|c| c.set(a.subj));
    CB_BUF.with(|c| c.set(bufptr));
    CB_BUFLEN.with(|c| c.set(a.buflen));
    SC_POLICY.with(|c| c.set(sc_pol));
    CC_POLICY.with(|c| c.set(cc_pol));
    SC_LOG.with(|l| l.borrow_mut().clear());
    CC_LOG.with(|l| l.borrow_mut().clear());

    let rc = (api.pcre2_substitute_8)(
        code, a.subj, a.slen, a.start, a.options, md, mc, a.rep, a.rlen, bufptr, &mut outlen,
    );

    let mdp = if md.is_null() {
        None
    } else {
        Some(md_probe(api, md))
    };
    if !md.is_null() {
        (api.pcre2_match_data_free_8)(md);
    }
    (
        SubOut {
            rc,
            outlen,
            buf,
            md: mdp,
        },
        SC_LOG.with(|l| l.borrow().clone()),
        CC_LOG.with(|l| l.borrow().clone()),
    )
}

/// A (pattern, match-context) pair: the same configuration in both libraries.
struct Pair<'a> {
    c: &'a Api,
    r: &'a Api,
    code_c: Ptr,
    code_r: Ptr,
    mc_c: Ptr,
    mc_r: Ptr,
    sc_pol: u32,
    cc_pol: u32,
}

impl Pair<'_> {
    /// One differential substitute call. Compares rc, *blength, the WHOLE output
    /// buffer (including the poison tail), the caller's match data and every
    /// callout invocation.
    unsafe fn one(&self, a: &SubArgs, mdm: MdMode, label: &str) -> SubOut {
        if std::env::var_os("SUBST_TRACE").is_some() {
            eprintln!(
                "TRACE {} opts={:#x} start={} slen={} rlen={} buflen={} nullbuf={} subjnull={} repnull={} md={}",
                label,
                a.options,
                a.start,
                a.slen as i64,
                a.rlen as i64,
                a.buflen,
                a.null_buf,
                a.subj.is_null(),
                a.rep.is_null(),
                mdm == MdMode::Null
            );
        }
        let (oc, scc, ccc) = call_side(
            self.c,
            self.code_c,
            self.mc_c,
            a,
            mdm,
            self.sc_pol,
            self.cc_pol,
        );
        let (or, scr, ccr) = call_side(
            self.r,
            self.code_r,
            self.mc_r,
            a,
            mdm,
            self.sc_pol,
            self.cc_pol,
        );
        assert_eq!(
            oc.rc, or.rc,
            "{} : rc differs (buflen={} start={} opts={:#x})",
            label, a.buflen, a.start, a.options
        );
        assert_eq!(
            oc.outlen, or.outlen,
            "{} : *blength differs (rc={} buflen={} opts={:#x})",
            label, oc.rc, a.buflen, a.options
        );
        assert_eq!(
            oc.buf, or.buf,
            "{} : output buffer differs (rc={} buflen={} opts={:#x})",
            label, oc.rc, a.buflen, a.options
        );
        assert_eq!(
            oc.md, or.md,
            "{} : caller match_data differs (rc={} buflen={})",
            label, oc.rc, a.buflen
        );
        assert_eq!(
            scc, scr,
            "{} : substitute callout log differs (rc={} buflen={})",
            label, oc.rc, a.buflen
        );
        assert_eq!(
            ccc, ccr,
            "{} : substitute CASE callout log differs (rc={} buflen={})",
            label, oc.rc, a.buflen
        );
        RC_SEEN.with(|s| s.borrow_mut().insert(oc.rc));
        oc
    }

    /// Sweep the output buffer length from 0 to `need+4` (capped), comparing
    /// rc AND *blength AND the buffer for EVERY size.
    unsafe fn sweep(&self, a: &SubArgs, mdm: MdMode, label: &str, cap: usize, n: &mut usize) {
        let mut probe = *a;
        probe.buflen = 512;
        let big = self.one(&probe, mdm, label);
        *n += 1;
        // `need_known` is true when the probe told us the exact required length,
        // which is what makes the "huge declared length" cases below safe.
        let (need, need_known) = if big.rc >= 0 {
            (big.outlen + 1, true)
        } else if big.outlen != PCRE2_UNSET && big.outlen < 512 {
            (big.outlen + 1, true)
        } else {
            (4, false)
        };
        let top = (need + 4).min(cap);
        for bl in 0..=top {
            let mut aa = *a;
            aa.buflen = bl;
            self.one(&aa, mdm, label);
            *n += 1;
        }
        // and a NULL buffer, which the C tolerates when nothing is ever written
        let mut aa = *a;
        aa.buflen = 0;
        aa.null_buf = true;
        aa.options |= PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
        self.one(&aa, mdm, label);
        *n += 1;
        // and *blength == PCRE2_ZERO_TERMINATED (and SIZE_MAX/2) on entry, i.e. a
        // "huge" declared buffer. Only done when the probe established the exact
        // required length, so that the real 4096-byte allocation cannot overflow.
        if need_known && need + 16 < 4096 {
            for d in [PCRE2_ZERO_TERMINATED, usize::MAX / 2] {
                let mut aa = *a;
                aa.buflen = 4096;
                aa.declared = Some(d);
                self.one(&aa, mdm, label);
                *n += 1;
            }
        }
    }
}

// ======================================================= compile helpers

unsafe fn compile2(c: &Api, r: &Api, pat: &[u8], opts: u32) -> Option<(Ptr, Ptr)> {
    compile2_ctx(c, r, pat, opts, ptr::null_mut(), ptr::null_mut())
}

unsafe fn compile2_ctx(
    c: &Api,
    r: &Api,
    pat: &[u8],
    opts: u32,
    cx_c: Ptr,
    cx_r: Ptr,
) -> Option<(Ptr, Ptr)> {
    let mut ec1: i32 = 0;
    let mut eo1: usize = 0;
    let cc = (c.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut ec1, &mut eo1, cx_c);
    let mut ec2: i32 = 0;
    let mut eo2: usize = 0;
    let rr = (r.pcre2_compile_8)(pat.as_ptr(), pat.len(), opts, &mut ec2, &mut eo2, cx_r);
    assert_eq!(
        (cc.is_null(), ec1, eo1),
        (rr.is_null(), ec2, eo2),
        "compile differs: pat={:?} opts={:#x}",
        String::from_utf8_lossy(pat),
        opts
    );
    if cc.is_null() {
        None
    } else {
        Some((cc, rr))
    }
}

unsafe fn free_pair(c: &Api, r: &Api, cc: Ptr, rr: Ptr) {
    (c.pcre2_code_free_8)(cc);
    (r.pcre2_code_free_8)(rr);
}

unsafe fn valid_utf(api: &Api, s: &[u8]) -> bool {
    let mut off: usize = 0;
    (api._pcre2_valid_utf_8)(s.as_ptr(), s.len(), &mut off) == 0
}

// ======================================================= C170

#[test]
fn c170_substitute_null_zeroterm_and_offsets() {
    println!("C170 pcre2_substitute: NULL/ZERO_TERMINATED/BADOFFSET pre-match checks");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        // subjects that are NUL-terminated so PCRE2_ZERO_TERMINATED is meaningful
        let subjects: [&[u8]; 5] = [
            b"\0",
            b"abcabc\0",
            b"aXbXc\0",
            b"ab\0cd\0",
            b"the quick brown fox\0",
        ];
        let reps: [&[u8]; 5] = [b"", b"Z", b"[$0]", b"a\0b", b"$1-$0"];
        // NUL-terminated copies, for the PCRE2_ZERO_TERMINATED conversions
        let reps_z: Vec<Vec<u8>> = reps
            .iter()
            .map(|s| {
                let mut v = s.to_vec();
                v.push(0);
                v
            })
            .collect();
        for pat in [
            &b"b"[..],
            &b"(a)(b)?"[..],
            &b"a*"[..],
            &b"(?<nm>X)"[..],
            &b"\0"[..],
        ] {
            let Some((cc, rr)) = compile2(c, r, pat, 0) else {
                continue;
            };
            let p = Pair {
                c,
                r,
                code_c: cc,
                code_r: rr,
                mc_c: ptr::null_mut(),
                mc_r: ptr::null_mut(),
                sc_pol: 0,
                cc_pol: 0,
            };
            for subj in subjects {
                let real = subj.len() - 1; // without the terminating NUL
                for (ri, rep) in reps.into_iter().enumerate() {
                    let repz: &[u8] = &reps_z[ri];
                    for gl in [0, PCRE2_SUBSTITUTE_GLOBAL] {
                        for mdm in [MdMode::Null, MdMode::Fresh] {
                            // ---- ordinary lengths, every interesting start offset
                            for &st in &[0usize, real / 2, real, real + 1, real + 9] {
                                let mut a = args(subj, rep);
                                a.slen = real;
                                a.start = st;
                                a.options = gl;
                                p.sweep(&a, mdm, "C170/offset", 14, &mut n);
                            }
                            // ---- subject length PCRE2_ZERO_TERMINATED
                            let mut a = args(subj, rep);
                            a.slen = PCRE2_ZERO_TERMINATED;
                            a.options = gl;
                            p.sweep(&a, mdm, "C170/slen=ZT", 14, &mut n);

                            // ---- replacement length PCRE2_ZERO_TERMINATED
                            let mut a = args(subj, repz);
                            a.slen = real;
                            a.rlen = PCRE2_ZERO_TERMINATED;
                            a.options = gl;
                            p.sweep(&a, mdm, "C170/rlen=ZT", 14, &mut n);

                            // ---- replacement NULL, rlength 0 (legal) and non-zero
                            for rl in [0usize, 1, 5, PCRE2_ZERO_TERMINATED] {
                                let mut a = args(subj, rep);
                                a.slen = real;
                                a.rep = ptr::null();
                                a.rlen = rl;
                                a.options = gl;
                                p.sweep(&a, mdm, "C170/rep=NULL", 10, &mut n);
                            }

                            // ---- subject NULL, length 0 (legal) and non-zero
                            for sl in [0usize, 1, 7] {
                                let mut a = args(subj, rep);
                                a.subj = ptr::null();
                                a.slen = sl;
                                a.options = gl;
                                p.sweep(&a, mdm, "C170/subj=NULL", 10, &mut n);
                            }
                        }
                    }
                }
            }
            free_pair(c, r, cc, rr);
        }
    }
    println!("C170: {} substitute comparisons; distinct rc values seen: {:?}", n, rc_seen());
}

// ======================================================= C171

#[test]
fn c171_substitute_literal() {
    println!("C171 pcre2_substitute: PCRE2_SUBSTITUTE_LITERAL on and off");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let reps: [&[u8]; 16] = [
            b"$",
            b"$1",
            b"$$",
            b"\\",
            b"\\U",
            b"\\Q",
            b"${1}",
            b"$0$1$2",
            b"\\U$1\\E",
            b"${1:-x}",
            b"\\u\\Labc",
            b"$&",
            b"$`",
            b"a$b\\c",
            b"",
            b"\\n\\t\\x41",
        ];
        for pat in [&b"(b)"[..], &b"(a)(b)"[..], &b"b+"[..]] {
            let Some((cc, rr)) = compile2(c, r, pat, 0) else {
                continue;
            };
            let p = Pair {
                c,
                r,
                code_c: cc,
                code_r: rr,
                mc_c: ptr::null_mut(),
                mc_r: ptr::null_mut(),
                sc_pol: 0,
                cc_pol: 0,
            };
            for subj in [&b"abcabc"[..], &b"b"[..], &b""[..], &b"xxbbxx"[..]] {
                for rep in reps {
                    for lit in [0, PCRE2_SUBSTITUTE_LITERAL] {
                        for ext in [0, PCRE2_SUBSTITUTE_EXTENDED] {
                            for gl in [0, PCRE2_SUBSTITUTE_GLOBAL] {
                                let mut a = args(subj, rep);
                                a.options = lit | ext | gl;
                                p.sweep(&a, MdMode::Null, "C171/literal", 14, &mut n);
                            }
                        }
                    }
                }
            }
            free_pair(c, r, cc, rr);
        }
    }
    println!("C171: {} substitute comparisons; distinct rc values seen: {:?}", n, rc_seen());
}

// ======================================================= C172

#[test]
fn c172_substitute_global() {
    println!("C172 pcre2_substitute: PCRE2_SUBSTITUTE_GLOBAL and the bumpalong");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let pats: [(&[u8], u32); 12] = [
            (b"a", 0),
            (b"a*", 0),
            (b"\\b", 0),
            (b"(?=x)", 0),
            (b"", 0),
            (b"x?", 0),
            (b"(?=(a))b", 0),
            (b"(?<=a)\\Kb", 0),
            (b"a\\Kb", 0),
            (b"(?:(?=(x))\\K)?y", 0),
            (b".", PCRE2_UTF),
            (b"\\X", PCRE2_UTF | PCRE2_UCP),
        ];
        let subjects: [&[u8]; 9] = [
            b"",
            b"a",
            b"aaa",
            b"abcabcabc",
            b"a\r\nb\r\nc",
            b"x\ny\nz",
            b"\xc3\xa9\xc3\xa8\xc3\xa7",
            b"a\xe2\x82\xacb",
            b"\0a\0a\0",
        ];
        for (pat, popt) in pats {
            for nl in [
                0,
                PCRE2_NEWLINE_CR,
                PCRE2_NEWLINE_LF,
                PCRE2_NEWLINE_CRLF,
                PCRE2_NEWLINE_ANY,
                PCRE2_NEWLINE_ANYCRLF,
                PCRE2_NEWLINE_NUL,
            ] {
                let cx_c = (c.pcre2_compile_context_create_8)(ptr::null_mut());
                let cx_r = (r.pcre2_compile_context_create_8)(ptr::null_mut());
                if nl != 0 {
                    assert_eq!(
                        (c.pcre2_set_newline_8)(cx_c, nl),
                        (r.pcre2_set_newline_8)(cx_r, nl)
                    );
                }
                let compiled = compile2_ctx(c, r, pat, popt, cx_c, cx_r);
                (c.pcre2_compile_context_free_8)(cx_c);
                (r.pcre2_compile_context_free_8)(cx_r);
                let Some((cc, rr)) = compiled else { continue };
                let p = Pair {
                    c,
                    r,
                    code_c: cc,
                    code_r: rr,
                    mc_c: ptr::null_mut(),
                    mc_r: ptr::null_mut(),
                    sc_pol: 0,
                    cc_pol: 0,
                };
                for subj in subjects {
                    if popt & PCRE2_UTF != 0 && !valid_utf(c, subj) {
                        // still a valid differential case (both must report the
                        // same UTF error), but do not set NO_UTF_CHECK
                    }
                    for rep in [&b"-"[..], &b""[..], &b"[$0]"[..], &b"$0$0"[..]] {
                        for gl in [0, PCRE2_SUBSTITUTE_GLOBAL] {
                            for st in [0usize, 1] {
                                if st > subj.len() {
                                    continue;
                                }
                                let mut a = args(subj, rep);
                                a.start = st;
                                a.options = gl;
                                p.sweep(&a, MdMode::Fresh, "C172/global", 16, &mut n);
                            }
                        }
                    }
                }
                free_pair(c, r, cc, rr);
            }
        }
    }
    println!("C172: {} substitute comparisons; distinct rc values seen: {:?}", n, rc_seen());
}

// ======================================================= C173

#[test]
fn c173_substitute_dollar_forms() {
    println!("C173 pcre2_substitute: every $ replacement form");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let reps: [&[u8]; 40] = [
            b"$$",
            b"$&",
            b"$`",
            b"$'",
            b"$_",
            b"$+",
            b"$+{nm}",
            b"$+{nosuch}",
            b"${1}",
            b"${nm}",
            b"$<nm>",
            b"$<nosuch>",
            b"${*MARK}",
            b"$*MARK",
            b"${*NOPE}",
            b"$*NOPE",
            b"$12",
            b"$0",
            b"$1",
            b"$9",
            b"$99",
            b"$",
            b"${1",
            b"${",
            b"$<1",
            b"$<*",
            b"$<",
            b"${}",
            b"$<>",
            b"${*}",
            b"$nm",
            b"$nosuch",
            b"x$1y$2z",
            b"$&$&",
            b"${nm}${nm}",
            b"$+$+",
            b"a$",
            b"$1$",
            b"${*MARK}$0",
            b"$'$`",
        ];
        let pats: [&[u8]; 6] = [
            b"(a)(b)",
            b"(?<nm>a)(b)?",
            b"x",
            b"(*MARK:MK)(a)",
            b"(?<nm>a)(?<nm2>b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)(m)",
            b"(*MARK:M\0K)a",
        ];
        for pat in pats {
            let Some((cc, rr)) = compile2(c, r, pat, 0) else {
                continue;
            };
            let p = Pair {
                c,
                r,
                code_c: cc,
                code_r: rr,
                mc_c: ptr::null_mut(),
                mc_r: ptr::null_mut(),
                sc_pol: 0,
                cc_pol: 0,
            };
            for subj in [&b"zzabzz"[..], &b"a"[..], &b"x"[..], &b""[..]] {
                for rep in reps {
                    for ext in [0, PCRE2_SUBSTITUTE_EXTENDED] {
                        for un in [
                            0,
                            PCRE2_SUBSTITUTE_UNSET_EMPTY,
                            PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                            PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                        ] {
                            let mut a = args(subj, rep);
                            a.options = ext | un;
                            a.buflen = 64;
                            p.one(&a, MdMode::Fresh, "C173/dollar-big");
                            n += 1;
                            // a couple of tight buffers per case (the full sweep
                            // for every one of these would be needlessly slow)
                            for bl in [0usize, 1, 2, 3, 5, 8] {
                                let mut a = args(subj, rep);
                                a.options = ext | un;
                                a.buflen = bl;
                                p.one(&a, MdMode::Fresh, "C173/dollar-tight");
                                n += 1;
                                let mut a2 = a;
                                a2.options |= PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
                                p.one(&a2, MdMode::Fresh, "C173/dollar-tight-ovl");
                                n += 1;
                            }
                        }
                    }
                    // partial matching rejects $' and $_
                    let mut a = args(subj, rep);
                    a.options = PCRE2_SUBSTITUTE_REPLACEMENT_ONLY | PCRE2_PARTIAL_SOFT;
                    p.one(&a, MdMode::Fresh, "C173/dollar-partial");
                    n += 1;
                }
            }
            free_pair(c, r, cc, rr);
        }
    }
    println!("C173: {} substitute comparisons; distinct rc values seen: {:?}", n, rc_seen());
}

// ======================================================= C174

#[test]
fn c174_substitute_extended_braces() {
    println!("C174 pcre2_substitute: EXTENDED ${{name:-default}} / ${{name:+a:b}}");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let deep = {
            // nesting depth beyond PTR_STACK_SIZE = 20
            let mut v: Vec<u8> = Vec::new();
            for _ in 0..25 {
                v.extend_from_slice(b"${1:+");
            }
            v.extend_from_slice(b"x");
            for _ in 0..25 {
                v.extend_from_slice(b"}");
            }
            v
        };
        let reps: Vec<&[u8]> = vec![
            b"${1:-default}",
            b"${1:+set:unset}",
            b"${1:-}",
            b"${1:+}",
            b"${1:+:}",
            b"${1:?x}",
            b"${1:-x",
            b"${1:+a",
            b"${1:+a:b",
            b"${1:+${2:-x}:y}",
            b"${1:-$2}",
            b"${1:-${2:-${3:-z}}}",
            b"${nm:-d}",
            b"${nm:+y:n}",
            b"${nosuch:-d}",
            b"${nosuch:+y:n}",
            b"${1:-\\Q:\\E}",
            b"${1:-\\g<nm>}",
            b"${1:-\\g{2}}",
            b"${1:-\\z}",
            b"${1:-\\U}",
            b"${1:-\\L}",
            b"${1:-\\u}",
            b"${1:-\\l}",
            b"${1:-\\E}",
            b"${1:-\\}",
            b"${1:-a:b}",
            b"${*MARK:-d}",
            b"${1:-x}y",
            b"a${1:+p:q}b",
            b"${99:-d}",
            b"${1:",
            b"${1:-",
            b"${1:+",
            &deep,
        ];
        let pats: [&[u8]; 4] = [b"(a)(b)?", b"(?<nm>a)?(b)", b"(*MARK:MK)(a)", b"(a)"];
        for pat in pats {
            let Some((cc, rr)) = compile2(c, r, pat, 0) else {
                continue;
            };
            let p = Pair {
                c,
                r,
                code_c: cc,
                code_r: rr,
                mc_c: ptr::null_mut(),
                mc_r: ptr::null_mut(),
                sc_pol: 0,
                cc_pol: 0,
            };
            for subj in [&b"ab"[..], &b"b"[..], &b"a"[..], &b""[..]] {
                for rep in reps.iter() {
                    for ext in [0, PCRE2_SUBSTITUTE_EXTENDED] {
                        for un in [
                            0,
                            PCRE2_SUBSTITUTE_UNSET_EMPTY,
                            PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                        ] {
                            let mut a = args(subj, rep);
                            a.options = ext | un;
                            p.sweep(&a, MdMode::Fresh, "C174/extended", 14, &mut n);
                        }
                    }
                }
            }
            free_pair(c, r, cc, rr);
        }
    }
    println!("C174: {} substitute comparisons; distinct rc values seen: {:?}", n, rc_seen());
}

// ======================================================= C175

#[test]
fn c175_substitute_backslash_escapes() {
    println!("C175 pcre2_substitute: EXTENDED backslash replacements");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let reps: [&[u8]; 44] = [
            b"\\L",
            b"\\U",
            b"\\l",
            b"\\u",
            b"\\E",
            b"\\Q",
            b"\\l\\U",
            b"\\u\\L",
            b"\\l\\Ux",
            b"\\u\\Lx",
            b"\\u",
            b"x\\u",
            b"\\n",
            b"\\t",
            b"\\b",
            b"\\v",
            b"\\r",
            b"\\f",
            b"\\a",
            b"\\e",
            b"\\0",
            b"\\x41",
            b"\\x{100}",
            b"\\x{",
            b"\\o{101}",
            b"\\o{",
            b"\\1",
            b"\\2",
            b"\\9",
            b"\\g{2}",
            b"\\g{nm}",
            b"\\g<nm>",
            b"\\g<1>",
            b"\\g",
            b"\\g<nm",
            b"\\g<",
            b"\\g<>",
            b"\\z",
            b"\\A",
            b"\\Z",
            b"\\c",
            b"\\cA",
            b"\\Qab\\Ecd",
            b"\\\\",
        ];
        let pats: [(&[u8], u32); 4] = [
            (b"(a)(b)?", 0),
            (b"(?<nm>a)(b)", 0),
            (b"(a)", PCRE2_UTF),
            (b"(?<nm>.)", PCRE2_UTF | PCRE2_UCP),
        ];
        for (pat, popt) in pats {
            let Some((cc, rr)) = compile2(c, r, pat, popt) else {
                continue;
            };
            let p = Pair {
                c,
                r,
                code_c: cc,
                code_r: rr,
                mc_c: ptr::null_mut(),
                mc_r: ptr::null_mut(),
                sc_pol: 0,
                cc_pol: 0,
            };
            for subj in [&b"ab"[..], &b"a"[..], &b"AB"[..], &b""[..]] {
                for rep in reps {
                    for ext in [0, PCRE2_SUBSTITUTE_EXTENDED] {
                        for gl in [0, PCRE2_SUBSTITUTE_GLOBAL] {
                            let mut a = args(subj, rep);
                            a.options = ext | gl;
                            p.sweep(&a, MdMode::Fresh, "C175/backslash", 12, &mut n);
                        }
                    }
                }
            }
            free_pair(c, r, cc, rr);
        }
    }
    println!("C175: {} substitute comparisons; distinct rc values seen: {:?}", n, rc_seen());
}

// ======================================================= C176

#[test]
fn c176_substitute_case_forcing_default() {
    println!("C176 pcre2_substitute: built-in case forcing (no case callout)");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let reps: [&[u8]; 24] = [
            b"\\Uabc",
            b"\\Labc",
            b"\\uabc",
            b"\\labc",
            b"\\u$1",
            b"\\l$1",
            b"\\U$1\\Eabc",
            b"\\l\\Uabc",
            b"\\u\\Labc",
            b"\\u$2",
            b"\\U$1$2\\E-$1",
            b"\\u\\U$1",
            b"\\l\\L$1",
            b"\\U\\x{df}",
            b"\\U\\x{130}",
            b"\\U\\x{131}",
            b"\\Ui",
            b"\\Li",
            b"\\U\\xe9",
            b"\\L\\xc9",
            b"\\U$0\\Q$0",
            b"\\Uab\\Qcd",
            b"\\u",
            b"\\U",
        ];
        let pats: [(&[u8], u32, u32); 8] = [
            (b"(a)(b)?", 0, 0),
            (b"(a)(b)?", PCRE2_UCP, 0),
            (b"(.)(.)?", PCRE2_UTF, 0),
            (b"(.)(.)?", PCRE2_UTF | PCRE2_UCP, 0),
            (b"(\\w)(\\w)?", PCRE2_UTF | PCRE2_UCP, PCRE2_EXTRA_TURKISH_CASING),
            (b"(\\w)(\\w)?", 0, 0),
            (b"()(x)?", 0, 0),
            (b"(.)", PCRE2_UTF, PCRE2_EXTRA_TURKISH_CASING),
        ];
        for (pat, popt, xopt) in pats {
            let cx_c = (c.pcre2_compile_context_create_8)(ptr::null_mut());
            let cx_r = (r.pcre2_compile_context_create_8)(ptr::null_mut());
            assert_eq!(
                (c.pcre2_set_compile_extra_options_8)(cx_c, xopt),
                (r.pcre2_set_compile_extra_options_8)(cx_r, xopt)
            );
            let compiled = compile2_ctx(c, r, pat, popt, cx_c, cx_r);
            (c.pcre2_compile_context_free_8)(cx_c);
            (r.pcre2_compile_context_free_8)(cx_r);
            let Some((cc, rr)) = compiled else { continue };
            let p = Pair {
                c,
                r,
                code_c: cc,
                code_r: rr,
                mc_c: ptr::null_mut(),
                mc_r: ptr::null_mut(),
                sc_pol: 0,
                cc_pol: 0,
            };
            let subjects: [&[u8]; 8] = [
                b"ab",
                b"AB",
                b"a",
                b"",
                b"\xc3\xa9\xc3\x89",
                b"\xc3\x9f",
                b"i\xc4\xb1",
                b"\xff\xfe",
            ];
            for subj in subjects {
                if popt & PCRE2_UTF != 0 && !valid_utf(c, subj) {
                    // both libraries must report the identical UTF error
                }
                for rep in reps {
                    for ext in [0, PCRE2_SUBSTITUTE_EXTENDED] {
                        for gl in [0, PCRE2_SUBSTITUTE_GLOBAL] {
                            let mut a = args(subj, rep);
                            a.options = ext | gl;
                            p.sweep(&a, MdMode::Fresh, "C176/forcecase", 14, &mut n);
                        }
                    }
                }
            }
            free_pair(c, r, cc, rr);
        }
    }
    println!("C176: {} substitute comparisons; distinct rc values seen: {:?}", n, rc_seen());
}

// ======================================================= C177

#[test]
fn c177_substitute_case_callout() {
    println!("C177 pcre2_substitute: pcre2_set_substitute_case_callout");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let reps: [&[u8]; 14] = [
            b"\\Uabc",
            b"\\Labc",
            b"\\uabc",
            b"\\labc",
            b"\\u$1",
            b"\\l$1",
            b"\\U$1\\Eabc",
            b"\\l\\Uabcdef",
            b"\\u\\Labcdef",
            b"\\U$1$2",
            b"x\\Uy\\Lz",
            b"\\U\\x{100}b",
            b"\\u",
            b"\\l\\U",
        ];
        for (pat, popt) in [(&b"(a)(b)?"[..], 0u32), (&b"(.)(.)?"[..], PCRE2_UTF)] {
            let Some((cc, rr)) = compile2(c, r, pat, popt) else {
                continue;
            };
            for data_null in [true, false] {
                let mc_c = (c.pcre2_match_context_create_8)(ptr::null_mut());
                let mc_r = (r.pcre2_match_context_create_8)(ptr::null_mut());
                let dp = if data_null {
                    ptr::null_mut()
                } else {
                    sentinel_ptr()
                };
                assert_eq!(
                    (c.pcre2_set_substitute_case_callout_8)(mc_c, Some(cc_cb), dp),
                    (r.pcre2_set_substitute_case_callout_8)(mc_r, Some(cc_cb), dp)
                );
                for pol in 0..6u32 {
                    let p = Pair {
                        c,
                        r,
                        code_c: cc,
                        code_r: rr,
                        mc_c,
                        mc_r,
                        sc_pol: 0,
                        cc_pol: pol,
                    };
                    for subj in [&b"ab"[..], &b"AB"[..], &b"a"[..], &b"\xc3\xa9x"[..]] {
                        for rep in reps {
                            for gl in [0, PCRE2_SUBSTITUTE_GLOBAL] {
                                let mut a = args(subj, rep);
                                a.options = PCRE2_SUBSTITUTE_EXTENDED | gl;
                                // generous / exact / too small, with and without
                                // OVERFLOW_LENGTH — the sweep covers all of them
                                p.sweep(&a, MdMode::Fresh, "C177/case-callout", 16, &mut n);
                                let mut a2 = a;
                                a2.options |= PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
                                p.sweep(&a2, MdMode::Fresh, "C177/case-callout-ovl", 16, &mut n);
                            }
                        }
                    }
                }
                (c.pcre2_match_context_free_8)(mc_c);
                (r.pcre2_match_context_free_8)(mc_r);
            }
            free_pair(c, r, cc, rr);
        }
    }
    let calls = CC_CALLS.with(|x| x.get());
    println!("C177: {} substitute comparisons, {} callout invocations; distinct rc values seen: {:?}", n, calls, rc_seen());
    assert!(calls > 1000, "the callout only fired {} times", calls);
}

// ======================================================= C178

#[test]
fn c178_substitute_callout() {
    println!("C178 pcre2_substitute: pcre2_set_substitute_callout");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        for (pat, popt) in [
            (&b"a"[..], 0u32),
            (&b"(a)(b)?"[..], 0),
            (&b"a*"[..], 0),
            (&b"(?<nm>x)"[..], 0),
        ] {
            let Some((cc, rr)) = compile2(c, r, pat, popt) else {
                continue;
            };
            for data_null in [true, false] {
                let mc_c = (c.pcre2_match_context_create_8)(ptr::null_mut());
                let mc_r = (r.pcre2_match_context_create_8)(ptr::null_mut());
                let dp = if data_null {
                    ptr::null_mut()
                } else {
                    sentinel_ptr()
                };
                assert_eq!(
                    (c.pcre2_set_substitute_callout_8)(mc_c, Some(sc_cb), dp),
                    (r.pcre2_set_substitute_callout_8)(mc_r, Some(sc_cb), dp)
                );
                for pol in 0..6u32 {
                    let p = Pair {
                        c,
                        r,
                        code_c: cc,
                        code_r: rr,
                        mc_c,
                        mc_r,
                        sc_pol: pol,
                        cc_pol: 0,
                    };
                    for subj in [
                        &b"a"[..],
                        &b"aaa"[..],
                        &b"xaxaxax"[..],
                        &b"zzz"[..],
                        &b""[..],
                    ] {
                        for rep in [&b"-"[..], &b"[$0]"[..], &b""[..], &b"$1$1$1"[..]] {
                            for gl in [0, PCRE2_SUBSTITUTE_GLOBAL] {
                                for ro in [0, PCRE2_SUBSTITUTE_REPLACEMENT_ONLY] {
                                    let mut a = args(subj, rep);
                                    a.options = gl | ro;
                                    p.sweep(&a, MdMode::Fresh, "C178/callout", 16, &mut n);
                                    let mut a2 = a;
                                    a2.options |= PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
                                    p.sweep(&a2, MdMode::Fresh, "C178/callout-ovl", 16, &mut n);
                                }
                            }
                        }
                    }
                }
                (c.pcre2_match_context_free_8)(mc_c);
                (r.pcre2_match_context_free_8)(mc_r);
            }
            free_pair(c, r, cc, rr);
        }
    }
    let calls = SC_CALLS.with(|x| x.get());
    println!("C178: {} substitute comparisons, {} callout invocations; distinct rc values seen: {:?}", n, calls, rc_seen());
    assert!(calls > 1000, "the callout only fired {} times", calls);
}

// ======================================================= C179

#[test]
fn c179_substitute_overflow_length() {
    println!("C179 pcre2_substitute: PCRE2_SUBSTITUTE_OVERFLOW_LENGTH bookkeeping");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let reps: [&[u8]; 12] = [
            b"-",
            b"[$0]",
            b"$0$0$0$0",
            b"\\Uabcdefgh",
            b"ab\\Ucdefgh\\Eij",
            b"\\u$0$0",
            b"\\U$0\\L$0",
            b"$1$1$1$1$1$1",
            b"xxxxxxxxxxxxxxxxxxxx",
            b"",
            b"\\l\\Uabcdefgh",
            b"a\\Ub\\Lc\\Ed",
        ];
        for (pat, popt) in [(&b"(a)"[..], 0u32), (&b"(.)"[..], PCRE2_UTF), (&b"a+"[..], 0)] {
            let Some((cc, rr)) = compile2(c, r, pat, popt) else {
                continue;
            };
            // once with no case callout (immediate transformation) and once with
            // one (delayed transformation via do_case_copy)
            for use_cc in [false, true] {
                let mc_c = (c.pcre2_match_context_create_8)(ptr::null_mut());
                let mc_r = (r.pcre2_match_context_create_8)(ptr::null_mut());
                if use_cc {
                    (c.pcre2_set_substitute_case_callout_8)(mc_c, Some(cc_cb), sentinel_ptr());
                    (r.pcre2_set_substitute_case_callout_8)(mc_r, Some(cc_cb), sentinel_ptr());
                }
                for pol in if use_cc { 0..6u32 } else { 0..1u32 } {
                    let p = Pair {
                        c,
                        r,
                        code_c: cc,
                        code_r: rr,
                        mc_c,
                        mc_r,
                        sc_pol: 0,
                        cc_pol: pol,
                    };
                    for subj in [&b"aaa"[..], &b"xaaay"[..], &b"a"[..], &b""[..]] {
                        for rep in reps {
                            for ovl in [0, PCRE2_SUBSTITUTE_OVERFLOW_LENGTH] {
                                for ro in [0, PCRE2_SUBSTITUTE_REPLACEMENT_ONLY] {
                                    for st in [0usize, 1] {
                                        if st > subj.len() {
                                            continue;
                                        }
                                        let mut a = args(subj, rep);
                                        a.start = st;
                                        a.options = PCRE2_SUBSTITUTE_EXTENDED
                                            | PCRE2_SUBSTITUTE_GLOBAL
                                            | ovl
                                            | ro;
                                        // exhaustive size sweep 0..=need+4
                                        p.sweep(&a, MdMode::Fresh, "C179/overflow", 40, &mut n);
                                    }
                                }
                            }
                        }
                    }
                }
                (c.pcre2_match_context_free_8)(mc_c);
                (r.pcre2_match_context_free_8)(mc_r);
            }
            free_pair(c, r, cc, rr);
        }
    }
    let calls = CC_CALLS.with(|x| x.get());
    println!("C179: {} substitute comparisons, {} callout invocations; distinct rc values seen: {:?}", n, calls, rc_seen());
    assert!(calls > 1000, "the callout only fired {} times", calls);
}

// ======================================================= C180

#[test]
fn c180_substitute_unset_and_unknown() {
    println!("C180 pcre2_substitute: UNSET_EMPTY / UNKNOWN_UNSET");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let reps: [&[u8]; 14] = [
            b"$1",
            b"${1}",
            b"${1:-d}",
            b"${1:+y:n}",
            b"$2",
            b"${2:-d}",
            b"$9",
            b"${nosuch}",
            b"$+",
            b"$+{nosuch}",
            b"$+{nm}",
            b"${nosuch:-d}",
            b"${nosuch:+y:n}",
            b"[$1|$2]",
        ];
        let pats: [&[u8]; 5] = [b"(a)|(b)", b"(a)", b"x", b"(?<nm>a)|(?<nm2>b)", b"(a)(b)(c)"];
        for pat in pats {
            let Some((cc, rr)) = compile2(c, r, pat, 0) else {
                continue;
            };
            let p = Pair {
                c,
                r,
                code_c: cc,
                code_r: rr,
                mc_c: ptr::null_mut(),
                mc_r: ptr::null_mut(),
                sc_pol: 0,
                cc_pol: 0,
            };
            for subj in [&b"b"[..], &b"a"[..], &b"x"[..], &b"abc"[..]] {
                for rep in reps {
                    for ue in [0, PCRE2_SUBSTITUTE_UNSET_EMPTY] {
                        for uu in [0, PCRE2_SUBSTITUTE_UNKNOWN_UNSET] {
                            for ext in [0, PCRE2_SUBSTITUTE_EXTENDED] {
                                let mut a = args(subj, rep);
                                a.options = ue | uu | ext;
                                p.sweep(&a, MdMode::Fresh, "C180/unset", 12, &mut n);
                            }
                        }
                    }
                }
            }
            // a match_data with too small an ovector: $+ must give UNAVAILABLE
            for oc in [1u32, 2, 3] {
                for rep in reps {
                    for uu in [0, PCRE2_SUBSTITUTE_UNKNOWN_UNSET] {
                        let subj: &[u8] = b"abc";
                        let mdc = (c.pcre2_match_data_create_8)(oc, ptr::null_mut());
                        let mdr = (r.pcre2_match_data_create_8)(oc, ptr::null_mut());
                        let ovc = (c.pcre2_get_ovector_pointer_8)(mdc);
                        let ovr = (r.pcre2_get_ovector_pointer_8)(mdr);
                        for i in 0..(oc as usize) * 2 {
                            *ovc.add(i) = PCRE2_UNSET;
                            *ovr.add(i) = PCRE2_UNSET;
                        }
                        (c.pcre2_match_8)(cc, subj.as_ptr(), subj.len(), 0, 0, mdc, ptr::null_mut());
                        (r.pcre2_match_8)(rr, subj.as_ptr(), subj.len(), 0, 0, mdr, ptr::null_mut());
                        let mut bc = vec![POISON; 64 + TAIL];
                        let mut br = vec![POISON; 64 + TAIL];
                        let mut lc = 64usize;
                        let mut lr = 64usize;
                        let rcc = (c.pcre2_substitute_8)(
                            cc,
                            subj.as_ptr(),
                            subj.len(),
                            0,
                            uu,
                            mdc,
                            ptr::null_mut(),
                            rep.as_ptr(),
                            rep.len(),
                            bc.as_mut_ptr(),
                            &mut lc,
                        );
                        let rcr = (r.pcre2_substitute_8)(
                            rr,
                            subj.as_ptr(),
                            subj.len(),
                            0,
                            uu,
                            mdr,
                            ptr::null_mut(),
                            rep.as_ptr(),
                            rep.len(),
                            br.as_mut_ptr(),
                            &mut lr,
                        );
                        assert_eq!(
                            (rcc, lc, &bc),
                            (rcr, lr, &br),
                            "C180/small-ovec: pat={:?} rep={:?} oveccount={} uu={:#x}",
                            String::from_utf8_lossy(pat),
                            String::from_utf8_lossy(rep),
                            oc,
                            uu
                        );
                        n += 1;
                        (c.pcre2_match_data_free_8)(mdc);
                        (r.pcre2_match_data_free_8)(mdr);
                    }
                }
            }
            free_pair(c, r, cc, rr);
        }
    }
    println!("C180: {} substitute comparisons; distinct rc values seen: {:?}", n, rc_seen());
}

// ======================================================= C181

#[test]
fn c181_substitute_replacement_only_and_partial() {
    println!("C181 pcre2_substitute: REPLACEMENT_ONLY and PCRE2_PARTIAL_*");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let pats: [&[u8]; 5] = [b"(b)", b"abc", b"a(?=bcd)", b"a+", b"(?<nm>xyz)"];
        for pat in pats {
            let Some((cc, rr)) = compile2(c, r, pat, 0) else {
                continue;
            };
            let p = Pair {
                c,
                r,
                code_c: cc,
                code_r: rr,
                mc_c: ptr::null_mut(),
                mc_r: ptr::null_mut(),
                sc_pol: 0,
                cc_pol: 0,
            };
            for subj in [
                &b"zzbzz"[..],
                &b"ab"[..],
                &b"abc"[..],
                &b"xxabcxx"[..],
                &b"xy"[..],
                &b""[..],
            ] {
                for rep in [&b"-"[..], &b"[$0]"[..], &b"$1"[..], &b""[..]] {
                    for ro in [0, PCRE2_SUBSTITUTE_REPLACEMENT_ONLY] {
                        for pm in [0, PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
                            for st in [0usize, 1, 2] {
                                if st > subj.len() {
                                    continue;
                                }
                                let mut a = args(subj, rep);
                                a.start = st;
                                a.options = ro | pm;
                                p.sweep(&a, MdMode::Fresh, "C181/reponly", 14, &mut n);
                                let mut a2 = a;
                                a2.options |= PCRE2_SUBSTITUTE_GLOBAL;
                                p.sweep(&a2, MdMode::Fresh, "C181/reponly-g", 14, &mut n);
                            }
                        }
                    }
                }
            }
            free_pair(c, r, cc, rr);
        }
    }
    println!("C181: {} substitute comparisons; distinct rc values seen: {:?}", n, rc_seen());
}

// ======================================================= C182

#[test]
fn c182_substitute_matched_identity() {
    println!("C182 pcre2_substitute: PCRE2_SUBSTITUTE_MATCHED identity checks");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        static EMPTY_SUBJ: [u8; 1] = [0];
        let subj: &[u8] = b"xxabcxxabcxx";
        let other: &[u8] = b"yyabcyyabcyy";
        let reps: [&[u8]; 6] = [b"-", b"[$0]", b"$1", b"$+", b"${nm}", b"\\U$0"];
        let Some((cc, rr)) = compile2(c, r, b"(?<nm>a)(b)c", 0) else {
            panic!("compile failed")
        };
        let Some((cc2, rr2)) = compile2(c, r, b"(?<nm>a)(b)c", 0) else {
            panic!("compile failed")
        };
        // a pattern that can match an empty subject, needed for the `length == 0`
        // shortcut of the DIFFSUBSSUBJECT check
        let Some((cce, rre)) = compile2(c, r, b"(?<nm>x)?", 0) else {
            panic!("compile failed")
        };
        let wsize = 200usize;

        // A closure that builds a match data on one side according to a recipe,
        // then runs substitute with PCRE2_SUBSTITUTE_MATCHED.
        #[derive(Clone, Copy, PartialEq, Debug)]
        enum Recipe {
            Good,
            NoMatch,
            OtherNegative,
            Dfa,
            DiffCode,
            DiffSubjPtr,
            DiffSubjLen,
            CopiedSame,
            CopiedDifferent,
            CopiedEmpty,
            DiffOffset,
            DiffOptions,
            OnlySubOptionsDiffer,
            SmallOvec,
            NullMd,
        }
        let recipes = [
            Recipe::Good,
            Recipe::NoMatch,
            Recipe::OtherNegative,
            Recipe::Dfa,
            Recipe::DiffCode,
            Recipe::DiffSubjPtr,
            Recipe::DiffSubjLen,
            Recipe::CopiedSame,
            Recipe::CopiedDifferent,
            Recipe::CopiedEmpty,
            Recipe::DiffOffset,
            Recipe::DiffOptions,
            Recipe::OnlySubOptionsDiffer,
            Recipe::SmallOvec,
            Recipe::NullMd,
        ];

        for rec in recipes {
            for rep in reps {
                for extra in [
                    0u32,
                    PCRE2_SUBSTITUTE_GLOBAL,
                    PCRE2_SUBSTITUTE_EXTENDED,
                    PCRE2_SUBSTITUTE_UNSET_EMPTY | PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
                    PCRE2_NO_UTF_CHECK,
                ] {
                    for buflen in [0usize, 4, 8, 64] {
                        let mut outs: Vec<(i32, usize, Vec<u8>)> = Vec::new();
                        for (api, code, codeb, codee) in
                            [(c, cc, cc2, cce), (r, rr, rr2, rre)]
                        {
                            let mut sub_opts = PCRE2_SUBSTITUTE_MATCHED | extra;
                            let mut start = 2usize;
                            let mut use_subj = subj;
                            let mut use_code = code;
                            let mut md: Ptr = ptr::null_mut();
                            match rec {
                                Recipe::NullMd => {}
                                Recipe::Dfa => {
                                    md = fresh_md(api, code);
                                    let mut ws = vec![0i32; wsize];
                                    (api.pcre2_dfa_match_8)(
                                        code,
                                        subj.as_ptr(),
                                        subj.len(),
                                        2,
                                        0,
                                        md,
                                        ptr::null_mut(),
                                        ws.as_mut_ptr(),
                                        wsize,
                                    );
                                }
                                Recipe::OtherNegative => {
                                    md = fresh_md(api, code);
                                    // a PARTIAL result, i.e. rc == -2
                                    let s: &[u8] = b"xxab";
                                    (api.pcre2_match_8)(
                                        code,
                                        s.as_ptr(),
                                        s.len(),
                                        2,
                                        PCRE2_PARTIAL_HARD,
                                        md,
                                        ptr::null_mut(),
                                    );
                                }
                                Recipe::NoMatch => {
                                    md = fresh_md(api, code);
                                    let s: &[u8] = b"zzzzzzzzzzzz";
                                    (api.pcre2_match_8)(
                                        code,
                                        s.as_ptr(),
                                        s.len(),
                                        2,
                                        0,
                                        md,
                                        ptr::null_mut(),
                                    );
                                    // the substitute must be told the same subject
                                    use_subj = s;
                                }
                                Recipe::DiffCode => {
                                    md = fresh_md(api, codeb);
                                    (api.pcre2_match_8)(
                                        codeb,
                                        subj.as_ptr(),
                                        subj.len(),
                                        2,
                                        0,
                                        md,
                                        ptr::null_mut(),
                                    );
                                }
                                Recipe::DiffSubjPtr => {
                                    md = fresh_md(api, code);
                                    (api.pcre2_match_8)(
                                        code,
                                        other.as_ptr(),
                                        other.len(),
                                        2,
                                        0,
                                        md,
                                        ptr::null_mut(),
                                    );
                                }
                                Recipe::DiffSubjLen => {
                                    md = fresh_md(api, code);
                                    (api.pcre2_match_8)(
                                        code,
                                        subj.as_ptr(),
                                        subj.len() - 1,
                                        2,
                                        0,
                                        md,
                                        ptr::null_mut(),
                                    );
                                }
                                Recipe::CopiedEmpty => {
                                    // a successful match on an EMPTY subject with
                                    // PCRE2_COPY_MATCHED_SUBJECT leaves
                                    // match_data->subject == NULL, so the identity
                                    // check must take the `length == 0` shortcut
                                    use_code = codee;
                                    md = fresh_md(api, codee);
                                    let empty = vec![0u8; 1];
                                    (api.pcre2_match_8)(
                                        codee,
                                        empty.as_ptr(),
                                        0,
                                        0,
                                        PCRE2_COPY_MATCHED_SUBJECT,
                                        md,
                                        ptr::null_mut(),
                                    );
                                    sub_opts |= PCRE2_COPY_MATCHED_SUBJECT;
                                    use_subj = &EMPTY_SUBJ[..0];
                                    start = 0;
                                }
                                Recipe::CopiedSame | Recipe::CopiedDifferent => {
                                    md = fresh_md(api, code);
                                    let src: &[u8] = if rec == Recipe::CopiedSame {
                                        subj
                                    } else {
                                        other
                                    };
                                    // a private copy with the same contents, so the
                                    // pointer differs but the memcmp succeeds
                                    let copy = src.to_vec();
                                    (api.pcre2_match_8)(
                                        code,
                                        copy.as_ptr(),
                                        copy.len(),
                                        2,
                                        PCRE2_COPY_MATCHED_SUBJECT,
                                        md,
                                        ptr::null_mut(),
                                    );
                                    sub_opts |= PCRE2_COPY_MATCHED_SUBJECT;
                                }
                                Recipe::DiffOffset => {
                                    md = fresh_md(api, code);
                                    (api.pcre2_match_8)(
                                        code,
                                        subj.as_ptr(),
                                        subj.len(),
                                        3,
                                        0,
                                        md,
                                        ptr::null_mut(),
                                    );
                                }
                                Recipe::DiffOptions => {
                                    md = fresh_md(api, code);
                                    (api.pcre2_match_8)(
                                        code,
                                        subj.as_ptr(),
                                        subj.len(),
                                        2,
                                        PCRE2_NOTBOL,
                                        md,
                                        ptr::null_mut(),
                                    );
                                }
                                Recipe::OnlySubOptionsDiffer => {
                                    md = fresh_md(api, code);
                                    (api.pcre2_match_8)(
                                        code,
                                        subj.as_ptr(),
                                        subj.len(),
                                        2,
                                        0,
                                        md,
                                        ptr::null_mut(),
                                    );
                                    sub_opts |= PCRE2_SUBSTITUTE_GLOBAL
                                        | PCRE2_SUBSTITUTE_EXTENDED
                                        | PCRE2_NO_UTF_CHECK;
                                }
                                Recipe::SmallOvec => {
                                    md = (api.pcre2_match_data_create_8)(1, ptr::null_mut());
                                    let ov = (api.pcre2_get_ovector_pointer_8)(md);
                                    *ov = PCRE2_UNSET;
                                    *ov.add(1) = PCRE2_UNSET;
                                    (api.pcre2_match_8)(
                                        code,
                                        subj.as_ptr(),
                                        subj.len(),
                                        2,
                                        0,
                                        md,
                                        ptr::null_mut(),
                                    );
                                }
                                Recipe::Good => {
                                    md = fresh_md(api, code);
                                    (api.pcre2_match_8)(
                                        code,
                                        subj.as_ptr(),
                                        subj.len(),
                                        2,
                                        0,
                                        md,
                                        ptr::null_mut(),
                                    );
                                }
                            }
                            if rec == Recipe::DiffOffset {
                                start = 2; // substitute uses 2, the match used 3
                            }
                            let mut buf = vec![POISON; buflen + TAIL];
                            let mut ol = buflen;
                            let rcv = (api.pcre2_substitute_8)(
                                use_code,
                                use_subj.as_ptr(),
                                use_subj.len(),
                                start,
                                sub_opts,
                                md,
                                ptr::null_mut(),
                                rep.as_ptr(),
                                rep.len(),
                                buf.as_mut_ptr(),
                                &mut ol,
                            );
                            outs.push((rcv, ol, buf));
                            if !md.is_null() {
                                (api.pcre2_match_data_free_8)(md);
                            }
                        }
                        assert_eq!(
                            outs[0], outs[1],
                            "C182 {:?}: rep={:?} extra={:#x} buflen={}",
                            rec,
                            String::from_utf8_lossy(rep),
                            extra,
                            buflen
                        );
                        if buflen == 64 && extra == 0 {
                            println!(
                                "  C182 {:22?} rep={:8} -> rc={} *blength={}",
                                rec,
                                format!("{:?}", String::from_utf8_lossy(rep)),
                                outs[0].0,
                                outs[0].1 as i64
                            );
                        }
                        n += 1;
                    }
                }
            }
        }
        free_pair(c, r, cc, rr);
        free_pair(c, r, cc2, rr2);
        free_pair(c, r, cce, rre);
    }
    println!("C182: {} substitute comparisons", n);
}

// ======================================================= C183

#[test]
fn c183_substitute_utf() {
    println!("C183 pcre2_substitute: UTF validation of the replacement, group names");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let long128: Vec<u8> = {
            let mut v = b"${".to_vec();
            v.extend(std::iter::repeat(b'a').take(128));
            v.push(b'}');
            v
        };
        let long129: Vec<u8> = {
            let mut v = b"${".to_vec();
            v.extend(std::iter::repeat(b'a').take(129));
            v.push(b'}');
            v
        };
        let reps: Vec<&[u8]> = vec![
            b"\xff",
            b"\xc3",
            b"\xc3\x28",
            b"a\xe2\x82b",
            b"\xed\xa0\x80",
            b"\xf5\x80\x80\x80",
            b"\xc3\xa9",
            b"\xe2\x82\xac",
            b"\xf0\x9f\x98\x80",
            b"\\x{100}",
            b"\\x{10ffff}",
            b"\\x{110000}",
            b"${\xc3\xa9}",
            b"$<\xc3\xa9>",
            b"${}",
            b"$<>",
            &long128,
            &long129,
            b"\\g<\xc3\xa9>",
            b"[$0]\xc3\xa9",
        ];
        for (pat, popt) in [
            (&b"(?<\xc3\xa9>.)"[..], PCRE2_UTF | PCRE2_UCP),
            (&b"(.)"[..], PCRE2_UTF),
            (&b"(.)"[..], 0u32),
            (&b"(?<a>.)"[..], PCRE2_UTF),
        ] {
            let Some((cc, rr)) = compile2(c, r, pat, popt) else {
                continue;
            };
            let p = Pair {
                c,
                r,
                code_c: cc,
                code_r: rr,
                mc_c: ptr::null_mut(),
                mc_r: ptr::null_mut(),
                sc_pol: 0,
                cc_pol: 0,
            };
            for subj in [
                &b"a"[..],
                &b"\xc3\xa9"[..],
                &b"\xe2\x82\xac"[..],
                &b""[..],
                &b"abc"[..],
            ] {
                let subj_ok = valid_utf(c, subj) == valid_utf(r, subj) && valid_utf(c, subj);
                for rep in reps.iter() {
                    let rep_ok = valid_utf(c, rep);
                    for ext in [0, PCRE2_SUBSTITUTE_EXTENDED] {
                        // Without NO_UTF_CHECK: always safe, the library validates.
                        let mut a = args(subj, rep);
                        a.options = ext;
                        p.sweep(&a, MdMode::Fresh, "C183/utf", 12, &mut n);
                        // With NO_UTF_CHECK only when the data really is valid,
                        // because otherwise PCRE2 documents undefined behaviour.
                        if popt & PCRE2_UTF == 0 || (subj_ok && rep_ok) {
                            let mut a2 = args(subj, rep);
                            a2.options = ext | PCRE2_NO_UTF_CHECK;
                            p.sweep(&a2, MdMode::Fresh, "C183/utf-nocheck", 12, &mut n);
                        }
                    }
                }
            }
            free_pair(c, r, cc, rr);
        }
    }
    println!("C183: {} substitute comparisons; distinct rc values seen: {:?}", n, rc_seen());
}

// ======================================================= C170-C183 fuzz

#[test]
fn c170_c183_substitute_random_cross_product() {
    println!("C170-C183 pcre2_substitute: randomized cross product (fixed seed)");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        // ---- pattern pool: 0..8 captures, named / duplicate / optional groups,
        // empty-matching, anchored and UTF patterns
        let pats: [(&[u8], u32); 22] = [
            (b"a", 0),
            (b"(a)", 0),
            (b"(a)(b)", 0),
            (b"(?<name>a)", 0),
            (b"(?<name>a)(b)(c)", 0),
            (b"(a)(b)(c)(d)(e)(f)(g)(h)", 0),
            (b"(?J)(?<name>a)|(?<name>b)", 0),
            (b"(?J)(?:(?<name>a)|(?<name>b)|(?<name>c))", 0),
            (b"(a)?(b)", 0),
            (b"a*", 0),
            (b"\\b", 0),
            (b"(?=x)", 0),
            (b"^a", 0),
            (b"a$", 0),
            (b"(?<name>\\w+)", 0),
            (b"[a-c]+", 0),
            (b"(*MARK:MK)x", 0),
            (b"(?<name>.)(?<other>.)?", 0),
            (b"\\d", 0),
            (b"(?<name>.)", PCRE2_UTF | PCRE2_UCP),
            (b"(\\x{100})?(.)", PCRE2_UTF),
            (b"(?J)(?<name>x)|(?<name>\\x{e9})", PCRE2_UTF | PCRE2_UCP),
        ];
        let mut codes: Vec<(Ptr, Ptr, u32)> = Vec::new();
        for (pat, popt) in pats {
            if let Some((a, b)) = compile2(c, r, pat, popt) {
                codes.push((a, b, popt));
            }
        }
        assert!(codes.len() >= 20, "pattern pool did not compile");

        // ---- subject pool
        let mut rng = Rng::new(0x5175_7B57_C170_C183u64);
        let mut subjects: Vec<Vec<u8>> = Vec::new();
        for i in 0..140 {
            let len = rng.below(41) as usize;
            let kind = i % 7;
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                let b = match kind {
                    0 => *rng.pick(b"abcxyz"),
                    1 => rng.byte(),
                    2 => 0x80u8 | (rng.byte() & 0x7f),
                    3 => *rng.pick(b"\r\n\0abc"),
                    4 => *rng.pick(b"abcABC012 -"),
                    5 => 0,
                    _ => *rng.pick(b"a\xc3\xa9\xe2\x82\xac\xf0\x9f\x98\x80x"),
                };
                v.push(b);
            }
            if kind == 6 {
                // build a valid UTF-8 string of the same rough size
                v.clear();
                let cps: [u32; 6] = [0x41, 0xe9, 0x20ac, 0x1f600, 0x7f, 0x100];
                while v.len() < len {
                    let cp = cps[rng.below(6) as usize];
                    let mut tmp = [0u8; 8];
                    let l = (c._pcre2_ord2utf_8)(cp, tmp.as_mut_ptr()) as usize;
                    if v.len() + l > len {
                        break;
                    }
                    v.extend_from_slice(&tmp[..l]);
                }
            }
            subjects.push(v);
        }

        // ---- replacement pool built from random tokens
        let tokens: [&[u8]; 48] = [
            b"a",
            b"Z",
            b"-",
            b" ",
            b"\xff",
            b"\x00",
            b"$0",
            b"$1",
            b"$2",
            b"$3",
            b"$9",
            b"${1}",
            b"${2}",
            b"$name",
            b"${name}",
            b"$$",
            b"$&",
            b"\\$",
            b"\\\\",
            b"\\n",
            b"\\U",
            b"\\L",
            b"\\u",
            b"\\l",
            b"\\E",
            b"${name:-default}",
            b"${name:+a:b}",
            b"${1:-$2}",
            b"${",
            b"$",
            b"$99",
            b"$`",
            b"$'",
            b"$_",
            b"$+",
            b"${*MARK}",
            b"$<name>",
            b"\\Q",
            b"\\x41",
            b"\\o{101}",
            b"\\g{2}",
            b"\\g<name>",
            b"\\1",
            b"\\Qxy\\E",
            b"${1:+${2:-x}:y}",
            b"${1:?x}",
            b"${1:-x",
            b"$<1",
        ];
        let mut reps: Vec<Vec<u8>> = Vec::new();
        for _ in 0..160 {
            let k = rng.below(7) as usize;
            let mut v = Vec::new();
            for _ in 0..k {
                v.extend_from_slice(rng.pick(&tokens));
            }
            reps.push(v);
        }

        // NUL-terminated copies for the PCRE2_ZERO_TERMINATED conversions
        let subjects_z: Vec<Vec<u8>> = subjects
            .iter()
            .map(|s| {
                let mut v = s.clone();
                v.push(0);
                v
            })
            .collect();
        let reps_z: Vec<Vec<u8>> = reps
            .iter()
            .map(|s| {
                let mut v = s.clone();
                v.push(0);
                v
            })
            .collect();

        let sub_bits: [u32; 8] = [
            PCRE2_SUBSTITUTE_GLOBAL,
            PCRE2_SUBSTITUTE_EXTENDED,
            PCRE2_SUBSTITUTE_LITERAL,
            PCRE2_SUBSTITUTE_UNSET_EMPTY,
            PCRE2_SUBSTITUTE_UNKNOWN_UNSET,
            PCRE2_SUBSTITUTE_OVERFLOW_LENGTH,
            PCRE2_SUBSTITUTE_REPLACEMENT_ONLY,
            PCRE2_SUBSTITUTE_MATCHED,
        ];
        let match_bits: [u32; 8] = [
            PCRE2_NOTBOL,
            PCRE2_NOTEOL,
            PCRE2_NOTEMPTY,
            PCRE2_NOTEMPTY_ATSTART,
            PCRE2_ANCHORED,
            PCRE2_ENDANCHORED,
            PCRE2_PARTIAL_SOFT,
            PCRE2_PARTIAL_HARD,
        ];

        // four match-context variants per library: no callouts, substitute
        // callout only, case callout only, and both
        let mut mcs: Vec<(Ptr, Ptr)> = Vec::new();
        for which in 0..4u32 {
            let a = (c.pcre2_match_context_create_8)(ptr::null_mut());
            let b = (r.pcre2_match_context_create_8)(ptr::null_mut());
            if which & 1 != 0 {
                (c.pcre2_set_substitute_callout_8)(a, Some(sc_cb), sentinel_ptr());
                (r.pcre2_set_substitute_callout_8)(b, Some(sc_cb), sentinel_ptr());
            }
            if which & 2 != 0 {
                (c.pcre2_set_substitute_case_callout_8)(a, Some(cc_cb), ptr::null_mut());
                (r.pcre2_set_substitute_case_callout_8)(b, Some(cc_cb), ptr::null_mut());
            }
            mcs.push((a, b));
        }

        let iters = 45000;
        for _ in 0..iters {
            let (cc, rr, popt) = codes[rng.below(codes.len() as u32) as usize];
            let si = rng.below(subjects.len() as u32) as usize;
            let ri = rng.below(reps.len() as u32) as usize;
            let subj = &subjects[si];
            let rep = &reps[ri];

            let mut options = 0u32;
            for (i, b) in sub_bits.iter().enumerate() {
                let _ = i;
                if rng.below(3) == 0 {
                    options |= *b;
                }
            }
            for b in match_bits.iter() {
                if rng.below(6) == 0 {
                    options |= *b;
                }
            }
            // NO_UTF_CHECK is only safe when the data is genuinely valid UTF-8
            if popt & PCRE2_UTF == 0 || (valid_utf(c, subj) && valid_utf(c, rep)) {
                if rng.below(3) == 0 {
                    options |= PCRE2_NO_UTF_CHECK;
                }
            }
            let start = if subj.is_empty() {
                0
            } else {
                rng.below(subj.len() as u32 + 2) as usize
            };
            let mdm = if options & PCRE2_SUBSTITUTE_MATCHED != 0 {
                MdMode::Matched
            } else if rng.below(2) == 0 {
                MdMode::Fresh
            } else {
                MdMode::Null
            };

            let mut a = args(subj, rep);
            a.start = start;
            a.options = options;
            // PCRE2_ZERO_TERMINATED requires a genuinely NUL-terminated buffer
            let szi = rng.below(8) == 0;
            let rzi = rng.below(8) == 0;
            if szi {
                a.subj = subjects_z[si].as_ptr();
                a.slen = PCRE2_ZERO_TERMINATED;
            }
            if rzi {
                a.rep = reps_z[ri].as_ptr();
                a.rlen = PCRE2_ZERO_TERMINATED;
            }

            let (mc_c, mc_r) = mcs[rng.below(4) as usize];
            let p = Pair {
                c,
                r,
                code_c: cc,
                code_r: rr,
                mc_c,
                mc_r,
                sc_pol: rng.below(6),
                cc_pol: rng.below(6),
            };

            // one generous call to learn the needed length ...
            let mut probe = a;
            probe.buflen = 256;
            let big = p.one(&probe, mdm, "C170-C183/fuzz-probe");
            n += 1;
            let need = if big.rc >= 0 {
                big.outlen + 1
            } else if big.outlen != PCRE2_UNSET && big.outlen < 256 {
                big.outlen + 1
            } else {
                4
            };
            // ... then every buffer size from 0 to need+4 (capped)
            let top = (need + 4).min(18);
            for bl in 0..=top {
                let mut aa = a;
                aa.buflen = bl;
                p.one(&aa, mdm, "C170-C183/fuzz");
                n += 1;
            }
            // and a NULL buffer with zero length
            let mut aa = a;
            aa.buflen = 0;
            aa.null_buf = true;
            aa.options |= PCRE2_SUBSTITUTE_OVERFLOW_LENGTH;
            p.one(&aa, mdm, "C170-C183/fuzz-nullbuf");
            n += 1;
        }
        for (a, b, _) in codes {
            free_pair(c, r, a, b);
        }
        for (a, b) in mcs {
            (c.pcre2_match_context_free_8)(a);
            (r.pcre2_match_context_free_8)(b);
        }
    }
    println!(
        "C170-C183 fuzz: {} substitute comparisons, {} substitute-callout and {} case-callout invocations; distinct rc values seen: {:?}",
        n,
        SC_CALLS.with(|x| x.get()),
        CC_CALLS.with(|x| x.get()),
        rc_seen()
    );
    assert!(n >= 50_000, "only {} substitute comparisons", n);
}

// ======================================================= C184-C187

#[derive(PartialEq, Eq, Debug, Clone)]
struct SubstrOut {
    len_rc: i32,
    len: usize,
    len_null_rc: i32,
    copy: Vec<(usize, i32, usize, Vec<u8>)>,
    get_rc: i32,
    get_size: usize,
    get_bytes: Option<Vec<u8>>,
}

unsafe fn probe_number(api: &Api, md: Ptr, group: u32) -> SubstrOut {
    let mut len: usize = 0xdead_beef;
    let len_rc = (api.pcre2_substring_length_bynumber_8)(md, group, &mut len);
    let len_null_rc = (api.pcre2_substring_length_bynumber_8)(md, group, ptr::null_mut());
    let known = if len_rc == 0 { len } else { 2 };

    let mut copy = Vec::new();
    for sz in 0..=known + 2 {
        let mut buf = vec![POISON; sz + TAIL];
        let mut szv = sz;
        let rc = (api.pcre2_substring_copy_bynumber_8)(md, group, buf.as_mut_ptr(), &mut szv);
        copy.push((sz, rc, szv, buf));
    }

    let mut sp: *mut u8 = ptr::null_mut();
    let mut gs: usize = 0xdead_beef;
    let get_rc = (api.pcre2_substring_get_bynumber_8)(md, group, &mut sp, &mut gs);
    let get_bytes = if get_rc == 0 && !sp.is_null() {
        let v = std::slice::from_raw_parts(sp, gs + 1).to_vec();
        (api.pcre2_substring_free_8)(sp);
        Some(v)
    } else {
        assert!(get_rc != 0, "{}: get_bynumber returned 0 with NULL", api.tag);
        None
    };
    SubstrOut {
        len_rc,
        len: if len_rc == 0 { len } else { 0xdead_beef },
        len_null_rc,
        copy,
        get_rc,
        get_size: if get_rc == 0 { gs } else { 0xdead_beef },
        get_bytes,
    }
}

unsafe fn probe_name(api: &Api, md: Ptr, name: &[u8]) -> SubstrOut {
    let mut len: usize = 0xdead_beef;
    let len_rc = (api.pcre2_substring_length_byname_8)(md, name.as_ptr(), &mut len);
    let len_null_rc = (api.pcre2_substring_length_byname_8)(md, name.as_ptr(), ptr::null_mut());
    let known = if len_rc == 0 { len } else { 2 };

    let mut copy = Vec::new();
    for sz in 0..=known + 2 {
        let mut buf = vec![POISON; sz + TAIL];
        let mut szv = sz;
        let rc = (api.pcre2_substring_copy_byname_8)(md, name.as_ptr(), buf.as_mut_ptr(), &mut szv);
        copy.push((sz, rc, szv, buf));
    }

    let mut sp: *mut u8 = ptr::null_mut();
    let mut gs: usize = 0xdead_beef;
    let get_rc = (api.pcre2_substring_get_byname_8)(md, name.as_ptr(), &mut sp, &mut gs);
    let get_bytes = if get_rc == 0 && !sp.is_null() {
        let v = std::slice::from_raw_parts(sp, gs + 1).to_vec();
        (api.pcre2_substring_free_8)(sp);
        Some(v)
    } else {
        None
    };
    SubstrOut {
        len_rc,
        len: if len_rc == 0 { len } else { 0xdead_beef },
        len_null_rc,
        copy,
        get_rc,
        get_size: if get_rc == 0 { gs } else { 0xdead_beef },
        get_bytes,
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
struct ListOut {
    rc: i32,
    /// byte offset of every returned string pointer within the returned block
    offs: Vec<isize>,
    strings: Vec<Vec<u8>>,
    lengths: Option<Vec<usize>>,
}

unsafe fn probe_list(api: &Api, md: Ptr, with_lengths: bool, count_hint: usize) -> ListOut {
    let mut list: *mut *mut u8 = ptr::null_mut();
    let mut lens: *mut usize = ptr::null_mut();
    let rc = (api.pcre2_substring_list_get_8)(
        md,
        &mut list,
        if with_lengths {
            &mut lens
        } else {
            ptr::null_mut()
        },
    );
    if rc != 0 {
        return ListOut {
            rc,
            offs: vec![],
            strings: vec![],
            lengths: None,
        };
    }
    let base = list as *const u8;
    let mut offs = Vec::new();
    let mut strings = Vec::new();
    let mut i = 0usize;
    loop {
        let p = *list.add(i);
        if p.is_null() {
            offs.push(-1);
            break;
        }
        offs.push(p as isize - base as isize);
        i += 1;
        if i > count_hint + 4 {
            panic!("{}: substring list not NULL-terminated", api.tag);
        }
    }
    let cnt = i;
    let lengths = if with_lengths && !lens.is_null() {
        Some(std::slice::from_raw_parts(lens, cnt).to_vec())
    } else {
        None
    };
    for j in 0..cnt {
        let p = *list.add(j);
        let l = match &lengths {
            Some(v) => v[j],
            None => (api._pcre2_strlen_8)(p),
        };
        strings.push(std::slice::from_raw_parts(p, l + 1).to_vec());
    }
    (api.pcre2_substring_list_free_8)(list);
    ListOut {
        rc,
        offs,
        strings,
        lengths,
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
struct ScanOut {
    rc: i32,
    entries: Vec<u8>,
    span: isize,
    null_rc: i32,
    number_rc: i32,
}

unsafe fn probe_scan(api: &Api, code: Ptr, name: &[u8]) -> ScanOut {
    let mut first: *mut u8 = ptr::null_mut();
    let mut last: *mut u8 = ptr::null_mut();
    let rc = (api.pcre2_substring_nametable_scan_8)(code, name.as_ptr(), &mut first, &mut last);
    let (entries, span) = if rc > 0 {
        let span = last as isize - first as isize;
        let total = span as usize + rc as usize;
        (std::slice::from_raw_parts(first as *const u8, total).to_vec(), span)
    } else {
        (Vec::new(), 0)
    };
    let null_rc =
        (api.pcre2_substring_nametable_scan_8)(code, name.as_ptr(), ptr::null_mut(), ptr::null_mut());
    let number_rc = (api.pcre2_substring_number_from_name_8)(code, name.as_ptr());
    ScanOut {
        rc,
        entries,
        span,
        null_rc,
        number_rc,
    }
}

#[test]
fn c184_c187_substring_functions() {
    println!("C184-C187 pcre2_substring_*: all eleven functions");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let mut long_name: Vec<u8> = std::iter::repeat(b'q').take(300).collect();
        long_name.push(0);
        let names: Vec<Vec<u8>> = vec![
            b"one\0".to_vec(),
            b"two\0".to_vec(),
            b"n\0".to_vec(),
            b"x\0".to_vec(),
            b"yy\0".to_vec(),
            b"zzz\0".to_vec(),
            b"aa\0".to_vec(),
            b"bb\0".to_vec(),
            b"cc\0".to_vec(),
            b"dd\0".to_vec(),
            b"nosuch\0".to_vec(),
            b"\0".to_vec(),
            long_name,
            b"\xc3\xa9\0".to_vec(),
            b"name\0".to_vec(),
            b"NAME\0".to_vec(),
            b"a\0".to_vec(),
        ];

        let pats: [(&[u8], u32); 12] = [
            (b"(a)(b)(c)", 0),
            (b"(?<one>a)(?<two>b)", 0),
            (b"(?J)(?<n>a)|(?<n>b)", 0),
            (b"(?<x>a)(?<yy>b)(?<zzz>c)?", 0),
            (b"(?<aa>a)(?<bb>b)(?<cc>c)(?<dd>d)", 0),
            (b"(?J)(?:(?<n>a)|(?<n>b)|(?<n>c)|(?<n>d))", 0),
            (b"()", 0),
            (b"(?=(a))b", 0),
            (b"a\\Kb", 0),
            (b"(?<name>x)?y", 0),
            (b"abc", 0),
            (b"(?<\xc3\xa9>.)", PCRE2_UTF | PCRE2_UCP),
        ];
        let subjects: [&[u8]; 8] = [
            b"abcd",
            b"b",
            b"a",
            b"y",
            b"ab\0cd",
            b"",
            b"\xc3\xa9",
            b"zzabcdzz",
        ];

        for (pat, popt) in pats {
            let Some((cc, rr)) = compile2(c, r, pat, popt) else {
                continue;
            };
            let mut ccount: u32 = 0;
            (c.pcre2_pattern_info_8)(cc, PCRE2_INFO_CAPTURECOUNT, &mut ccount as *mut u32 as Ptr);

            // ---- nametable_scan / number_from_name (C187): independent of the match
            for name in names.iter() {
                let a = probe_scan(c, cc, name);
                let b = probe_scan(r, rr, name);
                assert_eq!(
                    a,
                    b,
                    "C187 nametable_scan: pat={:?} name={:?}",
                    String::from_utf8_lossy(pat),
                    String::from_utf8_lossy(name)
                );
                n += 1;
            }

            // ---- the match-data variants
            #[derive(Clone, Copy, Debug)]
            enum Md {
                Full,
                Partial,
                Dfa,
                SmallOvec,
                NoMatch,
                Error,
            }
            for subj in subjects {
                for kind in [
                    Md::Full,
                    Md::Partial,
                    Md::Dfa,
                    Md::SmallOvec,
                    Md::NoMatch,
                    Md::Error,
                ] {
                    let mut per_side: Vec<(Vec<SubstrOut>, Vec<SubstrOut>, ListOut, ListOut)> =
                        Vec::new();
                    for (api, code) in [(c, cc), (r, rr)] {
                        let md = match kind {
                            Md::SmallOvec => {
                                let md = (api.pcre2_match_data_create_8)(1, ptr::null_mut());
                                let ov = (api.pcre2_get_ovector_pointer_8)(md);
                                *ov = PCRE2_UNSET;
                                *ov.add(1) = PCRE2_UNSET;
                                (api.pcre2_match_8)(
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    0,
                                    0,
                                    md,
                                    ptr::null_mut(),
                                );
                                md
                            }
                            Md::Dfa => {
                                let md = fresh_md(api, code);
                                let mut ws = vec![0i32; 200];
                                (api.pcre2_dfa_match_8)(
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    0,
                                    0,
                                    md,
                                    ptr::null_mut(),
                                    ws.as_mut_ptr(),
                                    200,
                                );
                                md
                            }
                            Md::Partial => {
                                let md = fresh_md(api, code);
                                (api.pcre2_match_8)(
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    0,
                                    PCRE2_PARTIAL_HARD,
                                    md,
                                    ptr::null_mut(),
                                );
                                md
                            }
                            Md::NoMatch => {
                                let md = fresh_md(api, code);
                                let s: &[u8] = b"\x01\x02\x03";
                                (api.pcre2_match_8)(
                                    code,
                                    s.as_ptr(),
                                    s.len(),
                                    0,
                                    0,
                                    md,
                                    ptr::null_mut(),
                                );
                                md
                            }
                            Md::Error => {
                                let md = fresh_md(api, code);
                                // start offset past the end: rc == BADOFFSET
                                (api.pcre2_match_8)(
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    subj.len() + 3,
                                    0,
                                    md,
                                    ptr::null_mut(),
                                );
                                md
                            }
                            Md::Full => {
                                let md = fresh_md(api, code);
                                (api.pcre2_match_8)(
                                    code,
                                    subj.as_ptr(),
                                    subj.len(),
                                    0,
                                    0,
                                    md,
                                    ptr::null_mut(),
                                );
                                md
                            }
                        };
                        let mut byno = Vec::new();
                        for g in 0..=ccount + 2 {
                            byno.push(probe_number(api, md, g));
                        }
                        let mut byname = Vec::new();
                        for name in names.iter() {
                            byname.push(probe_name(api, md, name));
                        }
                        let l1 = probe_list(api, md, true, (ccount + 1) as usize);
                        let l2 = probe_list(api, md, false, (ccount + 1) as usize);
                        (api.pcre2_match_data_free_8)(md);
                        per_side.push((byno, byname, l1, l2));
                    }
                    let (ac, an, al1, al2) = &per_side[0];
                    let (bc, bn, bl1, bl2) = &per_side[1];
                    for (g, (x, y)) in ac.iter().zip(bc.iter()).enumerate() {
                        assert_eq!(
                            x,
                            y,
                            "C185 bynumber: pat={:?} subj={:?} md={:?} group={}",
                            String::from_utf8_lossy(pat),
                            subj,
                            kind,
                            g
                        );
                        n += 1;
                    }
                    for (i, (x, y)) in an.iter().zip(bn.iter()).enumerate() {
                        assert_eq!(
                            x,
                            y,
                            "C184 byname: pat={:?} subj={:?} md={:?} name={:?}",
                            String::from_utf8_lossy(pat),
                            subj,
                            kind,
                            String::from_utf8_lossy(&names[i])
                        );
                        n += 1;
                    }
                    assert_eq!(
                        al1,
                        bl1,
                        "C186 list_get(lengths): pat={:?} subj={:?} md={:?}",
                        String::from_utf8_lossy(pat),
                        subj,
                        kind
                    );
                    assert_eq!(
                        al2,
                        bl2,
                        "C186 list_get(no lengths): pat={:?} subj={:?} md={:?}",
                        String::from_utf8_lossy(pat),
                        subj,
                        kind
                    );
                    n += 2;
                }
            }

            // ---- C186: a general context whose malloc fails
            {
                let subj: &[u8] = b"abcd";
                let mut outs: Vec<(i32, i32, i32)> = Vec::new();
                for (api, code) in [(c, cc), (r, rr)] {
                    let gc =
                        (api.pcre2_general_context_create_8)(Some(t_malloc), Some(t_free), ptr::null_mut());
                    assert!(!gc.is_null());
                    let md = (api.pcre2_match_data_create_from_pattern_8)(code, gc);
                    let nov = (api.pcre2_get_ovector_count_8)(md) as usize;
                    let ov = (api.pcre2_get_ovector_pointer_8)(md);
                    for i in 0..nov * 2 {
                        *ov.add(i) = PCRE2_UNSET;
                    }
                    (api.pcre2_match_8)(code, subj.as_ptr(), subj.len(), 0, 0, md, ptr::null_mut());
                    MALLOC_BUDGET.with(|x| x.set(0));
                    let mut list: *mut *mut u8 = ptr::null_mut();
                    let mut lens: *mut usize = ptr::null_mut();
                    let rc1 = (api.pcre2_substring_list_get_8)(md, &mut list, &mut lens);
                    let rc2 = (api.pcre2_substring_list_get_8)(md, &mut list, ptr::null_mut());
                    let mut sp: *mut u8 = ptr::null_mut();
                    let mut gs: usize = 0;
                    let rc3 = (api.pcre2_substring_get_bynumber_8)(md, 0, &mut sp, &mut gs);
                    MALLOC_BUDGET.with(|x| x.set(-1));
                    if rc3 == 0 && !sp.is_null() {
                        (api.pcre2_substring_free_8)(sp);
                    }
                    outs.push((rc1, rc2, rc3));
                    (api.pcre2_match_data_free_8)(md);
                    (api.pcre2_general_context_free_8)(gc);
                }
                assert_eq!(
                    outs[0], outs[1],
                    "C186 malloc failure: pat={:?}",
                    String::from_utf8_lossy(pat)
                );
                n += 1;
            }

            free_pair(c, r, cc, rr);
        }
    }
    println!("C184-C187: {} substring comparisons", n);
}

// ======================================================= C188 / C189

unsafe fn serialize_of(api: &Api, codes: &[Ptr], gc: Ptr) -> (i32, Option<Vec<u8>>, usize) {
    let mut bytes: *mut u8 = ptr::null_mut();
    let mut size: usize = 0xdead_beef;
    let rc = (api.pcre2_serialize_encode_8)(
        codes.as_ptr() as *const Ptr,
        codes.len() as i32,
        &mut bytes,
        &mut size,
        gc,
    );
    if rc < 0 {
        return (rc, None, size);
    }
    let v = std::slice::from_raw_parts(bytes, size).to_vec();
    (api.pcre2_serialize_free_8)(bytes);
    (rc, Some(v), size)
}

/// Match a subject with `code` and report everything observable.
unsafe fn match_probe(api: &Api, code: Ptr, subj: &[u8]) -> (i32, Vec<usize>) {
    let md = (api.pcre2_match_data_create_from_pattern_8)(code, ptr::null_mut());
    let nov = (api.pcre2_get_ovector_count_8)(md) as usize;
    let ov = (api.pcre2_get_ovector_pointer_8)(md);
    for i in 0..nov * 2 {
        *ov.add(i) = PCRE2_UNSET;
    }
    let rc = (api.pcre2_match_8)(code, subj.as_ptr(), subj.len(), 0, 0, md, ptr::null_mut());
    let out = std::slice::from_raw_parts(ov, nov * 2).to_vec();
    (api.pcre2_match_data_free_8)(md);
    (rc, out)
}

#[test]
fn c188_c189_serialize() {
    println!("C188-C189 pcre2_serialize_encode / decode / get_number_of_codes");
    let (c, r) = both();
    let mut n = 0usize;
    unsafe {
        let pats: [&[u8]; 8] = [
            b"abc",
            b"(a)(b)(c)",
            b"(?<one>x)|(?<two>y)",
            b"^\\d{2,4}$",
            b"[a-z]+\\b",
            b"(?i)HeLLo",
            b"(?<nm>.)(?<other>.)?",
            b"a(?:b|c)*d",
        ];
        let subjects: [&[u8]; 6] = [b"abc", b"x", b"1234", b"hello", b"abcd", b"zz"];

        let mut codes_c: Vec<Ptr> = Vec::new();
        let mut codes_r: Vec<Ptr> = Vec::new();
        for pat in pats {
            let Some((a, b)) = compile2(c, r, pat, 0) else {
                panic!("compile failed for {:?}", String::from_utf8_lossy(pat))
            };
            codes_c.push(a);
            codes_r.push(b);
        }

        // -------------------------------------------------- C188: encode
        for gc_used in [false, true] {
            let gc_c = if gc_used {
                (c.pcre2_general_context_create_8)(Some(t_malloc), Some(t_free), ptr::null_mut())
            } else {
                ptr::null_mut()
            };
            let gc_r = if gc_used {
                (r.pcre2_general_context_create_8)(Some(t_malloc), Some(t_free), ptr::null_mut())
            } else {
                ptr::null_mut()
            };
            for cnt in [1usize, 2, 8] {
                let a = serialize_of(c, &codes_c[..cnt], gc_c);
                let b = serialize_of(r, &codes_r[..cnt], gc_r);
                assert_eq!(a.0, b.0, "C188 encode rc differs (n={})", cnt);
                assert_eq!(a.2, b.2, "C188 serialized_size differs (n={})", cnt);
                assert_eq!(
                    a.1,
                    b.1,
                    "C188 serialized byte stream differs (n={})",
                    cnt
                );
                n += 1;

                let bytes = a.1.clone().unwrap();
                let bc = (c.pcre2_serialize_get_number_of_codes_8)(bytes.as_ptr());
                let br = (r.pcre2_serialize_get_number_of_codes_8)(bytes.as_ptr());
                assert_eq!((bc, br), (cnt as i32, cnt as i32), "C189 get_number_of_codes");
                n += 1;
            }
            // NULL argument checks and bad counts
            let mut bp: *mut u8 = ptr::null_mut();
            let mut sz: usize = 0;
            assert_eq!(
                (c.pcre2_serialize_encode_8)(ptr::null(), 1, &mut bp, &mut sz, gc_c),
                (r.pcre2_serialize_encode_8)(ptr::null(), 1, &mut bp, &mut sz, gc_r),
                "C188 codes == NULL"
            );
            assert_eq!(
                (c.pcre2_serialize_encode_8)(
                    codes_c.as_ptr() as *const Ptr,
                    1,
                    ptr::null_mut(),
                    &mut sz,
                    gc_c
                ),
                (r.pcre2_serialize_encode_8)(
                    codes_r.as_ptr() as *const Ptr,
                    1,
                    ptr::null_mut(),
                    &mut sz,
                    gc_r
                ),
                "C188 serialized_bytes == NULL"
            );
            assert_eq!(
                (c.pcre2_serialize_encode_8)(
                    codes_c.as_ptr() as *const Ptr,
                    1,
                    &mut bp,
                    ptr::null_mut(),
                    gc_c
                ),
                (r.pcre2_serialize_encode_8)(
                    codes_r.as_ptr() as *const Ptr,
                    1,
                    &mut bp,
                    ptr::null_mut(),
                    gc_r
                ),
                "C188 serialized_size == NULL"
            );
            for cnt in [0i32, -1, i32::MIN] {
                let mut b1: *mut u8 = ptr::null_mut();
                let mut s1: usize = 0xdead;
                let mut b2: *mut u8 = ptr::null_mut();
                let mut s2: usize = 0xdead;
                let x =
                    (c.pcre2_serialize_encode_8)(codes_c.as_ptr() as *const Ptr, cnt, &mut b1, &mut s1, gc_c);
                let y =
                    (r.pcre2_serialize_encode_8)(codes_r.as_ptr() as *const Ptr, cnt, &mut b2, &mut s2, gc_r);
                assert_eq!((x, s1), (y, s2), "C188 number_of_codes = {}", cnt);
                n += 1;
            }
            // an array containing a NULL element
            {
                let ac: [Ptr; 3] = [codes_c[0], ptr::null_mut(), codes_c[1]];
                let ar: [Ptr; 3] = [codes_r[0], ptr::null_mut(), codes_r[1]];
                let x = serialize_of(c, &ac, gc_c);
                let y = serialize_of(r, &ar, gc_r);
                assert_eq!((x.0, x.2), (y.0, y.2), "C188 NULL element");
                n += 1;
            }
            // a corrupted magic number
            {
                let save_c = *(codes_c[0] as *const u8).add(RC_MAGIC).cast::<u32>();
                let save_r = *(codes_r[0] as *const u8).add(RC_MAGIC).cast::<u32>();
                assert_eq!(save_c, MAGIC_NUMBER);
                assert_eq!(save_r, MAGIC_NUMBER);
                *(codes_c[0] as *mut u8).add(RC_MAGIC).cast::<u32>() = 0x1234_5678;
                *(codes_r[0] as *mut u8).add(RC_MAGIC).cast::<u32>() = 0x1234_5678;
                let x = serialize_of(c, &codes_c[..2], gc_c);
                let y = serialize_of(r, &codes_r[..2], gc_r);
                assert_eq!((x.0, x.2), (y.0, y.2), "C188 BADMAGIC");
                n += 1;
                *(codes_c[0] as *mut u8).add(RC_MAGIC).cast::<u32>() = save_c;
                *(codes_r[0] as *mut u8).add(RC_MAGIC).cast::<u32>() = save_r;
            }
            // two codes compiled with different character tables
            {
                let tb_c = (c.pcre2_maketables_8)(ptr::null_mut());
                let tb_r = (r.pcre2_maketables_8)(ptr::null_mut());
                assert!(!tb_c.is_null() && !tb_r.is_null());
                let cx_c = (c.pcre2_compile_context_create_8)(ptr::null_mut());
                let cx_r = (r.pcre2_compile_context_create_8)(ptr::null_mut());
                (c.pcre2_set_character_tables_8)(cx_c, tb_c);
                (r.pcre2_set_character_tables_8)(cx_r, tb_r);
                let Some((oc, or)) = compile2_ctx(c, r, b"other", 0, cx_c, cx_r) else {
                    panic!("compile with custom tables failed")
                };
                let ac: [Ptr; 2] = [codes_c[0], oc];
                let ar: [Ptr; 2] = [codes_r[0], or];
                let x = serialize_of(c, &ac, gc_c);
                let y = serialize_of(r, &ar, gc_r);
                assert_eq!((x.0, x.2), (y.0, y.2), "C188 MIXEDTABLES");
                assert_eq!(x.0, PCRE2_ERROR_MIXEDTABLES, "C188 expected MIXEDTABLES");
                n += 1;
                // and the single custom-tables code on its own must round-trip
                let x1 = serialize_of(c, &[oc], gc_c);
                let y1 = serialize_of(r, &[or], gc_r);
                assert_eq!(x1, y1, "C188 custom tables single code");
                n += 1;
                (c.pcre2_code_free_8)(oc);
                (r.pcre2_code_free_8)(or);
                (c.pcre2_compile_context_free_8)(cx_c);
                (r.pcre2_compile_context_free_8)(cx_r);
                (c.pcre2_maketables_free_8)(ptr::null_mut(), tb_c);
                (r.pcre2_maketables_free_8)(ptr::null_mut(), tb_r);
            }
            // a gcontext whose malloc fails
            if gc_used {
                MALLOC_BUDGET.with(|x| x.set(0));
                let x = serialize_of(c, &codes_c[..2], gc_c);
                let y = serialize_of(r, &codes_r[..2], gc_r);
                MALLOC_BUDGET.with(|x| x.set(-1));
                assert_eq!((x.0, x.2), (y.0, y.2), "C188 NOMEMORY");
                assert_eq!(x.0, PCRE2_ERROR_NOMEMORY);
                n += 1;
            }
            if gc_used {
                (c.pcre2_general_context_free_8)(gc_c);
                (r.pcre2_general_context_free_8)(gc_r);
            }
        }

        // -------------------------------------------------- C189: decode
        let stream_c = serialize_of(c, &codes_c[..8], ptr::null_mut()).1.unwrap();
        let stream_r = serialize_of(r, &codes_r[..8], ptr::null_mut()).1.unwrap();
        assert_eq!(stream_c, stream_r, "C188 8-code stream differs");
        // sanity-check the header both libraries produced, so that the
        // corruptions below really do hit the fields they claim to
        assert_eq!(
            u32::from_ne_bytes(stream_c[0..4].try_into().unwrap()),
            0x5052_3253,
            "SERIALIZED_DATA_MAGIC"
        );
        assert_eq!(
            u32::from_ne_bytes(stream_c[8..12].try_into().unwrap()),
            0x0008_0801,
            "SERIALIZED_DATA_CONFIG (8-bit units, 64-bit pointers and PCRE2_SIZE)"
        );
        assert_eq!(
            i32::from_ne_bytes(stream_c[12..16].try_into().unwrap()),
            8,
            "number_of_codes in the header"
        );

        // same-library round trip, cross-library round trip, and clamping
        for (dec, enc_stream, tag) in [
            (c, &stream_c, "C->C"),
            (r, &stream_r, "R->R"),
            (r, &stream_c, "C->R"),
            (c, &stream_r, "R->C"),
        ] {
            for want in [1i32, 2, 8, 9, 20] {
                let mut out: Vec<Ptr> = vec![ptr::null_mut(); 24];
                let rc = (dec.pcre2_serialize_decode_8)(
                    out.as_mut_ptr(),
                    want,
                    enc_stream.as_ptr(),
                    ptr::null_mut(),
                );
                assert_eq!(rc, want.min(8), "C189 {} decode rc (want {})", tag, want);
                for i in 0..rc as usize {
                    assert!(!out[i].is_null());
                    for subj in subjects {
                        let a = match_probe(dec, out[i], subj);
                        // compare against the natively compiled pattern in the
                        // SAME library
                        let native = if std::ptr::eq(dec as *const Api, c as *const Api) {
                            codes_c[i]
                        } else {
                            codes_r[i]
                        };
                        let b = match_probe(dec, native, subj);
                        assert_eq!(
                            a, b,
                            "C189 {} decoded code {} behaves differently on {:?}",
                            tag, i, subj
                        );
                        n += 1;
                    }
                    (dec.pcre2_code_free_8)(out[i]);
                }
                n += 1;
            }
        }

        // NULL / bad count
        {
            let mut out: Vec<Ptr> = vec![ptr::null_mut(); 8];
            assert_eq!(
                (c.pcre2_serialize_decode_8)(out.as_mut_ptr(), 1, ptr::null(), ptr::null_mut()),
                (r.pcre2_serialize_decode_8)(out.as_mut_ptr(), 1, ptr::null(), ptr::null_mut()),
                "C189 bytes == NULL"
            );
            assert_eq!(
                (c.pcre2_serialize_decode_8)(ptr::null_mut(), 1, stream_c.as_ptr(), ptr::null_mut()),
                (r.pcre2_serialize_decode_8)(ptr::null_mut(), 1, stream_r.as_ptr(), ptr::null_mut()),
                "C189 codes == NULL"
            );
            for cnt in [0i32, -1, i32::MIN] {
                assert_eq!(
                    (c.pcre2_serialize_decode_8)(
                        out.as_mut_ptr(),
                        cnt,
                        stream_c.as_ptr(),
                        ptr::null_mut()
                    ),
                    (r.pcre2_serialize_decode_8)(
                        out.as_mut_ptr(),
                        cnt,
                        stream_r.as_ptr(),
                        ptr::null_mut()
                    ),
                    "C189 number_of_codes = {}",
                    cnt
                );
                n += 1;
            }
            assert_eq!(
                (c.pcre2_serialize_get_number_of_codes_8)(ptr::null()),
                (r.pcre2_serialize_get_number_of_codes_8)(ptr::null()),
                "C189 get_number_of_codes(NULL)"
            );
            n += 1;
        }

        // corrupted streams
        let code_block0 = SERIALIZED_HDR + TABLES_LENGTH;
        let corruptions: Vec<(&str, Box<dyn Fn(&mut Vec<u8>)>)> = vec![
            (
                "count=0",
                Box::new(|v: &mut Vec<u8>| v[12..16].copy_from_slice(&0i32.to_ne_bytes())),
            ),
            (
                "count=-3",
                Box::new(|v: &mut Vec<u8>| v[12..16].copy_from_slice(&(-3i32).to_ne_bytes())),
            ),
            (
                "bad magic",
                Box::new(|v: &mut Vec<u8>| v[0..4].copy_from_slice(&0x1234_5678u32.to_ne_bytes())),
            ),
            (
                "byte-swapped magic",
                Box::new(|v: &mut Vec<u8>| {
                    let m = u32::from_ne_bytes([v[0], v[1], v[2], v[3]]);
                    v[0..4].copy_from_slice(&m.swap_bytes().to_ne_bytes())
                }),
            ),
            (
                "bad version",
                Box::new(|v: &mut Vec<u8>| v[4..8].copy_from_slice(&0xffff_0001u32.to_ne_bytes())),
            ),
            (
                "bad config",
                Box::new(|v: &mut Vec<u8>| v[8..12].copy_from_slice(&0x0004_0810u32.to_ne_bytes())),
            ),
            // SERIALIZED_DATA_CONFIG == sizeof(PCRE2_UCHAR) | sizeof(void*)<<8 |
            // sizeof(PCRE2_SIZE)<<16, i.e. 0x080801 for this build
            (
                "config: 16-bit code units",
                Box::new(|v: &mut Vec<u8>| v[8..12].copy_from_slice(&0x0008_0802u32.to_ne_bytes())),
            ),
            (
                "config: 32-bit pointers",
                Box::new(|v: &mut Vec<u8>| v[8..12].copy_from_slice(&0x0008_0401u32.to_ne_bytes())),
            ),
            (
                "config: 32-bit PCRE2_SIZE",
                Box::new(|v: &mut Vec<u8>| v[8..12].copy_from_slice(&0x0004_0801u32.to_ne_bytes())),
            ),
            (
                "blocksize = sizeof(real_code)",
                Box::new(move |v: &mut Vec<u8>| {
                    let at = code_block0 + RC_BLOCKSIZE;
                    v[at..at + 8].copy_from_slice(&SIZEOF_REAL_CODE.to_ne_bytes())
                }),
            ),
            (
                "blocksize = 0",
                Box::new(move |v: &mut Vec<u8>| {
                    let at = code_block0 + RC_BLOCKSIZE;
                    v[at..at + 8].copy_from_slice(&0usize.to_ne_bytes())
                }),
            ),
            (
                "code body magic",
                Box::new(move |v: &mut Vec<u8>| {
                    let at = code_block0 + RC_MAGIC;
                    v[at..at + 4].copy_from_slice(&0xdead_beefu32.to_ne_bytes())
                }),
            ),
            (
                "name_entry_size = 132",
                Box::new(move |v: &mut Vec<u8>| {
                    let at = code_block0 + RC_NAME_ENTRY_SIZE;
                    v[at..at + 2].copy_from_slice(&132u16.to_ne_bytes())
                }),
            ),
            (
                "name_entry_size = 131",
                Box::new(move |v: &mut Vec<u8>| {
                    let at = code_block0 + RC_NAME_ENTRY_SIZE;
                    v[at..at + 2].copy_from_slice(&131u16.to_ne_bytes())
                }),
            ),
            (
                "name_count = 10001",
                Box::new(move |v: &mut Vec<u8>| {
                    let at = code_block0 + RC_NAME_COUNT;
                    v[at..at + 2].copy_from_slice(&10001u16.to_ne_bytes())
                }),
            ),
            (
                "name_count = 10000",
                Box::new(move |v: &mut Vec<u8>| {
                    let at = code_block0 + RC_NAME_COUNT;
                    v[at..at + 2].copy_from_slice(&10000u16.to_ne_bytes())
                }),
            ),
        ];
        for (label, f) in corruptions.iter() {
            let mut vc = stream_c.clone();
            let mut vr = stream_r.clone();
            f(&mut vc);
            f(&mut vr);
            assert_eq!(vc, vr, "C189 corruption {} produced different streams", label);
            let gc = (c.pcre2_serialize_get_number_of_codes_8)(vc.as_ptr());
            let gr = (r.pcre2_serialize_get_number_of_codes_8)(vr.as_ptr());
            assert_eq!(gc, gr, "C189 get_number_of_codes after {}", label);
            n += 1;
            for want in [1i32, 8] {
                let mut oc: Vec<Ptr> = vec![ptr::null_mut(); 12];
                let mut or: Vec<Ptr> = vec![ptr::null_mut(); 12];
                let rc1 =
                    (c.pcre2_serialize_decode_8)(oc.as_mut_ptr(), want, vc.as_ptr(), ptr::null_mut());
                let rc2 =
                    (r.pcre2_serialize_decode_8)(or.as_mut_ptr(), want, vr.as_ptr(), ptr::null_mut());
                assert_eq!(rc1, rc2, "C189 decode rc after {} (want {})", label, want);
                println!(
                    "  C189 corruption {:30} want={} -> get_number_of_codes={} decode={}",
                    label, want, gc, rc1
                );
                // whatever survived must be reported identically
                let nulls1: Vec<bool> = oc.iter().map(|p| p.is_null()).collect();
                let nulls2: Vec<bool> = or.iter().map(|p| p.is_null()).collect();
                assert_eq!(nulls1, nulls2, "C189 codes[] after {}", label);
                n += 1;
                if rc1 > 0 {
                    for i in 0..rc1 as usize {
                        for subj in subjects {
                            let a = match_probe(c, oc[i], subj);
                            let b = match_probe(r, or[i], subj);
                            assert_eq!(a, b, "C189 decoded code after {}", label);
                            n += 1;
                        }
                        (c.pcre2_code_free_8)(oc[i]);
                        (r.pcre2_code_free_8)(or[i]);
                    }
                }
            }
        }

        // an unaligned bytes pointer is deliberately tolerated
        {
            for skew in 1..4usize {
                let mut vc = vec![0u8; skew];
                vc.extend_from_slice(&stream_c);
                let mut vr = vec![0u8; skew];
                vr.extend_from_slice(&stream_r);
                let mut oc: Vec<Ptr> = vec![ptr::null_mut(); 12];
                let mut or: Vec<Ptr> = vec![ptr::null_mut(); 12];
                let rc1 = (c.pcre2_serialize_decode_8)(
                    oc.as_mut_ptr(),
                    3,
                    vc.as_ptr().add(skew),
                    ptr::null_mut(),
                );
                let rc2 = (r.pcre2_serialize_decode_8)(
                    or.as_mut_ptr(),
                    3,
                    vr.as_ptr().add(skew),
                    ptr::null_mut(),
                );
                assert_eq!(rc1, rc2, "C189 unaligned decode rc (skew {})", skew);
                assert_eq!(
                    (c.pcre2_serialize_get_number_of_codes_8)(vc.as_ptr().add(skew)),
                    (r.pcre2_serialize_get_number_of_codes_8)(vr.as_ptr().add(skew)),
                    "C189 unaligned get_number_of_codes"
                );
                n += 1;
                if rc1 > 0 {
                    for i in 0..rc1 as usize {
                        for subj in subjects {
                            assert_eq!(
                                match_probe(c, oc[i], subj),
                                match_probe(r, or[i], subj),
                                "C189 unaligned decoded code {}",
                                i
                            );
                            n += 1;
                        }
                        (c.pcre2_code_free_8)(oc[i]);
                        (r.pcre2_code_free_8)(or[i]);
                    }
                }
            }
        }

        // a gcontext whose malloc fails at code index 0 and at index > 0
        for budget in [0i64, 1, 2, 3] {
            let mut outs: Vec<(i32, Vec<bool>)> = Vec::new();
            for (api, stream) in [(c, &stream_c), (r, &stream_r)] {
                let gc =
                    (api.pcre2_general_context_create_8)(Some(t_malloc), Some(t_free), ptr::null_mut());
                let mut out: Vec<Ptr> = vec![ptr::null_mut(); 12];
                MALLOC_BUDGET.with(|x| x.set(budget));
                let rc =
                    (api.pcre2_serialize_decode_8)(out.as_mut_ptr(), 8, stream.as_ptr(), gc);
                MALLOC_BUDGET.with(|x| x.set(-1));
                let nulls: Vec<bool> = out.iter().map(|p| p.is_null()).collect();
                if rc > 0 {
                    for i in 0..rc as usize {
                        (api.pcre2_code_free_8)(out[i]);
                    }
                }
                (api.pcre2_general_context_free_8)(gc);
                outs.push((rc, nulls));
            }
            assert_eq!(
                outs[0], outs[1],
                "C189 decode with malloc budget {}",
                budget
            );
            n += 1;
        }

        for i in 0..codes_c.len() {
            (c.pcre2_code_free_8)(codes_c[i]);
            (r.pcre2_code_free_8)(codes_r[i]);
        }
    }
    println!("C188-C189: {} serialize comparisons", n);
}
