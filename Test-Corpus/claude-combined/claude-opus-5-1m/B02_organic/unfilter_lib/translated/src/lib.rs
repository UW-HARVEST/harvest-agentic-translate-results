#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// Exported globals (must match C linkage names exactly)
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59,
    67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

// ---------------------------------------------------------------------------
// Internal state used by cp_inflate.
// ---------------------------------------------------------------------------

struct CpState {
    bits: u64,
    count: i32,
    // Raw pointer into the input buffer (aligned).
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

impl CpState {
    fn new() -> Self {
        Self {
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
            lookup: [0u16; 1 << 9],
            lit: [0u32; 288],
            dst: [0u32; 32],
            len: [0u32; 19],
            nlit: 0,
            ndst: 0,
            nlen: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Helper functions translated faithfully from the C source.
// ---------------------------------------------------------------------------

fn cp_would_overflow(s: &CpState, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

unsafe fn cp_ptr(s: &CpState) -> *mut u8 {
    debug_assert!(s.bits_left & 7 == 0);
    let base = unsafe { s.words.offset(s.word_index as isize) } as *mut u8;
    unsafe { base.offset(-((s.count / 8) as isize)) }
}

unsafe fn cp_peak_bits(s: &mut CpState, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { *s.words.offset(s.word_index as isize) };
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
    // ((uint64_t)1 << num_bits_to_read) - 1, where num_bits_to_read can be 0.
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

unsafe fn cp_read_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
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

/// Reproduces the C `cp_build` function. When `with_lookup` is true, also
/// fills in the state's lookup table (mirroring the `s != NULL` branch).
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
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if let Some(ref s) = s {
        // Need to write to s.lookup; the immutable borrow above is OK because
        // we'll obtain a fresh mutable borrow below. To keep borrow-checker
        // happy, we just clear via direct path below.
        let _ = s;
    }

    // Re-take a fresh mutable borrow path: pass the state as an Option and
    // clear lookup if present.
    if let Some(s) = s {
        s.lookup.fill(0);
        for i in 0..sym_count {
            let len = lens[i] as i32;
            if len != 0 {
                debug_assert!(len < 16);
                let code = codes[len as usize] as u32;
                codes[len as usize] += 1;
                let slot = first[len as usize] as u32;
                first[len as usize] += 1;
                tree[slot as usize] =
                    (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        s.lookup[j] = ((len << 9) as u16) | (i as u16);
                        j += 1usize << len;
                    }
                }
            }
        }
        first[15]
    } else {
        for i in 0..sym_count {
            let len = lens[i] as i32;
            if len != 0 {
                debug_assert!(len < 16);
                let code = codes[len as usize] as u32;
                codes[len as usize] += 1;
                let slot = first[len as usize] as u32;
                first[len as usize] += 1;
                tree[slot as usize] =
                    (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            }
        }
        first[15]
    }
}

unsafe fn cp_stored(s: &mut CpState) -> i32 {
    let align = s.count & 7;
    unsafe {
        cp_read_bits(s, align);
        let len_val = (cp_read_bits(s, 16) & 0xFFFF) as u16;
        let nlen_val = (cp_read_bits(s, 16) & 0xFFFF) as u16;
        if len_val != !nlen_val {
            cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
            return 0;
        }
        if !(s.bits_left / 8 <= len_val as i32) {
            cp_error_reason =
                b"Stored block extends beyond end of input stream.\0".as_ptr()
                    as *const c_char;
            return 0;
        }
        let p = cp_ptr(s);
        ptr::copy_nonoverlapping(p, s.out, len_val as usize);
        s.out = s.out.add(len_val as usize);
        1
    }
}

unsafe fn cp_fixed(s: &mut CpState) -> i32 {
    // First 288 entries: literal/length tree.
    let table_ptr = ptr::addr_of_mut!(cp_fixed_table) as *mut u8;
    unsafe {
        let lit_slice = std::slice::from_raw_parts(table_ptr, 288);
        let dst_slice = std::slice::from_raw_parts(table_ptr.add(288), 32);
        // Copies into local buffers to satisfy borrow checker (cp_build needs
        // &mut s and the tables that live inside s).
        let lit_lens: Vec<u8> = lit_slice.to_vec();
        let dst_lens: Vec<u8> = dst_slice.to_vec();
        let mut lit_tree = s.lit;
        let nlit = cp_build(Some(s), &mut lit_tree, &lit_lens, 288);
        s.lit = lit_tree;
        s.nlit = nlit as u32;
        let mut dst_tree = s.dst;
        let ndst = cp_build(None, &mut dst_tree, &dst_lens, 32);
        s.dst = dst_tree;
        s.ndst = ndst as u32;
    }
    1
}

unsafe fn cp_decode(s: &mut CpState, tree: &[u32], hi: i32) -> i32 {
    let bits = unsafe { cp_peak_bits(s, 16) };
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;
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
    let _len = 32u32 - (key & 0xF);
    debug_assert!((search >> _len) == (key >> _len));
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

unsafe fn cp_dynamic(s: &mut CpState) -> i32 {
    unsafe {
        let mut lenlens = [0u8; 19];
        let nlit = 257 + cp_read_bits(s, 5) as i32;
        let ndst = 1 + cp_read_bits(s, 5) as i32;
        let nlen = 4 + cp_read_bits(s, 4) as i32;
        for i in 0..nlen as usize {
            let perm = cp_permutation_order[i] as usize;
            lenlens[perm] = cp_read_bits(s, 3) as u8;
        }
        // build len tree
        let mut len_tree = s.len;
        let n_len_built = cp_build(None, &mut len_tree, &lenlens, 19);
        s.len = len_tree;
        s.nlen = n_len_built as u32;

        let mut lens = [0u8; 288 + 32];
        let mut n: i32 = 0;
        let total = nlit + ndst;
        while n < total {
            let len_tree_local = s.len; // copy out for borrow
            let sym = cp_decode(s, &len_tree_local, s.nlen as i32);
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
        let mut lit_tree = s.lit;
        let nlit_built = cp_build(Some(s), &mut lit_tree, &lens, nlit as usize);
        s.lit = lit_tree;
        s.nlit = nlit_built as u32;
        let mut dst_tree = s.dst;
        let dst_lens_slice: Vec<u8> =
            lens[nlit as usize..(nlit as usize + ndst as usize)].to_vec();
        let ndst_built =
            cp_build(None, &mut dst_tree, &dst_lens_slice, ndst as usize);
        s.dst = dst_tree;
        s.ndst = ndst_built as u32;
    }
    1
}

unsafe fn cp_block(s: &mut CpState) -> i32 {
    unsafe {
        loop {
            let lit_tree = s.lit;
            let symbol = cp_decode(s, &lit_tree, s.nlit as i32);
            if symbol < 256 {
                if !(s.out.offset(1) <= s.out_end) {
                    cp_error_reason =
                        b"Attempted to overwrite out buffer while outputting a symbol.\0"
                            .as_ptr() as *const c_char;
                    return 0;
                }
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            } else if symbol > 256 {
                let symbol = symbol - 257;
                let extra_len = cp_len_extra_bits[symbol as usize] as i32;
                let length = cp_read_bits(s, extra_len) as i32
                    + cp_len_base[symbol as usize] as i32;
                let dst_tree = s.dst;
                let distance_symbol = cp_decode(s, &dst_tree, s.ndst as i32);
                let extra_dist =
                    cp_dist_extra_bits[distance_symbol as usize] as i32;
                let backwards_distance = cp_read_bits(s, extra_dist) as i32
                    + cp_dist_base[distance_symbol as usize] as i32;
                if !(s.out.offset(-(backwards_distance as isize)) >= s.begin) {
                    cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                    return 0;
                }
                if !(s.out.offset(length as isize) <= s.out_end) {
                    cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                    return 0;
                }
                let src = s.out.offset(-(backwards_distance as isize));
                let dst_ptr = s.out;
                s.out = s.out.add(length as usize);
                if backwards_distance == 1 {
                    ptr::write_bytes(dst_ptr, *src, length as usize);
                } else {
                    let mut srcp = src;
                    let mut dstp = dst_ptr;
                    let mut remaining = length;
                    while remaining > 0 {
                        *dstp = *srcp;
                        dstp = dstp.add(1);
                        srcp = srcp.add(1);
                        remaining -= 1;
                    }
                }
            } else {
                break;
            }
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    in_ptr: *mut c_void,
    in_bytes: c_int,
    out_ptr: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        let mut s_box = Box::new(CpState::new());
        let s: &mut CpState = &mut s_box;
        s.bits = 0;
        s.count = 0;
        s.word_index = 0;
        s.bits_left = in_bytes * 8;
        let in_addr = in_ptr as usize;
        let aligned = (in_addr + 3) & !3usize;
        let first_bytes = (aligned - in_addr) as i32;
        s.words = (in_ptr as *mut u8).offset(first_bytes as isize) as *mut u32;
        s.word_count = (in_bytes - first_bytes) / 4;
        let last_bytes = (in_bytes - first_bytes) & 3;
        for i in 0..first_bytes {
            let byte = *(in_ptr as *mut u8).offset(i as isize);
            s.bits |= (byte as u64) << (i * 8);
        }
        s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
        s.final_word = 0;
        for i in 0..last_bytes {
            let byte =
                *(in_ptr as *mut u8).offset((in_bytes - last_bytes + i) as isize);
            // C: s->final_word |= ((uint8_t*)in)[...] << (i * 8);
            // The shift result is `int` in C, but accumulated in u32, so mimic
            // exactly via wrapping shifts.
            s.final_word |= (byte as u32) << (i * 8);
        }
        s.count = first_bytes * 8;
        s.out = out_ptr as *mut u8;
        s.out_end = s.out.offset(out_bytes as isize);
        s.begin = out_ptr as *mut u8;
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
                        b"Detected unknown block type within input stream.\0"
                            .as_ptr() as *const c_char;
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
        // Box drop frees s.
        1
    }
}

// ---------------------------------------------------------------------------
// PNG filter related code (the only function exposed via lib.h).
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(
    w: c_int,
    h: c_int,
    bpp: c_int,
    raw: *mut u8,
) -> c_int {
    unsafe {
        let len = w * bpp;
        let mut raw = raw;
        let prev: *mut u8;
        let mut x: i32;
        if h > 0 {
            let filt = *raw;
            raw = raw.add(1);
            match filt {
                0 => {}
                1 => {
                    x = bpp;
                    while x < len {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                2 => {}
                3 => {
                    x = bpp;
                    while x < len {
                        let v = (*raw.offset(x as isize)).wrapping_add(
                            *raw.offset((x - bpp) as isize) / 2,
                        );
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                4 => {
                    x = bpp;
                    while x < len {
                        let pae = cp_paeth(*raw.offset((x - bpp) as isize), 0, 0);
                        let v = (*raw.offset(x as isize)).wrapping_add(pae);
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                _ => return 0,
            }
        }
        prev = raw;
        let mut prev = prev;
        raw = raw.offset(len as isize);
        let mut y = 1;
        while y < h {
            let filt = *raw;
            raw = raw.add(1);
            match filt {
                0 => {}
                1 => {
                    x = 0;
                    while x < bpp {
                        let v = (*raw.offset(x as isize)).wrapping_add(0);
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                    while x < len {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                2 => {
                    x = 0;
                    while x < bpp {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*prev.offset(x as isize));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                    while x < len {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*prev.offset(x as isize));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                3 => {
                    x = 0;
                    while x < bpp {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*prev.offset(x as isize) / 2);
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                    while x < len {
                        let sum = (*raw.offset((x - bpp) as isize) as u32)
                            .wrapping_add(*prev.offset(x as isize) as u32);
                        let v = (*raw.offset(x as isize))
                            .wrapping_add((sum / 2) as u8);
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                4 => {
                    x = 0;
                    while x < bpp {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*prev.offset(x as isize));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                    while x < len {
                        let pae = cp_paeth(
                            *raw.offset((x - bpp) as isize),
                            *prev.offset(x as isize),
                            *prev.offset((x - bpp) as isize),
                        );
                        let v = (*raw.offset(x as isize)).wrapping_add(pae);
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                _ => return 0,
            }
            y += 1;
            prev = raw;
            raw = raw.offset(len as isize);
        }
        1
    }
}
