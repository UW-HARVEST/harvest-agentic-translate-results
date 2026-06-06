#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(static_mut_refs)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[inline]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

#[inline]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

static mut cp_error_reason: *const c_char = std::ptr::null();

static cp_fixed_table: [u8; 288 + 32] = {
    let mut t = [0u8; 288 + 32];
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

static cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

static cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

static cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

static cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

static cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

#[repr(C)]
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
            words: std::ptr::null(),
            word_count: 0,
            word_index: 0,
            bits_left: 0,
            final_word_available: 0,
            final_word: 0,
            out: std::ptr::null_mut(),
            out_end: std::ptr::null_mut(),
            begin: std::ptr::null_mut(),
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

fn cp_would_overflow(s: &cp_state_t, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

unsafe fn cp_ptr(s: &cp_state_t) -> *mut u8 {
    debug_assert!((s.bits_left & 7) == 0);
    // (char *)(s->words + s->word_index) - (s->count / 8)
    unsafe {
        let p = s.words.add(s.word_index as usize) as *mut u8;
        p.offset(-(s.count as isize / 8))
    }
}

unsafe fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u64 {
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
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

fn cp_consume_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u32 {
    debug_assert!(s.count >= num_bits_to_read);
    // Match C's `(((uint64_t)1 << num_bits_to_read) - 1)` semantics.
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

unsafe fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    debug_assert!(s.bits_left > 0);
    debug_assert!(s.count <= 64);
    debug_assert!(!cp_would_overflow(s, num_bits_to_read));
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

fn cp_build(s: Option<&mut cp_state_t>, tree: &mut [u32], lens: &[u8], sym_count: i32) -> i32 {
    let mut codes: [i32; 16] = [0; 16];
    let mut first: [i32; 16] = [0; 16];
    let mut counts: [i32; 16] = [0; 16];
    for n in 0..sym_count {
        counts[lens[n as usize] as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if let Some(ref s_ref) = s {
        // memset(s->lookup, 0, sizeof(s->lookup));
        let _ = s_ref; // suppress unused
    }
    // Need to do the lookup zeroing if `s` is provided. Reborrow.
    let s_opt: Option<&mut cp_state_t> = s;
    let s_opt = match s_opt {
        Some(state) => {
            for v in state.lookup.iter_mut() {
                *v = 0;
            }
            Some(state)
        }
        None => None,
    };

    let mut s_opt = s_opt;
    for i in 0..sym_count {
        let len = lens[i as usize] as i32;
        if len != 0 {
            debug_assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as u32;
            first[len as usize] += 1;
            tree[slot as usize] =
                (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(ref mut s_ref) = s_opt {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                    while j < (1 << 9) {
                        s_ref.lookup[j as usize] = ((len << 9) | i) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut cp_state_t) -> i32 {
    unsafe {
        cp_read_bits(s, s.count & 7);
    }
    let LEN: u16 = unsafe { cp_read_bits(s, 16) } as u16;
    let NLEN: u16 = unsafe { cp_read_bits(s, 16) } as u16;
    if !(LEN == !NLEN) {
        unsafe {
            cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
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
        std::ptr::copy_nonoverlapping(p, s.out, LEN as usize);
        s.out = s.out.add(LEN as usize);
    }
    1
}

fn cp_fixed(s: &mut cp_state_t) -> i32 {
    // s->nlit = cp_build(s, s->lit, cp_fixed_table, 288);
    // We need to split borrows. Since we modify both lit and lookup in the same call,
    // we can't simply split &mut. Use a raw pointer approach here for safety.
    let nlit = {
        // Pass `s` as Option<&mut> and `s.lit` slice via raw pointer.
        let lit_ptr = s.lit.as_mut_ptr();
        let lit_slice: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(lit_ptr, 288) };
        cp_build(Some(s), lit_slice, &cp_fixed_table[..288], 288)
    };
    s.nlit = nlit as u32;
    let ndst = {
        let dst_ptr = s.dst.as_mut_ptr();
        let dst_slice: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(dst_ptr, 32) };
        cp_build(None, dst_slice, &cp_fixed_table[288..(288 + 32)], 32)
    };
    s.ndst = ndst as u32;
    1
}

unsafe fn cp_decode(s: &mut cp_state_t, tree: &[u32], hi_in: i32) -> i32 {
    let bits = unsafe { cp_peak_bits(s, 16) };
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: i32 = 0;
    let mut hi: i32 = hi_in;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < tree[guess as usize] {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = tree[(lo - 1) as usize];
    let len = 32 - (key & 0xF);
    debug_assert!((search >> len) == (key >> len));
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

unsafe fn cp_dynamic(s: &mut cp_state_t) -> i32 {
    let mut lenlens: [u8; 19] = [0; 19];
    let nlit: i32 = 257 + unsafe { cp_read_bits(s, 5) } as i32;
    let ndst: i32 = 1 + unsafe { cp_read_bits(s, 5) } as i32;
    let nlen: i32 = 4 + unsafe { cp_read_bits(s, 4) } as i32;
    for i in 0..nlen {
        let bits = unsafe { cp_read_bits(s, 3) } as u8;
        lenlens[cp_permutation_order[i as usize] as usize] = bits;
    }
    let nlen_built = {
        let len_ptr = s.len.as_mut_ptr();
        let len_slice: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(len_ptr, 19) };
        cp_build(None, len_slice, &lenlens, 19)
    };
    s.nlen = nlen_built as u32;

    let mut lens: [u8; 288 + 32] = [0; 288 + 32];
    let total = nlit + ndst;
    let mut n: i32 = 0;
    while n < total {
        // Need to call cp_decode but tree borrows from s.len. Copy s.len out
        // then pass slice to cp_decode that doesn't conflict.
        let len_copy: [u32; 19] = s.len;
        let nlen_val = s.nlen as i32;
        let sym = unsafe { cp_decode(s, &len_copy, nlen_val) };
        match sym {
            16 => {
                let mut i = 3 + unsafe { cp_read_bits(s, 2) } as i32;
                while i != 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + unsafe { cp_read_bits(s, 3) } as i32;
                while i != 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + unsafe { cp_read_bits(s, 7) } as i32;
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
    let nlit_built = {
        let lit_ptr = s.lit.as_mut_ptr();
        let lit_slice: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(lit_ptr, 288) };
        cp_build(Some(s), lit_slice, &lens[..nlit as usize], nlit)
    };
    s.nlit = nlit_built as u32;
    let ndst_built = {
        let dst_ptr = s.dst.as_mut_ptr();
        let dst_slice: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(dst_ptr, 32) };
        cp_build(
            None,
            dst_slice,
            &lens[(nlit as usize)..((nlit + ndst) as usize)],
            ndst,
        )
    };
    s.ndst = ndst_built as u32;
    1
}

unsafe fn cp_block(s: &mut cp_state_t) -> i32 {
    loop {
        let lit_copy: [u32; 288] = s.lit;
        let nlit_val = s.nlit as i32;
        let symbol = unsafe { cp_decode(s, &lit_copy, nlit_val) };
        if symbol < 256 {
            if !(unsafe { s.out.add(1) } <= s.out_end) {
                unsafe {
                    cp_error_reason =
                        b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr()
                            as *const c_char;
                }
                return 0;
            }
            unsafe {
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = unsafe { cp_read_bits(s, cp_len_extra_bits[symbol as usize] as i32) }
                as i32
                + cp_len_base[symbol as usize] as i32;
            let dst_copy: [u32; 32] = s.dst;
            let ndst_val = s.ndst as i32;
            let distance_symbol = unsafe { cp_decode(s, &dst_copy, ndst_val) };
            let backwards_distance =
                unsafe { cp_read_bits(s, cp_dist_extra_bits[distance_symbol as usize] as i32) }
                    as i32
                    + cp_dist_base[distance_symbol as usize] as i32;
            if !(unsafe { s.out.offset(-(backwards_distance as isize)) } >= s.begin) {
                unsafe {
                    cp_error_reason =
                        b"Attempted to write before out buffer (invalid backwards distance).\0"
                            .as_ptr() as *const c_char;
                }
                return 0;
            }
            if !(unsafe { s.out.offset(length as isize) } <= s.out_end) {
                unsafe {
                    cp_error_reason =
                        b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr()
                            as *const c_char;
                }
                return 0;
            }
            let mut src_p = unsafe { s.out.offset(-(backwards_distance as isize)) };
            let mut dst_p = s.out;
            unsafe {
                s.out = s.out.offset(length as isize);
            }
            match backwards_distance {
                1 => {
                    let v = unsafe { *src_p };
                    unsafe {
                        std::ptr::write_bytes(dst_p, v, length as usize);
                    }
                }
                _ => {
                    let mut length = length;
                    while length != 0 {
                        unsafe {
                            *dst_p = *src_p;
                            dst_p = dst_p.add(1);
                            src_p = src_p.add(1);
                        }
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
pub unsafe extern "C" fn cp_inflate(
    in_ptr: *mut c_void,
    in_bytes: c_int,
    out_ptr: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut state = Box::new(cp_state_t::new());
    let s: &mut cp_state_t = &mut *state;
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;
    let in_addr = in_ptr as usize;
    let first_bytes: i32 = (((in_addr + 3) & !3usize).wrapping_sub(in_addr)) as i32;
    s.words = unsafe { (in_ptr as *const u8).add(first_bytes as usize) } as *const u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    let in_u8 = in_ptr as *const u8;
    for i in 0..first_bytes {
        let b = unsafe { *in_u8.add(i as usize) } as u64;
        s.bits |= b << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        let b = unsafe { *in_u8.add((in_bytes - last_bytes + i) as usize) } as u32;
        s.final_word |= b << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out_ptr as *mut u8;
    s.out_end = unsafe { s.out.add(out_bytes as usize) };
    s.begin = out_ptr as *mut u8;
    let mut count = 0;
    let mut bfinal: u32;
    loop {
        bfinal = unsafe { cp_read_bits(s, 1) };
        let btype = unsafe { cp_read_bits(s, 2) };
        match btype {
            0 => {
                if unsafe { cp_stored(s) } == 0 {
                    return 0;
                }
            }
            1 => {
                cp_fixed(s);
                if unsafe { cp_block(s) } == 0 {
                    return 0;
                }
            }
            2 => {
                unsafe {
                    cp_dynamic(s);
                }
                if unsafe { cp_block(s) } == 0 {
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
        count += 1;
        if bfinal != 0 {
            break;
        }
    }
    let _ = count;
    1
}

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p: i32 = a as i32 + b as i32 - c as i32;
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

#[repr(C)]
struct cp_raw_png_t {
    p: *const u8,
    end: *const u8,
}

fn cp_make32(s: &[u8]) -> u32 {
    ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | (s[3] as u32)
}

unsafe fn cp_chunk(png: &mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    let bytes = unsafe { std::slice::from_raw_parts(png.p, 8) };
    let len = cp_make32(&bytes[..4]);
    let start = png.p;
    if &bytes[4..8] == &chunk[..] && len >= minlen {
        let offset = (len + 12) as isize;
        if unsafe { png.p.offset(offset) } <= png.end {
            png.p = unsafe { png.p.offset(offset) };
            return unsafe { start.add(8) };
        }
    }
    std::ptr::null()
}

unsafe fn cp_find(png: &mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    while png.p < png.end {
        let bytes = unsafe { std::slice::from_raw_parts(png.p, 8) };
        let len = cp_make32(&bytes[..4]);
        let start = png.p;
        png.p = unsafe { png.p.offset((len + 12) as isize) };
        if &bytes[4..8] == &chunk[..] && len >= minlen && png.p <= png.end {
            return unsafe { start.add(8) };
        }
    }
    std::ptr::null()
}

unsafe fn cp_unfilter(w: i32, h: i32, bpp: i32, raw_in: *mut u8) -> i32 {
    let len = (w * bpp) as isize;
    let mut raw = raw_in;
    let mut prev: *mut u8;
    if h > 0 {
        let filter = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter {
            0 => {}
            1 => {
                let mut x = bpp as isize;
                while x < len {
                    unsafe {
                        let v = *raw.offset(x - bpp as isize);
                        *raw.offset(x) = (*raw.offset(x)).wrapping_add(v);
                    }
                    x += 1;
                }
            }
            2 => {}
            3 => {
                let mut x = bpp as isize;
                while x < len {
                    unsafe {
                        let v = *raw.offset(x - bpp as isize);
                        *raw.offset(x) = (*raw.offset(x)).wrapping_add(v / 2);
                    }
                    x += 1;
                }
            }
            4 => {
                let mut x = bpp as isize;
                while x < len {
                    unsafe {
                        let a = *raw.offset(x - bpp as isize);
                        *raw.offset(x) = (*raw.offset(x)).wrapping_add(cp_paeth(a, 0, 0));
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }
    }
    prev = raw;
    raw = unsafe { raw.offset(len) };
    for _y in 1..h {
        let filter = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter {
            0 => {}
            1 => {
                let mut x: isize = 0;
                while x < bpp as isize {
                    // raw[x] += 0; no-op
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let v = *raw.offset(x - bpp as isize);
                        *raw.offset(x) = (*raw.offset(x)).wrapping_add(v);
                    }
                    x += 1;
                }
            }
            2 => {
                let mut x: isize = 0;
                while x < bpp as isize {
                    unsafe {
                        let pv = *prev.offset(x);
                        *raw.offset(x) = (*raw.offset(x)).wrapping_add(pv);
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let pv = *prev.offset(x);
                        *raw.offset(x) = (*raw.offset(x)).wrapping_add(pv);
                    }
                    x += 1;
                }
            }
            3 => {
                let mut x: isize = 0;
                while x < bpp as isize {
                    unsafe {
                        let pv = *prev.offset(x);
                        *raw.offset(x) = (*raw.offset(x)).wrapping_add(pv / 2);
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let a = *raw.offset(x - bpp as isize);
                        let pv = *prev.offset(x);
                        *raw.offset(x) =
                            (*raw.offset(x)).wrapping_add(((a as i32 + pv as i32) / 2) as u8);
                    }
                    x += 1;
                }
            }
            4 => {
                let mut x: isize = 0;
                while x < bpp as isize {
                    unsafe {
                        let pv = *prev.offset(x);
                        *raw.offset(x) = (*raw.offset(x)).wrapping_add(pv);
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let a = *raw.offset(x - bpp as isize);
                        let b = *prev.offset(x);
                        let c = *prev.offset(x - bpp as isize);
                        *raw.offset(x) = (*raw.offset(x)).wrapping_add(cp_paeth(a, b, c));
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }
        prev = raw;
        raw = unsafe { raw.offset(len) };
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
        unsafe {
            src = src.add(1);
        }
        for _x in 0..w {
            unsafe {
                match bpp {
                    1 => {
                        let s0 = *src.add(0);
                        *dst = cp_make_pixel(s0, s0, s0);
                        dst = dst.add(1);
                    }
                    2 => {
                        let s0 = *src.add(0);
                        let s1 = *src.add(1);
                        *dst = cp_make_pixel_a(s0, s0, s0, s1);
                        dst = dst.add(1);
                    }
                    3 => {
                        let s0 = *src.add(0);
                        let s1 = *src.add(1);
                        let s2 = *src.add(2);
                        *dst = cp_make_pixel(s0, s1, s2);
                        dst = dst.add(1);
                    }
                    4 => {
                        let s0 = *src.add(0);
                        let s1 = *src.add(1);
                        let s2 = *src.add(2);
                        let s3 = *src.add(3);
                        *dst = cp_make_pixel_a(s0, s1, s2, s3);
                        dst = dst.add(1);
                    }
                    _ => {}
                }
                src = src.add(bpp as usize);
            }
        }
    }
}
