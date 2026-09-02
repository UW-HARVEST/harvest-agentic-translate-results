//! Phase C, ERRORS.md rows 20–22, 25–31, 30: NULL / zero / oversized buffers
//! and the generic decompression rejections.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_ulonglong, c_void};

type FnCompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
type FnDecompress = unsafe extern "C" fn(*mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCtxCompress =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t, c_int) -> size_t;
type FnCtxDecompress =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnCompress2 =
    unsafe extern "C" fn(*mut c_void, *mut c_void, size_t, *const c_void, size_t) -> size_t;
type FnBound = unsafe extern "C" fn(size_t) -> size_t;
type FnSetParam = unsafe extern "C" fn(*mut c_void, c_int, c_int) -> size_t;
type FnReset = unsafe extern "C" fn(*mut c_void, c_int) -> size_t;
type FnGetFCS = unsafe extern "C" fn(*const c_void, size_t) -> c_ulonglong;

fn mkframe(src: &[u8], level: c_int, checksum: c_int) -> Vec<u8> {
    unsafe {
        let e = Err2::new();
        let (cn, _) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let (cf, _) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        let (sp, _) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (c2, _) = both::<FnCompress2>("ZSTD_compress2");
        let (bnd, _) = both::<FnBound>("ZSTD_compressBound");
        let cc = cn();
        sp(cc, ZSTD_c_compressionLevel, level);
        sp(cc, ZSTD_c_checksumFlag, checksum);
        let cap = bnd(src.len()) + 64;
        let mut o = vec![0u8; cap];
        let s = if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
        let n = c2(cc, o.as_mut_ptr() as *mut c_void, cap, s, src.len());
        assert!(!e.c.is_err(n));
        cf(cc);
        o.truncate(n);
        o
    }
}

/// ERRORS rows 20–21, 29–30: NULL destination pointers and zero-capacity
/// destinations on both compression and decompression.
#[test]
fn null_and_zero_dst() {
    unsafe {
        let e = Err2::new();
        let (cc, rc) = both::<FnCompress>("ZSTD_compress");
        let (cd, rd) = both::<FnDecompress>("ZSTD_decompress");
        let mut rng = Rng::new(0xC201);
        for &shape in ALL_SHAPES {
            for &len in &[0usize, 1, 100, 5000] {
                let src = gen(shape, len, &mut rng);
                let sp =
                    if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void };
                for lvl in [1i32, 3, 19] {
                    // dst == NULL with capacity 0
                    let a = cc(std::ptr::null_mut(), 0, sp, src.len(), lvl);
                    let b = rc(std::ptr::null_mut(), 0, sp, src.len(), lvl);
                    e.eq(&format!("compress null/0 shape={shape:?} len={} lvl={lvl}", src.len()),
                         a, b);
                    // valid dst pointer, capacity 0
                    let mut one = [0u8; 1];
                    let a = cc(one.as_mut_ptr() as *mut c_void, 0, sp, src.len(), lvl);
                    let b = rc(one.as_mut_ptr() as *mut c_void, 0, sp, src.len(), lvl);
                    e.eq(&format!("compress dst/0 shape={shape:?} len={} lvl={lvl}", src.len()),
                         a, b);
                }
                let frame = mkframe(&src, 3, 0);
                let a = cd(std::ptr::null_mut(), 0, frame.as_ptr() as *const c_void, frame.len());
                let b = rd(std::ptr::null_mut(), 0, frame.as_ptr() as *const c_void, frame.len());
                e.eq(&format!("decompress null/0 shape={shape:?} len={}", src.len()), a, b);
                let mut one = [0u8; 1];
                let a = cd(one.as_mut_ptr() as *mut c_void, 0,
                           frame.as_ptr() as *const c_void, frame.len());
                let b = rd(one.as_mut_ptr() as *mut c_void, 0,
                           frame.as_ptr() as *const c_void, frame.len());
                e.eq(&format!("decompress dst/0 shape={shape:?} len={}", src.len()), a, b);
            }
        }
    }
}

/// ERRORS row 22: NULL source with a zero size (well-defined), on every
/// one-shot entry point. A NULL source with a NON-zero size is undefined
/// behaviour in the C (it dereferences without a guard), so it is not compared.
#[test]
fn null_src_zero_size() {
    unsafe {
        let e = Err2::new();
        let (cc, rc) = both::<FnCompress>("ZSTD_compress");
        let (cd, rd) = both::<FnDecompress>("ZSTD_decompress");
        let (ccn, rcn) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let (cdn, rdn) = both::<FnVoidToPtr>("ZSTD_createDCtx");
        let (ccc, rcc) = both::<FnCtxCompress>("ZSTD_compressCCtx");
        let (cdd, rdd) = both::<FnCtxDecompress>("ZSTD_decompressDCtx");
        let (cc2, rc2) = both::<FnCompress2>("ZSTD_compress2");
        let cctx_c = ccn();
        let cctx_r = rcn();
        let dctx_c = cdn();
        let dctx_r = rdn();
        let mut out = vec![0u8; 4096];
        for lvl in [-5i32, 0, 1, 3, 22] {
            let a = cc(out.as_mut_ptr() as *mut c_void, out.len(), std::ptr::null(), 0, lvl);
            let b = rc(out.as_mut_ptr() as *mut c_void, out.len(), std::ptr::null(), 0, lvl);
            e.eq(&format!("compress null-src lvl={lvl}"), a, b);
            let a = ccc(cctx_c, out.as_mut_ptr() as *mut c_void, out.len(), std::ptr::null(), 0, lvl);
            let b = rcc(cctx_r, out.as_mut_ptr() as *mut c_void, out.len(), std::ptr::null(), 0, lvl);
            e.eq(&format!("compressCCtx null-src lvl={lvl}"), a, b);
        }
        let a = cc2(cctx_c, out.as_mut_ptr() as *mut c_void, out.len(), std::ptr::null(), 0);
        let b = rc2(cctx_r, out.as_mut_ptr() as *mut c_void, out.len(), std::ptr::null(), 0);
        e.eq("compress2 null-src", a, b);
        let a = cd(out.as_mut_ptr() as *mut c_void, out.len(), std::ptr::null(), 0);
        let b = rd(out.as_mut_ptr() as *mut c_void, out.len(), std::ptr::null(), 0);
        e.eq("decompress null-src", a, b);
        let a = cdd(dctx_c, out.as_mut_ptr() as *mut c_void, out.len(), std::ptr::null(), 0);
        let b = rdd(dctx_r, out.as_mut_ptr() as *mut c_void, out.len(), std::ptr::null(), 0);
        e.eq("decompressDCtx null-src", a, b);
        let (cf, rf) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        cf(cctx_c);
        rf(cctx_r);
        let (cdf, rdf) = both::<FnPtrToSize>("ZSTD_freeDCtx");
        cdf(dctx_c);
        rdf(dctx_r);
    }
}

/// ERRORS row 25: `srcSize == 0` on every decompression entry point.
#[test]
fn empty_src() {
    unsafe {
        let e = Err2::new();
        let (cd, rd) = both::<FnDecompress>("ZSTD_decompress");
        let (cdn, rdn) = both::<FnVoidToPtr>("ZSTD_createDCtx");
        let (cdd, rdd) = both::<FnCtxDecompress>("ZSTD_decompressDCtx");
        let (cfcs, rfcs) = both::<FnGetFCS>("ZSTD_getFrameContentSize");
        let (cffcs, rffcs) = both::<unsafe extern "C" fn(*const c_void, size_t) -> size_t>(
            "ZSTD_findFrameCompressedSize",
        );
        let d1 = cdn();
        let d2 = rdn();
        let mut out = vec![0u8; 4096];
        let dummy = [0u8; 1];
        for srcptr in [std::ptr::null(), dummy.as_ptr() as *const c_void] {
            e.eq("decompress srcSize=0",
                 cd(out.as_mut_ptr() as *mut c_void, out.len(), srcptr, 0),
                 rd(out.as_mut_ptr() as *mut c_void, out.len(), srcptr, 0));
            e.eq("decompressDCtx srcSize=0",
                 cdd(d1, out.as_mut_ptr() as *mut c_void, out.len(), srcptr, 0),
                 rdd(d2, out.as_mut_ptr() as *mut c_void, out.len(), srcptr, 0));
            assert_eq!(cfcs(srcptr, 0), rfcs(srcptr, 0), "getFrameContentSize srcSize=0");
            e.eq("findFrameCompressedSize srcSize=0", cffcs(srcptr, 0), rffcs(srcptr, 0));
        }
        let (cf, rf) = both::<FnPtrToSize>("ZSTD_freeDCtx");
        cf(d1);
        rf(d2);
    }
}

/// ERRORS row 26: garbage / unknown magic. Exhaustive over the first 4 bytes'
/// neighbourhood plus thousands of random buffers.
#[test]
fn bad_magic() {
    unsafe {
        let e = Err2::new();
        let (cd, rd) = both::<FnDecompress>("ZSTD_decompress");
        let (cif, rif) = both::<unsafe extern "C" fn(*const c_void, size_t) -> std::os::raw::c_uint>(
            "ZSTD_isFrame",
        );
        let (cfcs, rfcs) = both::<FnGetFCS>("ZSTD_getFrameContentSize");
        let mut rng = Rng::new(0xC202);
        let mut out = vec![0u8; 1 << 16];

        // sweep every value of the low magic byte and every skippable variant
        let mut cases: Vec<Vec<u8>> = Vec::new();
        for b0 in 0u32..=0xFFu32 {
            let mut v = vec![0u8; 32];
            v[0] = b0 as u8;
            v[1] = 0xB5;
            v[2] = 0x2F;
            v[3] = 0xFD;
            cases.push(v);
        }
        for m in [
            0xFD2FB528u32, 0xFD2FB527, 0xFD2FB526, 0xFD2FB525, 0xFD2FB524, 0xFD2FB523,
            0xFD2FB522, 0xFD2FB51E, 0x184D2A50, 0x184D2A5F, 0x184D2A60, 0x184D2A4F, 0,
            0xFFFFFFFF,
        ] {
            for extra in [0usize, 4, 12, 40] {
                let mut v = m.to_le_bytes().to_vec();
                v.extend((0..extra).map(|_| rng.byte()));
                cases.push(v);
            }
        }
        for _ in 0..5000 {
            let n = rng.below(48);
            cases.push((0..n).map(|_| rng.byte()).collect());
        }
        for (i, c) in cases.iter().enumerate() {
            let p = if c.is_empty() { std::ptr::null() } else { c.as_ptr() as *const c_void };
            e.eq(&format!("decompress garbage #{i} ({})", hexdump(c, 16)),
                 cd(out.as_mut_ptr() as *mut c_void, out.len(), p, c.len()),
                 rd(out.as_mut_ptr() as *mut c_void, out.len(), p, c.len()));
            assert_eq!(cif(p, c.len()), rif(p, c.len()), "isFrame #{i}");
            assert_eq!(cfcs(p, c.len()), rfcs(p, c.len()), "getFrameContentSize #{i}");
        }
    }
}

/// ERRORS rows 27–28: exhaustive truncation of real frames, at the header and
/// in the body.
#[test]
fn truncated_frames() {
    unsafe {
        let e = Err2::new();
        let (cd, rd) = both::<FnDecompress>("ZSTD_decompress");
        let (cffcs, rffcs) = both::<unsafe extern "C" fn(*const c_void, size_t) -> size_t>(
            "ZSTD_findFrameCompressedSize",
        );
        let (cfcs, rfcs) = both::<FnGetFCS>("ZSTD_getFrameContentSize");
        let (cdb, rdb) = both::<FnGetFCS>("ZSTD_decompressBound");
        let mut rng = Rng::new(0xC203);
        let mut out = vec![0u8; 1 << 18];
        for &shape in ALL_SHAPES {
            for &len in &[0usize, 1, 100, 5000] {
                for &ck in &[0i32, 1] {
                    let src = gen(shape, len, &mut rng);
                    let frame = mkframe(&src, 5, ck);
                    for cut in 0..=frame.len() {
                        let p = frame.as_ptr() as *const c_void;
                        let ctx = format!("{shape:?} len={} ck={ck} cut={cut}", src.len());
                        e.eq(&format!("decompress {ctx}"),
                             cd(out.as_mut_ptr() as *mut c_void, out.len(), p, cut),
                             rd(out.as_mut_ptr() as *mut c_void, out.len(), p, cut));
                        e.eq(&format!("findFrameCompressedSize {ctx}"),
                             cffcs(p, cut), rffcs(p, cut));
                        assert_eq!(cfcs(p, cut), rfcs(p, cut), "getFrameContentSize {ctx}");
                        assert_eq!(cdb(p, cut), rdb(p, cut), "decompressBound {ctx}");
                    }
                }
            }
        }
    }
}

/// ERRORS row 29: destination capacity sweep on decompression.
#[test]
fn dst_too_small() {
    unsafe {
        let e = Err2::new();
        let (cd, rd) = both::<FnDecompress>("ZSTD_decompress");
        let mut rng = Rng::new(0xC204);
        for &shape in ALL_SHAPES {
            for &len in &[0usize, 1, 2, 100, 5000, 70_000] {
                let src = gen(shape, len, &mut rng);
                let frame = mkframe(&src, 3, 0);
                let n = src.len();
                for cap in [
                    0usize, 1, 2, 3, n / 2, n.saturating_sub(2), n.saturating_sub(1), n, n + 1,
                ] {
                    let mut o1 = vec![0u8; cap.max(1)];
                    let mut o2 = vec![0u8; cap.max(1)];
                    let a = cd(o1.as_mut_ptr() as *mut c_void, cap,
                               frame.as_ptr() as *const c_void, frame.len());
                    let b = rd(o2.as_mut_ptr() as *mut c_void, cap,
                               frame.as_ptr() as *const c_void, frame.len());
                    let ctx = format!("{shape:?} len={n} cap={cap}");
                    e.eq(&ctx, a, b);
                    if !e.c.is_err(a) {
                        assert_bytes_eq(&ctx, &o1[..a], &o2[..b]);
                    }
                }
            }
        }
    }
}

/// ERRORS row 31 + CONFIGS row 99: trailing garbage, concatenated frames,
/// interleaved skippable frames.
#[test]
fn multi_frame_and_trailing_garbage() {
    unsafe {
        let e = Err2::new();
        let (cd, rd) = both::<FnDecompress>("ZSTD_decompress");
        let (cfds, rfds) = both::<FnGetFCS>("ZSTD_findDecompressedSize");
        let (cdb, rdb) = both::<FnGetFCS>("ZSTD_decompressBound");
        let (cffcs, rffcs) = both::<unsafe extern "C" fn(*const c_void, size_t) -> size_t>(
            "ZSTD_findFrameCompressedSize",
        );
        let mut rng = Rng::new(0xC205);
        let mut out = vec![0u8; 1 << 20];

        // build a pool of pieces
        let mut pieces: Vec<Vec<u8>> = Vec::new();
        for &shape in &[Shape::Text, Shape::Random, Shape::Zeros, Shape::Empty] {
            for &len in &[0usize, 1, 500, 20_000] {
                let src = gen(shape, len, &mut rng);
                pieces.push(mkframe(&src, 3, 0));
                pieces.push(mkframe(&src, 3, 1));
            }
        }
        // skippable frames
        for variant in 0u32..16 {
            let payload: Vec<u8> = (0..rng.below(40)).map(|_| rng.byte()).collect();
            let mut v = (0x184D2A50u32 + variant).to_le_bytes().to_vec();
            v.extend((payload.len() as u32).to_le_bytes());
            v.extend_from_slice(&payload);
            pieces.push(v);
        }

        for i in 0..3000 {
            let n = 1 + rng.below(5);
            let mut buf = Vec::new();
            for _ in 0..n {
                buf.extend_from_slice(&pieces[rng.below(pieces.len())]);
            }
            // sometimes append trailing garbage
            match i % 4 {
                1 => buf.extend((0..1 + rng.below(8)).map(|_| rng.byte())),
                2 => buf.extend_from_slice(&[0x28, 0xB5, 0x2F]),
                3 => buf.push(0),
                _ => {}
            }
            let p = buf.as_ptr() as *const c_void;
            let ctx = format!("multi#{i} nframes={n} len={}", buf.len());
            e.eq(&format!("decompress {ctx}"),
                 cd(out.as_mut_ptr() as *mut c_void, out.len(), p, buf.len()),
                 rd(out.as_mut_ptr() as *mut c_void, out.len(), p, buf.len()));
            assert_eq!(cfds(p, buf.len()), rfds(p, buf.len()), "findDecompressedSize {ctx}");
            assert_eq!(cdb(p, buf.len()), rdb(p, buf.len()), "decompressBound {ctx}");
            e.eq(&format!("findFrameCompressedSize {ctx}"),
                 cffcs(p, buf.len()), rffcs(p, buf.len()));
            // and every truncation of the first 24 bytes
            for cut in 0..24.min(buf.len()) {
                e.eq(&format!("decompress {ctx} cut={cut}"),
                     cd(out.as_mut_ptr() as *mut c_void, out.len(), p, cut),
                     rd(out.as_mut_ptr() as *mut c_void, out.len(), p, cut));
            }
        }
    }
}

/// ERRORS row 33: frames whose window exceeds `ZSTD_d_windowLogMax`.
#[test]
fn window_too_large() {
    unsafe {
        let e = Err2::new();
        let (cdn, rdn) = both::<FnVoidToPtr>("ZSTD_createDCtx");
        let (cdf, rdf) = both::<FnPtrToSize>("ZSTD_freeDCtx");
        let (cds, rds) = both::<FnSetParam>("ZSTD_DCtx_setParameter");
        let (crs, rrs) = both::<FnReset>("ZSTD_DCtx_reset");
        let (cdd, rdd) = both::<FnCtxDecompress>("ZSTD_decompressDCtx");
        let (cmw, rmw) = both::<unsafe extern "C" fn(*mut c_void, size_t) -> size_t>(
            "ZSTD_DCtx_setMaxWindowSize",
        );
        let (ccn, _) = both::<FnVoidToPtr>("ZSTD_createCCtx");
        let (ccf, _) = both::<FnPtrToSize>("ZSTD_freeCCtx");
        let (csp, _) = both::<FnSetParam>("ZSTD_CCtx_setParameter");
        let (crst, _) = both::<FnReset>("ZSTD_CCtx_reset");
        let (cc2, _) = both::<FnCompress2>("ZSTD_compress2");
        let (bnd, _) = both::<FnBound>("ZSTD_compressBound");

        let mut rng = Rng::new(0xC206);
        let src = gen(Shape::Text, 300_000, &mut rng);
        let cctx = ccn();
        let d1 = cdn();
        let d2 = rdn();
        let mut out = vec![0u8; src.len() + 64];

        for wl in [10i32, 15, 18, 20, 23, 25, 27, 28, 30, 31] {
            crst(cctx, ZSTD_reset_session_and_parameters);
            if csp(cctx, ZSTD_c_windowLog, wl) > usize::MAX / 2 {
                continue;
            }
            let cap = bnd(src.len()) + 64;
            let mut fr = vec![0u8; cap];
            let n = cc2(cctx, fr.as_mut_ptr() as *mut c_void, cap,
                        src.as_ptr() as *const c_void, src.len());
            if e.c.is_err(n) {
                continue;
            }
            fr.truncate(n);
            for wlm in [10i32, 15, 17, 18, 20, 23, 25, 27, 28, 30, 31] {
                crs(d1, ZSTD_reset_session_and_parameters);
                rrs(d2, ZSTD_reset_session_and_parameters);
                let a = cds(d1, ZSTD_d_windowLogMax, wlm);
                let b = rds(d2, ZSTD_d_windowLogMax, wlm);
                e.eq(&format!("set windowLogMax={wlm}"), a, b);
                if e.c.is_err(a) {
                    continue;
                }
                let x = cdd(d1, out.as_mut_ptr() as *mut c_void, out.len(),
                            fr.as_ptr() as *const c_void, fr.len());
                let y = rdd(d2, out.as_mut_ptr() as *mut c_void, out.len(),
                            fr.as_ptr() as *const c_void, fr.len());
                e.eq(&format!("decompress wl={wl} windowLogMax={wlm}"), x, y);
            }
            // the deprecated setMaxWindowSize path
            for ws in [0usize, 1, 1 << 10, 1 << 17, 1 << 20, 1 << 27, 1usize << 31, usize::MAX] {
                crs(d1, ZSTD_reset_session_and_parameters);
                rrs(d2, ZSTD_reset_session_and_parameters);
                e.eq(&format!("setMaxWindowSize({ws})"), cmw(d1, ws), rmw(d2, ws));
                let x = cdd(d1, out.as_mut_ptr() as *mut c_void, out.len(),
                            fr.as_ptr() as *const c_void, fr.len());
                let y = rdd(d2, out.as_mut_ptr() as *mut c_void, out.len(),
                            fr.as_ptr() as *const c_void, fr.len());
                e.eq(&format!("decompress wl={wl} maxWindowSize={ws}"), x, y);
            }
        }
        ccf(cctx);
        cdf(d1);
        rdf(d2);
    }
}
