//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input / rejection condition, calls
//! BOTH `.so`s, and asserts they return the SAME error code or sentinel.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void, CStr};

fn hdr(arr: *mut c_void) -> ArrayHeader {
    unsafe { *(arr as *mut ArrayHeader).offset(-1) }
}
fn temp_of(hash_ptr: *mut c_void, es: usize) -> isize {
    unsafe { (*((hash_ptr as *mut u8).sub(es) as *mut ArrayHeader).offset(-1)).temp }
}

// =========================================================================
// stbds_arrgrowf
// =========================================================================

/// ERRORS row 1: capacity already sufficient -> the *same* pointer, untouched.
#[test]
fn err01_arrgrowf_returns_same_pointer() {
    let (c, r) = libs();
    for &es in &[1usize, 8, 24] {
        unsafe {
            let a = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 16);
            let b = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 16);
            for &(addlen, min_cap) in &[(0usize, 0usize), (0, 1), (0, 16), (5, 0), (16, 0), (16, 16)] {
                let a2 = (c.arrgrowf)(a, es, addlen, min_cap);
                let b2 = (r.arrgrowf)(b, es, addlen, min_cap);
                let ctx = format!("err01 es={es} addlen={addlen} min_cap={min_cap}");
                assert_eq!(a2, a, "{ctx}: C pointer changed");
                assert_eq!(b2, b, "{ctx}: Rust pointer changed");
                assert_eq!(hdr(a2).capacity, 16, "{ctx}");
                assert_eq!(hdr(b2).capacity, 16, "{ctx}");
                assert_eq!(hdr(a2).length, hdr(b2).length, "{ctx}");
            }
            (c.arrfreef)(a);
            (r.arrfreef)(b);
        }
    }
}

/// ERRORS row 2: `a == NULL, addlen == 0, min_cap == 0` -> NULL, no allocation.
#[test]
fn err02_arrgrowf_null_when_nothing_requested() {
    let (c, r) = libs();
    for &es in &[0usize, 1, 8, 16, 24, 1024] {
        unsafe {
            let a = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let b = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(a.is_null(), "err02 es={es}: C must return NULL");
            assert!(b.is_null(), "err02 es={es}: Rust must return NULL");
        }
    }
}

/// ERRORS row 3: 1..=3 requested from nothing -> capacity is bumped to 4.
#[test]
fn err03_arrgrowf_minimum_capacity_four() {
    let (c, r) = libs();
    for &es in &[1usize, 8, 24] {
        for &(addlen, min_cap) in &[(1usize, 0usize), (2, 0), (3, 0), (0, 1), (0, 2), (0, 3), (1, 3), (3, 1)] {
            unsafe {
                let a = (c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                let b = (r.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                let ctx = format!("err03 es={es} addlen={addlen} min_cap={min_cap}");
                assert_eq!(hdr(a).capacity, 4, "{ctx}: C");
                assert_eq!(hdr(b).capacity, 4, "{ctx}: Rust");
                assert_eq!(hdr(a).length, 0, "{ctx}");
                assert_eq!(hdr(b).length, 0, "{ctx}");
                assert!(hdr(a).hash_table.is_null() && hdr(b).hash_table.is_null(), "{ctx}");
                assert_eq!(hdr(a).temp, 0, "{ctx}");
                assert_eq!(hdr(b).temp, 0, "{ctx}");
                (c.arrfreef)(a);
                (r.arrfreef)(b);
            }
        }
    }
}

// =========================================================================
// stbds_hash_bytes / stbds_hash_string
// =========================================================================

/// ERRORS row 6: `p == NULL, len == 0` must not dereference.
#[test]
fn err06_hash_bytes_null_zero_length() {
    let (c, r) = libs();
    for &seed in &[0usize, 1, DEFAULT_SEED, usize::MAX] {
        unsafe {
            let hc = (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let hr = (r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(hc, hr, "err06 seed={seed:#x}");
        }
    }
}

/// ERRORS rows 7 + 8: the `int` sign-extension of `d[3] << 24` / `d[7] << 24`.
#[test]
fn err07_08_hash_bytes_sign_extension() {
    let (c, r) = libs();
    // one high byte at a time, at every offset, for every length 0..=24
    for len in 0usize..=24 {
        for off in 0..len.max(1) {
            let mut buf = vec![0u8; len.max(1)];
            if off < len {
                buf[off] = 0xff;
            }
            unsafe {
                let p = buf.as_mut_ptr() as *mut c_void;
                let hc = (c.hash_bytes)(p, len, DEFAULT_SEED);
                let hr = (r.hash_bytes)(p, len, DEFAULT_SEED);
                assert_eq!(hc, hr, "err07 len={len} off={off}");
            }
        }
    }
    // every possible single trailing byte with a 1..=7 byte tail
    for len in 1usize..=7 {
        for b in 0u16..=255 {
            let mut buf = vec![b as u8; len];
            unsafe {
                let p = buf.as_mut_ptr() as *mut c_void;
                assert_eq!(
                    (c.hash_bytes)(p, len, DEFAULT_SEED),
                    (r.hash_bytes)(p, len, DEFAULT_SEED),
                    "err07 len={len} byte={b:#x}"
                );
            }
        }
    }
}

/// ERRORS rows 9 + 10: empty string, and bytes >= 0x80 added as `unsigned char`.
#[test]
fn err09_10_hash_string_edges() {
    let (c, r) = libs();
    unsafe {
        for &seed in &[0usize, 1, DEFAULT_SEED, usize::MAX] {
            let mut empty = [0i8; 1];
            assert_eq!(
                (c.hash_string)(empty.as_mut_ptr(), seed),
                (r.hash_string)(empty.as_mut_ptr(), seed),
                "err09 seed={seed:#x}"
            );
            // every single byte 1..=255 as a 1-char string
            for b in 1u16..=255 {
                let mut s = [b as i8, 0];
                assert_eq!(
                    (c.hash_string)(s.as_mut_ptr(), seed),
                    (r.hash_string)(s.as_mut_ptr(), seed),
                    "err10 seed={seed:#x} byte={b:#x}"
                );
            }
            // runs of high bytes
            for n in 1usize..=40 {
                let mut s: Vec<i8> = vec![-1i8; n];
                s.push(0);
                assert_eq!(
                    (c.hash_string)(s.as_mut_ptr(), seed),
                    (r.hash_string)(s.as_mut_ptr(), seed),
                    "err10 seed={seed:#x} 0xff x{n}"
                );
            }
        }
    }
}

// =========================================================================
// stbds_hmfree_func
// =========================================================================

/// ERRORS row 12: `a == NULL` is a silent no-op (must not free or crash).
#[test]
fn err12_hmfree_null() {
    let (c, r) = libs();
    unsafe {
        for &es in &[0usize, 1, 8, 16, 1024] {
            (c.hmfree_func)(std::ptr::null_mut(), es);
            (r.hmfree_func)(std::ptr::null_mut(), es);
        }
    }
}

/// ERRORS row 13: array without a hash table -> `free(NULL)` + free(header).
#[test]
fn err13_hmfree_without_hash_table() {
    let (c, r) = libs();
    unsafe {
        for &es in &[1usize, 8, 16] {
            let a = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            let b = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            assert!(hdr(a).hash_table.is_null() && hdr(b).hash_table.is_null());
            (c.hmfree_func)(a, es);
            (r.hmfree_func)(b, es);
        }
    }
}

/// ERRORS row 14: `SH_STRDUP` frees the duplicated keys for `i` in `1..length`.
#[test]
fn err14_hmfree_strdup_frees_keys() {
    let _g = serial();
    let (c, r) = libs();
    let es = 16usize;
    for &n in &[0usize, 1, 5, 120] {
        set_seed(DEFAULT_SEED);
        let mut rng = Rng::new(0x1400 ^ n as u64);
        let mut keys = Keys::strings(&mut rng, n.max(1), 30, ASCII);
        unsafe {
            let mut tc = (c.shmode_func)(es, SH_STRDUP);
            let mut tr = (r.shmode_func)(es, SH_STRDUP);
            for i in 0..n {
                tc = (c.hmput_key)(tc, es, keys.ptr(i), 8, HM_STRING);
                tr = (r.hmput_key)(tr, es, keys.ptr(i), 8, HM_STRING);
            }
            let sc = snapshot(tc, es);
            let sr = snapshot(tr, es);
            assert_eq!(sc, sr, "err14 n={n}");
            assert_eq!(sc.arena_mode, SH_STRDUP as u8);
            // the duplicated keys are distinct allocations from the inputs
            for i in 0..n {
                let p = *((tc as *mut u8).add(es * i) as *const *const c_char);
                assert_ne!(p as usize, keys.ptr(i) as usize, "err14: SH_STRDUP must copy");
            }
            (c.hmfree_func)((tc as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((tr as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// =========================================================================
// stbds_hmget_key / stbds_hmget_key_ts
// =========================================================================

/// ERRORS rows 15/18/21: absent key -> -1 (via find_slot's HASH_EMPTY exit).
#[test]
fn err15_18_21_absent_key_is_minus_one() {
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x1501);
    let mut keys = Keys::binary(&mut rng, 600, 8);
    let mut d = Dual::new(16, 8, HM_BINARY, KeyRepr::Bytes);
    for i in 0..300 {
        let k = keys.ptr(i);
        d.put(k, "err15 put");
    }
    for i in 300..600 {
        let k = keys.ptr(i);
        assert_eq!(d.get(k, &format!("err21 absent i={i}")), -1);
        assert_eq!(d.get_ts(k, &format!("err18 absent i={i}")), -1);
    }
    d.free();
}

/// ERRORS rows 16/19: `a == NULL` bootstraps; key/keysize/mode are ignored, so a
/// NULL key is accepted.
#[test]
fn err16_19_bootstrap_from_null_ignores_key() {
    let (c, r) = libs();
    let _g = serial();
    for &es in &[1usize, 8, 16, 24] {
        for &mode in &[-1i32, 0, 1, 2, 99, i32::MIN, i32::MAX] {
            for &keysize in &[0usize, 1, 8, usize::MAX] {
                set_seed(DEFAULT_SEED);
                unsafe {
                    let mut tc: isize = 7;
                    let mut tr: isize = 7;
                    let a = (c.hmget_key_ts)(std::ptr::null_mut(), es, std::ptr::null_mut(), keysize, &mut tc, mode);
                    let b = (r.hmget_key_ts)(std::ptr::null_mut(), es, std::ptr::null_mut(), keysize, &mut tr, mode);
                    let ctx = format!("err16 es={es} mode={mode} keysize={keysize}");
                    assert_eq!(tc, -1, "{ctx}: C *temp");
                    assert_eq!(tr, -1, "{ctx}: Rust *temp");
                    assert_eq!(snapshot(a, es), snapshot(b, es), "{ctx}");
                    (c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
                    (r.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);

                    // hmget_key writes the sentinel into header->temp as well
                    let a2 = (c.hmget_key)(std::ptr::null_mut(), es, std::ptr::null_mut(), keysize, mode);
                    let b2 = (r.hmget_key)(std::ptr::null_mut(), es, std::ptr::null_mut(), keysize, mode);
                    assert_eq!(temp_of(a2, es), -1, "{ctx}: C header->temp");
                    assert_eq!(temp_of(b2, es), -1, "{ctx}: Rust header->temp");
                    assert_eq!(snapshot(a2, es), snapshot(b2, es), "{ctx}");
                    (c.hmfree_func)((a2 as *mut u8).sub(es) as *mut c_void, es);
                    (r.hmfree_func)((b2 as *mut u8).sub(es) as *mut c_void, es);
                }
            }
        }
    }
}

/// ERRORS rows 17/20: `a != NULL` but `hash_table == 0` -> -1, `a` unchanged.
#[test]
fn err17_20_no_hash_table_yields_minus_one() {
    let (c, r) = libs();
    let mut key = [0xABu8; 8];
    for &es in &[1usize, 8, 16, 24] {
        unsafe {
            let ra = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
            let rb = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
            let a = (ra as *mut u8).add(es) as *mut c_void;
            let b = (rb as *mut u8).add(es) as *mut c_void;
            let k = key.as_mut_ptr() as *mut c_void;
            let ctx = format!("err17 es={es}");

            let mut tc: isize = 5;
            let mut tr: isize = 5;
            let a2 = (c.hmget_key_ts)(a, es, k, 8, &mut tc, HM_BINARY);
            let b2 = (r.hmget_key_ts)(b, es, k, 8, &mut tr, HM_BINARY);
            assert_eq!(a2, a, "{ctx}: C must return `a` unchanged");
            assert_eq!(b2, b, "{ctx}: Rust must return `a` unchanged");
            assert_eq!(tc, -1, "{ctx}: C *temp");
            assert_eq!(tr, -1, "{ctx}: Rust *temp");

            let a3 = (c.hmget_key)(a, es, k, 8, HM_BINARY);
            let b3 = (r.hmget_key)(b, es, k, 8, HM_BINARY);
            assert_eq!(a3, a, "{ctx}");
            assert_eq!(b3, b, "{ctx}");
            assert_eq!(temp_of(a3, es), -1, "{ctx}: C header->temp");
            assert_eq!(temp_of(b3, es), -1, "{ctx}: Rust header->temp");
            assert_eq!(snapshot(a3, es), snapshot(b3, es), "{ctx}");

            (c.hmfree_func)(ra, es);
            (r.hmfree_func)(rb, es);
        }
    }
}

// =========================================================================
// stbds_hmput_default
// =========================================================================

/// ERRORS rows 22/23/24.
#[test]
fn err22_23_24_hmput_default() {
    let (c, r) = libs();
    for &es in &[1usize, 8, 16, 24] {
        unsafe {
            // row 22: from NULL
            let a = (c.hmput_default)(std::ptr::null_mut(), es);
            let b = (r.hmput_default)(std::ptr::null_mut(), es);
            assert_eq!(snapshot(a, es), snapshot(b, es), "err22 es={es}");
            assert_eq!(snapshot(a, es).length, 1);
            // row 24: length != 0 -> rejected, same pointer
            let a2 = (c.hmput_default)(a, es);
            let b2 = (r.hmput_default)(b, es);
            assert_eq!(a2, a, "err24 es={es}: C");
            assert_eq!(b2, b, "err24 es={es}: Rust");
            assert_eq!(snapshot(a2, es).length, 1);
            (c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);

            // row 23: length == 0 -> regrow to 1
            let ra = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
            let rb = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
            let ha = (ra as *mut u8).add(es) as *mut c_void;
            let hb = (rb as *mut u8).add(es) as *mut c_void;
            let a3 = (c.hmput_default)(ha, es);
            let b3 = (r.hmput_default)(hb, es);
            assert_eq!(snapshot(a3, es), snapshot(b3, es), "err23 es={es}");
            assert_eq!(snapshot(a3, es).length, 1, "err23 es={es}");
            (c.hmfree_func)((a3 as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((b3 as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// =========================================================================
// stbds_hmput_key
// =========================================================================

/// ERRORS rows 25/29: bootstrap from NULL and the fresh-table `string.mode`.
#[test]
fn err25_29_hmput_key_bootstrap() {
    let (c, r) = libs();
    let _g = serial();
    for &es in &[8usize, 16, 24] {
        for &mode in &[-5i32, 0, 1, 2, 7, i32::MIN, i32::MAX] {
            set_seed(DEFAULT_SEED);
            unsafe {
                // string modes hash the key as a C string, so NUL-terminate it
                let mut key = [0x11u8; 32];
                key[9] = 0;
                let k = key.as_mut_ptr() as *mut c_void;
                let a = (c.hmput_key)(std::ptr::null_mut(), es, k, 8, mode);
                let b = (r.hmput_key)(std::ptr::null_mut(), es, k, 8, mode);
                let ctx = format!("err25 es={es} mode={mode}");
                assert_eq!(snapshot(a, es), snapshot(b, es), "{ctx}");
                let s = snapshot(a, es);
                assert_eq!(s.length, 2, "{ctx}: default slot + 1 element");
                assert_eq!(s.slot_count, 8, "{ctx}");
                assert_eq!(s.used_count, 1, "{ctx}");
                assert_eq!(
                    s.arena_mode,
                    if mode >= 1 { SH_DEFAULT as u8 } else { 0 },
                    "{ctx}: fresh string.mode"
                );
                assert_eq!(temp_of(a, es), 0, "{ctx}");
                assert_eq!(temp_of(b, es), 0, "{ctx}");
                (c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
                (r.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
            }
        }
    }
}

/// ERRORS row 26: duplicate found in the upper sub-loop -> no insert, `temp_key`
/// updated. ERRORS row 27: found in the wrap-around sub-loop -> `temp_key` is
/// NOT updated. Both libraries must classify every key into the same bucket.
#[test]
fn err26_27_duplicate_hit_temp_key_asymmetry() {
    let (c, r) = libs();
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x2601);
    let es = 16usize;
    // enough keys that buckets fill and the wrap-around sub-loop is exercised
    let n = 4000;
    let mut keys = Keys::strings(&mut rng, n, 24, ASCII);
    const SENTINEL: usize = 0xDEAD_BEEF_0000_0001;
    unsafe {
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        for i in 0..keys.len() {
            tc = (c.hmput_key)(tc, es, keys.ptr(i), 8, HM_STRING);
            tr = (r.hmput_key)(tr, es, keys.ptr(i), 8, HM_STRING);
        }
        assert_eq!(snapshot(tc, es), snapshot(tr, es), "err26: after inserts");

        let table_c = |t: *mut c_void| {
            (*((t as *mut u8).sub(es) as *mut ArrayHeader).offset(-1)).hash_table as *mut HashIndex
        };
        let mut upper = 0usize;
        let mut wrap = 0usize;
        for i in 0..keys.len() {
            // poison temp_key in both, then re-put: only the upper sub-loop
            // writes it back
            (*table_c(tc)).temp_key = SENTINEL as *mut c_char;
            (*table_c(tr)).temp_key = SENTINEL as *mut c_char;
            tc = (c.hmput_key)(tc, es, keys.ptr(i), 8, HM_STRING);
            tr = (r.hmput_key)(tr, es, keys.ptr(i), 8, HM_STRING);
            let kc = (*table_c(tc)).temp_key as usize;
            let kr = (*table_c(tr)).temp_key as usize;
            assert_eq!(kc, kr, "err26/27: temp_key path differs for key {i}");
            assert_eq!(temp_of(tc, es), i as isize, "err26: re-put index for key {i}");
            assert_eq!(temp_of(tr, es), i as isize, "err26: re-put index for key {i}");
            if kc == SENTINEL {
                wrap += 1; // row 27: wrap-around sub-loop, temp_key untouched
            } else {
                assert_eq!(kc, keys.ptr(i) as usize, "err26: temp_key must be the stored key");
                upper += 1; // row 26: upper sub-loop, temp_key written
            }
        }
        assert!(upper > 0, "err26: the upper sub-loop was never exercised");
        assert!(
            wrap > 0,
            "err27: the wrap-around sub-loop was never exercised ({upper} upper hits) — \
             increase the key count"
        );
        (c.hmfree_func)((tc as *mut u8).sub(es) as *mut c_void, es);
        (r.hmfree_func)((tr as *mut u8).sub(es) as *mut c_void, es);
    }
}

/// ERRORS row 28: `if (hash < 2) hash += 2` — no in-use slot may ever carry the
/// reserved hashes 0 (EMPTY) or 1 (DELETED).
#[test]
fn err28_reserved_hash_values_never_stored() {
    let _g = serial();
    for &(mode, sh) in &[(HM_BINARY, SH_NONE), (HM_STRING, SH_DEFAULT)] {
        for &seed in &[0usize, 1, DEFAULT_SEED, usize::MAX] {
            set_seed(seed);
            let mut rng = Rng::new(0x2800 ^ seed as u64 ^ mode as u64);
            let es = 16usize;
            let mut d = if sh == SH_NONE {
                Dual::new(es, 8, mode, KeyRepr::Bytes)
            } else {
                Dual::with_shmode(es, 8, mode, sh, KeyRepr::SharedPtr)
            };
            let mut keys = if mode == HM_BINARY {
                Keys::binary(&mut rng, 800, 8)
            } else {
                Keys::strings(&mut rng, 800, 20, ASCII)
            };
            for i in 0..keys.len() {
                let k = keys.ptr(i);
                d.put(k, &format!("err28 mode={mode} put i={i}"));
            }
            let s = d.snap_c();
            for (slot, &(h, idx)) in s.slots.iter().enumerate() {
                if idx >= 0 {
                    assert!(h >= 2, "err28: in-use slot {slot} carries reserved hash {h}");
                }
            }
            d.free();
        }
    }
    set_seed(DEFAULT_SEED);
}

/// ERRORS rows 30/31: table growth at the threshold, and tombstone reuse.
#[test]
fn err30_31_growth_and_tombstone_reuse() {
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x3001);
    let es = 16usize;
    let mut keys = Keys::binary(&mut rng, 200, 8);
    let mut d = Dual::new(es, 8, HM_BINARY, KeyRepr::Bytes);
    let mut counts = Vec::new();
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        d.put(k, &format!("err30 put i={i}"));
        counts.push((d.snap_c().slot_count, d.snap_c().used_count));
    }
    // row 30: growth happens exactly when used_count reaches the threshold
    assert_eq!(counts[5].0, 8, "err30: still 8 slots after 6 inserts");
    assert_eq!(counts[6].0, 16, "err30: 7th insert doubles");
    // row 31: delete then re-insert -> the tombstone slot is reused
    let before = d.snap_c();
    let k0 = keys.ptr(0);
    assert_eq!(d.del(k0, 0, "err31 del"), 1);
    let mid = d.snap_c();
    assert_eq!(mid.tombstone_count, before.tombstone_count + 1, "err31: tombstone recorded");
    assert_eq!(mid.used_count, before.used_count - 1);
    d.put(k0, "err31 reput");
    let after = d.snap_c();
    assert_eq!(after.tombstone_count, before.tombstone_count, "err31: tombstone consumed");
    assert_eq!(after.used_count, before.used_count);
    d.free();
}

/// ERRORS rows 33/34: out-of-range `mode` enum values across the FFI boundary.
#[test]
fn err33_34_out_of_range_mode_enum() {
    let (c, r) = libs();
    let _g = serial();
    // A C enum accepts any int. `mode >= 1` selects string handling, everything
    // else binary. Verify the split for every mode-taking entry point.
    let modes: [c_int; 12] = [i32::MIN, -1000, -2, -1, 0, 1, 2, 3, 4, 99, 1 << 20, i32::MAX];
    let es = 16usize;
    for &mode in &modes {
        set_seed(DEFAULT_SEED);
        let mut rng = Rng::new(0x3300 ^ mode as u32 as u64);
        // Keys are NUL-terminated AND >= 8 bytes, so both the binary
        // interpretation (memcmp of 8 bytes) and the string interpretation
        // (strcmp) are in bounds.
        let mut keys = Keys::strings(&mut rng, 120, 24, ASCII);
        keys.bufs.retain(|b| b.len() >= 9);
        unsafe {
            let mut tc: *mut c_void = std::ptr::null_mut();
            let mut tr: *mut c_void = std::ptr::null_mut();
            for i in 0..keys.len() {
                tc = (c.hmput_key)(tc, es, keys.ptr(i), 8, mode);
                tr = (r.hmput_key)(tr, es, keys.ptr(i), 8, mode);
                assert_eq!(snapshot(tc, es), snapshot(tr, es), "err33 mode={mode} put i={i}");
                assert_eq!(temp_of(tc, es), temp_of(tr, es), "err33 mode={mode} temp i={i}");
            }
            let sc = snapshot(tc, es);
            assert_eq!(
                sc.arena_mode,
                if mode >= 1 { SH_DEFAULT as u8 } else { 0 },
                "err33/34 mode={mode}: string.mode split"
            );
            // lookups with the same mode
            for i in 0..keys.len() {
                let a = (c.hmget_key)(tc, es, keys.ptr(i), 8, mode);
                let b = (r.hmget_key)(tr, es, keys.ptr(i), 8, mode);
                assert_eq!(a, tc);
                assert_eq!(b, tr);
                assert_eq!(temp_of(tc, es), temp_of(tr, es), "err33 mode={mode} get i={i}");
                assert_eq!(temp_of(tc, es), i as isize, "err33 mode={mode} get i={i}");
            }
            // deletes: last-element-first, so the mode-dependent re-find branch
            // (which aborts in C for mode > 1) is never entered
            for step in 0..keys.len() {
                let i = keys.len() - 1 - step;
                tc = (c.hmdel_key)(tc, es, keys.ptr(i), 8, 0, mode);
                tr = (r.hmdel_key)(tr, es, keys.ptr(i), 8, 0, mode);
                assert_eq!(snapshot(tc, es), snapshot(tr, es), "err33 mode={mode} del i={i}");
                assert_eq!(temp_of(tc, es), 1, "err33 mode={mode} del flag i={i}");
                assert_eq!(temp_of(tr, es), 1, "err33 mode={mode} del flag i={i}");
            }
            (c.hmfree_func)((tc as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((tr as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

/// ERRORS row 35: `string.mode` outside `{1,2,3}` takes the `switch` default arm
/// and memcpy's `keysize` key bytes into the element.
#[test]
fn err35_string_mode_default_arm_memcpys() {
    let (c, r) = libs();
    let _g = serial();
    let es = 24usize;
    // string.mode values that must all land in the default arm
    for &shm in &[0i32, 4, 5, 200, 255] {
        set_seed(DEFAULT_SEED);
        let mut rng = Rng::new(0x3500 ^ shm as u64);
        let mut keys = Keys::binary(&mut rng, 100, 8);
        unsafe {
            let mut tc = (c.shmode_func)(es, shm);
            let mut tr = (r.shmode_func)(es, shm);
            assert_eq!(snapshot(tc, es).arena_mode, (shm as u32 & 0xff) as u8);
            for i in 0..keys.len() {
                tc = (c.hmput_key)(tc, es, keys.ptr(i), 8, HM_BINARY);
                tr = (r.hmput_key)(tr, es, keys.ptr(i), 8, HM_BINARY);
                let ctx = format!("err35 shm={shm} i={i}");
                assert_eq!(snapshot(tc, es), snapshot(tr, es), "{ctx}");
                let idx = temp_of(tc, es);
                assert_eq!(idx, temp_of(tr, es), "{ctx}");
                // the key BYTES (not a pointer) must be in the element
                let ec = std::slice::from_raw_parts((tc as *mut u8).offset(es as isize * idx), 8);
                let er = std::slice::from_raw_parts((tr as *mut u8).offset(es as isize * idx), 8);
                assert_eq!(ec, er, "{ctx}: element key bytes");
                assert_eq!(ec, &keys.bufs[i][..8], "{ctx}: must be a byte copy of the key");
            }
            (c.hmfree_func)((tc as *mut u8).sub(es) as *mut c_void, es);
            (r.hmfree_func)((tr as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

/// ERRORS row 36: `keysize == 0` -> `memcmp(...,0) == 0` always matches.
#[test]
fn err36_keysize_zero_always_matches() {
    let (c, r) = libs();
    let _g = serial();
    let es = 16usize;
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x3601);
    unsafe {
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        let mut bufs: Vec<Box<[u8]>> = (0..25).map(|_| rng.bytes(8).into_boxed_slice()).collect();
        for i in 0..bufs.len() {
            let k = bufs[i].as_mut_ptr() as *mut c_void;
            tc = (c.hmput_key)(tc, es, k, 0, HM_BINARY);
            tr = (r.hmput_key)(tr, es, k, 0, HM_BINARY);
            assert_eq!(snapshot(tc, es), snapshot(tr, es), "err36 put i={i}");
            assert_eq!(temp_of(tc, es), 0, "err36: every key collapses onto index 0");
            assert_eq!(temp_of(tr, es), 0);
        }
        assert_eq!(snapshot(tc, es).length, 2, "err36: only one element ever exists");
        assert_eq!(snapshot(tc, es).used_count, 1);
        // and deleting once empties it
        let k = bufs[0].as_mut_ptr() as *mut c_void;
        tc = (c.hmdel_key)(tc, es, k, 0, 0, HM_BINARY);
        tr = (r.hmdel_key)(tr, es, k, 0, 0, HM_BINARY);
        assert_eq!(snapshot(tc, es), snapshot(tr, es), "err36 del");
        assert_eq!(temp_of(tc, es), 1);
        assert_eq!(snapshot(tc, es).length, 1);
        (c.hmfree_func)((tc as *mut u8).sub(es) as *mut c_void, es);
        (r.hmfree_func)((tr as *mut u8).sub(es) as *mut c_void, es);
    }
}

/// ERRORS row 37: `stbds_shmode_func` truncates `mode` to `unsigned char`.
#[test]
fn err37_shmode_func_truncation() {
    let (c, r) = libs();
    let _g = serial();
    let es = 16usize;
    let modes: [c_int; 16] =
        [i32::MIN, -257, -256, -255, -1, 0, 1, 2, 3, 4, 254, 255, 256, 257, 259, i32::MAX];
    for &m in &modes {
        set_seed(DEFAULT_SEED);
        unsafe {
            let a = (c.shmode_func)(es, m);
            let b = (r.shmode_func)(es, m);
            let ctx = format!("err37 mode={m}");
            assert_eq!(snapshot(a, es), snapshot(b, es), "{ctx}");
            assert_eq!(snapshot(a, es).arena_mode, (m as u32 & 0xff) as u8, "{ctx}");
            if snapshot(a, es).arena_mode as i32 != SH_STRDUP {
                (c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
                (r.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
            }
        }
    }
}

// =========================================================================
// stbds_hmdel_key
// =========================================================================

/// ERRORS row 38: `a == NULL` -> NULL.
#[test]
fn err38_hmdel_null_returns_null() {
    let (c, r) = libs();
    let mut key = [0u8; 16];
    let k = key.as_mut_ptr() as *mut c_void;
    for &es in &[0usize, 1, 8, 16, 1024] {
        for &mode in &[-1i32, 0, 1, 2, 99, i32::MIN, i32::MAX] {
            for &keyoffset in &[0usize, 8, usize::MAX] {
                unsafe {
                    let a = (c.hmdel_key)(std::ptr::null_mut(), es, k, 8, keyoffset, mode);
                    let b = (r.hmdel_key)(std::ptr::null_mut(), es, k, 8, keyoffset, mode);
                    let ctx = format!("err38 es={es} mode={mode} keyoffset={keyoffset}");
                    assert!(a.is_null(), "{ctx}: C must return NULL");
                    assert!(b.is_null(), "{ctx}: Rust must return NULL");
                }
            }
        }
    }
}

/// ERRORS row 39: `hash_table == 0` -> `temp = 0`, `a` returned unchanged.
#[test]
fn err39_hmdel_without_hash_table() {
    let (c, r) = libs();
    let mut key = [0x5Au8; 8];
    for &es in &[1usize, 8, 16, 24] {
        unsafe {
            let ra = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
            let rb = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 8);
            // poison temp so we can see it being cleared to 0
            (*(ra as *mut ArrayHeader).offset(-1)).temp = 0x1234;
            (*(rb as *mut ArrayHeader).offset(-1)).temp = 0x1234;
            let a = (ra as *mut u8).add(es) as *mut c_void;
            let b = (rb as *mut u8).add(es) as *mut c_void;
            let k = key.as_mut_ptr() as *mut c_void;
            let a2 = (c.hmdel_key)(a, es, k, 8, 0, HM_BINARY);
            let b2 = (r.hmdel_key)(b, es, k, 8, 0, HM_BINARY);
            let ctx = format!("err39 es={es}");
            assert_eq!(a2, a, "{ctx}: C pointer");
            assert_eq!(b2, b, "{ctx}: Rust pointer");
            assert_eq!(temp_of(a2, es), 0, "{ctx}: C temp must be cleared to 0");
            assert_eq!(temp_of(b2, es), 0, "{ctx}: Rust temp must be cleared to 0");
            assert_eq!(snapshot(a2, es), snapshot(b2, es), "{ctx}");
            (c.hmfree_func)(ra, es);
            (r.hmfree_func)(rb, es);
        }
    }
}

/// ERRORS rows 40/41/43/47: the delete flag, and the "already final" fast path.
#[test]
fn err40_41_43_47_delete_flag_and_paths() {
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x4001);
    let es = 16usize;
    let mut keys = Keys::binary(&mut rng, 200, 8);
    let mut d = Dual::new(es, 8, HM_BINARY, KeyRepr::Bytes);
    for i in 0..100 {
        let k = keys.ptr(i);
        d.put(k, "err40 put");
    }
    // row 40: absent key -> temp == 0, length unchanged
    let len_before = d.snap_c().length;
    for i in 100..200 {
        let k = keys.ptr(i);
        assert_eq!(d.del(k, 0, &format!("err40 absent i={i}")), 0, "err40: absent -> 0");
        assert_eq!(d.snap_c().length, len_before, "err40: length must not change");
    }
    // rows 41/47: present key -> temp == 1; deleting the final element skips the
    // memmove + re-find branch entirely
    for step in 0..100 {
        let i = 99 - step;
        let k = keys.ptr(i);
        assert_eq!(d.del(k, 0, &format!("err47 final i={i}")), 1, "err41: present -> 1");
    }
    // row 43: `used_count >= 0` is a dead assert on a size_t; draining to empty
    // must not trip anything
    assert_eq!(d.snap_c().length, 1);
    assert_eq!(d.snap_c().used_count, 0);
    d.free();
}

/// ERRORS row 46: `mode == STBDS_HM_STRING` is an exact comparison, so a
/// `SH_STRDUP` table deleted with `mode == 2` does NOT free the key.
#[test]
fn err46_mode_two_skips_strdup_free() {
    let (c, r) = libs();
    let _g = serial();
    let es = 16usize;
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x4601);
    let mut keys = Keys::strings(&mut rng, 80, 20, ASCII);
    unsafe {
        let mut tc = (c.shmode_func)(es, SH_STRDUP);
        let mut tr = (r.shmode_func)(es, SH_STRDUP);
        for i in 0..keys.len() {
            tc = (c.hmput_key)(tc, es, keys.ptr(i), 8, 2);
            tr = (r.hmput_key)(tr, es, keys.ptr(i), 8, 2);
        }
        assert_eq!(snapshot(tc, es), snapshot(tr, es), "err46: after inserts");
        // remember the last element's key pointer; with mode == 2 the C does not
        // free it, so it must still hold the original text after the delete
        for step in 0..keys.len() {
            let i = keys.len() - 1 - step;
            let idxc = snapshot(tc, es).length as isize - 2;
            let pc = *((tc as *mut u8).offset(es as isize * idxc) as *const *const c_char);
            let pr = *((tr as *mut u8).offset(es as isize * idxc) as *const *const c_char);
            let want = CStr::from_ptr(pc).to_bytes().to_vec();
            assert_eq!(want, CStr::from_ptr(pr).to_bytes().to_vec());
            tc = (c.hmdel_key)(tc, es, keys.ptr(i), 8, 0, 2);
            tr = (r.hmdel_key)(tr, es, keys.ptr(i), 8, 0, 2);
            let ctx = format!("err46 step={step} key={i}");
            assert_eq!(snapshot(tc, es), snapshot(tr, es), "{ctx}");
            assert_eq!(temp_of(tc, es), 1, "{ctx}");
            assert_eq!(temp_of(tr, es), 1, "{ctx}");
            // not freed -> still readable and unchanged, identically in both
            assert_eq!(CStr::from_ptr(pc).to_bytes(), &want[..], "{ctx}: C key was freed");
            assert_eq!(CStr::from_ptr(pr).to_bytes(), &want[..], "{ctx}: Rust key was freed");
        }
        (c.hmfree_func)((tc as *mut u8).sub(es) as *mut c_void, es);
        (r.hmfree_func)((tr as *mut u8).sub(es) as *mut c_void, es);
    }
}

/// ERRORS rows 48/49/50: shrink, same-size rebuild, and the 8-slot floor.
#[test]
fn err48_49_50_shrink_and_rebuild() {
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x4801);
    let es = 16usize;
    let mut keys = Keys::binary(&mut rng, 500, 8);
    let mut d = Dual::new(es, 8, HM_BINARY, KeyRepr::Bytes);
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        d.put(k, "err48 put");
    }
    let mut shrinks = 0;
    let mut rebuilds = 0;
    let mut prev = d.snap_c();
    let mut order: Vec<usize> = (0..keys.len()).collect();
    for i in (1..order.len()).rev() {
        order.swap(i, rng.below(i + 1));
    }
    for &i in &order {
        let k = keys.ptr(i);
        d.del(k, 0, &format!("err48 del i={i}"));
        let now = d.snap_c();
        if now.slot_count < prev.slot_count {
            shrinks += 1;
            assert_eq!(now.slot_count, prev.slot_count / 2, "err48: must halve");
        } else if now.tombstone_count < prev.tombstone_count {
            rebuilds += 1;
            assert_eq!(now.slot_count, prev.slot_count, "err49: same-size rebuild");
            assert_eq!(now.tombstone_count, 0, "err49: tombstones purged");
        }
        assert!(now.slot_count >= 8, "err50: must never go below 8 slots");
        prev = now;
    }
    assert!(shrinks > 0, "err48: no shrink observed");
    assert!(rebuilds > 0, "err49: no same-size rebuild observed");
    assert_eq!(prev.slot_count, 8, "err50: floor is 8");
    d.free();
}

// =========================================================================
// stbds_stralloc / stbds_strreset
// =========================================================================

/// ERRORS rows 51/52/53/54/55/58: every branch of `stbds_stralloc`.
#[test]
fn err51_58_stralloc_branches() {
    let (c, r) = libs();
    let mut rng = Rng::new(0x5101);

    // row 58: empty string -> 1 byte carved; row 51: fits in `remaining`
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        let mut empty = [0u8; 1];
        for i in 0..512 {
            let pc = (c.stralloc)(&mut ac, empty.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, empty.as_mut_ptr() as *mut c_char);
            assert_eq!(ac.remaining, ar.remaining, "err58: remaining at #{i}");
            assert_eq!(ac.block, ar.block, "err58: block at #{i}");
            assert_eq!(CStr::from_ptr(pc).to_bytes().len(), 0);
            assert_eq!(CStr::from_ptr(pr).to_bytes().len(), 0);
        }
        // 512 one-byte allocations exactly exhaust the first 512-byte block
        assert_eq!(ac.remaining, ar.remaining);
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }

    // row 52: len > remaining but <= blocksize -> new block, remaining = blocksize
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        for i in 0..30 {
            let mut s = rng.cstring(200, ASCII);
            let p = s.as_mut_ptr() as *mut c_char;
            (c.stralloc)(&mut ac, p);
            (r.stralloc)(&mut ar, p);
            assert_eq!((ac.remaining, ac.block), (ar.remaining, ar.block), "err52 #{i}");
        }
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }

    // row 54: oversized with storage == NULL -> remaining set to 0
    unsafe {
        for &len in &[512usize, 513, 5000] {
            let mut ac = StringArena::zeroed();
            let mut ar = StringArena::zeroed();
            let mut s = rng.cstring(len, ASCII);
            let p = s.as_mut_ptr() as *mut c_char;
            let pc = (c.stralloc)(&mut ac, p);
            let pr = (r.stralloc)(&mut ar, p);
            let ctx = format!("err54 len={len}");
            assert_eq!(ac.remaining, 0, "{ctx}: C remaining");
            assert_eq!(ar.remaining, 0, "{ctx}: Rust remaining");
            assert_eq!(ac.block, ar.block, "{ctx}: block");
            // the result is the head block's own storage
            assert_eq!(pc as usize, (&raw mut (*ac.storage).storage) as usize, "{ctx}: C");
            assert_eq!(pr as usize, (&raw mut (*ar.storage).storage) as usize, "{ctx}: Rust");
            assert_eq!(CStr::from_ptr(pc).to_bytes(), CStr::from_ptr(pr).to_bytes(), "{ctx}");
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
    }

    // row 53: oversized with an existing head block -> spliced in AFTER the head
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        let mut small = rng.cstring(4, ASCII);
        (c.stralloc)(&mut ac, small.as_mut_ptr() as *mut c_char);
        (r.stralloc)(&mut ar, small.as_mut_ptr() as *mut c_char);
        let rem_c = ac.remaining;
        let rem_r = ar.remaining;
        let head_c = ac.storage;
        let head_r = ar.storage;
        let mut big = rng.cstring(9000, ASCII);
        let pc = (c.stralloc)(&mut ac, big.as_mut_ptr() as *mut c_char);
        let pr = (r.stralloc)(&mut ar, big.as_mut_ptr() as *mut c_char);
        assert_eq!(ac.remaining, rem_c, "err53: C remaining must be untouched");
        assert_eq!(ar.remaining, rem_r, "err53: Rust remaining must be untouched");
        assert_eq!(ac.storage, head_c, "err53: C head must not change");
        assert_eq!(ar.storage, head_r, "err53: Rust head must not change");
        // second block in the list is the oversized one
        assert_eq!(pc as usize, (&raw mut (*(*ac.storage).next).storage) as usize, "err53: C");
        assert_eq!(pr as usize, (&raw mut (*(*ar.storage).next).storage) as usize, "err53: Rust");
        assert_eq!(CStr::from_ptr(pc).to_bytes(), CStr::from_ptr(pr).to_bytes());
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }

    // row 55: `block` saturates once blocksize >= 1<<20 (i.e. block >= 22)
    unsafe {
        for blk in 18u8..=26 {
            let mut ac = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: blk, mode: 0 };
            let mut ar = ac;
            let mut s = rng.cstring(3, ASCII);
            let p = s.as_mut_ptr() as *mut c_char;
            (c.stralloc)(&mut ac, p);
            (r.stralloc)(&mut ar, p);
            let ctx = format!("err55 block={blk}");
            assert_eq!(ac.block, ar.block, "{ctx}");
            assert_eq!(ac.remaining, ar.remaining, "{ctx}");
            assert_eq!(ac.block, if blk <= 21 { blk + 1 } else { blk }, "{ctx}: saturation");
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
    }
}

/// ERRORS row 56: caller-supplied `block` big enough that `512 << (block>>1)`
/// shifts by >= 64. `block == 110` gives a shift of 55, i.e. `2^64` -> 0, so the
/// oversized-block branch is taken for *any* length.
#[test]
fn err56_stralloc_shift_overflow() {
    let (c, r) = libs();
    let mut rng = Rng::new(0x5601);
    // NOTE: `block` values whose masked shift lands in the middle of the range
    // (e.g. 200 -> shift 100 & 63 = 36 -> blocksize 2^45) make the C ask
    // `realloc` for 32 TiB, get NULL, and then write through it. That is an OOM
    // crash in both libraries, so those values are excluded (ERRORS.md row 4).
    for &blk in &[110u8, 111, 112, 113, 126, 127, 128, 254, 255] {
        for &len in &[0usize, 1, 40, 700] {
            unsafe {
                let mut ac = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: blk, mode: 0 };
                let mut ar = ac;
                let mut s = rng.cstring(len, ASCII);
                let p = s.as_mut_ptr() as *mut c_char;
                let pc = (c.stralloc)(&mut ac, p);
                let pr = (r.stralloc)(&mut ar, p);
                let ctx = format!("err56 block={blk} len={len}");
                assert_eq!(ac.block, ar.block, "{ctx}: block");
                assert_eq!(ac.remaining, ar.remaining, "{ctx}: remaining");
                assert_eq!(ac.storage.is_null(), ar.storage.is_null(), "{ctx}");
                assert_eq!(
                    CStr::from_ptr(pc).to_bytes(),
                    CStr::from_ptr(pr).to_bytes(),
                    "{ctx}: content"
                );
                // Whether the result is a whole block's `storage` (the oversized
                // path) or a carve out of the head block is the structural
                // observable; both libraries must pick the same branch.
                // block>>1 == 55/56/63 wraps `512 << k` to 0 -> always oversized;
                // block == 128 masks the shift to 0 -> blocksize 512 again.
                let head_c = (&raw mut (*ac.storage).storage) as usize;
                let head_r = (&raw mut (*ar.storage).storage) as usize;
                assert_eq!(
                    pc as usize == head_c,
                    pr as usize == head_r,
                    "{ctx}: oversized-vs-carve branch differs"
                );
                assert_eq!(
                    (pc as usize).wrapping_sub(head_c),
                    (pr as usize).wrapping_sub(head_r),
                    "{ctx}: offset within the head block differs"
                );
                (c.strreset)(&mut ac);
                (r.strreset)(&mut ar);
            }
        }
    }
}

/// ERRORS row 59: `strreset` on an arena with no blocks still zeroes everything.
#[test]
fn err59_strreset_zeroes_everything() {
    let (c, r) = libs();
    unsafe {
        for &(blk, mode, rem) in &[(0u8, 0u8, 0usize), (7, 3, 123), (255, 255, usize::MAX)] {
            let mut ac = StringArena { storage: std::ptr::null_mut(), remaining: rem, block: blk, mode };
            let mut ar = ac;
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
            let ctx = format!("err59 blk={blk} mode={mode} rem={rem}");
            assert!(ac.storage.is_null() && ar.storage.is_null(), "{ctx}");
            assert_eq!((ac.remaining, ac.block, ac.mode), (0, 0, 0), "{ctx}: C");
            assert_eq!((ar.remaining, ar.block, ar.mode), (0, 0, 0), "{ctx}: Rust");
        }
    }
}

// =========================================================================
// strkey
// =========================================================================

/// ERRORS rows 61/62: extreme `int` values and the shared static buffer.
#[test]
fn err61_62_strkey_extremes() {
    let (c, r) = libs();
    unsafe {
        for &n in &[i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            let sc = CStr::from_ptr(pc).to_bytes().to_vec();
            let sr = CStr::from_ptr(pr).to_bytes().to_vec();
            assert_eq!(sc, sr, "err61 n={n}");
            assert_eq!(sc, format!("test_{n}").into_bytes(), "err61 n={n}");
            assert!(sc.len() < 256, "err61: must fit the 256-byte static buffer");
        }
        // row 62: same buffer every time
        let p1 = (c.strkey)(i32::MIN);
        let p2 = (c.strkey)(0);
        assert_eq!(p1, p2);
        assert_eq!(CStr::from_ptr(p1).to_bytes(), b"test_0");
        let q1 = (r.strkey)(i32::MIN);
        let q2 = (r.strkey)(0);
        assert_eq!(q1, q2);
        assert_eq!(CStr::from_ptr(q1).to_bytes(), b"test_0");
    }
}
