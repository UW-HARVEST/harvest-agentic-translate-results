use std::os::raw::{c_float, c_int};

type Mp3dSampleT = i16;

#[inline]
fn mp3d_scale_pcm(sample: f32) -> i16 {
    // Comparisons in C are done in double precision because the literals
    // 32766.5 and -32767.5 have no `f` suffix.
    if (sample as f64) >= 32766.5_f64 {
        return 32767i16;
    }
    if (sample as f64) <= -32767.5_f64 {
        return -32768i16;
    }
    // (int16_t)(sample + .5f) — truncation toward zero in C.
    // `as i16` in Rust performs saturating conversion; the prior range
    // checks guarantee the value is well within i16 range here.
    let mut s: i16 = (sample + 0.5_f32) as i16;
    // s -= (s < 0) — subtracts 1 if negative, else 0.
    if s < 0 {
        s -= 1;
    }
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pair(
    pcm: *mut Mp3dSampleT,
    nch: c_int,
    z: *const c_float,
) {
    unsafe {
        let mut z = z;
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

        z = z.add(2);

        a = *z.add(14 * 64) * 104.0;
        a += *z.add(12 * 64) * 1567.0;
        a += *z.add(10 * 64) * 9727.0;
        a += *z.add(8 * 64) * 64019.0;
        a += *z.add(6 * 64) * -9975.0;
        a += *z.add(4 * 64) * -45.0;
        a += *z.add(2 * 64) * 146.0;
        a += *z.add(0 * 64) * -5.0;
        *pcm.add((16 * nch) as usize) = mp3d_scale_pcm(a);
    }
}
