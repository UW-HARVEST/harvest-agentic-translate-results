//! Phase B (valid-path) + Phase C (error/boundary-path) differential tests.
//!
//! Every row of `CONFIGS.md` and `ERRORS.md` has a test here. Both sides are
//! invoked exclusively through `dlopen`/`dlsym` on their respective `.so`.

mod harness;

use harness::{Half2FloatWide64, Pair, Rng};
use std::sync::OnceLock;

fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(harness::load)
}

/// Number of randomized samples per `CONFIGS.md` row that has a random axis.
const SAMPLES_PER_ROW: usize = 20_000;

/// Compose `h` from the exponent index `n` (0..=63) and the low 10 bits.
fn h_of(n: u16, lo: u16) -> u16 {
    debug_assert!(n < 64 && lo < 0x400);
    (n << 10) | lo
}

/// Drive one `CONFIGS.md` row: every explicit boundary value, plus
/// `SAMPLES_PER_ROW` seeded-random draws from the row's `(n, lo)` domain.
fn run_row(row: u32, ns: &[u16], los: &[u16], boundaries: &[u16]) {
    let p = pair();
    assert!(!ns.is_empty() && !los.is_empty(), "row {row}: empty domain");

    for &h in boundaries {
        p.assert_same(h);
    }
    // Seed derived from the row number so each row is independently
    // reproducible but the whole suite is deterministic.
    let mut rng = Rng::new(0xC0FFEE_0000_0000 ^ u64::from(row));
    for _ in 0..SAMPLES_PER_ROW {
        let n = ns[rng.below(ns.len() as u32) as usize];
        let lo = los[rng.below(los.len() as u32) as usize];
        p.assert_same(h_of(n, lo));
    }
}

fn nonzero_los() -> Vec<u16> {
    (1..0x400).collect()
}
fn all_los() -> Vec<u16> {
    (0..0x400).collect()
}

// ---------------------------------------------------------------------------
// Sanity: we really did load two distinct shared objects.
// ---------------------------------------------------------------------------

#[test]
fn loaded_two_distinct_shared_objects() {
    let p = pair();
    assert_ne!(
        p.c_path, p.rust_path,
        "C and Rust .so paths must differ (got the same file twice)"
    );
    assert!(
        std::env::var_os("HALF2FLOAT_C_SO").is_some()
            || p.c_path.to_string_lossy().contains("c_src"),
        "C .so should come from c_src/build unless HALF2FLOAT_C_SO overrides it, got {}",
        p.c_path.display()
    );
    assert!(
        p.rust_path
            .to_string_lossy()
            .contains("libhalf2float_lib.so"),
        "Rust .so should be the cdylib, got {}",
        p.rust_path.display()
    );
    // A value that is not trivially 0 on both sides, proving the symbols resolve
    // to real code rather than to the same stub.
    assert_eq!(p.c_bits(0x3C00), 0x3F80_0000, "C half2float(1.0h) != 1.0f");
    assert_eq!(
        p.rust_bits(0x3C00),
        0x3F80_0000,
        "Rust half2float(1.0h) != 1.0f"
    );
}

// ---------------------------------------------------------------------------
// Phase B — CONFIGS.md rows 1..23
// ---------------------------------------------------------------------------

mod phase_b_configuration_surface {
    use super::*;

    #[test]
    fn row01_n0_offset0_lo_zero_positive_zero() {
        run_row(1, &[0], &[0], &[0x0000]);
    }

    #[test]
    fn row02_n0_offset0_lo_nonzero_positive_subnormals() {
        run_row(2, &[0], &nonzero_los(), &[0x0001, 0x0002, 0x0200, 0x03FE]);
    }

    #[test]
    fn row03_n0_lo_max_first_half_upper_bound() {
        run_row(3, &[0], &[0x3FF], &[0x03FF]);
    }

    #[test]
    fn row04_n1_offset400_second_half_lower_bound() {
        run_row(4, &[1], &all_los(), &[0x0400, 0x0401, 0x07FF]);
    }

    #[test]
    fn row05_n2_to_30_regular_positive_normals() {
        let ns: Vec<u16> = (2..=30).collect();
        run_row(5, &ns, &all_los(), &[0x0800, 0x3C00, 0x4000, 0x77FF]);
    }

    #[test]
    fn row06_n30_lo_max_largest_finite_positive() {
        run_row(6, &[30], &[0x3FF], &[0x7BFF]);
    }

    #[test]
    fn row07_n31_irregular_exponent_lo_zero_plus_inf() {
        run_row(7, &[31], &[0], &[0x7C00]);
        let p = pair();
        assert_eq!(p.c_bits(0x7C00), 0x7F80_0000, "C must yield +Inf");
    }

    #[test]
    fn row08_n31_irregular_exponent_lo_nonzero_positive_nan_payloads() {
        run_row(8, &[31], &nonzero_los(), &[0x7C01, 0x7DFF, 0x7FFE]);
    }

    #[test]
    fn row09_n31_lo_max_largest_positive_sum() {
        run_row(9, &[31], &[0x3FF], &[0x7FFF]);
        let p = pair();
        assert_eq!(p.c_bits(0x7FFF), 0x7FFF_E000);
    }

    #[test]
    fn row10_n32_second_offset0_entry_negative_zero() {
        run_row(10, &[32], &[0], &[0x8000]);
        let p = pair();
        assert_eq!(p.c_bits(0x8000), 0x8000_0000, "C must yield -0.0");
    }

    #[test]
    fn row11_n32_offset0_lo_nonzero_negative_subnormals() {
        run_row(11, &[32], &nonzero_los(), &[0x8001, 0x8002, 0x8200, 0x83FE]);
    }

    #[test]
    fn row12_n32_lo_max_first_half_upper_bound_negative_side() {
        run_row(12, &[32], &[0x3FF], &[0x83FF]);
    }

    #[test]
    fn row13_n33_offset400_second_half_lower_bound_negative_side() {
        run_row(13, &[33], &all_los(), &[0x8400, 0x8401, 0x87FF]);
    }

    #[test]
    fn row14_n34_to_62_regular_negative_normals() {
        let ns: Vec<u16> = (34..=62).collect();
        run_row(14, &ns, &all_los(), &[0x8800, 0xBC00, 0xC000, 0xF7FF]);
    }

    #[test]
    fn row15_n62_lo_max_largest_magnitude_finite_negative() {
        run_row(15, &[62], &[0x3FF], &[0xFBFF]);
    }

    #[test]
    fn row16_n63_irregular_exponent_lo_zero_minus_inf() {
        run_row(16, &[63], &[0], &[0xFC00]);
        let p = pair();
        assert_eq!(p.c_bits(0xFC00), 0xFF80_0000, "C must yield -Inf");
    }

    #[test]
    fn row17_n63_irregular_exponent_lo_nonzero_negative_nan_payloads() {
        run_row(17, &[63], &nonzero_los(), &[0xFC01, 0xFDFF, 0xFFFE]);
    }

    #[test]
    fn row18_n63_lo_max_largest_sum_overall() {
        run_row(18, &[63], &[0x3FF], &[0xFFFF]);
        let p = pair();
        assert_eq!(p.c_bits(0xFFFF), 0xFFFF_E000);
    }

    /// Full offset x exponent x index-boundary cross product: every `n` against
    /// every interesting low-bit boundary.
    #[test]
    fn row19_full_n_times_index_boundary_cross_product() {
        let p = pair();
        let mut count = 0;
        for n in 0u16..64 {
            for lo in [0u16, 1, 0x1FF, 0x3FE, 0x3FF] {
                p.assert_same(h_of(n, lo));
                count += 1;
            }
        }
        assert_eq!(count, 320, "expected the full 64x5 cross product");
    }

    /// Seeded uniform sweep over the entire domain — catches value-dependent
    /// bugs that region-restricted rows could miss.
    #[test]
    fn row20_seeded_uniform_random_full_domain() {
        let p = pair();
        let mut rng = Rng::new(0x5EED_0000_0000_0014);
        for _ in 0..200_000 {
            p.assert_same(rng.u16());
        }
    }

    /// Exhaustive: `half2float` has a 16-bit domain, so "all inputs" is cheap
    /// and is the strongest possible valid-path check.
    #[test]
    fn row21_exhaustive_all_65536_inputs_ascending() {
        let p = pair();
        let mut mismatches = Vec::new();
        for h in 0u16..=u16::MAX {
            let (cb, rb) = (p.c_bits(h), p.rust_bits(h));
            if cb != rb {
                mismatches.push((h, cb, rb));
                if mismatches.len() >= 16 {
                    break;
                }
            }
        }
        assert!(
            mismatches.is_empty(),
            "exhaustive sweep found {} mismatch(es), first few: {:X?}",
            mismatches.len(),
            mismatches
        );
    }

    /// Exhaustive again but in a seeded-shuffled order, interleaving the two
    /// libraries, so any order-dependent or lazily-initialised internal state
    /// introduced by the translation would show up.
    #[test]
    fn row22_exhaustive_shuffled_order_interleaved() {
        let p = pair();
        let mut order: Vec<u16> = (0u16..=u16::MAX).collect();
        let mut rng = Rng::new(0x5117_F1E0_0000_0016);
        // Fisher-Yates
        for i in (1..order.len()).rev() {
            let j = rng.below((i + 1) as u32) as usize;
            order.swap(i, j);
        }
        for &h in &order {
            p.assert_same(h);
        }
        // And a second pass in the reverse of that order: results must repeat.
        for &h in order.iter().rev() {
            p.assert_same(h);
        }
    }

    /// Concurrent invocation through the same `.so` handles.
    #[test]
    fn row23_concurrent_invocation_is_consistent() {
        let p = pair();
        // `extern "C" fn` pointers are Copy + Send + Sync; the leaked Library
        // handles keep the code mapped for the whole process.
        let (c, r) = (p.c, p.rust);
        let mut handles = Vec::new();
        for t in 0..8u32 {
            handles.push(std::thread::spawn(move || {
                let mut rng = Rng::new(0xF00D_0000 ^ u64::from(t));
                for _ in 0..40_000 {
                    let h = rng.u16();
                    // SAFETY: scalar C ABI, no shared mutable state.
                    let (cb, rb) = unsafe { (c(h).to_bits(), r(h).to_bits()) };
                    assert_eq!(cb, rb, "thread {t}: divergence at h=0x{h:04X}");
                }
            }));
        }
        for h in handles {
            h.join().expect("worker thread panicked");
        }
    }
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md rows 1..13
// ---------------------------------------------------------------------------

mod phase_c_error_surface {
    use super::*;

    /// Rows 1 & 2: the minimum and maximum inputs, i.e. both extremes of the
    /// `m__mantissa` index range (0 and 2047). Neither side may reject.
    #[test]
    fn row01_row02_min_and_max_input_never_rejected() {
        let p = pair();
        p.assert_same(0x0000);
        p.assert_same(0xFFFF);
        assert_eq!(p.c_bits(0x0000), 0x0000_0000);
        assert_eq!(p.rust_bits(0x0000), 0x0000_0000);
        assert_eq!(p.c_bits(0xFFFF), 0xFFFF_E000);
        assert_eq!(p.rust_bits(0xFFFF), 0xFFFF_E000);
    }

    /// Rows 3 & 4: the `m__offset` 0 -> 0x400 transition, i.e. mantissa index
    /// 1023 (last of the first table half) then 1024 (first of the second).
    /// One step past each boundary is included on both sides.
    #[test]
    fn row03_row04_offset_transition_positive_side() {
        let p = pair();
        for h in [0x03FEu16, 0x03FF, 0x0400, 0x0401] {
            p.assert_same(h);
        }
    }

    /// Rows 5 & 6: the same transition reached through the *second*
    /// `m__offset[n] == 0` entry (`n == 32`), which is the easy one to miss.
    #[test]
    fn row05_row06_offset_transition_negative_side() {
        let p = pair();
        for h in [0x83FEu16, 0x83FF, 0x8400, 0x8401] {
            p.assert_same(h);
        }
    }

    /// Row 7: `n == 31` irregular exponent, the large-sum site on the positive
    /// side. Must not trip a Rust overflow panic.
    #[test]
    fn row07_irregular_exponent_31_no_overflow_panic() {
        let p = pair();
        for h in [0x7BFFu16, 0x7C00, 0x7C01] {
            p.assert_same(h);
        }
        assert_eq!(p.rust_bits(0x7C00), 0x7F80_0000);
    }

    /// Row 8: `n == 63` irregular exponent — the arithmetically largest
    /// addition in the library (`0x387FE000 + 0xC7800000`).
    #[test]
    fn row08_irregular_exponent_63_no_overflow_panic() {
        let p = pair();
        for h in [0xFBFFu16, 0xFC00, 0xFC01] {
            p.assert_same(h);
        }
        assert_eq!(p.rust_bits(0xFC00), 0xFF80_0000);
    }

    /// Row 9: maximum sum on each sign, checked as exact bit patterns.
    #[test]
    fn row09_maximum_sums_exact_bits() {
        let p = pair();
        for (h, want) in [(0x7FFFu16, 0x7FFF_E000u32), (0xFFFF, 0xFFFF_E000)] {
            assert_eq!(p.c_bits(h), want, "C h=0x{h:04X}");
            assert_eq!(p.rust_bits(h), want, "Rust h=0x{h:04X}");
        }
    }

    /// Row 10: NaN payloads must be carried through bit-exactly, not
    /// canonicalised. Exhaustive over every NaN encoding on both signs.
    #[test]
    fn row10_nan_payloads_bit_exact_not_canonicalised() {
        let p = pair();
        for n in [31u16, 63] {
            for lo in 1..0x400u16 {
                let h = (n << 10) | lo;
                let (cb, rb) = (p.c_bits(h), p.rust_bits(h));
                assert_eq!(cb, rb, "NaN payload divergence at h=0x{h:04X}");
                // The C output really is a NaN with a non-canonical payload,
                // which is what makes the bit comparison meaningful.
                assert_eq!(cb & 0x7F80_0000, 0x7F80_0000, "h=0x{h:04X} not NaN/Inf");
                assert_ne!(cb & 0x007F_FFFF, 0, "h=0x{h:04X} unexpectedly Inf");
                assert!(f32::from_bits(cb).is_nan());
            }
        }
        // Explicit ERRORS.md row-10 values.
        for h in [0x7C01u16, 0x7DFF, 0xFC01, 0xFDFF] {
            p.assert_same(h);
        }
    }

    /// Row 11: caller does not zero-extend the `uint16_t`. The C ABI leaves the
    /// upper half of the argument register unspecified, so the requirement is
    /// only that the Rust `.so` behaves *identically to the C `.so`*. This is
    /// the closest analogue here to "out-of-range enum value across FFI":
    /// a bit pattern with no valid `uint16_t` interpretation.
    #[test]
    fn row11_argument_with_garbage_in_high_bits_matches_c() {
        let p = pair();
        let mut rng = Rng::new(0xABAD_1DEA_0000_000B);
        let mut divergences = Vec::new();
        let mut hi_patterns: Vec<u32> = vec![
            0x0000_0000,
            0x0001_0000,
            0xFFFF_0000,
            0x7FFF_0000,
            0x8000_0000,
            0xDEAD_0000,
        ];
        for _ in 0..64 {
            hi_patterns.push(rng.next_u32() & 0xFFFF_0000);
        }
        for hi in hi_patterns {
            for lo in [0x0000u32, 0x0001, 0x03FF, 0x0400, 0x7C00, 0x8000, 0xFFFF] {
                let arg = hi | lo;
                // SAFETY: calling the same symbol through a wider-argument
                // signature. On the SysV x86-64 / AArch64 C ABI the argument
                // occupies a whole register either way, so this is a legal
                // machine-level call; the point is to compare the two
                // implementations' treatment of the unspecified high bits.
                let (cb, rb) = unsafe { ((p.c_wide)(arg).to_bits(), (p.rust_wide)(arg).to_bits()) };
                if cb != rb {
                    divergences.push((arg, cb, rb));
                }
            }
        }
        assert!(
            divergences.is_empty(),
            "Rust and C disagree when the uint16_t argument is not zero-extended \
             ({} case(s)), first few: {:X?}",
            divergences.len(),
            &divergences[..divergences.len().min(8)]
        );
    }

    /// Row 11, continued: garbage in the upper *32* bits of the full argument
    /// register. The C reads `edi` only, so bits 32..63 must be irrelevant on
    /// both sides.
    #[test]
    fn row11b_argument_with_garbage_in_upper_64_bits_matches_c() {
        let p = pair();
        let c: Half2FloatWide64 = p.c_wide64;
        let r: Half2FloatWide64 = p.rust_wide64;
        let mut rng = Rng::new(0xABAD_1DEA_0000_00BB);
        let mut divergences = Vec::new();
        for _ in 0..2_000 {
            let arg = rng.next_u64();
            // SAFETY: the symbol is reached through a wider-argument signature;
            // the argument still occupies one register. Comparing the two
            // implementations' handling of the unspecified high bits is the point.
            let (cb, rb) = unsafe { (c(arg).to_bits(), r(arg).to_bits()) };
            if cb != rb {
                divergences.push((arg, cb, rb));
            }
        }
        for arg in [0u64, u64::MAX, 0xFFFF_FFFF_FFFF_0000, 0xDEAD_BEEF_DEAD_03FF] {
            let (cb, rb) = unsafe { (c(arg).to_bits(), r(arg).to_bits()) };
            if cb != rb {
                divergences.push((arg, cb, rb));
            }
        }
        assert!(
            divergences.is_empty(),
            "Rust and C disagree on garbage in the upper bits of the argument \
             register ({} case(s)), first few: {:X?}",
            divergences.len(),
            &divergences[..divergences.len().min(8)]
        );
    }

    /// Row 12: document/enforce that the generic pointer/length/enum FFI
    /// boundaries are structurally unreachable — the API is a single scalar in,
    /// scalar out. If the header ever grows a pointer parameter this test's
    /// premise must be revisited.
    #[test]
    fn row12_no_pointer_length_or_enum_parameters_exist() {
        let header = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("c_src/include/lib.h"),
        )
        .expect("must be able to read c_src/include/lib.h");
        let decls: Vec<&str> = header
            .lines()
            .map(str::trim)
            .filter(|l| l.contains("half2float"))
            .collect();
        assert_eq!(
            decls,
            vec!["float half2float(uint16_t h);"],
            "public API changed; ERRORS.md row 12 (no pointer/length/enum \
             parameters, so those boundaries are unreachable) must be re-derived"
        );
        assert!(
            !header.contains('*') && !header.contains("enum") && !header.contains("size_t"),
            "public header gained a pointer/enum/length type; re-derive ERRORS.md"
        );
    }

    /// Row 13: repeated and interleaved calls are pure — no internal state.
    #[test]
    fn row13_calls_are_pure_and_order_independent() {
        let p = pair();
        let probes = [
            0x0000u16, 0xFFFF, 0x3C00, 0x7C00, 0x8000, 0xFC00, 0x03FF, 0x0400, 0x83FF, 0x8400,
        ];
        // Baseline.
        let baseline: Vec<(u32, u32)> = probes.iter().map(|&h| (p.c_bits(h), p.rust_bits(h))).collect();
        // Hammer the whole domain in between to disturb any hidden state.
        let mut rng = Rng::new(0x1DEA_0000_0000_000D);
        for _ in 0..50_000 {
            p.assert_same(rng.u16());
        }
        for (i, &h) in probes.iter().enumerate() {
            assert_eq!(
                (p.c_bits(h), p.rust_bits(h)),
                baseline[i],
                "result for h=0x{h:04X} changed after intervening calls"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Phase D — symbol parity asserted from inside the test suite as well.
// ---------------------------------------------------------------------------

mod phase_d_symbol_parity {
    use super::*;

    fn exported_symbols(so: &std::path::Path) -> Vec<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", "--format=posix"])
            .arg(so)
            .output()
            .expect("nm must be available");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            so.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let ignored = ["_ITM_", "__cxa_finalize", "__gmon_start__", "_init", "_fini"];
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .filter(|s| !ignored.iter().any(|ig| s.starts_with(ig)))
            .map(str::to_owned)
            .collect();
        v.sort();
        v.dedup();
        v
    }

    #[test]
    fn every_c_exported_symbol_is_exported_by_rust() {
        let p = pair();
        let c = exported_symbols(&p.c_path);
        let r = exported_symbols(&p.rust_path);
        assert!(
            c.contains(&"half2float".to_string()),
            "C .so should export half2float, got {c:?}"
        );
        let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "symbols exported by the C .so but missing from the Rust .so: {missing:?}"
        );
    }

    #[test]
    fn rust_so_has_no_unresolved_non_libc_symbols() {
        let p = pair();
        let out = std::process::Command::new("nm")
            .args(["-D", "--undefined-only", "--format=posix"])
            .arg(&p.rust_path)
            .output()
            .expect("nm must be available");
        assert!(out.status.success());
        let text = String::from_utf8_lossy(&out.stdout);
        let suspicious: Vec<&str> = text
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .filter(|s| {
                // Everything the Rust std/unwinder legitimately imports.
                !(s.contains("@GLIBC")
                    || s.contains("@GCC")
                    || s.starts_with("_Unwind")
                    || s.starts_with("_ITM_")
                    || s.starts_with("__")
                    || matches!(
                        *s,
                        "malloc"
                            | "calloc"
                            | "realloc"
                            | "free"
                            | "memcpy"
                            | "memmove"
                            | "memset"
                            | "bcmp"
                            | "strlen"
                            | "abort"
                            | "getenv"
                            | "getcwd"
                            | "readlink"
                            | "realpath"
                            | "open64"
                            | "close"
                            | "read"
                            | "write"
                            | "writev"
                            | "lseek64"
                            | "fstat64"
                            | "stat64"
                            | "statx"
                            | "mmap64"
                            | "munmap"
                            | "syscall"
                            | "gettid"
                            | "posix_memalign"
                            | "dl_iterate_phdr"
                            | "pthread_key_create"
                            | "pthread_key_delete"
                            | "pthread_setspecific"
                            | "pthread_getspecific"
                    ))
            })
            .collect();
        assert!(
            suspicious.is_empty(),
            "Rust .so has unresolved non-libc symbols: {suspicious:?}"
        );
    }
}
