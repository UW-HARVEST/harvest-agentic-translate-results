pub type Utf8 = u8;
pub type Utf16 = u16;
pub type Utf32 = u32;
pub type Latin1 = u8;
pub type Ascii = u8;

#[inline]
fn read_utf16le(word: Utf16) -> u16 {
    u16::from_le(word)
}

#[inline]
fn write_utf16le(word: u16) -> Utf16 {
    word.to_le()
}

pub fn utf8_validate(data: &[Utf8]) -> bool {
    let mut pos = 0usize;

    while pos < data.len() {
        if pos + 16 <= data.len() && data[pos..pos + 16].iter().all(|&b| b < 0x80) {
            pos += 16;
            continue;
        }

        let word = data[pos];
        if word < 0x80 {
            pos += 1;
            while pos < data.len() && data[pos] < 0x80 {
                pos += 1;
            }
            continue;
        }

        if (word & 0b1110_0000) == 0b1100_0000 {
            let next_pos = pos + 2;
            if next_pos > data.len() {
                return false;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return false;
            }
            let code_point =
                (u32::from(word & 0b0001_1111) << 6) | u32::from(data[pos + 1] & 0b0011_1111);
            if code_point < 0x80 || code_point > 0x7ff {
                return false;
            }
            pos = next_pos;
        } else if (word & 0b1111_0000) == 0b1110_0000 {
            let next_pos = pos + 3;
            if next_pos > data.len() {
                return false;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000
                || (data[pos + 2] & 0b1100_0000) != 0b1000_0000
            {
                return false;
            }
            let code_point = (u32::from(word & 0b0000_1111) << 12)
                | (u32::from(data[pos + 1] & 0b0011_1111) << 6)
                | u32::from(data[pos + 2] & 0b0011_1111);
            if code_point < 0x800
                || code_point > 0xffff
                || (0xd800..0xe000).contains(&code_point)
            {
                return false;
            }
            pos = next_pos;
        } else if (word & 0b1111_1000) == 0b1111_0000 {
            let next_pos = pos + 4;
            if next_pos > data.len() {
                return false;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000
                || (data[pos + 2] & 0b1100_0000) != 0b1000_0000
                || (data[pos + 3] & 0b1100_0000) != 0b1000_0000
            {
                return false;
            }
            let code_point = (u32::from(word & 0b0000_0111) << 18)
                | (u32::from(data[pos + 1] & 0b0011_1111) << 12)
                | (u32::from(data[pos + 2] & 0b0011_1111) << 6)
                | u32::from(data[pos + 3] & 0b0011_1111);
            if code_point <= 0xffff || code_point > 0x10ffff {
                return false;
            }
            pos = next_pos;
        } else {
            return false;
        }
    }

    true
}

pub fn utf8_length_from_utf16le(data: &[Utf16]) -> usize {
    let mut counter = 0usize;

    for &word in data {
        let word = read_utf16le(word);
        if word <= 0x7f {
            counter += 1;
        } else if word <= 0x7ff {
            counter += 2;
        } else if word <= 0xd7ff || word >= 0xe000 {
            counter += 3;
        } else {
            counter += 2;
        }
    }

    counter
}

pub fn utf8_length_from_utf32(data: &[Utf32]) -> usize {
    data.iter()
        .map(|&word| {
            1usize
                + usize::from(word > 0x7f)
                + usize::from(word > 0x7ff)
                + usize::from(word > 0xffff)
        })
        .sum()
}

pub fn utf8_length_from_latin1(data: &[Latin1]) -> usize {
    data.iter().map(|&b| 1usize + usize::from((b >> 7) != 0)).sum()
}

pub fn utf8_convert_to_utf16le(data: &[Utf8], result: &mut [Utf16]) -> usize {
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < data.len() {
        if pos + 8 <= data.len() && data[pos..pos + 8].iter().all(|&b| b < 0x80) {
            for &b in &data[pos..pos + 8] {
                if out >= result.len() {
                    return 0;
                }
                result[out] = write_utf16le(u16::from(b));
                out += 1;
            }
            pos += 8;
            continue;
        }

        let leading = data[pos];
        if leading < 0x80 {
            if out >= result.len() {
                return 0;
            }
            result[out] = write_utf16le(u16::from(leading));
            out += 1;
            pos += 1;
        } else if (leading & 0b1110_0000) == 0b1100_0000 {
            if pos + 1 >= data.len() {
                break;
            }
            if out >= result.len() {
                return 0;
            }
            let code_point = (u16::from(leading & 0b0001_1111) << 6)
                | u16::from(data[pos + 1] & 0b0011_1111);
            result[out] = write_utf16le(code_point);
            out += 1;
            pos += 2;
        } else if (leading & 0b1111_0000) == 0b1110_0000 {
            if pos + 2 >= data.len() {
                break;
            }
            if out >= result.len() {
                return 0;
            }
            let code_point = (u16::from(leading & 0b0000_1111) << 12)
                | (u16::from(data[pos + 1] & 0b0011_1111) << 6)
                | u16::from(data[pos + 2] & 0b0011_1111);
            result[out] = write_utf16le(code_point);
            out += 1;
            pos += 3;
        } else if (leading & 0b1111_1000) == 0b1111_0000 {
            if pos + 3 >= data.len() {
                break;
            }
            if out + 1 >= result.len() {
                return 0;
            }
            let mut code_point = (u32::from(leading & 0b0000_0111) << 18)
                | (u32::from(data[pos + 1] & 0b0011_1111) << 12)
                | (u32::from(data[pos + 2] & 0b0011_1111) << 6)
                | u32::from(data[pos + 3] & 0b0011_1111);
            code_point = code_point.wrapping_sub(0x10000);
            let high_surrogate = 0xd800u16 + ((code_point >> 10) as u16);
            let low_surrogate = 0xdc00u16 + ((code_point & 0x3ff) as u16);
            result[out] = write_utf16le(high_surrogate);
            result[out + 1] = write_utf16le(low_surrogate);
            out += 2;
            pos += 4;
        } else {
            return 0;
        }
    }

    out
}

pub fn utf8_convert_to_utf32(data: &[Utf8], result: &mut [Utf32]) -> usize {
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < data.len() {
        if pos + 16 <= data.len() && data[pos..pos + 16].iter().all(|&b| b < 0x80) {
            for &b in &data[pos..pos + 16] {
                if out >= result.len() {
                    return 0;
                }
                result[out] = u32::from(b);
                out += 1;
            }
            pos += 16;
            continue;
        }

        let leading = data[pos];
        if leading < 0x80 {
            if out >= result.len() {
                return 0;
            }
            result[out] = u32::from(leading);
            out += 1;
            pos += 1;
        } else if (leading & 0b1110_0000) == 0b1100_0000 {
            if pos + 1 >= data.len() {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            let code_point = (u32::from(leading & 0b0001_1111) << 6)
                | u32::from(data[pos + 1] & 0b0011_1111);
            if code_point < 0x80 || code_point > 0x7ff {
                return 0;
            }
            if out >= result.len() {
                return 0;
            }
            result[out] = code_point;
            out += 1;
            pos += 2;
        } else if (leading & 0b1111_0000) == 0b1110_0000 {
            if pos + 2 >= data.len() {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000
                || (data[pos + 2] & 0b1100_0000) != 0b1000_0000
            {
                return 0;
            }
            let code_point = (u32::from(leading & 0b0000_1111) << 12)
                | (u32::from(data[pos + 1] & 0b0011_1111) << 6)
                | u32::from(data[pos + 2] & 0b0011_1111);
            if code_point < 0x800
                || code_point > 0xffff
                || (0xd800..0xe000).contains(&code_point)
            {
                return 0;
            }
            if out >= result.len() {
                return 0;
            }
            result[out] = code_point;
            out += 1;
            pos += 3;
        } else if (leading & 0b1111_1000) == 0b1111_0000 {
            if pos + 3 >= data.len() {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000
                || (data[pos + 2] & 0b1100_0000) != 0b1000_0000
                || (data[pos + 3] & 0b1100_0000) != 0b1000_0000
            {
                return 0;
            }
            let code_point = (u32::from(leading & 0b0000_0111) << 18)
                | (u32::from(data[pos + 1] & 0b0011_1111) << 12)
                | (u32::from(data[pos + 2] & 0b0011_1111) << 6)
                | u32::from(data[pos + 3] & 0b0011_1111);
            if code_point <= 0xffff || code_point > 0x10ffff {
                return 0;
            }
            if out >= result.len() {
                return 0;
            }
            result[out] = code_point;
            out += 1;
            pos += 4;
        } else {
            return 0;
        }
    }

    out
}

pub fn utf8_convert_to_latin1(data: &[Utf8], result: &mut [Latin1]) -> usize {
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < data.len() {
        if pos + 16 <= data.len() && data[pos..pos + 16].iter().all(|&b| b < 0x80) {
            if out + 16 > result.len() {
                return 0;
            }
            result[out..out + 16].copy_from_slice(&data[pos..pos + 16]);
            out += 16;
            pos += 16;
            continue;
        }

        let leading = data[pos];
        if leading < 0x80 {
            if out >= result.len() {
                return 0;
            }
            result[out] = leading;
            out += 1;
            pos += 1;
        } else if (leading & 0b1110_0000) == 0b1100_0000 {
            if pos + 1 >= data.len() {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            let code_point = (u32::from(leading & 0b0001_1111) << 6)
                | u32::from(data[pos + 1] & 0b0011_1111);
            if code_point < 0x80 || code_point > 0xff {
                return 0;
            }
            if out >= result.len() {
                return 0;
            }
            result[out] = code_point as u8;
            out += 1;
            pos += 2;
        } else {
            return 0;
        }
    }

    out
}

pub fn utf16le_validate(data: &[Utf16]) -> bool {
    let mut pos = 0usize;

    while pos < data.len() {
        let word = read_utf16le(data[pos]);
        if (word & 0xf800) == 0xd800 {
            if pos + 1 >= data.len() {
                return false;
            }
            let diff = word.wrapping_sub(0xd800);
            if diff > 0x03ff {
                return false;
            }
            let next = read_utf16le(data[pos + 1]);
            let diff = next.wrapping_sub(0xdc00);
            if diff > 0x03ff {
                return false;
            }
            pos += 2;
        } else {
            pos += 1;
        }
    }

    true
}

pub fn utf16_length_from_utf8(data: &[Utf8]) -> usize {
    let mut counter = 0usize;

    for &byte in data {
        if (byte as i8) > -65 {
            counter += 1;
        }
        if byte >= 240 {
            counter += 1;
        }
    }

    counter
}

pub fn utf16_length_from_utf32(data: &[Utf32]) -> usize {
    data.iter()
        .map(|&word| 1usize + usize::from(word > 0xffff))
        .sum()
}

pub fn utf16_length_from_latin1(data: &[Latin1]) -> usize {
    data.len()
}

pub fn utf16le_convert_to_utf8(data: &[Utf16], result: &mut [Utf8]) -> usize {
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < data.len() {
        if pos + 4 <= data.len()
            && data[pos..pos + 4]
                .iter()
                .copied()
                .map(read_utf16le)
                .all(|word| (word & 0xff80) == 0)
        {
            if out + 4 > result.len() {
                return 0;
            }
            for word in data[pos..pos + 4].iter().copied().map(read_utf16le) {
                result[out] = word as u8;
                out += 1;
            }
            pos += 4;
            continue;
        }

        let word = read_utf16le(data[pos]);
        if (word & 0xff80) == 0 {
            if out >= result.len() {
                return 0;
            }
            result[out] = word as u8;
            out += 1;
            pos += 1;
        } else if (word & 0xf800) == 0 {
            if out + 1 >= result.len() {
                return 0;
            }
            result[out] = ((word >> 6) as u8) | 0b1100_0000;
            result[out + 1] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            out += 2;
            pos += 1;
        } else if (word & 0xf800) != 0xd800 {
            if out + 2 >= result.len() {
                return 0;
            }
            result[out] = ((word >> 12) as u8) | 0b1110_0000;
            result[out + 1] = (((word >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            result[out + 2] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            out += 3;
            pos += 1;
        } else {
            let diff = word.wrapping_sub(0xd800);
            if pos + 1 >= data.len() {
                return 0;
            }
            let word2 = read_utf16le(data[pos + 1]);
            let value = (u32::from(diff) << 10)
                .wrapping_add(u32::from(word2.wrapping_sub(0xdc00)))
                .wrapping_add(0x10000);
            if out + 3 >= result.len() {
                return 0;
            }
            result[out] = ((value >> 18) as u8) | 0b1111_0000;
            result[out + 1] = (((value >> 12) & 0b11_1111) as u8) | 0b1000_0000;
            result[out + 2] = (((value >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            result[out + 3] = ((value & 0b11_1111) as u8) | 0b1000_0000;
            out += 4;
            pos += 2;
        }
    }

    out
}

pub fn utf16le_convert_to_utf32(data: &[Utf16], result: &mut [Utf32]) -> usize {
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < data.len() {
        let word = read_utf16le(data[pos]);
        if (word & 0xf800) != 0xd800 {
            if out >= result.len() {
                return 0;
            }
            result[out] = u32::from(word);
            out += 1;
            pos += 1;
        } else {
            let diff = word.wrapping_sub(0xd800);
            if diff > 0x03ff || pos + 1 >= data.len() {
                return 0;
            }
            let word2 = read_utf16le(data[pos + 1]);
            let value = (u32::from(diff) << 10)
                .wrapping_add(u32::from(word2.wrapping_sub(0xdc00)))
                .wrapping_add(0x10000);
            let diff2 = word2.wrapping_sub(0xdc00);
            if diff2 > 0x03ff {
                return 0;
            }
            if out >= result.len() {
                return 0;
            }
            result[out] = value;
            out += 1;
            pos += 2;
        }
    }

    out
}

pub fn utf16le_convert_to_latin1(data: &[Utf16], result: &mut [Latin1]) -> usize {
    let mut overflow = 0u16;
    let mut out = 0usize;

    for &word in data {
        if out >= result.len() {
            return 0;
        }
        let word = read_utf16le(word);
        overflow |= word;
        result[out] = (word & 0xff) as u8;
        out += 1;
    }

    if (overflow & 0xff00) != 0 {
        return 0;
    }

    out
}

pub fn utf32_validate(data: &[Utf32]) -> bool {
    data.iter()
        .all(|&word| word <= 0x10ffff && !(0xd800..=0xdfff).contains(&word))
}

pub fn utf32_length_from_utf8(data: &[Utf8]) -> usize {
    data.iter().filter(|&&b| (b as i8) > -65).count()
}

pub fn utf32_length_from_utf16le(data: &[Utf16]) -> usize {
    data.iter()
        .copied()
        .map(read_utf16le)
        .filter(|&word| (word & 0xfc00) != 0xdc00)
        .count()
}

pub fn utf32_length_from_latin1(data: &[Latin1]) -> usize {
    data.len()
}

pub fn utf32_convert_to_utf8(data: &[Utf32], result: &mut [Utf8]) -> usize {
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < data.len() {
        if pos + 2 <= data.len() && data[pos] <= 0x7f && data[pos + 1] <= 0x7f {
            if out + 2 > result.len() {
                return 0;
            }
            result[out] = data[pos] as u8;
            result[out + 1] = data[pos + 1] as u8;
            out += 2;
            pos += 2;
            continue;
        }

        let word = data[pos];
        if (word & 0xffffff80) == 0 {
            if out >= result.len() {
                return 0;
            }
            result[out] = word as u8;
            out += 1;
            pos += 1;
        } else if (word & 0xfffff800) == 0 {
            if out + 1 >= result.len() {
                return 0;
            }
            result[out] = ((word >> 6) as u8) | 0b1100_0000;
            result[out + 1] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            out += 2;
            pos += 1;
        } else if (word & 0xffff0000) == 0 {
            if (0xd800..=0xdfff).contains(&word) {
                return 0;
            }
            if out + 2 >= result.len() {
                return 0;
            }
            result[out] = ((word >> 12) as u8) | 0b1110_0000;
            result[out + 1] = (((word >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            result[out + 2] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            out += 3;
            pos += 1;
        } else {
            if word > 0x10ffff {
                return 0;
            }
            if out + 3 >= result.len() {
                return 0;
            }
            result[out] = ((word >> 18) as u8) | 0b1111_0000;
            result[out + 1] = (((word >> 12) & 0b11_1111) as u8) | 0b1000_0000;
            result[out + 2] = (((word >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            result[out + 3] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            out += 4;
            pos += 1;
        }
    }

    out
}

pub fn utf32_convert_to_utf16le(data: &[Utf32], result: &mut [Utf16]) -> usize {
    let mut out = 0usize;

    for &word in data {
        if (word & 0xffff0000) == 0 {
            if (0xd800..=0xdfff).contains(&word) {
                return 0;
            }
            if out >= result.len() {
                return 0;
            }
            result[out] = write_utf16le(word as u16);
            out += 1;
        } else {
            if word > 0x10ffff || out + 1 >= result.len() {
                return 0;
            }
            let word = word - 0x10000;
            let high_surrogate = 0xd800u16 + ((word >> 10) as u16);
            let low_surrogate = 0xdc00u16 + ((word & 0x3ff) as u16);
            result[out] = write_utf16le(high_surrogate);
            result[out + 1] = write_utf16le(low_surrogate);
            out += 2;
        }
    }

    out
}

pub fn utf32_convert_to_latin1(data: &[Utf32], result: &mut [Latin1]) -> usize {
    let mut overflow = 0u32;
    let mut out = 0usize;

    for &word in data {
        if out >= result.len() {
            return 0;
        }
        overflow |= word;
        result[out] = (word & 0xff) as u8;
        out += 1;
    }

    if (overflow & 0xffff_ff00) != 0 {
        return 0;
    }

    out
}

pub fn latin1_length_from_utf8(data: &[Utf8]) -> usize {
    data.iter().filter(|&&b| (b as i8) > -65).count()
}

pub fn latin1_length_from_utf16le(data: &[Utf16]) -> usize {
    data.len()
}

pub fn latin1_length_from_utf32(data: &[Utf32]) -> usize {
    data.len()
}

pub fn latin1_convert_to_utf8(data: &[Latin1], result: &mut [Utf8]) -> usize {
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < data.len() {
        if pos + 16 <= data.len() && data[pos..pos + 16].iter().all(|&b| b < 0x80) {
            if out + 16 > result.len() {
                return 0;
            }
            result[out..out + 16].copy_from_slice(&data[pos..pos + 16]);
            out += 16;
            pos += 16;
            continue;
        }

        let byte = data[pos];
        if (byte & 0x80) == 0 {
            if out >= result.len() {
                return 0;
            }
            result[out] = byte;
            out += 1;
        } else {
            if out + 1 >= result.len() {
                return 0;
            }
            result[out] = (byte >> 6) | 0b1100_0000;
            result[out + 1] = (byte & 0b11_1111) | 0b1000_0000;
            out += 2;
        }
        pos += 1;
    }

    out
}

pub fn latin1_convert_to_utf16le(data: &[Latin1], result: &mut [Utf16]) -> usize {
    if result.len() < data.len() {
        return 0;
    }

    for (idx, &byte) in data.iter().enumerate() {
        result[idx] = write_utf16le(u16::from(byte));
    }

    data.len()
}

pub fn latin1_convert_to_utf32(data: &[Latin1], result: &mut [Utf32]) -> usize {
    if result.len() < data.len() {
        return 0;
    }

    for (idx, &byte) in data.iter().enumerate() {
        result[idx] = u32::from(byte);
    }

    data.len()
}

pub fn ascii_validate(data: &[Ascii]) -> bool {
    data.iter().all(|&b| b < 0x80)
}
