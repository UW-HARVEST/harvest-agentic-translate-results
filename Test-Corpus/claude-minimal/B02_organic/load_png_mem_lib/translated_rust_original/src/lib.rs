#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(unused_assignments)]
#![allow(unused_mut)]
#![allow(unused_variables)]
#![allow(dead_code)]

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
    pub w: i32,
    pub h: i32,
    pub pix: *mut cp_pixel_t,
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

// Global error reason. Mirrors `const char *cp_error_reason;` from C.
static mut CP_ERROR_REASON: *const c_char = ptr::null();

fn set_error(msg: &'static [u8]) {
    // msg should be null-terminated
    unsafe {
        CP_ERROR_REASON = msg.as_ptr() as *const c_char;
    }
}

#[no_mangle]
pub static mut cp_error_reason: *const c_char = ptr::null();

fn sync_error() {
    unsafe {
        cp_error_reason = CP_ERROR_REASON;
    }
}

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
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59,
    67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
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

impl CpState {
    fn new() -> Self {
        CpState {
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

fn cp_would_overflow(s: &CpState, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

unsafe fn cp_ptr(s: &CpState) -> *mut u8 {
    debug_assert!((s.bits_left & 7) == 0);
    // Equivalent to: (char *)(s->words + s->word_index) - (s->count / 8)
    let p = s.words.offset(s.word_index as isize) as *mut u8;
    p.offset(-(s.count as isize / 8))
}

unsafe fn cp_peak_bits(s: &mut CpState, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = *s.words.offset(s.word_index as isize);
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

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    debug_assert!(s.count >= num_bits_to_read);
    let bits = if num_bits_to_read >= 64 {
        s.bits as u32
    } else {
        (s.bits & ((1u64 << num_bits_to_read) - 1)) as u32
    };
    if num_bits_to_read >= 64 {
        s.bits = 0;
    } else {
        s.bits >>= num_bits_to_read;
    }
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    debug_assert!(s.bits_left > 0);
    debug_assert!(s.count <= 64);
    debug_assert!(!cp_would_overflow(s, num_bits_to_read));
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

fn cp_build(s: Option<&mut CpState>, tree: &mut [u32], lens: &[u8], sym_count: usize) -> i32 {
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
    if let Some(ref state) = s {
        // We can't borrow s twice, so handle below
    }
    // Reset lookup if state present
    let has_state = s.is_some();
    if let Some(state) = s.as_ref() {
        // we reset below
    }
    // Use a raw approach: take Option<&mut> and handle
    let state_ptr: *mut CpState = match s {
        Some(state) => state as *mut CpState,
        None => ptr::null_mut(),
    };
    unsafe {
        if !state_ptr.is_null() {
            for v in (*state_ptr).lookup.iter_mut() {
                *v = 0;
            }
        }
        for i in 0..sym_count {
            let len = lens[i] as i32;
            if len != 0 {
                debug_assert!(len < 16);
                let code = codes[len as usize] as u32;
                codes[len as usize] += 1;
                let slot = first[len as usize] as usize;
                first[len as usize] += 1;
                tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
                if !state_ptr.is_null() && len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                    while j < (1 << 9) {
                        (*state_ptr).lookup[j as usize] = ((len << 9) | (i as i32)) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut CpState) -> i32 {
    let bits_to_skip = s.count & 7;
    cp_read_bits(s, bits_to_skip);
    let len_v = cp_read_bits(s, 16) as u16;
    let nlen_v = cp_read_bits(s, 16) as u16;
    if len_v != !nlen_v {
        set_error(b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0");
        return 0;
    }
    if !(s.bits_left / 8 <= len_v as i32) {
        set_error(b"Stored block extends beyond end of input stream.\0");
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p, s.out, len_v as usize);
    s.out = s.out.add(len_v as usize);
    1
}

fn cp_fixed(s: &mut CpState) -> i32 {
    // Build literal tree from first 288 entries, distance tree from next 32
    let mut lit_tree = [0u32; 288];
    let mut dst_tree = [0u32; 32];
    let nlit = cp_build(Some(s), &mut lit_tree, &CP_FIXED_TABLE[..288], 288);
    let ndst = cp_build(None, &mut dst_tree, &CP_FIXED_TABLE[288..], 32);
    s.lit = lit_tree;
    s.dst = dst_tree;
    s.nlit = nlit as u32;
    s.ndst = ndst as u32;
    1
}

unsafe fn cp_decode(s: &mut CpState, tree: &[u32], hi: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: i32 = 0;
    let mut hi = hi;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < tree[guess as usize] {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = tree[(lo - 1) as usize];
    let len = 32u32 - (key & 0xF);
    debug_assert!((search >> len) == (key >> len));
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

unsafe fn cp_dynamic(s: &mut CpState) -> i32 {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen as usize {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    let mut len_tree = [0u32; 19];
    let nlen_built = cp_build(None, &mut len_tree, &lenlens, 19);
    s.len = len_tree;
    s.nlen = nlen_built as u32;

    let mut lens = [0u8; 288 + 32];
    let mut n: usize = 0;
    while n < (nlit + ndst) as usize {
        let sym = cp_decode(s, &s.len.clone(), s.nlen as i32);
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as i32;
                while i > 0 {
                    lens[n] = lens[n - 1];
                    n += 1;
                    i -= 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as i32;
                while i > 0 {
                    lens[n] = 0;
                    n += 1;
                    i -= 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as i32;
                while i > 0 {
                    lens[n] = 0;
                    n += 1;
                    i -= 1;
                }
            }
            _ => {
                lens[n] = sym as u8;
                n += 1;
            }
        }
    }
    let mut lit_tree = [0u32; 288];
    let mut dst_tree = [0u32; 32];
    let nlit_built = cp_build(Some(s), &mut lit_tree, &lens[..nlit as usize], nlit as usize);
    let ndst_built = cp_build(None, &mut dst_tree, &lens[nlit as usize..], ndst as usize);
    s.lit = lit_tree;
    s.dst = dst_tree;
    s.nlit = nlit_built as u32;
    s.ndst = ndst_built as u32;
    1
}

unsafe fn cp_block(s: &mut CpState) -> i32 {
    loop {
        let lit_clone = s.lit;
        let nlit = s.nlit as i32;
        let symbol = cp_decode(s, &lit_clone, nlit);
        if symbol < 256 {
            if !(s.out as usize + 1 <= s.out_end as usize) {
                set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return 0;
            }
            *s.out = symbol as u8;
            s.out = s.out.add(1);
        } else if symbol > 256 {
            let symbol_idx = (symbol - 257) as usize;
            let length = cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol_idx] as i32) as i32
                + CP_LEN_BASE[symbol_idx] as i32;
            let dst_clone = s.dst;
            let ndst = s.ndst as i32;
            let distance_symbol = cp_decode(s, &dst_clone, ndst) as usize;
            let backwards_distance =
                cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol] as i32) as i32
                    + CP_DIST_BASE[distance_symbol] as i32;
            if !((s.out as isize - backwards_distance as isize) >= s.begin as isize) {
                set_error(b"Attempted to write before out buffer (invalid backwards distance).\0");
                return 0;
            }
            if !((s.out as usize + length as usize) <= s.out_end as usize) {
                set_error(b"Attempted to overwrite out buffer while outputting a string.\0");
                return 0;
            }
            let src = s.out.offset(-(backwards_distance as isize));
            let dst = s.out;
            s.out = s.out.add(length as usize);
            if backwards_distance == 1 {
                ptr::write_bytes(dst, *src, length as usize);
            } else {
                let mut src_p = src;
                let mut dst_p = dst;
                let mut remaining = length;
                while remaining > 0 {
                    *dst_p = *src_p;
                    dst_p = dst_p.add(1);
                    src_p = src_p.add(1);
                    remaining -= 1;
                }
            }
        } else {
            break;
        }
    }
    1
}

unsafe fn cp_inflate(
    in_ptr: *const u8,
    in_bytes: i32,
    out_ptr: *mut u8,
    out_bytes: i32,
) -> i32 {
    let mut s = Box::new(CpState::new());
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;
    let in_addr = in_ptr as usize;
    let aligned_addr = (in_addr + 3) & !3usize;
    let first_bytes = (aligned_addr - in_addr) as i32;
    s.words = (in_ptr as *const u8).add(first_bytes as usize) as *const u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        s.bits |= (*(in_ptr.add(i as usize)) as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        s.final_word |=
            (*(in_ptr.add((in_bytes - last_bytes + i) as usize)) as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out_ptr;
    s.out_end = out_ptr.add(out_bytes as usize);
    s.begin = out_ptr;
    let mut count = 0i32;
    loop {
        let bfinal = cp_read_bits(&mut s, 1);
        let btype = cp_read_bits(&mut s, 2);
        match btype {
            0 => {
                if cp_stored(&mut s) == 0 {
                    sync_error();
                    return 0;
                }
            }
            1 => {
                cp_fixed(&mut s);
                if cp_block(&mut s) == 0 {
                    sync_error();
                    return 0;
                }
            }
            2 => {
                cp_dynamic(&mut s);
                if cp_block(&mut s) == 0 {
                    sync_error();
                    return 0;
                }
            }
            3 => {
                set_error(b"Detected unknown block type within input stream.\0");
                sync_error();
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
    ((*s.add(0) as u32) << 24)
        | ((*s.add(1) as u32) << 16)
        | ((*s.add(2) as u32) << 8)
        | (*s.add(3) as u32)
}

unsafe fn cp_chunk(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    let len = cp_make32(png.p);
    let start = png.p;
    let chunk_match = *start.add(4) == chunk[0]
        && *start.add(5) == chunk[1]
        && *start.add(6) == chunk[2]
        && *start.add(7) == chunk[3];
    if chunk_match && len >= minlen {
        let offset = (len + 12) as usize;
        if (png.p as usize + offset) <= png.end as usize {
            png.p = png.p.add(offset);
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    while (png.p as usize) < (png.end as usize) {
        let len = cp_make32(png.p);
        let start = png.p;
        png.p = png.p.add((len + 12) as usize);
        let chunk_match = *start.add(4) == chunk[0]
            && *start.add(5) == chunk[1]
            && *start.add(6) == chunk[2]
            && *start.add(7) == chunk[3];
        if chunk_match && len >= minlen && png.p as usize <= png.end as usize {
            return start.add(8);
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
        let filter_byte = *raw;
        raw = raw.add(1);
        match filter_byte {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    *raw.offset(x as isize) =
                        (*raw.offset(x as isize)).wrapping_add(*raw.offset((x - bpp) as isize));
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    *raw.offset(x as isize) =
                        (*raw.offset(x as isize)).wrapping_add(*raw.offset((x - bpp) as isize) / 2);
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
    raw = raw.add(len as usize);
    for _y in 1..h {
        let filter_byte = *raw;
        raw = raw.add(1);
        match filter_byte {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(0);
                    x += 1;
                }
                while x < len {
                    *raw.offset(x as isize) =
                        (*raw.offset(x as isize)).wrapping_add(*raw.offset((x - bpp) as isize));
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
                    let val = ((*raw.offset((x - bpp) as isize) as u16
                        + *prev.offset(x as isize) as u16)
                        / 2) as u8;
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(val);
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
                    *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(cp_paeth(
                        *raw.offset((x - bpp) as isize),
                        *prev.offset(x as isize),
                        *prev.offset((x - bpp) as isize),
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

unsafe fn cp_convert(bpp: i32, w: i32, h: i32, src_in: *mut u8, dst_in: *mut cp_pixel_t) {
    let mut src = src_in;
    let mut dst = dst_in;
    for _y in 0..h {
        src = src.add(1);
        for _x in 0..w {
            match bpp {
                1 => {
                    *dst = cp_make_pixel(*src.add(0), *src.add(0), *src.add(0));
                    dst = dst.add(1);
                }
                2 => {
                    *dst = cp_make_pixel_a(*src.add(0), *src.add(0), *src.add(0), *src.add(1));
                    dst = dst.add(1);
                }
                3 => {
                    *dst = cp_make_pixel(*src.add(0), *src.add(1), *src.add(2));
                    dst = dst.add(1);
                }
                4 => {
                    *dst = cp_make_pixel_a(*src.add(0), *src.add(1), *src.add(2), *src.add(3));
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
        return 255;
    }
    if (index as u32) >= trns_len {
        return 255;
    }
    *trns.offset(index as isize)
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
        src = src.add(1);
        for _x in 0..w {
            let c = *src as i32;
            let r = *plte.offset((c * 3) as isize);
            let g = *plte.offset((c * 3 + 1) as isize);
            let b = *plte.offset((c * 3 + 2) as isize);
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

#[no_mangle]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: i32) -> cp_image_t {
    let sig: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];
    let mut img = cp_image_t {
        w: 0,
        h: 0,
        pix: ptr::null_mut(),
    };
    let mut data: *mut u8 = ptr::null_mut();
    let mut datalen: i32 = 0;
    let mut png = CpRawPng {
        p: png_data,
        end: png_data.add(png_length as usize),
    };

    // Use a closure-style "goto cp_err" flow with a labeled block returning Result
    let result: Result<cp_image_t, ()> = (|| {
        // Check signature
        for i in 0..8 {
            if *png.p.add(i) != sig[i] {
                set_error(b"incorrect file signature (is this a png file?)\0");
                return Err(());
            }
        }
        png.p = png.p.add(8);

        let ihdr = cp_chunk(&mut png, b"IHDR", 13);
        if ihdr.is_null() {
            set_error(b"unable to find IHDR chunk\0");
            return Err(());
        }
        let bit_depth = *ihdr.add(8) as i32;
        let color_type = *ihdr.add(9) as i32;
        if bit_depth != 8 {
            set_error(b"only bit-depth of 8 is supported\0");
            return Err(());
        }
        let bpp: i32;
        match color_type {
            0 => bpp = 1,
            2 => bpp = 3,
            3 => bpp = 1,
            4 => bpp = 2,
            6 => bpp = 4,
            _ => {
                set_error(b"unknown color type\0");
                return Err(());
            }
        }
        let w = (cp_make32(ihdr) as i32) + 1;
        let h = cp_make32(ihdr.add(4)) as i32;
        if !(w >= 1) {
            set_error(b"invalid IHDR chunk found, image width was less than 1\0");
            return Err(());
        }
        if !(h >= 1) {
            set_error(b"invalid IHDR chunk found, image height was less than 1\0");
            return Err(());
        }
        let pix_size = std::mem::size_of::<cp_pixel_t>() as i64;
        if !((w as i64) * (h as i64) * pix_size < i32::MAX as i64) {
            set_error(b"image too large\0");
            return Err(());
        }
        let pix_bytes = (w * h) as usize * std::mem::size_of::<cp_pixel_t>();
        img.w = w - 1;
        img.h = h;
        let layout = std::alloc::Layout::from_size_align(pix_bytes, std::mem::align_of::<cp_pixel_t>())
            .unwrap();
        img.pix = std::alloc::alloc(layout) as *mut cp_pixel_t;
        if img.pix.is_null() {
            set_error(b"unable to allocate raw image space\0");
            return Err(());
        }

        let compression = *ihdr.add(10) as i32;
        let filter = *ihdr.add(11) as i32;
        let interlace = *ihdr.add(12) as i32;
        if compression != 0 {
            set_error(b"only standard compression DEFLATE is supported\0");
            return Err(());
        }
        if filter != 0 {
            set_error(b"only standard adaptive filtering is supported\0");
            return Err(());
        }
        if interlace != 0 {
            set_error(b"interlacing is not supported\0");
            return Err(());
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

        // Compute total IDAT length
        datalen = 0;
        {
            let mut idat = cp_find(&mut png, b"IDAT", 0);
            while !idat.is_null() {
                let len = cp_get_chunk_byte_length(idat);
                datalen += len as i32;
                idat = cp_chunk(&mut png, b"IDAT", 0);
            }
        }
        png.p = first;
        let data_layout =
            std::alloc::Layout::from_size_align(datalen.max(1) as usize, 1).unwrap();
        data = std::alloc::alloc(data_layout);
        let mut offset_v = 0i32;
        {
            let mut idat = cp_find(&mut png, b"IDAT", 0);
            while !idat.is_null() {
                let len = cp_get_chunk_byte_length(idat);
                ptr::copy_nonoverlapping(idat, data.add(offset_v as usize), len as usize);
                offset_v += len as i32;
                idat = cp_chunk(&mut png, b"IDAT", 0);
            }
        }
        if !(!data.is_null() && datalen >= 6) {
            set_error(b"corrupt zlib structure in DEFLATE stream\0");
            return Err(());
        }
        if !((*data.add(0) & 0x0f) == 0x08) {
            set_error(b"only zlib compression method (RFC 1950) is supported\0");
            return Err(());
        }
        if !((*data.add(0) & 0xf0) <= 0x70) {
            set_error(b"innapropriate window size detected\0");
            return Err(());
        }
        if !((*data.add(1) & 0x20) == 0) {
            set_error(b"preset dictionary is present and not supported\0");
            return Err(());
        }
        if !(cp_out_size(&img, 4) >= 1) {
            set_error(b"invalid image size found\0");
            return Err(());
        }
        if !(cp_out_size(&img, bpp) >= 1) {
            set_error(b"invalid image size found\0");
            return Err(());
        }
        let out = (img.pix as *mut u8)
            .add(cp_out_size(&img, 4) as usize - cp_out_size(&img, bpp) as usize);

        if cp_inflate(data.add(2), datalen - 6, out, pix_bytes as i32) == 0 {
            set_error(b"DEFLATE algorithm failed\0");
            return Err(());
        }
        if cp_unfilter(img.w, img.h, bpp, out) == 0 {
            set_error(b"invalid filter byte found\0");
            return Err(());
        }

        if color_type == 3 {
            if plte.is_null() {
                set_error(b"color type of indexed requires a PLTE chunk\0");
                return Err(());
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
        // success: free data only
        let data_layout2 =
            std::alloc::Layout::from_size_align(datalen.max(1) as usize, 1).unwrap();
        std::alloc::dealloc(data, data_layout2);
        data = ptr::null_mut();
        Ok(cp_image_t {
            w: img.w,
            h: img.h,
            pix: img.pix,
        })
    })();

    sync_error();
    match result {
        Ok(image) => image,
        Err(_) => {
            if !data.is_null() {
                let data_layout =
                    std::alloc::Layout::from_size_align(datalen.max(1) as usize, 1).unwrap();
                std::alloc::dealloc(data, data_layout);
            }
            if !img.pix.is_null() {
                let pix_bytes = (img.w + 1) as usize * img.h as usize
                    * std::mem::size_of::<cp_pixel_t>();
                if pix_bytes > 0 {
                    let layout = std::alloc::Layout::from_size_align(
                        pix_bytes,
                        std::mem::align_of::<cp_pixel_t>(),
                    )
                    .unwrap();
                    std::alloc::dealloc(img.pix as *mut u8, layout);
                }
            }
            img.pix = ptr::null_mut();
            img
        }
    }
}
