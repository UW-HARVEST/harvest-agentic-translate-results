//! Phase C — error-path differential tests, one per reachable row of
//! `ERRORS.md`, plus the generic FFI boundaries (null pointers, zero lengths,
//! out-of-range enum values passed as plain `int`).
//!
//! Rows whose trigger aborts the process are compared by re-executing this test
//! binary as a child (`abort_child`) once per library and comparing the exact
//! termination status.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

const PIN: usize = 0x3141_5926;
const HM_BINARY: c_int = 0;
const HM_STRING: c_int = 1;
const SH_DEFAULT: c_int = 1;

fn key4(v: u32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

/// ERRORS row 1: `stbds_hmfree_func(NULL, elemsize)` is a defined no-op.
#[test]
fn err01_hmfree_null() {
    let _g = lock();
    let (c, r) = libs();
    for elemsize in [0usize, 1, 8, 4096] {
        unsafe {
            (c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

/// ERRORS row 2: `hmfree_func` on an array that has no hash table.
#[test]
fn err02_hmfree_no_table() {
    let _g = lock();
    let mut m = MapPair::new(8, 4, KeyKind::Bytes);
    m.put_default();
    assert!(!m.snap().0.has_table);
    m.free(); // must not crash on either side
}

/// ERRORS row 3: `stbds_hmdel_key(NULL, ...)` returns NULL — the only NULL
/// sentinel in the API.
#[test]
fn err03_hmdel_null_returns_null() {
    let _g = lock();
    let (c, r) = libs();
    let mut key = key4(1234);
    for mode in [HM_BINARY, HM_STRING, 2, -1, c_int::MAX, c_int::MIN] {
        for keyoffset in [0usize, 4, 8] {
            unsafe {
                let cp = (c.hmdel_key)(
                    std::ptr::null_mut(), 8, key.as_mut_ptr() as *mut c_void, 4, keyoffset, mode);
                let rp = (r.hmdel_key)(
                    std::ptr::null_mut(), 8, key.as_mut_ptr() as *mut c_void, 4, keyoffset, mode);
                assert!(cp.is_null(), "C hmdel_key(NULL) must return NULL");
                assert!(rp.is_null(), "Rust hmdel_key(NULL) must return NULL");
            }
        }
    }
}

/// ERRORS row 4: `hmdel_key` on a map with `hash_table == 0` — sets
/// `temp = 0`, returns the map unchanged.
#[test]
fn err04_hmdel_no_table() {
    let _g = lock();
    let mut m = MapPair::new(8, 4, KeyKind::Bytes);
    m.put_default();
    // poison temp so we can see it being set to 0
    unsafe {
        let ch = (m.c as *mut u8).sub(m.elemsize).sub(HEADER_SIZE) as *mut RawArrayHeader;
        let rh = (m.r as *mut u8).sub(m.elemsize).sub(HEADER_SIZE) as *mut RawArrayHeader;
        (*ch).temp = 0x7777;
        (*rh).temp = 0x7777;
    }
    let before_len = m.snap().0.length;
    let t = m.del(&key4(42), HM_BINARY);
    assert_eq!(t, 0, "hmdel_key with no table must leave temp == 0");
    assert_eq!(m.snap().0.length, before_len, "length must be unchanged");
    assert!(!m.snap().0.has_table);
    m.free();
}

/// ERRORS row 5: `hmdel_key` for a key that is not present.
#[test]
fn err05_hmdel_absent_key() {
    let _g = lock();
    for n in [1usize, 6, 8, 40] {
        let mut m = MapPair::new(8, 4, KeyKind::Bytes);
        m.pin_seed(PIN);
        for i in 0..n as u32 {
            m.put(&key4(i * 37 + 1), HM_BINARY, i as u8);
        }
        let before = m.snap().0;
        let t = m.del(&key4(0xDEAD_BEEF), HM_BINARY);
        let after = m.snap().0;
        assert_eq!(t, 0, "n={n}: absent delete must report temp == 0");
        assert_eq!(after.length, before.length, "n={n}: length changed");
        assert_eq!(after.used_count, before.used_count, "n={n}: used_count changed");
        assert_eq!(after.tombstone_count, before.tombstone_count, "n={n}: tombstone_count changed");
        assert_eq!(after.bucket_hash, before.bucket_hash, "n={n}: buckets changed");
        m.free();
    }
}

/// ERRORS rows 6-9: the three `-1` sentinel paths of `hmget_key_ts` /
/// `hmget_key`.
#[test]
fn err06_09_get_sentinels() {
    let _g = lock();
    let (c, r) = libs();

    // row 6: a == NULL -> allocates, *temp = -1
    for mode in [HM_BINARY, HM_STRING, 2, -5] {
        for elemsize in [8usize, 16, 24] {
            unsafe {
                let mut k = b"abcdefg\0".to_vec();
                let mut ct: isize = 0x1234;
                let mut rt: isize = 0x1234;
                let cp = (c.hmget_key_ts)(
                    std::ptr::null_mut(), elemsize, k.as_mut_ptr() as *mut c_void, 8, &mut ct, mode);
                let rp = (r.hmget_key_ts)(
                    std::ptr::null_mut(), elemsize, k.as_mut_ptr() as *mut c_void, 8, &mut rt, mode);
                assert_eq!(ct, -1, "C: hmget_key_ts(NULL) must set *temp = -1");
                assert_eq!(rt, -1, "Rust: hmget_key_ts(NULL) must set *temp = -1");
                assert!(!cp.is_null() && !rp.is_null(), "must allocate");
                let a = snapshot_map(cp, elemsize, 0, KeyKind::Bytes);
                let b = snapshot_map(rp, elemsize, 0, KeyKind::Bytes);
                assert_eq!(a, b, "mode={mode} elemsize={elemsize}");
                assert_eq!(a.length, 1);
                assert!(!a.has_table);
                (c.hmfree_func)((cp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (r.hmfree_func)((rp as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }

    // row 7: table == 0 on a non-NULL map; row 9: same through hmget_key
    let mut m = MapPair::new(16, 4, KeyKind::Bytes);
    m.put_default();
    assert_eq!(m.get_ts(&key4(7), HM_BINARY), -1, "row7 get_ts");
    assert_eq!(m.get(&key4(7), HM_BINARY), -1, "row7/9 get");
    m.free();

    // row 8: key absent from a populated table (both probe halves)
    let mut m = MapPair::new(8, 4, KeyKind::Bytes);
    m.pin_seed(PIN);
    for i in 0..60u32 {
        m.put(&key4(i * 4099 + 7), HM_BINARY, i as u8);
    }
    let mut misses = 0;
    for probe in 0..500u32 {
        let k = 0x8000_0000u32 + probe;
        if m.get_ts(&key4(k), HM_BINARY) == -1 {
            misses += 1;
        }
        assert_eq!(m.get(&key4(k), HM_BINARY), m.get_ts(&key4(k), HM_BINARY));
    }
    assert!(misses > 400, "expected mostly misses, got {misses}");
    m.free();
}

/// ERRORS rows 10-11: `stbds_hm_find_slot` returning -1 from each of its two
/// loop halves. A large table with long probe chains guarantees both halves are
/// reached; the observable is that C and Rust agree on every single lookup.
#[test]
fn err10_11_find_slot_both_halves() {
    let _g = lock();
    for seed in [0usize, 1, PIN, usize::MAX] {
        let mut m = MapPair::new(8, 4, KeyKind::Bytes);
        m.pin_seed(seed);
        // dense table -> many buckets nearly full -> wrap-around half exercised
        for i in 0..200u32 {
            m.put(&key4(i), HM_BINARY, i as u8);
        }
        for i in 0..400u32 {
            let t = m.get(&key4(i), HM_BINARY);
            if i < 200 {
                assert!(t >= 0, "seed={seed:#x}: lost key {i}");
            } else {
                assert_eq!(t, -1, "seed={seed:#x}: phantom key {i}");
            }
        }
        m.free();
    }
}

/// ERRORS rows 12, 13, 13a: `stbds_arrgrowf` early return and capacity floor.
#[test]
fn err12_13_arrgrowf_boundaries() {
    let _g = lock();
    let (c, r) = libs();
    unsafe {
        // row 13: NULL + addlen 0 + min_cap 0 -> returns NULL, allocates nothing
        for elemsize in [0usize, 1, 8, 64, 4096] {
            let cp = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let rp = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(cp.is_null(), "C arrgrowf(NULL,{elemsize},0,0) must be NULL");
            assert!(rp.is_null(), "Rust arrgrowf(NULL,{elemsize},0,0) must be NULL");
        }
        // row 13a: capacity floor of 4
        for elemsize in [1usize, 8, 16] {
            for (addlen, min_cap) in [(1usize, 0usize), (2, 0), (3, 0), (0, 1), (0, 2), (0, 3)] {
                let cp = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let rp = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ch = *((cp as *mut u8).sub(HEADER_SIZE) as *const RawArrayHeader);
                let rh = *((rp as *mut u8).sub(HEADER_SIZE) as *const RawArrayHeader);
                assert_eq!(ch.capacity, 4, "C floor for ({addlen},{min_cap})");
                assert_eq!(rh.capacity, 4, "Rust floor for ({addlen},{min_cap})");
                assert_eq!(ch.length, rh.length);
                assert_eq!(ch.temp, rh.temp);
                assert_eq!(ch.hash_table.is_null(), rh.hash_table.is_null());
                (c.arrfreef)(cp);
                (r.arrfreef)(rp);
            }
            // row 12: min_cap <= cap -> unchanged pointer
            let cp = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8);
            let rp = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8);
            for min_cap in [0usize, 1, 7, 8] {
                let cp2 = (c.arrgrowf)(cp, elemsize, 0, min_cap);
                let rp2 = (r.arrgrowf)(rp, elemsize, 0, min_cap);
                assert_eq!(cp2, cp, "C must return the same pointer for min_cap={min_cap}");
                assert_eq!(rp2, rp, "Rust must return the same pointer for min_cap={min_cap}");
            }
            (c.arrfreef)(cp);
            (r.arrfreef)(rp);
        }
    }
}

/// ERRORS rows 14-15: `hmput_default` with `length == 0` and with `length != 0`.
#[test]
fn err14_15_hmput_default_branches() {
    let _g = lock();
    let mut m = MapPair::new(16, 4, KeyKind::Bytes);
    m.put_default();
    let after_first = m.snap().0;
    assert_eq!(after_first.length, 1);
    // row 15: no-op
    m.put_default();
    assert_eq!(m.snap().0, after_first, "row15 must be a no-op");
    // row 14: length == 0 -> re-grow and re-zero
    unsafe {
        for p in [m.c, m.r] {
            let h = (p as *mut u8).sub(m.elemsize).sub(HEADER_SIZE) as *mut RawArrayHeader;
            (*h).length = 0;
            // dirty the element so we can see the re-zeroing
            std::ptr::write_bytes((p as *mut u8).sub(m.elemsize), 0xCC, m.elemsize);
        }
    }
    m.put_default();
    let (a, _) = m.snap();
    assert_eq!(a.length, 1);
    assert_eq!(a.elements[0], vec![0u8; m.elemsize], "row14 must re-zero element 0");
    m.free();
}

/// ERRORS row 27 / CONFIGS row 29+33: `mode` is a plain `int` with no enum
/// validation. Every value `< 1` must behave like `STBDS_HM_BINARY` and every
/// value `>= 1` like `STBDS_HM_STRING` for put/get.
#[test]
fn err27_out_of_range_mode_enum() {
    let _g = lock();

    // binary half: all these must produce byte-identical maps
    let binary_modes: [c_int; 6] = [0, -1, -2, -1000, c_int::MIN, c_int::MIN + 1];
    let mut reference: Option<MapSnapshot> = None;
    for mode in binary_modes {
        let mut m = MapPair::new(8, 4, KeyKind::Bytes);
        m.pin_seed(PIN);
        for i in 0..25u32 {
            m.put(&key4(i * 991 + 3), mode, i as u8);
        }
        for i in 0..25u32 {
            assert!(m.get(&key4(i * 991 + 3), mode) >= 0, "mode={mode}");
        }
        assert_eq!(m.get(&key4(0xFFFF_0000), mode), -1, "mode={mode}");
        let s = m.snap().0;
        match &reference {
            Some(prev) => assert_eq!(&s, prev, "mode={mode} must equal mode=0 exactly"),
            None => reference = Some(s),
        }
        m.free();
    }

    // string half: 1, 2, 3, 999, INT_MAX must all behave like STBDS_HM_STRING
    let string_modes: [c_int; 6] = [1, 2, 3, 4, 999, c_int::MAX];
    let mut reference: Option<MapSnapshot> = None;
    let keys: Vec<Vec<u8>> = (0..25)
        .map(|i| format!("key_{i}\0").into_bytes())
        .collect();
    for mode in string_modes {
        let mut m = MapPair::with_shmode(24, 8, KeyKind::StrPtr, SH_DEFAULT, PIN);
        for (i, k) in keys.iter().enumerate() {
            m.put(k, mode, i as u8);
        }
        for k in &keys {
            assert!(m.get(k, mode) >= 0, "mode={mode}: lost key");
        }
        assert_eq!(m.get(b"nope\0", mode), -1, "mode={mode}");
        let s = m.snap().0;
        match &reference {
            Some(prev) => assert_eq!(&s, prev, "mode={mode} must equal mode=1 exactly"),
            None => reference = Some(s),
        }
        m.free();
    }
}

/// ERRORS row 28 / CONFIGS row 57: `stbds_shmode_func`'s truncating
/// `(unsigned char) mode` cast, and the resulting `switch` fall-through to
/// `default:` for `string.mode` values with no enum variant.
#[test]
fn err28_shmode_out_of_range() {
    let _g = lock();
    let (c, r) = libs();
    // every possible resulting byte, including the 4..=255 range that has no
    // enum variant and therefore hits the `default:` memcpy branch
    let modes: [c_int; 14] = [4, 5, 100, 254, 255, 256, 257, 258, 259, 260, 511, 512, c_int::MIN, c_int::MAX];
    for mode in modes {
        unsafe {
            (c.rand_seed)(PIN);
            (r.rand_seed)(PIN);
            let ct = (c.shmode_func)(16, mode);
            let rt = (r.shmode_func)(16, mode);
            let want = (mode as u32 & 0xFF) as u8;
            let a = snapshot_map(ct, 16, 0, KeyKind::Bytes);
            let b = snapshot_map(rt, 16, 0, KeyKind::Bytes);
            assert_eq!(a, b, "shmode_func(16, {mode})");
            assert_eq!(a.string_mode, want, "shmode_func(16, {mode}) string.mode");
            (c.hmfree_func)((ct as *mut u8).sub(16) as *mut c_void, 16);
            (r.hmfree_func)((rt as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
    // and drive the resulting default: branch with real puts for a value that
    // aliases no variant (255) and one that aliases SH_ARENA via 256+3
    for (mode, expect) in [(255i32, 255u8), (259, 3)] {
        let mut m = MapPair::with_shmode(16, 8, KeyKind::Bytes, mode, PIN);
        assert_eq!(m.snap().0.string_mode, expect);
        for (i, k) in [&b"aaaaaaa\0"[..], b"bbbbbbb\0", b"ccccccc\0"].iter().enumerate() {
            // mode 255 -> default: memcpy branch (inline key, binary-style);
            // mode 3 -> SH_ARENA which copies into the arena.
            m.kind = if expect == 3 { KeyKind::StrPtr } else { KeyKind::Bytes };
            m.put(k, HM_STRING, 0x50 + i as u8);
        }
        m.free();
    }
}

/// ERRORS rows 29-30: zero-length inputs.
#[test]
fn err29_30_zero_length() {
    let _g = lock();
    let (c, r) = libs();
    unsafe {
        // hash_bytes(len = 0) never dereferences `p`, so even NULL is defined
        for seed in [0usize, 1, PIN, usize::MAX, usize::MAX >> 1, 1 << 63] {
            let ch = (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let rh = (r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(ch, rh, "hash_bytes(NULL, 0, {seed:#x})");
            let mut buf = [0u8; 8];
            let ch2 = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            let rh2 = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(ch2, rh2);
            assert_eq!(ch, ch2, "hash_bytes(len=0) must ignore the pointer");
            // empty string
            let mut e = [0u8; 1];
            let cs = (c.hash_string)(e.as_mut_ptr() as *mut c_char, seed);
            let rs = (r.hash_string)(e.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(cs, rs, "hash_string(\"\", {seed:#x})");
        }
    }
}

/// Generic boundary: `keysize == 0`. `memcmp(_, _, 0) == 0` makes every key
/// compare equal, so the map collapses to a single entry. Also `key == NULL`
/// with `keysize == 0`, which never dereferences the key.
#[test]
fn err_generic_keysize_zero() {
    let _g = lock();
    let (c, r) = libs();
    for elemsize in [8usize, 16] {
        let mut m = MapPair::new(elemsize, 0, KeyKind::Bytes);
        m.pin_seed(PIN);
        for i in 0..10u32 {
            m.put(&key4(i), HM_BINARY, 0x10 + i as u8);
        }
        let (a, _) = m.snap();
        assert_eq!(a.length, 2, "keysize=0 collapses to one real entry");
        assert_eq!(a.used_count, 1);
        assert_eq!(m.get(&key4(12345), HM_BINARY), 0, "any key matches");
        m.free();
    }
    // key == NULL, keysize == 0: neither the hash nor the compare nor the
    // `memcpy(elem, key, 0)` dereferences the key.
    unsafe {
        (c.rand_seed)(PIN);
        (r.rand_seed)(PIN);
        let cp = (c.hmput_key)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 0, HM_BINARY);
        let rp = (r.hmput_key)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 0, HM_BINARY);
        // `memcpy(elem, key, 0)` writes nothing, so the new element is still
        // uninitialised realloc memory. Fill it identically on both sides
        // before comparing (the C leaves it undefined, so it is not part of the
        // observable contract).
        let ct = (*((cp as *mut u8).sub(16).sub(HEADER_SIZE) as *const RawArrayHeader)).temp;
        let rt = (*((rp as *mut u8).sub(16).sub(HEADER_SIZE) as *const RawArrayHeader)).temp;
        assert_eq!(ct, rt, "temp for NULL-key put");
        std::ptr::write_bytes((cp as *mut u8).offset(16 * ct), 0x77, 16);
        std::ptr::write_bytes((rp as *mut u8).offset(16 * rt), 0x77, 16);
        let a = snapshot_map(cp, 16, 0, KeyKind::Bytes);
        let b = snapshot_map(rp, 16, 0, KeyKind::Bytes);
        assert_eq!(a, b, "hmput_key(NULL map, NULL key, keysize 0)");
        (c.hmfree_func)((cp as *mut u8).sub(16) as *mut c_void, 16);
        (r.hmfree_func)((rp as *mut u8).sub(16) as *mut c_void, 16);
    }
}

/// CONFIGS row 51 / generic boundary: `hmdel_key` with a `keyoffset` that does
/// not match where `hmput_key` stored the key (`hmput_key` hardcodes
/// `keyoffset = 0`). The comparison then reads the value bytes, never matches,
/// and the delete degenerates into the "key not found" path.
#[test]
fn err_generic_keyoffset_mismatch() {
    let _g = lock();
    for keyoffset in [4usize, 8, 12] {
        let mut m = MapPair::new(16, 4, KeyKind::Bytes);
        m.pin_seed(PIN);
        for i in 0..20u32 {
            m.put(&key4(i * 13 + 1), HM_BINARY, 0xEE);
        }
        let before = m.snap().0;
        m.keyoffset = keyoffset;
        for i in 0..20u32 {
            let t = m.del(&key4(i * 13 + 1), HM_BINARY);
            assert_eq!(t, 0, "keyoffset={keyoffset}: must not match anything");
        }
        m.keyoffset = 0;
        let after = m.snap().0;
        assert_eq!(after.length, before.length, "keyoffset={keyoffset}: nothing must be removed");
        assert_eq!(after.used_count, before.used_count);
        m.free();
    }
}

/// ERRORS row 34: `strkey` at the `int` boundaries.
#[test]
fn err34_strkey_boundaries() {
    let _g = lock();
    let (c, r) = libs();
    for n in [0i32, 1, -1, i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1, 9, 11, -2147483647] {
        unsafe {
            let cb = cstr_bytes((c.strkey)(n));
            let rb = cstr_bytes((r.strkey)(n));
            assert_eq!(cb, rb, "strkey({n})");
            assert_eq!(cb, format!("test_{n}").into_bytes());
        }
    }
}

// ---------------------------------------------------------------------------
// Abort-parity rows (ERRORS rows 20-21, 25-26)
// ---------------------------------------------------------------------------

/// ERRORS row 25: `intput(9)` and `intput(11)` trip
/// `STBDS_ASSERT(hmget(intmap, num) == 7)` because key 9 / key 11 was
/// overwritten by a later `hmput`. Both libraries must die identically.
#[test]
fn err25_intput_aborts_on_9_and_11() {
    assert_abort_parity("intput_9");
    assert_abort_parity("intput_11");
    // and confirm they really do abort rather than both silently succeeding
    let (code, sig) = run_child("intput_9", "c");
    assert_eq!(sig, Some(6), "C intput(9) should SIGABRT, got code={code:?} sig={sig:?}");
    let (code, sig) = run_child("intput_9", "rust");
    assert_eq!(sig, Some(6), "Rust intput(9) should SIGABRT, got code={code:?} sig={sig:?}");
}

/// ERRORS row 26: `intput(7)` (7 is a *value*, not a key) must exit cleanly.
#[test]
fn err26_intput_ok_on_7() {
    assert_abort_parity("intput_7");
    let (code, sig) = run_child("intput_7", "c");
    assert_eq!((code, sig), (Some(0), None), "C intput(7) should exit 0");
    let (code, sig) = run_child("intput_7", "rust");
    assert_eq!((code, sig), (Some(0), None), "Rust intput(7) should exit 0");
}

/// ERRORS rows 20-21: deleting a non-last element with `mode >= 2`.
/// `hmdel_key` hashes/compares as a string (`mode >= STBDS_HM_STRING`) but its
/// post-`memmove` slot re-find takes the `mode != STBDS_HM_STRING` branch and
/// passes the raw element bytes instead of the stored `char *`, so the re-find
/// fails and `STBDS_ASSERT(slot >= 0)` (or the following
/// `STBDS_ASSERT(b->index[i] == final_index)`) fires.
#[test]
fn err20_21_hmdel_mode2_abort() {
    assert_abort_parity("del_mode2_nonlast");
    let (code, sig) = run_child("del_mode2_nonlast", "c");
    assert_eq!(sig, Some(6), "C should SIGABRT, got code={code:?} sig={sig:?}");
    let (code, sig) = run_child("del_mode2_nonlast", "rust");
    assert_eq!(sig, Some(6), "Rust should SIGABRT, got code={code:?} sig={sig:?}");
}

/// Child-process driver. A no-op unless `DIFFTEST_CASE` is set, so it is inert
/// during a normal `cargo test` run.
#[test]
fn abort_child() {
    let case = match std::env::var(CASE_ENV) {
        Ok(c) => c,
        Err(_) => return,
    };
    let l = child_lib();
    unsafe {
        (l.rand_seed)(PIN);
        match case.as_str() {
            "intput_7" => (l.intput)(7),
            "intput_9" => (l.intput)(9),
            "intput_11" => (l.intput)(11),
            "del_mode2_nonlast" => {
                // SH_DEFAULT table (stores the caller's char*), 6 keys, then
                // delete a NON-last one with mode = 2.
                let elemsize = 24usize;
                let mut t = (l.shmode_func)(elemsize, SH_DEFAULT);
                let keys: Vec<Vec<u8>> = (0..6)
                    .map(|i| format!("abort_key_{i}\0").into_bytes())
                    .collect();
                let mut keep: Vec<Box<[u8]>> = Vec::new();
                for k in &keys {
                    let b: Box<[u8]> = k.clone().into_boxed_slice();
                    let p = b.as_ptr() as *mut c_void;
                    keep.push(b);
                    t = (l.hmput_key)(t, elemsize, p, 8, HM_STRING);
                }
                let mut victim = keys[0].clone();
                t = (l.hmdel_key)(
                    t, elemsize, victim.as_mut_ptr() as *mut c_void, 8, 0, 2);
                // if we somehow get here, report a distinct clean exit so the
                // parent's comparison still catches a divergence
                println!("survived, t={:?}", t);
                std::process::exit(0);
            }
            other => panic!("unknown case {other}"),
        }
    }
    // reached only by the non-aborting cases
    std::process::exit(0);
}
