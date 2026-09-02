//! Phase B: differential tests for the simple one-shot API and the
//! informational / bound-query helpers.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_ulonglong, c_void};

type FnCompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
type FnDecompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnGetFCS = unsafe extern "C" fn(*const c_void, size_t) -> c_ulonglong;
type FnFindFrameCompressedSize = unsafe extern "C" fn(*const c_void, size_t) -> size_t;
type FnCompressBound = unsafe extern "C" fn(size_t) -> size_t;

#[test]
fn version_and_static_info() {
    unsafe {
        let (cn, rn) = both::<FnVoidToUint>("ZSTD_versionNumber");
        assert_eq!(cn(), rn(), "ZSTD_versionNumber");
        let (cs, rs_) = both::<unsafe extern "C" fn() -> *const std::os::raw::c_char>(
            "ZSTD_versionString",
        );
        assert_eq!(cstr(cs()), cstr(rs_()), "ZSTD_versionString");
        for n in ["ZSTD_minCLevel", "ZSTD_maxCLevel", "ZSTD_defaultCLevel"] {
            let (a, b) = both::<FnVoidToInt>(n);
            assert_eq!(a(), b(), "{n}");
        }
        for n in [
            "ZSTD_CStreamInSize",
            "ZSTD_CStreamOutSize",
            "ZSTD_DStreamInSize",
            "ZSTD_DStreamOutSize",
        ] {
            let (a, b) = both::<FnVoidToSize>(n);
            assert_eq!(a(), b(), "{n}");
        }
    }
}

#[test]
fn compress_bound_and_decompress_bound() {
    unsafe {
        let (cb, rb) = both::<FnCompressBound>("ZSTD_compressBound");
        let mut rng = Rng::new(0x5eed_0001);
        let mut cases: Vec<usize> = LENS.to_vec();
        cases.extend([
            usize::MAX,
            usize::MAX - 1,
            usize::MAX / 2,
            1 << 30,
            (1usize << 31) + 7,
            0x7fff_ffff_ffff_ffff,
        ]);
        for _ in 0..200 {
            cases.push(rng.next_u64() as usize);
        }
        for &n in &cases {
            assert_eq!(cb(n), rb(n), "ZSTD_compressBound({n})");
        }
        // ZSTD_decompressBound over real frames
        let (cdb, rdb) = both::<FnGetFCS>("ZSTD_decompressBound");
        let (cc, _) = both::<FnCompress>("ZSTD_compress");
        for &len in &[0usize, 1, 100, 5000, 70000] {
            let src = gen(Shape::Text, len, &mut rng);
            let mut buf = vec![0u8; cb(src.len()) + 64];
            let n = cc(buf.as_mut_ptr() as *mut c_void, buf.len(), src.as_ptr() as *const c_void,
                       src.len(), 3);
            assert!(!Err2::new().c.is_err(n));
            assert_eq!(
                cdb(buf.as_ptr() as *const c_void, n),
                rdb(buf.as_ptr() as *const c_void, n),
                "ZSTD_decompressBound len={len}"
            );
        }
    }
}

/// One-shot compress at every level, over every data shape and many lengths.
/// Asserts the compressed bytes are byte-identical, and that each library can
/// decompress the other's output (cross-decompression).
#[test]
fn oneshot_compress_all_levels_shapes() {
    unsafe {
        let e = Err2::new();
        let (cc, rc) = both::<FnCompress>("ZSTD_compress");
        let (cd, rd) = both::<FnDecompress>("ZSTD_decompress");
        let (cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let (minl, _) = both::<FnVoidToInt>("ZSTD_minCLevel");
        let (maxl, _) = both::<FnVoidToInt>("ZSTD_maxCLevel");
        let lo = minl();
        let hi = maxl();
        let mut rng = Rng::new(0x5eed_0002);

        // levels: all "normal" levels plus a sample of the negative range
        let mut levels: Vec<c_int> = (1..=hi).collect();
        levels.push(0);
        for l in [-1, -2, -3, -5, -10, -50, -1000, -131072] {
            if l >= lo {
                levels.push(l);
            }
        }
        levels.push(lo);

        for &shape in ALL_SHAPES {
            for &len in &[0usize, 1, 13, 100, 1024, 5000, 40_000, 131_100] {
                let src = gen(shape, len, &mut rng);
                for &lvl in &levels {
                    let cap = cb(src.len()) + 32;
                    let mut cbuf = vec![0xAAu8; cap];
                    let mut rbuf = vec![0xAAu8; cap];
                    let cn = cc(cbuf.as_mut_ptr() as *mut c_void, cap,
                                src.as_ptr() as *const c_void, src.len(), lvl);
                    let rn = rc(rbuf.as_mut_ptr() as *mut c_void, cap,
                                src.as_ptr() as *const c_void, src.len(), lvl);
                    let ctx = format!("compress shape={shape:?} len={len} lvl={lvl}");
                    e.eq(&ctx, cn, rn);
                    if e.c.is_err(cn) {
                        continue;
                    }
                    assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);

                    // cross-decompress: C decodes RS output and vice versa
                    let mut d1 = vec![0u8; src.len() + 16];
                    let mut d2 = vec![0u8; src.len() + 16];
                    let a = cd(d1.as_mut_ptr() as *mut c_void, d1.len(),
                               rbuf.as_ptr() as *const c_void, rn);
                    let b = rd(d2.as_mut_ptr() as *mut c_void, d2.len(),
                               cbuf.as_ptr() as *const c_void, cn);
                    e.eq(&format!("{ctx} / cross-decompress"), a, b);
                    assert_eq!(a, src.len(), "{ctx}: roundtrip size");
                    assert_bytes_eq(&format!("{ctx} / decoded"), &d1[..a], &src);
                    assert_bytes_eq(&format!("{ctx} / decoded rs"), &d2[..b], &src);
                }
            }
        }
    }
}

/// Randomized property sweep: random shapes, random lengths, random levels.
#[test]
fn oneshot_random_property_sweep() {
    unsafe {
        let e = Err2::new();
        let (cc, rc) = both::<FnCompress>("ZSTD_compress");
        let (cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let (maxl, _) = both::<FnVoidToInt>("ZSTD_maxCLevel");
        let hi = maxl();
        let mut rng = Rng::new(0x5eed_0003);
        for i in 0..1500 {
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let len = LENS[rng.below(LENS.len())];
            let lvl = rng.range(-7, hi as i64) as c_int;
            let src = gen(shape, len, &mut rng);
            let cap = cb(src.len()) + 16;
            let mut cbuf = vec![0u8; cap];
            let mut rbuf = vec![0u8; cap];
            let cn = cc(cbuf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void,
                        src.len(), lvl);
            let rn = rc(rbuf.as_mut_ptr() as *mut c_void, cap, src.as_ptr() as *const c_void,
                        src.len(), lvl);
            let ctx = format!("#{i} shape={shape:?} len={len} lvl={lvl}");
            e.eq(&ctx, cn, rn);
            if !e.c.is_err(cn) {
                assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);
            }
        }
    }
}

/// Tight / undersized destination buffers — both must return the same
/// `dstSize_tooSmall` (or succeed identically).
#[test]
fn oneshot_tight_dst_buffers() {
    unsafe {
        let e = Err2::new();
        let (cc, rc) = both::<FnCompress>("ZSTD_compress");
        let (cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let mut rng = Rng::new(0x5eed_0004);
        for &shape in ALL_SHAPES {
            for &len in &[0usize, 1, 64, 1024, 20_000] {
                let src = gen(shape, len, &mut rng);
                // find the exact needed size using C
                let full = cb(src.len()) + 16;
                let mut tmp = vec![0u8; full];
                let need = cc(tmp.as_mut_ptr() as *mut c_void, full,
                              src.as_ptr() as *const c_void, src.len(), 3);
                if e.c.is_err(need) {
                    continue;
                }
                for cap in [0usize, 1, 2, 3, need.saturating_sub(1), need, need + 1] {
                    let mut cbuf = vec![0u8; cap.max(1)];
                    let mut rbuf = vec![0u8; cap.max(1)];
                    let cn = cc(cbuf.as_mut_ptr() as *mut c_void, cap,
                                src.as_ptr() as *const c_void, src.len(), 3);
                    let rn = rc(rbuf.as_mut_ptr() as *mut c_void, cap,
                                src.as_ptr() as *const c_void, src.len(), 3);
                    let ctx = format!("tight dst shape={shape:?} len={len} cap={cap}");
                    e.eq(&ctx, cn, rn);
                    if !e.c.is_err(cn) {
                        assert_bytes_eq(&ctx, &cbuf[..cn], &rbuf[..rn]);
                    }
                }
            }
        }
    }
}

#[test]
fn frame_info_functions() {
    unsafe {
        let e = Err2::new();
        let (cc, _) = both::<FnCompress>("ZSTD_compress");
        let (cb, _) = both::<FnCompressBound>("ZSTD_compressBound");
        let (cfcs, rfcs) = both::<FnGetFCS>("ZSTD_getFrameContentSize");
        let (cffcs, rffcs) = both::<FnFindFrameCompressedSize>("ZSTD_findFrameCompressedSize");
        let (cfdcs, rfdcs) = both::<FnGetFCS>("ZSTD_findDecompressedSize");
        let (cfhs, rfhs) = both::<FnFindFrameCompressedSize>("ZSTD_frameHeaderSize");
        let (cgds, rgds) = both::<FnGetFCS>("ZSTD_getDecompressedSize");
        type FnGetFH = unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, size_t) -> size_t;
        let (cgfh, rgfh) = both::<FnGetFH>("ZSTD_getFrameHeader");
        type FnGetFHAdv =
            unsafe extern "C" fn(*mut ZSTD_frameHeader, *const c_void, size_t, c_int) -> size_t;
        let (cgfha, rgfha) = both::<FnGetFHAdv>("ZSTD_getFrameHeader_advanced");

        let mut rng = Rng::new(0x5eed_0005);
        let mut frames: Vec<Vec<u8>> = Vec::new();
        for &shape in ALL_SHAPES {
            for &len in &[0usize, 1, 1000, 70_000] {
                let src = gen(shape, len, &mut rng);
                let mut buf = vec![0u8; cb(src.len()) + 64];
                let n = cc(buf.as_mut_ptr() as *mut c_void, buf.len(),
                           src.as_ptr() as *const c_void, src.len(), 5);
                if !e.c.is_err(n) {
                    buf.truncate(n);
                    frames.push(buf);
                }
            }
        }
        // plus a skippable frame and garbage
        let mut skip = vec![0x50u8, 0x2A, 0x4D, 0x18, 4, 0, 0, 0, 1, 2, 3, 4];
        frames.push(skip.clone());
        skip[0] = 0x5F; // another skippable magic variant
        frames.push(skip);
        frames.push(vec![]);
        frames.push(vec![0x28]);
        frames.push(vec![0x28, 0xB5, 0x2F, 0xFD]);
        frames.push((0..40).map(|_| rng.byte()).collect());

        for (fi, f) in frames.iter().enumerate() {
            // truncations exercise srcSizeWrong / short-header handling
            for cut in [f.len(), f.len() / 2, 1, 2, 3, 4, 5, 6, 8, 0] {
                if cut > f.len() {
                    continue;
                }
                let p = f.as_ptr() as *const c_void;
                let ctx = format!("frame#{fi} cut={cut}");
                assert_eq!(cfcs(p, cut), rfcs(p, cut), "getFrameContentSize {ctx}");
                assert_eq!(cgds(p, cut), rgds(p, cut), "getDecompressedSize {ctx}");
                assert_eq!(cfdcs(p, cut), rfdcs(p, cut), "findDecompressedSize {ctx}");
                e.eq(&format!("findFrameCompressedSize {ctx}"), cffcs(p, cut), rffcs(p, cut));
                e.eq(&format!("frameHeaderSize {ctx}"), cfhs(p, cut), rfhs(p, cut));

                let mut ch: ZSTD_frameHeader = std::mem::zeroed();
                let mut rh: ZSTD_frameHeader = std::mem::zeroed();
                let a = cgfh(&mut ch, p, cut);
                let b = rgfh(&mut rh, p, cut);
                e.eq(&format!("getFrameHeader {ctx}"), a, b);
                if a == 0 {
                    assert_eq!(ch, rh, "getFrameHeader struct {ctx}");
                }
                // format: 0 = f_zstd1, 1 = f_zstd1_magicless, plus out-of-range
                for fmt in [0i32, 1, 2, -1, 99] {
                    let mut ch2: ZSTD_frameHeader = std::mem::zeroed();
                    let mut rh2: ZSTD_frameHeader = std::mem::zeroed();
                    let a = cgfha(&mut ch2, p, cut, fmt);
                    let b = rgfha(&mut rh2, p, cut, fmt);
                    e.eq(&format!("getFrameHeader_advanced {ctx} fmt={fmt}"), a, b);
                    if a == 0 {
                        assert_eq!(ch2, rh2, "getFrameHeader_advanced struct {ctx} fmt={fmt}");
                    }
                }
            }
        }
    }
}

#[test]
fn magic_and_misc_predicates() {
    unsafe {
        type FnU32ToU32 = unsafe extern "C" fn(c_uint) -> c_uint;
        let (a, b) = both::<FnU32ToU32>("ZSTD_isSkippableFrame");
        // ZSTD_isSkippableFrame actually takes (buffer, size)
        let _ = (a, b);
        type FnBufSize = unsafe extern "C" fn(*const c_void, size_t) -> c_uint;
        let (cis, ris) = both::<FnBufSize>("ZSTD_isSkippableFrame");
        let mut rng = Rng::new(0x5eed_0006);
        for i in 0..3000 {
            let n = rng.below(9);
            let mut buf: Vec<u8> = (0..n).map(|_| rng.byte()).collect();
            // bias towards real magics
            if i % 3 == 0 && n >= 4 {
                let m: u32 = if i % 6 == 0 { 0x184D2A50 + (i as u32 % 16) } else { 0xFD2FB528 };
                buf[..4].copy_from_slice(&m.to_le_bytes());
            }
            let p = buf.as_ptr() as *const c_void;
            assert_eq!(cis(p, n), ris(p, n), "isSkippableFrame #{i} buf={:?}", buf);
        }
        // null pointer / zero size
        assert_eq!(cis(std::ptr::null(), 0), ris(std::ptr::null(), 0));
    }
}

#[test]
fn read_skippable_frame() {
    unsafe {
        let e = Err2::new();
        type FnWriteSkip = unsafe extern "C" fn(
            *mut c_void, size_t, *const c_void, size_t, c_uint,
        ) -> size_t;
        type FnReadSkip = unsafe extern "C" fn(
            *mut c_void, size_t, *mut c_uint, *const c_void, size_t,
        ) -> size_t;
        let (cw, rw) = both::<FnWriteSkip>("ZSTD_writeSkippableFrame");
        let (cr, rr) = both::<FnReadSkip>("ZSTD_readSkippableFrame");
        let mut rng = Rng::new(0x5eed_0007);
        for &len in &[0usize, 1, 4, 100, 5000] {
            let src = gen(Shape::Random, len, &mut rng);
            for variant in [0u32, 1, 7, 15, 16, 255, 0xFFFF_FFFF] {
                for cap in [0usize, 4, 7, 8, len + 8, len + 9] {
                    let mut cb = vec![0u8; cap.max(1)];
                    let mut rb = vec![0u8; cap.max(1)];
                    let a = cw(cb.as_mut_ptr() as *mut c_void, cap,
                               src.as_ptr() as *const c_void, src.len(), variant);
                    let b = rw(rb.as_mut_ptr() as *mut c_void, cap,
                               src.as_ptr() as *const c_void, src.len(), variant);
                    let ctx = format!("writeSkippableFrame len={len} var={variant} cap={cap}");
                    e.eq(&ctx, a, b);
                    if e.c.is_err(a) {
                        continue;
                    }
                    assert_bytes_eq(&ctx, &cb[..a], &rb[..b]);
                    // read it back with each library
                    for rcap in [0usize, 1, len.saturating_sub(1), len, len + 1] {
                        let mut o1 = vec![0u8; rcap.max(1)];
                        let mut o2 = vec![0u8; rcap.max(1)];
                        let mut v1: c_uint = 0xdead;
                        let mut v2: c_uint = 0xdead;
                        let x = cr(o1.as_mut_ptr() as *mut c_void, rcap, &mut v1,
                                   cb.as_ptr() as *const c_void, a);
                        let y = rr(o2.as_mut_ptr() as *mut c_void, rcap, &mut v2,
                                   rb.as_ptr() as *const c_void, b);
                        let ctx2 = format!("{ctx} readback rcap={rcap}");
                        e.eq(&ctx2, x, y);
                        assert_eq!(v1, v2, "{ctx2} variant out");
                        if !e.c.is_err(x) {
                            assert_bytes_eq(&ctx2, &o1[..x], &o2[..y]);
                        }
                    }
                    // NULL variant pointer is allowed
                    let mut o = vec![0u8; len.max(1)];
                    let x = cr(o.as_mut_ptr() as *mut c_void, len, std::ptr::null_mut(),
                               cb.as_ptr() as *const c_void, a);
                    let y = rr(o.as_mut_ptr() as *mut c_void, len, std::ptr::null_mut(),
                               rb.as_ptr() as *const c_void, b);
                    e.eq(&format!("{ctx} readback null-variant"), x, y);
                }
            }
        }
    }
}
