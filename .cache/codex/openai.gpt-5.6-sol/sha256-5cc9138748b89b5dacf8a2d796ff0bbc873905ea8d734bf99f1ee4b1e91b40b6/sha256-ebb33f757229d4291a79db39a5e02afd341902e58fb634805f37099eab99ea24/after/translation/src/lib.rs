#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr::{addr_of, addr_of_mut};

#[repr(C)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

const fn fixed_table() -> [u8; 320] {
    let mut table = [0; 320];
    let mut i = 0;
    while i < 144 {
        table[i] = 8;
        i += 1;
    }
    while i < 256 {
        table[i] = 9;
        i += 1;
    }
    while i < 280 {
        table[i] = 7;
        i += 1;
    }
    while i < 288 {
        table[i] = 8;
        i += 1;
    }
    while i < 320 {
        table[i] = 5;
        i += 1;
    }
    table
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = std::ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 320] = fixed_table();

#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

const STORED_COMPLEMENT: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
const STORED_INPUT: &[u8] = b"Stored block extends beyond end of input stream.\0";
const SYMBOL_OUTPUT: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.\0";
const DISTANCE_OUTPUT: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
const STRING_OUTPUT: &[u8] = b"Attempted to overwrite out buffer while outputting a string.\0";
const UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";

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

unsafe extern "C" {
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memset(dst: *mut c_void, value: c_int, count: usize) -> *mut c_void;
}

unsafe fn set_error(message: &'static [u8]) {
    cp_error_reason = message.as_ptr().cast();
}

unsafe fn cp_would_overflow(s: *mut CpState, num_bits: c_int) -> bool {
    ((*s).bits_left + (*s).count) - num_bits < 0
}

unsafe fn cp_ptr(s: *mut CpState) -> *mut c_char {
    assert_eq!((*s).bits_left & 7, 0);
    (*s).words
        .add((*s).word_index as usize)
        .cast::<c_char>()
        .sub(((*s).count / 8) as usize)
}

unsafe fn cp_peak_bits(s: *mut CpState, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.add((*s).word_index as usize);
            (*s).word_index += 1;
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count += 32;
            assert!((*s).word_index <= (*s).word_count);
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
    assert!((*s).count >= num_bits_to_read);
    let bits = ((*s).bits & ((1u64 << num_bits_to_read) - 1)) as u32;
    (*s).bits >>= num_bits_to_read;
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!((*s).bits_left > 0);
    assert!((*s).count <= 64);
    assert!(!cp_would_overflow(s, num_bits_to_read));
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8)
}

unsafe fn cp_build(s: *mut CpState, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    for n in 0..sym_count {
        counts[*lens.add(n as usize) as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if !s.is_null() {
        (*s).lookup.fill(0);
    }
    for i in 0..sym_count {
        let code_len = *lens.add(i as usize) as usize;
        if code_len != 0 {
            assert!(code_len < 16);
            let code = codes[code_len] as u32;
            codes[code_len] += 1;
            let slot = first[code_len] as usize;
            first[code_len] += 1;
            *tree.add(slot) = (code << (32 - code_len)) | ((i as u32) << 4) | code_len as u32;
            if !s.is_null() && code_len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - code_len)) as usize;
                while j < (1 << 9) {
                    (*s).lookup[j] = ((code_len << 9) | i as usize) as u16;
                    j += 1 << code_len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: *mut CpState) -> bool {
    cp_read_bits(s, (*s).count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        set_error(STORED_COMPLEMENT);
        return false;
    }
    if !((*s).bits_left / 8 <= len as c_int) {
        set_error(STORED_INPUT);
        return false;
    }
    let p = cp_ptr(s);
    memcpy((*s).out.cast(), p.cast(), len as usize);
    (*s).out = (*s).out.add(len as usize);
    true
}

unsafe fn cp_fixed(s: *mut CpState) -> bool {
    (*s).nlit = cp_build(
        s,
        addr_of_mut!((*s).lit).cast::<u32>(),
        addr_of!(cp_fixed_table).cast::<u8>(),
        288,
    ) as u32;
    (*s).ndst = cp_build(
        std::ptr::null_mut(),
        addr_of_mut!((*s).dst).cast::<u32>(),
        addr_of!(cp_fixed_table).cast::<u8>().add(288),
        32,
    ) as u32;
    true
}

unsafe fn cp_decode(s: *mut CpState, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xffff;
    let mut lo = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.add(guess as usize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.offset((lo - 1) as isize);
    let len = 32 - (key & 0xf);
    assert_eq!(search >> len, key >> len);
    let code = cp_consume_bits(s, (key & 0xf) as c_int);
    let _ = code;
    ((key >> 4) & 0xfff) as c_int
}

unsafe fn cp_dynamic(s: *mut CpState) -> bool {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as c_int;
    let ndst = 1 + cp_read_bits(s, 5) as c_int;
    let nlen = 4 + cp_read_bits(s, 4) as c_int;
    for i in 0..nlen {
        let permutation = *addr_of!(cp_permutation_order).cast::<u8>().add(i as usize) as usize;
        lenlens[permutation] = cp_read_bits(s, 3) as u8;
    }
    (*s).nlen = cp_build(
        std::ptr::null_mut(),
        addr_of_mut!((*s).len).cast::<u32>(),
        lenlens.as_ptr(),
        19,
    ) as u32;
    let mut lens = [0u8; 288 + 32];
    let mut n = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, addr_of!((*s).len).cast::<u32>(), (*s).nlen as c_int);
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as c_int;
                while i != 0 {
                    *lens.as_mut_ptr().add(n as usize) = *lens.as_ptr().offset((n - 1) as isize);
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as c_int;
                while i != 0 {
                    *lens.as_mut_ptr().add(n as usize) = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as c_int;
                while i != 0 {
                    *lens.as_mut_ptr().add(n as usize) = 0;
                    i -= 1;
                    n += 1;
                }
            }
            _ => {
                *lens.as_mut_ptr().add(n as usize) = sym as u8;
                n += 1;
            }
        }
    }
    (*s).nlit = cp_build(s, addr_of_mut!((*s).lit).cast::<u32>(), lens.as_ptr(), nlit) as u32;
    (*s).ndst = cp_build(
        std::ptr::null_mut(),
        addr_of_mut!((*s).dst).cast::<u32>(),
        lens.as_ptr().add(nlit as usize),
        ndst,
    ) as u32;
    true
}

unsafe fn cp_block(s: *mut CpState) -> bool {
    loop {
        let mut symbol = cp_decode(s, addr_of!((*s).lit).cast::<u32>(), (*s).nlit as c_int);
        if symbol < 256 {
            if (*s).out.add(1) > (*s).out_end {
                set_error(SYMBOL_OUTPUT);
                return false;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.add(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length = cp_read_bits(
                s,
                *addr_of!(cp_len_extra_bits)
                    .cast::<u8>()
                    .add(symbol as usize) as c_int,
            ) + *addr_of!(cp_len_base).cast::<u32>().add(symbol as usize);
            let distance_symbol =
                cp_decode(s, addr_of!((*s).dst).cast::<u32>(), (*s).ndst as c_int);
            let backwards_distance = cp_read_bits(
                s,
                *addr_of!(cp_dist_extra_bits)
                    .cast::<u8>()
                    .add(distance_symbol as usize) as c_int,
            ) + *addr_of!(cp_dist_base)
                .cast::<u32>()
                .add(distance_symbol as usize);

            if (*s).out.offset(-(backwards_distance as isize)) < (*s).begin {
                set_error(DISTANCE_OUTPUT);
                return false;
            }
            if (*s).out.add(length as usize) > (*s).out_end {
                set_error(STRING_OUTPUT);
                return false;
            }
            let mut src = (*s).out.sub(backwards_distance as usize);
            let mut dst = (*s).out;
            (*s).out = (*s).out.add(length as usize);
            if backwards_distance == 1 {
                memset(dst.cast(), *src as c_int, length as usize);
            } else {
                let mut remaining = length;
                while remaining != 0 {
                    *dst = *src;
                    dst = dst.add(1);
                    src = src.add(1);
                    remaining -= 1;
                }
            }
        } else {
            break;
        }
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    output: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let s = calloc(1, size_of::<CpState>()).cast::<CpState>();
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes * 8;
    let input_addr = input as usize;
    let first_bytes = (((input_addr + 3) & !3) - input_addr) as c_int;
    (*s).words = input.cast::<u8>().add(first_bytes as usize).cast::<u32>();
    (*s).word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        (*s).bits |= (*input.cast::<u8>().add(i as usize) as u64) << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        (*s).final_word |=
            (*input.cast::<u8>().add((in_bytes - last_bytes + i) as usize) as u32) << (i * 8);
    }
    (*s).count = first_bytes * 8;
    (*s).out = output.cast();
    (*s).out_end = (*s).out.offset(out_bytes as isize);
    (*s).begin = output.cast();

    let result = loop {
        let bfinal = cp_read_bits(s, 1);
        let btype = cp_read_bits(s, 2);
        let ok = match btype {
            0 => cp_stored(s),
            1 => cp_fixed(s) && cp_block(s),
            2 => cp_dynamic(s) && cp_block(s),
            _ => {
                set_error(UNKNOWN_BLOCK);
                false
            }
        };
        if !ok {
            break 0;
        }
        if bfinal != 0 {
            break 1;
        }
    };
    free(s.cast());
    result
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> CpPixel {
    CpPixel { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> CpPixel {
    CpPixel { r, g, b, a: 0xff }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn convert_pix(
    bpp: c_int,
    w: c_int,
    h: c_int,
    mut src: *mut u8,
    mut dst: *mut CpPixel,
) {
    for _ in 0..h {
        src = src.add(1);
        for _ in 0..w {
            match bpp {
                1 => *dst = cp_make_pixel(*src, *src, *src),
                2 => *dst = cp_make_pixel_a(*src, *src, *src, *src.add(1)),
                3 => *dst = cp_make_pixel(*src, *src.add(1), *src.add(2)),
                4 => *dst = cp_make_pixel_a(*src, *src.add(1), *src.add(2), *src.add(3)),
                _ => {}
            }
            dst = dst.add(1);
            src = src.offset(bpp as isize);
        }
    }
}
