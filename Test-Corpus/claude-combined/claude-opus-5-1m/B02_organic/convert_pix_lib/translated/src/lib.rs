#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::c_char;
use std::os::raw::c_int;
use std::ptr;

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
pub static mut cp_error_reason: *const c_char = ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = [
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8,
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
];

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

unsafe fn cp_would_overflow(s: *mut cp_state_t, num_bits: c_int) -> c_int {
    let s = &*s;
    if (s.bits_left + s.count) - num_bits < 0 {
        1
    } else {
        0
    }
}

unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut c_char {
    let s = &mut *s;
    debug_assert!((s.bits_left & 7) == 0);
    // (char *)(s->words + s->word_index) - (s->count / 8)
    let p = s.words.offset(s.word_index as isize) as *mut c_char;
    p.offset(-((s.count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    let s = &mut *s;
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

unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    let s = &mut *s;
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

unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    {
        let sr = &*s;
        debug_assert!(sr.bits_left > 0);
        debug_assert!(sr.count <= 64);
    }
    debug_assert!(cp_would_overflow(s, num_bits_to_read) == 0);
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
    s: *mut cp_state_t,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    let mut codes: [c_int; 16] = [0; 16];
    let mut first: [c_int; 16] = [0; 16];
    let mut counts: [c_int; 16] = [0; 16];
    for n in 0..sym_count {
        let l = *lens.offset(n as isize) as usize;
        counts[l] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if !s.is_null() {
        let sr = &mut *s;
        for v in sr.lookup.iter_mut() {
            *v = 0;
        }
    }
    for i in 0..sym_count {
        let len = *lens.offset(i as isize) as c_int;
        if len != 0 {
            debug_assert!(len < 16);
            let li = len as usize;
            let code = codes[li] as u32;
            codes[li] += 1;
            let slot = first[li] as u32;
            first[li] += 1;
            *tree.offset(slot as isize) = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let sr = &mut *s;
                let shift: u32 = 16u32 - (len as u32);
                let mut j: usize = (cp_rev16(code) >> shift) as usize;
                while j < (1 << 9) {
                    sr.lookup[j] = (((len as u32) << 9) | (i as u32)) as u16;
                    j += 1usize << (len as usize);
                }
            }
        }
    }
    let max_index = first[15];
    max_index
}

unsafe fn cp_stored(s: *mut cp_state_t) -> c_int {
    let count_val = (*s).count;
    cp_read_bits(s, count_val & 7);
    let len_val = cp_read_bits(s, 16) as u16;
    let nlen_val = cp_read_bits(s, 16) as u16;
    if !(len_val == !nlen_val) {
        cp_error_reason =
            b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0"
                .as_ptr() as *const c_char;
        return 0;
    }
    if !((*s).bits_left / 8 <= len_val as c_int) {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, len_val as usize);
    (*s).out = (*s).out.offset(len_val as isize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> c_int {
    let nlit = cp_build(
        s,
        (*s).lit.as_mut_ptr(),
        (&raw mut cp_fixed_table) as *const u8,
        288,
    );
    (*s).nlit = nlit as u32;
    let ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        ((&raw mut cp_fixed_table) as *const u8).offset(288),
        32,
    );
    (*s).ndst = ndst as u32;
    1
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *mut u32, hi_in: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    let mut hi: c_int = hi_in;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.offset((lo - 1) as isize);
    let _len = 32 - (key & 0xF);
    debug_assert!((search >> _len) == (key >> _len));
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> c_int {
    let mut lenlens: [u8; 19] = [0; 19];
    let nlit: c_int = 257 + cp_read_bits(s, 5) as c_int;
    let ndst: c_int = 1 + cp_read_bits(s, 5) as c_int;
    let nlen: c_int = 4 + cp_read_bits(s, 4) as c_int;
    for i in 0..nlen {
        let idx = cp_permutation_order[i as usize] as usize;
        lenlens[idx] = cp_read_bits(s, 3) as u8;
    }
    let nlen_built = cp_build(
        ptr::null_mut(),
        (*s).len.as_mut_ptr(),
        lenlens.as_ptr(),
        19,
    );
    (*s).nlen = nlen_built as u32;
    let mut lens: [u8; 288 + 32] = [0; 288 + 32];
    let mut n: c_int = 0;
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
    let nlit_built = cp_build(s, (*s).lit.as_mut_ptr(), lens.as_ptr(), nlit);
    (*s).nlit = nlit_built as u32;
    let nlit_off: isize = nlit as isize;
    let ndst_built = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        lens.as_ptr().offset(nlit_off),
        ndst,
    );
    (*s).ndst = ndst_built as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> c_int {
    loop {
        let symbol = cp_decode(s, (*s).lit.as_mut_ptr(), (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.offset(1) as usize <= (*s).out_end as usize) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr()
                        as *const c_char;
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.offset(1);
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = (cp_read_bits(s, cp_len_extra_bits[symbol as usize] as c_int)
                + cp_len_base[symbol as usize]) as c_int;
            let distance_symbol = cp_decode(s, (*s).dst.as_mut_ptr(), (*s).ndst as c_int);
            let backwards_distance = (cp_read_bits(
                s,
                cp_dist_extra_bits[distance_symbol as usize] as c_int,
            ) + cp_dist_base[distance_symbol as usize]) as c_int;
            if !((*s).out.offset(-(backwards_distance as isize)) as usize
                >= (*s).begin as usize)
            {
                cp_error_reason =
                    b"Attempted to write before out buffer (invalid backwards distance).\0"
                        .as_ptr() as *const c_char;
                return 0;
            }
            if !((*s).out.offset(length as isize) as usize <= (*s).out_end as usize) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr()
                        as *const c_char;
                return 0;
            }
            let mut src = (*s).out.offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.offset(length as isize);
            match backwards_distance {
                1 => {
                    let val = *src as u8;
                    ptr::write_bytes(dst as *mut u8, val, length as usize);
                }
                _ => {
                    let mut remaining = length;
                    while remaining != 0 {
                        *dst = *src;
                        dst = dst.offset(1);
                        src = src.offset(1);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    in_ptr: *mut std::ffi::c_void,
    in_bytes: c_int,
    out_ptr: *mut std::ffi::c_void,
    out_bytes: c_int,
) -> c_int {
    // calloc-allocated zero-initialized state
    let layout = std::alloc::Layout::new::<cp_state_t>();
    let s = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;
    if s.is_null() {
        return 0;
    }

    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes * 8;

    let in_addr = in_ptr as usize;
    let first_bytes: c_int = (((in_addr + 3) & !3usize) - in_addr) as c_int;

    (*s).words = (in_ptr as *mut c_char).offset(first_bytes as isize) as *mut u32;
    (*s).word_count = (in_bytes - first_bytes) / 4;
    let last_bytes: c_int = (in_bytes - first_bytes) & 3;

    for i in 0..first_bytes {
        let byte = *(in_ptr as *const u8).offset(i as isize);
        (*s).bits |= (byte as u64) << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        let byte = *(in_ptr as *const u8).offset((in_bytes - last_bytes + i) as isize);
        (*s).final_word |= (byte as u32) << (i * 8);
    }
    (*s).count = first_bytes * 8;

    (*s).out = out_ptr as *mut c_char;
    (*s).out_end = (*s).out.offset(out_bytes as isize);
    (*s).begin = out_ptr as *mut c_char;

    let mut count: c_int = 0;
    let mut bfinal: c_int;
    let result: c_int;
    loop {
        bfinal = cp_read_bits(s, 1) as c_int;
        let btype = cp_read_bits(s, 2) as c_int;
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    result = 0;
                    std::alloc::dealloc(s as *mut u8, layout);
                    return result;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    result = 0;
                    std::alloc::dealloc(s as *mut u8, layout);
                    return result;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    result = 0;
                    std::alloc::dealloc(s as *mut u8, layout);
                    return result;
                }
            }
            3 => {
                cp_error_reason =
                    b"Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
                result = 0;
                std::alloc::dealloc(s as *mut u8, layout);
                return result;
            }
            _ => {}
        }
        count += 1;
        if bfinal != 0 {
            break;
        }
    }
    let _ = count;
    std::alloc::dealloc(s as *mut u8, layout);
    1
}

#[unsafe(no_mangle)]
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
