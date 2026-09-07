//! Differential tests: load BOTH the C `.so` and the Rust `.so` via
//! `libloading` and compare their `rev16` outputs through the FFI boundary.
//!
//! The Rust function is NEVER called directly — it is always resolved as an
//! exported symbol from `librev16_lib.so`, exactly as an external C consumer
//! would, so the `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! Phase B rows come from `CONFIGS.md`; Phase C rows from `ERRORS.md`.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

type Rev16 = unsafe extern "C" fn(u32) -> u32;

/// Workspace root = parent of the `translation` crate directory.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    let build = workspace_root().join("c_src/build");
    // The CMake project name is derived from the parent directory name, so the
    // `.so` file name is not fixed. Discover it instead of hardcoding.
    let mut found: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e} — build the C lib first", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension().map(|x| x == "so").unwrap_or(false)
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("lib"))
                    .unwrap_or(false)
        })
        .collect();
    found.sort();
    assert_eq!(
        found.len(),
        1,
        "expected exactly one C shared object in {}, found {:?}",
        build.display(),
        found
    );
    found.pop().unwrap()
}

fn rust_lib_path() -> PathBuf {
    // Prefer an explicit path from the driver script, which guarantees the
    // cdylib was rebuilt from the current sources.
    if let Some(p) = std::env::var_os("REV16_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "REV16_RUST_SO={} is not a file", p.display());
        assert_fresh(&p);
        return p;
    }

    // The test binary lives in <profile-dir>/deps/, so the cdylib built for
    // this same profile is one level up. Fall back to debug/release.
    let exe = std::env::current_exe().expect("current_exe");
    let mut candidates = Vec::new();
    if let Some(deps) = exe.parent() {
        if let Some(profile) = deps.parent() {
            candidates.push(profile.join("librev16_lib.so"));
        }
    }
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    candidates.push(target.join("debug/librev16_lib.so"));
    candidates.push(target.join("release/librev16_lib.so"));

    for c in &candidates {
        if c.is_file() {
            assert_fresh(c);
            return c.clone();
        }
    }
    panic!("could not locate librev16_lib.so; looked in {candidates:?}");
}

/// `cargo test` does NOT rebuild the `cdylib` target, so a `.so` left over from
/// an earlier `cargo build` can silently be tested instead of the current
/// source. That would make every differential test vacuously pass. Refuse to
/// run against a `.so` older than the Rust source it is supposed to contain.
fn assert_fresh(so: &Path) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let mtime = |p: &Path| {
        std::fs::metadata(p)
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
            .modified()
            .unwrap_or_else(|e| panic!("mtime {}: {e}", p.display()))
    };
    let so_t = mtime(so);
    let src_t = mtime(&src);
    assert!(
        so_t >= src_t,
        "STALE ARTIFACT: {} is older than {}.\n\
         `cargo test` does not rebuild the cdylib. Run \
         `cargo build` (or ./run_verification.sh) first, or set REV16_RUST_SO.",
        so.display(),
        src.display()
    );
}

/// Both libraries, loaded once per test.
struct Pair {
    _c: Library,
    _r: Library,
    c: Rev16,
    r: Rev16,
    /// Number of differential comparisons performed. Asserted at the end of
    /// each row so a loop that silently does no work cannot pass vacuously.
    calls: std::cell::Cell<u64>,
    /// Running checksum over every (input, output) pair, so the comparisons
    /// provably depend on the values produced.
    sum: std::cell::Cell<u64>,
}

impl Pair {
    fn load() -> Self {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        unsafe {
            let _c = Library::new(&cp).unwrap_or_else(|e| panic!("load {}: {e}", cp.display()));
            let _r = Library::new(&rp).unwrap_or_else(|e| panic!("load {}: {e}", rp.display()));
            let cs: Symbol<Rev16> = _c
                .get(b"rev16\0")
                .unwrap_or_else(|e| panic!("C rev16 symbol: {e}"));
            let rs: Symbol<Rev16> = _r
                .get(b"rev16\0")
                .unwrap_or_else(|e| panic!("Rust rev16 symbol: {e}"));
            let c = *cs;
            let r = *rs;
            Pair {
                _c,
                _r,
                c,
                r,
                calls: std::cell::Cell::new(0),
                sum: std::cell::Cell::new(0),
            }
        }
    }

    /// Call both and assert bit-identical `uint32_t` results.
    #[track_caller]
    fn check(&self, a: u32) {
        let cv = unsafe { (self.c)(a) };
        let rv = unsafe { (self.r)(a) };
        assert_eq!(
            cv, rv,
            "rev16 divergence for input {a:#010x}: C = {cv:#010x}, Rust = {rv:#010x}"
        );
        self.calls.set(self.calls.get() + 1);
        self.sum.set(
            self.sum
                .get()
                .wrapping_mul(0x100_0000_01B3)
                .wrapping_add(((a as u64) << 32) | cv as u64),
        );
    }

    /// Assert the row actually performed the expected amount of work and that
    /// the outputs were non-degenerate.
    #[track_caller]
    fn expect_calls(&self, n: u64) {
        assert_eq!(
            self.calls.get(),
            n,
            "row performed {} differential comparisons, expected {n}",
            self.calls.get()
        );
        assert_ne!(self.sum.get(), 0, "checksum degenerate — outputs ignored?");
    }
}

/// SplitMix64 — deterministic, fixed seed, reproducible across runs.
struct Rng(u64);

impl Rng {
    fn new() -> Self {
        Rng(0x243F_6A88_85A3_08D3)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    fn next_u16(&mut self) -> u32 {
        (self.next_u64() >> 48) as u32
    }
}

const MASKS: [u32; 8] = [
    0xAAAA, 0x5555, 0xCCCC, 0x3333, 0xF0F0, 0x0F0F, 0xFF00, 0x00FF,
];

// ---------------------------------------------------------------------------
// Phase B — valid-path differential tests, one test per CONFIGS.md row
// ---------------------------------------------------------------------------

/// Row 1: exhaustive over the entire low half, upper half zero.
#[test]
fn configs_row01_exhaustive_low_half() {
    let p = Pair::load();
    for a in 0u32..=0xFFFF {
        p.check(a);
    }
    p.expect_calls(65_536);
}

/// Row 2: exhaustive over the low half with the upper half saturated.
#[test]
fn configs_row02_exhaustive_low_half_high_saturated() {
    let p = Pair::load();
    for lo in 0u32..=0xFFFF {
        p.check(0xFFFF_0000 | lo);
    }
    p.expect_calls(65_536);
}

/// Row 3: randomized over the full 32-bit domain.
#[test]
fn configs_row03_randomized_full_domain() {
    let p = Pair::load();
    let mut rng = Rng::new();
    for _ in 0..2_000_000 {
        p.check(rng.next_u32());
    }
    p.expect_calls(2_000_000);
}

/// Row 4: random non-zero upper half crossed with random low half.
#[test]
fn configs_row04_randomized_half_cross_product() {
    let p = Pair::load();
    let mut rng = Rng::new();
    for _ in 0..200_000 {
        let hi = rng.next_u16();
        let lo = rng.next_u16();
        p.check((hi << 16) | lo);
        // and the same low half with a guaranteed non-zero upper half
        p.check(((hi | 1) << 16) | lo);
    }
    p.expect_calls(400_000);
}

/// Row 5: every single-bit input across the full 32-bit width.
#[test]
fn configs_row05_single_bit_set() {
    let p = Pair::load();
    for k in 0..32 {
        p.check(1u32 << k);
    }
    p.expect_calls(32);
}

/// Row 6: every single-bit-clear input across the full 32-bit width.
#[test]
fn configs_row06_single_bit_clear() {
    let p = Pair::load();
    for k in 0..32 {
        p.check(!(1u32 << k));
    }
    p.expect_calls(32);
}

/// Row 7: the mask constants and their immediate neighbours.
#[test]
fn configs_row07_mask_constants_and_neighbours() {
    let p = Pair::load();
    for m in MASKS {
        p.check(m.wrapping_sub(1));
        p.check(m);
        p.check(m.wrapping_add(1));
        // also with junk in the upper half
        p.check(0xDEAD_0000 | m);
    }
    p.expect_calls(32);
}

/// Row 8: byte/nibble boundaries and popcount 0 / 1 / 15 / 16 shapes.
#[test]
fn configs_row08_boundary_and_popcount_shapes() {
    let p = Pair::load();
    for a in [0u32, 1, 0xFFFF, 0x8000, 0x0100, 0x1000, 0x00FF, 0xFF00] {
        p.check(a);
    }
    // popcount 0 and 16
    p.check(0x0000);
    p.check(0xFFFF);
    // popcount 1 and 15 over the low half
    for k in 0..16 {
        p.check(1u32 << k);
        p.check(0xFFFF & !(1u32 << k));
    }
    p.expect_calls(42);
}

/// Row 9: shapes that would expose a mis-implemented 16-bit truncation or a
/// `<<` that overflows past bit 15.
#[test]
fn configs_row09_left_shift_overflow_shapes() {
    let p = Pair::load();
    for a in [0x5555u32, 0xAAAA, 0xFFFF, 0x8888, 0xC000, 0xE000, 0xF000] {
        p.check(a);
    }
    let mut rng = Rng::new();
    for _ in 0..100_000 {
        // sample only values with the top low-half bit set
        p.check(0x8000 | (rng.next_u16() & 0x7FFF));
    }
    p.expect_calls(100_007);
}

/// Row 10: byte-order sensitive shapes.
#[test]
fn configs_row10_byte_order_shapes() {
    let p = Pair::load();
    for a in [0x00FFu32, 0xFF00, 0x1234, 0x00AB, 0xAB00, 0x0102, 0x0201] {
        p.check(a);
    }
    let mut rng = Rng::new();
    let mut n = 0;
    while n < 100_000 {
        let v = rng.next_u16();
        if (v & 0xFF) != ((v >> 8) & 0xFF) {
            p.check(v);
            n += 1;
        }
    }
    p.expect_calls(100_007);
}

/// Row 11: composed / repeated driving through the FFI — confirms there is no
/// hidden state and that C and Rust agree at every step of a chain.
#[test]
fn configs_row11_composed_repeated_calls() {
    let p = Pair::load();
    let mut rng = Rng::new();
    for _ in 0..100_000 {
        let x = rng.next_u32();
        // first application must agree
        p.check(x);
        let c1 = unsafe { (p.c)(x) };
        let r1 = unsafe { (p.r)(x) };
        assert_eq!(c1, r1);
        // second application, fed each library its own previous output
        let c2 = unsafe { (p.c)(c1) };
        let r2 = unsafe { (p.r)(r1) };
        assert_eq!(
            c2, r2,
            "composed rev16(rev16({x:#010x})) diverged: C = {c2:#010x}, Rust = {r2:#010x}"
        );
        // and cross-fed, to catch an asymmetric wrapper
        let cx = unsafe { (p.c)(r1) };
        let rx = unsafe { (p.r)(c1) };
        assert_eq!(cx, rx);
    }
    p.expect_calls(100_000);
}

// ---------------------------------------------------------------------------
// Phase C — error/boundary-path differential tests (ERRORS.md)
//
// The C API has ZERO rejection paths (see ERRORS.md: no asserts, no error
// returns, no range checks, no pointer or enum parameters). `rev16` is total
// over all 2^32 inputs. What remains testable are the generic FFI boundaries,
// rows C1..C8 of ERRORS.md — asserted here to return the SAME concrete value
// from both libraries, not merely "both survived".
// ---------------------------------------------------------------------------

#[test]
fn phase_c_generic_boundaries() {
    let p = Pair::load();

    // C1 zero / minimum, C2 maximum
    p.check(0x0000_0000);
    p.check(0xFFFF_FFFF);

    // C3 one step past the implied 16-bit range
    p.check(0x0000_FFFF);
    p.check(0x0001_0000);
    p.check(0x0001_FFFF);

    // C4 high half saturated, low half swept
    for lo in 0u32..=0xFFFF {
        p.check(0xFFFF_0000 | lo);
    }

    // C5 / C6 every single-bit set and clear across all 32 bits
    for k in 0..32 {
        p.check(1u32 << k);
        p.check(!(1u32 << k));
    }

    // C7 sign-bit and signed-limit reinterpretations crossing the boundary as
    // unsigned (C enums/ints accept any bit pattern; so does uint32_t)
    for a in [
        0x8000_0000u32,
        0x7FFF_FFFF,
        (-1i32) as u32,
        (i32::MIN) as u32,
        (i32::MAX) as u32,
        u32::MAX - 1,
    ] {
        p.check(a);
    }

    // C8 mask edges +/- 1
    for m in MASKS {
        p.check(m.wrapping_sub(1));
        p.check(m);
        p.check(m.wrapping_add(1));
    }
    p.expect_calls(65_635);
}

/// Belt-and-braces: an exhaustive sweep of the upper half with a fixed low
/// half, proving the discard semantics hold for every possible high pattern.
#[test]
fn phase_c_exhaustive_upper_half_discard() {
    let p = Pair::load();
    for hi in 0u32..=0xFFFF {
        p.check((hi << 16) | 0x1234);
    }
    p.expect_calls(65_536);
}

/// Sanity: both libraries really do export the symbol under the exact name,
/// and a missing symbol is a hard failure rather than a silent skip.
#[test]
fn phase_d_both_export_rev16_by_exact_name() {
    let cp = c_lib_path();
    let rp = rust_lib_path();
    unsafe {
        let c = Library::new(&cp).unwrap();
        let r = Library::new(&rp).unwrap();
        let _: Symbol<Rev16> = c.get(b"rev16\0").expect("C .so must export rev16");
        let _: Symbol<Rev16> = r.get(b"rev16\0").expect("Rust .so must export rev16");
        // A name that must NOT exist, so the lookup above is meaningful.
        assert!(r.get::<Rev16>(b"rev16_rust\0").is_err());
        assert!(r.get::<Rev16>(b"_ZN10translation5rev16E\0").is_err());
    }
}
