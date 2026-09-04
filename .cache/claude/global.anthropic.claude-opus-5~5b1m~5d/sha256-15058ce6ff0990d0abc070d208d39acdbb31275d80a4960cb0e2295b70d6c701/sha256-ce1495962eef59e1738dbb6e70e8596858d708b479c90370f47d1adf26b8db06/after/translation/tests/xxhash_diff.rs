//! Phase B/C differential tests for the namespaced xxHash API
//! (`LZ4_XXH32*` / `LZ4_XXH64*`), driven through both `.so` files.

mod common;

use common::*;
use std::ffi::c_void;
use std::os::raw::{c_uint, c_ulonglong};

type FnXxh32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> u32;
type FnXxh64 = unsafe extern "C" fn(*const c_void, usize, c_ulonglong) -> u64;
type FnCreate = unsafe extern "C" fn() -> *mut c_void;
type FnFree = unsafe extern "C" fn(*mut c_void) -> i32;
type FnCopy = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnReset32 = unsafe extern "C" fn(*mut c_void, c_uint) -> i32;
type FnReset64 = unsafe extern "C" fn(*mut c_void, c_ulonglong) -> i32;
type FnUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> i32;
type FnDigest32 = unsafe extern "C" fn(*const c_void) -> u32;
type FnDigest64 = unsafe extern "C" fn(*const c_void) -> u64;
type FnCanon32 = unsafe extern "C" fn(*mut c_void, u32);
type FnCanon64 = unsafe extern "C" fn(*mut c_void, u64);
type FnFromCanon32 = unsafe extern "C" fn(*const c_void) -> u32;
type FnFromCanon64 = unsafe extern "C" fn(*const c_void) -> u64;

struct Xxh {
    xxh32: FnXxh32,
    xxh64: FnXxh64,
    create32: FnCreate,
    free32: FnFree,
    copy32: FnCopy,
    reset32: FnReset32,
    update32: FnUpdate,
    digest32: FnDigest32,
    create64: FnCreate,
    free64: FnFree,
    copy64: FnCopy,
    reset64: FnReset64,
    update64: FnUpdate,
    digest64: FnDigest64,
    canon32: FnCanon32,
    from_canon32: FnFromCanon32,
    canon64: FnCanon64,
    from_canon64: FnFromCanon64,
    version: FnUIntVoid,
}

fn bind(l: &Lib) -> Xxh {
    Xxh {
        xxh32: l.sym("LZ4_XXH32"),
        xxh64: l.sym("LZ4_XXH64"),
        create32: l.sym("LZ4_XXH32_createState"),
        free32: l.sym("LZ4_XXH32_freeState"),
        copy32: l.sym("LZ4_XXH32_copyState"),
        reset32: l.sym("LZ4_XXH32_reset"),
        update32: l.sym("LZ4_XXH32_update"),
        digest32: l.sym("LZ4_XXH32_digest"),
        create64: l.sym("LZ4_XXH64_createState"),
        free64: l.sym("LZ4_XXH64_freeState"),
        copy64: l.sym("LZ4_XXH64_copyState"),
        reset64: l.sym("LZ4_XXH64_reset"),
        update64: l.sym("LZ4_XXH64_update"),
        digest64: l.sym("LZ4_XXH64_digest"),
        canon32: l.sym("LZ4_XXH32_canonicalFromHash"),
        from_canon32: l.sym("LZ4_XXH32_hashFromCanonical"),
        canon64: l.sym("LZ4_XXH64_canonicalFromHash"),
        from_canon64: l.sym("LZ4_XXH64_hashFromCanonical"),
        version: l.sym("LZ4_XXH_versionNumber"),
    }
}

fn pair() -> (Xxh, Xxh) {
    let p = libs();
    (bind(&p.c), bind(&p.r))
}

// --- CONFIGS row: version number ---------------------------------------------
#[test]
fn xxh_version_number() {
    let (c, r) = pair();
    unsafe { assert_eq!((c.version)(), (r.version)()) };
}

// --- CONFIGS rows: one-shot XXH32/XXH64, all length classes & seeds ----------
#[test]
fn xxh_oneshot_all_lengths() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xA1B2_C3D4);
    let lens: Vec<usize> = (0usize..=140)
        .chain([
            141, 200, 255, 256, 257, 511, 512, 1000, 1024, 4095, 4096, 16384, 65535, 65536,
        ])
        .collect();
    let seeds32: [u32; 6] = [0, 1, 0xFFFF_FFFF, 0x9E37_79B1, 2654435761, 42];
    let seeds64: [u64; 6] = [
        0,
        1,
        u64::MAX,
        0x9E37_79B1_85EB_CA87,
        0x1234_5678_9ABC_DEF0,
        42,
    ];
    for &len in &lens {
        for shape in ALL_SHAPES {
            let data = gen(*shape, len, &mut rng);
            for &s in &seeds32 {
                let a = unsafe { (c.xxh32)(data.as_ptr() as *const c_void, len, s) };
                let b = unsafe { (r.xxh32)(data.as_ptr() as *const c_void, len, s) };
                assert_eq!(a, b, "XXH32 len={} shape={:?} seed={:#x}", len, shape, s);
            }
            for &s in &seeds64 {
                let a = unsafe { (c.xxh64)(data.as_ptr() as *const c_void, len, s) };
                let b = unsafe { (r.xxh64)(data.as_ptr() as *const c_void, len, s) };
                assert_eq!(a, b, "XXH64 len={} shape={:?} seed={:#x}", len, shape, s);
            }
        }
    }
}

// --- CONFIGS rows: streaming with random chunk splits ------------------------
#[test]
fn xxh_streaming_random_splits() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x5EED_0001);
    for iter in 0..300 {
        let len = rng.range(0, 70_000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let data = gen(shape, len, &mut rng);
        let seed32 = rng.next_u32();
        let seed64 = rng.next_u64();

        // build a random chunking of the input
        let mut chunks: Vec<usize> = Vec::new();
        let mut left = len;
        while left > 0 {
            let n = rng.range(1, left.min(4096) + 1);
            chunks.push(n);
            left -= n;
        }
        if chunks.is_empty() {
            chunks.push(0); // exercise a zero-length update
        }

        for (name, is32) in [("XXH32", true), ("XXH64", false)] {
            let (cs, rs) = unsafe {
                if is32 {
                    ((c.create32)(), (r.create32)())
                } else {
                    ((c.create64)(), (r.create64)())
                }
            };
            assert!(!cs.is_null() && !rs.is_null());
            unsafe {
                if is32 {
                    assert_eq!((c.reset32)(cs, seed32), (r.reset32)(rs, seed32));
                } else {
                    assert_eq!((c.reset64)(cs, seed64), (r.reset64)(rs, seed64));
                }
                let mut off = 0usize;
                for &n in &chunks {
                    let p = data.as_ptr().add(off) as *const c_void;
                    let (a, b) = if is32 {
                        ((c.update32)(cs, p, n), (r.update32)(rs, p, n))
                    } else {
                        ((c.update64)(cs, p, n), (r.update64)(rs, p, n))
                    };
                    assert_eq!(a, b, "{} update rc iter={}", name, iter);
                    off += n;
                    // intermediate digests must match too
                    if is32 {
                        assert_eq!(
                            (c.digest32)(cs),
                            (r.digest32)(rs),
                            "{} intermediate digest iter={} off={}",
                            name,
                            iter,
                            off
                        );
                    } else {
                        assert_eq!(
                            (c.digest64)(cs),
                            (r.digest64)(rs),
                            "{} intermediate digest iter={} off={}",
                            name,
                            iter,
                            off
                        );
                    }
                }
                // final digest must equal the one-shot value
                if is32 {
                    let d = (c.digest32)(cs);
                    assert_eq!(d, (r.digest32)(rs));
                    assert_eq!(
                        d,
                        (c.xxh32)(data.as_ptr() as *const c_void, len, seed32),
                        "streaming vs one-shot XXH32"
                    );
                } else {
                    let d = (c.digest64)(cs);
                    assert_eq!(d, (r.digest64)(rs));
                    assert_eq!(
                        d,
                        (c.xxh64)(data.as_ptr() as *const c_void, len, seed64),
                        "streaming vs one-shot XXH64"
                    );
                }
                if is32 {
                    assert_eq!((c.free32)(cs), (r.free32)(rs));
                } else {
                    assert_eq!((c.free64)(cs), (r.free64)(rs));
                }
            }
        }
    }
}

// --- CONFIGS row: copyState ---------------------------------------------------
#[test]
fn xxh_copy_state() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xC0FF_EE01);
    for _ in 0..50 {
        let data = gen(Shape::Mixed, rng.range(0, 5000), &mut rng);
        let half = data.len() / 2;
        let seed = rng.next_u32();
        unsafe {
            let (c1, c2) = ((c.create32)(), (c.create32)());
            let (r1, r2) = ((r.create32)(), (r.create32)());
            (c.reset32)(c1, seed);
            (r.reset32)(r1, seed);
            (c.update32)(c1, data.as_ptr() as *const c_void, half);
            (r.update32)(r1, data.as_ptr() as *const c_void, half);
            (c.copy32)(c2, c1);
            (r.copy32)(r2, r1);
            let p = data.as_ptr().add(half) as *const c_void;
            (c.update32)(c2, p, data.len() - half);
            (r.update32)(r2, p, data.len() - half);
            assert_eq!((c.digest32)(c2), (r.digest32)(r2), "XXH32 copyState");
            assert_eq!((c.digest32)(c1), (r.digest32)(r1), "XXH32 original state");
            (c.free32)(c1);
            (c.free32)(c2);
            (r.free32)(r1);
            (r.free32)(r2);

            let (c1, c2) = ((c.create64)(), (c.create64)());
            let (r1, r2) = ((r.create64)(), (r.create64)());
            let seed64 = rng.next_u64();
            (c.reset64)(c1, seed64);
            (r.reset64)(r1, seed64);
            (c.update64)(c1, data.as_ptr() as *const c_void, half);
            (r.update64)(r1, data.as_ptr() as *const c_void, half);
            (c.copy64)(c2, c1);
            (r.copy64)(r2, r1);
            (c.update64)(c2, p, data.len() - half);
            (r.update64)(r2, p, data.len() - half);
            assert_eq!((c.digest64)(c2), (r.digest64)(r2), "XXH64 copyState");
            (c.free64)(c1);
            (c.free64)(c2);
            (r.free64)(r1);
            (r.free64)(r2);
        }
    }
}

// --- CONFIGS row: canonical representation -----------------------------------
#[test]
fn xxh_canonical_roundtrip() {
    let (c, r) = pair();
    let mut rng = Rng::new(0xCA_0000);
    let mut vals32: Vec<u32> = vec![0, 1, 0x7FFF_FFFF, 0x8000_0000, u32::MAX];
    let mut vals64: Vec<u64> = vec![0, 1, u64::MAX, 0x0102_0304_0506_0708];
    for _ in 0..200 {
        vals32.push(rng.next_u32());
        vals64.push(rng.next_u64());
    }
    for v in vals32 {
        let mut cb = [0u8; 4];
        let mut rb = [0u8; 4];
        unsafe {
            (c.canon32)(cb.as_mut_ptr() as *mut c_void, v);
            (r.canon32)(rb.as_mut_ptr() as *mut c_void, v);
        }
        assert_eq!(cb, rb, "XXH32_canonicalFromHash({:#x})", v);
        let a = unsafe { (c.from_canon32)(cb.as_ptr() as *const c_void) };
        let b = unsafe { (r.from_canon32)(rb.as_ptr() as *const c_void) };
        assert_eq!(a, b);
        assert_eq!(a, v);
    }
    for v in vals64 {
        let mut cb = [0u8; 8];
        let mut rb = [0u8; 8];
        unsafe {
            (c.canon64)(cb.as_mut_ptr() as *mut c_void, v);
            (r.canon64)(rb.as_mut_ptr() as *mut c_void, v);
        }
        assert_eq!(cb, rb, "XXH64_canonicalFromHash({:#x})", v);
        let a = unsafe { (c.from_canon64)(cb.as_ptr() as *const c_void) };
        let b = unsafe { (r.from_canon64)(rb.as_ptr() as *const c_void) };
        assert_eq!(a, b);
        assert_eq!(a, v);
    }
}

// --- CONFIGS row: unaligned input pointers ----------------------------------
#[test]
fn xxh_unaligned_inputs() {
    let (c, r) = pair();
    let mut rng = Rng::new(0x00A1_1697_u64);
    let buf = gen(Shape::Random, 4096, &mut rng);
    for off in 0..17usize {
        for len in [0usize, 1, 3, 4, 7, 8, 15, 16, 17, 31, 32, 33, 100, 1000, 2000] {
            let p = unsafe { buf.as_ptr().add(off) } as *const c_void;
            assert_eq!(
                unsafe { (c.xxh32)(p, len, 7) },
                unsafe { (r.xxh32)(p, len, 7) },
                "XXH32 unaligned off={} len={}",
                off,
                len
            );
            assert_eq!(
                unsafe { (c.xxh64)(p, len, 7) },
                unsafe { (r.xxh64)(p, len, 7) },
                "XXH64 unaligned off={} len={}",
                off,
                len
            );
        }
    }
}

// --- ERRORS rows: NULL handling ---------------------------------------------
#[test]
fn xxh_error_paths() {
    let (c, r) = pair();
    // XXH32/XXH64 with NULL input and len 0 (documented: allowed)
    assert_eq!(
        unsafe { (c.xxh32)(std::ptr::null(), 0, 0) },
        unsafe { (r.xxh32)(std::ptr::null(), 0, 0) },
        "XXH32(NULL,0,0)"
    );
    assert_eq!(
        unsafe { (c.xxh64)(std::ptr::null(), 0, 0) },
        unsafe { (r.xxh64)(std::ptr::null(), 0, 0) },
        "XXH64(NULL,0,0)"
    );
    // NOTE: XXH32_reset(NULL,..) / XXH32_update(NULL,..) memcpy/deref the state
    // pointer unconditionally in the C source (xxhash.c:446, :461) — a NULL state
    // is hard UB that segfaults in C, so it is not a testable rejection.
    // freeState(NULL) -> XXH_OK (free(NULL) is a no-op)
    assert_eq!(
        unsafe { (c.free32)(std::ptr::null_mut()) },
        unsafe { (r.free32)(std::ptr::null_mut()) },
        "XXH32_freeState(NULL)"
    );
    assert_eq!(
        unsafe { (c.free64)(std::ptr::null_mut()) },
        unsafe { (r.free64)(std::ptr::null_mut()) },
        "XXH64_freeState(NULL)"
    );
    // update with NULL input, len == 0 -> XXH_OK ; NULL input & len != 0 -> XXH_ERROR
    unsafe {
        let cs = (c.create32)();
        let rs = (r.create32)();
        (c.reset32)(cs, 0);
        (r.reset32)(rs, 0);
        assert_eq!(
            (c.update32)(cs, std::ptr::null(), 0),
            (r.update32)(rs, std::ptr::null(), 0),
            "XXH32_update(state,NULL,0)"
        );
        assert_eq!(
            (c.update32)(cs, std::ptr::null(), 4),
            (r.update32)(rs, std::ptr::null(), 4),
            "XXH32_update(state,NULL,4)"
        );
        assert_eq!((c.digest32)(cs), (r.digest32)(rs));
        (c.free32)(cs);
        (r.free32)(rs);

        let cs = (c.create64)();
        let rs = (r.create64)();
        (c.reset64)(cs, 0);
        (r.reset64)(rs, 0);
        assert_eq!(
            (c.update64)(cs, std::ptr::null(), 0),
            (r.update64)(rs, std::ptr::null(), 0),
            "XXH64_update(state,NULL,0)"
        );
        assert_eq!(
            (c.update64)(cs, std::ptr::null(), 4),
            (r.update64)(rs, std::ptr::null(), 4),
            "XXH64_update(state,NULL,4)"
        );
        assert_eq!((c.digest64)(cs), (r.digest64)(rs));
        (c.free64)(cs);
        (r.free64)(rs);
    }
}

// --- CONFIGS row: digest without reset (fresh state) -------------------------
#[test]
fn xxh_digest_on_fresh_state() {
    let (c, r) = pair();
    unsafe {
        let cs = (c.create32)();
        let rs = (r.create32)();
        (c.reset32)(cs, 0);
        (r.reset32)(rs, 0);
        assert_eq!((c.digest32)(cs), (r.digest32)(rs), "XXH32 empty digest");
        (c.free32)(cs);
        (r.free32)(rs);
        let cs = (c.create64)();
        let rs = (r.create64)();
        (c.reset64)(cs, 0);
        (r.reset64)(rs, 0);
        assert_eq!((c.digest64)(cs), (r.digest64)(rs), "XXH64 empty digest");
        (c.free64)(cs);
        (r.free64)(rs);
    }
}
