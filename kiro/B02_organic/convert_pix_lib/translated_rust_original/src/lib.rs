#![allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    clippy::missing_safety_doc
)]

use std::ffi::c_char;
use std::ptr;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
struct cp_image_t {
    w: i32,
    h: i32,
    pix: *mut cp_pixel_t,
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 320] = [
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
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67,
    83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

#[repr(C)]
struct cp_state_t {
    bits: u64,
    count: i32,
    words: *mut u32,
    word_count: i32,
    word_index: i32,
    bits_left: i32,
    final_word_available: i32,
    final_word: u32,
    out: *mut c_char,
    out_end: *mut c_char,
    begin: *mut c_char,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: i32) -> bool {
    ((*s).bits_left + (*s).count) - num_bits < 0
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    ((*s).words.add((*s).word_index as usize) as *mut c_char).offset(-((*s).count / 8) as isize)
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: i32) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.add((*s).word_index as usize);
            (*s).word_index += 1;
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count += 32;
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count += (*s).bits_left;
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: i32) -> u32 {
    let bits = (*s).bits & (((1u64) << num_bits_to_read) - 1);
    (*s).bits >>= num_bits_to_read;
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits as u32
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: i32) -> u32 {
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(a: u32) -> u32 {
    let a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    let a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    let a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8)
}

unsafe fn cp_build(
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: i32,
) -> i32 {
    let mut counts = [0i32; 16];
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];

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
        ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, (*s).lookup.len());
    }

    for i in 0..sym_count {
        let len = *lens.add(i as usize) as i32;
        if len != 0 {
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as u32;
            first[len as usize] += 1;
            *tree.add(slot as usize) =
                (code << (32 - len)) | ((i as u32) << 4) | (len as u32);

            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                while j < (1 << 9) {
                    (*s).lookup[j as usize] = ((len << 9) | i) as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: *mut cp_state_t) -> i32 {
    cp_read_bits(s, (*s).count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    if !((*s).bits_left / 8 <= len as i32) {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p, (*s).out, len as usize);
    (*s).out = (*s).out.add(len as usize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> i32 {
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), cp_fixed_table.as_ptr(), 288) as u32;
    (*s).ndst = cp_build(ptr::null_mut(), (*s).dst.as_mut_ptr(), cp_fixed_table.as_ptr().add(288), 32) as u32;
    1
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, hi: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0i32;
    let mut hi = hi;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.add(guess as usize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.add((lo - 1) as usize);
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> i32 {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen {
        lenlens[cp_permutation_order[i as usize] as usize] = cp_read_bits(s, 3) as u8;
    }
    (*s).nlen = cp_build(ptr::null_mut(), (*s).len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;

    let mut lens = [0u8; 320];
    let mut n = 0i32;
    while n < nlit + ndst {
        let sym = cp_decode(s, (*s).len.as_mut_ptr(), (*s).nlen as i32);
        match sym {
            16 => {
                let count = 3 + cp_read_bits(s, 2) as i32;
                for _ in 0..count {
                    lens[n as usize] = lens[(n - 1) as usize];
                    n += 1;
                }
            }
            17 => {
                let count = 3 + cp_read_bits(s, 3) as i32;
                for _ in 0..count {
                    lens[n as usize] = 0;
                    n += 1;
                }
            }
            18 => {
                let count = 11 + cp_read_bits(s, 7) as i32;
                for _ in 0..count {
                    lens[n as usize] = 0;
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
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        lens.as_ptr().add(nlit as usize),
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> i32 {
    loop {
        let symbol = cp_decode(s, (*s).lit.as_mut_ptr(), (*s).nlit as i32);
        if symbol < 256 {
            if !((*s).out.add(1) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.add(1);
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = cp_read_bits(s, cp_len_extra_bits[symbol as usize] as i32) as i32
                + cp_len_base[symbol as usize] as i32;
            let distance_symbol = cp_decode(s, (*s).dst.as_mut_ptr(), (*s).ndst as i32);
            let backwards_distance =
                cp_read_bits(s, cp_dist_extra_bits[distance_symbol as usize] as i32) as i32
                    + cp_dist_base[distance_symbol as usize] as i32;
            if !((*s).out.offset(-(backwards_distance as isize)) >= (*s).begin) {
                cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                return 0;
            }
            if !((*s).out.add(length as usize) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                return 0;
            }
            let src_ptr = (*s).out.offset(-(backwards_distance as isize));
            let dst_ptr = (*s).out;
            (*s).out = (*s).out.add(length as usize);
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst_ptr, *src_ptr as u8, length as usize);
                }
                _ => {
                    let mut d = dst_ptr;
                    let mut sr = src_ptr;
                    let mut rem = length;
                    while rem > 0 {
                        *d = *sr;
                        d = d.add(1);
                        sr = sr.add(1);
                        rem -= 1;
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
pub unsafe extern "C" fn cp_inflate(
    in_: *mut u8,
    in_bytes: i32,
    out: *mut u8,
    out_bytes: i32,
) -> i32 {
    let layout = std::alloc::Layout::new::<cp_state_t>();
    let s = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;
    if s.is_null() {
        return 0;
    }

    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes * 8;

    let in_ptr = in_ as *mut c_char;
    let first_bytes = ((((in_ as usize) + 3) & !3) - (in_ as usize)) as i32;
    (*s).words = in_ptr.add(first_bytes as usize) as *mut u32;
    (*s).word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;

    for i in 0..first_bytes {
        (*s).bits |= (*in_.add(i as usize) as u64) << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        (*s).final_word |=
            (*in_.add((in_bytes - last_bytes + i) as usize) as u32) << (i * 8);
    }
    (*s).count = first_bytes * 8;

    (*s).out = out as *mut c_char;
    (*s).out_end = (out as *mut c_char).add(out_bytes as usize);
    (*s).begin = out as *mut c_char;

    loop {
        let bfinal = cp_read_bits(s, 1);
        let btype = cp_read_bits(s, 2);
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    std::alloc::dealloc(s as *mut u8, layout);
                    return 0;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    std::alloc::dealloc(s as *mut u8, layout);
                    return 0;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    std::alloc::dealloc(s as *mut u8, layout);
                    return 0;
                }
            }
            3 => {
                cp_error_reason = b"Detected unknown block type within input stream.\0".as_ptr()
                    as *const c_char;
                std::alloc::dealloc(s as *mut u8, layout);
                return 0;
            }
            _ => {}
        }
        if bfinal != 0 {
            break;
        }
    }
    std::alloc::dealloc(s as *mut u8, layout);
    1
}

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[repr(C)]
struct cp_raw_png_t {
    p: *const u8,
    end: *const u8,
}

unsafe fn cp_make32(s: *const u8) -> u32 {
    ((*s.add(0) as u32) << 24)
        | ((*s.add(1) as u32) << 16)
        | ((*s.add(2) as u32) << 8)
        | (*s.add(3) as u32)
}

unsafe fn cp_chunk(png: *mut cp_raw_png_t, chunk: *const u8, minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if libc_memcmp(start.add(4), chunk, 4) == 0 && len >= minlen {
        let offset = len as isize + 12;
        if (*png).p.offset(offset) <= (*png).end {
            (*png).p = (*png).p.offset(offset);
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: *mut cp_raw_png_t, chunk: *const u8, minlen: u32) -> *const u8 {
    while (*png).p < (*png).end {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        (*png).p = (*png).p.add(len as usize + 12);
        if libc_memcmp(start.add(4), chunk, 4) == 0 && len >= minlen && (*png).p <= (*png).end {
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn libc_memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let diff = *a.add(i) as i32 - *b.add(i) as i32;
        if diff != 0 {
            return diff;
        }
    }
    0
}

unsafe fn cp_unfilter(w: i32, h: i32, bpp: i32, mut raw: *mut u8) -> i32 {
    let len = w * bpp;
    let mut x: i32;

    if h > 0 {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize) / 2);
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    *raw.add(x as usize) = (*raw.add(x as usize))
                        .wrapping_add(cp_paeth(*raw.add((x - bpp) as usize), 0, 0));
                    x += 1;
                }
            }
            _ => return 0,
        }
    }

    let mut prev = raw;
    raw = raw.add(len as usize);

    for _y in 1..h {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(0);
                    x += 1;
                }
                while x < len {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                    x += 1;
                }
                while x < len {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize) / 2);
                    x += 1;
                }
                while x < len {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(
                        ((*raw.add((x - bpp) as usize) as u32 + *prev.add(x as usize) as u32) / 2)
                            as u8,
                    );
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                    x += 1;
                }
                while x < len {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(cp_paeth(
                        *raw.add((x - bpp) as usize),
                        *prev.add(x as usize),
                        *prev.add((x - bpp) as usize),
                    ));
                    x += 1;
                }
            }
            _ => return 0,
        }
        prev = raw;
        raw = raw.add(len as usize);
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn convert_pix(
    bpp: i32,
    w: i32,
    h: i32,
    mut src: *mut u8,
    mut dst: *mut cp_pixel_t,
) {
    for _y in 0..h {
        src = src.add(1);
        for _x in 0..w {
            match bpp {
                1 => {
                    *dst = cp_make_pixel(*src.add(0), *src.add(0), *src.add(0));
                }
                2 => {
                    *dst = cp_make_pixel_a(*src.add(0), *src.add(0), *src.add(0), *src.add(1));
                }
                3 => {
                    *dst = cp_make_pixel(*src.add(0), *src.add(1), *src.add(2));
                }
                4 => {
                    *dst = cp_make_pixel_a(*src.add(0), *src.add(1), *src.add(2), *src.add(3));
                }
                _ => {}
            }
            src = src.add(bpp as usize);
            dst = dst.add(1);
        }
    }
}
