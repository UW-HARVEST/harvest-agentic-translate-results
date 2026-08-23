//! `sodium/codecs.c`

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::common::{EINVAL, ERANGE, SIZE_MAX, memchr, memcmp, set_errno, strchr};
use crate::sodium::core::sodium_misuse;

pub const SODIUM_BASE64_VARIANT_ORIGINAL: c_int = 1;
pub const SODIUM_BASE64_VARIANT_ORIGINAL_NO_PADDING: c_int = 3;
pub const SODIUM_BASE64_VARIANT_URLSAFE: c_int = 5;
pub const SODIUM_BASE64_VARIANT_URLSAFE_NO_PADDING: c_int = 7;

const VARIANT_NO_PADDING_MASK: u32 = 0x2;
const VARIANT_URLSAFE_MASK: u32 = 0x4;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    let mut i: usize = 0;

    if bin_len >= SIZE_MAX / 2 || hex_maxlen <= bin_len * 2 {
        sodium_misuse();
    }
    while i < bin_len {
        let b8 = unsafe { *bin.add(i) };
        let c = (b8 & 0xf) as u32;
        let b = (b8 >> 4) as u32;
        let lo = (87u32.wrapping_add(c).wrapping_add(c.wrapping_sub(10) >> 8 & !38u32)) as u8;
        let hi = (87u32.wrapping_add(b).wrapping_add(b.wrapping_sub(10) >> 8 & !38u32)) as u8;
        let mut x: u32 = ((lo as u32) << 8) | (hi as u32);
        unsafe { *hex.add(i * 2) = x as u8 as c_char };
        x >>= 8;
        unsafe { *hex.add(i * 2 + 1) = x as u8 as c_char };
        i += 1;
    }
    unsafe { *hex.add(i * 2) = 0 };

    hex
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_hex2bin(
    bin: *mut u8,
    bin_maxlen: usize,
    hex: *const c_char,
    hex_len: usize,
    ignore: *const c_char,
    bin_len: *mut usize,
    hex_end: *mut *const c_char,
) -> c_int {
    let mut bin_pos: usize = 0;
    let mut hex_pos: usize = 0;
    let mut ret: c_int = 0;
    let mut c_acc: u8 = 0;
    let mut state: u8 = 0;

    while hex_pos < hex_len {
        let c = unsafe { *hex.add(hex_pos) } as u8;
        let c_num = c ^ 48u8;
        let c_num0 = (((c_num as u32).wrapping_sub(10)) >> 8) as u8;
        let c_alpha = (c & !32u8).wrapping_sub(55);
        let c_alpha0 = ((((c_alpha as u32).wrapping_sub(10)) ^ ((c_alpha as u32).wrapping_sub(16)))
            >> 8) as u8;
        if (c_num0 | c_alpha0) == 0 {
            if !ignore.is_null()
                && state == 0
                && !unsafe { strchr(ignore, c as c_int) }.is_null()
            {
                hex_pos += 1;
                continue;
            }
            break;
        }
        let c_val = (c_num0 & c_num) | (c_alpha0 & c_alpha);
        if bin_pos >= bin_maxlen {
            ret = -1;
            set_errno(ERANGE);
            break;
        }
        if state == 0 {
            c_acc = c_val.wrapping_mul(16);
        } else {
            unsafe { *bin.add(bin_pos) = c_acc | c_val };
            bin_pos += 1;
        }
        state = !state;
        hex_pos += 1;
    }
    if state != 0 {
        hex_pos = hex_pos.wrapping_sub(1);
        set_errno(EINVAL);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if !hex_end.is_null() {
        unsafe { *hex_end = hex.add(hex_pos) };
    } else if hex_pos != hex_len {
        set_errno(EINVAL);
        ret = -1;
    }
    if !bin_len.is_null() {
        unsafe { *bin_len = bin_pos };
    }
    ret
}

// Constant-time comparison macros over 0..255; 0x00 = false, 0xFF = true.
#[inline(always)]
const fn eq(x: u32, y: u32) -> u32 {
    ((((0u32.wrapping_sub(x ^ y)) >> 8) & 0xFF) ^ 0xFF)
}

#[inline(always)]
const fn gt(x: u32, y: u32) -> u32 {
    ((y.wrapping_sub(x)) >> 8) & 0xFF
}

#[inline(always)]
const fn ge(x: u32, y: u32) -> u32 {
    gt(y, x) ^ 0xFF
}

#[inline(always)]
const fn lt(x: u32, y: u32) -> u32 {
    gt(y, x)
}

#[inline(always)]
const fn le(x: u32, y: u32) -> u32 {
    ge(y, x)
}

fn b64_byte_to_char(x: u32) -> c_int {
    ((lt(x, 26) & (x.wrapping_add(b'A' as u32)))
        | (ge(x, 26) & lt(x, 52) & (x.wrapping_add((b'a' as u32).wrapping_sub(26))))
        | (ge(x, 52) & lt(x, 62) & (x.wrapping_add((b'0' as u32).wrapping_sub(52))))
        | (eq(x, 62) & (b'+' as u32))
        | (eq(x, 63) & (b'/' as u32))) as c_int
}

fn b64_char_to_byte(c_: c_int) -> u32 {
    let c = (c_ as u8) as u32;
    let x = (ge(c, b'A' as u32) & le(c, b'Z' as u32) & (c.wrapping_sub(b'A' as u32)))
        | (ge(c, b'a' as u32)
            & le(c, b'z' as u32)
            & (c.wrapping_sub((b'a' as u32).wrapping_sub(26))))
        | (ge(c, b'0' as u32)
            & le(c, b'9' as u32)
            & (c.wrapping_sub((b'0' as u32).wrapping_sub(52))))
        | (eq(c, b'+' as u32) & 62)
        | (eq(c, b'/' as u32) & 63);

    x | (eq(x, 0) & (eq(c, b'A' as u32) ^ 0xFF))
}

fn b64_byte_to_urlsafe_char(x: u32) -> c_int {
    ((lt(x, 26) & (x.wrapping_add(b'A' as u32)))
        | (ge(x, 26) & lt(x, 52) & (x.wrapping_add((b'a' as u32).wrapping_sub(26))))
        | (ge(x, 52) & lt(x, 62) & (x.wrapping_add((b'0' as u32).wrapping_sub(52))))
        | (eq(x, 62) & (b'-' as u32))
        | (eq(x, 63) & (b'_' as u32))) as c_int
}

fn b64_urlsafe_char_to_byte(c_: c_int) -> u32 {
    let c = (c_ as u8) as u32;
    let x = (ge(c, b'A' as u32) & le(c, b'Z' as u32) & (c.wrapping_sub(b'A' as u32)))
        | (ge(c, b'a' as u32)
            & le(c, b'z' as u32)
            & (c.wrapping_sub((b'a' as u32).wrapping_sub(26))))
        | (ge(c, b'0' as u32)
            & le(c, b'9' as u32)
            & (c.wrapping_sub((b'0' as u32).wrapping_sub(52))))
        | (eq(c, b'-' as u32) & 62)
        | (eq(c, b'_' as u32) & 63);

    x | (eq(x, 0) & (eq(c, b'A' as u32) ^ 0xFF))
}

fn sodium_base64_check_variant(variant: c_int) {
    if ((variant as u32) & !0x6u32) != 0x1 {
        sodium_misuse();
    }
}

/// `sodium_base64_ENCODED_LEN(BIN_LEN, VARIANT)`
///
/// ```text
/// (BIN_LEN)/3U > (SIZE_MAX - 5)/4U ? (size_t) SIZE_MAX
///   : (((BIN_LEN)/3U)*4U +
///      (((rem) | ((rem) >> 1)) & 1U) *
///        (4U - ((0U - (((VARIANT) & 2U) >> 1)) & (3U - (rem)))) + 1U)
/// ```
/// with `rem = (BIN_LEN) - ((BIN_LEN)/3U)*3U`.
#[inline]
pub fn sodium_base64_encoded_len_macro(bin_len: usize, variant: c_int) -> usize {
    if bin_len / 3 > (SIZE_MAX - 5) / 4 {
        return SIZE_MAX;
    }
    let rem = bin_len - (bin_len / 3) * 3;
    let flag = (rem | (rem >> 1)) & 1;
    // `0U - ((VARIANT & 2U) >> 1)` is computed in `unsigned int` and then
    // widened to size_t, so only the low 32 bits are set.
    let mask = (0u32.wrapping_sub(((variant as u32) & 2) >> 1)) as usize;
    (bin_len / 3) * 4 + flag * (4usize.wrapping_sub(mask & 3usize.wrapping_sub(rem))) + 1
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_base64_encoded_len(bin_len: usize, variant: c_int) -> usize {
    sodium_base64_check_variant(variant);

    if bin_len / 3 > (SIZE_MAX - 5) / 4 {
        sodium_misuse();
    }
    sodium_base64_encoded_len_macro(bin_len, variant)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_bin2base64(
    b64: *mut c_char,
    b64_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
    variant: c_int,
) -> *mut c_char {
    let mut acc_len: usize = 0;
    let mut b64_pos: usize = 0;
    let mut bin_pos: usize = 0;
    let mut acc: u32 = 0;

    sodium_base64_check_variant(variant);
    let nibbles = bin_len / 3;
    if nibbles > (SIZE_MAX - 5) / 4 {
        sodium_misuse();
    }
    let remainder = bin_len - 3 * nibbles;
    let mut b64_len = nibbles * 4;
    if remainder != 0 {
        if ((variant as u32) & VARIANT_NO_PADDING_MASK) == 0 {
            b64_len += 4;
        } else {
            b64_len += 2 + (remainder >> 1);
        }
    }
    if b64_maxlen <= b64_len {
        sodium_misuse();
    }
    if ((variant as u32) & VARIANT_URLSAFE_MASK) != 0 {
        while bin_pos < bin_len {
            acc = (acc << 8) + unsafe { *bin.add(bin_pos) } as u32;
            bin_pos += 1;
            acc_len += 8;
            while acc_len >= 6 {
                acc_len -= 6;
                unsafe {
                    *b64.add(b64_pos) =
                        b64_byte_to_urlsafe_char((acc >> acc_len) & 0x3F) as u8 as c_char
                };
                b64_pos += 1;
            }
        }
        if acc_len > 0 {
            unsafe {
                *b64.add(b64_pos) =
                    b64_byte_to_urlsafe_char((acc << (6 - acc_len)) & 0x3F) as u8 as c_char
            };
            b64_pos += 1;
        }
    } else {
        while bin_pos < bin_len {
            acc = (acc << 8) + unsafe { *bin.add(bin_pos) } as u32;
            bin_pos += 1;
            acc_len += 8;
            while acc_len >= 6 {
                acc_len -= 6;
                unsafe {
                    *b64.add(b64_pos) = b64_byte_to_char((acc >> acc_len) & 0x3F) as u8 as c_char
                };
                b64_pos += 1;
            }
        }
        if acc_len > 0 {
            unsafe {
                *b64.add(b64_pos) = b64_byte_to_char((acc << (6 - acc_len)) & 0x3F) as u8 as c_char
            };
            b64_pos += 1;
        }
    }
    while b64_pos < b64_len {
        unsafe { *b64.add(b64_pos) = b'=' as c_char };
        b64_pos += 1;
    }
    loop {
        unsafe { *b64.add(b64_pos) = 0 };
        b64_pos += 1;
        if b64_pos >= b64_maxlen {
            break;
        }
    }

    b64
}

unsafe fn sodium_base642bin_skip_padding(
    b64: *const c_char,
    b64_len: usize,
    b64_pos_p: &mut usize,
    ignore: *const c_char,
    mut padding_len: usize,
) -> c_int {
    while padding_len > 0 {
        if *b64_pos_p >= b64_len {
            set_errno(ERANGE);
            return -1;
        }
        let c = unsafe { *b64.add(*b64_pos_p) };
        if c == b'=' as c_char {
            padding_len -= 1;
        } else if ignore.is_null() || unsafe { strchr(ignore, c as c_int) }.is_null() {
            set_errno(EINVAL);
            return -1;
        }
        *b64_pos_p += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_base642bin(
    bin: *mut u8,
    bin_maxlen: usize,
    b64: *const c_char,
    b64_len: usize,
    ignore: *const c_char,
    bin_len: *mut usize,
    b64_end: *mut *const c_char,
    variant: c_int,
) -> c_int {
    let mut acc_len: usize = 0;
    let mut b64_pos: usize = 0;
    let mut bin_pos: usize = 0;
    let mut ret: c_int = 0;
    let mut acc: u32 = 0;

    sodium_base64_check_variant(variant);
    let is_urlsafe = (variant as u32) & VARIANT_URLSAFE_MASK;
    while b64_pos < b64_len {
        let c = unsafe { *b64.add(b64_pos) };
        let d = if is_urlsafe != 0 {
            b64_urlsafe_char_to_byte(c as c_int)
        } else {
            b64_char_to_byte(c as c_int)
        };
        if d == 0xFF {
            if !ignore.is_null() && !unsafe { strchr(ignore, c as c_int) }.is_null() {
                b64_pos += 1;
                continue;
            }
            break;
        }
        acc = (acc << 6) + d;
        acc_len += 6;
        if acc_len >= 8 {
            acc_len -= 8;
            if bin_pos >= bin_maxlen {
                set_errno(ERANGE);
                ret = -1;
                break;
            }
            unsafe { *bin.add(bin_pos) = ((acc >> acc_len) & 0xFF) as u8 };
            bin_pos += 1;
        }
        b64_pos += 1;
    }
    if acc_len > 4 || (acc & ((1u32 << acc_len) - 1)) != 0 {
        ret = -1;
    } else if ret == 0 && ((variant as u32) & VARIANT_NO_PADDING_MASK) == 0 {
        ret = unsafe {
            sodium_base642bin_skip_padding(b64, b64_len, &mut b64_pos, ignore, acc_len / 2)
        };
    }
    if ret != 0 {
        bin_pos = 0;
    } else if !ignore.is_null() {
        while b64_pos < b64_len
            && !unsafe { strchr(ignore, *b64.add(b64_pos) as c_int) }.is_null()
        {
            b64_pos += 1;
        }
    }
    if !b64_end.is_null() {
        unsafe { *b64_end = b64.add(b64_pos) };
    } else if b64_pos != b64_len {
        set_errno(EINVAL);
        ret = -1;
    }
    if !bin_len.is_null() {
        unsafe { *bin_len = bin_pos };
    }
    ret
}

fn ip_hex_digit(ch: c_int) -> c_int {
    if ch >= b'0' as c_int && ch <= b'9' as c_int {
        return ch - b'0' as c_int;
    }
    let lower = (ch as u32) | 32u32;
    if lower >= b'a' as u32 && lower <= b'f' as u32 {
        return (lower - b'a' as u32 + 10) as c_int;
    }
    -1
}

unsafe fn parse_ipv4(src: *const c_char, end: *const c_char, out: *mut u8) -> c_int {
    let mut p = src;

    if src.is_null() || end.is_null() || out.is_null() || src >= end {
        return 0;
    }
    for i in 0..4usize {
        let mut val: u32 = 0;
        let mut digits: c_int = 0;

        while p < end && unsafe { *p } >= b'0' as c_char && unsafe { *p } <= b'9' as c_char {
            val = val * 10 + ((unsafe { *p } as u8) - b'0') as u32;
            p = unsafe { p.add(1) };
            digits += 1;
            if digits > 3 || val > 255 {
                return 0;
            }
        }
        if digits == 0 {
            return 0;
        }
        unsafe { *out.add(i) = val as u8 };

        if i < 3 {
            if p >= end {
                return 0;
            }
            let ch = unsafe { *p };
            p = unsafe { p.add(1) };
            if ch != b'.' as c_char {
                return 0;
            }
        }
    }
    (p == end) as c_int
}

unsafe fn parse_ipv6(src: *const c_char, end: *const c_char, out: *mut u8) -> c_int {
    let mut tmp = [0u8; 16];
    let tmp_base = tmp.as_mut_ptr();
    let mut tp = tmp_base;
    let endp = unsafe { tmp_base.add(16) };
    let mut colonp: *mut u8 = ptr::null_mut();
    let mut p = src;
    let mut curtok = src;
    let mut val: u32 = 0;
    let mut saw_xdigit: c_int = 0;
    let mut xdigits: c_int = 0;

    if src.is_null() || end.is_null() || out.is_null() || src >= end {
        return 0;
    }
    if unsafe { *p } == b':' as c_char {
        p = unsafe { p.add(1) };
        if p >= end || unsafe { *p } != b':' as c_char {
            return 0;
        }
        colonp = tp;
        p = unsafe { p.add(1) };
        curtok = p;
    }
    while p < end {
        let ch = unsafe { *p } as c_int;

        if ch == b':' as c_int {
            if saw_xdigit == 0 {
                if !colonp.is_null() {
                    return 0;
                }
                colonp = tp;
                p = unsafe { p.add(1) };
                curtok = p;
                continue;
            }
            if unsafe { tp.add(2) } > endp {
                return 0;
            }
            unsafe {
                *tp = (val >> 8) as u8;
                tp = tp.add(1);
                *tp = (val & 0xff) as u8;
                tp = tp.add(1);
            }
            val = 0;
            saw_xdigit = 0;
            xdigits = 0;
            p = unsafe { p.add(1) };
            curtok = p;
            if p >= end {
                return 0;
            }
            continue;
        }
        if ch == b'.' as c_int {
            if unsafe { tp.add(4) } > endp || unsafe { parse_ipv4(curtok, end, tp) } == 0 {
                return 0;
            }
            tp = unsafe { tp.add(4) };
            saw_xdigit = 0;
            break;
        }
        let hv = ip_hex_digit(ch);
        if hv < 0 || xdigits >= 4 {
            return 0;
        }
        val = (val << 4) | (hv as u32);
        saw_xdigit = 1;
        xdigits += 1;
        p = unsafe { p.add(1) };
    }
    if saw_xdigit != 0 {
        if unsafe { tp.add(2) } > endp {
            return 0;
        }
        unsafe {
            *tp = (val >> 8) as u8;
            tp = tp.add(1);
            *tp = (val & 0xff) as u8;
            tp = tp.add(1);
        }
    }
    if !colonp.is_null() {
        let n = (tp as usize) - (colonp as usize);

        if tp == endp {
            return 0;
        }
        unsafe {
            ptr::copy(colonp, endp.sub(n), n);
            let zlen = (endp as usize) - n - (colonp as usize);
            if zlen != 0 {
                ptr::write_bytes(colonp, 0, zlen);
            }
        }
        tp = endp;
    }
    if tp != endp {
        return 0;
    }
    unsafe { ptr::copy_nonoverlapping(tmp_base, out, 16) };

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_ip2bin(bin: *mut u8, ip: *const c_char, ip_len_: usize) -> c_int {
    let ip_end = unsafe { ip.add(ip_len_) };
    let mut end = ip;
    let mut v4 = [0u8; 4];

    while end < ip_end && unsafe { *end } != 0 {
        end = unsafe { end.add(1) };
    }
    let zone = unsafe {
        memchr(
            ip as *const c_void,
            b'%' as c_int,
            (end as usize) - (ip as usize),
        )
    } as *const c_char;
    if !zone.is_null() {
        let mut z = unsafe { zone.add(1) };
        while z < end {
            let c = unsafe { *z };
            let ok = (c >= b'0' as c_char && c <= b'9' as c_char)
                || (c >= b'a' as c_char && c <= b'z' as c_char)
                || (c >= b'A' as c_char && c <= b'Z' as c_char)
                || c == b'-' as c_char
                || c == b'_' as c_char
                || c == b'.' as c_char;
            if !ok {
                return -1;
            }
            z = unsafe { z.add(1) };
        }
        if unsafe { zone.add(1) } >= end {
            return -1;
        }
        end = zone;
    }
    let is_ipv6 = !unsafe {
        memchr(
            ip as *const c_void,
            b':' as c_int,
            (end as usize) - (ip as usize),
        )
    }
    .is_null();
    if !zone.is_null() && !is_ipv6 {
        return -1;
    }
    if is_ipv6 {
        return if unsafe { parse_ipv6(ip, end, bin) } != 0 {
            0
        } else {
            -1
        };
    }
    if unsafe { parse_ipv4(ip, end, v4.as_mut_ptr()) } == 0 {
        return -1;
    }
    unsafe {
        ptr::write_bytes(bin, 0, 10);
        *bin.add(10) = 0xff;
        *bin.add(11) = 0xff;
        ptr::copy_nonoverlapping(v4.as_ptr(), bin.add(12), 4);
    }

    0
}

const IPV4_MAPPED_PREFIX: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

fn ip_write_num(buf: &mut [u8; 46], p: &mut usize, mut val: u32, base: u32) {
    let mut tmp = [0u8; 4];
    let mut n = 0usize;

    loop {
        let d = val % base;
        tmp[n] = if d < 10 {
            (b'0' as u32 + d) as u8
        } else {
            (b'a' as u32 + d - 10) as u8
        };
        n += 1;
        val /= base;
        if val == 0 {
            break;
        }
    }

    while n > 0 {
        n -= 1;
        buf[*p] = tmp[n];
        *p += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_bin2ip(
    ip: *mut c_char,
    ip_maxlen: usize,
    bin: *const u8,
) -> *mut c_char {
    let mut buf = [0u8; 46];
    let mut p: usize = 0;
    let mut best_start: i32 = -1;
    let mut best_len: i32 = 0;
    let mut cur_start: i32 = -1;
    let mut cur_len: i32 = 0;

    if ip_maxlen <= 2 {
        return ptr::null_mut();
    }
    if unsafe {
        memcmp(
            bin as *const c_void,
            IPV4_MAPPED_PREFIX.as_ptr() as *const c_void,
            12,
        )
    } == 0
    {
        for i in 0..4usize {
            if i != 0 {
                buf[p] = b'.';
                p += 1;
            }
            ip_write_num(&mut buf, &mut p, unsafe { *bin.add(12 + i) } as u32, 10);
        }
        let len = p;
        if len >= ip_maxlen {
            return ptr::null_mut();
        }
        // The C code copies len + 1 bytes (the extra byte is uninitialised
        // stack) and then explicitly NUL-terminates, so the visible result is
        // the same as copying len bytes plus the terminator.
        unsafe {
            ptr::copy_nonoverlapping(buf.as_ptr(), ip as *mut u8, len + 1);
            *ip.add(len) = 0;
        }

        return ip;
    }
    for i in 0..8i32 {
        let word = ((unsafe { *bin.add((i * 2) as usize) } as u32) << 8)
            | (unsafe { *bin.add((i * 2 + 1) as usize) } as u32);

        if word == 0 {
            if cur_start < 0 {
                cur_start = i;
            }
            cur_len += 1;
        } else {
            if cur_len > best_len {
                best_start = cur_start;
                best_len = cur_len;
            }
            cur_start = -1;
            cur_len = 0;
        }
    }
    if cur_len > best_len {
        best_start = cur_start;
        best_len = cur_len;
    }
    if best_len < 2 {
        best_start = -1;
    }
    let mut i: i32 = 0;
    while i < 8 {
        if i == best_start {
            buf[p] = b':';
            p += 1;
            buf[p] = b':';
            p += 1;
            i += best_len - 1;
            i += 1;
            continue;
        }
        if i != 0 && (best_start < 0 || i != best_start + best_len) {
            buf[p] = b':';
            p += 1;
        }
        let word = ((unsafe { *bin.add((i * 2) as usize) } as u32) << 8)
            | (unsafe { *bin.add((i * 2 + 1) as usize) } as u32);
        ip_write_num(&mut buf, &mut p, word, 16);
        i += 1;
    }
    let len = p;
    if len >= ip_maxlen {
        return ptr::null_mut();
    }
    unsafe {
        ptr::copy_nonoverlapping(buf.as_ptr(), ip as *mut u8, len);
        *ip.add(len) = 0;
    }

    ip
}
