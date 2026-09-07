//! Phase B rows 58-63: full composed pipelines. Options are applied, then the
//! map is driven end-to-end with interleaved put / get / get_ts / del /
//! put_default / arrgrowf operations, verifying after *every* operation. Bugs
//! that only show up in the composed pipeline (rehash while tombstoned, shrink
//! immediately after a swap-delete, arena refill across a table rebuild) are
//! invisible to per-function tests.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

const SEED: u64 = 0x5EED_1234;

fn bin_key(v: u64, keysize: usize) -> Vec<u8> {
    let b = v.to_le_bytes();
    (0..keysize).map(|i| b[i % 8]).collect()
}

/// Interleaved binary-key pipeline.
fn pipeline_binary(h: &Harness, elemsize: usize, keysize: usize, ops: u64, seed: u64) {
    let mut rng = Rng::new(seed);
    let mut m = MapPair::new(&h.c, &h.r, elemsize, keysize, false);
    let mut keys = Keys::new();
    let pool: Vec<Vec<u8>> = (0..300u64)
        .map(|i| bin_key(i.wrapping_mul(0x9E37_79B9_7F4A_7C15), keysize))
        .collect();
    for op in 0..ops {
        let kb = pool[rng.below(pool.len())].clone();
        let k = keys.add_bytes(&kb);
        let what = rng.below(20);
        match what {
            0..=8 => {
                m.put(k, &kb, HM_BINARY, (op & 0xff) as u8);
            }
            9..=12 => {
                m.get(k, HM_BINARY);
            }
            13..=14 => {
                m.get_ts(k, HM_BINARY);
            }
            15..=18 => {
                m.del(k, 0, HM_BINARY);
            }
            _ => {
                m.put_default();
            }
        }
        m.check(&format!(
            "pipeline bin e={} k={} op={} what={}",
            elemsize, keysize, op, what
        ));
        if keys.bufs.len() > 6000 {
            keys.bufs.clear();
        }
    }
    m.free();
}

/// Interleaved string-key pipeline. `shmode` < 0 means "let hmput_key pick
/// SH_DEFAULT implicitly".
fn pipeline_string(h: &Harness, shmode: c_int, mode: c_int, ops: u64, seed: u64) {
    let elemsize = 24usize;
    let mut rng = Rng::new(seed);
    let mut m = MapPair::new(&h.c, &h.r, elemsize, 8, true);
    if shmode >= 0 {
        m.shmode(shmode);
    }
    // SH_DEFAULT stores the caller's pointer, so every key buffer must stay
    // alive for the whole pipeline.
    let mut keys = Keys::new();
    let pool: Vec<String> = (0..300)
        .map(|i| {
            let n = 1 + (i % 90);
            format!("{}_{}", "k".repeat(n), i)
        })
        .collect();
    for op in 0..ops {
        let s = &pool[rng.below(pool.len())];
        let k = keys.add_str(s);
        let what = rng.below(20);
        match what {
            0..=8 => {
                m.put(k, &[], mode, (op & 0xff) as u8);
            }
            9..=12 => {
                m.get(k, mode);
            }
            13..=14 => {
                m.get_ts(k, mode);
            }
            15..=18 => {
                m.del(k, 0, mode);
            }
            _ => {
                m.put_default();
            }
        }
        m.check(&format!(
            "pipeline str shmode={} mode={} op={} what={}",
            shmode, mode, op, what
        ));
    }
    m.free();
}

#[test]
fn cfg_58_pipeline_binary() {
    let h = setup(0x3141_5926);
    pipeline_binary(&h, 8, 4, 4000, SEED ^ 58);
}

#[test]
fn cfg_59_pipeline_str_default() {
    let h = setup(0x3141_5926);
    pipeline_string(&h, -1, HM_STRING, 3000, SEED ^ 59);
    pipeline_string(&h, SH_DEFAULT, HM_STRING, 3000, SEED ^ 0x59);
}

#[test]
fn cfg_60_pipeline_strdup() {
    let h = setup(0x3141_5926);
    pipeline_string(&h, SH_STRDUP, HM_STRING, 3000, SEED ^ 60);
}

#[test]
fn cfg_61_pipeline_arena() {
    let h = setup(0x3141_5926);
    pipeline_string(&h, SH_ARENA, HM_STRING, 3000, SEED ^ 61);
}

#[test]
fn cfg_62_pipeline_elemsize_cross() {
    let h = setup(0x3141_5926);
    for elemsize in [8usize, 16, 24, 32, 64] {
        for keysize in [1usize, 2, 4, 8, 16] {
            if keysize > elemsize {
                continue;
            }
            pipeline_binary(
                &h,
                elemsize,
                keysize,
                300,
                SEED ^ 62 ^ ((elemsize as u64) << 8) ^ keysize as u64,
            );
        }
    }
}

#[test]
fn cfg_63_reseed_midstream() {
    // stbds_rand_seed changes the global LCG. Reseeding *between* table
    // creations must produce the same per-table seeds in both libraries.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 63);
    let elemsize = 16usize;
    for round in 0..12u64 {
        let s = rng.next_u64() as usize;
        unsafe {
            (h.c.rand_seed)(s);
            (h.r.rand_seed)(s);
        }
        let mut m = MapPair::new(&h.c, &h.r, elemsize, 4, false);
        let mut keys = Keys::new();
        for i in 0..250u64 {
            let kb = bin_key(rng.next_u64(), 4);
            let k = keys.add_bytes(&kb);
            m.put(k, &kb, HM_BINARY, i as u8);
            if i % 31 == 0 {
                // reseed midstream: affects only *subsequent* rebuilds
                let s2 = rng.next_u64() as usize;
                unsafe {
                    (h.c.rand_seed)(s2);
                    (h.r.rand_seed)(s2);
                }
            }
        }
        m.check(&format!("reseed round {}", round));
        // interleave deletes so the table rebuilds inherit the seed via `ot`
        for i in 0..120u64 {
            let kb = bin_key(rng.next_u64(), 4);
            let k = keys.add_bytes(&kb);
            if i % 2 == 0 {
                m.del(k, 0, HM_BINARY);
            } else {
                m.put(k, &kb, HM_BINARY, i as u8);
            }
            m.check(&format!("reseed round {} tail {}", round, i));
        }
        m.free();
    }
}

#[test]
fn cfg_31b_mixed_array_and_map() {
    // Interleave plain-array growth with map operations so both allocators run
    // against the same heap in the same order in both libraries.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 0x31b);
    let elemsize = 16usize;
    let mut m = MapPair::new(&h.c, &h.r, elemsize, 4, false);
    let mut ac: *mut c_void = std::ptr::null_mut();
    let mut ar: *mut c_void = std::ptr::null_mut();
    let mut keys = Keys::new();
    for op in 0..1500u64 {
        if rng.below(3) == 0 {
            let addlen = rng.below(5);
            let min_cap = rng.below(50);
            unsafe {
                ac = (h.c.arrgrowf)(ac, 8, addlen, min_cap);
                ar = (h.r.arrgrowf)(ar, 8, addlen, min_cap);
                let dc = describe_arr(ac, 8, 0);
                let dr = describe_arr(ar, 8, 0);
                assert_same(&format!("mixed arr op={}", op), &dc, &dr);
            }
        } else {
            let kb = bin_key(rng.next_u64(), 4);
            let k = keys.add_bytes(&kb);
            if rng.below(4) == 0 {
                m.del(k, 0, HM_BINARY);
            } else {
                m.put(k, &kb, HM_BINARY, (op & 0xff) as u8);
            }
            m.check(&format!("mixed map op={}", op));
        }
    }
    unsafe {
        if !ac.is_null() {
            (h.c.arrfreef)(ac);
        }
        if !ar.is_null() {
            (h.r.arrfreef)(ar);
        }
    }
    m.free();
}
