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

#[cfg(test)]
mod tests {
    use super::half2float;

    fn reference_bits(h: u16) -> u32 {
        let sign = u32::from(h & 0x8000) << 16;
        let exponent = u32::from((h >> 10) & 0x1f);
        let fraction = u32::from(h & 0x03ff);

        if exponent == 0 {
            if fraction == 0 {
                return sign;
            }

            let mut mantissa = fraction;
            let mut output_exponent = 113_u32;
            while mantissa & 0x0400 == 0 {
                mantissa <<= 1;
                output_exponent -= 1;
            }
            sign | (output_exponent << 23) | ((mantissa & 0x03ff) << 13)
        } else if exponent == 31 {
            sign | 0x7f80_0000 | (fraction << 13)
        } else {
            sign | ((exponent + 112) << 23) | (fraction << 13)
        }
    }

    #[test]
    fn matches_reference_for_every_input() {
        for h in u16::MIN..=u16::MAX {
            assert_eq!(half2float(h).to_bits(), reference_bits(h), "input {h:#06x}");
        }
    }
}
