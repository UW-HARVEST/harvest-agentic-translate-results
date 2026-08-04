//! Translation of c_src/src/lib.c to Rust.
//!
//! Mirrors the C `premultiply` routine exactly so that, given the same
//! input pixel buffer, the resulting bytes are byte-identical.

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(C)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug)]
pub struct CpImage {
    pub w: i32,
    pub h: i32,
    pub pix: Vec<CpPixel>,
}

/// Pre-multiply alpha into RGB channels.
///
/// Faithful translation of `premultiply` in `c_src/src/lib.c`. Performs the
/// computation in `f32` and truncates back to `u8`, matching the C cast
/// semantics.
pub fn premultiply(img: &mut CpImage) {
    let w = img.w as i64;
    let h = img.h as i64;
    let pixel_size: i64 = std::mem::size_of::<CpPixel>() as i64;
    let stride: i64 = w * pixel_size;
    let total_bytes: i64 = stride * h;

    // Reinterpret the pixel slice as a flat byte slice (matching the C
    // `(uint8_t *)img->pix` reinterpretation).
    let data: &mut [u8] = unsafe {
        std::slice::from_raw_parts_mut(
            img.pix.as_mut_ptr() as *mut u8,
            (img.pix.len() * std::mem::size_of::<CpPixel>()) as usize,
        )
    };

    let mut i: i64 = 0;
    while i < total_bytes {
        let idx = i as usize;
        let a = data[idx + 3] as f32 / 255.0f32;
        let mut r = data[idx + 0] as f32 / 255.0f32;
        let mut g = data[idx + 1] as f32 / 255.0f32;
        let mut b = data[idx + 2] as f32 / 255.0f32;
        r *= a;
        g *= a;
        b *= a;
        data[idx + 0] = float_to_u8_trunc(r * 255.0f32);
        data[idx + 1] = float_to_u8_trunc(g * 255.0f32);
        data[idx + 2] = float_to_u8_trunc(b * 255.0f32);
        i += pixel_size;
    }
}

/// Mimic C's `(uint8_t)<float>` cast: truncate toward zero, with values that
/// are in-range mapped directly. Values are guaranteed to be in `[0, 255]` for
/// the premultiply use case, but we clamp defensively to preserve the C
/// implementation-defined behavior for out-of-range conversions.
fn float_to_u8_trunc(v: f32) -> u8 {
    // Truncate toward zero, matching C's float-to-integer conversion.
    let truncated = v.trunc();
    if truncated >= 255.0 {
        255
    } else if truncated <= 0.0 {
        0
    } else {
        truncated as u8
    }
}
