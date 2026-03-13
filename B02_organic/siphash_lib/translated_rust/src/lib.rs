use std::ffi::c_int;

fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(32);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let d = p;
    let mut v0 = (((0x736f6d65_usize) << 16 << 16).wrapping_add(0x70736575)) ^ seed;
    let mut v1 = (((0x646f7261_usize) << 16 << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    let mut v2 = (((0x6c796765_usize) << 16 << 16).wrapping_add(0x6e657261)) ^ seed;
    let mut v3 = (((0x74656462_usize) << 16 << 16).wrapping_add(0x79746573)) ^ !seed;
    v0 ^= 0x0706050403020100_usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;
    v2 ^= 0x0706050403020100_usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_usize ^ !seed;

    let sz = std::mem::size_of::<usize>();
    let mut i = 0usize;
    while i + sz <= len {
        let dd = unsafe { d.add(i) };
        let data: usize;
        unsafe {
            // Lower 4 bytes: unsigned char promoted to int (i32) in C, then to size_t
            // d[3] << 24 can set the sign bit of int, which sign-extends to size_t
            let lo = (*dd.add(0) as i32)
                | ((*dd.add(1) as i32) << 8)
                | ((*dd.add(2) as i32) << 16)
                | ((*dd.add(3) as i32) << 24);
            // Upper 4 bytes: same pattern, then << 16 << 16
            let hi = (*dd.add(4) as i32)
                | ((*dd.add(5) as i32) << 8)
                | ((*dd.add(6) as i32) << 16)
                | ((*dd.add(7) as i32) << 24);
            data = (lo as usize) | ((hi as usize) << 16 << 16);
        }
        v3 ^= data;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        i += sz;
    }

    let mut data: usize = len << (sz * 8 - 8);
    let dd = unsafe { d.add(i) };
    let rem = len - i;
    // C fallthrough switch — each case falls into the next
    if rem >= 7 {
        data |= unsafe { (*dd.add(6) as usize) << 24 << 24 };
    }
    if rem >= 6 {
        data |= unsafe { (*dd.add(5) as usize) << 20 << 20 };
    }
    if rem >= 5 {
        data |= unsafe { (*dd.add(4) as usize) << 16 << 16 };
    }
    if rem >= 4 {
        // C: (d[3] << 24) — unsigned char promoted to int
        data |= unsafe { ((*dd.add(3) as i32) << 24) as usize };
    }
    if rem >= 3 {
        data |= unsafe { ((*dd.add(2) as i32) << 16) as usize };
    }
    if rem >= 2 {
        data |= unsafe { ((*dd.add(1) as i32) << 8) as usize };
    }
    if rem >= 1 {
        data |= unsafe { *dd.add(0) as usize };
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
pub extern "C" fn stbds_hash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

#[unsafe(no_mangle)]
pub extern "C" fn siphash(init: c_int) {
    let mut mem = [0u8; 64];
    let mut z = init;
    for i in 0..64 {
        mem[i] = z as u8;
        z += 1;
    }
    for i in 0..64 {
        let hash = stbds_siphash_bytes(mem.as_ptr(), i, 0);
        print!("  {{ ");
        for j in 0..8 {
            print!("0x{:02x}, ", ((hash >> (j * 8)) & 255) as u8);
        }
        println!(" }},");
    }
}
