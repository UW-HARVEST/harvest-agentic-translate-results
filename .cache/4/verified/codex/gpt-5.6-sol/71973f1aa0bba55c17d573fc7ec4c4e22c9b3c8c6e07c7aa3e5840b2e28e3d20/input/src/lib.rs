fn conversion_parameters(index: u32) -> (u32, u32) {
    let sign = (index & 0x100) << 7;
    let exponent = index & 0xff;

    match exponent {
        0..=102 => (sign, 24),
        103..=112 => (sign | (1 << (exponent - 103)), 126 - exponent),
        113..=142 => (sign | ((exponent - 112) << 10), 13),
        143..=254 => (sign | 0x7c00, 24),
        255 => (sign | 0x7c00, 13),
        _ => unreachable!(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn float2half(flt: f32) -> u16 {
    let bits = flt.to_bits();
    let index = (bits >> 23) & 0x1ff;
    let (base, shift) = conversion_parameters(index);

    (base + ((bits & 0x007f_ffff) >> shift)) as u16
}

#[cfg(test)]
mod tests {
    use super::float2half;

    #[test]
    fn converts_representative_values() {
        let cases = [
            (0x0000_0000, 0x0000),
            (0x8000_0000, 0x8000),
            (0x3380_0000, 0x0001),
            (0x387f_e000, 0x03ff),
            (0x3880_0000, 0x0400),
            (0x3f80_0000, 0x3c00),
            (0x4000_0000, 0x4000),
            (0x477f_e000, 0x7bff),
            (0x4780_0000, 0x7c00),
            (0x7f80_0000, 0x7c00),
            (0x7fc0_0000, 0x7e00),
            (0xff80_0000, 0xfc00),
            (0xffc0_0000, 0xfe00),
        ];

        for (input, expected) in cases {
            assert_eq!(float2half(f32::from_bits(input)), expected);
        }
    }
}
