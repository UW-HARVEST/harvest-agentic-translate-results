//! Phase B — CONFIGS.md group 10: the `lz4frame` decompression state machine,
//! driven with whole-frame, 1-byte and random src/dst chunking, every option,
//! skippable frames, and dictionaries.
mod common;
use common::frame::*;
use common::*;

const SEED: u64 = 0x46_5241_4D45_0010;

fn is_err(v: usize) -> bool {
    v > (0usize.wrapping_sub(24))
}

struct Dctx {
    c: *mut u8,
    r: *mut u8,
    free: (FnFreeDctx, FnFreeDctx),
}
impl Drop for Dctx {
    fn drop(&mut self) {
        unsafe {
            (self.free.0)(self.c);
            (self.free.1)(self.r);
        }
    }
}
fn new_dctx() -> Dctx {
    let (cc, rc) = syms::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let free = syms::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let mut c: *mut u8 = std::ptr::null_mut();
    let mut r: *mut u8 = std::ptr::null_mut();
    let a = unsafe { cc(&mut c, LZ4F_VERSION) };
    let b = unsafe { rc(&mut r, LZ4F_VERSION) };
    assert_eq!(a, b, "createDecompressionContext");
    assert!(!c.is_null() && !r.is_null());
    Dctx { c, r, free }
}

/// Build a frame with the C library.
fn c_frame(src: &[u8], p: *const Prefs) -> Vec<u8> {
    let (cf, _) = syms::<FnCompressFrame>("LZ4F_compressFrame");
    let (cfb, _) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let bound = unsafe { cfb(src.len(), p) };
    let mut out = vec![0u8; bound];
    let n = unsafe { cf(out.as_mut_ptr(), bound, src.as_ptr(), src.len(), p) };
    assert!(!is_err(n), "c_frame failed: {n:#x}");
    out.truncate(n);
    out
}

#[derive(Copy, Clone, Debug)]
enum Chunk {
    Whole,
    One,
    Random,
    Fixed(usize),
}

/// Feed `frame` to both decoders with the given src/dst chunking and compare
/// every return value, every `*srcSizePtr`/`*dstSizePtr` and the payload.
#[allow(clippy::too_many_arguments)]
fn decode_and_compare(
    label: &str,
    frame: &[u8],
    expect: &[u8],
    src_chunk: Chunk,
    dst_chunk: Chunk,
    dopt: *const DecompressOptions,
    dict: Option<&[u8]>,
    rng: &mut Rng,
) {
    let (cd, rd) = syms::<FnDecompress>("LZ4F_decompress");
    let (cdd, rdd) = syms::<FnDecompressUsingDict>("LZ4F_decompress_usingDict");
    let ctx = new_dctx();

    // `stableDst` pledges that the previously decoded 64 KB sits immediately
    // before `dstBuffer` (lz4frame.h:371). Honouring that means writing into ONE
    // persistent buffer and advancing the pointer, not handing the decoder a
    // fresh allocation on every call -- otherwise BOTH libraries produce the
    // same wrong bytes and the test proves nothing.
    let stable = !dopt.is_null() && unsafe { (*dopt).stable_dst } != 0;
    let mut cstable = vec![0u8; expect.len() + 65536 + 16];
    let mut rstable = vec![0u8; expect.len() + 65536 + 16];

    let mut cpay: Vec<u8> = Vec::new();
    let mut rpay: Vec<u8> = Vec::new();
    let mut src_off = 0usize;
    let mut guard = 0usize;

    loop {
        guard += 1;
        assert!(guard < 4_000_000, "{label}: decode did not terminate");
        let avail = frame.len() - src_off;
        let want_src = match src_chunk {
            Chunk::Whole => avail,
            Chunk::One => 1.min(avail),
            Chunk::Random => rng.range(1, 4096).min(avail),
            Chunk::Fixed(k) => k.min(avail),
        };
        let dcap = match dst_chunk {
            Chunk::Whole => (expect.len() + 16).max(1),
            Chunk::One => 1,
            Chunk::Random => rng.range(1, 8192),
            Chunk::Fixed(k) => k.max(1),
        };

        let mut scratch_c = vec![0u8; dcap];
        let mut scratch_r = vec![0u8; dcap];
        let dcap = if stable {
            dcap.min(cstable.len() - cpay.len())
        } else {
            dcap
        };
        let (cdst, rdst) = if stable {
            unsafe {
                (
                    cstable.as_mut_ptr().add(cpay.len()),
                    rstable.as_mut_ptr().add(rpay.len()),
                )
            }
        } else {
            (scratch_c.as_mut_ptr(), scratch_r.as_mut_ptr())
        };
        let mut cds = dcap;
        let mut rds = dcap;
        let mut css = want_src;
        let mut rss = want_src;
        let sp = unsafe { frame.as_ptr().add(src_off) };

        let (cn, rn) = match dict {
            None => (
                unsafe { cd(ctx.c, cdst, &mut cds, sp, &mut css, dopt) },
                unsafe { rd(ctx.r, rdst, &mut rds, sp, &mut rss, dopt) },
            ),
            Some(d) => (
                unsafe { cdd(ctx.c, cdst, &mut cds, sp, &mut css, d.as_ptr(), d.len(), dopt) },
                unsafe { rdd(ctx.r, rdst, &mut rds, sp, &mut rss, d.as_ptr(), d.len(), dopt) },
            ),
        };
        assert_eq!(cn, rn, "{label}: decompress hint C={cn:#x} R={rn:#x} (srcOff={src_off})");
        assert_eq!(css, rss, "{label}: *srcSizePtr C={css} R={rss} (srcOff={src_off})");
        assert_eq!(cds, rds, "{label}: *dstSizePtr C={cds} R={rds} (srcOff={src_off})");
        if is_err(cn) {
            return;
        }
        let cchunk: Vec<u8> = unsafe { std::slice::from_raw_parts(cdst, cds) }.to_vec();
        let rchunk: Vec<u8> = unsafe { std::slice::from_raw_parts(rdst, rds) }.to_vec();
        assert_eq!(cchunk, rchunk, "{label}: payload chunk differs");
        cpay.extend_from_slice(&cchunk);
        rpay.extend_from_slice(&rchunk);
        let _ = (&mut scratch_c, &mut scratch_r);
        src_off += css;

        if cn == 0 {
            break; // frame complete
        }
        if css == 0 && cds == 0 {
            // no progress possible
            assert_eq!(src_off, frame.len(), "{label}: stalled before end of frame");
            break;
        }
        if src_off >= frame.len() && css == 0 && cds == 0 {
            break;
        }
    }
    assert_eq!(cpay, rpay, "{label}: decoded payload differs");
    assert_eq!(cpay, expect, "{label}: decoded payload != original");
}

/// Rows 165-167: `LZ4F_headerSize` and `LZ4F_getFrameInfo`.
#[test]
fn g10_header_size_and_frame_info() {
    let (chs, rhs) = syms::<FnHeaderSize>("LZ4F_headerSize");
    let (cgi, rgi) = syms::<FnGetFrameInfo>("LZ4F_getFrameInfo");
    let (cdc, rdc) = syms::<FnDecompress>("LZ4F_decompress");
    let mut rng = Rng::new(SEED);

    for (desc, p) in prefs_matrix().iter() {
        let len = rng.range(0, 5000);
        let src = mkdata(Shape::Textish, len, &mut rng);
        let mut pp = *p;
        pp.frame_info.content_size = if rng.next_u32() % 2 == 0 { len as u64 } else { 0 };
        let frame = c_frame(&src, &pp);

        // headerSize over every prefix length
        for n in 0..=frame.len().min(24) {
            let a = unsafe { chs(frame.as_ptr(), n) };
            let b = unsafe { rhs(frame.as_ptr(), n) };
            assert_eq!(a, b, "headerSize(n={n}) [{desc}]: C={a:#x} R={b:#x}");
        }

        // getFrameInfo before decoding, with exactly-enough and not-enough src
        for n in [0usize, 1, 4, 5, 6, 7, 15, 19, frame.len()] {
            let n = n.min(frame.len());
            let ctx = new_dctx();
            let mut cfi = FrameInfo::default();
            let mut rfi = FrameInfo::default();
            let mut cs = n;
            let mut rs = n;
            let a = unsafe { cgi(ctx.c, &mut cfi, frame.as_ptr(), &mut cs) };
            let b = unsafe { rgi(ctx.r, &mut rfi, frame.as_ptr(), &mut rs) };
            assert_eq!(a, b, "getFrameInfo(n={n}) [{desc}]: C={a:#x} R={b:#x}");
            assert_eq!(cs, rs, "getFrameInfo(n={n}) *srcSizePtr [{desc}]");
            if !is_err(a) {
                assert_eq!(cfi, rfi, "getFrameInfo(n={n}) frameInfo [{desc}]");
            }
        }

        // Row 167: getFrameInfo mid-frame (after some decoding)
        if frame.len() > 25 && len > 0 {
            let ctx = new_dctx();
            let mut cbuf = vec![0u8; len + 16];
            let mut rbuf = vec![0u8; len + 16];
            let mut cds = 4usize.min(len);
            let mut rds = cds;
            let mut css = frame.len().min(25);
            let mut rss = css;
            unsafe { cdc(ctx.c, cbuf.as_mut_ptr(), &mut cds, frame.as_ptr(), &mut css, std::ptr::null()) };
            unsafe { rdc(ctx.r, rbuf.as_mut_ptr(), &mut rds, frame.as_ptr(), &mut rss, std::ptr::null()) };
            let mut cfi = FrameInfo::default();
            let mut rfi = FrameInfo::default();
            let mut cs = frame.len() - css;
            let mut rs = frame.len() - rss;
            let a = unsafe { cgi(ctx.c, &mut cfi, frame.as_ptr().add(css), &mut cs) };
            let b = unsafe { rgi(ctx.r, &mut rfi, frame.as_ptr().add(rss), &mut rs) };
            assert_eq!(a, b, "getFrameInfo mid-frame [{desc}]: C={a:#x} R={b:#x}");
            if !is_err(a) {
                assert_eq!(cfi, rfi, "getFrameInfo mid-frame info [{desc}]");
            }
        }
    }
}

/// Rows 168-175: decode every preference combination with whole-frame,
/// 1-byte and random src/dst chunking.
#[test]
fn g10_decompress_chunking() {
    let mut rng = Rng::new(SEED ^ 1);
    for (desc, p) in prefs_matrix_small().iter() {
        for &len in &[0usize, 1, 13, 1000, 70000, 300000] {
            for &declared in &[false, true] {
                let mut pp = *p;
                if declared {
                    pp.frame_info.content_size = len as u64;
                }
                let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                let src = mkdata(shape, len, &mut rng);
                let frame = c_frame(&src, &pp);
                let combos: &[(Chunk, Chunk)] = if len <= 5000 {
                    &[
                        (Chunk::Whole, Chunk::Whole),
                        (Chunk::Whole, Chunk::One),
                        (Chunk::One, Chunk::Whole),
                        (Chunk::One, Chunk::One),
                        (Chunk::Random, Chunk::Random),
                        (Chunk::Fixed(19), Chunk::Fixed(7)),
                    ]
                } else {
                    &[
                        (Chunk::Whole, Chunk::Whole),
                        (Chunk::Random, Chunk::Random),
                        (Chunk::Fixed(4096), Chunk::Fixed(1000)),
                        (Chunk::Whole, Chunk::Fixed(1)),
                    ]
                };
                for &(sc, dc) in combos {
                    decode_and_compare(
                        &format!(
                            "decode [{desc}] len={len} declared={declared} {shape:?} src={sc:?} dst={dc:?}"
                        ),
                        &frame,
                        &src,
                        sc,
                        dc,
                        std::ptr::null(),
                        None,
                        &mut rng,
                    );
                }
            }
        }
    }
}

/// Row 174: a frame built from uncompressed (stored) blocks.
#[test]
fn g10_decompress_stored_blocks() {
    let (cbg, _) = syms::<FnCompressBegin>("LZ4F_compressBegin");
    let (cuu, _) = syms::<FnCompressUpdate>("LZ4F_uncompressedUpdate");
    let (cend, _) = syms::<FnFlush>("LZ4F_compressEnd");
    let (cbnd, _) = syms::<FnFrameBound>("LZ4F_compressBound");
    let (ccc, _) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cfc, _) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let mut rng = Rng::new(SEED ^ 2);

    for &bsid in &[0i32, 4, 5] {
        for &cck in &[0i32, 1] {
            for &bck in &[0i32, 1] {
                let mut p = Prefs::default();
                p.frame_info.block_size_id = bsid;
                p.frame_info.block_mode = 1; // uncompressedUpdate requires independent blocks
                p.frame_info.content_checksum_flag = cck;
                p.frame_info.block_checksum_flag = bck;

                for &len in &[0usize, 1, 100, 70000, 300000] {
                    let src = mkdata(Shape::Random, len, &mut rng);
                    // build the frame with C, all-stored blocks
                    let mut cctx: *mut u8 = std::ptr::null_mut();
                    unsafe { ccc(&mut cctx, LZ4F_VERSION) };
                    let mut frame: Vec<u8> = Vec::new();
                    let mut h = vec![0u8; 64];
                    let n = unsafe { cbg(cctx, h.as_mut_ptr(), 64, &p) };
                    frame.extend_from_slice(&h[..n]);
                    let mut off = 0usize;
                    while off < len {
                        let k = rng.range(1, 40000).min(len - off);
                        let cap = unsafe { cbnd(k, &p) }.max(k + 5_000_000 / 64);
                        let mut d = vec![0u8; cap];
                        let m = unsafe {
                            cuu(cctx, d.as_mut_ptr(), cap, src.as_ptr().add(off), k, std::ptr::null())
                        };
                        assert!(!is_err(m), "uncompressedUpdate failed {m:#x}");
                        frame.extend_from_slice(&d[..m]);
                        off += k;
                    }
                    let cap = unsafe { cbnd(0, &p) }.max(16);
                    let mut d = vec![0u8; cap];
                    let m = unsafe { cend(cctx, d.as_mut_ptr(), cap, std::ptr::null()) };
                    frame.extend_from_slice(&d[..m]);
                    unsafe { cfc(cctx) };

                    for &(sc, dc) in &[
                        (Chunk::Whole, Chunk::Whole),
                        (Chunk::Random, Chunk::Random),
                        (Chunk::Fixed(3), Chunk::Fixed(5)),
                    ] {
                        decode_and_compare(
                            &format!("stored bsid={bsid} cck={cck} bck={bck} len={len} {sc:?}/{dc:?}"),
                            &frame,
                            &src,
                            sc,
                            dc,
                            std::ptr::null(),
                            None,
                            &mut rng,
                        );
                    }
                }
            }
        }
    }
}

/// Rows 176-177: skippable frames.
#[test]
fn g10_skippable_frames() {
    let (cd, rd) = syms::<FnDecompress>("LZ4F_decompress");
    let (chs, rhs) = syms::<FnHeaderSize>("LZ4F_headerSize");
    let mut rng = Rng::new(SEED ^ 3);

    for magic_low in [0u32, 1, 7, 0xF] {
        for &skip_len in &[0u32, 1, 4, 100, 70000] {
            let mut frame: Vec<u8> = Vec::new();
            frame.extend_from_slice(&(0x184D2A50u32 | magic_low).to_le_bytes());
            frame.extend_from_slice(&skip_len.to_le_bytes());
            frame.extend(mkdata(Shape::Random, skip_len as usize, &mut rng));

            let a = unsafe { chs(frame.as_ptr(), frame.len().min(8)) };
            let b = unsafe { rhs(frame.as_ptr(), frame.len().min(8)) };
            assert_eq!(a, b, "headerSize(skippable magic {magic_low:#x})");

            // decode the skippable frame alone
            for &(sc, dc) in &[(Chunk::Whole, Chunk::Whole), (Chunk::Fixed(1), Chunk::Fixed(1)), (Chunk::Random, Chunk::Random)] {
                decode_and_compare(
                    &format!("skippable magic={magic_low:#x} len={skip_len} {sc:?}/{dc:?}"),
                    &frame,
                    &[],
                    sc,
                    dc,
                    std::ptr::null(),
                    None,
                    &mut rng,
                );
            }

            // Row 177: skippable frame followed by a real frame
            let payload = mkdata(Shape::Textish, 3000, &mut rng);
            let mut p = Prefs::default();
            p.frame_info.content_checksum_flag = 1;
            p.frame_info.block_checksum_flag = 1;
            let real = c_frame(&payload, &p);
            let mut combined = frame.clone();
            combined.extend_from_slice(&real);

            // Feed both frames through one dctx; the C returns 0 after the
            // skippable frame, so drive it manually here.
            let ctx = new_dctx();
            let mut off = 0usize;
            let mut cpay: Vec<u8> = Vec::new();
            let mut rpay: Vec<u8> = Vec::new();
            let mut guard = 0;
            while off < combined.len() {
                guard += 1;
                assert!(guard < 100000);
                let mut cbuf = vec![0u8; 4096];
                let mut rbuf = vec![0u8; 4096];
                let mut cds = cbuf.len();
                let mut rds = rbuf.len();
                let mut css = combined.len() - off;
                let mut rss = css;
                let x = unsafe {
                    cd(ctx.c, cbuf.as_mut_ptr(), &mut cds, combined.as_ptr().add(off), &mut css, std::ptr::null())
                };
                let y = unsafe {
                    rd(ctx.r, rbuf.as_mut_ptr(), &mut rds, combined.as_ptr().add(off), &mut rss, std::ptr::null())
                };
                assert_eq!(x, y, "skip+real hint at off={off}");
                assert_eq!(css, rss, "skip+real *srcSizePtr at off={off}");
                assert_eq!(cds, rds, "skip+real *dstSizePtr at off={off}");
                if is_err(x) {
                    break;
                }
                assert_eq!(&cbuf[..cds], &rbuf[..rds], "skip+real payload chunk");
                cpay.extend_from_slice(&cbuf[..cds]);
                rpay.extend_from_slice(&rbuf[..rds]);
                if css == 0 && cds == 0 {
                    break;
                }
                off += css;
            }
            assert_eq!(cpay, rpay, "skip+real payload");
            assert_eq!(cpay, payload, "skip+real payload != original");
        }
    }
}

/// Rows 178-181: `stableDst`, `skipChecksums` (including over a corrupted
/// checksum), and a NULL options pointer.
#[test]
fn g10_decompress_options() {
    let mut rng = Rng::new(SEED ^ 4);
    for (desc, p) in prefs_matrix_small().iter().step_by(2) {
        for &stable_dst in &[0u32, 1] {
            for &skip in &[0u32, 1] {
                let dopt = DecompressOptions {
                    stable_dst,
                    skip_checksums: skip,
                    ..Default::default()
                };
                for &len in &[0usize, 1, 1000, 70000, 300000] {
                    let src = mkdata(Shape::Textish, len, &mut rng);
                    let frame = c_frame(&src, p);
                    let dc = Chunk::Random;
                    decode_and_compare(
                        &format!("dopt [{desc}] stableDst={stable_dst} skip={skip} len={len}"),
                        &frame,
                        &src,
                        Chunk::Random,
                        dc,
                        &dopt,
                        None,
                        &mut rng,
                    );
                }
            }
        }
    }

    // Row 180: skipChecksums over a frame whose checksums are corrupted.
    let (cd, rd) = syms::<FnDecompress>("LZ4F_decompress");
    for &bck in &[0i32, 1] {
        let mut p = Prefs::default();
        p.frame_info.content_checksum_flag = 1;
        p.frame_info.block_checksum_flag = bck;
        p.frame_info.block_size_id = 4;
        let src = mkdata(Shape::Textish, 5000, &mut rng);
        let good = c_frame(&src, &p);
        // corrupt the trailing content checksum (last 4 bytes)
        let mut bad = good.clone();
        let n = bad.len();
        bad[n - 1] ^= 0xFF;
        for &skip in &[0u32, 1] {
            let dopt = DecompressOptions { skip_checksums: skip, ..Default::default() };
            let ctx = new_dctx();
            let mut cbuf = vec![0u8; 6000];
            let mut rbuf = vec![0u8; 6000];
            let mut cds = cbuf.len();
            let mut rds = rbuf.len();
            let mut css = bad.len();
            let mut rss = bad.len();
            let x = unsafe { cd(ctx.c, cbuf.as_mut_ptr(), &mut cds, bad.as_ptr(), &mut css, &dopt) };
            let y = unsafe { rd(ctx.r, rbuf.as_mut_ptr(), &mut rds, bad.as_ptr(), &mut rss, &dopt) };
            assert_eq!(x, y, "corrupt-checksum bck={bck} skip={skip}: C={x:#x} R={y:#x}");
            assert_eq!(cds, rds, "corrupt-checksum dstSize bck={bck} skip={skip}");
            assert_eq!(css, rss, "corrupt-checksum srcSize bck={bck} skip={skip}");
            assert_eq!(&cbuf[..cds], &rbuf[..rds], "corrupt-checksum payload");
        }
        // and a corrupted BLOCK checksum
        if bck == 1 {
            let mut bad = good.clone();
            // the first block checksum sits right after the first block body;
            // flipping a byte in the middle of the frame hits block data or its
            // checksum -- either way both libraries must agree.
            let mid = bad.len() / 2;
            bad[mid] ^= 0x01;
            for &skip in &[0u32, 1] {
                let dopt = DecompressOptions { skip_checksums: skip, ..Default::default() };
                let ctx = new_dctx();
                let mut cbuf = vec![0u8; 6000];
                let mut rbuf = vec![0u8; 6000];
                let mut cds = cbuf.len();
                let mut rds = rbuf.len();
                let mut css = bad.len();
                let mut rss = bad.len();
                let x = unsafe { cd(ctx.c, cbuf.as_mut_ptr(), &mut cds, bad.as_ptr(), &mut css, &dopt) };
                let y = unsafe { rd(ctx.r, rbuf.as_mut_ptr(), &mut rds, bad.as_ptr(), &mut rss, &dopt) };
                assert_eq!(x, y, "corrupt-block skip={skip}: C={x:#x} R={y:#x}");
                assert_eq!(cds, rds);
                assert_eq!(css, rss);
                assert_eq!(&cbuf[..cds], &rbuf[..rds]);
            }
        }
    }
}

/// Row 182: `LZ4F_resetDecompressionContext` mid-frame, then decode a fresh frame.
#[test]
fn g10_reset_decompression_context() {
    let (cd, rd) = syms::<FnDecompress>("LZ4F_decompress");
    let (crc, rrc) = syms::<FnResetDctx>("LZ4F_resetDecompressionContext");
    let mut rng = Rng::new(SEED ^ 5);

    for (desc, p) in prefs_matrix_small().iter().step_by(4) {
        let src1 = mkdata(Shape::Textish, 20000, &mut rng);
        let src2 = mkdata(Shape::Periodic, 9000, &mut rng);
        let f1 = c_frame(&src1, p);
        let f2 = c_frame(&src2, p);
        let ctx = new_dctx();
        // partially decode f1
        let mut cbuf = vec![0u8; 100];
        let mut rbuf = vec![0u8; 100];
        let mut cds = 100;
        let mut rds = 100;
        let mut css = f1.len() / 3;
        let mut rss = css;
        let x = unsafe { cd(ctx.c, cbuf.as_mut_ptr(), &mut cds, f1.as_ptr(), &mut css, std::ptr::null()) };
        let y = unsafe { rd(ctx.r, rbuf.as_mut_ptr(), &mut rds, f1.as_ptr(), &mut rss, std::ptr::null()) };
        assert_eq!(x, y, "reset: partial decode [{desc}]");
        assert_eq!(cds, rds);
        assert_eq!(css, rss);
        // reset, then decode f2 in full
        unsafe { crc(ctx.c) };
        unsafe { rrc(ctx.r) };
        let mut cpay: Vec<u8> = Vec::new();
        let mut rpay: Vec<u8> = Vec::new();
        let mut off = 0usize;
        let mut guard = 0;
        while off < f2.len() {
            guard += 1;
            assert!(guard < 100000);
            let mut cbuf = vec![0u8; 4096];
            let mut rbuf = vec![0u8; 4096];
            let mut cds = cbuf.len();
            let mut rds = rbuf.len();
            let mut css = f2.len() - off;
            let mut rss = css;
            let x = unsafe {
                cd(ctx.c, cbuf.as_mut_ptr(), &mut cds, f2.as_ptr().add(off), &mut css, std::ptr::null())
            };
            let y = unsafe {
                rd(ctx.r, rbuf.as_mut_ptr(), &mut rds, f2.as_ptr().add(off), &mut rss, std::ptr::null())
            };
            assert_eq!(x, y, "reset: post-reset decode [{desc}] off={off}");
            assert_eq!(cds, rds);
            assert_eq!(css, rss);
            if is_err(x) {
                break;
            }
            cpay.extend_from_slice(&cbuf[..cds]);
            rpay.extend_from_slice(&rbuf[..rds]);
            if css == 0 && cds == 0 {
                break;
            }
            off += css;
            if x == 0 {
                break;
            }
        }
        assert_eq!(cpay, rpay, "reset: payload [{desc}]");
        assert_eq!(cpay, src2, "reset: payload != original [{desc}]");
    }
}

/// Rows 183-184: `LZ4F_decompress_usingDict`.
#[test]
fn g10_decompress_using_dict() {
    let (ccd, _) = syms::<FnCreateCDict>("LZ4F_createCDict");
    let (cfd, _) = syms::<FnFreeCDict>("LZ4F_freeCDict");
    let (cfc, _) = syms::<FnCompressFrameCDict>("LZ4F_compressFrame_usingCDict");
    let (cfb, _) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let (ccc, _) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let (cfcx, _) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let mut rng = Rng::new(SEED ^ 6);
    let mut arena = BlockArena::new();
    let mut cctx: *mut u8 = std::ptr::null_mut();
    unsafe { ccc(&mut cctx, LZ4F_VERSION) };

    for &ds in &[0usize, 1, 4, 1000, 65536, 70000] {
        let dict = arena.keep(mkdata(Shape::Textish, ds, &mut rng));
        let cdict = unsafe { ccd(dict.as_ptr(), ds) };
        for (desc, p) in prefs_matrix_small().iter().step_by(3) {
            for &len in &[0usize, 1, 1000, 70000] {
                let src = if ds > 16 {
                    let take = len.min(ds);
                    let mut v = dict[..take].to_vec();
                    v.extend(mkdata(Shape::Textish, len - take, &mut rng));
                    v
                } else {
                    mkdata(Shape::Textish, len, &mut rng)
                };
                // frame compressed WITH the dictionary
                let bound = unsafe { cfb(len, p) };
                let mut fbuf = vec![0u8; bound];
                let n = unsafe {
                    cfc(cctx, fbuf.as_mut_ptr(), bound, src.as_ptr(), len, cdict, p)
                };
                assert!(!is_err(n), "compressFrame_usingCDict failed {n:#x}");
                fbuf.truncate(n);

                for &(sc, dc) in &[
                    (Chunk::Whole, Chunk::Whole),
                    (Chunk::One, Chunk::One),
                    (Chunk::Random, Chunk::Random),
                ] {
                    if matches!(sc, Chunk::One) && len > 5000 {
                        continue;
                    }
                    decode_and_compare(
                        &format!("decompress_usingDict ds={ds} len={len} [{desc}] {sc:?}/{dc:?}"),
                        &fbuf,
                        &src,
                        sc,
                        dc,
                        std::ptr::null(),
                        Some(dict),
                        &mut rng,
                    );
                }
            }
        }
        unsafe { cfd(cdict) };
    }
    unsafe { cfcx(cctx) };
}

/// Rows 185-186: `_advanced` dctx constructor and the `LZ4F_freeDecompressionContext`
/// return value (which reports the current `dStage`).
#[test]
fn g10_dctx_lifecycle() {
    let (cca, rca) = syms::<FnCreateDctxAdvanced>("LZ4F_createDecompressionContext_advanced");
    let (cfd, rfd) = syms::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let (cd, rd) = syms::<FnDecompress>("LZ4F_decompress");
    let mut rng = Rng::new(SEED ^ 7);

    // _advanced with LZ4F_defaultCMem
    let ac = unsafe { cca(CustomMem::default_cmem(), LZ4F_VERSION) };
    let ar = unsafe { rca(CustomMem::default_cmem(), LZ4F_VERSION) };
    assert_eq!(ac.is_null(), ar.is_null(), "createDecompressionContext_advanced");
    assert!(!ac.is_null());
    let src = mkdata(Shape::Textish, 5000, &mut rng);
    let mut p = Prefs::default();
    p.frame_info.content_checksum_flag = 1;
    let frame = c_frame(&src, &p);
    let mut cbuf = vec![0u8; 6000];
    let mut rbuf = vec![0u8; 6000];
    let mut cds = cbuf.len();
    let mut rds = rbuf.len();
    let mut css = frame.len();
    let mut rss = frame.len();
    let x = unsafe { cd(ac, cbuf.as_mut_ptr(), &mut cds, frame.as_ptr(), &mut css, std::ptr::null()) };
    let y = unsafe { rd(ar, rbuf.as_mut_ptr(), &mut rds, frame.as_ptr(), &mut rss, std::ptr::null()) };
    assert_eq!(x, y);
    assert_eq!(&cbuf[..cds], &rbuf[..rds]);
    // complete frame -> free must report 0
    assert_eq!(unsafe { cfd(ac) }, unsafe { rfd(ar) }, "free after complete frame");

    // freed mid-frame -> free reports the raw dStage
    let bc = unsafe { cca(CustomMem::default_cmem(), LZ4F_VERSION) };
    let br = unsafe { rca(CustomMem::default_cmem(), LZ4F_VERSION) };
    let mut cds = 10usize;
    let mut rds = 10usize;
    let mut css = frame.len() / 2;
    let mut rss = css;
    unsafe { cd(bc, cbuf.as_mut_ptr(), &mut cds, frame.as_ptr(), &mut css, std::ptr::null()) };
    unsafe { rd(br, rbuf.as_mut_ptr(), &mut rds, frame.as_ptr(), &mut rss, std::ptr::null()) };
    assert_eq!(unsafe { cfd(bc) }, unsafe { rfd(br) }, "free mid-frame reports dStage");

    // free(NULL)
    assert_eq!(
        unsafe { cfd(std::ptr::null_mut()) },
        unsafe { rfd(std::ptr::null_mut()) },
        "freeDecompressionContext(NULL)"
    );
}

/// Row 187: cross-library round-trip — every preference combination compressed
/// by one library must decode identically in the other.
#[test]
fn g10_cross_library_roundtrip() {
    let (cf, rf) = syms::<FnCompressFrame>("LZ4F_compressFrame");
    let (cfb, _) = syms::<FnFrameBound>("LZ4F_compressFrameBound");
    let (cd, rd) = syms::<FnDecompress>("LZ4F_decompress");
    let (ccd, rcd) = syms::<FnCreateDctx>("LZ4F_createDecompressionContext");
    let (cfd, rfd) = syms::<FnFreeDctx>("LZ4F_freeDecompressionContext");
    let mut rng = Rng::new(SEED ^ 8);

    for (desc, p) in prefs_matrix().iter() {
        let len = rng.range(0, 200_000);
        let src = mkdata(ALL_SHAPES[rng.below(ALL_SHAPES.len())], len, &mut rng);
        let bound = unsafe { cfb(len, p) };
        let mut cbuf = vec![0u8; bound];
        let mut rbuf = vec![0u8; bound];
        let cn = unsafe { cf(cbuf.as_mut_ptr(), bound, src.as_ptr(), len, p) };
        let rn = unsafe { rf(rbuf.as_mut_ptr(), bound, src.as_ptr(), len, p) };
        assert_eq!(cn, rn, "roundtrip compress [{desc}]");
        cbuf.truncate(cn);
        rbuf.truncate(rn);
        assert_eq!(cbuf, rbuf, "roundtrip frames differ [{desc}]");

        // `frameInfo.frameType` is documented as a READ-ONLY field
        // (lz4frame.h:177): setting it does not make LZ4F_compressFrame emit a
        // skippable frame, so the payload still round-trips. Genuine skippable
        // frames are constructed byte-wise in g10_skippable_frames.
        let expect: &[u8] = &src;

        // Rust frame -> C decoder
        for (which, frame) in [("Rust->C", &rbuf), ("C->Rust", &cbuf)] {
            let mut dctx: *mut u8 = std::ptr::null_mut();
            let f = if which == "Rust->C" { cd } else { rd };
            let create = if which == "Rust->C" { ccd } else { rcd };
            let free = if which == "Rust->C" { cfd } else { rfd };
            unsafe { create(&mut dctx, LZ4F_VERSION) };
            let mut out = vec![0u8; len + 16];
            let mut ds = out.len();
            let mut ss = frame.len();
            let hint = unsafe { f(dctx, out.as_mut_ptr(), &mut ds, frame.as_ptr(), &mut ss, std::ptr::null()) };
            assert!(!is_err(hint), "{which} decode failed {hint:#x} [{desc}]");
            assert_eq!(&out[..ds], expect, "{which} payload mismatch [{desc}]");
            unsafe { free(dctx) };
        }
    }
}
