//! Minimal deterministic reproducers for the currently-failing HC paths.
//! (kept small so a fix can be verified in one second)
mod common;
use common::*;

type FnCreateStreamHC = unsafe extern "C" fn() -> *mut u8;
type FnFreeStreamHC = unsafe extern "C" fn(*mut u8) -> i32;
type FnResetStreamHC = unsafe extern "C" fn(*mut u8, i32);
type FnHCContinue = unsafe extern "C" fn(*mut u8, *const u8, *mut u8, i32, i32) -> i32;
type FnLoadDictHC = unsafe extern "C" fn(*mut u8, *const u8, i32) -> i32;

/// Non-contiguous `LZ4_compress_HC_continue` -> `LZ4HC_setExternalDict`.
#[test]
fn repro_hc_extdict() {
    let (cc, rc) = syms::<FnHCContinue>("LZ4_compress_HC_continue");
    let (crs, rrs) = syms::<FnResetStreamHC>("LZ4_resetStreamHC");
    let (ccr, rcr) = syms::<FnCreateStreamHC>("LZ4_createStreamHC");
    let (cfr, rfr) = syms::<FnFreeStreamHC>("LZ4_freeStreamHC");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");

    for lvl in [2i32, 3, 9, 12] {
        let cs = unsafe { ccr() };
        let rs = unsafe { rcr() };
        unsafe { crs(cs, lvl) };
        unsafe { rrs(rs, lvl) };
        let mut rng = Rng::new(7);
        let mut arena = BlockArena::new();
        for k in 0..8 {
            let n = 11000 + k * 137;
            let blk = arena.keep(mkdata(Shape::Periodic, n, &mut rng));
            let bound = unsafe { cb(n as i32) } as usize;
            let mut cd = vec![0u8; bound];
            let mut rd = vec![0u8; bound];
            let cn = unsafe { cc(cs, blk.as_ptr(), cd.as_mut_ptr(), n as i32, bound as i32) };
            let rn = unsafe { rc(rs, blk.as_ptr(), rd.as_mut_ptr(), n as i32, bound as i32) };
            same(&format!("repro extdict lvl={lvl} k={k} n={n}"), cn as i64, &cd, rn as i64, &rd);
        }
        unsafe { cfr(cs) };
        unsafe { rfr(rs) };
    }
}

/// `LZ4_loadDictHC` with a tiny (<4 byte) dictionary at level 2 (lz4mid).
#[test]
fn repro_hc_loaddict_small() {
    let (cl, rl) = syms::<FnLoadDictHC>("LZ4_loadDictHC");
    let (cc, rc) = syms::<FnHCContinue>("LZ4_compress_HC_continue");
    let (crs, rrs) = syms::<FnResetStreamHC>("LZ4_resetStreamHC");
    let (ccr, rcr) = syms::<FnCreateStreamHC>("LZ4_createStreamHC");
    let (cfr, rfr) = syms::<FnFreeStreamHC>("LZ4_freeStreamHC");
    let (cb, _) = syms::<FnCompressBound>("LZ4_compressBound");

    for lvl in [1i32, 2, 3, 9, 12] {
        for ds in [0usize, 1, 2, 3, 4, 5, 100] {
            let mut rng = Rng::new(11);
            let dict = mkdata(Shape::Textish, ds, &mut rng);
            let cs = unsafe { ccr() };
            let rs = unsafe { rcr() };
            unsafe { crs(cs, lvl) };
            unsafe { rrs(rs, lvl) };
            let a = unsafe { cl(cs, dict.as_ptr(), ds as i32) };
            let b = unsafe { rl(rs, dict.as_ptr(), ds as i32) };
            assert_eq!(a, b, "loadDictHC lvl={lvl} ds={ds}");
            let mut arena = BlockArena::new();
            for k in 0..4 {
                let n = 500 + k * 13;
                let blk = arena.keep(mkdata(Shape::Textish, n, &mut rng));
                let bound = unsafe { cb(n as i32) } as usize;
                let mut cd = vec![0u8; bound];
                let mut rd = vec![0u8; bound];
                let cn = unsafe { cc(cs, blk.as_ptr(), cd.as_mut_ptr(), n as i32, bound as i32) };
                let rn = unsafe { rc(rs, blk.as_ptr(), rd.as_mut_ptr(), n as i32, bound as i32) };
                same(&format!("repro loaddict lvl={lvl} ds={ds} k={k}"), cn as i64, &cd, rn as i64, &rd);
            }
            unsafe { cfr(cs) };
            unsafe { rfr(rs) };
        }
    }
}
