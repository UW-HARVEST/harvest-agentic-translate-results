use std::ffi::c_int;

// Reproduce C integer promotion: unsigned char values shifted as i32, then cast to u64 (sign-extending)
#[inline(always)]
fn c_int_expr(v: i32) -> u64 {
    v as u64 // sign-extends, matching C's implicit int->size_t conversion
}

#[inline(always)]
fn rotate(v: u64, n: u32) -> u64 {
    (v << n) | (v >> (64 - n))
}

macro_rules! sipround {
    ($v0:expr, $v1:expr, $v2:expr, $v3:expr) => {
        $v0 = $v0.wrapping_add($v1);
        $v1 = rotate($v1, 13);
        $v1 ^= $v0;
        $v0 = rotate($v0, 32);
        $v2 = $v2.wrapping_add($v3);
        $v3 = rotate($v3, 16);
        $v3 ^= $v2;
        $v2 = $v2.wrapping_add($v1);
        $v1 = rotate($v1, 17);
        $v1 ^= $v2;
        $v2 = rotate($v2, 32);
        $v0 = $v0.wrapping_add($v3);
        $v3 = rotate($v3, 21);
        $v3 ^= $v0;
    };
}

fn stbds_siphash_bytes(p: *const u8, len: usize, seed: u64) -> u64 {
    let mut d = p;
    let mut v0 = (((0x736f6d65_u64) << 16 << 16).wrapping_add(0x70736575)) ^ seed;
    let mut v1 = (((0x646f7261_u64) << 16 << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    let mut v2 = (((0x6c796765_u64) << 16 << 16).wrapping_add(0x6e657261)) ^ seed;
    let mut v3 = (((0x74656462_u64) << 16 << 16).wrapping_add(0x79746573)) ^ !seed;
    v0 ^= 0x0706050403020100_u64 ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908_u64 ^ !seed;
    v2 ^= 0x0706050403020100_u64 ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908_u64 ^ !seed;

    let len_u64 = len as u64;
    let mut i: u64 = 0;
    while i + 8 <= len_u64 {
        let bytes = unsafe { std::slice::from_raw_parts(d, 8) };
        // Reproduce C: lower 32 bits via int promotion (d[0]|(d[1]<<8)|(d[2]<<16)|(d[3]<<24)) as int -> size_t
        let lo = c_int_expr(
            (bytes[0] as i32)
                | ((bytes[1] as i32) << 8)
                | ((bytes[2] as i32) << 16)
                | ((bytes[3] as i32) << 24),
        );
        let hi = (c_int_expr(
            (bytes[4] as i32)
                | ((bytes[5] as i32) << 8)
                | ((bytes[6] as i32) << 16)
                | ((bytes[7] as i32) << 24),
        )) << 16
            << 16;
        let data = lo | hi;

        v3 ^= data;
        for _ in 0..2 {
            sipround!(v0, v1, v2, v3);
        }
        v0 ^= data;
        i += 8;
        d = unsafe { d.add(8) };
    }

    let remain = len_u64.wrapping_sub(i);
    let mut data: u64 = len_u64 << 56;
    let rem = unsafe { std::slice::from_raw_parts(d, remain as usize) };

    // C fallthrough switch
    if remain >= 7 {
        data |= (rem[6] as u64) << 24 << 24;
    }
    if remain >= 6 {
        data |= (rem[5] as u64) << 20 << 20;
    }
    if remain >= 5 {
        data |= (rem[4] as u64) << 16 << 16;
    }
    if remain >= 4 {
        data |= c_int_expr((rem[3] as i32) << 24);
    }
    if remain >= 3 {
        data |= c_int_expr((rem[2] as i32) << 16);
    }
    if remain >= 2 {
        data |= c_int_expr((rem[1] as i32) << 8);
    }
    if remain >= 1 {
        data |= rem[0] as u64;
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
pub extern "C" fn stbds_hash_bytes(p: *mut u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed as u64) as usize
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
