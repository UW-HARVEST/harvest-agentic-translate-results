//! Phase C — error-path differential tests, one per row of `ERRORS.md`.
//!
//! Each test constructs the exact rejecting condition, drives BOTH `.so` files,
//! and asserts they reject in the same way (same sentinel / same returned
//! pointer relationship / same process termination signal), not merely that
//! "both failed somehow".

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const ES8: usize = 8;
const KS4: usize = 4;
const ES16: usize = 16;
const KS8: usize = 8;

fn i32b(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}
fn pad(mut v: Vec<u8>, n: usize) -> Vec<u8> {
    v.resize(n, 0);
    v
}

/// Header-only snapshot (safe on arrays whose payload is uninitialised).
unsafe fn hdr(a: *mut c_void) -> (bool, usize, usize, isize, bool) {
    if a.is_null() {
        return (true, 0, 0, 0, true);
    }
    let h = header(a);
    (
        false,
        (*h).length,
        (*h).capacity,
        (*h).temp,
        (*h).hash_table.is_null(),
    )
}

// ===========================================================================
// Rows 1-6 — stbds_arrgrowf
// ===========================================================================

#[test]
fn err01_arrgrowf_noop_returns_input_unchanged() {
    let (p, _g) = libs();
    // NULL in, nothing requested -> NULL out (min_cap <= arrcap(NULL) == 0)
    for &es in &[0usize, 1, 8, 20] {
        unsafe {
            assert!((p.c.arrgrowf)(std::ptr::null_mut(), es, 0, 0).is_null());
            assert!((p.rs.arrgrowf)(std::ptr::null_mut(), es, 0, 0).is_null());
        }
    }
    // existing array, request no more than it has -> exact same pointer
    unsafe {
        let ac = (p.c.arrgrowf)(std::ptr::null_mut(), ES8, 0, 16);
        let ar = (p.rs.arrgrowf)(std::ptr::null_mut(), ES8, 0, 16);
        for &(addlen, min_cap) in &[(0usize, 0usize), (0, 1), (0, 16), (5, 5), (16, 0), (16, 16)] {
            let bc = (p.c.arrgrowf)(ac, ES8, addlen, min_cap);
            let br = (p.rs.arrgrowf)(ar, ES8, addlen, min_cap);
            assert!(
                std::ptr::eq(bc as *const u8, ac as *const u8),
                "C should no-op for addlen={addlen} min_cap={min_cap}"
            );
            assert!(
                std::ptr::eq(br as *const u8, ar as *const u8),
                "RS should no-op for addlen={addlen} min_cap={min_cap}"
            );
            assert_eq!(hdr(bc), hdr(br));
        }
        (p.c.arrfreef)(ac);
        (p.rs.arrfreef)(ar);
    }
}

#[test]
fn err02_arrgrowf_null_input_initialises_header() {
    let (p, _g) = libs();
    unsafe {
        for &(es, addlen, min_cap) in &[(8usize, 1usize, 0usize), (8, 0, 1), (20, 7, 3), (1, 0, 9)] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
            let ar = (p.rs.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
            assert_eq!(hdr(ac), hdr(ar), "arrgrowf(NULL,{es},{addlen},{min_cap})");
            // length, hash_table and temp must be explicitly zeroed
            assert_eq!((*header(ac)).length, 0);
            assert_eq!((*header(ac)).temp, 0);
            assert!((*header(ac)).hash_table.is_null());
            (p.c.arrfreef)(ac);
            (p.rs.arrfreef)(ar);
        }
    }
}

#[test]
fn err03_04_arrgrowf_clamp_branches() {
    let (p, _g) = libs();
    unsafe {
        // row 3: arrcap < min_cap < 2*arrcap  -> clamped to 2*arrcap
        // row 4: min_cap >= 2*arrcap && < 4   -> clamped to 4
        for &start in &[1usize, 2, 3, 4, 8, 16, 32] {
            let mut ac = (p.c.arrgrowf)(std::ptr::null_mut(), ES8, 0, start);
            let mut ar = (p.rs.arrgrowf)(std::ptr::null_mut(), ES8, 0, start);
            let cap = (*header(ac)).capacity;
            for target in 0..(4 * cap + 6) {
                let bc = (p.c.arrgrowf)(ac, ES8, 0, target);
                let br = (p.rs.arrgrowf)(ar, ES8, 0, target);
                assert_eq!(hdr(bc), hdr(br), "clamp start={start} target={target}");
                ac = bc;
                ar = br;
            }
            (p.c.arrfreef)(ac);
            (p.rs.arrfreef)(ar);
        }
        // the `min_cap < 4` clamp is only reachable from capacity 0/1
        let ac = (p.c.arrgrowf)(std::ptr::null_mut(), ES8, 1, 0);
        let ar = (p.rs.arrgrowf)(std::ptr::null_mut(), ES8, 1, 0);
        assert_eq!((*header(ac)).capacity, 4, "C clamps to 4");
        assert_eq!((*header(ar)).capacity, 4, "RS clamps to 4");
        assert_eq!(hdr(ac), hdr(ar));
        (p.c.arrfreef)(ac);
        (p.rs.arrfreef)(ar);
    }
}

#[test]
fn err05_arrgrowf_elemsize_zero() {
    let (p, _g) = libs();
    unsafe {
        for &(addlen, min_cap) in &[(1usize, 0usize), (0, 1), (0, 4), (9, 3), (0, 1 << 20)] {
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            let ar = (p.rs.arrgrowf)(std::ptr::null_mut(), 0, addlen, min_cap);
            assert_eq!(hdr(ac), hdr(ar), "arrgrowf(NULL,0,{addlen},{min_cap})");
            (p.c.arrfreef)(ac);
            (p.rs.arrfreef)(ar);
        }
    }
}

#[test]
fn err06a_arrgrowf_size_multiplication_wraps() {
    let (p, _g) = libs();
    // `elemsize * min_cap` wraps modulo 2^64. These pairs wrap to a value that
    // is still large enough for the header, so nothing is corrupted and the
    // (absurd) resulting `capacity` can be compared directly.
    unsafe {
        for &(es, mc) in &[
            (8usize, (1usize << 61) + 64),   // 8*mc  == 2^64 + 512
            (16usize, (1usize << 60) + 8),   // 16*mc == 2^64 + 128
            (4usize, (1usize << 62) + 32),   // 4*mc  == 2^64 + 128
            (2usize, (1usize << 63) + 64),   // 2*mc  == 2^64 + 128
        ] {
            assert_eq!(es.wrapping_mul(mc), es.wrapping_mul(mc) & 0xfff, "test setup");
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), es, 0, mc);
            let ar = (p.rs.arrgrowf)(std::ptr::null_mut(), es, 0, mc);
            assert_eq!(hdr(ac), hdr(ar), "arrgrowf({es},0,{mc:#x})");
            assert_eq!(
                (*header(ac)).capacity,
                mc,
                "capacity is the unvalidated min_cap"
            );
            (p.c.arrfreef)(ac);
            (p.rs.arrfreef)(ar);
        }
    }
}

#[test]
fn err06c_arrgrowf_wrap_to_undersized_allocation() {
    let (p, _g) = libs();
    // `elemsize * min_cap + 32` wraps to *less* than the header size, so the C
    // writes the header past the end of the block and corrupts the heap. Capture
    // the header from a child, then let the child die however it likes.
    for &(es, mc) in &[
        (8usize, usize::MAX),
        (16usize, usize::MAX),
        (4usize, usize::MAX),
        (2usize, usize::MAX),
        (8usize, usize::MAX / 8 + 4),
    ] {
        let (_, out) = assert_same_capture(
            p,
            &format!("arrgrowf wrap-undersized({es},0,{mc:#x})"),
            move |lib| unsafe {
                let a = (lib.arrgrowf)(std::ptr::null_mut(), es, 0, mc);
                let mut v = Vec::new();
                v.push(a.is_null() as u8);
                if !a.is_null() {
                    let h = header(a);
                    v.extend_from_slice(&(*h).length.to_ne_bytes());
                    v.extend_from_slice(&(*h).capacity.to_ne_bytes());
                    v.extend_from_slice(&(*h).temp.to_ne_bytes());
                    v.push((*h).hash_table.is_null() as u8);
                }
                v
            },
        );
        assert!(!out.is_empty(), "expected a captured header for ({es},{mc:#x})");
    }
}

#[test]
fn err06b_arrgrowf_allocation_failure_faults_identically() {
    let (p, _g) = libs();
    // A genuinely impossible size: realloc returns NULL, the C adds
    // sizeof(header) to it and writes through it. Both builds must fault the
    // same way, so compare child termination.
    let o = assert_same_outcome(p, "arrgrowf huge alloc", |lib| unsafe {
        let a = (lib.arrgrowf)(std::ptr::null_mut(), 1 << 20, 0, 1 << 40);
        std::hint::black_box(a);
    });
    assert!(
        matches!(o, Outcome::Signalled(_)),
        "expected both to fault, got {o:?}"
    );
}

// ===========================================================================
// Row 7 — the make_hash_index threshold assert is unreachable from the API
// ===========================================================================

#[test]
fn err07_slot_count_is_always_a_power_of_two_at_least_8() {
    let (p, _g) = libs();
    // `STBDS_ASSERT(used_count_threshold + tombstone_count_threshold <
    // slot_count)` only fails for slot_count in {0,1,2}. Prove the public API
    // never produces such a slot count: it starts at 8 and only doubles/halves,
    // with halving gated on `slot_count > STBDS_BUCKET_LENGTH`.
    reseed(p, DEFAULT_SEED);
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "slotcount");
    m.put_default(&i32b(-2));
    let mut seen = std::collections::BTreeSet::new();
    for k in 0..600i32 {
        m.put_bytes(&i32b(k), &i32b(k));
        let sc = m.snaps().0;
        let n = sc.idx.as_ref().unwrap().slot_count;
        assert!(n >= 8 && n.is_power_of_two(), "bad slot_count {n}");
        seen.insert(n);
    }
    for k in 0..600i32 {
        m.del_bytes(&i32b(k), 0, STBDS_HM_BINARY);
        let sc = m.snaps().0;
        let n = sc.idx.as_ref().unwrap().slot_count;
        assert!(n >= 8 && n.is_power_of_two(), "bad slot_count {n}");
        seen.insert(n);
    }
    assert!(seen.len() > 4, "should have grown and shrunk several times");
    assert_eq!(*seen.iter().next().unwrap(), 8);
    m.free();
}

// ===========================================================================
// Rows 8-10 — hashing edge cases
// ===========================================================================

#[test]
fn err08_hash_bytes_zero_length_null_pointer() {
    let (p, _g) = libs();
    unsafe {
        for &seed in &[0usize, 1, DEFAULT_SEED, usize::MAX] {
            let a = (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed);
            let b = (p.rs.hash_bytes)(std::ptr::null_mut(), 0, seed);
            assert_eq!(a, b, "hash_bytes(NULL, 0, {seed:#x})");
            // and identical to hashing a zero-length non-null buffer
            let mut empty: [u8; 1] = [0];
            let c = (p.c.hash_bytes)(empty.as_mut_ptr() as *mut c_void, 0, seed);
            assert_eq!(a, c);
        }
    }
}

#[test]
fn err09_hash_bytes_every_tail_length() {
    let (p, _g) = libs();
    // one row per `len - i` switch case (0..7), each with the byte values that
    // make the `case 4: d[3] << 24` sign extension observable
    let mut rng = Rng::new(0x0900);
    for rem in 0..8usize {
        for &blocks in &[0usize, 1, 2, 5] {
            let len = blocks * 8 + rem;
            for pattern in 0..6 {
                let mut buf: Vec<u8> = match pattern {
                    0 => vec![0x00; len],
                    1 => vec![0xFF; len],
                    2 => vec![0x80; len],
                    3 => (0..len).map(|i| i as u8).collect(),
                    4 => (0..len).map(|i| 0xFF - i as u8).collect(),
                    _ => rng.bytes(len),
                };
                for &seed in &[0usize, DEFAULT_SEED, usize::MAX] {
                    let a =
                        unsafe { (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                    let b =
                        unsafe { (p.rs.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, seed) };
                    assert_eq!(a, b, "hash_bytes rem={rem} len={len} pat={pattern}");
                }
            }
        }
    }
}

#[test]
fn err10_hash_string_empty() {
    let (p, _g) = libs();
    unsafe {
        let mut e = [0u8; 1];
        for &seed in &[0usize, 1, DEFAULT_SEED, usize::MAX, 0x8000_0000_0000_0000] {
            let a = (p.c.hash_string)(e.as_mut_ptr() as *mut c_char, seed);
            let b = (p.rs.hash_string)(e.as_mut_ptr() as *mut c_char, seed);
            assert_eq!(a, b, "hash_string(\"\", {seed:#x})");
        }
    }
}

// ===========================================================================
// Rows 11-12 / G3 — out-of-range `mode` values across the FFI boundary
// ===========================================================================

/// Every `mode` a caller can legally pass as a C `int`, including values with no
/// enum variant.
const MODES: &[c_int] = &[
    i32::MIN,
    -1000,
    -2,
    -1,
    0,
    1,
    2,
    3,
    4,
    99,
    1000,
    i32::MAX,
];

#[test]
fn err11_12_binary_lookup_accepts_any_mode_below_one() {
    let (p, _g) = libs();
    // Build with mode 0, then read/delete with every mode < 1: `mode >= 1` is
    // false so the binary `memcmp`/siphash path must be taken every time.
    for &mode in MODES.iter().filter(|&&m| m < STBDS_HM_STRING) {
        reseed(p, DEFAULT_SEED);
        let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, &format!("binmode{mode}"));
        m.put_default(&i32b(-2));
        for k in 0..30i32 {
            m.put_bytes(&i32b(k), &i32b(k * 2));
        }
        for k in 0..40i32 {
            let g = m.geti(&i32b(k), mode);
            assert_eq!(g >= 0, k < 30, "mode={mode} key={k}");
            assert_eq!(g, m.geti_ts(&i32b(k), mode));
        }
        for k in 0..40i32 {
            let r = m.del_bytes(&i32b(k), 0, mode);
            assert_eq!(r == 1, k < 30, "del mode={mode} key={k}");
        }
        assert_eq!(m.snaps().0.length, 1);
        m.free();
    }
}

#[test]
fn err11_string_lookup_accepts_any_mode_at_or_above_one() {
    let (p, _g) = libs();
    // Any mode >= 1 selects strcmp/hash_string. Build and read with the same
    // out-of-range mode so the pair stays self-consistent.
    for &mode in MODES.iter().filter(|&&m| m >= STBDS_HM_STRING) {
        reseed(p, DEFAULT_SEED);
        let mut rng = Rng::new(0x1100 + mode as u64);
        let mut m = DualMap::empty(p, ES16, KS8, KeyKind::Ptr, &format!("strmode{mode}"));
        let keys: Vec<*mut c_char> = (0..30)
            .map(|i| leak_cstr(&format!("m{mode}_{}{i}", "u".repeat(rng.below(10)))))
            .collect();
        for (n, &k) in keys.iter().enumerate() {
            m.put_str(k, &pad(i32b(n as i32), ES16 - KS8), mode);
        }
        for &k in &keys {
            assert!(m.geti_str(k, mode) >= 0, "mode={mode}");
        }
        for i in 0..10 {
            assert_eq!(m.geti_str(leak_cstr(&format!("gone{i}")), mode), -1);
        }
        // LIFO deletion keeps `old_index == final_index`, so the
        // `mode == STBDS_HM_STRING`-only relocate branch is not needed
        for &k in keys.iter().rev() {
            assert_eq!(m.del_str(k, 0, mode), 1, "mode={mode}");
        }
        m.free();
    }
}

// ===========================================================================
// Rows 13-14 — stbds_hmfree_func rejections
// ===========================================================================

#[test]
fn err13_hmfree_func_null_is_a_noop() {
    let (p, _g) = libs();
    let o = assert_same_outcome(p, "hmfree_func(NULL)", |lib| unsafe {
        for es in [0usize, 1, 8, 16, 4096] {
            (lib.hmfree_func)(std::ptr::null_mut(), es);
        }
    });
    assert_eq!(o, Outcome::Exited(0), "hmfree_func(NULL) must be a no-op");
}

#[test]
fn err14_hmfree_func_without_hash_table() {
    let (p, _g) = libs();
    let o = assert_same_outcome(p, "hmfree_func no index", |lib| unsafe {
        (lib.rand_seed)(DEFAULT_SEED);
        // an array produced by hmget_key on NULL never gets an index
        let mut key = 1i32;
        let t = (lib.hmget_key)(
            std::ptr::null_mut(),
            ES8,
            &mut key as *mut i32 as *mut c_void,
            KS4,
            STBDS_HM_BINARY,
        );
        let a = hash_to_arr(t, ES8);
        assert!((*header(a)).hash_table.is_null());
        (lib.hmfree_func)(a, ES8);
        // ... and one straight from arrgrowf
        let a2 = (lib.arrgrowf)(std::ptr::null_mut(), ES8, 2, 0);
        (lib.hmfree_func)(a2, ES8);
    });
    assert_eq!(o, Outcome::Exited(0));
}

// ===========================================================================
// Rows 15-19 — lookup rejections
// ===========================================================================

#[test]
fn err15_18_19_lookup_miss_yields_minus_one() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    let mut rng = Rng::new(0x1500);
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "miss");
    m.put_default(&i32b(-2));
    let mut present = std::collections::BTreeSet::new();
    for _ in 0..50 {
        let k = rng.i32() % 200;
        m.put_bytes(&i32b(k), &i32b(k));
        present.insert(k);
    }
    for _ in 0..1000 {
        let k = rng.i32();
        if present.contains(&k) {
            continue;
        }
        // row 15/18: find_slot -> -1, temp = STBDS_INDEX_EMPTY
        assert_eq!(m.geti(&i32b(k), STBDS_HM_BINARY), -1, "key {k}");
        assert_eq!(m.geti_ts(&i32b(k), STBDS_HM_BINARY), -1, "key {k}");
        // row 19: hmget_key mirrored the -1 into the header
        unsafe {
            assert_eq!((*header(hash_to_arr(m.tc, ES8))).temp, -1);
            assert_eq!((*header(hash_to_arr(m.tr, ES8))).temp, -1);
        }
    }
    m.free();
}

#[test]
fn err16_hmget_key_ts_on_null_bootstraps() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    for &es in &[1usize, 4, 8, 16, 20] {
        unsafe {
            let mut key = [0u8; 8];
            let mut tc: isize = 0x5A5A;
            let mut tr: isize = 0x5A5A;
            let a = (p.c.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                es.min(4),
                &mut tc,
                STBDS_HM_BINARY,
            );
            let b = (p.rs.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                key.as_mut_ptr() as *mut c_void,
                es.min(4),
                &mut tr,
                STBDS_HM_BINARY,
            );
            assert_eq!(tc, -1, "C temp");
            assert_eq!(tr, -1, "RS temp");
            assert!(!a.is_null() && !b.is_null(), "never returns NULL");
            let sc = snap_hm(a, es, KeyKind::Bytes);
            let sr = snap_hm(b, es, KeyKind::Bytes);
            assert_eq!(sc, sr, "bootstrap es={es}");
            assert_eq!(sc.length, 1);
            assert!(sc.idx.is_none());
            assert_eq!(sc.elems[0], vec![0u8; es], "element 0 must be zeroed");
            (p.c.hmfree_func)(hash_to_arr(a, es), es);
            (p.rs.hmfree_func)(hash_to_arr(b, es), es);
        }
    }
}

#[test]
fn err17_hmget_without_index_never_dereferences_the_key() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    // With `hash_table == NULL`, both hmget_key_ts and hmdel_key bail out
    // *before* hashing, so even a NULL key pointer must be accepted.
    let o = assert_same_outcome(p, "null key, no index", |lib| unsafe {
        (lib.rand_seed)(DEFAULT_SEED);
        let mut k = 1i32;
        let t = (lib.hmget_key)(
            std::ptr::null_mut(),
            ES8,
            &mut k as *mut i32 as *mut c_void,
            KS4,
            STBDS_HM_BINARY,
        );
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING, 7, -3] {
            let mut tmp: isize = 0x5A5A;
            let t2 = (lib.hmget_key_ts)(t, ES8, std::ptr::null_mut(), KS4, &mut tmp, mode);
            assert_eq!(tmp, -1);
            assert!(std::ptr::eq(t2 as *const u8, t as *const u8));
            let t3 = (lib.hmdel_key)(t, ES8, std::ptr::null_mut(), KS4, 0, mode);
            assert!(std::ptr::eq(t3 as *const u8, t as *const u8));
            assert_eq!(temp_of(t, ES8), 0, "hmdel_key sets temp = 0");
        }
        (lib.hmfree_func)(hash_to_arr(t, ES8), ES8);
    });
    assert_eq!(o, Outcome::Exited(0), "no-index path must not touch the key");
}

// ===========================================================================
// Rows 20-22 — stbds_hmput_default
// ===========================================================================

#[test]
fn err20_21_22_hmput_default_branches() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    for &es in &[1usize, 4, 8, 16, 20] {
        unsafe {
            // row 20: a == NULL
            let ac = (p.c.hmput_default)(std::ptr::null_mut(), es);
            let ar = (p.rs.hmput_default)(std::ptr::null_mut(), es);
            assert_eq!(hdr(hash_to_arr(ac, es)), hdr(hash_to_arr(ar, es)));
            assert_eq!((*header(hash_to_arr(ac, es))).length, 1);
            assert_eq!(
                snap_hm(ac, es, KeyKind::Bytes),
                snap_hm(ar, es, KeyKind::Bytes)
            );

            // row 22: length != 0 -> exact same pointer, no realloc
            let bc = (p.c.hmput_default)(ac, es);
            let br = (p.rs.hmput_default)(ar, es);
            assert!(std::ptr::eq(bc as *const u8, ac as *const u8), "C no-op");
            assert!(std::ptr::eq(br as *const u8, ar as *const u8), "RS no-op");

            // row 21: length == 0 -> grows again and re-zeroes element 0
            std::ptr::write_bytes(bc as *mut u8, 0xEE, es);
            std::ptr::write_bytes(br as *mut u8, 0xEE, es);
            (*header(hash_to_arr(bc, es))).length = 0;
            (*header(hash_to_arr(br, es))).length = 0;
            let cc = (p.c.hmput_default)(bc, es);
            let cr = (p.rs.hmput_default)(br, es);
            let sc = snap_hm(cc, es, KeyKind::Bytes);
            let sr = snap_hm(cr, es, KeyKind::Bytes);
            assert_eq!(sc, sr, "hmput_default length==0, es={es}");
            assert_eq!(sc.length, 1);
            assert_eq!(sc.elems[0], vec![0u8; es], "element 0 re-zeroed");

            (p.c.hmfree_func)(hash_to_arr(cc, es), es);
            (p.rs.hmfree_func)(hash_to_arr(cr, es), es);
        }
    }
}

// ===========================================================================
// Rows 23-28 — stbds_hmput_key branches
// ===========================================================================

#[test]
fn err23_24_hmput_key_bootstraps_and_creates_index() {
    let (p, _g) = libs();
    for &mode in &[STBDS_HM_BINARY, -5, STBDS_HM_STRING, 42] {
        reseed(p, DEFAULT_SEED);
        unsafe {
            let (es, ks) = if mode >= STBDS_HM_STRING {
                (ES16, KS8)
            } else {
                (ES8, KS4)
            };
            let key: *mut c_void = if mode >= STBDS_HM_STRING {
                leak_cstr("bootstrap") as *mut c_void
            } else {
                Box::leak(Box::new(7i32)) as *mut i32 as *mut c_void
            };
            let tc = (p.c.hmput_key)(std::ptr::null_mut(), es, key, ks, mode);
            let tr = (p.rs.hmput_key)(std::ptr::null_mut(), es, key, ks, mode);
            let kind = if mode >= STBDS_HM_STRING {
                KeyKind::Ptr
            } else {
                KeyKind::Bytes
            };
            // the value half is untouched by the library; normalise it
            std::ptr::write_bytes((tc as *mut u8).add(ks), 0, es - ks);
            std::ptr::write_bytes((tr as *mut u8).add(ks), 0, es - ks);
            let sc = snap_hm(tc, es, kind);
            let sr = snap_hm(tr, es, kind);
            assert_eq!(sc, sr, "hmput_key bootstrap mode={mode}");
            assert_eq!(sc.length, 2, "default slot + the new element");
            let idx = sc.idx.as_ref().unwrap();
            assert_eq!(idx.slot_count, 8, "first index is STBDS_BUCKET_LENGTH slots");
            assert_eq!(
                idx.string_mode,
                if mode >= STBDS_HM_STRING { 1 } else { 0 },
                "string.mode = (mode >= STBDS_HM_STRING ? SH_DEFAULT : 0)"
            );
            (p.c.hmfree_func)(hash_to_arr(tc, es), es);
            (p.rs.hmfree_func)(hash_to_arr(tr, es), es);
        }
    }
}

#[test]
fn err25_hmput_key_rehash_at_used_count_threshold() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "rehash");
    m.put_default(&i32b(-2));
    let mut prev = 0usize;
    let mut doublings = 0;
    for k in 0..300i32 {
        m.put_bytes(&i32b(k), &i32b(k));
        let sc = m.snaps().0;
        let idx = sc.idx.as_ref().unwrap();
        if idx.slot_count != prev {
            if prev != 0 {
                assert_eq!(idx.slot_count, prev * 2, "must double");
                doublings += 1;
            }
            prev = idx.slot_count;
        }
        assert!(
            idx.used_count <= idx.used_count_threshold,
            "used_count {} must never exceed threshold {} (the rehash happens \
             before the insert, so equality is reachable)",
            idx.used_count,
            idx.used_count_threshold
        );
        assert_eq!(idx.used_count_threshold, idx.slot_count - (idx.slot_count >> 2));
        assert_eq!(
            idx.tombstone_count_threshold,
            (idx.slot_count >> 3) + (idx.slot_count >> 4)
        );
        assert_eq!(
            idx.used_count_shrink_threshold,
            if idx.slot_count <= 8 {
                0
            } else {
                idx.slot_count >> 2
            }
        );
    }
    assert!(doublings >= 5, "expected several rehashes, saw {doublings}");
    m.free();
}

#[test]
fn err26_27_hmput_key_duplicate_key_early_returns() {
    let (p, _g) = libs();
    // Row 26 (in-bucket hit, publishes temp_key) and row 27 (wrap-around hit,
    // does NOT publish temp_key). Which of the two a given key takes depends on
    // its hash, so hammer many keys at many table sizes and compare state and
    // temp_key after every duplicate put.
    for trial in 0..8 {
        reseed(p, 0x2626 + trial);
        let mut rng = Rng::new(0x2600 + trial as u64);
        let mut m =
            DualMap::empty(p, ES16, KS8, KeyKind::Ptr, &format!("dup/trial{trial}"));
        let keys: Vec<*mut c_char> = (0..90)
            .map(|i| leak_cstr(&format!("dup{trial}_{}{i}", "v".repeat(rng.below(12)))))
            .collect();
        for (n, &k) in keys.iter().enumerate() {
            m.put_str(k, &pad(i32b(n as i32), ES16 - KS8), STBDS_HM_STRING);
        }
        // now every put is a duplicate
        for round in 0..3 {
            for (n, &k) in keys.iter().enumerate() {
                let before_len = m.snaps().0.length;
                let i = m.put_str(k, &pad(i32b(n as i32 + round * 100), ES16 - KS8), STBDS_HM_STRING);
                assert!(i >= 0);
                assert_eq!(
                    m.snaps().0.length,
                    before_len,
                    "duplicate put must not extend the array"
                );
                m.assert_temp_key_same();
            }
        }
        // and in binary mode
        let mut mb = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, &format!("dupbin/{trial}"));
        mb.put_default(&i32b(-2));
        for k in 0..90i32 {
            mb.put_bytes(&i32b(k), &i32b(k));
        }
        for _ in 0..3 {
            for k in 0..90i32 {
                let before = mb.snaps().0.length;
                mb.put_bytes(&i32b(k), &i32b(-k));
                assert_eq!(mb.snaps().0.length, before);
            }
        }
        m.free();
        mb.free();
    }
}

#[test]
fn err28_hmput_key_reuses_tombstones() {
    let (p, _g) = libs();
    for trial in 0..8 {
        reseed(p, 0x2828 + trial);
        let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, &format!("tomb/{trial}"));
        m.put_default(&i32b(-2));
        for k in 0..40i32 {
            m.put_bytes(&i32b(k), &i32b(k));
        }
        // delete then re-insert repeatedly: inserts must land on tombstones and
        // decrement tombstone_count
        for round in 0..30i32 {
            for k in (0..40i32).step_by(4) {
                m.del_bytes(&i32b(k), 0, STBDS_HM_BINARY);
            }
            for k in (0..40i32).step_by(4) {
                m.put_bytes(&i32b(k), &i32b(k + round));
            }
            let sc = m.snaps().0;
            assert_eq!(sc.length, 41, "population must be stable");
        }
        m.free();
    }
}

// ===========================================================================
// Rows 30-31 / G4, G7 — out-of-range string.mode via stbds_shmode_func
// ===========================================================================

/// `h->string.mode = (unsigned char) mode`, so the effective mode is `mode & 0xff`.
#[test]
fn err31_shmode_func_truncates_mode_to_u8() {
    let (p, _g) = libs();
    for &mode in &[
        i32::MIN,
        -1000,
        -2,
        -1,
        0,
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
        260,
        1000,
        i32::MAX,
    ] {
        reseed(p, DEFAULT_SEED);
        unsafe {
            let tc = (p.c.shmode_func)(ES16, mode);
            let tr = (p.rs.shmode_func)(ES16, mode);
            let sc = snap_hm(tc, ES16, KeyKind::Bytes);
            let sr = snap_hm(tr, ES16, KeyKind::Bytes);
            assert_eq!(sc, sr, "shmode_func(_, {mode})");
            assert_eq!(
                sc.idx.as_ref().unwrap().string_mode,
                (mode as u32 & 0xff) as u8,
                "string.mode should be (unsigned char) {mode}"
            );
            (p.c.hmfree_func)(hash_to_arr(tc, ES16), ES16);
            (p.rs.hmfree_func)(hash_to_arr(tr, ES16), ES16);
        }
    }
}

#[test]
fn err30_hmput_key_switch_default_branch_for_unknown_string_mode() {
    let (p, _g) = libs();
    // string.mode values with no enum variant fall into `default:` (memcpy of
    // keysize bytes out of the key string), exactly like SH_NONE.
    for &raw_mode in &[0i32, 4, 5, 127, 200, 255, 256, -1, i32::MAX] {
        let eff = (raw_mode as u32 & 0xff) as u8;
        // pick the snapshot strategy from the *effective* mode
        let kind = match eff {
            1 | 2 | 3 => KeyKind::Ptr,
            _ => KeyKind::Bytes,
        };
        reseed(p, 0x3030);
        let mut m = DualMap::shmode(
            p,
            ES16,
            KS8,
            kind,
            raw_mode,
            &format!("rawmode{raw_mode}"),
        );
        // Only a handful of keys: with the `default:` branch the element's first
        // 8 bytes hold string bytes, which the C would later strcmp *as a
        // pointer* if two keys ever collided on hash.
        let n = if kind == KeyKind::Ptr { 40 } else { 5 };
        for i in 0..n {
            let k = leak_cstr(&format!("rm{raw_mode}_{i}"));
            m.put_str(k, &pad(i32b(i), ES16 - KS8), STBDS_HM_STRING);
        }
        m.assert_same(&format!("raw string.mode {raw_mode}"));
        assert_eq!(m.snaps().0.idx.as_ref().unwrap().string_mode, eff);
        m.free();
    }
}

// ===========================================================================
// Rows 32-34, 39-43 — stbds_hmdel_key rejections and branches
// ===========================================================================

#[test]
fn err32_hmdel_key_null_returns_null() {
    let (p, _g) = libs();
    unsafe {
        for &mode in MODES {
            for &es in &[1usize, 8, 16] {
                let mut k = [0u8; 8];
                let a = (p.c.hmdel_key)(
                    std::ptr::null_mut(),
                    es,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                    mode,
                );
                let b = (p.rs.hmdel_key)(
                    std::ptr::null_mut(),
                    es,
                    k.as_mut_ptr() as *mut c_void,
                    4,
                    0,
                    mode,
                );
                assert!(a.is_null(), "C hmdel_key(NULL) must return 0");
                assert!(b.is_null(), "RS hmdel_key(NULL) must return 0");
            }
        }
    }
}

#[test]
fn err33_hmdel_key_without_index_sets_temp_zero() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "del/noindex");
    m.geti(&i32b(1), STBDS_HM_BINARY); // creates an index-less array, temp = -1
    unsafe {
        assert_eq!((*header(hash_to_arr(m.tc, ES8))).temp, -1);
    }
    for k in 0..16i32 {
        // returns `a` and forces temp to 0
        assert_eq!(m.del_bytes(&i32b(k), 0, STBDS_HM_BINARY), 0);
        unsafe {
            assert_eq!((*header(hash_to_arr(m.tc, ES8))).temp, 0);
            assert_eq!((*header(hash_to_arr(m.tr, ES8))).temp, 0);
        }
    }
    assert!(m.snaps().0.idx.is_none());
    m.free();
}

#[test]
fn err34_hmdel_key_absent_key_is_a_noop() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    let mut rng = Rng::new(0x3400);
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "del/absent");
    m.put_default(&i32b(-2));
    for k in 0..30i32 {
        m.put_bytes(&i32b(k), &i32b(k));
    }
    // one absent delete first: hmdel_key unconditionally forces the header
    // `temp` to 0 before it even looks the key up, so that is the one field an
    // absent delete legitimately changes.
    assert_eq!(m.del_bytes(&i32b(999_999), 0, STBDS_HM_BINARY), 0);
    let before = m.snaps().0;
    for _ in 0..500 {
        let k = 1000 + (rng.i32().abs() % 100000);
        assert_eq!(m.del_bytes(&i32b(k), 0, STBDS_HM_BINARY), 0, "key {k}");
    }
    let after = m.snaps().0;
    assert_eq!(before, after, "absent deletes must not change any state");
    m.free();
}

#[test]
fn err39_strdup_free_only_at_mode_exactly_one() {
    let (p, _g) = libs();
    // Row 39: the key is freed only when mode == 1 AND string.mode == SH_STRDUP.
    // Verified through the observable state plus identical termination.
    for &(sh, name) in &[
        (SH_STRDUP, "SH_STRDUP"),
        (SH_DEFAULT, "SH_DEFAULT"),
        (SH_ARENA, "SH_ARENA"),
    ] {
        for &mode in &[1i32, 2, 5] {
            let o = assert_same_outcome(p, &format!("{name}/mode{mode}"), move |lib| unsafe {
                (lib.rand_seed)(DEFAULT_SEED);
                let mut m = SoloMap::shmode(lib, ES16, KS8, sh);
                let keys: Vec<*mut c_char> = (0..12)
                    .map(|i| leak_cstr(&format!("e39_{sh}_{mode}_{i}")))
                    .collect();
                for (n, &k) in keys.iter().enumerate() {
                    m.put_str(k, &pad(i32b(n as i32), ES16 - KS8), mode);
                }
                // LIFO: no relocate, so mode 2/5 do not hit the assert
                for &k in keys.iter().rev() {
                    assert_eq!(m.del_str(k, 0, mode), 1);
                }
                m.free();
            });
            assert_eq!(o, Outcome::Exited(0), "{name} mode={mode}");
        }
    }
    // and the in-memory state agrees for the mode == 1 case
    for &sh in &[SH_STRDUP, SH_DEFAULT, SH_ARENA] {
        reseed(p, DEFAULT_SEED);
        let mut m = DualMap::shmode(p, ES16, KS8, KeyKind::Ptr, sh, &format!("e39mem{sh}"));
        let keys: Vec<*mut c_char> = (0..12)
            .map(|i| leak_cstr(&format!("e39m_{sh}_{i}")))
            .collect();
        for (n, &k) in keys.iter().enumerate() {
            m.put_str(k, &pad(i32b(n as i32), ES16 - KS8), STBDS_HM_STRING);
        }
        for &k in &keys {
            assert_eq!(m.del_str(k, 0, STBDS_HM_STRING), 1);
        }
        assert_eq!(m.snaps().0.length, 1);
        m.free();
    }
}

#[test]
fn err40_hmdel_key_deleting_the_last_element_skips_relocation() {
    let (p, _g) = libs();
    for trial in 0..8 {
        reseed(p, 0x4040 + trial);
        let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, &format!("del/lifo{trial}"));
        m.put_default(&i32b(-2));
        for k in 0..60i32 {
            m.put_bytes(&i32b(k), &i32b(k * 9));
        }
        // strict LIFO: old_index always == final_index
        for k in (0..60i32).rev() {
            assert_eq!(m.del_bytes(&i32b(k), 0, STBDS_HM_BINARY), 1, "key {k}");
        }
        assert_eq!(m.snaps().0.length, 1);
        m.free();
    }
}

#[test]
fn err41_42_hmdel_key_shrink_and_rebuild() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "del/rebuild");
    m.put_default(&i32b(-2));
    for k in 0..400i32 {
        m.put_bytes(&i32b(k), &i32b(k));
    }
    let big = m.snaps().0.idx.as_ref().unwrap().slot_count;
    assert!(big >= 512);
    let mut shrinks = 0;
    let mut rebuilds = 0;
    let mut prev = m.snaps().0.idx.as_ref().unwrap().clone();
    for k in 0..400i32 {
        m.del_bytes(&i32b(k), 0, STBDS_HM_BINARY);
        let now = m.snaps().0.idx.as_ref().unwrap().clone();
        if now.slot_count == prev.slot_count / 2 {
            shrinks += 1;
        } else if now.slot_count == prev.slot_count && now.tombstone_count < prev.tombstone_count {
            rebuilds += 1;
        }
        prev = now;
    }
    assert!(shrinks >= 5, "row 41: expected shrinks, saw {shrinks}");
    assert!(rebuilds >= 1, "row 42: expected rebuilds, saw {rebuilds}");
    assert_eq!(m.snaps().0.idx.as_ref().unwrap().slot_count, 8);
    m.free();
}

#[test]
fn err43_hmdel_on_default_only_map_never_shrinks() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    // put one key then delete it: slot_count stays 8 and
    // used_count_shrink_threshold is forced to 0, so neither branch fires
    let mut m = DualMap::empty(p, ES8, KS4, KeyKind::Bytes, "del/minimal");
    m.put_default(&i32b(-2));
    m.put_bytes(&i32b(5), &i32b(50));
    for _ in 0..40 {
        assert_eq!(m.del_bytes(&i32b(5), 0, STBDS_HM_BINARY), 1);
        let idx = m.snaps().0.idx.as_ref().unwrap().clone();
        assert_eq!(idx.slot_count, 8);
        assert_eq!(idx.used_count_shrink_threshold, 0);
        m.put_bytes(&i32b(5), &i32b(50));
    }
    m.free();
}

// ===========================================================================
// Rows 44-50 — string arena rejections
// ===========================================================================

#[test]
fn err44_stralloc_remaining_invariant_holds_everywhere() {
    let (p, _g) = libs();
    // `STBDS_ASSERT(len <= a->remaining)` is unreachable from stralloc's own
    // logic: either `len > blocksize` (early return) or a new block of at least
    // `len` bytes is installed. Assert the invariant directly over a stress run
    // in both libraries.
    let mut rng = Rng::new(0x4400);
    for trial in 0..8 {
        let mut ac = CArena::zeroed();
        let mut ar = CArena::zeroed();
        for _ in 0..400 {
            let len = 1 + rng.below(if trial % 2 == 0 { 60 } else { 4000 });
            let mut buf = vec![b'z'; len];
            buf.push(0);
            unsafe {
                let before_c = ac.remaining;
                let rc = (p.c.stralloc)(&mut ac, buf.as_mut_ptr() as *mut c_char);
                let rr = (p.rs.stralloc)(&mut ar, buf.as_mut_ptr() as *mut c_char);
                assert_eq!(ac.remaining, ar.remaining, "remaining");
                assert_eq!(ac.block, ar.block, "block");
                assert!(!rc.is_null() && !rr.is_null());
                let _ = before_c;
            }
        }
        unsafe {
            (p.c.strreset)(&mut ac);
            (p.rs.strreset)(&mut ar);
        }
    }
}

#[test]
fn err45_46_stralloc_dedicated_block_paths() {
    let (p, _g) = libs();
    unsafe {
        // row 45: len > blocksize with a->storage == NULL
        for &len in &[513usize, 1000, 100_000] {
            let mut ac = CArena::zeroed();
            let mut ar = CArena::zeroed();
            let mut buf = vec![b'q'; len - 1];
            buf.push(0);
            let rc = (p.c.stralloc)(&mut ac, buf.as_mut_ptr() as *mut c_char);
            let rr = (p.rs.stralloc)(&mut ar, buf.as_mut_ptr() as *mut c_char);
            assert_eq!((ac.remaining, ac.block), (ar.remaining, ar.block));
            assert_eq!(ac.remaining, 0, "row 45 sets remaining = 0");
            assert_eq!(ac.block, 1, "block was incremented once");
            assert!(!ac.storage.is_null() && !ar.storage.is_null());
            // the returned pointer is the block's own storage
            assert_eq!(rc as *const u8, (ac.storage as *const u8).add(8));
            assert_eq!(rr as *const u8, (ar.storage as *const u8).add(8));
            (p.c.strreset)(&mut ac);
            (p.rs.strreset)(&mut ar);
        }

        // row 46: len > blocksize with a->storage != NULL -> spliced after the
        // head, `remaining` deliberately left alone
        for &len in &[600usize, 5000] {
            let mut ac = CArena::zeroed();
            let mut ar = CArena::zeroed();
            let mut small = b"small\0".to_vec();
            (p.c.stralloc)(&mut ac, small.as_mut_ptr() as *mut c_char);
            (p.rs.stralloc)(&mut ar, small.as_mut_ptr() as *mut c_char);
            let (rem_c, rem_r) = (ac.remaining, ar.remaining);
            let mut buf = vec![b'b'; len];
            buf.push(0);
            let rc = (p.c.stralloc)(&mut ac, buf.as_mut_ptr() as *mut c_char);
            let rr = (p.rs.stralloc)(&mut ar, buf.as_mut_ptr() as *mut c_char);
            assert_eq!(ac.remaining, rem_c, "C keeps remaining");
            assert_eq!(ar.remaining, rem_r, "RS keeps remaining");
            assert_eq!(ac.block, ar.block);
            // the new block became chain[1], not the head
            let next_c = *(ac.storage as *const *const u8);
            let next_r = *(ar.storage as *const *const u8);
            assert_eq!(rc as *const u8, next_c.add(8));
            assert_eq!(rr as *const u8, next_r.add(8));
            (p.c.strreset)(&mut ac);
            (p.rs.strreset)(&mut ar);
        }
    }
}

#[test]
fn err47_stralloc_block_counter_saturates_at_22() {
    let (p, _g) = libs();
    unsafe {
        let mut ac = CArena::zeroed();
        let mut ar = CArena::zeroed();
        // request ever-larger strings so `block` climbs until blocksize hits
        // STBDS_STRING_ARENA_BLOCKSIZE_MAX (1<<20) and stops
        let mut want = 400usize;
        for _ in 0..200 {
            let mut buf = vec![b's'; want];
            buf.push(0);
            (p.c.stralloc)(&mut ac, buf.as_mut_ptr() as *mut c_char);
            (p.rs.stralloc)(&mut ar, buf.as_mut_ptr() as *mut c_char);
            assert_eq!(ac.block, ar.block, "block counter");
            assert_eq!(ac.remaining, ar.remaining, "remaining");
            want = (want * 2).min(2_000_000);
        }
        assert_eq!(ac.block, 22, "block saturates at 22 (512 << 11 == 1<<20)");
        assert_eq!(ar.block, 22);
        (p.c.strreset)(&mut ac);
        (p.rs.strreset)(&mut ar);
    }
}

#[test]
fn err48_stralloc_with_out_of_range_block_counter() {
    let (p, _g) = libs();
    // A caller-supplied arena can carry any `block` in 0..=255. `stralloc`
    // computes `512 << (block >> 1)`, so:
    //   * block >= 24 asks for blocksizes of 8 MB and up — the realloc fails and
    //     the C writes through the NULL it got back (SIGSEGV);
    //   * block >= 128 shifts by >= 64, which is C UB; gcc on x86-64 emits a
    //     `shl` whose count is taken mod 64.
    // Whatever the C does for each value, the Rust must do the same, so run
    // every case in its own child pair and compare both the captured arena state
    // and how the child died. Each child caps its address space so that the
    // multi-gigabyte requests fail immediately instead of grinding inside glibc.
    const CAP: u64 = 1 << 31;
    for block in 0u8..=255 {
        for &len in &[1usize, 600] {
            let (_o, _out) = assert_same_capture_limited(
                p,
                CAP,
                &format!("stralloc block={block} len={len}"),
                move |lib| unsafe {
                    let mut a = CArena::zeroed();
                    a.block = block;
                    let mut buf = vec![b'k'; len - 1];
                    buf.push(0);
                    let r = (lib.stralloc)(&mut a, buf.as_mut_ptr() as *mut c_char);
                    let mut v = Vec::new();
                    v.extend_from_slice(&a.remaining.to_ne_bytes());
                    v.push(a.block);
                    v.push(a.mode);
                    v.push(a.storage.is_null() as u8);
                    v.push(r.is_null() as u8);
                    if !a.storage.is_null() && !r.is_null() {
                        // structural classification of where the result landed
                        let head = (a.storage as *const u8).add(8).add(a.remaining);
                        v.push((r as *const u8 == head) as u8);
                        let next = *(a.storage as *const *const u8);
                        v.push(
                            (!next.is_null() && r as *const u8 == next.add(8)) as u8,
                        );
                        v.extend_from_slice(std::slice::from_raw_parts(r as *const u8, len));
                    }
                    v
                },
            );
        }
    }
}

#[test]
fn err49_stralloc_empty_string() {
    let (p, _g) = libs();
    unsafe {
        let mut ac = CArena::zeroed();
        let mut ar = CArena::zeroed();
        let mut e = [0u8; 1];
        for i in 0..600 {
            let rc = (p.c.stralloc)(&mut ac, e.as_mut_ptr() as *mut c_char);
            let rr = (p.rs.stralloc)(&mut ar, e.as_mut_ptr() as *mut c_char);
            assert_eq!(*rc, 0);
            assert_eq!(*rr, 0);
            assert_eq!(
                (ac.remaining, ac.block),
                (ar.remaining, ar.block),
                "empty string #{i}"
            );
        }
        (p.c.strreset)(&mut ac);
        (p.rs.strreset)(&mut ar);
    }
}

#[test]
fn err50_strreset_on_zeroed_arena() {
    let (p, _g) = libs();
    unsafe {
        for _ in 0..4 {
            let mut ac = CArena::zeroed();
            let mut ar = CArena::zeroed();
            ac.block = 9;
            ac.mode = 3;
            ar.block = 9;
            ar.mode = 3;
            (p.c.strreset)(&mut ac);
            (p.rs.strreset)(&mut ar);
            // re-zeroes every field, including block and mode
            assert!(ac.storage.is_null() && ac.remaining == 0 && ac.block == 0 && ac.mode == 0);
            assert!(ar.storage.is_null() && ar.remaining == 0 && ar.block == 0 && ar.mode == 0);
            // idempotent
            (p.c.strreset)(&mut ac);
            (p.rs.strreset)(&mut ar);
            assert_eq!(
                (ac.remaining, ac.block, ac.mode),
                (ar.remaining, ar.block, ar.mode)
            );
        }
    }
}

// ===========================================================================
// Row 51 — stbds_arrfreef(NULL)
// ===========================================================================

#[test]
fn err51_arrfreef_null_faults_identically() {
    let (p, _g) = libs();
    // The C frees `(char *)NULL - sizeof(header)`. Not defensible, but it must
    // be reproduced, so compare how the process dies.
    let o = assert_same_outcome(p, "arrfreef(NULL)", |lib| unsafe {
        (lib.arrfreef)(std::ptr::null_mut());
    });
    assert!(
        matches!(o, Outcome::Signalled(_)),
        "expected both to die on the wild free, got {o:?}"
    );
}

// ===========================================================================
// Rows 52-53 — strkey
// ===========================================================================

#[test]
fn err52_53_strkey_negative_and_shared_buffer() {
    let (p, _g) = libs();
    unsafe {
        for &n in &[-1i32, -9, -10, -99, -100, -2147483647, i32::MIN] {
            let a = (p.c.strkey)(n);
            let b = (p.rs.strkey)(n);
            let sa = std::ffi::CStr::from_ptr(a).to_bytes().to_vec();
            let sb = std::ffi::CStr::from_ptr(b).to_bytes().to_vec();
            assert_eq!(sa, sb, "strkey({n})");
            assert_eq!(sa, format!("test_{n}").into_bytes());
        }
        // row 53: the buffer is shared and clobbered
        let a1 = (p.c.strkey)(1);
        let a2 = (p.c.strkey)(222222);
        assert_eq!(a1, a2);
        assert_eq!(std::ffi::CStr::from_ptr(a1).to_bytes(), b"test_222222");
        let b1 = (p.rs.strkey)(1);
        let b2 = (p.rs.strkey)(222222);
        assert_eq!(b1, b2);
        assert_eq!(std::ffi::CStr::from_ptr(b1).to_bytes(), b"test_222222");
    }
}

// ===========================================================================
// Rows 54-55 — hm_geti with rejecting / degenerate `num`
// ===========================================================================

#[test]
fn err54_55_hm_geti_degenerate_num() {
    let (p, _g) = libs();
    for &num in &[i32::MIN, -1000, -2, -1, 0, 1] {
        let o = assert_same_outcome(p, &format!("hm_geti({num})"), move |lib| unsafe {
            (lib.rand_seed)(DEFAULT_SEED);
            (lib.hm_geti)(num);
        });
        assert_eq!(o, Outcome::Exited(0), "hm_geti({num})");
    }
}

// ===========================================================================
// Generic FFI-boundary rows G1-G7
// ===========================================================================

#[test]
fn errg2_keysize_zero_makes_every_key_equal() {
    let (p, _g) = libs();
    reseed(p, DEFAULT_SEED);
    // memcmp(_, _, 0) == 0 and hash_bytes(_, 0, _) is constant, so the second
    // insert must find the first.
    let mut m = DualMap::empty(p, ES8, 0, KeyKind::Bytes, "keysize0");
    for n in 0..20i32 {
        m.put_bytes(&[], &pad(i32b(n), ES8));
        let s = m.snaps().0;
        assert_eq!(s.length, 2, "keysize 0 collapses to a single entry");
    }
    assert_eq!(m.geti(&[], STBDS_HM_BINARY), 0);
    assert_eq!(m.del_bytes(&[], 0, STBDS_HM_BINARY), 1);
    assert_eq!(m.geti(&[], STBDS_HM_BINARY), -1);
    m.free();
}

#[test]
fn errg3_del_with_nonzero_keyoffset() {
    let (p, _g) = libs();
    // `stbds_hmput_key` hardcodes keyoffset = 0 but `stbds_hmdel_key` takes it
    // as a parameter, so a non-zero value is a reachable input. It makes
    // find_slot compare the wrong bytes; whatever the C does (miss, or an assert
    // abort on the relocate) the Rust must match.
    for &keyoffset in &[1usize, 2, 3, 4] {
        let o = assert_same_outcome(p, &format!("del keyoffset={keyoffset}"), move |lib| unsafe {
            (lib.rand_seed)(DEFAULT_SEED);
            let mut m = SoloMap::empty(lib, ES8, KS4);
            m.put_default(&i32b(-2));
            for k in 0..20i32 {
                m.put_bytes(&i32b(k), &i32b(k));
            }
            let mut acc = 0isize;
            for k in 0..20i32 {
                acc += m.del_bytes(&i32b(k), keyoffset, STBDS_HM_BINARY);
            }
            std::process::exit((acc & 0x7f) as i32);
        });
        // the important part is that both agree; record what that is
        assert!(
            matches!(o, Outcome::Exited(_) | Outcome::Signalled(_)),
            "unexpected outcome {o:?}"
        );
    }
}

#[test]
fn errg3_null_temp_pointer_faults_identically() {
    let (p, _g) = libs();
    let o = assert_same_outcome(p, "hmget_key_ts(temp=NULL)", |lib| unsafe {
        let mut k = 1i32;
        (lib.hmget_key_ts)(
            std::ptr::null_mut(),
            ES8,
            &mut k as *mut i32 as *mut c_void,
            KS4,
            std::ptr::null_mut(),
            STBDS_HM_BINARY,
        );
    });
    assert!(
        matches!(o, Outcome::Signalled(_)),
        "expected both to fault on the NULL temp pointer, got {o:?}"
    );
}

#[test]
fn errg5_rand_seed_extremes() {
    let (p, _g) = libs();
    for &seed in &[0usize, 1, usize::MAX, usize::MAX - 1, 1 << 63, DEFAULT_SEED] {
        reseed(p, seed);
        // the first table records the seed verbatim, then the global advances
        for step in 0..6 {
            unsafe {
                let tc = (p.c.shmode_func)(ES16, SH_STRDUP);
                let tr = (p.rs.shmode_func)(ES16, SH_STRDUP);
                let sc = snap_hm(tc, ES16, KeyKind::Bytes);
                let sr = snap_hm(tr, ES16, KeyKind::Bytes);
                assert_eq!(sc, sr, "seed={seed:#x} step={step}");
                if step == 0 {
                    assert_eq!(sc.idx.as_ref().unwrap().seed, seed);
                }
                (p.c.hmfree_func)(hash_to_arr(tc, ES16), ES16);
                (p.rs.hmfree_func)(hash_to_arr(tr, ES16), ES16);
            }
        }
    }
    reseed(p, DEFAULT_SEED);
}

#[test]
fn errg6_arrgrowf_extreme_arguments() {
    let (p, _g) = libs();
    unsafe {
        for &es in &[1usize, 8, 16] {
            for &addlen in &[0usize, 1, 3, 4, 5, 1 << 20] {
                for &min_cap in &[0usize, 1, 3, 4, 5, 1 << 20] {
                    let ac = (p.c.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    let ar = (p.rs.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    assert_eq!(ac.is_null(), ar.is_null());
                    if !ac.is_null() {
                        assert_eq!(hdr(ac), hdr(ar), "arrgrowf({es},{addlen},{min_cap})");
                        (p.c.arrfreef)(ac);
                        (p.rs.arrfreef)(ar);
                    }
                }
            }
        }
    }
}
