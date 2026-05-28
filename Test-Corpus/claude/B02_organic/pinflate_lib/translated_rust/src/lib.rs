// Rust translation of c_src/src/lib.c (pinflate)
// Faithful translation that exposes the same C ABI symbols.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::c_void;
use core::ptr;

// Global error reason pointer; same name & ABI as C global.
#[no_mangle]
pub static mut cp_error_reason: *const u8 = ptr::null();

#[no_mangle]
pub static mut cp_fixed_table: [u8; 288 + 32] = {
    let mut t = [0u8; 320];
    let mut i = 0;
    while i < 144 {
        t[i] = 8;
        i += 1;
    } // 0..143 -> 8
    while i < 256 {
        t[i] = 9;
        i += 1;
    } // 144..255 -> 9
    while i < 280 {
        t[i] = 7;
        i += 1;
    } // 256..279 -> 7
    while i < 288 {
        t[i] = 8;
        i += 1;
    } // 280..287 -> 8
    while i < 320 {
        t[i] = 5;
        i += 1;
    } // distance codes -> 5
    t
};

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

#[repr(C)]
struct cp_state_t {
    bits: u64,
    count: i32,
    words: *mut u32,
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

#[inline]
unsafe fn cp_would_overflow(s: *const cp_state_t, num_bits: i32) -> bool {
    ((*s).bits_left + (*s).count) - num_bits < 0
}

#[inline]
unsafe fn cp_ptr(s: *mut cp_state_t) -> *mut u8 {
    debug_assert!((*s).bits_left & 7 == 0);
    let p = (*s).words.offset((*s).word_index as isize) as *mut u8;
    p.offset(-(((*s).count / 8) as isize))
}

#[inline]
unsafe fn cp_peak_bits(s: *mut cp_state_t, num_bits_to_read: i32) -> u64 {
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

#[inline]
unsafe fn cp_consume_bits(s: *mut cp_state_t, num_bits_to_read: i32) -> u32 {
    debug_assert!((*s).count >= num_bits_to_read);
    let mask: u64 = if num_bits_to_read >= 64 {
        u64::MAX
    } else {
        (1u64 << num_bits_to_read).wrapping_sub(1)
    };
    let bits = ((*s).bits & mask) as u32;
    (*s).bits >>= num_bits_to_read;
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits
}

#[inline]
unsafe fn cp_read_bits(s: *mut cp_state_t, num_bits_to_read: i32) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    debug_assert!((*s).bits_left > 0);
    debug_assert!((*s).count <= 64);
    debug_assert!(!cp_would_overflow(s, num_bits_to_read));
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

#[inline]
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
    sym_count: i32,
) -> i32 {
    let mut codes: [i32; 16] = [0; 16];
    let mut first: [i32; 16] = [0; 16];
    let mut counts: [i32; 16] = [0; 16];
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
        let len = *lens.offset(i as isize) as i32;
        if len != 0 {
            debug_assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as u32;
            first[len as usize] += 1;
            *tree.offset(slot as isize) =
                (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                while j < (1 << 9) {
                    (*s).lookup[j as usize] =
                        (((len as u32) << 9) | (i as u32)) as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: *mut cp_state_t) -> i32 {
    cp_read_bits(s, (*s).count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if !(len == !nlen) {
        cp_error_reason =
            b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0"
                .as_ptr();
        return 0;
    }
    if !((*s).bits_left / 8 <= len as i32) {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr();
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p, (*s).out, len as usize);
    (*s).out = (*s).out.add(len as usize);
    1
}

unsafe fn cp_fixed(s: *mut cp_state_t) -> i32 {
    (*s).nlit = cp_build(
        s,
        (*s).lit.as_mut_ptr(),
        cp_fixed_table.as_ptr(),
        288,
    ) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        cp_fixed_table.as_ptr().add(288),
        32,
    ) as u32;
    1
}

unsafe fn cp_decode(s: *mut cp_state_t, tree: *const u32, mut hi: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: i32 = 0;
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
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

unsafe fn cp_dynamic(s: *mut cp_state_t) -> i32 {
    let mut lenlens: [u8; 19] = [0; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen {
        let idx = cp_permutation_order[i as usize] as usize;
        lenlens[idx] = cp_read_bits(s, 3) as u8;
    }
    (*s).nlen = cp_build(ptr::null_mut(), (*s).len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;
    let mut lens: [u8; 288 + 32] = [0; 288 + 32];
    let mut n: i32 = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, (*s).len.as_ptr(), (*s).nlen as i32);
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
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        (*s).dst.as_mut_ptr(),
        lens.as_ptr().offset(nlit as isize),
        ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut cp_state_t) -> i32 {
    loop {
        let symbol = cp_decode(s, (*s).lit.as_ptr(), (*s).nlit as i32);
        if symbol < 256 {
            if !((*s).out as usize + 1 <= (*s).out_end as usize) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr();
                return 0;
            }
            *(*s).out = symbol as u8;
            (*s).out = (*s).out.add(1);
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = cp_read_bits(s, cp_len_extra_bits[symbol as usize] as i32) as i32
                + cp_len_base[symbol as usize] as i32;
            let distance_symbol = cp_decode(s, (*s).dst.as_ptr(), (*s).ndst as i32);
            let backwards_distance = cp_read_bits(
                s,
                cp_dist_extra_bits[distance_symbol as usize] as i32,
            ) as i32
                + cp_dist_base[distance_symbol as usize] as i32;
            // Check: s->out - backwards_distance >= s->begin
            let out_minus = (*s).out as isize - backwards_distance as isize;
            if !(out_minus >= (*s).begin as isize) {
                cp_error_reason =
                    b"Attempted to write before out buffer (invalid backwards distance).\0"
                        .as_ptr();
                return 0;
            }
            if !((*s).out as usize + length as usize <= (*s).out_end as usize) {
                cp_error_reason =
                    b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr();
                return 0;
            }
            let mut src = ((*s).out as *const u8).offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.add(length as usize);
            match backwards_distance {
                1 => {
                    let v = *src;
                    for _ in 0..length {
                        *dst = v;
                        dst = dst.add(1);
                    }
                }
                _ => {
                    let mut len = length;
                    while len != 0 {
                        len -= 1;
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

#[no_mangle]
pub unsafe extern "C" fn pinflate(
    in_ptr: *mut c_void,
    in_bytes: i32,
    out_ptr: *mut c_void,
    out_bytes: i32,
) -> i32 {
    // calloc-like allocation of a zeroed cp_state_t
    let layout = core::alloc::Layout::new::<cp_state_t>();
    let s = std::alloc::alloc_zeroed(layout) as *mut cp_state_t;
    if s.is_null() {
        return 0;
    }
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes * 8;
    let in_addr = in_ptr as usize;
    let aligned = (in_addr + 3) & !3usize;
    let first_bytes = (aligned - in_addr) as i32;
    (*s).words = (in_ptr as *mut u8).offset(first_bytes as isize) as *mut u32;
    (*s).word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        let b = *((in_ptr as *mut u8).offset(i as isize)) as u64;
        (*s).bits |= b << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        let b = *((in_ptr as *mut u8).offset((in_bytes - last_bytes + i) as isize)) as u32;
        (*s).final_word |= b << (i * 8);
    }
    (*s).count = first_bytes * 8;
    (*s).out = out_ptr as *mut u8;
    (*s).out_end = (out_ptr as *mut u8).offset(out_bytes as isize);
    (*s).begin = out_ptr as *mut u8;

    let mut count: i32 = 0;
    let mut bfinal: u32;
    let mut err = false;
    loop {
        bfinal = cp_read_bits(s, 1);
        let btype = cp_read_bits(s, 2);
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
    std::alloc::dealloc(s as *mut u8, layout);
    if err {
        0
    } else {
        1
    }
}
