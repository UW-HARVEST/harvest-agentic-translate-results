use std::ffi::c_int;

type Mp3dSample = i16;

#[inline]
fn scale_pcm(sample: f32) -> Mp3dSample {
    if sample >= 32766.5 {
        return 32767;
    }
    if sample <= -32767.5 {
        return -32768;
    }
    let mut scaled = (sample + 0.5) as Mp3dSample;
    scaled -= (scaled < 0) as Mp3dSample;
    scaled
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pair(pcm: *mut Mp3dSample, nch: c_int, z: *const f32) {
    let mut a = unsafe { (*z.add(14 * 64) - *z) * 29.0 };
    a += unsafe { (*z.add(64) + *z.add(13 * 64)) * 213.0 };
    a += unsafe { (*z.add(12 * 64) - *z.add(2 * 64)) * 459.0 };
    a += unsafe { (*z.add(3 * 64) + *z.add(11 * 64)) * 2037.0 };
    a += unsafe { (*z.add(10 * 64) - *z.add(4 * 64)) * 5153.0 };
    a += unsafe { (*z.add(5 * 64) + *z.add(9 * 64)) * 6574.0 };
    a += unsafe { (*z.add(8 * 64) - *z.add(6 * 64)) * 37489.0 };
    a += unsafe { *z.add(7 * 64) * 75038.0 };
    unsafe { *pcm = scale_pcm(a) };

    let z = unsafe { z.add(2) };
    a = unsafe { *z.add(14 * 64) * 104.0 };
    a += unsafe { *z.add(12 * 64) * 1567.0 };
    a += unsafe { *z.add(10 * 64) * 9727.0 };
    a += unsafe { *z.add(8 * 64) * 64019.0 };
    a += unsafe { *z.add(6 * 64) * -9975.0 };
    a += unsafe { *z.add(4 * 64) * -45.0 };
    a += unsafe { *z.add(2 * 64) * 146.0 };
    a += unsafe { *z * -5.0 };
    let output_offset = nch.wrapping_mul(16) as isize;
    unsafe { *pcm.offset(output_offset) = scale_pcm(a) };
}
