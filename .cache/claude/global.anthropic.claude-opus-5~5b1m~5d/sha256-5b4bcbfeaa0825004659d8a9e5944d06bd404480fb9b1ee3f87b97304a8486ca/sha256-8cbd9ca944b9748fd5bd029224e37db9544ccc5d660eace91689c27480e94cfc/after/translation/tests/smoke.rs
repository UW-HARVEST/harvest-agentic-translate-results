//! Fast sanity check: both `.so`s load, export both symbols, and agree on a
//! handful of inputs. Everything substantive lives in `configs.rs` (Phase B)
//! and `errors.rs` (Phase C).

mod common;
use common::*;

#[test]
fn both_libraries_load_and_export_both_symbols() {
    let p = load();
    println!("C    = {}", p.c.path.display());
    println!("Rust = {}", p.rs.path.display());
}

#[test]
fn smoke_spectral() {
    let p = load();
    let a: Vec<f32> = (0..8).map(|i| 1.0 + i as f32).collect();
    let b: Vec<f32> = (0..8).map(|i| 8.0 - i as f32).collect();
    diff_spectral(&p, "smoke", &a, &b, 8, Alias::Disjoint);
}

#[test]
fn smoke_match() {
    let p = load();
    let t: Vec<f64> = (0..32).map(|i| ((i * 7) % 13) as f64 * 0.1).collect();
    let r: Vec<f64> = (0..32).map(|i| ((i * 5) % 11) as f64 * 0.1).collect();
    for th in [0.0, 0.5, 1.0] {
        diff_match(&p, "smoke", &t, &r, 32, th, false);
    }
}
