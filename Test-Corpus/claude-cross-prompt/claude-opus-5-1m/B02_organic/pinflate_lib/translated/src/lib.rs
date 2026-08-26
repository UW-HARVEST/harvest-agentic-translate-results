// Translated from c_src/src/lib.c
// DEFLATE decoder

// Helper to set error reason from any thread-local-ish context.
// We use a thread_local for safety.
thread_local! {
    pub static ERROR_REASON: std::cell::Cell<&'static str> = const { std::cell::Cell::new("") };
}

fn set_error(s: &'static str) {
    ERROR_REASON.with(|e| e.set(s));
}

#[allow(dead_code)]
pub fn get_error() -> &'static str {
    ERROR_REASON.with(|e| e.get())
}

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
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59,
    67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

static CP_DIST_EXTRA_BITS: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

static CP_DIST_BASE: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

struct CpState<'a> {
    bits: u64,
    count: i32,
    // raw input buffer
    input: &'a [u8],
    // input layout
    first_bytes: usize,
    word_count: i32,
    word_index: i32,
    bits_left: i32,
    final_word_available: i32,
    final_word: u32,
    // output
    out: &'a mut [u8],
    out_pos: usize,
    out_end: usize,
    begin: usize,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

impl<'a> CpState<'a> {
    fn new(input: &'a [u8], first_bytes: usize, word_count: i32, output: &'a mut [u8]) -> Self {
        let out_end = output.len();
        Self {
            bits: 0,
            count: 0,
            input,
            first_bytes,
            word_count,
            word_index: 0,
            bits_left: 0,
            final_word_available: 0,
            final_word: 0,
            out: output,
            out_pos: 0,
            out_end,
            begin: 0,
            lookup: [0u16; 1 << 9],
            lit: [0u32; 288],
            dst: [0u32; 32],
            len: [0u32; 19],
            nlit: 0,
            ndst: 0,
            nlen: 0,
        }
    }

    /// Read the i'th 32-bit word from the aligned region.
    fn get_word(&self, idx: i32) -> u32 {
        let start = self.first_bytes + (idx as usize) * 4;
        let b0 = self.input[start] as u32;
        let b1 = self.input[start + 1] as u32;
        let b2 = self.input[start + 2] as u32;
        let b3 = self.input[start + 3] as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    /// Returns the absolute position (in input bytes) of the next byte to read
    /// in the bitstream. This corresponds to the C code's `cp_ptr`.
    fn ptr_offset(&self) -> usize {
        // assert(!(s->bits_left & 7));
        debug_assert!(self.bits_left & 7 == 0);
        // (char *)(s->words + s->word_index) - (s->count / 8)
        let word_byte_offset = self.first_bytes + (self.word_index as usize) * 4;
        word_byte_offset - (self.count as usize / 8)
    }
}

fn cp_would_overflow(s: &CpState, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

fn cp_peak_bits(s: &mut CpState, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = s.get_word(s.word_index);
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
        (1u64 << num_bits_to_read).wrapping_sub(1)
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

/// Build the canonical Huffman tree. If `update_lookup` is true, also fill `s.lookup`.
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
        // Will clear lookup below; need mutable. We'll handle below.
        let _ = s;
    }

    if let Some(s_ref) = s {
        for v in s_ref.lookup.iter_mut() {
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
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        s_ref.lookup[j] = ((len << 9) as u16) | (i as u16);
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

fn cp_stored(s: &mut CpState) -> Result<(), ()> {
    let bits_to_skip = s.count & 7;
    cp_read_bits(s, bits_to_skip);
    let len_val = cp_read_bits(s, 16) as u16;
    let nlen_val = cp_read_bits(s, 16) as u16;

    if !(len_val == !nlen_val) {
        set_error(
            "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
        );
        return Err(());
    }
    if !(s.bits_left / 8 <= len_val as i32) {
        set_error("Stored block extends beyond end of input stream.");
        return Err(());
    }
    let p = s.ptr_offset();
    let len_usize = len_val as usize;
    // memcpy(s->out, p, LEN)
    // copy from input[p..p+len] to s.out[s.out_pos..]
    // Note: this is a copy from input buffer to output buffer
    s.out[s.out_pos..s.out_pos + len_usize]
        .copy_from_slice(&s.input[p..p + len_usize]);
    s.out_pos += len_usize;
    Ok(())
}

fn cp_fixed(s: &mut CpState) {
    // s->nlit = cp_build(s, s->lit, cp_fixed_table, 288);
    let mut lit = s.lit;
    let nlit = cp_build(Some(s), &mut lit, &CP_FIXED_TABLE[0..288], 288);
    s.lit = lit;
    s.nlit = nlit as u32;
    // s->ndst = cp_build(0, s->dst, cp_fixed_table + 288, 32);
    let mut dst = s.dst;
    let ndst = cp_build(None, &mut dst, &CP_FIXED_TABLE[288..288 + 32], 32);
    s.dst = dst;
    s.ndst = ndst as u32;
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

fn cp_dynamic(s: &mut CpState) {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen as usize {
        let val = cp_read_bits(s, 3) as u8;
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = val;
    }
    let mut len_arr = s.len;
    s.nlen = cp_build(None, &mut len_arr, &lenlens, 19) as u32;
    s.len = len_arr;

    let mut lens = [0u8; 288 + 32];
    let total = nlit + ndst;
    let mut n = 0i32;
    while n < total {
        let len_arr = s.len;
        let nlen_local = s.nlen as i32;
        let sym = cp_decode(s, &len_arr, nlen_local);
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
    let mut lit = s.lit;
    s.nlit = cp_build(Some(s), &mut lit, &lens[0..nlit as usize], nlit as usize) as u32;
    s.lit = lit;
    let mut dst = s.dst;
    s.ndst = cp_build(None, &mut dst, &lens[nlit as usize..(nlit + ndst) as usize], ndst as usize) as u32;
    s.dst = dst;
}

fn cp_block(s: &mut CpState) -> Result<(), ()> {
    loop {
        let lit = s.lit;
        let nlit = s.nlit as i32;
        let symbol = cp_decode(s, &lit, nlit);
        if symbol < 256 {
            if !(s.out_pos + 1 <= s.out_end) {
                set_error("Attempted to overwrite out buffer while outputting a symbol.");
                return Err(());
            }
            s.out[s.out_pos] = symbol as u8;
            s.out_pos += 1;
        } else if symbol > 256 {
            let symbol = (symbol - 257) as usize;
            let length = (cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol] as i32)
                + CP_LEN_BASE[symbol]) as i32;
            let dst = s.dst;
            let ndst = s.ndst as i32;
            let distance_symbol = cp_decode(s, &dst, ndst) as usize;
            let backwards_distance = (cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol] as i32)
                + CP_DIST_BASE[distance_symbol]) as i32;
            // s->out - backwards_distance >= s->begin
            if !(s.out_pos as i64 - backwards_distance as i64 >= s.begin as i64) {
                set_error(
                    "Attempted to write before out buffer (invalid backwards distance).",
                );
                return Err(());
            }
            if !(s.out_pos + length as usize <= s.out_end) {
                set_error("Attempted to overwrite out buffer while outputting a string.");
                return Err(());
            }
            let src_idx = s.out_pos - backwards_distance as usize;
            let dst_idx = s.out_pos;
            s.out_pos += length as usize;
            if backwards_distance == 1 {
                let v = s.out[src_idx];
                for i in 0..length as usize {
                    s.out[dst_idx + i] = v;
                }
            } else {
                // while (length--) *dst++ = *src++;
                // This is byte-by-byte and may overlap forward (allows RLE-like)
                for i in 0..length as usize {
                    s.out[dst_idx + i] = s.out[src_idx + i];
                }
            }
        } else {
            break;
        }
    }
    Ok(())
}

pub fn pinflate(input: &[u8], output: &mut [u8]) -> Result<usize, &'static str> {
    let in_bytes = input.len() as i32;

    // int first_bytes = (int)((((size_t)in + 3) & ~3) - (size_t)in);
    // In C, `in` is the actual pointer. In Rust, we don't have an actual aligned-pointer
    // semantics; we treat `first_bytes = 0` (input begins at "aligned" position 0).
    // This deviates from raw pointer arithmetic but is the only consistent way to
    // process Rust slices. The behavior is equivalent when `in` happens to be 4-byte
    // aligned (which is the case for typical heap allocations and for a Vec<u8>).
    let first_bytes: usize = 0;
    let word_count = (in_bytes - first_bytes as i32) / 4;
    let last_bytes = (in_bytes - first_bytes as i32) & 3;

    let out_len = output.len();
    let mut s = CpState::new(input, first_bytes, word_count, output);
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;

    for i in 0..first_bytes {
        s.bits |= (input[i] as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes as usize {
        s.final_word |= (input[in_bytes as usize - last_bytes as usize + i] as u32) << (i * 8);
    }
    s.count = (first_bytes as i32) * 8;
    s.out_pos = 0;
    s.out_end = out_len;
    s.begin = 0;

    let mut _count = 0i32;
    loop {
        let bfinal = cp_read_bits(&mut s, 1);
        let btype = cp_read_bits(&mut s, 2);
        match btype {
            0 => {
                if cp_stored(&mut s).is_err() {
                    return Err(get_error());
                }
            }
            1 => {
                cp_fixed(&mut s);
                if cp_block(&mut s).is_err() {
                    return Err(get_error());
                }
            }
            2 => {
                cp_dynamic(&mut s);
                if cp_block(&mut s).is_err() {
                    return Err(get_error());
                }
            }
            3 => {
                set_error("Detected unknown block type within input stream.");
                return Err(get_error());
            }
            _ => unreachable!(),
        }
        _count += 1;
        if bfinal != 0 {
            break;
        }
    }

    let written = s.out_pos;
    Ok(written)
}
