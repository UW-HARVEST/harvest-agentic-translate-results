#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(static_mut_refs)]
#![allow(unused_assignments)]

use std::ffi::{c_char, c_int, c_void};

// -----------------------------------------------------------------------------
// Globals matching C externs.
// -----------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = std::ptr::null();

#[unsafe(no_mangle)]
pub static cp_fixed_table: [u8; 288 + 32] = [
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
pub static cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67,
    83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

#[unsafe(no_mangle)]
pub static cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

// -----------------------------------------------------------------------------
// Decoder state.
// -----------------------------------------------------------------------------

struct CpState<'a> {
    bits: u64,
    count: i32,
    input: &'a [u8],
    in_bytes: i32,
    word_index: usize,
    word_count: usize,
    bits_left: i32,
    final_word_available: bool,
    final_word: u32,
    out: &'a mut [u8],
    out_offset: usize,
    out_end: usize,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len_tree: [u32; 19],
    nlit: usize,
    ndst: usize,
    nlen: usize,
}

#[derive(Copy, Clone)]
enum TreeKind {
    Lit,
    Dst,
    Len,
}

// -----------------------------------------------------------------------------
// Helpers.
// -----------------------------------------------------------------------------

unsafe fn set_error(msg: &'static [u8]) {
    // msg must be NUL-terminated.
    cp_error_reason = msg.as_ptr() as *const c_char;
}

fn cp_would_overflow(s: &CpState, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

fn cp_peak_bits(s: &mut CpState, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let off = s.word_index * 4;
            let word = u32::from_le_bytes([
                s.input[off],
                s.input[off + 1],
                s.input[off + 2],
                s.input[off + 3],
            ]);
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
    let n = num_bits_to_read;
    let mask: u64 = if n <= 0 {
        0
    } else if n >= 64 {
        u64::MAX
    } else {
        (1u64 << n) - 1
    };
    let bits = (s.bits & mask) as u32;
    if n >= 64 {
        s.bits = 0;
    } else if n > 0 {
        s.bits >>= n;
    }
    s.count -= n;
    s.bits_left -= n;
    bits
}

fn cp_read_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    let _ = cp_would_overflow; // silence unused warning if any
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

// cp_build constructs a Huffman tree (sorted by canonical code) and optionally
// populates a 9-bit lookup table.
fn cp_build(
    lookup: Option<&mut [u16; 1 << 9]>,
    tree: &mut [u32],
    lens: &[u8],
    sym_count: usize,
) -> usize {
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

    if let Some(lk) = lookup {
        for slot in lk.iter_mut() {
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
                tree[slot] =
                    (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        lk[j] = (((len as usize) << 9) | i) as u16;
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
                tree[slot] =
                    (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            }
        }
    }

    first[15] as usize
}

fn cp_stored(s: &mut CpState) -> i32 {
    let align = s.count & 7;
    cp_read_bits(s, align);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;

    if !(len == !nlen) {
        unsafe {
            set_error(
                b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0",
            );
        }
        return 0;
    }

    if !((s.bits_left / 8) <= len as i32) {
        unsafe {
            set_error(b"Stored block extends beyond end of input stream.\0");
        }
        return 0;
    }

    // cp_ptr(s) returns the input byte position for the bytes still buffered in
    // s.bits plus the bytes not yet loaded. With byte-aligned s.count the
    // position is `in_bytes - bits_left/8`.
    let p = (s.in_bytes - s.bits_left / 8) as usize;
    let len_us = len as usize;
    s.out[s.out_offset..s.out_offset + len_us]
        .copy_from_slice(&s.input[p..p + len_us]);
    s.out_offset += len_us;
    1
}

fn cp_fixed(s: &mut CpState) -> i32 {
    s.nlit = cp_build(
        Some(&mut s.lookup),
        &mut s.lit[..],
        &cp_fixed_table[..288],
        288,
    );
    s.ndst = cp_build(None, &mut s.dst[..], &cp_fixed_table[288..288 + 32], 32);
    1
}

fn cp_decode(s: &mut CpState, tree_kind: TreeKind, hi_in: usize) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search: u32 = (cp_rev16(bits as u32) << 16) | 0xFFFF;

    let mut lo: usize = 0;
    let mut hi: usize = hi_in;

    let key: u32 = {
        let tree_slice: &[u32] = match tree_kind {
            TreeKind::Lit => &s.lit[..],
            TreeKind::Dst => &s.dst[..],
            TreeKind::Len => &s.len_tree[..],
        };
        while lo < hi {
            let guess = (lo + hi) >> 1;
            if search < tree_slice[guess] {
                hi = guess;
            } else {
                lo = guess + 1;
            }
        }
        tree_slice[lo - 1]
    };

    let key_low = (key & 0xF) as i32;
    cp_consume_bits(s, key_low);
    ((key >> 4) & 0xFFF) as i32
}

fn cp_dynamic(s: &mut CpState) -> i32 {
    let mut lenlens = [0u8; 19];
    let nlit = (257 + cp_read_bits(s, 5)) as usize;
    let ndst = (1 + cp_read_bits(s, 5)) as usize;
    let nlen = (4 + cp_read_bits(s, 4)) as usize;
    for i in 0..nlen {
        let bits3 = cp_read_bits(s, 3) as u8;
        let idx = cp_permutation_order[i] as usize;
        lenlens[idx] = bits3;
    }
    s.nlen = cp_build(None, &mut s.len_tree[..], &lenlens[..], 19);

    // The C version declares `lens` uninitialised; we zero it for safety.
    let mut lens = [0u8; 288 + 32];
    let mut n: usize = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, TreeKind::Len, s.nlen);
        match sym {
            16 => {
                let mut i = (3 + cp_read_bits(s, 2)) as i32;
                while i != 0 {
                    lens[n] = lens[n - 1];
                    n += 1;
                    i -= 1;
                }
            }
            17 => {
                let mut i = (3 + cp_read_bits(s, 3)) as i32;
                while i != 0 {
                    lens[n] = 0;
                    n += 1;
                    i -= 1;
                }
            }
            18 => {
                let mut i = (11 + cp_read_bits(s, 7)) as i32;
                while i != 0 {
                    lens[n] = 0;
                    n += 1;
                    i -= 1;
                }
            }
            other => {
                lens[n] = other as u8;
                n += 1;
            }
        }
    }

    s.nlit = cp_build(
        Some(&mut s.lookup),
        &mut s.lit[..],
        &lens[..nlit],
        nlit,
    );
    s.ndst = cp_build(
        None,
        &mut s.dst[..],
        &lens[nlit..nlit + ndst],
        ndst,
    );
    1
}

fn cp_block(s: &mut CpState) -> i32 {
    loop {
        let symbol = cp_decode(s, TreeKind::Lit, s.nlit);
        if symbol < 256 {
            if !(s.out_offset + 1 <= s.out_end) {
                unsafe {
                    set_error(
                        b"Attempted to overwrite out buffer while outputting a symbol.\0",
                    );
                }
                return 0;
            }
            s.out[s.out_offset] = symbol as u8;
            s.out_offset += 1;
        } else if symbol > 256 {
            let symbol_idx = (symbol - 257) as usize;
            let length = (cp_read_bits(s, cp_len_extra_bits[symbol_idx] as i32)
                .wrapping_add(cp_len_base[symbol_idx])) as i32;
            let distance_symbol = cp_decode(s, TreeKind::Dst, s.ndst) as usize;
            let backwards_distance =
                (cp_read_bits(s, cp_dist_extra_bits[distance_symbol] as i32)
                    .wrapping_add(cp_dist_base[distance_symbol])) as i32;

            // Equivalent of: `s->out - backwards_distance >= s->begin`.
            // Use signed comparison to mirror the C semantics.
            if !((s.out_offset as i64) - (backwards_distance as i64) >= 0i64) {
                unsafe {
                    set_error(
                        b"Attempted to write before out buffer (invalid backwards distance).\0",
                    );
                }
                return 0;
            }
            if !((s.out_offset as i64) + (length as i64)
                <= (s.out_end as i64))
            {
                unsafe {
                    set_error(
                        b"Attempted to overwrite out buffer while outputting a string.\0",
                    );
                }
                return 0;
            }

            let length_us = length as usize;
            let bd_us = backwards_distance as usize;
            let src_start = s.out_offset - bd_us;
            let dst_start = s.out_offset;
            s.out_offset += length_us;

            if backwards_distance == 1 {
                let val = s.out[src_start];
                for i in 0..length_us {
                    s.out[dst_start + i] = val;
                }
            } else {
                // Byte-by-byte copy to support overlapping ranges.
                for i in 0..length_us {
                    s.out[dst_start + i] = s.out[src_start + i];
                }
            }
        } else {
            // symbol == 256: end of block.
            break;
        }
    }
    1
}

// -----------------------------------------------------------------------------
// Public entry point.
// -----------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pinflate(
    in_ptr: *mut c_void,
    in_bytes: c_int,
    out_ptr: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    if in_ptr.is_null() || out_ptr.is_null() {
        return 0;
    }
    if in_bytes < 0 || out_bytes < 0 {
        return 0;
    }

    let in_bytes_us = in_bytes as usize;
    let out_bytes_us = out_bytes as usize;

    let input: &[u8] = std::slice::from_raw_parts(in_ptr as *const u8, in_bytes_us);
    let output: &mut [u8] =
        std::slice::from_raw_parts_mut(out_ptr as *mut u8, out_bytes_us);

    pinflate_impl(input, in_bytes, output, out_bytes)
}

fn pinflate_impl(
    input: &[u8],
    in_bytes: c_int,
    output: &mut [u8],
    out_bytes: c_int,
) -> c_int {
    // The C performs a 4-byte alignment dance on the input pointer to do
    // aligned u32 loads. We sidestep that and read u32 values from any byte
    // offset. The bit-stream observed downstream is identical because the
    // C also fills the buffer with the same bytes in the same order.
    let in_bytes_us = in_bytes as usize;
    let word_count: usize = in_bytes_us / 4;
    let last_bytes: usize = in_bytes_us & 3;

    let mut final_word: u32 = 0;
    for i in 0..last_bytes {
        let idx = in_bytes_us - last_bytes + i;
        final_word |= (input[idx] as u32) << (i * 8);
    }

    let mut s = CpState {
        bits: 0,
        count: 0,
        input,
        in_bytes,
        word_index: 0,
        word_count,
        bits_left: (in_bytes as i32).wrapping_mul(8),
        final_word_available: last_bytes != 0,
        final_word,
        out: output,
        out_offset: 0,
        out_end: out_bytes as usize,
        lookup: [0u16; 1 << 9],
        lit: [0u32; 288],
        dst: [0u32; 32],
        len_tree: [0u32; 19],
        nlit: 0,
        ndst: 0,
        nlen: 0,
    };

    let mut count = 0i32;
    loop {
        let bfinal = cp_read_bits(&mut s, 1);
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
            _ => {
                // btype == 3 (or any other unexpected value)
                unsafe {
                    set_error(
                        b"Detected unknown block type within input stream.\0",
                    );
                }
                return 0;
            }
        }
        count = count.wrapping_add(1);
        if bfinal != 0 {
            break;
        }
    }
    let _ = count;
    1
}
