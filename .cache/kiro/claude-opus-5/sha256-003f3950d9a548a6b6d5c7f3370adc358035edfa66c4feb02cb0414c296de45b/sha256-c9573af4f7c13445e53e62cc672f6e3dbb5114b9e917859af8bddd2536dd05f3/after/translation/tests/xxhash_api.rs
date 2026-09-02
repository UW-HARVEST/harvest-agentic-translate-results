//! Differential tests for the namespaced xxHash API exported by LZ4.
//!
//! The C library is compiled with `XXH_NAMESPACE=LZ4_`, so every exported
//! symbol is prefixed `LZ4_`. Every function is looked up from BOTH the C and
//! the Rust `.so` and the results are compared. Rust is never called directly.
//!
//! Behavioral facts confirmed against c_src/src/xxhash.c:
//!  * `XXH_ACCEPT_NULL_INPUT_POINTER` defaults to 0 (not defined by the build),
//!    therefore `XXH*_update(state, NULL, len)` returns `XXH_ERROR` for ANY
//!    `len` (including 0), and the one-shot `XXH32/XXH64` functions do NOT guard
//!    a NULL pointer (they would dereference it), so we never pass NULL to the
//!    one-shot functions.
//!  * `XXH_OK == 0`, `XXH_ERROR == 1`.
//!  * canonical representation is big-endian.

mod common;

use common::*;
use libloading::Symbol;

// ------------------------------------------------------------------ signatures

type FnXXH32 = unsafe extern "C" fn(*const u8, usize, u32) -> u32;
type FnXXH64 = unsafe extern "C" fn(*const u8, usize, u64) -> u64;
type FnCreate = unsafe extern "C" fn() -> *mut u8;
type FnFree = unsafe extern "C" fn(*mut u8) -> i32;
type FnReset32 = unsafe extern "C" fn(*mut u8, u32) -> i32;
type FnReset64 = unsafe extern "C" fn(*mut u8, u64) -> i32;
type FnUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type FnDigest32 = unsafe extern "C" fn(*mut u8) -> u32;
type FnDigest64 = unsafe extern "C" fn(*mut u8) -> u64;
type FnCopy = unsafe extern "C" fn(*mut u8, *const u8);
type FnCanon32 = unsafe extern "C" fn(*mut u8, u32);
type FnCanon64 = unsafe extern "C" fn(*mut u8, u64);
type FnFromCanon32 = unsafe extern "C" fn(*const u8) -> u32;
type FnFromCanon64 = unsafe extern "C" fn(*const u8) -> u64;
type FnVersion = unsafe extern "C" fn() -> u32;

const XXH_OK: i32 = 0;
const XXH_ERROR: i32 = 1;

/// Pick a random shape without holding a borrow across a `make_data` call.
fn rand_shape(rng: &mut Rng) -> Shape {
    SHAPES[rng.below(7)]
}

const LENGTHS: [usize; 25] = [
    0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 255, 256, 1024, 4096,
    10000,
];

// ================================================================= XXH32 32-bit

#[test]
fn oneshot_xxh32() {
    let (c, r) = sym::<FnXXH32>("LZ4_XXH32");
    let mut rng = Rng::new(0x3200_0001);

    let mut seeds: Vec<u32> = vec![0, 1, 0xFFFF_FFFF];
    for _ in 0..6 {
        seeds.push(rng.next_u32());
    }

    for &len in LENGTHS.iter() {
        for &shape in SHAPES.iter() {
            let data = make_data(&mut rng, len, shape);
            for &seed in &seeds {
                let ctx = format!("XXH32 len={len} shape={shape:?} seed={seed:#x}");
                let cv = unsafe { c(data.as_ptr(), len, seed) };
                let rv = unsafe { r(data.as_ptr(), len, seed) };
                eq(&ctx, cv, rv);
            }
        }
    }
}

#[test]
fn oneshot_xxh64() {
    let (c, r) = sym::<FnXXH64>("LZ4_XXH64");
    let mut rng = Rng::new(0x6400_0001);

    let mut seeds: Vec<u64> = vec![0, 1, 0xFFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF, 0x0123_4567_89AB_CDEF];
    for _ in 0..6 {
        seeds.push(rng.next_u64());
    }

    for &len in LENGTHS.iter() {
        for &shape in SHAPES.iter() {
            let data = make_data(&mut rng, len, shape);
            for &seed in &seeds {
                let ctx = format!("XXH64 len={len} shape={shape:?} seed={seed:#x}");
                let cv = unsafe { c(data.as_ptr(), len, seed) };
                let rv = unsafe { r(data.as_ptr(), len, seed) };
                eq(&ctx, cv, rv);
            }
        }
    }
}

// --------------------------------------------------------------- streaming 32

/// Chunk patterns applied to a buffer. Each returns a Vec of chunk sizes that
/// sum to `len`.
fn chunk_patterns(rng: &mut Rng, len: usize) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();
    // all at once
    out.push(vec![len]);
    // one byte at a time
    out.push(vec![1; len]);
    // fixed chunk sizes
    for &cs in &[3usize, 15, 16, 17, 31, 32, 33] {
        let mut v = Vec::new();
        let mut rem = len;
        while rem > 0 {
            let n = cs.min(rem);
            v.push(n);
            rem -= n;
        }
        out.push(v);
    }
    // a couple of random chunkings
    for _ in 0..2 {
        let mut v = Vec::new();
        let mut rem = len;
        while rem > 0 {
            let n = rng.range(1, 40).min(rem);
            v.push(n);
            rem -= n;
        }
        out.push(v);
    }
    out
}

#[test]
fn streaming_xxh32() {
    let (c_os, r_os) = sym::<FnXXH32>("LZ4_XXH32");
    let (c_create, r_create) = sym::<FnCreate>("LZ4_XXH32_createState");
    let (c_free, r_free) = sym::<FnFree>("LZ4_XXH32_freeState");
    let (c_reset, r_reset) = sym::<FnReset32>("LZ4_XXH32_reset");
    let (c_update, r_update) = sym::<FnUpdate>("LZ4_XXH32_update");
    let (c_digest, r_digest) = sym::<FnDigest32>("LZ4_XXH32_digest");

    let mut rng = Rng::new(0x3200_1111);
    let seeds: [u32; 3] = [0, 1, 0xDEAD_BEEF];

    for &len in LENGTHS.iter() {
        for &shape in [Shape::Random, Shape::LowEntropy, Shape::Zeros, Shape::Mixed].iter() {
            let data = make_data(&mut rng, len, shape);
            for &seed in &seeds {
                let oneshot_c = unsafe { c_os(data.as_ptr(), len, seed) };
                let oneshot_r = unsafe { r_os(data.as_ptr(), len, seed) };
                let ctx0 = format!("XXH32 oneshot len={len} shape={shape:?} seed={seed:#x}");
                eq(&ctx0, oneshot_c, oneshot_r);

                for chunks in chunk_patterns(&mut rng, len) {
                    let ctx = format!(
                        "XXH32 stream len={len} shape={shape:?} seed={seed:#x} nchunks={}",
                        chunks.len()
                    );

                    // C
                    let cs = unsafe { c_create() };
                    assert!(!cs.is_null(), "{ctx}: C createState returned null");
                    eq(&format!("{ctx} reset"), unsafe { c_reset(cs, seed) }, XXH_OK);
                    let mut off = 0usize;
                    for &n in &chunks {
                        let rc = unsafe { c_update(cs, data.as_ptr().add(off), n) };
                        eq(&format!("{ctx} C update"), rc, XXH_OK);
                        off += n;
                    }
                    let c_stream = unsafe { c_digest(cs) };
                    eq(&format!("{ctx} C free"), unsafe { c_free(cs) }, XXH_OK);

                    // Rust
                    let rs = unsafe { r_create() };
                    assert!(!rs.is_null(), "{ctx}: Rust createState returned null");
                    eq(&format!("{ctx} reset"), unsafe { r_reset(rs, seed) }, XXH_OK);
                    let mut off = 0usize;
                    for &n in &chunks {
                        let rc = unsafe { r_update(rs, data.as_ptr().add(off), n) };
                        eq(&format!("{ctx} Rust update"), rc, XXH_OK);
                        off += n;
                    }
                    let r_stream = unsafe { r_digest(rs) };
                    eq(&format!("{ctx} Rust free"), unsafe { r_free(rs) }, XXH_OK);

                    // streaming C == streaming Rust
                    eq(&format!("{ctx} stream C vs Rust"), c_stream, r_stream);
                    // streaming == one-shot of the same library
                    eq(&format!("{ctx} C stream vs C oneshot"), c_stream, oneshot_c);
                    eq(&format!("{ctx} Rust stream vs Rust oneshot"), r_stream, oneshot_r);
                }
            }
        }
    }
}

#[test]
fn streaming_xxh64() {
    let (c_os, r_os) = sym::<FnXXH64>("LZ4_XXH64");
    let (c_create, r_create) = sym::<FnCreate>("LZ4_XXH64_createState");
    let (c_free, r_free) = sym::<FnFree>("LZ4_XXH64_freeState");
    let (c_reset, r_reset) = sym::<FnReset64>("LZ4_XXH64_reset");
    let (c_update, r_update) = sym::<FnUpdate>("LZ4_XXH64_update");
    let (c_digest, r_digest) = sym::<FnDigest64>("LZ4_XXH64_digest");

    let mut rng = Rng::new(0x6400_1111);
    let seeds: [u64; 3] = [0, 1, 0xDEAD_BEEF_CAFE_F00D];

    for &len in LENGTHS.iter() {
        for &shape in [Shape::Random, Shape::LowEntropy, Shape::Zeros, Shape::Mixed].iter() {
            let data = make_data(&mut rng, len, shape);
            for &seed in &seeds {
                let oneshot_c = unsafe { c_os(data.as_ptr(), len, seed) };
                let oneshot_r = unsafe { r_os(data.as_ptr(), len, seed) };
                let ctx0 = format!("XXH64 oneshot len={len} shape={shape:?} seed={seed:#x}");
                eq(&ctx0, oneshot_c, oneshot_r);

                for chunks in chunk_patterns(&mut rng, len) {
                    let ctx = format!(
                        "XXH64 stream len={len} shape={shape:?} seed={seed:#x} nchunks={}",
                        chunks.len()
                    );

                    let cs = unsafe { c_create() };
                    assert!(!cs.is_null(), "{ctx}: C createState returned null");
                    eq(&format!("{ctx} reset"), unsafe { c_reset(cs, seed) }, XXH_OK);
                    let mut off = 0usize;
                    for &n in &chunks {
                        let rc = unsafe { c_update(cs, data.as_ptr().add(off), n) };
                        eq(&format!("{ctx} C update"), rc, XXH_OK);
                        off += n;
                    }
                    let c_stream = unsafe { c_digest(cs) };
                    eq(&format!("{ctx} C free"), unsafe { c_free(cs) }, XXH_OK);

                    let rs = unsafe { r_create() };
                    assert!(!rs.is_null(), "{ctx}: Rust createState returned null");
                    eq(&format!("{ctx} reset"), unsafe { r_reset(rs, seed) }, XXH_OK);
                    let mut off = 0usize;
                    for &n in &chunks {
                        let rc = unsafe { r_update(rs, data.as_ptr().add(off), n) };
                        eq(&format!("{ctx} Rust update"), rc, XXH_OK);
                        off += n;
                    }
                    let r_stream = unsafe { r_digest(rs) };
                    eq(&format!("{ctx} Rust free"), unsafe { r_free(rs) }, XXH_OK);

                    eq(&format!("{ctx} stream C vs Rust"), c_stream, r_stream);
                    eq(&format!("{ctx} C stream vs C oneshot"), c_stream, oneshot_c);
                    eq(&format!("{ctx} Rust stream vs Rust oneshot"), r_stream, oneshot_r);
                }
            }
        }
    }
}

// --------------------------------------------------------------- copyState 32

#[test]
fn copy_state_xxh32() {
    let (c_create, r_create) = sym::<FnCreate>("LZ4_XXH32_createState");
    let (c_free, r_free) = sym::<FnFree>("LZ4_XXH32_freeState");
    let (c_reset, r_reset) = sym::<FnReset32>("LZ4_XXH32_reset");
    let (c_update, r_update) = sym::<FnUpdate>("LZ4_XXH32_update");
    let (c_digest, r_digest) = sym::<FnDigest32>("LZ4_XXH32_digest");
    let (c_copy, r_copy) = sym::<FnCopy>("LZ4_XXH32_copyState");

    let mut rng = Rng::new(0x3200_2222);

    for iter in 0..40 {
        let prefix_len = rng.range(0, 300);
        let a_len = rng.range(0, 300);
        let b_len = rng.range(0, 300);
        let seed = rng.next_u32();
        let prefix = { let sh = rand_shape(&mut rng); make_data(&mut rng, prefix_len, sh) };
        let tail_a = { let sh = rand_shape(&mut rng); make_data(&mut rng, a_len, sh) };
        let tail_b = { let sh = rand_shape(&mut rng); make_data(&mut rng, b_len, sh) };
        let ctx = format!("XXH32 copy iter={iter} pre={prefix_len} a={a_len} b={b_len} seed={seed:#x}");

        // Compute per-library the digests for (orig -> tail_a) and (copy -> tail_b).
        let run = |create: &Symbol<FnCreate>,
                   free: &Symbol<FnFree>,
                   reset: &Symbol<FnReset32>,
                   update: &Symbol<FnUpdate>,
                   digest: &Symbol<FnDigest32>,
                   copy: &Symbol<FnCopy>|
         -> (u32, u32) {
            unsafe {
                let orig = create();
                assert!(!orig.is_null());
                assert_eq!(reset(orig, seed), XXH_OK);
                assert_eq!(update(orig, prefix.as_ptr(), prefix_len), XXH_OK);
                // copy into second state after common prefix
                let copyd = create();
                assert!(!copyd.is_null());
                copy(copyd, orig as *const u8);
                // feed DIFFERENT tails
                assert_eq!(update(orig, tail_a.as_ptr(), a_len), XXH_OK);
                assert_eq!(update(copyd, tail_b.as_ptr(), b_len), XXH_OK);
                let da = digest(orig);
                let db = digest(copyd);
                assert_eq!(free(orig), XXH_OK);
                assert_eq!(free(copyd), XXH_OK);
                (da, db)
            }
        };

        let (ca, cb) = run(&c_create, &c_free, &c_reset, &c_update, &c_digest, &c_copy);
        let (ra, rb) = run(&r_create, &r_free, &r_reset, &r_update, &r_digest, &r_copy);
        eq(&format!("{ctx} orig+tailA"), ca, ra);
        eq(&format!("{ctx} copy+tailB"), cb, rb);
    }
}

#[test]
fn copy_state_xxh64() {
    let (c_create, r_create) = sym::<FnCreate>("LZ4_XXH64_createState");
    let (c_free, r_free) = sym::<FnFree>("LZ4_XXH64_freeState");
    let (c_reset, r_reset) = sym::<FnReset64>("LZ4_XXH64_reset");
    let (c_update, r_update) = sym::<FnUpdate>("LZ4_XXH64_update");
    let (c_digest, r_digest) = sym::<FnDigest64>("LZ4_XXH64_digest");
    let (c_copy, r_copy) = sym::<FnCopy>("LZ4_XXH64_copyState");

    let mut rng = Rng::new(0x6400_2222);

    for iter in 0..40 {
        let prefix_len = rng.range(0, 400);
        let a_len = rng.range(0, 400);
        let b_len = rng.range(0, 400);
        let seed = rng.next_u64();
        let prefix = { let sh = rand_shape(&mut rng); make_data(&mut rng, prefix_len, sh) };
        let tail_a = { let sh = rand_shape(&mut rng); make_data(&mut rng, a_len, sh) };
        let tail_b = { let sh = rand_shape(&mut rng); make_data(&mut rng, b_len, sh) };
        let ctx = format!("XXH64 copy iter={iter} pre={prefix_len} a={a_len} b={b_len} seed={seed:#x}");

        let run = |create: &Symbol<FnCreate>,
                   free: &Symbol<FnFree>,
                   reset: &Symbol<FnReset64>,
                   update: &Symbol<FnUpdate>,
                   digest: &Symbol<FnDigest64>,
                   copy: &Symbol<FnCopy>|
         -> (u64, u64) {
            unsafe {
                let orig = create();
                assert!(!orig.is_null());
                assert_eq!(reset(orig, seed), XXH_OK);
                assert_eq!(update(orig, prefix.as_ptr(), prefix_len), XXH_OK);
                let copyd = create();
                assert!(!copyd.is_null());
                copy(copyd, orig as *const u8);
                assert_eq!(update(orig, tail_a.as_ptr(), a_len), XXH_OK);
                assert_eq!(update(copyd, tail_b.as_ptr(), b_len), XXH_OK);
                let da = digest(orig);
                let db = digest(copyd);
                assert_eq!(free(orig), XXH_OK);
                assert_eq!(free(copyd), XXH_OK);
                (da, db)
            }
        };

        let (ca, cb) = run(&c_create, &c_free, &c_reset, &c_update, &c_digest, &c_copy);
        let (ra, rb) = run(&r_create, &r_free, &r_reset, &r_update, &r_digest, &r_copy);
        eq(&format!("{ctx} orig+tailA"), ca, ra);
        eq(&format!("{ctx} copy+tailB"), cb, rb);
    }
}

// --------------------------------------------------- digest is non-destructive

#[test]
fn digest_is_non_destructive() {
    // XXH32
    {
        let (c_create, r_create) = sym::<FnCreate>("LZ4_XXH32_createState");
        let (c_free, r_free) = sym::<FnFree>("LZ4_XXH32_freeState");
        let (c_reset, r_reset) = sym::<FnReset32>("LZ4_XXH32_reset");
        let (c_update, r_update) = sym::<FnUpdate>("LZ4_XXH32_update");
        let (c_digest, r_digest) = sym::<FnDigest32>("LZ4_XXH32_digest");

        let mut rng = Rng::new(0x3200_3333);
        for iter in 0..30 {
            let l1 = rng.range(0, 200);
            let l2 = rng.range(0, 200);
            let seed = rng.next_u32();
            let d1 = { let sh = rand_shape(&mut rng); make_data(&mut rng, l1, sh) };
            let d2 = { let sh = rand_shape(&mut rng); make_data(&mut rng, l2, sh) };
            let ctx = format!("XXH32 nondestructive iter={iter} l1={l1} l2={l2} seed={seed:#x}");

            let run = |create: &Symbol<FnCreate>,
                       free: &Symbol<FnFree>,
                       reset: &Symbol<FnReset32>,
                       update: &Symbol<FnUpdate>,
                       digest: &Symbol<FnDigest32>|
             -> (u32, u32, u32) {
                unsafe {
                    let s = create();
                    assert_eq!(reset(s, seed), XXH_OK);
                    assert_eq!(update(s, d1.as_ptr(), l1), XXH_OK);
                    let a = digest(s);
                    let b = digest(s); // twice, no update in between
                    assert_eq!(update(s, d2.as_ptr(), l2), XXH_OK);
                    let cc = digest(s);
                    assert_eq!(free(s), XXH_OK);
                    (a, b, cc)
                }
            };
            let (ca, cb, ccc) = run(&c_create, &c_free, &c_reset, &c_update, &c_digest);
            let (ra, rb, rcc) = run(&r_create, &r_free, &r_reset, &r_update, &r_digest);
            // digest called twice yields the same value within a library
            eq(&format!("{ctx} C digest twice"), ca, cb);
            eq(&format!("{ctx} Rust digest twice"), ra, rb);
            // C vs Rust across all three digests
            eq(&format!("{ctx} first digest"), ca, ra);
            eq(&format!("{ctx} second digest"), cb, rb);
            eq(&format!("{ctx} after more updates"), ccc, rcc);
        }
    }
    // XXH64
    {
        let (c_create, r_create) = sym::<FnCreate>("LZ4_XXH64_createState");
        let (c_free, r_free) = sym::<FnFree>("LZ4_XXH64_freeState");
        let (c_reset, r_reset) = sym::<FnReset64>("LZ4_XXH64_reset");
        let (c_update, r_update) = sym::<FnUpdate>("LZ4_XXH64_update");
        let (c_digest, r_digest) = sym::<FnDigest64>("LZ4_XXH64_digest");

        let mut rng = Rng::new(0x6400_3333);
        for iter in 0..30 {
            let l1 = rng.range(0, 250);
            let l2 = rng.range(0, 250);
            let seed = rng.next_u64();
            let d1 = { let sh = rand_shape(&mut rng); make_data(&mut rng, l1, sh) };
            let d2 = { let sh = rand_shape(&mut rng); make_data(&mut rng, l2, sh) };
            let ctx = format!("XXH64 nondestructive iter={iter} l1={l1} l2={l2} seed={seed:#x}");

            let run = |create: &Symbol<FnCreate>,
                       free: &Symbol<FnFree>,
                       reset: &Symbol<FnReset64>,
                       update: &Symbol<FnUpdate>,
                       digest: &Symbol<FnDigest64>|
             -> (u64, u64, u64) {
                unsafe {
                    let s = create();
                    assert_eq!(reset(s, seed), XXH_OK);
                    assert_eq!(update(s, d1.as_ptr(), l1), XXH_OK);
                    let a = digest(s);
                    let b = digest(s);
                    assert_eq!(update(s, d2.as_ptr(), l2), XXH_OK);
                    let cc = digest(s);
                    assert_eq!(free(s), XXH_OK);
                    (a, b, cc)
                }
            };
            let (ca, cb, ccc) = run(&c_create, &c_free, &c_reset, &c_update, &c_digest);
            let (ra, rb, rcc) = run(&r_create, &r_free, &r_reset, &r_update, &r_digest);
            eq(&format!("{ctx} C digest twice"), ca, cb);
            eq(&format!("{ctx} Rust digest twice"), ra, rb);
            eq(&format!("{ctx} first digest"), ca, ra);
            eq(&format!("{ctx} second digest"), cb, rb);
            eq(&format!("{ctx} after more updates"), ccc, rcc);
        }
    }
}

// ------------------------------------------------------------- canonical 32/64

#[test]
fn canonical_roundtrip_32() {
    let (c_canon, r_canon) = sym::<FnCanon32>("LZ4_XXH32_canonicalFromHash");
    let (c_from, r_from) = sym::<FnFromCanon32>("LZ4_XXH32_hashFromCanonical");

    let mut rng = Rng::new(0x3200_4444);

    // canonicalFromHash on many random hashes: compare raw big-endian bytes,
    // then round-trip through hashFromCanonical.
    let mut hashes: Vec<u32> = vec![0, 1, 0xFFFF_FFFF, 0x0000_00FF, 0xFF00_0000, 0x12345678];
    for _ in 0..2000 {
        hashes.push(rng.next_u32());
    }
    for &h in &hashes {
        let mut cbuf = [0u8; 4];
        let mut rbuf = [0u8; 4];
        unsafe {
            c_canon(cbuf.as_mut_ptr(), h);
            r_canon(rbuf.as_mut_ptr(), h);
        }
        eq_bytes(&format!("XXH32 canonicalFromHash h={h:#x}"), &cbuf, &rbuf);
        // canonical is big-endian
        eq_bytes(
            &format!("XXH32 canonical big-endian h={h:#x}"),
            &cbuf,
            &h.to_be_bytes(),
        );
        let cb = unsafe { c_from(cbuf.as_ptr()) };
        let rb = unsafe { r_from(rbuf.as_ptr()) };
        eq(&format!("XXH32 hashFromCanonical roundtrip h={h:#x}"), cb, rb);
        eq(&format!("XXH32 roundtrip equals original h={h:#x}"), cb, h);
    }

    // arbitrary random canonical byte arrays
    for _ in 0..2000 {
        let mut buf = [0u8; 4];
        for b in buf.iter_mut() {
            *b = rng.byte();
        }
        let cv = unsafe { c_from(buf.as_ptr()) };
        let rv = unsafe { r_from(buf.as_ptr()) };
        eq(&format!("XXH32 hashFromCanonical arbitrary {buf:02x?}"), cv, rv);
        eq(
            &format!("XXH32 hashFromCanonical arbitrary big-endian {buf:02x?}"),
            cv,
            u32::from_be_bytes(buf),
        );
    }
}

#[test]
fn canonical_roundtrip_64() {
    let (c_canon, r_canon) = sym::<FnCanon64>("LZ4_XXH64_canonicalFromHash");
    let (c_from, r_from) = sym::<FnFromCanon64>("LZ4_XXH64_hashFromCanonical");

    let mut rng = Rng::new(0x6400_4444);

    let mut hashes: Vec<u64> = vec![
        0,
        1,
        0xFFFF_FFFF_FFFF_FFFF,
        0x0000_0000_0000_00FF,
        0xFF00_0000_0000_0000,
        0x0123_4567_89AB_CDEF,
    ];
    for _ in 0..2000 {
        hashes.push(rng.next_u64());
    }
    for &h in &hashes {
        let mut cbuf = [0u8; 8];
        let mut rbuf = [0u8; 8];
        unsafe {
            c_canon(cbuf.as_mut_ptr(), h);
            r_canon(rbuf.as_mut_ptr(), h);
        }
        eq_bytes(&format!("XXH64 canonicalFromHash h={h:#x}"), &cbuf, &rbuf);
        eq_bytes(
            &format!("XXH64 canonical big-endian h={h:#x}"),
            &cbuf,
            &h.to_be_bytes(),
        );
        let cb = unsafe { c_from(cbuf.as_ptr()) };
        let rb = unsafe { r_from(rbuf.as_ptr()) };
        eq(&format!("XXH64 hashFromCanonical roundtrip h={h:#x}"), cb, rb);
        eq(&format!("XXH64 roundtrip equals original h={h:#x}"), cb, h);
    }

    for _ in 0..2000 {
        let mut buf = [0u8; 8];
        for b in buf.iter_mut() {
            *b = rng.byte();
        }
        let cv = unsafe { c_from(buf.as_ptr()) };
        let rv = unsafe { r_from(buf.as_ptr()) };
        eq(&format!("XXH64 hashFromCanonical arbitrary {buf:02x?}"), cv, rv);
        eq(
            &format!("XXH64 hashFromCanonical arbitrary big-endian {buf:02x?}"),
            cv,
            u64::from_be_bytes(buf),
        );
    }
}

// ----------------------------------------------------------------- version

#[test]
fn version_number() {
    let (c, r) = sym::<FnVersion>("LZ4_XXH_versionNumber");
    let cv = unsafe { c() };
    let rv = unsafe { r() };
    eq("XXH_versionNumber", cv, rv);
    // header declares XXH_VERSION_NUMBER = 0*10000 + 6*100 + 5 = 605
    eq("XXH_versionNumber value", cv, 605u32);
}

// --------------------------------------------------------------- null & zero

#[test]
fn null_and_zero_len() {
    // Behavior (XXH_ACCEPT_NULL_INPUT_POINTER == 0 in this build):
    //   update(state, NULL, len) -> XXH_ERROR for ANY len, including 0.
    // We assert C and Rust return the SAME code (whatever it is), and that the
    // observed code matches the C source (XXH_ERROR).

    // ---- XXH32 update NULL ----
    {
        let (c_create, r_create) = sym::<FnCreate>("LZ4_XXH32_createState");
        let (c_free, r_free) = sym::<FnFree>("LZ4_XXH32_freeState");
        let (c_reset, r_reset) = sym::<FnReset32>("LZ4_XXH32_reset");
        let (c_update, r_update) = sym::<FnUpdate>("LZ4_XXH32_update");

        let null: *const u8 = std::ptr::null();

        // NULL + len == 0
        let cc0 = unsafe {
            let s = c_create();
            assert_eq!(c_reset(s, 0), XXH_OK);
            let rc = c_update(s, null, 0);
            assert_eq!(c_free(s), XXH_OK);
            rc
        };
        let rc0 = unsafe {
            let s = r_create();
            assert_eq!(r_reset(s, 0), XXH_OK);
            let rc = r_update(s, null, 0);
            assert_eq!(r_free(s), XXH_OK);
            rc
        };
        eq("XXH32 update NULL len=0 C vs Rust", cc0, rc0);
        eq("XXH32 update NULL len=0 == XXH_ERROR", cc0, XXH_ERROR);

        // NULL + len != 0 (guarded in C, returns before dereference)
        let cc1 = unsafe {
            let s = c_create();
            assert_eq!(c_reset(s, 0), XXH_OK);
            let rc = c_update(s, null, 123);
            assert_eq!(c_free(s), XXH_OK);
            rc
        };
        let rc1 = unsafe {
            let s = r_create();
            assert_eq!(r_reset(s, 0), XXH_OK);
            let rc = r_update(s, null, 123);
            assert_eq!(r_free(s), XXH_OK);
            rc
        };
        eq("XXH32 update NULL len!=0 C vs Rust", cc1, rc1);
        eq("XXH32 update NULL len!=0 == XXH_ERROR", cc1, XXH_ERROR);
    }

    // ---- XXH64 update NULL ----
    {
        let (c_create, r_create) = sym::<FnCreate>("LZ4_XXH64_createState");
        let (c_free, r_free) = sym::<FnFree>("LZ4_XXH64_freeState");
        let (c_reset, r_reset) = sym::<FnReset64>("LZ4_XXH64_reset");
        let (c_update, r_update) = sym::<FnUpdate>("LZ4_XXH64_update");

        let null: *const u8 = std::ptr::null();

        let cc0 = unsafe {
            let s = c_create();
            assert_eq!(c_reset(s, 0), XXH_OK);
            let rc = c_update(s, null, 0);
            assert_eq!(c_free(s), XXH_OK);
            rc
        };
        let rc0 = unsafe {
            let s = r_create();
            assert_eq!(r_reset(s, 0), XXH_OK);
            let rc = r_update(s, null, 0);
            assert_eq!(r_free(s), XXH_OK);
            rc
        };
        eq("XXH64 update NULL len=0 C vs Rust", cc0, rc0);
        eq("XXH64 update NULL len=0 == XXH_ERROR", cc0, XXH_ERROR);

        let cc1 = unsafe {
            let s = c_create();
            assert_eq!(c_reset(s, 0), XXH_OK);
            let rc = c_update(s, null, 123);
            assert_eq!(c_free(s), XXH_OK);
            rc
        };
        let rc1 = unsafe {
            let s = r_create();
            assert_eq!(r_reset(s, 0), XXH_OK);
            let rc = r_update(s, null, 123);
            assert_eq!(r_free(s), XXH_OK);
            rc
        };
        eq("XXH64 update NULL len!=0 C vs Rust", cc1, rc1);
        eq("XXH64 update NULL len!=0 == XXH_ERROR", cc1, XXH_ERROR);
    }

    // ---- one-shot with len == 0 (non-null pointer; NULL is NOT guarded in
    //      the one-shot path in this build, so we never pass NULL there) ----
    {
        let (c32, r32) = sym::<FnXXH32>("LZ4_XXH32");
        let (c64, r64) = sym::<FnXXH64>("LZ4_XXH64");
        let dummy = [0u8; 1]; // valid, readable pointer; len==0 reads nothing

        for &seed in &[0u32, 1, 0xFFFF_FFFF] {
            let cv = unsafe { c32(dummy.as_ptr(), 0, seed) };
            let rv = unsafe { r32(dummy.as_ptr(), 0, seed) };
            eq(&format!("XXH32 oneshot len=0 seed={seed:#x}"), cv, rv);
        }
        for &seed in &[0u64, 1, 0xFFFF_FFFF_FFFF_FFFF] {
            let cv = unsafe { c64(dummy.as_ptr(), 0, seed) };
            let rv = unsafe { r64(dummy.as_ptr(), 0, seed) };
            eq(&format!("XXH64 oneshot len=0 seed={seed:#x}"), cv, rv);
        }
    }
}
