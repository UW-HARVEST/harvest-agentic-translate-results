#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_int;
use std::os::raw::c_char;
use std::ptr;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

impl Default for cp_image_t {
    fn default() -> Self {
        cp_image_t {
            w: 0,
            h: 0,
            pix: ptr::null_mut(),
        }
    }
}

#[inline]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

#[inline]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

#[no_mangle]
pub static mut cp_error_reason: *const c_char = ptr::null();

static cp_fixed_table: [u8; 288 + 32] = [
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

static cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

static cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

static cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

static cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

static cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

#[repr(C)]
struct cp_state_t {
    bits: u64,
    count: i32,
    words: *const u8, // pointer to start of words within the input buffer
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

impl cp_state_t {
    fn new() -> Self {
        cp_state_t {
            bits: 0,
            count: 0,
            words: ptr::null(),
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

#[inline]
fn cp_would_overflow(s: &cp_state_t, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

// Returns the byte pointer to the position of the next byte to be read, given that
// we are aligned on a byte boundary.
unsafe fn cp_ptr(s: &cp_state_t) -> *const u8 {
    debug_assert!(s.bits_left & 7 == 0);
    // (char *)(s->words + s->word_index) - (s->count / 8)
    let words_byte_ptr = s.words.add((s.word_index as usize) * 4);
    words_byte_ptr.sub((s.count / 8) as usize)
}

unsafe fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            // read 32-bit word at s.words[word_index]
            let off = (s.word_index as usize) * 4;
            let p = s.words.add(off);
            let word = u32::from_le_bytes([
                *p,
                *p.add(1),
                *p.add(2),
                *p.add(3),
            ]);
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

fn cp_consume_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u32 {
    debug_assert!(s.count >= num_bits_to_read);
    let bits = if num_bits_to_read >= 64 {
        s.bits as u32
    } else {
        (s.bits & ((1u64 << num_bits_to_read) - 1)) as u32
    };
    if num_bits_to_read >= 64 {
        s.bits = 0;
    } else {
        s.bits >>= num_bits_to_read;
    }
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    debug_assert!(s.bits_left > 0);
    debug_assert!(s.count <= 64);
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

// Build a Huffman tree. If `s_lookup` is Some, populate it.
fn cp_build(
    s_lookup: Option<&mut [u16; 1 << 9]>,
    tree: &mut [u32],
    lens: &[u8],
    sym_count: i32,
) -> i32 {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    for n in 0..sym_count as usize {
        counts[lens[n] as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if let Some(ref _lk) = s_lookup {
        // memset lookup to 0 - we'll do it via the borrow below.
    }
    // Reset lookup to 0 if provided
    if let Some(lk) = s_lookup.as_deref() {
        // workaround: we can't both borrow again later; need different approach
        let _ = lk;
    }
    // Re-do reset properly: take ownership of mutable reference
    let mut lookup_opt = s_lookup;
    if let Some(lk) = lookup_opt.as_deref_mut() {
        for v in lk.iter_mut() {
            *v = 0;
        }
    }

    for i in 0..sym_count as usize {
        let len = lens[i] as i32;
        if len != 0 {
            debug_assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as usize;
            first[len as usize] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(lk) = lookup_opt.as_deref_mut() {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                    while j < (1i32 << 9) {
                        lk[j as usize] = ((len << 9) | i as i32) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut cp_state_t) -> i32 {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    if !(s.bits_left / 8 <= len as i32) {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p, s.out, len as usize);
    s.out = s.out.add(len as usize);
    1
}

fn cp_fixed(s: &mut cp_state_t) -> i32 {
    // Build literal/length tree using lookup
    {
        // Take pieces out: we need &mut lookup and &mut lit at the same time.
        // We'll split the borrow manually using raw pointers, or just use local copies.
        let mut lit = s.lit;
        let mut lookup = s.lookup;
        let nlit = cp_build(Some(&mut lookup), &mut lit, &cp_fixed_table[..288], 288);
        s.lit = lit;
        s.lookup = lookup;
        s.nlit = nlit as u32;
    }
    {
        let mut dst = s.dst;
        let ndst = cp_build(None, &mut dst, &cp_fixed_table[288..], 32);
        s.dst = dst;
        s.ndst = ndst as u32;
    }
    1
}

unsafe fn cp_decode(s: &mut cp_state_t, tree: &[u32], hi_in: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: i32 = 0;
    let mut hi: i32 = hi_in;
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
    debug_assert_eq!(search >> len, key >> len);
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

unsafe fn cp_dynamic(s: &mut cp_state_t) -> i32 {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen as usize {
        lenlens[cp_permutation_order[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    {
        let mut len_arr = s.len;
        let nlen_built = cp_build(None, &mut len_arr, &lenlens, 19);
        s.len = len_arr;
        s.nlen = nlen_built as u32;
    }
    let mut lens = [0u8; 288 + 32];
    let mut n: i32 = 0;
    while n < nlit + ndst {
        let len_tree = s.len;
        let nlen_built = s.nlen as i32;
        let sym = cp_decode(s, &len_tree, nlen_built);
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
        let mut lit = s.lit;
        let mut lookup = s.lookup;
        let nlit_built = cp_build(Some(&mut lookup), &mut lit, &lens[..nlit as usize], nlit);
        s.lit = lit;
        s.lookup = lookup;
        s.nlit = nlit_built as u32;
    }
    {
        let mut dst = s.dst;
        let ndst_built = cp_build(
            None,
            &mut dst,
            &lens[nlit as usize..(nlit + ndst) as usize],
            ndst,
        );
        s.dst = dst;
        s.ndst = ndst_built as u32;
    }
    1
}

unsafe fn cp_block(s: &mut cp_state_t) -> i32 {
    loop {
        let lit_tree = s.lit;
        let nlit_local = s.nlit as i32;
        let symbol = cp_decode(s, &lit_tree, nlit_local);
        if symbol < 256 {
            if !(s.out as usize + 1 <= s.out_end as usize) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                return 0;
            }
            *s.out = symbol as u8;
            s.out = s.out.add(1);
        } else if symbol > 256 {
            let symbol_idx = (symbol - 257) as usize;
            let length = (cp_read_bits(s, cp_len_extra_bits[symbol_idx] as i32)
                + cp_len_base[symbol_idx]) as i32;
            let dst_tree = s.dst;
            let ndst_local = s.ndst as i32;
            let distance_symbol = cp_decode(s, &dst_tree, ndst_local) as usize;
            let backwards_distance = (cp_read_bits(s, cp_dist_extra_bits[distance_symbol] as i32)
                + cp_dist_base[distance_symbol]) as i32;
            if !((s.out as isize - backwards_distance as isize) >= s.begin as isize) {
                cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                return 0;
            }
            if !(s.out as usize + length as usize <= s.out_end as usize) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                return 0;
            }
            let src = s.out.sub(backwards_distance as usize);
            let dst = s.out;
            s.out = s.out.add(length as usize);
            match backwards_distance {
                1 => {
                    let v = *src;
                    for i in 0..length as usize {
                        *dst.add(i) = v;
                    }
                }
                _ => {
                    let mut len_remaining = length;
                    let mut s_p = src;
                    let mut d_p = dst;
                    while len_remaining > 0 {
                        *d_p = *s_p;
                        d_p = d_p.add(1);
                        s_p = s_p.add(1);
                        len_remaining -= 1;
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
    let mut s = Box::new(cp_state_t::new());
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;
    let in_addr = in_ptr as usize;
    let first_bytes: i32 = (((in_addr + 3) & !3usize) - in_addr) as i32;
    s.words = (in_ptr as *const u8).add(first_bytes as usize);
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes: i32 = (in_bytes - first_bytes) & 3;
    let in_bytes_ptr = in_ptr as *const u8;
    for i in 0..first_bytes as usize {
        s.bits |= (*in_bytes_ptr.add(i) as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes as usize {
        s.final_word |= (*in_bytes_ptr.add((in_bytes - last_bytes) as usize + i) as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out_ptr as *mut u8;
    s.out_end = (out_ptr as *mut u8).add(out_bytes as usize);
    s.begin = out_ptr as *mut u8;
    let mut _count: i32 = 0;
    let mut bfinal: i32;
    loop {
        bfinal = cp_read_bits(&mut s, 1) as i32;
        let btype = cp_read_bits(&mut s, 2);
        match btype {
            0 => {
                if cp_stored(&mut s) == 0 {
                    return 0;
                }
            }
            1 => {
                cp_fixed(&mut s);
                if cp_block(&mut s) == 0 {
                    return 0;
                }
            }
            2 => {
                cp_dynamic(&mut s);
                if cp_block(&mut s) == 0 {
                    return 0;
                }
            }
            3 => {
                cp_error_reason = b"Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
                return 0;
            }
            _ => {}
        }
        _count += 1;
        if bfinal != 0 {
            break;
        }
    }
    1
}

#[inline]
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

#[inline]
unsafe fn cp_make32(s: *const u8) -> u32 {
    ((*s.add(0) as u32) << 24)
        | ((*s.add(1) as u32) << 16)
        | ((*s.add(2) as u32) << 8)
        | (*s.add(3) as u32)
}

struct cp_raw_png_t {
    p: *const u8,
    end: *const u8,
}

unsafe fn cp_chunk(png: &mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    let len = cp_make32(png.p);
    let start = png.p;
    let chunk_bytes = std::slice::from_raw_parts(start.add(4), 4);
    if chunk_bytes == &chunk[..] && len >= minlen {
        let offset = (len + 12) as usize;
        if (png.p as usize) + offset <= png.end as usize {
            png.p = png.p.add(offset);
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: &mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    while (png.p as usize) < png.end as usize {
        let len = cp_make32(png.p);
        let start = png.p;
        png.p = png.p.add((len + 12) as usize);
        let chunk_bytes = std::slice::from_raw_parts(start.add(4), 4);
        if chunk_bytes == &chunk[..] && len >= minlen && png.p as usize <= png.end as usize {
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_unfilter(w: i32, h: i32, bpp: i32, raw_in: *mut u8) -> i32 {
    let len = (w * bpp) as isize;
    let mut raw = raw_in;
    let mut prev: *mut u8;
    let mut x: isize;
    if h > 0 {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                x = bpp as isize;
                while x < len {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(*raw.offset(x - bpp as isize));
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp as isize;
                while x < len {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add((*raw.offset(x - bpp as isize)) / 2);
                    x += 1;
                }
            }
            4 => {
                x = bpp as isize;
                while x < len {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(cp_paeth(*raw.offset(x - bpp as isize), 0, 0));
                    x += 1;
                }
            }
            _ => return 0,
        }
    }
    prev = raw;
    raw = raw.offset(len);
    let mut y: i32 = 1;
    while y < h {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                x = 0;
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
                x = 0;
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
                x = 0;
                while x < bpp as isize {
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add((*prev.offset(x)) / 2);
                    x += 1;
                }
                while x < len {
                    let v = ((*raw.offset(x - bpp as isize) as u32 + *prev.offset(x) as u32) / 2) as u8;
                    *raw.offset(x) = (*raw.offset(x)).wrapping_add(v);
                    x += 1;
                }
            }
            4 => {
                x = 0;
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

unsafe fn cp_convert(bpp: i32, w: i32, h: i32, src_in: *mut u8, dst_in: *mut cp_pixel_t) {
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

#[inline]
unsafe fn cp_get_alpha_for_indexed_image(index: i32, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() {
        255
    } else if index as u32 >= trns_len {
        255
    } else {
        *trns.add(index as usize)
    }
}

unsafe fn cp_depalette(
    w: i32,
    h: i32,
    src_in: *mut u8,
    dst_in: *mut cp_pixel_t,
    plte: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    let mut src = src_in;
    let mut dst = dst_in;
    for _y in 0..h {
        src = src.add(1);
        for _x in 0..w {
            let c = *src as i32;
            let r = *plte.add((c * 3) as usize);
            let g = *plte.add((c * 3 + 1) as usize);
            let b = *plte.add((c * 3 + 2) as usize);
            let a = cp_get_alpha_for_indexed_image(c, trns, trns_len);
            *dst = cp_make_pixel_a(r, g, b, a);
            dst = dst.add(1);
            src = src.add(1);
        }
    }
}

#[inline]
unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    cp_make32(chunk.sub(8))
}

#[inline]
fn cp_out_size(img: &cp_image_t, bpp: i32) -> i32 {
    (img.w + 1) * img.h * bpp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    let sig: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    let mut img = cp_image_t::default();
    let mut data: *mut u8 = ptr::null_mut();

    let mut png = cp_raw_png_t {
        p: png_data,
        end: png_data.add(png_length as usize),
    };

    // signature check
    let png_sig = std::slice::from_raw_parts(png.p, 8);
    if png_sig != &sig[..] {
        cp_error_reason = b"incorrect file signature (is this a png file?)\0".as_ptr() as *const c_char;
        // cp_err
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    png.p = png.p.add(8);

    let ihdr = cp_chunk(&mut png, b"IHDR", 13);
    if ihdr.is_null() {
        cp_error_reason = b"unable to find IHDR chunk\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }

    let bit_depth = *ihdr.add(8) as i32;
    let color_type = *ihdr.add(9) as i32;

    if bit_depth != 8 {
        cp_error_reason = b"only bit-depth of 8 is supported\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }

    let bpp: i32;
    match color_type {
        0 => bpp = 1,
        2 => bpp = 3,
        3 => bpp = 1,
        4 => bpp = 2,
        6 => bpp = 4,
        _ => {
            cp_error_reason = b"unknown color type\0".as_ptr() as *const c_char;
            libc_free(data as *mut std::ffi::c_void);
            libc_free(img.pix as *mut std::ffi::c_void);
            img.pix = ptr::null_mut();
            return img;
        }
    }

    let w = (cp_make32(ihdr) + 1) as i32;
    let h = cp_make32(ihdr.add(4)) as i32;

    if !(w >= 1) {
        cp_error_reason = b"invalid IHDR chunk found, image width was less than 1\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    if !(h >= 1) {
        cp_error_reason = b"invalid IHDR chunk found, image height was less than 1\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    if !((w as i64) * (h as i64) * (std::mem::size_of::<cp_pixel_t>() as i64) < i32::MAX as i64) {
        cp_error_reason = b"image too large\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    let pix_bytes = (w * h * std::mem::size_of::<cp_pixel_t>() as i32) as usize;
    img.w = w - 1;
    img.h = h;
    img.pix = libc_malloc(pix_bytes) as *mut cp_pixel_t;

    if img.pix.is_null() {
        cp_error_reason = b"unable to allocate raw image space\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }

    let compression = *ihdr.add(10) as i32;
    let filter = *ihdr.add(11) as i32;
    let interlace = *ihdr.add(12) as i32;

    if compression != 0 {
        cp_error_reason = b"only standard compression DEFLATE is supported\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    if filter != 0 {
        cp_error_reason = b"only standard adaptive filtering is supported\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    if interlace != 0 {
        cp_error_reason = b"interlacing is not supported\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }

    let mut first = png.p;
    let plte = cp_find(&mut png, b"PLTE", 0);
    if plte.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    let trns = cp_find(&mut png, b"tRNS", 0);
    if trns.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    let mut datalen: i32 = 0;
    {
        let mut idat = cp_find(&mut png, b"IDAT", 0);
        while !idat.is_null() {
            let len = cp_get_chunk_byte_length(idat);
            datalen += len as i32;
            idat = cp_chunk(&mut png, b"IDAT", 0);
        }
    }
    png.p = first;
    data = libc_malloc(datalen as usize) as *mut u8;
    let mut offset: i32 = 0;
    {
        let mut idat = cp_find(&mut png, b"IDAT", 0);
        while !idat.is_null() {
            let len = cp_get_chunk_byte_length(idat);
            ptr::copy_nonoverlapping(idat, data.add(offset as usize), len as usize);
            offset += len as i32;
            idat = cp_chunk(&mut png, b"IDAT", 0);
        }
    }

    if !(!data.is_null() && datalen >= 6) {
        cp_error_reason = b"corrupt zlib structure in DEFLATE stream\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    if (*data & 0x0f) != 0x08 {
        cp_error_reason = b"only zlib compression method (RFC 1950) is supported\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    if (*data & 0xf0) > 0x70 {
        cp_error_reason = b"innapropriate window size detected\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    if (*data.add(1) & 0x20) != 0 {
        cp_error_reason = b"preset dictionary is present and not supported\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    if !(cp_out_size(&img, 4) >= 1) {
        cp_error_reason = b"invalid image size found\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    if !(cp_out_size(&img, bpp) >= 1) {
        cp_error_reason = b"invalid image size found\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }

    let out = (img.pix as *mut u8)
        .add(cp_out_size(&img, 4) as usize)
        .sub(cp_out_size(&img, bpp) as usize);

    if cp_inflate(
        data.add(2) as *mut std::ffi::c_void,
        datalen - 6,
        out as *mut std::ffi::c_void,
        pix_bytes as i32,
    ) == 0
    {
        cp_error_reason = b"DEFLATE algorithm failed\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }
    if cp_unfilter(img.w, img.h, bpp, out) == 0 {
        cp_error_reason = b"invalid filter byte found\0".as_ptr() as *const c_char;
        libc_free(data as *mut std::ffi::c_void);
        libc_free(img.pix as *mut std::ffi::c_void);
        img.pix = ptr::null_mut();
        return img;
    }

    if color_type == 3 {
        if plte.is_null() {
            cp_error_reason = b"color type of indexed requires a PLTE chunk\0".as_ptr() as *const c_char;
            libc_free(data as *mut std::ffi::c_void);
            libc_free(img.pix as *mut std::ffi::c_void);
            img.pix = ptr::null_mut();
            return img;
        }
        let trns_len = if trns.is_null() {
            0
        } else {
            cp_get_chunk_byte_length(trns)
        };
        cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
    } else {
        cp_convert(bpp, img.w, img.h, out, img.pix);
    }
    libc_free(data as *mut std::ffi::c_void);
    img
}

// Use libc malloc/free to match C
extern "C" {
    fn malloc(size: usize) -> *mut std::ffi::c_void;
    fn free(ptr: *mut std::ffi::c_void);
}

unsafe fn libc_malloc(size: usize) -> *mut std::ffi::c_void {
    malloc(size)
}

unsafe fn libc_free(p: *mut std::ffi::c_void) {
    if !p.is_null() {
        free(p);
    }
}
