use std::ffi::c_int;

type Mp3dSampleT = i16;

fn mp3d_scale_pcm(sample: f32) -> i16 {
    if sample >= 32766.5 {
        return 32767_i16;
    }
    if sample <= -32767.5 {
        return -32768_i16;
    }
    let mut s = (sample + 0.5) as i16;
    s -= (s < 0) as i16;
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_pair(pcm: *mut Mp3dSampleT, nch: c_int, z: *const f32) {
    unsafe {
        let mut a: f32;
        a = (*z.add(14 * 64) - *z.add(0)) * 29.0;
        a += (*z.add(1 * 64) + *z.add(13 * 64)) * 213.0;
        a += (*z.add(12 * 64) - *z.add(2 * 64)) * 459.0;
        a += (*z.add(3 * 64) + *z.add(11 * 64)) * 2037.0;
        a += (*z.add(10 * 64) - *z.add(4 * 64)) * 5153.0;
        a += (*z.add(5 * 64) + *z.add(9 * 64)) * 6574.0;
        a += (*z.add(8 * 64) - *z.add(6 * 64)) * 37489.0;
        a += *z.add(7 * 64) * 75038.0;
        *pcm.add(0) = mp3d_scale_pcm(a);

        let z = z.add(2);
        a = *z.add(14 * 64) * 104.0;
        a += *z.add(12 * 64) * 1567.0;
        a += *z.add(10 * 64) * 9727.0;
        a += *z.add(8 * 64) * 64019.0;
        a += *z.add(6 * 64) * -9975.0;
        a += *z.add(4 * 64) * -45.0;
        a += *z.add(2 * 64) * 146.0;
        a += *z.add(0 * 64) * -5.0;
        *pcm.add(16 * nch as usize) = mp3d_scale_pcm(a);
    }
}
