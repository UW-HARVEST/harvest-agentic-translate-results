//! Phase B — CONFIGS.md rows 97..102: static (no-malloc) allocation.
//!
//! Every workspace is a 64-byte-aligned buffer whose size comes from the
//! matching `ZSTD_estimate*Size*()` call, plus the "one byte too small" and
//! "generous" variants; both libraries must agree on acceptance/rejection and
//! on all subsequent output.
mod common;
use common::*;
use std::ffi::{c_int, c_ulonglong, c_void};

type FnInitStatic = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
type FnCompress2 = unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize) -> usize;
type FnInitStaticCDict = unsafe extern "C" fn(
    *mut c_void,
    usize,
    *const c_void,
    usize,
    c_int,
    c_int,
    ZSTD_compressionParameters,
) -> *const c_void;
type FnInitStaticDDict =
    unsafe extern "C" fn(*mut c_void, usize, *const c_void, usize, c_int, c_int) -> *const c_void;

/// 64-byte aligned scratch buffer.
struct Ws {
    buf: Vec<u64>,
    len: usize,
}

impl Ws {
    fn new(len: usize) -> Ws {
        // u64 vec gives 8-byte alignment; zstd's static init only requires
        // sizeof(void*) alignment (it aligns internally), and it must behave
        // identically for both libraries given the same pointer alignment.
        let n = len / 8 + 2;
        Ws { buf: vec![0u64; n], len }
    }
    fn ptr(&mut self) -> *mut c_void {
        self.buf.as_mut_ptr() as *mut c_void
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.buf.as_ptr() as *const u8, self.len) }
    }
}

fn sizes_around(n: usize) -> Vec<usize> {
    let mut v = vec![n, n + 1, n + 64, n * 2];
    if n > 0 {
        v.push(n - 1);
        v.push(n / 2);
    }
    v.push(0);
    v.push(1);
    v
}

// ------------------------------------------------------------------ row 97

#[test]
fn row97_static_cctx() {
    unsafe {
        let (ic, ir) = duo::<FnInitStatic>("ZSTD_initStaticCCtx");
        let (est, _) = duo::<unsafe extern "C" fn(c_int) -> usize>("ZSTD_estimateCCtxSize");
        let (estp, _) = duo::<unsafe extern "C" fn(ZSTD_compressionParameters) -> usize>(
            "ZSTD_estimateCCtxSize_usingCParams",
        );
        let (gc, _) =
            duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_compressionParameters>(
                "ZSTD_getCParams",
            );
        let (cc, cr) = duo::<FnCompressCCtx>("ZSTD_compressCCtx");
        let (c2c, c2r) = duo::<FnCompress2>("ZSTD_compress2");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (szc, szr) = duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_sizeof_CCtx");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let (dc, dr) = duo::<FnDecompress>("ZSTD_decompress");

        let mut rng = Rng::new(97);
        for lvl in [1, 3, 6, 12, 19, 22] {
            let need = est(lvl);
            for ws_len in sizes_around(need) {
                let mut wc = Ws::new(ws_len);
                let mut wr = Ws::new(ws_len);
                let pc = ic(wc.ptr(), ws_len);
                let pr = ir(wr.ptr(), ws_len);
                eqv(
                    &format!("row97 lvl={lvl} ws={ws_len} initStaticCCtx null?"),
                    pc.is_null(),
                    pr.is_null(),
                );
                if pc.is_null() {
                    continue;
                }
                eqv(
                    &format!("row97 lvl={lvl} ws={ws_len} sizeof_CCtx"),
                    szc(pc),
                    szr(pr),
                );
                let sz = rng.below(50_000);
                let src = gen_class(rng.below(N_CLASSES), sz, lvl as u64);
                let cap = bd(sz);
                let mut oc = vec![0u8; cap];
                let mut or_ = vec![0u8; cap];
                let a = cc(
                    pc,
                    oc.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                    lvl,
                );
                let b = cr(
                    pr,
                    or_.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                    lvl,
                );
                let w = format!("row97 lvl={lvl} ws={ws_len} sz={sz}");
                eqv(&format!("{w} compressCCtx"), a, b);
                eqbuf(&format!("{w} dst"), &oc, &or_);
                if !is_err(a) {
                    let mut p1 = vec![0u8; sz + 8];
                    let mut p2 = vec![0u8; sz + 8];
                    let x = dc(
                        p1.as_mut_ptr() as *mut c_void,
                        p1.len(),
                        oc.as_ptr() as *const c_void,
                        a,
                    );
                    let y = dr(
                        p2.as_mut_ptr() as *mut c_void,
                        p2.len(),
                        or_.as_ptr() as *const c_void,
                        b,
                    );
                    eqv(&format!("{w} roundtrip"), x, y);
                    eqbuf(&format!("{w} roundtrip dst"), &p1, &p2);
                }
                // the advanced API on a static CCtx: parameters that need more
                // memory must be rejected identically
                for (p, v) in [
                    (ZSTD_c_compressionLevel, 22),
                    (ZSTD_c_windowLog, 27),
                    (ZSTD_c_strategy, 9),
                    (ZSTD_c_enableLongDistanceMatching, 1),
                    (ZSTD_c_ldmMinMatch, 64),
                    (ZSTD_c_nbWorkers, 1),
                ] {
                    eqv(
                        &format!("{w} setParameter({p},{v})"),
                        spc(pc, p, v),
                        spr(pr, p, v),
                    );
                }
                let mut oc2 = vec![0u8; cap];
                let mut or2 = vec![0u8; cap];
                let a = c2c(
                    pc,
                    oc2.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                );
                let b = c2r(
                    pr,
                    or2.as_mut_ptr() as *mut c_void,
                    cap,
                    src.as_ptr() as *const c_void,
                    sz,
                );
                eqv(&format!("{w} compress2 after params"), a, b);
                eqbuf(&format!("{w} compress2 dst"), &oc2, &or2);
            }
        }

        // ZSTD_estimateCCtxSize_usingCParams-sized workspaces over a cParams grid
        for lvl in [1, 5, 11, 19] {
            let cp = gc(lvl, 0, 0);
            let need = estp(cp);
            for ws_len in [need, need - 1, need + 128] {
                let mut wc = Ws::new(ws_len);
                let mut wr = Ws::new(ws_len);
                let pc = ic(wc.ptr(), ws_len);
                let pr = ir(wr.ptr(), ws_len);
                eqv(
                    &format!("row97b lvl={lvl} ws={ws_len} null?"),
                    pc.is_null(),
                    pr.is_null(),
                );
            }
        }
    }
}

// ------------------------------------------------------------------ row 98

#[test]
fn row98_static_cstream() {
    unsafe {
        let (ic, ir) = duo::<FnInitStatic>("ZSTD_initStaticCStream");
        let (est, _) = duo::<unsafe extern "C" fn(c_int) -> usize>("ZSTD_estimateCStreamSize");
        let (s2c, s2r) = duo::<FnStream2>("ZSTD_compressStream2");
        let (spc, spr) = duo::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (szc, szr) = duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_sizeof_CStream");
        let (osz, _) = duo::<FnSizeT0>("ZSTD_CStreamOutSize");

        for lvl in [1, 3, 9, 19] {
            let need = est(lvl);
            for ws_len in [need, need - 1, need + 1024] {
                let mut wc = Ws::new(ws_len);
                let mut wr = Ws::new(ws_len);
                let pc = ic(wc.ptr(), ws_len);
                let pr = ir(wr.ptr(), ws_len);
                let w = format!("row98 lvl={lvl} ws={ws_len}");
                eqv(&format!("{w} null?"), pc.is_null(), pr.is_null());
                if pc.is_null() {
                    continue;
                }
                eqv(&format!("{w} sizeof_CStream"), szc(pc), szr(pr));
                eqv(
                    &format!("{w} set level"),
                    spc(pc, ZSTD_c_compressionLevel, lvl),
                    spr(pr, ZSTD_c_compressionLevel, lvl),
                );
                let src = gen_class(4, 90_000, lvl as u64);
                let ocap = osz();
                let mut outc = vec![0u8; ocap];
                let mut outr = vec![0u8; ocap];
                let mut framec: Vec<u8> = Vec::new();
                let mut framer: Vec<u8> = Vec::new();
                let mut posc = 0usize;
                let mut posr = 0usize;
                let mut step = 0;
                loop {
                    step += 1;
                    let end = (posc + 9000).min(src.len());
                    let mut ibc = ZSTD_inBuffer {
                        src: src.as_ptr() as *const c_void,
                        size: end,
                        pos: posc,
                    };
                    let mut ibr = ZSTD_inBuffer {
                        src: src.as_ptr() as *const c_void,
                        size: end,
                        pos: posr,
                    };
                    let mut obc = ZSTD_outBuffer {
                        dst: outc.as_mut_ptr() as *mut c_void,
                        size: ocap,
                        pos: 0,
                    };
                    let mut obr = ZSTD_outBuffer {
                        dst: outr.as_mut_ptr() as *mut c_void,
                        size: ocap,
                        pos: 0,
                    };
                    let op = if end == src.len() { ZSTD_e_end } else { ZSTD_e_continue };
                    let a = s2c(pc, &mut obc, &mut ibc, op);
                    let b = s2r(pr, &mut obr, &mut ibr, op);
                    eqv(&format!("{w} step={step} compressStream2"), a, b);
                    eqv(&format!("{w} step={step} in.pos"), ibc.pos, ibr.pos);
                    eqv(&format!("{w} step={step} out.pos"), obc.pos, obr.pos);
                    eqbuf(
                        &format!("{w} step={step} out"),
                        &outc[..obc.pos],
                        &outr[..obr.pos],
                    );
                    framec.extend_from_slice(&outc[..obc.pos]);
                    framer.extend_from_slice(&outr[..obr.pos]);
                    posc = ibc.pos;
                    posr = ibr.pos;
                    if is_err(a) || (op == ZSTD_e_end && a == 0) || step > 500 {
                        break;
                    }
                }
                eqbuf(&format!("{w} whole frame"), &framec, &framer);
            }
        }
    }
}

// ------------------------------------------------------------------ rows 99, 100

#[test]
fn row99_100_static_dctx_dstream() {
    unsafe {
        let (idc, idr) = duo::<FnInitStatic>("ZSTD_initStaticDCtx");
        let (isc, isr) = duo::<FnInitStatic>("ZSTD_initStaticDStream");
        let (edc, _) = duo::<FnSizeT0>("ZSTD_estimateDCtxSize");
        let (eds, _) = duo::<FnSizeT1>("ZSTD_estimateDStreamSize");
        let (dc, dr) = duo::<FnDecompressDCtx>("ZSTD_decompressDCtx");
        let (dsc, dsr) = duo::<FnDStream>("ZSTD_decompressStream");
        let (szc, szr) = duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_sizeof_DCtx");
        let (i2c, i2r) = duo::<unsafe extern "C" fn(*mut c_void) -> usize>("ZSTD_initDStream");

        let need = edc();
        let mut rng = Rng::new(99);
        for ws_len in sizes_around(need) {
            let mut wc = Ws::new(ws_len);
            let mut wr = Ws::new(ws_len);
            let pc = idc(wc.ptr(), ws_len);
            let pr = idr(wr.ptr(), ws_len);
            eqv(
                &format!("row99 ws={ws_len} initStaticDCtx null?"),
                pc.is_null(),
                pr.is_null(),
            );
            if pc.is_null() {
                continue;
            }
            eqv(&format!("row99 ws={ws_len} sizeof_DCtx"), szc(pc), szr(pr));
            for i in 0..8 {
                let sz = rng.below(60_000);
                let src = gen_class(rng.below(N_CLASSES), sz, i);
                let frame = c_compress(&src, rng.range(-3, 19));
                let mut p1 = vec![0u8; sz + 8];
                let mut p2 = vec![0u8; sz + 8];
                let x = dc(
                    pc,
                    p1.as_mut_ptr() as *mut c_void,
                    p1.len(),
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                );
                let y = dr(
                    pr,
                    p2.as_mut_ptr() as *mut c_void,
                    p2.len(),
                    frame.as_ptr() as *const c_void,
                    frame.len(),
                );
                eqv(&format!("row99 ws={ws_len} i={i} decompressDCtx"), x, y);
                eqbuf(&format!("row99 ws={ws_len} i={i} dst"), &p1, &p2);
            }
        }

        // static DStream over a windowSize grid
        for wsz in [1usize << 10, 1 << 17, 1 << 20, 1 << 22] {
            let need = eds(wsz);
            for ws_len in [need, need - 1, need + 512] {
                let mut wc = Ws::new(ws_len);
                let mut wr = Ws::new(ws_len);
                let pc = isc(wc.ptr(), ws_len);
                let pr = isr(wr.ptr(), ws_len);
                let w = format!("row100 wsz={wsz} ws={ws_len}");
                eqv(&format!("{w} null?"), pc.is_null(), pr.is_null());
                if pc.is_null() {
                    continue;
                }
                eqv(&format!("{w} initDStream"), i2c(pc), i2r(pr));
                let sz = 40_000usize;
                let src = gen_class(4, sz, wsz as u64);
                let frame = c_compress(&src, 5);
                let mut p1 = vec![0u8; sz + 16];
                let mut p2 = vec![0u8; sz + 16];
                let mut ibc = ZSTD_inBuffer {
                    src: frame.as_ptr() as *const c_void,
                    size: frame.len(),
                    pos: 0,
                };
                let mut ibr = ibc;
                let mut obc = ZSTD_outBuffer {
                    dst: p1.as_mut_ptr() as *mut c_void,
                    size: p1.len(),
                    pos: 0,
                };
                let mut obr = ZSTD_outBuffer {
                    dst: p2.as_mut_ptr() as *mut c_void,
                    size: p2.len(),
                    pos: 0,
                };
                let mut step = 0;
                loop {
                    step += 1;
                    let a = dsc(pc, &mut obc, &mut ibc);
                    let b = dsr(pr, &mut obr, &mut ibr);
                    eqv(&format!("{w} step={step} decompressStream"), a, b);
                    eqv(&format!("{w} step={step} in.pos"), ibc.pos, ibr.pos);
                    eqv(&format!("{w} step={step} out.pos"), obc.pos, obr.pos);
                    if is_err(a) || a == 0 || step > 300 {
                        break;
                    }
                }
                eqbuf(&format!("{w} dst"), &p1, &p2);
            }
        }
    }
}

// ------------------------------------------------------------------ rows 101, 102

#[test]
fn row101_102_static_cdict_ddict() {
    unsafe {
        let (icc, icr) = duo::<FnInitStaticCDict>("ZSTD_initStaticCDict");
        let (idc_, idr_) = duo::<FnInitStaticDDict>("ZSTD_initStaticDDict");
        let (ecd, _) = duo::<unsafe extern "C" fn(usize, ZSTD_compressionParameters, c_int) -> usize>(
            "ZSTD_estimateCDictSize_advanced",
        );
        let (edd, _) = duo::<unsafe extern "C" fn(usize, c_int) -> usize>("ZSTD_estimateDDictSize");
        let (gc, _) =
            duo::<unsafe extern "C" fn(c_int, c_ulonglong, usize) -> ZSTD_compressionParameters>(
                "ZSTD_getCParams",
            );
        let (szcd_c, szcd_r) =
            duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_sizeof_CDict");
        let (szdd_c, szdd_r) =
            duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_sizeof_DDict");
        let (idc2, idr2) =
            duo::<unsafe extern "C" fn(*const c_void) -> c_uint32>("ZSTD_getDictID_fromCDict");
        let (iddc, iddr) =
            duo::<unsafe extern "C" fn(*const c_void) -> c_uint32>("ZSTD_getDictID_fromDDict");
        let (dcc, dcr) = duo::<unsafe extern "C" fn(*const c_void) -> *const c_void>(
            "ZSTD_DDict_dictContent",
        );
        let (dsc_, dsr_) =
            duo::<unsafe extern "C" fn(*const c_void) -> usize>("ZSTD_DDict_dictSize");
        let (cuc, cur) = duo::<
            unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize, *const c_void) -> usize,
        >("ZSTD_compress_usingCDict");
        let (duc, dur) = duo::<
            unsafe extern "C" fn(*mut c_void, *mut c_void, usize, *const c_void, usize, *const c_void) -> usize,
        >("ZSTD_decompress_usingDDict");
        let (bd, _) = duo::<FnSizeT1>("ZSTD_compressBound");
        let cctx = CtxPair::cctx();
        let dctx = CtxPair::dctx();

        let mut rng = Rng::new(101);
        for &dsz in &[0usize, 1, 7, 256, 4096, 32 * 1024] {
            let dict = gen_class(rng.below(N_CLASSES), dsz, dsz as u64);
            let dp = if dsz == 0 {
                std::ptr::null()
            } else {
                dict.as_ptr() as *const c_void
            };
            for lvl in [1, 6, 19] {
                let cp = gc(lvl, 0, dsz);
                for dlm in [ZSTD_dlm_byCopy, ZSTD_dlm_byRef] {
                    for dct in [ZSTD_dct_auto, ZSTD_dct_rawContent, ZSTD_dct_fullDict] {
                        let need = ecd(dsz, cp, dlm);
                        for ws_len in [need, need.saturating_sub(1), need + 256] {
                            let mut wc = Ws::new(ws_len);
                            let mut wr = Ws::new(ws_len);
                            let pc = icc(wc.ptr(), ws_len, dp, dsz, dlm, dct, cp);
                            let pr = icr(wr.ptr(), ws_len, dp, dsz, dlm, dct, cp);
                            let w = format!(
                                "row101 dsz={dsz} lvl={lvl} dlm={dlm} dct={dct} ws={ws_len}"
                            );
                            eqv(&format!("{w} null?"), pc.is_null(), pr.is_null());
                            if pc.is_null() {
                                continue;
                            }
                            eqv(&format!("{w} sizeof_CDict"), szcd_c(pc), szcd_r(pr));
                            eqv(&format!("{w} getDictID_fromCDict"), idc2(pc), idr2(pr));
                            let sz = rng.below(30_000);
                            let src = gen_class(rng.below(N_CLASSES), sz, dsz as u64 ^ 7);
                            let cap = bd(sz);
                            let mut oc = vec![0u8; cap];
                            let mut or_ = vec![0u8; cap];
                            let a = cuc(
                                cctx.c,
                                oc.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                sz,
                                pc,
                            );
                            let b = cur(
                                cctx.r,
                                or_.as_mut_ptr() as *mut c_void,
                                cap,
                                src.as_ptr() as *const c_void,
                                sz,
                                pr,
                            );
                            eqv(&format!("{w} compress_usingCDict"), a, b);
                            eqbuf(&format!("{w} dst"), &oc, &or_);

                            // matching static DDict
                            let dneed = edd(dsz, dlm);
                            let mut wdc = Ws::new(dneed);
                            let mut wdr = Ws::new(dneed);
                            let qc = idc_(wdc.ptr(), dneed, dp, dsz, dlm, dct);
                            let qr = idr_(wdr.ptr(), dneed, dp, dsz, dlm, dct);
                            eqv(&format!("{w} DDict null?"), qc.is_null(), qr.is_null());
                            if qc.is_null() {
                                continue;
                            }
                            eqv(&format!("{w} sizeof_DDict"), szdd_c(qc), szdd_r(qr));
                            eqv(&format!("{w} getDictID_fromDDict"), iddc(qc), iddr(qr));
                            let nc = dsc_(qc);
                            let nr = dsr_(qr);
                            eqv(&format!("{w} DDict_dictSize"), nc, nr);
                            let cptr = dcc(qc);
                            let rptr = dcr(qr);
                            eqv(&format!("{w} DDict_dictContent null?"), cptr.is_null(), rptr.is_null());
                            if !cptr.is_null() && nc > 0 {
                                let cb = std::slice::from_raw_parts(cptr as *const u8, nc);
                                let rb = std::slice::from_raw_parts(rptr as *const u8, nr);
                                eqbuf(&format!("{w} DDict content"), cb, rb);
                            }
                            if is_err(a) {
                                continue;
                            }
                            let mut p1 = vec![0u8; sz + 8];
                            let mut p2 = vec![0u8; sz + 8];
                            let x = duc(
                                dctx.c,
                                p1.as_mut_ptr() as *mut c_void,
                                p1.len(),
                                oc.as_ptr() as *const c_void,
                                a,
                                qc,
                            );
                            let y = dur(
                                dctx.r,
                                p2.as_mut_ptr() as *mut c_void,
                                p2.len(),
                                or_.as_ptr() as *const c_void,
                                b,
                                qr,
                            );
                            eqv(&format!("{w} decompress_usingDDict"), x, y);
                            eqbuf(&format!("{w} decompress dst"), &p1, &p2);
                        }
                        // out-of-range dictContentType / dictLoadMethod enum
                        // values crossing the FFI boundary
                        let need = ecd(dsz, cp, dlm);
                        let mut wc = Ws::new(need);
                        let mut wr = Ws::new(need);
                        for bad in [-1i32, 3, 99] {
                            let pc = icc(wc.ptr(), need, dp, dsz, dlm, bad, cp);
                            let pr = icr(wr.ptr(), need, dp, dsz, dlm, bad, cp);
                            eqv(
                                &format!("row101 bad dct={bad} dsz={dsz} null?"),
                                pc.is_null(),
                                pr.is_null(),
                            );
                            let qc = idc_(wc.ptr(), need, dp, dsz, bad, dct);
                            let qr = idr_(wr.ptr(), need, dp, dsz, bad, dct);
                            eqv(
                                &format!("row102 bad dlm={bad} dsz={dsz} null?"),
                                qc.is_null(),
                                qr.is_null(),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[allow(non_camel_case_types)]
type c_uint32 = u32;
