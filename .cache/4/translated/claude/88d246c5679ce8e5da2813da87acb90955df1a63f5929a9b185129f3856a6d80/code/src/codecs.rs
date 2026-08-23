//! Translation of `c_src/libsodium/sodium/codecs.c`.
//!
//! `ACQUIRE_FENCE` expands to `(void) 0` in the reference build (neither
//! `HAVE_GCC_MEMORY_FENCES` nor `HAVE_C11_MEMORY_FENCES` is defined), so it is
//! simply dropped here.

use crate::common::*;
use core::ffi::{c_char, c_int};

extern "C" {
    /// `sodium/utils.c` — `__attribute__((noreturn))`.
    fn sodium_misuse() -> !;
    fn __errno_location() -> *mut c_int;
}

const EINVAL: c_int = 22;
const ERANGE: c_int = 34;

#[inline(always)]
unsafe fn set_errno(e: c_int) {
    *__errno_location() = e;
}

/// `strchr(3)`.  Note that, as in C, a search for `\0` succeeds and returns a
/// pointer to the terminator.
#[inline]
unsafe fn strchr(s: *const c_char, c: c_int) -> *const c_char {
    let ch = c as u8;
    let mut p = s;
    loop {
        let b = *(p as *const u8);
        if b == ch {
            return p;
        }
        if b == 0 {
            return core::ptr::null();
        }
        p = p.add(1);
    }
}

/// `memchr(3)`.
#[inline]
unsafe fn memchr(s: *const c_char, c: c_int, n: usize) -> *const c_char {
    let ch = c as u8;
    let mut i: usize = 0;
    while i < n {
        if *(s.add(i) as *const u8) == ch {
            return s.add(i);
        }
        i += 1;
    }
    core::ptr::null()
}

/// `memcmp(3)`, restricted to the sign of the first difference.
#[inline]
unsafe fn memcmp(a: *const u8, b: *const u8, n: usize) -> c_int {
    let mut i: usize = 0;
    while i < n {
        let x = *a.add(i);
        let y = *b.add(i);
        if x != y {
            return x as c_int - y as c_int;
        }
        i += 1;
    }
    0
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
    let mut x: u32;
    let mut b: u32;
    let mut c: u32;

    if bin_len >= usize::MAX / 2 || hex_maxlen <= bin_len.wrapping_mul(2) {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    while i < bin_len {
        c = (*bin.add(i) & 0xf) as u32;
        b = (*bin.add(i) >> 4) as u32;
        x = ((87u32
            .wrapping_add(c)
            .wrapping_add((c.wrapping_sub(10) >> 8) & !38u32)) as u8 as u32)
            << 8
            | (87u32
                .wrapping_add(b)
                .wrapping_add((b.wrapping_sub(10) >> 8) & !38u32)) as u8 as u32;
        *hex.add(i.wrapping_mul(2)) = x as u8 as c_char;
        x >>= 8;
        *hex.add(i.wrapping_mul(2).wrapping_add(1)) = x as u8 as c_char;
        i += 1;
    }
    *hex.add(i.wrapping_mul(2)) = 0;

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
    let mut c_alpha0: u8;
    let mut c_alpha: u8;
    let mut c_num0: u8;
    let mut c_num: u8;
    let mut c_val: u8;
    let mut state: u8 = 0;

    while hex_pos < hex_len {
        c = *(hex.add(hex_pos) as *const u8);
        c_num = (c as u32 ^ 48u32) as u8;
        c_num0 = ((c_num as u32).wrapping_sub(10) >> 8) as u8;
        c_alpha = ((c as u32 & !32u32).wrapping_sub(55)) as u8;
        c_alpha0 = ((((c_alpha as u32).wrapping_sub(10)) ^ ((c_alpha as u32).wrapping_sub(16)))
            >> 8) as u8;
        if (c_num0 | c_alpha0) == 0 {
            if !ignore.is_null() && state == 0 && !strchr(ignore, c as c_int).is_null() {
                hex_pos += 1;
                continue;
            }
            break;
        }
        c_val = (c_num0 & c_num) | (c_alpha0 & c_alpha);
        if bin_pos >= bin_maxlen {
            ret = -1;
            set_errno(ERANGE);
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
        set_errno(EINVAL);
        ret = -1;
    }
    if ret != 0 {
        bin_pos = 0;
    }
    if !hex_end.is_null() {
        *hex_end = hex.add(hex_pos);
    } else if hex_pos != hex_len {
        set_errno(EINVAL);
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
fn eq(x: u32, y: u32) -> u32 {
    ((((0u32.wrapping_sub(x ^ y)) >> 8) & 0xFF) ^ 0xFF)
}

#[inline(always)]
fn gt(x: u32, y: u32) -> u32 {
    ((y.wrapping_sub(x)) >> 8) & 0xFF
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

fn b64_byte_to_char(x: u32) -> c_int {
    ((lt(x, 26) & x.wrapping_add(b'A' as u32))
        | (ge(x, 26) & lt(x, 52) & x.wrapping_add((b'a' as u32).wrapping_sub(26)))
        | (ge(x, 52) & lt(x, 62) & x.wrapping_add((b'0' as u32).wrapping_sub(52)))
        | (eq(x, 62) & b'+' as u32)
        | (eq(x, 63) & b'/' as u32)) as c_int
}

fn b64_char_to_byte(c_: c_int) -> u32 {
    let c: u32 = (c_ as u8) as u32;
    let x: u32 = (ge(c, b'A' as u32) & le(c, b'Z' as u32) & c.wrapping_sub(b'A' as u32))
        | (ge(c, b'a' as u32) & le(c, b'z' as u32) & c.wrapping_sub((b'a' as u32).wrapping_sub(26)))
        | (ge(c, b'0' as u32) & le(c, b'9' as u32) & c.wrapping_sub((b'0' as u32).wrapping_sub(52)))
        | (eq(c, b'+' as u32) & 62)
        | (eq(c, b'/' as u32) & 63);

    x | (eq(x, 0) & (eq(c, b'A' as u32) ^ 0xFF))
}

fn b64_byte_to_urlsafe_char(x: u32) -> c_int {
    ((lt(x, 26) & x.wrapping_add(b'A' as u32))
        | (ge(x, 26) & lt(x, 52) & x.wrapping_add((b'a' as u32).wrapping_sub(26)))
        | (ge(x, 52) & lt(x, 62) & x.wrapping_add((b'0' as u32).wrapping_sub(52)))
        | (eq(x, 62) & b'-' as u32)
        | (eq(x, 63) & b'_' as u32)) as c_int
}

fn b64_urlsafe_char_to_byte(c_: c_int) -> u32 {
    let c: u32 = (c_ as u8) as u32;
    let x: u32 = (ge(c, b'A' as u32) & le(c, b'Z' as u32) & c.wrapping_sub(b'A' as u32))
        | (ge(c, b'a' as u32) & le(c, b'z' as u32) & c.wrapping_sub((b'a' as u32).wrapping_sub(26)))
        | (ge(c, b'0' as u32) & le(c, b'9' as u32) & c.wrapping_sub((b'0' as u32).wrapping_sub(52)))
        | (eq(c, b'-' as u32) & 62)
        | (eq(c, b'_' as u32) & 63);

    x | (eq(x, 0) & (eq(c, b'A' as u32) ^ 0xFF))
}

const VARIANT_NO_PADDING_MASK: u32 = 0x2;
const VARIANT_URLSAFE_MASK: u32 = 0x4;

unsafe fn sodium_base64_check_variant(variant: c_int) {
    if ((variant as u32) & !0x6u32) != 0x1 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
}

/// `sodium_base64_ENCODED_LEN(BIN_LEN, VARIANT)` from `sodium/utils.h`.
#[inline]
fn sodium_base64_encoded_len_macro(bin_len: usize, variant: c_int) -> usize {
    if bin_len / 3 > (usize::MAX - 5) / 4 {
        return usize::MAX;
    }
    let rem: usize = bin_len.wrapping_sub((bin_len / 3).wrapping_mul(3));
    /* `(0U - (((VARIANT) & 2U) >> 1))` is an `unsigned int`, zero-extended to
     * `size_t` before the bitwise AND with `(3U - rem)`. */
    let nopad: usize = (0u32.wrapping_sub(((variant as u32) & 2) >> 1)) as usize;
    (bin_len / 3)
        .wrapping_mul(4)
        .wrapping_add(
            ((rem | (rem >> 1)) & 1)
                .wrapping_mul(4usize.wrapping_sub(nopad & 3usize.wrapping_sub(rem))),
        )
        .wrapping_add(1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_base64_encoded_len(bin_len: usize, variant: c_int) -> usize {
    sodium_base64_check_variant(variant);

    if bin_len / 3 > (usize::MAX - 5) / 4 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
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
    let mut acc: u32 = 0;

    sodium_base64_check_variant(variant);
    nibbles = bin_len / 3;
    if nibbles > (usize::MAX - 5) / 4 {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    remainder = bin_len.wrapping_sub(3usize.wrapping_mul(nibbles));
    b64_len = nibbles.wrapping_mul(4);
    if remainder != 0 {
        if ((variant as u32) & VARIANT_NO_PADDING_MASK) == 0 {
            b64_len = b64_len.wrapping_add(4);
        } else {
            b64_len = b64_len.wrapping_add(2usize.wrapping_add(remainder >> 1));
        }
    }
    if b64_maxlen <= b64_len {
        sodium_misuse(); /* LCOV_EXCL_LINE */
    }
    if ((variant as u32) & VARIANT_URLSAFE_MASK) != 0 {
        while bin_pos < bin_len {
            acc = acc.wrapping_shl(8).wrapping_add(*bin.add(bin_pos) as u32);
            bin_pos += 1;
            acc_len = acc_len.wrapping_add(8);
            while acc_len >= 6 {
                acc_len -= 6;
                *b64.add(b64_pos) =
                    b64_byte_to_urlsafe_char(acc.wrapping_shr(acc_len as u32) & 0x3F) as u8
                        as c_char;
                b64_pos += 1;
            }
        }
        if acc_len > 0 {
            *b64.add(b64_pos) =
                b64_byte_to_urlsafe_char(acc.wrapping_shl((6 - acc_len) as u32) & 0x3F) as u8
                    as c_char;
            b64_pos += 1;
        }
    } else {
        while bin_pos < bin_len {
            acc = acc.wrapping_shl(8).wrapping_add(*bin.add(bin_pos) as u32);
            bin_pos += 1;
            acc_len = acc_len.wrapping_add(8);
            while acc_len >= 6 {
                acc_len -= 6;
                *b64.add(b64_pos) =
                    b64_byte_to_char(acc.wrapping_shr(acc_len as u32) & 0x3F) as u8 as c_char;
                b64_pos += 1;
            }
        }
        if acc_len > 0 {
            *b64.add(b64_pos) =
                b64_byte_to_char(acc.wrapping_shl((6 - acc_len) as u32) & 0x3F) as u8 as c_char;
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
        if !(b64_pos < b64_maxlen) {
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
            set_errno(ERANGE);
            return -1;
        }
        /* ACQUIRE_FENCE == (void) 0 */
        c = *b64.add(*b64_pos_p) as c_int;
        if c == b'=' as c_int {
            padding_len -= 1;
        } else if ignore.is_null() || strchr(ignore, c).is_null() {
            set_errno(EINVAL);
            return -1;
        }
        *b64_pos_p = (*b64_pos_p).wrapping_add(1);
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
    let is_urlsafe: c_int;
    let mut ret: c_int = 0;
    let mut acc: u32 = 0;
    let mut d: u32;
    let mut c: c_char;

    sodium_base64_check_variant(variant);
    is_urlsafe = ((variant as u32) & VARIANT_URLSAFE_MASK) as c_int;
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
        acc = acc.wrapping_shl(6).wrapping_add(d);
        acc_len = acc_len.wrapping_add(6);
        if acc_len >= 8 {
            acc_len -= 8;
            if bin_pos >= bin_maxlen {
                set_errno(ERANGE);
                ret = -1;
                break;
            }
            *bin.add(bin_pos) = (acc.wrapping_shr(acc_len as u32) & 0xFF) as u8;
            bin_pos += 1;
        }
        b64_pos += 1;
    }
    if acc_len > 4 || (acc & (1u32.wrapping_shl(acc_len as u32)).wrapping_sub(1)) != 0 {
        ret = -1;
    } else if ret == 0 && ((variant as u32) & VARIANT_NO_PADDING_MASK) == 0 {
        ret = _sodium_base642bin_skip_padding(b64, b64_len, &mut b64_pos, ignore, acc_len / 2);
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
        set_errno(EINVAL);
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
    if ((ch as u32) | 32u32) >= b'a' as u32 && ((ch as u32) | 32u32) <= b'f' as u32 {
        return (((ch as u32) | 32u32)
            .wrapping_sub(b'a' as u32)
            .wrapping_add(10)) as c_int;
    }
    -1
}

unsafe fn parse_ipv4(src: *const c_char, end: *const c_char, out: *mut u8) -> c_int {
    let mut p: *const c_char = src;
    let mut i: c_int;

    if src.is_null() || end.is_null() || out.is_null() || src >= end {
        return 0;
    }
    i = 0;
    while i < 4 {
        let mut val: u32 = 0;
        let mut digits: c_int = 0;

        while p < end && *p >= b'0' as c_char && *p <= b'9' as c_char {
            val = val
                .wrapping_mul(10)
                .wrapping_add((*p as c_int - b'0' as c_int) as u32);
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
            if p >= end {
                return 0;
            }
            let ch = *p;
            p = p.add(1);
            if ch != b'.' as c_char {
                return 0;
            }
        }
        i += 1;
    }
    (p == end) as c_int
}

unsafe fn parse_ipv6(src: *const c_char, end: *const c_char, out: *mut u8) -> c_int {
    let mut tmp: [u8; 16] = [0; 16];
    /* `tp`, `endp` and `colonp` are kept as offsets into `tmp` so that the
     * one-past-the-end arithmetic of the C original stays well defined. */
    let mut tp: usize = 0;
    let endp: usize = 16;
    let mut colonp: usize = 0;
    let mut have_colonp: bool = false;
    let mut p: *const c_char = src;
    let mut curtok: *const c_char = src;
    let mut val: u32 = 0;
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
        have_colonp = true;
        p = p.add(1);
        curtok = p;
    }
    while p < end {
        ch = *p as c_int;

        if ch == b':' as c_int {
            if saw_xdigit == 0 {
                if have_colonp {
                    return 0;
                }
                colonp = tp;
                have_colonp = true;
                p = p.add(1);
                curtok = p;
                continue;
            }
            if tp + 2 > endp {
                return 0;
            }
            tmp[tp] = (val >> 8) as u8;
            tp += 1;
            tmp[tp] = (val & 0xff) as u8;
            tp += 1;
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
            if tp + 4 > endp || parse_ipv4(curtok, end, tmp.as_mut_ptr().add(tp)) == 0 {
                return 0;
            }
            tp += 4;
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
        if tp + 2 > endp {
            return 0;
        }
        tmp[tp] = (val >> 8) as u8;
        tp += 1;
        tmp[tp] = (val & 0xff) as u8;
        tp += 1;
    }
    if have_colonp {
        let n: usize = tp - colonp;

        if tp == endp {
            return 0;
        }
        memmove(
            tmp.as_mut_ptr().add(endp - n),
            tmp.as_ptr().add(colonp),
            n,
        );
        memset(tmp.as_mut_ptr().add(colonp), 0, endp - n - colonp);
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
    ip_len_: usize, /* Some AIX versions define a macro named "ip_len" */
) -> c_int {
    let ip_end: *const c_char = ip.add(ip_len_);
    let mut end: *const c_char = ip;
    let zone: *const c_char;
    let mut z: *const c_char;
    let mut v4: [u8; 4] = [0; 4];
    let is_ipv6: c_int;

    while end < ip_end && *end != 0 {
        end = end.add(1);
    }
    zone = memchr(ip, b'%' as c_int, end.offset_from(ip) as usize);
    if !zone.is_null() {
        z = zone.add(1);
        while z < end {
            if !((*z >= b'0' as c_char && *z <= b'9' as c_char)
                || (*z >= b'a' as c_char && *z <= b'z' as c_char)
                || (*z >= b'A' as c_char && *z <= b'Z' as c_char)
                || *z == b'-' as c_char
                || *z == b'_' as c_char
                || *z == b'.' as c_char)
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
    is_ipv6 = (!memchr(ip, b':' as c_int, end.offset_from(ip) as usize).is_null()) as c_int;
    if !zone.is_null() && is_ipv6 == 0 {
        return -1;
    }
    if is_ipv6 != 0 {
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

static ipv4_mapped_prefix: [u8; 12] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff];

unsafe fn ip_write_num(p: &mut *mut c_char, mut val: u32, base: c_int) {
    let mut buf: [c_char; 4] = [0; 4];
    let mut n: c_int = 0;

    loop {
        let d: u32 = val % (base as u32);

        buf[n as usize] = (if d < 10 {
            (b'0' as u32).wrapping_add(d)
        } else {
            (b'a' as u32).wrapping_add(d).wrapping_sub(10)
        }) as u8 as c_char;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn sodium_bin2ip(
    ip: *mut c_char,
    ip_maxlen: usize,
    bin: *const u8,
) -> *mut c_char {
    let mut buf: [c_char; 46] = [0; 46];
    let mut p: *mut c_char = buf.as_mut_ptr();
    let mut i: c_int;
    let mut best_start: c_int = -1;
    let mut best_len: c_int = 0;
    let mut cur_start: c_int = -1;
    let mut cur_len: c_int = 0;
    let len: usize;

    if ip_maxlen <= 2 {
        return core::ptr::null_mut();
    }
    if memcmp(bin, ipv4_mapped_prefix.as_ptr(), 12) == 0 {
        i = 0;
        while i < 4 {
            if i != 0 {
                *p = b'.' as c_char;
                p = p.add(1);
            }
            ip_write_num(&mut p, *bin.add(12 + i as usize) as u32, 10);
            i += 1;
        }
        let len = p.offset_from(buf.as_ptr()) as usize;
        if len >= ip_maxlen {
            return core::ptr::null_mut();
        }
        memcpy(
            ip as *mut u8,
            buf.as_ptr() as *const u8,
            len.wrapping_add(1),
        );
        *ip.add(len) = 0;

        return ip;
    }
    i = 0;
    while i < 8 {
        let word: u32 =
            ((*bin.add((i * 2) as usize) as u32) << 8) | (*bin.add((i * 2 + 1) as usize) as u32);

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
            ((*bin.add((i * 2) as usize) as u32) << 8) | (*bin.add((i * 2 + 1) as usize) as u32),
            16,
        );
        i += 1;
    }
    len = p.offset_from(buf.as_ptr()) as usize;
    if len >= ip_maxlen {
        return core::ptr::null_mut();
    }
    memcpy(ip as *mut u8, buf.as_ptr() as *const u8, len);
    *ip.add(len) = 0;

    ip
}
