const TABLE_LEN: usize = 512;

const fn base_table() -> [u16; TABLE_LEN] {
    let mut table = [0; TABLE_LEN];
    let mut exponent = 0;

    while exponent < 256 {
        let unbiased = exponent as i32 - 127;
        let base = if unbiased < -24 {
            0
        } else if unbiased < -14 {
            0x0400 >> (-unbiased - 14)
        } else if unbiased <= 15 {
            (unbiased + 15) << 10
        } else {
            0x7c00
        } as u16;

        table[exponent] = base;
        table[exponent | 0x100] = base | 0x8000;
        exponent += 1;
    }

    table
}

const fn shift_table() -> [u8; TABLE_LEN] {
    let mut table = [0; TABLE_LEN];
    let mut exponent = 0;

    while exponent < 256 {
        let unbiased = exponent as i32 - 127;
        let shift = if unbiased < -24 {
            24
        } else if unbiased < -14 {
            -unbiased - 1
        } else if unbiased <= 15 {
            13
        } else if unbiased < 128 {
            24
        } else {
            13
        } as u8;

        table[exponent] = shift;
        table[exponent | 0x100] = shift;
        exponent += 1;
    }

    table
}

static BASE: [u16; TABLE_LEN] = base_table();
static SHIFT: [u8; TABLE_LEN] = shift_table();

#[unsafe(no_mangle)]
pub extern "C" fn float2half(flt: f32) -> u16 {
    let bits = flt.to_bits();
    let index = ((bits >> 23) & 0x1ff) as usize;

    BASE[index] + (((bits & 0x007f_ffff) >> SHIFT[index]) as u16)
}

#[cfg(test)]
mod tests {
    use super::float2half;

    #[test]
    fn representative_encodings_match_the_reference() {
        let cases = [
            (0x0000_0000, 0x0000),
            (0x8000_0000, 0x8000),
            (0x3380_0000, 0x0001),
            (0x387f_e000, 0x03ff),
            (0x3880_0000, 0x0400),
            (0x3f80_0000, 0x3c00),
            (0x477f_e000, 0x7bff),
            (0x4780_0000, 0x7c00),
            (0x7f80_0000, 0x7c00),
            (0x7fc0_0000, 0x7e00),
            (0xff80_0000, 0xfc00),
            (0xffff_ffff, 0xffff),
        ];

        for (input, expected) in cases {
            assert_eq!(float2half(f32::from_bits(input)), expected);
        }
    }
}
