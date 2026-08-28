//! Loader smoke test: proves both `.so`s open and both exports are callable.

mod common;

use common::*;

#[test]
fn both_libraries_load_and_export_encode_quant() {
    let l = libs();
    eprintln!("C    .so: {}", l.c_path.display());
    eprintln!("Rust .so: {}", l.rust_path.display());
    // A single trivial call through each export.
    let a = Args::new(5, 64, 100, 120, 130, 0);
    let c = call_c(a);
    let r = call_rust(a);
    assert_eq!(c, r, "smoke: C={c} Rust={r}");
    eprintln!("encode_quant(5,64,100,120,130,0) = {c}");
}

#[test]
fn quick_random_agreement() {
    let mut rng = Rng::for_row("smoke");
    for _ in 0..10_000 {
        let a = Args::new(
            rng.i32_any(),
            rng.i32_any(),
            rng.i32_any(),
            rng.i32_any(),
            rng.i32_any(),
            rng.i32_any(),
        );
        check("smoke", a);
    }
}
