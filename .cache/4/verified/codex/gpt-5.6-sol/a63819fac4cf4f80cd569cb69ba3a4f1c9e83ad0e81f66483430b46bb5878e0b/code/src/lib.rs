#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

const fn pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

const fn pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    pixel_a(r, g, b, 0xff)
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

const fn make_fixed_table() -> [u8; 288 + 32] {
    let mut table = [0; 288 + 32];
    let mut i = 0;
    while i < 144 {
        table[i] = 8;
        i += 1;
    }
    while i < 256 {
        table[i] = 9;
        i += 1;
    }
    while i < 280 {
        table[i] = 7;
        i += 1;
    }
    while i < 288 {
        table[i] = 8;
        i += 1;
    }
    while i < 320 {
        table[i] = 5;
        i += 1;
    }
    table
}

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = make_fixed_table();

#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

const ERR_STORED_COMPLEMENT: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
const ERR_STORED_END: &[u8] = b"Stored block extends beyond end of input stream.\0";
const ERR_SYMBOL_OUTPUT: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.\0";
const ERR_BACKWARDS: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
const ERR_STRING_OUTPUT: &[u8] = b"Attempted to overwrite out buffer while outputting a string.\0";
const ERR_BLOCK_TYPE: &[u8] = b"Detected unknown block type within input stream.\0";
const ERR_SIGNATURE: &[u8] = b"incorrect file signature (is this a png file?)\0";
const ERR_IHDR: &[u8] = b"unable to find IHDR chunk\0";
const ERR_BIT_DEPTH: &[u8] = b"only bit-depth of 8 is supported\0";
const ERR_COLOR_TYPE: &[u8] = b"unknown color type\0";
const ERR_WIDTH: &[u8] = b"invalid IHDR chunk found, image width was less than 1\0";
const ERR_HEIGHT: &[u8] = b"invalid IHDR chunk found, image height was less than 1\0";
const ERR_IMAGE_TOO_LARGE: &[u8] = b"image too large\0";
const ERR_ALLOC_IMAGE: &[u8] = b"unable to allocate raw image space\0";
const ERR_COMPRESSION: &[u8] = b"only standard compression DEFLATE is supported\0";
const ERR_FILTER_METHOD: &[u8] = b"only standard adaptive filtering is supported\0";
const ERR_INTERLACE: &[u8] = b"interlacing is not supported\0";
const ERR_ZLIB_STRUCTURE: &[u8] = b"corrupt zlib structure in DEFLATE stream\0";
const ERR_ZLIB_METHOD: &[u8] = b"only zlib compression method (RFC 1950) is supported\0";
const ERR_WINDOW: &[u8] = b"innapropriate window size detected\0";
const ERR_DICTIONARY: &[u8] = b"preset dictionary is present and not supported\0";
const ERR_IMAGE_SIZE: &[u8] = b"invalid image size found\0";
const ERR_DEFLATE: &[u8] = b"DEFLATE algorithm failed\0";
const ERR_FILTER_BYTE: &[u8] = b"invalid filter byte found\0";
const ERR_PALETTE: &[u8] = b"color type of indexed requires a PLTE chunk\0";

unsafe fn set_error(message: &'static [u8]) {
    cp_error_reason = message.as_ptr().cast();
}

#[repr(C)]
struct CpState {
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

unsafe fn would_overflow(s: *mut CpState, num_bits: c_int) -> bool {
    ((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0
}

unsafe fn state_ptr(s: *mut CpState) -> *mut c_char {
    assert!((*s).bits_left & 7 == 0);
    (*s).words
        .offset((*s).word_index as isize)
        .cast::<c_char>()
        .offset(-(((*s).count / 8) as isize))
}

unsafe fn peek_bits(s: *mut CpState, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.offset((*s).word_index as isize);
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

unsafe fn consume_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!((*s).count >= num_bits_to_read);
    let mask = (1u64 << num_bits_to_read) - 1;
    let bits = ((*s).bits & mask) as u32;
    (*s).bits >>= num_bits_to_read;
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits
}

unsafe fn read_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!((*s).bits_left > 0);
    assert!((*s).count <= 64);
    assert!(!would_overflow(s, num_bits_to_read));
    peek_bits(s, num_bits_to_read);
    consume_bits(s, num_bits_to_read)
}

fn rev16(mut a: u32) -> u32 {
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8)
}

unsafe fn build(s: *mut CpState, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    for n in 0..sym_count {
        counts[*lens.offset(n as isize) as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if !s.is_null() {
        ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, 1 << 9);
    }
    for i in 0..sym_count {
        let len = *lens.offset(i as isize) as usize;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as isize;
            first[len] += 1;
            *tree.offset(slot) = (code << (32 - len)) | ((i as u32) << 4) | len as u32;
            if !s.is_null() && len <= 9 {
                let mut j = (rev16(code) >> (16 - len)) as usize;
                while j < (1 << 9) {
                    (*s).lookup[j] = ((len << 9) | i as usize) as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn stored(s: *mut CpState) -> c_int {
    read_bits(s, (*s).count & 7);
    let len = read_bits(s, 16) as u16;
    let nlen = read_bits(s, 16) as u16;
    if len != !nlen {
        set_error(ERR_STORED_COMPLEMENT);
        return 0;
    }
    if (*s).bits_left / 8 > len as c_int {
        set_error(ERR_STORED_END);
        return 0;
    }
    let p = state_ptr(s);
    ptr::copy_nonoverlapping(p, (*s).out, len as usize);
    (*s).out = (*s).out.add(len as usize);
    1
}

unsafe fn fixed(s: *mut CpState) -> c_int {
    (*s).nlit = build(
        s,
        (*s).lit.as_mut_ptr(),
        ptr::addr_of!(cp_fixed_table).cast::<u8>(),
        288,
    ) as u32;
    (*s).ndst = build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        ptr::addr_of!(cp_fixed_table).cast::<u8>().add(288),
        32,
    ) as u32;
    1
}

unsafe fn decode(s: *mut CpState, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = peek_bits(s, 16);
    let search = (rev16(bits as u32) << 16) | 0xffff;
    let mut lo = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.offset((lo - 1) as isize);
    let len = 32 - (key & 0x0f);
    assert!((search >> len) == (key >> len));
    consume_bits(s, (key & 0x0f) as c_int);
    ((key >> 4) & 0x0fff) as c_int
}

unsafe fn dynamic(s: *mut CpState) -> c_int {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + read_bits(s, 5) as c_int;
    let ndst = 1 + read_bits(s, 5) as c_int;
    let nlen = 4 + read_bits(s, 4) as c_int;
    for i in 0..nlen {
        let order = *ptr::addr_of!(cp_permutation_order)
            .cast::<u8>()
            .offset(i as isize) as usize;
        lenlens[order] = read_bits(s, 3) as u8;
    }
    (*s).nlen = build(ptr::null_mut(), (*s).len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;
    let mut lens = [0u8; 288 + 32];
    let mut n = 0;
    while n < nlit + ndst {
        let sym = decode(s, (*s).len.as_ptr(), (*s).nlen as c_int);
        match sym {
            16 => {
                let mut i = 3 + read_bits(s, 2) as c_int;
                while i != 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + read_bits(s, 3) as c_int;
                while i != 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + read_bits(s, 7) as c_int;
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
    (*s).nlit = build(s, (*s).lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
    (*s).ndst = build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        lens.as_ptr().add(nlit as usize),
        ndst,
    ) as u32;
    1
}

unsafe fn block(s: *mut CpState) -> c_int {
    loop {
        let mut symbol = decode(s, (*s).lit.as_ptr(), (*s).nlit as c_int);
        if symbol < 256 {
            if (*s).out.add(1) > (*s).out_end {
                set_error(ERR_SYMBOL_OUTPUT);
                return 0;
            }
            *(*s).out = symbol as u8 as c_char;
            (*s).out = (*s).out.add(1);
        } else if symbol > 256 {
            symbol -= 257;
            let len_extra = *ptr::addr_of!(cp_len_extra_bits)
                .cast::<u8>()
                .offset(symbol as isize);
            let len_base = *ptr::addr_of!(cp_len_base)
                .cast::<u32>()
                .offset(symbol as isize);
            let mut length = read_bits(s, len_extra as c_int).wrapping_add(len_base) as c_int;
            let distance_symbol = decode(s, (*s).dst.as_ptr(), (*s).ndst as c_int);
            let dist_extra = *ptr::addr_of!(cp_dist_extra_bits)
                .cast::<u8>()
                .offset(distance_symbol as isize);
            let dist_base = *ptr::addr_of!(cp_dist_base)
                .cast::<u32>()
                .offset(distance_symbol as isize);
            let backwards_distance =
                read_bits(s, dist_extra as c_int).wrapping_add(dist_base) as c_int;
            if (*s).out.offset(-(backwards_distance as isize)) < (*s).begin {
                set_error(ERR_BACKWARDS);
                return 0;
            }
            if (*s).out.offset(length as isize) > (*s).out_end {
                set_error(ERR_STRING_OUTPUT);
                return 0;
            }
            let mut src = (*s).out.offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.offset(length as isize);
            if backwards_distance == 1 {
                ptr::write_bytes(dst, *src as u8, length as usize);
            } else {
                while length != 0 {
                    *dst = *src;
                    dst = dst.add(1);
                    src = src.add(1);
                    length -= 1;
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
    input: *mut c_void,
    in_bytes: c_int,
    output: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let s = calloc(1, size_of::<CpState>()).cast::<CpState>();
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);
    let input_addr = input as usize;
    let first_bytes = ((input_addr.wrapping_add(3) & !3).wrapping_sub(input_addr)) as c_int;
    (*s).words = input.cast::<u8>().add(first_bytes as usize).cast::<u32>();
    (*s).word_count = in_bytes.wrapping_sub(first_bytes) / 4;
    let last_bytes = in_bytes.wrapping_sub(first_bytes) & 3;
    for i in 0..first_bytes {
        (*s).bits |= (*input.cast::<u8>().offset(i as isize) as u64) << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        (*s).final_word |= (*input
            .cast::<u8>()
            .offset((in_bytes - last_bytes + i) as isize) as u32)
            << (i * 8);
    }
    (*s).count = first_bytes * 8;
    (*s).out = output.cast();
    (*s).out_end = (*s).out.offset(out_bytes as isize);
    (*s).begin = output.cast();

    loop {
        let bfinal = read_bits(s, 1);
        let btype = read_bits(s, 2);
        match btype {
            0 => {
                if stored(s) == 0 {
                    free(s.cast());
                    return 0;
                }
            }
            1 => {
                fixed(s);
                if block(s) == 0 {
                    free(s.cast());
                    return 0;
                }
            }
            2 => {
                dynamic(s);
                if block(s) == 0 {
                    free(s.cast());
                    return 0;
                }
            }
            _ => {
                set_error(ERR_BLOCK_TYPE);
                free(s.cast());
                return 0;
            }
        }
        if bfinal != 0 {
            break;
        }
    }
    free(s.cast());
    1
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as c_int + b as c_int - c as c_int;
    let pa = (p - a as c_int).abs();
    let pb = (p - b as c_int).abs();
    let pc = (p - c as c_int).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

struct RawPng {
    p: *const u8,
    end: *const u8,
}

unsafe fn make32(s: *const u8) -> u32 {
    ((*s as u32) << 24) | ((*s.add(1) as u32) << 16) | ((*s.add(2) as u32) << 8) | *s.add(3) as u32
}

unsafe fn chunk(png: *mut RawPng, name: *const u8, minlen: u32) -> *const u8 {
    let len = make32((*png).p);
    let start = (*png).p;
    if libc_memcmp(start.add(4), name, 4) == 0 && len >= minlen {
        let offset = len.wrapping_add(12) as usize;
        if (*png).p.add(offset) <= (*png).end {
            (*png).p = (*png).p.add(offset);
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn find(png: *mut RawPng, name: *const u8, minlen: u32) -> *const u8 {
    while (*png).p < (*png).end {
        let len = make32((*png).p);
        let start = (*png).p;
        (*png).p = (*png).p.add(len.wrapping_add(12) as usize);
        if libc_memcmp(start.add(4), name, 4) == 0 && len >= minlen && (*png).p <= (*png).end {
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn libc_memcmp(a: *const u8, b: *const u8, count: usize) -> c_int {
    unsafe extern "C" {
        fn memcmp(a: *const c_void, b: *const c_void, count: usize) -> c_int;
    }
    memcmp(a.cast(), b.cast(), count)
}

unsafe fn unfilter(w: c_int, h: c_int, bpp: c_int, mut raw: *mut u8) -> c_int {
    let len = w.wrapping_mul(bpp);
    if h > 0 {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 | 2 => {}
            1 => {
                for x in bpp..len {
                    let p = raw.offset(x as isize);
                    *p = (*p).wrapping_add(*p.offset(-(bpp as isize)));
                }
            }
            3 => {
                for x in bpp..len {
                    let p = raw.offset(x as isize);
                    *p = (*p).wrapping_add(*p.offset(-(bpp as isize)) / 2);
                }
            }
            4 => {
                for x in bpp..len {
                    let p = raw.offset(x as isize);
                    *p = (*p).wrapping_add(paeth(*p.offset(-(bpp as isize)), 0, 0));
                }
            }
            _ => return 0,
        }
    }
    let mut prev = raw;
    raw = raw.offset(len as isize);
    for _ in 1..h {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                for x in bpp..len {
                    let p = raw.offset(x as isize);
                    *p = (*p).wrapping_add(*p.offset(-(bpp as isize)));
                }
            }
            2 => {
                for x in 0..len {
                    let p = raw.offset(x as isize);
                    *p = (*p).wrapping_add(*prev.offset(x as isize));
                }
            }
            3 => {
                for x in 0..bpp {
                    let p = raw.offset(x as isize);
                    *p = (*p).wrapping_add(*prev.offset(x as isize) / 2);
                }
                for x in bpp..len {
                    let p = raw.offset(x as isize);
                    let average = ((*p.offset(-(bpp as isize)) as c_int
                        + *prev.offset(x as isize) as c_int)
                        / 2) as u8;
                    *p = (*p).wrapping_add(average);
                }
            }
            4 => {
                for x in 0..bpp {
                    let p = raw.offset(x as isize);
                    *p = (*p).wrapping_add(*prev.offset(x as isize));
                }
                for x in bpp..len {
                    let p = raw.offset(x as isize);
                    *p = (*p).wrapping_add(paeth(
                        *p.offset(-(bpp as isize)),
                        *prev.offset(x as isize),
                        *prev.offset((x - bpp) as isize),
                    ));
                }
            }
            _ => return 0,
        }
        prev = raw;
        raw = raw.offset(len as isize);
    }
    1
}

unsafe fn convert(bpp: c_int, w: c_int, h: c_int, mut src: *mut u8, mut dst: *mut cp_pixel_t) {
    for _ in 0..h {
        src = src.add(1);
        for _ in 0..w {
            *dst = match bpp {
                1 => pixel(*src, *src, *src),
                2 => pixel_a(*src, *src, *src, *src.add(1)),
                3 => pixel(*src, *src.add(1), *src.add(2)),
                4 => pixel_a(*src, *src.add(1), *src.add(2), *src.add(3)),
                _ => *dst,
            };
            dst = dst.add(1);
            src = src.offset(bpp as isize);
        }
    }
}

unsafe fn alpha_for_index(index: c_int, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() || index as u32 >= trns_len {
        255
    } else {
        *trns.offset(index as isize)
    }
}

unsafe fn depalette(
    w: c_int,
    h: c_int,
    mut src: *mut u8,
    mut dst: *mut cp_pixel_t,
    plte: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    for _ in 0..h {
        src = src.add(1);
        for _ in 0..w {
            let c = *src as usize;
            *dst = pixel_a(
                *plte.add(c * 3),
                *plte.add(c * 3 + 1),
                *plte.add(c * 3 + 2),
                alpha_for_index(c as c_int, trns, trns_len),
            );
            dst = dst.add(1);
            src = src.add(1);
        }
    }
}

unsafe fn chunk_byte_length(chunk: *const u8) -> u32 {
    make32(chunk.offset(-8))
}

fn out_size(img: &cp_image_t, bpp: c_int) -> c_int {
    img.w.wrapping_add(1).wrapping_mul(img.h).wrapping_mul(bpp)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    let mut img = cp_image_t {
        w: 0,
        h: 0,
        pix: ptr::null_mut(),
    };
    let mut data: *mut u8 = ptr::null_mut();
    let mut png = RawPng {
        p: png_data,
        end: png_data.offset(png_length as isize),
    };

    if libc_memcmp(png.p, b"\x89PNG\r\n\x1a\n".as_ptr(), 8) != 0 {
        set_error(ERR_SIGNATURE);
        return img;
    }
    png.p = png.p.add(8);
    let ihdr = chunk(&mut png, b"IHDR".as_ptr(), 13);
    if ihdr.is_null() {
        set_error(ERR_IHDR);
        return img;
    }
    let bit_depth = *ihdr.add(8) as c_int;
    let color_type = *ihdr.add(9) as c_int;
    if bit_depth != 8 {
        set_error(ERR_BIT_DEPTH);
        return img;
    }
    let bpp = match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => {
            set_error(ERR_COLOR_TYPE);
            return img;
        }
    };
    let w = make32(ihdr).wrapping_add(1) as c_int;
    let h = make32(ihdr.add(4)) as c_int;
    if w < 1 {
        set_error(ERR_WIDTH);
        return img;
    }
    if h < 1 {
        set_error(ERR_HEIGHT);
        return img;
    }
    if (w as i64)
        .wrapping_mul(h as i64)
        .wrapping_mul(size_of::<cp_pixel_t>() as i64)
        >= c_int::MAX as i64
    {
        set_error(ERR_IMAGE_TOO_LARGE);
        return img;
    }
    let pix_bytes = w
        .wrapping_mul(h)
        .wrapping_mul(size_of::<cp_pixel_t>() as c_int);
    img.w = w - 1;
    img.h = h;
    img.pix = malloc(pix_bytes as usize).cast();
    if img.pix.is_null() {
        set_error(ERR_ALLOC_IMAGE);
        return img;
    }
    let fail = |img: &mut cp_image_t, data: *mut u8, message: &'static [u8]| {
        set_error(message);
        free(data.cast());
        free(img.pix.cast());
        img.pix = ptr::null_mut();
        *img
    };

    let compression = *ihdr.add(10);
    let filter_method = *ihdr.add(11);
    let interlace = *ihdr.add(12);
    if compression != 0 {
        return fail(&mut img, data, ERR_COMPRESSION);
    }
    if filter_method != 0 {
        return fail(&mut img, data, ERR_FILTER_METHOD);
    }
    if interlace != 0 {
        return fail(&mut img, data, ERR_INTERLACE);
    }

    let mut first = png.p;
    let plte = find(&mut png, b"PLTE".as_ptr(), 0);
    if plte.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    let trns = find(&mut png, b"tRNS".as_ptr(), 0);
    if trns.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    let mut datalen = 0i32;
    let mut idat = find(&mut png, b"IDAT".as_ptr(), 0);
    while !idat.is_null() {
        datalen = datalen.wrapping_add(chunk_byte_length(idat) as i32);
        idat = chunk(&mut png, b"IDAT".as_ptr(), 0);
    }
    png.p = first;
    data = malloc(datalen as usize).cast();
    let mut offset = 0i32;
    idat = find(&mut png, b"IDAT".as_ptr(), 0);
    while !idat.is_null() {
        let len = chunk_byte_length(idat);
        ptr::copy_nonoverlapping(idat, data.offset(offset as isize), len as usize);
        offset = offset.wrapping_add(len as i32);
        idat = chunk(&mut png, b"IDAT".as_ptr(), 0);
    }
    if data.is_null() || datalen < 6 {
        return fail(&mut img, data, ERR_ZLIB_STRUCTURE);
    }
    if (*data & 0x0f) != 0x08 {
        return fail(&mut img, data, ERR_ZLIB_METHOD);
    }
    if (*data & 0xf0) > 0x70 {
        return fail(&mut img, data, ERR_WINDOW);
    }
    if (*data.add(1) & 0x20) != 0 {
        return fail(&mut img, data, ERR_DICTIONARY);
    }
    if out_size(&img, 4) < 1 {
        return fail(&mut img, data, ERR_IMAGE_SIZE);
    }
    if out_size(&img, bpp) < 1 {
        return fail(&mut img, data, ERR_IMAGE_SIZE);
    }
    let out = img
        .pix
        .cast::<u8>()
        .offset((out_size(&img, 4) - out_size(&img, bpp)) as isize);
    if cp_inflate(data.add(2).cast(), datalen - 6, out.cast(), pix_bytes) == 0 {
        return fail(&mut img, data, ERR_DEFLATE);
    }
    if unfilter(img.w, img.h, bpp, out) == 0 {
        return fail(&mut img, data, ERR_FILTER_BYTE);
    }
    if color_type == 3 {
        if plte.is_null() {
            return fail(&mut img, data, ERR_PALETTE);
        }
        let trns_len = if trns.is_null() {
            0
        } else {
            chunk_byte_length(trns)
        };
        depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
    } else {
        convert(bpp, img.w, img.h, out, img.pix);
    }
    free(data.cast());
    img
}
