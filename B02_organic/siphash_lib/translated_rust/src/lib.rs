use std::ffi::c_int;

fn rotl(val: usize, n: u32) -> usize {
    (val << n) | (val >> ((std::mem::size_of::<usize>() * 8) as u32 - n))
}

macro_rules! sipround {
    ($v0:expr, $v1:expr, $v2:expr, $v3:expr) => {
        $v0 = $v0.wrapping_add($v1);
        $v1 = rotl($v1, 13);
        $v1 ^= $v0;
        $v0 = rotl($v0, (std::mem::size_of::<usize>() * 8 / 2) as u32);
        $v2 = $v2.wrapping_add($v3);
        $v3 = rotl($v3, 16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = rotl($v1, 17);
        $v1 ^= $v2;
        $v2 = rotl($v2, (std::mem::size_of::<usize>() * 8 / 2) as u32);
        $v0 = $v0.wrapping_add($v3);
        $v3 = rotl($v3, 21);
        $v3 ^= $v0;
    };
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

    let mut i: usize = 0;
    while i + std::mem::size_of::<usize>() <= len {
        let dp = unsafe { d.add(i) };
        let mut data: usize;
        unsafe {
            let lo = (*dp.add(0) as i32)
                | (*dp.add(1) as i32) << 8
                | (*dp.add(2) as i32) << 16
                | (*dp.add(3) as i32) << 24;
            data = lo as usize;
            let hi = (*dp.add(4) as i32)
                | (*dp.add(5) as i32) << 8
                | (*dp.add(6) as i32) << 16
                | (*dp.add(7) as i32) << 24;
            data |= (hi as usize) << 16 << 16;
        }
        v3 ^= data;
        for _ in 0..2 {
            sipround!(v0, v1, v2, v3);
        }
        v0 ^= data;
        i += std::mem::size_of::<usize>();
    }

    let mut data: usize = len << ((std::mem::size_of::<usize>() * 8 - 8) as u32);
    let dp = unsafe { d.add(i) };
    let rem = len - i;
    // Fallthrough switch: each case falls through to the next.
    // Cases 7..1 accumulate into data, case 0 is break.
    if rem >= 7 {
        data |= (unsafe { *dp.add(6) } as usize) << 24 << 24;
    }
    if rem >= 6 {
        data |= (unsafe { *dp.add(5) } as usize) << 20 << 20;
    }
    if rem >= 5 {
        data |= (unsafe { *dp.add(4) } as usize) << 16 << 16;
    }
    if rem >= 4 {
        // C: (d[3] << 24) — unsigned char promoted to int, shifted 24.
        // If d[3] >= 128, result is negative int, sign-extended to size_t.
        data |= ((unsafe { *dp.add(3) } as i32) << 24) as usize;
    }
    if rem >= 3 {
        data |= (unsafe { *dp.add(2) } as usize) << 16;
    }
    if rem >= 2 {
        data |= (unsafe { *dp.add(1) } as usize) << 8;
    }
    if rem >= 1 {
        data |= unsafe { *dp.add(0) } as usize;
    }

    v3 ^= data;
    for _ in 0..2 {
        sipround!(v0, v1, v2, v3);
    }
    v0 ^= data;
    v2 ^= 0xff;
    for _ in 0..4 {
        sipround!(v0, v1, v2, v3);
    }
    v0 ^ v1 ^ v2 ^ v3
}

#[unsafe(no_mangle)]
pub extern "C" fn stbds_hash_bytes(p: *mut std::ffi::c_void, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p as *const u8, len, seed)
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
