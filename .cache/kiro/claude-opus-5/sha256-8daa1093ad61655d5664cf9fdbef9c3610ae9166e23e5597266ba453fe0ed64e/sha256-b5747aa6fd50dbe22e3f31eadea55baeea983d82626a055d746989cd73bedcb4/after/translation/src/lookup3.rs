//! Translation of `src/lookup3.h` (Bob Jenkins' hashlittle, public domain).
//!
//! Only the byte-at-a-time branch is implemented: on a little endian machine
//! all three branches of the C original compute the same value for the same
//! input bytes, they only differ in how many bytes are loaded at a time.

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

pub unsafe fn hashlittle(key: *const u8, length: usize, initval: u32) -> u32 {
    let mut a: u32;
    let mut b: u32;
    let mut c: u32;

    a = 0xdeadbeefu32
        .wrapping_add(length as u32)
        .wrapping_add(initval);
    b = a;
    c = a;

    let mut k = key;
    let mut length = length;

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

    /* last block: all the case statements fall through */
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
    if length >= 1 {
        a = a.wrapping_add(*k.add(0) as u32);
    } else {
        /* zero length strings require no mixing */
        return c;
    }

    final_mix(&mut a, &mut b, &mut c);
    c
}

#[inline]
pub fn hashsize(n: usize) -> usize {
    1usize << n
}

#[inline]
pub fn hashmask(n: usize) -> usize {
    hashsize(n) - 1
}
