//! Phase B — CONFIGS.md rows 34..42: `stbds_hmdel_key` (memmove + index patch,
//! tombstones, shrink and rebuild) and `stbds_hmfree_func`.

mod common;

use common::*;
use std::ffi::{c_int, c_void};

/// The key of the entry currently stored at hash-side index `idx`, copied into
/// caller-owned memory (that is what a real consumer passes to `hmdel`/`shdel`).
unsafe fn key_at(m: &DualMap, keys: &mut Keys, idx: usize) -> *mut c_void {
    let cfg = m.cfg;
    let e = m.c.add(cfg.elemsize * idx);
    if cfg.stores_pointer_key() {
        let p = std::ptr::read_unaligned(e as *const *const u8);
        let mut s = Vec::new();
        let mut i = 0isize;
        loop {
            let b = *p.offset(i);
            if b == 0 {
                break;
            }
            s.push(b);
            i += 1;
        }
        keys.string(&s)
    } else {
        let bytes = std::slice::from_raw_parts(e, cfg.keysize).to_vec();
        keys.raw(&bytes)
    }
}

unsafe fn fill<'a>(
    m: &mut DualMap<'a>,
    keys: &mut Keys,
    n: usize,
    rng: &mut Rng,
) -> Vec<*mut c_void> {
    let mut ks = Vec::new();
    for i in 0..n {
        let k = if m.cfg.string_input() {
            keys.string(format!("key-{:06}-{}", i, rng.byte()).as_bytes())
        } else {
            let mut b = rng.bytes(m.cfg.keysize);
            if m.cfg.keysize >= 4 {
                b[..4].copy_from_slice(&(i as u32).to_le_bytes());
            }
            keys.raw(&b)
        };
        let v = rng.bytes(m.cfg.valsize);
        m.put(k, &v);
        ks.push(k);
    }
    ks
}

const BIN: MapCfg = MapCfg {
    elemsize: 16,
    keysize: 8,
    keyoffset: 0,
    valoff: 8,
    valsize: 8,
    mode: STBDS_HM_BINARY,
    smode: 0,
};

// row 34 -----------------------------------------------------------------------
#[test]
fn cfg_34_del_last() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xD_34);
    unsafe {
        for n in [1usize, 2, 6, 13, 30, 100] {
            let mut keys = Keys::new();
            let mut m = DualMap::new_lazy(c, r, BIN, 0xD34);
            fill(&mut m, &mut keys, n, &mut rng);
            while m.len() > 0 {
                let last = m.len() as usize - 1;
                let k = key_at(&m, &mut keys, last);
                assert_eq!(m.del(k), 1, "delete of the last element must report 1");
            }
            m.free();
        }
    }
}

// row 35 -----------------------------------------------------------------------
#[test]
fn cfg_35_del_middle_and_first() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xD_35);
    unsafe {
        for n in [2usize, 3, 6, 13, 30, 100] {
            // always delete index 0 -> the final element is memmoved into the
            // hole and its bucket index patched
            let mut keys = Keys::new();
            let mut m = DualMap::new_lazy(c, r, BIN, 0xD35);
            fill(&mut m, &mut keys, n, &mut rng);
            while m.len() > 0 {
                let k = key_at(&m, &mut keys, 0);
                assert_eq!(m.del(k), 1);
            }
            m.free();

            // always delete the middle
            let mut m = DualMap::new_lazy(c, r, BIN, 0xD35);
            fill(&mut m, &mut keys, n, &mut rng);
            while m.len() > 0 {
                let mid = (m.len() as usize) / 2;
                let k = key_at(&m, &mut keys, mid);
                assert_eq!(m.del(k), 1);
            }
            m.free();
        }
    }
}

// row 36 -----------------------------------------------------------------------
#[test]
fn cfg_36_del_random_order() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xD_36);
    unsafe {
        for n in [5usize, 13, 40, 200] {
            for seed in [0usize, 0x3141_5926, 0x5EED] {
                let mut keys = Keys::new();
                let mut m = DualMap::new_lazy(c, r, BIN, seed);
                fill(&mut m, &mut keys, n, &mut rng);
                while m.len() > 0 {
                    let i = rng.below(m.len() as usize);
                    let k = key_at(&m, &mut keys, i);
                    assert_eq!(m.del(k), 1);
                }
                m.free();
            }
        }
    }
}

// row 37 -----------------------------------------------------------------------
#[test]
fn cfg_37_shrink_path() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xD_37);
    unsafe {
        // 100 entries -> slot_count 256, used_count_shrink_threshold 64;
        // deleting down to 0 walks 256->128->64->32->16->8
        for n in [100usize, 300, 700] {
            let mut keys = Keys::new();
            let mut m = DualMap::new_lazy(c, r, BIN, 0xD37);
            fill(&mut m, &mut keys, n, &mut rng);
            while m.len() > 0 {
                let i = rng.below(m.len() as usize);
                let k = key_at(&m, &mut keys, i);
                assert_eq!(m.del(k), 1);
            }
            m.free();
        }
    }
}

// row 38 -----------------------------------------------------------------------
#[test]
fn cfg_38_rebuild_path_small_table() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xD_38);
    unsafe {
        // slot_count stays 8 (<=5 live entries): shrink is suppressed
        // (used_count_shrink_threshold == 0) but tombstone_count_threshold == 1,
        // so every second delete triggers the rebuild branch
        for _ in 0..40 {
            let mut keys = Keys::new();
            let mut m = DualMap::new_lazy(c, r, BIN, 0xD38);
            let mut live: Vec<*mut c_void> = Vec::new();
            for _ in 0..200 {
                if live.len() < 5 && rng.below(2) == 0 {
                    let k = keys.raw(&rng.bytes(8));
                    let v = rng.bytes(8);
                    m.put(k, &v);
                    live.push(k);
                } else if !live.is_empty() {
                    let i = rng.below(live.len());
                    let k = key_at(&m, &mut keys, i.min(m.len() as usize - 1));
                    m.del(k);
                    live.remove(i.min(live.len() - 1));
                }
            }
            m.free();
        }
    }
}

// row 39 -----------------------------------------------------------------------
#[test]
fn cfg_39_tombstone_reuse_stress() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xD_39);
    unsafe {
        for seed in [0usize, 1, 0x3141_5926, 0xFFFF_FFFF_FFFF_FFFF] {
            let mut keys = Keys::new();
            let mut m = DualMap::new_lazy(c, r, BIN, seed);
            // a fixed pool so that deleted keys get re-inserted (tombstone reuse)
            let pool: Vec<*mut c_void> = (0..60u64)
                .map(|i| keys.raw(&i.to_le_bytes()))
                .collect();
            for _ in 0..800 {
                match rng.below(3) {
                    0 | 1 => {
                        let k = pool[rng.below(pool.len())];
                        let v = rng.bytes(8);
                        m.put(k, &v);
                    }
                    _ => {
                        let k = pool[rng.below(pool.len())];
                        m.del(k);
                    }
                }
            }
            m.free();
        }
    }
}

// row 40 -----------------------------------------------------------------------
#[test]
fn cfg_40_del_string_maps() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xD_40);
    unsafe {
        for smode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
            let cfg = MapCfg::strmap(16, 8, 8, STBDS_HM_STRING, smode);
            for n in [1usize, 2, 6, 13, 100] {
                // delete last
                let mut keys = Keys::new();
                let mut m = DualMap::new_shmode(c, r, cfg, 0xD40, smode);
                fill(&mut m, &mut keys, n, &mut rng);
                while m.len() > 0 {
                    let k = key_at(&m, &mut keys, m.len() as usize - 1);
                    assert_eq!(m.del(k), 1);
                }
                m.free();

                // delete first (memmove + re-lookup of the moved string key)
                let mut m = DualMap::new_shmode(c, r, cfg, 0xD41, smode);
                fill(&mut m, &mut keys, n, &mut rng);
                while m.len() > 0 {
                    let k = key_at(&m, &mut keys, 0);
                    assert_eq!(m.del(k), 1);
                }
                m.free();

                // random order + re-insert
                let mut m = DualMap::new_shmode(c, r, cfg, 0xD42, smode);
                let ks = fill(&mut m, &mut keys, n, &mut rng);
                for _ in 0..4 * n {
                    if m.len() > 0 && rng.below(2) == 0 {
                        let i = rng.below(m.len() as usize);
                        let k = key_at(&m, &mut keys, i);
                        assert_eq!(m.del(k), 1);
                    } else {
                        let k = ks[rng.below(ks.len())];
                        let v = rng.bytes(8);
                        m.put(k, &v);
                    }
                }
                m.free();
            }
        }
    }
}

// row 41 -----------------------------------------------------------------------
#[test]
fn cfg_41_del_keyoffset_variants() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xD_41);
    unsafe {
        // hmput_key always stores the key at offset 0, so a non-zero keyoffset
        // (the `pshdel` / PTR_TO_STRING layout) makes `stbds_hm_find_slot`
        // compare the *wrong* bytes: both libraries must miss identically.
        for keyoffset in [0usize, 1, 4, 8] {
            let cfg = MapCfg {
                elemsize: 24,
                keysize: 8,
                keyoffset,
                valoff: 8,
                valsize: 16,
                mode: STBDS_HM_BINARY,
                smode: 0,
            };
            let mut keys = Keys::new();
            let mut m = DualMap::new_lazy(c, r, cfg, 0xD43);
            let ks = fill(&mut m, &mut keys, 20, &mut rng);
            for &k in &ks {
                m.del(k);
            }
            // and with the correct offset afterwards
            let cfg0 = MapCfg { keyoffset: 0, ..cfg };
            m.cfg = cfg0;
            while m.len() > 0 {
                let k = key_at(&m, &mut keys, 0);
                assert_eq!(m.del(k), 1);
            }
            m.free();
        }
    }
}

// row 42 -----------------------------------------------------------------------
#[test]
fn cfg_42_hmfree_variants() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xD_42);
    unsafe {
        // binary / DEFAULT / STRDUP / ARENA, after 0, 1 and many inserts and
        // after deletes; a leaked or double-freed block aborts the process
        for n in [0usize, 1, 6, 13, 100] {
            let mut keys = Keys::new();
            let mut m = DualMap::new_lazy(c, r, BIN, 0xD44);
            fill(&mut m, &mut keys, n, &mut rng);
            m.free();

            for smode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
                let cfg = MapCfg::strmap(16, 8, 8, STBDS_HM_STRING, smode);
                let mut m = DualMap::new_shmode(c, r, cfg, 0xD45, smode);
                fill(&mut m, &mut keys, n, &mut rng);
                m.free();

                let mut m = DualMap::new_shmode(c, r, cfg, 0xD46, smode);
                fill(&mut m, &mut keys, n, &mut rng);
                if n > 0 {
                    for _ in 0..n / 2 {
                        let k = key_at(&m, &mut keys, 0);
                        m.del(k);
                    }
                }
                m.free();
            }
        }
        // shmode map that is freed without ever being used
        for smode in [STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA, 42] {
            let cfg = MapCfg::strmap(16, 8, 8, STBDS_HM_STRING, (smode as u8) as c_int);
            let mut m = DualMap::new_shmode(c, r, cfg, 0xD47, smode);
            m.free();
        }
    }
}
