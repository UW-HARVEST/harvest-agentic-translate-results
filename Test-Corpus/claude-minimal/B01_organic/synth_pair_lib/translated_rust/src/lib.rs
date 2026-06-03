//! Rust translation of c_src/src/lib.c

pub type Mp3dSampleT = i16;

fn mp3d_scale_pcm(sample: f32) -> i16 {
    if sample >= 32766.5 {
        return 32767i16;
    }
    if sample <= -32767.5 {
        return -32768i16;
    }
    let mut s: i16 = (sample + 0.5f32) as i16;
    s -= (s < 0) as i16;
    s
}

/// Safe Rust version of `synth_pair`.
///
/// `pcm` must be large enough so that index `16 * nch` is valid.
/// `z` must be large enough so that index `2 + 14 * 64` is valid.
pub fn synth_pair_safe(pcm: &mut [Mp3dSampleT], nch: usize, z: &[f32]) {
    let mut a: f32;
    a = (z[14 * 64] - z[0]) * 29.0;
    a += (z[1 * 64] + z[13 * 64]) * 213.0;
    a += (z[12 * 64] - z[2 * 64]) * 459.0;
    a += (z[3 * 64] + z[11 * 64]) * 2037.0;
    a += (z[10 * 64] - z[4 * 64]) * 5153.0;
    a += (z[5 * 64] + z[9 * 64]) * 6574.0;
    a += (z[8 * 64] - z[6 * 64]) * 37489.0;
    a += z[7 * 64] * 75038.0;
    pcm[0] = mp3d_scale_pcm(a);

    let z = &z[2..];
    a = z[14 * 64] * 104.0;
    a += z[12 * 64] * 1567.0;
    a += z[10 * 64] * 9727.0;
    a += z[8 * 64] * 64019.0;
    a += z[6 * 64] * -9975.0;
    a += z[4 * 64] * -45.0;
    a += z[2 * 64] * 146.0;
    a += z[0 * 64] * -5.0;
    pcm[16 * nch] = mp3d_scale_pcm(a);
}

/// C-compatible FFI wrapper that mirrors the original C signature:
/// `void synth_pair(mp3d_sample_t *pcm, int nch, const float *z);`
///
/// # Safety
///
/// The caller must ensure that `pcm` points to a buffer with at least
/// `16 * nch + 1` valid `i16` elements, and that `z` points to a buffer
/// with at least `2 + 14 * 64 + 1` valid `f32` elements.
#[no_mangle]
pub unsafe extern "C" fn synth_pair(
    pcm: *mut Mp3dSampleT,
    nch: core::ffi::c_int,
    z: *const f32,
) {
    let nch_usize = nch as usize;
    let pcm_slice = core::slice::from_raw_parts_mut(pcm, 16 * nch_usize + 1);
    let z_slice = core::slice::from_raw_parts(z, 2 + 14 * 64 + 1);
    synth_pair_safe(pcm_slice, nch_usize, z_slice);
}
