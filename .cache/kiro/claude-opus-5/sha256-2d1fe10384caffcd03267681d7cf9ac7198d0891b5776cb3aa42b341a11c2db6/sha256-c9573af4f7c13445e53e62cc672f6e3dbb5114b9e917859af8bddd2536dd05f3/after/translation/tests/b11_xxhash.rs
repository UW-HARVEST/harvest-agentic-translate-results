//! Differential tests for the xxHash surface (namespaced `ZSTD_XXH*`).
//!
//! The C build uses `XXH_NAMESPACE=ZSTD_` and defines `XXH_NO_XXH3`, so only
//! the XXH32 and XXH64 families are exported. Every exported symbol reported by
//! `nm -D --defined-only c_src/build/libzstd.so | grep XXH` is exercised here:
//!
//!   ZSTD_XXH_versionNumber
//!   ZSTD_XXH32  ZSTD_XXH32_createState  ZSTD_XXH32_freeState  ZSTD_XXH32_reset
//!   ZSTD_XXH32_update  ZSTD_XXH32_digest  ZSTD_XXH32_copyState
//!   ZSTD_XXH32_canonicalFromHash  ZSTD_XXH32_hashFromCanonical
//!   ZSTD_XXH64  ZSTD_XXH64_createState  ZSTD_XXH64_freeState  ZSTD_XXH64_reset
//!   ZSTD_XXH64_update  ZSTD_XXH64_digest  ZSTD_XXH64_copyState
//!   ZSTD_XXH64_canonicalFromHash  ZSTD_XXH64_hashFromCanonical
//!
//! Every call crosses the FFI boundary through `both::<T>()` so the Rust
//! translation's `#[no_mangle]` wrappers are what actually run.
#![allow(non_snake_case)]
mod harness;
use harness::*;
use std::os::raw::{c_int, c_uint, c_void};

// ------------------------------------------------------------------- typedefs

// XXH_errorcode is an enum { XXH_OK=0, XXH_ERROR } -> C int.
type FnXXH32 = unsafe extern "C" fn(*const c_void, size_t, c_uint) -> c_uint;
type FnXXH64 = unsafe extern "C" fn(*const c_void, size_t, u64) -> u64;
type FnVoidToPtr = unsafe extern "C" fn() -> *mut c_void;
type FnFreeState = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnCopyState = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnReset32 = unsafe extern "C" fn(*mut c_void, c_uint) -> c_int;
type FnReset64 = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type FnUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, size_t) -> c_int;
type FnDigest32 = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnDigest64 = unsafe extern "C" fn(*const c_void) -> u64;
// canonicalFromHash(dst, hash): void ; hashFromCanonical(src) -> hash
type FnCanon32 = unsafe extern "C" fn(*mut c_void, c_uint);
type FnFromCanon32 = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnCanon64 = unsafe extern "C" fn(*mut c_void, u64);
type FnFromCanon64 = unsafe extern "C" fn(*const c_void) -> u64;
type FnVersion = unsafe extern "C" fn() -> c_uint;

// The static-linking-only state struct layouts from xxhash.h. We only ever
// treat these as opaque byte blobs (for the raw-state memcmp test); the actual
// state objects are always allocated by the library's own createState().
const XXH32_STATE_SIZE: usize = 48; // (total_len_32,large_len,v[4],mem32[4],memsize,reserved)*4
const XXH64_STATE_SIZE: usize = 88; // total_len(8)+v[4](32)+mem64[4](32)+memsize(4)+reserved32(4)+reserved64(8)

/// Seeds to sweep for both hash families.
fn seeds32(rng: &mut Rng) -> Vec<u32> {
    vec![0, 1, 0xdead_beef, u32::MAX, rng.next_u32(), rng.next_u32()]
}
fn seeds64(rng: &mut Rng) -> Vec<u64> {
    vec![0, 1, 0xdead_beef, u64::MAX, rng.next_u64(), rng.next_u64()]
}

// -------------------------------------------------------------- one-shot XXH32

#[test]
fn xxh32_oneshot_all_shapes_lens_seeds() {
    unsafe {
        let (c32, r32) = both::<FnXXH32>("ZSTD_XXH32");
        let mut rng = Rng::new(0xB11_0001);
        for &shape in ALL_SHAPES {
            for &len in LENS {
                let src = gen(shape, len, &mut rng);
                // IMPORTANT: pass src.len(), not len (Shape::Empty ignores len).
                for &seed in &seeds32(&mut rng) {
                    let ptr = if src.is_empty() {
                        std::ptr::null()
                    } else {
                        src.as_ptr() as *const c_void
                    };
                    let a = c32(ptr, src.len(), seed);
                    let b = r32(ptr, src.len(), seed);
                    assert_eq!(
                        a, b,
                        "ZSTD_XXH32 shape={shape:?} len={} seed={seed:#x}: C={a:#x} RS={b:#x}",
                        src.len()
                    );
                }
            }
        }
    }
}

// -------------------------------------------------------------- one-shot XXH64

#[test]
fn xxh64_oneshot_all_shapes_lens_seeds() {
    unsafe {
        let (c64, r64) = both::<FnXXH64>("ZSTD_XXH64");
        let mut rng = Rng::new(0xB11_0002);
        for &shape in ALL_SHAPES {
            for &len in LENS {
                let src = gen(shape, len, &mut rng);
                for &seed in &seeds64(&mut rng) {
                    let ptr = if src.is_empty() {
                        std::ptr::null()
                    } else {
                        src.as_ptr() as *const c_void
                    };
                    let a = c64(ptr, src.len(), seed);
                    let b = r64(ptr, src.len(), seed);
                    assert_eq!(
                        a, b,
                        "ZSTD_XXH64 shape={shape:?} len={} seed={seed:#x}: C={a:#x} RS={b:#x}",
                        src.len()
                    );
                }
            }
        }
    }
}

// ----------------------------------------------------------- streaming XXH32

const CHUNKS: &[usize] = &[1, 2, 3, 7, 16, 31, 32, 4096];

/// Streaming reset/update/digest with many chunk sizes. Proves:
///  * the streaming digest equals the one-shot digest, and
///  * C and Rust streaming states agree bit-for-bit at every step.
#[test]
fn xxh32_streaming_equals_oneshot_and_agrees() {
    unsafe {
        let (c32, _r32) = both::<FnXXH32>("ZSTD_XXH32");
        let (cnew, rnew) = both::<FnVoidToPtr>("ZSTD_XXH32_createState");
        let (cfree, rfree) = both::<FnFreeState>("ZSTD_XXH32_freeState");
        let (crst, rrst) = both::<FnReset32>("ZSTD_XXH32_reset");
        let (cupd, rupd) = both::<FnUpdate>("ZSTD_XXH32_update");
        let (cdig, rdig) = both::<FnDigest32>("ZSTD_XXH32_digest");

        let mut rng = Rng::new(0xB11_0003);
        for &shape in ALL_SHAPES {
            for &len in &[0usize, 1, 2, 3, 7, 15, 16, 17, 31, 32, 33, 100, 1000, 4096, 20000] {
                let src = gen(shape, len, &mut rng);
                let oneshot = c32(
                    if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void },
                    src.len(),
                    0,
                );
                let mut chunk_sizes: Vec<usize> = CHUNKS.to_vec();
                chunk_sizes.push(1 + rng.below(64)); // random chunk size
                for &chunk in &chunk_sizes {
                    let cs = cnew();
                    let rs = rnew();
                    assert!(!cs.is_null() && !rs.is_null());
                    assert_eq!(crst(cs, 0), rrst(rs, 0), "XXH32_reset agreement");
                    let mut off = 0usize;
                    while off < src.len() {
                        let n = chunk.min(src.len() - off);
                        let p = src[off..].as_ptr() as *const c_void;
                        let a = cupd(cs, p, n);
                        let b = rupd(rs, p, n);
                        assert_eq!(a, b, "XXH32_update rc shape={shape:?} chunk={chunk}");
                        off += n;
                        // raw state bytes must match after each update
                        assert_state_eq_32(cs, rs, &format!(
                            "XXH32 state shape={shape:?} len={} chunk={chunk} off={off}",
                            src.len()));
                    }
                    let cd = cdig(cs);
                    let rd = rdig(rs);
                    assert_eq!(cd, rd, "XXH32 streaming digest C vs RS");
                    assert_eq!(
                        cd, oneshot,
                        "XXH32 streaming != oneshot shape={shape:?} len={} chunk={chunk}",
                        src.len()
                    );
                    assert_eq!(cfree(cs), rfree(rs), "XXH32_freeState rc");
                }
            }
        }
    }
}

// ----------------------------------------------------------- streaming XXH64

#[test]
fn xxh64_streaming_equals_oneshot_and_agrees() {
    unsafe {
        let (c64, _r64) = both::<FnXXH64>("ZSTD_XXH64");
        let (cnew, rnew) = both::<FnVoidToPtr>("ZSTD_XXH64_createState");
        let (cfree, rfree) = both::<FnFreeState>("ZSTD_XXH64_freeState");
        let (crst, rrst) = both::<FnReset64>("ZSTD_XXH64_reset");
        let (cupd, rupd) = both::<FnUpdate>("ZSTD_XXH64_update");
        let (cdig, rdig) = both::<FnDigest64>("ZSTD_XXH64_digest");

        let mut rng = Rng::new(0xB11_0004);
        for &shape in ALL_SHAPES {
            for &len in &[0usize, 1, 3, 7, 8, 15, 16, 31, 32, 33, 63, 64, 100, 1000, 4096, 20000] {
                let src = gen(shape, len, &mut rng);
                let oneshot = c64(
                    if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void },
                    src.len(),
                    0,
                );
                let mut chunk_sizes: Vec<usize> = CHUNKS.to_vec();
                chunk_sizes.push(1 + rng.below(64));
                for &chunk in &chunk_sizes {
                    let cs = cnew();
                    let rs = rnew();
                    assert!(!cs.is_null() && !rs.is_null());
                    assert_eq!(crst(cs, 0), rrst(rs, 0), "XXH64_reset agreement");
                    let mut off = 0usize;
                    while off < src.len() {
                        let n = chunk.min(src.len() - off);
                        let p = src[off..].as_ptr() as *const c_void;
                        let a = cupd(cs, p, n);
                        let b = rupd(rs, p, n);
                        assert_eq!(a, b, "XXH64_update rc shape={shape:?} chunk={chunk}");
                        off += n;
                        assert_state_eq_64(cs, rs, &format!(
                            "XXH64 state shape={shape:?} len={} chunk={chunk} off={off}",
                            src.len()));
                    }
                    let cd = cdig(cs);
                    let rd = rdig(rs);
                    assert_eq!(cd, rd, "XXH64 streaming digest C vs RS");
                    assert_eq!(
                        cd, oneshot,
                        "XXH64 streaming != oneshot shape={shape:?} len={} chunk={chunk}",
                        src.len()
                    );
                    assert_eq!(cfree(cs), rfree(rs), "XXH64_freeState rc");
                }
            }
        }
    }
}

// ------------------------------------------------------------- reset w/ seed

#[test]
fn xxh32_streaming_seeded() {
    unsafe {
        let (c32, _) = both::<FnXXH32>("ZSTD_XXH32");
        let (cnew, rnew) = both::<FnVoidToPtr>("ZSTD_XXH32_createState");
        let (cfree, rfree) = both::<FnFreeState>("ZSTD_XXH32_freeState");
        let (crst, rrst) = both::<FnReset32>("ZSTD_XXH32_reset");
        let (cupd, rupd) = both::<FnUpdate>("ZSTD_XXH32_update");
        let (cdig, rdig) = both::<FnDigest32>("ZSTD_XXH32_digest");
        let mut rng = Rng::new(0xB11_0005);
        for &seed in &[0u32, 1, 0xdead_beef, u32::MAX, rng.next_u32()] {
            for &len in &[0usize, 5, 16, 40, 1000] {
                let src = gen(Shape::Random, len, &mut rng);
                let one = c32(
                    if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void },
                    src.len(), seed);
                let cs = cnew();
                let rs = rnew();
                assert_eq!(crst(cs, seed), rrst(rs, seed));
                if !src.is_empty() {
                    assert_eq!(
                        cupd(cs, src.as_ptr() as *const c_void, src.len()),
                        rupd(rs, src.as_ptr() as *const c_void, src.len()));
                }
                let cd = cdig(cs);
                assert_eq!(cd, rdig(rs));
                assert_eq!(cd, one, "seeded XXH32 stream != oneshot seed={seed:#x} len={len}");
                cfree(cs);
                rfree(rs);
            }
        }
    }
}

#[test]
fn xxh64_streaming_seeded() {
    unsafe {
        let (c64, _) = both::<FnXXH64>("ZSTD_XXH64");
        let (cnew, rnew) = both::<FnVoidToPtr>("ZSTD_XXH64_createState");
        let (cfree, rfree) = both::<FnFreeState>("ZSTD_XXH64_freeState");
        let (crst, rrst) = both::<FnReset64>("ZSTD_XXH64_reset");
        let (cupd, rupd) = both::<FnUpdate>("ZSTD_XXH64_update");
        let (cdig, rdig) = both::<FnDigest64>("ZSTD_XXH64_digest");
        let mut rng = Rng::new(0xB11_0006);
        for &seed in &[0u64, 1, 0xdead_beef, u64::MAX, rng.next_u64()] {
            for &len in &[0usize, 7, 32, 80, 1000] {
                let src = gen(Shape::Random, len, &mut rng);
                let one = c64(
                    if src.is_empty() { std::ptr::null() } else { src.as_ptr() as *const c_void },
                    src.len(), seed);
                let cs = cnew();
                let rs = rnew();
                assert_eq!(crst(cs, seed), rrst(rs, seed));
                if !src.is_empty() {
                    assert_eq!(
                        cupd(cs, src.as_ptr() as *const c_void, src.len()),
                        rupd(rs, src.as_ptr() as *const c_void, src.len()));
                }
                let cd = cdig(cs);
                assert_eq!(cd, rdig(rs));
                assert_eq!(cd, one, "seeded XXH64 stream != oneshot seed={seed:#x} len={len}");
                cfree(cs);
                rfree(rs);
            }
        }
    }
}

// ------------------------------------------------------------------ copyState

/// copyState then continue: reset+update part, copy, feed the rest to the copy,
/// and confirm the copy's digest equals a straight-through hash. Also verify
/// the copied raw state bytes match between C and Rust.
#[test]
fn xxh32_copystate_then_continue() {
    unsafe {
        let (c32, _) = both::<FnXXH32>("ZSTD_XXH32");
        let (cnew, rnew) = both::<FnVoidToPtr>("ZSTD_XXH32_createState");
        let (cfree, rfree) = both::<FnFreeState>("ZSTD_XXH32_freeState");
        let (crst, rrst) = both::<FnReset32>("ZSTD_XXH32_reset");
        let (cupd, rupd) = both::<FnUpdate>("ZSTD_XXH32_update");
        let (cdig, rdig) = both::<FnDigest32>("ZSTD_XXH32_digest");
        let (ccp, rcp) = both::<FnCopyState>("ZSTD_XXH32_copyState");
        let mut rng = Rng::new(0xB11_0007);
        for &len in &[1usize, 16, 40, 100, 5000] {
            let src = gen(Shape::Text, len, &mut rng);
            let split = src.len() / 2;
            let full = c32(src.as_ptr() as *const c_void, src.len(), 0);
            let cs = cnew();
            let rs = rnew();
            crst(cs, 0);
            rrst(rs, 0);
            cupd(cs, src.as_ptr() as *const c_void, split);
            rupd(rs, src.as_ptr() as *const c_void, split);
            // copy into fresh states
            let cc = cnew();
            let rc = rnew();
            ccp(cc, cs);
            rcp(rc, rs);
            assert_state_eq_32(cc, rc, &format!("XXH32 copied state len={len}"));
            // continue on the copies
            let p = src[split..].as_ptr() as *const c_void;
            cupd(cc, p, src.len() - split);
            rupd(rc, p, src.len() - split);
            let cd = cdig(cc);
            assert_eq!(cd, rdig(rc), "XXH32 copy digest C vs RS len={len}");
            assert_eq!(cd, full, "XXH32 copyState+continue != full hash len={len}");
            cfree(cs); cfree(cc); rfree(rs); rfree(rc);
        }
    }
}

#[test]
fn xxh64_copystate_then_continue() {
    unsafe {
        let (c64, _) = both::<FnXXH64>("ZSTD_XXH64");
        let (cnew, rnew) = both::<FnVoidToPtr>("ZSTD_XXH64_createState");
        let (cfree, rfree) = both::<FnFreeState>("ZSTD_XXH64_freeState");
        let (crst, rrst) = both::<FnReset64>("ZSTD_XXH64_reset");
        let (cupd, rupd) = both::<FnUpdate>("ZSTD_XXH64_update");
        let (cdig, rdig) = both::<FnDigest64>("ZSTD_XXH64_digest");
        let (ccp, rcp) = both::<FnCopyState>("ZSTD_XXH64_copyState");
        let mut rng = Rng::new(0xB11_0008);
        for &len in &[1usize, 32, 80, 100, 5000] {
            let src = gen(Shape::Text, len, &mut rng);
            let split = src.len() / 2;
            let full = c64(src.as_ptr() as *const c_void, src.len(), 0);
            let cs = cnew();
            let rs = rnew();
            crst(cs, 0);
            rrst(rs, 0);
            cupd(cs, src.as_ptr() as *const c_void, split);
            rupd(rs, src.as_ptr() as *const c_void, split);
            let cc = cnew();
            let rc = rnew();
            ccp(cc, cs);
            rcp(rc, rs);
            assert_state_eq_64(cc, rc, &format!("XXH64 copied state len={len}"));
            let p = src[split..].as_ptr() as *const c_void;
            cupd(cc, p, src.len() - split);
            rupd(rc, p, src.len() - split);
            let cd = cdig(cc);
            assert_eq!(cd, rdig(rc), "XXH64 copy digest C vs RS len={len}");
            assert_eq!(cd, full, "XXH64 copyState+continue != full hash len={len}");
            cfree(cs); cfree(cc); rfree(rs); rfree(rc);
        }
    }
}

// -------------------------------------------------------- canonical round-trip

#[test]
fn xxh32_canonical_roundtrip() {
    unsafe {
        let (ccanon, rcanon) = both::<FnCanon32>("ZSTD_XXH32_canonicalFromHash");
        let (cfrom, rfrom) = both::<FnFromCanon32>("ZSTD_XXH32_hashFromCanonical");
        let mut rng = Rng::new(0xB11_0009);
        // include boundary values plus 5000 random hashes
        let mut hashes: Vec<u32> = vec![0, 1, 2, 0x7fff_ffff, 0x8000_0000, u32::MAX - 1, u32::MAX];
        for _ in 0..5000 {
            hashes.push(rng.next_u32());
        }
        for &h in &hashes {
            let mut cbuf = [0u8; 4];
            let mut rbuf = [0u8; 4];
            ccanon(cbuf.as_mut_ptr() as *mut c_void, h);
            rcanon(rbuf.as_mut_ptr() as *mut c_void, h);
            assert_eq!(cbuf, rbuf, "XXH32_canonicalFromHash bytes h={h:#x}");
            // canonical is big-endian
            assert_eq!(cbuf, h.to_be_bytes(), "XXH32 canonical not big-endian h={h:#x}");
            let cb = cfrom(cbuf.as_ptr() as *const c_void);
            let rb = rfrom(rbuf.as_ptr() as *const c_void);
            assert_eq!(cb, rb, "XXH32_hashFromCanonical h={h:#x}");
            assert_eq!(cb, h, "XXH32 canonical round-trip h={h:#x}");
        }
    }
}

#[test]
fn xxh64_canonical_roundtrip() {
    unsafe {
        let (ccanon, rcanon) = both::<FnCanon64>("ZSTD_XXH64_canonicalFromHash");
        let (cfrom, rfrom) = both::<FnFromCanon64>("ZSTD_XXH64_hashFromCanonical");
        let mut rng = Rng::new(0xB11_000A);
        let mut hashes: Vec<u64> =
            vec![0, 1, 2, 0x7fff_ffff_ffff_ffff, 0x8000_0000_0000_0000, u64::MAX - 1, u64::MAX];
        for _ in 0..5000 {
            hashes.push(rng.next_u64());
        }
        for &h in &hashes {
            let mut cbuf = [0u8; 8];
            let mut rbuf = [0u8; 8];
            ccanon(cbuf.as_mut_ptr() as *mut c_void, h);
            rcanon(rbuf.as_mut_ptr() as *mut c_void, h);
            assert_eq!(cbuf, rbuf, "XXH64_canonicalFromHash bytes h={h:#x}");
            assert_eq!(cbuf, h.to_be_bytes(), "XXH64 canonical not big-endian h={h:#x}");
            let cb = cfrom(cbuf.as_ptr() as *const c_void);
            let rb = rfrom(rbuf.as_ptr() as *const c_void);
            assert_eq!(cb, rb, "XXH64_hashFromCanonical h={h:#x}");
            assert_eq!(cb, h, "XXH64 canonical round-trip h={h:#x}");
        }
    }
}

// ---------------------------------------------------------- NULL / zero-length

#[test]
fn xxh_null_and_zero_length() {
    unsafe {
        let (c32, r32) = both::<FnXXH32>("ZSTD_XXH32");
        let (c64, r64) = both::<FnXXH64>("ZSTD_XXH64");
        // NULL pointer with zero length is explicitly allowed by the header.
        for &seed in &[0u32, 1, 0xdead_beef, u32::MAX] {
            assert_eq!(
                c32(std::ptr::null(), 0, seed),
                r32(std::ptr::null(), 0, seed),
                "XXH32(NULL,0,{seed:#x})"
            );
        }
        for &seed in &[0u64, 1, 0xdead_beef, u64::MAX] {
            assert_eq!(
                c64(std::ptr::null(), 0, seed),
                r64(std::ptr::null(), 0, seed),
                "XXH64(NULL,0,{seed:#x})"
            );
        }
        // zero-length update with NULL input is also allowed.
        let (cnew, rnew) = both::<FnVoidToPtr>("ZSTD_XXH32_createState");
        let (cfree, rfree) = both::<FnFreeState>("ZSTD_XXH32_freeState");
        let (crst, rrst) = both::<FnReset32>("ZSTD_XXH32_reset");
        let (cupd, rupd) = both::<FnUpdate>("ZSTD_XXH32_update");
        let (cdig, rdig) = both::<FnDigest32>("ZSTD_XXH32_digest");
        let cs = cnew();
        let rs = rnew();
        crst(cs, 0);
        rrst(rs, 0);
        assert_eq!(cupd(cs, std::ptr::null(), 0), rupd(rs, std::ptr::null(), 0),
                   "XXH32_update(NULL,0)");
        assert_state_eq_32(cs, rs, "XXH32 state after NULL update");
        assert_eq!(cdig(cs), rdig(rs), "XXH32 digest after NULL update");
        cfree(cs);
        rfree(rs);
    }
}

// ----------------------------------------------- raw-state memcmp sequences

/// Run identical operation sequences and memcmp the raw state struct bytes from
/// C and Rust. Uses createState so allocation/alignment is handled by each lib.
#[test]
fn xxh32_raw_state_memcmp_sequences() {
    unsafe {
        let (cnew, rnew) = both::<FnVoidToPtr>("ZSTD_XXH32_createState");
        let (cfree, rfree) = both::<FnFreeState>("ZSTD_XXH32_freeState");
        let (crst, rrst) = both::<FnReset32>("ZSTD_XXH32_reset");
        let (cupd, rupd) = both::<FnUpdate>("ZSTD_XXH32_update");
        let mut rng = Rng::new(0xB11_000B);
        for trial in 0..200 {
            let cs = cnew();
            let rs = rnew();
            let seed = rng.next_u32();
            assert_eq!(crst(cs, seed), rrst(rs, seed));
            assert_state_eq_32(cs, rs, &format!("XXH32 reset trial={trial} seed={seed:#x}"));
            // a randomized series of updates
            let nops = 1 + rng.below(6);
            for op in 0..nops {
                let n = rng.below(200);
                let data = gen(Shape::Random, n, &mut rng);
                let p = if data.is_empty() {
                    std::ptr::null()
                } else {
                    data.as_ptr() as *const c_void
                };
                assert_eq!(cupd(cs, p, data.len()), rupd(rs, p, data.len()));
                assert_state_eq_32(cs, rs, &format!("XXH32 trial={trial} op={op} n={}", data.len()));
            }
            cfree(cs);
            rfree(rs);
        }
    }
}

#[test]
fn xxh64_raw_state_memcmp_sequences() {
    unsafe {
        let (cnew, rnew) = both::<FnVoidToPtr>("ZSTD_XXH64_createState");
        let (cfree, rfree) = both::<FnFreeState>("ZSTD_XXH64_freeState");
        let (crst, rrst) = both::<FnReset64>("ZSTD_XXH64_reset");
        let (cupd, rupd) = both::<FnUpdate>("ZSTD_XXH64_update");
        let mut rng = Rng::new(0xB11_000C);
        for trial in 0..200 {
            let cs = cnew();
            let rs = rnew();
            let seed = rng.next_u64();
            assert_eq!(crst(cs, seed), rrst(rs, seed));
            assert_state_eq_64(cs, rs, &format!("XXH64 reset trial={trial} seed={seed:#x}"));
            let nops = 1 + rng.below(6);
            for op in 0..nops {
                let n = rng.below(200);
                let data = gen(Shape::Random, n, &mut rng);
                let p = if data.is_empty() {
                    std::ptr::null()
                } else {
                    data.as_ptr() as *const c_void
                };
                assert_eq!(cupd(cs, p, data.len()), rupd(rs, p, data.len()));
                assert_state_eq_64(cs, rs, &format!("XXH64 trial={trial} op={op} n={}", data.len()));
            }
            cfree(cs);
            rfree(rs);
        }
    }
}

// ------------------------------------------------------------- versionNumber

#[test]
fn xxh_version_number() {
    unsafe {
        let (cv, rv) = both::<FnVersion>("ZSTD_XXH_versionNumber");
        assert_eq!(cv(), rv(), "ZSTD_XXH_versionNumber");
    }
}

// ---------------------------------------------------------------- state helpers

/// memcmp the first `XXH32_STATE_SIZE` bytes of two live state pointers.
#[track_caller]
unsafe fn assert_state_eq_32(c: *mut c_void, r: *mut c_void, ctx: &str) {
    let cb = std::slice::from_raw_parts(c as *const u8, XXH32_STATE_SIZE);
    let rb = std::slice::from_raw_parts(r as *const u8, XXH32_STATE_SIZE);
    assert_bytes_eq(ctx, cb, rb);
}

#[track_caller]
unsafe fn assert_state_eq_64(c: *mut c_void, r: *mut c_void, ctx: &str) {
    let cb = std::slice::from_raw_parts(c as *const u8, XXH64_STATE_SIZE);
    let rb = std::slice::from_raw_parts(r as *const u8, XXH64_STATE_SIZE);
    assert_bytes_eq(ctx, cb, rb);
}

// Silence unused-import warnings if a helper type is not referenced.
#[allow(dead_code)]
fn _unused(_: c_int) {}
