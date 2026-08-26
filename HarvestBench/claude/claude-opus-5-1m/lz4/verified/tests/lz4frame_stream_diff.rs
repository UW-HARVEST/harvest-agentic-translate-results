//! Differential tests for the lz4frame.c **STREAMING COMPRESSION** API.
//!
//! Scope (only these symbols):
//!   LZ4F_createCompressionContext, LZ4F_createCompressionContext_advanced,
//!   LZ4F_freeCompressionContext, LZ4F_compressBegin, LZ4F_compressBegin_internal,
//!   LZ4F_compressBegin_usingCDict, LZ4F_compressBegin_usingDict,
//!   LZ4F_compressBegin_usingDictOnce, LZ4F_compressUpdate, LZ4F_uncompressedUpdate,
//!   LZ4F_flush, LZ4F_compressEnd.
//!
//! `LZ4F_compressBound`, `LZ4F_createCDict`/`LZ4F_freeCDict` and the decompression
//! entry points are used only as *helpers* (sizing / round-trip validation).
//!
//! Every call goes through BOTH shared libraries' export tables, in lock-step:
//! after every single call the return value **and** the whole destination buffer
//! (pre-filled with the same 0xAA sentinel in both libraries) are compared.

mod common;

use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};
use std::ptr;

/// Fill byte used for BOTH destination buffers, so untouched bytes compare equal.
const SENTINEL: u8 = 0xAA;

// ---------------------------------------------------------------------------
// Signatures — verified against c_src/src/lz4frame.c + lz4frame.h
// ---------------------------------------------------------------------------

/// `LZ4F_errorCode_t LZ4F_createCompressionContext(LZ4F_cctx** , unsigned)`
type FnCreateCctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
/// `LZ4F_cctx* LZ4F_createCompressionContext_advanced(LZ4F_CustomMem, unsigned)`
type FnCreateCctxAdv = unsafe extern "C" fn(LZ4F_CustomMem, c_uint) -> *mut c_void;
/// `LZ4F_errorCode_t LZ4F_freeCompressionContext(LZ4F_cctx*)`
type FnFreeCctx = unsafe extern "C" fn(*mut c_void) -> usize;
/// `size_t LZ4F_compressBegin(cctx, dst, dstCapacity, prefs)`
type FnBegin =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_preferences_t) -> usize;
/// `size_t LZ4F_compressBegin_usingDict[Once](cctx, dst, dstCapacity, dict, dictSize, prefs)`
type FnBeginDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_preferences_t,
) -> usize;
/// `size_t LZ4F_compressBegin_usingCDict(cctx, dst, dstCapacity, cdict, prefs)`
type FnBeginCDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
/// `size_t LZ4F_compressBegin_internal(cctx, dst, dstCapacity, dict, dictSize, cdict, prefs)`
type FnBeginInternal = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const c_void,
    *const LZ4F_preferences_t,
) -> usize;
/// `size_t LZ4F_compressUpdate / LZ4F_uncompressedUpdate(cctx, dst, dstCap, src, srcSize, cOpt)`
type FnUpdate = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    usize,
    *const c_void,
    usize,
    *const LZ4F_compressOptions_t,
) -> usize;
/// `size_t LZ4F_flush / LZ4F_compressEnd(cctx, dst, dstCapacity, cOpt)`
type FnFlush =
    unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const LZ4F_compressOptions_t) -> usize;

// helpers only
type FnBound = unsafe extern "C" fn(usize, *const LZ4F_preferences_t) -> usize;
type FnCreateCDict = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnFreeCDict = unsafe extern "C" fn(*mut c_void);
type FnCreateDctx = unsafe extern "C" fn(*mut *mut c_void, c_uint) -> usize;
type FnFreeDctx = unsafe extern "C" fn(*mut c_void) -> usize;
type FnDecompress = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut usize,
    *const c_void,
    *mut usize,
    *const LZ4F_decompressOptions_t,
) -> usize;

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn describe(code: usize) -> String {
    if lz4f_is_error(code) {
        format!("ERROR({})", lz4f_error_code(code))
    } else {
        format!("{}", code)
    }
}

#[track_caller]
fn same_ret(label: &str, c: usize, r: usize) {
    if c != r {
        panic!(
            "{}: return mismatch\n  C   = {} (raw 0x{:x})\n  Rust= {} (raw 0x{:x})",
            label,
            describe(c),
            c,
            describe(r),
            r
        );
    }
}

#[track_caller]
fn expect_err(label: &str, c: usize, r: usize, want: i32) {
    same_ret(label, c, r);
    assert!(
        lz4f_is_error(c),
        "{}: expected error {} but call succeeded with {}",
        label,
        want,
        c
    );
    assert_eq!(
        lz4f_error_code(c),
        want,
        "{}: wrong error code (raw 0x{:x})",
        label,
        c
    );
}

/// `LZ4F_compressBound` from both libraries (cross-checked), used purely for sizing.
fn bound_both(src_size: usize, prefs: &LZ4F_preferences_t) -> usize {
    let (cf, rf) = both::<FnBound>("LZ4F_compressBound");
    unsafe {
        let a = cf(src_size, prefs as *const _);
        let b = rf(src_size, prefs as *const _);
        assert_eq!(a, b, "LZ4F_compressBound({}) mismatch", src_size);
        a
    }
}

/// Mirrors `LZ4F_getBlockSize()` for the *valid* ids only (test-local, no FFI).
fn block_size(bsid: c_int) -> usize {
    match bsid {
        0 | 4 => 64 * 1024,
        5 => 256 * 1024,
        6 => 1024 * 1024,
        7 => 4 * 1024 * 1024,
        other => panic!("block_size: invalid blockSizeID {}", other),
    }
}

fn base_prefs(bsid: c_int, mode: c_int, level: c_int, autoflush: c_uint) -> LZ4F_preferences_t {
    let mut p = LZ4F_preferences_t::default();
    p.frameInfo.blockSizeID = bsid;
    p.frameInfo.blockMode = mode;
    p.compressionLevel = level;
    p.autoFlush = autoflush;
    p
}

// ---------------------------------------------------------------------------
// Round-trip validation (decompression side used only as an oracle)
// ---------------------------------------------------------------------------

fn decompress_with(is_c: bool, frame: &[u8], label: &str) -> Result<Vec<u8>, i32> {
    let (cc, rc) = both::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (cfr, rfr) = both::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let (cd, rd) = both::<FnDecompress>("LZ4F_decompress");
    let (create, free_fn, dec) = if is_c { (cc, cfr, cd) } else { (rc, rfr, rd) };

    unsafe {
        let mut dctx: *mut c_void = ptr::null_mut();
        let cr = create(&mut dctx, LZ4F_VERSION);
        assert_eq!(cr, 0, "{}: createDecompressionContext failed", label);

        let mut out: Vec<u8> = Vec::new();
        let mut tmp = vec![0u8; 1 << 16];
        let mut spos = 0usize;
        let mut outcome: Result<Vec<u8>, i32>;
        loop {
            let mut dsz = tmp.len();
            let mut ssz = frame.len() - spos;
            let r = dec(
                dctx,
                tmp.as_mut_ptr() as *mut c_void,
                &mut dsz,
                frame.as_ptr().add(spos) as *const c_void,
                &mut ssz,
                ptr::null(),
            );
            if lz4f_is_error(r) {
                outcome = Err(lz4f_error_code(r));
                break;
            }
            out.extend_from_slice(&tmp[..dsz]);
            spos += ssz;
            if r == 0 {
                outcome = Ok(std::mem::take(&mut out));
                break;
            }
            if ssz == 0 && dsz == 0 {
                outcome = Err(-1); // no forward progress
                break;
            }
        }
        free_fn(dctx);
        outcome
    }
}

/// The produced frame must decode back to `original` in BOTH libraries.
fn assert_round_trip(frame: &[u8], original: &[u8], label: &str) {
    for (is_c, tag) in [(true, "C"), (false, "Rust")] {
        match decompress_with(is_c, frame, label) {
            Ok(v) => assert_bytes_eq(
                &format!("{}: {} LZ4F_decompress round-trip payload", label, tag),
                original,
                &v,
            ),
            Err(e) => panic!(
                "{}: {} round-trip failed, LZ4F_decompress error {} (frame {} bytes: {})",
                label,
                tag,
                e,
                frame.len(),
                hexdump(frame, 64)
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// The lock-step session driver
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Begin {
    /// `LZ4F_compressBegin`
    Plain,
    /// `LZ4F_compressBegin_usingDict`
    Dict,
    /// `LZ4F_compressBegin_usingDictOnce`
    DictOnce,
    /// `LZ4F_compressBegin_usingCDict`
    CDict,
    /// `LZ4F_compressBegin_internal` with dict==NULL, cdict==NULL
    Internal,
    /// `LZ4F_compressBegin_internal` with a raw dict
    InternalDict,
    /// `LZ4F_compressBegin_internal` with a CDict
    InternalCDict,
}

impl Begin {
    fn name(self) -> &'static str {
        match self {
            Begin::Plain => "LZ4F_compressBegin",
            Begin::Dict => "LZ4F_compressBegin_usingDict",
            Begin::DictOnce => "LZ4F_compressBegin_usingDictOnce",
            Begin::CDict => "LZ4F_compressBegin_usingCDict",
            Begin::Internal => "LZ4F_compressBegin_internal(-,-)",
            Begin::InternalDict => "LZ4F_compressBegin_internal(dict)",
            Begin::InternalCDict => "LZ4F_compressBegin_internal(cdict)",
        }
    }
    fn uses_cdict(self) -> bool {
        matches!(self, Begin::CDict | Begin::InternalCDict)
    }
    fn uses_dict(self) -> bool {
        matches!(self, Begin::Dict | Begin::DictOnce | Begin::InternalDict)
    }
}

#[derive(Clone, Copy)]
struct BeginSpec<'a> {
    kind: Begin,
    /// `None` => pass a NULL dict pointer with size 0.
    dict: Option<&'a [u8]>,
    /// Lie about the dictionary size (error rows).
    size_override: Option<usize>,
    /// Pass a NULL `preferencesPtr`.
    null_prefs: bool,
}

impl<'a> BeginSpec<'a> {
    fn plain() -> Self {
        BeginSpec { kind: Begin::Plain, dict: None, size_override: None, null_prefs: false }
    }
    fn of(kind: Begin) -> Self {
        BeginSpec { kind, dict: None, size_override: None, null_prefs: false }
    }
    fn with_dict(kind: Begin, dict: &'a [u8]) -> Self {
        BeginSpec { kind, dict: Some(dict), size_override: None, null_prefs: false }
    }
    fn null_prefs(mut self) -> Self {
        self.null_prefs = true;
        self
    }
}

/// One step of a streaming session.
#[derive(Clone, Copy, Debug)]
enum Op {
    /// `LZ4F_compressUpdate` with N bytes
    Upd(usize),
    /// `LZ4F_uncompressedUpdate` with N bytes
    Unc(usize),
    /// `LZ4F_flush`
    Flush,
}

struct Sess {
    c: *mut c_void,
    r: *mut c_void,
    ccd: *mut c_void,
    rcd: *mut c_void,
    prefs: LZ4F_preferences_t,
    cout: Vec<u8>,
    rout: Vec<u8>,
    cbuf: Vec<u8>,
    rbuf: Vec<u8>,
    label: String,
    step: usize,
    /// Header length returned by the last successful `compressBegin*`.
    header_len: usize,
}

impl Sess {
    fn new(label: &str, prefs: LZ4F_preferences_t) -> Sess {
        let (cf, rf) = both::<FnCreateCctx>("LZ4F_createCompressionContext");
        unsafe {
            let mut c: *mut c_void = ptr::null_mut();
            let mut r: *mut c_void = ptr::null_mut();
            let cr = cf(&mut c, LZ4F_VERSION);
            let rr = rf(&mut r, LZ4F_VERSION);
            same_ret(&format!("{}: LZ4F_createCompressionContext", label), cr, rr);
            assert_eq!(cr, 0, "{}: createCompressionContext failed", label);
            assert!(!c.is_null() && !r.is_null(), "{}: NULL cctx", label);
            Sess {
                c,
                r,
                ccd: ptr::null_mut(),
                rcd: ptr::null_mut(),
                prefs,
                cout: Vec::new(),
                rout: Vec::new(),
                cbuf: Vec::new(),
                rbuf: Vec::new(),
                label: label.to_string(),
                step: 0,
                header_len: 0,
            }
        }
    }

    /// Extra bytes allocated BEYOND the `dstCapacity` we hand to the library.
    ///
    /// `LZ4F_compressUpdateImpl` checks `dstCapacity` BEFORE calling
    /// `LZ4F_flush()` on a blockCompressMode switch and then advances `dstPtr`
    /// by the flushed byte count WITHOUT deducting it from the remaining budget
    /// (lz4frame.c:1006-1016). So on a compressed->uncompressed switch the C
    /// itself can write — and report — MORE than `dstCapacity`; observed:
    /// `LZ4F_uncompressedUpdate(srcSize=65536, dstCapacity=65560) == 65574`.
    /// Without slack that is a heap overflow inside the C library, which aborts
    /// the process before any comparison can happen. With slack, the overrun
    /// lands in our own allocation and stays fully comparable between C and Rust.
    const SLACK: usize = 1 << 17;

    fn prep(&mut self, cap: usize) {
        let total = cap + Self::SLACK;
        if self.cbuf.len() < total {
            self.cbuf.resize(total, SENTINEL);
            self.rbuf.resize(total, SENTINEL);
        }
        self.cbuf[..total].fill(SENTINEL);
        self.rbuf[..total].fill(SENTINEL);
    }

    fn cmp_dst(&self, cap: usize, ctx: &str) {
        // Compare the usable region AND the slack region: both sides were
        // pre-filled with the same sentinel, so this also detects a divergence
        // in how far past `dstCapacity` each implementation wrote.
        let total = cap + Self::SLACK;
        assert_bytes_eq(ctx, &self.cbuf[..total], &self.rbuf[..total]);
    }

    /// Append the accepted output of the last call to the accumulated frames.
    fn take(&mut self, n: usize) {
        let c = self.cbuf[..n].to_vec();
        let r = self.rbuf[..n].to_vec();
        self.cout.extend_from_slice(&c);
        self.rout.extend_from_slice(&r);
    }

    fn begin(&mut self, spec: BeginSpec) -> usize {
        self.begin_cap(spec, LZ4F_HEADER_SIZE_MAX + 24)
    }

    fn begin_cap(&mut self, spec: BeginSpec, cap: usize) -> usize {
        self.step += 1;
        let prefs = self.prefs;
        let pp: *const LZ4F_preferences_t =
            if spec.null_prefs { ptr::null() } else { &prefs as *const _ };
        let (dptr, dlen) = match spec.dict {
            Some(d) => (d.as_ptr() as *const c_void, d.len()),
            None => (ptr::null::<c_void>(), 0usize),
        };
        let dlen = spec.size_override.unwrap_or(dlen);

        if spec.kind.uses_cdict() {
            let d = spec.dict.expect("cdict kinds need a dictionary");
            let (cc, rc) = both::<FnCreateCDict>("LZ4F_createCDict");
            unsafe {
                self.ccd = cc(d.as_ptr() as *const c_void, d.len());
                self.rcd = rc(d.as_ptr() as *const c_void, d.len());
            }
            assert_eq!(
                self.ccd.is_null(),
                self.rcd.is_null(),
                "{}: LZ4F_createCDict nullness",
                self.label
            );
            assert!(!self.ccd.is_null(), "{}: LZ4F_createCDict returned NULL", self.label);
        }

        self.prep(cap);
        let label = format!("{} [step {}] {}", self.label, self.step, spec.kind.name());
        let (cn, rn) = unsafe {
            let cd = self.cbuf.as_mut_ptr() as *mut c_void;
            let rd = self.rbuf.as_mut_ptr() as *mut c_void;
            match spec.kind {
                Begin::Plain => {
                    let (cf, rf) = both::<FnBegin>("LZ4F_compressBegin");
                    (cf(self.c, cd, cap, pp), rf(self.r, rd, cap, pp))
                }
                Begin::Dict => {
                    let (cf, rf) = both::<FnBeginDict>("LZ4F_compressBegin_usingDict");
                    (
                        cf(self.c, cd, cap, dptr, dlen, pp),
                        rf(self.r, rd, cap, dptr, dlen, pp),
                    )
                }
                Begin::DictOnce => {
                    let (cf, rf) = both::<FnBeginDict>("LZ4F_compressBegin_usingDictOnce");
                    (
                        cf(self.c, cd, cap, dptr, dlen, pp),
                        rf(self.r, rd, cap, dptr, dlen, pp),
                    )
                }
                Begin::CDict => {
                    let (cf, rf) = both::<FnBeginCDict>("LZ4F_compressBegin_usingCDict");
                    (
                        cf(self.c, cd, cap, self.ccd as *const c_void, pp),
                        rf(self.r, rd, cap, self.rcd as *const c_void, pp),
                    )
                }
                Begin::Internal => {
                    let (cf, rf) = both::<FnBeginInternal>("LZ4F_compressBegin_internal");
                    (
                        cf(self.c, cd, cap, ptr::null(), 0, ptr::null(), pp),
                        rf(self.r, rd, cap, ptr::null(), 0, ptr::null(), pp),
                    )
                }
                Begin::InternalDict => {
                    let (cf, rf) = both::<FnBeginInternal>("LZ4F_compressBegin_internal");
                    (
                        cf(self.c, cd, cap, dptr, dlen, ptr::null(), pp),
                        rf(self.r, rd, cap, dptr, dlen, ptr::null(), pp),
                    )
                }
                Begin::InternalCDict => {
                    let (cf, rf) = both::<FnBeginInternal>("LZ4F_compressBegin_internal");
                    (
                        cf(self.c, cd, cap, ptr::null(), 0, self.ccd as *const c_void, pp),
                        rf(self.r, rd, cap, ptr::null(), 0, self.rcd as *const c_void, pp),
                    )
                }
            }
        };
        same_ret(&label, cn, rn);
        self.cmp_dst(cap, &format!("{}: dst buffer", label));
        if !lz4f_is_error(cn) {
            assert!(
                cn >= LZ4F_HEADER_SIZE_MIN && cn <= LZ4F_HEADER_SIZE_MAX,
                "{}: implausible header size {}",
                label,
                cn
            );
            self.header_len = cn;
            self.take(cn);
        }
        cn
    }

    fn update(&mut self, src: &[u8], copts: Option<&LZ4F_compressOptions_t>) -> usize {
        let cap = bound_both(src.len(), &self.prefs) + 16;
        self.update_cap(false, src, copts, cap)
    }

    fn unc_update(&mut self, src: &[u8], copts: Option<&LZ4F_compressOptions_t>) -> usize {
        let cap = bound_both(src.len(), &self.prefs).max(src.len()) + 16;
        self.update_cap(true, src, copts, cap)
    }

    fn update_cap(
        &mut self,
        unc: bool,
        src: &[u8],
        copts: Option<&LZ4F_compressOptions_t>,
        cap: usize,
    ) -> usize {
        self.step += 1;
        let name = if unc { "LZ4F_uncompressedUpdate" } else { "LZ4F_compressUpdate" };
        let (cf, rf) = both::<FnUpdate>(name);
        let op: *const LZ4F_compressOptions_t =
            copts.map(|o| o as *const _).unwrap_or(ptr::null());
        self.prep(cap);
        let label = format!(
            "{} [step {}] {}(srcSize={}, dstCapacity={})",
            self.label, self.step, name, src.len(), cap
        );
        let (cn, rn) = unsafe {
            let sp = src.as_ptr() as *const c_void;
            (
                cf(self.c, self.cbuf.as_mut_ptr() as *mut c_void, cap, sp, src.len(), op),
                rf(self.r, self.rbuf.as_mut_ptr() as *mut c_void, cap, sp, src.len(), op),
            )
        };
        same_ret(&label, cn, rn);
        self.cmp_dst(cap, &format!("{}: dst buffer", label));
        if !lz4f_is_error(cn) {
            // NOTE: `cn > cap` is REACHABLE in the C on a blockCompressMode
            // switch (see `Sess::SLACK`). We therefore only require that C and
            // Rust report the SAME count (already asserted by `same_ret`) and
            // that it stays inside our slack-padded allocation.
            assert!(
                cn <= cap + Self::SLACK,
                "{}: returned {} beyond capacity {} + slack {}",
                label, cn, cap, Self::SLACK
            );
            self.take(cn);
        }
        cn
    }

    fn flush(&mut self, copts: Option<&LZ4F_compressOptions_t>) -> usize {
        let cap = bound_both(0, &self.prefs) + 16;
        self.flush_cap(copts, cap)
    }

    fn flush_cap(&mut self, copts: Option<&LZ4F_compressOptions_t>, cap: usize) -> usize {
        self.step += 1;
        let (cf, rf) = both::<FnFlush>("LZ4F_flush");
        let op: *const LZ4F_compressOptions_t =
            copts.map(|o| o as *const _).unwrap_or(ptr::null());
        self.prep(cap);
        let label =
            format!("{} [step {}] LZ4F_flush(dstCapacity={})", self.label, self.step, cap);
        let (cn, rn) = unsafe {
            (
                cf(self.c, self.cbuf.as_mut_ptr() as *mut c_void, cap, op),
                rf(self.r, self.rbuf.as_mut_ptr() as *mut c_void, cap, op),
            )
        };
        same_ret(&label, cn, rn);
        self.cmp_dst(cap, &format!("{}: dst buffer", label));
        if !lz4f_is_error(cn) {
            // NOTE: `cn > cap` is REACHABLE in the C on a blockCompressMode
            // switch (see `Sess::SLACK`). We therefore only require that C and
            // Rust report the SAME count (already asserted by `same_ret`) and
            // that it stays inside our slack-padded allocation.
            assert!(
                cn <= cap + Self::SLACK,
                "{}: returned {} beyond capacity {} + slack {}",
                label, cn, cap, Self::SLACK
            );
            self.take(cn);
        }
        cn
    }

    fn end(&mut self, copts: Option<&LZ4F_compressOptions_t>) -> usize {
        let cap = bound_both(0, &self.prefs) + 16;
        self.end_cap(copts, cap)
    }

    fn end_cap(&mut self, copts: Option<&LZ4F_compressOptions_t>, cap: usize) -> usize {
        self.step += 1;
        let (cf, rf) = both::<FnFlush>("LZ4F_compressEnd");
        let op: *const LZ4F_compressOptions_t =
            copts.map(|o| o as *const _).unwrap_or(ptr::null());
        self.prep(cap);
        let label =
            format!("{} [step {}] LZ4F_compressEnd(dstCapacity={})", self.label, self.step, cap);
        let (cn, rn) = unsafe {
            (
                cf(self.c, self.cbuf.as_mut_ptr() as *mut c_void, cap, op),
                rf(self.r, self.rbuf.as_mut_ptr() as *mut c_void, cap, op),
            )
        };
        same_ret(&label, cn, rn);
        self.cmp_dst(cap, &format!("{}: dst buffer", label));
        if !lz4f_is_error(cn) {
            assert!(cn >= 4, "{}: compressEnd returned {} (< 4)", label, cn);
            self.take(cn);
        }
        cn
    }

    /// Feed `input` through `ops`, asserting after every call.
    fn drive(&mut self, input: &[u8], ops: &[Op], copts: Option<&LZ4F_compressOptions_t>) {
        let mut pos = 0usize;
        for op in ops {
            match *op {
                Op::Upd(n) => {
                    assert!(pos + n <= input.len(), "{}: op overruns input", self.label);
                    self.update(&input[pos..pos + n], copts);
                    pos += n;
                }
                Op::Unc(n) => {
                    assert!(pos + n <= input.len(), "{}: op overruns input", self.label);
                    self.unc_update(&input[pos..pos + n], copts);
                    pos += n;
                }
                Op::Flush => {
                    self.flush(copts);
                }
            }
        }
        assert_eq!(pos, input.len(), "{}: ops did not consume all input", self.label);
    }

    /// Both accumulated frames must be byte-identical; returns the frame.
    fn frame(&self) -> Vec<u8> {
        assert_bytes_eq(
            &format!("{}: WHOLE FRAME", self.label),
            &self.cout,
            &self.rout,
        );
        self.cout.clone()
    }

    fn reset_frame(&mut self) {
        self.cout.clear();
        self.rout.clear();
    }
}

impl Drop for Sess {
    fn drop(&mut self) {
        let (cf, rf) = both::<FnFreeCctx>("LZ4F_freeCompressionContext");
        unsafe {
            let a = cf(self.c);
            let b = rf(self.r);
            assert_eq!(a, b, "{}: LZ4F_freeCompressionContext return", self.label);
            assert_eq!(a, 0, "{}: LZ4F_freeCompressionContext != OK", self.label);
            if !self.ccd.is_null() {
                let (cfd, rfd) = both::<FnFreeCDict>("LZ4F_freeCDict");
                cfd(self.ccd);
                rfd(self.rcd);
            }
        }
    }
}

/// Full pipeline in one shot: create -> begin -> ops -> end -> free.
fn run_frame(
    label: &str,
    prefs: &LZ4F_preferences_t,
    spec: BeginSpec,
    input: &[u8],
    ops: &[Op],
    copts: Option<&LZ4F_compressOptions_t>,
    round_trip: bool,
) -> Vec<u8> {
    let mut s = Sess::new(label, *prefs);
    let hdr = s.begin(spec);
    assert!(!lz4f_is_error(hdr), "{}: compressBegin failed: {}", label, describe(hdr));
    s.drive(input, ops, copts);
    let e = s.end(copts);
    assert!(!lz4f_is_error(e), "{}: compressEnd failed: {}", label, describe(e));
    let frame = s.frame();
    if round_trip {
        assert_round_trip(&frame, input, label);
    }
    frame
}

// ---------------------------------------------------------------------------
// Chunking plans
// ---------------------------------------------------------------------------

fn ops_single(total: usize) -> Vec<Op> {
    vec![Op::Upd(total)]
}

fn ops_fixed(total: usize, chunk: usize) -> Vec<Op> {
    let chunk = chunk.max(1);
    let mut v = Vec::new();
    let mut p = 0;
    while p < total {
        let n = chunk.min(total - p);
        v.push(Op::Upd(n));
        p += n;
    }
    if v.is_empty() {
        v.push(Op::Upd(0));
    }
    v
}

fn ops_random(total: usize, rng: &mut Rng, lo: usize, hi: usize) -> Vec<Op> {
    let mut v = Vec::new();
    let mut p = 0;
    while p < total {
        let n = rng.range(lo, hi).min(total - p);
        v.push(Op::Upd(n));
        p += n;
    }
    if v.is_empty() {
        v.push(Op::Upd(0));
    }
    v
}

/// Fixed chunks with zero-length updates interleaved before/after each real one.
fn ops_with_zeros(total: usize, chunk: usize) -> Vec<Op> {
    let mut v = vec![Op::Upd(0)];
    for op in ops_fixed(total, chunk) {
        v.push(op);
        v.push(Op::Upd(0));
    }
    v.push(Op::Upd(0));
    v
}

/// Fixed chunks with a `LZ4F_flush` after every `k`-th update (plus a double flush).
fn ops_flush_every(total: usize, chunk: usize, k: usize) -> Vec<Op> {
    let mut v = Vec::new();
    for (i, op) in ops_fixed(total, chunk).into_iter().enumerate() {
        v.push(op);
        if (i + 1) % k == 0 {
            v.push(Op::Flush);
            if i == 0 {
                v.push(Op::Flush); // flush twice in a row
            }
        }
    }
    v
}

/// One big chunk, then single bytes — crosses a block boundary cheaply.
fn ops_prefix_then_bytes(total: usize, prefix: usize) -> Vec<Op> {
    let prefix = prefix.min(total);
    let mut v = vec![Op::Upd(prefix)];
    for _ in prefix..total {
        v.push(Op::Upd(1));
    }
    v
}

// ===========================================================================
// 1. Context lifecycle
// ===========================================================================

#[test]
fn frame_stream_context_create_free() {
    let (cc, rc) = both::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cca, rca) = both::<FnCreateCctxAdv>("LZ4F_createCompressionContext_advanced");
    let (cfr, rfr) = both::<FnFreeCctx>("LZ4F_freeCompressionContext");

    unsafe {
        // --- LZ4F_createCompressionContext(NULL, version) -> parameter_null
        let cn = cc(ptr::null_mut(), LZ4F_VERSION);
        let rn = rc(ptr::null_mut(), LZ4F_VERSION);
        expect_err(
            "LZ4F_createCompressionContext(NULL)",
            cn,
            rn,
            err::ERROR_parameter_null,
        );

        // --- LZ4F_freeCompressionContext(NULL) -> OK_NoError
        let cf0 = cfr(ptr::null_mut());
        let rf0 = rfr(ptr::null_mut());
        same_ret("LZ4F_freeCompressionContext(NULL)", cf0, rf0);
        assert_eq!(cf0, err::OK_NoError as usize);

        // --- the `version` argument is never validated by the C code
        for v in [0u32, 1, 99, LZ4F_VERSION, 101, 1000, u32::MAX] {
            let mut cp: *mut c_void = ptr::null_mut();
            let mut rp: *mut c_void = ptr::null_mut();
            let a = cc(&mut cp, v);
            let b = rc(&mut rp, v);
            same_ret(&format!("LZ4F_createCompressionContext(version={})", v), a, b);
            assert_eq!(a, 0, "version {} unexpectedly rejected", v);
            assert!(!cp.is_null() && !rp.is_null());
            // out-pointer must be written even though the version is bogus
            same_ret("free after create", cfr(cp), rfr(rp));

            // ..._advanced with the default custom-mem
            let ca = cca(LZ4F_CustomMem::default(), v);
            let ra = rca(LZ4F_CustomMem::default(), v);
            assert_eq!(
                ca.is_null(),
                ra.is_null(),
                "LZ4F_createCompressionContext_advanced(version={}) nullness",
                v
            );
            assert!(!ca.is_null());
            same_ret("free after create_advanced", cfr(ca), rfr(ra));
        }
    }

    // A context created with a bogus version still compresses identically.
    let mut rng = Rng::new(0x51E5_0001);
    let input = gen_text(&mut rng, 5000);
    let prefs = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
    let (cca, rca) = both::<FnCreateCctxAdv>("LZ4F_createCompressionContext_advanced");
    unsafe {
        let c = cca(LZ4F_CustomMem::default(), 7);
        let r = rca(LZ4F_CustomMem::default(), 7);
        assert!(!c.is_null() && !r.is_null());
        let mut s = Sess {
            c,
            r,
            ccd: ptr::null_mut(),
            rcd: ptr::null_mut(),
            prefs,
            cout: Vec::new(),
            rout: Vec::new(),
            cbuf: Vec::new(),
            rbuf: Vec::new(),
            label: "advanced-cctx version=7".to_string(),
            step: 0,
            header_len: 0,
        };
        s.begin(BeginSpec::plain());
        s.drive(&input, &ops_fixed(input.len(), 700), None);
        s.end(None);
        let f = s.frame();
        assert_round_trip(&f, &input, "advanced-cctx version=7");
    }
}

// ===========================================================================
// 2. Preference cross-product
// ===========================================================================

#[test]
fn frame_stream_preference_matrix() {
    // Every blockSizeID x blockMode x blockChecksumFlag x contentChecksumFlag
    // combination, with the remaining axes rotated so each value appears.
    const BSIDS: [c_int; 5] = [0, LZ4F_max64KB, LZ4F_max256KB, LZ4F_max1MB, LZ4F_max4MB];
    const LEVELS: [c_int; 7] = [0, 1, 2, 3, 9, 12, -1];
    const SIZES: [usize; 6] = [0, 1, 100, 5000, 70000, 200000];

    let mut rng = Rng::new(0x0BAD_F00D_1234_5678);
    let mut i = 0usize;
    for &bsid in BSIDS.iter() {
        for &mode in [LZ4F_blockLinked, LZ4F_blockIndependent].iter() {
            for &bchk in [LZ4F_noBlockChecksum, LZ4F_blockChecksumEnabled].iter() {
                for &cchk in [LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled].iter() {
                    let level = LEVELS[i % LEVELS.len()];
                    let total = SIZES[i % SIZES.len()];
                    let autoflush = ((i / 2) % 2) as c_uint;
                    let favor = ((i / 3) % 2) as c_uint;
                    let with_content_size = i % 2 == 0;
                    let with_dict_id = i % 3 == 0;
                    let skippable = i % 4 == 0;
                    let stable = (i / 5) % 2 == 1;
                    let null_copts = i % 7 == 0;
                    let shape = i % N_SHAPES;

                    let mut p = base_prefs(bsid, mode, level, autoflush);
                    p.frameInfo.blockChecksumFlag = bchk;
                    p.frameInfo.contentChecksumFlag = cchk;
                    p.favorDecSpeed = favor;
                    p.frameInfo.contentSize = if with_content_size { total as u64 } else { 0 };
                    p.frameInfo.dictID = if with_dict_id { 0xDEAD_BEEF } else { 0 };
                    p.frameInfo.frameType =
                        if skippable { LZ4F_skippableFrame } else { LZ4F_frame };
                    if i % 11 == 0 {
                        p.reserved = [0xDEAD, 0xBEEF, 0xCAFE]; // must be ignored
                    }

                    let mut copts = LZ4F_compressOptions_t::default();
                    copts.stableSrc = if stable { 1 } else { 0 };
                    if i % 13 == 0 {
                        copts.reserved = [1, 2, 3]; // must be ignored
                    }
                    let co: Option<&LZ4F_compressOptions_t> =
                        if null_copts { None } else { Some(&copts) };

                    let input = gen_shape(&mut rng, shape, total);
                    let bs = block_size(bsid);
                    let ops = match i % 6 {
                        0 => ops_single(total),
                        1 => ops_fixed(total, 1000),
                        2 => ops_fixed(total, 65535),
                        3 => ops_random(total, &mut rng, 1, 40_000),
                        4 => ops_with_zeros(total, 777),
                        _ => ops_flush_every(total, (bs / 3).max(1), 2),
                    };

                    let label = format!(
                        "matrix#{} bsid={} mode={} bchk={} cchk={} lvl={} af={} favor={} \
                         cs={} dictID={} ft={} stable={} nullOpts={} shape={} total={} pattern={}",
                        i,
                        bsid,
                        mode,
                        bchk,
                        cchk,
                        level,
                        autoflush,
                        favor,
                        p.frameInfo.contentSize,
                        p.frameInfo.dictID,
                        p.frameInfo.frameType,
                        copts.stableSrc,
                        null_copts,
                        shape_name(shape),
                        total,
                        i % 6
                    );
                    run_frame(&label, &p, BeginSpec::plain(), &input, &ops, co, true);
                    i += 1;
                }
            }
        }
    }
    assert_eq!(i, 40, "expected the full 5x2x2x2 matrix");
}

// ===========================================================================
// 3. Compression levels
// ===========================================================================

#[test]
fn frame_stream_compression_levels() {
    // LZ4HC_CLEVEL_MIN == 2 in this tree: 0/1 = LZ4 fast, 2 = lz4mid,
    // 3..9 = hashChain, 10..12 = optimal. Extremes exercise the clamps.
    const LEVELS: [c_int; 14] = [
        c_int::MIN,
        -65537,
        -1,
        0,
        1,
        2,
        3,
        6,
        9,
        10,
        11,
        12,
        13,
        c_int::MAX,
    ];
    let mut rng = Rng::new(0x1E7E_1234u64 ^ 0x9E37_79B9);
    for (li, &level) in LEVELS.iter().enumerate() {
        // Optimal-parser levels are slow: keep those inputs small.
        let hc_slow = level >= LZ4HC_CLEVEL_OPT_MIN || level < 0;
        let total = if hc_slow { 24_000 } else { 150_000 };
        let input = gen_shape(&mut rng, li % N_SHAPES, total);

        for (ci, &(mode, af, cchk, bchk, favor)) in [
            (LZ4F_blockLinked, 0u32, 1, 1, 0u32),
            (LZ4F_blockIndependent, 1u32, 0, 1, 1u32),
            (LZ4F_blockLinked, 1u32, 1, 0, 1u32),
        ]
        .iter()
        .enumerate()
        {
            let mut p = base_prefs(LZ4F_max64KB, mode, level, af);
            p.frameInfo.contentChecksumFlag = cchk;
            p.frameInfo.blockChecksumFlag = bchk;
            p.favorDecSpeed = favor;

            let ops = match ci {
                0 => ops_fixed(total, 9_000),
                1 => ops_single(total),
                _ => ops_flush_every(total, 20_000, 1),
            };
            let label = format!(
                "level={} cfg={} mode={} af={} cchk={} bchk={} favor={} total={}",
                level, ci, mode, af, cchk, bchk, favor, total
            );
            run_frame(&label, &p, BeginSpec::plain(), &input, &ops, None, true);
        }
    }
}

// ===========================================================================
// 4. Chunking patterns — the tmpBuff / tmpIn / tmpInSize state machine
// ===========================================================================

#[test]
fn frame_stream_chunking_patterns() {
    let bs = block_size(LZ4F_max64KB);
    let mut rng = Rng::new(0xC0FF_EE00_1234_ABCD);

    // Fixed chunk sizes, iteration-capped so byte-at-a-time stays cheap.
    const CHUNKS: [usize; 8] = [1, 2, 3, 7, 16, 100, 1000, 65535];
    let big_total = 3 * 65536 + 1234;

    for &(mode, af) in [
        (LZ4F_blockLinked, 0u32),
        (LZ4F_blockLinked, 1u32),
        (LZ4F_blockIndependent, 0u32),
        (LZ4F_blockIndependent, 1u32),
    ]
    .iter()
    {
        for &level in [1, 3].iter() {
            let mut p = base_prefs(LZ4F_max64KB, mode, level, af);
            p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
            p.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;

            for &chunk in CHUNKS.iter() {
                let total = big_total.min(chunk.saturating_mul(2500)).max(chunk);
                let input = gen_selfref(&mut rng, total);
                let label = format!(
                    "chunk={} mode={} af={} lvl={} total={}",
                    chunk, mode, af, level, total
                );
                run_frame(
                    &label,
                    &p,
                    BeginSpec::plain(),
                    &input,
                    &ops_fixed(total, chunk),
                    None,
                    true,
                );
            }

            // Chunk sizes landing exactly on / around the block boundary.
            for delta in [-1i64, 0, 1] {
                let chunk = (bs as i64 + delta) as usize;
                let total = chunk * 3 + 5;
                let input = gen_mixed(&mut rng, total);
                let label = format!(
                    "boundary chunk={}({:+}) mode={} af={} lvl={} total={}",
                    chunk, delta, mode, af, level, total
                );
                run_frame(
                    &label,
                    &p,
                    BeginSpec::plain(),
                    &input,
                    &ops_fixed(total, chunk),
                    None,
                    true,
                );
            }

            // Prefix that lands 1 byte short of the boundary, then single bytes:
            // crosses the boundary with byte-at-a-time updates.
            for prefix_delta in [-2i64, -1, 0, 1] {
                let prefix = (bs as i64 + prefix_delta) as usize;
                let total = prefix + 6;
                let input = gen_text(&mut rng, total);
                let label = format!(
                    "prefix={}({:+}) then bytes mode={} af={} lvl={}",
                    prefix, prefix_delta, mode, af, level
                );
                run_frame(
                    &label,
                    &p,
                    BeginSpec::plain(),
                    &input,
                    &ops_prefix_then_bytes(total, prefix),
                    None,
                    true,
                );
            }

            // Random chunk sizes.
            for (k, &(lo, hi)) in [(1usize, 5usize), (1, 300), (1, 70000), (60000, 70000)]
                .iter()
                .enumerate()
            {
                let total = if hi <= 5 { 3_000 } else { big_total };
                let input = gen_shape(&mut rng, k, total);
                let ops = ops_random(total, &mut rng, lo, hi);
                let label = format!(
                    "random[{},{}] mode={} af={} lvl={} total={} nops={}",
                    lo,
                    hi,
                    mode,
                    af,
                    level,
                    total,
                    ops.len()
                );
                run_frame(&label, &p, BeginSpec::plain(), &input, &ops, None, true);
            }

            // Zero-length updates interleaved with real ones.
            for &chunk in [1usize, 4095, 65536].iter() {
                let total = (chunk * 4).min(big_total);
                let input = gen_random(&mut rng, total);
                let label = format!(
                    "zeros chunk={} mode={} af={} lvl={} total={}",
                    chunk, mode, af, level, total
                );
                run_frame(
                    &label,
                    &p,
                    BeginSpec::plain(),
                    &input,
                    &ops_with_zeros(total, chunk),
                    None,
                    true,
                );
            }

            // stableSrc == 1 keeps the dictionary in the caller's buffer instead of
            // saving it into tmpBuff — a completely different tmpIn trajectory.
            let mut copts = LZ4F_compressOptions_t::default();
            copts.stableSrc = 1;
            let total = big_total;
            let input = gen_selfref(&mut rng, total);
            for &chunk in [7usize, 4096, 65536].iter() {
                let label = format!(
                    "stableSrc chunk={} mode={} af={} lvl={} total={}",
                    chunk, mode, af, level, total
                );
                run_frame(
                    &label,
                    &p,
                    BeginSpec::plain(),
                    &input,
                    &ops_fixed(total, chunk),
                    Some(&copts),
                    true,
                );
            }
        }
    }
}

// ===========================================================================
// 5. Flush behaviour
// ===========================================================================

#[test]
fn frame_stream_flush() {
    let mut rng = Rng::new(0xF10_5411_0000_0001);

    for &(mode, af) in [
        (LZ4F_blockLinked, 0u32),
        (LZ4F_blockLinked, 1u32),
        (LZ4F_blockIndependent, 0u32),
        (LZ4F_blockIndependent, 1u32),
    ]
    .iter()
    {
        for &level in [1, 9].iter() {
            let mut p = base_prefs(LZ4F_max64KB, mode, level, af);
            p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
            p.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;
            let bs = block_size(LZ4F_max64KB);

            // (a) flush right after begin, twice: nothing buffered -> must be 0.
            {
                let mut s = Sess::new(&format!("flush-empty mode={} af={} lvl={}", mode, af, level), p);
                s.begin(BeginSpec::plain());
                let a = s.flush(None);
                assert_eq!(a, 0, "flush with empty tmpIn must return 0");
                let b = s.flush(None);
                assert_eq!(b, 0, "second flush must also return 0");
                // and again through a NULL cOptPtr / explicit options
                let co = LZ4F_compressOptions_t::default();
                assert_eq!(s.flush(Some(&co)), 0);
                s.end(None);
                let f = s.frame();
                assert_round_trip(&f, &[], "flush-empty");
            }

            // (b) flush after every update, incl. double flushes.
            {
                let total = 40_000;
                let input = gen_mixed(&mut rng, total);
                let label = format!("flush-every mode={} af={} lvl={}", mode, af, level);
                run_frame(
                    &label,
                    &p,
                    BeginSpec::plain(),
                    &input,
                    &ops_flush_every(total, 3_000, 1),
                    None,
                    true,
                );
            }

            // (c) flush right after a block-boundary-aligned update: tmpInSize == 0.
            {
                let total = 2 * bs;
                let input = gen_periodic(&mut rng, total, 31);
                let ops = vec![
                    Op::Upd(bs),
                    Op::Flush,
                    Op::Flush,
                    Op::Upd(bs),
                    Op::Flush,
                    Op::Upd(0),
                    Op::Flush,
                ];
                let label = format!("flush-aligned mode={} af={} lvl={}", mode, af, level);
                run_frame(&label, &p, BeginSpec::plain(), &input, &ops, None, true);
            }

            // (d) flush interleaved so that tmpIn walks the whole tmpBuff
            //     (autoFlush==0 + blockLinked triggers the localSaveDict rewind).
            {
                let total = 5 * bs + 999;
                let input = gen_selfref(&mut rng, total);
                let ops = ops_flush_every(total, bs / 2 + 1, 3);
                let label = format!("flush-rewind mode={} af={} lvl={}", mode, af, level);
                run_frame(&label, &p, BeginSpec::plain(), &input, &ops, None, true);
            }
        }
    }
}

// ===========================================================================
// 6. LZ4F_uncompressedUpdate (stored blocks) incl. mode switching
// ===========================================================================

#[test]
fn frame_stream_uncompressed_update() {
    let mut rng = Rng::new(0x0000_C0C0_u64);
    let bs = block_size(LZ4F_max64KB);

    // `LZ4F_uncompressedUpdate` is documented as "only supported when
    // LZ4F_blockIndependent is used" (lz4frame.h:707), and the blockLinked path
    // runs into `assert(blockCompression == LZ4B_COMPRESSED)` at lz4frame.c:1071
    // — a contract violation. Driving it anyway corrupts the heap INSIDE THE C
    // LIBRARY ITSELF (glibc reports "corrupted size vs. prev_size" and aborts),
    // so it is undefined behaviour, not a comparable behaviour. Only the
    // supported blockIndependent mode is exercised here, exactly as the other
    // UB rows are excluded in tests/lz4_errors.rs.
    for &mode in [LZ4F_blockIndependent].iter() {
        for &af in [0u32, 1].iter() {
            for &level in [1, 9].iter() {
                for &cchk in [LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled].iter() {
                    for &bchk in [LZ4F_noBlockChecksum, LZ4F_blockChecksumEnabled].iter() {
                        let mut p = base_prefs(LZ4F_max64KB, mode, level, af);
                        p.frameInfo.contentChecksumFlag = cchk;
                        p.frameInfo.blockChecksumFlag = bchk;
                        let rt = mode == LZ4F_blockIndependent;

                        let plans: [(&str, Vec<Op>, usize); 6] = [
                            // uncompressed only, several blocks
                            ("unc-only", vec![Op::Unc(bs), Op::Unc(bs), Op::Unc(777)], 2 * bs + 777),
                            // compressed first, then switch -> flush buffered data
                            (
                                "upd-then-unc",
                                vec![Op::Upd(1000), Op::Unc(2000), Op::Unc(10)],
                                3010,
                            ),
                            // uncompressed first, then switch back
                            (
                                "unc-then-upd",
                                vec![Op::Unc(1000), Op::Upd(2000), Op::Upd(10)],
                                3010,
                            ),
                            // rapid alternation across a block boundary
                            (
                                "alternating",
                                vec![
                                    Op::Upd(bs - 10),
                                    Op::Unc(20),
                                    Op::Upd(30),
                                    Op::Unc(bs),
                                    Op::Upd(5),
                                    Op::Flush,
                                    Op::Unc(5),
                                ],
                                2 * bs + 50,
                            ),
                            // zero-length uncompressed updates
                            (
                                "unc-zeros",
                                vec![Op::Unc(0), Op::Unc(100), Op::Unc(0), Op::Upd(0), Op::Unc(100)],
                                200,
                            ),
                            // flush between stored blocks
                            (
                                "unc-flush",
                                vec![Op::Unc(500), Op::Flush, Op::Flush, Op::Unc(500), Op::Flush],
                                1000,
                            ),
                        ];

                        for (name, ops, total) in plans.iter() {
                            let input = gen_shape(&mut rng, *total % N_SHAPES, *total);
                            let label = format!(
                                "unc/{} mode={} af={} lvl={} cchk={} bchk={}",
                                name, mode, af, level, cchk, bchk
                            );
                            run_frame(&label, &p, BeginSpec::plain(), &input, ops, None, rt);
                        }
                    }
                }
            }
        }
    }

    // stableSrc + NULL cOptPtr on the uncompressed path.
    let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockIndependent, 1, 0);
    p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
    let input = gen_random(&mut rng, 3 * bs + 3);
    let mut copts = LZ4F_compressOptions_t::default();
    copts.stableSrc = 1;
    for (tag, co) in [("stableSrc", Some(&copts)), ("nullOpts", None)] {
        run_frame(
            &format!("unc/{}", tag),
            &p,
            BeginSpec::plain(),
            &input,
            &vec![Op::Unc(bs), Op::Upd(bs), Op::Unc(bs + 3)],
            co,
            true,
        );
    }
}

// ===========================================================================
// 7. Dictionaries
// ===========================================================================

#[test]
fn frame_stream_dictionaries() {
    let mut rng = Rng::new(0xD1C7_0000_1234_5678);
    // One backing buffer so every `&dictsrc[..n]` is a valid pointer (even n == 0).
    let dictsrc = gen_text(&mut rng, 70_000);
    const DICT_SIZES: [usize; 8] = [0, 1, 8, 100, 4096, 65535, 65536, 70000];

    let input = {
        // Highly correlated with the dictionary so the dict really changes output.
        let mut v = dictsrc[..4096].to_vec();
        v.extend_from_slice(&dictsrc[1000..5096]);
        v.extend_from_slice(&gen_text(&mut rng, 60_000));
        v
    };

    for &kind in [
        Begin::Dict,
        Begin::DictOnce,
        Begin::InternalDict,
        Begin::CDict,
        Begin::InternalCDict,
    ]
    .iter()
    {
        for &dsize in DICT_SIZES.iter() {
            for &level in [1, 9].iter() {
                for &mode in [LZ4F_blockLinked, LZ4F_blockIndependent].iter() {
                    let mut p = base_prefs(LZ4F_max64KB, mode, level, 0);
                    p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
                    p.frameInfo.dictID = 0x1234_5678;
                    let dict = &dictsrc[..dsize];
                    let label = format!(
                        "{} dictSize={} lvl={} mode={}",
                        kind.name(),
                        dsize,
                        level,
                        mode
                    );
                    // The dictionary is *not* supplied to the decoder, so a
                    // round-trip is only meaningful when no dictionary is in play.
                    run_frame(
                        &label,
                        &p,
                        BeginSpec::with_dict(kind, dict),
                        &input,
                        &ops_fixed(input.len(), 9_000),
                        None,
                        dsize == 0 && !kind.uses_cdict(),
                    );
                }
            }
        }
    }

    // NULL dict pointer with size 0 -> `if (dictBuffer)` is false, no loadDict.
    for &kind in [Begin::Dict, Begin::DictOnce, Begin::InternalDict, Begin::Internal].iter() {
        let p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
        let f = run_frame(
            &format!("{} NULL-dict", kind.name()),
            &p,
            BeginSpec::of(kind),
            &input,
            &ops_fixed(input.len(), 9_000),
            None,
            true,
        );
        // ... which must be byte-identical to plain LZ4F_compressBegin.
        let g = run_frame(
            "plain reference",
            &p,
            BeginSpec::plain(),
            &input,
            &ops_fixed(input.len(), 9_000),
            None,
            true,
        );
        assert_bytes_eq(
            &format!("{} with NULL dict == LZ4F_compressBegin", kind.name()),
            &g,
            &f,
        );
    }

    // NULL preferencesPtr on every begin flavour.
    for &kind in [
        Begin::Plain,
        Begin::Dict,
        Begin::DictOnce,
        Begin::CDict,
        Begin::Internal,
        Begin::InternalDict,
        Begin::InternalCDict,
    ]
    .iter()
    {
        let p = LZ4F_preferences_t::default();
        let dict = &dictsrc[..4096];
        let spec = if kind.uses_cdict() || kind.uses_dict() {
            BeginSpec::with_dict(kind, dict).null_prefs()
        } else {
            BeginSpec::of(kind).null_prefs()
        };
        run_frame(
            &format!("{} NULL prefs", kind.name()),
            &p,
            spec,
            &input,
            &ops_fixed(input.len(), 30_000),
            None,
            false,
        );
    }

    // `usingDict` is documented as a thin wrapper over `usingDictOnce`, and the
    // dictionary is only ever applied to the FIRST block. Prove both:
    //  - the two entry points produce identical frames,
    //  - the dictionary really changes the output (so it is really being tested),
    //  - for blockIndependent, everything after block #1 matches the dict-less frame.
    {
        let dict = &dictsrc[..65536];
        let multi = {
            let mut v = dictsrc[..70_000].to_vec();
            v.extend_from_slice(&dictsrc[..70_000]);
            v
        };
        for &mode in [LZ4F_blockLinked, LZ4F_blockIndependent].iter() {
            let mut p = base_prefs(LZ4F_max64KB, mode, 1, 1);
            p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
            let ops = ops_single(multi.len());

            let f_dict = run_frame(
                &format!("dictOnce-vs-dict usingDict mode={}", mode),
                &p,
                BeginSpec::with_dict(Begin::Dict, dict),
                &multi,
                &ops,
                None,
                false,
            );
            let f_once = run_frame(
                &format!("dictOnce-vs-dict usingDictOnce mode={}", mode),
                &p,
                BeginSpec::with_dict(Begin::DictOnce, dict),
                &multi,
                &ops,
                None,
                false,
            );
            assert_bytes_eq(
                &format!("usingDict == usingDictOnce (mode={})", mode),
                &f_dict,
                &f_once,
            );

            let f_none = run_frame(
                &format!("dictOnce-vs-dict no-dict mode={}", mode),
                &p,
                BeginSpec::plain(),
                &multi,
                &ops,
                None,
                true,
            );
            // A RAW dictionary (`LZ4F_compressBegin_usingDict` /
            // `_usingDictOnce`) only affects LZ4F_blockLinked frames. For
            // LZ4F_blockIndependent, `LZ4F_compressBlock` calls
            // `LZ4F_initStream(ctx, cdict, ...)` on EVERY block and then, when
            // `cdict == NULL`, compresses with the one-shot
            // `LZ4_compress_fast_extState_fastReset` (lz4frame.c:911-921) —
            // which resets the context and therefore DISCARDS the dictionary
            // that `LZ4F_compressBegin_usingDict` loaded via `LZ4_loadDict`.
            // So `f_dict == f_none` is CORRECT C behaviour there. Only a real
            // `LZ4F_CDict` survives, because it is re-attached on every block.
            if mode == LZ4F_blockLinked {
                assert_ne!(
                    f_dict, f_none,
                    "mode={} (blockLinked): the dictionary had no effect at all,                      test is vacuous",
                    mode
                );
            } else {
                assert_eq!(
                    f_dict, f_none,
                    "mode={} (blockIndependent): a RAW dictionary must be a                      no-op because every block is compressed with the one-shot                      fastReset API (lz4frame.c:919)",
                    mode
                );
            }

        }
    }

    // A CDict must also visibly change the output.
    {
        let dict = &dictsrc[..65536];
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 1);
        p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
        let f_cd = run_frame(
            "cdict effect usingCDict",
            &p,
            BeginSpec::with_dict(Begin::CDict, dict),
            &input,
            &ops_single(input.len()),
            None,
            false,
        );
        let f_none = run_frame(
            "cdict effect none",
            &p,
            BeginSpec::plain(),
            &input,
            &ops_single(input.len()),
            None,
            true,
        );
        assert_ne!(f_cd, f_none, "the CDict had no effect at all, test is vacuous");
    }
}

// ===========================================================================
// 8. Context reuse (no free between frames)
// ===========================================================================

#[test]
fn frame_stream_context_reuse() {
    let mut rng = Rng::new(0x2E05_1234_5678_9ABC);
    let bs64 = block_size(LZ4F_max64KB);

    // (a) Same preferences, three frames back to back.
    {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
        p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
        p.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;
        let mut s = Sess::new("reuse same prefs", p);
        for f in 0..3 {
            let input = gen_shape(&mut rng, f, 3 * bs64 + 77);
            s.reset_frame();
            s.begin(BeginSpec::plain());
            s.drive(&input, &ops_fixed(input.len(), 7_000), None);
            s.end(None);
            let frame = s.frame();
            assert_round_trip(&frame, &input, &format!("reuse same prefs frame#{}", f));
        }
    }

    // (b) Preferences changed between frames, forcing every cctx-management path:
    //     fast->HC (realloc), HC->fast (reset only), blockSizeID up (tmpBuff grow),
    //     blockSizeID down (tmpBuff kept), autoFlush toggling, contentSize on/off.
    {
        let cfgs: [(c_int, c_int, c_int, u32, bool); 7] = [
            (LZ4F_max64KB, LZ4F_blockLinked, 1, 0, false),
            (LZ4F_max64KB, LZ4F_blockIndependent, 12, 0, true),
            (LZ4F_max1MB, LZ4F_blockLinked, 1, 0, false),
            (LZ4F_max64KB, LZ4F_blockLinked, 9, 1, true),
            (LZ4F_max256KB, LZ4F_blockIndependent, 0, 1, false),
            (LZ4F_max64KB, LZ4F_blockLinked, 2, 0, true),
            (LZ4F_default, LZ4F_blockIndependent, -3, 1, false),
        ];
        let mut s = Sess::new("reuse changing prefs", LZ4F_preferences_t::default());
        for (i, &(bsid, mode, level, af, with_cs)) in cfgs.iter().enumerate() {
            let total = 2 * block_size(bsid) + 1234;
            let input = gen_shape(&mut rng, i, total);
            let mut p = base_prefs(bsid, mode, level, af);
            p.frameInfo.contentChecksumFlag = if i % 2 == 0 {
                LZ4F_contentChecksumEnabled
            } else {
                LZ4F_noContentChecksum
            };
            p.frameInfo.blockChecksumFlag = if i % 3 == 0 {
                LZ4F_blockChecksumEnabled
            } else {
                LZ4F_noBlockChecksum
            };
            p.frameInfo.contentSize = if with_cs { total as u64 } else { 0 };
            s.prefs = p;
            s.reset_frame();
            s.begin(BeginSpec::plain());
            s.drive(&input, &ops_fixed(total, block_size(bsid) / 3 + 1), None);
            s.end(None);
            let frame = s.frame();
            assert_round_trip(
                &frame,
                &input,
                &format!("reuse changing prefs frame#{} bsid={} lvl={}", i, bsid, level),
            );
        }
    }

    // (c) `blockCompressMode` is NOT reset by compressBegin: a frame that ended
    //     in uncompressed mode is followed by one that starts compressed.
    {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockIndependent, 1, 0);
        p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
        let mut s = Sess::new("reuse blockCompressMode", p);
        let input = gen_mixed(&mut rng, 5_000);
        for f in 0..4 {
            s.reset_frame();
            s.begin(BeginSpec::plain());
            if f % 2 == 0 {
                s.drive(&input, &vec![Op::Unc(2_000), Op::Upd(3_000)], None);
            } else {
                s.drive(&input, &vec![Op::Upd(2_000), Op::Unc(3_000)], None);
            }
            s.end(None);
            let frame = s.frame();
            assert_round_trip(&frame, &input, &format!("reuse bcm frame#{}", f));
        }
    }

    // (d) Reuse with a dictionary / CDict on the second frame.
    {
        let dictsrc = gen_text(&mut rng, 65_536);
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
        p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
        let mut s = Sess::new("reuse with dict", p);
        let input = gen_text(&mut rng, 90_000);

        s.begin(BeginSpec::plain());
        s.drive(&input, &ops_fixed(input.len(), 20_000), None);
        s.end(None);
        let f0 = s.frame();
        assert_round_trip(&f0, &input, "reuse with dict frame#0");

        s.reset_frame();
        s.begin(BeginSpec::with_dict(Begin::DictOnce, &dictsrc[..40_000]));
        s.drive(&input, &ops_fixed(input.len(), 20_000), None);
        s.end(None);
        let _ = s.frame();

        s.reset_frame();
        s.begin(BeginSpec::with_dict(Begin::CDict, &dictsrc[..65_536]));
        s.drive(&input, &ops_fixed(input.len(), 20_000), None);
        s.end(None);
        let _ = s.frame();

        // back to no dictionary: cctx->cdict must have been cleared
        s.reset_frame();
        s.begin(BeginSpec::plain());
        s.drive(&input, &ops_fixed(input.len(), 20_000), None);
        s.end(None);
        let f3 = s.frame();
        // `cctx->cdict` IS cleared by `LZ4F_compressBegin` (lz4frame.c:758,
        // `cctx->cdict = cdict;` with cdict == NULL), and `LZ4_prepareTable`
        // additionally clears `dictCtx`/`dictionary`/`dictSize` (lz4.c:917-919).
        // But `LZ4F_initStream` only performs a *fast* reset for blockLinked
        // (`LZ4_resetStream_fast` -> `LZ4_prepareTable`, lz4frame.c:761), which
        // DELIBERATELY RE-USES the hash table and advances `currentOffset` by
        // 64 KB (lz4.c:903-914). So a reused cctx finds different matches than a
        // pristine one, and `f3 != f0` is CORRECT C behaviour — we must not
        // assert equality here. What matters is that C and Rust agree, which
        // `Sess::frame()` already asserted for all four frames, and that the
        // dictionary is genuinely no longer applied (so f3 decodes with no dict).
        assert_ne!(
            f0, f3,
            "expected the reused-cctx frame to differ from the pristine one              (LZ4_resetStream_fast re-uses the hash table); if these are equal              the fast-reset path is not being exercised"
        );
        assert_round_trip(&f3, &input, "reuse with dict frame#3");
    }
}

// ===========================================================================
// 9. Input shapes and sizes around the block boundary
// ===========================================================================

#[test]
fn frame_stream_shapes_and_sizes() {
    let mut rng = Rng::new(0x5AFE_0000_1234_5678);
    let bs = block_size(LZ4F_max64KB);
    let sizes = [0usize, 1, 2, 15, bs - 1, bs, bs + 1, 3 * bs + 7, 9 * bs + 1];

    for shape in 0..N_SHAPES {
        for &total in sizes.iter() {
            let input = gen_shape(&mut rng, shape, total);
            for (ci, &(mode, af, cchk, bchk, lvl)) in [
                (LZ4F_blockLinked, 0u32, 1, 1, 1),
                (LZ4F_blockIndependent, 1u32, 0, 0, 1),
                (LZ4F_blockLinked, 1u32, 1, 0, 3),
            ]
            .iter()
            .enumerate()
            {
                let mut p = base_prefs(LZ4F_max64KB, mode, lvl, af);
                p.frameInfo.contentChecksumFlag = cchk;
                p.frameInfo.blockChecksumFlag = bchk;
                p.frameInfo.contentSize = total as u64; // exercises the 8-byte field
                let ops = if ci == 0 {
                    ops_single(total)
                } else {
                    ops_fixed(total, 4_096)
                };
                let label = format!(
                    "shape={} total={} cfg={} mode={} af={} lvl={}",
                    shape_name(shape),
                    total,
                    ci,
                    mode,
                    af,
                    lvl
                );
                run_frame(&label, &p, BeginSpec::plain(), &input, &ops, None, true);
            }
        }
    }

    // Large block sizes, multi-block, cheap level.
    for &bsid in [LZ4F_max256KB, LZ4F_max1MB, LZ4F_max4MB].iter() {
        let bs = block_size(bsid);
        for &(mode, af) in [
            (LZ4F_blockLinked, 0u32),
            (LZ4F_blockLinked, 1u32),
            (LZ4F_blockIndependent, 0u32),
            (LZ4F_blockIndependent, 1u32),
        ]
        .iter()
        {
            let total = 2 * bs + 1234;
            let input = gen_mixed(&mut rng, total);
            let mut p = base_prefs(bsid, mode, 1, af);
            p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
            p.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;
            let label = format!("bigblocks bsid={} mode={} af={} total={}", bsid, mode, af, total);
            run_frame(
                &label,
                &p,
                BeginSpec::plain(),
                &input,
                &ops_fixed(total, bs / 2 + 3),
                None,
                true,
            );
        }
    }
}

// ===========================================================================
// 10. Error rows
// ===========================================================================

#[test]
fn frame_stream_error_state_uninitialized() {
    let (cu, ru) = both::<FnUpdate>("LZ4F_compressUpdate");
    let (cuu, ruu) = both::<FnUpdate>("LZ4F_uncompressedUpdate");
    let (cfl, rfl) = both::<FnFlush>("LZ4F_flush");
    let (ce, re) = both::<FnFlush>("LZ4F_compressEnd");

    let src = vec![0x42u8; 1000];
    let cap = 4096usize;

    let mut s = Sess::new("uninitialized", base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 1));
    unsafe {
        // --- before any compressBegin: cStage == 0
        s.prep(cap);
        let a = cu(
            s.c,
            s.cbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            ptr::null(),
        );
        let b = ru(
            s.r,
            s.rbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            ptr::null(),
        );
        expect_err(
            "compressUpdate before compressBegin",
            a,
            b,
            err::ERROR_compressionState_uninitialized,
        );
        s.cmp_dst(cap, "compressUpdate before begin: dst untouched");

        s.prep(cap);
        let a = cuu(
            s.c,
            s.cbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            ptr::null(),
        );
        let b = ruu(
            s.r,
            s.rbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            ptr::null(),
        );
        expect_err(
            "uncompressedUpdate before compressBegin",
            a,
            b,
            err::ERROR_compressionState_uninitialized,
        );
        s.cmp_dst(cap, "uncompressedUpdate before begin: dst untouched");

        // LZ4F_flush returns 0 *before* looking at cStage, because tmpInSize == 0.
        s.prep(cap);
        let a = cfl(s.c, s.cbuf.as_mut_ptr() as *mut c_void, cap, ptr::null());
        let b = rfl(s.r, s.rbuf.as_mut_ptr() as *mut c_void, cap, ptr::null());
        same_ret("flush on fresh cctx", a, b);
        assert_eq!(a, 0, "flush on a fresh cctx must return 0 (nothing buffered)");
        s.cmp_dst(cap, "flush on fresh cctx: dst untouched");

        // compressEnd on a fresh cctx *succeeds*: flush is a no-op and the
        // zero-initialized prefs mean no content checksum -> 4 bytes of endMark.
        s.prep(cap);
        let a = ce(s.c, s.cbuf.as_mut_ptr() as *mut c_void, cap, ptr::null());
        let b = re(s.r, s.rbuf.as_mut_ptr() as *mut c_void, cap, ptr::null());
        same_ret("compressEnd on fresh cctx", a, b);
        assert_eq!(a, 4, "compressEnd on a fresh cctx should emit only the endMark");
        s.cmp_dst(cap, "compressEnd on fresh cctx: dst");
    }

    // --- after a completed frame: cStage is back to 0
    let mut s2 = Sess::new("after-end", base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 1));
    s2.begin(BeginSpec::plain());
    s2.drive(&src, &ops_fixed(src.len(), 300), None);
    s2.end(None);
    let f = s2.frame();
    assert_round_trip(&f, &src, "after-end frame");
    unsafe {
        s2.prep(cap);
        let a = cu(
            s2.c,
            s2.cbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            ptr::null(),
        );
        let b = ru(
            s2.r,
            s2.rbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            ptr::null(),
        );
        expect_err(
            "compressUpdate after compressEnd",
            a,
            b,
            err::ERROR_compressionState_uninitialized,
        );
        s2.cmp_dst(cap, "compressUpdate after end: dst untouched");

        s2.prep(cap);
        let a = cuu(
            s2.c,
            s2.cbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            ptr::null(),
        );
        let b = ruu(
            s2.r,
            s2.rbuf.as_mut_ptr() as *mut c_void,
            cap,
            src.as_ptr() as *const c_void,
            src.len(),
            ptr::null(),
        );
        expect_err(
            "uncompressedUpdate after compressEnd",
            a,
            b,
            err::ERROR_compressionState_uninitialized,
        );
        s2.cmp_dst(cap, "uncompressedUpdate after end: dst untouched");
    }
}

#[test]
fn frame_stream_error_begin_capacity() {
    let mut rng = Rng::new(0xBE61_0000_0000_0011);
    let dictsrc = gen_text(&mut rng, 4096);

    // Every begin flavour, capacity 0..=18 must fail, 19 must succeed.
    for &kind in [
        Begin::Plain,
        Begin::Dict,
        Begin::DictOnce,
        Begin::CDict,
        Begin::Internal,
        Begin::InternalDict,
        Begin::InternalCDict,
    ]
    .iter()
    {
        for &(cs, did) in [(0u64, 0u32), (12345u64, 0xDEAD_BEEF)].iter() {
            let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
            p.frameInfo.contentSize = cs;
            p.frameInfo.dictID = did;

            for cap in 0..=LZ4F_HEADER_SIZE_MAX {
                let mut s = Sess::new(
                    &format!("{} cap={} cs={} dictID={}", kind.name(), cap, cs, did),
                    p,
                );
                let spec = if kind.uses_cdict() || kind.uses_dict() {
                    BeginSpec::with_dict(kind, &dictsrc[..4096])
                } else {
                    BeginSpec::of(kind)
                };
                let n = s.begin_cap(spec, cap);
                if cap < LZ4F_HEADER_SIZE_MAX {
                    assert!(
                        lz4f_is_error(n),
                        "{}: cap={} should be rejected",
                        kind.name(),
                        cap
                    );
                    assert_eq!(
                        lz4f_error_code(n),
                        err::ERROR_dstMaxSize_tooSmall,
                        "{}: cap={} wrong error",
                        kind.name(),
                        cap
                    );
                } else {
                    assert!(
                        !lz4f_is_error(n),
                        "{}: cap=19 must succeed, got {}",
                        kind.name(),
                        describe(n)
                    );
                    let expect = 7 + if cs != 0 { 8 } else { 0 } + if did != 0 { 4 } else { 0 };
                    assert_eq!(n, expect, "{}: header size", kind.name());
                }
            }
        }
    }
}

#[test]
fn frame_stream_error_update_capacity() {
    let mut rng = Rng::new(0x0CA9_0000_1234_0001);
    let src = gen_mixed(&mut rng, 3000);

    // (a) autoFlush == 1: LZ4F_compressBound is the exact minimum.
    for &(cchk, bchk) in [(0, 0), (1, 1), (1, 0)].iter() {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockIndependent, 1, 1);
        p.frameInfo.contentChecksumFlag = cchk;
        p.frameInfo.blockChecksumFlag = bchk;
        let need = bound_both(src.len(), &p);
        for cap in [0usize, 1, 4, need / 2, need - 1, need] {
            let mut s = Sess::new(&format!("upd cap={} need={}", cap, need), p);
            s.begin(BeginSpec::plain());
            let n = s.update_cap(false, &src, None, cap);
            if cap < need {
                expect_err(
                    &format!("compressUpdate cap={} < bound={}", cap, need),
                    n,
                    n,
                    err::ERROR_dstMaxSize_tooSmall,
                );
            } else {
                assert!(!lz4f_is_error(n), "cap={} == bound should succeed", cap);
            }
        }
    }

    // (b) with data already buffered the bound grows by tmpInSize.
    {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
        p.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;
        let bs = block_size(LZ4F_max64KB);
        // buffer bs-10 bytes, then push 100 more: one full block must come out.
        let big = gen_selfref(&mut rng, bs + 100);
        let need_second = {
            // LZ4F_compressBound() uses alreadyBuffered = SIZE_MAX for autoFlush==0,
            // which clamps to blockSize-1 — i.e. an upper bound of the real need.
            bound_both(100, &p)
        };
        for cap in [0usize, 1, 8, need_second] {
            let mut s = Sess::new(&format!("buffered upd cap={}", cap), p);
            s.begin(BeginSpec::plain());
            s.update(&big[..bs - 10], None);
            let n = s.update_cap(false, &big[bs - 10..], None, cap);
            if cap < bs + 8 {
                expect_err(
                    &format!("buffered compressUpdate cap={}", cap),
                    n,
                    n,
                    err::ERROR_dstMaxSize_tooSmall,
                );
            } else {
                assert!(!lz4f_is_error(n), "cap={} should succeed", cap);
            }
        }
    }

    // (c) autoFlush == 0, srcSize < blockSize.
    //
    //     NOTE: the public `LZ4F_compressBound(srcSize, prefs)` calls
    //     `LZ4F_compressBound_internal(srcSize, prefs, (size_t)-1)`, and the
    //     `alreadyBuffered` argument is clamped to `blockSize - 1`
    //     (lz4frame.c:391-392). So the public bound ALWAYS budgets for a full
    //     block plus a partial one and is therefore >= blockSize — it is never
    //     "tiny", not even when the data is only going to be buffered. We
    //     therefore do not predict the bound; we sweep dstCapacity and require
    //     C and Rust to agree at EVERY value, then pin only the two facts the C
    //     really guarantees: capacity 0 is rejected, and `LZ4F_compressBound`
    //     is always sufficient.
    {
        let p = base_prefs(LZ4F_max64KB, LZ4F_blockIndependent, 1, 0);
        let need = bound_both(src.len(), &p);
        assert!(
            need >= block_size(LZ4F_max64KB),
            "LZ4F_compressBound should budget a whole block, got {}",
            need
        );

        // dstCapacity == 0 must be rejected.
        let mut s = Sess::new("autoflush0 cap=0", p);
        s.begin(BeginSpec::plain());
        let n = s.update_cap(false, &src, None, 0);
        expect_err("compressUpdate cap=0", n, n, err::ERROR_dstMaxSize_tooSmall);

        // dstCapacity == LZ4F_compressBound must succeed; the data is only
        // buffered here, so nothing is emitted yet.
        let mut s = Sess::new("autoflush0 cap=bound", p);
        s.begin(BeginSpec::plain());
        let n = s.update_cap(false, &src, None, need);
        assert!(!lz4f_is_error(n), "cap=bound must succeed, got {}", describe(n));
        assert_eq!(n, 0, "srcSize < blockSize with autoFlush=0 buffers everything");

        // Sweep a wide range of capacities: the ONLY requirement is that C and
        // Rust behave identically at each one (asserted inside `update_cap`).
        for cap in [
            1usize, 2, 3, 4, 8, 16, 64, 255, 256, 1000,
            src.len() / 2, src.len() - 1, src.len(), src.len() + 1,
            need / 2, need - 1, need, need + 1, need + 4096,
        ] {
            let mut s = Sess::new(&format!("autoflush0 cap sweep cap={}", cap), p);
            s.begin(BeginSpec::plain());
            s.update_cap(false, &src, None, cap);
        }
    }

    // (d) LZ4F_uncompressedUpdate has the EXTRA `dstCapacity < srcSize` check
    //     (lz4frame.c:1009-1010) on top of the shared bound check. Sweep both
    //     sides of `srcSize` and require identical behaviour; the one fact the
    //     C guarantees is that any capacity < srcSize is rejected.
    //
    //     Only LZ4F_blockIndependent is used: `LZ4F_uncompressedUpdate` is
    //     documented as supported only in that mode (lz4frame.h:707) and the
    //     blockLinked path hits `assert(blockCompression == LZ4B_COMPRESSED)`
    //     at lz4frame.c:1071.
    for &mode in [LZ4F_blockIndependent].iter() {
        let p = base_prefs(LZ4F_max64KB, mode, 1, 0);
        let need = bound_both(src.len(), &p);
        for cap in [
            0usize, 1, 8, src.len() / 2, src.len() - 1, src.len(), src.len() + 8,
            need, need + 8,
        ] {
            let mut s = Sess::new(&format!("unc cap={} mode={}", cap, mode), p);
            s.begin(BeginSpec::plain());
            let n = s.update_cap(true, &src, None, cap);
            if cap < src.len() {
                expect_err(
                    &format!("uncompressedUpdate cap={} < srcSize={}", cap, src.len()),
                    n,
                    n,
                    err::ERROR_dstMaxSize_tooSmall,
                );
            } else {
                assert!(
                    !lz4f_is_error(n),
                    "cap={} >= srcSize={} should succeed, got {}",
                    cap,
                    src.len(),
                    describe(n)
                );
            }
        }
    }
}

#[test]
fn frame_stream_error_flush_capacity() {
    let mut rng = Rng::new(0xF1F1_0000_0000_0002);
    let src = gen_text(&mut rng, 1000);

    for &bchk in [LZ4F_noBlockChecksum, LZ4F_blockChecksumEnabled].iter() {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
        p.frameInfo.blockChecksumFlag = bchk;
        // LZ4F_flush requires tmpInSize + BHSize + BFSize, regardless of bchk.
        let need = src.len() + LZ4F_BLOCK_HEADER_SIZE + LZ4F_BLOCK_CHECKSUM_SIZE;
        for cap in [0usize, 1, 7, need / 2, need - 1, need, need + 1] {
            let mut s = Sess::new(&format!("flush cap={} bchk={}", cap, bchk), p);
            s.begin(BeginSpec::plain());
            s.update(&src, None); // buffered, nothing emitted
            let n = s.flush_cap(None, cap);
            if cap < need {
                expect_err(
                    &format!("flush cap={} need={}", cap, need),
                    n,
                    n,
                    err::ERROR_dstMaxSize_tooSmall,
                );
            } else {
                assert!(!lz4f_is_error(n), "flush cap={} should succeed", cap);
            }
        }

        // Nothing buffered -> early `return 0` happens *before* the capacity check.
        let mut s = Sess::new("flush empty cap=0", p);
        s.begin(BeginSpec::plain());
        assert_eq!(s.flush_cap(None, 0), 0, "flush with empty tmpIn and cap=0 must be 0");
    }
}

#[test]
fn frame_stream_error_end_capacity() {
    let mut rng = Rng::new(0xE4D0_0000_0000_0003);
    let src = gen_text(&mut rng, 1000);

    // (a) nothing buffered: 4 bytes needed, 8 with a content checksum.
    for &cchk in [LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled].iter() {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockIndependent, 1, 1);
        p.frameInfo.contentChecksumFlag = cchk;
        let need = if cchk == LZ4F_contentChecksumEnabled { 8 } else { 4 };
        for cap in 0..=9usize {
            let mut s = Sess::new(&format!("end cap={} cchk={}", cap, cchk), p);
            s.begin(BeginSpec::plain());
            s.update(&src, None);
            let n = s.end_cap(None, cap);
            if cap < need {
                expect_err(
                    &format!("compressEnd cap={} cchk={}", cap, cchk),
                    n,
                    n,
                    err::ERROR_dstMaxSize_tooSmall,
                );
            } else {
                assert!(!lz4f_is_error(n), "compressEnd cap={} should succeed", cap);
                assert_eq!(n, need, "compressEnd size");
            }
        }
    }

    // (b) buffered data: the embedded flush fails first, then the endMark check.
    for &cchk in [LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled].iter() {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
        p.frameInfo.contentChecksumFlag = cchk;
        let flush_need = src.len() + LZ4F_BLOCK_HEADER_SIZE + LZ4F_BLOCK_CHECKSUM_SIZE;
        for cap in [
            0usize,
            flush_need - 1,
            flush_need,
            flush_need + 1,
            flush_need + 4,
            flush_need + 8,
            flush_need + 64,
        ] {
            let mut s = Sess::new(&format!("end buffered cap={} cchk={}", cap, cchk), p);
            s.begin(BeginSpec::plain());
            s.update(&src, None);
            let n = s.end_cap(None, cap);
            // The flush writes a compressed block whose size we do not model
            // exactly; only require that C and Rust agree (checked inside
            // end_cap) and that a generous capacity succeeds.
            if cap >= flush_need + 64 {
                assert!(!lz4f_is_error(n), "compressEnd cap={} should succeed", cap);
            }
            if lz4f_is_error(n) {
                assert_eq!(
                    lz4f_error_code(n),
                    err::ERROR_dstMaxSize_tooSmall,
                    "compressEnd cap={} unexpected error",
                    cap
                );
            }
        }
    }
}

#[test]
fn frame_stream_error_content_size_mismatch() {
    let mut rng = Rng::new(0xC512_0000_0000_0004);
    let src = gen_mixed(&mut rng, 5000);

    for &cchk in [LZ4F_noContentChecksum, LZ4F_contentChecksumEnabled].iter() {
        for &af in [0u32, 1].iter() {
            for &(declared, supplied, want_ok) in [
                (5000u64, 5000usize, true),
                (5000, 4999, false),
                (5000, 5000 - 1000, false),
                (4999, 5000, false),
                (1, 5000, false),
                (5000 + 1, 5000, false),
            ]
            .iter()
            {
                let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, af);
                p.frameInfo.contentChecksumFlag = cchk;
                p.frameInfo.contentSize = declared;
                let label = format!(
                    "contentSize declared={} supplied={} cchk={} af={}",
                    declared, supplied, cchk, af
                );
                let mut s = Sess::new(&label, p);
                s.begin(BeginSpec::plain());
                s.drive(&src[..supplied], &ops_fixed(supplied, 900), None);
                let n = s.end(None);
                if want_ok {
                    assert!(!lz4f_is_error(n), "{}: should succeed, got {}", label, describe(n));
                    let f = s.frame();
                    assert_round_trip(&f, &src[..supplied], &label);
                } else {
                    expect_err(&label, n, n, err::ERROR_frameSize_wrong);
                }
            }
        }
    }
}

#[test]
fn frame_stream_error_dict_size_too_large() {
    let mut rng = Rng::new(0xD177_0000_0000_0005);
    // A *small* real buffer with a lying size: lz4frame.c:766-768 checks
    // `dictSize > INT_MAX` before dereferencing dictBuffer, so this is safe.
    let small = gen_text(&mut rng, 64);
    let huge = 0x8000_0000usize;
    assert!(huge > c_int::MAX as usize);

    let p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
    for &kind in [Begin::Dict, Begin::DictOnce, Begin::InternalDict].iter() {
        for &size in [huge, usize::MAX, (c_int::MAX as usize) + 1].iter() {
            for &level in [1, 9].iter() {
                let mut pp = p;
                pp.compressionLevel = level;
                let mut s = Sess::new(
                    &format!("{} dictSize=0x{:x} lvl={}", kind.name(), size, level),
                    pp,
                );
                let spec = BeginSpec {
                    kind,
                    dict: Some(&small),
                    size_override: Some(size),
                    null_prefs: false,
                };
                let cap = LZ4F_HEADER_SIZE_MAX + 24;
                let n = s.begin_cap(spec, cap);
                expect_err(
                    &format!("{} dictSize=0x{:x}", kind.name(), size),
                    n,
                    n,
                    err::ERROR_parameter_invalid,
                );
                // The header is written only after the dictionary is loaded, so
                // dst must be completely untouched in both libraries.
                s.cmp_dst(cap, "dictSize>INT_MAX: dst untouched");
            }
        }
    }

    // INT_MAX itself is accepted by the size check (we do not follow through with
    // an actual load, that would read 2GB), so only the boundary above is tested.
}

// ---------------------------------------------------------------------------
// Allocation-failure injection
// ---------------------------------------------------------------------------

#[repr(C)]
struct AllocState {
    calls: u64,
    fail_at: u64,
    live: i64,
}

const HDR: usize = 16;

fn alloc_raw(opaque: *mut c_void, size: usize, zero: bool) -> *mut c_void {
    unsafe {
        let st = &mut *(opaque as *mut AllocState);
        st.calls += 1;
        if st.fail_at != 0 && st.calls == st.fail_at {
            return ptr::null_mut();
        }
        let total = size + HDR;
        let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
        let p = if zero {
            std::alloc::alloc_zeroed(layout)
        } else {
            std::alloc::alloc(layout)
        };
        if p.is_null() {
            return ptr::null_mut();
        }
        *(p as *mut usize) = total;
        st.live += 1;
        p.add(HDR) as *mut c_void
    }
}

extern "C" fn test_alloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    alloc_raw(opaque, size, false)
}

extern "C" fn test_calloc(opaque: *mut c_void, size: usize) -> *mut c_void {
    alloc_raw(opaque, size, true)
}

extern "C" fn test_free(opaque: *mut c_void, address: *mut c_void) {
    if address.is_null() {
        return;
    }
    unsafe {
        let st = &mut *(opaque as *mut AllocState);
        let base = (address as *mut u8).sub(HDR);
        let total = *(base as *mut usize);
        let layout = std::alloc::Layout::from_size_align(total, 16).unwrap();
        std::alloc::dealloc(base, layout);
        st.live -= 1;
    }
}

fn cmem_for(st: &mut AllocState, with_calloc: bool) -> LZ4F_CustomMem {
    LZ4F_CustomMem {
        customAlloc: Some(test_alloc),
        customCalloc: if with_calloc { Some(test_calloc) } else { None },
        customFree: Some(test_free),
        opaqueState: st as *mut AllocState as *mut c_void,
    }
}

#[test]
fn frame_stream_allocation_failures() {
    let (cca, rca) = both::<FnCreateCctxAdv>("LZ4F_createCompressionContext_advanced");
    let (cfr, rfr) = both::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cb, rb) = both::<FnBegin>("LZ4F_compressBegin");

    // Allocation sites reached, in order:
    //   #1 LZ4F_cctx        (calloc, in createCompressionContext_advanced)
    //   #2 lz4CtxPtr        (malloc, in compressBegin_internal)
    //   #3 tmpBuff          (malloc, in compressBegin_internal)
    //   #4 lz4CtxPtr again  (second compressBegin needing a bigger ctx)
    for with_calloc in [false, true] {
        // ---- (1) failure of the cctx allocation itself
        {
            let mut cst = AllocState { calls: 0, fail_at: 1, live: 0 };
            let mut rst = AllocState { calls: 0, fail_at: 1, live: 0 };
            unsafe {
                let c = cca(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                let r = rca(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                assert!(c.is_null(), "C: cctx alloc failure must yield NULL");
                assert!(r.is_null(), "Rust: cctx alloc failure must yield NULL");
            }
            assert_eq!(cst.calls, rst.calls, "allocation call counts (site 1)");
            assert_eq!(cst.calls, 1);
            assert_eq!((cst.live, rst.live), (0, 0), "leak after site-1 failure");
        }

        // ---- (2)/(3) failures inside compressBegin
        for fail_at in [2u64, 3] {
            let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockIndependent, 1, 0);
            p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
            let mut cst = AllocState { calls: 0, fail_at, live: 0 };
            let mut rst = AllocState { calls: 0, fail_at, live: 0 };
            let mut cbuf = vec![SENTINEL; 64];
            let mut rbuf = vec![SENTINEL; 64];
            unsafe {
                let c = cca(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                let r = rca(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                assert!(!c.is_null() && !r.is_null());
                let a = cb(c, cbuf.as_mut_ptr() as *mut c_void, 64, &p as *const _);
                let b = rb(r, rbuf.as_mut_ptr() as *mut c_void, 64, &p as *const _);
                expect_err(
                    &format!("compressBegin with allocation #{} failing", fail_at),
                    a,
                    b,
                    err::ERROR_allocation_failed,
                );
                assert_bytes_eq(
                    &format!("alloc-fail #{}: dst untouched", fail_at),
                    &cbuf,
                    &rbuf,
                );
                same_ret("free after alloc failure", cfr(c), rfr(r));
            }
            assert_eq!(
                cst.calls, rst.calls,
                "allocation call counts (fail_at={})",
                fail_at
            );
            assert_eq!(cst.calls, fail_at, "expected to reach allocation #{}", fail_at);
            assert_eq!(
                (cst.live, rst.live),
                (0, 0),
                "leak after fail_at={} (C live={}, Rust live={})",
                fail_at,
                cst.live,
                rst.live
            );
        }

        // ---- (4) failure of the *second* lz4 context allocation (fast -> HC)
        {
            let mut cst = AllocState { calls: 0, fail_at: 4, live: 0 };
            let mut rst = AllocState { calls: 0, fail_at: 4, live: 0 };
            let mut p1 = base_prefs(LZ4F_max64KB, LZ4F_blockIndependent, 1, 0);
            p1.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
            let mut p2 = p1;
            p2.compressionLevel = 12;
            let mut cbuf = vec![SENTINEL; 64];
            let mut rbuf = vec![SENTINEL; 64];
            unsafe {
                let c = cca(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                let r = rca(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                let a = cb(c, cbuf.as_mut_ptr() as *mut c_void, 64, &p1 as *const _);
                let b = rb(r, rbuf.as_mut_ptr() as *mut c_void, 64, &p1 as *const _);
                same_ret("first compressBegin (custom alloc)", a, b);
                assert!(!lz4f_is_error(a), "first compressBegin should succeed");
                assert_bytes_eq("custom-alloc header", &cbuf, &rbuf);

                cbuf.fill(SENTINEL);
                rbuf.fill(SENTINEL);
                let a = cb(c, cbuf.as_mut_ptr() as *mut c_void, 64, &p2 as *const _);
                let b = rb(r, rbuf.as_mut_ptr() as *mut c_void, 64, &p2 as *const _);
                expect_err(
                    "second compressBegin with HC ctx allocation failing",
                    a,
                    b,
                    err::ERROR_allocation_failed,
                );
                assert_bytes_eq("alloc-fail #4: dst untouched", &cbuf, &rbuf);
                same_ret("free after alloc failure #4", cfr(c), rfr(r));
            }
            assert_eq!(cst.calls, rst.calls, "allocation call counts (site 4)");
            assert_eq!(cst.calls, 4);
            assert_eq!((cst.live, rst.live), (0, 0), "leak after site-4 failure");
        }

        // ---- (5) no injected failure: a whole frame through custom allocators
        {
            let mut rng = Rng::new(0xA110_C000_0000_0001);
            let input = gen_mixed(&mut rng, 200_000);
            let mut cst = AllocState { calls: 0, fail_at: 0, live: 0 };
            let mut rst = AllocState { calls: 0, fail_at: 0, live: 0 };
            let mut p = base_prefs(LZ4F_max256KB, LZ4F_blockLinked, 3, 0);
            p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
            p.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;
            unsafe {
                let c = cca(cmem_for(&mut cst, with_calloc), LZ4F_VERSION);
                let r = rca(cmem_for(&mut rst, with_calloc), LZ4F_VERSION);
                assert!(!c.is_null() && !r.is_null());
                let mut s = Sess {
                    c,
                    r,
                    ccd: ptr::null_mut(),
                    rcd: ptr::null_mut(),
                    prefs: p,
                    cout: Vec::new(),
                    rout: Vec::new(),
                    cbuf: Vec::new(),
                    rbuf: Vec::new(),
                    label: format!("customMem frame (calloc={})", with_calloc),
                    step: 0,
                    header_len: 0,
                };
                s.begin(BeginSpec::plain());
                s.drive(&input, &ops_fixed(input.len(), 33_333), None);
                s.end(None);
                let f = s.frame();
                assert_round_trip(&f, &input, "customMem frame");
                drop(s); // frees both contexts through the custom allocators
            }
            assert_eq!(cst.calls, rst.calls, "allocation call counts (success path)");
            assert!(cst.calls >= 3, "expected >= 3 allocations, got {}", cst.calls);
            assert_eq!(
                (cst.live, rst.live),
                (0, 0),
                "custom allocator leak: C={} Rust={}",
                cst.live,
                rst.live
            );
        }
    }
}

// ===========================================================================
// 11. Out-of-range enum values crossing the FFI
// ===========================================================================

#[test]
fn frame_stream_out_of_range_enums() {
    let mut rng = Rng::new(0xE0_0000_1234_5678);
    let src = gen_mixed(&mut rng, 1000);

    // ---- blockSizeID: compressBegin does not validate it at all.
    // LZ4F_getBlockSize() then returns an error code as maxBlockSize, so keep
    // autoFlush == 1 (no tmpBuff involvement) and a single small update.
    for &bsid in [-1i32, 1, 2, 3, 8, 9, 100, c_int::MIN, c_int::MAX].iter() {
        for &mode in [LZ4F_blockLinked, LZ4F_blockIndependent].iter() {
            let mut p = base_prefs(bsid, mode, 1, 1);
            p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
            let label = format!("bsid={} mode={}", bsid, mode);
            let mut s = Sess::new(&label, p);
            let hdr = s.begin(BeginSpec::plain());
            same_ret(&format!("{} begin", label), hdr, hdr);
            if lz4f_is_error(hdr) {
                continue;
            }
            // Size the buffer ourselves: LZ4F_compressBound is meaningless here.
            let cap = src.len() + 4096;
            let n = s.update_cap(false, &src, None, cap);
            if !lz4f_is_error(n) {
                s.end_cap(None, 4096);
            }
        }
    }
    // ... and one autoFlush == 0 case with a tiny payload (tmpBuff gets a
    // wrapped-around size, so nothing bigger than that may be buffered).
    for &bsid in [8i32, -1].iter() {
        let mut p = base_prefs(bsid, LZ4F_blockIndependent, 1, 0);
        p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
        let label = format!("bsid={} af=0", bsid);
        let mut s = Sess::new(&label, p);
        let hdr = s.begin(BeginSpec::plain());
        if lz4f_is_error(hdr) {
            continue;
        }
        let n = s.update_cap(false, &src[..64], None, 4096);
        if !lz4f_is_error(n) {
            s.flush_cap(None, 4096);
            s.end_cap(None, 4096);
        }
    }

    // ---- blockMode: only 0/1 have meaning; other values are masked with & 1 in
    // the header and compare unequal to both enum values elsewhere.
    for &mode in [2i32, 3, -1, 7, c_int::MIN, c_int::MAX].iter() {
        for &af in [0u32, 1].iter() {
            let mut p = base_prefs(LZ4F_max64KB, mode, 1, af);
            p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
            let label = format!("blockMode={} af={}", mode, af);
            let mut s = Sess::new(&label, p);
            let hdr = s.begin(BeginSpec::plain());
            assert!(!lz4f_is_error(hdr), "{}: begin failed {}", label, describe(hdr));
            let cap = bound_both(src.len(), &p) + 16;
            let n = s.update_cap(false, &src, None, cap);
            if !lz4f_is_error(n) {
                s.flush_cap(None, cap);
                s.end_cap(None, cap);
            }
        }
    }

    // ---- contentChecksumFlag / blockChecksumFlag: the header uses `& 1` while
    // the emit paths compare against the exact enum value, and blockChecksumFlag
    // is *multiplied* by BFSize.  Only C-vs-Rust agreement is asserted.
    for &cchk in [2i32, 3, -1].iter() {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockIndependent, 1, 1);
        p.frameInfo.contentChecksumFlag = cchk;
        let label = format!("contentChecksumFlag={}", cchk);
        let mut s = Sess::new(&label, p);
        let hdr = s.begin(BeginSpec::plain());
        assert!(!lz4f_is_error(hdr), "{}: begin failed", label);
        let cap = src.len() + 4096;
        let n = s.update_cap(false, &src, None, cap);
        if !lz4f_is_error(n) && n <= cap {
            s.end_cap(None, 4096);
        }
    }
    for &bchk in [2i32, 3].iter() {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockIndependent, 1, 1);
        p.frameInfo.blockChecksumFlag = bchk;
        let label = format!("blockChecksumFlag={}", bchk);
        let mut s = Sess::new(&label, p);
        let hdr = s.begin(BeginSpec::plain());
        assert!(!lz4f_is_error(hdr), "{}: begin failed", label);
        let cap = bound_both(src.len(), &p) + 16;
        let n = s.update_cap(false, &src, None, cap);
        if !lz4f_is_error(n) && n <= cap {
            s.end_cap(None, cap);
        }
    }
    // blockChecksumFlag == -1 wraps `(U32)crcFlag * BFSize` into a ~16GB stride,
    // so only a single block may be produced and the return value must not be
    // used for indexing; the point is that C and Rust wrap identically.
    {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockIndependent, 1, 1);
        p.frameInfo.blockChecksumFlag = -1;
        let mut s = Sess::new("blockChecksumFlag=-1", p);
        let hdr = s.begin(BeginSpec::plain());
        assert!(!lz4f_is_error(hdr), "begin failed");
        let cap = src.len() + 4096;
        let (cf, rf) = both::<FnUpdate>("LZ4F_compressUpdate");
        s.prep(cap);
        unsafe {
            let a = cf(
                s.c,
                s.cbuf.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                ptr::null(),
            );
            let b = rf(
                s.r,
                s.rbuf.as_mut_ptr() as *mut c_void,
                cap,
                src.as_ptr() as *const c_void,
                src.len(),
                ptr::null(),
            );
            same_ret("blockChecksumFlag=-1 compressUpdate", a, b);
        }
        s.cmp_dst(cap, "blockChecksumFlag=-1: dst buffer");
        // Do not call compressEnd: the context is in a wrapped state.
        s.reset_frame();
    }

    // ---- frameType is completely unused by the compression side: the frame must
    // be byte-identical to frameType == LZ4F_frame.
    {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
        p.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
        let reference = run_frame(
            "frameType reference",
            &p,
            BeginSpec::plain(),
            &src,
            &ops_fixed(src.len(), 300),
            None,
            true,
        );
        for &ft in [LZ4F_skippableFrame, 2, 3, -1, c_int::MAX].iter() {
            let mut q = p;
            q.frameInfo.frameType = ft;
            let f = run_frame(
                &format!("frameType={}", ft),
                &q,
                BeginSpec::plain(),
                &src,
                &ops_fixed(src.len(), 300),
                None,
                true,
            );
            assert_bytes_eq(
                &format!("frameType={} must not change the output", ft),
                &reference,
                &f,
            );
        }
    }

    // ---- dictID values are copied verbatim into the header.
    for &did in [0u32, 1, 0xDEAD_BEEF, u32::MAX].iter() {
        let mut p = base_prefs(LZ4F_max64KB, LZ4F_blockLinked, 1, 0);
        p.frameInfo.dictID = did;
        p.frameInfo.contentSize = src.len() as u64;
        let f = run_frame(
            &format!("dictID=0x{:x}", did),
            &p,
            BeginSpec::plain(),
            &src,
            &ops_fixed(src.len(), 700),
            None,
            true,
        );
        let expect_hdr = 7 + 8 + if did != 0 { 4 } else { 0 };
        assert!(f.len() > expect_hdr);
    }
}
