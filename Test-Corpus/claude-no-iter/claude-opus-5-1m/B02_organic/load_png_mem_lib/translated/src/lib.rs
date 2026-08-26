#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

use std::ffi::c_int;
use std::os::raw::c_char;
use std::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut std::ffi::c_void;
    fn free(ptr: *mut std::ffi::c_void);
}

#[repr(C)]
#[derive(Copy, Clone)]
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

#[inline]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

#[inline]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

static CP_FIXED_TABLE: [u8; 288 + 32] = [
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

static CP_PERMUTATION_ORDER: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

static CP_LEN_EXTRA_BITS: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];

static CP_LEN_BASE: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67,
    83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

static CP_DIST_EXTRA_BITS: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

static CP_DIST_BASE: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

struct CpState {
    bits: u64,
    count: i32,
    in_buf: *const u8,
    first_bytes: i32,
    word_count: i32,
    word_index: i32,
    bits_left: i32,
    final_word_available: i32,
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

impl CpState {
    fn new() -> Self {
        Self {
            bits: 0,
            count: 0,
            in_buf: ptr::null(),
            first_bytes: 0,
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

fn cp_ptr(s: &CpState) -> *const u8 {
    // (char *)(s->words + s->word_index) - (s->count / 8)
    // = in_buf + first_bytes + word_index*4 - count/8
    unsafe {
        s.in_buf
            .offset((s.first_bytes + s.word_index * 4 - s.count / 8) as isize)
    }
}

fn cp_peak_bits(s: &mut CpState, _num_bits_to_read: i32) -> u64 {
    if s.count < _num_bits_to_read {
        if s.word_index < s.word_count {
            let offset = (s.first_bytes + s.word_index * 4) as usize;
            let word = unsafe {
                let p = s.in_buf.add(offset);
                u32::from_le_bytes([*p, *p.add(1), *p.add(2), *p.add(3)])
            };
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
        } else if s.final_word_available != 0 {
            let word = s.final_word;
            s.bits |= (word as u64) << s.count;
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    let mask = if num_bits_to_read == 64 {
        !0u64
    } else {
        (1u64 << num_bits_to_read) - 1
    };
    let bits = (s.bits & mask) as u32;
    if num_bits_to_read >= 64 {
        s.bits = 0;
    } else {
        s.bits >>= num_bits_to_read;
    }
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

fn cp_read_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(a: u32) -> u32 {
    let mut a = a;
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

fn cp_build(
    mut lookup: Option<&mut [u16; 1 << 9]>,
    tree: &mut [u32],
    lens: &[u8],
    sym_count: usize,
) -> i32 {
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
    if let Some(lk) = lookup.as_deref_mut() {
        for v in lk.iter_mut() {
            *v = 0;
        }
    }
    for i in 0..sym_count {
        let len = lens[i] as i32;
        if len != 0 {
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as usize;
            first[len as usize] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(lk) = lookup.as_deref_mut() {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        lk[j] = ((len as u16) << 9) | (i as u16);
                        j += 1usize << len;
                    }
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut CpState) -> i32 {
    cp_read_bits(s, s.count & 7);
    let len_val = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len_val != !nlen {
        cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    if !(s.bits_left / 8 <= len_val as i32) {
        cp_error_reason =
            b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p, s.out, len_val as usize);
    s.out = s.out.add(len_val as usize);
    1
}

fn cp_fixed(s: &mut CpState) -> i32 {
    s.nlit = cp_build(Some(&mut s.lookup), &mut s.lit, &CP_FIXED_TABLE, 288) as u32;
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32) as u32;
    1
}

fn cp_decode(s: &mut CpState, tree: *const u32, hi: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFFu32;
    let mut hi = hi;
    let mut lo = 0i32;
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
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

fn cp_dynamic(s: &mut CpState) -> i32 {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen as usize {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    s.nlen = cp_build(None, &mut s.len, &lenlens, 19) as u32;
    let mut lens = [0u8; 288 + 32];
    let mut n = 0i32;
    while n < nlit + ndst {
        let len_ptr = s.len.as_ptr();
        let nlen_count = s.nlen as i32;
        let sym = cp_decode(s, len_ptr, nlen_count);
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
    s.nlit = cp_build(Some(&mut s.lookup), &mut s.lit, &lens, nlit as usize) as u32;
    s.ndst = cp_build(
        None,
        &mut s.dst,
        &lens[nlit as usize..],
        ndst as usize,
    ) as u32;
    1
}

unsafe fn cp_block(s: &mut CpState) -> i32 {
    loop {
        let lit_ptr = s.lit.as_ptr();
        let nlit_count = s.nlit as i32;
        let symbol = cp_decode(s, lit_ptr, nlit_count);
        if symbol < 256 {
            if !(s.out as usize + 1 <= s.out_end as usize) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr()
                        as *const c_char;
                return 0;
            }
            *s.out = symbol as u8;
            s.out = s.out.add(1);
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = (cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol as usize] as i32)
                + CP_LEN_BASE[symbol as usize]) as i32;
            let dst_ptr = s.dst.as_ptr();
            let ndst_count = s.ndst as i32;
            let distance_symbol = cp_decode(s, dst_ptr, ndst_count);
            let backwards_distance = (cp_read_bits(
                s,
                CP_DIST_EXTRA_BITS[distance_symbol as usize] as i32,
            ) + CP_DIST_BASE[distance_symbol as usize]) as i32;
            if !((s.out as isize - backwards_distance as isize) >= s.begin as isize) {
                cp_error_reason =
                    b"Attempted to write before out buffer (invalid backwards distance).\0"
                        .as_ptr() as *const c_char;
                return 0;
            }
            if !(s.out as usize + length as usize <= s.out_end as usize) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr()
                        as *const c_char;
                return 0;
            }
            let mut src = s.out.offset(-(backwards_distance as isize));
            let mut dst = s.out;
            s.out = s.out.add(length as usize);
            if backwards_distance == 1 {
                ptr::write_bytes(dst, *src, length as usize);
            } else {
                let mut len = length;
                while len > 0 {
                    *dst = *src;
                    dst = dst.add(1);
                    src = src.add(1);
                    len -= 1;
                }
            }
        } else {
            break;
        }
    }
    1
}

unsafe fn cp_inflate(in_data: *const u8, in_bytes: i32, out_data: *mut u8, out_bytes: i32) -> i32 {
    let mut s = Box::new(CpState::new());
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;
    s.in_buf = in_data;
    let first_bytes = (((in_data as usize + 3) & !3) - in_data as usize) as i32;
    s.first_bytes = first_bytes;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        let byte = *in_data.add(i as usize);
        s.bits |= (byte as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        let byte = *in_data.add((in_bytes - last_bytes + i) as usize);
        s.final_word |= (byte as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out_data;
    s.out_end = out_data.add(out_bytes as usize);
    s.begin = out_data;
    let mut _count = 0i32;
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
                cp_error_reason =
                    b"Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
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

unsafe fn cp_make32(s: *const u8) -> u32 {
    ((*s as u32) << 24)
        | ((*s.add(1) as u32) << 16)
        | ((*s.add(2) as u32) << 8)
        | (*s.add(3) as u32)
}

unsafe fn slice4_eq(p: *const u8, s: &[u8; 4]) -> bool {
    *p == s[0] && *p.add(1) == s[1] && *p.add(2) == s[2] && *p.add(3) == s[3]
}

unsafe fn cp_chunk(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    let len = cp_make32(png.p);
    let start = png.p;
    if slice4_eq(start.add(4), chunk) && len >= minlen {
        let offset = (len + 12) as isize;
        if (png.p as usize) + (offset as usize) <= png.end as usize {
            png.p = png.p.offset(offset);
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    while (png.p as usize) < (png.end as usize) {
        let len = cp_make32(png.p);
        let start = png.p;
        png.p = png.p.offset((len + 12) as isize);
        if slice4_eq(start.add(4), chunk) && len >= minlen && (png.p as usize) <= (png.end as usize)
        {
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_unfilter(w: i32, h: i32, bpp: i32, raw: *mut u8) -> i32 {
    let len = w * bpp;
    let mut raw = raw;
    let mut prev: *mut u8;
    let mut x: i32;
    if h > 0 {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    *raw.add(x as usize) = (*raw.add(x as usize))
                        .wrapping_add(*raw.add((x - bpp) as usize));
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
    let mut y = 1i32;
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
                    *raw.add(x as usize) = (*raw.add(x as usize))
                        .wrapping_add(*raw.add((x - bpp) as usize));
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    *raw.add(x as usize) = (*raw.add(x as usize))
                        .wrapping_add(*prev.add(x as usize));
                    x += 1;
                }
                while x < len {
                    *raw.add(x as usize) = (*raw.add(x as usize))
                        .wrapping_add(*prev.add(x as usize));
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
                    let avg = ((*raw.add((x - bpp) as usize) as u16
                        + *prev.add(x as usize) as u16)
                        / 2) as u8;
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(avg);
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    *raw.add(x as usize) = (*raw.add(x as usize))
                        .wrapping_add(*prev.add(x as usize));
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
        y += 1;
        prev = raw;
        raw = raw.add(len as usize);
    }
    1
}

unsafe fn cp_convert(bpp: i32, w: i32, h: i32, src: *mut u8, dst: *mut cp_pixel_t) {
    let mut src = src;
    let mut dst = dst;
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
                    *dst = cp_make_pixel_a(*src, *src.add(1), *src.add(2), *src.add(3));
                    dst = dst.add(1);
                }
                _ => {}
            }
            src = src.add(bpp as usize);
        }
    }
}

unsafe fn cp_get_alpha_for_indexed_image(index: i32, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() {
        255
    } else if (index as u32) >= trns_len {
        255
    } else {
        *trns.add(index as usize)
    }
}

unsafe fn cp_depalette(
    w: i32,
    h: i32,
    src: *mut u8,
    dst: *mut cp_pixel_t,
    plte: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    let mut src = src;
    let mut dst = dst;
    for _y in 0..h {
        src = src.add(1);
        for _x in 0..w {
            let c = *src as i32;
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

unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    cp_make32(chunk.offset(-8))
}

fn cp_out_size(img: &cp_image_t, bpp: i32) -> i32 {
    (img.w + 1) * img.h * bpp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    let sig: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    let mut img = cp_image_t {
        w: 0,
        h: 0,
        pix: ptr::null_mut(),
    };
    let mut data: *mut u8 = ptr::null_mut();

    let ok: bool = 'attempt: {
        let mut png = CpRawPng {
            p: png_data,
            end: png_data.add(png_length as usize),
        };

        // signature check
        let mut sig_eq = true;
        for i in 0..8 {
            if *png.p.add(i) != sig[i] {
                sig_eq = false;
                break;
            }
        }
        if !sig_eq {
            cp_error_reason =
                b"incorrect file signature (is this a png file?)\0".as_ptr() as *const c_char;
            break 'attempt false;
        }
        png.p = png.p.add(8);

        let ihdr = cp_chunk(&mut png, b"IHDR", 13);
        if ihdr.is_null() {
            cp_error_reason = b"unable to find IHDR chunk\0".as_ptr() as *const c_char;
            break 'attempt false;
        }

        let bit_depth = *ihdr.add(8) as i32;
        let color_type = *ihdr.add(9) as i32;

        if bit_depth != 8 {
            cp_error_reason = b"only bit-depth of 8 is supported\0".as_ptr() as *const c_char;
            break 'attempt false;
        }

        let bpp: i32 = match color_type {
            0 => 1,
            2 => 3,
            3 => 1,
            4 => 2,
            6 => 4,
            _ => {
                cp_error_reason = b"unknown color type\0".as_ptr() as *const c_char;
                break 'attempt false;
            }
        };

        let w = cp_make32(ihdr) as i32 + 1;
        let h = cp_make32(ihdr.add(4)) as i32;

        if w < 1 {
            cp_error_reason =
                b"invalid IHDR chunk found, image width was less than 1\0".as_ptr() as *const c_char;
            break 'attempt false;
        }
        if h < 1 {
            cp_error_reason = b"invalid IHDR chunk found, image height was less than 1\0".as_ptr()
                as *const c_char;
            break 'attempt false;
        }
        let pix_size_bytes =
            (w as i64) * (h as i64) * (std::mem::size_of::<cp_pixel_t>() as i64);
        if !(pix_size_bytes < i32::MAX as i64) {
            cp_error_reason = b"image too large\0".as_ptr() as *const c_char;
            break 'attempt false;
        }

        let pix_bytes = w * h * std::mem::size_of::<cp_pixel_t>() as i32;
        img.w = w - 1;
        img.h = h;
        img.pix = malloc(pix_bytes as usize) as *mut cp_pixel_t;

        if img.pix.is_null() {
            cp_error_reason =
                b"unable to allocate raw image space\0".as_ptr() as *const c_char;
            break 'attempt false;
        }

        let compression = *ihdr.add(10) as i32;
        let filter = *ihdr.add(11) as i32;
        let interlace = *ihdr.add(12) as i32;

        if compression != 0 {
            cp_error_reason =
                b"only standard compression DEFLATE is supported\0".as_ptr() as *const c_char;
            break 'attempt false;
        }
        if filter != 0 {
            cp_error_reason =
                b"only standard adaptive filtering is supported\0".as_ptr() as *const c_char;
            break 'attempt false;
        }
        if interlace != 0 {
            cp_error_reason = b"interlacing is not supported\0".as_ptr() as *const c_char;
            break 'attempt false;
        }

        let mut first_pos = png.p;
        let plte = cp_find(&mut png, b"PLTE", 0);
        if plte.is_null() {
            png.p = first_pos;
        } else {
            first_pos = png.p;
        }
        let trns = cp_find(&mut png, b"tRNS", 0);
        if trns.is_null() {
            png.p = first_pos;
        } else {
            first_pos = png.p;
        }

        let mut datalen: i32 = 0;
        {
            let mut idat = cp_find(&mut png, b"IDAT", 0);
            while !idat.is_null() {
                let len = cp_get_chunk_byte_length(idat) as i32;
                datalen += len;
                idat = cp_chunk(&mut png, b"IDAT", 0);
            }
        }
        png.p = first_pos;
        data = malloc(datalen as usize) as *mut u8;
        let mut offset: i32 = 0;
        {
            let mut idat = cp_find(&mut png, b"IDAT", 0);
            while !idat.is_null() {
                let len = cp_get_chunk_byte_length(idat) as i32;
                ptr::copy_nonoverlapping(idat, data.add(offset as usize), len as usize);
                offset += len;
                idat = cp_chunk(&mut png, b"IDAT", 0);
            }
        }

        if !(!data.is_null() && datalen >= 6) {
            cp_error_reason =
                b"corrupt zlib structure in DEFLATE stream\0".as_ptr() as *const c_char;
            break 'attempt false;
        }
        if (*data & 0x0f) != 0x08 {
            cp_error_reason =
                b"only zlib compression method (RFC 1950) is supported\0".as_ptr() as *const c_char;
            break 'attempt false;
        }
        if !((*data & 0xf0) <= 0x70) {
            cp_error_reason =
                b"innapropriate window size detected\0".as_ptr() as *const c_char;
            break 'attempt false;
        }
        if (*data.add(1) & 0x20) != 0 {
            cp_error_reason =
                b"preset dictionary is present and not supported\0".as_ptr() as *const c_char;
            break 'attempt false;
        }

        if !(cp_out_size(&img, 4) >= 1) {
            cp_error_reason = b"invalid image size found\0".as_ptr() as *const c_char;
            break 'attempt false;
        }
        if !(cp_out_size(&img, bpp) >= 1) {
            cp_error_reason = b"invalid image size found\0".as_ptr() as *const c_char;
            break 'attempt false;
        }

        let out = (img.pix as *mut u8)
            .add((cp_out_size(&img, 4) - cp_out_size(&img, bpp)) as usize);

        if cp_inflate(data.add(2), datalen - 6, out, pix_bytes) == 0 {
            cp_error_reason = b"DEFLATE algorithm failed\0".as_ptr() as *const c_char;
            break 'attempt false;
        }

        if cp_unfilter(img.w, img.h, bpp, out) == 0 {
            cp_error_reason = b"invalid filter byte found\0".as_ptr() as *const c_char;
            break 'attempt false;
        }

        if color_type == 3 {
            if plte.is_null() {
                cp_error_reason =
                    b"color type of indexed requires a PLTE chunk\0".as_ptr() as *const c_char;
                break 'attempt false;
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

        true
    };

    if !ok {
        free(data as *mut _);
        free(img.pix as *mut _);
        img.pix = ptr::null_mut();
        return img;
    }

    free(data as *mut _);
    img
}
