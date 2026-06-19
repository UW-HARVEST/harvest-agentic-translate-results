#![allow(dead_code, non_camel_case_types, non_snake_case)]

use std::ffi::{c_char, c_int, c_void};
use std::mem::zeroed;
use std::ptr;

const LOOKUP_BITS: usize = 9;
const LOOKUP_SIZE: usize = 1 << LOOKUP_BITS;

#[repr(C)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
struct cp_image_t {
    w: c_int,
    h: c_int,
    pix: *mut cp_pixel_t,
}

#[repr(C)]
struct cp_state_t {
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
    lookup: [u16; LOOKUP_SIZE],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

#[repr(C)]
struct cp_raw_png_t {
    p: *const u8,
    end: *const u8,
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

static ERR_STORED_LEN: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
static ERR_STORED_OVERFLOW: &[u8] = b"Stored block extends beyond end of input stream.\0";
static ERR_SYMBOL_OVERFLOW: &[u8] =
    b"Attempted to overwrite out buffer while outputting a symbol.\0";
static ERR_DISTANCE: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
static ERR_STRING_OVERFLOW: &[u8] =
    b"Attempted to overwrite out buffer while outputting a string.\0";
static ERR_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";

static CP_FIXED_TABLE: [u8; 320] = [
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
static CP_PERMUTATION_ORDER: [u8; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
static CP_LEN_EXTRA_BITS: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];
static CP_LEN_BASE: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67,
    83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];
static CP_DIST_EXTRA_BITS: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11,
    11, 12, 12, 13, 13, 0, 0,
];
static CP_DIST_BASE: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769,
    1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

#[inline]
fn set_error(msg: &'static [u8]) {
    unsafe {
        cp_error_reason = msg.as_ptr() as *const c_char;
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
fn ptr_addr<T>(p: *const T) -> usize {
    p as usize
}

fn cp_would_overflow(s: &cp_state_t, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

fn cp_ptr(s: &cp_state_t) -> *const u8 {
    assert_eq!(s.bits_left & 7, 0);
    unsafe { (s.words.add(s.word_index as usize) as *const u8).sub((s.count / 8) as usize) }
}

fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u64 {
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

fn cp_consume_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    assert!(s.count >= num_bits_to_read);
    let bits = (s.bits & ((1u64 << num_bits_to_read) - 1)) as u32;
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!(s.bits_left > 0);
    assert!(s.count <= 64);
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

fn cp_build(
    mut lookup: Option<&mut [u16; LOOKUP_SIZE]>,
    tree: &mut [u32],
    lens: &[u8],
    sym_count: usize,
) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];

    for n in 0..sym_count {
        counts[lens[n] as usize] += 1;
    }

    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;

    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }

    if let Some(table) = lookup.as_mut() {
        table.fill(0);
    }

    for i in 0..sym_count {
        let len = lens[i] as usize;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | len as u32;
            if let Some(table) = lookup.as_mut() {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < LOOKUP_SIZE {
                        table[j] = ((len as u16) << 9) | i as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    }

    first[15]
}

fn cp_stored(s: &mut cp_state_t) -> c_int {
    cp_read_bits(s, s.count & 7);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;

    if LEN != !NLEN {
        set_error(ERR_STORED_LEN);
        return 0;
    }

    if s.bits_left / 8 > LEN as c_int {
        set_error(ERR_STORED_OVERFLOW);
        return 0;
    }

    let p = cp_ptr(s);
    unsafe {
        ptr::copy_nonoverlapping(p, s.out, LEN as usize);
        s.out = s.out.add(LEN as usize);
    }
    1
}

fn cp_fixed(s: &mut cp_state_t) -> c_int {
    s.nlit = cp_build(Some(&mut s.lookup), &mut s.lit, &CP_FIXED_TABLE[..288], 288) as u32;
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32) as u32;
    1
}

fn cp_decode(s: &mut cp_state_t, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0;
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
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

fn cp_dynamic(s: &mut cp_state_t) -> c_int {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as c_int;
    let ndst = 1 + cp_read_bits(s, 5) as c_int;
    let nlen = 4 + cp_read_bits(s, 4) as c_int;

    for i in 0..nlen as usize {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }

    s.nlen = cp_build(None, &mut s.len, &lenlens, 19) as u32;

    let mut lens = [0u8; 320];
    let mut n = 0;
    while n < (nlit + ndst) as usize {
        let sym = cp_decode(s, s.len.as_ptr(), s.nlen as c_int);
        match sym {
            16 => {
                let repeat = 3 + cp_read_bits(s, 2) as usize;
                for _ in 0..repeat {
                    lens[n] = lens[n - 1];
                    n += 1;
                }
            }
            17 => {
                let repeat = 3 + cp_read_bits(s, 3) as usize;
                for _ in 0..repeat {
                    lens[n] = 0;
                    n += 1;
                }
            }
            18 => {
                let repeat = 11 + cp_read_bits(s, 7) as usize;
                for _ in 0..repeat {
                    lens[n] = 0;
                    n += 1;
                }
            }
            _ => {
                lens[n] = sym as u8;
                n += 1;
            }
        }
    }

    s.nlit = cp_build(Some(&mut s.lookup), &mut s.lit, &lens, nlit as usize) as u32;
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit as usize..], ndst as usize) as u32;
    1
}

fn cp_block(s: &mut cp_state_t) -> c_int {
    loop {
        let symbol = cp_decode(s, s.lit.as_ptr(), s.nlit as c_int);
        if symbol < 256 {
            if ptr_addr(unsafe { s.out.add(1) }) > ptr_addr(s.out_end) {
                set_error(ERR_SYMBOL_OVERFLOW);
                return 0;
            }
            unsafe {
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let symbol = (symbol - 257) as usize;
            let length = cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol] as c_int) + CP_LEN_BASE[symbol];
            let distance_symbol = cp_decode(s, s.dst.as_ptr(), s.ndst as c_int) as usize;
            let backwards_distance = cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol] as c_int)
                + CP_DIST_BASE[distance_symbol];

            if ptr_addr(s.out) < ptr_addr(s.begin).wrapping_add(backwards_distance as usize) {
                set_error(ERR_DISTANCE);
                return 0;
            }

            if ptr_addr(s.out).wrapping_add(length as usize) > ptr_addr(s.out_end) {
                set_error(ERR_STRING_OVERFLOW);
                return 0;
            }

            unsafe {
                let mut src = s.out.sub(backwards_distance as usize);
                let mut dst = s.out;
                s.out = s.out.add(length as usize);
                match backwards_distance {
                    1 => ptr::write_bytes(dst, *src, length as usize),
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
            }
        } else {
            break;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    r#in: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut s: Box<cp_state_t> = Box::new(zeroed());
    let in_ptr = r#in as *const u8;
    let in_addr = in_ptr as usize;

    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;

    let first_bytes = (((in_addr + 3) & !3usize) - in_addr) as c_int;
    s.words = in_ptr.add(first_bytes as usize) as *const u32;
    s.word_count = (in_bytes - first_bytes) / 4;

    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes as usize {
        s.bits |= ((*in_ptr.add(i)) as u64) << (i * 8);
    }

    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes as usize {
        s.final_word |= ((*in_ptr.add(in_bytes as usize - last_bytes as usize + i)) as u32) << (i * 8);
    }

    s.count = first_bytes * 8;
    s.out = out as *mut u8;
    s.out_end = s.out.add(out_bytes as usize);
    s.begin = out as *mut u8;

    let mut _count = 0;
    let mut bfinal;

    loop {
        bfinal = cp_read_bits(&mut s, 1);
        let btype = cp_read_bits(&mut s, 2);
        match btype {
            0 => {
                if cp_stored(&mut s) == 0 {
                    return 0;
                }
            }
            1 => {
                cp_fixed(&mut s);
                if cp_block(&mut s) == 0 {
                    return 0;
                }
            }
            2 => {
                cp_dynamic(&mut s);
                if cp_block(&mut s) == 0 {
                    return 0;
                }
            }
            3 => {
                set_error(ERR_UNKNOWN_BLOCK);
                return 0;
            }
            _ => unreachable!(),
        }
        _count += 1;
        if bfinal != 0 {
            break;
        }
    }

    1
}

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
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

fn cp_make32(s: *const u8) -> u32 {
    unsafe {
        ((*s.add(0) as u32) << 24)
            | ((*s.add(1) as u32) << 16)
            | ((*s.add(2) as u32) << 8)
            | (*s.add(3) as u32)
    }
}

fn chunk_eq(a: *const u8, b: *const c_char) -> bool {
    unsafe {
        *a.add(0) == *b.add(0) as u8
            && *a.add(1) == *b.add(1) as u8
            && *a.add(2) == *b.add(2) as u8
            && *a.add(3) == *b.add(3) as u8
    }
}

fn cp_chunk(png: &mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    let len = cp_make32(png.p);
    let start = png.p;
    if chunk_eq(unsafe { start.add(4) }, chunk) && len >= minlen {
        let offset = len as usize + 12;
        if unsafe { png.p.add(offset) } <= png.end {
            png.p = unsafe { png.p.add(offset) };
            return unsafe { start.add(8) };
        }
    }
    ptr::null()
}

fn cp_find(png: &mut cp_raw_png_t, chunk: *const c_char, minlen: u32) -> *const u8 {
    while png.p < png.end {
        let len = cp_make32(png.p);
        let start = png.p;
        png.p = unsafe { png.p.add(len as usize + 12) };
        if chunk_eq(unsafe { start.add(4) }, chunk) && len >= minlen && png.p <= png.end {
            return unsafe { start.add(8) };
        }
    }
    ptr::null()
}

fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len = w * bpp;
    let mut x;
    let mut raw = raw;
    let mut prev;

    if h > 0 {
        unsafe {
            match *raw {
                0 => {}
                1 => {
                    raw = raw.add(1);
                    x = bpp;
                    while x < len {
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                        x += 1;
                    }
                    raw = raw.sub(1);
                }
                2 => {}
                3 => {
                    raw = raw.add(1);
                    x = bpp;
                    while x < len {
                        *raw.add(x as usize) =
                            (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize) / 2);
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
                _ => return 0,
            }
        }
    }

    prev = unsafe { raw.add(1) };
    raw = unsafe { raw.add((len + 1) as usize) };

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
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                        x += 1;
                    }
                    raw = raw.sub(1);
                }
                2 => {
                    raw = raw.add(1);
                    x = 0;
                    while x < bpp {
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                        x += 1;
                    }
                    while x < len {
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                        x += 1;
                    }
                    raw = raw.sub(1);
                }
                3 => {
                    raw = raw.add(1);
                    x = 0;
                    while x < bpp {
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize) / 2);
                        x += 1;
                    }
                    while x < len {
                        *raw.add(x as usize) = (*raw.add(x as usize))
                            .wrapping_add(((*raw.add((x - bpp) as usize) as u16 + *prev.add(x as usize) as u16) / 2) as u8);
                        x += 1;
                    }
                    raw = raw.sub(1);
                }
                4 => {
                    raw = raw.add(1);
                    x = 0;
                    while x < bpp {
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
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
                _ => return 0,
            }
            prev = raw.add(1);
            raw = raw.add((len + 1) as usize);
        }
    }

    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn convert_pix(
    bpp: c_int,
    w: c_int,
    h: c_int,
    src: *mut u8,
    dst: *mut cp_pixel_t,
) {
    let mut src = src;
    let mut dst = dst;

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
            dst = dst.add(1);
            src = src.add(bpp as usize);
        }
    }
}
