use std::ffi::{c_float, c_int};
use std::os::raw::c_short;

pub type mp3d_sample_t = c_short;

fn mp3d_scale_pcm(sample: f32) -> i16 {
    if sample >= 32766.5 {
        return 32767;
    }
    if sample <= -32767.5 {
        return -32768;
    }
    let s = (sample + 0.5) as i16;
    s - (s < 0) as i16
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pair(pcm: *mut mp3d_sample_t, nch: c_int, z: *const c_float) {
    let pcm_slice = unsafe { std::slice::from_raw_parts_mut(pcm, (16 * nch as usize) + 1) };
    let z_slice = unsafe { std::slice::from_raw_parts(z, 15 * 64) };
    
    let mut a: f32;
    a = (z_slice[14 * 64] - z_slice[0]) * 29.0;
    a += (z_slice[1 * 64] + z_slice[13 * 64]) * 213.0;
    a += (z_slice[12 * 64] - z_slice[2 * 64]) * 459.0;
    a += (z_slice[3 * 64] + z_slice[11 * 64]) * 2037.0;
    a += (z_slice[10 * 64] - z_slice[4 * 64]) * 5153.0;
    a += (z_slice[5 * 64] + z_slice[9 * 64]) * 6574.0;
    a += (z_slice[8 * 64] - z_slice[6 * 64]) * 37489.0;
    a += z_slice[7 * 64] * 75038.0;
    pcm_slice[0] = mp3d_scale_pcm(a);
    
    let z2 = &z_slice[2..];
    a = z2[14 * 64] * 104.0;
    a += z2[12 * 64] * 1567.0;
    a += z2[10 * 64] * 9727.0;
    a += z2[8 * 64] * 64019.0;
    a += z2[6 * 64] * -9975.0;
    a += z2[4 * 64] * -45.0;
    a += z2[2 * 64] * 146.0;
    a += z2[0 * 64] * -5.0;
    pcm_slice[16 * nch as usize] = mp3d_scale_pcm(a);
}
