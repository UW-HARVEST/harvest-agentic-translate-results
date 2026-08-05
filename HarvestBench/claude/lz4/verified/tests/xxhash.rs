// Phase B/C — xxHash (namespaced LZ4_XXH*) differential tests.
mod common;

use common::*;
use std::os::raw::{c_int, c_void};

type XXH32 = unsafe extern "C" fn(*const c_void, usize, u32) -> u32;
type XXH64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
type CreateState = unsafe extern "C" fn() -> *mut c_void;
type FreeState = unsafe extern "C" fn(*mut c_void) -> c_int;
type Reset32 = unsafe extern "C" fn(*mut c_void, u32) -> c_int;
type Reset64 = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type Update = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
type Digest32 = unsafe extern "C" fn(*const c_void) -> u32;
type Digest64 = unsafe extern "C" fn(*const c_void) -> u64;
type CopyState = unsafe extern "C" fn(*mut c_void, *const c_void);
type CanonicalFromHash32 = unsafe extern "C" fn(*mut c_void, u32);
type CanonicalFromHash64 = unsafe extern "C" fn(*mut c_void, u64);
type HashFromCanonical32 = unsafe extern "C" fn(*const c_void) -> u32;
type HashFromCanonical64 = unsafe extern "C" fn(*const c_void) -> u64;

fn sizes() -> Vec<usize> {
    vec![0, 1, 3, 4, 7, 8, 15, 16, 31, 32, 33, 63, 64, 100, 255, 1000, 4096, 100000]
}

#[test]
fn test_versions() {
    let libs = Libs::load();
    unsafe {
        let cv: libloading::Symbol<unsafe extern "C" fn() -> u32> = csym(&libs, b"LZ4_XXH_versionNumber");
        let rv: libloading::Symbol<unsafe extern "C" fn() -> u32> = rsym(&libs, b"LZ4_XXH_versionNumber");
        assert_eq!(cv(), rv());
    }
}

#[test]
fn test_xxh32_oneshot() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x3232);
    unsafe {
        let c: libloading::Symbol<XXH32> = csym(&libs, b"LZ4_XXH32");
        let r: libloading::Symbol<XXH32> = rsym(&libs, b"LZ4_XXH32");
        for &sz in sizes().iter() {
            for _ in 0..6 {
                let data = rng.random(sz);
                let seed = rng.next_u32();
                let cv = c(data.as_ptr() as *const c_void, sz, seed);
                let rv = r(data.as_ptr() as *const c_void, sz, seed);
                assert_eq!(cv, rv, "XXH32 sz={} seed={}", sz, seed);
            }
        }
    }
}

#[test]
fn test_xxh64_oneshot() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x6464);
    unsafe {
        let c: libloading::Symbol<XXH64> = csym(&libs, b"LZ4_XXH64");
        let r: libloading::Symbol<XXH64> = rsym(&libs, b"LZ4_XXH64");
        for &sz in sizes().iter() {
            for _ in 0..6 {
                let data = rng.random(sz);
                let seed = rng.next_u64();
                let cv = c(data.as_ptr() as *const c_void, sz, seed);
                let rv = r(data.as_ptr() as *const c_void, sz, seed);
                assert_eq!(cv, rv, "XXH64 sz={} seed={}", sz, seed);
            }
        }
    }
}

#[test]
fn test_xxh32_streaming() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x3200);
    unsafe {
        let c_cs: libloading::Symbol<CreateState> = csym(&libs, b"LZ4_XXH32_createState");
        let r_cs: libloading::Symbol<CreateState> = rsym(&libs, b"LZ4_XXH32_createState");
        let c_fs: libloading::Symbol<FreeState> = csym(&libs, b"LZ4_XXH32_freeState");
        let r_fs: libloading::Symbol<FreeState> = rsym(&libs, b"LZ4_XXH32_freeState");
        let c_reset: libloading::Symbol<Reset32> = csym(&libs, b"LZ4_XXH32_reset");
        let r_reset: libloading::Symbol<Reset32> = rsym(&libs, b"LZ4_XXH32_reset");
        let c_upd: libloading::Symbol<Update> = csym(&libs, b"LZ4_XXH32_update");
        let r_upd: libloading::Symbol<Update> = rsym(&libs, b"LZ4_XXH32_update");
        let c_dig: libloading::Symbol<Digest32> = csym(&libs, b"LZ4_XXH32_digest");
        let r_dig: libloading::Symbol<Digest32> = rsym(&libs, b"LZ4_XXH32_digest");

        for total in [0usize, 1, 10, 100, 5000, 100000] {
            let data = rng.random(total);
            let seed = rng.next_u32();
            let cst = c_cs();
            let rst = r_cs();
            c_reset(cst, seed);
            r_reset(rst, seed);
            // feed in random-sized chunks
            let mut off = 0;
            while off < total {
                let chunk = (1 + rng.range(total - off + 1)).min(total - off);
                let cu = c_upd(cst, data.as_ptr().add(off) as *const c_void, chunk);
                let ru = r_upd(rst, data.as_ptr().add(off) as *const c_void, chunk);
                assert_eq!(cu, ru, "update ret");
                off += chunk;
            }
            assert_eq!(c_dig(cst), r_dig(rst), "XXH32 streaming digest total={}", total);
            c_fs(cst);
            r_fs(rst);
        }
    }
}

#[test]
fn test_xxh64_streaming() {
    let libs = Libs::load();
    let mut rng = Rng::new(0x6400);
    unsafe {
        let c_cs: libloading::Symbol<CreateState> = csym(&libs, b"LZ4_XXH64_createState");
        let r_cs: libloading::Symbol<CreateState> = rsym(&libs, b"LZ4_XXH64_createState");
        let c_fs: libloading::Symbol<FreeState> = csym(&libs, b"LZ4_XXH64_freeState");
        let r_fs: libloading::Symbol<FreeState> = rsym(&libs, b"LZ4_XXH64_freeState");
        let c_reset: libloading::Symbol<Reset64> = csym(&libs, b"LZ4_XXH64_reset");
        let r_reset: libloading::Symbol<Reset64> = rsym(&libs, b"LZ4_XXH64_reset");
        let c_upd: libloading::Symbol<Update> = csym(&libs, b"LZ4_XXH64_update");
        let r_upd: libloading::Symbol<Update> = rsym(&libs, b"LZ4_XXH64_update");
        let c_dig: libloading::Symbol<Digest64> = csym(&libs, b"LZ4_XXH64_digest");
        let r_dig: libloading::Symbol<Digest64> = rsym(&libs, b"LZ4_XXH64_digest");

        for total in [0usize, 1, 10, 100, 5000, 100000] {
            let data = rng.random(total);
            let seed = rng.next_u64();
            let cst = c_cs();
            let rst = r_cs();
            c_reset(cst, seed);
            r_reset(rst, seed);
            let mut off = 0;
            while off < total {
                let chunk = (1 + rng.range(total - off + 1)).min(total - off);
                c_upd(cst, data.as_ptr().add(off) as *const c_void, chunk);
                r_upd(rst, data.as_ptr().add(off) as *const c_void, chunk);
                off += chunk;
            }
            assert_eq!(c_dig(cst), r_dig(rst), "XXH64 streaming digest total={}", total);
            c_fs(cst);
            r_fs(rst);
        }
    }
}

#[test]
fn test_canonical_roundtrip() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xca00);
    unsafe {
        let c_cf32: libloading::Symbol<CanonicalFromHash32> = csym(&libs, b"LZ4_XXH32_canonicalFromHash");
        let r_cf32: libloading::Symbol<CanonicalFromHash32> = rsym(&libs, b"LZ4_XXH32_canonicalFromHash");
        let c_hf32: libloading::Symbol<HashFromCanonical32> = csym(&libs, b"LZ4_XXH32_hashFromCanonical");
        let r_hf32: libloading::Symbol<HashFromCanonical32> = rsym(&libs, b"LZ4_XXH32_hashFromCanonical");
        let c_cf64: libloading::Symbol<CanonicalFromHash64> = csym(&libs, b"LZ4_XXH64_canonicalFromHash");
        let r_cf64: libloading::Symbol<CanonicalFromHash64> = rsym(&libs, b"LZ4_XXH64_canonicalFromHash");
        let c_hf64: libloading::Symbol<HashFromCanonical64> = csym(&libs, b"LZ4_XXH64_hashFromCanonical");
        let r_hf64: libloading::Symbol<HashFromCanonical64> = rsym(&libs, b"LZ4_XXH64_hashFromCanonical");

        for _ in 0..200 {
            let h32 = rng.next_u32();
            let mut cc = [0u8; 4];
            let mut rc = [0u8; 4];
            c_cf32(cc.as_mut_ptr() as *mut c_void, h32);
            r_cf32(rc.as_mut_ptr() as *mut c_void, h32);
            assert_eq!(cc, rc, "canonicalFromHash32 {}", h32);
            let cb = c_hf32(cc.as_ptr() as *const c_void);
            let rb = r_hf32(rc.as_ptr() as *const c_void);
            assert_eq!(cb, rb);
            assert_eq!(cb, h32);

            let h64 = rng.next_u64();
            let mut cc8 = [0u8; 8];
            let mut rc8 = [0u8; 8];
            c_cf64(cc8.as_mut_ptr() as *mut c_void, h64);
            r_cf64(rc8.as_mut_ptr() as *mut c_void, h64);
            assert_eq!(cc8, rc8, "canonicalFromHash64 {}", h64);
            let cb64 = c_hf64(cc8.as_ptr() as *const c_void);
            let rb64 = r_hf64(rc8.as_ptr() as *const c_void);
            assert_eq!(cb64, rb64);
            assert_eq!(cb64, h64);
        }
    }
}

#[test]
fn test_copystate() {
    let libs = Libs::load();
    let mut rng = Rng::new(0xc0b1);
    unsafe {
        let c_cs: libloading::Symbol<CreateState> = csym(&libs, b"LZ4_XXH64_createState");
        let r_cs: libloading::Symbol<CreateState> = rsym(&libs, b"LZ4_XXH64_createState");
        let c_reset: libloading::Symbol<Reset64> = csym(&libs, b"LZ4_XXH64_reset");
        let r_reset: libloading::Symbol<Reset64> = rsym(&libs, b"LZ4_XXH64_reset");
        let c_upd: libloading::Symbol<Update> = csym(&libs, b"LZ4_XXH64_update");
        let r_upd: libloading::Symbol<Update> = rsym(&libs, b"LZ4_XXH64_update");
        let c_dig: libloading::Symbol<Digest64> = csym(&libs, b"LZ4_XXH64_digest");
        let r_dig: libloading::Symbol<Digest64> = rsym(&libs, b"LZ4_XXH64_digest");
        let c_cp: libloading::Symbol<CopyState> = csym(&libs, b"LZ4_XXH64_copyState");
        let r_cp: libloading::Symbol<CopyState> = rsym(&libs, b"LZ4_XXH64_copyState");
        let c_fs: libloading::Symbol<FreeState> = csym(&libs, b"LZ4_XXH64_freeState");
        let r_fs: libloading::Symbol<FreeState> = rsym(&libs, b"LZ4_XXH64_freeState");

        let data = rng.random(5000);
        let cst = c_cs();
        let rst = r_cs();
        c_reset(cst, 42);
        r_reset(rst, 42);
        c_upd(cst, data.as_ptr() as *const c_void, 2000);
        r_upd(rst, data.as_ptr() as *const c_void, 2000);
        // copy mid-stream
        let cst2 = c_cs();
        let rst2 = r_cs();
        c_cp(cst2, cst);
        r_cp(rst2, rst);
        // feed remainder to the copies
        c_upd(cst2, data.as_ptr().add(2000) as *const c_void, 3000);
        r_upd(rst2, data.as_ptr().add(2000) as *const c_void, 3000);
        assert_eq!(c_dig(cst2), r_dig(rst2), "copyState digest");
        c_fs(cst); r_fs(rst); c_fs(cst2); r_fs(rst2);
    }
}
