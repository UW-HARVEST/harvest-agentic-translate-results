#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_uchar, c_void};
use std::mem::size_of;
use std::ptr;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

impl Default for cp_image_t {
    fn default() -> Self {
        Self {
            w: 0,
            h: 0,
            pix: ptr::null_mut(),
        }
    }
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

const ERR_STORED_COMPLEMENT: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
const ERR_STORED_PAST_END: &[u8] = b"Stored block extends beyond end of input stream.\0";
const ERR_OUT_SYMBOL: &[u8] =
    b"Attempted to overwrite out buffer while outputting a symbol.\0";
const ERR_BACK_DISTANCE: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
const ERR_OUT_STRING: &[u8] =
    b"Attempted to overwrite out buffer while outputting a string.\0";
const ERR_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";
const ERR_SIGNATURE: &[u8] = b"incorrect file signature (is this a png file?)\0";
const ERR_IHDR: &[u8] = b"unable to find IHDR chunk\0";
const ERR_BIT_DEPTH: &[u8] = b"only bit-depth of 8 is supported\0";
const ERR_COLOR_TYPE: &[u8] = b"unknown color type\0";
const ERR_WIDTH: &[u8] = b"invalid IHDR chunk found, image width was less than 1\0";
const ERR_HEIGHT: &[u8] = b"invalid IHDR chunk found, image height was less than 1\0";
const ERR_TOO_LARGE: &[u8] = b"image too large\0";
const ERR_ALLOC_IMAGE: &[u8] = b"unable to allocate raw image space\0";
const ERR_COMPRESSION: &[u8] = b"only standard compression DEFLATE is supported\0";
const ERR_FILTER: &[u8] = b"only standard adaptive filtering is supported\0";
const ERR_INTERLACE: &[u8] = b"interlacing is not supported\0";
const ERR_ZLIB_STRUCTURE: &[u8] = b"corrupt zlib structure in DEFLATE stream\0";
const ERR_ZLIB_METHOD: &[u8] = b"only zlib compression method (RFC 1950) is supported\0";
const ERR_ZLIB_WINDOW: &[u8] = b"innapropriate window size detected\0";
const ERR_ZLIB_DICT: &[u8] = b"preset dictionary is present and not supported\0";
const ERR_INVALID_IMAGE_SIZE: &[u8] = b"invalid image size found\0";
const ERR_DEFLATE: &[u8] = b"DEFLATE algorithm failed\0";
const ERR_FILTER_BYTE: &[u8] = b"invalid filter byte found\0";
const ERR_INDEXED_PLTE: &[u8] = b"color type of indexed requires a PLTE chunk\0";

const CP_FIXED_TABLE: [u8; 320] = [
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
const CP_PERMUTATION_ORDER: [u8; 19] =
    [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
const CP_LEN_EXTRA_BITS: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];
const CP_LEN_BASE: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67,
    83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];
const CP_DIST_EXTRA_BITS: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];
const CP_DIST_BASE: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

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

impl Default for CpState {
    fn default() -> Self {
        Self {
            bits: 0,
            count: 0,
            words: ptr::null(),
            word_count: 0,
            word_index: 0,
            bits_left: 0,
            final_word_available: 0,
            final_word: 0,
            out: ptr::null_mut(),
            out_end: ptr::null_mut(),
            begin: ptr::null_mut(),
            lookup: [0; 1 << 9],
            lit: [0; 288],
            dst: [0; 32],
            len: [0; 19],
            nlit: 0,
            ndst: 0,
            nlen: 0,
        }
    }
}

#[inline]
fn set_error(msg: &'static [u8]) {
    unsafe {
        cp_error_reason = msg.as_ptr().cast();
    }
}

#[inline]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

#[inline]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

#[inline]
fn cp_would_overflow(s: &CpState, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

#[inline]
fn cp_ptr(s: &CpState) -> *const u8 {
    assert_eq!(s.bits_left & 7, 0);
    unsafe { (s.words.add(s.word_index as usize) as *const u8).sub((s.count / 8) as usize) }
}

fn cp_peak_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { *s.words.add(s.word_index as usize) };
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            assert!(s.word_index <= s.word_count);
        } else if s.final_word_available != 0 {
            let word = s.final_word;
            s.bits |= (word as u64) << s.count;
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(s.count >= num_bits_to_read);
    let mask = ((1u64 << num_bits_to_read) - 1) as u32;
    let bits = (s.bits as u32) & mask;
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!(s.bits_left > 0);
    assert!(s.count <= 64);
    assert!(!cp_would_overflow(s, num_bits_to_read));
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

#[inline]
fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8)
}

fn cp_build(
    mut lookup: Option<&mut [u16; 1 << 9]>,
    tree: &mut [u32],
    lens: &[u8],
    sym_count: c_int,
) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    for n in 0..(sym_count as usize) {
        counts[lens[n] as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if let Some(table) = lookup.as_deref_mut() {
        table.fill(0);
    }
    for i in 0..(sym_count as usize) {
        let len = lens[i] as usize;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(table) = lookup.as_deref_mut() {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        table[j] = ((len as u16) << 9) | (i as u16);
                        j += 1 << len;
                    }
                }
            }
        }
    }
    first[15]
}

fn cp_stored(s: &mut CpState) -> bool {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        set_error(ERR_STORED_COMPLEMENT);
        return false;
    }
    if s.bits_left / 8 > len as c_int {
        set_error(ERR_STORED_PAST_END);
        return false;
    }
    let p = cp_ptr(s);
    unsafe {
        ptr::copy_nonoverlapping(p, s.out, len as usize);
        s.out = s.out.add(len as usize);
    }
    true
}

fn cp_fixed(s: &mut CpState) -> bool {
    let lookup = &mut s.lookup;
    let lit = &mut s.lit;
    s.nlit = cp_build(Some(lookup), lit, &CP_FIXED_TABLE, 288) as u32;
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32) as u32;
    true
}

fn cp_decode(s: &mut CpState, tree: *const u32, hi: u32) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0i32;
    let mut hi = hi as i32;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < unsafe { *tree.add(guess as usize) } {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = unsafe { *tree.add((lo - 1) as usize) };
    let len = 32 - (key & 0xF);
    assert_eq!(search >> len, key >> len);
    let _ = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

fn cp_dynamic(s: &mut CpState) -> bool {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..(nlen as usize) {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    s.nlen = cp_build(None, &mut s.len, &lenlens, 19) as u32;
    let mut lens = [0u8; 320];
    let mut n = 0i32;
    while n < nlit + ndst {
        let sym = cp_decode(s, s.len.as_ptr(), s.nlen);
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as i32;
                while i != 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as i32;
                while i != 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as i32;
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
    let lookup = &mut s.lookup;
    let lit = &mut s.lit;
    s.nlit = cp_build(Some(lookup), lit, &lens, nlit) as u32;
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit as usize..], ndst) as u32;
    true
}

fn cp_block(s: &mut CpState) -> bool {
    loop {
        let symbol = cp_decode(s, s.lit.as_ptr(), s.nlit);
        if symbol < 256 {
            unsafe {
                if s.out.add(1) > s.out_end {
                    set_error(ERR_OUT_SYMBOL);
                    return false;
                }
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length =
                cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol as usize] as c_int) as i32
                    + CP_LEN_BASE[symbol as usize] as i32;
            let distance_symbol = cp_decode(s, s.dst.as_ptr(), s.ndst);
            let backwards_distance =
                cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol as usize] as c_int) as i32
                    + CP_DIST_BASE[distance_symbol as usize] as i32;
            unsafe {
                if s.out.sub(backwards_distance as usize) < s.begin {
                    set_error(ERR_BACK_DISTANCE);
                    return false;
                }
                if s.out.add(length as usize) > s.out_end {
                    set_error(ERR_OUT_STRING);
                    return false;
                }
                let src = s.out.sub(backwards_distance as usize);
                let mut dst = s.out;
                s.out = s.out.add(length as usize);
                if backwards_distance == 1 {
                    libc::memset(dst.cast::<c_void>(), *src as i32, length as usize);
                } else {
                    let mut remaining = length;
                    let mut src_cur = src;
                    while remaining != 0 {
                        *dst = *src_cur;
                        dst = dst.add(1);
                        src_cur = src_cur.add(1);
                        remaining -= 1;
                    }
                }
            }
        } else {
            break;
        }
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut s = CpState::default();
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;
    let input_addr = input as usize;
    let first_bytes = (((input_addr + 3) & !3usize).wrapping_sub(input_addr)) as c_int;
    s.words = (input as *mut u8).add(first_bytes as usize) as *const u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..(first_bytes as usize) {
        s.bits |= ((*((input as *mut u8).add(i)) as u64) << (i * 8)) as u64;
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..(last_bytes as usize) {
        s.final_word |= (*((input as *mut u8).add((in_bytes - last_bytes) as usize + i)) as u32)
            << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out as *mut u8;
    s.out_end = s.out.add(out_bytes as usize);
    s.begin = out as *mut u8;
    let mut bfinal;
    loop {
        bfinal = cp_read_bits(&mut s, 1);
        let btype = cp_read_bits(&mut s, 2);
        match btype {
            0 => {
                if !cp_stored(&mut s) {
                    return 0;
                }
            }
            1 => {
                cp_fixed(&mut s);
                if !cp_block(&mut s) {
                    return 0;
                }
            }
            2 => {
                cp_dynamic(&mut s);
                if !cp_block(&mut s) {
                    return 0;
                }
            }
            3 => {
                set_error(ERR_UNKNOWN_BLOCK);
                return 0;
            }
            _ => unreachable!(),
        }
        if bfinal != 0 {
            break;
        }
    }
    1
}

#[inline]
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

struct CpRawPng {
    p: *const u8,
    end: *const u8,
}

fn cp_make32(s: *const u8) -> u32 {
    unsafe {
        ((*s.add(0) as u32) << 24)
            | ((*s.add(1) as u32) << 16)
            | ((*s.add(2) as u32) << 8)
            | (*s.add(3) as u32)
    }
}

fn chunk_name_eq(chunk: *const u8, name: &[u8; 4]) -> bool {
    unsafe {
        *chunk.add(0) == name[0]
            && *chunk.add(1) == name[1]
            && *chunk.add(2) == name[2]
            && *chunk.add(3) == name[3]
    }
}

fn cp_chunk(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    let len = cp_make32(png.p);
    let start = png.p;
    if chunk_name_eq(unsafe { start.add(4) }, chunk) && len >= minlen {
        let offset = len.wrapping_add(12);
        if unsafe { png.p.add(offset as usize) } <= png.end {
            png.p = unsafe { png.p.add(offset as usize) };
            return unsafe { start.add(8) };
        }
    }
    ptr::null()
}

fn cp_find(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    while png.p < png.end {
        let len = cp_make32(png.p);
        let start = png.p;
        png.p = unsafe { png.p.add(len.wrapping_add(12) as usize) };
        if chunk_name_eq(unsafe { start.add(4) }, chunk) && len >= minlen && png.p <= png.end {
            return unsafe { start.add(8) };
        }
    }
    ptr::null()
}

fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> bool {
    let len = w * bpp;
    let mut prev;
    let mut x;
    let mut raw = raw;
    if h > 0 {
        unsafe {
            match *raw {
                0 => {}
                1 => {
                    raw = raw.add(1);
                    x = bpp;
                    while x < len {
                        *raw.add(x as usize) = (*raw.add(x as usize))
                            .wrapping_add(*raw.add((x - bpp) as usize));
                        x += 1;
                    }
                    raw = raw.sub(1);
                }
                2 => {}
                3 => {
                    raw = raw.add(1);
                    x = bpp;
                    while x < len {
                        *raw.add(x as usize) = (*raw.add(x as usize))
                            .wrapping_add(*raw.add((x - bpp) as usize) / 2);
                        x += 1;
                    }
                    raw = raw.sub(1);
                }
                4 => {
                    raw = raw.add(1);
                    x = bpp;
                    while x < len {
                        *raw.add(x as usize) = (*raw.add(x as usize))
                            .wrapping_add(cp_paeth(*raw.add((x - bpp) as usize), 0, 0));
                        x += 1;
                    }
                    raw = raw.sub(1);
                }
                _ => return false,
            }
        }
    }
    prev = unsafe { raw.add(1) };
    raw = unsafe { raw.add(1 + len as usize) };
    for _y in 1..h {
        unsafe {
            match *raw {
                0 => {}
                1 => {
                    raw = raw.add(1);
                    x = 0;
                    while x < bpp {
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(0);
                        x += 1;
                    }
                    while x < len {
                        *raw.add(x as usize) = (*raw.add(x as usize))
                            .wrapping_add(*raw.add((x - bpp) as usize));
                        x += 1;
                    }
                    raw = raw.sub(1);
                }
                2 => {
                    raw = raw.add(1);
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
                    raw = raw.sub(1);
                }
                3 => {
                    raw = raw.add(1);
                    x = 0;
                    while x < bpp {
                        *raw.add(x as usize) =
                            (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize) / 2);
                        x += 1;
                    }
                    while x < len {
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(
                            ((*raw.add((x - bpp) as usize) as u16 + *prev.add(x as usize) as u16)
                                / 2) as u8,
                        );
                        x += 1;
                    }
                    raw = raw.sub(1);
                }
                4 => {
                    raw = raw.add(1);
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
                    raw = raw.sub(1);
                }
                _ => return false,
            }
            prev = raw.add(1);
            raw = raw.add(1 + len as usize);
        }
    }
    true
}

fn cp_convert(bpp: c_int, w: c_int, h: c_int, mut src: *mut u8, mut dst: *mut cp_pixel_t) {
    for _y in 0..h {
        unsafe {
            src = src.add(1);
            for _x in 0..w {
                *dst = match bpp {
                    1 => cp_make_pixel(*src, *src, *src),
                    2 => cp_make_pixel_a(*src, *src, *src, *src.add(1)),
                    3 => cp_make_pixel(*src, *src.add(1), *src.add(2)),
                    4 => cp_make_pixel_a(*src, *src.add(1), *src.add(2), *src.add(3)),
                    _ => unreachable!(),
                };
                dst = dst.add(1);
                src = src.add(bpp as usize);
            }
        }
    }
}

fn cp_get_alpha_for_indexed_image(index: c_int, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() || index as u32 >= trns_len {
        255
    } else {
        unsafe { *trns.add(index as usize) }
    }
}

fn cp_depalette(
    w: c_int,
    h: c_int,
    mut src: *mut u8,
    mut dst: *mut cp_pixel_t,
    plte: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    for _y in 0..h {
        unsafe {
            src = src.add(1);
            for _x in 0..w {
                let c = *src as usize;
                let r = *plte.add(c * 3);
                let g = *plte.add(c * 3 + 1);
                let b = *plte.add(c * 3 + 2);
                let a = cp_get_alpha_for_indexed_image(c as c_int, trns, trns_len);
                *dst = cp_make_pixel_a(r, g, b, a);
                dst = dst.add(1);
                src = src.add(1);
            }
        }
    }
}

#[inline]
fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    cp_make32(unsafe { chunk.sub(8) })
}

#[inline]
fn cp_out_size(img: &cp_image_t, bpp: c_int) -> c_int {
    (img.w + 1) * img.h * bpp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(
    png_data: *const c_uchar,
    png_length: c_int,
) -> cp_image_t {
    const SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    let mut img = cp_image_t::default();
    let mut data: *mut u8 = ptr::null_mut();
    let mut png = CpRawPng {
        p: png_data,
        end: png_data.add(png_length as usize),
    };

    if ptr::read_unaligned(png.p.cast::<[u8; 8]>()) != *SIG {
        set_error(ERR_SIGNATURE);
        return img;
    }
    png.p = png.p.add(8);

    let ihdr = cp_chunk(&mut png, b"IHDR", 13);
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

    let w = cp_make32(ihdr) as c_int + 1;
    let h = cp_make32(ihdr.add(4)) as c_int;
    if w < 1 {
        set_error(ERR_WIDTH);
        return img;
    }
    if h < 1 {
        set_error(ERR_HEIGHT);
        return img;
    }
    if (w as i64) * (h as i64) * (size_of::<cp_pixel_t>() as i64) >= (c_int::MAX as i64) {
        set_error(ERR_TOO_LARGE);
        return img;
    }

    let pix_bytes = w * h * size_of::<cp_pixel_t>() as c_int;
    img.w = w - 1;
    img.h = h;
    img.pix = libc::malloc(pix_bytes as usize).cast::<cp_pixel_t>();
    if img.pix.is_null() {
        set_error(ERR_ALLOC_IMAGE);
        return img;
    }

    let compression = *ihdr.add(10) as c_int;
    let filter = *ihdr.add(11) as c_int;
    let interlace = *ihdr.add(12) as c_int;
    if compression != 0 {
        set_error(ERR_COMPRESSION);
        goto_err(&mut data, &mut img);
        return img;
    }
    if filter != 0 {
        set_error(ERR_FILTER);
        goto_err(&mut data, &mut img);
        return img;
    }
    if interlace != 0 {
        set_error(ERR_INTERLACE);
        goto_err(&mut data, &mut img);
        return img;
    }

    let mut first = png.p;
    let plte = cp_find(&mut png, b"PLTE", 0);
    if plte.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    let trns = cp_find(&mut png, b"tRNS", 0);
    if trns.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }

    let mut datalen = 0i32;
    let mut idat = cp_find(&mut png, b"IDAT", 0);
    while !idat.is_null() {
        let len = cp_get_chunk_byte_length(idat) as c_int;
        datalen += len;
        idat = cp_chunk(&mut png, b"IDAT", 0);
    }

    png.p = first;
    data = libc::malloc(datalen as usize).cast::<u8>();
    let mut offset = 0i32;
    let mut idat = cp_find(&mut png, b"IDAT", 0);
    while !idat.is_null() {
        let len = cp_get_chunk_byte_length(idat) as usize;
        ptr::copy_nonoverlapping(idat, data.add(offset as usize), len);
        offset += len as i32;
        idat = cp_chunk(&mut png, b"IDAT", 0);
    }

    if data.is_null() || datalen < 6 {
        set_error(ERR_ZLIB_STRUCTURE);
        goto_err(&mut data, &mut img);
        return img;
    }
    if (*data & 0x0f) != 0x08 {
        set_error(ERR_ZLIB_METHOD);
        goto_err(&mut data, &mut img);
        return img;
    }
    if (*data & 0xf0) > 0x70 {
        set_error(ERR_ZLIB_WINDOW);
        goto_err(&mut data, &mut img);
        return img;
    }
    if (*data.add(1) & 0x20) != 0 {
        set_error(ERR_ZLIB_DICT);
        goto_err(&mut data, &mut img);
        return img;
    }
    if cp_out_size(&img, 4) < 1 {
        set_error(ERR_INVALID_IMAGE_SIZE);
        goto_err(&mut data, &mut img);
        return img;
    }
    if cp_out_size(&img, bpp) < 1 {
        set_error(ERR_INVALID_IMAGE_SIZE);
        goto_err(&mut data, &mut img);
        return img;
    }

    let out = (img.pix as *mut u8).add((cp_out_size(&img, 4) - cp_out_size(&img, bpp)) as usize);
    if cp_inflate(data.add(2).cast(), datalen - 6, out.cast(), pix_bytes) == 0 {
        set_error(ERR_DEFLATE);
        goto_err(&mut data, &mut img);
        return img;
    }
    if !cp_unfilter(img.w, img.h, bpp, out) {
        set_error(ERR_FILTER_BYTE);
        goto_err(&mut data, &mut img);
        return img;
    }
    if color_type == 3 {
        if plte.is_null() {
            set_error(ERR_INDEXED_PLTE);
            goto_err(&mut data, &mut img);
            return img;
        }
        let trns_len = if trns.is_null() { 0 } else { cp_get_chunk_byte_length(trns) };
        cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
    } else {
        cp_convert(bpp, img.w, img.h, out, img.pix);
    }
    libc::free(data.cast());
    img
}

unsafe fn goto_err(data: &mut *mut u8, img: &mut cp_image_t) {
    libc::free((*data).cast());
    libc::free(img.pix.cast());
    img.pix = ptr::null_mut();
}
