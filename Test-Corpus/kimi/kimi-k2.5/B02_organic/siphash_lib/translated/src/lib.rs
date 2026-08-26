use std::os::raw::{c_int, c_uchar};

fn stbds_siphash_bytes(p: *const c_uchar, len: usize, seed: usize) -> usize {
    let d = p;
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;
    let mut data: usize;
    
    v0 = ((0x736f6d65usize.wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x70736575) ^ seed;
    v1 = ((0x646f7261usize.wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x6e646f6d) ^ !seed;
    v2 = ((0x6c796765usize.wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x6e657261) ^ seed;
    v3 = ((0x74656462usize.wrapping_shl(16)).wrapping_shl(16)).wrapping_add(0x79746573) ^ !seed;
    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    
    let mut i = 0usize;
    while i + std::mem::size_of::<usize>() <= len {
        unsafe {
            data = (*d.add(i) as usize)
                | ((*d.add(i + 1) as usize) << 8)
                | ((*d.add(i + 2) as usize) << 16)
                | ((*d.add(i + 3) as usize) << 24);
            data |= ((*d.add(i + 4) as usize)
                | ((*d.add(i + 5) as usize) << 8)
                | ((*d.add(i + 6) as usize) << 16)
                | ((*d.add(i + 7) as usize) << 24))
                .wrapping_shl(16)
                .wrapping_shl(16);
        }
        v3 ^= data;
        for _ in 0..2 {
            v0 = v0.wrapping_add(v1);
            v1 = v1.rotate_left(13);
            v1 ^= v0;
            v0 = v0.rotate_left(std::mem::size_of::<usize>() * 4);
            v2 = v2.wrapping_add(v3);
            v3 = v3.rotate_left(16);
            v3 ^= v2;
            v2 = v2.wrapping_add(v1);
            v1 = v1.rotate_left(17);
            v1 ^= v2;
            v2 = v2.rotate_left(std::mem::size_of::<usize>() * 4);
            v0 = v0.wrapping_add(v3);
            v3 = v3.rotate_left(21);
            v3 ^= v0;
        }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }
    
    data = len.wrapping_shl((std::mem::size_of::<usize>() * 8) - 8);
    match len - i {
        7 => {
            unsafe { data |= ((*d.add(i + 6) as usize) << 24).wrapping_shl(24); }
            data |= unsafe { ((*d.add(i + 5) as usize) << 20).wrapping_shl(20); }
            data |= unsafe { ((*d.add(i + 4) as usize) << 16).wrapping_shl(16); }
            data |= unsafe { (*d.add(i + 3) as usize) << 24; }
            data |= unsafe { (*d.add(i + 2) as usize) << 16; }
            data |= unsafe { (*d.add(i + 1) as usize) << 8; }
            data |= unsafe { *d.add(i) as usize; }
        }
        6 => {
            data |= unsafe { ((*d.add(i + 5) as usize) << 20).wrapping_shl(20); }
            data |= unsafe { ((*d.add(i + 4) as usize) << 16).wrapping_shl(16); }
            data |= unsafe { (*d.add(i + 3) as usize) << 24; }
            data |= unsafe { (*d.add(i + 2) as usize) << 16; }
            data |= unsafe { (*d.add(i + 1) as usize) << 8; }
            data |= unsafe { *d.add(i) as usize; }
        }
        5 => {
            data |= unsafe { ((*d.add(i + 4) as usize) << 16).wrapping_shl(16); }
            data |= unsafe { (*d.add(i + 3) as usize) << 24; }
            data |= unsafe { (*d.add(i + 2) as usize) << 16; }
            data |= unsafe { (*d.add(i + 1) as usize) << 8; }
            data |= unsafe { *d.add(i) as usize; }
        }
        4 => {
            data |= unsafe { (*d.add(i + 3) as usize) << 24; }
            data |= unsafe { (*d.add(i + 2) as usize) << 16; }
            data |= unsafe { (*d.add(i + 1) as usize) << 8; }
            data |= unsafe { *d.add(i) as usize; }
        }
        3 => {
            data |= unsafe { (*d.add(i + 2) as usize) << 16; }
            data |= unsafe { (*d.add(i + 1) as usize) << 8; }
            data |= unsafe { *d.add(i) as usize; }
        }
        2 => {
            data |= unsafe { (*d.add(i + 1) as usize) << 8; }
            data |= unsafe { *d.add(i) as usize; }
        }
        1 => {
            data |= unsafe { *d.add(i) as usize; }
        }
        _ => {}
    }
    
    v3 ^= data;
    for _ in 0..2 {
        v0 = v0.wrapping_add(v1);
        v1 = v1.rotate_left(13);
        v1 ^= v0;
        v0 = v0.rotate_left(std::mem::size_of::<usize>() * 4);
        v2 = v2.wrapping_add(v3);
        v3 = v3.rotate_left(16);
        v3 ^= v2;
        v2 = v2.wrapping_add(v1);
        v1 = v1.rotate_left(17);
        v1 ^= v2;
        v2 = v2.rotate_left(std::mem::size_of::<usize>() * 4);
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
        v0 = v0.rotate_left(std::mem::size_of::<usize>() * 4);
        v2 = v2.wrapping_add(v3);
        v3 = v3.rotate_left(16);
        v3 ^= v2;
        v2 = v2.wrapping_add(v1);
        v1 = v1.rotate_left(17);
        v1 ^= v2;
        v2 = v2.rotate_left(std::mem::size_of::<usize>() * 4);
        v0 = v0.wrapping_add(v3);
        v3 = v3.rotate_left(21);
        v3 ^= v0;
    }
    v0 ^ v1 ^ v2 ^ v3
}

fn stbds_hash_bytes(p: *const c_uchar, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn siphash(init: c_int) {
    let mut mem: [c_uchar; 64] = [0; 64];
    let mut z = init;
    for i in 0..64 {
        mem[i] = z as c_uchar;
        z = z.wrapping_add(1);
    }
    for i in 0..64 {
        let hash = stbds_hash_bytes(mem.as_ptr(), i, 0);
        print!("  {{ ");
        for j in 0..8 {
            print!("0x{:02x}, ", ((hash >> (j * 8)) & 255) as c_uchar);
        }
        println!(" }},");
    }
}
