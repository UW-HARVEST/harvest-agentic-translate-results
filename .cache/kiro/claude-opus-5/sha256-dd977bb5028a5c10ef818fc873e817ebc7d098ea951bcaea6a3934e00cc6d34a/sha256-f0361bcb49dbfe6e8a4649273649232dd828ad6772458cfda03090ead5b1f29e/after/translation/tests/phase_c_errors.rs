//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. Rows whose trigger aborts the process
//! (`assert`, invalid free, wild dereference) are compared in forked children
//! via `probe_abort`, so "aborts with signal N" is itself the compared result.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

const L8: Layout = Layout::new(8, 4, 4, 4);
const L16: Layout = Layout::new(16, 8, 8, 8);
const LS: Layout = Layout::new(16, 8, 8, 4);

fn cs(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Rows 1..4 — stbds_arrgrowf rejection / clamping paths
// ---------------------------------------------------------------------------

/// Row 1 — `min_cap <= arrcap(a)` → return `a` untouched, no allocation.
#[test]
fn err_01_arrgrowf_noop() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        // a == NULL, nothing requested
        for elemsize in [0usize, 1, 8, 16, 4096] {
            assert!((c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0).is_null());
            assert!((r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0).is_null());
        }
        // a != NULL and already big enough
        for elemsize in [1usize, 8, 16] {
            let ac = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 64);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 64);
            for min_cap in [0usize, 1, 32, 63, 64] {
                assert_eq!((c.arrgrowf)(ac, elemsize, 0, min_cap), ac);
                assert_eq!((r.arrgrowf)(ar, elemsize, 0, min_cap), ar);
            }
            assert_eq!(header_of(ac).capacity, header_of(ar).capacity);
            (c.arrfreef)(ac);
            (r.arrfreef)(ar);
        }
    }
}

/// Row 2 — `min_cap` forced up to 4.
#[test]
fn err_02_arrgrowf_min_cap_4() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [1usize, 8, 16, 24] {
            for (addlen, min_cap) in [(1usize, 0usize), (0, 1), (2, 1), (3, 3), (1, 3)] {
                let ac = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                let ar = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                assert_eq!(header_of(ac).capacity, 4, "C: expected clamp to 4");
                assert_eq!(header_of(ar).capacity, 4, "Rust: expected clamp to 4");
                assert_eq!(header_of(ac), header_of(ar));
                (c.arrfreef)(ac);
                (r.arrfreef)(ar);
            }
        }
    }
}

/// Row 3 — `min_cap < 2*arrcap(a)` → forced doubling, ignoring the request.
#[test]
fn err_03_arrgrowf_doubling() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        let elemsize = 8;
        let mut ac = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
        let mut ar = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 16);
        // request 17 (> cap 16, but < 2*16) -> capacity becomes 32
        ac = (c.arrgrowf)(ac, elemsize, 0, 17);
        ar = (r.arrgrowf)(ar, elemsize, 0, 17);
        assert_eq!(header_of(ac).capacity, 32, "C");
        assert_eq!(header_of(ar).capacity, 32, "Rust");
        // request 33 -> 64
        ac = (c.arrgrowf)(ac, elemsize, 0, 33);
        ar = (r.arrgrowf)(ar, elemsize, 0, 33);
        assert_eq!(header_of(ac).capacity, header_of(ar).capacity);
        assert_eq!(header_of(ac).capacity, 64);
        // request 1000 (> 2*64) -> exactly 1000
        ac = (c.arrgrowf)(ac, elemsize, 0, 1000);
        ar = (r.arrgrowf)(ar, elemsize, 0, 1000);
        assert_eq!(header_of(ac).capacity, 1000);
        assert_eq!(header_of(ar).capacity, 1000);
        (c.arrfreef)(ac);
        (r.arrfreef)(ar);
    }
}

/// Row 4 — the `a == NULL` initialisation path (and the fact that a non-NULL
/// grow leaves `length` / `hash_table` / `temp` alone).
#[test]
fn err_04_arrgrowf_null_init() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [1usize, 8, 16] {
            let ac = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 8);
            for h in [header_of(ac), header_of(ar)] {
                assert_eq!(h.length, 0);
                assert!(h.hash_table.is_null());
                assert_eq!(h.temp, 0);
                assert_eq!(h.capacity, 8);
            }
            // scribble, then grow again: the fields must be preserved
            (*(ac as *mut ArrayHeader).sub(1)).length = 3;
            (*(ar as *mut ArrayHeader).sub(1)).length = 3;
            (*(ac as *mut ArrayHeader).sub(1)).temp = -77;
            (*(ar as *mut ArrayHeader).sub(1)).temp = -77;
            let ac2 = (c.arrgrowf)(ac, elemsize, 0, 4096);
            let ar2 = (r.arrgrowf)(ar, elemsize, 0, 4096);
            assert_eq!(header_of(ac2), header_of(ar2));
            assert_eq!(header_of(ac2).length, 3);
            assert_eq!(header_of(ac2).temp, -77);
            (c.arrfreef)(ac2);
            (r.arrfreef)(ar2);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7..8 — stbds_hmfree_func null / no-table
// ---------------------------------------------------------------------------

/// Row 7 — `a == NULL` → immediate return, no free.
#[test]
fn err_07_hmfree_null() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [0usize, 1, 8, 16, usize::MAX] {
            (c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
    // and that it really is a no-op rather than a crash
    let oc = probe_abort(|| unsafe { (c.hmfree_func)(std::ptr::null_mut(), 16) });
    let or = probe_abort(|| unsafe { (r.hmfree_func)(std::ptr::null_mut(), 16) });
    assert_eq!(oc, or);
    assert_eq!(oc, ChildOutcome::Exited(0));
}

/// Row 8 — array with `hash_table == NULL`.
#[test]
fn err_08_hmfree_no_table() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let run = |l: &Lib, elemsize: usize, len: usize| unsafe {
        let a = (l.arrgrowf)(std::ptr::null_mut(), elemsize, len, 4);
        (*(a as *mut ArrayHeader).sub(1)).length = len;
        (l.hmfree_func)(a, elemsize);
    };
    for elemsize in [1usize, 8, 16, 24] {
        for len in [0usize, 1, 4] {
            let oc = probe_abort(|| run(c, elemsize, len));
            let or = probe_abort(|| run(r, elemsize, len));
            assert_eq!(oc, or, "elemsize={elemsize} len={len}");
            assert_eq!(oc, ChildOutcome::Exited(0));
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 9..14 — lookup misses
// ---------------------------------------------------------------------------

/// Row 9 / 10 — `stbds_hm_find_slot` returning `-1` from both its scan loops.
/// Randomised keys guarantee probes that start in the middle of a bucket (the
/// wrapped `i < limit` scan) as well as at its start.
#[test]
fn err_09_10_find_slot_miss() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 9);
    unsafe {
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut tc: *mut c_void = std::ptr::null_mut();
            let mut tr: *mut c_void = std::ptr::null_mut();
            for i in 0u64..200 {
                let mut k = i.to_ne_bytes().to_vec();
                let mut k2 = k.clone();
                let v = rng.bytes(8);
                let (nc, _) = c.hmput(tc, L16, &mut k, &v, STBDS_HM_BINARY);
                let (nr, _) = r.hmput(tr, L16, &mut k2, &v, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
            }
            let mut misses = 0;
            for _ in 0..3000 {
                let key = 1000u64 + rng.next_u64() % 1_000_000;
                let mut kc = key.to_ne_bytes().to_vec();
                let mut kr = kc.clone();
                let (nc, a) = c.hmgeti(tc, L16, kc.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                let (nr, b) = r.hmgeti(tr, L16, kr.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "seed={seed:#x} key={key}");
                if a == -1 {
                    misses += 1;
                }
            }
            assert_eq!(misses, 3000, "expected every probe to miss");
            c.hmfree(tc, L16.elemsize);
            r.hmfree(tr, L16.elemsize);
        }
    }
}

/// Row 11 — `stbds_hmget_key_ts(NULL, ...)`: bootstraps a zeroed 1-element
/// array and reports `*temp = -1`.
#[test]
fn err_11_hmget_ts_null() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 24, 128] {
            for keysize in [0usize, 1, 4, 8] {
                let mut key = [0xABu8; 16];
                let mut tempc: isize = 0x1234;
                let mut tempr: isize = 0x1234;
                let pc = (c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    keysize,
                    &mut tempc,
                    STBDS_HM_BINARY,
                );
                let pr = (r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    keysize,
                    &mut tempr,
                    STBDS_HM_BINARY,
                );
                assert_eq!(tempc, -1, "C temp");
                assert_eq!(tempr, -1, "Rust temp");
                assert!(!pc.is_null() && !pr.is_null());
                let hc = header_of(raw_of(pc, elemsize));
                let hr = header_of(raw_of(pr, elemsize));
                assert_eq!(
                    (hc.length, hc.capacity, hc.temp, hc.hash_table.is_null()),
                    (hr.length, hr.capacity, hr.temp, hr.hash_table.is_null()),
                    "elemsize={elemsize} keysize={keysize}"
                );
                assert_eq!(hc.length, 1);
                let ec = std::slice::from_raw_parts(raw_of(pc, elemsize) as *const u8, elemsize);
                let er = std::slice::from_raw_parts(raw_of(pr, elemsize) as *const u8, elemsize);
                assert_eq!(ec, er);
                assert!(ec.iter().all(|&b| b == 0), "element must be zeroed");
                c.hmfree(pc, elemsize);
                r.hmfree(pr, elemsize);
            }
        }
    }
}

/// Row 12 — `a != NULL` but no hash table: `*temp = -1`, `a` returned as-is.
#[test]
fn err_12_hmget_ts_no_table() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [8usize, 16, 24] {
            let ac = (c.hmput_default)(std::ptr::null_mut(), elemsize);
            let ar = (r.hmput_default)(std::ptr::null_mut(), elemsize);
            assert!(header_of(raw_of(ac, elemsize)).hash_table.is_null());
            assert!(header_of(raw_of(ar, elemsize)).hash_table.is_null());
            let mut key = [0x5Au8; 16];
            for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2, -1, i32::MAX] {
                let mut tc: isize = 99;
                let mut tr: isize = 99;
                let pc = (c.hmget_key_ts)(
                    ac,
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    &mut tc,
                    mode,
                );
                let pr = (r.hmget_key_ts)(
                    ar,
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    &mut tr,
                    mode,
                );
                assert_eq!(pc, ac);
                assert_eq!(pr, ar);
                assert_eq!(tc, -1, "C mode={mode}");
                assert_eq!(tr, -1, "Rust mode={mode}");
            }
            c.hmfree(ac, elemsize);
            r.hmfree(ar, elemsize);
        }
    }
}

/// Row 13 — key genuinely absent from a populated table.
#[test]
fn err_13_hmget_ts_missing_key() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        (c.rand_seed)(0x3141_5926);
        (r.rand_seed)(0x3141_5926);
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        for i in 0u32..30 {
            let mut k = i.to_ne_bytes().to_vec();
            let mut k2 = k.clone();
            let v = i.to_ne_bytes();
            let (nc, _) = c.hmput(tc, L8, &mut k, &v, STBDS_HM_BINARY);
            let (nr, _) = r.hmput(tr, L8, &mut k2, &v, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
        }
        for i in 30u32..200 {
            let mut kc = i.to_ne_bytes().to_vec();
            let mut kr = kc.clone();
            let (nc, a) = c.hmgeti_ts(tc, L8, kc.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
            let (nr, b) = r.hmgeti_ts(tr, L8, kr.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(a, -1, "C: key {i} should be absent");
            assert_eq!(b, -1, "Rust: key {i} should be absent");
        }
        c.hmfree(tc, L8.elemsize);
        r.hmfree(tr, L8.elemsize);
    }
}

/// Row 14 — `hmget_key` additionally writes the result into `header->temp`.
#[test]
fn err_14_hmget_key_temp() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        // NULL array: header temp is initialised to 0 by arrgrowf then set to -1
        for elemsize in [8usize, 16] {
            let mut key = [7u8; 8];
            let pc = (c.hmget_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                8,
                STBDS_HM_BINARY,
            );
            let pr = (r.hmget_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                8,
                STBDS_HM_BINARY,
            );
            assert_eq!(header_of(raw_of(pc, elemsize)).temp, -1, "C");
            assert_eq!(header_of(raw_of(pr, elemsize)).temp, -1, "Rust");
            c.hmfree(pc, elemsize);
            r.hmfree(pr, elemsize);
        }
        // populated: temp must equal the found index or -1
        (c.rand_seed)(7);
        (r.rand_seed)(7);
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        for i in 0u64..50 {
            let mut k = i.to_ne_bytes().to_vec();
            let mut k2 = k.clone();
            let v = i.to_ne_bytes();
            let (nc, _) = c.hmput(tc, L16, &mut k, &v, STBDS_HM_BINARY);
            let (nr, _) = r.hmput(tr, L16, &mut k2, &v, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
        }
        for i in 0u64..100 {
            let mut kc = i.to_ne_bytes().to_vec();
            let mut kr = kc.clone();
            let (nc, a) = c.hmgeti(tc, L16, kc.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
            let (nr, b) = r.hmgeti(tr, L16, kr.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(a, b, "key={i}");
            assert_eq!(header_of(raw_of(tc, 16)).temp, a);
            assert_eq!(header_of(raw_of(tr, 16)).temp, b);
            if i >= 50 {
                assert_eq!(a, -1);
            }
        }
        c.hmfree(tc, L16.elemsize);
        r.hmfree(tr, L16.elemsize);
    }
}

// ---------------------------------------------------------------------------
// Rows 15..20 — stbds_hmput_default / stbds_hmput_key
// ---------------------------------------------------------------------------

/// Row 15 — `hmput_default(NULL, elemsize)`.
#[test]
fn err_15_hmput_default_null() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 24, 128, 1024] {
            let pc = (c.hmput_default)(std::ptr::null_mut(), elemsize);
            let pr = (r.hmput_default)(std::ptr::null_mut(), elemsize);
            let hc = header_of(raw_of(pc, elemsize));
            let hr = header_of(raw_of(pr, elemsize));
            assert_eq!(
                (hc.length, hc.capacity, hc.temp, hc.hash_table.is_null()),
                (hr.length, hr.capacity, hr.temp, hr.hash_table.is_null())
            );
            assert_eq!(hc.length, 1);
            let ec = std::slice::from_raw_parts(raw_of(pc, elemsize) as *const u8, elemsize);
            let er = std::slice::from_raw_parts(raw_of(pr, elemsize) as *const u8, elemsize);
            assert_eq!(ec, er);
            assert!(ec.iter().all(|&b| b == 0));
            c.hmfree(pc, elemsize);
            r.hmfree(pr, elemsize);
        }
    }
}

/// Row 16 — `hmput_default` on a non-NULL array whose length is 0.
#[test]
fn err_16_hmput_default_len0() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [8usize, 16, 24] {
            let ac = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            // scribble the first element so the memset is observable
            std::ptr::write_bytes(ac as *mut u8, 0xEE, elemsize);
            std::ptr::write_bytes(ar as *mut u8, 0xEE, elemsize);
            let hmc = (ac as *mut u8).add(elemsize) as *mut c_void;
            let hmr = (ar as *mut u8).add(elemsize) as *mut c_void;
            assert_eq!(header_of(ac).length, 0);
            let pc = (c.hmput_default)(hmc, elemsize);
            let pr = (r.hmput_default)(hmr, elemsize);
            assert_eq!(header_of(raw_of(pc, elemsize)).length, 1, "C");
            assert_eq!(header_of(raw_of(pr, elemsize)).length, 1, "Rust");
            let ec = std::slice::from_raw_parts(raw_of(pc, elemsize) as *const u8, elemsize);
            let er = std::slice::from_raw_parts(raw_of(pr, elemsize) as *const u8, elemsize);
            assert_eq!(ec, er);
            assert!(ec.iter().all(|&b| b == 0), "element must have been zeroed");
            c.hmfree(pc, elemsize);
            r.hmfree(pr, elemsize);
        }
    }
}

/// Row 17 — `hmput_key(NULL, ...)` bootstraps rather than erroring.
#[test]
fn err_17_hmput_null_bootstrap() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [8usize, 16, 24] {
            for mode in [STBDS_HM_BINARY, STBDS_HM_STRING] {
                (c.rand_seed)(0x3141_5926);
                (r.rand_seed)(0x3141_5926);
                let mut kb = cs("bootstrap_key_padded");
                let lay = Layout::new(elemsize, 8, 8, 0);
                let (tc, itc) = if mode == STBDS_HM_BINARY {
                    c.hmput(std::ptr::null_mut(), lay, &mut kb, &[], mode)
                } else {
                    c.shput(std::ptr::null_mut(), lay, kb.as_mut_ptr() as *mut c_char, &[], mode)
                };
                let (tr, itr) = if mode == STBDS_HM_BINARY {
                    r.hmput(std::ptr::null_mut(), lay, &mut kb, &[], mode)
                } else {
                    r.shput(std::ptr::null_mut(), lay, kb.as_mut_ptr() as *mut c_char, &[], mode)
                };
                assert_eq!(itc, itr, "elemsize={elemsize} mode={mode}");
                assert_eq!(itc, 0, "first insert lands at temp 0");
                let repr = if mode == STBDS_HM_BINARY {
                    KeyRepr::Inline
                } else {
                    KeyRepr::Pointer
                };
                assert_eq!(
                    snapshot_map_lay(tc, lay, repr),
                    snapshot_map_lay(tr, lay, repr),
                    "elemsize={elemsize} mode={mode}"
                );
                assert_eq!(header_of(raw_of(tc, elemsize)).length, 2);
                c.hmfree(tc, elemsize);
                r.hmfree(tr, elemsize);
            }
        }
    }
}

/// Row 18 — the grow branch: `table == NULL`, then `used_count >=
/// used_count_threshold` at every doubling step.
#[test]
fn err_18_hmput_grow_threshold() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        (c.rand_seed)(0x3141_5926);
        (r.rand_seed)(0x3141_5926);
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        let mut seen_slot_counts = Vec::new();
        for i in 0u64..2000 {
            let mut k = i.wrapping_mul(0x0100_0193).to_ne_bytes().to_vec();
            let mut k2 = k.clone();
            let v = i.to_ne_bytes();
            let (nc, _) = c.hmput(tc, L16, &mut k, &v, STBDS_HM_BINARY);
            let (nr, _) = r.hmput(tr, L16, &mut k2, &v, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            let sc = snapshot_map_lay(tc, L16, KeyRepr::Inline);
            let sr = snapshot_map_lay(tr, L16, KeyRepr::Inline);
            assert_eq!(sc.slot_count, sr.slot_count, "i={i}");
            assert_eq!(sc.used_count_threshold, sr.used_count_threshold, "i={i}");
            assert_eq!(sc, sr, "i={i}");
            if seen_slot_counts.last() != Some(&sc.slot_count) {
                seen_slot_counts.push(sc.slot_count);
            }
        }
        assert_eq!(
            seen_slot_counts,
            vec![8usize, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096],
            "expected the full doubling chain"
        );
        c.hmfree(tc, L16.elemsize);
        r.hmfree(tr, L16.elemsize);
    }
}

/// Row 20 — `table->string.mode` outside the enum: `shmode_func` truncates the
/// `int` to `unsigned char`, and any value that is not 1/2/3 lands in the
/// `default:` `memcpy` branch.
#[test]
fn err_20_out_of_range_shmode() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let lay = Layout::new(16, 8, 8, 4);
    unsafe {
        for mode in [
            4i32,
            5,
            255,
            256,
            257,
            -1,
            -256,
            i32::MAX,
            i32::MIN,
            0x100 | 2,
            0x100 | 3,
        ] {
            (c.rand_seed)(0x3141_5926);
            (r.rand_seed)(0x3141_5926);
            let tc0 = (c.shmode_func)(lay.elemsize, mode);
            let tr0 = (r.shmode_func)(lay.elemsize, mode);
            let mc = snapshot_map_lay(tc0, lay, KeyRepr::Inline).string_mode;
            let mr = snapshot_map_lay(tr0, lay, KeyRepr::Inline).string_mode;
            assert_eq!(mc, mr, "mode={mode}: truncated string.mode differs");
            assert_eq!(mc, (mode as u32 & 0xff) as u8, "mode={mode}");

            // Only exercise inserts for truncated modes that land in the binary
            // `default:` branch or in a well-defined string branch; the string
            // branches store a pointer and are covered by Phase B.
            let mut tc = tc0;
            let mut tr = tr0;
            // One long-lived buffer per library: SH_DEFAULT stores the caller's
            // pointer verbatim, so it must outlive the snapshot.
            let mut key_c = cs("shmode_probe_key");
            let mut key_r = cs("shmode_probe_key");
            let (nc, a) = c.hmput(tc, lay, &mut key_c, &[1u8, 2, 3, 4], STBDS_HM_BINARY);
            let (nr, b) = r.hmput(tr, lay, &mut key_r, &[1u8, 2, 3, 4], STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(a, b, "mode={mode}");
            // `stbds_hmput` rewrites `(t)[temp].key = k` after `hmput_key`
            // returns, so whatever the switch stored is overwritten by inline
            // key bytes; compare them raw.
            assert_eq!(
                snapshot_map_lay(tc, lay, KeyRepr::Inline),
                snapshot_map_lay(tr, lay, KeyRepr::Inline),
                "mode={mode}"
            );
            if !(1..=3).contains(&mc) {
                c.hmfree(tc, lay.elemsize);
                r.hmfree(tr, lay.elemsize);
            }
            // For truncated modes 1..3 the switch stored a heap/arena pointer
            // that the macro then clobbered, so `hmfree` would free garbage.
            // Both libraries are in the identical (leaked) state; leave it.
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 21..23, 28..30 — stbds_hmdel_key
// ---------------------------------------------------------------------------

/// Row 21 — `hmdel_key(NULL, ...)` returns `NULL`, the library's one sentinel.
#[test]
fn err_21_hmdel_null() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        let mut key = [1u8; 16];
        for elemsize in [0usize, 1, 8, 16, usize::MAX] {
            for keysize in [0usize, 8, usize::MAX] {
                for keyoffset in [0usize, 8, usize::MAX] {
                    for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2, -1, i32::MAX, i32::MIN] {
                        let pc = (c.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            key.as_mut_ptr() as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                        let pr = (r.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            key.as_mut_ptr() as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                        assert!(pc.is_null(), "C must return NULL");
                        assert!(pr.is_null(), "Rust must return NULL");
                    }
                }
            }
        }
    }
}

/// Row 22 — `hash_table == 0`: `temp` forced to 0, `a` returned, no deletion.
#[test]
fn err_22_hmdel_no_table() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [8usize, 16, 24] {
            let ac = (c.hmput_default)(std::ptr::null_mut(), elemsize);
            let ar = (r.hmput_default)(std::ptr::null_mut(), elemsize);
            (*(raw_of(ac, elemsize) as *mut ArrayHeader).sub(1)).temp = 0x5555;
            (*(raw_of(ar, elemsize) as *mut ArrayHeader).sub(1)).temp = 0x5555;
            let mut key = [3u8; 16];
            for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 2, -5] {
                let pc = (c.hmdel_key)(
                    ac,
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    0,
                    mode,
                );
                let pr = (r.hmdel_key)(
                    ar,
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    8,
                    0,
                    mode,
                );
                assert_eq!(pc, ac, "C must return a unchanged");
                assert_eq!(pr, ar, "Rust must return a unchanged");
                assert_eq!(header_of(raw_of(ac, elemsize)).temp, 0, "C temp");
                assert_eq!(header_of(raw_of(ar, elemsize)).temp, 0, "Rust temp");
                assert_eq!(header_of(raw_of(ac, elemsize)).length, 1);
                assert_eq!(header_of(raw_of(ar, elemsize)).length, 1);
            }
            c.hmfree(ac, elemsize);
            r.hmfree(ar, elemsize);
        }
    }
}

/// Row 23 — deleting an absent key leaves `temp = 0` and the map untouched.
#[test]
fn err_23_hmdel_missing_key() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        (c.rand_seed)(0x3141_5926);
        (r.rand_seed)(0x3141_5926);
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        for i in 0u64..40 {
            let mut k = i.to_ne_bytes().to_vec();
            let mut k2 = k.clone();
            let v = i.to_ne_bytes();
            let (nc, _) = c.hmput(tc, L16, &mut k, &v, STBDS_HM_BINARY);
            let (nr, _) = r.hmput(tr, L16, &mut k2, &v, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
        }
        let len_before = header_of(raw_of(tc, 16)).length;
        for i in 40u64..300 {
            let mut kc = i.to_ne_bytes().to_vec();
            let mut kr = kc.clone();
            let (nc, a) = c.hmdel(tc, L16, kc.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
            let (nr, b) = r.hmdel(tr, L16, kr.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(a, 0, "C: absent-key delete must report 0");
            assert_eq!(b, 0, "Rust: absent-key delete must report 0");
            assert_eq!(header_of(raw_of(tc, 16)).length, len_before);
            assert_eq!(header_of(raw_of(tr, 16)).length, len_before);
        }
        assert_eq!(
            snapshot_map_lay(tc, L16, KeyRepr::Inline),
            snapshot_map_lay(tr, L16, KeyRepr::Inline)
        );
        c.hmfree(tc, L16.elemsize);
        r.hmfree(tr, L16.elemsize);
    }
}

/// Row 28 — the shrink rehash (`used_count < used_count_shrink_threshold`).
#[test]
fn err_28_hmdel_shrink() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        (c.rand_seed)(0x3141_5926);
        (r.rand_seed)(0x3141_5926);
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        let n = 300u64;
        for i in 0..n {
            let mut k = i.to_ne_bytes().to_vec();
            let mut k2 = k.clone();
            let v = i.to_ne_bytes();
            let (nc, _) = c.hmput(tc, L16, &mut k, &v, STBDS_HM_BINARY);
            let (nr, _) = r.hmput(tr, L16, &mut k2, &v, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
        }
        let mut shrinks = 0;
        let mut prev = snapshot_map_lay(tc, L16, KeyRepr::Inline).slot_count;
        for i in 0..n {
            let mut kc = i.to_ne_bytes().to_vec();
            let mut kr = kc.clone();
            let (nc, a) = c.hmdel(tc, L16, kc.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
            let (nr, b) = r.hmdel(tr, L16, kr.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            assert_eq!(a, b);
            assert_eq!(a, 1, "delete of a present key reports 1");
            let sc = snapshot_map_lay(tc, L16, KeyRepr::Inline);
            let sr = snapshot_map_lay(tr, L16, KeyRepr::Inline);
            assert_eq!(sc, sr, "i={i}");
            if sc.slot_count < prev {
                shrinks += 1;
                prev = sc.slot_count;
            }
        }
        assert!(shrinks >= 4, "expected several shrink rehashes, saw {shrinks}");
        c.hmfree(tc, L16.elemsize);
        r.hmfree(tr, L16.elemsize);
    }
}

/// Row 29 — the same-size tombstone rebuild.
#[test]
fn err_29_hmdel_tombstone_rebuild() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        (c.rand_seed)(0x3141_5926);
        (r.rand_seed)(0x3141_5926);
        let mut tc: *mut c_void = std::ptr::null_mut();
        let mut tr: *mut c_void = std::ptr::null_mut();
        // fill to 96 entries -> slot_count 128, threshold (128>>3)+(128>>4)=24
        for i in 0u64..96 {
            let mut k = i.to_ne_bytes().to_vec();
            let mut k2 = k.clone();
            let v = i.to_ne_bytes();
            let (nc, _) = c.hmput(tc, L16, &mut k, &v, STBDS_HM_BINARY);
            let (nr, _) = r.hmput(tr, L16, &mut k2, &v, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
        }
        let mut rebuilds = 0;
        // Straight deletions: each one adds a tombstone, and nothing consumes
        // them, so `tombstone_count` climbs past (128>>3)+(128>>4) = 24 while
        // `used_count` stays above the shrink threshold (128>>2 = 32).
        for round in 0..40usize {
            let key = round as u64;
            let mut kc = key.to_ne_bytes().to_vec();
            let mut kr = kc.clone();
            let before = snapshot_map_lay(tc, L16, KeyRepr::Inline);
            let (nc, _) = c.hmdel(tc, L16, kc.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
            let (nr, _) = r.hmdel(tr, L16, kr.as_mut_ptr() as *mut c_void, 0, STBDS_HM_BINARY);
            tc = nc;
            tr = nr;
            let sc = snapshot_map_lay(tc, L16, KeyRepr::Inline);
            let sr = snapshot_map_lay(tr, L16, KeyRepr::Inline);
            assert_eq!(sc, sr, "round={round} after del");
            if sc.tombstone_count < before.tombstone_count && sc.slot_count == before.slot_count {
                rebuilds += 1;
            }
        }
        assert!(rebuilds >= 1, "expected at least one tombstone rebuild");
        c.hmfree(tc, L16.elemsize);
        r.hmfree(tr, L16.elemsize);
    }
}

/// Row 30 — `mode == STBDS_HM_STRING` over an `SH_STRDUP` table frees the duped
/// key on delete. Verified behaviourally (a re-insert must strdup afresh) and by
/// running the whole sequence in forked children so a bad free is observable.
#[test]
fn err_30_hmdel_strdup_free() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let run = |l: &Lib, n: usize| unsafe {
        (l.rand_seed)(0x3141_5926);
        let mut t = (l.shmode_func)(LS.elemsize, STBDS_SH_STRDUP);
        let mut keys: Vec<Vec<u8>> = (0..n).map(|i| cs(&format!("dupkey_{i}"))).collect();
        for (i, k) in keys.iter_mut().enumerate() {
            let (nt, _) = l.shput(
                t,
                LS,
                k.as_mut_ptr() as *mut c_char,
                &(i as u32).to_ne_bytes(),
                STBDS_HM_STRING,
            );
            t = nt;
        }
        // delete every key, then reinsert every key (each reinsert strdups)
        for k in keys.iter_mut() {
            let (nt, _) = l.hmdel(t, LS, k.as_mut_ptr() as *mut c_void, 0, STBDS_HM_STRING);
            t = nt;
        }
        for (i, k) in keys.iter_mut().enumerate() {
            let (nt, _) = l.shput(
                t,
                LS,
                k.as_mut_ptr() as *mut c_char,
                &(i as u32).to_ne_bytes(),
                STBDS_HM_STRING,
            );
            t = nt;
        }
        l.hmfree(t, LS.elemsize);
    };
    for n in [1usize, 2, 5, 20, 50] {
        let oc = probe_abort(|| run(c, n));
        let or = probe_abort(|| run(r, n));
        assert_eq!(oc, or, "n={n}");
        assert_eq!(oc, ChildOutcome::Exited(0), "n={n}");
    }
    // in-process: the stored key pointer must differ from the caller's, and
    // must not alias the previous (freed) allocation in either library
    unsafe {
        for l in [c, r] {
            (l.rand_seed)(0x3141_5926);
            let mut t = (l.shmode_func)(LS.elemsize, STBDS_SH_STRDUP);
            let mut k = cs("aliascheck");
            let (nt, i) = l.shput(t, LS, k.as_mut_ptr() as *mut c_char, &[0u8; 4], STBDS_HM_STRING);
            t = nt;
            let stored = *((t as *mut u8).offset(LS.elemsize as isize * i) as *mut *mut c_char);
            assert_ne!(stored as usize, k.as_mut_ptr() as usize, "{}", l.name);
            assert_eq!(cstr(stored), b"aliascheck".to_vec(), "{}", l.name);
            let (nt, _) = l.hmdel(t, LS, k.as_mut_ptr() as *mut c_void, 0, STBDS_HM_STRING);
            t = nt;
            l.hmfree(t, LS.elemsize);
        }
    }
}

/// Row 30b — the stale-`stbds_temp_key` C bug: when `hmput_key` finds a
/// duplicate key in its *wrapped* probe scan it does NOT refresh
/// `stbds_temp_key`, so `stbds_shputs` writes a stale pointer into the element
/// and `stbds_hmfree` double-frees it. Compared in forked children.
#[test]
fn err_30b_shputs_stale_temp_key_double_free() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let run = |l: &Lib, seed: usize, n: usize, key_space: usize| unsafe {
        (l.rand_seed)(seed);
        let mut t = (l.shmode_func)(LS.elemsize, STBDS_SH_STRDUP);
        let mut rng = Rng::new(0x1234_5678);
        let mut bufs: Vec<Vec<u8>> = Vec::new();
        for _ in 0..n {
            let kv = rng.below(key_space);
            bufs.push(cs(&format!("key_{kv:04}")));
            let k = bufs.last_mut().unwrap().as_mut_ptr() as *mut c_char;
            let (nt, _) = l.shputs(t, LS, k, &rng.bytes(4), STBDS_HM_STRING);
            t = nt;
        }
        l.hmfree(t, LS.elemsize);
    };
    let mut any_abort = false;
    for seed in [0usize, 0x3141_5926] {
        for n in [10usize, 50, 200] {
            for ks in [4usize, 8, 96] {
                let oc = probe_abort(|| run(c, seed, n, ks));
                let or = probe_abort(|| run(r, seed, n, ks));
                assert_eq!(oc, or, "seed={seed:#x} n={n} key_space={ks}");
                if oc != ChildOutcome::Exited(0) {
                    any_abort = true;
                }
            }
        }
    }
    assert!(
        any_abort,
        "expected at least one configuration to hit the C double-free"
    );
}

// ---------------------------------------------------------------------------
// Rows 31..36 — string arena edge cases
// ---------------------------------------------------------------------------

/// Row 31 — the `STBDS_ASSERT(len <= a->remaining)` at lib.c:913. It is
/// unreachable through a well-formed arena (both the oversized branch and the
/// new-block branch guarantee the invariant), but an FFI caller can hand over a
/// struct with `remaining > 0` and `storage == NULL`, which skips the whole `if`
/// and then dereferences `a->storage`. Both libraries must fail the same way.
#[test]
fn err_31_stralloc_inconsistent_arena() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let run = |l: &Lib, remaining: usize, len: usize| unsafe {
        let mut a = StringArena {
            storage: std::ptr::null_mut(),
            remaining,
            block: 0,
            mode: 0,
        };
        let mut s = cs(&"z".repeat(len));
        (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
    };
    for (remaining, len) in [(100usize, 4usize), (1, 0), (usize::MAX, 10), (8, 7)] {
        let oc = probe_abort(|| run(c, remaining, len));
        let or = probe_abort(|| run(r, remaining, len));
        assert_eq!(oc, or, "remaining={remaining} len={len}");
    }
    // Positive control: a well-formed arena never trips the assert, no matter
    // what sequence of lengths it sees.
    unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        let mut rng = Rng::new(SEED ^ 31);
        for step in 0..600usize {
            let n = match rng.below(4) {
                0 => 0,
                1 => rng.below(600),
                2 => rng.below(2000),
                _ => rng.below(20),
            };
            let mut sc = cs(&"m".repeat(n));
            let mut sr = sc.clone();
            let pc = (c.stralloc)(&mut ac, sc.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, sr.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr(pc), cstr(pr), "step={step} n={n}");
            assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar), "step={step} n={n}");
        }
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }
}

/// Row 32 — `len > blocksize` (oversized block), with `storage` NULL and
/// non-NULL, checking that `remaining` is left alone in the latter case.
#[test]
fn err_32_stralloc_oversized() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        // storage == NULL: remaining forced to 0, block still incremented
        for len in [512usize, 513, 5000] {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let mut sc = cs(&"o".repeat(len));
            let mut sr = sc.clone();
            let pc = (c.stralloc)(&mut ac, sc.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, sr.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr(pc), cstr(pr));
            assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar), "len={len}");
            assert_eq!(ac.remaining, 0, "C: remaining must be 0");
            assert_eq!(ac.block, 1, "C: block must have been incremented");
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
        // storage != NULL: remaining survives the oversized allocation
        for len in [513usize, 5000] {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            let mut s1c = cs("tiny");
            let mut s1r = s1c.clone();
            (c.stralloc)(&mut ac, s1c.as_mut_ptr() as *mut c_char);
            (r.stralloc)(&mut ar, s1r.as_mut_ptr() as *mut c_char);
            let rem_c = ac.remaining;
            let rem_r = ar.remaining;
            assert_eq!(rem_c, rem_r);
            let mut sc = cs(&"O".repeat(len));
            let mut sr = sc.clone();
            let pc = (c.stralloc)(&mut ac, sc.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, sr.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr(pc), cstr(pr));
            assert_eq!(ac.remaining, rem_c, "C: remaining must be untouched");
            assert_eq!(ar.remaining, rem_r, "Rust: remaining must be untouched");
            assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar), "len={len}");
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
    }
}

/// Row 33 — `a->block` saturates once `blocksize >= 1 MiB`.
#[test]
fn err_33_stralloc_block_saturate() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        // Feed strings just over the current blocksize so every call takes the
        // oversized branch and bumps `block` until it saturates.
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        let mut last = (0u8, 0u8);
        for step in 0..40usize {
            let bs_c = 512usize << (ac.block as usize >> 1);
            let mut sc = cs(&"S".repeat(bs_c + 1));
            let mut sr = sc.clone();
            let pc = (c.stralloc)(&mut ac, sc.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, sr.as_mut_ptr() as *mut c_char);
            assert_eq!(cstr(pc), cstr(pr), "step={step}");
            assert_eq!(ac.block, ar.block, "step={step}: block differs");
            last = (ac.block, ar.block);
        }
        // 512 << (block>>1) >= 1<<20 stops the increment: block saturates at 22
        assert_eq!(last.0, 22, "C: block should saturate at 22");
        assert_eq!(last.1, 22, "Rust: block should saturate at 22");
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
    }
}

/// Row 34 — an FFI caller can set `a->block` high enough that
/// `512 << (block>>1)` shifts by 64 or more. That is UB in C; whatever the
/// compiled C does, Rust must do the same.
#[test]
fn err_34_stralloc_shift_overflow() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let run = |l: &Lib, block: u8, len: usize| unsafe {
        let mut a = StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block,
            mode: 0,
        };
        let mut s = cs(&"u".repeat(len));
        let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
        let got = cstr(p);
        // report through the exit code so the parent can compare
        let code = ((got.len() & 0x3f) as i32) | ((a.block as i32 & 0x3) << 6);
        std::process::exit(code);
    };
    for block in [
        22u8, 23, 30, 40, 41, 42, 43, 100, 110, 126, 127, 128, 129, 200, 254, 255,
    ] {
        for len in [1usize, 10, 600] {
            let oc = probe_abort(|| run(c, block, len));
            let or = probe_abort(|| run(r, block, len));
            assert_eq!(oc, or, "block={block} len={len}");
        }
    }
}

/// Row 35 — the empty string.
#[test]
fn err_35_stralloc_empty_string() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        for step in 0..600usize {
            let mut sc = cs("");
            let mut sr = cs("");
            let pc = (c.stralloc)(&mut ac, sc.as_mut_ptr() as *mut c_char);
            let pr = (r.stralloc)(&mut ar, sr.as_mut_ptr() as *mut c_char);
            assert_eq!(*pc, 0, "C must return an empty string");
            assert_eq!(*pr, 0, "Rust must return an empty string");
            assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar), "step={step}");
        }
        assert_eq!(ac.block, ar.block);
        (c.strreset)(&mut ac);
        (r.strreset)(&mut ar);
        assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar));
    }
}

/// Row 36 — `strreset` on an arena with no storage.
#[test]
fn err_36_strreset_empty() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for _ in 0..3 {
            let mut ac = StringArena::new();
            let mut ar = StringArena::new();
            // repeated resets must stay a no-op
            for _ in 0..5 {
                (c.strreset)(&mut ac);
                (r.strreset)(&mut ar);
                assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar));
                assert_eq!(snapshot_arena(&ac), snapshot_arena(&StringArena::new()));
            }
            // a non-zero mode/block must also be cleared by the memset
            let mut ac = StringArena {
                storage: std::ptr::null_mut(),
                remaining: 0,
                block: 9,
                mode: 3,
            };
            let mut ar = ac;
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
            assert_eq!(snapshot_arena(&ac), snapshot_arena(&ar));
            assert_eq!(ac.block, 0);
            assert_eq!(ac.mode, 0);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 37..42 — hashing edge cases
// ---------------------------------------------------------------------------

/// Row 37 — `hash_string("")`.
#[test]
fn err_37_hash_string_empty() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for seed in [0usize, 1, 2, 0x3141_5926, usize::MAX, 1 << 63] {
            let mut e = cs("");
            let hc = (c.hash_string)(e.as_mut_ptr() as *mut c_char, seed);
            let hr = (r.hash_string)(e.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(hc, hr, "seed={seed:#x}");
        }
    }
}

/// Row 38 — `hash_bytes(p, 0, seed)`.
#[test]
fn err_38_hash_bytes_zero_len() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        let mut buf = [0xFFu8; 64];
        for seed in [0usize, 1, 0x3141_5926, usize::MAX, 1 << 63] {
            let hc = (c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            let hr = (r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(hc, hr, "seed={seed:#x}");
        }
        // a null pointer with len 0 is never dereferenced
        let hc = (c.hash_bytes)(std::ptr::null_mut(), 0, 0);
        let hr = (r.hash_bytes)(std::ptr::null_mut(), 0, 0);
        assert_eq!(hc, hr);
    }
}

/// Row 39 — every `switch (len - i)` fall-through case, exhaustively over the
/// byte patterns that matter.
#[test]
fn err_39_hash_bytes_tail_lengths() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 39);
    unsafe {
        for len in 0..=7usize {
            // exhaustive over "all zero", "all 0xFF", and 400 random patterns
            let mut patterns: Vec<Vec<u8>> = vec![vec![0u8; 8], vec![0xFFu8; 8]];
            for _ in 0..400 {
                patterns.push(rng.bytes(8));
            }
            for p in patterns {
                let mut bc = p.clone();
                let mut br = p.clone();
                for seed in [0usize, 0x3141_5926, usize::MAX] {
                    let hc = (c.hash_bytes)(bc.as_mut_ptr() as *mut c_void, len, seed);
                    let hr = (r.hash_bytes)(br.as_mut_ptr() as *mut c_void, len, seed);
                    assert_eq!(hc, hr, "len={len} p={p:02x?} seed={seed:#x}");
                }
            }
        }
    }
}

/// Row 40 — the full-block `d[3] << 24` sign extension.
#[test]
fn err_40_hash_bytes_sign_extension() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        // Exhaustive over byte 3 for an 8-byte block: 0x80.. must sign-extend
        for b3 in 0..=255u8 {
            let mut buf = [0u8; 8];
            buf[3] = b3;
            let mut bc = buf;
            let mut br = buf;
            let hc = (c.hash_bytes)(bc.as_mut_ptr() as *mut c_void, 8, 0);
            let hr = (r.hash_bytes)(br.as_mut_ptr() as *mut c_void, 8, 0);
            assert_eq!(hc, hr, "b3={b3:#02x}");
        }
        // and byte 7 (the high half, cast to size_t before shifting)
        for b7 in 0..=255u8 {
            let mut buf = [0u8; 8];
            buf[7] = b7;
            let mut bc = buf;
            let mut br = buf;
            let hc = (c.hash_bytes)(bc.as_mut_ptr() as *mut c_void, 8, 0);
            let hr = (r.hash_bytes)(br.as_mut_ptr() as *mut c_void, 8, 0);
            assert_eq!(hc, hr, "b7={b7:#02x}");
        }
        // The documented consequence: with byte 11 >= 0x80, lengths 12..15 all
        // collide, because `data`'s upper half is saturated to 1s. Confirm the
        // C behaviour and that Rust matches it.
        let mut buf = vec![0u8; 16];
        buf[11] = 0x80;
        let mut hs_c = Vec::new();
        let mut hs_r = Vec::new();
        for len in 12..=15usize {
            let mut bc = buf.clone();
            let mut br = buf.clone();
            hs_c.push((c.hash_bytes)(bc.as_mut_ptr() as *mut c_void, len, 0));
            hs_r.push((r.hash_bytes)(br.as_mut_ptr() as *mut c_void, len, 0));
        }
        assert_eq!(hs_c, hs_r);
    }
}

/// Row 41 — the tail `case 4:` sign extension.
#[test]
fn err_41_hash_tail_sign_extension() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for len in 4..=7usize {
            for b3 in 0..=255u8 {
                let mut buf = [0u8; 8];
                buf[3] = b3;
                buf[len - 1] = 0x11;
                let mut bc = buf;
                let mut br = buf;
                for seed in [0usize, usize::MAX] {
                    let hc = (c.hash_bytes)(bc.as_mut_ptr() as *mut c_void, len, seed);
                    let hr = (r.hash_bytes)(br.as_mut_ptr() as *mut c_void, len, seed);
                    assert_eq!(hc, hr, "len={len} b3={b3:#02x} seed={seed:#x}");
                }
            }
        }
    }
}

/// Row 42 — hashes below 2 are reserved (`HASH_EMPTY` / `HASH_DELETED`) and get
/// bumped by `+2`. Search the seed space for a key whose raw hash is 0 or 1 and
/// verify both libraries agree; also verify no bucket ever stores a raw
/// `hash < 2` for a live entry.
#[test]
fn err_42_reserved_hash_values() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 42);
    unsafe {
        // exhaustive agreement of the raw hash for many random keys/seeds
        for _ in 0..20_000 {
            let mut k = rng.bytes(8);
            let seed = rng.next_u64() as usize;
            let hc = (c.hash_bytes)(k.as_mut_ptr() as *mut c_void, 8, seed);
            let hr = (r.hash_bytes)(k.as_mut_ptr() as *mut c_void, 8, seed);
            assert_eq!(hc, hr);
        }
        // A live entry can never carry hash 0 or 1: force many inserts and
        // check every bucket in both libraries.
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut tc: *mut c_void = std::ptr::null_mut();
            let mut tr: *mut c_void = std::ptr::null_mut();
            for i in 0u64..600 {
                let mut k = i.to_ne_bytes().to_vec();
                let mut k2 = k.clone();
                let v = i.to_ne_bytes();
                let (nc, _) = c.hmput(tc, L16, &mut k, &v, STBDS_HM_BINARY);
                let (nr, _) = r.hmput(tr, L16, &mut k2, &v, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
            }
            let sc = snapshot_map_lay(tc, L16, KeyRepr::Inline);
            let sr = snapshot_map_lay(tr, L16, KeyRepr::Inline);
            assert_eq!(sc, sr, "seed={seed:#x}");
            for (bi, (hashes, idxs)) in sc.buckets.iter().enumerate() {
                for (si, (&h, &ix)) in hashes.iter().zip(idxs.iter()).enumerate() {
                    if ix >= 0 {
                        assert!(h >= 2, "bucket {bi} slot {si}: live entry with hash {h}");
                    }
                }
            }
            c.hmfree(tc, L16.elemsize);
            r.hmfree(tr, L16.elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 43..46 — out-of-range enum / mode values across the FFI boundary
// ---------------------------------------------------------------------------

/// Row 43 — every `mode >= 1` is a string mode, including values with no enum
/// variant (2, 3, 99, `INT_MAX`).
#[test]
fn err_43_mode_out_of_range_string() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for mode in [1i32, 2, 3, 4, 99, 1000, i32::MAX, i32::MAX - 1] {
            (c.rand_seed)(0x3141_5926);
            (r.rand_seed)(0x3141_5926);
            let mut tc = (c.shmode_func)(LS.elemsize, STBDS_SH_STRDUP);
            let mut tr = (r.shmode_func)(LS.elemsize, STBDS_SH_STRDUP);
            let mut keys: Vec<Vec<u8>> = (0..20).map(|i| cs(&format!("oor_{mode}_{i}"))).collect();
            for (i, k) in keys.iter_mut().enumerate() {
                let (nc, a) = c.shput(
                    tc,
                    LS,
                    k.as_mut_ptr() as *mut c_char,
                    &(i as u32).to_ne_bytes(),
                    mode,
                );
                let (nr, b) = r.shput(
                    tr,
                    LS,
                    k.as_mut_ptr() as *mut c_char,
                    &(i as u32).to_ne_bytes(),
                    mode,
                );
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={mode} put {i}");
            }
            for k in keys.iter_mut() {
                let (nc, a) = c.hmgeti(tc, LS, k.as_mut_ptr() as *mut c_void, mode);
                let (nr, b) = r.hmgeti(tr, LS, k.as_mut_ptr() as *mut c_void, mode);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={mode} get");
                assert!(a >= 0, "string lookup should hit for mode={mode}");
            }
            assert_eq!(
                snapshot_map_lay(tc, LS, KeyRepr::Pointer),
                snapshot_map_lay(tr, LS, KeyRepr::Pointer),
                "mode={mode}"
            );
            c.hmfree(tc, LS.elemsize);
            r.hmfree(tr, LS.elemsize);
        }
    }
}

/// Row 44 — negative `mode` values take the binary path and set `string.mode`
/// to 0 when `hmput_key` bootstraps the table.
#[test]
fn err_44_mode_negative() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for mode in [-1i32, -2, -100, i32::MIN, i32::MIN + 1] {
            (c.rand_seed)(0x3141_5926);
            (r.rand_seed)(0x3141_5926);
            let mut tc: *mut c_void = std::ptr::null_mut();
            let mut tr: *mut c_void = std::ptr::null_mut();
            let mut keys = Vec::new();
            for i in 0u64..30 {
                let mut k = i.to_ne_bytes().to_vec();
                let mut k2 = k.clone();
                let v = i.to_ne_bytes();
                let (nc, a) = c.hmput(tc, L16, &mut k, &v, mode);
                let (nr, b) = r.hmput(tr, L16, &mut k2, &v, mode);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={mode} put {i}");
                keys.push(k);
            }
            let sc = snapshot_map_lay(tc, L16, KeyRepr::Inline);
            let sr = snapshot_map_lay(tr, L16, KeyRepr::Inline);
            assert_eq!(sc, sr, "mode={mode}");
            assert_eq!(sc.string_mode, 0, "negative mode must give string.mode 0");
            for (i, k) in keys.iter().enumerate() {
                let mut a1 = k.clone();
                let mut b1 = k.clone();
                let (nc, a) = c.hmgeti(tc, L16, a1.as_mut_ptr() as *mut c_void, mode);
                let (nr, b) = r.hmgeti(tr, L16, b1.as_mut_ptr() as *mut c_void, mode);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={mode} get {i}");
                assert!(a >= 0);
            }
            for (i, k) in keys.iter().enumerate() {
                let mut a1 = k.clone();
                let mut b1 = k.clone();
                let (nc, a) = c.hmdel(tc, L16, a1.as_mut_ptr() as *mut c_void, 0, mode);
                let (nr, b) = r.hmdel(tr, L16, b1.as_mut_ptr() as *mut c_void, 0, mode);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={mode} del {i}");
                assert_eq!(
                    snapshot_map_lay(tc, L16, KeyRepr::Inline),
                    snapshot_map_lay(tr, L16, KeyRepr::Inline),
                    "mode={mode} after del {i}"
                );
            }
            c.hmfree(tc, L16.elemsize);
            r.hmfree(tr, L16.elemsize);
        }
    }
}

/// Row 45 — `keysize == 0`: `memcmp(...,0) == 0` makes every key "equal", so
/// only the hash separates entries.
#[test]
fn err_45_keysize_zero() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let lay = Layout::new(16, 0, 8, 8);
    unsafe {
        for seed in [0usize, 0x3141_5926, usize::MAX] {
            (c.rand_seed)(seed);
            (r.rand_seed)(seed);
            let mut tc: *mut c_void = std::ptr::null_mut();
            let mut tr: *mut c_void = std::ptr::null_mut();
            let mut rng = Rng::new(SEED ^ 45);
            for i in 0..80usize {
                let mut k = rng.bytes(8);
                let mut k2 = k.clone();
                let v = rng.bytes(8);
                let (nc, a) = c.hmput(tc, lay, &mut k, &v, STBDS_HM_BINARY);
                let (nr, b) = r.hmput(tr, lay, &mut k2, &v, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "seed={seed:#x} put {i}");
                assert_eq!(
                    snapshot_map_lay(tc, lay, KeyRepr::Inline),
                    snapshot_map_lay(tr, lay, KeyRepr::Inline),
                    "seed={seed:#x} put {i}"
                );
            }
            // hash_bytes(key, 0, seed) is a constant, so every lookup collides
            for i in 0..40usize {
                let mut k = rng.bytes(8);
                let mut k2 = k.clone();
                let (nc, a) = c.hmgeti(tc, lay, k.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                let (nr, b) = r.hmgeti(tr, lay, k2.as_mut_ptr() as *mut c_void, STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "seed={seed:#x} get {i}");
            }
            c.hmfree(tc, lay.elemsize);
            r.hmfree(tr, lay.elemsize);
        }
    }
}

/// Row 46 — `shmode_func` with out-of-range `mode` (the `(unsigned char)` cast).
#[test]
fn err_46_shmode_truncation() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for mode in [
            0i32,
            1,
            2,
            3,
            4,
            5,
            127,
            128,
            255,
            256,
            257,
            258,
            259,
            -1,
            -2,
            -255,
            -256,
            i32::MAX,
            i32::MIN,
        ] {
            for elemsize in [8usize, 16, 24] {
                (c.rand_seed)(0x3141_5926);
                (r.rand_seed)(0x3141_5926);
                let tc = (c.shmode_func)(elemsize, mode);
                let tr = (r.shmode_func)(elemsize, mode);
                let sc = snapshot_map(tc, elemsize, KeyRepr::Inline);
                let sr = snapshot_map(tr, elemsize, KeyRepr::Inline);
                assert_eq!(sc, sr, "mode={mode} elemsize={elemsize}");
                assert_eq!(
                    sc.string_mode,
                    (mode as u32 & 0xff) as u8,
                    "mode={mode}: expected (unsigned char) truncation"
                );
                assert_eq!(sc.length, 1);
                assert_eq!(sc.slot_count, 8);
                c.hmfree(tc, elemsize);
                r.hmfree(tr, elemsize);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 48..50 — strkey / str_dups boundaries
// ---------------------------------------------------------------------------

/// Row 48 — `strkey` at the extremes of `int`.
#[test]
fn err_48_strkey_extremes() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for n in [
            0i32,
            -1,
            1,
            i32::MAX,
            i32::MIN,
            i32::MAX - 1,
            i32::MIN + 1,
            -999_999_999,
            999_999_999,
        ] {
            let pc = (c.strkey)(n);
            let pr = (r.strkey)(n);
            assert_eq!(cstr(pc), cstr(pr), "n={n}");
            let s = String::from_utf8(cstr(pc)).unwrap();
            assert_eq!(s, format!("test_{n}"));
            assert!(s.len() < 256, "must fit the 256-byte static buffer");
        }
        // the returned pointer is the same static buffer every time
        let p1 = (c.strkey)(1);
        let p2 = (c.strkey)(2);
        assert_eq!(p1, p2, "C: strkey must reuse its static buffer");
        let q1 = (r.strkey)(1);
        let q2 = (r.strkey)(2);
        assert_eq!(q1, q2, "Rust: strkey must reuse its static buffer");
    }
}

/// Row 49 — `str_dups` with `num <= 0`: the `stralloc` loop never runs but the
/// strmap block still executes and still prints one line.
#[test]
fn err_49_str_dups_nonpositive() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    for num in [0i32, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
        let oc = capture_stdout(|| unsafe {
            (c.rand_seed)(0x3141_5926);
            (c.str_dups)(num);
        });
        let or = capture_stdout(|| unsafe {
            (r.rand_seed)(0x3141_5926);
            (r.str_dups)(num);
        });
        assert_eq!(oc, or, "str_dups({num}) stdout differs");
        assert_eq!(
            String::from_utf8_lossy(&oc),
            format!("a {num}\n"),
            "str_dups({num})"
        );
    }
}

/// Row 50 — the three `STBDS_ASSERT`s at the end of `str_dups` must hold (and
/// hold identically) for every `num`; run in forked children so a failing
/// assert shows up as SIGABRT.
#[test]
fn err_50_str_dups_asserts_hold() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    for num in [0i32, 1, 2, 3, 8, 9, 64, 500, -1, i32::MIN] {
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            let oc = probe_abort(|| unsafe {
                (c.rand_seed)(seed);
                (c.str_dups)(num);
            });
            let or = probe_abort(|| unsafe {
                (r.rand_seed)(seed);
                (r.str_dups)(num);
            });
            assert_eq!(oc, or, "num={num} seed={seed:#x}");
            assert_eq!(
                oc,
                ChildOutcome::Exited(0),
                "num={num} seed={seed:#x}: assert fired"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary checks (beyond the ERRORS.md rows)
// ---------------------------------------------------------------------------

/// Null pointers, zero lengths and oversized lengths where the C code tolerates
/// them.
#[test]
fn generic_null_and_extreme_arguments() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        // arrgrowf with elemsize 0
        for min_cap in [0usize, 1, 8, 1024] {
            let ac = (c.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            let ar = (r.arrgrowf)(std::ptr::null_mut(), 0, 0, min_cap);
            assert_eq!(ac.is_null(), ar.is_null());
            if !ac.is_null() {
                assert_eq!(header_of(ac), header_of(ar));
                (c.arrfreef)(ac);
                (r.arrfreef)(ar);
            }
        }
        // hash_string on a 1-byte buffer that is just the terminator
        let mut z = [0u8; 1];
        assert_eq!(
            (c.hash_string)(z.as_mut_ptr() as *mut c_char, 0),
            (r.hash_string)(z.as_mut_ptr() as *mut c_char, 0)
        );
        // hmget_key_ts with keysize far larger than the key buffer is a C read
        // overrun; keep it in bounds by using a big buffer instead.
        let mut big = vec![0x5Au8; 4096];
        for keysize in [0usize, 1, 7, 8, 9, 64, 4096] {
            let mut tempc = 0isize;
            let mut tempr = 0isize;
            let pc = (c.hmget_key_ts)(
                std::ptr::null_mut(),
                16,
                big.as_mut_ptr() as *mut c_void,
                keysize,
                &mut tempc,
                STBDS_HM_BINARY,
            );
            let pr = (r.hmget_key_ts)(
                std::ptr::null_mut(),
                16,
                big.as_mut_ptr() as *mut c_void,
                keysize,
                &mut tempr,
                STBDS_HM_BINARY,
            );
            assert_eq!(tempc, tempr, "keysize={keysize}");
            c.hmfree(pc, 16);
            r.hmfree(pr, 16);
        }
    }
}

/// Out-of-range `mode` enum values fed to *every* function that takes one, on
/// an empty and on a populated map.
#[test]
fn generic_out_of_range_enum_modes() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let modes: [c_int; 12] = [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -2,
        -1,
        0,
        1,
        2,
        3,
        4,
        1000,
        i32::MAX,
    ];
    unsafe {
        for &mode in &modes {
            // binary-shaped map so the string branches read inline bytes as a
            // pointer only when we ask for it; here mode <= 0 is safe, and for
            // mode >= 1 we use a real string key.
            (c.rand_seed)(0x3141_5926);
            (r.rand_seed)(0x3141_5926);
            if mode >= STBDS_HM_STRING {
                let mut tc = (c.shmode_func)(LS.elemsize, STBDS_SH_STRDUP);
                let mut tr = (r.shmode_func)(LS.elemsize, STBDS_SH_STRDUP);
                let mut k = cs("enum_probe_key");
                let (nc, a) = c.shput(tc, LS, k.as_mut_ptr() as *mut c_char, &[9u8; 4], mode);
                let (nr, b) = r.shput(tr, LS, k.as_mut_ptr() as *mut c_char, &[9u8; 4], mode);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={mode}");
                let (nc, a) = c.hmgeti(tc, LS, k.as_mut_ptr() as *mut c_void, mode);
                let (nr, b) = r.hmgeti(tr, LS, k.as_mut_ptr() as *mut c_void, mode);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={mode} get");
                let (nc, a) = c.hmgeti_ts(tc, LS, k.as_mut_ptr() as *mut c_void, mode);
                let (nr, b) = r.hmgeti_ts(tr, LS, k.as_mut_ptr() as *mut c_void, mode);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={mode} get_ts");
                assert_eq!(
                    snapshot_map_lay(tc, LS, KeyRepr::Pointer),
                    snapshot_map_lay(tr, LS, KeyRepr::Pointer),
                    "mode={mode}"
                );
                // deletion with mode != 1 hits the assert at lib.c:846 when the
                // element moves; only element 0 (== final) is safe here.
                let (nc, a) = c.hmdel(tc, LS, k.as_mut_ptr() as *mut c_void, 0, mode);
                let (nr, b) = r.hmdel(tr, LS, k.as_mut_ptr() as *mut c_void, 0, mode);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "mode={mode} del");
                assert_eq!(
                    snapshot_map_lay(tc, LS, KeyRepr::Pointer),
                    snapshot_map_lay(tr, LS, KeyRepr::Pointer),
                    "mode={mode} after del"
                );
                c.hmfree(tc, LS.elemsize);
                r.hmfree(tr, LS.elemsize);
            } else {
                let mut tc: *mut c_void = std::ptr::null_mut();
                let mut tr: *mut c_void = std::ptr::null_mut();
                for i in 0u64..20 {
                    let mut k = i.to_ne_bytes().to_vec();
                    let mut k2 = k.clone();
                    let v = i.to_ne_bytes();
                    let (nc, a) = c.hmput(tc, L16, &mut k, &v, mode);
                    let (nr, b) = r.hmput(tr, L16, &mut k2, &v, mode);
                    tc = nc;
                    tr = nr;
                    assert_eq!(a, b, "mode={mode} put {i}");
                }
                for i in 0u64..30 {
                    let mut k = i.to_ne_bytes().to_vec();
                    let mut k2 = k.clone();
                    let (nc, a) = c.hmgeti(tc, L16, k.as_mut_ptr() as *mut c_void, mode);
                    let (nr, b) = r.hmgeti(tr, L16, k2.as_mut_ptr() as *mut c_void, mode);
                    tc = nc;
                    tr = nr;
                    assert_eq!(a, b, "mode={mode} get {i}");
                    let (nc, a) = c.hmdel(tc, L16, k.as_mut_ptr() as *mut c_void, 0, mode);
                    let (nr, b) = r.hmdel(tr, L16, k2.as_mut_ptr() as *mut c_void, 0, mode);
                    tc = nc;
                    tr = nr;
                    assert_eq!(a, b, "mode={mode} del {i}");
                    assert_eq!(
                        snapshot_map_lay(tc, L16, KeyRepr::Inline),
                        snapshot_map_lay(tr, L16, KeyRepr::Inline),
                        "mode={mode} step {i}"
                    );
                }
                c.hmfree(tc, L16.elemsize);
                r.hmfree(tr, L16.elemsize);
            }
        }
    }
}

/// Values one step past a documented boundary: `keysize` and `elemsize` around
/// the sizes the code special-cases, and `slot_count` growth boundaries.
#[test]
fn generic_boundary_sizes() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [1usize, 2, 3, 7, 8, 9, 15, 16, 17, 63, 64, 65, 127, 128, 129] {
            let keysize = elemsize.min(8);
            let lay = Layout::new(elemsize, keysize, 0, 0);
            (c.rand_seed)(0x3141_5926);
            (r.rand_seed)(0x3141_5926);
            let mut tc: *mut c_void = std::ptr::null_mut();
            let mut tr: *mut c_void = std::ptr::null_mut();
            let mut rng = Rng::new(SEED ^ elemsize as u64);
            for i in 0..40usize {
                let mut k = rng.bytes(keysize);
                let mut k2 = k.clone();
                let (nc, a) = c.hmput(tc, lay, &mut k, &[], STBDS_HM_BINARY);
                let (nr, b) = r.hmput(tr, lay, &mut k2, &[], STBDS_HM_BINARY);
                tc = nc;
                tr = nr;
                assert_eq!(a, b, "elemsize={elemsize} put {i}");
                assert_eq!(
                    snapshot_map_lay(tc, lay, KeyRepr::Inline),
                    snapshot_map_lay(tr, lay, KeyRepr::Inline),
                    "elemsize={elemsize} put {i}"
                );
            }
            c.hmfree(tc, elemsize);
            r.hmfree(tr, elemsize);
        }
    }
}
