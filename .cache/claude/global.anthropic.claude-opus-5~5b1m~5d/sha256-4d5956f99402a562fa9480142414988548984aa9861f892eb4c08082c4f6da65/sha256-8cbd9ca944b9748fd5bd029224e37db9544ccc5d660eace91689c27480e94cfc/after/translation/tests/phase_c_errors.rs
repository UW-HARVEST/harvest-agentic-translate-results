//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every rejection the C library performs is a sentinel return (`NULL`, `-1`,
//! an unchanged pointer, `temp = -1`) or a fatal `assert()`.  Both kinds are
//! compared: sentinels by value, aborts by forking and checking that *both*
//! libraries die with `SIGABRT`.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void, CStr, CString};

fn bin_kind(ks: usize) -> KeyKind {
    KeyKind::Bytes { ks, cmp_end: ks + 4 }
}
const STR_ES: usize = 16;
const STR_KS: usize = 8;
const STR_VOFF: usize = 8;
fn str_kind() -> KeyKind {
    KeyKind::StrPtr { off: 0, cmp_end: 12 }
}

fn seed(p: &Pair, s: usize) {
    unsafe {
        (p.c.rand_seed)(s);
        (p.r.rand_seed)(s);
    }
}

unsafe fn hdr(a: *mut c_void) -> ArrayHeader {
    *((a as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader)
}

// ===========================================================================
// Row 1 — arrgrowf early return when min_cap <= capacity
// ===========================================================================

#[test]
fn err_01_arrgrowf_noop() {
    let (_g, p) = libs();
    unsafe {
        for &es in &[1usize, 4, 8, 16] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), es, 0, 4);
            let hc0 = hdr(ac);
            let hr0 = hdr(ar);
            for &(addlen, min_cap) in &[(0usize, 0usize), (0, 1), (0, 4), (1, 0), (4, 0), (2, 3)] {
                let bc = (p.c.arrgrowf)(ac, es, addlen, min_cap);
                let br = (p.r.arrgrowf)(ar, es, addlen, min_cap);
                assert_eq!(bc, ac, "C must return the same pointer (es={es})");
                assert_eq!(br, ar, "Rust must return the same pointer (es={es})");
                assert_eq!(hdr(ac).length, hc0.length);
                assert_eq!(hdr(ac).capacity, hc0.capacity);
                assert_eq!(hdr(ar).length, hr0.length);
                assert_eq!(hdr(ar).capacity, hr0.capacity);
                assert_eq!(arr_snap(bc, 0), arr_snap(br, 0));
            }
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

// ===========================================================================
// Rows 2a / 2b — arrgrowf(NULL, .., 0, 0) returns NULL; min_cap < 4 folds to 4
// ===========================================================================

#[test]
fn err_02_arrgrowf_null_noalloc() {
    let (_g, p) = libs();
    unsafe {
        for &es in &[0usize, 1, 4, 8, 16, 24] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), es, 0, 0);
            assert!(ac.is_null(), "C arrgrowf(NULL,{es},0,0) must return NULL");
            assert!(ar.is_null(), "Rust arrgrowf(NULL,{es},0,0) must return NULL");
        }
    }
}

#[test]
fn err_02_arrgrowf_min4() {
    let (_g, p) = libs();
    unsafe {
        for &es in &[1usize, 4, 8, 16] {
            for &mc in &[1usize, 2, 3] {
                let ac = (p.c.arrgrowf)(std::ptr::null_mut(), es, 0, mc);
                let ar = (p.r.arrgrowf)(std::ptr::null_mut(), es, 0, mc);
                assert_eq!(hdr(ac).capacity, 4, "C capacity for min_cap={mc}");
                assert_eq!(hdr(ar).capacity, 4, "Rust capacity for min_cap={mc}");
                assert_eq!(arr_snap(ac, 0), arr_snap(ar, 0));
                (p.c.arrfreef)(ac);
                (p.r.arrfreef)(ar);
            }
            // addlen alone can also drive min_cap up
            for &al in &[1usize, 2, 3] {
                let ac = (p.c.arrgrowf)(std::ptr::null_mut(), es, al, 0);
                let ar = (p.r.arrgrowf)(std::ptr::null_mut(), es, al, 0);
                assert_eq!(hdr(ac).capacity, 4);
                assert_eq!(arr_snap(ac, 0), arr_snap(ar, 0));
                (p.c.arrfreef)(ac);
                (p.r.arrfreef)(ar);
            }
        }
    }
}

// ===========================================================================
// Row 3 — arrgrowf size-computation overflow
// ===========================================================================

/// `elemsize * min_cap + sizeof(header)` is computed in wrapping `size_t`
/// arithmetic.  `min_cap = 2^60 + 254` with `elemsize = 16` wraps the product
/// to 4064, so the actual allocation is 4096 bytes while `capacity` records the
/// astronomically large request.  Both libraries must wrap identically.
#[test]
fn err_03_arrgrowf_overflow() {
    let (_g, p) = libs();
    unsafe {
        let es = 16usize;
        let min_cap = (1usize << 60) + 254; // 16*min_cap == 4064 (mod 2^64)
        assert_eq!(es.wrapping_mul(min_cap).wrapping_add(32), 4096);
        let ac = (p.c.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
        let ar = (p.r.arrgrowf)(std::ptr::null_mut(), es, 0, min_cap);
        assert!(!ac.is_null() && !ar.is_null());
        assert_eq!(hdr(ac).capacity, min_cap, "C capacity");
        assert_eq!(hdr(ar).capacity, min_cap, "Rust capacity");
        assert_eq!(arr_snap(ac, 0), arr_snap(ar, 0));
        (p.c.arrfreef)(ac);
        (p.r.arrfreef)(ar);

        // and a second wrapping combination
        let es2 = 8usize;
        let mc2 = (1usize << 61) + 100; // 8*mc2 == 800 (mod 2^64)
        assert_eq!(es2.wrapping_mul(mc2).wrapping_add(32), 832);
        let bc = (p.c.arrgrowf)(std::ptr::null_mut(), es2, 0, mc2);
        let br = (p.r.arrgrowf)(std::ptr::null_mut(), es2, 0, mc2);
        assert_eq!(hdr(bc).capacity, mc2);
        assert_eq!(arr_snap(bc, 0), arr_snap(br, 0));
        (p.c.arrfreef)(bc);
        (p.r.arrfreef)(br);
    }
}

// ===========================================================================
// Rows 4-8 — hmget_key / hmget_key_ts rejections
// ===========================================================================

#[test]
fn err_04_hmget_ts_null() {
    let (_g, p) = libs();
    seed(p, 0x3141_5926);
    unsafe {
        for &(es, ks) in &[(8usize, 4usize), (16, 8), (5, 1), (32, 32)] {
            let mut key = vec![0xABu8; ks];
            let kp = key.as_mut_ptr() as *mut c_void;
            for &mode in &[STBDS_HM_BINARY, STBDS_HM_STRING, -1, 7] {
                // string modes would strcmp a NULL key pointer; a == NULL never
                // reaches the compare, so every mode is safe here.
                let mut tc: isize = 12345;
                let mut tr: isize = 12345;
                let ac = (p.c.hmget_key_ts)(std::ptr::null_mut(), es, kp, ks, &mut tc, mode);
                let ar = (p.r.hmget_key_ts)(std::ptr::null_mut(), es, kp, ks, &mut tr, mode);
                assert_eq!((tc, tr), (-1, -1), "temp must be STBDS_INDEX_EMPTY (es={es} mode={mode})");
                let sc = snap(ac, es, KeyKind::Bytes { ks, cmp_end: es });
                let sr = snap(ar, es, KeyKind::Bytes { ks, cmp_end: es });
                assert_eq!(sc, sr, "es={es} mode={mode}");
                assert_eq!(sc.length, 1);
                assert!(sc.table.is_none());
                assert_eq!(sc.elems[0].0.as_deref().unwrap(), &vec![0u8; ks][..], "elem zeroed");
                (p.c.hmfree_func)((ac as *mut u8).sub(es) as *mut c_void, es);
                (p.r.hmfree_func)((ar as *mut u8).sub(es) as *mut c_void, es);
            }
        }
    }
}

#[test]
fn err_05_hmget_ts_no_table() {
    let (_g, p) = libs();
    seed(p, 0x3141_5926);
    let ks = 4usize;
    let es = 8usize;
    unsafe {
        let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
        mc.put_default();
        mr.put_default();
        let mut key = vec![1u8, 2, 3, 4];
        let kp = key.as_mut_ptr() as *mut c_void;
        let before_c = mc.t;
        let before_r = mr.t;
        assert_eq!(mc.geti_ts(kp), -1, "C temp on a table-less map");
        assert_eq!(mr.geti_ts(kp), -1, "Rust temp on a table-less map");
        assert_eq!(mc.t, before_c, "pointer must be returned unchanged");
        assert_eq!(mr.t, before_r);
        assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)));
        mc.free();
        mr.free();
    }
}

#[test]
fn err_06_hmget_ts_miss() {
    let (_g, p) = libs();
    let ks = 4usize;
    let es = 8usize;
    let mut rng = Rng::new(0xE006);
    for &n in &[1usize, 7, 8, 9, 64, 200] {
        seed(p, 0x3141_5926);
        unsafe {
            let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
            let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
            let mut present = std::collections::HashSet::new();
            for i in 0..n {
                let mut k = rng.bytes(ks);
                present.insert(k.clone());
                let kp = k.as_mut_ptr() as *mut c_void;
                let tc = mc.put_key(kp);
                let tr = mr.put_key(kp);
                mc.set_i32(tc, ks, i as i32);
                mr.set_i32(tr, ks, i as i32);
            }
            let mut misses = 0;
            for _ in 0..200 {
                let mut k = rng.bytes(ks);
                if present.contains(&k) {
                    continue;
                }
                misses += 1;
                let kp = k.as_mut_ptr() as *mut c_void;
                let tc = mc.geti_ts(kp);
                let tr = mr.geti_ts(kp);
                assert_eq!(tc, tr, "n={n} miss temp");
                assert_eq!(tc, -1, "a miss must set temp to STBDS_INDEX_EMPTY");
                // hmget_key (non-ts) must record the same -1 in the header
                assert_eq!(mc.geti(kp), -1);
                assert_eq!(mr.geti(kp), -1);
            }
            assert!(misses > 100, "expected plenty of misses, got {misses}");
            assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)));
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn err_07_hmget_key_null() {
    let (_g, p) = libs();
    seed(p, 0x3141_5926);
    unsafe {
        let es = 8usize;
        let ks = 4usize;
        let mut key = vec![9u8; ks];
        let kp = key.as_mut_ptr() as *mut c_void;
        let ac = (p.c.hmget_key)(std::ptr::null_mut(), es, kp, ks, STBDS_HM_BINARY);
        let ar = (p.r.hmget_key)(std::ptr::null_mut(), es, kp, ks, STBDS_HM_BINARY);
        assert_eq!(map_temp(ac, es), -1, "C stbds_temp");
        assert_eq!(map_temp(ar, es), -1, "Rust stbds_temp");
        assert_eq!(snap(ac, es, bin_kind(ks)), snap(ar, es, bin_kind(ks)));
        (p.c.hmfree_func)((ac as *mut u8).sub(es) as *mut c_void, es);
        (p.r.hmfree_func)((ar as *mut u8).sub(es) as *mut c_void, es);
    }
}

// ===========================================================================
// Rows 9-10 — hmput_default
// ===========================================================================

#[test]
fn err_08_hmput_default_noop() {
    let (_g, p) = libs();
    seed(p, 0x3141_5926);
    let (es, ks) = (8usize, 4usize);
    unsafe {
        let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
        mc.put_default();
        mr.put_default();
        mc.set_i32(-1, ks, -42);
        mr.set_i32(-1, ks, -42);
        let tc0 = mc.t;
        let tr0 = mr.t;
        for _ in 0..5 {
            mc.put_default();
            mr.put_default();
            assert_eq!(mc.t, tc0, "no reallocation");
            assert_eq!(mr.t, tr0);
            assert_eq!(mc.get_i32(-1, ks), -42, "default value must survive");
            assert_eq!(mr.get_i32(-1, ks), -42);
            assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)));
        }
        mc.free();
        mr.free();
    }
}

#[test]
fn err_09_hmput_default_null() {
    let (_g, p) = libs();
    seed(p, 0x3141_5926);
    unsafe {
        for &es in &[1usize, 4, 8, 12, 16, 40] {
            let ac = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let ar = (p.r.hmput_default)(std::ptr::null_mut(), es);
            let kind = KeyKind::Bytes { ks: es, cmp_end: es };
            let sc = snap(ac, es, kind);
            assert_eq!(sc, snap(ar, es, kind), "es={es}");
            assert_eq!(sc.length, 1);
            assert!(sc.table.is_none());
            assert_eq!(sc.elems[0].0.as_deref().unwrap(), &vec![0u8; es][..]);
            (p.c.hmfree_func)((ac as *mut u8).sub(es) as *mut c_void, es);
            (p.r.hmfree_func)((ar as *mut u8).sub(es) as *mut c_void, es);
        }
    }
}

// ===========================================================================
// Rows 11-13 — hmdel_key rejections
// ===========================================================================

#[test]
fn err_10_hmdel_null() {
    let (_g, p) = libs();
    unsafe {
        let mut key = vec![1u8, 2, 3, 4];
        let kp = key.as_mut_ptr() as *mut c_void;
        for &mode in &[STBDS_HM_BINARY, STBDS_HM_STRING, -1, 5, c_int::MAX, c_int::MIN] {
            for &(es, ks, ko) in &[(8usize, 4usize, 0usize), (16, 8, 0), (24, 4, 8)] {
                let rc = (p.c.hmdel_key)(std::ptr::null_mut(), es, kp, ks, ko, mode);
                let rr = (p.r.hmdel_key)(std::ptr::null_mut(), es, kp, ks, ko, mode);
                assert!(rc.is_null(), "C hmdel_key(NULL) must return NULL (mode={mode})");
                assert!(rr.is_null(), "Rust hmdel_key(NULL) must return NULL (mode={mode})");
            }
        }
    }
}

#[test]
fn err_11_hmdel_no_table() {
    let (_g, p) = libs();
    seed(p, 0x3141_5926);
    let (es, ks) = (8usize, 4usize);
    unsafe {
        let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
        mc.put_default();
        mr.put_default();
        // poison `temp` so we can see hmdel_key set it to 0
        (*((mc.t as *mut u8).sub(es + HEADER_SIZE) as *mut ArrayHeader)).temp = 777;
        (*((mr.t as *mut u8).sub(es + HEADER_SIZE) as *mut ArrayHeader)).temp = 777;
        let mut key = vec![1u8, 2, 3, 4];
        let kp = key.as_mut_ptr() as *mut c_void;
        let tc0 = mc.t;
        let tr0 = mr.t;
        assert_eq!(mc.del(kp), 0, "C temp must be 0");
        assert_eq!(mr.del(kp), 0, "Rust temp must be 0");
        assert_eq!(mc.t, tc0);
        assert_eq!(mr.t, tr0);
        assert_eq!(mc.len(), 0);
        assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)));
        mc.free();
        mr.free();
    }
}

#[test]
fn err_12_hmdel_miss() {
    let (_g, p) = libs();
    let (es, ks) = (8usize, 4usize);
    let mut rng = Rng::new(0xE012);
    for &n in &[1usize, 8, 9, 100] {
        seed(p, 0x3141_5926);
        unsafe {
            let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
            let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
            let mut present = std::collections::HashSet::new();
            for i in 0..n {
                let mut k = rng.bytes(ks);
                present.insert(k.clone());
                let kp = k.as_mut_ptr() as *mut c_void;
                let tc = mc.put_key(kp);
                let tr = mr.put_key(kp);
                mc.set_i32(tc, ks, i as i32);
                mr.set_i32(tr, ks, i as i32);
            }
            let before = mc.snap(bin_kind(ks));
            let mut tries = 0;
            for _ in 0..100 {
                let mut k = rng.bytes(ks);
                if present.contains(&k) {
                    continue;
                }
                tries += 1;
                let kp = k.as_mut_ptr() as *mut c_void;
                let dc = mc.del(kp);
                let dr = mr.del(kp);
                assert_eq!(dc, dr, "n={n}");
                assert_eq!(dc, 0, "a missed delete must leave temp == 0");
            }
            assert!(tries > 50);
            let after = mc.snap(bin_kind(ks));
            assert_eq!(after.length, before.length, "missed deletes must not shrink");
            let (tb, ta) = (before.table.clone().unwrap(), after.table.clone().unwrap());
            assert_eq!(tb.used_count, ta.used_count);
            assert_eq!(tb.tombstone_count, ta.tombstone_count);
            assert_eq!(tb.buckets, ta.buckets);
            assert_eq!(after, mr.snap(bin_kind(ks)));
            mc.free();
            mr.free();
        }
    }
}

// ===========================================================================
// Row 14 — hmfree_func(NULL)
// ===========================================================================

#[test]
fn err_13_hmfree_null() {
    let (_g, p) = libs();
    unsafe {
        for &es in &[0usize, 1, 8, 16, 1024] {
            assert_eq!(
                outcome_of(|| (p.c.hmfree_func)(std::ptr::null_mut(), es)),
                Outcome::Ok,
                "C hmfree_func(NULL, {es})"
            );
            assert_eq!(
                outcome_of(|| (p.r.hmfree_func)(std::ptr::null_mut(), es)),
                Outcome::Ok,
                "Rust hmfree_func(NULL, {es})"
            );
        }
    }
}

// ===========================================================================
// Row 18 — `STBDS_ASSERT(table->used_count >= 0)` is a no-op on size_t
// ===========================================================================

/// `used_count` is a `size_t`, so `assert(used_count >= 0)` can never fire; a
/// delete performed while `used_count == 0` wraps it to `SIZE_MAX` instead of
/// aborting.  The Rust translation must wrap identically (and must not panic on
/// a debug-mode subtract overflow).
#[test]
fn err_14_hmdel_used_count_wrap() {
    let (_g, p) = libs();
    seed(p, 0x3141_5926);
    let (es, ks) = (8usize, 4usize);
    unsafe {
        let mut mc = Map::new(&p.c, es, ks, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, es, ks, STBDS_HM_BINARY);
        let mut key = vec![7u8, 7, 7, 7];
        let kp = key.as_mut_ptr() as *mut c_void;
        let tc = mc.put_key(kp);
        let tr = mr.put_key(kp);
        mc.set_i32(tc, ks, 1);
        mr.set_i32(tr, ks, 1);
        // force used_count to 0 behind the library's back
        for m in [&mc, &mr] {
            let h = (m.t as *mut u8).sub(es + HEADER_SIZE) as *mut ArrayHeader;
            (*((*h).hash_table as *mut HashIndex)).used_count = 0;
        }
        let dc = mc.del(kp);
        let dr = mr.del(kp);
        assert_eq!(dc, dr, "delete return");
        assert_eq!(dc, 1, "a hit must set temp to 1");
        let sc = mc.snap(bin_kind(ks));
        assert_eq!(sc, mr.snap(bin_kind(ks)), "state after the wrapping delete");
        assert_eq!(
            sc.table.as_ref().unwrap().used_count,
            usize::MAX,
            "used_count must wrap to SIZE_MAX, not abort"
        );
        mc.free();
        mr.free();
    }
}

// ===========================================================================
// Rows 22-25 — stbds_stralloc edge cases
// ===========================================================================

#[test]
fn err_15_stralloc_oversize() {
    let (_g, p) = libs();
    let mut rng = Rng::new(0xE015);
    // (a) oversized string into a completely fresh arena
    for &len in &[513usize, 600, 4096, 100_000] {
        let s = rng.cstring(len);
        unsafe {
            let mut ac = StringArena::zeroed();
            let mut ar = StringArena::zeroed();
            let pc = (p.c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(pc).to_bytes(), s.as_bytes());
            assert_eq!(CStr::from_ptr(pr).to_bytes(), s.as_bytes());
            let sc = arena_snap(&ac);
            assert_eq!(sc, arena_snap(&ar), "fresh-arena oversize len={len}");
            assert_eq!(sc.remaining, 0, "remaining must stay 0 for the oversize path");
            assert_eq!(sc.block, 1, "block is still incremented");
            assert_eq!(sc.block_chain_len, 1);
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
        }
    }
    // (b) oversized string into a non-empty arena: spliced *after* storage
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        let small = CString::new("small").unwrap();
        (p.c.stralloc)(&mut ac, small.as_ptr() as *mut c_char);
        (p.r.stralloc)(&mut ar, small.as_ptr() as *mut c_char);
        assert_eq!(arena_snap(&ac), arena_snap(&ar));
        let before = arena_snap(&ac);
        let big = rng.cstring(5000);
        let pc = (p.c.stralloc)(&mut ac, big.as_ptr() as *mut c_char);
        let pr = (p.r.stralloc)(&mut ar, big.as_ptr() as *mut c_char);
        assert_eq!(CStr::from_ptr(pc).to_bytes(), big.as_bytes());
        assert_eq!(CStr::from_ptr(pr).to_bytes(), big.as_bytes());
        let after = arena_snap(&ac);
        assert_eq!(after, arena_snap(&ar), "non-empty arena oversize");
        assert_eq!(
            after.remaining, before.remaining,
            "the oversize path must not touch `remaining`"
        );
        assert_eq!(after.block, before.block + 1);
        assert_eq!(after.block_chain_len, 2);
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}

#[test]
fn err_16_stralloc_blocksize_max() {
    let (_g, p) = libs();
    let s = CString::new("x").unwrap();
    unsafe {
        for block in [0u8, 1, 2, 19, 20, 21, 22, 23, 24, 40] {
            let mut ac = StringArena { storage: std::ptr::null_mut(), remaining: 0, block, mode: 0 };
            let mut ar = StringArena { storage: std::ptr::null_mut(), remaining: 0, block, mode: 0 };
            let pc = (p.c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(pc).to_bytes(), b"x");
            assert_eq!(CStr::from_ptr(pr).to_bytes(), b"x");
            let sc = arena_snap(&ac);
            assert_eq!(sc, arena_snap(&ar), "block={block}");
            let expected_blocksize = 512usize.wrapping_shl((block >> 1) as u32);
            assert_eq!(
                sc.remaining,
                expected_blocksize - 2,
                "block={block} blocksize={expected_blocksize}"
            );
            assert_eq!(
                sc.block,
                if expected_blocksize < (1 << 20) { block + 1 } else { block },
                "block={block} increment gate"
            );
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
        }
    }
}

/// The whole `unsigned char` range for `a->block`, including the values whose
/// `512 << (block>>1)` shift count exceeds 63 (undefined in C, an `shl` with a
/// masked count in practice) and the `++a->block` wraparound at 255.
#[test]
fn err_17_stralloc_block_255() {
    let (_g, p) = libs();
    let s = CString::new("hi").unwrap();
    unsafe {
        let mut tested = 0usize;
        let mut wrapped_to_zero = 0usize;
        let mut shift_over_63 = 0usize;
        for block in 0u8..=255 {
            // `blocksize = 512 << (block >> 1)`.  For shift counts in 18..=54
            // this is a multi-gigabyte request that `realloc` fails, and the C
            // then dereferences NULL; that identical crash is not observable
            // from a live harness, so those blocks are skipped.  Everything
            // else is covered: the normal sizes, the shifts that wrap the
            // product to 0, and (block >= 128) the shift counts above 63 that
            // are UB in C and get masked to 6 bits by the hardware `shl`.
            let expected = 512usize.wrapping_shl((block >> 1) as u32);
            if expected != 0 && expected > (1 << 26) {
                continue;
            }
            tested += 1;
            if expected == 0 {
                wrapped_to_zero += 1;
            }
            if (block >> 1) > 63 {
                shift_over_63 += 1;
            }
            let mut ac = StringArena { storage: std::ptr::null_mut(), remaining: 0, block, mode: 0 };
            let mut ar = StringArena { storage: std::ptr::null_mut(), remaining: 0, block, mode: 0 };
            let pc = (p.c.stralloc)(&mut ac, s.as_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, s.as_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(pc).to_bytes(), b"hi", "C content block={block}");
            assert_eq!(CStr::from_ptr(pr).to_bytes(), b"hi", "Rust content block={block}");
            assert_eq!(arena_snap(&ac), arena_snap(&ar), "block={block}");
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
        }
        assert!(tested > 80, "only {tested} block values were exercised");
        assert!(wrapped_to_zero > 0, "the blocksize-wraps-to-0 path was never hit");
        assert!(shift_over_63 > 0, "the shift-count > 63 path was never hit");
    }
}

#[test]
fn err_18_stralloc_empty_str() {
    let (_g, p) = libs();
    let empty = CString::new("").unwrap();
    unsafe {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        for i in 0..600 {
            let pc = (p.c.stralloc)(&mut ac, empty.as_ptr() as *mut c_char);
            let pr = (p.r.stralloc)(&mut ar, empty.as_ptr() as *mut c_char);
            assert_eq!(CStr::from_ptr(pc).to_bytes(), b"");
            assert_eq!(CStr::from_ptr(pr).to_bytes(), b"");
            let sc = arena_snap(&ac);
            assert_eq!(sc, arena_snap(&ar), "empty string i={i}");
            if i == 0 {
                assert_eq!(sc.remaining, 511);
                assert_eq!(sc.block, 1);
            }
        }
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
    }
}

#[test]
fn err_19_strreset_empty() {
    let (_g, p) = libs();
    unsafe {
        for _ in 0..3 {
            let mut ac = StringArena::zeroed();
            let mut ar = StringArena::zeroed();
            (p.c.strreset)(&mut ac);
            (p.r.strreset)(&mut ar);
            let want =
                ArenaSnap { remaining: 0, block: 0, mode: 0, has_storage: false, block_chain_len: 0 };
            assert_eq!(arena_snap(&ac), want);
            assert_eq!(arena_snap(&ar), want);
        }
        // strreset must also clear `block` and `mode`
        let mut ac = StringArena { storage: std::ptr::null_mut(), remaining: 99, block: 7, mode: 3 };
        let mut ar = StringArena { storage: std::ptr::null_mut(), remaining: 99, block: 7, mode: 3 };
        (p.c.strreset)(&mut ac);
        (p.r.strreset)(&mut ar);
        assert_eq!(arena_snap(&ac), arena_snap(&ar));
        assert_eq!(arena_snap(&ac).block, 0);
        assert_eq!(arena_snap(&ac).mode, 0);
    }
}

// ===========================================================================
// Rows 27-31 — hashing boundaries
// ===========================================================================

#[test]
fn err_20_hash_bytes_zero_len() {
    let (_g, p) = libs();
    let mut buf = [0xFFu8; 16];
    let ptr = buf.as_mut_ptr() as *mut c_void;
    for &seed_v in &[0usize, 1, 0x3141_5926, usize::MAX, 0xDEAD_BEEF_CAFE_BABE] {
        let hc = unsafe { (p.c.hash_bytes)(ptr, 0, seed_v) };
        let hr = unsafe { (p.r.hash_bytes)(ptr, 0, seed_v) };
        assert_eq!(hc, hr, "hash_bytes(p, 0, {seed_v:#x})");
    }
}

#[test]
fn err_21_hash_bytes_null_zero() {
    let (_g, p) = libs();
    for &seed_v in &[0usize, 1, 0x3141_5926, usize::MAX] {
        let hc = unsafe { (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed_v) };
        let hr = unsafe { (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed_v) };
        assert_eq!(hc, hr, "hash_bytes(NULL, 0, {seed_v:#x})");
        // and it must equal the len==0 result for a valid pointer
        let mut buf = [1u8; 8];
        let hc2 = unsafe { (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, seed_v) };
        assert_eq!(hc, hc2, "len==0 must never read through the pointer");
    }
}

#[test]
fn err_22_hash_string_empty() {
    let (_g, p) = libs();
    let empty = CString::new("").unwrap();
    for &seed_v in &[0usize, 1, 2, 0x3141_5926, usize::MAX, 1 << 63] {
        let hc = unsafe { (p.c.hash_string)(empty.as_ptr() as *mut c_char, seed_v) };
        let hr = unsafe { (p.r.hash_string)(empty.as_ptr() as *mut c_char, seed_v) };
        assert_eq!(hc, hr, "hash_string(\"\", {seed_v:#x})");
    }
}

#[test]
fn err_23_hash_string_high_bytes() {
    let (_g, p) = libs();
    // single high bytes, and long runs of them: the `(unsigned char)` cast in
    // the C must not sign-extend.
    for b in 0x01u8..=0xFF {
        for n in [1usize, 2, 8, 9, 33] {
            let cs = CString::new(vec![b; n]).unwrap();
            for &seed_v in &[0usize, 0x3141_5926, usize::MAX] {
                let hc = unsafe { (p.c.hash_string)(cs.as_ptr() as *mut c_char, seed_v) };
                let hr = unsafe { (p.r.hash_string)(cs.as_ptr() as *mut c_char, seed_v) };
                assert_eq!(hc, hr, "hash_string([{b:#x}; {n}], {seed_v:#x})");
            }
        }
    }
}

#[test]
fn err_24_hash_bytes_sign_extend() {
    let (_g, p) = libs();
    // Exhaustively cover every tail length with the top tail byte's high bit
    // set (the `case 4: data |= (d[3] << 24)` negative-int path) and the main
    // loop's `d[3]`/`d[7]` high bits.
    for len in 1usize..=24 {
        for pattern in 0u8..=7 {
            let mut buf: Vec<u8> = (0..len).map(|i| ((i as u8) << 1) | 1).collect();
            for i in 0..len {
                if (i as u8) % 4 == 3 && (pattern & 1) != 0 {
                    buf[i] |= 0x80;
                }
                if (i as u8) % 8 == 7 && (pattern & 2) != 0 {
                    buf[i] |= 0x80;
                }
                if (pattern & 4) != 0 {
                    buf[i] |= 0x80;
                }
            }
            let ptr = buf.as_mut_ptr() as *mut c_void;
            for &seed_v in &[0usize, 0x3141_5926, usize::MAX] {
                let hc = unsafe { (p.c.hash_bytes)(ptr, len, seed_v) };
                let hr = unsafe { (p.r.hash_bytes)(ptr, len, seed_v) };
                assert_eq!(hc, hr, "len={len} pattern={pattern} seed={seed_v:#x} {buf:02x?}");
            }
        }
    }
    // every possible single tail byte value at every tail position
    for len in 1usize..=7 {
        for v in 0u8..=255 {
            let mut buf = vec![0x11u8; len];
            buf[len - 1] = v;
            let ptr = buf.as_mut_ptr() as *mut c_void;
            let hc = unsafe { (p.c.hash_bytes)(ptr, len, 5) };
            let hr = unsafe { (p.r.hash_bytes)(ptr, len, 5) };
            assert_eq!(hc, hr, "len={len} last={v:#x}");
        }
    }
}

// ===========================================================================
// Rows 32-35 — out-of-range `mode` / `sh_mode` enum values across the FFI
// ===========================================================================

/// Any `mode >= STBDS_HM_STRING` behaves exactly like `STBDS_HM_STRING` for
/// hashing and comparison.  Deletes are excluded because `hmdel_key` compares
/// `mode == STBDS_HM_STRING` (see `err_29`).
#[test]
fn err_25_mode_out_of_range_high() {
    let (_g, p) = libs();
    let mut rng = Rng::new(0xE025);
    for &mode in &[STBDS_HM_STRING, 2, 7, 1000, c_int::MAX] {
        seed(p, 0x3141_5926);
        unsafe {
            let mut mc = Map::shmode(&p.c, STR_ES, STR_KS, mode, STBDS_SH_STRDUP);
            let mut mr = Map::shmode(&p.r, STR_ES, STR_KS, mode, STBDS_SH_STRDUP);
            let mut keys = Vec::new();
            for i in 0..80usize {
                let n = 1 + rng.below(20);
                let k = rng.cstring(n);
                if keys.iter().any(|x: &CString| *x == k) {
                    continue;
                }
                let kp = k.as_ptr() as *mut c_void;
                let tc = mc.put_key(kp);
                let tr = mr.put_key(kp);
                assert_eq!(tc, tr, "mode={mode} put i={i}");
                mc.set_i32(tc, STR_VOFF, i as i32);
                mr.set_i32(tr, STR_VOFF, i as i32);
                assert_eq!(mc.snap(str_kind()), mr.snap(str_kind()), "mode={mode} i={i}");
                keys.push(k);
            }
            for (i, k) in keys.iter().enumerate() {
                let kp = k.as_ptr() as *mut c_void;
                let ic = mc.geti(kp);
                let ir = mr.geti(kp);
                assert_eq!(ic, ir, "mode={mode} geti i={i}");
                assert!(ic >= 0, "mode={mode} must find the key via strcmp");
            }
            // misses
            for _ in 0..20 {
                let n = 24 + rng.below(8);
                let k = rng.cstring(n);
                let kp = k.as_ptr() as *mut c_void;
                assert_eq!(mc.geti(kp), mr.geti(kp));
            }
            assert_eq!(mc.snap(str_kind()), mr.snap(str_kind()));
            mc.free();
            mr.free();
        }
    }
}

/// Any `mode < STBDS_HM_STRING` — including negatives, which no enumerator has
/// — behaves exactly like `STBDS_HM_BINARY`.
#[test]
fn err_26_mode_out_of_range_low() {
    let (_g, p) = libs();
    let ks = 4usize;
    let es = 8usize;
    let mut rng = Rng::new(0xE026);
    for &mode in &[STBDS_HM_BINARY, -1, -7, c_int::MIN] {
        seed(p, 0x3141_5926);
        unsafe {
            let mut mc = Map::new(&p.c, es, ks, mode);
            let mut mr = Map::new(&p.r, es, ks, mode);
            let mut keys = Vec::new();
            for i in 0..100usize {
                let mut k = rng.bytes(ks);
                let kp = k.as_mut_ptr() as *mut c_void;
                let tc = mc.put_key(kp);
                let tr = mr.put_key(kp);
                assert_eq!(tc, tr, "mode={mode} put i={i}");
                mc.set_i32(tc, ks, i as i32);
                mr.set_i32(tr, ks, i as i32);
                assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)), "mode={mode} i={i}");
                keys.push(k);
            }
            let sc = mc.snap(bin_kind(ks));
            assert_eq!(
                sc.table.as_ref().unwrap().arena_mode,
                0,
                "mode={mode} must select STBDS_SH_NONE"
            );
            for (i, k) in keys.iter().enumerate() {
                let mut kb = k.clone();
                let kp = kb.as_mut_ptr() as *mut c_void;
                assert_eq!(mc.geti(kp), mr.geti(kp), "mode={mode} geti i={i}");
                assert_eq!(mc.del(kp), mr.del(kp), "mode={mode} del i={i}");
                assert_eq!(mc.snap(bin_kind(ks)), mr.snap(bin_kind(ks)), "mode={mode} del i={i}");
            }
            mc.free();
            mr.free();
        }
    }
}

/// `hmput_key` on a fresh map picks `nt->string.mode` from the `mode` argument:
/// `STBDS_SH_DEFAULT` for `mode >= STBDS_HM_STRING`, `0` otherwise.
#[test]
fn err_27_mode_selects_string_mode() {
    let (_g, p) = libs();
    let long_key = CString::new("this-key-is-definitely-longer-than-eight-bytes").unwrap();
    for &mode in &[c_int::MIN, -5, -1, 0, 1, 2, 1000, c_int::MAX] {
        seed(p, 0x3141_5926);
        unsafe {
            let kp = long_key.as_ptr() as *mut c_void;
            let ac = (p.c.hmput_key)(std::ptr::null_mut(), STR_ES, kp, STR_KS, mode);
            let ar = (p.r.hmput_key)(std::ptr::null_mut(), STR_ES, kp, STR_KS, mode);
            let expect = if mode >= STBDS_HM_STRING { 1u8 } else { 0u8 };
            let kind = if expect == 1 { str_kind() } else { KeyKind::Bytes { ks: 8, cmp_end: 12 } };
            set_elem_i32(ac, STR_ES, 0, STR_VOFF, 55);
            set_elem_i32(ar, STR_ES, 0, STR_VOFF, 55);
            let sc = snap(ac, STR_ES, kind);
            assert_eq!(sc, snap(ar, STR_ES, kind), "mode={mode}");
            assert_eq!(
                sc.table.as_ref().unwrap().arena_mode,
                expect,
                "mode={mode} string.mode"
            );
            (p.c.hmfree_func)((ac as *mut u8).sub(STR_ES) as *mut c_void, STR_ES);
            (p.r.hmfree_func)((ar as *mut u8).sub(STR_ES) as *mut c_void, STR_ES);
        }
    }
}

/// `stbds_shmode_func` stores `(unsigned char) mode` with no validation, so any
/// `int` is accepted and truncated.  Values that land outside
/// `{DEFAULT, STRDUP, ARENA}` make `hmput_key`'s `switch` take `default:` and
/// `memcpy` `keysize` raw bytes.
#[test]
fn err_28_shmode_out_of_range() {
    let (_g, p) = libs();
    let long_key = CString::new("another-key-longer-than-eight-bytes-for-memcpy").unwrap();
    for &mode in &[4i32, 5, 99, 255, 256, 260, -1, -256, c_int::MAX, c_int::MIN] {
        seed(p, 0x3141_5926);
        let truncated = (mode as u32 & 0xff) as u8;
        // stay away from the three modes that treat the key as a string
        if matches!(truncated, 1 | 2 | 3) {
            continue;
        }
        unsafe {
            let tc = (p.c.shmode_func)(STR_ES, mode);
            let tr = (p.r.shmode_func)(STR_ES, mode);
            let kind = KeyKind::Bytes { ks: STR_KS, cmp_end: 12 };
            let s0 = snap(tc, STR_ES, kind);
            assert_eq!(s0, snap(tr, STR_ES, kind), "shmode_func({mode}) initial state");
            assert_eq!(
                s0.table.as_ref().unwrap().arena_mode, truncated,
                "shmode_func({mode}) must store (unsigned char) mode"
            );

            // one insert exercising the `default: memcpy` branch
            let mut mc = Map { api: &p.c, t: tc, elemsize: STR_ES, keysize: STR_KS, keyoffset: 0, mode: STBDS_HM_BINARY };
            let mut mr = Map { api: &p.r, t: tr, elemsize: STR_ES, keysize: STR_KS, keyoffset: 0, mode: STBDS_HM_BINARY };
            let kp = long_key.as_ptr() as *mut c_void;
            let ic = mc.put_key(kp);
            let ir = mr.put_key(kp);
            assert_eq!(ic, ir, "shmode_func({mode}) put");
            mc.set_i32(ic, STR_VOFF, 9);
            mr.set_i32(ir, STR_VOFF, 9);
            let s1 = mc.snap(kind);
            assert_eq!(s1, mr.snap(kind), "shmode_func({mode}) after put");
            assert_eq!(
                s1.elems[1].0.as_deref().unwrap(),
                &long_key.as_bytes()[..STR_KS],
                "default: branch must memcpy the raw key bytes"
            );
            mc.free();
            mr.free();
        }
    }
}

/// `stbds_hmdel_key` uses `mode == STBDS_HM_STRING` (not `>=`) to decide how to
/// re-find the element that was moved into the freed slot.  With `mode == 2`
/// (which *is* "string" for hashing) it therefore hashes the *address* of the
/// key pointer, fails to find the slot, and trips
/// `STBDS_ASSERT(slot >= 0)` — provided the deleted element is not the last one.
/// Both libraries must abort.
#[test]
fn err_29_hmdel_mode_eq_vs_ge() {
    let (_g, p) = libs();
    let keys: Vec<CString> = (0..4)
        .map(|i| CString::new(format!("mode-eq-key-{i}")).unwrap())
        .collect();

    fn build_with<'a>(api: &'a Api, keys: &[CString]) -> Map<'a> {
        unsafe {
            let mut m = Map::shmode(api, STR_ES, STR_KS, 2, STBDS_SH_DEFAULT);
            for (i, k) in keys.iter().enumerate() {
                let idx = m.put_key(k.as_ptr() as *mut c_void);
                m.set_i32(idx, STR_VOFF, i as i32);
            }
            m
        }
    }

    unsafe {
        // sanity: the first inserted key sits at index 0 (not the last), and a
        // `mode == STBDS_HM_STRING` delete of it works fine.
        seed(p, 0x3141_5926);
        let mut ok_c = build_with(&p.c, &keys);
        let mut ok_r = build_with(&p.r, &keys);
        assert_eq!(ok_c.geti(keys[0].as_ptr() as *mut c_void), 0);
        assert_eq!(ok_r.geti(keys[0].as_ptr() as *mut c_void), 0);
        ok_c.mode = STBDS_HM_STRING;
        ok_r.mode = STBDS_HM_STRING;
        assert_eq!(ok_c.del(keys[0].as_ptr() as *mut c_void), 1);
        assert_eq!(ok_r.del(keys[0].as_ptr() as *mut c_void), 1);
        assert_eq!(ok_c.snap(str_kind()), ok_r.snap(str_kind()));
        ok_c.free();
        ok_r.free();

        // now the same delete with mode == 2 must abort in both libraries
        for &mode in &[2i32, 5, 1000] {
            seed(p, 0x3141_5926);
            let oc = outcome_of(|| {
                let mut m = build_with(&p.c, &keys);
                m.mode = mode;
                m.del(keys[0].as_ptr() as *mut c_void);
            });
            seed(p, 0x3141_5926);
            let or = outcome_of(|| {
                let mut m = build_with(&p.r, &keys);
                m.mode = mode;
                m.del(keys[0].as_ptr() as *mut c_void);
            });
            assert_eq!(oc, Outcome::Signal(SIGABRT), "C must abort for mode={mode}");
            assert_eq!(or, Outcome::Signal(SIGABRT), "Rust must abort for mode={mode}");
        }

        // ... but deleting the *last* element skips the re-find, so mode == 2
        // is harmless there and must behave identically in both libraries.
        for &mode in &[2i32, 5, 1000] {
            seed(p, 0x3141_5926);
            let mut mc = build_with(&p.c, &keys);
            let mut mr = build_with(&p.r, &keys);
            mc.mode = mode;
            mr.mode = mode;
            let last = keys.len() - 1;
            assert_eq!(mc.geti(keys[last].as_ptr() as *mut c_void), last as isize);
            let dc = mc.del(keys[last].as_ptr() as *mut c_void);
            let dr = mr.del(keys[last].as_ptr() as *mut c_void);
            assert_eq!(dc, dr, "mode={mode} last-element delete");
            assert_eq!(mc.snap(str_kind()), mr.snap(str_kind()), "mode={mode}");
            mc.free();
            mr.free();
        }
    }
}

// ===========================================================================
// Rows 37-38 — duplicate insert paths (main loop vs. wrap-around loop)
// ===========================================================================

#[test]
fn err_30_hmput_duplicate() {
    let (_g, p) = libs();
    seed(p, 0x3141_5926);
    let keys: Vec<CString> =
        (0..50).map(|i| CString::new(format!("dup-{i}")).unwrap()).collect();
    unsafe {
        let mut mc = Map::shmode(&p.c, STR_ES, STR_KS, STBDS_HM_STRING, STBDS_SH_STRDUP);
        let mut mr = Map::shmode(&p.r, STR_ES, STR_KS, STBDS_HM_STRING, STBDS_SH_STRDUP);
        let mut first = Vec::new();
        for (i, k) in keys.iter().enumerate() {
            let tc = mc.put_key(k.as_ptr() as *mut c_void);
            let tr = mr.put_key(k.as_ptr() as *mut c_void);
            assert_eq!(tc, tr);
            mc.set_i32(tc, STR_VOFF, i as i32);
            mr.set_i32(tr, STR_VOFF, i as i32);
            first.push(tc);
        }
        let len_before = mc.len();
        for (i, k) in keys.iter().enumerate() {
            let tc = mc.put_key(k.as_ptr() as *mut c_void);
            let tr = mr.put_key(k.as_ptr() as *mut c_void);
            assert_eq!(tc, tr, "duplicate put i={i}");
            assert_eq!(tc, first[i], "duplicate put must return the existing index");
            assert_eq!(
                map_temp_key(mc.t, STR_ES),
                map_temp_key(mr.t, STR_ES),
                "temp_key after duplicate put i={i}"
            );
            assert_eq!(mc.snap(str_kind()), mr.snap(str_kind()), "i={i}");
        }
        assert_eq!(mc.len(), len_before, "duplicate puts must not change the length");
        assert_eq!(mr.len(), len_before);
        mc.free();
        mr.free();
    }
}

/// The duplicate-key branch inside `hmput_key`'s *wrap-around* loop
/// (`for (i = 0; i < limit; ++i)`) does **not** update `stbds_temp_key`, unlike
/// the first loop.  This test locates keys that actually take that branch and
/// asserts both libraries agree on the resulting (stale) `temp_key`.
#[test]
fn err_31_hmput_dup_wrap_loop() {
    let (_g, p) = libs();
    let mut wrap_hits = 0usize;
    let mut rng = Rng::new(0xE031);
    for round in 0..6 {
        seed(p, 0x3141_5926 + round);
        let keys: Vec<CString> = (0..300)
            .map(|_| {
                let n = 4 + rng.below(12);
                rng.cstring(n)
            })
            .collect();
        unsafe {
            let mut mc = Map::shmode(&p.c, STR_ES, STR_KS, STBDS_HM_STRING, STBDS_SH_DEFAULT);
            let mut mr = Map::shmode(&p.r, STR_ES, STR_KS, STBDS_HM_STRING, STBDS_SH_DEFAULT);
            for (i, k) in keys.iter().enumerate() {
                let tc = mc.put_key(k.as_ptr() as *mut c_void);
                let tr = mr.put_key(k.as_ptr() as *mut c_void);
                assert_eq!(tc, tr);
                mc.set_i32(tc, STR_VOFF, i as i32);
                mr.set_i32(tr, STR_VOFF, i as i32);
            }
            let table_seed = mc.snap(str_kind()).table.unwrap().seed;
            let slot_count = mc.snap(str_kind()).table.unwrap().slot_count;
            for k in keys.iter() {
                // Re-derive where this key's probe starts and where it landed,
                // exactly like stbds_hm_find_slot does.
                let mut h = (p.c.hash_string)(k.as_ptr() as *mut c_char, table_seed);
                if h < 2 {
                    h += 2;
                }
                let pos = h & (slot_count - 1);
                let buckets = mc.snap(str_kind()).table.unwrap().buckets;
                // find the slot holding this key within the first probed bucket
                let base = pos & !BUCKET_MASK;
                let mut found: Option<usize> = None;
                for i in 0..BUCKET_LENGTH {
                    let (bh, bi) = buckets[base + i];
                    if bh == h && bi >= 0 {
                        let e = (mc.t as *mut u8).offset(bi * STR_ES as isize);
                        let kp = *(e as *const *const c_char);
                        if CStr::from_ptr(kp).to_bytes() == k.as_bytes() {
                            found = Some(i);
                            break;
                        }
                    }
                }
                let takes_wrap_loop = matches!(found, Some(i) if i < (pos & BUCKET_MASK));
                let before_c = map_temp_key(mc.t, STR_ES);
                let tc = mc.put_key(k.as_ptr() as *mut c_void);
                let tr = mr.put_key(k.as_ptr() as *mut c_void);
                assert_eq!(tc, tr);
                let after_c = map_temp_key(mc.t, STR_ES);
                let after_r = map_temp_key(mr.t, STR_ES);
                assert_eq!(after_c, after_r, "temp_key must match (wrap={takes_wrap_loop})");
                if takes_wrap_loop {
                    wrap_hits += 1;
                    assert_eq!(
                        after_c, before_c,
                        "the wrap-around duplicate branch must leave temp_key untouched"
                    );
                }
            }
            assert_eq!(mc.snap(str_kind()), mr.snap(str_kind()));
            mc.free();
            mr.free();
        }
    }
    assert!(wrap_hits > 0, "the wrap-around duplicate branch was never exercised");
}

// ===========================================================================
// Rows 39-40 — sh_geti / strkey boundaries
// ===========================================================================

#[test]
fn err_32_sh_geti_nonpositive() {
    let (_g, p) = libs();
    for num in [0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        unsafe {
            (p.c.rand_seed)(0x3141_5926);
            let oc = capture_stdout("c", || (p.c.sh_geti)(num));
            (p.r.rand_seed)(0x3141_5926);
            let or = capture_stdout("r", || (p.r.sh_geti)(num));
            assert_eq!(oc, or, "sh_geti({num}) stdout");
            assert!(oc.is_empty(), "sh_geti({num}) must print nothing, got {oc:?}");
        }
    }
}

#[test]
fn err_33_strkey_extremes() {
    let (_g, p) = libs();
    for n in [0i32, -1, 1, i32::MIN, i32::MAX, i32::MIN + 1, -999_999_999, 999_999_999] {
        unsafe {
            let pc = (p.c.strkey)(n);
            let pr = (p.r.strkey)(n);
            assert_eq!(CStr::from_ptr(pc).to_bytes(), CStr::from_ptr(pr).to_bytes(), "strkey({n})");
            assert_eq!(CStr::from_ptr(pc).to_str().unwrap(), format!("test_{n}"));
        }
    }
}

// ===========================================================================
// Rows 42-43 — hmfree_func variants
// ===========================================================================

#[test]
fn err_34_hmfree_no_table() {
    let (_g, p) = libs();
    seed(p, 0x3141_5926);
    unsafe {
        for &es in &[1usize, 4, 8, 16] {
            let ac = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let ar = (p.r.hmput_default)(std::ptr::null_mut(), es);
            let raw_c = (ac as *mut u8).sub(es) as *mut c_void;
            let raw_r = (ar as *mut u8).sub(es) as *mut c_void;
            assert!((*((raw_c as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader))
                .hash_table
                .is_null());
            assert_eq!(outcome_of(|| (p.c.hmfree_func)(raw_c, es)), Outcome::Ok, "C es={es}");
            assert_eq!(outcome_of(|| (p.r.hmfree_func)(raw_r, es)), Outcome::Ok, "Rust es={es}");
            // free for real in this process too
            (p.c.hmfree_func)(raw_c, es);
            (p.r.hmfree_func)(raw_r, es);
        }
    }
}

#[test]
fn err_35_hmfree_strdup_empty() {
    let (_g, p) = libs();
    for &sh in &[STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        seed(p, 0x3141_5926);
        unsafe {
            let tc = (p.c.shmode_func)(STR_ES, sh);
            let tr = (p.r.shmode_func)(STR_ES, sh);
            let kind = KeyKind::Bytes { ks: STR_KS, cmp_end: 12 };
            // length is 1: only the default element, so the strdup-free loop
            // `for (i = 1; i < 1; ++i)` runs zero times.
            let s = snap(tc, STR_ES, kind);
            assert_eq!(s.length, 1);
            assert_eq!(s.table.as_ref().unwrap().arena_mode, sh as u8);
            let raw_c = (tc as *mut u8).sub(STR_ES) as *mut c_void;
            let raw_r = (tr as *mut u8).sub(STR_ES) as *mut c_void;
            assert_eq!(outcome_of(|| (p.c.hmfree_func)(raw_c, STR_ES)), Outcome::Ok, "C sh={sh}");
            assert_eq!(
                outcome_of(|| (p.r.hmfree_func)(raw_r, STR_ES)),
                Outcome::Ok,
                "Rust sh={sh}"
            );
            (p.c.hmfree_func)(raw_c, STR_ES);
            (p.r.hmfree_func)(raw_r, STR_ES);
        }
    }
}

// ===========================================================================
// Extra generic boundaries: zero elemsize / zero keysize
// ===========================================================================

#[test]
fn err_36_arrgrowf_elemsize_zero() {
    let (_g, p) = libs();
    unsafe {
        for &(addlen, min_cap) in &[(0usize, 1usize), (1, 0), (0, 4), (0, 1 << 40), (7, 3)] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            assert_eq!(ac.is_null(), ar.is_null());
            if ac.is_null() {
                continue;
            }
            assert_eq!(arr_snap(ac, 0), arr_snap(ar, 0), "elemsize=0 addlen={addlen} min_cap={min_cap}");
            (p.c.arrfreef)(ac);
            (p.r.arrfreef)(ar);
        }
    }
}

/// `keysize == 0` makes `memcmp(a, b, 0)` always compare equal and
/// `stbds_hash_bytes(key, 0, seed)` return one fixed hash, so the map collapses
/// to a single entry no matter what keys are supplied.
#[test]
fn err_37_keysize_zero_binary() {
    let (_g, p) = libs();
    seed(p, 0x3141_5926);
    let es = 4usize;
    let mut rng = Rng::new(0xE037);
    unsafe {
        let mut mc = Map::new(&p.c, es, 0, STBDS_HM_BINARY);
        let mut mr = Map::new(&p.r, es, 0, STBDS_HM_BINARY);
        for i in 0..50usize {
            let mut k = rng.bytes(8);
            let kp = k.as_mut_ptr() as *mut c_void;
            let tc = mc.put_key(kp);
            let tr = mr.put_key(kp);
            assert_eq!(tc, tr, "keysize=0 put i={i}");
            mc.set_i32(tc, 0, i as i32);
            mr.set_i32(tr, 0, i as i32);
            let kind = KeyKind::Bytes { ks: 0, cmp_end: 4 };
            assert_eq!(mc.snap(kind), mr.snap(kind), "keysize=0 i={i}");
        }
        assert_eq!(mc.len(), 1, "keysize=0 collapses every key into one entry");
        assert_eq!(mr.len(), 1);
        let mut k = rng.bytes(8);
        let kp = k.as_mut_ptr() as *mut c_void;
        assert_eq!(mc.geti(kp), mr.geti(kp));
        assert_eq!(mc.del(kp), mr.del(kp));
        let kind = KeyKind::Bytes { ks: 0, cmp_end: 4 };
        assert_eq!(mc.snap(kind), mr.snap(kind));
        mc.free();
        mr.free();
    }
}

#[test]
fn err_38_shmode_elemsize_zero() {
    let (_g, p) = libs();
    for &sh in &[STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        seed(p, 0x3141_5926);
        unsafe {
            let tc = (p.c.shmode_func)(0, sh);
            let tr = (p.r.shmode_func)(0, sh);
            let hc = *((tc as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader);
            let hr = *((tr as *mut u8).sub(HEADER_SIZE) as *mut ArrayHeader);
            assert_eq!((hc.length, hc.capacity, hc.temp), (hr.length, hr.capacity, hr.temp));
            let ti_c = &*(hc.hash_table as *mut HashIndex);
            let ti_r = &*(hr.hash_table as *mut HashIndex);
            assert_eq!(ti_c.slot_count, ti_r.slot_count);
            assert_eq!(ti_c.seed, ti_r.seed);
            assert_eq!(ti_c.string.mode, ti_r.string.mode);
            assert_eq!(ti_c.string.mode, sh as u8);
            assert_eq!(outcome_of(|| (p.c.hmfree_func)(tc, 0)), Outcome::Ok, "C sh={sh}");
            assert_eq!(outcome_of(|| (p.r.hmfree_func)(tr, 0)), Outcome::Ok, "Rust sh={sh}");
            (p.c.hmfree_func)(tc, 0);
            (p.r.hmfree_func)(tr, 0);
        }
    }
}
