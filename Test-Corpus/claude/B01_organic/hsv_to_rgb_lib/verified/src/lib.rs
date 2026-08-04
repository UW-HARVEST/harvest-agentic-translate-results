use std::ffi::c_float;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsv_to_rgb(dest: *mut c_float, src: *const c_float) {
    let mut h: f32 = unsafe { *src.offset(0) };
    let s: f32 = unsafe { *src.offset(1) };
    let v: f32 = unsafe { *src.offset(2) };

    if s == 0.0 {
        unsafe {
            *dest.offset(0) = v;
            *dest.offset(1) = v;
            *dest.offset(2) = v;
        }
        return;
    }

    h /= 60.0_f32;
    // Match C's `(int)floorf(h)`. Rust's `as i32` saturates NaN and
    // out-of-range to 0 / i32::MAX / i32::MIN, but the C code on x86
    // compiles down to `cvttss2si`, which produces 0x80000000 (i32::MIN)
    // for NaN, +Inf, and any value that cannot be represented in i32.
    // Replicate that behaviour explicitly so the Rust output is
    // byte-identical to the C output.
    let h_floor = h.floor();
    let i: i32 = if h_floor.is_nan()
        || h_floor < -2147483648.0_f32
        || h_floor >= 2147483648.0_f32
    {
        i32::MIN
    } else {
        h_floor as i32
    };
    let f: f32 = h - (i as f32);
    let p: f32 = v * (1.0 - s);
    let q: f32 = v * (1.0 - s * f);
    let t: f32 = v * (1.0 - s * (1.0 - f));

    let (r, g, b): (f32, f32, f32) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };

    unsafe {
        *dest.offset(0) = r;
        *dest.offset(1) = g;
        *dest.offset(2) = b;
    }
}
