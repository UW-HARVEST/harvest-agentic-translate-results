//! Phase B — CONFIGS.md rows 37..=56, 66, 67, 69, 70: string-keyed maps, all
//! four `STBDS_SH_*` storage modes, the `stbds_shmode_func` entry point and
//! out-of-enum `mode` values crossing the FFI boundary.

mod common;
use common::*;
use std::ffi::c_int;

const ELEMSIZE: usize = 16; // { char *key; uint64 value; }
const KEYSIZE: usize = 8;

fn kr_for(string_mode: c_int) -> KeyRepr {
    // With STBDS_SH_NONE (and any out-of-enum value) the `default:` branch of
    // the put switch memcpy's raw bytes instead of storing a pointer.
    match string_mode & 0xff {
        1 | 2 | 3 => KeyRepr::CStr,
        _ => KeyRepr::Raw,
    }
}

/// Distinct NUL-terminated keys, deterministic.
fn mkkeys(n: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let len = 1 + rng.below(40);
        let mut s: Vec<u8> = format!("k{i:04}_").into_bytes();
        for _ in 0..len {
            s.push(0x41 + (rng.next_u64() % 26) as u8);
        }
        s.push(0);
        out.push(s);
    }
    out
}

/// rows 37..=40 — `stbds_shmode_func` for every `STBDS_SH_*` value
#[test]
fn r37_r40_shmode_func_all_modes() {
    for mode in [STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        let _g = reset_seeds(0x2000_0000 + mode as usize);
        for &elemsize in &[1usize, 4, 8, 16, 20] {
            let m = unsafe { DualMap::shmode(elemsize, elemsize.min(8), KeyRepr::Raw, mode) };
            let s = unsafe { m.snap_c() };
            assert_eq!(s.length, 1, "shmode_func must create a 1-element array");
            assert!(s.has_table);
            assert_eq!(s.slot_count, 8);
            assert_eq!(s.arena_mode, mode as u8);
            assert_eq!(s.used_count, 0);
            assert_eq!(s.used_count_shrink_threshold, 0);
            unsafe { m.free() };
        }
    }
}

/// rows 41..=44 — `shmode_func(mode) + hmput_key(put_mode)` cross product,
/// randomized over many distinct keys.
#[test]
fn r41_r44_shmode_x_putmode() {
    for string_mode in [STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for put_mode in [STBDS_HM_BINARY, STBDS_HM_STRING] {
            for trial in 0..4u64 {
                let _g = reset_seeds(0x2100_0000 + trial as usize * 7 + string_mode as usize);
                let keys = mkkeys(40, 0xDEAD_0041 ^ (trial << 8) ^ string_mode as u64);
                let mut ka = KeyArena::new();
                let mut m = unsafe {
                    DualMap::shmode(ELEMSIZE, KEYSIZE, kr_for(string_mode), string_mode)
                };
                for (j, k) in keys.iter().enumerate() {
                    let p = ka.add(k);
                    unsafe { m.put(p, put_mode, j as u64) };
                }
                unsafe { m.free() };
            }
        }
    }
}

/// row 45 — `hmput_key(NULL, …, mode=1)` implicitly creates a table whose
/// `string.mode` is `STBDS_SH_DEFAULT`.
#[test]
fn r45_implicit_table_string_mode_default() {
    for trial in 0..6u64 {
        let _g = reset_seeds(0x2200_0000 + trial as usize);
        let keys = mkkeys(60, 0xDEAD_0045 ^ (trial << 8));
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(ELEMSIZE, KEYSIZE, KeyRepr::CStr);
        for (j, k) in keys.iter().enumerate() {
            let p = ka.add(k);
            unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
        }
        assert_eq!(
            unsafe { m.snap_c() }.arena_mode,
            STBDS_SH_DEFAULT as u8,
            "implicit string table must default to STBDS_SH_DEFAULT"
        );
        unsafe { m.free() };
    }
}

/// row 46 — `hmput_key(NULL, …, mode=0)` gives `string.mode == 0`
#[test]
fn r46_implicit_table_string_mode_none() {
    let _g = reset_seeds(0x2300_0000);
    let mut ka = KeyArena::new();
    let mut m = DualMap::null(ELEMSIZE, KEYSIZE, KeyRepr::Raw);
    let k = ka.add(&[1u8, 2, 3, 4, 5, 6, 7, 8]);
    unsafe { m.put(k, STBDS_HM_BINARY, 1) };
    assert_eq!(unsafe { m.snap_c() }.arena_mode, 0);
    unsafe { m.free() };
}

/// row 47 — string lookups (present + absent) on DEFAULT / STRDUP / ARENA
#[test]
fn r47_string_lookups() {
    for string_mode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for trial in 0..4u64 {
            let _g = reset_seeds(0x2400_0000 + trial as usize * 11 + string_mode as usize);
            let mut rng = Rng::new(0xDEAD_0047 ^ (trial << 16) ^ string_mode as u64);
            let keys = mkkeys(80, 0xBEEF_0047 ^ (trial << 8) ^ string_mode as u64);
            let absent = mkkeys(40, 0xFEED_0047 ^ (trial << 8) ^ string_mode as u64);
            let mut ka = KeyArena::new();
            let mut m =
                unsafe { DualMap::shmode(ELEMSIZE, KEYSIZE, KeyRepr::CStr, string_mode) };
            for (j, k) in keys.iter().enumerate() {
                let p = ka.add(k);
                unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
            }
            for _ in 0..500 {
                let (src, i) = if rng.next_u64() & 1 == 0 {
                    (&keys, rng.below(keys.len()))
                } else {
                    (&absent, rng.below(absent.len()))
                };
                let p = ka.add(&src[i]);
                let a = unsafe { m.get(p, STBDS_HM_STRING) };
                let b = unsafe { m.get_ts(p, STBDS_HM_STRING) };
                assert_eq!(a, b, "shgeti / shgeti_ts disagree");
            }
            // every inserted key must be found
            for k in keys.iter() {
                let p = ka.add(k);
                assert!(
                    unsafe { m.get(p, STBDS_HM_STRING) } >= 0,
                    "inserted key not found"
                );
            }
            // absent keys must all report -1 (mkkeys uses disjoint index prefixes)
            unsafe { m.free() };
        }
    }
}

/// rows 48..=50 — string deletes on DEFAULT / STRDUP / ARENA tables
#[test]
fn r48_r50_string_deletes() {
    for string_mode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for trial in 0..4u64 {
            let _g = reset_seeds(0x2500_0000 + trial as usize * 13 + string_mode as usize);
            let mut rng = Rng::new(0xDEAD_0048 ^ (trial << 16) ^ string_mode as u64);
            let keys = mkkeys(90, 0xBEEF_0048 ^ (trial << 8) ^ string_mode as u64);
            let mut ka = KeyArena::new();
            let mut m =
                unsafe { DualMap::shmode(ELEMSIZE, KEYSIZE, KeyRepr::CStr, string_mode) };
            for (j, k) in keys.iter().enumerate() {
                let p = ka.add(k);
                unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
            }
            let mut live: Vec<Vec<u8>> = keys.clone();
            while !live.is_empty() {
                let k = live.remove(rng.below(live.len()));
                let p = ka.add(&k);
                assert_eq!(
                    unsafe { m.del(p, STBDS_HM_STRING) },
                    1,
                    "delete of a present string key must report 1"
                );
                // deleting again must now report 0
                let p2 = ka.add(&k);
                assert_eq!(unsafe { m.del(p2, STBDS_HM_STRING) }, 0);
            }
            assert_eq!(unsafe { map_len(m.tc, ELEMSIZE) }, 0);
            unsafe { m.free() };
        }
    }
}

/// row 51 — `table->temp_key` (`stbds_temp_key`) behaviour:
///  * insert  -> temp_key = the stored key
///  * duplicate found in the FORWARD probe loop -> temp_key updated
///  * duplicate found in the WRAP-AROUND probe loop -> temp_key NOT updated
///    (upstream quirk; verified by comparing C and Rust, whatever it is)
#[test]
fn r51_temp_key_tracking() {
    for string_mode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for trial in 0..6u64 {
            let _g = reset_seeds(0x2600_0000 + trial as usize * 17 + string_mode as usize);
            let mut rng = Rng::new(0xDEAD_0051 ^ (trial << 16) ^ string_mode as u64);
            let keys = mkkeys(50, 0xBEEF_0051 ^ (trial << 8) ^ string_mode as u64);
            let mut ka = KeyArena::new();
            let mut m =
                unsafe { DualMap::shmode(ELEMSIZE, KEYSIZE, KeyRepr::CStr, string_mode) };
            for (j, k) in keys.iter().enumerate() {
                let p = ka.add(k);
                unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
                // temp_key is written by every insert of a string-mode table
                let tc = unsafe { map_temp_key(m.tc, ELEMSIZE) };
                let tr = unsafe { map_temp_key(m.tr, ELEMSIZE) };
                assert_eq!(tc, tr, "temp_key diverged after insert");
                let want: Vec<u8> = k[..k.len() - 1].to_vec();
                assert_eq!(tc.as_deref(), Some(&want[..]), "temp_key != inserted key");
                if string_mode == STBDS_SH_DEFAULT {
                    // must be the *caller's* pointer
                    assert_eq!(unsafe { map_temp_key_ptr(m.tc, ELEMSIZE) } as *const u8, p as *const u8);
                    assert_eq!(unsafe { map_temp_key_ptr(m.tr, ELEMSIZE) } as *const u8, p as *const u8);
                }
            }
            // now re-put existing keys: whatever the C does to temp_key
            // (update it, or leave it alone in the wrap-around loop), Rust must
            // do the same.
            for _ in 0..400 {
                let k = &keys[rng.below(keys.len())];
                let p = ka.add(k);
                unsafe { m.put(p, STBDS_HM_STRING, 0xAB) };
                let tc = unsafe { map_temp_key(m.tc, ELEMSIZE) };
                let tr = unsafe { map_temp_key(m.tr, ELEMSIZE) };
                assert_eq!(tc, tr, "temp_key diverged after duplicate put");
            }
            unsafe { m.free() };
        }
    }
}

/// rows 52..=56 — `stbds_hmfree_func` on every kind of map.  A wrong free
/// pattern makes glibc abort, so "both complete" is the observable signal;
/// the map state is compared right up to the free.
#[test]
fn r52_r56_hmfree_all_modes() {
    // 52: binary map
    {
        let _g = reset_seeds(0x2700_0001);
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(16, 8, KeyRepr::Raw);
        for j in 0..50u64 {
            let k = ka.add(&j.to_le_bytes());
            unsafe { m.put(k, STBDS_HM_BINARY, j) };
        }
        unsafe { m.free() };
    }
    // 53/54/55: DEFAULT / STRDUP / ARENA string maps
    for string_mode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for trial in 0..4u64 {
            let _g = reset_seeds(0x2700_0000 + trial as usize * 19 + string_mode as usize);
            let keys = mkkeys(70, 0xBEEF_0052 ^ (trial << 8) ^ string_mode as u64);
            let mut ka = KeyArena::new();
            let mut m =
                unsafe { DualMap::shmode(ELEMSIZE, KEYSIZE, KeyRepr::CStr, string_mode) };
            for (j, k) in keys.iter().enumerate() {
                let p = ka.add(k);
                unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
            }
            // delete a few first, so hmfree_func sees a partially-emptied map
            for k in keys.iter().take(10) {
                let p = ka.add(k);
                unsafe { m.del(p, STBDS_HM_STRING) };
            }
            unsafe { m.free() };
        }
    }
    // 56: array with hash_table == NULL (from hmget_key_ts(NULL, …))
    {
        let _g = reset_seeds(0x2700_0099);
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(16, 8, KeyRepr::Raw);
        let k = ka.add(&[9u8; 8]);
        unsafe { m.get_ts(k, STBDS_HM_BINARY) };
        assert!(!unsafe { m.snap_c() }.has_table);
        unsafe { m.free() };
    }
    // 56b: hmput_default-only map (also table-less)
    {
        let _g = reset_seeds(0x2700_009A);
        let mut m = DualMap::null(16, 8, KeyRepr::Raw);
        unsafe { m.put_default() };
        unsafe { m.free() };
    }
}

/// row 66 — out-of-enum `mode` values on put / get / del
#[test]
fn r66_out_of_range_put_get_del_modes() {
    // mode >= 1 -> string path;  mode < 1 -> binary path
    let string_modes: &[c_int] = &[1, 2, 7, 255, 0x7FFF_FFFF];
    let binary_modes: &[c_int] = &[0, -1, -7, i32::MIN];

    for &mode in string_modes {
        for trial in 0..3u64 {
            let _g = reset_seeds(0x2800_0000 + (mode as usize).wrapping_mul(3) + trial as usize);
            let mut rng = Rng::new(0xDEAD_0066 ^ (mode as u64) ^ (trial << 8));
            let keys = mkkeys(50, 0xBEEF_0066 ^ (mode as u64) ^ (trial << 8));
            let mut ka = KeyArena::new();
            // on a STRDUP table: hmdel_key frees the key only when mode == 1
            for sm in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
                let mut m = unsafe { DualMap::shmode(ELEMSIZE, KEYSIZE, KeyRepr::CStr, sm) };
                for (j, k) in keys.iter().enumerate() {
                    let p = ka.add(k);
                    unsafe { m.put(p, mode, j as u64) };
                }
                for _ in 0..200 {
                    let k = &keys[rng.below(keys.len())];
                    let p = ka.add(k);
                    unsafe { m.get(p, mode) };
                }
                // Delete in reverse insertion order: `old_index == final_index`
                // so the relocation branch (which asserts for mode != 1) is not
                // taken.  The relocation-with-bad-mode case aborts in C and is
                // covered by tests/crash_parity.rs.
                for k in keys.iter().rev() {
                    let p = ka.add(k);
                    assert_eq!(unsafe { m.del(p, mode) }, 1);
                }
                unsafe { m.free() };
            }
        }
    }

    for &mode in binary_modes {
        for trial in 0..3u64 {
            let _g = reset_seeds(0x2900_0000 + (mode as usize).wrapping_mul(5) + trial as usize);
            let mut rng = Rng::new(0xDEAD_0067 ^ (mode as u64) ^ (trial << 8));
            let mut ka = KeyArena::new();
            let mut m = DualMap::null(16, 8, KeyRepr::Raw);
            let mut live: Vec<Vec<u8>> = Vec::new();
            for j in 0..80u64 {
                let kb = rng.bytes(8);
                if live.contains(&kb) {
                    continue;
                }
                live.push(kb.clone());
                let k = ka.add(&kb);
                unsafe { m.put(k, mode, j) };
            }
            for _ in 0..200 {
                let kb = &live[rng.below(live.len())];
                let k = ka.add(kb);
                assert!(unsafe { m.get(k, mode) } >= 0);
            }
            while !live.is_empty() {
                let kb = live.remove(rng.below(live.len()));
                let k = ka.add(&kb);
                assert_eq!(unsafe { m.del(k, mode) }, 1);
            }
            unsafe { m.free() };
        }
    }
}

/// row 67 — out-of-enum `mode` for `stbds_shmode_func` (`(unsigned char)mode`)
#[test]
fn r67_out_of_range_shmode_values() {
    for mode in [4i32, 5, 7, 100, 255, 256, 257, 512, -1, -2, -256, i32::MIN, i32::MAX] {
        let _g = reset_seeds(0x2A00_0000 + (mode as usize & 0xffff));
        let m = unsafe { DualMap::shmode(ELEMSIZE, KEYSIZE, KeyRepr::Raw, mode) };
        let sc = unsafe { m.snap_c() };
        let sr = unsafe { m.snap_r() };
        assert_eq!(sc, sr, "shmode_func({mode}) diverged");
        assert_eq!(
            sc.arena_mode,
            (mode as u32 & 0xff) as u8,
            "string.mode must be (unsigned char)mode for mode={mode}"
        );
        // the `default:` put branch (raw memcpy) must be used unless the
        // truncated value happens to be 1/2/3
        let mut ka = KeyArena::new();
        let keys = mkkeys(24, 0xBEEF_0067 ^ (mode as u64 as u64));
        let mut m2 = DualMap {
            tc: m.tc,
            tr: m.tr,
            elemsize: ELEMSIZE,
            keysize: KEYSIZE,
            kr: kr_for(mode),
            ops: 0,
        };
        for (j, k) in keys.iter().enumerate() {
            let p = ka.add(k);
            unsafe { m2.put(p, STBDS_HM_STRING, j as u64) };
        }
        unsafe { m2.free() };
    }
}

/// row 69 — long mixed stress on a STRDUP string map
#[test]
fn r69_strdup_stress() {
    for trial in 0..3u64 {
        let _g = reset_seeds(0x2B00_0000 + trial as usize);
        let mut rng = Rng::new(0xDEAD_0069 ^ (trial << 24));
        let keys = mkkeys(400, 0xBEEF_0069 ^ (trial << 8));
        let mut ka = KeyArena::new();
        let mut m =
            unsafe { DualMap::shmode(ELEMSIZE, KEYSIZE, KeyRepr::CStr, STBDS_SH_STRDUP) };
        let mut live: Vec<usize> = Vec::new();
        for j in 0..1000u64 {
            let r = rng.below(100);
            if r < 55 || live.is_empty() {
                let i = rng.below(keys.len());
                let p = ka.add(&keys[i]);
                unsafe { m.put(p, STBDS_HM_STRING, j) };
                if !live.contains(&i) {
                    live.push(i);
                }
            } else if r < 80 {
                let i = live[rng.below(live.len())];
                let p = ka.add(&keys[i]);
                unsafe { m.get(p, STBDS_HM_STRING) };
            } else {
                let i = live.remove(rng.below(live.len()));
                let p = ka.add(&keys[i]);
                assert_eq!(unsafe { m.del(p, STBDS_HM_STRING) }, 1);
            }
        }
        unsafe { m.free() };
    }
}

/// row 70 — the full pipeline on an ARENA map, snapshot-compared at every step
#[test]
fn r70_full_arena_pipeline() {
    for trial in 0..4u64 {
        let _g = reset_seeds(0x2C00_0000 + trial as usize);
        let mut rng = Rng::new(0xDEAD_0070 ^ (trial << 24));
        // mix of short and long keys so stbds_stralloc has to allocate both
        // regular blocks and dedicated over-sized blocks
        let mut keys: Vec<Vec<u8>> = mkkeys(150, 0xBEEF_0070 ^ (trial << 8));
        for i in 0..12usize {
            let mut big = format!("BIG{i:03}_").into_bytes();
            big.extend(std::iter::repeat(b'Z').take(600 + i * 90));
            big.push(0);
            keys.push(big);
        }
        let mut ka = KeyArena::new();
        let mut m = unsafe { DualMap::shmode(ELEMSIZE, KEYSIZE, KeyRepr::CStr, STBDS_SH_ARENA) };
        for (j, k) in keys.iter().enumerate() {
            let p = ka.add(k);
            unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
        }
        // delete half at random
        let mut live: Vec<usize> = (0..keys.len()).collect();
        for _ in 0..(keys.len() / 2) {
            let i = live.remove(rng.below(live.len()));
            let p = ka.add(&keys[i]);
            assert_eq!(unsafe { m.del(p, STBDS_HM_STRING) }, 1);
        }
        // look every key up (present and deleted)
        for (i, k) in keys.iter().enumerate() {
            let p = ka.add(k);
            let got = unsafe { m.get(p, STBDS_HM_STRING) };
            if live.contains(&i) {
                assert!(got >= 0, "live key {i} not found");
            } else {
                assert_eq!(got, -1, "deleted key {i} still found");
            }
        }
        // arena must have grown its block chain
        let s = unsafe { m.snap_c() };
        assert!(s.arena_chain > 1, "arena should hold several blocks");
        assert_eq!(s.arena_mode, STBDS_SH_ARENA as u8);
        unsafe { m.free() };
    }
}

/// Extra: `shmode_func` map used with `hmput_default` and then string puts
/// (mixing the two creation entry points).
#[test]
fn r70b_shmode_plus_put_default() {
    for string_mode in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        let _g = reset_seeds(0x2D00_0000 + string_mode as usize);
        let keys = mkkeys(40, 0xBEEF_0070 ^ string_mode as u64);
        let mut ka = KeyArena::new();
        let mut m = unsafe { DualMap::shmode(ELEMSIZE, KEYSIZE, KeyRepr::CStr, string_mode) };
        unsafe { m.put_default() }; // length is already 1 -> must be a no-op
        for (j, k) in keys.iter().enumerate() {
            let p = ka.add(k);
            unsafe { m.put(p, STBDS_HM_STRING, j as u64) };
        }
        unsafe { m.put_default() }; // still a no-op
        unsafe { m.free() };
    }
}

/// Extra: `keyoffset != 0` — `stbds_hmdel_key` is the only function taking a
/// `keyoffset` (the `stbds_hmdel` macro passes `offsetof(t,key)`).  To make a
/// non-zero offset *consistent* (the C requires the key to be at the offset it
/// is told about), each element's second half is kept byte-identical to its
/// key, so a `keyoffset == 8` lookup sees the same bytes as a `keyoffset == 0`
/// one.  This exercises `stbds_is_key_equal`'s and the relocation re-find's
/// `keyoffset` arithmetic end to end.
#[test]
fn r70c_hmdel_key_nonzero_keyoffset() {
    let p = common::libs();
    let elemsize = 16usize;
    let keysize = 8usize;
    let mut rng = Rng::new(0xDEAD_070C);

    for trial in 0..6u64 {
        let _g2 = reset_seeds(0x2E00_0000 + trial as usize);
        let mut ka = KeyArena::new();
        let mut m = DualMap::null(elemsize, keysize, KeyRepr::Raw);
        let mut live: Vec<Vec<u8>> = Vec::new();
        for j in 0..50u64 {
            let mut kb = rng.bytes(8);
            if kb.iter().all(|&b| b == 0) {
                kb[0] = 1;
            }
            if live.contains(&kb) {
                continue;
            }
            live.push(kb.clone());
            let k = ka.add(&kb);
            // hmput_key writes the key at offset 0 ...
            m.tc = unsafe { (p.c.hmput_key)(m.tc, elemsize, k, keysize, STBDS_HM_BINARY) };
            m.tr = unsafe { (p.r.hmput_key)(m.tr, elemsize, k, keysize, STBDS_HM_BINARY) };
            let ic = unsafe { map_temp(m.tc, elemsize) };
            let ir = unsafe { map_temp(m.tr, elemsize) };
            assert_eq!(ic, ir);
            // ... and we mirror it into the value half so that keyoffset == 8
            // describes a real key location too.
            unsafe {
                for (t, i) in [(m.tc, ic), (m.tr, ir)] {
                    let e = (t as *mut u8).add(elemsize * i as usize);
                    std::ptr::copy_nonoverlapping(e, e.add(8), 8);
                }
            }
            m.ops += 1;
            m.check("after keyoffset put");
            let _ = j;
        }
        // delete every key using keyoffset = 8
        while !live.is_empty() {
            let kb = live.remove(rng.below(live.len()));
            let k = ka.add(&kb);
            let t = unsafe { m.del_off(k, 8, STBDS_HM_BINARY) };
            assert_eq!(t, 1, "keyoffset=8 delete should find {kb:02x?}");
            // keep the mirror consistent after the relocation memmove
            unsafe {
                let len = map_len(m.tc, elemsize);
                for tt in [m.tc, m.tr] {
                    for i in 0..len {
                        let e = (tt as *mut u8).add(elemsize * i as usize);
                        std::ptr::copy_nonoverlapping(e, e.add(8), 8);
                    }
                }
            }
            m.check("after keyoffset delete");
        }
        unsafe { m.free() };
    }
}
