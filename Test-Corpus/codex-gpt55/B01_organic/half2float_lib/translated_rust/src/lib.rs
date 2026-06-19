fn half_to_float_bits(h: u16) -> u32 {
    let sign = ((h & 0x8000) as u32) << 16;
    let exponent = (h >> 10) & 0x1f;
    let mantissa = h & 0x03ff;

    if exponent == 0 {
        if mantissa == 0 {
            sign
        } else {
            let mut normalized = mantissa as u32;
            let mut exponent_value = -14i32;

            while (normalized & 0x0400) == 0 {
                normalized <<= 1;
                exponent_value -= 1;
            }

            normalized &= 0x03ff;
            sign | (((exponent_value + 127) as u32) << 23) | (normalized << 13)
        }
    } else if exponent == 0x1f {
        sign | 0x7f80_0000 | ((mantissa as u32) << 13)
    } else {
        sign | (((exponent as u32) + 112) << 23) | ((mantissa as u32) << 13)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn half2float(h: u16) -> f32 {
    f32::from_bits(half_to_float_bits(h))
}

#[cfg(test)]
mod tests {
    use super::half_to_float_bits;

    #[test]
    fn converts_representative_values() {
        assert_eq!(half_to_float_bits(0x0000), 0x0000_0000);
        assert_eq!(half_to_float_bits(0x8000), 0x8000_0000);
        assert_eq!(half_to_float_bits(0x0001), 0x3380_0000);
        assert_eq!(half_to_float_bits(0x03ff), 0x387f_c000);
        assert_eq!(half_to_float_bits(0x0400), 0x3880_0000);
        assert_eq!(half_to_float_bits(0x3c00), 0x3f80_0000);
        assert_eq!(half_to_float_bits(0x7c00), 0x7f80_0000);
        assert_eq!(half_to_float_bits(0x7e00), 0x7fc0_0000);
        assert_eq!(half_to_float_bits(0xfc00), 0xff80_0000);
    }
}
