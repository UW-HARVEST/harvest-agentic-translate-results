use std::os::raw::c_int;

pub type Mp3dSampleT = i16;

fn mp3d_scale_pcm(sample: f32) -> i16 {
    if sample >= 32766.5 {
        return 32767;
    }
    if sample <= -32767.5 {
        return -32768;
    }
    let mut s = (sample + 0.5f32) as i16;
    s -= (s < 0) as i16;
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_pair(pcm: *mut Mp3dSampleT, nch: c_int, z: *const f32) {
    let nch = nch as isize;
    unsafe {
        let mut a: f32;
        a = (*z.offset(14 * 64) - *z.offset(0)) * 29.0;
        a += (*z.offset(1 * 64) + *z.offset(13 * 64)) * 213.0;
        a += (*z.offset(12 * 64) - *z.offset(2 * 64)) * 459.0;
        a += (*z.offset(3 * 64) + *z.offset(11 * 64)) * 2037.0;
        a += (*z.offset(10 * 64) - *z.offset(4 * 64)) * 5153.0;
        a += (*z.offset(5 * 64) + *z.offset(9 * 64)) * 6574.0;
        a += (*z.offset(8 * 64) - *z.offset(6 * 64)) * 37489.0;
        a += *z.offset(7 * 64) * 75038.0;
        *pcm.offset(0) = mp3d_scale_pcm(a);

        let z = z.offset(2);
        a = *z.offset(14 * 64) * 104.0;
        a += *z.offset(12 * 64) * 1567.0;
        a += *z.offset(10 * 64) * 9727.0;
        a += *z.offset(8 * 64) * 64019.0;
        a += *z.offset(6 * 64) * -9975.0;
        a += *z.offset(4 * 64) * -45.0;
        a += *z.offset(2 * 64) * 146.0;
        a += *z.offset(0 * 64) * -5.0;
        *pcm.offset(16 * nch) = mp3d_scale_pcm(a);
    }
}
