//! Phase D gap-closing: exported entry points that are reachable across the FFI
//! boundary with public/opaque types but were not covered by any other test
//! file.
//!
//! Everything here was identified mechanically by `tools/coverage.sh`, which
//! records every symbol the suite actually `dlsym`s at runtime and diffs it
//! against `nm -D`. The symbols remaining after this file are documented in
//! `SYMBOLS.md`: they take *private* struct types by pointer
//! (`ZSTD_MatchState_t*`, `seqStore_t*`, `ZSTD_entropyCTables_t*`, ...) that have
//! no stable public layout, so they cannot be called directly by an external
//! consumer at all — they are exercised indirectly through the public API.

mod common;
use common::*;

type CCtx = *mut std::ffi::c_void;
type DCtx = *mut std::ffi::c_void;
type CDict = *mut std::ffi::c_void;
type DDict = *mut std::ffi::c_void;
type CCtxParams = *mut std::ffi::c_void;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct CParams {
    window_log: u32,
    chain_log: u32,
    hash_log: u32,
    search_log: u32,
    min_match: u32,
    target_length: u32,
    strategy: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct FParams {
    content_size_flag: i32,
    checksum_flag: i32,
    no_dict_id_flag: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct Params {
    c: CParams,
    f: FParams,
}

type Fn_bound = unsafe extern "C" fn(usize) -> usize;
type Fn_errCode = unsafe extern "C" fn(usize) -> i32;
type Fn_isError = unsafe extern "C" fn(usize) -> u32;
type Fn_chunk = unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize;

/// `ZSTD_noCompressLiterals` / `ZSTD_compressRleLiteralsBlock` — the two literal
/// block writers with plain (dst, cap, src, size) signatures. Both emit a
/// literals-section header, so the exact bytes matter.
#[test]
fn literal_block_writers_match() {
    let i = impls();
    let (c_nc, r_nc) =
        i.pair::<unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_noCompressLiterals",
        );
    let (c_rle, r_rle) =
        i.pair::<unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_compressRleLiteralsBlock",
        );
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");

    let mut rng = Rng::new(0x1178_0001);

    // sizes spanning the 1/2/3-byte literals-header size classes
    // (the header width changes at 31 and 4095 bytes)
    let sizes = [
        0usize, 1, 2, 3, 4, 30, 31, 32, 33, 1000, 4094, 4095, 4096, 4097, 65535, 65536,
        131_072,
    ];

    for &n in &sizes {
        for &shape in &[Shape::Constant, Shape::Random, Shape::SkewedText] {
            let src = gen_shape(shape, n, &mut rng);
            // dst capacity sweep incl. exactly-enough and one-short
            for &cap in &[0usize, 1, 2, 3, 4, 5, n, n + 1, n + 3, n + 8, n + 16] {
                let mut a = vec![0xC3u8; cap.max(1)];
                let mut b = vec![0x3Cu8; cap.max(1)];
                let x = unsafe { c_nc(a.as_mut_ptr(), cap, src.as_ptr(), n) };
                let y = unsafe { r_nc(b.as_mut_ptr(), cap, src.as_ptr(), n) };
                let tag = format!("noCompressLiterals n={n} shape={shape:?} cap={cap}");
                assert_eq_dbg(&tag, x, y);
                unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(x), r_cd(y)) };
                if unsafe { c_isE(x) } == 0 {
                    assert_bytes_eq(&tag, &a[..x], &b[..y]);
                }

                // RLE variant: the C only `assert(dstCapacity >= 4)` and then
                // `(void)dstCapacity` (zstd_compress_literals.c:86) — with
                // asserts compiled out (DEBUGLEVEL==0) it writes up to 4 bytes
                // with NO runtime bound check, so `dstCapacity < 4` is undefined
                // behaviour IN THE C and segfaults both libraries identically.
                //
                // It also does `ostart[flSize] = *(const BYTE*)src;` with no
                // `srcSize` check, so `srcSize == 0` dereferences a zero-length
                // buffer — likewise UB in the C (an RLE literals block of zero
                // bytes is meaningless). Both preconditions were confirmed by
                // probing the C `.so` on its own: `(cap=4, srcSize=0)` faults.
                //
                // Only the in-contract domain (`dstCapacity >= 4 && srcSize >= 1`)
                // is compared.
                if cap < 4 || n < 1 {
                    continue;
                }
                let mut a = vec![0xC3u8; cap.max(1)];
                let mut b = vec![0x3Cu8; cap.max(1)];
                let x = unsafe { c_rle(a.as_mut_ptr(), cap, src.as_ptr(), n) };
                let y = unsafe { r_rle(b.as_mut_ptr(), cap, src.as_ptr(), n) };
                let tag = format!("compressRleLiteralsBlock n={n} shape={shape:?} cap={cap}");
                assert_eq_dbg(&tag, x, y);
                unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(x), r_cd(y)) };
                if unsafe { c_isE(x) } == 0 {
                    assert_bytes_eq(&tag, &a[..x], &b[..y]);
                }
            }
        }
    }
}

/// `ZSTD_compressBegin_advanced` / `ZSTD_compressBegin_usingCDict_deprecated`
/// plus the `_public` and `_deprecated` block/continue aliases.
#[test]
fn begin_advanced_and_public_aliases_match() {
    let i = impls();
    let (c_new, r_new) = i.pair::<unsafe extern "C" fn() -> CCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<unsafe extern "C" fn(CCtx) -> usize>("ZSTD_freeCCtx");
    let (c_ba, r_ba) = i.pair::<unsafe extern "C" fn(
        CCtx,
        *const u8,
        usize,
        Params,
        u64,
    ) -> usize>("ZSTD_compressBegin_advanced");
    let (c_bcd, r_bcd) = i.pair::<unsafe extern "C" fn(CCtx, CDict) -> usize>(
        "ZSTD_compressBegin_usingCDict_deprecated",
    );
    let (c_cp, r_cp) = i.pair::<Fn_chunk>("ZSTD_compressContinue_public");
    let (c_ep, r_ep) = i.pair::<Fn_chunk>("ZSTD_compressEnd_public");
    let (c_cbd, r_cbd) = i.pair::<Fn_chunk>("ZSTD_compressBlock_deprecated");
    let (c_dbd, r_dbd) = i.pair::<Fn_chunk>("ZSTD_decompressBlock_deprecated");
    let (c_inv, r_inv) = i.pair::<unsafe extern "C" fn(CCtx)>("ZSTD_invalidateRepCodes");
    let (c_tr, r_tr) = i.pair::<unsafe extern "C" fn(CCtx, usize)>("ZSTD_CCtx_trace");
    let (c_gp, _) = i.pair::<unsafe extern "C" fn(i32, u64, usize) -> Params>("ZSTD_getParams");
    let (c_ccd, r_ccd) =
        i.pair::<unsafe extern "C" fn(*const u8, usize, i32) -> CDict>("ZSTD_createCDict");
    let (c_fcd, r_fcd) = i.pair::<unsafe extern "C" fn(CDict) -> usize>("ZSTD_freeCDict");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");
    let (c_dnew, r_dnew) = i.pair::<unsafe extern "C" fn() -> DCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_freeDCtx");
    let (c_dbeg, r_dbeg) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_decompressBegin");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };
    let mut rng = Rng::new(0x8E61_0001);

    let dict = gen_shape(Shape::SkewedText, 8192, &mut rng);
    let ccdict = unsafe { c_ccd(dict.as_ptr(), dict.len(), 5) };
    let rcdict = unsafe { r_ccd(dict.as_ptr(), dict.len(), 5) };
    assert!(!ccdict.is_null() && !rcdict.is_null());

    for &len in &[0usize, 1, 700, 30_000, 140_000] {
        let src = gen_shape(Shape::Tabular, len, &mut rng);
        let cap = unsafe { c_bound(len) } + 4096;

        // ---- compressBegin_advanced with several parameter sets and dicts
        for &lvl in &[1i32, 3, 19] {
            for d in [&dict[..], &[][..]] {
                for pledged in [len as u64, ZSTD_CONTENTSIZE_UNKNOWN] {
                    let params = unsafe { c_gp(lvl, len as u64, d.len()) };
                    let (a, b) = unsafe {
                        (
                            c_ba(cc, d.as_ptr(), d.len(), params, pledged),
                            r_ba(rc, d.as_ptr(), d.len(), params, pledged),
                        )
                    };
                    let tag = format!(
                        "compressBegin_advanced lvl={lvl} dict={} pledged={pledged} len={len}",
                        d.len()
                    );
                    assert_eq_dbg(&tag, a, b);
                    unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
                    if unsafe { c_isE(a) } != 0 {
                        continue;
                    }

                    // drive the frame with the *_public* aliases
                    let mut cb = vec![0u8; cap];
                    let mut rb = vec![0u8; cap];
                    let half = len / 2;
                    let a1 = unsafe { c_cp(cc, cb.as_mut_ptr(), cap, src.as_ptr(), half) };
                    let b1 = unsafe { r_cp(rc, rb.as_mut_ptr(), cap, src.as_ptr(), half) };
                    assert_eq_dbg(&format!("{tag} / compressContinue_public"), a1, b1);
                    if unsafe { c_isE(a1) } != 0 {
                        continue;
                    }
                    let a2 = unsafe {
                        c_ep(
                            cc,
                            cb.as_mut_ptr().add(a1),
                            cap - a1,
                            src.as_ptr().add(half),
                            len - half,
                        )
                    };
                    let b2 = unsafe {
                        r_ep(
                            rc,
                            rb.as_mut_ptr().add(b1),
                            cap - b1,
                            src.as_ptr().add(half),
                            len - half,
                        )
                    };
                    assert_eq_dbg(&format!("{tag} / compressEnd_public"), a2, b2);
                    if unsafe { c_isE(a2) } == 0 {
                        assert_bytes_eq(
                            &format!("{tag} / frame"),
                            &cb[..a1 + a2],
                            &rb[..b1 + b2],
                        );
                    }
                }
            }
        }

        // ---- compressBegin_usingCDict_deprecated
        let (a, b) = unsafe { (c_bcd(cc, ccdict), r_bcd(rc, rcdict)) };
        assert_eq_dbg(&format!("compressBegin_usingCDict_deprecated len={len}"), a, b);
        if unsafe { c_isE(a) } == 0 {
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let x = unsafe { c_ep(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let y = unsafe { r_ep(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len) };
            assert_eq_dbg(&format!("usingCDict_deprecated end len={len}"), x, y);
            if unsafe { c_isE(x) } == 0 {
                assert_bytes_eq(
                    &format!("usingCDict_deprecated frame len={len}"),
                    &cb[..x],
                    &rb[..y],
                );
            }
        }

        // ---- ZSTD_invalidateRepCodes + ZSTD_CCtx_trace (void, must not diverge
        // in their observable effect on the next frame)
        for extra in [0usize, 1, 1000] {
            unsafe {
                let params = c_gp(3, len as u64, 0);
                c_ba(cc, std::ptr::null(), 0, params, ZSTD_CONTENTSIZE_UNKNOWN);
                r_ba(rc, std::ptr::null(), 0, params, ZSTD_CONTENTSIZE_UNKNOWN);
                c_inv(cc);
                r_inv(rc);
                c_tr(cc, extra);
                r_tr(rc, extra);
            }
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let x = unsafe { c_ep(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let y = unsafe { r_ep(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len) };
            assert_eq_dbg(&format!("after invalidateRepCodes/trace({extra}) len={len}"), x, y);
            if unsafe { c_isE(x) } == 0 {
                assert_bytes_eq(
                    &format!("after invalidateRepCodes/trace({extra}) frame"),
                    &cb[..x],
                    &rb[..y],
                );
            }
        }

        // ---- compressBlock_deprecated / decompressBlock_deprecated
        if len <= 131_072 {
            unsafe {
                let params = c_gp(3, len as u64, 0);
                c_ba(cc, std::ptr::null(), 0, params, ZSTD_CONTENTSIZE_UNKNOWN);
                r_ba(rc, std::ptr::null(), 0, params, ZSTD_CONTENTSIZE_UNKNOWN);
            }
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let x = unsafe { c_cbd(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let y = unsafe { r_cbd(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let tag = format!("compressBlock_deprecated len={len}");
            assert_eq_dbg(&tag, x, y);
            unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(x), r_cd(y)) };
            if unsafe { c_isE(x) } == 0 && x > 0 {
                assert_bytes_eq(&tag, &cb[..x], &rb[..y]);
                unsafe {
                    c_dbeg(cd);
                    r_dbeg(rd);
                }
                let mut d1 = vec![0u8; len.max(1) + 64];
                let mut d2 = vec![0u8; len.max(1) + 64];
                let p = unsafe { c_dbd(cd, d1.as_mut_ptr(), d1.len(), cb.as_ptr(), x) };
                let q = unsafe { r_dbd(rd, d2.as_mut_ptr(), d2.len(), rb.as_ptr(), y) };
                assert_eq_dbg(&format!("{tag} / decompressBlock_deprecated"), p, q);
                if unsafe { c_isE(p) } == 0 {
                    assert_bytes_eq(&format!("{tag} / block payload"), &d1[..p], &d2[..q]);
                }
            }
        }
    }

    unsafe {
        c_fcd(ccdict);
        r_fcd(rcdict);
        c_free(cc);
        r_free(rc);
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// The `ZSTD_CCtx_params`-taking estimators and `ZSTD_getCParamsFromCCtxParams`.
#[test]
fn cctxparams_estimators_match() {
    let i = impls();
    let (c_pnew, r_pnew) = i.pair::<unsafe extern "C" fn() -> CCtxParams>("ZSTD_createCCtxParams");
    let (c_pfree, r_pfree) =
        i.pair::<unsafe extern "C" fn(CCtxParams) -> usize>("ZSTD_freeCCtxParams");
    let (c_pset, r_pset) =
        i.pair::<unsafe extern "C" fn(CCtxParams, i32, i32) -> usize>("ZSTD_CCtxParams_setParameter");
    let (c_pinit, r_pinit) =
        i.pair::<unsafe extern "C" fn(CCtxParams, i32) -> usize>("ZSTD_CCtxParams_init");
    let (c_ec, r_ec) = i.pair::<unsafe extern "C" fn(CCtxParams) -> usize>(
        "ZSTD_estimateCCtxSize_usingCCtxParams",
    );
    let (c_es, r_es) = i.pair::<unsafe extern "C" fn(CCtxParams) -> usize>(
        "ZSTD_estimateCStreamSize_usingCCtxParams",
    );
    let (c_gcp, r_gcp) = i.pair::<unsafe extern "C" fn(CCtxParams, u64, usize, i32) -> CParams>(
        "ZSTD_getCParamsFromCCtxParams",
    );
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let cp = unsafe { c_pnew() };
    let rp = unsafe { r_pnew() };
    assert!(!cp.is_null() && !rp.is_null());

    // fresh (uninitialised) params object first — the C has a defined answer
    unsafe {
        assert_eq_dbg("estimateCCtxSize_usingCCtxParams(fresh)", c_ec(cp), r_ec(rp));
        assert_eq_dbg("estimateCStreamSize_usingCCtxParams(fresh)", c_es(cp), r_es(rp));
    }

    let param_sets: Vec<Vec<(i32, i32)>> = vec![
        vec![],
        vec![(ZSTD_c_compressionLevel, 1)],
        vec![(ZSTD_c_compressionLevel, 19)],
        vec![(ZSTD_c_compressionLevel, -10)],
        vec![(ZSTD_c_windowLog, 10)],
        vec![(ZSTD_c_windowLog, 27)],
        vec![(ZSTD_c_strategy, ZSTD_btultra2), (ZSTD_c_windowLog, 22)],
        vec![(ZSTD_c_strategy, ZSTD_fast), (ZSTD_c_hashLog, 24)],
        // NOTE: LDM must be FULLY specified here. `ZSTD_estimateCCtxSize_usingCCtxParams`
        // does not run `ZSTD_ldm_adjustParameters`, so it divides by
        // `ldmParams.hashRateLog`; with LDM enabled but `ZSTD_c_ldmHashRateLog`
        // left at its 0 default the C ITSELF takes SIGFPE (verified by probing
        // the C .so alone: enabling LDM with 0, 1, 2 or 3 of the four LDM
        // sub-params set all crash; setting all four returns cleanly).
        vec![
            (ZSTD_c_enableLongDistanceMatching, 1),
            (ZSTD_c_ldmHashLog, 20),
            (ZSTD_c_ldmBucketSizeLog, 3),
            (ZSTD_c_ldmMinMatch, 32),
            (ZSTD_c_ldmHashRateLog, 4),
        ],
        vec![(ZSTD_c_useRowMatchFinder, ZSTD_ps_enable), (ZSTD_c_strategy, ZSTD_lazy2)],
        vec![(ZSTD_c_maxBlockSize, 8192)],
        vec![(ZSTD_c_srcSizeHint, 1 << 20)],
        vec![(ZSTD_c_nbWorkers, 0)],
    ];

    for lvl_init in [1i32, 3, 19] {
        for ps in &param_sets {
            unsafe {
                assert_eq_dbg(
                    &format!("CCtxParams_init({lvl_init})"),
                    c_pinit(cp, lvl_init),
                    r_pinit(rp, lvl_init),
                );
                for &(id, v) in ps {
                    assert_eq_dbg(
                        &format!("CCtxParams_setParameter({id},{v})"),
                        c_pset(cp, id, v),
                        r_pset(rp, id, v),
                    );
                }
                let tag = format!("init={lvl_init} params={ps:?}");
                let (a, b) = (c_ec(cp), r_ec(rp));
                assert_eq_dbg(&format!("estimateCCtxSize_usingCCtxParams [{tag}]"), a, b);
                assert_eq_dbg(
                    &format!("estimateCCtxSize_usingCCtxParams [{tag}] code"),
                    c_cd(a),
                    r_cd(b),
                );
                let (x, y) = (c_es(cp), r_es(rp));
                assert_eq_dbg(&format!("estimateCStreamSize_usingCCtxParams [{tag}]"), x, y);

                // ZSTD_getCParamsFromCCtxParams over srcSizeHint / dictSize /
                // every ZSTD_CParamMode_e value, incl. out-of-range enum values.
                for srchint in [
                    0u64,
                    1,
                    1000,
                    1 << 20,
                    1u64 << 40,
                    ZSTD_CONTENTSIZE_UNKNOWN,
                ] {
                    for dsz in [0usize, 1, 4096, 1 << 20] {
                        for mode in [-1i32, 0, 1, 2, 3, 4, 99] {
                            let p = c_gcp(cp, srchint, dsz, mode);
                            let q = r_gcp(rp, srchint, dsz, mode);
                            assert_eq_dbg(
                                &format!(
                                    "getCParamsFromCCtxParams [{tag}] src={srchint} dict={dsz} mode={mode}"
                                ),
                                p,
                                q,
                            );
                        }
                    }
                }
            }
        }
    }

    unsafe {
        c_pfree(cp);
        r_pfree(rp);
    }
}

/// `ZSTD_copyDDictParameters` and `ZSTD_decodeSeqHeaders`.
#[test]
fn ddict_copy_and_seq_headers_match() {
    let i = impls();
    let (c_dnew, r_dnew) = i.pair::<unsafe extern "C" fn() -> DCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_freeDCtx");
    let (c_cdd, r_cdd) =
        i.pair::<unsafe extern "C" fn(*const u8, usize) -> DDict>("ZSTD_createDDict");
    let (c_fdd, r_fdd) = i.pair::<unsafe extern "C" fn(DDict) -> usize>("ZSTD_freeDDict");
    let (c_cpy, r_cpy) =
        i.pair::<unsafe extern "C" fn(DCtx, DDict)>("ZSTD_copyDDictParameters");
    let (c_gid, r_gid) = i.pair::<unsafe extern "C" fn(DCtx) -> u32>("ZSTD_getDictID_fromDDict")
        ;
    let (c_dsh, r_dsh) =
        i.pair::<unsafe extern "C" fn(DCtx, *mut i32, *const u8, usize) -> usize>(
            "ZSTD_decodeSeqHeaders",
        );
    let (c_dbeg, r_dbeg) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_decompressBegin");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };
    let mut rng = Rng::new(0xDDC0_0001);

    // ZSTD_copyDDictParameters over several dictionary shapes/sizes
    for dlen in [0usize, 1, 8, 1024, 65536] {
        let dict = gen_shape(Shape::SkewedText, dlen, &mut rng);
        let a = unsafe { c_cdd(dict.as_ptr(), dlen) };
        let b = unsafe { r_cdd(dict.as_ptr(), dlen) };
        assert_eq_dbg(
            &format!("createDDict({dlen}) null-ness"),
            a.is_null(),
            b.is_null(),
        );
        if a.is_null() || b.is_null() {
            continue;
        }
        unsafe {
            c_dbeg(cd);
            r_dbeg(rd);
            c_cpy(cd, a);
            r_cpy(rd, b);
            // dictID reported by the DDict must match after the copy
            assert_eq_dbg(&format!("getDictID_fromDDict({dlen})"), c_gid(a), r_gid(b));
            c_fdd(a);
            r_fdd(b);
        }
    }

    // ZSTD_decodeSeqHeaders on real sequence-section bytes plus fuzz.
    // The C reads nbSeq then the three symbol-compression modes; feed it both
    // structured and random input and compare the nbSeq write-back too.
    for _ in 0..30_000 {
        let n = rng.range(0, 24);
        let mut buf = vec![0u8; n.max(1)];
        for x in buf.iter_mut() {
            *x = rng.byte();
        }
        let mut nb1 = -12345i32;
        let mut nb2 = -12345i32;
        unsafe {
            c_dbeg(cd);
            r_dbeg(rd);
        }
        let a = unsafe { c_dsh(cd, &mut nb1, buf.as_ptr(), n) };
        let b = unsafe { r_dsh(rd, &mut nb2, buf.as_ptr(), n) };
        let tag = format!("decodeSeqHeaders n={n} buf={:02x?}", &buf[..n.min(8)]);
        assert_eq_dbg(&tag, a, b);
        assert_eq_dbg(&format!("{tag} nbSeq"), nb1, nb2);
        unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
    }

    // structured: legal nbSeq encodings (1-byte, 2-byte and 3-byte forms)
    // followed by a legal all-predefined symbol-mode byte.
    let mut cases: Vec<Vec<u8>> = Vec::new();
    for nb in [0u32, 1, 0x7F, 0x80, 0x81, 0xFF, 0x100, 0x7F00, 0xFFFF] {
        let mut v = Vec::new();
        if nb == 0 {
            v.push(0u8);
        } else if nb < 128 {
            v.push(nb as u8);
        } else if nb < 0x7F00 {
            v.push((((nb - 0x80) >> 8) + 0x80) as u8);
            v.push(((nb - 0x80) & 0xFF) as u8);
        } else {
            v.push(0xFF);
            v.push(((nb - 0x7F00) & 0xFF) as u8);
            v.push((((nb - 0x7F00) >> 8) & 0xFF) as u8);
        }
        // symbol compression modes byte: 4 x 2-bit fields
        for modes in 0u16..=255 {
            let mut w = v.clone();
            w.push(modes as u8);
            // some FSE/RLE modes need following bytes; append a few
            for _ in 0..8 {
                w.push(rng.byte());
            }
            cases.push(w);
        }
    }
    for buf in &cases {
        for take in [1usize, 2, 3, 4, 5, 8, buf.len()] {
            if take > buf.len() {
                continue;
            }
            let mut nb1 = 0i32;
            let mut nb2 = 0i32;
            unsafe {
                c_dbeg(cd);
                r_dbeg(rd);
            }
            let a = unsafe { c_dsh(cd, &mut nb1, buf.as_ptr(), take) };
            let b = unsafe { r_dsh(rd, &mut nb2, buf.as_ptr(), take) };
            let tag = format!("decodeSeqHeaders structured take={take} {:02x?}", &buf[..take.min(6)]);
            assert_eq_dbg(&tag, a, b);
            assert_eq_dbg(&format!("{tag} nbSeq"), nb1, nb2);
        }
    }

    unsafe {
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// `ZSTD_registerSequenceProducer` / `ZSTD_CCtxParams_registerSequenceProducer`
/// — the external sequence-producer hook.
///
/// A registered producer that always fails is the interesting differential case:
/// with `ZSTD_c_enableSeqProducerFallback` off, both libraries must return
/// `ZSTD_error_sequenceProducer_failed`; with it on, both must silently fall back
/// to the internal match finder and produce IDENTICAL frames.
#[test]
fn sequence_producer_hook_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<unsafe extern "C" fn() -> CCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<unsafe extern "C" fn(CCtx) -> usize>("ZSTD_freeCCtx");
    let (c_rst, r_rst) = i.pair::<unsafe extern "C" fn(CCtx, i32) -> usize>("ZSTD_CCtx_reset");
    let (c_set, r_set) =
        i.pair::<unsafe extern "C" fn(CCtx, i32, i32) -> usize>("ZSTD_CCtx_setParameter");
    let (c_reg, r_reg) = i.pair::<unsafe extern "C" fn(
        CCtx,
        *mut std::ffi::c_void,
        Option<SeqProdFn>,
    )>("ZSTD_registerSequenceProducer");
    let (c_preg, r_preg) = i.pair::<unsafe extern "C" fn(
        CCtxParams,
        *mut std::ffi::c_void,
        Option<SeqProdFn>,
    )>("ZSTD_CCtxParams_registerSequenceProducer");
    let (c_pnew, r_pnew) = i.pair::<unsafe extern "C" fn() -> CCtxParams>("ZSTD_createCCtxParams");
    let (c_pfree, r_pfree) =
        i.pair::<unsafe extern "C" fn(CCtxParams) -> usize>("ZSTD_freeCCtxParams");
    let (c_apply, r_apply) = i.pair::<unsafe extern "C" fn(CCtx, CCtxParams) -> usize>(
        "ZSTD_CCtx_setParametersUsingCCtxParams",
    );
    let (c_c2, r_c2) = i.pair::<Fn_chunk>("ZSTD_compress2");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_isE, _) = i.pair::<Fn_isError>("ZSTD_isError");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let mut rng = Rng::new(0x5E90_0001);

    for &fallback in &[0i32, 1] {
        for &len in &[0usize, 1, 5000, 200_000] {
            let src = gen_shape(Shape::SkewedText, len, &mut rng);
            let cap = unsafe { c_bound(len) } + 64;

            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
                // an always-failing producer, registered on both
                c_reg(cc, std::ptr::null_mut(), Some(failing_seq_producer));
                r_reg(rc, std::ptr::null_mut(), Some(failing_seq_producer));
                assert_eq_dbg(
                    "set enableSeqProducerFallback",
                    c_set(cc, ZSTD_c_enableSeqProducerFallback, fallback),
                    r_set(rc, ZSTD_c_enableSeqProducerFallback, fallback),
                );
            }

            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let b = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let tag = format!("seqProducer(failing) fallback={fallback} len={len}");
            assert_eq_dbg(&tag, a, b);
            unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
            if unsafe { c_isE(a) } == 0 {
                assert_bytes_eq(&tag, &cb[..a], &rb[..b]);
            }

            // unregister (NULL fn) must restore normal behaviour identically
            unsafe {
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
                c_reg(cc, std::ptr::null_mut(), None);
                r_reg(rc, std::ptr::null_mut(), None);
            }
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let b = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let tag = format!("seqProducer(unregistered) len={len}");
            assert_eq_dbg(&tag, a, b);
            if unsafe { c_isE(a) } == 0 {
                assert_bytes_eq(&tag, &cb[..a], &rb[..b]);
            }

            // and the same through the CCtxParams-level registration
            let cp = unsafe { c_pnew() };
            let rp = unsafe { r_pnew() };
            unsafe {
                c_preg(cp, std::ptr::null_mut(), Some(failing_seq_producer));
                r_preg(rp, std::ptr::null_mut(), Some(failing_seq_producer));
                c_rst(cc, ZSTD_reset_session_and_parameters);
                r_rst(rc, ZSTD_reset_session_and_parameters);
                assert_eq_dbg(
                    "apply cctxParams with seq producer",
                    c_apply(cc, cp),
                    r_apply(rc, rp),
                );
                c_set(cc, ZSTD_c_enableSeqProducerFallback, fallback);
                r_set(rc, ZSTD_c_enableSeqProducerFallback, fallback);
            }
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = unsafe { c_c2(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let b = unsafe { r_c2(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len) };
            let tag = format!("cctxParams seqProducer fallback={fallback} len={len}");
            assert_eq_dbg(&tag, a, b);
            unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
            if unsafe { c_isE(a) } == 0 {
                assert_bytes_eq(&tag, &cb[..a], &rb[..b]);
            }
            unsafe {
                c_pfree(cp);
                r_pfree(rp);
            }
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
    }
}

/// `ZSTD_sequenceProducer_F` — returns a size_t; any value that
/// `ZSTD_isError()` accepts signals failure.
type SeqProdFn = unsafe extern "C" fn(
    *mut std::ffi::c_void, // sequenceProducerState
    *mut ZSTD_Sequence,    // outSeqs
    usize,                 // outSeqsCapacity
    *const std::ffi::c_void, // src
    usize,                 // srcSize
    *const std::ffi::c_void, // dict
    usize,                 // dictSize
    i32,                   // compressionLevel
    usize,                 // windowSize
) -> usize;

/// Always reports failure, which is what drives the two interesting branches
/// (hard error vs. fallback to the internal match finder).
unsafe extern "C" fn failing_seq_producer(
    _state: *mut std::ffi::c_void,
    _out: *mut ZSTD_Sequence,
    _out_cap: usize,
    _src: *const std::ffi::c_void,
    _src_size: usize,
    _dict: *const std::ffi::c_void,
    _dict_size: usize,
    _level: i32,
    _window: usize,
) -> usize {
    // ZSTD_SEQUENCE_PRODUCER_ERROR == (size_t)(-1)
    usize::MAX
}

/// `ZSTD_selectBlockCompressor` returns a FUNCTION POINTER. The addresses
/// necessarily differ between the two libraries, so what is compared is the
/// *structure* of the selection: which (strategy, rowMatchFinder, dictMode)
/// combinations map to the same function as each other, and which map to NULL.
/// An identical equivalence-class partition proves the dispatch table agrees
/// without depending on absolute addresses.
#[test]
fn select_block_compressor_partition_matches() {
    let i = impls();
    let (c_sel, r_sel) = i.pair::<unsafe extern "C" fn(i32, i32, i32) -> *const ()>(
        "ZSTD_selectBlockCompressor",
    );

    let mut c_ids: Vec<(String, isize)> = Vec::new();
    let mut r_ids: Vec<(String, isize)> = Vec::new();
    let mut c_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut r_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();

    // The C body is a raw table lookup
    //   static const ZSTD_BlockCompressor_f blockCompressor[4][ZSTD_STRATEGY_MAX+1]
    // (zstd_compress.c:3071) indexed directly by `dictMode` and `strat`, guarded
    // only by an `assert()` that is compiled out in this build. Indices beyond
    // dictMode 3 / strategy 9 therefore read PAST the array — undefined behaviour
    // in the C, which is why the sweep stops at the real table bounds. (Strategy
    // 0 IS in range: the table has an explicit "default for 0" entry.)
    for strat in 0i32..=9 {
        for rmf in 0i32..=2 {
            for dm in 0i32..=3 {
                let key = format!("strat={strat} rmf={rmf} dictMode={dm}");
                let a = unsafe { c_sel(strat, rmf, dm) } as usize;
                let b = unsafe { r_sel(strat, rmf, dm) } as usize;
                // map each distinct address to a small dense id, per library
                let ca = if a == 0 {
                    -1
                } else {
                    let n = c_map.len();
                    *c_map.entry(a).or_insert(n) as isize
                };
                let rb = if b == 0 {
                    -1
                } else {
                    let n = r_map.len();
                    *r_map.entry(b).or_insert(n) as isize
                };
                c_ids.push((key.clone(), ca));
                r_ids.push((key, rb));
            }
        }
    }

    assert_eq_dbg(
        "ZSTD_selectBlockCompressor: number of distinct compressors",
        c_map.len(),
        r_map.len(),
    );
    for ((k1, v1), (k2, v2)) in c_ids.iter().zip(r_ids.iter()) {
        assert_eq!(k1, k2);
        assert!(
            v1 == v2,
            "ZSTD_selectBlockCompressor[{k1}]: C selects compressor class {v1}, \
             Rust selects {v2} (-1 == NULL). The dispatch table differs."
        );
    }
    // sanity: the sweep must actually find several distinct compressors,
    // otherwise the partition comparison would be trivially satisfied.
    assert!(
        c_map.len() >= 8,
        "expected many distinct block compressors, found {}",
        c_map.len()
    );
}
