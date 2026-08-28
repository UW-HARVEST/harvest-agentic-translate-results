//! Level 6: the branches the ordinary API-level tests never reach.
//!
//! Identified by running the whole suite against a gcov-instrumented build of
//! `c_src/src/lib.c`; everything still uncovered afterwards is either an
//! assertion-failure path or requires a genuine 64-bit hash collision that the
//! `hash_collision_with_unequal_key` test forges by hand.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

unsafe fn reseed(seed: usize) {
    let libs = libs();
    libs.c.rand_seed(seed);
    libs.rs.rand_seed(seed);
}

unsafe fn table_of(t: *mut u8, elemsize: usize) -> *mut HashIndex {
    let raw = t.sub(elemsize);
    (*((raw as *mut ArrHeader).offset(-1))).hash_table as *mut HashIndex
}

#[derive(Debug, PartialEq, Eq)]
struct ArenaSnap {
    remaining: usize,
    block: u8,
    mode: u8,
    chain_len: usize,
}

#[repr(C)]
struct StringBlock {
    next: *const StringBlock,
    storage: [c_char; 8],
}

unsafe fn snap_arena(a: *const StringArena) -> ArenaSnap {
    let mut chain_len = 0usize;
    let mut p = (*a).storage as *const StringBlock;
    while !p.is_null() {
        chain_len += 1;
        p = (*p).next;
        assert!(chain_len < 1_000_000, "arena chain loop");
    }
    ArenaSnap {
        remaining: (*a).remaining,
        block: (*a).block,
        mode: (*a).mode,
        chain_len,
    }
}

#[test]
fn arena_block_counter_saturates() {
    // `stbds_stralloc` doubles its block size every second allocation and stops
    // incrementing `a->block` once the block size reaches
    // STBDS_STRING_ARENA_BLOCKSIZE_MAX (1 MiB). Reaching the cap takes a few
    // MiB of traffic; two further block allocations *at* the cap are needed to
    // actually run the "do not increment" path.
    let _g = guard();
    let libs = libs();
    let mut ca = StringArena::zeroed();
    let mut ra = StringArena::zeroed();
    let mut buf = CStrBuf::from_bytes(&vec![b'k'; 4000]);

    unsafe {
        let mut blocks_at_cap = 0usize;
        let mut prev_chain = 0usize;
        let mut saturated = false;
        for step in 0..20_000 {
            let cp = libs.c.stralloc(&mut ca, buf.as_ptr());
            let rp = libs.rs.stralloc(&mut ra, buf.as_ptr());
            assert_eq!(read_cstr(cp), read_cstr(rp), "step {step}");
            let cs = snap_arena(&ca);
            assert_eq!(cs, snap_arena(&ra), "arena diverged at step {step}");

            // 512 << (22 >> 1) == 1 MiB, so `block` saturates at 22.
            if cs.block >= 22 {
                assert_eq!(cs.block, 22, "block counter moved past the cap");
                saturated = true;
                if cs.chain_len > prev_chain {
                    blocks_at_cap += 1;
                }
            }
            prev_chain = cs.chain_len;
            if blocks_at_cap >= 2 {
                break;
            }
        }
        assert!(saturated, "never reached the 1 MiB block-size cap");
        assert!(
            blocks_at_cap >= 2,
            "never allocated a second block while at the cap"
        );
        libs.c.strreset(&mut ca);
        libs.rs.strreset(&mut ra);
        assert_eq!(snap_arena(&ca), snap_arena(&ra));
    }
}

#[test]
fn hmput_default_on_zero_length_map() {
    // `stbds_hmput_default`'s second condition (`length == 0` on a non-NULL
    // map) and its `a ? HASH_TO_ARR(a) : NULL` argument are only reachable if
    // an external caller hands it an array that has capacity but no elements -
    // exactly what `stbds_arrgrowf` produces.
    let _g = guard();
    let libs = libs();
    let es = 8usize;
    unsafe {
        let craw = libs.c.arrgrowf(std::ptr::null_mut(), es, 0, 1);
        let rraw = libs.rs.arrgrowf(std::ptr::null_mut(), es, 0, 1);
        assert_eq!(snap_arr(craw, Fmt::Raw, 0), snap_arr(rraw, Fmt::Raw, 0));
        assert_eq!((*(craw as *mut ArrHeader).offset(-1)).length, 0);

        let ct = libs.c.hmput_default(craw.add(es) as *mut c_void, es);
        let rt = libs.rs.hmput_default(rraw.add(es) as *mut c_void, es);
        assert_eq!(
            snap_hm(ct, Fmt::BinaryKV),
            snap_hm(rt, Fmt::BinaryKV),
            "hmput_default on a zero-length map"
        );

        libs.c.hmfree_func(ct.sub(es) as *mut c_void, es);
        libs.rs.hmfree_func(rt.sub(es) as *mut c_void, es);
    }
}

#[test]
fn hash_collision_with_unequal_key() {
    // `stbds_hm_find_slot` and `stbds_hmput_key` both have a branch for
    // "the stored hash matches but the key does not". Real collisions are a
    // 2^-64 event, so plant one: write a probe key's hash into an *empty* slot
    // and point that slot at element 0, whose key differs.
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::BinaryKV;
    let es = fmt.elemsize();

    unsafe {
        reseed(0x5EED_5EED);
        let mut ct: *mut u8 = std::ptr::null_mut();
        let mut rt: *mut u8 = std::ptr::null_mut();
        for k in 1..40i32 {
            ct = hmput_i32(&libs.c, ct, k, k);
            rt = hmput_i32(&libs.rs, rt, k, k);
        }
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "setup");

        let ctab = table_of(ct, es);
        let rtab = table_of(rt, es);
        let slot_count = (*ctab).slot_count;
        assert_eq!(slot_count, (*rtab).slot_count);
        let seed = (*ctab).seed;
        assert_eq!(seed, (*rtab).seed);

        // Find an absent key whose first probe slot is currently empty.
        let mut planted = None;
        for cand in 10_000i32..12_000 {
            let mut key = cand;
            let mut h = libs
                .c
                .hash_bytes(&mut key as *mut i32 as *mut c_void, 4, seed);
            // Cross-check the exported hash from both libraries.
            let h2 = libs
                .rs
                .hash_bytes(&mut key as *mut i32 as *mut c_void, 4, seed);
            assert_eq!(h, h2, "hash_bytes disagreed for {cand}");
            if h < 2 {
                h += 2;
            }
            let pos = h & (slot_count - 1);
            let bi = pos >> 3;
            let si = pos & 7;
            if (*(*ctab).storage.add(bi)).hash[si] == 0 {
                planted = Some((cand, h, bi, si));
                break;
            }
        }
        let (cand, h, bi, si) = planted.expect("no candidate key landed on an empty slot");

        // Plant the identical forged entry in both tables: element 0 is the
        // zeroed default slot, so its key (0) never equals `cand`.
        for tab in [ctab, rtab] {
            let b = (*tab).storage.add(bi);
            (*b).hash[si] = h;
            (*b).index[si] = 0;
        }
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "after planting");

        // Lookup: hash hit, key miss -> must keep probing and report absent.
        let (c2, ci) = hmgeti_i32(&libs.c, ct, cand);
        let (r2, ri) = hmgeti_i32(&libs.rs, rt, cand);
        ct = c2;
        rt = r2;
        assert_eq!(ci, ri, "collided lookup for {cand}");
        assert_eq!(ci, -1, "forged slot must not be reported as a match");
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt));

        // Delete: same find_slot path.
        let (c2, cr) = hmdel_i32(&libs.c, ct, cand);
        let (r2, rr) = hmdel_i32(&libs.rs, rt, cand);
        ct = c2;
        rt = r2;
        assert_eq!(cr, rr, "collided delete for {cand}");
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt));

        // Insert: hmput_key's own "hash matches, key differs" branch.
        ct = hmput_i32(&libs.c, ct, cand, -7);
        rt = hmput_i32(&libs.rs, rt, cand, -7);
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "collided insert");

        // And it must be findable afterwards.
        let (c2, ci) = hmgeti_i32(&libs.c, ct, cand);
        let (r2, ri) = hmgeti_i32(&libs.rs, rt, cand);
        ct = c2;
        rt = r2;
        assert_eq!(ci, ri);
        assert!(ci >= 0);

        libs.c.hmfree_func(ct.sub(es) as *mut c_void, es);
        libs.rs.hmfree_func(rt.sub(es) as *mut c_void, es);
    }
}

#[test]
fn string_hash_collision_with_unequal_key() {
    // Same forged collision, but in STBDS_HM_STRING mode so the strcmp side of
    // `stbds_is_key_equal` runs with a mismatching key.
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::StrKV;
    let es = fmt.elemsize();

    let mut keys: Vec<Vec<u8>> = (0..40)
        .map(|i| {
            let mut b = format!("key_{}", i).into_bytes();
            b.push(0);
            b
        })
        .collect();

    unsafe {
        reseed(0x1234_ABCD);
        let mut ct = libs.c.shmode_func(es, SH_STRDUP);
        let mut rt = libs.rs.shmode_func(es, SH_STRDUP);
        for i in 0..keys.len() {
            let k = keys[i].as_mut_ptr() as *mut c_char;
            ct = shput(&libs.c, ct, k, i as i32);
            rt = shput(&libs.rs, rt, k, i as i32);
        }
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "setup");

        let ctab = table_of(ct, es);
        let rtab = table_of(rt, es);
        let slot_count = (*ctab).slot_count;
        let seed = (*ctab).seed;
        assert_eq!(seed, (*rtab).seed);

        let mut planted = None;
        for n in 0..2000 {
            let mut cand = CStrBuf::new(&format!("absent_{}", n));
            let p = cand.as_ptr();
            let mut h = libs.c.hash_string(p, seed);
            assert_eq!(h, libs.rs.hash_string(p, seed), "hash_string disagreed");
            if h < 2 {
                h += 2;
            }
            let pos = h & (slot_count - 1);
            let bi = pos >> 3;
            let si = pos & 7;
            if (*(*ctab).storage.add(bi)).hash[si] == 0 {
                planted = Some((n, h, bi, si));
                break;
            }
        }
        let (n, h, bi, si) = planted.expect("no candidate string landed on an empty slot");
        let mut cand = CStrBuf::new(&format!("absent_{}", n));

        // Point the forged slot at element 1 (a real entry with a real key
        // pointer), so `strcmp` compares two valid, different strings.
        for tab in [ctab, rtab] {
            let b = (*tab).storage.add(bi);
            (*b).hash[si] = h;
            (*b).index[si] = 1;
        }
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "after planting");

        let (c2, ci) = shgeti(&libs.c, ct, cand.as_ptr());
        let (r2, ri) = shgeti(&libs.rs, rt, cand.as_ptr());
        ct = c2;
        rt = r2;
        assert_eq!(ci, ri, "collided string lookup");
        assert_eq!(ci, -1);

        ct = shput(&libs.c, ct, cand.as_ptr(), 999);
        rt = shput(&libs.rs, rt, cand.as_ptr(), 999);
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "collided string insert");

        libs.c.hmfree_func(ct.sub(es) as *mut c_void, es);
        libs.rs.hmfree_func(rt.sub(es) as *mut c_void, es);
    }
}

#[test]
fn wrap_around_collision_with_unequal_key() {
    // Both `stbds_hm_find_slot` and `stbds_hmput_key` scan a bucket twice:
    // from `pos & 7` up to 7, then from 0 up to `pos & 7`. The
    // "hash matches, key differs" branch of that *second* (wrap-around) scan is
    // only reachable if the colliding entry sits below the probe's start index
    // and no empty slot is seen on the way there.
    //
    // So: pick an absent key whose probe lands in a completely empty bucket at
    // index >= 2, fill slots [pos&7 .. 7] with a non-matching non-empty hash,
    // and plant the colliding hash at slot 0 of the same bucket.
    let _g = guard();
    let libs = libs();
    let fmt = Fmt::BinaryKV;
    let es = fmt.elemsize();
    const FILLER: usize = 0x0BAD_0BAD_0BAD_0BAD;

    unsafe {
        reseed(0xC011DE);
        let mut ct: *mut u8 = std::ptr::null_mut();
        let mut rt: *mut u8 = std::ptr::null_mut();
        for k in 1..30i32 {
            ct = hmput_i32(&libs.c, ct, k, k);
            rt = hmput_i32(&libs.rs, rt, k, k);
        }
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "setup");

        let ctab = table_of(ct, es);
        let rtab = table_of(rt, es);
        let slot_count = (*ctab).slot_count;
        let seed = (*ctab).seed;
        assert_eq!(seed, (*rtab).seed);

        let mut chosen = None;
        for cand in 10_000i32..200_000 {
            let mut key = cand;
            let mut h = libs
                .c
                .hash_bytes(&mut key as *mut i32 as *mut c_void, 4, seed);
            assert_eq!(
                h,
                libs.rs
                    .hash_bytes(&mut key as *mut i32 as *mut c_void, 4, seed),
                "hash_bytes disagreed for {cand}"
            );
            if h < 2 {
                h += 2;
            }
            if h == FILLER {
                continue;
            }
            let pos = h & (slot_count - 1);
            let start = pos & 7;
            if start < 2 {
                continue;
            }
            let b = (*ctab).storage.add(pos >> 3);
            // Only the slots this test writes to (and slot 1, where the later
            // insert must land) need to be free.
            let free = (start..8).all(|i| (*b).hash[i] == 0)
                && (*b).hash[0] == 0
                && (*b).hash[1] == 0;
            if free {
                chosen = Some((cand, h, pos >> 3, start));
                break;
            }
        }
        let (cand, h, bi, start) = chosen.expect("no candidate landed in a usable bucket");

        for tab in [ctab, rtab] {
            let b = (*tab).storage.add(bi);
            for i in start..8 {
                (*b).hash[i] = FILLER;
                (*b).index[i] = 0;
            }
            (*b).hash[0] = h;
            (*b).index[0] = 0; // element 0's key is 0, never equal to `cand`
        }
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "after planting");

        // find_slot: wrap-around scan, hash hit, key miss.
        let (c2, ci) = hmgeti_i32(&libs.c, ct, cand);
        let (r2, ri) = hmgeti_i32(&libs.rs, rt, cand);
        ct = c2;
        rt = r2;
        assert_eq!(ci, ri, "wrap-around collided lookup for {cand}");
        assert_eq!(ci, -1);
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt));

        // hmput_key: same wrap-around scan, then insert into the empty slot 1.
        ct = hmput_i32(&libs.c, ct, cand, 12345);
        rt = hmput_i32(&libs.rs, rt, cand, 12345);
        assert_eq!(
            snap_hm(ct, fmt),
            snap_hm(rt, fmt),
            "wrap-around collided insert"
        );

        let (c2, ci) = hmgeti_i32(&libs.c, ct, cand);
        let (r2, ri) = hmgeti_i32(&libs.rs, rt, cand);
        ct = c2;
        rt = r2;
        assert_eq!(ci, ri);
        assert!(ci >= 0, "inserted key not findable");

        // Re-putting it takes the wrap-around "key already present" branch.
        ct = hmput_i32(&libs.c, ct, cand, 999);
        rt = hmput_i32(&libs.rs, rt, cand, 999);
        assert_eq!(snap_hm(ct, fmt), snap_hm(rt, fmt), "wrap-around overwrite");

        libs.c.hmfree_func(ct.sub(es) as *mut c_void, es);
        libs.rs.hmfree_func(rt.sub(es) as *mut c_void, es);
    }
}
