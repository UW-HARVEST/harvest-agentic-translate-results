pub type Utf8 = u8;
pub type Utf16 = u16;
pub type Utf32 = u32;
pub type Latin1 = u8;
pub type Ascii = u8;

#[inline]
fn from_le_u16(n: u16) -> u16 {
    u16::from_le(n)
}

#[inline]
fn to_le_u16(n: u16) -> u16 {
    n.to_le()
}

pub fn utf8_validate(data: &[Utf8]) -> bool {
    let len = data.len();
    let mut pos: usize = 0;
    while pos < len {
        // Fast ASCII path: if next 16 bytes are all ASCII, skip them
        let next_pos = pos + 16;
        if next_pos <= len {
            let mut high_bit = 0u8;
            for i in 0..16 {
                high_bit |= data[pos + i];
            }
            if (high_bit & 0x80) == 0 {
                pos = next_pos;
                continue;
            }
        }
        let mut word = data[pos];
        while word < 0b10000000 {
            pos += 1;
            if pos == len {
                return true;
            }
            word = data[pos];
        }
        if (word & 0b11100000) == 0b11000000 {
            let next_pos = pos + 2;
            if next_pos > len {
                return false;
            }
            if (data[pos + 1] & 0b11000000) != 0b10000000 {
                return false;
            }
            let code_point: u32 = (((word & 0b00011111) as u32) << 6)
                | ((data[pos + 1] & 0b00111111) as u32);
            if code_point < 0x80 || code_point > 0x7ff {
                return false;
            }
            pos = next_pos;
        } else if (word & 0b11110000) == 0b11100000 {
            let next_pos = pos + 3;
            if next_pos > len {
                return false;
            }
            if (data[pos + 1] & 0b11000000) != 0b10000000 {
                return false;
            }
            if (data[pos + 2] & 0b11000000) != 0b10000000 {
                return false;
            }
            let code_point: u32 = (((word & 0b00001111) as u32) << 12)
                | (((data[pos + 1] & 0b00111111) as u32) << 6)
                | ((data[pos + 2] & 0b00111111) as u32);
            if code_point < 0x800
                || code_point > 0xffff
                || (code_point > 0xd7ff && code_point < 0xe000)
            {
                return false;
            }
            pos = next_pos;
        } else if (word & 0b11111000) == 0b11110000 {
            let next_pos = pos + 4;
            if next_pos > len {
                return false;
            }
            if (data[pos + 1] & 0b11000000) != 0b10000000 {
                return false;
            }
            if (data[pos + 2] & 0b11000000) != 0b10000000 {
                return false;
            }
            if (data[pos + 3] & 0b11000000) != 0b10000000 {
                return false;
            }
            let code_point: u32 = (((word & 0b00000111) as u32) << 18)
                | (((data[pos + 1] & 0b00111111) as u32) << 12)
                | (((data[pos + 2] & 0b00111111) as u32) << 6)
                | ((data[pos + 3] & 0b00111111) as u32);
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
    let mut counter: usize = 0;
    for &raw in data.iter() {
        let word = from_le_u16(raw);
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
    let mut counter: usize = 0;
    for &c in data.iter() {
        counter += 1;
        if c > 0x7f {
            counter += 1;
        }
        if c > 0x7ff {
            counter += 1;
        }
        if c > 0xffff {
            counter += 1;
        }
    }
    counter
}

pub fn utf8_length_from_latin1(data: &[Latin1]) -> usize {
    let mut counter: usize = data.len();
    for &c in data.iter() {
        counter += (c >> 7) as usize;
    }
    counter
}

pub fn utf8_convert_to_utf16le(data: &[Utf8], result: &mut [Utf16]) -> usize {
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        // Fast ASCII path: 8 bytes at a time
        if pos + 8 <= len {
            let mut high_bit = 0u8;
            for i in 0..8 {
                high_bit |= data[pos + i];
            }
            if (high_bit & 0x80) == 0 {
                let final_pos = pos + 8;
                while pos < final_pos {
                    result[out] = to_le_u16(data[pos] as u16);
                    out += 1;
                    pos += 1;
                }
                continue;
            }
        }
        let leading_byte = data[pos];
        if leading_byte < 0b10000000 {
            result[out] = to_le_u16(leading_byte as u16);
            out += 1;
            pos += 1;
        } else if (leading_byte & 0b11100000) == 0b11000000 {
            if pos + 1 >= len {
                break;
            }
            let code_point: u16 = (((leading_byte & 0b00011111) as u16) << 6)
                | ((data[pos + 1] & 0b00111111) as u16);
            result[out] = to_le_u16(code_point);
            out += 1;
            pos += 2;
        } else if (leading_byte & 0b11110000) == 0b11100000 {
            if pos + 2 >= len {
                break;
            }
            let code_point: u16 = (((leading_byte & 0b00001111) as u16) << 12)
                | (((data[pos + 1] & 0b00111111) as u16) << 6)
                | ((data[pos + 2] & 0b00111111) as u16);
            result[out] = to_le_u16(code_point);
            out += 1;
            pos += 3;
        } else if (leading_byte & 0b11111000) == 0b11110000 {
            if pos + 3 >= len {
                break;
            }
            let code_point: u32 = (((leading_byte & 0b00000111) as u32) << 18)
                | (((data[pos + 1] & 0b00111111) as u32) << 12)
                | (((data[pos + 2] & 0b00111111) as u32) << 6)
                | ((data[pos + 3] & 0b00111111) as u32);
            let code_point = code_point - 0x10000;
            let high_surrogate: u16 = 0xd800 + ((code_point >> 10) as u16);
            let low_surrogate: u16 = 0xdc00 + ((code_point & 0x3ff) as u16);
            result[out] = to_le_u16(high_surrogate);
            out += 1;
            result[out] = to_le_u16(low_surrogate);
            out += 1;
            pos += 4;
        } else {
            return 0;
        }
    }

    out
}

pub fn utf8_convert_to_utf32(data: &[Utf8], result: &mut [Utf32]) -> usize {
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        if pos + 16 <= len {
            let mut high_bit = 0u8;
            for i in 0..16 {
                high_bit |= data[pos + i];
            }
            if (high_bit & 0x80) == 0 {
                let final_pos = pos + 16;
                while pos < final_pos {
                    result[out] = data[pos] as u32;
                    out += 1;
                    pos += 1;
                }
                continue;
            }
        }
        let leading_byte = data[pos];
        if leading_byte < 0b10000000 {
            result[out] = leading_byte as u32;
            out += 1;
            pos += 1;
        } else if (leading_byte & 0b11100000) == 0b11000000 {
            if pos + 1 >= len {
                return 0;
            }
            if (data[pos + 1] & 0b11000000) != 0b10000000 {
                return 0;
            }
            let code_point: u32 = (((leading_byte & 0b00011111) as u32) << 6)
                | ((data[pos + 1] & 0b00111111) as u32);
            if code_point < 0x80 || code_point > 0x7ff {
                return 0;
            }
            result[out] = code_point;
            out += 1;
            pos += 2;
        } else if (leading_byte & 0b11110000) == 0b11100000 {
            if pos + 2 >= len {
                return 0;
            }
            if (data[pos + 1] & 0b11000000) != 0b10000000 {
                return 0;
            }
            if (data[pos + 2] & 0b11000000) != 0b10000000 {
                return 0;
            }
            let code_point: u32 = (((leading_byte & 0b00001111) as u32) << 12)
                | (((data[pos + 1] & 0b00111111) as u32) << 6)
                | ((data[pos + 2] & 0b00111111) as u32);
            if code_point < 0x800
                || code_point > 0xffff
                || (code_point > 0xd7ff && code_point < 0xe000)
            {
                return 0;
            }
            result[out] = code_point;
            out += 1;
            pos += 3;
        } else if (leading_byte & 0b11111000) == 0b11110000 {
            if pos + 3 >= len {
                return 0;
            }
            if (data[pos + 1] & 0b11000000) != 0b10000000 {
                return 0;
            }
            if (data[pos + 2] & 0b11000000) != 0b10000000 {
                return 0;
            }
            if (data[pos + 3] & 0b11000000) != 0b10000000 {
                return 0;
            }
            let code_point: u32 = (((leading_byte & 0b00000111) as u32) << 18)
                | (((data[pos + 1] & 0b00111111) as u32) << 12)
                | (((data[pos + 2] & 0b00111111) as u32) << 6)
                | ((data[pos + 3] & 0b00111111) as u32);
            if code_point <= 0xffff || code_point > 0x10ffff {
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
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        if pos + 16 <= len {
            let mut high_bit = 0u8;
            for i in 0..16 {
                high_bit |= data[pos + i];
            }
            if (high_bit & 0x80) == 0 {
                let final_pos = pos + 16;
                while pos < final_pos {
                    result[out] = data[pos];
                    out += 1;
                    pos += 1;
                }
                continue;
            }
        }
        let leading_byte = data[pos];
        if leading_byte < 0b10000000 {
            result[out] = leading_byte;
            out += 1;
            pos += 1;
        } else if (leading_byte & 0b11100000) == 0b11000000 {
            if pos + 1 >= len {
                return 0;
            }
            if (data[pos + 1] & 0b11000000) != 0b10000000 {
                return 0;
            }
            let code_point: u32 = (((leading_byte & 0b00011111) as u32) << 6)
                | ((data[pos + 1] & 0b00111111) as u32);
            if code_point < 0x80 || code_point > 0xff {
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
    let len = data.len();
    let mut pos: usize = 0;
    while pos < len {
        let word = from_le_u16(data[pos]);
        if (word & 0xf800) == 0xd800 {
            if pos + 1 >= len {
                return false;
            }
            let diff = word.wrapping_sub(0xd800);
            if diff > 0x3ff {
                return false;
            }
            let word2 = from_le_u16(data[pos + 1]);
            let diff2 = word2.wrapping_sub(0xdc00);
            if diff2 > 0x3ff {
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
    let mut counter: usize = 0;
    for &b in data.iter() {
        if (b as i8) > -65 {
            counter += 1;
        }
        if b >= 240 {
            counter += 1;
        }
    }
    counter
}

pub fn utf16_length_from_utf32(data: &[Utf32]) -> usize {
    let mut counter: usize = 0;
    for &c in data.iter() {
        counter += 1;
        if c > 0xffff {
            counter += 1;
        }
    }
    counter
}

pub fn utf16_length_from_latin1(data: &[Latin1]) -> usize {
    data.len()
}

pub fn utf16le_convert_to_utf8(data: &[Utf16], result: &mut [Utf8]) -> usize {
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        // Fast ASCII path: 4 u16 words at once
        if pos + 4 <= len {
            let mut high_bits: u16 = 0;
            for i in 0..4 {
                high_bits |= from_le_u16(data[pos + i]);
            }
            if (high_bits & 0xff80) == 0 {
                let final_pos = pos + 4;
                while pos < final_pos {
                    result[out] = from_le_u16(data[pos]) as u8;
                    out += 1;
                    pos += 1;
                }
                continue;
            }
        }
        let word = from_le_u16(data[pos]);
        if (word & 0xff80) == 0 {
            result[out] = word as u8;
            out += 1;
            pos += 1;
        } else if (word & 0xf800) == 0 {
            result[out] = ((word >> 6) as u8) | 0b11000000;
            out += 1;
            result[out] = ((word & 0b111111) as u8) | 0b10000000;
            out += 1;
            pos += 1;
        } else if (word & 0xf800) != 0xd800 {
            result[out] = ((word >> 12) as u8) | 0b11100000;
            out += 1;
            result[out] = (((word >> 6) & 0b111111) as u8) | 0b10000000;
            out += 1;
            result[out] = ((word & 0b111111) as u8) | 0b10000000;
            out += 1;
            pos += 1;
        } else {
            let diff = word.wrapping_sub(0xd800);
            if pos + 1 >= len {
                return 0;
            }
            let word2 = from_le_u16(data[pos + 1]);
            let value: u32 = ((diff as u32) << 10)
                .wrapping_add(word2.wrapping_sub(0xdc00) as u32)
                .wrapping_add(0x10000);
            result[out] = ((value >> 18) as u8) | 0b11110000;
            out += 1;
            result[out] = (((value >> 12) & 0b111111) as u8) | 0b10000000;
            out += 1;
            result[out] = (((value >> 6) & 0b111111) as u8) | 0b10000000;
            out += 1;
            result[out] = ((value & 0b111111) as u8) | 0b10000000;
            out += 1;
            pos += 2;
        }
    }

    out
}

pub fn utf16le_convert_to_utf32(data: &[Utf16], result: &mut [Utf32]) -> usize {
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        let word = from_le_u16(data[pos]);
        if (word & 0xf800) != 0xd800 {
            result[out] = word as u32;
            out += 1;
            pos += 1;
        } else {
            let diff = word.wrapping_sub(0xd800);
            if diff > 0x3ff {
                return 0;
            }
            if pos + 1 >= len {
                return 0;
            }
            let word2 = from_le_u16(data[pos + 1]);
            let value: u32 = ((diff as u32) << 10)
                .wrapping_add(word2.wrapping_sub(0xdc00) as u32)
                .wrapping_add(0x10000);
            let diff2 = word2.wrapping_sub(0xdc00);
            if diff2 > 0x3ff {
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
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;
    let mut overflow: u16 = 0;

    while pos < len {
        let word = from_le_u16(data[pos]);
        overflow |= word;
        result[out] = (word & 0xff) as u8;
        out += 1;
        pos += 1;
    }

    if (overflow & 0xff00) != 0 {
        return 0;
    }

    out
}

pub fn utf32_validate(data: &[Utf32]) -> bool {
    for &word in data.iter() {
        if word > 0x10ffff || (word >= 0xd800 && word <= 0xdfff) {
            return false;
        }
    }
    true
}

pub fn utf32_length_from_utf8(data: &[Utf8]) -> usize {
    let mut counter: usize = 0;
    for &b in data.iter() {
        if (b as i8) > -65 {
            counter += 1;
        }
    }
    counter
}

pub fn utf32_length_from_utf16le(data: &[Utf16]) -> usize {
    let mut counter: usize = 0;
    for &raw in data.iter() {
        let word = from_le_u16(raw);
        if (word & 0xfc00) != 0xdc00 {
            counter += 1;
        }
    }
    counter
}

pub fn utf32_length_from_latin1(data: &[Latin1]) -> usize {
    data.len()
}

pub fn utf32_convert_to_utf8(data: &[Utf32], result: &mut [Utf8]) -> usize {
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        if pos + 2 <= len {
            // Two consecutive ASCII codepoints fast path
            if (data[pos] & 0xffffff80) == 0 && (data[pos + 1] & 0xffffff80) == 0 {
                result[out] = data[pos] as u8;
                out += 1;
                result[out] = data[pos + 1] as u8;
                out += 1;
                pos += 2;
                continue;
            }
        }
        let word = data[pos];
        if (word & 0xffffff80) == 0 {
            result[out] = word as u8;
            out += 1;
            pos += 1;
        } else if (word & 0xfffff800) == 0 {
            result[out] = ((word >> 6) as u8) | 0b11000000;
            out += 1;
            result[out] = ((word & 0b111111) as u8) | 0b10000000;
            out += 1;
            pos += 1;
        } else if (word & 0xffff0000) == 0 {
            if word >= 0xd800 && word <= 0xdfff {
                return 0;
            }
            result[out] = ((word >> 12) as u8) | 0b11100000;
            out += 1;
            result[out] = (((word >> 6) & 0b111111) as u8) | 0b10000000;
            out += 1;
            result[out] = ((word & 0b111111) as u8) | 0b10000000;
            out += 1;
            pos += 1;
        } else {
            if word > 0x10ffff {
                return 0;
            }
            result[out] = ((word >> 18) as u8) | 0b11110000;
            out += 1;
            result[out] = (((word >> 12) & 0b111111) as u8) | 0b10000000;
            out += 1;
            result[out] = (((word >> 6) & 0b111111) as u8) | 0b10000000;
            out += 1;
            result[out] = ((word & 0b111111) as u8) | 0b10000000;
            out += 1;
            pos += 1;
        }
    }

    out
}

pub fn utf32_convert_to_utf16le(data: &[Utf32], result: &mut [Utf16]) -> usize {
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        let word = data[pos];
        if (word & 0xffff0000) == 0 {
            if word >= 0xd800 && word <= 0xdfff {
                return 0;
            }
            result[out] = to_le_u16(word as u16);
            out += 1;
        } else {
            if word > 0x10ffff {
                return 0;
            }
            let w = word - 0x10000;
            let high_surrogate: u16 = 0xd800 + ((w >> 10) as u16);
            let low_surrogate: u16 = 0xdc00 + ((w & 0x3ff) as u16);
            result[out] = to_le_u16(high_surrogate);
            out += 1;
            result[out] = to_le_u16(low_surrogate);
            out += 1;
        }
        pos += 1;
    }

    out
}

pub fn utf32_convert_to_latin1(data: &[Utf32], result: &mut [Latin1]) -> usize {
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;
    let mut overflow: u32 = 0;

    while pos < len {
        let word = data[pos];
        overflow |= word;
        result[out] = (word & 0xff) as u8;
        out += 1;
        pos += 1;
    }

    if (overflow & 0xffffff00) != 0 {
        return 0;
    }

    out
}

pub fn latin1_length_from_utf8(data: &[Utf8]) -> usize {
    let mut counter: usize = 0;
    for &b in data.iter() {
        if (b as i8) > -65 {
            counter += 1;
        }
    }
    counter
}

pub fn latin1_length_from_utf16le(data: &[Utf16]) -> usize {
    data.len()
}

pub fn latin1_length_from_utf32(data: &[Utf32]) -> usize {
    data.len()
}

pub fn latin1_convert_to_utf8(data: &[Latin1], result: &mut [Utf8]) -> usize {
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        if pos + 16 <= len {
            let mut high_bit = 0u8;
            for i in 0..16 {
                high_bit |= data[pos + i];
            }
            if (high_bit & 0x80) == 0 {
                let final_pos = pos + 16;
                while pos < final_pos {
                    result[out] = data[pos];
                    out += 1;
                    pos += 1;
                }
                continue;
            }
        }
        let byte = data[pos];
        if (byte & 0x80) == 0 {
            result[out] = byte;
            out += 1;
            pos += 1;
        } else {
            result[out] = (byte >> 6) | 0b11000000;
            out += 1;
            result[out] = (byte & 0b111111) | 0b10000000;
            out += 1;
            pos += 1;
        }
    }

    out
}

pub fn latin1_convert_to_utf16le(data: &[Latin1], result: &mut [Utf16]) -> usize {
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        result[out] = to_le_u16(data[pos] as u16);
        out += 1;
        pos += 1;
    }

    out
}

pub fn latin1_convert_to_utf32(data: &[Latin1], result: &mut [Utf32]) -> usize {
    let len = data.len();
    for i in 0..len {
        result[i] = data[i] as u32;
    }
    len
}

pub fn ascii_validate(data: &[Ascii]) -> bool {
    let len = data.len();
    let mut pos: usize = 0;

    while pos + 16 <= len {
        let mut high_bit = 0u8;
        for i in 0..16 {
            high_bit |= data[pos + i];
        }
        if (high_bit & 0x80) != 0 {
            return false;
        }
        pos += 16;
    }
    while pos < len {
        if data[pos] >= 0b10000000 {
            return false;
        }
        pos += 1;
    }
    true
}
