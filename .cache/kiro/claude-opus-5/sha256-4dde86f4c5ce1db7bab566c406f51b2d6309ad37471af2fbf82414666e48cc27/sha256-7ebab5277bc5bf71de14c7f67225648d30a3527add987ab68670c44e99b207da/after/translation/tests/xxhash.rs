//! xxhash (namespaced `LZ4_XXH*`) — lowest level primitives.
mod common;

use common::*;
use std::os::raw::c_void;

const SEEDS: [u32; 6] = [0, 1, 0xDEADBEEF, 0x9E3779B1, u32::MAX, 0x12345678];
const SEEDS64: [u64; 7] = [
    0,
    1,
    0xDEAD_BEEF,
    0x9E37_79B9_7F4A_7C15,
    u64::MAX,
    0x0123_4567_89AB_CDEF,
    1 << 63,
];

fn all_lens() -> Vec<usize> {
    let mut v: Vec<usize> = (0usize..=80).collect();
    v.extend_from_slice(&[
        100, 127, 128, 129, 200, 255, 256, 257, 511, 512, 1000, 1024, 4095, 4096, 5000, 65535,
        65536, 100000,
    ]);
    v
}

#[test]
fn xxh32_oneshot() {
    let (cf, rf) = pair!("LZ4_XXH32", fn(*const c_void, usize, u32) -> u32);
    let big = gen_mixed(100_001, 7);
    unsafe {
        for &len in &all_lens() {
            let data = &big[..len];
            for &s in &SEEDS {
                let a = cf(data.as_ptr() as *const c_void, len, s);
                let b = rf(data.as_ptr() as *const c_void, len, s);
                assert_eq!(a, b, "XXH32 len={} seed={:#x}", len, s);
            }
        }
        // NULL + len 0
        assert_eq!(cf(std::ptr::null(), 0, 5), rf(std::ptr::null(), 0, 5));
        // unaligned starts
        for off in 1..8usize {
            for &len in &[0usize, 1, 3, 15, 16, 17, 33, 64, 1000] {
                let p = big[off..].as_ptr() as *const c_void;
                assert_eq!(cf(p, len, 0), rf(p, len, 0), "XXH32 off={} len={}", off, len);
            }
        }
    }
}

#[test]
fn xxh64_oneshot() {
    let (cf, rf) = pair!("LZ4_XXH64", fn(*const c_void, usize, u64) -> u64);
    let big = gen_mixed(100_001, 11);
    unsafe {
        for &len in &all_lens() {
            let data = &big[..len];
            for &s in &SEEDS64 {
                let a = cf(data.as_ptr() as *const c_void, len, s);
                let b = rf(data.as_ptr() as *const c_void, len, s);
                assert_eq!(a, b, "XXH64 len={} seed={:#x}", len, s);
            }
        }
        assert_eq!(cf(std::ptr::null(), 0, 5), rf(std::ptr::null(), 0, 5));
        for off in 1..8usize {
            for &len in &[0usize, 1, 3, 15, 16, 17, 33, 64, 1000] {
                let p = big[off..].as_ptr() as *const c_void;
                assert_eq!(cf(p, len, 0), rf(p, len, 0), "XXH64 off={} len={}", off, len);
            }
        }
    }
}

#[test]
fn xxh32_streaming() {
    let (c_new, r_new) = pair!("LZ4_XXH32_createState", fn() -> *mut c_void);
    let (c_free, r_free) = pair!("LZ4_XXH32_freeState", fn(*mut c_void) -> i32);
    let (c_reset, r_reset) = pair!("LZ4_XXH32_reset", fn(*mut c_void, u32) -> i32);
    let (c_upd, r_upd) = pair!("LZ4_XXH32_update", fn(*mut c_void, *const c_void, usize) -> i32);
    let (c_dig, r_dig) = pair!("LZ4_XXH32_digest", fn(*const c_void) -> u32);
    let (c_copy, r_copy) = pair!("LZ4_XXH32_copyState", fn(*mut c_void, *const c_void));

    let data = gen_mixed(70_000, 3);
    let chunkings: [&[usize]; 7] = [
        &[1],
        &[2, 3],
        &[15, 1, 16],
        &[16],
        &[17, 5, 100],
        &[4096, 7, 1],
        &[65535, 1, 2, 3],
    ];
    unsafe {
        let cs = c_new();
        let rs = r_new();
        assert!(!cs.is_null() && !rs.is_null());
        let cs2 = c_new();
        let rs2 = r_new();

        for &s in &SEEDS {
            for cks in &chunkings {
                assert_eq!(c_reset(cs, s), r_reset(rs, s));
                let mut pos = 0usize;
                let mut i = 0usize;
                let mut copied = false;
                while pos < data.len() {
                    let n = cks[i % cks.len()].min(data.len() - pos);
                    i += 1;
                    let p = data[pos..].as_ptr() as *const c_void;
                    assert_eq!(c_upd(cs, p, n), r_upd(rs, p, n));
                    pos += n;
                    assert_eq!(c_dig(cs), r_dig(rs), "intermediate digest seed={:#x}", s);
                    if !copied && pos > 300 {
                        copied = true;
                        c_copy(cs2, cs);
                        r_copy(rs2, rs);
                        assert_eq!(c_dig(cs2), r_dig(rs2), "copied state digest");
                    }
                }
                assert_eq!(c_dig(cs), r_dig(rs), "final digest seed={:#x}", s);
            }
        }
        // NULL / zero-length updates
        assert_eq!(c_reset(cs, 0), r_reset(rs, 0));
        assert_eq!(
            c_upd(cs, std::ptr::null(), 0),
            r_upd(rs, std::ptr::null(), 0)
        );
        assert_eq!(c_dig(cs), r_dig(rs));

        assert_eq!(c_free(cs), r_free(rs));
        assert_eq!(c_free(cs2), r_free(rs2));
        assert_eq!(c_free(std::ptr::null_mut()), r_free(std::ptr::null_mut()));
    }
}

#[test]
fn xxh64_streaming() {
    let (c_new, r_new) = pair!("LZ4_XXH64_createState", fn() -> *mut c_void);
    let (c_free, r_free) = pair!("LZ4_XXH64_freeState", fn(*mut c_void) -> i32);
    let (c_reset, r_reset) = pair!("LZ4_XXH64_reset", fn(*mut c_void, u64) -> i32);
    let (c_upd, r_upd) = pair!("LZ4_XXH64_update", fn(*mut c_void, *const c_void, usize) -> i32);
    let (c_dig, r_dig) = pair!("LZ4_XXH64_digest", fn(*const c_void) -> u64);
    let (c_copy, r_copy) = pair!("LZ4_XXH64_copyState", fn(*mut c_void, *const c_void));

    let data = gen_mixed(70_000, 23);
    let chunkings: [&[usize]; 7] = [
        &[1],
        &[2, 3],
        &[31, 1, 32],
        &[32],
        &[33, 5, 100],
        &[4096, 7, 1],
        &[65535, 1, 2, 3],
    ];
    unsafe {
        let cs = c_new();
        let rs = r_new();
        let cs2 = c_new();
        let rs2 = r_new();
        for &s in &SEEDS64 {
            for cks in &chunkings {
                assert_eq!(c_reset(cs, s), r_reset(rs, s));
                let mut pos = 0usize;
                let mut i = 0usize;
                let mut copied = false;
                while pos < data.len() {
                    let n = cks[i % cks.len()].min(data.len() - pos);
                    i += 1;
                    let p = data[pos..].as_ptr() as *const c_void;
                    assert_eq!(c_upd(cs, p, n), r_upd(rs, p, n));
                    pos += n;
                    assert_eq!(c_dig(cs), r_dig(rs));
                    if !copied && pos > 300 {
                        copied = true;
                        c_copy(cs2, cs);
                        r_copy(rs2, rs);
                        assert_eq!(c_dig(cs2), r_dig(rs2));
                    }
                }
                assert_eq!(c_dig(cs), r_dig(rs), "final digest seed={:#x}", s);
            }
        }
        assert_eq!(c_free(cs), r_free(rs));
        assert_eq!(c_free(cs2), r_free(rs2));
        assert_eq!(c_free(std::ptr::null_mut()), r_free(std::ptr::null_mut()));
    }
}

#[test]
fn xxh_canonical() {
    unsafe {
        {
            let (cf, rf) = pair!("LZ4_XXH32_canonicalFromHash", fn(*mut u8, u32));
            let (cg, rg) = pair!("LZ4_XXH32_hashFromCanonical", fn(*const u8) -> u32);
            for h in [0u32, 1, 0x0102_0304, 0xFFFF_FFFF, 0x8000_0000, 12345] {
                let mut a = [0u8; 4];
                let mut b = [0u8; 4];
                cf(a.as_mut_ptr(), h);
                rf(b.as_mut_ptr(), h);
                assert_eq!(a, b, "XXH32_canonicalFromHash({:#x})", h);
                assert_eq!(cg(a.as_ptr()), rg(b.as_ptr()));
                assert_eq!(cg(a.as_ptr()), h);
            }
        }
        {
            let (cf, rf) = pair!("LZ4_XXH64_canonicalFromHash", fn(*mut u8, u64));
            let (cg, rg) = pair!("LZ4_XXH64_hashFromCanonical", fn(*const u8) -> u64);
            for h in [
                0u64,
                1,
                0x0102_0304_0506_0708,
                u64::MAX,
                1 << 63,
                123_456_789,
            ] {
                let mut a = [0u8; 8];
                let mut b = [0u8; 8];
                cf(a.as_mut_ptr(), h);
                rf(b.as_mut_ptr(), h);
                assert_eq!(a, b, "XXH64_canonicalFromHash({:#x})", h);
                assert_eq!(cg(a.as_ptr()), rg(b.as_ptr()));
                assert_eq!(cg(a.as_ptr()), h);
            }
        }
    }
}

/// The internal state must be byte-identical too, so that `copyState` and any
/// struct-level assumptions hold.
#[test]
fn xxh_state_bytes_match() {
    let (c_reset32, r_reset32) = pair!("LZ4_XXH32_reset", fn(*mut u8, u32) -> i32);
    let (c_upd32, r_upd32) = pair!("LZ4_XXH32_update", fn(*mut u8, *const c_void, usize) -> i32);
    let (c_reset64, r_reset64) = pair!("LZ4_XXH64_reset", fn(*mut u8, u64) -> i32);
    let (c_upd64, r_upd64) = pair!("LZ4_XXH64_update", fn(*mut u8, *const c_void, usize) -> i32);

    let data = gen_mixed(5000, 99);
    // XXH32_state_t is 48 bytes, XXH64_state_t is 88; use generous aligned space.
    unsafe {
        for &n in &[0usize, 1, 15, 16, 17, 64, 100, 4096, 5000] {
            let mut a = Aligned::new(256);
            let mut b = Aligned::new(256);
            assert_eq!(c_reset32(a.ptr(), 7), r_reset32(b.ptr(), 7));
            let p = data.as_ptr() as *const c_void;
            assert_eq!(c_upd32(a.ptr(), p, n), r_upd32(b.ptr(), p, n));
            assert_eq!(
                a.as_slice()[..48],
                b.as_slice()[..48],
                "XXH32 state after {} bytes",
                n
            );

            let mut a = Aligned::new(256);
            let mut b = Aligned::new(256);
            assert_eq!(c_reset64(a.ptr(), 7), r_reset64(b.ptr(), 7));
            assert_eq!(c_upd64(a.ptr(), p, n), r_upd64(b.ptr(), p, n));
            assert_eq!(
                a.as_slice()[..88],
                b.as_slice()[..88],
                "XXH64 state after {} bytes",
                n
            );
        }
    }
}
