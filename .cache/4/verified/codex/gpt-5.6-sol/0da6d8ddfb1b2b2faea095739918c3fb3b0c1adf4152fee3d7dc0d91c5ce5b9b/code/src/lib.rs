use std::ffi::c_float;

/// Converts an HSV color triplet to RGB.
///
/// The caller must provide readable and writable pointers to three consecutive
/// `c_float` values, respectively.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    // Read the complete input first because the C API permits overlapping buffers.
    let (mut h, s, v) = unsafe { (src.read(), src.add(1).read(), src.add(2).read()) };

    if s == 0.0 {
        unsafe {
            dest.write(v);
            dest.add(1).write(v);
            dest.add(2).write(v);
        }
        return;
    }

    h /= 60.0_f32;
    let i = c_float_to_int(h.floor());
    let f = h - i as f32;
    let p = v * (1.0_f32 - s);
    let q = v * (1.0_f32 - s * f);
    let t = v * (1.0_f32 - s * (1.0_f32 - f));

    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    unsafe {
        dest.write(r);
        dest.add(1).write(g);
        dest.add(2).write(b);
    }
}

#[inline]
fn c_float_to_int(value: f32) -> i32 {
    #[cfg(target_arch = "x86")]
    unsafe {
        use core::arch::x86::{_mm_cvttss_si32, _mm_set_ss};
        _mm_cvttss_si32(_mm_set_ss(value))
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        use core::arch::x86_64::{_mm_cvttss_si32, _mm_set_ss};
        _mm_cvttss_si32(_mm_set_ss(value))
    }

    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    {
        value as i32
    }
}
