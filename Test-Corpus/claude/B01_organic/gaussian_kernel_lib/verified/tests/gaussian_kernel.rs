use libloading::{Library, Symbol};
use std::os::raw::c_int;

type GaussianKernelFn = unsafe extern "C" fn(*mut f32, c_int, f32);

const C_LIB_PATH: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB_PATH: &str = "target/release/libgaussian_kernel_lib.so";

unsafe fn load_fn(lib: &Library) -> Symbol<GaussianKernelFn> {
    unsafe { lib.get(b"gaussian_kernel\0").expect("symbol not found") }
}

fn run_case(size: i32, radius: f32) {
    let c_lib = unsafe { Library::new(C_LIB_PATH) }.expect("failed to load C lib");
    let rust_lib = unsafe { Library::new(RUST_LIB_PATH) }.expect("failed to load Rust lib");
    let c_fn = unsafe { load_fn(&c_lib) };
    let rust_fn = unsafe { load_fn(&rust_lib) };

    let mut c_buf: Vec<f32> = vec![0.0f32; size.max(0) as usize];
    let mut rust_buf: Vec<f32> = vec![0.0f32; size.max(0) as usize];

    unsafe {
        c_fn(c_buf.as_mut_ptr(), size, radius);
        rust_fn(rust_buf.as_mut_ptr(), size, radius);
    }

    // Compare byte for byte
    let c_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(c_buf.as_ptr() as *const u8, c_buf.len() * 4)
    };
    let r_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(rust_buf.as_ptr() as *const u8, rust_buf.len() * 4)
    };
    assert_eq!(
        c_bytes, r_bytes,
        "byte mismatch for size={size} radius={radius}\n c={:?}\n r={:?}",
        c_buf, rust_buf
    );
}

#[test]
fn test_gaussian_kernel_basic_sizes() {
    let radii = [0.5f32, 1.0, 1.5, 2.0, 3.5, 5.0, 10.0];
    for &size in &[1, 3, 5, 7, 9, 11, 15, 21, 31, 51, 101] {
        for &r in &radii {
            run_case(size, r);
        }
    }
}

#[test]
fn test_gaussian_kernel_even_sizes() {
    // Even sizes - the loop runs from -hsize to +hsize inclusive (size+1 elements when even).
    // Replicate exact C behavior; we still allocate generously.
    for &size in &[2, 4, 6, 8, 10, 16, 32] {
        for &r in &[0.5f32, 1.0, 2.0, 4.0] {
            // For even size, loop writes hsize*2+1 = size+1 elements; we need an extra.
            let alloc_n = (size + 2) as usize;
            let c_lib = unsafe { Library::new(C_LIB_PATH) }.unwrap();
            let rust_lib = unsafe { Library::new(RUST_LIB_PATH) }.unwrap();
            let c_fn = unsafe { load_fn(&c_lib) };
            let rust_fn = unsafe { load_fn(&rust_lib) };

            let mut c_buf = vec![0f32; alloc_n];
            let mut rust_buf = vec![0f32; alloc_n];

            unsafe {
                c_fn(c_buf.as_mut_ptr(), size, r);
                rust_fn(rust_buf.as_mut_ptr(), size, r);
            }

            let c_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(c_buf.as_ptr() as *const u8, c_buf.len() * 4)
            };
            let r_bytes: &[u8] = unsafe {
                std::slice::from_raw_parts(rust_buf.as_ptr() as *const u8, rust_buf.len() * 4)
            };
            assert_eq!(c_bytes, r_bytes, "even size={size} radius={r}");
        }
    }
}

#[test]
fn test_gaussian_kernel_extreme_radii() {
    let extreme = [0.01f32, 0.1, 100.0, 1000.0, 1e6, 1e-3];
    for &size in &[1, 3, 7, 15, 31] {
        for &r in &extreme {
            run_case(size, r);
        }
    }
}

#[test]
fn test_symbols_present() {
    let c_lib = unsafe { Library::new(C_LIB_PATH) }.unwrap();
    let rust_lib = unsafe { Library::new(RUST_LIB_PATH) }.unwrap();
    let _c: Symbol<GaussianKernelFn> = unsafe { c_lib.get(b"gaussian_kernel\0").unwrap() };
    let _r: Symbol<GaussianKernelFn> = unsafe { rust_lib.get(b"gaussian_kernel\0").unwrap() };
}
