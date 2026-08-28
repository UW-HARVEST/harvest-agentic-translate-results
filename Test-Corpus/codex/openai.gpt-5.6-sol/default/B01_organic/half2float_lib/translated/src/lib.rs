#[unsafe(no_mangle)]
pub extern "C" fn half2float(h: u16) -> f32 {
    let sign = u32::from(h & 0x8000) << 16;
    let exponent = (h >> 10) & 0x1f;
    let fraction = u32::from(h & 0x03ff);

    let bits = match exponent {
        0 if fraction == 0 => sign,
        0 => {
            let leading_zeros = fraction.leading_zeros() - 21;
            let normalized = fraction << leading_zeros;
            let exponent = 113 - leading_zeros;
            sign | (exponent << 23) | ((normalized & 0x03ff) << 13)
        }
        31 => sign | 0x7f80_0000 | (fraction << 13),
        _ => sign | (u32::from(exponent) + 112) << 23 | (fraction << 13),
    };

    f32::from_bits(bits)
}
