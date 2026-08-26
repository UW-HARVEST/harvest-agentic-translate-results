//! Translation of the DEFLATE decompressor part of `lib.c`
//! (`cp_state_t`, `cp_build`, `cp_decode`, `cp_stored`, `cp_fixed`,
//! `cp_dynamic`, `cp_block`, `cp_inflate`).
//!
//! The C code performs a number of deliberately-preserved out-of-bounds
//! accesses (most notably `tree[lo - 1]` in `cp_decode` when `lo == 0`), so the
//! state structure is laid out with `#[repr(C)]` exactly like the C struct and
//! all Huffman-tree accesses go through raw pointers derived from the base of
//! the whole structure. That way an access such as `s->lit[-1]` reads the very
//! same bytes it reads in C (the tail of `s->lookup`).

use core::ffi::{c_char, c_int, c_void};
use core::mem::offset_of;

use crate::tables;

/// ```c
/// typedef struct cp_state_t {
///   uint64_t bits;
///   int count;
///   uint32_t *words;
///   int word_count;
///   int word_index;
///   int bits_left;
///   int final_word_available;
///   uint32_t final_word;
///   char *out;
///   char *out_end;
///   char *begin;
///   uint16_t lookup[(1 << 9)];
///   uint32_t lit[288];
///   uint32_t dst[32];
///   uint32_t len[19];
///   uint32_t nlit;
///   uint32_t ndst;
///   uint32_t nlen;
/// } cp_state_t;
/// ```
#[repr(C)]
pub struct CpState {
    pub bits: u64,
    pub count: c_int,
    pub words: *mut u32,
    pub word_count: c_int,
    pub word_index: c_int,
    pub bits_left: c_int,
    pub final_word_available: c_int,
    pub final_word: u32,
    pub out: *mut c_char,
    pub out_end: *mut c_char,
    pub begin: *mut c_char,
    pub lookup: [u16; 1 << 9],
    pub lit: [u32; 288],
    pub dst: [u32; 32],
    pub len: [u32; 19],
    pub nlit: u32,
    pub ndst: u32,
    pub nlen: u32,
}

impl CpState {
    /// Equivalent of `calloc(1, sizeof(cp_state_t))`.
    const fn zeroed() -> CpState {
        CpState {
            bits: 0,
            count: 0,
            words: core::ptr::null_mut(),
            word_count: 0,
            word_index: 0,
            bits_left: 0,
            final_word_available: 0,
            final_word: 0,
            out: core::ptr::null_mut(),
            out_end: core::ptr::null_mut(),
            begin: core::ptr::null_mut(),
            lookup: [0; 1 << 9],
            lit: [0; 288],
            dst: [0; 32],
            len: [0; 19],
            nlit: 0,
            ndst: 0,
            nlen: 0,
        }
    }
}

/// Pointer to a `uint32_t` array member of the state, keeping the provenance of
/// the whole structure so that the negative indexing done by `cp_decode`
/// behaves exactly like it does in C.
#[inline]
unsafe fn state_u32_field(s: *mut CpState, byte_offset: usize) -> *mut u32 {
    (s as *mut u8).add(byte_offset) as *mut u32
}

#[inline]
unsafe fn lit_ptr(s: *mut CpState) -> *mut u32 {
    state_u32_field(s, offset_of!(CpState, lit))
}

#[inline]
unsafe fn dst_ptr(s: *mut CpState) -> *mut u32 {
    state_u32_field(s, offset_of!(CpState, dst))
}

#[inline]
unsafe fn len_ptr(s: *mut CpState) -> *mut u32 {
    state_u32_field(s, offset_of!(CpState, len))
}

/// ```c
/// static int cp_would_overflow(cp_state_t *s, int num_bits) {
///   return (s->bits_left + s->count) - num_bits < 0;
/// }
/// ```
unsafe fn cp_would_overflow(s: *mut CpState, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

/// ```c
/// static char *cp_ptr(cp_state_t *s) {
///   assert(!(s->bits_left & 7));
///   return (char *)(s->words + s->word_index) - (s->count / 8);
/// }
/// ```
unsafe fn cp_ptr(s: *mut CpState) -> *mut c_char {
    c_assert!(((*s).bits_left & 7) == 0, "!(s->bits_left & 7)", "cp_ptr", 95);
    let base = ((*s).words as *mut u8).wrapping_offset(((*s).word_index as isize).wrapping_mul(4));
    base.wrapping_offset(-((*s).count as isize / 8)) as *mut c_char
}

/// ```c
/// static uint64_t cp_peak_bits(cp_state_t *s, int num_bits_to_read) { ... }
/// ```
unsafe fn cp_peak_bits(s: *mut CpState, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = core::ptr::read_unaligned((*s).words.wrapping_offset((*s).word_index as isize));
            (*s).word_index = (*s).word_index.wrapping_add(1);
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count = (*s).count.wrapping_add(32);
            c_assert!(
                (*s).word_index <= (*s).word_count,
                "s->word_index <= s->word_count",
                "cp_peak_bits",
                104
            );
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count = (*s).count.wrapping_add((*s).bits_left);
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

/// ```c
/// static uint32_t cp_consume_bits(cp_state_t *s, int num_bits_to_read) { ... }
/// ```
unsafe fn cp_consume_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    c_assert!(
        (*s).count >= num_bits_to_read,
        "s->count >= num_bits_to_read",
        "cp_consume_bits",
        115
    );
    let bits = ((*s).bits & (1u64.wrapping_shl(num_bits_to_read as u32).wrapping_sub(1))) as u32;
    (*s).bits >>= num_bits_to_read as u32 & 63;
    (*s).count = (*s).count.wrapping_sub(num_bits_to_read);
    (*s).bits_left = (*s).bits_left.wrapping_sub(num_bits_to_read);
    bits
}

/// ```c
/// static uint32_t cp_read_bits(cp_state_t *s, int num_bits_to_read) { ... }
/// ```
unsafe fn cp_read_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    c_assert!(
        num_bits_to_read <= 32,
        "num_bits_to_read <= 32",
        "cp_read_bits",
        123
    );
    c_assert!(
        num_bits_to_read >= 0,
        "num_bits_to_read >= 0",
        "cp_read_bits",
        124
    );
    c_assert!((*s).bits_left > 0, "s->bits_left > 0", "cp_read_bits", 125);
    c_assert!((*s).count <= 64, "s->count <= 64", "cp_read_bits", 126);
    c_assert!(
        cp_would_overflow(s, num_bits_to_read) == 0,
        "!cp_would_overflow(s, num_bits_to_read)",
        "cp_read_bits",
        127
    );
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

/// ```c
/// static uint32_t cp_rev16(uint32_t a) { ... }
/// ```
fn cp_rev16(a: u32) -> u32 {
    let a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    let a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    let a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8)
}

/// ```c
/// static int cp_build(cp_state_t *s, uint32_t *tree, uint8_t *lens, int sym_count) { ... }
/// ```
///
/// `counts` is indexed with `lens[n]`, which the C code does not validate; the
/// array is oversized here so that such an access cannot trap. Note that any
/// length `>= 16` makes the `assert(len < 16)` below abort anyway.
unsafe fn cp_build(
    s: *mut CpState,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 256];

    let mut n: c_int = 0;
    while n < sym_count {
        let l = *lens.wrapping_offset(n as isize) as usize;
        counts[l] = counts[l].wrapping_add(1);
        n += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    n = 1;
    while n <= 15 {
        let i = n as usize;
        codes[i] = (codes[i - 1].wrapping_add(counts[i - 1])) << 1;
        first[i] = first[i - 1].wrapping_add(counts[i - 1]);
        n += 1;
    }
    if !s.is_null() {
        core::ptr::write_bytes((*s).lookup.as_mut_ptr() as *mut u8, 0, 2 * (1 << 9));
    }
    let mut i: c_int = 0;
    while i < sym_count {
        let len = *lens.wrapping_offset(i as isize) as c_int;
        if len != 0 {
            c_assert!(len < 16, "len < 16", "cp_build", 154);
            let code = codes[len as usize] as u32;
            codes[len as usize] = codes[len as usize].wrapping_add(1);
            let slot = first[len as usize] as u32;
            first[len as usize] = first[len as usize].wrapping_add(1);
            *tree.wrapping_offset(slot as i32 as isize) = (code.wrapping_shl((32 - len) as u32))
                | ((i as u32).wrapping_shl(4))
                | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j: c_int = (cp_rev16(code) >> (16 - len)) as c_int;
                while j < (1 << 9) {
                    (*s).lookup[j as usize] = (((len as u32) << 9) | (i as u32)) as u16;
                    j += 1 << len;
                }
            }
        }
        i += 1;
    }
    first[15]
}

/// ```c
/// static int cp_stored(cp_state_t *s) { ... }
/// ```
unsafe fn cp_stored(s: *mut CpState) -> c_int {
    cp_read_bits(s, (*s).count & 7);
    let len_field = cp_read_bits(s, 16) as u16;
    let nlen_field = cp_read_bits(s, 16) as u16;
    if !(len_field == !nlen_field) {
        tables::set_error_reason(tables::ERR_LEN_NLEN);
        return 0;
    }
    if !((*s).bits_left / 8 <= len_field as c_int) {
        tables::set_error_reason(tables::ERR_STORED_BEYOND);
        return 0;
    }
    let p = cp_ptr(s);
    core::ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, len_field as usize);
    (*s).out = (*s).out.wrapping_offset(len_field as isize);
    1
}

/// ```c
/// static int cp_fixed(cp_state_t *s) {
///   s->nlit = cp_build(s, s->lit, cp_fixed_table, 288);
///   s->ndst = cp_build(0, s->dst, cp_fixed_table + 288, 32);
///   return 1;
/// }
/// ```
unsafe fn cp_fixed(s: *mut CpState) -> c_int {
    let table = core::ptr::addr_of_mut!(tables::cp_fixed_table) as *const u8;
    (*s).nlit = cp_build(s, lit_ptr(s), table, 288) as u32;
    (*s).ndst = cp_build(
        core::ptr::null_mut(),
        dst_ptr(s),
        table.wrapping_add(288),
        32,
    ) as u32;
    1
}

/// ```c
/// static int cp_decode(cp_state_t *s, uint32_t *tree, int hi) { ... }
/// ```
unsafe fn cp_decode(s: *mut CpState, tree: *mut u32, hi_in: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo: c_int = 0;
    let mut hi = hi_in;
    while lo < hi {
        let guess = (lo.wrapping_add(hi)) >> 1;
        if search < *tree.wrapping_offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess.wrapping_add(1);
        }
    }
    let key = *tree.wrapping_offset(lo.wrapping_sub(1) as isize);
    let len = 32u32.wrapping_sub(key & 0xF);
    // `search >> len` / `key >> len` are shifts by 32 when `key & 0xF == 0`,
    // which the unoptimised C build performs as a shift by `len & 31`.
    c_assert!(
        search.wrapping_shr(len) == key.wrapping_shr(len),
        "(search >> len) == (key >> len)",
        "cp_decode",
        217
    );
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

/// ```c
/// static int cp_dynamic(cp_state_t *s) { ... }
/// ```
///
/// The C function keeps `uint8_t lens[288 + 32]` on the stack, reads `lens[-1]`
/// when the very first symbol is 16 and may run past the end of the array for
/// malformed streams. A slightly larger buffer with one leading slack byte is
/// used so that those accesses stay inside allocated memory.
unsafe fn cp_dynamic(s: *mut CpState) -> c_int {
    let mut lenlens = [0u8; 19];
    let nlit: c_int = 257i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    let ndst: c_int = 1i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    let nlen: c_int = 4i32.wrapping_add(cp_read_bits(s, 4) as c_int);
    let order = core::ptr::addr_of_mut!(tables::cp_permutation_order) as *const u8;
    let mut i: c_int = 0;
    while i < nlen {
        let idx = *order.wrapping_offset(i as isize) as usize;
        lenlens[idx] = cp_read_bits(s, 3) as u8;
        i += 1;
    }
    (*s).nlen = cp_build(
        core::ptr::null_mut(),
        len_ptr(s),
        lenlens.as_ptr(),
        19,
    ) as u32;

    let mut lens_storage = [0u8; 1 + (288 + 32) + 160];
    let lens = lens_storage.as_mut_ptr().add(1);

    let mut n: c_int = 0;
    while n < nlit.wrapping_add(ndst) {
        let sym = cp_decode(s, len_ptr(s), (*s).nlen as c_int);
        match sym {
            16 => {
                let mut i: c_int = 3i32.wrapping_add(cp_read_bits(s, 2) as c_int);
                while i != 0 {
                    *lens.wrapping_offset(n as isize) =
                        *lens.wrapping_offset(n.wrapping_sub(1) as isize);
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i: c_int = 3i32.wrapping_add(cp_read_bits(s, 3) as c_int);
                while i != 0 {
                    *lens.wrapping_offset(n as isize) = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i: c_int = 11i32.wrapping_add(cp_read_bits(s, 7) as c_int);
                while i != 0 {
                    *lens.wrapping_offset(n as isize) = 0;
                    i -= 1;
                    n += 1;
                }
            }
            _ => {
                *lens.wrapping_offset(n as isize) = sym as u8;
                n += 1;
            }
        }
    }
    (*s).nlit = cp_build(s, lit_ptr(s), lens, nlit) as u32;
    (*s).ndst = cp_build(
        core::ptr::null_mut(),
        dst_ptr(s),
        lens.wrapping_offset(nlit as isize),
        ndst,
    ) as u32;
    1
}

/// ```c
/// static int cp_block(cp_state_t *s) { ... }
/// ```
unsafe fn cp_block(s: *mut CpState) -> c_int {
    let len_extra = core::ptr::addr_of_mut!(tables::cp_len_extra_bits) as *const u8;
    let len_base = core::ptr::addr_of_mut!(tables::cp_len_base) as *const u32;
    let dist_extra = core::ptr::addr_of_mut!(tables::cp_dist_extra_bits) as *const u8;
    let dist_base = core::ptr::addr_of_mut!(tables::cp_dist_base) as *const u32;
    loop {
        let mut symbol = cp_decode(s, lit_ptr(s), (*s).nlit as c_int);
        if symbol < 256 {
            // if (!(s->out + 1 <= s->out_end)) { ... }
            if !(((*s).out as usize).wrapping_add(1) <= (*s).out_end as usize) {
                tables::set_error_reason(tables::ERR_OUT_SYMBOL);
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.wrapping_offset(1);
        } else if symbol > 256 {
            symbol = symbol.wrapping_sub(257);
            let length: c_int = cp_read_bits(s, *len_extra.wrapping_offset(symbol as isize) as c_int)
                .wrapping_add(*len_base.wrapping_offset(symbol as isize))
                as c_int;
            let distance_symbol = cp_decode(s, dst_ptr(s), (*s).ndst as c_int);
            let backwards_distance: c_int =
                cp_read_bits(s, *dist_extra.wrapping_offset(distance_symbol as isize) as c_int)
                    .wrapping_add(*dist_base.wrapping_offset(distance_symbol as isize))
                    as c_int;
            if !(((*s).out as usize).wrapping_sub(backwards_distance as isize as usize)
                >= (*s).begin as usize)
            {
                tables::set_error_reason(tables::ERR_BACKWARDS);
                return 0;
            }
            if !(((*s).out as usize).wrapping_add(length as isize as usize)
                <= (*s).out_end as usize)
            {
                tables::set_error_reason(tables::ERR_OUT_STRING);
                return 0;
            }
            let mut src = (*s).out.wrapping_offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.wrapping_offset(length as isize);
            let mut length = length;
            match backwards_distance {
                1 => {
                    // memset(dst, *src, length)
                    let v = *src;
                    core::ptr::write_bytes(dst as *mut u8, v as u8, length as usize);
                }
                _ => {
                    // while (length--) *dst++ = *src++;
                    while length != 0 {
                        length -= 1;
                        *dst = *src;
                        dst = dst.wrapping_offset(1);
                        src = src.wrapping_offset(1);
                    }
                }
            }
        } else {
            break;
        }
    }
    1
}

/// ```c
/// int cp_inflate(void *in, int in_bytes, void *out, int out_bytes);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    in_: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut boxed = Box::new(CpState::zeroed());
    let s: *mut CpState = &mut *boxed;

    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);
    let in_addr = in_ as usize;
    let first_bytes: c_int = (((in_addr.wrapping_add(3)) & !3usize).wrapping_sub(in_addr)) as c_int;
    (*s).words = (in_ as *mut u8).wrapping_offset(first_bytes as isize) as *mut u32;
    (*s).word_count = in_bytes.wrapping_sub(first_bytes) / 4;
    let last_bytes: c_int = in_bytes.wrapping_sub(first_bytes) & 3;
    let in_u8 = in_ as *const u8;
    let mut i: c_int = 0;
    while i < first_bytes {
        (*s).bits |= (*in_u8.wrapping_offset(i as isize) as u64).wrapping_shl((i * 8) as u32);
        i += 1;
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    let mut i: c_int = 0;
    while i < last_bytes {
        (*s).final_word |= ((*in_u8
            .wrapping_offset(in_bytes.wrapping_sub(last_bytes).wrapping_add(i) as isize)
            as c_int)
            << (i * 8)) as u32;
        i += 1;
    }
    (*s).count = first_bytes.wrapping_mul(8);
    (*s).out = out as *mut c_char;
    (*s).out_end = (*s).out.wrapping_offset(out_bytes as isize);
    (*s).begin = out as *mut c_char;
    let mut count: c_int = 0;
    let mut bfinal: c_int;
    loop {
        bfinal = cp_read_bits(s, 1) as c_int;
        let btype = cp_read_bits(s, 2) as c_int;
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
                tables::set_error_reason(tables::ERR_UNKNOWN_BLOCK);
                return 0;
            }
            _ => {}
        }
        count = count.wrapping_add(1);
        if bfinal != 0 {
            break;
        }
    }
    let _ = count;
    1
}
