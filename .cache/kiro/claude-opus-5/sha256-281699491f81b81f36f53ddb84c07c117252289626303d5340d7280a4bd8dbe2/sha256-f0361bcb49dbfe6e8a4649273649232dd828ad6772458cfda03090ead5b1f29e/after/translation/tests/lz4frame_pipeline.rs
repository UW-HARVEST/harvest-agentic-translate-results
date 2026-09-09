//! Phase B — CONFIGS.md group 9: the LOW-LEVEL `lz4frame` compression pipeline.
//! Drives `createCompressionContext` -> `compressBegin*` -> many
//! `compressUpdate`/`uncompressedUpdate`/`flush` -> `compressEnd`, accumulating
//! the whole frame and comparing it byte-for-byte.
mod common;
use common::frame::*;
use common::*;

const SEED: u64 = 0x46_5241_4D45_0009;

fn is_err(v: usize) -> bool {
    v > (0usize.wrapping_sub(24))
}

struct Cctx {
    c: *mut u8,
    r: *mut u8,
    free: (FnFreeCctx, FnFreeCctx),
}
impl Drop for Cctx {
    fn drop(&mut self) {
        unsafe {
            (self.free.0)(self.c);
            (self.free.1)(self.r);
        }
    }
}
fn new_cctx() -> Cctx {
    let (cc, rc) = syms::<FnCreateCctx>("LZ4F_createCompressionContext");
    let free = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let mut c: *mut u8 = std::ptr::null_mut();
    let mut r: *mut u8 = std::ptr::null_mut();
    let a = unsafe { cc(&mut c, LZ4F_VERSION) };
    let b = unsafe { rc(&mut r, LZ4F_VERSION) };
    assert_eq!(a, b, "createCompressionContext return");
    assert!(!c.is_null() && !r.is_null());
    Cctx { c, r, free }
}

/// How the source is split across `compressUpdate` calls.
#[derive(Copy, Clone, Debug)]
enum Split {
    Whole,
    One,
    Random,
    BlockExact,
    BlockMinus1,
    BlockPlus1,
}
const SPLITS: [Split; 6] = [
    Split::Whole,
    Split::One,
    Split::Random,
    Split::BlockExact,
    Split::BlockMinus1,
    Split::BlockPlus1,
];

fn block_size(bsid: i32) -> usize {
    match bsid {
        5 => 256 * 1024,
        6 => 1024 * 1024,
        7 => 4 * 1024 * 1024,
        _ => 64 * 1024,
    }
}

/// Run one full low-level session on both libraries and compare the produced
/// frames byte-for-byte. `uncompressed_every` > 0 interleaves
/// `LZ4F_uncompressedUpdate`; `flush_every` > 0 interleaves `LZ4F_flush`.
#[allow(clippy::too_many_arguments)]
fn pipeline(
    label: &str,
    src: &[u8],
    p: *const Prefs,
    prefs_for_bound: *const Prefs,
    split: Split,
    copt: *const CompressOptions,
    uncompressed_every: usize,
    flush_every: usize,
    rng: &mut Rng,
    begin: BeginKind,
) {
    let (cbg, rbg) = syms::<FnCompressBegin>("LZ4F_compressBegin");
    let (cbd, rbd) = syms::<FnCompressBeginDict>("LZ4F_compressBegin_usingDict");
    let (cbd1, rbd1) = syms::<FnCompressBeginDict>("LZ4F_compressBegin_usingDictOnce");
    let (cbc, rbc) = syms::<FnCompressBeginCDict>("LZ4F_compressBegin_usingCDict");
    let (cu, ru) = syms::<FnCompressUpdate>("LZ4F_compressUpdate");
    let (cuu, ruu) = syms::<FnCompressUpdate>("LZ4F_uncompressedUpdate");
    let (cfl, rfl) = syms::<FnFlush>("LZ4F_flush");
    let (cend, rend) = syms::<FnFlush>("LZ4F_compressEnd");
    let (cbnd, _) = syms::<FnFrameBound>("LZ4F_compressBound");

    let ctx = new_cctx();
    let mut cout: Vec<u8> = Vec::new();
    let mut rout: Vec<u8> = Vec::new();
    let mut hdr_c = vec![0u8; 64];
    let mut hdr_r = vec![0u8; 64];

    let (hc, hr) = match begin {
        BeginKind::Plain => (
            unsafe { cbg(ctx.c, hdr_c.as_mut_ptr(), hdr_c.len(), p) },
            unsafe { rbg(ctx.r, hdr_r.as_mut_ptr(), hdr_r.len(), p) },
        ),
        BeginKind::UsingDict(d) => (
            unsafe { cbd(ctx.c, hdr_c.as_mut_ptr(), hdr_c.len(), d.as_ptr(), d.len(), p) },
            unsafe { rbd(ctx.r, hdr_r.as_mut_ptr(), hdr_r.len(), d.as_ptr(), d.len(), p) },
        ),
        BeginKind::UsingDictOnce(d) => (
            unsafe { cbd1(ctx.c, hdr_c.as_mut_ptr(), hdr_c.len(), d.as_ptr(), d.len(), p) },
            unsafe { rbd1(ctx.r, hdr_r.as_mut_ptr(), hdr_r.len(), d.as_ptr(), d.len(), p) },
        ),
        BeginKind::UsingCDict(cd_c, cd_r) => (
            unsafe { cbc(ctx.c, hdr_c.as_mut_ptr(), hdr_c.len(), cd_c, p) },
            unsafe { rbc(ctx.r, hdr_r.as_mut_ptr(), hdr_r.len(), cd_r, p) },
        ),
    };
    assert_eq!(hc, hr, "{label}: compressBegin C={hc:#x} R={hr:#x}");
    if is_err(hc) {
        return;
    }
    assert_eq!(&hdr_c[..hc], &hdr_r[..hr], "{label}: frame header differs");
    cout.extend_from_slice(&hdr_c[..hc]);
    rout.extend_from_slice(&hdr_r[..hr]);

    let bs = block_size(unsafe { (*prefs_for_bound).frame_info.block_size_id });
    let mut off = 0usize;
    let mut k = 0usize;
    while off < src.len() {
        let n = match split {
            Split::Whole => src.len() - off,
            Split::One => 1,
            Split::Random => rng.range(1, (bs * 2).min(70000)).min(src.len() - off),
            Split::BlockExact => bs.min(src.len() - off),
            Split::BlockMinus1 => (bs - 1).min(src.len() - off),
            Split::BlockPlus1 => (bs + 1).min(src.len() - off),
        };
        let use_uncompressed = uncompressed_every > 0 && k % uncompressed_every == 0;
        // "Important rule: dstCapacity MUST be large enough to store the entire
        //  source buffer as no compression is done for this operation"
        //  (lz4frame.h:701) -- compressBound alone is not a sufficient bound.
        let cap = if use_uncompressed {
            unsafe { cbnd(n, prefs_for_bound) }.max(n + bs + 64)
        } else {
            unsafe { cbnd(n, prefs_for_bound) }
        };
        let mut cd = vec![0x8Bu8; cap];
        let mut rd = vec![0x8Bu8; cap];
        let sp = unsafe { src.as_ptr().add(off) };
        let (cn, rn) = if use_uncompressed {
            (
                unsafe { cuu(ctx.c, cd.as_mut_ptr(), cap, sp, n, copt) },
                unsafe { ruu(ctx.r, rd.as_mut_ptr(), cap, sp, n, copt) },
            )
        } else {
            (
                unsafe { cu(ctx.c, cd.as_mut_ptr(), cap, sp, n, copt) },
                unsafe { ru(ctx.r, rd.as_mut_ptr(), cap, sp, n, copt) },
            )
        };
        assert_eq!(
            cn, rn,
            "{label}: {} k={k} off={off} n={n} C={cn:#x} R={rn:#x}",
            if use_uncompressed { "uncompressedUpdate" } else { "compressUpdate" }
        );
        if is_err(cn) {
            return;
        }
        assert_eq!(&cd[..cn], &rd[..cn], "{label}: update output differs k={k}");
        cout.extend_from_slice(&cd[..cn]);
        rout.extend_from_slice(&rd[..rn]);
        off += n;
        k += 1;

        if flush_every > 0 && k % flush_every == 0 {
            let cap = unsafe { cbnd(0, prefs_for_bound) };
            let mut cd = vec![0u8; cap.max(8)];
            let mut rd = vec![0u8; cap.max(8)];
            let a = unsafe { cfl(ctx.c, cd.as_mut_ptr(), cd.len(), copt) };
            let b = unsafe { rfl(ctx.r, rd.as_mut_ptr(), rd.len(), copt) };
            assert_eq!(a, b, "{label}: flush k={k} C={a:#x} R={b:#x}");
            if is_err(a) {
                return;
            }
            assert_eq!(&cd[..a], &rd[..a], "{label}: flush output differs k={k}");
            cout.extend_from_slice(&cd[..a]);
            rout.extend_from_slice(&rd[..b]);
        }
    }

    let cap = unsafe { cbnd(0, prefs_for_bound) }.max(16);
    let mut cd = vec![0u8; cap];
    let mut rd = vec![0u8; cap];
    let a = unsafe { cend(ctx.c, cd.as_mut_ptr(), cap, copt) };
    let b = unsafe { rend(ctx.r, rd.as_mut_ptr(), cap, copt) };
    assert_eq!(a, b, "{label}: compressEnd C={a:#x} R={b:#x}");
    if is_err(a) {
        return;
    }
    assert_eq!(&cd[..a], &rd[..a], "{label}: compressEnd output differs");
    cout.extend_from_slice(&cd[..a]);
    rout.extend_from_slice(&rd[..b]);

    assert_eq!(
        cout.len(),
        rout.len(),
        "{label}: total frame length differs {} vs {}",
        cout.len(),
        rout.len()
    );
    assert_eq!(cout, rout, "{label}: assembled frame differs");
}

enum BeginKind {
    Plain,
    UsingDict(&'static [u8]),
    UsingDictOnce(&'static [u8]),
    UsingCDict(*const u8, *const u8),
}

/// Rows 147-155: the full pipeline over the preference matrix x split strategy.
#[test]
fn g9_pipeline_matrix() {
    let mut rng = Rng::new(SEED);
    for (desc, p) in prefs_matrix_small().iter() {
        for &split in SPLITS.iter() {
            for &len in &[0usize, 1, 13, 1000, 70000, 300000] {
                // 1-byte splitting on large inputs would take minutes; cap it
                if matches!(split, Split::One) && len > 70000 {
                    continue;
                }
                let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
                let src = mkdata(shape, len, &mut rng);
                pipeline(
                    &format!("pipeline [{desc}] split={split:?} len={len} {shape:?}"),
                    &src,
                    p,
                    p,
                    split,
                    std::ptr::null(),
                    0,
                    0,
                    &mut rng,
                    BeginKind::Plain,
                );
            }
        }
    }
    // NULL prefs (row 147)
    for &split in SPLITS.iter() {
        for &len in &[0usize, 1, 13, 1000, 70000] {
            let src = mkdata(Shape::Textish, len, &mut rng);
            let defaults = Prefs::default();
            pipeline(
                &format!("pipeline NULL-prefs split={split:?} len={len}"),
                &src,
                std::ptr::null(),
                &defaults,
                split,
                std::ptr::null(),
                0,
                0,
                &mut rng,
                BeginKind::Plain,
            );
        }
    }
}

/// Rows 152-153: `stableSrc` and interleaved explicit `LZ4F_flush`.
#[test]
fn g9_options_and_flush() {
    let mut rng = Rng::new(SEED ^ 1);
    for (desc, p) in prefs_matrix_small().iter().step_by(3) {
        for &stable_src in &[0u32, 1] {
            let copt = CompressOptions { stable_src, ..Default::default() };
            for &flush_every in &[0usize, 1, 2, 3] {
                for &len in &[0usize, 1, 1000, 70000, 300000] {
                    let src = mkdata(Shape::Textish, len, &mut rng);
                    pipeline(
                        &format!(
                            "flush [{desc}] stableSrc={stable_src} flushEvery={flush_every} len={len}"
                        ),
                        &src,
                        p,
                        p,
                        Split::Random,
                        &copt,
                        0,
                        flush_every,
                        &mut rng,
                        BeginKind::Plain,
                    );
                }
            }
        }
    }
}

/// Rows 156-158: `LZ4F_uncompressedUpdate`, alone and interleaved.
#[test]
fn g9_uncompressed_update() {
    let mut rng = Rng::new(SEED ^ 2);
    for (desc, p) in prefs_matrix_small().iter() {
        // "This operation is only supported when LZ4F_blockIndependent is used"
        // (lz4frame.h:707). With linked blocks the C leaves the state in a UB
        // condition, so that combination is not a testable input.
        if p.frame_info.block_mode != 1 {
            continue;
        }
        for &every in &[1usize, 2, 3] {
            for &len in &[0usize, 1, 13, 1000, 70000, 300000] {
                for &split in &[Split::Random, Split::BlockExact, Split::Whole] {
                    let src = mkdata(Shape::Textish, len, &mut rng);
                    pipeline(
                        &format!("uncompressedUpdate [{desc}] every={every} len={len} split={split:?}"),
                        &src,
                        p,
                        p,
                        split,
                        std::ptr::null(),
                        every,
                        0,
                        &mut rng,
                        BeginKind::Plain,
                    );
                }
            }
        }
    }
}

/// Rows 154: contentSize declared and satisfied exactly.
#[test]
fn g9_declared_content_size() {
    let mut rng = Rng::new(SEED ^ 3);
    for &bsid in &[0i32, 4, 5, 7] {
        for &cck in &[0i32, 1] {
            for &len in &[0usize, 1, 13, 1000, 70000, 300000] {
                let mut p = Prefs::default();
                p.frame_info.block_size_id = bsid;
                p.frame_info.content_checksum_flag = cck;
                p.frame_info.content_size = len as u64;
                let src = mkdata(Shape::Textish, len, &mut rng);
                pipeline(
                    &format!("contentSize bsid={bsid} cck={cck} len={len}"),
                    &src,
                    &p,
                    &p,
                    Split::Random,
                    std::ptr::null(),
                    0,
                    0,
                    &mut rng,
                    BeginKind::Plain,
                );
            }
        }
    }
}

/// Rows 159-161: dictionary-primed `compressBegin` variants.
#[test]
fn g9_begin_with_dict() {
    let (ccd, rcd) = syms::<FnCreateCDict>("LZ4F_createCDict");
    let (cfd, rfd) = syms::<FnFreeCDict>("LZ4F_freeCDict");
    let mut rng = Rng::new(SEED ^ 4);
    let mut arena = BlockArena::new();

    for &ds in &[0usize, 1, 4, 1000, 65536, 70000] {
        // dictBuffer must outlive the whole session
        let dict = arena.keep(mkdata(Shape::Textish, ds, &mut rng));
        let cdc = unsafe { ccd(dict.as_ptr(), ds) };
        let cdr = unsafe { rcd(dict.as_ptr(), ds) };
        for (desc, p) in prefs_matrix_small().iter().step_by(2) {
            for &len in &[0usize, 13, 1000, 70000] {
                let src = if ds > 16 {
                    let take = len.min(ds);
                    let mut v = dict[..take].to_vec();
                    v.extend(mkdata(Shape::Textish, len - take, &mut rng));
                    v
                } else {
                    mkdata(Shape::Textish, len, &mut rng)
                };
                pipeline(
                    &format!("begin_usingDict ds={ds} len={len} [{desc}]"),
                    &src,
                    p,
                    p,
                    Split::Random,
                    std::ptr::null(),
                    0,
                    0,
                    &mut rng,
                    BeginKind::UsingDict(dict),
                );
                pipeline(
                    &format!("begin_usingDictOnce ds={ds} len={len} [{desc}]"),
                    &src,
                    p,
                    p,
                    Split::Random,
                    std::ptr::null(),
                    0,
                    0,
                    &mut rng,
                    BeginKind::UsingDictOnce(dict),
                );
                if !cdc.is_null() {
                    pipeline(
                        &format!("begin_usingCDict ds={ds} len={len} [{desc}]"),
                        &src,
                        p,
                        p,
                        Split::Random,
                        std::ptr::null(),
                        0,
                        0,
                        &mut rng,
                        BeginKind::UsingCDict(cdc, cdr),
                    );
                }
            }
        }
        unsafe { cfd(cdc) };
        unsafe { rfd(cdr) };
    }
}

/// Rows 162-164: the `_advanced` context constructor, and cctx REUSE across
/// frames including switching the compression strategy (which changes
/// `lz4CtxAlloc`/`lz4CtxType` inside the context).
#[test]
fn g9_cctx_reuse_and_advanced() {
    let (cca, rca) = syms::<FnCreateCctxAdvanced>("LZ4F_createCompressionContext_advanced");
    let (cfc, rfc) = syms::<FnFreeCctx>("LZ4F_freeCompressionContext");
    let (cbg, rbg) = syms::<FnCompressBegin>("LZ4F_compressBegin");
    let (cu, ru) = syms::<FnCompressUpdate>("LZ4F_compressUpdate");
    let (cend, rend) = syms::<FnFlush>("LZ4F_compressEnd");
    let (cbnd, _) = syms::<FnFrameBound>("LZ4F_compressBound");
    let mut rng = Rng::new(SEED ^ 5);

    for ctor in 0..2 {
        // 0 = plain createCompressionContext, 1 = _advanced with defaultCMem
        let (cc, rc): (*mut u8, *mut u8) = if ctor == 0 {
            let ctx = new_cctx();
            let p = (ctx.c, ctx.r);
            std::mem::forget(ctx); // freed manually below
            p
        } else {
            let a = unsafe { cca(CustomMem::default_cmem(), LZ4F_VERSION) };
            let b = unsafe { rca(CustomMem::default_cmem(), LZ4F_VERSION) };
            assert_eq!(a.is_null(), b.is_null(), "createCompressionContext_advanced");
            (a, b)
        };
        assert!(!cc.is_null() && !rc.is_null());

        // Ten consecutive frames on the SAME context, cycling the strategy so
        // the internal lz4/lz4hc context is reallocated and re-typed.
        for round in 0..10 {
            let lvl = [0i32, -3, 1, 2, 9, 10, 12, 0, 12, 3][round];
            let mut p = Prefs::default();
            p.compression_level = lvl;
            p.frame_info.block_size_id = [0i32, 4, 5, 6, 7][round % 5];
            p.frame_info.block_mode = (round % 2) as i32;
            p.frame_info.content_checksum_flag = (round % 3 == 0) as i32;
            p.frame_info.block_checksum_flag = (round % 2 == 1) as i32;

            let len = rng.range(0, 200_000);
            let src = mkdata(ALL_SHAPES[rng.below(ALL_SHAPES.len())], len, &mut rng);

            let mut cout: Vec<u8> = Vec::new();
            let mut rout: Vec<u8> = Vec::new();
            let mut hc = vec![0u8; 64];
            let mut hr = vec![0u8; 64];
            let a = unsafe { cbg(cc, hc.as_mut_ptr(), 64, &p) };
            let b = unsafe { rbg(rc, hr.as_mut_ptr(), 64, &p) };
            assert_eq!(a, b, "reuse round={round} compressBegin");
            assert_eq!(&hc[..a], &hr[..b], "reuse round={round} header");
            cout.extend_from_slice(&hc[..a]);
            rout.extend_from_slice(&hr[..b]);

            let mut off = 0usize;
            while off < len {
                let n = rng.range(1, 90000).min(len - off);
                let cap = unsafe { cbnd(n, &p) };
                let mut cd = vec![0u8; cap];
                let mut rd = vec![0u8; cap];
                let sp = unsafe { src.as_ptr().add(off) };
                let x = unsafe { cu(cc, cd.as_mut_ptr(), cap, sp, n, std::ptr::null()) };
                let y = unsafe { ru(rc, rd.as_mut_ptr(), cap, sp, n, std::ptr::null()) };
                assert_eq!(x, y, "reuse round={round} update off={off} n={n}");
                assert_eq!(&cd[..x], &rd[..y], "reuse round={round} update output");
                cout.extend_from_slice(&cd[..x]);
                rout.extend_from_slice(&rd[..y]);
                off += n;
            }
            let cap = unsafe { cbnd(0, &p) }.max(16);
            let mut cd = vec![0u8; cap];
            let mut rd = vec![0u8; cap];
            let x = unsafe { cend(cc, cd.as_mut_ptr(), cap, std::ptr::null()) };
            let y = unsafe { rend(rc, rd.as_mut_ptr(), cap, std::ptr::null()) };
            assert_eq!(x, y, "reuse round={round} compressEnd");
            assert_eq!(&cd[..x], &rd[..y], "reuse round={round} end output");
            cout.extend_from_slice(&cd[..x]);
            rout.extend_from_slice(&rd[..y]);
            assert_eq!(cout, rout, "reuse round={round} lvl={lvl}: frame differs");
        }
        assert_eq!(unsafe { cfc(cc) }, unsafe { rfc(rc) }, "freeCompressionContext");
    }
    // free(NULL)
    assert_eq!(
        unsafe { cfc(std::ptr::null_mut()) },
        unsafe { rfc(std::ptr::null_mut()) },
        "freeCompressionContext(NULL)"
    );
}
