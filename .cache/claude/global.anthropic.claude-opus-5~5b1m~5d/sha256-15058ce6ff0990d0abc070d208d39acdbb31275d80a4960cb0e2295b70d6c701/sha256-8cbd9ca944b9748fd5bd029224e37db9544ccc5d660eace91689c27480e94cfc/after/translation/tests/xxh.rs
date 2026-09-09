//! Phase B/C differential tests for the namespaced xxHash surface
//! (`LZ4_XXH32*`, `LZ4_XXH64*`).
//!
//! Every call goes through `dlsym` on both shared libraries.
mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

const SEEDS32: &[u32] = &[
    0,
    1,
    0xFFFF_FFFF,
    0x9E37_79B1,
    0x85EB_CA77,
    0xC2B2_AE3D,
    0x2757_D61B,
    0x1656_67B1,
    12345,
];
const SEEDS64: &[u64] = &[
    0,
    1,
    u64::MAX,
    0x9E37_79B1_85EB_CA87,
    0xC2B2_AE3D_27D4_EB4F,
    0x1656_67B1_9E37_79B9,
    0xFFFF_FFFF,
    0x1_0000_0000,
    0xDEAD_BEEF_CAFE_BABE,
];

/// Lengths chosen to straddle every branch of the xxHash implementations:
/// the 16-byte (XXH32 stripe) and 32-byte (XXH64 stripe) main loops, the
/// 4/8-byte tail loops, and the 1-byte tail loop.
const LENS: &[usize] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11, 12, 13, 15, 16, 17, 19, 23, 24, 25, 31, 32, 33, 39, 40, 47,
    48, 63, 64, 65, 95, 96, 127, 128, 129, 200, 255, 256, 257, 1000, 4095, 4096, 4097, 16384,
    65536, 100_000,
];

fn state32_pair() -> (*mut c_void, *mut c_void) {
    let cs = unsafe { c().get::<Fn_XXH_createState>("LZ4_XXH32_createState")() };
    let rs = unsafe { r().get::<Fn_XXH_createState>("LZ4_XXH32_createState")() };
    assert!(!cs.is_null() && !rs.is_null());
    (cs, rs)
}
fn state64_pair() -> (*mut c_void, *mut c_void) {
    let cs = unsafe { c().get::<Fn_XXH_createState>("LZ4_XXH64_createState")() };
    let rs = unsafe { r().get::<Fn_XXH_createState>("LZ4_XXH64_createState")() };
    assert!(!cs.is_null() && !rs.is_null());
    (cs, rs)
}

// ===========================================================================
// CONFIGS rows: XXH32/XXH64 one-shot, all lengths x all seeds x all shapes
// ===========================================================================

#[test]
fn xxh32_oneshot_all_lengths_seeds_shapes() {
    let mut rng = Rng::new(0x5EED_0001);
    for &shape in ALL_SHAPES {
        for &len in LENS {
            let data = gen(&mut rng, len, shape);
            for &seed in SEEDS32 {
                let (cv, rv) = both(|l| unsafe {
                    l.get::<Fn_XXH32>("LZ4_XXH32")(data.as_ptr() as *const c_void, len, seed)
                });
                assert_eq!(
                    cv, rv,
                    "LZ4_XXH32 len={} seed={:#x} shape={:?}",
                    len, seed, shape
                );
            }
        }
    }
}

#[test]
fn xxh64_oneshot_all_lengths_seeds_shapes() {
    let mut rng = Rng::new(0x5EED_0002);
    for &shape in ALL_SHAPES {
        for &len in LENS {
            let data = gen(&mut rng, len, shape);
            for &seed in SEEDS64 {
                let (cv, rv) = both(|l| unsafe {
                    l.get::<Fn_XXH64>("LZ4_XXH64")(data.as_ptr() as *const c_void, len, seed)
                });
                assert_eq!(
                    cv, rv,
                    "LZ4_XXH64 len={} seed={:#x} shape={:?}",
                    len, seed, shape
                );
            }
        }
    }
}

#[test]
fn xxh32_oneshot_random_fuzz() {
    let mut rng = Rng::new(0x5EED_0003);
    for i in 0..4000 {
        let len = rng.below(2000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let data = gen(&mut rng, len, shape);
        let seed = rng.next_u32();
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_XXH32>("LZ4_XXH32")(data.as_ptr() as *const c_void, len, seed)
        });
        assert_eq!(cv, rv, "iter {} len={} seed={:#x}", i, len, seed);
    }
}

#[test]
fn xxh64_oneshot_random_fuzz() {
    let mut rng = Rng::new(0x5EED_0004);
    for i in 0..4000 {
        let len = rng.below(2000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let data = gen(&mut rng, len, shape);
        let seed = rng.next_u64();
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_XXH64>("LZ4_XXH64")(data.as_ptr() as *const c_void, len, seed)
        });
        assert_eq!(cv, rv, "iter {} len={} seed={:#x}", i, len, seed);
    }
}

// ===========================================================================
// streaming: reset / update x N chunks / digest
// ===========================================================================

#[test]
fn xxh32_streaming_matches_oneshot_and_c() {
    let mut rng = Rng::new(0x5EED_0005);
    let (cs, rs) = state32_pair();
    for iter in 0..1500 {
        let total = rng.below(3000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let data = gen(&mut rng, total, shape);
        let seed = if iter % 3 == 0 {
            0
        } else {
            SEEDS32[rng.below(SEEDS32.len())]
        };

        let cr = unsafe { c().get::<Fn_XXH32_reset>("LZ4_XXH32_reset")(cs, seed) };
        let rr = unsafe { r().get::<Fn_XXH32_reset>("LZ4_XXH32_reset")(rs, seed) };
        assert_eq!(cr, rr, "reset");

        // split into 1..=6 random chunks (including possible empty chunks)
        let nchunks = rng.range(1, 6);
        let mut bounds: Vec<usize> = (0..nchunks - 1).map(|_| rng.below(total + 1)).collect();
        bounds.push(0);
        bounds.push(total);
        bounds.sort_unstable();
        for w in bounds.windows(2) {
            let (a, b) = (w[0], w[1]);
            let cu = unsafe {
                c().get::<Fn_XXH_update>("LZ4_XXH32_update")(
                    cs,
                    data[a..b].as_ptr() as *const c_void,
                    b - a,
                )
            };
            let ru = unsafe {
                r().get::<Fn_XXH_update>("LZ4_XXH32_update")(
                    rs,
                    data[a..b].as_ptr() as *const c_void,
                    b - a,
                )
            };
            assert_eq!(cu, ru, "update({}..{})", a, b);
        }
        let cd = unsafe { c().get::<Fn_XXH32_digest>("LZ4_XXH32_digest")(cs) };
        let rd = unsafe { r().get::<Fn_XXH32_digest>("LZ4_XXH32_digest")(rs) };
        assert_eq!(cd, rd, "digest iter={} total={} seed={:#x}", iter, total, seed);

        // and it must agree with the one-shot value
        let oneshot = unsafe {
            c().get::<Fn_XXH32>("LZ4_XXH32")(data.as_ptr() as *const c_void, total, seed)
        };
        assert_eq!(cd, oneshot, "streaming vs one-shot (C), iter={}", iter);
    }
    unsafe {
        c().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(cs);
        r().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(rs);
    }
}

#[test]
fn xxh64_streaming_matches_oneshot_and_c() {
    let mut rng = Rng::new(0x5EED_0006);
    let (cs, rs) = state64_pair();
    for iter in 0..1500 {
        let total = rng.below(3000);
        let shape = ALL_SHAPES[rng.below(ALL_SHAPES.len())];
        let data = gen(&mut rng, total, shape);
        let seed = if iter % 3 == 0 {
            0
        } else {
            SEEDS64[rng.below(SEEDS64.len())]
        };

        let cr = unsafe { c().get::<Fn_XXH64_reset>("LZ4_XXH64_reset")(cs, seed) };
        let rr = unsafe { r().get::<Fn_XXH64_reset>("LZ4_XXH64_reset")(rs, seed) };
        assert_eq!(cr, rr, "reset");

        let nchunks = rng.range(1, 6);
        let mut bounds: Vec<usize> = (0..nchunks - 1).map(|_| rng.below(total + 1)).collect();
        bounds.push(0);
        bounds.push(total);
        bounds.sort_unstable();
        for w in bounds.windows(2) {
            let (a, b) = (w[0], w[1]);
            let cu = unsafe {
                c().get::<Fn_XXH_update>("LZ4_XXH64_update")(
                    cs,
                    data[a..b].as_ptr() as *const c_void,
                    b - a,
                )
            };
            let ru = unsafe {
                r().get::<Fn_XXH_update>("LZ4_XXH64_update")(
                    rs,
                    data[a..b].as_ptr() as *const c_void,
                    b - a,
                )
            };
            assert_eq!(cu, ru, "update({}..{})", a, b);
        }
        let cd = unsafe { c().get::<Fn_XXH64_digest>("LZ4_XXH64_digest")(cs) };
        let rd = unsafe { r().get::<Fn_XXH64_digest>("LZ4_XXH64_digest")(rs) };
        assert_eq!(cd, rd, "digest iter={} total={} seed={:#x}", iter, total, seed);

        let oneshot = unsafe {
            c().get::<Fn_XXH64>("LZ4_XXH64")(data.as_ptr() as *const c_void, total, seed)
        };
        assert_eq!(cd, oneshot, "streaming vs one-shot (C), iter={}", iter);
    }
    unsafe {
        c().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(cs);
        r().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(rs);
    }
}

// ===========================================================================
// copyState: the state must be byte-identical between C and Rust, since it is
// a public (documented-size) structure that callers may serialise.
// ===========================================================================

#[test]
fn xxh32_copy_state_and_resume() {
    let mut rng = Rng::new(0x5EED_0007);
    let (cs, rs) = state32_pair();
    let cs2 = unsafe { c().get::<Fn_XXH_createState>("LZ4_XXH32_createState")() };
    let rs2 = unsafe { r().get::<Fn_XXH_createState>("LZ4_XXH32_createState")() };

    for iter in 0..600 {
        let seed = SEEDS32[rng.below(SEEDS32.len())];
        unsafe {
            c().get::<Fn_XXH32_reset>("LZ4_XXH32_reset")(cs, seed);
            r().get::<Fn_XXH32_reset>("LZ4_XXH32_reset")(rs, seed);
        }
        let a = { let n = rng.below(500); gen(&mut rng, n, Shape::Text) };
        let b = { let n = rng.below(500); gen(&mut rng, n, Shape::Incompressible) };
        unsafe {
            c().get::<Fn_XXH_update>("LZ4_XXH32_update")(cs, a.as_ptr() as *const c_void, a.len());
            r().get::<Fn_XXH_update>("LZ4_XXH32_update")(rs, a.as_ptr() as *const c_void, a.len());
            c().get::<Fn_XXH_copyState>("LZ4_XXH32_copyState")(cs2, cs);
            r().get::<Fn_XXH_copyState>("LZ4_XXH32_copyState")(rs2, rs);
            // resume both the original and the copy
            c().get::<Fn_XXH_update>("LZ4_XXH32_update")(cs2, b.as_ptr() as *const c_void, b.len());
            r().get::<Fn_XXH_update>("LZ4_XXH32_update")(rs2, b.as_ptr() as *const c_void, b.len());
        }
        let cd = unsafe { c().get::<Fn_XXH32_digest>("LZ4_XXH32_digest")(cs2) };
        let rd = unsafe { r().get::<Fn_XXH32_digest>("LZ4_XXH32_digest")(rs2) };
        assert_eq!(cd, rd, "copyState digest iter={}", iter);

        let mut cat = a.clone();
        cat.extend_from_slice(&b);
        let expect = unsafe {
            c().get::<Fn_XXH32>("LZ4_XXH32")(cat.as_ptr() as *const c_void, cat.len(), seed)
        };
        assert_eq!(cd, expect, "copyState resume iter={}", iter);
    }
    unsafe {
        c().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(cs);
        c().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(cs2);
        r().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(rs);
        r().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(rs2);
    }
}

#[test]
fn xxh64_copy_state_and_resume() {
    let mut rng = Rng::new(0x5EED_0008);
    let (cs, rs) = state64_pair();
    let cs2 = unsafe { c().get::<Fn_XXH_createState>("LZ4_XXH64_createState")() };
    let rs2 = unsafe { r().get::<Fn_XXH_createState>("LZ4_XXH64_createState")() };

    for iter in 0..600 {
        let seed = SEEDS64[rng.below(SEEDS64.len())];
        unsafe {
            c().get::<Fn_XXH64_reset>("LZ4_XXH64_reset")(cs, seed);
            r().get::<Fn_XXH64_reset>("LZ4_XXH64_reset")(rs, seed);
        }
        let a = { let n = rng.below(500); gen(&mut rng, n, Shape::Text) };
        let b = { let n = rng.below(500); gen(&mut rng, n, Shape::Incompressible) };
        unsafe {
            c().get::<Fn_XXH_update>("LZ4_XXH64_update")(cs, a.as_ptr() as *const c_void, a.len());
            r().get::<Fn_XXH_update>("LZ4_XXH64_update")(rs, a.as_ptr() as *const c_void, a.len());
            c().get::<Fn_XXH_copyState>("LZ4_XXH64_copyState")(cs2, cs);
            r().get::<Fn_XXH_copyState>("LZ4_XXH64_copyState")(rs2, rs);
            c().get::<Fn_XXH_update>("LZ4_XXH64_update")(cs2, b.as_ptr() as *const c_void, b.len());
            r().get::<Fn_XXH_update>("LZ4_XXH64_update")(rs2, b.as_ptr() as *const c_void, b.len());
        }
        let cd = unsafe { c().get::<Fn_XXH64_digest>("LZ4_XXH64_digest")(cs2) };
        let rd = unsafe { r().get::<Fn_XXH64_digest>("LZ4_XXH64_digest")(rs2) };
        assert_eq!(cd, rd, "copyState digest iter={}", iter);

        let mut cat = a.clone();
        cat.extend_from_slice(&b);
        let expect = unsafe {
            c().get::<Fn_XXH64>("LZ4_XXH64")(cat.as_ptr() as *const c_void, cat.len(), seed)
        };
        assert_eq!(cd, expect, "copyState resume iter={}", iter);
    }
    unsafe {
        c().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(cs);
        c().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(cs2);
        r().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(rs);
        r().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(rs2);
    }
}

/// The `XXH*_state_t` layout is public (`XXH_STATIC_LINKING_ONLY`), so the
/// bytes a caller can observe must be identical.  Compare the raw state after
/// reset + update between the two libraries.
#[test]
fn xxh_state_bytes_identical() {
    // Sizes from xxhash.h: XXH32_state_s = 12 x U32 (48 bytes),
    // XXH64_state_s = 11 fields, 88 bytes.  Compare a conservative prefix.
    let mut rng = Rng::new(0x5EED_0009);
    for &(create, reset32, update, size) in &[
        ("LZ4_XXH32_createState", "LZ4_XXH32_reset", "LZ4_XXH32_update", 48usize),
    ] {
        let cs = unsafe { c().get::<Fn_XXH_createState>(create)() };
        let rs = unsafe { r().get::<Fn_XXH_createState>(create)() };
        for &seed in SEEDS32 {
            unsafe {
                c().get::<Fn_XXH32_reset>(reset32)(cs, seed);
                r().get::<Fn_XXH32_reset>(reset32)(rs, seed);
            }
            let d = { let n = rng.below(400); gen(&mut rng, n, Shape::Text) };
            unsafe {
                c().get::<Fn_XXH_update>(update)(cs, d.as_ptr() as *const c_void, d.len());
                r().get::<Fn_XXH_update>(update)(rs, d.as_ptr() as *const c_void, d.len());
            }
            let cb = unsafe { std::slice::from_raw_parts(cs as *const u8, size) };
            let rb = unsafe { std::slice::from_raw_parts(rs as *const u8, size) };
            assert_bytes_eq!(format!("XXH32 state bytes seed={:#x}", seed), cb, rb);
        }
        unsafe {
            c().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(cs);
            r().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(rs);
        }
    }

    let cs = unsafe { c().get::<Fn_XXH_createState>("LZ4_XXH64_createState")() };
    let rs = unsafe { r().get::<Fn_XXH_createState>("LZ4_XXH64_createState")() };
    for &seed in SEEDS64 {
        unsafe {
            c().get::<Fn_XXH64_reset>("LZ4_XXH64_reset")(cs, seed);
            r().get::<Fn_XXH64_reset>("LZ4_XXH64_reset")(rs, seed);
        }
        let d = { let n = rng.below(400); gen(&mut rng, n, Shape::Runs) };
        unsafe {
            c().get::<Fn_XXH_update>("LZ4_XXH64_update")(cs, d.as_ptr() as *const c_void, d.len());
            r().get::<Fn_XXH_update>("LZ4_XXH64_update")(rs, d.as_ptr() as *const c_void, d.len());
        }
        let cb = unsafe { std::slice::from_raw_parts(cs as *const u8, 88) };
        let rb = unsafe { std::slice::from_raw_parts(rs as *const u8, 88) };
        assert_bytes_eq!(format!("XXH64 state bytes seed={:#x}", seed), cb, rb);
    }
    unsafe {
        c().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(cs);
        r().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(rs);
    }
}

// ===========================================================================
// canonical representation
// ===========================================================================

#[test]
fn xxh_canonical_round_trip() {
    let mut rng = Rng::new(0x5EED_000A);
    for _ in 0..3000 {
        let h32 = rng.next_u32();
        let mut cbuf = [0u8; 4];
        let mut rbuf = [0u8; 4];
        unsafe {
            c().get::<Fn_XXH32_canonicalFromHash>("LZ4_XXH32_canonicalFromHash")(
                cbuf.as_mut_ptr() as *mut c_void,
                h32,
            );
            r().get::<Fn_XXH32_canonicalFromHash>("LZ4_XXH32_canonicalFromHash")(
                rbuf.as_mut_ptr() as *mut c_void,
                h32,
            );
        }
        assert_bytes_eq!(format!("XXH32 canonical of {:#x}", h32), cbuf, rbuf);
        // canonical form is big-endian
        assert_eq!(cbuf, h32.to_be_bytes());

        let raw: [u8; 4] = rng.next_u32().to_le_bytes();
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_XXH32_hashFromCanonical>("LZ4_XXH32_hashFromCanonical")(
                raw.as_ptr() as *const c_void,
            )
        });
        assert_eq!(cv, rv, "XXH32_hashFromCanonical {:02x?}", raw);

        let h64 = rng.next_u64();
        let mut cbuf = [0u8; 8];
        let mut rbuf = [0u8; 8];
        unsafe {
            c().get::<Fn_XXH64_canonicalFromHash>("LZ4_XXH64_canonicalFromHash")(
                cbuf.as_mut_ptr() as *mut c_void,
                h64,
            );
            r().get::<Fn_XXH64_canonicalFromHash>("LZ4_XXH64_canonicalFromHash")(
                rbuf.as_mut_ptr() as *mut c_void,
                h64,
            );
        }
        assert_bytes_eq!(format!("XXH64 canonical of {:#x}", h64), cbuf, rbuf);
        assert_eq!(cbuf, h64.to_be_bytes());

        let raw: [u8; 8] = rng.next_u64().to_le_bytes();
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_XXH64_hashFromCanonical>("LZ4_XXH64_hashFromCanonical")(
                raw.as_ptr() as *const c_void,
            )
        });
        assert_eq!(cv, rv, "XXH64_hashFromCanonical {:02x?}", raw);
    }
}

// ===========================================================================
// ERRORS.md rows 138-141
// ===========================================================================

/// ERRORS.md row 138 / 139: `XXH*_update(state, NULL, len)` must return
/// `XXH_ERROR` (1) identically in both libraries.
#[test]
fn err_xxh_update_null_input() {
    let (cs, rs) = state32_pair();
    unsafe {
        c().get::<Fn_XXH32_reset>("LZ4_XXH32_reset")(cs, 0);
        r().get::<Fn_XXH32_reset>("LZ4_XXH32_reset")(rs, 0);
    }
    for &len in &[0usize, 1, 7, 16, 100] {
        let cv = unsafe {
            c().get::<Fn_XXH_update>("LZ4_XXH32_update")(cs, std::ptr::null(), len)
        };
        let rv = unsafe {
            r().get::<Fn_XXH_update>("LZ4_XXH32_update")(rs, std::ptr::null(), len)
        };
        assert_eq!(cv, rv, "XXH32_update(NULL, {})", len);
    }
    unsafe {
        c().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(cs);
        r().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(rs);
    }

    let (cs, rs) = state64_pair();
    unsafe {
        c().get::<Fn_XXH64_reset>("LZ4_XXH64_reset")(cs, 0);
        r().get::<Fn_XXH64_reset>("LZ4_XXH64_reset")(rs, 0);
    }
    for &len in &[0usize, 1, 7, 32, 100] {
        let cv = unsafe {
            c().get::<Fn_XXH_update>("LZ4_XXH64_update")(cs, std::ptr::null(), len)
        };
        let rv = unsafe {
            r().get::<Fn_XXH_update>("LZ4_XXH64_update")(rs, std::ptr::null(), len)
        };
        assert_eq!(cv, rv, "XXH64_update(NULL, {})", len);
    }
    unsafe {
        c().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(cs);
        r().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(rs);
    }
}

/// ERRORS.md row 140: `createState` never returns NULL under normal conditions.
#[test]
fn err_xxh_create_state_non_null() {
    for name in ["LZ4_XXH32_createState", "LZ4_XXH64_createState"] {
        let (cp, rp) = both(|l| unsafe { l.get::<Fn_XXH_createState>(name)() });
        assert!(!cp.is_null(), "C {} returned NULL", name);
        assert!(!rp.is_null(), "Rust {} returned NULL", name);
        let free = if name.contains("32") {
            "LZ4_XXH32_freeState"
        } else {
            "LZ4_XXH64_freeState"
        };
        unsafe {
            c().get::<Fn_XXH_freeState>(free)(cp);
            r().get::<Fn_XXH_freeState>(free)(rp);
        }
    }
}

/// ERRORS.md row 141: `freeState(NULL)` must return `XXH_OK` (0) in both.
#[test]
fn err_xxh_free_state_null() {
    for name in ["LZ4_XXH32_freeState", "LZ4_XXH64_freeState"] {
        let (cv, rv): (c_int, c_int) =
            both(|l| unsafe { l.get::<Fn_XXH_freeState>(name)(std::ptr::null_mut()) });
        assert_eq!(cv, rv, "{}(NULL)", name);
    }
}

/// `XXH32/XXH64(NULL, 0, seed)` — the one-shot form with a NULL pointer and a
/// zero length.  The C code dereferences nothing here, so it must produce the
/// well-defined empty-input hash.
#[test]
fn xxh_oneshot_null_zero_len() {
    for &seed in SEEDS32 {
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_XXH32>("LZ4_XXH32")(std::ptr::null(), 0, seed)
        });
        assert_eq!(cv, rv, "XXH32(NULL,0,{:#x})", seed);
    }
    for &seed in SEEDS64 {
        let (cv, rv) = both(|l| unsafe {
            l.get::<Fn_XXH64>("LZ4_XXH64")(std::ptr::null(), 0, seed)
        });
        assert_eq!(cv, rv, "XXH64(NULL,0,{:#x})", seed);
    }
}

/// `digest` without any `update` (freshly reset state), and `digest` called
/// twice (must be idempotent and identical between the libraries).
#[test]
fn xxh_digest_empty_and_idempotent() {
    let (cs, rs) = state32_pair();
    for &seed in SEEDS32 {
        unsafe {
            c().get::<Fn_XXH32_reset>("LZ4_XXH32_reset")(cs, seed);
            r().get::<Fn_XXH32_reset>("LZ4_XXH32_reset")(rs, seed);
        }
        let c1 = unsafe { c().get::<Fn_XXH32_digest>("LZ4_XXH32_digest")(cs) };
        let c2 = unsafe { c().get::<Fn_XXH32_digest>("LZ4_XXH32_digest")(cs) };
        let r1 = unsafe { r().get::<Fn_XXH32_digest>("LZ4_XXH32_digest")(rs) };
        let r2 = unsafe { r().get::<Fn_XXH32_digest>("LZ4_XXH32_digest")(rs) };
        assert_eq!((c1, c2), (r1, r2), "XXH32 empty digest seed={:#x}", seed);
    }
    unsafe {
        c().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(cs);
        r().get::<Fn_XXH_freeState>("LZ4_XXH32_freeState")(rs);
    }

    let (cs, rs) = state64_pair();
    for &seed in SEEDS64 {
        unsafe {
            c().get::<Fn_XXH64_reset>("LZ4_XXH64_reset")(cs, seed);
            r().get::<Fn_XXH64_reset>("LZ4_XXH64_reset")(rs, seed);
        }
        let c1 = unsafe { c().get::<Fn_XXH64_digest>("LZ4_XXH64_digest")(cs) };
        let c2 = unsafe { c().get::<Fn_XXH64_digest>("LZ4_XXH64_digest")(cs) };
        let r1 = unsafe { r().get::<Fn_XXH64_digest>("LZ4_XXH64_digest")(rs) };
        let r2 = unsafe { r().get::<Fn_XXH64_digest>("LZ4_XXH64_digest")(rs) };
        assert_eq!((c1, c2), (r1, r2), "XXH64 empty digest seed={:#x}", seed);
    }
    unsafe {
        c().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(cs);
        r().get::<Fn_XXH_freeState>("LZ4_XXH64_freeState")(rs);
    }
}

/// Unaligned input pointers: xxHash has separate aligned / unaligned read
/// paths (`XXH_FORCE_ALIGN_CHECK`), so every start offset 0..8 must be probed.
#[test]
fn xxh_unaligned_inputs() {
    let mut rng = Rng::new(0x5EED_000B);
    let buf = gen(&mut rng, 5000, Shape::Incompressible);
    for off in 0..16usize {
        for &len in &[0usize, 1, 4, 15, 16, 17, 31, 32, 33, 64, 200, 1000, 4000] {
            if off + len > buf.len() {
                continue;
            }
            let p = unsafe { buf.as_ptr().add(off) } as *const c_void;
            let (cv, rv) = both(|l| unsafe { l.get::<Fn_XXH32>("LZ4_XXH32")(p, len, 0x1234) });
            assert_eq!(cv, rv, "XXH32 off={} len={}", off, len);
            let (cv, rv) = both(|l| unsafe { l.get::<Fn_XXH64>("LZ4_XXH64")(p, len, 0x1234) });
            assert_eq!(cv, rv, "XXH64 off={} len={}", off, len);
        }
    }
}
