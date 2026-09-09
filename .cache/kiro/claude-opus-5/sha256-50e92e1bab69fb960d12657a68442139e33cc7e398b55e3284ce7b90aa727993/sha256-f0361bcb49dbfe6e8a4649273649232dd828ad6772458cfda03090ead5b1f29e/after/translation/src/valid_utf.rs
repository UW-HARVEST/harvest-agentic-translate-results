//! Translation of `pcre2_valid_utf.c` (8-bit / UTF-8, `SUPPORT_UNICODE` on).

use crate::internal::*;
use crate::tables;
use core::ffi::c_int;

/// `PRIV(valid_utf)` — validate a UTF-8 string.
///
/// Returns `0` if the string is a valid UTF-8 string, otherwise a non-zero
/// error code, setting `*erroroffset` to the offset of the bad character.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_valid_utf_8(
    string: PCRE2_SPTR,
    length: PCRE2_SIZE,
    erroroffset: *mut PCRE2_SIZE,
) -> c_int {
    unsafe {
        let mut p = string;
        let mut length = length;

        while length > 0 {
            let ab: u32;
            let d: u32;

            let c = *p as u32;
            length -= 1;

            if c < 128 {
                // ASCII character
                p = p.add(1);
                continue;
            }

            if c < 0xc0 {
                // Isolated 10xx xxxx byte
                *erroroffset = p.offset_from(string) as PCRE2_SIZE;
                return PCRE2_ERROR_UTF8_ERR20 as c_int;
            }

            if c >= 0xfe {
                // Invalid 0xfe or 0xff bytes
                *erroroffset = p.offset_from(string) as PCRE2_SIZE;
                return PCRE2_ERROR_UTF8_ERR21 as c_int;
            }

            // Number of additional bytes (1-5)
            ab = tables::_pcre2_utf8_table4[(c & 0x3f) as usize] as u32;
            if (length as u32) < ab {
                // Missing bytes
                *erroroffset = p.offset_from(string) as PCRE2_SIZE;
                match ab - length as u32 {
                    1 => return PCRE2_ERROR_UTF8_ERR1 as c_int,
                    2 => return PCRE2_ERROR_UTF8_ERR2 as c_int,
                    3 => return PCRE2_ERROR_UTF8_ERR3 as c_int,
                    4 => return PCRE2_ERROR_UTF8_ERR4 as c_int,
                    5 => return PCRE2_ERROR_UTF8_ERR5 as c_int,
                    _ => {}
                }
            }
            length -= ab as PCRE2_SIZE; // Length remaining

            // Check top bits in the second byte
            p = p.add(1);
            d = *p as u32;
            if (d & 0xc0) != 0x80 {
                *erroroffset = p.offset_from(string) as PCRE2_SIZE - 1;
                return PCRE2_ERROR_UTF8_ERR6 as c_int;
            }

            // For each length, check that the remaining bytes start with the
            // 0x80 bit set and not the 0x40 bit. Then check for an overlong
            // sequence, and for the excluded range 0xd800 to 0xdfff.
            match ab {
                // 2-byte character. No further bytes to check for 0x80. Check
                // first byte for xx00 000x (overlong sequence).
                1 => {
                    if (c & 0x3e) == 0 {
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 1;
                        return PCRE2_ERROR_UTF8_ERR15 as c_int;
                    }
                }

                // 3-byte character. Check third byte for 0x80. Then check first
                // 2 bytes for 1110 0000, xx0x xxxx (overlong sequence) or
                // 1110 1101, 1010 xxxx (0xd800 - 0xdfff).
                2 => {
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80 {
                        // Third byte
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 2;
                        return PCRE2_ERROR_UTF8_ERR7 as c_int;
                    }
                    if c == 0xe0 && (d & 0x20) == 0 {
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 2;
                        return PCRE2_ERROR_UTF8_ERR16 as c_int;
                    }
                    if c == 0xed && d >= 0xa0 {
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 2;
                        return PCRE2_ERROR_UTF8_ERR14 as c_int;
                    }
                }

                // 4-byte character. Check 3rd and 4th bytes for 0x80. Then
                // check first 2 bytes for 1111 0000, xx00 xxxx (overlong
                // sequence), then check for a character greater than 0x0010ffff
                // (f4 8f bf bf).
                3 => {
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80 {
                        // Third byte
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 2;
                        return PCRE2_ERROR_UTF8_ERR7 as c_int;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80 {
                        // Fourth byte
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 3;
                        return PCRE2_ERROR_UTF8_ERR8 as c_int;
                    }
                    if c == 0xf0 && (d & 0x30) == 0 {
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 3;
                        return PCRE2_ERROR_UTF8_ERR17 as c_int;
                    }
                    if c > 0xf4 || (c == 0xf4 && d > 0x8f) {
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 3;
                        return PCRE2_ERROR_UTF8_ERR13 as c_int;
                    }
                }

                // 5-byte character. Check 3rd, 4th, and 5th bytes for 0x80.
                // Then check for 1111 1000, xx00 0xxx.
                4 => {
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80 {
                        // Third byte
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 2;
                        return PCRE2_ERROR_UTF8_ERR7 as c_int;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80 {
                        // Fourth byte
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 3;
                        return PCRE2_ERROR_UTF8_ERR8 as c_int;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80 {
                        // Fifth byte
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 4;
                        return PCRE2_ERROR_UTF8_ERR9 as c_int;
                    }
                    if c == 0xf8 && (d & 0x38) == 0 {
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 4;
                        return PCRE2_ERROR_UTF8_ERR18 as c_int;
                    }
                }

                // 6-byte character. Check 3rd-6th bytes for 0x80. Then check
                // for 1111 1100, xx00 00xx.
                5 => {
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80 {
                        // Third byte
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 2;
                        return PCRE2_ERROR_UTF8_ERR7 as c_int;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80 {
                        // Fourth byte
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 3;
                        return PCRE2_ERROR_UTF8_ERR8 as c_int;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80 {
                        // Fifth byte
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 4;
                        return PCRE2_ERROR_UTF8_ERR9 as c_int;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80 {
                        // Sixth byte
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 5;
                        return PCRE2_ERROR_UTF8_ERR10 as c_int;
                    }
                    if c == 0xfc && (d & 0x3c) == 0 {
                        *erroroffset = p.offset_from(string) as PCRE2_SIZE - 5;
                        return PCRE2_ERROR_UTF8_ERR19 as c_int;
                    }
                }

                _ => {}
            }

            // Character is valid under RFC 2279, but 4-byte and 5-byte
            // characters are excluded by RFC 3629. The pointer p is currently
            // at the last byte of the character.
            if ab > 3 {
                *erroroffset = p.offset_from(string) as PCRE2_SIZE - ab as PCRE2_SIZE;
                return if ab == 4 {
                    PCRE2_ERROR_UTF8_ERR11 as c_int
                } else {
                    PCRE2_ERROR_UTF8_ERR12 as c_int
                };
            }

            p = p.add(1);
        }

        0
    }
}
