use std::ffi::c_float;

fn table_entry(j: u32) -> (u16, u32) {
    let sign = if (j & 0x100) != 0 { 0x8000 } else { 0x0000 };
    let exponent = (j & 0xff) as i32 - 127;

    if exponent < -24 {
        (sign, 24)
    } else if exponent < -14 {
        (
            sign | (0x0400u16 >> (-exponent - 14) as u32),
            (-exponent - 1) as u32,
        )
    } else if exponent <= 15 {
        (sign | (((exponent + 15) as u16) << 10), 13)
    } else if exponent < 128 {
        (sign | 0x7c00, 24)
    } else {
        (sign | 0x7c00, 13)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn float2half(flt: c_float) -> u16 {
    let n = flt.to_bits();
    let j = (n >> 23) & 0x1ff;
    let (base, shift) = table_entry(j);

    (base as u32 + ((n & 0x007f_ffff) >> shift)) as u16
}
