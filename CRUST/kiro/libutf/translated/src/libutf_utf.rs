pub type Utf8 = u8;
pub type Utf16 = u16;
pub type Utf32 = u32;
pub type Latin1 = u8;
pub type Ascii = u8;

pub fn utf8_validate(data: &[Utf8]) -> bool {
    let len = data.len();
    let mut pos = 0usize;

    while pos < len {
        let next_pos = pos + 16;
        if next_pos <= len {
            let mut all_ascii = true;
            for i in pos..next_pos {
                if data[i] & 0x80 != 0 {
                    all_ascii = false;
                    break;
                }
            }
            if all_ascii {
                pos = next_pos;
                continue;
            }
        }
        let word = data[pos];
        if word < 0x80 {
            pos += 1;
            continue;
        }
        if word & 0xe0 == 0xc0 {
            let np = pos + 2;
            if np > len { return false; }
            if data[pos + 1] & 0xc0 != 0x80 { return false; }
            let cp = ((word as u32 & 0x1f) << 6) | (data[pos + 1] as u32 & 0x3f);
            if cp < 0x80 || cp > 0x7ff { return false; }
            pos = np;
        } else if word & 0xf0 == 0xe0 {
            let np = pos + 3;
            if np > len { return false; }
            if data[pos + 1] & 0xc0 != 0x80 { return false; }
            if data[pos + 2] & 0xc0 != 0x80 { return false; }
            let cp = ((word as u32 & 0x0f) << 12)
                | ((data[pos + 1] as u32 & 0x3f) << 6)
                | (data[pos + 2] as u32 & 0x3f);
            if cp < 0x800 || cp > 0xffff || (cp > 0xd7ff && cp < 0xe000) { return false; }
            pos = np;
        } else if word & 0xf8 == 0xf0 {
            let np = pos + 4;
            if np > len { return false; }
            if data[pos + 1] & 0xc0 != 0x80 { return false; }
            if data[pos + 2] & 0xc0 != 0x80 { return false; }
            if data[pos + 3] & 0xc0 != 0x80 { return false; }
            let cp = ((word as u32 & 0x07) << 18)
                | ((data[pos + 1] as u32 & 0x3f) << 12)
                | ((data[pos + 2] as u32 & 0x3f) << 6)
                | (data[pos + 3] as u32 & 0x3f);
            if cp <= 0xffff || cp > 0x10ffff { return false; }
            pos = np;
        } else {
            return false;
        }
    }
    true
}

pub fn utf8_length_from_utf16le(data: &[Utf16]) -> usize {
    let mut counter = 0usize;
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
    let mut counter = 0usize;
    for &v in data {
        counter += 1;
        counter += (v > 0x7f) as usize;
        counter += (v > 0x7ff) as usize;
        counter += (v > 0xffff) as usize;
    }
    counter
}

pub fn utf8_length_from_latin1(data: &[Latin1]) -> usize {
    let mut counter = data.len();
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
        if pos + 8 <= len {
            let mut all_ascii = true;
            for i in pos..pos + 8 {
                if data[i] & 0x80 != 0 { all_ascii = false; break; }
            }
            if all_ascii {
                for i in pos..pos + 8 {
                    result[out] = data[i] as u16;
                    out += 1;
                }
                pos += 8;
                continue;
            }
        }
        let b = data[pos];
        if b < 0x80 {
            result[out] = b as u16;
            out += 1;
            pos += 1;
        } else if b & 0xe0 == 0xc0 {
            if pos + 1 >= len { break; }
            let cp = ((b as u16 & 0x1f) << 6) | (data[pos + 1] as u16 & 0x3f);
            result[out] = cp;
            out += 1;
            pos += 2;
        } else if b & 0xf0 == 0xe0 {
            if pos + 2 >= len { break; }
            let cp = ((b as u16 & 0x0f) << 12)
                | ((data[pos + 1] as u16 & 0x3f) << 6)
                | (data[pos + 2] as u16 & 0x3f);
            result[out] = cp;
            out += 1;
            pos += 3;
        } else if b & 0xf8 == 0xf0 {
            if pos + 3 >= len { break; }
            let cp = ((b as u32 & 0x07) << 18)
                | ((data[pos + 1] as u32 & 0x3f) << 12)
                | ((data[pos + 2] as u32 & 0x3f) << 6)
                | (data[pos + 3] as u32 & 0x3f);
            let cp = cp - 0x10000;
            result[out] = (0xd800 + (cp >> 10)) as u16;
            out += 1;
            result[out] = (0xdc00 + (cp & 0x3ff)) as u16;
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
            for i in pos..pos + 16 {
                if data[i] & 0x80 != 0 { all_ascii = false; break; }
            }
            if all_ascii {
                for i in pos..pos + 16 {
                    result[out] = data[i] as u32;
                    out += 1;
                }
                pos += 16;
                continue;
            }
        }
        let b = data[pos];
        if b < 0x80 {
            result[out] = b as u32;
            out += 1;
            pos += 1;
        } else if b & 0xe0 == 0xc0 {
            if pos + 1 >= len { return 0; }
            if data[pos + 1] & 0xc0 != 0x80 { return 0; }
            let cp = ((b as u32 & 0x1f) << 6) | (data[pos + 1] as u32 & 0x3f);
            if cp < 0x80 || cp > 0x7ff { return 0; }
            result[out] = cp;
            out += 1;
            pos += 2;
        } else if b & 0xf0 == 0xe0 {
            if pos + 2 >= len { return 0; }
            if data[pos + 1] & 0xc0 != 0x80 { return 0; }
            if data[pos + 2] & 0xc0 != 0x80 { return 0; }
            let cp = ((b as u32 & 0x0f) << 12)
                | ((data[pos + 1] as u32 & 0x3f) << 6)
                | (data[pos + 2] as u32 & 0x3f);
            if cp < 0x800 || cp > 0xffff || (cp > 0xd7ff && cp < 0xe000) { return 0; }
            result[out] = cp;
            out += 1;
            pos += 3;
        } else if b & 0xf8 == 0xf0 {
            if pos + 3 >= len { return 0; }
            if data[pos + 1] & 0xc0 != 0x80 { return 0; }
            if data[pos + 2] & 0xc0 != 0x80 { return 0; }
            if data[pos + 3] & 0xc0 != 0x80 { return 0; }
            let cp = ((b as u32 & 0x07) << 18)
                | ((data[pos + 1] as u32 & 0x3f) << 12)
                | ((data[pos + 2] as u32 & 0x3f) << 6)
                | (data[pos + 3] as u32 & 0x3f);
            if cp <= 0xffff || cp > 0x10ffff { return 0; }
            result[out] = cp;
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
            for i in pos..pos + 16 {
                if data[i] & 0x80 != 0 { all_ascii = false; break; }
            }
            if all_ascii {
                for i in pos..pos + 16 {
                    result[out] = data[i];
                    out += 1;
                }
                pos += 16;
                continue;
            }
        }
        let b = data[pos];
        if b < 0x80 {
            result[out] = b;
            out += 1;
            pos += 1;
        } else if b & 0xe0 == 0xc0 {
            if pos + 1 >= len { return 0; }
            if data[pos + 1] & 0xc0 != 0x80 { return 0; }
            let cp = ((b as u32 & 0x1f) << 6) | (data[pos + 1] as u32 & 0x3f);
            if cp < 0x80 || cp > 0xff { return 0; }
            result[out] = cp as u8;
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
        if word & 0xf800 == 0xd800 {
            if pos + 1 >= len { return false; }
            let diff = word.wrapping_sub(0xd800);
            if diff > 0x3ff { return false; }
            let word2 = data[pos + 1];
            let diff2 = word2.wrapping_sub(0xdc00);
            if diff2 > 0x3ff { return false; }
            pos += 2;
        } else {
            pos += 1;
        }
    }
    true
}

pub fn utf16_length_from_utf8(data: &[Utf8]) -> usize {
    let mut counter = 0usize;
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
    let mut counter = 0usize;
    for &v in data {
        counter += 1;
        counter += (v > 0xffff) as usize;
    }
    counter
}

pub fn utf16_length_from_latin1(_data: &[Latin1]) -> usize {
    _data.len()
}

pub fn utf16le_convert_to_utf8(data: &[Utf16], result: &mut [Utf8]) -> usize {
    let len = data.len();
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        if pos + 4 <= len {
            let mut all_ascii = true;
            for i in pos..pos + 4 {
                if data[i] & 0xff80 != 0 { all_ascii = false; break; }
            }
            if all_ascii {
                for i in pos..pos + 4 {
                    result[out] = data[i] as u8;
                    out += 1;
                }
                pos += 4;
                continue;
            }
        }
        let word = data[pos];
        if word & 0xff80 == 0 {
            result[out] = word as u8;
            out += 1;
            pos += 1;
        } else if word & 0xf800 == 0 {
            result[out] = ((word >> 6) | 0xc0) as u8;
            out += 1;
            result[out] = ((word & 0x3f) | 0x80) as u8;
            out += 1;
            pos += 1;
        } else if word & 0xf800 != 0xd800 {
            result[out] = ((word >> 12) | 0xe0) as u8;
            out += 1;
            result[out] = (((word >> 6) & 0x3f) | 0x80) as u8;
            out += 1;
            result[out] = ((word & 0x3f) | 0x80) as u8;
            out += 1;
            pos += 1;
        } else {
            let diff = word.wrapping_sub(0xd800);
            if pos + 1 >= len { return 0; }
            let word2 = data[pos + 1];
            let value = (diff as u32) * 1024 + (word2 as u32).wrapping_sub(0xdc00) + 0x10000;
            result[out] = ((value >> 18) | 0xf0) as u8;
            out += 1;
            result[out] = (((value >> 12) & 0x3f) | 0x80) as u8;
            out += 1;
            result[out] = (((value >> 6) & 0x3f) | 0x80) as u8;
            out += 1;
            result[out] = ((value & 0x3f) | 0x80) as u8;
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
        if word & 0xf800 != 0xd800 {
            result[out] = word as u32;
            out += 1;
            pos += 1;
        } else {
            let diff = word.wrapping_sub(0xd800);
            if diff > 0x3ff { return 0; }
            if pos + 1 >= len { return 0; }
            let word2 = data[pos + 1];
            let value = ((diff as u32) << 10) + (word2 as u32).wrapping_sub(0xdc00) + 0x10000;
            let diff2 = word2.wrapping_sub(0xdc00);
            if diff2 > 0x3ff { return 0; }
            result[out] = value;
            out += 1;
            pos += 2;
        }
    }
    out
}

pub fn utf16le_convert_to_latin1(data: &[Utf16], result: &mut [Latin1]) -> usize {
    let mut overflow: u16 = 0;
    let mut out = 0usize;
    for &word in data {
        overflow |= word;
        result[out] = (word & 0xff) as u8;
        out += 1;
    }
    if overflow & 0xff00 != 0 { return 0; }
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
    let mut counter = 0usize;
    for &b in data {
        if (b as i8) > -65 {
            counter += 1;
        }
    }
    counter
}

pub fn utf32_length_from_utf16le(data: &[Utf16]) -> usize {
    let mut counter = 0usize;
    for &word in data {
        counter += ((word & 0xfc00) != 0xdc00) as usize;
    }
    counter
}

pub fn utf32_length_from_latin1(_data: &[Latin1]) -> usize {
    _data.len()
}

pub fn utf32_convert_to_utf8(data: &[Utf32], result: &mut [Utf8]) -> usize {
    let len = data.len();
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        if pos + 2 <= len && (data[pos] | data[pos + 1]) & 0xffffff80 == 0 {
            result[out] = data[pos] as u8;
            out += 1;
            result[out] = data[pos + 1] as u8;
            out += 1;
            pos += 2;
            continue;
        }
        let word = data[pos];
        if word & 0xffffff80 == 0 {
            result[out] = word as u8;
            out += 1;
        } else if word & 0xfffff800 == 0 {
            result[out] = ((word >> 6) | 0xc0) as u8;
            out += 1;
            result[out] = ((word & 0x3f) | 0x80) as u8;
            out += 1;
        } else if word & 0xffff0000 == 0 {
            if word >= 0xd800 && word <= 0xdfff { return 0; }
            result[out] = ((word >> 12) | 0xe0) as u8;
            out += 1;
            result[out] = (((word >> 6) & 0x3f) | 0x80) as u8;
            out += 1;
            result[out] = ((word & 0x3f) | 0x80) as u8;
            out += 1;
        } else {
            if word > 0x10ffff { return 0; }
            result[out] = ((word >> 18) | 0xf0) as u8;
            out += 1;
            result[out] = (((word >> 12) & 0x3f) | 0x80) as u8;
            out += 1;
            result[out] = (((word >> 6) & 0x3f) | 0x80) as u8;
            out += 1;
            result[out] = ((word & 0x3f) | 0x80) as u8;
            out += 1;
        }
        pos += 1;
    }
    out
}

pub fn utf32_convert_to_utf16le(data: &[Utf32], result: &mut [Utf16]) -> usize {
    let len = data.len();
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        let word = data[pos];
        if word & 0xffff0000 == 0 {
            if word >= 0xd800 && word <= 0xdfff { return 0; }
            result[out] = word as u16;
            out += 1;
        } else {
            if word > 0x10ffff { return 0; }
            let w = word - 0x10000;
            result[out] = (0xd800 + (w >> 10)) as u16;
            out += 1;
            result[out] = (0xdc00 + (w & 0x3ff)) as u16;
            out += 1;
        }
        pos += 1;
    }
    out
}

pub fn utf32_convert_to_latin1(data: &[Utf32], result: &mut [Latin1]) -> usize {
    let mut overflow: u32 = 0;
    let mut out = 0usize;
    for &word in data {
        overflow |= word;
        result[out] = (word & 0xff) as u8;
        out += 1;
    }
    if overflow & 0xffffff00 != 0 { return 0; }
    out
}

pub fn latin1_length_from_utf8(data: &[Utf8]) -> usize {
    let mut counter = 0usize;
    for &b in data {
        if (b as i8) > -65 {
            counter += 1;
        }
    }
    counter
}

pub fn latin1_length_from_utf16le(_data: &[Utf16]) -> usize {
    _data.len()
}

pub fn latin1_length_from_utf32(_data: &[Utf32]) -> usize {
    _data.len()
}

pub fn latin1_convert_to_utf8(data: &[Latin1], result: &mut [Utf8]) -> usize {
    let len = data.len();
    let mut pos = 0usize;
    let mut out = 0usize;

    while pos < len {
        if pos + 16 <= len {
            let mut all_ascii = true;
            for i in pos..pos + 16 {
                if data[i] & 0x80 != 0 { all_ascii = false; break; }
            }
            if all_ascii {
                for i in pos..pos + 16 {
                    result[out] = data[i];
                    out += 1;
                }
                pos += 16;
                continue;
            }
        }
        let byte = data[pos];
        if byte & 0x80 == 0 {
            result[out] = byte;
            out += 1;
        } else {
            result[out] = (byte >> 6) | 0xc0;
            out += 1;
            result[out] = (byte & 0x3f) | 0x80;
            out += 1;
        }
        pos += 1;
    }
    out
}

pub fn latin1_convert_to_utf16le(data: &[Latin1], result: &mut [Utf16]) -> usize {
    for (i, &b) in data.iter().enumerate() {
        result[i] = b as u16;
    }
    data.len()
}

pub fn latin1_convert_to_utf32(data: &[Latin1], result: &mut [Utf32]) -> usize {
    for (i, &b) in data.iter().enumerate() {
        result[i] = b as u32;
    }
    data.len()
}

pub fn ascii_validate(data: &[Ascii]) -> bool {
    for &b in data {
        if b >= 0x80 { return false; }
    }
    true
}
