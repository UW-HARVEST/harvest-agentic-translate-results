#![allow(dead_code, unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr::{copy_nonoverlapping, null, write_bytes};

#[repr(C)]
#[derive(Copy, Clone)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct CpImage {
    w: c_int,
    h: c_int,
    pix: *mut CpPixel,
}

#[repr(C)]
struct CpRawPng {
    p: *const u8,
    end: *const u8,
}

#[repr(C)]
struct CpState {
    bits: u64,
    count: c_int,
    words: *const u32,
    word_count: c_int,
    word_index: c_int,
    bits_left: c_int,
    final_word_available: c_int,
    final_word: u32,
    out: *mut u8,
    out_end: *mut u8,
    begin: *mut u8,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = null();

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
pub static mut cp_permutation_order: [u8; 19] =
    [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5,
    5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67,
    83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11,
    11, 12, 12, 13, 13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769,
    1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

const ERR_STORED_COMPLEMENT: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
const ERR_STORED_EXTENDS: &[u8] = b"Stored block extends beyond end of input stream.\0";
const ERR_OUT_SYMBOL: &[u8] =
    b"Attempted to overwrite out buffer while outputting a symbol.\0";
const ERR_BACKWARDS_DISTANCE: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
const ERR_OUT_STRING: &[u8] =
    b"Attempted to overwrite out buffer while outputting a string.\0";
const ERR_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> CpPixel {
    CpPixel { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> CpPixel {
    CpPixel { r, g, b, a: 0xFF }
}

unsafe fn cp_would_overflow(s: *mut CpState, num_bits: c_int) -> bool {
    ((*s).bits_left + (*s).count) - num_bits < 0
}

unsafe fn cp_ptr(s: *mut CpState) -> *mut u8 {
    assert!(((*s).bits_left & 7) == 0);
    ((*s).words as *mut u8)
        .add((*s).word_index as usize * size_of::<u32>())
        .sub(((*s).count / 8) as usize)
}

unsafe fn cp_peak_bits(s: *mut CpState, num_bits_to_read: c_int) -> u64 {
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

unsafe fn cp_consume_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!((*s).count >= num_bits_to_read);
    let mask = ((1u64 << num_bits_to_read) - 1) as u64;
    let bits = ((*s).bits & mask) as u32;
    (*s).bits >>= num_bits_to_read;
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!((*s).bits_left > 0);
    assert!((*s).count <= 64);
    assert!(!cp_would_overflow(s, num_bits_to_read));
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

unsafe fn cp_build(
    s: *mut CpState,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
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
        (*s).lookup.fill(0);
    }

    for i in 0..sym_count {
        let len = *lens.add(i as usize) as i32;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as usize;
            first[len as usize] += 1;
            *tree.add(slot) = (code << (32 - len)) | ((i as u32) << 4) | len as u32;
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                while j < (1 << 9) {
                    (*s).lookup[j] = ((len << 9) | i) as u16;
                    j += 1usize << len;
                }
            }
        }
    }

    first[15]
}

unsafe fn cp_stored(s: *mut CpState) -> c_int {
    cp_read_bits(s, (*s).count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;

    if len != !nlen {
        cp_error_reason = ERR_STORED_COMPLEMENT.as_ptr() as *const c_char;
        return 0;
    }
    if (*s).bits_left / 8 > len as c_int {
        cp_error_reason = ERR_STORED_EXTENDS.as_ptr() as *const c_char;
        return 0;
    }

    let p = cp_ptr(s);
    copy_nonoverlapping(p, (*s).out, len as usize);
    (*s).out = (*s).out.add(len as usize);
    1
}

unsafe fn cp_fixed(s: *mut CpState) -> c_int {
    (*s).nlit = cp_build(
        s,
        (*s).lit.as_mut_ptr(),
        core::ptr::addr_of!(cp_fixed_table) as *const u8,
        288,
    ) as u32;
    (*s).ndst = cp_build(
        null::<CpState>() as *mut CpState,
        (*s).dst.as_mut_ptr(),
        (core::ptr::addr_of!(cp_fixed_table) as *const u8).add(288),
        32,
    ) as u32;
    1
}

unsafe fn cp_decode(s: *mut CpState, tree: *mut u32, mut hi: c_int) -> c_int {
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
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: *mut CpState) -> c_int {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as c_int;
    let ndst = 1 + cp_read_bits(s, 5) as c_int;
    let nlen = 4 + cp_read_bits(s, 4) as c_int;

    for i in 0..nlen {
        lenlens[cp_permutation_order[i as usize] as usize] = cp_read_bits(s, 3) as u8;
    }

    (*s).nlen = cp_build(
        null::<CpState>() as *mut CpState,
        (*s).len.as_mut_ptr(),
        lenlens.as_ptr(),
        19,
    ) as u32;

    let mut lens = [0u8; 288 + 32];
    let mut n = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, (*s).len.as_mut_ptr(), (*s).nlen as c_int);
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

    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
    (*s).ndst = cp_build(
        null::<CpState>() as *mut CpState,
        (*s).dst.as_mut_ptr(),
        lens.as_ptr().add(nlit as usize),
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut CpState) -> c_int {
    loop {
        let mut symbol = cp_decode(s, (*s).lit.as_mut_ptr(), (*s).nlit as c_int);
        if symbol < 256 {
            if (*s).out.wrapping_add(1) > (*s).out_end {
                cp_error_reason = ERR_OUT_SYMBOL.as_ptr() as *const c_char;
                return 0;
            }
            *(*s).out = symbol as u8;
            (*s).out = (*s).out.add(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length = cp_read_bits(
                s,
                cp_len_extra_bits[symbol as usize] as c_int,
            ) as c_int
                + cp_len_base[symbol as usize] as c_int;
            let distance_symbol = cp_decode(s, (*s).dst.as_mut_ptr(), (*s).ndst as c_int);
            let backwards_distance = cp_read_bits(
                s,
                cp_dist_extra_bits[distance_symbol as usize] as c_int,
            ) as c_int
                + cp_dist_base[distance_symbol as usize] as c_int;

            if (*s).out.wrapping_offset(-(backwards_distance as isize)) < (*s).begin {
                cp_error_reason = ERR_BACKWARDS_DISTANCE.as_ptr() as *const c_char;
                return 0;
            }
            if (*s).out.wrapping_add(length as usize) > (*s).out_end {
                cp_error_reason = ERR_OUT_STRING.as_ptr() as *const c_char;
                return 0;
            }

            let mut src = (*s).out.sub(backwards_distance as usize);
            let mut dst = (*s).out;
            (*s).out = (*s).out.add(length as usize);

            match backwards_distance {
                1 => write_bytes(dst, *src, length as usize),
                _ => {
                    let mut remaining = length;
                    while remaining != 0 {
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
                        remaining -= 1;
                    }
                }
            }
        } else {
            break;
        }
    }
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

fn cp_make32(s: *const u8) -> u32 {
    unsafe {
        ((*s.add(0) as u32) << 24)
            | ((*s.add(1) as u32) << 16)
            | ((*s.add(2) as u32) << 8)
            | (*s.add(3) as u32)
    }
}

unsafe fn cp_chunk(png: *mut CpRawPng, chunk: *const c_char, minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if libc_memcmp(start.add(4), chunk as *const u8, 4) == 0 && len >= minlen {
        let offset = len as usize + 12;
        if start.add(offset) <= (*png).end {
            (*png).p = start.add(offset);
            return start.add(8);
        }
    }
    null()
}

unsafe fn cp_find(png: *mut CpRawPng, chunk: *const c_char, minlen: u32) -> *const u8 {
    while (*png).p < (*png).end {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        (*png).p = (*png).p.add(len as usize + 12);
        if libc_memcmp(start.add(4), chunk as *const u8, 4) == 0
            && len >= minlen
            && (*png).p <= (*png).end
        {
            return start.add(8);
        }
    }
    null()
}

fn libc_memcmp(a: *const u8, b: *const u8, len: usize) -> c_int {
    for i in 0..len {
        let av = unsafe { *a.add(i) };
        let bv = unsafe { *b.add(i) };
        if av != bv {
            return av as c_int - bv as c_int;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let _ = cp_make_pixel_a(0, 0, 0, 0);
    let _ = cp_make_pixel(0, 0, 0);

    let mut state = Box::new(CpState {
        bits: 0,
        count: 0,
        words: null(),
        word_count: 0,
        word_index: 0,
        bits_left: 0,
        final_word_available: 0,
        final_word: 0,
        out: out as *mut u8,
        out_end: out as *mut u8,
        begin: out as *mut u8,
        lookup: [0; 1 << 9],
        lit: [0; 288],
        dst: [0; 32],
        len: [0; 19],
        nlit: 0,
        ndst: 0,
        nlen: 0,
    });

    let s = &mut *state as *mut CpState;
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes * 8;

    let in_addr = input as usize;
    let first_bytes = (((in_addr + 3) & !3usize) - in_addr) as c_int;
    (*s).words = (input as *mut u8).wrapping_offset(first_bytes as isize) as *const u32;
    (*s).word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;

    for i in 0..first_bytes {
        (*s).bits |= (*(input as *const u8).add(i as usize) as u64) << (i * 8);
    }

    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        (*s).final_word |=
            (*(input as *const u8).add((in_bytes - last_bytes + i) as usize) as u32) << (i * 8);
    }

    (*s).count = first_bytes * 8;
    (*s).out = out as *mut u8;
    (*s).out_end = (out as *mut u8).wrapping_offset(out_bytes as isize);
    (*s).begin = out as *mut u8;

    let mut _count = 0;
    let mut bfinal;
    loop {
        bfinal = cp_read_bits(s, 1);
        let btype = cp_read_bits(s, 2);
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
                cp_error_reason = ERR_UNKNOWN_BLOCK.as_ptr() as *const c_char;
                return 0;
            }
            _ => {}
        }
        _count += 1;
        if bfinal != 0 {
            break;
        }
    }

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len = w * bpp;
    let mut raw = raw;
    let mut prev: *mut u8;
    let mut x;

    if h > 0 {
        match *raw {
            0 => {
                raw = raw.add(1);
            }
            1 => {
                raw = raw.add(1);
                x = bpp;
                while x < len {
                    let idx = x as usize;
                    *raw.add(idx) = (*raw.add(idx)).wrapping_add(*raw.add((x - bpp) as usize));
                    x += 1;
                }
            }
            2 => {
                raw = raw.add(1);
            }
            3 => {
                raw = raw.add(1);
                x = bpp;
                while x < len {
                    let idx = x as usize;
                    *raw.add(idx) =
                        (*raw.add(idx)).wrapping_add(*raw.add((x - bpp) as usize) / 2);
                    x += 1;
                }
            }
            4 => {
                raw = raw.add(1);
                x = bpp;
                while x < len {
                    let idx = x as usize;
                    *raw.add(idx) = (*raw.add(idx))
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
        match *raw {
            0 => {
                raw = raw.add(1);
            }
            1 => {
                raw = raw.add(1);
                x = 0;
                while x < bpp {
                    let idx = x as usize;
                    *raw.add(idx) = (*raw.add(idx)).wrapping_add(0);
                    x += 1;
                }
                while x < len {
                    let idx = x as usize;
                    *raw.add(idx) = (*raw.add(idx)).wrapping_add(*raw.add((x - bpp) as usize));
                    x += 1;
                }
            }
            2 => {
                raw = raw.add(1);
                x = 0;
                while x < bpp {
                    let idx = x as usize;
                    *raw.add(idx) = (*raw.add(idx)).wrapping_add(*prev.add(idx));
                    x += 1;
                }
                while x < len {
                    let idx = x as usize;
                    *raw.add(idx) = (*raw.add(idx)).wrapping_add(*prev.add(idx));
                    x += 1;
                }
            }
            3 => {
                raw = raw.add(1);
                x = 0;
                while x < bpp {
                    let idx = x as usize;
                    *raw.add(idx) = (*raw.add(idx)).wrapping_add(*prev.add(idx) / 2);
                    x += 1;
                }
                while x < len {
                    let idx = x as usize;
                    *raw.add(idx) = (*raw.add(idx)).wrapping_add(
                        ((*raw.add((x - bpp) as usize) as u16 + *prev.add(idx) as u16) / 2) as u8,
                    );
                    x += 1;
                }
            }
            4 => {
                raw = raw.add(1);
                x = 0;
                while x < bpp {
                    let idx = x as usize;
                    *raw.add(idx) = (*raw.add(idx)).wrapping_add(*prev.add(idx));
                    x += 1;
                }
                while x < len {
                    let idx = x as usize;
                    *raw.add(idx) = (*raw.add(idx)).wrapping_add(cp_paeth(
                        *raw.add((x - bpp) as usize),
                        *prev.add(idx),
                        *prev.add((x - bpp) as usize),
                    ));
                    x += 1;
                }
            }
            _ => return 0,
        }

        prev = raw;
        raw = raw.add(len as usize);
        y += 1;
    }

    1
}
