//! Phase C — error-path differential tests. One test per row of ERRORS.md.
//!
//! Every test constructs the exact invalid input / rejection condition, calls
//! BOTH libraries through their `.so` exports, and asserts they return the SAME
//! sentinel (`NULL`, `-1`, `0`, `1`, unchanged pointer, …) — not merely "both
//! failed".
//!
//! Two rows (ERRORS.md #4 and #5) are genuine UB in the C (`realloc` failure is
//! unchecked; `stbds_arrfreef(NULL)` computes `free((char*)0 - 32)`); they are
//! documented rather than executed, since running them would abort the harness
//! identically for both libraries and prove nothing.

mod common;

use common::map::*;
use common::*;
use std::ffi::{c_char, c_int, c_void};

fn setup(seed: usize) -> Pair {
    let p = pair();
    p.reseed(seed);
    p
}

// ---------------------------------------------------------------------------
// Row 1 — stbds_arrgrowf(NULL, es, 0, 0) returns NULL
// ---------------------------------------------------------------------------

#[test]
fn err_01_arrgrowf_null_zero_returns_null() {
    let p = setup(1);
    for &elemsize in &[0usize, 1, 4, 8, 16, 1024, usize::MAX] {
        let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        let ra = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0) };
        dq(format!("elemsize={elemsize}"), ca.is_null(), ra.is_null());
        assert!(ca.is_null(), "C must return NULL for elemsize={elemsize}");
        assert!(ra.is_null(), "Rust must return NULL for elemsize={elemsize}");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — early return of the *same* pointer when min_cap <= arrcap
// ---------------------------------------------------------------------------

#[test]
fn err_02_arrgrowf_no_growth_returns_same() {
    let p = setup(2);
    for &elemsize in &[1usize, 4, 8, 16] {
        let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0) };
        let ra = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0) };
        let cap = unsafe { hdr_snapshot(ca) }.capacity;
        assert_eq!(cap, 4, "the `min_cap < 4` bump should give capacity 4");
        assert_eq!(unsafe { hdr_snapshot(ra) }.capacity, 4);
        // length is 0, so any (addlen, min_cap) with max(addlen, min_cap) <= 4
        // must be a no-op returning the identical pointer.
        for addlen in 0..=4usize {
            for min_cap in 0..=4usize {
                let nca = unsafe { (p.c.arrgrowf)(ca, elemsize, addlen, min_cap) };
                let nra = unsafe { (p.rs.arrgrowf)(ra, elemsize, addlen, min_cap) };
                let ctx = format!("elemsize={elemsize} addlen={addlen} min_cap={min_cap}");
                assert_eq!(nca, ca, "C must early-return the same pointer [{ctx}]");
                assert_eq!(nra, ra, "Rust must early-return the same pointer [{ctx}]");
                dq(ctx, unsafe { hdr_snapshot(nca) }, unsafe {
                    hdr_snapshot(nra)
                });
            }
        }
        // one past the range must NOT early-return
        let nca = unsafe { (p.c.arrgrowf)(ca, elemsize, 0, 5) };
        let nra = unsafe { (p.rs.arrgrowf)(ra, elemsize, 0, 5) };
        dq(
            format!("elemsize={elemsize} grow to 5"),
            unsafe { hdr_snapshot(nca) },
            unsafe { hdr_snapshot(nra) },
        );
        assert!(unsafe { hdr_snapshot(nca) }.capacity >= 5);
        unsafe {
            (p.c.arrfreef)(nca);
            (p.rs.arrfreef)(nra);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3 — elemsize == 0
// ---------------------------------------------------------------------------

#[test]
fn err_03_arrgrowf_zero_elemsize() {
    let p = setup(3);
    for &(addlen, min_cap) in &[(1usize, 0usize), (0, 1), (0, 4), (100, 0), (0, 100_000)] {
        let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap) };
        let ra = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap) };
        let ctx = format!("addlen={addlen} min_cap={min_cap}");
        dq(ctx.clone(), ca.is_null(), ra.is_null());
        dq(ctx, unsafe { hdr_snapshot(ca) }, unsafe { hdr_snapshot(ra) });
        unsafe {
            (p.c.arrfreef)(ca);
            (p.rs.arrfreef)(ra);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 4 & 5 — documented UB, deliberately not executed.
// ---------------------------------------------------------------------------

#[test]
fn err_04_05_documented_ub() {
    // Row 4: `stbds_arrgrowf` never checks `realloc`'s return value (lib.c:297),
    //        so an allocation failure dereferences `NULL + sizeof(header)`.
    // Row 5: `stbds_arrfreef(NULL)` calls `free((char *) NULL - 32)` (lib.c:314).
    // Both are undefined behaviour in the C and would take the whole test
    // process down identically for either library; there is nothing
    // differential to observe. Recorded here so the row is not silently
    // skipped.
    //
    // What *is* checkable: the two libraries agree on the boundary just below
    // the failure, i.e. the largest allocation that still succeeds behaves the
    // same. (Verified by err_03 / err_02 above.)
}

// ---------------------------------------------------------------------------
// Row 6 — stbds_hmfree_func(NULL, elemsize)
// ---------------------------------------------------------------------------

#[test]
fn err_06_hmfree_null() {
    let p = setup(6);
    for &elemsize in &[0usize, 1, 4, 16, 1024] {
        // Must return without touching anything. If either library dereferenced
        // the NULL this test would crash.
        unsafe {
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.rs.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7 — stbds_hmfree_func on an array with hash_table == NULL
// ---------------------------------------------------------------------------

#[test]
fn err_07_hmfree_no_table() {
    let p = setup(7);
    for &elemsize in &[1usize, 4, 16, 64] {
        let ch = unsafe { (p.c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let rh = unsafe { (p.rs.hmput_default)(std::ptr::null_mut(), elemsize) };
        let ctx = format!("elemsize={elemsize}");
        dq(
            ctx.clone(),
            unsafe { hash_hdr_snapshot(ch, elemsize) },
            unsafe { hash_hdr_snapshot(rh, elemsize) },
        );
        assert!(
            !unsafe { hash_hdr_snapshot(ch, elemsize) }.has_table,
            "[{ctx}] there must be no hash table yet"
        );
        unsafe {
            (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
            (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
        }
        // Also with a plain arrgrowf array (length 0, no table).
        let ca = unsafe { (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0) };
        let ra = unsafe { (p.rs.arrgrowf)(std::ptr::null_mut(), elemsize, 1, 0) };
        unsafe {
            (p.c.hmfree_func)(ca, elemsize);
            (p.rs.hmfree_func)(ra, elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 8-12 — the -1 / STBDS_INDEX_EMPTY sentinel out of the get path
// ---------------------------------------------------------------------------

#[test]
fn err_09_hmget_ts_null_a() {
    let p = setup(9);
    for &elemsize in &[1usize, 4, 8, 16] {
        for &keysize in &[0usize, 1, 4, 8] {
            if keysize > elemsize {
                continue;
            }
            let mut key = [0xABu8; 8];
            let mut ct: isize = 0x1234;
            let mut rt: isize = 0x1234;
            let ch = unsafe {
                (p.c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    keysize,
                    &mut ct,
                    HM_BINARY,
                )
            };
            let rh = unsafe {
                (p.rs.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    keysize,
                    &mut rt,
                    HM_BINARY,
                )
            };
            let ctx = format!("elemsize={elemsize} keysize={keysize}");
            dq(format!("{ctx} / temp"), ct, rt);
            assert_eq!(ct, -1, "[{ctx}] must be STBDS_INDEX_EMPTY");
            assert!(!ch.is_null() && !rh.is_null(), "[{ctx}] must allocate");
            dq(
                format!("{ctx} / header"),
                unsafe { hash_hdr_snapshot(ch, elemsize) },
                unsafe { hash_hdr_snapshot(rh, elemsize) },
            );
            dq(
                format!("{ctx} / elems"),
                unsafe { hash_elems(ch, elemsize) },
                unsafe { hash_elems(rh, elemsize) },
            );
            assert_eq!(unsafe { hash_hdr_snapshot(ch, elemsize) }.length, 1);
            unsafe {
                (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
                (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
            }
        }
    }
}

#[test]
fn err_10_hmget_ts_no_table() {
    let p = setup(10);
    let elemsize = 16usize;
    let keysize = 4usize;
    // hmput_default builds an array with length 1 and NO hash table.
    let mut ch = unsafe { (p.c.hmput_default)(std::ptr::null_mut(), elemsize) };
    let mut rh = unsafe { (p.rs.hmput_default)(std::ptr::null_mut(), elemsize) };
    for probe in 0..64u32 {
        let mut key = probe.to_le_bytes();
        let mut ct: isize = 0x7777;
        let mut rt: isize = 0x7777;
        ch = unsafe {
            (p.c.hmget_key_ts)(
                ch,
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                &mut ct,
                HM_BINARY,
            )
        };
        rh = unsafe {
            (p.rs.hmget_key_ts)(
                rh,
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                &mut rt,
                HM_BINARY,
            )
        };
        dq(format!("no-table probe={probe}"), ct, rt);
        assert_eq!(ct, -1, "table==NULL must yield -1");
        dq(
            format!("no-table probe={probe} header"),
            unsafe { hash_hdr_snapshot(ch, elemsize) },
            unsafe { hash_hdr_snapshot(rh, elemsize) },
        );
    }
    unsafe {
        (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
        (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
    }
}

#[test]
fn err_11_hmget_ts_missing_key() {
    let p = setup(11);
    let elemsize = 16usize;
    let keysize = 4usize;
    let mut cm = Map::new(&p.c, elemsize, keysize);
    let mut rm = Map::new(&p.rs, elemsize, keysize);
    // Insert only odd keys.
    for i in 0..200u32 {
        let mut ck = (i * 2 + 1).to_le_bytes().to_vec();
        let mut rk = ck.clone();
        let v = vec![i as u8; elemsize - keysize];
        cm.put_binary(&mut ck, &v, HM_BINARY);
        rm.put_binary(&mut rk, &v, HM_BINARY);
    }
    // Every even key must miss with exactly -1 in both.
    for i in 0..400u32 {
        let mut ck = (i * 2).to_le_bytes();
        let mut rk = (i * 2).to_le_bytes();
        let ct = cm.geti_ts(ck.as_mut_ptr() as *mut c_void, HM_BINARY);
        let rt = rm.geti_ts(rk.as_mut_ptr() as *mut c_void, HM_BINARY);
        dq(format!("miss key={}", i * 2), ct, rt);
        assert_eq!(ct, -1, "expected STBDS_INDEX_EMPTY");
    }
    assert_same_binary("after misses", &cm, &rm);
    cm.free();
    rm.free();
}

#[test]
fn err_12_hmget_key_missing_writes_temp() {
    let p = setup(12);
    let elemsize = 8usize;
    let keysize = 4usize;
    let mut cm = Map::new(&p.c, elemsize, keysize);
    let mut rm = Map::new(&p.rs, elemsize, keysize);
    for i in 0..64u32 {
        let mut ck = (i | 0x8000_0000).to_le_bytes().to_vec();
        let mut rk = ck.clone();
        let v = vec![i as u8; elemsize - keysize];
        cm.put_binary(&mut ck, &v, HM_BINARY);
        rm.put_binary(&mut rk, &v, HM_BINARY);
    }
    for i in 0..200u32 {
        let mut ck = i.to_le_bytes();
        let mut rk = i.to_le_bytes();
        let ci = cm.geti(ck.as_mut_ptr() as *mut c_void, HM_BINARY);
        let ri = rm.geti(rk.as_mut_ptr() as *mut c_void, HM_BINARY);
        dq(format!("miss {i}"), ci, ri);
        assert_eq!(ci, -1);
        // hmget_key stores the result into header->temp
        dq(format!("miss {i} header"), cm.hdr(), rm.hdr());
        assert_eq!(cm.hdr().unwrap().temp, -1);
    }
    // and the a == NULL path
    let mut cm2 = Map::new(&p.c, elemsize, keysize);
    let mut rm2 = Map::new(&p.rs, elemsize, keysize);
    let mut k = 5u32.to_le_bytes();
    dq(
        "hmget_key on NULL",
        cm2.geti(k.as_mut_ptr() as *mut c_void, HM_BINARY),
        rm2.geti(k.as_mut_ptr() as *mut c_void, HM_BINARY),
    );
    assert_eq!(cm2.temp(), -1);
    cm.free();
    rm.free();
    cm2.free();
    rm2.free();
}

// ---------------------------------------------------------------------------
// Rows 13-15 — stbds_hmput_default
// ---------------------------------------------------------------------------

#[test]
fn err_13_14_15_hmput_default() {
    let p = setup(13);
    for &elemsize in &[1usize, 4, 8, 16, 100] {
        // 13: a == NULL
        let ch = unsafe { (p.c.hmput_default)(std::ptr::null_mut(), elemsize) };
        let rh = unsafe { (p.rs.hmput_default)(std::ptr::null_mut(), elemsize) };
        let ctx = format!("elemsize={elemsize}");
        assert!(!ch.is_null() && !rh.is_null());
        dq(
            format!("{ctx} / 13 header"),
            unsafe { hash_hdr_snapshot(ch, elemsize) },
            unsafe { hash_hdr_snapshot(rh, elemsize) },
        );
        dq(
            format!("{ctx} / 13 elems"),
            unsafe { hash_elems(ch, elemsize) },
            unsafe { hash_elems(rh, elemsize) },
        );
        assert_eq!(unsafe { hash_hdr_snapshot(ch, elemsize) }.length, 1);
        assert!(
            unsafe { hash_elems(ch, elemsize) }.iter().all(|&b| b == 0),
            "the default slot must be zeroed"
        );

        // 15: no-op when length != 0
        let ch2 = unsafe { (p.c.hmput_default)(ch, elemsize) };
        let rh2 = unsafe { (p.rs.hmput_default)(rh, elemsize) };
        assert_eq!(ch2, ch, "[{ctx}] C must return the identical pointer");
        assert_eq!(rh2, rh, "[{ctx}] Rust must return the identical pointer");

        // 14: length == 0 -> re-grow and re-zero
        unsafe {
            for h in [ch, rh] {
                let a = hash_to_arr(h, elemsize);
                let hp = (a as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader;
                (*hp).length = 0;
                // dirty the default slot so we can see the re-zeroing
                std::ptr::write_bytes(a as *mut u8, 0xCD, elemsize);
            }
        }
        let ch3 = unsafe { (p.c.hmput_default)(ch, elemsize) };
        let rh3 = unsafe { (p.rs.hmput_default)(rh, elemsize) };
        dq(
            format!("{ctx} / 14 header"),
            unsafe { hash_hdr_snapshot(ch3, elemsize) },
            unsafe { hash_hdr_snapshot(rh3, elemsize) },
        );
        dq(
            format!("{ctx} / 14 elems"),
            unsafe { hash_elems(ch3, elemsize) },
            unsafe { hash_elems(rh3, elemsize) },
        );
        assert_eq!(unsafe { hash_hdr_snapshot(ch3, elemsize) }.length, 1);
        assert!(
            unsafe { hash_elems(ch3, elemsize) }.iter().all(|&b| b == 0),
            "[{ctx}] the default slot must be re-zeroed"
        );
        unsafe {
            (p.c.hmfree_func)(hash_to_arr(ch3, elemsize), elemsize);
            (p.rs.hmfree_func)(hash_to_arr(rh3, elemsize), elemsize);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 16-18 — stbds_hmput_key bootstrap + the two "unreachable" asserts
// ---------------------------------------------------------------------------

#[test]
fn err_16_hmput_key_null_a() {
    let p = setup(16);
    for &(elemsize, keysize) in &[(1usize, 1usize), (4, 4), (16, 4), (8, 8), (20, 8), (5, 0)] {
        let mut key = vec![0x5Au8; keysize.max(1)];
        let ch = unsafe {
            (p.c.hmput_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                HM_BINARY,
            )
        };
        let rh = unsafe {
            (p.rs.hmput_key)(
                std::ptr::null_mut(),
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                HM_BINARY,
            )
        };
        let ctx = format!("elemsize={elemsize} keysize={keysize}");
        assert!(!ch.is_null(), "[{ctx}] C must never return NULL");
        assert!(!rh.is_null(), "[{ctx}] Rust must never return NULL");
        dq(
            format!("{ctx} / header"),
            unsafe { hash_hdr_snapshot(ch, elemsize) },
            unsafe { hash_hdr_snapshot(rh, elemsize) },
        );
        // Only the DEFAULT slot (element 0, memset to 0) and the `keysize`
        // key bytes of element 1 are defined; `stbds_hmput_key` never
        // initialises the value part, so those bytes are uninitialised malloc
        // memory in BOTH libraries and must not be compared.
        let cel = unsafe { hash_elems(ch, elemsize) };
        let rel = unsafe { hash_elems(rh, elemsize) };
        dq(
            format!("{ctx} / default slot"),
            cel[..elemsize].to_vec(),
            rel[..elemsize].to_vec(),
        );
        assert!(
            cel[..elemsize].iter().all(|&b| b == 0),
            "[{ctx}] the default slot must be zeroed"
        );
        dq(
            format!("{ctx} / key bytes"),
            cel[elemsize..elemsize + keysize].to_vec(),
            rel[elemsize..elemsize + keysize].to_vec(),
        );
        assert_eq!(
            &cel[elemsize..elemsize + keysize],
            &key[..keysize],
            "[{ctx}] the key must be memcpy'd verbatim"
        );
        dq(
            format!("{ctx} / table"),
            unsafe { table_snapshot(ch, elemsize) },
            unsafe { table_snapshot(rh, elemsize) },
        );
        unsafe {
            (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
            (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
        }
    }
}

#[test]
fn err_17_hmput_key_assert_unreachable() {
    // `STBDS_ASSERT((size_t) i+1 <= stbds_arrcap(a))` (lib.c:778). Asserts are
    // LIVE in the C .so (it links __assert_fail), so if the invariant were
    // violated the C would abort. Drive thousands of inserts across every grow
    // boundary and require both libraries to return normally with identical
    // length/capacity.
    let p = setup(17);
    let mut rng = Rng::new(17);
    for &(elemsize, keysize) in &[(4usize, 4usize), (16, 4), (8, 8)] {
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        for i in 0..3000 {
            let mut ck = rng.bytes(keysize);
            let mut rk = ck.clone();
            let v = vec![i as u8; elemsize - keysize];
            let ci = cm.put_binary(&mut ck, &v, HM_BINARY);
            let ri = rm.put_binary(&mut rk, &v, HM_BINARY);
            dq(format!("es={elemsize} i={i}"), ci, ri);
            let ch = cm.hdr().unwrap();
            dq(format!("es={elemsize} i={i} hdr"), ch.clone(), rm.hdr().unwrap());
            assert!(
                ch.length <= ch.capacity,
                "length {} > capacity {}",
                ch.length,
                ch.capacity
            );
        }
        assert_same_binary("err_17 final", &cm, &rm);
        cm.free();
        rm.free();
    }
}

#[test]
fn err_18_make_hash_index_assert_unreachable() {
    // `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold <
    // slot_count)` (lib.c:401). Verify the invariant holds for every slot_count
    // the API can reach, and that the two libraries compute the *same* three
    // thresholds.
    let p = setup(18);
    let mut cm = Map::new(&p.c, 16, 4);
    let mut rm = Map::new(&p.rs, 16, 4);
    let mut seen_slot_counts = std::collections::BTreeSet::new();
    for i in 0..6000u32 {
        let mut ck = i.to_le_bytes().to_vec();
        let mut rk = ck.clone();
        let v = vec![i as u8; 12];
        cm.put_binary(&mut ck, &v, HM_BINARY);
        rm.put_binary(&mut rk, &v, HM_BINARY);
        let ct = cm.table().unwrap();
        dq(format!("i={i} table"), Some(ct.clone()), rm.table());
        assert!(
            ct.used_count_threshold + ct.tombstone_count_threshold < ct.slot_count,
            "assert would fire: {} + {} >= {}",
            ct.used_count_threshold,
            ct.tombstone_count_threshold,
            ct.slot_count
        );
        seen_slot_counts.insert(ct.slot_count);
    }
    assert!(
        seen_slot_counts.len() >= 8,
        "expected many table sizes, saw {seen_slot_counts:?}"
    );
    cm.free();
    rm.free();
}

// ---------------------------------------------------------------------------
// Row 19 — stbds_shmode_func with an out-of-enum-range mode
// ---------------------------------------------------------------------------

#[test]
fn err_19_shmode_out_of_range() {
    let p = setup(19);
    // (unsigned char) truncation: 7->7, -1->255, 256->0, 257->1, INT_MAX->255,
    // INT_MIN->0. Only the values that truncate into {1,2,3} pick a string
    // branch; the rest fall through to `default:` (memcpy).
    for &mode in &[
        4i32, 5, 7, 100, 255, 256, 257, 512, -1, -2, -256, i32::MAX, i32::MIN,
    ] {
        let elemsize = 16usize;
        let ch = unsafe { (p.c.shmode_func)(elemsize, mode) };
        let rh = unsafe { (p.rs.shmode_func)(elemsize, mode) };
        let ctx = format!("mode={mode}");
        dq(
            format!("{ctx} / header"),
            unsafe { hash_hdr_snapshot(ch, elemsize) },
            unsafe { hash_hdr_snapshot(rh, elemsize) },
        );
        let ct = unsafe { table_snapshot(ch, elemsize) }.unwrap();
        dq(
            format!("{ctx} / table"),
            Some(ct.clone()),
            unsafe { table_snapshot(rh, elemsize) },
        );
        assert_eq!(
            ct.arena_mode,
            (mode as u32 & 0xff) as u8,
            "[{ctx}] string.mode must be (unsigned char) mode"
        );
        unsafe {
            (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
            (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
        }
    }

    // Now actually *use* such a table with binary puts: the `switch` must take
    // `default:` (memcpy) for every mode whose truncation is not 1/2/3.
    for &mode in &[4i32, 7, 100, 255, 256, -1, i32::MAX, i32::MIN] {
        let trunc = (mode as u32 & 0xff) as u8;
        if (1..=3).contains(&trunc) {
            continue; // those are the valid string modes, covered in Phase B
        }
        let elemsize = 16usize;
        let keysize = 4usize;
        let mut cm = Map::with_shmode(&p.c, elemsize, keysize, mode);
        let mut rm = Map::with_shmode(&p.rs, elemsize, keysize, mode);
        for i in 0..200u32 {
            let mut ck = i.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            let v = vec![i as u8; elemsize - keysize];
            dq(
                format!("mode={mode} put#{i}"),
                cm.put_binary(&mut ck, &v, HM_BINARY),
                rm.put_binary(&mut rk, &v, HM_BINARY),
            );
            assert_same_binary(&format!("mode={mode} put#{i}"), &cm, &rm);
        }
        // The keys must have been memcpy'd verbatim (default branch).
        for i in 0..200u32 {
            let mut ck = i.to_le_bytes();
            let mut rk = i.to_le_bytes();
            dq(
                format!("mode={mode} get#{i}"),
                cm.geti(ck.as_mut_ptr() as *mut c_void, HM_BINARY),
                rm.geti(rk.as_mut_ptr() as *mut c_void, HM_BINARY),
            );
        }
        cm.free();
        rm.free();
    }
}

// ---------------------------------------------------------------------------
// Rows 20-22 — stbds_hmdel_key rejections
// ---------------------------------------------------------------------------

#[test]
fn err_20_hmdel_null_a() {
    let p = setup(20);
    for &(elemsize, keysize) in &[(1usize, 1usize), (4, 4), (16, 4), (8, 8)] {
        for &keyoffset in &[0usize, 4, 1000] {
            for &mode in &[HM_BINARY, HM_STRING, 2, -1, i32::MAX, i32::MIN] {
                let mut key = vec![0u8; keysize.max(1)];
                let ca = unsafe {
                    (p.c.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.as_mut_ptr() as *mut c_void,
                        keysize,
                        keyoffset,
                        mode,
                    )
                };
                let ra = unsafe {
                    (p.rs.hmdel_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.as_mut_ptr() as *mut c_void,
                        keysize,
                        keyoffset,
                        mode,
                    )
                };
                let ctx = format!("es={elemsize} ks={keysize} ko={keyoffset} mode={mode}");
                dq(ctx.clone(), ca.is_null(), ra.is_null());
                assert!(ca.is_null(), "[{ctx}] C must return 0");
                assert!(ra.is_null(), "[{ctx}] Rust must return 0");
            }
        }
    }
}

#[test]
fn err_21_hmdel_no_table() {
    let p = setup(21);
    let elemsize = 16usize;
    let keysize = 4usize;
    let mut ch = unsafe { (p.c.hmput_default)(std::ptr::null_mut(), elemsize) };
    let mut rh = unsafe { (p.rs.hmput_default)(std::ptr::null_mut(), elemsize) };
    // Poison temp so we can see hmdel_key set it to 0.
    unsafe {
        for h in [ch, rh] {
            let a = hash_to_arr(h, elemsize);
            ((a as *mut u8).sub(HDR_SIZE) as *mut ArrayHeader)
                .as_mut()
                .unwrap()
                .temp = 0x4242;
        }
    }
    for probe in 0..32u32 {
        let mut key = probe.to_le_bytes();
        let nch = unsafe {
            (p.c.hmdel_key)(
                ch,
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                0,
                HM_BINARY,
            )
        };
        let nrh = unsafe {
            (p.rs.hmdel_key)(
                rh,
                elemsize,
                key.as_mut_ptr() as *mut c_void,
                keysize,
                0,
                HM_BINARY,
            )
        };
        let ctx = format!("no-table del probe={probe}");
        assert_eq!(nch, ch, "[{ctx}] C must return `a` unchanged");
        assert_eq!(nrh, rh, "[{ctx}] Rust must return `a` unchanged");
        let chs = unsafe { hash_hdr_snapshot(nch, elemsize) };
        dq(ctx.clone(), chs.clone(), unsafe {
            hash_hdr_snapshot(nrh, elemsize)
        });
        assert_eq!(chs.temp, 0, "[{ctx}] temp must be reset to 0");
        assert_eq!(chs.length, 1, "[{ctx}] length must be untouched");
        ch = nch;
        rh = nrh;
    }
    unsafe {
        (p.c.hmfree_func)(hash_to_arr(ch, elemsize), elemsize);
        (p.rs.hmfree_func)(hash_to_arr(rh, elemsize), elemsize);
    }
}

#[test]
fn err_22_hmdel_missing_key() {
    let p = setup(22);
    let elemsize = 16usize;
    let keysize = 4usize;
    let mut cm = Map::new(&p.c, elemsize, keysize);
    let mut rm = Map::new(&p.rs, elemsize, keysize);
    for i in 0..150u32 {
        let mut ck = (i * 2 + 1).to_le_bytes().to_vec();
        let mut rk = ck.clone();
        let v = vec![i as u8; elemsize - keysize];
        cm.put_binary(&mut ck, &v, HM_BINARY);
        rm.put_binary(&mut rk, &v, HM_BINARY);
    }
    let len_before = cm.hdr().unwrap().length;
    for i in 0..300u32 {
        let mut ck = (i * 2).to_le_bytes();
        let mut rk = (i * 2).to_le_bytes();
        let ctx = format!("missing del key={}", i * 2);
        dq(
            format!("{ctx} / result"),
            cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
            rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
        );
        assert_eq!(cm.temp(), 0, "[{ctx}] the 'nothing deleted' sentinel is 0");
        assert_eq!(
            cm.hdr().unwrap().length,
            len_before,
            "[{ctx}] length must not change"
        );
        assert_same_binary(&ctx, &cm, &rm);
    }
    // deleting an already-deleted key is also a miss
    let mut k = 1u32.to_le_bytes();
    let mut k2 = 1u32.to_le_bytes();
    dq(
        "first del",
        cm.del(k.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
        rm.del(k2.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
    );
    assert_eq!(cm.temp(), 1);
    let mut k = 1u32.to_le_bytes();
    let mut k2 = 1u32.to_le_bytes();
    dq(
        "second del",
        cm.del(k.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
        rm.del(k2.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
    );
    assert_eq!(cm.temp(), 0, "re-deleting must report 0");
    assert_same_binary("double delete", &cm, &rm);
    cm.free();
    rm.free();
}

// ---------------------------------------------------------------------------
// Rows 23, 25, 26 — the three "unreachable" asserts inside stbds_hmdel_key
// ---------------------------------------------------------------------------

#[test]
fn err_23_25_26_hmdel_asserts() {
    // slot < slot_count (lib.c:828), slot >= 0 after the re-find (846), and
    // b->index[i] == final_index (849). All three are live asserts in the C, so
    // a single violated invariant aborts the process. Hammer the delete path
    // with a shuffled workload and additionally check the post-conditions the
    // asserts encode.
    let p = setup(23);
    let mut rng = Rng::new(23);
    for &n in &[2usize, 9, 33, 200, 900] {
        let mut keys: Vec<u32> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while keys.len() < n {
            let k = rng.next_u32();
            if seen.insert(k) {
                keys.push(k);
            }
        }
        let elemsize = 16usize;
        let keysize = 4usize;
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        for (i, &k) in keys.iter().enumerate() {
            let mut ck = k.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            let v = vec![i as u8; elemsize - keysize];
            cm.put_binary(&mut ck, &v, HM_BINARY);
            rm.put_binary(&mut rk, &v, HM_BINARY);
        }
        let mut order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
        for (step, &oi) in order.iter().enumerate() {
            let mut ck = keys[oi].to_le_bytes();
            let mut rk = keys[oi].to_le_bytes();
            let ctx = format!("n={n} step={step}");
            dq(
                format!("{ctx} / result"),
                cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
            );
            assert_same_binary(&ctx, &cm, &rm);
            // Assert 828's invariant: every in-use bucket index is a valid
            // element index, and every stored slot is < slot_count.
            let t = cm.table().unwrap();
            let len = cm.hdr().unwrap().length as isize;
            let mut in_use = 0usize;
            for b in &t.buckets {
                for j in 0..BUCKET_LENGTH {
                    if b.index[j] >= 0 {
                        in_use += 1;
                        assert!(
                            b.index[j] < len - 1,
                            "[{ctx}] bucket index {} out of range (len {len})",
                            b.index[j]
                        );
                    }
                }
            }
            assert_eq!(
                in_use,
                (len - 1) as usize,
                "[{ctx}] every live element must have exactly one slot"
            );
        }
        cm.free();
        rm.free();
    }
}

// ---------------------------------------------------------------------------
// Row 24 — `--table->used_count` wraps when used_count == 0
// ---------------------------------------------------------------------------

#[test]
fn err_24_hmdel_used_count_wrap() {
    // `used_count` is a `size_t`, so `STBDS_ASSERT(table->used_count >= 0)`
    // (lib.c:832) can never fire; the decrement at line 829 simply WRAPS.
    // Reachable across the FFI boundary by poking the table (a caller can hand
    // the library any memory it likes), so both libs must wrap identically.
    let p = setup(24);
    let elemsize = 16usize;
    let keysize = 4usize;
    // slot_count stays 8 (shrink disabled) so the wrapped value cannot trigger
    // the shrink branch.
    let mut cm = Map::new(&p.c, elemsize, keysize);
    let mut rm = Map::new(&p.rs, elemsize, keysize);
    for i in 0..3u32 {
        let mut ck = i.to_le_bytes().to_vec();
        let mut rk = ck.clone();
        let v = vec![i as u8; elemsize - keysize];
        cm.put_binary(&mut ck, &v, HM_BINARY);
        rm.put_binary(&mut rk, &v, HM_BINARY);
    }
    assert_eq!(cm.table().unwrap().slot_count, 8);
    // Force used_count to 0 in both.
    unsafe {
        for m in [&cm, &rm] {
            let hdr = header(hash_to_arr(m.h, m.elemsize));
            let t = hdr.hash_table as *mut HashIndex;
            (*t).used_count = 0;
        }
    }
    let mut ck = 1u32.to_le_bytes();
    let mut rk = 1u32.to_le_bytes();
    dq(
        "wrap delete result",
        cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
        rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
    );
    let ct = cm.table().unwrap();
    dq("wrap delete table", Some(ct.clone()), rm.table());
    assert_eq!(
        ct.used_count,
        usize::MAX,
        "the size_t decrement must wrap to SIZE_MAX"
    );
    dq("wrap delete header", cm.hdr(), rm.hdr());
    cm.free();
    rm.free();
}

// ---------------------------------------------------------------------------
// Row 27 — deleting the last element skips the memmove + re-find
// ---------------------------------------------------------------------------

#[test]
fn err_27_hmdel_last_element() {
    let p = setup(27);
    let elemsize = 16usize;
    let keysize = 4usize;
    for &n in &[1usize, 2, 3, 9, 40] {
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        for i in 0..n as u32 {
            let mut ck = i.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            let v = vec![i as u8; elemsize - keysize];
            cm.put_binary(&mut ck, &v, HM_BINARY);
            rm.put_binary(&mut rk, &v, HM_BINARY);
        }
        // Insertion order == element order, so key n-1 is the last element.
        for i in (0..n as u32).rev() {
            let mut ck = i.to_le_bytes();
            let mut rk = i.to_le_bytes();
            let ctx = format!("n={n} last-del key={i}");
            dq(
                format!("{ctx} / result"),
                cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
            );
            assert_eq!(cm.temp(), 1, "[{ctx}]");
            assert_same_binary(&ctx, &cm, &rm);
        }
        assert_eq!(cm.len(), 0);
        cm.free();
        rm.free();
    }
}

// ---------------------------------------------------------------------------
// Row 28 — mode == 2 on a string table
// ---------------------------------------------------------------------------

#[test]
fn err_28_hmdel_mode_two_string_table() {
    // `stbds_hmdel_key` tests `mode == STBDS_HM_STRING` (exactly 1) at lines
    // 836 and 842, while `stbds_hm_find_slot` tests `mode >= 1`. With mode == 2
    // on a string table the key IS hashed as a string (so the slot is found),
    // but the strdup'd key is NOT freed and the re-find at line 843 would be
    // handed the raw bytes of the stored `char*`.
    //
    // Restricted to `old_index == final_index` (deleting the last element),
    // where the whole memmove + re-find block is skipped, because otherwise the
    // C's live `STBDS_ASSERT(slot >= 0)` at line 846 aborts (documented).
    let p = setup(28);
    for sh in [SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &mode in &[2i32, 3, 99, i32::MAX] {
            let elemsize = 16usize;
            let keysize = 8usize;
            let mut cm = Map::with_shmode(&p.c, elemsize, keysize, sh);
            let mut rm = Map::with_shmode(&p.rs, elemsize, keysize, sh);
            let keys: Vec<Vec<u8>> = (0..6).map(|i| format!("k{i}").into_bytes()).collect();
            for (i, k) in keys.iter().enumerate() {
                let v = vec![i as u8; elemsize - keysize];
                cm.put_string(leak_cstring(k), &v, HM_STRING);
                rm.put_string(leak_cstring(k), &v, HM_STRING);
            }
            assert_same_string(&format!("sh={sh} mode={mode} built"), &cm, &rm);
            // Delete from the back: always the last element.
            for i in (0..keys.len()).rev() {
                let ctx = format!("sh={sh} mode={mode} del#{i}");
                dq(
                    format!("{ctx} / result"),
                    cm.del(leak_cstring(&keys[i]) as *mut c_void, 0, mode),
                    rm.del(leak_cstring(&keys[i]) as *mut c_void, 0, mode),
                );
                assert_eq!(cm.temp(), 1, "[{ctx}] mode>=1 still finds the slot");
                assert_same_string(&ctx, &cm, &rm);
            }
            assert_eq!(cm.len(), 0);
            // NOTE: with SH_STRDUP + mode != 1 the keys are intentionally
            // leaked by the C (line 836 is false). hmfree_func only sweeps
            // elements 1..length, which is now empty, so both libs leak the
            // same amount — nothing to free here beyond the table itself.
            cm.free();
            rm.free();
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 29-30 — shrink and rebuild paths
// ---------------------------------------------------------------------------

#[test]
fn err_29_30_hmdel_shrink_and_rebuild() {
    let p = setup(29);
    let mut rng = Rng::new(29);
    let elemsize = 16usize;
    let keysize = 4usize;
    // Shrink: build big, then delete everything.
    {
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        let keys: Vec<u32> = (0..600u32).collect();
        for &k in &keys {
            let mut ck = k.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            let v = vec![k as u8; elemsize - keysize];
            cm.put_binary(&mut ck, &v, HM_BINARY);
            rm.put_binary(&mut rk, &v, HM_BINARY);
        }
        let start = cm.table().unwrap().slot_count;
        let mut shrinks = 0;
        let mut prev = start;
        for &k in &keys {
            let mut ck = k.to_le_bytes();
            let mut rk = k.to_le_bytes();
            let ctx = format!("shrink del key={k}");
            dq(
                format!("{ctx} / result"),
                cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
            );
            assert_same_binary(&ctx, &cm, &rm);
            let now = cm.table().unwrap().slot_count;
            if now < prev {
                shrinks += 1;
            }
            prev = now;
        }
        assert!(shrinks >= 3, "expected several shrinks, saw {shrinks}");
        assert_eq!(prev, 8, "should have shrunk back to the minimum");
        cm.free();
        rm.free();
    }
    // Rebuild: batched delete/reinsert so used_count stays high.
    {
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        // Sizing matters: the rebuild branch (line 858) is only reachable when
        // `tombstone_count_threshold` (slot_count/8 + slot_count/16) is crossed
        // BEFORE `used_count` falls below `used_count_shrink_threshold`
        // (slot_count/4), since the shrink check runs first. With 60 keys the
        // table is 128 slots: shrink at used < 32, rebuild at tombstones > 24,
        // and 60 - 25 = 35 >= 32, so a 26-delete batch hits rebuild first.
        let keys: Vec<u32> = (0..60).map(|_| rng.next_u32()).collect();
        for (i, &k) in keys.iter().enumerate() {
            let mut ck = k.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            let v = vec![i as u8; elemsize - keysize];
            cm.put_binary(&mut ck, &v, HM_BINARY);
            rm.put_binary(&mut rk, &v, HM_BINARY);
        }
        let mut rebuilds = 0;
        for round in 0..5 {
            let mut prev = cm.table().unwrap();
            for step in 0..26 {
                let k = keys[(round * 7 + step) % keys.len()];
                let mut ck = k.to_le_bytes();
                let mut rk = k.to_le_bytes();
                let ctx = format!("rebuild round={round} del step={step}");
                dq(
                    format!("{ctx} / result"),
                    cm.del(ck.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                    rm.del(rk.as_mut_ptr() as *mut c_void, 0, HM_BINARY),
                );
                assert_same_binary(&ctx, &cm, &rm);
                let now = cm.table().unwrap();
                if now.slot_count == prev.slot_count
                    && now.tombstone_count == 0
                    && prev.tombstone_count >= prev.tombstone_count_threshold
                {
                    rebuilds += 1;
                }
                prev = now;
            }
            for step in 0..26 {
                let k = keys[(round * 7 + step) % keys.len()];
                let mut ck = k.to_le_bytes().to_vec();
                let mut rk = ck.clone();
                let v = vec![step as u8; elemsize - keysize];
                let ctx = format!("rebuild round={round} put step={step}");
                dq(
                    format!("{ctx} / index"),
                    cm.put_binary(&mut ck, &v, HM_BINARY),
                    rm.put_binary(&mut rk, &v, HM_BINARY),
                );
                assert_same_binary(&ctx, &cm, &rm);
            }
        }
        assert!(rebuilds >= 1, "never hit the rebuild path");
        cm.free();
        rm.free();
    }
}

// ---------------------------------------------------------------------------
// Rows 31-35 — string arena
// ---------------------------------------------------------------------------

fn fresh_arena() -> StringArena {
    StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 }
}

#[test]
fn err_31_stralloc_assert() {
    // `STBDS_ASSERT(len <= a->remaining)` (lib.c:913). Drive the arena with a
    // long adversarial sequence and require both libraries to never violate it
    // (a violation would abort the C).
    let p = setup(31);
    let mut rng = Rng::new(31);
    let mut ca = fresh_arena();
    let mut ra = fresh_arena();
    for i in 0..3000 {
        // mix of tiny, boundary and oversize requests
        let len = match rng.below(6) {
            0 => 0,
            1 => 1,
            2 => 511,
            3 => 512,
            4 => rng.below(1000),
            _ => rng.below(60),
        };
        let mut s: Vec<u8> = vec![b'x'; len];
        s.push(0);
        let sp = s.as_mut_ptr() as *mut c_char;
        let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
        let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
        let ctx = format!("i={i} len={len}");
        dq(format!("{ctx} / content"), unsafe { cstr(cp) }, unsafe {
            cstr(rp)
        });
        dq(format!("{ctx} / remaining"), ca.remaining, ra.remaining);
        dq(format!("{ctx} / block"), ca.block, ra.block);
    }
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.rs.strreset)(&mut ra);
    }
}

#[test]
fn err_32_stralloc_oversize() {
    let p = setup(32);
    // (a) fresh arena, oversize
    for &len in &[513usize, 1024, 100_000] {
        let mut ca = fresh_arena();
        let mut ra = fresh_arena();
        let mut s: Vec<u8> = vec![b'o'; len];
        s.push(0);
        let sp = s.as_mut_ptr() as *mut c_char;
        let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
        let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
        let ctx = format!("fresh oversize len={len}");
        dq(format!("{ctx} / content"), unsafe { cstr(cp) }, unsafe {
            cstr(rp)
        });
        dq(format!("{ctx} / remaining"), ca.remaining, ra.remaining);
        dq(format!("{ctx} / block"), ca.block, ra.block);
        assert_eq!(ca.remaining, 0, "[{ctx}] fresh oversize sets remaining = 0");
        assert_eq!(ca.block, 1, "[{ctx}] block is still incremented");
        dq(
            format!("{ctx} / offset"),
            (cp as usize) - (ca.storage as usize),
            (rp as usize) - (ra.storage as usize),
        );
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.rs.strreset)(&mut ra);
        }
    }
    // (b) arena with existing storage: the splice branch, which leaves
    //     `remaining` alone.
    let mut ca = fresh_arena();
    let mut ra = fresh_arena();
    let mut small = b"seed\0".to_vec();
    let smp = small.as_mut_ptr() as *mut c_char;
    unsafe {
        (p.c.stralloc)(&mut ca, smp);
        (p.rs.stralloc)(&mut ra, smp);
    }
    let rem_before = ca.remaining;
    assert_eq!(rem_before, ra.remaining);
    for round in 0..3 {
        let mut s: Vec<u8> = vec![b'O'; 2_000_000];
        s.push(0);
        let sp = s.as_mut_ptr() as *mut c_char;
        let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
        let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
        let ctx = format!("splice round={round}");
        dq(
            format!("{ctx} / content len"),
            unsafe { cstr(cp) }.len(),
            unsafe { cstr(rp) }.len(),
        );
        dq(format!("{ctx} / remaining"), ca.remaining, ra.remaining);
        dq(format!("{ctx} / block"), ca.block, ra.block);
        assert_eq!(
            ca.remaining, rem_before,
            "[{ctx}] the splice branch must not touch `remaining`"
        );
    }
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.rs.strreset)(&mut ra);
    }
}

#[test]
fn err_33_stralloc_empty_string() {
    let p = setup(33);
    let mut ca = fresh_arena();
    let mut ra = fresh_arena();
    let mut s = b"\0".to_vec();
    let sp = s.as_mut_ptr() as *mut c_char;
    let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
    let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
    assert_eq!(unsafe { cstr(cp) }, Vec::<u8>::new());
    assert_eq!(unsafe { cstr(rp) }, Vec::<u8>::new());
    dq("empty / remaining", ca.remaining, ra.remaining);
    dq("empty / block", ca.block, ra.block);
    assert_eq!(ca.remaining, 511, "512 - len(1)");
    assert_eq!(ca.block, 1);
    dq(
        "empty / offset",
        (cp as usize) - (ca.storage as usize),
        (rp as usize) - (ra.storage as usize),
    );
    // A run of empty strings fills the 512-byte block one byte at a time.
    for i in 0..600 {
        let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
        let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
        let ctx = format!("empty run i={i}");
        dq(format!("{ctx} / remaining"), ca.remaining, ra.remaining);
        dq(format!("{ctx} / block"), ca.block, ra.block);
        dq(
            format!("{ctx} / offset"),
            (cp as usize) - (ca.storage as usize),
            (rp as usize) - (ra.storage as usize),
        );
    }
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.rs.strreset)(&mut ra);
    }
}

#[test]
fn err_34_stralloc_block_saturation() {
    // blocksize = 512u << (block>>1); ++block only while blocksize < 1<<20.
    // So block climbs 0,1,...,22 and then stops.
    let p = setup(34);
    let mut ca = fresh_arena();
    let mut ra = fresh_arena();
    for i in 0..40 {
        let len = ca.remaining + 1; // always forces a new block
        assert_eq!(ca.remaining, ra.remaining, "diverged at i={i}");
        let mut s: Vec<u8> = vec![b'S'; len.saturating_sub(1)];
        s.push(0);
        let sp = s.as_mut_ptr() as *mut c_char;
        let cp = unsafe { (p.c.stralloc)(&mut ca, sp) };
        let rp = unsafe { (p.rs.stralloc)(&mut ra, sp) };
        let ctx = format!("saturate i={i} len={len} block={}", ca.block);
        dq(
            format!("{ctx} / content len"),
            unsafe { cstr(cp) }.len(),
            unsafe { cstr(rp) }.len(),
        );
        dq(format!("{ctx} / remaining"), ca.remaining, ra.remaining);
        dq(format!("{ctx} / block"), ca.block, ra.block);
    }
    assert_eq!(
        ca.block, 22,
        "block must saturate at 22 (512 << 11 == 1 MiB)"
    );
    assert_eq!(ra.block, 22);
    unsafe {
        (p.c.strreset)(&mut ca);
        (p.rs.strreset)(&mut ra);
    }
}

#[test]
fn err_35_strreset_empty() {
    let p = setup(35);
    for &(remaining, block, mode) in &[
        (0usize, 0u8, 0u8),
        (0, 5, 3),
        (12345, 200, 255),
    ] {
        let mut ca = StringArena { storage: std::ptr::null_mut(), remaining, block, mode };
        let mut ra = StringArena { storage: std::ptr::null_mut(), remaining, block, mode };
        unsafe {
            (p.c.strreset)(&mut ca);
            (p.rs.strreset)(&mut ra);
        }
        dq(format!("strreset({remaining},{block},{mode})"), ca, ra);
        assert!(ca.storage.is_null());
        assert_eq!((ca.remaining, ca.block, ca.mode), (0, 0, 0));
    }
}

// ---------------------------------------------------------------------------
// Rows 36-39 — the hash functions' boundaries
// ---------------------------------------------------------------------------

#[test]
fn err_36_hash_low_values() {
    // `if (hash < 2) hash += 2;` reserves 0 (EMPTY) and 1 (DELETED). The fixup
    // lives in the *callers* (lines 596 and 719), so verify it by recomputing
    // the expected slot hash from the exported hash function and finding it in
    // BOTH libraries' bucket arrays.
    let p = setup(36);
    let elemsize = 16usize;
    let keysize = 4usize;
    let mut cm = Map::new(&p.c, elemsize, keysize);
    let mut rm = Map::new(&p.rs, elemsize, keysize);
    for i in 0..400u32 {
        let mut ck = i.to_le_bytes().to_vec();
        let mut rk = ck.clone();
        let v = vec![i as u8; elemsize - keysize];
        cm.put_binary(&mut ck, &v, HM_BINARY);
        rm.put_binary(&mut rk, &v, HM_BINARY);
    }
    let ct = cm.table().unwrap();
    let rt = rm.table().unwrap();
    dq("tables", Some(ct.clone()), Some(rt.clone()));
    for i in 0..400u32 {
        let mut k = i.to_le_bytes();
        let raw_c = unsafe { (p.c.hash_bytes)(k.as_mut_ptr() as *mut c_void, keysize, ct.seed) };
        let raw_r = unsafe { (p.rs.hash_bytes)(k.as_mut_ptr() as *mut c_void, keysize, rt.seed) };
        assert_eq!(raw_c, raw_r, "raw hash for {i}");
        let expected = if raw_c < 2 { raw_c + 2 } else { raw_c };
        assert!(expected >= 2, "the fixup must lift 0/1 out of the way");
        let found = ct
            .buckets
            .iter()
            .any(|b| b.hash.iter().any(|&h| h == expected));
        assert!(found, "slot hash {expected:#x} for key {i} not in the C table");
        let found_r = rt
            .buckets
            .iter()
            .any(|b| b.hash.iter().any(|&h| h == expected));
        assert!(found_r, "slot hash {expected:#x} for key {i} not in the Rust table");
    }
    // No live slot may ever carry the reserved values 0 or 1.
    for b in &ct.buckets {
        for j in 0..BUCKET_LENGTH {
            if b.index[j] >= 0 {
                assert!(b.hash[j] >= 2, "live slot carries reserved hash {}", b.hash[j]);
            }
        }
    }
    cm.free();
    rm.free();
}

#[test]
fn err_37_hash_string_empty() {
    let p = setup(37);
    let mut s = b"\0".to_vec();
    let sp = s.as_mut_ptr() as *mut c_char;
    for &seed in &[
        0usize,
        1,
        2,
        0x3141_5926,
        usize::MAX,
        0x8000_0000_0000_0000,
    ] {
        let cv = unsafe { (p.c.hash_string)(sp, seed) };
        let rv = unsafe { (p.rs.hash_string)(sp, seed) };
        dq(format!("hash_string(\"\", {seed:#x})"), cv, rv);
    }
}

#[test]
fn err_38_hash_bytes_zero_len() {
    let p = setup(38);
    let mut dummy = [0xEEu8; 64];
    for &seed in &[
        0usize,
        1,
        2,
        0x3141_5926,
        usize::MAX,
        0x8000_0000_0000_0000,
        0xdead_beef,
    ] {
        let cv = unsafe { (p.c.hash_bytes)(dummy.as_mut_ptr() as *mut c_void, 0, seed) };
        let rv = unsafe { (p.rs.hash_bytes)(dummy.as_mut_ptr() as *mut c_void, 0, seed) };
        dq(format!("hash_bytes(len=0, {seed:#x})"), cv, rv);
        // len == 0 must ignore the buffer entirely.
        let mut other = [0x11u8; 64];
        let cv2 = unsafe { (p.c.hash_bytes)(other.as_mut_ptr() as *mut c_void, 0, seed) };
        assert_eq!(cv, cv2, "len=0 must not read the buffer");
    }
}

#[test]
fn err_39_hash_bytes_tail_lengths() {
    // Every `switch (len - i)` fall-through case, including `case 4`'s
    // sign-extending `data |= (d[3] << 24)`.
    let p = setup(39);
    let mut rng = Rng::new(39);
    for len in 1..=7usize {
        for &top in &[0x00u8, 0x01, 0x7f, 0x80, 0xfe, 0xff] {
            for _ in 0..400 {
                let mut d = rng.bytes(len);
                *d.last_mut().unwrap() = top;
                if len >= 4 {
                    d[3] = top;
                }
                let mut buf = d.clone();
                let ptr = buf.as_mut_ptr() as *mut c_void;
                let seed = rng.next_u64() as usize;
                let cv = unsafe { (p.c.hash_bytes)(ptr, len, seed) };
                let rv = unsafe { (p.rs.hash_bytes)(ptr, len, seed) };
                dq(format!("tail len={len} top={top:#02x} d={d:02x?}"), cv, rv);
            }
        }
    }
    // Distinct results per tail length for the same prefix (i.e. `len` really
    // participates via `data = len << 56`).
    let mut d = [1u8, 2, 3, 4, 5, 6, 7];
    let mut hs = std::collections::HashSet::new();
    for len in 1..=7usize {
        let h = unsafe { (p.c.hash_bytes)(d.as_mut_ptr() as *mut c_void, len, 0) };
        assert_eq!(h, unsafe {
            (p.rs.hash_bytes)(d.as_mut_ptr() as *mut c_void, len, 0)
        });
        hs.insert(h);
    }
    assert_eq!(hs.len(), 7, "each tail length must hash differently");
}

// ---------------------------------------------------------------------------
// Row 40 — out-of-range enum `mode` across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn err_40_mode_out_of_range() {
    let p = setup(40);
    let mut rng = Rng::new(40);

    // (a) Negative modes on a BINARY map must behave exactly like mode 0,
    //     because the only test is `mode >= STBDS_HM_STRING (1)`.
    for &mode in &[-1i32, -2, -1000, i32::MIN] {
        let elemsize = 16usize;
        let keysize = 4usize;
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        let keys: Vec<u32> = (0..150).map(|_| rng.next_u32()).collect();
        for (i, &k) in keys.iter().enumerate() {
            let mut ck = k.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            let v = vec![i as u8; elemsize - keysize];
            dq(
                format!("neg mode={mode} put#{i}"),
                cm.put_binary(&mut ck, &v, mode),
                rm.put_binary(&mut rk, &v, mode),
            );
            assert_same_binary(&format!("neg mode={mode} put#{i}"), &cm, &rm);
        }
        for (i, &k) in keys.iter().enumerate() {
            let mut ck = k.to_le_bytes();
            let mut rk = k.to_le_bytes();
            let ci = cm.geti(ck.as_mut_ptr() as *mut c_void, mode);
            let ri = rm.geti(rk.as_mut_ptr() as *mut c_void, mode);
            dq(format!("neg mode={mode} get#{i}"), ci, ri);
            assert!(ci >= 0, "negative mode must still use the binary path");
            let ct = cm.geti_ts(ck.as_mut_ptr() as *mut c_void, mode);
            let rt = rm.geti_ts(rk.as_mut_ptr() as *mut c_void, mode);
            dq(format!("neg mode={mode} get_ts#{i}"), ct, rt);
        }
        // hmdel_key with a negative mode: line 836 and 842 are both false, so
        // the raw key bytes are used for the re-find, which is the correct
        // thing for a binary map -> full delete coverage.
        let mut order: Vec<usize> = (0..keys.len()).collect();
        for i in (1..order.len()).rev() {
            let j = rng.below(i + 1);
            order.swap(i, j);
        }
        for (step, &oi) in order.iter().enumerate() {
            let mut ck = keys[oi].to_le_bytes();
            let mut rk = keys[oi].to_le_bytes();
            let ctx = format!("neg mode={mode} del step={step}");
            dq(
                format!("{ctx} / result"),
                cm.del(ck.as_mut_ptr() as *mut c_void, 0, mode),
                rm.del(rk.as_mut_ptr() as *mut c_void, 0, mode),
            );
            assert_same_binary(&ctx, &cm, &rm);
        }
        cm.free();
        rm.free();
    }

    // (b) mode >= 2 on a STRING map must behave exactly like mode 1 for
    //     put/get (`mode >= STBDS_HM_STRING`).
    for &mode in &[2i32, 3, 7, 1000, i32::MAX] {
        for sh in [None, Some(SH_DEFAULT), Some(SH_STRDUP), Some(SH_ARENA)] {
            let elemsize = 16usize;
            let keysize = 8usize;
            let (mut cm, mut rm) = match sh {
                None => (
                    Map::new(&p.c, elemsize, keysize),
                    Map::new(&p.rs, elemsize, keysize),
                ),
                Some(m) => (
                    Map::with_shmode(&p.c, elemsize, keysize, m),
                    Map::with_shmode(&p.rs, elemsize, keysize, m),
                ),
            };
            let keys: Vec<Vec<u8>> = (0..120).map(|i| format!("m{mode}_k{i}").into_bytes()).collect();
            for (i, k) in keys.iter().enumerate() {
                let v = vec![i as u8; elemsize - keysize];
                let ci = cm.put_string(leak_cstring(k), &v, mode);
                let ri = rm.put_string(leak_cstring(k), &v, mode);
                let ctx = format!("mode={mode} sh={sh:?} put#{i}");
                dq(format!("{ctx} / index"), ci, ri);
                assert_same_string(&ctx, &cm, &rm);
            }
            for (i, k) in keys.iter().enumerate() {
                let ci = cm.geti(leak_cstring(k) as *mut c_void, mode);
                let ri = rm.geti(leak_cstring(k) as *mut c_void, mode);
                dq(format!("mode={mode} sh={sh:?} get#{i}"), ci, ri);
                assert!(ci >= 0, "mode>=2 must use the string path");
                let ct = cm.geti_ts(leak_cstring(k) as *mut c_void, mode);
                let rt = rm.geti_ts(leak_cstring(k) as *mut c_void, mode);
                dq(format!("mode={mode} sh={sh:?} get_ts#{i}"), ct, rt);
            }
            // misses
            for i in 0..50 {
                let miss = format!("\x03miss{i}").into_bytes();
                let ct = cm.geti(leak_cstring(&miss) as *mut c_void, mode);
                let rt = rm.geti(leak_cstring(&miss) as *mut c_void, mode);
                dq(format!("mode={mode} sh={sh:?} miss#{i}"), ct, rt);
                assert_eq!(ct, -1);
            }
            // Deletes with mode >= 2 are restricted to the last element
            // (see err_28) — the interior case trips the C's live assert.
            for i in (0..keys.len()).rev() {
                let ctx = format!("mode={mode} sh={sh:?} del#{i}");
                dq(
                    format!("{ctx} / result"),
                    cm.del(leak_cstring(&keys[i]) as *mut c_void, 0, mode),
                    rm.del(leak_cstring(&keys[i]) as *mut c_void, 0, mode),
                );
                assert_same_string(&ctx, &cm, &rm);
            }
            cm.free();
            rm.free();
        }
    }

    // (c) mode >= 2 for *lookups* on a BINARY map: the string path hashes the
    //     raw key bytes as a C string, so lookups miss with -1 in both libs.
    //     (No `is_key_equal` can fire because the hash never matches, which is
    //     what keeps this safe.)
    {
        let elemsize = 16usize;
        let keysize = 4usize;
        let mut cm = Map::new(&p.c, elemsize, keysize);
        let mut rm = Map::new(&p.rs, elemsize, keysize);
        for i in 0..100u32 {
            let mut ck = i.to_le_bytes().to_vec();
            let mut rk = ck.clone();
            let v = vec![i as u8; elemsize - keysize];
            cm.put_binary(&mut ck, &v, HM_BINARY);
            rm.put_binary(&mut rk, &v, HM_BINARY);
        }
        for &mode in &[2i32, 5, i32::MAX] {
            for i in 0..100u32 {
                // NUL-terminated so hash_string cannot run off the end
                let mut k = [0u8; 8];
                k[0] = 1 + (i % 200) as u8;
                let ci = cm.geti_ts(k.as_mut_ptr() as *mut c_void, mode);
                let ri = rm.geti_ts(k.as_mut_ptr() as *mut c_void, mode);
                dq(format!("binary map string-mode get mode={mode} i={i}"), ci, ri);
            }
            assert_same_binary(&format!("binary map string-mode mode={mode}"), &cm, &rm);
        }
        cm.free();
        rm.free();
    }
}

// ---------------------------------------------------------------------------
// Row 41 — keysize == 0
// ---------------------------------------------------------------------------

#[test]
fn err_41_zero_keysize() {
    let p = setup(41);
    let mut rng = Rng::new(41);
    for &elemsize in &[1usize, 4, 8, 16] {
        let mut cm = Map::new(&p.c, elemsize, 0);
        let mut rm = Map::new(&p.rs, elemsize, 0);
        for i in 0..80 {
            let v = rng.bytes(elemsize);
            let ci = cm.put_binary(&mut [], &v, HM_BINARY);
            let ri = rm.put_binary(&mut [], &v, HM_BINARY);
            dq(format!("es={elemsize} zero-keysize put#{i}"), ci, ri);
            assert_same_binary(&format!("es={elemsize} zero-keysize put#{i}"), &cm, &rm);
        }
        // gets and dels with keysize 0
        for i in 0..40 {
            let ci = cm.geti(std::ptr::null_mut(), HM_BINARY);
            let ri = rm.geti(std::ptr::null_mut(), HM_BINARY);
            dq(format!("es={elemsize} zero-keysize get#{i}"), ci, ri);
        }
        for i in 0..40 {
            let ctx = format!("es={elemsize} zero-keysize del#{i}");
            dq(
                format!("{ctx} / result"),
                cm.del(std::ptr::null_mut(), 0, HM_BINARY),
                rm.del(std::ptr::null_mut(), 0, HM_BINARY),
            );
            assert_same_binary(&ctx, &cm, &rm);
        }
        cm.free();
        rm.free();
    }
}

// ---------------------------------------------------------------------------
// Rows 42-43 — strkey
// ---------------------------------------------------------------------------

#[test]
fn err_42_strkey_int_min() {
    let p = setup(42);
    let cp = unsafe { (p.c.strkey)(i32::MIN) };
    let cs = unsafe { cstr(cp) };
    let rp = unsafe { (p.rs.strkey)(i32::MIN) };
    let rs = unsafe { cstr(rp) };
    dq("strkey(INT_MIN)", cs.clone(), rs);
    assert_eq!(cs, b"test_-2147483648".to_vec());
    assert_eq!(cs.len(), 16);
}

#[test]
fn err_43_strkey_boundaries() {
    let p = setup(43);
    for n in [
        0i32,
        1,
        -1,
        9,
        10,
        -9,
        -10,
        99,
        -99,
        100,
        -100,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        1_000_000_000,
        -1_000_000_000,
    ] {
        let cs = unsafe { cstr((p.c.strkey)(n)) };
        let rs = unsafe { cstr((p.rs.strkey)(n)) };
        dq(format!("strkey({n})"), cs.clone(), rs);
        assert_eq!(cs, format!("test_{n}").into_bytes());
    }
    // The buffer is a 256-byte static; the longest output is 17 bytes incl. NUL.
    let cs = unsafe { cstr((p.c.strkey)(i32::MIN)) };
    assert!(cs.len() + 1 <= 256);
}

// ---------------------------------------------------------------------------
// Row 44 — arr_del boundaries
// ---------------------------------------------------------------------------

#[test]
fn err_44_arr_del_boundaries() {
    let p = setup(44);
    // The i == 3 iteration makes `arrdeln`'s memmove count `4-1-3 == 0` and
    // `arrdelswap` a self-assignment. `arr_del` is void, so the observable
    // requirement is simply that both libraries complete without aborting or
    // corrupting the allocator (a bad memmove length would trip glibc).
    for n in [
        0i32,
        1,
        -1,
        2,
        3,
        4,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ] {
        unsafe {
            (p.c.arr_del)(n);
            (p.rs.arr_del)(n);
        }
    }
    // Repeat many times to shake out any leak/corruption asymmetry.
    let mut rng = Rng::new(44);
    for _ in 0..2000 {
        let n = rng.next_u32() as i32;
        unsafe {
            (p.c.arr_del)(n);
            (p.rs.arr_del)(n);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 17, 18, 23-26, 31 — STBDS_ASSERT abort parity
// ---------------------------------------------------------------------------
//
// `#define STBDS_ASSERT assert` (lib.c:3) and the C is built without `NDEBUG`
// (its `.so` links `__assert_fail`), so all 7 asserts are LIVE and `abort()`.
// The Rust reproduces them with `assert!`, and a panic under an `extern "C"` fn
// also aborts — so a violated invariant must kill the process with the SAME
// signal in both libraries.
//
// This cannot be asserted in-process (the abort is the observable), so each
// scenario is run in a SUBPROCESS: the test re-execs this very test binary with
// `STBDS_ABORT_SCENARIO` / `STBDS_ABORT_LIB` set, and compares the two exit
// statuses.

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

/// `(scenario, must_abort)`.
///
/// Only `lib.c:846` is reachable through the public API. The other six asserts
/// (401, 778, 828, 832, 849, 913) are unreachable for every input a caller can
/// supply — 828 because `stbds_hm_find_slot` returns `(pos & ~7) + i` with
/// `pos < slot_count` and `i < 8`, so the result is always `< slot_count`; 832
/// because `used_count` is a `size_t`; 401 because every `slot_count` the API can
/// produce is a power of two `>= 8`, for which `0.75n + 0.1875n < n`; 778 because
/// the preceding `arrgrowf` guarantees it; 913 because the `len > blocksize`
/// branch returns early and the other branch sets `remaining = blocksize >= len`;
/// 849 because a successful re-find always lands on the moved element's own slot.
/// They are nonetheless present in the Rust now, so if any ever does fire the two
/// libraries abort identically instead of the C aborting and the Rust performing
/// a wild pointer write.
const SCENARIOS: &[(&str, bool)] = &[
    // Control: a perfectly ordinary workload. Must exit 0 for both — this
    // proves the harness can tell "aborted" from "did not abort".
    ("control_ok", false),
    // lib.c:846 `STBDS_ASSERT(slot >= 0)`. `stbds_hmdel_key` tests
    // `mode == STBDS_HM_STRING` (exactly 1) at line 842, but
    // `stbds_hm_find_slot` tests `mode >= 1`. With `mode > 1` and an INTERIOR
    // delete, the re-find is handed the raw bytes of the stored `char *`, hashes
    // them as a string, finds nothing, and returns -1.
    ("hmdel_mode2_interior_default", true),
    ("hmdel_mode2_interior_strdup", true),
    ("hmdel_mode2_interior_arena", true),
    ("hmdel_mode3_interior_default", true),
    ("hmdel_modemax_interior_default", true),
    // A non-zero `keyoffset` pointing at a field that matches no key: the
    // *initial* `stbds_hm_find_slot` already misses, so this is the row-22
    // early return, NOT an abort. Included to pin that down.
    ("hmdel_bad_keyoffset", false),
];

fn scenario_status(lib: &str, scenario: &str) -> std::process::ExitStatus {
    let exe = std::env::current_exe().expect("current_exe");
    Command::new(exe)
        .args([
            "--exact",
            "abort_scenario_runner",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("STBDS_ABORT_SCENARIO", scenario)
        .env("STBDS_ABORT_LIB", lib)
        .env_remove("RUST_BACKTRACE")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn scenario")
}

#[test]
fn err_asserts_abort_parity() {
    // Never recurse: inside the child this env var is set.
    if std::env::var_os("STBDS_ABORT_SCENARIO").is_some() {
        return;
    }
    let mut aborted = 0usize;
    let mut survived = 0usize;
    for &(scenario, must_abort) in SCENARIOS {
        let c = scenario_status("c", scenario);
        let r = scenario_status("rust", scenario);
        // THE assertion: identical termination status.
        assert_eq!(
            (c.code(), c.signal()),
            (r.code(), r.signal()),
            "[{scenario}] C exited {c:?} but Rust exited {r:?}"
        );
        // The harness process is killed by the abort, so depending on how it is
        // reaped this shows up as signal 6 or as code 134.
        let died = c.signal() == Some(6) || c.code() == Some(134);
        assert_eq!(
            died, must_abort,
            "[{scenario}] expected must_abort={must_abort}, got {c:?}"
        );
        if died {
            aborted += 1;
        } else {
            assert_eq!(c.code(), Some(0), "[{scenario}] expected a clean exit");
            survived += 1;
        }
    }
    assert!(aborted >= 5, "expected >= 5 aborting scenarios, got {aborted}");
    assert!(survived >= 2, "expected >= 2 clean scenarios, got {survived}");
}

/// The child half of `err_asserts_abort_parity`. A no-op unless
/// `STBDS_ABORT_SCENARIO` is set, so a normal `cargo test` run ignores it.
#[test]
fn abort_scenario_runner() {
    let Some(scenario) = std::env::var_os("STBDS_ABORT_SCENARIO") else {
        return;
    };
    let scenario = scenario.to_string_lossy().into_owned();
    let which = std::env::var("STBDS_ABORT_LIB").unwrap();
    // Open only ONE library: no global lock, no cross-library interference.
    let lib = if which == "c" {
        Lib::open_c()
    } else {
        Lib::open_rust()
    };
    unsafe { (lib.rand_seed)(0x3141_5926) };

    let elemsize = 16usize;
    let keysize = 8usize;

    match scenario.as_str() {
        "control_ok" => {
            // An ordinary string map: build it, look everything up, delete it
            // all with the correct mode, free it. Must not abort.
            let mut m = Map::with_shmode(&lib, elemsize, keysize, SH_DEFAULT);
            let keys: Vec<Vec<u8>> = (0..50).map(|i| format!("ok_{i}").into_bytes()).collect();
            for (i, k) in keys.iter().enumerate() {
                m.put_string(leak_cstring(k), &vec![i as u8; elemsize - keysize], HM_STRING);
            }
            for k in &keys {
                assert!(m.geti(leak_cstring(k) as *mut c_void, HM_STRING) >= 0);
            }
            for k in &keys {
                m.del(leak_cstring(k) as *mut c_void, 0, HM_STRING);
            }
            assert_eq!(m.len(), 0);
            m.free();
        }
        "hmdel_mode2_interior_default"
        | "hmdel_mode2_interior_strdup"
        | "hmdel_mode2_interior_arena"
        | "hmdel_mode3_interior_default"
        | "hmdel_modemax_interior_default" => {
            let mode: c_int = if scenario.contains("mode2") {
                2
            } else if scenario.contains("mode3") {
                3
            } else {
                i32::MAX
            };
            let sh = if scenario.ends_with("strdup") {
                SH_STRDUP
            } else if scenario.ends_with("arena") {
                SH_ARENA
            } else {
                SH_DEFAULT
            };
            let mut m = Map::with_shmode(&lib, elemsize, keysize, sh);
            let keys: Vec<Vec<u8>> = (0..6).map(|i| format!("k{i}").into_bytes()).collect();
            for (i, k) in keys.iter().enumerate() {
                m.put_string(leak_cstring(k), &vec![i as u8; elemsize - keysize], HM_STRING);
            }
            // keys[0] is at element 0, the last element is 5 -> interior.
            m.del(leak_cstring(&keys[0]) as *mut c_void, 0, mode);
            m.free();
        }
        "hmdel_bad_keyoffset" => {
            // Binary map whose value half is a constant sentinel. Deleting an
            // interior element with keyoffset = 4 makes the re-find look for the
            // moved element's *sentinel* bytes, which no slot's key hashes to,
            // so `stbds_hm_find_slot` returns -1.
            let ks = 4usize;
            let mut m = Map::new(&lib, elemsize, ks);
            for i in 0..8u32 {
                let mut k = i.to_le_bytes().to_vec();
                let mut v = 0xDEAD_BEEFu32.to_le_bytes().to_vec();
                v.extend_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
                v.extend_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
                m.put_binary(&mut k, &v, HM_BINARY);
            }
            let mut k = 0u32.to_le_bytes();
            m.del(k.as_mut_ptr() as *mut c_void, 4, HM_BINARY);
            m.free();
        }
        other => panic!("unknown scenario {other}"),
    }
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// Row 34 (extended) — stbds_stralloc across the FULL range of `a->block`
// ---------------------------------------------------------------------------
//
// `a->block` is an `unsigned char`, and `stbds_stralloc` is EXPORTED, so a caller
// can hand it any value 0..=255. The C then computes
// `(size_t) 512 << (a->block >> 1)` with a shift count up to 127; gcc emits
// `shlq %cl`, which masks the count to 6 bits, so e.g. `block = 128` gives count
// `64 & 63 == 0` and `blocksize == 512`.
//
// This is the second divergence verification found: the Rust used a plain `<<`,
// which is fine in the release profile (`overflow-checks = false`) but PANICS in
// the dev profile for any count >= 64. It now uses `wrapping_shl`, whose count is
// masked the same way.
//
// Each (library, block) pair runs in its own subprocess because some `block`
// values make the C's unchecked `realloc` (ERRORS.md row 4) return NULL, which the
// C then writes through.

/// `a->block` values around every interesting boundary: the `>> 1` pairs, the
/// 1 MiB saturation point (22), the first count that reaches 64 before masking,
/// and the wrap of `++a->block` at 255.
const BLOCKS: &[u8] = &[
    0, 1, 2, 3, 20, 21, 22, 23, 24, 42, 43, 44, 45, 62, 63, 64, 65, 100, 126, 127, 128, 129, 130,
    199, 200, 252, 253, 254, 255,
];

#[derive(Debug, PartialEq, Eq)]
enum StrallocOutcome {
    /// Returned normally: (string contents, a->block, a->remaining, offset of the
    /// returned pointer inside its block).
    Ok(Vec<u8>, u8, usize, usize),
    /// The process died. `stbds_realloc` is unchecked in the C (lib.c:894/906), so
    /// for large `blocksize` it returns NULL and the C stores through it.
    Crashed,
}

fn stralloc_outcome(lib: &Lib, block: u8) -> StrallocOutcome {
    let mut arena = StringArena {
        storage: std::ptr::null_mut(),
        remaining: 0,
        block,
        mode: 0,
    };
    let mut s = b"hello\0".to_vec();
    let p = unsafe { (lib.stralloc)(&mut arena, s.as_mut_ptr() as *mut c_char) };
    StrallocOutcome::Ok(
        unsafe { cstr(p) },
        arena.block,
        arena.remaining,
        (p as usize).wrapping_sub(arena.storage as usize),
    )
}

#[test]
fn err_34b_stralloc_block_full_range() {
    if std::env::var_os("STBDS_BLOCK").is_some() {
        return; // we are the child
    }
    let mut matched_ok = 0usize;
    let mut matched_crash = 0usize;
    for &block in BLOCKS {
        let c = block_child("c", block);
        let r = block_child("rust", block);
        match (&c, &r) {
            (StrallocOutcome::Ok(..), StrallocOutcome::Ok(..)) => {
                assert_eq!(c, r, "stralloc block={block}: C {c:?} vs Rust {r:?}");
                matched_ok += 1;
            }
            (StrallocOutcome::Crashed, StrallocOutcome::Crashed) => {
                // The C dereferences the NULL from its unchecked `realloc`
                // (ERRORS.md row 4). The release Rust faults identically
                // (SIGSEGV); the dev profile's null-pointer check traps the same
                // UB one step earlier and aborts instead. Either way both die on
                // the same input, which is all that is defined here.
                matched_crash += 1;
            }
            _ => panic!("stralloc block={block}: C {c:?} but Rust {r:?}"),
        }
    }
    assert!(
        matched_ok >= 20,
        "expected >= 20 blocks where both return normally, got {matched_ok}"
    );
    assert!(
        matched_crash >= 1,
        "expected >= 1 block where the C's unchecked realloc faults, got {matched_crash}"
    );
}

fn block_child(which: &str, block: u8) -> StrallocOutcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out = Command::new(exe)
        .args([
            "--exact",
            "stralloc_block_runner",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("STBDS_BLOCK", block.to_string())
        .env("STBDS_ABORT_LIB", which)
        .env_remove("RUST_BACKTRACE")
        .stderr(Stdio::null())
        .output()
        .expect("spawn block child");
    if !out.status.success() && out.status.code() != Some(0) {
        // died by signal, or the harness reported failure
        let txt = String::from_utf8_lossy(&out.stdout);
        if !txt.contains("STRALLOC_RESULT ") {
            return StrallocOutcome::Crashed;
        }
    }
    let txt = String::from_utf8_lossy(&out.stdout);
    let line = txt
        .lines()
        .find(|l| l.contains("STRALLOC_RESULT "))
        .unwrap_or_else(|| {
            panic!(
                "child produced no result for {which} block={block}: status={:?} stdout={txt:?}",
                out.status
            )
        });
    let line = &line[line.find("STRALLOC_RESULT ").unwrap()..];
    let f: Vec<&str> = line.split_whitespace().collect();
    StrallocOutcome::Ok(
        f[1].as_bytes().to_vec(),
        f[2].parse().unwrap(),
        f[3].parse().unwrap(),
        f[4].parse().unwrap(),
    )
}

/// Child half of `err_34b_stralloc_block_full_range`.
#[test]
fn stralloc_block_runner() {
    let Some(block) = std::env::var_os("STBDS_BLOCK") else {
        return;
    };
    let block: u8 = block.to_string_lossy().parse().unwrap();
    let which = std::env::var("STBDS_ABORT_LIB").unwrap();
    let lib = if which == "c" {
        Lib::open_c()
    } else {
        Lib::open_rust()
    };
    match stralloc_outcome(&lib, block) {
        StrallocOutcome::Ok(s, b, rem, off) => {
            println!(
                "STRALLOC_RESULT {} {} {} {}",
                String::from_utf8_lossy(&s),
                b,
                rem,
                off
            );
            use std::io::Write;
            std::io::stdout().flush().ok();
        }
        StrallocOutcome::Crashed => unreachable!(),
    }
    std::process::exit(0);
}
