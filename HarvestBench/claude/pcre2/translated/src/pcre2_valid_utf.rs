use crate::pcre2_internal::*;

// UTF-8 validation. Error codes:
const PCRE2_ERROR_UTF8_ERR2: i32 = -4;
const PCRE2_ERROR_UTF8_ERR3: i32 = -5;
const PCRE2_ERROR_UTF8_ERR4: i32 = -6;
const PCRE2_ERROR_UTF8_ERR5: i32 = -7;
const PCRE2_ERROR_UTF8_ERR6: i32 = -8;
const PCRE2_ERROR_UTF8_ERR7: i32 = -9;
const PCRE2_ERROR_UTF8_ERR8: i32 = -10;
const PCRE2_ERROR_UTF8_ERR9: i32 = -11;
const PCRE2_ERROR_UTF8_ERR10: i32 = -12;
const PCRE2_ERROR_UTF8_ERR11: i32 = -13;
const PCRE2_ERROR_UTF8_ERR12: i32 = -14;
const PCRE2_ERROR_UTF8_ERR13: i32 = -15;
const PCRE2_ERROR_UTF8_ERR14: i32 = -16;
const PCRE2_ERROR_UTF8_ERR15: i32 = -17;
const PCRE2_ERROR_UTF8_ERR16: i32 = -18;
const PCRE2_ERROR_UTF8_ERR17: i32 = -19;
const PCRE2_ERROR_UTF8_ERR18: i32 = -20;
const PCRE2_ERROR_UTF8_ERR19: i32 = -21;
const PCRE2_ERROR_UTF8_ERR20: i32 = -22;
const PCRE2_ERROR_UTF8_ERR1_: i32 = -3;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_valid_utf_8(
    string: PCRE2_SPTR,
    mut length: PCRE2_SIZE,
    erroroffset: *mut PCRE2_SIZE,
) -> i32 {
    let mut p = string;
    while length > 0 {
        let c = *p as u32;
        length -= 1;

        if c < 128 {
            p = p.add(1);
            continue;
        }

        if c < 0xc0 {
            *erroroffset = p.offset_from(string) as PCRE2_SIZE;
            return PCRE2_ERROR_UTF8_ERR20;
        }

        if c >= 0xfe {
            *erroroffset = p.offset_from(string) as PCRE2_SIZE;
            return PCRE2_ERROR_UTF8_ERR21;
        }

        let ab = _pcre2_utf8_table4[(c & 0x3f) as usize] as usize;
        if length < ab {
            *erroroffset = p.offset_from(string) as PCRE2_SIZE;
            match ab - length {
                1 => return PCRE2_ERROR_UTF8_ERR1_,
                2 => return PCRE2_ERROR_UTF8_ERR2,
                3 => return PCRE2_ERROR_UTF8_ERR3,
                4 => return PCRE2_ERROR_UTF8_ERR4,
                5 => return PCRE2_ERROR_UTF8_ERR5,
                _ => {}
            }
        }
        length -= ab;

        // Second byte
        p = p.add(1);
        let d = *p as u32;
        if (d & 0xc0) != 0x80 {
            *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(1);
            return PCRE2_ERROR_UTF8_ERR6;
        }

        match ab {
            1 => {
                if (c & 0x3e) == 0 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(1);
                    return PCRE2_ERROR_UTF8_ERR15;
                }
            }
            2 => {
                p = p.add(1);
                if (*p as u32 & 0xc0) != 0x80 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(2);
                    return PCRE2_ERROR_UTF8_ERR7;
                }
                if c == 0xe0 && (d & 0x20) == 0 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(2);
                    return PCRE2_ERROR_UTF8_ERR16;
                }
                if c == 0xed && d >= 0xa0 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(2);
                    return PCRE2_ERROR_UTF8_ERR14;
                }
            }
            3 => {
                p = p.add(1);
                if (*p as u32 & 0xc0) != 0x80 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(2);
                    return PCRE2_ERROR_UTF8_ERR7;
                }
                p = p.add(1);
                if (*p as u32 & 0xc0) != 0x80 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(3);
                    return PCRE2_ERROR_UTF8_ERR8;
                }
                if c == 0xf0 && (d & 0x30) == 0 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(3);
                    return PCRE2_ERROR_UTF8_ERR17;
                }
                if c > 0xf4 || (c == 0xf4 && d > 0x8f) {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(3);
                    return PCRE2_ERROR_UTF8_ERR13;
                }
            }
            4 => {
                p = p.add(1);
                if (*p as u32 & 0xc0) != 0x80 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(2);
                    return PCRE2_ERROR_UTF8_ERR7;
                }
                p = p.add(1);
                if (*p as u32 & 0xc0) != 0x80 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(3);
                    return PCRE2_ERROR_UTF8_ERR8;
                }
                p = p.add(1);
                if (*p as u32 & 0xc0) != 0x80 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(4);
                    return PCRE2_ERROR_UTF8_ERR9;
                }
                if c == 0xf8 && (d & 0x38) == 0 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(4);
                    return PCRE2_ERROR_UTF8_ERR18;
                }
            }
            5 => {
                p = p.add(1);
                if (*p as u32 & 0xc0) != 0x80 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(2);
                    return PCRE2_ERROR_UTF8_ERR7;
                }
                p = p.add(1);
                if (*p as u32 & 0xc0) != 0x80 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(3);
                    return PCRE2_ERROR_UTF8_ERR8;
                }
                p = p.add(1);
                if (*p as u32 & 0xc0) != 0x80 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(4);
                    return PCRE2_ERROR_UTF8_ERR9;
                }
                p = p.add(1);
                if (*p as u32 & 0xc0) != 0x80 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(5);
                    return PCRE2_ERROR_UTF8_ERR10;
                }
                if c == 0xfc && (d & 0x3c) == 0 {
                    *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(5);
                    return PCRE2_ERROR_UTF8_ERR19;
                }
            }
            _ => {}
        }

        if ab > 3 {
            *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(ab);
            return if ab == 4 { PCRE2_ERROR_UTF8_ERR11 } else { PCRE2_ERROR_UTF8_ERR12 };
        }

        p = p.add(1);
    }
    0
}
