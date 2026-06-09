// Translation of c_src/src/lib.c to Rust.
//
// The original C code is a shared library exposing a single function,
// `synth_pair`. There is no `main` in the C source, so the executable
// produced here mirrors that by having a no-op `main`.

#![allow(dead_code)]

pub type Mp3dSampleT = i16;

fn mp3d_scale_pcm(sample: f32) -> i16 {
    if sample >= 32766.5 {
        return 32767i16;
    }
    if sample <= -32767.5 {
        return -32768i16;
    }
    // C: int16_t s = (int16_t)(sample + .5f);
    // C cast from float to int16_t truncates toward zero.
    let s_full = (sample + 0.5f32) as i32;
    let mut s = s_full as i16;
    // C: s -= (s < 0);
    if s < 0 {
        s = s.wrapping_sub(1);
    }
    s
}

/// Safe Rust translation of synth_pair.
///
/// `pcm` is the output buffer, `nch` is the number of channels,
/// `z` is the input buffer. Writes into `pcm[0]` and `pcm[16 * nch]`.
pub fn synth_pair(pcm: &mut [Mp3dSampleT], nch: usize, z: &[f32]) {
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

    // After z += 2 in C, indexing is offset by 2.
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

fn main() {
    // The C library has no main, so produce no output.
}
