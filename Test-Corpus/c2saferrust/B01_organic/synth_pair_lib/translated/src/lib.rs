

pub type __int16_t = i16;
pub type int16_t = __int16_t;
pub type mp3d_sample_t = int16_t;
fn mp3d_scale_pcm(sample: f32) -> i16 {
    if sample >= 32766.5 {
        return 32767;
    }
    if sample <= -32767.5 {
        return -32768;
    }

    let s = (sample + 0.5) as i16;
    s - if s < 0 { 1 } else { 0 }
}

#[no_mangle]
pub fn synth_pair(pcm: &mut [mp3d_sample_t], nch: i32, z: &[f32]) {
    let nch = nch as usize;

    let mut a: f32 =
        (z[14 * 64] - z[0]) * 29.0;
    a += (z[1 * 64] + z[13 * 64]) * 213.0;
    a += (z[12 * 64] - z[2 * 64]) * 459.0;
    a += (z[3 * 64] + z[11 * 64]) * 2037.0;
    a += (z[10 * 64] - z[4 * 64]) * 5153.0;
    a += (z[5 * 64] + z[9 * 64]) * 6574.0;
    a += (z[8 * 64] - z[6 * 64]) * 37489.0;
    a += z[7 * 64] * 75038.0;
    pcm[0] = mp3d_scale_pcm(a) as mp3d_sample_t;

    let z = &z[2..];
    let mut a: f32 = z[14 * 64] * 104.0;
    a += z[12 * 64] * 1567.0;
    a += z[10 * 64] * 9727.0;
    a += z[8 * 64] * 64019.0;
    a += z[6 * 64] * -9975.0;
    a += z[4 * 64] * -45.0;
    a += z[2 * 64] * 146.0;
    a += z[0] * -5.0;
    pcm[16 * nch] = mp3d_scale_pcm(a) as mp3d_sample_t;
}

