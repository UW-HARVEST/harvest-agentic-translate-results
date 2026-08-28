#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut f32, src: *const f32) {
    let mut h = unsafe { *src };
    let s = unsafe { *src.add(1) };
    let v = unsafe { *src.add(2) };

    if s == 0.0 {
        unsafe {
            *dest = v;
            *dest.add(1) = v;
            *dest.add(2) = v;
        }
        return;
    }

    h /= 60.0;
    let i = c_float_to_int(h.floor());
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));

    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    unsafe {
        *dest = r;
        *dest.add(1) = g;
        *dest.add(2) = b;
    }
}

#[cfg(target_arch = "x86_64")]
fn c_float_to_int(value: f32) -> i32 {
    // Match the cvttss2si emitted by the reference C build for undefined casts.
    unsafe { core::arch::x86_64::_mm_cvttss_si32(core::arch::x86_64::_mm_set_ss(value)) }
}

#[cfg(target_arch = "x86")]
fn c_float_to_int(value: f32) -> i32 {
    // Match the cvttss2si emitted by the reference C build for undefined casts.
    unsafe { core::arch::x86::_mm_cvttss_si32(core::arch::x86::_mm_set_ss(value)) }
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
fn c_float_to_int(value: f32) -> i32 {
    value as i32
}
