//! Translated from `c_src/libsodium/sodium/codecs.c`.
//!
//! bin2hex / hex2bin, base64 encode/decode variants, ip2bin / bin2ip.

#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_void};

use crate::csys::{memcmp, memcpy, memmove, memset, strchr, size_t};

extern "C" {
    fn sodium_misuse() -> !;
    fn memchr(s: *const c_void, c: c_int, n: size_t) -> *mut c_void;
}

/// `SIZE_MAX` as used by the C source (matches `usize::MAX` on this target).
const SIZE_MAX: usize = usize::MAX;

// ---------------------------------------------------------------------
// bin2hex / hex2bin
// ---------------------------------------------------------------------

/// `(unsigned char) (87U + v + (((v - 10U) >> 8) & ~38U))`, computed in
/// 32-bit unsigned arithmetic exactly as C promotes it, then truncated to a
/// byte (matching the explicit `(unsigned char)` cast in the source).
#[inline(always)]
fn hex_nibble_enc(v: u32) -> u32 {
    let raw = 87u32
        .wrapping_add(v)
        .wrapping_add((v.wrapping_sub(10) >> 8) & !38u32);
    (raw as u8) as u32
}

#[no_mangle]
pub unsafe extern "C" fn sodium_bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    let mut i: usize = 0;
    let mut x: u32;

    if bin_len >= SIZE_MAX / 2 || hex_maxlen <= bin_len.wrapping_mul(2) {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    while i < bin_len {
        let byte = *bin.add(i) as u32;
        let c = byte & 0xf;
        let b = byte >> 4;
        x = (hex_nibble_enc(c) << 8) | hex_nibble_enc(b);
        *hex.add(i * 2) = x as u8 as c_char;
        x >>= 8;
        *hex.add(i * 2 + 1) = x as u8 as c_char;
        i += 1;
    }
    *hex.add(i * 2) = 0;

    hex
}

#[no_mangle]
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
    let mut c: u8;
    let mut c_acc: u8 = 0;
    let mut c_alpha0: u8;
    let mut c_alpha: u8;
    let mut c_num0: u8;
    let mut c_num: u8;
    let mut c_val: u8;
    let mut state: u8 = 0;

    while hex_pos < hex_len {
        c = *hex.add(hex_pos) as u8;
        c_num = c ^ 48u8;
        // `c_num - 10U` promotes to *unsigned* 32-bit arithmetic in C, so the
        // subsequent `>> 8` is a logical (zero-filling) shift, not the
        // arithmetic shift Rust's `i32 >>` would give.
        c_num0 = (((c_num as u32).wrapping_sub(10)) >> 8) as u8;
        c_alpha = (c & !32u8).wrapping_sub(55);
        c_alpha0 = ((((c_alpha as u32).wrapping_sub(10)) ^ ((c_alpha as u32).wrapping_sub(16)))
            >> 8) as u8;

        if (c_num0 | c_alpha0) == 0u8 {
            if !ignore.is_null() && state == 0 && !strchr(ignore, c as c_int).is_null() {
                hex_pos += 1;
                continue;
            }
            break;
        }
        c_val = (c_num0 & c_num) | (c_alpha0 & c_alpha);
        if bin_pos >= bin_maxlen {
            ret = -1;
            crate::csys::set_errno(crate::csys::ERANGE);
            break;
        }
        if state == 0 {
            c_acc = c_val.wrapping_mul(16);
        } else {
            *bin.add(bin_pos) = c_acc | c_val;
            bin_pos += 1;
        }
        state = !state;
        hex_pos += 1;
    }
    if state != 0 {
        hex_pos = hex_pos.wrapping_sub(1);
        crate::csys::set_errno(crate::csys::EINVAL);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if !hex_end.is_null() {
        *hex_end = hex.add(hex_pos);
    } else if hex_pos != hex_len {
        crate::csys::set_errno(crate::csys::EINVAL);
        ret = -1;
    }
    if !bin_len.is_null() {
        *bin_len = bin_pos;
    }
    ret
}

// ---------------------------------------------------------------------
// Constant-time comparison macros (EQ/GT/GE/LT/LE), operating on values in
// the 0..255 range represented as u32. Returned value is 0x00 on "false",
// 0xFF on "true".
// ---------------------------------------------------------------------

#[inline(always)]
fn eq(x: u32, y: u32) -> u32 {
    ((0u32.wrapping_sub(x ^ y)) >> 8 & 0xFF) ^ 0xFF
}

#[inline(always)]
fn gt(x: u32, y: u32) -> u32 {
    (y.wrapping_sub(x)) >> 8 & 0xFF
}

#[inline(always)]
fn ge(x: u32, y: u32) -> u32 {
    gt(y, x) ^ 0xFF
}

#[inline(always)]
fn lt(x: u32, y: u32) -> u32 {
    gt(y, x)
}

#[inline(always)]
fn le(x: u32, y: u32) -> u32 {
    ge(y, x)
}

fn b64_byte_to_char(x: u32) -> i32 {
    ((lt(x, 26) & (x.wrapping_add('A' as u32)))
        | (ge(x, 26) & lt(x, 52) & (x.wrapping_add('a' as u32).wrapping_sub(26)))
        | (ge(x, 52) & lt(x, 62) & (x.wrapping_add('0' as u32).wrapping_sub(52)))
        | (eq(x, 62) & '+' as u32)
        | (eq(x, 63) & '/' as u32)) as i32
}

fn b64_char_to_byte(c_: i32) -> u32 {
    let c = (c_ as u8) as u32;
    let x = (ge(c, 'A' as u32) & le(c, 'Z' as u32) & (c.wrapping_sub('A' as u32)))
        | (ge(c, 'a' as u32) & le(c, 'z' as u32) & (c.wrapping_sub(('a' as u32).wrapping_sub(26))))
        | (ge(c, '0' as u32) & le(c, '9' as u32) & (c.wrapping_sub(('0' as u32).wrapping_sub(52))))
        | (eq(c, '+' as u32) & 62)
        | (eq(c, '/' as u32) & 63);

    x | (eq(x, 0) & (eq(c, 'A' as u32) ^ 0xFF))
}

fn b64_byte_to_urlsafe_char(x: u32) -> i32 {
    ((lt(x, 26) & (x.wrapping_add('A' as u32)))
        | (ge(x, 26) & lt(x, 52) & (x.wrapping_add('a' as u32).wrapping_sub(26)))
        | (ge(x, 52) & lt(x, 62) & (x.wrapping_add('0' as u32).wrapping_sub(52)))
        | (eq(x, 62) & '-' as u32)
        | (eq(x, 63) & '_' as u32)) as i32
}

fn b64_urlsafe_char_to_byte(c_: i32) -> u32 {
    let c = (c_ as u8) as u32;
    let x = (ge(c, 'A' as u32) & le(c, 'Z' as u32) & (c.wrapping_sub('A' as u32)))
        | (ge(c, 'a' as u32) & le(c, 'z' as u32) & (c.wrapping_sub(('a' as u32).wrapping_sub(26))))
        | (ge(c, '0' as u32) & le(c, '9' as u32) & (c.wrapping_sub(('0' as u32).wrapping_sub(52))))
        | (eq(c, '-' as u32) & 62)
        | (eq(c, '_' as u32) & 63);

    x | (eq(x, 0) & (eq(c, 'A' as u32) ^ 0xFF))
}

const VARIANT_NO_PADDING_MASK: u32 = 0x2;
const VARIANT_URLSAFE_MASK: u32 = 0x4;

unsafe fn sodium_base64_check_variant(variant: c_int) {
    if ((variant as u32) & !0x6u32) != 0x1u32 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
}

/// Mirrors the `sodium_base64_ENCODED_LEN` macro from `utils.h`.
fn sodium_base64_encoded_len_macro(bin_len: usize, variant: c_int) -> usize {
    let variant = variant as u32;
    let nibbles = bin_len / 3;
    if nibbles > (SIZE_MAX - 5) / 4 {
        return SIZE_MAX;
    }
    let remainder = bin_len - nibbles * 3;
    let has_remainder: usize = (((remainder | (remainder >> 1)) & 1) != 0) as usize;
    let no_padding_bit: u32 = (variant & 2) >> 1;
    let mask: u32 = 0u32.wrapping_sub(no_padding_bit); // 0x0 or 0xFFFFFFFF
    let term = (4u32.wrapping_sub(mask & (3u32.wrapping_sub(remainder as u32)))) as usize;
    nibbles.wrapping_mul(4) + has_remainder.wrapping_mul(term) + 1
}

#[no_mangle]
pub unsafe extern "C" fn sodium_base64_encoded_len(bin_len: usize, variant: c_int) -> usize {
    sodium_base64_check_variant(variant);

    if bin_len / 3 > (SIZE_MAX - 5) / 4 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    sodium_base64_encoded_len_macro(bin_len, variant)
}

#[no_mangle]
pub unsafe extern "C" fn sodium_bin2base64(
    b64: *mut c_char,
    b64_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
    variant: c_int,
) -> *mut c_char {
    let mut acc_len: usize = 0;
    let mut b64_len: usize;
    let mut b64_pos: usize = 0;
    let mut bin_pos: usize = 0;
    let nibbles: usize;
    let remainder: usize;
    let mut acc: u32 = 0;

    sodium_base64_check_variant(variant);
    nibbles = bin_len / 3;
    if nibbles > (SIZE_MAX - 5) / 4 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    remainder = bin_len - 3 * nibbles;
    b64_len = nibbles * 4;
    if remainder != 0 {
        if ((variant as u32) & VARIANT_NO_PADDING_MASK) == 0 {
            b64_len += 4;
        } else {
            b64_len += 2 + (remainder >> 1);
        }
    }
    if b64_maxlen <= b64_len {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if ((variant as u32) & VARIANT_URLSAFE_MASK) != 0 {
        while bin_pos < bin_len {
            acc = (acc << 8).wrapping_add(*bin.add(bin_pos) as u32);
            bin_pos += 1;
            acc_len += 8;
            while acc_len >= 6 {
                acc_len -= 6;
                *b64.add(b64_pos) = b64_byte_to_urlsafe_char((acc >> acc_len) & 0x3F) as c_char;
                b64_pos += 1;
            }
        }
        if acc_len > 0 {
            *b64.add(b64_pos) =
                b64_byte_to_urlsafe_char((acc << (6 - acc_len)) & 0x3F) as c_char;
            b64_pos += 1;
        }
    } else {
        while bin_pos < bin_len {
            acc = (acc << 8).wrapping_add(*bin.add(bin_pos) as u32);
            bin_pos += 1;
            acc_len += 8;
            while acc_len >= 6 {
                acc_len -= 6;
                *b64.add(b64_pos) = b64_byte_to_char((acc >> acc_len) & 0x3F) as c_char;
                b64_pos += 1;
            }
        }
        if acc_len > 0 {
            *b64.add(b64_pos) = b64_byte_to_char((acc << (6 - acc_len)) & 0x3F) as c_char;
            b64_pos += 1;
        }
    }
    debug_assert!(b64_pos <= b64_len);
    while b64_pos < b64_len {
        *b64.add(b64_pos) = b'=' as c_char;
        b64_pos += 1;
    }
    loop {
        *b64.add(b64_pos) = 0;
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
    b64_pos_p: *mut usize,
    ignore: *const c_char,
    padding_len: usize,
) -> c_int {
    let mut padding_len = padding_len;
    let mut c: c_int;

    while padding_len > 0 {
        if *b64_pos_p >= b64_len {
            crate::csys::set_errno(crate::csys::ERANGE);
            return -1;
        }
        // ACQUIRE_FENCE resolves to a no-op under this build configuration.
        // `char` is signed on this target: sign-extend like C's `int c = b64[i];`.
        c = *b64.add(*b64_pos_p) as c_int;
        if c == '=' as c_int {
            padding_len -= 1;
        } else if ignore.is_null() || strchr(ignore, c).is_null() {
            crate::csys::set_errno(crate::csys::EINVAL);
            return -1;
        }
        *b64_pos_p += 1;
    }
    0
}

#[no_mangle]
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
    let is_urlsafe: u32;
    let mut ret: c_int = 0;
    let mut acc: u32 = 0;
    let mut d: u32;
    let mut c: c_char;

    sodium_base64_check_variant(variant);
    is_urlsafe = (variant as u32) & VARIANT_URLSAFE_MASK;
    while b64_pos < b64_len {
        c = *b64.add(b64_pos);
        if is_urlsafe != 0 {
            d = b64_urlsafe_char_to_byte(c as c_int);
        } else {
            d = b64_char_to_byte(c as c_int);
        }
        if d == 0xFF {
            if !ignore.is_null() && !strchr(ignore, c as c_int).is_null() {
                b64_pos += 1;
                continue;
            }
            break;
        }
        acc = (acc << 6).wrapping_add(d);
        acc_len += 6;
        if acc_len >= 8 {
            acc_len -= 8;
            if bin_pos >= bin_maxlen {
                crate::csys::set_errno(crate::csys::ERANGE);
                ret = -1;
                break;
            }
            *bin.add(bin_pos) = ((acc >> acc_len) & 0xFF) as u8;
            bin_pos += 1;
        }
        b64_pos += 1;
    }
    if acc_len > 4 || (acc & ((1u32 << acc_len) - 1)) != 0 {
        ret = -1;
    } else if ret == 0 && ((variant as u32) & VARIANT_NO_PADDING_MASK) == 0 {
        ret = sodium_base642bin_skip_padding(b64, b64_len, &mut b64_pos, ignore, acc_len / 2);
    }
    if ret != 0 {
        bin_pos = 0;
    } else if !ignore.is_null() {
        while b64_pos < b64_len && !strchr(ignore, *b64.add(b64_pos) as c_int).is_null() {
            b64_pos += 1;
        }
    }
    if !b64_end.is_null() {
        *b64_end = b64.add(b64_pos);
    } else if b64_pos != b64_len {
        crate::csys::set_errno(crate::csys::EINVAL);
        ret = -1;
    }
    if !bin_len.is_null() {
        *bin_len = bin_pos;
    }
    ret
}

// ---------------------------------------------------------------------
// ip2bin / bin2ip
// ---------------------------------------------------------------------

unsafe fn ip_hex_digit(ch: c_int) -> c_int {
    if ch >= '0' as c_int && ch <= '9' as c_int {
        return ch - '0' as c_int;
    }
    if ((ch as u32) | 32) >= 'a' as u32 && ((ch as u32) | 32) <= 'f' as u32 {
        return (((ch as u32) | 32) - 'a' as u32 + 10) as c_int;
    }
    -1
}

unsafe fn parse_ipv4(src: *const c_char, end: *const c_char, out: *mut u8) -> c_int {
    let mut p = src;
    let mut i: i32;

    if src.is_null() || end.is_null() || out.is_null() || src >= end {
        return 0;
    }
    i = 0;
    while i < 4 {
        let mut val: u32 = 0;
        let mut digits: i32 = 0;

        while p < end && (*p as c_int) >= '0' as c_int && (*p as c_int) <= '9' as c_int {
            val = val
                .wrapping_mul(10)
                .wrapping_add(((*p as c_int) - '0' as c_int) as u32);
            p = p.add(1);
            digits += 1;
            if digits > 3 || val > 255 {
                return 0;
            }
        }
        if digits == 0 {
            return 0;
        }
        *out.add(i as usize) = val as u8;

        if i < 3 {
            if p >= end || {
                let c = *p;
                p = p.add(1);
                c
            } != b'.' as c_char
            {
                return 0;
            }
        }
        i += 1;
    }
    (p == end) as c_int
}

unsafe fn parse_ipv6(src: *const c_char, end: *const c_char, out: *mut u8) -> c_int {
    let mut tmp: [u8; 16] = [0; 16];
    let tmp_ptr: *mut u8 = tmp.as_mut_ptr();
    let mut tp: *mut u8 = tmp_ptr;
    let endp: *mut u8 = tmp_ptr.add(16);
    let mut colonp: *mut u8 = core::ptr::null_mut();
    let mut p: *const c_char = src;
    let mut curtok: *const c_char;
    let mut val: u32 = 0;
    let mut saw_xdigit: c_int = 0;
    let mut xdigits: c_int = 0;
    let mut ch: c_int;
    let mut hv: c_int;

    if src.is_null() || end.is_null() || out.is_null() || src >= end {
        return 0;
    }
    curtok = src;
    if *p as u8 == b':' {
        p = p.add(1);
        if p >= end || *p as u8 != b':' {
            return 0;
        }
        colonp = tp;
        p = p.add(1);
        curtok = p;
    }
    while p < end {
        ch = *p as c_int; // `char` is signed on this target: sign-extend like C.

        if ch == ':' as c_int {
            if saw_xdigit == 0 {
                if !colonp.is_null() {
                    return 0;
                }
                colonp = tp;
                p = p.add(1);
                curtok = p;
                continue;
            }
            if tp.add(2) > endp {
                return 0;
            }
            *tp = (val >> 8) as u8;
            tp = tp.add(1);
            *tp = (val & 0xff) as u8;
            tp = tp.add(1);
            val = 0;
            saw_xdigit = 0;
            xdigits = 0;
            p = p.add(1);
            curtok = p;
            if p >= end {
                return 0;
            }
            continue;
        }
        if ch == '.' as c_int {
            if tp.add(4) > endp || parse_ipv4(curtok, end, tp) == 0 {
                return 0;
            }
            tp = tp.add(4);
            saw_xdigit = 0;
            break;
        }
        hv = ip_hex_digit(ch);
        if hv < 0 || xdigits >= 4 {
            return 0;
        }
        val = (val << 4) | (hv as u32);
        saw_xdigit = 1;
        xdigits += 1;
        p = p.add(1);
    }
    if saw_xdigit != 0 {
        if tp.add(2) > endp {
            return 0;
        }
        *tp = (val >> 8) as u8;
        tp = tp.add(1);
        *tp = (val & 0xff) as u8;
        tp = tp.add(1);
    }
    if !colonp.is_null() {
        let n: usize = tp.offset_from(colonp) as usize;

        if tp == endp {
            return 0;
        }
        memmove(
            endp.sub(n) as *mut c_void,
            colonp as *const c_void,
            n as size_t,
        );
        memset(
            colonp as *mut c_void,
            0,
            endp.sub(n).offset_from(colonp) as size_t,
        );
        tp = endp;
    }
    if tp != endp {
        return 0;
    }
    memcpy(out as *mut c_void, tmp_ptr as *const c_void, 16);

    1
}

#[no_mangle]
pub unsafe extern "C" fn sodium_ip2bin(bin: *mut u8, ip: *const c_char, ip_len_: usize) -> c_int {
    let ip_end = ip.add(ip_len_);
    let mut end: *const c_char = ip;
    let mut zone: *const c_char;
    let mut z: *const c_char;
    let mut v4: [u8; 4] = [0; 4];
    let is_ipv6: bool;

    while end < ip_end && *end != 0 {
        end = end.add(1);
    }
    zone = memchr(ip as *const c_void, '%' as c_int, end.offset_from(ip) as size_t)
        as *const c_char;
    if !zone.is_null() {
        z = zone.add(1);
        while z < end {
            let zc = *z as u8;
            if !((zc >= b'0' && zc <= b'9')
                || (zc >= b'a' && zc <= b'z')
                || (zc >= b'A' && zc <= b'Z')
                || zc == b'-'
                || zc == b'_'
                || zc == b'.')
            {
                return -1;
            }
            z = z.add(1);
        }
        if zone.add(1) >= end {
            return -1;
        }
        end = zone;
    }
    is_ipv6 = !memchr(ip as *const c_void, ':' as c_int, end.offset_from(ip) as size_t).is_null();
    if !zone.is_null() && !is_ipv6 {
        return -1;
    }
    if is_ipv6 {
        return if parse_ipv6(ip, end, bin) != 0 { 0 } else { -1 };
    }
    if parse_ipv4(ip, end, v4.as_mut_ptr()) == 0 {
        return -1;
    }
    memset(bin as *mut c_void, 0, 10);
    *bin.add(10) = 0xff;
    *bin.add(11) = 0xff;
    memcpy(bin.add(12) as *mut c_void, v4.as_ptr() as *const c_void, 4);

    0
}

static IPV4_MAPPED_PREFIX: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

unsafe fn ip_write_num(p: *mut *mut c_char, val: u32, base: i32) {
    let mut buf: [c_char; 4] = [0; 4];
    let mut n: i32 = 0;
    let mut val = val;

    loop {
        let d = val % (base as u32);

        buf[n as usize] = (if d < 10 {
            b'0' + d as u8
        } else {
            b'a' + (d as u8) - 10
        }) as c_char;
        n += 1;
        val /= base as u32;
        if val == 0 {
            break;
        }
    }

    while n > 0 {
        n -= 1;
        **p = buf[n as usize];
        *p = (*p).add(1);
    }
}

#[no_mangle]
pub unsafe extern "C" fn sodium_bin2ip(
    ip: *mut c_char,
    ip_maxlen: usize,
    bin: *const u8,
) -> *mut c_char {
    let mut buf: [c_char; 46] = [0; 46];
    let mut p: *mut c_char = buf.as_mut_ptr();
    let mut i: i32;
    let mut best_start: i32 = -1;
    let mut best_len: i32 = 0;
    let mut cur_start: i32 = -1;
    let mut cur_len: i32 = 0;
    let len: usize;

    if ip_maxlen <= 2 {
        return core::ptr::null_mut();
    }
    if memcmp(
        bin as *const c_void,
        IPV4_MAPPED_PREFIX.as_ptr() as *const c_void,
        12,
    ) == 0
    {
        i = 0;
        while i < 4 {
            if i != 0 {
                *p = b'.' as c_char;
                p = p.add(1);
            }
            ip_write_num(&mut p, *bin.add(12 + i as usize) as u32, 10);
            i += 1;
        }
        len = p.offset_from(buf.as_ptr()) as usize;
        if len >= ip_maxlen {
            return core::ptr::null_mut();
        }
        memcpy(
            ip as *mut c_void,
            buf.as_ptr() as *const c_void,
            (len + 1) as size_t,
        );
        *ip.add(len) = 0;

        return ip;
    }
    i = 0;
    while i < 8 {
        let word = ((*bin.add(i as usize * 2) as u32) << 8) | (*bin.add(i as usize * 2 + 1) as u32);

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
        i += 1;
    }
    if cur_len > best_len {
        best_start = cur_start;
        best_len = cur_len;
    }
    if best_len < 2 {
        best_start = -1;
    }
    i = 0;
    while i < 8 {
        if i == best_start {
            *p = b':' as c_char;
            p = p.add(1);
            *p = b':' as c_char;
            p = p.add(1);
            i += best_len - 1;
            i += 1;
            continue;
        }
        if i != 0 && (best_start < 0 || i != best_start + best_len) {
            *p = b':' as c_char;
            p = p.add(1);
        }
        ip_write_num(
            &mut p,
            ((*bin.add(i as usize * 2) as u32) << 8) | (*bin.add(i as usize * 2 + 1) as u32),
            16,
        );
        i += 1;
    }
    len = p.offset_from(buf.as_ptr()) as usize;
    if len >= ip_maxlen {
        return core::ptr::null_mut();
    }
    memcpy(ip as *mut c_void, buf.as_ptr() as *const c_void, len as size_t);
    *ip.add(len) = 0;

    ip
}
