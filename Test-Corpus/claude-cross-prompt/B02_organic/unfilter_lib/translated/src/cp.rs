// Translation of c_src/src/lib.c to safe Rust.
// The original C is a library (no main); these functions are exposed so that
// the binary's main can link against them, but the binary itself does nothing
// (matching the C library having no executable entry point).

#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]

use std::cell::RefCell;

thread_local! {
    pub static CP_ERROR_REASON: RefCell<&'static str> = const { RefCell::new("") };
}

pub fn set_error(reason: &'static str) {
    CP_ERROR_REASON.with(|r| *r.borrow_mut() = reason);
}

pub static CP_FIXED_TABLE: [u8; 288 + 32] = [
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

pub static CP_PERMUTATION_ORDER: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

pub static CP_LEN_EXTRA_BITS: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];

pub static CP_LEN_BASE: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67,
    83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

pub static CP_DIST_EXTRA_BITS: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

pub static CP_DIST_BASE: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

pub struct CpState {
    pub bits: u64,
    pub count: i32,
    // Source bytes for the stream (replaces words pointer indexing).
    pub words: Vec<u32>,
    pub word_count: i32,
    pub word_index: i32,
    pub bits_left: i32,
    pub final_word_available: bool,
    pub final_word: u32,
    // For stored block: original input bytes plus offsets used by cp_ptr.
    pub input_bytes: Vec<u8>,
    pub first_bytes: i32,
    pub out: Vec<u8>,
    pub out_pos: usize,
    pub out_end: usize,
    pub lookup: Vec<u16>,
    pub lit: Vec<u32>,
    pub dst: Vec<u32>,
    pub len: Vec<u32>,
    pub nlit: u32,
    pub ndst: u32,
    pub nlen: u32,
}

impl CpState {
    pub fn new() -> Self {
        Self {
            bits: 0,
            count: 0,
            words: Vec::new(),
            word_count: 0,
            word_index: 0,
            bits_left: 0,
            final_word_available: false,
            final_word: 0,
            input_bytes: Vec::new(),
            first_bytes: 0,
            out: Vec::new(),
            out_pos: 0,
            out_end: 0,
            lookup: vec![0u16; 1 << 9],
            lit: vec![0u32; 288],
            dst: vec![0u32; 32],
            len: vec![0u32; 19],
            nlit: 0,
            ndst: 0,
            nlen: 0,
        }
    }
}

fn cp_would_overflow(s: &CpState, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

// Returns the byte offset within `input_bytes` corresponding to the current
// word_index minus count/8 bytes of pushback.
fn cp_ptr_offset(s: &CpState) -> usize {
    debug_assert!(s.bits_left & 7 == 0);
    // Equivalent to: (char *)(s->words + s->word_index) - (s->count / 8)
    // words start at first_bytes within input_bytes; each word is 4 bytes.
    let word_byte_offset = s.first_bytes as usize + (s.word_index as usize) * 4;
    word_byte_offset - (s.count as usize / 8)
}

fn cp_peak_bits(s: &mut CpState, _num_bits_to_read: i32) -> u64 {
    if s.count < _num_bits_to_read {
        if s.word_index < s.word_count {
            let word = s.words[s.word_index as usize];
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
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

fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

fn cp_build(
    lookup: Option<&mut [u16]>,
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
    if let Some(ref lk) = &lookup {
        // Zero out lookup table.
        let _ = lk; // placeholder; we'll handle below
    }
    // Need to clear lookup if Some.
    let mut lookup_ref = lookup;
    if let Some(ref mut lk) = lookup_ref {
        for v in lk.iter_mut() {
            *v = 0;
        }
    }

    for i in 0..sym_count as usize {
        let len_v = lens[i] as i32;
        if len_v != 0 {
            debug_assert!(len_v < 16);
            let code = codes[len_v as usize] as u32;
            codes[len_v as usize] += 1;
            let slot = first[len_v as usize] as usize;
            first[len_v as usize] += 1;
            // (code << (32 - len)) | (i << 4) | len
            let shifted = if len_v >= 32 {
                0u32
            } else {
                code.wrapping_shl((32 - len_v) as u32)
            };
            tree[slot] = shifted | ((i as u32) << 4) | (len_v as u32);
            if let Some(ref mut lk) = lookup_ref {
                if len_v <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len_v)) as usize;
                    while j < (1 << 9) {
                        lk[j] = ((len_v << 9) | i as i32) as u16;
                        j += 1usize << len_v;
                    }
                }
            }
        }
    }
    first[15]
}

fn cp_stored(s: &mut CpState) -> bool {
    cp_read_bits(s, s.count & 7);
    let len_v = cp_read_bits(s, 16) as u16;
    let nlen_v = cp_read_bits(s, 16) as u16;
    if len_v != !nlen_v {
        set_error(
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
        );
        return false;
    }
    if !(s.bits_left / 8 <= len_v as i32) {
        set_error("Stored block extends beyond end of input stream.");
        return false;
    }
    let p = cp_ptr_offset(s);
    let len_usize = len_v as usize;
    // memcpy(s->out, p, LEN) — copy from input_bytes[p..p+len] to s.out[s.out_pos..]
    for i in 0..len_usize {
        s.out[s.out_pos + i] = s.input_bytes[p + i];
    }
    s.out_pos += len_usize;
    true
}

fn cp_fixed(s: &mut CpState) -> bool {
    // Build literal table (with lookup).
    let mut lit = std::mem::take(&mut s.lit);
    let mut lookup = std::mem::take(&mut s.lookup);
    s.nlit = cp_build(Some(&mut lookup), &mut lit, &CP_FIXED_TABLE[..288], 288) as u32;
    s.lit = lit;
    s.lookup = lookup;
    // Build distance table (no lookup).
    let mut dst = std::mem::take(&mut s.dst);
    s.ndst = cp_build(None, &mut dst, &CP_FIXED_TABLE[288..288 + 32], 32) as u32;
    s.dst = dst;
    true
}

fn cp_decode(s: &mut CpState, tree: &[u32], hi_in: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;
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
    let _len = 32 - (key & 0xF);
    let code_bits = (key & 0xF) as i32;
    let _code = cp_consume_bits(s, code_bits);
    ((key >> 4) & 0xFFF) as i32
}

fn cp_dynamic(s: &mut CpState) -> bool {
    let mut lenlens = [0u8; 19];
    let nlit_count = 257 + cp_read_bits(s, 5) as i32;
    let ndst_count = 1 + cp_read_bits(s, 5) as i32;
    let nlen_count = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen_count as usize {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    let mut len_tree = std::mem::take(&mut s.len);
    s.nlen = cp_build(None, &mut len_tree, &lenlens, 19) as u32;
    s.len = len_tree;

    let mut lens = vec![0u8; 288 + 32];
    let total = nlit_count + ndst_count;
    let len_tree_local = s.len.clone();
    let nlen_local = s.nlen as i32;

    let mut n: i32 = 0;
    while n < total {
        let sym = cp_decode(s, &len_tree_local, nlen_local);
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as i32;
                while i > 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    n += 1;
                    i -= 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as i32;
                while i > 0 {
                    lens[n as usize] = 0;
                    n += 1;
                    i -= 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as i32;
                while i > 0 {
                    lens[n as usize] = 0;
                    n += 1;
                    i -= 1;
                }
            }
            _ => {
                lens[n as usize] = sym as u8;
                n += 1;
            }
        }
    }

    let mut lit = std::mem::take(&mut s.lit);
    let mut lookup = std::mem::take(&mut s.lookup);
    s.nlit = cp_build(
        Some(&mut lookup),
        &mut lit,
        &lens[..nlit_count as usize],
        nlit_count,
    ) as u32;
    s.lit = lit;
    s.lookup = lookup;

    let mut dst = std::mem::take(&mut s.dst);
    s.ndst = cp_build(
        None,
        &mut dst,
        &lens[nlit_count as usize..(nlit_count + ndst_count) as usize],
        ndst_count,
    ) as u32;
    s.dst = dst;
    true
}

fn cp_block(s: &mut CpState) -> bool {
    loop {
        let lit_tree = s.lit.clone();
        let nlit = s.nlit as i32;
        let symbol = cp_decode(s, &lit_tree, nlit);
        if symbol < 256 {
            if !(s.out_pos + 1 <= s.out_end) {
                set_error("Attempted to overwrite out buffer while outputting a symbol.");
                return false;
            }
            s.out[s.out_pos] = symbol as u8;
            s.out_pos += 1;
        } else if symbol > 256 {
            let symbol_idx = (symbol - 257) as usize;
            let extra = CP_LEN_EXTRA_BITS[symbol_idx] as i32;
            let length = (cp_read_bits(s, extra) + CP_LEN_BASE[symbol_idx]) as i32;
            let dst_tree = s.dst.clone();
            let ndst = s.ndst as i32;
            let distance_symbol = cp_decode(s, &dst_tree, ndst) as usize;
            let extra_d = CP_DIST_EXTRA_BITS[distance_symbol] as i32;
            let backwards_distance =
                (cp_read_bits(s, extra_d) + CP_DIST_BASE[distance_symbol]) as i32;
            if !((s.out_pos as i32) - backwards_distance >= 0) {
                set_error(
                    "Attempted to write before out buffer (invalid backwards distance).",
                );
                return false;
            }
            if !(s.out_pos + length as usize <= s.out_end) {
                set_error("Attempted to overwrite out buffer while outputting a string.");
                return false;
            }
            let src_pos = s.out_pos - backwards_distance as usize;
            let dst_pos = s.out_pos;
            s.out_pos += length as usize;
            if backwards_distance == 1 {
                let v = s.out[src_pos];
                for i in 0..length as usize {
                    s.out[dst_pos + i] = v;
                }
            } else {
                // Sequential copy that observes overlap (matches C while loop).
                for i in 0..length as usize {
                    s.out[dst_pos + i] = s.out[src_pos + i];
                }
            }
        } else {
            break;
        }
    }
    true
}

pub fn cp_inflate(input: &[u8], out: &mut [u8]) -> bool {
    let in_bytes = input.len() as i32;
    let out_bytes = out.len();
    let mut s = CpState::new();
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;

    // Match the C alignment math but on a logical offset rather than pointer.
    // first_bytes = ((input_addr + 3) & ~3) - input_addr — i.e. bytes needed
    // to align to 4. In Rust we can choose the input as already-byte-aligned
    // (offset 0), since we own a Vec. To mirror behavior identically, we use
    // 0 here: the C code's per-input padding only mattered for unaligned in
    // pointers from the caller. We replicate the structure (copy first_bytes
    // into bits, treat the remaining as words) but with first_bytes = 0.
    let first_bytes = 0i32;
    s.first_bytes = first_bytes;

    let remaining = in_bytes - first_bytes;
    let word_count = remaining / 4;
    s.word_count = word_count;
    let last_bytes = remaining & 3;

    s.input_bytes = input.to_vec();

    // Build u32 word array from input bytes (little-endian, matching x86 C).
    let mut words = Vec::with_capacity(word_count as usize);
    for i in 0..word_count as usize {
        let off = first_bytes as usize + i * 4;
        let w = (input[off] as u32)
            | ((input[off + 1] as u32) << 8)
            | ((input[off + 2] as u32) << 16)
            | ((input[off + 3] as u32) << 24);
        words.push(w);
    }
    s.words = words;

    for i in 0..first_bytes as usize {
        s.bits |= (input[i] as u64) << (i * 8);
    }
    s.final_word_available = last_bytes != 0;
    s.final_word = 0;
    for i in 0..last_bytes as usize {
        s.final_word |= (input[in_bytes as usize - last_bytes as usize + i] as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out.to_vec();
    s.out_pos = 0;
    s.out_end = out_bytes;

    let mut _count = 0;
    loop {
        let bfinal = cp_read_bits(&mut s, 1);
        let btype = cp_read_bits(&mut s, 2);
        match btype {
            0 => {
                if !cp_stored(&mut s) {
                    return false;
                }
            }
            1 => {
                cp_fixed(&mut s);
                if !cp_block(&mut s) {
                    return false;
                }
            }
            2 => {
                cp_dynamic(&mut s);
                if !cp_block(&mut s) {
                    return false;
                }
            }
            3 => {
                set_error("Detected unknown block type within input stream.");
                return false;
            }
            _ => {}
        }
        _count += 1;
        if bfinal != 0 {
            break;
        }
    }
    // Copy back results into out.
    out.copy_from_slice(&s.out);
    true
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

pub fn unfilter(w: i32, h: i32, bpp: i32, raw: &mut [u8]) -> i32 {
    let len = (w * bpp) as usize;
    let bpp_u = bpp as usize;
    // Cursor within `raw` (mirrors the moving `raw` pointer in C).
    let mut cur: usize = 0;

    if h > 0 {
        let filter = raw[cur];
        cur += 1;
        match filter {
            0 => {}
            1 => {
                for x in bpp_u..len {
                    raw[cur + x] = raw[cur + x].wrapping_add(raw[cur + x - bpp_u]);
                }
            }
            2 => {}
            3 => {
                for x in bpp_u..len {
                    raw[cur + x] = raw[cur + x].wrapping_add(raw[cur + x - bpp_u] / 2);
                }
            }
            4 => {
                for x in bpp_u..len {
                    let v = cp_paeth(raw[cur + x - bpp_u], 0, 0);
                    raw[cur + x] = raw[cur + x].wrapping_add(v);
                }
            }
            _ => return 0,
        }
    }

    let mut prev = cur;
    cur += len;

    let mut y = 1;
    while y < h {
        let filter = raw[cur];
        cur += 1;
        match filter {
            0 => {}
            1 => {
                for x in 0..bpp_u {
                    raw[cur + x] = raw[cur + x].wrapping_add(0);
                }
                for x in bpp_u..len {
                    raw[cur + x] = raw[cur + x].wrapping_add(raw[cur + x - bpp_u]);
                }
            }
            2 => {
                for x in 0..bpp_u {
                    raw[cur + x] = raw[cur + x].wrapping_add(raw[prev + x]);
                }
                for x in bpp_u..len {
                    raw[cur + x] = raw[cur + x].wrapping_add(raw[prev + x]);
                }
            }
            3 => {
                for x in 0..bpp_u {
                    raw[cur + x] = raw[cur + x].wrapping_add(raw[prev + x] / 2);
                }
                for x in bpp_u..len {
                    let sum = raw[cur + x - bpp_u] as u32 + raw[prev + x] as u32;
                    raw[cur + x] = raw[cur + x].wrapping_add((sum / 2) as u8);
                }
            }
            4 => {
                for x in 0..bpp_u {
                    raw[cur + x] = raw[cur + x].wrapping_add(raw[prev + x]);
                }
                for x in bpp_u..len {
                    let v = cp_paeth(raw[cur + x - bpp_u], raw[prev + x], raw[prev + x - bpp_u]);
                    raw[cur + x] = raw[cur + x].wrapping_add(v);
                }
            }
            _ => return 0,
        }
        prev = cur;
        cur += len;
        y += 1;
    }
    1
}
