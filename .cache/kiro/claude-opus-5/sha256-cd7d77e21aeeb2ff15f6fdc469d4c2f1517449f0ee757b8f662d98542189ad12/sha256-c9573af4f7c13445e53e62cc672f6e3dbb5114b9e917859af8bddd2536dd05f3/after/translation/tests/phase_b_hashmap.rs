//! Phase B — valid-path differential tests for the hash-map entry points.
//!
//! CONFIGS.md rows 14-34. Every map is driven through BOTH `.so` files in
//! lockstep by `common::DualMap`, and the complete state (array header, element
//! payload, and every field + every bucket slot of the hash index) is compared
//! after every single operation.

mod common;

use common::*;
use std::ffi::c_void;

/// `{ int key; int value; }`
const ES8: usize = 8;
const KS4: usize = 4;
/// `{ char *key; int value; }`
const ES16: usize = 16;
const KS8: usize = 8;
/// `{ int key[2]; int b, c, d; }`
const ES20: usize = 20;

fn i32b(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}

fn pad(mut v: Vec<u8>, n: usize) -> Vec<u8> {
    v.resize(n, 0);
    v
}

// ---------------------------------------------------------------------------
// Row 14 — stbds_shmode_func across all string modes and element sizes
// ---------------------------------------------------------------------------

#[test]
fn row14_shmode_func_modes_x_elemsizes() {
    let (p, _g) = libs();
    for &seed in &[DEFAULT_SEED, 0usize, 1, usize::MAX] {
        for &mode in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &elemsize in &[1usize, 2, 4, 8, 16, 20, 64] {
                reseed(p, seed);
                let tc = unsafe { (p.c.shmode_func)(elemsize, mode) };
                let tr = unsafe { (p.rs.shmode_func)(elemsize, mode) };
                let sc = unsafe { snap_hm(tc, elemsize, KeyKind::Bytes) };
                let sr = unsafe { snap_hm(tr, elemsize, KeyKind::Bytes) };
                assert_eq!(sc, sr, "shmode_func({elemsize},{mode}) seed={seed:#x}");
                assert_eq!(sc.length, 1);
                assert_eq!(sc.idx.as_ref().unwrap().string_mode, mode as u8);
                unsafe {
                    (p.c.hmfree_func)(hash_to_arr(tc, elemsize), elemsize);
                    (p.rs.hmfree_func)(hash_to_arr(tr, elemsize), elemsize);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — stbds_hmput_default
// ---------------------------------------------------------------------------

#[test]
fn row15_hmput_default() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    for &elemsize in &[1usize, 4, 8, 16, 20] {
        let keysize = elemsize.min(4);
        let mut m = DualMap::empty(p, elemsize, keysize, KeyKind::Bytes, "hmput_default");

        // (a) a == NULL -> bootstrap
        m.put_default(&vec![0xAA; elemsize - keysize]);
        let (s, _) = m.snaps();
        assert_eq!(s.length, 1, "hmput_default must produce length 1");

        // (b) called again with length != 0 -> pure no-op
        let before = m.snaps().0;
        m.put_default(&vec![0xBB; elemsize - keysize]);
        let after = m.snaps().0;
        assert_eq!(before.capacity, after.capacity, "no realloc expected");

        // (c) length forced to 0 -> grows and re-zeroes element 0
        unsafe {
            (*header(hash_to_arr(m.tc, elemsize))).length = 0;
            (*header(hash_to_arr(m.tr, elemsize))).length = 0;
        }
        m.put_default(&vec![0xCC; elemsize - keysize]);
        m.assert_same("hmput_default length==0");
        m.free();
    }
}

// ---------------------------------------------------------------------------
// Rows 16-18 — stbds_hmput_key, binary mode
// ---------------------------------------------------------------------------

#[test]
fn row16_hmput_binary_int_key_growth() {
    let (p, _g) = libs();
    // 1..64 inserts crosses the 8 -> 16 -> 32 -> 64 -> 128 rehash boundaries
    // (used_count_threshold is slot_count - slot_count/4).
    for trial in 0..16 {
        reseed(p, DEFAULT_SEED ^ trial);
        let mut rng = Rng::new(0x1600 + trial as u64);
        let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, &format!("bin8/trial{trial}"));
        m.put_default(&i32b(-2));
        for n in 0..96 {
            let k = if trial % 2 == 0 {
                n as i32
            } else {
                rng.i32() % 1000
            };
            m.put_bytes(&i32b(k), &i32b(k.wrapping_mul(5)));
            // read back through both APIs
            let i = m.geti(&i32b(k), STBDS_HM_BINARY);
            assert!(i >= 0, "just-inserted key {k} must be found");
            assert_eq!(i, m.geti_ts(&i32b(k), STBDS_HM_BINARY));
            // a key that is certainly absent
            m.geti(&i32b(i32::MIN + n as i32), STBDS_HM_BINARY);
        }
        m.free();
    }
}

#[test]
fn row17_hmput_binary_wide_key_with_duplicates() {
    let (p, _g) = libs();
    for trial in 0..12 {
        reseed(p, 12345 + trial);
        let mut rng = Rng::new(0x1700 + trial as u64);
        let mut m = DualMap::empty(p, ES20, KS8, KeyKind::Bytes, &format!("bin20/trial{trial}"));
        m.put_default(&vec![0x77; ES20 - KS8]);
        let mut seen: Vec<Vec<u8>> = Vec::new();
        for n in 0..120 {
            // 1/3 of the puts re-use an existing key -> the "key already
            // present" early-return branches of hmput_key
            let key = if !seen.is_empty() && rng.below(3) == 0 {
                seen[rng.below(seen.len())].clone()
            } else {
                let k = rng.bytes(KS8);
                seen.push(k.clone());
                k
            };
            let val: Vec<u8> = rng.bytes(ES20 - KS8);
            m.put_bytes(&key, &val);
            assert!(m.geti(&key, STBDS_HM_BINARY) >= 0);
            let _ = n;
        }
        for k in &seen {
            assert!(m.geti(k, STBDS_HM_BINARY) >= 0, "key vanished");
        }
        m.free();
    }
}

#[test]
fn row18_hmput_binary_key_is_whole_element() {
    let (p, _g) = libs();
    for &elemsize in &[1usize, 2, 4, 8, 16] {
        for trial in 0..8 {
            reseed(p, 0xABCD + trial);
            let mut rng = Rng::new(0x1800 + (elemsize * 100 + trial as usize) as u64);
            let mut m = DualMap::empty(
                p,
                elemsize,
                elemsize,
                KeyKind::Bytes,
                &format!("keyonly/{elemsize}/trial{trial}"),
            );
            for _ in 0..60 {
                let key = rng.bytes(elemsize);
                m.put_bytes(&key, &[]);
                assert!(m.geti(&key, STBDS_HM_BINARY) >= 0);
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 19-23 — stbds_hmput_key, string modes
// ---------------------------------------------------------------------------

fn string_keys(rng: &mut Rng, n: usize, tag: &str) -> Vec<*mut std::ffi::c_char> {
    (0..n)
        .map(|i| {
            let l = rng.below(24);
            leak_cstr(&format!("{tag}_{}{}", "z".repeat(l), i))
        })
        .collect()
}

#[test]
fn row19_shput_sh_default_autocreated() {
    let (p, _g) = libs();
    for trial in 0..12 {
        reseed(p, 0x777 + trial);
        let mut rng = Rng::new(0x1900 + trial as u64);
        // starting from NULL, hmput_key creates the index itself and sets
        // string.mode = SH_DEFAULT because mode >= STBDS_HM_STRING
        let mut m = DualMap::empty(p, ES16, KS8, KeyKind::Ptr, &format!("shdef/trial{trial}"));
        let keys = string_keys(&mut rng, 80, "d");
        for (n, &k) in keys.iter().enumerate() {
            let i = m.put_str(k, &pad(i32b(n as i32), ES16 - KS8), STBDS_HM_STRING);
            assert!(i >= 0);
            m.assert_temp_key_same();
            assert!(m.geti_str(k, STBDS_HM_STRING) >= 0);
        }
        // re-put every key (duplicate path)
        for (n, &k) in keys.iter().enumerate() {
            m.put_str(k, &pad(i32b(-(n as i32)), ES16 - KS8), STBDS_HM_STRING);
            m.assert_temp_key_same();
        }
        // absent keys
        for i in 0..20 {
            assert_eq!(
                m.geti_str(leak_cstr(&format!("absent_{i}")), STBDS_HM_STRING),
                -1
            );
        }
        m.free();
    }
}

#[test]
fn row20_shput_sh_strdup() {
    let (p, _g) = libs();
    for trial in 0..12 {
        reseed(p, 0x888 + trial);
        let mut rng = Rng::new(0x2000 + trial as u64);
        let mut m = DualMap::shmode(
            p,
            ES16,
            KS8,
            KeyKind::Ptr,
            SH_STRDUP,
            &format!("strdup/trial{trial}"),
        );
        let keys = string_keys(&mut rng, 80, "s");
        for (n, &k) in keys.iter().enumerate() {
            m.put_str(k, &pad(i32b(n as i32 * 3), ES16 - KS8), STBDS_HM_STRING);
            m.assert_temp_key_same();
            assert!(m.geti_str(k, STBDS_HM_STRING) >= 0);
        }
        // stored key must be a *copy*, not the caller pointer
        unsafe {
            let stored = *((m.tc as *const u8).add(0) as *const *const std::ffi::c_char);
            assert!(
                stored != keys[0] || keys.is_empty(),
                "SH_STRDUP must not store the caller pointer"
            );
        }
        for &k in &keys {
            m.put_str(k, &pad(i32b(9), ES16 - KS8), STBDS_HM_STRING);
        }
        m.free();
    }
}

#[test]
fn row21_shput_sh_arena() {
    let (p, _g) = libs();
    for trial in 0..12 {
        reseed(p, 0x999 + trial);
        let mut rng = Rng::new(0x2100 + trial as u64);
        let mut m = DualMap::shmode(
            p,
            ES16,
            KS8,
            KeyKind::Ptr,
            SH_ARENA,
            &format!("arena/trial{trial}"),
        );
        // mix short keys with keys long enough to force dedicated arena blocks
        for n in 0..120 {
            let k = if n % 17 == 0 {
                leak_cstr(&format!("{}{n}", "L".repeat(700 + rng.below(2000))))
            } else {
                leak_cstr(&format!("a_{}{n}", "y".repeat(rng.below(30))))
            };
            m.put_str(k, &pad(i32b(n), ES16 - KS8), STBDS_HM_STRING);
            m.assert_temp_key_same();
            assert!(m.geti_str(k, STBDS_HM_STRING) >= 0);
        }
        m.free();
    }
}

#[test]
fn row22_shput_sh_none_default_memcpy_branch() {
    let (p, _g) = libs();
    // string mode (>= STBDS_HM_STRING) over a table whose string.mode is 0,
    // so hmput_key falls into `default:` and memcpy's keysize bytes out of the
    // key STRING into the element. The element's first 8 bytes are therefore
    // string bytes, not a pointer -> snapshot as raw Bytes and never look the
    // keys up again (the C would strcmp through those bytes as a pointer).
    for trial in 0..8 {
        reseed(p, 0xA1A + trial);
        let mut m = DualMap::shmode(
            p,
            ES16,
            KS8,
            KeyKind::Bytes,
            SH_NONE,
            &format!("shnone/trial{trial}"),
        );
        for n in 0..5 {
            let k = leak_cstr(&format!("nk{n}_{trial}"));
            m.put_str(k, &pad(i32b(n), ES16 - KS8), STBDS_HM_STRING);
        }
        m.assert_same("sh_none puts");
        m.free();
    }
}

#[test]
fn row23_shput_out_of_range_string_modes() {
    let (p, _g) = libs();
    // any mode >= 1 is "string"; 2 and 99 have no enum variant but are legal
    // ints across the FFI boundary.
    for &mode in &[2i32, 3, 7, 99, i32::MAX] {
        for trial in 0..4 {
            reseed(p, 0xB2B + trial);
            let mut rng = Rng::new(0x2300 + mode as u64 + trial as u64);
            let mut m =
                DualMap::empty(p, ES16, KS8, KeyKind::Ptr, &format!("mode{mode}/trial{trial}"));
            let keys = string_keys(&mut rng, 40, "m");
            for (n, &k) in keys.iter().enumerate() {
                m.put_str(k, &pad(i32b(n as i32), ES16 - KS8), mode);
                assert!(m.geti_str(k, mode) >= 0);
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 24-26 — stbds_hmget_key / stbds_hmget_key_ts
// ---------------------------------------------------------------------------

#[test]
fn row24_hmget_on_null_and_indexless_and_populated() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);

    // (a) hmget_key on NULL: bootstraps a 1-element array, temp = -1
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "get/null");
    assert_eq!(m.geti(&i32b(1), STBDS_HM_BINARY), -1);
    let (s, _) = m.snaps();
    assert_eq!((s.length, s.temp), (1, -1));
    assert!(s.idx.is_none(), "hmget_key must not create an index");

    // (b) same array again: a != NULL but hash_table == NULL
    for k in 0..32 {
        assert_eq!(m.geti(&i32b(k), STBDS_HM_BINARY), -1);
        assert_eq!(m.geti_ts(&i32b(k), STBDS_HM_BINARY), -1);
    }
    m.free();

    // (c) populated
    let mut rng = Rng::new(0x2400);
    for trial in 0..8 {
        reseed(p, 0xC3C + trial);
        let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, &format!("get/pop{trial}"));
        m.put_default(&i32b(-2));
        let mut present: Vec<i32> = Vec::new();
        for _ in 0..70 {
            let k = rng.i32() % 500;
            m.put_bytes(&i32b(k), &i32b(k.wrapping_mul(7)));
            present.push(k);
        }
        for _ in 0..300 {
            let k = rng.i32() % 1000;
            let i = m.geti(&i32b(k), STBDS_HM_BINARY);
            let expect_found = present.contains(&k);
            assert_eq!(i >= 0, expect_found, "presence mismatch for {k}");
        }
        m.free();
    }
}

#[test]
fn row25_hmget_key_vs_ts_temp_semantics() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    let mut rng = Rng::new(0x2500);
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "get/ts");
    m.put_default(&i32b(-2));
    for _ in 0..60 {
        let k = rng.i32() % 200;
        m.put_bytes(&i32b(k), &i32b(k));
    }
    for _ in 0..400 {
        let k = rng.i32() % 400;
        // hmget_key mirrors temp into the header; hmget_key_ts must NOT.
        let sentinel: isize = 0x1234_5678;
        unsafe {
            (*header(hash_to_arr(m.tc, ES8))).temp = sentinel;
            (*header(hash_to_arr(m.tr, ES8))).temp = sentinel;
        }
        let ts = m.geti_ts(&i32b(k), STBDS_HM_BINARY);
        unsafe {
            assert_eq!(
                (*header(hash_to_arr(m.tc, ES8))).temp,
                sentinel,
                "C hmget_key_ts must not touch the header temp"
            );
            assert_eq!(
                (*header(hash_to_arr(m.tr, ES8))).temp,
                sentinel,
                "RS hmget_key_ts must not touch the header temp"
            );
        }
        let g = m.geti(&i32b(k), STBDS_HM_BINARY);
        assert_eq!(g, ts, "hmget_key and hmget_key_ts disagree for {k}");
    }
    m.free();
}

#[test]
fn row26_shget_across_string_modes() {
    let (p, _g) = libs();
    for &(mode, name) in &[
        (SH_DEFAULT, "SH_DEFAULT"),
        (SH_STRDUP, "SH_STRDUP"),
        (SH_ARENA, "SH_ARENA"),
    ] {
        for trial in 0..8 {
            reseed(p, 0xD4D + trial);
            let mut rng = Rng::new(0x2600 + trial as u64 + mode as u64 * 31);
            let mut m =
                DualMap::shmode(p, ES16, KS8, KeyKind::Ptr, mode, &format!("{name}/get{trial}"));
            let keys = string_keys(&mut rng, 60, "g");
            for (n, &k) in keys.iter().enumerate() {
                m.put_str(k, &pad(i32b(n as i32), ES16 - KS8), STBDS_HM_STRING);
            }
            for &k in &keys {
                assert!(m.geti_str(k, STBDS_HM_STRING) >= 0);
            }
            for i in 0..40 {
                assert_eq!(
                    m.geti_str(leak_cstr(&format!("nope_{i}_{trial}")), STBDS_HM_STRING),
                    -1
                );
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 27-31 — stbds_hmdel_key
// ---------------------------------------------------------------------------

#[test]
fn row27_hmdel_last_and_middle() {
    let (p, _g) = libs();
    for trial in 0..12 {
        reseed(p, 0xE5E + trial);
        let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, &format!("del/lm{trial}"));
        m.put_default(&i32b(-2));
        let n = 24i32;
        for k in 0..n {
            m.put_bytes(&i32b(k), &i32b(k * 3));
        }
        // delete the most recently inserted (old_index == final_index)
        assert_eq!(m.del_bytes(&i32b(n - 1), 0, STBDS_HM_BINARY), 1);
        // delete from the middle (memmove + slot re-point)
        assert_eq!(m.del_bytes(&i32b(n / 2), 0, STBDS_HM_BINARY), 1);
        assert_eq!(m.del_bytes(&i32b(0), 0, STBDS_HM_BINARY), 1);
        // deleting again must be a no-op returning 0
        assert_eq!(m.del_bytes(&i32b(0), 0, STBDS_HM_BINARY), 0);
        // remaining keys still reachable
        for k in 1..n - 1 {
            if k == n / 2 {
                assert_eq!(m.geti(&i32b(k), STBDS_HM_BINARY), -1);
            } else {
                assert!(m.geti(&i32b(k), STBDS_HM_BINARY) >= 0, "lost key {k}");
            }
        }
        m.free();
    }
}

#[test]
fn row28_hmdel_tombstone_rebuild() {
    let (p, _g) = libs();
    for trial in 0..8 {
        reseed(p, 0xF6F + trial);
        let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, &format!("del/tomb{trial}"));
        m.put_default(&i32b(-2));
        // grow to 64 slots, then churn: repeated put/delete of fresh keys drives
        // tombstone_count past tombstone_count_threshold without shrinking.
        for k in 0..48i32 {
            m.put_bytes(&i32b(k), &i32b(k));
        }
        let mut next = 1000i32;
        for _ in 0..200 {
            m.put_bytes(&i32b(next), &i32b(next));
            m.del_bytes(&i32b(next), 0, STBDS_HM_BINARY);
            next += 1;
        }
        for k in 0..48i32 {
            assert!(m.geti(&i32b(k), STBDS_HM_BINARY) >= 0, "lost key {k}");
        }
        m.free();
    }
}

#[test]
fn row29_hmdel_shrink_rebuild() {
    let (p, _g) = libs();
    for trial in 0..8 {
        reseed(p, 0x1717 + trial);
        let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, &format!("del/shrink{trial}"));
        m.put_default(&i32b(-2));
        // grow well past 32 slots ...
        let n = 200i32;
        for k in 0..n {
            m.put_bytes(&i32b(k), &i32b(k));
        }
        let sc = m.snaps().0;
        assert!(sc.idx.as_ref().unwrap().slot_count >= 256);
        // ... then delete almost everything, crossing every shrink threshold
        for k in 0..n {
            m.del_bytes(&i32b(k), 0, STBDS_HM_BINARY);
        }
        let sc = m.snaps().0;
        assert_eq!(sc.idx.as_ref().unwrap().slot_count, 8, "should shrink to 8");
        assert_eq!(sc.length, 1, "only the default slot remains");
        for k in 0..n {
            assert_eq!(m.geti(&i32b(k), STBDS_HM_BINARY), -1);
        }
        m.free();
    }
}

#[test]
fn row30_shdel_across_string_modes() {
    let (p, _g) = libs();
    for &(mode, name) in &[
        (SH_DEFAULT, "SH_DEFAULT"),
        (SH_STRDUP, "SH_STRDUP"),
        (SH_ARENA, "SH_ARENA"),
    ] {
        for trial in 0..8 {
            reseed(p, 0x1818 + trial);
            let mut rng = Rng::new(0x3000 + trial as u64 + mode as u64 * 17);
            let mut m =
                DualMap::shmode(p, ES16, KS8, KeyKind::Ptr, mode, &format!("{name}/del{trial}"));
            let keys = string_keys(&mut rng, 70, "x");
            for (n, &k) in keys.iter().enumerate() {
                m.put_str(k, &pad(i32b(n as i32), ES16 - KS8), STBDS_HM_STRING);
            }
            // delete in a shuffled order
            let mut order: Vec<usize> = (0..keys.len()).collect();
            for i in (1..order.len()).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            for &i in &order {
                assert_eq!(m.del_str(keys[i], 0, STBDS_HM_STRING), 1);
                assert_eq!(m.del_str(keys[i], 0, STBDS_HM_STRING), 0, "double delete");
            }
            let s = m.snaps().0;
            assert_eq!(s.length, 1);
            m.free();
        }
    }
}

#[test]
fn row31_shdel_mode2_skips_strdup_free() {
    let (p, _g) = libs();
    // `hmdel_key` frees the strdup'd key only when `mode == STBDS_HM_STRING`
    // *exactly*, and likewise only re-locates the moved element with
    // `hash_string` when `mode == 1`. With `mode == 2` the hashing is still
    // string-based but neither of those two `mode == 1` branches is taken.
    //
    // (a) deleting in reverse insertion order keeps `old_index == final_index`,
    // so the re-locate step is skipped and the call completes normally.
    for trial in 0..8 {
        reseed(p, 0x1919 + trial);
        let mut rng = Rng::new(0x3100 + trial as u64);
        let mut m = DualMap::shmode(
            p,
            ES16,
            KS8,
            KeyKind::Ptr,
            SH_STRDUP,
            &format!("mode2-strdup-lifo/{trial}"),
        );
        let keys = string_keys(&mut rng, 40, "q");
        for (n, &k) in keys.iter().enumerate() {
            m.put_str(k, &pad(i32b(n as i32), ES16 - KS8), 2);
        }
        for &k in keys.iter().rev() {
            assert_eq!(m.del_str(k, 0, 2), 1);
        }
        assert_eq!(m.snaps().0.length, 1);
        m.free();
    }

    // (b) deleting a NON-last element with `mode == 2` makes the C re-locate the
    // moved element by hashing the *element bytes* as a string (the `else`
    // branch), which cannot find the slot, so `STBDS_ASSERT(slot >= 0)` aborts.
    // The Rust must abort identically. Compared via child exit status.
    let o = assert_same_outcome(p, "mode2-strdup-fifo", |lib| unsafe {
        (lib.rand_seed)(0x1919);
        let mut m = SoloMap::shmode(lib, ES16, KS8, SH_STRDUP);
        let keys: Vec<_> = (0..8)
            .map(|i| leak_cstr(&format!("fifo_{i}")))
            .collect();
        for (n, &k) in keys.iter().enumerate() {
            m.put_str(k, &pad(i32b(n as i32), ES16 - KS8), 2);
        }
        // deletes the FIRST element => old_index != final_index
        m.del_str(keys[0], 0, 2);
        m.free();
    });
    assert_eq!(
        o,
        Outcome::Signalled(6),
        "both builds are expected to abort here (live assert)"
    );
}

// ---------------------------------------------------------------------------
// Rows 32-33 — randomized interleaved op streams
// ---------------------------------------------------------------------------

#[test]
fn row32_random_op_stream_binary() {
    let (p, _g) = libs();
    for trial in 0..6 {
        reseed(p, 0x2A2A + trial);
        let mut rng = Rng::new(0x3200 + trial as u64);
        let key_space = [8usize, 64, 4096][trial as usize % 3];
        let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, &format!("stream/bin{trial}"));
        m.put_default(&i32b(-2));
        let mut live: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
        for step in 0..2000 {
            let k = rng.below(key_space) as i32;
            match rng.below(10) {
                0..=4 => {
                    m.put_bytes(&i32b(k), &i32b(k ^ step));
                    live.insert(k);
                }
                5..=6 => {
                    let i = m.geti(&i32b(k), STBDS_HM_BINARY);
                    assert_eq!(i >= 0, live.contains(&k), "presence mismatch k={k}");
                }
                7 => {
                    let i = m.geti_ts(&i32b(k), STBDS_HM_BINARY);
                    assert_eq!(i >= 0, live.contains(&k), "presence mismatch (ts) k={k}");
                }
                _ => {
                    let r = m.del_bytes(&i32b(k), 0, STBDS_HM_BINARY);
                    assert_eq!(r == 1, live.contains(&k), "delete result mismatch k={k}");
                    live.remove(&k);
                }
            }
        }
        m.free();
    }
}

#[test]
fn row33_random_op_stream_string() {
    let (p, _g) = libs();
    for &(mode, name) in &[
        (SH_DEFAULT, "SH_DEFAULT"),
        (SH_STRDUP, "SH_STRDUP"),
        (SH_ARENA, "SH_ARENA"),
    ] {
        for trial in 0..3 {
            reseed(p, 0x2B2B + trial);
            let mut rng = Rng::new(0x3300 + trial as u64 + mode as u64 * 7);
            let space: Vec<*mut std::ffi::c_char> = (0..128)
                .map(|i| leak_cstr(&format!("st{name}{trial}_{i}")))
                .collect();
            let mut m = DualMap::shmode(
                p,
                ES16,
                KS8,
                KeyKind::Ptr,
                mode,
                &format!("stream/{name}{trial}"),
            );
            let mut live = std::collections::BTreeSet::new();
            for step in 0..1200 {
                let ki = rng.below(space.len());
                let k = space[ki];
                match rng.below(10) {
                    0..=4 => {
                        m.put_str(k, &pad(i32b(step as i32), ES16 - KS8), STBDS_HM_STRING);
                        live.insert(ki);
                    }
                    5..=7 => {
                        let i = m.geti_str(k, STBDS_HM_STRING);
                        assert_eq!(i >= 0, live.contains(&ki), "presence mismatch");
                    }
                    _ => {
                        let r = m.del_str(k, 0, STBDS_HM_STRING);
                        assert_eq!(r == 1, live.contains(&ki), "delete result mismatch");
                        live.remove(&ki);
                    }
                }
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// Row 34 — stbds_hmfree_func across every table shape
// ---------------------------------------------------------------------------

#[test]
fn row34_hmfree_func_shapes() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);

    // (a) NULL is a documented no-op in both
    unsafe {
        (p.c.hmfree_func)(std::ptr::null_mut(), ES8);
        (p.rs.hmfree_func)(std::ptr::null_mut(), ES8);
    }

    // (b) index-less array (produced by hmget_key on NULL)
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "free/indexless");
    m.geti(&i32b(1), STBDS_HM_BINARY);
    assert!(m.snaps().0.idx.is_none());
    m.free();

    // (c) populated binary table
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "free/binary");
    m.put_default(&i32b(-2));
    for k in 0..40i32 {
        m.put_bytes(&i32b(k), &i32b(k));
    }
    m.free();

    // (d)/(e) SH_STRDUP and SH_ARENA (hmfree_func must free keys / reset arena)
    for &mode in &[SH_STRDUP, SH_ARENA, SH_DEFAULT, SH_NONE] {
        reseed(p, 0x3434);
        let mut rng = Rng::new(0x3400 + mode as u64);
        let kind = if mode == SH_NONE {
            KeyKind::Bytes
        } else {
            KeyKind::Ptr
        };
        let mut m = DualMap::shmode(p, ES16, KS8, kind, mode, &format!("free/mode{mode}"));
        for n in 0..(if mode == SH_NONE { 4 } else { 60 }) {
            let l = rng.below(if mode == SH_ARENA { 900 } else { 20 });
            let k = leak_cstr(&format!("f{}{n}", "p".repeat(l)));
            m.put_str(k, &pad(i32b(n), ES16 - KS8), STBDS_HM_STRING);
        }
        m.assert_same("before free");
        m.free();
    }

    // (f) freeing an array grown by arrgrowf but never used as a map
    unsafe {
        let ac = (p.c.arrgrowf)(std::ptr::null_mut(), ES8, 4, 0);
        let ar = (p.rs.arrgrowf)(std::ptr::null_mut(), ES8, 4, 0);
        (*header(ac)).length = 1;
        (*header(ar)).length = 1;
        std::ptr::write_bytes(ac as *mut u8, 0, ES8);
        std::ptr::write_bytes(ar as *mut u8, 0, ES8);
        let sc = snap_arr(ac, ES8, KeyKind::Bytes);
        let sr = snap_arr(ar, ES8, KeyKind::Bytes);
        assert_eq!(sc, sr);
        (p.c.hmfree_func)(ac as *mut c_void, ES8);
        (p.rs.hmfree_func)(ar as *mut c_void, ES8);
    }
}
