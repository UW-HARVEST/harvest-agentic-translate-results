use libloading::{Library, Symbol};
use std::path::PathBuf;

type SynthPairFn = unsafe extern "C" fn(*mut i16, i32, *const f32);

fn lib_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = manifest.join("c_src/build/libtranslated_rust.so");
    let rust_so = manifest.join("target/debug/libsynth_pair_lib.so");
    assert!(c_so.exists(), "C .so not found at {}", c_so.display());
    assert!(rust_so.exists(), "Rust .so not found at {}", rust_so.display());
    (c_so, rust_so)
}

fn call_synth_pair(lib: &Library, nch: i32, z: &[f32]) -> Vec<i16> {
    unsafe {
        let func: Symbol<SynthPairFn> = lib.get(b"synth_pair").unwrap();
        let mut pcm = vec![0i16; (16 * nch + 1) as usize];
        func(pcm.as_mut_ptr(), nch, z.as_ptr());
        pcm
    }
}

fn make_z(fill: impl Fn(usize) -> f32) -> Vec<f32> {
    (0..899).map(|i| fill(i)).collect()
}

#[test]
fn test_synth_pair_zeros() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        for nch in [1, 2] {
            let z = make_z(|_| 0.0);
            let c_out = call_synth_pair(&c_lib, nch, &z);
            let rs_out = call_synth_pair(&rs_lib, nch, &z);
            assert_eq!(c_out, rs_out, "mismatch zeros nch={nch}");
        }
    }
}

#[test]
fn test_synth_pair_ones() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        for nch in [1, 2] {
            let z = make_z(|_| 1.0);
            let c_out = call_synth_pair(&c_lib, nch, &z);
            let rs_out = call_synth_pair(&rs_lib, nch, &z);
            assert_eq!(c_out, rs_out, "mismatch ones nch={nch}");
        }
    }
}

#[test]
fn test_synth_pair_index_pattern() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        for nch in [1, 2] {
            let z = make_z(|i| i as f32 * 0.01);
            let c_out = call_synth_pair(&c_lib, nch, &z);
            let rs_out = call_synth_pair(&rs_lib, nch, &z);
            assert_eq!(c_out, rs_out, "mismatch index nch={nch}");
        }
    }
}

#[test]
fn test_synth_pair_negative() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        for nch in [1, 2] {
            let z = make_z(|i| -(i as f32) * 0.05);
            let c_out = call_synth_pair(&c_lib, nch, &z);
            let rs_out = call_synth_pair(&rs_lib, nch, &z);
            assert_eq!(c_out, rs_out, "mismatch negative nch={nch}");
        }
    }
}

#[test]
fn test_synth_pair_saturation() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        for nch in [1, 2] {
            // Large values to trigger clamping
            let z = make_z(|_| 999.0);
            let c_out = call_synth_pair(&c_lib, nch, &z);
            let rs_out = call_synth_pair(&rs_lib, nch, &z);
            assert_eq!(c_out, rs_out, "mismatch saturation nch={nch}");

            let z = make_z(|_| -999.0);
            let c_out = call_synth_pair(&c_lib, nch, &z);
            let rs_out = call_synth_pair(&rs_lib, nch, &z);
            assert_eq!(c_out, rs_out, "mismatch neg saturation nch={nch}");
        }
    }
}

#[test]
fn test_synth_pair_mixed() {
    let (c_path, rs_path) = lib_paths();
    unsafe {
        let c_lib = Library::new(&c_path).unwrap();
        let rs_lib = Library::new(&rs_path).unwrap();
        for nch in [1, 2] {
            let z = make_z(|i| if i % 2 == 0 { 0.3 } else { -0.7 });
            let c_out = call_synth_pair(&c_lib, nch, &z);
            let rs_out = call_synth_pair(&rs_lib, nch, &z);
            assert_eq!(c_out, rs_out, "mismatch mixed nch={nch}");
        }
    }
}
