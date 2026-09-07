//! Phase C — error / rejection-path differential tests, one per ERRORS.md row.
//! Aborting and crashing rows are run in forked child processes so the C and
//! the Rust failure mode (signal + glibc diagnostic) can be compared.

mod common;

use common::driver::*;
use common::*;
use std::ffi::{c_void, CString};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const SEED: u64 = 0xBADF00D;

fn libs() -> (Lib, Lib) {
    both()
}

fn k4(v: u32) -> Vec<u8> {
    v.to_le_bytes().to_vec()
}

unsafe fn grow_snap(a: *mut c_void) -> String {
    if a.is_null() {
        return "NULL".to_string();
    }
    let h = header(a);
    format!("len={} cap={} temp={} table_null={}", h.length, h.capacity, h.temp, h.hash_table.is_null())
}

unsafe fn free_if(f: ArrFreeF, a: *mut c_void) {
    if !a.is_null() {
        f(a);
    }
}

fn new_arena() -> StringArena {
    StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 }
}

// ==========================================================================
// crash / abort harness
// ==========================================================================

/// Runs `abort_helper` in a fresh child process with `ABORT_CASE` set.
fn run_case(which: &str, case: &str) -> (Option<i32>, Option<i32>, String) {
    let exe = std::env::current_exe().unwrap();
    let out = Command::new(exe)
        .args(["--exact", "abort_helper", "--nocapture", "--test-threads=1"])
        .env("ABORT_CASE", format!("{which}:{case}"))
        .output()
        .expect("failed to spawn abort helper");
    (
        out.status.code(),
        out.status.signal(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// The assertion diagnostic, with the (identical) program-name prefix stripped.
fn assert_line(stderr: &str) -> String {
    for l in stderr.lines() {
        if let Some(i) = l.find(": ") {
            if l.contains("Assertion") {
                return l[i + 2..].to_string();
            }
        }
        if l.contains("free(): ") || l.contains("munmap_chunk") || l.contains("Aborted") {
            return l.trim().to_string();
        }
    }
    String::new()
}

#[track_caller]
fn assert_same_failure(case: &str) {
    let (cc, cs, ce) = run_case("c", case);
    let (rc, rs, re) = run_case("rust", case);
    assert_eq!(cs, rs, "case {case}: signal differs (C {cs:?} vs Rust {rs:?})\nC stderr:\n{ce}\nRust stderr:\n{re}");
    assert_eq!(cc, rc, "case {case}: exit code differs");
    assert_eq!(
        assert_line(&ce),
        assert_line(&re),
        "case {case}: diagnostic differs\nC stderr:\n{ce}\nRust stderr:\n{re}"
    );
    assert!(
        cs.is_some(),
        "case {case}: expected a fatal signal, got exit code {cc:?}\n{ce}"
    );
}

/// The worker: only does something when `ABORT_CASE` is set, so a plain
/// `cargo test` run treats it as a trivially passing test.
#[test]
fn abort_helper() {
    let Ok(spec) = std::env::var("ABORT_CASE") else {
        return;
    };
    let (which, case) = spec.split_once(':').unwrap();
    let path = if which == "c" { c_so_path() } else { rust_so_path() };
    let l = unsafe { Lib::open("x", &path) };
    unsafe {
        match case {
            // ERRORS row 38 / 39
            "intput9" => (l.intput)(9),
            "intput11" => (l.intput)(11),
            // ERRORS row 41
            "arrfree_null" => (l.arrfreef)(std::ptr::null_mut()),
            // ERRORS row 42
            "get_ts_null_temp" => {
                let mut k = k4(7);
                (l.hmget_key_ts)(
                    std::ptr::null_mut(),
                    8,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    std::ptr::null_mut(),
                    0,
                );
            }
            // ERRORS rows 16 / 17
            "corrupt_key" => {
                (l.rand_seed)(0x31415926);
                let mut t: *mut c_void = std::ptr::null_mut();
                for i in 1..=5u32 {
                    let mut k = k4(i);
                    t = (l.hmput_key)(t, 8, k.as_mut_ptr() as *mut c_void, 4, 0);
                    let temp = map_header(t, 8).temp;
                    let e = (t as *mut u8).offset(temp * 8);
                    std::ptr::copy_nonoverlapping(k.as_ptr(), e, 4);
                    std::ptr::copy_nonoverlapping(k4(i * 100).as_ptr(), e.add(4), 4);
                }
                // scribble over the last element's key behind the library's back
                let last = (t as *mut u8).add(8 * 4);
                std::ptr::copy_nonoverlapping(k4(0xAAAA_AAAA).as_ptr(), last, 4);
                let mut k = k4(1);
                (l.hmdel_key)(t, 8, k.as_mut_ptr() as *mut c_void, 4, 0, 0);
            }
            // ERRORS row 24 (allocation-failure tail)
            "block_huge" => {
                let mut a = StringArena {
                    storage: std::ptr::null_mut(),
                    remaining: 0,
                    block: 200,
                    mode: 0,
                };
                let s = CString::new("xyz").unwrap();
                (l.stralloc)(&mut a as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            }
            // ERRORS row 44
            "del_mode2" => {
                (l.rand_seed)(0x31415926);
                let mut t = (l.shmode_func)(16, 2);
                let keys: Vec<CString> = (0..5)
                    .map(|i| CString::new(format!("key{i}")).unwrap())
                    .collect();
                for k in &keys {
                    t = (l.hmput_key)(t, 16, k.as_ptr() as *mut c_void, 8, 2);
                }
                (l.hmdel_key)(t, 16, keys[0].as_ptr() as *mut c_void, 8, 0, 2);
            }
            other => panic!("unknown ABORT_CASE {other}"),
        }
    }
    // If we get here the operation did not abort; report that as a clean exit.
    std::process::exit(0);
}

// ==========================================================================
// rows 1-4 : stbds_arrgrowf
// ==========================================================================

#[test]
fn err_01_arrgrowf_no_grow() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [1usize, 8, 40] {
            let ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
            for min_cap in [0usize, 1, 8, 15, 16] {
                let ca2 = (c.arrgrowf)(ca, elemsize, 0, min_cap);
                let ra2 = (r.arrgrowf)(ra, elemsize, 0, min_cap);
                assert_eq!(ca2, ca, "C must return `a` unchanged (min_cap={min_cap})");
                assert_eq!(ra2, ra, "Rust must return `a` unchanged (min_cap={min_cap})");
                assert_same(
                    &format!("no-grow min_cap={min_cap}"),
                    &grow_snap(ca2),
                    &grow_snap(ra2),
                );
            }
            free_if(c.arrfreef, ca);
            free_if(r.arrfreef, ra);
        }
    }
}

#[test]
fn err_02_arrgrowf_null_input() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [1usize, 4, 8, 40] {
            for (addlen, min_cap) in [(0usize, 1usize), (1, 0), (7, 0), (0, 100), (1000, 1)] {
                let ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                assert_same(
                    &format!("arrgrowf(NULL,{elemsize},{addlen},{min_cap})"),
                    &grow_snap(ca),
                    &grow_snap(ra),
                );
                assert_eq!(header(ca).length, 0);
                assert!(header(ca).hash_table.is_null());
                assert_eq!(header(ca).temp, 0);
                free_if(c.arrfreef, ca);
                free_if(r.arrfreef, ra);
            }
        }
    }
}

#[test]
fn err_03_arrgrowf_min_cap_floor() {
    let (c, r) = libs();
    unsafe {
        for min_cap in [1usize, 2, 3, 4, 5] {
            let ca = (c.arrgrowf)(std::ptr::null_mut(), 8, 0, min_cap);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), 8, 0, min_cap);
            let expect = if min_cap < 4 { 4 } else { min_cap };
            assert_eq!(header(ca).capacity, expect, "C floor at min_cap={min_cap}");
            assert_eq!(header(ra).capacity, expect, "Rust floor at min_cap={min_cap}");
            assert_same("cap floor", &grow_snap(ca), &grow_snap(ra));
            free_if(c.arrfreef, ca);
            free_if(r.arrfreef, ra);
        }
    }
}

#[test]
fn err_04_arrgrowf_elemsize_zero() {
    let (c, r) = libs();
    unsafe {
        for min_cap in [0usize, 1, 4, 1000, usize::MAX / 2] {
            let ca = (c.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            assert_same(
                &format!("arrgrowf(NULL,0,0,{min_cap})"),
                &grow_snap(ca),
                &grow_snap(ra),
            );
            free_if(c.arrfreef, ca);
            free_if(r.arrfreef, ra);
        }
    }
}

#[test]
fn err_43_arrgrowf_returns_null() {
    let (c, r) = libs();
    unsafe {
        // addlen == 0 && min_cap == 0 && a == NULL  =>  min_cap (0) <= arrcap (0)
        // so the function returns its NULL argument without allocating.
        for elemsize in [0usize, 1, 8, 40] {
            let ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(ca.is_null(), "C must return NULL for elemsize={elemsize}");
            assert!(ra.is_null(), "Rust must return NULL for elemsize={elemsize}");
        }
    }
}

// ==========================================================================
// rows 5-6 : stbds_hmfree_func
// ==========================================================================

#[test]
fn err_05_hmfree_null() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [0usize, 1, 8, 16] {
            (c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

#[test]
fn err_06_hmfree_no_table() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [8usize, 16] {
            let ca = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            assert!(header(ca).hash_table.is_null());
            assert!(header(ra).hash_table.is_null());
            (c.hmfree_func)(ca, elemsize);
            (r.hmfree_func)(ra, elemsize);
        }
    }
}

// ==========================================================================
// rows 8-11 : lookup misses
// ==========================================================================

#[test]
fn err_08_hmget_ts_null_map() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [8usize, 16, 24] {
            seed_both(&c, &r, 0x31415926);
            let mut k = vec![0u8; 8];
            let mut tc: isize = 0x5A5A;
            let mut tr: isize = 0x5A5A;
            let ct = (c.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                k.as_mut_ptr() as *mut c_void,
                4,
                &mut tc,
                0,
            );
            let rt = (r.hmget_key_ts)(
                std::ptr::null_mut(),
                elemsize,
                k.as_mut_ptr() as *mut c_void,
                4,
                &mut tr,
                0,
            );
            assert_eq!(tc, -1, "C: *temp must be STBDS_INDEX_EMPTY");
            assert_eq!(tr, -1, "Rust: *temp must be STBDS_INDEX_EMPTY");
            assert!(!ct.is_null() && !rt.is_null(), "a fresh map must be returned");
            assert_same(
                "hmget_key_ts(NULL)",
                &snapshot_map(ct, elemsize, 4, KeyKind::Inline),
                &snapshot_map(rt, elemsize, 4, KeyKind::Inline),
            );
            (c.hmfree_func)((ct as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((rt as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn err_09_hmget_ts_no_table() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "err09");
        p.put_default();
        let ct0 = p.ct;
        let rt0 = p.rt;
        for kv in [0u32, 1, 42, u32::MAX] {
            assert_eq!(p.get_bin_ts(&k4(kv)), -1);
        }
        assert_eq!(p.ct, ct0, "C must return `a` unchanged");
        assert_eq!(p.rt, rt0, "Rust must return `a` unchanged");
        p.check("no-table hmget_key_ts");
        p.free();
    }
}

#[test]
fn err_10_hmget_ts_missing_key() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 10);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "err10");
        let mut present = std::collections::HashSet::new();
        for i in 0..100u32 {
            let kv = rng.next_u32() | 1;
            present.insert(kv);
            p.put_bin(&k4(kv), &k4(i));
        }
        let ct0 = p.ct;
        let rt0 = p.rt;
        let mut misses = 0;
        for _ in 0..500 {
            let kv = rng.next_u32() & !1u32;
            if present.contains(&kv) {
                continue;
            }
            assert_eq!(p.get_bin_ts(&k4(kv)), -1, "absent key must give -1");
            misses += 1;
        }
        assert!(misses > 100);
        assert_eq!(p.ct, ct0);
        assert_eq!(p.rt, rt0);
        p.check("missing-key hmget_key_ts");
        p.free();
    }
}

#[test]
fn err_11_hmget_key_missing() {
    let (c, r) = libs();
    unsafe {
        // (a) NULL map
        seed_both(&c, &r, 0x31415926);
        let mut k = k4(3);
        let ct = (c.hmget_key)(std::ptr::null_mut(), 8, k.as_mut_ptr() as *mut c_void, 4, 0);
        let rt = (r.hmget_key)(std::ptr::null_mut(), 8, k.as_mut_ptr() as *mut c_void, 4, 0);
        assert_eq!(map_header(ct, 8).temp, -1, "C header temp");
        assert_eq!(map_header(rt, 8).temp, -1, "Rust header temp");
        assert_same(
            "hmget_key(NULL)",
            &snapshot_map(ct, 8, 4, KeyKind::Inline),
            &snapshot_map(rt, 8, 4, KeyKind::Inline),
        );
        (c.hmfree_func)((ct as *mut u8).sub(8) as *mut c_void, 8);
        (r.hmfree_func)((rt as *mut u8).sub(8) as *mut c_void, 8);

        // (b) no table  and  (c) populated table, absent key
        seed_both(&c, &r, 0x31415926);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "err11");
        p.put_default();
        assert_eq!(p.get_bin(&k4(5)), -1);
        let mut rng = Rng::new(SEED ^ 11);
        for i in 0..80u32 {
            p.put_bin(&k4(rng.next_u32() | 1), &k4(i));
        }
        for _ in 0..200 {
            let kv = rng.next_u32() & !1u32;
            let t = p.get_bin(&k4(kv));
            assert!(t == -1 || t >= 0);
        }
        p.check("hmget_key misses");
        p.free();
    }
}

// ==========================================================================
// rows 12-14 : delete rejections
// ==========================================================================

#[test]
fn err_12_hmdel_null_map() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [8usize, 16] {
            for mode in [-1i32, 0, 1, 2, i32::MAX, i32::MIN] {
                let mut k = k4(1);
                let ct = (c.hmdel_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                    mode,
                );
                let rt = (r.hmdel_key)(
                    std::ptr::null_mut(),
                    elemsize,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                    mode,
                );
                assert!(ct.is_null(), "C must return NULL (mode={mode})");
                assert!(rt.is_null(), "Rust must return NULL (mode={mode})");
            }
        }
    }
}

#[test]
fn err_13_hmdel_no_table() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "err13");
        p.put_default();
        let before = p.snap(p.ct);
        for kv in [0u32, 1, 99] {
            assert_eq!(p.del_bin(&k4(kv), 0), 0, "temp must be 0 for a table-less map");
        }
        assert_same("hmdel on table-less map", &before, &p.snap(p.ct));
        p.check("hmdel no table");
        p.free();
    }
}

#[test]
fn err_14_hmdel_missing_key() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 14);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "err14");
        let mut present = std::collections::HashSet::new();
        for i in 0..100u32 {
            let kv = rng.next_u32() | 1;
            present.insert(kv);
            p.put_bin(&k4(kv), &k4(i));
        }
        let mut before: Option<String> = None;
        let mut tries = 0;
        for _ in 0..500 {
            let kv = rng.next_u32() & !1u32;
            if present.contains(&kv) {
                continue;
            }
            assert_eq!(p.del_bin(&k4(kv), 0), 0, "missing-key delete must yield temp 0");
            // a failed delete must leave everything (bar `temp`, which it zeroes)
            // untouched, in both implementations
            let now = p.snap(p.ct);
            match &before {
                None => before = Some(now),
                Some(b) => assert_same("state across failed deletes", b, &now),
            }
            tries += 1;
        }
        assert!(tries > 100);
        p.check("hmdel missing key");
        p.free();
    }
}

// ==========================================================================
// rows 15, 19, 20 : asserts that must never fire
// ==========================================================================

#[test]
fn err_15_hmdel_slot_assert_never_fires() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 15);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "err15");
        for i in 0..4000u32 {
            let kv = (rng.next_u32() % 128) | 1;
            if i % 2 == 0 {
                p.put_bin(&k4(kv), &k4(i));
            } else {
                p.del_bin(&k4(kv), 0);
            }
        }
        p.check("heavy put/del workload");
        p.free();
    }
}

#[test]
fn err_19_hmput_cap_assert_never_fires() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [8usize, 16, 40] {
            seed_both(&c, &r, 0x31415926);
            let mut rng = Rng::new(SEED ^ 19 ^ elemsize as u64);
            let mut p = Pair::empty(&c, &r, elemsize, 4, 0, KeyKind::Inline, "err19");
            for _ in 0..3000u32 {
                let key = k4(rng.next_u32());
                let val = rng.bytes(elemsize - 4);
                p.put_bin(&key, &val);
            }
            p.check("3000 inserts");
            p.free();
        }
    }
}

#[test]
fn err_20_shrink_floor_no_abort() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 20);
        let mut p = Pair::empty(&c, &r, 8, 4, 0, KeyKind::Inline, "err20");
        let mut keys = Vec::new();
        for i in 0..400u32 {
            let kv = rng.next_u32() | 1;
            if keys.contains(&kv) {
                continue;
            }
            keys.push(kv);
            p.put_bin(&k4(kv), &k4(i));
        }
        // delete everything: drives make_hash_index(slot_count>>1) down to the
        // 8-slot floor, where the used/tombstone threshold assert would fire if
        // the floor guard were wrong.
        for kv in &keys {
            p.del_bin(&k4(*kv), 0);
        }
        p.check("fully drained map");
        // and refill / drain again
        for (i, kv) in keys.iter().enumerate() {
            p.put_bin(&k4(*kv), &k4(i as u32));
        }
        for kv in &keys {
            p.del_bin(&k4(*kv), 0);
        }
        p.check("second drain");
        p.free();
    }
}

// ==========================================================================
// rows 16/17 : corrupted-key delete aborts identically
// ==========================================================================

#[test]
fn err_16_hmdel_corrupt_key_abort() {
    assert_same_failure("corrupt_key");
}

// ==========================================================================
// row 18 : mode == 2 must not free a strdup'd key
// ==========================================================================

#[test]
fn err_18_hmdel_mode_two_no_free() {
    let (c, r) = libs();
    unsafe {
        // Deleting the *last* element takes the `old_index == final_index`
        // branch, so no key re-lookup happens and the C does not abort.  With
        // mode == 2 the `mode == STBDS_HM_STRING` guard is false, so the
        // strdup'd key is deliberately leaked rather than freed.
        for sh_mode in [1, 2, 3] {
            seed_both(&c, &r, 0x31415926);
            let key = CString::new("only-key").unwrap();
            let mut ct = (c.shmode_func)(16, sh_mode);
            let mut rt = (r.shmode_func)(16, sh_mode);
            ct = (c.hmput_key)(ct, 16, key.as_ptr() as *mut c_void, 8, 2);
            rt = (r.hmput_key)(rt, 16, key.as_ptr() as *mut c_void, 8, 2);
            // initialise the value half (the C macro's `t[temp].value = v`);
            // otherwise we would be comparing uninitialised malloc bytes
            for t in [ct, rt] {
                let e = (t as *mut u8).offset(map_header(t, 16).temp * 16);
                std::ptr::copy_nonoverlapping(42u64.to_le_bytes().as_ptr(), e.add(8), 8);
            }
            assert_same(
                &format!("mode2 put sh={sh_mode}"),
                &snapshot_map(ct, 16, 8, KeyKind::Pointer),
                &snapshot_map(rt, 16, 8, KeyKind::Pointer),
            );
            ct = (c.hmdel_key)(ct, 16, key.as_ptr() as *mut c_void, 8, 0, 2);
            rt = (r.hmdel_key)(rt, 16, key.as_ptr() as *mut c_void, 8, 0, 2);
            assert_eq!(map_header(ct, 16).temp, 1, "C: delete must report success");
            assert_eq!(map_header(rt, 16).temp, 1, "Rust: delete must report success");
            assert_same(
                &format!("mode2 del sh={sh_mode}"),
                &snapshot_map(ct, 16, 8, KeyKind::Inline),
                &snapshot_map(rt, 16, 8, KeyKind::Inline),
            );
            (c.hmfree_func)((ct as *mut u8).sub(16) as *mut c_void, 16);
            (r.hmfree_func)((rt as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

// ==========================================================================
// rows 22-26 : string arena boundaries
// ==========================================================================

#[test]
fn err_22_stralloc_oversize_first() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 22);
    unsafe {
        for len in [512usize, 513, 1024, 1 << 20, (1 << 20) + 1] {
            let s = rng.ascii(len);
            let mut ca = new_arena();
            let mut ra = new_arena();
            let pc = (c.stralloc)(&mut ca as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            let pr = (r.stralloc)(&mut ra as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            assert_eq!(cstr(pc), cstr(pr), "oversize-first content len={len}");
            assert_same(
                &format!("oversize-first arena len={len}"),
                &snapshot_arena(&ca),
                &snapshot_arena(&ra),
            );
            if len >= 512 {
                // len > blocksize(512) => dedicated block, remaining forced to 0
                assert_eq!(ca.remaining, 0, "C remaining");
                assert_eq!(ra.remaining, 0, "Rust remaining");
            }
            (c.strreset)(&mut ca as *mut _ as *mut c_void);
            (r.strreset)(&mut ra as *mut _ as *mut c_void);
        }
    }
}

#[test]
fn err_23_stralloc_oversize_splice() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 23);
    unsafe {
        let mut ca = new_arena();
        let mut ra = new_arena();
        // first a small string so `storage != NULL` and `remaining > 0`
        let small = rng.ascii(10);
        (c.stralloc)(&mut ca as *mut _ as *mut c_void, small.as_ptr() as *mut i8);
        (r.stralloc)(&mut ra as *mut _ as *mut c_void, small.as_ptr() as *mut i8);
        let rem_before_c = ca.remaining;
        let rem_before_r = ra.remaining;
        assert_eq!(rem_before_c, rem_before_r);
        // now an oversize string: spliced in as storage->next, `remaining` kept
        let big = rng.ascii(5000);
        let pc = (c.stralloc)(&mut ca as *mut _ as *mut c_void, big.as_ptr() as *mut i8);
        let pr = (r.stralloc)(&mut ra as *mut _ as *mut c_void, big.as_ptr() as *mut i8);
        assert_eq!(cstr(pc), cstr(pr));
        assert_eq!(ca.remaining, rem_before_c, "C: remaining must be untouched");
        assert_eq!(ra.remaining, rem_before_r, "Rust: remaining must be untouched");
        assert_same("splice arena", &snapshot_arena(&ca), &snapshot_arena(&ra));
        // the spliced chain must still be walkable by strreset
        (c.strreset)(&mut ca as *mut _ as *mut c_void);
        (r.strreset)(&mut ra as *mut _ as *mut c_void);
        assert_same("splice reset", &snapshot_arena(&ca), &snapshot_arena(&ra));
    }
}

#[test]
fn err_24_stralloc_block_ceiling() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 24);
    unsafe {
        // Drive `block` from 0 well past the value where 512<<(block>>1) hits
        // BLOCKSIZE_MAX, and also start from hand-set extreme `block` values.
        // `blocksize = 512 << (block>>1)` (with the shift count masked to 6 bits
        // on x86-64, in both implementations).  Values that ask for a multi-
        // gigabyte block, or that make `realloc` fail outright, are covered by
        // the `block_huge` fatal-signal differential instead.
        for start_block in [0u8, 1, 20, 21, 22, 110, 111, 120, 126, 127, 128, 129] {
            let mut ca = new_arena();
            let mut ra = new_arena();
            ca.block = start_block;
            ra.block = start_block;
            let s = rng.ascii(8);
            let pc = (c.stralloc)(&mut ca as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            let pr = (r.stralloc)(&mut ra as *mut _ as *mut c_void, s.as_ptr() as *mut i8);
            assert_eq!(cstr(pc), cstr(pr), "block={start_block} content");
            assert_same(
                &format!("block={start_block}"),
                &snapshot_arena(&ca),
                &snapshot_arena(&ra),
            );
            (c.strreset)(&mut ca as *mut _ as *mut c_void);
            (r.strreset)(&mut ra as *mut _ as *mut c_void);
        }
    }
}

#[test]
fn err_25_stralloc_empty_string() {
    let (c, r) = libs();
    unsafe {
        let e = CString::new("").unwrap();
        let mut ca = new_arena();
        let mut ra = new_arena();
        for i in 0..600 {
            let pc = (c.stralloc)(&mut ca as *mut _ as *mut c_void, e.as_ptr() as *mut i8);
            let pr = (r.stralloc)(&mut ra as *mut _ as *mut c_void, e.as_ptr() as *mut i8);
            assert_eq!(cstr(pc), cstr(pr), "empty-string alloc {i}");
            assert_same(
                &format!("empty-string arena {i}"),
                &snapshot_arena(&ca),
                &snapshot_arena(&ra),
            );
        }
        (c.strreset)(&mut ca as *mut _ as *mut c_void);
        (r.strreset)(&mut ra as *mut _ as *mut c_void);
    }
}

#[test]
fn err_26_strreset_empty_idempotent() {
    let (c, r) = libs();
    unsafe {
        let mut ca = new_arena();
        let mut ra = new_arena();
        for _ in 0..5 {
            (c.strreset)(&mut ca as *mut _ as *mut c_void);
            (r.strreset)(&mut ra as *mut _ as *mut c_void);
            assert_same("strreset idempotent", &snapshot_arena(&ca), &snapshot_arena(&ra));
            assert!(ca.storage.is_null() && ra.storage.is_null());
            assert_eq!(ca.remaining, 0);
            assert_eq!(ra.remaining, 0);
            assert_eq!(ca.block, 0);
            assert_eq!(ra.block, 0);
            assert_eq!(ca.mode, 0);
            assert_eq!(ra.mode, 0);
        }
        // a non-zero mode is also cleared (memset over the whole struct)
        ca.mode = 3;
        ra.mode = 3;
        ca.block = 9;
        ra.block = 9;
        (c.strreset)(&mut ca as *mut _ as *mut c_void);
        (r.strreset)(&mut ra as *mut _ as *mut c_void);
        assert_same("strreset zeroes mode", &snapshot_arena(&ca), &snapshot_arena(&ra));
    }
}

// ==========================================================================
// rows 27, 29, 30 : hashing boundaries
// ==========================================================================

#[test]
fn err_27_hash_bytes_zero_len() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 27);
    unsafe {
        let mut buf = vec![0u8; 64];
        let mut seeds: Vec<usize> = vec![0, 1, 0x31415926, usize::MAX];
        for _ in 0..16 {
            seeds.push(rng.next_u64() as usize);
        }
        for &s in &seeds {
            let hc = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, s);
            let hr = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, s);
            assert_eq!(hc, hr, "hash_bytes len=0 seed={s:#x}");
        }
        // also a genuinely zero-sized allocation
        let empty: Vec<u8> = Vec::new();
        let hc = (c.hash_bytes)(empty.as_ptr() as *mut c_void, 0, 7);
        let hr = (r.hash_bytes)(empty.as_ptr() as *mut c_void, 0, 7);
        assert_eq!(hc, hr);
    }
}

#[test]
fn err_29_hash_string_empty() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 29);
    unsafe {
        let e = CString::new("").unwrap();
        let mut seeds: Vec<usize> = vec![0, 1, usize::MAX, 0x31415926];
        for _ in 0..16 {
            seeds.push(rng.next_u64() as usize);
        }
        for &s in &seeds {
            assert_eq!(
                (c.hash_string)(e.as_ptr() as *mut i8, s),
                (r.hash_string)(e.as_ptr() as *mut i8, s),
                "hash_string(\"\") seed={s:#x}"
            );
        }
    }
}

#[test]
fn err_30_hash_below_two_fixup() {
    let (c, r) = libs();
    unsafe {
        // hash_string("", seed) == F(0) + seed, so a seed of -F(0) / 1-F(0)
        // produces a raw hash of exactly 0 / 1 and forces the `hash += 2`
        // fix-up in both hm_find_slot and hmput_key.
        let e = CString::new("").unwrap();
        let f0 = (c.hash_string)(e.as_ptr() as *mut i8, 0);
        assert_eq!(f0, (r.hash_string)(e.as_ptr() as *mut i8, 0));
        for target in [0usize, 1] {
            let seed = target.wrapping_sub(f0);
            assert_eq!(
                (c.hash_string)(e.as_ptr() as *mut i8, seed),
                target,
                "seed derivation failed"
            );
            seed_both(&c, &r, seed);
            let mut p = Pair::shmode(&c, &r, 16, 8, 1, 2, KeyKind::Pointer, "err30");
            p.put_str(&e, &7u64.to_le_bytes());
            p.check(&format!("hash=={target} insert"));
            assert!(p.get_str(&e) >= 0, "the hash<2 key must be findable");
            p.check(&format!("hash=={target} lookup"));
            // add more keys so the fixed-up hash coexists with normal ones
            let mut rng = Rng::new(SEED ^ target as u64);
            for i in 0..40u64 {
                let k = rng.ascii_len(1, 12);
                p.put_str(&k, &i.to_le_bytes());
            }
            assert!(p.get_str(&e) >= 0);
            p.check(&format!("hash=={target} coexistence"));
            assert_eq!(p.del_str(&e), 1, "the hash<2 key must be deletable");
            p.check(&format!("hash=={target} delete"));
            p.free();
        }
    }
}

// ==========================================================================
// rows 31-33 : out-of-range enum values across the FFI boundary
// ==========================================================================

#[test]
fn err_31_shmode_out_of_range() {
    let (c, r) = libs();
    unsafe {
        for sh_mode in [-1i32, 4, 5, 255, 256, 257, i32::MIN, i32::MAX] {
            seed_both(&c, &r, 0x31415926);
            let ct = (c.shmode_func)(16, sh_mode);
            let rt = (r.shmode_func)(16, sh_mode);
            let cm = (*(map_header(ct, 16).hash_table as *mut HashIndex)).string.mode;
            let rm = (*(map_header(rt, 16).hash_table as *mut HashIndex)).string.mode;
            assert_eq!(cm, rm, "string.mode for sh_mode={sh_mode}");
            assert_eq!(cm, (sh_mode as u32 & 0xff) as u8, "must be (unsigned char) mode");
            assert_same(
                &format!("shmode_func(16,{sh_mode})"),
                &snapshot_map(ct, 16, 8, KeyKind::Inline),
                &snapshot_map(rt, 16, 8, KeyKind::Inline),
            );
            // string.mode values outside {1,2,3} hit the `default:` memcpy arm
            if !(1..=3).contains(&cm) {
                let mut rng = Rng::new(SEED ^ 31 ^ sh_mode as u64 as u64);
                let mut p = Pair {
                    c: &c,
                    r: &r,
                    ct,
                    rt,
                    elemsize: 16,
                    keysize: 8,
                    mode: 0,
                    kind: KeyKind::Inline,
                    label: format!("err31 sh={sh_mode}"),
                };
                for i in 0..60u64 {
                    let key = rng.next_u64().to_le_bytes().to_vec();
                    p.put_bin(&key, &i.to_le_bytes());
                }
                p.check("default: memcpy arm");
                p.free();
            } else {
                (c.hmfree_func)((ct as *mut u8).sub(16) as *mut c_void, 16);
                (r.hmfree_func)((rt as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
}

#[test]
fn err_32_mode_out_of_range() {
    let (c, r) = libs();
    unsafe {
        // negative modes take the *binary* path (mode >= STBDS_HM_STRING fails)
        for mode in [-1i32, -2, i32::MIN] {
            seed_both(&c, &r, 0x31415926);
            let mut rng = Rng::new(SEED ^ 32 ^ mode as u32 as u64);
            let mut p = Pair::empty(&c, &r, 8, 4, mode, KeyKind::Inline, &format!("err32 mode={mode}"));
            let mut keys = Vec::new();
            for i in 0..120u32 {
                let kv = rng.next_u32() | 1;
                keys.push(kv);
                p.put_bin(&k4(kv), &k4(i));
            }
            for kv in &keys {
                assert!(p.get_bin(&k4(*kv)) >= 0);
            }
            for kv in &keys {
                p.del_bin(&k4(*kv), 0);
            }
            p.check("negative mode == binary");
            p.free();
        }
        // modes >= 1 take the *string* path
        for mode in [2i32, 3, 1000, i32::MAX] {
            seed_both(&c, &r, 0x31415926);
            let mut rng = Rng::new(SEED ^ 132 ^ mode as u32 as u64);
            let mut p = Pair::empty(&c, &r, 16, 8, mode, KeyKind::Pointer, &format!("err32 mode={mode}"));
            let keys: Vec<CString> = (0..50).map(|_| rng.ascii_len(1, 20)).collect();
            for (i, k) in keys.iter().enumerate() {
                p.put_str(k, &(i as u64).to_le_bytes());
            }
            for k in &keys {
                assert!(p.get_str(k) >= 0, "mode={mode}: string key must be found");
            }
            p.check(&format!("mode={mode} string path"));
            // deleting the most recently added key hits old_index==final_index
            let last = keys.last().unwrap();
            assert_eq!(p.del_str(last), 1);
            p.check(&format!("mode={mode} tail delete"));
            p.free();
        }
    }
}

#[test]
fn err_33_put_string_mode_default() {
    let (c, r) = libs();
    unsafe {
        for (mode, expect) in [(-1i32, 0u8), (0, 0), (1, 1), (2, 1), (i32::MAX, 1), (i32::MIN, 0)] {
            seed_both(&c, &r, 0x31415926);
            let key = CString::new("abcdefg").unwrap();
            let ct = (c.hmput_key)(std::ptr::null_mut(), 16, key.as_ptr() as *mut c_void, 8, mode);
            let rt = (r.hmput_key)(std::ptr::null_mut(), 16, key.as_ptr() as *mut c_void, 8, mode);
            let cm = (*(map_header(ct, 16).hash_table as *mut HashIndex)).string.mode;
            let rm = (*(map_header(rt, 16).hash_table as *mut HashIndex)).string.mode;
            assert_eq!(cm, rm, "string.mode for mode={mode}");
            assert_eq!(cm, expect, "mode={mode} must yield string.mode {expect}");
            // initialise the value half (uninitialised malloc bytes otherwise)
            for t in [ct, rt] {
                let e = (t as *mut u8).offset(map_header(t, 16).temp * 16);
                std::ptr::copy_nonoverlapping(7u64.to_le_bytes().as_ptr(), e.add(8), 8);
            }
            let kind = if expect == 1 { KeyKind::Pointer } else { KeyKind::Inline };
            assert_same(
                &format!("hmput_key first insert mode={mode}"),
                &snapshot_map(ct, 16, 8, kind),
                &snapshot_map(rt, 16, 8, kind),
            );
            (c.hmfree_func)((ct as *mut u8).sub(16) as *mut c_void, 16);
            (r.hmfree_func)((rt as *mut u8).sub(16) as *mut c_void, 16);
        }
    }
}

// ==========================================================================
// rows 34-36 : hmput_default and degenerate key sizes
// ==========================================================================

#[test]
fn err_34_hmput_default_twice() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [1usize, 8, 16, 40] {
            seed_both(&c, &r, 0x31415926);
            let ct1 = (c.hmput_default)(std::ptr::null_mut(), elemsize);
            let rt1 = (r.hmput_default)(std::ptr::null_mut(), elemsize);
            assert_same(
                &format!("hmput_default(NULL,{elemsize})"),
                &snapshot_map(ct1, elemsize, 1, KeyKind::Inline),
                &snapshot_map(rt1, elemsize, 1, KeyKind::Inline),
            );
            let ct2 = (c.hmput_default)(ct1, elemsize);
            let rt2 = (r.hmput_default)(rt1, elemsize);
            assert_eq!(ct2, ct1, "C: second call must be a no-op");
            assert_eq!(rt2, rt1, "Rust: second call must be a no-op");
            assert_same(
                "hmput_default twice",
                &snapshot_map(ct2, elemsize, 1, KeyKind::Inline),
                &snapshot_map(rt2, elemsize, 1, KeyKind::Inline),
            );
            (c.hmfree_func)((ct2 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            (r.hmfree_func)((rt2 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn err_35_keysize_zero() {
    let (c, r) = libs();
    unsafe {
        seed_both(&c, &r, 0x31415926);
        let mut rng = Rng::new(SEED ^ 35);
        let mut p = Pair::empty(&c, &r, 8, 0, 0, KeyKind::Inline, "err35 keysize=0");
        for i in 0..50u32 {
            let val = rng.bytes(8);
            p.put_bin(&[], &val);
            let _ = i;
        }
        // every zero-length key compares equal, so the map holds exactly 1 entry
        let h = map_header(p.ct, 8);
        assert_eq!(h.length, 2, "1 default slot + 1 entry");
        p.check("keysize=0");
        assert_eq!(p.get_bin(&[]), 0);
        assert_eq!(p.del_bin(&[], 0), 1);
        p.check("keysize=0 after delete");
        p.free();
    }
}

#[test]
fn err_36_keysize_equals_elemsize() {
    let (c, r) = libs();
    unsafe {
        for elemsize in [1usize, 4, 8, 16] {
            seed_both(&c, &r, 0x31415926);
            let mut rng = Rng::new(SEED ^ 36 ^ elemsize as u64);
            let mut p = Pair::empty(
                &c,
                &r,
                elemsize,
                elemsize,
                0,
                KeyKind::Inline,
                &format!("err36 elemsize=keysize={elemsize}"),
            );
            let mut keys = Vec::new();
            for _ in 0..80 {
                let key = rng.bytes(elemsize);
                keys.push(key.clone());
                p.put_bin(&key, &[]);
            }
            p.check("keysize == elemsize");
            for k in &keys {
                assert!(p.get_bin(k) >= 0);
            }
            for k in &keys {
                p.del_bin(k, 0);
            }
            p.check("keysize == elemsize drained");
            p.free();
        }
    }
}

// ==========================================================================
// rows 37, 40 : strkey / intput valid extremes
// ==========================================================================

#[test]
fn err_37_strkey_extremes() {
    let (c, r) = libs();
    unsafe {
        for n in [0i32, 1, -1, 9, 11, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            assert_eq!(cstr(pc), cstr(pr), "strkey({n})");
            let bc: Vec<u8> = (0..64).map(|i| *(pc as *const u8).add(i)).collect();
            let br: Vec<u8> = (0..64).map(|i| *(pr as *const u8).add(i)).collect();
            let lc = bc.iter().position(|&b| b == 0).unwrap();
            assert_eq!(&bc[..=lc], &br[..=lc], "strkey({n}) bytes");
        }
    }
}

#[test]
fn err_40_intput_ok_values() {
    let (c, r) = libs();
    unsafe {
        for n in [i32::MIN, -1, 0, 1, 8, 10, 12, i32::MAX, 3, 7] {
            seed_both(&c, &r, 0x31415926);
            (c.intput)(n);
            (r.intput)(n);
        }
    }
}

// ==========================================================================
// rows 38, 39, 41, 42, 44 : fatal-signal differentials
// ==========================================================================

#[test]
fn err_38_intput_9_aborts() {
    assert_same_failure("intput9");
}

#[test]
fn err_39_intput_11_aborts() {
    assert_same_failure("intput11");
}

#[test]
fn err_41_arrfreef_null_both_abort() {
    assert_same_failure("arrfree_null");
}

#[test]
fn err_42_hmget_ts_null_temp_both_crash() {
    assert_same_failure("get_ts_null_temp");
}

#[test]
fn err_44_hmdel_mode_two_abort() {
    assert_same_failure("del_mode2");
}

#[test]
fn err_24b_stralloc_block_alloc_failure() {
    // block == 200 => blocksize == 512 << 36, whose realloc fails; the C then
    // dereferences the NULL block.  Both implementations must die the same way.
    assert_same_failure("block_huge");
}
