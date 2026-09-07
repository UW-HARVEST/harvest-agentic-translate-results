//! Phase C: one differential test per row of `ERRORS.md`, plus the generic
//! boundaries (NULL pointers, zero/oversized lengths, out-of-range enum values
//! one step past the documented range).
mod common;
use common::*;
use std::ffi::{c_char, c_void};

unsafe fn hdr_of(a: *mut c_void) -> ArrayHeader {
    *((a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)
}

// ===========================================================================
// rows 1-5: stbds_arrgrowf rejection / clamping
// ===========================================================================

// row 1: request already satisfied => identical pointer, nothing reallocated
#[test]
fn e01_arrgrowf_request_fits_returns_same_pointer() {
    let _s = session(0x31415926);
    let l = libs();
    unsafe {
        for &elemsize in &[1usize, 4, 8, 16] {
            let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 32);
            let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 32);
            for &min_cap in &[0usize, 1, 31, 32] {
                assert_eq!((l.c.arrgrowf)(ca, elemsize, 0, min_cap), ca);
                assert_eq!((l.r.arrgrowf)(ra, elemsize, 0, min_cap), ra);
            }
            assert_eq!(hdr_of(ca).capacity, hdr_of(ra).capacity);
            assert_eq!(hdr_of(ca).capacity, 32);
            (l.c.arrfreef)(ca);
            (l.r.arrfreef)(ra);
        }
    }
}

// row 2: a == NULL initialises length / hash_table / temp
#[test]
fn e02_arrgrowf_from_null_initialises_header() {
    let _s = session(0x31415926);
    let l = libs();
    unsafe {
        for &elemsize in &[1usize, 8, 40] {
            let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 7);
            let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 7);
            let (h, g) = (hdr_of(ca), hdr_of(ra));
            assert_eq!((h.length, h.capacity, h.temp), (0, 7, 0));
            assert!(h.hash_table.is_null());
            assert_eq!((g.length, g.capacity, g.temp), (0, 7, 0));
            assert!(g.hash_table.is_null());
            (l.c.arrfreef)(ca);
            (l.r.arrfreef)(ra);
        }
    }
}

// rows 3-4: `min_cap < 4` clamp and the `2*arrcap` doubling floor
#[test]
fn e03_e04_arrgrowf_clamps() {
    let _s = session(0x31415926);
    let l = libs();
    unsafe {
        // from NULL: min_cap 1,2,3 -> 4
        for &min_cap in &[1usize, 2, 3] {
            let ca = (l.c.arrgrowf)(std::ptr::null_mut(), 8, 0, min_cap);
            let ra = (l.r.arrgrowf)(std::ptr::null_mut(), 8, 0, min_cap);
            assert_eq!(hdr_of(ca).capacity, hdr_of(ra).capacity);
            assert_eq!(hdr_of(ca).capacity, 4, "min_cap {min_cap} must clamp to 4");
            (l.c.arrfreef)(ca);
            (l.r.arrfreef)(ra);
        }
        // from cap=10: asking for 11 gives 20 (doubling floor), 21 gives 21
        let mut ca = (l.c.arrgrowf)(std::ptr::null_mut(), 8, 0, 10);
        let mut ra = (l.r.arrgrowf)(std::ptr::null_mut(), 8, 0, 10);
        ca = (l.c.arrgrowf)(ca, 8, 0, 11);
        ra = (l.r.arrgrowf)(ra, 8, 0, 11);
        assert_eq!(hdr_of(ca).capacity, hdr_of(ra).capacity);
        assert_eq!(hdr_of(ca).capacity, 20);
        ca = (l.c.arrgrowf)(ca, 8, 0, 41);
        ra = (l.r.arrgrowf)(ra, 8, 0, 41);
        assert_eq!(hdr_of(ca).capacity, hdr_of(ra).capacity);
        assert_eq!(hdr_of(ca).capacity, 41);
        (l.c.arrfreef)(ca);
        (l.r.arrfreef)(ra);
    }
}

// row 5: addlen == 0 && min_cap == 0 on a NULL array returns NULL in both
#[test]
fn e05_arrgrowf_zero_request_from_null_returns_null() {
    let _s = session(0x31415926);
    let l = libs();
    unsafe {
        for &elemsize in &[0usize, 1, 8, 4096] {
            let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(ca.is_null(), "C must return NULL (elemsize={elemsize})");
            assert!(ra.is_null(), "Rust must return NULL (elemsize={elemsize})");
        }
        // and on a non-NULL array it is a no-op
        let ca = (l.c.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        let ra = (l.r.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        assert_eq!((l.c.arrgrowf)(ca, 8, 0, 0), ca);
        assert_eq!((l.r.arrgrowf)(ra, 8, 0, 0), ra);
        (l.c.arrfreef)(ca);
        (l.r.arrfreef)(ra);
    }
    // elemsize == 0 is the degenerate boundary: header-only allocation
    let l = libs();
    unsafe {
        let ca = (l.c.arrgrowf)(std::ptr::null_mut(), 0, 0, 1);
        let ra = (l.r.arrgrowf)(std::ptr::null_mut(), 0, 0, 1);
        let (h, g) = (hdr_of(ca), hdr_of(ra));
        assert_eq!((h.length, h.capacity, h.temp), (g.length, g.capacity, g.temp));
        assert_eq!(h.capacity, 4);
        (l.c.arrfreef)(ca);
        (l.r.arrfreef)(ra);
    }
}

// ===========================================================================
// rows 6-7: stbds_hmfree_func
// ===========================================================================

// row 6: a == NULL must be an immediate, harmless return
#[test]
fn e06_hmfree_null() {
    let _s = session(0x31415926);
    let l = libs();
    unsafe {
        for &elemsize in &[0usize, 1, 8, 16, 1_000_000] {
            (l.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (l.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

// row 7: hash_table == NULL skips the strdup loop and strreset
#[test]
fn e07_hmfree_without_hash_table() {
    let _s = session(0x31415926);
    for &elemsize in &[8usize, 16, 32] {
        let mut m = MapPair::null(elemsize, "e07");
        m.put_default();
        assert!(m.snap_c().table.is_none());
        assert!(m.snap_r().table.is_none());
        m.assert_same("tableless map");
        m.free(); // must not crash and must not touch the (absent) arena
    }
}

// ===========================================================================
// rows 8-12: lookup misses
// ===========================================================================

// row 8 + 11 + 12: absent key => -1 through both the header and the out-param
#[test]
fn e08_e11_e12_lookup_miss_returns_minus_one() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(811);
    let mut m = MapPair::null(8, "e08");
    for i in 0..64u32 {
        let mut k = i;
        m.put(&mut k as *mut u32 as *mut c_void, 4, HM_BINARY);
        unsafe { m.fill_tail(4, i as u64) };
    }
    m.assert_same("populated");
    for _ in 0..1000 {
        let k = 1000 + rng.next_u64() as u32;
        let (a, b) = m.get(&mut { k } as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!((a, b), (-1, -1), "hmget_key miss must be -1");
        unsafe {
            assert_eq!(m.temp_c(), -1);
            assert_eq!(m.temp_r(), -1);
        }
        let (a, b) = m.get_ts(&mut { k } as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!((a, b), (-1, -1), "hmget_key_ts miss must be -1");
    }
    m.assert_same("after misses");
    m.free();
}

// row 9: a == NULL creates the map, reports -1
#[test]
fn e09_get_on_null_map() {
    let _s = session(0x31415926);
    for &elemsize in &[1usize, 4, 8, 16, 64] {
        let mut m = MapPair::null(elemsize, "e09");
        let (a, b) = m.get_ts(&mut 1u32 as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!((a, b), (-1, -1));
        assert_eq!(m.snap_c().length, 1);
        m.assert_same("created by get_ts");
        // element 0 must be zeroed
        assert!(m.snap_c().elems[0].iter().all(|&b| b == 0));
        m.free();
    }
}

// row 10: table == NULL => -1
#[test]
fn e10_get_on_tableless_map() {
    let _s = session(0x31415926);
    let mut m = MapPair::null(16, "e10");
    m.put_default();
    for k in 0..64u32 {
        let (a, b) = m.get_ts(&mut { k } as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!((a, b), (-1, -1));
        let (a, b) = m.get(&mut { k } as *mut u32 as *mut c_void, 4, HM_BINARY);
        assert_eq!((a, b), (-1, -1));
    }
    m.assert_same("tableless lookups");
    m.free();
}

// ===========================================================================
// rows 13-15: stbds_hmput_default
// ===========================================================================

#[test]
fn e13_e14_e15_hmput_default() {
    let _s = session(0x31415926);
    let l = libs();
    // row 13: from NULL
    for &elemsize in &[1usize, 8, 16] {
        let mut m = MapPair::null(elemsize, "e13");
        m.put_default();
        assert_eq!(m.snap_c().length, 1);
        m.assert_same("from NULL");
        // row 15: second call is a no-op
        let (c1, r1) = (m.c, m.r);
        m.put_default();
        assert_eq!(m.c, c1);
        assert_eq!(m.r, r1);
        m.assert_same("second call");
        m.free();
    }
    // row 14: length == 0 (an array grown by arrgrowf but never filled)
    unsafe {
        let elemsize = 16usize;
        let ca = (l.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        let ra = (l.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
        assert_eq!(hdr_of(ca).length, 0);
        let cm = (l.c.hmput_default)((ca as *mut u8).add(elemsize) as *mut c_void, elemsize);
        let rm = (l.r.hmput_default)((ra as *mut u8).add(elemsize) as *mut c_void, elemsize);
        assert_eq!(snap_map(cm, elemsize), snap_map(rm, elemsize));
        assert_eq!(snap_map(cm, elemsize).length, 1);
        (l.c.hmfree_func)((cm as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        (l.r.hmfree_func)((rm as *mut u8).sub(elemsize) as *mut c_void, elemsize);
    }
}

// ===========================================================================
// rows 16-19: stbds_hmdel_key rejections
// ===========================================================================

// row 16: a == NULL returns NULL
#[test]
fn e16_hmdel_null_map() {
    let _s = session(0x31415926);
    let l = libs();
    unsafe {
        let mut key = 1u32;
        for &mode in &[-1i32, 0, 1, 2, 99, i32::MAX, i32::MIN] {
            for &elemsize in &[1usize, 8, 16] {
                let c = (l.c.hmdel_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    &mut key as *mut u32 as *mut c_void,
                    4,
                    0,
                    mode,
                );
                let r = (l.r.hmdel_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    &mut key as *mut u32 as *mut c_void,
                    4,
                    0,
                    mode,
                );
                assert!(c.is_null(), "C must return NULL (mode={mode})");
                assert!(r.is_null(), "Rust must return NULL (mode={mode})");
            }
        }
    }
}

// row 17: hash_table == NULL => temp = 0, map untouched
#[test]
fn e17_hmdel_tableless_map() {
    let _s = session(0x31415926);
    let mut m = MapPair::null(16, "e17");
    m.put_default();
    let (c1, r1) = (m.c, m.r);
    for k in 0..40u32 {
        m.del(&mut { k } as *mut u32 as *mut c_void, 4, 0, HM_BINARY);
        assert_eq!(m.c, c1);
        assert_eq!(m.r, r1);
        unsafe {
            assert_eq!(m.temp_c(), 0, "temp must be 0");
            assert_eq!(m.temp_r(), 0);
        }
        assert_eq!(m.snap_c().length, 1, "length unchanged");
        m.assert_same("tableless delete");
    }
    m.free();
}

// row 18: absent key => temp = 0, length unchanged
#[test]
fn e18_hmdel_absent_key() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(18);
    for &shmode in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let mut m = MapPair::shmode(16, shmode, "e18");
        let string_mode = shmode != SH_NONE;
        for i in 0..40u64 {
            if string_mode {
                let n = 1 + rng.below(20) as usize;
                let k = rng.cstring(n);
                let kp = m.own(&k);
                m.put(kp, 8, HM_STRING);
            } else {
                let k = rng.bytes(8);
                let kp = m.own(&k);
                m.put(kp, 8, HM_BINARY);
            }
            unsafe { m.fill_tail(8, i) };
        }
        let len_before = m.snap_c().length;
        for _ in 0..200 {
            if string_mode {
                let n = 1 + rng.below(20) as usize;
                let mut k = rng.cstring(n);
                k.insert(0, b'#');
                let kp = m.own(&k);
                m.del(kp, 8, 0, HM_STRING);
            } else {
                let mut k = rng.bytes(8);
                k[7] = 0xee;
                let kp = m.own(&k);
                m.del(kp, 8, 0, HM_BINARY);
            }
            unsafe {
                assert_eq!(m.temp_c(), 0);
                assert_eq!(m.temp_r(), 0);
            }
            assert_eq!(m.snap_c().length, len_before);
            m.assert_same("absent delete");
        }
        m.free();
    }
}

// row 19: successful delete => temp = 1, tombstone written, length shrinks
#[test]
fn e19_hmdel_success_marks_tombstone() {
    let _s = session(0x31415926);
    let mut m = MapPair::null(8, "e19");
    for i in 0..40u32 {
        let mut k = i;
        m.put(&mut k as *mut u32 as *mut c_void, 4, HM_BINARY);
        unsafe { m.fill_tail(4, i as u64) };
    }
    for i in (0..40u32).rev() {
        let before = m.snap_c().length;
        m.del(&mut { i } as *mut u32 as *mut c_void, 4, 0, HM_BINARY);
        unsafe {
            assert_eq!(m.temp_c(), 1);
            assert_eq!(m.temp_r(), 1);
        }
        assert_eq!(m.snap_c().length, before - 1);
        let t = m.snap_c().table.unwrap();
        // a tombstone is hash == 1 / index == -2
        assert!(t.buckets.iter().any(|&(h, ix)| h == 1 && ix == -2) || t.tombstone_count == 0);
        m.assert_same("delete step");
    }
    m.free();
}

// ===========================================================================
// row 20: hash < 2 is bumped to hash + 2
// ===========================================================================

// `stbds_hash_string("", seed) == K + seed` for a constant K (the `hash ^= seed`
// zeroes out the seed contribution), so a seed can be chosen that makes the raw
// hash exactly 0 or exactly 1 -- which is the only way to reach the `hash < 2`
// branch on purpose.
#[test]
fn e20_hash_below_two_is_bumped() {
    let _s = session(0x31415926);
    let l = libs();
    let empty = b"\0";
    unsafe {
        let k0 = (l.c.hash_string)(empty.as_ptr() as *mut c_char, 0);
        let k0r = (l.r.hash_string)(empty.as_ptr() as *mut c_char, 0);
        assert_eq!(k0, k0r);
        for target in 0usize..2 {
            let seed = target.wrapping_sub(k0);
            let hc = (l.c.hash_string)(empty.as_ptr() as *mut c_char, seed);
            let hr = (l.r.hash_string)(empty.as_ptr() as *mut c_char, seed);
            assert_eq!(hc, hr);
            assert_eq!(hc, target, "seed construction must yield hash == {target}");

            // now drive a real map with that seed and check the stored bucket hash
            seed_both(seed);
            let mut m = MapPair::null(16, "e20");
            let kp = m.own(empty);
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, 0) };
            m.assert_same(&format!("empty-string key with raw hash {target}"));
            let t = m.snap_c().table.unwrap();
            assert_eq!(t.seed, seed, "table must use the seed we set");
            let stored: Vec<usize> = t
                .buckets
                .iter()
                .filter(|&&(_, ix)| ix >= 0)
                .map(|&(h, _)| h)
                .collect();
            assert_eq!(stored, vec![target + 2], "hash < 2 must be bumped by 2");
            m.free();
        }
    }
}

// ===========================================================================
// rows 21-22: `mode` classification, including out-of-range enum values
// ===========================================================================

// row 21: every mode >= 1 takes the strcmp path
#[test]
fn e21_modes_ge_one_use_strcmp() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(21);
    for &mode in &[1i32, 2, 3, 4, 5, 99, 1 << 20, i32::MAX] {
        let mut m = MapPair::shmode(16, SH_STRDUP, "e21");
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < 40 {
            let n = 1 + rng.below(18) as usize;
            let k = rng.cstring(n);
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let kp = m.own(k);
            m.put(kp, 8, mode);
            unsafe { m.fill_tail(8, i as u64) };
            m.assert_same(&format!("mode={mode} put {i}"));
        }
        // a *different pointer* holding the same text must still be found:
        // proof that strcmp (not pointer/memcmp) was used
        for k in &keys {
            let copy = k.clone();
            let kp = m.own(&copy);
            let (a, b) = m.get(kp, 8, mode);
            assert_eq!(a, b);
            assert!(a >= 0, "mode={mode} strcmp lookup must succeed");
        }
        m.assert_same("after strcmp lookups");
        m.free();
    }
}

// row 22: mode 0 and every negative mode take the memcmp path
#[test]
fn e22_modes_lt_one_use_memcmp() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(22);
    for &mode in &[0i32, -1, -2, -99, i32::MIN] {
        let mut m = MapPair::null(16, "e22");
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < 40 {
            let k = rng.bytes(8);
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let kp = m.own(k);
            m.put(kp, 8, mode);
            unsafe { m.fill_tail(8, i as u64) };
            m.assert_same(&format!("mode={mode} put {i}"));
        }
        assert_eq!(
            m.snap_c().table.unwrap().arena_mode,
            0,
            "mode {mode} must be treated as binary"
        );
        for k in &keys {
            let copy = k.clone();
            let kp = m.own(&copy);
            let (a, b) = m.get(kp, 8, mode);
            assert_eq!(a, b);
            assert!(a >= 0, "mode={mode} memcmp lookup must succeed");
        }
        m.assert_same("after memcmp lookups");
        m.free();
    }
    // keysize == 0 boundary: memcmp of 0 bytes always compares equal, so every
    // key collapses onto the first entry.
    let mut m = MapPair::null(8, "e22/keysize0");
    for i in 0..20u32 {
        let mut k = i;
        m.put(&mut k as *mut u32 as *mut c_void, 0, HM_BINARY);
        unsafe { m.fill_tail(0, i as u64) };
        m.assert_same(&format!("keysize=0 put {i}"));
    }
    assert_eq!(m.snap_c().length, 2, "all zero-length keys are the same key");
    m.free();
}

// ===========================================================================
// rows 23-24: the string.mode a fresh table gets from hmput_key
// ===========================================================================

#[test]
fn e23_e24_fresh_table_string_mode() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(2324);
    for &mode in &[1i32, 2, 99, i32::MAX] {
        let mut m = MapPair::null(16, "e23");
        let n = 1 + rng.below(10) as usize;
        let k = rng.cstring(n);
        let kp = m.own(&k);
        m.put(kp, 8, mode);
        unsafe { m.fill_tail(8, 0) };
        m.assert_same(&format!("mode={mode}"));
        assert_eq!(m.snap_c().table.unwrap().arena_mode, SH_DEFAULT as u8);
        m.free();
    }
    for &mode in &[0i32, -1, i32::MIN] {
        let mut m = MapPair::null(16, "e24");
        let k = rng.bytes(8);
        let kp = m.own(&k);
        m.put(kp, 8, mode);
        unsafe { m.fill_tail(8, 0) };
        m.assert_same(&format!("mode={mode}"));
        assert_eq!(m.snap_c().table.unwrap().arena_mode, 0);
        m.free();
    }
}

// ===========================================================================
// row 25: hmdel_key frees the strdup'd key only when mode == 1 exactly
// ===========================================================================

// Only tail deletes are exercised: for a non-tail delete with `mode != 1` the C
// re-lookup hashes the raw pointer bytes and then trips
// `STBDS_ASSERT(slot >= 0)`, i.e. the C aborts (see ERRORS.md row 25 note).
#[test]
fn e25_hmdel_mode_exactly_one_frees_strdup_key() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(25);
    for &mode in &[1i32, 2, 3, 99] {
        for &n in &[1usize, 5, 20] {
            let mut m = MapPair::shmode(16, SH_STRDUP, "e25");
            let mut keys: Vec<Vec<u8>> = Vec::new();
            while keys.len() < n {
                let sz = 1 + rng.below(18) as usize;
                let k = rng.cstring(sz);
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
            for (i, k) in keys.iter().enumerate() {
                let kp = m.own(k);
                m.put(kp, 8, HM_STRING);
                unsafe { m.fill_tail(8, i as u64) };
            }
            m.assert_same("populated");
            for i in (0..keys.len()).rev() {
                let kp = m.own(&keys[i]);
                m.del(kp, 8, 0, mode);
                unsafe {
                    assert_eq!(m.temp_c(), 1, "mode={mode} tail delete must succeed");
                    assert_eq!(m.temp_r(), 1);
                }
                m.assert_same(&format!("mode={mode} n={n} tail delete {i}"));
            }
            m.free();
        }
    }
}

// ===========================================================================
// rows 26-27: hmput_key's `default:` memcpy arm, and shmode truncation
// ===========================================================================

#[test]
fn e26_e27_shmode_truncation_selects_switch_arm() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(2627);
    // every distinct (unsigned char) mode value that matters
    let modes: Vec<i32> = vec![
        0, 1, 2, 3, 4, 5, 6, 127, 128, 200, 254, 255, 256, 257, 258, 259, 260, 511, 512, -1, -2,
        -3, -4, -255, -256, i32::MAX, i32::MIN,
    ];
    for &mode in &modes {
        let mut m = MapPair::shmode(16, mode, "e26");
        let truncated = (mode as u32 & 0xff) as u8;
        assert_eq!(
            m.snap_c().table.unwrap().arena_mode,
            truncated,
            "mode {mode} must truncate to {truncated}"
        );
        assert_eq!(m.snap_r().table.unwrap().arena_mode, truncated);
        m.assert_same(&format!("fresh shmode={mode}"));
        // the memcpy arm stores raw bytes and a later strcmp would read them as a
        // pointer (UB in the C too), so keep those maps tiny
        let n = if matches!(truncated, 1 | 2 | 3) { 30 } else { 3 };
        for i in 0..n {
            let sz = 1 + rng.below(20) as usize;
            let k = rng.cstring(sz);
            let kp = m.own(&k);
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, i as u64) };
            m.assert_same(&format!("shmode={mode} put {i}"));
        }
        m.free();
    }
    // elemsize boundary for shmode_func
    for &elemsize in &[0usize, 1, 8, 4096] {
        let l = libs();
        unsafe {
            let c = (l.c.shmode_func)(elemsize, SH_ARENA);
            let r = (l.r.shmode_func)(elemsize, SH_ARENA);
            assert_eq!(snap_map(c, elemsize), snap_map(r, elemsize), "elemsize={elemsize}");
            (l.c.hmfree_func)((c as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (l.r.hmfree_func)((r as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// rows 28-32: hash function boundaries
// ===========================================================================

// row 28
#[test]
fn e28_hash_string_empty() {
    let _s = session(0x31415926);
    let l = libs();
    let e = b"\0";
    unsafe {
        let mut rng = Rng::with(28);
        for _ in 0..1000 {
            let seed = rng.next_u64() as usize;
            assert_eq!(
                (l.c.hash_string)(e.as_ptr() as *mut c_char, seed),
                (l.r.hash_string)(e.as_ptr() as *mut c_char, seed),
                "hash_string(\"\", {seed:#x})"
            );
        }
        for &seed in &[0usize, 1, usize::MAX, 0x31415926] {
            assert_eq!(
                (l.c.hash_string)(e.as_ptr() as *mut c_char, seed),
                (l.r.hash_string)(e.as_ptr() as *mut c_char, seed)
            );
        }
    }
}

// rows 29-32
#[test]
fn e29_e32_hash_bytes_boundaries() {
    let _s = session(0x31415926);
    let l = libs();
    let mut rng = Rng::with(2932);
    unsafe {
        // row 29: len == 0 (NULL data pointer is never dereferenced in that case)
        for &seed in &[0usize, 1, usize::MAX, 0x31415926] {
            let a = (l.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let b = (l.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(a, b, "hash_bytes(NULL, 0, {seed:#x})");
        }
        // rows 30-32: tail lengths and the int sign-extension positions
        for len in 0..40usize {
            for _ in 0..30 {
                let mut v = rng.bytes(len);
                // deliberately set / clear the high bit of every byte 3 and 7
                for (i, b) in v.iter_mut().enumerate() {
                    match (i % 8, rng.next_u64() & 1) {
                        (3, 0) | (7, 0) => *b |= 0x80,
                        (3, _) | (7, _) => *b &= 0x7f,
                        _ => {}
                    }
                }
                let seed = rng.next_u64() as usize;
                let mut c = v.clone();
                let mut r = v.clone();
                assert_eq!(
                    (l.c.hash_bytes)(c.as_mut_ptr() as *mut c_void, len, seed),
                    (l.r.hash_bytes)(r.as_mut_ptr() as *mut c_void, len, seed),
                    "len={len} seed={seed:#x} bytes={v:02x?}"
                );
            }
        }
        // extreme lengths
        for &len in &[1usize, 7, 8, 9, 15, 16, 17, 4095, 4096, 65537] {
            let mut v = vec![0xffu8; len];
            let mut w = v.clone();
            assert_eq!(
                (l.c.hash_bytes)(v.as_mut_ptr() as *mut c_void, len, 0),
                (l.r.hash_bytes)(w.as_mut_ptr() as *mut c_void, len, 0),
                "len={len} all-0xff"
            );
        }
    }
}

// ===========================================================================
// rows 33-37, 39: string arena boundaries
// ===========================================================================

#[test]
fn e33_e39_arena_boundaries() {
    let _s = session(0x31415926);
    let l = libs();
    unsafe {
        // row 39: strreset on an all-zero arena, twice
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        (l.c.strreset)(&mut ca);
        (l.r.strreset)(&mut ra);
        (l.c.strreset)(&mut ca);
        (l.r.strreset)(&mut ra);
        assert_eq!(
            (ca.remaining, ca.block, ca.mode, ca.storage.is_null()),
            (ra.remaining, ra.block, ra.mode, ra.storage.is_null())
        );
        assert!(ca.storage.is_null() && ra.storage.is_null());

        // rows 33-36: the three placement paths and the block counter
        for &(len, preset_block) in &[
            (1usize, 0u8),
            (10, 0),
            (511, 0),
            (512, 0),  // len(513 incl NUL) > 512 -> dedicated
            (513, 0),
            (5000, 0),
            (10, 21),
            (10, 22),  // 512<<11 == 1<<20 -> counter saturates
            (10, 23),
            (10, 24),
            (2_000_000, 22),
        ] {
            let mut cav = StringArena::zeroed();
            let mut rav = StringArena::zeroed();
            cav.block = preset_block;
            rav.block = preset_block;
            let mut s = vec![b'z'; len];
            s.push(0);
            let mut sc = s.clone();
            let mut sr = s.clone();
            let cp = (l.c.stralloc)(&mut cav, sc.as_mut_ptr() as *mut c_char);
            let rp = (l.r.stralloc)(&mut rav, sr.as_mut_ptr() as *mut c_char);
            assert_eq!(
                (cav.remaining, cav.block, cav.mode),
                (rav.remaining, rav.block, rav.mode),
                "len={len} block={preset_block}"
            );
            let cs = std::slice::from_raw_parts(cp as *const u8, len);
            let rs = std::slice::from_raw_parts(rp as *const u8, len);
            assert_eq!(cs, &s[..len]);
            assert_eq!(rs, &s[..len]);
            // is the returned pointer the head block's storage?
            let c_is_head = cp as *const u8 == std::ptr::addr_of!((*cav.storage).storage) as *const u8;
            let r_is_head = rp as *const u8 == std::ptr::addr_of!((*rav.storage).storage) as *const u8;
            assert_eq!(c_is_head, r_is_head, "placement len={len} block={preset_block}");
            (l.c.strreset)(&mut cav);
            (l.r.strreset)(&mut rav);
        }
    }
}

// ===========================================================================
// rows 47, 55: driver boundaries
// ===========================================================================

// rows 47 + 55 share one test function because `capture_stdout` redirects the
// process-wide fd 1 (see phase_b_strput.rs).
#[test]
fn e47_e55_driver_boundaries() {
    let _s = session(0x31415926);
    let l = libs();
    // row 55: strkey with extreme ints
    unsafe {
        for &n in &[0i32, 1, -1, i32::MAX, i32::MIN, -2_147_483_647] {
            let c = std::ffi::CStr::from_ptr((l.c.strkey)(n)).to_bytes().to_vec();
            let r = std::ffi::CStr::from_ptr((l.r.strkey)(n)).to_bytes().to_vec();
            assert_eq!(c, r, "strkey({n})");
            assert_eq!(c, format!("test_{n}").into_bytes());
            assert!(c.len() < 256, "must fit the 256-byte static buffer");
        }
    }
    // row 47: str_put with num <= 0 skips the stralloc loop entirely
    for &n in &[0i32, -1, -2, -1000, i32::MIN] {
        seed_both(0x31415926);
        let c = unsafe { capture_stdout("ec", || (l.c.str_put)(n)) };
        seed_both(0x31415926);
        let r = unsafe { capture_stdout("er", || (l.r.str_put)(n)) };
        assert_eq!(c, r, "str_put({n}) stdout");
        assert_eq!(c, format!("a {n}\n").into_bytes());
    }
}

// ===========================================================================
// rows 49-52: table resize / tombstone policy
// ===========================================================================

// rows 49 + 50: shrink and same-size rebuild
#[test]
fn e49_e50_shrink_and_rebuild() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(4950);
    let mut m = MapPair::null(8, "e49");
    // grow well past 8 slots
    let mut keys: Vec<u32> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    while keys.len() < 500 {
        let k = rng.next_u64() as u32;
        if seen.insert(k) {
            keys.push(k);
        }
    }
    for (i, &k) in keys.iter().enumerate() {
        let mut kk = k;
        m.put(&mut kk as *mut u32 as *mut c_void, 4, HM_BINARY);
        unsafe { m.fill_tail(4, i as u64) };
    }
    let big = m.snap_c().table.unwrap().slot_count;
    assert!(big >= 512, "table must have grown (slot_count={big})");
    m.assert_same("grown");

    // deleting everything walks the shrink chain all the way back to 8
    let mut shrinks = 0;
    let mut rebuilds = 0;
    let mut prev = big;
    for (step, &k) in keys.iter().enumerate() {
        let before_tomb = m.snap_c().table.unwrap().tombstone_count;
        m.del(&mut { k } as *mut u32 as *mut c_void, 4, 0, HM_BINARY);
        m.assert_same(&format!("delete step {step}"));
        let t = m.snap_c().table.unwrap();
        if t.slot_count < prev {
            shrinks += 1;
            prev = t.slot_count;
        } else if t.tombstone_count == 0 && before_tomb > 0 {
            rebuilds += 1;
        }
    }
    assert!(shrinks >= 5, "shrink path must have been taken (got {shrinks})");
    assert!(rebuilds >= 1, "rebuild path must have been taken (got {rebuilds})");
    assert_eq!(m.snap_c().table.unwrap().slot_count, 8);
    m.free();
}

// row 51: grow at used_count >= used_count_threshold
#[test]
fn e51_grow_threshold_exact() {
    let _s = session(0x31415926);
    let mut m = MapPair::null(8, "e51");
    let mut last = 0usize;
    let mut growths = Vec::new();
    for i in 0..2000u32 {
        let mut k = i;
        m.put(&mut k as *mut u32 as *mut c_void, 4, HM_BINARY);
        unsafe { m.fill_tail(4, i as u64) };
        let t = m.snap_c().table.unwrap();
        if t.slot_count != last {
            growths.push((i, t.slot_count, t.used_count_threshold));
            last = t.slot_count;
        }
        m.assert_same(&format!("insert {i}"));
    }
    assert_eq!(
        growths
            .iter()
            .map(|&(_, sc, _)| sc)
            .collect::<Vec<_>>(),
        vec![8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096],
        "growth sequence"
    );
    m.free();
}

// row 52: an insert that lands on a tombstone reuses it
#[test]
fn e52_insert_reuses_tombstone() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(52);
    let mut m = MapPair::null(8, "e52");
    let mut live: Vec<u32> = Vec::new();
    for i in 0..300u32 {
        let mut k = i;
        m.put(&mut k as *mut u32 as *mut c_void, 4, HM_BINARY);
        unsafe { m.fill_tail(4, i as u64) };
        live.push(i);
    }
    let mut reuses = 0;
    for step in 0..1500u32 {
        // delete one, insert a fresh one: tombstone_count goes up then down
        let i = rng.below(live.len() as u64) as usize;
        let victim = live.remove(i);
        m.del(&mut { victim } as *mut u32 as *mut c_void, 4, 0, HM_BINARY);
        let after_del = m.snap_c().table.unwrap().tombstone_count;
        let mut nk = 100_000 + step;
        m.put(&mut nk as *mut u32 as *mut c_void, 4, HM_BINARY);
        unsafe { m.fill_tail(4, step as u64) };
        live.push(nk);
        let after_put = m.snap_c().table.unwrap();
        if after_put.tombstone_count < after_del {
            reuses += 1;
        }
        m.assert_same(&format!("churn step {step}"));
    }
    assert!(reuses > 0, "the tombstone-reuse branch must have been taken");
    m.free();
}
