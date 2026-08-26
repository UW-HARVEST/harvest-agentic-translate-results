use std::ffi::c_void;
use std::os::raw::c_int;

// Match C's `size_t` on the target platform.
#[allow(non_camel_case_types)]
type size_t = usize;

#[inline]
fn rotl(x: usize, k: u32) -> usize {
    let bits = (core::mem::size_of::<usize>() * 8) as u32;
    (x << k) | (x >> (bits - k))
}

#[inline]
fn sipround(v0: &mut usize, v1: &mut usize, v2: &mut usize, v3: &mut usize) {
    let bits = (core::mem::size_of::<usize>() * 8) as u32;
    *v0 = v0.wrapping_add(*v1);
    *v1 = rotl(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl(*v0, bits / 2);
    *v2 = v2.wrapping_add(*v3);
    *v3 = rotl(*v3, 16);
    *v3 ^= *v2;
    *v2 = v2.wrapping_add(*v1);
    *v1 = rotl(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl(*v2, bits / 2);
    *v0 = v0.wrapping_add(*v3);
    *v3 = rotl(*v3, 21);
    *v3 ^= *v0;
}

fn stbds_siphash_bytes_safe(d: &[u8], len: usize, seed: usize) -> usize {
    // The original C code interprets `len` as the slice length and reads up to `len` bytes from `d`.
    // We mirror that exactly.
    let mut v0: usize;
    let mut v1: usize;
    let mut v2: usize;
    let mut v3: usize;

    // (((size_t)0x736f6d65 << 16) << 16) + 0x70736575
    v0 = (((0x736f6d65usize << 16) << 16).wrapping_add(0x70736575)) ^ seed;
    v1 = (((0x646f7261usize << 16) << 16).wrapping_add(0x6e646f6d)) ^ !seed;
    v2 = (((0x6c796765usize << 16) << 16).wrapping_add(0x6e657261)) ^ seed;
    v3 = (((0x74656462usize << 16) << 16).wrapping_add(0x79746573)) ^ !seed;

    v0 ^= 0x0706050403020100usize ^ seed;
    v1 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;
    v2 ^= 0x0706050403020100usize ^ seed;
    v3 ^= 0x0f0e0d0c0b0a0908usize ^ !seed;

    let size_of_size_t = core::mem::size_of::<usize>();

    let mut i: usize = 0;
    let mut off: usize = 0;
    let mut data: usize;

    // Main loop: process chunks of size_of(size_t) bytes.
    while i + size_of_size_t <= len {
        // data = d[0] | (d[1] << 8) | (d[2] << 16) | (d[3] << 24);
        // In C, the shift expression is `int`. If d[3] >= 0x80, the result is a
        // negative int and gets sign-extended when assigned to size_t. We
        // reproduce that behavior exactly.
        let lo: i32 = (d[off] as i32)
            | ((d[off + 1] as i32) << 8)
            | ((d[off + 2] as i32) << 16)
            | ((d[off + 3] as i32) << 24);
        data = lo as usize; // sign-extends to 64-bit when high bit set
        // data |= (size_t)(d[4] | (d[5] << 8) | (d[6] << 16) | (d[7] << 24)) << 16 << 16;
        let hi: i32 = (d[off + 4] as i32)
            | ((d[off + 5] as i32) << 8)
            | ((d[off + 6] as i32) << 16)
            | ((d[off + 7] as i32) << 24);
        data |= ((hi as usize) << 16) << 16;

        v3 ^= data;
        for _ in 0..2 {
            sipround(&mut v0, &mut v1, &mut v2, &mut v3);
        }
        v0 ^= data;

        i += size_of_size_t;
        off += size_of_size_t;
    }

    // data = len << (((sizeof(size_t)) * 8) - 8);
    let total_bits = (size_of_size_t * 8) as u32;
    data = len << (total_bits - 8);

    let remaining = len - i;
    // C switch with fall-through.
    if remaining >= 7 {
        data |= ((d[off + 6] as usize) << 24) << 24;
    }
    if remaining >= 6 {
        data |= ((d[off + 5] as usize) << 20) << 20;
    }
    if remaining >= 5 {
        data |= ((d[off + 4] as usize) << 16) << 16;
    }
    if remaining >= 4 {
        // BUG-preserving: in C, `d[3] << 24` is computed as int. If d[3] >= 0x80,
        // it becomes a negative int and is sign-extended into size_t.
        data |= ((d[off + 3] as i32) << 24) as usize;
    }
    if remaining >= 3 {
        data |= ((d[off + 2] as i32) << 16) as usize;
    }
    if remaining >= 2 {
        data |= ((d[off + 1] as i32) << 8) as usize;
    }
    if remaining >= 1 {
        data |= d[off] as usize;
    }
    // case 0: break;

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
pub unsafe extern "C" fn stbds_hash_bytes(p: *mut c_void, len: size_t, seed: size_t) -> size_t {
    if len == 0 {
        // Safe to pass an empty slice.
        return stbds_siphash_bytes_safe(&[], 0, seed);
    }
    let bytes = unsafe { core::slice::from_raw_parts(p as *const u8, len) };
    stbds_siphash_bytes_safe(bytes, len, seed)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn siphash(init: c_int) {
    use std::io::Write;

    let mut mem: [u8; 64] = [0u8; 64];
    let mut z: c_int = init;
    for i in 0..64usize {
        // mem[i] = z; (z is int — assigning to unsigned char truncates to low 8 bits)
        mem[i] = (z as u32 & 0xff) as u8;
        z = z.wrapping_add(1);
    }

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for i in 0..64usize {
        let hash = stbds_siphash_bytes_safe(&mem, i, 0);
        // printf("  { ");
        let _ = out.write_all(b"  { ");
        for j in 0..8u32 {
            // printf("0x%02x, ", (unsigned char) ((hash >> (j*8)) & 255));
            let byte = ((hash >> (j * 8)) & 0xff) as u8;
            let _ = write!(out, "0x{:02x}, ", byte);
        }
        // printf(" },\n");
        let _ = out.write_all(b" },\n");
    }
    let _ = out.flush();
}
