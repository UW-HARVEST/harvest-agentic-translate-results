//! Phase B — differential tests for the hash-map entry points.
//!
//! CONFIGS.md rows 10-32, 39, 40.
//!
//! Every operation is driven through the exported `stbds_hm*` / `stbds_shmode_func`
//! functions exactly like the `stbds_hmput` / `stbds_hmgeti` / `stbds_hmdel`
//! macros do, and after each step the ENTIRE structure is compared: array header
//! (`length`, `capacity`, `temp`, table presence), the element payload bytes, and
//! every scalar + bucket of the `stbds_hash_index`.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEED: usize = 0x31415926;

fn reseed() {
    let (c, r) = apis();
    unsafe {
        (c.rand_seed)(SEED);
        (r.rand_seed)(SEED);
    }
}

unsafe fn cmp(ctx: &str, mc: &Map, mr: &Map) {
    assert_same(ctx, &mc.snap(), &mr.snap());
}

fn pair(
    elemsize: usize,
    keysize: usize,
    keyoffset: usize,
    pointer_keys: bool,
) -> (Map<'static>, Map<'static>) {
    let (c, r) = apis();
    (
        Map::new(c, elemsize, keysize, keyoffset, pointer_keys),
        Map::new(r, elemsize, keysize, keyoffset, pointer_keys),
    )
}

fn pair_shmode(
    elemsize: usize,
    keysize: usize,
    keyoffset: usize,
    mode: c_int,
) -> (Map<'static>, Map<'static>) {
    let (c, r) = apis();
    unsafe {
        (
            Map::new_shmode(c, elemsize, keysize, keyoffset, mode),
            Map::new_shmode(r, elemsize, keysize, keyoffset, mode),
        )
    }
}

// ---------------------------------------------------------------------------
// rows 10-12: mode = STBDS_HM_BINARY, 4-byte keys
// ---------------------------------------------------------------------------

#[test]
fn row10_binary_single_element() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        let mut k = 1u32.to_ne_bytes();
        let v = 0x1111_1111u32.to_ne_bytes();
        mc.put(&mut k, &v, 4, HM_BINARY);
        mr.put(&mut k, &v, 4, HM_BINARY);
        cmp("binary put 1", &mc, &mr);
        assert_same("binary get 1", &mc.get(&mut k, HM_BINARY), &mr.get(&mut k, HM_BINARY));
        cmp("binary get state", &mc, &mr);
        let mut miss = 2u32.to_ne_bytes();
        assert_same(
            "binary get miss",
            &mc.get(&mut miss, HM_BINARY),
            &mr.get(&mut miss, HM_BINARY),
        );
        cmp("binary miss state", &mc, &mr);
        mc.free();
        mr.free();
    }
}

#[test]
fn row11_binary_bucket_and_growth_boundaries() {
    for count in [0usize, 1, 2, 5, 6, 7, 8, 9, 16, 17, 32, 33] {
        let _g = seed_lock();
        reseed();
        let (mut mc, mut mr) = pair(8, 4, 0, false);
        unsafe {
            for i in 0..count {
                let mut k = (i as u32).to_ne_bytes();
                let v = (0xdead_0000u32 + i as u32).to_ne_bytes();
                mc.put(&mut k, &v, 4, HM_BINARY);
                mr.put(&mut k, &v, 4, HM_BINARY);
                cmp(&format!("count={count} put #{i}"), &mc, &mr);
            }
            for i in 0..count + 4 {
                let mut k = (i as u32).to_ne_bytes();
                assert_same(
                    &format!("count={count} get #{i}"),
                    &mc.get(&mut k, HM_BINARY),
                    &mr.get(&mut k, HM_BINARY),
                );
            }
            cmp(&format!("count={count} final"), &mc, &mr);
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row12_binary_500_random_keys_with_duplicates() {
    let _g = seed_lock();
    for trial in 0..5u64 {
        reseed();
        let mut rng = Rng::new(0xabcd_0000 + trial);
        let (mut mc, mut mr) = pair(8, 4, 0, false);
        unsafe {
            for i in 0..500 {
                // small key space -> many duplicate/overwrite paths
                let mut k = (rng.below(200) as u32).to_ne_bytes();
                let v = rng.next_u32().to_ne_bytes();
                mc.put(&mut k, &v, 4, HM_BINARY);
                mr.put(&mut k, &v, 4, HM_BINARY);
                cmp(&format!("trial={trial} put #{i}"), &mc, &mr);
            }
            for i in 0..250u32 {
                let mut k = i.to_ne_bytes();
                assert_same(
                    &format!("trial={trial} get #{i}"),
                    &mc.get(&mut k, HM_BINARY),
                    &mr.get(&mut k, HM_BINARY),
                );
            }
            cmp(&format!("trial={trial} final"), &mc, &mr);
            mc.free();
            mr.free();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 13-15: other key/element shapes
// ---------------------------------------------------------------------------

#[test]
fn row13_binary_8byte_keys() {
    let _g = seed_lock();
    reseed();
    let mut rng = Rng::new(0x1357);
    let (mut mc, mut mr) = pair(16, 8, 0, false);
    unsafe {
        for i in 0..300 {
            let mut k = rng.next_u64().to_ne_bytes();
            let v = rng.next_u64().to_ne_bytes();
            mc.put(&mut k, &v, 8, HM_BINARY);
            mr.put(&mut k, &v, 8, HM_BINARY);
            cmp(&format!("u64 put #{i}"), &mc, &mr);
            assert_same(
                &format!("u64 get #{i}"),
                &mc.get(&mut k, HM_BINARY),
                &mr.get(&mut k, HM_BINARY),
            );
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn row14_binary_16byte_keys() {
    let _g = seed_lock();
    reseed();
    let mut rng = Rng::new(0x2468);
    let (mut mc, mut mr) = pair(32, 16, 0, false);
    unsafe {
        for i in 0..300 {
            let mut k = rng.bytes(16);
            // reuse an earlier key every few iterations
            if i % 5 == 0 {
                k = vec![(i % 7) as u8; 16];
            }
            let v = rng.bytes(16);
            mc.put(&mut k, &v, 16, HM_BINARY);
            mr.put(&mut k, &v, 16, HM_BINARY);
            cmp(&format!("2word put #{i}"), &mc, &mr);
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn row15_binary_odd_keysizes() {
    for keysize in [1usize, 2, 3, 5, 6, 7, 9, 12, 17] {
        let _g = seed_lock();
        reseed();
        let mut rng = Rng::new(0x9000 + keysize as u64);
        let elemsize = keysize + 4;
        let (mut mc, mut mr) = pair(elemsize, keysize, 0, false);
        unsafe {
            for i in 0..120 {
                let mut k = rng.bytes(keysize);
                let v = rng.bytes(4);
                mc.put(&mut k, &v, keysize, HM_BINARY);
                mr.put(&mut k, &v, keysize, HM_BINARY);
                cmp(&format!("keysize={keysize} put #{i}"), &mc, &mr);
                assert_same(
                    &format!("keysize={keysize} get #{i}"),
                    &mc.get(&mut k, HM_BINARY),
                    &mr.get(&mut k, HM_BINARY),
                );
            }
            cmp(&format!("keysize={keysize} final"), &mc, &mr);
            mc.free();
            mr.free();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 16-19: hmget_key_ts, hmput_default, hash_table == NULL
// ---------------------------------------------------------------------------

#[test]
fn row16_hmget_key_ts_from_null_and_lookups() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        let mut k = 7u32.to_ne_bytes();
        // first call on a NULL map: allocates the default slot, *temp = -1
        assert_same(
            "ts on NULL",
            &mc.get_ts(&mut k, HM_BINARY),
            &mr.get_ts(&mut k, HM_BINARY),
        );
        cmp("ts on NULL state", &mc, &mr);
        // hash_table is still NULL here -> *temp = -1 again
        assert_same(
            "ts no table",
            &mc.get_ts(&mut k, HM_BINARY),
            &mr.get_ts(&mut k, HM_BINARY),
        );
        cmp("ts no table state", &mc, &mr);

        for i in 0..40u32 {
            let mut kk = i.to_ne_bytes();
            let v = (i * 3).to_ne_bytes();
            mc.put(&mut kk, &v, 4, HM_BINARY);
            mr.put(&mut kk, &v, 4, HM_BINARY);
        }
        cmp("ts after puts", &mc, &mr);
        for i in 0..60u32 {
            let mut kk = i.to_ne_bytes();
            assert_same(
                &format!("ts get #{i}"),
                &mc.get_ts(&mut kk, HM_BINARY),
                &mr.get_ts(&mut kk, HM_BINARY),
            );
            // _ts must leave header->temp alone
            cmp(&format!("ts get #{i} state"), &mc, &mr);
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn row17_hmget_key_with_null_hash_table() {
    let _g = seed_lock();
    reseed();
    for elemsize in [8usize, 16, 32] {
        let (mut mc, mut mr) = pair(elemsize, 4, 0, false);
        unsafe {
            mc.put_default();
            mr.put_default();
            cmp("default only", &mc, &mr);
            let mut k = 5u32.to_ne_bytes();
            assert_same(
                "get with NULL table",
                &mc.get(&mut k, HM_BINARY),
                &mr.get(&mut k, HM_BINARY),
            );
            cmp("get with NULL table state", &mc, &mr);
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row18_hmput_default_variants() {
    let _g = seed_lock();
    reseed();
    for elemsize in [8usize, 16, 32] {
        let (mut mc, mut mr) = pair(elemsize, 4, 0, false);
        unsafe {
            for i in 0..4 {
                mc.put_default();
                mr.put_default();
                cmp(&format!("put_default es={elemsize} #{i}"), &mc, &mr);
            }
            // force length back to 0: the second branch of the C condition
            (*mc.header()).length = 0;
            (*mr.header()).length = 0;
            mc.put_default();
            mr.put_default();
            cmp(&format!("put_default es={elemsize} len0"), &mc, &mr);
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row19_hmput_default_then_normal_puts() {
    let _g = seed_lock();
    reseed();
    let mut rng = Rng::new(0x4321);
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        mc.put_default();
        mr.put_default();
        let dv = 0xffff_ffffu32.to_ne_bytes();
        // hmdefault(t,v) writes into t[-1].value
        std::ptr::copy_nonoverlapping(dv.as_ptr(), (mc.a as *mut u8).sub(8).add(4), 4);
        std::ptr::copy_nonoverlapping(dv.as_ptr(), (mr.a as *mut u8).sub(8).add(4), 4);
        cmp("default value", &mc, &mr);
        for i in 0..80 {
            let mut k = (rng.below(60) as u32).to_ne_bytes();
            let v = rng.next_u32().to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
            cmp(&format!("default+put #{i}"), &mc, &mr);
        }
        for i in 0..80u32 {
            let mut k = i.to_ne_bytes();
            assert_same(
                &format!("default+get #{i}"),
                &mc.get(&mut k, HM_BINARY),
                &mr.get(&mut k, HM_BINARY),
            );
        }
        mc.free();
        mr.free();
    }
}

// ---------------------------------------------------------------------------
// rows 20-23: hmdel_key
// ---------------------------------------------------------------------------

#[test]
fn row20_binary_delete_churn() {
    let _g = seed_lock();
    for trial in 0..4u64 {
        reseed();
        let mut rng = Rng::new(0x5150_0000 + trial);
        let (mut mc, mut mr) = pair(8, 4, 0, false);
        unsafe {
            for i in 0..400 {
                let mut k = (rng.below(120) as u32).to_ne_bytes();
                match rng.below(3) {
                    0 => {
                        let v = rng.next_u32().to_ne_bytes();
                        mc.put(&mut k, &v, 4, HM_BINARY);
                        mr.put(&mut k, &v, 4, HM_BINARY);
                    }
                    1 => {
                        assert_same(
                            &format!("churn del t={trial} #{i}"),
                            &mc.del(&mut k, HM_BINARY),
                            &mr.del(&mut k, HM_BINARY),
                        );
                    }
                    _ => {
                        assert_same(
                            &format!("churn get t={trial} #{i}"),
                            &mc.get(&mut k, HM_BINARY),
                            &mr.get(&mut k, HM_BINARY),
                        );
                    }
                }
                cmp(&format!("churn t={trial} #{i}"), &mc, &mr);
            }
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row21_delete_until_shrink() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        for i in 0..200u32 {
            let mut k = i.to_ne_bytes();
            let v = (i ^ 0x55).to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
        }
        cmp("shrink: filled", &mc, &mr);
        for i in 0..200u32 {
            let mut k = i.to_ne_bytes();
            assert_same(
                &format!("shrink del #{i}"),
                &mc.del(&mut k, HM_BINARY),
                &mr.del(&mut k, HM_BINARY),
            );
            cmp(&format!("shrink del #{i} state"), &mc, &mr);
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn row22_tombstone_rebuild() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        // fill to a large-ish table, then alternately delete and re-insert other
        // keys so tombstone_count climbs past 3/16 * slot_count without the
        // used_count dropping below the shrink threshold.
        for i in 0..300u32 {
            let mut k = i.to_ne_bytes();
            let v = i.to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
        }
        for round in 0..300u32 {
            let mut kd = round.to_ne_bytes();
            assert_same(
                &format!("tomb del #{round}"),
                &mc.del(&mut kd, HM_BINARY),
                &mr.del(&mut kd, HM_BINARY),
            );
            let mut ka = (1000 + round).to_ne_bytes();
            let v = round.to_ne_bytes();
            mc.put(&mut ka, &v, 4, HM_BINARY);
            mr.put(&mut ka, &v, 4, HM_BINARY);
            cmp(&format!("tomb round #{round}"), &mc, &mr);
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn row23_delete_with_nonzero_keyoffset() {
    let _g = seed_lock();
    for &keyoffset in &[4usize, 8, 12] {
        reseed();
        let elemsize = 16;
        let (mut mc, mut mr) = pair(elemsize, 4, keyoffset, false);
        unsafe {
            for i in 0..30u32 {
                let mut k = i.to_ne_bytes();
                let v = [0xAAu8; 12];
                mc.put(&mut k, &v, 4, HM_BINARY);
                mr.put(&mut k, &v, 4, HM_BINARY);
            }
            cmp(&format!("keyoffset={keyoffset} filled"), &mc, &mr);
            // hmput_key always stores the key at offset 0, so a delete that looks
            // for it at `keyoffset` can never match -> the "not found" path.
            for i in 0..30u32 {
                let mut k = i.to_ne_bytes();
                assert_same(
                    &format!("keyoffset={keyoffset} del #{i}"),
                    &mc.del(&mut k, HM_BINARY),
                    &mr.del(&mut k, HM_BINARY),
                );
                cmp(&format!("keyoffset={keyoffset} del #{i} state"), &mc, &mr);
            }
            mc.free();
            mr.free();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 24-29: string modes
// ---------------------------------------------------------------------------

/// Runs a string-keyed workload (put/get/delete-last/delete-missing) and
/// compares everything.  `shmode` is `None` for the implicit `SH_DEFAULT` that
/// `hmput_key` installs itself.
fn string_workload(tag: &str, shmode: Option<c_int>, mode: c_int, n: usize, del_last_only: bool) {
    let _g = seed_lock();
    reseed();
    let elemsize = 16usize;
    let keysize = 8usize;
    let (mut mc, mut mr) = match shmode {
        Some(m) => pair_shmode(elemsize, keysize, 0, m),
        None => pair(elemsize, keysize, 0, true),
    };
    let mut rng = Rng::new(0xf00d);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for i in 0..n {
            let mut k: Vec<u8> = format!("k{:06}", i % (n / 2).max(1)).into_bytes();
            k.push(0);
            keys.push(k.clone());
            let key = keys.last_mut().unwrap();
            let v = rng.next_u64().to_ne_bytes();
            mc.put(key, &v, 8, mode);
            mr.put(key, &v, 8, mode);
            assert_same(&format!("{tag} temp_key #{i}"), &mc.temp_key(), &mr.temp_key());
            cmp(&format!("{tag} put #{i}"), &mc, &mr);
        }
        for i in 0..n + 5 {
            let mut k: Vec<u8> = format!("k{:06}", i).into_bytes();
            k.push(0);
            assert_same(
                &format!("{tag} get #{i}"),
                &mc.get(&mut k, mode),
                &mr.get(&mut k, mode),
            );
            cmp(&format!("{tag} get #{i} state"), &mc, &mr);
        }
        // deletions
        let ndel = if del_last_only { 1 } else { n };
        for i in 0..ndel {
            // delete the LAST array element when `del_last_only` (avoids the
            // `mode != STBDS_HM_STRING` re-find path, see ERRORS.md #18/A5)
            let idx = if del_last_only {
                let len = (*mc.header()).length;
                let p = (mc.a as *const u8).add((len - 2) * elemsize) as *const *const c_char;
                read_cstr(*p)
            } else {
                format!("k{:06}", i).into_bytes()
            };
            let mut k = idx.clone();
            k.push(0);
            assert_same(
                &format!("{tag} del #{i}"),
                &mc.del(&mut k, mode),
                &mr.del(&mut k, mode),
            );
            cmp(&format!("{tag} del #{i} state"), &mc, &mr);
        }
        // missing key
        let mut miss = b"nope-not-here\0".to_vec();
        assert_same(
            &format!("{tag} del missing"),
            &mc.del(&mut miss, mode),
            &mr.del(&mut miss, mode),
        );
        cmp(&format!("{tag} del missing state"), &mc, &mr);
        mc.free();
        mr.free();
    }
}

#[test]
fn row24_string_mode_implicit_default() {
    string_workload("implicit-default", None, HM_STRING, 200, false);
}

#[test]
fn row25_string_mode_strdup() {
    string_workload("strdup", Some(SH_STRDUP), HM_STRING, 200, false);
}

#[test]
fn row26_string_mode_arena() {
    string_workload("arena", Some(SH_ARENA), HM_STRING, 200, false);
    // long strings that exceed a whole arena block
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair_shmode(16, 8, 0, SH_ARENA);
    unsafe {
        for i in 0..40usize {
            let mut k: Vec<u8> = vec![b'a' + (i % 26) as u8; if i % 4 == 0 { 700 } else { 30 }];
            k.push(0);
            let v = (i as u64).to_ne_bytes();
            mc.put(&mut k, &v, 8, HM_STRING);
            mr.put(&mut k, &v, 8, HM_STRING);
            cmp(&format!("arena-long put #{i}"), &mc, &mr);
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn row27_string_mode_none_uses_memcpy_branch() {
    let _g = seed_lock();
    reseed();
    // string.mode == STBDS_SH_NONE with mode >= STBDS_HM_STRING: the `switch`
    // falls to `default:` and memcpy's `keysize` raw bytes into the element.
    // Only distinct keys and no lookups are safe here (a lookup would treat
    // those raw bytes as a `char*`), so the test compares the payload bytes.
    let (mut mc, mut mr) = pair_shmode(16, 8, 0, SH_NONE);
    mc.pointer_keys = false;
    mr.pointer_keys = false;
    unsafe {
        let mut live: Vec<Vec<u8>> = Vec::new();
        for i in 0..5usize {
            let mut k: Vec<u8> = format!("sk{}", i).into_bytes();
            while k.len() < 8 {
                k.push(0);
            }
            live.push(k);
            let key = live.last_mut().unwrap();
            let v = (i as u64).to_ne_bytes();
            mc.put(key, &v, 8, HM_STRING);
            mr.put(key, &v, 8, HM_STRING);
            cmp(&format!("sh_none put #{i}"), &mc, &mr);
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn row28_string_mode_explicit_default() {
    string_workload("explicit-default", Some(SH_DEFAULT), HM_STRING, 200, false);
}

#[test]
fn row29_mode_ptr_to_string() {
    // mode == 2: `mode >= STBDS_HM_STRING` for hashing/compare, but
    // `mode != STBDS_HM_STRING` in hmdel_key (no strdup free, binary re-find).
    string_workload("ptr2str-default", Some(SH_DEFAULT), HM_PTR_TO_STRING, 60, true);
    string_workload("ptr2str-strdup", Some(SH_STRDUP), HM_PTR_TO_STRING, 60, true);
    string_workload("ptr2str-arena", Some(SH_ARENA), HM_PTR_TO_STRING, 60, true);
    string_workload("ptr2str-implicit", None, HM_PTR_TO_STRING, 60, true);
}

// ---------------------------------------------------------------------------
// row 30: out-of-range `mode` values crossing the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn row30_out_of_range_mode_string_side() {
    for mode in [3 as c_int, 4, 1000, c_int::MAX] {
        string_workload(&format!("mode={mode}"), Some(SH_DEFAULT), mode, 40, true);
    }
}

#[test]
fn row30_out_of_range_mode_binary_side() {
    for mode in [-1 as c_int, -2, c_int::MIN] {
        let _g = seed_lock();
        reseed();
        let mut rng = Rng::new(0x77 + mode as u64);
        let (mut mc, mut mr) = pair(8, 4, 0, false);
        unsafe {
            for i in 0..120 {
                let mut k = (rng.below(50) as u32).to_ne_bytes();
                let v = rng.next_u32().to_ne_bytes();
                mc.put(&mut k, &v, 4, mode);
                mr.put(&mut k, &v, 4, mode);
                cmp(&format!("mode={mode} put #{i}"), &mc, &mr);
            }
            for i in 0..60u32 {
                let mut k = i.to_ne_bytes();
                assert_same(
                    &format!("mode={mode} get #{i}"),
                    &mc.get(&mut k, mode),
                    &mr.get(&mut k, mode),
                );
                assert_same(
                    &format!("mode={mode} del #{i}"),
                    &mc.del(&mut k, mode),
                    &mr.del(&mut k, mode),
                );
                cmp(&format!("mode={mode} state #{i}"), &mc, &mr);
            }
            mc.free();
            mr.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 31: shmode_func with out-of-enum modes (truncated to unsigned char)
// ---------------------------------------------------------------------------

#[test]
fn row31_shmode_func_out_of_enum() {
    for shmode in [
        0 as c_int, 1, 2, 3, 4, 5, 100, 255, 256, 257, 258, 259, 260, -1, -253, c_int::MIN,
        c_int::MAX,
    ] {
        let _g = seed_lock();
        reseed();
        let (mut mc, mut mr) = pair_shmode(16, 8, 0, shmode);
        let truncated = (shmode as u32 & 0xff) as u8;
        let pointer_path = matches!(truncated, 1 | 2 | 3);
        mc.pointer_keys = pointer_path;
        mr.pointer_keys = pointer_path;
        unsafe {
            assert_same(
                &format!("shmode={shmode} fresh"),
                &mc.snap(),
                &mr.snap(),
            );
            assert_eq!(
                mc.snap().unwrap().index.unwrap().arena_mode,
                truncated,
                "C truncation of shmode {shmode}"
            );
            // the SH_DEFAULT path stores the CALLER's pointer, so the key
            // buffers must outlive the map.
            let mut live: Vec<Vec<u8>> = Vec::new();
            for i in 0..6usize {
                let mut k: Vec<u8> = format!("key{}", i).into_bytes();
                k.push(0);
                while k.len() < 8 {
                    k.push(0);
                }
                live.push(k);
                let key = live.last_mut().unwrap();
                let v = (i as u64).to_ne_bytes();
                mc.put(key, &v, 8, HM_STRING);
                mr.put(key, &v, 8, HM_STRING);
                cmp(&format!("shmode={shmode} put #{i}"), &mc, &mr);
            }
            if pointer_path {
                for i in 0..8usize {
                    let mut k: Vec<u8> = format!("key{}", i).into_bytes();
                    k.push(0);
                    while k.len() < 8 {
                        k.push(0);
                    }
                    assert_same(
                        &format!("shmode={shmode} get #{i}"),
                        &mc.get(&mut k, HM_STRING),
                        &mr.get(&mut k, HM_STRING),
                    );
                    cmp(&format!("shmode={shmode} get #{i} state"), &mc, &mr);
                }
            }
            mc.free();
            mr.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 32: hmfree_func matrix
// ---------------------------------------------------------------------------

#[test]
fn row32_hmfree_matrix() {
    let (c, r) = apis();
    for shmode in [None, Some(SH_NONE), Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
        for nputs in [0usize, 1, 5, 40] {
            let _g = seed_lock();
            reseed();
            let (mut mc, mut mr) = match shmode {
                Some(m) => pair_shmode(16, 8, 0, m),
                None => pair(16, 8, 0, true),
            };
            if shmode == Some(SH_NONE) {
                mc.pointer_keys = false;
                mr.pointer_keys = false;
            }
            unsafe {
                let mut live: Vec<Vec<u8>> = Vec::new();
                for i in 0..nputs {
                    let mut k: Vec<u8> = format!("f{:05}", i).into_bytes();
                    k.push(0);
                    while k.len() < 8 {
                        k.push(0);
                    }
                    live.push(k);
                    let key = live.last_mut().unwrap();
                    let v = (i as u64).to_ne_bytes();
                    mc.put(key, &v, 8, HM_STRING);
                    mr.put(key, &v, 8, HM_STRING);
                }
                cmp(&format!("free-matrix {shmode:?}/{nputs} before"), &mc, &mr);
                mc.free();
                mr.free();
                assert!(mc.a.is_null() && mr.a.is_null());
            }
        }
    }
    // hmfree_func on a plain array (hash_table == NULL)
    unsafe {
        let ac = (c.arrgrowf)(std::ptr::null_mut(), 8, 4, 0);
        let ar = (r.arrgrowf)(std::ptr::null_mut(), 8, 4, 0);
        (c.hmfree_func)(ac, 8);
        (r.hmfree_func)(ar, 8);
    }
    // hmfree_func(NULL) is a documented no-op
    unsafe {
        (c.hmfree_func)(std::ptr::null_mut(), 8);
        (r.hmfree_func)(std::ptr::null_mut(), 8);
    }
}

// ---------------------------------------------------------------------------
// rows 39-40: mixed workload / growth interop
// ---------------------------------------------------------------------------

#[test]
fn row39_mixed_random_workload() {
    for (tag, shmode, mode) in [
        ("bin", None, HM_BINARY),
        ("str-default", None, HM_STRING),
        ("str-strdup", Some(SH_STRDUP), HM_STRING),
        ("str-arena", Some(SH_ARENA), HM_STRING),
    ] {
        for trial in 0..3u64 {
            let _g = seed_lock();
            reseed();
            let pointer_keys = mode >= HM_STRING;
            let (mut mc, mut mr) = match shmode {
                Some(m) => pair_shmode(16, 8, 0, m),
                None => pair(16, 8, 0, pointer_keys),
            };
            let mut rng = Rng::new(0xbeef_0000 + trial * 977);
            let mut live: Vec<Vec<u8>> = Vec::new();
            unsafe {
                for i in 0..400 {
                    let id = rng.below(90);
                    let mut key: Vec<u8> = if pointer_keys {
                        let mut k = format!("kk{:05}", id).into_bytes();
                        k.push(0);
                        k
                    } else {
                        (id as u64).to_ne_bytes().to_vec()
                    };
                    match rng.below(10) {
                        0..=4 => {
                            live.push(key.clone());
                            let k = live.last_mut().unwrap();
                            let v = rng.next_u64().to_ne_bytes();
                            mc.put(k, &v, 8, mode);
                            mr.put(k, &v, 8, mode);
                        }
                        5..=6 => {
                            assert_same(
                                &format!("{tag} t={trial} get #{i}"),
                                &mc.get(&mut key, mode),
                                &mr.get(&mut key, mode),
                            );
                        }
                        7 => {
                            assert_same(
                                &format!("{tag} t={trial} get_ts #{i}"),
                                &mc.get_ts(&mut key, mode),
                                &mr.get_ts(&mut key, mode),
                            );
                        }
                        8 => {
                            assert_same(
                                &format!("{tag} t={trial} del #{i}"),
                                &mc.del(&mut key, mode),
                                &mr.del(&mut key, mode),
                            );
                        }
                        _ => {
                            mc.put_default();
                            mr.put_default();
                        }
                    }
                    cmp(&format!("{tag} t={trial} op #{i}"), &mc, &mr);
                }
                mc.free();
                mr.free();
            }
        }
    }
}

#[test]
fn row40_array_realloc_keeps_hash_table() {
    let _g = seed_lock();
    reseed();
    let (mut mc, mut mr) = pair(8, 4, 0, false);
    unsafe {
        let mut caps = Vec::new();
        for i in 0..600u32 {
            let mut k = i.to_ne_bytes();
            let v = i.to_ne_bytes();
            mc.put(&mut k, &v, 4, HM_BINARY);
            mr.put(&mut k, &v, 4, HM_BINARY);
            let sc = mc.snap().unwrap();
            let sr = mr.snap().unwrap();
            assert_same(&format!("realloc-interop #{i}"), &sc, &sr);
            assert!(sc.has_table);
            caps.push(sc.capacity);
        }
        // the element array really did get reallocated several times
        caps.dedup();
        assert!(caps.len() > 5, "expected several reallocations, got {caps:?}");
        mc.free();
        mr.free();
    }
}
