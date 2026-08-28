//! Negative control (mutation testing): proves the differential suite has the
//! POWER to detect translation errors, rather than passing vacuously.
//!
//! Each mutation below is a realistic C-to-Rust transcription slip. The
//! **unmodified** `c_src/src/lib.c` is read, one mutation is applied to an
//! in-memory copy, that copy is compiled into `$TMPDIR` (nothing in `c_src/` is
//! written to), and the mutant `.so` is compared against the Rust `.so` using the
//! very same axis generators and randomized inputs the real Phase B tests use.
//!
//! Every mutant MUST be caught. A mutant that survives means the corresponding
//! real bug would also slip through, and the suite needs strengthening.

mod common;

use common::*;

use std::path::PathBuf;
use std::process::Command;

use libloading::{Library, Symbol};

/// (label, from, to) - `from` must occur in the C source.
const MUTATIONS: &[(&str, &str, &str)] = &[
    // Candidate-selection logic (lines 57-61).
    ("drop 2nd selection if", "if (d2 < d0)", "if (0 && d2 < d0)"),
    ("drop 1st selection if", "if (d1 < d0)", "if (0 && d1 < d0)"),
    ("running-best instead of original d0", "if (d2 < d0)", "if (d2 < d1)"),
    ("swap uni1/uni2 assignment", "uni = uni1;", "uni = uni2;"),
    ("<= instead of < for d1", "if (d1 < d0)", "if (d1 <= d0)"),
    ("<= instead of < for d2", "if (d2 < d0)", "if (d2 <= d0)"),
    // The secondary-target penalty weight (lines 50/53/56).
    ("d3 >> 4 instead of >> 5", "d3 >> 5", "d3 >> 4"),
    ("d3 >> 6 instead of >> 5", "d3 >> 5", "d3 >> 6"),
    // The branchless absolute value (lines 35/41/47/49/52/55).
    ("abs shift 30 instead of 31", ">> 31", ">> 30"),
    // Candidate clamping (lines 8/10).
    ("clamp mask ~15 instead of ~7", "& (~7)", "& (~15)"),
    ("clamp mask ~3 instead of ~7", "& (~7)", "& (~3)"),
    // The lsbit mode switch (lines 12/13/20).
    ("lsbit == 2 instead of == 4", "lsbit == 4", "lsbit == 2"),
    ("lsbit & 2 instead of & 1", "lsbit & 1", "lsbit & 2"),
    ("dither uses | instead of &", "(uni >> 1) & (uni >> 2)", "(uni >> 1) | (uni >> 2)"),
    ("dither shift 3 instead of 2", "(uni >> 1) & (uni >> 2)", "(uni >> 1) & (uni >> 3)"),
    ("force bit0 to 3 instead of 1", "uni |= 1;", "uni |= 3;"),
    // The diff expression (lines 30/36/42) - precedence / constants.
    ("drop the +1 in the multiplier", "(2 * (uni & 7) + 1)", "(2 * (uni & 7))"),
    ("multiplier uses 3* not 2*", "(2 * (uni & 7) + 1)", "(3 * (uni & 7) + 1)"),
    ("mask uni & 15 not & 7 in multiplier", "(2 * (uni & 7) + 1)", "(2 * (uni & 15) + 1)"),
    ("wrong parenthesisation of the product", "((2 * (uni & 7) + 1) * step) / 8", "(2 * (uni & 7) + 1) * (step / 8)"),
    ("divide by 4 instead of 8", ") / 8;", ") / 4;"),
    // Sign-of-diff test (lines 31/37/43).
    ("sign bit 16 instead of 8", "uni & 8", "uni & 16"),
    ("sign bit 4 instead of 8", "uni & 8", "uni & 4"),
    // Candidate construction (lines 6/7).
    ("uni + 2 instead of uni + 1", "uni1 = uni + 1;", "uni1 = uni + 2;"),
    ("uni - 2 instead of uni - 1", "uni2 = uni - 1;", "uni2 = uni - 2;"),
    // Which target feeds which distortion (lines 34/48).
    ("primary distortion uses tgt2", "d0 = tgt - p0;", "d0 = tgt2 - p0;"),
    ("secondary distortion uses tgt", "d3 = tgt2 - p0;", "d3 = tgt - p0;"),
    // Prediction (lines 33/39/45).
    ("p0 subtracts instead of adds diff", "p0 = pred + diff;", "p0 = pred - diff;"),
];

fn scratch() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("encode_quant_mutants")
}

fn c_source() -> String {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/src/lib.c");
    std::fs::read_to_string(p).expect("read c_src/src/lib.c")
}

fn build_mutant(idx: usize, src: &str) -> Option<PathBuf> {
    let dir = scratch();
    std::fs::create_dir_all(&dir).ok()?;
    let c = dir.join(format!("mutant{idx}.c"));
    let so = dir.join(format!("libmutant{idx}.so"));
    std::fs::write(&c, src).ok()?;
    let inc = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src/include");
    let st = Command::new("cc")
        .args(["-shared", "-fPIC", "-O2", "-w"])
        .arg("-I")
        .arg(inc)
        .arg(&c)
        .arg("-o")
        .arg(&so)
        .status()
        .ok()?;
    if st.success() && so.is_file() { Some(so) } else { None }
}

/// The same randomized, all-axes input set the real Phase B rows use.
fn probe_inputs() -> Vec<Args> {
    let mut rng = Rng::for_row("mutation-control");
    let mut v = Vec::with_capacity(120_000);
    for _ in 0..100_000 {
        let l = L_CLASSES[(rng.next_u64() % 13) as usize];
        let u = U_CLASSES[(rng.next_u64() % 12) as usize];
        let vc = V_CLASSES[(rng.next_u64() % 9) as usize];
        v.push(gen_args(l, u, vc, &mut rng));
    }
    // Plus the dense canonical grid, which is where subtle bit-level slips show.
    for uni in 0..=15i32 {
        for lsbit in 0..=8i32 {
            for step in [0i32, 1, 7, 8, 64, 255, -64, i32::MAX] {
                for (pred, tgt, tgt2) in [
                    (0i32, 0i32, 0i32),
                    (1000, 1137, 900),
                    (-5000, 4999, -1),
                ] {
                    v.push(Args::new(uni, step, pred, tgt, tgt2, lsbit));
                }
            }
        }
    }
    v
}

#[test]
fn every_mutant_is_detected() {
    if Command::new("cc").arg("--version").output().is_err() {
        eprintln!("SKIP: no C compiler");
        return;
    }
    let base = c_source();
    let inputs = probe_inputs();
    eprintln!("{} probe inputs per mutant", inputs.len());

    let mut survivors: Vec<String> = Vec::new();
    let mut unbuildable: Vec<String> = Vec::new();
    let mut caught = 0usize;

    for (i, (label, from, to)) in MUTATIONS.iter().enumerate() {
        assert!(
            base.contains(from),
            "mutation #{i} ({label}): pattern {from:?} not present in the C source; \
             the mutation list is stale and must be updated"
        );
        // Replace only the first occurrence for statement-level mutations, all
        // occurrences for the expression-level ones (both are realistic slips).
        let mutated = base.replacen(from, to, 1);
        assert_ne!(&mutated, &base, "mutation #{i} ({label}) changed nothing");

        let Some(so) = build_mutant(i, &mutated) else {
            unbuildable.push((*label).to_string());
            continue;
        };

        let lib = unsafe { Library::new(&so) }.expect("dlopen mutant");
        let f: EncodeQuantFn = unsafe {
            let s: Symbol<EncodeQuantFn> = lib.get(b"encode_quant\0").expect("mutant symbol");
            *s
        };

        let mut diverged = 0usize;
        for &a in &inputs {
            let mv = unsafe { f(a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit) };
            if mv != call_rust(a) {
                diverged += 1;
            }
        }
        if diverged == 0 {
            survivors.push((*label).to_string());
            eprintln!("  SURVIVED  #{i} {label}");
        } else {
            caught += 1;
            eprintln!(
                "  caught    #{i} {label}  ({diverged}/{} inputs differ)",
                inputs.len()
            );
        }
    }

    eprintln!(
        "mutation score: {caught}/{} caught ({} unbuildable)",
        MUTATIONS.len() - unbuildable.len(),
        unbuildable.len()
    );
    assert!(
        unbuildable.is_empty(),
        "some mutants failed to compile: {unbuildable:?}"
    );
    assert!(
        survivors.is_empty(),
        "{} mutant(s) SURVIVED the differential inputs: {survivors:?}\n\
         The suite cannot detect these classes of translation error - strengthen it.",
        survivors.len()
    );
    assert_eq!(caught, MUTATIONS.len(), "not every mutant was evaluated");
}

/// Sanity: the UNMUTATED C source, rebuilt the same way, must agree with Rust.
/// Without this, `every_mutant_is_detected` could "catch" mutants merely because
/// the rebuild pipeline is broken.
#[test]
fn unmutated_rebuild_agrees() {
    if Command::new("cc").arg("--version").output().is_err() {
        eprintln!("SKIP: no C compiler");
        return;
    }
    let base = c_source();
    let so = build_mutant(9999, &base).expect("build unmutated copy");
    let lib = unsafe { Library::new(&so) }.expect("dlopen");
    let f: EncodeQuantFn = unsafe {
        let s: Symbol<EncodeQuantFn> = lib.get(b"encode_quant\0").unwrap();
        *s
    };
    let inputs = probe_inputs();
    let mut diverged = 0usize;
    for &a in &inputs {
        let mv = unsafe { f(a.uni, a.step, a.pred, a.tgt, a.tgt2, a.lsbit) };
        if mv != call_rust(a) {
            diverged += 1;
        }
    }
    assert_eq!(
        diverged, 0,
        "the unmutated C rebuild disagrees with Rust on {diverged} inputs - \
         the mutation pipeline itself is broken"
    );
    eprintln!("unmutated rebuild agrees on all {} inputs", inputs.len());
}
