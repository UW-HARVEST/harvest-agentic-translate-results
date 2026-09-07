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
unsafe fn byte_at(data: *const u8, index: usize) -> usize {
    // This has the same pointer validity requirements as the C implementation.
    unsafe { usize::from(data.add(index).read()) }
}

#[inline]
fn sign_extended_u32(value: usize) -> usize {
    (value as u32 as i32 as isize) as usize
}

unsafe fn siphash_bytes(data: *const u8, len: usize, seed: usize) -> usize {
    let mut v0 = 0x736f6d6570736575usize ^ seed;
    let mut v1 = 0x646f72616e646f6dusize ^ !seed;
    let mut v2 = 0x6c7967656e657261usize ^ seed;
    let mut v3 = 0x7465646279746573usize ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut i = 0usize;
    while i.wrapping_add(size_of::<usize>()) <= len {
        let mut word = sign_extended_u32(
            unsafe { byte_at(data, i) }
                | (unsafe { byte_at(data, i + 1) } << 8)
                | (unsafe { byte_at(data, i + 2) } << 16)
                | (unsafe { byte_at(data, i + 3) } << 24),
        );
        word |= (unsafe { byte_at(data, i + 4) }
            | (unsafe { byte_at(data, i + 5) } << 8)
            | (unsafe { byte_at(data, i + 6) } << 16)
            | (unsafe { byte_at(data, i + 7) } << 24))
            << 32;

        v3 ^= word;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= word;
        i = i.wrapping_add(size_of::<usize>());
    }

    let mut word = len << (usize::BITS - 8);
    let tail = len - i;
    if tail >= 7 {
        word |= unsafe { byte_at(data, i + 6) } << 48;
    }
    if tail >= 6 {
        word |= unsafe { byte_at(data, i + 5) } << 40;
    }
    if tail >= 5 {
        word |= unsafe { byte_at(data, i + 4) } << 32;
    }
    if tail >= 4 {
        word |= sign_extended_u32(unsafe { byte_at(data, i + 3) } << 24);
    }
    if tail >= 3 {
        word |= unsafe { byte_at(data, i + 2) } << 16;
    }
    if tail >= 2 {
        word |= unsafe { byte_at(data, i + 1) } << 8;
    }
    if tail >= 1 {
        word |= unsafe { byte_at(data, i) };
    }

    v3 ^= word;
    for _ in 0..2 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }
    v0 ^= word;
    v2 ^= 0xff;
    for _ in 0..4 {
        sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
    }

    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    unsafe { siphash_bytes(p.cast(), len, seed) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn siphash(init: c_int) {
    const ROW_START: &[u8] = b"  { \0";
    const BYTE: &[u8] = b"0x%02x, \0";
    const ROW_END: &[u8] = b" },\n\0";

    let mut mem = [0u8; 64];
    let mut z = init;
    for byte in &mut mem {
        *byte = z as u8;
        z = z.wrapping_add(1);
    }

    for len in 0..64 {
        let hash = unsafe { siphash_bytes(mem.as_ptr(), len, 0) };
        unsafe {
            printf(ROW_START.as_ptr().cast());
        }
        for j in 0..8 {
            let byte = ((hash >> (j * 8)) & 255) as c_int;
            unsafe {
                printf(BYTE.as_ptr().cast(), byte);
            }
        }
        unsafe {
            printf(ROW_END.as_ptr().cast());
        }
    }
}
