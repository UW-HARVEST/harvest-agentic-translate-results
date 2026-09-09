//! Phase C — error-path differential tests, part 2: `lz4frame.c` (rows 71-114),
//! `lz4file.c` (rows 115-138), `xxhash.c` (rows 139-148) and the generic
//! FFI-boundary rows 149-155 (out-of-range enum values in particular).
mod common;
use common::frame::*;
use common::*;
use std::ffi::{CStr, CString};

const SEED: u64 = 0x4552_524F_5200_0002;

/// `LZ4F_ERROR_maxCode` is 24 (verified from `LZ4F_LIST_ERRORS`).
const MAX_CODE: usize = 24;
fn is_err(v: usize) -> bool {
    v > (0usize.wrapping_sub(MAX_CODE))
}
/// Recover the enum ordinal from an error result.
fn code_of(v: usize) -> usize {
    0usize.wrapping_sub(v)
}

// ================================================ lz4frame rows 71-74 ========

/// Rows 71, 73: context creation with a NULL out-pointer.
#[test]
fn e_frame_create_null_out_ptr() {
    let (cc, rc) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cdx, rdx) = syms::<FnCreateDctx>("LZ4F_createDecompressionContext");
    for ver in [0u32, 1, LZ4F_VERSION, 99, 9999, u32::MAX] {
        let a = unsafe { cc(std::ptr::null_mut(), ver) };
        let b = unsafe { rc(std::ptr::null_mut(), ver) };
        assert_eq!(a, b, "row71 createCctx(NULL, {ver}): C={a:#x} R={b:#x}");
        assert!(is_err(a), "row71 C must reject a NULL cctxPtr");
        assert_eq!(code_of(a), 21, "row71 expected ERROR_parameter_null (21)");

        let a = unsafe { cdx(std::ptr::null_mut(), ver) };
        let b = unsafe { rdx(std::ptr::null_mut(), ver) };
        assert_eq!(a, b, "row73 createDctx(NULL, {ver}): C={a:#x} R={b:#x}");
        assert!(is_err(a), "row73 C must reject a NULL dctxPtr");
        assert_eq!(code_of(a), 21, "row73 expected ERROR_parameter_null (21)");
    }
}

/// Row 154: the `version` argument is accepted whatever its value.
#[test]
fn e_frame_version_ignored() {
    let (cc, rc) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cf, rf) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cd, rd) = syms::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (cfd, rfd) = syms::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    for ver in [0u32, 1, 99, LZ4F_VERSION, 101, 9999, u32::MAX] {
        let mut a: *mut u8 = std::ptr::null_mut();
        let mut b: *mut u8 = std::ptr::null_mut();
        let x = unsafe { cc(&mut a, ver) };
        let y = unsafe { rc(&mut b, ver) };
        assert_eq!(x, y, "row154 createCctx(ver={ver}): C={x:#x} R={y:#x}");
        assert_eq!(a.is_null(), b.is_null());
        assert_eq!(unsafe { cf(a) }, unsafe { rf(b) });

        let mut a: *mut u8 = std::ptr::null_mut();
        let mut b: *mut u8 = std::ptr::null_mut();
        let x = unsafe { cd(&mut a, ver) };
        let y = unsafe { rd(&mut b, ver) };
        assert_eq!(x, y, "row154 createDctx(ver={ver}): C={x:#x} R={y:#x}");
        assert_eq!(unsafe { cfd(a) }, unsafe { rfd(b) });
    }
}

// ================================================ lz4frame rows 75-84 ========

/// Row 75: `LZ4F_compressBegin*` with `dstCapacity` below the header size.
#[test]
fn e_frame_begin_dst_too_small() {
    let (ccc, rcc) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cfc, rfc) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cbg, rbg) = syms::<FnCompressBegin>("LZ4F_compressBegin");
    let (cbd, rbd) = syms::<FnCompressBeginDict>("LZ4F_compressBegin_usingDict");
    let (cb1, rb1) = syms::<FnCompressBeginDict>("LZ4F_compressBegin_usingDictOnce");
    let (cbc, rbc) = syms::<FnCompressBeginCDict>("LZ4F_compressBegin_usingCDict");
    let (ccd, rcd) = syms::<FnCreateCDict>("LZ4F_createCDict");
    let (cfd, rfd) = syms::<FnFreeCDict>("LZ4F_freeCDict");
    let mut rng = Rng::new(SEED);
    let dict = mkdata(Shape::Textish, 4096, &mut rng);
    let cdc = unsafe { ccd(dict.as_ptr(), 4096) };
    let cdr = unsafe { rcd(dict.as_ptr(), 4096) };

    for (desc, p) in prefs_matrix().iter() {
        for cap in 0..=HEADER_SIZE_MAX + 1 {
            let mut a: *mut u8 = std::ptr::null_mut();
            let mut b: *mut u8 = std::ptr::null_mut();
            unsafe { ccc(&mut a, LZ4F_VERSION) };
            unsafe { rcc(&mut b, LZ4F_VERSION) };
            let mut cd = vec![0x71u8; cap.max(1)];
            let mut rd = vec![0x71u8; cap.max(1)];
            let x = unsafe { cbg(a, cd.as_mut_ptr(), cap, p) };
            let y = unsafe { rbg(b, rd.as_mut_ptr(), cap, p) };
            assert_eq!(x, y, "row75 compressBegin(cap={cap}) [{desc}]: C={x:#x} R={y:#x}");
            if is_err(x) {
                assert_eq!(code_of(x), 11, "row75 expected ERROR_dstMaxSize_tooSmall (11)");
            } else {
                assert_eq!(&cd[..x], &rd[..y], "row75 header bytes differ (cap={cap})");
            }
            unsafe { cfc(a) };
            unsafe { rfc(b) };

            // the three dictionary-primed variants
            for (name, cfn, rfn) in [
                ("usingDict", cbd, rbd),
                ("usingDictOnce", cb1, rb1),
            ] {
                let mut a: *mut u8 = std::ptr::null_mut();
                let mut b: *mut u8 = std::ptr::null_mut();
                unsafe { ccc(&mut a, LZ4F_VERSION) };
                unsafe { rcc(&mut b, LZ4F_VERSION) };
                let mut cd = vec![0x71u8; cap.max(1)];
                let mut rd = vec![0x71u8; cap.max(1)];
                let x = unsafe { cfn(a, cd.as_mut_ptr(), cap, dict.as_ptr(), 4096, p) };
                let y = unsafe { rfn(b, rd.as_mut_ptr(), cap, dict.as_ptr(), 4096, p) };
                assert_eq!(x, y, "row75 compressBegin_{name}(cap={cap}) [{desc}]");
                if !is_err(x) {
                    assert_eq!(&cd[..x], &rd[..y]);
                }
                unsafe { cfc(a) };
                unsafe { rfc(b) };
            }
            let mut a: *mut u8 = std::ptr::null_mut();
            let mut b: *mut u8 = std::ptr::null_mut();
            unsafe { ccc(&mut a, LZ4F_VERSION) };
            unsafe { rcc(&mut b, LZ4F_VERSION) };
            let mut cd = vec![0x71u8; cap.max(1)];
            let mut rd = vec![0x71u8; cap.max(1)];
            let x = unsafe { cbc(a, cd.as_mut_ptr(), cap, cdc, p) };
            let y = unsafe { rbc(b, rd.as_mut_ptr(), cap, cdr, p) };
            assert_eq!(x, y, "row75 compressBegin_usingCDict(cap={cap}) [{desc}]");
            if !is_err(x) {
                assert_eq!(&cd[..x], &rd[..y]);
            }
            unsafe { cfc(a) };
            unsafe { rfc(b) };
        }
    }
    unsafe { cfd(cdc) };
    unsafe { rfd(cdr) };
}

/// Row 86: `dictSize > INT_MAX`.
///
/// Only `LZ4F_compressBegin_usingDict*` performs this check, and it does so
/// (lz4frame.c:768) BEFORE touching `dictBuffer`, so a small buffer with a huge
/// declared size is a valid probe. `LZ4F_createCDict` has NO such check: it goes
/// straight to `dictStart += dictSize - 64 KB; memcpy(...)` (lz4frame.c:546-558),
/// which reads far out of bounds. A huge `dictSize` is therefore not a testable
/// input for `createCDict` -- both libraries segfault identically, which proves
/// nothing -- so it is probed only with sizes it actually handles.
#[test]
fn e_frame_dict_size_too_large() {
    let (ccd, rcd) = syms::<FnCreateCDict>("LZ4F_createCDict");
    let (cbd, rbd) = syms::<FnCompressBeginDict>("LZ4F_compressBegin_usingDict");
    let (ccc, rcc) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cfc, rfc) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let mut rng = Rng::new(SEED ^ 1);
    let dict = mkdata(Shape::Textish, 4096, &mut rng);

    for ds in [
        (i32::MAX as usize) + 1,
        (i32::MAX as usize) + 2,
        usize::MAX / 2,
        usize::MAX,
    ] {
        let mut a: *mut u8 = std::ptr::null_mut();
        let mut b: *mut u8 = std::ptr::null_mut();
        unsafe { ccc(&mut a, LZ4F_VERSION) };
        unsafe { rcc(&mut b, LZ4F_VERSION) };
        let mut cd = vec![0u8; 64];
        let mut rd = vec![0u8; 64];
        let x = unsafe { cbd(a, cd.as_mut_ptr(), 64, dict.as_ptr(), ds, std::ptr::null()) };
        let y = unsafe { rbd(b, rd.as_mut_ptr(), 64, dict.as_ptr(), ds, std::ptr::null()) };
        assert_eq!(x, y, "row86 compressBegin_usingDict(ds={ds:#x}): C={x:#x} R={y:#x}");
        assert!(is_err(x), "row86 C must reject dictSize > INT_MAX");
        assert_eq!(code_of(x), 4, "row86 expected ERROR_parameter_invalid (4)");
        unsafe { cfc(a) };
        unsafe { rfc(b) };

    }

    // createCDict over the sizes it does handle, including the 64 KB truncation
    // boundary and a size larger than the buffer's own length is NOT probed
    // (out-of-bounds read); only in-bounds sizes are used here.
    let (cfd, rfd) = syms::<FnFreeCDict>("LZ4F_freeCDict");
    for ds in [0usize, 1, 4, 4095, 4096] {
        let ca = unsafe { ccd(dict.as_ptr(), ds) };
        let ra = unsafe { rcd(dict.as_ptr(), ds) };
        assert_eq!(ca.is_null(), ra.is_null(), "row86 createCDict(ds={ds})");
        unsafe { cfd(ca) };
        unsafe { rfd(ra) };
    }
    let big = mkdata(Shape::Textish, 200_000, &mut rng);
    for ds in [65535usize, 65536, 65537, 100_000, 200_000] {
        let ca = unsafe { ccd(big.as_ptr(), ds) };
        let ra = unsafe { rcd(big.as_ptr(), ds) };
        assert_eq!(ca.is_null(), ra.is_null(), "row86 createCDict(ds={ds})");
        unsafe { cfd(ca) };
        unsafe { rfd(ra) };
    }
}

/// Rows 77, 79: `compressUpdate` / `uncompressedUpdate` before `compressBegin`
/// (and after `compressEnd`) -> `ERROR_compressionState_uninitialized`.
#[test]
fn e_frame_state_uninitialized() {
    let (ccc, rcc) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cfc, rfc) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cu, ru) = syms::<FnCompressUpdate>("LZ4F_compressUpdate");
    let (cuu, ruu) = syms::<FnCompressUpdate>("LZ4F_uncompressedUpdate");
    let (cbg, rbg) = syms::<FnCompressBegin>("LZ4F_compressBegin");
    let (cend, rend) = syms::<FnFlush>("LZ4F_compressEnd");
    let mut rng = Rng::new(SEED ^ 2);
    let src = mkdata(Shape::Textish, 5000, &mut rng);

    for (name, cfn, rfn) in [
        ("compressUpdate", cu, ru),
        ("uncompressedUpdate", cuu, ruu),
    ] {
        // (a) fresh context, no compressBegin
        let mut a: *mut u8 = std::ptr::null_mut();
        let mut b: *mut u8 = std::ptr::null_mut();
        unsafe { ccc(&mut a, LZ4F_VERSION) };
        unsafe { rcc(&mut b, LZ4F_VERSION) };
        let mut cd = vec![0u8; 100_000];
        let mut rd = vec![0u8; 100_000];
        let x = unsafe { cfn(a, cd.as_mut_ptr(), cd.len(), src.as_ptr(), 5000, std::ptr::null()) };
        let y = unsafe { rfn(b, rd.as_mut_ptr(), rd.len(), src.as_ptr(), 5000, std::ptr::null()) };
        assert_eq!(x, y, "rows77/79 {name} before begin: C={x:#x} R={y:#x}");
        assert!(is_err(x), "rows77/79 C must reject an uninitialised state");
        assert_eq!(
            code_of(x),
            20,
            "rows77/79 expected ERROR_compressionState_uninitialized (20)"
        );

        // (b) after compressEnd
        let mut h = vec![0u8; 64];
        let mut h2 = vec![0u8; 64];
        let mut p = Prefs::default();
        p.frame_info.block_mode = 1; // uncompressedUpdate needs independent blocks
        unsafe { cbg(a, h.as_mut_ptr(), 64, &p) };
        unsafe { rbg(b, h2.as_mut_ptr(), 64, &p) };
        let mut e1 = vec![0u8; 1024];
        let mut e2 = vec![0u8; 1024];
        unsafe { cend(a, e1.as_mut_ptr(), 1024, std::ptr::null()) };
        unsafe { rend(b, e2.as_mut_ptr(), 1024, std::ptr::null()) };
        let x = unsafe { cfn(a, cd.as_mut_ptr(), cd.len(), src.as_ptr(), 5000, std::ptr::null()) };
        let y = unsafe { rfn(b, rd.as_mut_ptr(), rd.len(), src.as_ptr(), 5000, std::ptr::null()) };
        assert_eq!(x, y, "rows77/79 {name} after end: C={x:#x} R={y:#x}");
        assert_eq!(code_of(x), 20, "rows77/79 expected code 20 after compressEnd");
        unsafe { cfc(a) };
        unsafe { rfc(b) };
    }
}

/// Rows 78, 80, 81, 82: insufficient `dstCapacity` on `compressUpdate`,
/// `uncompressedUpdate`, `flush` and `compressEnd`.
#[test]
fn e_frame_update_dst_too_small() {
    let (ccc, rcc) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cfc, rfc) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cbg, rbg) = syms::<FnCompressBegin>("LZ4F_compressBegin");
    let (cu, ru) = syms::<FnCompressUpdate>("LZ4F_compressUpdate");
    let (cuu, ruu) = syms::<FnCompressUpdate>("LZ4F_uncompressedUpdate");
    let (cfl, rfl) = syms::<FnFlush>("LZ4F_flush");
    let (cend, rend) = syms::<FnFlush>("LZ4F_compressEnd");
    let (cbnd, _) = syms::<FnFrameBound>("LZ4F_compressBound");
    let mut rng = Rng::new(SEED ^ 3);

    for (desc, p) in prefs_matrix_small().iter() {
        for &len in &[1usize, 100, 70000] {
            let src = mkdata(Shape::Textish, len, &mut rng);
            let bound = unsafe { cbnd(len, p) };
            // Row 78/80: sweep capacities from 0 up to the required bound
            for cap in [
                0usize, 1, 2, 3, 4, 8, 16, bound / 4, bound / 2, bound - 1, bound,
            ] {
                for uncompressed in [false, true] {
                    if uncompressed && p.frame_info.block_mode != 1 {
                        continue; // only supported with independent blocks
                    }
                    let mut a: *mut u8 = std::ptr::null_mut();
                    let mut b: *mut u8 = std::ptr::null_mut();
                    unsafe { ccc(&mut a, LZ4F_VERSION) };
                    unsafe { rcc(&mut b, LZ4F_VERSION) };
                    let mut h = vec![0u8; 64];
                    let mut h2 = vec![0u8; 64];
                    unsafe { cbg(a, h.as_mut_ptr(), 64, p) };
                    unsafe { rbg(b, h2.as_mut_ptr(), 64, p) };
                    let mut cd = vec![0x7Au8; cap.max(1)];
                    let mut rd = vec![0x7Au8; cap.max(1)];
                    let (cfn, rfn) = if uncompressed { (cuu, ruu) } else { (cu, ru) };
                    let x = unsafe {
                        cfn(a, cd.as_mut_ptr(), cap, src.as_ptr(), len, std::ptr::null())
                    };
                    let y = unsafe {
                        rfn(b, rd.as_mut_ptr(), cap, src.as_ptr(), len, std::ptr::null())
                    };
                    assert_eq!(
                        x, y,
                        "rows78/80 update(cap={cap}) uncompressed={uncompressed} len={len} [{desc}]: C={x:#x} R={y:#x}"
                    );
                    if is_err(x) {
                        assert_eq!(
                            code_of(x),
                            11,
                            "rows78/80 expected ERROR_dstMaxSize_tooSmall (11)"
                        );
                    } else {
                        assert_eq!(&cd[..x], &rd[..y]);
                    }
                    unsafe { cfc(a) };
                    unsafe { rfc(b) };
                }
            }

            // Rows 81/82: flush and compressEnd with a too-small destination
            for cap in [0usize, 1, 2, 3, 4, 5, 8, 16] {
                let mut a: *mut u8 = std::ptr::null_mut();
                let mut b: *mut u8 = std::ptr::null_mut();
                unsafe { ccc(&mut a, LZ4F_VERSION) };
                unsafe { rcc(&mut b, LZ4F_VERSION) };
                let mut h = vec![0u8; 64];
                let mut h2 = vec![0u8; 64];
                unsafe { cbg(a, h.as_mut_ptr(), 64, p) };
                unsafe { rbg(b, h2.as_mut_ptr(), 64, p) };
                // buffer some data so flush has work to do
                let big = unsafe { cbnd(len, p) };
                let mut t1 = vec![0u8; big];
                let mut t2 = vec![0u8; big];
                unsafe { cu(a, t1.as_mut_ptr(), big, src.as_ptr(), len, std::ptr::null()) };
                unsafe { ru(b, t2.as_mut_ptr(), big, src.as_ptr(), len, std::ptr::null()) };

                let mut cd = vec![0x7Bu8; cap.max(1)];
                let mut rd = vec![0x7Bu8; cap.max(1)];
                let x = unsafe { cfl(a, cd.as_mut_ptr(), cap, std::ptr::null()) };
                let y = unsafe { rfl(b, rd.as_mut_ptr(), cap, std::ptr::null()) };
                assert_eq!(x, y, "row81 flush(cap={cap}) len={len} [{desc}]: C={x:#x} R={y:#x}");
                if !is_err(x) {
                    assert_eq!(&cd[..x], &rd[..y]);
                }
                let mut cd = vec![0x7Cu8; cap.max(1)];
                let mut rd = vec![0x7Cu8; cap.max(1)];
                let x = unsafe { cend(a, cd.as_mut_ptr(), cap, std::ptr::null()) };
                let y = unsafe { rend(b, rd.as_mut_ptr(), cap, std::ptr::null()) };
                assert_eq!(x, y, "row82 compressEnd(cap={cap}) len={len} [{desc}]: C={x:#x} R={y:#x}");
                if !is_err(x) {
                    assert_eq!(&cd[..x], &rd[..y]);
                }
                unsafe { cfc(a) };
                unsafe { rfc(b) };
            }
        }
    }
}

/// Row 83: `compressEnd` when the declared `contentSize` does not match what was
/// actually fed -> `ERROR_frameSize_wrong`.
#[test]
fn e_frame_content_size_mismatch() {
    let (ccc, rcc) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cfc, rfc) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cbg, rbg) = syms::<FnCompressBegin>("LZ4F_compressBegin");
    let (cu, ru) = syms::<FnCompressUpdate>("LZ4F_compressUpdate");
    let (cend, rend) = syms::<FnFlush>("LZ4F_compressEnd");
    let (cbnd, _) = syms::<FnFrameBound>("LZ4F_compressBound");
    let mut rng = Rng::new(SEED ^ 4);

    for &declared in &[1u64, 100, 5000, 4999, 5001, u64::MAX] {
        for &actual in &[0usize, 1, 5000] {
            let mut p = Prefs::default();
            p.frame_info.content_size = declared;
            let src = mkdata(Shape::Textish, actual, &mut rng);
            let mut a: *mut u8 = std::ptr::null_mut();
            let mut b: *mut u8 = std::ptr::null_mut();
            unsafe { ccc(&mut a, LZ4F_VERSION) };
            unsafe { rcc(&mut b, LZ4F_VERSION) };
            let mut h = vec![0u8; 64];
            let mut h2 = vec![0u8; 64];
            let x = unsafe { cbg(a, h.as_mut_ptr(), 64, &p) };
            let y = unsafe { rbg(b, h2.as_mut_ptr(), 64, &p) };
            assert_eq!(x, y);
            assert_eq!(&h[..x], &h2[..y]);
            if actual > 0 {
                let cap = unsafe { cbnd(actual, &p) };
                let mut t1 = vec![0u8; cap];
                let mut t2 = vec![0u8; cap];
                let x = unsafe { cu(a, t1.as_mut_ptr(), cap, src.as_ptr(), actual, std::ptr::null()) };
                let y = unsafe { ru(b, t2.as_mut_ptr(), cap, src.as_ptr(), actual, std::ptr::null()) };
                assert_eq!(x, y);
                assert_eq!(&t1[..x], &t2[..y]);
            }
            let cap = unsafe { cbnd(0, &p) }.max(32);
            let mut cd = vec![0u8; cap];
            let mut rd = vec![0u8; cap];
            let x = unsafe { cend(a, cd.as_mut_ptr(), cap, std::ptr::null()) };
            let y = unsafe { rend(b, rd.as_mut_ptr(), cap, std::ptr::null()) };
            assert_eq!(
                x, y,
                "row83 compressEnd declared={declared} actual={actual}: C={x:#x} R={y:#x}"
            );
            if declared != actual as u64 {
                assert!(is_err(x), "row83 C must reject the size mismatch");
                assert_eq!(code_of(x), 14, "row83 expected ERROR_frameSize_wrong (14)");
            } else {
                assert!(!is_err(x));
                assert_eq!(&cd[..x], &rd[..y]);
            }
            unsafe { cfc(a) };
            unsafe { rfc(b) };
        }
    }
}

/// Row 84: `LZ4F_compressFrame*` with `dstCapacity` below the frame bound.
#[test]
fn e_frame_one_shot_dst_too_small() {
    let (cf, rf) = syms::<FnCompressFrame>("LZ4F_compressFrame");
    let (cfb, _) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let mut rng = Rng::new(SEED ^ 5);
    for (desc, p) in prefs_matrix_small().iter() {
        for &len in &[0usize, 1, 13, 1000, 70000] {
            let src = mkdata(Shape::Textish, len, &mut rng);
            let bound = unsafe { cfb(len, p) };
            for cap in [0usize, 1, 2, 5, 10, 18, 19, 20, bound / 2, bound - 1, bound] {
                let mut cd = vec![0x84u8; cap.max(1)];
                let mut rd = vec![0x84u8; cap.max(1)];
                let x = unsafe { cf(cd.as_mut_ptr(), cap, src.as_ptr(), len, p) };
                let y = unsafe { rf(rd.as_mut_ptr(), cap, src.as_ptr(), len, p) };
                assert_eq!(
                    x, y,
                    "row84 compressFrame(cap={cap}) len={len} [{desc}]: C={x:#x} R={y:#x}"
                );
                if !is_err(x) {
                    assert_eq!(&cd[..x], &rd[..y]);
                }
            }
        }
    }
}

// =============================================== lz4frame rows 87-104 ========

/// Rows 87-89: `LZ4F_headerSize` rejections.
#[test]
fn e_frame_header_size_rejections() {
    let (c, r) = syms::<FnHeaderSize>("LZ4F_headerSize");
    let mut rng = Rng::new(SEED ^ 6);
    let junk = mkdata(Shape::Random, 64, &mut rng);

    // Row 87: NULL src
    for n in [0usize, 1, 4, 5, 19, 64] {
        let a = unsafe { c(std::ptr::null(), n) };
        let b = unsafe { r(std::ptr::null(), n) };
        assert_eq!(a, b, "row87 headerSize(NULL, {n}): C={a:#x} R={b:#x}");
        assert!(is_err(a), "row87 C must reject NULL src");
        assert_eq!(code_of(a), 15, "row87 expected ERROR_srcPtr_wrong (15)");
    }
    // Row 88: srcSize < 5
    for n in [0usize, 1, 2, 3, 4] {
        let a = unsafe { c(junk.as_ptr(), n) };
        let b = unsafe { r(junk.as_ptr(), n) };
        assert_eq!(a, b, "row88 headerSize(n={n}): C={a:#x} R={b:#x}");
        assert!(is_err(a), "row88 C must reject srcSize < 5");
        assert_eq!(code_of(a), 12, "row88 expected ERROR_frameHeader_incomplete (12)");
    }
    // Row 89: unknown magic. Sweep the whole 32-bit magic space at the byte
    // level around the two valid families.
    let mut probe = vec![0u8; 32];
    for magic in [
        0u32,
        1,
        0x184D2203,
        0x184D2205,
        0x184D2A4F,
        0x184D2A50,
        0x184D2A5F,
        0x184D2A60,
        0x184D2204u32.swap_bytes(),
        0xFFFF_FFFF,
    ] {
        probe[..4].copy_from_slice(&magic.to_le_bytes());
        for n in [5usize, 6, 7, 8, 19, 32] {
            let a = unsafe { c(probe.as_ptr(), n) };
            let b = unsafe { r(probe.as_ptr(), n) };
            assert_eq!(a, b, "row89 headerSize(magic={magic:#x}, n={n}): C={a:#x} R={b:#x}");
            let valid = magic == 0x184D2204 || (magic & 0xFFFF_FFF0) == 0x184D2A50;
            if !valid {
                assert!(is_err(a), "row89 C must reject magic {magic:#x}");
                assert_eq!(code_of(a), 13, "row89 expected ERROR_frameType_unknown (13)");
            }
        }
    }
    // exhaustive over the low byte of the skippable family
    for low in 0u32..=0xFF {
        let magic = 0x184D2A00u32 | low;
        probe[..4].copy_from_slice(&magic.to_le_bytes());
        let a = unsafe { c(probe.as_ptr(), 8) };
        let b = unsafe { r(probe.as_ptr(), 8) };
        assert_eq!(a, b, "row89 headerSize(skippable low={low:#x}): C={a:#x} R={b:#x}");
    }
}

/// Rows 90-91: `LZ4F_getFrameInfo` rejections.
#[test]
fn e_frame_get_frame_info_rejections() {
    let (cgi, rgi) = syms::<FnGetFrameInfo>("LZ4F_getFrameInfo");
    let (cd, rd) = syms::<FnDecompress>("LZ4F_decompress");
    let (ccd, rcd) = syms::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (cfd, rfd) = syms::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let (cf, _) = syms::<FnCompressFrame>("LZ4F_compressFrame");
    let (cfb, _) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let mut rng = Rng::new(SEED ^ 7);

    let src = mkdata(Shape::Textish, 30000, &mut rng);
    let mut p = Prefs::default();
    p.frame_info.content_checksum_flag = 1;
    let bound = unsafe { cfb(30000, &p) };
    let mut frame = vec![0u8; bound];
    let n = unsafe { cf(frame.as_mut_ptr(), bound, src.as_ptr(), 30000, &p) };
    frame.truncate(n);

    // Row 91: not enough src for the header
    for n in [0usize, 1, 2, 3, 4, 5, 6] {
        let mut a: *mut u8 = std::ptr::null_mut();
        let mut b: *mut u8 = std::ptr::null_mut();
        unsafe { ccd(&mut a, LZ4F_VERSION) };
        unsafe { rcd(&mut b, LZ4F_VERSION) };
        let mut cfi = FrameInfo::default();
        let mut rfi = FrameInfo::default();
        let mut cs = n;
        let mut rs = n;
        let x = unsafe { cgi(a, &mut cfi, frame.as_ptr(), &mut cs) };
        let y = unsafe { rgi(b, &mut rfi, frame.as_ptr(), &mut rs) };
        assert_eq!(x, y, "row91 getFrameInfo(n={n}): C={x:#x} R={y:#x}");
        assert_eq!(cs, rs, "row91 *srcSizePtr (n={n})");
        if is_err(x) {
            assert_eq!(code_of(x), 12, "row91 expected ERROR_frameHeader_incomplete (12)");
        }
        unsafe { cfd(a) };
        unsafe { rfd(b) };
    }

    // Row 90: called after the frame has started decoding
    let mut a: *mut u8 = std::ptr::null_mut();
    let mut b: *mut u8 = std::ptr::null_mut();
    unsafe { ccd(&mut a, LZ4F_VERSION) };
    unsafe { rcd(&mut b, LZ4F_VERSION) };
    let mut cbuf = vec![0u8; 40000];
    let mut rbuf = vec![0u8; 40000];
    let mut cds = 100usize;
    let mut rds = 100usize;
    let mut css = frame.len();
    let mut rss = frame.len();
    unsafe { cd(a, cbuf.as_mut_ptr(), &mut cds, frame.as_ptr(), &mut css, std::ptr::null()) };
    unsafe { rd(b, rbuf.as_mut_ptr(), &mut rds, frame.as_ptr(), &mut rss, std::ptr::null()) };
    let mut cfi = FrameInfo::default();
    let mut rfi = FrameInfo::default();
    let mut cs = frame.len();
    let mut rs = frame.len();
    let x = unsafe { cgi(a, &mut cfi, frame.as_ptr(), &mut cs) };
    let y = unsafe { rgi(b, &mut rfi, frame.as_ptr(), &mut rs) };
    assert_eq!(x, y, "row90 getFrameInfo mid-frame: C={x:#x} R={y:#x}");
    assert_eq!(cs, rs, "row90 *srcSizePtr");
    if !is_err(x) {
        assert_eq!(cfi, rfi);
    }
    unsafe { cfd(a) };
    unsafe { rfd(b) };
}

/// Rows 92-98: header-decode rejections, driven by mutating a valid header
/// field by field so each `RETURN_ERROR` branch is hit deliberately.
#[test]
fn e_frame_header_decode_rejections() {
    let (cd, rd) = syms::<FnDecompress>("LZ4F_decompress");
    let (ccd, rcd) = syms::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (cfd, rfd) = syms::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let (cf, _) = syms::<FnCompressFrame>("LZ4F_compressFrame");
    let (cfb, _) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let mut rng = Rng::new(SEED ^ 8);

    let src = mkdata(Shape::Textish, 3000, &mut rng);
    let mut p = Prefs::default();
    p.frame_info.content_checksum_flag = 1;
    p.frame_info.block_checksum_flag = 1;
    p.frame_info.block_size_id = 4;
    let bound = unsafe { cfb(3000, &p) };
    let mut base = vec![0u8; bound];
    let n = unsafe { cf(base.as_mut_ptr(), bound, src.as_ptr(), 3000, &p) };
    base.truncate(n);

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // Row 93: every single-bit corruption of the magic number
    for byte in 0..4usize {
        for bit in 0..8 {
            let mut f = base.clone();
            f[byte] ^= 1 << bit;
            cases.push((format!("magic byte{byte} bit{bit}"), f));
        }
    }
    // Rows 94-96: every FLG byte value (byte 4)
    for flg in 0u16..256 {
        let mut f = base.clone();
        f[4] = flg as u8;
        cases.push((format!("FLG={flg:#04x}"), f));
    }
    // Row 97 + row 96: every BD byte value (byte 5)
    for bd in 0u16..256 {
        let mut f = base.clone();
        f[5] = bd as u8;
        cases.push((format!("BD={bd:#04x}"), f));
    }
    // Row 98: every single-byte value of the header checksum
    let hc_index = {
        // find the header-checksum position: magic(4) + FLG + BD + optional
        // contentSize(8) + optional dictID(4) + HC(1)
        let flg = base[4];
        let mut i = 6usize;
        if (flg >> 3) & 1 == 1 {
            i += 8;
        }
        if flg & 1 == 1 {
            i += 4;
        }
        i
    };
    for v in 0u16..256 {
        let mut f = base.clone();
        f[hc_index] = v as u8;
        cases.push((format!("headerChecksum={v:#04x}"), f));
    }
    // Row 99: announced block size beyond the frame maximum
    for bs in [
        0x0000_0001u32,
        0x0001_0000,
        0x0001_0001,
        0x7FFF_FFFF,
        0x8001_0001,
        0xFFFF_FFFF,
        0x7FFF_FFFE,
    ] {
        let mut f = base.clone();
        f[hc_index + 1..hc_index + 5].copy_from_slice(&bs.to_le_bytes());
        cases.push((format!("blockSize={bs:#x}"), f));
    }
    // Rows 100-104: corrupt the body / checksums
    for i in [hc_index + 5, base.len() / 2, base.len() - 5, base.len() - 1] {
        for bit in 0..8 {
            let mut f = base.clone();
            f[i] ^= 1 << bit;
            cases.push((format!("body byte{i} bit{bit}"), f));
        }
    }
    // truncations at every length
    for t in 0..base.len().min(64) {
        cases.push((format!("trunc {t}"), base[..t].to_vec()));
    }

    for (name, frame) in cases.iter() {
        for &feed_all in &[true, false] {
            let mut a: *mut u8 = std::ptr::null_mut();
            let mut b: *mut u8 = std::ptr::null_mut();
            unsafe { ccd(&mut a, LZ4F_VERSION) };
            unsafe { rcd(&mut b, LZ4F_VERSION) };
            let mut cout: Vec<u8> = Vec::new();
            let mut rout: Vec<u8> = Vec::new();
            let mut off = 0usize;
            let mut guard = 0;
            loop {
                guard += 1;
                if guard > 200_000 {
                    break;
                }
                let want = if feed_all { frame.len() - off } else { 1.min(frame.len() - off) };
                let mut cbuf = vec![0u8; 8192];
                let mut rbuf = vec![0u8; 8192];
                let mut cds = cbuf.len();
                let mut rds = rbuf.len();
                let mut css = want;
                let mut rss = want;
                let sp = unsafe { frame.as_ptr().add(off) };
                let x = unsafe { cd(a, cbuf.as_mut_ptr(), &mut cds, sp, &mut css, std::ptr::null()) };
                let y = unsafe { rd(b, rbuf.as_mut_ptr(), &mut rds, sp, &mut rss, std::ptr::null()) };
                assert_eq!(
                    x, y,
                    "rows92-104 [{name}] feed_all={feed_all} off={off}: C={x:#x}({}) R={y:#x}({})",
                    code_of(x),
                    code_of(y)
                );
                assert_eq!(css, rss, "rows92-104 [{name}] *srcSizePtr off={off}");
                assert_eq!(cds, rds, "rows92-104 [{name}] *dstSizePtr off={off}");
                if is_err(x) {
                    break;
                }
                assert_eq!(&cbuf[..cds], &rbuf[..rds], "rows92-104 [{name}] payload");
                cout.extend_from_slice(&cbuf[..cds]);
                rout.extend_from_slice(&rbuf[..rds]);
                off += css;
                if x == 0 {
                    break;
                }
                if css == 0 && cds == 0 {
                    break;
                }
            }
            assert_eq!(cout, rout, "rows92-104 [{name}] accumulated payload");
            unsafe { cfd(a) };
            unsafe { rfd(b) };
        }
    }
}

/// Rows 92-104 (broad): randomized frame fuzzing, so no decode branch escapes.
#[test]
fn e_frame_decode_fuzz() {
    let (cd, rd) = syms::<FnDecompress>("LZ4F_decompress");
    let (ccd, rcd) = syms::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (cfd, rfd) = syms::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let (cf, _) = syms::<FnCompressFrame>("LZ4F_compressFrame");
    let (cfb, _) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let mut rng = Rng::new(SEED ^ 9);
    let matrix = prefs_matrix_small();

    for round in 0..30000 {
        let mut frame: Vec<u8> = if round % 5 == 0 {
            // pure garbage
            mkdata(Shape::Random, rng.range(0, 80), &mut rng)
        } else {
            let p = &matrix[rng.below(matrix.len())].1;
            let len = rng.range(0, 3000);
            let src = mkdata(ALL_SHAPES[rng.below(ALL_SHAPES.len())], len, &mut rng);
            let bound = unsafe { cfb(len, p) };
            let mut f = vec![0u8; bound];
            let n = unsafe { cf(f.as_mut_ptr(), bound, src.as_ptr(), len, p) };
            f.truncate(n);
            // corrupt / truncate it
            if !f.is_empty() {
                for _ in 0..rng.range(1, 4) {
                    let i = rng.below(f.len());
                    f[i] = rng.byte();
                }
                if rng.next_u32() % 3 == 0 {
                    let t = rng.below(f.len());
                    f.truncate(t);
                }
            }
            f
        };
        // occasionally give it a valid magic so header decode proceeds further
        if round % 7 == 0 && frame.len() >= 4 {
            frame[..4].copy_from_slice(&0x184D2204u32.to_le_bytes());
        }

        let mut a: *mut u8 = std::ptr::null_mut();
        let mut b: *mut u8 = std::ptr::null_mut();
        unsafe { ccd(&mut a, LZ4F_VERSION) };
        unsafe { rcd(&mut b, LZ4F_VERSION) };
        let mut off = 0usize;
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 5000 {
                break;
            }
            let dcap = [1usize, 17, 4096][guard % 3];
            let mut cbuf = vec![0u8; dcap];
            let mut rbuf = vec![0u8; dcap];
            let mut cds = dcap;
            let mut rds = dcap;
            let mut css = frame.len() - off;
            let mut rss = css;
            let sp = unsafe { frame.as_ptr().add(off) };
            let x = unsafe { cd(a, cbuf.as_mut_ptr(), &mut cds, sp, &mut css, std::ptr::null()) };
            let y = unsafe { rd(b, rbuf.as_mut_ptr(), &mut rds, sp, &mut rss, std::ptr::null()) };
            assert_eq!(
                x, y,
                "frame fuzz round={round} off={off}: C={x:#x}({}) R={y:#x}({}) frame={}",
                code_of(x),
                code_of(y),
                hexish(&frame)
            );
            assert_eq!(css, rss, "frame fuzz round={round} *srcSizePtr");
            assert_eq!(cds, rds, "frame fuzz round={round} *dstSizePtr");
            if is_err(x) || x == 0 {
                break;
            }
            assert_eq!(&cbuf[..cds], &rbuf[..rds], "frame fuzz round={round} payload");
            off += css;
            if css == 0 && cds == 0 {
                break;
            }
        }
        unsafe { cfd(a) };
        unsafe { rfd(b) };
    }
}

/// Rows 107, 149: `LZ4F_getBlockSize` with out-of-range IDs (including values
/// no enum variant covers — C enums accept any int across the FFI boundary).
#[test]
fn e_frame_get_block_size_invalid() {
    let (c, r) = syms::<FnGetBlockSize>("LZ4F_getBlockSize");
    for id in 0u32..=512 {
        let a = unsafe { c(id) };
        let b = unsafe { r(id) };
        assert_eq!(a, b, "rows107/149 getBlockSize({id}): C={a:#x} R={b:#x}");
        if !(id == 0 || (4..=7).contains(&id)) {
            assert!(is_err(a), "row107 C must reject blockSizeID {id}");
            assert_eq!(code_of(a), 2, "row107 expected ERROR_maxBlockSize_invalid (2)");
        }
    }
    for id in [
        1000u32, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFF9, 0xFFFF_FFFF, // == (unsigned)-1
    ] {
        let a = unsafe { c(id) };
        let b = unsafe { r(id) };
        assert_eq!(a, b, "row149 getBlockSize({id:#x}): C={a:#x} R={b:#x}");
        assert!(is_err(a));
    }
}

/// Rows 108-112: `LZ4F_isError`, `LZ4F_getErrorName`, `LZ4F_getErrorCode` over
/// the whole boundary region and a wide sweep of ordinary values.
#[test]
fn e_frame_error_helpers() {
    let (cie, rie) = syms::<FnIsError>("LZ4F_isError");
    let (cgn, rgn) = syms::<FnGetErrorName>("LZ4F_getErrorName");
    let (cgc, rgc) = syms::<FnGetErrorCode>("LZ4F_getErrorCode");

    let mut probes: Vec<usize> = Vec::new();
    // every negated ordinal 0..=40 (covers the whole enum plus past maxCode)
    for k in 0usize..=40 {
        probes.push(0usize.wrapping_sub(k));
    }
    // ordinary sizes and powers of two
    for v in [
        0usize, 1, 2, 19, 100, 65535, 65536, 1 << 20, 1 << 31, 1 << 40, 1 << 62,
        usize::MAX / 2,
        usize::MAX / 2 + 1,
    ] {
        probes.push(v);
    }
    let mut rng = Rng::new(SEED ^ 0xA);
    for _ in 0..2000 {
        probes.push(rng.next_u64() as usize);
    }

    for &v in probes.iter() {
        let a = unsafe { cie(v) };
        let b = unsafe { rie(v) };
        assert_eq!(a, b, "row108/109 isError({v:#x}): C={a} R={b}");
        // boundary: maxCode itself is NOT an error
        if v == 0usize.wrapping_sub(MAX_CODE) {
            assert_eq!(a, 0, "row108 -maxCode must NOT be an error");
        }
        if v == 0usize.wrapping_sub(MAX_CODE - 1) {
            assert_eq!(a, 1, "row109 -(maxCode-1) must be an error");
        }

        let ca = unsafe { CStr::from_ptr(cgn(v)) };
        let ra = unsafe { CStr::from_ptr(rgn(v)) };
        assert_eq!(ca, ra, "rows110/111 getErrorName({v:#x})");

        let a = unsafe { cgc(v) };
        let b = unsafe { rgc(v) };
        assert_eq!(a, b, "row112 getErrorCode({v:#x}): C={a} R={b}");
    }

    // Every valid enum ordinal must map to its literal spelling.
    let expected = [
        "OK_NoError",
        "ERROR_GENERIC",
        "ERROR_maxBlockSize_invalid",
        "ERROR_blockMode_invalid",
        "ERROR_parameter_invalid",
        "ERROR_compressionLevel_invalid",
        "ERROR_headerVersion_wrong",
        "ERROR_blockChecksum_invalid",
        "ERROR_reservedFlag_set",
        "ERROR_allocation_failed",
        "ERROR_srcSize_tooLarge",
        "ERROR_dstMaxSize_tooSmall",
        "ERROR_frameHeader_incomplete",
        "ERROR_frameType_unknown",
        "ERROR_frameSize_wrong",
        "ERROR_srcPtr_wrong",
        "ERROR_decompressionFailed",
        "ERROR_headerChecksum_invalid",
        "ERROR_contentChecksum_invalid",
        "ERROR_frameDecoding_alreadyStarted",
        "ERROR_compressionState_uninitialized",
        "ERROR_parameter_null",
        "ERROR_io_write",
        "ERROR_io_read",
    ];
    for (k, name) in expected.iter().enumerate().skip(1) {
        let v = 0usize.wrapping_sub(k);
        let ca = unsafe { CStr::from_ptr(cgn(v)) }.to_str().unwrap();
        let ra = unsafe { CStr::from_ptr(rgn(v)) }.to_str().unwrap();
        assert_eq!(ca, ra, "row111 getErrorName(-{k})");
        assert_eq!(ca, *name, "row111 getErrorName(-{k}) spelling");
        assert_eq!(unsafe { cgc(v) }, k as i32, "row112 getErrorCode(-{k})");
        assert_eq!(unsafe { rgc(v) }, k as i32, "row112 Rust getErrorCode(-{k})");
    }
    // a non-error result
    let ca = unsafe { CStr::from_ptr(cgn(19)) }.to_str().unwrap();
    let ra = unsafe { CStr::from_ptr(rgn(19)) }.to_str().unwrap();
    assert_eq!(ca, ra);
    assert_eq!(ca, "Unspecified error code", "row110");
}

/// Row 113-114: `free*` with NULL and the mid-frame dStage report.
#[test]
fn e_frame_free_null_and_dstage() {
    let (cfc, rfc) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cfd, rfd) = syms::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let (cfcd, rfcd) = syms::<FnFreeCDict>("LZ4F_freeCDict");
    assert_eq!(
        unsafe { cfc(std::ptr::null_mut()) },
        unsafe { rfc(std::ptr::null_mut()) },
        "row113 freeCompressionContext(NULL)"
    );
    assert_eq!(
        unsafe { cfd(std::ptr::null_mut()) },
        unsafe { rfd(std::ptr::null_mut()) },
        "row113 freeDecompressionContext(NULL)"
    );
    unsafe { cfcd(std::ptr::null_mut()) };
    unsafe { rfcd(std::ptr::null_mut()) };
}

// ============================================ lz4frame rows 150-153 ==========

/// Rows 150-153: OUT-OF-RANGE ENUM VALUES in `LZ4F_preferences_t`. A C enum
/// accepts any `int`, so these are real inputs the library must handle; the
/// Rust must reproduce whatever the C does, bit for bit.
#[test]
fn e_frame_out_of_range_enums() {
    let (cf, rf) = syms::<FnCompressFrame>("LZ4F_compressFrame");
    let (cfb, rfb) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let (cbnd, rbnd) = syms::<FnFrameBound>("LZ4F_compressBound");
    let (ccc, rcc) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cfc, rfc) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cbg, rbg) = syms::<FnCompressBegin>("LZ4F_compressBegin");
    let mut rng = Rng::new(SEED ^ 0xB);
    let src = mkdata(Shape::Textish, 5000, &mut rng);

    // Values with no valid enum variant, plus the valid ones as a control.
    let bsids: [i32; 14] = [-1000, -2, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, i32::MAX];
    let bmodes: [i32; 7] = [i32::MIN, -1, 0, 1, 2, 99, i32::MAX];
    let flags: [i32; 7] = [i32::MIN, -1, 0, 1, 2, 99, i32::MAX];
    let ftypes: [i32; 7] = [i32::MIN, -1, 0, 1, 2, 99, i32::MAX];

    for &bsid in bsids.iter() {
        // bound functions first: they must agree even for invalid IDs
        for &n in &[0usize, 1, 5000, 70000] {
            let mut p = Prefs::default();
            p.frame_info.block_size_id = bsid;
            let a = unsafe { cfb(n, &p) };
            let b = unsafe { rfb(n, &p) };
            assert_eq!(a, b, "row150 compressFrameBound(bsid={bsid}, n={n}): C={a:#x} R={b:#x}");
            let a = unsafe { cbnd(n, &p) };
            let b = unsafe { rbnd(n, &p) };
            assert_eq!(a, b, "row150 compressBound(bsid={bsid}, n={n}): C={a:#x} R={b:#x}");
        }
    }

    for &bsid in bsids.iter() {
        for &bmode in bmodes.iter() {
            for &cck in flags.iter() {
                for &bck in flags.iter() {
                    for &ftype in ftypes.iter() {
                        let mut p = Prefs::default();
                        p.frame_info.block_size_id = bsid;
                        p.frame_info.block_mode = bmode;
                        p.frame_info.content_checksum_flag = cck;
                        p.frame_info.block_checksum_flag = bck;
                        p.frame_info.frame_type = ftype;
                        let lbl = format!(
                            "rows150-153 bsid={bsid} bmode={bmode} cck={cck} bck={bck} ftype={ftype}"
                        );
                        // compressBegin: header emission with invalid enums
                        let mut a: *mut u8 = std::ptr::null_mut();
                        let mut b: *mut u8 = std::ptr::null_mut();
                        unsafe { ccc(&mut a, LZ4F_VERSION) };
                        unsafe { rcc(&mut b, LZ4F_VERSION) };
                        let mut ch = vec![0u8; 64];
                        let mut rh = vec![0u8; 64];
                        let x = unsafe { cbg(a, ch.as_mut_ptr(), 64, &p) };
                        let y = unsafe { rbg(b, rh.as_mut_ptr(), 64, &p) };
                        assert_eq!(x, y, "{lbl}: compressBegin C={x:#x} R={y:#x}");
                        if !is_err(x) {
                            assert_eq!(
                                &ch[..x], &rh[..y],
                                "{lbl}: header bytes differ\n C={}\n R={}",
                                hexish(&ch[..x]), hexish(&rh[..y])
                            );
                        }
                        unsafe { cfc(a) };
                        unsafe { rfc(b) };

                        // One-shot compressFrame. Its precondition is
                        // dstCapacity >= LZ4F_compressFrameBound(srcSize, prefs),
                        // and an invalid blockSizeID makes the C derive a huge
                        // block size (so a huge bound). Handing it anything smaller
                        // makes the C itself write out of bounds, so those
                        // combinations are only covered through the bound and
                        // compressBegin comparisons above.
                        let bound = unsafe { cfb(5000, &p) };
                        if is_err(bound) || bound > 64 * 1024 * 1024 {
                            continue;
                        }
                        let cap = bound.max(1);
                        let mut cd = vec![0u8; cap];
                        let mut rd = vec![0u8; cap];
                        let x = unsafe { cf(cd.as_mut_ptr(), cap, src.as_ptr(), 5000, &p) };
                        let y = unsafe { rf(rd.as_mut_ptr(), cap, src.as_ptr(), 5000, &p) };
                        assert_eq!(x, y, "{lbl}: compressFrame C={x:#x} R={y:#x}");
                        if !is_err(x) {
                            assert_eq!(
                                &cd[..x], &rd[..y],
                                "{lbl}: frame bytes differ\n C={}\n R={}",
                                hexish(&cd[..x]), hexish(&rd[..y])
                            );
                        }
                    }
                }
            }
        }
    }

    // reserved[] fields must be zero for forward compatibility; feed non-zero
    // values and confirm both libraries do the same thing.
    for junk in [1u32, 0xFFFF_FFFF] {
        let mut p = Prefs::default();
        p.reserved = [junk, junk, junk];
        p.frame_info.block_size_id = 4;
        let bound = unsafe { cfb(5000, &p) };
        assert_eq!(bound, unsafe { rfb(5000, &p) });
        let mut cd = vec![0u8; bound];
        let mut rd = vec![0u8; bound];
        let x = unsafe { cf(cd.as_mut_ptr(), bound, src.as_ptr(), 5000, &p) };
        let y = unsafe { rf(rd.as_mut_ptr(), bound, src.as_ptr(), 5000, &p) };
        assert_eq!(x, y, "reserved={junk:#x}: C={x:#x} R={y:#x}");
        if !is_err(x) {
            assert_eq!(&cd[..x], &rd[..y]);
        }
    }
    // and the decompressOptions reserved fields
    let (cdz, rdz) = syms::<FnDecompress>("LZ4F_decompress");
    let (ccd, rcd) = syms::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (cfd, rfd) = syms::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let mut p = Prefs::default();
    p.frame_info.content_checksum_flag = 1;
    let bound = unsafe { cfb(5000, &p) };
    let mut frame = vec![0u8; bound];
    let n = unsafe { cf(frame.as_mut_ptr(), bound, src.as_ptr(), 5000, &p) };
    frame.truncate(n);
    for junk in [0u32, 1, 2, 0xFFFF_FFFF] {
        let dopt = DecompressOptions {
            stable_dst: junk,
            skip_checksums: junk,
            reserved1: junk,
            reserved0: junk,
        };
        let mut a: *mut u8 = std::ptr::null_mut();
        let mut b: *mut u8 = std::ptr::null_mut();
        unsafe { ccd(&mut a, LZ4F_VERSION) };
        unsafe { rcd(&mut b, LZ4F_VERSION) };
        // stableDst is only meaningful with a persistent buffer; use one.
        let mut cbuf = vec![0u8; 5000 + 65536 + 16];
        let mut rbuf = vec![0u8; 5000 + 65536 + 16];
        let mut cds = cbuf.len();
        let mut rds = rbuf.len();
        let mut css = frame.len();
        let mut rss = frame.len();
        let x = unsafe { cdz(a, cbuf.as_mut_ptr(), &mut cds, frame.as_ptr(), &mut css, &dopt) };
        let y = unsafe { rdz(b, rbuf.as_mut_ptr(), &mut rds, frame.as_ptr(), &mut rss, &dopt) };
        assert_eq!(x, y, "dopt junk={junk:#x}: C={x:#x} R={y:#x}");
        assert_eq!(cds, rds);
        assert_eq!(css, rss);
        assert_eq!(&cbuf[..cds], &rbuf[..rds]);
        unsafe { cfd(a) };
        unsafe { rfd(b) };
    }
    // and compressOptions reserved
    let (cu, ru) = syms::<FnCompressUpdate>("LZ4F_compressUpdate");
    for junk in [0u32, 1, 0xFFFF_FFFF] {
        let copt = CompressOptions { stable_src: junk, reserved: [junk, junk, junk] };
        let mut a: *mut u8 = std::ptr::null_mut();
        let mut b: *mut u8 = std::ptr::null_mut();
        unsafe { ccc(&mut a, LZ4F_VERSION) };
        unsafe { rcc(&mut b, LZ4F_VERSION) };
        let mut ch = vec![0u8; 64];
        let mut rh = vec![0u8; 64];
        unsafe { cbg(a, ch.as_mut_ptr(), 64, &p) };
        unsafe { rbg(b, rh.as_mut_ptr(), 64, &p) };
        let cap = unsafe { cbnd(5000, &p) };
        let mut cd = vec![0u8; cap];
        let mut rd = vec![0u8; cap];
        let x = unsafe { cu(a, cd.as_mut_ptr(), cap, src.as_ptr(), 5000, &copt) };
        let y = unsafe { ru(b, rd.as_mut_ptr(), cap, src.as_ptr(), 5000, &copt) };
        assert_eq!(x, y, "copt junk={junk:#x}: C={x:#x} R={y:#x}");
        assert_eq!(&cd[..x], &rd[..y]);
        unsafe { cfc(a) };
        unsafe { rfc(b) };
    }
}

/// Row 155: the version / limit scalars.
#[test]
fn e_frame_scalars() {
    let (cv, rv) = syms::<FnGetVersion>("LZ4F_getVersion");
    assert_eq!(unsafe { cv() }, unsafe { rv() }, "row155 LZ4F_getVersion");
    assert_eq!(unsafe { cv() }, 100);
    let (cm, rm) = syms::<FnLevelMax>("LZ4F_compressionLevel_max");
    assert_eq!(unsafe { cm() }, unsafe { rm() }, "row155 LZ4F_compressionLevel_max");
    assert_eq!(unsafe { cm() }, 12);
    type FnI = unsafe extern "C" fn() -> i32;
    let (c, r) = syms::<FnI>("LZ4_versionNumber");
    assert_eq!(unsafe { c() }, unsafe { r() }, "row155 LZ4_versionNumber");
    assert_eq!(unsafe { c() }, 11000);
    type FnStr = unsafe extern "C" fn() -> *const i8;
    let (c, r) = syms::<FnStr>("LZ4_versionString");
    let a = unsafe { CStr::from_ptr(c()) };
    let b = unsafe { CStr::from_ptr(r()) };
    assert_eq!(a, b, "row155 LZ4_versionString");
    assert_eq!(a.to_str().unwrap(), "1.10.0");
}

// ================================================= lz4file rows 115-138 ======

type FnFopen = unsafe extern "C" fn(*const i8, *const i8) -> *mut u8;
type FnFclose = unsafe extern "C" fn(*mut u8) -> i32;
type FnWriteOpen = unsafe extern "C" fn(*mut *mut u8, *mut u8, *const Prefs) -> usize;
type FnWrite = unsafe extern "C" fn(*mut u8, *const u8, usize) -> usize;
type FnWriteClose = unsafe extern "C" fn(*mut u8) -> usize;
type FnReadOpen = unsafe extern "C" fn(*mut *mut u8, *mut u8) -> usize;
type FnRead = unsafe extern "C" fn(*mut u8, *mut u8, usize) -> usize;
type FnReadClose = unsafe extern "C" fn(*mut u8) -> usize;

fn libc_fns() -> (FnFopen, FnFclose) {
    let lib = unsafe { libloading::os::unix::Library::this() };
    let fopen: libloading::os::unix::Symbol<FnFopen> = unsafe { lib.get(b"fopen\0") }.unwrap();
    let fclose: libloading::os::unix::Symbol<FnFclose> = unsafe { lib.get(b"fclose\0") }.unwrap();
    (*fopen, *fclose)
}

fn tmp(tag: &str) -> String {
    format!("{}/lz4err_{}_{}.bin", std::env::temp_dir().display(), std::process::id(), tag)
}

/// Rows 115, 121, 124, 125, 131, 135: NULL-argument rejections.
#[test]
fn e_file_null_args() {
    let (fopen, fclose) = libc_fns();
    let (cwo, rwo) = syms::<FnWriteOpen>("LZ4F_writeOpen");
    let (cw, rw) = syms::<FnWrite>("LZ4F_write");
    let (cwc, rwc) = syms::<FnWriteClose>("LZ4F_writeClose");
    let (cro, rro) = syms::<FnReadOpen>("LZ4F_readOpen");
    let (crd, rrd) = syms::<FnRead>("LZ4F_read");
    let (crc, rrc) = syms::<FnReadClose>("LZ4F_readClose");

    let path = tmp("nullargs");
    std::fs::write(&path, b"garbage but long enough to fill 19 bytes for readOpen").unwrap();
    let cpath = CString::new(path.clone()).unwrap();
    let wmode = CString::new("wb").unwrap();
    let rmode = CString::new("rb").unwrap();

    // Row 125: writeOpen with fp == NULL
    let mut ctx: *mut u8 = std::ptr::null_mut();
    let a = unsafe { cwo(&mut ctx, std::ptr::null_mut(), std::ptr::null()) };
    let b = unsafe { rwo(&mut ctx, std::ptr::null_mut(), std::ptr::null()) };
    assert_eq!(a, b, "row125 writeOpen(fp=NULL): C={a:#x} R={b:#x}");
    assert_eq!(code_of(a), 21, "row125 expected ERROR_parameter_null (21)");
    // Row 125: writeOpen with lz4fWrite == NULL
    let fp = unsafe { fopen(cpath.as_ptr(), wmode.as_ptr()) };
    let a = unsafe { cwo(std::ptr::null_mut(), fp, std::ptr::null()) };
    let b = unsafe { rwo(std::ptr::null_mut(), fp, std::ptr::null()) };
    assert_eq!(a, b, "row125 writeOpen(ctx=NULL): C={a:#x} R={b:#x}");
    assert_eq!(code_of(a), 21);
    unsafe { fclose(fp) };

    // Row 131: write with NULL ctx / NULL buf
    let buf = [0u8; 16];
    let a = unsafe { cw(std::ptr::null_mut(), buf.as_ptr(), 16) };
    let b = unsafe { rw(std::ptr::null_mut(), buf.as_ptr(), 16) };
    assert_eq!(a, b, "row131 write(ctx=NULL): C={a:#x} R={b:#x}");
    assert_eq!(code_of(a), 21);
    // a real ctx with a NULL buffer
    let fp = unsafe { fopen(cpath.as_ptr(), wmode.as_ptr()) };
    let mut cctx: *mut u8 = std::ptr::null_mut();
    let mut rctx: *mut u8 = std::ptr::null_mut();
    unsafe { cwo(&mut cctx, fp, std::ptr::null()) };
    unsafe { rwo(&mut rctx, fp, std::ptr::null()) };
    for n in [0usize, 1, 16] {
        let a = unsafe { cw(cctx, std::ptr::null(), n) };
        let b = unsafe { rw(rctx, std::ptr::null(), n) };
        assert_eq!(a, b, "row131 write(buf=NULL, n={n}): C={a:#x} R={b:#x}");
        assert_eq!(code_of(a), 21);
    }
    unsafe { cwc(cctx) };
    unsafe { rwc(rctx) };
    unsafe { fclose(fp) };

    // Row 135: writeClose with NULL
    let a = unsafe { cwc(std::ptr::null_mut()) };
    let b = unsafe { rwc(std::ptr::null_mut()) };
    assert_eq!(a, b, "row135 writeClose(NULL): C={a:#x} R={b:#x}");
    assert_eq!(code_of(a), 21);

    // Row 115: readOpen with fp == NULL / ctx == NULL
    let mut ctx: *mut u8 = std::ptr::null_mut();
    let a = unsafe { cro(&mut ctx, std::ptr::null_mut()) };
    let b = unsafe { rro(&mut ctx, std::ptr::null_mut()) };
    assert_eq!(a, b, "row115 readOpen(fp=NULL): C={a:#x} R={b:#x}");
    assert_eq!(code_of(a), 21);
    let fp = unsafe { fopen(cpath.as_ptr(), rmode.as_ptr()) };
    let a = unsafe { cro(std::ptr::null_mut(), fp) };
    let b = unsafe { rro(std::ptr::null_mut(), fp) };
    assert_eq!(a, b, "row115 readOpen(ctx=NULL): C={a:#x} R={b:#x}");
    assert_eq!(code_of(a), 21);
    unsafe { fclose(fp) };

    // Row 121: read with NULL ctx / NULL buf
    let mut sbuf = [0u8; 16];
    let a = unsafe { crd(std::ptr::null_mut(), sbuf.as_mut_ptr(), 16) };
    let b = unsafe { rrd(std::ptr::null_mut(), sbuf.as_mut_ptr(), 16) };
    assert_eq!(a, b, "row121 read(ctx=NULL): C={a:#x} R={b:#x}");
    assert_eq!(code_of(a), 21);
    // Row 124: readClose with NULL
    let a = unsafe { crc(std::ptr::null_mut()) };
    let b = unsafe { rrc(std::ptr::null_mut()) };
    assert_eq!(a, b, "row124 readClose(NULL): C={a:#x} R={b:#x}");
    assert_eq!(code_of(a), 21);

    let _ = std::fs::remove_file(&path);
}

/// Rows 117-119: `LZ4F_readOpen` on files that are empty, too short, or carry an
/// invalid header.
#[test]
fn e_file_read_open_rejections() {
    let (fopen, fclose) = libc_fns();
    let (cro, rro) = syms::<FnReadOpen>("LZ4F_readOpen");
    let (crc, rrc) = syms::<FnReadClose>("LZ4F_readClose");
    let rmode = CString::new("rb").unwrap();
    let mut rng = Rng::new(SEED ^ 0xC);

    let path = tmp("readopen");
    let cpath = CString::new(path.clone()).unwrap();

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    // Row 117: shorter than LZ4F_HEADER_SIZE_MAX (19)
    for n in 0..24usize {
        cases.push((format!("len {n}"), mkdata(Shape::Random, n, &mut rng)));
    }
    // Rows 118-119: valid-length but invalid header content
    for magic in [0u32, 0x184D2203, 0x184D2205, 0xFFFF_FFFF, 0x184D2204] {
        for flg in [0u8, 0x01, 0x40, 0x44, 0x60, 0x64, 0x80, 0xFF] {
            for bd in [0u8, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0xFF] {
                let mut f = vec![0u8; 32];
                f[..4].copy_from_slice(&magic.to_le_bytes());
                f[4] = flg;
                f[5] = bd;
                cases.push((format!("magic={magic:#x} flg={flg:#02x} bd={bd:#02x}"), f));
            }
        }
    }

    for (name, bytes) in cases.iter() {
        std::fs::write(&path, bytes).unwrap();
        let mut results = Vec::new();
        for which in 0..2 {
            let (ro, rc) = if which == 0 { (cro, crc) } else { (rro, rrc) };
            let fp = unsafe { fopen(cpath.as_ptr(), rmode.as_ptr()) };
            let mut ctx: *mut u8 = std::ptr::null_mut();
            let a = unsafe { ro(&mut ctx, fp) };
            let closed = if is_err(a) { 0usize } else { unsafe { rc(ctx) } };
            unsafe { fclose(fp) };
            results.push((a, closed));
        }
        assert_eq!(
            results[0], results[1],
            "rows117-119 readOpen [{name}]: C={:#x}/{:#x} R={:#x}/{:#x}",
            results[0].0, results[0].1, results[1].0, results[1].1
        );
    }
    let _ = std::fs::remove_file(&path);
}

/// Rows 122, 123, 134: `LZ4F_read` / `LZ4F_write` with size 0 and on a corrupt
/// body (the error must propagate identically).
#[test]
fn e_file_read_write_body_errors() {
    let (fopen, fclose) = libc_fns();
    let (cwo, rwo) = syms::<FnWriteOpen>("LZ4F_writeOpen");
    let (cw, rw) = syms::<FnWrite>("LZ4F_write");
    let (cwc, rwc) = syms::<FnWriteClose>("LZ4F_writeClose");
    let (cro, rro) = syms::<FnReadOpen>("LZ4F_readOpen");
    let (crd, rrd) = syms::<FnRead>("LZ4F_read");
    let (crc, rrc) = syms::<FnReadClose>("LZ4F_readClose");
    let wmode = CString::new("wb").unwrap();
    let rmode = CString::new("rb").unwrap();
    let mut rng = Rng::new(SEED ^ 0xD);

    let path = tmp("body");
    let cpath = CString::new(path.clone()).unwrap();

    // build a real file with the C library
    let src = mkdata(Shape::Textish, 200_000, &mut rng);
    let mut p = Prefs::default();
    p.frame_info.content_checksum_flag = 1;
    p.frame_info.block_checksum_flag = 1;
    {
        let fp = unsafe { fopen(cpath.as_ptr(), wmode.as_ptr()) };
        let mut ctx: *mut u8 = std::ptr::null_mut();
        let a = unsafe { cwo(&mut ctx, fp, &p) };
        assert!(!is_err(a));
        // Row 134: size == 0 must return 0
        let z1 = unsafe { cw(ctx, src.as_ptr(), 0) };
        assert_eq!(z1, 0, "row134 write(0) must return 0");
        unsafe { cw(ctx, src.as_ptr(), src.len()) };
        unsafe { cwc(ctx) };
        unsafe { fclose(fp) };
    }
    let good = std::fs::read(&path).unwrap();

    // Rows 122: corrupt the body at many positions and compare the read chain
    for round in 0..600 {
        let mut bad = good.clone();
        let n = bad.len();
        for _ in 0..rng.range(1, 3) {
            let i = 20 + rng.below(n - 20);
            bad[i] = rng.byte();
        }
        if round % 4 == 0 {
            bad.truncate(20 + rng.below(n - 20));
        }
        std::fs::write(&path, &bad).unwrap();

        let mut results = Vec::new();
        for which in 0..2 {
            let (ro, rdf, rc) = if which == 0 { (cro, crd, crc) } else { (rro, rrd, rrc) };
            let fp = unsafe { fopen(cpath.as_ptr(), rmode.as_ptr()) };
            let mut ctx: *mut u8 = std::ptr::null_mut();
            let mut rets: Vec<usize> = Vec::new();
            let mut payload: Vec<u8> = Vec::new();
            let a = unsafe { ro(&mut ctx, fp) };
            rets.push(a);
            if !is_err(a) {
                // Row 123: size == 0 must return 0
                let mut scratch = vec![0u8; 4096];
                rets.push(unsafe { rdf(ctx, scratch.as_mut_ptr(), 0) });
                let mut guard = 0;
                loop {
                    guard += 1;
                    if guard > 100_000 {
                        break;
                    }
                    let k = unsafe { rdf(ctx, scratch.as_mut_ptr(), 4096) };
                    rets.push(k);
                    if is_err(k) || k == 0 {
                        break;
                    }
                    payload.extend_from_slice(&scratch[..k]);
                }
                rets.push(unsafe { rc(ctx) });
            }
            unsafe { fclose(fp) };
            results.push((rets, payload));
        }
        assert_eq!(
            results[0].0, results[1].0,
            "row122 read chain round={round}: C={:x?} R={:x?}",
            results[0].0, results[1].0
        );
        assert_eq!(results[0].1, results[1].1, "row122 payload round={round}");
    }
    let _ = std::fs::remove_file(&path);
}

/// Rows 127, 136-138: invalid `blockSizeID` on `writeOpen`, and the fact that
/// `LZ4F_writeClose` SWALLOWS a previously latched error.
#[test]
fn e_file_write_open_and_close_semantics() {
    let (fopen, fclose) = libc_fns();
    let (cwo, rwo) = syms::<FnWriteOpen>("LZ4F_writeOpen");
    let (cw, rw) = syms::<FnWrite>("LZ4F_write");
    let (cwc, rwc) = syms::<FnWriteClose>("LZ4F_writeClose");
    let wmode = CString::new("wb").unwrap();
    let mut rng = Rng::new(SEED ^ 0xE);
    let cpathname = tmp("wopen_c");
    let rpathname = tmp("wopen_r");

    // Row 127: every blockSizeID, valid and invalid
    for bsid in -3i32..12 {
        let mut p = Prefs::default();
        p.frame_info.block_size_id = bsid;
        let src = mkdata(Shape::Textish, 5000, &mut rng);
        let mut results = Vec::new();
        for (which, path) in [(0usize, &cpathname), (1usize, &rpathname)] {
            let (wo, w, wc) = if which == 0 { (cwo, cw, cwc) } else { (rwo, rw, rwc) };
            let cp = CString::new(path.clone()).unwrap();
            let fp = unsafe { fopen(cp.as_ptr(), wmode.as_ptr()) };
            let mut ctx: *mut u8 = std::ptr::null_mut();
            let mut rets = Vec::new();
            let a = unsafe { wo(&mut ctx, fp, &p) };
            rets.push(a);
            if !is_err(a) {
                rets.push(unsafe { w(ctx, src.as_ptr(), 5000) });
                rets.push(unsafe { wc(ctx) });
            }
            unsafe { fclose(fp) };
            results.push((rets, std::fs::read(path).unwrap_or_default()));
        }
        assert_eq!(
            results[0].0, results[1].0,
            "row127 writeOpen(bsid={bsid}): C={:x?} R={:x?}",
            results[0].0, results[1].0
        );
        assert_eq!(results[0].1, results[1].1, "row127 file bytes (bsid={bsid})");
        if !(bsid == 0 || (4..=7).contains(&bsid)) {
            assert!(is_err(results[0].0[0]), "row127 C must reject bsid={bsid}");
            assert_eq!(
                code_of(results[0].0[0]),
                2,
                "row127 expected ERROR_maxBlockSize_invalid (2)"
            );
        }
    }

    // Rows 133, 136, 138: a write error is latched, and writeClose returns
    // LZ4F_OK_NoError anyway. Provoke the write failure by opening the file
    // READ-ONLY so every fwrite is short.
    let rdonly = CString::new("rb").unwrap();
    std::fs::write(&cpathname, b"x").unwrap();
    std::fs::write(&rpathname, b"x").unwrap();
    let src = mkdata(Shape::Textish, 5000, &mut rng);
    let mut results = Vec::new();
    for (which, path) in [(0usize, &cpathname), (1usize, &rpathname)] {
        let (wo, w, wc) = if which == 0 { (cwo, cw, cwc) } else { (rwo, rw, rwc) };
        let cp = CString::new(path.clone()).unwrap();
        let fp = unsafe { fopen(cp.as_ptr(), rdonly.as_ptr()) };
        assert!(!fp.is_null());
        let mut ctx: *mut u8 = std::ptr::null_mut();
        let mut rets = Vec::new();
        let a = unsafe { wo(&mut ctx, fp, std::ptr::null()) };
        rets.push(a);
        if !is_err(a) {
            rets.push(unsafe { w(ctx, src.as_ptr(), 5000) });
            rets.push(unsafe { wc(ctx) });
        }
        unsafe { fclose(fp) };
        results.push(rets);
    }
    assert_eq!(
        results[0], results[1],
        "rows130/133/136 read-only fp: C={:x?} R={:x?}",
        results[0], results[1]
    );
    // The header fwrite fails first -> io_write from writeOpen (row 130).
    assert!(is_err(results[0][0]), "row130 C must report the short fwrite");
    assert_eq!(code_of(results[0][0]), 22, "row130 expected ERROR_io_write (22)");

    let _ = std::fs::remove_file(&cpathname);
    let _ = std::fs::remove_file(&rpathname);
}

// ================================================== xxhash rows 139-148 ======

/// Rows 139-140: `XXH*_update` with a NULL input pointer returns `XXH_ERROR`.
/// Rows 141-142: `reset` never errors. Rows 143-145: `freeState` / `createState`.
/// Rows 146-148.
#[test]
fn e_xxhash_error_surface() {
    type FnCreate = unsafe extern "C" fn() -> *mut u8;
    type FnFree = unsafe extern "C" fn(*mut u8) -> i32;
    type FnReset32 = unsafe extern "C" fn(*mut u8, u32) -> i32;
    type FnReset64 = unsafe extern "C" fn(*mut u8, u64) -> i32;
    type FnUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
    type FnDigest32 = unsafe extern "C" fn(*const u8) -> u32;
    type FnDigest64 = unsafe extern "C" fn(*const u8) -> u64;
    type FnXxh32 = unsafe extern "C" fn(*const u8, usize, u32) -> u32;
    type FnXxh64 = unsafe extern "C" fn(*const u8, usize, u64) -> u64;
    type FnVer = unsafe extern "C" fn() -> u32;

    // Row 148
    let (c, r) = syms::<FnVer>("LZ4_XXH_versionNumber");
    assert_eq!(unsafe { c() }, unsafe { r() }, "row148");
    assert_eq!(unsafe { c() }, 605, "row148 XXH_VERSION_NUMBER");

    // Rows 143-145
    for (create, free) in [
        ("LZ4_XXH32_createState", "LZ4_XXH32_freeState"),
        ("LZ4_XXH64_createState", "LZ4_XXH64_freeState"),
    ] {
        let (cc, rc) = syms::<FnCreate>(create);
        let (cf, rf) = syms::<FnFree>(free);
        let a = unsafe { cc() };
        let b = unsafe { rc() };
        assert_eq!(a.is_null(), b.is_null(), "row145 {create}");
        assert!(!a.is_null());
        assert_eq!(unsafe { cf(a) }, unsafe { rf(b) }, "{free}");
        // Rows 143-144: freeState(NULL) == XXH_OK (0)
        let a = unsafe { cf(std::ptr::null_mut()) };
        let b = unsafe { rf(std::ptr::null_mut()) };
        assert_eq!(a, b, "rows143-144 {free}(NULL): C={a} R={b}");
        assert_eq!(a, 0, "rows143-144 {free}(NULL) must return XXH_OK");
    }

    // Rows 139, 141: XXH32
    {
        let (cc, rc) = syms::<FnCreate>("LZ4_XXH32_createState");
        let (cf, rf) = syms::<FnFree>("LZ4_XXH32_freeState");
        let (cr, rr) = syms::<FnReset32>("LZ4_XXH32_reset");
        let (cu, ru) = syms::<FnUpdate>("LZ4_XXH32_update");
        let (cdg, rdg) = syms::<FnDigest32>("LZ4_XXH32_digest");
        let a = unsafe { cc() };
        let b = unsafe { rc() };
        // Row 141: reset always succeeds, for every seed
        for seed in [0u32, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF] {
            let x = unsafe { cr(a, seed) };
            let y = unsafe { rr(b, seed) };
            assert_eq!(x, y, "row141 XXH32_reset(seed={seed:#x}): C={x} R={y}");
            assert_eq!(x, 0, "row141 XXH32_reset must always return XXH_OK");
        }
        // Row 139: update with a NULL input, every length
        for len in [0usize, 1, 15, 16, 17, 1000, usize::MAX] {
            let x = unsafe { cu(a, std::ptr::null(), len) };
            let y = unsafe { ru(b, std::ptr::null(), len) };
            assert_eq!(x, y, "row139 XXH32_update(NULL, {len}): C={x} R={y}");
            assert_eq!(x, 1, "row139 must return XXH_ERROR (1)");
        }
        // the state must be untouched by the rejected updates
        let x = unsafe { cdg(a) };
        let y = unsafe { rdg(b) };
        assert_eq!(x, y, "row139 digest after rejected updates");
        unsafe { cf(a) };
        unsafe { rf(b) };
    }
    // Rows 140, 142: XXH64
    {
        let (cc, rc) = syms::<FnCreate>("LZ4_XXH64_createState");
        let (cf, rf) = syms::<FnFree>("LZ4_XXH64_freeState");
        let (cr, rr) = syms::<FnReset64>("LZ4_XXH64_reset");
        let (cu, ru) = syms::<FnUpdate>("LZ4_XXH64_update");
        let (cdg, rdg) = syms::<FnDigest64>("LZ4_XXH64_digest");
        let a = unsafe { cc() };
        let b = unsafe { rc() };
        for seed in [0u64, 1, 0xFFFF_FFFF, u64::MAX] {
            let x = unsafe { cr(a, seed) };
            let y = unsafe { rr(b, seed) };
            assert_eq!(x, y, "row142 XXH64_reset(seed={seed:#x}): C={x} R={y}");
            assert_eq!(x, 0, "row142 XXH64_reset must always return XXH_OK");
        }
        for len in [0usize, 1, 31, 32, 33, 1000, usize::MAX] {
            let x = unsafe { cu(a, std::ptr::null(), len) };
            let y = unsafe { ru(b, std::ptr::null(), len) };
            assert_eq!(x, y, "row140 XXH64_update(NULL, {len}): C={x} R={y}");
            assert_eq!(x, 1, "row140 must return XXH_ERROR (1)");
        }
        let x = unsafe { cdg(a) };
        let y = unsafe { rdg(b) };
        assert_eq!(x, y, "row140 digest after rejected updates");
        unsafe { cf(a) };
        unsafe { rf(b) };
    }

    // Row 146: one-shot with NULL input and len == 0 is NOT an error
    let (c32, r32) = syms::<FnXxh32>("LZ4_XXH32");
    let (c64, r64) = syms::<FnXxh64>("LZ4_XXH64");
    for seed in [0u32, 1, 0xFFFF_FFFF] {
        let a = unsafe { c32(std::ptr::null(), 0, seed) };
        let b = unsafe { r32(std::ptr::null(), 0, seed) };
        assert_eq!(a, b, "row146 XXH32(NULL, 0, {seed:#x}): C={a:#x} R={b:#x}");
    }
    for seed in [0u64, 1, u64::MAX] {
        let a = unsafe { c64(std::ptr::null(), 0, seed) };
        let b = unsafe { r64(std::ptr::null(), 0, seed) };
        assert_eq!(a, b, "row146 XXH64(NULL, 0, {seed:#x}): C={a:#x} R={b:#x}");
    }
}
