// Translation of c_src/src/lib.c to Rust
// Preserves byte-identical behavior with the C library.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_int};

#[repr(C)]
#[derive(Copy, Clone)]
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
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59,
    67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

#[repr(C)]
struct cp_state_t {
    bits: u64,
    count: c_int,
    words: *mut u32,
    word_count: c_int,
    word_index: c_int,
    bits_left: c_int,
    final_word_available: c_int,
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

unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    unsafe {
        if ((*s).bits_left + (*s).count) - num_bits < 0 {
            1
        } else {
            0
        }
    }
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    unsafe {
        debug_assert!(((*s).bits_left & 7) == 0);
        let p = (*s).words.add((*s).word_index as usize) as *mut c_char;
        p.offset(-((*s).count as isize / 8))
    }
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    unsafe {
        if (*s).count < num_bits_to_read {
            if (*s).word_index < (*s).word_count {
                let word = *(*s).words.add((*s).word_index as usize);
                (*s).word_index += 1;
                (*s).bits |= (word as u64) << (*s).count;
                (*s).count += 32;
                debug_assert!((*s).word_index <= (*s).word_count);
            } else if (*s).final_word_available != 0 {
                let word = (*s).final_word;
                (*s).bits |= (word as u64) << (*s).count;
                (*s).count += (*s).bits_left;
                (*s).final_word_available = 0;
            }
        }
        (*s).bits
    }
}

unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    unsafe {
        debug_assert!((*s).count >= num_bits_to_read);
        let mask = if num_bits_to_read >= 64 {
            u64::MAX
        } else {
            (1u64 << num_bits_to_read) - 1
        };
        let bits = ((*s).bits & mask) as u32;
        (*s).bits >>= num_bits_to_read;
        (*s).count -= num_bits_to_read;
        (*s).bits_left -= num_bits_to_read;
        bits
    }
}

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    unsafe {
        debug_assert!(num_bits_to_read <= 32);
        debug_assert!(num_bits_to_read >= 0);
        debug_assert!((*s).bits_left > 0);
        debug_assert!((*s).count <= 64);
        debug_assert!(cp_would_overflow(s, num_bits_to_read) == 0);
        cp_peak_bits(s, num_bits_to_read);
        cp_consume_bits(s, num_bits_to_read)
    }
}

fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

unsafe fn cp_build(
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    unsafe {
        let mut codes: [c_int; 16] = [0; 16];
        let mut first: [c_int; 16] = [0; 16];
        let mut counts: [c_int; 16] = [0; 16];
        for n in 0..sym_count {
            let l = *lens.add(n as usize) as usize;
            counts[l] += 1;
        }
        counts[0] = 0;
        codes[0] = 0;
        first[0] = 0;
        for n in 1..=15 {
            codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
            first[n] = first[n - 1] + counts[n - 1];
        }
        if !s.is_null() {
            for slot in (*s).lookup.iter_mut() {
                *slot = 0;
            }
        }
        for i in 0..sym_count {
            let len = *lens.add(i as usize) as c_int;
            if len != 0 {
                debug_assert!(len < 16);
                let code = codes[len as usize] as u32;
                codes[len as usize] += 1;
                let slot = first[len as usize] as u32;
                first[len as usize] += 1;
                *tree.add(slot as usize) =
                    (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
                if !s.is_null() && len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        (*s).lookup[j] =
                            (((len as u32) << 9) | (i as u32)) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
        first[15]
    }
}

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    unsafe {
        cp_read_bits(s, (*s).count & 7);
        let len_v = cp_read_bits(s, 16) as u16;
        let nlen = cp_read_bits(s, 16) as u16;
        if !(len_v == !nlen) {
            cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
            return 0;
        }
        if !((*s).bits_left / 8 <= len_v as c_int) {
            cp_error_reason =
                b"Stored block extends beyond end of input stream.\0".as_ptr()
                    as *const c_char;
            return 0;
        }
        let p = cp_ptr(s);
        std::ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, len_v as usize);
        (*s).out = (*s).out.add(len_v as usize);
        1
    }
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    unsafe {
        let lit_ptr = (*s).lit.as_mut_ptr();
        let dst_ptr = (*s).dst.as_mut_ptr();
        let table_ptr = (&raw const cp_fixed_table) as *const u8;
        (*s).nlit = cp_build(s, lit_ptr, table_ptr, 288) as u32;
        (*s).ndst =
            cp_build(std::ptr::null_mut(), dst_ptr, table_ptr.add(288), 32) as u32;
        1
    }
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *const u32, mut hi: c_int) -> c_int {
    unsafe {
        let bits = cp_peak_bits(s, 16);
        let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
        let mut lo: c_int = 0;
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
        debug_assert!((search >> len) == (key >> len));
        let _code = cp_consume_bits(s, (key & 0xF) as c_int);
        ((key >> 4) & 0xFFF) as c_int
    }
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    unsafe {
        let mut lenlens: [u8; 19] = [0; 19];
        let nlit = 257 + cp_read_bits(s, 5) as c_int;
        let ndst = 1 + cp_read_bits(s, 5) as c_int;
        let nlen = 4 + cp_read_bits(s, 4) as c_int;
        for i in 0..nlen {
            let order = cp_permutation_order[i as usize] as usize;
            lenlens[order] = cp_read_bits(s, 3) as u8;
        }
        let len_ptr = (*s).len.as_mut_ptr();
        (*s).nlen = cp_build(std::ptr::null_mut(), len_ptr, lenlens.as_ptr(), 19) as u32;
        let mut lens: [u8; 288 + 32] = [0; 288 + 32];
        let mut n: c_int = 0;
        while n < nlit + ndst {
            let sym = cp_decode(s, (*s).len.as_ptr(), (*s).nlen as c_int);
            match sym {
                16 => {
                    let mut i = 3 + cp_read_bits(s, 2) as c_int;
                    while i != 0 {
                        lens[n as usize] = lens[(n - 1) as usize];
                        i -= 1;
                        n += 1;
                    }
                }
                17 => {
                    let mut i = 3 + cp_read_bits(s, 3) as c_int;
                    while i != 0 {
                        lens[n as usize] = 0;
                        i -= 1;
                        n += 1;
                    }
                }
                18 => {
                    let mut i = 11 + cp_read_bits(s, 7) as c_int;
                    while i != 0 {
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
        let lit_ptr = (*s).lit.as_mut_ptr();
        let dst_ptr = (*s).dst.as_mut_ptr();
        (*s).nlit = cp_build(s, lit_ptr, lens.as_ptr(), nlit) as u32;
        (*s).ndst = cp_build(
            std::ptr::null_mut(),
            dst_ptr,
            lens.as_ptr().add(nlit as usize),
            ndst,
        ) as u32;
        1
    }
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    unsafe {
        loop {
            let mut symbol = cp_decode(s, (*s).lit.as_ptr(), (*s).nlit as c_int);
            if symbol < 256 {
                if !((*s).out.add(1) <= (*s).out_end) {
                    cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0"
                        .as_ptr() as *const c_char;
                    return 0;
                }
                *(*s).out = symbol as c_char;
                (*s).out = (*s).out.add(1);
            } else if symbol > 256 {
                symbol -= 257;
                let length = cp_read_bits(s, cp_len_extra_bits[symbol as usize] as c_int)
                    as c_int
                    + cp_len_base[symbol as usize] as c_int;
                let distance_symbol = cp_decode(s, (*s).dst.as_ptr(), (*s).ndst as c_int);
                let backwards_distance =
                    cp_read_bits(s, cp_dist_extra_bits[distance_symbol as usize] as c_int)
                        as c_int
                        + cp_dist_base[distance_symbol as usize] as c_int;
                if !((*s).out.offset(-(backwards_distance as isize)) >= (*s).begin) {
                    cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0"
                        .as_ptr() as *const c_char;
                    return 0;
                }
                if !((*s).out.add(length as usize) <= (*s).out_end) {
                    cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0"
                        .as_ptr() as *const c_char;
                    return 0;
                }
                let mut src = (*s).out.offset(-(backwards_distance as isize));
                let mut dst = (*s).out;
                (*s).out = (*s).out.add(length as usize);
                if backwards_distance == 1 {
                    std::ptr::write_bytes(dst as *mut u8, *src as u8, length as usize);
                } else {
                    let mut len_left = length;
                    while len_left != 0 {
                        len_left -= 1;
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
                    }
                }
            } else {
                break;
            }
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    in_ptr: *mut c_void,
    in_bytes: c_int,
    out_ptr: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        // calloc-equivalent: zero-initialized state
        let layout = std::alloc::Layout::new::<cp_state_t>();
        let s = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;
        if s.is_null() {
            return 0;
        }
        (*s).bits = 0;
        (*s).count = 0;
        (*s).word_index = 0;
        (*s).bits_left = in_bytes * 8;
        let in_addr = in_ptr as usize;
        let first_bytes = ((in_addr + 3) & !3usize) - in_addr;
        let first_bytes = first_bytes as c_int;
        (*s).words = (in_ptr as *mut u8).add(first_bytes as usize) as *mut u32;
        (*s).word_count = (in_bytes - first_bytes) / 4;
        let last_bytes = (in_bytes - first_bytes) & 3;
        for i in 0..first_bytes {
            (*s).bits |= (*(in_ptr as *const u8).add(i as usize) as u64) << (i * 8);
        }
        (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
        (*s).final_word = 0;
        for i in 0..last_bytes {
            (*s).final_word |= (*(in_ptr as *const u8)
                .add((in_bytes - last_bytes + i) as usize)
                as u32)
                << (i * 8);
        }
        (*s).count = first_bytes * 8;
        (*s).out = out_ptr as *mut c_char;
        (*s).out_end = (*s).out.add(out_bytes as usize);
        (*s).begin = out_ptr as *mut c_char;
        let mut _count = 0;
        let mut bfinal: c_int;
        let result;
        'outer: loop {
            bfinal = cp_read_bits(s, 1) as c_int;
            let btype = cp_read_bits(s, 2) as c_int;
            match btype {
                0 => {
                    if cp_stored(s) == 0 {
                        result = 0;
                        break 'outer;
                    }
                }
                1 => {
                    cp_fixed(s);
                    if cp_block(s) == 0 {
                        result = 0;
                        break 'outer;
                    }
                }
                2 => {
                    cp_dynamic(s);
                    if cp_block(s) == 0 {
                        result = 0;
                        break 'outer;
                    }
                }
                3 => {
                    cp_error_reason = b"Detected unknown block type within input stream.\0"
                        .as_ptr() as *const c_char;
                    result = 0;
                    break 'outer;
                }
                _ => {}
            }
            _count += 1;
            if bfinal != 0 {
                result = 1;
                break 'outer;
            }
        }
        std::alloc::dealloc(s as *mut u8, layout);
        result
    }
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
    unsafe {
        ((*s.add(0) as u32) << 24)
            | ((*s.add(1) as u32) << 16)
            | ((*s.add(2) as u32) << 8)
            | (*s.add(3) as u32)
    }
}

unsafe fn cp_chunk(
    png: *mut cp_raw_png_t,
    chunk: *const u8,
    minlen: u32,
) -> *const u8 {
    unsafe {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        let cmp = std::slice::from_raw_parts(start.add(4), 4)
            == std::slice::from_raw_parts(chunk, 4);
        if cmp && len >= minlen {
            let offset = (len + 12) as usize;
            if (*png).p.add(offset) <= (*png).end {
                (*png).p = (*png).p.add(offset);
                return start.add(8);
            }
        }
        std::ptr::null()
    }
}

unsafe fn cp_find(
    png: *mut cp_raw_png_t,
    chunk: *const u8,
    minlen: u32,
) -> *const u8 {
    unsafe {
        while (*png).p < (*png).end {
            let len = cp_make32((*png).p);
            let start = (*png).p;
            (*png).p = (*png).p.add((len + 12) as usize);
            let cmp = std::slice::from_raw_parts(start.add(4), 4)
                == std::slice::from_raw_parts(chunk, 4);
            if cmp && len >= minlen && (*png).p <= (*png).end {
                return start.add(8);
            }
        }
        std::ptr::null()
    }
}

unsafe fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw_in: *mut u8) -> c_int {
    unsafe {
        let len = w * bpp;
        let mut raw = raw_in;
        let mut prev: *mut u8;
        let mut x: c_int;
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
                        *raw.add(x as usize) = (*raw.add(x as usize))
                            .wrapping_add(*raw.add((x - bpp) as usize) / 2);
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
        prev = raw;
        raw = raw.add(len as usize);
        let mut y = 1;
        while y < h {
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
                        *raw.add(x as usize) = (*raw.add(x as usize))
                            .wrapping_add(*prev.add(x as usize) / 2);
                        x += 1;
                    }
                    while x < len {
                        let v = ((*raw.add((x - bpp) as usize) as u32
                            + *prev.add(x as usize) as u32)
                            / 2) as u8;
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(v);
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
                        let p = cp_paeth(
                            *raw.add((x - bpp) as usize),
                            *prev.add(x as usize),
                            *prev.add((x - bpp) as usize),
                        );
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(p);
                        x += 1;
                    }
                }
                _ => return 0,
            }
            y += 1;
            prev = raw;
            raw = raw.add(len as usize);
        }
        1
    }
}

unsafe fn cp_convert(bpp: c_int, w: c_int, h: c_int, src_in: *mut u8, dst_in: *mut cp_pixel_t) {
    unsafe {
        let mut src = src_in;
        let mut dst = dst_in;
        for _y in 0..h {
            src = src.add(1);
            for _x in 0..w {
                match bpp {
                    1 => {
                        *dst = cp_make_pixel(*src, *src, *src);
                        dst = dst.add(1);
                    }
                    2 => {
                        *dst = cp_make_pixel_a(*src, *src, *src, *src.add(1));
                        dst = dst.add(1);
                    }
                    3 => {
                        *dst = cp_make_pixel(*src, *src.add(1), *src.add(2));
                        dst = dst.add(1);
                    }
                    4 => {
                        *dst = cp_make_pixel_a(
                            *src,
                            *src.add(1),
                            *src.add(2),
                            *src.add(3),
                        );
                        dst = dst.add(1);
                    }
                    _ => {}
                }
                src = src.add(bpp as usize);
            }
        }
    }
}

unsafe fn cp_get_alpha_for_indexed_image(
    index: c_int,
    trns: *const u8,
    trns_len: u32,
) -> u8 {
    unsafe {
        if trns.is_null() {
            255
        } else if (index as u32) >= trns_len {
            255
        } else {
            *trns.add(index as usize)
        }
    }
}

unsafe fn cp_depalette(
    w: c_int,
    h: c_int,
    src_in: *mut u8,
    dst_in: *mut cp_pixel_t,
    plte: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    unsafe {
        let mut src = src_in;
        let mut dst = dst_in;
        for _y in 0..h {
            src = src.add(1);
            for _x in 0..w {
                let c = *src as c_int;
                let r = *plte.add((c * 3) as usize);
                let g = *plte.add((c * 3 + 1) as usize);
                let b = *plte.add((c * 3 + 2) as usize);
                let a = cp_get_alpha_for_indexed_image(c, trns, trns_len);
                *dst = cp_make_pixel_a(r, g, b, a);
                dst = dst.add(1);
                src = src.add(1);
            }
        }
    }
}

unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    unsafe { cp_make32(chunk.offset(-8)) }
}

unsafe fn cp_out_size(img: *const cp_image_t, bpp: c_int) -> c_int {
    unsafe { ((*img).w + 1) * (*img).h * bpp }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(
    png_data: *const u8,
    png_length: c_int,
) -> cp_image_t {
    unsafe {
        let sig: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
        let mut img = cp_image_t {
            w: 0,
            h: 0,
            pix: std::ptr::null_mut(),
        };
        let mut data: *mut u8 = std::ptr::null_mut();

        let mut png = cp_raw_png_t {
            p: png_data,
            end: png_data.add(png_length as usize),
        };

        // signature check
        if std::slice::from_raw_parts(png.p, 8) != &sig[..] {
            cp_error_reason =
                b"incorrect file signature (is this a png file?)\0".as_ptr()
                    as *const c_char;
            return cp_err(data, &mut img);
        }
        png.p = png.p.add(8);

        let ihdr = cp_chunk(&mut png, b"IHDR".as_ptr(), 13);
        if ihdr.is_null() {
            cp_error_reason = b"unable to find IHDR chunk\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }

        let bit_depth = *ihdr.add(8) as c_int;
        let color_type = *ihdr.add(9) as c_int;
        if bit_depth != 8 {
            cp_error_reason =
                b"only bit-depth of 8 is supported\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }

        let bpp: c_int = match color_type {
            0 => 1,
            2 => 3,
            3 => 1,
            4 => 2,
            6 => 4,
            _ => {
                cp_error_reason = b"unknown color type\0".as_ptr() as *const c_char;
                return cp_err(data, &mut img);
            }
        };

        let w = (cp_make32(ihdr) as c_int) + 1;
        let h = cp_make32(ihdr.add(4)) as c_int;
        if !(w >= 1) {
            cp_error_reason =
                b"invalid IHDR chunk found, image width was less than 1\0"
                    .as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }
        if !(h >= 1) {
            cp_error_reason =
                b"invalid IHDR chunk found, image height was less than 1\0"
                    .as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }
        let total = (w as i64) * (h as i64) * (std::mem::size_of::<cp_pixel_t>() as i64);
        if !(total < c_int::MAX as i64) {
            cp_error_reason = b"image too large\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }
        let pix_bytes = w * h * std::mem::size_of::<cp_pixel_t>() as c_int;
        img.w = w - 1;
        img.h = h;
        // malloc(pix_bytes)
        img.pix = libc_malloc(pix_bytes as usize) as *mut cp_pixel_t;
        if img.pix.is_null() {
            cp_error_reason =
                b"unable to allocate raw image space\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }

        let compression = *ihdr.add(10) as c_int;
        let filter = *ihdr.add(11) as c_int;
        let interlace = *ihdr.add(12) as c_int;
        if compression != 0 {
            cp_error_reason =
                b"only standard compression DEFLATE is supported\0".as_ptr()
                    as *const c_char;
            return cp_err(data, &mut img);
        }
        if filter != 0 {
            cp_error_reason =
                b"only standard adaptive filtering is supported\0".as_ptr()
                    as *const c_char;
            return cp_err(data, &mut img);
        }
        if interlace != 0 {
            cp_error_reason =
                b"interlacing is not supported\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }

        let mut first = png.p;
        let plte = cp_find(&mut png, b"PLTE".as_ptr(), 0);
        if plte.is_null() {
            png.p = first;
        } else {
            first = png.p;
        }
        let trns = cp_find(&mut png, b"tRNS".as_ptr(), 0);
        if trns.is_null() {
            png.p = first;
        } else {
            first = png.p;
        }
        let mut datalen: c_int = 0;
        {
            let mut idat = cp_find(&mut png, b"IDAT".as_ptr(), 0);
            while !idat.is_null() {
                let len = cp_get_chunk_byte_length(idat);
                datalen += len as c_int;
                idat = cp_chunk(&mut png, b"IDAT".as_ptr(), 0);
            }
        }
        png.p = first;
        data = libc_malloc(datalen as usize) as *mut u8;
        let mut offset: c_int = 0;
        {
            let mut idat = cp_find(&mut png, b"IDAT".as_ptr(), 0);
            while !idat.is_null() {
                let len = cp_get_chunk_byte_length(idat);
                std::ptr::copy_nonoverlapping(idat, data.add(offset as usize), len as usize);
                offset += len as c_int;
                idat = cp_chunk(&mut png, b"IDAT".as_ptr(), 0);
            }
        }
        if data.is_null() || datalen < 6 {
            cp_error_reason =
                b"corrupt zlib structure in DEFLATE stream\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }
        if (*data & 0x0f) != 0x08 {
            cp_error_reason =
                b"only zlib compression method (RFC 1950) is supported\0".as_ptr()
                    as *const c_char;
            return cp_err(data, &mut img);
        }
        if (*data & 0xf0) > 0x70 {
            cp_error_reason =
                b"innapropriate window size detected\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }
        if (*data.add(1) & 0x20) != 0 {
            cp_error_reason =
                b"preset dictionary is present and not supported\0".as_ptr()
                    as *const c_char;
            return cp_err(data, &mut img);
        }
        if !(cp_out_size(&img, 4) >= 1) {
            cp_error_reason =
                b"invalid image size found\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }
        if !(cp_out_size(&img, bpp) >= 1) {
            cp_error_reason =
                b"invalid image size found\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }
        let out = (img.pix as *mut u8)
            .add(cp_out_size(&img, 4) as usize - cp_out_size(&img, bpp) as usize);
        if cp_inflate(
            data.add(2) as *mut c_void,
            datalen - 6,
            out as *mut c_void,
            pix_bytes,
        ) == 0
        {
            cp_error_reason = b"DEFLATE algorithm failed\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }
        if cp_unfilter(img.w, img.h, bpp, out) == 0 {
            cp_error_reason =
                b"invalid filter byte found\0".as_ptr() as *const c_char;
            return cp_err(data, &mut img);
        }
        if color_type == 3 {
            if plte.is_null() {
                cp_error_reason = b"color type of indexed requires a PLTE chunk\0"
                    .as_ptr() as *const c_char;
                return cp_err(data, &mut img);
            }
            let trns_len = if !trns.is_null() {
                cp_get_chunk_byte_length(trns)
            } else {
                0
            };
            cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
        } else {
            cp_convert(bpp, img.w, img.h, out, img.pix);
        }
        libc_free(data as *mut c_void);
        img
    }
}

unsafe fn cp_err(data: *mut u8, img: &mut cp_image_t) -> cp_image_t {
    unsafe {
        libc_free(data as *mut c_void);
        libc_free(img.pix as *mut c_void);
        img.pix = std::ptr::null_mut();
        cp_image_t {
            w: img.w,
            h: img.h,
            pix: img.pix,
        }
    }
}

// Use libc malloc/free so the C caller can free the same buffer if needed.
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

unsafe fn libc_malloc(size: usize) -> *mut c_void {
    unsafe { malloc(size) }
}

unsafe fn libc_free(ptr: *mut c_void) {
    unsafe { free(ptr) }
}
