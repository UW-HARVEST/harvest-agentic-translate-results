use std::ffi::c_int;

#[allow(non_camel_case_types)]
pub type mp3d_sample_t = i16;

#[inline]
fn mp3d_scale_pcm(sample: f32) -> i16 {
    // C compares `sample` (float) with double-precision literals 32766.5 and -32767.5,
    // both of which are exactly representable in f32. Comparisons are done after the
    // usual arithmetic conversions promote `sample` to double.
    if (sample as f64) >= 32766.5_f64 {
        return 32767i16;
    }
    if (sample as f64) <= -32767.5_f64 {
        return -32768i16;
    }
    // (int16_t)(sample + .5f) — truncation toward zero.
    let mut s: i16 = (sample + 0.5f32) as i16;
    // s -= (s < 0);
    s -= (s < 0) as i16;
    s
}

/// # Safety
/// Caller must provide a `pcm` buffer with capacity for at least `16 * nch + 1` samples,
/// and a `z` buffer with at least `15 * 64 + 2` valid `f32` elements (matching the C ABI).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pair(
    pcm: *mut mp3d_sample_t,
    nch: c_int,
    z: *const f32,
) {
    let mut a: f32;

    a = (*z.offset(14 * 64) - *z.offset(0)) * 29f32;
    a += (*z.offset(1 * 64) + *z.offset(13 * 64)) * 213f32;
    a += (*z.offset(12 * 64) - *z.offset(2 * 64)) * 459f32;
    a += (*z.offset(3 * 64) + *z.offset(11 * 64)) * 2037f32;
    a += (*z.offset(10 * 64) - *z.offset(4 * 64)) * 5153f32;
    a += (*z.offset(5 * 64) + *z.offset(9 * 64)) * 6574f32;
    a += (*z.offset(8 * 64) - *z.offset(6 * 64)) * 37489f32;
    a += *z.offset(7 * 64) * 75038f32;
    *pcm.offset(0) = mp3d_scale_pcm(a);

    let z = z.offset(2);
    a = *z.offset(14 * 64) * 104f32;
    a += *z.offset(12 * 64) * 1567f32;
    a += *z.offset(10 * 64) * 9727f32;
    a += *z.offset(8 * 64) * 64019f32;
    a += *z.offset(6 * 64) * -9975f32;
    a += *z.offset(4 * 64) * -45f32;
    a += *z.offset(2 * 64) * 146f32;
    a += *z.offset(0 * 64) * -5f32;
    *pcm.offset((16 * nch) as isize) = mp3d_scale_pcm(a);
}
