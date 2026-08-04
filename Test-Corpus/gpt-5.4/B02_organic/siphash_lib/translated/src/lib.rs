use std::ffi::c_void;
use std::os::raw::c_int;

#[cfg(target_pointer_width = "64")]
type CSizeT = u64;
#[cfg(target_pointer_width = "32")]
type CSizeT = u32;

#[inline]
fn rotl(x: CSizeT, b: u32) -> CSizeT {
    x.rotate_left(b)
}

#[inline]
fn sipround(v0: &mut CSizeT, v1: &mut CSizeT, v2: &mut CSizeT, v3: &mut CSizeT) {
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotl(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl(*v0, (CSizeT::BITS / 2) as u32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotl(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotl(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl(*v2, (CSizeT::BITS / 2) as u32);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotl(*v3, 21);
    *v3 ^= *v0;
}

fn stbds_siphash_bytes_impl(bytes: &[u8], seed: CSizeT) -> CSizeT {
    let mut i = 0usize;
    let mut v0: CSizeT = (((0x736f6d65 as CSizeT) << 16) << 16).wrapping_add(0x70736575 as CSizeT) ^ seed;
    let mut v1: CSizeT = (((0x646f7261 as CSizeT) << 16) << 16).wrapping_add(0x6e646f6d as CSizeT) ^ !seed;
    let mut v2: CSizeT = (((0x6c796765 as CSizeT) << 16) << 16).wrapping_add(0x6e657261 as CSizeT) ^ seed;
    let mut v3: CSizeT = (((0x74656462 as CSizeT) << 16) << 16).wrapping_add(0x79746573 as CSizeT) ^ !seed;

    v0 ^= 0x0706050403020100u64 as CSizeT ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908u64 as CSizeT ^ !seed;
    v2 ^= 0x0706050403020100u64 as CSizeT ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908u64 as CSizeT ^ !seed;

    while i + std::mem::size_of::<CSizeT>() <= bytes.len() {
        let d = &bytes[i..];
        let mut data: CSizeT = (d[0] as CSizeT)
            | ((d[1] as CSizeT) << 8)
            | ((d[2] as CSizeT) << 16)
            | ((d[3] as CSizeT) << 24);
        data |= (((d[4] as CSizeT)
            | ((d[5] as CSizeT) << 8)
            | ((d[6] as CSizeT) << 16)
            | ((d[7] as CSizeT) << 24))
            << 16)
            << 16;

        v3 ^= data;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;
        i += std::mem::size_of::<CSizeT>();
    }

    let rem = &bytes[i..];
    let mut data = (bytes.len() as CSizeT) << (CSizeT::BITS - 8);
    match rem.len() {
        7 => {
            data |= ((rem[6] as CSizeT) << 24) << 24;
            data |= ((rem[5] as CSizeT) << 20) << 20;
            data |= ((rem[4] as CSizeT) << 16) << 16;
            data |= (rem[3] as CSizeT) << 24;
            data |= (rem[2] as CSizeT) << 16;
            data |= (rem[1] as CSizeT) << 8;
            data |= rem[0] as CSizeT;
        }
        6 => {
            data |= ((rem[5] as CSizeT) << 20) << 20;
            data |= ((rem[4] as CSizeT) << 16) << 16;
            data |= (rem[3] as CSizeT) << 24;
            data |= (rem[2] as CSizeT) << 16;
            data |= (rem[1] as CSizeT) << 8;
            data |= rem[0] as CSizeT;
        }
        5 => {
            data |= ((rem[4] as CSizeT) << 16) << 16;
            data |= (rem[3] as CSizeT) << 24;
            data |= (rem[2] as CSizeT) << 16;
            data |= (rem[1] as CSizeT) << 8;
            data |= rem[0] as CSizeT;
        }
        4 => {
            data |= (rem[3] as CSizeT) << 24;
            data |= (rem[2] as CSizeT) << 16;
            data |= (rem[1] as CSizeT) << 8;
            data |= rem[0] as CSizeT;
        }
        3 => {
            data |= (rem[2] as CSizeT) << 16;
            data |= (rem[1] as CSizeT) << 8;
            data |= rem[0] as CSizeT;
        }
        2 => {
            data |= (rem[1] as CSizeT) << 8;
            data |= rem[0] as CSizeT;
        }
        1 => {
            data |= rem[0] as CSizeT;
        }
        _ => {}
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
pub extern "C" fn stbds_hash_bytes(p: *mut c_void, len: usize, seed: usize) -> usize {
    let bytes = if len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(p as *const u8, len) }
    };
    stbds_siphash_bytes_impl(bytes, seed as CSizeT) as usize
}

#[unsafe(no_mangle)]
pub extern "C" fn siphash(init: c_int) {
    let mut mem = [0u8; 64];
    let mut z = init;
    for b in &mut mem {
        *b = z as u8;
        z = z.wrapping_add(1);
    }
    for i in 0..64usize {
        let hash = stbds_siphash_bytes_impl(&mem[..i], 0);
        print!("  {{ ");
        for j in 0..8u32 {
            let byte = ((hash >> (j * 8)) & 255) as u8;
            print!("0x{:02x}, ", byte);
        }
        println!(" }},");
    }
}
