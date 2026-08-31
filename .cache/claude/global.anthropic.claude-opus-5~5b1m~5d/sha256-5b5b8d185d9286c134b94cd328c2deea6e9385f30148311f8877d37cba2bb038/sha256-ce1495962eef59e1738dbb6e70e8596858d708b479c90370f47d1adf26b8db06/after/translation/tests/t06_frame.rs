//! Phase B/C: frame introspection, size estimation, static-workspace init,
//! context copying, skippable frames and the low-level raw BLOCK API.
//!
//! These are the entry points a real consumer uses to size buffers and to walk
//! a byte stream, so they must agree with the C on both valid and malformed
//! input. Everything is called through the `.so` exports of both libraries.

mod common;
use common::*;

type CCtx = *mut std::ffi::c_void;
type DCtx = *mut std::ffi::c_void;

type Fn_createCCtx = unsafe extern "C" fn() -> CCtx;
type Fn_freeCCtx = unsafe extern "C" fn(CCtx) -> usize;
type Fn_createDCtx = unsafe extern "C" fn() -> DCtx;
type Fn_freeDCtx = unsafe extern "C" fn(DCtx) -> usize;
type Fn_setParam = unsafe extern "C" fn(CCtx, i32, i32) -> usize;
type Fn_reset = unsafe extern "C" fn(CCtx, i32) -> usize;
type Fn_compress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, i32) -> usize;
type Fn_compressCCtx =
    unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize, i32) -> usize;
type Fn_decompressDCtx = unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize) -> usize;
type Fn_bound = unsafe extern "C" fn(usize) -> usize;
type Fn_srcOnly_sz = unsafe extern "C" fn(*const u8, usize) -> usize;
type Fn_srcOnly_u64 = unsafe extern "C" fn(*const u8, usize) -> u64;
type Fn_srcOnly_u32 = unsafe extern "C" fn(*const u8, usize) -> u32;
type Fn_isError = unsafe extern "C" fn(usize) -> u32;
type Fn_errCode = unsafe extern "C" fn(usize) -> i32;
type Fn_sizeof = unsafe extern "C" fn(*const std::ffi::c_void) -> usize;
type Fn_getFrameHeader = unsafe extern "C" fn(*mut ZSTD_frameHeader, *const u8, usize) -> usize;
type Fn_getFrameHeaderAdv =
    unsafe extern "C" fn(*mut ZSTD_frameHeader, *const u8, usize, i32) -> usize;

/// `ZSTD_compressionParameters` — { unsigned x6; ZSTD_strategy }
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

/// `ZSTD_frameParameters` — { int contentSizeFlag, checksumFlag, noDictIDFlag }
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

/// `ZSTD_frameProgression`
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
struct FrameProgression {
    ingested: u64,
    consumed: u64,
    produced: u64,
    flushed: u64,
    current_job_id: u32,
    nb_active_workers: u32,
}

fn make_frames(i: &Impls, rng: &mut Rng) -> Vec<(String, Vec<u8>)> {
    let (c_comp, _) = i.pair::<Fn_compress>("ZSTD_compress");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_new, _) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_set, _) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_rst, _) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_c2, _) =
        i.pair::<unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_compress2",
        );

    let mut out = Vec::new();
    // plain frames of various shapes/sizes/levels
    for &shape in &ALL_SHAPES {
        for &len in &[0usize, 1, 1000, 200_000] {
            for &lvl in &[1i32, 9, 19] {
                let src = gen_shape(shape, len, rng);
                let cap = unsafe { c_bound(len) };
                let mut f = vec![0u8; cap];
                let n = unsafe { c_comp(f.as_mut_ptr(), cap, src.as_ptr(), len, lvl) };
                f.truncate(n);
                out.push((format!("plain shape={shape:?} len={len} lvl={lvl}"), f));
            }
        }
    }
    // frames with option variations that change the HEADER specifically
    let cc = unsafe { c_new() };
    for opts in [
        vec![(ZSTD_c_checksumFlag, 1)],
        vec![(ZSTD_c_contentSizeFlag, 0)],
        vec![(ZSTD_c_checksumFlag, 1), (ZSTD_c_contentSizeFlag, 0)],
        vec![(ZSTD_c_windowLog, 10)],
        vec![(ZSTD_c_windowLog, 27)],
        vec![(ZSTD_c_format, ZSTD_f_zstd1_magicless)],
    ] {
        let len = 50_000;
        let src = gen_shape(Shape::SkewedText, len, rng);
        unsafe {
            c_rst(cc, ZSTD_reset_session_and_parameters);
            for &(id, v) in &opts {
                c_set(cc, id, v);
            }
        }
        let cap = unsafe { c_bound(len) };
        let mut f = vec![0u8; cap];
        let n = unsafe { c_c2(cc, f.as_mut_ptr(), cap, src.as_ptr(), len) };
        if n < cap {
            f.truncate(n);
            out.push((format!("opts={opts:?}"), f));
        }
    }
    unsafe {
        let (c_free, _) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
        c_free(cc);
    }
    out
}

/// Malformed / adversarial buffers that every frame-parsing entry point must
/// classify identically. Includes wrong magic, right magic + garbage, skippable
/// magic variants, legacy magics, truncations and oversized declared sizes.
fn malformed_buffers(rng: &mut Rng) -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<(String, Vec<u8>)> = Vec::new();
    v.push(("empty".into(), vec![]));
    for n in 1..=8 {
        v.push((format!("zeros{n}"), vec![0u8; n]));
        v.push((format!("ff{n}"), vec![0xFFu8; n]));
    }
    // correct zstd magic followed by garbage of many lengths
    for tail in [0usize, 1, 2, 3, 4, 5, 6, 8, 12, 20, 64] {
        let mut b = ZSTD_MAGICNUMBER.to_le_bytes().to_vec();
        for _ in 0..tail {
            b.push(rng.byte());
        }
        v.push((format!("magic+garbage{tail}"), b));
    }
    // every skippable magic variant, with and without a length field
    for var in 0u32..16 {
        let m = ZSTD_MAGIC_SKIPPABLE_START + var;
        let mut b = m.to_le_bytes().to_vec();
        v.push((format!("skippable_magic_only var={var}"), b.clone()));
        b.extend_from_slice(&16u32.to_le_bytes());
        v.push((format!("skippable_hdr var={var}"), b.clone()));
        b.extend_from_slice(&[0xABu8; 16]);
        v.push((format!("skippable_full var={var}"), b.clone()));
        // declared length far bigger than the buffer
        let mut b2 = m.to_le_bytes().to_vec();
        b2.extend_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        v.push((format!("skippable_huge_len var={var}"), b2));
    }
    // dictionary magic (not a frame)
    v.push((
        "dict_magic".into(),
        ZSTD_MAGIC_DICTIONARY.to_le_bytes().to_vec(),
    ));
    // legacy magics v01..v07 — zstd_legacy.h dispatches on these
    for m in [
        0xFD2FB51Eu32, 0xFD2FB522, 0xFD2FB523, 0xFD2FB524, 0xFD2FB525, 0xFD2FB526,
        0xFD2FB527,
    ] {
        let mut b = m.to_le_bytes().to_vec();
        for _ in 0..32 {
            b.push(rng.byte());
        }
        v.push((format!("legacy_magic {m:#x}"), b));
    }
    // pure random buffers
    for n in [4usize, 5, 9, 18, 100, 1000] {
        let mut b = vec![0u8; n];
        for x in b.iter_mut() {
            *x = rng.byte();
        }
        v.push((format!("random{n}"), b));
    }
    v
}

#[test]
fn frame_introspection_matches() {
    let i = impls();
    let mut rng = Rng::new(0x1234_5678);

    let (c_fh, r_fh) = i.pair::<Fn_getFrameHeader>("ZSTD_getFrameHeader");
    let (c_fha, r_fha) = i.pair::<Fn_getFrameHeaderAdv>("ZSTD_getFrameHeader_advanced");
    let (c_fhs, r_fhs) = i.pair::<Fn_srcOnly_sz>("ZSTD_frameHeaderSize");
    let (c_fcs, r_fcs) = i.pair::<Fn_srcOnly_u64>("ZSTD_getFrameContentSize");
    let (c_gds, r_gds) = i.pair::<Fn_srcOnly_u64>("ZSTD_getDecompressedSize");
    let (c_ffcs, r_ffcs) = i.pair::<Fn_srcOnly_sz>("ZSTD_findFrameCompressedSize");
    let (c_fds, r_fds) = i.pair::<Fn_srcOnly_u64>("ZSTD_findDecompressedSize");
    let (c_db, r_db) = i.pair::<Fn_srcOnly_u64>("ZSTD_decompressBound");
    let (c_isf, r_isf) = i.pair::<Fn_srcOnly_u32>("ZSTD_isFrame");
    let (c_issk, r_issk) = i.pair::<Fn_srcOnly_u32>("ZSTD_isSkippableFrame");
    let (c_dm, r_dm) = i.pair::<Fn_srcOnly_sz>("ZSTD_decompressionMargin");
    let (c_edsf, r_edsf) = i.pair::<Fn_srcOnly_sz>("ZSTD_estimateDStreamSize_fromFrame");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let mut cases = make_frames(i, &mut rng);
    cases.extend(malformed_buffers(&mut rng));

    // also every truncation of the first few valid frames
    let extra: Vec<(String, Vec<u8>)> = cases
        .iter()
        .take(6)
        .flat_map(|(n, f)| {
            (0..f.len().min(40))
                .map(|k| (format!("{n} trunc{k}"), f[..k].to_vec()))
                .collect::<Vec<_>>()
        })
        .collect();
    cases.extend(extra);

    for (name, buf) in &cases {
        let p = buf.as_ptr();
        let n = buf.len();

        // ZSTD_getFrameHeader — compare the FULL filled struct, not just the rc
        let mut h1 = ZSTD_frameHeader::default();
        let mut h2 = ZSTD_frameHeader::default();
        let (a, b) = unsafe { (c_fh(&mut h1, p, n), r_fh(&mut h2, p, n)) };
        assert_eq_dbg(&format!("[{name}] getFrameHeader rc"), a, b);
        assert_eq_dbg(&format!("[{name}] getFrameHeader struct"), h1, h2);
        unsafe { assert_eq_dbg(&format!("[{name}] getFrameHeader code"), c_cd(a), r_cd(b)) };

        // _advanced with both formats, plus out-of-range format enum values
        for fmt in [0i32, 1, 2, -1, 99] {
            let mut g1 = ZSTD_frameHeader::default();
            let mut g2 = ZSTD_frameHeader::default();
            let (x, y) = unsafe { (c_fha(&mut g1, p, n, fmt), r_fha(&mut g2, p, n, fmt)) };
            assert_eq_dbg(&format!("[{name}] getFrameHeader_advanced({fmt}) rc"), x, y);
            assert_eq_dbg(
                &format!("[{name}] getFrameHeader_advanced({fmt}) struct"),
                g1,
                g2,
            );
        }

        macro_rules! same_sz {
            ($f:ident, $g:ident, $label:literal) => {{
                let (x, y) = unsafe { ($f(p, n), $g(p, n)) };
                assert_eq_dbg(&format!("[{name}] {}", $label), x, y);
            }};
        }
        same_sz!(c_fhs, r_fhs, "frameHeaderSize");
        same_sz!(c_ffcs, r_ffcs, "findFrameCompressedSize");
        same_sz!(c_dm, r_dm, "decompressionMargin");
        same_sz!(c_edsf, r_edsf, "estimateDStreamSize_fromFrame");
        same_sz!(c_fcs, r_fcs, "getFrameContentSize");
        same_sz!(c_gds, r_gds, "getDecompressedSize");
        same_sz!(c_fds, r_fds, "findDecompressedSize");
        same_sz!(c_db, r_db, "decompressBound");
        same_sz!(c_isf, r_isf, "isFrame");
        same_sz!(c_issk, r_issk, "isSkippableFrame");
    }
}

/// `ZSTD_getCParams` / `ZSTD_getParams` / `ZSTD_adjustCParams` /
/// `ZSTD_checkCParams` return whole structs by value across the FFI boundary —
/// a classic place for layout/ABI mistakes.
#[test]
fn cparams_derivation_matches() {
    let i = impls();
    let (c_gc, r_gc) = i.pair::<unsafe extern "C" fn(i32, u64, usize) -> CParams>("ZSTD_getCParams");
    let (c_gp, r_gp) = i.pair::<unsafe extern "C" fn(i32, u64, usize) -> Params>("ZSTD_getParams");
    let (c_ad, r_ad) =
        i.pair::<unsafe extern "C" fn(CParams, u64, usize) -> CParams>("ZSTD_adjustCParams");
    let (c_ck, r_ck) = i.pair::<unsafe extern "C" fn(CParams) -> usize>("ZSTD_checkCParams");
    let (c_ec, r_ec) =
        i.pair::<unsafe extern "C" fn(CParams) -> usize>("ZSTD_estimateCCtxSize_usingCParams");
    let (c_es, r_es) =
        i.pair::<unsafe extern "C" fn(CParams) -> usize>("ZSTD_estimateCStreamSize_usingCParams");

    let levels = [
        i32::MIN, -131_072, -1000, -50, -1, 0, 1, 3, 9, 19, 22, 23, 100, i32::MAX,
    ];
    let srcsizes: [u64; 9] = [
        0,
        1,
        1000,
        1 << 20,
        1 << 30,
        1u64 << 40,
        u64::MAX / 2,
        ZSTD_CONTENTSIZE_UNKNOWN,
        ZSTD_CONTENTSIZE_ERROR,
    ];
    let dictsizes = [0usize, 1, 1024, 1 << 20];

    for &lvl in &levels {
        for &ss in &srcsizes {
            for &ds in &dictsizes {
                let (a, b) = unsafe { (c_gc(lvl, ss, ds), r_gc(lvl, ss, ds)) };
                assert_eq_dbg(&format!("ZSTD_getCParams({lvl},{ss},{ds})"), a, b);
                let (a2, b2) = unsafe { (c_gp(lvl, ss, ds), r_gp(lvl, ss, ds)) };
                assert_eq_dbg(&format!("ZSTD_getParams({lvl},{ss},{ds})"), a2, b2);

                // feed the derived cParams back through adjust/check/estimate
                for &(x, y) in &[(a, b)] {
                    let (p, q) = unsafe { (c_ad(x, ss, ds), r_ad(y, ss, ds)) };
                    assert_eq_dbg(&format!("adjustCParams({lvl},{ss},{ds})"), p, q);
                    unsafe {
                        assert_eq_dbg(&format!("checkCParams({lvl})"), c_ck(x), r_ck(y));
                        assert_eq_dbg(&format!("estimateCCtxSize_usingCParams"), c_ec(x), r_ec(y));
                        assert_eq_dbg(&format!("estimateCStreamSize_usingCParams"), c_es(x), r_es(y));
                    }
                }
            }
        }
    }

    // deliberately INVALID / out-of-range cParams structs — checkCParams must
    // reject them identically, and adjust/estimate must agree too.
    let mut rng = Rng::new(0xCAFE_9001);
    for _ in 0..3000 {
        let p = CParams {
            window_log: rng.range(0, 40) as u32,
            chain_log: rng.range(0, 40) as u32,
            hash_log: rng.range(0, 40) as u32,
            search_log: rng.range(0, 40) as u32,
            min_match: rng.range(0, 12) as u32,
            target_length: rng.range(0, 200_000) as u32,
            strategy: rng.range(0, 12) as i32,
        };
        unsafe {
            assert_eq_dbg(&format!("checkCParams({p:?})"), c_ck(p), r_ck(p));
            let (x, y) = (c_ad(p, 1 << 20, 0), r_ad(p, 1 << 20, 0));
            assert_eq_dbg(&format!("adjustCParams({p:?})"), x, y);
            assert_eq_dbg(&format!("estimateCCtxSize_usingCParams({p:?})"), c_ec(p), r_ec(p));
            assert_eq_dbg(&format!("estimateCStreamSize_usingCParams({p:?})"), c_es(p), r_es(p));
        }
    }
}

/// Every `ZSTD_estimate*` / `ZSTD_sizeof_*` / `ZSTD_decodingBufferSize_min`
/// entry point — these size buffers for callers, so a mismatch is a real bug.
#[test]
fn size_estimation_matches() {
    let i = impls();

    for name in [
        "ZSTD_estimateCCtxSize",
        "ZSTD_estimateCStreamSize",
        "ZSTD_estimateCDictSize",
    ] {
        let (c, r) = i.pair::<unsafe extern "C" fn(i32) -> usize>(name);
        // NOTE: the upper level bound stops at 200 on purpose. The C
        // `ZSTD_estimateCCtxSize`/`ZSTD_estimateCStreamSize` bodies are
        // `for (level = MIN(compressionLevel,1); level <= compressionLevel; level++)`
        // so a level of `i32::MAX` makes the C ITSELF spin ~2^31 times. That is the
        // C's own designed behaviour, not a divergence; probing it would only time
        // the test out in both libraries. Negative extremes are cheap (the loop body
        // runs once) so `i32::MIN` IS covered.
        for lvl in [i32::MIN, -131_072, -1000, -1, 0, 1, 3, 19, 22, 23, 200] {
            // ZSTD_estimateCDictSize takes (size_t, int) — skip here, done below
            if name == "ZSTD_estimateCDictSize" {
                continue;
            }
            unsafe { assert_eq_dbg(&format!("{name}({lvl})"), c(lvl), r(lvl)) };
        }
    }

    let (c, r) = i.pair::<unsafe extern "C" fn(usize, i32) -> usize>("ZSTD_estimateCDictSize");
    for ds in [0usize, 1, 1024, 1 << 20] {
        for lvl in [-1000i32, -1, 0, 1, 19, 22, 100] {
            unsafe {
                assert_eq_dbg(&format!("ZSTD_estimateCDictSize({ds},{lvl})"), c(ds, lvl), r(ds, lvl))
            };
        }
    }

    for name in ["ZSTD_estimateDCtxSize", "ZSTD_estimateDStreamSize"] {
        // DCtx size takes no args; DStreamSize takes a windowSize
        if name == "ZSTD_estimateDCtxSize" {
            let (c, r) = i.pair::<unsafe extern "C" fn() -> usize>(name);
            unsafe { assert_eq_dbg(name, c(), r()) };
        } else {
            let (c, r) = i.pair::<unsafe extern "C" fn(usize) -> usize>(name);
            for w in [0usize, 1, 1024, 1 << 20, 1 << 27, usize::MAX / 2] {
                unsafe { assert_eq_dbg(&format!("{name}({w})"), c(w), r(w)) };
            }
        }
    }

    let (c, r) = i.pair::<unsafe extern "C" fn(usize) -> usize>("ZSTD_estimateDDictSize");
    // ZSTD_estimateDDictSize(size_t dictSize, ZSTD_dictLoadMethod_e)
    let (c2, r2) = i.pair::<unsafe extern "C" fn(usize, i32) -> usize>("ZSTD_estimateDDictSize");
    for ds in [0usize, 1, 1024, 1 << 20] {
        for m in [0i32, 1, 2, -1] {
            unsafe {
                assert_eq_dbg(
                    &format!("ZSTD_estimateDDictSize({ds},{m})"),
                    c2(ds, m),
                    r2(ds, m),
                )
            };
        }
    }
    let _ = (c, r);

    let (c, r) = i.pair::<unsafe extern "C" fn(u64, u64) -> usize>("ZSTD_decodingBufferSize_min");
    for w in [0u64, 1, 1024, 1 << 20, 1 << 27, 1u64 << 40] {
        for fcs in [
            0u64,
            1,
            1000,
            1 << 20,
            1u64 << 40,
            ZSTD_CONTENTSIZE_UNKNOWN,
            ZSTD_CONTENTSIZE_ERROR,
        ] {
            unsafe {
                assert_eq_dbg(
                    &format!("ZSTD_decodingBufferSize_min({w},{fcs})"),
                    c(w, fcs),
                    r(w, fcs),
                )
            };
        }
    }
}

/// `ZSTD_sizeof_*` on live contexts, and `ZSTD_getFrameProgression` /
/// `ZSTD_toFlushNow` mid-stream.
#[test]
fn sizeof_and_progression_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_dnew, r_dnew) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_szc, r_szc) = i.pair::<Fn_sizeof>("ZSTD_sizeof_CCtx");
    let (c_szd, r_szd) = i.pair::<Fn_sizeof>("ZSTD_sizeof_DCtx");
    let (c_szcs, r_szcs) = i.pair::<Fn_sizeof>("ZSTD_sizeof_CStream");
    let (c_szds, r_szds) = i.pair::<Fn_sizeof>("ZSTD_sizeof_DStream");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_cc, r_cc) = i.pair::<Fn_compressCCtx>("ZSTD_compressCCtx");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");
    let (c_fp, r_fp) =
        i.pair::<unsafe extern "C" fn(CCtx) -> FrameProgression>("ZSTD_getFrameProgression");
    let (c_tf, r_tf) = i.pair::<unsafe extern "C" fn(CCtx) -> usize>("ZSTD_toFlushNow");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };

    // sizes on a fresh context
    unsafe {
        assert_eq_dbg("sizeof_CCtx fresh", c_szc(cc), r_szc(rc));
        assert_eq_dbg("sizeof_DCtx fresh", c_szd(cd), r_szd(rd));
        assert_eq_dbg("sizeof_CStream fresh", c_szcs(cc), r_szcs(rc));
        assert_eq_dbg("sizeof_DStream fresh", c_szds(cd), r_szds(rd));
        assert_eq_dbg("frameProgression fresh", c_fp(cc), r_fp(rc));
        assert_eq_dbg("toFlushNow fresh", c_tf(cc), r_tf(rc));
    }

    // sizes after real work at several levels — allocation decisions must match
    let mut rng = Rng::new(0x5123_0F00);
    for &lvl in &[-5i32, 1, 3, 9, 19, 22] {
        for &len in &[100usize, 50_000, 300_000] {
            let src = gen_shape(Shape::SkewedText, len, &mut rng);
            let cap = unsafe { c_bound(len) };
            let mut cb = vec![0u8; cap];
            let mut rb = vec![0u8; cap];
            let a = unsafe { c_cc(cc, cb.as_mut_ptr(), cap, src.as_ptr(), len, lvl) };
            let b = unsafe { r_cc(rc, rb.as_mut_ptr(), cap, src.as_ptr(), len, lvl) };
            assert_eq_dbg(&format!("compressCCtx lvl={lvl} len={len}"), a, b);
            assert_bytes_eq(&format!("compressCCtx lvl={lvl} len={len}"), &cb[..a], &rb[..b]);
            unsafe {
                assert_eq_dbg(
                    &format!("sizeof_CCtx after lvl={lvl} len={len}"),
                    c_szc(cc),
                    r_szc(rc),
                );
                assert_eq_dbg(
                    &format!("frameProgression after lvl={lvl} len={len}"),
                    c_fp(cc),
                    r_fp(rc),
                );
                assert_eq_dbg(&format!("toFlushNow lvl={lvl}"), c_tf(cc), r_tf(rc));
            }

            // and the decoder side
            let mut d1 = vec![0u8; len + 8];
            let mut d2 = vec![0u8; len + 8];
            let (cdec, rdec) = i.pair::<Fn_decompressDCtx>("ZSTD_decompressDCtx");
            let x = unsafe { cdec(cd, d1.as_mut_ptr(), d1.len(), cb.as_ptr(), a) };
            let y = unsafe { rdec(rd, d2.as_mut_ptr(), d2.len(), rb.as_ptr(), b) };
            assert_eq_dbg("decompressDCtx", x, y);
            unsafe {
                assert_eq_dbg("sizeof_DCtx after", c_szd(cd), r_szd(rd));
            }
        }
    }

    // NULL context is explicitly allowed by the C (returns 0)
    unsafe {
        assert_eq_dbg(
            "sizeof_CCtx(NULL)",
            c_szc(std::ptr::null()),
            r_szc(std::ptr::null()),
        );
        assert_eq_dbg(
            "sizeof_DCtx(NULL)",
            c_szd(std::ptr::null()),
            r_szd(std::ptr::null()),
        );
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// Static (caller-provided workspace) initialisation: `ZSTD_initStaticCCtx` /
/// `initStaticDCtx` / `initStaticCStream` / `initStaticDStream`, including
/// workspaces that are too small (must return NULL in both).
#[test]
fn static_init_matches() {
    let i = impls();
    let (c_isc, r_isc) =
        i.pair::<unsafe extern "C" fn(*mut u8, usize) -> CCtx>("ZSTD_initStaticCCtx");
    let (c_isd, r_isd) =
        i.pair::<unsafe extern "C" fn(*mut u8, usize) -> DCtx>("ZSTD_initStaticDCtx");
    let (c_iscs, r_iscs) =
        i.pair::<unsafe extern "C" fn(*mut u8, usize) -> CCtx>("ZSTD_initStaticCStream");
    let (c_isds, r_isds) =
        i.pair::<unsafe extern "C" fn(*mut u8, usize) -> DCtx>("ZSTD_initStaticDStream");
    let (c_ec, _) = i.pair::<unsafe extern "C" fn(i32) -> usize>("ZSTD_estimateCCtxSize");
    let (c_ed, _) = i.pair::<unsafe extern "C" fn() -> usize>("ZSTD_estimateDCtxSize");
    let (c_cc, r_cc) = i.pair::<Fn_compressCCtx>("ZSTD_compressCCtx");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");

    let need_c = unsafe { c_ec(1) };
    let need_d = unsafe { c_ed() };

    // undersized, exact and oversized workspaces; also misaligned starts
    for &(sz, label) in &[
        (0usize, "zero"),
        (1, "one"),
        (need_c / 2, "half"),
        (need_c - 1, "one-short"),
        (need_c, "exact"),
        (need_c + 1024, "over"),
    ] {
        for off in [0usize, 1, 3, 8] {
            let mut wc = vec![0u8; sz + 16];
            let mut wr = vec![0u8; sz + 16];
            let a = unsafe { c_isc(wc.as_mut_ptr().add(off), sz) };
            let b = unsafe { r_isc(wr.as_mut_ptr().add(off), sz) };
            assert_eq_dbg(
                &format!("initStaticCCtx({label}={sz}, off={off}) null-ness"),
                a.is_null(),
                b.is_null(),
            );
            if !a.is_null() && !b.is_null() {
                // and it must actually work identically
                let mut rng = Rng::new(0x57A71C);
                let src = gen_shape(Shape::SkewedText, 4000, &mut rng);
                let cap = unsafe { c_bound(src.len()) };
                let mut cb = vec![0u8; cap];
                let mut rb = vec![0u8; cap];
                let x = unsafe {
                    c_cc(a, cb.as_mut_ptr(), cap, src.as_ptr(), src.len(), 1)
                };
                let y = unsafe {
                    r_cc(b, rb.as_mut_ptr(), cap, src.as_ptr(), src.len(), 1)
                };
                assert_eq_dbg(&format!("static cctx compress ({label})"), x, y);
                if x < cap {
                    assert_bytes_eq("static cctx bytes", &cb[..x], &rb[..y]);
                }
            }
        }
    }

    for &(sz, label) in &[
        (0usize, "zero"),
        (need_d / 2, "half"),
        (need_d - 1, "one-short"),
        (need_d, "exact"),
        (need_d + 4096, "over"),
    ] {
        let mut wc = vec![0u8; sz + 16];
        let mut wr = vec![0u8; sz + 16];
        let a = unsafe { c_isd(wc.as_mut_ptr(), sz) };
        let b = unsafe { r_isd(wr.as_mut_ptr(), sz) };
        assert_eq_dbg(
            &format!("initStaticDCtx({label}={sz}) null-ness"),
            a.is_null(),
            b.is_null(),
        );
    }

    // streaming variants need much larger workspaces; just compare null-ness
    // across a wide sweep, which is where the C's sizing logic shows up.
    for sz in [0usize, 1 << 10, 1 << 14, 1 << 16, 1 << 18, 1 << 20, 1 << 22] {
        let mut wc = vec![0u8; sz];
        let mut wr = vec![0u8; sz];
        let a = unsafe { c_iscs(wc.as_mut_ptr(), sz) };
        let b = unsafe { r_iscs(wr.as_mut_ptr(), sz) };
        assert_eq_dbg(
            &format!("initStaticCStream({sz}) null-ness"),
            a.is_null(),
            b.is_null(),
        );
        let mut wc = vec![0u8; sz];
        let mut wr = vec![0u8; sz];
        let a = unsafe { c_isds(wc.as_mut_ptr(), sz) };
        let b = unsafe { r_isds(wr.as_mut_ptr(), sz) };
        assert_eq_dbg(
            &format!("initStaticDStream({sz}) null-ness"),
            a.is_null(),
            b.is_null(),
        );
    }
}

/// Skippable frames: write then read back, across every magic variant and many
/// payload sizes, plus the rejection paths.
#[test]
fn skippable_frames_match() {
    let i = impls();
    let (c_w, r_w) = i
        .pair::<unsafe extern "C" fn(*mut u8, usize, *const u8, usize, u32) -> usize>(
            "ZSTD_writeSkippableFrame",
        );
    // NOTE the argument order: in the C, `magicVariant` is the THIRD parameter,
    // NOT the last —
    //   ZSTD_readSkippableFrame(void* dst, size_t dstCapacity,
    //                           unsigned* magicVariant,
    //                           const void* src, size_t srcSize)
    // (zstd.h / decompress/zstd_decompress.c:614). Getting this wrong makes the
    // library read the frame out of the `magicVariant` pointer and segfault.
    let (c_r, r_r) = i
        .pair::<unsafe extern "C" fn(*mut u8, usize, *mut u32, *const u8, usize) -> usize>(
            "ZSTD_readSkippableFrame",
        );
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let mut rng = Rng::new(0x5C19_0000);
    for &payload_len in &[0usize, 1, 2, 3, 7, 8, 100, 4096, 70_000] {
        let payload = gen_shape(Shape::Random, payload_len, &mut rng);
        // magic variants 0..15 valid; 16 and above must be rejected
        for var in [0u32, 1, 7, 15, 16, 17, 255, u32::MAX] {
            // undersized, exact and oversized destinations
            let exact = payload_len + 8;
            for cap in [0usize, 1, 7, 8, exact.saturating_sub(1), exact, exact + 16] {
                let mut cb = vec![0u8; cap.max(1)];
                let mut rb = vec![0u8; cap.max(1)];
                let a = unsafe {
                    c_w(cb.as_mut_ptr(), cap, payload.as_ptr(), payload_len, var)
                };
                let b = unsafe {
                    r_w(rb.as_mut_ptr(), cap, payload.as_ptr(), payload_len, var)
                };
                let tag = format!("writeSkippableFrame len={payload_len} var={var} cap={cap}");
                assert_eq_dbg(&tag, a, b);
                unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
                if a > usize::MAX - 200 {
                    continue;
                }
                assert_bytes_eq(&tag, &cb[..a], &rb[..b]);

                // read it back — including into undersized destinations
                for rcap in [0usize, 1, payload_len.saturating_sub(1), payload_len, payload_len + 8]
                {
                    let mut d1 = vec![0u8; rcap.max(1)];
                    let mut d2 = vec![0u8; rcap.max(1)];
                    let mut v1 = 0xDEAD_BEEFu32;
                    let mut v2 = 0xDEAD_BEEFu32;
                    let x = unsafe { c_r(d1.as_mut_ptr(), rcap, &mut v1, cb.as_ptr(), a) };
                    let y = unsafe { r_r(d2.as_mut_ptr(), rcap, &mut v2, rb.as_ptr(), b) };
                    let t2 = format!("{tag} / read rcap={rcap}");
                    assert_eq_dbg(&t2, x, y);
                    assert_eq_dbg(&format!("{t2} magicVariant"), v1, v2);
                    unsafe { assert_eq_dbg(&format!("{t2} code"), c_cd(x), r_cd(y)) };
                    if x <= usize::MAX - 200 {
                        assert_bytes_eq(&t2, &d1[..x], &d2[..y]);
                    }
                }
            }
        }
    }

    // readSkippableFrame on things that are NOT skippable frames
    for (name, buf) in malformed_buffers(&mut rng) {
        let mut d1 = vec![0u8; 256];
        let mut d2 = vec![0u8; 256];
        let mut v1 = 0u32;
        let mut v2 = 0u32;
        let x = unsafe { c_r(d1.as_mut_ptr(), d1.len(), &mut v1, buf.as_ptr(), buf.len()) };
        let y = unsafe { r_r(d2.as_mut_ptr(), d2.len(), &mut v2, buf.as_ptr(), buf.len()) };
        assert_eq_dbg(&format!("readSkippableFrame[{name}]"), x, y);
        assert_eq_dbg(&format!("readSkippableFrame[{name}] variant"), v1, v2);
        unsafe { assert_eq_dbg(&format!("readSkippableFrame[{name}] code"), c_cd(x), r_cd(y)) };
    }
}

/// The raw BLOCK API (`ZSTD_getBlockSize` / `ZSTD_compressBlock` /
/// `ZSTD_decompressBlock` / `ZSTD_insertBlock`) — the lowest-level public entry
/// points, which bypass frame framing entirely.
#[test]
fn raw_block_api_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_dnew, r_dnew) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_set, r_set) = i.pair::<Fn_setParam>("ZSTD_CCtx_setParameter");
    let (c_rst, r_rst) = i.pair::<Fn_reset>("ZSTD_CCtx_reset");
    let (c_gbs, r_gbs) = i.pair::<unsafe extern "C" fn(CCtx) -> usize>("ZSTD_getBlockSize");
    let (c_cb, r_cb) =
        i.pair::<unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_compressBlock",
        );
    let (c_db, r_db) =
        i.pair::<unsafe extern "C" fn(DCtx, *mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_decompressBlock",
        );
    let (c_ib, r_ib) =
        i.pair::<unsafe extern "C" fn(DCtx, *const u8, usize) -> usize>("ZSTD_insertBlock");
    let (c_cbeg, r_cbeg) =
        i.pair::<unsafe extern "C" fn(CCtx, i32) -> usize>("ZSTD_compressBegin");
    let (c_dbeg, r_dbeg) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_decompressBegin");
    let (c_cd, r_cd) = i.pair::<Fn_errCode>("ZSTD_getErrorCode");

    let cc = unsafe { c_new() };
    let rc = unsafe { r_new() };
    let cd = unsafe { c_dnew() };
    let rd = unsafe { r_dnew() };
    let mut rng = Rng::new(0xB10C_C0DE);

    // block size depends on windowLog — sweep it
    for &wl in &[10i32, 15, 17, 20, 27] {
        unsafe {
            c_rst(cc, ZSTD_reset_session_and_parameters);
            r_rst(rc, ZSTD_reset_session_and_parameters);
            assert_eq_dbg(
                "set windowLog",
                c_set(cc, ZSTD_c_windowLog, wl),
                r_set(rc, ZSTD_c_windowLog, wl),
            );
            assert_eq_dbg("compressBegin", c_cbeg(cc, 3), r_cbeg(rc, 3));
            assert_eq_dbg(&format!("getBlockSize wl={wl}"), c_gbs(cc), r_gbs(rc));
        }
        let bs = unsafe { c_gbs(cc) };

        for &shape in &ALL_SHAPES {
            // sizes around the block-size boundary, including 0 and bs+1
            for &len in &[0usize, 1, 2, 100, bs / 2, bs - 1, bs, bs + 1] {
                if len > 200_000 {
                    continue;
                }
                let src = gen_shape(shape, len, &mut rng);
                unsafe {
                    c_rst(cc, ZSTD_reset_session_and_parameters);
                    r_rst(rc, ZSTD_reset_session_and_parameters);
                    c_set(cc, ZSTD_c_windowLog, wl);
                    r_set(rc, ZSTD_c_windowLog, wl);
                    c_cbeg(cc, 3);
                    r_cbeg(rc, 3);
                }
                let cap = len + 1024;
                let mut cb_ = vec![0u8; cap];
                let mut rb_ = vec![0u8; cap];
                let a = unsafe { c_cb(cc, cb_.as_mut_ptr(), cap, src.as_ptr(), len) };
                let b = unsafe { r_cb(rc, rb_.as_mut_ptr(), cap, src.as_ptr(), len) };
                let tag = format!("compressBlock wl={wl} shape={shape:?} len={len}");
                assert_eq_dbg(&tag, a, b);
                unsafe { assert_eq_dbg(&format!("{tag} code"), c_cd(a), r_cd(b)) };
                if a > usize::MAX - 200 {
                    continue;
                }
                assert_bytes_eq(&tag, &cb_[..a], &rb_[..b]);

                // decompress the block back (a==0 means "not compressible", the
                // caller is expected to store the raw block itself)
                unsafe {
                    assert_eq_dbg("decompressBegin", c_dbeg(cd), r_dbeg(rd));
                }
                if a == 0 {
                    // insertBlock is the documented path for uncompressed blocks
                    let x = unsafe { c_ib(cd, src.as_ptr(), len) };
                    let y = unsafe { r_ib(rd, src.as_ptr(), len) };
                    assert_eq_dbg(&format!("{tag} / insertBlock"), x, y);
                } else {
                    let mut d1 = vec![0u8; len.max(1) + 64];
                    let mut d2 = vec![0u8; len.max(1) + 64];
                    let x = unsafe { c_db(cd, d1.as_mut_ptr(), d1.len(), cb_.as_ptr(), a) };
                    let y = unsafe { r_db(rd, d2.as_mut_ptr(), d2.len(), rb_.as_ptr(), b) };
                    assert_eq_dbg(&format!("{tag} / decompressBlock"), x, y);
                    unsafe {
                        assert_eq_dbg(&format!("{tag} / decompressBlock code"), c_cd(x), r_cd(y))
                    };
                    if x <= usize::MAX - 200 {
                        assert_bytes_eq(&format!("{tag} / block payload"), &d1[..x], &d2[..y]);
                    }
                }

                // undersized destination sweep for compressBlock
                for small in [0usize, 1, 8, a / 2] {
                    unsafe {
                        c_rst(cc, ZSTD_reset_session_and_parameters);
                        r_rst(rc, ZSTD_reset_session_and_parameters);
                        c_set(cc, ZSTD_c_windowLog, wl);
                        r_set(rc, ZSTD_c_windowLog, wl);
                        c_cbeg(cc, 3);
                        r_cbeg(rc, 3);
                    }
                    let mut c2 = vec![0u8; small.max(1)];
                    let mut r2 = vec![0u8; small.max(1)];
                    let x = unsafe { c_cb(cc, c2.as_mut_ptr(), small, src.as_ptr(), len) };
                    let y = unsafe { r_cb(rc, r2.as_mut_ptr(), small, src.as_ptr(), len) };
                    assert_eq_dbg(&format!("{tag} / dst={small}"), x, y);
                    unsafe {
                        assert_eq_dbg(&format!("{tag} / dst={small} code"), c_cd(x), r_cd(y))
                    };
                }
            }
        }
    }

    // decompressBlock on garbage
    for (name, buf) in malformed_buffers(&mut rng) {
        unsafe {
            c_dbeg(cd);
            r_dbeg(rd);
        }
        let mut d1 = vec![0u8; 1 << 18];
        let mut d2 = vec![0u8; 1 << 18];
        let x = unsafe { c_db(cd, d1.as_mut_ptr(), d1.len(), buf.as_ptr(), buf.len()) };
        let y = unsafe { r_db(rd, d2.as_mut_ptr(), d2.len(), buf.as_ptr(), buf.len()) };
        assert_eq_dbg(&format!("decompressBlock[{name}]"), x, y);
        unsafe { assert_eq_dbg(&format!("decompressBlock[{name}] code"), c_cd(x), r_cd(y)) };
        if x <= usize::MAX - 200 {
            assert_bytes_eq(&format!("decompressBlock[{name}] payload"), &d1[..x], &d2[..y]);
        }
    }

    unsafe {
        c_free(cc);
        r_free(rc);
        c_dfree(cd);
        r_dfree(rd);
    }
}

/// `ZSTD_copyCCtx` / `ZSTD_copyDCtx` — cloning a prepared context must yield a
/// context that compresses identically in both libraries.
#[test]
fn copy_context_matches() {
    let i = impls();
    let (c_new, r_new) = i.pair::<Fn_createCCtx>("ZSTD_createCCtx");
    let (c_free, r_free) = i.pair::<Fn_freeCCtx>("ZSTD_freeCCtx");
    let (c_cpy, r_cpy) =
        i.pair::<unsafe extern "C" fn(CCtx, CCtx, u64) -> usize>("ZSTD_copyCCtx");
    let (c_cbeg, r_cbeg) = i.pair::<unsafe extern "C" fn(CCtx, i32) -> usize>("ZSTD_compressBegin");
    let (c_cb, r_cb) =
        i.pair::<unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_compressContinue",
        );
    let (c_ce, r_ce) =
        i.pair::<unsafe extern "C" fn(CCtx, *mut u8, usize, *const u8, usize) -> usize>(
            "ZSTD_compressEnd",
        );
    let (c_dnew, r_dnew) = i.pair::<Fn_createDCtx>("ZSTD_createDCtx");
    let (c_dfree, r_dfree) = i.pair::<Fn_freeDCtx>("ZSTD_freeDCtx");
    let (c_dcpy, r_dcpy) = i.pair::<unsafe extern "C" fn(DCtx, DCtx)>("ZSTD_copyDCtx");
    let (c_dbeg, r_dbeg) = i.pair::<unsafe extern "C" fn(DCtx) -> usize>("ZSTD_decompressBegin");
    let (c_bound, _) = i.pair::<Fn_bound>("ZSTD_compressBound");

    let mut rng = Rng::new(0xC0FA_C700);
    for &lvl in &[1i32, 3, 9, 19] {
        for pledge in [ZSTD_CONTENTSIZE_UNKNOWN, 0u64, 5000] {
            let src = gen_shape(Shape::Tabular, 5000, &mut rng);

            let csrc = unsafe { c_new() };
            let rsrc = unsafe { r_new() };
            let cdst = unsafe { c_new() };
            let rdst = unsafe { r_new() };
            unsafe {
                assert_eq_dbg("compressBegin", c_cbeg(csrc, lvl), r_cbeg(rsrc, lvl));
                assert_eq_dbg(
                    &format!("copyCCtx pledge={pledge}"),
                    c_cpy(cdst, csrc, pledge),
                    r_cpy(rdst, rsrc, pledge),
                );
            }

            let cap = unsafe { c_bound(src.len()) } + 64;
            let mut cb_ = vec![0u8; cap];
            let mut rb_ = vec![0u8; cap];
            let half = src.len() / 2;
            let a1 = unsafe { c_cb(cdst, cb_.as_mut_ptr(), cap, src.as_ptr(), half) };
            let b1 = unsafe { r_cb(rdst, rb_.as_mut_ptr(), cap, src.as_ptr(), half) };
            assert_eq_dbg(&format!("compressContinue lvl={lvl}"), a1, b1);
            let a2 = unsafe {
                c_ce(
                    cdst,
                    cb_.as_mut_ptr().add(a1),
                    cap - a1,
                    src.as_ptr().add(half),
                    src.len() - half,
                )
            };
            let b2 = unsafe {
                r_ce(
                    rdst,
                    rb_.as_mut_ptr().add(b1),
                    cap - b1,
                    src.as_ptr().add(half),
                    src.len() - half,
                )
            };
            assert_eq_dbg(&format!("compressEnd lvl={lvl}"), a2, b2);
            if a1 <= usize::MAX - 200 && a2 <= usize::MAX - 200 {
                assert_bytes_eq(
                    &format!("copyCCtx frame lvl={lvl} pledge={pledge}"),
                    &cb_[..a1 + a2],
                    &rb_[..b1 + b2],
                );
            }

            // copyDCtx
            let cds = unsafe { c_dnew() };
            let rds = unsafe { r_dnew() };
            let cdd = unsafe { c_dnew() };
            let rdd = unsafe { r_dnew() };
            unsafe {
                c_dbeg(cds);
                r_dbeg(rds);
                c_dcpy(cdd, cds);
                r_dcpy(rdd, rds);
                c_dfree(cds);
                r_dfree(rds);
                c_dfree(cdd);
                r_dfree(rdd);
                c_free(csrc);
                r_free(rsrc);
                c_free(cdst);
                r_free(rdst);
            }
        }
    }
}
