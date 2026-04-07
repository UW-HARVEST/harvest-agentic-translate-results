use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

type NormalizeFn = unsafe extern "C" fn(*mut f32, *const f32, c_int);

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libnormalize_lib.so")
}

fn load_normalize(lib: &Library) -> Symbol<NormalizeFn> {
    unsafe { lib.get(b"normalize").expect("symbol not found") }
}

fn call_normalize(f: &NormalizeFn, src: &[f32], inplace: bool) -> Vec<f32> {
    let size = src.len() as c_int;
    if inplace {
        let mut buf = src.to_vec();
        unsafe { f(buf.as_mut_ptr(), buf.as_ptr(), size) };
        buf
    } else {
        let mut dest = vec![f32::NAN; src.len()];
        unsafe { f(dest.as_mut_ptr(), src.as_ptr(), size) };
        dest
    }
}

fn compare(label: &str, src: &[f32], inplace: bool) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };
    let c_fn = load_normalize(&c_lib);
    let r_fn = load_normalize(&r_lib);

    let c_out = call_normalize(&c_fn, src, inplace);
    let r_out = call_normalize(&r_fn, src, inplace);

    let c_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(c_out.as_ptr() as *const u8, c_out.len() * 4)
    };
    let r_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(r_out.as_ptr() as *const u8, r_out.len() * 4)
    };
    assert_eq!(c_bytes, r_bytes, "{label}: C={c_out:?} Rust={r_out:?}");
}

#[test] fn normal_vec()      { compare("normal",   &[3.0, 4.0], false); }
#[test] fn normal_inplace()  { compare("inplace",  &[3.0, 4.0], true); }
#[test] fn zero_vec()        { compare("zero",     &[0.0, 0.0, 0.0], false); }
#[test] fn zero_inplace()    { compare("zero_ip",  &[0.0, 0.0], true); }
#[test] fn single()          { compare("single",   &[5.0], false); }
#[test] fn negatives()       { compare("neg",      &[-1.0, -2.0, -3.0], false); }
#[test] fn mixed()           { compare("mixed",    &[-1.0, 0.0, 1.0], false); }
#[test] fn large()           { compare("large",    &[1e20, 1e20], false); }
#[test] fn tiny()            { compare("tiny",     &[1e-30, 1e-30], false); }
#[test] fn empty()           { compare("empty",    &[], false); }
#[test] fn one_zero()        { compare("one_zero", &[0.0], false); }
