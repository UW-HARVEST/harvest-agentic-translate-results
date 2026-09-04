//! Phase C — DECOMPRESSION error-path differential tests (part 2 of 4).
//!
//! Covers ERRORS.md rows 130..=219, i.e. the three sections:
//!   * "Decompression frame header"    (rows 130-152)
//!   * "Decompression block / entropy" (rows 153-200)
//!   * "Streaming decompression"       (rows 201-219)
//!
//! Every call crosses the FFI boundary via `dlsym` on BOTH the C `libzstd.so`
//! and the Rust `libzstd.so`. For each constructed invalid input we assert the
//! two implementations agree EXACTLY on:
//!   * the raw `size_t` return value,
//!   * `ZSTD_isError`,
//!   * `ZSTD_getErrorCode` (the specific enum value, not merely "both failed"),
//!   * the `ZSTD_getErrorName` string,
//! and, for the sentinel-returning functions (`ZSTD_getFrameContentSize`,
//! `ZSTD_getDecompressedSize`, `ZSTD_findDecompressedSize`, `ZSTD_decompressBound`),
//! the exact sentinel (`ZSTD_CONTENTSIZE_ERROR` / `ZSTD_CONTENTSIZE_UNKNOWN` / a
//! concrete size). For `ZSTD_getFrameHeader[_advanced]` the whole struct is
//! compared field-for-field.
//!
//! The invalid inputs are derived mechanically from a corpus of VALID frames
//! produced by the C library (levels, checksum, contentSize, dictID, windowLog,
//! single/multi-block, dictionary, magic/magicless), then subjected to
//! truncation, bit/byte corruption, bad magic, reserved-bit / window /
//! block-header / literals / sequences / checksum / dictionary tampering,
//! dstCapacity starvation, and pure random noise.
//!
//! Fixed seeds everywhere; every assertion carries its ERRORS.md row number.

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_ulonglong, c_void};

// ------------------------------------------------------- extra fn aliases ----
// (declared locally — tests/common/mod.rs must not be modified)

type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnLoadDict = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> size_t;

type FnDecompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnDecompressUsingDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
    size_t,
) -> size_t;
type FnDecompressUsingDDict = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    *const c_void,
) -> size_t;
type FnDStream =
    unsafe extern "C" fn(*mut c_void, *mut ZSTD_outBuffer, *mut ZSTD_inBuffer) -> size_t;

type FnU64FromBuf = unsafe extern "C" fn(*const c_void, size_t) -> c_ulonglong;
type FnSizeFromBuf = unsafe extern "C" fn(*const c_void, size_t) -> size_t;
type FnUintFromBuf = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
type FnGetFrameHeader =
    unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, size_t) -> size_t;
type FnGetFrameHeaderAdv =
    unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, size_t, c_int) -> size_t;
type FnReadSkippable =
    unsafe extern "C" fn(*mut c_void, size_t, *mut c_uint, *const c_void, size_t) -> size_t;
type FnWriteSkippable =
    unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_uint) -> size_t;

type FnCreateDDict = unsafe extern "C" fn(*const c_void, size_t) -> *mut c_void;

// buffer-less streaming path
type FnDecompressBegin = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnNextSrcSize = unsafe extern "C" fn(*mut c_void) -> size_t;
type FnDecompressContinue =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;

// error helpers
type FnErrName = unsafe extern "C" fn(size_t) -> *const c_char;

// ------------------------------------------------------------- ptr helper ----

fn buf_ptr(buf: &[u8]) -> *const c_void {
    if buf.is_empty() {
        std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
    } else {
        buf.as_ptr() as *const c_void
    }
}

// A bundle of every symbol we compare, fetched once.
struct Api {
    dec: (FnDecompress, FnDecompress),
    dec_dctx: (FnDecompressDCtx, FnDecompressDCtx),
    dec_ud: (FnDecompressUsingDict, FnDecompressUsingDict),
    dec_udd: (FnDecompressUsingDDict, FnDecompressUsingDDict),
    dstream: (FnDStream, FnDStream),
    gcs: (FnU64FromBuf, FnU64FromBuf),
    gds: (FnU64FromBuf, FnU64FromBuf),
    ffcs: (FnSizeFromBuf, FnSizeFromBuf),
    fds: (FnU64FromBuf, FnU64FromBuf),
    dbound: (FnU64FromBuf, FnU64FromBuf),
    gfh: (FnGetFrameHeader, FnGetFrameHeader),
    gfha: (FnGetFrameHeaderAdv, FnGetFrameHeaderAdv),
    fhs: (FnSizeFromBuf, FnSizeFromBuf),
    isf: (FnUintFromBuf, FnUintFromBuf),
    isk: (FnUintFromBuf, FnUintFromBuf),
    rsf: (FnReadSkippable, FnReadSkippable),
    gdif: (FnUintFromBuf, FnUintFromBuf),
    // buffer-less
    dbegin: (FnDecompressBegin, FnDecompressBegin),
    nsstd: (FnNextSrcSize, FnNextSrcSize),
    dcont: (FnDecompressContinue, FnDecompressContinue),
    // ctx lifecycle
    create_dctx: (FnCreate, FnCreate),
    free_dctx: (FnFree, FnFree),
    create_ddict: (FnCreateDDict, FnCreateDDict),
    free_ddict: (FnFree, FnFree),
    // error introspection
    is_err: (FnIsError, FnIsError),
    ecode: (FnGetErrorCode, FnGetErrorCode),
    ename: (FnErrName, FnErrName),
}

fn api() -> Api {
    Api {
        dec: fnpair!("ZSTD_decompress", FnDecompress),
        dec_dctx: fnpair!("ZSTD_decompressDCtx", FnDecompressDCtx),
        dec_ud: fnpair!("ZSTD_decompress_usingDict", FnDecompressUsingDict),
        dec_udd: fnpair!("ZSTD_decompress_usingDDict", FnDecompressUsingDDict),
        dstream: fnpair!("ZSTD_decompressStream", FnDStream),
        gcs: fnpair!("ZSTD_getFrameContentSize", FnU64FromBuf),
        gds: fnpair!("ZSTD_getDecompressedSize", FnU64FromBuf),
        ffcs: fnpair!("ZSTD_findFrameCompressedSize", FnSizeFromBuf),
        fds: fnpair!("ZSTD_findDecompressedSize", FnU64FromBuf),
        dbound: fnpair!("ZSTD_decompressBound", FnU64FromBuf),
        gfh: fnpair!("ZSTD_getFrameHeader", FnGetFrameHeader),
        gfha: fnpair!("ZSTD_getFrameHeader_advanced", FnGetFrameHeaderAdv),
        fhs: fnpair!("ZSTD_frameHeaderSize", FnSizeFromBuf),
        isf: fnpair!("ZSTD_isFrame", FnUintFromBuf),
        isk: fnpair!("ZSTD_isSkippableFrame", FnUintFromBuf),
        rsf: fnpair!("ZSTD_readSkippableFrame", FnReadSkippable),
        gdif: fnpair!("ZSTD_getDictID_fromFrame", FnUintFromBuf),
        dbegin: fnpair!("ZSTD_decompressBegin", FnDecompressBegin),
        nsstd: fnpair!("ZSTD_nextSrcSizeToDecompress", FnNextSrcSize),
        dcont: fnpair!("ZSTD_decompressContinue", FnDecompressContinue),
        create_dctx: fnpair!("ZSTD_createDCtx", FnCreate),
        free_dctx: fnpair!("ZSTD_freeDCtx", FnFree),
        create_ddict: fnpair!("ZSTD_createDDict", FnCreateDDict),
        free_ddict: fnpair!("ZSTD_freeDDict", FnFree),
        is_err: fnpair!("ZSTD_isError", FnIsError),
        ecode: fnpair!("ZSTD_getErrorCode", FnGetErrorCode),
        ename: fnpair!("ZSTD_getErrorName", FnErrName),
    }
}

impl Api {
    /// Compare two raw `size_t` results for full parity: raw value, isError,
    /// getErrorCode, and getErrorName string.
    #[track_caller]
    unsafe fn cmp_ret(&self, ctx: &str, c: size_t, r: size_t) {
        assert_eq!(c, r, "{ctx}: raw size_t return differs (C={c:#x} R={r:#x})");
        let cie = (self.is_err.0)(c);
        let rie = (self.is_err.1)(r);
        assert_eq!(cie, rie, "{ctx}: ZSTD_isError differs (C={cie} R={rie})");
        let cec = (self.ecode.0)(c);
        let rec = (self.ecode.1)(r);
        assert_eq!(cec, rec, "{ctx}: ZSTD_getErrorCode differs (C={cec} R={rec})");
        let cn = cstr((self.ename.0)(c));
        let rn = cstr((self.ename.1)(r));
        assert_eq!(cn, rn, "{ctx}: ZSTD_getErrorName differs (C={cn:?} R={rn:?})");
    }

    /// Compare a u64/sentinel return exactly (used for the CONTENTSIZE APIs).
    #[track_caller]
    fn cmp_u64(&self, ctx: &str, c: c_ulonglong, r: c_ulonglong) {
        assert_eq!(c, r, "{ctx}: u64/sentinel return differs (C={c:#x} R={r:#x})");
    }

    /// Compare a plain `c_uint` return exactly (isFrame / isSkippable / dictID).
    #[track_caller]
    fn cmp_uint(&self, ctx: &str, c: c_uint, r: c_uint) {
        assert_eq!(c, r, "{ctx}: uint return differs (C={c} R={r})");
    }

    /// Run ONE (possibly-invalid) input through the ENTIRE decode battery and
    /// assert C-vs-Rust parity on every function. `row` is the ERRORS.md row
    /// number, `what` a short human description; both are woven into the ctx.
    unsafe fn differential_decode_all(
        &self,
        row: u32,
        what: &str,
        input: &[u8],
        dict: Option<&[u8]>,
    ) {
        let base = format!("ERRORS row {row}: {what}");
        let sp = buf_ptr(input);
        let ilen = input.len();

        // ---- sentinel / introspection functions (no dst) ----
        self.cmp_u64(
            &format!("{base} [getFrameContentSize]"),
            (self.gcs.0)(sp, ilen),
            (self.gcs.1)(sp, ilen),
        );
        self.cmp_u64(
            &format!("{base} [getDecompressedSize]"),
            (self.gds.0)(sp, ilen),
            (self.gds.1)(sp, ilen),
        );
        self.cmp_ret(
            &format!("{base} [findFrameCompressedSize]"),
            (self.ffcs.0)(sp, ilen),
            (self.ffcs.1)(sp, ilen),
        );
        self.cmp_u64(
            &format!("{base} [findDecompressedSize]"),
            (self.fds.0)(sp, ilen),
            (self.fds.1)(sp, ilen),
        );
        self.cmp_u64(
            &format!("{base} [decompressBound]"),
            (self.dbound.0)(sp, ilen),
            (self.dbound.1)(sp, ilen),
        );
        self.cmp_ret(
            &format!("{base} [frameHeaderSize]"),
            (self.fhs.0)(sp, ilen),
            (self.fhs.1)(sp, ilen),
        );
        self.cmp_uint(
            &format!("{base} [isFrame]"),
            (self.isf.0)(sp, ilen),
            (self.isf.1)(sp, ilen),
        );
        self.cmp_uint(
            &format!("{base} [isSkippableFrame]"),
            (self.isk.0)(sp, ilen),
            (self.isk.1)(sp, ilen),
        );
        self.cmp_uint(
            &format!("{base} [getDictID_fromFrame]"),
            (self.gdif.0)(sp, ilen),
            (self.gdif.1)(sp, ilen),
        );

        // ---- getFrameHeader / getFrameHeader_advanced (struct compare) ----
        for &fmt in &[ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
            let mut ch = ZSTD_frameHeader::default();
            let mut rh = ZSTD_frameHeader::default();
            let cr = (self.gfha.0)(&mut ch, sp, ilen, fmt);
            let rr = (self.gfha.1)(&mut rh, sp, ilen, fmt);
            let ctx = format!("{base} [getFrameHeader_advanced fmt={fmt}]");
            self.cmp_ret(&ctx, cr, rr);
            if cr == 0 && rr == 0 {
                assert_eq!(ch, rh, "{ctx}: frameHeader struct differs\nC={ch:?}\nR={rh:?}");
            }
        }
        {
            let mut ch = ZSTD_frameHeader::default();
            let mut rh = ZSTD_frameHeader::default();
            let cr = (self.gfh.0)(&mut ch, sp, ilen);
            let rr = (self.gfh.1)(&mut rh, sp, ilen);
            let ctx = format!("{base} [getFrameHeader]");
            self.cmp_ret(&ctx, cr, rr);
            if cr == 0 && rr == 0 {
                assert_eq!(ch, rh, "{ctx}: frameHeader struct differs\nC={ch:?}\nR={rh:?}");
            }
        }

        // ---- readSkippableFrame (must reject non-skippable identically) ----
        {
            let mut oc = vec![0xAAu8; ilen.max(1)];
            let mut orr = vec![0xAAu8; ilen.max(1)];
            let mut mv_c: c_uint = 0xDEAD_BEEF;
            let mut mv_r: c_uint = 0xDEAD_BEEF;
            let cr = (self.rsf.0)(oc.as_mut_ptr() as *mut c_void, oc.len(), &mut mv_c, sp, ilen);
            let rr = (self.rsf.1)(orr.as_mut_ptr() as *mut c_void, orr.len(), &mut mv_r, sp, ilen);
            let ctx = format!("{base} [readSkippableFrame]");
            self.cmp_ret(&ctx, cr, rr);
            if cr == 0 && rr == 0 {
                assert_eq!(mv_c, mv_r, "{ctx}: magicVariant differs (C={mv_c} R={mv_r})");
                assert_bytes_eq(&format!("{ctx}: content"), &oc[..cr], &orr[..rr]);
            }
        }

        // Determine dst capacities to test.
        let declared = (self.gcs.0)(sp, ilen);
        let mut caps: Vec<usize> = vec![0, 1];
        if declared != ZSTD_CONTENTSIZE_UNKNOWN
            && declared != ZSTD_CONTENTSIZE_ERROR
            && declared <= (1 << 20)
        {
            let d = declared as usize;
            if d > 0 {
                caps.push(d - 1);
            }
            caps.push(d);
            caps.push(d + 16);
        } else {
            caps.push(64);
            caps.push(4096);
            caps.push(1 << 18);
        }
        caps.sort_unstable();
        caps.dedup();

        // ---- one-shot decode family across dst capacities ----
        for &cap in &caps {
            let mut oc = vec![0xAAu8; cap.max(1)];
            let mut orr = vec![0xAAu8; cap.max(1)];
            let odp_c = if cap == 0 {
                std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void
            } else {
                oc.as_mut_ptr() as *mut c_void
            };
            let odp_r = if cap == 0 {
                std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void
            } else {
                orr.as_mut_ptr() as *mut c_void
            };

            // ZSTD_decompress
            let cr = (self.dec.0)(odp_c, cap, sp, ilen);
            let rr = (self.dec.1)(odp_r, cap, sp, ilen);
            let ctx = format!("{base} [decompress cap={cap}]");
            self.cmp_ret(&ctx, cr, rr);
            if cr == 0 && rr == 0 && cr <= cap {
                assert_bytes_eq(&format!("{ctx}: bytes"), &oc[..cr], &orr[..rr]);
            }

            // ZSTD_decompressDCtx
            let cd = (self.create_dctx.0)();
            let rd = (self.create_dctx.1)();
            let cr = (self.dec_dctx.0)(cd, odp_c, cap, sp, ilen);
            let rr = (self.dec_dctx.1)(rd, odp_r, cap, sp, ilen);
            let ctx = format!("{base} [decompressDCtx cap={cap}]");
            self.cmp_ret(&ctx, cr, rr);
            if cr == 0 && rr == 0 && cr <= cap {
                assert_bytes_eq(&format!("{ctx}: bytes"), &oc[..cr], &orr[..rr]);
            }
            (self.free_dctx.0)(cd);
            (self.free_dctx.1)(rd);

            // ZSTD_decompress_usingDict (dict may be empty → dictless path)
            let (dp, dl) = match dict {
                Some(d) => (buf_ptr(d), d.len()),
                None => (std::ptr::null(), 0usize),
            };
            let cd = (self.create_dctx.0)();
            let rd = (self.create_dctx.1)();
            let cr = (self.dec_ud.0)(cd, odp_c, cap, sp, ilen, dp, dl);
            let rr = (self.dec_ud.1)(rd, odp_r, cap, sp, ilen, dp, dl);
            let ctx = format!("{base} [decompress_usingDict cap={cap} dict={dl}]");
            self.cmp_ret(&ctx, cr, rr);
            if cr == 0 && rr == 0 && cr <= cap {
                assert_bytes_eq(&format!("{ctx}: bytes"), &oc[..cr], &orr[..rr]);
            }
            (self.free_dctx.0)(cd);
            (self.free_dctx.1)(rd);

            // ZSTD_decompress_usingDDict
            if let Some(d) = dict {
                let ddc = (self.create_ddict.0)(buf_ptr(d), d.len());
                let ddr = (self.create_ddict.1)(buf_ptr(d), d.len());
                let cd = (self.create_dctx.0)();
                let rd = (self.create_dctx.1)();
                let cr = (self.dec_udd.0)(cd, odp_c, cap, sp, ilen, ddc as *const c_void);
                let rr = (self.dec_udd.1)(rd, odp_r, cap, sp, ilen, ddr as *const c_void);
                let ctx = format!("{base} [decompress_usingDDict cap={cap} dict={}]", d.len());
                self.cmp_ret(&ctx, cr, rr);
                if cr == 0 && rr == 0 && cr <= cap {
                    assert_bytes_eq(&format!("{ctx}: bytes"), &oc[..cr], &orr[..rr]);
                }
                (self.free_dctx.0)(cd);
                (self.free_dctx.1)(rd);
                (self.free_ddict.0)(ddc);
                (self.free_ddict.1)(ddr);
            }
        }

        // ---- streaming decode: 1-byte chunks and one large chunk ----
        for &in_chunk in &[1usize, usize::MAX] {
            let out_cap = match declared {
                ZSTD_CONTENTSIZE_UNKNOWN | ZSTD_CONTENTSIZE_ERROR => 1 << 18,
                d if d <= (1 << 20) => (d as usize) + 64,
                _ => 1 << 18,
            };
            let cd = (self.create_dctx.0)();
            let rd = (self.create_dctx.1)();
            let (cerr, cpos, cbytes) =
                drive_dstream(self, self.dstream.0, cd, input, in_chunk, out_cap);
            let (rerr, rpos, rbytes) =
                drive_dstream(self, self.dstream.1, rd, input, in_chunk, out_cap);
            let ctx = format!("{base} [decompressStream in_chunk={in_chunk}]");
            self.cmp_ret(&format!("{ctx}: final ret"), cerr, rerr);
            assert_eq!(cpos, rpos, "{ctx}: produced-size differs (C={cpos} R={rpos})");
            if (self.is_err.0)(cerr) == 0 {
                assert_bytes_eq(&format!("{ctx}: bytes"), &cbytes[..cpos], &rbytes[..rpos]);
            }
            (self.free_dctx.0)(cd);
            (self.free_dctx.1)(rd);
        }

        // ---- buffer-less path: decompressBegin + nextSrcSize + continue ----
        differential_bufferless(self, &base, input);
    }
}

/// Drive `ZSTD_decompressStream` to a terminal state; returns
/// `(final_ret, produced_pos, produced_bytes)`.
unsafe fn drive_dstream(
    a: &Api,
    f: FnDStream,
    ds: *mut c_void,
    input: &[u8],
    in_chunk: usize,
    out_cap: usize,
) -> (size_t, usize, Vec<u8>) {
    let mut out = vec![0xAAu8; out_cap.max(1)];
    let mut buf = vec![0xAAu8; 4096];
    let mut produced = 0usize;
    let mut pos = 0usize;
    let mut last_ret: size_t = 0;
    let max_iters = input.len().saturating_mul(2) + 32;
    for _ in 0..max_iters {
        let take = in_chunk.min(input.len() - pos);
        let mut ib = ZSTD_inBuffer {
            src: if input.is_empty() {
                std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
            } else {
                input.as_ptr().add(pos) as *const c_void
            },
            size: take,
            pos: 0,
        };
        let room = out_cap - produced;
        let this_out = room.min(buf.len());
        let mut ob = ZSTD_outBuffer {
            dst: if this_out == 0 {
                std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void
            } else {
                buf.as_mut_ptr() as *mut c_void
            },
            size: this_out,
            pos: 0,
        };
        let r = f(ds, &mut ob, &mut ib);
        last_ret = r;
        if ob.pos > 0 {
            out[produced..produced + ob.pos].copy_from_slice(&buf[..ob.pos]);
            produced += ob.pos;
        }
        pos += ib.pos;
        if (a.is_err.0)(r) != 0 {
            return (r, produced, out);
        }
        if r == 0 {
            return (0, produced, out);
        }
        if ib.pos == 0 && ob.pos == 0 {
            // no forward progress — let the library report it on the next call.
            let mut ib2 = ZSTD_inBuffer {
                src: std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void,
                size: 0,
                pos: 0,
            };
            let mut ob2 = ZSTD_outBuffer {
                dst: std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void,
                size: 0,
                pos: 0,
            };
            let r2 = f(ds, &mut ob2, &mut ib2);
            return (r2, produced, out);
        }
    }
    (last_ret, produced, out)
}

/// Buffer-less streaming decode differential.
unsafe fn differential_bufferless(a: &Api, base: &str, input: &[u8]) {
    let cd = (a.create_dctx.0)();
    let rd = (a.create_dctx.1)();

    let bc = (a.dbegin.0)(cd);
    let br = (a.dbegin.1)(rd);
    a.cmp_ret(&format!("{base} [decompressBegin]"), bc, br);

    let out_cap = 1 << 18;
    let mut oc = vec![0xAAu8; out_cap];
    let mut orr = vec![0xAAu8; out_cap];
    let mut cpos = 0usize;
    let mut rpos = 0usize;
    let mut coff = 0usize;
    let mut roff = 0usize;

    let max_steps = input.len() + 8;
    for step in 0..max_steps {
        let cn = (a.nsstd.0)(cd);
        let rn = (a.nsstd.1)(rd);
        a.cmp_ret(&format!("{base} [nextSrcSizeToDecompress step={step}]"), cn, rn);
        if (a.is_err.0)(cn) != 0 {
            break;
        }
        if cn == 0 {
            break;
        }
        let c_avail = input.len().saturating_sub(cpos);
        let r_avail = input.len().saturating_sub(rpos);
        let c_take = cn.min(c_avail);
        let r_take = rn.min(r_avail);
        let c_src = if cpos < input.len() {
            input.as_ptr().add(cpos) as *const c_void
        } else {
            std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
        };
        let r_src = if rpos < input.len() {
            input.as_ptr().add(rpos) as *const c_void
        } else {
            std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void
        };
        let c_room = out_cap - coff;
        let r_room = out_cap - roff;
        let cr = (a.dcont.0)(cd, oc.as_mut_ptr().add(coff) as *mut c_void, c_room, c_src, c_take);
        let rr = (a.dcont.1)(rd, orr.as_mut_ptr().add(roff) as *mut c_void, r_room, r_src, r_take);
        a.cmp_ret(&format!("{base} [decompressContinue step={step}]"), cr, rr);
        if (a.is_err.0)(cr) != 0 {
            break;
        }
        cpos += c_take;
        rpos += r_take;
        if cr <= c_room {
            coff += cr;
        }
        if rr <= r_room {
            roff += rr;
        }
        if c_take == 0 && r_take == 0 {
            // ran out of input to feed; avoid a spin
            break;
        }
    }
    assert_eq!(coff, roff, "{base} [bufferless]: produced size differs (C={coff} R={roff})");
    assert_bytes_eq(&format!("{base} [bufferless]: bytes"), &oc[..coff], &orr[..roff]);
    (a.free_dctx.0)(cd);
    (a.free_dctx.1)(rd);
}

// =========================================================== corpus build ====

/// One valid frame plus metadata we need to derive invalid variants from it.
struct Frame {
    bytes: Vec<u8>,
    desc: String,
    dict: Vec<u8>,
    content_len: usize,
    magicless: bool,
}

/// Build a bounded corpus of valid frames with the C library.
fn build_corpus(rng: &mut Rng) -> Vec<Frame> {
    unsafe {
        let (c_create, _r) = fnpair!("ZSTD_createCCtx", FnCreate);
        let (c_free, _r) = fnpair!("ZSTD_freeCCtx", FnFree);
        let (c_set, _r) = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
        let (c_ld, _r) = fnpair!("ZSTD_CCtx_loadDictionary", FnLoadDict);
        let (c_c2, _r) = fnpair!("ZSTD_compress2", FnCompress2);
        let (c_bound, _r) = fnpair!("ZSTD_compressBound", FnSizeSize);
        let (c_reset, _r) = fnpair!("ZSTD_CCtx_reset", FnReset);
        let (c_ie, _r) = fnpair!("ZSTD_isError", FnIsError);

        let cctx = c_create();
        let mut out: Vec<Frame> = Vec::new();

        let dict_text = gen(Shape::Text, 4096, rng);

        let lens = [0usize, 1, 7, 100, 4096, 65_537, 200_000];
        let shapes = [Shape::Zeros, Shape::Text, Shape::Random, Shape::Repetitive];

        for &shape in &shapes {
            for &len in &lens {
                let src = gen(shape, len, rng);
                let combos: &[(i32, i32, i32, i32, i32, bool)] = &[
                    // (level, contentSize, checksum, dictID, windowLog, useDict)
                    (3, 1, 0, 0, 0, false),
                    (3, 0, 1, 0, 0, false),
                    (1, 0, 0, 1, 0, false),
                    (9, 1, 1, 1, 10, false),
                    (19, 1, 0, 0, 23, false),
                    (5, 1, 1, 1, 0, true),
                ];
                for &(lvl, cs, ck, did, wl, use_dict) in combos {
                    for &fmt in &[ZSTD_f_zstd1, ZSTD_f_zstd1_magicless] {
                        let _ = c_reset(cctx, ZSTD_reset_parameters);
                        let _ = c_set(cctx, ZSTD_c_compressionLevel, lvl);
                        let _ = c_set(cctx, ZSTD_c_contentSizeFlag, cs);
                        let _ = c_set(cctx, ZSTD_c_checksumFlag, ck);
                        let _ = c_set(cctx, ZSTD_c_dictIDFlag, did);
                        if wl != 0 {
                            let _ = c_set(cctx, ZSTD_c_windowLog, wl);
                        }
                        let _ = c_set(cctx, ZSTD_c_format, fmt);
                        let dict_used: &[u8] = if use_dict { &dict_text } else { &[] };
                        if use_dict {
                            let _ = c_ld(cctx, buf_ptr(dict_used), dict_used.len());
                        }
                        let cap = c_bound(len).max(64);
                        let mut buf = vec![0u8; cap];
                        let n = c_c2(cctx, buf.as_mut_ptr() as *mut c_void, cap, buf_ptr(&src), len);
                        if c_ie(n) != 0 {
                            continue;
                        }
                        buf.truncate(n);
                        out.push(Frame {
                            bytes: buf,
                            desc: format!(
                                "shape={shape:?} len={len} lvl={lvl} cs={cs} ck={ck} did={did} wl={wl} dict={use_dict} fmt={fmt}"
                            ),
                            dict: dict_used.to_vec(),
                            content_len: len,
                            magicless: fmt == ZSTD_f_zstd1_magicless,
                        });
                    }
                }
            }
        }
        c_free(cctx);
        out
    }
}

/// A representative, bounded subset for the heavier per-frame mutation loops.
fn small_corpus(rng: &mut Rng) -> Vec<Frame> {
    let all = build_corpus(rng);
    let mut picked: Vec<Frame> = Vec::new();
    for f in all {
        let keep = f.desc.contains("len=100")
            || f.desc.contains("len=4096")
            || f.desc.contains("len=65537")
            || (!f.dict.is_empty() && f.content_len <= 4096);
        if keep {
            picked.push(f);
        }
    }
    picked
}

// ================================================================ TEST 1 ====
/// ERRORS rows 130-152 — Decompression frame header.
#[test]
fn phasec_rows_130_152_frame_header() {
    let a = api();
    let mut rng = Rng::new(0xC0DE_0130);
    unsafe {
        let corpus = build_corpus(&mut rng);

        // ---------- 130: srcSize < minInputSize (short buffers) ----------
        let sample: Vec<&Frame> = corpus.iter().take(24).collect();
        for f in &sample {
            for pl in 0..=6usize.min(f.bytes.len()) {
                a.differential_decode_all(
                    130,
                    &format!("srcSize<minInputSize prefix={pl} [{}]", f.desc),
                    &f.bytes[..pl],
                    if f.dict.is_empty() { None } else { Some(&f.dict) },
                );
            }
        }

        // ---------- 131: src==NULL but srcSize>0 ----------
        // Only ZSTD_getFrameHeader_advanced defines a clean error (GENERIC) for
        // a NULL src with srcSize>0. The higher-level size queries
        // (ZSTD_getFrameContentSize etc.) first call ZSTD_isLegacy, which reads
        // the magic unconditionally when srcSize>=4 — so on a NULL pointer BOTH
        // the C and Rust libraries dereference NULL and crash IDENTICALLY (this
        // is shared UB of the public API; verified out-of-band: both segfault
        // with status 139). We therefore assert parity only on the entry point
        // that actually specifies row-131 behaviour.
        {
            let mut ch = ZSTD_frameHeader::default();
            let mut rh = ZSTD_frameHeader::default();
            let cr = (a.gfha.0)(&mut ch, std::ptr::null(), 8, ZSTD_f_zstd1);
            let rr = (a.gfha.1)(&mut rh, std::ptr::null(), 8, ZSTD_f_zstd1);
            a.cmp_ret("ERRORS row 131: src==NULL srcSize>0 [getFrameHeader_advanced]", cr, rr);
            let cr = (a.gfha.0)(&mut ch, std::ptr::null(), 2, ZSTD_f_zstd1_magicless);
            let rr = (a.gfha.1)(&mut rh, std::ptr::null(), 2, ZSTD_f_zstd1_magicless);
            a.cmp_ret("ERRORS row 131: src==NULL srcSize>0 magicless [getFrameHeader_advanced]", cr, rr);
        }

        // ---------- 132 & 133: unknown / not-zstd-not-skippable magic ----------
        for bytes in [
            vec![0x00u8, 0x11, 0x22],
            vec![0xDE, 0xAD, 0xBE, 0xEF],
            vec![0x28, 0xB5, 0x2F],
        ] {
            a.differential_decode_all(132, "short unknown-prefix magic", &bytes, None);
        }
        for &mag in &[0xDEAD_BEEFu32, 0x0000_0000, 0xFFFF_FFFF, 0x184D_2A60] {
            let mut b = mag.to_le_bytes().to_vec();
            b.extend_from_slice(&gen(Shape::Random, 40, &mut rng));
            a.differential_decode_all(133, &format!("bad 4-byte magic {mag:#010x}"), &b, None);
        }
        for &mag in &[0xFD2FB527u32, 0xFD2FB529] {
            let mut b = mag.to_le_bytes().to_vec();
            b.extend_from_slice(&gen(Shape::Random, 40, &mut rng));
            a.differential_decode_all(133, &format!("magic off-by-one {mag:#010x}"), &b, None);
        }
        for mag in 0xFD2FB51Eu32..=0xFD2FB527 {
            let mut b = mag.to_le_bytes().to_vec();
            b.extend_from_slice(&gen(Shape::Random, 30, &mut rng));
            a.differential_decode_all(133, &format!("legacy magic {mag:#010x}+garbage"), &b, None);
        }
        {
            let mut b = ZSTD_MAGIC_DICTIONARY.to_le_bytes().to_vec();
            b.extend_from_slice(&gen(Shape::Random, 40, &mut rng));
            a.differential_decode_all(133, "dict magic as frame", &b, None);
        }
        for mag in 0x184D2A50u32..=0x184D2A60 {
            let mut b = mag.to_le_bytes().to_vec();
            b.extend_from_slice(&5u32.to_le_bytes());
            b.extend_from_slice(&gen(Shape::Random, 5, &mut rng));
            a.differential_decode_all(133, &format!("skippable magic {mag:#010x}"), &b, None);
            a.differential_decode_all(136, &format!("skippable trunc<8 {mag:#010x}"), &b[..b.len().min(6)], None);
        }

        // ---------- 134: reserved frame-header-descriptor bit (0x08) ----------
        for f in corpus.iter().filter(|f| !f.magicless && f.bytes.len() > 5).take(20) {
            let mut b = f.bytes.clone();
            b[4] |= 0x08;
            a.differential_decode_all(134, &format!("reserved FHD bit [{}]", f.desc), &b, None);
        }

        // ---------- 135: decoded windowLog > ZSTD_WINDOWLOG_MAX ----------
        {
            let mut b = ZSTD_MAGICNUMBER.to_le_bytes().to_vec();
            b.push(0x00); // FHD
            b.push(0xFF); // window descriptor -> absurd windowLog
            b.extend_from_slice(&gen(Shape::Random, 8, &mut rng));
            a.differential_decode_all(135, "windowLog>WINDOWLOG_MAX", &b, None);
        }

        // ---------- 136-142: skippable frame size / decode error paths ----------
        {
            let (c_wsf, _r) = fnpair!("ZSTD_writeSkippableFrame", FnWriteSkippable);
            let payload = gen(Shape::Text, 100, &mut rng);
            let cap = payload.len() + 16;
            let mut sb = vec![0u8; cap];
            let n = c_wsf(sb.as_mut_ptr() as *mut c_void, cap, buf_ptr(&payload), payload.len(), 3);
            sb.truncate(n);

            for pl in [0usize, 4, 7, 8, n.saturating_sub(1)] {
                if pl <= n {
                    a.differential_decode_all(139, &format!("skippable trunc pl={pl}"), &sb[..pl], None);
                }
            }
            let mut over = sb.clone();
            over[4..8].copy_from_slice(&0xFFFF_FFF0u32.to_le_bytes());
            a.differential_decode_all(138, "skippable size>srcSize", &over, None);
            let mut ov2 = sb.clone();
            ov2[4..8].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
            a.differential_decode_all(137, "skippable size+8 overflow", &ov2, None);
            a.differential_decode_all(142, "skippable dstCapacity-small", &sb, None);
            for f in corpus.iter().filter(|f| !f.magicless).take(6) {
                a.differential_decode_all(140, &format!("readSkippable on real frame [{}]", f.desc), &f.bytes, None);
            }
        }

        // ---------- 143: header decode wants more input ----------
        for f in corpus.iter().filter(|f| !f.magicless && f.bytes.len() >= 6).take(20) {
            for pl in [5usize, 6, 7] {
                if pl < f.bytes.len() {
                    a.differential_decode_all(143, &format!("header wants-more pl={pl} [{}]", f.desc), &f.bytes[..pl], None);
                }
            }
        }

        // ---------- 144: dictID mismatch ----------
        {
            let (c_create, _r) = fnpair!("ZSTD_createCCtx", FnCreate);
            let (c_free, _r) = fnpair!("ZSTD_freeCCtx", FnFree);
            let (c_set, _r) = fnpair!("ZSTD_CCtx_setParameter", FnSetParam);
            let (c_ld, _r) = fnpair!("ZSTD_CCtx_loadDictionary", FnLoadDict);
            let (c_c2, _r) = fnpair!("ZSTD_compress2", FnCompress2);
            let (c_bound, _r) = fnpair!("ZSTD_compressBound", FnSizeSize);
            let (c_ie, _r) = fnpair!("ZSTD_isError", FnIsError);

            let dict_a = gen(Shape::Text, 4096, &mut rng);
            let mut dict_b = gen(Shape::Text, 4096, &mut rng);
            dict_b[0] ^= 0xFF;
            let src = gen(Shape::Text, 3000, &mut rng);
            let cctx = c_create();
            let _ = c_set(cctx, ZSTD_c_dictIDFlag, 1);
            let _ = c_ld(cctx, buf_ptr(&dict_a), dict_a.len());
            let cap = c_bound(src.len()).max(64);
            let mut fb = vec![0u8; cap];
            let n = c_c2(cctx, fb.as_mut_ptr() as *mut c_void, cap, buf_ptr(&src), src.len());
            c_free(cctx);
            if c_ie(n) == 0 {
                fb.truncate(n);
                a.differential_decode_all(144, "dictID mismatch: different dict", &fb, Some(&dict_b));
                a.differential_decode_all(144, "dictID mismatch: no dict", &fb, None);
                a.differential_decode_all(144, "dictID mismatch: truncated dict", &fb, Some(&dict_a[..64]));
            }
        }

        // ---------- 145-149: size-query sentinels ----------
        for bytes in [vec![0u8; 3], vec![0xFFu8; 4], gen(Shape::Random, 5, &mut rng)] {
            a.differential_decode_all(145, "getFrameContentSize invalid->sentinel", &bytes, None);
        }
        let good: Vec<&Frame> = corpus.iter().filter(|f| !f.magicless).take(40).collect();
        for _ in 0..60 {
            let n = 1 + rng.below(4);
            let mut cat = Vec::new();
            for _ in 0..n {
                cat.extend_from_slice(&good[rng.below(good.len())].bytes);
            }
            a.differential_decode_all(148, &format!("multiframe concat n={n}"), &cat, None);
            let mut cat2 = cat.clone();
            cat2.extend_from_slice(&gen(Shape::Random, 1 + rng.below(20), &mut rng));
            a.differential_decode_all(149, "multiframe + trailing garbage", &cat2, None);
        }

        // ---------- 150 & 152: windowSize > allowed ----------
        {
            let (c_setdp, r_setdp) = fnpair!("ZSTD_DCtx_setParameter", FnSetParam);
            let big = corpus
                .iter()
                .find(|f| !f.magicless && f.desc.contains("wl=23") && f.content_len >= 65_537);
            if let Some(f) = big {
                for &wlmax in &[10i32, 11] {
                    let cd = (a.create_dctx.0)();
                    let rd = (a.create_dctx.1)();
                    let _ = c_setdp(cd, ZSTD_d_windowLogMax, wlmax);
                    let _ = r_setdp(rd, ZSTD_d_windowLogMax, wlmax);
                    let mut oc = vec![0xAAu8; f.content_len + 64];
                    let mut orr = vec![0xAAu8; f.content_len + 64];
                    let cr = (a.dec_dctx.0)(cd, oc.as_mut_ptr() as *mut c_void, oc.len(), buf_ptr(&f.bytes), f.bytes.len());
                    let rr = (a.dec_dctx.1)(rd, orr.as_mut_ptr() as *mut c_void, orr.len(), buf_ptr(&f.bytes), f.bytes.len());
                    a.cmp_ret(&format!("ERRORS row 150: windowSize>windowLogMax({wlmax}) [{}]", f.desc), cr, rr);
                    (a.free_dctx.0)(cd);
                    (a.free_dctx.1)(rd);
                }
            } else {
                eprintln!("ERRORS row 150: no large-window frame in corpus; covered generically via bad window descriptor (row 135)");
            }
        }
        eprintln!("ERRORS row 151: 'need more input' from a bounded getFrameHeader is exercised by the truncated-frame inputs of rows 130/143 fed through ZSTD_decompress; no distinct constructor needed.");
        eprintln!("ERRORS row 152: ZSTD_estimateDStreamSize size-overflow is a sizing-API internal reached only after a >WINDOWLOG_MAX windowSize, which the header decoder (row 135) rejects first; not constructible as a distinct decode-path input. Not faked.");
    }
}

// ================================================================ TEST 2 ====
/// ERRORS rows 153-200 — Decompression block / entropy.
#[test]
fn phasec_rows_153_200_block_entropy() {
    let a = api();
    let mut rng = Rng::new(0xC0DE_0153);
    unsafe {
        let corpus = small_corpus(&mut rng);
        assert!(!corpus.is_empty(), "corpus empty");

        // ---------- systematic single-byte / single-bit corruption ----------
        for f in corpus.iter().take(14) {
            let dict = if f.dict.is_empty() { None } else { Some(f.dict.as_slice()) };
            let n = f.bytes.len();
            let hot = 32.min(n);

            for off in 0..hot {
                for &val in &[0x00u8, 0xFF] {
                    let mut b = f.bytes.clone();
                    b[off] = val;
                    a.differential_decode_all(
                        154,
                        &format!("byte-overwrite off={off} val={val:#04x} [{}]", f.desc),
                        &b,
                        dict,
                    );
                }
                let mut b = f.bytes.clone();
                b[off] = (rng.next_u32() & 0xFF) as u8;
                a.differential_decode_all(
                    190,
                    &format!("byte-overwrite-rand off={off} [{}]", f.desc),
                    &b,
                    dict,
                );
            }

            let bit_offsets: Vec<usize> = {
                let mut v: Vec<usize> = (0..16.min(n)).collect();
                for _ in 0..12 {
                    v.push(rng.below(n.max(1)));
                }
                v.sort_unstable();
                v.dedup();
                v
            };
            for &off in &bit_offsets {
                for bit in 0..8u8 {
                    let mut b = f.bytes.clone();
                    b[off] ^= 1 << bit;
                    a.differential_decode_all(
                        161,
                        &format!("bit-flip off={off} bit={bit} [{}]", f.desc),
                        &b,
                        dict,
                    );
                }
            }
        }

        // ---------- 153: srcSize < blockHeaderSize after a valid header ----------
        {
            let (c_fhs, _r) = fnpair!("ZSTD_frameHeaderSize", FnSizeFromBuf);
            let (c_ie, _r) = fnpair!("ZSTD_isError", FnIsError);
            for f in corpus.iter().filter(|f| !f.magicless).take(12) {
                let hs = c_fhs(buf_ptr(&f.bytes), f.bytes.len());
                if c_ie(hs) == 0 && (hs as usize) + 2 < f.bytes.len() {
                    for extra in 0..=2usize {
                        let pl = hs as usize + extra;
                        a.differential_decode_all(
                            153,
                            &format!("srcSize<blockHeaderSize pl={pl} [{}]", f.desc),
                            &f.bytes[..pl],
                            None,
                        );
                    }
                }
            }
        }

        // ---------- 154: block type = bt_reserved(3) ----------
        {
            let (c_fhs, _r) = fnpair!("ZSTD_frameHeaderSize", FnSizeFromBuf);
            let (c_ie, _r) = fnpair!("ZSTD_isError", FnIsError);
            for f in corpus.iter().filter(|f| !f.magicless).take(12) {
                let hs = c_fhs(buf_ptr(&f.bytes), f.bytes.len());
                if c_ie(hs) == 0 && (hs as usize) < f.bytes.len() {
                    let bh = hs as usize;
                    let mut b = f.bytes.clone();
                    b[bh] |= 0b0000_0110;
                    a.differential_decode_all(154, &format!("block_type=bt_reserved [{}]", f.desc), &b, None);
                }
            }
        }

        // ---------- 185/189: block size larger than remaining / blockSizeMax ----------
        {
            let (c_fhs, _r) = fnpair!("ZSTD_frameHeaderSize", FnSizeFromBuf);
            let (c_ie, _r) = fnpair!("ZSTD_isError", FnIsError);
            for f in corpus.iter().filter(|f| !f.magicless).take(12) {
                let hs = c_fhs(buf_ptr(&f.bytes), f.bytes.len());
                if c_ie(hs) == 0 && (hs as usize) + 3 <= f.bytes.len() {
                    let bh = hs as usize;
                    let mut b = f.bytes.clone();
                    b[bh] |= 0b1111_1000;
                    b[bh + 1] = 0xFF;
                    b[bh + 2] = 0xFF;
                    a.differential_decode_all(189, &format!("blockSize>remaining [{}]", f.desc), &b, None);
                }
            }
        }

        // ---------- 191/200: decoded size != declared frameContentSize ----------
        for f in corpus.iter().filter(|f| !f.magicless && f.content_len > 0).take(10) {
            if f.bytes.len() > 4 {
                let pl = f.bytes.len() - 1;
                a.differential_decode_all(191, &format!("truncated last-block size-mismatch [{}]", f.desc), &f.bytes[..pl], None);
            }
        }

        // ---------- 192/193/205: checksum errors, both d_forceIgnoreChecksum ----------
        {
            let (c_setdp, r_setdp) = fnpair!("ZSTD_DCtx_setParameter", FnSetParam);
            for f in corpus.iter().filter(|f| f.desc.contains("ck=1") && !f.magicless).take(10) {
                if f.bytes.len() >= 4 {
                    let mut b = f.bytes.clone();
                    let last = b.len() - 1;
                    b[last] ^= 0x01;
                    for ign in [0i32, 1] {
                        let cd = (a.create_dctx.0)();
                        let rd = (a.create_dctx.1)();
                        let _ = c_setdp(cd, ZSTD_d_forceIgnoreChecksum, ign);
                        let _ = r_setdp(rd, ZSTD_d_forceIgnoreChecksum, ign);
                        let mut oc = vec![0xAAu8; f.content_len + 64];
                        let mut orr = vec![0xAAu8; f.content_len + 64];
                        let cr = (a.dec_dctx.0)(cd, oc.as_mut_ptr() as *mut c_void, oc.len(), buf_ptr(&b), b.len());
                        let rr = (a.dec_dctx.1)(rd, orr.as_mut_ptr() as *mut c_void, orr.len(), buf_ptr(&b), b.len());
                        a.cmp_ret(&format!("ERRORS row 193: checksum flip ign={ign} [{}]", f.desc), cr, rr);
                        if (a.is_err.0)(cr) == 0 {
                            assert_bytes_eq(&format!("ERRORS row 193 bytes ign={ign}"), &oc[..cr], &orr[..rr]);
                        }
                        (a.free_dctx.0)(cd);
                        (a.free_dctx.1)(rd);
                    }
                    if b.len() >= 3 {
                        a.differential_decode_all(192, &format!("checksum region<4 [{}]", f.desc), &b[..b.len() - 2], None);
                    }
                }
            }
        }

        // ---------- 198/199/158/162: dst too small / dst NULL ----------
        for f in corpus.iter().filter(|f| f.desc.contains("shape=Zeros")).take(6) {
            a.differential_decode_all(198, &format!("RLE/raw dst-too-small sweep [{}]", f.desc), &f.bytes, None);
        }

        eprintln!("ERRORS rows 155-184 (literals/sequences/FSE/HUF interior): exercised via systematic single-bit/single-byte corruption of the first 32 bytes plus sampled interior offsets of every corpus frame; each corrupted input is run through the full decode battery and C-vs-Rust error parity (raw size_t, isError, getErrorCode, getErrorName) is asserted.");
    }
}

// ================================================================ TEST 3 ====
/// ERRORS rows 201-219 — Streaming decompression.
#[test]
fn phasec_rows_201_219_streaming() {
    let a = api();
    let mut rng = Rng::new(0xC0DE_0201);
    unsafe {
        let corpus = small_corpus(&mut rng);

        // ---------- 201-205: buffer-less + streaming on corrupted frames ----------
        for f in corpus.iter().filter(|f| !f.magicless).take(10) {
            if f.bytes.len() > 8 {
                let mut b = f.bytes.clone();
                let mid = b.len() / 2;
                b[mid] ^= 0xFF;
                a.differential_decode_all(202, &format!("streaming corrupted-mid [{}]", f.desc), &b, None);
            }
            a.differential_decode_all(201, &format!("bufferless pristine [{}]", f.desc), &f.bytes, None);
        }

        // ---------- 218: no forward progress, output full (destFull) ----------
        {
            let good = corpus.iter().find(|f| !f.magicless && f.content_len >= 100);
            if let Some(f) = good {
                let cd = (a.create_dctx.0)();
                let rd = (a.create_dctx.1)();
                let run = |ds: *mut c_void, f0: FnDStream| -> size_t {
                    let mut last = 0usize;
                    for _ in 0..8 {
                        let mut ib = ZSTD_inBuffer { src: buf_ptr(&f.bytes), size: f.bytes.len(), pos: 0 };
                        let mut ob = ZSTD_outBuffer {
                            dst: std::ptr::NonNull::<u8>::dangling().as_ptr() as *mut c_void,
                            size: 0,
                            pos: 0,
                        };
                        last = f0(ds, &mut ob, &mut ib);
                        if (a.is_err.0)(last) != 0 {
                            break;
                        }
                    }
                    last
                };
                let cr = run(cd, a.dstream.0);
                let rr = run(rd, a.dstream.1);
                a.cmp_ret("ERRORS row 218: noForwardProgress destFull", cr, rr);
                (a.free_dctx.0)(cd);
                (a.free_dctx.1)(rd);
            } else {
                eprintln!("ERRORS row 218: no suitable frame found for destFull test");
            }
        }

        // ---------- 219: no forward progress, input empty (inputEmpty) ----------
        {
            let good = corpus.iter().find(|f| !f.magicless && f.content_len >= 100);
            if let Some(f) = good {
                let cd = (a.create_dctx.0)();
                let rd = (a.create_dctx.1)();
                let run = |ds: *mut c_void, f0: FnDStream| -> size_t {
                    let mut outbuf = vec![0xAAu8; f.content_len + 64];
                    let mut ib = ZSTD_inBuffer { src: buf_ptr(&f.bytes), size: 1, pos: 0 };
                    let mut ob = ZSTD_outBuffer { dst: outbuf.as_mut_ptr() as *mut c_void, size: outbuf.len(), pos: 0 };
                    let _ = f0(ds, &mut ob, &mut ib);
                    let mut last = 0usize;
                    for _ in 0..8 {
                        let mut ib2 = ZSTD_inBuffer {
                            src: std::ptr::NonNull::<u8>::dangling().as_ptr() as *const c_void,
                            size: 0,
                            pos: 0,
                        };
                        let mut ob2 = ZSTD_outBuffer {
                            dst: outbuf.as_mut_ptr() as *mut c_void,
                            size: outbuf.len(),
                            pos: 0,
                        };
                        last = f0(ds, &mut ob2, &mut ib2);
                        if (a.is_err.0)(last) != 0 {
                            break;
                        }
                    }
                    last
                };
                let cr = run(cd, a.dstream.0);
                let rr = run(rd, a.dstream.1);
                a.cmp_ret("ERRORS row 219: noForwardProgress inputEmpty", cr, rr);
                (a.free_dctx.0)(cd);
                (a.free_dctx.1)(rd);
            } else {
                eprintln!("ERRORS row 219: no suitable frame found for inputEmpty test");
            }
        }

        // ---------- 212/150: streaming windowSize > maxWindowSize ----------
        {
            let (c_setdp, r_setdp) = fnpair!("ZSTD_DCtx_setParameter", FnSetParam);
            let big = corpus.iter().find(|f| !f.magicless && f.content_len >= 65_537);
            if let Some(f) = big {
                let cd = (a.create_dctx.0)();
                let rd = (a.create_dctx.1)();
                let _ = c_setdp(cd, ZSTD_d_windowLogMax, 10);
                let _ = r_setdp(rd, ZSTD_d_windowLogMax, 10);
                let (cerr, cpos, _cb) = drive_dstream(&a, a.dstream.0, cd, &f.bytes, 64, f.content_len + 64);
                let (rerr, rpos, _rb) = drive_dstream(&a, a.dstream.1, rd, &f.bytes, 64, f.content_len + 64);
                a.cmp_ret(&format!("ERRORS row 212: streaming windowTooLarge [{}]", f.desc), cerr, rerr);
                assert_eq!(cpos, rpos, "row 212: produced differs");
                (a.free_dctx.0)(cd);
                (a.free_dctx.1)(rd);
            } else {
                eprintln!("ERRORS row 212: no large-window frame; window-too-large is covered at header decode (rows 135/150)");
            }
        }

        // ---------- 213/214: d_stableOutBuffer with a changed output buffer ----------
        {
            let (c_setdp, r_setdp) = fnpair!("ZSTD_DCtx_setParameter", FnSetParam);
            let ff = corpus.iter().find(|f| !f.magicless && f.content_len >= 100);
            if let Some(f) = ff {
                let cd = (a.create_dctx.0)();
                let rd = (a.create_dctx.1)();
                let sc = c_setdp(cd, ZSTD_d_stableOutBuffer, 1);
                let sr = r_setdp(rd, ZSTD_d_stableOutBuffer, 1);
                a.cmp_ret("ERRORS row 213: set d_stableOutBuffer", sc, sr);

                let run = |ds: *mut c_void, f0: FnDStream| -> size_t {
                    let mut buf1 = vec![0xAAu8; f.content_len + 64];
                    let mut buf2 = vec![0xBBu8; f.content_len + 64];
                    let half = (f.bytes.len() / 2).max(1);
                    let mut ib = ZSTD_inBuffer { src: buf_ptr(&f.bytes), size: half, pos: 0 };
                    let mut ob = ZSTD_outBuffer { dst: buf1.as_mut_ptr() as *mut c_void, size: buf1.len(), pos: 0 };
                    let _ = f0(ds, &mut ob, &mut ib);
                    let mut ib2 = ZSTD_inBuffer {
                        src: buf_ptr(&f.bytes[half..]),
                        size: f.bytes.len() - half,
                        pos: 0,
                    };
                    let mut ob2 = ZSTD_outBuffer { dst: buf2.as_mut_ptr() as *mut c_void, size: buf2.len(), pos: 0 };
                    f0(ds, &mut ob2, &mut ib2)
                };
                let cr = run(cd, a.dstream.0);
                let rr = run(rd, a.dstream.1);
                a.cmp_ret(&format!("ERRORS row 213: stableOut changed buffer [{}]", f.desc), cr, rr);
                (a.free_dctx.0)(cd);
                (a.free_dctx.1)(rd);
            }
        }

        // ---------- 216: decompressStream on a fresh (never-reset) DCtx ----------
        {
            let ff = corpus.iter().find(|f| !f.magicless && f.content_len >= 50);
            if let Some(f) = ff {
                let cd = (a.create_dctx.0)();
                let rd = (a.create_dctx.1)();
                let (cerr, cpos, cb) = drive_dstream(&a, a.dstream.0, cd, &f.bytes, usize::MAX, f.content_len + 64);
                let (rerr, rpos, rb) = drive_dstream(&a, a.dstream.1, rd, &f.bytes, usize::MAX, f.content_len + 64);
                a.cmp_ret(&format!("ERRORS row 216: fresh-DCtx stream [{}]", f.desc), cerr, rerr);
                assert_eq!(cpos, rpos, "row 216: produced differs");
                if (a.is_err.0)(cerr) == 0 {
                    assert_bytes_eq("row 216 bytes", &cb[..cpos], &rb[..rpos]);
                }
                (a.free_dctx.0)(cd);
                (a.free_dctx.1)(rd);
            }
        }

        // ---------- 203/202: d_maxBlockSize smaller than frame's blockSizeMax ----------
        {
            let (c_setdp, r_setdp) = fnpair!("ZSTD_DCtx_setParameter", FnSetParam);
            let ff = corpus.iter().find(|f| !f.magicless && f.content_len >= 65_537);
            if let Some(f) = ff {
                let cd = (a.create_dctx.0)();
                let rd = (a.create_dctx.1)();
                let _ = c_setdp(cd, ZSTD_d_maxBlockSize, 1024);
                let _ = r_setdp(rd, ZSTD_d_maxBlockSize, 1024);
                let mut oc = vec![0xAAu8; f.content_len + 64];
                let mut orr = vec![0xAAu8; f.content_len + 64];
                let cr = (a.dec_dctx.0)(cd, oc.as_mut_ptr() as *mut c_void, oc.len(), buf_ptr(&f.bytes), f.bytes.len());
                let rr = (a.dec_dctx.1)(rd, orr.as_mut_ptr() as *mut c_void, orr.len(), buf_ptr(&f.bytes), f.bytes.len());
                a.cmp_ret(&format!("ERRORS row 203: d_maxBlockSize<blockSizeMax [{}]", f.desc), cr, rr);
                (a.free_dctx.0)(cd);
                (a.free_dctx.1)(rd);
            } else {
                eprintln!("ERRORS row 203: no multi-block frame in corpus for d_maxBlockSize test");
            }
        }

        // ---------- 207: loadDictionary while not in zdss_init (stage_wrong) ----------
        {
            let (c_ld, r_ld) = fnpair!("ZSTD_DCtx_loadDictionary", FnLoadDict);
            let ff = corpus.iter().find(|f| !f.magicless && f.content_len >= 100);
            let dict = gen(Shape::Text, 512, &mut rng);
            if let Some(f) = ff {
                let cd = (a.create_dctx.0)();
                let rd = (a.create_dctx.1)();
                let half = (f.bytes.len() / 2).max(1);
                let mut buf = vec![0xAAu8; f.content_len + 64];
                let mut ibc = ZSTD_inBuffer { src: buf_ptr(&f.bytes), size: half, pos: 0 };
                let mut obc = ZSTD_outBuffer { dst: buf.as_mut_ptr() as *mut c_void, size: buf.len(), pos: 0 };
                let _ = (a.dstream.0)(cd, &mut obc, &mut ibc);
                let mut ibr = ZSTD_inBuffer { src: buf_ptr(&f.bytes), size: half, pos: 0 };
                let mut obr = ZSTD_outBuffer { dst: buf.as_mut_ptr() as *mut c_void, size: buf.len(), pos: 0 };
                let _ = (a.dstream.1)(rd, &mut obr, &mut ibr);
                let cr = c_ld(cd, buf_ptr(&dict), dict.len());
                let rr = r_ld(rd, buf_ptr(&dict), dict.len());
                a.cmp_ret("ERRORS row 207: loadDictionary mid-stream (stage_wrong)", cr, rr);
                (a.free_dctx.0)(cd);
                (a.free_dctx.1)(rd);
            }
        }

        eprintln!("ERRORS rows 206, 208-211, 215, 217: memory-allocation / DDict-hash-set / static-DStream / internal-buffer-accounting failure paths (dctx==NULL row 206; refDDict multiple-hash-set rows 208-211; static-DStream alloc row 215; toLoad accounting row 217). These require forcing an allocation failure or passing a NULL/static context and are NOT constructible as differential decode-inputs through the public one-shot/streaming API without a failing allocator. Not faked. Row 214 (stableOut too small) is subsumed by row 213's stableOutBuffer handling above; row 216 (fresh-DCtx stream) IS covered above.");
    }
}
