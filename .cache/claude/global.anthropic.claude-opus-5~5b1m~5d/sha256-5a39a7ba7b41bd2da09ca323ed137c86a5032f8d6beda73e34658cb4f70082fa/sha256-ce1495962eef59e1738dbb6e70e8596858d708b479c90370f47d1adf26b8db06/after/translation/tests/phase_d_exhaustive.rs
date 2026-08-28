//! Phase D — exhaustive differential sweep.
//!
//! `float2half` takes a single `float` and no other state, so its **entire**
//! input domain is the 2^32 `f32` bit patterns. That makes exhaustive
//! verification possible, and an exhaustive pass subsumes every row of
//! `CONFIGS.md` (row 22) and `ERRORS.md` (row 12): there is provably no input
//! on which the two implementations differ.
//!
//! Both implementations are called through their `.so` exports via
//! `libloading`. The work is split across threads; the C function is pure and
//! reentrant (it only reads two `static` const tables) and so is the Rust one,
//! so sharding is sound.
//!
//! Environment knobs:
//! * `EXHAUSTIVE_STRIDE` — sample every Nth bit pattern instead of all of them
//!   (default `1` = truly exhaustive). Used to keep CI time bounded.
//! * `EXHAUSTIVE_THREADS` — worker count (default: available parallelism).

mod common;

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use common::*;

const TOTAL: u64 = 1u64 << 32;

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&v| v > 0)
        .unwrap_or(default)
}

#[test]
fn exhaustive_all_2_pow_32_bit_patterns() {
    let libs = Libs::load();
    let c_fn = libs.c_raw();
    let rust_fn = libs.rust_raw();

    let stride = env_u64("EXHAUSTIVE_STRIDE", 1);
    let threads = env_u64(
        "EXHAUSTIVE_THREADS",
        std::thread::available_parallelism()
            .map(|n| n.get() as u64)
            .unwrap_or(4),
    )
    .max(1);

    eprintln!(
        "exhaustive sweep: C={} Rust={}\n  stride={stride} threads={threads} \
         (~{} inputs per implementation)",
        libs.c_path.display(),
        libs.rust_path.display(),
        TOTAL / stride,
    );

    let checked = AtomicU64::new(0);
    let mismatches = AtomicU64::new(0);
    let first_mismatch: Mutex<Option<(u32, u16, u16)>> = Mutex::new(None);

    let chunk = TOTAL.div_ceil(threads);
    let start_time = std::time::Instant::now();

    std::thread::scope(|scope| {
        for t in 0..threads {
            let checked = &checked;
            let mismatches = &mismatches;
            let first_mismatch = &first_mismatch;
            scope.spawn(move || {
                let lo = t * chunk;
                let hi = ((t + 1) * chunk).min(TOTAL);
                // Keep every thread on the same stride lattice.
                let mut i = lo.next_multiple_of(stride);
                let mut local_checked: u64 = 0;
                let mut local_bad: u64 = 0;
                while i < hi {
                    let bits = i as u32;
                    let x = f32::from_bits(bits);
                    let a = unsafe { c_fn(x) };
                    let b = unsafe { rust_fn(x) };
                    if a != b {
                        local_bad += 1;
                        let mut slot = first_mismatch.lock().unwrap();
                        if slot.is_none_or(|(pb, _, _)| bits < pb) {
                            *slot = Some((bits, a, b));
                        }
                    }
                    local_checked += 1;
                    i += stride;
                }
                checked.fetch_add(local_checked, Ordering::Relaxed);
                mismatches.fetch_add(local_bad, Ordering::Relaxed);
            });
        }
    });

    let elapsed = start_time.elapsed();
    let n = checked.load(Ordering::Relaxed);
    let bad = mismatches.load(Ordering::Relaxed);
    eprintln!(
        "  checked {n} inputs in {:.1?} ({:.0} M inputs/s), mismatches: {bad}",
        elapsed,
        (n as f64 / elapsed.as_secs_f64()) / 1e6
    );

    if let Some((bits, c, r)) = *first_mismatch.lock().unwrap() {
        let x = f32::from_bits(bits);
        panic!(
            "EXHAUSTIVE DIVERGENCE: {bad} mismatching input(s) out of {n}.\n  \
             lowest mismatching input bits 0x{bits:08X} (f32 {x:e}, sign={} exp={} \
             mant=0x{:06X}, j={})\n  C returned 0x{c:04X}, Rust returned 0x{r:04X}",
            bits >> 31,
            (bits >> 23) & 0xFF,
            bits & 0x7F_FFFF,
            index_of(bits),
        );
    }

    // Guard against a silently empty sweep.
    let expected = TOTAL.div_ceil(stride);
    assert_eq!(
        n, expected,
        "the sweep visited {n} inputs but should have visited {expected}"
    );
    assert_eq!(bad, 0, "there must be zero mismatches");
}

/// Independent cross-check of the exhaustive sweep: verify the C `.so` matches
/// the behavioural model parsed out of the C *source* tables, over the whole
/// 2^32 domain. If this passes and the sweep above passes, then the Rust
/// implements the C tables exactly.
#[test]
fn exhaustive_c_matches_table_model() {
    let libs = Libs::load();
    let c_fn = libs.c_raw();
    let (base, shift) = read_c_tables();

    let stride = env_u64("EXHAUSTIVE_STRIDE", 1);
    let threads = env_u64(
        "EXHAUSTIVE_THREADS",
        std::thread::available_parallelism()
            .map(|n| n.get() as u64)
            .unwrap_or(4),
    )
    .max(1);

    let mismatches = AtomicU64::new(0);
    let first: Mutex<Option<(u32, u16, u16)>> = Mutex::new(None);
    let chunk = TOTAL.div_ceil(threads);

    std::thread::scope(|scope| {
        for t in 0..threads {
            let mismatches = &mismatches;
            let first = &first;
            let base = &base;
            let shift = &shift;
            scope.spawn(move || {
                let lo = t * chunk;
                let hi = ((t + 1) * chunk).min(TOTAL);
                let mut i = lo.next_multiple_of(stride);
                let mut local_bad = 0u64;
                while i < hi {
                    let bits = i as u32;
                    let j = ((bits >> 23) & 0x1ff) as usize;
                    let model = (base[j] as u32)
                        .wrapping_add((bits & 0x007f_ffff) >> shift[j] as u32)
                        as u16;
                    let got = unsafe { c_fn(f32::from_bits(bits)) };
                    if got != model {
                        local_bad += 1;
                        let mut slot = first.lock().unwrap();
                        if slot.is_none_or(|(pb, _, _)| bits < pb) {
                            *slot = Some((bits, got, model));
                        }
                    }
                    i += stride;
                }
                mismatches.fetch_add(local_bad, Ordering::Relaxed);
            });
        }
    });

    if let Some((bits, got, model)) = *first.lock().unwrap() {
        panic!(
            "C .so deviates from the model parsed from its own source at bits \
             0x{bits:08X}: .so gave 0x{got:04X}, table model says 0x{model:04X}"
        );
    }
    assert_eq!(mismatches.load(Ordering::Relaxed), 0);
}

/// Exhaustive over every input whose result is value-dependent in a way the
/// simpler rows cannot reach: the two `shift == 13` special indices (`j == 255`
/// and `j == 511`, i.e. Inf/NaN) across all 2^23 mantissas, and the ten
/// varying-shift subnormal exponents across all 2^23 mantissas each.
///
/// Runs unconditionally (it is not affected by `EXHAUSTIVE_STRIDE`), so even a
/// strided CI run still gets exhaustive coverage of the trickiest regions.
#[test]
fn exhaustive_special_regions_unstrided() {
    let libs = Libs::load();
    let c_fn = libs.c_raw();
    let rust_fn = libs.rust_raw();

    // Exponents worth an unconditional full mantissa sweep:
    //  255      -> Inf/NaN, shift 13 (payload propagates)
    //  103..112 -> half subnormals, a different shift for each exponent
    //  113, 142 -> first/last half-normal exponent
    //  143      -> first saturating exponent
    //  0        -> float zero/subnormal
    let exponents: Vec<u32> = {
        let mut v = vec![0u32, 255, 113, 142, 143];
        v.extend(103..=112);
        v
    };

    let mismatches = AtomicU64::new(0);
    let first: Mutex<Option<(u32, u16, u16)>> = Mutex::new(None);

    std::thread::scope(|scope| {
        for &exp in &exponents {
            for sign in 0..2u32 {
                let mismatches = &mismatches;
                let first = &first;
                scope.spawn(move || {
                    let mut local_bad = 0u64;
                    for m in 0..=0x7F_FFFFu32 {
                        let bits = (sign << 31) | (exp << 23) | m;
                        let x = f32::from_bits(bits);
                        let a = unsafe { c_fn(x) };
                        let b = unsafe { rust_fn(x) };
                        if a != b {
                            local_bad += 1;
                            let mut slot = first.lock().unwrap();
                            if slot.is_none_or(|(pb, _, _)| bits < pb) {
                                *slot = Some((bits, a, b));
                            }
                        }
                    }
                    mismatches.fetch_add(local_bad, Ordering::Relaxed);
                });
            }
        }
    });

    if let Some((bits, c, r)) = *first.lock().unwrap() {
        panic!(
            "DIVERGENCE in a special region at bits 0x{bits:08X} \
             (exp={}, mant=0x{:06X}): C 0x{c:04X}, Rust 0x{r:04X}",
            (bits >> 23) & 0xFF,
            bits & 0x7F_FFFF
        );
    }
    assert_eq!(mismatches.load(Ordering::Relaxed), 0);
}

/// Symbol parity, asserted from inside the test suite (Phase D).
///
/// Runs `nm -D --defined-only` on both `.so` files and requires that every
/// symbol exported by the C library is also exported by the Rust library,
/// under the exact same name.
#[test]
fn symbol_parity_c_so_vs_rust_so() {
    use std::process::Command;

    let libs = Libs::load();

    fn exported(path: &std::path::Path) -> Vec<String> {
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
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let (_addr, kind, name) = (it.next()?, it.next()?, it.next()?);
                // Global/weak text or data symbols only.
                if matches!(kind, "T" | "D" | "B" | "R" | "W" | "V" | "G" | "S") {
                    Some(name.to_string())
                } else {
                    None
                }
            })
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let c_syms = exported(&libs.c_path);
    let rust_syms = exported(&libs.rust_path);

    eprintln!("C .so exports {} symbol(s): {c_syms:?}", c_syms.len());
    eprintln!("Rust .so exports {} symbol(s): {rust_syms:?}", rust_syms.len());

    assert!(
        c_syms.contains(&"float2half".to_string()),
        "sanity: the C .so should export float2half"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is MISSING {} symbol(s) exported by the C .so: {missing:?}\n\
         Every C symbol must be exported by the Rust .so with the exact same name.",
        missing.len()
    );

    // The two private tables must NOT be exported by either library.
    for forbidden in ["m__base", "m__shift", "M_BASE", "M_SHIFT"] {
        assert!(
            !c_syms.iter().any(|s| s == forbidden),
            "C .so unexpectedly exports {forbidden}"
        );
        assert!(
            !rust_syms.iter().any(|s| s == forbidden),
            "Rust .so exports {forbidden}, but it has internal linkage in C"
        );
    }
}
