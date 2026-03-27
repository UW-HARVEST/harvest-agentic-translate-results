use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libsynth_pair_lib.so")
}

/// Call C synth_pair via .so
fn call_c_synth_pair(nch: i32, z: &[f32]) -> (i16, i16, usize) {
    unsafe {
        let lib = Library::new(c_lib_path()).expect("Failed to load C .so");
        let func: Symbol<unsafe extern "C" fn(*mut i16, i32, *const f32)> =
            lib.get(b"synth_pair").expect("synth_pair not found in C .so");
        let mut pcm = vec![0i16; 16 * nch as usize + 1];
        func(pcm.as_mut_ptr(), nch, z.as_ptr());
        (pcm[0], pcm[16 * nch as usize], pcm.len())
    }
}

/// Call Rust synth_pair directly
fn call_rust_synth_pair(nch: i32, z: &[f32]) -> (i16, i16) {
    let mut pcm = vec![0i16; 16 * nch as usize + 1];
    unsafe {
        synth_pair_lib::synth_pair(pcm.as_mut_ptr(), nch, z.as_ptr());
    }
    (pcm[0], pcm[16 * nch as usize])
}

fn make_z(seed: u32) -> Vec<f32> {
    // Need at least (14*64 + 2) + 1 = 899 elements for z, plus z+2 offsets
    let n = 14 * 64 + 2 + 1;
    let mut z = vec![0.0f32; n];
    let mut v = seed;
    for i in 0..n {
        // Simple LCG for deterministic pseudo-random floats
        v = v.wrapping_mul(1103515245).wrapping_add(12345);
        z[i] = ((v >> 16) as i32 as f32) / 100.0;
    }
    z
}

#[test]
fn test_synth_pair_zeros() {
    let z = vec![0.0f32; 1024];
    for nch in [1, 2] {
        let (c0, c1, _) = call_c_synth_pair(nch, &z);
        let (r0, r1) = call_rust_synth_pair(nch, &z);
        assert_eq!((c0, c1), (r0, r1), "mismatch with zeros, nch={nch}");
    }
}

#[test]
fn test_synth_pair_ones() {
    let z = vec![1.0f32; 1024];
    for nch in [1, 2] {
        let (c0, c1, _) = call_c_synth_pair(nch, &z);
        let (r0, r1) = call_rust_synth_pair(nch, &z);
        assert_eq!((c0, c1), (r0, r1), "mismatch with ones, nch={nch}");
    }
}

#[test]
fn test_synth_pair_negative() {
    let z = vec![-1.0f32; 1024];
    for nch in [1, 2] {
        let (c0, c1, _) = call_c_synth_pair(nch, &z);
        let (r0, r1) = call_rust_synth_pair(nch, &z);
        assert_eq!((c0, c1), (r0, r1), "mismatch with negatives, nch={nch}");
    }
}

#[test]
fn test_synth_pair_large_values() {
    let z = vec![1000.0f32; 1024];
    for nch in [1, 2] {
        let (c0, c1, _) = call_c_synth_pair(nch, &z);
        let (r0, r1) = call_rust_synth_pair(nch, &z);
        assert_eq!((c0, c1), (r0, r1), "mismatch with large values, nch={nch}");
    }
}

#[test]
fn test_synth_pair_pseudo_random_seeds() {
    for seed in [1u32, 42, 12345, 999999, 0xDEADBEEF] {
        let z = make_z(seed);
        for nch in [1, 2] {
            let (c0, c1, _) = call_c_synth_pair(nch, &z);
            let (r0, r1) = call_rust_synth_pair(nch, &z);
            assert_eq!(
                (c0, c1), (r0, r1),
                "mismatch seed={seed}, nch={nch}: C=({c0},{c1}) Rust=({r0},{r1})"
            );
        }
    }
}

#[test]
fn test_synth_pair_boundary_clipping() {
    // Values designed to trigger clipping in mp3d_scale_pcm
    let mut z = vec![0.0f32; 1024];
    // Set z[7*64] to a large value to push pcm[0] to clipping
    z[7 * 64] = 1.0;
    for nch in [1, 2] {
        let (c0, c1, _) = call_c_synth_pair(nch, &z);
        let (r0, r1) = call_rust_synth_pair(nch, &z);
        assert_eq!((c0, c1), (r0, r1), "mismatch boundary test, nch={nch}");
    }
}
