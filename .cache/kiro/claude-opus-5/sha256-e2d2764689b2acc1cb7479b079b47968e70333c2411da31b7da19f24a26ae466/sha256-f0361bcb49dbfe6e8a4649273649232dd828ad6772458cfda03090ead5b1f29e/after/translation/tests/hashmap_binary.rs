//! Phase B rows 20-34, 47, 51: the binary-key (`STBDS_HM_BINARY`) hash map,
//! driven through `stbds_hmput_key` / `stbds_hmget_key` / `stbds_hmget_key_ts`
//! / `stbds_hmdel_key` / `stbds_hmput_default` / `stbds_hmfree_func`.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

const SEED: u64 = 0x5EED_1234;

fn key_of(v: u64, keysize: usize) -> Vec<u8> {
    let b = v.to_le_bytes();
    (0..keysize).map(|i| b[i % 8]).collect()
}

#[test]
fn cfg_20_put_default_fresh() {
    let h = setup(0x3141_5926);
    for elemsize in [1usize, 4, 8, 16, 32, 64] {
        let mut m = MapPair::new(&h.c, &h.r, elemsize, 4, false);
        m.put_default();
        m.check(&format!("hmput_default fresh e={}", elemsize));
        m.free();
    }
}

#[test]
fn cfg_21_put_default_idempotent() {
    let h = setup(0x3141_5926);
    for elemsize in [8usize, 16, 32] {
        let mut m = MapPair::new(&h.c, &h.r, elemsize, 4, false);
        m.put_default();
        let first = m.desc();
        for _ in 0..5 {
            m.put_default();
        }
        m.check(&format!("hmput_default idempotent e={}", elemsize));
        assert_eq!(first.0, m.desc().0, "second hmput_default must be a no-op");
        m.free();
    }
}

#[test]
fn cfg_22_bin_single() {
    let h = setup(0x3141_5926);
    let mut m = MapPair::new(&h.c, &h.r, 8, 4, false);
    let mut keys = Keys::new();
    let kb = key_of(0xDEAD_BEEF, 4);
    let k = keys.add_bytes(&kb);
    let t = m.put(k, &kb, HM_BINARY, 0x11);
    assert_eq!(t, 0, "first insert must land at index 0");
    m.check("bin single");
    assert_eq!(m.get(k, HM_BINARY), 0);
    m.check("bin single after get");
    m.free();
}

#[test]
fn cfg_23_bin_to_threshold() {
    let h = setup(0x3141_5926);
    // 8 slots, used_count_threshold = 8 - 2 = 6: the 6th insert triggers growth
    let mut m = MapPair::new(&h.c, &h.r, 8, 4, false);
    let mut keys = Keys::new();
    for i in 0..10u64 {
        let kb = key_of(i, 4);
        let k = keys.add_bytes(&kb);
        m.put(k, &kb, HM_BINARY, i as u8);
        m.check(&format!("bin threshold insert {}", i));
    }
    m.free();
}

#[test]
fn cfg_24_bin_many_grow() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 24);
    let mut m = MapPair::new(&h.c, &h.r, 8, 4, false);
    let mut keys = Keys::new();
    let mut used: Vec<Vec<u8>> = Vec::new();
    for i in 0..400u64 {
        let kb = key_of(rng.next_u64(), 4);
        let k = keys.add_bytes(&kb);
        m.put(k, &kb, HM_BINARY, (i & 0xff) as u8);
        used.push(kb);
        if i % 17 == 0 {
            m.check(&format!("bin many insert {}", i));
        }
    }
    m.check("bin many final");
    // every inserted key must resolve identically
    for kb in &used {
        let k = keys.add_bytes(kb);
        let t = m.get(k, HM_BINARY);
        assert!(t >= 0, "inserted key vanished");
    }
    m.check("bin many after gets");
    m.free();
}

#[test]
fn cfg_25_bin_keysize8() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 25);
    let mut m = MapPair::new(&h.c, &h.r, 16, 8, false);
    let mut keys = Keys::new();
    for i in 0..300u64 {
        let kb = key_of(rng.next_u64(), 8);
        let k = keys.add_bytes(&kb);
        m.put(k, &kb, HM_BINARY, (i & 0xff) as u8);
    }
    m.check("bin keysize8");
    m.free();
}

#[test]
fn cfg_26_bin_keysize16() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 26);
    // stbds_struct2 shape: int key[2] ... but exercise a real 16-byte key
    let mut m = MapPair::new(&h.c, &h.r, 32, 16, false);
    let mut keys = Keys::new();
    for i in 0..300u64 {
        let mut kb = key_of(rng.next_u64(), 8);
        kb.extend(key_of(rng.next_u64(), 8));
        let k = keys.add_bytes(&kb);
        m.put(k, &kb, HM_BINARY, (i & 0xff) as u8);
    }
    m.check("bin keysize16");
    m.free();
}

#[test]
fn cfg_27_bin_odd_keysizes() {
    let h = setup(0x3141_5926);
    for keysize in [1usize, 2, 3, 5, 6, 7, 9, 12, 15] {
        let mut rng = Rng::new(SEED ^ (0x2700 + keysize as u64));
        let elemsize = keysize + 8;
        let mut m = MapPair::new(&h.c, &h.r, elemsize, keysize, false);
        let mut keys = Keys::new();
        // keysize 1 has only 256 distinct keys; keep the count modest so the
        // table does not fill with duplicates only
        for i in 0..200u64 {
            let kb = key_of(rng.next_u64(), keysize);
            let k = keys.add_bytes(&kb);
            m.put(k, &kb, HM_BINARY, (i & 0xff) as u8);
        }
        m.check(&format!("bin odd keysize={}", keysize));
        m.free();
    }
}

#[test]
fn cfg_28_bin_duplicates() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 28);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let mut pool: Vec<Vec<u8>> = (0..40u64).map(|i| key_of(i * 7 + 1, 4)).collect();
    pool.push(key_of(0, 4));
    for i in 0..600u64 {
        let kb = pool[rng.below(pool.len())].clone();
        let k = keys.add_bytes(&kb);
        m.put(k, &kb, HM_BINARY, (i & 0xff) as u8);
        if i % 23 == 0 {
            m.check(&format!("bin dup op {}", i));
        }
    }
    m.check("bin dup final");
    m.free();
}

#[test]
fn cfg_29_bin_get_all() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 29);
    let mut m = MapPair::new(&h.c, &h.r, 8, 4, false);
    let mut keys = Keys::new();
    let present: Vec<Vec<u8>> = (0..250u64).map(|_| key_of(rng.next_u64(), 4)).collect();
    for (i, kb) in present.iter().enumerate() {
        let k = keys.add_bytes(kb);
        m.put(k, kb, HM_BINARY, i as u8);
    }
    for kb in &present {
        let k = keys.add_bytes(kb);
        assert!(m.get(k, HM_BINARY) >= 0);
    }
    // absent keys must all report -1 in both libraries
    for _ in 0..500 {
        let kb = key_of(rng.next_u64(), 4);
        if present.contains(&kb) {
            continue;
        }
        let k = keys.add_bytes(&kb);
        assert_eq!(m.get(k, HM_BINARY), -1, "absent key must report -1");
    }
    m.check("bin get all");
    m.free();
}

#[test]
fn cfg_30_bin_get_ts() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 30);
    let mut m = MapPair::new(&h.c, &h.r, 8, 4, false);
    let mut keys = Keys::new();
    let present: Vec<Vec<u8>> = (0..250u64).map(|_| key_of(rng.next_u64(), 4)).collect();
    for (i, kb) in present.iter().enumerate() {
        let k = keys.add_bytes(kb);
        m.put(k, kb, HM_BINARY, i as u8);
    }
    // hmget_key_ts must NOT touch header->temp
    let before = m.desc();
    for kb in &present {
        let k = keys.add_bytes(kb);
        assert!(m.get_ts(k, HM_BINARY) >= 0);
    }
    for _ in 0..500 {
        let kb = key_of(rng.next_u64(), 4);
        if present.contains(&kb) {
            continue;
        }
        let k = keys.add_bytes(&kb);
        assert_eq!(m.get_ts(k, HM_BINARY), -1);
    }
    let after = m.desc();
    assert_same("bin get_ts", &after.0, &after.1);
    assert_eq!(before.0, after.0, "hmget_key_ts must not mutate the map");
    m.free();

    // fresh (a == NULL) path of hmget_key_ts
    let mut m2 = MapPair::new(&h.c, &h.r, 8, 4, false);
    let kb = key_of(1, 4);
    let k = keys.add_bytes(&kb);
    assert_eq!(m2.get_ts(k, HM_BINARY), -1);
    m2.check("bin get_ts fresh");
    m2.free();
}

#[test]
fn cfg_31_bin_del_basic() {
    let h = setup(0x3141_5926);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let ks: Vec<Vec<u8>> = (0..5u64).map(|i| key_of(1000 + i, 4)).collect();
    for (i, kb) in ks.iter().enumerate() {
        let k = keys.add_bytes(kb);
        m.put(k, kb, HM_BINARY, i as u8);
    }
    m.check("bin del setup");

    // absent
    let miss = key_of(999_999, 4);
    let km = keys.add_bytes(&miss);
    assert_eq!(m.del(km, 0, HM_BINARY), 0, "absent delete -> temp 0");
    m.check("bin del absent");

    // last element
    let kl = keys.add_bytes(&ks[4]);
    assert_eq!(m.del(kl, 0, HM_BINARY), 1);
    m.check("bin del last");

    // first element (forces the swap + re-find path)
    let kf = keys.add_bytes(&ks[0]);
    assert_eq!(m.del(kf, 0, HM_BINARY), 1);
    m.check("bin del first");

    // deleting again must now report not-found
    let kf2 = keys.add_bytes(&ks[0]);
    assert_eq!(m.del(kf2, 0, HM_BINARY), 0);
    m.check("bin del twice");

    // drain the rest
    for i in [1usize, 2, 3] {
        let k = keys.add_bytes(&ks[i]);
        assert_eq!(m.del(k, 0, HM_BINARY), 1);
        m.check(&format!("bin drain {}", i));
    }
    m.free();
}

#[test]
fn cfg_32_bin_storm() {
    // Randomised put/get/get_ts/del storm: reaches table growth, tombstone
    // rebuilds and shrinks. Verified after every single operation.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 32);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    let pool: Vec<Vec<u8>> = (0..500u64).map(|i| key_of(i.wrapping_mul(0x9E37_79B9), 4)).collect();
    for op in 0..4000u64 {
        let kb = pool[rng.below(pool.len())].clone();
        let k = keys.add_bytes(&kb);
        match rng.below(10) {
            0..=4 => {
                m.put(k, &kb, HM_BINARY, (op & 0xff) as u8);
            }
            5..=6 => {
                m.get(k, HM_BINARY);
            }
            7 => {
                m.get_ts(k, HM_BINARY);
            }
            _ => {
                m.del(k, 0, HM_BINARY);
            }
        }
        m.check(&format!("bin storm op {}", op));
        if keys.bufs.len() > 8000 {
            keys.bufs.clear();
        }
    }
    m.free();
}

#[test]
fn cfg_33_bin_keyoffset() {
    // stbds_hmdel_key takes an explicit keyoffset (STBDS_OFFSETOF(t,key)).
    // Drive a layout where the key does NOT sit at offset 0.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 33);
    let elemsize = 16usize;
    let keysize = 4usize;
    for keyoffset in [0usize, 4, 8, 12] {
        let mut m = MapPair::new(&h.c, &h.r, elemsize, keysize, false);
        let mut keys = Keys::new();
        // Insert normally (hmput_key hard-codes keyoffset 0) but then delete
        // with the offset variant, exactly as stbds_hmdel does.
        let ks: Vec<Vec<u8>> = (0..30u64).map(|_| key_of(rng.next_u64(), keysize)).collect();
        for (i, kb) in ks.iter().enumerate() {
            let k = keys.add_bytes(kb);
            m.put(k, kb, HM_BINARY, i as u8);
        }
        m.check(&format!("keyoffset={} setup", keyoffset));
        for kb in ks.iter() {
            let k = keys.add_bytes(kb);
            m.del(k, keyoffset, HM_BINARY);
            m.check(&format!("keyoffset={} del", keyoffset));
        }
        m.free();
    }
}

#[test]
fn cfg_34_bin_free() {
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 34);
    for elemsize in [8usize, 16, 32] {
        for n in [0usize, 1, 5, 6, 7, 100] {
            let mut m = MapPair::new(&h.c, &h.r, elemsize, 4, false);
            let mut keys = Keys::new();
            for i in 0..n {
                let kb = key_of(rng.next_u64(), 4);
                let k = keys.add_bytes(&kb);
                m.put(k, &kb, HM_BINARY, i as u8);
            }
            m.check(&format!("bin free e={} n={}", elemsize, n));
            m.free();
            m.check("bin after free");
        }
    }
}

#[test]
fn cfg_47_shmode_none_binary() {
    // An explicitly created table with string.mode == STBDS_SH_NONE used with
    // binary keys: the `switch` in hmput_key falls to `default:` -> memcpy.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 47);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    m.shmode(SH_NONE);
    m.check("shmode NONE fresh");
    let mut keys = Keys::new();
    for i in 0..200u64 {
        let kb = key_of(rng.next_u64(), 4);
        let k = keys.add_bytes(&kb);
        m.put(k, &kb, HM_BINARY, i as u8);
    }
    m.check("shmode NONE populated");
    m.free();
}

#[test]
fn cfg_51_mode_negative() {
    // mode < STBDS_HM_STRING -> binary path everywhere, for every entry point.
    let h = setup(0x3141_5926);
    for mode in [-1 as c_int, -2, -128, c_int::MIN] {
        let mut rng = Rng::new(SEED ^ 51);
        let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
        let mut keys = Keys::new();
        let ks: Vec<Vec<u8>> = (0..120u64).map(|_| key_of(rng.next_u64(), 4)).collect();
        for (i, kb) in ks.iter().enumerate() {
            let k = keys.add_bytes(kb);
            m.put(k, kb, mode, i as u8);
        }
        m.check(&format!("mode={} put", mode));
        for kb in ks.iter() {
            let k = keys.add_bytes(kb);
            assert!(m.get(k, mode) >= 0);
            m.get_ts(k, mode);
        }
        m.check(&format!("mode={} get", mode));
        for kb in ks.iter() {
            let k = keys.add_bytes(kb);
            m.del(k, 0, mode);
            m.check(&format!("mode={} del", mode));
        }
        m.free();
    }
}

#[test]
fn err_16_put_keysize_zero() {
    // keysize == 0: memcmp(...,0) always returns 0 -> every probe with a
    // matching hash compares "equal". Behaviour must still be identical.
    let h = setup(0x3141_5926);
    let mut m = MapPair::new(&h.c, &h.r, 8, 0, false);
    let mut keys = Keys::new();
    let empty: Vec<u8> = Vec::new();
    for i in 0..20u64 {
        let k = keys.add_bytes(&[0u8; 8]);
        let _ = i;
        m.put(k, &empty, HM_BINARY, 0);
        m.check("keysize 0 put");
    }
    let k = keys.add_bytes(&[0u8; 8]);
    m.get(k, HM_BINARY);
    m.check("keysize 0 get");
    m.del(k, 0, HM_BINARY);
    m.check("keysize 0 del");
    m.free();
}

#[test]
fn err_15_put_capacity_invariant() {
    // The STBDS_ASSERT((size_t)i+1 <= arrcap(a)) in hmput_key: assert the
    // invariant holds in BOTH libraries after every insert.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 15);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    for i in 0..1000u64 {
        let kb = key_of(rng.next_u64(), 4);
        let k = keys.add_bytes(&kb);
        m.put(k, &kb, HM_BINARY, i as u8);
        unsafe {
            for m2 in [m.mc, m.mr] {
                let hd = &*header_of((m2 as *mut u8).sub(16) as *mut c_void);
                assert!(hd.length <= hd.capacity, "capacity invariant violated");
            }
        }
    }
    m.free();
}

#[test]
fn err_52_hash_index_invariant() {
    // STBDS_ASSERT(used_thr + tomb_thr < slot_count) in make_hash_index.
    let h = setup(0x3141_5926);
    let mut rng = Rng::new(SEED ^ 52);
    let mut m = MapPair::new(&h.c, &h.r, 16, 4, false);
    let mut keys = Keys::new();
    for i in 0..1500u64 {
        let kb = key_of(rng.next_u64(), 4);
        let k = keys.add_bytes(&kb);
        if rng.below(3) == 0 {
            m.del(k, 0, HM_BINARY);
        } else {
            m.put(k, &kb, HM_BINARY, i as u8);
        }
        unsafe {
            for mm in [m.mc, m.mr] {
                let hd = &*header_of((mm as *mut u8).sub(16) as *mut c_void);
                if !hd.hash_table.is_null() {
                    let t = &*(hd.hash_table as *const HashIndex);
                    assert!(
                        t.used_count_threshold + t.tombstone_count_threshold < t.slot_count,
                        "make_hash_index invariant violated"
                    );
                    assert!(t.slot_count.is_power_of_two() && t.slot_count >= 8);
                    assert_eq!(t.slot_count_log2, t.slot_count.trailing_zeros() as usize);
                }
            }
        }
    }
    m.free();
}
