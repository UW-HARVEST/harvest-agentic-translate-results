use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct LmVec2 {
    x: f32,
    y: f32,
}

type ToBarycentricFn = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;

fn c_lib_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // CARGO_TARGET_DIR may not be set; fallback to target/debug
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut p = PathBuf::from(manifest_dir).join("target/debug/libto_barycentric_lib.so");
    if !p.exists() {
        // try release
        p = PathBuf::from(manifest_dir).join("target/release/libto_barycentric_lib.so");
    }
    p
}

unsafe fn load_symbol<'lib>(
    lib: &'lib Library,
    name: &str,
) -> Symbol<'lib, ToBarycentricFn> {
    lib.get(name.as_bytes()).expect("symbol not found")
}

fn run_case(p1: LmVec2, p2: LmVec2, p3: LmVec2, p: LmVec2) {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<ToBarycentricFn> = load_symbol(&c_lib, "to_barycentric");
        let r_fn: Symbol<ToBarycentricFn> = load_symbol(&r_lib, "to_barycentric");

        let c_out = c_fn(p1, p2, p3, p);
        let r_out = r_fn(p1, p2, p3, p);

        // byte-identical comparison via to_bits
        assert_eq!(
            c_out.x.to_bits(),
            r_out.x.to_bits(),
            "x mismatch for inputs p1=({},{}) p2=({},{}) p3=({},{}) p=({},{}): C={} Rust={}",
            p1.x, p1.y, p2.x, p2.y, p3.x, p3.y, p.x, p.y,
            c_out.x, r_out.x
        );
        assert_eq!(
            c_out.y.to_bits(),
            r_out.y.to_bits(),
            "y mismatch for inputs p1=({},{}) p2=({},{}) p3=({},{}) p=({},{}): C={} Rust={}",
            p1.x, p1.y, p2.x, p2.y, p3.x, p3.y, p.x, p.y,
            c_out.y, r_out.y
        );
    }
}

fn v(x: f32, y: f32) -> LmVec2 {
    LmVec2 { x, y }
}

#[test]
fn test_unit_triangle_corners() {
    // p1 = (0,0), p2 = (1,0), p3 = (0,1), evaluate at corners and centroid
    let p1 = v(0.0, 0.0);
    let p2 = v(1.0, 0.0);
    let p3 = v(0.0, 1.0);
    run_case(p1, p2, p3, v(0.0, 0.0));
    run_case(p1, p2, p3, v(1.0, 0.0));
    run_case(p1, p2, p3, v(0.0, 1.0));
    run_case(p1, p2, p3, v(0.3333333, 0.3333333));
    run_case(p1, p2, p3, v(0.5, 0.5));
}

#[test]
fn test_arbitrary_triangle() {
    let p1 = v(1.5, -2.5);
    let p2 = v(4.0, 1.0);
    let p3 = v(-2.0, 3.0);
    run_case(p1, p2, p3, v(0.0, 0.0));
    run_case(p1, p2, p3, v(2.0, 0.5));
    run_case(p1, p2, p3, v(-10.0, 10.0));
    run_case(p1, p2, p3, v(100.0, -50.0));
}

#[test]
fn test_negative_and_large_values() {
    let p1 = v(-1000.0, -1000.0);
    let p2 = v(1000.0, -1000.0);
    let p3 = v(0.0, 1000.0);
    run_case(p1, p2, p3, v(0.0, 0.0));
    run_case(p1, p2, p3, v(-500.0, 500.0));
    run_case(p1, p2, p3, v(1234.5, -678.9));
}

#[test]
fn test_small_values() {
    let p1 = v(0.0001, 0.0002);
    let p2 = v(0.0005, 0.0001);
    let p3 = v(0.0003, 0.0007);
    run_case(p1, p2, p3, v(0.0004, 0.0003));
    run_case(p1, p2, p3, v(0.0002, 0.0006));
}

#[test]
fn test_pseudo_random_cases() {
    // simple LCG to generate deterministic pseudo-random floats
    let mut state: u32 = 0xDEADBEEF;
    let mut next = || -> f32 {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        let v = (state >> 16) as i32 as f32; // signed-ish range
        v / 1000.0
    };
    for _ in 0..200 {
        let p1 = v(next(), next());
        let p2 = v(next(), next());
        let p3 = v(next(), next());
        let p = v(next(), next());
        run_case(p1, p2, p3, p);
    }
}

#[test]
fn test_collinear_degenerate() {
    // Degenerate: collinear points produce inf/nan; both implementations
    // should yield the same bit pattern.
    let p1 = v(0.0, 0.0);
    let p2 = v(1.0, 1.0);
    let p3 = v(2.0, 2.0);
    run_case(p1, p2, p3, v(0.5, 0.5));
}
