use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = std::ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = [
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
];

#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [16, 17, 18, 0, 8,  7, 9,  6, 10, 5,
                                    11, 4,  12, 3, 13, 2, 14, 1, 15];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1,
                                     1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4,
                                     4, 4, 5, 5, 5, 5, 0, 0, 0];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3,  4,  5,  6,  7,  8,  9,  10,  11,  13,  15,  17,  19,  23, 27, 31,
    35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 0,  0];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [0,  0,  0,  0,  1,  1,  2,  2,  3, 3, 4,
                                      4,  5,  5,  6,  6,  7,  7,  8,  8, 9, 9,
                                      10, 10, 11, 11, 12, 12, 13, 13, 0, 0];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1,    2,    3,    4,    5,    7,     9,     13,    17,  25,   33,
    49,   65,   97,   129,  193,  257,   385,   513,   769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0,   0];

#[repr(C)]
pub struct cp_state_t {
    pub bits: u64,
    pub count: c_int,
    pub words: *mut u32,
    pub word_count: c_int,
    pub word_index: c_int,
    pub bits_left: c_int,
    pub final_word_available: c_int,
    pub final_word: u32,
    pub out: *mut u8,
    pub out_end: *mut u8,
    pub begin: *mut u8,
    pub lookup: [u16; 1 << 9],
    pub lit: [u32; 288],
    pub dst: [u32; 32],
    pub len: [u32; 19],
    pub nlit: u32,
    pub ndst: u32,
    pub nlen: u32,
}

unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    if ((*s).bits_left + (*s).count) - num_bits < 0 { 1 } else { 0 }
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut u8 {
    assert!(((*s).bits_left & 7) == 0);
    ((*s).words.add((*s).word_index as usize) as *mut u8).sub(((*s).count / 8) as usize)
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.add((*s).word_index as usize);
            (*s).word_index += 1;
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count += 32;
            assert!((*s).word_index <= (*s).word_count);
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count += (*s).bits_left;
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    assert!((*s).count >= num_bits_to_read);
    let bits = ((*s).bits & ((1u64 << num_bits_to_read) - 1)) as u32;
    (*s).bits >>= num_bits_to_read;
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!((*s).bits_left > 0);
    assert!((*s).count <= 64);
    assert!(cp_would_overflow(s, num_bits_to_read) == 0);
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

unsafe fn cp_build(s: *mut cp_state_t, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    for n in 0..sym_count {
        counts[*lens.add(n as usize) as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if !s.is_null() {
        std::ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, (*s).lookup.len());
    }
    for i in 0..sym_count {
        let len = *lens.add(i as usize) as i32;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len as usize];
            codes[len as usize] += 1;
            let slot = first[len as usize];
            first[len as usize] += 1;
            *tree.add(slot as usize) = ((code as u32) << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code as u32) >> (16 - len)) as i32;
                while j < (1 << 9) {
                    (*s).lookup[j as usize] = ((len << 9) | i) as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    cp_read_bits(s, (*s).count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != (!nlen) {
        cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    if ((*s).bits_left / 8) <= len as i32 {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    let p = cp_ptr(s);
    std::ptr::copy_nonoverlapping(p, (*s).out, len as usize);
    (*s).out = (*s).out.add(len as usize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), cp_fixed_table.as_ptr(), 288) as u32;
    (*s).ndst = cp_build(std::ptr::null_mut(), (*s).dst.as_mut_ptr(), cp_fixed_table.as_ptr().add(288), 32) as u32;
    1
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, mut hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.add(guess as usize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.add((lo - 1) as usize);
    let len = 32 - (key & 0xF);
    assert!((search >> len) == (key >> len));
    let code = cp_consume_bits(s, (key & 0xF) as i32);
    let _ = code;
    ((key >> 4) & 0xFFF) as i32
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen {
        lenlens[cp_permutation_order[i as usize] as usize] = cp_read_bits(s, 3) as u8;
    }
    (*s).nlen = cp_build(std::ptr::null_mut(), (*s).len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;
    let mut lens = [0u8; 288 + 32];
    let mut n = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, (*s).len.as_mut_ptr(), (*s).nlen as i32);
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as i32;
                while i > 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as i32;
                while i > 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as i32;
                while i > 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            _ => {
                lens[n as usize] = sym as u8;
                n += 1;
            }
        }
    }
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
    (*s).ndst = cp_build(std::ptr::null_mut(), (*s).dst.as_mut_ptr(), lens.as_ptr().add(nlit as usize), ndst) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    loop {
        let mut symbol = cp_decode(s, (*s).lit.as_mut_ptr(), (*s).nlit as i32);
        if symbol < 256 {
            if (*s).out.add(1) > (*s).out_end {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                return 0;
            }
            *(*s).out = symbol as u8;
            (*s).out = (*s).out.add(1);
        } else if symbol > 256 {
            symbol -= 257;
            let mut length = cp_read_bits(s, cp_len_extra_bits[symbol as usize] as i32) as i32 + cp_len_base[symbol as usize] as i32;
            let distance_symbol = cp_decode(s, (*s).dst.as_mut_ptr(), (*s).ndst as i32);
            let backwards_distance = cp_read_bits(s, cp_dist_extra_bits[distance_symbol as usize] as i32) as i32 + cp_dist_base[distance_symbol as usize] as i32;
            if (*s).out.sub(backwards_distance as usize) < (*s).begin {
                cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                return 0;
            }
            if (*s).out.add(length as usize) > (*s).out_end {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                return 0;
            }
            let mut src = (*s).out.sub(backwards_distance as usize);
            let mut dst = (*s).out;
            (*s).out = (*s).out.add(length as usize);
            match backwards_distance {
                1 => {
                    std::ptr::write_bytes(dst, *src, length as usize);
                }
                _ => {
                    while length > 0 {
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
                        length -= 1;
                    }
                }
            }
        } else {
            break;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn pinflate(in_: *mut c_void, in_bytes: c_int, out: *mut c_void, out_bytes: c_int) -> c_int {
    unsafe {
        let s = std::alloc::alloc_zeroed(std::alloc::Layout::new::<cp_state_t>()) as *mut cp_state_t;
        if s.is_null() {
            return 0;
        }
        (*s).bits = 0;
        (*s).count = 0;
        (*s).word_index = 0;
        (*s).bits_left = in_bytes * 8;
        let first_bytes = (((in_ as usize + 3) & !3) - in_ as usize) as i32;
        (*s).words = (in_ as *mut u8).add(first_bytes as usize) as *mut u32;
        (*s).word_count = (in_bytes - first_bytes) / 4;
        let last_bytes = (in_bytes - first_bytes) & 3;
        for i in 0..first_bytes {
            (*s).bits |= (*(in_ as *mut u8).add(i as usize) as u64) << (i * 8);
        }
        (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
        (*s).final_word = 0;
        for i in 0..last_bytes {
            (*s).final_word |= (*(in_ as *mut u8).add((in_bytes - last_bytes + i) as usize) as u32) << (i * 8);
        }
        (*s).count = first_bytes * 8;
        (*s).out = out as *mut u8;
        (*s).out_end = (*s).out.add(out_bytes as usize);
        (*s).begin = out as *mut u8;
        let mut count = 0;
        let mut bfinal;
        loop {
            bfinal = cp_read_bits(s, 1);
            let btype = cp_read_bits(s, 2);
            match btype {
                0 => {
                    if cp_stored(s) == 0 {
                        std::alloc::dealloc(s as *mut u8, std::alloc::Layout::new::<cp_state_t>());
                        return 0;
                    }
                }
                1 => {
                    cp_fixed(s);
                    if cp_block(s) == 0 {
                        std::alloc::dealloc(s as *mut u8, std::alloc::Layout::new::<cp_state_t>());
                        return 0;
                    }
                }
                2 => {
                    cp_dynamic(s);
                    if cp_block(s) == 0 {
                        std::alloc::dealloc(s as *mut u8, std::alloc::Layout::new::<cp_state_t>());
                        return 0;
                    }
                }
                3 => {
                    cp_error_reason = b"Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
                    std::alloc::dealloc(s as *mut u8, std::alloc::Layout::new::<cp_state_t>());
                    return 0;
                }
                _ => {}
            }
            count += 1;
            if bfinal != 0 {
                break;
            }
        }
        std::alloc::dealloc(s as *mut u8, std::alloc::Layout::new::<cp_state_t>());
        1
    }
}
