// Translation of c_src/src/lib.c (a DEFLATE decompressor)

use std::os::raw::{c_int, c_void};
use std::ptr;

static mut CP_ERROR_REASON: *const u8 = ptr::null();

#[allow(dead_code)]
#[derive(Copy, Clone)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[allow(dead_code)]
struct CpImage {
    w: c_int,
    h: c_int,
    pix: *mut CpPixel,
}

#[allow(dead_code)]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> CpPixel {
    CpPixel { r, g, b, a }
}

#[allow(dead_code)]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> CpPixel {
    CpPixel { r, g, b, a: 0xFF }
}

static CP_FIXED_TABLE: [u8; 288 + 32] = {
    let mut t = [0u8; 288 + 32];
    let mut i = 0;
    while i < 144 {
        t[i] = 8;
        i += 1;
    }
    while i < 256 {
        t[i] = 9;
        i += 1;
    }
    while i < 280 {
        t[i] = 7;
        i += 1;
    }
    while i < 288 {
        t[i] = 8;
        i += 1;
    }
    while i < 288 + 32 {
        t[i] = 5;
        i += 1;
    }
    t
};

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

#[repr(C)]
struct CpState {
    bits: u64,
    count: c_int,
    words: *const u32,
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

impl CpState {
    fn new() -> Self {
        CpState {
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

fn cp_would_overflow(s: &CpState, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

unsafe fn cp_ptr(s: &CpState) -> *const u8 {
    debug_assert!((s.bits_left & 7) == 0);
    // (char *)(s->words + s->word_index) - (s->count / 8)
    let base = s.words.offset(s.word_index as isize) as *const u8;
    base.offset(-((s.count / 8) as isize))
}

unsafe fn cp_peak_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            // Read potentially-unaligned u32 (matches C's pointer-cast behavior on x86)
            let word_ptr = s.words.offset(s.word_index as isize);
            let word = ptr::read_unaligned(word_ptr);
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
    let mask: u64 = if num_bits_to_read >= 64 {
        u64::MAX
    } else {
        (1u64 << num_bits_to_read) - 1
    };
    let bits = (s.bits & mask) as u32;
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
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

/// Build a Huffman tree.
///
/// `s` is optional; when provided, the lookup table is rebuilt for short codes.
fn cp_build(
    s: Option<&mut CpState>,
    tree: &mut [u32],
    lens: &[u8],
    sym_count: usize,
) -> c_int {
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

    if let Some(ref state) = s {
        // Need to clear lookup; we'll do it through a re-borrow below.
        let _ = state;
    }

    // We need mutable access to s.lookup. Re-acquire by matching.
    if let Some(state) = s {
        for slot in state.lookup.iter_mut() {
            *slot = 0;
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
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        state.lookup[j] = ((len << 9) as u16) | (i as u16);
                        j += 1usize << len;
                    }
                }
            }
        }
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
    }

    first[15]
}

unsafe fn cp_stored(s: &mut CpState) -> c_int {
    cp_read_bits(s, s.count & 7);
    let len_val = cp_read_bits(s, 16) as u16;
    let nlen_val = cp_read_bits(s, 16) as u16;
    if len_val != !nlen_val {
        CP_ERROR_REASON =
            b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0"
                .as_ptr();
        return 0;
    }
    if !(s.bits_left / 8 <= len_val as c_int) {
        CP_ERROR_REASON = b"Stored block extends beyond end of input stream.\0".as_ptr();
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p, s.out, len_val as usize);
    s.out = s.out.add(len_val as usize);
    1
}

unsafe fn cp_fixed(s: &mut CpState) -> c_int {
    // Build literal/length tree using the first 288 entries of CP_FIXED_TABLE
    let mut lit = s.lit;
    s.nlit = cp_build(Some(s), &mut lit, &CP_FIXED_TABLE[..288], 288) as u32;
    s.lit = lit;

    let mut dst = s.dst;
    s.ndst = cp_build(None, &mut dst, &CP_FIXED_TABLE[288..], 32) as u32;
    s.dst = dst;
    1
}

unsafe fn cp_decode(s: &mut CpState, tree: &[u32], hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
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
    debug_assert_eq!(search >> len, key >> len);
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: &mut CpState) -> c_int {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as usize;
    let ndst = 1 + cp_read_bits(s, 5) as usize;
    let nlen = 4 + cp_read_bits(s, 4) as usize;
    for i in 0..nlen {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    let mut len_tree = s.len;
    s.nlen = cp_build(None, &mut len_tree, &lenlens, 19) as u32;
    s.len = len_tree;

    let mut lens = [0u8; 288 + 32];
    let mut n: usize = 0;
    while n < nlit + ndst {
        let len_tree_local = s.len;
        let nlen_local = s.nlen as c_int;
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

    let mut lit_tree = s.lit;
    s.nlit = cp_build(Some(s), &mut lit_tree, &lens[..nlit], nlit) as u32;
    s.lit = lit_tree;

    let mut dst_tree = s.dst;
    s.ndst = cp_build(None, &mut dst_tree, &lens[nlit..nlit + ndst], ndst) as u32;
    s.dst = dst_tree;

    1
}

unsafe fn cp_block(s: &mut CpState) -> c_int {
    loop {
        let lit_tree = s.lit;
        let nlit = s.nlit as c_int;
        let symbol = cp_decode(s, &lit_tree, nlit);
        if symbol < 256 {
            if !(s.out.add(1) <= s.out_end) {
                CP_ERROR_REASON =
                    b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr();
                return 0;
            }
            *s.out = symbol as u8;
            s.out = s.out.add(1);
        } else if symbol > 256 {
            let symbol_idx: usize = (symbol as i32 - 257) as usize;
            let length = (cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol_idx] as c_int)
                + CP_LEN_BASE[symbol_idx]) as i32;
            let dst_tree = s.dst;
            let ndst = s.ndst as c_int;
            let distance_symbol = cp_decode(s, &dst_tree, ndst) as usize;
            let backwards_distance = (cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol] as c_int)
                + CP_DIST_BASE[distance_symbol]) as i32;
            if !(s.out.offset(-(backwards_distance as isize)) >= s.begin) {
                CP_ERROR_REASON =
                    b"Attempted to write before out buffer (invalid backwards distance).\0"
                        .as_ptr();
                return 0;
            }
            if !(s.out.offset(length as isize) <= s.out_end) {
                CP_ERROR_REASON =
                    b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr();
                return 0;
            }
            let mut src = s.out.offset(-(backwards_distance as isize));
            let mut dst = s.out;
            s.out = s.out.offset(length as isize);
            if backwards_distance == 1 {
                ptr::write_bytes(dst, *src, length as usize);
            } else {
                let mut remaining = length;
                while remaining > 0 {
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
    1
}

/// Decompress a raw DEFLATE stream.
///
/// # Safety
///
/// `in_ptr` must point to at least `in_bytes` bytes of valid memory, and
/// `out` must point to at least `out_bytes` bytes of writable memory.
#[no_mangle]
pub unsafe extern "C" fn pinflate(
    in_ptr: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut state = Box::new(CpState::new());
    let s = state.as_mut();

    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;

    let in_addr = in_ptr as usize;
    let first_bytes = (((in_addr + 3) & !3usize) - in_addr) as c_int;
    s.words = (in_ptr as *const u8).offset(first_bytes as isize) as *const u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;

    let in_u8 = in_ptr as *const u8;
    for i in 0..first_bytes {
        s.bits |= (*in_u8.offset(i as isize) as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        s.final_word |= (*in_u8.offset((in_bytes - last_bytes + i) as isize) as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out as *mut u8;
    s.out_end = (out as *mut u8).offset(out_bytes as isize);
    s.begin = out as *mut u8;

    let mut count = 0;
    loop {
        let bfinal = cp_read_bits(s, 1);
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
                CP_ERROR_REASON = b"Detected unknown block type within input stream.\0".as_ptr();
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
