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

fn siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p.cast::<u8>();
    let mut v0 = ((0x736f6d65usize << 32) + 0x70736575) ^ seed;
    let mut v1 = ((0x646f7261usize << 32) + 0x6e646f6d) ^ !seed;
    let mut v2 = ((0x6c796765usize << 32) + 0x6e657261) ^ seed;
    let mut v3 = ((0x74656462usize << 32) + 0x79746573) ^ !seed;
    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let mut i = 0;
    while i + size_of::<usize>() <= len {
        let data = unsafe {
            (i32::from(d.read())
                | (i32::from(d.add(1).read()) << 8)
                | (i32::from(d.add(2).read()) << 16)
                | (i32::from(d.add(3).read()) << 24)) as usize
                | (usize::from(d.add(4).read()) << 32)
                | (usize::from(d.add(5).read()) << 40)
                | (usize::from(d.add(6).read()) << 48)
                | (usize::from(d.add(7).read()) << 56)
        };
        v3 ^= data;
        for _ in 0..2 {
            sip_round(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        i += size_of::<usize>();
        d = unsafe { d.add(size_of::<usize>()) };
    }

    let mut data = len << (usize::BITS - 8);
    let remaining = len - i;
    unsafe {
        if remaining >= 7 {
            data |= usize::from(d.add(6).read()) << 48;
        }
        if remaining >= 6 {
            data |= usize::from(d.add(5).read()) << 40;
        }
        if remaining >= 5 {
            data |= usize::from(d.add(4).read()) << 32;
        }
        if remaining >= 4 {
            data |= (i32::from(d.add(3).read()) << 24) as usize;
        }
        if remaining >= 3 {
            data |= usize::from(d.add(2).read()) << 16;
        }
        if remaining >= 2 {
            data |= usize::from(d.add(1).read()) << 8;
        }
        if remaining >= 1 {
            data |= usize::from(d.read());
        }
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
pub extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    siphash_bytes(p, len, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn siphash(init: c_int) {
    const OPEN: &[u8] = b"  { \0";
    const BYTE: &[u8] = b"0x%02x, \0";
    const CLOSE: &[u8] = b" },\n\0";

    let mut mem = [0u8; 64];
    let mut z = init;
    for byte in &mut mem {
        *byte = z as u8;
        z = z.wrapping_add(1);
    }

    for len in 0..64 {
        let hash = siphash_bytes(mem.as_mut_ptr().cast(), len, 0);
        unsafe {
            printf(OPEN.as_ptr().cast());
            for j in 0..8 {
                let byte = ((hash >> (j * 8)) & 255) as c_int;
                printf(BYTE.as_ptr().cast(), byte);
            }
            printf(CLOSE.as_ptr().cast());
        }
    }
}
