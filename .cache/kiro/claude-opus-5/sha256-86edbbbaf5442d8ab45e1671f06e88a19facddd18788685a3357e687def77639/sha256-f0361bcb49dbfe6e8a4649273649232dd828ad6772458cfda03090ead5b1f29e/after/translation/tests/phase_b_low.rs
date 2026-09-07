//! Phase B — lowest-level entry points: `stbds_hash_bytes`,
//! `stbds_hash_string`, `stbds_rand_seed`, `stbds_arrgrowf`, `stbds_arrfreef`.
//!
//! CONFIGS.md rows 1-19.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

const SEED: u64 = 0xC0FFEE;

fn hash_bytes_both(buf: &mut [u8], len: usize, seed: usize) -> (usize, usize) {
    let (c, r) = libs();
    // Give each library its own copy so neither can observe the other's writes.
    let mut cb = buf.to_vec();
    let mut rb = buf.to_vec();
    unsafe {
        (
            (c.hash_bytes)(cb.as_mut_ptr() as *mut c_void, len, seed),
            (r.hash_bytes)(rb.as_mut_ptr() as *mut c_void, len, seed),
        )
    }
}

/// CONFIGS row 1: len == 0.
#[test]
fn row01_hash_bytes_len0() {
    let mut rng = Rng::new(SEED);
    for seed in [0usize, 1, 0x31415926, usize::MAX, usize::MAX >> 1] {
        let mut buf = rng.bytes(64);
        let (a, b) = hash_bytes_both(&mut buf, 0, seed);
        assert_eq!(a, b, "hash_bytes(len=0, seed={seed:#x})");
    }
}

/// CONFIGS rows 2-5: every `switch (len - i)` case and block count for
/// len 0..=64, randomized bytes, many iterations.
#[test]
fn row02_05_hash_bytes_all_lengths() {
    let mut rng = Rng::new(SEED ^ 0x11);
    for _ in 0..200 {
        let mut buf = rng.bytes(96);
        let seed = rng.next_u64() as usize;
        for len in 0..=64usize {
            let (a, b) = hash_bytes_both(&mut buf, len, seed);
            assert_eq!(a, b, "hash_bytes(len={len}, seed={seed:#x}) bytes={:?}", &buf[..len]);
        }
    }
}

/// CONFIGS row 6: the signed-`int` overflow quirk — high bit set in byte 3 and
/// byte 7 of every 8-byte block, plus in the `case 4` / `case 3` tail.
#[test]
fn row06_hash_bytes_signed_overflow_quirk() {
    let mut rng = Rng::new(SEED ^ 0x22);
    for _ in 0..300 {
        let mut buf = rng.bytes(64);
        // force the top bit of bytes 3 and 7 of every block
        for i in 0..64 {
            if i % 8 == 3 || i % 8 == 7 {
                buf[i] |= 0x80;
            }
        }
        let seed = rng.next_u64() as usize;
        for len in 0..=64usize {
            let (a, b) = hash_bytes_both(&mut buf, len, seed);
            assert_eq!(a, b, "quirk hash_bytes(len={len})");
        }
    }
    // and the all-0xFF / all-0x80 extremes
    for pat in [0xFFu8, 0x80, 0x00, 0x7F] {
        let mut buf = vec![pat; 64];
        for len in 0..=64usize {
            for seed in [0usize, usize::MAX, 0x31415926] {
                let (a, b) = hash_bytes_both(&mut buf, len, seed);
                assert_eq!(a, b, "pat={pat:#x} len={len} seed={seed:#x}");
            }
        }
    }
}

/// CONFIGS row 7: seed sweep on fixed bytes.
#[test]
fn row07_hash_bytes_seed_sweep() {
    let mut rng = Rng::new(SEED ^ 0x33);
    let mut buf = rng.bytes(64);
    for _ in 0..2000 {
        let seed = rng.next_u64() as usize;
        for len in [0usize, 1, 7, 8, 9, 15, 16, 33, 64] {
            let (a, b) = hash_bytes_both(&mut buf, len, seed);
            assert_eq!(a, b, "seed sweep len={len} seed={seed:#x}");
        }
    }
    for seed in [0usize, 1, 2, usize::MAX, usize::MAX - 1, 1 << 63, (1 << 63) | 1] {
        for len in 0..=64 {
            let (a, b) = hash_bytes_both(&mut buf, len, seed);
            assert_eq!(a, b, "extreme seed {seed:#x} len={len}");
        }
    }
}

fn hash_string_both(s: &[u8], seed: usize) -> (usize, usize) {
    let (c, r) = libs();
    let mut cs = s.to_vec();
    let mut rs = s.to_vec();
    unsafe {
        (
            (c.hash_string)(cs.as_mut_ptr() as *mut c_char, seed),
            (r.hash_string)(rs.as_mut_ptr() as *mut c_char, seed),
        )
    }
}

/// CONFIGS row 8: empty string.
#[test]
fn row08_hash_string_empty() {
    for seed in [0usize, 1, 0x31415926, usize::MAX, usize::MAX >> 1, 1 << 63] {
        let (a, b) = hash_string_both(b"\0", seed);
        assert_eq!(a, b, "hash_string(\"\", {seed:#x})");
    }
}

/// CONFIGS rows 9 + 11: 1..64 ASCII chars, randomized seeds.
#[test]
fn row09_11_hash_string_ascii() {
    let mut rng = Rng::new(SEED ^ 0x44);
    for _ in 0..300 {
        let seed = rng.next_u64() as usize;
        for n in 1..=64usize {
            let s = rng.cstring(n);
            let (a, b) = hash_string_both(&s, seed);
            assert_eq!(a, b, "hash_string(len={n}, seed={seed:#x})");
        }
    }
    // fixed strings against extreme seeds
    for s in [
        &b"a\0"[..],
        b"ab\0",
        b"test_0\0",
        b"test_-2147483648\0",
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\0",
    ] {
        for seed in [0usize, 1, usize::MAX, 0x31415926, 1 << 63] {
            let (a, b) = hash_string_both(s, seed);
            assert_eq!(a, b, "fixed hash_string seed={seed:#x}");
        }
    }
}

/// CONFIGS row 10: bytes >= 0x80 (the `(unsigned char) *str++` promotion).
#[test]
fn row10_hash_string_high_bytes() {
    let mut rng = Rng::new(SEED ^ 0x55);
    for _ in 0..300 {
        let seed = rng.next_u64() as usize;
        for n in 1..=48usize {
            let mut s = rng.cstring_raw(n);
            // guarantee at least one >= 0x80 byte
            s[0] |= 0x80;
            let (a, b) = hash_string_both(&s, seed);
            assert_eq!(a, b, "hash_string high bytes len={n} seed={seed:#x}");
        }
    }
    for s in [&b"\x80\0"[..], b"\xff\0", b"\xff\xfe\xfd\xfc\0", b"\x80\x80\x80\x80\x80\x80\x80\x80\0"] {
        for seed in [0usize, usize::MAX, 0x31415926] {
            let (a, b) = hash_string_both(s, seed);
            assert_eq!(a, b, "high-byte fixed string");
        }
    }
}

// ---------------------------------------------------------------------------
// stbds_arrgrowf / stbds_arrfreef  (CONFIGS rows 13-19)
// ---------------------------------------------------------------------------

/// Grow with the same parameters on both sides and compare header state.
/// `payload` is written in full after each grow so the compared bytes are
/// always initialised.
fn grow_both(
    state: &mut (*mut c_void, *mut c_void),
    elemsize: usize,
    addlen: usize,
    min_cap: usize,
    fill: u8,
) -> (ArrSnapshot, ArrSnapshot) {
    let (c, r) = libs();
    unsafe {
        let ca = (c.arrgrowf)(state.0, elemsize, addlen, min_cap);
        let ra = (r.arrgrowf)(state.1, elemsize, addlen, min_cap);
        state.0 = ca;
        state.1 = ra;
        assert_eq!(
            ca.is_null(),
            ra.is_null(),
            "null-ness mismatch: arrgrowf(_, {elemsize}, {addlen}, {min_cap}) C={ca:?} RUST={ra:?}"
        );
        if ca.is_null() {
            return (snapshot_arr(ca, 0), snapshot_arr(ra, 0));
        }
        // capacity should match; fill the whole capacity so payload compare is
        // meaningful and deterministic.
        let ccap = (*((ca as *mut u8).sub(HEADER_SIZE) as *const RawArrayHeader)).capacity;
        let rcap = (*((ra as *mut u8).sub(HEADER_SIZE) as *const RawArrayHeader)).capacity;
        assert_eq!(ccap, rcap, "capacity mismatch elemsize={elemsize} addlen={addlen} min_cap={min_cap}");
        let n = elemsize * ccap;
        std::ptr::write_bytes(ca as *mut u8, fill, n);
        std::ptr::write_bytes(ra as *mut u8, fill, n);
        (snapshot_arr(ca, n), snapshot_arr(ra, n))
    }
}

/// CONFIGS rows 13-14: fresh allocation, small `min_cap` (the `< 4` floor),
/// including the `min_cap == 0` case where the C returns NULL unchanged.
#[test]
fn row13_14_arrgrowf_fresh_small() {
    let (c, r) = libs();
    for elemsize in [1usize, 2, 3, 4, 8, 12, 16, 64] {
        for (addlen, min_cap) in [(0usize, 0usize), (0, 1), (0, 2), (0, 3), (0, 4), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0)] {
            let mut st = (std::ptr::null_mut(), std::ptr::null_mut());
            let (a, b) = grow_both(&mut st, elemsize, addlen, min_cap, 0xAB);
            assert_eq!(a, b, "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap})");
            if addlen == 0 && min_cap == 0 {
                // `min_cap (0) <= arrcap(NULL) (0)` -> early `return a` == NULL
                assert!(a.null, "arrgrowf(NULL,e,0,0) must return NULL");
                continue;
            }
            assert!(!a.null);
            assert_eq!(a.length, 0);
            assert_eq!(a.temp, 0);
            assert!(a.hash_table_null);
            unsafe {
                (c.arrfreef)(st.0);
                (r.arrfreef)(st.1);
            }
        }
    }
}

/// CONFIGS row 15: randomized fresh allocations.
#[test]
fn row15_arrgrowf_fresh_random() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0x66);
    for _ in 0..3000 {
        let elemsize = 1 + rng.below(64);
        let addlen = rng.below(1000);
        let min_cap = rng.below(1000);
        let mut st = (std::ptr::null_mut(), std::ptr::null_mut());
        let (a, b) = grow_both(&mut st, elemsize, addlen, min_cap, (rng.next_u64() & 0xFF) as u8);
        assert_eq!(a, b, "arrgrowf(NULL, {elemsize}, {addlen}, {min_cap})");
        unsafe {
            (c.arrfreef)(st.0);
            (r.arrfreef)(st.1);
        }
    }
}

/// CONFIGS rows 16-18: existing array — no-op, doubling, and exact-min_cap
/// branches.
#[test]
fn row16_18_arrgrowf_existing_branches() {
    let (c, r) = libs();
    for elemsize in [1usize, 4, 8, 16, 24] {
        // start with capacity 8
        let mut st = (std::ptr::null_mut(), std::ptr::null_mut());
        let _ = grow_both(&mut st, elemsize, 0, 8, 0x01);
        // row 16: min_cap <= cap -> unchanged (same pointer, no realloc)
        for min_cap in [0usize, 1, 4, 7, 8] {
            let before = unsafe { (snapshot_arr(st.0, elemsize * 8), snapshot_arr(st.1, elemsize * 8)) };
            let p0 = st.0;
            let p1 = st.1;
            let (a, b) = grow_both(&mut st, elemsize, 0, min_cap, 0x01);
            assert_eq!(a, b, "row16 elemsize={elemsize} min_cap={min_cap}");
            assert_eq!(a, before.0, "row16: C array changed");
            assert_eq!(b, before.1, "row16: Rust array changed");
            assert_eq!(st.0, p0, "row16: C pointer moved");
            assert_eq!(st.1, p1, "row16: Rust pointer moved");
        }
        // row 17: cap < min_cap < 2*cap -> doubles to 16
        let (a, b) = grow_both(&mut st, elemsize, 0, 9, 0x02);
        assert_eq!(a, b);
        assert_eq!(a.capacity, 16, "row17 doubling branch");
        // row 18: min_cap >= 2*cap -> exactly min_cap
        let (a, b) = grow_both(&mut st, elemsize, 0, 100, 0x03);
        assert_eq!(a, b);
        assert_eq!(a.capacity, 100, "row18 exact branch");
        unsafe {
            (c.arrfreef)(st.0);
            (r.arrfreef)(st.1);
        }
    }
}

/// CONFIGS row 19: long growth chain, header fields (length/temp/hash_table)
/// preserved across reallocs, then free.
#[test]
fn row19_arrgrowf_chain() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0x77);
    for elemsize in [1usize, 4, 8, 13, 32] {
        let mut st = (std::ptr::null_mut(), std::ptr::null_mut());
        let _ = grow_both(&mut st, elemsize, 1, 0, 0x10);
        // simulate arrput: bump length, write payload, grow when needed
        for step in 0..200usize {
            unsafe {
                let ch = (st.0 as *mut u8).sub(HEADER_SIZE) as *mut RawArrayHeader;
                let rh = (st.1 as *mut u8).sub(HEADER_SIZE) as *mut RawArrayHeader;
                assert_eq!((*ch).length, (*rh).length);
                assert_eq!((*ch).capacity, (*rh).capacity);
                if (*ch).length + 1 > (*ch).capacity {
                    let n = rng.below(3) + 1;
                    let (a, b) = grow_both(&mut st, elemsize, n, 0, (step & 0xFF) as u8);
                    assert_eq!(a, b, "chain grow step={step} elemsize={elemsize}");
                }
                let ch = (st.0 as *mut u8).sub(HEADER_SIZE) as *mut RawArrayHeader;
                let rh = (st.1 as *mut u8).sub(HEADER_SIZE) as *mut RawArrayHeader;
                (*ch).length += 1;
                (*rh).length += 1;
                (*ch).temp = step as isize;
                (*rh).temp = step as isize;
            }
        }
        unsafe {
            let n = elemsize * (*((st.0 as *mut u8).sub(HEADER_SIZE) as *const RawArrayHeader)).capacity;
            assert_eq!(snapshot_arr(st.0, n), snapshot_arr(st.1, n), "chain final elemsize={elemsize}");
            (c.arrfreef)(st.0);
            (r.arrfreef)(st.1);
        }
    }
}

/// CONFIGS row 65: `strkey`.
#[test]
fn row65_strkey() {
    let (c, r) = libs();
    let mut rng = Rng::new(SEED ^ 0x88);
    let mut cases: Vec<i32> = vec![0, 1, -1, 9, 11, 12345, -12345, i32::MAX, i32::MIN, 10, -10, 100000, -999999999];
    for _ in 0..200 {
        cases.push(rng.next_u32() as i32);
    }
    for n in cases {
        unsafe {
            let cb = cstr_bytes((c.strkey)(n));
            let rb = cstr_bytes((r.strkey)(n));
            assert_eq!(cb, rb, "strkey({n}): C={:?} RUST={:?}",
                String::from_utf8_lossy(&cb), String::from_utf8_lossy(&rb));
            // and the expected C `sprintf("test_%d")` bytes
            assert_eq!(cb, format!("test_{n}").into_bytes(), "strkey({n}) unexpected");
        }
    }
}

/// CONFIGS rows 5-7 extension: many-block inputs (len 65..=320) for
/// `stbds_hash_bytes`, and long strings for `stbds_hash_string`. The
/// per-8-byte-block loop runs 8..40 times here, which the len<=64 sweep above
/// does not reach.
#[test]
fn row05_07_hash_long_inputs() {
    let mut rng = Rng::new(SEED ^ 0x99);
    for _ in 0..60 {
        let mut buf = rng.bytes(384);
        // half the runs force the high bit on the sign-sensitive byte positions
        if rng.below(2) == 0 {
            for i in 0..384 {
                if i % 8 == 3 || i % 8 == 7 {
                    buf[i] |= 0x80;
                }
            }
        }
        let seed = rng.next_u64() as usize;
        for len in 65..=320usize {
            let (a, b) = hash_bytes_both(&mut buf, len, seed);
            assert_eq!(a, b, "long hash_bytes(len={len}, seed={seed:#x})");
        }
    }
    // long strings, ASCII and high-byte
    for _ in 0..60 {
        let seed = rng.next_u64() as usize;
        for n in [65usize, 100, 127, 128, 129, 255, 256, 511, 1000] {
            let s = rng.cstring(n);
            let (a, b) = hash_string_both(&s, seed);
            assert_eq!(a, b, "long hash_string(ascii len={n}, seed={seed:#x})");
            let s = rng.cstring_raw(n);
            let (a, b) = hash_string_both(&s, seed);
            assert_eq!(a, b, "long hash_string(raw len={n}, seed={seed:#x})");
        }
    }
}
