//! Translation of c_src/src/lib.c to Rust.
//!
//! Provides a PNG `unfilter` routine and a DEFLATE `cp_inflate` routine
//! mirroring the C implementation as closely as possible.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::too_many_arguments)]

use std::os::raw::{c_char, c_int, c_void};

/// Mutable global mirroring the C `cp_error_reason`.
///
/// In the C version this is a `const char *`. In Rust we hold an
/// `Option<&'static str>` that is set whenever an error occurs during
/// inflation. Access is intentionally unsynchronized to match the C API.
static mut CP_ERROR_REASON: Option<&'static str> = None;

#[inline]
fn set_error(msg: &'static str) {
    unsafe {
        CP_ERROR_REASON = Some(msg);
    }
}

/// Returns the most recently set error reason, if any.
pub fn cp_error_reason() -> Option<&'static str> {
    unsafe { CP_ERROR_REASON }
}

static CP_FIXED_TABLE: [u8; 288 + 32] = [
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

static CP_PERMUTATION_ORDER: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

static CP_LEN_EXTRA_BITS: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

static CP_LEN_BASE: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

static CP_DIST_EXTRA_BITS: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

static CP_DIST_BASE: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

/// Decoder state. The original C `cp_state_t` mixes pointers into the input
/// buffer with output pointers; we keep references to slices instead.
struct CpState<'a> {
    bits: u64,
    count: i32,
    /// Bytes following the initial alignment-padding bytes, viewed as `u32`.
    words: &'a [u32],
    word_count: i32,
    word_index: i32,
    bits_left: i32,
    final_word_available: bool,
    final_word: u32,

    /// Reference to the original input bytes (used for `cp_ptr`/stored blocks).
    in_bytes: &'a [u8],
    /// Number of bytes at the start of `in_bytes` skipped to reach
    /// 4-byte alignment (and packed into `bits` initially).
    first_bytes: i32,

    out: &'a mut [u8],
    /// Current write index into `out`.
    out_pos: usize,

    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

#[inline]
fn cp_would_overflow(s: &CpState, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

/// Returns the byte offset within `in_bytes` corresponding to the current
/// read position. Mirrors the C `cp_ptr` which returns a pointer.
fn cp_ptr_offset(s: &CpState) -> usize {
    debug_assert!(s.bits_left & 7 == 0);
    // (s->words + s->word_index) - (s->count / 8)
    let words_byte_offset = s.first_bytes as usize + (s.word_index as usize) * 4;
    words_byte_offset - (s.count as usize) / 8
}

fn cp_peak_bits(s: &mut CpState, _num_bits_to_read: i32) -> u64 {
    if s.count < _num_bits_to_read {
        if s.word_index < s.word_count {
            let word = s.words[s.word_index as usize];
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            debug_assert!(s.word_index <= s.word_count);
        } else if s.final_word_available {
            let word = s.final_word;
            s.bits |= (word as u64) << s.count;
            s.count += s.bits_left;
            s.final_word_available = false;
        }
    }
    s.bits
}

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
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

fn cp_read_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    debug_assert!(s.bits_left > 0);
    debug_assert!(s.count <= 64);
    debug_assert!(!cp_would_overflow(s, num_bits_to_read));
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

/// Build a Huffman tree. If `update_lookup` is true, the state's lookup
/// table is also populated (mirroring the `s` non-NULL case in C).
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

    if let Some(ref state) = s {
        // Clear the lookup table.
        let _ = state; // borrow check workaround
    }
    // We need a mutable borrow if `s` is Some, so handle that explicitly.
    let s_opt = s;
    if let Some(ref mut state) = { s_opt } {
        for v in state.lookup.iter_mut() {
            *v = 0;
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
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                    while j < (1 << 9) {
                        state.lookup[j as usize] = ((len << 9) | (i as i32)) as u16;
                        j += 1 << len;
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
                let slot = first[len as usize] as usize;
                first[len as usize] += 1;
                tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            }
        }
        first[15]
    }
}

fn cp_stored(s: &mut CpState) -> bool {
    let extra = s.count & 7;
    cp_read_bits(s, extra);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        set_error(
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
        );
        return false;
    }
    if !(s.bits_left / 8 <= len as i32) {
        set_error("Stored block extends beyond end of input stream.");
        return false;
    }
    let off = cp_ptr_offset(s);
    let len_usize = len as usize;
    if off + len_usize > s.in_bytes.len() {
        set_error("Stored block extends beyond end of input stream.");
        return false;
    }
    if s.out_pos + len_usize > s.out.len() {
        set_error("Stored block exceeds output buffer.");
        return false;
    }
    s.out[s.out_pos..s.out_pos + len_usize]
        .copy_from_slice(&s.in_bytes[off..off + len_usize]);
    s.out_pos += len_usize;
    true
}

fn cp_fixed(s: &mut CpState) -> bool {
    // Build literal tree (with lookup).
    let mut lit = [0u32; 288];
    let nlit = cp_build(Some(s), &mut lit, &CP_FIXED_TABLE[..288], 288);
    s.lit = lit;
    s.nlit = nlit as u32;

    // Build distance tree (no lookup).
    let mut dst = [0u32; 32];
    let ndst = cp_build(None, &mut dst, &CP_FIXED_TABLE[288..], 32);
    s.dst = dst;
    s.ndst = ndst as u32;
    true
}

fn cp_decode(s: &mut CpState, tree: &[u32], hi_in: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0i32;
    let mut hi = hi_in;
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

fn cp_dynamic(s: &mut CpState) -> bool {
    let mut lenlens = [0u8; 19];
    let nlit_count = 257 + cp_read_bits(s, 5) as usize;
    let ndst_count = 1 + cp_read_bits(s, 5) as usize;
    let nlen_count = 4 + cp_read_bits(s, 4) as usize;
    for i in 0..nlen_count {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    let mut len_tree = [0u32; 19];
    let nlen_tree = cp_build(None, &mut len_tree, &lenlens, 19);
    s.len = len_tree;
    s.nlen = nlen_tree as u32;

    let mut lens = [0u8; 288 + 32];
    let total = nlit_count + ndst_count;
    let mut n: usize = 0;
    while n < total {
        let len_tree_local = s.len;
        let nlen_local = s.nlen as i32;
        let sym = cp_decode(s, &len_tree_local, nlen_local);
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

    let mut lit = [0u32; 288];
    let nlit_built = cp_build(Some(s), &mut lit, &lens[..nlit_count], nlit_count);
    s.lit = lit;
    s.nlit = nlit_built as u32;

    let mut dst = [0u32; 32];
    let ndst_built = cp_build(None, &mut dst, &lens[nlit_count..nlit_count + ndst_count], ndst_count);
    s.dst = dst;
    s.ndst = ndst_built as u32;
    true
}

fn cp_block(s: &mut CpState) -> bool {
    loop {
        let lit_tree = s.lit;
        let nlit = s.nlit as i32;
        let symbol = cp_decode(s, &lit_tree, nlit);
        if symbol < 256 {
            if !(s.out_pos + 1 <= s.out.len()) {
                set_error("Attempted to overwrite out buffer while outputting a symbol.");
                return false;
            }
            s.out[s.out_pos] = symbol as u8;
            s.out_pos += 1;
        } else if symbol > 256 {
            let symbol_idx = (symbol - 257) as usize;
            let length = (cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol_idx] as i32)
                + CP_LEN_BASE[symbol_idx]) as usize;
            let dst_tree = s.dst;
            let ndst = s.ndst as i32;
            let distance_symbol = cp_decode(s, &dst_tree, ndst) as usize;
            let backwards_distance = (cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol] as i32)
                + CP_DIST_BASE[distance_symbol]) as usize;
            if !(s.out_pos >= backwards_distance) {
                set_error("Attempted to write before out buffer (invalid backwards distance).");
                return false;
            }
            if !(s.out_pos + length <= s.out.len()) {
                set_error("Attempted to overwrite out buffer while outputting a string.");
                return false;
            }
            let src_start = s.out_pos - backwards_distance;
            let dst_start = s.out_pos;
            s.out_pos += length;
            if backwards_distance == 1 {
                let byte = s.out[src_start];
                for i in 0..length {
                    s.out[dst_start + i] = byte;
                }
            } else {
                for i in 0..length {
                    s.out[dst_start + i] = s.out[src_start + i];
                }
            }
        } else {
            break;
        }
    }
    true
}

/// Inflates DEFLATE-compressed data.
///
/// Returns `1` on success, `0` on failure (matching the C function).
pub fn cp_inflate_rs(in_bytes: &[u8], out: &mut [u8]) -> i32 {
    let in_len = in_bytes.len();
    let in_addr = in_bytes.as_ptr() as usize;
    let first_bytes = (((in_addr + 3) & !3usize) - in_addr) as i32;
    let first_bytes = first_bytes.min(in_len as i32);

    let last_bytes = (in_len as i32 - first_bytes) & 3;
    let word_count = (in_len as i32 - first_bytes - last_bytes) / 4;

    // Build a slice of u32s starting at the aligned offset. The original
    // C code casts the input pointer to `uint32_t*`. Here we read the bytes
    // into an owned vector to avoid alignment/aliasing issues in safe Rust.
    let mut words: Vec<u32> = Vec::with_capacity(word_count.max(0) as usize);
    for i in 0..word_count as usize {
        let base = first_bytes as usize + i * 4;
        let w = (in_bytes[base] as u32)
            | ((in_bytes[base + 1] as u32) << 8)
            | ((in_bytes[base + 2] as u32) << 16)
            | ((in_bytes[base + 3] as u32) << 24);
        words.push(w);
    }

    let mut bits: u64 = 0;
    for i in 0..first_bytes as usize {
        bits |= (in_bytes[i] as u64) << (i * 8);
    }
    let mut final_word: u32 = 0;
    for i in 0..last_bytes as usize {
        final_word |= (in_bytes[in_len - last_bytes as usize + i] as u32) << (i * 8);
    }

    let mut s = CpState {
        bits,
        count: first_bytes * 8,
        words: &words,
        word_count,
        word_index: 0,
        bits_left: in_len as i32 * 8,
        final_word_available: last_bytes != 0,
        final_word,
        in_bytes,
        first_bytes,
        out,
        out_pos: 0,
        lookup: [0; 1 << 9],
        lit: [0; 288],
        dst: [0; 32],
        len: [0; 19],
        nlit: 0,
        ndst: 0,
        nlen: 0,
    };

    let mut bfinal: u32;
    loop {
        bfinal = cp_read_bits(&mut s, 1);
        let btype = cp_read_bits(&mut s, 2);
        match btype {
            0 => {
                if !cp_stored(&mut s) {
                    return 0;
                }
            }
            1 => {
                cp_fixed(&mut s);
                if !cp_block(&mut s) {
                    return 0;
                }
            }
            2 => {
                cp_dynamic(&mut s);
                if !cp_block(&mut s) {
                    return 0;
                }
            }
            _ => {
                set_error("Detected unknown block type within input stream.");
                return 0;
            }
        }
        if bfinal != 0 {
            break;
        }
    }
    1
}

/// FFI-compatible version of `cp_inflate`, mirroring the C signature.
///
/// # Safety
/// The caller must guarantee that `in_ptr` points to `in_bytes` valid bytes
/// of input, and that `out_ptr` points to `out_bytes` valid writable bytes.
#[no_mangle]
pub unsafe extern "C" fn cp_inflate(
    in_ptr: *mut c_void,
    in_bytes: c_int,
    out_ptr: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    if in_ptr.is_null() || out_ptr.is_null() || in_bytes < 0 || out_bytes < 0 {
        return 0;
    }
    let in_slice = std::slice::from_raw_parts(in_ptr as *const u8, in_bytes as usize);
    let out_slice = std::slice::from_raw_parts_mut(out_ptr as *mut u8, out_bytes as usize);
    cp_inflate_rs(in_slice, out_slice) as c_int
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

/// Safe Rust version of `unfilter`.
///
/// Returns `1` on success and `0` on an unknown filter byte.
pub fn unfilter_rs(w: i32, h: i32, bpp: i32, raw: &mut [u8]) -> i32 {
    let len = (w * bpp) as usize;
    let bpp_us = bpp as usize;

    // First scanline.
    let mut row_start: usize = 0;
    if h > 0 {
        let filter = raw[row_start];
        row_start += 1;
        match filter {
            0 => {}
            1 => {
                let mut x = bpp_us;
                while x < len {
                    raw[row_start + x] = raw[row_start + x].wrapping_add(raw[row_start + x - bpp_us]);
                    x += 1;
                }
            }
            2 => {}
            3 => {
                let mut x = bpp_us;
                while x < len {
                    raw[row_start + x] =
                        raw[row_start + x].wrapping_add(raw[row_start + x - bpp_us] / 2);
                    x += 1;
                }
            }
            4 => {
                let mut x = bpp_us;
                while x < len {
                    let p = cp_paeth(raw[row_start + x - bpp_us], 0, 0);
                    raw[row_start + x] = raw[row_start + x].wrapping_add(p);
                    x += 1;
                }
            }
            _ => return 0,
        }
    }

    let mut prev_start: usize = row_start;
    row_start += len;

    for _y in 1..h {
        let filter = raw[row_start];
        row_start += 1;
        match filter {
            0 => {}
            1 => {
                // First bpp bytes: raw[x] += 0 (no-op)
                let mut x = bpp_us;
                while x < len {
                    raw[row_start + x] = raw[row_start + x].wrapping_add(raw[row_start + x - bpp_us]);
                    x += 1;
                }
            }
            2 => {
                // Both loops in C add prev[x] for all x in [0, len).
                let mut x = 0usize;
                while x < len {
                    let p = raw[prev_start + x];
                    raw[row_start + x] = raw[row_start + x].wrapping_add(p);
                    x += 1;
                }
            }
            3 => {
                let mut x = 0usize;
                while x < bpp_us {
                    let p = raw[prev_start + x];
                    raw[row_start + x] = raw[row_start + x].wrapping_add(p / 2);
                    x += 1;
                }
                while x < len {
                    let avg =
                        ((raw[row_start + x - bpp_us] as u32 + raw[prev_start + x] as u32) / 2) as u8;
                    raw[row_start + x] = raw[row_start + x].wrapping_add(avg);
                    x += 1;
                }
            }
            4 => {
                let mut x = 0usize;
                while x < bpp_us {
                    let p = raw[prev_start + x];
                    raw[row_start + x] = raw[row_start + x].wrapping_add(p);
                    x += 1;
                }
                while x < len {
                    let pae = cp_paeth(
                        raw[row_start + x - bpp_us],
                        raw[prev_start + x],
                        raw[prev_start + x - bpp_us],
                    );
                    raw[row_start + x] = raw[row_start + x].wrapping_add(pae);
                    x += 1;
                }
            }
            _ => return 0,
        }
        prev_start = row_start;
        row_start += len;
    }

    1
}

/// FFI-compatible version of `unfilter`, mirroring the C signature
/// `int unfilter(int w, int h, int bpp, uint8_t *raw);`.
///
/// # Safety
/// `raw` must point to at least `h * (w * bpp + 1)` bytes of writable memory.
#[no_mangle]
pub unsafe extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    if raw.is_null() || w < 0 || h < 0 || bpp < 0 {
        return 0;
    }
    let total = (h as usize) * ((w as usize) * (bpp as usize) + 1);
    let slice = std::slice::from_raw_parts_mut(raw, total);
    unfilter_rs(w, h, bpp, slice) as c_int
}

// Suppress unused warnings for items that mirror C declarations but aren't
// referenced by the public API in safe Rust.
#[allow(dead_code)]
fn _suppress_unused() {
    let _ = c_char::default();
}
