#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// Global error reason pointer (matches the C `const char *cp_error_reason;`)
static mut CP_ERROR_REASON: *const c_char = ptr::null();

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
    len_arr: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

#[inline]
fn cp_would_overflow(s: &CpState, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

#[inline]
unsafe fn cp_ptr(s: &CpState) -> *mut c_char {
    // (char *)(s->words + s->word_index) - (s->count / 8)
    let p = s.words.add(s.word_index as usize) as *mut c_char;
    p.sub((s.count / 8) as usize)
}

#[inline]
unsafe fn cp_peak_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = *s.words.add(s.word_index as usize);
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

#[inline]
fn cp_consume_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
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

#[inline]
unsafe fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

#[inline]
fn cp_rev16(a: u32) -> u32 {
    let mut a = a;
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
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if !s.is_null() {
        for slot in (*s).lookup.iter_mut() {
            *slot = 0;
        }
    }
    for i in 0..sym_count {
        let len = *lens.offset(i as isize) as c_int;
        if len != 0 {
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as u32;
            first[len as usize] += 1;
            *tree.offset(slot as isize) =
                (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    (*s).lookup[j as usize] = ((len << 9) | i) as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut CpState) -> c_int {
    let bits_to_skip = s.count & 7;
    cp_read_bits(s, bits_to_skip);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        CP_ERROR_REASON = c"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.".as_ptr();
        return 0;
    }
    if !(s.bits_left / 8 <= LEN as c_int) {
        CP_ERROR_REASON = c"Stored block extends beyond end of input stream.".as_ptr();
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p, s.out, LEN as usize);
    s.out = s.out.add(LEN as usize);
    1
}

unsafe fn cp_fixed(s: &mut CpState) -> c_int {
    let lit_ptr = s.lit.as_mut_ptr();
    let dst_ptr = s.dst.as_mut_ptr();
    s.nlit = cp_build(s as *mut CpState, lit_ptr, CP_FIXED_TABLE.as_ptr(), 288) as u32;
    s.ndst = cp_build(
        ptr::null_mut(),
        dst_ptr,
        CP_FIXED_TABLE.as_ptr().add(288),
        32,
    ) as u32;
    1
}

unsafe fn cp_decode(s: &mut CpState, tree: *mut u32, hi_in: c_int) -> c_int {
    let mut hi = hi_in;
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
    let _len = 32 - (key & 0xF);
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: &mut CpState) -> c_int {
    let mut lenlens: [u8; 19] = [0; 19];
    let nlit = 257 + cp_read_bits(s, 5) as c_int;
    let ndst = 1 + cp_read_bits(s, 5) as c_int;
    let nlen = 4 + cp_read_bits(s, 4) as c_int;
    for i in 0..nlen as usize {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    let len_ptr = s.len_arr.as_mut_ptr();
    s.nlen = cp_build(ptr::null_mut(), len_ptr, lenlens.as_ptr(), 19) as u32;

    let mut lens: [u8; 288 + 32] = [0; 288 + 32];
    let total = nlit + ndst;
    let mut n: c_int = 0;
    while n < total {
        let sym = cp_decode(s, len_ptr, s.nlen as c_int);
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
    let lit_ptr = s.lit.as_mut_ptr();
    let dst_ptr = s.dst.as_mut_ptr();
    s.nlit = cp_build(s as *mut CpState, lit_ptr, lens.as_ptr(), nlit) as u32;
    s.ndst = cp_build(
        ptr::null_mut(),
        dst_ptr,
        lens.as_ptr().add(nlit as usize),
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: &mut CpState) -> c_int {
    loop {
        let lit_ptr = s.lit.as_mut_ptr();
        let symbol = cp_decode(s, lit_ptr, s.nlit as c_int);
        if symbol < 256 {
            if !(s.out.add(1) <= s.out_end) {
                CP_ERROR_REASON =
                    c"Attempted to overwrite out buffer while outputting a symbol.".as_ptr();
                return 0;
            }
            *s.out = symbol as c_char;
            s.out = s.out.add(1);
        } else if symbol > 256 {
            let symbol_idx = (symbol - 257) as usize;
            let mut length = (cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol_idx] as c_int)
                + CP_LEN_BASE[symbol_idx]) as c_int;
            let dst_ptr = s.dst.as_mut_ptr();
            let distance_symbol = cp_decode(s, dst_ptr, s.ndst as c_int) as usize;
            let backwards_distance = (cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol] as c_int)
                + CP_DIST_BASE[distance_symbol])
                as c_int;
            if !(s.out.offset(-(backwards_distance as isize)) >= s.begin) {
                CP_ERROR_REASON =
                    c"Attempted to write before out buffer (invalid backwards distance).".as_ptr();
                return 0;
            }
            if !(s.out.offset(length as isize) <= s.out_end) {
                CP_ERROR_REASON =
                    c"Attempted to overwrite out buffer while outputting a string.".as_ptr();
                return 0;
            }
            let mut src = s.out.offset(-(backwards_distance as isize));
            let mut dst = s.out;
            s.out = s.out.offset(length as isize);
            match backwards_distance {
                1 => {
                    let v = *src as u8;
                    ptr::write_bytes(dst, v, length as usize);
                }
                _ => {
                    while length != 0 {
                        length -= 1;
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
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
pub unsafe extern "C" fn pinflate(
    in_ptr: *mut c_void,
    in_bytes: c_int,
    out_ptr: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    // Allocate state on the heap (matches calloc semantics: zeroed)
    let layout = std::alloc::Layout::new::<CpState>();
    let s_raw = std::alloc::alloc_zeroed(layout) as *mut CpState;
    if s_raw.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    let s = &mut *s_raw;

    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;

    let in_addr = in_ptr as usize;
    let first_bytes_usz = ((in_addr + 3) & !3usize) - in_addr;
    let first_bytes = first_bytes_usz as c_int;
    s.words = (in_ptr as *mut c_char).add(first_bytes as usize) as *mut u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    let in_u8 = in_ptr as *const u8;
    for i in 0..first_bytes {
        let b = *in_u8.offset(i as isize) as u64;
        s.bits |= b << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        let b = *in_u8.offset((in_bytes - last_bytes + i) as isize) as u32;
        s.final_word |= b << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out_ptr as *mut c_char;
    s.out_end = s.out.add(out_bytes as usize);
    s.begin = out_ptr as *mut c_char;

    let mut count: c_int = 0;
    let mut bfinal: c_int;
    loop {
        bfinal = cp_read_bits(s, 1) as c_int;
        let btype = cp_read_bits(s, 2) as c_int;
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    std::alloc::dealloc(s_raw as *mut u8, layout);
                    return 0;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    std::alloc::dealloc(s_raw as *mut u8, layout);
                    return 0;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    std::alloc::dealloc(s_raw as *mut u8, layout);
                    return 0;
                }
            }
            3 => {
                CP_ERROR_REASON = c"Detected unknown block type within input stream.".as_ptr();
                std::alloc::dealloc(s_raw as *mut u8, layout);
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
    std::alloc::dealloc(s_raw as *mut u8, layout);
    1
}
