use std::ffi::c_void;
use std::os::raw::c_int;

fn stbds_siphash_bytes(p_ptr: *const c_void, len: usize, seed: usize) -> usize {
    let p = if len > 0 && !p_ptr.is_null() {
        unsafe { std::slice::from_raw_parts(p_ptr as *const u8, len) }
    } else {
        &[]
    };

    let mut v0: usize = ((((0x736f6d65usize).wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x70736575)) ^ seed;
    let mut v1: usize = ((((0x646f7261usize).wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x6e646f6d)) ^ !seed;
    let mut v2: usize = ((((0x6c796765usize).wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x6e657261)) ^ seed;
    let mut v3: usize = ((((0x74656462usize).wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x79746573)) ^ !seed;
    v0 ^= (0x0706050403020100u64 as usize) ^ seed;
    v1 ^= (0x0f0e0d0c0b0a0908u64 as usize) ^ !seed;
    v2 ^= (0x0706050403020100u64 as usize) ^ seed;
    v3 ^= (0x0f0e0d0c0b0a0908u64 as usize) ^ !seed;

    let mut i = 0;
    let mut d_idx = 0;
    while i + std::mem::size_of::<usize>() <= len {
        let mut chunk = [0u8; std::mem::size_of::<usize>()];
        chunk.copy_from_slice(&p[d_idx..d_idx + std::mem::size_of::<usize>()]);
        let data = usize::from_le_bytes(chunk);
        v3 ^= data;
        for _ in 0..2 {
            v0 = v0.wrapping_add(v1);
            v1 = v1.rotate_left(13);
            v1 ^= v0;
            v0 = v0.rotate_left((std::mem::size_of::<usize>() as u32 * 8) / 2);
            v2 = v2.wrapping_add(v3);
            v3 = v3.rotate_left(16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = v1.rotate_left(17);
            v1 ^= v2;
            v2 = v2.rotate_left((std::mem::size_of::<usize>() as u32 * 8) / 2);
            v0 = v0.wrapping_add(v3);
            v3 = v3.rotate_left(21);
            v3 ^= v0;
        }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
        d_idx += std::mem::size_of::<usize>();
    }

    let mut data: usize = len.wrapping_shl((std::mem::size_of::<usize>() as u32 * 8) - 8);
    let rem = len - i;
    if rem >= 7 {
        data |= (p[d_idx + 6] as usize).wrapping_shl(24).wrapping_shl(24);
    }
    if rem >= 6 {
        data |= (p[d_idx + 5] as usize).wrapping_shl(20).wrapping_shl(20);
    }
    if rem >= 5 {
        data |= (p[d_idx + 4] as usize).wrapping_shl(16).wrapping_shl(16);
    }
    if rem >= 4 {
        data |= (p[d_idx + 3] as usize).wrapping_shl(24);
    }
    if rem >= 3 {
        data |= (p[d_idx + 2] as usize).wrapping_shl(16);
    }
    if rem >= 2 {
        data |= (p[d_idx + 1] as usize).wrapping_shl(8);
    }
    if rem >= 1 {
        data |= p[d_idx] as usize;
    }

    v3 ^= data;
    for _ in 0..2 {
        v0 = v0.wrapping_add(v1);
        v1 = v1.rotate_left(13);
        v1 ^= v0;
        v0 = v0.rotate_left((std::mem::size_of::<usize>() as u32 * 8) / 2);
        v2 = v2.wrapping_add(v3);
        v3 = v3.rotate_left(16);
        v3 ^= v2;
        v2 = v2.wrapping_add(v1);
        v1 = v1.rotate_left(17);
        v1 ^= v2;
        v2 = v2.rotate_left((std::mem::size_of::<usize>() as u32 * 8) / 2);
        v0 = v0.wrapping_add(v3);
        v3 = v3.rotate_left(21);
        v3 ^= v0;
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        v0 = v0.wrapping_add(v1);
        v1 = v1.rotate_left(13);
        v1 ^= v0;
        v0 = v0.rotate_left((std::mem::size_of::<usize>() as u32 * 8) / 2);
        v2 = v2.wrapping_add(v3);
        v3 = v3.rotate_left(16);
        v3 ^= v2;
        v2 = v2.wrapping_add(v1);
        v1 = v1.rotate_left(17);
        v1 ^= v2;
        v2 = v2.rotate_left((std::mem::size_of::<usize>() as u32 * 8) / 2);
        v0 = v0.wrapping_add(v3);
        v3 = v3.rotate_left(21);
        v3 ^= v0;
    }
    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hash_bytes(p: *const c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn siphash(init: c_int) {
    let mut mem = [0u8; 64];
    let mut z = init;
    for i in 0..64 {
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }
    for i in 0..64 {
        let hash = stbds_hash_bytes(mem.as_ptr() as *const c_void, i, 0);
        print!("  {{ ");
        for j in 0..8 {
            print!("0x{:02x}, ", (hash.wrapping_shr(j * 8)) & 255);
        }
        println!(" }},");
    }
}
