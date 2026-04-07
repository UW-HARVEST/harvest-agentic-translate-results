use libloading::{Library, Symbol};
use std::path::PathBuf;

type GaussianKernelFn = unsafe extern "C" fn(*mut f32, i32, f32);

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_c_lib() -> Library {
    let path = project_root().join("c_src/build/libtranslated_rust.so");
    unsafe { Library::new(&path).expect("Failed to load C .so") }
}

fn load_rust_lib() -> Library {
    let path = project_root().join("target/debug/libgaussian_kernel_lib.so");
    unsafe { Library::new(&path).expect("Failed to load Rust .so") }
}

fn call_gaussian_kernel(lib: &Library, size: i32, radius: f32) -> Vec<f32> {
    unsafe {
        let func: Symbol<GaussianKernelFn> = lib.get(b"gaussian_kernel").expect("symbol not found");
        let mut dest = vec![0.0f32; size as usize];
        func(dest.as_mut_ptr(), size, radius);
        dest
    }
}

fn assert_identical(c_out: &[f32], rust_out: &[f32], label: &str) {
    assert_eq!(c_out.len(), rust_out.len(), "{label}: length mismatch");
    for (i, (c, r)) in c_out.iter().zip(rust_out.iter()).enumerate() {
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "{label}: mismatch at index {i}: C={c} (0x{:08x}) vs Rust={r} (0x{:08x})",
            c.to_bits(),
            r.to_bits()
        );
    }
}

#[test]
fn test_gaussian_kernel_various_inputs() {
    let c_lib = load_c_lib();
    let rust_lib = load_rust_lib();

    let test_cases: Vec<(i32, f32)> = vec![
        (1, 1.0),
        (3, 1.0),
        (5, 1.0),
        (5, 2.0),
        (5, 0.5),
        (7, 1.0),
        (7, 3.0),
        (9, 1.5),
        (11, 2.0),
        (15, 1.0),
        (21, 4.0),
        (1, 0.1),
        (3, 10.0),
        (31, 1.0),
        (31, 0.01),
    ];

    for (size, radius) in &test_cases {
        let c_out = call_gaussian_kernel(&c_lib, *size, *radius);
        let rust_out = call_gaussian_kernel(&rust_lib, *size, *radius);
        assert_identical(&c_out, &rust_out, &format!("size={size}, radius={radius}"));
    }
}

#[test]
fn test_gaussian_kernel_edge_cases() {
    let c_lib = load_c_lib();
    let rust_lib = load_rust_lib();

    // Large radius
    let c_out = call_gaussian_kernel(&c_lib, 5, 1000.0);
    let rust_out = call_gaussian_kernel(&rust_lib, 5, 1000.0);
    assert_identical(&c_out, &rust_out, "size=5, radius=1000");

    // Small radius
    let c_out = call_gaussian_kernel(&c_lib, 3, 0.001);
    let rust_out = call_gaussian_kernel(&rust_lib, 3, 0.001);
    assert_identical(&c_out, &rust_out, "size=3, radius=0.001");

    // Even size (C uses integer division for hsize)
    let c_out = call_gaussian_kernel(&c_lib, 4, 1.0);
    let rust_out = call_gaussian_kernel(&rust_lib, 4, 1.0);
    assert_identical(&c_out, &rust_out, "size=4, radius=1.0");

    // size=1
    let c_out = call_gaussian_kernel(&c_lib, 1, 0.5);
    let rust_out = call_gaussian_kernel(&rust_lib, 1, 0.5);
    assert_identical(&c_out, &rust_out, "size=1, radius=0.5");
}
