//! Phase B — CONFIGS.md rows 67..69 (and ERRORS.md row 55): the composed public
//! entry point `hm_geti`, the only symbol declared in `c_src/include/lib.h`.
//!
//! `hm_geti` returns nothing and validates itself with 12 `STBDS_ASSERT`s, so a
//! divergence would show up either as an abort (in exactly one library) or as a
//! different amount of global-seed consumption.  Both are checked:
//!   * the call itself must return normally (no `SIGABRT`) in both libraries, and
//!   * the *global* `stbds_hash_seed` must have advanced identically, which is
//!     observed by building a fresh map afterwards and comparing its
//!     `hash_index.seed` (plus its whole layout).

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const NUMS: [c_int; 33] = [
    c_int::MIN,
    -1000,
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
    10,
    11,
    12,
    13,
    14,
    15,
    16,
    17,
    24,
    25,
    31,
    32,
    33,
    63,
    64,
    65,
    100,
    127,
    128,
    257,
];

/// Builds a probe map and returns its snapshot; the snapshot embeds
/// `hash_index.seed`, i.e. the current value of the library's global seed.
unsafe fn seed_probe(api: &Api) -> String {
    let es = 8usize;
    let cfg = MapCfg::binary(es, 4);
    let mut t: *mut c_void = std::ptr::null_mut();
    for k in 0..3u32 {
        t = map_put_binary(api, t, &cfg, &k.to_le_bytes(), &[1, 2, 3, 4]);
    }
    let s = snapshot_map(t, es, KeyKind::Raw);
    map_free(api, t, es);
    s
}

// --------------------------------------------------------------- row 67
#[test]
fn row67_hm_geti_all_boundaries() {
    let _g = global_lock();
    let (c, r) = load_both();
    unsafe {
        for &num in &NUMS {
            pin_seed(&c, &r, 0x31415926);
            (c.hm_geti)(num);
            (r.hm_geti)(num);
            diff_eq(
                &format!("row67 hm_geti({num}) global-seed effect"),
                &seed_probe(&c),
                &seed_probe(&r),
            );
        }
        // also with a large value (many rehashes inside hm_geti)
        for &num in &[500 as c_int, 1000, 2000] {
            pin_seed(&c, &r, 0x31415926);
            (c.hm_geti)(num);
            (r.hm_geti)(num);
            diff_eq(
                &format!("row67 hm_geti({num}) global-seed effect"),
                &seed_probe(&c),
                &seed_probe(&r),
            );
        }
    }
}

// --------------------------------------------------------------- row 68
#[test]
fn row68_hm_geti_seed_evolution() {
    let _g = global_lock();
    let (c, r) = load_both();
    let mut rng = Rng::new(68);
    unsafe {
        // (a) repeated calls WITHOUT re-pinning: the global seed keeps evolving
        pin_seed(&c, &r, 0xdead_beef);
        for i in 0..25 {
            let num = (i * 7 % 40) as c_int;
            (c.hm_geti)(num);
            (r.hm_geti)(num);
            diff_eq(
                &format!("row68 unpinned iteration {i} (num={num})"),
                &seed_probe(&c),
                &seed_probe(&r),
            );
        }
        // (b) many different pinned seeds
        let mut seeds = vec![0usize, 1, 2, 0x31415926, usize::MAX, usize::MAX - 1];
        for _ in 0..10 {
            seeds.push(rng.next_usize());
        }
        for s in seeds {
            for &num in &[0 as c_int, 1, 9, 33, 64] {
                pin_seed(&c, &r, s);
                (c.hm_geti)(num);
                (r.hm_geti)(num);
                diff_eq(
                    &format!("row68 seed={s:#x} num={num}"),
                    &seed_probe(&c),
                    &seed_probe(&r),
                );
            }
        }
    }
}

// --------------------------------------------------------------- row 69
#[test]
fn row69_hm_geti_interleaved_with_the_rest_of_the_api() {
    let _g = global_lock();
    let (c, r) = load_both();
    let mut rng = Rng::new(69);
    let es = 16usize;
    let cfg = MapCfg::binary(es, 8);
    unsafe {
        pin_seed(&c, &r, 0x0BADC0DE);
        let mut ct: *mut c_void = std::ptr::null_mut();
        let mut rt: *mut c_void = std::ptr::null_mut();
        let mut live: std::collections::BTreeSet<u64> = Default::default();
        for step in 0..400u32 {
            let ctx = format!("row69 step={step}");
            match rng.below(8) {
                0 => {
                    let num = rng.below(40) as c_int;
                    (c.hm_geti)(num);
                    (r.hm_geti)(num);
                    diff_eq(&format!("{ctx} hm_geti({num})"), &seed_probe(&c), &seed_probe(&r));
                }
                1 => {
                    let n = rng.i32();
                    let cp = (c.strkey)(n);
                    let rp = (r.strkey)(n);
                    diff_eq_val(&format!("{ctx} strkey({n})"), cstr(cp), cstr(rp));
                }
                2..=4 => {
                    let k = rng.below(30);
                    ct = map_put_binary(&c, ct, &cfg, &k.to_le_bytes(), &[0x5Au8; 16]);
                    rt = map_put_binary(&r, rt, &cfg, &k.to_le_bytes(), &[0x5Au8; 16]);
                    live.insert(k);
                }
                5..=6 => {
                    let k = rng.below(30);
                    let mut key = k.to_le_bytes();
                    let (nct, ci) = map_geti(&c, ct, &cfg, &mut key);
                    let mut key = k.to_le_bytes();
                    let (nrt, ri) = map_geti(&r, rt, &cfg, &mut key);
                    ct = nct;
                    rt = nrt;
                    diff_eq_val(&format!("{ctx} get({k})"), ci, ri);
                    diff_eq_val(&format!("{ctx} presence({k})"), ci >= 0, live.contains(&k));
                }
                _ => {
                    let k = rng.below(30);
                    let mut key = k.to_le_bytes();
                    let (nct, cr) = map_del(&c, ct, &cfg, &mut key);
                    let mut key = k.to_le_bytes();
                    let (nrt, rr) = map_del(&r, rt, &cfg, &mut key);
                    ct = nct;
                    rt = nrt;
                    diff_eq_val(&format!("{ctx} del({k})"), cr, rr);
                    live.remove(&k);
                }
            }
            diff_eq(&ctx, &snapshot_map(ct, es, KeyKind::Raw), &snapshot_map(rt, es, KeyKind::Raw));
        }
        map_free(&c, ct, es);
        map_free(&r, rt, es);
    }
}
