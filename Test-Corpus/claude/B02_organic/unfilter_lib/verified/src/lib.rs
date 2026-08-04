use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Public data (must be exported with the same names as the C .so)
// ---------------------------------------------------------------------------

#[no_mangle]
pub static mut cp_error_reason: *const c_char = core::ptr::null();

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

// ---------------------------------------------------------------------------
// cp_state_t (matches C layout)
// ---------------------------------------------------------------------------

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

unsafe fn cp_would_overflow(s: *mut CpState, num_bits: c_int) -> c_int {
    if ((*s).bits_left + (*s).count) - num_bits < 0 {
        1
    } else {
        0
    }
}

unsafe fn cp_ptr(s: *mut CpState) -> *mut c_char {
    debug_assert!((*s).bits_left & 7 == 0);
    let base = (*s).words.offset((*s).word_index as isize) as *mut c_char;
    base.offset(-(((*s).count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut CpState, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.offset((*s).word_index as isize);
            (*s).word_index += 1;
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count += 32;
            debug_assert!((*s).word_index <= (*s).word_count);
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
    debug_assert!((*s).count >= num_bits_to_read);
    let mask: u64 = if num_bits_to_read >= 64 {
        u64::MAX
    } else {
        (1u64 << num_bits_to_read) - 1
    };
    let bits = ((*s).bits & mask) as u32;
    if num_bits_to_read >= 64 {
        (*s).bits = 0;
    } else {
        (*s).bits >>= num_bits_to_read;
    }
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    debug_assert!((*s).bits_left > 0);
    debug_assert!((*s).count <= 64);
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
    s: *mut CpState,
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
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if !s.is_null() {
        for v in (*s).lookup.iter_mut() {
            *v = 0;
        }
    }
    for i in 0..sym_count {
        let len = *lens.offset(i as isize) as c_int;
        if len != 0 {
            debug_assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as u32;
            first[len as usize] += 1;
            let entry: u32 = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            *tree.offset(slot as isize) = entry;
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                while j < (1 << 9) {
                    (*s).lookup[j] = (((len as u32) << 9) | (i as u32)) as u16;
                    j += 1usize << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: *mut CpState) -> c_int {
    cp_read_bits(s, (*s).count & 7);
    let len_val = cp_read_bits(s, 16) as u16;
    let nlen_val = cp_read_bits(s, 16) as u16;
    if !(len_val == !nlen_val) {
        cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    if !((*s).bits_left / 8 <= len_val as c_int) {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    let p = cp_ptr(s);
    core::ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, len_val as usize);
    (*s).out = (*s).out.offset(len_val as isize);
    1
}

unsafe fn cp_fixed(s: *mut CpState) -> c_int {
    let lit_ptr = (*s).lit.as_mut_ptr();
    let dst_ptr = (*s).dst.as_mut_ptr();
    let table_ptr = core::ptr::addr_of_mut!(cp_fixed_table) as *mut u8;
    (*s).nlit = cp_build(s, lit_ptr, table_ptr, 288) as u32;
    (*s).ndst = cp_build(core::ptr::null_mut(), dst_ptr, table_ptr.offset(288), 32) as u32;
    1
}

unsafe fn cp_decode(s: *mut CpState, tree: *mut u32, mut hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;
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
    let _len = 32u32 - (key & 0xF);
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: *mut CpState) -> c_int {
    let mut lenlens: [u8; 19] = [0; 19];
    let nlit = 257 + cp_read_bits(s, 5) as c_int;
    let ndst = 1 + cp_read_bits(s, 5) as c_int;
    let nlen = 4 + cp_read_bits(s, 4) as c_int;
    for i in 0..nlen {
        let idx = cp_permutation_order[i as usize] as usize;
        lenlens[idx] = cp_read_bits(s, 3) as u8;
    }
    let len_ptr = (*s).len.as_mut_ptr();
    (*s).nlen = cp_build(core::ptr::null_mut(), len_ptr, lenlens.as_ptr(), 19) as u32;

    let mut lens: [u8; 288 + 32] = [0; 288 + 32];
    let mut n: c_int = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, len_ptr, (*s).nlen as c_int);
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
    let lit_ptr = (*s).lit.as_mut_ptr();
    let dst_ptr = (*s).dst.as_mut_ptr();
    (*s).nlit = cp_build(s, lit_ptr, lens.as_ptr(), nlit) as u32;
    (*s).ndst = cp_build(core::ptr::null_mut(), dst_ptr, lens.as_ptr().offset(nlit as isize), ndst) as u32;
    1
}

unsafe fn cp_block(s: *mut CpState) -> c_int {
    loop {
        let lit_ptr = (*s).lit.as_mut_ptr();
        let symbol = cp_decode(s, lit_ptr, (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.offset(1) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.offset(1);
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let len_extra = cp_len_extra_bits[symbol as usize] as c_int;
            let length = cp_read_bits(s, len_extra) as c_int + cp_len_base[symbol as usize] as c_int;
            let dst_ptr = (*s).dst.as_mut_ptr();
            let distance_symbol = cp_decode(s, dst_ptr, (*s).ndst as c_int);
            let dist_extra = cp_dist_extra_bits[distance_symbol as usize] as c_int;
            let backwards_distance =
                cp_read_bits(s, dist_extra) as c_int + cp_dist_base[distance_symbol as usize] as c_int;
            if !((*s).out.offset(-(backwards_distance as isize)) >= (*s).begin) {
                cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                return 0;
            }
            if !((*s).out.offset(length as isize) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                return 0;
            }
            let mut src = (*s).out.offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.offset(length as isize);
            if backwards_distance == 1 {
                core::ptr::write_bytes(dst as *mut u8, *src as u8, length as usize);
            } else {
                let mut remaining = length;
                while remaining > 0 {
                    *dst = *src;
                    dst = dst.offset(1);
                    src = src.offset(1);
                    remaining -= 1;
                }
            }
        } else {
            break;
        }
    }
    1
}

extern "C" {
    fn calloc(num: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

#[no_mangle]
pub unsafe extern "C" fn cp_inflate(
    in_: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let s = calloc(1, core::mem::size_of::<CpState>()) as *mut CpState;
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes * 8;
    let in_addr = in_ as usize;
    let first_bytes = ((in_addr + 3) & !3usize).wrapping_sub(in_addr) as c_int;
    (*s).words = (in_ as *mut c_char).offset(first_bytes as isize) as *mut u32;
    (*s).word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        let b = *((in_ as *mut u8).offset(i as isize)) as u64;
        (*s).bits |= b << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        let b = *((in_ as *mut u8).offset((in_bytes - last_bytes + i) as isize)) as u32;
        (*s).final_word |= b << (i * 8);
    }
    (*s).count = first_bytes * 8;
    (*s).out = out as *mut c_char;
    (*s).out_end = (*s).out.offset(out_bytes as isize);
    (*s).begin = out as *mut c_char;
    let mut count = 0;
    let mut bfinal: c_int;
    loop {
        bfinal = cp_read_bits(s, 1) as c_int;
        let btype = cp_read_bits(s, 2) as c_int;
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    free(s as *mut c_void);
                    return 0;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    free(s as *mut c_void);
                    return 0;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    free(s as *mut c_void);
                    return 0;
                }
            }
            3 => {
                cp_error_reason =
                    b"Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
                free(s as *mut c_void);
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
    free(s as *mut c_void);
    1
}

// ---------------------------------------------------------------------------
// Paeth and unfilter (public API from lib.h)
// ---------------------------------------------------------------------------

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

#[no_mangle]
pub unsafe extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len: isize = (w as isize) * (bpp as isize);
    let bpp_isize: isize = bpp as isize;

    let mut raw_ofs: isize = 0;

    if h > 0 {
        let filter = *raw.offset(raw_ofs);
        raw_ofs += 1;
        match filter {
            0 => {}
            1 => {
                let mut x = bpp_isize;
                while x < len {
                    let v = *raw.offset(raw_ofs + x - bpp_isize);
                    let cur = *raw.offset(raw_ofs + x);
                    *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    x += 1;
                }
            }
            2 => {}
            3 => {
                let mut x = bpp_isize;
                while x < len {
                    let v = *raw.offset(raw_ofs + x - bpp_isize) / 2;
                    let cur = *raw.offset(raw_ofs + x);
                    *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    x += 1;
                }
            }
            4 => {
                let mut x = bpp_isize;
                while x < len {
                    let a = *raw.offset(raw_ofs + x - bpp_isize);
                    let v = cp_paeth(a, 0, 0);
                    let cur = *raw.offset(raw_ofs + x);
                    *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    x += 1;
                }
            }
            _ => return 0,
        }
    }

    let mut prev_ofs: isize = raw_ofs;
    raw_ofs += len;

    let mut y: c_int = 1;
    while y < h {
        let filter = *raw.offset(raw_ofs);
        raw_ofs += 1;
        match filter {
            0 => {}
            1 => {
                let mut x: isize = 0;
                while x < bpp_isize {
                    x += 1;
                }
                while x < len {
                    let v = *raw.offset(raw_ofs + x - bpp_isize);
                    let cur = *raw.offset(raw_ofs + x);
                    *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    x += 1;
                }
            }
            2 => {
                let mut x: isize = 0;
                while x < bpp_isize {
                    let v = *raw.offset(prev_ofs + x);
                    let cur = *raw.offset(raw_ofs + x);
                    *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let v = *raw.offset(prev_ofs + x);
                    let cur = *raw.offset(raw_ofs + x);
                    *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    x += 1;
                }
            }
            3 => {
                let mut x: isize = 0;
                while x < bpp_isize {
                    let v = *raw.offset(prev_ofs + x) / 2;
                    let cur = *raw.offset(raw_ofs + x);
                    *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let a = *raw.offset(raw_ofs + x - bpp_isize) as u32;
                    let b = *raw.offset(prev_ofs + x) as u32;
                    let v = ((a + b) / 2) as u8;
                    let cur = *raw.offset(raw_ofs + x);
                    *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    x += 1;
                }
            }
            4 => {
                let mut x: isize = 0;
                while x < bpp_isize {
                    let v = *raw.offset(prev_ofs + x);
                    let cur = *raw.offset(raw_ofs + x);
                    *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    x += 1;
                }
                while x < len {
                    let a = *raw.offset(raw_ofs + x - bpp_isize);
                    let b = *raw.offset(prev_ofs + x);
                    let c = *raw.offset(prev_ofs + x - bpp_isize);
                    let v = cp_paeth(a, b, c);
                    let cur = *raw.offset(raw_ofs + x);
                    *raw.offset(raw_ofs + x) = cur.wrapping_add(v);
                    x += 1;
                }
            }
            _ => return 0,
        }
        y += 1;
        prev_ofs = raw_ofs;
        raw_ofs += len;
    }

    1
}
