pub type Utf8 = u8;
pub type Utf16 = u16;
pub type Utf32 = u32;
pub type Latin1 = u8;
pub type Ascii = u8;

pub fn utf8_validate(data: &[Utf8]) -> bool {
    let len = data.len();
    let mut pos = 0usize;

    while pos < len {
        // ASCII fast path: check next 16 bytes
        let next_pos = pos + 16;
        if next_pos <= len {
            let mut all_ascii = true;
            for i in 0..16 {
                if data[pos + i] & 0x80 != 0 {
                    all_ascii = false;
                    break;
                }
            }
            if all_ascii {
                pos = next_pos;
                continue;
            }
        }

        let mut word = data[pos];
        while word < 0b1000_0000 {
            pos += 1;
            if pos == len {
                return true;
            }
            word = data[pos];
        }

        if (word & 0b1110_0000) == 0b1100_0000 {
            let next_pos = pos + 2;
            if next_pos > len {
                return false;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return false;
            }
            let code_point: u32 = (((word & 0b0001_1111) as u32) << 6)
                | (data[pos + 1] & 0b0011_1111) as u32;
            if code_point < 0x80 || code_point > 0x7ff {
                return false;
            }
            pos = next_pos;
        } else if (word & 0b1111_0000) == 0b1110_0000 {
            let next_pos = pos + 3;
            if next_pos > len {
                return false;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return false;
            }
            if (data[pos + 2] & 0b1100_0000) != 0b1000_0000 {
                return false;
            }
            let code_point: u32 = (((word & 0b0000_1111) as u32) << 12)
                | (((data[pos + 1] & 0b0011_1111) as u32) << 6)
                | (data[pos + 2] & 0b0011_1111) as u32;
            if code_point < 0x800
                || code_point > 0xffff
                || (code_point > 0xd7ff && code_point < 0xe000)
            {
                return false;
            }
            pos = next_pos;
        } else if (word & 0b1111_1000) == 0b1111_0000 {
            let next_pos = pos + 4;
            if next_pos > len {
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
            let code_point: u32 = (((word & 0b0000_0111) as u32) << 18)
                | (((data[pos + 1] & 0b0011_1111) as u32) << 12)
                | (((data[pos + 2] & 0b0011_1111) as u32) << 6)
                | (data[pos + 3] & 0b0011_1111) as u32;
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
    for &word in data {
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
    for &w in data {
        counter += 1;
        if w > 0x7f {
            counter += 1;
        }
        if w > 0x7ff {
            counter += 1;
        }
        if w > 0xffff {
            counter += 1;
        }
    }
    counter
}

pub fn utf8_length_from_latin1(data: &[Latin1]) -> usize {
    let mut counter: usize = data.len();
    for &b in data {
        counter += (b >> 7) as usize;
    }
    counter
}

pub fn utf8_convert_to_utf16le(data: &[Utf8], result: &mut [Utf16]) -> usize {
    let len = data.len();
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        // ASCII fast path: 8 bytes
        if pos + 8 <= len {
            let mut all_ascii = true;
            for i in 0..8 {
                if data[pos + i] & 0x80 != 0 {
                    all_ascii = false;
                    break;
                }
            }
            if all_ascii {
                let final_pos = pos + 8;
                while pos < final_pos {
                    result[out] = data[pos] as u16;
                    out += 1;
                    pos += 1;
                }
                continue;
            }
        }

        let leading_byte = data[pos];
        if leading_byte < 0b1000_0000 {
            result[out] = leading_byte as u16;
            out += 1;
            pos += 1;
        } else if (leading_byte & 0b1110_0000) == 0b1100_0000 {
            if pos + 1 >= len {
                break;
            }
            let code_point: u16 = (((leading_byte & 0b0001_1111) as u16) << 6)
                | (data[pos + 1] & 0b0011_1111) as u16;
            result[out] = code_point;
            out += 1;
            pos += 2;
        } else if (leading_byte & 0b1111_0000) == 0b1110_0000 {
            if pos + 2 >= len {
                break;
            }
            let code_point: u16 = (((leading_byte & 0b0000_1111) as u16) << 12)
                | (((data[pos + 1] & 0b0011_1111) as u16) << 6)
                | (data[pos + 2] & 0b0011_1111) as u16;
            result[out] = code_point;
            out += 1;
            pos += 3;
        } else if (leading_byte & 0b1111_1000) == 0b1111_0000 {
            if pos + 3 >= len {
                break;
            }
            let mut code_point: u32 = (((leading_byte & 0b0000_0111) as u32) << 18)
                | (((data[pos + 1] & 0b0011_1111) as u32) << 12)
                | (((data[pos + 2] & 0b0011_1111) as u32) << 6)
                | (data[pos + 3] & 0b0011_1111) as u32;
            code_point -= 0x10000;
            let high_surrogate = (0xd800 + (code_point >> 10)) as u16;
            let low_surrogate = (0xdc00 + (code_point & 0x3ff)) as u16;
            result[out] = high_surrogate;
            out += 1;
            result[out] = low_surrogate;
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
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        if pos + 16 <= len {
            let mut all_ascii = true;
            for i in 0..16 {
                if data[pos + i] & 0x80 != 0 {
                    all_ascii = false;
                    break;
                }
            }
            if all_ascii {
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
        if leading_byte < 0b1000_0000 {
            result[out] = leading_byte as u32;
            out += 1;
            pos += 1;
        } else if (leading_byte & 0b1110_0000) == 0b1100_0000 {
            if pos + 1 >= len {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            let code_point: u32 = (((leading_byte & 0b0001_1111) as u32) << 6)
                | (data[pos + 1] & 0b0011_1111) as u32;
            if code_point < 0x80 || code_point > 0x7ff {
                return 0;
            }
            result[out] = code_point;
            out += 1;
            pos += 2;
        } else if (leading_byte & 0b1111_0000) == 0b1110_0000 {
            if pos + 2 >= len {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            if (data[pos + 2] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            let code_point: u32 = (((leading_byte & 0b0000_1111) as u32) << 12)
                | (((data[pos + 1] & 0b0011_1111) as u32) << 6)
                | (data[pos + 2] & 0b0011_1111) as u32;
            if code_point < 0x800
                || code_point > 0xffff
                || (code_point > 0xd7ff && code_point < 0xe000)
            {
                return 0;
            }
            result[out] = code_point;
            out += 1;
            pos += 3;
        } else if (leading_byte & 0b1111_1000) == 0b1111_0000 {
            if pos + 3 >= len {
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
            let code_point: u32 = (((leading_byte & 0b0000_0111) as u32) << 18)
                | (((data[pos + 1] & 0b0011_1111) as u32) << 12)
                | (((data[pos + 2] & 0b0011_1111) as u32) << 6)
                | (data[pos + 3] & 0b0011_1111) as u32;
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
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        if pos + 16 <= len {
            let mut all_ascii = true;
            for i in 0..16 {
                if data[pos + i] & 0x80 != 0 {
                    all_ascii = false;
                    break;
                }
            }
            if all_ascii {
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
        if leading_byte < 0b1000_0000 {
            result[out] = leading_byte;
            out += 1;
            pos += 1;
        } else if (leading_byte & 0b1110_0000) == 0b1100_0000 {
            if pos + 1 >= len {
                return 0;
            }
            if (data[pos + 1] & 0b1100_0000) != 0b1000_0000 {
                return 0;
            }
            let code_point: u32 = (((leading_byte & 0b0001_1111) as u32) << 6)
                | (data[pos + 1] & 0b0011_1111) as u32;
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
    let mut pos = 0usize;

    while pos < len {
        let word = data[pos];
        if (word & 0xf800) == 0xd800 {
            if pos + 1 >= len {
                return false;
            }
            let diff = word.wrapping_sub(0xd800);
            if diff > 0x3ff {
                return false;
            }
            let word2 = data[pos + 1];
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
    for &b in data {
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
    for &w in data {
        counter += 1;
        if w > 0xffff {
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
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        // ASCII fast path: 4 words
        if pos + 4 <= len {
            let mut all_ascii = true;
            for i in 0..4 {
                if data[pos + i] & 0xff80 != 0 {
                    all_ascii = false;
                    break;
                }
            }
            if all_ascii {
                let final_pos = pos + 4;
                while pos < final_pos {
                    result[out] = data[pos] as u8;
                    out += 1;
                    pos += 1;
                }
                continue;
            }
        }

        let word = data[pos];
        if (word & 0xff80) == 0 {
            result[out] = word as u8;
            out += 1;
            pos += 1;
        } else if (word & 0xf800) == 0 {
            result[out] = ((word >> 6) as u8) | 0b1100_0000;
            out += 1;
            result[out] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            pos += 1;
        } else if (word & 0xf800) != 0xd800 {
            result[out] = ((word >> 12) as u8) | 0b1110_0000;
            out += 1;
            result[out] = (((word >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            result[out] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            pos += 1;
        } else {
            let diff = word.wrapping_sub(0xd800) as u32;
            if pos + 1 >= len {
                return 0;
            }
            let word2 = data[pos + 1];
            let value: u32 = (diff << 10) + (word2.wrapping_sub(0xdc00) as u32) + 0x10000;
            result[out] = ((value >> 18) as u8) | 0b1111_0000;
            out += 1;
            result[out] = (((value >> 12) & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            result[out] = (((value >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            result[out] = ((value & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            pos += 2;
        }
    }

    out
}

pub fn utf16le_convert_to_utf32(data: &[Utf16], result: &mut [Utf32]) -> usize {
    let len = data.len();
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        let word = data[pos];
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
            let word2 = data[pos + 1];
            let value: u32 = ((diff as u32) << 10) + (word2.wrapping_sub(0xdc00) as u32) + 0x10000;
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
    let mut pos = 0usize;
    let mut out = 0usize;
    let mut overflow: u16 = 0;

    while pos < len {
        let word = data[pos];
        overflow |= word;
        result[out] = (word & 0xff) as u8;
        out += 1;
        pos += 1;
    }

    if overflow & 0xff00 != 0 {
        return 0;
    }

    out
}

pub fn utf32_validate(data: &[Utf32]) -> bool {
    for &word in data {
        if word > 0x10ffff || (word >= 0xd800 && word <= 0xdfff) {
            return false;
        }
    }
    true
}

pub fn utf32_length_from_utf8(data: &[Utf8]) -> usize {
    let mut counter: usize = 0;
    for &b in data {
        if (b as i8) > -65 {
            counter += 1;
        }
    }
    counter
}

pub fn utf32_length_from_utf16le(data: &[Utf16]) -> usize {
    let mut counter: usize = 0;
    for &word in data {
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
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        if pos + 2 <= len {
            if (data[pos] & 0xffff_ff80) == 0 && (data[pos + 1] & 0xffff_ff80) == 0 {
                result[out] = data[pos] as u8;
                out += 1;
                result[out] = data[pos + 1] as u8;
                out += 1;
                pos += 2;
                continue;
            }
        }

        let word = data[pos];
        if (word & 0xffff_ff80) == 0 {
            result[out] = word as u8;
            out += 1;
            pos += 1;
        } else if (word & 0xffff_f800) == 0 {
            result[out] = ((word >> 6) as u8) | 0b1100_0000;
            out += 1;
            result[out] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            pos += 1;
        } else if (word & 0xffff_0000) == 0 {
            if word >= 0xd800 && word <= 0xdfff {
                return 0;
            }
            result[out] = ((word >> 12) as u8) | 0b1110_0000;
            out += 1;
            result[out] = (((word >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            result[out] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            pos += 1;
        } else {
            if word > 0x10ffff {
                return 0;
            }
            result[out] = ((word >> 18) as u8) | 0b1111_0000;
            out += 1;
            result[out] = (((word >> 12) & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            result[out] = (((word >> 6) & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            result[out] = ((word & 0b11_1111) as u8) | 0b1000_0000;
            out += 1;
            pos += 1;
        }
    }

    out
}

pub fn utf32_convert_to_utf16le(data: &[Utf32], result: &mut [Utf16]) -> usize {
    let len = data.len();
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        let mut word = data[pos];
        if (word & 0xffff_0000) == 0 {
            if word >= 0xd800 && word <= 0xdfff {
                return 0;
            }
            result[out] = word as u16;
            out += 1;
        } else {
            if word > 0x10ffff {
                return 0;
            }
            word -= 0x10000;
            let high_surrogate = (0xd800 + (word >> 10)) as u16;
            let low_surrogate = (0xdc00 + (word & 0x3ff)) as u16;
            result[out] = high_surrogate;
            out += 1;
            result[out] = low_surrogate;
            out += 1;
        }
        pos += 1;
    }

    out
}

pub fn utf32_convert_to_latin1(data: &[Utf32], result: &mut [Latin1]) -> usize {
    let len = data.len();
    let mut pos = 0usize;
    let mut out = 0usize;
    let mut overflow: u32 = 0;

    while pos < len {
        let word = data[pos];
        overflow |= word;
        result[out] = (word & 0xff) as u8;
        out += 1;
        pos += 1;
    }

    if overflow & 0xffff_ff00 != 0 {
        return 0;
    }

    out
}

pub fn latin1_length_from_utf8(data: &[Utf8]) -> usize {
    let mut counter: usize = 0;
    for &b in data {
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
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        if pos + 16 <= len {
            let mut all_ascii = true;
            for i in 0..16 {
                if data[pos + i] & 0x80 != 0 {
                    all_ascii = false;
                    break;
                }
            }
            if all_ascii {
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
            result[out] = (byte >> 6) | 0b1100_0000;
            out += 1;
            result[out] = (byte & 0b11_1111) | 0b1000_0000;
            out += 1;
            pos += 1;
        }
    }

    out
}

pub fn latin1_convert_to_utf16le(data: &[Latin1], result: &mut [Utf16]) -> usize {
    let len = data.len();
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        result[out] = data[pos] as u16;
        out += 1;
        pos += 1;
    }

    out
}

pub fn latin1_convert_to_utf32(data: &[Latin1], result: &mut [Utf32]) -> usize {
    let mut out = 0usize;
    for &b in data {
        result[out] = b as u32;
        out += 1;
    }
    out
}

pub fn ascii_validate(data: &[Ascii]) -> bool {
    for &b in data {
        if b & 0x80 != 0 {
            return false;
        }
    }
    true
}
