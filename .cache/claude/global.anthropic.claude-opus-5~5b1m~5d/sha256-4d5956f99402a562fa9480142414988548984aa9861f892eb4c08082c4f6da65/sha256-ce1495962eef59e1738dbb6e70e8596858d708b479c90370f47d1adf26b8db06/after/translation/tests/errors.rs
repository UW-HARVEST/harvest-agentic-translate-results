//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Rows whose trigger makes the C `assert()` fire terminate the process, so
//! those are run in a *subprocess* (once per library) and compared by exit
//! signal.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::os::unix::process::ExitStatusExt;

// ---------------------------------------------------------------------------
// subprocess plumbing for the abort rows
// ---------------------------------------------------------------------------

fn run_scenario(scenario: &str, lib: &str) -> std::process::Output {
    let exe = std::env::current_exe().expect("current_exe");
    std::process::Command::new(exe)
        .args([
            "zzz_abort_worker",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("HARVEST_SCENARIO", scenario)
        .env("HARVEST_LIB", lib)
        .output()
        .expect("spawn worker")
}

/// Assert both libraries die the same way for `scenario`.
fn assert_same_abort(scenario: &str) {
    let c = run_scenario(scenario, "c");
    let r = run_scenario(scenario, "r");
    let sig = |o: &std::process::Output| o.status.signal();
    let code = |o: &std::process::Output| o.status.code();
    let saw_no_abort = |o: &std::process::Output| {
        String::from_utf8_lossy(&o.stdout).contains("HARVEST_NO_ABORT")
            || String::from_utf8_lossy(&o.stderr).contains("HARVEST_NO_ABORT")
    };
    assert!(
        !saw_no_abort(&c),
        "scenario {}: C did NOT abort\nstdout: {}\nstderr: {}",
        scenario,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&c.stderr)
    );
    assert!(
        !saw_no_abort(&r),
        "scenario {}: Rust did NOT abort\nstdout: {}\nstderr: {}",
        scenario,
        String::from_utf8_lossy(&r.stdout),
        String::from_utf8_lossy(&r.stderr)
    );
    assert_eq!(
        sig(&c),
        Some(6),
        "scenario {}: C should die with SIGABRT, got signal={:?} code={:?}\nstderr: {}",
        scenario,
        sig(&c),
        code(&c),
        String::from_utf8_lossy(&c.stderr)
    );
    assert_eq!(
        sig(&r),
        Some(6),
        "scenario {}: Rust should die with SIGABRT, got signal={:?} code={:?}\nstderr: {}",
        scenario,
        sig(&r),
        code(&r),
        String::from_utf8_lossy(&r.stderr)
    );
}

/// The worker.  Does nothing unless `HARVEST_SCENARIO` is set, so it is a
/// harmless no-op during a normal test run.
#[test]
fn zzz_abort_worker() {
    let Ok(scenario) = std::env::var("HARVEST_SCENARIO") else {
        return;
    };
    let which = std::env::var("HARVEST_LIB").unwrap_or_else(|_| "c".into());
    let p = pair();
    let api = if which == "c" { &p.c } else { &p.r };
    reseed(DEFAULT_SEED);
    unsafe {
        (api.rand_seed)(DEFAULT_SEED);
        match scenario.as_str() {
            // ERRORS.md #32/#33: with `mode != STBDS_HM_STRING` the swap-with-last
            // re-lookup passes the ADDRESS of the moved element instead of the
            // stored `char *`, so `stbds_hm_find_slot` cannot find it and
            // `STBDS_ASSERT(slot >= 0)` fires (lib.c:846).
            "hmdel_mode2_swap" | "hmdel_modemax_swap" => {
                let mode: c_int = if scenario == "hmdel_mode2_swap" {
                    2
                } else {
                    c_int::MAX
                };
                let elemsize = 16usize;
                let mut t = (api.shmode_func)(elemsize, SH_STRDUP as c_int);
                let mut keys: Vec<Vec<u8>> = (0..5)
                    .map(|i| format!("abort_key_{}\0", i).into_bytes())
                    .collect();
                for k in keys.iter_mut() {
                    t = (api.hmput_key)(t, elemsize, k.as_mut_ptr() as *mut c_void, 8, mode);
                }
                // deleting the FIRST key forces old_index != final_index
                t = (api.hmdel_key)(
                    t,
                    elemsize,
                    keys[0].as_mut_ptr() as *mut c_void,
                    8,
                    0,
                    mode,
                );
                let _ = t;
            }
            other => panic!("unknown scenario {}", other),
        }
    }
    println!("HARVEST_NO_ABORT");
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

unsafe fn header_of(a: *mut c_void) -> ArrayHeader {
    *((a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)
}

unsafe fn table_of(t: *mut c_void, elemsize: usize) -> Option<HashIndex> {
    if t.is_null() {
        return None;
    }
    let h = header_of((t as *mut u8).sub(elemsize) as *mut c_void);
    if h.hash_table.is_null() {
        None
    } else {
        Some(*(h.hash_table as *mut HashIndex))
    }
}

// =========================================================================
// rows 1..5 — stbds_arrgrowf
// =========================================================================

#[test]
fn e01_arrgrowf_no_grow() {
    let p = pair();
    unsafe {
        for &elemsize in &[1usize, 8, 16, 24] {
            let mut ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8);
            let mut ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8);
            let hc = header_of(ac);
            let hr = header_of(ar);
            for &mc in &[0usize, 1, 4, 7, 8] {
                let bc = ac;
                let br = ar;
                ac = (p.c.arrgrowf)(ac, elemsize, 0, mc);
                ar = (p.r.arrgrowf)(ar, elemsize, 0, mc);
                assert_eq!(ac, bc, "C must return the input for min_cap={}", mc);
                assert_eq!(ar, br, "Rust must return the input for min_cap={}", mc);
                assert_eq!(header_of(ac).capacity, hc.capacity);
                assert_eq!(header_of(ar).capacity, hr.capacity);
                assert_eq!(header_of(ac).length, hc.length);
                assert_eq!(header_of(ar).length, hr.length);
            }
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

#[test]
fn e02_arrgrowf_null_input() {
    let p = pair();
    unsafe {
        for &elemsize in &[1usize, 8, 16, 64] {
            for &min_cap in &[1usize, 2, 4, 5, 100] {
                let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
                let hc = header_of(ac);
                let hr = header_of(ar);
                assert_eq!(hc.length, 0);
                assert_eq!(hr.length, 0);
                assert!(hc.hash_table.is_null());
                assert!(hr.hash_table.is_null());
                assert_eq!(hc.temp, 0);
                assert_eq!(hr.temp, 0);
                assert_eq!(hc.capacity, min_cap.max(4));
                assert_eq!(hr.capacity, min_cap.max(4));
                (p.c.arrfreef)(ac);
                (p.r.arrfreef)(ar);
            }
        }
    }
}

#[test]
fn e03_arrgrowf_zero_elemsize() {
    let p = pair();
    unsafe {
        for &min_cap in &[1usize, 4, 100] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            assert_eq!(header_of(ac).capacity, min_cap.max(4));
            assert_eq!(header_of(ar).capacity, min_cap.max(4));
            assert_eq!(header_of(ac).length, 0);
            assert_eq!(header_of(ar).length, 0);
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

#[test]
fn e04_arrgrowf_zero_zero() {
    // min_len == 0 and min_cap == 0, so `min_cap <= stbds_arrcap(NULL) == 0`
    // hits FIRST and the function returns the input pointer, i.e. NULL.
    // The `min_cap < 4` branch is NOT reached.
    let p = pair();
    unsafe {
        for &elemsize in &[0usize, 1, 8, 16, 64] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(ac.is_null(), "C: arrgrowf(NULL,{},0,0) must return NULL", elemsize);
            assert!(ar.is_null(), "Rust: arrgrowf(NULL,{},0,0) must return NULL", elemsize);
        }
    }
}

#[test]
fn e05_arrgrowf_overflow_capacity() {
    let p = pair();
    unsafe {
        // elemsize*min_cap wraps to exactly 0, so the allocation is just the
        // 32-byte header; both implementations must compute the same size and
        // the same `capacity` field.
        for &(elemsize, min_cap) in &[
            (1usize << 62, 4usize),
            (1usize << 63, 2usize),
            (1usize << 63, 4usize),
            (1usize << 32, 1usize << 32),
        ] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, min_cap);
            assert!(!ac.is_null() && !ar.is_null());
            let hc = header_of(ac);
            let hr = header_of(ar);
            assert_eq!(hc.capacity, hr.capacity, "wrapped capacity must agree");
            assert_eq!(hc.length, hr.length);
            assert_eq!(hc.temp, hr.temp);
            assert_eq!(hc.hash_table.is_null(), hr.hash_table.is_null());
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

// =========================================================================
// rows 6..7 — stbds_hmfree_func null / table-less
// =========================================================================

#[test]
fn e06_hmfree_null() {
    let p = pair();
    unsafe {
        for &elemsize in &[0usize, 1, 8, 16, 64, usize::MAX] {
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

#[test]
fn e07_hmfree_no_table() {
    let p = pair();
    unsafe {
        for &elemsize in &[1usize, 8, 16, 64] {
            let tc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let tr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
            assert!(table_of(tc, elemsize).is_none());
            assert!(table_of(tr, elemsize).is_none());
            (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// =========================================================================
// rows 8..9 — stbds_hm_find_slot misses (both bucket halves)
// =========================================================================

#[test]
fn e08_find_slot_miss_upper() {
    let mut rng = Rng::new(0xE008);
    // vary the fill level so misses land in both the upper and the wrapped
    // half of a bucket
    for n in [1usize, 3, 5, 6, 7, 8, 13, 30, 100] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(16, KeyRepr::Raw, format!("miss n={}", n)).with_value_offset(8);
            let mut present = std::collections::HashSet::new();
            for i in 0..n {
                let mut k = rng.bytes(8);
                k[0] = i as u8;
                present.insert(k.clone());
                same_idx("miss", mp.put(&mut k, 8, HM_BINARY));
            }
            for _ in 0..400 {
                let mut k = rng.bytes(8);
                if present.contains(&k) {
                    continue;
                }
                assert_eq!(same_idx("miss", mp.get(&mut k, 8, HM_BINARY)), -1);
                assert_eq!(same_idx("miss", mp.get_ts(&mut k, 8, HM_BINARY)), -1);
                assert_eq!(same_idx("miss", mp.del(&mut k, 8, 0, HM_BINARY)), 0);
            }
            mp.free();
        }
    }
}

#[test]
fn e09_find_slot_miss_wrapped() {
    // Same rejection, driven through string keys (the strcmp comparison path)
    // and through maps whose buckets are mostly full so that probing wraps.
    let mut rng = Rng::new(0xE009);
    for n in [6usize, 7, 12, 13, 24, 50] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(16, KeyRepr::Pointer, format!("miss-str n={}", n))
                .with_value_offset(8);
            mp.shmode(SH_STRDUP as c_int);
            mp.put_default();
            let mut keys: Vec<Vec<u8>> =
                (0..n).map(|i| format!("present_{}\0", i).into_bytes()).collect();
            for k in keys.iter_mut() {
                same_idx("miss-str", mp.put(k, 8, HM_STRING));
            }
            for j in 0..400 {
                let l = 1 + rng.below(30);
                let mut k = rng.cstring(l, ASCII);
                if keys.contains(&k) {
                    continue;
                }
                assert_eq!(same_idx("miss-str", mp.get(&mut k, 8, HM_STRING)), -1, "j={}", j);
                assert_eq!(same_idx("miss-str", mp.del(&mut k, 8, 0, HM_STRING)), 0);
            }
            mp.free();
        }
    }
}

// =========================================================================
// rows 10..13 — stbds_hmget_key(_ts) rejections
// =========================================================================

#[test]
fn e10_hmget_ts_null_map() {
    let p = pair();
    unsafe {
        for &elemsize in &[1usize, 8, 16, 64] {
            let mut k = vec![9u8; 8];
            let mut tc: isize = 0x1234;
            let mut tr: isize = 0x1234;
            let rc = (p.c.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                k.as_mut_ptr() as *mut c_void,
                8,
                &mut tc,
                HM_BINARY,
            );
            let rr = (p.r.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                k.as_mut_ptr() as *mut c_void,
                8,
                &mut tr,
                HM_BINARY,
            );
            assert_eq!(tc, -1, "C *temp");
            assert_eq!(tr, -1, "Rust *temp");
            assert_eq!(
                snap_map(rc, elemsize, KeyRepr::Raw),
                snap_map(rr, elemsize, KeyRepr::Raw)
            );
            assert_eq!(header_of((rc as *mut u8).sub(elemsize) as *mut c_void).length, 1);
            (p.c.hmfree_func)((rc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((rr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e11_hmget_ts_no_table() {
    let p = pair();
    unsafe {
        for &elemsize in &[8usize, 16, 64] {
            let tc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let tr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
            let mut k = vec![3u8; 8];
            let mut a: isize = 77;
            let mut b: isize = 77;
            let rc = (p.c.hmget_key_ts)(tc, elemsize, k.as_mut_ptr() as *mut c_void, 8, &mut a, HM_BINARY);
            let rr = (p.r.hmget_key_ts)(tr, elemsize, k.as_mut_ptr() as *mut c_void, 8, &mut b, HM_BINARY);
            assert_eq!(a, -1);
            assert_eq!(b, -1);
            assert_eq!(rc, tc, "must return `a` unchanged");
            assert_eq!(rr, tr, "must return `a` unchanged");
            (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e12_hmget_ts_missing_key() {
    let mut rng = Rng::new(0xE012);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "ts-missing").with_value_offset(8);
        let mut present = std::collections::HashSet::new();
        for _ in 0..40 {
            let mut k = rng.bytes(8);
            present.insert(k.clone());
            same_idx("ts-missing", mp.put(&mut k, 8, HM_BINARY));
        }
        for _ in 0..200 {
            let mut k = rng.bytes(8);
            if present.contains(&k) {
                continue;
            }
            assert_eq!(same_idx("ts-missing", mp.get_ts(&mut k, 8, HM_BINARY)), -1);
        }
        mp.free();
    }
}

#[test]
fn e13_hmget_key_missing() {
    let mut rng = Rng::new(0xE013);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "get-missing").with_value_offset(8);
        let mut present = std::collections::HashSet::new();
        for _ in 0..40 {
            let mut k = rng.bytes(8);
            present.insert(k.clone());
            same_idx("get-missing", mp.put(&mut k, 8, HM_BINARY));
        }
        for _ in 0..200 {
            let mut k = rng.bytes(8);
            if present.contains(&k) {
                continue;
            }
            // hmget_key stores the sentinel in header->temp
            assert_eq!(same_idx("get-missing", mp.get(&mut k, 8, HM_BINARY)), -1);
            let (a, b) = mp.temps();
            assert_eq!(a, -1);
            assert_eq!(b, -1);
        }
        mp.free();
    }
}

// =========================================================================
// rows 14..16 — stbds_hmput_default
// =========================================================================

#[test]
fn e14_hmput_default_null() {
    let p = pair();
    unsafe {
        for &elemsize in &[1usize, 8, 12, 16, 64] {
            let tc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let tr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
            let sc = snap_map(tc, elemsize, KeyRepr::Raw);
            assert_eq!(sc, snap_map(tr, elemsize, KeyRepr::Raw));
            assert_eq!(sc.length, 1);
            assert!(!sc.has_table);
            assert!(sc.elems[0].iter().all(|&b| b == 0), "default element is memset to 0");
            (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e15_hmput_default_len0() {
    let p = pair();
    unsafe {
        for &elemsize in &[8usize, 16, 24] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            assert_eq!(header_of(ac).length, 0);
            let tc = (p.c.hmput_default)((ac as *mut u8).add(elemsize) as *mut c_void, elemsize);
            let tr = (p.r.hmput_default)((ar as *mut u8).add(elemsize) as *mut c_void, elemsize);
            let sc = snap_map(tc, elemsize, KeyRepr::Raw);
            assert_eq!(sc, snap_map(tr, elemsize, KeyRepr::Raw));
            assert_eq!(sc.length, 1);
            (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e16_hmput_default_noop() {
    let p = pair();
    unsafe {
        for &elemsize in &[8usize, 16, 24] {
            let tc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let tr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
            for _ in 0..5 {
                let c2 = (p.c.hmput_default)(tc, elemsize);
                let r2 = (p.r.hmput_default)(tr, elemsize);
                assert_eq!(c2, tc, "C must return `a` unchanged");
                assert_eq!(r2, tr, "Rust must return `a` unchanged");
                assert_eq!(header_of((tc as *mut u8).sub(elemsize) as *mut c_void).length, 1);
                assert_eq!(header_of((tr as *mut u8).sub(elemsize) as *mut c_void).length, 1);
            }
            (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// =========================================================================
// rows 17..25 — stbds_hmput_key
// =========================================================================

#[test]
fn e17_hmput_key_null_map() {
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for &elemsize in &[8usize, 16, 24] {
            let mut mp = MapPair::new(elemsize, KeyRepr::Raw, "put-null").with_value_offset(8.min(elemsize));
            let mut k = vec![7u8; 8];
            let idx = same_idx("put-null", mp.put(&mut k, 8, HM_BINARY));
            assert_eq!(idx, 0, "the first inserted entry has index 0");
            let sc = snap_map(mp.tc, elemsize, KeyRepr::Raw);
            assert_eq!(sc.length, 2, "default element + one entry");
            assert!(sc.has_table);
            mp.free();
        }
    }
}

#[test]
fn e18_hmput_key_first_table() {
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for &(mode, want_str_mode) in &[
            (HM_BINARY, SH_NONE),
            (-1i32, SH_NONE),
            (c_int::MIN, SH_NONE),
            (HM_STRING, SH_DEFAULT),
            (2i32, SH_DEFAULT),
            (c_int::MAX, SH_DEFAULT),
        ] {
            let repr = if want_str_mode == SH_DEFAULT {
                KeyRepr::Pointer
            } else {
                KeyRepr::Raw
            };
            let mut mp =
                MapPair::new(16, repr, format!("first-table mode={}", mode)).with_value_offset(8);
            let mut k = b"first_table_key\0".to_vec();
            same_idx("first-table", mp.put(&mut k, 8, mode));
            let t = table_of(mp.tc, 16).expect("table");
            let tr = table_of(mp.tr, 16).expect("table");
            assert_eq!(t.slot_count, 8);
            assert_eq!(tr.slot_count, 8);
            assert_eq!(t.string.mode, want_str_mode, "mode={}", mode);
            assert_eq!(tr.string.mode, want_str_mode, "mode={}", mode);
            mp.free();
        }
    }
}

#[test]
fn e19_hmput_key_growth() {
    let mut rng = Rng::new(0xE019);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "growth").with_value_offset(8);
        let mut slots = Vec::new();
        for i in 0..200 {
            let mut k = rng.bytes(8);
            k[0] = i as u8;
            k[1] = (i >> 8) as u8;
            same_idx("growth", mp.put(&mut k, 8, HM_BINARY));
            let t = table_of(mp.tc, 16).unwrap();
            let tr = table_of(mp.tr, 16).unwrap();
            assert_eq!(t.slot_count, tr.slot_count);
            assert_eq!(t.used_count, tr.used_count);
            assert_eq!(t.used_count_threshold, tr.used_count_threshold);
            assert!(t.used_count < t.used_count_threshold + 1);
            slots.push(t.slot_count);
        }
        // model of `slot_count = (table == NULL) ? 8 : slot_count*2` guarded by
        // `used_count >= used_count_threshold` (= slot_count - slot_count/4)
        let mut model = Vec::new();
        let mut slot = 0usize;
        let mut used = 0usize;
        for _ in 0..200 {
            if slot == 0 {
                slot = 8;
            } else if used >= slot - (slot >> 2) {
                slot *= 2;
            }
            used += 1;
            model.push(slot);
        }
        assert_eq!(slots, model, "table growth schedule");
        assert_eq!(slots[0], 8);
        assert_eq!(slots[5], 8);
        assert_eq!(slots[6], 16);
        assert_eq!(slots[11], 16);
        assert_eq!(slots[12], 32);
        mp.free();
    }
}

#[test]
fn e20_e21_hmput_duplicate_paths() {
    // `temp_key` is written on the "duplicate found in the UPPER half" path
    // (lib.c:733) but NOT on the wrapped-half path (lib.c:748).  Zeroing it
    // first in both libraries makes that difference observable, and the two
    // implementations must make the same choice for every key.
    let _g = seed_guard(DEFAULT_SEED);
    let mut rng = Rng::new(0xE020);
    let mut saw_written = false;
    let mut saw_untouched = false;
    unsafe {
        for round in 0..80 {
            // keep the map far below used_count_threshold(6) so that a
            // duplicate put cannot trigger a table rebuild
            let mut mp = MapPair::new(16, KeyRepr::Pointer, format!("dupkey {}", round))
                .with_value_offset(8);
            mp.shmode(SH_STRDUP as c_int);
            mp.put_default();
            let mut keys: Vec<Vec<u8>> = (0..4)
                .map(|_| {
                    let l = 1 + rng.below(12);
                    rng.cstring(l, ASCII)
                })
                .collect();
            keys.sort();
            keys.dedup();
            for k in keys.iter_mut() {
                same_idx("dupkey", mp.put(k, 8, HM_STRING));
            }
            let t = table_of(mp.tc, 16).unwrap();
            assert!(t.used_count < t.used_count_threshold, "no rebuild may happen");
            for k in keys.iter_mut() {
                mp.zero_temp_key();
                let idx = same_idx("dupkey", mp.put(k, 8, HM_STRING));
                assert!(idx >= 0);
                let kc = table_of(mp.tc, 16).unwrap().temp_key;
                let kr = table_of(mp.tr, 16).unwrap().temp_key;
                assert_eq!(
                    kc.is_null(),
                    kr.is_null(),
                    "temp_key written-ness must agree for {:?}",
                    show(k)
                );
                if kc.is_null() {
                    saw_untouched = true;
                } else {
                    saw_written = true;
                    assert_eq!(read_cstr(kc), read_cstr(kr));
                    assert_eq!(read_cstr(kc), k[..k.len() - 1].to_vec());
                }
            }
            mp.free();
        }
    }
    assert!(saw_written, "the upper-half duplicate path was never taken");
    // the wrapped-half path is rare; just report whether it was reached
    if !saw_untouched {
        eprintln!("note: the wrapped-half duplicate path was not reached in this run");
    }
}

#[test]
fn e22_hmput_reuse_tombstone() {
    let mut rng = Rng::new(0xE022);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "tombstone-reuse").with_value_offset(8);
        let mut keys: Vec<Vec<u8>> = (0..4)
            .map(|i| {
                let mut v = rng.bytes(8);
                v[0] = i as u8;
                v
            })
            .collect();
        for k in keys.iter_mut() {
            same_idx("tr", mp.put(k, 8, HM_BINARY));
        }
        let mut saw_reuse = false;
        for round in 0..40 {
            let i = round % keys.len();
            same_idx("tr", mp.del(&mut keys[i], 8, 0, HM_BINARY));
            let before = table_of(mp.tc, 16).unwrap();
            let before_r = table_of(mp.tr, 16).unwrap();
            assert_eq!(before.tombstone_count, before_r.tombstone_count);
            same_idx("tr", mp.put(&mut keys[i], 8, HM_BINARY));
            let after = table_of(mp.tc, 16).unwrap();
            let after_r = table_of(mp.tr, 16).unwrap();
            assert_eq!(after.tombstone_count, after_r.tombstone_count);
            assert_eq!(after.used_count, after_r.used_count);
            if before.tombstone_count > 0 && after.tombstone_count == before.tombstone_count - 1 {
                saw_reuse = true;
            }
        }
        assert!(saw_reuse, "the tombstone-reuse branch was never taken");
        mp.free();
    }
}

#[test]
fn e23_hmput_array_growth_assert() {
    // exercises `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` across many
    // capacity doublings; if it ever fired the process would die
    let mut rng = Rng::new(0xE023);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for &elemsize in &[8usize, 16, 24, 40] {
            let mut mp = MapPair::new(elemsize, KeyRepr::Raw, format!("arraygrow es={}", elemsize))
                .with_value_offset(8.min(elemsize));
            for i in 0..300u32 {
                let mut k = i.to_le_bytes().to_vec();
                k.extend_from_slice(&rng.bytes(4));
                same_idx("arraygrow", mp.put(&mut k, 8, HM_BINARY));
                let hc = header_of((mp.tc as *mut u8).sub(elemsize) as *mut c_void);
                let hr = header_of((mp.tr as *mut u8).sub(elemsize) as *mut c_void);
                assert!(hc.length <= hc.capacity);
                assert!(hr.length <= hr.capacity);
                assert_eq!(hc.length, hr.length);
                assert_eq!(hc.capacity, hr.capacity);
            }
            mp.free();
        }
    }
}

#[test]
fn e24_put_default_switch_branch() {
    // `string.mode` outside {1,2,3} takes `default: memcpy(elem, key, keysize)`
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for &sm in &[SH_NONE, 4u8, 17u8, 255u8] {
            for &keysize in &[1usize, 4, 8, 16] {
                let mut mp = MapPair::new(24, KeyRepr::Raw, format!("switch sm={}", sm))
                    .with_value_offset(keysize);
                mp.shmode(sm as c_int);
                mp.put_default();
                let mut k = b"switch_branch_key_0123456789\0".to_vec();
                let idx = same_idx("switch", mp.put(&mut k, keysize, HM_STRING));
                // the raw key bytes must be in the element, not a pointer
                let ec = std::slice::from_raw_parts(
                    (mp.tc as *const u8).add(idx as usize * 24),
                    keysize,
                );
                let er = std::slice::from_raw_parts(
                    (mp.tr as *const u8).add(idx as usize * 24),
                    keysize,
                );
                assert_eq!(ec, &k[..keysize], "C must memcpy the raw key bytes");
                assert_eq!(er, &k[..keysize], "Rust must memcpy the raw key bytes");
                mp.free();
            }
        }
    }
}

#[test]
fn e25_zero_keysize_binary() {
    let mut rng = Rng::new(0xE025);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for &elemsize in &[8usize, 16, 24] {
            let mut mp = MapPair::new(elemsize, KeyRepr::Raw, "ks0").with_value_offset(0);
            let mut first = None;
            for _ in 0..30 {
                let mut k = rng.bytes(8);
                let idx = same_idx("ks0", mp.put(&mut k, 0, HM_BINARY));
                match first {
                    None => first = Some(idx),
                    Some(f) => assert_eq!(idx, f, "memcmp(_,_,0)==0 collapses every key"),
                }
            }
            assert_eq!(
                header_of((mp.tc as *mut u8).sub(elemsize) as *mut c_void).length,
                2
            );
            mp.free();
        }
    }
}

// =========================================================================
// row 26 — stbds_shmode_func with out-of-range modes
// =========================================================================

#[test]
fn e26_shmode_out_of_range() {
    let p = pair();
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for &(mode, want) in &[
            (0i32, 0u8),
            (1, 1),
            (2, 2),
            (3, 3),
            (4, 4),
            (255, 255),
            (256, 0),
            (257, 1),
            (-1, 255),
            (-2, 254),
            (c_int::MIN, 0),
            (c_int::MAX, 255),
        ] {
            reseed(DEFAULT_SEED);
            let tc = (p.c.shmode_func)(16, mode);
            reseed(DEFAULT_SEED);
            let tr = (p.r.shmode_func)(16, mode);
            let a = table_of(tc, 16).unwrap();
            let b = table_of(tr, 16).unwrap();
            assert_eq!(a.string.mode, want, "C string.mode for mode={}", mode);
            assert_eq!(b.string.mode, want, "Rust string.mode for mode={}", mode);
            assert_eq!(
                snap_map(tc, 16, KeyRepr::Raw),
                snap_map(tr, 16, KeyRepr::Raw)
            );
            (p.c.hmfree_func)((tc as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((tr as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

// =========================================================================
// rows 27..36 — stbds_hmdel_key
// =========================================================================

#[test]
fn e27_hmdel_null_map() {
    let p = pair();
    unsafe {
        let mut k = vec![1u8; 8];
        for &elemsize in &[1usize, 8, 16, 64] {
            for &mode in &[HM_BINARY, HM_STRING, 2, -1] {
                let rc = (p.c.hmdel_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    8,
                    0,
                    mode,
                );
                let rr = (p.r.hmdel_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    8,
                    0,
                    mode,
                );
                assert!(rc.is_null(), "C must return 0");
                assert!(rr.is_null(), "Rust must return 0");
            }
        }
    }
}

#[test]
fn e28_hmdel_no_table() {
    let p = pair();
    unsafe {
        for &elemsize in &[8usize, 16, 24] {
            let tc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize);
            let tr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize);
            // poison temp so we can see it being set to 0
            for t in [tc, tr] {
                let h = (t as *mut u8).sub(elemsize).sub(HEADER_SIZE) as *mut ArrayHeader;
                (*h).temp = 0x5EED;
            }
            let mut k = vec![2u8; 8];
            let rc = (p.c.hmdel_key)(tc, elemsize, k.as_mut_ptr() as *mut c_void, 8, 0, HM_BINARY);
            let rr = (p.r.hmdel_key)(tr, elemsize, k.as_mut_ptr() as *mut c_void, 8, 0, HM_BINARY);
            assert_eq!(rc, tc);
            assert_eq!(rr, tr);
            let hc = header_of((tc as *mut u8).sub(elemsize) as *mut c_void);
            let hr = header_of((tr as *mut u8).sub(elemsize) as *mut c_void);
            assert_eq!(hc.temp, 0, "C must set temp = 0");
            assert_eq!(hr.temp, 0, "Rust must set temp = 0");
            assert_eq!(hc.length, 1);
            assert_eq!(hr.length, 1);
            (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e29_hmdel_missing_key() {
    let mut rng = Rng::new(0xE029);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "del-missing").with_value_offset(8);
        let mut present = std::collections::HashSet::new();
        for _ in 0..30 {
            let mut k = rng.bytes(8);
            present.insert(k.clone());
            same_idx("del-missing", mp.put(&mut k, 8, HM_BINARY));
        }
        let len = header_of((mp.tc as *mut u8).sub(16) as *mut c_void).length;
        for _ in 0..200 {
            let mut k = rng.bytes(8);
            if present.contains(&k) {
                continue;
            }
            let before = mp.tc;
            assert_eq!(same_idx("del-missing", mp.del(&mut k, 8, 0, HM_BINARY)), 0);
            assert_eq!(mp.tc, before, "must return `a` unchanged");
            assert_eq!(
                header_of((mp.tc as *mut u8).sub(16) as *mut c_void).length,
                len
            );
        }
        mp.free();
    }
}

#[test]
fn e30_e31_hmdel_slot_and_used_count_invariants() {
    let mut rng = Rng::new(0xE030);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "del-invariants").with_value_offset(8);
        let mut keys: Vec<Vec<u8>> = (0..120)
            .map(|i| {
                let mut v = rng.bytes(8);
                v[0] = i as u8;
                v[1] = (i >> 8) as u8;
                v
            })
            .collect();
        for k in keys.iter_mut() {
            same_idx("di", mp.put(k, 8, HM_BINARY));
        }
        for k in keys.iter_mut() {
            same_idx("di", mp.del(k, 8, 0, HM_BINARY));
            let t = table_of(mp.tc, 16).unwrap();
            let r = table_of(mp.tr, 16).unwrap();
            // `used_count` is size_t, so `used_count >= 0` (lib.c:832) can never
            // fire; what matters is that it never wraps below zero.
            assert!(t.used_count < (1usize << 62), "used_count wrapped: {}", t.used_count);
            assert!(r.used_count < (1usize << 62), "used_count wrapped: {}", r.used_count);
            assert_eq!(t.used_count, r.used_count);
            assert_eq!(t.slot_count, r.slot_count);
            assert!(t.slot_count >= 8);
        }
        // extra deletes of already-absent keys must not touch used_count
        let before = table_of(mp.tc, 16).unwrap().used_count;
        for k in keys.iter_mut() {
            assert_eq!(same_idx("di", mp.del(k, 8, 0, HM_BINARY)), 0);
        }
        assert_eq!(table_of(mp.tc, 16).unwrap().used_count, before);
        assert_eq!(table_of(mp.tr, 16).unwrap().used_count, before);
        mp.free();
    }
}

#[test]
fn e32_hmdel_swap_asserts() {
    // Deleting a non-last element runs the memmove + re-lookup path guarded by
    // `STBDS_ASSERT(slot >= 0)` and `STBDS_ASSERT(b->index[i] == final_index)`.
    let mut rng = Rng::new(0xE032);
    for &mode in &[HM_BINARY, HM_STRING] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let repr = if mode == HM_STRING {
                KeyRepr::Pointer
            } else {
                KeyRepr::Raw
            };
            let mut mp =
                MapPair::new(16, repr, format!("swap mode={}", mode)).with_value_offset(8);
            if mode == HM_STRING {
                mp.shmode(SH_STRDUP as c_int);
                mp.put_default();
            }
            let mut keys: Vec<Vec<u8>> = if mode == HM_STRING {
                (0..80).map(|i| format!("swap_{}\0", i).into_bytes()).collect()
            } else {
                (0..80)
                    .map(|i| {
                        let mut v = rng.bytes(8);
                        v[0] = i as u8;
                        v
                    })
                    .collect()
            };
            for k in keys.iter_mut() {
                same_idx("swap", mp.put(k, 8, mode));
            }
            // always delete index 0 -> guaranteed swap-with-last
            for i in 0..keys.len() {
                let mut k = keys[i].clone();
                assert_eq!(same_idx("swap", mp.del(&mut k, 8, 0, mode)), 1);
            }
            assert_eq!(header_of((mp.tc as *mut u8).sub(16) as *mut c_void).length, 1);
            mp.free();
        }
    }
}

#[test]
fn e33_hmdel_mode_two_no_free() {
    // The `free()` uses `mode == STBDS_HM_STRING`, so mode 2 skips it.  With a
    // swap-with-last that also makes the re-lookup take the wrong branch and
    // `STBDS_ASSERT(slot >= 0)` fires — identically in both implementations.
    assert_same_abort("hmdel_mode2_swap");
    assert_same_abort("hmdel_modemax_swap");

    // ... while the no-memmove order completes fine in both.
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for &mode in &[2i32, c_int::MAX] {
            let mut mp = MapPair::new(16, KeyRepr::Pointer, format!("m{}", mode))
                .with_value_offset(8);
            mp.shmode(SH_STRDUP as c_int);
            mp.put_default();
            let mut keys: Vec<Vec<u8>> =
                (0..20).map(|i| format!("nofree_{}\0", i).into_bytes()).collect();
            for k in keys.iter_mut() {
                same_idx("m", mp.put(k, 8, mode));
            }
            for i in (0..keys.len()).rev() {
                assert_eq!(same_idx("m", mp.del(&mut keys[i], 8, 0, mode)), 1);
            }
            mp.free();
        }
    }
}

#[test]
fn e34_hmdel_shrink() {
    let mut rng = Rng::new(0xE034);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "shrink").with_value_offset(8);
        let mut keys: Vec<Vec<u8>> = (0..60)
            .map(|i| {
                let mut v = rng.bytes(8);
                v[0] = i as u8;
                v
            })
            .collect();
        for k in keys.iter_mut() {
            same_idx("sh", mp.put(k, 8, HM_BINARY));
        }
        let start = table_of(mp.tc, 16).unwrap().slot_count;
        assert_eq!(start, 128, "60 entries live in a 128-slot table");
        let mut shrinks = Vec::new();
        for k in keys.iter_mut() {
            let before = table_of(mp.tc, 16).unwrap();
            same_idx("sh", mp.del(k, 8, 0, HM_BINARY));
            let after = table_of(mp.tc, 16).unwrap();
            let after_r = table_of(mp.tr, 16).unwrap();
            assert_eq!(after.slot_count, after_r.slot_count);
            if after.slot_count < before.slot_count {
                assert_eq!(after.slot_count, before.slot_count >> 1);
                shrinks.push((before.slot_count, after.slot_count));
            }
        }
        // every shrink halves the table, bottoming out at STBDS_BUCKET_LENGTH
        let mut want = Vec::new();
        let mut s = start;
        while s > 8 {
            want.push((s, s >> 1));
            s >>= 1;
        }
        assert_eq!(shrinks, want, "shrink chain");
        mp.free();
    }
}

#[test]
fn e35_hmdel_tombstone_rebuild() {
    let mut rng = Rng::new(0xE035);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "tombrebuild").with_value_offset(8);
        let mut keys: Vec<Vec<u8>> = (0..20)
            .map(|i| {
                let mut v = rng.bytes(8);
                v[0] = i as u8;
                v
            })
            .collect();
        for k in keys.iter_mut() {
            same_idx("tb", mp.put(k, 8, HM_BINARY));
        }
        let t0 = table_of(mp.tc, 16).unwrap();
        assert_eq!(t0.slot_count, 32);
        assert_eq!(t0.tombstone_count_threshold, 6);
        let mut rebuilds = 0;
        for k in keys.iter_mut() {
            let before = table_of(mp.tc, 16).unwrap();
            same_idx("tb", mp.del(k, 8, 0, HM_BINARY));
            let after = table_of(mp.tc, 16).unwrap();
            let after_r = table_of(mp.tr, 16).unwrap();
            assert_eq!(after.tombstone_count, after_r.tombstone_count);
            assert_eq!(after.slot_count, after_r.slot_count);
            if after.slot_count == before.slot_count
                && before.tombstone_count > 0
                && after.tombstone_count == 0
            {
                rebuilds += 1;
            }
        }
        assert!(rebuilds > 0, "no same-size tombstone rebuild happened");
        mp.free();
    }
}

#[test]
fn e36_hmdel_no_shrink_at_min() {
    let mut rng = Rng::new(0xE036);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "no-shrink").with_value_offset(8);
        let mut keys: Vec<Vec<u8>> = (0..5)
            .map(|i| {
                let mut v = rng.bytes(8);
                v[0] = i as u8;
                v
            })
            .collect();
        for k in keys.iter_mut() {
            same_idx("ns", mp.put(k, 8, HM_BINARY));
        }
        let t = table_of(mp.tc, 16).unwrap();
        assert_eq!(t.slot_count, 8);
        assert_eq!(
            t.used_count_shrink_threshold, 0,
            "slot_count <= STBDS_BUCKET_LENGTH forces the shrink threshold to 0"
        );
        for k in keys.iter_mut() {
            same_idx("ns", mp.del(k, 8, 0, HM_BINARY));
            assert_eq!(table_of(mp.tc, 16).unwrap().slot_count, 8);
            assert_eq!(table_of(mp.tr, 16).unwrap().slot_count, 8);
        }
        mp.free();
    }
}

// =========================================================================
// rows 37..38 — stbds_make_hash_index invariants
// =========================================================================

#[test]
fn e37_make_hash_index_threshold_invariant() {
    let mut rng = Rng::new(0xE037);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Raw, "invariant").with_value_offset(8);
        let mut seen = std::collections::HashSet::new();
        for i in 0..600u32 {
            let mut k = i.to_le_bytes().to_vec();
            k.extend_from_slice(&rng.bytes(4));
            same_idx("inv", mp.put(&mut k, 8, HM_BINARY));
            for t in [table_of(mp.tc, 16).unwrap(), table_of(mp.tr, 16).unwrap()] {
                assert!(
                    t.used_count_threshold + t.tombstone_count_threshold < t.slot_count,
                    "STBDS_ASSERT would fire for slot_count {}",
                    t.slot_count
                );
                assert_eq!(t.slot_count_log2, t.slot_count.trailing_zeros() as usize);
                seen.insert(t.slot_count);
            }
        }
        assert!(seen.contains(&8) && seen.contains(&1024), "slot counts seen: {:?}", seen);
        mp.free();
    }
}

#[test]
fn e38_hash_below_two_bias() {
    // `if (hash < 2) hash += 2;` guarantees a stored hash never collides with
    // STBDS_HASH_EMPTY (0) or STBDS_HASH_DELETED (1).
    let mut rng = Rng::new(0xE038);
    for seed in [0usize, 1, DEFAULT_SEED, usize::MAX] {
        let _g = seed_guard(seed);
        unsafe {
            let mut mp = MapPair::new(16, KeyRepr::Raw, format!("bias seed={:#x}", seed))
                .with_value_offset(8);
            for i in 0..300u32 {
                let mut k = i.to_le_bytes().to_vec();
                k.extend_from_slice(&rng.bytes(4));
                same_idx("bias", mp.put(&mut k, 8, HM_BINARY));
            }
            for (which, t) in [("C", mp.tc), ("Rust", mp.tr)] {
                let ti = table_of(t, 16).unwrap();
                for b in 0..(ti.slot_count >> 3) {
                    let bucket = *ti.storage.add(b);
                    for j in 0..8 {
                        if bucket.index[j] >= 0 {
                            assert!(
                                bucket.hash[j] >= 2,
                                "{}: in-use slot holds sentinel hash {}",
                                which,
                                bucket.hash[j]
                            );
                        }
                    }
                }
            }
            mp.free();
        }
    }
}

// =========================================================================
// rows 39..45 — string arena
// =========================================================================

#[test]
fn e39_stralloc_remaining_assert() {
    // The assert `len <= a->remaining` must hold on every path; if it ever
    // failed the process would abort.  Drive many shapes through it.
    let p = pair();
    let mut rng = Rng::new(0xE039);
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        for _ in 0..3000 {
            let len = match rng.below(6) {
                0 => 0,
                1 => 500 + rng.below(2000),
                _ => rng.below(200),
            };
            let mut s = rng.cstring(len, ASCII);
            let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert_eq!(read_cstr(pc), read_cstr(pr));
            assert_eq!(ac.remaining, ar.remaining);
            assert_eq!(ac.block, ar.block);
        }
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}

#[test]
fn e40_stralloc_oversized_first() {
    let p = pair();
    let mut rng = Rng::new(0xE040);
    unsafe {
        for &payload in &[512usize, 513, 1024, 4096] {
            let mut ac = StringArena::zeroed();
            let mut ar = StringArena::zeroed();
            let mut s = rng.cstring(payload, ASCII);
            let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert_eq!(read_cstr(pc), s[..payload].to_vec());
            assert_eq!(read_cstr(pr), s[..payload].to_vec());
            assert_eq!(ac.remaining, 0, "C remaining forced to 0");
            assert_eq!(ar.remaining, 0, "Rust remaining forced to 0");
            assert_eq!(ac.block, 1);
            assert_eq!(ar.block, 1);
            assert!(!ac.storage.is_null() && !ar.storage.is_null());
            // the returned pointer is the new block's storage (offset 8)
            assert_eq!(pc as usize, ac.storage as usize + 8);
            assert_eq!(pr as usize, ar.storage as usize + 8);
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
        }
    }
}

#[test]
fn e41_stralloc_oversized_after() {
    let p = pair();
    let mut rng = Rng::new(0xE041);
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        let mut small = rng.cstring(10, ASCII);
        (p.c.stralloc)(&mut ac, small.as_mut_ptr() as *mut c_char);
        (p.r.stralloc)(&mut ar, small.as_mut_ptr() as *mut c_char);
        let head_c = ac.storage;
        let head_r = ar.storage;
        let rem_c = ac.remaining;
        let rem_r = ar.remaining;
        let mut big = rng.cstring(4096, ASCII);
        let pc = (p.c.stralloc)(&mut ac, big.as_mut_ptr() as *mut c_char);
        let pr = (p.r.stralloc)(&mut ar, big.as_mut_ptr() as *mut c_char);
        assert_eq!(ac.storage, head_c, "head must not change");
        assert_eq!(ar.storage, head_r, "head must not change");
        assert_eq!(ac.remaining, rem_c, "remaining must be untouched");
        assert_eq!(ar.remaining, rem_r, "remaining must be untouched");
        // spliced in as head->next
        assert_eq!(pc as usize, *(head_c as *mut *mut u8) as usize + 8);
        assert_eq!(pr as usize, *(head_r as *mut *mut u8) as usize + 8);
        assert_eq!(read_cstr(pc), big[..4096].to_vec());
        assert_eq!(read_cstr(pr), big[..4096].to_vec());
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}

#[test]
fn e42_stralloc_empty_string() {
    let p = pair();
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        let mut e = vec![0u8];
        for i in 0..600 {
            let pc = (p.c.stralloc)(&mut ac, e.as_mut_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, e.as_mut_ptr() as *mut c_char);
            assert_eq!(read_cstr(pc), Vec::<u8>::new());
            assert_eq!(read_cstr(pr), Vec::<u8>::new());
            assert_eq!(ac.remaining, ar.remaining, "i={}", i);
            assert_eq!(ac.block, ar.block, "i={}", i);
        }
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}

#[test]
fn e43_stralloc_block_saturation() {
    let p = pair();
    let mut rng = Rng::new(0xE043);
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        for i in 0..27 {
            let bs = 512usize << (ac.block >> 1);
            let mut s = rng.cstring(bs + 100, ASCII);
            (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert_eq!(ac.block, ar.block, "i={}", i);
        }
        assert_eq!(ac.block, 22);
        assert_eq!(ar.block, 22);
        for _ in 0..3 {
            let mut s = rng.cstring((1 << 20) + 100, ASCII);
            (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            assert_eq!(ac.block, 22, "block must saturate");
            assert_eq!(ar.block, 22, "block must saturate");
        }
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}

#[test]
fn e44_strreset_empty() {
    let p = pair();
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        for _ in 0..5 {
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
            assert!(ac.storage.is_null() && ar.storage.is_null());
            assert_eq!((ac.remaining, ac.block, ac.mode), (0, 0, 0));
            assert_eq!((ar.remaining, ar.block, ar.mode), (0, 0, 0));
        }
        // a hand-poisoned `mode` field must also be zeroed
        ac.mode = 7;
        ar.mode = 7;
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
        assert_eq!(ac.mode, 0);
        assert_eq!(ar.mode, 0);
    }
}

#[test]
fn e45_strreset_chain() {
    let p = pair();
    let mut rng = Rng::new(0xE045);
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        for i in 0..400 {
            let len = if i % 17 == 0 { 4000 } else { 1 + (i % 60) };
            let mut s = rng.cstring(len, ASCII);
            (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
            (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
        }
        assert!(!ac.storage.is_null() && !ar.storage.is_null());
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
        assert_eq!((ac.remaining, ac.block, ac.mode, ac.storage.is_null()), (0, 0, 0, true));
        assert_eq!((ar.remaining, ar.block, ar.mode, ar.storage.is_null()), (0, 0, 0, true));
        // reuse behaves like a fresh arena
        let mut s = rng.cstring(10, ASCII);
        (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
        (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
        assert_eq!(ac.remaining, 512 - 11);
        assert_eq!(ar.remaining, 512 - 11);
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}

// =========================================================================
// rows 46..50 — hashing edge cases and out-of-range `mode`
// =========================================================================

#[test]
fn e46_hash_bytes_zero_len() {
    let p = pair();
    let mut rng = Rng::new(0xE046);
    unsafe {
        for _ in 0..200 {
            let seed = rng.next_u64() as usize;
            let a = (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let b = (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(a, b, "seed={:#x}", seed);
        }
        for seed in [0usize, 1, DEFAULT_SEED, usize::MAX] {
            assert_eq!(
                (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed),
                (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed)
            );
        }
    }
}

#[test]
fn e47_hash_bytes_tail_cases() {
    let p = pair();
    let mut rng = Rng::new(0xE047);
    unsafe {
        for len in 0..=7usize {
            for _ in 0..500 {
                let mut buf = rng.bytes(8);
                // hit case 4's `d[3] << 24` sign extension explicitly
                if rng.below(2) == 0 {
                    buf[3] |= 0x80;
                }
                let seed = rng.next_u64() as usize;
                let a = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                let b = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                assert_eq!(a, b, "len={} seed={:#x} buf={:02x?}", len, seed, buf);
            }
        }
    }
}

#[test]
fn e48_hash_string_empty() {
    let p = pair();
    let mut rng = Rng::new(0xE048);
    unsafe {
        let mut e = vec![0u8];
        for _ in 0..500 {
            let seed = rng.next_u64() as usize;
            assert_eq!(
                (p.c.hash_string)(e.as_mut_ptr() as *mut c_char, seed),
                (p.r.hash_string)(e.as_mut_ptr() as *mut c_char, seed),
                "seed={:#x}",
                seed
            );
        }
    }
}

#[test]
fn e49_hash_string_high_bytes() {
    let p = pair();
    let mut rng = Rng::new(0xE049);
    unsafe {
        for len in 1..=40usize {
            for _ in 0..40 {
                let mut s: Vec<u8> = (0..len).map(|_| 0x80 | (rng.next_u8() & 0x7f)).collect();
                s.push(0);
                let seed = rng.next_u64() as usize;
                assert_eq!(
                    (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
                    (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
                    "len={} seed={:#x}",
                    len,
                    seed
                );
            }
        }
    }
}

#[test]
fn e50_mode_out_of_range_enum() {
    // `mode` is a plain `int`, so any value can cross the FFI boundary.  The
    // branch taken is decided purely by `mode >= STBDS_HM_STRING (1)`.
    let mut rng = Rng::new(0xE050);
    let binary_modes = [c_int::MIN, -1000, -1, 0];
    let string_modes = [1i32, 2, 7, 1000, c_int::MAX];

    for &mode in &binary_modes {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(16, KeyRepr::Raw, format!("bin mode={}", mode))
                .with_value_offset(8);
            let mut keys: Vec<Vec<u8>> = (0..30)
                .map(|i| {
                    let mut v = rng.bytes(8);
                    v[0] = i as u8;
                    v
                })
                .collect();
            for k in keys.iter_mut() {
                assert!(same_idx("bm", mp.put(k, 8, mode)) >= 0);
            }
            // binary mode: the element holds the raw key bytes
            for (i, k) in keys.iter_mut().enumerate() {
                let idx = same_idx("bm", mp.get(k, 8, mode));
                assert_eq!(idx, i as isize);
            }
            for k in keys.iter_mut() {
                assert_eq!(same_idx("bm", mp.del(k, 8, 0, mode)), 1);
            }
            mp.free();
        }
    }

    for &mode in &string_modes {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(16, KeyRepr::Pointer, format!("str mode={}", mode))
                .with_value_offset(8);
            mp.shmode(SH_STRDUP as c_int);
            mp.put_default();
            let mut keys: Vec<Vec<u8>> = (0..30)
                .map(|i| format!("enum_mode_{}\0", i).into_bytes())
                .collect();
            for k in keys.iter_mut() {
                assert!(same_idx("sm", mp.put(k, 8, mode)) >= 0);
            }
            for k in keys.iter_mut() {
                assert!(same_idx("sm", mp.get(k, 8, mode)) >= 0);
            }
            // delete last-to-first: mode != 1 would otherwise abort (see e33)
            for i in (0..keys.len()).rev() {
                assert_eq!(same_idx("sm", mp.del(&mut keys[i], 8, 0, mode)), 1);
            }
            mp.free();
        }
    }
}

// =========================================================================
// rows 51..53 — strkey / sh_geti
// =========================================================================

#[test]
fn e51_strkey_extremes() {
    let _g = globals_guard();
    let p = pair();
    unsafe {
        for &n in &[0i32, 1, -1, 9, -9, 100, -100, i32::MAX, i32::MIN] {
            let a = read_cstr((p.c.strkey)(n));
            let b = read_cstr((p.r.strkey)(n));
            assert_eq!(show(&a), show(&b), "strkey({})", n);
            assert_eq!(show(&a), format!("test_{}", n));
            assert!(a.len() < 256, "must fit the 256-byte static buffer");
        }
        // pointer stability across calls
        let p1 = (p.c.strkey)(1);
        let p2 = (p.c.strkey)(2);
        assert_eq!(p1, p2);
        let r1 = (p.r.strkey)(1);
        let r2 = (p.r.strkey)(2);
        assert_eq!(r1, r2);
    }
}

#[test]
fn e52_sh_geti_non_positive() {
    let nums: Vec<c_int> = vec![0, -1, -2, -1000, c_int::MIN];
    let outs = sh_geti_diff(DEFAULT_SEED, &nums);
    for (i, &num) in nums.iter().enumerate() {
        assert!(outs[i].is_empty(), "sh_geti({}) must print nothing", num);
    }
}

#[test]
fn e53_sh_geti_asserts_hold() {
    // Every `STBDS_ASSERT` inside `sh_geti` must hold; if any fired the worker
    // subprocess would be killed by SIGABRT and `sh_geti_diff` would report a
    // non-successful exit status.
    let nums: Vec<c_int> = vec![1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233];
    let outs = sh_geti_diff(DEFAULT_SEED, &nums);
    for o in &outs {
        assert!(!o.is_empty());
    }
    // and with a variety of seeds
    let mut rng = Rng::new(0xE053);
    for _ in 0..6 {
        let seed = rng.next_u64() as usize;
        let outs = sh_geti_diff(seed, &nums);
        for o in &outs {
            assert!(!o.is_empty());
        }
    }
}

// =========================================================================
// rows 55..60
// =========================================================================

#[test]
fn e55_arrfreef_valid_only() {
    // `stbds_arrfreef(NULL)` would `free(NULL - 32)`, which is undefined
    // behaviour in the C original; both implementations compute the same
    // address and call the same `free`, so it is verified by inspection only.
    let p = pair();
    unsafe {
        for &elemsize in &[1usize, 8, 16, 64] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            assert_eq!(header_of(ac).capacity, 4);
            assert_eq!(header_of(ar).capacity, 4);
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

#[test]
fn e56_hmdel_bad_keyoffset() {
    let mut rng = Rng::new(0xE056);
    for &keyoffset in &[4usize, 8, 12, 16] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(32, KeyRepr::Raw, format!("badoff {}", keyoffset))
                .with_value_offset(8);
            let mut keys: Vec<Vec<u8>> = (0..25)
                .map(|i| {
                    let mut v = rng.bytes(8);
                    v[0] = i as u8;
                    v
                })
                .collect();
            for k in keys.iter_mut() {
                same_idx("bo", mp.put(k, 8, HM_BINARY));
            }
            let len = header_of((mp.tc as *mut u8).sub(32) as *mut c_void).length;
            for k in keys.iter_mut() {
                assert_eq!(
                    same_idx("bo", mp.del(k, 8, keyoffset, HM_BINARY)),
                    0,
                    "keyoffset {} must not find anything",
                    keyoffset
                );
            }
            assert_eq!(header_of((mp.tc as *mut u8).sub(32) as *mut c_void).length, len);
            assert_eq!(header_of((mp.tr as *mut u8).sub(32) as *mut c_void).length, len);
            mp.free();
        }
    }
}

#[test]
fn e57_keysize_equals_elemsize() {
    let mut rng = Rng::new(0xE057);
    for &elemsize in &[8usize, 16, 24, 32] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(elemsize, KeyRepr::Raw, format!("ks=es={}", elemsize))
                .with_value_offset(elemsize);
            let mut keys: Vec<Vec<u8>> = (0..40)
                .map(|i| {
                    let mut v = rng.bytes(elemsize);
                    v[0] = i as u8;
                    v
                })
                .collect();
            for k in keys.iter_mut() {
                same_idx("kses", mp.put(k, elemsize, HM_BINARY));
            }
            for (i, k) in keys.iter_mut().enumerate() {
                assert_eq!(same_idx("kses", mp.get(k, elemsize, HM_BINARY)), i as isize);
            }
            for k in keys.iter_mut() {
                assert_eq!(same_idx("kses", mp.del(k, elemsize, 0, HM_BINARY)), 1);
            }
            mp.free();
        }
    }
}

#[test]
fn e58_hmfree_strdup_empty() {
    let p = pair();
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for &elemsize in &[8usize, 16, 24, 64] {
            let tc = (p.c.shmode_func)(elemsize, SH_STRDUP as c_int);
            let tr = (p.r.shmode_func)(elemsize, SH_STRDUP as c_int);
            let sc = snap_map(tc, elemsize, KeyRepr::Pointer);
            assert_eq!(sc.length, 1, "only the default element exists");
            assert_eq!(sc, snap_map(tr, elemsize, KeyRepr::Pointer));
            (p.c.hmfree_func)((tc as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)((tr as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn e59_hmfree_arena() {
    let mut rng = Rng::new(0xE059);
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for n in [1usize, 5, 50] {
            let mut mp = MapPair::new(16, KeyRepr::Pointer, format!("free-arena n={}", n))
                .with_value_offset(8);
            mp.shmode(SH_ARENA as c_int);
            mp.put_default();
            let mut keys: Vec<Vec<u8>> = Vec::new();
            let mut seen = std::collections::HashSet::new();
            while keys.len() < n {
                let l = if keys.len() % 3 == 0 {
                    600 + rng.below(1000)
                } else {
                    1 + rng.below(40)
                };
                let k = rng.cstring(l, ASCII);
                if seen.insert(k.clone()) {
                    keys.push(k);
                }
            }
            for k in keys.iter_mut() {
                same_idx("fa", mp.put(k, 8, HM_STRING));
            }
            let t = table_of(mp.tc, 16).unwrap();
            let r = table_of(mp.tr, 16).unwrap();
            assert_eq!(t.string.mode, SH_ARENA);
            assert_eq!(r.string.mode, SH_ARENA);
            assert_eq!(t.string.remaining, r.string.remaining);
            assert_eq!(t.string.block, r.string.block);
            assert_eq!(t.string.storage.is_null(), r.string.storage.is_null());
            mp.free(); // strreset must free the whole chain, keys not freed twice
        }
    }
}

#[test]
fn e60_hmfree_default_mode() {
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        let mut mp = MapPair::new(16, KeyRepr::Pointer, "free-default").with_value_offset(8);
        mp.shmode(SH_DEFAULT as c_int);
        mp.put_default();
        let mut keys: Vec<Vec<u8>> = (0..50)
            .map(|i| format!("caller_owned_{}\0", i).into_bytes())
            .collect();
        for k in keys.iter_mut() {
            same_idx("fd", mp.put(k, 8, HM_STRING));
        }
        mp.free();
        // the caller's buffers must be untouched
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(
                show(&read_cstr(k.as_ptr() as *const c_char)),
                format!("caller_owned_{}", i)
            );
        }
    }
}

// =========================================================================
// Additional generic-boundary rows (ERRORS.md #61..#64)
// =========================================================================

/// #61 — `stbds_stralloc` with a hand-crafted `a->block`.
///
/// `blocksize = (size_t)512u << (a->block >> 1)` and `a->block` is an
/// `unsigned char`, so a caller can make the shift count as large as 127.
/// Shifts >= 64 are UB in C but on x86-64 the count is masked to 6 bits; the
/// Rust translation reproduces that with `wrapping_shl`.  Only block values
/// whose resulting blocksize is either small or wraps to 0 are exercised — the
/// values in between would ask `realloc` for terabytes and then dereference the
/// resulting NULL in BOTH implementations.
#[test]
fn e61_stralloc_handcrafted_block_counter() {
    let p = pair();
    let mut rng = Rng::new(0xE061);
    // (block >> 1) & 63 <= 11  =>  blocksize <= 1 MiB
    // (block >> 1) & 63 >= 55  =>  512 << k wraps to 0
    let mut blocks: Vec<u8> = (0u8..=23).collect();
    blocks.extend(110u8..=127); // k = 55..63
    blocks.extend(238u8..=255); // k = 119..127 -> masked to 55..63
    unsafe {
        for &b in &blocks {
            for &payload in &[0usize, 1, 10, 700, 5000] {
                let mut ac = StringArena::zeroed();
                let mut ar = StringArena::zeroed();
                ac.block = b;
                ar.block = b;
                let mut s = rng.cstring(payload, ASCII);
                let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
                let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
                assert_eq!(
                    read_cstr(pc),
                    s[..payload].to_vec(),
                    "C content for block={} payload={}",
                    b,
                    payload
                );
                assert_eq!(
                    read_cstr(pr),
                    s[..payload].to_vec(),
                    "Rust content for block={} payload={}",
                    b,
                    payload
                );
                assert_eq!(
                    (ac.remaining, ac.block, ac.mode, ac.storage.is_null()),
                    (ar.remaining, ar.block, ar.mode, ar.storage.is_null()),
                    "arena state diverged for block={} payload={}",
                    b,
                    payload
                );
                (p.c.strreset)(&mut ac);
                (p.r.strreset)(&mut ar);
            }
        }
    }
}

/// #62 — `stbds_shmode_func(0, mode)` / `stbds_hmfree_func(a, 0)`:
/// a zero `elemsize` makes `STBDS_ARR_TO_HASH` the identity.
#[test]
fn e62_zero_elemsize_map_lifecycle() {
    let p = pair();
    let _g = seed_guard(DEFAULT_SEED);
    unsafe {
        for &mode in &[0i32, 1, 2, 3, 4, 255] {
            reseed(DEFAULT_SEED);
            let tc = (p.c.shmode_func)(0, mode);
            reseed(DEFAULT_SEED);
            let tr = (p.r.shmode_func)(0, mode);
            let hc = header_of((tc as *mut u8) as *mut c_void);
            let hr = header_of((tr as *mut u8) as *mut c_void);
            assert_eq!(hc.length, 1);
            assert_eq!(hr.length, 1);
            assert_eq!(hc.capacity, hr.capacity);
            let a = table_of(tc, 0).unwrap();
            let b = table_of(tr, 0).unwrap();
            assert_eq!(a.slot_count, b.slot_count);
            assert_eq!(a.seed, b.seed);
            assert_eq!(a.string.mode, b.string.mode);
            assert_eq!(a.string.mode, (mode as u8));
            (p.c.hmfree_func)(tc, 0);
            (p.r.hmfree_func)(tr, 0);
        }
        // hmput_default / hmget_key_ts / hmdel_key with elemsize 0
        let tc = (p.c.hmput_default)(std::ptr::null_mut(), 0);
        let tr = (p.r.hmput_default)(std::ptr::null_mut(), 0);
        assert_eq!(header_of(tc).length, 1);
        assert_eq!(header_of(tr).length, 1);
        let mut k = vec![1u8; 8];
        let mut a: isize = 5;
        let mut b: isize = 5;
        let rc = (p.c.hmget_key_ts)(tc, 0, k.as_mut_ptr() as *mut c_void, 8, &mut a, HM_BINARY);
        let rr = (p.r.hmget_key_ts)(tr, 0, k.as_mut_ptr() as *mut c_void, 8, &mut b, HM_BINARY);
        assert_eq!(a, -1);
        assert_eq!(b, -1);
        assert_eq!(rc, tc);
        assert_eq!(rr, tr);
        let dc = (p.c.hmdel_key)(tc, 0, k.as_mut_ptr() as *mut c_void, 8, 0, HM_BINARY);
        let dr = (p.r.hmdel_key)(tr, 0, k.as_mut_ptr() as *mut c_void, 8, 0, HM_BINARY);
        assert_eq!(dc, tc);
        assert_eq!(dr, tr);
        assert_eq!(header_of(tc).temp, 0);
        assert_eq!(header_of(tr).temp, 0);
        (p.c.hmfree_func)(tc, 0);
        (p.r.hmfree_func)(tr, 0);
    }
}

/// #63 — exhaustive single-byte / two-byte inputs to `stbds_hash_bytes` and
/// `stbds_hash_string` (every possible byte value, including 0x00 and 0xFF).
#[test]
fn e63_hash_exhaustive_small_inputs() {
    let p = pair();
    unsafe {
        for seed in [0usize, 1, DEFAULT_SEED, usize::MAX] {
            for b0 in 0u8..=255 {
                let mut buf = [b0, 0, 0, 0, 0, 0, 0, 0];
                for len in 1..=8usize {
                    let a = (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    let c = (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed);
                    assert_eq!(a, c, "hash_bytes b0={} len={} seed={:#x}", b0, len, seed);
                }
                if b0 != 0 {
                    let mut s = [b0, 0];
                    let a = (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                    let c = (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed);
                    assert_eq!(a, c, "hash_string b0={} seed={:#x}", b0, seed);
                    for b1 in [1u8, 0x7f, 0x80, 0xff] {
                        let mut s2 = [b0, b1, 0];
                        let a = (p.c.hash_string)(s2.as_mut_ptr() as *mut c_char, seed);
                        let c = (p.r.hash_string)(s2.as_mut_ptr() as *mut c_char, seed);
                        assert_eq!(a, c, "hash_string {:?} seed={:#x}", (b0, b1), seed);
                    }
                }
                // every byte in every tail position
                for pos in 0..8usize {
                    let mut buf2 = [0u8; 8];
                    buf2[pos] = b0;
                    for len in (pos + 1)..=8usize {
                        let a = (p.c.hash_bytes)(buf2.as_mut_ptr() as *mut c_void, len, seed);
                        let c = (p.r.hash_bytes)(buf2.as_mut_ptr() as *mut c_void, len, seed);
                        assert_eq!(
                            a, c,
                            "hash_bytes pos={} b0={} len={} seed={:#x}",
                            pos, b0, len, seed
                        );
                    }
                }
            }
        }
    }
}

/// #64 — `stbds_hmget_key` / `stbds_hmput_key` / `stbds_hmdel_key` with a
/// `keysize` one step past the element size for the binary path.  The `memcmp`
/// then reads `keysize` bytes out of the element; both implementations must
/// perform the same read and reach the same conclusion.  `keysize` is kept
/// within the array's allocation (elements are contiguous) to stay inside
/// mapped memory.
#[test]
fn e64_keysize_one_past_element() {
    let mut rng = Rng::new(0xE064);
    for &(elemsize, keysize) in &[(16usize, 17usize), (16, 24), (24, 25), (32, 33)] {
        let _g = seed_guard(DEFAULT_SEED);
        unsafe {
            let mut mp = MapPair::new(
                elemsize,
                KeyRepr::Raw,
                format!("ks>es es={} ks={}", elemsize, keysize),
            )
            .with_value_offset(elemsize);
            // insert enough entries that element i+1 always exists in the
            // allocation, then only ever probe with the FIRST elements
            let mut keys: Vec<Vec<u8>> = (0..20)
                .map(|i| {
                    let mut v = rng.bytes(keysize);
                    v[0] = i as u8;
                    v
                })
                .collect();
            for k in keys.iter_mut() {
                assert!(same_idx("kspast", mp.put(k, keysize, HM_BINARY)) >= 0);
            }
            for k in keys.iter_mut() {
                // C and Rust must agree on found / not-found, whatever the
                // out-of-element bytes happen to be
                let a = mp.get(k, keysize, HM_BINARY);
                assert_eq!(a.0, a.1, "get result must agree");
                let b = mp.get_ts(k, keysize, HM_BINARY);
                assert_eq!(b.0, b.1, "get_ts result must agree");
            }
            mp.free();
        }
    }
}

/// #65 — layout parity of the private `stbds_hash_index` struct.
///
/// `stbds_make_hash_index` computes
/// `t->storage = STBDS_ALIGN_FWD((size_t)(t+1), 64)`, so the distance from the
/// struct base to the bucket array is a pure function of
/// `(t_addr % 64, sizeof(stbds_hash_index))`.  Collecting that function from
/// both libraries and comparing it proves the two structs have the same size
/// (the harness already proves the field *offsets* agree, since it reads
/// `slot_count` / `seed` / `string.mode` / `storage` out of the C struct with
/// the Rust definition and gets exactly the documented values).
#[test]
fn e65_hash_index_layout_parity() {
    let p = pair();
    let _g = seed_guard(DEFAULT_SEED);
    let mut map_c: std::collections::HashMap<usize, usize> = Default::default();
    let mut map_r: std::collections::HashMap<usize, usize> = Default::default();
    let expect_size = std::mem::size_of::<HashIndex>();
    unsafe {
        let mut keep = Vec::new();
        for i in 0..400 {
            let tc = (p.c.shmode_func)(16, SH_STRDUP as c_int);
            let tr = (p.r.shmode_func)(16, SH_STRDUP as c_int);
            for (which, t) in [(0usize, tc), (1usize, tr)] {
                let h = header_of((t as *mut u8).sub(16) as *mut c_void);
                let base = h.hash_table as usize;
                let ti = &*(h.hash_table as *mut HashIndex);
                let off = (ti.storage as usize) - base;
                let rem = base % 64;
                // the model, using the Rust-known sizeof
                let want = ((rem + expect_size + 63) & !63usize) - rem;
                assert_eq!(
                    off, want,
                    "iteration {} lib {}: storage offset {} does not match \
                     ALIGN_FWD(base+{},64) for base%64 == {}",
                    i, which, off, expect_size, rem
                );
                let m = if which == 0 { &mut map_c } else { &mut map_r };
                if let Some(prev) = m.insert(rem, off) {
                    assert_eq!(prev, off, "lib {}: inconsistent offset for rem {}", which, rem);
                }
            }
            keep.push((tc, tr));
        }
        // the observed (rem -> off) functions must be identical
        for (rem, off) in &map_c {
            if let Some(o) = map_r.get(rem) {
                assert_eq!(off, o, "layout mismatch for base%64 == {}", rem);
            }
        }
        assert!(!map_c.is_empty() && !map_r.is_empty());
        assert_eq!(expect_size, 104, "stbds_hash_index is 104 bytes on LP64");
        for (tc, tr) in keep {
            (p.c.hmfree_func)((tc as *mut u8).sub(16) as *mut c_void, 16);
            (p.r.hmfree_func)((tr as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

/// #66 — layout parity of `stbds_array_header` and `stbds_string_block`.
///
/// `stbds_header(t) == (stbds_array_header *) t - 1`, so the distance from the
/// header to the payload is `sizeof(stbds_array_header)`; `stbds_stralloc`
/// returns `sb->storage`, whose offset inside the block is
/// `offsetof(stbds_string_block, storage)`.  Both are checked against the C
/// library's actual behaviour.
#[test]
fn e66_header_and_block_layout_parity() {
    let p = pair();
    unsafe {
        assert_eq!(HEADER_SIZE, 32, "stbds_array_header is 32 bytes on LP64");
        // realloc'ed blocks are 16-byte aligned; the payload the C hands back is
        // exactly HEADER_SIZE past the malloc block, so payload % 16 == 0.
        for &elemsize in &[1usize, 8, 16] {
            for _ in 0..64 {
                let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8);
                let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8);
                assert_eq!(
                    (ac as usize) % 16,
                    0,
                    "C payload must be 16-byte aligned (header size 32)"
                );
                assert_eq!(
                    (ar as usize) % 16,
                    0,
                    "Rust payload must be 16-byte aligned (header size 32)"
                );
                (p.c.arrfreef)(ac);
                (p.r.arrfreef)(ar);
            }
        }
        // string block: sizeof == 16, offsetof(storage) == 8
        let mut s = b"layout\0".to_vec();
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        let pc = (p.c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
        let pr = (p.r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
        // first string in a fresh 512-byte block lands at storage + 512 - 7
        assert_eq!(
            pc as usize - ac.storage as usize,
            8 + 512 - 7,
            "C block layout (offsetof(storage) must be 8)"
        );
        assert_eq!(
            pr as usize - ar.storage as usize,
            8 + 512 - 7,
            "Rust block layout (offsetof(storage) must be 8)"
        );
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}
