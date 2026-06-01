pub type Utf8 = u8;
pub type Utf16 = u16;
pub type Utf32 = u32;
pub type Latin1 = u8;
pub type Ascii = u8;

pub fn utf8_validate(data: &[Utf8]) -> bool {
    let len = data.len();
    let mut pos: usize = 0;

    while pos < len {
        let next_pos = pos + 16;
        if next_pos <= len {
            let v1 = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            let v2 = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().unwrap());
            let v = v1 | v2;
            if (v & 0x8080808080808080) == 0 {
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
        let next_pos;
        if (word & 0b11100000) == 0b11000000 {
            next_pos = pos + 2;
            if next_pos > len {
                return false;
            }
            if (data[pos + 1] & 0b11000000) != 0b10000000 {
                return false;
            }
            let code_point: u32 = (((word & 0b00011111) as u32) << 6)
                | ((data[pos + 1] & 0b00111111) as u32);
            if code_point < 0x80 || 0x7ff < code_point {
                return false;
            }
        } else if (word & 0b11110000) == 0b11100000 {
            next_pos = pos + 3;
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
                || 0xffff < code_point
                || (0xd7ff < code_point && code_point < 0xe000)
            {
                return false;
            }
        } else if (word & 0b11111000) == 0b11110000 {
            next_pos = pos + 4;
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
            if code_point <= 0xffff || 0x10ffff < code_point {
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
    let mut counter: usize = 0;
    for &word in data.iter() {
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
    for &d in data.iter() {
        counter += 1;
        if d > 0x7f {
            counter += 1;
        }
        if d > 0x7ff {
            counter += 1;
        }
        if d > 0xffff {
            counter += 1;
        }
    }
    counter
}

pub fn utf8_length_from_latin1(data: &[Latin1]) -> usize {
    let mut counter: usize = data.len();
    for &d in data.iter() {
        counter += (d >> 7) as usize;
    }
    counter
}

pub fn utf8_convert_to_utf16le(data: &[Utf8], result: &mut [Utf16]) -> usize {
    let len = data.len();
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        if pos + 8 <= len {
            let v = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            if (v & 0x8080808080808080) == 0 {
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
        if leading_byte < 0b10000000 {
            result[out] = leading_byte as u16;
            out += 1;
            pos += 1;
        } else if (leading_byte & 0b11100000) == 0b11000000 {
            if pos + 1 >= len {
                break;
            }
            let code_point: u16 = (((leading_byte & 0b00011111) as u16) << 6)
                | ((data[pos + 1] & 0b00111111) as u16);
            result[out] = code_point;
            out += 1;
            pos += 2;
        } else if (leading_byte & 0b11110000) == 0b11100000 {
            if pos + 2 >= len {
                break;
            }
            let code_point: u16 = (((leading_byte & 0b00001111) as u16) << 12)
                | (((data[pos + 1] & 0b00111111) as u16) << 6)
                | ((data[pos + 2] & 0b00111111) as u16);
            result[out] = code_point;
            out += 1;
            pos += 3;
        } else if (leading_byte & 0b11111000) == 0b11110000 {
            if pos + 3 >= len {
                break;
            }
            let mut code_point: u32 = (((leading_byte & 0b00000111) as u32) << 18)
                | (((data[pos + 1] & 0b00111111) as u32) << 12)
                | (((data[pos + 2] & 0b00111111) as u32) << 6)
                | ((data[pos + 3] & 0b00111111) as u32);
            code_point -= 0x10000;
            let high_surrogate: u16 = 0xd800 + (code_point >> 10) as u16;
            let low_surrogate: u16 = 0xdc00 + (code_point & 0x3ff) as u16;
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
    let mut pos: usize = 0;
    let mut out: usize = 0;

    while pos < len {
        if pos + 16 <= len {
            let v1 = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            let v2 = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().unwrap());
            let v = v1 | v2;
            if (v & 0x8080808080808080) == 0 {
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
            if code_point < 0x80 || 0x7ff < code_point {
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
                || 0xffff < code_point
                || (0xd7ff < code_point && code_point < 0xe000)
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
            if code_point <= 0xffff || 0x10ffff < code_point {
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
            let v1 = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            let v2 = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().unwrap());
            let v = v1 | v2;
            if (v & 0x8080808080808080) == 0 {
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
            if code_point < 0x80 || 0xff < code_point {
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
    for &d in data.iter() {
        if (d as i8) > -65 {
            counter += 1;
        }
        if d >= 240 {
            counter += 1;
        }
    }
    counter
}

pub fn utf16_length_from_utf32(data: &[Utf32]) -> usize {
    let mut counter: usize = 0;
    for &d in data.iter() {
        counter += 1;
        if d > 0xffff {
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
        if pos + 4 <= len {
            // Check if all 4 u16 are ASCII (top byte zero, low byte < 0x80)
            // The C check is: (v & 0xff80ff80ff80ff80) == 0 on the little-endian-loaded u64.
            // Since Rust u16 are native u16, just check each value is < 0x80.
            let w0 = data[pos];
            let w1 = data[pos + 1];
            let w2 = data[pos + 2];
            let w3 = data[pos + 3];
            if (w0 | w1 | w2 | w3) < 0x80 {
                result[out] = w0 as u8;
                out += 1;
                result[out] = w1 as u8;
                out += 1;
                result[out] = w2 as u8;
                out += 1;
                result[out] = w3 as u8;
                out += 1;
                pos += 4;
                continue;
            }
        }
        let word = data[pos];
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
            let word2 = data[pos + 1];
            let value: u32 = ((diff as u32) << 10) + (word2.wrapping_sub(0xdc00) as u32) + 0x10000;
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
            let value: u32 =
                ((diff as u32) << 10) + (word2.wrapping_sub(0xdc00) as u32) + 0x10000;
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
        let word = data[pos];
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
    for &d in data.iter() {
        if (d as i8) > -65 {
            counter += 1;
        }
    }
    counter
}

pub fn utf32_length_from_utf16le(data: &[Utf16]) -> usize {
    let mut counter: usize = 0;
    for &word in data.iter() {
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
            // The C code uses memcpy to load 8 bytes from utf32 array (2 elements)
            // and checks (v & 0xffffff80ffffff80) == 0. This is equivalent to checking
            // both elements < 0x80 (assuming little-endian).
            if data[pos] < 0x80 && data[pos + 1] < 0x80 {
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
            result[out] = word as u16;
            out += 1;
        } else {
            if word > 0x10ffff {
                return 0;
            }
            let w = word - 0x10000;
            let high_surrogate: u16 = 0xd800 + ((w >> 10) as u16);
            let low_surrogate: u16 = 0xdc00 + ((w & 0x3ff) as u16);
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
    for &d in data.iter() {
        if (d as i8) > -65 {
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
            let v1 = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
            let v2 = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().unwrap());
            let v = v1 | v2;
            if (v & 0x8080808080808080) == 0 {
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
        result[out] = data[pos] as u16;
        out += 1;
        pos += 1;
    }

    out
}

pub fn latin1_convert_to_utf32(data: &[Latin1], result: &mut [Utf32]) -> usize {
    let len = data.len();
    let mut out: usize = 0;
    for i in 0..len {
        result[out] = data[i] as u32;
        out += 1;
    }
    out
}

pub fn ascii_validate(data: &[Ascii]) -> bool {
    let len = data.len();
    let mut pos: usize = 0;

    while pos + 16 <= len {
        let v1 = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        let v2 = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().unwrap());
        let v = v1 | v2;
        if (v & 0x8080808080808080) != 0 {
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
