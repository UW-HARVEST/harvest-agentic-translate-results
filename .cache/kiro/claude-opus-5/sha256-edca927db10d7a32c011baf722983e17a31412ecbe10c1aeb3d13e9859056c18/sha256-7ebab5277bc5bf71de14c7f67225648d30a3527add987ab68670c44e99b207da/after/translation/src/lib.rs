//! Rust translation of `c_src/src/lib.c` (a stripped-down cute_png DEFLATE
//! inflater exposing a single entry point, `pinflate`).
//!
//! The translation is deliberately literal: the state struct keeps the same
//! `#[repr(C)]` layout as the C original, pointer arithmetic is reproduced
//! with raw pointers (including the places where the C code reads or writes
//! out of bounds), the `assert()` calls are kept (the CMake project builds
//! without `NDEBUG`, so they are live), and the original checks are performed
//! in the original order -- bugs included.
//!
//! One place needs more than a literal transcription. `cp_dynamic` writes code
//! lengths into `uint8_t lens[288 + 32]` from loops that only re-test their
//! bound *between* run-length groups, so a malformed stream can push the index
//! up to 137 entries past the array and overwrite the rest of the stack frame --
//! including `nlit`, `ndst`, the loop counters and the index itself, all of
//! which are re-read afterwards. To stay observably identical there,
//! [`CpDynamicFrame`] reproduces the frame layout the C compiler emits and the
//! writes go through a raw pointer, so the same fields get clobbered in the same
//! order. See that type's documentation for the offsets and how they were
//! obtained.

use std::ffi::{c_char, c_int, c_void};
use std::ptr::{self, addr_of, addr_of_mut};

// ---------------------------------------------------------------------------
// Globals with external linkage in the C translation unit.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

/// The C original spells out 144 `8`s, 112 `9`s, 24 `7`s, 8 `8`s and 32 `5`s.
const fn cp_make_fixed_table() -> [u8; 288 + 32] {
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
    while i < 320 {
        t[i] = 5;
        i += 1;
    }
    t
}

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = cp_make_fixed_table();

#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

/// Helper mirroring `cp_error_reason = "..."`.
#[inline]
unsafe fn set_error(msg: &'static [u8]) {
    cp_error_reason = msg.as_ptr() as *const c_char;
}

// ---------------------------------------------------------------------------
// Decoder state (layout-compatible with the C `cp_state_t`).
// ---------------------------------------------------------------------------

#[repr(C)]
struct CpState {
    bits: u64,
    count: c_int,
    words: *mut u32,
    word_count: c_int,
    word_index: c_int,
    bits_left: c_int,
    final_word_available: c_int,
    final_word: u32,
    out: *mut c_char,
    out_end: *mut c_char,
    begin: *mut c_char,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

// ---------------------------------------------------------------------------
// Bit reader
// ---------------------------------------------------------------------------

unsafe fn cp_would_overflow(s: *mut CpState, num_bits: c_int) -> c_int {
    (((*s).bits_left.wrapping_add((*s).count)).wrapping_sub(num_bits) < 0) as c_int
}

unsafe fn cp_ptr(s: *mut CpState) -> *mut c_char {
    assert!((*s).bits_left & 7 == 0);
    ((*s).words.offset((*s).word_index as isize) as *mut c_char)
        .offset(-(((*s).count / 8) as isize))
}

unsafe fn cp_peak_bits(s: *mut CpState, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.offset((*s).word_index as isize);
            (*s).word_index += 1;
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count += 32;
            assert!((*s).word_index <= (*s).word_count);
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64).wrapping_shl((*s).count as u32);
            (*s).count += (*s).bits_left;
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

unsafe fn cp_consume_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!((*s).count >= num_bits_to_read);
    let mask = (1u64.wrapping_shl(num_bits_to_read as u32)).wrapping_sub(1);
    let bits = ((*s).bits & mask) as u32;
    (*s).bits = (*s).bits.wrapping_shr(num_bits_to_read as u32);
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!((*s).bits_left > 0);
    assert!((*s).count <= 64);
    assert!(cp_would_overflow(s, num_bits_to_read) == 0);
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

// ---------------------------------------------------------------------------
// Huffman table construction / decoding
// ---------------------------------------------------------------------------

/// `s` may be null (matching the C code's `cp_build(0, ...)` calls).
///
/// `counts` is oversized relative to the C original (`int counts[16]`) so that
/// a malformed stream producing a code length >= 16 does not panic before the
/// C code's `assert(len < 16)` has a chance to fire.
unsafe fn cp_build(s: *mut CpState, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 256];

    let mut n = 0;
    while n < sym_count {
        counts[*lens.offset(n as isize) as usize] += 1;
        n += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }

    if !s.is_null() {
        ptr::write_bytes(addr_of_mut!((*s).lookup) as *mut u8, 0, 2 * (1 << 9));
    }

    let mut i = 0;
    while i < sym_count {
        let len = *lens.offset(i as isize) as usize;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as u32;
            first[len] += 1;
            *tree.offset(slot as isize) =
                (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - len)) as i32;
                while j < (1 << 9) {
                    (*s).lookup[j as usize] = ((len << 9) | (i as usize)) as u16;
                    j += 1 << len;
                }
            }
        }
        i += 1;
    }

    first[15]
}

unsafe fn cp_decode(s: *mut CpState, tree: *mut u32, hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0i32;
    let mut hi = hi;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.offset(guess as isize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.offset((lo - 1) as isize);
    let len = 32u32.wrapping_sub(key & 0xF);
    assert!(search.wrapping_shr(len) == key.wrapping_shr(len));
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

// ---------------------------------------------------------------------------
// Block decoders
// ---------------------------------------------------------------------------

unsafe fn cp_stored(s: *mut CpState) -> c_int {
    // 3.2.3: skip any remaining bits in the current partially processed byte.
    cp_read_bits(s, (*s).count & 7);
    // 3.2.4: read LEN and NLEN, which should complement each other.
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if !(len == !nlen) {
        set_error(
            b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0",
        );
        return 0;
    }
    if !((*s).bits_left / 8 <= len as c_int) {
        set_error(b"Stored block extends beyond end of input stream.\0");
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p as *const u8, (*s).out as *mut u8, len as usize);
    (*s).out = (*s).out.offset(len as isize);
    1
}

unsafe fn cp_fixed(s: *mut CpState) -> c_int {
    (*s).nlit = cp_build(
        s,
        addr_of_mut!((*s).lit) as *mut u32,
        addr_of!(cp_fixed_table) as *const u8,
        288,
    ) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        addr_of_mut!((*s).dst) as *mut u32,
        (addr_of!(cp_fixed_table) as *const u8).offset(288),
        32,
    ) as u32;
    1
}

/// Byte-exact stand-in for the stack frame GCC gives `cp_dynamic` at `-O0`.
///
/// The C original writes its code lengths into `uint8_t lens[288 + 32]` from
/// loops whose only bound is `n < nlit + ndst`, re-tested *between* run-length
/// groups. A single symbol 18 can add 138 entries, so `n` reaches
/// `nlit + ndst - 1 + 138` -- up to 457 -- and the tail of that run lands on
/// whatever follows the array in the frame.
///
/// Field offsets below are the ones the compiled object actually uses
/// (`objdump -d` on `cp_dynamic`; all relative to `%rbp`):
///
/// ```text
///   -0x188  s            (parameter spill slot)
///   -0x180  lens[320]
///   -0x040  lenlens[19]
///   -0x02d  9 bytes of padding, never initialised
///   -0x024  sym
///   -0x020  nlen
///   -0x01c  ndst
///   -0x018  nlit
///   -0x014  i    (symbol 18 loop)
///   -0x010  i    (symbol 17 loop)
///   -0x00c  i    (symbol 16 loop)
///   -0x008  n
///   -0x004  i    (code-length permutation loop)
///    0x000  saved %rbp
///    0x008  return address
/// ```
///
/// Reproducing the layout is what makes the overflow observable in the same way
/// it is in C: `lens[356..]` overwrites `ndst`, `lens[360..]` overwrites `nlit`,
/// and further out the loop counters and `n` itself. Because the loop condition
/// and the two closing `cp_build` calls re-read `nlit`/`ndst` from the frame,
/// the corrupted values feed straight back into the decode.
#[repr(C)]
struct CpDynamicFrame {
    /// Slack below the frame, for the (already undefined) case where `n` is
    /// corrupted to a negative value.
    below: [u8; 4096],
    /// The `s` parameter's spill slot. `lens[-1]`, which the C code reads when
    /// symbol 16 arrives with `n == 0`, is this pointer's most significant byte.
    s_slot: *mut CpState,
    lens: [u8; 288 + 32],
    lenlens: [u8; 19],
    _gap: [u8; 9],
    sym: c_int,
    nlen: c_int,
    ndst: c_int,
    nlit: c_int,
    i18: c_int,
    i17: c_int,
    i16: c_int,
    n: c_int,
    i_perm: c_int,
    saved_rbp: u64,
    ret_addr: u64,
    /// Slack above the frame. In C these bytes belong to the callers' frames,
    /// so a write this far out redirects control flow on return and has no
    /// representable equivalent; the slack at least keeps the writes inside
    /// memory this crate owns.
    beyond: [u8; 4096],
}

unsafe fn cp_dynamic(s: *mut CpState) -> c_int {
    // The offsets above are load-bearing, so pin them down at compile time.
    const _: () = {
        let lens = std::mem::offset_of!(CpDynamicFrame, lens);
        assert!(std::mem::offset_of!(CpDynamicFrame, s_slot) + 8 == lens);
        assert!(std::mem::offset_of!(CpDynamicFrame, lenlens) - lens == 0x140);
        assert!(std::mem::offset_of!(CpDynamicFrame, sym) - lens == 0x15c);
        assert!(std::mem::offset_of!(CpDynamicFrame, nlen) - lens == 0x160);
        assert!(std::mem::offset_of!(CpDynamicFrame, ndst) - lens == 0x164);
        assert!(std::mem::offset_of!(CpDynamicFrame, nlit) - lens == 0x168);
        assert!(std::mem::offset_of!(CpDynamicFrame, i18) - lens == 0x16c);
        assert!(std::mem::offset_of!(CpDynamicFrame, i17) - lens == 0x170);
        assert!(std::mem::offset_of!(CpDynamicFrame, i16) - lens == 0x174);
        assert!(std::mem::offset_of!(CpDynamicFrame, n) - lens == 0x178);
        assert!(std::mem::offset_of!(CpDynamicFrame, i_perm) - lens == 0x17c);
        assert!(std::mem::offset_of!(CpDynamicFrame, saved_rbp) - lens == 0x180);
        assert!(std::mem::offset_of!(CpDynamicFrame, ret_addr) - lens == 0x188);
    };

    let mut frame: Box<CpDynamicFrame> = Box::new(std::mem::zeroed());
    let f: *mut CpDynamicFrame = &mut *frame;
    (*f).s_slot = s;
    let lens: *mut u8 = addr_of_mut!((*f).lens) as *mut u8;
    let lenlens: *mut u8 = addr_of_mut!((*f).lenlens) as *mut u8;

    (*f).nlit = 257i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    (*f).ndst = 1i32.wrapping_add(cp_read_bits(s, 5) as c_int);
    (*f).nlen = 4i32.wrapping_add(cp_read_bits(s, 4) as c_int);

    (*f).i_perm = 0;
    while (*f).i_perm < (*f).nlen {
        let bits = cp_read_bits(s, 3) as u8;
        let idx =
            *(addr_of!(cp_permutation_order) as *const u8).offset((*f).i_perm as isize) as isize;
        *lenlens.offset(idx) = bits;
        (*f).i_perm = (*f).i_perm.wrapping_add(1);
    }
    (*s).nlen = cp_build(
        ptr::null_mut(),
        addr_of_mut!((*s).len) as *mut u32,
        lenlens as *const u8,
        19,
    ) as u32;

    (*f).n = 0;
    while (*f).n < (*f).nlit.wrapping_add((*f).ndst) {
        (*f).sym = cp_decode(s, addr_of_mut!((*s).len) as *mut u32, (*s).nlen as c_int);
        match (*f).sym {
            16 => {
                (*f).i16 = 3i32.wrapping_add(cp_read_bits(s, 2) as i32);
                while (*f).i16 != 0 {
                    let prev = *lens.offset(((*f).n as isize).wrapping_sub(1));
                    *lens.offset((*f).n as isize) = prev;
                    (*f).i16 = (*f).i16.wrapping_sub(1);
                    (*f).n = (*f).n.wrapping_add(1);
                }
            }
            17 => {
                (*f).i17 = 3i32.wrapping_add(cp_read_bits(s, 3) as i32);
                while (*f).i17 != 0 {
                    *lens.offset((*f).n as isize) = 0;
                    (*f).i17 = (*f).i17.wrapping_sub(1);
                    (*f).n = (*f).n.wrapping_add(1);
                }
            }
            18 => {
                (*f).i18 = 11i32.wrapping_add(cp_read_bits(s, 7) as i32);
                while (*f).i18 != 0 {
                    *lens.offset((*f).n as isize) = 0;
                    (*f).i18 = (*f).i18.wrapping_sub(1);
                    (*f).n = (*f).n.wrapping_add(1);
                }
            }
            _ => {
                // `lens[n++] = (uint8_t)sym`: the post-increment lands in the
                // frame before the store, which matters once the store is the
                // thing overwriting `n`.
                let at = (*f).n;
                (*f).n = at.wrapping_add(1);
                *lens.offset(at as isize) = (*f).sym as u8;
            }
        }
    }

    (*s).nlit = cp_build(
        s,
        addr_of_mut!((*s).lit) as *mut u32,
        lens as *const u8,
        (*f).nlit,
    ) as u32;
    (*s).ndst = cp_build(
        ptr::null_mut(),
        addr_of_mut!((*s).dst) as *mut u32,
        (lens as *const u8).offset((*f).nlit as isize),
        (*f).ndst,
    ) as u32;
    1
}

unsafe fn cp_block(s: *mut CpState) -> c_int {
    loop {
        let mut symbol = cp_decode(s, addr_of_mut!((*s).lit) as *mut u32, (*s).nlit as c_int);
        if symbol < 256 {
            if !((*s).out.offset(1) as usize <= (*s).out_end as usize) {
                set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return 0;
            }
            *(*s).out = symbol as c_char;
            (*s).out = (*s).out.offset(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length_extra = cp_read_bits(
                s,
                *(addr_of!(cp_len_extra_bits) as *const u8).offset(symbol as isize) as c_int,
            );
            let length = length_extra
                .wrapping_add(*(addr_of!(cp_len_base) as *const u32).offset(symbol as isize))
                as c_int;

            let distance_symbol =
                cp_decode(s, addr_of_mut!((*s).dst) as *mut u32, (*s).ndst as c_int);
            let distance_extra = cp_read_bits(
                s,
                *(addr_of!(cp_dist_extra_bits) as *const u8).offset(distance_symbol as isize)
                    as c_int,
            );
            let backwards_distance = distance_extra.wrapping_add(
                *(addr_of!(cp_dist_base) as *const u32).offset(distance_symbol as isize),
            ) as c_int;

            if !((*s).out.offset(-(backwards_distance as isize)) as usize
                >= (*s).begin as usize)
            {
                set_error(
                    b"Attempted to write before out buffer (invalid backwards distance).\0",
                );
                return 0;
            }
            if !((*s).out.offset(length as isize) as usize <= (*s).out_end as usize) {
                set_error(b"Attempted to overwrite out buffer while outputting a string.\0");
                return 0;
            }

            let mut src = (*s).out.offset(-(backwards_distance as isize));
            let mut dst = (*s).out;
            (*s).out = (*s).out.offset(length as isize);
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst as *mut u8, *src as u8, length as usize);
                }
                _ => {
                    let mut length = length;
                    while length != 0 {
                        length -= 1;
                        *dst = *src;
                        dst = dst.offset(1);
                        src = src.offset(1);
                    }
                }
            }
        } else {
            break;
        }
    }
    1
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Inflates a raw DEFLATE stream, mirroring
/// `int pinflate(void *in, int in_bytes, void *out, int out_bytes)`.
///
/// Returns 1 on success and 0 on failure, with `cp_error_reason` pointing at a
/// static message.
///
/// # Safety
///
/// `in` must be readable for `in_bytes` and `out` writable for `out_bytes`, and
/// both lengths must be non-negative. Beyond that, the caller inherits the C
/// original's contract: `cp_stored` copies its 16-bit `LEN` field without
/// consulting `out_end`, so a stored block can write up to 65535 bytes past
/// `out`, and a malformed stream can abort the process through one of the live
/// `assert()`s or fail to terminate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pinflate(
    r#in: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    // calloc(1, sizeof(cp_state_t)) / free(s)
    let mut boxed: Box<CpState> = Box::new(std::mem::zeroed());
    let s: *mut CpState = &mut *boxed;

    let in_u8 = r#in as *mut u8;

    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes.wrapping_mul(8);

    let first_bytes = ((((r#in as usize) + 3) & !3usize) - (r#in as usize)) as c_int;
    (*s).words = in_u8.offset(first_bytes as isize) as *mut u32;
    (*s).word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;

    for i in 0..first_bytes {
        (*s).bits |= (*in_u8.offset(i as isize) as u64) << (i * 8);
    }

    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        (*s).final_word |=
            ((*in_u8.offset((in_bytes - last_bytes + i) as isize) as i32) << (i * 8)) as u32;
    }
    (*s).count = first_bytes * 8;

    (*s).out = out as *mut c_char;
    (*s).out_end = (*s).out.offset(out_bytes as isize);
    (*s).begin = out as *mut c_char;

    let mut _count: c_int = 0;
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
                set_error(b"Detected unknown block type within input stream.\0");
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
