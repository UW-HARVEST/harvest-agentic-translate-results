//! Phase B (valid-path) + Phase D (symbol parity) differential tests.
//!
//! Every row of `CONFIGS.md` has a test here.  Both libraries are always
//! reached through `dlopen` + the exported C symbols.

mod harness;

use harness::{boundary_i32s, Pair, Rng};

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================
mod configs {
    use super::*;

    /// Row 1 — fresh state, 0 prior calls: the very first `static_sum(v)` must
    /// return exactly `v`, which is the observable proof that `sum` starts at 0.
    #[test]
    fn row01_fresh_state_accumulator_starts_at_zero() {
        let mut rng = Rng::new(0x0101_0001);
        for _ in 0..64 {
            let v = rng.i32_any();
            let p = Pair::fresh();
            let got = p.check_static_sum(v, "row01");
            assert_eq!(got, v, "fresh sum was not 0 for first update {v}");
        }
    }

    /// Row 2 — fresh state, one call, degenerate `update == 0`.
    #[test]
    fn row02_fresh_single_zero_update() {
        let p = Pair::fresh();
        let got = p.check_static_sum(0, "row02");
        assert_eq!(got, 0);
    }

    /// Row 3 — fresh state, one call, randomized small updates.
    #[test]
    fn row03_fresh_single_small_update_random() {
        let mut rng = Rng::new(0x0303_0003);
        for i in 0..512 {
            let v = rng.i32_in(-1000, 1000);
            let p = Pair::fresh();
            p.check_static_sum(v, &format!("row03 #{i}"));
        }
    }

    /// Row 4 — fresh state, one call, randomized full-domain updates.
    #[test]
    fn row04_fresh_single_full_domain_update_random() {
        let mut rng = Rng::new(0x0404_0004);
        for i in 0..512 {
            let v = rng.i32_any();
            let p = Pair::fresh();
            p.check_static_sum(v, &format!("row04 #{i}"));
        }
    }

    /// Row 5 — fresh state, exactly two calls: the first accumulation step,
    /// full-domain (so roughly half the samples overflow the add).
    #[test]
    fn row05_fresh_two_calls_full_domain() {
        let mut rng = Rng::new(0x0505_0005);
        for i in 0..512 {
            let a = rng.i32_any();
            let b = rng.i32_any();
            let p = Pair::fresh();
            let s1 = p.check_static_sum(a, &format!("row05 #{i} first"));
            let s2 = p.check_static_sum(b, &format!("row05 #{i} second"));
            assert_eq!(s1, a);
            assert_eq!(s2, a.wrapping_add(b));
        }
    }

    /// Row 6 — many calls, small values, no-overflow regime.
    #[test]
    fn row06_many_calls_small_values_no_overflow() {
        let mut rng = Rng::new(0x0606_0006);
        let p = Pair::fresh();
        for i in 0..1024 {
            let v = rng.i32_in(-1000, 1000);
            p.check_static_sum(v, &format!("row06 step {i}"));
        }
    }

    /// Row 7 — many calls, full-domain values: the add-overflow regime is hit
    /// over and over, so wrap-around agreement is checked thousands of times.
    #[test]
    fn row07_many_calls_full_domain_overflowing() {
        let mut rng = Rng::new(0x0707_0007);
        let p = Pair::fresh();
        for i in 0..4096 {
            let v = rng.i32_any();
            p.check_static_sum(v, &format!("row07 step {i}"));
        }
    }

    /// Row 8 — many calls, all-zero sequence (identity sequence).
    #[test]
    fn row08_many_zero_updates_are_identity() {
        let p = Pair::fresh();
        for i in 0..256 {
            let got = p.check_static_sum(0, &format!("row08 step {i}"));
            assert_eq!(got, 0);
        }
    }

    /// Row 9 — monotone `+1` walk up then monotone `-1` walk back to 0.
    #[test]
    fn row09_monotone_up_then_down() {
        let p = Pair::fresh();
        for i in 0..500 {
            p.check_static_sum(1, &format!("row09 up {i}"));
        }
        for i in 0..500 {
            p.check_static_sum(-1, &format!("row09 down {i}"));
        }
        assert_eq!(p.check_static_sum(0, "row09 final"), 0);
    }

    /// Row 10 — the whole boundary set in a seeded random order.
    #[test]
    fn row10_boundary_set_random_order() {
        let mut vals = boundary_i32s();
        let mut rng = Rng::new(0x0A0A_000A);
        // Fisher-Yates with the seeded PRNG.
        for i in (1..vals.len()).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            vals.swap(i, j);
        }
        let p = Pair::fresh();
        for (i, &v) in vals.iter().enumerate() {
            p.check_static_sum(v, &format!("row10 step {i} value {v}"));
        }
    }

    /// Row 11 — pre-seeded extreme state, then randomized full-domain updates.
    #[test]
    fn row11_extreme_seeded_state_then_random() {
        for seed_val in [i32::MAX, i32::MIN] {
            let mut rng = Rng::new(0x0B0B_0000 ^ seed_val as u32 as u64);
            let p = Pair::fresh();
            let s = p.check_static_sum(seed_val, "row11 seed");
            assert_eq!(s, seed_val);
            for i in 0..1024 {
                let v = rng.i32_any();
                p.check_static_sum(v, &format!("row11 seed={seed_val} step {i}"));
            }
        }
    }

    /// Row 12 — `driver(0)` on fresh state: 10 identical lines, no overflow.
    #[test]
    fn row12_driver_stride_zero_fresh() {
        let p = Pair::fresh();
        let out = p.check_driver(0, "row12");
        assert_eq!(out, b"0\n".repeat(10), "unexpected driver(0) output");
    }

    /// Row 13 — `driver(1)` and `driver(-1)` on fresh state.
    #[test]
    fn row13_driver_stride_plus_minus_one_fresh() {
        // sum after each iteration: 0,1,3,6,10,15,21,28,36,45
        let p = Pair::fresh();
        let out = p.check_driver(1, "row13 +1");
        assert_eq!(out, b"0\n1\n3\n6\n10\n15\n21\n28\n36\n45\n".to_vec());

        let q = Pair::fresh();
        let out = q.check_driver(-1, "row13 -1");
        assert_eq!(out, b"0\n-1\n-3\n-6\n-10\n-15\n-21\n-28\n-36\n-45\n".to_vec());
    }

    /// Row 14 — fresh state, randomized small strides (no-overflow regime).
    #[test]
    fn row14_driver_small_random_stride_fresh() {
        let mut rng = Rng::new(0x0E0E_000E);
        for i in 0..256 {
            let s = rng.i32_in(-1000, 1000);
            let p = Pair::fresh();
            let out = p.check_driver(s, &format!("row14 #{i}"));
            assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
        }
    }

    /// Row 15 — fresh state, randomized full-domain strides: the `i * stride`
    /// multiply overflows and the accumulated add overflows too.
    #[test]
    fn row15_driver_full_domain_random_stride_fresh() {
        let mut rng = Rng::new(0x0F0F_000F);
        for i in 0..256 {
            let s = rng.i32_any();
            let p = Pair::fresh();
            let out = p.check_driver(s, &format!("row15 #{i}"));
            assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
        }
    }

    /// Row 16 — every boundary stride, fresh instance each (multiply-overflow).
    #[test]
    fn row16_driver_boundary_strides_fresh() {
        for &s in boundary_i32s().iter() {
            let p = Pair::fresh();
            p.check_driver(s, &format!("row16 stride={s}"));
        }
    }

    /// Row 17 — `stride = 0x2000_0000`: `i * stride` stays in range for the
    /// first few `i` while the *accumulated* sum overflows mid-loop.
    #[test]
    fn row17_driver_add_overflow_only_regime() {
        for s in [0x2000_0000i32, 0x1000_0000, -0x2000_0000, 0x0800_0000] {
            let p = Pair::fresh();
            let out = p.check_driver(s, &format!("row17 stride={s}"));
            assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 10);
        }
    }

    /// Row 18 — `driver` called repeatedly on one instance (state carries
    /// across calls, 8 * 10 = 80 lines).
    #[test]
    fn row18_driver_called_repeatedly_same_instance() {
        let mut rng = Rng::new(0x1212_0012);
        for i in 0..32 {
            let s = rng.i32_any();
            let p = Pair::fresh();
            for call in 0..8 {
                p.check_driver(s, &format!("row18 #{i} stride={s} call {call}"));
            }
        }
    }

    /// Row 19 — non-fresh state: seed with N random `static_sum` calls, then
    /// run `driver` (the composed pipeline over dirty state).
    #[test]
    fn row19_driver_over_dirty_state() {
        let mut rng = Rng::new(0x1313_0013);
        for i in 0..128 {
            let n = rng.below(8) as usize;
            let seeds: Vec<i32> = (0..n).map(|_| rng.i32_any()).collect();
            let stride = rng.i32_any();
            let p = Pair::fresh();
            p.seed(&seeds, &format!("row19 #{i}"));
            p.check_driver(stride, &format!("row19 #{i} stride={stride} after {n} seeds"));
        }
    }

    /// Row 20 — randomized interleaving of both entry points on one shared
    /// instance: return values and stdout bytes are both compared.
    #[test]
    fn row20_interleaved_static_sum_and_driver() {
        let mut rng = Rng::new(0x1414_0014);
        let p = Pair::fresh();
        for step in 0..256 {
            let v = if rng.below(2) == 0 {
                rng.i32_in(-100_000, 100_000)
            } else {
                rng.i32_any()
            };
            if rng.below(2) == 0 {
                p.check_static_sum(v, &format!("row20 step {step} static_sum"));
            } else {
                p.check_driver(v, &format!("row20 step {step} driver"));
            }
        }
    }

    /// Row 21 — the project builds no executable, so there is no binary stdout
    /// to compare.  Asserted against the actual CMakeLists to keep this
    /// claim honest if the C build ever grows an `add_executable`.
    #[test]
    fn row21_no_binary_target() {
        let cml = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/CMakeLists.txt");
        let text = std::fs::read_to_string(&cml).expect("read CMakeLists.txt");
        assert!(
            !text.to_lowercase().contains("add_executable"),
            "c_src now builds an executable; a binary stdout comparison must be added"
        );
        let cargo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let cargo_text = std::fs::read_to_string(&cargo).expect("read Cargo.toml");
        assert!(
            !cargo_text.contains("[[bin]]"),
            "the Rust crate now builds a binary; add a stdout comparison"
        );
        // The `src/` tree must likewise contain no `main`.
        let bin_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/bin");
        assert!(!bin_dir.exists(), "src/bin appeared; add a stdout comparison");
    }
}

// ===========================================================================
// Phase D — symbol parity
// ===========================================================================
mod symbols {
    use super::harness::{c_so_path, rust_so_path};
    use std::process::Command;

    fn defined_dynamic_symbols(path: &std::path::Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", path.to_str().unwrap()])
            .output()
            .expect("run nm");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// Every symbol the C `.so` exports must also be exported by the Rust
    /// `.so`, under the exact same name.
    #[test]
    fn rust_so_exports_every_c_symbol() {
        let c = defined_dynamic_symbols(&c_so_path());
        let r = defined_dynamic_symbols(&rust_so_path());
        assert!(!c.is_empty(), "nm found no exported symbols in the C .so");

        let missing: Vec<&String> = c.iter().filter(|s| !r.contains(s)).collect();
        assert!(
            missing.is_empty(),
            "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}"
        );

        // Both public functions from staticloop.h must really be there.
        for want in ["static_sum", "driver"] {
            assert!(c.contains(&want.to_string()), "C .so lacks {want}");
            assert!(r.contains(&want.to_string()), "Rust .so lacks {want}");
        }
    }

    /// The Rust `.so` must not have undefined references outside libc /
    /// libgcc-unwind / ld.so.
    #[test]
    fn rust_so_has_no_undefined_non_libc_symbols() {
        let out = Command::new("nm")
            .args(["-D", "-u", rust_so_path().to_str().unwrap()])
            .output()
            .expect("run nm -u");
        assert!(out.status.success());
        let text = String::from_utf8_lossy(&out.stdout);
        let known_prefixes = [
            "_ITM_", "_Unwind_", "__cxa_", "__errno_location", "__gmon_start__",
            "__tls_get_addr", "__libc_", "__assert",
        ];
        let known_exact = [
            "abort", "bcmp", "calloc", "close", "dl_iterate_phdr", "free", "fstat", "fstat64",
            "getcwd", "getenv", "gettid", "lseek", "lseek64", "malloc", "memcmp", "memcpy",
            "memmove", "memset", "mmap", "mmap64", "munmap", "open", "open64", "posix_memalign",
            "printf", "pthread_key_create", "pthread_key_delete", "pthread_getspecific",
            "pthread_setspecific", "pthread_mutex_lock", "pthread_mutex_unlock", "read",
            "readlink", "realloc", "realpath", "stat", "stat64", "statx", "strlen", "syscall",
            "sysconf", "write", "writev", "fflush", "dup", "dup2", "fwrite", "puts",
        ];
        let mut bad = Vec::new();
        for line in text.lines() {
            let Some(sym) = line.split_whitespace().last() else {
                continue;
            };
            let base = sym.split('@').next().unwrap();
            if known_prefixes.iter().any(|p| base.starts_with(p))
                || known_exact.contains(&base)
            {
                continue;
            }
            bad.push(base.to_string());
        }
        assert!(
            bad.is_empty(),
            "Rust .so has unexpected undefined (non-libc) symbols: {bad:?}"
        );
    }
}
