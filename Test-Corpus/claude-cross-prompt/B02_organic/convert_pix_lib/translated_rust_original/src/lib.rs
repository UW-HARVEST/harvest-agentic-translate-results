// Translated from c_src/src/lib.c — preserves original behavior, including any bugs.

#![allow(dead_code)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_memcpy)]

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

pub static mut cp_error_reason: Option<&'static str> = None;

pub static cp_fixed_table: [u8; 288 + 32] = [
    // 144 entries of 8 (literals 0..=143)
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
    // 112 entries of 9 (literals 144..=255)
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
    // 24 entries of 7 (literals 256..=279)
    7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
    // 8 entries of 8 (literals 280..=287)
    8, 8, 8, 8, 8, 8, 8, 8,
    // 32 entries of 5 (distance codes)
    5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
];

pub static cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

pub static cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

pub static cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

pub static cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

pub static cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

pub struct cp_state_t<'a> {
    pub bits: u64,
    pub count: i32,
    pub words: &'a [u32],
    pub word_count: i32,
    pub word_index: i32,
    pub bits_left: i32,
    pub final_word_available: i32,
    pub final_word: u32,
    pub out: &'a mut [u8],
    pub out_pos: usize,
    pub out_len: usize,
    pub lookup: [u16; 1 << 9],
    pub lit: [u32; 288],
    pub dst: [u32; 32],
    pub len: [u32; 19],
    pub nlit: u32,
    pub ndst: u32,
    pub nlen: u32,
    // Original input buffer reference, used for `cp_ptr` (stored block memcpy).
    pub input: &'a [u8],
    pub first_bytes: i32,
}

fn cp_would_overflow(s: &cp_state_t, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

/// Returns offset (in bytes) into the input buffer for stored block reads.
fn cp_ptr_offset(s: &cp_state_t) -> usize {
    assert!(s.bits_left & 7 == 0);
    // (char *)(s->words + s->word_index) - (s->count / 8)
    // word offset from input start in bytes:
    let words_byte_offset = (s.first_bytes as usize) + (s.word_index as usize) * 4;
    words_byte_offset - ((s.count as usize) / 8)
}

fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = s.words[s.word_index as usize];
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            assert!(s.word_index <= s.word_count);
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
    assert!(s.count >= num_bits_to_read);
    let mask: u64 = if num_bits_to_read >= 64 {
        u64::MAX
    } else {
        ((1u64) << num_bits_to_read).wrapping_sub(1)
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

fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!(s.bits_left > 0);
    assert!(s.count <= 64);
    assert!(!cp_would_overflow(s, num_bits_to_read));
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
    s: Option<&mut cp_state_t>,
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
    if let Some(ref s_ref) = s {
        // We can't borrow s twice; reset lookup later. Use raw pointer trick via splitting.
        let _ = s_ref;
    }

    if let Some(state) = s {
        for v in state.lookup.iter_mut() {
            *v = 0;
        }
        for i in 0..sym_count as usize {
            let len = lens[i] as i32;
            if len != 0 {
                assert!(len < 16);
                let code = codes[len as usize] as u32;
                codes[len as usize] += 1;
                let slot = first[len as usize] as u32;
                first[len as usize] += 1;
                tree[slot as usize] =
                    (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                    while j < (1 << 9) {
                        state.lookup[j as usize] =
                            (((len as u32) << 9) | (i as u32)) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    } else {
        for i in 0..sym_count as usize {
            let len = lens[i] as i32;
            if len != 0 {
                assert!(len < 16);
                let code = codes[len as usize] as u32;
                codes[len as usize] += 1;
                let slot = first[len as usize] as u32;
                first[len as usize] += 1;
                tree[slot as usize] =
                    (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            }
        }
    }
    first[15]
}

fn cp_stored(s: &mut cp_state_t) -> i32 {
    let extra = s.count & 7;
    cp_read_bits(s, extra);
    let LEN = cp_read_bits(s, 16) as u16;
    let NLEN = cp_read_bits(s, 16) as u16;
    if !(LEN == !NLEN) {
        unsafe {
            cp_error_reason = Some(
                "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
            );
        }
        return 0;
    }
    if !(s.bits_left / 8 <= LEN as i32) {
        unsafe {
            cp_error_reason = Some("Stored block extends beyond end of input stream.");
        }
        return 0;
    }
    let p = cp_ptr_offset(s);
    let len = LEN as usize;
    // memcpy(s->out, p, LEN);
    s.out[s.out_pos..s.out_pos + len].copy_from_slice(&s.input[p..p + len]);
    s.out_pos += len;
    1
}

fn cp_fixed(s: &mut cp_state_t) -> i32 {
    // Build literal tree (uses lookup table), then distance tree (no lookup).
    // We need to call cp_build twice on the state with two different output trees.
    // Take ownership of lit/dst arrays into temporary copies isn't needed since
    // we pass &mut [u32] for tree.
    let lens_lit = &cp_fixed_table[..288];
    let mut lit = s.lit;
    s.nlit = cp_build(Some(s), &mut lit, lens_lit, 288) as u32;
    s.lit = lit;

    let lens_dst = &cp_fixed_table[288..288 + 32];
    let mut dst = s.dst;
    s.ndst = cp_build(None, &mut dst, lens_dst, 32) as u32;
    s.dst = dst;
    1
}

fn cp_decode(s: &mut cp_state_t, tree: &[u32], hi_in: i32) -> i32 {
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
    if len < 32 {
        assert!((search >> len) == (key >> len));
    }
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

fn cp_dynamic(s: &mut cp_state_t) -> i32 {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen as usize {
        lenlens[cp_permutation_order[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    let mut len_tree = s.len;
    s.nlen = cp_build(None, &mut len_tree, &lenlens, 19) as u32;
    s.len = len_tree;

    let mut lens = [0u8; 288 + 32];
    let total = nlit + ndst;
    let mut n = 0i32;
    while n < total {
        let len_tree_local = s.len;
        let nlen_local = s.nlen as i32;
        let sym = cp_decode(s, &len_tree_local, nlen_local);
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
    let lit_lens = &lens[..nlit as usize];
    let mut lit = s.lit;
    s.nlit = cp_build(Some(s), &mut lit, lit_lens, nlit) as u32;
    s.lit = lit;

    let dst_lens = &lens[nlit as usize..(nlit + ndst) as usize];
    let mut dst = s.dst;
    s.ndst = cp_build(None, &mut dst, dst_lens, ndst) as u32;
    s.dst = dst;
    1
}

fn cp_block(s: &mut cp_state_t) -> i32 {
    loop {
        let lit_local = s.lit;
        let nlit_local = s.nlit as i32;
        let symbol = cp_decode(s, &lit_local, nlit_local);
        if symbol < 256 {
            if !(s.out_pos + 1 <= s.out_len) {
                unsafe {
                    cp_error_reason =
                        Some("Attempted to overwrite out buffer while outputting a symbol.");
                }
                return 0;
            }
            s.out[s.out_pos] = symbol as u8;
            s.out_pos += 1;
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = cp_read_bits(s, cp_len_extra_bits[symbol as usize] as i32) as i32
                + cp_len_base[symbol as usize] as i32;
            let dst_local = s.dst;
            let ndst_local = s.ndst as i32;
            let distance_symbol = cp_decode(s, &dst_local, ndst_local);
            let backwards_distance =
                cp_read_bits(s, cp_dist_extra_bits[distance_symbol as usize] as i32) as i32
                    + cp_dist_base[distance_symbol as usize] as i32;
            if !((s.out_pos as i32) - backwards_distance >= 0) {
                unsafe {
                    cp_error_reason = Some(
                        "Attempted to write before out buffer (invalid backwards distance).",
                    );
                }
                return 0;
            }
            if !((s.out_pos as i32) + length <= s.out_len as i32) {
                unsafe {
                    cp_error_reason =
                        Some("Attempted to overwrite out buffer while outputting a string.");
                }
                return 0;
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
                for i in 0..length as usize {
                    s.out[dst_pos + i] = s.out[src_pos + i];
                }
            }
        } else {
            break;
        }
    }
    1
}

pub fn cp_inflate(input: &[u8], output: &mut [u8]) -> i32 {
    let in_bytes = input.len() as i32;
    let out_bytes = output.len();

    // Replicate `first_bytes = (((size_t)in + 3) & ~3) - (size_t)in;` — alignment
    // depends on the input pointer. Since `&[u8]` doesn't expose that pointer to
    // safe code, we use 0 (i.e. assume aligned) which matches the well-defined
    // behavior most callers expect when buffers are 4-byte aligned.
    let first_bytes: i32 = 0;
    let words_byte_start = first_bytes as usize;
    let word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;

    // Build a Vec<u32> view of the words region.
    let mut words: Vec<u32> = Vec::with_capacity(word_count as usize);
    for i in 0..word_count as usize {
        let base = words_byte_start + i * 4;
        let w = (input[base] as u32)
            | ((input[base + 1] as u32) << 8)
            | ((input[base + 2] as u32) << 16)
            | ((input[base + 3] as u32) << 24);
        words.push(w);
    }

    let mut bits: u64 = 0;
    for i in 0..first_bytes as usize {
        bits |= (input[i] as u64) << (i * 8);
    }
    let final_word_available = if last_bytes != 0 { 1 } else { 0 };
    let mut final_word: u32 = 0;
    for i in 0..last_bytes as usize {
        final_word |= (input[in_bytes as usize - last_bytes as usize + i] as u32) << (i * 8);
    }
    let count = first_bytes * 8;

    let out_len = out_bytes;
    let mut s = cp_state_t {
        bits,
        count,
        words: &words,
        word_count,
        word_index: 0,
        bits_left: in_bytes * 8,
        final_word_available,
        final_word,
        out: output,
        out_pos: 0,
        out_len,
        lookup: [0u16; 1 << 9],
        lit: [0u32; 288],
        dst: [0u32; 32],
        len: [0u32; 19],
        nlit: 0,
        ndst: 0,
        nlen: 0,
        input,
        first_bytes,
    };

    let mut _count = 0;
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
                    cp_error_reason = Some("Detected unknown block type within input stream.");
                }
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

pub struct cp_raw_png_t<'a> {
    pub data: &'a [u8],
    pub p: usize,
    pub end: usize,
}

fn cp_make32(s: &[u8]) -> u32 {
    ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | (s[3] as u32)
}

fn cp_chunk(png: &mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> Option<usize> {
    let len = cp_make32(&png.data[png.p..png.p + 4]);
    let start = png.p;
    if &png.data[start + 4..start + 8] == chunk && len >= minlen {
        let offset = (len as usize) + 12;
        if png.p + offset <= png.end {
            png.p += offset;
            return Some(start + 8);
        }
    }
    None
}

fn cp_find(png: &mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> Option<usize> {
    while png.p < png.end {
        let len = cp_make32(&png.data[png.p..png.p + 4]);
        let start = png.p;
        png.p += (len as usize) + 12;
        if &png.data[start + 4..start + 8] == chunk && len >= minlen && png.p <= png.end {
            return Some(start + 8);
        }
    }
    None
}

fn cp_unfilter(w: i32, h: i32, bpp: i32, raw: &mut [u8]) -> i32 {
    let len = (w * bpp) as usize;
    let mut idx: usize = 0;

    if h > 0 {
        let filter = raw[idx];
        idx += 1;
        match filter {
            0 => {}
            1 => {
                for x in (bpp as usize)..len {
                    raw[idx + x] = raw[idx + x].wrapping_add(raw[idx + x - bpp as usize]);
                }
            }
            2 => {}
            3 => {
                for x in (bpp as usize)..len {
                    raw[idx + x] = raw[idx + x].wrapping_add(raw[idx + x - bpp as usize] / 2);
                }
            }
            4 => {
                for x in (bpp as usize)..len {
                    raw[idx + x] =
                        raw[idx + x].wrapping_add(cp_paeth(raw[idx + x - bpp as usize], 0, 0));
                }
            }
            _ => return 0,
        }
    }

    let mut prev = idx;
    idx += len;

    for _y in 1..h {
        let filter = raw[idx];
        idx += 1;
        match filter {
            0 => {}
            1 => {
                for x in 0..bpp as usize {
                    raw[idx + x] = raw[idx + x].wrapping_add(0);
                }
                for x in (bpp as usize)..len {
                    raw[idx + x] = raw[idx + x].wrapping_add(raw[idx + x - bpp as usize]);
                }
            }
            2 => {
                for x in 0..bpp as usize {
                    raw[idx + x] = raw[idx + x].wrapping_add(raw[prev + x]);
                }
                for x in (bpp as usize)..len {
                    raw[idx + x] = raw[idx + x].wrapping_add(raw[prev + x]);
                }
            }
            3 => {
                for x in 0..bpp as usize {
                    raw[idx + x] = raw[idx + x].wrapping_add(raw[prev + x] / 2);
                }
                for x in (bpp as usize)..len {
                    let sum = (raw[idx + x - bpp as usize] as u16 + raw[prev + x] as u16) / 2;
                    raw[idx + x] = raw[idx + x].wrapping_add(sum as u8);
                }
            }
            4 => {
                for x in 0..bpp as usize {
                    raw[idx + x] = raw[idx + x].wrapping_add(raw[prev + x]);
                }
                for x in (bpp as usize)..len {
                    raw[idx + x] = raw[idx + x].wrapping_add(cp_paeth(
                        raw[idx + x - bpp as usize],
                        raw[prev + x],
                        raw[prev + x - bpp as usize],
                    ));
                }
            }
            _ => return 0,
        }
        prev = idx;
        idx += len;
    }
    1
}

pub fn convert_pix(bpp: i32, w: i32, h: i32, src: &[u8], dst: &mut [cp_pixel_t]) {
    let mut s_idx: usize = 0;
    let mut d_idx: usize = 0;
    for _y in 0..h {
        s_idx += 1;
        for _x in 0..w {
            match bpp {
                1 => {
                    dst[d_idx] = cp_make_pixel(src[s_idx], src[s_idx], src[s_idx]);
                    d_idx += 1;
                }
                2 => {
                    dst[d_idx] =
                        cp_make_pixel_a(src[s_idx], src[s_idx], src[s_idx], src[s_idx + 1]);
                    d_idx += 1;
                }
                3 => {
                    dst[d_idx] = cp_make_pixel(src[s_idx], src[s_idx + 1], src[s_idx + 2]);
                    d_idx += 1;
                }
                4 => {
                    dst[d_idx] = cp_make_pixel_a(
                        src[s_idx],
                        src[s_idx + 1],
                        src[s_idx + 2],
                        src[s_idx + 3],
                    );
                    d_idx += 1;
                }
                _ => {}
            }
            s_idx += bpp as usize;
        }
    }
}
