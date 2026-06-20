pub type Utf8 = u8;
pub type Utf16 = u16;
pub type Utf32 = u32;
pub type Latin1 = u8;
pub type Ascii = u8;

#[inline]
fn is_be() -> bool {
    cfg!(target_endian = "big")
}

#[inline]
fn read_utf16le(word: Utf16) -> Utf16 {
    if is_be() {
        word.swap_bytes()
    } else {
        word
    }
}

#[inline]
fn write_utf16le(word: Utf16) -> Utf16 {
    if is_be() {
        word.swap_bytes()
    } else {
        word
    }
}

#[inline]
fn non_continuation(byte: u8) -> bool {
    (byte as i8) > -65
}

pub fn utf8_validate(data: &[Utf8]) -> bool {
    let mut pos = 0;

    while pos < data.len() {
        while pos < data.len() && data[pos] < 0x80 {
            pos += 1;
        }
        if pos == data.len() {
            return true;
        }

        let word = data[pos];
        let next_pos;

        if (word & 0b1110_0000) == 0b1100_0000 {
            next_pos = pos + 2;
            if next_pos > data.len() {
                return false;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return false;
            }
            let code_point =
                (((word & 0b0001_1111) as u32) << 6) | ((data[pos + 1] & 0b0011_1111) as u32);
            if !(0x80..=0x7ff).contains(&code_point) {
                return false;
            }
        } else if (word & 0b1111_0000) == 0b1110_0000 {
            next_pos = pos + 3;
            if next_pos > data.len() {
                return false;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return false;
            }
            if (data[pos + 2] & 0b1100_0000) != 0b1000_0000 {
                return false;
            }
            let code_point = (((word & 0b0000_1111) as u32) << 12)
                | (((data[pos + 1] & 0b0011_1111) as u32) << 6)
                | ((data[pos + 2] & 0b0011_1111) as u32);
            if code_point < 0x800
                || code_point > 0xffff
                || (0xd800..0xe000).contains(&code_point)
            {
                return false;
            }
        } else if (word & 0b1111_1000) == 0b1111_0000 {
            next_pos = pos + 4;
            if next_pos > data.len() {
                return false;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return false;
            }
            if (data[pos + 2] & 0b1100_0000) != 0b1000_0000 {
                return false;
            }
            if (data[pos + 3] & 0b1100_0000) != 0b1000_0000 {
                return false;
            }
            let code_point = (((word & 0b0000_0111) as u32) << 18)
                | (((data[pos + 1] & 0b0011_1111) as u32) << 12)
                | (((data[pos + 2] & 0b0011_1111) as u32) << 6)
                | ((data[pos + 3] & 0b0011_1111) as u32);
            if code_point <= 0xffff || code_point > 0x10ffff {
                return false;
            }
        } else {
            return false;
        }

        pos = next_pos;
    }

    true
}

pub fn utf8_length_from_utf16le(data: &[Utf16]) -> usize {
    let mut counter = 0;

    for &raw in data {
        let word = read_utf16le(raw);
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
    let mut counter = 0;

    for &word in data {
        counter += 1;
        counter += usize::from(word > 0x7f);
        counter += usize::from(word > 0x7ff);
        counter += usize::from(word > 0xffff);
    }

    counter
}

pub fn utf8_length_from_latin1(data: &[Latin1]) -> usize {
    data.len() + data.iter().map(|&b| usize::from(b >> 7)).sum::<usize>()
}

pub fn utf8_convert_to_utf16le(data: &[Utf8], result: &mut [Utf16]) -> usize {
    let mut pos = 0;
    let mut written = 0;

    while pos < data.len() {
        let leading_byte = data[pos];
        if leading_byte < 0b1000_0000 {
            if written >= result.len() {
                return 0;
            }
            result[written] = write_utf16le(leading_byte as u16);
            written += 1;
            pos += 1;
        } else if (leading_byte & 0b1110_0000) == 0b1100_0000 {
            if pos + 1 >= data.len() {
                break;
            }
            if written >= result.len() {
                return 0;
            }
            let code_point = (((leading_byte & 0b0001_1111) as u16) << 6)
                | ((data[pos + 1] & 0b0011_1111) as u16);
            result[written] = write_utf16le(code_point);
            written += 1;
            pos += 2;
        } else if (leading_byte & 0b1111_0000) == 0b1110_0000 {
            if pos + 2 >= data.len() {
                break;
            }
            if written >= result.len() {
                return 0;
            }
            let code_point = (((leading_byte & 0b0000_1111) as u16) << 12)
                | (((data[pos + 1] & 0b0011_1111) as u16) << 6)
                | ((data[pos + 2] & 0b0011_1111) as u16);
            result[written] = write_utf16le(code_point);
            written += 1;
            pos += 3;
        } else if (leading_byte & 0b1111_1000) == 0b1111_0000 {
            if pos + 3 >= data.len() {
                break;
            }
            if written + 1 >= result.len() {
                return 0;
            }
            let mut code_point = (((leading_byte & 0b0000_0111) as u32) << 18)
                | (((data[pos + 1] & 0b0011_1111) as u32) << 12)
                | (((data[pos + 2] & 0b0011_1111) as u32) << 6)
                | ((data[pos + 3] & 0b0011_1111) as u32);
            code_point = code_point.wrapping_sub(0x10000);
            let high_surrogate = 0xd800 + ((code_point >> 10) as u16);
            let low_surrogate = 0xdc00 + ((code_point & 0x3ff) as u16);
            result[written] = write_utf16le(high_surrogate);
            result[written + 1] = write_utf16le(low_surrogate);
            written += 2;
            pos += 4;
        } else {
            return 0;
        }
    }

    written
}

pub fn utf8_convert_to_utf32(data: &[Utf8], result: &mut [Utf32]) -> usize {
    let mut pos = 0;
    let mut written = 0;

    while pos < data.len() {
        if written >= result.len() {
            return 0;
        }

        let leading_byte = data[pos];
        if leading_byte < 0b1000_0000 {
            result[written] = leading_byte as u32;
            written += 1;
            pos += 1;
        } else if (leading_byte & 0b1110_0000) == 0b1100_0000 {
            if pos + 1 >= data.len() {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            let code_point = (((leading_byte & 0b0001_1111) as u32) << 6)
                | ((data[pos + 1] & 0b0011_1111) as u32);
            if !(0x80..=0x7ff).contains(&code_point) {
                return 0;
            }
            result[written] = code_point;
            written += 1;
            pos += 2;
        } else if (leading_byte & 0b1111_0000) == 0b1110_0000 {
            if pos + 2 >= data.len() {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            if (data[pos + 2] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            let code_point = (((leading_byte & 0b0000_1111) as u32) << 12)
                | (((data[pos + 1] & 0b0011_1111) as u32) << 6)
                | ((data[pos + 2] & 0b0011_1111) as u32);
            if code_point < 0x800
                || code_point > 0xffff
                || (0xd800..0xe000).contains(&code_point)
            {
                return 0;
            }
            result[written] = code_point;
            written += 1;
            pos += 3;
        } else if (leading_byte & 0b1111_1000) == 0b1111_0000 {
            if pos + 3 >= data.len() {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            if (data[pos + 2] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            if (data[pos + 3] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            let code_point = (((leading_byte & 0b0000_0111) as u32) << 18)
                | (((data[pos + 1] & 0b0011_1111) as u32) << 12)
                | (((data[pos + 2] & 0b0011_1111) as u32) << 6)
                | ((data[pos + 3] & 0b0011_1111) as u32);
            if code_point <= 0xffff || code_point > 0x10ffff {
                return 0;
            }
            result[written] = code_point;
            written += 1;
            pos += 4;
        } else {
            return 0;
        }
    }

    written
}

pub fn utf8_convert_to_latin1(data: &[Utf8], result: &mut [Latin1]) -> usize {
    let mut pos = 0;
    let mut written = 0;

    while pos < data.len() {
        if written >= result.len() {
            return 0;
        }

        let leading_byte = data[pos];
        if leading_byte < 0b1000_0000 {
            result[written] = leading_byte;
            written += 1;
            pos += 1;
        } else if (leading_byte & 0b1110_0000) == 0b1100_0000 {
            if pos + 1 >= data.len() {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            let code_point = (((leading_byte & 0b0001_1111) as u32) << 6)
                | ((data[pos + 1] & 0b0011_1111) as u32);
            if code_point < 0x80 || code_point > 0xff {
                return 0;
            }
            result[written] = code_point as u8;
            written += 1;
            pos += 2;
        } else {
            return 0;
        }
    }

    written
}

pub fn utf16le_validate(data: &[Utf16]) -> bool {
    let mut pos = 0;

    while pos < data.len() {
        let word = read_utf16le(data[pos]);
        if (word & 0xf800) == 0xd800 {
            if pos + 1 >= data.len() {
                return false;
            }
            let diff = word.wrapping_sub(0xd800);
            if diff > 0x3ff {
                return false;
            }
            let next_word = read_utf16le(data[pos + 1]);
            let next_diff = next_word.wrapping_sub(0xdc00);
            if next_diff > 0x3ff {
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
    let mut counter = 0;

    for &byte in data {
        if non_continuation(byte) {
            counter += 1;
        }
        if byte >= 240 {
            counter += 1;
        }
    }

    counter
}

pub fn utf16_length_from_utf32(data: &[Utf32]) -> usize {
    let mut counter = 0;

    for &word in data {
        counter += 1;
        counter += usize::from(word > 0xffff);
    }

    counter
}

pub fn utf16_length_from_latin1(data: &[Latin1]) -> usize {
    data.len()
}

pub fn utf16le_convert_to_utf8(data: &[Utf16], result: &mut [Utf8]) -> usize {
    let mut pos = 0;
    let mut written = 0;

    while pos < data.len() {
        let word = read_utf16le(data[pos]);
        if (word & 0xff80) == 0 {
            if written >= result.len() {
                return 0;
            }
            result[written] = word as u8;
            written += 1;
            pos += 1;
        } else if (word & 0xf800) == 0 {
            if written + 1 >= result.len() {
                return 0;
            }
            result[written] = ((word >> 6) as u8) | 0b1100_0000;
            result[written + 1] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            written += 2;
            pos += 1;
        } else if (word & 0xf800) != 0xd800 {
            if written + 2 >= result.len() {
                return 0;
            }
            result[written] = ((word >> 12) as u8) | 0b1110_0000;
            result[written + 1] = (((word >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            result[written + 2] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            written += 3;
            pos += 1;
        } else {
            if pos + 1 >= data.len() || written + 3 >= result.len() {
                return 0;
            }
            let diff = word.wrapping_sub(0xd800);
            let next_word = read_utf16le(data[pos + 1]);
            let value = (((diff as u32) << 10)
                .wrapping_add((next_word as u32).wrapping_sub(0xdc00)))
            .wrapping_add(0x10000);
            result[written] = ((value >> 18) as u8) | 0b1111_0000;
            result[written + 1] = (((value >> 12) & 0b11_1111) as u8) | 0b1000_0000;
            result[written + 2] = (((value >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            result[written + 3] = ((value & 0b11_1111) as u8) | 0b1000_0000;
            written += 4;
            pos += 2;
        }
    }

    written
}

pub fn utf16le_convert_to_utf32(data: &[Utf16], result: &mut [Utf32]) -> usize {
    let mut pos = 0;
    let mut written = 0;

    while pos < data.len() {
        if written >= result.len() {
            return 0;
        }

        let word = read_utf16le(data[pos]);
        if (word & 0xf800) != 0xd800 {
            result[written] = word as u32;
            written += 1;
            pos += 1;
        } else {
            let diff = word.wrapping_sub(0xd800);
            if diff > 0x3ff {
                return 0;
            }
            if pos + 1 >= data.len() {
                return 0;
            }
            let next_word = read_utf16le(data[pos + 1]);
            let value = (((diff as u32) << 10)
                .wrapping_add((next_word as u32).wrapping_sub(0xdc00)))
            .wrapping_add(0x10000);
            let next_diff = next_word.wrapping_sub(0xdc00);
            if next_diff > 0x3ff {
                return 0;
            }
            result[written] = value;
            written += 1;
            pos += 2;
        }
    }

    written
}

pub fn utf16le_convert_to_latin1(data: &[Utf16], result: &mut [Latin1]) -> usize {
    let mut overflow = 0u16;
    let mut written = 0;

    for &raw in data {
        if written >= result.len() {
            return 0;
        }
        let word = read_utf16le(raw);
        overflow |= word;
        result[written] = (word & 0xff) as u8;
        written += 1;
    }

    if (overflow & 0xff00) != 0 {
        return 0;
    }

    written
}

pub fn utf32_validate(data: &[Utf32]) -> bool {
    data.iter()
        .all(|&word| word <= 0x10ffff && !(0xd800..=0xdfff).contains(&word))
}

pub fn utf32_length_from_utf8(data: &[Utf8]) -> usize {
    data.iter().filter(|&&byte| non_continuation(byte)).count()
}

pub fn utf32_length_from_utf16le(data: &[Utf16]) -> usize {
    data.iter()
        .map(|&raw| usize::from((read_utf16le(raw) & 0xfc00) != 0xdc00))
        .sum()
}

pub fn utf32_length_from_latin1(data: &[Latin1]) -> usize {
    data.len()
}

pub fn utf32_convert_to_utf8(data: &[Utf32], result: &mut [Utf8]) -> usize {
    let mut written = 0;

    for &word in data {
        if (word & 0xffffff80) == 0 {
            if written >= result.len() {
                return 0;
            }
            result[written] = word as u8;
            written += 1;
        } else if (word & 0xfffff800) == 0 {
            if written + 1 >= result.len() {
                return 0;
            }
            result[written] = ((word >> 6) as u8) | 0b1100_0000;
            result[written + 1] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            written += 2;
        } else if (word & 0xffff0000) == 0 {
            if (0xd800..=0xdfff).contains(&word) || written + 2 >= result.len() {
                return 0;
            }
            result[written] = ((word >> 12) as u8) | 0b1110_0000;
            result[written + 1] = (((word >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            result[written + 2] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            written += 3;
        } else {
            if word > 0x10ffff || written + 3 >= result.len() {
                return 0;
            }
            result[written] = ((word >> 18) as u8) | 0b1111_0000;
            result[written + 1] = (((word >> 12) & 0b11_1111) as u8) | 0b1000_0000;
            result[written + 2] = (((word >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            result[written + 3] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            written += 4;
        }
    }

    written
}

pub fn utf32_convert_to_utf16le(data: &[Utf32], result: &mut [Utf16]) -> usize {
    let mut written = 0;

    for &mut_word in data {
        let mut word = mut_word;
        if (word & 0xffff0000) == 0 {
            if (0xd800..=0xdfff).contains(&word) || written >= result.len() {
                return 0;
            }
            result[written] = write_utf16le(word as u16);
            written += 1;
        } else {
            if word > 0x10ffff || written + 1 >= result.len() {
                return 0;
            }
            word -= 0x10000;
            let high_surrogate = 0xd800 + ((word >> 10) as u16);
            let low_surrogate = 0xdc00 + ((word & 0x3ff) as u16);
            result[written] = write_utf16le(high_surrogate);
            result[written + 1] = write_utf16le(low_surrogate);
            written += 2;
        }
    }

    written
}

pub fn utf32_convert_to_latin1(data: &[Utf32], result: &mut [Latin1]) -> usize {
    let mut overflow = 0u32;
    let mut written = 0;

    for &word in data {
        if written >= result.len() {
            return 0;
        }
        overflow |= word;
        result[written] = (word & 0xff) as u8;
        written += 1;
    }

    if (overflow & 0xffff_ff00) != 0 {
        return 0;
    }

    written
}

pub fn latin1_length_from_utf8(data: &[Utf8]) -> usize {
    data.iter().filter(|&&byte| non_continuation(byte)).count()
}

pub fn latin1_length_from_utf16le(data: &[Utf16]) -> usize {
    data.len()
}

pub fn latin1_length_from_utf32(data: &[Utf32]) -> usize {
    data.len()
}

pub fn latin1_convert_to_utf8(data: &[Latin1], result: &mut [Utf8]) -> usize {
    let mut written = 0;

    for &byte in data {
        if byte & 0x80 == 0 {
            if written >= result.len() {
                return 0;
            }
            result[written] = byte;
            written += 1;
        } else {
            if written + 1 >= result.len() {
                return 0;
            }
            result[written] = (byte >> 6) | 0b1100_0000;
            result[written + 1] = (byte & 0b11_1111) | 0b1000_0000;
            written += 2;
        }
    }

    written
}

pub fn latin1_convert_to_utf16le(data: &[Latin1], result: &mut [Utf16]) -> usize {
    let mut written = 0;

    for &byte in data {
        if written >= result.len() {
            return 0;
        }
        result[written] = write_utf16le(byte as u16);
        written += 1;
    }

    written
}

pub fn latin1_convert_to_utf32(data: &[Latin1], result: &mut [Utf32]) -> usize {
    let mut written = 0;

    for &byte in data {
        if written >= result.len() {
            return 0;
        }
        result[written] = byte as u32;
        written += 1;
    }

    written
}

pub fn ascii_validate(data: &[Ascii]) -> bool {
    data.iter().all(|&byte| byte < 0x80)
}
