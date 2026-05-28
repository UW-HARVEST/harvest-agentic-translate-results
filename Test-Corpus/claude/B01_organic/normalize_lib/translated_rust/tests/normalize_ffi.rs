use libloading::{Library, Symbol};
use std::os::raw::c_int;

type NormalizeFn = unsafe extern "C" fn(*mut f32, *const f32, c_int);

const C_LIB_PATH: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB_PATH: &str = "target/release/libnormalize_lib.so";

unsafe fn load_normalize(lib: &Library) -> Symbol<NormalizeFn> {
    unsafe { lib.get(b"normalize\0").expect("normalize symbol not found") }
}

fn run_both(input: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let size = input.len() as c_int;
    let mut c_dest = vec![0f32; input.len()];
    let mut r_dest = vec![0f32; input.len()];

    unsafe {
        let c_lib = Library::new(C_LIB_PATH).expect("failed to load C lib");
        let r_lib = Library::new(RUST_LIB_PATH).expect("failed to load Rust lib");

        let c_fn = load_normalize(&c_lib);
        let r_fn = load_normalize(&r_lib);

        c_fn(c_dest.as_mut_ptr(), input.as_ptr(), size);
        r_fn(r_dest.as_mut_ptr(), input.as_ptr(), size);
    }

    (c_dest, r_dest)
}

fn assert_bytes_equal(a: &[f32], b: &[f32]) {
    assert_eq!(a.len(), b.len(), "length mismatch");
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        assert_eq!(
            x.to_bits(),
            y.to_bits(),
            "byte mismatch at index {}: c={:?} (0x{:08x}), rust={:?} (0x{:08x})",
            i,
            x,
            x.to_bits(),
            y,
            y.to_bits()
        );
    }
}

#[test]
fn normalize_basic_unit_vector() {
    let input = vec![3.0f32, 4.0f32];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_already_unit() {
    let input = vec![1.0f32, 0.0f32, 0.0f32];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_all_zeros() {
    // sum == 0, dest != src branch -> memset
    let input = vec![0.0f32; 8];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_empty() {
    let input: Vec<f32> = vec![];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_single_element_positive() {
    let input = vec![5.0f32];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_single_element_negative() {
    let input = vec![-7.5f32];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_single_element_zero() {
    let input = vec![0.0f32];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_mixed_signs() {
    let input = vec![-1.0f32, 2.0f32, -3.0f32, 4.0f32, -5.0f32];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_large_array() {
    let input: Vec<f32> = (0..1000).map(|i| (i as f32) * 0.001).collect();
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_small_values() {
    let input = vec![1e-20f32, 2e-20f32, 3e-20f32];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_large_values() {
    let input = vec![1e10f32, 2e10f32, 3e10f32];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_underflow_zero_sum() {
    // Values small enough that squared sum underflows to 0
    let input = vec![1e-30f32, 1e-30f32];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_negative_zero_inputs() {
    let input = vec![-0.0f32, -0.0f32, -0.0f32];
    let (c, r) = run_both(&input);
    assert_bytes_equal(&c, &r);
}

#[test]
fn normalize_aliased_dest_equals_src_zeros() {
    // dest == src case with sum == 0: must NOT memset (since dest == src)
    // Simulate by using same pointer for dest and src.
    let mut input = vec![0.0f32; 5];
    let size = input.len() as c_int;

    let c_lib = unsafe { Library::new(C_LIB_PATH) }.unwrap();
    let r_lib = unsafe { Library::new(RUST_LIB_PATH) }.unwrap();

    let c_fn: Symbol<NormalizeFn> = unsafe { c_lib.get(b"normalize\0").unwrap() };
    let r_fn: Symbol<NormalizeFn> = unsafe { r_lib.get(b"normalize\0").unwrap() };

    let mut c_buf = input.clone();
    let mut r_buf = input.clone();

    unsafe {
        c_fn(c_buf.as_mut_ptr(), c_buf.as_ptr(), size);
        r_fn(r_buf.as_mut_ptr(), r_buf.as_ptr(), size);
    }

    assert_bytes_equal(&c_buf, &r_buf);
    // sanity: input untouched
    let _ = input.as_mut_ptr();
}

#[test]
fn normalize_aliased_dest_equals_src_nonzero() {
    let input = vec![1.0f32, 2.0f32, 3.0f32, 4.0f32];
    let size = input.len() as c_int;

    let c_lib = unsafe { Library::new(C_LIB_PATH) }.unwrap();
    let r_lib = unsafe { Library::new(RUST_LIB_PATH) }.unwrap();

    let c_fn: Symbol<NormalizeFn> = unsafe { c_lib.get(b"normalize\0").unwrap() };
    let r_fn: Symbol<NormalizeFn> = unsafe { r_lib.get(b"normalize\0").unwrap() };

    let mut c_buf = input.clone();
    let mut r_buf = input.clone();

    unsafe {
        c_fn(c_buf.as_mut_ptr(), c_buf.as_ptr(), size);
        r_fn(r_buf.as_mut_ptr(), r_buf.as_ptr(), size);
    }

    assert_bytes_equal(&c_buf, &r_buf);
}

#[test]
fn normalize_zeros_dest_initially_nonzero() {
    // Make sure that when sum==0 and dest!=src, dest is zeroed.
    let input = vec![0.0f32; 10];
    let size = input.len() as c_int;

    let c_lib = unsafe { Library::new(C_LIB_PATH) }.unwrap();
    let r_lib = unsafe { Library::new(RUST_LIB_PATH) }.unwrap();

    let c_fn: Symbol<NormalizeFn> = unsafe { c_lib.get(b"normalize\0").unwrap() };
    let r_fn: Symbol<NormalizeFn> = unsafe { r_lib.get(b"normalize\0").unwrap() };

    let mut c_buf = vec![42.0f32; input.len()];
    let mut r_buf = vec![42.0f32; input.len()];

    unsafe {
        c_fn(c_buf.as_mut_ptr(), input.as_ptr(), size);
        r_fn(r_buf.as_mut_ptr(), input.as_ptr(), size);
    }

    assert_bytes_equal(&c_buf, &r_buf);
    for v in &c_buf {
        assert_eq!(v.to_bits(), 0u32);
    }
}
