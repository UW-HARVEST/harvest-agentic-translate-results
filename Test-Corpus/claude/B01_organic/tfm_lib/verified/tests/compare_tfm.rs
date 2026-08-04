use libloading::{Library, Symbol};
use std::path::PathBuf;

type TfmFn = unsafe extern "C" fn(dest: *mut f32, src: *const f32, count: i32);

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_libs() -> (Library, Library) {
    let root = workspace_root();
    let c_path = root.join("c_src/build/libtranslated_rust.so");
    let r_path = root.join("target/debug/libtfm_lib.so");
    unsafe {
        let c = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("Failed to load C lib at {:?}: {}", c_path, e));
        let r = Library::new(&r_path)
            .unwrap_or_else(|e| panic!("Failed to load Rust lib at {:?}: {}", r_path, e));
        (c, r)
    }
}

fn run_case(src: &[f32]) {
    assert!(src.len() % 3 == 0);
    let count = (src.len() / 3) as i32;

    let (c_lib, r_lib) = load_libs();

    let mut c_dest = vec![0.0f32; (count as usize) * 2];
    let mut r_dest = vec![0.0f32; (count as usize) * 2];

    unsafe {
        let c_fn: Symbol<TfmFn> = c_lib.get(b"tfm").unwrap();
        let r_fn: Symbol<TfmFn> = r_lib.get(b"tfm").unwrap();
        c_fn(c_dest.as_mut_ptr(), src.as_ptr(), count);
        r_fn(r_dest.as_mut_ptr(), src.as_ptr(), count);
    }

    // Byte-identical comparison via raw bits.
    let c_bits: Vec<u32> = c_dest.iter().map(|f| f.to_bits()).collect();
    let r_bits: Vec<u32> = r_dest.iter().map(|f| f.to_bits()).collect();
    assert_eq!(
        c_bits, r_bits,
        "Mismatch.\n c={:?}\n r={:?}\n  src={:?}",
        c_dest, r_dest, src
    );
}

#[test]
fn test_zero_count() {
    let (c_lib, r_lib) = load_libs();
    let mut c_dest = [0.0f32; 4];
    let mut r_dest = [0.0f32; 4];
    let src = [0.0f32; 0];
    unsafe {
        let c_fn: Symbol<TfmFn> = c_lib.get(b"tfm").unwrap();
        let r_fn: Symbol<TfmFn> = r_lib.get(b"tfm").unwrap();
        c_fn(c_dest.as_mut_ptr(), src.as_ptr(), 0);
        r_fn(r_dest.as_mut_ptr(), src.as_ptr(), 0);
    }
    assert_eq!(c_dest, r_dest);
}

#[test]
fn test_simple_case_lt() {
    // src[0] < src[1] branch
    run_case(&[1.0, 2.0, 0.5]);
}

#[test]
fn test_simple_case_ge() {
    // src[0] >= src[1] branch
    run_case(&[2.0, 1.0, 0.5]);
}

#[test]
fn test_equal_branch() {
    // equality goes to else branch
    run_case(&[1.0, 1.0, 0.0]);
}

#[test]
fn test_zeros() {
    run_case(&[0.0, 0.0, 0.0]);
}

#[test]
fn test_negative_values() {
    run_case(&[-1.0, 2.0, -0.5]);
    run_case(&[-2.0, -3.0, 1.5]);
}

#[test]
fn test_large_values() {
    run_case(&[1.0e6, 2.0e6, 3.0e5]);
    run_case(&[1.0e10, 1.0e9, 1.0e8]);
}

#[test]
fn test_small_values() {
    run_case(&[1.0e-6, 2.0e-6, 3.0e-7]);
}

#[test]
fn test_multiple_entries() {
    let mut data = Vec::new();
    data.extend_from_slice(&[1.0, 2.0, 0.5]);
    data.extend_from_slice(&[2.0, 1.0, 0.5]);
    data.extend_from_slice(&[0.0, 0.0, 0.0]);
    data.extend_from_slice(&[1.5, 1.5, 0.25]);
    data.extend_from_slice(&[-2.0, 3.0, 1.0]);
    data.extend_from_slice(&[10.0, 20.0, 5.0]);
    run_case(&data);
}

#[test]
fn test_special_floats() {
    // Subnormals, large/small magnitudes.
    run_case(&[f32::MIN_POSITIVE, f32::MIN_POSITIVE * 2.0, 1.0e-30]);
    run_case(&[1.0e30, 1.0e30, 1.0e15]);
}

#[test]
fn test_random_batch() {
    use std::num::Wrapping;
    // Simple xorshift PRNG so we don't depend on rand crate.
    let mut state: Wrapping<u32> = Wrapping(0xdeadbeefu32);
    let mut next = || -> u32 {
        let mut x = state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        state = x;
        x.0
    };
    let mut rand_f = || -> f32 {
        // Generate values in roughly [-100, 100]
        let bits = next();
        let f = (bits as f32) / (u32::MAX as f32) * 200.0 - 100.0;
        f
    };
    let n = 1024;
    let mut data: Vec<f32> = Vec::with_capacity(n * 3);
    for _ in 0..n {
        data.push(rand_f());
        data.push(rand_f());
        data.push(rand_f());
    }
    run_case(&data);
}

#[test]
fn test_count_one() {
    run_case(&[3.0, 7.0, 2.0]);
}

#[test]
fn test_count_two() {
    run_case(&[3.0, 7.0, 2.0, 9.0, 4.0, -1.5]);
}

#[test]
fn test_negative_sqd_potentially() {
    // Edge case where dy2*dy2 - 2*dx2*dy2 + dx2*dx2 + 4*dxy*dxy could cause weirdness.
    // sqd should always be >= 0 mathematically but float may differ.
    run_case(&[1.0e7, 1.0e7 + 1.0, 1.0e-3]);
}
