//! Differential tests: the C shared library vs. the Rust cdylib.
//!
//! Both libraries are loaded at runtime with `libloading` and every call goes
//! through the `.so` export table. The Rust function is *never* called
//! directly, so these tests also exercise the `#[unsafe(no_mangle)] extern "C"`
//! wrapper and the real C ABI, exactly as an external consumer would.
//!
//! Phase B (valid paths) is driven by `CONFIGS.md`; Phase C (boundaries) by
//! `ERRORS.md`.

use libloading::{Library, Symbol};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

type Rev16Fn = unsafe extern "C" fn(u32) -> u32;

/// The crate root (`translation/`).
fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The repository root, i.e. the parent of `translation/`.
fn repo_root() -> PathBuf {
    crate_root()
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

/// Finds the single `.so` produced by the C CMake build.
///
/// The CMake project name is derived from the parent directory name, so the
/// file name is not fixed; scan `c_src/build` for any `lib*.so` instead of
/// hard-coding it.
fn find_c_library() -> PathBuf {
    let build_dir = repo_root().join("c_src").join("build");
    assert!(
        build_dir.is_dir(),
        "C build directory {} does not exist. Build it first with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        build_dir.display()
    );

    let mut found: Vec<PathBuf> = std::fs::read_dir(&build_dir)
        .expect("c_src/build must be readable")
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.extension().and_then(|e| e.to_str()) == Some("so")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("lib"))
        })
        .collect();
    found.sort();

    assert!(
        !found.is_empty(),
        "no lib*.so found in {}; build the C library first",
        build_dir.display()
    );
    found.remove(0)
}

/// Builds the Rust cdylib from the *current* sources and returns its path.
///
/// This must not simply search `target/` for a `librev16_lib.so`: `cargo test`
/// does **not** build the `cdylib` artifact (it only builds the crate as a test
/// harness), so any pre-existing `.so` lying around in `target/debug` or
/// `target/release` can be arbitrarily stale. Loading a stale library would
/// make these differential tests pass against an old binary while the actual
/// source diverges from the C — a silent false negative.
///
/// So build it explicitly, into a dedicated target directory (a separate
/// directory means a separate Cargo build lock, avoiding a deadlock against the
/// `cargo test` invocation that is running us), and assert the artifact is
/// newer than the sources.
fn build_rust_library() -> PathBuf {
    const FILE: &str = "librev16_lib.so";
    static PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

    PATH.get_or_init(|| {
        let target_dir = crate_root().join("target").join("ffi-dylib");
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());

        let mut cmd = std::process::Command::new(cargo);
        cmd.current_dir(crate_root())
            .args([
                "build",
                "--offline",
                "--release",
                "--lib",
                "--target-dir",
                target_dir.to_str().unwrap(),
            ])
            // Don't inherit the outer cargo/test environment, which would
            // confuse the nested invocation.
            .env_remove("RUSTC_WRAPPER")
            .env_remove("CARGO_UNSTABLE_BUILD_STD")
            .env_remove("CARGO_BUILD_TARGET_DIR")
            .env_remove("CARGO_TARGET_DIR");

        let out = cmd
            .output()
            .expect("failed to spawn `cargo build` for the cdylib");
        assert!(
            out.status.success(),
            "building the Rust cdylib failed:\n--- stdout ---\n{}\n--- stderr ---\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );

        let so = target_dir.join("release").join(FILE);
        assert!(
            so.is_file(),
            "cargo reported success but {} does not exist",
            so.display()
        );

        // Freshness guard: the artifact must be at least as new as the sources
        // it was built from.
        let so_mtime = std::fs::metadata(&so)
            .and_then(|m| m.modified())
            .expect("cdylib mtime");
        let src_mtime = std::fs::metadata(crate_root().join("src").join("lib.rs"))
            .and_then(|m| m.modified())
            .expect("src/lib.rs mtime");
        assert!(
            so_mtime >= src_mtime,
            "{} is older than src/lib.rs — the loaded library is stale",
            so.display()
        );

        so
    })
    .clone()
}

/// Back-compat alias used throughout the tests.
fn find_rust_library() -> PathBuf {
    build_rust_library()
}

fn load(path: &Path) -> Library {
    // SAFETY: the path points at a shared object we just built ourselves; it
    // runs no non-trivial initialisers.
    unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to load {}: {e}", path.display()))
}

/// Both implementations, loaded through their export tables.
struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    c: Rev16Fn,
    rust: Rev16Fn,
}

impl Pair {
    fn load() -> Self {
        let c_path = find_c_library();
        let rust_path = find_rust_library();

        let c_lib = load(&c_path);
        let rust_lib = load(&rust_path);

        // SAFETY: `rev16` in both libraries has the C signature
        // `uint32_t rev16(uint32_t)`, which matches `Rev16Fn`.
        let c = unsafe {
            let sym: Symbol<Rev16Fn> = c_lib
                .get(b"rev16\0")
                .unwrap_or_else(|e| panic!("`rev16` missing from {}: {e}", c_path.display()));
            *sym
        };
        let rust = unsafe {
            let sym: Symbol<Rev16Fn> = rust_lib
                .get(b"rev16\0")
                .unwrap_or_else(|e| panic!("`rev16` missing from {}: {e}", rust_path.display()));
            *sym
        };

        Pair {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    }

    /// Calls both libraries and asserts byte-identical results.
    #[track_caller]
    fn assert_same(&self, input: u32, context: &str) -> u32 {
        // SAFETY: plain integer-in/integer-out C functions; no state, no
        // pointers, nothing to invalidate.
        let c_out = unsafe { (self.c)(input) };
        let rust_out = unsafe { (self.rust)(input) };
        assert_eq!(
            c_out, rust_out,
            "{context}: divergence on input {input:#010x} ({input}): \
             C returned {c_out:#010x}, Rust returned {rust_out:#010x}"
        );
        // Byte-for-byte equality of the returned values.
        assert_eq!(
            c_out.to_ne_bytes(),
            rust_out.to_ne_bytes(),
            "{context}: byte representation differs on input {input:#010x}"
        );
        c_out
    }

}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, property-style testing without extra deps)
// ---------------------------------------------------------------------------

/// SplitMix64 — small, fast, and reproducible across runs and platforms.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
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
}

/// Iterations per randomized row. Large enough to cover a lot of value space,
/// small enough to keep the suite fast.
const ITERS: usize = 200_000;

// ---------------------------------------------------------------------------
// Reference model, used only to assert the tests themselves are meaningful.
// ---------------------------------------------------------------------------

/// Independent reference: reverse the low 16 bits, discarding bits 16..=31.
/// Deliberately written differently from both implementations so that a shared
/// mistake in the mask/shift sequence cannot hide.
fn reference(a: u32) -> u32 {
    let low = (a & 0xFFFF) as u16;
    low.reverse_bits() as u32
}

// ---------------------------------------------------------------------------
// Phase B — valid-path differential tests, one per CONFIGS.md row
// ---------------------------------------------------------------------------

/// CONFIGS row 1 — zero input.
#[test]
fn phase_b_row01_zero() {
    let p = Pair::load();
    let out = p.assert_same(0, "row 1: zero");
    assert_eq!(out, 0, "row 1: rev16(0) must be 0");
}

/// CONFIGS row 2 — every single in-mask bit, `1 << k` for k in 0..=15.
#[test]
fn phase_b_row02_single_low_bits() {
    let p = Pair::load();
    for k in 0..16u32 {
        let input = 1u32 << k;
        let out = p.assert_same(input, "row 2: single low bit");
        assert_eq!(
            out,
            1u32 << (15 - k),
            "row 2: bit {k} must map to bit {}",
            15 - k
        );
    }
}

/// CONFIGS row 3 — every single out-of-mask bit, `1 << k` for k in 16..=31.
/// The 16-bit masks discard these, so the result must be 0.
#[test]
fn phase_b_row03_single_high_bits() {
    let p = Pair::load();
    for k in 16..32u32 {
        let input = 1u32 << k;
        let out = p.assert_same(input, "row 3: single high bit");
        assert_eq!(out, 0, "row 3: high bit {k} must be discarded");
    }
}

/// CONFIGS row 4 — all in-mask bits set (a bit-palindrome).
#[test]
fn phase_b_row04_all_low_bits() {
    let p = Pair::load();
    let out = p.assert_same(0xFFFF, "row 4: 0xFFFF");
    assert_eq!(out, 0xFFFF, "row 4: rev16(0xFFFF) must be 0xFFFF");
}

/// CONFIGS row 5 — all 32 bits set; the high half must be discarded.
#[test]
fn phase_b_row05_all_32_bits() {
    let p = Pair::load();
    let out = p.assert_same(0xFFFF_FFFF, "row 5: 0xFFFFFFFF");
    assert_eq!(
        out, 0xFFFF,
        "row 5: rev16(0xFFFFFFFF) must be 0xFFFF (high half discarded)"
    );
}

/// CONFIGS row 6 — payload entirely in the discarded high half.
#[test]
fn phase_b_row06_high_half_payload_only() {
    let p = Pair::load();
    for input in [0xFFFF_0000u32, 0xAAAA_0000, 0x5555_0000, 0x8000_0000, 0x0001_0000] {
        let out = p.assert_same(input, "row 6: high-half-only payload");
        assert_eq!(out, 0, "row 6: rev16({input:#010x}) must be 0");
    }
}

/// CONFIGS row 7 — random high-half garbage combined with a low-half payload.
/// Proves the high bits never leak into the result: the output must depend only
/// on the low 16 bits.
#[test]
fn phase_b_row07_high_garbage_does_not_leak() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0007);
    for _ in 0..ITERS {
        let lo = rng.next_u32() & 0xFFFF;
        let hi = rng.next_u32() & 0xFFFF;
        let combined = (hi << 16) | lo;

        let with_garbage = p.assert_same(combined, "row 7: hi garbage | lo payload");
        let clean = p.assert_same(lo, "row 7: lo payload alone");
        assert_eq!(
            with_garbage, clean,
            "row 7: high half {hi:#06x} leaked into the result for low half {lo:#06x}"
        );
    }
}

/// CONFIGS row 8 — byte-aligned shapes; exercises the `>>8` / `<<8` statement.
#[test]
fn phase_b_row08_byte_aligned_shapes() {
    let p = Pair::load();
    let inputs = [
        0x00FFu32, 0xFF00, 0xAA00, 0x00AA, 0x5500, 0x0055, 0x0100, 0x0001, 0x8001, 0x0180,
    ];
    for input in inputs {
        let out = p.assert_same(input, "row 8: byte-aligned");
        assert_eq!(out, reference(input), "row 8: reference mismatch");
    }
}

/// CONFIGS row 9 — nibble-aligned shapes; exercises the `>>4` / `<<4`
/// statement.
#[test]
fn phase_b_row09_nibble_aligned_shapes() {
    let p = Pair::load();
    for input in [0x0F0Fu32, 0xF0F0, 0x0F00, 0x00F0, 0x000F, 0xF000, 0x0FF0, 0xF00F] {
        let out = p.assert_same(input, "row 9: nibble-aligned");
        assert_eq!(out, reference(input), "row 9: reference mismatch");
    }
}

/// CONFIGS row 10 — the exact mask constants and adjacent-bit shapes;
/// exercises the `>>1`/`<<1` and `>>2`/`<<2` statements.
#[test]
fn phase_b_row10_mask_constant_shapes() {
    let p = Pair::load();
    for input in [0x3333u32, 0xCCCC, 0x5555, 0xAAAA, 0x9999, 0x6666, 0x1111, 0x8888] {
        let out = p.assert_same(input, "row 10: mask constants");
        assert_eq!(out, reference(input), "row 10: reference mismatch");
    }
}

/// CONFIGS row 11 — bit-palindromic inputs, where reversal is the identity.
/// Constructed by mirroring a random byte into the upper half of the 16-bit
/// word.
#[test]
fn phase_b_row11_palindromes() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0011);
    for _ in 0..20_000 {
        let low_byte = (rng.next_u32() & 0xFF) as u16;
        let mirrored = low_byte.reverse_bits() >> 8; // top byte mirrors bottom
        let palindrome = ((mirrored << 8) | low_byte) as u32;

        let out = p.assert_same(palindrome, "row 11: palindrome");
        assert_eq!(
            out, palindrome,
            "row 11: {palindrome:#06x} is a bit-palindrome, so rev16 must be the identity"
        );
    }
}

/// CONFIGS row 12 — sign-bit-adjacent shapes: bit 15, bit 31, and both.
/// Catches a signed-vs-unsigned promotion or sign-extension mistake.
#[test]
fn phase_b_row12_sign_bit_shapes() {
    let p = Pair::load();
    let mut cases = vec![
        0x8000u32,
        0x8000_0000,
        0x8000_8000,
        0x0000_8000,
        0xFFFF_8000,
        0x8000_FFFF,
        0x7FFF,
        0x8001,
        0xFFFF_7FFF,
    ];
    // Randomized: bit 31 and/or bit 15 forced on top of random noise.
    let mut rng = Rng::new(0x5EED_0012);
    for _ in 0..20_000 {
        let noise = rng.next_u32();
        cases.push(noise | 0x8000_0000);
        cases.push(noise | 0x0000_8000);
        cases.push(noise | 0x8000_8000);
        cases.push(noise & !0x8000_8000);
    }

    for input in cases {
        let out = p.assert_same(input, "row 12: sign-bit shapes");
        assert_eq!(out, reference(input), "row 12: reference mismatch");
    }
}

/// CONFIGS row 13 — involution: applying `rev16` twice to a 16-bit value
/// returns it unchanged. Drives the entry point repeatedly (composed pipeline).
#[test]
fn phase_b_row13_involution() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0013);
    for _ in 0..50_000 {
        let input = rng.next_u32();

        let once = p.assert_same(input, "row 13: first application");
        let twice = p.assert_same(once, "row 13: second application");
        assert_eq!(
            twice,
            input & 0xFFFF,
            "row 13: rev16(rev16({input:#010x})) must be the low half of the input"
        );

        let thrice = p.assert_same(twice, "row 13: third application");
        assert_eq!(thrice, once, "row 13: third application must equal the first");
    }
}

/// CONFIGS row 14 — uniformly random full-range `u32` inputs, fixed seed.
#[test]
fn phase_b_row14_random_full_range() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0014);
    for _ in 0..ITERS {
        let input = rng.next_u32();
        let out = p.assert_same(input, "row 14: uniform random");
        assert_eq!(out, reference(input), "row 14: reference mismatch");
    }
}

/// CONFIGS row 15 — random inputs biased sparse (`r & r`) and dense (`r | r`).
#[test]
fn phase_b_row15_biased_density() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_0015);
    for _ in 0..ITERS / 2 {
        let sparse = rng.next_u32() & rng.next_u32() & rng.next_u32();
        let dense = rng.next_u32() | rng.next_u32() | rng.next_u32();

        let s = p.assert_same(sparse, "row 15: sparse");
        assert_eq!(s, reference(sparse), "row 15: sparse reference mismatch");

        let d = p.assert_same(dense, "row 15: dense");
        assert_eq!(d, reference(dense), "row 15: dense reference mismatch");
    }
}

/// CONFIGS row 16 — exhaustive sweep of `0..=0x1FFFF`: every 16-bit value plus
/// one bit past the mask width.
#[test]
fn phase_b_row16_exhaustive_low_17_bits() {
    let p = Pair::load();
    for input in 0u32..=0x1_FFFF {
        let out = p.assert_same(input, "row 16: exhaustive 0..=0x1FFFF");
        assert_eq!(out, reference(input), "row 16: reference mismatch");
    }
}

// ---------------------------------------------------------------------------
// Phase C — boundary / "error"-path parity
//
// ERRORS.md establishes that the C library has no rejection path at all:
// `rev16` is total over its whole `uint32_t` domain, takes no pointer and no
// enum, and contains no branch, assert, or error return. The Phase C obligation
// is therefore total-ness parity — for every input class that a typical C API
// would reject, both libraries must accept it and return the same value rather
// than trapping.
// ---------------------------------------------------------------------------

/// ERRORS row C1 — zero input ("zero length" analogue).
#[test]
fn phase_c_zero() {
    let p = Pair::load();
    let out = p.assert_same(0, "C1: zero input");
    assert_eq!(out, 0);
    assert_eq!(out, reference(0));
}

/// ERRORS row C2 — maximum / "oversized" input: every bit set, including the
/// bits the 16-bit masks discard. Must not trap and must not saturate.
#[test]
fn phase_c_saturated_and_extremes() {
    let p = Pair::load();
    for input in [
        u32::MAX,
        u32::MAX - 1,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFE,
        i32::MIN as u32,
        i32::MAX as u32,
    ] {
        let out = p.assert_same(input, "C2: extreme value");
        assert_eq!(out, reference(input), "C2: reference mismatch");
        assert!(
            out <= 0xFFFF,
            "C2: result {out:#010x} must always fit in 16 bits"
        );
    }
}

/// ERRORS row C3 — one step past the 16-bit range the masks operate on.
#[test]
fn phase_c_one_past_range() {
    let p = Pair::load();
    for input in [
        0xFFFEu32, // one before the top in-mask value
        0xFFFF,    // the top in-mask value
        0x1_0000,  // exactly one past
        0x1_0001,  // one past, with a low bit
        0x1_FFFF,  // one past, all low bits set
        0x2_0000,  // two past
    ] {
        let out = p.assert_same(input, "C3: one past the mask range");
        assert_eq!(out, reference(input), "C3: reference mismatch");
    }
}

/// ERRORS row C4 — inputs whose entire payload lives in the discarded high
/// half, including every single high bit on its own.
#[test]
fn phase_c_high_half_only() {
    let p = Pair::load();
    for k in 16..32u32 {
        let input = 1u32 << k;
        let out = p.assert_same(input, "C4: single high bit");
        assert_eq!(out, 0, "C4: high bit {k} must be discarded");
    }
    for input in [0xFFFF_0000u32, 0xDEAD_0000, 0x8000_0000, 0xABCD_0000] {
        let out = p.assert_same(input, "C4: high-half-only payload");
        assert_eq!(out, 0, "C4: rev16({input:#010x}) must be 0");
    }
}

/// ERRORS row C5 — every single-bit input across the full 32-bit width.
#[test]
fn phase_c_single_bits() {
    let p = Pair::load();
    for k in 0..32u32 {
        let input = 1u32 << k;
        let out = p.assert_same(input, "C5: single bit");
        assert_eq!(out, reference(input), "C5: reference mismatch for bit {k}");
    }
}

/// ERRORS rows C6/C7 — documented as not applicable (no enum and no pointer
/// parameter exists), so the closest real analogue is asserting that the API
/// truly is a pure `u32 -> u32` total function: no input value is rejected, and
/// repeated calls are stateless and order-independent.
#[test]
fn phase_c_no_rejected_inputs_and_stateless() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_00C6);

    // Every input is accepted; nothing traps or aborts.
    let mut samples: Vec<u32> = (0..32).map(|k| 1u32 << k).collect();
    samples.extend([0, u32::MAX, 0xFFFF, 0x1_0000]);
    samples.extend((0..10_000).map(|_| rng.next_u32()));

    let first_pass: Vec<u32> = samples
        .iter()
        .map(|&i| p.assert_same(i, "C6/C7: total function"))
        .collect();

    // Stateless: replaying the same inputs in reverse order yields the same
    // results, so no hidden global state exists on either side.
    for (idx, &input) in samples.iter().enumerate().rev() {
        let again = p.assert_same(input, "C6/C7: stateless replay");
        assert_eq!(
            again, first_pass[idx],
            "C6/C7: rev16({input:#010x}) is not stateless"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase D — symbol parity, asserted from inside the test suite
// ---------------------------------------------------------------------------

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// under the exact same name. Uses `nm -D --defined-only`; skipped if `nm` is
/// unavailable.
#[test]
fn phase_d_symbol_parity() {
    fn defined_symbols(path: &Path) -> Option<Vec<String>> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", path.to_str().unwrap()])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut symbols: Vec<String> = text
            .lines()
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let (_addr, kind, name) = (parts.next()?, parts.next()?, parts.next()?);
                // Global text/data/bss/weak definitions only; skip the
                // toolchain's own bookkeeping symbols.
                if !matches!(kind, "T" | "D" | "B" | "W" | "R" | "V" | "G" | "S") {
                    return None;
                }
                if name.starts_with("_ITM_")
                    || name.starts_with("__cxa")
                    || name.starts_with("__gmon")
                    || name == "_init"
                    || name == "_fini"
                    || name == "_edata"
                    || name == "_end"
                    || name == "__bss_start"
                {
                    return None;
                }
                Some(name.to_string())
            })
            .collect();
        symbols.sort();
        symbols.dedup();
        Some(symbols)
    }

    let c_path = find_c_library();
    let rust_path = find_rust_library();

    let Some(c_syms) = defined_symbols(&c_path) else {
        eprintln!("`nm` unavailable; skipping symbol-parity check");
        return;
    };
    let Some(rust_syms) = defined_symbols(&rust_path) else {
        eprintln!("`nm` unavailable; skipping symbol-parity check");
        return;
    };

    assert!(
        c_syms.contains(&"rev16".to_string()),
        "sanity: the C .so must export `rev16`; got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but missing from the Rust .so: {missing:?}\n\
         C: {c_syms:?}\nRust: {rust_syms:?}"
    );

    // Additionally confirm every C symbol is actually *callable* through the
    // Rust .so's export table (not merely present in the symbol table).
    let rust_lib = load(&rust_path);
    for name in &c_syms {
        let mut bytes = name.clone().into_bytes();
        bytes.push(0);
        // SAFETY: we only resolve the symbol; we do not call it with a guessed
        // signature here.
        let resolved = unsafe { rust_lib.get::<*const ()>(&bytes) };
        assert!(
            resolved.is_ok(),
            "`{name}` is not resolvable from the Rust .so export table"
        );
    }
}

/// Guards the test harness itself: the independent reference model must agree
/// with the C library. If this fails, the reference is wrong, not the
/// translation.
#[test]
fn harness_reference_agrees_with_c() {
    let p = Pair::load();
    let mut rng = Rng::new(0x5EED_BEEF);
    for _ in 0..50_000 {
        let input = rng.next_u32();
        // SAFETY: integer-in/integer-out C function.
        let c_out = unsafe { (p.c)(input) };
        assert_eq!(
            c_out,
            reference(input),
            "reference model disagrees with the C library on {input:#010x}"
        );
    }
}

// ---------------------------------------------------------------------------
// Exhaustive proof
// ---------------------------------------------------------------------------

/// Exhaustive differential check over the **entire** `uint32_t` domain: all
/// 2^32 inputs, comparing C against Rust through both `.so` export tables.
///
/// The API surface is a single pure `u32 -> u32` function, so this is not a
/// sample — it is a complete proof of behavioural equivalence, leaving no
/// value-dependent path unexercised.
///
/// Marked `#[ignore]` because it takes far longer than a normal unit test; run
/// it with:
///
/// ```text
/// cargo test --offline --release -- --ignored --nocapture
/// ```
#[test]
#[ignore = "exhaustive 2^32 sweep; run explicitly with --ignored"]
fn exhaustive_all_u32_inputs() {
    let p = Pair::load();

    let mut checked: u64 = 0;
    let mut input: u32 = 0;
    loop {
        // SAFETY: integer-in/integer-out C functions with no state.
        let c_out = unsafe { (p.c)(input) };
        let rust_out = unsafe { (p.rust)(input) };
        if c_out != rust_out {
            panic!(
                "divergence at {input:#010x}: C={c_out:#010x} Rust={rust_out:#010x}"
            );
        }
        checked += 1;

        if input == u32::MAX {
            break;
        }
        input += 1;
    }

    assert_eq!(checked, 1u64 << 32, "must have checked every u32 value");
    println!("exhaustively verified all {checked} u32 inputs: C == Rust");
}
