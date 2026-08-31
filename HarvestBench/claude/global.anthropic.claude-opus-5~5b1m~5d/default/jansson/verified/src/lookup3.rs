//! Translation of `src/lookup3.h` (Bob Jenkins' lookup3 hash, public domain).

use core::ffi::c_void;

#[inline]
pub fn hashsize(n: usize) -> usize {
    1usize << n
}

#[inline]
pub fn hashmask(n: usize) -> usize {
    hashsize(n) - 1
}

#[inline(always)]
fn rot(x: u32, k: u32) -> u32 {
    (x << k) | (x >> (32 - k))
}

#[inline(always)]
fn mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *a = a.wrapping_sub(*c);
    *a ^= rot(*c, 4);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= rot(*a, 6);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= rot(*b, 8);
    *b = b.wrapping_add(*a);
    *a = a.wrapping_sub(*c);
    *a ^= rot(*c, 16);
    *c = c.wrapping_add(*b);
    *b = b.wrapping_sub(*a);
    *b ^= rot(*a, 19);
    *a = a.wrapping_add(*c);
    *c = c.wrapping_sub(*b);
    *c ^= rot(*b, 4);
    *b = b.wrapping_add(*a);
}

#[inline(always)]
fn final_mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 14));
    *a ^= *c;
    *a = a.wrapping_sub(rot(*c, 11));
    *b ^= *a;
    *b = b.wrapping_sub(rot(*a, 25));
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 16));
    *a ^= *c;
    *a = a.wrapping_sub(rot(*c, 4));
    *b ^= *a;
    *b = b.wrapping_sub(rot(*a, 14));
    *c ^= *b;
    *c = c.wrapping_sub(rot(*b, 24));
}

/// `hashlittle()` -- hash a variable-length key into a 32-bit value.
///
/// This mirrors the C implementation bit for bit, including the "masking
/// trick" that reads up to three bytes past the end of a word-aligned key.
pub unsafe fn hashlittle(key: *const c_void, mut length: usize, initval: u32) -> u32 {
    let mut a: u32;
    let mut b: u32;
    let mut c: u32;

    /* Set up the internal state */
    a = 0xdeadbeefu32
        .wrapping_add(length as u32)
        .wrapping_add(initval);
    b = a;
    c = a;

    let ui = key as usize;
    if (ui & 0x3) == 0 {
        let mut k = key as *const u32; /* read 32-bit chunks */

        /*------ all but last block: aligned reads and affect 32 bits of (a,b,c) */
        while length > 12 {
            a = a.wrapping_add(*k.add(0));
            b = b.wrapping_add(*k.add(1));
            c = c.wrapping_add(*k.add(2));
            mix(&mut a, &mut b, &mut c);
            length -= 12;
            k = k.add(3);
        }

        /*----------------------------- handle the last (probably partial) block */
        match length {
            12 => {
                c = c.wrapping_add(*k.add(2));
                b = b.wrapping_add(*k.add(1));
                a = a.wrapping_add(*k.add(0));
            }
            11 => {
                c = c.wrapping_add(*k.add(2) & 0xffffff);
                b = b.wrapping_add(*k.add(1));
                a = a.wrapping_add(*k.add(0));
            }
            10 => {
                c = c.wrapping_add(*k.add(2) & 0xffff);
                b = b.wrapping_add(*k.add(1));
                a = a.wrapping_add(*k.add(0));
            }
            9 => {
                c = c.wrapping_add(*k.add(2) & 0xff);
                b = b.wrapping_add(*k.add(1));
                a = a.wrapping_add(*k.add(0));
            }
            8 => {
                b = b.wrapping_add(*k.add(1));
                a = a.wrapping_add(*k.add(0));
            }
            7 => {
                b = b.wrapping_add(*k.add(1) & 0xffffff);
                a = a.wrapping_add(*k.add(0));
            }
            6 => {
                b = b.wrapping_add(*k.add(1) & 0xffff);
                a = a.wrapping_add(*k.add(0));
            }
            5 => {
                b = b.wrapping_add(*k.add(1) & 0xff);
                a = a.wrapping_add(*k.add(0));
            }
            4 => {
                a = a.wrapping_add(*k.add(0));
            }
            3 => {
                a = a.wrapping_add(*k.add(0) & 0xffffff);
            }
            2 => {
                a = a.wrapping_add(*k.add(0) & 0xffff);
            }
            1 => {
                a = a.wrapping_add(*k.add(0) & 0xff);
            }
            0 => return c, /* zero length strings require no mixing */
            _ => {}
        }
    } else if (ui & 0x1) == 0 {
        let mut k = key as *const u16; /* read 16-bit chunks */

        /*--------------- all but last block: aligned reads and different mixing */
        while length > 12 {
            a = a.wrapping_add(*k.add(0) as u32 + ((*k.add(1) as u32) << 16));
            b = b.wrapping_add(*k.add(2) as u32 + ((*k.add(3) as u32) << 16));
            c = c.wrapping_add(*k.add(4) as u32 + ((*k.add(5) as u32) << 16));
            mix(&mut a, &mut b, &mut c);
            length -= 12;
            k = k.add(6);
        }

        /*----------------------------- handle the last (probably partial) block */
        let k8 = key as *const u8;
        match length {
            12 => {
                c = c.wrapping_add(*k.add(4) as u32 + ((*k.add(5) as u32) << 16));
                b = b.wrapping_add(*k.add(2) as u32 + ((*k.add(3) as u32) << 16));
                a = a.wrapping_add(*k.add(0) as u32 + ((*k.add(1) as u32) << 16));
            }
            11 => {
                c = c.wrapping_add((*k8.add(10) as u32) << 16);
                c = c.wrapping_add(*k.add(4) as u32);
                b = b.wrapping_add(*k.add(2) as u32 + ((*k.add(3) as u32) << 16));
                a = a.wrapping_add(*k.add(0) as u32 + ((*k.add(1) as u32) << 16));
            }
            10 => {
                c = c.wrapping_add(*k.add(4) as u32);
                b = b.wrapping_add(*k.add(2) as u32 + ((*k.add(3) as u32) << 16));
                a = a.wrapping_add(*k.add(0) as u32 + ((*k.add(1) as u32) << 16));
            }
            9 => {
                c = c.wrapping_add(*k8.add(8) as u32);
                b = b.wrapping_add(*k.add(2) as u32 + ((*k.add(3) as u32) << 16));
                a = a.wrapping_add(*k.add(0) as u32 + ((*k.add(1) as u32) << 16));
            }
            8 => {
                b = b.wrapping_add(*k.add(2) as u32 + ((*k.add(3) as u32) << 16));
                a = a.wrapping_add(*k.add(0) as u32 + ((*k.add(1) as u32) << 16));
            }
            7 => {
                b = b.wrapping_add((*k8.add(6) as u32) << 16);
                b = b.wrapping_add(*k.add(2) as u32);
                a = a.wrapping_add(*k.add(0) as u32 + ((*k.add(1) as u32) << 16));
            }
            6 => {
                b = b.wrapping_add(*k.add(2) as u32);
                a = a.wrapping_add(*k.add(0) as u32 + ((*k.add(1) as u32) << 16));
            }
            5 => {
                b = b.wrapping_add(*k8.add(4) as u32);
                a = a.wrapping_add(*k.add(0) as u32 + ((*k.add(1) as u32) << 16));
            }
            4 => {
                a = a.wrapping_add(*k.add(0) as u32 + ((*k.add(1) as u32) << 16));
            }
            3 => {
                a = a.wrapping_add((*k8.add(2) as u32) << 16);
                a = a.wrapping_add(*k.add(0) as u32);
            }
            2 => {
                a = a.wrapping_add(*k.add(0) as u32);
            }
            1 => {
                a = a.wrapping_add(*k8.add(0) as u32);
            }
            0 => return c, /* zero length requires no mixing */
            _ => {}
        }
    } else {
        /* need to read the key one byte at a time */
        let mut k = key as *const u8;

        /*--------------- all but the last block: affect some 32 bits of (a,b,c) */
        while length > 12 {
            a = a.wrapping_add(*k.add(0) as u32);
            a = a.wrapping_add((*k.add(1) as u32) << 8);
            a = a.wrapping_add((*k.add(2) as u32) << 16);
            a = a.wrapping_add((*k.add(3) as u32) << 24);
            b = b.wrapping_add(*k.add(4) as u32);
            b = b.wrapping_add((*k.add(5) as u32) << 8);
            b = b.wrapping_add((*k.add(6) as u32) << 16);
            b = b.wrapping_add((*k.add(7) as u32) << 24);
            c = c.wrapping_add(*k.add(8) as u32);
            c = c.wrapping_add((*k.add(9) as u32) << 8);
            c = c.wrapping_add((*k.add(10) as u32) << 16);
            c = c.wrapping_add((*k.add(11) as u32) << 24);
            mix(&mut a, &mut b, &mut c);
            length -= 12;
            k = k.add(12);
        }

        /*-------------------------------- last block: affect all 32 bits of (c) */
        /* all the case statements fall through */
        if length == 0 {
            return c;
        }
        if length >= 12 {
            c = c.wrapping_add((*k.add(11) as u32) << 24);
        }
        if length >= 11 {
            c = c.wrapping_add((*k.add(10) as u32) << 16);
        }
        if length >= 10 {
            c = c.wrapping_add((*k.add(9) as u32) << 8);
        }
        if length >= 9 {
            c = c.wrapping_add(*k.add(8) as u32);
        }
        if length >= 8 {
            b = b.wrapping_add((*k.add(7) as u32) << 24);
        }
        if length >= 7 {
            b = b.wrapping_add((*k.add(6) as u32) << 16);
        }
        if length >= 6 {
            b = b.wrapping_add((*k.add(5) as u32) << 8);
        }
        if length >= 5 {
            b = b.wrapping_add(*k.add(4) as u32);
        }
        if length >= 4 {
            a = a.wrapping_add((*k.add(3) as u32) << 24);
        }
        if length >= 3 {
            a = a.wrapping_add((*k.add(2) as u32) << 16);
        }
        if length >= 2 {
            a = a.wrapping_add((*k.add(1) as u32) << 8);
        }
        a = a.wrapping_add(*k.add(0) as u32);
    }

    final_mix(&mut a, &mut b, &mut c);
    c
}
