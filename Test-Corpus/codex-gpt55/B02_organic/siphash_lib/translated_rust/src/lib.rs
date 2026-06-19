use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[inline]
fn sip_round(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(usize::BITS / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(usize::BITS / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

#[inline]
unsafe fn byte_at(d: *const u8, offset: usize) -> usize {
    unsafe { *d.add(offset) as usize }
}

#[inline]
unsafe fn promoted_int_word(d: *const u8, offset: usize) -> usize {
    let word = unsafe { *d.add(offset) as i32 }
        | ((unsafe { *d.add(offset + 1) as i32 }) << 8)
        | ((unsafe { *d.add(offset + 2) as i32 }) << 16)
        | ((unsafe { *d.add(offset + 3) as i32 }) << 24);
    word as usize
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p.cast::<u8>();
    let mut v0 = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    let mut v1 = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    let mut v2 = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    let mut v3 = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut i = 0usize;
    while i + size_of::<usize>() <= len {
        let mut data = unsafe { promoted_int_word(d, 0) };
        data |= unsafe { promoted_int_word(d, 4) } << 16 << 16;

        v3 ^= data;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += size_of::<usize>();
        d = unsafe { d.add(size_of::<usize>()) };
    }

    let mut data = len << ((size_of::<usize>() * 8) - 8);
    match len - i {
        7 => {
            data |= (unsafe { byte_at(d, 6) } << 24) << 24;
            data |= (unsafe { byte_at(d, 5) } << 20) << 20;
            data |= (unsafe { byte_at(d, 4) } << 16) << 16;
            data |= unsafe { promoted_int_word(d, 0) };
        }
        6 => {
            data |= (unsafe { byte_at(d, 5) } << 20) << 20;
            data |= (unsafe { byte_at(d, 4) } << 16) << 16;
            data |= unsafe { promoted_int_word(d, 0) };
        }
        5 => {
            data |= (unsafe { byte_at(d, 4) } << 16) << 16;
            data |= unsafe { promoted_int_word(d, 0) };
        }
        4 => {
            data |= unsafe { promoted_int_word(d, 0) };
        }
        3 => {
            data |= unsafe { byte_at(d, 2) } << 16;
            data |= unsafe { byte_at(d, 1) } << 8;
            data |= unsafe { byte_at(d, 0) };
        }
        2 => {
            data |= unsafe { byte_at(d, 1) } << 8;
            data |= unsafe { byte_at(d, 0) };
        }
        1 => {
            data |= unsafe { byte_at(d, 0) };
        }
        _ => {}
    }

    v3 ^= data;
    for _ in 0..2 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { stbds_siphash_bytes(p, len, seed) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn siphash(init: c_int) {
    let mut mem = [0u8; 64];
    let mut z = init;
    for byte in &mut mem {
        *byte = z as u8;
        z = z.wrapping_add(1);
    }

    for i in 0..64usize {
        let hash = unsafe { stbds_hash_bytes(mem.as_mut_ptr().cast::<c_void>(), i, 0) };
        unsafe { printf(c"  { ".as_ptr()) };
        for j in 0..8usize {
            let byte = ((hash >> (j * 8)) & 255) as c_int;
            unsafe { printf(c"0x%02x, ".as_ptr(), byte) };
        }
        unsafe { printf(c" },\n".as_ptr()) };
    }
}
