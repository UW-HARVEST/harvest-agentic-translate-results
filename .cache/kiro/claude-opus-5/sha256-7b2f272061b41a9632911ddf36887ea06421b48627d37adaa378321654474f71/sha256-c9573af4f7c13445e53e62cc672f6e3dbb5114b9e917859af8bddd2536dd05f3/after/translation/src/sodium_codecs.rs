//! Translation of `sodium/codecs.c`

use core::ffi::{c_char, c_int, c_uint};
use core::ptr;

use crate::common::{memcmp, memcpy, memmove, memset};
use crate::sodium_core::sodium_misuse;

/// C `strchr()` on a NUL-terminated string; returns true when `c` is found.
/// Note that `strchr(s, 0)` matches the terminator itself.
unsafe fn strchr_found(s: *const c_char, c: c_int) -> bool {
    let needle = c as u8;
    let mut p = s as *const u8;
    loop {
        let v = *p;
        if v == needle {
            return true;
        }
        if v == 0 {
            return false;
        }
        p = p.add(1);
    }
}

unsafe fn memchr_ptr(s: *const u8, c: u8, n: usize) -> *const u8 {
    for i in 0..n {
        if *s.add(i) == c {
            return s.add(i);
        }
    }
    ptr::null()
}

/* Derived from original code by CodesInChaos */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_bin2hex(
    hex: *mut c_char,
    hex_maxlen: usize,
    bin: *const u8,
    bin_len: usize,
) -> *mut c_char {
    let mut i: usize = 0;
    let mut x: c_uint;

    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len * 2 {
        sodium_misuse();
    }
    while i < bin_len {
        let c = (*bin.add(i) & 0xf) as c_int;
        let b = (*bin.add(i) >> 4) as c_int;
        x = (((87u32
            .wrapping_add(c as u32)
            .wrapping_add(((c as u32).wrapping_sub(10) >> 8) & !38u32)) as u8)
            as c_uint)
            << 8
            | (((87u32
                .wrapping_add(b as u32)
                .wrapping_add(((b as u32).wrapping_sub(10) >> 8) & !38u32)) as u8)
                as c_uint);
        *hex.add(i * 2) = x as c_char;
        x >>= 8;
        *hex.add(i * 2 + 1) = x as c_char;
        i += 1;
    }
    *hex.add(i * 2) = 0;

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
    let mut c: u8;
    let mut c_acc: u8 = 0;
    let mut state: u8 = 0;

    while hex_pos < hex_len {
        c = *(hex.add(hex_pos)) as u8;
        let c_num = c ^ 48u8;
        let c_num0 = ((c_num as u32).wrapping_sub(10) >> 8) as u8;
        let c_alpha = (c & !32u8).wrapping_sub(55);
        let c_alpha0 = ((((c_alpha as u32).wrapping_sub(10)) ^ ((c_alpha as u32).wrapping_sub(16)))
            >> 8) as u8;
        if (c_num0 | c_alpha0) == 0 {
            if !ignore.is_null() && state == 0 && strchr_found(ignore, c as c_int) {
                hex_pos += 1;
                continue;
            }
            break;
        }
        let c_val = (c_num0 & c_num) | (c_alpha0 & c_alpha);
        if bin_pos >= bin_maxlen {
            ret = -1;
            crate::set_errno(crate::ERANGE);
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
        crate::set_errno(crate::EINVAL);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if !hex_end.is_null() {
        *hex_end = hex.add(hex_pos);
    } else if hex_pos != hex_len {
        crate::set_errno(crate::EINVAL);
        ret = -1;
    }
    if !bin_len.is_null() {
        *bin_len = bin_pos;
    }
    ret
}

/*
 * Some macros for constant-time comparisons. These work over values in
 * the 0..255 range. Returned value is 0x00 on "false", 0xFF on "true".
 *
 * Original code by Thomas Pornin.
 */
#[inline(always)]
fn eq(x: c_uint, y: c_uint) -> c_uint {
    ((((0u32.wrapping_sub(x ^ y)) >> 8) & 0xFF) ^ 0xFF) as c_uint
}

#[inline(always)]
fn gt(x: c_uint, y: c_uint) -> c_uint {
    ((y.wrapping_sub(x)) >> 8) & 0xFF
}

#[inline(always)]
fn ge(x: c_uint, y: c_uint) -> c_uint {
    gt(y, x) ^ 0xFF
}

#[inline(always)]
fn lt(x: c_uint, y: c_uint) -> c_uint {
    gt(y, x)
}

#[inline(always)]
fn le(x: c_uint, y: c_uint) -> c_uint {
    ge(y, x)
}

fn b64_byte_to_char(x: c_uint) -> c_int {
    ((lt(x, 26) & (x.wrapping_add(b'A' as u32)))
        | (ge(x, 26) & lt(x, 52) & (x.wrapping_add((b'a' as u32).wrapping_sub(26))))
        | (ge(x, 52) & lt(x, 62) & (x.wrapping_add((b'0' as u32).wrapping_sub(52))))
        | (eq(x, 62) & b'+' as u32)
        | (eq(x, 63) & b'/' as u32)) as c_int
}

fn b64_char_to_byte(c_: c_int) -> c_uint {
    let c = (c_ as u8) as c_uint;
    let x = (ge(c, b'A' as u32) & le(c, b'Z' as u32) & (c.wrapping_sub(b'A' as u32)))
        | (ge(c, b'a' as u32) & le(c, b'z' as u32) & (c.wrapping_sub((b'a' as u32).wrapping_sub(26))))
        | (ge(c, b'0' as u32) & le(c, b'9' as u32) & (c.wrapping_sub((b'0' as u32).wrapping_sub(52))))
        | (eq(c, b'+' as u32) & 62)
        | (eq(c, b'/' as u32) & 63);

    x | (eq(x, 0) & (eq(c, b'A' as u32) ^ 0xFF))
}

fn b64_byte_to_urlsafe_char(x: c_uint) -> c_int {
    ((lt(x, 26) & (x.wrapping_add(b'A' as u32)))
        | (ge(x, 26) & lt(x, 52) & (x.wrapping_add((b'a' as u32).wrapping_sub(26))))
        | (ge(x, 52) & lt(x, 62) & (x.wrapping_add((b'0' as u32).wrapping_sub(52))))
        | (eq(x, 62) & b'-' as u32)
        | (eq(x, 63) & b'_' as u32)) as c_int
}

fn b64_urlsafe_char_to_byte(c_: c_int) -> c_uint {
    let c = (c_ as u8) as c_uint;
    let x = (ge(c, b'A' as u32) & le(c, b'Z' as u32) & (c.wrapping_sub(b'A' as u32)))
        | (ge(c, b'a' as u32) & le(c, b'z' as u32) & (c.wrapping_sub((b'a' as u32).wrapping_sub(26))))
        | (ge(c, b'0' as u32) & le(c, b'9' as u32) & (c.wrapping_sub((b'0' as u32).wrapping_sub(52))))
        | (eq(c, b'-' as u32) & 62)
        | (eq(c, b'_' as u32) & 63);

    x | (eq(x, 0) & (eq(c, b'A' as u32) ^ 0xFF))
}

const VARIANT_NO_PADDING_MASK: c_uint = 0x2;
const VARIANT_URLSAFE_MASK: c_uint = 0x4;

fn sodium_base64_check_variant(variant: c_int) {
    if ((variant as c_uint) & !0x6u32) != 0x1 {
        sodium_misuse();
    }
}

/// `sodium_base64_ENCODED_LEN(BIN_LEN, VARIANT)` from utils.h
fn sodium_base64_encoded_len_macro(bin_len: usize, variant: c_int) -> usize {
    if bin_len / 3 > (usize::MAX - 5) / 4 {
        return usize::MAX;
    }
    let rem = bin_len - (bin_len / 3) * 3;
    (bin_len / 3) * 4
        + ((rem | (rem >> 1)) & 1)
            * (4usize.wrapping_sub(
                (0usize.wrapping_sub((((variant as c_uint) & 2) >> 1) as usize))
                    & (3usize.wrapping_sub(rem)),
            ))
        + 1
}

#[unsafe(no_mangle)]
pub extern "C" fn sodium_base64_encoded_len(bin_len: usize, variant: c_int) -> usize {
    sodium_base64_check_variant(variant);

    if bin_len / 3 > (usize::MAX - 5) / 4 {
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
    let mut b64_len: usize;
    let mut b64_pos: usize = 0;
    let mut bin_pos: usize = 0;
    let nibbles: usize;
    let remainder: usize;
    let mut acc: c_uint = 0;

    sodium_base64_check_variant(variant);
    nibbles = bin_len / 3;
    if nibbles > (usize::MAX - 5) / 4 {
        sodium_misuse();
    }
    remainder = bin_len - 3 * nibbles;
    b64_len = nibbles * 4;
    if remainder != 0 {
        if ((variant as c_uint) & VARIANT_NO_PADDING_MASK) == 0 {
            b64_len += 4;
        } else {
            b64_len += 2 + (remainder >> 1);
        }
    }
    if b64_maxlen <= b64_len {
        sodium_misuse();
    }
    if ((variant as c_uint) & VARIANT_URLSAFE_MASK) != 0 {
        while bin_pos < bin_len {
            acc = (acc << 8).wrapping_add(*bin.add(bin_pos) as c_uint);
            bin_pos += 1;
            acc_len += 8;
            while acc_len >= 6 {
                acc_len -= 6;
                *b64.add(b64_pos) =
                    b64_byte_to_urlsafe_char((acc >> acc_len) & 0x3F) as c_char;
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
            acc = (acc << 8).wrapping_add(*bin.add(bin_pos) as c_uint);
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

unsafe fn _sodium_base642bin_skip_padding(
    b64: *const c_char,
    b64_len: usize,
    b64_pos_p: *mut usize,
    ignore: *const c_char,
    mut padding_len: usize,
) -> c_int {
    let mut c: c_int;

    while padding_len > 0 {
        if *b64_pos_p >= b64_len {
            crate::set_errno(crate::ERANGE);
            return -1;
        }
        c = *b64.add(*b64_pos_p) as c_int;
        if c == b'=' as c_int {
            padding_len -= 1;
        } else if ignore.is_null() || !strchr_found(ignore, c) {
            crate::set_errno(crate::EINVAL);
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
    let is_urlsafe: c_uint;
    let mut ret: c_int = 0;
    let mut acc: c_uint = 0;
    let mut d: c_uint;
    let mut c: c_char;

    sodium_base64_check_variant(variant);
    is_urlsafe = (variant as c_uint) & VARIANT_URLSAFE_MASK;
    while b64_pos < b64_len {
        c = *b64.add(b64_pos);
        if is_urlsafe != 0 {
            d = b64_urlsafe_char_to_byte(c as c_int);
        } else {
            d = b64_char_to_byte(c as c_int);
        }
        if d == 0xFF {
            if !ignore.is_null() && strchr_found(ignore, c as c_int) {
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
                crate::set_errno(crate::ERANGE);
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
    } else if ret == 0 && ((variant as c_uint) & VARIANT_NO_PADDING_MASK) == 0 {
        ret = _sodium_base642bin_skip_padding(
            b64,
            b64_len,
            &mut b64_pos,
            ignore,
            acc_len / 2,
        );
    }
    if ret != 0 {
        bin_pos = 0;
    } else if !ignore.is_null() {
        while b64_pos < b64_len && strchr_found(ignore, *b64.add(b64_pos) as c_int) {
            b64_pos += 1;
        }
    }
    if !b64_end.is_null() {
        *b64_end = b64.add(b64_pos);
    } else if b64_pos != b64_len {
        crate::set_errno(crate::EINVAL);
        ret = -1;
    }
    if !bin_len.is_null() {
        *bin_len = bin_pos;
    }
    ret
}

fn ip_hex_digit(ch: c_int) -> c_int {
    if ch >= b'0' as c_int && ch <= b'9' as c_int {
        return ch - b'0' as c_int;
    }
    if ((ch as c_uint) | 32) >= b'a' as c_uint && ((ch as c_uint) | 32) <= b'f' as c_uint {
        return (((ch as c_uint) | 32).wrapping_sub(b'a' as c_uint).wrapping_add(10)) as c_int;
    }
    -1
}

unsafe fn parse_ipv4(src: *const c_char, end: *const c_char, out: *mut u8) -> c_int {
    let mut p = src;

    if src.is_null() || end.is_null() || out.is_null() || src >= end {
        return 0;
    }
    for i in 0..4 {
        let mut val: c_uint = 0;
        let mut digits: c_int = 0;

        while p < end && *p >= b'0' as c_char && *p <= b'9' as c_char {
            val = val
                .wrapping_mul(10)
                .wrapping_add((*p as u8 - b'0') as c_uint);
            p = p.add(1);
            digits += 1;
            if digits > 3 || val > 255 {
                return 0;
            }
        }
        if digits == 0 {
            return 0;
        }
        *out.add(i) = val as u8;

        if i < 3 {
            if p >= end {
                return 0;
            }
            let ch = *p;
            p = p.add(1);
            if ch != b'.' as c_char {
                return 0;
            }
        }
    }
    (p == end) as c_int
}

unsafe fn parse_ipv6(src: *const c_char, end: *const c_char, out: *mut u8) -> c_int {
    let mut tmp: [u8; 16] = [0; 16];
    let mut tp: *mut u8 = tmp.as_mut_ptr();
    let endp: *mut u8 = tmp.as_mut_ptr().add(16);
    let mut colonp: *mut u8 = ptr::null_mut();
    let mut p = src;
    let mut curtok = src;
    let mut val: c_uint = 0;
    let mut saw_xdigit: c_int = 0;
    let mut xdigits: c_int = 0;
    let mut ch: c_int;
    let mut hv: c_int;

    if src.is_null() || end.is_null() || out.is_null() || src >= end {
        return 0;
    }
    if *p == b':' as c_char {
        p = p.add(1);
        if p >= end || *p != b':' as c_char {
            return 0;
        }
        colonp = tp;
        p = p.add(1);
        curtok = p;
    }
    while p < end {
        ch = *p as c_int;

        if ch == b':' as c_int {
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
        if ch == b'.' as c_int {
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
        val = (val << 4) | (hv as c_uint);
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
        let n = tp.offset_from(colonp) as usize;

        if tp == endp {
            return 0;
        }
        memmove(endp.sub(n), colonp, n);
        memset(colonp, 0, endp.sub(n).offset_from(colonp) as usize);
        tp = endp;
    }
    if tp != endp {
        return 0;
    }
    memcpy(out, tmp.as_ptr(), 16);

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_ip2bin(
    bin: *mut u8,
    ip: *const c_char,
    ip_len_: usize,
) -> c_int {
    let ip_end = ip.add(ip_len_);
    let mut end = ip;
    let zone: *const c_char;
    let mut z: *const c_char;
    let mut v4: [u8; 4] = [0; 4];
    let is_ipv6: bool;

    while end < ip_end && *end != 0 {
        end = end.add(1);
    }
    zone = memchr_ptr(ip as *const u8, b'%', end.offset_from(ip) as usize) as *const c_char;
    if !zone.is_null() {
        z = zone.add(1);
        while z < end {
            let c = *z as u8;
            if !((c >= b'0' && c <= b'9')
                || (c >= b'a' && c <= b'z')
                || (c >= b'A' && c <= b'Z')
                || c == b'-'
                || c == b'_'
                || c == b'.')
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
    is_ipv6 =
        !memchr_ptr(ip as *const u8, b':', end.offset_from(ip) as usize).is_null();
    if !zone.is_null() && !is_ipv6 {
        return -1;
    }
    if is_ipv6 {
        return if parse_ipv6(ip, end, bin) != 0 { 0 } else { -1 };
    }
    if parse_ipv4(ip, end, v4.as_mut_ptr()) == 0 {
        return -1;
    }
    memset(bin, 0, 10);
    *bin.add(10) = 0xff;
    *bin.add(11) = 0xff;
    memcpy(bin.add(12), v4.as_ptr(), 4);

    0
}

static IPV4_MAPPED_PREFIX: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

unsafe fn ip_write_num(p: &mut *mut c_char, mut val: c_uint, base: c_int) {
    let mut buf: [c_char; 4] = [0; 4];
    let mut n: usize = 0;

    loop {
        let d = val % (base as c_uint);

        buf[n] = if d < 10 {
            (b'0' as c_uint + d) as c_char
        } else {
            (b'a' as c_uint + d - 10) as c_char
        };
        n += 1;
        val /= base as c_uint;
        if val == 0 {
            break;
        }
    }

    while n > 0 {
        n -= 1;
        **p = buf[n];
        *p = (*p).add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_bin2ip(
    ip: *mut c_char,
    ip_maxlen: usize,
    bin: *const u8,
) -> *mut c_char {
    // The C code leaves `buf` uninitialized; the ipv4-mapped branch copies
    // `len + 1` bytes out of it and then overwrites the extra byte with 0.
    let mut buf: [c_char; 46] = [0; 46];
    let mut p: *mut c_char = buf.as_mut_ptr();
    let mut i: c_int;
    let mut best_start: c_int = -1;
    let mut best_len: c_int = 0;
    let mut cur_start: c_int = -1;
    let mut cur_len: c_int = 0;
    let len: usize;

    if ip_maxlen <= 2 {
        return ptr::null_mut();
    }
    if memcmp(bin, IPV4_MAPPED_PREFIX.as_ptr(), 12) == 0 {
        i = 0;
        while i < 4 {
            if i != 0 {
                *p = b'.' as c_char;
                p = p.add(1);
            }
            ip_write_num(&mut p, *bin.add(12 + i as usize) as c_uint, 10);
            i += 1;
        }
        let len = p.offset_from(buf.as_ptr()) as usize;
        if len >= ip_maxlen {
            return ptr::null_mut();
        }
        memcpy(ip as *mut u8, buf.as_ptr() as *const u8, len + 1);
        *ip.add(len) = 0;

        return ip;
    }
    i = 0;
    while i < 8 {
        let word = ((*bin.add((i * 2) as usize) as c_uint) << 8)
            | (*bin.add((i * 2 + 1) as usize) as c_uint);

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
            ((*bin.add((i * 2) as usize) as c_uint) << 8)
                | (*bin.add((i * 2 + 1) as usize) as c_uint),
            16,
        );
        i += 1;
    }
    len = p.offset_from(buf.as_ptr()) as usize;
    if len >= ip_maxlen {
        return ptr::null_mut();
    }
    memcpy(ip as *mut u8, buf.as_ptr() as *const u8, len);
    *ip.add(len) = 0;

    ip
}
