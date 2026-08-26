use std::io::Write;

#[inline(always)]
fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    let half = (std::mem::size_of::<usize>() * 8) / 2;
    *v0 = v0.wrapping_add(*v1);
    *v1 = v1.rotate_left(13);
    *v1 ^= *v0;
    *v0 = v0.rotate_left(half as u32);
    *v2 = v2.wrapping_add(*v3);
    *v3 = v3.rotate_left(16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = v1.rotate_left(17);
    *v1 ^= *v2;
    *v2 = v2.rotate_left(half as u32);
    *v0 = v0.wrapping_add(*v3);
    *v3 = v3.rotate_left(21);
    *v3 ^= *v0;
}

fn stbds_siphash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    let mut d = p;
    let usize_size = std::mem::size_of::<usize>();

    // Build 64-bit-style constants the same way the C code does, so that
    // the resulting usize value is correct on both 32-bit and 64-bit targets.
    let c0: usize = (((0x736f6d65usize) << 16) << 16).wrapping_add(0x70736575);
    let c1: usize = (((0x646f7261usize) << 16) << 16).wrapping_add(0x6e646f6d);
    let c2: usize = (((0x6c796765usize) << 16) << 16).wrapping_add(0x6e657261);
    let c3: usize = (((0x74656462usize) << 16) << 16).wrapping_add(0x79746573);

    let mut v0: usize = c0 ^ seed;
    let mut v1: usize = c1 ^ !seed;
    let mut v2: usize = c2 ^ seed;
    let mut v3: usize = c3 ^ !seed;

    // 0x0706050403020100ull and 0x0f0e0d0c0b0a0908ull as usize.
    // On 32-bit, the upper 32 bits are truncated; this matches C semantics.
    let k0: usize = 0x0706050403020100u64 as usize;
    let k1: usize = 0x0f0e0d0c0b0a0908u64 as usize;

    v0 ^= k0 ^ seed;
    v1 ^= k1 ^ !seed;
    v2 ^= k0 ^ seed;
    v3 ^= k1 ^ !seed;

    let mut data: usize;
    let mut i: usize = 0;

    while i + usize_size <= len {
        unsafe {
            // Lower 32 bits
            let b0 = *d.add(0) as usize;
            let b1 = (*d.add(1) as i32) << 8;
            let b2 = (*d.add(2) as i32) << 16;
            let b3 = (*d.add(3) as i32) << 24;
            data = b0 | (b1 as u32 as usize) | (b2 as u32 as usize) | (b3 as u32 as usize);

            // Upper 32 bits (only meaningful when usize is 64-bit)
            let e0 = *d.add(4) as i32;
            let e1 = (*d.add(5) as i32) << 8;
            let e2 = (*d.add(6) as i32) << 16;
            let e3 = (*d.add(7) as i32) << 24;
            let upper32 = (e0 | e1 | e2 | e3) as u32 as usize;
            data |= (upper32 << 16) << 16;
        }

        v3 ^= data;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += usize_size;
        unsafe {
            d = d.add(usize_size);
        }
    }

    // Final block
    let bits = usize_size * 8;
    data = len << (bits - 8);

    let remaining = len - i;
    unsafe {
        // Switch fallthrough from the matching case downward.
        if remaining >= 7 {
            data |= ((*d.add(6) as usize) << 24) << 24;
        }
        if remaining >= 6 {
            data |= ((*d.add(5) as usize) << 20) << 20;
        }
        if remaining >= 5 {
            data |= ((*d.add(4) as usize) << 16) << 16;
        }
        if remaining >= 4 {
            // d[3] is unsigned char promoted to int; (int << 24) cast to usize
            let v = ((*d.add(3) as i32) << 24) as u32 as usize;
            data |= v;
        }
        if remaining >= 3 {
            let v = ((*d.add(2) as i32) << 16) as u32 as usize;
            data |= v;
        }
        if remaining >= 2 {
            let v = ((*d.add(1) as i32) << 8) as u32 as usize;
            data |= v;
        }
        if remaining >= 1 {
            data |= *d.add(0) as usize;
        }
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

pub fn stbds_hash_bytes(p: *const u8, len: usize, seed: usize) -> usize {
    stbds_siphash_bytes(p, len, seed)
}

#[no_mangle]
pub extern "C" fn siphash(init: i32) {
    let mut mem: [u8; 64] = [0; 64];
    let mut z: i32 = init;
    for i in 0..64 {
        mem[i] = z as u8;
        z = z.wrapping_add(1);
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for i in 0..64usize {
        let hash = stbds_hash_bytes(mem.as_ptr(), i, 0);
        write!(out, "  {{ ").unwrap();
        for j in 0..8 {
            // Match C printf "0x%02x, " for each byte of the 64-bit value.
            // On 32-bit platforms shifting by >= 32 would be UB in C; we
            // mirror the original behavior on a 64-bit target by widening.
            let byte = ((hash as u64) >> (j * 8)) & 0xff;
            write!(out, "0x{:02x}, ", byte as u8).unwrap();
        }
        writeln!(out, " }},").unwrap();
    }
}
