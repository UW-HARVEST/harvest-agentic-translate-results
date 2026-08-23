//! Differential tests for the xxHash portion of the LZ4 translation
//! (`c_src/src/xxhash.c` vs `src/xxhash.rs`), xxHash 0.6.5 with
//! `XXH_NAMESPACE=LZ4_`.
//!
//! Every call is dispatched through BOTH shared libraries' export tables, so
//! the Rust `#[no_mangle]` wrappers are exercised exactly as a C caller would.
//!
//! Facts about this build of xxhash.c that drive the test design:
//!   * `XXH_FORCE_MEMORY_ACCESS` is undefined on x86_64 -> `XXH_read32/64` use
//!     `memcpy`, so unaligned reads are legal.
//!   * `XXH_FORCE_ALIGN_CHECK` is 0 on x86_64 (`__x86_64__` branch), so the
//!     one-shot entry points ALWAYS take the `XXH_unaligned` path regardless of
//!     input alignment.  The alignment sweep below still runs every pointer
//!     residue 0..8 because the Rust translation must reproduce that choice.
//!   * `XXH_ACCEPT_NULL_INPUT_POINTER` is 0, so `XXH*_update()` returns
//!     `XXH_ERROR` for a NULL input pointer *whatever* the length is, and it
//!     does so before touching the state.
//!   * `XXH_CPU_LITTLE_ENDIAN` is true here, so the little-endian paths run.
//!
//! One clause of xxhash.c is provably untestable here: in `XXH32_update_endian`
//! the `(len>=16)` term of
//! `state->large_len |= (len>=16) | (state->total_len_32>=16)` is redundant,
//! because `total_len_32` has already been incremented by `len` on the line
//! above, so `total_len_32 >= len`.  It can only matter after the 32-bit
//! `total_len_32` counter wraps, i.e. after >= 4 GiB of input, which is out of
//! scope for a unit test.

mod common;
use common::*;
use std::os::raw::{c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// State blob sizes.
//
// From xxhash.h (`XXH_STATIC_LINKING_ONLY` section) on this target:
//   sizeof(XXH32_state_t) == 48   (total_len_32, large_len, v1..v4,
//                                  mem32[4], memsize, reserved)
//   sizeof(XXH64_state_t) == 88   (total_len, v1..v4, mem64[4], memsize,
//                                  reserved[2], + 4 bytes tail padding)
// The Rust `#[repr(C)]` structs in src/xxhash.rs have the identical field
// order and types, so the layouts ARE identical and the raw blobs are
// byte-comparable.
//
// One wrinkle: `XXH32_reset`/`XXH64_reset` deliberately copy only
// `sizeof(state) - sizeof(state.reserved)` bytes (44 / 80), leaving the tail
// UNINITIALISED (`createState` is a plain `malloc`).  To make the raw blob
// comparison deterministic every state is pre-filled with the SAME sentinel
// byte in both libraries before use, exactly as the harness rule requires for
// output buffers.
// ---------------------------------------------------------------------------
const XXH32_STATE_SIZE: usize = 48;
const XXH64_STATE_SIZE: usize = 88;
const XXH32_RESET_WRITTEN: usize = 44;
const XXH64_RESET_WRITTEN: usize = 80;
const SENTINEL: u8 = 0xAA;
/// A *second*, distinct sentinel used to pre-fill `copyState` destinations.  It
/// must differ from `SENTINEL` (which is what a freshly `reset()` source has in
/// its untouched `reserved` tail) so that a `copyState` which skips the tail is
/// detectable.  Both libraries always get the SAME fill.
const DST_SENTINEL: u8 = 0x5A;

/// Internal accumulation buffer sizes (`mem32` / `mem64`) that
/// `XXH*_update()` fills before flushing a round.
const XXH32_BUFSIZE: usize = 16;
const XXH64_BUFSIZE: usize = 32;

// ---------------------------------------------------------------------------
// Signature aliases
// ---------------------------------------------------------------------------

type FnVersion = unsafe extern "C" fn() -> c_uint;
type FnXXH32 = unsafe extern "C" fn(*const c_void, usize, c_uint) -> c_uint;
type FnXXH64 = unsafe extern "C" fn(*const c_void, usize, u64) -> u64;
type FnCreateState = unsafe extern "C" fn() -> *mut c_void;
type FnFreeState = unsafe extern "C" fn(*mut c_void) -> c_int;
type FnCopyState = unsafe extern "C" fn(*mut c_void, *const c_void);
type FnReset32 = unsafe extern "C" fn(*mut c_void, c_uint) -> c_int;
type FnReset64 = unsafe extern "C" fn(*mut c_void, u64) -> c_int;
type FnUpdate = unsafe extern "C" fn(*mut c_void, *const c_void, usize) -> c_int;
type FnDigest32 = unsafe extern "C" fn(*const c_void) -> c_uint;
type FnDigest64 = unsafe extern "C" fn(*const c_void) -> u64;
type FnCanon32From = unsafe extern "C" fn(*mut XXH32_canonical_t, c_uint);
type FnCanon32To = unsafe extern "C" fn(*const XXH32_canonical_t) -> c_uint;
type FnCanon64From = unsafe extern "C" fn(*mut XXH64_canonical_t, u64);
type FnCanon64To = unsafe extern "C" fn(*const XXH64_canonical_t) -> u64;

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// A never-empty static buffer, used whenever a *valid, non-NULL* pointer is
/// needed for a zero-length call (a `Vec`'s pointer is dangling when empty).
static ZERO_PAD: [u8; 64] = [0u8; 64];

fn data_ptr(v: &[u8]) -> *const u8 {
    if v.is_empty() {
        ZERO_PAD.as_ptr()
    } else {
        v.as_ptr()
    }
}

/// Copy `data` into a freshly allocated buffer whose start address is
/// `align`-aligned **plus** `offset` bytes (i.e. deliberately misaligned).
fn aligned_copy(data: &[u8], align: usize, offset: usize) -> AlignedBuf {
    let mut b = AlignedBuf::with_offset(data.len(), align, offset);
    if !data.is_empty() {
        unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), b.as_mut_ptr(), data.len()) };
    }
    b
}

fn oneshot_lengths() -> Vec<usize> {
    vec![
        0, 1, 2, 3, 4, 5, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256,
        1023, 1024, 1025, 4096, 65535, 65536, 100000,
    ]
}

/// Lengths for the alignment sweep — every residue class around the 4/8/16/32
/// byte read granularities, kept small so 9 offsets x 2 seeds stays cheap.
fn align_lengths() -> Vec<usize> {
    vec![
        0, 1, 2, 3, 4, 5, 7, 8, 9, 12, 15, 16, 17, 23, 24, 31, 32, 33, 39, 40, 47, 48, 63, 64, 65,
        95, 96, 127, 128, 129, 255, 257, 1024, 4097,
    ]
}

fn seeds32() -> Vec<u32> {
    vec![0, 1, 0xFFFF_FFFF, 0x8000_0000, 0x9E37_79B1, 2654435761]
}

fn seeds64() -> Vec<u64> {
    vec![
        0,
        1,
        0xFFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x8000_0000_0000_0000,
        0x9E37_79B1_85EB_CA87,
        11400714785074694791,
    ]
}

/// Total input lengths used by the streaming tests.
fn stream_lengths() -> Vec<usize> {
    vec![
        0, 1, 2, 3, 5, 8, 15, 16, 17, 31, 32, 33, 47, 48, 63, 64, 65, 100, 127, 128, 129, 255, 256,
        511, 512, 1023, 1024, 1025, 4096, 10007,
    ]
}

/// Fixed chunk sizes demanded by the task, straddling both internal buffer
/// sizes (16 for XXH32, 32 for XXH64) and their +/-1 neighbours.
fn chunk_sizes() -> Vec<usize> {
    vec![1, 2, 3, 5, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65]
}

// ---------------------------------------------------------------------------
// XXH32 streaming duo
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Api32 {
    create: FnCreateState,
    free: FnFreeState,
    copy: FnCopyState,
    reset: FnReset32,
    update: FnUpdate,
    digest: FnDigest32,
    tag: &'static str,
}

fn api32_pair() -> (Api32, Api32) {
    let (cc, rc) = both::<FnCreateState>("LZ4_XXH32_createState");
    let (cf, rf) = both::<FnFreeState>("LZ4_XXH32_freeState");
    let (ccp, rcp) = both::<FnCopyState>("LZ4_XXH32_copyState");
    let (cr, rr) = both::<FnReset32>("LZ4_XXH32_reset");
    let (cu, ru) = both::<FnUpdate>("LZ4_XXH32_update");
    let (cd, rd) = both::<FnDigest32>("LZ4_XXH32_digest");
    (
        Api32 { create: cc, free: cf, copy: ccp, reset: cr, update: cu, digest: cd, tag: "C" },
        Api32 { create: rc, free: rf, copy: rcp, reset: rr, update: ru, digest: rd, tag: "Rust" },
    )
}

struct S32 {
    api: Api32,
    p: *mut c_void,
}

impl S32 {
    fn new(api: Api32) -> S32 {
        let p = unsafe { (api.create)() };
        assert!(!p.is_null(), "{}: LZ4_XXH32_createState returned NULL", api.tag);
        unsafe { std::ptr::write_bytes(p as *mut u8, SENTINEL, XXH32_STATE_SIZE) };
        S32 { api, p }
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.p as *const u8, XXH32_STATE_SIZE) }
    }
    fn snapshot(&self) -> [u8; XXH32_STATE_SIZE] {
        let mut a = [0u8; XXH32_STATE_SIZE];
        a.copy_from_slice(self.bytes());
        a
    }
}

impl Drop for S32 {
    fn drop(&mut self) {
        unsafe { (self.api.free)(self.p) };
    }
}

struct Duo32 {
    c: S32,
    r: S32,
}

impl Duo32 {
    fn new() -> Duo32 {
        let (ca, ra) = api32_pair();
        Duo32 { c: S32::new(ca), r: S32::new(ra) }
    }

    fn check_state(&self, ctx: &str, what: &str) {
        if self.c.bytes() != self.r.bytes() {
            assert_bytes_eq(
                &format!("{}: XXH32_state_t blob differs after {}", ctx, what),
                self.c.bytes(),
                self.r.bytes(),
            );
        }
    }

    fn reset(&self, ctx: &str, seed: u32) {
        let rc_c = unsafe { (self.c.api.reset)(self.c.p, seed) };
        let rc_r = unsafe { (self.r.api.reset)(self.r.p, seed) };
        assert_eq!(rc_c, XXH_OK, "{}: C XXH32_reset(seed={:#x})", ctx, seed);
        assert_eq!(
            rc_c, rc_r,
            "{}: XXH32_reset(seed={:#x}) errorcode C={} Rust={}",
            ctx, seed, rc_c, rc_r
        );
        self.check_state(ctx, &format!("reset(seed={:#x})", seed));
    }

    /// Returns the shared errorcode (already asserted identical).
    fn update(&self, ctx: &str, idx: usize, buf: *const u8, len: usize) -> c_int {
        let rc_c = unsafe { (self.c.api.update)(self.c.p, buf as *const c_void, len) };
        let rc_r = unsafe { (self.r.api.update)(self.r.p, buf as *const c_void, len) };
        if rc_c != rc_r {
            panic!(
                "{}: XXH32_update #{} (len={}) errorcode C={} Rust={}",
                ctx, idx, len, rc_c, rc_r
            );
        }
        if self.c.bytes() != self.r.bytes() {
            self.check_state(ctx, &format!("update #{} (len={})", idx, len));
        }
        rc_c
    }

    /// Digest from both libraries; asserts equality and that the state blob is
    /// unmodified by the (const) digest call.
    fn digest(&self, ctx: &str) -> u32 {
        let bc = self.c.snapshot();
        let br = self.r.snapshot();
        let hc = unsafe { (self.c.api.digest)(self.c.p) };
        let hr = unsafe { (self.r.api.digest)(self.r.p) };
        assert_eq!(hc, hr, "{}: XXH32_digest C={:#010x} Rust={:#010x}", ctx, hc, hr);
        assert_bytes_eq(
            &format!("{}: C XXH32_digest mutated the state", ctx),
            &bc,
            self.c.bytes(),
        );
        assert_bytes_eq(
            &format!("{}: Rust XXH32_digest mutated the state", ctx),
            &br,
            self.r.bytes(),
        );
        hc
    }

    fn copy_from(&self, ctx: &str, src: &Duo32) {
        // Pre-fill BOTH destinations with the same distinct sentinel: XXH32_copyState
        // is a full-struct `memcpy(dst, src, sizeof(*dst))`, so every one of the 48
        // bytes (including the `reserved` tail) must be overwritten.
        unsafe {
            std::ptr::write_bytes(self.c.p as *mut u8, DST_SENTINEL, XXH32_STATE_SIZE);
            std::ptr::write_bytes(self.r.p as *mut u8, DST_SENTINEL, XXH32_STATE_SIZE);
        }
        unsafe { (self.c.api.copy)(self.c.p, src.c.p as *const c_void) };
        unsafe { (self.r.api.copy)(self.r.p, src.r.p as *const c_void) };
        // copyState is a full-struct memcpy, so the destination must be a byte
        // for byte clone of the source in each library, and equal across them.
        assert_bytes_eq(
            &format!("{}: C XXH32_copyState did not clone the source blob", ctx),
            src.c.bytes(),
            self.c.bytes(),
        );
        assert_bytes_eq(
            &format!("{}: Rust XXH32_copyState did not clone the source blob", ctx),
            src.r.bytes(),
            self.r.bytes(),
        );
        self.check_state(ctx, "copyState");
    }
}

// ---------------------------------------------------------------------------
// XXH64 streaming duo (same shape, 64-bit types)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Api64 {
    create: FnCreateState,
    free: FnFreeState,
    copy: FnCopyState,
    reset: FnReset64,
    update: FnUpdate,
    digest: FnDigest64,
    tag: &'static str,
}

fn api64_pair() -> (Api64, Api64) {
    let (cc, rc) = both::<FnCreateState>("LZ4_XXH64_createState");
    let (cf, rf) = both::<FnFreeState>("LZ4_XXH64_freeState");
    let (ccp, rcp) = both::<FnCopyState>("LZ4_XXH64_copyState");
    let (cr, rr) = both::<FnReset64>("LZ4_XXH64_reset");
    let (cu, ru) = both::<FnUpdate>("LZ4_XXH64_update");
    let (cd, rd) = both::<FnDigest64>("LZ4_XXH64_digest");
    (
        Api64 { create: cc, free: cf, copy: ccp, reset: cr, update: cu, digest: cd, tag: "C" },
        Api64 { create: rc, free: rf, copy: rcp, reset: rr, update: ru, digest: rd, tag: "Rust" },
    )
}

struct S64 {
    api: Api64,
    p: *mut c_void,
}

impl S64 {
    fn new(api: Api64) -> S64 {
        let p = unsafe { (api.create)() };
        assert!(!p.is_null(), "{}: LZ4_XXH64_createState returned NULL", api.tag);
        unsafe { std::ptr::write_bytes(p as *mut u8, SENTINEL, XXH64_STATE_SIZE) };
        S64 { api, p }
    }
    fn bytes(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.p as *const u8, XXH64_STATE_SIZE) }
    }
    fn snapshot(&self) -> [u8; XXH64_STATE_SIZE] {
        let mut a = [0u8; XXH64_STATE_SIZE];
        a.copy_from_slice(self.bytes());
        a
    }
}

impl Drop for S64 {
    fn drop(&mut self) {
        unsafe { (self.api.free)(self.p) };
    }
}

struct Duo64 {
    c: S64,
    r: S64,
}

impl Duo64 {
    fn new() -> Duo64 {
        let (ca, ra) = api64_pair();
        Duo64 { c: S64::new(ca), r: S64::new(ra) }
    }

    fn check_state(&self, ctx: &str, what: &str) {
        if self.c.bytes() != self.r.bytes() {
            assert_bytes_eq(
                &format!("{}: XXH64_state_t blob differs after {}", ctx, what),
                self.c.bytes(),
                self.r.bytes(),
            );
        }
    }

    fn reset(&self, ctx: &str, seed: u64) {
        let rc_c = unsafe { (self.c.api.reset)(self.c.p, seed) };
        let rc_r = unsafe { (self.r.api.reset)(self.r.p, seed) };
        assert_eq!(rc_c, XXH_OK, "{}: C XXH64_reset(seed={:#x})", ctx, seed);
        assert_eq!(
            rc_c, rc_r,
            "{}: XXH64_reset(seed={:#x}) errorcode C={} Rust={}",
            ctx, seed, rc_c, rc_r
        );
        self.check_state(ctx, &format!("reset(seed={:#x})", seed));
    }

    fn update(&self, ctx: &str, idx: usize, buf: *const u8, len: usize) -> c_int {
        let rc_c = unsafe { (self.c.api.update)(self.c.p, buf as *const c_void, len) };
        let rc_r = unsafe { (self.r.api.update)(self.r.p, buf as *const c_void, len) };
        if rc_c != rc_r {
            panic!(
                "{}: XXH64_update #{} (len={}) errorcode C={} Rust={}",
                ctx, idx, len, rc_c, rc_r
            );
        }
        if self.c.bytes() != self.r.bytes() {
            self.check_state(ctx, &format!("update #{} (len={})", idx, len));
        }
        rc_c
    }

    fn digest(&self, ctx: &str) -> u64 {
        let bc = self.c.snapshot();
        let br = self.r.snapshot();
        let hc = unsafe { (self.c.api.digest)(self.c.p) };
        let hr = unsafe { (self.r.api.digest)(self.r.p) };
        assert_eq!(hc, hr, "{}: XXH64_digest C={:#018x} Rust={:#018x}", ctx, hc, hr);
        assert_bytes_eq(
            &format!("{}: C XXH64_digest mutated the state", ctx),
            &bc,
            self.c.bytes(),
        );
        assert_bytes_eq(
            &format!("{}: Rust XXH64_digest mutated the state", ctx),
            &br,
            self.r.bytes(),
        );
        hc
    }

    fn copy_from(&self, ctx: &str, src: &Duo64) {
        // Pre-fill BOTH destinations with the same distinct sentinel: XXH64_copyState
        // is a full-struct `memcpy(dst, src, sizeof(*dst))`, so every one of the 88
        // bytes (including the `reserved` tail and the trailing padding) must be
        // overwritten.
        unsafe {
            std::ptr::write_bytes(self.c.p as *mut u8, DST_SENTINEL, XXH64_STATE_SIZE);
            std::ptr::write_bytes(self.r.p as *mut u8, DST_SENTINEL, XXH64_STATE_SIZE);
        }
        unsafe { (self.c.api.copy)(self.c.p, src.c.p as *const c_void) };
        unsafe { (self.r.api.copy)(self.r.p, src.r.p as *const c_void) };
        assert_bytes_eq(
            &format!("{}: C XXH64_copyState did not clone the source blob", ctx),
            src.c.bytes(),
            self.c.bytes(),
        );
        assert_bytes_eq(
            &format!("{}: Rust XXH64_copyState did not clone the source blob", ctx),
            src.r.bytes(),
            self.r.bytes(),
        );
        self.check_state(ctx, "copyState");
    }
}

// ---------------------------------------------------------------------------
// One-shot comparison helpers
// ---------------------------------------------------------------------------

fn cmp32(ctx: &str, p: *const u8, len: usize, seed: u32) -> u32 {
    let (c, r) = both::<FnXXH32>("LZ4_XXH32");
    let hc = unsafe { c(p as *const c_void, len, seed) };
    let hr = unsafe { r(p as *const c_void, len, seed) };
    assert_eq!(
        hc, hr,
        "{}: LZ4_XXH32(len={}, seed={:#010x}) C={:#010x} Rust={:#010x}",
        ctx, len, seed, hc, hr
    );
    hc
}

fn cmp64(ctx: &str, p: *const u8, len: usize, seed: u64) -> u64 {
    let (c, r) = both::<FnXXH64>("LZ4_XXH64");
    let hc = unsafe { c(p as *const c_void, len, seed) };
    let hr = unsafe { r(p as *const c_void, len, seed) };
    assert_eq!(
        hc, hr,
        "{}: LZ4_XXH64(len={}, seed={:#018x}) C={:#018x} Rust={:#018x}",
        ctx, len, seed, hc, hr
    );
    hc
}

// ===========================================================================
// Version
// ===========================================================================

#[test]
fn xxh_version_number() {
    let (c, r) = both::<FnVersion>("LZ4_XXH_versionNumber");
    let vc = unsafe { c() };
    let vr = unsafe { r() };
    assert_eq!(vc, vr, "LZ4_XXH_versionNumber C={} Rust={}", vc, vr);
    // XXH_VERSION_MAJOR*10000 + MINOR*100 + RELEASE == 0*10000 + 6*100 + 5
    assert_eq!(vc, 605, "unexpected xxHash version number {}", vc);
}

// ===========================================================================
// One-shot hashing
// ===========================================================================

#[test]
fn xxh32_oneshot_length_shape_seed_sweep() {
    let mut rng = Rng::new(0x3200_0001);
    for len in oneshot_lengths() {
        for shape in 0..N_SHAPES {
            let data = gen_shape(&mut rng, shape, len);
            assert_eq!(data.len(), len);
            let p = data_ptr(&data);
            for seed in seeds32() {
                cmp32(
                    &format!("XXH32 oneshot shape={} len={}", shape_name(shape), len),
                    p,
                    len,
                    seed,
                );
            }
        }
    }
}

#[test]
fn xxh64_oneshot_length_shape_seed_sweep() {
    let mut rng = Rng::new(0x6400_0001);
    for len in oneshot_lengths() {
        for shape in 0..N_SHAPES {
            let data = gen_shape(&mut rng, shape, len);
            assert_eq!(data.len(), len);
            let p = data_ptr(&data);
            for seed in seeds64() {
                cmp64(
                    &format!("XXH64 oneshot shape={} len={}", shape_name(shape), len),
                    p,
                    len,
                    seed,
                );
            }
        }
    }
}

/// xxhash.c has distinct `XXH_aligned` / `XXH_unaligned` read paths selected by
/// `XXH_FORCE_ALIGN_CHECK`; on x86_64 the check is compiled out, so every
/// pointer residue must give the same answer. Run offsets 0..8 off a
/// 16-byte-aligned allocation.
#[test]
fn xxh32_oneshot_alignment_sweep() {
    let mut rng = Rng::new(0x3200_A11C);
    for (i, len) in align_lengths().into_iter().enumerate() {
        let data = gen_shape(&mut rng, i, len);
        let mut reference: Option<u32> = None;
        for offset in 0..=8usize {
            let buf = aligned_copy(&data, 16, offset);
            assert_eq!(buf.as_slice(), &data[..], "aligned_copy corrupted the payload");
            for seed in [0u32, 0x1234_5678] {
                let h = cmp32(
                    &format!("XXH32 align len={} offset={} ptr%16={}", len, offset, (buf.as_ptr() as usize) % 16),
                    buf.as_ptr(),
                    len,
                    seed,
                );
                if seed == 0 {
                    match reference {
                        None => reference = Some(h),
                        Some(h0) => assert_eq!(
                            h0, h,
                            "XXH32 len={} is alignment-sensitive: offset 0 -> {:#010x}, offset {} -> {:#010x}",
                            len, h0, offset, h
                        ),
                    }
                }
            }
        }
    }
}

#[test]
fn xxh64_oneshot_alignment_sweep() {
    let mut rng = Rng::new(0x6400_A11C);
    for (i, len) in align_lengths().into_iter().enumerate() {
        let data = gen_shape(&mut rng, i, len);
        let mut reference: Option<u64> = None;
        for offset in 0..=8usize {
            let buf = aligned_copy(&data, 16, offset);
            assert_eq!(buf.as_slice(), &data[..], "aligned_copy corrupted the payload");
            for seed in [0u64, 0xDEAD_BEEF_CAFE_F00D] {
                let h = cmp64(
                    &format!("XXH64 align len={} offset={} ptr%16={}", len, offset, (buf.as_ptr() as usize) % 16),
                    buf.as_ptr(),
                    len,
                    seed,
                );
                if seed == 0 {
                    match reference {
                        None => reference = Some(h),
                        Some(h0) => assert_eq!(
                            h0, h,
                            "XXH64 len={} is alignment-sensitive: offset 0 -> {:#018x}, offset {} -> {:#018x}",
                            len, h0, offset, h
                        ),
                    }
                }
            }
        }
    }
}

#[test]
fn xxh_oneshot_randomized_lengths_and_seeds() {
    let mut rng = Rng::new(0xF00D_1234);
    for iter in 0..700usize {
        // Length distribution biased towards the short/finalize-heavy range but
        // still reaching multi-KB inputs that exercise the main loops.
        let len = match rng.below(20) {
            0 => rng.range(5000, 40000),
            1..=5 => rng.range(300, 5000),
            _ => rng.range(0, 300),
        };
        let shape = rng.below(N_SHAPES);
        let data = gen_shape(&mut rng, shape, len);
        let p = data_ptr(&data);
        let s32 = rng.next_u32();
        let s64 = rng.next_u64();
        let ctx = format!("random iter={} shape={} len={}", iter, shape_name(shape), len);
        cmp32(&ctx, p, len, s32);
        cmp32(&ctx, p, len, 0);
        cmp64(&ctx, p, len, s64);
        cmp64(&ctx, p, len, 0);
    }
}

// ===========================================================================
// State allocation
// ===========================================================================

#[test]
fn xxh32_state_create_and_free() {
    let (c_create, r_create) = both::<FnCreateState>("LZ4_XXH32_createState");
    let (c_free, r_free) = both::<FnFreeState>("LZ4_XXH32_freeState");
    for i in 0..64 {
        let pc = unsafe { c_create() };
        let pr = unsafe { r_create() };
        assert!(!pc.is_null(), "iter {}: C LZ4_XXH32_createState returned NULL", i);
        assert!(!pr.is_null(), "iter {}: Rust LZ4_XXH32_createState returned NULL", i);
        // Prove the block really is >= sizeof(XXH32_state_t) in both libs.
        unsafe {
            std::ptr::write_bytes(pc as *mut u8, SENTINEL, XXH32_STATE_SIZE);
            std::ptr::write_bytes(pr as *mut u8, SENTINEL, XXH32_STATE_SIZE);
        }
        let rc = unsafe { c_free(pc) };
        let rr = unsafe { r_free(pr) };
        assert_eq!(rc, XXH_OK, "iter {}: C LZ4_XXH32_freeState -> {}", i, rc);
        assert_eq!(rc, rr, "iter {}: LZ4_XXH32_freeState C={} Rust={}", i, rc, rr);
    }
}

#[test]
fn xxh64_state_create_and_free() {
    let (c_create, r_create) = both::<FnCreateState>("LZ4_XXH64_createState");
    let (c_free, r_free) = both::<FnFreeState>("LZ4_XXH64_freeState");
    for i in 0..64 {
        let pc = unsafe { c_create() };
        let pr = unsafe { r_create() };
        assert!(!pc.is_null(), "iter {}: C LZ4_XXH64_createState returned NULL", i);
        assert!(!pr.is_null(), "iter {}: Rust LZ4_XXH64_createState returned NULL", i);
        unsafe {
            std::ptr::write_bytes(pc as *mut u8, SENTINEL, XXH64_STATE_SIZE);
            std::ptr::write_bytes(pr as *mut u8, SENTINEL, XXH64_STATE_SIZE);
        }
        let rc = unsafe { c_free(pc) };
        let rr = unsafe { r_free(pr) };
        assert_eq!(rc, XXH_OK, "iter {}: C LZ4_XXH64_freeState -> {}", i, rc);
        assert_eq!(rc, rr, "iter {}: LZ4_XXH64_freeState C={} Rust={}", i, rc, rr);
    }
}

// ===========================================================================
// reset() — errorcode + raw state bytes
// ===========================================================================

#[test]
fn xxh32_reset_raw_state_bytes() {
    let duo = Duo32::new();
    let mut rng = Rng::new(0x3200_5E7);
    let mut seeds = seeds32();
    for _ in 0..64 {
        seeds.push(rng.next_u32());
    }
    for seed in seeds {
        // Re-prime BOTH blobs with the same sentinel so the deliberately
        // untouched `reserved` tail is deterministic.
        unsafe {
            std::ptr::write_bytes(duo.c.p as *mut u8, SENTINEL, XXH32_STATE_SIZE);
            std::ptr::write_bytes(duo.r.p as *mut u8, SENTINEL, XXH32_STATE_SIZE);
        }
        let ctx = format!("XXH32 reset seed={:#010x}", seed);
        duo.reset(&ctx, seed);
        // reset() must copy exactly `sizeof(state) - sizeof(reserved)` bytes.
        assert_eq!(
            &duo.c.bytes()[XXH32_RESET_WRITTEN..],
            &[SENTINEL; XXH32_STATE_SIZE - XXH32_RESET_WRITTEN][..],
            "{}: C XXH32_reset wrote into the reserved tail",
            ctx
        );
        assert_eq!(
            &duo.r.bytes()[XXH32_RESET_WRITTEN..],
            &[SENTINEL; XXH32_STATE_SIZE - XXH32_RESET_WRITTEN][..],
            "{}: Rust XXH32_reset wrote into the reserved tail",
            ctx
        );
        // A fresh state must digest to the empty-input one-shot hash.
        let h = duo.digest(&ctx);
        let one = cmp32(&ctx, ZERO_PAD.as_ptr(), 0, seed);
        assert_eq!(h, one, "{}: digest after reset {:#010x} != XXH32(\"\") {:#010x}", ctx, h, one);
    }
}

#[test]
fn xxh64_reset_raw_state_bytes() {
    let duo = Duo64::new();
    let mut rng = Rng::new(0x6400_5E7);
    let mut seeds = seeds64();
    for _ in 0..64 {
        seeds.push(rng.next_u64());
    }
    for seed in seeds {
        unsafe {
            std::ptr::write_bytes(duo.c.p as *mut u8, SENTINEL, XXH64_STATE_SIZE);
            std::ptr::write_bytes(duo.r.p as *mut u8, SENTINEL, XXH64_STATE_SIZE);
        }
        let ctx = format!("XXH64 reset seed={:#018x}", seed);
        duo.reset(&ctx, seed);
        assert_eq!(
            &duo.c.bytes()[XXH64_RESET_WRITTEN..],
            &[SENTINEL; XXH64_STATE_SIZE - XXH64_RESET_WRITTEN][..],
            "{}: C XXH64_reset wrote into the reserved tail",
            ctx
        );
        assert_eq!(
            &duo.r.bytes()[XXH64_RESET_WRITTEN..],
            &[SENTINEL; XXH64_STATE_SIZE - XXH64_RESET_WRITTEN][..],
            "{}: Rust XXH64_reset wrote into the reserved tail",
            ctx
        );
        let h = duo.digest(&ctx);
        let one = cmp64(&ctx, ZERO_PAD.as_ptr(), 0, seed);
        assert_eq!(h, one, "{}: digest after reset {:#018x} != XXH64(\"\") {:#018x}", ctx, h, one);
    }
}

// ===========================================================================
// Streaming: fixed chunk sizes / single shot / byte-at-a-time
// ===========================================================================

/// Feed `data` to both libraries in `chunk`-sized pieces (`chunk == 0` means one
/// single update of everything) and return the agreed digest.
fn stream32_fixed(duo: &Duo32, ctx: &str, seed: u32, data: &[u8], chunk: usize) -> u32 {
    duo.reset(ctx, seed);
    let p = data_ptr(data);
    if data.is_empty() {
        // A single zero-length update on a valid pointer is a defined no-op.
        assert_eq!(duo.update(ctx, 0, p, 0), XXH_OK, "{}: empty update", ctx);
    } else if chunk == 0 {
        assert_eq!(duo.update(ctx, 0, p, data.len()), XXH_OK, "{}: single update", ctx);
    } else {
        let mut off = 0usize;
        let mut idx = 0usize;
        while off < data.len() {
            let n = chunk.min(data.len() - off);
            assert_eq!(
                duo.update(ctx, idx, unsafe { p.add(off) }, n),
                XXH_OK,
                "{}: update #{}",
                ctx,
                idx
            );
            off += n;
            idx += 1;
        }
    }
    duo.digest(ctx)
}

fn stream64_fixed(duo: &Duo64, ctx: &str, seed: u64, data: &[u8], chunk: usize) -> u64 {
    duo.reset(ctx, seed);
    let p = data_ptr(data);
    if data.is_empty() {
        assert_eq!(duo.update(ctx, 0, p, 0), XXH_OK, "{}: empty update", ctx);
    } else if chunk == 0 {
        assert_eq!(duo.update(ctx, 0, p, data.len()), XXH_OK, "{}: single update", ctx);
    } else {
        let mut off = 0usize;
        let mut idx = 0usize;
        while off < data.len() {
            let n = chunk.min(data.len() - off);
            assert_eq!(
                duo.update(ctx, idx, unsafe { p.add(off) }, n),
                XXH_OK,
                "{}: update #{}",
                ctx,
                idx
            );
            off += n;
            idx += 1;
        }
    }
    duo.digest(ctx)
}

/// Feed `data` following an explicit list of chunk lengths (which must sum to
/// `data.len()`; zero entries are allowed and meaningful).
fn stream32_pattern(duo: &Duo32, ctx: &str, seed: u32, data: &[u8], chunks: &[usize]) -> u32 {
    duo.reset(ctx, seed);
    let p = data_ptr(data);
    let mut off = 0usize;
    for (idx, &n) in chunks.iter().enumerate() {
        assert!(off + n <= data.len(), "{}: chunk list overruns the input", ctx);
        assert_eq!(
            duo.update(ctx, idx, unsafe { p.add(off) }, n),
            XXH_OK,
            "{}: update #{} (len={})",
            ctx,
            idx,
            n
        );
        off += n;
    }
    assert_eq!(off, data.len(), "{}: chunk list did not cover the input", ctx);
    duo.digest(ctx)
}

fn stream64_pattern(duo: &Duo64, ctx: &str, seed: u64, data: &[u8], chunks: &[usize]) -> u64 {
    duo.reset(ctx, seed);
    let p = data_ptr(data);
    let mut off = 0usize;
    for (idx, &n) in chunks.iter().enumerate() {
        assert!(off + n <= data.len(), "{}: chunk list overruns the input", ctx);
        assert_eq!(
            duo.update(ctx, idx, unsafe { p.add(off) }, n),
            XXH_OK,
            "{}: update #{} (len={})",
            ctx,
            idx,
            n
        );
        off += n;
    }
    assert_eq!(off, data.len(), "{}: chunk list did not cover the input", ctx);
    duo.digest(ctx)
}

#[test]
fn xxh32_streaming_chunk_patterns() {
    let duo = Duo32::new();
    let mut rng = Rng::new(0x3200_C401);
    for (i, len) in stream_lengths().into_iter().enumerate() {
        let shape = i % N_SHAPES;
        let data = gen_shape(&mut rng, shape, len);
        let seed = if i % 3 == 0 { 0 } else { rng.next_u32() };
        let want = cmp32(
            &format!("XXH32 stream oneshot ref len={} shape={}", len, shape_name(shape)),
            data_ptr(&data),
            len,
            seed,
        );

        // one single update of the whole input
        let ctx = format!("XXH32 stream single len={} shape={}", len, shape_name(shape));
        let got = stream32_fixed(&duo, &ctx, seed, &data, 0);
        assert_eq!(got, want, "{}: digest {:#010x} != one-shot {:#010x}", ctx, got, want);

        for chunk in chunk_sizes() {
            // byte-at-a-time on the very large inputs is redundant and slow
            if chunk <= 3 && len > 4096 {
                continue;
            }
            let ctx = format!(
                "XXH32 stream chunk={} len={} shape={}",
                chunk,
                len,
                shape_name(shape)
            );
            let got = stream32_fixed(&duo, &ctx, seed, &data, chunk);
            assert_eq!(got, want, "{}: digest {:#010x} != one-shot {:#010x}", ctx, got, want);
        }
    }
}

#[test]
fn xxh64_streaming_chunk_patterns() {
    let duo = Duo64::new();
    let mut rng = Rng::new(0x6400_C401);
    for (i, len) in stream_lengths().into_iter().enumerate() {
        let shape = i % N_SHAPES;
        let data = gen_shape(&mut rng, shape, len);
        let seed = if i % 3 == 0 { 0 } else { rng.next_u64() };
        let want = cmp64(
            &format!("XXH64 stream oneshot ref len={} shape={}", len, shape_name(shape)),
            data_ptr(&data),
            len,
            seed,
        );

        let ctx = format!("XXH64 stream single len={} shape={}", len, shape_name(shape));
        let got = stream64_fixed(&duo, &ctx, seed, &data, 0);
        assert_eq!(got, want, "{}: digest {:#018x} != one-shot {:#018x}", ctx, got, want);

        for chunk in chunk_sizes() {
            if chunk <= 3 && len > 4096 {
                continue;
            }
            let ctx = format!(
                "XXH64 stream chunk={} len={} shape={}",
                chunk,
                len,
                shape_name(shape)
            );
            let got = stream64_fixed(&duo, &ctx, seed, &data, chunk);
            assert_eq!(got, want, "{}: digest {:#018x} != one-shot {:#018x}", ctx, got, want);
        }
    }
}

/// Build a randomised chunk list covering `len` bytes, occasionally emitting a
/// zero-length chunk (with a valid pointer, which xxhash.c treats as a no-op).
fn random_chunks(rng: &mut Rng, len: usize, max_chunk: usize) -> Vec<usize> {
    let mut chunks = Vec::new();
    let mut off = 0usize;
    if len == 0 {
        chunks.push(0);
        return chunks;
    }
    while off < len {
        if rng.below(6) == 0 {
            chunks.push(0);
        }
        let n = rng.range(1, max_chunk).min(len - off);
        chunks.push(n);
        off += n;
    }
    if rng.bool() {
        chunks.push(0);
    }
    chunks
}

#[test]
fn xxh32_streaming_random_chunks() {
    let duo = Duo32::new();
    let mut rng = Rng::new(0x3200_C4AD);
    for iter in 0..400usize {
        let len = match rng.below(10) {
            0 => rng.range(2000, 12000),
            1..=3 => rng.range(200, 2000),
            _ => rng.range(0, 200),
        };
        let shape = rng.below(N_SHAPES);
        let data = gen_shape(&mut rng, shape, len);
        let seed = if iter % 4 == 0 { 0 } else { rng.next_u32() };
        let want = cmp32(
            &format!("XXH32 randchunk ref iter={} len={}", iter, len),
            data_ptr(&data),
            len,
            seed,
        );
        let max_chunk = *[1usize, 2, 4, 17, 33, 64, 200].get(rng.below(7)).unwrap();
        let chunks = random_chunks(&mut rng, len, max_chunk);
        let ctx = format!(
            "XXH32 randchunk iter={} len={} shape={} nchunks={} max={}",
            iter,
            len,
            shape_name(shape),
            chunks.len(),
            max_chunk
        );
        let got = stream32_pattern(&duo, &ctx, seed, &data, &chunks);
        assert_eq!(got, want, "{}: digest {:#010x} != one-shot {:#010x}", ctx, got, want);
    }
}

#[test]
fn xxh64_streaming_random_chunks() {
    let duo = Duo64::new();
    let mut rng = Rng::new(0x6400_C4AD);
    for iter in 0..400usize {
        let len = match rng.below(10) {
            0 => rng.range(2000, 12000),
            1..=3 => rng.range(200, 2000),
            _ => rng.range(0, 200),
        };
        let shape = rng.below(N_SHAPES);
        let data = gen_shape(&mut rng, shape, len);
        let seed = if iter % 4 == 0 { 0 } else { rng.next_u64() };
        let want = cmp64(
            &format!("XXH64 randchunk ref iter={} len={}", iter, len),
            data_ptr(&data),
            len,
            seed,
        );
        let max_chunk = *[1usize, 2, 4, 33, 65, 128, 400].get(rng.below(7)).unwrap();
        let chunks = random_chunks(&mut rng, len, max_chunk);
        let ctx = format!(
            "XXH64 randchunk iter={} len={} shape={} nchunks={} max={}",
            iter,
            len,
            shape_name(shape),
            chunks.len(),
            max_chunk
        );
        let got = stream64_pattern(&duo, &ctx, seed, &data, &chunks);
        assert_eq!(got, want, "{}: digest {:#018x} != one-shot {:#018x}", ctx, got, want);
    }
}

// ===========================================================================
// Streaming: splits landing exactly on the internal buffer boundary
// ===========================================================================

#[test]
fn xxh32_streaming_boundary_splits() {
    let duo = Duo32::new();
    let mut rng = Rng::new(0x3200_B0D1);
    // Candidate split points: 0, 1 and every internal-buffer multiple
    // (16 bytes for XXH32) plus/minus one byte.
    let mut cands: Vec<usize> = vec![0, 1];
    for k in 1..=4usize {
        let b = k * XXH32_BUFSIZE;
        cands.push(b - 1);
        cands.push(b);
        cands.push(b + 1);
    }
    for (i, len) in [16usize, 17, 31, 32, 33, 48, 63, 64, 65, 80, 100, 200]
        .into_iter()
        .enumerate()
    {
        let shape = i % N_SHAPES;
        let data = gen_shape(&mut rng, shape, len);
        let seed = rng.next_u32();
        let want = cmp32(
            &format!("XXH32 split ref len={}", len),
            data_ptr(&data),
            len,
            seed,
        );
        for &s1 in cands.iter().filter(|&&s| s <= len) {
            for &s2 in cands.iter().filter(|&&s| s <= len) {
                if s2 < s1 {
                    continue;
                }
                let chunks = [s1, s2 - s1, len - s2];
                let ctx = format!(
                    "XXH32 split len={} shape={} at {}/{} ({:?})",
                    len,
                    shape_name(shape),
                    s1,
                    s2,
                    chunks
                );
                let got = stream32_pattern(&duo, &ctx, seed, &data, &chunks);
                assert_eq!(got, want, "{}: digest {:#010x} != one-shot {:#010x}", ctx, got, want);
            }
        }
    }
}

#[test]
fn xxh64_streaming_boundary_splits() {
    let duo = Duo64::new();
    let mut rng = Rng::new(0x6400_B0D1);
    let mut cands: Vec<usize> = vec![0, 1];
    for k in 1..=4usize {
        let b = k * XXH64_BUFSIZE;
        cands.push(b - 1);
        cands.push(b);
        cands.push(b + 1);
    }
    for (i, len) in [32usize, 33, 63, 64, 65, 96, 127, 128, 129, 160, 200, 300]
        .into_iter()
        .enumerate()
    {
        let shape = i % N_SHAPES;
        let data = gen_shape(&mut rng, shape, len);
        let seed = rng.next_u64();
        let want = cmp64(
            &format!("XXH64 split ref len={}", len),
            data_ptr(&data),
            len,
            seed,
        );
        for &s1 in cands.iter().filter(|&&s| s <= len) {
            for &s2 in cands.iter().filter(|&&s| s <= len) {
                if s2 < s1 {
                    continue;
                }
                let chunks = [s1, s2 - s1, len - s2];
                let ctx = format!(
                    "XXH64 split len={} shape={} at {}/{} ({:?})",
                    len,
                    shape_name(shape),
                    s1,
                    s2,
                    chunks
                );
                let got = stream64_pattern(&duo, &ctx, seed, &data, &chunks);
                assert_eq!(got, want, "{}: digest {:#018x} != one-shot {:#018x}", ctx, got, want);
            }
        }
    }
}

// ===========================================================================
// Streaming: zero-length updates interleaved between real updates
// ===========================================================================

#[test]
fn xxh32_streaming_zero_length_updates() {
    let duo = Duo32::new();
    let mut rng = Rng::new(0x3200_0E70);
    for (i, len) in [0usize, 1, 5, 15, 16, 17, 31, 32, 33, 64, 100, 257, 1000]
        .into_iter()
        .enumerate()
    {
        let shape = i % N_SHAPES;
        let data = gen_shape(&mut rng, shape, len);
        let seed = rng.next_u32();
        let ctx = format!("XXH32 zerolen len={} shape={}", len, shape_name(shape));
        let want = cmp32(&ctx, data_ptr(&data), len, seed);

        duo.reset(&ctx, seed);
        let p = data_ptr(&data);
        let mut off = 0usize;
        let mut idx = 0usize;
        loop {
            // (a) zero-length update with a non-NULL pointer: defined no-op.
            assert_eq!(
                duo.update(&ctx, idx, unsafe { p.add(off) }, 0),
                XXH_OK,
                "{}: zero-length update on a valid pointer must be XXH_OK",
                ctx
            );
            idx += 1;
            assert_eq!(
                duo.update(&ctx, idx, ZERO_PAD.as_ptr(), 0),
                XXH_OK,
                "{}: zero-length update on an unrelated valid pointer must be XXH_OK",
                ctx
            );
            idx += 1;

            // (b) zero-length update with a NULL pointer. xxhash.c checks
            //     `input==NULL` FIRST and, because XXH_ACCEPT_NULL_INPUT_POINTER
            //     is 0, returns XXH_ERROR without touching the state — even for
            //     len == 0.
            let bc = duo.c.snapshot();
            let br = duo.r.snapshot();
            let rc = duo.update(&ctx, idx, std::ptr::null(), 0);
            assert_eq!(rc, XXH_ERROR, "{}: XXH32_update(state, NULL, 0) must be XXH_ERROR", ctx);
            assert_bytes_eq(
                &format!("{}: C XXH32_update(state,NULL,0) modified the state", ctx),
                &bc,
                duo.c.bytes(),
            );
            assert_bytes_eq(
                &format!("{}: Rust XXH32_update(state,NULL,0) modified the state", ctx),
                &br,
                duo.r.bytes(),
            );
            idx += 1;

            if off >= data.len() {
                break;
            }
            let n = rng.range(1, 20).min(data.len() - off);
            assert_eq!(duo.update(&ctx, idx, unsafe { p.add(off) }, n), XXH_OK, "{}", ctx);
            off += n;
            idx += 1;
        }
        let got = duo.digest(&ctx);
        assert_eq!(got, want, "{}: digest {:#010x} != one-shot {:#010x}", ctx, got, want);
    }
}

#[test]
fn xxh64_streaming_zero_length_updates() {
    let duo = Duo64::new();
    let mut rng = Rng::new(0x6400_0E70);
    for (i, len) in [0usize, 1, 5, 31, 32, 33, 63, 64, 65, 128, 200, 513, 2000]
        .into_iter()
        .enumerate()
    {
        let shape = i % N_SHAPES;
        let data = gen_shape(&mut rng, shape, len);
        let seed = rng.next_u64();
        let ctx = format!("XXH64 zerolen len={} shape={}", len, shape_name(shape));
        let want = cmp64(&ctx, data_ptr(&data), len, seed);

        duo.reset(&ctx, seed);
        let p = data_ptr(&data);
        let mut off = 0usize;
        let mut idx = 0usize;
        loop {
            assert_eq!(
                duo.update(&ctx, idx, unsafe { p.add(off) }, 0),
                XXH_OK,
                "{}: zero-length update on a valid pointer must be XXH_OK",
                ctx
            );
            idx += 1;
            assert_eq!(
                duo.update(&ctx, idx, ZERO_PAD.as_ptr(), 0),
                XXH_OK,
                "{}: zero-length update on an unrelated valid pointer must be XXH_OK",
                ctx
            );
            idx += 1;

            let bc = duo.c.snapshot();
            let br = duo.r.snapshot();
            let rc = duo.update(&ctx, idx, std::ptr::null(), 0);
            assert_eq!(rc, XXH_ERROR, "{}: XXH64_update(state, NULL, 0) must be XXH_ERROR", ctx);
            assert_bytes_eq(
                &format!("{}: C XXH64_update(state,NULL,0) modified the state", ctx),
                &bc,
                duo.c.bytes(),
            );
            assert_bytes_eq(
                &format!("{}: Rust XXH64_update(state,NULL,0) modified the state", ctx),
                &br,
                duo.r.bytes(),
            );
            idx += 1;

            if off >= data.len() {
                break;
            }
            let n = rng.range(1, 40).min(data.len() - off);
            assert_eq!(duo.update(&ctx, idx, unsafe { p.add(off) }, n), XXH_OK, "{}", ctx);
            off += n;
            idx += 1;
        }
        let got = duo.digest(&ctx);
        assert_eq!(got, want, "{}: digest {:#018x} != one-shot {:#018x}", ctx, got, want);
    }
}

// ===========================================================================
// digest() idempotence
// ===========================================================================

#[test]
fn xxh32_digest_idempotent() {
    let duo = Duo32::new();
    let mut rng = Rng::new(0x3200_D16E);
    for (i, len) in [0usize, 1, 7, 15, 16, 17, 33, 64, 129, 1000, 5000]
        .into_iter()
        .enumerate()
    {
        let shape = i % N_SHAPES;
        let data = gen_shape(&mut rng, shape, len);
        let seed = rng.next_u32();
        let ctx = format!("XXH32 idempotent len={} shape={}", len, shape_name(shape));
        let want = cmp32(&ctx, data_ptr(&data), len, seed);

        duo.reset(&ctx, seed);
        // Feed in a few chunks and digest after every one; digest must be
        // repeatable and must not disturb the running state.
        let p = data_ptr(&data);
        let mut off = 0usize;
        let mut idx = 0usize;
        while off < data.len() {
            let n = rng.range(1, 24).min(data.len() - off);
            duo.update(&ctx, idx, unsafe { p.add(off) }, n);
            off += n;
            idx += 1;
            // partial digest == one-shot over the prefix consumed so far
            let mid = duo.digest(&ctx);
            let mid2 = duo.digest(&ctx);
            let mid3 = duo.digest(&ctx);
            assert_eq!(mid, mid2, "{}: digest not idempotent at prefix {}", ctx, off);
            assert_eq!(mid, mid3, "{}: digest not idempotent at prefix {}", ctx, off);
            let want_mid = cmp32(&ctx, p, off, seed);
            assert_eq!(
                mid, want_mid,
                "{}: partial digest at {} = {:#010x} != one-shot prefix {:#010x}",
                ctx, off, mid, want_mid
            );
        }
        for k in 0..5 {
            let got = duo.digest(&ctx);
            assert_eq!(got, want, "{}: digest call #{} = {:#010x} != {:#010x}", ctx, k, got, want);
        }
    }
}

#[test]
fn xxh64_digest_idempotent() {
    let duo = Duo64::new();
    let mut rng = Rng::new(0x6400_D16E);
    for (i, len) in [0usize, 1, 7, 31, 32, 33, 65, 128, 257, 1000, 5000]
        .into_iter()
        .enumerate()
    {
        let shape = i % N_SHAPES;
        let data = gen_shape(&mut rng, shape, len);
        let seed = rng.next_u64();
        let ctx = format!("XXH64 idempotent len={} shape={}", len, shape_name(shape));
        let want = cmp64(&ctx, data_ptr(&data), len, seed);

        duo.reset(&ctx, seed);
        let p = data_ptr(&data);
        let mut off = 0usize;
        let mut idx = 0usize;
        while off < data.len() {
            let n = rng.range(1, 40).min(data.len() - off);
            duo.update(&ctx, idx, unsafe { p.add(off) }, n);
            off += n;
            idx += 1;
            let mid = duo.digest(&ctx);
            let mid2 = duo.digest(&ctx);
            let mid3 = duo.digest(&ctx);
            assert_eq!(mid, mid2, "{}: digest not idempotent at prefix {}", ctx, off);
            assert_eq!(mid, mid3, "{}: digest not idempotent at prefix {}", ctx, off);
            let want_mid = cmp64(&ctx, p, off, seed);
            assert_eq!(
                mid, want_mid,
                "{}: partial digest at {} = {:#018x} != one-shot prefix {:#018x}",
                ctx, off, mid, want_mid
            );
        }
        for k in 0..5 {
            let got = duo.digest(&ctx);
            assert_eq!(got, want, "{}: digest call #{} = {:#018x} != {:#018x}", ctx, k, got, want);
        }
    }
}

// ===========================================================================
// copyState()
// ===========================================================================

#[test]
fn xxh32_copy_state_midstream() {
    let mut rng = Rng::new(0x3200_C0FF);
    for (i, len) in [0usize, 1, 8, 15, 16, 17, 31, 32, 33, 64, 129, 300, 2000]
        .into_iter()
        .enumerate()
    {
        let shape = i % N_SHAPES;
        let prefix = gen_shape(&mut rng, shape, len);
        // Two DIFFERENT tails so the original and the copy diverge afterwards.
        let la = rng.range(0, 70);
        let tail_a = gen_shape(&mut rng, shape + 1, la);
        let lb = rng.range(0, 70);
        let tail_b = gen_shape(&mut rng, shape + 2, lb);
        let seed = rng.next_u32();

        let orig = Duo32::new();
        let copy = Duo32::new();
        let ctx = format!(
            "XXH32 copyState len={} shape={} tails={}/{}",
            len,
            shape_name(shape),
            tail_a.len(),
            tail_b.len()
        );

        orig.reset(&ctx, seed);
        if !prefix.is_empty() {
            orig.update(&ctx, 0, prefix.as_ptr(), prefix.len());
        }
        // The destination still holds the pristine sentinel pattern, so the
        // blob comparison inside copy_from also proves copyState overwrote the
        // FULL struct (all 48 bytes) in both libraries.
        copy.copy_from(&ctx, &orig);

        let d_orig = orig.digest(&ctx);
        let d_copy = copy.digest(&ctx);
        assert_eq!(
            d_orig, d_copy,
            "{}: digest of copy {:#010x} != digest of original {:#010x}",
            ctx, d_copy, d_orig
        );

        // Continue BOTH with different data.
        if !tail_a.is_empty() {
            orig.update(&ctx, 1, tail_a.as_ptr(), tail_a.len());
        }
        if !tail_b.is_empty() {
            copy.update(&ctx, 1, tail_b.as_ptr(), tail_b.len());
        }

        let mut full_a = prefix.clone();
        full_a.extend_from_slice(&tail_a);
        let mut full_b = prefix.clone();
        full_b.extend_from_slice(&tail_b);

        let want_a = cmp32(&ctx, data_ptr(&full_a), full_a.len(), seed);
        let want_b = cmp32(&ctx, data_ptr(&full_b), full_b.len(), seed);
        let got_a = orig.digest(&ctx);
        let got_b = copy.digest(&ctx);
        assert_eq!(got_a, want_a, "{}: original digest {:#010x} != {:#010x}", ctx, got_a, want_a);
        assert_eq!(got_b, want_b, "{}: copy digest {:#010x} != {:#010x}", ctx, got_b, want_b);

        // Copying back must restore an exact clone.
        copy.copy_from(&ctx, &orig);
        let restored = copy.digest(&ctx);
        assert_eq!(restored, got_a, "{}: copy-back digest {:#010x} != {:#010x}", ctx, restored, got_a);
    }
}

#[test]
fn xxh64_copy_state_midstream() {
    let mut rng = Rng::new(0x6400_C0FF);
    for (i, len) in [0usize, 1, 8, 31, 32, 33, 63, 64, 65, 128, 257, 400, 2000]
        .into_iter()
        .enumerate()
    {
        let shape = i % N_SHAPES;
        let prefix = gen_shape(&mut rng, shape, len);
        let la = rng.range(0, 130);
        let tail_a = gen_shape(&mut rng, shape + 1, la);
        let lb = rng.range(0, 130);
        let tail_b = gen_shape(&mut rng, shape + 2, lb);
        let seed = rng.next_u64();

        let orig = Duo64::new();
        let copy = Duo64::new();
        let ctx = format!(
            "XXH64 copyState len={} shape={} tails={}/{}",
            len,
            shape_name(shape),
            tail_a.len(),
            tail_b.len()
        );

        orig.reset(&ctx, seed);
        if !prefix.is_empty() {
            orig.update(&ctx, 0, prefix.as_ptr(), prefix.len());
        }
        copy.copy_from(&ctx, &orig);

        let d_orig = orig.digest(&ctx);
        let d_copy = copy.digest(&ctx);
        assert_eq!(
            d_orig, d_copy,
            "{}: digest of copy {:#018x} != digest of original {:#018x}",
            ctx, d_copy, d_orig
        );

        if !tail_a.is_empty() {
            orig.update(&ctx, 1, tail_a.as_ptr(), tail_a.len());
        }
        if !tail_b.is_empty() {
            copy.update(&ctx, 1, tail_b.as_ptr(), tail_b.len());
        }

        let mut full_a = prefix.clone();
        full_a.extend_from_slice(&tail_a);
        let mut full_b = prefix.clone();
        full_b.extend_from_slice(&tail_b);

        let want_a = cmp64(&ctx, data_ptr(&full_a), full_a.len(), seed);
        let want_b = cmp64(&ctx, data_ptr(&full_b), full_b.len(), seed);
        let got_a = orig.digest(&ctx);
        let got_b = copy.digest(&ctx);
        assert_eq!(got_a, want_a, "{}: original digest {:#018x} != {:#018x}", ctx, got_a, want_a);
        assert_eq!(got_b, want_b, "{}: copy digest {:#018x} != {:#018x}", ctx, got_b, want_b);

        copy.copy_from(&ctx, &orig);
        let restored = copy.digest(&ctx);
        assert_eq!(restored, got_a, "{}: copy-back digest {:#018x} != {:#018x}", ctx, restored, got_a);
    }
}

// ===========================================================================
// Streaming from deliberately misaligned source buffers
// ===========================================================================

#[test]
fn xxh32_streaming_misaligned_chunks() {
    let duo = Duo32::new();
    let mut rng = Rng::new(0x3200_A116);
    for (i, len) in [1usize, 16, 17, 33, 64, 65, 200, 1000].into_iter().enumerate() {
        let shape = i % N_SHAPES;
        let data = gen_shape(&mut rng, shape, len);
        let seed = rng.next_u32();
        for offset in 0..=8usize {
            let buf = aligned_copy(&data, 16, offset);
            let want = cmp32(
                &format!("XXH32 misaligned ref len={} off={}", len, offset),
                buf.as_ptr(),
                len,
                seed,
            );
            for chunk in [0usize, 1, 3, 7, 16, 17, 33] {
                let ctx = format!(
                    "XXH32 misaligned stream len={} off={} chunk={} shape={}",
                    len,
                    offset,
                    chunk,
                    shape_name(shape)
                );
                duo.reset(&ctx, seed);
                let p = buf.as_ptr();
                if chunk == 0 {
                    duo.update(&ctx, 0, p, len);
                } else {
                    let mut off = 0usize;
                    let mut idx = 0usize;
                    while off < len {
                        let n = chunk.min(len - off);
                        duo.update(&ctx, idx, unsafe { p.add(off) }, n);
                        off += n;
                        idx += 1;
                    }
                }
                let got = duo.digest(&ctx);
                assert_eq!(got, want, "{}: digest {:#010x} != one-shot {:#010x}", ctx, got, want);
            }
        }
    }
}

#[test]
fn xxh64_streaming_misaligned_chunks() {
    let duo = Duo64::new();
    let mut rng = Rng::new(0x6400_A116);
    for (i, len) in [1usize, 32, 33, 65, 128, 129, 300, 1000].into_iter().enumerate() {
        let shape = i % N_SHAPES;
        let data = gen_shape(&mut rng, shape, len);
        let seed = rng.next_u64();
        for offset in 0..=8usize {
            let buf = aligned_copy(&data, 16, offset);
            let want = cmp64(
                &format!("XXH64 misaligned ref len={} off={}", len, offset),
                buf.as_ptr(),
                len,
                seed,
            );
            for chunk in [0usize, 1, 3, 7, 32, 33, 65] {
                let ctx = format!(
                    "XXH64 misaligned stream len={} off={} chunk={} shape={}",
                    len,
                    offset,
                    chunk,
                    shape_name(shape)
                );
                duo.reset(&ctx, seed);
                let p = buf.as_ptr();
                if chunk == 0 {
                    duo.update(&ctx, 0, p, len);
                } else {
                    let mut off = 0usize;
                    let mut idx = 0usize;
                    while off < len {
                        let n = chunk.min(len - off);
                        duo.update(&ctx, idx, unsafe { p.add(off) }, n);
                        off += n;
                        idx += 1;
                    }
                }
                let got = duo.digest(&ctx);
                assert_eq!(got, want, "{}: digest {:#018x} != one-shot {:#018x}", ctx, got, want);
            }
        }
    }
}

// ===========================================================================
// Canonical representation
// ===========================================================================

#[test]
fn xxh32_canonical_from_hash_and_back() {
    let (c_from, r_from) = both::<FnCanon32From>("LZ4_XXH32_canonicalFromHash");
    let (c_to, r_to) = both::<FnCanon32To>("LZ4_XXH32_hashFromCanonical");

    let mut hashes: Vec<u32> = vec![
        0,
        1,
        2,
        0xFF,
        0x100,
        0xFF00,
        0x00FF_0000,
        0x8000_0000,
        0x7FFF_FFFF,
        0xFFFF_FFFF,
        0x0102_0304,
        0xDEAD_BEEF,
    ];
    let mut rng = Rng::new(0x3200_CA40);
    for _ in 0..300 {
        hashes.push(rng.next_u32());
    }

    for h in hashes {
        // Same sentinel fill on BOTH sides, then compare the FULL buffer.
        let mut cc = XXH32_canonical_t { digest: [SENTINEL; 4] };
        let mut cr = XXH32_canonical_t { digest: [SENTINEL; 4] };
        unsafe {
            c_from(&mut cc, h);
            r_from(&mut cr, h);
        }
        assert_bytes_eq(
            &format!("LZ4_XXH32_canonicalFromHash({:#010x})", h),
            &cc.digest,
            &cr.digest,
        );
        // canonical form is big-endian by definition
        assert_eq!(
            cc.digest,
            h.to_be_bytes(),
            "C canonicalFromHash({:#010x}) is not big-endian: {:02x?}",
            h,
            cc.digest
        );

        // round-trip hash -> canonical -> hash
        let bc = unsafe { c_to(&cc) };
        let br = unsafe { r_to(&cr) };
        assert_eq!(
            bc, br,
            "LZ4_XXH32_hashFromCanonical({:02x?}) C={:#010x} Rust={:#010x}",
            cc.digest, bc, br
        );
        assert_eq!(bc, h, "XXH32 canonical round-trip lost data: {:#010x} -> {:#010x}", h, bc);
    }

    // hashFromCanonical over explicit and random canonical inputs
    let mut canons: Vec<[u8; 4]> = vec![
        [0x00; 4],
        [0xFF; 4],
        [0x00, 0x00, 0x00, 0x01],
        [0x01, 0x00, 0x00, 0x00],
        [0x80, 0x00, 0x00, 0x00],
        [0xDE, 0xAD, 0xBE, 0xEF],
    ];
    for _ in 0..300 {
        canons.push([rng.byte(), rng.byte(), rng.byte(), rng.byte()]);
    }
    for d in canons {
        let cc = XXH32_canonical_t { digest: d };
        let cr = XXH32_canonical_t { digest: d };
        let hc = unsafe { c_to(&cc) };
        let hr = unsafe { r_to(&cr) };
        assert_eq!(
            hc, hr,
            "LZ4_XXH32_hashFromCanonical({:02x?}) C={:#010x} Rust={:#010x}",
            d, hc, hr
        );
        assert_eq!(hc, u32::from_be_bytes(d), "C hashFromCanonical({:02x?}) is not big-endian", d);

        // round-trip canonical -> hash -> canonical
        let mut back_c = XXH32_canonical_t { digest: [SENTINEL; 4] };
        let mut back_r = XXH32_canonical_t { digest: [SENTINEL; 4] };
        unsafe {
            c_from(&mut back_c, hc);
            r_from(&mut back_r, hr);
        }
        assert_bytes_eq(
            &format!("XXH32 canonical->hash->canonical for {:02x?}", d),
            &back_c.digest,
            &back_r.digest,
        );
        assert_eq!(back_c.digest, d, "XXH32 canonical round-trip lost data for {:02x?}", d);
    }
}

#[test]
fn xxh64_canonical_from_hash_and_back() {
    let (c_from, r_from) = both::<FnCanon64From>("LZ4_XXH64_canonicalFromHash");
    let (c_to, r_to) = both::<FnCanon64To>("LZ4_XXH64_hashFromCanonical");

    let mut hashes: Vec<u64> = vec![
        0,
        1,
        0xFF,
        0xFF00,
        0xFFFF_FFFF,
        0x1_0000_0000,
        0x8000_0000_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x0102_0304_0506_0708,
        0xDEAD_BEEF_CAFE_F00D,
    ];
    let mut rng = Rng::new(0x6400_CA40);
    for _ in 0..300 {
        hashes.push(rng.next_u64());
    }

    for h in hashes {
        let mut cc = XXH64_canonical_t { digest: [SENTINEL; 8] };
        let mut cr = XXH64_canonical_t { digest: [SENTINEL; 8] };
        unsafe {
            c_from(&mut cc, h);
            r_from(&mut cr, h);
        }
        assert_bytes_eq(
            &format!("LZ4_XXH64_canonicalFromHash({:#018x})", h),
            &cc.digest,
            &cr.digest,
        );
        assert_eq!(
            cc.digest,
            h.to_be_bytes(),
            "C canonicalFromHash({:#018x}) is not big-endian: {:02x?}",
            h,
            cc.digest
        );

        let bc = unsafe { c_to(&cc) };
        let br = unsafe { r_to(&cr) };
        assert_eq!(
            bc, br,
            "LZ4_XXH64_hashFromCanonical({:02x?}) C={:#018x} Rust={:#018x}",
            cc.digest, bc, br
        );
        assert_eq!(bc, h, "XXH64 canonical round-trip lost data: {:#018x} -> {:#018x}", h, bc);
    }

    let mut canons: Vec<[u8; 8]> = vec![
        [0x00; 8],
        [0xFF; 8],
        [0, 0, 0, 0, 0, 0, 0, 1],
        [1, 0, 0, 0, 0, 0, 0, 0],
        [0x80, 0, 0, 0, 0, 0, 0, 0],
        [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xF0, 0x0D],
    ];
    for _ in 0..300 {
        let mut d = [0u8; 8];
        for b in d.iter_mut() {
            *b = rng.byte();
        }
        canons.push(d);
    }
    for d in canons {
        let cc = XXH64_canonical_t { digest: d };
        let cr = XXH64_canonical_t { digest: d };
        let hc = unsafe { c_to(&cc) };
        let hr = unsafe { r_to(&cr) };
        assert_eq!(
            hc, hr,
            "LZ4_XXH64_hashFromCanonical({:02x?}) C={:#018x} Rust={:#018x}",
            d, hc, hr
        );
        assert_eq!(hc, u64::from_be_bytes(d), "C hashFromCanonical({:02x?}) is not big-endian", d);

        let mut back_c = XXH64_canonical_t { digest: [SENTINEL; 8] };
        let mut back_r = XXH64_canonical_t { digest: [SENTINEL; 8] };
        unsafe {
            c_from(&mut back_c, hc);
            r_from(&mut back_r, hr);
        }
        assert_bytes_eq(
            &format!("XXH64 canonical->hash->canonical for {:02x?}", d),
            &back_c.digest,
            &back_r.digest,
        );
        assert_eq!(back_c.digest, d, "XXH64 canonical round-trip lost data for {:02x?}", d);
    }
}

// ===========================================================================
// Phase C — error paths
//
// Every `XXH_ERROR` return and every NULL check reachable in xxhash.c is
// covered here.  There is exactly ONE `XXH_ERROR` return per algorithm:
// the `if (input==NULL) return XXH_ERROR;` at the top of
// `XXH32_update_endian` / `XXH64_update_endian` (xxhash.c:454-459 and
// :914-919), taken because `XXH_ACCEPT_NULL_INPUT_POINTER` is 0 in this build.
//
// NOT tested, because xxhash.c has NO NULL check for the *state* pointer and
// would dereference it, i.e. these are guaranteed segfaults in the C ground
// truth rather than defined behaviour:
//   * `LZ4_XXH32_reset(NULL, seed)`  / `LZ4_XXH64_reset(NULL, seed)`
//        -> `memcpy(statePtr, &state, ...)` with statePtr == NULL
//           (xxhash.c:446 / :907)
//   * `LZ4_XXH32_update(NULL, buf, len)` / `LZ4_XXH64_update(NULL, ...)`
//        -> `state->total_len_32 += (unsigned)len;` (xxhash.c:464 / :924).
//           Note the NULL check guards `input`, NOT `state`.
//   * `LZ4_XXH32_digest(NULL)` / `LZ4_XXH64_digest(NULL)`
//        -> `state->large_len` / `state->total_len` (xxhash.c:531 / :985)
//   * `LZ4_XXH32_copyState(NULL, src)` / `..._copyState(dst, NULL)`
//        -> `memcpy` on a NULL operand (xxhash.c:434 / :895)
//   * `LZ4_XXH32(NULL, len, seed)` with len != 0
//        -> reads through the NULL pointer in XXH32_finalize / the main loop.
// ===========================================================================

#[test]
fn xxh_error_paths_null_pointers() {
    // ---- freeState(NULL): `XXH_free(NULL)` is `free(NULL)`, a defined no-op,
    //      and the function unconditionally returns XXH_OK. -----------------
    let (c_f32, r_f32) = both::<FnFreeState>("LZ4_XXH32_freeState");
    let rc = unsafe { c_f32(std::ptr::null_mut()) };
    let rr = unsafe { r_f32(std::ptr::null_mut()) };
    assert_eq!(rc, XXH_OK, "C LZ4_XXH32_freeState(NULL) -> {}", rc);
    assert_eq!(rc, rr, "LZ4_XXH32_freeState(NULL) C={} Rust={}", rc, rr);

    let (c_f64, r_f64) = both::<FnFreeState>("LZ4_XXH64_freeState");
    let rc = unsafe { c_f64(std::ptr::null_mut()) };
    let rr = unsafe { r_f64(std::ptr::null_mut()) };
    assert_eq!(rc, XXH_OK, "C LZ4_XXH64_freeState(NULL) -> {}", rc);
    assert_eq!(rc, rr, "LZ4_XXH64_freeState(NULL) C={} Rust={}", rc, rr);

    // ---- one-shot with a NULL input and length 0.
    //      XXH32_endian_align never dereferences `p` when len == 0:
    //      len < 16 -> h32 = seed + PRIME32_5, then XXH32_finalize with
    //      (len & 15) == 0 falls straight into `case 0: return avalanche`.
    //      So this IS defined here (only len != 0 would crash).
    for seed in seeds32() {
        let h = cmp32("LZ4_XXH32(NULL, 0, seed)", std::ptr::null(), 0, seed);
        // must equal hashing a zero-length buffer at a valid address
        let h2 = cmp32("LZ4_XXH32(valid, 0, seed)", ZERO_PAD.as_ptr(), 0, seed);
        assert_eq!(
            h, h2,
            "LZ4_XXH32(NULL,0,{:#010x})={:#010x} != LZ4_XXH32(ptr,0,..)={:#010x}",
            seed, h, h2
        );
    }
    for seed in seeds64() {
        let h = cmp64("LZ4_XXH64(NULL, 0, seed)", std::ptr::null(), 0, seed);
        let h2 = cmp64("LZ4_XXH64(valid, 0, seed)", ZERO_PAD.as_ptr(), 0, seed);
        assert_eq!(
            h, h2,
            "LZ4_XXH64(NULL,0,{:#018x})={:#018x} != LZ4_XXH64(ptr,0,..)={:#018x}",
            seed, h, h2
        );
    }

    // ---- update(state, NULL, len) for len == 0 AND len != 0.
    //      `if (input==NULL) return XXH_ERROR;` runs before any dereference of
    //      `input` and before any write to `state`, so both are safe and must
    //      report the identical error code and leave the state untouched.
    let payload = {
        let mut rng = Rng::new(0xE770_0001);
        gen_random(&mut rng, 512)
    };

    let d32 = Duo32::new();
    d32.reset("error-path XXH32", 0x5EED_1234);
    for &len in &[0usize, 1, 2, 15, 16, 17, 31, 32, 100, 1024, usize::MAX / 2] {
        let bc = d32.c.snapshot();
        let br = d32.r.snapshot();
        let rc = d32.update("error-path XXH32", 0, std::ptr::null(), len);
        assert_eq!(
            rc, XXH_ERROR,
            "LZ4_XXH32_update(state, NULL, {}) must return XXH_ERROR, got {}",
            len, rc
        );
        assert_bytes_eq(
            &format!("C LZ4_XXH32_update(state,NULL,{}) modified the state", len),
            &bc,
            d32.c.bytes(),
        );
        assert_bytes_eq(
            &format!("Rust LZ4_XXH32_update(state,NULL,{}) modified the state", len),
            &br,
            d32.r.bytes(),
        );
    }
    // ... and the stream is still usable and correct afterwards.
    d32.update("error-path XXH32", 1, payload.as_ptr(), payload.len());
    let got = d32.digest("error-path XXH32");
    let want = cmp32("error-path XXH32 ref", payload.as_ptr(), payload.len(), 0x5EED_1234);
    assert_eq!(
        got, want,
        "XXH32 stream corrupted by failed updates: {:#010x} != {:#010x}",
        got, want
    );

    let d64 = Duo64::new();
    d64.reset("error-path XXH64", 0x5EED_1234_5678_9ABC);
    for &len in &[0usize, 1, 2, 31, 32, 33, 63, 64, 100, 1024, usize::MAX / 2] {
        let bc = d64.c.snapshot();
        let br = d64.r.snapshot();
        let rc = d64.update("error-path XXH64", 0, std::ptr::null(), len);
        assert_eq!(
            rc, XXH_ERROR,
            "LZ4_XXH64_update(state, NULL, {}) must return XXH_ERROR, got {}",
            len, rc
        );
        assert_bytes_eq(
            &format!("C LZ4_XXH64_update(state,NULL,{}) modified the state", len),
            &bc,
            d64.c.bytes(),
        );
        assert_bytes_eq(
            &format!("Rust LZ4_XXH64_update(state,NULL,{}) modified the state", len),
            &br,
            d64.r.bytes(),
        );
    }
    d64.update("error-path XXH64", 1, payload.as_ptr(), payload.len());
    let got = d64.digest("error-path XXH64");
    let want = cmp64(
        "error-path XXH64 ref",
        payload.as_ptr(),
        payload.len(),
        0x5EED_1234_5678_9ABC,
    );
    assert_eq!(
        got, want,
        "XXH64 stream corrupted by failed updates: {:#018x} != {:#018x}",
        got, want
    );

    // ---- reset() is the only other function returning XXH_errorcode and it
    //      can only ever return XXH_OK; verify that for a wide seed range.
    let mut rng = Rng::new(0xE770_0002);
    for _ in 0..32 {
        let s32 = rng.next_u32();
        d32.reset("error-path reset32", s32);
        let s64 = rng.next_u64();
        d64.reset("error-path reset64", s64);
    }
}
