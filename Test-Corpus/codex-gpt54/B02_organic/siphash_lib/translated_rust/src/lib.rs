use std::ffi::{c_int, c_void};
use std::io::{self, Write};
use std::mem;

#[inline]
fn rot(x: usize, n: u32) -> usize {
    let bits = (mem::size_of::<usize>() * 8) as u32;
    (x << n) | (x >> (bits - n))
}

#[inline]
fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    let half_bits = (mem::size_of::<usize>() * 8 / 2) as u32;

    *v0 = v0.wrapping_add(*v1);
    *v1 = rot(*v1, 13);
    *v1 ^= *v0;
    *v0 = rot(*v0, half_bits);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rot(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rot(*v1, 17);
    *v1 ^= *v2;
    *v2 = rot(*v2, half_bits);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rot(*v3, 21);
    *v3 ^= *v0;
}

#[inline]
unsafe fn load_byte(d: *const u8, idx: usize) -> usize {
    *d.add(idx) as usize
}

unsafe fn stbds_siphash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let mut d = p as *const u8;
    let mut i: usize;
    let mut j: usize;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;
    let word_bytes = mem::size_of::<usize>();
    let bits = word_bytes * 8;

    v0 = ((((0x736f6d65usize) << 16) << 16).wrapping_add(0x70736575usize)) ^ seed;
    v1 = ((((0x646f7261usize) << 16) << 16).wrapping_add(0x6e646f6dusize)) ^ !seed;
    v2 = ((((0x6c796765usize) << 16) << 16).wrapping_add(0x6e657261usize)) ^ seed;
    v3 = ((((0x74656462usize) << 16) << 16).wrapping_add(0x79746573usize)) ^ !seed;
    v0 ^= (0x0706050403020100u64 as usize) ^ seed;
    v1 ^= (0x0f0e0d0c0b0a0908u64 as usize) ^ !seed;
    v2 ^= (0x0706050403020100u64 as usize) ^ seed;
    v3 ^= (0x0f0e0d0c0b0a0908u64 as usize) ^ !seed;

    i = 0;
    while i + word_bytes <= len {
        data = load_byte(d, 0)
            | (load_byte(d, 1) << 8)
            | (load_byte(d, 2) << 16)
            | (load_byte(d, 3) << 24);
        data |= ((load_byte(d, 4)
            | (load_byte(d, 5) << 8)
            | (load_byte(d, 6) << 16)
            | (load_byte(d, 7) << 24))
            << 16)
            << 16;

        v3 ^= data;
        j = 0;
        while j < 2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
            j += 1;
        }
        v0 ^= data;

        i += word_bytes;
        d = d.wrapping_add(word_bytes);
    }

    data = len << (bits - 8);
    match len - i {
        7 => {
            data |= load_byte(d, 6) << 48;
            data |= load_byte(d, 5) << 40;
            data |= load_byte(d, 4) << 32;
            data |= load_byte(d, 3) << 24;
            data |= load_byte(d, 2) << 16;
            data |= load_byte(d, 1) << 8;
            data |= load_byte(d, 0);
        }
        6 => {
            data |= load_byte(d, 5) << 40;
            data |= load_byte(d, 4) << 32;
            data |= load_byte(d, 3) << 24;
            data |= load_byte(d, 2) << 16;
            data |= load_byte(d, 1) << 8;
            data |= load_byte(d, 0);
        }
        5 => {
            data |= load_byte(d, 4) << 32;
            data |= load_byte(d, 3) << 24;
            data |= load_byte(d, 2) << 16;
            data |= load_byte(d, 1) << 8;
            data |= load_byte(d, 0);
        }
        4 => {
            data |= load_byte(d, 3) << 24;
            data |= load_byte(d, 2) << 16;
            data |= load_byte(d, 1) << 8;
            data |= load_byte(d, 0);
        }
        3 => {
            data |= load_byte(d, 2) << 16;
            data |= load_byte(d, 1) << 8;
            data |= load_byte(d, 0);
        }
        2 => {
            data |= load_byte(d, 1) << 8;
            data |= load_byte(d, 0);
        }
        1 => {
            data |= load_byte(d, 0);
        }
        0 => {}
        _ => unreachable!(),
    }

    v3 ^= data;
    j = 0;
    while j < 2 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        j += 1;
    }
    v0 ^= data;
    v2 ^= 0xff;
    j = 0;
    while j < 4 {
        sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        j += 1;
    }
    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn siphash(init: c_int) {
    let mut mem = [0u8; 64];
    let mut z = init;

    for byte in &mut mem {
        *byte = z as u8;
        z = z.wrapping_add(1);
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    for i in 0..64usize {
        let hash = unsafe { stbds_hash_bytes(mem.as_mut_ptr().cast::<c_void>(), i, 0) };
        let _ = out.write_all(b"  { ");
        for j in 0..8usize {
            let byte = ((hash >> (j * 8)) & 255) as u8;
            let _ = write!(out, "0x{byte:02x}, ");
        }
        let _ = out.write_all(b" },\n");
    }
}
