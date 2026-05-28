use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

type ColourblindFn = unsafe extern "C" fn(c_int, *mut f32, *mut f32, *mut f32);

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libtranslated_rust.so");
    p
}

fn rust_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Try debug; fall back to release.
    let dbg = p.join("target/debug/libcolourblind_lib.so");
    if dbg.exists() {
        return dbg;
    }
    p.push("target/release/libcolourblind_lib.so");
    p
}

fn run_for_lib(path: &PathBuf, imp: c_int, r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    unsafe {
        let lib = Library::new(path).expect("failed to load library");
        let f: Symbol<ColourblindFn> = lib.get(b"colourblind").expect("colourblind symbol");
        let mut rr = r;
        let mut gg = g;
        let mut bb = b;
        f(imp, &mut rr, &mut gg, &mut bb);
        (rr, gg, bb)
    }
}

fn assert_match(imp: c_int, r: f32, g: f32, b: f32) {
    let (cr, cg, cb) = run_for_lib(&c_so_path(), imp, r, g, b);
    let (rr, rg, rb) = run_for_lib(&rust_so_path(), imp, r, g, b);
    assert_eq!(cr.to_bits(), rr.to_bits(), "imp={} input=({},{},{}) red mismatch C={} R={}", imp, r, g, b, cr, rr);
    assert_eq!(cg.to_bits(), rg.to_bits(), "imp={} input=({},{},{}) green mismatch C={} R={}", imp, r, g, b, cg, rg);
    assert_eq!(cb.to_bits(), rb.to_bits(), "imp={} input=({},{},{}) blue mismatch C={} R={}", imp, r, g, b, cb, rb);
}

#[test]
fn test_colourblind_protanopia_basic() {
    assert_match(0, 0.0, 0.0, 0.0);
    assert_match(0, 1.0, 0.0, 0.0);
    assert_match(0, 0.0, 1.0, 0.0);
    assert_match(0, 0.0, 0.0, 1.0);
    assert_match(0, 1.0, 1.0, 1.0);
    assert_match(0, 0.5, 0.5, 0.5);
    assert_match(0, 0.25, 0.75, 0.5);
    assert_match(0, -1.0, 1.0, 0.5);
}

#[test]
fn test_colourblind_deuteranopia_basic() {
    assert_match(1, 0.0, 0.0, 0.0);
    assert_match(1, 1.0, 0.0, 0.0);
    assert_match(1, 0.0, 1.0, 0.0);
    assert_match(1, 0.0, 0.0, 1.0);
    assert_match(1, 1.0, 1.0, 1.0);
    assert_match(1, 0.5, 0.5, 0.5);
    assert_match(1, 0.25, 0.75, 0.5);
    assert_match(1, -1.0, 1.0, 0.5);
}

#[test]
fn test_colourblind_tritanopia_basic() {
    assert_match(2, 0.0, 0.0, 0.0);
    assert_match(2, 1.0, 0.0, 0.0);
    assert_match(2, 0.0, 1.0, 0.0);
    assert_match(2, 0.0, 0.0, 1.0);
    assert_match(2, 1.0, 1.0, 1.0);
    assert_match(2, 0.5, 0.5, 0.5);
    assert_match(2, 0.25, 0.75, 0.5);
    assert_match(2, -1.0, 1.0, 0.5);
}

#[test]
fn test_colourblind_unknown_impairment_no_change() {
    // Unknown impairment values should leave inputs unchanged in both impls
    // (C: switch falls through silently; Rust: matches `_ => {}`)
    for imp in [3, 99, -1, 1000] {
        let (cr, cg, cb) = run_for_lib(&c_so_path(), imp, 0.1, 0.2, 0.3);
        let (rr, rg, rb) = run_for_lib(&rust_so_path(), imp, 0.1, 0.2, 0.3);
        assert_eq!(cr.to_bits(), rr.to_bits(), "imp={} red", imp);
        assert_eq!(cg.to_bits(), rg.to_bits(), "imp={} green", imp);
        assert_eq!(cb.to_bits(), rb.to_bits(), "imp={} blue", imp);
    }
}

#[test]
fn test_colourblind_random_inputs() {
    // Use a deterministic LCG to generate a wide range of float inputs.
    let mut state: u64 = 0xDEADBEEFCAFEBABE;
    let mut next = || {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let bits = (state >> 32) as u32;
        f32::from_bits(bits)
    };
    for _ in 0..200 {
        let r = next();
        let g = next();
        let b = next();
        // Skip NaNs to keep bit-exact comparison meaningful for non-NaNs;
        // but actually NaN bit patterns from same operations should also match.
        for imp in 0..3 {
            // Compare as bit patterns; matching is byte-exact even for NaNs
            let (cr, cg, cb) = run_for_lib(&c_so_path(), imp, r, g, b);
            let (rr, rg, rb) = run_for_lib(&rust_so_path(), imp, r, g, b);
            assert_eq!(cr.to_bits(), rr.to_bits(), "imp={} input=({:e},{:e},{:e})", imp, r, g, b);
            assert_eq!(cg.to_bits(), rg.to_bits(), "imp={} input=({:e},{:e},{:e})", imp, r, g, b);
            assert_eq!(cb.to_bits(), rb.to_bits(), "imp={} input=({:e},{:e},{:e})", imp, r, g, b);
        }
    }
}
