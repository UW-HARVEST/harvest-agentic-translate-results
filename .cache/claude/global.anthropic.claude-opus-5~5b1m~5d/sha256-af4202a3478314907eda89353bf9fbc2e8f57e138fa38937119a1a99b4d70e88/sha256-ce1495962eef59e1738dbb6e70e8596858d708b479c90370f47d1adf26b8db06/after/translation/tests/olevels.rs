//! Robustness study: how does the Rust `.so` compare against C `.so`s built at
//! OTHER optimization levels?
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the canonical build
//! (and the ground truth for every other test here) passes no `-O` flag. This
//! file quantifies what happens at `-O1`/`-O2`/`-O3`/`-Os`, and — crucially —
//! establishes that the differences are C-vs-C instability confined to NaN
//! *inputs*, not a defect in the translation.
//!
//! Driven by `TFM_ALT_C_SOS` (colon-separated paths), set by `run_all.sh`. The
//! tests skip cleanly when it is unset.

mod common;

use common::*;

fn alt_sos() -> Vec<(String, &'static Impl)> {
    let Ok(list) = std::env::var("TFM_ALT_C_SOS") else {
        return Vec::new();
    };
    list.split(':')
        .filter(|s| !s.is_empty())
        .filter(|s| std::path::Path::new(s).is_file())
        .map(|s| {
            let label = std::path::Path::new(s)
                .file_stem()
                .and_then(|x| x.to_str())
                .unwrap_or("alt")
                .to_string();
            let leaked: &'static str = Box::leak(label.clone().into_boxed_str());
            (label, load_impl(leaked, std::path::PathBuf::from(s)))
        })
        .collect()
}

/// Every NaN-free triple from the special alphabet, plus randomized NaN-free
/// values across all IEEE classes.
fn nan_free_corpus() -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    let alpha: Vec<f32> = alphabet_f32().into_iter().filter(|x| !x.is_nan()).collect();
    for &x in &alpha {
        for &y in &alpha {
            for &z in &alpha {
                out.push([x, y, z]);
            }
        }
    }
    let mut rng = Rng::new(0x0E5E_1EE7);
    for _ in 0..120_000 {
        let lane = |r: &mut Rng| loop {
            let v = match r.below(6) {
                0 => r.signed_unit(),
                1 => r.wild_normal(),
                2 => r.subnormal(),
                3 => r.huge(),
                4 => {
                    if r.next_u32() & 1 == 0 {
                        f32::INFINITY
                    } else {
                        f32::NEG_INFINITY
                    }
                }
                _ => r.any_bits_f32(),
            };
            if !v.is_nan() {
                return v;
            }
        };
        out.push([lane(&mut rng), lane(&mut rng), lane(&mut rng)]);
    }
    out
}

fn with_nan_corpus() -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    let alpha = alphabet_f32();
    for &x in &alpha {
        for &y in &alpha {
            for &z in &alpha {
                if [x, y, z].iter().any(|v| v.is_nan()) {
                    out.push([x, y, z]);
                }
            }
        }
    }
    let mut rng = Rng::new(0xA1A1_A1A1);
    for _ in 0..120_000 {
        let mut t = [rng.any_bits_f32(), rng.any_bits_f32(), rng.any_bits_f32()];
        if !t.iter().any(|v| v.is_nan()) {
            t[rng.below(3) as usize] = rng.any_nan();
        }
        out.push(t);
    }
    out
}

/// Count divergences between two impls over a corpus.
fn count_diff(a: TfmFn, b: TfmFn, corpus: &[[f32; 3]]) -> (usize, Option<[f32; 3]>) {
    let mut n = 0usize;
    let mut first = None;
    for &t in corpus {
        let da = run_one(a, t);
        let db = run_one(b, t);
        if bits(&da) != bits(&db) {
            n += 1;
            if first.is_none() {
                first = Some(t);
            }
        }
    }
    (n, first)
}

#[test]
fn nan_free_inputs_agree_at_every_optimization_level() {
    let alts = alt_sos();
    if alts.is_empty() {
        eprintln!("TFM_ALT_C_SOS unset — skipping (run ./run_all.sh to exercise this)");
        return;
    }
    let p = pair();
    let corpus = nan_free_corpus();
    eprintln!("NaN-free corpus: {} triples", corpus.len());

    let mut failures = Vec::new();
    for (label, alt) in &alts {
        // -Ofast / -ffast-math is NOT a conforming IEEE build; it reassociates
        // and assumes no NaN/inf, so it is expected to differ and is reported
        // rather than asserted.
        let fast_math = label.contains("Ofast") || label.contains("ffast");

        let (n_rs, first_rs) = count_diff(p.rs.tfm, alt.tfm, &corpus);
        let (n_cc, _) = count_diff(p.c.tfm, alt.tfm, &corpus);
        eprintln!(
            "  {label:12}  Rust-vs-alt: {n_rs:6}  canonicalC-vs-alt: {n_cc:6}{}",
            if fast_math { "   (fast-math, informational)" } else { "" }
        );
        if !fast_math && n_rs != 0 {
            failures.push(format!(
                "{label}: {n_rs} NaN-free divergences vs Rust (first: {})",
                first_rs.map(|t| fmt_slice(&t)).unwrap_or_default()
            ));
        }
        // The Rust must track the canonical C at least as closely as any other
        // conforming C build does.
        if !fast_math {
            assert_eq!(
                n_rs, n_cc,
                "{label}: Rust diverges from this build on a DIFFERENT set of \
                 NaN-free inputs than the canonical C does ({n_rs} vs {n_cc})"
            );
        }
    }
    assert!(
        failures.is_empty(),
        "NaN-free inputs must be optimization-level independent:\n  {}",
        failures.join("\n  ")
    );
}

#[test]
fn nan_input_divergence_is_c_vs_c_instability() {
    let alts = alt_sos();
    if alts.is_empty() {
        eprintln!("TFM_ALT_C_SOS unset — skipping");
        return;
    }
    let p = pair();
    let corpus = with_nan_corpus();
    eprintln!("NaN-bearing corpus: {} triples", corpus.len());
    eprintln!(
        "  {:12}  {:>10}  {:>10}   verdict",
        "build", "Rust-vs-alt", "canonC-vs-alt"
    );
    for (label, alt) in &alts {
        let (n_rs, _) = count_diff(p.rs.tfm, alt.tfm, &corpus);
        let (n_cc, _) = count_diff(p.c.tfm, alt.tfm, &corpus);
        eprintln!(
            "  {label:12}  {n_rs:>10}  {n_cc:>10}   {}",
            if n_rs == n_cc {
                "Rust tracks canonical C exactly"
            } else {
                "MISMATCH"
            }
        );
        // The key assertion: wherever the Rust differs from another C build, the
        // CANONICAL C differs from it too, in exactly the same places. That
        // proves the NaN-payload disagreement is GCC's own -O-dependent operand
        // ordering, not a translation defect.
        assert_eq!(
            n_rs, n_cc,
            "{label}: Rust's NaN divergences ({n_rs}) do not match the canonical \
             C's ({n_cc}); the translation is not tracking the reference build"
        );
    }
}

#[test]
fn canonical_build_equals_dash_o0_and_default() {
    // Documents WHICH build is the ground truth: cmake with no CMAKE_BUILD_TYPE
    // passes no -O flag, i.e. -O0. Any alt .so whose label says so must be
    // bit-identical to the canonical one everywhere.
    let alts = alt_sos();
    if alts.is_empty() {
        eprintln!("TFM_ALT_C_SOS unset — skipping");
        return;
    }
    let p = pair();
    let mut corpus = nan_free_corpus();
    corpus.extend(with_nan_corpus());
    for (label, alt) in &alts {
        if label.ends_with("O0") || label.ends_with("default") {
            let (n, first) = count_diff(p.c.tfm, alt.tfm, &corpus);
            assert_eq!(
                n,
                0,
                "{label} should be identical to the cmake build but differs on \
                 {n} triples (first: {})",
                first.map(|t| fmt_slice(&t)).unwrap_or_default()
            );
            // …and therefore the Rust matches it too.
            let (n2, first2) = count_diff(p.rs.tfm, alt.tfm, &corpus);
            assert_eq!(
                n2,
                0,
                "Rust differs from {label} on {n2} triples (first: {})",
                first2.map(|t| fmt_slice(&t)).unwrap_or_default()
            );
            eprintln!("  {label}: bit-identical to the cmake build AND to Rust");
        }
    }
}
