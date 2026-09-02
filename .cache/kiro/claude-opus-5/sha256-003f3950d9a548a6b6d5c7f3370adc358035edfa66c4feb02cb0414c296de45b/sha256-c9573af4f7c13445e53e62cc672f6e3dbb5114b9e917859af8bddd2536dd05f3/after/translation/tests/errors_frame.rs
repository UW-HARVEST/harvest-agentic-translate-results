//! Phase C — frame / file / xxhash error-path differential tests.
//! ERRORS.md rows 57-139.

mod common;
use common::*;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct FrameInfo {
    block_size_id: i32,
    block_mode: i32,
    content_checksum_flag: i32,
    frame_type: i32,
    content_size: u64,
    dict_id: u32,
    block_checksum_flag: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
struct Prefs {
    frame_info: FrameInfo,
    compression_level: i32,
    auto_flush: u32,
    favor_dec_speed: u32,
    reserved: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct COpts {
    stable_src: u32,
    reserved: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct DOpts {
    stable_dst: u32,
    skip_checksums: u32,
    reserved1: u32,
    reserved0: u32,
}

type FGetBlockSize = unsafe extern "C" fn(i32) -> usize;
type FCompressFrame = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const Prefs) -> usize;
type FFrameBound = unsafe extern "C" fn(usize, *const Prefs) -> usize;
type FBound = unsafe extern "C" fn(usize, *const Prefs) -> usize;
type FCreateCctx = unsafe extern "C" fn(*mut *mut u8, u32) -> usize;
type FFreeCctx = unsafe extern "C" fn(*mut u8) -> usize;
type FBegin = unsafe extern "C" fn(*mut u8, *mut u8, usize, *const Prefs) -> usize;
type FUpdate =
    unsafe extern "C" fn(*mut u8, *mut u8, usize, *const u8, usize, *const COpts) -> usize;
type FFlush = unsafe extern "C" fn(*mut u8, *mut u8, usize, *const COpts) -> usize;
type FBeginDict =
    unsafe extern "C" fn(*mut u8, *mut u8, usize, *const u8, usize, *const Prefs) -> usize;
type FCreateDctx = unsafe extern "C" fn(*mut *mut u8, u32) -> usize;
type FFreeDctx = unsafe extern "C" fn(*mut u8) -> usize;
type FDecompress =
    unsafe extern "C" fn(*mut u8, *mut u8, *mut usize, *const u8, *mut usize, *const DOpts) -> usize;
type FGetFrameInfo = unsafe extern "C" fn(*mut u8, *mut FrameInfo, *const u8, *mut usize) -> usize;
type FHeaderSize = unsafe extern "C" fn(*const u8, usize) -> usize;
type FIsError = unsafe extern "C" fn(usize) -> u32;
type FErrName = unsafe extern "C" fn(usize) -> *const std::os::raw::c_char;
type FErrCode = unsafe extern "C" fn(usize) -> i32;
type FCreateCDict = unsafe extern "C" fn(*const u8, usize) -> *mut u8;
type FFreeCDict = unsafe extern "C" fn(*mut u8);

const V: u32 = 100;

fn c_is_err(code: usize) -> bool {
    let (c, _) = sym::<FIsError>("LZ4F_isError");
    unsafe { c(code) != 0 }
}

/// Rows 57-60, 133: `LZ4F_getBlockSize` over every in-range, out-of-range and
/// out-of-enum blockSizeID, including values with no valid variant.
#[test]
fn rows57_60_get_block_size() {
    let (c, r) = sym::<FGetBlockSize>("LZ4F_getBlockSize");
    let mut ids: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -8,
        -1,
        0,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        99,
        255,
        256,
        65536,
        i32::MAX - 1,
        i32::MAX,
    ];
    let mut rng = Rng::new(0xF57);
    for _ in 0..3000 {
        ids.push(rng.next_u32() as i32);
    }
    for id in ids {
        let (a, b) = unsafe { (c(id), r(id)) };
        let ctx = format!("row57-60 getBlockSize({id})");
        eq(&ctx, a, b);
        // spot-check the documented mapping
        match id {
            0 | 4 => eq(&format!("{ctx} value"), a, 65536),
            5 => eq(&format!("{ctx} value"), a, 262_144),
            6 => eq(&format!("{ctx} value"), a, 1_048_576),
            7 => eq(&format!("{ctx} value"), a, 4_194_304),
            _ => assert!(c_is_err(a), "{ctx}: expected an error, got {a}"),
        }
    }
}

/// Rows 61-62: `LZ4F_compressFrame` with too-small dst and invalid blockSizeID.
#[test]
fn rows61_62_compress_frame_rejections() {
    let (c, r) = sym::<FCompressFrame>("LZ4F_compressFrame");
    let (cfb, rfb) = sym::<FFrameBound>("LZ4F_compressFrameBound");
    let mut rng = Rng::new(0xF61);

    // row 61: dstCapacity < compressFrameBound
    for &shape in &SHAPES {
        for len in [0usize, 1, 100, 65536, 70_000] {
            let src = make_data(&mut rng, len, shape);
            let p = Prefs::default();
            let (ba, bb) = unsafe { (cfb(len, &p), rfb(len, &p)) };
            eq(&format!("row61 frameBound len={len}"), ba, bb);
            for cap in [0usize, 1, 4, 18, 19, ba / 2, ba.saturating_sub(1)] {
                let mut cd = vec![0x11u8; cap + 32];
                let mut rd = vec![0x11u8; cap + 32];
                let (a, b) = unsafe {
                    (
                        c(cd.as_mut_ptr(), cap, src.as_ptr(), len, &p),
                        r(rd.as_mut_ptr(), cap, src.as_ptr(), len, &p),
                    )
                };
                let ctx = format!("row61 compressFrame shape={shape:?} len={len} cap={cap}");
                eq(&ctx, a, b);
                eq_bytes(&format!("{ctx} buf"), &cd, &rd);
                if cap < ba {
                    assert!(c_is_err(a), "{ctx}: expected dstMaxSize_tooSmall, got {a}");
                }
            }
        }
    }

    // row 62: invalid blockSizeID propagated out of LZ4F_getBlockSize
    for bad in [1i32, 2, 3, 8, 9, 99, -1, i32::MIN, i32::MAX] {
        let p = Prefs {
            frame_info: FrameInfo {
                block_size_id: bad,
                ..Default::default()
            },
            ..Default::default()
        };
        let len = 4096usize;
        let src = make_data(&mut rng, len, Shape::Text);
        let (ba, bb) = unsafe { (cfb(len, &p), rfb(len, &p)) };
        eq(&format!("row62 frameBound bsid={bad}"), ba, bb);
        let mut cd = vec![0u8; 1 << 20];
        let mut rd = vec![0u8; 1 << 20];
        let (a, b) = unsafe {
            (
                c(cd.as_mut_ptr(), cd.len(), src.as_ptr(), len, &p),
                r(rd.as_mut_ptr(), rd.len(), src.as_ptr(), len, &p),
            )
        };
        let ctx = format!("row62 compressFrame bsid={bad}");
        eq(&ctx, a, b);
        eq_bytes(&format!("{ctx} buf"), &cd, &rd);
        // NOTE (verified in the C): `LZ4F_compressFrame` does NOT propagate
        // maxBlockSize_invalid. `LZ4F_optimalBSID` (lz4frame.c:359) normalises
        // any ID above max64KB down to a valid one when srcSize is small, and
        // for IDs 1..3 the error code returned by `LZ4F_getBlockSize` is used
        // as a (huge) block size, so the frame is produced successfully. The
        // differential requirement is that both implementations agree, which is
        // asserted above. ERRORS.md row 62 records this.
    }
}

/// Rows 63, 75: `LZ4F_create*Context` with a NULL out-parameter.
#[test]
fn rows63_75_create_context_null() {
    let (cc, rc) = sym::<FCreateCctx>("LZ4F_createCompressionContext");
    let (cd, rd) = sym::<FCreateDctx>("LZ4F_createDecompressionContext");
    for ver in [0u32, 1, V, 999, u32::MAX] {
        let (a, b) = unsafe { (cc(std::ptr::null_mut(), ver), rc(std::ptr::null_mut(), ver)) };
        let ctx = format!("row63 createCompressionContext(NULL,{ver})");
        eq(&ctx, a, b);
        assert!(c_is_err(a), "{ctx}: expected parameter_null, got {a}");

        let (a, b) = unsafe { (cd(std::ptr::null_mut(), ver), rd(std::ptr::null_mut(), ver)) };
        let ctx = format!("row75 createDecompressionContext(NULL,{ver})");
        eq(&ctx, a, b);
        assert!(c_is_err(a), "{ctx}: expected parameter_null, got {a}");
    }
    // rows 99-100: free on NULL must not crash and must agree
    let (cf, rf) = sym::<FFreeCctx>("LZ4F_freeCompressionContext");
    let (a, b) = unsafe { (cf(std::ptr::null_mut()), rf(std::ptr::null_mut())) };
    eq("row99 freeCompressionContext(NULL)", a, b);
    let (cf, rf) = sym::<FFreeDctx>("LZ4F_freeDecompressionContext");
    let (a, b) = unsafe { (cf(std::ptr::null_mut()), rf(std::ptr::null_mut())) };
    eq("row100 freeDecompressionContext(NULL)", a, b);
    // rows 101-102: CDict with empty/NULL dict, and freeCDict(NULL)
    let (ccd, rcd) = sym::<FCreateCDict>("LZ4F_createCDict");
    let (cfd, rfd) = sym::<FFreeCDict>("LZ4F_freeCDict");
    let mut rng = Rng::new(0xF63);
    let d = make_data(&mut rng, 16, Shape::Text);
    for n in [0usize, 1, 16] {
        let (a, b) = unsafe { (ccd(d.as_ptr(), n), rcd(d.as_ptr(), n)) };
        eq(&format!("row101 createCDict(n={n}) null-ness"), a.is_null(), b.is_null());
        unsafe {
            cfd(a);
            rfd(b);
        }
    }
    unsafe {
        cfd(std::ptr::null_mut());
        rfd(std::ptr::null_mut());
    }
}

/// Rows 65-74, 103: low-level compression-stage and capacity rejections.
#[test]
fn rows65_74_streaming_rejections() {
    let (ccc, rcc) = sym::<FCreateCctx>("LZ4F_createCompressionContext");
    let (cfc, rfc) = sym::<FFreeCctx>("LZ4F_freeCompressionContext");
    let (cbg, rbg) = sym::<FBegin>("LZ4F_compressBegin");
    let (cup, rup) = sym::<FUpdate>("LZ4F_compressUpdate");
    let (cuu, ruu) = sym::<FUpdate>("LZ4F_uncompressedUpdate");
    let (cfl, rfl) = sym::<FFlush>("LZ4F_flush");
    let (cen, ren) = sym::<FFlush>("LZ4F_compressEnd");
    let (cbd, rbd) = sym::<FBeginDict>("LZ4F_compressBegin_usingDict");
    let (cbn, rbn) = sym::<FBound>("LZ4F_compressBound");
    let mut rng = Rng::new(0xF65);

    // row 65: compressBegin with dstCapacity < LZ4F_HEADER_SIZE_MAX
    for cap in [0usize, 1, 5, 7, 15, 18] {
        for &bsid in &[0i32, 4, 7] {
            let p = Prefs {
                frame_info: FrameInfo {
                    block_size_id: bsid,
                    content_checksum_flag: 1,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut got = Vec::new();
            for (cr, fr, bg) in [(&ccc, &cfc, &cbg), (&rcc, &rfc, &rbg)] {
                unsafe {
                    let mut ctx: *mut u8 = std::ptr::null_mut();
                    cr(&mut ctx, V);
                    let mut d = vec![0x22u8; cap + 32];
                    let n = bg(ctx, d.as_mut_ptr(), cap, &p);
                    fr(ctx);
                    got.push((n, d));
                }
            }
            let c = format!("row65 compressBegin cap={cap} bsid={bsid}");
            eq(&c, got[0].0, got[1].0);
            eq_bytes(&format!("{c} buf"), &got[0].1, &got[1].1);
            assert!(c_is_err(got[0].0), "{c}: expected dstMaxSize_tooSmall");
        }
    }

    // rows 67, 69, 70: calling update/uncompressedUpdate/flush before begin
    let src = make_data(&mut rng, 4096, Shape::Text);
    for which in 0..3 {
        let mut got = Vec::new();
        for (cr, fr, up, uu, fl) in [
            (&ccc, &cfc, &cup, &cuu, &cfl),
            (&rcc, &rfc, &rup, &ruu, &rfl),
        ] {
            unsafe {
                let mut ctx: *mut u8 = std::ptr::null_mut();
                cr(&mut ctx, V);
                let mut d = vec![0x33u8; 1 << 17];
                let o = COpts::default();
                let n = match which {
                    0 => up(ctx, d.as_mut_ptr(), d.len(), src.as_ptr(), src.len(), &o),
                    1 => uu(ctx, d.as_mut_ptr(), d.len(), src.as_ptr(), src.len(), &o),
                    _ => fl(ctx, d.as_mut_ptr(), d.len(), &o),
                };
                fr(ctx);
                got.push(n);
            }
        }
        let c = format!("row67/69/70 pre-begin call which={which}");
        eq(&c, got[0], got[1]);
        // compressUpdate / uncompressedUpdate DO reject an uninitialised state,
        // but LZ4F_flush returns 0 first via `if (tmpInSize == 0) return 0;`
        // (lz4frame.c:1167), which precedes the cStage check. ERRORS.md row 70.
        if which != 2 {
            assert!(
                c_is_err(got[0]),
                "{c}: expected compressionState_uninitialized, got {}",
                got[0]
            );
        } else {
            eq(&format!("{c} flush returns 0"), got[0], 0);
        }
    }

    // row 68: compressUpdate with dstCapacity < compressBound(srcSize)
    for &bsid in &[4i32, 5] {
        for len in [1usize, 100, 65536, 70_000] {
            let s = make_data(&mut rng, len, Shape::Random);
            let p = Prefs {
                frame_info: FrameInfo {
                    block_size_id: bsid,
                    ..Default::default()
                },
                ..Default::default()
            };
            let need = unsafe { cbn(len, &p) };
            for cap in [0usize, 1, 4, need / 2, need.saturating_sub(1)] {
                let mut got = Vec::new();
                for (cr, fr, bg, up) in
                    [(&ccc, &cfc, &cbg, &cup), (&rcc, &rfc, &rbg, &rup)]
                {
                    unsafe {
                        let mut ctx: *mut u8 = std::ptr::null_mut();
                        cr(&mut ctx, V);
                        let mut h = vec![0u8; 64];
                        bg(ctx, h.as_mut_ptr(), h.len(), &p);
                        let mut d = vec![0x44u8; cap + 32];
                        let o = COpts::default();
                        let n = up(ctx, d.as_mut_ptr(), cap, s.as_ptr(), len, &o);
                        fr(ctx);
                        got.push((n, d));
                    }
                }
                let c = format!("row68 compressUpdate bsid={bsid} len={len} cap={cap}");
                eq(&c, got[0].0, got[1].0);
                eq_bytes(&format!("{c} buf"), &got[0].1, &got[1].1);
            }
        }
    }

    // row 71: flush with dstCapacity below the buffered requirement
    for &bsid in &[4i32, 5] {
        for feed in [1usize, 100, 5000] {
            let s = make_data(&mut rng, feed, Shape::Random);
            let p = Prefs {
                frame_info: FrameInfo {
                    block_size_id: bsid,
                    block_checksum_flag: 1,
                    ..Default::default()
                },
                ..Default::default()
            };
            for cap in [0usize, 1, 2, 4, 8] {
                let mut got = Vec::new();
                for (cr, fr, bg, up, fl) in [
                    (&ccc, &cfc, &cbg, &cup, &cfl),
                    (&rcc, &rfc, &rbg, &rup, &rfl),
                ] {
                    unsafe {
                        let mut ctx: *mut u8 = std::ptr::null_mut();
                        cr(&mut ctx, V);
                        let mut h = vec![0u8; 64];
                        bg(ctx, h.as_mut_ptr(), h.len(), &p);
                        let big = cbn(feed, &p);
                        let mut d = vec![0u8; big + 64];
                        let o = COpts::default();
                        up(ctx, d.as_mut_ptr(), big, s.as_ptr(), feed, &o);
                        let mut f = vec![0x55u8; cap + 32];
                        let n = fl(ctx, f.as_mut_ptr(), cap, &o);
                        fr(ctx);
                        got.push((n, f));
                    }
                }
                let c = format!("row71 flush bsid={bsid} feed={feed} cap={cap}");
                eq(&c, got[0].0, got[1].0);
                eq_bytes(&format!("{c} buf"), &got[0].1, &got[1].1);
            }
        }
    }

    // rows 72-74: compressEnd capacity guards and frameSize_wrong
    for &ccs in &[0i32, 1] {
        for cap in [0usize, 1, 3, 4, 7, 8] {
            let p = Prefs {
                frame_info: FrameInfo {
                    block_size_id: 4,
                    content_checksum_flag: ccs,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut got = Vec::new();
            for (cr, fr, bg, en) in [(&ccc, &cfc, &cbg, &cen), (&rcc, &rfc, &rbg, &ren)] {
                unsafe {
                    let mut ctx: *mut u8 = std::ptr::null_mut();
                    cr(&mut ctx, V);
                    let mut h = vec![0u8; 64];
                    bg(ctx, h.as_mut_ptr(), h.len(), &p);
                    let mut d = vec![0x66u8; cap + 32];
                    let o = COpts::default();
                    let n = en(ctx, d.as_mut_ptr(), cap, &o);
                    fr(ctx);
                    got.push((n, d));
                }
            }
            let c = format!("row72/73 compressEnd ccs={ccs} cap={cap}");
            eq(&c, got[0].0, got[1].0);
            eq_bytes(&format!("{c} buf"), &got[0].1, &got[1].1);
            let need = if ccs == 1 { 8 } else { 4 };
            if cap < need {
                assert!(c_is_err(got[0].0), "{c}: expected dstMaxSize_tooSmall");
            }
        }
    }

    // row 74: declared contentSize != bytes actually fed -> frameSize_wrong
    for (declared, fed) in [
        (100u64, 0usize),
        (100, 50),
        (100, 99),
        (100, 101),
        (100, 200),
        (0, 10),
        (5000, 4999),
    ] {
        let s = make_data(&mut rng, fed, Shape::Text);
        let p = Prefs {
            frame_info: FrameInfo {
                block_size_id: 4,
                content_size: declared,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut got = Vec::new();
        for (cr, fr, bg, up, en) in [
            (&ccc, &cfc, &cbg, &cup, &cen),
            (&rcc, &rfc, &rbg, &rup, &ren),
        ] {
            unsafe {
                let mut ctx: *mut u8 = std::ptr::null_mut();
                cr(&mut ctx, V);
                let mut h = vec![0u8; 64];
                let hn = bg(ctx, h.as_mut_ptr(), h.len(), &p);
                let big = cbn(fed, &p) + 64;
                let mut d = vec![0u8; big + 64];
                let o = COpts::default();
                let un = up(ctx, d.as_mut_ptr(), big, s.as_ptr(), fed, &o);
                let mut e = vec![0u8; 256];
                let en_ = en(ctx, e.as_mut_ptr(), e.len(), &o);
                fr(ctx);
                got.push((hn, un, en_));
            }
        }
        let c = format!("row74 contentSize declared={declared} fed={fed}");
        eq(&format!("{c} begin"), got[0].0, got[1].0);
        eq(&format!("{c} update"), got[0].1, got[1].1);
        eq(&format!("{c} end"), got[0].2, got[1].2);
        // `contentSize == 0` means "unknown" (no size field is written and no
        // size check is performed), so only a NON-ZERO declared size that
        // disagrees with the bytes actually fed triggers frameSize_wrong.
        if declared != 0 && declared != fed as u64 {
            assert!(
                c_is_err(got[0].2),
                "{c}: expected frameSize_wrong from compressEnd, got {}",
                got[0].2
            );
        }
    }

    // row 66: compressBegin_usingDict with dictSize > INT_MAX is not
    // constructible (it would need a >2GiB allocation); exercise the adjacent
    // in-range boundary values instead so the guard's accept-side is covered.
    for n in [0usize, 1, 4, 65536] {
        let d = make_data(&mut rng, n.max(1), Shape::Text);
        let p = Prefs::default();
        let mut got = Vec::new();
        for (cr, fr, bd) in [(&ccc, &cfc, &cbd), (&rcc, &rfc, &rbd)] {
            unsafe {
                let mut ctx: *mut u8 = std::ptr::null_mut();
                cr(&mut ctx, V);
                let mut h = vec![0x77u8; 64];
                let x = bd(ctx, h.as_mut_ptr(), h.len(), d.as_ptr(), n, &p);
                fr(ctx);
                got.push((x, h));
            }
        }
        let c = format!("row66 compressBegin_usingDict dictSize={n}");
        eq(&c, got[0].0, got[1].0);
        eq_bytes(&format!("{c} hdr"), &got[0].1, &got[1].1);
    }
}

/// Rows 77-84, 86-87: header parsing rejections via `headerSize` /
/// `getFrameInfo` / `decompress`.
#[test]
fn rows77_87_header_rejections() {
    let (chs, rhs) = sym::<FHeaderSize>("LZ4F_headerSize");
    let (ccd, rcd) = sym::<FCreateDctx>("LZ4F_createDecompressionContext");
    let (cfd, rfd) = sym::<FFreeDctx>("LZ4F_freeDecompressionContext");
    let (cgi, rgi) = sym::<FGetFrameInfo>("LZ4F_getFrameInfo");
    let mut rng = Rng::new(0xF77);

    // row 77: srcSize < minFHSize (5)
    let good: [u8; 19] = [
        0x04, 0x22, 0x4D, 0x18, 0x60, 0x40, 0x82, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    for k in 0..5usize {
        let (a, b) = unsafe { (chs(good.as_ptr(), k), rhs(good.as_ptr(), k)) };
        let c = format!("row77 headerSize(srcSize={k})");
        eq(&c, a, b);
        assert!(c_is_err(a), "{c}: expected frameHeader_incomplete, got {a}");
    }

    // row 78: bad magic number (and the skippable-frame range)
    for magic in [
        0u32,
        1,
        0x184D2203,
        0x184D2205,
        0x184D2A4F,
        0x184D2A50,
        0x184D2A5F,
        0x184D2A60,
        0xFFFF_FFFF,
    ] {
        let mut h = good;
        h[..4].copy_from_slice(&magic.to_le_bytes());
        for k in [5usize, 6, 7, 8, 19] {
            let (a, b) = unsafe { (chs(h.as_ptr(), k), rhs(h.as_ptr(), k)) };
            eq(&format!("row78 headerSize magic={magic:#x} k={k}"), a, b);
        }
    }

    // rows 79-84, 86-87: drive getFrameInfo with systematically mutated headers
    // (FLG reserved bit, version, BD reserved bits, blockSizeID, checksum byte)
    let mut cases: Vec<Vec<u8>> = Vec::new();
    // build a valid header first, then mutate it
    for flg in 0u16..256 {
        for bd in [0x40u8, 0x50, 0x60, 0x70, 0x00, 0x10, 0x20, 0x30, 0x80, 0x71] {
            let mut h = vec![0x04u8, 0x22, 0x4D, 0x18, flg as u8, bd];
            // header checksum over bytes [4..6]
            h.push(0);
            cases.push(h);
        }
    }
    // random header fuzzing
    for _ in 0..4000 {
        let n = rng.range(5, 24);
        let mut h = vec![0x04u8, 0x22, 0x4D, 0x18];
        for _ in 4..n {
            h.push(rng.byte());
        }
        cases.push(h);
    }
    // and fully random bytes
    for _ in 0..2000 {
        let n = rng.range(1, 30);
        cases.push(make_data(&mut rng, n, Shape::Random));
    }

    for (i, h) in cases.iter().enumerate() {
        // headerSize
        let (a, b) = unsafe { (chs(h.as_ptr(), h.len()), rhs(h.as_ptr(), h.len())) };
        eq(&format!("row79-84 headerSize case={i} {:02x?}", &h[..h.len().min(8)]), a, b);

        // getFrameInfo
        let mut got = Vec::new();
        for (cr, fr, gi) in [(&ccd, &cfd, &cgi), (&rcd, &rfd, &rgi)] {
            unsafe {
                let mut ctx: *mut u8 = std::ptr::null_mut();
                cr(&mut ctx, V);
                let mut fi = FrameInfo::default();
                let mut ssz = h.len();
                let n = gi(ctx, &mut fi, h.as_ptr(), &mut ssz);
                fr(ctx);
                got.push((n, fi, ssz));
            }
        }
        let c = format!("row79-87 getFrameInfo case={i} {:02x?}", &h[..h.len().min(8)]);
        eq(&format!("{c} ret"), got[0].0, got[1].0);
        eq(&format!("{c} info"), got[0].1, got[1].1);
        eq(&format!("{c} consumed"), got[0].2, got[1].2);
    }

    // row 85: getFrameInfo with src == NULL
    for ssz0 in [0usize, 1, 5, 19] {
        let mut got = Vec::new();
        for (cr, fr, gi) in [(&ccd, &cfd, &cgi), (&rcd, &rfd, &rgi)] {
            unsafe {
                let mut ctx: *mut u8 = std::ptr::null_mut();
                cr(&mut ctx, V);
                let mut fi = FrameInfo::default();
                let mut ssz = ssz0;
                let n = gi(ctx, &mut fi, std::ptr::null(), &mut ssz);
                fr(ctx);
                got.push((n, ssz));
            }
        }
        let c = format!("row85 getFrameInfo(src=NULL, srcSize={ssz0})");
        eq(&format!("{c} ret"), got[0].0, got[1].0);
        eq(&format!("{c} consumed"), got[0].1, got[1].1);
        assert!(c_is_err(got[0].0), "{c}: expected srcPtr_wrong, got {}", got[0].0);
    }

    // row 89: getFrameInfo with srcSize == 0 at the storeFrameHeader stage
    let mut got = Vec::new();
    for (cr, fr, gi) in [(&ccd, &cfd, &cgi), (&rcd, &rfd, &rgi)] {
        unsafe {
            let mut ctx: *mut u8 = std::ptr::null_mut();
            cr(&mut ctx, V);
            let mut fi = FrameInfo::default();
            // feed 1 byte first so the dctx is mid-header, then ask with 0
            let mut s1 = 1usize;
            let r1 = gi(ctx, &mut fi, good.as_ptr(), &mut s1);
            let mut s2 = 0usize;
            let r2 = gi(ctx, &mut fi, good.as_ptr(), &mut s2);
            fr(ctx);
            got.push((r1, s1, r2, s2));
        }
    }
    eq("row89 getFrameInfo srcSize=0", got[0], got[1]);
}

/// Rows 88, 90-95: decode-stage rejections on corrupted frames.
#[test]
fn rows88_95_decode_rejections() {
    let (ccf, _) = sym::<FCompressFrame>("LZ4F_compressFrame");
    let (cfb, _) = sym::<FFrameBound>("LZ4F_compressFrameBound");
    let (ccd, rcd) = sym::<FCreateDctx>("LZ4F_createDecompressionContext");
    let (cfd, rfd) = sym::<FFreeDctx>("LZ4F_freeDecompressionContext");
    let (cdc, rdc) = sym::<FDecompress>("LZ4F_decompress");
    let (cgi, rgi) = sym::<FGetFrameInfo>("LZ4F_getFrameInfo");
    let mut rng = Rng::new(0xF88);

    let configs = [
        (4i32, 0i32, 0i32, 0i32),
        (4, 1, 1, 1),
        (5, 0, 1, 0),
        (7, 1, 0, 1),
    ];

    for (bsid, bmode, ccs, bcs) in configs {
        for &shape in &SHAPES {
            for len in [1usize, 100, 5000, 70_000] {
                let src = make_data(&mut rng, len, shape);
                let p = Prefs {
                    frame_info: FrameInfo {
                        block_size_id: bsid,
                        block_mode: bmode,
                        content_checksum_flag: ccs,
                        content_size: 0,
                        dict_id: 0,
                        frame_type: 0,
                        block_checksum_flag: bcs,
                    },
                    ..Default::default()
                };
                let cap = unsafe { cfb(len, &p) };
                let mut fr0 = vec![0u8; cap + 32];
                let n = unsafe { ccf(fr0.as_mut_ptr(), cap, src.as_ptr(), len, &p) };
                if c_is_err(n) {
                    continue;
                }
                fr0.truncate(n);

                // rows 90-94: single-byte corruptions anywhere in the frame hit
                // the blockSize, blockChecksum, decompressionFailed,
                // frameSize_wrong and contentChecksum branches.
                for _ in 0..60 {
                    let mut bad = fr0.clone();
                    let i = rng.below(bad.len());
                    bad[i] ^= 1 << rng.below(8);
                    let mut got = Vec::new();
                    for (cr, fr, dc) in [(&ccd, &cfd, &cdc), (&rcd, &rfd, &rdc)] {
                        unsafe {
                            let mut ctx: *mut u8 = std::ptr::null_mut();
                            cr(&mut ctx, V);
                            let mut out = vec![0x99u8; len + 8192];
                            let mut hints = Vec::new();
                            let mut t = 0usize;
                            let mut soff = 0usize;
                            for _ in 0..64 {
                                let mut dsz = out.len() - t;
                                let mut ssz = bad.len() - soff;
                                if dsz == 0 {
                                    break;
                                }
                                let h = dc(
                                    ctx,
                                    out.as_mut_ptr().add(t),
                                    &mut dsz,
                                    bad.as_ptr().add(soff),
                                    &mut ssz,
                                    std::ptr::null(),
                                );
                                hints.push(h);
                                t += dsz;
                                soff += ssz;
                                if c_is_err(h) || h == 0 || (dsz == 0 && ssz == 0) {
                                    break;
                                }
                            }
                            fr(ctx);
                            out.truncate(t);
                            got.push((hints, out));
                        }
                    }
                    let c = format!(
                        "row90-94 corrupt bsid={bsid} bmode={bmode} ccs={ccs} bcs={bcs} len={len} i={i}"
                    );
                    eq(&format!("{c} hints"), &got[0].0, &got[1].0);
                    eq_bytes(&c, &got[0].1, &got[1].1);
                }

                // row 95: truncated frame -> nonzero hint, no error
                for cut in [1usize, fr0.len() / 3, fr0.len() / 2, fr0.len() - 1] {
                    if cut == 0 || cut >= fr0.len() {
                        continue;
                    }
                    let mut got = Vec::new();
                    for (cr, fr, dc) in [(&ccd, &cfd, &cdc), (&rcd, &rfd, &rdc)] {
                        unsafe {
                            let mut ctx: *mut u8 = std::ptr::null_mut();
                            cr(&mut ctx, V);
                            let mut out = vec![0u8; len + 8192];
                            let mut dsz = out.len();
                            let mut ssz = cut;
                            let h = dc(
                                ctx,
                                out.as_mut_ptr(),
                                &mut dsz,
                                fr0.as_ptr(),
                                &mut ssz,
                                std::ptr::null(),
                            );
                            fr(ctx);
                            out.truncate(dsz);
                            got.push((h, dsz, ssz, out));
                        }
                    }
                    let c = format!("row95 truncated len={len} cut={cut}");
                    eq(&format!("{c} hint"), got[0].0, got[1].0);
                    eq(&format!("{c} dst"), got[0].1, got[1].1);
                    eq(&format!("{c} src"), got[0].2, got[1].2);
                    eq_bytes(&c, &got[0].3, &got[1].3);
                }

                // row 88: getFrameInfo after decoding has already started
                let mut got = Vec::new();
                for (cr, fr, dc, gi) in
                    [(&ccd, &cfd, &cdc, &cgi), (&rcd, &rfd, &rdc, &rgi)]
                {
                    unsafe {
                        let mut ctx: *mut u8 = std::ptr::null_mut();
                        cr(&mut ctx, V);
                        let mut out = vec![0u8; len + 8192];
                        let mut dsz = out.len();
                        let mut ssz = fr0.len();
                        let h = dc(
                            ctx,
                            out.as_mut_ptr(),
                            &mut dsz,
                            fr0.as_ptr(),
                            &mut ssz,
                            std::ptr::null(),
                        );
                        let mut fi = FrameInfo::default();
                        let mut s2 = fr0.len();
                        let g = gi(ctx, &mut fi, fr0.as_ptr(), &mut s2);
                        fr(ctx);
                        got.push((h, g, fi, s2));
                    }
                }
                let c = format!("row88 getFrameInfo mid-decode bsid={bsid} len={len}");
                eq(&format!("{c} hint"), got[0].0, got[1].0);
                eq(&format!("{c} ret"), got[0].1, got[1].1);
                eq(&format!("{c} info"), got[0].2, got[1].2);
            }
        }
    }
}

/// Rows 96-98, 138-139: error-helper functions over the whole `size_t` range,
/// including codes with no valid enum variant.
#[test]
fn rows96_98_138_139_error_helpers() {
    let (cie, rie) = sym::<FIsError>("LZ4F_isError");
    let (cen, ren) = sym::<FErrName>("LZ4F_getErrorName");
    let (cec, rec) = sym::<FErrCode>("LZ4F_getErrorCode");

    let mut codes: Vec<usize> = Vec::new();
    // every enum code and well past maxCode
    for i in 0..64usize {
        codes.push(0usize.wrapping_sub(i));
    }
    for v in [
        0usize,
        1,
        2,
        3,
        100,
        1 << 16,
        1 << 32,
        usize::MAX / 4,
        usize::MAX / 2,
        usize::MAX - 10_000,
        usize::MAX - 1000,
        usize::MAX,
    ] {
        codes.push(v);
    }
    let mut rng = Rng::new(0xF96);
    for _ in 0..5000 {
        codes.push(rng.next_u64() as usize);
    }

    for &c in &codes {
        eq(&format!("row96 isError({c})"), unsafe { cie(c) }, unsafe {
            rie(c)
        });
        eq(&format!("row98/139 getErrorCode({c})"), unsafe { cec(c) }, unsafe {
            rec(c)
        });
        unsafe {
            let a = std::ffi::CStr::from_ptr(cen(c));
            let b = std::ffi::CStr::from_ptr(ren(c));
            eq(&format!("row97/138 getErrorName({c})"), a, b);
        }
    }
}

/// Rows 134-137: out-of-range enum values in `LZ4F_preferences_t` crossing the
/// FFI boundary. C enums accept any `int`, so these are real inputs.
#[test]
fn rows134_137_out_of_range_enums() {
    let (cc, rc) = sym::<FCompressFrame>("LZ4F_compressFrame");
    let (cfb, rfb) = sym::<FFrameBound>("LZ4F_compressFrameBound");
    let mut rng = Rng::new(0xF134);
    let weird: [i32; 9] = [-1, -2, i32::MIN, 2, 3, 7, 99, 255, i32::MAX];

    for &shape in &SHAPES {
        let len = rng.range(1, 40_000);
        let src = make_data(&mut rng, len, shape);
        for field in 0..4 {
            for &w in &weird {
                let mut fi = FrameInfo {
                    block_size_id: 4,
                    ..Default::default()
                };
                match field {
                    0 => fi.block_mode = w,             // row 134
                    1 => fi.content_checksum_flag = w,  // row 135
                    2 => fi.block_checksum_flag = w,    // row 136
                    _ => fi.frame_type = w,             // row 137
                }
                let p = Prefs {
                    frame_info: fi,
                    ..Default::default()
                };
                let (ba, bb) = unsafe { (cfb(len, &p), rfb(len, &p)) };
                eq(&format!("row134-137 frameBound field={field} w={w}"), ba, bb);
                // An out-of-range flag makes the C's
                // `BHSize + contentChecksumFlag * BFSize` term wrap, so the
                // bound can legitimately exceed 4 GiB. That value IS the
                // observable behaviour and is compared above; we just cannot
                // allocate it, so skip the call itself in that case.
                if ba > (1usize << 26) {
                    continue;
                }
                let cap = if c_is_err(ba) { 1 << 20 } else { ba };
                let mut cd = vec![0xC7u8; cap + 64];
                let mut rd = vec![0xC7u8; cap + 64];
                let (a, b) = unsafe {
                    (
                        cc(cd.as_mut_ptr(), cap, src.as_ptr(), len, &p),
                        rc(rd.as_mut_ptr(), cap, src.as_ptr(), len, &p),
                    )
                };
                let c = format!(
                    "row134-137 compressFrame field={field} w={w} shape={shape:?} len={len}"
                );
                eq(&c, a, b);
                eq_bytes(&format!("{c} buf"), &cd, &rd);
            }
        }

        // reserved[] fields must be zero for forward compatibility; check that
        // whatever the C does with non-zero values, the Rust does too.
        for r0 in [1u32, 0xFFFF_FFFF] {
            let p = Prefs {
                frame_info: FrameInfo {
                    block_size_id: 4,
                    ..Default::default()
                },
                reserved: [r0, r0, r0],
                ..Default::default()
            };
            let (ba, bb) = unsafe { (cfb(len, &p), rfb(len, &p)) };
            eq(&format!("reserved frameBound r={r0}"), ba, bb);
            if ba > (1usize << 26) {
                continue;
            }
            let cap = if c_is_err(ba) { 1 << 20 } else { ba };
            let mut cd = vec![0u8; cap + 64];
            let mut rd = vec![0u8; cap + 64];
            let (a, b) = unsafe {
                (
                    cc(cd.as_mut_ptr(), cap, src.as_ptr(), len, &p),
                    rc(rd.as_mut_ptr(), cap, src.as_ptr(), len, &p),
                )
            };
            eq(&format!("reserved compressFrame r={r0}"), a, b);
            eq_bytes(&format!("reserved compressFrame r={r0} buf"), &cd, &rd);
        }

        // and an out-of-range compressionLevel / autoFlush / favorDecSpeed
        for lvl in [i32::MIN, -9999, 13, 100, i32::MAX] {
            for af in [2u32, 0xFFFF_FFFF] {
                let p = Prefs {
                    frame_info: FrameInfo {
                        block_size_id: 4,
                        ..Default::default()
                    },
                    compression_level: lvl,
                    auto_flush: af,
                    favor_dec_speed: af,
                    reserved: [0; 3],
                };
                let ba = unsafe { cfb(len, &p) };
                if ba > (1usize << 26) {
                    continue;
                }
                let cap = if c_is_err(ba) { 1 << 20 } else { ba };
                let mut cd = vec![0u8; cap + 64];
                let mut rd = vec![0u8; cap + 64];
                let (a, b) = unsafe {
                    (
                        cc(cd.as_mut_ptr(), cap, src.as_ptr(), len, &p),
                        rc(rd.as_mut_ptr(), cap, src.as_ptr(), len, &p),
                    )
                };
                let c = format!("prefs lvl={lvl} af/fds={af} len={len}");
                eq(&c, a, b);
                eq_bytes(&format!("{c} buf"), &cd, &rd);
            }
        }
    }
}

/// Rows 124-132: xxhash NULL / zero-length rejections and canonical decode.
#[test]
fn rows124_132_xxhash_errors() {
    type F32One = unsafe extern "C" fn(*const u8, usize, u32) -> u32;
    type F64One = unsafe extern "C" fn(*const u8, usize, u64) -> u64;
    type FCreate = unsafe extern "C" fn() -> *mut u8;
    type FFreeSt = unsafe extern "C" fn(*mut u8) -> i32;
    type FReset32 = unsafe extern "C" fn(*mut u8, u32) -> i32;
    type FReset64 = unsafe extern "C" fn(*mut u8, u64) -> i32;
    type FUpd = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
    type FDig32 = unsafe extern "C" fn(*mut u8) -> u32;
    type FDig64 = unsafe extern "C" fn(*mut u8) -> u64;
    type FCanon32 = unsafe extern "C" fn(*mut u8, u32);
    type FCanon64 = unsafe extern "C" fn(*mut u8, u64);
    type FFromCanon32 = unsafe extern "C" fn(*const u8) -> u32;
    type FFromCanon64 = unsafe extern "C" fn(*const u8) -> u64;

    let (cc32, rc32) = sym::<FCreate>("LZ4_XXH32_createState");
    let (cf32, rf32) = sym::<FFreeSt>("LZ4_XXH32_freeState");
    let (cr32, rr32) = sym::<FReset32>("LZ4_XXH32_reset");
    let (cu32, ru32) = sym::<FUpd>("LZ4_XXH32_update");
    let (cd32, rd32) = sym::<FDig32>("LZ4_XXH32_digest");
    let (cc64, rc64) = sym::<FCreate>("LZ4_XXH64_createState");
    let (cf64, rf64) = sym::<FFreeSt>("LZ4_XXH64_freeState");
    let (cr64, rr64) = sym::<FReset64>("LZ4_XXH64_reset");
    let (cu64, ru64) = sym::<FUpd>("LZ4_XXH64_update");
    let (cd64, rd64) = sym::<FDig64>("LZ4_XXH64_digest");

    // rows 125-128: update with a NULL input pointer
    for len in [0usize, 1, 16, 4096] {
        let mut got = Vec::new();
        for (cr, fr, rs, up, dg) in [
            (&cc32, &cf32, &cr32, &cu32, &cd32),
            (&rc32, &rf32, &rr32, &ru32, &rd32),
        ] {
            unsafe {
                let s = cr();
                let r0 = rs(s, 0);
                let r1 = up(s, std::ptr::null(), len);
                let d = dg(s);
                let r2 = fr(s);
                got.push((r0, r1, d, r2));
            }
        }
        eq(&format!("row125/126 XXH32_update(NULL,{len})"), got[0], got[1]);

        let mut got = Vec::new();
        for (cr, fr, rs, up, dg) in [
            (&cc64, &cf64, &cr64, &cu64, &cd64),
            (&rc64, &rf64, &rr64, &ru64, &rd64),
        ] {
            unsafe {
                let s = cr();
                let r0 = rs(s, 0);
                let r1 = up(s, std::ptr::null(), len);
                let d = dg(s);
                let r2 = fr(s);
                got.push((r0, r1, d, r2));
            }
        }
        eq(&format!("row127/128 XXH64_update(NULL,{len})"), got[0], got[1]);
    }

    // row 129: one-shot with len == 0 (valid pointer)
    let (c1, r1) = sym::<F32One>("LZ4_XXH32");
    let (c2, r2) = sym::<F64One>("LZ4_XXH64");
    let mut rng = Rng::new(0xF124);
    let buf = make_data(&mut rng, 16, Shape::Text);
    for seed in [0u32, 1, 0xFFFF_FFFF] {
        eq(
            &format!("row129 XXH32(len=0,seed={seed})"),
            unsafe { c1(buf.as_ptr(), 0, seed) },
            unsafe { r1(buf.as_ptr(), 0, seed) },
        );
    }
    for seed in [0u64, 1, u64::MAX] {
        eq(
            &format!("row129 XXH64(len=0,seed={seed})"),
            unsafe { c2(buf.as_ptr(), 0, seed) },
            unsafe { r2(buf.as_ptr(), 0, seed) },
        );
    }

    // row 130: freeState(NULL)
    eq("row130 XXH32_freeState(NULL)", unsafe {
        cf32(std::ptr::null_mut())
    }, unsafe { rf32(std::ptr::null_mut()) });
    eq("row130 XXH64_freeState(NULL)", unsafe {
        cf64(std::ptr::null_mut())
    }, unsafe { rf64(std::ptr::null_mut()) });

    // rows 131-132: canonical decode accepts ANY bytes; round-trip must agree
    let (ccan32, rcan32) = sym::<FCanon32>("LZ4_XXH32_canonicalFromHash");
    let (cfc32, rfc32) = sym::<FFromCanon32>("LZ4_XXH32_hashFromCanonical");
    let (ccan64, rcan64) = sym::<FCanon64>("LZ4_XXH64_canonicalFromHash");
    let (cfc64, rfc64) = sym::<FFromCanon64>("LZ4_XXH64_hashFromCanonical");
    for _ in 0..4000 {
        let h32 = rng.next_u32();
        let mut a = [0u8; 4];
        let mut b = [0u8; 4];
        unsafe {
            ccan32(a.as_mut_ptr(), h32);
            rcan32(b.as_mut_ptr(), h32);
        }
        eq_bytes(&format!("row131 canonicalFromHash32({h32})"), &a, &b);
        eq(
            &format!("row131 hashFromCanonical32({h32})"),
            unsafe { cfc32(a.as_ptr()) },
            unsafe { rfc32(b.as_ptr()) },
        );

        let h64 = rng.next_u64();
        let mut a = [0u8; 8];
        let mut b = [0u8; 8];
        unsafe {
            ccan64(a.as_mut_ptr(), h64);
            rcan64(b.as_mut_ptr(), h64);
        }
        eq_bytes(&format!("row132 canonicalFromHash64({h64})"), &a, &b);
        eq(
            &format!("row132 hashFromCanonical64({h64})"),
            unsafe { cfc64(a.as_ptr()) },
            unsafe { rfc64(b.as_ptr()) },
        );

        // arbitrary canonical bytes (no validation in the C)
        let raw = make_data(&mut rng, 8, Shape::Random);
        eq(
            "row131 hashFromCanonical32(random)",
            unsafe { cfc32(raw.as_ptr()) },
            unsafe { rfc32(raw.as_ptr()) },
        );
        eq(
            "row132 hashFromCanonical64(random)",
            unsafe { cfc64(raw.as_ptr()) },
            unsafe { rfc64(raw.as_ptr()) },
        );
    }
}
