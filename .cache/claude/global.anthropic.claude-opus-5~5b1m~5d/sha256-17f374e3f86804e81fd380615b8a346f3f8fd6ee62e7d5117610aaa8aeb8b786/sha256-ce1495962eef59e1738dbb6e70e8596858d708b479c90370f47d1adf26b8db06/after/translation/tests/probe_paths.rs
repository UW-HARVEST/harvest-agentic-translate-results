//! Phase B (extra) — the multi-bucket probe path.
//!
//! `stbds_hm_find_slot` and `stbds_hmput_key` both end their outer loop with
//!
//! ```c
//!     pos += step;
//!     step += STBDS_BUCKET_LENGTH;
//!     pos &= (table->slot_count-1);
//! ```
//!
//! which is only reached when a whole 8-slot bucket contains neither the key
//! nor an empty slot.  Random data essentially never produces two *consecutive*
//! full buckets, so the `step` growth (2nd hop onwards) is unreachable by
//! property testing.  These tests therefore build the bucket array by hand —
//! byte-identically in the C map and in the Rust map — and then drive the real
//! exported entry points over it.

mod common;
use common::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A crafted bucket layout can make a *diverging* probe loop spin forever
/// (the C terminates, a wrong `step`/`pos` update does not).  Turn that into a
/// fast, clearly-labelled failure instead of a hung test run.
struct Watchdog(Arc<AtomicBool>);

impl Watchdog {
    fn new(secs: u64, what: &'static str) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        let d2 = done.clone();
        std::thread::spawn(move || {
            for _ in 0..(secs * 20) {
                std::thread::sleep(Duration::from_millis(50));
                if d2.load(Ordering::SeqCst) {
                    return;
                }
            }
            eprintln!(
                "WATCHDOG: `{what}` did not finish within {secs}s — a probe loop \
                 diverged from the C implementation (it never terminates)."
            );
            std::process::abort();
        });
        Watchdog(done)
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

const ELEMSIZE: usize = 16;
const KEYSIZE: usize = 8;

const HASH_EMPTY: usize = 0;
const HASH_DELETED: usize = 1;
const INDEX_EMPTY: isize = -1;
const INDEX_DELETED: isize = -2;

/// Full description of an artificial hash-index state.
struct TableSpec {
    /// (slot, hash, index) triples; every slot not listed is EMPTY.
    slots: Vec<(usize, usize, isize)>,
    used_count: usize,
    tombstone_count: usize,
}

unsafe fn table_of(t: *mut c_void, elemsize: usize) -> *mut HashIndex {
    unsafe {
        let base = (t as *mut u8).sub(elemsize);
        let h = base.sub(HDR_SIZE) as *mut ArrHeader;
        (*h).hash_table as *mut HashIndex
    }
}

/// Overwrite the whole bucket array + counters of `t`'s table from `spec`.
unsafe fn impose(t: *mut c_void, elemsize: usize, spec: &TableSpec) {
    unsafe {
        let ti = table_of(t, elemsize);
        assert!(!ti.is_null());
        let sc = (*ti).slot_count;
        for i in 0..(sc >> BUCKET_SHIFT) {
            let bk = (*ti).storage.add(i);
            for j in 0..BUCKET_LEN {
                (*bk).hash[j] = HASH_EMPTY;
                (*bk).index[j] = INDEX_EMPTY;
            }
        }
        for &(slot, hash, index) in &spec.slots {
            assert!(slot < sc);
            let bk = (*ti).storage.add(slot >> BUCKET_SHIFT);
            (*bk).hash[slot & BUCKET_MASK] = hash;
            (*bk).index[slot & BUCKET_MASK] = index;
        }
        (*ti).used_count = spec.used_count;
        (*ti).tombstone_count = spec.tombstone_count;
    }
}

/// Grow a fresh map until `slot_count == want`, returning the map plus the keys
/// in insertion order (key `i` lives at element index `i`).
fn build_map_slot(ka: &mut KeyArena, want: usize) -> (DualMap, Vec<Vec<u8>>) {
    let mut m = DualMap::null(ELEMSIZE, KEYSIZE, KeyRepr::Raw);
    let mut keys = Vec::new();
    let mut n = 0u64;
    while unsafe { m.snap_c() }.slot_count < want {
        n += 1;
        let kb = (n * 0x9E37_79B9_7F4A_7C15u64).to_le_bytes().to_vec();
        let p = ka.add(&kb);
        let idx = unsafe { m.put(p, STBDS_HM_BINARY, n) };
        assert_eq!(idx, keys.len() as isize, "keys must be distinct");
        keys.push(kb);
        assert!(n < 400, "table failed to reach {want} slots");
    }
    assert_eq!(unsafe { m.snap_c() }.slot_count, want);
    (m, keys)
}

fn build_map_slot32(ka: &mut KeyArena) -> (DualMap, Vec<Vec<u8>>) {
    build_map_slot(ka, 32)
}

/// The (adjusted) hash the library will compute for `key`.
unsafe fn key_hash(t: *mut c_void, elemsize: usize, key: *mut c_void, keysize: usize) -> usize {
    unsafe {
        let ti = table_of(t, elemsize);
        let mut h = (common::libs().c.hash_bytes)(key, keysize, (*ti).seed);
        if h < 2 {
            h += 2;
        }
        h
    }
}

/// Fill buckets `b(pos0)` and `b(pos0+8)` completely with non-matching hashes so
/// that a probe starting at `pos0` must hop twice.
fn two_full_buckets(pos0: usize, hk: usize, extra: &mut Vec<(usize, usize, isize)>) -> usize {
    let pos1 = (pos0 + 8) & 31;
    let pos2 = (pos1 + 16) & 31;
    let (b0, b1, b2) = (pos0 >> 3, pos1 >> 3, pos2 >> 3);
    assert!(b0 != b1 && b1 != b2 && b0 != b2, "bucket geometry broken");
    for (bi, tag) in [(b0, 0x100usize), (b1, 0x200usize)] {
        for j in 0..BUCKET_LEN {
            let h = tag + j;
            assert_ne!(h, hk);
            assert_ne!(h, HASH_EMPTY);
            extra.push((bi * BUCKET_LEN + j, h, 0));
        }
    }
    pos2
}

/// `stbds_hm_find_slot` HIT after two bucket hops (`step` growth exercised).
#[test]
fn p01_find_slot_hit_after_two_hops() {
    let _wd = Watchdog::new(30, "p01_find_slot_hit_after_two_hops");
    for trial in 0..8u64 {
        let _g = reset_seeds(0x5000_0000 + trial as usize);
        let mut ka = KeyArena::new();
        let (mut m, keys) = build_map_slot32(&mut ka);

        // the key we will look for lives in element 1 -> bucket index 0
        let kp = ka.add(&keys[0]);
        let hk = unsafe { key_hash(m.tc, ELEMSIZE, kp, KEYSIZE) };
        let hk_r = unsafe { key_hash(m.tr, ELEMSIZE, kp, KEYSIZE) };
        assert_eq!(hk, hk_r, "the two libraries must agree on the hash");

        let pos0 = hk & 31;
        let mut slots: Vec<(usize, usize, isize)> = Vec::new();
        let pos2 = two_full_buckets(pos0, hk, &mut slots);
        // the real entry, reachable only after hop 1 (step 8) + hop 2 (step 16)
        slots.push((pos2, hk, 0));
        let spec = TableSpec {
            slots,
            used_count: 17,
            tombstone_count: 0,
        };
        unsafe {
            impose(m.tc, ELEMSIZE, &spec);
            impose(m.tr, ELEMSIZE, &spec);
        }
        m.check("after imposing the artificial table");

        // hmget_key / hmget_key_ts must find it, after two hops
        let a = unsafe { m.get(kp, STBDS_HM_BINARY) };
        let b = unsafe { m.get_ts(kp, STBDS_HM_BINARY) };
        assert_eq!(a, 0, "the key must be found at element index 0");
        assert_eq!(b, 0);

        // and a *different* key must miss (bucket b2 has 7 empty slots)
        let other = ka.add(&keys[1]);
        let oh = unsafe { key_hash(m.tc, ELEMSIZE, other, KEYSIZE) };
        if (oh & 31) == pos0 {
            // would follow the same chain; skip this trial's miss check
        } else {
            let c = unsafe { m.get(other, STBDS_HM_BINARY) };
            assert_eq!(c, -1);
        }
        unsafe { m.free() };
    }
}

/// `stbds_hm_find_slot` MISS after two bucket hops.
#[test]
fn p02_find_slot_miss_after_two_hops() {
    let _wd = Watchdog::new(30, "p02_find_slot_miss_after_two_hops");
    for trial in 0..8u64 {
        let _g = reset_seeds(0x5100_0000 + trial as usize);
        let mut ka = KeyArena::new();
        let (mut m, keys) = build_map_slot32(&mut ka);
        let kp = ka.add(&keys[0]);
        let hk = unsafe { key_hash(m.tc, ELEMSIZE, kp, KEYSIZE) };
        let pos0 = hk & 31;
        let mut slots: Vec<(usize, usize, isize)> = Vec::new();
        let _pos2 = two_full_buckets(pos0, hk, &mut slots);
        // bucket 2 stays completely empty -> the miss is reported from there
        let spec = TableSpec {
            slots,
            used_count: 16,
            tombstone_count: 0,
        };
        unsafe {
            impose(m.tc, ELEMSIZE, &spec);
            impose(m.tr, ELEMSIZE, &spec);
        }
        m.check("after imposing the artificial table");
        assert_eq!(unsafe { m.get(kp, STBDS_HM_BINARY) }, -1);
        assert_eq!(unsafe { m.get_ts(kp, STBDS_HM_BINARY) }, -1);
        // hmdel_key must also report "not found"
        assert_eq!(unsafe { m.del(kp, STBDS_HM_BINARY) }, 0);
        unsafe { m.free() };
    }
}

/// `stbds_hmput_key` INSERT after two bucket hops (`found_empty_slot` reached
/// from a third bucket).
#[test]
fn p03_hmput_insert_after_two_hops() {
    let _wd = Watchdog::new(30, "p03_hmput_insert_after_two_hops");
    for trial in 0..8u64 {
        let _g = reset_seeds(0x5200_0000 + trial as usize);
        let mut ka = KeyArena::new();
        let (mut m, _keys) = build_map_slot32(&mut ka);

        // a brand-new key
        let nkb = (0xABCD_0000_0000_0000u64 + trial).to_le_bytes().to_vec();
        let np = ka.add(&nkb);
        let hk = unsafe { key_hash(m.tc, ELEMSIZE, np, KEYSIZE) };
        assert_eq!(hk, unsafe { key_hash(m.tr, ELEMSIZE, np, KEYSIZE) });

        let pos0 = hk & 31;
        let mut slots: Vec<(usize, usize, isize)> = Vec::new();
        let pos2 = two_full_buckets(pos0, hk, &mut slots);
        let spec = TableSpec {
            slots,
            used_count: 16, // < used_count_threshold (24) -> no grow
            tombstone_count: 0,
        };
        unsafe {
            impose(m.tc, ELEMSIZE, &spec);
            impose(m.tr, ELEMSIZE, &spec);
        }
        m.check("after imposing the artificial table");

        let idx = unsafe { m.put(np, STBDS_HM_BINARY, 0x1234) };
        // the new entry must sit exactly at pos2 in BOTH libraries
        let sc = unsafe { m.snap_c() };
        assert_eq!(sc.buckets[pos2], (hk, idx), "insert landed on the wrong slot");
        assert_eq!(sc.used_count, 17);
        // and it must be findable
        assert_eq!(unsafe { m.get(np, STBDS_HM_BINARY) }, idx);
        unsafe { m.free() };
    }
}

/// `stbds_hmput_key` INSERT that hops twice and then reuses a TOMBSTONE found
/// back in the first bucket (`tombstone >= 0` + `--tombstone_count`).
#[test]
fn p04_hmput_tombstone_reuse_after_two_hops() {
    let _wd = Watchdog::new(30, "p04_hmput_tombstone_reuse_after_two_hops");
    for trial in 0..8u64 {
        let _g = reset_seeds(0x5300_0000 + trial as usize);
        let mut ka = KeyArena::new();
        let (mut m, _keys) = build_map_slot32(&mut ka);

        let nkb = (0x1357_0000_0000_0000u64 + trial).to_le_bytes().to_vec();
        let np = ka.add(&nkb);
        let hk = unsafe { key_hash(m.tc, ELEMSIZE, np, KEYSIZE) };

        let pos0 = hk & 31;
        let mut slots: Vec<(usize, usize, isize)> = Vec::new();
        let _pos2 = two_full_buckets(pos0, hk, &mut slots);
        // turn one slot of the FIRST bucket into a tombstone; it is still
        // non-empty, so the probe keeps hopping, but it is remembered.
        let tomb_slot = (pos0 & !BUCKET_MASK) + ((pos0 + 3) & BUCKET_MASK);
        for s in slots.iter_mut() {
            if s.0 == tomb_slot {
                s.1 = HASH_DELETED;
                s.2 = INDEX_DELETED;
            }
        }
        let spec = TableSpec {
            slots,
            used_count: 15,
            tombstone_count: 4,
        };
        unsafe {
            impose(m.tc, ELEMSIZE, &spec);
            impose(m.tr, ELEMSIZE, &spec);
        }
        m.check("after imposing the artificial table");

        let idx = unsafe { m.put(np, STBDS_HM_BINARY, 0x9876) };
        let sc = unsafe { m.snap_c() };
        // the C reuses the *first* tombstone it saw (scanning the forward loop
        // then the wrap-around loop of the first bucket)
        assert_eq!(
            sc.buckets[tomb_slot],
            (hk, idx),
            "the tombstone slot should have been reused"
        );
        assert_eq!(sc.tombstone_count, 3, "tombstone_count must be decremented");
        assert_eq!(sc.used_count, 16);
        assert_eq!(unsafe { m.get(np, STBDS_HM_BINARY) }, idx);
        unsafe { m.free() };
    }
}

/// A duplicate key found in the WRAP-AROUND inner loop (`i < limit`) of both
/// `stbds_hm_find_slot` and `stbds_hmput_key`, constructed deterministically.
#[test]
fn p05_duplicate_in_wraparound_loop() {
    let _wd = Watchdog::new(30, "p05_duplicate_in_wraparound_loop");
    for trial in 0..8u64 {
        let _g = reset_seeds(0x5400_0000 + trial as usize);
        let mut ka = KeyArena::new();
        let (mut m, keys) = build_map_slot32(&mut ka);
        let kp = ka.add(&keys[0]);
        let hk = unsafe { key_hash(m.tc, ELEMSIZE, kp, KEYSIZE) };
        let pos0 = hk & 31;
        let lo = pos0 & BUCKET_MASK;
        if lo == 0 {
            // the wrap-around loop has zero iterations for this hash
            unsafe { m.free() };
            continue;
        }
        // Fill the whole bucket (so neither inner loop can bail out on an empty
        // slot) and put the real entry at slot 0 — i.e. the *first* slot the
        // wrap-around loop (`for i = 0; i < limit`) examines.
        let bbase = pos0 & !BUCKET_MASK;
        let mut slots: Vec<(usize, usize, isize)> = Vec::new();
        slots.push((bbase, hk, 0));
        for j in 1..BUCKET_LEN {
            let h = 0x300 + j;
            assert_ne!(h, hk);
            assert_ne!(h, HASH_EMPTY);
            slots.push((bbase + j, h, 0));
        }
        let spec = TableSpec {
            slots,
            used_count: 12,
            tombstone_count: 0,
        };
        unsafe {
            impose(m.tc, ELEMSIZE, &spec);
            impose(m.tr, ELEMSIZE, &spec);
        }
        m.check("after imposing the artificial table");

        // find_slot: hit in the wrap-around loop
        assert_eq!(unsafe { m.get(kp, STBDS_HM_BINARY) }, 0);
        assert_eq!(unsafe { m.get_ts(kp, STBDS_HM_BINARY) }, 0);
        // hmput_key: duplicate hit in the wrap-around loop.  The C does NOT
        // update stbds_temp_key there; the Rust must not either.
        let idx = unsafe { m.put(kp, STBDS_HM_BINARY, 0x5555) };
        assert_eq!(idx, 0, "duplicate put must reuse element index 0");
        let sc = unsafe { m.snap_c() };
        assert_eq!(sc.used_count, 12, "a duplicate must not bump used_count");
        assert_eq!(sc.length, unsafe { m.snap_r() }.length);
        unsafe { m.free() };
    }
}

/// `stbds_make_hash_index`'s own rehash loop has a third copy of the
/// `pos += step; step += 8` walk.  Force TWO hops during a table grow by
/// crafting 17 old entries that all map to the same bucket of the new table.
#[test]
fn p06_rehash_multi_hop_on_grow() {
    let _wd = Watchdog::new(30, "p06_rehash_multi_hop_on_grow");
    for trial in 0..6u64 {
        let _g = reset_seeds(0x5500_0000 + trial as usize);
        let mut ka = KeyArena::new();
        let (mut m, _keys) = build_map_slot32(&mut ka);

        // 17 in-use entries whose hash is ≡ 0 (mod 64), i.e. they all want
        // bucket 0 of the 64-slot table the grow will create.
        let mut slots: Vec<(usize, usize, isize)> = Vec::new();
        for k in 0..17usize {
            let h = 64 * (k + 1); // distinct, non-zero, ≡ 0 (mod 64)
            slots.push((k + 1, h, 0));
        }
        let spec = TableSpec {
            slots,
            // >= used_count_threshold(32) = 24 -> the next put grows to 64
            used_count: 24,
            tombstone_count: 0,
        };
        unsafe {
            impose(m.tc, ELEMSIZE, &spec);
            impose(m.tr, ELEMSIZE, &spec);
        }
        m.check("after imposing the artificial table");

        let nkb = (0x2468_0000_0000_0000u64 + trial).to_le_bytes().to_vec();
        let np = ka.add(&nkb);
        let idx = unsafe { m.put(np, STBDS_HM_BINARY, 0x4242) };

        let sc = unsafe { m.snap_c() };
        assert_eq!(sc.slot_count, 64, "the table must have grown");
        // bucket 0 and bucket 1 must be completely full of rehashed entries and
        // the 17th must have landed in bucket 3 (pos 24) -> two hops
        for j in 0..8usize {
            assert_ne!(sc.buckets[j].0, 0, "bucket 0 slot {j} should be occupied");
            assert_ne!(sc.buckets[8 + j].0, 0, "bucket 1 slot {j} should be occupied");
        }
        assert_ne!(sc.buckets[24].0, 0, "the 17th entry must be at slot 24");
        assert_eq!(unsafe { m.get(np, STBDS_HM_BINARY) }, idx);
        unsafe { m.free() };
    }
}

/// Same rehash walk, but reached through the SHRINK path of `stbds_hmdel_key`
/// (`stbds_make_hash_index(slot_count>>1, table)`): 64 -> 32 slots, with 17
/// crafted entries that all want bucket 0 of the new 32-slot table, so the last
/// one needs two hops.
#[test]
fn p07_rehash_multi_hop_on_shrink() {
    let _wd = Watchdog::new(30, "p07_rehash_multi_hop_on_shrink");
    for trial in 0..6u64 {
        let _g = reset_seeds(0x5600_0000 + trial as usize);
        let mut ka = KeyArena::new();
        let (mut m, keys) = build_map_slot(&mut ka, 64);

        // delete the LAST element so `old_index == final_index` and no
        // relocation re-find happens over the artificial table
        let last_idx = (keys.len() - 1) as isize;
        let kp = ka.add(&keys[keys.len() - 1]);
        let hk = unsafe { key_hash(m.tc, ELEMSIZE, kp, KEYSIZE) };
        let key_slot = hk & 63;

        let mut slots: Vec<(usize, usize, isize)> = vec![(key_slot, hk, last_idx)];
        let mut placed = 0usize;
        let mut slot = 0usize;
        while placed < 17 {
            if slot != key_slot {
                let h = 32 * (placed + 1) * 64; // != 0, ≡ 0 (mod 32)
                assert_ne!(h, hk);
                slots.push((slot, h, 0));
                placed += 1;
            }
            slot += 1;
            assert!(slot < 64);
        }
        let spec = TableSpec {
            slots,
            // after --used_count: 15 < used_count_shrink_threshold(64) = 16
            used_count: 16,
            tombstone_count: 0,
        };
        unsafe {
            impose(m.tc, ELEMSIZE, &spec);
            impose(m.tr, ELEMSIZE, &spec);
        }
        m.check("after imposing the artificial table");

        let d = unsafe { m.del(kp, STBDS_HM_BINARY) };
        assert_eq!(d, 1, "the crafted key must be found");
        let sc = unsafe { m.snap_c() };
        assert_eq!(sc.slot_count, 32, "the table must have shrunk 64 -> 32");
        // buckets 0 and 1 full of rehashed entries, the 17th two hops away
        for j in 0..8usize {
            assert_ne!(sc.buckets[j].0, 0, "bucket 0 slot {j}");
            assert_ne!(sc.buckets[8 + j].0, 0, "bucket 1 slot {j}");
        }
        assert_ne!(sc.buckets[24].0, 0, "the 17th entry must be at slot 24");
        unsafe { m.free() };
    }
}
