//! Phase B — valid-path differential tests, rows 1..14 of CONFIGS.md:
//! `stbds_arrgrowf`, `stbds_arrfreef`, `stbds_hash_bytes`, `stbds_hash_string`,
//! `stbds_rand_seed`.

mod common;

use common::*;
use std::ffi::{c_char, c_void};

const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

// ---------------------------------------------------------------------------
// Row 0 (harness smoke test): symbol parity through dlopen
// ---------------------------------------------------------------------------
#[test]
fn row_00_both_libraries_load() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "RUST");
}

// ---------------------------------------------------------------------------
// Rows 1..6: stbds_arrgrowf / stbds_arrfreef
// ---------------------------------------------------------------------------

/// Row 1 — no-op path: `min_cap <= arrcap(a)` with `a == NULL`.
#[test]
fn row_01_arrgrowf_noop_null() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 24, 128] {
            let pc = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            let pr = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0);
            assert!(pc.is_null(), "C returned non-null for the no-op path");
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "elemsize={elemsize}: null-ness differs"
            );
        }
    }
}

/// Row 2 — `min_cap` clamped up to 4 when the array is fresh.
#[test]
fn row_02_arrgrowf_min_cap_4() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 24, 128] {
            for addlen in 0..=4usize {
                for min_cap in 0..=4usize {
                    if addlen == 0 && min_cap == 0 {
                        continue; // row 1
                    }
                    let pc = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    let pr = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    assert!(!pc.is_null() && !pr.is_null());
                    let hc = header_of(pc);
                    let hr = header_of(pr);
                    assert_eq!(
                        (hc.length, hc.capacity, hc.temp, hc.hash_table.is_null()),
                        (hr.length, hr.capacity, hr.temp, hr.hash_table.is_null()),
                        "elemsize={elemsize} addlen={addlen} min_cap={min_cap}"
                    );
                    (c.arrfreef)(pc);
                    (r.arrfreef)(pr);
                }
            }
        }
    }
}

/// Row 3 — fresh array, random explicit `min_cap`.
#[test]
fn row_03_arrgrowf_random_min_cap() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 3);
    unsafe {
        for _ in 0..2000 {
            let elemsize = [1usize, 2, 4, 8, 16, 24, 128][rng.below(7)];
            let min_cap = rng.below(4096);
            let addlen = rng.below(64);
            let pc = (c.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
            let pr = (r.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
            if pc.is_null() {
                assert!(pr.is_null());
                continue;
            }
            let hc = header_of(pc);
            let hr = header_of(pr);
            assert_eq!(
                (hc.length, hc.capacity, hc.temp, hc.hash_table.is_null()),
                (hr.length, hr.capacity, hr.temp, hr.hash_table.is_null()),
                "elemsize={elemsize} addlen={addlen} min_cap={min_cap}"
            );
            (c.arrfreef)(pc);
            (r.arrfreef)(pr);
        }
    }
}

/// Row 4 — doubling chain driven by repeated `addlen=1` growth, with the
/// caller maintaining `length` exactly as the `stbds_arrput` macro does.
#[test]
fn row_04_arrgrowf_doubling_chain() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 24] {
            let mut pc: *mut c_void = std::ptr::null_mut();
            let mut pr: *mut c_void = std::ptr::null_mut();
            for step in 0..300usize {
                // stbds_arrmaybegrow(a,1)
                let need = |p: *mut c_void| -> bool {
                    p.is_null() || header_of(p).length + 1 > header_of(p).capacity
                };
                if need(pc) {
                    pc = (c.arrgrowf)(pc, elemsize, 1, 0);
                }
                if need(pr) {
                    pr = (r.arrgrowf)(pr, elemsize, 1, 0);
                }
                let hc = header_of(pc);
                let hr = header_of(pr);
                assert_eq!(
                    (hc.length, hc.capacity, hc.temp),
                    (hr.length, hr.capacity, hr.temp),
                    "elemsize={elemsize} step={step}"
                );
                // write a byte pattern and bump length
                std::ptr::write_bytes(
                    (pc as *mut u8).add(elemsize * hc.length),
                    (step & 0xff) as u8,
                    elemsize,
                );
                std::ptr::write_bytes(
                    (pr as *mut u8).add(elemsize * hr.length),
                    (step & 0xff) as u8,
                    elemsize,
                );
                (*(pc as *mut ArrayHeader).sub(1)).length += 1;
                (*(pr as *mut ArrayHeader).sub(1)).length += 1;
            }
            let n = header_of(pc).length * elemsize;
            let bc = std::slice::from_raw_parts(pc as *const u8, n);
            let br = std::slice::from_raw_parts(pr as *const u8, n);
            assert_eq!(bc, br, "elemsize={elemsize}: element bytes differ");
            (c.arrfreef)(pc);
            (r.arrfreef)(pr);
        }
    }
}

/// Row 5 — grow an existing array with `min_cap` far above `2*cap`.
#[test]
fn row_05_arrgrowf_explicit_large_min_cap() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 5);
    unsafe {
        for _ in 0..500 {
            let elemsize = [4usize, 8, 16, 24][rng.below(4)];
            let mut pc = (c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            let mut pr = (r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
            for _ in 0..5 {
                let min_cap = rng.below(1 << 14);
                let addlen = rng.below(32);
                pc = (c.arrgrowf)(pc, elemsize, addlen, min_cap);
                pr = (r.arrgrowf)(pr, elemsize, addlen, min_cap);
                let hc = header_of(pc);
                let hr = header_of(pr);
                assert_eq!(
                    (hc.length, hc.capacity, hc.temp),
                    (hr.length, hr.capacity, hr.temp),
                    "elemsize={elemsize} addlen={addlen} min_cap={min_cap}"
                );
            }
            (c.arrfreef)(pc);
            (r.arrfreef)(pr);
        }
    }
}

/// Row 6 — alloc/free round trip preserving element bytes across growth.
#[test]
fn row_06_arrgrowf_preserves_contents() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 6);
    unsafe {
        for _ in 0..200 {
            let elemsize = [1usize, 3, 4, 8, 16, 24][rng.below(6)];
            let n = 1 + rng.below(200);
            let data = rng.bytes(elemsize * n);
            let mut pc: *mut c_void = std::ptr::null_mut();
            let mut pr: *mut c_void = std::ptr::null_mut();
            for i in 0..n {
                if pc.is_null() || header_of(pc).length + 1 > header_of(pc).capacity {
                    pc = (c.arrgrowf)(pc, elemsize, 1, 0);
                }
                if pr.is_null() || header_of(pr).length + 1 > header_of(pr).capacity {
                    pr = (r.arrgrowf)(pr, elemsize, 1, 0);
                }
                let off = elemsize * i;
                std::ptr::copy_nonoverlapping(
                    data.as_ptr().add(off),
                    (pc as *mut u8).add(off),
                    elemsize,
                );
                std::ptr::copy_nonoverlapping(
                    data.as_ptr().add(off),
                    (pr as *mut u8).add(off),
                    elemsize,
                );
                (*(pc as *mut ArrayHeader).sub(1)).length += 1;
                (*(pr as *mut ArrayHeader).sub(1)).length += 1;
            }
            assert_eq!(header_of(pc).capacity, header_of(pr).capacity);
            let bc = std::slice::from_raw_parts(pc as *const u8, elemsize * n);
            let br = std::slice::from_raw_parts(pr as *const u8, elemsize * n);
            assert_eq!(bc, &data[..]);
            assert_eq!(br, &data[..]);
            (c.arrfreef)(pc);
            (r.arrfreef)(pr);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7..11: stbds_hash_bytes
// ---------------------------------------------------------------------------

fn hash_bytes_both(c: &Lib, r: &Lib, buf: &[u8], len: usize, seed: usize) -> (usize, usize) {
    unsafe {
        let mut b = buf.to_vec();
        let hc = (c.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
        let hr = (r.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, seed);
        (hc, hr)
    }
}

/// Row 7 — `len == 0`.
#[test]
fn row_07_hash_bytes_len_zero() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let buf = [0u8; 8];
    for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
        let (hc, hr) = hash_bytes_both(c, r, &buf, 0, seed);
        assert_eq!(hc, hr, "seed={seed:#x}");
    }
}

/// Row 8 — `len` 1..7: every `switch` fall-through case.
#[test]
fn row_08_hash_bytes_tail_lengths() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..4000 {
        let buf = rng.bytes(8);
        let len = 1 + rng.below(7);
        let seed = rng.next_u64() as usize;
        let (hc, hr) = hash_bytes_both(c, r, &buf, len, seed);
        assert_eq!(hc, hr, "len={len} seed={seed:#x} buf={buf:02x?}");
    }
}

/// Row 9 — `len` 8..64: main loop plus every tail.
#[test]
fn row_09_hash_bytes_long() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..4000 {
        let len = 8 + rng.below(57);
        let buf = rng.bytes(len);
        let seed = rng.next_u64() as usize;
        let (hc, hr) = hash_bytes_both(c, r, &buf, len, seed);
        assert_eq!(hc, hr, "len={len} seed={seed:#x}");
    }
}

/// Row 10 — the `d[3] << 24` sign-extension quirk: force bit 7 of byte 3 of
/// every 8-byte block, and of the tail's byte 3.
#[test]
fn row_10_hash_bytes_sign_extension() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 10);
    for _ in 0..4000 {
        let len = rng.below(65);
        let mut buf = rng.bytes(len.max(1));
        for i in (3..buf.len()).step_by(8) {
            buf[i] |= 0x80;
        }
        let seed = rng.next_u64() as usize;
        let (hc, hr) = hash_bytes_both(c, r, &buf, len, seed);
        assert_eq!(hc, hr, "len={len} seed={seed:#x} buf={buf:02x?}");
    }
    // and the mirror: byte 3 of every block cleared
    for _ in 0..2000 {
        let len = rng.below(65);
        let mut buf = rng.bytes(len.max(1));
        for i in (3..buf.len()).step_by(8) {
            buf[i] &= 0x7f;
        }
        let seed = rng.next_u64() as usize;
        let (hc, hr) = hash_bytes_both(c, r, &buf, len, seed);
        assert_eq!(hc, hr, "len={len} seed={seed:#x}");
    }
}

/// Row 11 — extreme seeds (the seed is XOR-cancelled by the C initialisers, but
/// both sides must agree regardless).
#[test]
fn row_11_hash_bytes_seed_axis() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 11);
    let seeds = [
        0usize,
        1,
        0x3141_5926,
        usize::MAX,
        usize::MAX - 1,
        1 << 63,
        0xAAAA_AAAA_AAAA_AAAA,
    ];
    for &seed in &seeds {
        for len in 0..40usize {
            let buf = rng.bytes(len.max(1));
            let (hc, hr) = hash_bytes_both(c, r, &buf, len, seed);
            assert_eq!(hc, hr, "seed={seed:#x} len={len}");
        }
    }
    // The C code XORs `seed` in twice, so it must not affect the result. Verify
    // that observable property holds for BOTH libraries identically.
    let buf = rng.bytes(37);
    let (h0, _) = hash_bytes_both(c, r, &buf, 37, 0);
    for &seed in &seeds {
        let (hc, hr) = hash_bytes_both(c, r, &buf, 37, seed);
        assert_eq!(hc, h0, "C: seed unexpectedly changed the hash");
        assert_eq!(hr, h0, "Rust: seed unexpectedly changed the hash");
    }
}

// ---------------------------------------------------------------------------
// Rows 12..13: stbds_hash_string
// ---------------------------------------------------------------------------

fn hash_string_both(c: &Lib, r: &Lib, s: &[u8], seed: usize) -> (usize, usize) {
    unsafe {
        let mut b = s.to_vec();
        assert_eq!(*b.last().unwrap(), 0, "needs NUL terminator");
        let hc = (c.hash_string)(b.as_mut_ptr() as *mut c_char, seed);
        let hr = (r.hash_string)(b.as_mut_ptr() as *mut c_char, seed);
        (hc, hr)
    }
}

/// Row 12 — empty, short, long, and high-bit-byte strings.
#[test]
fn row_12_hash_string_shapes() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 12);

    for fixed in [
        &b"\0"[..],
        &b"a\0"[..],
        &b"\xff\0"[..],
        &b"\x80\x80\x80\0"[..],
        &b"test_0\0"[..],
        &b"the quick brown fox jumps over the lazy dog\0"[..],
    ] {
        for seed in [0usize, 1, 0x3141_5926, usize::MAX] {
            let (hc, hr) = hash_string_both(c, r, fixed, seed);
            assert_eq!(hc, hr, "s={fixed:02x?} seed={seed:#x}");
        }
    }

    for _ in 0..4000 {
        let n = rng.below(64);
        let s = rng.cstr_bytes(n);
        let seed = rng.next_u64() as usize;
        let (hc, hr) = hash_string_both(c, r, &s, seed);
        assert_eq!(hc, hr, "n={n} seed={seed:#x}");
    }
}

/// Row 13 — seed axis for `hash_string` (here the seed IS mixed in).
#[test]
fn row_13_hash_string_seed_axis() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    let mut rng = Rng::new(SEED ^ 13);
    let seeds = [
        0usize,
        1,
        0x3141_5926,
        usize::MAX,
        1 << 63,
        0x5555_5555_5555_5555,
    ];
    for &seed in &seeds {
        for n in 0..48usize {
            let s = rng.cstr_bytes(n);
            let (hc, hr) = hash_string_both(c, r, &s, seed);
            assert_eq!(hc, hr, "seed={seed:#x} n={n}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14: stbds_rand_seed and the advancing global seed
// ---------------------------------------------------------------------------

/// Row 14 — `rand_seed` sets the global; each fresh table consumes it and
/// advances it by `seed = seed*a + b`. Observe the whole sequence through
/// `shmode_func`, which stores the consumed seed in the table.
#[test]
fn row_14_rand_seed_sequence() {
    let __b = both();
    let (c, r) = (&__b.c, &__b.r);
    unsafe {
        for start in [0usize, 1, 0x3141_5926, usize::MAX, 0xDEAD_BEEF_CAFE_BABE] {
            (c.rand_seed)(start);
            (r.rand_seed)(start);
            for i in 0..16 {
                let hc = (c.shmode_func)(16, STBDS_SH_ARENA);
                let hr = (r.shmode_func)(16, STBDS_SH_ARENA);
                let sc = snapshot_map(hc, 16, KeyRepr::Inline);
                let sr = snapshot_map(hr, 16, KeyRepr::Inline);
                assert_eq!(sc, sr, "start={start:#x} iteration={i}");
                (c.hmfree_func)((hc as *mut u8).sub(16) as *mut c_void, 16);
                (r.hmfree_func)((hr as *mut u8).sub(16) as *mut c_void, 16);
            }
        }
    }
}
