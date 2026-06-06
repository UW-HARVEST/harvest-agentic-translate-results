#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_assignments)]

use std::ffi::c_int;
use std::os::raw::c_void;

// ---------- Globals (matching C non-static globals) ----------

#[no_mangle]
pub static mut cp_error_reason: *const u8 = std::ptr::null();

const fn make_cp_fixed_table() -> [u8; 288 + 32] {
    let mut arr = [0u8; 288 + 32];
    let mut i = 0;
    while i < 144 {
        arr[i] = 8;
        i += 1;
    }
    while i < 256 {
        arr[i] = 9;
        i += 1;
    }
    while i < 280 {
        arr[i] = 7;
        i += 1;
    }
    while i < 288 {
        arr[i] = 8;
        i += 1;
    }
    while i < 320 {
        arr[i] = 5;
        i += 1;
    }
    arr
}

#[no_mangle]
pub static mut cp_fixed_table: [u8; 288 + 32] = make_cp_fixed_table();

#[no_mangle]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[no_mangle]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

#[no_mangle]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

#[no_mangle]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

#[no_mangle]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

// ---------- State struct ----------

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

// ---------- Bit-reading helpers ----------

#[inline]
fn cp_would_overflow(s: &cp_state_t, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

#[inline]
unsafe fn cp_ptr(s: &cp_state_t) -> *mut u8 {
    debug_assert!(s.bits_left & 7 == 0);
    (s.words.add(s.word_index as usize) as *mut u8).offset(-((s.count / 8) as isize))
}

unsafe fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = *s.words.add(s.word_index as usize);
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
    let mask = if num_bits_to_read >= 64 {
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
        let l = *lens.add(n as usize) as usize;
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
        let s_ref = &mut *s;
        for v in s_ref.lookup.iter_mut() {
            *v = 0;
        }
    }
    for i in 0..sym_count {
        let len = *lens.add(i as usize) as c_int;
        if len != 0 {
            debug_assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as u32;
            first[len as usize] += 1;
            *tree.add(slot as usize) = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let s_ref = &mut *s;
                let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                while j < (1 << 9) {
                    s_ref.lookup[j as usize] = ((len << 9) | i) as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut cp_state_t) -> c_int {
    cp_read_bits(s, s.count & 7);
    let LEN: u16 = cp_read_bits(s, 16) as u16;
    let NLEN: u16 = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        cp_error_reason =
            b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0"
                .as_ptr();
        return 0;
    }
    if !(s.bits_left / 8 <= LEN as c_int) {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr();
        return 0;
    }
    let p = cp_ptr(s);
    std::ptr::copy_nonoverlapping(p, s.out, LEN as usize);
    s.out = s.out.add(LEN as usize);
    1
}

unsafe fn cp_fixed(s: &mut cp_state_t) -> c_int {
    let s_ptr: *mut cp_state_t = s;
    s.nlit = cp_build(
        s_ptr,
        s.lit.as_mut_ptr(),
        cp_fixed_table.as_ptr(),
        288,
    ) as u32;
    s.ndst = cp_build(
        std::ptr::null_mut(),
        s.dst.as_mut_ptr(),
        cp_fixed_table.as_ptr().add(288),
        32,
    ) as u32;
    1
}

unsafe fn cp_decode(s: &mut cp_state_t, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.add(guess as usize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.add((lo - 1) as usize);
    let _len = 32 - (key & 0xF);
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: &mut cp_state_t) -> c_int {
    let mut lenlens: [u8; 19] = [0; 19];
    let nlit = 257 + cp_read_bits(s, 5) as c_int;
    let ndst = 1 + cp_read_bits(s, 5) as c_int;
    let nlen = 4 + cp_read_bits(s, 4) as c_int;
    for i in 0..nlen {
        lenlens[cp_permutation_order[i as usize] as usize] = cp_read_bits(s, 3) as u8;
    }
    let s_ptr: *mut cp_state_t = s;
    s.nlen = cp_build(
        std::ptr::null_mut(),
        s.len.as_mut_ptr(),
        lenlens.as_ptr(),
        19,
    ) as u32;
    let mut lens: [u8; 288 + 32] = [0; 288 + 32];
    let mut n: c_int = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, s.len.as_ptr(), s.nlen as c_int);
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
    s.nlit = cp_build(s_ptr, s.lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
    s.ndst = cp_build(
        std::ptr::null_mut(),
        s.dst.as_mut_ptr(),
        lens.as_ptr().add(nlit as usize),
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: &mut cp_state_t) -> c_int {
    loop {
        let symbol = cp_decode(s, s.lit.as_ptr(), s.nlit as c_int);
        if symbol < 256 {
            if !(s.out as usize + 1 <= s.out_end as usize) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr();
                return 0;
            }
            *s.out = symbol as u8;
            s.out = s.out.add(1);
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = (cp_read_bits(s, cp_len_extra_bits[symbol as usize] as c_int)
                + cp_len_base[symbol as usize]) as c_int;
            let distance_symbol = cp_decode(s, s.dst.as_ptr(), s.ndst as c_int);
            let backwards_distance = (cp_read_bits(
                s,
                cp_dist_extra_bits[distance_symbol as usize] as c_int,
            ) + cp_dist_base[distance_symbol as usize])
                as c_int;
            if !(s.out as isize - backwards_distance as isize >= s.begin as isize) {
                cp_error_reason =
                    b"Attempted to write before out buffer (invalid backwards distance).\0"
                        .as_ptr();
                return 0;
            }
            if !(s.out as usize + length as usize <= s.out_end as usize) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr();
                return 0;
            }
            let mut src = s.out.offset(-(backwards_distance as isize));
            let mut dst = s.out;
            s.out = s.out.add(length as usize);
            match backwards_distance {
                1 => {
                    let v = *src;
                    for i in 0..length as usize {
                        *dst.add(i) = v;
                    }
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
    // Allocate zeroed cp_state_t (matching calloc behavior)
    let layout = std::alloc::Layout::new::<cp_state_t>();
    let s_raw = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;
    if s_raw.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    let s = &mut *s_raw;
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;
    let in_addr = in_ptr as usize;
    let first_bytes = (((in_addr + 3) & !3) - in_addr) as c_int;
    s.words = (in_ptr as *mut u8).add(first_bytes as usize) as *mut u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes as usize {
        s.bits |= (*(in_ptr as *mut u8).add(i) as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes as usize {
        s.final_word |=
            (*(in_ptr as *mut u8).add((in_bytes - last_bytes) as usize + i) as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out as *mut u8;
    s.out_end = (out as *mut u8).add(out_bytes as usize);
    s.begin = out as *mut u8;
    let mut count: c_int = 0;
    let mut bfinal: c_int;
    let mut err = false;
    loop {
        bfinal = cp_read_bits(s, 1) as c_int;
        let btype = cp_read_bits(s, 2) as c_int;
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    err = true;
                    break;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    err = true;
                    break;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    err = true;
                    break;
                }
            }
            3 => {
                cp_error_reason = b"Detected unknown block type within input stream.\0".as_ptr();
                err = true;
                break;
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
    if err {
        0
    } else {
        1
    }
}

// ---------- PNG filter ----------

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
    let len = w * bpp;
    let mut raw = raw;
    let mut prev: *mut u8;
    let mut x: c_int;
    if h > 0 {
        let f = *raw;
        raw = raw.add(1);
        match f {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
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
    let mut y: c_int = 1;
    while y < h {
        let f = *raw;
        raw = raw.add(1);
        match f {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(0);
                    x += 1;
                }
                while x < len {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                    x += 1;
                }
                while x < len {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize) / 2);
                    x += 1;
                }
                while x < len {
                    let r_xb = *raw.add((x - bpp) as usize) as i32;
                    let p_x = *prev.add(x as usize) as i32;
                    let avg = ((r_xb + p_x) / 2) as u8;
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(avg);
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    *raw.add(x as usize) =
                        (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
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
