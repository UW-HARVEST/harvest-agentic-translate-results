//! Phase B — valid-path differential tests, gated on `CONFIGS.md`.
//!
//! Every row of `CONFIGS.md` (the 28 reachable internal branch signatures plus
//! the shape rows S1..S11) gets a test here. Both implementations are invoked
//! only through their `.so` exports.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Rows 1..28 — the reachable internal branch signatures.
//
// The five pipeline stages are `static` in the C, so they are not callable
// through either `.so` (see SYMBOLS.md). Their branch arms are instead reached
// by driving `tritanopia` with inputs known to select them. The witness inputs
// live in `tests/data/signatures.txt`, generated from the C itself by
// instrumenting a separate copy of `lib.c` (c_src untouched), so the
// classification is ground truth rather than a guess.
// ---------------------------------------------------------------------------

struct Sig {
    remove_gamma_arms: String,
    apply_gamma_arms: String,
    denorm_buckets: String,
    count: u64,
    witnesses: Vec<Rgb255>,
}

fn load_signatures() -> Vec<Sig> {
    let raw = include_str!("data/signatures.txt");
    let mut out: Vec<Sig> = Vec::new();
    for line in raw.lines() {
        let f: Vec<&str> = line.split_whitespace().collect();
        match f.first().copied() {
            Some("SIG") => out.push(Sig {
                remove_gamma_arms: f[1].to_string(),
                apply_gamma_arms: f[2].to_string(),
                denorm_buckets: f[3].to_string(),
                count: f[4].parse().unwrap(),
                witnesses: Vec::new(),
            }),
            Some("W") => {
                let x = Rgb255::new(
                    f[1].parse().unwrap(),
                    f[2].parse().unwrap(),
                    f[3].parse().unwrap(),
                );
                out.last_mut().expect("W before SIG").witnesses.push(x);
            }
            _ => {}
        }
    }
    out
}

#[test]
fn configs_rows_1_to_28_all_branch_signatures() {
    let sigs = load_signatures();
    assert_eq!(
        sigs.len(),
        28,
        "CONFIGS.md declares 28 reachable signatures; data file has {}",
        sigs.len()
    );

    let mut total_checked = 0usize;
    for (i, s) in sigs.iter().enumerate() {
        let row = i + 1;
        assert!(
            !s.witnesses.is_empty(),
            "row {row} has no witness inputs — cannot verify it"
        );

        // 1) every stored witness (stride-sampled across the whole cube)
        let label = format!(
            "CONFIGS row {row} (removeGamma={} applyGamma={} denorm={})",
            s.remove_gamma_arms, s.apply_gamma_arms, s.denorm_buckets
        );
        total_checked += assert_same_all(&label, s.witnesses.iter().copied());

        // 2) randomized neighbourhood around each witness: perturb channels by
        //    small deltas so we cover value-dependent behaviour rather than a
        //    single hand-picked point. Seed is derived from the row index, so
        //    the whole run is reproducible.
        let mut rng = Rng::new(0xC0FFEE_0000 + row as u64);
        for w in &s.witnesses {
            for _ in 0..8 {
                let d = rng.next_u64();
                let jitter = |v: u8, sh: u32| -> u8 {
                    let delta = ((d >> sh) & 0x7) as i32 - 3;
                    (v as i32 + delta).clamp(0, 255) as u8
                };
                let x = Rgb255::new(jitter(w.r, 0), jitter(w.g, 8), jitter(w.b, 16));
                assert_same(x);
                total_checked += 1;
            }
        }

        // Sanity: the declared population must be positive.
        assert!(s.count > 0, "row {row} declares count 0");
    }

    assert!(
        total_checked > 20_000,
        "expected a substantial number of checks, got {total_checked}"
    );
    eprintln!("Phase B rows 1-28: {total_checked} differential checks passed");
}

/// Rows 1..28 again, but each signature family isolated so a failure names the
/// exact family. Groups the 28 rows by the R-channel `cbDenorm` bucket, which is
/// the axis carrying the undefined-behaviour conversions.
#[test]
fn configs_denorm_bucket_families() {
    let sigs = load_signatures();
    for (family, digit) in [("negative-wrap", '0'), ("in-range", '1'), ("overflow-wrap", '2')] {
        let inputs: Vec<Rgb255> = sigs
            .iter()
            .filter(|s| s.denorm_buckets.starts_with(digit))
            .flat_map(|s| s.witnesses.iter().copied())
            .collect();
        assert!(
            !inputs.is_empty(),
            "no witnesses for the {family} cbDenorm family — CONFIGS.md says it is reachable"
        );
        let n = assert_same_all(&format!("cbDenorm {family} family"), inputs);
        eprintln!("  cbDenorm {family:>13} family: {n} inputs OK");
    }
}

// ---------------------------------------------------------------------------
// Shape rows S1..S11
// ---------------------------------------------------------------------------

/// S1 — all 8 corners of the RGB cube.
#[test]
fn s1_cube_corners() {
    let mut v = Vec::new();
    for &r in &[0u8, 255] {
        for &g in &[0u8, 255] {
            for &b in &[0u8, 255] {
                v.push(Rgb255::new(r, g, b));
            }
        }
    }
    assert_eq!(assert_same_all("S1 cube corners", v), 8);
}

/// S2 — one step either side of the `cbRemoveGammaRGB` threshold
/// (`byte/255 > 0.04045` flips between 10 and 11), in every channel combination.
#[test]
fn s2_remove_gamma_threshold_boundary() {
    let mut v = Vec::new();
    for &r in &[9u8, 10, 11, 12] {
        for &g in &[9u8, 10, 11, 12] {
            for &b in &[9u8, 10, 11, 12] {
                v.push(Rgb255::new(r, g, b));
            }
        }
    }
    assert_eq!(assert_same_all("S2 threshold boundary", v), 64);
}

/// S3 — the 256 greys.
#[test]
fn s3_greys() {
    let v: Vec<Rgb255> = (0..=255u8).map(|x| Rgb255::new(x, x, x)).collect();
    assert_eq!(assert_same_all("S3 greys", v), 256);
}

/// S4 — single-channel ramps (isolates each matrix column).
#[test]
fn s4_single_channel_ramps() {
    for (name, f) in [
        ("R", (|v| Rgb255::new(v, 0, 0)) as fn(u8) -> Rgb255),
        ("G", |v| Rgb255::new(0, v, 0)),
        ("B", |v| Rgb255::new(0, 0, v)),
    ] {
        let v: Vec<Rgb255> = (0..=255u8).map(f).collect();
        assert_eq!(assert_same_all(&format!("S4 ramp {name}"), v), 256);
    }
}

/// S5 — saturated-pair ramps (drives the strongly out-of-range denorm region).
#[test]
fn s5_saturated_pair_ramps() {
    for (name, f) in [
        ("R", (|v| Rgb255::new(v, 255, 255)) as fn(u8) -> Rgb255),
        ("G", |v| Rgb255::new(255, v, 255)),
        ("B", |v| Rgb255::new(255, 255, v)),
    ] {
        let v: Vec<Rgb255> = (0..=255u8).map(f).collect();
        assert_eq!(assert_same_all(&format!("S5 sat ramp {name}"), v), 256);
    }
}

/// S6 — dense low band `0..=12`: the linear arms of both gamma functions.
#[test]
fn s6_low_band_dense() {
    let mut v = Vec::new();
    for r in 0..=12u8 {
        for g in 0..=12u8 {
            for b in 0..=12u8 {
                v.push(Rgb255::new(r, g, b));
            }
        }
    }
    assert_eq!(assert_same_all("S6 low band", v), 13 * 13 * 13);
}

/// S7 — dense high band `243..=255`: the overflow-wrap region.
#[test]
fn s7_high_band_dense() {
    let mut v = Vec::new();
    for r in 243..=255u8 {
        for g in 243..=255u8 {
            for b in 243..=255u8 {
                v.push(Rgb255::new(r, g, b));
            }
        }
    }
    assert_eq!(assert_same_all("S7 high band", v), 13 * 13 * 13);
}

/// S8 — seeded pseudorandom property-style sweep.
#[test]
fn s8_randomized_property_sweep() {
    let mut rng = Rng::new(0x5EED_1234_5678_9ABC);
    let v: Vec<Rgb255> = (0..400_000).map(|_| rng.next_rgb()).collect();
    assert_eq!(assert_same_all("S8 random sweep", v), 400_000);
}

/// S9 — exhaustive: all 16,777,216 inputs. The input domain of the entire public
/// API is a 3-byte struct, so this is a complete proof of equivalence for every
/// reachable input, subsuming all other rows.
#[test]
fn s9_exhaustive_all_16m() {
    let libs = libs();
    let (c, r) = (libs.c, libs.r);
    let mut bad = 0usize;
    let mut first: Vec<String> = Vec::new();

    for x in all_inputs() {
        let cv = unsafe { c(x) };
        let rv = unsafe { r(x) };
        if cv != rv {
            bad += 1;
            if first.len() < 20 {
                first.push(format!(
                    "  in=({:3},{:3},{:3})  C=({:3},{:3},{:3})  Rust=({:3},{:3},{:3})",
                    x.r, x.g, x.b, cv.r, cv.g, cv.b, rv.r, rv.g, rv.b
                ));
            }
        }
    }

    assert!(
        bad == 0,
        "EXHAUSTIVE: {bad} of 16777216 inputs diverge; first divergences:\n{}",
        first.join("\n")
    );
    eprintln!("S9 exhaustive: all 16777216 inputs byte-identical");
}

/// S10 — statelessness: the library has no globals, so results must not depend
/// on call order or history. Interleave, repeat, and reverse-order the calls.
#[test]
fn s10_stateless_call_order() {
    let mut rng = Rng::new(0xA5A5_0000_1111_2222);
    let probes: Vec<Rgb255> = (0..2000).map(|_| rng.next_rgb()).collect();

    // reference pass
    let c_ref: Vec<Rgb255> = probes.iter().map(|&x| call_c(x)).collect();
    let r_ref: Vec<Rgb255> = probes.iter().map(|&x| call_r(x)).collect();
    assert_eq!(c_ref, r_ref, "S10 straight pass diverges");

    // reversed order
    for (i, &x) in probes.iter().enumerate().rev() {
        assert_eq!(call_c(x), c_ref[i], "C not stateless at {i}");
        assert_eq!(call_r(x), r_ref[i], "Rust not stateless at {i}");
    }

    // repeated / interleaved, with the other library's calls in between
    for (i, &x) in probes.iter().enumerate() {
        for _ in 0..3 {
            let a = call_c(x);
            let _ = call_r(probes[(i * 7 + 3) % probes.len()]);
            let b = call_c(x);
            let d = call_r(x);
            assert_eq!(a, b, "C varied across repeats at {i}");
            assert_eq!(d, r_ref[i], "Rust varied across interleaving at {i}");
            assert_eq!(a, d, "C/Rust diverge under interleaving at {i}");
        }
    }
}

/// Confirms the harness really did load two distinct files and that both export
/// the symbol — guards against accidentally testing one library against itself.
#[test]
fn harness_loads_two_distinct_shared_objects() {
    let l = libs();
    let c = l.c_path.canonicalize().unwrap();
    let r = l.r_path.canonicalize().unwrap();
    assert_ne!(c, r, "both handles point at the same file");
    assert!(
        c.to_string_lossy().contains("c_src"),
        "C library not from c_src: {}",
        c.display()
    );
    assert!(
        r.to_string_lossy().contains("target"),
        "Rust library not from target/: {}",
        r.display()
    );
    eprintln!("C   .so: {}", c.display());
    eprintln!("Rust.so: {}", r.display());
    // The two function pointers must be different code addresses.
    assert_ne!(l.c as usize, l.r as usize, "same function address");
}
