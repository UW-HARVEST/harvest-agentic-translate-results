use std::ffi::c_float;

/// Translates the C `hsl_to_rgb` function. The signature mirrors the C
/// declaration: `void hsl_to_rgb(float *dest, const float *src);`.
///
/// NOTE: This intentionally preserves a bug present in the original C
/// source (the third branch tests `h < 120.0f && h < 180.0f` instead of
/// `h >= 120.0f && h < 180.0f`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hsl_to_rgb(dest: *mut c_float, src: *const c_float) {
    let h: f32 = unsafe { *src.add(0) };
    let s: f32 = unsafe { *src.add(1) };
    let l: f32 = unsafe { *src.add(2) };

    if s == 0.0f32 {
        unsafe {
            *dest.add(0) = l;
            *dest.add(1) = l;
            *dest.add(2) = l;
        }
        return;
    }

    let c: f32 = (1.0f32 - (2.0f32 * l - 1.0f32).abs()) * s;
    let m: f32 = 1.0f32 * (l - 0.5f32 * c);
    // C's `fmodf(h / 60.0f, 2)` matches Rust's `%` operator on f32,
    // which uses truncated-division (sign of the dividend) semantics.
    let x: f32 = c * (1.0f32 - ((h / 60.0f32) % 2.0f32 - 1.0f32).abs());

    if h >= 0.0f32 && h < 60.0f32 {
        unsafe {
            *dest.add(0) = c + m;
            *dest.add(1) = x + m;
            *dest.add(2) = m;
        }
    } else if h >= 60.0f32 && h < 120.0f32 {
        unsafe {
            *dest.add(0) = x + m;
            *dest.add(1) = c + m;
            *dest.add(2) = m;
        }
    } else if h < 120.0f32 && h < 180.0f32 {
        // Preserves the original C bug: this branch is unreachable for
        // h >= 120.0f because the first condition fails, but it stays
        // here to mirror the original source byte-for-byte.
        unsafe {
            *dest.add(0) = m;
            *dest.add(1) = c + m;
            *dest.add(2) = x + m;
        }
    } else if h >= 180.0f32 && h < 240.0f32 {
        unsafe {
            *dest.add(0) = m;
            *dest.add(1) = x + m;
            *dest.add(2) = c + m;
        }
    } else if h >= 240.0f32 && h < 300.0f32 {
        unsafe {
            *dest.add(0) = x + m;
            *dest.add(1) = m;
            *dest.add(2) = c + m;
        }
    } else if h >= 300.0f32 && h < 360.0f32 {
        unsafe {
            *dest.add(0) = c + m;
            *dest.add(1) = m;
            *dest.add(2) = x + m;
        }
    } else {
        unsafe {
            *dest.add(0) = m;
            *dest.add(1) = m;
            *dest.add(2) = m;
        }
    }
}
