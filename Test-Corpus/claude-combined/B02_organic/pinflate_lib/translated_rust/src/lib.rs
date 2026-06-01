#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(static_mut_refs)]

use std::ffi::{c_char, c_int, c_void};

// ---- Public globals (mirror C globals with external linkage) ----

#[no_mangle]
pub static mut cp_error_reason: *const c_char = std::ptr::null();

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
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67,
    83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
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

// ---- Internal state ----

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

impl CpState {
    fn zeroed() -> Self {
        CpState {
            bits: 0,
            count: 0,
            words: std::ptr::null_mut(),
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

// ---- Helpers ----

fn cp_would_overflow(s: &CpState, num_bits: c_int) -> c_int {
    if (s.bits_left + s.count) - num_bits < 0 {
        1
    } else {
        0
    }
}

unsafe fn cp_ptr(s: &CpState) -> *mut c_char {
    debug_assert!(s.bits_left & 7 == 0);
    let base = s.words.add(s.word_index as usize) as *mut c_char;
    base.offset(-((s.count / 8) as isize))
}

fn cp_peak_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
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

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    debug_assert!(s.count >= num_bits_to_read);
    // Match C: (((uint64_t)1 << num_bits_to_read) - 1)
    // For num_bits_to_read == 0, this is 0. Avoid Rust panic for shift-by-64
    // via wrapping/checked behavior; C asserts num_bits <= 32.
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

fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
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

/// Build a Huffman tree.
/// `tree` is a slice into the state's tree storage; `lens` is a slice of code lengths.
/// If `update_lookup` is true, also updates `s.lookup`.
fn cp_build(
    s: Option<&mut CpState>,
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
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    let lookup_ptr: *mut [u16; 1 << 9] = match s {
        Some(state) => {
            for v in state.lookup.iter_mut() {
                *v = 0;
            }
            &mut state.lookup as *mut _
        }
        None => std::ptr::null_mut(),
    };
    for i in 0..sym_count {
        let len = lens[i] as i32;
        if len != 0 {
            debug_assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as usize;
            first[len as usize] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if !lookup_ptr.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                while j < (1 << 9) {
                    unsafe {
                        (*lookup_ptr)[j as usize] =
                            (((len as u32) << 9) | (i as u32)) as u16;
                    }
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut CpState) -> c_int {
    let extra = s.count & 7;
    cp_read_bits(s, extra);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    if !(s.bits_left / 8 <= LEN as i32) {
        cp_error_reason =
            b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    let p = cp_ptr(s);
    std::ptr::copy_nonoverlapping(p as *const u8, s.out as *mut u8, LEN as usize);
    s.out = s.out.add(LEN as usize);
    1
}

fn cp_fixed(s: &mut CpState) -> c_int {
    // s->nlit = cp_build(s, s->lit, cp_fixed_table, 288);
    // We need to split the borrow: pass the fixed table separately
    let table_ptr: *const u8 = unsafe { cp_fixed_table.as_ptr() };
    let lens_first: &[u8] = unsafe { std::slice::from_raw_parts(table_ptr, 288) };
    let lens_second: &[u8] = unsafe { std::slice::from_raw_parts(table_ptr.add(288), 32) };

    // For nlit: we need both `s` (mutable, for lookup table) AND s->lit.
    // Take lit out by raw pointer trick.
    let lit_ptr = s.lit.as_mut_ptr();
    let lit_slice: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(lit_ptr, 288) };
    let nlit = cp_build(Some(s), lit_slice, lens_first, 288);
    s.nlit = nlit as u32;

    let dst_ptr = s.dst.as_mut_ptr();
    let dst_slice: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(dst_ptr, 32) };
    let ndst = cp_build(None, dst_slice, lens_second, 32);
    s.ndst = ndst as u32;

    1
}

fn cp_decode(s: &mut CpState, tree: &[u32], hi: i32) -> i32 {
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
    let len = 32 - (key & 0xF);
    debug_assert!((search >> len) == (key >> len));
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

fn cp_dynamic(s: &mut CpState) -> c_int {
    let mut lenlens: [u8; 19] = [0; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen as usize {
        let bits = cp_read_bits(s, 3) as u8;
        let idx = unsafe { cp_permutation_order[i] } as usize;
        lenlens[idx] = bits;
    }
    {
        let len_ptr = s.len.as_mut_ptr();
        let len_slice: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(len_ptr, 19) };
        let nlen_built = cp_build(None, len_slice, &lenlens, 19);
        s.nlen = nlen_built as u32;
    }
    let mut lens: [u8; 288 + 32] = [0; 288 + 32];
    let mut n: i32 = 0;
    while n < nlit + ndst {
        let sym = {
            // cp_decode reads via cp_peak_bits/consume on `s`, but we need to pass &s.len
            // Snapshot the relevant tree.
            let tree_copy: [u32; 19] = s.len;
            cp_decode(s, &tree_copy, s.nlen as i32)
        };
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
        let lit_ptr = s.lit.as_mut_ptr();
        let lit_slice: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(lit_ptr, 288) };
        let nlit_built = cp_build(Some(s), lit_slice, &lens[..nlit as usize], nlit as usize);
        s.nlit = nlit_built as u32;
    }
    {
        let dst_ptr = s.dst.as_mut_ptr();
        let dst_slice: &mut [u32] = unsafe { std::slice::from_raw_parts_mut(dst_ptr, 32) };
        let ndst_built = cp_build(
            None,
            dst_slice,
            &lens[nlit as usize..(nlit + ndst) as usize],
            ndst as usize,
        );
        s.ndst = ndst_built as u32;
    }
    1
}

unsafe fn cp_block(s: &mut CpState) -> c_int {
    loop {
        let symbol = {
            let tree_copy: [u32; 288] = s.lit;
            cp_decode(s, &tree_copy, s.nlit as i32)
        };
        if symbol < 256 {
            if !(s.out.offset(1) <= s.out_end) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                return 0;
            }
            *s.out = symbol as c_char;
            s.out = s.out.add(1);
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = cp_read_bits(s, cp_len_extra_bits[symbol as usize] as i32)
                as i32
                + cp_len_base[symbol as usize] as i32;
            let distance_symbol = {
                let tree_copy: [u32; 32] = s.dst;
                cp_decode(s, &tree_copy, s.ndst as i32)
            };
            let backwards_distance =
                cp_read_bits(s, cp_dist_extra_bits[distance_symbol as usize] as i32) as i32
                    + cp_dist_base[distance_symbol as usize] as i32;
            if !(s.out.offset(-(backwards_distance as isize)) >= s.begin) {
                cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                return 0;
            }
            if !(s.out.offset(length as isize) <= s.out_end) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                return 0;
            }
            let src = s.out.offset(-(backwards_distance as isize));
            let dst = s.out;
            s.out = s.out.offset(length as isize);
            match backwards_distance {
                1 => {
                    let val = *src as u8;
                    std::ptr::write_bytes(dst as *mut u8, val, length as usize);
                }
                _ => {
                    let mut src = src;
                    let mut dst = dst;
                    let mut length = length;
                    while length != 0 {
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
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

#[no_mangle]
pub unsafe extern "C" fn pinflate(
    in_ptr: *mut c_void,
    in_bytes: c_int,
    out_ptr: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut state_box = Box::new(CpState::zeroed());
    let s: &mut CpState = &mut *state_box;
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;
    let in_addr = in_ptr as usize;
    let first_bytes: c_int = (((in_addr + 3) & !3usize) - in_addr) as c_int;
    s.words = (in_ptr as *mut c_char).add(first_bytes as usize) as *mut u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes: c_int = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        let b = *(in_ptr as *const u8).add(i as usize);
        s.bits |= (b as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        // Match C: ((uint8_t *)in)[in_bytes - last_bytes + i] << (i * 8)
        // C does this with default int promotion; the shift is on an int.
        let b = *(in_ptr as *const u8).add((in_bytes - last_bytes + i) as usize);
        s.final_word |= (b as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out_ptr as *mut c_char;
    s.out_end = s.out.add(out_bytes as usize);
    s.begin = out_ptr as *mut c_char;
    let mut count: i32 = 0;
    let mut bfinal: u32;
    loop {
        bfinal = cp_read_bits(s, 1);
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
                cp_error_reason =
                    b"Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
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
