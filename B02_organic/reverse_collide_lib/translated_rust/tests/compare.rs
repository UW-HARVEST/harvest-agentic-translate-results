use libloading::{Library, Symbol};
use std::os::raw::c_int;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libreverse_collide_lib.so"
    );
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

fn call_c_reverse_collide(lib: &Library, x: f32, y: f32, r: f32) -> c_int {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(f32, f32, f32) -> c_int> =
            lib.get(b"reverse_collide").unwrap();
        f(x, y, r)
    }
}

extern "C" {
    fn reverse_collide(x: f32, y: f32, r: f32) -> c_int;
}

fn rust_reverse_collide(x: f32, y: f32, r: f32) -> c_int {
    unsafe { reverse_collide(x, y, r) }
}

/// Test cases covering all collision branches:
/// - Circle at (-70,0) r=20
/// - AABB min=(-40,-40) max=(-15,-15)
/// - Capsule a=(-40,40) b=(-20,100) r=10
/// Result bits: 0=circle, 1=aabb, 2=capsule
#[test]
fn test_reverse_collide_comprehensive() {
    let lib = c_lib();
    let cases: &[(f32, f32, f32)] = &[
        // No collision
        (100.0, 100.0, 1.0),
        (0.0, 0.0, 1.0),
        // Circle-circle collision only (near -70,0)
        (-60.0, 0.0, 5.0),
        (-50.0, 0.0, 5.0),
        // AABB collision only (near -27.5, -27.5)
        (-27.5, -27.5, 5.0),
        (-30.0, -30.0, 1.0),
        // Capsule collision only (near -30, 70)
        (-30.0, 70.0, 5.0),
        (-35.0, 50.0, 5.0),
        // Multiple collisions
        (-40.0, 0.0, 30.0),
        (-50.0, 0.0, 50.0),
        // Edge cases
        (0.0, 0.0, 0.0),
        (-70.0, 0.0, 20.0),
        (-70.0, 0.0, 0.0),
        // Large radius hitting everything
        (0.0, 0.0, 200.0),
        // Negative coords
        (-100.0, -100.0, 10.0),
        (-40.0, 40.0, 1.0),
        (-20.0, 100.0, 1.0),
    ];

    for &(x, y, r) in cases {
        let c_result = call_c_reverse_collide(&lib, x, y, r);
        let rust_result = rust_reverse_collide(x, y, r);
        assert_eq!(
            c_result, rust_result,
            "Mismatch for ({}, {}, {}): C={}, Rust={}",
            x, y, r, c_result, rust_result
        );
    }
}
