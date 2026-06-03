// Rust translation of c_src/src/lib.c
//
// Mirrors the original C semantics, including raw-pointer FFI signatures, so
// the resulting cdylib has the same ABI as the C library.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

#[repr(C)]
#[derive(Copy, Clone, Default)]
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

// Public global error reason pointer to mirror `const char *cp_error_reason;`
#[no_mangle]
pub static mut cp_error_reason: *const c_char = ptr::null();

// `static mut` arrays mirroring the file-scope arrays in lib.c. They are not
// referenced from outside, but we keep them to mirror the original layout.
#[no_mangle]
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

#[no_mangle]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[no_mangle]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];

#[no_mangle]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59,
    67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

#[no_mangle]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

#[no_mangle]
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

impl cp_state_t {
    fn zeroed() -> Self {
        cp_state_t {
            bits: 0,
            count: 0,
            words: ptr::null_mut(),
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
fn cp_would_overflow(s: &cp_state_t, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

#[inline]
unsafe fn cp_ptr(s: &cp_state_t) -> *mut c_char {
    debug_assert!((s.bits_left & 7) == 0);
    // (char *)(s->words + s->word_index) - (s->count / 8)
    let base = s.words.offset(s.word_index as isize) as *mut c_char;
    base.offset(-((s.count / 8) as isize))
}

unsafe fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u64 {
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

fn cp_consume_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
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

unsafe fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
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

/// Build a Huffman tree. `s` is optional (used for the literal-table lookup
/// shortcut). When `s` is `None` the lookup table is left untouched.
unsafe fn cp_build(
    s: Option<&mut cp_state_t>,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];

    for n in 0..sym_count as usize {
        let l = *lens.add(n) as usize;
        counts[l] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }

    if let Some(ref s) = s {
        // Zero out the lookup table — std::ptr::write_bytes is the moral
        // equivalent of memset on a fixed-size array.
        let lookup_ptr = s.lookup.as_ptr() as *mut u16;
        ptr::write_bytes(lookup_ptr, 0, s.lookup.len());
    }

    // We can't use the &mut s reference twice without re-borrowing — convert
    // to a raw pointer for the inner write to the lookup table.
    let s_ptr: *mut cp_state_t = match s {
        Some(s) => s as *mut cp_state_t,
        None => ptr::null_mut(),
    };

    for i in 0..sym_count as usize {
        let len = *lens.add(i) as i32;
        if len != 0 {
            debug_assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as usize;
            first[len as usize] += 1;
            *tree.add(slot) = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if !s_ptr.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                while j < (1 << 9) {
                    (*s_ptr).lookup[j] = ((len << 9) as u16) | (i as u16);
                    j += 1usize << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut cp_state_t) -> c_int {
    cp_read_bits(s, s.count & 7);
    let len_v = cp_read_bits(s, 16) as u16;
    let nlen_v = cp_read_bits(s, 16) as u16;

    if len_v != !nlen_v {
        cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    if !((s.bits_left / 8) <= len_v as c_int) {
        cp_error_reason =
            b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p as *const u8, s.out as *mut u8, len_v as usize);
    s.out = s.out.add(len_v as usize);
    1
}

unsafe fn cp_fixed(s: &mut cp_state_t) -> c_int {
    let lit_ptr = s.lit.as_mut_ptr();
    let dst_ptr = s.dst.as_mut_ptr();
    let table_ptr: *const u8 = ptr::addr_of!(cp_fixed_table) as *const u8;
    s.nlit = cp_build(Some(&mut *(s as *mut cp_state_t)), lit_ptr, table_ptr, 288) as u32;
    s.ndst = cp_build(None, dst_ptr, table_ptr.add(288), 32) as u32;
    1
}

unsafe fn cp_decode(s: &mut cp_state_t, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.offset((lo - 1) as isize);
    let len = 32 - (key & 0xF);
    debug_assert!((search >> len) == (key >> len));
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: &mut cp_state_t) -> c_int {
    let mut lenlens = [0u8; 19];
    let nlit: c_int = 257i32 + cp_read_bits(s, 5) as c_int;
    let ndst: c_int = 1i32 + cp_read_bits(s, 5) as c_int;
    let nlen: c_int = 4i32 + cp_read_bits(s, 4) as c_int;
    for i in 0..nlen as usize {
        let idx = cp_permutation_order[i] as usize;
        lenlens[idx] = cp_read_bits(s, 3) as u8;
    }
    let len_ptr = s.len.as_mut_ptr();
    s.nlen = cp_build(None, len_ptr, lenlens.as_ptr(), 19) as u32;

    let mut lens = [0u8; 288 + 32];
    let mut n: c_int = 0;
    while n < nlit + ndst {
        // cp_decode borrows s mutably; we need to release after the call.
        let len_tree_ptr: *const u32 = s.len.as_ptr();
        let nlen_count = s.nlen as c_int;
        let sym = cp_decode(s, len_tree_ptr, nlen_count);
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as c_int;
                while i > 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    n += 1;
                    i -= 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as c_int;
                while i > 0 {
                    lens[n as usize] = 0;
                    n += 1;
                    i -= 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as c_int;
                while i > 0 {
                    lens[n as usize] = 0;
                    n += 1;
                    i -= 1;
                }
            }
            _ => {
                lens[n as usize] = sym as u8;
                n += 1;
            }
        }
    }

    let lit_ptr = s.lit.as_mut_ptr();
    let dst_ptr = s.dst.as_mut_ptr();
    s.nlit = cp_build(Some(&mut *(s as *mut cp_state_t)), lit_ptr, lens.as_ptr(), nlit) as u32;
    s.ndst = cp_build(None, dst_ptr, lens.as_ptr().add(nlit as usize), ndst) as u32;
    1
}

unsafe fn cp_block(s: &mut cp_state_t) -> c_int {
    loop {
        let lit_ptr: *const u32 = s.lit.as_ptr();
        let nlit_count = s.nlit as c_int;
        let mut symbol = cp_decode(s, lit_ptr, nlit_count);
        if symbol < 256 {
            if !(s.out.add(1) <= s.out_end) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                return 0;
            }
            *s.out = symbol as c_char;
            s.out = s.out.add(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length = cp_read_bits(s, cp_len_extra_bits[symbol as usize] as c_int) as c_int
                + cp_len_base[symbol as usize] as c_int;
            let dst_ptr: *const u32 = s.dst.as_ptr();
            let ndst_count = s.ndst as c_int;
            let distance_symbol = cp_decode(s, dst_ptr, ndst_count);
            let backwards_distance = cp_read_bits(
                s,
                cp_dist_extra_bits[distance_symbol as usize] as c_int,
            ) as c_int
                + cp_dist_base[distance_symbol as usize] as c_int;

            if !(s.out.offset(-(backwards_distance as isize)) >= s.begin) {
                cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                return 0;
            }
            if !(s.out.add(length as usize) <= s.out_end) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                return 0;
            }
            let mut src = s.out.offset(-(backwards_distance as isize));
            let mut dst = s.out;
            s.out = s.out.add(length as usize);
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst as *mut u8, *src as u8, length as usize);
                }
                _ => {
                    let mut remaining = length;
                    while remaining > 0 {
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

#[no_mangle]
pub unsafe extern "C" fn cp_inflate(
    in_ptr: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    // Allocate state on the heap to mirror calloc(1, sizeof(cp_state_t)).
    let mut state_box: Box<cp_state_t> = Box::new(cp_state_t::zeroed());
    let s: &mut cp_state_t = &mut state_box;

    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;

    let in_addr = in_ptr as usize;
    let first_bytes = (((in_addr + 3) & !3usize) - in_addr) as c_int;
    s.words = (in_ptr as *mut u8).offset(first_bytes as isize) as *mut u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;

    for i in 0..first_bytes as usize {
        let byte = *(in_ptr as *const u8).add(i);
        s.bits |= (byte as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes as usize {
        let byte = *(in_ptr as *const u8).add((in_bytes - last_bytes) as usize + i);
        s.final_word |= (byte as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out as *mut c_char;
    s.out_end = (out as *mut c_char).add(out_bytes as usize);
    s.begin = out as *mut c_char;

    let mut count: c_int = 0;
    loop {
        let bfinal = cp_read_bits(s, 1);
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
                cp_error_reason = b"Detected unknown block type within input stream.\0".as_ptr()
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
    let _ = count;
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

#[repr(C)]
struct cp_raw_png_t {
    p: *const u8,
    end: *const u8,
}

unsafe fn cp_make32(s: *const u8) -> u32 {
    ((*s.add(0) as u32) << 24)
        | ((*s.add(1) as u32) << 16)
        | ((*s.add(2) as u32) << 8)
        | (*s.add(3) as u32)
}

unsafe fn cp_chunk(
    png: &mut cp_raw_png_t,
    chunk: *const c_char,
    minlen: u32,
) -> *const u8 {
    let len = cp_make32(png.p);
    let start = png.p;
    if memcmp_eq(start.add(4) as *const c_char, chunk, 4) && len >= minlen {
        let offset = (len + 12) as isize;
        if png.p.offset(offset) <= png.end {
            png.p = png.p.offset(offset);
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(
    png: &mut cp_raw_png_t,
    chunk: *const c_char,
    minlen: u32,
) -> *const u8 {
    while png.p < png.end {
        let len = cp_make32(png.p);
        let start = png.p;
        png.p = png.p.offset((len + 12) as isize);
        if memcmp_eq(start.add(4) as *const c_char, chunk, 4)
            && len >= minlen
            && png.p <= png.end
        {
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn memcmp_eq(a: *const c_char, b: *const c_char, n: usize) -> bool {
    for i in 0..n {
        if *a.add(i) != *b.add(i) {
            return false;
        }
    }
    true
}

#[allow(dead_code)]
unsafe fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw_in: *mut u8) -> c_int {
    let len = (w * bpp) as isize;
    let mut raw = raw_in;
    if h > 0 {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                let mut x = bpp as isize;
                while x < len {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(*raw.offset(x - bpp as isize));
                    x += 1;
                }
            }
            2 => {}
            3 => {
                let mut x = bpp as isize;
                while x < len {
                    *raw.offset(x) =
                        (*raw.offset(x)).wrapping_add(*raw.offset(x - bpp as isize) / 2);
                    x += 1;
                }
            }
            4 => {
                let mut x = bpp as isize;
                while x < len {
                    *raw.offset(x) = (*raw.offset(x))
                        .wrapping_add(cp_paeth(*raw.offset(x - bpp as isize), 0, 0));
                    x += 1;
                }
            }
            _ => return 0,
        }
    }
    let mut prev = raw;
    raw = raw.offset(len);
    let mut y = 1;
    while y < h {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                let mut x: isize = 0;
                while x < bpp as isize {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(0);
                    x += 1;
                }
                while x < len {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(*raw.offset(x - bpp as isize));
                    x += 1;
                }
            }
            2 => {
                let mut x: isize = 0;
                while x < bpp as isize {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(*prev.offset(x));
                    x += 1;
                }
                while x < len {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(*prev.offset(x));
                    x += 1;
                }
            }
            3 => {
                let mut x: isize = 0;
                while x < bpp as isize {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(*prev.offset(x) / 2);
                    x += 1;
                }
                while x < len {
                    let v = (*raw.offset(x - bpp as isize) as u16 + *prev.offset(x) as u16) / 2;
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(v as u8);
                    x += 1;
                }
            }
            4 => {
                let mut x: isize = 0;
                while x < bpp as isize {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(*prev.offset(x));
                    x += 1;
                }
                while x < len {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(cp_paeth(
                        *raw.offset(x - bpp as isize),
                        *prev.offset(x),
                        *prev.offset(x - bpp as isize),
                    ));
                    x += 1;
                }
            }
            _ => return 0,
        }
        prev = raw;
        raw = raw.offset(len);
        y += 1;
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn convert_pix(
    bpp: c_int,
    w: c_int,
    h: c_int,
    src_in: *mut u8,
    dst_in: *mut cp_pixel_t,
) {
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

// Suppress dead-code warnings for helpers that aren't yet exercised by callers
// in this translation unit but mirror static helpers in the C source.
#[allow(dead_code)]
fn _silence_unused() {
    let _ = cp_chunk;
    let _ = cp_find;
    let _ = cp_unfilter;
}
