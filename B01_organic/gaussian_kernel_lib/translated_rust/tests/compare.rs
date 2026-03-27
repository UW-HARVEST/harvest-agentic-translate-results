use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn call_c(size: i32, radius: f32) -> Vec<f32> {
    let lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let func: Symbol<unsafe extern "C" fn(*mut f32, i32, f32)> =
        unsafe { lib.get(b"gaussian_kernel").expect("find symbol") };
    let mut buf = vec![0.0f32; size as usize];
    unsafe { func(buf.as_mut_ptr(), size, radius) };
    buf
}

fn call_rust(size: i32, radius: f32) -> Vec<f32> {
    let mut buf = vec![0.0f32; size as usize];
    unsafe { gaussian_kernel_lib::gaussian_kernel(buf.as_mut_ptr(), size, radius) };
    buf
}

fn as_bytes(v: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 4) }
}

#[test]
fn test_gaussian_kernel_matches() {
    let cases: &[(i32, f32)] = &[
        (1, 1.0),
        (3, 1.0),
        (5, 2.0),
        (7, 3.0),
        (11, 4.5),
        (21, 10.0),
        (1, 0.5),
        (9, 1.6),
    ];
    for &(size, radius) in cases {
        let c_out = call_c(size, radius);
        let r_out = call_rust(size, radius);
        assert_eq!(
            as_bytes(&c_out),
            as_bytes(&r_out),
            "Mismatch for size={size}, radius={radius}:\n  C:    {c_out:?}\n  Rust: {r_out:?}"
        );
    }
}
