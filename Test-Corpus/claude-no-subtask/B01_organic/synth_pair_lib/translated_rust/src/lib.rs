use std::ffi::c_int;

pub type Mp3dSampleT = i16;

#[inline]
fn mp3d_scale_pcm(sample: f32) -> i16 {
    // Note: 32766.5 and -32767.5 are double-precision literals in C, so the
    // comparison promotes `sample` to double. Match that here for byte-identical
    // behavior at edge cases.
    if (sample as f64) >= 32766.5_f64 {
        return 32767i16;
    }
    if (sample as f64) <= -32767.5_f64 {
        return -32768i16;
    }
    // Float-to-int cast in Rust 1.45+ saturates; the prior bounds checks
    // guarantee the value is in i16 range, so behavior matches C truncation.
    let mut s = (sample + 0.5_f32) as i16;
    s -= (s < 0) as i16;
    s
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pair(
    pcm: *mut Mp3dSampleT,
    nch: c_int,
    z: *const f32,
) {
    // Helper to read z[i] from the (possibly shifted) base pointer.
    unsafe fn zr(z: *const f32, i: isize) -> f32 {
        *z.offset(i)
    }

    let mut z_ptr = z;

    let mut a: f32;
    a = (unsafe { zr(z_ptr, 14 * 64) } - unsafe { zr(z_ptr, 0) }) * 29.0_f32;
    a += (unsafe { zr(z_ptr, 1 * 64) } + unsafe { zr(z_ptr, 13 * 64) }) * 213.0_f32;
    a += (unsafe { zr(z_ptr, 12 * 64) } - unsafe { zr(z_ptr, 2 * 64) }) * 459.0_f32;
    a += (unsafe { zr(z_ptr, 3 * 64) } + unsafe { zr(z_ptr, 11 * 64) }) * 2037.0_f32;
    a += (unsafe { zr(z_ptr, 10 * 64) } - unsafe { zr(z_ptr, 4 * 64) }) * 5153.0_f32;
    a += (unsafe { zr(z_ptr, 5 * 64) } + unsafe { zr(z_ptr, 9 * 64) }) * 6574.0_f32;
    a += (unsafe { zr(z_ptr, 8 * 64) } - unsafe { zr(z_ptr, 6 * 64) }) * 37489.0_f32;
    a += unsafe { zr(z_ptr, 7 * 64) } * 75038.0_f32;
    unsafe {
        *pcm.offset(0) = mp3d_scale_pcm(a);
    }

    z_ptr = unsafe { z_ptr.offset(2) };

    a = unsafe { zr(z_ptr, 14 * 64) } * 104.0_f32;
    a += unsafe { zr(z_ptr, 12 * 64) } * 1567.0_f32;
    a += unsafe { zr(z_ptr, 10 * 64) } * 9727.0_f32;
    a += unsafe { zr(z_ptr, 8 * 64) } * 64019.0_f32;
    a += unsafe { zr(z_ptr, 6 * 64) } * -9975.0_f32;
    a += unsafe { zr(z_ptr, 4 * 64) } * -45.0_f32;
    a += unsafe { zr(z_ptr, 2 * 64) } * 146.0_f32;
    a += unsafe { zr(z_ptr, 0 * 64) } * -5.0_f32;
    unsafe {
        *pcm.offset((16 * nch) as isize) = mp3d_scale_pcm(a);
    }
}
