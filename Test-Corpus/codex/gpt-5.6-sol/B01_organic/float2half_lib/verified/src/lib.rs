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
