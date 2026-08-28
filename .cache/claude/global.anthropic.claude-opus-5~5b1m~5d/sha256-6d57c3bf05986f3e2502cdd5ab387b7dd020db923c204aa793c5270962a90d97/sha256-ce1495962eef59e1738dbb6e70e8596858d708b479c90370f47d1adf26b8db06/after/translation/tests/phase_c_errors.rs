//! Phase C — one differential test per row of `ERRORS.md` that can be observed
//! in-process (the rows that end in `SIGSEGV`/`SIGABRT` live in
//! `tests/phase_c_crash.rs`, which compares the wait-status of two child
//! processes).

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

// row 1 ------------------------------------------------------------------------
#[test]
fn err_01_arrgrowf_noop() {
    let (c, r, _g) = libs();
    unsafe {
        // a == NULL and min_cap == 0 -> `min_cap <= stbds_arrcap(NULL)` -> NULL
        for elemsize in [0usize, 1, 8, 4096] {
            assert!(
                (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0).is_null(),
                "C must return NULL"
            );
            assert!(
                (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0).is_null(),
                "Rust must return NULL"
            );
        }
        // existing array: any request already covered by the capacity is a no-op
        let a = (c.arrgrowf)(std::ptr::null_mut(), 8, 0, 10);
        let b = (r.arrgrowf)(std::ptr::null_mut(), 8, 0, 10);
        let cap = read_header(a as *mut u8).capacity;
        assert_eq!(cap, read_header(b as *mut u8).capacity);
        for min_cap in 0..=cap {
            assert_eq!((c.arrgrowf)(a, 8, 0, min_cap), a);
            assert_eq!((r.arrgrowf)(b, 8, 0, min_cap), b);
        }
        // addlen that still fits is a no-op too
        assert_eq!((c.arrgrowf)(a, 8, cap, 0), a);
        assert_eq!((r.arrgrowf)(b, 8, cap, 0), b);
        (c.arrfreef)(a);
        (r.arrfreef)(b);
    }
}

// row 3 ------------------------------------------------------------------------
#[test]
fn err_03_arrgrowf_zero_elemsize() {
    let (c, r, _g) = libs();
    unsafe {
        let a = (c.arrgrowf)(std::ptr::null_mut(), 0, 0, 1) as *mut u8;
        let b = (r.arrgrowf)(std::ptr::null_mut(), 0, 0, 1) as *mut u8;
        let ha = read_header(a);
        let hb = read_header(b);
        assert_eq!((ha.length, ha.capacity, ha.temp), (0, 4, 0));
        assert_eq!((hb.length, hb.capacity, hb.temp), (0, 4, 0));
        assert!(ha.hash_table.is_null() && hb.hash_table.is_null());
        (c.arrfreef)(a as *mut c_void);
        (r.arrfreef)(b as *mut c_void);
    }
}

// row 6 ------------------------------------------------------------------------
#[test]
fn err_06_hash_string_empty() {
    let (c, r, _g) = libs();
    unsafe {
        let empty = b"\0";
        for seed in [0usize, 1, 2, 0x3141_5926, usize::MAX] {
            let hc = (c.hash_string)(empty.as_ptr() as *mut c_char, seed);
            let hr = (r.hash_string)(empty.as_ptr() as *mut c_char, seed);
            assert_eq!(hc, hr, "hash_string(\"\", {:#x})", seed);
        }
    }
}

// row 7 ------------------------------------------------------------------------
#[test]
fn err_07_hash_bytes_null_len0() {
    let (c, r, _g) = libs();
    unsafe {
        // len == 0 never dereferences `p`: NULL is a *valid* input here
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            let hc = (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let hr = (r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(hc, hr, "hash_bytes(NULL, 0, {:#x})", seed);
            // and identical to a non-NULL pointer with len 0
            let buf = [0u8; 1];
            assert_eq!(hc, (c.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed));
            assert_eq!(hr, (r.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed));
        }
    }
}

// row 8 ------------------------------------------------------------------------
#[test]
fn err_08_hash_bytes_short() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(8);
    unsafe {
        // one buffer of exactly `len` bytes: a read past `len` would be caught
        // by the differing content of the following heap bytes
        for len in 0..8usize {
            for _ in 0..500 {
                let v = rng.bytes(len);
                let seed = rng.next_u64() as usize;
                let p = if len == 0 {
                    std::ptr::null_mut()
                } else {
                    v.as_ptr() as *mut c_void
                };
                assert_eq!(
                    (c.hash_bytes)(p, len, seed),
                    (r.hash_bytes)(p, len, seed),
                    "hash_bytes({:?}, {}, {:#x})",
                    v,
                    len,
                    seed
                );
            }
        }
    }
}

// row 9 ------------------------------------------------------------------------
#[test]
fn err_09_mode_out_of_range() {
    let (c, r, _g) = libs();
    unsafe {
        // `mode` is an `int` on the FFI boundary: every value must dispatch the
        // same way in both libraries (>=1 string, <1 binary)
        let modes = [
            c_int::MIN,
            -1000,
            -2,
            -1,
            0,
            1,
            2,
            3,
            4,
            255,
            256,
            1000,
            c_int::MAX,
        ];
        let mut keys = Keys::new();
        for &mode in &modes {
            let string_input = mode >= STBDS_HM_STRING;
            let cfg = MapCfg {
                elemsize: 16,
                keysize: 8,
                keyoffset: 0,
                valoff: 8,
                valsize: 8,
                mode,
                smode: if string_input { STBDS_SH_DEFAULT } else { 0 },
            };
            let mut m = DualMap::new_lazy(c, r, cfg, 0x900);
            let k1 = if string_input {
                keys.string(b"abcdefgh")
            } else {
                keys.raw(b"abcdefgh")
            };
            let k2 = if string_input {
                keys.string(b"ABCDEFGH")
            } else {
                keys.raw(b"ABCDEFGH")
            };
            m.put(k1, &1u64.to_le_bytes());
            m.put(k2, &2u64.to_le_bytes());
            assert_eq!(m.get(k1), 0);
            assert_eq!(m.get(k2), 1);
            // the *other* dispatch class on the same map must miss identically
            // (a binary hash never matches a string hash)
            let other = MapCfg {
                mode: if string_input { 0 } else { 1 },
                ..cfg
            };
            let saved = m.cfg;
            m.cfg = MapCfg { smode: saved.smode, ..other };
            assert_eq!(m.get(k1), -1, "cross-dispatch lookup must miss (mode={})", mode);
            m.cfg = saved;
            m.free();
        }
    }
}

// row 10 -----------------------------------------------------------------------
#[test]
fn err_10_keysize_zero() {
    let (c, r, _g) = libs();
    unsafe {
        let cfg = MapCfg::binary(8, 0, 0, 8, STBDS_HM_BINARY);
        let mut keys = Keys::new();
        let mut m = DualMap::new_lazy(c, r, cfg, 0x1000);
        // memcmp(...,0) == 0: every key is "equal", so the map has one entry
        for i in 0..20u64 {
            let t = m.put(keys.raw(&i.to_le_bytes()), &i.to_le_bytes());
            assert_eq!(t, 0, "keysize 0 must always update entry 0");
            assert_eq!(m.len(), 1);
        }
        assert_eq!(m.get(keys.raw(b"anything")), 0);
        m.free();
    }
}

// row 11 -----------------------------------------------------------------------
#[test]
fn err_11_hmfree_null() {
    let (c, r, _g) = libs();
    unsafe {
        for elemsize in [0usize, 1, 16, 1 << 20] {
            (c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

// rows 12 + 15 + 16 ------------------------------------------------------------
#[test]
fn err_12_15_16_find_slot_miss() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(12);
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
            let mut m = DualMap::new_lazy(c, r, cfg, 0x1200);
            for i in 0..40u64 {
                let k = if cfg.string_input() {
                    keys.string(format!("present-{}", i).as_bytes())
                } else {
                    keys.raw(&i.to_le_bytes())
                };
                m.put(k, &i.to_le_bytes());
            }
            for i in 0..200u64 {
                let k = if cfg.string_input() {
                    keys.string(format!("absent-{}-{}", i, rng.byte()).as_bytes())
                } else {
                    keys.raw(&(0xF000_0000_0000_0000u64 + i).to_le_bytes())
                };
                // hmgeti / shgeti sentinel is -1 (STBDS_INDEX_EMPTY)
                assert_eq!(m.get(k), -1);
                assert_eq!(m.get_ts(k), -1);
            }
            m.free();
        }
    }
}

// rows 13 + 14 -----------------------------------------------------------------
#[test]
fn err_13_14_get_ts_sentinels() {
    let (c, r, _g) = libs();
    unsafe {
        let cfg = MapCfg::binary(16, 8, 8, 8, STBDS_HM_BINARY);
        let mut keys = Keys::new();
        let key = keys.raw(b"whatever");

        // row 13: a == NULL -> allocates, *temp = -1, returns non-NULL
        for &mode in &[0, 1, 2, -1] {
            let mut tc: isize = 0x7777;
            let mut tr: isize = 0x7777;
            let pc = (c.hmget_key_ts)(std::ptr::null_mut(), 16, key, 8, &mut tc, mode);
            let pr = (r.hmget_key_ts)(std::ptr::null_mut(), 16, key, 8, &mut tr, mode);
            assert!(!pc.is_null() && !pr.is_null());
            assert_eq!(tc, -1);
            assert_eq!(tr, -1);
            let hc = read_header((pc as *mut u8).sub(16));
            let hr = read_header((pr as *mut u8).sub(16));
            assert_eq!(hc.length, 1);
            assert_eq!(hr.length, 1);
            assert!(hc.hash_table.is_null() && hr.hash_table.is_null());
            // element 0 was memset to 0
            assert_eq!(
                std::slice::from_raw_parts((pc as *mut u8).sub(16), 16),
                &[0u8; 16]
            );
            assert_eq!(
                std::slice::from_raw_parts((pr as *mut u8).sub(16), 16),
                &[0u8; 16]
            );
            (c.hmfree_func)((pc as *mut u8).sub(16) as *mut c_void, 16);
            (r.hmfree_func)((pr as *mut u8).sub(16) as *mut c_void, 16);
        }

        // row 14: map without a hash table (built by hmput_default)
        let mut m = DualMap::new_lazy(c, r, cfg, 0x1400);
        m.put_default(&0u64.to_le_bytes());
        assert_eq!(m.get_ts(key), -1);
        assert_eq!(m.get(key), -1);
        m.free();
    }
}

// rows 17 + 18 -----------------------------------------------------------------
#[test]
fn err_17_18_put_default() {
    let (c, r, _g) = libs();
    unsafe {
        for elemsize in [1usize, 8, 16, 40] {
            // row 17: NULL -> allocate
            let pc = (c.hmput_default)(std::ptr::null_mut(), elemsize) as *mut u8;
            let pr = (r.hmput_default)(std::ptr::null_mut(), elemsize) as *mut u8;
            assert!(!pc.is_null() && !pr.is_null());
            let hc = read_header(pc.sub(elemsize));
            let hr = read_header(pr.sub(elemsize));
            assert_eq!((hc.length, hc.capacity, hc.temp), (1, 4, 0));
            assert_eq!((hr.length, hr.capacity, hr.temp), (1, 4, 0));
            assert_eq!(
                std::slice::from_raw_parts(pc.sub(elemsize), elemsize),
                std::slice::from_raw_parts(pr.sub(elemsize), elemsize)
            );
            // row 18: already initialised -> unchanged pointer, no allocation
            assert_eq!((c.hmput_default)(pc as *mut c_void, elemsize) as *mut u8, pc);
            assert_eq!((r.hmput_default)(pr as *mut c_void, elemsize) as *mut u8, pr);
            let hc2 = read_header(pc.sub(elemsize));
            assert_eq!((hc2.length, hc2.capacity), (1, 4));
            (c.hmfree_func)(pc.sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)(pr.sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// row 19 -----------------------------------------------------------------------
#[test]
fn err_19_smode_default_branch() {
    let (c, r, _g) = libs();
    unsafe {
        // `table->string.mode` outside {1,2,3}: the `switch default:` copies
        // `keysize` *bytes* of the key even when mode >= STBDS_HM_STRING
        for smode in [0i32, 4, 5, 42, 255] {
            let cfg = MapCfg {
                elemsize: 16,
                keysize: 8,
                keyoffset: 0,
                valoff: 8,
                valsize: 8,
                mode: STBDS_HM_STRING,
                smode,
            };
            let mut keys = Keys::new();
            let mut m = DualMap::new_shmode(c, r, cfg, 0x1900, smode);
            let key = keys.string(b"0123456789abcdef");
            m.put(key, &7u64.to_le_bytes());
            // the element must hold the first 8 *bytes of the string*
            for t in [m.c, m.r] {
                assert_eq!(std::slice::from_raw_parts(t, 8), b"01234567");
            }
            m.free();
        }
    }
}

// rows 20..23 + 25 + 31 --------------------------------------------------------
// The six live `assert()`s that cannot be reached through the exported API. The
// test verifies the invariant that keeps them from firing - in *both* libraries -
// across the same workloads that Phase B uses.
#[test]
fn err_20_to_25_and_31_documented_unreachable() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(2025);
    unsafe {
        // row 20: for every reachable slot_count (8 << k) the assert holds
        let mut sc = 8usize;
        while sc <= 1 << 40 {
            let uct = sc - (sc >> 2);
            let tct = (sc >> 3) + (sc >> 4);
            assert!(uct + tct < sc, "slot_count {} would fire the assert", sc);
            sc <<= 1;
        }

        let cfg = MapCfg::binary(16, 8, 8, 8, STBDS_HM_BINARY);
        let mut keys = Keys::new();
        let mut m = DualMap::new_lazy(c, r, cfg, 0x2025);
        let pool: Vec<*mut c_void> = (0..300u64).map(|i| keys.raw(&i.to_le_bytes())).collect();
        for step in 0..3000 {
            if step % 3 == 2 && m.len() > 0 {
                m.del(pool[rng.below(pool.len())]);
            } else {
                m.put(pool[rng.below(pool.len())], &rng.bytes(8));
            }
            for t in [m.c, m.r] {
                let h = read_header(t.sub(cfg.elemsize));
                // row 21: length never exceeds the capacity
                assert!(h.length <= h.capacity, "length {} > capacity {}", h.length, h.capacity);
                if h.hash_table.is_null() {
                    continue;
                }
                let ti = std::ptr::read_unaligned(h.hash_table as *const HashIndex);
                // row 20 (live table), row 23 (used_count never underflows)
                assert!(ti.used_count_threshold + ti.tombstone_count_threshold < ti.slot_count);
                assert!(ti.used_count <= ti.slot_count, "used_count underflowed");
                assert!(ti.tombstone_count <= ti.slot_count);
                assert_eq!(ti.used_count, h.length - 1);
                for b in 0..(ti.slot_count >> 3) {
                    let bk = std::ptr::read_unaligned(ti.storage.add(b));
                    for j in 0..8 {
                        // rows 22 + 25: every in-use slot points at a live entry
                        if bk.index[j] >= 0 {
                            assert!(
                                (bk.index[j] as usize) < h.length - 1,
                                "bucket index {} out of range (len {})",
                                bk.index[j],
                                h.length
                            );
                            assert!(bk.hash[j] >= 2, "reserved hash value in a live slot");
                        }
                    }
                }
            }
        }
        m.free();

        // row 31: `len <= a->remaining` holds after every stralloc
        for api in [c, r] {
            let mut arena = Arena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            for _ in 0..2000 {
                let n = rng.below(700);
                let mut s = vec![b'x'; n];
                s.push(0);
                let before = arena.remaining;
                let p = (api.stralloc)(&mut arena, s.as_ptr() as *mut c_char);
                assert!(!p.is_null());
                // either a fresh block was taken, or the bump stayed in range
                assert!(arena.remaining <= before.max(1 << 20));
                assert_eq!(*(p.add(n) as *const u8), 0);
            }
            (api.strreset)(&mut arena);
        }
    }
}

// row 26 -----------------------------------------------------------------------
#[test]
fn err_26_del_null_map() {
    let (c, r, _g) = libs();
    unsafe {
        let mut keys = Keys::new();
        let k = keys.raw(b"key");
        for &mode in &[c_int::MIN, -1, 0, 1, 2, c_int::MAX] {
            for &elemsize in &[0usize, 1, 16] {
                assert!((c.hmdel_key)(std::ptr::null_mut(), elemsize, k, 8, 0, mode).is_null());
                assert!((r.hmdel_key)(std::ptr::null_mut(), elemsize, k, 8, 0, mode).is_null());
            }
        }
    }
}

// rows 27 + 28 -----------------------------------------------------------------
#[test]
fn err_27_28_del_no_table_and_miss() {
    let (c, r, _g) = libs();
    unsafe {
        let cfg = MapCfg::binary(16, 8, 8, 8, STBDS_HM_BINARY);
        let mut keys = Keys::new();
        let key = keys.raw(b"absent!!");

        // row 27: table == NULL -> temp = 0, pointer unchanged
        let mut m = DualMap::new_lazy(c, r, cfg, 0x2700);
        m.put_default(&0u64.to_le_bytes());
        let (pc, pr) = (m.c, m.r);
        assert_eq!(m.del(key), 0);
        assert_eq!((m.c, m.r), (pc, pr));

        // row 28: key not present -> temp = 0, map untouched
        m.put(keys.raw(b"present!"), &1u64.to_le_bytes());
        for _ in 0..50 {
            assert_eq!(m.del(key), 0);
            assert_eq!(m.len(), 1);
        }
        m.free();
    }
}

// row 29 -----------------------------------------------------------------------
#[test]
fn err_29_del_mode2_no_free() {
    let (c, r, _g) = libs();
    unsafe {
        // mode == 2 on a STRDUP map: the guard is `mode == STBDS_HM_STRING`, so
        // the duplicated key is *not* freed. Deleting the last entry keeps
        // `old_index == final_index`, so no re-lookup happens (ERRORS.md row 24).
        let cfg = MapCfg::strmap(16, 8, 8, STBDS_HM_PTR_TO_STRING, STBDS_SH_STRDUP);
        let mut keys = Keys::new();
        let mut m = DualMap::new_shmode(c, r, cfg, 0x2900, STBDS_SH_STRDUP);
        m.put(keys.string(b"first"), &1u64.to_le_bytes());
        m.put(keys.string(b"second"), &2u64.to_le_bytes());
        // delete the last entry (index 1)
        let stored = std::ptr::read_unaligned(m.c.add(16) as *const *const c_char);
        let copy = {
            let mut v = Vec::new();
            let mut i = 0isize;
            while *(stored.offset(i) as *const u8) != 0 {
                v.push(*(stored.offset(i) as *const u8));
                i += 1;
            }
            keys.string(&v)
        };
        assert_eq!(m.del(copy), 1);
        assert_eq!(m.len(), 1);
        // a double free of the (leaked) key would abort inside hmfree_func
        m.free();
    }
}

// row 30 -----------------------------------------------------------------------
#[test]
fn err_30_del_bad_keyoffset() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(30);
    unsafe {
        for keyoffset in [1usize, 2, 4, 7, 8] {
            let cfg = MapCfg {
                elemsize: 16,
                keysize: 8,
                keyoffset,
                valoff: 8,
                valsize: 8,
                mode: STBDS_HM_BINARY,
                smode: 0,
            };
            let mut keys = Keys::new();
            let mut m = DualMap::new_lazy(c, r, cfg, 0x3000);
            let mut ks = Vec::new();
            for i in 0..10u64 {
                let k = keys.raw(&i.to_le_bytes());
                ks.push(k);
                m.put(k, &rng.bytes(8));
            }
            // the wrong bytes are hashed/compared -> miss, map unchanged
            for &k in &ks {
                assert_eq!(m.del(k), 0, "keyoffset {} must not delete", keyoffset);
                assert_eq!(m.len(), 10);
            }
            m.cfg = MapCfg { keyoffset: 0, ..cfg };
            for &k in &ks {
                assert_eq!(m.del(k), 1);
            }
            m.free();
        }
    }
}

// row 32 -----------------------------------------------------------------------
#[test]
fn err_32_stralloc_oversized() {
    let (c, r, _g) = libs();
    unsafe {
        for api in [c, r] {
            let mut a = Arena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: 0,
                mode: 0,
            };
            // empty arena + oversized string: remaining stays 0
            let big = {
                let mut v = vec![b'A'; 5000];
                v.push(0);
                v
            };
            let p = (api.stralloc)(&mut a, big.as_ptr() as *mut c_char);
            assert_eq!(a.remaining, 0);
            assert_eq!(a.block, 1);
            assert!(!a.storage.is_null());
            assert_eq!(*(p.add(4999) as *const u8), b'A');
            // non-empty arena + oversized string: remaining preserved
            let small = b"small\0";
            (api.stralloc)(&mut a, small.as_ptr() as *mut c_char);
            let rem = a.remaining;
            (api.stralloc)(&mut a, big.as_ptr() as *mut c_char);
            assert_eq!(a.remaining, rem);
            (api.strreset)(&mut a);
        }
    }
}

// rows 34 + 41 -----------------------------------------------------------------
#[test]
fn err_34_41_out_of_range_bytes() {
    let (c, r, _g) = libs();
    unsafe {
        // row 41: `stbds_shmode_func` truncates `mode` to an unsigned char
        for mode in [
            c_int::MIN,
            -1,
            0,
            1,
            2,
            3,
            4,
            255,
            256,
            257,
            1000,
            c_int::MAX,
        ] {
            let pc = (c.shmode_func)(16, mode) as *mut u8;
            let pr = (r.shmode_func)(16, mode) as *mut u8;
            let hc = read_header(pc.sub(16));
            let hr = read_header(pr.sub(16));
            let ic = std::ptr::read_unaligned(hc.hash_table as *const HashIndex);
            let ir = std::ptr::read_unaligned(hr.hash_table as *const HashIndex);
            assert_eq!(ic.string.mode, (mode as u32 & 0xff) as u8, "C truncation");
            assert_eq!(ir.string.mode, (mode as u32 & 0xff) as u8, "Rust truncation");
            assert_eq!(ic.slot_count, ir.slot_count);
            (c.hmfree_func)(pc.sub(16) as *mut c_void, 16);
            (r.hmfree_func)(pr.sub(16) as *mut c_void, 16);
        }
        // row 34: `a->block` out of range (shift count >= 64 in the C)
        for block in [0u8, 1, 30, 31, 128, 129, 158, 159] {
            let mut ac = Arena { storage: std::ptr::null_mut(), remaining: 0, block, mode: 0 };
            let mut ar = Arena { storage: std::ptr::null_mut(), remaining: 0, block, mode: 0 };
            let s = b"block-field\0";
            (c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            (r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_eq!(
                (ac.remaining, ac.block),
                (ar.remaining, ar.block),
                "block={} diverged",
                block
            );
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
    }
}

// row 36 -----------------------------------------------------------------------
#[test]
fn err_36_strreset_empty() {
    let (c, r, _g) = libs();
    unsafe {
        for api in [c, r] {
            let mut a = Arena {
                storage: std::ptr::null_mut(),
                remaining: 12345,
                block: 7,
                mode: 3,
            };
            (api.strreset)(&mut a);
            assert!(a.storage.is_null());
            assert_eq!(a.remaining, 0);
            assert_eq!(a.block, 0);
            assert_eq!(a.mode, 0);
            (api.strreset)(&mut a);
            assert_eq!(a.remaining, 0);
        }
    }
}

// row 38 -----------------------------------------------------------------------
#[test]
fn err_38_strkey_extremes() {
    let (c, r, _g) = libs();
    unsafe {
        for n in [c_int::MIN, c_int::MIN + 1, -1, 0, c_int::MAX] {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            let mut a = Vec::new();
            let mut b = Vec::new();
            let mut i = 0isize;
            while *(pc.offset(i) as *const u8) != 0 {
                a.push(*(pc.offset(i) as *const u8));
                i += 1;
            }
            i = 0;
            while *(pr.offset(i) as *const u8) != 0 {
                b.push(*(pr.offset(i) as *const u8));
                i += 1;
            }
            assert_eq!(a, b);
            assert!(a.len() < 256, "must fit the 256 byte static buffer");
            assert_eq!(a, format!("test_{}", n).into_bytes());
        }
    }
}

// row 40 -----------------------------------------------------------------------
// The `if (hash < 2) hash += 2` remap cannot be *triggered* without inverting
// SipHash-2-4, but the mapping it feeds is verified exactly: for every entry the
// bucket slot must hold the value produced by the exported hash function.
#[test]
fn err_40_reserved_hash_values() {
    let (c, r, _g) = libs();
    let mut rng = Rng::new(40);
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
            let mut m = DualMap::new_lazy(c, r, cfg, 0x4000);
            let mut ks = Vec::new();
            for i in 0..60u64 {
                let k = if cfg.string_input() {
                    keys.string(format!("hash-{}-{}", i, rng.byte()).as_bytes())
                } else {
                    keys.raw(&rng.bytes(8))
                };
                ks.push(k);
                m.put(k, &i.to_le_bytes());
            }
            for (t, api) in [(m.c, c), (m.r, r)] {
                let h = read_header(t.sub(cfg.elemsize));
                let ti = std::ptr::read_unaligned(h.hash_table as *const HashIndex);
                for (i, &k) in ks.iter().enumerate() {
                    let mut hash = if cfg.string_input() {
                        (api.hash_string)(k as *mut c_char, ti.seed)
                    } else {
                        (api.hash_bytes)(k, cfg.keysize, ti.seed)
                    };
                    if hash < 2 {
                        hash += 2;
                    }
                    // the entry must live in a slot carrying exactly this hash
                    let mut found = false;
                    for b in 0..(ti.slot_count >> 3) {
                        let bk = std::ptr::read_unaligned(ti.storage.add(b));
                        for j in 0..8 {
                            if bk.hash[j] == hash && bk.index[j] >= 0 {
                                found = true;
                            }
                        }
                    }
                    assert!(found, "[{}] no slot with the computed hash for key #{}", api.name, i);
                }
            }
            m.free();
        }
    }
}

// row 42 -----------------------------------------------------------------------
#[test]
fn err_42_put_zero_elemsize() {
    let (c, r, _g) = libs();
    unsafe {
        let cfg = MapCfg {
            elemsize: 0,
            keysize: 0,
            keyoffset: 0,
            valoff: 0,
            valsize: 0,
            mode: STBDS_HM_BINARY,
            smode: 0,
        };
        let mut keys = Keys::new();
        let mut m = DualMap::new_lazy(c, r, cfg, 0x4200);
        for i in 0..10u64 {
            let t = m.put(keys.raw(&i.to_le_bytes()), &[]);
            assert_eq!(t, 0, "keysize 0 -> everything is the same key");
        }
        assert_eq!(m.get(keys.raw(b"x")), 0);
        assert_eq!(m.del(keys.raw(b"x")), 1);
        m.free();
    }
}
