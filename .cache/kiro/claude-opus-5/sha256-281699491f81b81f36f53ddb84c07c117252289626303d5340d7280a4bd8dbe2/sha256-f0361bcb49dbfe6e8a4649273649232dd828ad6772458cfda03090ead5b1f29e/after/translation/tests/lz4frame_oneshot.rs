//! Phase B — CONFIGS.md group 8: `lz4frame.c` one-shot frame compression across
//! the full preference cross-product, plus CDicts and the bound functions.
mod common;
use common::frame::*;
use common::*;

const SEED: u64 = 0x46_5241_4D45_0008;

fn is_err(v: usize) -> bool {
    // LZ4F_isError: code > (size_t)(-LZ4F_ERROR_maxCode) with maxCode == 24
    v > (0usize.wrapping_sub(24))
}

/// Rows 117-141: `LZ4F_compressFrame` + `LZ4F_compressFrameBound` over every
/// preference combination x size class x payload shape.
#[test]
fn g8_compress_frame_matrix() {
    let (cf, rf) = syms::<FnCompressFrame>("LZ4F_compressFrame");
    let (cfb, rfb) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let mut rng = Rng::new(SEED);
    let matrix = prefs_matrix();
    let lens: [usize; 12] = [0, 1, 11, 12, 13, 100, 65535, 65536, 65537, 262144, 300000, 500000];

    for (desc, p) in matrix.iter() {
        for &len in lens.iter() {
            for &shape in &[Shape::Random, Shape::Periodic, Shape::Textish, Shape::Constant] {
                let src = mkdata(shape, len, &mut rng);
                let cbound = unsafe { cfb(len, p) };
                let rbound = unsafe { rfb(len, p) };
                assert_eq!(cbound, rbound, "compressFrameBound({len}) [{desc}]");
                assert!(!is_err(cbound), "bound is an error for {desc}");
                let mut cd = vec![0x6Cu8; cbound];
                let mut rd = vec![0x6Cu8; cbound];
                let cn = unsafe { cf(cd.as_mut_ptr(), cbound, src.as_ptr(), len, p) };
                let rn = unsafe { rf(rd.as_mut_ptr(), cbound, src.as_ptr(), len, p) };
                let lbl = format!("compressFrame len={len} {shape:?} [{desc}]");
                assert_eq!(cn, rn, "{lbl}: C={cn:#x} R={rn:#x}");
                if !is_err(cn) {
                    assert_eq!(&cd[..cn], &rd[..cn], "{lbl}: output differs");
                }
                // NULL prefs
                if len < 70000 {
                    let nb = unsafe { cfb(len, std::ptr::null()) };
                    assert_eq!(nb, unsafe { rfb(len, std::ptr::null()) });
                    let mut cd = vec![0u8; nb];
                    let mut rd = vec![0u8; nb];
                    let a = unsafe { cf(cd.as_mut_ptr(), nb, src.as_ptr(), len, std::ptr::null()) };
                    let b = unsafe { rf(rd.as_mut_ptr(), nb, src.as_ptr(), len, std::ptr::null()) };
                    assert_eq!(a, b, "compressFrame(NULL prefs) len={len}");
                    if !is_err(a) {
                        assert_eq!(&cd[..a], &rd[..a]);
                    }
                }
            }
        }
    }
}

/// Row 140: `dstCapacity` exactly at and one byte below the bound.
#[test]
fn g8_compress_frame_tight_capacity() {
    let (cf, rf) = syms::<FnCompressFrame>("LZ4F_compressFrame");
    let (cfb, _) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let mut rng = Rng::new(SEED ^ 1);
    for (desc, p) in prefs_matrix_small().iter() {
        for &len in &[0usize, 1, 13, 1000, 65537, 300000] {
            let src = mkdata(Shape::Textish, len, &mut rng);
            let bound = unsafe { cfb(len, p) };
            // First learn the actual size, then probe capacities around it.
            let mut probe = vec![0u8; bound];
            let actual = unsafe { cf(probe.as_mut_ptr(), bound, src.as_ptr(), len, p) };
            assert!(!is_err(actual));
            for cap in [bound, actual, actual.saturating_sub(1), actual / 2, 1, 0] {
                let mut cd = vec![0xE1u8; cap.max(1)];
                let mut rd = vec![0xE1u8; cap.max(1)];
                let cn = unsafe { cf(cd.as_mut_ptr(), cap, src.as_ptr(), len, p) };
                let rn = unsafe { rf(rd.as_mut_ptr(), cap, src.as_ptr(), len, p) };
                let lbl = format!("compressFrame cap={cap} len={len} [{desc}]");
                assert_eq!(cn, rn, "{lbl}: C={cn:#x} R={rn:#x}");
                if !is_err(cn) {
                    assert_eq!(&cd[..cn], &rd[..cn], "{lbl}");
                }
            }
        }
    }
}

/// Rows 142-143: CDict creation and `LZ4F_compressFrame_usingCDict`.
#[test]
fn g8_cdict() {
    let (ccd, rcd) = syms::<FnCreateCDict>("LZ4F_createCDict");
    let (ccda, rcda) = syms::<FnCreateCDictAdvanced>("LZ4F_createCDict_advanced");
    let (cfd, rfd) = syms::<FnFreeCDict>("LZ4F_freeCDict");
    let (cf, rf) = syms::<FnCompressFrameCDict>("LZ4F_compressFrame_usingCDict");
    let (cfb, _) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let (ccc, rcc) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cfc, rfc) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let mut rng = Rng::new(SEED ^ 2);

    // one cctx per library, reused for every call (as a real consumer would)
    let mut cctx_c: *mut u8 = std::ptr::null_mut();
    let mut cctx_r: *mut u8 = std::ptr::null_mut();
    assert_eq!(
        unsafe { ccc(&mut cctx_c, LZ4F_VERSION) },
        unsafe { rcc(&mut cctx_r, LZ4F_VERSION) },
        "createCompressionContext"
    );

    for &ds in &[0usize, 1, 4, 100, 1000, 65536, 70000] {
        let dict = mkdata(Shape::Textish, ds, &mut rng);
        let cdc = unsafe { ccd(dict.as_ptr(), ds) };
        let cdr = unsafe { rcd(dict.as_ptr(), ds) };
        assert_eq!(cdc.is_null(), cdr.is_null(), "createCDict({ds}) nullness");
        if cdc.is_null() {
            continue;
        }
        // the _advanced constructor with LZ4F_defaultCMem must behave the same
        let cda_c = unsafe { ccda(CustomMem::default_cmem(), dict.as_ptr(), ds) };
        let cda_r = unsafe { rcda(CustomMem::default_cmem(), dict.as_ptr(), ds) };
        assert_eq!(cda_c.is_null(), cda_r.is_null(), "createCDict_advanced({ds})");
        for (desc, p) in prefs_matrix_small().iter() {
            for &len in &[0usize, 1, 13, 1000, 70000] {
                // make the payload share content with the dictionary
                let src = if ds > 16 {
                    let take = len.min(ds);
                    let mut v = dict[..take].to_vec();
                    v.extend(mkdata(Shape::Textish, len - take, &mut rng));
                    v
                } else {
                    mkdata(Shape::Textish, len, &mut rng)
                };
                let bound = unsafe { cfb(len, p) };
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe { cf(cctx_c, cd.as_mut_ptr(), bound, src.as_ptr(), len, cdc, p) };
                let rn = unsafe { rf(cctx_r, rd.as_mut_ptr(), bound, src.as_ptr(), len, cdr, p) };
                let lbl = format!("compressFrame_usingCDict ds={ds} len={len} [{desc}]");
                assert_eq!(cn, rn, "{lbl}: C={cn:#x} R={rn:#x}");
                if !is_err(cn) {
                    assert_eq!(&cd[..cn], &rd[..cn], "{lbl}");
                }
            }
        }
        // NULL CDict must behave like no dictionary
        let bound = unsafe { cfb(1000, std::ptr::null()) };
        let src = mkdata(Shape::Textish, 1000, &mut rng);
        let mut cd = vec![0u8; bound];
        let mut rd = vec![0u8; bound];
        let a = unsafe {
            cf(cctx_c, cd.as_mut_ptr(), bound, src.as_ptr(), 1000, std::ptr::null(), std::ptr::null())
        };
        let b = unsafe {
            rf(cctx_r, rd.as_mut_ptr(), bound, src.as_ptr(), 1000, std::ptr::null(), std::ptr::null())
        };
        assert_eq!(a, b, "compressFrame_usingCDict(NULL cdict)");
        if !is_err(a) {
            assert_eq!(&cd[..a], &rd[..a]);
        }
        unsafe { cfd(cdc) };
        unsafe { rfd(cdr) };
        unsafe { cfd(cda_c) };
        unsafe { rfd(cda_r) };
    }
    // freeCDict(NULL) must be a no-op in both
    unsafe { cfd(std::ptr::null_mut()) };
    unsafe { rfd(std::ptr::null_mut()) };
    assert_eq!(unsafe { cfc(cctx_c) }, unsafe { rfc(cctx_r) }, "freeCompressionContext");
}

/// Rows 144-146: `LZ4F_getBlockSize`, `LZ4F_compressBound`, and the scalars.
#[test]
fn g8_bounds_and_scalars() {
    let (cgb, rgb) = syms::<FnGetBlockSize>("LZ4F_getBlockSize");
    for id in [0u32, 1, 2, 3, 4, 5, 6, 7, 8, 9, 99, u32::MAX] {
        let a = unsafe { cgb(id) };
        let b = unsafe { rgb(id) };
        assert_eq!(a, b, "LZ4F_getBlockSize({id}): C={a:#x} R={b:#x}");
    }

    let (ccb, rcb) = syms::<FnFrameBound>("LZ4F_compressBound");
    for (desc, p) in prefs_matrix().iter() {
        for &n in &[
            0usize, 1, 12, 13, 100, 65535, 65536, 65537, 262143, 262144, 1 << 20, 1 << 22,
            (1 << 22) + 1, 10_000_000,
        ] {
            let a = unsafe { ccb(n, p) };
            let b = unsafe { rcb(n, p) };
            assert_eq!(a, b, "LZ4F_compressBound({n}) [{desc}]: C={a:#x} R={b:#x}");
        }
    }
    for &n in &[0usize, 1, 65536, 262144, 1 << 22] {
        assert_eq!(
            unsafe { ccb(n, std::ptr::null()) },
            unsafe { rcb(n, std::ptr::null()) },
            "LZ4F_compressBound({n}, NULL)"
        );
    }

    let (cv, rv) = syms::<FnGetVersion>("LZ4F_getVersion");
    assert_eq!(unsafe { cv() }, unsafe { rv() }, "LZ4F_getVersion");
    assert_eq!(unsafe { cv() }, LZ4F_VERSION, "LZ4F_VERSION mismatch");
    let (cm, rm) = syms::<FnLevelMax>("LZ4F_compressionLevel_max");
    assert_eq!(unsafe { cm() }, unsafe { rm() }, "LZ4F_compressionLevel_max");
    assert_eq!(unsafe { cm() }, 12);
}
