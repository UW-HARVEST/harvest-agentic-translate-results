use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo puts cdylib in target/<profile>/
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libtfm_lib.so");
    p
}

type TfmFn = unsafe extern "C" fn(*mut f32, *const f32, i32);

fn call_tfm(lib: &Library, src: &[f32], count: i32) -> Vec<f32> {
    let mut dest = vec![0.0f32; count as usize * 2];
    unsafe {
        let f: Symbol<TfmFn> = lib.get(b"tfm").expect("symbol tfm not found");
        f(dest.as_mut_ptr(), src.as_ptr(), count);
    }
    dest
}

fn assert_byte_equal(c: &[f32], r: &[f32], label: &str) {
    assert_eq!(c.len(), r.len(), "{label}: length mismatch");
    for (i, (a, b)) in c.iter().zip(r.iter()).enumerate() {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "{label}[{i}]: C={a} (0x{:08x}) != Rust={b} (0x{:08x})",
            a.to_bits(),
            b.to_bits()
        );
    }
}

#[test]
fn test_tfm_branch_true() {
    // src[0] < src[1] → first branch
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let src = vec![1.0f32, 3.0, 0.5];
    let c_out = call_tfm(&c_lib, &src, 1);
    let r_out = call_tfm(&r_lib, &src, 1);
    assert_byte_equal(&c_out, &r_out, "branch_true");
}

#[test]
fn test_tfm_branch_false() {
    // src[0] >= src[1] → else branch
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let src = vec![5.0f32, 2.0, 1.0];
    let c_out = call_tfm(&c_lib, &src, 1);
    let r_out = call_tfm(&r_lib, &src, 1);
    assert_byte_equal(&c_out, &r_out, "branch_false");
}

#[test]
fn test_tfm_equal_values() {
    // src[0] == src[1] → else branch
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let src = vec![2.0f32, 2.0, 0.0];
    let c_out = call_tfm(&c_lib, &src, 1);
    let r_out = call_tfm(&r_lib, &src, 1);
    assert_byte_equal(&c_out, &r_out, "equal");
}

#[test]
fn test_tfm_zeros() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let src = vec![0.0f32, 0.0, 0.0];
    let c_out = call_tfm(&c_lib, &src, 1);
    let r_out = call_tfm(&r_lib, &src, 1);
    assert_byte_equal(&c_out, &r_out, "zeros");
}

#[test]
fn test_tfm_negative_sqd() {
    // Values that make sqd negative → clamp to 0
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();
    // sqd = (dy2-dx2)^2 + 4*dxy^2, always >= 0 for real inputs,
    // but test with values that exercise the max(0,sqd) path
    let src = vec![1.0f32, 2.0, 0.0];
    let c_out = call_tfm(&c_lib, &src, 1);
    let r_out = call_tfm(&r_lib, &src, 1);
    assert_byte_equal(&c_out, &r_out, "neg_sqd");
}

#[test]
fn test_tfm_multiple_elements() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();
    // 3 elements: mix of branches
    let src = vec![
        1.0f32, 3.0, 0.5,   // branch true
        5.0, 2.0, 1.0,      // branch false
        0.0, 0.0, 0.0,      // equal → else
    ];
    let c_out = call_tfm(&c_lib, &src, 3);
    let r_out = call_tfm(&r_lib, &src, 3);
    assert_byte_equal(&c_out, &r_out, "multi");
}

#[test]
fn test_tfm_count_zero() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let src = vec![1.0f32, 2.0, 3.0];
    let c_out = call_tfm(&c_lib, &src, 0);
    let r_out = call_tfm(&r_lib, &src, 0);
    assert_byte_equal(&c_out, &r_out, "count_zero");
}

#[test]
fn test_tfm_large_values() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let src = vec![1e10f32, 1e12, 1e6];
    let c_out = call_tfm(&c_lib, &src, 1);
    let r_out = call_tfm(&r_lib, &src, 1);
    assert_byte_equal(&c_out, &r_out, "large");
}

#[test]
fn test_tfm_negative_inputs() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();
    let src = vec![-3.0f32, -1.0, -0.5];
    let c_out = call_tfm(&c_lib, &src, 1);
    let r_out = call_tfm(&r_lib, &src, 1);
    assert_byte_equal(&c_out, &r_out, "negative");
}

#[test]
fn test_tfm_nan_inf() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.unwrap();
    let r_lib = unsafe { Library::new(rust_lib_path()) }.unwrap();
    for (label, src) in [
        ("nan", vec![f32::NAN, 1.0, 0.0]),
        ("inf", vec![f32::INFINITY, 1.0, 0.5]),
        ("neg_inf", vec![f32::NEG_INFINITY, 1.0, 0.5]),
    ] {
        let c_out = call_tfm(&c_lib, &src, 1);
        let r_out = call_tfm(&r_lib, &src, 1);
        assert_byte_equal(&c_out, &r_out, label);
    }
}
