use std::ffi::c_int;

pub type Mp3dSample = i16;

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

/// # Safety
/// `pcm` must point to a writable buffer with enough space for `pcm[0]` and
/// `pcm[16 * nch]`. `z` must point to a readable buffer of at least `15 * 64 + 2`
/// `f32` elements.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pair(pcm: *mut i16, nch: c_int, z: *const f32) {
    let mut a: f32;
    a = (*z.add(14 * 64) - *z.add(0)) * 29.0f32;
    a += (*z.add(1 * 64) + *z.add(13 * 64)) * 213.0f32;
    a += (*z.add(12 * 64) - *z.add(2 * 64)) * 459.0f32;
    a += (*z.add(3 * 64) + *z.add(11 * 64)) * 2037.0f32;
    a += (*z.add(10 * 64) - *z.add(4 * 64)) * 5153.0f32;
    a += (*z.add(5 * 64) + *z.add(9 * 64)) * 6574.0f32;
    a += (*z.add(8 * 64) - *z.add(6 * 64)) * 37489.0f32;
    a += *z.add(7 * 64) * 75038.0f32;
    *pcm.add(0) = mp3d_scale_pcm(a);
    let z = z.add(2);
    a = *z.add(14 * 64) * 104.0f32;
    a += *z.add(12 * 64) * 1567.0f32;
    a += *z.add(10 * 64) * 9727.0f32;
    a += *z.add(8 * 64) * 64019.0f32;
    a += *z.add(6 * 64) * -9975.0f32;
    a += *z.add(4 * 64) * -45.0f32;
    a += *z.add(2 * 64) * 146.0f32;
    a += *z.add(0 * 64) * -5.0f32;
    *pcm.add((16 * nch) as usize) = mp3d_scale_pcm(a);
}
