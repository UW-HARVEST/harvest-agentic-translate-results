//! Phase B/C — xxHash (`xxhash.c`, namespaced `LZ4_XXH*`).
//! CONFIGS.md rows 99–106, ERRORS.md rows 147–156.
#![allow(non_snake_case)]

mod common;
use common::*;

type FnXXH32 = unsafe extern "C" fn(*const CVoid, usize, u32) -> u32;
type FnXXH64 = unsafe extern "C" fn(*const CVoid, usize, u64) -> u64;
type FnCreate = unsafe extern "C" fn() -> *mut CVoid;
type FnFree = unsafe extern "C" fn(*mut CVoid) -> i32;
type FnCopy = unsafe extern "C" fn(*mut CVoid, *const CVoid);
type FnReset32 = unsafe extern "C" fn(*mut CVoid, u32) -> i32;
type FnReset64 = unsafe extern "C" fn(*mut CVoid, u64) -> i32;
type FnUpdate = unsafe extern "C" fn(*mut CVoid, *const CVoid, usize) -> i32;
type FnDigest32 = unsafe extern "C" fn(*const CVoid) -> u32;
type FnDigest64 = unsafe extern "C" fn(*const CVoid) -> u64;
type FnCanon32 = unsafe extern "C" fn(*mut u8, u32);
type FnCanon64 = unsafe extern "C" fn(*mut u8, u64);
type FnFromCanon32 = unsafe extern "C" fn(*const u8) -> u32;
type FnFromCanon64 = unsafe extern "C" fn(*const u8) -> u64;

/// Every length that crosses an internal boundary in the C implementation:
/// the 16-byte (XXH32) / 32-byte (XXH64) stripe, the 4/8-byte lane, and the
/// 1-byte tail loop.
const LENS: [usize; 30] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 15, 16, 17, 20, 23, 24, 31, 32, 33, 47, 48, 63, 64,
    65, 127, 1000, 4096,
];

const SEEDS32: [u32; 6] = [0, 1, 2, 0x9E3779B1, 0x7FFF_FFFF, 0xFFFF_FFFF];
const SEEDS64: [u64; 7] = [
    0,
    1,
    2,
    0x9E3779B185EBCA87,
    0x7FFF_FFFF_FFFF_FFFF,
    0xFFFF_FFFF_FFFF_FFFF,
    0x0000_0001_0000_0000,
];

#[test]
fn r105_version() {
    diff("XXH_versionNumber", |lib| unsafe {
        sym::<unsafe extern "C" fn() -> u32>(lib, "LZ4_XXH_versionNumber")()
    });
}

/* ================================================================== */
/* rows 99,100 / errors 147 — one-shot hashing                         */
/* ================================================================== */

#[test]
fn r099_r100_oneshot() {
    let mut rng = Rng::new(0x5EED_0099);
    for &shape in ALL_SHAPES.iter() {
        for &len in LENS.iter() {
            let data = mkdata(shape, len, &mut rng);
            diff(&format!("XXH oneshot {shape:?} len={len}"), |lib| unsafe {
                let f32 = sym::<FnXXH32>(lib, "LZ4_XXH32");
                let f64 = sym::<FnXXH64>(lib, "LZ4_XXH64");
                let mut out: Vec<u64> = Vec::new();
                for &s in SEEDS32.iter() {
                    out.push(f32(data.as_ptr() as *const CVoid, len, s) as u64);
                }
                for &s in SEEDS64.iter() {
                    out.push(f64(data.as_ptr() as *const CVoid, len, s));
                }
                out
            });
        }
    }
    // NULL input with length 0: the one-shot entry points have no NULL guard,
    // but with len == 0 they never dereference — this is a real, defined input.
    diff("XXH oneshot NULL/0", |lib| unsafe {
        let f32 = sym::<FnXXH32>(lib, "LZ4_XXH32");
        let f64 = sym::<FnXXH64>(lib, "LZ4_XXH64");
        (
            f32(std::ptr::null(), 0, 0),
            f32(std::ptr::null(), 0, 0xDEADBEEF),
            f64(std::ptr::null(), 0, 0),
            f64(std::ptr::null(), 0, 0xDEAD_BEEF_CAFE_BABE),
        )
    });
    // randomized lengths
    for i in 0..500 {
        let shape = ALL_SHAPES[i % ALL_SHAPES.len()];
        let len = rng.range(0, 20000);
        let data = mkdata(shape, len, &mut rng);
        let s32 = rng.next_u32();
        let s64 = rng.next_u64();
        diff(&format!("XXH rand #{i} len={len}"), |lib| unsafe {
            let f32 = sym::<FnXXH32>(lib, "LZ4_XXH32");
            let f64 = sym::<FnXXH64>(lib, "LZ4_XXH64");
            (
                f32(data.as_ptr() as *const CVoid, len, s32),
                f64(data.as_ptr() as *const CVoid, len, s64),
            )
        });
    }
}

/* ================================================================== */
/* row 106 — unaligned inputs                                          */
/* ================================================================== */

#[test]
fn r106_unaligned() {
    let mut rng = Rng::new(0x5EED_0106);
    let base = mkdata(Shape::Random, 8192, &mut rng);
    for off in 0usize..8 {
        for &len in LENS.iter() {
            if off + len > base.len() {
                continue;
            }
            diff(&format!("XXH unaligned off={off} len={len}"), |lib| unsafe {
                let f32 = sym::<FnXXH32>(lib, "LZ4_XXH32");
                let f64 = sym::<FnXXH64>(lib, "LZ4_XXH64");
                let p = base.as_ptr().add(off) as *const CVoid;
                (f32(p, len, 0x1234_5678), f64(p, len, 0x1234_5678_9ABC_DEF0))
            });
        }
    }
}

/* ================================================================== */
/* rows 101,102 / errors 148–155 — streaming                           */
/* ================================================================== */

fn stream_hash(
    lib: &libloading::Library,
    data: &[u8],
    chunks: &[usize],
    seed32: u32,
    seed64: u64,
) -> (Vec<i32>, u32, Vec<i32>, u64) {
    unsafe {
        let c32 = sym::<FnCreate>(lib, "LZ4_XXH32_createState")();
        let c64 = sym::<FnCreate>(lib, "LZ4_XXH64_createState")();
        assert!(!c32.is_null() && !c64.is_null());
        let mut codes32 = vec![sym::<FnReset32>(lib, "LZ4_XXH32_reset")(c32, seed32)];
        let mut codes64 = vec![sym::<FnReset64>(lib, "LZ4_XXH64_reset")(c64, seed64)];
        let u32f = sym::<FnUpdate>(lib, "LZ4_XXH32_update");
        let u64f = sym::<FnUpdate>(lib, "LZ4_XXH64_update");
        let mut off = 0usize;
        for &c in chunks {
            let n = c.min(data.len() - off);
            codes32.push(u32f(c32, data[off..].as_ptr() as *const CVoid, n));
            codes64.push(u64f(c64, data[off..].as_ptr() as *const CVoid, n));
            off += n;
            if off >= data.len() {
                break;
            }
        }
        // feed the remainder in one go
        if off < data.len() {
            let n = data.len() - off;
            codes32.push(u32f(c32, data[off..].as_ptr() as *const CVoid, n));
            codes64.push(u64f(c64, data[off..].as_ptr() as *const CVoid, n));
        }
        let d32 = sym::<FnDigest32>(lib, "LZ4_XXH32_digest")(c32);
        let d64 = sym::<FnDigest64>(lib, "LZ4_XXH64_digest")(c64);
        sym::<FnFree>(lib, "LZ4_XXH32_freeState")(c32);
        sym::<FnFree>(lib, "LZ4_XXH64_freeState")(c64);
        (codes32, d32, codes64, d64)
    }
}

#[test]
fn r101_r102_streaming() {
    let mut rng = Rng::new(0x5EED_0101);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 1, 15, 16, 17, 31, 32, 33, 100, 1000, 5000].iter() {
            let data = mkdata(shape, len, &mut rng);
            for pattern in 0..5 {
                let chunks: Vec<usize> = match pattern {
                    0 => vec![1; len.max(1)],
                    1 => vec![len],
                    2 => vec![3; len / 3 + 2],
                    3 => vec![16; len / 16 + 2],
                    _ => (0..40).map(|_| rng.range(0, 64)).collect(),
                };
                diff(
                    &format!("XXH stream {shape:?} len={len} pat={pattern}"),
                    |lib| stream_hash(lib, &data, &chunks, 0xA5A5_5A5A, 0x0123_4567_89AB_CDEF),
                );
            }
        }
    }
    // digest of an empty state, for every seed
    diff("XXH digest empty", |lib| unsafe {
        let mut out: Vec<u64> = Vec::new();
        for &s in SEEDS32.iter() {
            let c = sym::<FnCreate>(lib, "LZ4_XXH32_createState")();
            out.push(sym::<FnReset32>(lib, "LZ4_XXH32_reset")(c, s) as u64);
            out.push(sym::<FnDigest32>(lib, "LZ4_XXH32_digest")(c) as u64);
            sym::<FnFree>(lib, "LZ4_XXH32_freeState")(c);
        }
        for &s in SEEDS64.iter() {
            let c = sym::<FnCreate>(lib, "LZ4_XXH64_createState")();
            out.push(sym::<FnReset64>(lib, "LZ4_XXH64_reset")(c, s) as u64);
            out.push(sym::<FnDigest64>(lib, "LZ4_XXH64_digest")(c));
            sym::<FnFree>(lib, "LZ4_XXH64_freeState")(c);
        }
        out
    });
}

#[test]
fn e148_update_null_and_zero() {
    let mut rng = Rng::new(0x5EED_1148);
    let data = mkdata(Shape::Random, 256, &mut rng);
    diff("XXH update NULL / len 0", |lib| unsafe {
        let c32 = sym::<FnCreate>(lib, "LZ4_XXH32_createState")();
        let c64 = sym::<FnCreate>(lib, "LZ4_XXH64_createState")();
        sym::<FnReset32>(lib, "LZ4_XXH32_reset")(c32, 7);
        sym::<FnReset64>(lib, "LZ4_XXH64_reset")(c64, 7);
        let u32f = sym::<FnUpdate>(lib, "LZ4_XXH32_update");
        let u64f = sym::<FnUpdate>(lib, "LZ4_XXH64_update");
        let mut out: Vec<i64> = Vec::new();
        // NULL input -> XXH_ERROR, state must be untouched
        out.push(u32f(c32, std::ptr::null(), 0) as i64);
        out.push(u32f(c32, std::ptr::null(), 100) as i64);
        out.push(u64f(c64, std::ptr::null(), 0) as i64);
        out.push(u64f(c64, std::ptr::null(), 100) as i64);
        out.push(sym::<FnDigest32>(lib, "LZ4_XXH32_digest")(c32) as i64);
        out.push(sym::<FnDigest64>(lib, "LZ4_XXH64_digest")(c64) as i64);
        // zero-length update with a valid pointer -> XXH_OK, digest unchanged
        out.push(u32f(c32, data.as_ptr() as *const CVoid, 0) as i64);
        out.push(u64f(c64, data.as_ptr() as *const CVoid, 0) as i64);
        out.push(sym::<FnDigest32>(lib, "LZ4_XXH32_digest")(c32) as i64);
        out.push(sym::<FnDigest64>(lib, "LZ4_XXH64_digest")(c64) as i64);
        // then a real update
        out.push(u32f(c32, data.as_ptr() as *const CVoid, 256) as i64);
        out.push(u64f(c64, data.as_ptr() as *const CVoid, 256) as i64);
        out.push(sym::<FnDigest32>(lib, "LZ4_XXH32_digest")(c32) as i64);
        out.push(sym::<FnDigest64>(lib, "LZ4_XXH64_digest")(c64) as i64);
        sym::<FnFree>(lib, "LZ4_XXH32_freeState")(c32);
        sym::<FnFree>(lib, "LZ4_XXH64_freeState")(c64);
        out
    });
}

#[test]
fn e153_freeState_null() {
    diff("XXH freeState NULL", |lib| unsafe {
        (
            sym::<FnFree>(lib, "LZ4_XXH32_freeState")(std::ptr::null_mut()),
            sym::<FnFree>(lib, "LZ4_XXH64_freeState")(std::ptr::null_mut()),
        )
    });
}

/* ================================================================== */
/* row 103 — copyState                                                 */
/* ================================================================== */

#[test]
fn r103_copyState() {
    let mut rng = Rng::new(0x5EED_0103);
    for &shape in ALL_SHAPES.iter() {
        for &len in [0usize, 5, 16, 17, 33, 300, 3000].iter() {
            let a = mkdata(shape, len, &mut rng);
            let b = mkdata(shape, len + 7, &mut rng);
            diff(&format!("XXH copyState {shape:?} len={len}"), |lib| unsafe {
                let s1 = sym::<FnCreate>(lib, "LZ4_XXH32_createState")();
                let s2 = sym::<FnCreate>(lib, "LZ4_XXH32_createState")();
                let t1 = sym::<FnCreate>(lib, "LZ4_XXH64_createState")();
                let t2 = sym::<FnCreate>(lib, "LZ4_XXH64_createState")();
                sym::<FnReset32>(lib, "LZ4_XXH32_reset")(s1, 42);
                sym::<FnReset64>(lib, "LZ4_XXH64_reset")(t1, 42);
                let u32f = sym::<FnUpdate>(lib, "LZ4_XXH32_update");
                let u64f = sym::<FnUpdate>(lib, "LZ4_XXH64_update");
                u32f(s1, a.as_ptr() as *const CVoid, len);
                u64f(t1, a.as_ptr() as *const CVoid, len);
                sym::<FnCopy>(lib, "LZ4_XXH32_copyState")(s2, s1 as *const CVoid);
                sym::<FnCopy>(lib, "LZ4_XXH64_copyState")(t2, t1 as *const CVoid);
                // continue both copies with different data
                u32f(s1, b.as_ptr() as *const CVoid, b.len());
                u64f(t1, b.as_ptr() as *const CVoid, b.len());
                u32f(s2, b.as_ptr() as *const CVoid, b.len());
                u64f(t2, b.as_ptr() as *const CVoid, b.len());
                let r = (
                    sym::<FnDigest32>(lib, "LZ4_XXH32_digest")(s1),
                    sym::<FnDigest32>(lib, "LZ4_XXH32_digest")(s2),
                    sym::<FnDigest64>(lib, "LZ4_XXH64_digest")(t1),
                    sym::<FnDigest64>(lib, "LZ4_XXH64_digest")(t2),
                );
                for p in [s1, s2] {
                    sym::<FnFree>(lib, "LZ4_XXH32_freeState")(p);
                }
                for p in [t1, t2] {
                    sym::<FnFree>(lib, "LZ4_XXH64_freeState")(p);
                }
                r
            });
        }
    }
}

/* ================================================================== */
/* row 104 / error 156 — canonical representation                      */
/* ================================================================== */

#[test]
fn r104_canonical() {
    let mut rng = Rng::new(0x5EED_0104);
    let mut vals32: Vec<u32> = vec![0, 1, 0x7F, 0x80, 0xFF, 0x100, 0xFFFF, 0xFFFF_FFFF];
    let mut vals64: Vec<u64> = vec![0, 1, 0xFF, 0x1_0000_0000, u64::MAX];
    for _ in 0..64 {
        vals32.push(rng.next_u32());
        vals64.push(rng.next_u64());
    }
    diff("XXH canonical", |lib| unsafe {
        let c32 = sym::<FnCanon32>(lib, "LZ4_XXH32_canonicalFromHash");
        let c64 = sym::<FnCanon64>(lib, "LZ4_XXH64_canonicalFromHash");
        let f32 = sym::<FnFromCanon32>(lib, "LZ4_XXH32_hashFromCanonical");
        let f64 = sym::<FnFromCanon64>(lib, "LZ4_XXH64_hashFromCanonical");
        let mut out: Vec<u64> = Vec::new();
        for &v in vals32.iter() {
            let mut buf = [0u8; 4];
            c32(buf.as_mut_ptr(), v);
            out.extend(buf.iter().map(|&b| b as u64));
            out.push(f32(buf.as_ptr()) as u64);
        }
        for &v in vals64.iter() {
            let mut buf = [0u8; 8];
            c64(buf.as_mut_ptr(), v);
            out.extend(buf.iter().map(|&b| b as u64));
            out.push(f64(buf.as_ptr()));
        }
        // hashFromCanonical on arbitrary byte patterns
        for k in 0..32u8 {
            let b4 = [k, k.wrapping_add(1), k.wrapping_add(2), k.wrapping_add(3)];
            out.push(f32(b4.as_ptr()) as u64);
            let b8 = [k, 1, 2, 3, 4, 5, 6, k];
            out.push(f64(b8.as_ptr()));
        }
        out
    });
}
