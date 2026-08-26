#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_assignments)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(static_mut_refs)]

use std::ffi::c_int;
use std::os::raw::c_char;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
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
pub static mut cp_fixed_table: [u8; 288 + 32] = {
    let mut t = [0u8; 320];
    let mut i = 0;
    while i < 144 {
        t[i] = 8;
        i += 1;
    }
    while i < 256 {
        t[i] = 9;
        i += 1;
    }
    while i < 280 {
        t[i] = 7;
        i += 1;
    }
    while i < 288 {
        t[i] = 8;
        i += 1;
    }
    while i < 320 {
        t[i] = 5;
        i += 1;
    }
    t
};

#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
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

impl cp_state_t {
    fn zeroed() -> Self {
        cp_state_t {
            bits: 0,
            count: 0,
            words: std::ptr::null_mut(),
            word_count: 0,
            word_index: 0,
            bits_left: 0,
            final_word_available: 0,
            final_word: 0,
            out: std::ptr::null_mut(),
            out_end: std::ptr::null_mut(),
            begin: std::ptr::null_mut(),
            lookup: [0u16; 1 << 9],
            lit: [0u32; 288],
            dst: [0u32; 32],
            len: [0u32; 19],
            nlit: 0,
            ndst: 0,
            nlen: 0,
        }
    }
}

fn cp_would_overflow(s: &cp_state_t, num_bits: c_int) -> c_int {
    ((s.bits_left + s.count) - num_bits < 0) as c_int
}

unsafe fn cp_ptr(s: &cp_state_t) -> *mut c_char {
    debug_assert!((s.bits_left & 7) == 0);
    unsafe {
        (s.words.offset(s.word_index as isize) as *mut c_char).offset(-((s.count / 8) as isize))
    }
}

unsafe fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { *s.words.offset(s.word_index as isize) };
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            debug_assert!(s.word_index <= s.word_count);
        } else if s.final_word_available != 0 {
            let word = s.final_word;
            s.bits |= (word as u64) << s.count;
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

fn cp_consume_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    debug_assert!(s.count >= num_bits_to_read);
    let mask: u64 = if num_bits_to_read >= 64 {
        u64::MAX
    } else {
        (1u64 << num_bits_to_read) - 1
    };
    let bits = (s.bits & mask) as u32;
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    debug_assert!(s.bits_left > 0);
    debug_assert!(s.count <= 64);
    debug_assert!(cp_would_overflow(s, num_bits_to_read) == 0);
    unsafe {
        cp_peak_bits(s, num_bits_to_read);
    }
    cp_consume_bits(s, num_bits_to_read)
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
    let mut codes: [c_int; 16] = [0; 16];
    let mut first: [c_int; 16] = [0; 16];
    let mut counts: [c_int; 16] = [0; 16];
    for n in 0..sym_count {
        let l = unsafe { *lens.offset(n as isize) } as usize;
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
        unsafe {
            (*s).lookup = [0u16; 1 << 9];
        }
    }
    for i in 0..sym_count {
        let len = unsafe { *lens.offset(i as isize) } as c_int;
        if len != 0 {
            debug_assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as u32;
            first[len as usize] += 1;
            unsafe {
                *tree.offset(slot as isize) = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            }
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    unsafe {
                        (*s).lookup[j as usize] = (((len as u16) << 9) | (i as u16)) as u16;
                    }
                    j += 1 << len;
                }
            }
        }
    }
    let max_index = first[15];
    max_index
}

unsafe fn cp_stored(s: &mut cp_state_t) -> c_int {
    unsafe {
        cp_read_bits(s, s.count & 7);
        let LEN: u16 = cp_read_bits(s, 16) as u16;
        let NLEN: u16 = cp_read_bits(s, 16) as u16;
        if !(LEN == !NLEN) {
            cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
            return 0;
        }
        if !(s.bits_left / 8 <= LEN as c_int) {
            cp_error_reason =
                b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
            return 0;
        }
        let p = cp_ptr(s);
        std::ptr::copy_nonoverlapping(p as *const u8, s.out as *mut u8, LEN as usize);
        s.out = s.out.offset(LEN as isize);
        1
    }
}

unsafe fn cp_fixed(s: &mut cp_state_t) -> c_int {
    unsafe {
        s.nlit = cp_build(s as *mut cp_state_t, s.lit.as_mut_ptr(), cp_fixed_table.as_ptr(), 288)
            as u32;
        s.ndst = cp_build(
            std::ptr::null_mut(),
            s.dst.as_mut_ptr(),
            cp_fixed_table.as_ptr().offset(288),
            32,
        ) as u32;
    }
    1
}

unsafe fn cp_decode(s: &mut cp_state_t, tree: *mut u32, hi: c_int) -> c_int {
    let mut hi = hi;
    let bits = unsafe { cp_peak_bits(s, 16) };
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        let val = unsafe { *tree.offset(guess as isize) };
        if search < val {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = unsafe { *tree.offset((lo - 1) as isize) };
    let len: u32 = 32 - (key & 0xF);
    debug_assert!((search >> len) == (key >> len));
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: &mut cp_state_t) -> c_int {
    unsafe {
        let mut lenlens: [u8; 19] = [0; 19];
        let nlit = 257 + cp_read_bits(s, 5) as c_int;
        let ndst = 1 + cp_read_bits(s, 5) as c_int;
        let nlen = 4 + cp_read_bits(s, 4) as c_int;
        for i in 0..nlen {
            let idx = cp_permutation_order[i as usize] as usize;
            lenlens[idx] = cp_read_bits(s, 3) as u8;
        }
        s.nlen = cp_build(
            std::ptr::null_mut(),
            s.len.as_mut_ptr(),
            lenlens.as_ptr(),
            19,
        ) as u32;
        let mut lens: [u8; 288 + 32] = [0; 288 + 32];
        let mut n: c_int = 0;
        while n < nlit + ndst {
            let len_ptr = s.len.as_mut_ptr();
            let nlen_val = s.nlen as c_int;
            let sym = cp_decode(s, len_ptr, nlen_val);
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
        s.nlit =
            cp_build(s as *mut cp_state_t, s.lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
        s.ndst = cp_build(
            std::ptr::null_mut(),
            s.dst.as_mut_ptr(),
            lens.as_ptr().offset(nlit as isize),
            ndst,
        ) as u32;
    }
    1
}

unsafe fn cp_block(s: &mut cp_state_t) -> c_int {
    unsafe {
        loop {
            let lit_ptr = s.lit.as_mut_ptr();
            let nlit_val = s.nlit as c_int;
            let mut symbol = cp_decode(s, lit_ptr, nlit_val);
            if symbol < 256 {
                if !(s.out.offset(1) <= s.out_end) {
                    cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                    return 0;
                }
                *s.out = symbol as c_char;
                s.out = s.out.offset(1);
            } else if symbol > 256 {
                symbol -= 257;
                let length = cp_read_bits(s, cp_len_extra_bits[symbol as usize] as c_int) as c_int
                    + cp_len_base[symbol as usize] as c_int;
                let dst_ptr = s.dst.as_mut_ptr();
                let ndst_val = s.ndst as c_int;
                let distance_symbol = cp_decode(s, dst_ptr, ndst_val);
                let backwards_distance = cp_read_bits(
                    s,
                    cp_dist_extra_bits[distance_symbol as usize] as c_int,
                ) as c_int
                    + cp_dist_base[distance_symbol as usize] as c_int;
                if !(s.out.offset(-(backwards_distance as isize)) >= s.begin) {
                    cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                    return 0;
                }
                if !(s.out.offset(length as isize) <= s.out_end) {
                    cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                    return 0;
                }
                let mut src = s.out.offset(-(backwards_distance as isize));
                let mut dst = s.out;
                s.out = s.out.offset(length as isize);
                match backwards_distance {
                    1 => {
                        std::ptr::write_bytes(dst as *mut u8, *src as u8, length as usize);
                    }
                    _ => {
                        let mut length = length;
                        while length != 0 {
                            length -= 1;
                            *dst = *src;
                            dst = dst.offset(1);
                            src = src.offset(1);
                        }
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
    in_ptr: *mut std::ffi::c_void,
    in_bytes: c_int,
    out_ptr: *mut std::ffi::c_void,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        // calloc allocation - emulate with Box
        let mut state_box = Box::new(cp_state_t::zeroed());
        let s: &mut cp_state_t = &mut *state_box;
        s.bits = 0;
        s.count = 0;
        s.word_index = 0;
        s.bits_left = in_bytes * 8;
        let in_addr = in_ptr as usize;
        let first_bytes = ((in_addr + 3) & !3usize).wrapping_sub(in_addr) as c_int;
        s.words = (in_ptr as *mut c_char).offset(first_bytes as isize) as *mut u32;
        s.word_count = (in_bytes - first_bytes) / 4;
        let last_bytes = (in_bytes - first_bytes) & 3;
        for i in 0..first_bytes {
            let byte = *(in_ptr as *const u8).offset(i as isize);
            s.bits |= (byte as u64) << (i * 8);
        }
        s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
        s.final_word = 0;
        for i in 0..last_bytes {
            let byte =
                *(in_ptr as *const u8).offset((in_bytes - last_bytes + i) as isize);
            // Note: C does `byte << (i*8)` without explicit cast — byte is uint8_t,
            // gets promoted to int, shifts can be up to 24 which is fine for int.
            s.final_word |= (byte as u32) << (i * 8);
        }
        s.count = first_bytes * 8;
        s.out = out_ptr as *mut c_char;
        s.out_end = s.out.offset(out_bytes as isize);
        s.begin = out_ptr as *mut c_char;

        let mut count: c_int = 0;
        let mut bfinal: c_int;
        loop {
            bfinal = cp_read_bits(s, 1) as c_int;
            let btype = cp_read_bits(s, 2) as c_int;
            match btype {
                0 => {
                    if cp_stored(s) == 0 {
                        return 0;
                    }
                }
                1 => {
                    cp_fixed(s);
                    if cp_block(s) == 0 {
                        return 0;
                    }
                }
                2 => {
                    cp_dynamic(s);
                    if cp_block(s) == 0 {
                        return 0;
                    }
                }
                3 => {
                    cp_error_reason =
                        b"Detected unknown block type within input stream.\0".as_ptr()
                            as *const c_char;
                    return 0;
                }
                _ => {}
            }
            count += 1;
            if bfinal != 0 {
                break;
            }
        }
        1
    }
}

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p: c_int = a as c_int + b as c_int - c as c_int;
    let pa: c_int = (p - a as c_int).abs();
    let pb: c_int = (p - b as c_int).abs();
    let pc: c_int = (p - c as c_int).abs();
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
        let s0 = *s.offset(0) as u32;
        let s1 = *s.offset(1) as u32;
        let s2 = *s.offset(2) as u32;
        let s3 = *s.offset(3) as u32;
        // Match C: (s[0] << 24) | (s[1] << 16) | (s[2] << 8) | s[3]
        // In C, uint8_t is promoted to int before shift, so s[0]<<24 could be UB
        // for values >= 128 (signed overflow). We replicate using wrapping u32 ops.
        (s0 << 24) | (s1 << 16) | (s2 << 8) | s3
    }
}

unsafe fn cp_chunk(
    png: &mut cp_raw_png_t,
    chunk: *const c_char,
    minlen: u32,
) -> *const u8 {
    unsafe {
        let len = cp_make32(png.p);
        let start = png.p;
        let cmp = libc_memcmp(start.offset(4), chunk as *const u8, 4);
        if cmp == 0 && len >= minlen {
            let offset = (len + 12) as isize;
            if png.p.offset(offset) <= png.end {
                png.p = png.p.offset(offset);
                return start.offset(8);
            }
        }
        std::ptr::null()
    }
}

unsafe fn cp_find(png: &mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    unsafe {
        while png.p < png.end {
            let len = cp_make32(png.p);
            let start = png.p;
            png.p = png.p.offset((len + 12) as isize);
            let cmp = libc_memcmp(start.offset(4), chunk as *const u8, 4);
            if cmp == 0 && len >= minlen && png.p <= png.end {
                return start.offset(8);
            }
        }
        std::ptr::null()
    }
}

unsafe fn libc_memcmp(a: *const u8, b: *const u8, n: usize) -> c_int {
    unsafe {
        for i in 0..n {
            let av = *a.add(i);
            let bv = *b.add(i);
            if av != bv {
                return av as c_int - bv as c_int;
            }
        }
        0
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
            raw = raw.offset(1);
            match filter {
                0 => {}
                1 => {
                    x = bpp;
                    while x < len {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize));
                        x += 1;
                    }
                }
                2 => {}
                3 => {
                    x = bpp;
                    while x < len {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize) / 2);
                        x += 1;
                    }
                }
                4 => {
                    x = bpp;
                    while x < len {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add(cp_paeth(*raw.offset((x - bpp) as isize), 0, 0));
                        x += 1;
                    }
                }
                _ => return 0,
            }
        }
        prev = raw;
        raw = raw.offset(len as isize);
        let mut y = 1;
        while y < h {
            let filter = *raw;
            raw = raw.offset(1);
            match filter {
                0 => {}
                1 => {
                    x = 0;
                    while x < bpp {
                        *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(0);
                        x += 1;
                    }
                    while x < len {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize));
                        x += 1;
                    }
                }
                2 => {
                    x = 0;
                    while x < bpp {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                        x += 1;
                    }
                    while x < len {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                        x += 1;
                    }
                }
                3 => {
                    x = 0;
                    while x < bpp {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize) / 2);
                        x += 1;
                    }
                    while x < len {
                        let a = *raw.offset((x - bpp) as isize) as c_int;
                        let b = *prev.offset(x as isize) as c_int;
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(((a + b) / 2) as u8);
                        x += 1;
                    }
                }
                4 => {
                    x = 0;
                    while x < bpp {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                        x += 1;
                    }
                    while x < len {
                        let a = *raw.offset((x - bpp) as isize);
                        let b = *prev.offset(x as isize);
                        let c = *prev.offset((x - bpp) as isize);
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(cp_paeth(a, b, c));
                        x += 1;
                    }
                }
                _ => return 0,
            }
            y += 1;
            prev = raw;
            raw = raw.offset(len as isize);
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn convert_pix(
    bpp: c_int,
    w: c_int,
    h: c_int,
    src: *mut u8,
    dst: *mut cp_pixel_t,
) {
    unsafe {
        let mut src = src;
        let mut dst = dst;
        for _y in 0..h {
            src = src.offset(1);
            for _x in 0..w {
                match bpp {
                    1 => {
                        *dst = cp_make_pixel(*src.offset(0), *src.offset(0), *src.offset(0));
                        dst = dst.offset(1);
                    }
                    2 => {
                        *dst = cp_make_pixel_a(
                            *src.offset(0),
                            *src.offset(0),
                            *src.offset(0),
                            *src.offset(1),
                        );
                        dst = dst.offset(1);
                    }
                    3 => {
                        *dst = cp_make_pixel(*src.offset(0), *src.offset(1), *src.offset(2));
                        dst = dst.offset(1);
                    }
                    4 => {
                        *dst = cp_make_pixel_a(
                            *src.offset(0),
                            *src.offset(1),
                            *src.offset(2),
                            *src.offset(3),
                        );
                        dst = dst.offset(1);
                    }
                    _ => {}
                }
                src = src.offset(bpp as isize);
            }
        }
    }
}
