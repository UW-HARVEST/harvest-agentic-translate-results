//! Phase B — CONFIGS.md rows 31-38, 45-47, 50, 51: STRING-mode maps in every
//! `string.mode` (SH_NONE / SH_DEFAULT / SH_STRDUP / SH_ARENA), created both
//! implicitly (`hmput_key(NULL,…)`) and explicitly (`shmode_func`), plus
//! out-of-range `mode` / `shmode` enum values.

mod common;
use common::*;
use std::ffi::c_int;

fn keyset(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    while v.len() < n {
        // mix short/long, ASCII and high-bit bytes
        let len = 1 + rng.below(24);
        let k = if rng.next_u64() & 1 == 0 {
            rng.ascii(len)
        } else {
            rng.nonnul(len)
        };
        if !v.contains(&k) {
            v.push(k);
        }
    }
    v
}

#[test]
fn row31_string_implicit_sh_default() {
    let (p, _g) = libs();
    for &n in &[1usize, 2, 5, 6, 7, 8, 9, 17, 100, 300] {
        let mut rng = Rng::new(0xC0FFEE ^ 31 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        // implicit creation: hmput_key(NULL, …, mode = STBDS_HM_STRING)
        let mut m = Maps::empty(p, cfg_string());
        let keys = keyset(&mut rng, n);
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
        }
        assert_eq!(
            m.snap_c().table.as_ref().unwrap().arena_mode,
            SH_DEFAULT as u8,
            "implicit STRING map must get string.mode = SH_DEFAULT"
        );
        for k in &keys {
            assert!(m.get(k) >= 0, "key {k:02x?} missing");
        }
        for _ in 0..8 {
            let k = rng.ascii(30);
            if !keys.contains(&k) {
                assert_eq!(m.get(&k), -1);
            }
        }
        m.free();
    }
}

#[test]
fn row32_string_sh_strdup() {
    let (p, _g) = libs();
    for &n in &[1usize, 5, 6, 8, 20, 100, 250] {
        let mut rng = Rng::new(0xC0FFEE ^ 32 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::shmode(p, cfg_string(), SH_STRDUP);
        assert_eq!(
            m.snap_c().table.as_ref().unwrap().arena_mode,
            SH_STRDUP as u8
        );
        let keys = keyset(&mut rng, n);
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
            // the newly stored key must be the duplicate, and temp_key must
            // point at it with identical contents in both libraries
            unsafe {
                let ck = temp_key_of(m.ch, 16).unwrap();
                let rk = temp_key_of(m.rh, 16).unwrap();
                assert_eq!(ck, *k);
                assert_eq!(rk, *k);
                assert_ne!(
                    temp_key_ptr(m.ch, 16) as usize,
                    temp_key_ptr(m.rh, 16) as usize,
                    "STRDUP must allocate independent copies"
                );
            }
        }
        for k in &keys {
            assert!(m.get(k) >= 0);
        }
        // re-put every key: hits the "key already present" branch which also
        // refreshes temp_key from the stored duplicate
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
        }
        m.free();
    }
}

#[test]
fn row33_string_sh_arena() {
    let (p, _g) = libs();
    for &n in &[1usize, 5, 8, 20, 100, 250] {
        let mut rng = Rng::new(0xC0FFEE ^ 33 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::shmode(p, cfg_string(), SH_ARENA);
        assert_eq!(m.snap_c().table.as_ref().unwrap().arena_mode, SH_ARENA as u8);
        let keys = keyset(&mut rng, n);
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
        }
        for k in &keys {
            assert!(m.get(k) >= 0);
        }
        let t = m.snap_c().table.clone().unwrap();
        assert!(t.arena_block_count >= 1, "arena should have blocks");
        m.free();
    }
}

/// `string.mode` values that fall through to the `default:` (raw `memcpy`)
/// branch.  BINARY hashing is used so the reinterpreted bytes are never passed
/// to `strcmp` (which is what the C would do, and would be a wild dereference).
fn memcpy_mode_cfg() -> MapCfg {
    MapCfg {
        elemsize: 16,
        keysize: 8,
        keyoffset: 0,
        put_mode: HM_BINARY,
        del_mode: HM_BINARY,
        fmt: ElemFmt::Raw,
        value_off: 8,
        value_len: 8,
    }
}

#[test]
fn row34_shmode_none_falls_through_to_memcpy() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 34);
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::shmode(p, memcpy_mode_cfg(), SH_NONE);
    assert_eq!(m.snap_c().table.as_ref().unwrap().arena_mode, 0);
    let mut keys = Vec::new();
    for _ in 0..40 {
        let k: Vec<u8> = rng.bytes(8);
        if keys.contains(&k) {
            continue;
        }
        keys.push(k.clone());
        m.put_binary(&k, &rng.bytes(8));
    }
    for k in &keys {
        assert!(m.get(k) >= 0);
    }
    while !keys.is_empty() {
        let k = keys.remove(rng.below(keys.len()));
        assert_eq!(m.del(&k), 1);
    }
    m.free();
}

#[test]
fn row35_shmode_out_of_range_values() {
    let (p, _g) = libs();
    // (shmode, truncated byte, resulting behaviour)
    let cases: &[(c_int, u8)] = &[
        (4, 4),
        (255, 255),
        (256, 0),
        (259, 3),   // == SH_ARENA after truncation
        (-1, 255),
        (-256, 0),
        (513, 1),   // == SH_DEFAULT after truncation
        (770, 2),   // == SH_STRDUP after truncation
        (i32::MIN, 0),
        (i32::MAX, 255),
    ];
    for &(shmode, expect) in cases {
        let mut rng = Rng::new(0xC0FFEE ^ 35 ^ (shmode as i64 as u64));
        reset_seed(p, DEFAULT_SEED);
        let stores_pointer = matches!(expect, 1 | 2 | 3);
        let cfg = if stores_pointer {
            cfg_string()
        } else {
            memcpy_mode_cfg()
        };
        let mut m = Maps::shmode(p, cfg, shmode);
        let cmode = m.snap_c().table.as_ref().unwrap().arena_mode;
        let rmode = m.snap_r().table.as_ref().unwrap().arena_mode;
        assert_eq!(cmode, expect, "shmode_func({shmode}) truncation (C)");
        assert_eq!(rmode, expect, "shmode_func({shmode}) truncation (Rust)");
        if stores_pointer {
            let keys = keyset(&mut rng, 30);
            for k in &keys {
                m.put_string(k, &rng.bytes(8));
            }
            for k in &keys {
                assert!(m.get(k) >= 0, "shmode={shmode} key {k:02x?} missing");
            }
        } else {
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for _ in 0..30 {
                let k = rng.bytes(8);
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k.clone());
                m.put_binary(&k, &rng.bytes(8));
            }
            for k in &keys {
                assert!(m.get(k) >= 0, "shmode={shmode} key {k:02x?} missing");
            }
        }
        // SH_STRDUP maps must be freed through hmfree_func to release the dups
        m.free();
    }
}

#[test]
fn row36_mode_2_ptr_to_string() {
    // mode == 2 (STBDS_HM_PTR_TO_STRING): `mode >= STBDS_HM_STRING` so the
    // STRING hash/compare path is taken, but `mode == STBDS_HM_STRING` is
    // false, so hmdel_key skips the strdup-free and uses the raw-address
    // re-find branch (only safe when deleting the tail element).
    let (p, _g) = libs();
    let mut cfg = cfg_string();
    cfg.put_mode = 2;
    cfg.del_mode = 2;
    for &n in &[1usize, 5, 8, 40, 120] {
        let mut rng = Rng::new(0xC0FFEE ^ 36 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg);
        let keys = keyset(&mut rng, n);
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
        }
        assert_eq!(
            m.snap_c().table.as_ref().unwrap().arena_mode,
            SH_DEFAULT as u8,
            "mode=2 must still yield string.mode = SH_DEFAULT"
        );
        for k in &keys {
            assert!(m.get(k) >= 0);
        }
        // delete strictly from the tail => old_index == final_index
        let mut live = keys.clone();
        while let Some(k) = live.pop() {
            assert_eq!(m.del(&k), 1, "tail delete with mode=2");
        }
        m.free();
    }
}

#[test]
fn row37_mode_negative_is_binary() {
    let (p, _g) = libs();
    for &mode in &[-1i32, -2, i32::MIN, -1000] {
        let mut cfg = cfg_struct();
        cfg.put_mode = mode;
        cfg.del_mode = mode;
        let mut rng = Rng::new(0xC0FFEE ^ 37 ^ (mode as i64 as u64));
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for _ in 0..60 {
            let k = rng.bytes(4);
            if keys.contains(&k) {
                continue;
            }
            keys.push(k.clone());
            m.put_binary(&k, &rng.bytes(12));
        }
        assert_eq!(
            m.snap_c().table.as_ref().unwrap().arena_mode,
            0,
            "mode={mode} must give string.mode = 0 (BINARY)"
        );
        for k in &keys {
            assert!(m.get(k) >= 0, "mode={mode} key {k:02x?} missing");
        }
        while !keys.is_empty() {
            let k = keys.remove(rng.below(keys.len()));
            assert_eq!(m.del(&k), 1);
        }
        m.free();
    }
}

#[test]
fn row38_mode_large_is_string() {
    let (p, _g) = libs();
    for &mode in &[2i32, 3, 1000, i32::MAX] {
        let mut cfg = cfg_string();
        cfg.put_mode = mode;
        cfg.del_mode = mode;
        let mut rng = Rng::new(0xC0FFEE ^ 38 ^ (mode as i64 as u64));
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg);
        let keys = keyset(&mut rng, 60);
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
        }
        assert_eq!(
            m.snap_c().table.as_ref().unwrap().arena_mode,
            SH_DEFAULT as u8,
            "mode={mode} must give string.mode = SH_DEFAULT"
        );
        for k in &keys {
            assert!(m.get(k) >= 0, "mode={mode} key {k:02x?} missing");
        }
        // tail-only deletes (see row36 comment)
        let mut live = keys.clone();
        while let Some(k) = live.pop() {
            assert_eq!(m.del(&k), 1);
        }
        m.free();
    }
}

#[test]
fn row45_string_sh_default_delete() {
    let (p, _g) = libs();
    for &n in &[1usize, 2, 8, 40, 150] {
        let mut rng = Rng::new(0xC0FFEE ^ 45 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg_string());
        let keys = keyset(&mut rng, n);
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
        }
        // absent-key deletes first (must be a NOP returning 0)
        for _ in 0..5 {
            let k = rng.ascii(31);
            if !keys.contains(&k) {
                assert_eq!(m.del(&k), 0, "absent-key delete must return 0");
            }
        }
        let mut live = keys.clone();
        while !live.is_empty() {
            let k = live.remove(rng.below(live.len()));
            assert_eq!(m.del(&k), 1);
            assert_eq!(m.get(&k), -1);
            for r in &live {
                assert!(m.get(r) >= 0, "{r:02x?} lost after delete");
            }
        }
        m.free();
    }
}

#[test]
fn row46_string_sh_strdup_delete_frees_dup() {
    let (p, _g) = libs();
    for &n in &[1usize, 2, 8, 40, 150] {
        let mut rng = Rng::new(0xC0FFEE ^ 46 ^ n as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::shmode(p, cfg_string(), SH_STRDUP);
        let keys = keyset(&mut rng, n);
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
        }
        let mut live = keys.clone();
        while !live.is_empty() {
            let k = live.remove(rng.below(live.len()));
            assert_eq!(m.del(&k), 1);
            assert_eq!(m.get(&k), -1);
            for r in &live {
                assert!(m.get(r) >= 0);
            }
        }
        // re-insert after all the dups were freed
        for k in &keys {
            m.put_string(k, &rng.bytes(8));
        }
        m.free();
    }
}

#[test]
fn row47_string_sh_arena_delete_and_shrink() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 47);
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::shmode(p, cfg_string(), SH_ARENA);
    let keys = keyset(&mut rng, 200);
    for k in &keys {
        m.put_string(k, &rng.bytes(8));
    }
    let start = m.snap_c().table.as_ref().unwrap().slot_count;
    let mut live = keys.clone();
    let mut shrinks = 0;
    let mut prev = start;
    while !live.is_empty() {
        let k = live.remove(rng.below(live.len()));
        assert_eq!(m.del(&k), 1);
        let now = m.snap_c().table.as_ref().unwrap().slot_count;
        if now < prev {
            shrinks += 1;
            prev = now;
        }
    }
    assert!(shrinks >= 2, "expected shrinks, saw {shrinks}");
    m.free();
}

#[test]
fn row50_51_hmfree_strdup_and_arena() {
    let (p, _g) = libs();
    for &shmode in &[SH_STRDUP, SH_ARENA] {
        for &n in &[0usize, 1, 8, 100, 400] {
            let mut rng = Rng::new(0xC0FFEE ^ 50 ^ (shmode as u64) << 16 ^ n as u64);
            reset_seed(p, DEFAULT_SEED);
            let mut m = Maps::shmode(p, cfg_string(), shmode);
            let keys = keyset(&mut rng, n);
            for k in &keys {
                m.put_string(k, &rng.bytes(8));
            }
            m.compare("before hmfree");
            m.free(); // must free every dup / reset the whole arena
        }
    }
}
