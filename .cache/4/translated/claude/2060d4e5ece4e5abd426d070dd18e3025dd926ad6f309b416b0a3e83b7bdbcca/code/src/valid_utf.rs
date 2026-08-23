// Translated from pcre2_valid_utf.c
//
// This module contains an internal function for validating UTF character
// strings. Only the 8-bit branch is present in this build.

use crate::internal::*;
use crate::pcre2_pub::*;
use crate::tables::*;
use core::ffi::c_int;

/*************************************************
*           Validate a UTF string                *
*************************************************/

/* This function is called (optionally) at the start of compile or match, to
check that a supposed UTF string is actually valid. The early check means
that subsequent code can assume it is dealing with a valid string. The check
can be turned off for maximum performance, but the consequences of supplying an
invalid string are then undefined.

Arguments:
  string       points to the string
  length       length of string
  errp         pointer to an error position offset variable

Returns:       == 0    if the string is a valid UTF string
               != 0    otherwise, setting the offset of the bad character
*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn _pcre2_valid_utf_8(
    string: PCRE2_SPTR,
    length: PCRE2_SIZE,
    erroroffset: *mut PCRE2_SIZE,
) -> c_int {
    let mut p: PCRE2_SPTR;
    let mut c: u32;
    let mut length: PCRE2_SIZE = length;

    /* ----------------- Check a UTF-8 string ----------------- */

    /* Originally, this function checked according to RFC 2279, allowing for values
    in the range 0 to 0x7fffffff, up to 6 bytes long, but ensuring that they were
    in the canonical format. Once somebody had pointed out RFC 3629 to me (it
    obsoletes 2279), additional restrictions were applied. */

    p = string;
    while length > 0 {
        'continue_loop: {
            let ab: u32;
            let d: u32;

            c = *p as u32;
            length = length.wrapping_sub(1);

            if c < 128 {
                break 'continue_loop;
            } /* ASCII character */

            if c < 0xc0
            /* Isolated 10xx xxxx byte */
            {
                *erroroffset = p.offset_from(string) as PCRE2_SIZE;
                return PCRE2_ERROR_UTF8_ERR20;
            }

            if c >= 0xfe
            /* Invalid 0xfe or 0xff bytes */
            {
                *erroroffset = p.offset_from(string) as PCRE2_SIZE;
                return PCRE2_ERROR_UTF8_ERR21;
            }

            ab = _pcre2_utf8_table4[(c & 0x3f) as usize] as u32; /* Number of additional bytes (1-5) */
            if length < ab as PCRE2_SIZE
            /* Missing bytes */
            {
                *erroroffset = p.offset_from(string) as PCRE2_SIZE;
                match (ab as PCRE2_SIZE).wrapping_sub(length) {
                    1 => return PCRE2_ERROR_UTF8_ERR1,
                    2 => return PCRE2_ERROR_UTF8_ERR2,
                    3 => return PCRE2_ERROR_UTF8_ERR3,
                    4 => return PCRE2_ERROR_UTF8_ERR4,
                    5 => return PCRE2_ERROR_UTF8_ERR5,
                    _ => {}
                }
            }
            length = length.wrapping_sub(ab as PCRE2_SIZE); /* Length remaining */

            /* Check top bits in the second byte */

            p = p.add(1);
            d = *p as u32;
            if (d & 0xc0) != 0x80 {
                *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(1);
                return PCRE2_ERROR_UTF8_ERR6;
            }

            /* For each length, check that the remaining bytes start with the 0x80 bit
            set and not the 0x40 bit. Then check for an overlong sequence, and for the
            excluded range 0xd800 to 0xdfff. */

            match ab {
                /* 2-byte character. No further bytes to check for 0x80. Check first byte
                for for xx00 000x (overlong sequence). */
                1 => {
                    if (c & 0x3e) == 0 {
                        *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(1);
                        return PCRE2_ERROR_UTF8_ERR15;
                    }
                }

                /* 3-byte character. Check third byte for 0x80. Then check first 2 bytes
                  for 1110 0000, xx0x xxxx (overlong sequence) or
                      1110 1101, 1010 xxxx (0xd800 - 0xdfff) */
                2 => {
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80
                    /* Third byte */
                    {
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

                /* 4-byte character. Check 3rd and 4th bytes for 0x80. Then check first 2
                   bytes for for 1111 0000, xx00 xxxx (overlong sequence), then check for a
                   character greater than 0x0010ffff (f4 8f bf bf) */
                3 => {
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80
                    /* Third byte */
                    {
                        *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(2);
                        return PCRE2_ERROR_UTF8_ERR7;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80
                    /* Fourth byte */
                    {
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

                /* 5-byte and 6-byte characters are not allowed by RFC 3629, and will be
                rejected by the length test below. However, we do the appropriate tests
                here so that overlong sequences get diagnosed, and also in case there is
                ever an option for handling these larger code points. */

                /* 5-byte character. Check 3rd, 4th, and 5th bytes for 0x80. Then check for
                1111 1000, xx00 0xxx */
                4 => {
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80
                    /* Third byte */
                    {
                        *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(2);
                        return PCRE2_ERROR_UTF8_ERR7;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80
                    /* Fourth byte */
                    {
                        *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(3);
                        return PCRE2_ERROR_UTF8_ERR8;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80
                    /* Fifth byte */
                    {
                        *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(4);
                        return PCRE2_ERROR_UTF8_ERR9;
                    }
                    if c == 0xf8 && (d & 0x38) == 0 {
                        *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(4);
                        return PCRE2_ERROR_UTF8_ERR18;
                    }
                }

                /* 6-byte character. Check 3rd-6th bytes for 0x80. Then check for
                1111 1100, xx00 00xx. */
                5 => {
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80
                    /* Third byte */
                    {
                        *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(2);
                        return PCRE2_ERROR_UTF8_ERR7;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80
                    /* Fourth byte */
                    {
                        *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(3);
                        return PCRE2_ERROR_UTF8_ERR8;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80
                    /* Fifth byte */
                    {
                        *erroroffset = (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(4);
                        return PCRE2_ERROR_UTF8_ERR9;
                    }
                    p = p.add(1);
                    if (*p as u32 & 0xc0) != 0x80
                    /* Sixth byte */
                    {
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

            /* Character is valid under RFC 2279, but 4-byte and 5-byte characters are
            excluded by RFC 3629. The pointer p is currently at the last byte of the
            character. */

            if ab > 3 {
                *erroroffset =
                    (p.offset_from(string) as PCRE2_SIZE).wrapping_sub(ab as PCRE2_SIZE);
                return if ab == 4 {
                    PCRE2_ERROR_UTF8_ERR11
                } else {
                    PCRE2_ERROR_UTF8_ERR12
                };
            }
        }
        p = p.add(1);
    }
    0
}

/* End of pcre2_valid_utf.c */
