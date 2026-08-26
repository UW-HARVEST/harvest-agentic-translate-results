#![allow(non_camel_case_types, non_upper_case_globals, non_snake_case)]

use std::ffi::c_int;
use std::os::raw::c_char;
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

// Global error reason (external linkage in C source).
#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

// External linkage in C: we expose with the same name.
#[unsafe(no_mangle)]
pub static cp_fixed_table: [u8; 288 + 32] = [
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
pub static cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59,
    67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

#[unsafe(no_mangle)]
pub static cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

struct cp_state_t {
    bits: u64,
    count: i32,
    words: *const u32,
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

impl cp_state_t {
    fn new() -> Self {
        cp_state_t {
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

fn cp_would_overflow(s: &cp_state_t, num_bits: i32) -> i32 {
    if (s.bits_left + s.count) - num_bits < 0 {
        1
    } else {
        0
    }
}

unsafe fn cp_ptr(s: &cp_state_t) -> *mut u8 {
    debug_assert!((s.bits_left & 7) == 0);
    let p = unsafe { s.words.add(s.word_index as usize) as *const u8 };
    let p = unsafe { p.sub((s.count / 8) as usize) };
    p as *mut u8
}

fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { *s.words.add(s.word_index as usize) };
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            debug_assert!(s.word_index <= s.word_count);
        } else if s.final_word_available != 0 {
            let word = s.final_word;
            s.bits |= (word as u64) << s.count;
            // NOTE: This intentionally mirrors the original C source,
            // which adds bits_left here (preserved verbatim).
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

fn cp_consume_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u32 {
    debug_assert!(s.count >= num_bits_to_read);
    let mask: u64 = if num_bits_to_read >= 64 {
        u64::MAX
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

fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    debug_assert!(s.bits_left > 0);
    debug_assert!(s.count <= 64);
    debug_assert!(cp_would_overflow(s, num_bits_to_read) == 0);
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
    s: Option<&mut cp_state_t>,
    tree: &mut [u32],
    lens: &[u8],
    sym_count: i32,
) -> i32 {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    for n in 0..sym_count as usize {
        counts[lens[n] as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }

    if let Some(ref state) = s {
        // memset lookup to 0
        // We need to access via mutable; do it after the immutable check.
        let _ = state;
    }
    // Re-borrow mutably.
    let s_opt: Option<&mut cp_state_t> = s;
    if let Some(state) = s_opt {
        for v in state.lookup.iter_mut() {
            *v = 0;
        }
        for i in 0..sym_count as usize {
            let len = lens[i] as i32;
            if len != 0 {
                debug_assert!(len < 16);
                let code = codes[len as usize] as u32;
                codes[len as usize] += 1;
                let slot = first[len as usize] as usize;
                first[len as usize] += 1;
                tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        state.lookup[j] = ((len as u16) << 9) | (i as u16);
                        j += 1usize << len;
                    }
                }
            }
        }
    } else {
        for i in 0..sym_count as usize {
            let len = lens[i] as i32;
            if len != 0 {
                debug_assert!(len < 16);
                let code = codes[len as usize] as u32;
                codes[len as usize] += 1;
                let slot = first[len as usize] as usize;
                first[len as usize] += 1;
                tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut cp_state_t) -> i32 {
    cp_read_bits(s, s.count & 7);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        unsafe {
            cp_error_reason =
                b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0"
                    .as_ptr() as *const c_char;
        }
        return 0;
    }
    if !(s.bits_left / 8 <= LEN as i32) {
        unsafe {
            cp_error_reason =
                b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        }
        return 0;
    }
    let p = unsafe { cp_ptr(s) };
    unsafe {
        ptr::copy_nonoverlapping(p as *const u8, s.out, LEN as usize);
    }
    s.out = unsafe { s.out.add(LEN as usize) };
    1
}

fn cp_fixed(s: &mut cp_state_t) -> i32 {
    // build for s->lit using cp_fixed_table[0..288]
    {
        let mut lit = std::mem::replace(&mut s.lit, [0u32; 288]);
        let nlit =
            cp_build(Some(s), &mut lit, &cp_fixed_table[0..288], 288);
        s.lit = lit;
        s.nlit = nlit as u32;
    }
    {
        let mut dst = std::mem::replace(&mut s.dst, [0u32; 32]);
        let ndst = cp_build(None, &mut dst, &cp_fixed_table[288..288 + 32], 32);
        s.dst = dst;
        s.ndst = ndst as u32;
    }
    1
}

fn cp_decode(s: &mut cp_state_t, tree: &[u32], hi_in: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: i32 = 0;
    let mut hi = hi_in;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < tree[guess as usize] {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = tree[(lo - 1) as usize];
    let _len_check = 32 - (key & 0xF);
    let _ = _len_check;
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

fn cp_dynamic(s: &mut cp_state_t) -> i32 {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen as usize {
        lenlens[cp_permutation_order[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    {
        let mut len_arr = std::mem::replace(&mut s.len, [0u32; 19]);
        let nlen_built = cp_build(None, &mut len_arr, &lenlens, 19);
        s.len = len_arr;
        s.nlen = nlen_built as u32;
    }
    let mut lens = [0u8; 288 + 32];
    let mut n: i32 = 0;
    while n < nlit + ndst {
        let len_arr_copy: [u32; 19] = s.len;
        let sym = cp_decode(s, &len_arr_copy, s.nlen as i32);
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
    {
        let mut lit = std::mem::replace(&mut s.lit, [0u32; 288]);
        let nlit_built = cp_build(Some(s), &mut lit, &lens[0..nlit as usize], nlit);
        s.lit = lit;
        s.nlit = nlit_built as u32;
    }
    {
        let mut dst = std::mem::replace(&mut s.dst, [0u32; 32]);
        let ndst_built = cp_build(
            None,
            &mut dst,
            &lens[nlit as usize..(nlit as usize + ndst as usize)],
            ndst,
        );
        s.dst = dst;
        s.ndst = ndst_built as u32;
    }
    1
}

unsafe fn cp_block(s: &mut cp_state_t) -> i32 {
    loop {
        let lit_copy: [u32; 288] = s.lit;
        let symbol = cp_decode(s, &lit_copy, s.nlit as i32);
        if symbol < 256 {
            if !(unsafe { s.out.add(1) } <= s.out_end) {
                unsafe {
                    cp_error_reason =
                        b"Attempted to overwrite out buffer while outputting a symbol.\0"
                            .as_ptr() as *const c_char;
                }
                return 0;
            }
            unsafe {
                *s.out = symbol as u8;
            }
            s.out = unsafe { s.out.add(1) };
        } else if symbol > 256 {
            let symbol_idx = (symbol - 257) as usize;
            let length =
                cp_read_bits(s, cp_len_extra_bits[symbol_idx] as i32) as i32
                    + cp_len_base[symbol_idx] as i32;
            let dst_copy: [u32; 32] = s.dst;
            let distance_symbol = cp_decode(s, &dst_copy, s.ndst as i32);
            let backwards_distance = cp_read_bits(
                s,
                cp_dist_extra_bits[distance_symbol as usize] as i32,
            ) as i32
                + cp_dist_base[distance_symbol as usize] as i32;
            if !(unsafe { s.out.offset(-(backwards_distance as isize)) } >= s.begin) {
                unsafe {
                    cp_error_reason =
                        b"Attempted to write before out buffer (invalid backwards distance).\0"
                            .as_ptr() as *const c_char;
                }
                return 0;
            }
            if !(unsafe { s.out.add(length as usize) } <= s.out_end) {
                unsafe {
                    cp_error_reason =
                        b"Attempted to overwrite out buffer while outputting a string.\0"
                            .as_ptr() as *const c_char;
                }
                return 0;
            }
            let src = unsafe { s.out.offset(-(backwards_distance as isize)) } as *const u8;
            let mut dst_p = s.out;
            s.out = unsafe { s.out.add(length as usize) };
            match backwards_distance {
                1 => unsafe {
                    let val = *src;
                    for k in 0..length as usize {
                        *dst_p.add(k) = val;
                    }
                },
                _ => {
                    let mut len = length;
                    let mut sp = src;
                    unsafe {
                        while len != 0 {
                            *dst_p = *sp;
                            dst_p = dst_p.add(1);
                            sp = sp.add(1);
                            len -= 1;
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

unsafe fn cp_inflate(
    in_ptr: *mut u8,
    in_bytes: i32,
    out_ptr: *mut u8,
    out_bytes: i32,
) -> i32 {
    let mut s = Box::new(cp_state_t::new());
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;
    let in_addr = in_ptr as usize;
    let first_bytes: i32 = (((in_addr + 3) & !3usize) - in_addr) as i32;
    s.words = unsafe { (in_ptr as *const u8).add(first_bytes as usize) as *const u32 };
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes as usize {
        let b = unsafe { *(in_ptr as *const u8).add(i) };
        s.bits |= (b as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes as usize {
        let idx = (in_bytes as usize) - (last_bytes as usize) + i;
        let b = unsafe { *(in_ptr as *const u8).add(idx) };
        s.final_word |= (b as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out_ptr;
    s.out_end = unsafe { out_ptr.add(out_bytes as usize) };
    s.begin = out_ptr;
    let mut _count: i32 = 0;
    let mut bfinal: u32;
    loop {
        bfinal = cp_read_bits(&mut s, 1);
        let btype = cp_read_bits(&mut s, 2);
        match btype {
            0 => {
                let r = unsafe { cp_stored(&mut s) };
                if r == 0 {
                    return 0;
                }
            }
            1 => {
                cp_fixed(&mut s);
                let r = unsafe { cp_block(&mut s) };
                if r == 0 {
                    return 0;
                }
            }
            2 => {
                cp_dynamic(&mut s);
                let r = unsafe { cp_block(&mut s) };
                if r == 0 {
                    return 0;
                }
            }
            3 => {
                unsafe {
                    cp_error_reason =
                        b"Detected unknown block type within input stream.\0".as_ptr()
                            as *const c_char;
                }
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
    let p: i32 = (a as i32) + (b as i32) - (c as i32);
    let pa: i32 = (p - a as i32).abs();
    let pb: i32 = (p - b as i32).abs();
    let pc: i32 = (p - c as i32).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

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

unsafe fn cp_chunk(png: &mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    let len = unsafe { cp_make32(png.p) };
    let start = png.p;
    let matches = unsafe {
        *start.add(4) == chunk[0]
            && *start.add(5) == chunk[1]
            && *start.add(6) == chunk[2]
            && *start.add(7) == chunk[3]
    };
    if matches && len >= minlen {
        let offset = (len + 12) as usize;
        if unsafe { png.p.add(offset) } <= png.end {
            png.p = unsafe { png.p.add(offset) };
            return unsafe { start.add(8) };
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: &mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    while png.p < png.end {
        let len = unsafe { cp_make32(png.p) };
        let start = png.p;
        png.p = unsafe { png.p.add((len + 12) as usize) };
        let matches = unsafe {
            *start.add(4) == chunk[0]
                && *start.add(5) == chunk[1]
                && *start.add(6) == chunk[2]
                && *start.add(7) == chunk[3]
        };
        if matches && len >= minlen && png.p <= png.end {
            return unsafe { start.add(8) };
        }
    }
    ptr::null()
}

unsafe fn cp_unfilter(w: i32, h: i32, bpp: i32, raw_in: *mut u8) -> i32 {
    let len = w * bpp;
    let mut raw = raw_in;
    let mut prev: *mut u8;
    let mut x: i32;
    if h > 0 {
        let filter_byte = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter_byte {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    unsafe {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize));
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    unsafe {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize) / 2);
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    unsafe {
                        let pa = cp_paeth(*raw.offset((x - bpp) as isize), 0, 0);
                        let v = (*raw.offset(x as isize)).wrapping_add(pa);
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }
    }
    prev = raw;
    raw = unsafe { raw.add(len as usize) };
    let mut y = 1;
    while y < h {
        let filter_byte = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter_byte {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        let v = (*raw.offset(x as isize)).wrapping_add(0);
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize));
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*prev.offset(x as isize));
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*prev.offset(x as isize));
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*prev.offset(x as isize) / 2);
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let s_val = (*raw.offset((x - bpp) as isize) as u32
                            + *prev.offset(x as isize) as u32)
                            / 2;
                        let v = (*raw.offset(x as isize)).wrapping_add(s_val as u8);
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*prev.offset(x as isize));
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let pa = cp_paeth(
                            *raw.offset((x - bpp) as isize),
                            *prev.offset(x as isize),
                            *prev.offset((x - bpp) as isize),
                        );
                        let v = (*raw.offset(x as isize)).wrapping_add(pa);
                        *raw.offset(x as isize) = v;
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }
        prev = raw;
        raw = unsafe { raw.add(len as usize) };
        y += 1;
    }
    1
}

unsafe fn cp_convert(bpp: i32, w: i32, h: i32, src_in: *mut u8, dst_in: *mut cp_pixel_t) {
    let mut src = src_in;
    let mut dst = dst_in;
    for _y in 0..h {
        src = unsafe { src.add(1) };
        for _x in 0..w {
            unsafe {
                match bpp {
                    1 => {
                        *dst = cp_make_pixel(*src.add(0), *src.add(0), *src.add(0));
                        dst = dst.add(1);
                    }
                    2 => {
                        *dst =
                            cp_make_pixel_a(*src.add(0), *src.add(0), *src.add(0), *src.add(1));
                        dst = dst.add(1);
                    }
                    3 => {
                        *dst = cp_make_pixel(*src.add(0), *src.add(1), *src.add(2));
                        dst = dst.add(1);
                    }
                    4 => {
                        *dst = cp_make_pixel_a(
                            *src.add(0),
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

fn cp_get_alpha_for_indexed_image(index: i32, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() {
        return 255;
    }
    if (index as u32) >= trns_len {
        return 255;
    }
    unsafe { *trns.add(index as usize) }
}

unsafe fn cp_depalette(
    w: i32,
    h: i32,
    src_in: *mut u8,
    dst_in: *mut cp_pixel_t,
    plte: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    let mut src = src_in;
    let mut dst = dst_in;
    for _y in 0..h {
        src = unsafe { src.add(1) };
        for _x in 0..w {
            let c = unsafe { *src } as i32;
            let r = unsafe { *plte.add((c * 3) as usize) };
            let g = unsafe { *plte.add((c * 3 + 1) as usize) };
            let b = unsafe { *plte.add((c * 3 + 2) as usize) };
            let a = cp_get_alpha_for_indexed_image(c, trns, trns_len);
            unsafe {
                *dst = cp_make_pixel_a(r, g, b, a);
                dst = dst.add(1);
                src = src.add(1);
            }
        }
    }
}

unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    unsafe { cp_make32(chunk.sub(8)) }
}

fn cp_out_size(img: &cp_image_t, bpp: i32) -> i32 {
    (img.w + 1) * img.h * bpp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    let sig: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];
    let bit_depth: i32;
    let color_type: i32;
    let bpp: i32;
    let w: i32;
    let h: i32;
    let pix_bytes: i32;
    let compression: i32;
    let filter: i32;
    let interlace: i32;
    let mut datalen: i32;
    let offset_init: i32;
    let _ = offset_init;
    let mut img = cp_image_t {
        w: 0,
        h: 0,
        pix: ptr::null_mut(),
    };
    let mut data: *mut u8 = ptr::null_mut();
    let mut png = cp_raw_png_t {
        p: png_data,
        end: unsafe { png_data.add(png_length as usize) },
    };

    // Signature check
    let sig_match = unsafe {
        let p = png.p;
        (0..8).all(|i| *p.add(i) == sig[i])
    };
    if !sig_match {
        unsafe {
            cp_error_reason = b"incorrect file signature (is this a png file?)\0".as_ptr()
                as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    png.p = unsafe { png.p.add(8) };

    let ihdr = unsafe { cp_chunk(&mut png, b"IHDR", 13) };
    if ihdr.is_null() {
        unsafe {
            cp_error_reason = b"unable to find IHDR chunk\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    bit_depth = unsafe { *ihdr.add(8) } as i32;
    color_type = unsafe { *ihdr.add(9) } as i32;
    if !(bit_depth == 8) {
        unsafe {
            cp_error_reason = b"only bit-depth of 8 is supported\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    bpp = match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => {
            unsafe {
                cp_error_reason = b"unknown color type\0".as_ptr() as *const c_char;
            }
            return cp_err(data, &mut img);
        }
    };

    w = unsafe { cp_make32(ihdr) } as i32 + 1;
    h = unsafe { cp_make32(ihdr.add(4)) } as i32;
    if !(w >= 1) {
        unsafe {
            cp_error_reason =
                b"invalid IHDR chunk found, image width was less than 1\0".as_ptr()
                    as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    if !(h >= 1) {
        unsafe {
            cp_error_reason =
                b"invalid IHDR chunk found, image height was less than 1\0".as_ptr()
                    as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    let total = (w as i64) * (h as i64) * (std::mem::size_of::<cp_pixel_t>() as i64);
    if !(total < (i32::MAX as i64)) {
        unsafe {
            cp_error_reason = b"image too large\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    pix_bytes = w * h * (std::mem::size_of::<cp_pixel_t>() as i32);
    img.w = w - 1;
    img.h = h;
    // malloc-equivalent
    img.pix = unsafe { libc_malloc(pix_bytes as usize) as *mut cp_pixel_t };
    if img.pix.is_null() {
        unsafe {
            cp_error_reason =
                b"unable to allocate raw image space\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    compression = unsafe { *ihdr.add(10) } as i32;
    filter = unsafe { *ihdr.add(11) } as i32;
    interlace = unsafe { *ihdr.add(12) } as i32;
    if !(compression == 0) {
        unsafe {
            cp_error_reason =
                b"only standard compression DEFLATE is supported\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    if !(filter == 0) {
        unsafe {
            cp_error_reason =
                b"only standard adaptive filtering is supported\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    if !(interlace == 0) {
        unsafe {
            cp_error_reason = b"interlacing is not supported\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    let mut first = png.p;
    let plte = unsafe { cp_find(&mut png, b"PLTE", 0) };
    if plte.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    let trns = unsafe { cp_find(&mut png, b"tRNS", 0) };
    if trns.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    datalen = 0;
    {
        let mut idat = unsafe { cp_find(&mut png, b"IDAT", 0) };
        while !idat.is_null() {
            let len = unsafe { cp_get_chunk_byte_length(idat) };
            datalen += len as i32;
            idat = unsafe { cp_chunk(&mut png, b"IDAT", 0) };
        }
    }
    png.p = first;
    data = unsafe { libc_malloc(datalen as usize) as *mut u8 };
    let mut offset: i32 = 0;
    {
        let mut idat = unsafe { cp_find(&mut png, b"IDAT", 0) };
        while !idat.is_null() {
            let len = unsafe { cp_get_chunk_byte_length(idat) };
            unsafe {
                ptr::copy_nonoverlapping(idat, data.add(offset as usize), len as usize);
            }
            offset += len as i32;
            idat = unsafe { cp_chunk(&mut png, b"IDAT", 0) };
        }
    }
    if !(!data.is_null() && datalen >= 6) {
        unsafe {
            cp_error_reason =
                b"corrupt zlib structure in DEFLATE stream\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    if !((unsafe { *data } & 0x0f) == 0x08) {
        unsafe {
            cp_error_reason =
                b"only zlib compression method (RFC 1950) is supported\0".as_ptr()
                    as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    if !((unsafe { *data } & 0xf0) <= 0x70) {
        unsafe {
            cp_error_reason =
                b"innapropriate window size detected\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    if !((unsafe { *data.add(1) } & 0x20) == 0) {
        unsafe {
            cp_error_reason =
                b"preset dictionary is present and not supported\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    if !(cp_out_size(&img, 4) >= 1) {
        unsafe {
            cp_error_reason = b"invalid image size found\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    if !(cp_out_size(&img, bpp) >= 1) {
        unsafe {
            cp_error_reason = b"invalid image size found\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    let out: *mut u8 = unsafe {
        (img.pix as *mut u8)
            .add(cp_out_size(&img, 4) as usize)
            .sub(cp_out_size(&img, bpp) as usize)
    };
    let inflate_ok = unsafe {
        cp_inflate(
            data.add(2),
            datalen - 6,
            out,
            pix_bytes,
        )
    };
    if !(inflate_ok != 0) {
        unsafe {
            cp_error_reason = b"DEFLATE algorithm failed\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    if !(unsafe { cp_unfilter(img.w, img.h, bpp, out) } != 0) {
        unsafe {
            cp_error_reason = b"invalid filter byte found\0".as_ptr() as *const c_char;
        }
        return cp_err(data, &mut img);
    }
    if color_type == 3 {
        if plte.is_null() {
            unsafe {
                cp_error_reason =
                    b"color type of indexed requires a PLTE chunk\0".as_ptr() as *const c_char;
            }
            return cp_err(data, &mut img);
        }
        let trns_len = if !trns.is_null() {
            unsafe { cp_get_chunk_byte_length(trns) }
        } else {
            0
        };
        unsafe {
            cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
        }
    } else {
        unsafe {
            cp_convert(bpp, img.w, img.h, out, img.pix);
        }
    }
    unsafe {
        libc_free(data as *mut std::ffi::c_void);
    }
    img
}

fn cp_err(data: *mut u8, img: &mut cp_image_t) -> cp_image_t {
    unsafe {
        if !data.is_null() {
            libc_free(data as *mut std::ffi::c_void);
        }
        if !img.pix.is_null() {
            libc_free(img.pix as *mut std::ffi::c_void);
        }
    }
    img.pix = ptr::null_mut();
    cp_image_t {
        w: img.w,
        h: img.h,
        pix: ptr::null_mut(),
    }
}

// FFI to libc malloc/free so cp_pixel_t buffer is allocated via the C allocator.
extern "C" {
    fn malloc(size: usize) -> *mut std::ffi::c_void;
    fn free(ptr: *mut std::ffi::c_void);
}

#[inline]
unsafe fn libc_malloc(size: usize) -> *mut std::ffi::c_void {
    unsafe { malloc(size) }
}

#[inline]
unsafe fn libc_free(p: *mut std::ffi::c_void) {
    unsafe { free(p) }
}
