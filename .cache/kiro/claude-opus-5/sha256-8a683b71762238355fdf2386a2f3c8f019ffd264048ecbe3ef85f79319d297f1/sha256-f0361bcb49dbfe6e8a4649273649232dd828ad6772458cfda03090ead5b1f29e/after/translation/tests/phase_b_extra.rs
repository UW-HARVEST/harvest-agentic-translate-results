//! Phase B (extension) — CONFIGS.md rows 64-70: composed entry-point
//! sequences and input shapes that the earlier rows do not reach.

mod common;
use common::*;
use std::ffi::c_void;

fn i32k(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}

fn keyset(rng: &mut Rng, n: usize) -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    while v.len() < n {
        let len = 1 + rng.below(24);
        let k = rng.ascii(len);
        if !v.contains(&k) {
            v.push(k);
        }
    }
    v
}

/// Row 64: a map first materialised by `hmget_key(NULL, …)` (length 1, no hash
/// table) and only then populated with `hmput_key`.
#[test]
fn row64_get_then_put_on_same_map() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 64);
    for mode in [HM_BINARY, HM_STRING] {
        reset_seed(p, DEFAULT_SEED);
        let cfg = if mode == HM_STRING {
            cfg_string()
        } else {
            cfg_struct()
        };
        let mut m = Maps::empty(p, cfg);
        // first op is a lookup on a NULL map: allocates the array, no table
        if mode == HM_STRING {
            assert_eq!(m.get(b"nothing-here"), -1);
        } else {
            assert_eq!(m.get(&i32k(1)), -1);
        }
        assert!(!m.snap_c().has_table, "no hash table yet");
        assert_eq!(m.snap_c().length, 1);
        // now populate it: hmput_key must take the "a != NULL, table == NULL" path
        if mode == HM_STRING {
            let keys = keyset(&mut rng, 60);
            for k in &keys {
                m.put_string(k, &rng.bytes(8));
            }
            for k in &keys {
                assert!(m.get(k) >= 0);
            }
            let mut live = keys.clone();
            while !live.is_empty() {
                let k = live.remove(rng.below(live.len()));
                assert_eq!(m.del(&k), 1);
            }
        } else {
            let keys: Vec<i32> = (0..60).map(|i| i * 9 - 100).collect();
            for &k in &keys {
                m.put_binary(&i32k(k), &rng.bytes(12));
            }
            for &k in &keys {
                assert!(m.get(&i32k(k)) >= 0);
            }
            let mut live = keys.clone();
            while !live.is_empty() {
                let k = live.remove(rng.below(live.len()));
                assert_eq!(m.del(&i32k(k)), 1);
            }
        }
        m.free();
    }
}

/// Row 65: `hmdel_key` on a NULL map followed by real use, and `hmdel_key`
/// as the very first operation on a `hmget_key`-materialised map.
#[test]
fn row65_del_first_then_put() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 65);
    reset_seed(p, DEFAULT_SEED);
    let mut m = Maps::empty(p, cfg_struct());
    // delete on a NULL map returns NULL and leaves the driver's pointers NULL
    assert_eq!(m.del(&i32k(5)), 0);
    assert!(m.snap_c().null);
    // materialise via get, then delete (table == NULL branch), then put
    assert_eq!(m.get(&i32k(5)), -1);
    assert_eq!(m.del(&i32k(5)), 0);
    for k in 0..40i32 {
        m.put_binary(&i32k(k), &rng.bytes(12));
    }
    for k in 0..40i32 {
        assert!(m.get(&i32k(k)) >= 0);
    }
    m.free();
}

/// Row 66: `hmput_default` interleaved with a fully populated map (the default
/// element must be preserved across every growth and shrink).
#[test]
fn row66_default_element_survives_growth_and_shrink() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 66);
    reset_seed(p, DEFAULT_SEED);
    let elemsize = 16usize;
    unsafe {
        let mut ch = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
        let mut rh = (p.rs.hmput_default)(std::ptr::null_mut(), elemsize);
        // stamp a recognisable default value
        for b in 0..elemsize {
            *(ch as *mut u8).sub(elemsize).add(b) = (0xE0 ^ b) as u8;
            *(rh as *mut u8).sub(elemsize).add(b) = (0xE0 ^ b) as u8;
        }
        let want: Vec<u8> = (0..elemsize).map(|b| (0xE0 ^ b) as u8).collect();

        let keys: Vec<i32> = (0..300).map(|i| i * 5 + 1).collect();
        for &k in &keys {
            let mut key = i32k(k);
            let v = rng.bytes(12);
            ch = (p.c.hmput_key)(
                ch,
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                4,
                HM_BINARY,
            );
            rh = (p.rs.hmput_key)(
                rh,
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                4,
                HM_BINARY,
            );
            let ct = snap_hash(ch, elemsize, ElemFmt::Raw).temp;
            let rt = snap_hash(rh, elemsize, ElemFmt::Raw).temp;
            assert_eq!(ct, rt);
            for h in [ch, rh] {
                let e = (h as *mut u8).add(elemsize * ct as usize);
                std::ptr::copy_nonoverlapping(v.as_ptr(), e.add(4), 12);
                std::ptr::copy_nonoverlapping(key.as_ptr(), e, 4);
            }
            let cs = snap_hash(ch, elemsize, ElemFmt::Raw);
            eq_snap("default+put", &cs, &snap_hash(rh, elemsize, ElemFmt::Raw));
            assert_eq!(cs.elem0, want, "default element corrupted by growth");
            // hmput_default must be a NOP now
            let c2 = (p.c.hmput_default)(ch, elemsize);
            let r2 = (p.rs.hmput_default)(rh, elemsize);
            assert_eq!(c2, ch);
            assert_eq!(r2, rh);
        }
        // shrink the table all the way back down
        for &k in keys.iter().rev() {
            let mut key = i32k(k);
            ch = (p.c.hmdel_key)(
                ch,
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                4,
                0,
                HM_BINARY,
            );
            rh = (p.rs.hmdel_key)(
                rh,
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                4,
                0,
                HM_BINARY,
            );
            let cs = snap_hash(ch, elemsize, ElemFmt::Raw);
            eq_snap("default+del", &cs, &snap_hash(rh, elemsize, ElemFmt::Raw));
            assert_eq!(cs.elem0, want, "default element corrupted by shrink");
        }
        (p.c.hmfree_func)((ch as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (p.rs.hmfree_func)((rh as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

/// Row 67: unusual element sizes, including one where the element is exactly
/// the key pointer (`elemsize == 8`, zero-length value) and large elements.
#[test]
fn row67_extreme_elemsizes() {
    let (p, _g) = libs();
    // STRING map whose element is only the key pointer
    for &elemsize in &[8usize, 40, 64, 128] {
        let cfg = MapCfg {
            elemsize,
            keysize: 8,
            keyoffset: 0,
            put_mode: HM_STRING,
            del_mode: HM_STRING,
            fmt: ElemFmt::PtrKey,
            value_off: 8,
            value_len: elemsize - 8,
        };
        let mut rng = Rng::new(0xC0FFEE ^ 67 ^ elemsize as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::shmode(p, cfg, SH_STRDUP);
        let keys = keyset(&mut rng, 80);
        for k in &keys {
            m.put_string(k, &rng.bytes(elemsize - 8));
        }
        for k in &keys {
            assert!(m.get(k) >= 0, "elemsize={elemsize}");
        }
        let mut live = keys.clone();
        while !live.is_empty() {
            let k = live.remove(rng.below(live.len()));
            assert_eq!(m.del(&k), 1);
        }
        m.free();
    }
    // BINARY map with a large element and a wide key
    for &(elemsize, keysize) in &[(64usize, 32usize), (128, 64), (40, 24)] {
        let cfg = MapCfg {
            elemsize,
            keysize,
            keyoffset: 0,
            put_mode: HM_BINARY,
            del_mode: HM_BINARY,
            fmt: ElemFmt::Raw,
            value_off: keysize,
            value_len: elemsize - keysize,
        };
        let mut rng = Rng::new(0xC0FFEE ^ 67 ^ (elemsize as u64) << 8 ^ keysize as u64);
        reset_seed(p, DEFAULT_SEED);
        let mut m = Maps::empty(p, cfg);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for _ in 0..80 {
            let k = rng.bytes(keysize);
            if keys.contains(&k) {
                continue;
            }
            keys.push(k.clone());
            m.put_binary(&k, &rng.bytes(elemsize - keysize));
        }
        for k in &keys {
            assert!(m.get(k) >= 0, "es={elemsize} ks={keysize}");
        }
        while !keys.is_empty() {
            let k = keys.remove(rng.below(keys.len()));
            assert_eq!(m.del(&k), 1);
        }
        m.free();
    }
}

/// Row 68: STRING-mode delete with a non-zero `keyoffset` (the `*(char**)(elem
/// + keyoffset)` re-find branch).
#[test]
fn row68_string_delete_with_keyoffset() {
    let (p, _g) = libs();
    // element layout: 8 bytes of value, then the char* key at offset 8.
    // hmput_key always stores the key at offset 0, so the caller mirrors it to
    // `keyoffset` exactly like a real `{int pad[2]; char *key;}` struct field.
    let _cfg = MapCfg {
        elemsize: 24,
        keysize: 8,
        keyoffset: 8,
        put_mode: HM_STRING,
        del_mode: HM_STRING,
        fmt: ElemFmt::Raw,
        value_off: 16,
        value_len: 8,
    };
    let mut rng = Rng::new(0xC0FFEE ^ 68);
    reset_seed(p, DEFAULT_SEED);
    let keys = keyset(&mut rng, 60);
    let owned: Vec<CKey> = keys.iter().map(|k| CKey::new(k)).collect();
    unsafe {
        let mut ch: *mut c_void = std::ptr::null_mut();
        let mut rh: *mut c_void = std::ptr::null_mut();
        for (i, k) in owned.iter().enumerate() {
            ch = (p.c.hmput_key)(ch, 24, k.ptr() as *mut c_void, 8, HM_STRING);
            rh = (p.rs.hmput_key)(rh, 24, k.ptr() as *mut c_void, 8, HM_STRING);
            let ct = snap_hash(ch, 24, ElemFmt::Raw).temp;
            let rt = snap_hash(rh, 24, ElemFmt::Raw).temp;
            assert_eq!(ct, rt, "put #{i}");
            let v = rng.bytes(8);
            for h in [ch, rh] {
                let e = (h as *mut u8).add(24 * ct as usize);
                // mirror the stored pointer to keyoffset, then write the value
                let kp = *(e as *const usize);
                *(e.add(8) as *mut usize) = kp;
                std::ptr::copy_nonoverlapping(v.as_ptr(), e.add(16), 8);
            }
        }
        // now delete with keyoffset = 8: the post-memmove re-find dereferences
        // `*(char **)(elem + 8)`
        let mut order: Vec<usize> = (0..owned.len()).collect();
        while !order.is_empty() {
            let j = order.remove(rng.below(order.len()));
            let k = &owned[j];
            ch = (p.c.hmdel_key)(ch, 24, k.ptr() as *mut c_void, 8, 8, HM_STRING);
            rh = (p.rs.hmdel_key)(rh, 24, k.ptr() as *mut c_void, 8, 8, HM_STRING);
            let cs = snap_hash(ch, 24, ElemFmt::Raw);
            let rs = snap_hash(rh, 24, ElemFmt::Raw);
            assert_eq!(cs.length, rs.length, "length after keyoffset delete");
            assert_eq!(cs.temp, rs.temp, "temp after keyoffset delete");
            assert_eq!(
                cs.table.as_ref().unwrap().used_count,
                rs.table.as_ref().unwrap().used_count
            );
            assert_eq!(
                cs.table.as_ref().unwrap().buckets,
                rs.table.as_ref().unwrap().buckets
            );
            assert_eq!(cs.temp, 1, "delete must report success");
        }
        (p.c.hmfree_func)((ch as *mut u8).sub(24) as *mut c_void, 24);
        (p.rs.hmfree_func)((rh as *mut u8).sub(24) as *mut c_void, 24);
    }
}

/// Row 69: `shmode_func` map + `hmput_default` (which must be a NOP because
/// `shmode_func` already set `length = 1`), then normal use.
#[test]
fn row69_shmode_then_hmput_default() {
    let (p, _g) = libs();
    let mut rng = Rng::new(0xC0FFEE ^ 69);
    for &shmode in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        reset_seed(p, DEFAULT_SEED);
        let uses_ptr = shmode != SH_NONE;
        let cfg = if uses_ptr {
            cfg_string()
        } else {
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
        };
        let mut m = Maps::shmode(p, cfg, shmode);
        unsafe {
            let c2 = (p.c.hmput_default)(m.ch, 16);
            let r2 = (p.rs.hmput_default)(m.rh, 16);
            assert_eq!(c2, m.ch, "shmode={shmode}: hmput_default must be a NOP");
            assert_eq!(r2, m.rh);
        }
        m.compare("shmode + hmput_default");
        if uses_ptr {
            let keys = keyset(&mut rng, 50);
            for k in &keys {
                m.put_string(k, &rng.bytes(8));
            }
            for k in &keys {
                assert!(m.get(k) >= 0);
            }
        } else {
            let mut keys: Vec<Vec<u8>> = Vec::new();
            for _ in 0..50 {
                let k = rng.bytes(8);
                if keys.contains(&k) {
                    continue;
                }
                keys.push(k.clone());
                m.put_binary(&k, &rng.bytes(8));
            }
            for k in &keys {
                assert!(m.get(k) >= 0);
            }
        }
        m.free();
    }
}

/// Row 70: long randomized op streams over every `string.mode`, with a random
/// global seed per stream, comparing full state after every operation.
#[test]
fn row70_long_random_streams_all_string_modes() {
    let (p, _g) = libs();
    let mut outer = Rng::new(0xC0FFEE ^ 70);
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for stream in 0..4u64 {
            let gseed = outer.next_u64() as usize;
            let mut rng = Rng::new(0xC0FFEE ^ 70 ^ (shmode as u64) << 32 ^ stream);
            reset_seed(p, gseed);
            let mut m = Maps::shmode(p, cfg_string(), shmode);
            // small key space => heavy collisions, tombstones, rebuilds
            let space: Vec<Vec<u8>> = (0..50)
                .map(|i| format!("k{:02}", i).into_bytes())
                .collect();
            let mut live: Vec<Vec<u8>> = Vec::new();
            for _ in 0..800 {
                let k = space[rng.below(space.len())].clone();
                match rng.below(10) {
                    0..=4 => {
                        m.put_string(&k, &rng.bytes(8));
                        if !live.contains(&k) {
                            live.push(k);
                        }
                    }
                    5..=6 => {
                        let want = if live.contains(&k) { 1 } else { 0 };
                        assert_eq!(m.del(&k), want, "shmode={shmode} del");
                        live.retain(|x| x != &k);
                    }
                    7..=8 => {
                        assert_eq!(m.get(&k) >= 0, live.contains(&k));
                    }
                    _ => {
                        assert_eq!(m.get_ts(&k) >= 0, live.contains(&k));
                    }
                }
            }
            m.free();
        }
    }
}
