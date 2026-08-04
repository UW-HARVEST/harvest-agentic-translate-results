#[unsafe(no_mangle)]
pub extern "C" fn half2float(h: u16) -> f32 {
    let sign = ((h as u32) & 0x8000) << 16;
    let exponent = (h >> 10) & 0x1f;
    let mantissa = (h & 0x03ff) as u32;

    let bits = if exponent == 0 {
        if mantissa == 0 {
            sign
        } else {
            let mut normalized_mantissa = mantissa;
            let mut exponent_adjust = -14i32;

            while (normalized_mantissa & 0x0400) == 0 {
                normalized_mantissa <<= 1;
                exponent_adjust -= 1;
            }

            normalized_mantissa &= 0x03ff;
            sign
                | (((exponent_adjust + 127) as u32) << 23)
                | (normalized_mantissa << 13)
        }
    } else if exponent == 0x1f {
        sign | 0x7f80_0000 | (mantissa << 13)
    } else {
        sign | (((exponent as u32) + 112) << 23) | (mantissa << 13)
    };

    f32::from_bits(bits)
}
