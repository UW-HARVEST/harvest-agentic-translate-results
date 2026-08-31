//! Per-row differential tests for `ERRORS.md` rows 185..198 — the `## xxhash.c`
//! section, plus the three compile-time rows that trail it (196/197/198).
//!
//! EXACTLY ONE `#[test] fn err_NNN_...()` per ERRORS.md row, so that the audit
//! trail from row number to test is mechanical:
//!
//! | row | test |
//! |-----|------|
//! | 185 | `err_185_xxh32_update_null_input` |
//! | 186 | `err_186_xxh64_update_null_input` |
//! | 187 | `err_187_update_null_input_accept_null_input_pointer` |
//! | 188 | `err_188_oneshot_null_input` |
//! | 189 | `err_189_xxh32_createstate_malloc_failure` |
//! | 190 | `err_190_xxh64_createstate_malloc_failure` |
//! | 191 | `err_191_freestate_null_returns_xxh_ok` |
//! | 192 | `err_192_reset_null_state_no_check` |
//! | 193 | `err_193_digest_has_no_error_path` |
//! | 194 | `err_194_copystate_null_memcpy` |
//! | 195 | `err_195_finalize_unreachable_assert` |
//! | 196 | `err_196_canonical_static_assert` |
//! | 197 | `err_197_lz4frame_ptrdiff_static_assert` |
//! | 198 | `err_198_lz4_memory_usage_range_error` |
//!
//! Ground rules honoured throughout:
//!   * every call goes through a `.so` export via `libloading` — no Rust
//!     function is ever called directly;
//!   * `LZ4_XXH{32,64}_state_t` values are created, copied and freed by the
//!     SAME library, and `copyState` only ever copies between two states of the
//!     same library;
//!   * rows whose trigger is undefined behaviour (NULL deref), a compile-time
//!     `#error` / static assert, an unreachable `assert(0)`, or a `malloc`
//!     failure that cannot be hooked are NOT actually triggered: the test
//!     documents why and pins the closest reachable in-contract behaviour
//!     instead.
#![allow(unused_imports, non_snake_case)]

mod common;
use common::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// FFI signatures (namespaced with XXH_NAMESPACE=LZ4_)
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

type FnIsErr = unsafe extern "C" fn(usize) -> c_uint;
type FnErrName = unsafe extern "C" fn(usize) -> *const c_char;
type FnErrCode = unsafe extern "C" fn(usize) -> c_int;

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
    // Guard against the harness accidentally resolving both handles to the same
    // object: every assertion below would then be vacuous.
    assert_ne!(
        xxh_c as usize, xxh_r as usize,
        "harness bug: LZ4_XXH32 resolved to the same address in both libraries"
    );
    (
        Api32 { xxh: xxh_c, create: cre_c, free: fre_c, copy: cpy_c, reset: rst_c,
                update: upd_c, digest: dig_c, canon: can_c, from_canon: fca_c },
        Api32 { xxh: xxh_r, create: cre_r, free: fre_r, copy: cpy_r, reset: rst_r,
                update: upd_r, digest: dig_r, canon: can_r, from_canon: fca_r },
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
    assert_ne!(
        xxh_c as usize, xxh_r as usize,
        "harness bug: LZ4_XXH64 resolved to the same address in both libraries"
    );
    (
        Api64 { xxh: xxh_c, create: cre_c, free: fre_c, copy: cpy_c, reset: rst_c,
                update: upd_c, digest: dig_c, canon: can_c, from_canon: fca_c },
        Api64 { xxh: xxh_r, create: cre_r, free: fre_r, copy: cpy_r, reset: rst_r,
                update: upd_r, digest: dig_r, canon: can_r, from_canon: fca_r },
    )
}

// ---------------------------------------------------------------------------
// Small comparison helpers
// ---------------------------------------------------------------------------

#[track_caller]
unsafe fn same32(c: &Api32, r: &Api32, buf: &[u8], seed: u32, ctx: &str) -> u32 {
    let p = buf.as_ptr() as *const c_void;
    let hc = (c.xxh)(p, buf.len(), seed);
    let hr = (r.xxh)(p, buf.len(), seed);
    assert_eq!(
        hc, hr,
        "{ctx}: LZ4_XXH32 mismatch len={} seed={seed:#010x} C={hc:#010x} Rust={hr:#010x}\n  input: {}",
        buf.len(),
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
        "{ctx}: LZ4_XXH64 mismatch len={} seed={seed:#018x} C={hc:#018x} Rust={hr:#018x}\n  input: {}",
        buf.len(),
        hexdump(buf)
    );
    hc
}

/// A fresh, reset, `prefix`-fed state pair (C, Rust). Each state belongs to its
/// own library and must be freed by that same library.
#[track_caller]
unsafe fn primed32(c: &Api32, r: &Api32, prefix: &[u8], seed: u32) -> (*mut c_void, *mut c_void) {
    let sc = (c.create)();
    let sr = (r.create)();
    assert!(!sc.is_null(), "C LZ4_XXH32_createState returned NULL");
    assert!(!sr.is_null(), "Rust LZ4_XXH32_createState returned NULL");
    assert_eq!((c.reset)(sc, seed), (r.reset)(sr, seed), "XXH32_reset mismatch");
    let p = prefix.as_ptr() as *const c_void;
    assert_eq!(
        (c.update)(sc, p, prefix.len()),
        (r.update)(sr, p, prefix.len()),
        "XXH32_update(prefix) mismatch"
    );
    (sc, sr)
}

#[track_caller]
unsafe fn primed64(c: &Api64, r: &Api64, prefix: &[u8], seed: u64) -> (*mut c_void, *mut c_void) {
    let sc = (c.create)();
    let sr = (r.create)();
    assert!(!sc.is_null(), "C LZ4_XXH64_createState returned NULL");
    assert!(!sr.is_null(), "Rust LZ4_XXH64_createState returned NULL");
    assert_eq!((c.reset)(sc, seed), (r.reset)(sr, seed), "XXH64_reset mismatch");
    let p = prefix.as_ptr() as *const c_void;
    assert_eq!(
        (c.update)(sc, p, prefix.len()),
        (r.update)(sr, p, prefix.len()),
        "XXH64_update(prefix) mismatch"
    );
    (sc, sr)
}

// ===========================================================================
// Row 185 — `XXH32_update(state, NULL, len)` with the default
//           `XXH_ACCEPT_NULL_INPUT_POINTER == 0` (xxhash.c:70-72) returns
//           `XXH_ERROR` (1)  — xxhash.c:454-458.
//
// FORCED FOR REAL. `XXH32_update_endian` tests `input == NULL` *before* any
// dereference and before touching the state, so this is a fully in-contract,
// crash-free call for every `len`.
// ===========================================================================

#[test]
fn err_185_xxh32_update_null_input() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(0x0185_0000);

        // Property style: many prefix shapes/lengths and many seeds, so the
        // "state left untouched" claim is checked from a wide variety of
        // internal states (partially-filled 16-byte buffer, empty buffer,
        // several full stripes already absorbed, ...).
        for &shape in &ALL_SHAPES {
            for prefix_len in [0usize, 1, 7, 15, 16, 17, 40, 63, 64, 65, 1000] {
                let prefix = gen(&mut rng, shape, prefix_len);
                let seed = rng.next_u32();
                let (sc, sr) = primed32(&c, &r, &prefix, seed);

                // The digest of the state BEFORE the rejected update, and the
                // value it must equal (each library's own one-shot).
                let before_c = (c.digest)(sc);
                let before_r = (r.digest)(sr);
                assert_eq!(
                    before_c, before_r,
                    "row185: digest before the NULL update differs (prefix_len={prefix_len})"
                );
                let oneshot = same32(&c, &r, &prefix, seed, "row185 baseline");
                assert_eq!(before_c, oneshot, "row185: primed digest != one-shot");

                for &len in &[0usize, 1, 15, 16, 17, 1000] {
                    let ec = (c.update)(sc, std::ptr::null(), len);
                    let er = (r.update)(sr, std::ptr::null(), len);
                    assert_eq!(
                        ec, er,
                        "row185: LZ4_XXH32_update(state, NULL, {len}) C={ec} Rust={er}"
                    );
                    assert_eq!(
                        ec, XXH_ERROR,
                        "row185: LZ4_XXH32_update(state, NULL, {len}) must be XXH_ERROR(1), got {ec}"
                    );

                    // ... and the state must be bit-for-bit unaffected: the
                    // digest is unchanged and still equals the one-shot.
                    let after_c = (c.digest)(sc);
                    let after_r = (r.digest)(sr);
                    assert_eq!(
                        after_c, before_c,
                        "row185: C state was modified by the rejected update (len={len})"
                    );
                    assert_eq!(
                        after_r, before_r,
                        "row185: Rust state was modified by the rejected update (len={len})"
                    );
                    assert_eq!(after_c, after_r, "row185: digests diverge after len={len}");
                    assert_eq!(after_c, oneshot, "row185: digest != one-shot after len={len}");
                }

                // A real update after the rejected ones must still work and
                // agree, proving the state is genuinely still live.
                let tail = gen(&mut rng, shape, 37);
                let tp = tail.as_ptr() as *const c_void;
                assert_eq!(
                    (c.update)(sc, tp, tail.len()),
                    XXH_OK,
                    "row185: C update after rejected NULL update should succeed"
                );
                assert_eq!(
                    (r.update)(sr, tp, tail.len()),
                    XXH_OK,
                    "row185: Rust update after rejected NULL update should succeed"
                );
                let mut cat = prefix.clone();
                cat.extend_from_slice(&tail);
                let dc = (c.digest)(sc);
                let dr = (r.digest)(sr);
                assert_eq!(dc, dr, "row185: post-recovery digest mismatch");
                assert_eq!(
                    dc,
                    same32(&c, &r, &cat, seed, "row185 recovery"),
                    "row185: post-recovery digest != one-shot(prefix||tail)"
                );

                assert_eq!((c.free)(sc), (r.free)(sr), "row185: freeState mismatch");
            }
        }
    }
}

// ===========================================================================
// Row 186 — same as row 185 for `XXH64_update` / `XXH64_update_endian`
//           (xxhash.c:914-918) → `XXH_ERROR` (1).
//
// FORCED FOR REAL, same reasoning as row 185.
// ===========================================================================

#[test]
fn err_186_xxh64_update_null_input() {
    unsafe {
        let (c, r) = api64();
        let mut rng = Rng::new(0x0186_0000);

        for &shape in &ALL_SHAPES {
            for prefix_len in [0usize, 1, 7, 31, 32, 33, 40, 63, 64, 65, 1000] {
                let prefix = gen(&mut rng, shape, prefix_len);
                let seed = rng.next_u64();
                let (sc, sr) = primed64(&c, &r, &prefix, seed);

                let before_c = (c.digest)(sc);
                let before_r = (r.digest)(sr);
                assert_eq!(
                    before_c, before_r,
                    "row186: digest before the NULL update differs (prefix_len={prefix_len})"
                );
                let oneshot = same64(&c, &r, &prefix, seed, "row186 baseline");
                assert_eq!(before_c, oneshot, "row186: primed digest != one-shot");

                for &len in &[0usize, 1, 15, 16, 17, 1000] {
                    let ec = (c.update)(sc, std::ptr::null(), len);
                    let er = (r.update)(sr, std::ptr::null(), len);
                    assert_eq!(
                        ec, er,
                        "row186: LZ4_XXH64_update(state, NULL, {len}) C={ec} Rust={er}"
                    );
                    assert_eq!(
                        ec, XXH_ERROR,
                        "row186: LZ4_XXH64_update(state, NULL, {len}) must be XXH_ERROR(1), got {ec}"
                    );

                    let after_c = (c.digest)(sc);
                    let after_r = (r.digest)(sr);
                    assert_eq!(
                        after_c, before_c,
                        "row186: C state was modified by the rejected update (len={len})"
                    );
                    assert_eq!(
                        after_r, before_r,
                        "row186: Rust state was modified by the rejected update (len={len})"
                    );
                    assert_eq!(after_c, after_r, "row186: digests diverge after len={len}");
                    assert_eq!(after_c, oneshot, "row186: digest != one-shot after len={len}");
                }

                let tail = gen(&mut rng, shape, 41);
                let tp = tail.as_ptr() as *const c_void;
                assert_eq!((c.update)(sc, tp, tail.len()), XXH_OK, "row186: C update after reject");
                assert_eq!((r.update)(sr, tp, tail.len()), XXH_OK, "row186: Rust update after reject");
                let mut cat = prefix.clone();
                cat.extend_from_slice(&tail);
                let dc = (c.digest)(sc);
                let dr = (r.digest)(sr);
                assert_eq!(dc, dr, "row186: post-recovery digest mismatch");
                assert_eq!(
                    dc,
                    same64(&c, &r, &cat, seed, "row186 recovery"),
                    "row186: post-recovery digest != one-shot(prefix||tail)"
                );

                assert_eq!((c.free)(sc), (r.free)(sr), "row186: freeState mismatch");
            }
        }
    }
}

// ===========================================================================
// Row 187 — `XXH32_update` / `XXH64_update` with `input == NULL` when built
//           with `XXH_ACCEPT_NULL_INPUT_POINTER >= 1` (xxhash.c:456, :916)
//           would return `XXH_OK` (0), treating NULL as a zero-length input.
//
// NOT THIS BUILD'S CONFIG — covered as a configuration that cannot be reached.
// `c_src/src/xxhash.c` lines 65-72 read:
//
//     /*!XXH_ACCEPT_NULL_INPUT_POINTER :
//      * If input pointer is NULL, xxHash default behavior is to dereference it,
//      * triggering a segfault. When this macro is enabled, xxHash actively
//      * checks input for null pointer. It it is, result for null input pointers
//      * is the same as a null-length input. */
//     #ifndef XXH_ACCEPT_NULL_INPUT_POINTER   /* can be defined externally */
//     #  define XXH_ACCEPT_NULL_INPUT_POINTER 0
//     #endif
//
// and nothing in the build defines it externally, so the effective value is 0
// in BOTH libraries. The `return XXH_OK` arm is `#if`-ed out of the C object
// code entirely: there is no runtime input that can select it, and it cannot be
// re-enabled without rebuilding `c_src/` (forbidden here).
//
// So this test pins the ACTUAL configured behaviour — `XXH_ERROR (1)`,
// identically from C and Rust, for both `update` entry points — which is
// exactly what proves the Rust port compiled the same branch of that `#if`.
// ===========================================================================

#[test]
fn err_187_update_null_input_accept_null_input_pointer() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        let mut rng = Rng::new(0x0187_0000);

        // Property style over lengths (including the lengths that would matter
        // if NULL were accepted as a zero-length input) and seeds.
        let mut lens: Vec<usize> = (0usize..=33).collect();
        lens.extend_from_slice(&[63, 64, 65, 1000, 65536, usize::MAX / 2]);
        for _ in 0..8 {
            lens.push(rng.range(1, 100_000));
        }

        for &len in &lens {
            let seed32 = rng.next_u32();
            let seed64 = rng.next_u64();

            let (sc, sr) = primed32(&c32, &r32, &[], seed32);
            let base_c = (c32.digest)(sc);
            let base_r = (r32.digest)(sr);
            let ec = (c32.update)(sc, std::ptr::null(), len);
            let er = (r32.update)(sr, std::ptr::null(), len);
            assert_eq!(ec, er, "row187: XXH32_update(NULL,{len}) C={ec} Rust={er}");
            assert_eq!(
                ec, XXH_ERROR,
                "row187: with XXH_ACCEPT_NULL_INPUT_POINTER==0 the result must be \
                 XXH_ERROR(1), not XXH_OK(0); got {ec} for len={len}"
            );
            // If the `>= 1` variant had been compiled in, `XXH_OK` would have
            // been returned *and* the state would still be untouched; check the
            // state anyway so the two configurations differ only in the code.
            assert_eq!((c32.digest)(sc), base_c, "row187: C 32-bit state disturbed");
            assert_eq!((r32.digest)(sr), base_r, "row187: Rust 32-bit state disturbed");
            assert_eq!((c32.free)(sc), (r32.free)(sr));

            let (sc, sr) = primed64(&c64, &r64, &[], seed64);
            let base_c = (c64.digest)(sc);
            let base_r = (r64.digest)(sr);
            let ec = (c64.update)(sc, std::ptr::null(), len);
            let er = (r64.update)(sr, std::ptr::null(), len);
            assert_eq!(ec, er, "row187: XXH64_update(NULL,{len}) C={ec} Rust={er}");
            assert_eq!(
                ec, XXH_ERROR,
                "row187: with XXH_ACCEPT_NULL_INPUT_POINTER==0 the result must be \
                 XXH_ERROR(1), not XXH_OK(0); got {ec} for len={len}"
            );
            assert_eq!((c64.digest)(sc), base_c, "row187: C 64-bit state disturbed");
            assert_eq!((r64.digest)(sr), base_r, "row187: Rust 64-bit state disturbed");
            assert_eq!((c64.free)(sc), (r64.free)(sr));
        }
    }
}

// ===========================================================================
// Row 188 — one-shot `XXH32` / `XXH64` with `input == NULL`
//           (xxhash.c:359-364, :818-823).
//
// PARTIALLY UNREACHABLE — documented undefined behaviour for `len > 0`.
// With `XXH_ACCEPT_NULL_INPUT_POINTER == 0` the `if (p==NULL) { len=0; ... }`
// block in `XXH32_endian_align` / `XXH64_endian_align` is `#if`-ed out, so a
// NULL `input` with `len > 0` walks straight into `XXH_readLE32(p)` and
// dereferences address 0. xxhash.c:65-69 documents that as "triggering a
// segfault". Performing it here would abort the whole test binary (it is UB,
// not an error return), so it is NOT invoked.
//
// `len == 0` IS safe and in-contract even so: `len >= 16` is false, the
// `p + 4 <= bEnd` / `p < bEnd` residue loops immediately fail with
// `bEnd == p == NULL`, and no load ever happens. That case is forced for real
// below and must yield exactly the empty-input hash.
// ===========================================================================

#[test]
fn err_188_oneshot_null_input() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        let mut rng = Rng::new(0x0188_0000);

        // A real, non-NULL, zero-length buffer to compare against. Take it at
        // several alignments to show the result cannot depend on the pointer.
        let backing = [0xA5u8; 32];

        let mut seeds32: Vec<u32> = vec![0, 1, 0x9E37_79B1, 0xFFFF_FFFF, 0x8000_0000];
        for _ in 0..16 {
            seeds32.push(rng.next_u32());
        }
        for seed in seeds32 {
            let hc = (c32.xxh)(std::ptr::null(), 0, seed);
            let hr = (r32.xxh)(std::ptr::null(), 0, seed);
            assert_eq!(
                hc, hr,
                "row188: LZ4_XXH32(NULL, 0, {seed:#010x}) C={hc:#010x} Rust={hr:#010x}"
            );
            // each library's own hash of a real zero-length buffer
            for off in 0usize..8 {
                let p = backing.as_ptr().add(off) as *const c_void;
                let ec = (c32.xxh)(p, 0, seed);
                let er = (r32.xxh)(p, 0, seed);
                assert_eq!(ec, er, "row188: XXH32(valid,0) mismatch off={off}");
                assert_eq!(
                    hc, ec,
                    "row188: XXH32(NULL,0) != C XXH32(valid ptr,0) (seed={seed:#010x})"
                );
                assert_eq!(
                    hr, er,
                    "row188: XXH32(NULL,0) != Rust XXH32(valid ptr,0) (seed={seed:#010x})"
                );
            }
            // and against the empty slice through the shared helper
            assert_eq!(hc, same32(&c32, &r32, &backing[..0], seed, "row188 empty"));
        }

        let mut seeds64: Vec<u64> = vec![0, 1, 0x9E37_79B1, 0x9E37_79B1_85EB_CA87, u64::MAX];
        for _ in 0..16 {
            seeds64.push(rng.next_u64());
        }
        for seed in seeds64 {
            let hc = (c64.xxh)(std::ptr::null(), 0, seed);
            let hr = (r64.xxh)(std::ptr::null(), 0, seed);
            assert_eq!(
                hc, hr,
                "row188: LZ4_XXH64(NULL, 0, {seed:#018x}) C={hc:#018x} Rust={hr:#018x}"
            );
            for off in 0usize..8 {
                let p = backing.as_ptr().add(off) as *const c_void;
                let ec = (c64.xxh)(p, 0, seed);
                let er = (r64.xxh)(p, 0, seed);
                assert_eq!(ec, er, "row188: XXH64(valid,0) mismatch off={off}");
                assert_eq!(
                    hc, ec,
                    "row188: XXH64(NULL,0) != C XXH64(valid ptr,0) (seed={seed:#018x})"
                );
                assert_eq!(
                    hr, er,
                    "row188: XXH64(NULL,0) != Rust XXH64(valid ptr,0) (seed={seed:#018x})"
                );
            }
            assert_eq!(hc, same64(&c64, &r64, &backing[..0], seed, "row188 empty"));
        }
    }
}

// ===========================================================================
// Row 189 — `XXH32_createState`: `XXH_malloc(sizeof(XXH32_state_t))` returned
//           NULL (xxhash.c:422-425) → `NULL`.
//
// UNFORCEABLE. `XXH_malloc` is `#define XXH_malloc malloc` — plain libc
// `malloc` called from inside the shared object. There is no allocator hook,
// no injected failure and no size argument under test control (the size is a
// fixed `sizeof(XXH32_state_t)`, ~48 bytes, which malloc will not refuse), and
// exhausting the heap or interposing a failing `malloc` would break the C and
// Rust libraries asymmetrically (Rust's `#[no_mangle]` wrapper may allocate via
// the Rust global allocator) as well as the test harness itself. So the NULL
// return cannot be observed.
//
// Instead: pin the success path exactly — `createState` never returns NULL
// under normal conditions in EITHER library, distinct calls return distinct
// states, and the returned state is fully usable (reset -> update -> digest
// equals the one-shot) and freeable by its own library.
// ===========================================================================

#[test]
fn err_189_xxh32_createstate_malloc_failure() {
    unsafe {
        let (c, r) = api32();
        let mut rng = Rng::new(0x0189_0000);

        // Many allocations alive at once: distinct, non-NULL, non-aliasing.
        let mut live: Vec<(*mut c_void, *mut c_void)> = Vec::new();
        for i in 0..256 {
            let sc = (c.create)();
            let sr = (r.create)();
            assert!(!sc.is_null(), "row189: C LZ4_XXH32_createState returned NULL at #{i}");
            assert!(!sr.is_null(), "row189: Rust LZ4_XXH32_createState returned NULL at #{i}");
            assert!(
                live.iter().all(|&(a, b)| a != sc && b != sr),
                "row189: createState handed out an already-live pointer at #{i}"
            );
            live.push((sc, sr));
        }
        for (i, &(sc, sr)) in live.iter().enumerate() {
            // Every state is independently usable. NOTE: a state straight out of
            // createState is uninitialised, so it is reset before any digest.
            let seed = rng.next_u32();
            let dlen = rng.range(0, 300);
            let data = gen(&mut rng, ALL_SHAPES[i % ALL_SHAPES.len()], dlen);
            assert_eq!((c.reset)(sc, seed), (r.reset)(sr, seed), "row189: reset mismatch #{i}");
            let p = data.as_ptr() as *const c_void;
            assert_eq!(
                (c.update)(sc, p, data.len()),
                (r.update)(sr, p, data.len()),
                "row189: update mismatch #{i}"
            );
            let dc = (c.digest)(sc);
            let dr = (r.digest)(sr);
            assert_eq!(dc, dr, "row189: digest mismatch #{i}");
            assert_eq!(
                dc,
                same32(&c, &r, &data, seed, "row189"),
                "row189: fresh state's digest != one-shot (#{i}, len={})",
                data.len()
            );
        }
        for &(sc, sr) in &live {
            let (fc, fr) = ((c.free)(sc), (r.free)(sr));
            assert_eq!(fc, fr, "row189: freeState return mismatch");
            assert_eq!(fc, XXH_OK, "row189: freeState must return XXH_OK");
        }

        // Create/free churn: still never NULL after thousands of cycles.
        for i in 0..2000 {
            let sc = (c.create)();
            let sr = (r.create)();
            assert!(!sc.is_null(), "row189: C createState NULL in churn cycle {i}");
            assert!(!sr.is_null(), "row189: Rust createState NULL in churn cycle {i}");
            assert_eq!((c.free)(sc), (r.free)(sr));
        }
    }
}

// ===========================================================================
// Row 190 — `XXH64_createState`: same malloc-failure NULL return
//           (xxhash.c:883-886).
//
// UNFORCEABLE for exactly the reasons given for row 189. Success path pinned.
// ===========================================================================

#[test]
fn err_190_xxh64_createstate_malloc_failure() {
    unsafe {
        let (c, r) = api64();
        let mut rng = Rng::new(0x0190_0000);

        let mut live: Vec<(*mut c_void, *mut c_void)> = Vec::new();
        for i in 0..256 {
            let sc = (c.create)();
            let sr = (r.create)();
            assert!(!sc.is_null(), "row190: C LZ4_XXH64_createState returned NULL at #{i}");
            assert!(!sr.is_null(), "row190: Rust LZ4_XXH64_createState returned NULL at #{i}");
            assert!(
                live.iter().all(|&(a, b)| a != sc && b != sr),
                "row190: createState handed out an already-live pointer at #{i}"
            );
            live.push((sc, sr));
        }
        for (i, &(sc, sr)) in live.iter().enumerate() {
            let seed = rng.next_u64();
            let dlen = rng.range(0, 300);
            let data = gen(&mut rng, ALL_SHAPES[i % ALL_SHAPES.len()], dlen);
            assert_eq!((c.reset)(sc, seed), (r.reset)(sr, seed), "row190: reset mismatch #{i}");
            let p = data.as_ptr() as *const c_void;
            assert_eq!(
                (c.update)(sc, p, data.len()),
                (r.update)(sr, p, data.len()),
                "row190: update mismatch #{i}"
            );
            let dc = (c.digest)(sc);
            let dr = (r.digest)(sr);
            assert_eq!(dc, dr, "row190: digest mismatch #{i}");
            assert_eq!(
                dc,
                same64(&c, &r, &data, seed, "row190"),
                "row190: fresh state's digest != one-shot (#{i}, len={})",
                data.len()
            );
        }
        for &(sc, sr) in &live {
            let (fc, fr) = ((c.free)(sc), (r.free)(sr));
            assert_eq!(fc, fr, "row190: freeState return mismatch");
            assert_eq!(fc, XXH_OK, "row190: freeState must return XXH_OK");
        }

        for i in 0..2000 {
            let sc = (c.create)();
            let sr = (r.create)();
            assert!(!sc.is_null(), "row190: C createState NULL in churn cycle {i}");
            assert!(!sr.is_null(), "row190: Rust createState NULL in churn cycle {i}");
            assert_eq!((c.free)(sc), (r.free)(sr));
        }
    }
}

// ===========================================================================
// Row 191 — `XXH32_freeState(NULL)` / `XXH64_freeState(NULL)`
//           (xxhash.c:426-430, :887-891) → `XXH_OK` (0). free-on-NULL is
//           tolerated and never reports `XXH_ERROR`.
//
// FORCED FOR REAL: `XXH_free` is `free`, and `free(NULL)` is well defined, so
// passing NULL is completely safe. The functions return `XXH_OK`
// unconditionally.
// ===========================================================================

#[test]
fn err_191_freestate_null_returns_xxh_ok() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();

        // Repeat: freeState(NULL) must be idempotent and never latch an error.
        for i in 0..64 {
            let a = (c32.free)(std::ptr::null_mut());
            let b = (r32.free)(std::ptr::null_mut());
            assert_eq!(a, b, "row191: LZ4_XXH32_freeState(NULL) C={a} Rust={b} (call #{i})");
            assert_eq!(
                a, XXH_OK,
                "row191: LZ4_XXH32_freeState(NULL) must return XXH_OK(0), got {a}"
            );

            let a = (c64.free)(std::ptr::null_mut());
            let b = (r64.free)(std::ptr::null_mut());
            assert_eq!(a, b, "row191: LZ4_XXH64_freeState(NULL) C={a} Rust={b} (call #{i})");
            assert_eq!(
                a, XXH_OK,
                "row191: LZ4_XXH64_freeState(NULL) must return XXH_OK(0), got {a}"
            );
        }

        // The non-NULL path returns the same XXH_OK, so NULL is not
        // distinguishable by the return value.
        let sc = (c32.create)();
        let sr = (r32.create)();
        let a = (c32.free)(sc);
        let b = (r32.free)(sr);
        assert_eq!(a, b, "row191: XXH32_freeState(valid) C={a} Rust={b}");
        assert_eq!(a, XXH_OK, "row191: XXH32_freeState(valid) must be XXH_OK");
        let sc = (c64.create)();
        let sr = (r64.create)();
        let a = (c64.free)(sc);
        let b = (r64.free)(sr);
        assert_eq!(a, b, "row191: XXH64_freeState(valid) C={a} Rust={b}");
        assert_eq!(a, XXH_OK, "row191: XXH64_freeState(valid) must be XXH_OK");
    }
}

// ===========================================================================
// Row 192 — `XXH32_reset` / `XXH64_reset` with `statePtr == NULL`
//           (xxhash.c:437-450, :898-911).
//
// UNDEFINED BEHAVIOUR — NOT INVOKED. There is no NULL check anywhere in
// `reset`: the body fills a local `XXH32_state_t state` and then performs
// `memcpy(statePtr, &state, sizeof(state) - sizeof(state.reserved))`, which
// dereferences `statePtr` unconditionally. Calling `reset(NULL, seed)` would
// write ~44 bytes to address 0 and kill the test process (in both libraries),
// so it is deliberately not called.
//
// The auditable, reachable claim is the one the row itself makes: `reset` has
// NO error return — it is *always* `XXH_OK (0)`. That is pinned below across
// many seeds, plus the post-condition that a reset state digests to the
// empty-input hash for that seed (which is what proves the memcpy actually
// installed the seeded accumulators).
// ===========================================================================

#[test]
fn err_192_reset_null_state_no_check() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        let mut rng = Rng::new(0x0192_0000);
        let empty = [0u8; 1];
        let ep = empty.as_ptr() as *const c_void;

        let mut s32: Vec<u32> = vec![0, 1, 0x9E37_79B1, u32::MAX, 0x8000_0000, 2654435761];
        for _ in 0..64 {
            s32.push(rng.next_u32());
        }
        let sc = (c32.create)();
        let sr = (r32.create)();
        assert!(!sc.is_null() && !sr.is_null(), "row192: createState NULL");
        for &seed in &s32 {
            // repeated reset of the SAME state must also always be XXH_OK
            for round in 0..2 {
                let a = (c32.reset)(sc, seed);
                let b = (r32.reset)(sr, seed);
                assert_eq!(
                    a, b,
                    "row192: LZ4_XXH32_reset(state, {seed:#010x}) C={a} Rust={b} (round {round})"
                );
                assert_eq!(
                    a, XXH_OK,
                    "row192: LZ4_XXH32_reset must return XXH_OK(0), got {a} (seed={seed:#010x})"
                );
            }
            let dc = (c32.digest)(sc);
            let dr = (r32.digest)(sr);
            assert_eq!(dc, dr, "row192: digest after reset differs (seed={seed:#010x})");
            assert_eq!(
                dc,
                (c32.xxh)(ep, 0, seed),
                "row192: C reset+digest != XXH32(empty, seed={seed:#010x})"
            );
            assert_eq!(
                dr,
                (r32.xxh)(ep, 0, seed),
                "row192: Rust reset+digest != XXH32(empty, seed={seed:#010x})"
            );
            // reset must also wipe previously absorbed data
            let jlen = rng.range(1, 200);
            let junk = gen(&mut rng, Shape::Incompressible, jlen);
            let jp = junk.as_ptr() as *const c_void;
            assert_eq!((c32.update)(sc, jp, junk.len()), (r32.update)(sr, jp, junk.len()));
            assert_eq!((c32.reset)(sc, seed), XXH_OK, "row192: C re-reset not XXH_OK");
            assert_eq!((r32.reset)(sr, seed), XXH_OK, "row192: Rust re-reset not XXH_OK");
            assert_eq!((c32.digest)(sc), dc, "row192: C reset did not clear the state");
            assert_eq!((r32.digest)(sr), dr, "row192: Rust reset did not clear the state");
        }
        assert_eq!((c32.free)(sc), (r32.free)(sr));

        let mut s64: Vec<u64> = vec![
            0,
            1,
            0x9E37_79B1,
            0x9E37_79B1_85EB_CA87,
            u64::MAX,
            0x8000_0000_0000_0000,
            u32::MAX as u64,
        ];
        for _ in 0..64 {
            s64.push(rng.next_u64());
        }
        let sc = (c64.create)();
        let sr = (r64.create)();
        assert!(!sc.is_null() && !sr.is_null(), "row192: createState NULL");
        for &seed in &s64 {
            for round in 0..2 {
                let a = (c64.reset)(sc, seed);
                let b = (r64.reset)(sr, seed);
                assert_eq!(
                    a, b,
                    "row192: LZ4_XXH64_reset(state, {seed:#018x}) C={a} Rust={b} (round {round})"
                );
                assert_eq!(
                    a, XXH_OK,
                    "row192: LZ4_XXH64_reset must return XXH_OK(0), got {a} (seed={seed:#018x})"
                );
            }
            let dc = (c64.digest)(sc);
            let dr = (r64.digest)(sr);
            assert_eq!(dc, dr, "row192: digest after reset differs (seed={seed:#018x})");
            assert_eq!(
                dc,
                (c64.xxh)(ep, 0, seed),
                "row192: C reset+digest != XXH64(empty, seed={seed:#018x})"
            );
            assert_eq!(
                dr,
                (r64.xxh)(ep, 0, seed),
                "row192: Rust reset+digest != XXH64(empty, seed={seed:#018x})"
            );
            let jlen = rng.range(1, 200);
            let junk = gen(&mut rng, Shape::TextLike, jlen);
            let jp = junk.as_ptr() as *const c_void;
            assert_eq!((c64.update)(sc, jp, junk.len()), (r64.update)(sr, jp, junk.len()));
            assert_eq!((c64.reset)(sc, seed), XXH_OK, "row192: C re-reset not XXH_OK");
            assert_eq!((r64.reset)(sr, seed), XXH_OK, "row192: Rust re-reset not XXH_OK");
            assert_eq!((c64.digest)(sc), dc, "row192: C reset did not clear the state");
            assert_eq!((r64.digest)(sr), dr, "row192: Rust reset did not clear the state");
        }
        assert_eq!((c64.free)(sc), (r64.free)(sr));
    }
}

// ===========================================================================
// Row 193 — `XXH32_digest` / `XXH64_digest` with `state_in == NULL` or an
//           unreset state (xxhash.c:545-555, :1005-1014): there is NO
//           validation at all, and no error path — the functions return a hash
//           value (`XXH32_hash_t` / `XXH64_hash_t`), never an error code.
//
// The two illegal inputs the row names are NOT invoked:
//   * `digest(NULL)` immediately reads `state->total_len` / `state->v1..v4`
//     from address 0 → segfault. Not called.
//   * a raw `createState()` result that has never been `reset` is
//     uninitialised heap memory; digesting it reads indeterminate bytes, so C
//     and Rust would legitimately disagree and the comparison would be
//     meaningless (it is also UB). Not called.
//
// What IS asserted (identically in both libraries, property-style):
//   * a freshly-`reset` state digests to a well-defined value == the
//     empty-input one-shot for that seed;
//   * a reset-but-not-updated state keeps digesting to that same value;
//   * a state already digested several times keeps returning the same hash —
//     `digest` is a pure observer and never latches an error;
//   * digest interleaved with further updates always equals the one-shot of
//     everything consumed so far. There is no return value that could encode
//     "error", which is exactly the row's claim.
// ===========================================================================

#[test]
fn err_193_digest_has_no_error_path() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        let mut rng = Rng::new(0x0193_0000);
        let empty = [0u8; 1];
        let ep = empty.as_ptr() as *const c_void;

        for &shape in &ALL_SHAPES {
            for trial in 0..6 {
                let seed32 = rng.next_u32();
                let seed64 = rng.next_u64();
                let total = rng.range(0, 400);
                let data = gen(&mut rng, shape, total);

                // ---- 32-bit ----
                let sc = (c32.create)();
                let sr = (r32.create)();
                assert!(!sc.is_null() && !sr.is_null(), "row193: createState NULL");
                // NOTE: reset FIRST — never digest a never-reset createState result.
                assert_eq!((c32.reset)(sc, seed32), (r32.reset)(sr, seed32));

                // (a) freshly reset
                let fresh_c = (c32.digest)(sc);
                let fresh_r = (r32.digest)(sr);
                assert_eq!(
                    fresh_c, fresh_r,
                    "row193: digest of a freshly reset XXH32 state differs \
                     (seed={seed32:#010x}) C={fresh_c:#010x} Rust={fresh_r:#010x}"
                );
                assert_eq!(
                    fresh_c,
                    (c32.xxh)(ep, 0, seed32),
                    "row193: C fresh digest != XXH32(empty, seed)"
                );
                assert_eq!(
                    fresh_r,
                    (r32.xxh)(ep, 0, seed32),
                    "row193: Rust fresh digest != XXH32(empty, seed)"
                );

                // (b) reset but not updated, digested repeatedly: stable
                for k in 0..7 {
                    assert_eq!(
                        (c32.digest)(sc), fresh_c,
                        "row193: C XXH32 digest of an un-updated state changed at call {k}"
                    );
                    assert_eq!(
                        (r32.digest)(sr), fresh_r,
                        "row193: Rust XXH32 digest of an un-updated state changed at call {k}"
                    );
                }

                // (c) already digested several times, then updated: digest
                //     still tracks the consumed prefix exactly.
                let mut off = 0usize;
                while off < total {
                    let n = rng.range(1, (total - off).min(70));
                    let p = data[off..].as_ptr() as *const c_void;
                    assert_eq!(
                        (c32.update)(sc, p, n),
                        (r32.update)(sr, p, n),
                        "row193: XXH32_update return mismatch at off={off}"
                    );
                    off += n;
                    // digest three times in a row: no error, no drift
                    let a = (c32.digest)(sc);
                    let b = (c32.digest)(sc);
                    let x = (r32.digest)(sr);
                    let y = (r32.digest)(sr);
                    assert_eq!(a, b, "row193: C XXH32 digest not idempotent at off={off}");
                    assert_eq!(x, y, "row193: Rust XXH32 digest not idempotent at off={off}");
                    assert_eq!(a, x, "row193: XXH32 digest mismatch at off={off}");
                    assert_eq!(
                        a,
                        same32(&c32, &r32, &data[..off], seed32, "row193"),
                        "row193: XXH32 digest != one-shot at off={off} (shape={shape:?} trial={trial})"
                    );
                }
                assert_eq!((c32.free)(sc), (r32.free)(sr));

                // ---- 64-bit ----
                let sc = (c64.create)();
                let sr = (r64.create)();
                assert!(!sc.is_null() && !sr.is_null(), "row193: createState NULL");
                assert_eq!((c64.reset)(sc, seed64), (r64.reset)(sr, seed64));

                let fresh_c = (c64.digest)(sc);
                let fresh_r = (r64.digest)(sr);
                assert_eq!(
                    fresh_c, fresh_r,
                    "row193: digest of a freshly reset XXH64 state differs \
                     (seed={seed64:#018x}) C={fresh_c:#018x} Rust={fresh_r:#018x}"
                );
                assert_eq!(
                    fresh_c,
                    (c64.xxh)(ep, 0, seed64),
                    "row193: C fresh digest != XXH64(empty, seed)"
                );
                assert_eq!(
                    fresh_r,
                    (r64.xxh)(ep, 0, seed64),
                    "row193: Rust fresh digest != XXH64(empty, seed)"
                );
                for k in 0..7 {
                    assert_eq!(
                        (c64.digest)(sc), fresh_c,
                        "row193: C XXH64 digest of an un-updated state changed at call {k}"
                    );
                    assert_eq!(
                        (r64.digest)(sr), fresh_r,
                        "row193: Rust XXH64 digest of an un-updated state changed at call {k}"
                    );
                }
                let mut off = 0usize;
                while off < total {
                    let n = rng.range(1, (total - off).min(70));
                    let p = data[off..].as_ptr() as *const c_void;
                    assert_eq!(
                        (c64.update)(sc, p, n),
                        (r64.update)(sr, p, n),
                        "row193: XXH64_update return mismatch at off={off}"
                    );
                    off += n;
                    let a = (c64.digest)(sc);
                    let b = (c64.digest)(sc);
                    let x = (r64.digest)(sr);
                    let y = (r64.digest)(sr);
                    assert_eq!(a, b, "row193: C XXH64 digest not idempotent at off={off}");
                    assert_eq!(x, y, "row193: Rust XXH64 digest not idempotent at off={off}");
                    assert_eq!(a, x, "row193: XXH64 digest mismatch at off={off}");
                    assert_eq!(
                        a,
                        same64(&c64, &r64, &data[..off], seed64, "row193"),
                        "row193: XXH64 digest != one-shot at off={off} (shape={shape:?} trial={trial})"
                    );
                }
                assert_eq!((c64.free)(sc), (r64.free)(sr));
            }
        }
    }
}

// ===========================================================================
// Row 194 — `XXH32_copyState` / `XXH64_copyState` with a NULL `dstState` or
//           `srcState` (xxhash.c:432-435, :893-896): the body is a plain
//           `memcpy(dstState, srcState, sizeof(*dstState))` with no checks;
//           the function returns `void`, so a NULL argument is undefined
//           behaviour with no observable error.
//
// NOT INVOKED. `memcpy(NULL, src, 48)` / `memcpy(dst, NULL, 48)` would fault
// (and is UB even if it did not), and there is no return value to inspect, so
// there is literally nothing to compare. `void` also means the two libraries
// cannot disagree on a return code — the only observable behaviour is the
// bytes written into `dstState`, which is what is checked here.
//
// Property-style, over many random chunkings and shapes, and ALWAYS between two
// states of the SAME library:
//   * the copy digests exactly like its source;
//   * the copy is fully independent — diverging updates on the original and on
//     the copy leave both digests correct (== the one-shot of the respective
//     concatenations) at EVERY step, in C and in Rust;
//   * copy-of-a-copy chains behave the same;
//   * copying INTO a state that already held other data overwrites it wholly.
// ===========================================================================

#[test]
fn err_194_copystate_null_memcpy() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        let mut rng = Rng::new(0x0194_0000);

        for &shape in &ALL_SHAPES {
            for trial in 0..5 {
                let seed32 = rng.next_u32();
                let seed64 = rng.next_u64();
                let head_len = rng.range(0, 300);
                let head = gen(&mut rng, shape, head_len);
                // two different continuations, fed to the original and the copy
                let ta_len = rng.range(0, 200);
                let tail_a = gen(&mut rng, shape, ta_len);
                let tb_len = rng.range(0, 200);
                let tail_b = gen(&mut rng, Shape::Periodic, tb_len);

                // ---------------- 32-bit ----------------
                let (oc, or) = primed32(&c32, &r32, &head, seed32);
                // the destination deliberately already holds unrelated data, to
                // prove the memcpy overwrites the whole state
                let dirt_len = rng.range(1, 120);
                let dirt = gen(&mut rng, Shape::Degenerate, dirt_len);
                let (kc, kr) = primed32(&c32, &r32, &dirt, seed32 ^ 0xDEAD_BEEF);
                // SAME-LIBRARY copies only: C <- C, Rust <- Rust.
                (c32.copy)(kc, oc as *const c_void);
                (r32.copy)(kr, or as *const c_void);

                let base = same32(&c32, &r32, &head, seed32, "row194 head");
                assert_eq!((c32.digest)(oc), base, "row194: C original digest != one-shot");
                assert_eq!((r32.digest)(or), base, "row194: Rust original digest != one-shot");
                assert_eq!(
                    (c32.digest)(kc), base,
                    "row194: C copy digest != source digest (trial={trial})"
                );
                assert_eq!(
                    (r32.digest)(kr), base,
                    "row194: Rust copy digest != source digest (trial={trial})"
                );

                // diverging, randomly chunked updates on original and copy
                let mut ia = 0usize;
                let mut ib = 0usize;
                let mut step = 0usize;
                while ia < tail_a.len() || ib < tail_b.len() {
                    if ia < tail_a.len() {
                        let n = rng.range(1, (tail_a.len() - ia).min(37));
                        let p = tail_a[ia..].as_ptr() as *const c_void;
                        assert_eq!(
                            (c32.update)(oc, p, n),
                            (r32.update)(or, p, n),
                            "row194: original update mismatch (step {step})"
                        );
                        ia += n;
                    }
                    if ib < tail_b.len() {
                        let n = rng.range(1, (tail_b.len() - ib).min(37));
                        let p = tail_b[ib..].as_ptr() as *const c_void;
                        assert_eq!(
                            (c32.update)(kc, p, n),
                            (r32.update)(kr, p, n),
                            "row194: copy update mismatch (step {step})"
                        );
                        ib += n;
                    }
                    step += 1;

                    let mut cat_a = head.clone();
                    cat_a.extend_from_slice(&tail_a[..ia]);
                    let mut cat_b = head.clone();
                    cat_b.extend_from_slice(&tail_b[..ib]);
                    let want_a = same32(&c32, &r32, &cat_a, seed32, "row194 orig");
                    let want_b = same32(&c32, &r32, &cat_b, seed32, "row194 copy");

                    let (ga, gb) = ((c32.digest)(oc), (c32.digest)(kc));
                    let (ha, hb) = ((r32.digest)(or), (r32.digest)(kr));
                    assert_eq!(ga, ha, "row194: original digest diverged at step {step}");
                    assert_eq!(gb, hb, "row194: copy digest diverged at step {step}");
                    assert_eq!(
                        ga, want_a,
                        "row194: original != one-shot at step {step} (ia={ia})"
                    );
                    assert_eq!(gb, want_b, "row194: copy != one-shot at step {step} (ib={ib})");
                }

                // copy-of-a-copy chain
                let mut chain: Vec<(*mut c_void, *mut c_void)> = Vec::new();
                let (mut pc, mut pr) = (kc, kr);
                let want = (c32.digest)(kc);
                for link in 0..4 {
                    let nc = (c32.create)();
                    let nr = (r32.create)();
                    assert!(!nc.is_null() && !nr.is_null(), "row194: createState NULL");
                    (c32.copy)(nc, pc as *const c_void);
                    (r32.copy)(nr, pr as *const c_void);
                    let (a, b) = ((c32.digest)(nc), (r32.digest)(nr));
                    assert_eq!(a, b, "row194: chained copy digest mismatch at link {link}");
                    assert_eq!(a, want, "row194: chained copy lost the value at link {link}");
                    chain.push((nc, nr));
                    pc = nc;
                    pr = nr;
                }
                for (a, b) in chain {
                    assert_eq!((c32.free)(a), (r32.free)(b));
                }
                assert_eq!((c32.free)(oc), (r32.free)(or));
                assert_eq!((c32.free)(kc), (r32.free)(kr));

                // ---------------- 64-bit ----------------
                let (oc, or) = primed64(&c64, &r64, &head, seed64);
                let (kc, kr) = primed64(&c64, &r64, &dirt, seed64 ^ 0x1234_5678_9ABC_DEF0);
                (c64.copy)(kc, oc as *const c_void);
                (r64.copy)(kr, or as *const c_void);

                let base = same64(&c64, &r64, &head, seed64, "row194 head64");
                assert_eq!((c64.digest)(oc), base, "row194: C XXH64 original != one-shot");
                assert_eq!((r64.digest)(or), base, "row194: Rust XXH64 original != one-shot");
                assert_eq!((c64.digest)(kc), base, "row194: C XXH64 copy != source");
                assert_eq!((r64.digest)(kr), base, "row194: Rust XXH64 copy != source");

                let mut ia = 0usize;
                let mut ib = 0usize;
                let mut step = 0usize;
                while ia < tail_a.len() || ib < tail_b.len() {
                    if ia < tail_a.len() {
                        let n = rng.range(1, (tail_a.len() - ia).min(53));
                        let p = tail_a[ia..].as_ptr() as *const c_void;
                        assert_eq!(
                            (c64.update)(oc, p, n),
                            (r64.update)(or, p, n),
                            "row194: XXH64 original update mismatch (step {step})"
                        );
                        ia += n;
                    }
                    if ib < tail_b.len() {
                        let n = rng.range(1, (tail_b.len() - ib).min(53));
                        let p = tail_b[ib..].as_ptr() as *const c_void;
                        assert_eq!(
                            (c64.update)(kc, p, n),
                            (r64.update)(kr, p, n),
                            "row194: XXH64 copy update mismatch (step {step})"
                        );
                        ib += n;
                    }
                    step += 1;

                    let mut cat_a = head.clone();
                    cat_a.extend_from_slice(&tail_a[..ia]);
                    let mut cat_b = head.clone();
                    cat_b.extend_from_slice(&tail_b[..ib]);
                    let want_a = same64(&c64, &r64, &cat_a, seed64, "row194 orig64");
                    let want_b = same64(&c64, &r64, &cat_b, seed64, "row194 copy64");

                    let (ga, gb) = ((c64.digest)(oc), (c64.digest)(kc));
                    let (ha, hb) = ((r64.digest)(or), (r64.digest)(kr));
                    assert_eq!(ga, ha, "row194: XXH64 original digest diverged at step {step}");
                    assert_eq!(gb, hb, "row194: XXH64 copy digest diverged at step {step}");
                    assert_eq!(ga, want_a, "row194: XXH64 original != one-shot at step {step}");
                    assert_eq!(gb, want_b, "row194: XXH64 copy != one-shot at step {step}");
                }
                assert_eq!((c64.free)(oc), (r64.free)(or));
                assert_eq!((c64.free)(kc), (r64.free)(kr));
            }
        }
    }
}

// ===========================================================================
// Row 195 — the `assert(0)` immediately past the `switch (len & 15)` in
//           `XXH32_finalize` and past `switch (len & 31)` in `XXH64_finalize`
//           (xxhash.c:346-347, :806-807), commented "reaching this point is
//           deemed impossible".
//
// UNREACHABLE BY CONSTRUCTION. The switch in `XXH32_finalize` enumerates every
// value of `len & 15` (0..=15) and each case ends in `return`; likewise
// `XXH64_finalize` enumerates every value of `len & 31` (0..=31). Since the
// scrutinee is a bitwise-AND with 15 / 31 it is *arithmetically* impossible for
// it to be outside those ranges, so no input can reach the `assert(0)` and the
// release build's trailing `return h32;` is dead code. It cannot be forced.
//
// What is asserted instead is the property the assertion guards: that EVERY
// residue class is actually handled, and handled identically by both
// libraries. A mistranslated switch chain (a missing case, a fallthrough, a
// wrong `PROCESS`-step count) would show up here as a hash mismatch rather than
// as a silent hit on the impossible branch. Swept over every `len & 15`
// residue (0..=15) and every `len & 31` residue (0..=31), for several base
// lengths (so the residue is reached from 0, 1, 2, ... full stripes), several
// `Shape`s and several seeds — including via the streaming API, where
// `finalize` runs over the internal 16/32-byte buffer instead of over the
// caller's memory.
// ===========================================================================

#[test]
fn err_195_finalize_unreachable_assert() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        let mut rng = Rng::new(0x0195_0000);

        // Base lengths chosen so that (base + residue) exercises 0, 1, 2, 3, 8
        // and 64 complete stripes before the residue tail.
        let bases32 = [0usize, 16, 32, 48, 128, 1024];
        let bases64 = [0usize, 32, 64, 96, 256, 2048];
        let shapes = ALL_SHAPES;
        let seeds32 = [0u32, 1, 0x9E37_79B1, u32::MAX, 0xDEAD_BEEF];
        let seeds64 = [0u64, 1, 0x9E37_79B1_85EB_CA87, u64::MAX, 0x0BAD_F00D_DEAD_BEEF];

        // ---- XXH32: every `len & 15` residue class, one-shot ----
        let mut seen32 = [0usize; 16];
        for &shape in &shapes {
            let big = gen(&mut rng, shape, 1024 + 32);
            for &base in &bases32 {
                for residue in 0usize..16 {
                    let len = base + residue;
                    assert_eq!(len & 15, residue, "row195: bad test construction");
                    seen32[residue] += 1;
                    for &seed in &seeds32 {
                        same32(
                            &c32,
                            &r32,
                            &big[..len],
                            seed,
                            &format!("row195 xxh32 residue={residue} base={base} {shape:?}"),
                        );
                    }
                }
            }
        }
        assert!(
            seen32.iter().all(|&n| n >= bases32.len()),
            "row195: not every `len & 15` residue class was covered: {seen32:?}"
        );

        // ---- XXH64: every `len & 31` residue class, one-shot ----
        let mut seen64 = [0usize; 32];
        for &shape in &shapes {
            let big = gen(&mut rng, shape, 2048 + 64);
            for &base in &bases64 {
                for residue in 0usize..32 {
                    let len = base + residue;
                    assert_eq!(len & 31, residue, "row195: bad test construction");
                    seen64[residue] += 1;
                    for &seed in &seeds64 {
                        same64(
                            &c64,
                            &r64,
                            &big[..len],
                            seed,
                            &format!("row195 xxh64 residue={residue} base={base} {shape:?}"),
                        );
                    }
                }
            }
        }
        assert!(
            seen64.iter().all(|&n| n >= bases64.len()),
            "row195: not every `len & 31` residue class was covered: {seen64:?}"
        );

        // ---- the same residue classes reached through the streaming API, so
        //      `finalize` walks the state's internal `mem32`/`mem64` buffer ----
        for residue in 0usize..16 {
            let len = 64 + residue;
            let data = gen(&mut rng, Shape::Incompressible, len);
            let seed = rng.next_u32();
            let want = same32(&c32, &r32, &data, seed, "row195 stream32 oneshot");
            // feed one byte at a time so the residue is assembled inside the state
            let (sc, sr) = primed32(&c32, &r32, &[], seed);
            for i in 0..len {
                let p = data[i..].as_ptr() as *const c_void;
                assert_eq!((c32.update)(sc, p, 1), (r32.update)(sr, p, 1));
            }
            let (a, b) = ((c32.digest)(sc), (r32.digest)(sr));
            assert_eq!(a, b, "row195: streamed XXH32 digest mismatch (residue={residue})");
            assert_eq!(a, want, "row195: streamed XXH32 digest != one-shot (residue={residue})");
            assert_eq!((c32.free)(sc), (r32.free)(sr));
        }
        for residue in 0usize..32 {
            let len = 128 + residue;
            let data = gen(&mut rng, Shape::Incompressible, len);
            let seed = rng.next_u64();
            let want = same64(&c64, &r64, &data, seed, "row195 stream64 oneshot");
            let (sc, sr) = primed64(&c64, &r64, &[], seed);
            for i in 0..len {
                let p = data[i..].as_ptr() as *const c_void;
                assert_eq!((c64.update)(sc, p, 1), (r64.update)(sr, p, 1));
            }
            let (a, b) = ((c64.digest)(sc), (r64.digest)(sr));
            assert_eq!(a, b, "row195: streamed XXH64 digest mismatch (residue={residue})");
            assert_eq!(a, want, "row195: streamed XXH64 digest != one-shot (residue={residue})");
            assert_eq!((c64.free)(sc), (r64.free)(sr));
        }
    }
}

// ===========================================================================
// Row 196 — `XXH_STATIC_ASSERT(sizeof(XXH32_canonical_t) == sizeof(XXH32_hash_t))`
//           in `XXH32_canonicalFromHash` (xxhash.c:567) and the 64-bit
//           equivalent (xxhash.c:1020).
//
// COMPILE-TIME ONLY — CANNOT FAIL AT RUNTIME. `XXH_STATIC_ASSERT(c)` expands to
// `{ enum { XXH_sa = 1/(int)(!!(c)) }; }`, i.e. a division by zero in an enum
// initializer, which is a *translation* error. Both `liblz4.so` files exist and
// loaded, so both static assertions already held; there is no input that can
// make them fail and nothing to compare between C and Rust.
//
// What is asserted instead is the runtime invariant the static assertion exists
// to protect — that `canonicalFromHash`'s `memcpy(dst, &hash, sizeof(*dst))`
// moves exactly the whole hash and nothing more:
//   * exactly 4 (resp. 8) bytes are written: the sentinel bytes on both sides
//     of the destination inside an oversized buffer are untouched;
//   * the written bytes are big-endian ("human-readable write convention");
//   * the bytes are byte-identical between C and Rust;
//   * `hashFromCanonical` round-trips the value, and also agrees between the
//     two libraries on arbitrary (not canonically produced) input bytes.
// Property style: 0, all-ones and many random values.
// ===========================================================================

#[test]
fn err_196_canonical_static_assert() {
    unsafe {
        let (c32, r32) = api32();
        let (c64, r64) = api64();
        let mut rng = Rng::new(0x0196_0000);

        const PAD: usize = 16; // sentinel bytes before and after the canonical
        const FILL: u8 = 0x5A;

        // ---------------- 32-bit: exactly 4 bytes ----------------
        let mut vals32: Vec<u32> = vec![0, u32::MAX, 1, 0xFF, 0xFF00, 0x00FF_0000, 0x8000_0000];
        for _ in 0..600 {
            vals32.push(rng.next_u32());
        }
        for v in vals32 {
            let mut bc = [FILL; PAD + 4 + PAD];
            let mut br = [FILL; PAD + 4 + PAD];
            (c32.canon)(bc[PAD..].as_mut_ptr() as *mut c_void, v);
            (r32.canon)(br[PAD..].as_mut_ptr() as *mut c_void, v);

            // exactly 4 bytes written: both sentinel regions intact
            assert!(
                bc[..PAD].iter().all(|&x| x == FILL) && bc[PAD + 4..].iter().all(|&x| x == FILL),
                "row196: C LZ4_XXH32_canonicalFromHash wrote outside its 4 bytes for {v:#010x}: {bc:02x?}"
            );
            assert!(
                br[..PAD].iter().all(|&x| x == FILL) && br[PAD + 4..].iter().all(|&x| x == FILL),
                "row196: Rust LZ4_XXH32_canonicalFromHash wrote outside its 4 bytes for {v:#010x}: {br:02x?}"
            );

            // identical, and big-endian
            assert_eq!(
                &bc[PAD..PAD + 4],
                &br[PAD..PAD + 4],
                "row196: canonical bytes differ for {v:#010x}: C={:02x?} Rust={:02x?}",
                &bc[PAD..PAD + 4],
                &br[PAD..PAD + 4]
            );
            assert_eq!(
                &bc[PAD..PAD + 4],
                &v.to_be_bytes()[..],
                "row196: XXH32 canonical is not big-endian for {v:#010x}"
            );

            // round-trip, through both libraries
            let hc = (c32.from_canon)(bc[PAD..].as_ptr() as *const c_void);
            let hr = (r32.from_canon)(bc[PAD..].as_ptr() as *const c_void);
            assert_eq!(hc, hr, "row196: XXH32_hashFromCanonical mismatch for {v:#010x}");
            assert_eq!(hc, v, "row196: XXH32 canonical round-trip changed {v:#010x} into {hc:#010x}");
        }
        // arbitrary bytes, not produced by canonicalFromHash
        for _ in 0..300 {
            let b = [rng.byte(), rng.byte(), rng.byte(), rng.byte()];
            let hc = (c32.from_canon)(b.as_ptr() as *const c_void);
            let hr = (r32.from_canon)(b.as_ptr() as *const c_void);
            assert_eq!(hc, hr, "row196: XXH32_hashFromCanonical mismatch on {b:02x?}");
            assert_eq!(
                hc,
                u32::from_be_bytes(b),
                "row196: XXH32_hashFromCanonical is not a big-endian read"
            );
        }

        // ---------------- 64-bit: exactly 8 bytes ----------------
        let mut vals64: Vec<u64> = vec![
            0,
            u64::MAX,
            1,
            0xFF,
            0xFF00_0000_0000_0000,
            0x8000_0000_0000_0000,
            u32::MAX as u64,
        ];
        for _ in 0..600 {
            vals64.push(rng.next_u64());
        }
        for v in vals64 {
            let mut bc = [FILL; PAD + 8 + PAD];
            let mut br = [FILL; PAD + 8 + PAD];
            (c64.canon)(bc[PAD..].as_mut_ptr() as *mut c_void, v);
            (r64.canon)(br[PAD..].as_mut_ptr() as *mut c_void, v);

            assert!(
                bc[..PAD].iter().all(|&x| x == FILL) && bc[PAD + 8..].iter().all(|&x| x == FILL),
                "row196: C LZ4_XXH64_canonicalFromHash wrote outside its 8 bytes for {v:#018x}: {bc:02x?}"
            );
            assert!(
                br[..PAD].iter().all(|&x| x == FILL) && br[PAD + 8..].iter().all(|&x| x == FILL),
                "row196: Rust LZ4_XXH64_canonicalFromHash wrote outside its 8 bytes for {v:#018x}: {br:02x?}"
            );

            assert_eq!(
                &bc[PAD..PAD + 8],
                &br[PAD..PAD + 8],
                "row196: canonical bytes differ for {v:#018x}: C={:02x?} Rust={:02x?}",
                &bc[PAD..PAD + 8],
                &br[PAD..PAD + 8]
            );
            assert_eq!(
                &bc[PAD..PAD + 8],
                &v.to_be_bytes()[..],
                "row196: XXH64 canonical is not big-endian for {v:#018x}"
            );

            let hc = (c64.from_canon)(bc[PAD..].as_ptr() as *const c_void);
            let hr = (r64.from_canon)(bc[PAD..].as_ptr() as *const c_void);
            assert_eq!(hc, hr, "row196: XXH64_hashFromCanonical mismatch for {v:#018x}");
            assert_eq!(hc, v, "row196: XXH64 canonical round-trip changed {v:#018x} into {hc:#018x}");
        }
        for _ in 0..300 {
            let mut b = [0u8; 8];
            for x in b.iter_mut() {
                *x = rng.byte();
            }
            let hc = (c64.from_canon)(b.as_ptr() as *const c_void);
            let hr = (r64.from_canon)(b.as_ptr() as *const c_void);
            assert_eq!(hc, hr, "row196: XXH64_hashFromCanonical mismatch on {b:02x?}");
            assert_eq!(
                hc,
                u64::from_be_bytes(b),
                "row196: XXH64_hashFromCanonical is not a big-endian read"
            );
        }

        // Real hashes, not just synthetic values: canonicalise an actual digest
        // and round-trip it.
        for len in [0usize, 1, 15, 16, 17, 100, 1000] {
            let data = gen(&mut rng, Shape::TextLike, len);
            let seed32 = rng.next_u32();
            let h = same32(&c32, &r32, &data, seed32, "row196 real32");
            let mut bc = [FILL; 4];
            let mut br = [FILL; 4];
            (c32.canon)(bc.as_mut_ptr() as *mut c_void, h);
            (r32.canon)(br.as_mut_ptr() as *mut c_void, h);
            assert_eq!(bc, br, "row196: canonical of a real XXH32 digest differs");
            assert_eq!(bc, h.to_be_bytes(), "row196: real XXH32 canonical not big-endian");
            assert_eq!((c32.from_canon)(bc.as_ptr() as *const c_void), h);
            assert_eq!((r32.from_canon)(br.as_ptr() as *const c_void), h);

            let seed64 = rng.next_u64();
            let h = same64(&c64, &r64, &data, seed64, "row196 real64");
            let mut bc = [FILL; 8];
            let mut br = [FILL; 8];
            (c64.canon)(bc.as_mut_ptr() as *mut c_void, h);
            (r64.canon)(br.as_mut_ptr() as *mut c_void, h);
            assert_eq!(bc, br, "row196: canonical of a real XXH64 digest differs");
            assert_eq!(bc, h.to_be_bytes(), "row196: real XXH64 canonical not big-endian");
            assert_eq!((c64.from_canon)(bc.as_ptr() as *const c_void), h);
            assert_eq!((r64.from_canon)(br.as_ptr() as *const c_void), h);
        }
    }
}

// ===========================================================================
// Row 197 — `LZ4F_STATIC_ASSERT(sizeof(ptrdiff_t) >= sizeof(size_t))` inside
//           `LZ4F_returnErrorCode` (lz4frame.c:313-314), commented
//           "A compilation error here means sizeof(ptrdiff_t) is not large
//           enough".
//
// COMPILE-TIME ONLY — CANNOT FAIL AT RUNTIME, for the same reason as row 196:
// `LZ4F_STATIC_ASSERT` is a `1/(int)(!!(c))` enum initializer, so a violation
// is a translation error. On this LP64 target `sizeof(ptrdiff_t) ==
// sizeof(size_t) == 8`, both libraries compiled, so the assertion held in both.
//
// What is asserted instead is the runtime invariant the static assertion
// protects: that the `(LZ4F_errorCode_t)-(ptrdiff_t)code` round-trip
// `LZ4F_returnErrorCode` performs is exactly inverted by `LZ4F_getErrorCode`,
// and that `LZ4F_isError` / `LZ4F_getErrorName` classify the result the same
// way in both libraries — over the whole `[0 .. LZ4F_ERROR_maxCode]` boundary
// and the saturating extremes. `LZ4F_ERROR_maxCode == 24`, so `LZ4F_isError(x)`
// is `x > (size_t)-24`, i.e. true exactly on `err(1) ..= err(23)`.
// `LZ4F_getErrorName` is compared by C STRING CONTENT via `CStr`, never by
// pointer (the two libraries have separate string tables).
// ===========================================================================

#[test]
fn err_197_lz4frame_ptrdiff_static_assert() {
    unsafe {
        let l = libs();
        let (isc, isr) = pair!(l, FnIsErr, "LZ4F_isError");
        let (nmc, nmr) = pair!(l, FnErrName, "LZ4F_getErrorName");
        let (cdc, cdr) = pair!(l, FnErrCode, "LZ4F_getErrorCode");
        assert_ne!(
            isc as usize, isr as usize,
            "harness bug: LZ4F_isError resolved to the same address in both libraries"
        );

        // lz4frame.h:651-675, in order, as produced by LZ4F_GENERATE_STRING.
        const NAMES: [&str; 25] = [
            "OK_NoError",
            "ERROR_GENERIC",
            "ERROR_maxBlockSize_invalid",
            "ERROR_blockMode_invalid",
            "ERROR_parameter_invalid",
            "ERROR_compressionLevel_invalid",
            "ERROR_headerVersion_wrong",
            "ERROR_blockChecksum_invalid",
            "ERROR_reservedFlag_set",
            "ERROR_allocation_failed",
            "ERROR_srcSize_tooLarge",
            "ERROR_dstMaxSize_tooSmall",
            "ERROR_frameHeader_incomplete",
            "ERROR_frameType_unknown",
            "ERROR_frameSize_wrong",
            "ERROR_srcPtr_wrong",
            "ERROR_decompressionFailed",
            "ERROR_headerChecksum_invalid",
            "ERROR_contentChecksum_invalid",
            "ERROR_frameDecoding_alreadyStarted",
            "ERROR_compressionState_uninitialized",
            "ERROR_parameter_null",
            "ERROR_io_write",
            "ERROR_io_read",
            "ERROR_maxCode",
        ];
        const MAX_CODE: usize = 24; // LZ4F_ERROR_maxCode
        const UNSPEC: &str = "Unspecified error code";

        // The exact value axis required by the row, plus a randomized sweep.
        let mut codes: Vec<usize> = vec![0, 1, 23, 24, 25, usize::MAX / 2, usize::MAX];
        for k in 0usize..=25 {
            codes.push(err(k));
        }
        codes.push(err(100));
        // a few more interesting neighbours of the boundary
        for k in [26usize, 27, 50, 1000, 100_000] {
            codes.push(err(k));
        }
        let mut rng = Rng::new(0x0197_0000);
        for _ in 0..200 {
            codes.push(rng.next_u64() as usize);
        }

        for &code in &codes {
            // ---- LZ4F_isError ----
            let ia = (isc)(code);
            let ib = (isr)(code);
            assert_eq!(ia, ib, "row197: LZ4F_isError({code:#x}) C={ia} Rust={ib}");
            let want_is = code > err(MAX_CODE);
            assert_eq!(
                ia != 0,
                want_is,
                "row197: LZ4F_isError({code:#x}) = {ia}, expected {} (boundary is err({MAX_CODE}) = {:#x})",
                want_is as u32,
                err(MAX_CODE)
            );

            // ---- LZ4F_getErrorCode ----
            let ca = (cdc)(code);
            let cb = (cdr)(code);
            assert_eq!(ca, cb, "row197: LZ4F_getErrorCode({code:#x}) C={ca} Rust={cb}");
            // `(LZ4F_errorCodes)(-(ptrdiff_t)functionResult)` — this is the exact
            // inverse of LZ4F_returnErrorCode, which is what the static assert
            // guarantees is representable.
            let want_code: c_int = if !want_is {
                0 // LZ4F_OK_NoError
            } else {
                (0isize.wrapping_sub(code as isize)) as c_int
            };
            assert_eq!(
                ca, want_code,
                "row197: LZ4F_getErrorCode({code:#x}) = {ca}, expected {want_code}"
            );
            if want_is {
                // round-trip: returnErrorCode(getErrorCode(x)) == x
                let back = (0usize).wrapping_sub(ca as isize as usize);
                assert_eq!(
                    back, code,
                    "row197: -(ptrdiff_t) round-trip lost information for {code:#x} (code={ca}, back={back:#x})"
                );
            }

            // ---- LZ4F_getErrorName (compared by string CONTENT) ----
            let na = (nmc)(code);
            let nb = (nmr)(code);
            assert!(!na.is_null(), "row197: C LZ4F_getErrorName({code:#x}) returned NULL");
            assert!(!nb.is_null(), "row197: Rust LZ4F_getErrorName({code:#x}) returned NULL");
            let sa = CStr::from_ptr(na).to_bytes();
            let sb = CStr::from_ptr(nb).to_bytes();
            assert_eq!(
                sa,
                sb,
                "row197: LZ4F_getErrorName({code:#x}) differs: C={:?} Rust={:?}",
                String::from_utf8_lossy(sa),
                String::from_utf8_lossy(sb)
            );
            let want_name: &str = if want_is {
                NAMES[(0isize.wrapping_sub(code as isize)) as usize]
            } else {
                UNSPEC
            };
            assert_eq!(
                sa,
                want_name.as_bytes(),
                "row197: LZ4F_getErrorName({code:#x}) = {:?}, expected {want_name:?}",
                String::from_utf8_lossy(sa)
            );
        }

        // Explicit spot-checks for the specific values the row calls out, so the
        // audit does not have to re-derive them.
        for (code, is_err, name) in [
            (0usize, false, UNSPEC),
            (1usize, false, UNSPEC),
            (23usize, false, UNSPEC),
            (24usize, false, UNSPEC),
            (25usize, false, UNSPEC),
            (err(0), false, UNSPEC),          // err(0) == 0
            (err(1), true, "ERROR_GENERIC"),  // err(1) == usize::MAX
            (err(23), true, "ERROR_io_read"),
            (err(24), false, UNSPEC),         // == -maxCode, NOT an error
            (err(25), false, UNSPEC),
            (err(100), false, UNSPEC),
            (usize::MAX / 2, false, UNSPEC),
            (usize::MAX, true, "ERROR_GENERIC"),
        ] {
            assert_eq!(
                (isc)(code) != 0,
                is_err,
                "row197: spot-check LZ4F_isError({code:#x})"
            );
            assert_eq!((isc)(code), (isr)(code), "row197: spot-check isError C/Rust");
            let sa = CStr::from_ptr((nmc)(code)).to_bytes();
            let sb = CStr::from_ptr((nmr)(code)).to_bytes();
            assert_eq!(sa, name.as_bytes(), "row197: spot-check getErrorName({code:#x})");
            assert_eq!(sa, sb, "row197: spot-check getErrorName C/Rust ({code:#x})");
        }
    }
}

// ===========================================================================
// Row 198 — the `LZ4_MEMORY_USAGE` range check in lz4.h:166-172:
//           `#error "LZ4_MEMORY_USAGE is too small !"` below
//           `LZ4_MEMORY_USAGE_MIN (10)` and
//           `#error "... too large !"` above `LZ4_MEMORY_USAGE_MAX (20)`.
//
// COMPILE-TIME REJECTION — CANNOT BE TRIGGERED AT RUNTIME. `LZ4_MEMORY_USAGE`
// is a preprocessor macro consumed by `#if` directives; a bad value stops the
// build with `#error`, so a loaded `.so` proves the value was in range. It is
// not a parameter of any exported function and cannot be changed without
// rebuilding `c_src/` (forbidden here).
//
// What is asserted instead is the runtime invariant the range check protects —
// that both libraries agree on the ACTUAL configured value:
//   * `LZ4_sizeofState() == sizeof(LZ4_stream_t) == LZ4_STREAM_MINSIZE ==
//     (1 << LZ4_MEMORY_USAGE) + 32`, which for the default
//     `LZ4_MEMORY_USAGE_DEFAULT == 14` is `(1 << 14) + 32 == 16416`
//     (lz4.h:157-158, :729, lz4.c:752). A different `LZ4_MEMORY_USAGE` in the
//     Rust port would change this number.
//   * `LZ4_HASHLOG == LZ4_MEMORY_USAGE - 2` also drives the byU16/byU32
//     hash-table selection at `LZ4_64Klimit == 65547`; compression is therefore
//     compared byte-for-byte at srcSize 65546 (byU16), 65547 (the pivot, first
//     byU32 size) and 65548 (byU32) for several `Shape`s. A hash-table size or
//     log mismatch shows up as differing compressed output right there.
// ===========================================================================

#[test]
fn err_198_lz4_memory_usage_range_error() {
    unsafe {
        let l = libs();
        let (szc, szr) = pair!(l, FnVoidToInt, "LZ4_sizeofState");
        let (bndc, bndr) = pair!(l, FnCompressBound, "LZ4_compressBound");
        let (cdc, cdr) = pair!(l, FnCompressDefault, "LZ4_compress_default");
        let (dsc, dsr) = pair!(l, FnDecompressSafe, "LZ4_decompress_safe");
        assert_ne!(
            szc as usize, szr as usize,
            "harness bug: LZ4_sizeofState resolved to the same address in both libraries"
        );

        // ---- the configured LZ4_MEMORY_USAGE, observed through sizeofState ----
        const LZ4_MEMORY_USAGE: u32 = 14; // LZ4_MEMORY_USAGE_DEFAULT (lz4.h:159)
        const WANT: c_int = ((1usize << LZ4_MEMORY_USAGE) + 32) as c_int; // 16416
        let a = (szc)();
        let b = (szr)();
        assert_eq!(a, b, "row198: LZ4_sizeofState() C={a} Rust={b}");
        assert_eq!(
            a, WANT,
            "row198: LZ4_sizeofState() = {a}, expected (1 << {LZ4_MEMORY_USAGE}) + 32 = {WANT}; \
             the configured LZ4_MEMORY_USAGE is not {LZ4_MEMORY_USAGE}"
        );
        assert!(
            (10..=20).contains(&LZ4_MEMORY_USAGE),
            "row198: LZ4_MEMORY_USAGE must be within [LZ4_MEMORY_USAGE_MIN(10), \
             LZ4_MEMORY_USAGE_MAX(20)] or the build would have been rejected"
        );
        // deterministic and side-effect free
        for _ in 0..8 {
            assert_eq!((szc)(), WANT, "row198: C LZ4_sizeofState() not stable");
            assert_eq!((szr)(), WANT, "row198: Rust LZ4_sizeofState() not stable");
        }

        // ---- byU16 / byU32 pivot at LZ4_64Klimit == 65547 ----
        let mut rng = Rng::new(0x0198_0000);
        for &shape in &ALL_SHAPES {
            for &n in &[65546usize, 65547, 65548] {
                let src = gen(&mut rng, shape, n);
                let bc = (bndc)(n as c_int);
                let br = (bndr)(n as c_int);
                assert_eq!(bc, br, "row198: LZ4_compressBound({n}) C={bc} Rust={br}");
                let cap = bc as usize;
                let mut dc = vec![0xCCu8; cap];
                let mut dr = vec![0xCCu8; cap];
                let rc = (cdc)(
                    src.as_ptr() as *const c_char,
                    dc.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                let rr = (cdr)(
                    src.as_ptr() as *const c_char,
                    dr.as_mut_ptr() as *mut c_char,
                    n as c_int,
                    cap as c_int,
                );
                assert!(
                    rc > 0,
                    "row198: C LZ4_compress_default failed at srcSize={n} ({shape:?}), ret={rc}"
                );
                same_int_and_bytes(
                    &format!("row198 compress srcSize={n} {shape:?}"),
                    rc,
                    rr,
                    &dc,
                    &dr,
                );
                same_full_buffers(
                    &format!("row198 full dst srcSize={n} {shape:?}"),
                    &dc,
                    &dr,
                );

                // and both compressed blocks must decode back to the input in
                // both libraries (cross-decoding the C output with Rust too)
                for (who, blob) in [("C", &dc), ("Rust", &dr)] {
                    let mut oc = vec![0u8; n];
                    let mut or = vec![0u8; n];
                    let xc = (dsc)(
                        blob.as_ptr() as *const c_char,
                        oc.as_mut_ptr() as *mut c_char,
                        rc,
                        n as c_int,
                    );
                    let xr = (dsr)(
                        blob.as_ptr() as *const c_char,
                        or.as_mut_ptr() as *mut c_char,
                        rc,
                        n as c_int,
                    );
                    assert_eq!(
                        xc, n as c_int,
                        "row198: C decompress of the {who} block returned {xc} (srcSize={n})"
                    );
                    same_int_and_bytes(
                        &format!("row198 decompress {who} block srcSize={n} {shape:?}"),
                        xc,
                        xr,
                        &oc,
                        &or,
                    );
                    assert_eq!(oc, src, "row198: round-trip of the {who} block corrupted the data");
                    assert_eq!(or, src, "row198: Rust round-trip of the {who} block corrupted the data");
                }
            }
        }
    }
}
