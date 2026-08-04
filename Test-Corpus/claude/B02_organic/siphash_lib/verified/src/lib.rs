use core::ffi::{c_char, c_int, c_uint};

extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[inline(always)]
fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    let bits = (core::mem::size_of::<usize>() * 8) as u32;
    *v0 = v0.wrapping_add(*v1);
    *v1 = (*v1 << 13) | (*v1 >> (bits - 13));
    *v1 ^= *v0;
    *v0 = (*v0 << (bits / 2)) | (*v0 >> (bits - bits / 2));
    *v2 = v2.wrapping_add(*v3);
    *v3 = (*v3 << 16) | (*v3 >> (bits - 16));
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = (*v1 << 17) | (*v1 >> (bits - 17));
    *v1 ^= *v2;
    *v2 = (*v2 << (bits / 2)) | (*v2 >> (bits - bits / 2));
    *v0 = v0.wrapping_add(*v3);
    *v3 = (*v3 << 21) | (*v3 >> (bits - 21));
    *v3 ^= *v0;
}

/// Translation of stbds_siphash_bytes from c_src/src/lib.c
///
/// Note: this function reproduces certain C-implementation-defined behaviors
/// of the original code (e.g. the sign-extension that occurs when shifting
/// a byte into the sign bit of an `int` and then assigning to `size_t`).
unsafe fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let mut d = p;
    let usize_bytes = core::mem::size_of::<usize>();

    let mut v0: usize =
        ((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575) ^ seed;
    let mut v1: usize =
        ((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d) ^ !seed;
    let mut v2: usize =
        ((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261) ^ seed;
    let mut v3: usize =
        ((0x74656462usize << 16) << 16).wrapping_add(0x79746573) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut data: usize;
    let mut i: usize = 0;
    while i + usize_bytes <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // In C, the unsigned chars are promoted to int. If d[3] >= 0x80,
        // (d[3] << 24) sets the sign bit of the int, and assignment to size_t
        // sign-extends. Mimic that here.
        let b0 = *d as u32;
        let b1 = *d.add(1) as u32;
        let b2 = *d.add(2) as u32;
        let b3 = *d.add(3) as u32;
        let lo = b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
        data = (lo as i32) as usize;

        // data |= (size_t)(d[4] | (d[5]<<8) | (d[6]<<16) | (d[7]<<24)) << 16 << 16;
        let b4 = *d.add(4) as u32;
        let b5 = *d.add(5) as u32;
        let b6 = *d.add(6) as u32;
        let b7 = *d.add(7) as u32;
        let hi = b4 | (b5 << 8) | (b6 << 16) | (b7 << 24);
        data |= (((hi as i32) as usize) << 16) << 16;

        v3 ^= data;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i = i.wrapping_add(usize_bytes);
        d = d.add(usize_bytes);
    }

    data = len << ((usize_bytes * 8) - 8);
    let remain = len - i;
    // C switch with intentional fall-through. The cases assume size_t == 64 bits.
    if remain >= 7 {
        data |= ((*d.add(6) as usize) << 24) << 24;
    }
    if remain >= 6 {
        data |= ((*d.add(5) as usize) << 20) << 20;
    }
    if remain >= 5 {
        data |= ((*d.add(4) as usize) << 16) << 16;
    }
    if remain >= 4 {
        // (d[3] << 24) — same int/sign-extension as in the main loop.
        data |= (((*d.add(3) as u32) << 24) as i32) as usize;
    }
    if remain >= 3 {
        data |= (*d.add(2) as usize) << 16;
    }
    if remain >= 2 {
        data |= (*d.add(1) as usize) << 8;
    }
    if remain >= 1 {
        data |= *d as usize;
    }

    v3 ^= data;
    for _ in 0..2 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn siphash(init: c_int) {
    let mut mem: [u8; 64] = [0; 64];
    let mut z: c_int = init;
    for i in 0..64 {
        // Match C: `mem[i] = z;` where z is int — only the low byte is stored.
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }

    for i in 0..64usize {
        let hash = stbds_hash_bytes(mem.as_ptr(), i, 0);
        printf(b"  { \0".as_ptr() as *const c_char);
        for j in 0..8usize {
            let byte = ((hash >> (j * 8)) & 255) as u8;
            printf(
                b"0x%02x, \0".as_ptr() as *const c_char,
                byte as c_uint,
            );
        }
        printf(b" },\n\0".as_ptr() as *const c_char);
    }
}
