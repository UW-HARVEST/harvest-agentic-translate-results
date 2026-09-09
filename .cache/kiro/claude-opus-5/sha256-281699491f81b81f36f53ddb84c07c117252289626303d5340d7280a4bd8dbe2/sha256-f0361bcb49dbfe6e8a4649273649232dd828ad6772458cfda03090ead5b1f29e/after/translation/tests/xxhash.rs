//! Phase B — CONFIGS.md group 12: `xxhash.c`, exported as `LZ4_XXH*` because the
//! build defines `XXH_NAMESPACE=LZ4_`.
mod common;
use common::*;

const SEED: u64 = 0x58_5848_0012;

type FnXxh32 = unsafe extern "C" fn(*const u8, usize, u32) -> u32;
type FnXxh64 = unsafe extern "C" fn(*const u8, usize, u64) -> u64;
type FnCreate = unsafe extern "C" fn() -> *mut u8;
type FnFree = unsafe extern "C" fn(*mut u8) -> i32;
type FnCopy = unsafe extern "C" fn(*mut u8, *const u8);
type FnReset32 = unsafe extern "C" fn(*mut u8, u32) -> i32;
type FnReset64 = unsafe extern "C" fn(*mut u8, u64) -> i32;
type FnUpdate = unsafe extern "C" fn(*mut u8, *const u8, usize) -> i32;
type FnDigest32 = unsafe extern "C" fn(*const u8) -> u32;
type FnDigest64 = unsafe extern "C" fn(*const u8) -> u64;
type FnCanon32 = unsafe extern "C" fn(*mut u8, u32);
type FnCanon64 = unsafe extern "C" fn(*mut u8, u64);
type FnFromCanon32 = unsafe extern "C" fn(*const u8) -> u32;
type FnFromCanon64 = unsafe extern "C" fn(*const u8) -> u64;
type FnVersion = unsafe extern "C" fn() -> u32;

/// Length classes: every value below the stripe size, every stripe boundary,
/// and every remainder class of the finalize `switch`.
fn lengths() -> Vec<usize> {
    let mut v: Vec<usize> = (0..=40).collect();
    v.extend([
        41, 47, 48, 49, 63, 64, 65, 95, 96, 97, 127, 128, 129, 255, 256, 257, 511, 512, 1000, 4096,
        4097, 65536, 100_000,
    ]);
    v
}

/// Row 224.
#[test]
fn g12_version() {
    let (c, r) = syms::<FnVersion>("LZ4_XXH_versionNumber");
    let cv = unsafe { c() };
    assert_eq!(cv, unsafe { r() }, "LZ4_XXH_versionNumber");
    // XXH_VERSION_NUMBER = MAJOR*100*100 + MINOR*100 + RELEASE = 0 + 600 + 5
    assert_eq!(cv, 605, "unexpected xxhash version");
}

/// Rows 200-206: one-shot XXH32 over every length class, seed class, payload
/// shape and pointer alignment.
#[test]
fn g12_xxh32_oneshot() {
    let (c, r) = syms::<FnXxh32>("LZ4_XXH32");
    let mut rng = Rng::new(SEED);
    let seeds: [u32; 7] = [0, 1, 2, 0x7FFF_FFFF, 0x8000_0000, 0xDEAD_BEEF, 0xFFFF_FFFF];
    // one big backing buffer so we can slide the start pointer for alignment
    let backing = mkdata(Shape::Random, 100_016, &mut rng);
    for len in lengths() {
        for &seed in seeds.iter() {
            for align in 0..8usize {
                if align + len > backing.len() {
                    continue;
                }
                let p = unsafe { backing.as_ptr().add(align) };
                let a = unsafe { c(p, len, seed) };
                let b = unsafe { r(p, len, seed) };
                assert_eq!(a, b, "LZ4_XXH32 len={len} seed={seed:#x} align={align}: C={a:#x} R={b:#x}");
            }
        }
    }
    // shape variation (content-sensitive avalanche)
    for &shape in ALL_SHAPES.iter() {
        for _ in 0..60 {
            let len = rng.range(0, 3000);
            let src = mkdata(shape, len, &mut rng);
            let seed = rng.next_u32();
            let a = unsafe { c(src.as_ptr(), len, seed) };
            let b = unsafe { r(src.as_ptr(), len, seed) };
            assert_eq!(a, b, "LZ4_XXH32 {shape:?} len={len} seed={seed:#x}");
        }
    }
}

/// Rows 207-213: one-shot XXH64, same matrix.
#[test]
fn g12_xxh64_oneshot() {
    let (c, r) = syms::<FnXxh64>("LZ4_XXH64");
    let mut rng = Rng::new(SEED ^ 1);
    let seeds: [u64; 8] = [
        0,
        1,
        2,
        0xFFFF_FFFF,
        0x1_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        0xDEAD_BEEF_CAFE_BABE,
        u64::MAX,
    ];
    let backing = mkdata(Shape::Random, 100_016, &mut rng);
    for len in lengths() {
        for &seed in seeds.iter() {
            for align in 0..8usize {
                if align + len > backing.len() {
                    continue;
                }
                let p = unsafe { backing.as_ptr().add(align) };
                let a = unsafe { c(p, len, seed) };
                let b = unsafe { r(p, len, seed) };
                assert_eq!(a, b, "LZ4_XXH64 len={len} seed={seed:#x} align={align}: C={a:#x} R={b:#x}");
            }
        }
    }
    for &shape in ALL_SHAPES.iter() {
        for _ in 0..60 {
            let len = rng.range(0, 3000);
            let src = mkdata(shape, len, &mut rng);
            let seed = rng.next_u64();
            let a = unsafe { c(src.as_ptr(), len, seed) };
            let b = unsafe { r(src.as_ptr(), len, seed) };
            assert_eq!(a, b, "LZ4_XXH64 {shape:?} len={len} seed={seed:#x}");
        }
    }
}

/// Rows 214-219: XXH32 streaming — whole-buffer, 1-byte, and random chunk splits
/// crossing the 16-byte internal buffer, plus `copyState` and repeated `digest`.
#[test]
fn g12_xxh32_streaming() {
    let (cc, rc) = syms::<FnCreate>("LZ4_XXH32_createState");
    let (cf, rf) = syms::<FnFree>("LZ4_XXH32_freeState");
    let (ccp, rcp) = syms::<FnCopy>("LZ4_XXH32_copyState");
    let (cr, rr) = syms::<FnReset32>("LZ4_XXH32_reset");
    let (cu, ru) = syms::<FnUpdate>("LZ4_XXH32_update");
    let (cdg, rdg) = syms::<FnDigest32>("LZ4_XXH32_digest");
    let (c1, r1) = syms::<FnXxh32>("LZ4_XXH32");
    let mut rng = Rng::new(SEED ^ 2);

    // chunking strategies: 0 = whole, 1 = 1 byte, 2 = random, 3 = fixed 16, 4 = fixed 15, 5 = fixed 17
    for strategy in 0..6usize {
        for len in [
            0usize, 1, 3, 4, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 1000, 4096, 65537,
        ] {
            for &seed in &[0u32, 1, 0xFFFF_FFFF] {
                let src = mkdata(ALL_SHAPES[rng.below(ALL_SHAPES.len())], len, &mut rng);
                let cs = unsafe { cc() };
                let rs = unsafe { rc() };
                assert!(!cs.is_null() && !rs.is_null());
                assert_eq!(unsafe { cr(cs, seed) }, unsafe { rr(rs, seed) }, "XXH32_reset");

                let mut off = 0usize;
                let mut copy_c: *mut u8 = std::ptr::null_mut();
                let mut copy_r: *mut u8 = std::ptr::null_mut();
                let mut half_done = false;
                while off < len {
                    let n = match strategy {
                        0 => len - off,
                        1 => 1,
                        2 => rng.range(1, 40).min(len - off),
                        3 => 16usize.min(len - off),
                        4 => 15usize.min(len - off),
                        _ => 17usize.min(len - off),
                    };
                    let p = unsafe { src.as_ptr().add(off) };
                    let a = unsafe { cu(cs, p, n) };
                    let b = unsafe { ru(rs, p, n) };
                    assert_eq!(a, b, "XXH32_update len={len} strat={strategy} off={off} n={n}");
                    off += n;
                    // Row 219: mid-stream copyState, then keep hashing both
                    if !half_done && off * 2 >= len && len > 0 {
                        half_done = true;
                        copy_c = unsafe { cc() };
                        copy_r = unsafe { rc() };
                        unsafe { ccp(copy_c, cs) };
                        unsafe { rcp(copy_r, rs) };
                        let dc = unsafe { cdg(copy_c) };
                        let dr = unsafe { rdg(copy_r) };
                        assert_eq!(dc, dr, "XXH32 copyState digest len={len} strat={strategy}");
                    }
                }
                // Row 217: zero-length update with a valid pointer
                assert_eq!(unsafe { cu(cs, src.as_ptr(), 0) }, unsafe { ru(rs, src.as_ptr(), 0) });
                // Row 218: digest is idempotent
                let d1c = unsafe { cdg(cs) };
                let d1r = unsafe { rdg(rs) };
                let d2c = unsafe { cdg(cs) };
                let d2r = unsafe { rdg(rs) };
                assert_eq!(d1c, d1r, "XXH32 digest len={len} strat={strategy} seed={seed:#x}");
                assert_eq!(d1c, d2c, "XXH32 C digest not idempotent");
                assert_eq!(d1r, d2r, "XXH32 Rust digest not idempotent");
                // streaming must equal one-shot in both libraries
                let oc = unsafe { c1(src.as_ptr(), len, seed) };
                let or = unsafe { r1(src.as_ptr(), len, seed) };
                assert_eq!(d1c, oc, "C: streaming != one-shot len={len} strat={strategy}");
                assert_eq!(d1r, or, "Rust: streaming != one-shot len={len} strat={strategy}");

                if !copy_c.is_null() {
                    unsafe { cf(copy_c) };
                    unsafe { rf(copy_r) };
                }
                assert_eq!(unsafe { cf(cs) }, unsafe { rf(rs) }, "XXH32_freeState");
            }
        }
    }
}

/// Rows 220-221: XXH64 streaming, same strategies around the 32-byte buffer.
#[test]
fn g12_xxh64_streaming() {
    let (cc, rc) = syms::<FnCreate>("LZ4_XXH64_createState");
    let (cf, rf) = syms::<FnFree>("LZ4_XXH64_freeState");
    let (ccp, rcp) = syms::<FnCopy>("LZ4_XXH64_copyState");
    let (cr, rr) = syms::<FnReset64>("LZ4_XXH64_reset");
    let (cu, ru) = syms::<FnUpdate>("LZ4_XXH64_update");
    let (cdg, rdg) = syms::<FnDigest64>("LZ4_XXH64_digest");
    let (c1, r1) = syms::<FnXxh64>("LZ4_XXH64");
    let mut rng = Rng::new(SEED ^ 3);

    for strategy in 0..7usize {
        for len in [
            0usize, 1, 7, 8, 16, 24, 31, 32, 33, 63, 64, 65, 127, 128, 1000, 4096, 65537,
        ] {
            for &seed in &[0u64, 1, 0xFFFF_FFFF, u64::MAX] {
                let src = mkdata(ALL_SHAPES[rng.below(ALL_SHAPES.len())], len, &mut rng);
                let cs = unsafe { cc() };
                let rs = unsafe { rc() };
                assert_eq!(unsafe { cr(cs, seed) }, unsafe { rr(rs, seed) }, "XXH64_reset");
                let mut off = 0usize;
                let mut copy_c: *mut u8 = std::ptr::null_mut();
                let mut copy_r: *mut u8 = std::ptr::null_mut();
                let mut half_done = false;
                while off < len {
                    let n = match strategy {
                        0 => len - off,
                        1 => 1,
                        2 => rng.range(1, 70).min(len - off),
                        3 => 32usize.min(len - off),
                        4 => 31usize.min(len - off),
                        5 => 33usize.min(len - off),
                        _ => 8usize.min(len - off),
                    };
                    let p = unsafe { src.as_ptr().add(off) };
                    let a = unsafe { cu(cs, p, n) };
                    let b = unsafe { ru(rs, p, n) };
                    assert_eq!(a, b, "XXH64_update len={len} strat={strategy} off={off} n={n}");
                    off += n;
                    if !half_done && off * 2 >= len && len > 0 {
                        half_done = true;
                        copy_c = unsafe { cc() };
                        copy_r = unsafe { rc() };
                        unsafe { ccp(copy_c, cs) };
                        unsafe { rcp(copy_r, rs) };
                        assert_eq!(
                            unsafe { cdg(copy_c) },
                            unsafe { rdg(copy_r) },
                            "XXH64 copyState digest len={len} strat={strategy}"
                        );
                    }
                }
                assert_eq!(unsafe { cu(cs, src.as_ptr(), 0) }, unsafe { ru(rs, src.as_ptr(), 0) });
                let dc = unsafe { cdg(cs) };
                let dr = unsafe { rdg(rs) };
                assert_eq!(dc, dr, "XXH64 digest len={len} strat={strategy} seed={seed:#x}");
                assert_eq!(dc, unsafe { cdg(cs) }, "C XXH64 digest not idempotent");
                assert_eq!(dr, unsafe { rdg(rs) }, "Rust XXH64 digest not idempotent");
                assert_eq!(dc, unsafe { c1(src.as_ptr(), len, seed) }, "C stream != one-shot");
                assert_eq!(dr, unsafe { r1(src.as_ptr(), len, seed) }, "Rust stream != one-shot");
                if !copy_c.is_null() {
                    unsafe { cf(copy_c) };
                    unsafe { rf(copy_r) };
                }
                assert_eq!(unsafe { cf(cs) }, unsafe { rf(rs) }, "XXH64_freeState");
            }
        }
    }
}

/// Rows 222-223: canonical representation (always big-endian) round-trips.
#[test]
fn g12_canonical() {
    let (cc32, rc32) = syms::<FnCanon32>("LZ4_XXH32_canonicalFromHash");
    let (cf32, rf32) = syms::<FnFromCanon32>("LZ4_XXH32_hashFromCanonical");
    let (cc64, rc64) = syms::<FnCanon64>("LZ4_XXH64_canonicalFromHash");
    let (cf64, rf64) = syms::<FnFromCanon64>("LZ4_XXH64_hashFromCanonical");
    let mut rng = Rng::new(SEED ^ 4);

    let mut h32: Vec<u32> = vec![0, 1, 0xFF, 0x100, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF];
    for _ in 0..500 {
        h32.push(rng.next_u32());
    }
    for h in h32 {
        let mut cb = [0u8; 4];
        let mut rb = [0u8; 4];
        unsafe { cc32(cb.as_mut_ptr(), h) };
        unsafe { rc32(rb.as_mut_ptr(), h) };
        assert_eq!(cb, rb, "XXH32_canonicalFromHash({h:#x})");
        assert_eq!(cb, h.to_be_bytes(), "canonical form is not big-endian");
        let a = unsafe { cf32(cb.as_ptr()) };
        let b = unsafe { rf32(rb.as_ptr()) };
        assert_eq!(a, b, "XXH32_hashFromCanonical");
        assert_eq!(a, h, "XXH32 canonical round-trip");
        // decode arbitrary bytes too
        let raw: [u8; 4] = rng.next_u32().to_le_bytes();
        assert_eq!(unsafe { cf32(raw.as_ptr()) }, unsafe { rf32(raw.as_ptr()) });
    }

    let mut h64: Vec<u64> = vec![0, 1, 0xFF, 0xFFFF_FFFF, 0x1_0000_0000, u64::MAX];
    for _ in 0..500 {
        h64.push(rng.next_u64());
    }
    for h in h64 {
        let mut cb = [0u8; 8];
        let mut rb = [0u8; 8];
        unsafe { cc64(cb.as_mut_ptr(), h) };
        unsafe { rc64(rb.as_mut_ptr(), h) };
        assert_eq!(cb, rb, "XXH64_canonicalFromHash({h:#x})");
        assert_eq!(cb, h.to_be_bytes(), "canonical form is not big-endian");
        let a = unsafe { cf64(cb.as_ptr()) };
        let b = unsafe { rf64(rb.as_ptr()) };
        assert_eq!(a, b, "XXH64_hashFromCanonical");
        assert_eq!(a, h, "XXH64 canonical round-trip");
        let raw: [u8; 8] = rng.next_u64().to_le_bytes();
        assert_eq!(unsafe { cf64(raw.as_ptr()) }, unsafe { rf64(raw.as_ptr()) });
    }
}

/// Row 225: the state structs must have an identical byte layout, so a state
/// produced by one library can be consumed by the other.
#[test]
fn g12_state_layout_interop() {
    let mut rng = Rng::new(SEED ^ 5);

    // XXH32
    {
        let (cc, rc) = syms::<FnCreate>("LZ4_XXH32_createState");
        let (cf, rf) = syms::<FnFree>("LZ4_XXH32_freeState");
        let (cr, rr) = syms::<FnReset32>("LZ4_XXH32_reset");
        let (cu, ru) = syms::<FnUpdate>("LZ4_XXH32_update");
        let (cdg, rdg) = syms::<FnDigest32>("LZ4_XXH32_digest");
        for len in [0usize, 1, 15, 16, 17, 100, 1000] {
            let src = mkdata(Shape::Random, len, &mut rng);
            let cs = unsafe { cc() };
            let rs = unsafe { rc() };
            unsafe { cr(cs, 0x1234_5678) };
            unsafe { rr(rs, 0x1234_5678) };
            if len > 0 {
                unsafe { cu(cs, src.as_ptr(), len) };
                unsafe { ru(rs, src.as_ptr(), len) };
            }
            // 64 bytes covers sizeof(XXH32_state_t) (52)
            let cbytes = unsafe { std::slice::from_raw_parts(cs, 52) }.to_vec();
            let rbytes = unsafe { std::slice::from_raw_parts(rs, 52) }.to_vec();
            assert_eq!(
                cbytes, rbytes,
                "XXH32_state_t bytes differ after len={len}\n C={}\n R={}",
                hexish(&cbytes),
                hexish(&rbytes)
            );
            // cross-library digest
            assert_eq!(unsafe { cdg(rs) }, unsafe { rdg(cs) }, "cross-library XXH32 digest");
            unsafe { cf(cs) };
            unsafe { rf(rs) };
        }
    }
    // XXH64
    {
        let (cc, rc) = syms::<FnCreate>("LZ4_XXH64_createState");
        let (cf, rf) = syms::<FnFree>("LZ4_XXH64_freeState");
        let (cr, rr) = syms::<FnReset64>("LZ4_XXH64_reset");
        let (cu, ru) = syms::<FnUpdate>("LZ4_XXH64_update");
        let (cdg, rdg) = syms::<FnDigest64>("LZ4_XXH64_digest");
        for len in [0usize, 1, 31, 32, 33, 100, 1000] {
            let src = mkdata(Shape::Random, len, &mut rng);
            let cs = unsafe { cc() };
            let rs = unsafe { rc() };
            unsafe { cr(cs, 0xDEAD_BEEF_1234_5678) };
            unsafe { rr(rs, 0xDEAD_BEEF_1234_5678) };
            if len > 0 {
                unsafe { cu(cs, src.as_ptr(), len) };
                unsafe { ru(rs, src.as_ptr(), len) };
            }
            let cbytes = unsafe { std::slice::from_raw_parts(cs, 88) }.to_vec();
            let rbytes = unsafe { std::slice::from_raw_parts(rs, 88) }.to_vec();
            assert_eq!(
                cbytes, rbytes,
                "XXH64_state_t bytes differ after len={len}\n C={}\n R={}",
                hexish(&cbytes),
                hexish(&rbytes)
            );
            assert_eq!(unsafe { cdg(rs) }, unsafe { rdg(cs) }, "cross-library XXH64 digest");
            unsafe { cf(cs) };
            unsafe { rf(rs) };
        }
    }
}
