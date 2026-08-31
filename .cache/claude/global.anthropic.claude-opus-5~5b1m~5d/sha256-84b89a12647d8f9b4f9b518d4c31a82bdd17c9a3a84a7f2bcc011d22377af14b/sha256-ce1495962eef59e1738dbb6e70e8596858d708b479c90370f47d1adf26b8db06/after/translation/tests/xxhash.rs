//! Differential tests for the namespaced xxHash surface (`LZ4_XXH*`).
//!
//! Covers CONFIGS.md rows 163..173 ("xxhash (namespaced LZ4_XXH*)") and the
//! `## xxhash.c` rows of ERRORS.md (185..194).
//!
//! Every call goes through a `.so` export via libloading; opaque streaming
//! states are always created, copied and freed by the *same* library.
#![allow(unused_imports, non_snake_case)]

mod common;
use common::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// FFI signatures
// ---------------------------------------------------------------------------

type FnXxh32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> u32;
type FnXxh64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
type FnCreateState = unsafe extern "C" fn() -> *mut c_void;
type FnFreeState = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnCopyState = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnReset32 = unsafe extern "C" fn(*mut c_void, c_uint) -> c_int;
type FnReset64 = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type FnUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
type FnDigest32 = unsafe extern "C" fn(*const c_void) -> u32;
type FnDigest64 = unsafe extern "C" fn(*const c_void) -> u64;
type FnCanon32 = unsafe extern "C" fn(*mut c_void, u32);
type FnFromCanon32 = unsafe extern "C" fn(*const c_void) -> u32;
type FnCanon64 = unsafe extern "C" fn(*mut c_void, u64);
type FnFromCanon64 = unsafe extern "C" fn(*const c_void) -> u64;
type FnVersion = unsafe extern "C" fn() -> c_uint;

const XXH_OK: c_int = 0;
const XXH_ERROR: c_int = 1;

macro_rules! pair {
    ($l:expr, $t:ty, $n:expr) => {{
        let (a, b) = $l.sym::<$t>($n);
        (*a, *b)
    }};
}

#[derive(Copy, Clone)]
struct Api32 {
    xxh: FnXxh32,
    create: FnCreateState,
    free: FnFreeState,
    copy: FnCopyState,
    reset: FnReset32,
    update: FnUpdate,
    digest: FnDigest32,
    canon: FnCanon32,
    from_canon: FnFromCanon32,
}

#[derive(Copy, Clone)]
struct Api64 {
    xxh: FnXxh64,
    create: FnCreateState,
    free: FnFreeState,
    copy: FnCopyState,
    reset: FnReset64,
    update: FnUpdate,
    digest: FnDigest64,
    canon: FnCanon64,
    from_canon: FnFromCanon64,
}

unsafe fn api32() -> (Api32, Api32) {
    let l = libs();
    let (xxh_c, xxh_r) = pair!(l, FnXxh32, "LZ4_XXH32");
    let (cre_c, cre_r) = pair!(l, FnCreateState, "LZ4_XXH32_createState");
    let (fre_c, fre_r) = pair!(l, FnFreeState, "LZ4_XXH32_freeState");
    let (cpy_c, cpy_r) = pair!(l, FnCopyState, "LZ4_XXH32_copyState");
    let (rst_c, rst_r) = pair!(l, FnReset32, "LZ4_XXH32_reset");
    let (upd_c, upd_r) = pair!(l, FnUpdate, "LZ4_XXH32_update");
    let (dig_c, dig_r) = pair!(l, FnDigest32, "LZ4_XXH32_digest");
    let (can_c, can_r) = pair!(l, FnCanon32, "LZ4_XXH32_canonicalFromHash");
    let (fca_c, fca_r) = pair!(l, FnFromCanon32, "LZ4_XXH32_hashFromCanonical");
    (
        Api32 {
            xxh: xxh_c,
            create: cre_c,
            free: fre_c,
            copy: cpy_c,
            reset: rst_c,
            update: upd_c,
            digest: dig_c,
            canon: can_c,
            from_canon: fca_c,
        },
        Api32 {
            xxh: xxh_r,
            create: cre_r,
            free: fre_r,
            copy: cpy_r,
            reset: rst_r,
            update: upd_r,
            digest: dig_r,
            canon: can_r,
            from_canon: fca_r,
        },
    )
}

unsafe fn api64() -> (Api64, Api64) {
    let l = libs();
    let (xxh_c, xxh_r) = pair!(l, FnXxh64, "LZ4_XXH64");
    let (cre_c, cre_r) = pair!(l, FnCreateState, "LZ4_XXH64_createState");
    let (fre_c, fre_r) = pair!(l, FnFreeState, "LZ4_XXH64_freeState");
    let (cpy_c, cpy_r) = pair!(l, FnCopyState, "LZ4_XXH64_copyState");
    let (rst_c, rst_r) = pair!(l, FnReset64, "LZ4_XXH64_reset");
    let (upd_c, upd_r) = pair!(l, FnUpdate, "LZ4_XXH64_update");
    let (dig_c, dig_r) = pair!(l, FnDigest64, "LZ4_XXH64_digest");
    let (can_c, can_r) = pair!(l, FnCanon64, "LZ4_XXH64_canonicalFromHash");
    let (fca_c, fca_r) = pair!(l, FnFromCanon64, "LZ4_XXH64_hashFromCanonical");
    (
        Api64 {
            xxh: xxh_c,
            create: cre_c,
            free: fre_c,
            copy: cpy_c,
            reset: rst_c,
            update: upd_c,
            digest: dig_c,
            canon: can_c,
            from_canon: fca_c,
        },
        Api64 {
            xxh: xxh_r,
            create: cre_r,
            free: fre_r,
            copy: cpy_r,
            reset: rst_r,
            update: upd_r,
            digest: dig_r,
            canon: can_r,
            from_canon: fca_r,
        },
    )
}

// ---------------------------------------------------------------------------
// One-shot comparison helpers
// ---------------------------------------------------------------------------

#[track_caller]
unsafe fn same32(c: &Api32, r: &Api32, buf: &[u8], seed: u32, ctx: &str) -> u32 {
    let p = buf.as_ptr() as *const c_void;
    let hc = (c.xxh)(p, buf.len(), seed);
    let hr = (r.xxh)(p, buf.len(), seed);
    assert_eq!(
        hc, hr,
        "LZ4_XXH32 mismatch [{ctx}] len={} seed={:#010x} ptr_align={} : C={:#010x} Rust={:#010x}\n  input: {}",
        buf.len(),
        seed,
        (buf.as_ptr() as usize) & 7,
        hc,
        hr,
        hexdump(buf)
    );
    hc
}

#[track_caller]
unsafe fn same64(c: &Api64, r: &Api64, buf: &[u8], seed: u64, ctx: &str) -> u64 {
    let p = buf.as_ptr() as *const c_void;
    let hc = (c.xxh)(p, buf.len(), seed);
    let hr = (r.xxh)(p, buf.len(), seed);
    assert_eq!(
        hc, hr,
        "LZ4_XXH64 mismatch [{ctx}] len={} seed={:#018x} ptr_align={} : C={:#018x} Rust={:#018x}\n  input: {}",
        buf.len(),
        seed,
        (buf.as_ptr() as usize) & 7,
        hc,
        hr,
        hexdump(buf)
    );
    hc
}

fn seeds32() -> Vec<u32> {
    let mut rng = Rng::new(0xA5A5_1234);
    let mut v = vec![0u32, 1, 0x9E37_79B1, 0xFFFF_FFFF, 2654435761];
    for _ in 0..3 {
        v.push(rng.next_u32());
    }
    v
}

fn seeds64() -> Vec<u64> {
    let mut rng = Rng::new(0x5A5A_4321);
    let mut v = vec![
        0u64,
        1,
        0x9E37_79B1,
        0x9E37_79B1_85EB_CA87,
        u64::MAX,
        0xFFFF_FFFF,
    ];
    for _ in 0..3 {
        v.push(rng.next_u64());
    }
    v
}

// ---------------------------------------------------------------------------
// Streaming comparison helpers
// ---------------------------------------------------------------------------

/// Feed `data` to a fresh state in each library using the chunk sizes in
/// `chunks` (the remainder, if any, goes in one last update), asserting that
/// the two libraries agree on every `update` return code and on the digest
/// after every chunk. Also cross-checks the digest against each library's own
/// one-shot of the consumed prefix.
#[track_caller]
unsafe fn stream32(c: &Api32, r: &Api32, data: &[u8], seed: u32, chunks: &[usize], ctx: &str) {
    let sc = (c.create)();
    let sr = (r.create)();
    assert!(!sc.is_null(), "{ctx}: C createState returned NULL");
    assert!(!sr.is_null(), "{ctx}: Rust createState returned NULL");

    let rc = (c.reset)(sc, seed);
    let rr = (r.reset)(sr, seed);
    assert_eq!(rc, rr, "{ctx}: XXH32_reset return mismatch");
    assert_eq!(rc, XXH_OK, "{ctx}: XXH32_reset should be XXH_OK");

    // digest of an empty stream
    let dc = (c.digest)(sc);
    let dr = (r.digest)(sr);
    assert_eq!(dc, dr, "{ctx}: digest of empty stream mismatch (C={dc:#010x} Rust={dr:#010x})");

    let mut off = 0usize;
    let mut step = 0usize;
    let mut it = chunks.iter().copied();
    loop {
        let want = match it.next() {
            Some(n) => n,
            None => {
                if off == data.len() {
                    break;
                }
                data.len() - off
            }
        };
        let n = want.min(data.len() - off);
        let p = data[off..].as_ptr() as *const c_void;
        let uc = (c.update)(sc, p, n);
        let ur = (r.update)(sr, p, n);
        assert_eq!(
            uc, ur,
            "{ctx}: XXH32_update return mismatch at step {step} (off={off} n={n})"
        );
        off += n;
        step += 1;

        let dc = (c.digest)(sc);
        let dr = (r.digest)(sr);
        assert_eq!(
            dc, dr,
            "{ctx}: XXH32_digest mismatch after step {step} (consumed={off} seed={seed:#010x}) C={dc:#010x} Rust={dr:#010x}"
        );
        let oc = same32(c, r, &data[..off], seed, ctx);
        assert_eq!(
            dc, oc,
            "{ctx}: streamed digest != one-shot at consumed={off} (stream={dc:#010x} oneshot={oc:#010x})"
        );
    }

    assert_eq!((c.free)(sc), (r.free)(sr), "{ctx}: freeState return mismatch");
}

#[track_caller]
unsafe fn stream64(c: &Api64, r: &Api64, data: &[u8], seed: u64, chunks: &[usize], ctx: &str) {
    let sc = (c.create)();
    let sr = (r.create)();
    assert!(!sc.is_null(), "{ctx}: C createState returned NULL");
    assert!(!sr.is_null(), "{ctx}: Rust createState returned NULL");

    let rc = (c.reset)(sc, seed);
    let rr = (r.reset)(sr, seed);
    assert_eq!(rc, rr, "{ctx}: XXH64_reset return mismatch");
    assert_eq!(rc, XXH_OK, "{ctx}: XXH64_reset should be XXH_OK");

    let dc = (c.digest)(sc);
    let dr = (r.digest)(sr);
    assert_eq!(dc, dr, "{ctx}: digest of empty stream mismatch (C={dc:#018x} Rust={dr:#018x})");

    let mut off = 0usize;
    let mut step = 0usize;
    let mut it = chunks.iter().copied();
    loop {
        let want = match it.next() {
            Some(n) => n,
            None => {
                if off == data.len() {
                    break;
                }
                data.len() - off
            }
        };
        let n = want.min(data.len() - off);
        let p = data[off..].as_ptr() as *const c_void;
        let uc = (c.update)(sc, p, n);
        let ur = (r.update)(sr, p, n);
        assert_eq!(
            uc, ur,
            "{ctx}: XXH64_update return mismatch at step {step} (off={off} n={n})"
        );
        off += n;
        step += 1;

        let dc = (c.digest)(sc);
        let dr = (r.digest)(sr);
        assert_eq!(
            dc, dr,
            "{ctx}: XXH64_digest mismatch after step {step} (consumed={off} seed={seed:#018x}) C={dc:#018x} Rust={dr:#018x}"
        );
        let oc = same64(c, r, &data[..off], seed, ctx);
        assert_eq!(
            dc, oc,
            "{ctx}: streamed digest != one-shot at consumed={off} (stream={dc:#018x} oneshot={oc:#018x})"
        );
    }

    assert_eq!((c.free)(sc), (r.free)(sr), "{ctx}: freeState return mismatch");
}

/// A battery of chunk-size sequences for `len` bytes: 1-byte-at-a-time, tiny
/// sub-stripe chunks, chunks that straddle the stripe boundary, and random
/// splits.
fn chunk_patterns(rng: &mut Rng, len: usize, stripe: usize) -> Vec<Vec<usize>> {
    let mut out: Vec<Vec<usize>> = Vec::new();

    // whole input in one shot
    out.push(vec![len]);
    // one byte at a time
    out.push(vec![1; len]);
    // fixed sizes, several of them below / at / above the stripe size
    for k in [1usize, 2, 3, 4, 5, 7, 8, stripe - 1, stripe, stripe + 1, 2 * stripe - 1, 2 * stripe + 3] {
        if k == 0 {
            continue;
        }
        let mut v = Vec::new();
        let mut left = len;
        while left > 0 {
            let n = k.min(left);
            v.push(n);
            left -= n;
        }
        if v.is_empty() {
            v.push(0);
        }
        out.push(v);
    }
    // a zero-length update interleaved with real data
    out.push(vec![0, 1, 0, stripe - 1, 0, len]);
    // deliberately land exactly on the stripe boundary, then straddle it
    out.push(vec![stripe, 1, stripe - 1, stripe + 1, len]);
    out.push(vec![stripe - 1, 2, len]);
    out.push(vec![1, stripe - 1, len]);
    // random splits
    for _ in 0..6 {
        let mut v = Vec::new();
        let mut left = len;
        while left > 0 {
            let n = rng.range(0, (2 * stripe + 5).min(left)).max(if left > 0 { 0 } else { 0 });
            let n = if n == 0 { 1 } else { n };
            let n = n.min(left);
            v.push(n);
            left -= n;
        }
        out.push(v);
    }
    out
}

// ===========================================================================
// Row 163 — LZ4_XXH32, length 0 with a valid pointer and with NULL
// ===========================================================================

#[test]
fn row_163_xxh32_len0_valid_pointer_and_null() {
    unsafe {
        let (c, r) = api32();
        let buf = [0u8; 64];
        for &seed in &[0u32, 0x9E37_79B1, 1, 0xFFFF_FFFF] {
            // valid pointer, len 0
            let h = same32(&c, &r, &buf[..0], seed, "row163 valid ptr len0");
            // an offset pointer, still len 0
            let p = buf.as_ptr().add(7) as *const c_void;
            let hc = (c.xxh)(p, 0, seed);
            let hr = (r.xxh)(p, 0, seed);
            assert_eq!(hc, hr, "row163: offset ptr len0 seed={seed:#010x}");
            assert_eq!(h, hc, "row163: len0 must not depend on the pointer");

            // NULL pointer, len 0 (XXH_ACCEPT_NULL_INPUT_POINTER == 0, but no
            // dereference happens for len == 0)
            let hc = (c.xxh)(std::ptr::null(), 0, seed);
            let hr = (r.xxh)(std::ptr::null(), 0, seed);
            assert_eq!(
                hc, hr,
                "row163: LZ4_XXH32(NULL,0,{seed:#010x}) C={hc:#010x} Rust={hr:#010x}"
            );
            assert_eq!(h, hc, "row163: NULL/len0 must equal valid-ptr/len0");
        }
    }
}

// ===========================================================================
// Row 164 — LZ4_XXH32 length 1..7
// ===========================================================================

#[test]
fn row_164_xxh32_len_1_to_7() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(164);
        for &shape in &ALL_SHAPES {
            for len in 1usize..=7 {
                let data = gen(&mut rng, shape, len);
                for &seed in &seeds32() {
                    same32(&c, &r, &data, seed, &format!("row164 {shape:?}"));
                }
            }
        }
        // every byte value at length 1 (PROCESS1 chain)
        for b in 0u16..=255 {
            let data = [b as u8];
            same32(&c, &r, &data, 0, "row164 single byte");
            same32(&c, &r, &data, 0x9E37_79B1, "row164 single byte seeded");
        }
    }
}

// ===========================================================================
// Row 165 — LZ4_XXH32 length 8..15
// ===========================================================================

#[test]
fn row_165_xxh32_len_8_to_15() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(165);
        for &shape in &ALL_SHAPES {
            for len in 8usize..=15 {
                for _ in 0..4 {
                    let data = gen(&mut rng, shape, len);
                    for &seed in &seeds32() {
                        same32(&c, &r, &data, seed, &format!("row165 {shape:?}"));
                    }
                }
            }
        }
    }
}

// ===========================================================================
// Row 166 — LZ4_XXH32 stripes, big inputs, aligned + misaligned pointers
// ===========================================================================

#[test]
fn row_166_xxh32_stripe_boundaries_and_big_inputs() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(166);
        let lens = [16usize, 17, 31, 32, 33, 63, 64, 100, 127, 128, 1024, 65536, 1 << 20];
        for &shape in &ALL_SHAPES {
            for &len in &lens {
                let data = gen(&mut rng, shape, len);
                for &seed in &seeds32() {
                    same32(&c, &r, &data, seed, &format!("row166 {shape:?}"));
                }
            }
        }
    }
}

#[test]
fn row_166_xxh32_misaligned_input_pointers() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(1660);
        // One big buffer; hash slices starting at offsets 0..7 so the pointer
        // is deliberately unaligned for 4-byte reads.
        let big = gen(&mut rng, Shape::Incompressible, 4096 + 8);
        for off in 0usize..=7 {
            for len in 0usize..=200 {
                let s = &big[off..off + len];
                for &seed in &[0u32, 0x9E37_79B1, 0xFFFF_FFFF] {
                    same32(&c, &r, s, seed, &format!("row166 misaligned off={off}"));
                }
            }
            for &len in &[1024usize, 4096] {
                let s = &big[off..off + len];
                same32(&c, &r, s, 0, &format!("row166 misaligned big off={off}"));
                same32(&c, &r, s, 12345, &format!("row166 misaligned big off={off}"));
            }
        }
    }
}

#[test]
fn row_163_164_165_166_xxh32_exhaustive_len_0_to_300() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(0x3216_6000);
        // every `len & 15` residue class many times over, several shapes and
        // several seeds
        let shapes = [
            Shape::Incompressible,
            Shape::Compressible,
            Shape::TextLike,
            Shape::Degenerate,
        ];
        for &shape in &shapes {
            let base = gen(&mut rng, shape, 301);
            for len in 0usize..=300 {
                for &seed in &[0u32, 1, 0x9E37_79B1, 0xFFFF_FFFF, 0x1234_5678] {
                    same32(&c, &r, &base[..len], seed, &format!("sweep32 {shape:?}"));
                }
            }
        }
        // fresh random content per length as well
        for len in 0usize..=300 {
            let data = gen(&mut rng, Shape::Incompressible, len);
            let seed = rng.next_u32();
            same32(&c, &r, &data, seed, "sweep32 random seed");
        }
    }
}

// ===========================================================================
// Row 167 — XXH32 streaming basics
// ===========================================================================

#[test]
fn row_167_xxh32_stream_single_update_equals_oneshot() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(167);
        for &shape in &ALL_SHAPES {
            for len in [0usize, 1, 4, 15, 16, 17, 31, 32, 100, 1024, 65536] {
                let data = gen(&mut rng, shape, len);
                for &seed in &seeds32() {
                    let one = same32(&c, &r, &data, seed, "row167 oneshot");

                    let sc = (c.create)();
                    let sr = (r.create)();
                    assert!(!sc.is_null() && !sr.is_null(), "row167: createState NULL");
                    assert_eq!((c.reset)(sc, seed), (r.reset)(sr, seed), "row167 reset");
                    let p = data.as_ptr() as *const c_void;
                    assert_eq!((c.update)(sc, p, len), (r.update)(sr, p, len), "row167 update");
                    let dc = (c.digest)(sc);
                    let dr = (r.digest)(sr);
                    assert_eq!(dc, dr, "row167: digest mismatch len={len} seed={seed:#010x}");
                    assert_eq!(
                        dc, one,
                        "row167: single-update digest != one-shot (len={len} seed={seed:#010x})"
                    );
                    assert_eq!((c.free)(sc), (r.free)(sr), "row167 freeState");
                }
            }
        }
        // freeState(NULL) is tolerated and returns XXH_OK
        let fc = (c.free)(std::ptr::null_mut());
        let fr = (r.free)(std::ptr::null_mut());
        assert_eq!(fc, fr, "row167: XXH32_freeState(NULL) return mismatch");
        assert_eq!(fc, XXH_OK, "row167: XXH32_freeState(NULL) should be XXH_OK");
    }
}

// ===========================================================================
// Row 168 — XXH32_update chunkings
// ===========================================================================

#[test]
fn row_168_xxh32_update_chunkings() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(168);
        for len in [0usize, 1, 3, 8, 15, 16, 17, 20, 31, 32, 33, 47, 48, 64, 100, 257] {
            let data = gen(&mut rng, Shape::Incompressible, len);
            for pat in chunk_patterns(&mut rng, len, 16) {
                for &seed in &[0u32, 0x9E37_79B1] {
                    stream32(
                        &c,
                        &r,
                        &data,
                        seed,
                        &pat,
                        &format!("row168 len={len} pat={pat:?}"),
                    );
                }
            }
        }
        // larger payloads with randomized splits
        for len in [1024usize, 4096, 65536] {
            let data = gen(&mut rng, Shape::TextLike, len);
            for _ in 0..3 {
                let mut pat = Vec::new();
                let mut left = len;
                while left > 0 {
                    let n = rng.range(1, 200.min(left));
                    pat.push(n);
                    left -= n;
                }
                stream32(&c, &r, &data, 7, &pat, &format!("row168 big len={len}"));
            }
        }
    }
}

// ===========================================================================
// Row 169 — XXH32_digest repeated / XXH32_copyState
// ===========================================================================

#[test]
fn row_169_xxh32_digest_repeat_and_copystate() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(169);
        for len in [0usize, 5, 16, 17, 40, 1000] {
            let data = gen(&mut rng, Shape::TextLike, len);
            let seed = 0x9E37_79B1u32;

            let sc = (c.create)();
            let sr = (r.create)();
            assert!(!sc.is_null() && !sr.is_null());
            (c.reset)(sc, seed);
            (r.reset)(sr, seed);

            let half = len / 2;
            let p = data.as_ptr() as *const c_void;
            assert_eq!((c.update)(sc, p, half), (r.update)(sr, p, half));

            // digest twice — must be stable and identical
            let (a_c, a_r) = ((c.digest)(sc), (r.digest)(sr));
            let (b_c, b_r) = ((c.digest)(sc), (r.digest)(sr));
            assert_eq!(a_c, a_r, "row169: 1st digest mismatch (len={len})");
            assert_eq!(b_c, b_r, "row169: 2nd digest mismatch (len={len})");
            assert_eq!(a_c, b_c, "row169: C digest not idempotent");
            assert_eq!(a_r, b_r, "row169: Rust digest not idempotent");

            // copyState (same library only!) then diverging updates
            let cc = (c.create)();
            let cr = (r.create)();
            assert!(!cc.is_null() && !cr.is_null());
            (c.copy)(cc, sc as *const c_void);
            (r.copy)(cr, sr as *const c_void);
            let (d_c, d_r) = ((c.digest)(cc), (r.digest)(cr));
            assert_eq!(d_c, d_r, "row169: digest of copied state mismatch");
            assert_eq!(d_c, a_c, "row169: copied state digest != original");

            // diverge: feed the rest to the original, something else to the copy
            let rest = &data[half..];
            let pr = rest.as_ptr() as *const c_void;
            assert_eq!(
                (c.update)(sc, pr, rest.len()),
                (r.update)(sr, pr, rest.len())
            );
            let other = gen(&mut rng, Shape::Periodic, 37);
            let po = other.as_ptr() as *const c_void;
            assert_eq!((c.update)(cc, po, other.len()), (r.update)(cr, po, other.len()));

            let (e_c, e_r) = ((c.digest)(sc), (r.digest)(sr));
            let (f_c, f_r) = ((c.digest)(cc), (r.digest)(cr));
            assert_eq!(e_c, e_r, "row169: original-after-divergence mismatch");
            assert_eq!(f_c, f_r, "row169: copy-after-divergence mismatch");
            assert_eq!(
                e_c,
                same32(&c, &r, &data, seed, "row169 full"),
                "row169: original stream != one-shot of the whole input"
            );
            let mut cat = data[..half].to_vec();
            cat.extend_from_slice(&other);
            assert_eq!(
                f_c,
                same32(&c, &r, &cat, seed, "row169 copy-concat"),
                "row169: copy stream != one-shot of prefix||other"
            );

            // digest, then further updates, then digest again
            let sc2 = (c.create)();
            let sr2 = (r.create)();
            (c.reset)(sc2, seed);
            (r.reset)(sr2, seed);
            let mut consumed = 0usize;
            for step in [3usize, 13, 16, 1, 31] {
                let n = step.min(len - consumed);
                let pp = data[consumed..].as_ptr() as *const c_void;
                assert_eq!((c.update)(sc2, pp, n), (r.update)(sr2, pp, n));
                consumed += n;
                let g_c = (c.digest)(sc2);
                let g_r = (r.digest)(sr2);
                assert_eq!(g_c, g_r, "row169: interleaved digest mismatch at {consumed}");
                assert_eq!(
                    g_c,
                    same32(&c, &r, &data[..consumed], seed, "row169 interleaved"),
                    "row169: interleaved digest != one-shot"
                );
            }

            for s in [sc, sr, cc, cr, sc2, sr2] {
                let _ = s;
            }
            assert_eq!((c.free)(sc), (r.free)(sr));
            assert_eq!((c.free)(cc), (r.free)(cr));
            assert_eq!((c.free)(sc2), (r.free)(sr2));
        }
    }
}

// ===========================================================================
// Row 170 — XXH32 canonical representation
// ===========================================================================

#[test]
fn row_170_xxh32_canonical_roundtrip() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(170);
        let mut values: Vec<u32> = vec![0, 0xFFFF_FFFF, 1, 0x0000_00FF, 0x1234_5678, 0x8000_0000];
        for _ in 0..500 {
            values.push(rng.next_u32());
        }
        for v in values {
            let mut bc = [0u8; 4];
            let mut br = [0u8; 4];
            (c.canon)(bc.as_mut_ptr() as *mut c_void, v);
            (r.canon)(br.as_mut_ptr() as *mut c_void, v);
            assert_eq!(
                bc, br,
                "row170: canonicalFromHash bytes differ for {v:#010x}: C={bc:02x?} Rust={br:02x?}"
            );
            // big-endian byte order
            assert_eq!(bc, v.to_be_bytes(), "row170: canonical is not big-endian");

            let hc = (c.from_canon)(bc.as_ptr() as *const c_void);
            let hr = (r.from_canon)(bc.as_ptr() as *const c_void);
            assert_eq!(hc, hr, "row170: hashFromCanonical mismatch for {v:#010x}");
            assert_eq!(hc, v, "row170: round-trip changed the value");
        }
        // hashFromCanonical on arbitrary bytes (not produced by canonicalFromHash)
        for _ in 0..200 {
            let b = [rng.byte(), rng.byte(), rng.byte(), rng.byte()];
            let hc = (c.from_canon)(b.as_ptr() as *const c_void);
            let hr = (r.from_canon)(b.as_ptr() as *const c_void);
            assert_eq!(hc, hr, "row170: hashFromCanonical mismatch for {b:02x?}");
        }
    }
}

// ===========================================================================
// Row 171 — LZ4_XXH64 lengths + alignment
// ===========================================================================

#[test]
fn row_171_xxh64_lengths_and_null_and_misaligned() {
    unsafe {
        let (c, r) = api64();
        let mut rng = Rng::new(171);

        // length 0 with a valid pointer and with NULL
        let buf = [0u8; 64];
        for &seed in &[0u64, 1, 0x9E37_79B1_85EB_CA87, u64::MAX] {
            let h = same64(&c, &r, &buf[..0], seed, "row171 len0");
            let hc = (c.xxh)(std::ptr::null(), 0, seed);
            let hr = (r.xxh)(std::ptr::null(), 0, seed);
            assert_eq!(
                hc, hr,
                "row171: LZ4_XXH64(NULL,0,{seed:#018x}) C={hc:#018x} Rust={hr:#018x}"
            );
            assert_eq!(h, hc, "row171: NULL/len0 must equal valid-ptr/len0");
        }

        // every `len & 31` residue class plus the documented boundaries
        let mut lens: Vec<usize> = (0usize..=64).collect();
        lens.extend_from_slice(&[95, 96, 97, 127, 128, 1024, 65536, 1 << 20]);
        for &shape in &ALL_SHAPES {
            for &len in &lens {
                let data = gen(&mut rng, shape, len);
                for &seed in &seeds64() {
                    same64(&c, &r, &data, seed, &format!("row171 {shape:?}"));
                }
            }
        }

        // 8-byte-aligned vs deliberately misaligned input pointers
        let big = gen(&mut rng, Shape::Incompressible, 4096 + 8);
        assert_eq!(
            (big.as_ptr() as usize) & 7,
            0,
            "row171: expected the malloc'd buffer to be 8-byte aligned"
        );
        for off in 0usize..=7 {
            for len in 0usize..=200 {
                let s = &big[off..off + len];
                for &seed in &[0u64, 0x9E37_79B1_85EB_CA87, u64::MAX] {
                    same64(&c, &r, s, seed, &format!("row171 misaligned off={off}"));
                }
            }
            for &len in &[1024usize, 4096] {
                let s = &big[off..off + len];
                same64(&c, &r, s, 0, &format!("row171 misaligned big off={off}"));
                same64(&c, &r, s, 99, &format!("row171 misaligned big off={off}"));
            }
        }
    }
}

#[test]
fn row_171_xxh64_exhaustive_len_0_to_300() {
    unsafe {
        let (c, r) = api64();
        let mut rng = Rng::new(0x6417_1000);
        let shapes = [
            Shape::Incompressible,
            Shape::Compressible,
            Shape::TextLike,
            Shape::Degenerate,
        ];
        for &shape in &shapes {
            let base = gen(&mut rng, shape, 301);
            for len in 0usize..=300 {
                for &seed in &[0u64, 1, 0x9E37_79B1_85EB_CA87, u64::MAX, 0x1234_5678_9ABC_DEF0] {
                    same64(&c, &r, &base[..len], seed, &format!("sweep64 {shape:?}"));
                }
            }
        }
        for len in 0usize..=300 {
            let data = gen(&mut rng, Shape::Incompressible, len);
            let seed = rng.next_u64();
            same64(&c, &r, &data, seed, "sweep64 random seed");
        }
    }
}

// ===========================================================================
// Row 172 — XXH64 streaming
// ===========================================================================

#[test]
fn row_172_xxh64_streaming_and_chunkings() {
    unsafe {
        let (c, r) = api64();
        let mut rng = Rng::new(172);

        // single update equals the one-shot
        for &shape in &ALL_SHAPES {
            for len in [0usize, 1, 8, 31, 32, 33, 63, 64, 100, 1024, 65536] {
                let data = gen(&mut rng, shape, len);
                for &seed in &seeds64() {
                    let one = same64(&c, &r, &data, seed, "row172 oneshot");
                    let sc = (c.create)();
                    let sr = (r.create)();
                    assert!(!sc.is_null() && !sr.is_null());
                    assert_eq!((c.reset)(sc, seed), (r.reset)(sr, seed), "row172 reset");
                    let p = data.as_ptr() as *const c_void;
                    assert_eq!((c.update)(sc, p, len), (r.update)(sr, p, len), "row172 update");
                    let dc = (c.digest)(sc);
                    let dr = (r.digest)(sr);
                    assert_eq!(dc, dr, "row172: digest mismatch len={len} seed={seed:#018x}");
                    assert_eq!(dc, one, "row172: single update != one-shot (len={len})");
                    assert_eq!((c.free)(sc), (r.free)(sr));
                }
            }
        }

        // freeState(NULL)
        let fc = (c.free)(std::ptr::null_mut());
        let fr = (r.free)(std::ptr::null_mut());
        assert_eq!(fc, fr, "row172: XXH64_freeState(NULL) mismatch");
        assert_eq!(fc, XXH_OK);

        // chunkings: 1 byte at a time, sums < 32, straddling the 32-byte stripe
        for len in [0usize, 1, 7, 16, 31, 32, 33, 63, 64, 65, 96, 100, 257] {
            let data = gen(&mut rng, Shape::Incompressible, len);
            for pat in chunk_patterns(&mut rng, len, 32) {
                for &seed in &[0u64, u64::MAX] {
                    stream64(
                        &c,
                        &r,
                        &data,
                        seed,
                        &pat,
                        &format!("row172 len={len} pat={pat:?}"),
                    );
                }
            }
        }
        for len in [1024usize, 4096, 65536] {
            let data = gen(&mut rng, Shape::TextLike, len);
            for _ in 0..3 {
                let mut pat = Vec::new();
                let mut left = len;
                while left > 0 {
                    let n = rng.range(1, 200.min(left));
                    pat.push(n);
                    left -= n;
                }
                stream64(&c, &r, &data, 5, &pat, &format!("row172 big len={len}"));
            }
        }

        // copyState + diverging updates, digest twice, digest/update/digest
        for len in [0usize, 8, 32, 33, 500] {
            let data = gen(&mut rng, Shape::Compressible, len);
            let seed = 0x9E37_79B1_85EB_CA87u64;
            let sc = (c.create)();
            let sr = (r.create)();
            (c.reset)(sc, seed);
            (r.reset)(sr, seed);
            let half = len / 2;
            let p = data.as_ptr() as *const c_void;
            assert_eq!((c.update)(sc, p, half), (r.update)(sr, p, half));
            let (a_c, a_r) = ((c.digest)(sc), (r.digest)(sr));
            let (b_c, b_r) = ((c.digest)(sc), (r.digest)(sr));
            assert_eq!(a_c, a_r, "row172: digest#1 mismatch");
            assert_eq!(b_c, b_r, "row172: digest#2 mismatch");
            assert_eq!(a_c, b_c);
            assert_eq!(a_r, b_r);

            let cc = (c.create)();
            let cr = (r.create)();
            (c.copy)(cc, sc as *const c_void);
            (r.copy)(cr, sr as *const c_void);
            assert_eq!((c.digest)(cc), (r.digest)(cr), "row172: copied digest mismatch");

            let other = gen(&mut rng, Shape::Periodic, 41);
            let po = other.as_ptr() as *const c_void;
            assert_eq!((c.update)(cc, po, other.len()), (r.update)(cr, po, other.len()));
            let rest = &data[half..];
            let pr = rest.as_ptr() as *const c_void;
            assert_eq!((c.update)(sc, pr, rest.len()), (r.update)(sr, pr, rest.len()));
            let (e_c, e_r) = ((c.digest)(sc), (r.digest)(sr));
            let (f_c, f_r) = ((c.digest)(cc), (r.digest)(cr));
            assert_eq!(e_c, e_r, "row172: diverged original mismatch");
            assert_eq!(f_c, f_r, "row172: diverged copy mismatch");
            assert_eq!(e_c, same64(&c, &r, &data, seed, "row172 full"));
            let mut cat = data[..half].to_vec();
            cat.extend_from_slice(&other);
            assert_eq!(f_c, same64(&c, &r, &cat, seed, "row172 copy-concat"));

            assert_eq!((c.free)(sc), (r.free)(sr));
            assert_eq!((c.free)(cc), (r.free)(cr));
        }
    }
}

// ===========================================================================
// Row 173 — XXH64 canonical + LZ4_XXH_versionNumber
// ===========================================================================

#[test]
fn row_173_xxh64_canonical_roundtrip_and_version() {
    unsafe {
        let (c, r) = api64();
        let mut rng = Rng::new(173);
        let mut values: Vec<u64> = vec![
            0,
            u64::MAX,
            1,
            0xFF,
            0x1234_5678_9ABC_DEF0,
            0x8000_0000_0000_0000,
        ];
        for _ in 0..500 {
            values.push(rng.next_u64());
        }
        for v in values {
            let mut bc = [0u8; 8];
            let mut br = [0u8; 8];
            (c.canon)(bc.as_mut_ptr() as *mut c_void, v);
            (r.canon)(br.as_mut_ptr() as *mut c_void, v);
            assert_eq!(
                bc, br,
                "row173: canonicalFromHash bytes differ for {v:#018x}: C={bc:02x?} Rust={br:02x?}"
            );
            assert_eq!(bc, v.to_be_bytes(), "row173: canonical is not big-endian");
            let hc = (c.from_canon)(bc.as_ptr() as *const c_void);
            let hr = (r.from_canon)(bc.as_ptr() as *const c_void);
            assert_eq!(hc, hr, "row173: hashFromCanonical mismatch for {v:#018x}");
            assert_eq!(hc, v, "row173: round-trip changed the value");
        }
        for _ in 0..200 {
            let mut b = [0u8; 8];
            for x in b.iter_mut() {
                *x = rng.byte();
            }
            let hc = (c.from_canon)(b.as_ptr() as *const c_void);
            let hr = (r.from_canon)(b.as_ptr() as *const c_void);
            assert_eq!(hc, hr, "row173: hashFromCanonical mismatch for {b:02x?}");
        }

        let (vc, vr) = pair!(libs(), FnVersion, "LZ4_XXH_versionNumber");
        assert_ne!(
            vc as usize, vr as usize,
            "harness bug: the C and Rust symbols resolved to the same address"
        );
        assert_ne!(c.xxh as usize, r.xxh as usize, "harness bug: LZ4_XXH64 aliased");
        let (a, b) = (vc(), vr());
        assert_eq!(a, b, "row173: LZ4_XXH_versionNumber C={a} Rust={b}");
        assert_eq!(a, 605, "row173: expected XXH 0.6.5 -> 605, got {a}");
    }
}

// ===========================================================================
// ERRORS.md rows 185..194 (xxhash.c)
// ===========================================================================

/// ERRORS row 185 — `XXH32_update(state, NULL, len)` with the default
/// `XXH_ACCEPT_NULL_INPUT_POINTER == 0` must return `XXH_ERROR` (1) for both
/// zero and non-zero lengths, and must not disturb the state.
#[test]
fn errors_185_xxh32_update_null_input_returns_xxh_error() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(185);
        let data = gen(&mut rng, Shape::TextLike, 100);

        for &len in &[0usize, 1, 15, 16, 17, 1000] {
            let sc = (c.create)();
            let sr = (r.create)();
            assert_eq!((c.reset)(sc, 0), (r.reset)(sr, 0));
            let p = data.as_ptr() as *const c_void;
            assert_eq!((c.update)(sc, p, 40), (r.update)(sr, p, 40));

            let ec = (c.update)(sc, std::ptr::null(), len);
            let er = (r.update)(sr, std::ptr::null(), len);
            assert_eq!(
                ec, er,
                "errors185: XXH32_update(state,NULL,{len}) C={ec} Rust={er}"
            );
            assert_eq!(
                ec, XXH_ERROR,
                "errors185: XXH32_update(state,NULL,{len}) must be XXH_ERROR(1), got {ec}"
            );

            // the state must be untouched: digest still equals the 40-byte prefix
            let dc = (c.digest)(sc);
            let dr = (r.digest)(sr);
            assert_eq!(dc, dr, "errors185: digest after rejected update mismatch");
            assert_eq!(
                dc,
                same32(&c, &r, &data[..40], 0, "errors185"),
                "errors185: rejected update modified the state"
            );
            assert_eq!((c.free)(sc), (r.free)(sr));
        }
    }
}

/// ERRORS row 186 — same for `XXH64_update`.
#[test]
fn errors_186_xxh64_update_null_input_returns_xxh_error() {
    unsafe {
        let (c, r) = api64();
        let mut rng = Rng::new(186);
        let data = gen(&mut rng, Shape::TextLike, 100);

        for &len in &[0usize, 1, 31, 32, 33, 1000] {
            let sc = (c.create)();
            let sr = (r.create)();
            assert_eq!((c.reset)(sc, 0), (r.reset)(sr, 0));
            let p = data.as_ptr() as *const c_void;
            assert_eq!((c.update)(sc, p, 40), (r.update)(sr, p, 40));

            let ec = (c.update)(sc, std::ptr::null(), len);
            let er = (r.update)(sr, std::ptr::null(), len);
            assert_eq!(
                ec, er,
                "errors186: XXH64_update(state,NULL,{len}) C={ec} Rust={er}"
            );
            assert_eq!(
                ec, XXH_ERROR,
                "errors186: XXH64_update(state,NULL,{len}) must be XXH_ERROR(1), got {ec}"
            );

            let dc = (c.digest)(sc);
            let dr = (r.digest)(sr);
            assert_eq!(dc, dr, "errors186: digest after rejected update mismatch");
            assert_eq!(
                dc,
                same64(&c, &r, &data[..40], 0, "errors186"),
                "errors186: rejected update modified the state"
            );
            assert_eq!((c.free)(sc), (r.free)(sr));
        }
    }
}

/// ERRORS rows 187/188 — this build has `XXH_ACCEPT_NULL_INPUT_POINTER == 0`,
/// so `update(NULL, ...)` is an error (rows 185/186) and the one-shots only
/// tolerate NULL when `length == 0` (no dereference happens on that path).
#[test]
fn errors_187_188_oneshot_null_input_length_zero() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        for &seed in &[0u32, 1, 0x9E37_79B1, 0xFFFF_FFFF] {
            let hc = (c32.xxh)(std::ptr::null(), 0, seed);
            let hr = (r32.xxh)(std::ptr::null(), 0, seed);
            assert_eq!(
                hc, hr,
                "errors188: LZ4_XXH32(NULL,0,{seed:#010x}) C={hc:#010x} Rust={hr:#010x}"
            );
            // must be the hash of an empty input, NOT the "accept NULL" special case
            let empty: [u8; 1] = [0];
            let he = (c32.xxh)(empty.as_ptr() as *const c_void, 0, seed);
            assert_eq!(hc, he, "errors188: XXH32(NULL,0) != XXH32(valid,0)");
        }
        for &seed in &[0u64, 1, u64::MAX] {
            let hc = (c64.xxh)(std::ptr::null(), 0, seed);
            let hr = (r64.xxh)(std::ptr::null(), 0, seed);
            assert_eq!(
                hc, hr,
                "errors188: LZ4_XXH64(NULL,0,{seed:#018x}) C={hc:#018x} Rust={hr:#018x}"
            );
            let empty: [u8; 1] = [0];
            let he = (c64.xxh)(empty.as_ptr() as *const c_void, 0, seed);
            assert_eq!(hc, he, "errors188: XXH64(NULL,0) != XXH64(valid,0)");
        }
    }
}

/// ERRORS rows 189/190 — `createState` returns a non-NULL pointer under normal
/// conditions (the NULL-return path needs a malloc failure, which cannot be
/// forced here); the returned state is usable and freeable by its own library.
#[test]
fn errors_189_190_createstate_returns_usable_state() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        for _ in 0..64 {
            let sc = (c32.create)();
            let sr = (r32.create)();
            assert!(!sc.is_null(), "errors189: C XXH32_createState returned NULL");
            assert!(!sr.is_null(), "errors189: Rust XXH32_createState returned NULL");
            assert_eq!((c32.reset)(sc, 0), (r32.reset)(sr, 0));
            assert_eq!((c32.digest)(sc), (r32.digest)(sr));
            assert_eq!((c32.free)(sc), (r32.free)(sr));

            let sc = (c64.create)();
            let sr = (r64.create)();
            assert!(!sc.is_null(), "errors190: C XXH64_createState returned NULL");
            assert!(!sr.is_null(), "errors190: Rust XXH64_createState returned NULL");
            assert_eq!((c64.reset)(sc, 0), (r64.reset)(sr, 0));
            assert_eq!((c64.digest)(sc), (r64.digest)(sr));
            assert_eq!((c64.free)(sc), (r64.free)(sr));
        }
    }
}

/// ERRORS row 191 — `freeState(NULL)` is tolerated and returns `XXH_OK`.
#[test]
fn errors_191_freestate_null_returns_xxh_ok() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        let a = (c32.free)(std::ptr::null_mut());
        let b = (r32.free)(std::ptr::null_mut());
        assert_eq!(a, b, "errors191: XXH32_freeState(NULL) C={a} Rust={b}");
        assert_eq!(a, XXH_OK, "errors191: XXH32_freeState(NULL) must be XXH_OK");
        let a = (c64.free)(std::ptr::null_mut());
        let b = (r64.free)(std::ptr::null_mut());
        assert_eq!(a, b, "errors191: XXH64_freeState(NULL) C={a} Rust={b}");
        assert_eq!(a, XXH_OK, "errors191: XXH64_freeState(NULL) must be XXH_OK");
    }
}

/// ERRORS row 192 — `reset` has no error path: it always returns `XXH_OK`, for
/// every seed. (A NULL state is undefined behaviour and is not exercised.)
#[test]
fn errors_192_reset_always_returns_xxh_ok() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        let sc = (c32.create)();
        let sr = (r32.create)();
        for &seed in &seeds32() {
            let a = (c32.reset)(sc, seed);
            let b = (r32.reset)(sr, seed);
            assert_eq!(a, b, "errors192: XXH32_reset({seed:#010x}) C={a} Rust={b}");
            assert_eq!(a, XXH_OK, "errors192: XXH32_reset must return XXH_OK");
            // a freshly reset state digests to the empty-input hash
            assert_eq!(
                (c32.digest)(sc),
                (r32.digest)(sr),
                "errors192: digest after reset mismatch"
            );
            let e: [u8; 1] = [0];
            assert_eq!(
                (c32.digest)(sc),
                (c32.xxh)(e.as_ptr() as *const c_void, 0, seed),
                "errors192: reset+digest != XXH32(empty)"
            );
        }
        assert_eq!((c32.free)(sc), (r32.free)(sr));

        let sc = (c64.create)();
        let sr = (r64.create)();
        for &seed in &seeds64() {
            let a = (c64.reset)(sc, seed);
            let b = (r64.reset)(sr, seed);
            assert_eq!(a, b, "errors192: XXH64_reset({seed:#018x}) C={a} Rust={b}");
            assert_eq!(a, XXH_OK, "errors192: XXH64_reset must return XXH_OK");
            assert_eq!(
                (c64.digest)(sc),
                (r64.digest)(sr),
                "errors192: digest after reset mismatch"
            );
            let e: [u8; 1] = [0];
            assert_eq!(
                (c64.digest)(sc),
                (c64.xxh)(e.as_ptr() as *const c_void, 0, seed),
                "errors192: reset+digest != XXH64(empty)"
            );
        }
        assert_eq!((c64.free)(sc), (r64.free)(sr));
    }
}

/// ERRORS row 193 — `digest` has no error path: it always yields a hash, and
/// repeated calls (including after further updates) never report an error.
/// ERRORS row 194 — `copyState` is a plain `memcpy`: the copy must behave
/// exactly like the source. Copies are always taken between two states of the
/// SAME library.
#[test]
fn errors_193_194_digest_and_copystate_have_no_error_path() {
    unsafe {
        let (c, r) = api32();
        let (c64, r64) = api64();
        let mut rng = Rng::new(194);
        let data = gen(&mut rng, Shape::Incompressible, 200);

        let sc = (c.create)();
        let sr = (r.create)();
        (c.reset)(sc, 0);
        (r.reset)(sr, 0);
        let p = data.as_ptr() as *const c_void;
        (c.update)(sc, p, 200);
        (r.update)(sr, p, 200);
        for _ in 0..5 {
            assert_eq!((c.digest)(sc), (r.digest)(sr), "errors193: XXH32 digest not stable");
        }
        // chain of copies, each of which must reproduce the same digest
        let mut prev_c = sc;
        let mut prev_r = sr;
        let mut owned: Vec<(*mut c_void, *mut c_void)> = Vec::new();
        for _ in 0..4 {
            let nc = (c.create)();
            let nr = (r.create)();
            (c.copy)(nc, prev_c as *const c_void);
            (r.copy)(nr, prev_r as *const c_void);
            assert_eq!((c.digest)(nc), (r.digest)(nr), "errors194: copied digest mismatch");
            assert_eq!(
                (c.digest)(nc),
                (c.digest)(sc),
                "errors194: copy digest differs from the source (C)"
            );
            owned.push((nc, nr));
            prev_c = nc;
            prev_r = nr;
        }
        for (a, b) in owned {
            assert_eq!((c.free)(a), (r.free)(b));
        }
        assert_eq!((c.free)(sc), (r.free)(sr));

        let sc = (c64.create)();
        let sr = (r64.create)();
        (c64.reset)(sc, u64::MAX);
        (r64.reset)(sr, u64::MAX);
        (c64.update)(sc, p, 200);
        (r64.update)(sr, p, 200);
        for _ in 0..5 {
            assert_eq!((c64.digest)(sc), (r64.digest)(sr), "errors193: XXH64 digest not stable");
        }
        let nc = (c64.create)();
        let nr = (r64.create)();
        (c64.copy)(nc, sc as *const c_void);
        (r64.copy)(nr, sr as *const c_void);
        assert_eq!((c64.digest)(nc), (r64.digest)(nr), "errors194: XXH64 copy mismatch");
        assert_eq!((c64.free)(nc), (r64.free)(nr));
        assert_eq!((c64.free)(sc), (r64.free)(sr));
    }
}

/// ERRORS row 195 — the `finalize` "impossible" fallthrough. It is reached
/// only for `len & 15` / `len & 31` values outside the switch, which cannot
/// happen; this test simply pins every residue class so that a mistranslated
/// switch chain shows up as a hash mismatch.
#[test]
fn errors_195_finalize_all_residue_classes() {
    unsafe {
        let (c, r) = api32();
        let (c64, r64) = api64();
        let mut rng = Rng::new(195);
        let base = gen(&mut rng, Shape::Incompressible, 128);
        for len in 0usize..=127 {
            same32(&c, &r, &base[..len], 0, "errors195 xxh32");
            same32(&c, &r, &base[..len], 0xDEAD_BEEF, "errors195 xxh32 seeded");
            same64(&c64, &r64, &base[..len], 0, "errors195 xxh64");
            same64(&c64, &r64, &base[..len], u64::MAX, "errors195 xxh64 seeded");
        }
    }
}
