/// Converts an IEEE 754 binary16 bit pattern to binary32.
#[unsafe(no_mangle)]
pub extern "C" fn half2float(h: u16) -> f32 {
    let sign = u32::from(h & 0x8000) << 16;
    let exponent = (h >> 10) & 0x1f;
    let fraction = u32::from(h & 0x03ff);

    let bits = match exponent {
        0 if fraction == 0 => sign,
        0 => {
            let shift = fraction.leading_zeros() - 21;
            let normalized_fraction = (fraction << shift) & 0x03ff;
            let normalized_exponent = 113 - shift;
            sign | (normalized_exponent << 23) | (normalized_fraction << 13)
        }
        31 => sign | 0x7f80_0000 | (fraction << 13),
        _ => sign | (u32::from(exponent + 112) << 23) | (fraction << 13),
    };

    f32::from_bits(bits)
}
