//! Phase C — one differential test per row of `ERRORS.md`.

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const SIGSEGV: c_int = 11;
const SIGABRT: c_int = 6;

// ============================================================ row 1
#[test]
fn e01_arrgrowf_no_grow() {
    let (c, r) = pair();
    unsafe {
        // min_len == 0 and min_cap == 0 -> `min_cap <= arrcap(NULL) == 0`
        for es in [1usize, 8, 16, 4096] {
            let a = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let b = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(a.is_null(), "C must return NULL (es={es})");
            assert!(b.is_null(), "RUST must return NULL (es={es})");
        }
        // and on an existing array the *same pointer* comes back
        let a = (c.arrgrowf)(std::ptr::null_mut(), 16, 0, 10);
        let b = (r.arrgrowf)(std::ptr::null_mut(), 16, 0, 10);
        for mc in 0..=10usize {
            assert_eq!((c.arrgrowf)(a, 16, 0, mc), a, "C grew for min_cap={mc}");
            assert_eq!((r.arrgrowf)(b, 16, 0, mc), b, "RUST grew for min_cap={mc}");
        }
        (c.arrfreef)(a);
        (r.arrfreef)(b);
    }
}

// ============================================================ row 2
#[test]
fn e02_arrgrowf_size_overflow() {
    let (c, r) = pair();
    // elemsize*min_cap wraps to exactly 0, so `+ sizeof(header)` is a valid
    // 32-byte allocation and only the header is written.
    for (es, mc) in [
        (16usize, 1usize << 60),
        (8, 1usize << 61),
        (4, 1usize << 62),
        (2, 1usize << 63),
        (1 << 60, 16),
        (1 << 32, 1 << 32),
    ] {
        unsafe {
            let a = (c.arrgrowf)(std::ptr::null_mut(), es, 0, mc);
            let b = (r.arrgrowf)(std::ptr::null_mut(), es, 0, mc);
            assert!(!a.is_null() && !b.is_null(), "es={es} mc={mc}");
            let ha = *header_of_raw(a);
            let hb = *header_of_raw(b);
            assert_eq!(
                (ha.length, ha.capacity, ha.hash_table.is_null(), ha.temp),
                (hb.length, hb.capacity, hb.hash_table.is_null(), hb.temp),
                "header diverged for es={es} mc={mc}"
            );
            assert_eq!(ha.capacity, mc, "capacity must be the (overflowing) min_cap");
            (c.arrfreef)(a);
            (r.arrfreef)(b);
        }
    }
}

// ============================================================ row 3
#[test]
fn e03_hmfree_null() {
    let (c, r) = pair();
    unsafe {
        for es in [0usize, 1, 8, 16, usize::MAX] {
            (c.hmfree_func)(std::ptr::null_mut(), es);
            (r.hmfree_func)(std::ptr::null_mut(), es);
        }
    }
    // reaching here at all is the assertion: both must be a pure no-op
}

// ============================================================ row 4
#[test]
fn e04_hmfree_no_table() {
    let _g = lock();
    let (c, r) = pair();
    let es = 16usize;
    let key = CBuf::new(&le64(1));
    unsafe {
        // hmget_key(NULL, ...) creates a map that has *no* hash table
        let mut t: isize = 0;
        let a = (c.hmget_key_ts)(std::ptr::null_mut(), es, key.as_void(), 8, &mut t, HM_BINARY);
        let b = (r.hmget_key_ts)(std::ptr::null_mut(), es, key.as_void(), 8, &mut t, HM_BINARY);
        assert!((*map_header(a, es)).hash_table.is_null());
        assert!((*map_header(b, es)).hash_table.is_null());
        assert_eq!(snap(a, es, false), snap(b, es, false));
        (c.hmfree_func)(raw_of(a, es), es);
        (r.hmfree_func)(raw_of(b, es), es);

        // hmput_default(NULL, ...) likewise
        let a = (c.hmput_default)(std::ptr::null_mut(), es);
        let b = (r.hmput_default)(std::ptr::null_mut(), es);
        assert_eq!(snap(a, es, false), snap(b, es, false));
        (c.hmfree_func)(raw_of(a, es), es);
        (r.hmfree_func)(raw_of(b, es), es);
    }
}

// ============================================================ rows 5 & 6
/// Replicates `stbds_hm_find_slot`'s probe so the test can tell whether a slot
/// was located in the *upper* (`pos&7 .. 8`) or the *wrapped* (`0 .. pos&7`)
/// half of a bucket — the two distinct `return -1` sites and the two distinct
/// duplicate-hit sites in `stbds_hmput_key`.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Half {
    Upper,
    Wrapped,
}

unsafe fn slot_half(api: &Api, t: *mut c_void, es: usize, key: &[u8], keysize: usize) -> Option<Half> {
    let table = map_table(t, es);
    if table.is_null() {
        return None;
    }
    let kb = CBuf::new(key);
    let mut hash = (api.hash_bytes)(kb.as_void(), keysize, (*table).seed);
    if hash < 2 {
        hash += 2;
    }
    let slot_count = (*table).slot_count;
    let mut pos = hash & (slot_count - 1);
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = &*(*table).storage.add(pos >> BUCKET_SHIFT);
        for i in (pos & BUCKET_MASK)..BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                return Some(Half::Upper);
            } else if bucket.hash[i] == 0 {
                return None;
            }
        }
        for i in 0..(pos & BUCKET_MASK) {
            if bucket.hash[i] == hash {
                return Some(Half::Wrapped);
            } else if bucket.hash[i] == 0 {
                return None;
            }
        }
        pos = pos.wrapping_add(step);
        step += BUCKET_LENGTH;
        pos &= slot_count - 1;
    }
}

#[test]
fn e05_find_slot_miss() {
    let _g = lock();
    sync_seed(0x3141_5926);
    let (c, _r) = pair();
    let es = 16usize;
    let mut rng = Rng::new(0x0506);
    let mut m = Dual::new(es, false);
    for i in 0..400i64 {
        m.put_bin(&le64(i), 8, &le64(i), HM_BINARY);
    }
    m.check("find_slot_miss setup");
    let mut misses = 0usize;
    for _ in 0..3000usize {
        let k = 1000 + (rng.next_u64() % 100_000) as i64;
        let (a, b) = m.get(&le64(k), 8, HM_BINARY, false);
        assert_eq!((a, b), (-1, -1), "miss must report -1 for {k}");
        misses += 1;
        // both `return -1` sites must be reachable; `slot_half == None` covers
        // whichever half held the terminating empty slot
        assert!(unsafe { slot_half(c, m.c, es, &le64(k), 8) }.is_none());
    }
    assert!(misses == 3000);
    m.check("find_slot_miss after misses");
    m.free();
}

// ============================================================ row 7
#[test]
fn e07_hmget_ts_null() {
    let (c, r) = pair();
    for es in [8usize, 16, 24, 32] {
        for mode in [HM_BINARY, HM_STRING, 2, -1] {
            let key = CBuf::cstr(b"anything");
            unsafe {
                let mut ta: isize = 0x1234;
                let mut tb: isize = 0x1234;
                let a =
                    (c.hmget_key_ts)(std::ptr::null_mut(), es, key.as_void(), 8, &mut ta, mode);
                let b =
                    (r.hmget_key_ts)(std::ptr::null_mut(), es, key.as_void(), 8, &mut tb, mode);
                assert_eq!((ta, tb), (-1, -1), "es={es} mode={mode}");
                assert_eq!(snap(a, es, false), snap(b, es, false), "es={es} mode={mode}");
                assert_eq!((*map_header(a, es)).length, 1);
                (c.hmfree_func)(raw_of(a, es), es);
                (r.hmfree_func)(raw_of(b, es), es);
            }
        }
    }
}

// ============================================================ row 8
#[test]
fn e08_hmget_ts_no_table() {
    let (c, r) = pair();
    let es = 16usize;
    let key = CBuf::new(&le64(9));
    unsafe {
        let mut ta: isize = 0;
        let mut tb: isize = 0;
        let a = (c.hmget_key_ts)(std::ptr::null_mut(), es, key.as_void(), 8, &mut ta, HM_BINARY);
        let b = (r.hmget_key_ts)(std::ptr::null_mut(), es, key.as_void(), 8, &mut tb, HM_BINARY);
        // second call: `a != NULL` but `hash_table == NULL`
        for _ in 0..5 {
            ta = 0x77;
            tb = 0x77;
            let a2 = (c.hmget_key_ts)(a, es, key.as_void(), 8, &mut ta, HM_BINARY);
            let b2 = (r.hmget_key_ts)(b, es, key.as_void(), 8, &mut tb, HM_BINARY);
            assert_eq!(a2, a, "C must return the same pointer");
            assert_eq!(b2, b, "RUST must return the same pointer");
            assert_eq!((ta, tb), (-1, -1));
            assert_eq!(snap(a, es, false), snap(b, es, false));
        }
        (c.hmfree_func)(raw_of(a, es), es);
        (r.hmfree_func)(raw_of(b, es), es);
    }
}

// ============================================================ row 9 & 10
#[test]
fn e09_hmget_miss() {
    let _g = lock();
    sync_seed(11);
    let mut m = Dual::new(16, false);
    for i in 0..40i64 {
        m.put_bin(&le64(i), 8, &le64(i), HM_BINARY);
    }
    for k in [-1i64, 40, 41, 1 << 40, i64::MIN, i64::MAX] {
        let (a, b) = m.get_ts(&le64(k), 8, HM_BINARY, false);
        assert_eq!((a, b), (-1, -1), "get_ts miss for {k}");
        let (a, b) = m.get(&le64(k), 8, HM_BINARY, false);
        assert_eq!((a, b), (-1, -1), "get miss for {k}");
        m.check(&format!("miss {k}"));
    }
    m.free();
}

#[test]
fn e10_hmget_key_temp() {
    let _g = lock();
    let (c, r) = pair();
    let es = 16usize;
    let key = CBuf::new(&le64(5));
    unsafe {
        // hmget_key on a NULL map must publish -1 through header->temp
        let a = (c.hmget_key)(std::ptr::null_mut(), es, key.as_void(), 8, HM_BINARY);
        let b = (r.hmget_key)(std::ptr::null_mut(), es, key.as_void(), 8, HM_BINARY);
        assert_eq!(map_temp(a, es), -1);
        assert_eq!(map_temp(b, es), -1);
        assert_eq!(snap(a, es, false), snap(b, es, false));
        // second call: table still NULL
        let a2 = (c.hmget_key)(a, es, key.as_void(), 8, HM_BINARY);
        let b2 = (r.hmget_key)(b, es, key.as_void(), 8, HM_BINARY);
        assert_eq!((a2, b2), (a, b));
        assert_eq!(map_temp(a, es), -1);
        assert_eq!(map_temp(b, es), -1);
        (c.hmfree_func)(raw_of(a, es), es);
        (r.hmfree_func)(raw_of(b, es), es);
    }
}

// ============================================================ rows 11-13
#[test]
fn e11_hmput_default_null() {
    let (c, r) = pair();
    for es in [8usize, 16, 24, 40] {
        unsafe {
            let a = (c.hmput_default)(std::ptr::null_mut(), es);
            let b = (r.hmput_default)(std::ptr::null_mut(), es);
            assert_eq!(snap(a, es, false), snap(b, es, false), "es={es}");
            let h = *map_header(a, es);
            assert_eq!(h.length, 1);
            assert!(h.hash_table.is_null());
            // element 0 (the default) must be zeroed
            let mut z = vec![0u8; es];
            std::ptr::copy_nonoverlapping(map_elem(a, es, -1), z.as_mut_ptr(), es);
            assert_eq!(z, vec![0u8; es], "default element not zeroed");
            (c.hmfree_func)(raw_of(a, es), es);
            (r.hmfree_func)(raw_of(b, es), es);
        }
    }
}

#[test]
fn e12_hmput_default_len0() {
    let (c, r) = pair();
    let es = 16usize;
    unsafe {
        // an array with length == 0, reinterpreted as a map pointer
        let ra = (c.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
        let rb = (r.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
        assert_eq!((*header_of_raw(ra)).length, 0);
        assert_eq!((*header_of_raw(rb)).length, 0);
        // poison element 0 so we can prove hmput_default memsets it
        std::ptr::write_bytes(ra as *mut u8, 0xAB, es);
        std::ptr::write_bytes(rb as *mut u8, 0xAB, es);
        let ta = (ra as *mut u8).add(es) as *mut c_void;
        let tb = (rb as *mut u8).add(es) as *mut c_void;
        let a = (c.hmput_default)(ta, es);
        let b = (r.hmput_default)(tb, es);
        assert_eq!(a, ta, "C must reuse the allocation (min_cap 1 <= cap 4)");
        assert_eq!(b, tb, "RUST must reuse the allocation");
        assert_eq!(snap(a, es, false), snap(b, es, false));
        assert_eq!((*map_header(a, es)).length, 1);
        let mut z = vec![0u8; es];
        std::ptr::copy_nonoverlapping(map_elem(a, es, -1), z.as_mut_ptr(), es);
        assert_eq!(z, vec![0u8; es]);
        (c.hmfree_func)(raw_of(a, es), es);
        (r.hmfree_func)(raw_of(b, es), es);
    }
}

#[test]
fn e13_hmput_default_noop() {
    let (c, r) = pair();
    let es = 16usize;
    unsafe {
        let a = (c.hmput_default)(std::ptr::null_mut(), es);
        let b = (r.hmput_default)(std::ptr::null_mut(), es);
        // element 0 keeps whatever the caller wrote
        std::ptr::write_bytes(map_elem(a, es, -1), 0x5c, es);
        std::ptr::write_bytes(map_elem(b, es, -1), 0x5c, es);
        for _ in 0..4 {
            let a2 = (c.hmput_default)(a, es);
            let b2 = (r.hmput_default)(b, es);
            assert_eq!(a2, a);
            assert_eq!(b2, b);
            assert_eq!(snap(a, es, false), snap(b, es, false));
            let mut z = vec![0u8; es];
            std::ptr::copy_nonoverlapping(map_elem(a, es, -1), z.as_mut_ptr(), es);
            assert_eq!(z, vec![0x5cu8; es], "default element must NOT be re-zeroed");
        }
        (c.hmfree_func)(raw_of(a, es), es);
        (r.hmfree_func)(raw_of(b, es), es);
    }
}

// ============================================================ row 14
#[test]
fn e14_hmput_key_null() {
    let _g = lock();
    for es in [8usize, 16, 32] {
        sync_seed(0x2020);
        let mut m = Dual::new(es, false);
        let key = vec![0x11u8; 8.min(es)];
        let payload = vec![0x22u8; es - key.len()];
        let (a, b) = m.put_bin(&key, key.len(), &payload, HM_BINARY);
        assert_eq!((a, b), (0, 0), "es={es}");
        m.check(&format!("hmput_key on NULL map es={es}"));
        unsafe {
            assert_eq!((*map_header(m.c, es)).length, 2);
            assert!(!(*map_header(m.c, es)).hash_table.is_null());
        }
        m.free();
    }
}

// ============================================================ rows 15 & 16
#[test]
fn e15_hmput_dup() {
    let _g = lock();
    // Duplicate hits must be located in BOTH bucket halves over the course of
    // this workload; the two C branches differ (only the upper-half one updates
    // `stbds_temp_key`).
    let mut upper = 0usize;
    let mut wrapped = 0usize;
    let (c, _r) = pair();
    let es = 16usize;
    for seed in [0usize, 1, 2, 3, 0x3141_5926, 0xfeed_face, usize::MAX] {
        sync_seed(seed);
        let mut rng = Rng::new(0x1516 ^ seed as u64);
        let mut m = Dual::new(es, false);
        let mut keys: Vec<i64> = Vec::new();
        for i in 0..300usize {
            let k = if !keys.is_empty() && rng.below(2) == 0 {
                keys[rng.below(keys.len())]
            } else {
                (rng.next_u64() % 4096) as i64
            };
            let existed = keys.contains(&k);
            let half = if existed {
                unsafe { slot_half(c, m.c, es, &le64(k), 8) }
            } else {
                None
            };
            let (a, b) = m.put_bin(&le64(k), 8, &le64(i as i64), HM_BINARY);
            assert_eq!(a, b, "dup put diverged seed={seed:#x} #{i}");
            if existed {
                assert!(a >= 0, "duplicate must reuse an index");
                match half {
                    Some(Half::Upper) => upper += 1,
                    Some(Half::Wrapped) => wrapped += 1,
                    None => {}
                }
            }
            m.check(&format!("dup put seed={seed:#x} #{i}"));
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        m.free();
    }
    assert!(upper > 0, "upper-half duplicate branch never taken");
    assert!(wrapped > 0, "wrapped-half duplicate branch never taken");
}

#[test]
fn e16_hmput_dup_wrapped() {
    let _g = lock();
    // Same coverage requirement for string maps, where the two branches differ
    // observably: only the upper-half branch assigns `stbds_temp_key`.
    let (c, _r) = pair();
    let es = 16usize;
    let mut upper = 0usize;
    let mut wrapped = 0usize;
    for seed in [0usize, 5, 0x1234, 0x3141_5926] {
        sync_seed(seed);
        let mut rng = Rng::new(0x1600 ^ seed as u64);
        let mut m = Dual::new(es, true);
        m.shmode(SH_STRDUP);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for i in 0..300usize {
            let k = if !keys.is_empty() && rng.below(2) == 0 {
                keys[rng.below(keys.len())].clone()
            } else {
                rng.cbytes_len(1, 5, b'a', b'h')
            };
            let existed = keys.contains(&k);
            let half = if existed {
                unsafe { slot_half_str(c, m.c, es, &k) }
            } else {
                None
            };
            let (a, b) = m.put_str(&k, &le64(i as i64), HM_STRING);
            assert_eq!(a, b, "string dup put diverged seed={seed:#x} #{i}");
            if existed {
                match half {
                    Some(Half::Upper) => upper += 1,
                    Some(Half::Wrapped) => wrapped += 1,
                    None => {}
                }
            }
            m.check(&format!("string dup put seed={seed:#x} #{i}"));
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        m.free();
    }
    assert!(upper > 0, "string upper-half duplicate branch never taken");
    assert!(wrapped > 0, "string wrapped-half duplicate branch never taken");
}

unsafe fn slot_half_str(api: &Api, t: *mut c_void, es: usize, key: &[u8]) -> Option<Half> {
    let table = map_table(t, es);
    if table.is_null() {
        return None;
    }
    let kb = CBuf::cstr(key);
    let mut hash = (api.hash_string)(kb.as_char(), (*table).seed);
    if hash < 2 {
        hash += 2;
    }
    let slot_count = (*table).slot_count;
    let mut pos = hash & (slot_count - 1);
    let mut step = BUCKET_LENGTH;
    loop {
        let bucket = &*(*table).storage.add(pos >> BUCKET_SHIFT);
        for i in (pos & BUCKET_MASK)..BUCKET_LENGTH {
            if bucket.hash[i] == hash {
                return Some(Half::Upper);
            } else if bucket.hash[i] == 0 {
                return None;
            }
        }
        for i in 0..(pos & BUCKET_MASK) {
            if bucket.hash[i] == hash {
                return Some(Half::Wrapped);
            } else if bucket.hash[i] == 0 {
                return None;
            }
        }
        pos = pos.wrapping_add(step);
        step += BUCKET_LENGTH;
        pos &= slot_count - 1;
    }
}

// ============================================================ rows 17, 21, 24, 25, 27, 30
#[test]
fn unreachable_assert_invariants() {
    let _g = lock();
    // ERRORS.md rows 17/21/24/25/27/30 are `STBDS_ASSERT`s guarding internal
    // invariants.  Prove the invariants hold across a heavy randomised
    // workload, i.e. that the aborts are genuinely unreachable through the
    // public API (and identically so in both libraries).
    let es = 16usize;
    let mut min_slot_count = usize::MAX;
    for seed in [0usize, 0x3141_5926, usize::MAX] {
        sync_seed(seed);
        let mut rng = Rng::new(0x1717 ^ seed as u64);
        let mut m = Dual::new(es, false);
        let mut live: Vec<i64> = Vec::new();
        for _ in 0..2500usize {
            if rng.below(100) < 55 {
                let k = (rng.next_u64() % 300) as i64;
                m.put_bin(&le64(k), 8, &le64(k), HM_BINARY);
                if !live.contains(&k) {
                    live.push(k);
                }
            } else if !live.is_empty() {
                let k = live[rng.below(live.len())];
                m.del(&le64(k), 8, 0, HM_BINARY, false);
                live.retain(|&x| x != k);
            }
            unsafe {
                for t in [m.c, m.r] {
                    if t.is_null() {
                        continue;
                    }
                    let h = *map_header(t, es);
                    // row 17: `i+1 <= arrcap(a)` after the growth call
                    assert!(h.length <= h.capacity, "length {} > capacity {}", h.length, h.capacity);
                    let tab = map_table(t, es);
                    if !tab.is_null() {
                        let tt = *tab;
                        // row 25: make_hash_index is never called with slot_count <= 2
                        assert!(tt.slot_count >= BUCKET_LENGTH, "slot_count {}", tt.slot_count);
                        assert!(
                            tt.used_count_threshold + tt.tombstone_count_threshold
                                < tt.slot_count,
                            "row-25 invariant violated"
                        );
                        // row 21: every occupied slot index is < slot_count
                        assert!(tt.used_count <= tt.slot_count);
                        min_slot_count = min_slot_count.min(tt.slot_count);
                    }
                }
            }
        }
        m.free();
    }
    assert_eq!(min_slot_count, BUCKET_LENGTH);
}

// ============================================================ row 22
#[test]
fn e22_dead_assert_absent_from_c_so() {
    // gcc folds `STBDS_ASSERT(table->used_count >= 0)` (a size_t) away, so the
    // assertion text is not in the C .so's .rodata.  The Rust translation
    // correctly omits it too.
    let c_so = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so");
    let bytes = std::fs::read(&c_so).expect("C .so must be built");
    let needle = b"table->used_count >= 0";
    assert!(
        !bytes.windows(needle.len()).any(|w| w == needle),
        "gcc unexpectedly kept the dead assertion"
    );
    // the *live* assertion strings must be present, proving the search works
    for live in [
        &b"slot >= 0"[..],
        &b"b->index[i] == final_index"[..],
        &b"len <= a->remaining"[..],
        &b"slot < (ptrdiff_t) table->slot_count"[..],
        &b"(size_t) i+1 <= stbds_arrcap(a)"[..],
        &b"t->used_count_threshold + t->tombstone_count_threshold < t->slot_count"[..],
        &b"*strmap[0].key == 'a'"[..],
    ] {
        assert!(
            bytes.windows(live.len()).any(|w| w == live),
            "expected assertion string missing from the C .so: {:?}",
            std::str::from_utf8(live).unwrap()
        );
    }
    // and the Rust .so must carry exactly the same set
    let r_so = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/release/libsh_puts_lib.so");
    if let Ok(rbytes) = std::fs::read(&r_so) {
        assert!(!rbytes.windows(needle.len()).any(|w| w == needle));
        for live in [
            &b"slot >= 0"[..],
            &b"b->index[i] == final_index"[..],
            &b"len <= a->remaining"[..],
            &b"slot < (ptrdiff_t) table->slot_count"[..],
            &b"(size_t) i+1 <= stbds_arrcap(a)"[..],
            &b"t->used_count_threshold + t->tombstone_count_threshold < t->slot_count"[..],
            &b"*strmap[0].key == 'a'"[..],
        ] {
            assert!(
                rbytes.windows(live.len()).any(|w| w == live),
                "assertion string missing from the Rust .so: {:?}",
                std::str::from_utf8(live).unwrap()
            );
        }
    }
}

// ============================================================ row 23
#[test]
fn e23_hmdel_keyoffset_abort() {
    let _g = lock();
    let (c, r) = pair();
    let es = 16usize;
    sync_seed(0x3141_5926);
    let mut m = Dual::new(es, false);
    // element 0: payload == key, so `keyoffset = 8` still matches it
    m.put_bin(&le64(100), 8, &le64(100), HM_BINARY);
    // the rest: payload != key, so the post-memmove re-find must MISS
    for i in 1..5i64 {
        m.put_bin(&le64(100 + i), 8, &le64(0x7fff_0000 + i), HM_BINARY);
    }
    m.check("keyoffset-abort setup");
    let key = CBuf::new(&le64(100));
    let (mc, mr) = (m.c, m.r);
    let kp = key.as_void();

    let (oc, ec) = in_child(|| unsafe {
        (c.hmdel_key)(mc, es, kp, 8, 8, HM_BINARY);
    });
    let (or_, er) = in_child(|| unsafe {
        (r.hmdel_key)(mr, es, kp, 8, 8, HM_BINARY);
    });
    assert_eq!(oc, Outcome::Signalled(SIGABRT), "C did not abort; stderr={:?}", String::from_utf8_lossy(&ec));
    assert_eq!(or_, Outcome::Signalled(SIGABRT), "RUST did not abort; stderr={:?}", String::from_utf8_lossy(&er));
    let first = |v: &[u8]| {
        String::from_utf8_lossy(v.split(|&b| b == b'\n').next().unwrap_or(&[])).into_owned()
    };
    assert_eq!(first(&ec), first(&er), "assert messages differ");
    assert!(
        first(&ec).contains("Assertion `slot >= 0' failed."),
        "unexpected assert: {}",
        first(&ec)
    );
    assert!(first(&ec).contains(":846:"), "wrong line: {}", first(&ec));
    assert!(
        first(&ec).contains("stbds_hmdel_key"),
        "wrong function: {}",
        first(&ec)
    );
    m.free();
}

// ============================================================ row 26
#[test]
fn e26_stralloc_block_shift() {
    let (c, r) = pair();
    // `a->block >= 110` makes `512u << (block>>1)` shift by >= 55 bits, so
    // blocksize becomes 0 and *every* string takes the dedicated-block path.
    for block in [110u8, 111, 120, 127, 238, 250, 255] {
        let mut ac = StringArena::new();
        let mut ar = StringArena::new();
        ac.block = block;
        ar.block = block;
        let s = CBuf::cstr(b"tiny");
        unsafe {
            let pc = (c.stralloc)(&mut ac, s.as_char());
            let pr = (r.stralloc)(&mut ar, s.as_char());
            assert_eq!(cstr(pc), Some(b"tiny".to_vec()));
            assert_eq!(cstr(pr), Some(b"tiny".to_vec()));
            assert_eq!(
                (ac.remaining, ac.block, ac.storage.is_null()),
                (ar.remaining, ar.block, ar.storage.is_null()),
                "block={block}"
            );
            assert_eq!(ac.remaining, 0, "blocksize must have collapsed to 0");
            assert_eq!(ac.block, block.wrapping_add(1));
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
        }
    }
}

// ============================================================ row 28
#[test]
fn e28_stralloc_null_storage() {
    let (c, r) = pair();
    let s = CBuf::cstr(b"x");
    let sp = s.as_char();
    let mut ac = StringArena::new();
    ac.remaining = 100; // lies: there is no block
    let mut ar = ac;
    let pc = &mut ac as *mut StringArena;
    let pr = &mut ar as *mut StringArena;
    let (oc, _) = in_child(|| unsafe {
        (c.stralloc)(pc, sp);
    });
    let (or_, _) = in_child(|| unsafe {
        (r.stralloc)(pr, sp);
    });
    assert_eq!(oc, Outcome::Signalled(SIGSEGV), "C outcome {oc:?}");
    assert_fatal_equivalent(&oc, &or_, "stralloc with storage == NULL");
}

// ============================================================ rows 31 & 32
#[test]
fn e31_hash_bytes_null_zero() {
    let (c, r) = pair();
    unsafe {
        for seed in [0usize, 1, 0x3141_5926, usize::MAX, 1 << 63] {
            assert_eq!(
                (c.hash_bytes)(std::ptr::null_mut(), 0, seed),
                (r.hash_bytes)(std::ptr::null_mut(), 0, seed),
                "seed={seed:#x}"
            );
        }
    }
}

#[test]
fn e32_hash_bytes_null_nonzero() {
    let (c, r) = pair();
    for len in [1usize, 8, 4096] {
        let (oc, _) = in_child(|| unsafe {
            (c.hash_bytes)(std::ptr::null_mut(), len, 0);
        });
        let (or_, _) = in_child(|| unsafe {
            (r.hash_bytes)(std::ptr::null_mut(), len, 0);
        });
        assert_eq!(oc, Outcome::Signalled(SIGSEGV), "C len={len} -> {oc:?}");
        assert_fatal_equivalent(&oc, &or_, &format!("hash_bytes(NULL, {len})"));
    }
}

// ============================================================ row 33
#[test]
fn e33_hash_string_null() {
    let (c, r) = pair();
    let (oc, _) = in_child(|| unsafe {
        (c.hash_string)(std::ptr::null_mut(), 0);
    });
    let (or_, _) = in_child(|| unsafe {
        (r.hash_string)(std::ptr::null_mut(), 0);
    });
    assert_eq!(oc, Outcome::Signalled(SIGSEGV), "C -> {oc:?}");
    assert_fatal_equivalent(&oc, &or_, "null-pointer dereference");
}

// ============================================================ row 34
#[test]
fn e34_arrfreef_null() {
    let (c, r) = pair();
    let (oc, _) = in_child(|| unsafe {
        (c.arrfreef)(std::ptr::null_mut());
    });
    let (or_, _) = in_child(|| unsafe {
        (r.arrfreef)(std::ptr::null_mut());
    });
    assert!(
        matches!(oc, Outcome::Signalled(_)),
        "C should have died, got {oc:?}"
    );
    assert_eq!(or_, oc, "RUST -> {or_:?} != C -> {oc:?}");
}

// ============================================================ rows 35 & 36
#[test]
fn e35_stralloc_null_arena() {
    let (c, r) = pair();
    let s = CBuf::cstr(b"abc");
    let sp = s.as_char();
    let (oc, _) = in_child(|| unsafe {
        (c.stralloc)(std::ptr::null_mut(), sp);
    });
    let (or_, _) = in_child(|| unsafe {
        (r.stralloc)(std::ptr::null_mut(), sp);
    });
    assert_eq!(oc, Outcome::Signalled(SIGSEGV), "C -> {oc:?}");
    assert_fatal_equivalent(&oc, &or_, "null-pointer dereference");
}

#[test]
fn e36_strreset_null() {
    let (c, r) = pair();
    let (oc, _) = in_child(|| unsafe {
        (c.strreset)(std::ptr::null_mut());
    });
    let (or_, _) = in_child(|| unsafe {
        (r.strreset)(std::ptr::null_mut());
    });
    assert_eq!(oc, Outcome::Signalled(SIGSEGV), "C -> {oc:?}");
    assert_fatal_equivalent(&oc, &or_, "null-pointer dereference");
}

// ============================================================ row 40
#[test]
fn e40_keysize_zero() {
    let _g = lock();
    sync_seed(0x4040);
    let es = 16usize;
    let mut m = Dual::new(es, false);
    // keysize == 0: the hash is key-independent and memcmp(_, _, 0) == 0, so
    // every subsequent key is treated as a duplicate of the first one.
    for i in 0..20usize {
        let payload: Vec<u8> = (0..es).map(|k| (i * 16 + k) as u8).collect();
        let (a, b) = m.put_bin(&[], 0, &payload, HM_BINARY);
        assert_eq!((a, b), (0, 0), "keysize=0 put #{i} must reuse index 0");
        m.check(&format!("keysize=0 put #{i}"));
        assert_eq!(m.len(), (1, 1), "map must never grow past 1 entry");
    }
    // every lookup hits, whatever the key
    for _ in 0..10 {
        let (a, b) = m.get(&[], 0, HM_BINARY, false);
        assert_eq!((a, b), (0, 0));
    }
    let (a, b) = m.del(&[], 0, 0, HM_BINARY, false);
    assert_eq!((a, b), (1, 1));
    m.check("keysize=0 after delete");
    assert_eq!(m.len(), (0, 0));
    m.free();
}

// ============================================================ row 42
#[test]
fn e42_hmget_ts_null_temp() {
    let (c, r) = pair();
    let key = CBuf::new(&le64(1));
    let kp = key.as_void();
    let (oc, _) = in_child(|| unsafe {
        (c.hmget_key_ts)(std::ptr::null_mut(), 16, kp, 8, std::ptr::null_mut(), HM_BINARY);
    });
    let (or_, _) = in_child(|| unsafe {
        (r.hmget_key_ts)(std::ptr::null_mut(), 16, kp, 8, std::ptr::null_mut(), HM_BINARY);
    });
    assert_eq!(oc, Outcome::Signalled(SIGSEGV), "C -> {oc:?}");
    assert_fatal_equivalent(&oc, &or_, "null-pointer dereference");
}

// ============================================================ rows 18-20
#[test]
fn e18_hmdel_null() {
    let (c, r) = pair();
    let key = CBuf::new(&le64(1));
    unsafe {
        for es in [8usize, 16, 32] {
            for mode in [HM_BINARY, HM_STRING, 2, -1] {
                let a = (c.hmdel_key)(std::ptr::null_mut(), es, key.as_void(), 8, 0, mode);
                let b = (r.hmdel_key)(std::ptr::null_mut(), es, key.as_void(), 8, 0, mode);
                assert!(a.is_null(), "C es={es} mode={mode}");
                assert!(b.is_null(), "RUST es={es} mode={mode}");
            }
        }
    }
}

#[test]
fn e19_hmdel_no_table() {
    let (c, r) = pair();
    let es = 16usize;
    let key = CBuf::new(&le64(1));
    unsafe {
        let a = (c.hmput_default)(std::ptr::null_mut(), es);
        let b = (r.hmput_default)(std::ptr::null_mut(), es);
        (*map_header(a, es)).temp = 0x1234;
        (*map_header(b, es)).temp = 0x1234;
        let a2 = (c.hmdel_key)(a, es, key.as_void(), 8, 0, HM_BINARY);
        let b2 = (r.hmdel_key)(b, es, key.as_void(), 8, 0, HM_BINARY);
        assert_eq!(a2, a, "C must return the map unchanged");
        assert_eq!(b2, b, "RUST must return the map unchanged");
        assert_eq!(map_temp(a, es), 0, "temp must be reset to 0");
        assert_eq!(map_temp(b, es), 0, "temp must be reset to 0");
        assert_eq!(snap(a, es, false), snap(b, es, false));
        (c.hmfree_func)(raw_of(a, es), es);
        (r.hmfree_func)(raw_of(b, es), es);
    }
}

#[test]
fn e20_hmdel_miss() {
    let _g = lock();
    sync_seed(0x1919);
    let es = 16usize;
    let mut m = Dual::new(es, false);
    for i in 0..50i64 {
        m.put_bin(&le64(i), 8, &le64(i), HM_BINARY);
    }
    m.check("hmdel_miss setup");
    for k in [-1i64, 50, 51, i64::MIN, i64::MAX, 1 << 33] {
        let (a, b) = m.del(&le64(k), 8, 0, HM_BINARY, false);
        assert_eq!((a, b), (0, 0), "missing delete of {k} must report 0");
        m.check(&format!("hmdel_miss {k}"));
        assert_eq!(m.len(), (50, 50), "nothing may have been removed");
    }
    m.free();
}
