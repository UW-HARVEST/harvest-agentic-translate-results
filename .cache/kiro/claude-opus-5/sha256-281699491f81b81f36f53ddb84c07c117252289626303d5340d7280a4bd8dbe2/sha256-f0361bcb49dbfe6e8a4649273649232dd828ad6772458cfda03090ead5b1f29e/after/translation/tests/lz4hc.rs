//! Phase B — CONFIGS.md groups 5, 6 and 7: `lz4hc.c` one-shot HC compression
//! across every level (all three strategies), the streaming HC API with
//! dictionaries and options, and the deprecated HC exports.
mod common;
use common::*;

const SEED: u64 = 0x4C_5A34_4843_0005;

/// Every level class the C distinguishes, plus the out-of-range values that get
/// clamped. Note: level 1 is NOT promoted to the default (the C uses `< 1`).
const LEVELS: [i32; 19] = [
    i32::MIN, -5, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 100, i32::MAX,
];

type FnHC = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
type FnHCExt = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
type FnHCDestSize = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, *mut i32, i32, i32) -> i32;
type FnCreateStreamHC = unsafe extern "C" fn() -> *mut u8;
type FnFreeStreamHC = unsafe extern "C" fn(*mut u8) -> i32;
type FnInitStreamHC = unsafe extern "C" fn(*mut u8, usize) -> *mut u8;
type FnResetStreamHC = unsafe extern "C" fn(*mut u8, i32);
type FnLoadDictHC = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;
type FnSaveDictHC = unsafe extern "C" fn(*mut u8, *mut u8, i32) -> i32;
type FnAttachHC = unsafe extern "C" fn(*mut u8, *const u8);
type FnHCContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
type FnHCContinueDestSize = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, *mut i32, i32) -> i32;
type FnSetLevel = unsafe extern "C" fn(*mut u8, i32);
type FnFavor = unsafe extern "C" fn(*mut u8, u32);
type FnI = unsafe extern "C" fn() -> i32;

struct HcStreams {
    c: *mut u8,
    r: *mut u8,
    free: (FnFreeStreamHC, FnFreeStreamHC),
}
impl Drop for HcStreams {
    fn drop(&mut self) {
        unsafe {
            (self.free.0)(self.c);
            (self.free.1)(self.r);
        }
    }
}
fn new_hc() -> HcStreams {
    let (cc, rc) = syms::<FnCreateStreamHC>("LZ4_createStreamHC");
    let free = syms::<FnFreeStreamHC>("LZ4_freeStreamHC");
    let s = HcStreams { c: unsafe { cc() }, r: unsafe { rc() }, free };
    assert!(!s.c.is_null() && !s.r.is_null());
    s
}

// ============================================================ Group 5 ========

/// Rows 73-83: every level x every size class x every payload shape, with tight
/// and short destination capacities.
#[test]
fn g5_compress_hc_levels() {
    let (c, r) = syms::<FnHC>("LZ4_compress_HC");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED);
    let lens: Vec<usize> = BOUNDARY_LENS
        .iter()
        .copied()
        .chain([65535usize, 65536, 65547, 70000].into_iter())
        .collect();

    for &lvl in LEVELS.iter() {
        for &len in lens.iter() {
            for &shape in ALL_SHAPES.iter() {
                let src = mkdata(shape, len, &mut rng);
                let bound = unsafe { cb(len as i32) }.max(1) as usize;
                let mut cd = vec![0x9Eu8; bound];
                let mut rd = vec![0x9Eu8; bound];
                let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, bound as i32, lvl) };
                let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, bound as i32, lvl) };
                same_full(
                    &format!("compress_HC lvl={lvl} len={len} {shape:?}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
                if cn > 0 {
                    for cap in [cn, cn - 1, (cn / 2).max(1), 1] {
                        let mut cd = vec![0x21u8; cap as usize];
                        let mut rd = vec![0x21u8; cap as usize];
                        let a = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap, lvl) };
                        let b = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap, lvl) };
                        same(
                            &format!("compress_HC tight lvl={lvl} len={len} cap={cap} {shape:?}"),
                            a as i64, &cd, b as i64, &rd,
                        );
                    }
                }
            }
        }
    }
}

/// Row 82: large repetitive inputs, which drive the pattern-analysis (level 9)
/// and optimal-parser (10-12) paths hardest.
#[test]
fn g5_compress_hc_large_repetitive() {
    let (c, r) = syms::<FnHC>("LZ4_compress_HC");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x1);
    for &lvl in &[1i32, 2, 3, 8, 9, 10, 11, 12] {
        for &shape in &[Shape::Constant, Shape::Periodic, Shape::SmallAlphabet, Shape::Chunky, Shape::Textish] {
            for &len in &[70000usize, 150_000] {
                let src = mkdata(shape, len, &mut rng);
                let bound = unsafe { cb(len as i32) } as usize;
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, bound as i32, lvl) };
                let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, bound as i32, lvl) };
                same(
                    &format!("compress_HC big lvl={lvl} len={len} {shape:?}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
            }
        }
    }
}

/// Rows 84-85: explicit external HC state, fresh and reused (fastReset).
#[test]
fn g5_compress_hc_ext_state() {
    let (cs, rs) = syms::<FnI>("LZ4_sizeofStateHC");
    assert_eq!(unsafe { cs() }, unsafe { rs() }, "LZ4_sizeofStateHC");
    let sz = unsafe { cs() } as usize;
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");

    for name in ["LZ4_compress_HC_extStateHC", "LZ4_compress_HC_extStateHC_fastReset"] {
        let (c, r) = syms::<FnHCExt>(name);
        let mut cst = vec![0u64; sz / 8 + 4];
        let mut rst = vec![0u64; sz / 8 + 4];
        let mut rng = Rng::new(SEED ^ name.len() as u64);
        for round in 0..60 {
            let lvl = LEVELS[round % LEVELS.len()];
            let len = rng.range(0, 9000);
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let src = mkdata(shape, len, &mut rng);
            let bound = unsafe { cb(len as i32) }.max(1) as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let cn = unsafe {
                c(cst.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), len as i32, bound as i32, lvl)
            };
            let rn = unsafe {
                r(rst.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), len as i32, bound as i32, lvl)
            };
            same(&format!("{name} round={round} lvl={lvl} len={len}"), cn as i64, &cd, rn as i64, &rd);
        }
    }
}

/// Row 86: LZ4_compress_HC_destSize (fillOutput) across levels and targets.
#[test]
fn g5_compress_hc_dest_size() {
    let (c, r) = syms::<FnHCDestSize>("LZ4_compress_HC_destSize");
    let (cs, _) = syms::<FnI>("LZ4_sizeofStateHC");
    let sz = unsafe { cs() } as usize;
    let mut rng = Rng::new(SEED ^ 0x2);
    for &lvl in &[i32::MIN, 0, 1, 2, 3, 9, 10, 12, 13, i32::MAX] {
        for &len in &[0usize, 1, 12, 13, 100, 1000, 4096, 70000] {
            for &shape in ALL_SHAPES.iter() {
                let src = mkdata(shape, len, &mut rng);
                for &tgt in &[1i32, 2, 3, 12, 13, 30, 100, 1000, 100000] {
                    let mut cst = vec![0u64; sz / 8 + 4];
                    let mut rst = vec![0u64; sz / 8 + 4];
                    let mut cd = vec![0u8; tgt.max(1) as usize];
                    let mut rd = vec![0u8; tgt.max(1) as usize];
                    let mut cin = len as i32;
                    let mut rin = len as i32;
                    let cn = unsafe {
                        c(cst.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), &mut cin, tgt, lvl)
                    };
                    let rn = unsafe {
                        r(rst.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), &mut rin, tgt, lvl)
                    };
                    let lbl = format!("HC_destSize lvl={lvl} len={len} tgt={tgt} {shape:?}");
                    assert_eq!(cin, rin, "{lbl}: *srcSizePtr C={cin} R={rin}");
                    same(&lbl, cn as i64, &cd, rn as i64, &rd);
                }
            }
        }
    }
}

/// Row 87-88: `LZ4_initStreamHC` on a user buffer, plus HC scalars.
#[test]
fn g5_init_stream_hc_and_scalars() {
    let (ci, ri) = syms::<FnInitStreamHC>("LZ4_initStreamHC");
    let (cc, rc) = syms::<FnHCContinue>("LZ4_compress_HC_continue");
    let (crs, rrs) = syms::<FnResetStreamHC>("LZ4_resetStreamHC");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let (cs, _) = syms::<FnI>("LZ4_sizeofStateHC");
    let sz = unsafe { cs() } as usize;

    for name in ["LZ4_sizeofStateHC", "LZ4_sizeofStreamStateHC"] {
        let (c, r) = syms::<FnI>(name);
        assert_eq!(unsafe { c() }, unsafe { r() }, "{name}");
    }

    let mut cbuf = vec![0u64; sz / 8 + 2];
    let mut rbuf = vec![0u64; sz / 8 + 2];
    let cp = unsafe { ci(cbuf.as_mut_ptr() as *mut u8, sz) };
    let rp = unsafe { ri(rbuf.as_mut_ptr() as *mut u8, sz) };
    assert_eq!(cp.is_null(), rp.is_null());
    assert!(!cp.is_null());
    let mut rng = Rng::new(SEED ^ 0x3);
    for &lvl in &[2i32, 9, 12] {
        unsafe { crs(cp, lvl) };
        unsafe { rrs(rp, lvl) };
        let total = mkdata(Shape::Textish, 60000, &mut rng);
        let mut off = 0usize;
        let mut k = 0;
        while off < total.len() {
            let n = rng.range(1, 7000).min(total.len() - off);
            let bound = unsafe { cb(n as i32) } as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let cn = unsafe { cc(cp, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32, bound as i32) };
            let rn = unsafe { rc(rp, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32, bound as i32) };
            same(&format!("initStreamHC lvl={lvl} k={k} n={n}"), cn as i64, &cd, rn as i64, &rd);
            off += n;
            k += 1;
        }
        assert_eq!(&cbuf[..sz / 8], &rbuf[..sz / 8], "HC stream state diverged lvl={lvl}");
    }
}

// ============================================================ Group 6 ========

/// Rows 89-93: HC streaming — contiguous chaining, extDict chaining, ring
/// buffer, and the destSize streaming variant.
#[test]
fn g6_hc_streaming() {
    let (crs, rrs) = syms::<FnResetStreamHC>("LZ4_resetStreamHC");
    let (crf, rrf) = syms::<FnResetStreamHC>("LZ4_resetStreamHC_fast");
    let (cc, rc) = syms::<FnHCContinue>("LZ4_compress_HC_continue");
    let (cds, rds) = syms::<FnHCContinueDestSize>("LZ4_compress_HC_continue_destSize");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let (crb, _) = syms::<FnCompressBound>("LZ4_decoderRingBufferSize");
    let mut rng = Rng::new(SEED ^ 0x4);

    for &lvl in &[1i32, 2, 3, 9, 10, 12] {
        for &fastreset in &[false, true] {
            // contiguous
            for &shape in &[Shape::Textish, Shape::Periodic, Shape::Random] {
                let s = new_hc();
                if fastreset {
                    unsafe { crf(s.c, lvl) };
                    unsafe { rrf(s.r, lvl) };
                } else {
                    unsafe { crs(s.c, lvl) };
                    unsafe { rrs(s.r, lvl) };
                }
                let total = mkdata(shape, 180_000, &mut rng);
                let mut off = 0usize;
                let mut k = 0;
                while off < total.len() {
                    let n = rng.range(1, 9000).min(total.len() - off);
                    let bound = unsafe { cb(n as i32) } as usize;
                    let mut cd = vec![0u8; bound];
                    let mut rd = vec![0u8; bound];
                    let cn = unsafe {
                        cc(s.c, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32, bound as i32)
                    };
                    let rn = unsafe {
                        rc(s.r, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32, bound as i32)
                    };
                    same(
                        &format!("HC_continue contig lvl={lvl} fastreset={fastreset} {shape:?} k={k} n={n}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                    off += n;
                    k += 1;
                }
            }
            // non-contiguous -> LZ4HC_setExternalDict
            for &shape in &[Shape::Textish, Shape::Periodic] {
                let s = new_hc();
                if fastreset {
                    unsafe { crf(s.c, lvl) };
                    unsafe { rrf(s.r, lvl) };
                } else {
                    unsafe { crs(s.c, lvl) };
                    unsafe { rrs(s.r, lvl) };
                }
                let mut arena = BlockArena::new();
                for k in 0..12 {
                    let n = rng.range(1, 20000);
                    let blk = arena.keep(mkdata(shape, n, &mut rng));
                    let bound = unsafe { cb(n as i32) } as usize;
                    let mut cd = vec![0u8; bound];
                    let mut rd = vec![0u8; bound];
                    let cn = unsafe { cc(s.c, blk.as_ptr(), cd.as_mut_ptr(), n as i32, bound as i32) };
                    let rn = unsafe { rc(s.r, blk.as_ptr(), rd.as_mut_ptr(), n as i32, bound as i32) };
                    same(
                        &format!("HC_continue extdict lvl={lvl} fastreset={fastreset} {shape:?} k={k} n={n}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
            }
        }
        // ring buffer
        for &msg_max in &[1000usize, 9000] {
            let rbs = unsafe { crb(msg_max as i32) } as usize;
            let s = new_hc();
            unsafe { crs(s.c, lvl) };
            unsafe { rrs(s.r, lvl) };
            let mut cring = vec![0u8; rbs];
            let mut rring = vec![0u8; rbs];
            let mut pos = 0usize;
            for k in 0..40 {
                let n = rng.range(1, msg_max);
                if pos + n > rbs {
                    pos = 0;
                }
                let msg = mkdata(Shape::Textish, n, &mut rng);
                cring[pos..pos + n].copy_from_slice(&msg);
                rring[pos..pos + n].copy_from_slice(&msg);
                let bound = unsafe { cb(n as i32) } as usize;
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe {
                    cc(s.c, cring.as_ptr().add(pos), cd.as_mut_ptr(), n as i32, bound as i32)
                };
                let rn = unsafe {
                    rc(s.r, rring.as_ptr().add(pos), rd.as_mut_ptr(), n as i32, bound as i32)
                };
                same(
                    &format!("HC ring lvl={lvl} msg_max={msg_max} k={k} n={n} pos={pos}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
                pos += n;
            }
        }
        // continue_destSize
        for &shape in &[Shape::Textish, Shape::Random, Shape::Periodic] {
            let s = new_hc();
            unsafe { crs(s.c, lvl) };
            unsafe { rrs(s.r, lvl) };
            let total = mkdata(shape, 60000, &mut rng);
            let mut off = 0usize;
            let mut k = 0;
            while off < total.len() {
                let n = rng.range(1, 7000).min(total.len() - off);
                let tgt = [1i32, 12, 100, 3000, 60000][k % 5];
                let mut cd = vec![0u8; tgt.max(1) as usize];
                let mut rd = vec![0u8; tgt.max(1) as usize];
                let mut cin = n as i32;
                let mut rin = n as i32;
                let cn = unsafe {
                    cds(s.c, total.as_ptr().add(off), cd.as_mut_ptr(), &mut cin, tgt)
                };
                let rn = unsafe {
                    rds(s.r, total.as_ptr().add(off), rd.as_mut_ptr(), &mut rin, tgt)
                };
                let lbl = format!("HC_continue_destSize lvl={lvl} {shape:?} k={k} n={n} tgt={tgt}");
                assert_eq!(cin, rin, "{lbl}: *srcSizePtr C={cin} R={rin}");
                same(&lbl, cn as i64, &cd, rn as i64, &rd);
                // must advance by the amount actually consumed, like a real consumer
                off += (cin.max(0) as usize).max(1);
                k += 1;
            }
        }
    }
}

/// Row 94: `LZ4_loadDictHC` across the dictSize matrix and levels.
#[test]
fn g6_load_dict_hc() {
    let (cl, rl) = syms::<FnLoadDictHC>("LZ4_loadDictHC");
    let (crs, rrs) = syms::<FnResetStreamHC>("LZ4_resetStreamHC");
    let (cc, rc) = syms::<FnHCContinue>("LZ4_compress_HC_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x5);
    for &lvl in &[1i32, 2, 3, 9, 10, 12] {
        for &ds in &[0usize, 1, 2, 3, 4, 5, 100, 1000, 65535, 65536, 70000, 200000] {
            for &shape in &[Shape::Textish, Shape::Periodic, Shape::Random] {
                let dict = mkdata(shape, ds, &mut rng);
                let s = new_hc();
                unsafe { crs(s.c, lvl) };
                unsafe { rrs(s.r, lvl) };
                let a = unsafe { cl(s.c, dict.as_ptr(), ds as i32) };
                let b = unsafe { rl(s.r, dict.as_ptr(), ds as i32) };
                assert_eq!(a, b, "loadDictHC lvl={lvl} ds={ds} C={a} R={b}");
                let mut arena = BlockArena::new();
                for k in 0..4 {
                    let n = rng.range(1, 6000);
                    let blk = arena.keep(if ds > 32 && k % 2 == 0 {
                        let take = n.min(ds);
                        let st = rng.below(ds - take + 1);
                        let mut v = dict[st..st + take].to_vec();
                        v.extend(mkdata(shape, n - take, &mut rng));
                        v
                    } else {
                        mkdata(shape, n, &mut rng)
                    });
                    let bound = unsafe { cb(blk.len() as i32) } as usize;
                    let mut cd = vec![0u8; bound];
                    let mut rd = vec![0u8; bound];
                    let cn = unsafe {
                        cc(s.c, blk.as_ptr(), cd.as_mut_ptr(), blk.len() as i32, bound as i32)
                    };
                    let rn = unsafe {
                        rc(s.r, blk.as_ptr(), rd.as_mut_ptr(), blk.len() as i32, bound as i32)
                    };
                    same(
                        &format!("loadDictHC lvl={lvl} ds={ds} {shape:?} k={k}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
            }
        }
    }
}

/// Rows 95-98: `LZ4_attach_HC_dictionary` — the dictCtx path, the >4 KB
/// cold-start copy, the `isStateCompatible` mid/non-mid gate, the >=64 KB
/// detach, and detaching with NULL.
#[test]
fn g6_attach_hc_dictionary() {
    let (cl, rl) = syms::<FnLoadDictHC>("LZ4_loadDictHC");
    let (crs, rrs) = syms::<FnResetStreamHC>("LZ4_resetStreamHC");
    let (ca, ra) = syms::<FnAttachHC>("LZ4_attach_HC_dictionary");
    let (cc, rc) = syms::<FnHCContinue>("LZ4_compress_HC_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x6);

    // dict level x working level covers isStateCompatible (mid = levels 1-2)
    for &dlvl in &[2i32, 9, 12] {
        for &wlvl in &[2i32, 9, 12] {
            for &ds in &[0usize, 4, 100, 5000, 65536, 70000] {
                for &first in &[1usize, 100, 4095, 4096, 4097, 9000, 70000] {
                    let shape = Shape::Textish;
                    let dict = mkdata(shape, ds, &mut rng);
                    let d = new_hc();
                    unsafe { crs(d.c, dlvl) };
                    unsafe { rrs(d.r, dlvl) };
                    unsafe { cl(d.c, dict.as_ptr(), ds as i32) };
                    unsafe { rl(d.r, dict.as_ptr(), ds as i32) };

                    let w = new_hc();
                    unsafe { crs(w.c, wlvl) };
                    unsafe { rrs(w.r, wlvl) };
                    unsafe { ca(w.c, d.c) };
                    unsafe { ra(w.r, d.r) };

                    let mut lens = vec![first];
                    for _ in 0..3 {
                        lens.push(rng.range(1, 40000));
                    }
                    let mut arena = BlockArena::new();
                    for (k, &n) in lens.iter().enumerate() {
                        let blk = arena.keep(if ds > 32 {
                            let take = n.min(ds);
                            let st = rng.below(ds - take + 1);
                            let mut v = dict[st..st + take].to_vec();
                            v.extend(mkdata(shape, n - take, &mut rng));
                            v
                        } else {
                            mkdata(shape, n, &mut rng)
                        });
                        let bound = unsafe { cb(blk.len() as i32) } as usize;
                        let mut cd = vec![0u8; bound];
                        let mut rd = vec![0u8; bound];
                        let cn = unsafe {
                            cc(w.c, blk.as_ptr(), cd.as_mut_ptr(), blk.len() as i32, bound as i32)
                        };
                        let rn = unsafe {
                            rc(w.r, blk.as_ptr(), rd.as_mut_ptr(), blk.len() as i32, bound as i32)
                        };
                        same(
                            &format!("attachHC dlvl={dlvl} wlvl={wlvl} ds={ds} first={first} k={k}"),
                            cn as i64, &cd, rn as i64, &rd,
                        );
                    }
                }
            }
        }
    }

    // Row 98: detach with NULL
    for &lvl in &[2i32, 9, 12] {
        let dict = mkdata(Shape::Textish, 5000, &mut rng);
        let d = new_hc();
        unsafe { crs(d.c, lvl) };
        unsafe { rrs(d.r, lvl) };
        unsafe { cl(d.c, dict.as_ptr(), 5000) };
        unsafe { rl(d.r, dict.as_ptr(), 5000) };
        let w = new_hc();
        unsafe { crs(w.c, lvl) };
        unsafe { rrs(w.r, lvl) };
        unsafe { ca(w.c, d.c) };
        unsafe { ra(w.r, d.r) };
        unsafe { ca(w.c, std::ptr::null()) };
        unsafe { ra(w.r, std::ptr::null()) };
        let blk = mkdata(Shape::Textish, 3000, &mut rng);
        let bound = unsafe { cb(3000) } as usize;
        let mut cd = vec![0u8; bound];
        let mut rd = vec![0u8; bound];
        let cn = unsafe { cc(w.c, blk.as_ptr(), cd.as_mut_ptr(), 3000, bound as i32) };
        let rn = unsafe { rc(w.r, blk.as_ptr(), rd.as_mut_ptr(), 3000, bound as i32) };
        same(&format!("attachHC detach lvl={lvl}"), cn as i64, &cd, rn as i64, &rd);
    }
}

/// Row 99: `LZ4_saveDictHC`.
#[test]
fn g6_save_dict_hc() {
    let (cs, rs) = syms::<FnSaveDictHC>("LZ4_saveDictHC");
    let (crs, rrs) = syms::<FnResetStreamHC>("LZ4_resetStreamHC");
    let (cc, rc) = syms::<FnHCContinue>("LZ4_compress_HC_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x7);
    for &lvl in &[2i32, 9, 12] {
        for &ds in &[0usize, 1, 3, 4, 100, 65535, 65536, 70000] {
            for &shape in &[Shape::Textish, Shape::Periodic] {
                let s = new_hc();
                unsafe { crs(s.c, lvl) };
                unsafe { rrs(s.r, lvl) };
                let total = mkdata(shape, 100_000, &mut rng);
                let mut off = 0usize;
                while off < 50_000 {
                    let n = 7000usize.min(50_000 - off);
                    let bound = unsafe { cb(n as i32) } as usize;
                    let mut cd = vec![0u8; bound];
                    let mut rd = vec![0u8; bound];
                    let cn = unsafe {
                        cc(s.c, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32, bound as i32)
                    };
                    let rn = unsafe {
                        rc(s.r, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32, bound as i32)
                    };
                    same(&format!("saveDictHC pre lvl={lvl} ds={ds}"), cn as i64, &cd, rn as i64, &rd);
                    off += n;
                }
                let mut csafe = vec![0u8; ds.max(1)];
                let mut rsafe = vec![0u8; ds.max(1)];
                let a = unsafe { cs(s.c, csafe.as_mut_ptr(), ds as i32) };
                let b = unsafe { rs(s.r, rsafe.as_mut_ptr(), ds as i32) };
                assert_eq!(a, b, "saveDictHC lvl={lvl} ds={ds}: C={a} R={b}");
                assert_eq!(csafe, rsafe, "saveDictHC buffer lvl={lvl} ds={ds}");
                let mut arena = BlockArena::new();
                for k in 0..3 {
                    let n = rng.range(1, 6000);
                    let blk = arena.keep(mkdata(shape, n, &mut rng));
                    let bound = unsafe { cb(n as i32) } as usize;
                    let mut cd = vec![0u8; bound];
                    let mut rd = vec![0u8; bound];
                    let cn = unsafe { cc(s.c, blk.as_ptr(), cd.as_mut_ptr(), n as i32, bound as i32) };
                    let rn = unsafe { rc(s.r, blk.as_ptr(), rd.as_mut_ptr(), n as i32, bound as i32) };
                    same(&format!("saveDictHC post lvl={lvl} ds={ds} k={k}"), cn as i64, &cd, rn as i64, &rd);
                }
            }
        }
    }
    // saveDictHC(NULL, 0)
    let s = new_hc();
    assert_eq!(
        unsafe { cs(s.c, std::ptr::null_mut(), 0) },
        unsafe { rs(s.r, std::ptr::null_mut(), 0) },
        "saveDictHC(NULL,0)"
    );
}

/// Rows 100-102: `LZ4_setCompressionLevel` mid-stream and
/// `LZ4_favorDecompressionSpeed` (a no-op below level 10).
#[test]
fn g6_hc_options() {
    let (csl, rsl) = syms::<FnSetLevel>("LZ4_setCompressionLevel");
    let (cfv, rfv) = syms::<FnFavor>("LZ4_favorDecompressionSpeed");
    let (crs, rrs) = syms::<FnResetStreamHC>("LZ4_resetStreamHC");
    let (cc, rc) = syms::<FnHCContinue>("LZ4_compress_HC_continue");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x8);

    // mid-stream level changes
    for &shape in &[Shape::Textish, Shape::Periodic, Shape::Random] {
        let s = new_hc();
        unsafe { crs(s.c, 9) };
        unsafe { rrs(s.r, 9) };
        let total = mkdata(shape, 200_000, &mut rng);
        let mut off = 0usize;
        let mut k = 0;
        while off < total.len() {
            let lvl = LEVELS[k % LEVELS.len()];
            unsafe { csl(s.c, lvl) };
            unsafe { rsl(s.r, lvl) };
            let n = rng.range(1, 9000).min(total.len() - off);
            let bound = unsafe { cb(n as i32) } as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let cn = unsafe { cc(s.c, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32, bound as i32) };
            let rn = unsafe { rc(s.r, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32, bound as i32) };
            same(
                &format!("setCompressionLevel {shape:?} k={k} lvl={lvl} n={n}"),
                cn as i64, &cd, rn as i64, &rd,
            );
            off += n;
            k += 1;
        }
    }

    // favorDecSpeed on/off across all level classes
    for &favor in &[0u32, 1, 2, 0xFFFF_FFFF] {
        for &lvl in &[1i32, 2, 3, 9, 10, 11, 12] {
            for &shape in &[Shape::Textish, Shape::Periodic, Shape::Chunky, Shape::Random] {
                let s = new_hc();
                unsafe { crs(s.c, lvl) };
                unsafe { rrs(s.r, lvl) };
                unsafe { cfv(s.c, favor) };
                unsafe { rfv(s.r, favor) };
                let total = mkdata(shape, 90_000, &mut rng);
                let mut off = 0usize;
                let mut k = 0;
                while off < total.len() {
                    let n = 12000usize.min(total.len() - off);
                    let bound = unsafe { cb(n as i32) } as usize;
                    let mut cd = vec![0u8; bound];
                    let mut rd = vec![0u8; bound];
                    let cn = unsafe {
                        cc(s.c, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32, bound as i32)
                    };
                    let rn = unsafe {
                        rc(s.r, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32, bound as i32)
                    };
                    same(
                        &format!("favorDecSpeed={favor} lvl={lvl} {shape:?} k={k}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                    off += n;
                    k += 1;
                }
            }
        }
    }
}

/// Rows 103-104: round-trip HC output through the block decoder.
#[test]
fn g6_hc_roundtrip() {
    let (cc, rc) = syms::<FnHC>("LZ4_compress_HC");
    let (cd, rd) = syms::<FnDecompressSafe>("LZ4_decompress_safe");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let mut rng = Rng::new(SEED ^ 0x9);
    for &lvl in LEVELS.iter() {
        for _ in 0..12 {
            let len = rng.range(0, 30000);
            let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
            let src = mkdata(shape, len, &mut rng);
            let bound = unsafe { cb(len as i32) }.max(1) as usize;
            let mut cbuf = vec![0u8; bound];
            let mut rbuf = vec![0u8; bound];
            let cn = unsafe { cc(src.as_ptr(), cbuf.as_mut_ptr(), len as i32, bound as i32, lvl) };
            let rn = unsafe { rc(src.as_ptr(), rbuf.as_mut_ptr(), len as i32, bound as i32, lvl) };
            assert_eq!(cn, rn);
            if cn <= 0 {
                continue;
            }
            let mut out = vec![0u8; len + 8];
            let n = unsafe { cd(rbuf.as_ptr(), out.as_mut_ptr(), rn, (len + 8) as i32) };
            assert_eq!(n, len as i32, "Rust HC(lvl={lvl}) block rejected by C decoder");
            assert_eq!(&out[..len], &src[..]);
            let mut out = vec![0u8; len + 8];
            let n = unsafe { rd(cbuf.as_ptr(), out.as_mut_ptr(), cn, (len + 8) as i32) };
            assert_eq!(n, len as i32, "C HC(lvl={lvl}) block rejected by Rust decoder");
            assert_eq!(&out[..len], &src[..]);
        }
    }
}

// ============================================================ Group 7 ========

/// Rows 105-110: deprecated HC one-shot wrappers.
#[test]
fn g7_deprecated_hc_oneshot() {
    type Fn3 = unsafe extern "C" fn(*const u8, *mut u8, i32) -> i32;
    type Fn4 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32) -> i32;
    type Fn4S = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;
    type Fn5S = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let (cs, _) = syms::<FnI>("LZ4_sizeofStateHC");
    let sz = unsafe { cs() } as usize;
    let mut rng = Rng::new(SEED ^ 0xA);
    let lens: Vec<usize> = BOUNDARY_LENS
        .iter()
        .copied()
        .chain([65535usize, 65547, 70000].into_iter())
        .collect();

    for &len in lens.iter() {
        for &shape in ALL_SHAPES.iter() {
            let src = mkdata(shape, len, &mut rng);
            let bound = unsafe { cb(len as i32) }.max(1) as usize;

            // LZ4_compressHC (unbounded dst)
            let (c, r) = syms::<Fn3>("LZ4_compressHC");
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32) };
            let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32) };
            same(&format!("LZ4_compressHC len={len} {shape:?}"), cn as i64, &cd, rn as i64, &rd);

            // LZ4_compressHC_limitedOutput
            let (c, r) = syms::<Fn4>("LZ4_compressHC_limitedOutput");
            for cap in [bound as i32, 1, (bound / 2) as i32] {
                let mut cd = vec![0u8; cap.max(1) as usize];
                let mut rd = vec![0u8; cap.max(1) as usize];
                let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap) };
                let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap) };
                same(
                    &format!("compressHC_limitedOutput len={len} cap={cap} {shape:?}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
            }

            for &lvl in &[0i32, 1, 2, 9, 12, 13, -1, i32::MAX] {
                // LZ4_compressHC2 (unbounded dst)
                let (c, r) = syms::<Fn4>("LZ4_compressHC2");
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, lvl) };
                let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, lvl) };
                same(
                    &format!("compressHC2 len={len} lvl={lvl} {shape:?}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
                // LZ4_compressHC2_limitedOutput
                let (c, r) = syms::<Fn5S>("LZ4_compressHC2_limitedOutput");
                // note: signature is (src,dst,srcSize,maxOut,level) -> reuse Fn5S shape
                let _ = (c, r);
                type Fn5 = unsafe extern "C" fn(*const u8, *mut u8, i32, i32, i32) -> i32;
                let (c, r) = syms::<Fn5>("LZ4_compressHC2_limitedOutput");
                for cap in [bound as i32, 1] {
                    let mut cd = vec![0u8; cap.max(1) as usize];
                    let mut rd = vec![0u8; cap.max(1) as usize];
                    let cn = unsafe { c(src.as_ptr(), cd.as_mut_ptr(), len as i32, cap, lvl) };
                    let rn = unsafe { r(src.as_ptr(), rd.as_mut_ptr(), len as i32, cap, lvl) };
                    same(
                        &format!("compressHC2_limitedOutput len={len} cap={cap} lvl={lvl} {shape:?}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
                // withStateHC variants
                let (c, r) = syms::<Fn5S>("LZ4_compressHC2_withStateHC");
                let mut cst = vec![0u64; sz / 8 + 4];
                let mut rst = vec![0u64; sz / 8 + 4];
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe {
                    c(cst.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), len as i32, lvl)
                };
                let rn = unsafe {
                    r(rst.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), len as i32, lvl)
                };
                same(
                    &format!("compressHC2_withStateHC len={len} lvl={lvl} {shape:?}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
                type Fn6S = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
                let (c, r) = syms::<Fn6S>("LZ4_compressHC2_limitedOutput_withStateHC");
                for cap in [bound as i32, 1] {
                    let mut cd = vec![0u8; cap.max(1) as usize];
                    let mut rd = vec![0u8; cap.max(1) as usize];
                    let cn = unsafe {
                        c(cst.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), len as i32, cap, lvl)
                    };
                    let rn = unsafe {
                        r(rst.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), len as i32, cap, lvl)
                    };
                    same(
                        &format!("compressHC2_limitedOutput_withStateHC len={len} cap={cap} lvl={lvl}"),
                        cn as i64, &cd, rn as i64, &rd,
                    );
                }
            }

            // LZ4_compressHC_withStateHC / _limitedOutput_withStateHC (level fixed 0)
            let (c, r) = syms::<Fn4S>("LZ4_compressHC_withStateHC");
            let mut cst = vec![0u64; sz / 8 + 4];
            let mut rst = vec![0u64; sz / 8 + 4];
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let cn = unsafe {
                c(cst.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), len as i32)
            };
            let rn = unsafe {
                r(rst.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), len as i32)
            };
            same(
                &format!("compressHC_withStateHC len={len} {shape:?}"),
                cn as i64, &cd, rn as i64, &rd,
            );
            let (c, r) = syms::<Fn5S>("LZ4_compressHC_limitedOutput_withStateHC");
            for cap in [bound as i32, 1] {
                let mut cd = vec![0u8; cap.max(1) as usize];
                let mut rd = vec![0u8; cap.max(1) as usize];
                let cn = unsafe {
                    c(cst.as_mut_ptr() as *mut u8, src.as_ptr(), cd.as_mut_ptr(), len as i32, cap)
                };
                let rn = unsafe {
                    r(rst.as_mut_ptr() as *mut u8, src.as_ptr(), rd.as_mut_ptr(), len as i32, cap)
                };
                same(
                    &format!("compressHC_limitedOutput_withStateHC len={len} cap={cap} {shape:?}"),
                    cn as i64, &cd, rn as i64, &rd,
                );
            }
        }
    }
}

/// Rows 111-115: deprecated HC streaming wrappers.
#[test]
fn g7_deprecated_hc_streaming() {
    type FnCreateHC = unsafe extern "C" fn(*const u8) -> *mut u8;
    type FnFreeHC = unsafe extern "C" fn(*mut u8) -> i32;
    type FnResetStreamStateHC = unsafe extern "C" fn(*mut u8, *mut u8) -> i32;
    type FnCont4 = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32) -> i32;
    type FnCont5 = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
    type FnCont5L = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
    type FnCont6 = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32, i32) -> i32;
    type FnSlideHC = unsafe extern "C" fn(*mut u8) -> *mut u8;

    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");
    let (csz, rsz) = syms::<FnI>("LZ4_sizeofStreamStateHC");
    assert_eq!(unsafe { csz() }, unsafe { rsz() }, "LZ4_sizeofStreamStateHC");
    let sz = unsafe { csz() } as usize;
    let mut rng = Rng::new(SEED ^ 0xB);

    // LZ4_compressHC_continue / _limitedOutput_continue on a proper HC stream
    for &shape in &[Shape::Textish, Shape::Periodic, Shape::Random] {
        let s = new_hc();
        let (crs, rrs) = syms::<FnResetStreamHC>("LZ4_resetStreamHC");
        unsafe { crs(s.c, 9) };
        unsafe { rrs(s.r, 9) };
        let (cc, rc) = syms::<FnCont4>("LZ4_compressHC_continue");
        let (ccl, rcl) = syms::<FnCont5>("LZ4_compressHC_limitedOutput_continue");
        let total = mkdata(shape, 90_000, &mut rng);
        let mut off = 0usize;
        let mut k = 0;
        while off < total.len() {
            let n = rng.range(1, 9000).min(total.len() - off);
            let bound = unsafe { cb(n as i32) } as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let (cn, rn) = if k % 2 == 0 {
                (
                    unsafe { cc(s.c, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32) },
                    unsafe { rc(s.r, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32) },
                )
            } else {
                (
                    unsafe { ccl(s.c, total.as_ptr().add(off), cd.as_mut_ptr(), n as i32, bound as i32) },
                    unsafe { rcl(s.r, total.as_ptr().add(off), rd.as_mut_ptr(), n as i32, bound as i32) },
                )
            };
            same(&format!("compressHC_continue {shape:?} k={k} n={n}"), cn as i64, &cd, rn as i64, &rd);
            off += n;
            k += 1;
        }
    }

    // Row 113: LZ4_resetStreamStateHC + continue
    for &shape in &[Shape::Textish, Shape::Random] {
        let (crss, rrss) = syms::<FnResetStreamStateHC>("LZ4_resetStreamStateHC");
        let (cc, rc) = syms::<FnCont4>("LZ4_compressHC_continue");
        let mut cst = vec![0u64; sz / 8 + 4];
        let mut rst = vec![0u64; sz / 8 + 4];
        let mut inbuf = mkdata(shape, 90_000, &mut rng);
        let base = inbuf.as_mut_ptr();
        let a = unsafe { crss(cst.as_mut_ptr() as *mut u8, base) };
        let b = unsafe { rrss(rst.as_mut_ptr() as *mut u8, base) };
        assert_eq!(a, b, "LZ4_resetStreamStateHC");
        let mut off = 0usize;
        let mut k = 0;
        while off < 90_000 {
            let n = 9000usize.min(90_000 - off);
            let bound = unsafe { cb(n as i32) } as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let cn = unsafe { cc(cst.as_mut_ptr() as *mut u8, base.add(off), cd.as_mut_ptr(), n as i32) };
            let rn = unsafe { rc(rst.as_mut_ptr() as *mut u8, base.add(off), rd.as_mut_ptr(), n as i32) };
            same(&format!("resetStreamStateHC {shape:?} k={k}"), cn as i64, &cd, rn as i64, &rd);
            off += n;
            k += 1;
        }
    }

    // Rows 112, 114: LZ4_createHC / LZ4_compressHC2_continue / freeHC.
    //
    // These are the *degraded* deprecated entry points: they call
    // LZ4HC_compress_generic directly, so they perform none of the auto-init,
    // extDict rotation or overflow handling that LZ4_compress_HC_continue does.
    // Consequently they are only defined for a single contiguous input buffer
    // that is never slid, and the compression level must not change the
    // strategy mid-stream. (Verified: the C library itself nul-derefs if
    // LZ4_compressHC2_continue is called after LZ4_slideInputBufferHC, because
    // LZ4_resetStreamHC_fast sets prefixStart = NULL and the degraded path never
    // re-inits.) Row 115 therefore tests LZ4_slideInputBufferHC separately.
    for &lvl in &[0i32, 2, 9, 12] {
        let (ccr, rcr) = syms::<FnCreateHC>("LZ4_createHC");
        let (cfr, rfr) = syms::<FnFreeHC>("LZ4_freeHC");
        let (cc2, rc2) = syms::<FnCont5L>("LZ4_compressHC2_continue");
        let (cc2l, rc2l) = syms::<FnCont6>("LZ4_compressHC2_limitedOutput_continue");
        let mut inbuf = mkdata(Shape::Textish, 200_000, &mut rng);
        let base = inbuf.as_mut_ptr();
        let cst = unsafe { ccr(base) };
        let rst = unsafe { rcr(base) };
        assert!(!cst.is_null() && !rst.is_null());
        let mut off = 0usize;
        for k in 0..10 {
            let n = 20000usize.min(200_000 - off);
            if n == 0 {
                break;
            }
            let bound = unsafe { cb(n as i32) } as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let (cn, rn) = if k % 2 == 0 {
                (
                    unsafe { cc2(cst, base.add(off), cd.as_mut_ptr(), n as i32, lvl) },
                    unsafe { rc2(rst, base.add(off), rd.as_mut_ptr(), n as i32, lvl) },
                )
            } else {
                (
                    unsafe { cc2l(cst, base.add(off), cd.as_mut_ptr(), n as i32, bound as i32, lvl) },
                    unsafe { rc2l(rst, base.add(off), rd.as_mut_ptr(), n as i32, bound as i32, lvl) },
                )
            };
            same(&format!("compressHC2_continue lvl={lvl} k={k}"), cn as i64, &cd, rn as i64, &rd);
            off += n;
        }
        assert_eq!(unsafe { cfr(cst) }, unsafe { rfr(rst) }, "LZ4_freeHC");
    }
    assert_eq!(
        unsafe { syms::<FnFreeHC>("LZ4_freeHC").0(std::ptr::null_mut()) },
        unsafe { syms::<FnFreeHC>("LZ4_freeHC").1(std::ptr::null_mut()) },
        "LZ4_freeHC(NULL)"
    );

    // Row 115: LZ4_slideInputBufferHC. The returned pointer (an offset into the
    // caller's buffer) is the observable behaviour; the call also truncates the
    // stream history via LZ4_resetStreamHC_fast. Resume with the *non*-degraded
    // LZ4_compressHC_continue, which re-inits when prefixStart == NULL.
    for &lvl in &[2i32, 9, 12] {
        let (ccr, rcr) = syms::<FnCreateHC>("LZ4_createHC");
        let (cfr, rfr) = syms::<FnFreeHC>("LZ4_freeHC");
        let (csl, rsl) = syms::<FnSlideHC>("LZ4_slideInputBufferHC");
        let (cc, rc) = syms::<FnCont4>("LZ4_compressHC_continue");
        let (csl2, rsl2) = syms::<FnSetLevel>("LZ4_setCompressionLevel");
        let mut inbuf = mkdata(Shape::Textish, 200_000, &mut rng);
        let base = inbuf.as_mut_ptr();
        let cst = unsafe { ccr(base) };
        let rst = unsafe { rcr(base) };
        unsafe { csl2(cst, lvl) };
        unsafe { rsl2(rst, lvl) };
        let mut off = 0usize;
        for k in 0..8 {
            let n = 20000usize.min(200_000 - off);
            if n == 0 {
                break;
            }
            let bound = unsafe { cb(n as i32) } as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let cn = unsafe { cc(cst, base.add(off), cd.as_mut_ptr(), n as i32) };
            let rn = unsafe { rc(rst, base.add(off), rd.as_mut_ptr(), n as i32) };
            same(&format!("slideHC pre lvl={lvl} k={k}"), cn as i64, &cd, rn as i64, &rd);
            off += n;
            if k == 2 || k == 5 {
                let cp = unsafe { csl(cst) };
                let rp = unsafe { rsl(rst) };
                assert_eq!(
                    (cp as isize) - (base as isize),
                    (rp as isize) - (base as isize),
                    "LZ4_slideInputBufferHC returned offset (lvl={lvl} k={k})"
                );
                // History is truncated; continue from the reported position.
                off = ((cp as isize) - (base as isize)).max(0) as usize;
            }
        }
        assert_eq!(unsafe { cfr(cst) }, unsafe { rfr(rst) }, "LZ4_freeHC after slide");
    }
}

/// Row 116: `LZ4HC_searchExtDict` — an exported symbol absent from the public
/// header. Called directly with a real HC dictionary context (the
/// `LZ4_streamHC_t` union places `LZ4HC_CCtx_internal` at offset 0).
#[test]
fn g7_hc_search_ext_dict() {
    #[repr(C)]
    #[derive(Debug, PartialEq, Eq, Copy, Clone, Default)]
    struct Match {
        off: i32,
        len: i32,
        back: i32,
    }
    type FnSearch = unsafe extern "C" fn(
        *const u8, // ip
        u32,       // ipIndex
        *const u8, // iLowLimit
        *const u8, // iHighLimit
        *const u8, // dictCtx (LZ4HC_CCtx_internal*)
        u32,       // gDictEndIndex
        i32,       // currentBestML
        i32,       // nbAttempts
    ) -> Match;

    let (c, r) = syms::<FnSearch>("LZ4HC_searchExtDict");
    let (cl, rl) = syms::<FnLoadDictHC>("LZ4_loadDictHC");
    let (crs, rrs) = syms::<FnResetStreamHC>("LZ4_resetStreamHC");
    let mut rng = Rng::new(SEED ^ 0xC);

    for &lvl in &[3i32, 9, 12] {
        for &ds in &[64usize, 4096, 65536] {
            for &shape in &[Shape::Textish, Shape::Periodic, Shape::SmallAlphabet] {
                let dict = mkdata(shape, ds, &mut rng);
                let d = new_hc();
                unsafe { crs(d.c, lvl) };
                unsafe { rrs(d.r, lvl) };
                unsafe { cl(d.c, dict.as_ptr(), ds as i32) };
                unsafe { rl(d.r, dict.as_ptr(), ds as i32) };

                // Search buffer that intentionally shares content with the dict.
                let mut buf = dict[..ds.min(2000)].to_vec();
                buf.extend(mkdata(shape, 500, &mut rng));
                for &probe in &[0usize, 4, 17, 100, 500, 1000] {
                    if probe + 32 >= buf.len() {
                        continue;
                    }
                    for &best in &[3i32, 4, 8, 32] {
                        for &attempts in &[1i32, 4, 64, 256] {
                            for &gdict in &[ds as u32, ds as u32 + 1000, 1 << 20] {
                                let ip = unsafe { buf.as_ptr().add(probe) };
                                let low = buf.as_ptr();
                                let high = unsafe { buf.as_ptr().add(buf.len()) };
                                let ipidx = gdict + probe as u32;
                                let cm = unsafe { c(ip, ipidx, low, high, d.c, gdict, best, attempts) };
                                let rm = unsafe { r(ip, ipidx, low, high, d.r, gdict, best, attempts) };
                                assert_eq!(
                                    cm, rm,
                                    "LZ4HC_searchExtDict lvl={lvl} ds={ds} {shape:?} probe={probe} \
                                     best={best} attempts={attempts} gdict={gdict}: C={cm:?} R={rm:?}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
