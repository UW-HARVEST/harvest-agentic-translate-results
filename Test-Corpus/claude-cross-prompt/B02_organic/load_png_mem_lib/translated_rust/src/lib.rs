// Translation of c_src/src/lib.c (PNG loading library)
// Reproduces exact behavior of the original C code, including any quirks.

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::needless_range_loop)]

#[derive(Clone, Copy, Default, Debug)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Default, Debug)]
pub struct CpImage {
    pub w: i32,
    pub h: i32,
    pub pix: Vec<CpPixel>,
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> CpPixel {
    CpPixel { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> CpPixel {
    CpPixel { r, g, b, a: 0xFF }
}

pub static mut CP_ERROR_REASON: &str = "";

#[rustfmt::skip]
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

struct CpState {
    bits: u64,
    count: i32,
    // We work over a flat byte buffer, but track word-aligned reads explicitly
    input: Vec<u8>,            // copy of the input slice
    input_offset: usize,       // start offset for the words array within input (after first_bytes)
    word_count: i32,
    word_index: i32,
    bits_left: i32,
    final_word_available: i32,
    final_word: u32,
    out: Vec<u8>,
    out_pos: usize,
    out_end: usize,
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
            input: Vec::new(),
            input_offset: 0,
            word_count: 0,
            word_index: 0,
            bits_left: 0,
            final_word_available: 0,
            final_word: 0,
            out: Vec::new(),
            out_pos: 0,
            out_end: 0,
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

fn cp_would_overflow(s: &CpState, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

// Get the position in the input that maps to the current pointer
// Returns offset into s.input
fn cp_ptr_offset(s: &CpState) -> usize {
    debug_assert!((s.bits_left & 7) == 0);
    // (char*)(s->words + s->word_index) - (s->count / 8)
    s.input_offset + (s.word_index as usize) * 4 - (s.count as usize / 8)
}

fn cp_peak_bits(s: &mut CpState, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            // Read u32 from input at s.input_offset + word_index*4
            let off = s.input_offset + (s.word_index as usize) * 4;
            let word = u32::from_le_bytes([
                s.input[off],
                s.input[off + 1],
                s.input[off + 2],
                s.input[off + 3],
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

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    debug_assert!(s.count >= num_bits_to_read);
    let mask: u64 = if num_bits_to_read >= 64 {
        u64::MAX
    } else {
        ((1u64) << num_bits_to_read) - 1
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
    if let Some(ref state) = s {
        // We need a mutable borrow but we just used s mutably; we'll restructure.
        let _ = state;
    }
    // Use option pattern via index splitting: we'll do lookups separately.
    // Restructure: re-borrow as needed.
    // We need to clear lookup if s is Some.
    // To avoid reborrow gymnastics, accept Option and process inline:
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
                tree[slot] =
                    (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                    while j < (1 << 9) {
                        state.lookup[j as usize] =
                            ((len << 9) | (i as i32)) as u16;
                        j += 1 << len;
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
                tree[slot] =
                    (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            }
        }
    }
    first[15]
}

fn cp_stored(s: &mut CpState) -> i32 {
    let extra = s.count & 7;
    cp_read_bits(s, extra);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if !(len == !nlen) {
        unsafe {
            CP_ERROR_REASON =
                "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
        }
        return 0;
    }
    if !(s.bits_left / 8 <= len as i32) {
        unsafe {
            CP_ERROR_REASON = "Stored block extends beyond end of input stream.";
        }
        return 0;
    }
    let p_off = cp_ptr_offset(s);
    let bytes_to_copy = len as usize;
    // memcpy(s->out, p, LEN);
    for i in 0..bytes_to_copy {
        s.out[s.out_pos + i] = s.input[p_off + i];
    }
    s.out_pos += bytes_to_copy;
    1
}

fn cp_fixed(s: &mut CpState) -> i32 {
    let mut lit = [0u32; 288];
    let mut dst = [0u32; 32];
    let nlit = cp_build(Some(s), &mut lit, &CP_FIXED_TABLE[..288], 288);
    let ndst = cp_build(None, &mut dst, &CP_FIXED_TABLE[288..(288 + 32)], 32);
    s.lit = lit;
    s.dst = dst;
    s.nlit = nlit as u32;
    s.ndst = ndst as u32;
    1
}

fn cp_decode(s: &mut CpState, tree: &[u32], hi_in: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: i32 = 0;
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
    let len_field = 32 - (key & 0xF);
    debug_assert!((search >> len_field) == (key >> len_field));
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

fn cp_dynamic(s: &mut CpState) -> i32 {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen as usize {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    let mut len_tree = [0u32; 19];
    let nlen_built = cp_build(None, &mut len_tree, &lenlens, 19);
    s.len = len_tree;
    s.nlen = nlen_built as u32;

    let mut lens = [0u8; 288 + 32];
    let mut n: i32 = 0;
    let total = nlit + ndst;
    while n < total {
        let len_tree_local = s.len;
        let nlen_local = s.nlen as i32;
        let sym = cp_decode(s, &len_tree_local, nlen_local);
        match sym {
            16 => {
                let cnt = 3 + cp_read_bits(s, 2) as i32;
                for _ in 0..cnt {
                    lens[n as usize] = lens[(n - 1) as usize];
                    n += 1;
                }
            }
            17 => {
                let cnt = 3 + cp_read_bits(s, 3) as i32;
                for _ in 0..cnt {
                    lens[n as usize] = 0;
                    n += 1;
                }
            }
            18 => {
                let cnt = 11 + cp_read_bits(s, 7) as i32;
                for _ in 0..cnt {
                    lens[n as usize] = 0;
                    n += 1;
                }
            }
            _ => {
                lens[n as usize] = sym as u8;
                n += 1;
            }
        }
    }
    let mut lit_tree = [0u32; 288];
    let nlit_built = cp_build(Some(s), &mut lit_tree, &lens[..nlit as usize], nlit as usize);
    s.lit = lit_tree;
    s.nlit = nlit_built as u32;
    let mut dst_tree = [0u32; 32];
    let ndst_built = cp_build(
        None,
        &mut dst_tree,
        &lens[nlit as usize..(nlit + ndst) as usize],
        ndst as usize,
    );
    s.dst = dst_tree;
    s.ndst = ndst_built as u32;
    1
}

fn cp_block(s: &mut CpState) -> i32 {
    loop {
        let lit_tree = s.lit;
        let nlit = s.nlit as i32;
        let symbol = cp_decode(s, &lit_tree, nlit);
        if symbol < 256 {
            if !(s.out_pos + 1 <= s.out_end) {
                unsafe {
                    CP_ERROR_REASON =
                        "Attempted to overwrite out buffer while outputting a symbol.";
                }
                return 0;
            }
            s.out[s.out_pos] = symbol as u8;
            s.out_pos += 1;
        } else if symbol > 256 {
            let symbol_idx = (symbol - 257) as usize;
            let extra_bits_len = CP_LEN_EXTRA_BITS[symbol_idx] as i32;
            let length = (cp_read_bits(s, extra_bits_len)
                + CP_LEN_BASE[symbol_idx]) as i32;
            let dst_tree = s.dst;
            let ndst = s.ndst as i32;
            let distance_symbol = cp_decode(s, &dst_tree, ndst) as usize;
            let extra_dist_bits = CP_DIST_EXTRA_BITS[distance_symbol] as i32;
            let backwards_distance = (cp_read_bits(s, extra_dist_bits)
                + CP_DIST_BASE[distance_symbol])
                as i32;
            // s.out - backwards_distance >= s.begin   (begin is 0)
            if !((s.out_pos as i32) - backwards_distance >= 0) {
                unsafe {
                    CP_ERROR_REASON =
                        "Attempted to write before out buffer (invalid backwards distance).";
                }
                return 0;
            }
            if !(s.out_pos + length as usize <= s.out_end) {
                unsafe {
                    CP_ERROR_REASON =
                        "Attempted to overwrite out buffer while outputting a string.";
                }
                return 0;
            }
            let mut src_pos = (s.out_pos as i32 - backwards_distance) as usize;
            let mut dst_pos = s.out_pos;
            s.out_pos += length as usize;
            if backwards_distance == 1 {
                let val = s.out[src_pos];
                for i in 0..length as usize {
                    s.out[dst_pos + i] = val;
                }
            } else {
                let mut remaining = length;
                while remaining > 0 {
                    s.out[dst_pos] = s.out[src_pos];
                    dst_pos += 1;
                    src_pos += 1;
                    remaining -= 1;
                }
            }
        } else {
            break;
        }
    }
    1
}

pub fn cp_inflate(input: &[u8], out: &mut [u8]) -> i32 {
    let in_bytes = input.len() as i32;
    let out_bytes = out.len() as i32;
    let mut s = CpState::new();
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;
    // first_bytes: align input pointer to 4 bytes; since we use a Vec we always
    // allocate aligned memory, so first_bytes effectively becomes 0. Reproduce
    // the C code logic against that pretense - the memory layout in C depends on
    // the actual input pointer, but for our purpose we copy the input into a Vec
    // and treat it as 4-byte aligned. This matches what most callers see.
    s.input = input.to_vec();
    let in_ptr_align: usize = 0;
    let first_bytes: i32 = (((in_ptr_align + 3) & !3) - in_ptr_align) as i32;
    s.input_offset = first_bytes as usize;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes as usize {
        s.bits |= (s.input[i] as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes as usize {
        s.final_word |=
            (s.input[(in_bytes as usize) - last_bytes as usize + i] as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out.to_vec();
    s.out_pos = 0;
    s.out_end = out_bytes as usize;
    let mut count = 0;
    let mut bfinal: u32;
    loop {
        bfinal = cp_read_bits(&mut s, 1);
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
                unsafe {
                    CP_ERROR_REASON = "Detected unknown block type within input stream.";
                }
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
    // copy back
    out.copy_from_slice(&s.out);
    1
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

struct CpRawPng<'a> {
    data: &'a [u8],
    p: usize,
    end: usize,
}

fn cp_make32(s: &[u8]) -> u32 {
    ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | (s[3] as u32)
}

// Returns Some((offset_within_data_of_chunk_data, len)) if chunk matches and fits.
fn cp_chunk(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> Option<usize> {
    if png.p + 8 > png.end {
        return None;
    }
    let len = cp_make32(&png.data[png.p..png.p + 4]);
    let start = png.p;
    if &png.data[start + 4..start + 8] == chunk && len >= minlen {
        let offset = (len + 12) as usize;
        if png.p + offset <= png.end {
            png.p += offset;
            return Some(start + 8);
        }
    }
    None
}

fn cp_find(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> Option<usize> {
    while png.p < png.end {
        if png.p + 8 > png.end {
            return None;
        }
        let len = cp_make32(&png.data[png.p..png.p + 4]);
        let start = png.p;
        png.p = png.p.wrapping_add(len as usize + 12);
        if start + 8 > png.data.len() {
            return None;
        }
        if &png.data[start + 4..start + 8] == chunk && len >= minlen && png.p <= png.end {
            return Some(start + 8);
        }
    }
    None
}

fn cp_unfilter(w: i32, h: i32, bpp: i32, raw: &mut [u8]) -> i32 {
    let len = (w * bpp) as usize;
    let mut row_start: usize = 0;
    if h > 0 {
        let filter = raw[row_start];
        row_start += 1;
        match filter {
            0 => {}
            1 => {
                for x in (bpp as usize)..len {
                    raw[row_start + x] = raw[row_start + x].wrapping_add(raw[row_start + x - bpp as usize]);
                }
            }
            2 => {}
            3 => {
                for x in (bpp as usize)..len {
                    raw[row_start + x] =
                        raw[row_start + x].wrapping_add(raw[row_start + x - bpp as usize] / 2);
                }
            }
            4 => {
                for x in (bpp as usize)..len {
                    raw[row_start + x] = raw[row_start + x]
                        .wrapping_add(cp_paeth(raw[row_start + x - bpp as usize], 0, 0));
                }
            }
            _ => return 0,
        }
    }
    let mut prev_start = row_start;
    row_start += len;
    for _y in 1..h {
        let filter = raw[row_start];
        row_start += 1;
        match filter {
            0 => {}
            1 => {
                for x in 0..(bpp as usize) {
                    raw[row_start + x] = raw[row_start + x].wrapping_add(0);
                }
                for x in (bpp as usize)..len {
                    raw[row_start + x] =
                        raw[row_start + x].wrapping_add(raw[row_start + x - bpp as usize]);
                }
            }
            2 => {
                for x in 0..(bpp as usize) {
                    raw[row_start + x] = raw[row_start + x].wrapping_add(raw[prev_start + x]);
                }
                for x in (bpp as usize)..len {
                    raw[row_start + x] = raw[row_start + x].wrapping_add(raw[prev_start + x]);
                }
            }
            3 => {
                for x in 0..(bpp as usize) {
                    raw[row_start + x] = raw[row_start + x].wrapping_add(raw[prev_start + x] / 2);
                }
                for x in (bpp as usize)..len {
                    let val = ((raw[row_start + x - bpp as usize] as u16
                        + raw[prev_start + x] as u16)
                        / 2) as u8;
                    raw[row_start + x] = raw[row_start + x].wrapping_add(val);
                }
            }
            4 => {
                for x in 0..(bpp as usize) {
                    raw[row_start + x] = raw[row_start + x].wrapping_add(raw[prev_start + x]);
                }
                for x in (bpp as usize)..len {
                    let pae = cp_paeth(
                        raw[row_start + x - bpp as usize],
                        raw[prev_start + x],
                        raw[prev_start + x - bpp as usize],
                    );
                    raw[row_start + x] = raw[row_start + x].wrapping_add(pae);
                }
            }
            _ => return 0,
        }
        prev_start = row_start;
        row_start += len;
    }
    1
}

fn cp_convert(bpp: i32, w: i32, h: i32, src: &[u8], dst: &mut [CpPixel]) {
    let mut s_pos: usize = 0;
    let mut d_pos: usize = 0;
    for _y in 0..h {
        s_pos += 1;
        for _x in 0..w {
            match bpp {
                1 => {
                    dst[d_pos] = cp_make_pixel(src[s_pos], src[s_pos], src[s_pos]);
                    d_pos += 1;
                }
                2 => {
                    dst[d_pos] = cp_make_pixel_a(src[s_pos], src[s_pos], src[s_pos], src[s_pos + 1]);
                    d_pos += 1;
                }
                3 => {
                    dst[d_pos] = cp_make_pixel(src[s_pos], src[s_pos + 1], src[s_pos + 2]);
                    d_pos += 1;
                }
                4 => {
                    dst[d_pos] =
                        cp_make_pixel_a(src[s_pos], src[s_pos + 1], src[s_pos + 2], src[s_pos + 3]);
                    d_pos += 1;
                }
                _ => {}
            }
            s_pos += bpp as usize;
        }
    }
}

fn cp_get_alpha_for_indexed_image(index: i32, trns: Option<&[u8]>) -> u8 {
    match trns {
        None => 255,
        Some(t) => {
            if (index as usize) >= t.len() {
                255
            } else {
                t[index as usize]
            }
        }
    }
}

fn cp_depalette(
    w: i32,
    h: i32,
    src: &[u8],
    dst: &mut [CpPixel],
    plte: &[u8],
    trns: Option<&[u8]>,
) {
    let mut s_pos: usize = 0;
    let mut d_pos: usize = 0;
    for _y in 0..h {
        s_pos += 1;
        for _x in 0..w {
            let c = src[s_pos] as usize;
            let r = plte[c * 3];
            let g = plte[c * 3 + 1];
            let b = plte[c * 3 + 2];
            let a = cp_get_alpha_for_indexed_image(c as i32, trns);
            dst[d_pos] = cp_make_pixel_a(r, g, b, a);
            d_pos += 1;
            s_pos += 1;
        }
    }
}

fn cp_get_chunk_byte_length(chunk_offset: usize, data: &[u8]) -> u32 {
    cp_make32(&data[chunk_offset - 8..chunk_offset - 4])
}

fn cp_out_size(w: i32, h: i32, bpp: i32) -> i32 {
    (w + 1) * h * bpp
}

pub fn load_png_mem(png_data: &[u8]) -> CpImage {
    let png_length = png_data.len() as i32;
    let mut img = CpImage::default();
    let sig: &[u8] = b"\x89PNG\r\n\x1a\n";
    let mut png = CpRawPng {
        data: png_data,
        p: 0,
        end: png_length as usize,
    };
    if png.end < 8 || &png.data[..8] != sig {
        unsafe {
            CP_ERROR_REASON = "incorrect file signature (is this a png file?)";
        }
        return img;
    }
    png.p += 8;
    let ihdr = match cp_chunk(&mut png, b"IHDR", 13) {
        Some(o) => o,
        None => {
            unsafe {
                CP_ERROR_REASON = "unable to find IHDR chunk";
            }
            return img;
        }
    };
    let bit_depth = png.data[ihdr + 8] as i32;
    let color_type = png.data[ihdr + 9] as i32;
    if bit_depth != 8 {
        unsafe {
            CP_ERROR_REASON = "only bit-depth of 8 is supported";
        }
        return img;
    }
    let bpp = match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => {
            unsafe {
                CP_ERROR_REASON = "unknown color type";
            }
            return img;
        }
    };
    let w = (cp_make32(&png.data[ihdr..ihdr + 4]) as i32) + 1;
    let h = cp_make32(&png.data[ihdr + 4..ihdr + 8]) as i32;
    if !(w >= 1) {
        unsafe {
            CP_ERROR_REASON = "invalid IHDR chunk found, image width was less than 1";
        }
        return img;
    }
    if !(h >= 1) {
        unsafe {
            CP_ERROR_REASON = "invalid IHDR chunk found, image height was less than 1";
        }
        return img;
    }
    if !(((w as i64) * (h as i64) * (std::mem::size_of::<CpPixel>() as i64)) < i32::MAX as i64) {
        unsafe {
            CP_ERROR_REASON = "image too large";
        }
        return img;
    }
    let pix_bytes = (w as usize) * (h as usize) * std::mem::size_of::<CpPixel>();
    img.w = w - 1;
    img.h = h;
    img.pix = vec![CpPixel::default(); (w as usize) * (h as usize)];

    let compression = png.data[ihdr + 10] as i32;
    let filter = png.data[ihdr + 11] as i32;
    let interlace = png.data[ihdr + 12] as i32;
    if compression != 0 {
        unsafe {
            CP_ERROR_REASON = "only standard compression DEFLATE is supported";
        }
        img.pix.clear();
        return img;
    }
    if filter != 0 {
        unsafe {
            CP_ERROR_REASON = "only standard adaptive filtering is supported";
        }
        img.pix.clear();
        return img;
    }
    if interlace != 0 {
        unsafe {
            CP_ERROR_REASON = "interlacing is not supported";
        }
        img.pix.clear();
        return img;
    }

    let first = png.p;
    let plte = cp_find(&mut png, b"PLTE", 0);
    let first = if plte.is_none() {
        png.p = first;
        first
    } else {
        png.p
    };
    let trns = cp_find(&mut png, b"tRNS", 0);
    let first = if trns.is_none() {
        png.p = first;
        first
    } else {
        png.p
    };

    let mut datalen: i32 = 0;
    let mut idat_opt = cp_find(&mut png, b"IDAT", 0);
    while let Some(idat) = idat_opt {
        let len = cp_get_chunk_byte_length(idat, png.data);
        datalen += len as i32;
        idat_opt = cp_chunk(&mut png, b"IDAT", 0);
    }
    png.p = first;
    let mut data: Vec<u8> = vec![0u8; datalen.max(0) as usize];
    let mut offset: usize = 0;
    let mut idat_opt = cp_find(&mut png, b"IDAT", 0);
    while let Some(idat) = idat_opt {
        let len = cp_get_chunk_byte_length(idat, png.data) as usize;
        data[offset..offset + len].copy_from_slice(&png.data[idat..idat + len]);
        offset += len;
        idat_opt = cp_chunk(&mut png, b"IDAT", 0);
    }

    if !(datalen >= 6) {
        unsafe {
            CP_ERROR_REASON = "corrupt zlib structure in DEFLATE stream";
        }
        img.pix.clear();
        return img;
    }
    if !((data[0] & 0x0f) == 0x08) {
        unsafe {
            CP_ERROR_REASON = "only zlib compression method (RFC 1950) is supported";
        }
        img.pix.clear();
        return img;
    }
    if !((data[0] & 0xf0) <= 0x70) {
        unsafe {
            CP_ERROR_REASON = "innapropriate window size detected";
        }
        img.pix.clear();
        return img;
    }
    if !(data[1] & 0x20 == 0) {
        unsafe {
            CP_ERROR_REASON = "preset dictionary is present and not supported";
        }
        img.pix.clear();
        return img;
    }
    if !(cp_out_size(img.w, img.h, 4) >= 1) {
        unsafe {
            CP_ERROR_REASON = "invalid image size found";
        }
        img.pix.clear();
        return img;
    }
    if !(cp_out_size(img.w, img.h, bpp) >= 1) {
        unsafe {
            CP_ERROR_REASON = "invalid image size found";
        }
        img.pix.clear();
        return img;
    }

    let out_offset_in_pix = (cp_out_size(img.w, img.h, 4) - cp_out_size(img.w, img.h, bpp)) as usize;
    // We'll perform inflate into a temporary Vec<u8> then copy back into the pix buffer's tail.
    let mut out_buf: Vec<u8> = vec![0u8; pix_bytes];
    // The C code writes into (uint8_t*)img.pix + cp_out_size(img,4) - cp_out_size(img,bpp)
    // out_pos within pix_bytes is out_offset_in_pix. We pass the full pix_bytes buffer
    // but the inflate output is at "out", which has length out_bytes = pix_bytes (in C).
    // C code passes out_bytes = pix_bytes, but writes starting from a position INSIDE the
    // pix buffer. In our Rust translation, we pass a separate buffer and then place the
    // decoded data at the correct offset in the pixel buffer's byte view.
    let inflate_len = pix_bytes; // matches C's out_bytes argument
    let mut inflate_target: Vec<u8> = vec![0u8; inflate_len];
    if cp_inflate(&data[2..2 + (datalen as usize - 6)], &mut inflate_target) == 0 {
        unsafe {
            CP_ERROR_REASON = "DEFLATE algorithm failed";
        }
        img.pix.clear();
        return img;
    }
    // The relevant decoded data spans from index 0 to (h * (w * bpp + 1)) bytes; only this
    // data is referenced by cp_unfilter / cp_convert / cp_depalette via the offset within the
    // pixel byte view starting at out_offset_in_pix. Place it accordingly.
    // Build a scratch byte buffer representing the image's raw byte memory.
    let total_bytes = pix_bytes;
    // Copy pixels initially zero, then write out_buf starting at out_offset_in_pix
    out_buf.copy_from_slice(&vec![0u8; total_bytes]);
    let inflated_used = (img.h as usize) * ((img.w as usize) * (bpp as usize) + 1);
    out_buf[out_offset_in_pix..out_offset_in_pix + inflated_used]
        .copy_from_slice(&inflate_target[..inflated_used]);

    if cp_unfilter(img.w, img.h, bpp, &mut out_buf[out_offset_in_pix..]) == 0 {
        unsafe {
            CP_ERROR_REASON = "invalid filter byte found";
        }
        img.pix.clear();
        return img;
    }

    if color_type == 3 {
        let plte_off = match plte {
            Some(p) => p,
            None => {
                unsafe {
                    CP_ERROR_REASON = "color type of indexed requires a PLTE chunk";
                }
                img.pix.clear();
                return img;
            }
        };
        let trns_slice: Option<&[u8]> = trns.map(|t_off| {
            let tl = cp_get_chunk_byte_length(t_off, png.data) as usize;
            &png.data[t_off..t_off + tl]
        });
        // Compute plte slice length: at minimum we need indices referenced; cheaply provide all of it.
        let plte_len = cp_get_chunk_byte_length(plte_off, png.data) as usize;
        let plte_slice = &png.data[plte_off..plte_off + plte_len];
        cp_depalette(
            img.w,
            img.h,
            &out_buf[out_offset_in_pix..],
            &mut img.pix,
            plte_slice,
            trns_slice,
        );
    } else {
        cp_convert(bpp, img.w, img.h, &out_buf[out_offset_in_pix..], &mut img.pix);
    }
    img
}
