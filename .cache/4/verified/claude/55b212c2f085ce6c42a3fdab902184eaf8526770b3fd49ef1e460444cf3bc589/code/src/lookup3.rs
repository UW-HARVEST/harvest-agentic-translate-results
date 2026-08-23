//! Translation of `src/lookup3.h` (Bob Jenkins' `hashlittle`).
//!
//! `HASH_LITTLE_ENDIAN` is 1 on x86-64 (`__BYTE_ORDER == __LITTLE_ENDIAN`), so
//! the aligned 32-bit and 16-bit read paths are active, including the
//! "masking trick" that reads past the end of the buffer within the same
//! aligned word.  That is reproduced here with `read_unaligned` on the
//! (in practice aligned) pointer, which is why the reads are guarded by the
//! same alignment tests as the original.

use core::ffi::c_void;

#[inline(always)]
fn rot(x: u32, k: u32) -> u32 {
    (x << k) | (x >> (32 - k))
}

#[inline(always)]
pub fn hashsize(n: usize) -> usize {
    1usize << n
}

#[inline(always)]
pub fn hashmask(n: usize) -> usize {
    hashsize(n) - 1
}

macro_rules! mix {
    ($a:ident, $b:ident, $c:ident) => {{
        $a = $a.wrapping_sub($c);
        $a ^= rot($c, 4);
        $c = $c.wrapping_add($b);
        $b = $b.wrapping_sub($a);
        $b ^= rot($a, 6);
        $a = $a.wrapping_add($c);
        $c = $c.wrapping_sub($b);
        $c ^= rot($b, 8);
        $b = $b.wrapping_add($a);
        $a = $a.wrapping_sub($c);
        $a ^= rot($c, 16);
        $c = $c.wrapping_add($b);
        $b = $b.wrapping_sub($a);
        $b ^= rot($a, 19);
        $a = $a.wrapping_add($c);
        $c = $c.wrapping_sub($b);
        $c ^= rot($b, 4);
        $b = $b.wrapping_add($a);
    }};
}

macro_rules! final_mix {
    ($a:ident, $b:ident, $c:ident) => {{
        $c ^= $b;
        $c = $c.wrapping_sub(rot($b, 14));
        $a ^= $c;
        $a = $a.wrapping_sub(rot($c, 11));
        $b ^= $a;
        $b = $b.wrapping_sub(rot($a, 25));
        $c ^= $b;
        $c = $c.wrapping_sub(rot($b, 16));
        $a ^= $c;
        $a = $a.wrapping_sub(rot($c, 4));
        $b ^= $a;
        $b = $b.wrapping_sub(rot($a, 14));
        $c ^= $b;
        $c = $c.wrapping_sub(rot($b, 24));
    }};
}

pub unsafe fn hashlittle(key: *const c_void, length: usize, initval: u32) -> u32 {
    let mut a: u32;
    let mut b: u32;
    let mut c: u32;
    let mut length = length;

    /* Set up the internal state */
    a = 0xdeadbeefu32
        .wrapping_add(length as u32)
        .wrapping_add(initval);
    b = a;
    c = a;

    let ui = key as usize;

    if ui & 0x3 == 0 {
        let mut k = key as *const u32; /* read 32-bit chunks */

        /*--- all but last block: aligned reads and affect 32 bits of (a,b,c) */
        while length > 12 {
            a = a.wrapping_add(core::ptr::read_unaligned(k.add(0)));
            b = b.wrapping_add(core::ptr::read_unaligned(k.add(1)));
            c = c.wrapping_add(core::ptr::read_unaligned(k.add(2)));
            mix!(a, b, c);
            length -= 12;
            k = k.add(3);
        }

        /*-------------------------- handle the last (probably partial) block */
        macro_rules! k0 {
            () => {
                core::ptr::read_unaligned(k.add(0))
            };
        }
        macro_rules! k1 {
            () => {
                core::ptr::read_unaligned(k.add(1))
            };
        }
        macro_rules! k2 {
            () => {
                core::ptr::read_unaligned(k.add(2))
            };
        }

        match length {
            12 => {
                c = c.wrapping_add(k2!());
                b = b.wrapping_add(k1!());
                a = a.wrapping_add(k0!());
            }
            11 => {
                c = c.wrapping_add(k2!() & 0xffffff);
                b = b.wrapping_add(k1!());
                a = a.wrapping_add(k0!());
            }
            10 => {
                c = c.wrapping_add(k2!() & 0xffff);
                b = b.wrapping_add(k1!());
                a = a.wrapping_add(k0!());
            }
            9 => {
                c = c.wrapping_add(k2!() & 0xff);
                b = b.wrapping_add(k1!());
                a = a.wrapping_add(k0!());
            }
            8 => {
                b = b.wrapping_add(k1!());
                a = a.wrapping_add(k0!());
            }
            7 => {
                b = b.wrapping_add(k1!() & 0xffffff);
                a = a.wrapping_add(k0!());
            }
            6 => {
                b = b.wrapping_add(k1!() & 0xffff);
                a = a.wrapping_add(k0!());
            }
            5 => {
                b = b.wrapping_add(k1!() & 0xff);
                a = a.wrapping_add(k0!());
            }
            4 => {
                a = a.wrapping_add(k0!());
            }
            3 => {
                a = a.wrapping_add(k0!() & 0xffffff);
            }
            2 => {
                a = a.wrapping_add(k0!() & 0xffff);
            }
            1 => {
                a = a.wrapping_add(k0!() & 0xff);
            }
            0 => return c, /* zero length strings require no mixing */
            _ => {}
        }
    } else if ui & 0x1 == 0 {
        let mut k = key as *const u16; /* read 16-bit chunks */

        /*------------ all but last block: aligned reads and different mixing */
        while length > 12 {
            a = a.wrapping_add(
                (core::ptr::read_unaligned(k.add(0)) as u32)
                    .wrapping_add((core::ptr::read_unaligned(k.add(1)) as u32) << 16),
            );
            b = b.wrapping_add(
                (core::ptr::read_unaligned(k.add(2)) as u32)
                    .wrapping_add((core::ptr::read_unaligned(k.add(3)) as u32) << 16),
            );
            c = c.wrapping_add(
                (core::ptr::read_unaligned(k.add(4)) as u32)
                    .wrapping_add((core::ptr::read_unaligned(k.add(5)) as u32) << 16),
            );
            mix!(a, b, c);
            length -= 12;
            k = k.add(6);
        }

        /*-------------------------- handle the last (probably partial) block */
        let k8 = k as *const u8;
        macro_rules! h {
            ($i:expr) => {
                core::ptr::read_unaligned(k.add($i)) as u32
            };
        }
        macro_rules! b8 {
            ($i:expr) => {
                *k8.add($i) as u32
            };
        }

        match length {
            12 => {
                c = c.wrapping_add(h!(4).wrapping_add(h!(5) << 16));
                b = b.wrapping_add(h!(2).wrapping_add(h!(3) << 16));
                a = a.wrapping_add(h!(0).wrapping_add(h!(1) << 16));
            }
            11 => {
                c = c.wrapping_add(b8!(10) << 16);
                c = c.wrapping_add(h!(4));
                b = b.wrapping_add(h!(2).wrapping_add(h!(3) << 16));
                a = a.wrapping_add(h!(0).wrapping_add(h!(1) << 16));
            }
            10 => {
                c = c.wrapping_add(h!(4));
                b = b.wrapping_add(h!(2).wrapping_add(h!(3) << 16));
                a = a.wrapping_add(h!(0).wrapping_add(h!(1) << 16));
            }
            9 => {
                c = c.wrapping_add(b8!(8));
                b = b.wrapping_add(h!(2).wrapping_add(h!(3) << 16));
                a = a.wrapping_add(h!(0).wrapping_add(h!(1) << 16));
            }
            8 => {
                b = b.wrapping_add(h!(2).wrapping_add(h!(3) << 16));
                a = a.wrapping_add(h!(0).wrapping_add(h!(1) << 16));
            }
            7 => {
                b = b.wrapping_add(b8!(6) << 16);
                b = b.wrapping_add(h!(2));
                a = a.wrapping_add(h!(0).wrapping_add(h!(1) << 16));
            }
            6 => {
                b = b.wrapping_add(h!(2));
                a = a.wrapping_add(h!(0).wrapping_add(h!(1) << 16));
            }
            5 => {
                b = b.wrapping_add(b8!(4));
                a = a.wrapping_add(h!(0).wrapping_add(h!(1) << 16));
            }
            4 => {
                a = a.wrapping_add(h!(0).wrapping_add(h!(1) << 16));
            }
            3 => {
                a = a.wrapping_add(b8!(2) << 16);
                a = a.wrapping_add(h!(0));
            }
            2 => {
                a = a.wrapping_add(h!(0));
            }
            1 => {
                a = a.wrapping_add(b8!(0));
            }
            0 => return c, /* zero length requires no mixing */
            _ => {}
        }
    } else {
        /* need to read the key one byte at a time */
        let mut k = key as *const u8;

        /*------------ all but the last block: affect some 32 bits of (a,b,c) */
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
            mix!(a, b, c);
            length -= 12;
            k = k.add(12);
        }

        /*---------------------------- last block: affect all 32 bits of (c) */
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

    final_mix!(a, b, c);
    c
}
