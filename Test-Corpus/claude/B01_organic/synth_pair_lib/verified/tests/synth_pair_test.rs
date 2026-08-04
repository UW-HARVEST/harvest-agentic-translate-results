use libloading::{Library, Symbol};
use std::os::raw::c_int;

type SynthPairFn = unsafe extern "C" fn(*mut i16, c_int, *const f32);

const C_LIB_PATH: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB_PATH: &str = "target/release/libsynth_pair_lib.so";

fn z_len() -> usize {
    // The function reads z[14*64] originally, and after z += 2 reads up to (z+2)[14*64]
    // = z[14*64 + 2]. So we need 14*64 + 2 + 1 = 899 elements. Add some padding.
    14 * 64 + 3
}

fn run_both(z: &[f32], nch: c_int) -> (Vec<i16>, Vec<i16>) {
    assert!(z.len() >= z_len());
    let pcm_len = (16 * nch as usize) + 1;
    let sentinel: i16 = 0x5A5A;
    let mut pcm_c = vec![sentinel; pcm_len];
    let mut pcm_rust = vec![sentinel; pcm_len];

    unsafe {
        let c_lib = Library::new(C_LIB_PATH).expect("failed to load C lib");
        let c_synth: Symbol<SynthPairFn> = c_lib.get(b"synth_pair").expect("missing C symbol");
        let rust_lib = Library::new(RUST_LIB_PATH).expect("failed to load Rust lib");
        let rust_synth: Symbol<SynthPairFn> =
            rust_lib.get(b"synth_pair").expect("missing Rust symbol");

        c_synth(pcm_c.as_mut_ptr(), nch, z.as_ptr());
        rust_synth(pcm_rust.as_mut_ptr(), nch, z.as_ptr());
    }

    (pcm_c, pcm_rust)
}

fn assert_match(pcm_c: &[i16], pcm_rust: &[i16], label: &str) {
    assert_eq!(
        pcm_c, pcm_rust,
        "mismatch for {label}: C={pcm_c:?} Rust={pcm_rust:?}"
    );
}

#[test]
fn test_synth_pair_zeros() {
    let z = vec![0.0f32; z_len()];
    for nch in 1..=2 {
        let (c, r) = run_both(&z, nch);
        assert_match(&c, &r, &format!("zeros nch={nch}"));
    }
}

#[test]
fn test_synth_pair_ones() {
    let z = vec![1.0f32; z_len()];
    for nch in 1..=2 {
        let (c, r) = run_both(&z, nch);
        assert_match(&c, &r, &format!("ones nch={nch}"));
    }
}

#[test]
fn test_synth_pair_negative_ones() {
    let z = vec![-1.0f32; z_len()];
    for nch in 1..=2 {
        let (c, r) = run_both(&z, nch);
        assert_match(&c, &r, &format!("neg-ones nch={nch}"));
    }
}

#[test]
fn test_synth_pair_ramp() {
    let mut z = vec![0.0f32; z_len()];
    for (i, v) in z.iter_mut().enumerate() {
        *v = i as f32;
    }
    for nch in 1..=2 {
        let (c, r) = run_both(&z, nch);
        assert_match(&c, &r, &format!("ramp nch={nch}"));
    }
}

#[test]
fn test_synth_pair_alternating() {
    let mut z = vec![0.0f32; z_len()];
    for (i, v) in z.iter_mut().enumerate() {
        *v = if i % 2 == 0 { 1000.0 } else { -1000.0 };
    }
    for nch in 1..=2 {
        let (c, r) = run_both(&z, nch);
        assert_match(&c, &r, &format!("alt nch={nch}"));
    }
}

#[test]
fn test_synth_pair_saturation_high() {
    // Force saturation upward by giving a large positive at z[7*64]
    let mut z = vec![0.0f32; z_len()];
    z[7 * 64] = 1.0e6;
    for nch in 1..=2 {
        let (c, r) = run_both(&z, nch);
        assert_match(&c, &r, &format!("sat-high nch={nch}"));
    }
}

#[test]
fn test_synth_pair_saturation_low() {
    let mut z = vec![0.0f32; z_len()];
    z[7 * 64] = -1.0e6;
    for nch in 1..=2 {
        let (c, r) = run_both(&z, nch);
        assert_match(&c, &r, &format!("sat-low nch={nch}"));
    }
}

#[test]
fn test_synth_pair_random_seeded() {
    // Deterministic LCG so we don't pull in `rand`.
    let mut state: u32 = 0xdeadbeef;
    let mut z = vec![0.0f32; z_len()];
    for v in z.iter_mut() {
        state = state.wrapping_mul(1103515245).wrapping_add(12345);
        // Map to [-1.0, 1.0] then scale.
        let n = (state >> 16) as i16 as f32 / 32768.0;
        *v = n * 100.0;
    }
    for nch in 1..=4 {
        let (c, r) = run_both(&z, nch);
        assert_match(&c, &r, &format!("random nch={nch}"));
    }
}

#[test]
fn test_synth_pair_small_values() {
    let mut z = vec![0.0f32; z_len()];
    for (i, v) in z.iter_mut().enumerate() {
        *v = (i as f32) * 1e-7;
    }
    for nch in 1..=2 {
        let (c, r) = run_both(&z, nch);
        assert_match(&c, &r, &format!("small nch={nch}"));
    }
}

#[test]
fn test_synth_pair_boundary_pcm_value() {
    // Try to exercise the rounding edge of mp3d_scale_pcm.
    // mp3d_scale_pcm: If sample is e.g. -0.5, falls into -32767.5 < x <= 32766.5,
    // s = (sample + 0.5) as i16 = 0, then s -= (s < 0) = 0. So returns 0.
    // For sample = -0.5001, s = (-0.0001) as i16 = 0, so s -= 0 = 0. (truncation toward 0).
    // Try lots of tiny values.
    for offset in [-1.0, -0.5, -0.4, 0.0, 0.4, 0.5, 1.0, 1.5] {
        let mut z = vec![0.0f32; z_len()];
        // Set z[7*64] so a = 75038*z[7*64], so we can hit specific PCM values.
        z[7 * 64] = offset / 75038.0;
        for nch in 1..=2 {
            let (c, r) = run_both(&z, nch);
            assert_match(&c, &r, &format!("boundary off={offset} nch={nch}"));
        }
    }
}
