//! Phase B: frame information queries, skippable frames and bound functions.
//!
//! Every test drives BOTH the C `libzstd.so` (ground truth) and the Rust
//! `libzstd.so` and asserts identical return values / byte-identical output.
//!
//! CONFIGS 117-122, 161; ERRORS 115, 117, 164, 186-209, 259.
#![allow(non_snake_case, dead_code)]
mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

// ------------------------------------------------------------ fn types -----

type FnNew = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> usize;
type FnCompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_int) -> usize;
type FnDecompress = unsafe extern "C" fn(*mut u8, usize, *const u8, usize) -> usize;
type FnDecompressDCtx =
    unsafe extern "C" fn(*mut c_void, *mut u8, usize, *const u8, usize) -> usize;
type FnU64Query = unsafe extern "C" fn(*const u8, usize) -> c_ulonglong;
type FnSizeQuery = unsafe extern "C" fn(*const u8, usize) -> usize;
type FnUintQuery = unsafe extern "C" fn(*const u8, usize) -> c_uint;
type FnBound = unsafe extern "C" fn(usize) -> usize;
type FnDecodingBufMin = unsafe extern "C" fn(c_ulonglong, c_ulonglong) -> usize;
type FnWriteSkip = unsafe extern "C" fn(*mut u8, usize, *const u8, usize, c_uint) -> usize;
type FnReadSkip = unsafe extern "C" fn(*mut u8, usize, *mut c_uint, *const u8, usize) -> usize;
type FnGetFrameHeader = unsafe extern "C" fn(*mut FrameHeader, *const u8, usize) -> usize;
type FnGetFrameHeaderAdv =
    unsafe extern "C" fn(*mut FrameHeader, *const u8, usize, c_int) -> usize;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> usize;
type FnCompressStream2 =
    unsafe extern "C" fn(*mut c_void, *mut OutBuffer, *mut InBuffer, c_int) -> usize;

// ------------------------------------------------------------- structs -----

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameHeader {
    pub frameContentSize: c_ulonglong,
    pub windowSize: c_ulonglong,
    pub blockSizeMax: c_uint,
    pub frameType: c_int,
    pub headerSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
    pub _r1: c_uint,
    pub _r2: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct InBuffer {
    pub src: *const u8,
    pub size: usize,
    pub pos: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OutBuffer {
    pub dst: *mut u8,
    pub size: usize,
    pub pos: usize,
}

const C_CHECKSUMFLAG: c_int = 201;
const C_CONTENTSIZEFLAG: c_int = 200;
const C_COMPRESSIONLEVEL: c_int = 100;
const C_WINDOWLOG: c_int = 101;

const ZSTD_E_CONTINUE: c_int = 0;
const ZSTD_E_END: c_int = 2;

const ZSTD_MAGICNUMBER: u32 = 0xFD2FB528;
const ZSTD_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;

// ------------------------------------------------------------- helpers -----

/// ZSTD_compress in both libs; assert identical bytes; return the frame.
fn frame_with_fcs(src: &[u8], level: c_int) -> Vec<u8> {
    let (cc, rc) = unsafe { pair::<FnCompress>("ZSTD_compress") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let cap = unsafe { cb(src.len()) } + 64;
    let mut co = vec![0xAAu8; cap];
    let mut ro = vec![0x55u8; cap];
    let a = unsafe { cc(co.as_mut_ptr(), cap, src.as_ptr(), src.len(), level) };
    let b = unsafe { rc(ro.as_mut_ptr(), cap, src.as_ptr(), src.len(), level) };
    assert_eq!(a, b, "ZSTD_compress return (len={} lvl={level})", src.len());
    assert!(!is_error(a), "ZSTD_compress failed: {}", err_code(a));
    assert_bytes_eq("frame_with_fcs", &co[..a], &ro[..a]);
    co.truncate(a);
    co
}

/// Streamed compression with *unknown* pledged size => no FCS field in header.
fn frame_no_fcs(src: &[u8], level: c_int, checksum: c_int) -> Vec<u8> {
    let (cn, rn) = unsafe { pair::<FnNew>("ZSTD_createCCtx") };
    let (cf, rf) = unsafe { pair::<FnFree>("ZSTD_freeCCtx") };
    let (cs, rs) = unsafe { pair::<FnSetParam>("ZSTD_CCtx_setParameter") };
    let (cst, rst) = unsafe { pair::<FnCompressStream2>("ZSTD_compressStream2") };
    let (cb, _) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let cap = unsafe { cb(src.len()) } + 1024;

    let mut out: Vec<Vec<u8>> = Vec::new();
    for which in 0..2 {
        let ctx = if which == 0 { unsafe { cn() } } else { unsafe { rn() } };
        assert!(!ctx.is_null());
        let set = if which == 0 { &cs } else { &rs };
        let stream = if which == 0 { &cst } else { &rst };
        assert!(!is_error(unsafe { set(ctx, C_COMPRESSIONLEVEL, level) }));
        assert!(!is_error(unsafe { set(ctx, C_CHECKSUMFLAG, checksum) }));
        let mut buf = vec![0u8; cap];
        let mut ob = OutBuffer { dst: buf.as_mut_ptr(), size: cap, pos: 0 };
        // First an explicit ZSTD_e_continue call so the frame is initialized with
        // pledgedSrcSize == unknown (a single ZSTD_e_end call would turn the
        // available input size into the pledged size and emit an FCS field).
        {
            let mut ib = InBuffer { src: src.as_ptr(), size: 0, pos: 0 };
            let r = unsafe { stream(ctx, &mut ob, &mut ib, ZSTD_E_CONTINUE) };
            assert!(!is_error(r), "compressStream2(continue) err {}", err_code(r));
        }
        let mut fed = 0usize;
        while fed < src.len() {
            let take = (src.len() - fed).min(1024);
            let mut ib = InBuffer { src: unsafe { src.as_ptr().add(fed) }, size: take, pos: 0 };
            while ib.pos < ib.size {
                let r = unsafe { stream(ctx, &mut ob, &mut ib, ZSTD_E_CONTINUE) };
                assert!(!is_error(r), "compressStream2(continue) err {}", err_code(r));
                assert!(ob.pos < ob.size, "output buffer exhausted");
            }
            fed += take;
        }
        loop {
            let mut ib = InBuffer { src: src.as_ptr(), size: 0, pos: 0 };
            let r = unsafe { stream(ctx, &mut ob, &mut ib, ZSTD_E_END) };
            assert!(!is_error(r), "compressStream2(end) err {}", err_code(r));
            if r == 0 {
                break;
            }
            assert!(ob.pos < ob.size, "output buffer exhausted");
        }
        buf.truncate(ob.pos);
        out.push(buf);
        unsafe {
            if which == 0 {
                cf(ctx);
            } else {
                rf(ctx);
            }
        }
    }
    assert_bytes_eq("frame_no_fcs", &out[0], &out[1]);
    out.into_iter().next().unwrap()
}

/// ZSTD_writeSkippableFrame in both libs; assert identical; return the frame.
fn skippable(payload: &[u8], variant: c_uint) -> Vec<u8> {
    let (cw, rw) = unsafe { pair::<FnWriteSkip>("ZSTD_writeSkippableFrame") };
    let cap = payload.len() + 8;
    let mut co = vec![0xAAu8; cap];
    let mut ro = vec![0x55u8; cap];
    let a = unsafe { cw(co.as_mut_ptr(), cap, payload.as_ptr(), payload.len(), variant) };
    let b = unsafe { rw(ro.as_mut_ptr(), cap, payload.as_ptr(), payload.len(), variant) };
    assert_eq!(a, b, "writeSkippableFrame return");
    assert!(!is_error(a), "writeSkippableFrame err {}", err_code(a));
    assert_bytes_eq("skippable bytes", &co[..a], &ro[..a]);
    co.truncate(a);
    co
}

/// Run all five frame-size queries in both libs and require identical results.
fn size_queries(buf: &[u8], ctx: &str) {
    let (c_fds, r_fds) = unsafe { pair::<FnU64Query>("ZSTD_findDecompressedSize") };
    let (c_db, r_db) = unsafe { pair::<FnU64Query>("ZSTD_decompressBound") };
    let (c_ffcs, r_ffcs) = unsafe { pair::<FnSizeQuery>("ZSTD_findFrameCompressedSize") };
    let (c_gfcs, r_gfcs) = unsafe { pair::<FnU64Query>("ZSTD_getFrameContentSize") };
    let (c_gds, r_gds) = unsafe { pair::<FnU64Query>("ZSTD_getDecompressedSize") };
    let p = buf.as_ptr();
    let n = buf.len();
    unsafe {
        assert_eq!(c_fds(p, n), r_fds(p, n), "{ctx}: ZSTD_findDecompressedSize");
        assert_eq!(c_db(p, n), r_db(p, n), "{ctx}: ZSTD_decompressBound");
        assert_eq!(
            c_ffcs(p, n),
            r_ffcs(p, n),
            "{ctx}: ZSTD_findFrameCompressedSize (C err={})",
            err_code(c_ffcs(p, n))
        );
        assert_eq!(c_gfcs(p, n), r_gfcs(p, n), "{ctx}: ZSTD_getFrameContentSize");
        assert_eq!(c_gds(p, n), r_gds(p, n), "{ctx}: ZSTD_getDecompressedSize");
    }
}

/// Every prefix length in 0..min(len,40) plus len-1 and len/2.
fn truncations(buf: &[u8], ctx: &str) {
    let lim = buf.len().min(40);
    for k in 0..=lim {
        size_queries(&buf[..k], &format!("{ctx} trunc={k}"));
    }
    if buf.len() > 1 {
        size_queries(&buf[..buf.len() - 1], &format!("{ctx} trunc=len-1"));
        size_queries(&buf[..buf.len() / 2], &format!("{ctx} trunc=len/2"));
    }
}

fn le32(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

// ==================================================== CONFIGS 117 / 118 ====

#[test]
fn cfg_frame_size_queries() {
    let mut rng = Rng::new(0x117_118);

    // ---- single frame with FCS, many shapes / sizes
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 7, 1000, 65536, 131072, 200000] {
            let src = gen(shape, size, &mut rng);
            for &lvl in &[1, 3, 9, 19] {
                let f = frame_with_fcs(&src, lvl);
                let ctx = format!("fcs shape={shape:?} size={size} lvl={lvl}");
                size_queries(&f, &ctx);
                truncations(&f, &ctx);
            }
        }
    }

    // ---- single frame WITHOUT FCS (streamed, unknown pledged size)
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 1000, 131072, 300000] {
            let src = gen(shape, size, &mut rng);
            for &cks in &[0, 1] {
                let f = frame_no_fcs(&src, 5, cks);
                let ctx = format!("nofcs shape={shape:?} size={size} cks={cks}");
                size_queries(&f, &ctx);
                truncations(&f, &ctx);
            }
        }
    }

    // ---- 0-byte frame (both flavours)
    let z = frame_with_fcs(&[], 3);
    size_queries(&z, "empty fcs");
    truncations(&z, "empty fcs");
    let z2 = frame_no_fcs(&[], 3, 1);
    size_queries(&z2, "empty nofcs");
    truncations(&z2, "empty nofcs");

    // ---- 3 concatenated frames (mix of FCS / no-FCS)
    for round in 0..8 {
        let a = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.range(0, 20000), &mut rng);
        let b = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.range(0, 20000), &mut rng);
        let c = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.range(0, 20000), &mut rng);
        let mut cat = Vec::new();
        cat.extend_from_slice(&frame_with_fcs(&a, 3));
        cat.extend_from_slice(&frame_no_fcs(&b, 3, (round & 1) as c_int));
        cat.extend_from_slice(&frame_with_fcs(&c, 9));
        let ctx = format!("concat3 round={round}");
        size_queries(&cat, &ctx);
        truncations(&cat, &ctx);
    }

    // ---- frames interleaved with skippable frames
    for round in 0..8 {
        let a = gen(Shape::Text, rng.range(1, 30000), &mut rng);
        let b = gen(Shape::Random, rng.range(1, 5000), &mut rng);
        let pay1 = gen(Shape::Random, rng.range(0, 500), &mut rng);
        let pay2 = gen(Shape::Rle, rng.range(0, 500), &mut rng);
        let mut cat = Vec::new();
        cat.extend_from_slice(&skippable(&pay1, (round % 16) as c_uint));
        cat.extend_from_slice(&frame_with_fcs(&a, 5));
        cat.extend_from_slice(&skippable(&pay2, ((round + 3) % 16) as c_uint));
        cat.extend_from_slice(&frame_no_fcs(&b, 5, 1));
        cat.extend_from_slice(&skippable(&[], 0));
        let ctx = format!("skip-interleave round={round}");
        size_queries(&cat, &ctx);
        truncations(&cat, &ctx);
    }

    // ---- a non-frame random buffer
    for round in 0..20 {
        let junk = gen(Shape::Random, rng.range(0, 200), &mut rng);
        size_queries(&junk, &format!("junk round={round} len={}", junk.len()));
    }
    // random buffers that happen to start with the zstd magic
    for round in 0..20 {
        let mut junk = gen(Shape::Random, rng.range(4, 200), &mut rng);
        junk[..4].copy_from_slice(&le32(ZSTD_MAGICNUMBER));
        size_queries(&junk, &format!("magic-junk round={round} len={}", junk.len()));
    }
}

// =========================================================== CONFIG 119 ====

#[test]
fn cfg_decompression_margin() {
    let (c_dm, r_dm) = unsafe { pair::<FnSizeQuery>("ZSTD_decompressionMargin") };
    let (c_dec, r_dec) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };
    let mut rng = Rng::new(0x119);

    // (label, frame bytes, original bytes)
    let mut cases: Vec<(String, Vec<u8>, Vec<u8>)> = Vec::new();
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 1000, 131072, 250000] {
            let src = gen(shape, size, &mut rng);
            // with FCS, no checksum
            cases.push((
                format!("fcs shape={shape:?} size={size}"),
                frame_with_fcs(&src, 5),
                src.clone(),
            ));
            // streamed, checksum on
            cases.push((
                format!("cks shape={shape:?} size={size}"),
                frame_no_fcs(&src, 5, 1),
                src.clone(),
            ));
            // streamed, checksum off
            cases.push((
                format!("nocks shape={shape:?} size={size}"),
                frame_no_fcs(&src, 5, 0),
                src,
            ));
        }
    }
    // multi-frame input (+ skippable frames)
    for round in 0..6 {
        let a = gen(Shape::Text, rng.range(1, 40000), &mut rng);
        let b = gen(Shape::Mixed, rng.range(1, 40000), &mut rng);
        let mut frame = Vec::new();
        frame.extend_from_slice(&frame_with_fcs(&a, 3));
        frame.extend_from_slice(&skippable(&gen(Shape::Random, 40, &mut rng), 2));
        frame.extend_from_slice(&frame_no_fcs(&b, 3, 1));
        let mut orig = a.clone();
        orig.extend_from_slice(&b);
        cases.push((format!("multi round={round}"), frame, orig));
    }

    for (label, frame, orig) in &cases {
        let a = unsafe { c_dm(frame.as_ptr(), frame.len()) };
        let b = unsafe { r_dm(frame.as_ptr(), frame.len()) };
        assert_eq!(a, b, "{label}: ZSTD_decompressionMargin (C err={})", err_code(a));
        if is_error(a) {
            continue;
        }
        // In-place decompression: output buffer = origSize + margin, with the
        // compressed frame sitting at the very end of that buffer.
        let total = orig.len() + a;
        for which in 0..2 {
            let mut buf = vec![0u8; total];
            let off = total - frame.len();
            buf[off..].copy_from_slice(frame);
            let n = unsafe {
                let dst = buf.as_mut_ptr();
                let src = dst.add(off) as *const u8;
                if which == 0 {
                    c_dec(dst, total, src, frame.len())
                } else {
                    r_dec(dst, total, src, frame.len())
                }
            };
            assert!(!is_error(n), "{label}: in-place decompress err {}", err_code(n));
            assert_eq!(n, orig.len(), "{label}: in-place decompressed size");
            assert_bytes_eq(&format!("{label}: in-place bytes (lib {which})"), &buf[..n], orig);
        }
    }
}

// ============================================ CONFIG 120 / ERRORS 208,209 ==

#[test]
fn cfg_isframe_isskippable() {
    let (c_if, r_if) = unsafe { pair::<FnUintQuery>("ZSTD_isFrame") };
    let (c_is, r_is) = unsafe { pair::<FnUintQuery>("ZSTD_isSkippableFrame") };
    let mut rng = Rng::new(0x120);

    let check = |buf: &[u8], ctx: &str| unsafe {
        let a = c_if(buf.as_ptr(), buf.len());
        let b = r_if(buf.as_ptr(), buf.len());
        assert_eq!(a, b, "{ctx}: ZSTD_isFrame");
        let a = c_is(buf.as_ptr(), buf.len());
        let b = r_is(buf.as_ptr(), buf.len());
        assert_eq!(a, b, "{ctx}: ZSTD_isSkippableFrame");
    };

    // a real zstd frame
    let f = frame_with_fcs(&gen(Shape::Text, 10000, &mut rng), 3);
    check(&f, "zstd frame");
    for k in 0..=f.len().min(24) {
        check(&f[..k], &format!("zstd frame prefix {k}"));
    }

    // all 16 skippable magics (bare magic and full frames)
    for v in 0u32..16 {
        let magic = ZSTD_MAGIC_SKIPPABLE_START + v;
        let mut buf = Vec::new();
        buf.extend_from_slice(&le32(magic));
        check(&buf, &format!("bare skippable magic {magic:#x}"));
        let sf = skippable(&gen(Shape::Random, 33, &mut rng), v as c_uint);
        check(&sf, &format!("skippable frame variant {v}"));
        for k in 0..=sf.len() {
            check(&sf[..k], &format!("skippable frame variant {v} prefix {k}"));
        }
    }
    // just outside the skippable range
    for &magic in &[
        ZSTD_MAGIC_SKIPPABLE_START - 1,
        ZSTD_MAGIC_SKIPPABLE_START + 16,
        ZSTD_MAGIC_SKIPPABLE_START + 17,
    ] {
        let mut buf = le32(magic).to_vec();
        buf.extend_from_slice(&[0, 0, 0, 0]);
        check(&buf, &format!("near-skippable {magic:#x}"));
    }

    // legacy magics
    for &magic in &[
        0xFD2FB51Eu32, 0xFD2FB522, 0xFD2FB523, 0xFD2FB524, 0xFD2FB525, 0xFD2FB526, 0xFD2FB527,
    ] {
        for extra in [0usize, 1, 4, 16] {
            let mut buf = le32(magic).to_vec();
            for _ in 0..extra {
                buf.push(rng.byte());
            }
            check(&buf, &format!("legacy magic {magic:#x} extra={extra}"));
        }
    }

    // magicless frame: strip the 4-byte magic from a normal frame
    let ml = &f[4..];
    check(ml, "magicless frame");
    for k in 0..=ml.len().min(24) {
        check(&ml[..k], &format!("magicless prefix {k}"));
    }

    // buffers of 0/1/3/4 bytes and random buffers (ERRORS 208, 209)
    for n in [0usize, 1, 2, 3, 4] {
        for round in 0..8 {
            let b = gen(Shape::Random, n, &mut rng);
            check(&b, &format!("tiny n={n} round={round}"));
        }
    }
    for round in 0..64 {
        let b = gen(Shape::Random, rng.range(0, 40), &mut rng);
        check(&b, &format!("random round={round} len={}", b.len()));
    }
}

// =========================================================== CONFIG 121 ====

#[test]
fn cfg_skippable_roundtrip() {
    let (cw, rw) = unsafe { pair::<FnWriteSkip>("ZSTD_writeSkippableFrame") };
    let (cr, rr) = unsafe { pair::<FnReadSkip>("ZSTD_readSkippableFrame") };
    let mut rng = Rng::new(0x121);

    for &variant in &[0u32, 1, 7, 15] {
        for &n in &[0usize, 1, 8, 1000] {
            for round in 0..6 {
                let payload = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], n, &mut rng);
                let ctx = format!("skip v={variant} n={n} round={round}");
                // ---- write, with exact and generous capacity
                for &extra in &[0usize, 1, 64] {
                    let cap = n + 8 + extra;
                    let mut co = vec![0xAAu8; cap];
                    let mut ro = vec![0x55u8; cap];
                    let a = unsafe {
                        cw(co.as_mut_ptr(), cap, payload.as_ptr(), payload.len(), variant)
                    };
                    let b = unsafe {
                        rw(ro.as_mut_ptr(), cap, payload.as_ptr(), payload.len(), variant)
                    };
                    assert_eq!(a, b, "{ctx} extra={extra}: write return");
                    assert!(!is_error(a), "{ctx}: write err {}", err_code(a));
                    assert_eq!(a, n + 8, "{ctx}: write size");
                    assert_bytes_eq(&format!("{ctx} extra={extra}: write bytes"), &co[..a], &ro[..a]);
                }
                let frame = skippable(&payload, variant);

                // ---- read back, exact and generous dst capacity, magicVariant out-param
                for &dcap in &[n, n + 1, n + 64] {
                    let mut cd = vec![0xAAu8; dcap.max(1)];
                    let mut rd = vec![0x55u8; dcap.max(1)];
                    let mut cv: c_uint = 0xDEAD;
                    let mut rv: c_uint = 0xBEEF;
                    let a = unsafe {
                        cr(cd.as_mut_ptr(), dcap, &mut cv, frame.as_ptr(), frame.len())
                    };
                    let b = unsafe {
                        rr(rd.as_mut_ptr(), dcap, &mut rv, frame.as_ptr(), frame.len())
                    };
                    assert_eq!(a, b, "{ctx} dcap={dcap}: read return");
                    assert!(!is_error(a), "{ctx}: read err {}", err_code(a));
                    assert_eq!(a, n, "{ctx}: read size");
                    assert_eq!(cv, rv, "{ctx} dcap={dcap}: magicVariant out");
                    assert_eq!(cv, variant, "{ctx}: magicVariant value");
                    assert_bytes_eq(&format!("{ctx} dcap={dcap}: read bytes"), &cd[..a], &rd[..a]);
                    assert_bytes_eq(&format!("{ctx}: payload round trip"), &cd[..a], &payload);
                }
                // ---- read back with magicVariant == NULL
                let mut cd = vec![0u8; n.max(1)];
                let mut rd = vec![0u8; n.max(1)];
                let a = unsafe {
                    cr(cd.as_mut_ptr(), n, std::ptr::null_mut(), frame.as_ptr(), frame.len())
                };
                let b = unsafe {
                    rr(rd.as_mut_ptr(), n, std::ptr::null_mut(), frame.as_ptr(), frame.len())
                };
                assert_eq!(a, b, "{ctx}: read return (NULL variant)");
                assert_bytes_eq(&format!("{ctx}: read bytes (NULL variant)"), &cd[..a], &rd[..a]);
            }
        }
    }
}

// ======================================= ERRORS 115,117,164,203-207 ========

#[test]
fn err_skippable() {
    let (cw, rw) = unsafe { pair::<FnWriteSkip>("ZSTD_writeSkippableFrame") };
    let (cr, rr) = unsafe { pair::<FnReadSkip>("ZSTD_readSkippableFrame") };
    let (c_dec, r_dec) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };
    let mut rng = Rng::new(0x115_117);

    // ---- ERRORS 115: dstCapacity < srcSize + 8
    for &n in &[0usize, 1, 8, 100, 1000] {
        let payload = gen(Shape::Random, n, &mut rng);
        for cap in [0usize, 1, 4, 7, n + 7].into_iter().chain(if n > 0 { Some(n) } else { None }) {
            if cap >= n + 8 {
                continue;
            }
            let mut co = vec![0xAAu8; cap.max(1)];
            let mut ro = vec![0x55u8; cap.max(1)];
            let a = unsafe { cw(co.as_mut_ptr(), cap, payload.as_ptr(), payload.len(), 0) };
            let b = unsafe { rw(ro.as_mut_ptr(), cap, payload.as_ptr(), payload.len(), 0) };
            assert_eq!(a, b, "write n={n} cap={cap}: return (C err={})", err_code(a));
            assert!(is_error(a), "write n={n} cap={cap} should fail");
        }
    }

    // ---- ERRORS 117: magicVariant > 15
    for &v in &[16u32, 17, 99, 1000, u32::MAX, u32::MAX - 1] {
        for &n in &[0usize, 1, 100] {
            let payload = gen(Shape::Random, n, &mut rng);
            let cap = n + 64;
            let mut co = vec![0xAAu8; cap];
            let mut ro = vec![0x55u8; cap];
            let a = unsafe { cw(co.as_mut_ptr(), cap, payload.as_ptr(), payload.len(), v) };
            let b = unsafe { rw(ro.as_mut_ptr(), cap, payload.as_ptr(), payload.len(), v) };
            assert_eq!(a, b, "write variant={v} n={n}: return (C err={})", err_code(a));
            assert!(is_error(a), "write variant={v} should fail");
        }
    }

    // ---- ERRORS 203: readSkippableFrame with srcSize < 8
    let good = skippable(&gen(Shape::Text, 200, &mut rng), 3);
    for k in 0..8 {
        let mut cd = vec![0u8; 512];
        let mut rd = vec![0u8; 512];
        let mut cv: c_uint = 0;
        let mut rv: c_uint = 0;
        let a = unsafe { cr(cd.as_mut_ptr(), 512, &mut cv, good.as_ptr(), k) };
        let b = unsafe { rr(rd.as_mut_ptr(), 512, &mut rv, good.as_ptr(), k) };
        assert_eq!(a, b, "read srcSize={k}: return (C err={})", err_code(a));
        assert!(is_error(a), "read srcSize={k} should fail");
    }

    // ---- ERRORS 204: wrong magic
    for &magic in &[
        ZSTD_MAGICNUMBER,
        ZSTD_MAGIC_SKIPPABLE_START - 1,
        ZSTD_MAGIC_SKIPPABLE_START + 16,
        0u32,
        0xFFFFFFFF,
        0xFD2FB525,
    ] {
        let mut buf = le32(magic).to_vec();
        buf.extend_from_slice(&le32(4));
        buf.extend_from_slice(&[1, 2, 3, 4]);
        let mut cd = vec![0u8; 64];
        let mut rd = vec![0u8; 64];
        let mut cv: c_uint = 0;
        let mut rv: c_uint = 0;
        let a = unsafe { cr(cd.as_mut_ptr(), 64, &mut cv, buf.as_ptr(), buf.len()) };
        let b = unsafe { rr(rd.as_mut_ptr(), 64, &mut rv, buf.as_ptr(), buf.len()) };
        assert_eq!(a, b, "read magic={magic:#x}: return (C err={})", err_code(a));
    }

    // ---- ERRORS 205: declared sizeU32 + 8 overflows u32
    for &decl in &[0xFFFFFFFFu32, 0xFFFFFFF8, 0xFFFFFFFC] {
        let mut buf = le32(ZSTD_MAGIC_SKIPPABLE_START).to_vec();
        buf.extend_from_slice(&le32(decl));
        buf.extend_from_slice(&[0u8; 16]);
        let mut cd = vec![0u8; 64];
        let mut rd = vec![0u8; 64];
        let mut cv: c_uint = 0;
        let mut rv: c_uint = 0;
        let a = unsafe { cr(cd.as_mut_ptr(), 64, &mut cv, buf.as_ptr(), buf.len()) };
        let b = unsafe { rr(rd.as_mut_ptr(), 64, &mut rv, buf.as_ptr(), buf.len()) };
        assert_eq!(a, b, "read overflow decl={decl:#x}: return (C err={})", err_code(a));
        assert!(is_error(a));
    }

    // ---- ERRORS 206 / 164: declared size > srcSize
    for &(decl, extra) in &[(1000u32, 10usize), (100, 0), (5, 4), (0xFFFF, 40)] {
        let mut buf = le32(ZSTD_MAGIC_SKIPPABLE_START + 5).to_vec();
        buf.extend_from_slice(&le32(decl));
        for _ in 0..extra {
            buf.push(rng.byte());
        }
        let mut cd = vec![0u8; 0x20000];
        let mut rd = vec![0u8; 0x20000];
        let mut cv: c_uint = 0;
        let mut rv: c_uint = 0;
        let a = unsafe { cr(cd.as_mut_ptr(), 0x20000, &mut cv, buf.as_ptr(), buf.len()) };
        let b = unsafe { rr(rd.as_mut_ptr(), 0x20000, &mut rv, buf.as_ptr(), buf.len()) };
        assert_eq!(a, b, "read decl={decl} extra={extra}: return (C err={})", err_code(a));
        assert!(is_error(a));
        // ERRORS 164: ZSTD_decompress on such a truncated skippable frame
        let mut co = vec![0u8; 4096];
        let mut ro = vec![0u8; 4096];
        let a = unsafe { c_dec(co.as_mut_ptr(), 4096, buf.as_ptr(), buf.len()) };
        let b = unsafe { r_dec(ro.as_mut_ptr(), 4096, buf.as_ptr(), buf.len()) };
        assert_eq!(a, b, "decompress bad skippable decl={decl}: (C err={})", err_code(a));
        assert!(is_error(a));
    }

    // ---- ERRORS 207: payload size > dstCapacity
    for &n in &[1usize, 8, 1000] {
        let payload = gen(Shape::Text, n, &mut rng);
        let frame = skippable(&payload, 1);
        for dcap in 0..n.min(8) {
            let mut cd = vec![0u8; 16];
            let mut rd = vec![0u8; 16];
            let mut cv: c_uint = 0;
            let mut rv: c_uint = 0;
            let a = unsafe { cr(cd.as_mut_ptr(), dcap, &mut cv, frame.as_ptr(), frame.len()) };
            let b = unsafe { rr(rd.as_mut_ptr(), dcap, &mut rv, frame.as_ptr(), frame.len()) };
            assert_eq!(a, b, "read n={n} dcap={dcap}: return (C err={})", err_code(a));
            assert!(is_error(a));
        }
    }
}

// =========================================================== CONFIG 122 ====

#[test]
fn cfg_decompress_concat_skippable() {
    let (c_dec, r_dec) = unsafe { pair::<FnDecompress>("ZSTD_decompress") };
    let mut rng = Rng::new(0x122);

    for round in 0..16 {
        let a = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.range(0, 60000), &mut rng);
        let b = gen(ALL_SHAPES[rng.below(ALL_SHAPES.len())], rng.range(0, 60000), &mut rng);
        let p1 = gen(Shape::Random, rng.range(0, 300), &mut rng);
        let p2 = gen(Shape::Text, rng.range(0, 300), &mut rng);
        let p3 = gen(Shape::Rle, rng.range(0, 300), &mut rng);

        let mut cat = Vec::new();
        cat.extend_from_slice(&skippable(&p1, (round % 16) as c_uint));
        cat.extend_from_slice(&frame_with_fcs(&a, 3));
        cat.extend_from_slice(&skippable(&p2, ((round * 5 + 1) % 16) as c_uint));
        cat.extend_from_slice(&frame_no_fcs(&b, 7, (round & 1) as c_int));
        cat.extend_from_slice(&skippable(&p3, ((round * 3 + 2) % 16) as c_uint));

        let mut orig = a.clone();
        orig.extend_from_slice(&b);

        let cap = orig.len() + 64;
        let mut co = vec![0xAAu8; cap];
        let mut ro = vec![0x55u8; cap];
        let x = unsafe { c_dec(co.as_mut_ptr(), cap, cat.as_ptr(), cat.len()) };
        let y = unsafe { r_dec(ro.as_mut_ptr(), cap, cat.as_ptr(), cat.len()) };
        assert_eq!(x, y, "concat round={round}: decompress return (C err={})", err_code(x));
        assert!(!is_error(x), "concat round={round}: err {}", err_code(x));
        assert_bytes_eq(&format!("concat round={round}"), &co[..x], &ro[..y]);
        assert_bytes_eq(&format!("concat round={round} vs orig"), &co[..x], &orig);

        // and the size queries agree on the whole concatenation
        size_queries(&cat, &format!("concat round={round}"));
    }
}

// =========================================== CONFIG 161 / ERRORS 259 ======

#[test]
fn cfg_bound_functions() {
    let (c_cb, r_cb) = unsafe { pair::<FnBound>("ZSTD_compressBound") };
    let (c_db, r_db) = unsafe { pair::<FnU64Query>("ZSTD_decompressBound") };
    let (c_dbm, r_dbm) = unsafe { pair::<FnDecodingBufMin>("ZSTD_decodingBufferSize_min") };
    let mut rng = Rng::new(0x161);

    // ---- ZSTD_compressBound
    let mut sizes: Vec<usize> = vec![0, 1, 131072, 1 << 20, 1 << 30, usize::MAX / 2];
    sizes.extend_from_slice(&[2, 3, 7, 128, 65535, 65536, 131071, 131073, 1 << 24]);
    sizes.push(usize::MAX);
    sizes.push(0xFF00FF00FF00FF00);
    sizes.push(0xFF00FF00FF00FF00 - 1);
    for _ in 0..40 {
        sizes.push(rng.below(1 << 22));
    }
    for &n in &sizes {
        let a = unsafe { c_cb(n) };
        let b = unsafe { r_cb(n) };
        assert_eq!(a, b, "ZSTD_compressBound({n})");
    }

    // ---- ZSTD_decompressBound over real frames of those (reasonable) sizes
    for &n in &[0usize, 1, 131072, 1 << 20] {
        let src = gen(Shape::Mixed, n, &mut rng);
        for &lvl in &[1, 9] {
            let f = frame_with_fcs(&src, lvl);
            let a = unsafe { c_db(f.as_ptr(), f.len()) };
            let b = unsafe { r_db(f.as_ptr(), f.len()) };
            assert_eq!(a, b, "decompressBound(fcs n={n} lvl={lvl})");
            let f = frame_no_fcs(&src, lvl, 1);
            let a = unsafe { c_db(f.as_ptr(), f.len()) };
            let b = unsafe { r_db(f.as_ptr(), f.len()) };
            assert_eq!(a, b, "decompressBound(nofcs n={n} lvl={lvl})");
        }
    }

    // ---- ZSTD_decodingBufferSize_min: windowSize x frameContentSize matrix,
    // including huge values (ERRORS 259: on a 64-bit target size_t == u64, so the
    // "minRBSize != neededSize" narrowing check cannot fire; both libraries must
    // still agree on whatever they return, including the wrap-around cases).
    let windows: Vec<u64> = vec![
        0,
        1,
        1024,
        1 << 10,
        (1 << 17) - 1,
        1 << 17,
        (1 << 17) + 1,
        1 << 20,
        1 << 27,
        1 << 30,
        1u64 << 31,
        1u64 << 32,
        1u64 << 40,
        1u64 << 62,
        1u64 << 63,
        u64::MAX / 2,
        u64::MAX - 2,
        u64::MAX - 1,
        u64::MAX,
    ];
    let contents: Vec<u64> = vec![
        0,
        1,
        131072,
        1 << 20,
        1 << 30,
        (usize::MAX / 2) as u64,
        1u64 << 32,
        1u64 << 62,
        u64::MAX - 2,
        u64::MAX - 1,
        u64::MAX, // ZSTD_CONTENTSIZE_UNKNOWN
    ];
    for &w in &windows {
        for &fcs in &contents {
            let a = unsafe { c_dbm(w, fcs) };
            let b = unsafe { r_dbm(w, fcs) };
            assert_eq!(a, b, "ZSTD_decodingBufferSize_min({w},{fcs}) (C err={})", err_code(a));
        }
    }
    for _ in 0..200 {
        let w = rng.next_u64();
        let fcs = rng.next_u64();
        let a = unsafe { c_dbm(w, fcs) };
        let b = unsafe { r_dbm(w, fcs) };
        assert_eq!(a, b, "ZSTD_decodingBufferSize_min(rand {w},{fcs})");
    }
}

// ========================================================== ERRORS 202 ====

#[test]
fn err_decompression_margin() {
    let (c_dm, r_dm) = unsafe { pair::<FnSizeQuery>("ZSTD_decompressionMargin") };
    let mut rng = Rng::new(0x202);

    let cmp = |buf: &[u8], ctx: &str| unsafe {
        let a = c_dm(buf.as_ptr(), buf.len());
        let b = r_dm(buf.as_ptr(), buf.len());
        assert_eq!(a, b, "{ctx}: ZSTD_decompressionMargin (C err={})", err_code(a));
        a
    };

    // truncated frames of all lengths
    let f = frame_with_fcs(&gen(Shape::Text, 200000, &mut rng), 5);
    for k in 0..=f.len().min(64) {
        cmp(&f[..k], &format!("trunc {k}"));
    }
    cmp(&f[..f.len() - 1], "trunc len-1");
    cmp(&f[..f.len() / 2], "trunc len/2");

    // checksummed frame truncated inside the trailing checksum
    let fc = frame_no_fcs(&gen(Shape::Mixed, 200000, &mut rng), 5, 1);
    for k in 1..=4 {
        cmp(&fc[..fc.len() - k], &format!("cks trunc -{k}"));
    }

    // bad magic / random junk
    for round in 0..32 {
        let j = gen(Shape::Random, rng.range(0, 64), &mut rng);
        cmp(&j, &format!("junk {round}"));
    }
    // valid magic then junk
    for round in 0..32 {
        let mut j = gen(Shape::Random, rng.range(4, 64), &mut rng);
        j[..4].copy_from_slice(&le32(ZSTD_MAGICNUMBER));
        cmp(&j, &format!("magic junk {round}"));
    }
    // trailing garbage after a complete frame
    for extra in [1usize, 3, 5, 20] {
        let mut b = f.clone();
        for _ in 0..extra {
            b.push(rng.byte());
        }
        cmp(&b, &format!("trailing {extra}"));
    }
    // skippable frame with declared size > input
    let mut sk = le32(ZSTD_MAGIC_SKIPPABLE_START).to_vec();
    sk.extend_from_slice(&le32(9999));
    sk.extend_from_slice(&[0u8; 20]);
    cmp(&sk, "skippable oversized");
    // windowLog field 32
    cmp(&hdr_windowlog(22), "windowLog field 32");
}

/// Hand-built frame header with an explicit windowLog byte.
/// wlCode is stored in the top 5 bits: windowLog = wlCode + ZSTD_WINDOWLOG_ABSOLUTEMIN(10).
fn hdr_windowlog(wl_code: u8) -> Vec<u8> {
    let mut v = le32(ZSTD_MAGICNUMBER).to_vec();
    v.push(0x00); // fhd: no single segment, dictIDSizeCode 0, fcsID 0, no checksum
    v.push(wl_code << 3);
    // one raw last block of 1 byte
    let bh: u32 = 1 | (0 << 1) | (1 << 3); // lastBlock=1, bt_raw=0, size=1
    v.push((bh & 0xFF) as u8);
    v.push(((bh >> 8) & 0xFF) as u8);
    v.push(((bh >> 16) & 0xFF) as u8);
    v.push(0x42);
    v
}

/// Hand-built frame header: singleSegment with an explicit 8-byte FCS.
fn hdr_single_segment_fcs(fcs: u64) -> Vec<u8> {
    let mut v = le32(ZSTD_MAGICNUMBER).to_vec();
    v.push(0x20 | 0xC0); // singleSegment=1, fcsID=3
    v.extend_from_slice(&fcs.to_le_bytes());
    let bh: u32 = 1 | (1 << 3); // lastBlock, bt_raw, size=1
    v.push((bh & 0xFF) as u8);
    v.push(((bh >> 8) & 0xFF) as u8);
    v.push(((bh >> 16) & 0xFF) as u8);
    v.push(0x42);
    v
}

// ===================================================== ERRORS 197-200 =====

#[test]
fn err_findframecompressedsize() {
    let (c_f, r_f) = unsafe { pair::<FnSizeQuery>("ZSTD_findFrameCompressedSize") };
    let mut rng = Rng::new(0x197);

    let cmp = |buf: &[u8], ctx: &str| unsafe {
        let a = c_f(buf.as_ptr(), buf.len());
        let b = r_f(buf.as_ptr(), buf.len());
        assert_eq!(a, b, "{ctx}: findFrameCompressedSize (C err={})", err_code(a));
        a
    };

    // ERRORS 197: frame header incomplete (every truncation of the header)
    let f = frame_with_fcs(&gen(Shape::Text, 150000, &mut rng), 5);
    for k in 0..=24 {
        cmp(&f[..k.min(f.len())], &format!("hdr trunc {k}"));
    }

    // ERRORS 198: bad magic + windowTooLarge
    for round in 0..24 {
        let j = gen(Shape::Random, rng.range(0, 40), &mut rng);
        cmp(&j, &format!("bad magic {round}"));
    }
    for wl in [21u8, 22, 23, 31] {
        cmp(&hdr_windowlog(wl), &format!("windowLog code {wl}"));
    }

    // ERRORS 199: a block extends past srcSize
    for k in 1..=(f.len() - 8).min(400) {
        cmp(&f[..f.len() - k], &format!("block past end -{k}"));
    }

    // ERRORS 200: checksum flag but < 4 bytes remaining
    let fc = frame_no_fcs(&gen(Shape::Mixed, 150000, &mut rng), 5, 1);
    for k in 1..=4 {
        cmp(&fc[..fc.len() - k], &format!("cks remaining -{k}"));
    }

    // valid single/multi frame: exact answers must agree
    cmp(&f, "valid frame");
    let mut cat = f.clone();
    cat.extend_from_slice(&frame_with_fcs(&gen(Shape::Rle, 5000, &mut rng), 1));
    cmp(&cat, "first of two frames");
    cmp(&skippable(&gen(Shape::Random, 100, &mut rng), 4), "skippable frame");
}

// ================================================ ERRORS 190-196, 201 =====

#[test]
fn err_contentsize_queries() {
    let (c_gfcs, r_gfcs) = unsafe { pair::<FnU64Query>("ZSTD_getFrameContentSize") };
    let (c_gds, r_gds) = unsafe { pair::<FnU64Query>("ZSTD_getDecompressedSize") };
    let (c_fds, r_fds) = unsafe { pair::<FnU64Query>("ZSTD_findDecompressedSize") };
    let (c_db, r_db) = unsafe { pair::<FnU64Query>("ZSTD_decompressBound") };
    let mut rng = Rng::new(0x190);

    let cmp = |buf: &[u8], ctx: &str| unsafe {
        let p = buf.as_ptr();
        let n = buf.len();
        assert_eq!(c_gfcs(p, n), r_gfcs(p, n), "{ctx}: getFrameContentSize");
        assert_eq!(c_gds(p, n), r_gds(p, n), "{ctx}: getDecompressedSize");
        assert_eq!(c_fds(p, n), r_fds(p, n), "{ctx}: findDecompressedSize");
        assert_eq!(c_db(p, n), r_db(p, n), "{ctx}: decompressBound");
        c_gfcs(p, n)
    };

    // ERRORS 190: truncated header / bad magic => CONTENTSIZE_ERROR
    let f = frame_with_fcs(&gen(Shape::Text, 100000, &mut rng), 5);
    for k in 0..=24 {
        cmp(&f[..k.min(f.len())], &format!("trunc {k}"));
    }
    for round in 0..24 {
        cmp(&gen(Shape::Random, rng.range(0, 40), &mut rng), &format!("junk {round}"));
    }
    for wl in [21u8, 22, 31] {
        cmp(&hdr_windowlog(wl), &format!("windowLog code {wl}"));
    }

    // ERRORS 191: valid frame without the FCS field => CONTENTSIZE_UNKNOWN
    for &size in &[0usize, 1, 1000, 131072] {
        let g = frame_no_fcs(&gen(Shape::Text, size, &mut rng), 3, 0);
        let r = cmp(&g, &format!("unknown fcs size={size}"));
        assert_eq!(r, ZSTD_CONTENTSIZE_UNKNOWN, "unknown-FCS frame should report UNKNOWN");
    }

    // ERRORS 192: skippable frame => 0
    for v in 0u32..16 {
        let s = skippable(&gen(Shape::Random, 77, &mut rng), v as c_uint);
        let r = cmp(&s, &format!("skippable v={v}"));
        assert_eq!(r, 0, "skippable getFrameContentSize");
    }

    // ERRORS 193: getDecompressedSize returns 0 on error/unknown
    unsafe {
        let g = frame_no_fcs(&gen(Shape::Text, 5000, &mut rng), 3, 0);
        assert_eq!(c_gds(g.as_ptr(), g.len()), 0);
        assert_eq!(r_gds(g.as_ptr(), g.len()), 0);
    }

    // ERRORS 194: malformed skippable frame size
    for &decl in &[9999u32, 0xFFFFFFFF, 0xFFFFFFF8] {
        let mut sk = le32(ZSTD_MAGIC_SKIPPABLE_START + 2).to_vec();
        sk.extend_from_slice(&le32(decl));
        sk.extend_from_slice(&[0u8; 12]);
        cmp(&sk, &format!("bad skippable decl={decl:#x}"));
    }

    // ERRORS 195: accumulated size overflows u64 (two huge declared FCS frames)
    let huge = hdr_single_segment_fcs(0xFFFFFFFFFFFFFF00);
    cmp(&huge, "huge declared fcs");
    let mut two = huge.clone();
    two.extend_from_slice(&huge);
    cmp(&two, "two huge declared fcs (overflow)");
    for &fcs in &[
        1u64 << 32,
        1u64 << 40,
        1u64 << 62,
        u64::MAX - 3,
        u64::MAX - 2,
        u64::MAX - 1,
        u64::MAX,
    ] {
        cmp(&hdr_single_segment_fcs(fcs), &format!("declared fcs={fcs}"));
    }

    // ERRORS 196 / 201: leftover bytes not forming a complete frame
    for extra in [1usize, 2, 3, 4, 5, 6, 7, 8, 20] {
        let mut b = f.clone();
        for _ in 0..extra {
            b.push(rng.byte());
        }
        cmp(&b, &format!("trailing {extra}"));
    }
    // valid frame followed by a truncated one
    let g = frame_with_fcs(&gen(Shape::Rle, 3000, &mut rng), 1);
    for k in 1..g.len().min(30) {
        let mut b = f.clone();
        b.extend_from_slice(&g[..k]);
        cmp(&b, &format!("frame + truncated frame {k}"));
    }
}

// ===================================================== ERRORS 186-189 =====

#[test]
fn err_frameheader_queries() {
    let (c_fhs, r_fhs) = unsafe { pair::<FnSizeQuery>("ZSTD_frameHeaderSize") };
    let (c_gfh, r_gfh) = unsafe { pair::<FnGetFrameHeader>("ZSTD_getFrameHeader") };
    let (c_gfa, r_gfa) = unsafe { pair::<FnGetFrameHeaderAdv>("ZSTD_getFrameHeader_advanced") };
    let mut rng = Rng::new(0x186);

    // helper: compare frameHeaderSize + getFrameHeader + advanced(both formats)
    let cmp = |buf: &[u8], ctx: &str| unsafe {
        let p = buf.as_ptr();
        let n = buf.len();
        let a = c_fhs(p, n);
        let b = r_fhs(p, n);
        assert_eq!(a, b, "{ctx}: frameHeaderSize (C err={})", err_code(a));

        let mut ch = FrameHeader::default();
        let mut rh = FrameHeader::default();
        let x = c_gfh(&mut ch, p, n);
        let y = r_gfh(&mut rh, p, n);
        assert_eq!(x, y, "{ctx}: getFrameHeader (C err={})", err_code(x));
        if x == 0 {
            assert_eq!(ch, rh, "{ctx}: getFrameHeader struct");
        }
        for fmt in [0, 1] {
            let mut ch = FrameHeader::default();
            let mut rh = FrameHeader::default();
            let x = c_gfa(&mut ch, p, n, fmt);
            let y = r_gfa(&mut rh, p, n, fmt);
            assert_eq!(x, y, "{ctx}: getFrameHeader_advanced(fmt={fmt}) (C err={})", err_code(x));
            if x == 0 {
                assert_eq!(ch, rh, "{ctx}: getFrameHeader_advanced(fmt={fmt}) struct");
            }
        }
    };

    // ERRORS 186 / 188: srcSize below the minimum and every truncation of a
    // valid header, over many header shapes.
    for &shape in ALL_SHAPES {
        for &size in &[0usize, 1, 255, 256, 65791, 65792, 200000] {
            let src = gen(shape, size, &mut rng);
            for &lvl in &[1, 9] {
                let f = frame_with_fcs(&src, lvl);
                for k in 0..=f.len().min(24) {
                    cmp(&f[..k], &format!("fcs shape={shape:?} size={size} lvl={lvl} n={k}"));
                }
            }
            let f = frame_no_fcs(&src, 5, 1);
            for k in 0..=f.len().min(24) {
                cmp(&f[..k], &format!("nofcs shape={shape:?} size={size} n={k}"));
            }
        }
    }
    // skippable frame headers
    for v in 0u32..16 {
        let s = skippable(&gen(Shape::Random, 300, &mut rng), v as c_uint);
        for k in 0..=s.len().min(16) {
            cmp(&s[..k], &format!("skippable v={v} n={k}"));
        }
    }

    // ERRORS 189: unknown magic, plus random junk of every small length
    for round in 0..64 {
        let j = gen(Shape::Random, rng.range(0, 24), &mut rng);
        cmp(&j, &format!("junk {round} len={}", j.len()));
    }
    for &magic in &[0u32, 0xFFFFFFFF, ZSTD_MAGICNUMBER - 1, ZSTD_MAGICNUMBER + 1, 0xFD2FB525] {
        let mut b = le32(magic).to_vec();
        b.extend_from_slice(&[0u8; 20]);
        cmp(&b, &format!("magic {magic:#x}"));
        for k in 0..=8 {
            cmp(&b[..k], &format!("magic {magic:#x} n={k}"));
        }
    }
    // reserved bit / windowLog field 32 / huge singleSegment FCS
    cmp(&hdr_windowlog(22), "windowLog code 22 (=32)");
    cmp(&hdr_windowlog(31), "windowLog code 31 (=41)");
    cmp(&hdr_single_segment_fcs(1u64 << 32), "singleSegment fcs 2^32");
    cmp(&hdr_single_segment_fcs(u64::MAX), "singleSegment fcs u64::MAX");
    {
        let mut b = le32(ZSTD_MAGICNUMBER).to_vec();
        b.push(0x08); // reserved bit set
        b.extend_from_slice(&[0u8; 20]);
        cmp(&b, "reserved bit set");
    }

    // ERRORS 187: src == NULL while srcSize > 0 (getFrameHeader only; the
    // frameHeaderSize entry point dereferences src unconditionally, so it is
    // deliberately not called with NULL here).
    for n in [1usize, 4, 5, 6, 18, 1000] {
        let mut ch = FrameHeader::default();
        let mut rh = FrameHeader::default();
        let x = unsafe { c_gfh(&mut ch, std::ptr::null(), n) };
        let y = unsafe { r_gfh(&mut rh, std::ptr::null(), n) };
        assert_eq!(x, y, "NULL src n={n}: getFrameHeader (C err={})", err_code(x));
        assert!(is_error(x), "NULL src n={n} should be an error");
        for fmt in [0, 1] {
            let x = unsafe { c_gfa(&mut ch, std::ptr::null(), n, fmt) };
            let y = unsafe { r_gfa(&mut rh, std::ptr::null(), n, fmt) };
            assert_eq!(x, y, "NULL src n={n} fmt={fmt}: advanced (C err={})", err_code(x));
        }
    }
    // src == NULL with srcSize == 0 is legal and must return the wanted size
    for fmt in [0, 1] {
        let mut ch = FrameHeader::default();
        let mut rh = FrameHeader::default();
        let x = unsafe { c_gfa(&mut ch, std::ptr::null(), 0, fmt) };
        let y = unsafe { r_gfa(&mut rh, std::ptr::null(), 0, fmt) };
        assert_eq!(x, y, "NULL src n=0 fmt={fmt}: advanced");
    }

    // magicless frames: only reachable through the _advanced entry point
    let f = frame_with_fcs(&gen(Shape::Text, 60000, &mut rng), 5);
    let ml = &f[4..];
    for k in 0..=ml.len().min(20) {
        let mut ch = FrameHeader::default();
        let mut rh = FrameHeader::default();
        let x = unsafe { c_gfa(&mut ch, ml.as_ptr(), k, 1) };
        let y = unsafe { r_gfa(&mut rh, ml.as_ptr(), k, 1) };
        assert_eq!(x, y, "magicless n={k}: advanced (C err={})", err_code(x));
        if x == 0 {
            assert_eq!(ch, rh, "magicless n={k}: struct");
        }
    }
}
