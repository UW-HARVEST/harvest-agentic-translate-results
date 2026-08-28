//! Phase B — CONFIGS.md rows 15..33: the hash map insert/lookup surface, driven
//! through the low-level entry points (`stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmput_default`, `stbds_shmode_func`).
//!
//! After *every* call the complete observable state of both libraries is
//! compared (header, hash index, every bucket slot, arena, payload).

mod common;

use common::*;
use std::ffi::{c_int, c_void};

#[derive(Clone, Copy)]
enum Create {
    /// map materialised by the first `stbds_hmput_key(NULL, ...)`
    Lazy,
    /// `stbds_shmode_func(elemsize, mode)`
    ShMode(c_int),
    /// `stbds_hmput_default(NULL, elemsize)`
    PutDefault,
}

unsafe fn make<'a>(
    c: &'a Api,
    r: &'a Api,
    cfg: MapCfg,
    seed: usize,
    how: Create,
) -> DualMap<'a> {
    match how {
        Create::Lazy => DualMap::new_lazy(c, r, cfg, seed),
        Create::ShMode(m) => DualMap::new_shmode(c, r, cfg, seed, m),
        Create::PutDefault => {
            let mut m = DualMap::new_lazy(c, r, cfg, seed);
            m.put_default(&vec![0u8; cfg.valsize]);
            m
        }
    }
}

/// Insert `pool` distinct keys `ops` times (picked at random ⇒ updates as well),
/// verifying every intermediate state, then look every key up plus `pool`
/// absent keys.
unsafe fn workload(
    c: &Api,
    r: &Api,
    cfg: MapCfg,
    how: Create,
    seed: usize,
    pool: usize,
    ops: usize,
    rng: &mut Rng,
) {
    let mut keys = Keys::new();
    let mut pool_ptrs = Vec::new();
    let mut absent = Vec::new();
    for i in 0..pool {
        if cfg.string_input() {
            let n = 1 + rng.below(20);
            let mut s: Vec<u8> = (0..n).map(|_| 1 + (rng.byte() % 255)).collect();
            s.extend_from_slice(format!("#{}", i).as_bytes()); // guarantee distinct
            pool_ptrs.push(keys.string(&s));
            let mut a: Vec<u8> = (0..n).map(|_| 1 + (rng.byte() % 255)).collect();
            a.extend_from_slice(format!("absent#{}", i).as_bytes());
            absent.push(keys.string(&a));
        } else {
            let mut k = rng.bytes(cfg.keysize);
            if cfg.keysize >= 4 {
                k[..4].copy_from_slice(&(i as u32).to_le_bytes()); // distinct
            }
            pool_ptrs.push(keys.raw(&k));
            let mut a = rng.bytes(cfg.keysize);
            if cfg.keysize >= 4 {
                a[..4].copy_from_slice(&(0x8000_0000u32 + i as u32).to_le_bytes());
            }
            absent.push(keys.raw(&a));
        }
    }

    let mut m = make(c, r, cfg, seed, how);
    for _ in 0..ops {
        let k = pool_ptrs[rng.below(pool_ptrs.len().max(1)).min(pool_ptrs.len() - 1)];
        let v = rng.bytes(cfg.valsize);
        m.put(k, &v);
    }
    for &k in &pool_ptrs {
        m.get(k);
        m.get_ts(k);
    }
    for &k in &absent {
        m.get(k);
        m.get_ts(k);
    }
    m.free();
}

// rows 15..19 ------------------------------------------------------------------
#[test]
fn cfg_15_to_19_binary_counts() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_15);
    let cfg = MapCfg::binary(16, 8, 8, 8, STBDS_HM_BINARY);
    unsafe {
        // exactly at the growth boundaries: threshold(8)=6, threshold(16)=12,
        // threshold(32)=24, threshold(64)=48
        for n in [1usize, 2, 5, 6, 7, 11, 12, 13, 23, 24, 25, 47, 48, 49] {
            for seed in [0usize, 0x3141_5926, 0xDEAD_BEEF] {
                workload(c, r, cfg, Create::Lazy, seed, n, n, &mut rng);
            }
        }
        // row 19: 1000 elements, random keys
        workload(c, r, cfg, Create::Lazy, 0x1234, 1000, 1000, &mut rng);
    }
}

// row 20 -----------------------------------------------------------------------
#[test]
fn cfg_20_binary_updates() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_20);
    let cfg = MapCfg::binary(16, 8, 8, 8, STBDS_HM_BINARY);
    unsafe {
        // 20 distinct keys, 600 puts ⇒ mostly updates of existing entries
        workload(c, r, cfg, Create::Lazy, 7, 20, 600, &mut rng);
        workload(c, r, cfg, Create::Lazy, 8, 3, 200, &mut rng);
        workload(c, r, cfg, Create::Lazy, 9, 1, 50, &mut rng);
    }
}

// row 21 -----------------------------------------------------------------------
#[test]
fn cfg_21_keysize_one() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_21);
    // struct { char key; int value; } -> elemsize 8, key at 0, value at 4
    let cfg = MapCfg::binary(8, 1, 4, 4, STBDS_HM_BINARY);
    unsafe {
        workload(c, r, cfg, Create::Lazy, 0x3141_5926, 256, 1500, &mut rng);
        workload(c, r, cfg, Create::Lazy, 1, 7, 200, &mut rng);
    }
}

// row 22 -----------------------------------------------------------------------
#[test]
fn cfg_22_shapes() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_22);
    unsafe {
        let shapes: &[(usize, usize, usize, usize)] = &[
            (8, 2, 4, 4),
            (8, 4, 4, 4),
            (16, 2, 8, 8),
            (16, 4, 8, 4),
            (24, 16, 16, 8),
            (32, 16, 16, 16),
            (32, 4, 8, 24),
            (40, 8, 8, 32),
            (9, 1, 1, 8),
            (3, 1, 1, 2),
        ];
        for &(elemsize, keysize, valoff, valsize) in shapes {
            let cfg = MapCfg::binary(elemsize, keysize, valoff, valsize, STBDS_HM_BINARY);
            for n in [1usize, 6, 13, 40] {
                workload(c, r, cfg, Create::Lazy, 0x999, n, n * 2, &mut rng);
            }
        }
    }
}

// row 23 -----------------------------------------------------------------------
#[test]
fn cfg_23_keysize_zero() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_23);
    // keysize 0: memcmp(...,0)==0 so every key is "equal" -> one single entry
    let cfg = MapCfg::binary(8, 0, 0, 8, STBDS_HM_BINARY);
    unsafe {
        workload(c, r, cfg, Create::Lazy, 5, 30, 200, &mut rng);
    }
}

// row 24 -----------------------------------------------------------------------
#[test]
fn cfg_24_mode_negative() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_24);
    unsafe {
        for mode in [-1, c_int::MIN, -1000] {
            let cfg = MapCfg::binary(16, 8, 8, 8, mode);
            for n in [1usize, 6, 13, 60] {
                workload(c, r, cfg, Create::Lazy, 0x424, n, n * 2, &mut rng);
            }
        }
    }
}

// rows 25 + 26 -----------------------------------------------------------------
#[test]
fn cfg_25_26_string_default_mode() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_25);
    // struct { char *key; long value; }
    let cfg = MapCfg::strmap(16, 8, 8, STBDS_HM_STRING, STBDS_SH_DEFAULT);
    unsafe {
        for n in [1usize, 2, 5, 6, 7, 12, 13, 25, 200] {
            for seed in [0usize, 0x3141_5926] {
                workload(c, r, cfg, Create::Lazy, seed, n, n * 2, &mut rng);
            }
        }
        // row 26: identical *content* at different addresses must update, and
        // the stored key pointer must stay the first one
        let mut keys = Keys::new();
        let mut m = DualMap::new_lazy(c, r, cfg, 42);
        let first = keys.string(b"duplicate-content");
        m.put(first, &1u64.to_le_bytes());
        for i in 0..20u64 {
            let dup = keys.string(b"duplicate-content");
            assert_ne!(dup, first);
            let t = m.put(dup, &(i + 2).to_le_bytes());
            assert_eq!(t, 0, "must update entry 0");
            assert_eq!(m.len(), 1);
            // DEFAULT mode keeps the *original* pointer
            let stored = std::ptr::read_unaligned(m.c as *const *mut c_void);
            assert_eq!(stored, first);
            let stored_r = std::ptr::read_unaligned(m.r as *const *mut c_void);
            assert_eq!(stored_r, first);
        }
        m.free();
        // empty-string keys
        let mut m = DualMap::new_lazy(c, r, cfg, 43);
        let e1 = keys.string(b"");
        let e2 = keys.string(b"");
        m.put(e1, &7u64.to_le_bytes());
        assert_eq!(m.put(e2, &8u64.to_le_bytes()), 0);
        m.get(e2);
        m.free();
    }
}

// row 27 -----------------------------------------------------------------------
#[test]
fn cfg_27_mode_above_string() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_27);
    unsafe {
        for mode in [STBDS_HM_PTR_TO_STRING, 3, 7, c_int::MAX] {
            // lazily created ⇒ string.mode == STBDS_SH_DEFAULT
            let cfg = MapCfg::strmap(16, 8, 8, mode, STBDS_SH_DEFAULT);
            for n in [1usize, 6, 13, 50] {
                workload(c, r, cfg, Create::Lazy, 0x427, n, n * 2, &mut rng);
            }
            // deleting the *last* element is well defined for mode != 1
            // (`old_index == final_index` ⇒ no re-lookup, see ERRORS.md row 24)
            let mut keys = Keys::new();
            let mut m = DualMap::new_lazy(c, r, cfg, 0x428);
            let mut ks = Vec::new();
            for i in 0..6 {
                let k = keys.string(format!("key{}", i).as_bytes());
                ks.push(k);
                m.put(k, &(i as u64).to_le_bytes());
            }
            while m.len() > 0 {
                let last = m.len() as usize - 1;
                let stored =
                    std::ptr::read_unaligned(m.c.add(16 * last) as *const *mut c_void);
                assert_eq!(m.del(stored), 1);
            }
            m.free();
        }
    }
}

// row 28 -----------------------------------------------------------------------
#[test]
fn cfg_28_strdup_mode() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_28);
    let cfg = MapCfg::strmap(16, 8, 8, STBDS_HM_STRING, STBDS_SH_STRDUP);
    unsafe {
        for n in [1usize, 6, 13, 200] {
            workload(
                c,
                r,
                cfg,
                Create::ShMode(STBDS_SH_STRDUP),
                0x428,
                n,
                n * 2,
                &mut rng,
            );
        }
        // empty and long keys
        let mut keys = Keys::new();
        let mut m = DualMap::new_shmode(c, r, cfg, 1, STBDS_SH_STRDUP);
        m.put(keys.string(b""), &0u64.to_le_bytes());
        m.put(keys.string(&[b'x'; 100]), &1u64.to_le_bytes());
        m.put(keys.string(&[b'y'; 1000]), &2u64.to_le_bytes());
        m.free();
    }
}

// row 29 -----------------------------------------------------------------------
#[test]
fn cfg_29_arena_mode() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_29);
    let cfg = MapCfg::strmap(16, 8, 8, STBDS_HM_STRING, STBDS_SH_ARENA);
    unsafe {
        for n in [1usize, 6, 13, 200] {
            workload(
                c,
                r,
                cfg,
                Create::ShMode(STBDS_SH_ARENA),
                0x429,
                n,
                n * 2,
                &mut rng,
            );
        }
        // block overflow inside the arena: many short keys, then keys larger
        // than the current block (512), then larger than the maximum block
        let mut keys = Keys::new();
        let mut m = DualMap::new_shmode(c, r, cfg, 2, STBDS_SH_ARENA);
        for i in 0..300u64 {
            m.put(keys.string(format!("k{}", i).as_bytes()), &i.to_le_bytes());
        }
        m.put(keys.string(&[b'a'; 600]), &1u64.to_le_bytes());
        m.put(keys.string(&[b'b'; 4000]), &2u64.to_le_bytes());
        m.put(keys.string(&[b'c'; 511]), &3u64.to_le_bytes());
        m.put(keys.string(&[b'd'; 512]), &4u64.to_le_bytes());
        m.put(keys.string(&[b'e'; 513]), &5u64.to_le_bytes());
        m.free();
    }
}

// row 30 -----------------------------------------------------------------------
#[test]
fn cfg_30_explicit_default_mode() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_30);
    let cfg = MapCfg::strmap(16, 8, 8, STBDS_HM_STRING, STBDS_SH_DEFAULT);
    unsafe {
        for n in [1usize, 6, 13, 100] {
            workload(
                c,
                r,
                cfg,
                Create::ShMode(STBDS_SH_DEFAULT),
                0x430,
                n,
                n * 2,
                &mut rng,
            );
        }
    }
}

// row 31 -----------------------------------------------------------------------
#[test]
fn cfg_31_string_mode_out_of_range() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_31);
    unsafe {
        // (a) binary keys in a map whose string.mode is an out-of-range byte:
        //     `switch default:` == memcpy(key, keysize) -- fully defined
        for smode in [STBDS_SH_NONE, 4, 5, 100, 255, 256, 257, -1, c_int::MIN, c_int::MAX] {
            let cfg = MapCfg {
                elemsize: 16,
                keysize: 8,
                keyoffset: 0,
                valoff: 8,
                valsize: 8,
                mode: STBDS_HM_BINARY,
                smode: (smode as u8) as c_int,
            };
            for n in [1usize, 6, 13, 40] {
                workload(c, r, cfg, Create::ShMode(smode), 0x431, n, n * 2, &mut rng);
            }
        }
        // (b) string *hashing* (mode=1) but a `default:` storage branch: the
        //     first 8 bytes of the string are memcpy'ed into the element.
        //     Only distinct keys are inserted: a duplicate would make
        //     `stbds_is_key_equal` strcmp() a garbage pointer in *both*
        //     libraries (documented in ERRORS.md row 19).
        for smode in [STBDS_SH_NONE, 4, 255] {
            let cfg = MapCfg {
                elemsize: 16,
                keysize: 8,
                keyoffset: 0,
                valoff: 8,
                valsize: 8,
                mode: STBDS_HM_STRING,
                smode: (smode as u8) as c_int,
            };
            let mut keys = Keys::new();
            let mut m = DualMap::new_shmode(c, r, cfg, 0x432, smode);
            for i in 0..30u64 {
                let k = keys.string(format!("distinct-key-{:04}", i).as_bytes());
                m.put(k, &i.to_le_bytes());
            }
            m.free();
        }
    }
}

// row 32 -----------------------------------------------------------------------
#[test]
fn cfg_32_put_default_path() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_32);
    unsafe {
        let cfg = MapCfg::binary(16, 8, 8, 8, STBDS_HM_BINARY);
        for n in [0usize, 1, 6, 13, 40] {
            workload(c, r, cfg, Create::PutDefault, 0x432, n.max(1), n, &mut rng);
        }
        // hmdefault on an *existing* map must be a no-op, and the default value
        // survives every rehash
        let mut keys = Keys::new();
        let mut m = DualMap::new_lazy(c, r, cfg, 0x433);
        m.put_default(&0xAABB_CCDD_EEFF_0011u64.to_le_bytes());
        for i in 0..40u64 {
            m.put(keys.raw(&i.to_le_bytes()), &i.to_le_bytes());
            if i % 7 == 0 {
                m.put_default(&(0x1000 + i).to_le_bytes());
            }
        }
        m.free();
        // put_default on a NULL map, twice
        let mut m = DualMap::new_lazy(c, r, cfg, 0x434);
        m.put_default(&1u64.to_le_bytes());
        m.put_default(&2u64.to_le_bytes());
        m.get(keys.raw(&99u64.to_le_bytes()));
        m.free();
    }
}

// row 33 -----------------------------------------------------------------------
#[test]
fn cfg_33_hmget_key_ts() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(0xB_33);
    unsafe {
        for (mode, smode) in [(STBDS_HM_BINARY, 0), (STBDS_HM_STRING, STBDS_SH_DEFAULT)] {
            let cfg = MapCfg {
                elemsize: 16,
                keysize: 8,
                keyoffset: 0,
                valoff: 8,
                valsize: 8,
                mode,
                smode,
            };
            let mut keys = Keys::new();
            let key = if cfg.string_input() {
                keys.string(b"the-key")
            } else {
                keys.raw(&7u64.to_le_bytes())
            };
            // (a) NULL map: allocates, *temp = -1
            let mut m = DualMap::new_lazy(c, r, cfg, 0x433);
            assert_eq!(m.get_ts(key), -1);
            assert_eq!(m.len(), 0);
            // (b) map without a hash table (hmput_default) : *temp = -1
            m.put_default(&0u64.to_le_bytes());
            assert_eq!(m.get_ts(key), -1);
            // (c) hit / (d) miss
            m.put(key, &0x55u64.to_le_bytes());
            assert_eq!(m.get_ts(key), 0);
            let missing = if cfg.string_input() {
                keys.string(b"nope")
            } else {
                keys.raw(&8u64.to_le_bytes())
            };
            assert_eq!(m.get_ts(missing), -1);
            m.free();

            // randomized mix of both entry points
            let mut m = DualMap::new_lazy(c, r, cfg, 0x434);
            let mut pool = Vec::new();
            for i in 0..50u64 {
                pool.push(if cfg.string_input() {
                    keys.string(format!("s{}", i).as_bytes())
                } else {
                    keys.raw(&i.to_le_bytes())
                });
            }
            for _ in 0..400 {
                let k = pool[rng.below(pool.len())];
                match rng.below(3) {
                    0 => {
                        let v = rng.bytes(8);
                        m.put(k, &v);
                    }
                    1 => {
                        m.get(k);
                    }
                    _ => {
                        m.get_ts(k);
                    }
                }
            }
            m.free();
        }
    }
}
