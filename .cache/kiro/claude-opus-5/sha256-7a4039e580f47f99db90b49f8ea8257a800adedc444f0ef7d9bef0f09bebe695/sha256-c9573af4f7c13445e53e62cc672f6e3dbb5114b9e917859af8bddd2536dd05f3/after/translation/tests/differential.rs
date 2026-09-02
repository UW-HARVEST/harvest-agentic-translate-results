//! Differential tests: C `.so` vs Rust `.so`, both loaded through `libloading`.
//!
//! The Rust implementation is NEVER called directly — it is loaded from the
//! built `cdylib` and invoked through its exported `max_size_frame` symbol, so
//! the `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! Phase B rows live in `valid_paths`, Phase C rows in `error_paths`.

use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

/// The one and only exported entry point.
type MaxSizeFrameFn = unsafe extern "C" fn(u32, u32, u32) -> u32;

/// Holds both libraries plus the resolved function pointers.
struct Pair {
    // Kept alive so the function pointers stay valid for the process lifetime.
    _c_lib: Library,
    _rust_lib: Library,
    c: MaxSizeFrameFn,
    rust: MaxSizeFrameFn,
}

// SAFETY: the two function pointers refer to pure, reentrant, stateless
// arithmetic routines in libraries that are never unloaded.
unsafe impl Send for Pair {}
unsafe impl Sync for Pair {}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/translation
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p
}

fn find_c_so() -> PathBuf {
    let build_dir = workspace_root().join("c_src").join("build");
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&build_dir) {
        for e in entries.flatten() {
            let path = e.path();
            let name = match path.file_name().and_then(|s| s.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if name.starts_with("lib") && name.ends_with(".so") {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates.into_iter().next().unwrap_or_else(|| {
        panic!(
            "no C .so found in {}; build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            build_dir.display()
        )
    })
}

fn find_rust_so() -> PathBuf {
    // Integration tests live in <manifest>/tests and the artifact ends up in
    // target/{debug,release}/libmax_size_frame_lib.so. Prefer the profile the
    // test binary itself was built with, then fall back to the other one.
    let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let mut profiles = vec!["debug", "release"];
    if cfg!(not(debug_assertions)) {
        profiles.reverse();
    }
    for profile in profiles {
        let p = target.join(profile).join("libmax_size_frame_lib.so");
        if p.is_file() {
            return p;
        }
    }
    panic!(
        "no Rust cdylib found under {}; build it with `cargo build` / `cargo build --release`",
        target.display()
    );
}

fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c_path = find_c_so();
        let rust_path = find_rust_so();

        // SAFETY: loading two plain C-ABI shared objects; neither runs
        // arbitrary constructors beyond the standard CRT init.
        let c_lib = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", c_path.display()));
        let rust_lib = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", rust_path.display()));

        let c = unsafe {
            let s: Symbol<MaxSizeFrameFn> = c_lib
                .get(b"max_size_frame\0")
                .expect("C .so does not export `max_size_frame`");
            *s
        };
        let rust = unsafe {
            let s: Symbol<MaxSizeFrameFn> = rust_lib
                .get(b"max_size_frame\0")
                .expect("Rust .so does not export `max_size_frame`");
            *s
        };

        Pair {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    })
}

/// Call both libraries and assert byte-identical (`u32`-identical) results.
#[track_caller]
fn check(blocksize: u32, channels: u32, bitdepth: u32) -> u32 {
    let p = pair();
    // SAFETY: signature matches the C declaration exactly; args are by value.
    let c_out = unsafe { (p.c)(blocksize, channels, bitdepth) };
    let rust_out = unsafe { (p.rust)(blocksize, channels, bitdepth) };
    assert_eq!(
        c_out, rust_out,
        "divergence for max_size_frame(blocksize={blocksize} (0x{blocksize:08x}), \
         channels={channels} (0x{channels:08x}), bitdepth={bitdepth} (0x{bitdepth:08x})): \
         C returned {c_out} (0x{c_out:08x}), Rust returned {rust_out} (0x{rust_out:08x})"
    );
    c_out
}

/// Deterministic SplitMix64 so every "randomized" row is reproducible.
struct Rng(u64);

const SEED: u64 = 0x5F3D_C0DE_1234_5678;

impl Rng {
    fn new(stream: u64) -> Self {
        Rng(SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15))
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

    /// Uniform in `lo..=hi`.
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u32
    }

    fn pick(&mut self, xs: &[u32]) -> u32 {
        xs[(self.next_u64() % xs.len() as u64) as usize]
    }
}

/// Iterations per randomized row.
const ITERS: usize = 20_000;

/// Values that sit on every boundary the C expression distinguishes.
const INTERESTING: [u32; 16] = [
    0,
    1,
    2,
    3,
    7,
    8,
    31,
    32,
    33,
    255,
    256,
    65535,
    65536,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFF,
];

// ---------------------------------------------------------------------------
// Phase A — symbol parity, asserted from inside the test suite as well.
// ---------------------------------------------------------------------------

mod symbols {
    use super::*;

    #[test]
    fn both_libraries_export_max_size_frame() {
        // Resolving the symbol in `pair()` is itself the assertion; if either
        // `.so` lacked the export, `pair()` would panic with a clear message.
        let p = pair();
        let a = unsafe { (p.c)(4096, 2, 16) };
        let b = unsafe { (p.rust)(4096, 2, 16) };
        assert_eq!(a, b);
    }

    #[test]
    fn c_so_exports_no_symbol_the_rust_so_lacks() {
        // Mechanical `nm -D` diff, run from the test so it cannot drift from
        // SYMBOLS.md. Skipped gracefully if `nm` is unavailable.
        fn defined(path: &std::path::Path) -> Option<Vec<String>> {
            let out = std::process::Command::new("nm")
                .args(["-D", "--defined-only", "--format=posix"])
                .arg(path)
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let text = String::from_utf8_lossy(&out.stdout);
            let mut names: Vec<String> = text
                .lines()
                .filter_map(|l| l.split_whitespace().next())
                .map(|s| s.to_string())
                .collect();
            names.sort();
            names.dedup();
            Some(names)
        }

        let c_path = find_c_so();
        let rust_path = find_rust_so();
        let (c_syms, rust_syms) = match (defined(&c_path), defined(&rust_path)) {
            (Some(a), Some(b)) => (a, b),
            _ => {
                eprintln!("`nm` unavailable; skipping symbol-diff assertion");
                return;
            }
        };

        assert!(
            c_syms.contains(&"max_size_frame".to_string()),
            "sanity: C .so should export max_size_frame, got {c_syms:?}"
        );

        let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase B — valid-path rows from CONFIGS.md
// ---------------------------------------------------------------------------

mod valid_paths {
    use super::*;

    /// C1: stereo, bitdepth == 32 (the `bitdepth != 32` flag is 0).
    #[test]
    fn cfg_c1() {
        let mut rng = Rng::new(1);
        for _ in 0..ITERS {
            check(rng.range(1, 65535), 2, 32);
        }
    }

    /// C2: stereo, bitdepth in 1..=31 (flag = 1, term3 uses bitdepth + 1).
    #[test]
    fn cfg_c2() {
        let mut rng = Rng::new(2);
        for _ in 0..ITERS {
            check(rng.range(1, 65535), 2, rng.range(1, 31));
        }
    }

    /// C3: stereo, bitdepth above the 32 boundary.
    #[test]
    fn cfg_c3() {
        let mut rng = Rng::new(3);
        for _ in 0..ITERS {
            check(rng.range(1, 65535), 2, rng.range(33, 64));
        }
    }

    /// C4: stereo, zero bitdepth — flag still 1, so term3 == blocksize.
    #[test]
    fn cfg_c4() {
        let mut rng = Rng::new(4);
        for _ in 0..ITERS {
            check(rng.next_u32(), 2, 0);
        }
    }

    /// C5: stereo, empty blocksize.
    #[test]
    fn cfg_c5() {
        let mut rng = Rng::new(5);
        for _ in 0..ITERS {
            check(0, 2, rng.next_u32());
        }
    }

    /// C6: stereo, single-sample blocksize, full-range bitdepth.
    #[test]
    fn cfg_c6() {
        let mut rng = Rng::new(6);
        for _ in 0..ITERS {
            let bitdepth = if rng.next_u64() % 8 == 0 {
                32
            } else {
                rng.next_u32()
            };
            check(1, 2, bitdepth);
        }
    }

    /// C7: zero channels — every term collapses, numerator is exactly 7.
    #[test]
    fn cfg_c7() {
        let mut rng = Rng::new(7);
        for _ in 0..ITERS {
            let out = check(rng.next_u32(), 0, rng.next_u32());
            assert_eq!(out, 18, "channels==0 must always yield 18 + 0 + 7/8");
        }
    }

    /// C8: mono.
    #[test]
    fn cfg_c8() {
        let mut rng = Rng::new(8);
        for _ in 0..ITERS {
            let bitdepth = if rng.next_u64() % 4 == 0 {
                rng.pick(&[31, 32, 33])
            } else {
                rng.range(1, 64)
            };
            check(rng.range(1, 65535), 1, bitdepth);
        }
    }

    /// C9: channels == 3, one past the stereo special case.
    #[test]
    fn cfg_c9() {
        let mut rng = Rng::new(9);
        for _ in 0..ITERS {
            check(rng.range(1, 65535), 3, rng.range(1, 64));
        }
    }

    /// C10: many channels, non-stereo.
    #[test]
    fn cfg_c10() {
        let mut rng = Rng::new(10);
        for _ in 0..ITERS {
            check(rng.range(1, 65535), rng.range(4, 255), rng.range(1, 64));
        }
    }

    /// C11: non-stereo with bitdepth == 32 exactly.
    #[test]
    fn cfg_c11() {
        let mut rng = Rng::new(11);
        for _ in 0..ITERS {
            let mut channels = rng.range(0, 255);
            if channels == 2 {
                channels = 5; // keep this row strictly non-stereo
            }
            check(rng.range(0, 65535), channels, 32);
        }
    }

    /// C12: typical FLAC-like shapes, exhaustive cross-product.
    #[test]
    fn cfg_c12() {
        let blocksizes = [192u32, 576, 1152, 2304, 4096, 4608];
        let bitdepths = [8u32, 12, 16, 20, 24, 32];
        for &blocksize in &blocksizes {
            for channels in 1u32..=8 {
                for &bitdepth in &bitdepths {
                    check(blocksize, channels, bitdepth);
                }
            }
        }
    }

    /// C13: exhaustive boundary sweep (4 * 5 * 8 = 160 combinations).
    #[test]
    fn cfg_c13() {
        let channels_set = [0u32, 1, 2, 3];
        let bitdepth_set = [0u32, 1, 31, 32, 33];
        let blocksize_set = [0u32, 1, 2, 7, 8, 9, 65535, 65536];
        let mut count = 0usize;
        for &channels in &channels_set {
            for &bitdepth in &bitdepth_set {
                for &blocksize in &blocksize_set {
                    check(blocksize, channels, bitdepth);
                    count += 1;
                }
            }
        }
        assert_eq!(count, 160);
    }

    /// C14: sweep every residue class of the numerator mod 8, stereo and mono.
    #[test]
    fn cfg_c14() {
        // Mono with bitdepth == 1 makes the numerator == blocksize + 7, so
        // sweeping blocksize sweeps every residue class.
        for blocksize in 0u32..64 {
            check(blocksize, 1, 1);
        }
        // Stereo with bitdepth == 0: numerator == blocksize + 7.
        for blocksize in 0u32..64 {
            check(blocksize, 2, 0);
        }
        // Stereo with bitdepth == 32: numerator == 64*blocksize + 7.
        for blocksize in 0u32..64 {
            check(blocksize, 2, 32);
        }
        // Mixed widths so the numerator hits all residues from the other side.
        for bitdepth in 0u32..40 {
            for blocksize in 0u32..9 {
                check(blocksize, 2, bitdepth);
                check(blocksize, 1, bitdepth);
                check(blocksize, 3, bitdepth);
            }
        }
    }

    /// C15: stereo overflow regime — blocksize * bitdepth wraps mod 2^32.
    #[test]
    fn cfg_c15() {
        let mut rng = Rng::new(15);
        for _ in 0..ITERS {
            let blocksize = rng.range(1 << 16, u32::MAX);
            let bitdepth = rng.range(1 << 16, u32::MAX);
            check(blocksize, 2, bitdepth);
        }
    }

    /// C16: non-stereo overflow regime — huge channel count wraps term1 and
    /// also wraps the trailing `18 + channels`.
    #[test]
    fn cfg_c16() {
        let mut rng = Rng::new(16);
        for _ in 0..ITERS {
            let mut channels = rng.range(1 << 16, u32::MAX);
            if channels == 2 {
                channels = 3;
            }
            check(rng.next_u32(), channels, rng.next_u32());
        }
    }

    /// C17: bitdepth == u32::MAX so `bitdepth + (bitdepth != 32)` wraps to 0.
    #[test]
    fn cfg_c17() {
        let mut rng = Rng::new(17);
        check(0, 2, u32::MAX);
        check(1, 2, u32::MAX);
        check(u32::MAX, 2, u32::MAX);
        for _ in 0..ITERS {
            check(rng.next_u32(), 2, u32::MAX);
        }
    }

    /// C18: unconstrained full-range fuzz across all three arguments.
    #[test]
    fn cfg_c18() {
        let mut rng = Rng::new(18);
        for _ in 0..(ITERS * 10) {
            check(rng.next_u32(), rng.next_u32(), rng.next_u32());
        }
    }

    /// C19: exhaustive 16^3 cross-product of the interesting constants.
    #[test]
    fn cfg_c19() {
        let mut count = 0usize;
        for &blocksize in &INTERESTING {
            for &channels in &INTERESTING {
                for &bitdepth in &INTERESTING {
                    check(blocksize, channels, bitdepth);
                    count += 1;
                }
            }
        }
        assert_eq!(count, 4096);
    }

    /// C20: statelessness — repeated and interleaved calls stay in lockstep and
    /// each configuration keeps returning the same value.
    #[test]
    fn cfg_c20() {
        let configs = [
            (4096u32, 2u32, 16u32),
            (0, 0, 0),
            (u32::MAX, u32::MAX, u32::MAX),
            (1152, 1, 24),
            (576, 8, 32),
            (65535, 3, 31),
        ];
        let baseline: Vec<u32> = configs
            .iter()
            .map(|&(b, c, d)| check(b, c, d))
            .collect();
        for _ in 0..1000 {
            for (i, &(b, c, d)) in configs.iter().enumerate() {
                let out = check(b, c, d);
                assert_eq!(
                    out, baseline[i],
                    "max_size_frame is not stateless for ({b}, {c}, {d})"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase C — error/boundary rows from ERRORS.md
//
// The C function has no rejection path (see ERRORS.md for the grep evidence),
// so each row asserts the two libraries agree on the defined value and that
// neither traps. A Rust overflow panic would abort the process, since the
// release profile sets `panic = "abort"`.
// ---------------------------------------------------------------------------

mod error_paths {
    use super::*;

    /// E1: all-zero arguments.
    #[test]
    fn err_e1_all_zero() {
        let out = check(0, 0, 0);
        assert_eq!(out, 18, "18 + 0 + (0 + 7) / 8 == 18");
    }

    /// E2: zero channels with nonzero blocksize/bitdepth.
    #[test]
    fn err_e2_zero_channels() {
        let mut rng = Rng::new(102);
        for _ in 0..ITERS {
            let out = check(rng.range(1, u32::MAX), 0, rng.range(1, u32::MAX));
            assert_eq!(out, 18);
        }
    }

    /// E3: zero blocksize.
    #[test]
    fn err_e3_zero_blocksize() {
        let mut rng = Rng::new(103);
        for _ in 0..ITERS {
            let channels = rng.next_u32();
            let out = check(0, channels, rng.next_u32());
            assert_eq!(out, 18u32.wrapping_add(channels));
        }
    }

    /// E4: zero bitdepth — `bitdepth != 32` is still 1, so stereo term3 lives.
    #[test]
    fn err_e4_zero_bitdepth() {
        let mut rng = Rng::new(104);
        for _ in 0..ITERS {
            let blocksize = rng.next_u32();
            let out = check(blocksize, 2, 0);
            assert_eq!(out, 20u32.wrapping_add(blocksize.wrapping_add(7) / 8));
        }
        for _ in 0..ITERS {
            check(rng.next_u32(), rng.range(0, 8), 0);
        }
    }

    /// E5: u32::MAX in each argument position individually.
    #[test]
    fn err_e5_u32_max_each() {
        for &other_a in &INTERESTING {
            for &other_b in &INTERESTING {
                check(u32::MAX, other_a, other_b);
                check(other_a, u32::MAX, other_b);
                check(other_a, other_b, u32::MAX);
            }
        }
    }

    /// E6: maximal wraparound in every operation at once.
    #[test]
    fn err_e6_u32_max_all() {
        check(u32::MAX, u32::MAX, u32::MAX);
        check(u32::MAX, u32::MAX - 1, u32::MAX);
        check(u32::MAX - 1, u32::MAX, u32::MAX - 1);
    }

    /// E7: term1 alone overflows (non-stereo, so term1 is the live term).
    #[test]
    fn err_e7_numerator_overflow() {
        // blocksize * bitdepth * channels well past 2^32 while staying non-stereo.
        check(1 << 20, 1 << 6, 1 << 10);
        check(70000, 70000, 70000);
        check(0x1_0000, 3, 0x1_0000);
        check(0xFFFF, 5, 0x10000);
        let mut rng = Rng::new(107);
        for _ in 0..ITERS {
            let blocksize = rng.range(1 << 12, u32::MAX);
            let bitdepth = rng.range(1 << 12, u32::MAX);
            let mut channels = rng.range(3, 1 << 12);
            if channels == 2 {
                channels = 3;
            }
            check(blocksize, channels, bitdepth);
        }
    }

    /// E8: the `+7` itself wraps because the pre-`+7` sum is within 7 of 2^32.
    #[test]
    fn err_e8_plus7_wrap() {
        // Mono, bitdepth == 1: numerator_before_7 == blocksize. Choose the top
        // eight blocksize values so `blocksize + 7` wraps for all but one.
        for blocksize in (u32::MAX - 8)..=u32::MAX {
            check(blocksize, 1, 1);
        }
        // Stereo, bitdepth == 0: numerator_before_7 == blocksize as well.
        for blocksize in (u32::MAX - 8)..=u32::MAX {
            check(blocksize, 2, 0);
        }
        // Mono, bitdepth == 2: numerator_before_7 == 2*blocksize; pick
        // blocksize so 2*blocksize lands in the top-8 window mod 2^32.
        for k in 0u32..8 {
            let blocksize = 0u32.wrapping_sub(k) / 2;
            check(blocksize, 1, 2);
            check(blocksize.wrapping_add(1), 1, 2);
        }
        // Sweep the exact wrap window by construction: numerator_before_7 = n.
        for n in (u32::MAX - 7)..=u32::MAX {
            // mono, bitdepth 1 => numerator_before_7 == blocksize == n
            let out = check(n, 1, 1);
            assert_eq!(out, 19u32.wrapping_add(n.wrapping_add(7) / 8));
        }
    }

    /// E9: the final `18 + channels + bytes` add wraps.
    #[test]
    fn err_e9_final_add_wrap() {
        // channels near u32::MAX makes 18 + channels wrap on its own.
        for channels in (u32::MAX - 20)..=u32::MAX {
            if channels == 2 {
                continue;
            }
            check(0, channels, 0);
            check(1, channels, 1);
            check(u32::MAX, channels, u32::MAX);
        }
        // Stereo path: make `bytes` huge so 18 + 2 + bytes wraps.
        let mut rng = Rng::new(109);
        for _ in 0..ITERS {
            check(rng.range(1 << 24, u32::MAX), 2, rng.range(1 << 4, u32::MAX));
        }
    }

    /// E10: one step past each range boundary the C special-cases.
    #[test]
    fn err_e10_one_past_boundaries() {
        for channels in 0u32..=4 {
            for bitdepth in [0u32, 30, 31, 32, 33, 34, 63, 64, 65] {
                for blocksize in [0u32, 1, 7, 8, 9, 4096, 65535, 65536, u32::MAX] {
                    check(blocksize, channels, bitdepth);
                }
            }
        }
    }

    /// E11: values far outside any meaningful domain — C accepts any u32.
    #[test]
    fn err_e11_out_of_domain_values() {
        let wild = [
            0xFFFF_FFFEu32,
            0xDEAD_BEEF,
            0xCAFE_BABE,
            0x8000_0001,
            0x7FFF_FFFE,
            0xABAD_1DEA,
            u32::MAX,
        ];
        for &blocksize in &wild {
            for &channels in &wild {
                for &bitdepth in &wild {
                    check(blocksize, channels, bitdepth);
                }
            }
        }
        // Also mix wild values with the special-cased ones.
        for &wild_v in &wild {
            for &special in &[0u32, 1, 2, 3, 32] {
                check(wild_v, special, wild_v);
                check(wild_v, wild_v, special);
                check(special, wild_v, wild_v);
            }
        }
    }

    /// E12: division truncation edges, including post-wrap tiny numerators.
    #[test]
    fn err_e12_division_edges() {
        // numerator = blocksize + 7 for mono/bitdepth 1: sweep 7..=22 and the
        // wrapped tiny values produced by blocksize near u32::MAX.
        for blocksize in 0u32..=16 {
            check(blocksize, 1, 1);
        }
        for blocksize in (u32::MAX - 16)..=u32::MAX {
            check(blocksize, 1, 1);
        }
        // Divisor is the literal 8, so a divide-by-zero is impossible; confirm
        // the extreme numerator still agrees.
        check(u32::MAX, 1, 1);
        check(u32::MAX, 2, 32);
        // Every residue class mod 8 on a large numerator.
        let mut rng = Rng::new(112);
        for _ in 0..ITERS {
            check(rng.next_u32(), rng.pick(&[0, 1, 2, 3]), rng.pick(&[0, 1, 8, 31, 32, 33]));
        }
    }
}
