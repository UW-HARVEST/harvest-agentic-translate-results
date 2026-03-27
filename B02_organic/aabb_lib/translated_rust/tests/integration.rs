use libloading::{Library, Symbol};
use std::os::raw::c_int;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libaabb_lib.so"
    );
    unsafe { Library::new(path).expect("Failed to load C library") }
}

fn rust_lib() -> Library {
    // Find the Rust cdylib in target/debug/
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug");
    for entry in std::fs::read_dir(&dir).expect("no target/debug") {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_str().unwrap();
        if name.starts_with("libaabb_lib") && name.ends_with(".so") {
            return unsafe { Library::new(entry.path()).expect("Failed to load Rust library") };
        }
    }
    panic!("Rust .so not found in {:?}", dir);
}

/// Test the public `aabb` function with various inputs
#[test]
fn test_aabb_public_api() {
    let c = c_lib();
    let r = rust_lib();

    type AabbFn = unsafe extern "C" fn(f32, f32, f32, f32) -> c_int;
    let c_aabb: Symbol<AabbFn> = unsafe { c.get(b"aabb").unwrap() };
    let r_aabb: Symbol<AabbFn> = unsafe { r.get(b"aabb").unwrap() };

    let test_cases: &[(f32, f32, f32, f32)] = &[
        // Overlapping with all three shapes
        (-100.0, -100.0, 100.0, 100.0),
        // No overlap
        (200.0, 200.0, 300.0, 300.0),
        // Overlap with circle only (near -70, 0)
        (-95.0, -25.0, -45.0, 25.0),
        // Overlap with AABB only (near -40,-40 to -15,-15)
        (-42.0, -42.0, -13.0, -13.0),
        // Overlap with capsule only (near -40,40 to -20,100)
        (-55.0, 35.0, -25.0, 55.0),
        // Edge cases
        (0.0, 0.0, 0.0, 0.0),
        (-1.0, -1.0, 1.0, 1.0),
        // Large values
        (-1e6, -1e6, 1e6, 1e6),
        // Negative area (min > max)
        (10.0, 10.0, -10.0, -10.0),
        // Touching edges of the fixed AABB
        (-15.0, -15.0, -14.0, -14.0),
        (-41.0, -41.0, -40.0, -40.0),
        // Near circle boundary
        (-90.0, -1.0, -89.0, 1.0),
        (-50.0, -1.0, -49.0, 1.0),
        // Near capsule
        (-35.0, 80.0, -25.0, 90.0),
    ];

    for &(min_x, min_y, max_x, max_y) in test_cases {
        let c_result = unsafe { c_aabb(min_x, min_y, max_x, max_y) };
        let r_result = unsafe { r_aabb(min_x, min_y, max_x, max_y) };
        assert_eq!(
            c_result, r_result,
            "aabb({}, {}, {}, {}): C={} Rust={}",
            min_x, min_y, max_x, max_y, c_result, r_result
        );
    }
}
