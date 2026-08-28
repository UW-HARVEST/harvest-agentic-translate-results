//! Phase C — one differential test per row of ERRORS.md.
//!
//! Every test constructs the exact rejecting condition, calls BOTH `.so`s and
//! asserts they return the *same* sentinel (`NULL`, the unchanged pointer,
//! `STBDS_INDEX_EMPTY == -1`, `temp == 0`, …) — not merely "both failed".

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ===========================================================================
// A probe simulator, so the tests can *prove* they reach both the
// `for (i = pos&7 .. 7)` scan and the `for (i = 0 .. pos&7)` wrap-around scan,
// and that probes really do traverse STBDS_HASH_DELETED tombstones.
// ===========================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Probe {
    /// "first" = terminated in the `i = pos&7 .. 7` scan,
    /// "second" = terminated in the `i = 0 .. pos&7` wrap-around scan.
    scan: &'static str,
    /// "empty" = hit `STBDS_HASH_EMPTY`, "match" = hit an equal hash.
    kind: &'static str,
    tombstones: usize,
    buckets: usize,
}

unsafe fn key_eq(
    t: *mut c_void,
    es: usize,
    ks: usize,
    mode: c_int,
    keyoffset: usize,
    key: &[u8],
    idx: isize,
) -> bool {
    unsafe {
        let addr = (t as *mut u8)
            .wrapping_offset((es as isize) * idx)
            .wrapping_add(keyoffset);
        if mode >= HM_STRING {
            let stored = *(addr as *mut *mut c_char);
            if stored.is_null() {
                return false;
            }
            cstr_opt(stored) == cstr_opt(key.as_ptr() as *const c_char)
        } else {
            std::slice::from_raw_parts(addr, ks) == &key[..ks]
        }
    }
}

/// Replays `stbds_hm_find_slot`'s probe sequence without touching the library.
unsafe fn probe(
    l: &Lib,
    t: *mut c_void,
    es: usize,
    ks: usize,
    mode: c_int,
    keyoffset: usize,
    key: &[u8],
) -> Option<Probe> {
    unsafe {
        if t.is_null() {
            return None;
        }
        let ti = (*header(hash_to_arr(t, es))).hash_table as *mut HashIndex;
        if ti.is_null() {
            return None;
        }
        let seed = (*ti).seed;
        let sc = (*ti).slot_count;
        let mut hash = if mode >= HM_STRING {
            (l.hash_string)(key.as_ptr() as *mut c_char, seed)
        } else {
            (l.hash_bytes)(key.as_ptr() as *mut c_void, ks, seed)
        };
        if hash < 2 {
            hash = hash.wrapping_add(2);
        }
        let mut pos = hash & (sc - 1);
        let mut step = 8usize;
        let mut tombstones = 0usize;
        let mut buckets = 0usize;
        loop {
            buckets += 1;
            assert!(buckets < 10_000, "probe did not terminate");
            let b = (*ti).storage.add(pos >> 3);
            for i in (pos & 7)..8 {
                if (*b).hash[i] == hash {
                    if key_eq(t, es, ks, mode, keyoffset, key, (*b).index[i]) {
                        return Some(Probe { scan: "first", kind: "match", tombstones, buckets });
                    }
                } else if (*b).hash[i] == 0 {
                    return Some(Probe { scan: "first", kind: "empty", tombstones, buckets });
                } else if (*b).hash[i] == 1 {
                    tombstones += 1;
                }
            }
            for i in 0..(pos & 7) {
                if (*b).hash[i] == hash {
                    if key_eq(t, es, ks, mode, keyoffset, key, (*b).index[i]) {
                        return Some(Probe { scan: "second", kind: "match", tombstones, buckets });
                    }
                } else if (*b).hash[i] == 0 {
                    return Some(Probe { scan: "second", kind: "empty", tombstones, buckets });
                } else if (*b).hash[i] == 1 {
                    tombstones += 1;
                }
            }
            pos = pos.wrapping_add(step);
            step += 8;
            pos &= sc - 1;
        }
    }
}

fn bkeys(n: usize, salt: u64) -> Vec<Vec<u8>> {
    (0..n)
        .map(|i| bin_key(i as u64 * 0x9E37_79B9_7F4A_7C15 + salt, 8))
        .collect()
}

fn strkeys(n: usize, prefix: &str) -> Vec<Vec<u8>> {
    (0..n)
        .map(|i| padded_key(format!("{prefix}{i:05}").as_bytes()))
        .collect()
}

// ===========================================================================
// R1 / R2 / R3 — stbds_arrgrowf's rejection and fresh-array branches
// ===========================================================================

#[test]
fn r1_arrgrowf_early_out_returns_input_pointer_unchanged() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &es in &[1usize, 8, 16, 24, 64] {
            let ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 32);
            let ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 32);
            (*header(ca)).length = 10;
            (*header(ra)).length = 10;
            (*header(ca)).temp = 77;
            (*header(ra)).temp = 77;
            for &(addlen, min_cap) in &[(0usize, 0usize), (0, 1), (0, 31), (0, 32), (5, 0), (22, 32)]
            {
                let cbefore = snap_hdr(ca);
                let cp = (c.arrgrowf)(ca, es, addlen, min_cap);
                let rp = (r.arrgrowf)(ra, es, addlen, min_cap);
                assert_eq!(cp, ca, "C: pointer must be returned unchanged");
                assert_eq!(rp, ra, "Rust: pointer must be returned unchanged");
                assert_eq!(cbefore, snap_hdr(cp), "C: header must be untouched");
                assert_eq!(cbefore, snap_hdr(rp), "Rust: header must be untouched");
            }
            (c.arrfreef)(ca);
            (r.arrfreef)(ra);
        }
    }
}

#[test]
fn r2_arrgrowf_null_zero_zero_returns_null() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &es in &[0usize, 1, 8, 16, 1024, usize::MAX] {
            let cp = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let rp = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(cp.is_null() && rp.is_null(), "es={es}: both must return NULL");
        }
    }
}

#[test]
fn r3_arrgrowf_fresh_array_header_is_fully_initialised() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &es in &[0usize, 1, 8, 16, 24] {
            for &(addlen, min_cap) in &[(0usize, 1usize), (1, 0), (0, 4), (7, 0), (0, 100), (9, 3)] {
                let cp = (c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                let rp = (r.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                assert!(!cp.is_null() && !rp.is_null());
                assert_eq!(snap_hdr(cp), snap_hdr(rp), "es={es} {addlen}/{min_cap}");
                assert_eq!((*header(cp)).length, 0);
                assert_eq!((*header(cp)).temp, 0);
                assert!((*header(cp)).hash_table.is_null());
                assert_eq!((*header(rp)).length, 0);
                assert_eq!((*header(rp)).temp, 0);
                assert!((*header(rp)).hash_table.is_null());
                (c.arrfreef)(cp);
                (r.arrfreef)(rp);
            }
        }
    }
}

// ===========================================================================
// R4 / R5 — stbds_hmfree_func's null and no-table branches
// ===========================================================================

#[test]
fn r4_hmfree_func_null_is_a_noop() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &es in &[0usize, 1, 8, 16, 24, usize::MAX] {
            (c.hmfree_func)(std::ptr::null_mut(), es);
            (r.hmfree_func)(std::ptr::null_mut(), es);
        }
        // still alive and functional afterwards
        let ca = (c.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        let ra = (r.arrgrowf)(std::ptr::null_mut(), 8, 0, 4);
        assert_eq!(snap_hdr(ca), snap_hdr(ra));
        (c.arrfreef)(ca);
        (r.arrfreef)(ra);
    }
}

#[test]
fn r5_hmfree_func_with_null_hash_table() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &es in &[1usize, 8, 16, 24] {
            for &len in &[0usize, 1, 3] {
                let ca = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
                let ra = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
                (*header(ca)).length = len;
                (*header(ra)).length = len;
                assert!((*header(ca)).hash_table.is_null());
                assert!((*header(ra)).hash_table.is_null());
                // must not touch the (absent) table nor sweep any strings
                (c.hmfree_func)(ca, es);
                (r.hmfree_func)(ra, es);
            }
        }
    }
}

// ===========================================================================
// R6 / R7 / R35 — stbds_hm_find_slot's two `return -1` sites, and probes that
// have to walk *through* tombstones
// ===========================================================================

#[test]
fn r6_r7_r35_find_slot_returns_minus_one_from_both_scans() {
    let _g = lock();
    let (c, r) = both();
    let mut first = 0usize;
    let mut second = 0usize;
    let mut through_tombstones = 0usize;
    let mut multi_bucket = 0usize;

    unsafe {
        for seed in 0..300usize {
            let present = strkeys(14, "p");
            let absent = strkeys(14, "Q");
            sync_seed(seed.wrapping_mul(0x9E37_79B9) | 1);
            let mut cd = Drv::shmode(c, 16, 8, HM_STRING, SH_ARENA);
            let mut rd = Drv::shmode(r, 16, 8, HM_STRING, SH_ARENA);
            for (i, k) in present.iter().enumerate() {
                assert_eq!(cd.put(k, i as u8), rd.put(k, i as u8));
            }
            // create tombstones
            for k in present.iter().take(5) {
                assert_eq!(cd.del(k, 0), rd.del(k, 0));
            }
            eqs(&format!("r6r7 seed={seed} after deletes"), &cd.snap(), &rd.snap());

            for k in absent.iter() {
                // both libraries must agree on the sentinel
                let a = cd.get_ts(k);
                let b = rd.get_ts(k);
                assert_eq!(a, b, "seed={seed}: hmget_key_ts sentinel");
                assert_eq!(a, INDEX_EMPTY, "absent key must yield STBDS_INDEX_EMPTY");
                let ca = cd.get(k);
                let rb = rd.get(k);
                assert_eq!(ca, rb);
                assert_eq!(ca, INDEX_EMPTY);
                // hmdel on an absent key: temp == 0, map otherwise unchanged (R14)
                let cbefore = mask_temp(&cd.snap());
                let rbefore = mask_temp(&rd.snap());
                let cdel = cd.del(k, 0);
                let rdel = rd.del(k, 0);
                assert_eq!(cdel, rdel, "seed={seed}: hmdel sentinel");
                assert_eq!(cdel, 0, "hmdel of an absent key sets temp = 0");
                assert_eq!(cbefore, mask_temp(&cd.snap()), "C: rejected delete mutated state");
                assert_eq!(rbefore, mask_temp(&rd.snap()), "Rust: rejected delete mutated state");
                eqs(&format!("r14-inline seed={seed}"), &cd.snap(), &rd.snap());

                // which branch actually terminated the probe?
                let pc = probe(c, cd.t, 16, 8, HM_STRING, 0, k).unwrap();
                let pr = probe(r, rd.t, 16, 8, HM_STRING, 0, k).unwrap();
                assert_eq!(pc, pr, "seed={seed}: probe shape must match");
                assert_eq!(pc.kind, "empty");
                if pc.scan == "first" {
                    first += 1;
                } else {
                    second += 1;
                }
                if pc.tombstones > 0 {
                    through_tombstones += 1;
                }
                if pc.buckets > 1 {
                    multi_bucket += 1;
                }
            }
            cd.free();
            rd.free();
        }
    }

    // R6 and R7 are genuinely distinct branches; both must have been reached.
    assert!(first > 0, "R6 (first-scan `return -1`) never reached");
    assert!(second > 0, "R7 (wrap-around-scan `return -1`) never reached");
    assert!(
        through_tombstones > 0,
        "R35 (probe traversing STBDS_HASH_DELETED) never reached"
    );
    assert!(multi_bucket > 0, "multi-bucket probe never reached");
    eprintln!(
        "coverage: first-scan={first} second-scan={second} \
         via-tombstones={through_tombstones} multi-bucket={multi_bucket}"
    );
}

// ===========================================================================
// R8 / R9 / R10 / R11 — the three ways a lookup reports "absent"
// ===========================================================================

#[test]
fn r8_hmget_key_ts_on_null_map() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &es in &[1usize, 8, 16, 24] {
            for &ks in &[0usize, 1, 4, 8] {
                let k = bin_key(0x1234_5678, ks.max(1));
                let mut ct: isize = 0x7777;
                let mut rt: isize = 0x7777;
                let cp = (c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    es,
                    k.as_ptr() as *mut c_void,
                    ks,
                    &mut ct,
                    HM_BINARY,
                );
                let rp = (r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    es,
                    k.as_ptr() as *mut c_void,
                    ks,
                    &mut rt,
                    HM_BINARY,
                );
                assert_eq!(ct, rt, "es={es} ks={ks}: *temp");
                assert_eq!(ct, INDEX_EMPTY, "must be STBDS_INDEX_EMPTY");
                assert!(!cp.is_null() && !rp.is_null());
                eqs(
                    &format!("r8 es={es} ks={ks}"),
                    &snap_map(cp, es, KeyKind::Bin),
                    &snap_map(rp, es, KeyKind::Bin),
                );
                // a sentinel element was allocated and zeroed, and there is no table
                assert_eq!(hmlen(cp, es), 0);
                assert_eq!(hmlen(rp, es), 0);
                assert!((*header(hash_to_arr(cp, es))).hash_table.is_null());
                assert!((*header(hash_to_arr(rp, es))).hash_table.is_null());
                (c.hmfree_func)(hash_to_arr(cp, es), es);
                (r.hmfree_func)(hash_to_arr(rp, es), es);
            }
        }
    }
}

#[test]
fn r9_hmget_key_ts_with_null_hash_table() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &es in &[8usize, 16, 24] {
            // build a map that has an element but no hash table, via hmput_default
            let ct = (c.hmput_default)(std::ptr::null_mut(), es);
            let rt = (r.hmput_default)(std::ptr::null_mut(), es);
            assert!((*header(hash_to_arr(ct, es))).hash_table.is_null());
            assert!((*header(hash_to_arr(rt, es))).hash_table.is_null());

            let k = bin_key(99, 8);
            let mut ctmp: isize = 0x4242;
            let mut rtmp: isize = 0x4242;
            let cp = (c.hmget_key_ts)(ct, es, k.as_ptr() as *mut c_void, 8, &mut ctmp, HM_BINARY);
            let rp = (r.hmget_key_ts)(rt, es, k.as_ptr() as *mut c_void, 8, &mut rtmp, HM_BINARY);
            assert_eq!(ctmp, rtmp);
            assert_eq!(ctmp, -1, "table == NULL must give *temp = -1");
            assert_eq!(cp, ct, "must return `a` unchanged");
            assert_eq!(rp, rt, "must return `a` unchanged");

            // and hmget_key writes the same -1 into header->temp (R11)
            let cq = (c.hmget_key)(ct, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
            let rq = (r.hmget_key)(rt, es, k.as_ptr() as *mut c_void, 8, HM_BINARY);
            assert_eq!(temp_of(cq, es), -1);
            assert_eq!(temp_of(rq, es), -1);
            eqs(
                &format!("r9 es={es}"),
                &snap_map(cq, es, KeyKind::Bin),
                &snap_map(rq, es, KeyKind::Bin),
            );
            (c.hmfree_func)(hash_to_arr(cq, es), es);
            (r.hmfree_func)(hash_to_arr(rq, es), es);
        }
    }
}

#[test]
fn r10_r11_absent_key_sentinels() {
    let _g = lock();
    let (c, r) = both();
    for &mode in &[HM_BINARY, HM_STRING] {
        for seed in 0..40usize {
            sync_seed(seed * 4099 + 1);
            let (present, absent) = if mode == HM_STRING {
                (strkeys(20, "y"), strkeys(20, "Z"))
            } else {
                (bkeys(20, 11), bkeys(20, 0xABCD_1234))
            };
            let sh = if mode == HM_STRING { SH_STRDUP } else { SH_NONE };
            let mut cd = Drv::shmode(c, 16, 8, mode, sh);
            let mut rd = Drv::shmode(r, 16, 8, mode, sh);
            unsafe {
                for (i, k) in present.iter().enumerate() {
                    assert_eq!(cd.put(k, i as u8), rd.put(k, i as u8));
                }
                for k in absent.iter() {
                    assert_eq!(cd.get_ts(k), rd.get_ts(k));
                    assert_eq!(cd.get_ts(k), INDEX_EMPTY);
                    assert_eq!(cd.get(k), rd.get(k));
                    assert_eq!(cd.get(k), INDEX_EMPTY);
                    // hmgetp_null()'s test: `hmgeti(t,k) == -1 ? NULL : ...`
                    assert_eq!(temp_of(cd.t, 16), -1);
                    assert_eq!(temp_of(rd.t, 16), -1);
                }
                // present keys must NOT report -1
                for (i, k) in present.iter().enumerate() {
                    let a = cd.get(k);
                    assert_eq!(a, rd.get(k));
                    assert_ne!(a, -1, "present key #{i} reported absent");
                }
                cd.free();
                rd.free();
            }
        }
    }
}

// ===========================================================================
// R12 / R13 / R14 / R15 — stbds_hmdel_key's rejection branches
// ===========================================================================

#[test]
fn r12_hmdel_key_null_returns_null() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        let k = bin_key(5, 8);
        for &es in &[0usize, 1, 8, 16, 24] {
            for &ks in &[0usize, 1, 8] {
                for &off in &[0usize, 4, 1024] {
                    for &mode in &[HM_BINARY, HM_STRING, -1, 7] {
                        let cp = (c.hmdel_key)(
                            std::ptr::null_mut(),
                            es,
                            k.as_ptr() as *mut c_void,
                            ks,
                            off,
                            mode,
                        );
                        let rp = (r.hmdel_key)(
                            std::ptr::null_mut(),
                            es,
                            k.as_ptr() as *mut c_void,
                            ks,
                            off,
                            mode,
                        );
                        assert!(cp.is_null(), "C hmdel_key(NULL) must return NULL");
                        assert!(rp.is_null(), "Rust hmdel_key(NULL) must return NULL");
                    }
                }
            }
        }
        // and with a NULL key too
        let cp = (c.hmdel_key)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 0, 0, HM_BINARY);
        let rp = (r.hmdel_key)(std::ptr::null_mut(), 16, std::ptr::null_mut(), 0, 0, HM_BINARY);
        assert!(cp.is_null() && rp.is_null());
    }
}

#[test]
fn r13_hmdel_key_with_null_hash_table_sets_temp_zero() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &es in &[8usize, 16, 24] {
            let ct = (c.hmput_default)(std::ptr::null_mut(), es);
            let rt = (r.hmput_default)(std::ptr::null_mut(), es);
            // poison temp so the write is observable
            (*header(hash_to_arr(ct, es))).temp = 1234;
            (*header(hash_to_arr(rt, es))).temp = 1234;
            let k = bin_key(3, 8);
            let cp = (c.hmdel_key)(ct, es, k.as_ptr() as *mut c_void, 8, 0, HM_BINARY);
            let rp = (r.hmdel_key)(rt, es, k.as_ptr() as *mut c_void, 8, 0, HM_BINARY);
            assert_eq!(cp, ct, "must return `a` unchanged");
            assert_eq!(rp, rt, "must return `a` unchanged");
            assert_eq!(temp_of(cp, es), 0, "temp must be cleared to 0 first");
            assert_eq!(temp_of(rp, es), 0, "temp must be cleared to 0 first");
            assert_eq!(hmlen(cp, es), 0);
            assert_eq!(hmlen(rp, es), 0);
            eqs(
                &format!("r13 es={es}"),
                &snap_map(cp, es, KeyKind::Bin),
                &snap_map(rp, es, KeyKind::Bin),
            );
            (c.hmfree_func)(hash_to_arr(cp, es), es);
            (r.hmfree_func)(hash_to_arr(rp, es), es);
        }
    }
}

#[test]
fn r14_hmdel_key_absent_leaves_everything_unchanged() {
    let _g = lock();
    let (c, r) = both();
    for seed in 0..40usize {
        sync_seed(seed * 7717 + 3);
        let present = bkeys(15, 1);
        let absent = bkeys(15, 0xDEAD_0000);
        let mut cd = Drv::shmode(c, 16, 8, HM_BINARY, SH_NONE);
        let mut rd = Drv::shmode(r, 16, 8, HM_BINARY, SH_NONE);
        unsafe {
            for (i, k) in present.iter().enumerate() {
                assert_eq!(cd.put(k, i as u8), rd.put(k, i as u8));
            }
            for k in absent.iter() {
                let cbefore = mask_temp(&cd.snap());
                let rbefore = mask_temp(&rd.snap());
                let ca = cd.del(k, 0);
                let ra = rd.del(k, 0);
                assert_eq!(ca, ra);
                assert_eq!(ca, 0, "absent hmdel must yield temp == 0");
                // length / used_count / tombstone_count must be untouched: the
                // only field that may change is `temp`.
                assert_eq!(
                    cbefore,
                    mask_temp(&cd.snap()),
                    "C state changed by a rejected delete"
                );
                assert_eq!(
                    rbefore,
                    mask_temp(&rd.snap()),
                    "Rust state changed by a rejected delete"
                );
                eqs("r14 cross", &cd.snap(), &rd.snap());
            }
            cd.free();
            rd.free();
        }
    }
}

#[test]
fn r15_hmdel_key_nonzero_keyoffset_is_rejected() {
    let _g = lock();
    let (c, r) = both();
    for &(es, ks) in &[(16usize, 8usize), (24, 8), (32, 16), (16, 4)] {
        for &off in &[1usize, 2, 4, 8, 12] {
            if off + ks > es {
                continue;
            }
            sync_seed(0x1515);
            let keys = bkeys(10, 5);
            let mut cd = Drv::shmode(c, es, ks, HM_BINARY, SH_NONE);
            let mut rd = Drv::shmode(r, es, ks, HM_BINARY, SH_NONE);
            unsafe {
                for (i, k) in keys.iter().enumerate() {
                    assert_eq!(cd.put(k, i as u8), rd.put(k, i as u8));
                }
                for k in keys.iter() {
                    let ca = cd.del(k, off);
                    let ra = rd.del(k, off);
                    assert_eq!(ca, ra, "es={es} ks={ks} off={off}");
                    eqs(&format!("r15 es={es} ks={ks} off={off}"), &cd.snap(), &rd.snap());
                }
                // with keyoffset == 0 the very same deletes DO succeed
                for k in keys.iter() {
                    assert_eq!(cd.del(k, 0), rd.del(k, 0));
                }
                assert_eq!(cd.len(), 0);
                assert_eq!(rd.len(), 0);
                cd.free();
                rd.free();
            }
        }
    }
}

// ===========================================================================
// R16 — stbds_hmput_default's two branches
// ===========================================================================

#[test]
fn r16_hmput_default_branches() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &es in &[1usize, 8, 16, 24] {
            // (a) a == NULL  =>  allocate
            let ct = (c.hmput_default)(std::ptr::null_mut(), es);
            let rt = (r.hmput_default)(std::ptr::null_mut(), es);
            assert!(!ct.is_null() && !rt.is_null());
            eqs(
                &format!("r16a es={es}"),
                &snap_map(ct, es, KeyKind::Bin),
                &snap_map(rt, es, KeyKind::Bin),
            );
            assert_eq!((*header(hash_to_arr(ct, es))).length, 1);
            assert_eq!((*header(hash_to_arr(rt, es))).length, 1);

            // (b) length != 0  =>  returned byte-identical, same pointer
            let ct2 = (c.hmput_default)(ct, es);
            let rt2 = (r.hmput_default)(rt, es);
            assert_eq!(ct2, ct, "C must return the same pointer");
            assert_eq!(rt2, rt, "Rust must return the same pointer");
            eqs(
                &format!("r16b es={es}"),
                &snap_map(ct2, es, KeyKind::Bin),
                &snap_map(rt2, es, KeyKind::Bin),
            );

            // (c) length == 0  =>  allocate again
            (*header(hash_to_arr(ct2, es))).length = 0;
            (*header(hash_to_arr(rt2, es))).length = 0;
            let ct3 = (c.hmput_default)(ct2, es);
            let rt3 = (r.hmput_default)(rt2, es);
            eqs(
                &format!("r16c es={es}"),
                &snap_map(ct3, es, KeyKind::Bin),
                &snap_map(rt3, es, KeyKind::Bin),
            );
            assert_eq!((*header(hash_to_arr(ct3, es))).length, 1);
            assert_eq!((*header(hash_to_arr(rt3, es))).length, 1);
            (c.hmfree_func)(hash_to_arr(ct3, es), es);
            (r.hmfree_func)(hash_to_arr(rt3, es), es);
        }
    }
}

// ===========================================================================
// R17 / R18 — degenerate inputs to the hash functions
// ===========================================================================

#[test]
fn r17_hash_bytes_len_zero_never_dereferences() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &seed in &[0usize, 1, 2, DEFAULT_SEED, usize::MAX, 0x8000_0000_0000_0000] {
            let cv = (c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let rv = (r.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(cv, rv, "hash_bytes(NULL, 0, {seed:#x})");
            // the result depends only on the seed
            let buf = [0xEEu8; 32];
            assert_eq!(cv, (c.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed));
            assert_eq!(rv, (r.hash_bytes)(buf.as_ptr() as *mut c_void, 0, seed));
        }
    }
}

#[test]
fn r18_hash_string_empty() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        let empty = [0u8; 4];
        for &seed in &[0usize, 1, DEFAULT_SEED, usize::MAX, 0x1234_5678_9abc_def0] {
            let cv = (c.hash_string)(empty.as_ptr() as *mut c_char, seed);
            let rv = (r.hash_string)(empty.as_ptr() as *mut c_char, seed);
            assert_eq!(cv, rv, "hash_string(\"\", {seed:#x})");
        }
        // `hash ^= seed` zeroes the accumulator for the empty string, so the
        // result is exactly `K + seed` for a fixed K.
        let k0 = (c.hash_string)(empty.as_ptr() as *mut c_char, 0);
        for &seed in &[1usize, 12345, usize::MAX, DEFAULT_SEED] {
            assert_eq!(
                (c.hash_string)(empty.as_ptr() as *mut c_char, seed),
                k0.wrapping_add(seed)
            );
            assert_eq!(
                (r.hash_string)(empty.as_ptr() as *mut c_char, seed),
                k0.wrapping_add(seed)
            );
        }
    }
}

// ===========================================================================
// R19 — `if (hash < 2) hash += 2` in BOTH stbds_hm_find_slot and
//       stbds_hmput_key.
//
// Reachable because `stbds_hash_string("", seed) == K + seed`, so choosing
// `seed = -K` / `seed = 1-K` produces a raw hash of exactly 0 / 1 — the two
// values that would otherwise be indistinguishable from STBDS_HASH_EMPTY and
// STBDS_HASH_DELETED.
// ===========================================================================

#[test]
fn r19_hash_below_two_is_bumped() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        let empty = [0u8; 4];
        let k0 = (c.hash_string)(empty.as_ptr() as *mut c_char, 0);
        assert_eq!(k0, (r.hash_string)(empty.as_ptr() as *mut c_char, 0));

        for (raw, want) in [(0usize, 2usize), (1, 3)] {
            let seed = raw.wrapping_sub(k0);
            assert_eq!(
                (c.hash_string)(empty.as_ptr() as *mut c_char, seed),
                raw,
                "seed construction failed"
            );
            assert_eq!((r.hash_string)(empty.as_ptr() as *mut c_char, seed), raw);

            for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
                sync_seed(seed);
                let mut cd = Drv::shmode(c, 16, 8, HM_STRING, sh);
                let mut rd = Drv::shmode(r, 16, 8, HM_STRING, sh);
                let key = padded_key(b"");
                let ci = cd.put(&key, 0x5A);
                let ri = rd.put(&key, 0x5A);
                assert_eq!(ci, ri);
                assert_eq!(ci, 0);

                let cti = &*cd.table();
                let rti = &*rd.table();
                assert_eq!(cti.seed, seed);
                assert_eq!(rti.seed, seed);
                // hash `raw` (< 2) must have been stored as `raw + 2`
                let pos = want & (cti.slot_count - 1);
                assert_eq!(
                    (*cti.storage.add(pos >> 3)).hash[pos & 7],
                    want,
                    "C: raw hash {raw} must be stored as {want}"
                );
                assert_eq!(
                    (*rti.storage.add(pos >> 3)).hash[pos & 7],
                    want,
                    "Rust: raw hash {raw} must be stored as {want}"
                );
                assert_eq!((*cti.storage.add(pos >> 3)).index[pos & 7], 0);
                assert_eq!((*rti.storage.add(pos >> 3)).index[pos & 7], 0);
                eqs(&format!("r19 raw={raw} sh={sh}"), &cd.snap(), &rd.snap());

                // ... and the same bump in stbds_hm_find_slot, so the key is
                // still findable and deletable
                assert_eq!(cd.get(&key), 0);
                assert_eq!(rd.get(&key), 0);
                assert_eq!(cd.get_ts(&key), 0);
                assert_eq!(rd.get_ts(&key), 0);
                assert_eq!(cd.del(&key, 0), rd.del(&key, 0));
                assert_eq!(cd.len(), 0);
                assert_eq!(rd.len(), 0);
                eqs(&format!("r19-del raw={raw} sh={sh}"), &cd.snap(), &rd.snap());
                cd.free();
                rd.free();
            }
        }
    }
}

// ===========================================================================
// R20 / R21 — out-of-range `mode` selects the string vs binary hash function.
//
// Proved directly: the hash the library stored in the bucket must equal
// `stbds_hash_string(key, seed)` (string path) or
// `stbds_hash_bytes(key, keysize, seed)` (binary path), bumped by the `< 2` rule.
// ===========================================================================

/// The hash the library actually stored for the entry whose index is `idx`.
unsafe fn stored_hash(d: &Drv, idx: isize) -> Option<usize> {
    unsafe {
        let ti = d.table();
        if ti.is_null() {
            return None;
        }
        for b in 0..((*ti).slot_count >> 3) {
            let bk = (*ti).storage.add(b);
            for j in 0..8 {
                if (*bk).index[j] == idx && (*bk).hash[j] >= 2 {
                    return Some((*bk).hash[j]);
                }
            }
        }
        None
    }
}

fn expect_hash(l: &Lib, key: &[u8], ks: usize, mode: c_int, seed: usize) -> usize {
    unsafe {
        let mut h = if mode >= HM_STRING {
            (l.hash_string)(key.as_ptr() as *mut c_char, seed)
        } else {
            (l.hash_bytes)(key.as_ptr() as *mut c_void, ks, seed)
        };
        if h < 2 {
            h = h.wrapping_add(2);
        }
        h
    }
}

#[test]
fn r20_out_of_range_mode_takes_the_string_path() {
    let _g = lock();
    let (c, r) = both();
    for &mode in &[2i32, 3, 5, 42, 255, 256, 65536, c_int::MAX] {
        for &seed in &[1usize, DEFAULT_SEED, 0x9876] {
            sync_seed(seed);
            let keys = strkeys(5, "m");
            let mut cd = Drv::shmode(c, 16, 8, mode, SH_ARENA);
            let mut rd = Drv::shmode(r, 16, 8, mode, SH_ARENA);
            unsafe {
                for (i, k) in keys.iter().enumerate() {
                    let ci = cd.put(k, i as u8);
                    let ri = rd.put(k, i as u8);
                    assert_eq!(ci, ri);
                    let want_str = expect_hash(c, k, 8, HM_STRING, seed);
                    let want_bin = expect_hash(c, k, 8, HM_BINARY, seed);
                    assert_ne!(want_str, want_bin, "hash functions must differ");
                    assert_eq!(
                        stored_hash(&cd, ci),
                        Some(want_str),
                        "C mode={mode} must use stbds_hash_string"
                    );
                    assert_eq!(
                        stored_hash(&rd, ri),
                        Some(want_str),
                        "Rust mode={mode} must use stbds_hash_string"
                    );
                }
                eqs(&format!("r20 mode={mode} seed={seed:#x}"), &cd.snap(), &rd.snap());
                cd.free();
                rd.free();
            }
        }
    }
    // and starting from NULL, `nt->string.mode` becomes STBDS_SH_DEFAULT
    for &mode in &[2i32, 99, c_int::MAX] {
        sync_seed(DEFAULT_SEED);
        let keys = strkeys(3, "n");
        let mut cd = Drv::empty(c, 16, 8, mode);
        let mut rd = Drv::empty(r, 16, 8, mode);
        unsafe {
            assert_eq!(cd.put(&keys[0], 1), rd.put(&keys[0], 1));
            assert_eq!(cd.string_mode(), SH_DEFAULT as u8, "mode={mode}");
            assert_eq!(rd.string_mode(), SH_DEFAULT as u8, "mode={mode}");
            eqs(&format!("r20-null mode={mode}"), &cd.snap(), &rd.snap());
            cd.free();
            rd.free();
        }
    }
}

#[test]
fn r21_negative_mode_takes_the_binary_path() {
    let _g = lock();
    let (c, r) = both();
    for &mode in &[-1i32, -2, -255, c_int::MIN, c_int::MIN + 1] {
        for &seed in &[1usize, DEFAULT_SEED, 0x9876] {
            sync_seed(seed);
            let keys = bkeys(5, 0x77);
            let mut cd = Drv::shmode(c, 16, 8, mode, SH_NONE);
            let mut rd = Drv::shmode(r, 16, 8, mode, SH_NONE);
            unsafe {
                for (i, k) in keys.iter().enumerate() {
                    let ci = cd.put(k, i as u8);
                    let ri = rd.put(k, i as u8);
                    assert_eq!(ci, ri);
                    let want_bin = expect_hash(c, k, 8, HM_BINARY, seed);
                    assert_eq!(
                        stored_hash(&cd, ci),
                        Some(want_bin),
                        "C mode={mode} must use stbds_hash_bytes"
                    );
                    assert_eq!(
                        stored_hash(&rd, ri),
                        Some(want_bin),
                        "Rust mode={mode} must use stbds_hash_bytes"
                    );
                }
                eqs(&format!("r21 mode={mode} seed={seed:#x}"), &cd.snap(), &rd.snap());
                cd.free();
                rd.free();
            }
        }
        // from NULL: string.mode stays 0 (`mode >= STBDS_HM_STRING` is false)
        sync_seed(DEFAULT_SEED);
        let keys = bkeys(3, 0x99);
        let mut cd = Drv::empty(c, 16, 8, mode);
        let mut rd = Drv::empty(r, 16, 8, mode);
        unsafe {
            assert_eq!(cd.put(&keys[0], 1), rd.put(&keys[0], 1));
            assert_eq!(cd.string_mode(), 0, "mode={mode}");
            assert_eq!(rd.string_mode(), 0, "mode={mode}");
            eqs(&format!("r21-null mode={mode}"), &cd.snap(), &rd.snap());
            cd.free();
            rd.free();
        }
    }
}

// ===========================================================================
// R22 — hmdel_key with `mode >= 2`: `mode == STBDS_HM_STRING` is FALSE, so the
//       STBDS_SH_STRDUP copy is not freed and the relocation re-lookup would use
//       the binary key form.  Only the relocation-free (delete-last) case is
//       defined behaviour in the C original (ERRORS.md A5); it must match.
// ===========================================================================

#[test]
fn r22_hmdel_stringish_mode_delete_last() {
    let _g = lock();
    let (c, r) = both();
    for &mode in &[2i32, 5, 255, c_int::MAX] {
        for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &seed in &[0usize, 3, DEFAULT_SEED] {
                sync_seed(seed);
                let keys = strkeys(6, "d");
                let mut cd = Drv::shmode(c, 16, 8, mode, sh);
                let mut rd = Drv::shmode(r, 16, 8, mode, sh);
                unsafe {
                    for (i, k) in keys.iter().enumerate() {
                        assert_eq!(cd.put(k, i as u8), rd.put(k, i as u8));
                    }
                    for (i, k) in keys.iter().enumerate().rev() {
                        // this really is the last element
                        assert_eq!(cd.get(k), i as isize);
                        assert_eq!(rd.get(k), i as isize);
                        assert_eq!(cd.len(), i as isize + 1);
                        let a = cd.del(k, 0);
                        let b = rd.del(k, 0);
                        assert_eq!(a, b, "mode={mode} sh={sh}: hmdel temp");
                        assert_eq!(a, 1, "a successful hmdel sets temp = 1");
                        eqs(&format!("r22 mode={mode} sh={sh} #{i}"), &cd.snap(), &rd.snap());
                    }
                    assert_eq!(cd.len(), 0);
                    assert_eq!(rd.len(), 0);
                    cd.free();
                    rd.free();
                }
            }
        }
    }
}

// ===========================================================================
// R23 / R24 — stbds_shmode_func's `(unsigned char) mode`, and hmput_key's
//             `switch` falling through to `default: memcpy(...)`
// ===========================================================================

#[test]
fn r23_shmode_func_truncates_mode_to_unsigned_char() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &sh in &[
            0i32,
            1,
            2,
            3,
            4,
            5,
            127,
            128,
            254,
            255,
            256,
            257,
            65535,
            65536,
            -1,
            -2,
            -128,
            -256,
            c_int::MIN,
            c_int::MAX,
        ] {
            sync_seed(0x2323);
            for &es in &[8usize, 16] {
                let ct = (c.shmode_func)(es, sh);
                let rt = (r.shmode_func)(es, sh);
                let cm = (*((*header(hash_to_arr(ct, es))).hash_table as *mut HashIndex)).string.mode;
                let rm = (*((*header(hash_to_arr(rt, es))).hash_table as *mut HashIndex)).string.mode;
                assert_eq!(cm, sh as u8, "C: (unsigned char) {sh}");
                assert_eq!(rm, sh as u8, "Rust: (unsigned char) {sh}");
                assert_eq!(cm, rm);
                // the rest of the fresh table must be identical too
                eqs(
                    &format!("r23 sh={sh} es={es}"),
                    &snap_map(ct, es, KeyKind::Bin),
                    &snap_map(rt, es, KeyKind::Bin),
                );
                (c.hmfree_func)(hash_to_arr(ct, es), es);
                (r.hmfree_func)(hash_to_arr(rt, es), es);
            }
        }
    }
}

#[test]
fn r24_switch_default_memcpys_raw_key_bytes() {
    let _g = lock();
    let (c, r) = both();
    // every string.mode with no matching `case`
    for &sh in &[0i32, 4, 5, 6, 127, 200, 255] {
        for &(es, ks) in &[(8usize, 4usize), (16, 8), (24, 16), (16, 1)] {
            sync_seed(0x2424);
            let keys = bkeys(6, 0x1234);
            let mut cd = Drv::shmode(c, es, ks, HM_BINARY, sh);
            let mut rd = Drv::shmode(r, es, ks, HM_BINARY, sh);
            unsafe {
                assert!(!cd.stores_pointer(), "sh={sh} must hit `default:`");
                assert!(!rd.stores_pointer(), "sh={sh} must hit `default:`");
                for (i, k) in keys.iter().enumerate() {
                    let ci = cd.put(k, i as u8);
                    let ri = rd.put(k, i as u8);
                    assert_eq!(ci, ri);
                    // the element must literally hold `keysize` bytes of the key
                    let ca = hash_to_arr(cd.t, es) as *const u8;
                    let ra = hash_to_arr(rd.t, es) as *const u8;
                    let cbytes = std::slice::from_raw_parts(ca.add(es * (ci as usize + 1)), ks);
                    let rbytes = std::slice::from_raw_parts(ra.add(es * (ri as usize + 1)), ks);
                    assert_eq!(cbytes, &k[..ks], "C sh={sh}: memcpy of the key bytes");
                    assert_eq!(rbytes, &k[..ks], "Rust sh={sh}: memcpy of the key bytes");
                }
                eqs(&format!("r24 sh={sh} es={es} ks={ks}"), &cd.snap(), &rd.snap());
                cd.free();
                rd.free();
            }
        }
    }
}

// ===========================================================================
// R25 / R26 / R27 / R28 — stbds_stralloc's oversized-block and block-ladder
//                         branches
// ===========================================================================

fn arena_state(a: &StringArena) -> String {
    unsafe { snap_arena(a) }
}

#[test]
fn r25_oversized_string_into_a_non_empty_arena() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        let small = {
            let mut v = vec![b'x'; 10];
            v.push(0);
            v
        };
        let big = {
            let mut v = vec![b'y'; 2000]; // > the 512-byte first block
            v.push(0);
            v
        };
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        (c.stralloc)(&mut ca, small.as_ptr() as *mut c_char);
        (r.stralloc)(&mut ra, small.as_ptr() as *mut c_char);
        let cbefore = ca;
        let rbefore = ra;
        assert!(!ca.storage.is_null() && !ra.storage.is_null());

        let cp = (c.stralloc)(&mut ca, big.as_ptr() as *mut c_char);
        let rp = (r.stralloc)(&mut ra, big.as_ptr() as *mut c_char);
        assert_eq!(arena_state(&ca), arena_state(&ra));
        // head block and `remaining` untouched; the block is spliced as ->next
        assert_eq!(ca.storage, cbefore.storage, "C: head block must not change");
        assert_eq!(ra.storage, rbefore.storage, "Rust: head block must not change");
        assert_eq!(ca.remaining, cbefore.remaining, "C: remaining must be preserved");
        assert_eq!(ra.remaining, rbefore.remaining, "Rust: remaining must be preserved");
        let csb = (*(ca.storage as *mut StringBlock)).next;
        let rsb = (*(ra.storage as *mut StringBlock)).next;
        assert!(!csb.is_null() && !rsb.is_null());
        assert_eq!(cp as *mut u8, (csb as *mut u8).add(8));
        assert_eq!(rp as *mut u8, (rsb as *mut u8).add(8));
        assert_eq!(cstr_opt(cp), cstr_opt(rp));
        assert_eq!(cstr_opt(cp), cstr_opt(big.as_ptr() as *const c_char));
        (c.strreset)(&mut ca);
        (r.strreset)(&mut ra);
        assert_eq!(arena_state(&ca), arena_state(&ra));
    }
}

#[test]
fn r26_oversized_string_into_a_fresh_arena() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for n in [512usize, 513, 1000, 5000, 100_000] {
            let big = {
                let mut v = vec![b'z'; n];
                v.push(0);
                v
            };
            let mut ca = StringArena::zeroed();
            let mut ra = StringArena::zeroed();
            let cp = (c.stralloc)(&mut ca, big.as_ptr() as *mut c_char);
            let rp = (r.stralloc)(&mut ra, big.as_ptr() as *mut c_char);
            assert_eq!(arena_state(&ca), arena_state(&ra), "n={n}");
            assert_eq!(ca.remaining, 0, "C: remaining must be 0");
            assert_eq!(ra.remaining, 0, "Rust: remaining must be 0");
            assert_eq!(ca.block, 1);
            assert_eq!(ra.block, 1);
            assert!((*(ca.storage as *mut StringBlock)).next.is_null());
            assert!((*(ra.storage as *mut StringBlock)).next.is_null());
            assert_eq!(cp as *mut u8, (ca.storage as *mut u8).add(8));
            assert_eq!(rp as *mut u8, (ra.storage as *mut u8).add(8));
            assert_eq!(cstr_opt(cp), cstr_opt(rp));
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
        }
    }
}

#[test]
fn r27_block_field_shift_of_64_or_more() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        // `512 << (block>>1)` with `block>>1` in 55..=63 shifts every bit out on
        // x86-64 (count taken mod 64), giving blocksize == 0, so the
        // oversized-block path is taken for ANY string length.
        for block in [110u8, 111, 120, 126, 127, 238, 250, 254, 255] {
            let s = {
                let mut v = vec![b'q'; 3];
                v.push(0);
                v
            };
            let mut ca = StringArena::zeroed();
            let mut ra = StringArena::zeroed();
            ca.block = block;
            ra.block = block;
            let cp = (c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            let rp = (r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
            assert_eq!(arena_state(&ca), arena_state(&ra), "block={block}");
            assert_eq!(ca.remaining, 0, "blocksize 0 => oversized path");
            assert_eq!(ra.remaining, 0);
            assert_eq!(ca.block, block.wrapping_add(1), "block must be incremented");
            assert_eq!(ra.block, block.wrapping_add(1));
            assert_eq!(cstr_opt(cp), cstr_opt(rp));
            assert_eq!(cstr_opt(cp), cstr_opt(s.as_ptr() as *const c_char));
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
            assert_eq!(arena_state(&ca), arena_state(&ra));
        }
    }
}

#[test]
fn r28_block_stops_growing_at_the_1mib_ceiling() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        let s = {
            let mut v = vec![b'w'; 5];
            v.push(0);
            v
        };
        // block 21 -> blocksize 512<<10 == 512 KiB  < 1 MiB  => block increments
        // block 22 -> blocksize 512<<11 == 1 MiB   !< 1 MiB  => block frozen
        for (block, should_increment) in [(20u8, true), (21, true), (22, false), (23, false)] {
            let mut ca = StringArena::zeroed();
            let mut ra = StringArena::zeroed();
            ca.block = block;
            ra.block = block;
            (c.stralloc)(&mut ca, s.as_ptr() as *mut c_char);
            (r.stralloc)(&mut ra, s.as_ptr() as *mut c_char);
            assert_eq!(arena_state(&ca), arena_state(&ra), "block={block}");
            let want = if should_increment { block + 1 } else { block };
            assert_eq!(ca.block, want, "C block={block}");
            assert_eq!(ra.block, want, "Rust block={block}");
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
        }
    }
}

// ===========================================================================
// R29 / R30 — stbds_strreset on an empty arena, twice, and after a splice
// ===========================================================================

#[test]
fn r29_r30_strreset_is_idempotent_and_walks_the_whole_chain() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        // (a) empty arena, reset repeatedly
        let mut ca = StringArena::zeroed();
        let mut ra = StringArena::zeroed();
        ca.block = 9;
        ra.block = 9;
        ca.mode = 3;
        ra.mode = 3;
        ca.remaining = 123;
        ra.remaining = 123;
        for i in 0..4 {
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
            assert_eq!(arena_state(&ca), arena_state(&ra), "round {i}");
            assert!(ca.storage.is_null() && ra.storage.is_null());
            assert_eq!(ca.remaining, 0);
            assert_eq!(ra.remaining, 0);
            assert_eq!(ca.block, 0);
            assert_eq!(ra.block, 0);
            assert_eq!(ca.mode, 0, "strreset must zero all 24 bytes");
            assert_eq!(ra.mode, 0, "strreset must zero all 24 bytes");
        }

        // (b) an arena with a spliced oversized block plus many normal blocks
        for round in 0..3 {
            for i in 0..300usize {
                let n = if i % 50 == 7 { 3000 } else { i % 40 };
                let mut v = vec![b'a' + (i % 26) as u8; n];
                v.push(0);
                (c.stralloc)(&mut ca, v.as_ptr() as *mut c_char);
                (r.stralloc)(&mut ra, v.as_ptr() as *mut c_char);
                assert_eq!(arena_state(&ca), arena_state(&ra), "round {round} #{i}");
            }
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
            assert_eq!(arena_state(&ca), arena_state(&ra));
            assert!(ca.storage.is_null() && ra.storage.is_null());
            // reset again immediately: still a no-op
            (c.strreset)(&mut ca);
            (r.strreset)(&mut ra);
            assert_eq!(arena_state(&ca), arena_state(&ra));
        }
    }
}

// ===========================================================================
// R31 / R32 — strkey's integer formatting corners
// ===========================================================================

#[test]
fn r31_r32_strkey_negative_and_int_min() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        for &n in &[
            0i32,
            -1,
            -9,
            -10,
            -99,
            -100,
            -1000,
            i32::MIN,
            i32::MIN + 1,
            -2147483647,
            i32::MAX,
            i32::MAX - 1,
        ] {
            let cs = cstr_opt((c.strkey)(n));
            let rs = cstr_opt((r.strkey)(n));
            assert_eq!(cs, rs, "strkey({n})");
            assert_eq!(cs, format!("{:?}", format!("test_{n}")), "strkey({n})");
        }
        // INT_MIN is exactly 16 chars + NUL, well inside the 256-byte static buffer
        assert_eq!(format!("test_{}", i32::MIN).len(), 16);
    }
}

// ===========================================================================
// R33 / R34 — sh_puts with num <= 0
// ===========================================================================

#[test]
fn r33_r34_sh_puts_non_positive_num() {
    let _g = lock();
    let (c, r) = both();
    for &num in &[0i32, -1, -2, -100, -123_456, i32::MIN + 1, i32::MIN] {
        sync_seed(DEFAULT_SEED);
        let cout = capture_stdout(|| unsafe { (c.sh_puts)(num) });
        sync_seed(DEFAULT_SEED);
        let rout = capture_stdout(|| unsafe { (r.sh_puts)(num) });
        assert_eq!(cout, rout, "sh_puts({num}): {} vs {}", show(&cout), show(&rout));
        assert_eq!(cout, format!("a {num}\n").into_bytes(), "sh_puts({num})");
    }
}

// ===========================================================================
// R36 — insertion reclaiming a tombstone (`pos = tombstone; --tombstone_count`)
// ===========================================================================

#[test]
fn r36_insertion_reuses_a_tombstone() {
    let _g = lock();
    let (c, r) = both();
    let mut reuses = 0usize;
    unsafe {
        for seed in 0..200usize {
            sync_seed(seed.wrapping_mul(0x9E37_79B9) | 1);
            let keys = bkeys(24, 0x5151);
            let mut cd = Drv::shmode(c, 16, 8, HM_BINARY, SH_NONE);
            let mut rd = Drv::shmode(r, 16, 8, HM_BINARY, SH_NONE);
            let mut rng = Rng::new(0x3636_0000 + seed as u64);
            for _ in 0..120 {
                let k = rng.below(24) as usize;
                let ctc_before = (*cd.table()).tombstone_count;
                let rtc_before = (*rd.table()).tombstone_count;
                assert_eq!(ctc_before, rtc_before);
                if rng.next_u64() & 1 == 0 {
                    let tag = rng.byte(); // one tag, used for BOTH libraries
                    assert_eq!(cd.put(&keys[k], tag), rd.put(&keys[k], tag));
                    let ctc = (*cd.table()).tombstone_count;
                    let rtc = (*rd.table()).tombstone_count;
                    assert_eq!(ctc, rtc, "tombstone_count after put");
                    if ctc < ctc_before {
                        reuses += 1;
                    }
                } else {
                    assert_eq!(cd.del(&keys[k], 0), rd.del(&keys[k], 0));
                }
                eqs(&format!("r36 seed={seed}"), &cd.snap(), &rd.snap());
            }
            cd.free();
            rd.free();
        }
    }
    assert!(reuses > 0, "R36 (tombstone reclamation) never reached");
    eprintln!("coverage: tombstone reclamations = {reuses}");
}

// ===========================================================================
// G3 — zero lengths: keysize == 0 and elemsize == 0
// ===========================================================================

#[test]
fn g3_keysize_zero() {
    let _g = lock();
    let (c, r) = both();
    // `hash_bytes(key, 0, seed)` ignores the key and `memcmp(_,_,0)` is always
    // equal, so every key collides *and* compares equal: the map can only ever
    // hold the first key.
    for &es in &[8usize, 16, 24] {
        sync_seed(0x0303);
        let keys = bkeys(8, 0x4444);
        let mut ops: Vec<Op> = Vec::new();
        for i in 0..8 {
            ops.push(Op::Put(i, i as u8));
            ops.push(Op::Get(i));
            ops.push(Op::GetTs(i));
            ops.push(Op::Len);
        }
        for i in 0..8 {
            ops.push(Op::Del(i, 0));
            ops.push(Op::Len);
            ops.push(Op::Get(i));
        }
        run_ops(
            &format!("g3-keysize0 es={es}"),
            Drv::shmode(c, es, 0, HM_BINARY, SH_NONE),
            Drv::shmode(r, es, 0, HM_BINARY, SH_NONE),
            &keys,
            &ops,
        );
    }
}

#[test]
fn g3_elemsize_zero() {
    let _g = lock();
    let (c, r) = both();
    // elemsize 0 means every "element" aliases the array base; combined with
    // keysize 0 nothing is ever written past the header, so this degenerate but
    // legal configuration is fully defined.
    sync_seed(0x0404);
    let keys = bkeys(6, 0x5555);
    let mut ops: Vec<Op> = Vec::new();
    for i in 0..6 {
        ops.push(Op::Put(i, 0));
        ops.push(Op::Get(i));
        ops.push(Op::Len);
    }
    ops.push(Op::Del(0, 0));
    ops.push(Op::Len);
    ops.push(Op::Get(0));
    run_ops(
        "g3-elemsize0",
        Drv::empty(c, 0, 0, HM_BINARY),
        Drv::empty(r, 0, 0, HM_BINARY),
        &keys,
        &ops,
    );

    // arrgrowf / arrfreef with elemsize 0
    unsafe {
        for &(addlen, min_cap) in &[(0usize, 1usize), (0, 4), (3, 0), (0, 1000)] {
            let cp = (c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            let rp = (r.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            assert_eq!(snap_hdr(cp), snap_hdr(rp), "es=0 {addlen}/{min_cap}");
            (c.arrfreef)(cp);
            (r.arrfreef)(rp);
        }
    }
}

// ===========================================================================
// G5 — one step past each documented threshold
// ===========================================================================

#[test]
fn g5_threshold_boundaries() {
    let _g = lock();
    let (c, r) = both();
    unsafe {
        // used_count_threshold: 6 (sc 8), 12 (16), 24 (32), 48 (64)
        for &n in &[5usize, 6, 7, 11, 12, 13, 23, 24, 25, 47, 48, 49] {
            for &seed in &[0usize, DEFAULT_SEED] {
                sync_seed(seed);
                let keys = bkeys(n + 2, 0x6666);
                let mut cd = Drv::empty(c, 16, 8, HM_BINARY);
                let mut rd = Drv::empty(r, 16, 8, HM_BINARY);
                for i in 0..n {
                    assert_eq!(cd.put(&keys[i], i as u8), rd.put(&keys[i], i as u8));
                }
                let csc = (*cd.table()).slot_count;
                let rsc = (*rd.table()).slot_count;
                assert_eq!(csc, rsc, "n={n}: slot_count");
                // the exact ladder the thresholds imply
                let want = match n {
                    0..=6 => 8,
                    7..=12 => 16,
                    13..=24 => 32,
                    25..=48 => 64,
                    _ => 128,
                };
                assert_eq!(csc, want, "n={n}: slot_count ladder");
                assert_eq!((*cd.table()).used_count, n);
                assert_eq!((*rd.table()).used_count, n);
                assert_eq!(
                    (*cd.table()).used_count_threshold,
                    want - (want >> 2),
                    "n={n}"
                );
                assert_eq!(
                    (*cd.table()).tombstone_count_threshold,
                    (want >> 3) + (want >> 4)
                );
                assert_eq!(
                    (*cd.table()).used_count_shrink_threshold,
                    if want <= 8 { 0 } else { want >> 2 }
                );
                // A1: the live C assertion must hold at every size
                assert!(
                    (*cd.table()).used_count_threshold + (*cd.table()).tombstone_count_threshold
                        < (*cd.table()).slot_count,
                    "A1 violated at slot_count {want}"
                );
                eqs(&format!("g5 n={n} seed={seed:#x}"), &cd.snap(), &rd.snap());
                cd.free();
                rd.free();
            }
        }
    }
}

// ===========================================================================
// Branch-coverage proof for stbds_hmput_key's two probe scans (CONFIGS C38/C39).
//
// The C source handles a duplicate key differently depending on WHICH scan finds
// it: the `for (i = pos&7 .. 7)` scan assigns `stbds_temp_key`, the wrap-around
// `for (i = 0 .. pos&7)` scan does NOT.  Likewise `goto found_empty_slot` can be
// reached from either scan.  This test proves all four combinations are actually
// exercised by the suite (the probe is classified *before* each call, and only
// when no table growth will intervene).
// ===========================================================================

/// Part A — with a table loaded close to its 75 % `used_count_threshold`, all
/// four `stbds_hmput_key` probe outcomes occur.  Binary mode is used so
/// `stbds_temp_key` (uninitialised in a fresh table) is never involved.
#[test]
fn coverage_hmput_key_reaches_all_four_probe_outcomes() {
    let _g = lock();
    let (c, r) = both();
    let mut seen = std::collections::BTreeMap::<(&'static str, &'static str), usize>::new();

    unsafe {
        for seed in 0..150usize {
            sync_seed(seed.wrapping_mul(0x9E37_79B9) | 1);
            let keys = bkeys(48, 0x7070);
            let mut cd = Drv::shmode(c, 16, 8, HM_BINARY, SH_NONE);
            let mut rd = Drv::shmode(r, 16, 8, HM_BINARY, SH_NONE);
            let mut rng = Rng::new(0xC0FF_0000 + seed as u64);
            // fill up towards the load threshold
            for i in 0..40 {
                assert_eq!(cd.put(&keys[i], i as u8), rd.put(&keys[i], i as u8));
            }
            for _ in 0..200 {
                let k = rng.below(48) as usize;
                let ti = cd.table();
                let will_grow = (*ti).used_count >= (*ti).used_count_threshold;
                let pc = probe(c, cd.t, 16, 8, HM_BINARY, 0, &keys[k]);
                let pr = probe(r, rd.t, 16, 8, HM_BINARY, 0, &keys[k]);
                assert_eq!(pc, pr, "seed={seed}: probe shape must match");
                if rng.next_u64() % 4 == 0 {
                    assert_eq!(cd.del(&keys[k], 0), rd.del(&keys[k], 0));
                } else {
                    let tag = rng.byte();
                    assert_eq!(cd.put(&keys[k], tag), rd.put(&keys[k], tag), "seed={seed}");
                    if !will_grow {
                        if let Some(p) = pc {
                            *seen.entry((p.scan, p.kind)).or_insert(0) += 1;
                        }
                    }
                }
                eqs(&format!("covA seed={seed}"), &cd.snap(), &rd.snap());
            }
            cd.free();
            rd.free();
        }
    }

    eprintln!("coverage: hmput_key probe outcomes = {seen:?}");
    for want in [
        ("first", "empty"),
        ("second", "empty"),
        ("first", "match"),
        ("second", "match"),
    ] {
        assert!(
            seen.get(&want).copied().unwrap_or(0) > 0,
            "hmput_key outcome {want:?} was never reached; seen = {seen:?}"
        );
    }
}

/// Part B — the STRING-mode quirk: when a duplicate key is matched in the
/// *wrap-around* scan, `stbds_hmput_key` sets `stbds_temp(a)` but deliberately
/// does NOT set `stbds_temp_key(a)` (unlike the first scan, which sets both).
///
/// The seed is *searched for* so that the wrap-around match really happens in an
/// 8-slot table that never grows (5 keys => `used_count` peaks at 5 < 6), which
/// keeps `temp_key` well-defined throughout.
#[test]
fn coverage_hmput_key_second_scan_match_skips_temp_key() {
    let _g = lock();
    let (c, r) = both();
    let keys = strkeys(5, "w");
    let mut found = 0usize;

    unsafe {
        for seed in 0..40_000usize {
            sync_seed(seed);
            let mut cd = Drv::shmode(c, 16, 8, HM_STRING, SH_ARENA);
            for (i, k) in keys.iter().enumerate() {
                cd.put(k, i as u8);
            }
            // does any duplicate probe wrap into the second scan?
            let hit = keys.iter().position(|k| {
                matches!(
                    probe(c, cd.t, 16, 8, HM_STRING, 0, k),
                    Some(Probe { scan: "second", kind: "match", .. })
                )
            });
            cd.free();
            let Some(hit) = hit else { continue };

            // replay the whole sequence on BOTH libraries
            sync_seed(seed);
            let mut cd = Drv::shmode(c, 16, 8, HM_STRING, SH_ARENA);
            let mut rd = Drv::shmode(r, 16, 8, HM_STRING, SH_ARENA);
            for (i, k) in keys.iter().enumerate() {
                assert_eq!(cd.put(k, i as u8), rd.put(k, i as u8));
            }
            // `temp_key` currently points at the LAST inserted key
            let before_c = cd.temp_key_str();
            let before_r = rd.temp_key_str();
            assert_eq!(before_c, before_r, "seed={seed}");

            let ci = cd.put(&keys[hit], 0xEE);
            let ri = rd.put(&keys[hit], 0xEE);
            assert_eq!(ci, ri, "seed={seed}: wrap-around duplicate temp");
            assert_eq!(ci, hit as isize, "seed={seed}: must reuse the existing index");
            // the wrap-around scan must have left `temp_key` untouched
            assert_eq!(
                cd.temp_key_str(),
                before_c,
                "C: the wrap-around scan must NOT assign temp_key"
            );
            assert_eq!(
                rd.temp_key_str(),
                before_r,
                "Rust: the wrap-around scan must NOT assign temp_key"
            );
            assert_eq!((*cd.table()).slot_count, 8);
            assert_eq!((*rd.table()).slot_count, 8);
            eqs(&format!("covB seed={seed}"), &cd.snap(), &rd.snap());
            cd.free();
            rd.free();

            found += 1;
            if found >= 25 {
                break;
            }
        }
    }
    eprintln!("coverage: wrap-around duplicate puts verified = {found}");
    assert!(
        found > 0,
        "no seed produced a wrap-around duplicate match in an 8-slot table"
    );
}
