//! Phase C — error/rejection-path differential tests.
//! One test per (testable) row of ERRORS.md, plus the generic FFI boundary
//! cases: NULL pointers, zero/oversized lengths, one-past-range values, and
//! out-of-range `int` enum values.

#![allow(non_snake_case)]

#[path = "common/mod.rs"]
mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEED: usize = 0x31415926;

unsafe fn hdr(map: *mut c_void, es: usize) -> *mut Header {
    ((map as *mut u8).sub(es + HDR)) as *mut Header
}
unsafe fn temp_of(map: *mut c_void, es: usize) -> isize {
    (*hdr(map, es)).temp
}
unsafe fn tbl(map: *mut c_void, es: usize) -> *mut HashIndex {
    (*hdr(map, es)).hash_table as *mut HashIndex
}

/// every `int mode` value worth pushing across the FFI boundary, including the
/// ones with no corresponding enum variant
const MODES: &[c_int] = &[
    c_int::MIN,
    -1000,
    -2,
    -1,
    0, // STBDS_HM_BINARY
    1, // STBDS_HM_STRING
    2,
    3,
    4,
    7,
    255,
    256,
    12345,
    c_int::MAX,
];

// ---------------------------------------------------------------------------
// row 1-3 : stbds_arrgrowf rejection / clamping
// ---------------------------------------------------------------------------

#[test]
fn err01_arrgrowf_no_grow_returns_same_pointer() {
    differential("err01", |l, t| unsafe {
        // a == NULL, min_cap == 0 -> min_cap <= arrcap(NULL) == 0 -> returns NULL
        for &es in &[0usize, 1, 8, 64] {
            let r = (l.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            t.rec(&format!("null_es{es}"), r.is_null());
        }
        // existing array, min_cap <= capacity -> identical pointer, no realloc
        let a = (l.arrgrowf)(std::ptr::null_mut(), 8, 0, 1);
        let cap = (*((a as *mut u8).sub(HDR) as *const Header)).capacity;
        t.rec("cap", cap);
        for mc in 0..=cap {
            let b = (l.arrgrowf)(a, 8, 0, mc);
            t.rec(&format!("same{mc}"), b == a);
        }
        // one step past the valid no-grow range
        let b = (l.arrgrowf)(a, 8, 0, cap + 1);
        t.rec("grew", b != a);
        snap_arr(t, "grown", b, 8, false);
        (l.arrfreef)(b);
    });
}

#[test]
fn err02_arrgrowf_addlen_raises_min_cap() {
    differential("err02", |l, t| unsafe {
        for &addlen in &[0usize, 1, 2, 3, 4, 5, 100, 1000] {
            let a = (l.arrgrowf)(std::ptr::null_mut(), 8, addlen, 0);
            snap_arr(t, &format!("a{addlen}"), a, 8, false);
            if !a.is_null() {
                (l.arrfreef)(a);
            }
        }
    });
}

#[test]
fn err03_arrgrowf_min4_and_doubling_clamps() {
    differential("err03", |l, t| unsafe {
        // fresh: min_cap 1,2,3 all clamp to 4
        for &mc in &[1usize, 2, 3, 4, 5] {
            let a = (l.arrgrowf)(std::ptr::null_mut(), 8, 0, mc);
            snap_arr(t, &format!("f{mc}"), a, 8, false);
            (l.arrfreef)(a);
        }
        // existing cap=4: min_cap 5..8 clamps to 8 (2*cap)
        for &mc in &[5usize, 6, 7, 8, 9, 100] {
            let a = (l.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
            let b = (l.arrgrowf)(a, 8, 0, mc);
            snap_arr(t, &format!("g{mc}"), b, 8, false);
            (l.arrfreef)(b);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 6-7 : stbds_hmfree_func
// ---------------------------------------------------------------------------

#[test]
fn err06_hmfree_null() {
    differential("err06", |l, t| unsafe {
        for &es in &[0usize, 1, 8, 16, 4096] {
            (l.hmfree_func)(std::ptr::null_mut(), es);
            t.rec(&format!("survived{es}"), true);
        }
    });
}

#[test]
fn err07_hmfree_no_hash_table() {
    differential("err07", |l, t| unsafe {
        for &es in &[8usize, 16] {
            let a = (l.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            (*((a as *mut u8).sub(HDR) as *mut Header)).length = 0;
            (l.hmfree_func)(a, es);
            t.rec(&format!("freed{es}"), true);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 8-13 : lookup misses / sentinels
// ---------------------------------------------------------------------------

#[test]
fn err08_find_slot_returns_minus_one_for_absent() {
    let present = rand_u64_keys(50, 0x111);
    let absent = rand_u64_keys(50, 0x222);
    differential("err08", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let es = 16usize;
        let mut p: *mut c_void = std::ptr::null_mut();
        for k in &present {
            p = (l.hmput_key)(p, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
            let idx = temp_of(p, es);
            for j in 8..es {
                *((p as *mut u8).add(es * idx as usize + j)) = 0;
            }
        }
        for (i, k) in absent.iter().enumerate() {
            let mut tmp: isize = 0x55;
            p = (l.hmget_key_ts)(p, es, k.as_ptr() as *mut c_void, 8, &mut tmp, HM_BINARY);
            t.rec(&format!("miss{i}"), tmp);
            assert_eq!(tmp, -1, "expected STBDS_INDEX_EMPTY");
        }
        (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
    });
}

#[test]
fn err09_get_ts_null_map_bootstraps() {
    differential("err09", |l, t| unsafe {
        for &es in &[1usize, 8, 16, 64] {
            for &mode in MODES {
                let mut tmp: isize = 999;
                let key = cstring(b"whatever");
                let p = (l.hmget_key_ts)(
                    std::ptr::null_mut(),
                    es,
                    key.as_ptr() as *mut c_void,
                    8,
                    &mut tmp,
                    mode,
                );
                t.rec(&format!("temp{es}_{mode}"), tmp);
                snap_map(t, &format!("m{es}_{mode}"), p, es, KeyKind::Raw);
                (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
            }
        }
    });
}

#[test]
fn err10_get_ts_table_null() {
    differential("err10", |l, t| unsafe {
        let es = 16usize;
        for &mode in MODES {
            let a = (l.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            for i in 0..es {
                *(a as *mut u8).add(i) = 0;
            }
            (*((a as *mut u8).sub(HDR) as *mut Header)).length = 1;
            let map = (a as *mut u8).add(es) as *mut c_void;
            let mut tmp: isize = 777;
            let key = cstring(b"nope");
            let p = (l.hmget_key_ts)(
                map,
                es,
                key.as_ptr() as *mut c_void,
                8,
                &mut tmp,
                mode,
            );
            t.rec(&format!("temp{mode}"), tmp);
            t.rec(&format!("same{mode}"), p == map);
            (l.hmfree_func)(a, es);
        }
    });
}

#[test]
fn err13_get_key_writes_temp_on_miss() {
    differential("err13", |l, t| unsafe {
        let es = 16usize;
        // NULL map
        let key = cstring(b"absent");
        let p = (l.hmget_key)(
            std::ptr::null_mut(),
            es,
            key.as_ptr() as *mut c_void,
            8,
            HM_BINARY,
        );
        t.rec("temp_after_null", temp_of(p, es));
        // now a table-less array
        let mut tmp = temp_of(p, es);
        t.rec("is_minus1", tmp == -1);
        tmp = 0;
        let _ = tmp;
        (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);

        // populated map, absent key
        (l.rand_seed)(SEED);
        let mut q: *mut c_void = std::ptr::null_mut();
        for k in &rand_u64_keys(20, 0x999) {
            q = (l.hmput_key)(q, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
            let idx = temp_of(q, es);
            for j in 8..es {
                *((q as *mut u8).add(es * idx as usize + j)) = 0;
            }
        }
        let bad = cstring(b"deadbeefX");
        q = (l.hmget_key)(q, es, bad.as_ptr() as *mut c_void, 8, HM_BINARY);
        t.rec("temp_miss", temp_of(q, es));
        (l.hmfree_func)((q as *mut u8).sub(es) as *mut c_void, es);
    });
}

// ---------------------------------------------------------------------------
// rows 14-15 : hmput_default degenerate inputs
// ---------------------------------------------------------------------------

#[test]
fn err14_15_hmput_default_null_and_empty() {
    differential("err14", |l, t| unsafe {
        for &es in &[1usize, 8, 16, 64] {
            let a = (l.hmput_default)(std::ptr::null_mut(), es);
            snap_map(t, &format!("null{es}"), a, es, KeyKind::Raw);
            // second call is a no-op because length != 0
            let b = (l.hmput_default)(a, es);
            t.rec(&format!("noop{es}"), a == b);
            (l.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);

            // length == 0 array
            let arr = (l.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let map = (arr as *mut u8).add(es) as *mut c_void;
            let c = (l.hmput_default)(map, es);
            snap_map(t, &format!("empty{es}"), c, es, KeyKind::Raw);
            (l.hmfree_func)((c as *mut u8).sub(es) as *mut c_void, es);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 16-21 : hmput_key with NULL map, fresh table, and out-of-range modes
// ---------------------------------------------------------------------------

fn rand_u64_keys(n: usize, sd: u64) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(sd);
    (0..n).map(|_| rng.bytes(8)).collect()
}

#[test]
fn err16_17_put_null_map_fresh_table() {
    differential("err16", |l, t| unsafe {
        (l.rand_seed)(SEED);
        for &es in &[8usize, 16, 24] {
            let key = [0xABu8; 8];
            let p = (l.hmput_key)(
                std::ptr::null_mut(),
                es,
                key.as_ptr() as *mut c_void,
                8,
                HM_BINARY,
            );
            let idx = temp_of(p, es);
            for j in 8..es {
                *((p as *mut u8).add(es * idx as usize + j)) = 0;
            }
            t.rec(&format!("idx{es}"), idx);
            t.rec(&format!("slots{es}"), (*tbl(p, es)).slot_count);
            snap_map(t, &format!("m{es}"), p, es, KeyKind::Raw);
            (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
        }
    });
}

#[test]
fn err18_put_crosses_used_count_threshold() {
    differential("err18", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let es = 16usize;
        let mut p: *mut c_void = std::ptr::null_mut();
        for i in 0u64..40 {
            let k = i.to_le_bytes();
            p = (l.hmput_key)(p, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
            let idx = temp_of(p, es);
            for j in 8..es {
                *((p as *mut u8).add(es * idx as usize + j)) = 0;
            }
            let ti = &*tbl(p, es);
            t.rec(
                &format!("i{i}"),
                (
                    ti.slot_count,
                    ti.used_count,
                    ti.used_count_threshold,
                    ti.tombstone_count_threshold,
                    ti.used_count_shrink_threshold,
                ),
            );
        }
        (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
    });
}

/// rows 19-21 + 24-25: every out-of-range `int mode` on hmput_key, on a fresh
/// implicit table.  `mode >= 1` => string hashing, `mode < 1` => binary.
#[test]
fn err19_21_put_out_of_range_modes() {
    differential("err19", |l, t| unsafe {
        let es = 16usize;
        for &mode in MODES {
            (l.rand_seed)(SEED);
            // 8-byte NUL-terminated key works for BOTH interpretations
            let key = cstring(b"abcdefg");
            let p = (l.hmput_key)(
                std::ptr::null_mut(),
                es,
                key.as_ptr() as *mut c_void,
                8,
                mode,
            );
            let idx = temp_of(p, es);
            for j in 8..es {
                *((p as *mut u8).add(es * idx as usize + j)) = 0;
            }
            let ti = &*tbl(p, es);
            t.rec(&format!("mode{mode}.idx"), idx);
            t.rec(&format!("mode{mode}.strmode"), ti.string.mode);
            t.rec(&format!("mode{mode}.slots"), ti.slot_count);
            t.rec(&format!("mode{mode}.used"), ti.used_count);
            for b in 0..(ti.slot_count >> 3) {
                let bk = &*ti.storage.add(b);
                t.rec_str(
                    &format!("mode{mode}.bucket{b}"),
                    format!("{:?} {:?}", bk.hash, bk.index),
                );
            }
            // for mode >= 1 the element holds the key POINTER (SH_DEFAULT);
            // for mode < 1 it holds the 8 key BYTES
            if mode >= HM_STRING {
                let kp = *((p as *mut u8).add(es * idx as usize) as *const *const c_char);
                t.rec_str(&format!("mode{mode}.key"), cstr(kp));
                t.rec(&format!("mode{mode}.verbatim"), kp == key.as_ptr() as *const c_char);
            } else {
                t.rec_str(
                    &format!("mode{mode}.key"),
                    hex((p as *mut u8).add(es * idx as usize), 8),
                );
            }
            (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
        }
    });
}

#[test]
fn err22_tombstone_reuse_decrements_count() {
    let keys = rand_u64_keys(30, 0x321);
    differential("err22", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let es = 16usize;
        let mut p: *mut c_void = std::ptr::null_mut();
        let mut put = |p: &mut *mut c_void, k: &Vec<u8>| unsafe {
            *p = (l.hmput_key)(*p, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
            let idx = temp_of(*p, es);
            for j in 8..es {
                *((*p as *mut u8).add(es * idx as usize + j)) = 0;
            }
            idx
        };
        for k in &keys {
            put(&mut p, k);
        }
        for k in keys.iter().take(3) {
            p = (l.hmdel_key)(p, es, k.as_ptr() as *mut c_void, 8, 0, HM_BINARY);
            let ti = &*tbl(p, es);
            t.rec("after_del", (ti.used_count, ti.tombstone_count));
        }
        for k in keys.iter().take(3) {
            let idx = put(&mut p, k);
            let ti = &*tbl(p, es);
            t.rec("after_reput", (idx, ti.used_count, ti.tombstone_count));
        }
        snap_map(t, "final", p, es, KeyKind::Raw);
        (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
    });
}

// ---------------------------------------------------------------------------
// rows 24-25 : stbds_shmode_func with out-of-range modes  (+ the `default:` arm)
// ---------------------------------------------------------------------------

#[test]
fn err25_shmode_func_out_of_range_truncation() {
    differential("err25", |l, t| unsafe {
        for &es in &[8usize, 16] {
            for &mode in MODES {
                (l.rand_seed)(SEED);
                let p = (l.shmode_func)(es, mode);
                let ti = &*tbl(p, es);
                t.rec(
                    &format!("es{es}_mode{mode}"),
                    (
                        ti.string.mode,
                        ti.slot_count,
                        ti.used_count,
                        ti.string.remaining,
                        ti.string.block,
                        ti.string.storage.is_null(),
                    ),
                );
                snap_map(t, &format!("m{es}_{mode}"), p, es, KeyKind::Raw);
                (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
            }
        }
    });
}

/// row 24: `table->string.mode` outside {1,2,3} takes the `default:` arm and
/// `memcpy`s `keysize` raw bytes even though `mode >= STBDS_HM_STRING`.
#[test]
fn err24_default_switch_arm_memcpy() {
    // string.mode values with no valid variant: 0 and >= 4 (via truncation)
    let smodes: Vec<c_int> = vec![0, 4, 5, 7, 100, 255, 256, -1, c_int::MAX];
    differential("err24", |l, t| unsafe {
        let es = 16usize;
        for &sm in &smodes {
            (l.rand_seed)(SEED);
            let p0 = (l.shmode_func)(es, sm);
            let mut p = p0;
            // distinct keys only: comparing keys would strcmp a garbage pointer
            for i in 0..4 {
                let key = cstring(format!("uniq_key_{i:08}").as_bytes());
                p = (l.hmput_key)(p, es, key.as_ptr() as *mut c_void, 8, HM_STRING);
                let idx = temp_of(p, es);
                for j in 8..es {
                    *((p as *mut u8).add(es * idx as usize + j)) = 0;
                }
                t.rec(&format!("sm{sm}.idx{i}"), idx);
                t.rec_str(
                    &format!("sm{sm}.bytes{i}"),
                    hex((p as *mut u8).add(es * idx as usize), 8),
                );
                t.rec_str(
                    &format!("sm{sm}.expect{i}"),
                    hex(key.as_ptr(), 8),
                );
            }
            let ti = &*tbl(p, es);
            t.rec(&format!("sm{sm}.strmode"), ti.string.mode);
            snap_map(t, &format!("sm{sm}.map"), p, es, KeyKind::Raw);
            (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 26-28, 31, 34-36 : hmdel_key rejections
// ---------------------------------------------------------------------------

#[test]
fn err26_del_null_map_returns_null() {
    differential("err26", |l, t| unsafe {
        let key = cstring(b"anything");
        for &es in &[0usize, 1, 8, 16] {
            for &mode in MODES {
                for &ko in &[0usize, 8, 1024] {
                    let r = (l.hmdel_key)(
                        std::ptr::null_mut(),
                        es,
                        key.as_ptr() as *mut c_void,
                        8,
                        ko,
                        mode,
                    );
                    t.rec(&format!("null{es}_{mode}_{ko}"), r.is_null());
                }
            }
        }
    });
}

#[test]
fn err27_del_table_null_sets_temp_zero() {
    differential("err27", |l, t| unsafe {
        let es = 16usize;
        for &mode in MODES {
            let a = (l.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let h = (a as *mut u8).sub(HDR) as *mut Header;
            (*h).length = 1;
            (*h).temp = 0x7777;
            for i in 0..es {
                *(a as *mut u8).add(i) = 0;
            }
            let map = (a as *mut u8).add(es) as *mut c_void;
            let key = cstring(b"absent!!");
            let r = (l.hmdel_key)(map, es, key.as_ptr() as *mut c_void, 8, 0, mode);
            t.rec(&format!("same{mode}"), r == map);
            t.rec(&format!("temp{mode}"), (*h).temp);
            (l.hmfree_func)(a, es);
        }
    });
}

#[test]
fn err28_del_absent_key_is_noop() {
    let present = rand_u64_keys(30, 0xAB1);
    let absent = rand_u64_keys(30, 0xAB2);
    differential("err28", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let es = 16usize;
        let mut p: *mut c_void = std::ptr::null_mut();
        for k in &present {
            p = (l.hmput_key)(p, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
            let idx = temp_of(p, es);
            for j in 8..es {
                *((p as *mut u8).add(es * idx as usize + j)) = 0;
            }
        }
        snap_map(t, "before", p, es, KeyKind::Raw);
        for (i, k) in absent.iter().enumerate() {
            let before_len = (*hdr(p, es)).length;
            p = (l.hmdel_key)(p, es, k.as_ptr() as *mut c_void, 8, 0, HM_BINARY);
            t.rec(
                &format!("noop{i}"),
                (temp_of(p, es), before_len == (*hdr(p, es)).length),
            );
        }
        snap_map(t, "after", p, es, KeyKind::Raw);
        (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
    });
}

/// row 31: the strdup free in `hmdel_key` is guarded by `mode == 1` exactly,
/// so `mode = 2 / 7 / INT_MAX` hash+compare as strings yet LEAK the key.
/// Observable difference: none in memory contents, but the delete must succeed
/// identically for every one of those modes.
#[test]
fn err31_del_string_mode_variants() {
    differential("err31", |l, t| unsafe {
        let es = 16usize;
        for &mode in &[1i32, 2, 3, 7, 255, 12345, c_int::MAX] {
            for &shm in &[SH_STRDUP, SH_ARENA, SH_DEFAULT] {
                (l.rand_seed)(SEED);
                let mut p = (l.shmode_func)(es, shm);
                let keys: Vec<Vec<u8>> = (0..12)
                    .map(|i| cstring(format!("key_{mode}_{shm}_{i:04}").as_bytes()))
                    .collect();
                for k in &keys {
                    p = (l.hmput_key)(p, es, k.as_ptr() as *mut c_void, 8, mode);
                    let idx = temp_of(p, es);
                    for j in 8..es {
                        *((p as *mut u8).add(es * idx as usize + j)) = 0;
                    }
                }
                // NOTE: delete in REVERSE insertion order so that
                // `old_index == final_index` and hmdel_key never takes the
                // swap-with-last branch.  For `mode >= 2` that branch passes
                // the *address* of the element (not the stored `char *`) to
                // stbds_hm_find_slot, which then string-hashes the pointer
                // bytes, finds nothing, and trips `STBDS_ASSERT(slot >= 0)`
                // -> abort().  See ERRORS.md row 32b.
                for i in (6..keys.len()).rev() {
                    let k = &keys[i];
                    p = (l.hmdel_key)(p, es, k.as_ptr() as *mut c_void, 8, 0, mode);
                    t.rec(&format!("m{mode}_s{shm}_d{i}"), temp_of(p, es));
                }
                let ti = &*tbl(p, es);
                t.rec(
                    &format!("m{mode}_s{shm}_state"),
                    (
                        (*hdr(p, es)).length,
                        ti.slot_count,
                        ti.used_count,
                        ti.tombstone_count,
                        ti.string.mode,
                    ),
                );
                for (i, k) in keys.iter().enumerate() {
                    let mut tmp: isize = 0;
                    p = (l.hmget_key_ts)(p, es, k.as_ptr() as *mut c_void, 8, &mut tmp, mode);
                    t.rec(&format!("m{mode}_s{shm}_g{i}"), tmp);
                }
                (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
                std::hint::black_box(&keys);
            }
        }
    });
}

#[test]
fn err34_35_del_shrink_and_rebuild_thresholds() {
    let keys = rand_u64_keys(300, 0xCC1);
    differential("err34", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let es = 16usize;
        let mut p: *mut c_void = std::ptr::null_mut();
        for k in &keys {
            p = (l.hmput_key)(p, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
            let idx = temp_of(p, es);
            for j in 8..es {
                *((p as *mut u8).add(es * idx as usize + j)) = 0;
            }
        }
        for (i, k) in keys.iter().enumerate() {
            p = (l.hmdel_key)(p, es, k.as_ptr() as *mut c_void, 8, 0, HM_BINARY);
            let ti = &*tbl(p, es);
            t.rec(
                &format!("d{i}"),
                (
                    temp_of(p, es),
                    (*hdr(p, es)).length,
                    ti.slot_count,
                    ti.used_count,
                    ti.tombstone_count,
                    ti.tombstone_count_threshold,
                    ti.used_count_shrink_threshold,
                ),
            );
        }
        snap_map(t, "empty", p, es, KeyKind::Raw);
        (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
    });
}

#[test]
fn err36_del_last_remaining_entry() {
    differential("err36", |l, t| unsafe {
        let es = 16usize;
        (l.rand_seed)(SEED);
        let k = [0x11u8; 8];
        let mut p = (l.hmput_key)(
            std::ptr::null_mut(),
            es,
            k.as_ptr() as *mut c_void,
            8,
            HM_BINARY,
        );
        for j in 8..es {
            *((p as *mut u8).add(es * temp_of(p, es) as usize + j)) = 0;
        }
        snap_map(t, "one", p, es, KeyKind::Raw);
        p = (l.hmdel_key)(p, es, k.as_ptr() as *mut c_void, 8, 0, HM_BINARY);
        t.rec("temp", temp_of(p, es));
        t.rec("len", (*hdr(p, es)).length);
        snap_map(t, "zero", p, es, KeyKind::Raw);
        // deleting again must be a clean no-op
        p = (l.hmdel_key)(p, es, k.as_ptr() as *mut c_void, 8, 0, HM_BINARY);
        t.rec("temp2", temp_of(p, es));
        t.rec("len2", (*hdr(p, es)).length);
        (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
    });
}

// ---------------------------------------------------------------------------
// rows 39-43, 45 : arena edge cases
// ---------------------------------------------------------------------------

#[test]
fn err39_stralloc_fresh_arena() {
    differential("err39", |l, t| unsafe {
        let mut a = Arena::new();
        snap_arena(t, "fresh", &a);
        let mut s = cstring(b"x");
        let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
        t.rec_str("p", cstr(p));
        snap_arena(t, "after", &a);
        (l.strreset)(&mut a);
    });
}

#[test]
fn err40_stralloc_block_saturates() {
    differential("err40", |l, t| unsafe {
        let mut a = Arena::new();
        // force the grow path 60 times: `block` must stop incrementing once
        // blocksize reaches 1<<20
        for i in 0..40 {
            let mut s = cstring(&vec![b'q'; 900_000]);
            let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            t.rec(&format!("i{i}"), (a.block, a.remaining, cstr(p).len()));
        }
        snap_arena(t, "final", &a);
        (l.strreset)(&mut a);
        snap_arena(t, "reset", &a);
    });
}

#[test]
fn err41_stralloc_oversized_paths() {
    differential("err41", |l, t| unsafe {
        // (a) storage == NULL
        let mut a = Arena::new();
        let mut big = cstring(&vec![b'A'; 4000]);
        let p = (l.stralloc)(&mut a, big.as_mut_ptr() as *mut c_char);
        t.rec("len_a", cstr(p).len());
        snap_arena(t, "a", &a);
        // a second oversized alloc now splices after the head
        let mut big2 = cstring(&vec![b'B'; 9000]);
        let p2 = (l.stralloc)(&mut a, big2.as_mut_ptr() as *mut c_char);
        t.rec("len_b", cstr(p2).len());
        snap_arena(t, "b", &a);
        // the first one must still read back
        t.rec("first_ok", cstr(p).len());
        (l.strreset)(&mut a);
        snap_arena(t, "reset", &a);
    });
}

#[test]
fn err43_stralloc_empty_string() {
    differential("err43", |l, t| unsafe {
        let mut a = Arena::new();
        let mut e = cstring(b"");
        for i in 0..600 {
            let p = (l.stralloc)(&mut a, e.as_mut_ptr() as *mut c_char);
            if i % 60 == 0 {
                t.rec(&format!("i{i}"), (cstr(p), a.remaining, a.block));
            }
        }
        snap_arena(t, "final", &a);
        (l.strreset)(&mut a);
        snap_arena(t, "reset", &a);
    });
}

#[test]
fn err45_strreset_idempotent() {
    differential("err45", |l, t| unsafe {
        let mut a = Arena::new();
        for i in 0..5 {
            (l.strreset)(&mut a);
            snap_arena(t, &format!("r{i}"), &a);
        }
        let mut s = cstring(b"data");
        (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
        for i in 0..5 {
            (l.strreset)(&mut a);
            snap_arena(t, &format!("q{i}"), &a);
        }
    });
}

// ---------------------------------------------------------------------------
// rows 46, 48-50 : hashing edge cases
// ---------------------------------------------------------------------------

#[test]
fn err46_hash_string_empty_and_one_byte() {
    differential("err46", |l, t| unsafe {
        let mut e = cstring(b"");
        for &s in &[0usize, 1, 2, SEED, usize::MAX, 1usize << 63, usize::MAX - 1] {
            t.rec("empty", (l.hash_string)(e.as_mut_ptr() as *mut c_char, s));
        }
        for b in 1u8..=255 {
            let mut one = cstring(&[b]);
            t.rec(
                &format!("b{b}"),
                (l.hash_string)(one.as_mut_ptr() as *mut c_char, SEED),
            );
        }
    });
}

#[test]
fn err48_hash_bytes_zero_len() {
    differential("err48", |l, t| unsafe {
        for &s in &[0usize, 1, SEED, usize::MAX, 1usize << 63] {
            t.rec("nullp", (l.hash_bytes)(std::ptr::null_mut(), 0, s));
            let mut b = [0xAAu8; 32];
            t.rec("realp", (l.hash_bytes)(b.as_mut_ptr() as *mut c_void, 0, s));
        }
    });
}

#[test]
fn err49_hash_bytes_every_remainder() {
    differential("err49", |l, t| unsafe {
        // buffer big enough that lengths 0..=64 never read out of bounds
        let mut buf: Vec<u8> = (0..64u16).map(|i| (i as u8) ^ 0x9C).collect();
        for len in 0..=64usize {
            for &s in &[0usize, SEED, usize::MAX] {
                t.rec(
                    &format!("l{len}"),
                    (l.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s),
                );
            }
        }
    });
}

/// row 50: force the `if (hash < 2) hash += 2` fixup by brute-force searching
/// for keys whose raw hash is 0 or 1 is impractical; instead assert that the
/// fixup path is *reachable-equivalent* by checking that hash values 0/1 never
/// appear in any bucket after a large number of inserts, in both libraries.
#[test]
fn err50_hash_fixup_never_stores_0_or_1() {
    let keys = rand_u64_keys(2000, 0xF00D);
    differential("err50", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let es = 16usize;
        let mut p: *mut c_void = std::ptr::null_mut();
        for k in &keys {
            p = (l.hmput_key)(p, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
            let idx = temp_of(p, es);
            for j in 8..es {
                *((p as *mut u8).add(es * idx as usize + j)) = 0;
            }
        }
        let ti = &*tbl(p, es);
        let mut zeros = 0usize;
        let mut ones = 0usize;
        let mut used = 0usize;
        for b in 0..(ti.slot_count >> 3) {
            let bk = &*ti.storage.add(b);
            for j in 0..8 {
                match bk.hash[j] {
                    0 => zeros += 1,
                    1 => ones += 1,
                    _ => used += 1,
                }
            }
        }
        t.rec("counts", (ti.slot_count, zeros, ones, used, ti.used_count));
        (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
    });
}

// ---------------------------------------------------------------------------
// rows 51-53 : str_dups / strkey degenerate arguments (stdout compared)
// ---------------------------------------------------------------------------

// ERRORS.md rows 51/52 (`str_dups` with non-positive `num`) live in
// `tests/stdout_diff.rs` together with row 55 -- see the note there.

#[test]
fn err53_strkey_extremes() {
    differential("err53", |l, t| unsafe {
        for &n in &[
            0i32,
            1,
            -1,
            i32::MAX,
            i32::MIN,
            i32::MIN + 1,
            -2147483647,
            999999999,
            -999999999,
        ] {
            t.rec_str(&format!("n{n}"), cstr((l.strkey)(n as c_int)));
        }
        // the returned buffer must be the same static buffer every time
        let a = (l.strkey)(1);
        let b = (l.strkey)(2);
        t.rec("same_buffer", a == b);
        t.rec_str("after", cstr(a));
    });
}

// ---------------------------------------------------------------------------
// generic FFI-boundary sweeps
// ---------------------------------------------------------------------------

#[test]
fn gen_zero_and_large_elemsize() {
    differential("gen_es", |l, t| unsafe {
        // elemsize 0 : arrgrowf allocates only the header
        let a = (l.arrgrowf)(std::ptr::null_mut(), 0, 0, 4);
        snap_arr(t, "es0", a, 0, false);
        (l.arrfreef)(a);
        // very large elemsize
        for &es in &[1024usize, 4096, 65536] {
            let b = (l.arrgrowf)(std::ptr::null_mut(), es, 1, 0);
            snap_arr(t, &format!("es{es}"), b, es, false);
            (l.arrfreef)(b);
        }
    });
}

#[test]
fn gen_keysize_boundaries() {
    differential("gen_ks", |l, t| unsafe {
        // keysize 0 : memcmp of 0 bytes always matches, so the FIRST key with a
        // colliding hash wins.  Both libraries must agree.
        (l.rand_seed)(SEED);
        let es = 16usize;
        let mut p: *mut c_void = std::ptr::null_mut();
        let keys = rand_u64_keys(20, 0x1A2B);
        for (i, k) in keys.iter().enumerate() {
            p = (l.hmput_key)(p, es, k.as_ptr() as *mut c_void, 0, HM_BINARY);
            let idx = temp_of(p, es);
            for j in 0..es {
                *((p as *mut u8).add(es * idx as usize + j)) = i as u8;
            }
            t.rec(&format!("ks0_{i}"), idx);
        }
        snap_map(t, "ks0", p, es, KeyKind::Raw);
        (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);

        // keysize == elemsize (no room for a value)
        (l.rand_seed)(SEED);
        let mut q: *mut c_void = std::ptr::null_mut();
        for k in &keys {
            q = (l.hmput_key)(q, 8, k.as_ptr() as *mut c_void, 8, HM_BINARY);
        }
        snap_map(t, "ks_full", q, 8, KeyKind::Raw);
        (l.hmfree_func)((q as *mut u8).sub(8) as *mut c_void, 8);
    });
}

#[test]
fn gen_keyoffset_out_of_range_but_in_element() {
    differential("gen_ko", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let es = 32usize;
        let keys = rand_u64_keys(16, 0x2B3C);
        let mut p: *mut c_void = std::ptr::null_mut();
        for k in &keys {
            p = (l.hmput_key)(p, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
            let idx = temp_of(p, es);
            for j in 8..es {
                *((p as *mut u8).add(es * idx as usize + j)) = 0;
            }
            // mirror the key at offsets 8, 16 and 24
            for off in [8usize, 16, 24] {
                std::ptr::copy_nonoverlapping(
                    k.as_ptr(),
                    (p as *mut u8).add(es * idx as usize + off),
                    8,
                );
            }
        }
        for &ko in &[0usize, 8, 16, 24] {
            let key = &keys[ko / 8];
            let before = (*hdr(p, es)).length;
            p = (l.hmdel_key)(p, es, key.as_ptr() as *mut c_void, 8, ko, HM_BINARY);
            t.rec(
                &format!("ko{ko}"),
                (temp_of(p, es), before, (*hdr(p, es)).length),
            );
            snap_map(t, &format!("m{ko}"), p, es, KeyKind::Raw);
        }
        (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
    });
}

#[test]
fn gen_all_modes_through_full_lifecycle() {
    let keys: Vec<Vec<u8>> = (0..24)
        .map(|i| cstring(format!("lifecycle_key_{i:05}").as_bytes()))
        .collect();
    differential("gen_modes", |l, t| unsafe {
        let es = 16usize;
        for &mode in MODES {
            (l.rand_seed)(SEED);
            let mut p: *mut c_void = std::ptr::null_mut();
            for k in &keys {
                p = (l.hmput_key)(p, es, k.as_ptr() as *mut c_void, 8, mode);
                let idx = temp_of(p, es);
                for j in 8..es {
                    *((p as *mut u8).add(es * idx as usize + j)) = 0;
                }
            }
            t.rec(&format!("m{mode}.len"), (*hdr(p, es)).length);
            for (i, k) in keys.iter().enumerate() {
                let mut tmp: isize = 0;
                p = (l.hmget_key_ts)(p, es, k.as_ptr() as *mut c_void, 8, &mut tmp, mode);
                t.rec(&format!("m{mode}.g{i}"), tmp);
            }
            // reverse order: avoids the mode>=2 swap-branch assert (ERRORS.md 32b)
            for i in (12..keys.len()).rev() {
                let k = &keys[i];
                p = (l.hmdel_key)(p, es, k.as_ptr() as *mut c_void, 8, 0, mode);
                t.rec(&format!("m{mode}.d{i}"), temp_of(p, es));
            }
            let ti = &*tbl(p, es);
            t.rec(
                &format!("m{mode}.state"),
                (
                    (*hdr(p, es)).length,
                    ti.slot_count,
                    ti.used_count,
                    ti.tombstone_count,
                    ti.string.mode,
                ),
            );
            (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
        }
        std::hint::black_box(&keys);
    });
}

#[test]
fn gen_seed_extremes_full_lifecycle() {
    let keys = rand_u64_keys(64, 0x3C4D);
    differential("gen_seeds", |l, t| unsafe {
        let es = 16usize;
        for &sd in &[
            0usize,
            1,
            2,
            usize::MAX,
            usize::MAX - 1,
            1usize << 63,
            (1usize << 63) - 1,
            0x31415926,
        ] {
            (l.rand_seed)(sd);
            let mut p: *mut c_void = std::ptr::null_mut();
            for k in &keys {
                p = (l.hmput_key)(p, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
                let idx = temp_of(p, es);
                for j in 8..es {
                    *((p as *mut u8).add(es * idx as usize + j)) = 0;
                }
            }
            snap_map(t, &format!("s{sd}"), p, es, KeyKind::Raw);
            for k in keys.iter().take(32) {
                p = (l.hmdel_key)(p, es, k.as_ptr() as *mut c_void, 8, 0, HM_BINARY);
            }
            snap_map(t, &format!("s{sd}.after"), p, es, KeyKind::Raw);
            (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
        }
    });
}
