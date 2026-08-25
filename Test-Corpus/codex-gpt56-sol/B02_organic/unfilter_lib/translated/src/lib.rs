#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct CpImage {
    w: c_int,
    h: c_int,
    pix: *mut CpPixel,
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> CpPixel {
    CpPixel { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> CpPixel {
    CpPixel { r, g, b, a: 0xff }
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = make_fixed_table();

const fn make_fixed_table() -> [u8; 288 + 32] {
    let mut table = [0_u8; 288 + 32];
    let mut i = 0;
    while i <= 143 {
        table[i] = 8;
        i += 1;
    }
    while i <= 255 {
        table[i] = 9;
        i += 1;
    }
    while i <= 279 {
        table[i] = 7;
        i += 1;
    }
    while i <= 287 {
        table[i] = 8;
        i += 1;
    }
    while i < 288 + 32 {
        table[i] = 5;
        i += 1;
    }
    table
}

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
    fn zeroed() -> Self {
        Self {
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

fn cp_would_overflow(s: &CpState, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

unsafe fn cp_ptr(s: &CpState) -> *const u8 {
    assert!(s.bits_left & 7 == 0);
    unsafe {
        s.words
            .add(s.word_index as usize)
            .cast::<u8>()
            .sub((s.count / 8) as usize)
    }
}

unsafe fn cp_peek_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { ptr::read(s.words.add(s.word_index as usize)) };
            s.word_index += 1;
            s.bits |= u64::from(word) << s.count;
            s.count += 32;
            assert!(s.word_index <= s.word_count);
        } else if s.final_word_available != 0 {
            s.bits |= u64::from(s.final_word) << s.count;
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(s.count >= num_bits_to_read);
    let bits = s.bits & ((1_u64 << num_bits_to_read) - 1);
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits as u32
}

unsafe fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!(s.bits_left > 0);
    assert!(s.count <= 64);
    assert!(!cp_would_overflow(s, num_bits_to_read));
    unsafe { cp_peek_bits(s, num_bits_to_read) };
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8)
}

unsafe fn cp_build(
    mut state: Option<&mut CpState>,
    tree: *mut u32,
    lens: *const u8,
    sym_count: c_int,
) -> c_int {
    let mut codes = [0_i32; 16];
    let mut first = [0_i32; 16];
    let mut counts = [0_i32; 16];
    for n in 0..sym_count {
        let len = unsafe { *lens.add(n as usize) };
        counts[len as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if let Some(s) = state.as_deref_mut() {
        s.lookup.fill(0);
    }
    for i in 0..sym_count {
        let len = unsafe { *lens.add(i as usize) } as usize;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            unsafe {
                *tree.add(slot) = (code << (32 - len)) | ((i as u32) << 4) | len as u32;
            }
            if let Some(s) = state.as_deref_mut()
                && len <= 9
            {
                let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                while j < (1 << 9) {
                    s.lookup[j] = ((len << 9) | i as usize) as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn set_error(message: &'static [u8]) {
    unsafe {
        cp_error_reason = message.as_ptr().cast::<c_char>();
    }
}

unsafe fn cp_stored(s: &mut CpState) -> bool {
    let count = s.count & 7;
    unsafe { cp_read_bits(s, count) };
    let len = unsafe { cp_read_bits(s, 16) } as u16;
    let nlen = unsafe { cp_read_bits(s, 16) } as u16;
    if len != !nlen {
        unsafe {
            set_error(
                b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0",
            );
        }
        return false;
    }
    if s.bits_left / 8 > c_int::from(len) {
        unsafe {
            set_error(b"Stored block extends beyond end of input stream.\0");
        }
        return false;
    }
    let p = unsafe { cp_ptr(s) };
    unsafe {
        ptr::copy_nonoverlapping(p, s.out, len as usize);
        s.out = s.out.add(len as usize);
    }
    true
}

unsafe fn cp_fixed(s: &mut CpState) -> bool {
    let lit = s.lit.as_mut_ptr();
    let dst = s.dst.as_mut_ptr();
    let fixed = &raw const cp_fixed_table as *const u8;
    s.nlit = unsafe { cp_build(Some(s), lit, fixed, 288) } as u32;
    s.ndst = unsafe { cp_build(None, dst, fixed.add(288), 32) } as u32;
    true
}

unsafe fn cp_decode(s: &mut CpState, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = unsafe { cp_peek_bits(s, 16) };
    let search = (cp_rev16(bits as u32) << 16) | 0xffff;
    let mut lo = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < unsafe { *tree.add(guess as usize) } {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = unsafe { *tree.add((lo - 1) as usize) };
    let len = 32 - (key & 0xf);
    assert!(search >> len == key >> len);
    let _code = cp_consume_bits(s, (key & 0xf) as c_int);
    ((key >> 4) & 0xfff) as c_int
}

unsafe fn cp_dynamic(s: &mut CpState) -> bool {
    let mut lenlens = [0_u8; 19];
    let nlit = 257 + unsafe { cp_read_bits(s, 5) } as c_int;
    let ndst = 1 + unsafe { cp_read_bits(s, 5) } as c_int;
    let nlen = 4 + unsafe { cp_read_bits(s, 4) } as c_int;
    let permutation = &raw const cp_permutation_order as *const u8;
    for i in 0..nlen {
        let index = unsafe { *permutation.add(i as usize) } as usize;
        lenlens[index] = unsafe { cp_read_bits(s, 3) } as u8;
    }
    let len_tree = s.len.as_mut_ptr();
    s.nlen = unsafe { cp_build(None, len_tree, lenlens.as_ptr(), lenlens.len() as c_int) } as u32;
    let mut lens = [0_u8; 288 + 32];
    let mut n = 0;
    while n < nlit + ndst {
        let sym = unsafe { cp_decode(s, len_tree, s.nlen as c_int) };
        match sym {
            16 => {
                let repeat = 3 + unsafe { cp_read_bits(s, 2) } as c_int;
                for _ in 0..repeat {
                    lens[n as usize] = lens[(n - 1) as usize];
                    n += 1;
                }
            }
            17 => {
                let repeat = 3 + unsafe { cp_read_bits(s, 3) } as c_int;
                for _ in 0..repeat {
                    lens[n as usize] = 0;
                    n += 1;
                }
            }
            18 => {
                let repeat = 11 + unsafe { cp_read_bits(s, 7) } as c_int;
                for _ in 0..repeat {
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
    let lit = s.lit.as_mut_ptr();
    let dst = s.dst.as_mut_ptr();
    s.nlit = unsafe { cp_build(Some(s), lit, lens.as_ptr(), nlit) } as u32;
    s.ndst = unsafe { cp_build(None, dst, lens.as_ptr().add(nlit as usize), ndst) } as u32;
    true
}

unsafe fn cp_block(s: &mut CpState) -> bool {
    loop {
        let symbol = unsafe { cp_decode(s, s.lit.as_ptr(), s.nlit as c_int) };
        if symbol < 256 {
            if s.out.wrapping_add(1) > s.out_end {
                unsafe {
                    set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                }
                return false;
            }
            unsafe {
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let symbol = (symbol - 257) as usize;
            let len_extra = unsafe { *(&raw const cp_len_extra_bits as *const u8).add(symbol) };
            let len_base = unsafe { *(&raw const cp_len_base as *const u32).add(symbol) };
            let mut length =
                unsafe { cp_read_bits(s, c_int::from(len_extra)) }.wrapping_add(len_base) as usize;
            let distance_symbol = unsafe { cp_decode(s, s.dst.as_ptr(), s.ndst as c_int) } as usize;
            let dist_extra =
                unsafe { *(&raw const cp_dist_extra_bits as *const u8).add(distance_symbol) };
            let dist_base =
                unsafe { *(&raw const cp_dist_base as *const u32).add(distance_symbol) };
            let backwards_distance = unsafe { cp_read_bits(s, c_int::from(dist_extra)) }
                .wrapping_add(dist_base) as usize;
            if s.out.wrapping_sub(backwards_distance) < s.begin {
                unsafe {
                    set_error(
                        b"Attempted to write before out buffer (invalid backwards distance).\0",
                    );
                }
                return false;
            }
            if s.out.wrapping_add(length) > s.out_end {
                unsafe {
                    set_error(b"Attempted to overwrite out buffer while outputting a string.\0");
                }
                return false;
            }
            let mut src = s.out.wrapping_sub(backwards_distance);
            let mut dst = s.out;
            s.out = s.out.wrapping_add(length);
            if backwards_distance == 1 {
                unsafe {
                    ptr::write_bytes(dst, *src, length);
                }
            } else {
                while length != 0 {
                    unsafe {
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
                    }
                    length -= 1;
                }
            }
        } else {
            break;
        }
    }
    true
}

/// Inflate one raw DEFLATE stream into the caller-provided output buffer.
///
/// This mirrors the C ABI, including its integer return status and global
/// `cp_error_reason` side channel.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    output: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut s = Box::new(CpState::zeroed());
    s.bits_left = in_bytes.wrapping_mul(8);
    let input_address = input as usize;
    let first_bytes = ((input_address.wrapping_add(3) & !3).wrapping_sub(input_address)) as c_int;
    s.words = unsafe { input.cast::<u8>().add(first_bytes as usize).cast::<u32>() };
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        let byte = unsafe { *input.cast::<u8>().add(i as usize) };
        s.bits |= u64::from(byte) << (i * 8);
    }
    s.final_word_available = c_int::from(last_bytes != 0);
    for i in 0..last_bytes {
        let byte = unsafe { *input.cast::<u8>().add((in_bytes - last_bytes + i) as usize) };
        s.final_word |= u32::from(byte) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = output.cast::<u8>();
    s.out_end = s.out.wrapping_add(out_bytes as usize);
    s.begin = s.out;

    loop {
        let bfinal = unsafe { cp_read_bits(&mut s, 1) };
        let btype = unsafe { cp_read_bits(&mut s, 2) };
        let ok = match btype {
            0 => unsafe { cp_stored(&mut s) },
            1 => {
                unsafe { cp_fixed(&mut s) };
                unsafe { cp_block(&mut s) }
            }
            2 => {
                unsafe { cp_dynamic(&mut s) };
                unsafe { cp_block(&mut s) }
            }
            _ => {
                unsafe {
                    set_error(b"Detected unknown block type within input stream.\0");
                }
                false
            }
        };
        if !ok {
            return 0;
        }
        if bfinal != 0 {
            return 1;
        }
    }
}

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = c_int::from(a) + c_int::from(b) - c_int::from(c);
    let pa = (p - c_int::from(a)).abs();
    let pb = (p - c_int::from(b)).abs();
    let pc = (p - c_int::from(c)).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[repr(C)]
struct CpRawPng {
    p: *const u8,
    end: *const u8,
}

unsafe fn cp_make32(s: *const u8) -> u32 {
    unsafe {
        (u32::from(*s) << 24)
            | (u32::from(*s.add(1)) << 16)
            | (u32::from(*s.add(2)) << 8)
            | u32::from(*s.add(3))
    }
}

unsafe fn cp_chunk(png: &mut CpRawPng, chunk: *const c_char, minlen: u32) -> *const u8 {
    let len = unsafe { cp_make32(png.p) };
    let start = png.p;
    if unsafe { libc_memcmp(start.add(4), chunk.cast::<u8>(), 4) } == 0 && len >= minlen {
        let offset = len.wrapping_add(12) as usize;
        if png.p.wrapping_add(offset) <= png.end {
            png.p = png.p.wrapping_add(offset);
            return start.wrapping_add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: &mut CpRawPng, chunk: *const c_char, minlen: u32) -> *const u8 {
    while png.p < png.end {
        let len = unsafe { cp_make32(png.p) };
        let start = png.p;
        png.p = png.p.wrapping_add(len.wrapping_add(12) as usize);
        if unsafe { libc_memcmp(start.add(4), chunk.cast::<u8>(), 4) } == 0
            && len >= minlen
            && png.p <= png.end
        {
            return start.wrapping_add(8);
        }
    }
    ptr::null()
}

unsafe fn libc_memcmp(a: *const u8, b: *const u8, len: usize) -> c_int {
    for i in 0..len {
        let av = unsafe { *a.add(i) };
        let bv = unsafe { *b.add(i) };
        if av != bv {
            return c_int::from(av) - c_int::from(bv);
        }
    }
    0
}

/// Reverse PNG scanline filters in place.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, mut raw: *mut u8) -> c_int {
    let len = w.wrapping_mul(bpp);
    if h > 0 {
        let filter = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter {
            0 | 2 => {}
            1 => {
                for x in bpp..len {
                    let current = unsafe { *raw.add(x as usize) };
                    let left = unsafe { *raw.add((x - bpp) as usize) };
                    unsafe { *raw.add(x as usize) = current.wrapping_add(left) };
                }
            }
            3 => {
                for x in bpp..len {
                    let current = unsafe { *raw.add(x as usize) };
                    let left = unsafe { *raw.add((x - bpp) as usize) };
                    unsafe { *raw.add(x as usize) = current.wrapping_add(left / 2) };
                }
            }
            4 => {
                for x in bpp..len {
                    let current = unsafe { *raw.add(x as usize) };
                    let left = unsafe { *raw.add((x - bpp) as usize) };
                    unsafe {
                        *raw.add(x as usize) = current.wrapping_add(cp_paeth(left, 0, 0));
                    }
                }
            }
            _ => return 0,
        }
    }

    let mut prev = raw;
    raw = raw.wrapping_add(len as usize);
    for _ in 1..h {
        let filter = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter {
            0 => {}
            1 => {
                for x in bpp..len {
                    let current = unsafe { *raw.add(x as usize) };
                    let left = unsafe { *raw.add((x - bpp) as usize) };
                    unsafe { *raw.add(x as usize) = current.wrapping_add(left) };
                }
            }
            2 => {
                for x in 0..len {
                    let current = unsafe { *raw.add(x as usize) };
                    let above = unsafe { *prev.add(x as usize) };
                    unsafe { *raw.add(x as usize) = current.wrapping_add(above) };
                }
            }
            3 => {
                for x in 0..bpp {
                    let current = unsafe { *raw.add(x as usize) };
                    let above = unsafe { *prev.add(x as usize) };
                    unsafe { *raw.add(x as usize) = current.wrapping_add(above / 2) };
                }
                for x in bpp..len {
                    let current = unsafe { *raw.add(x as usize) };
                    let left = unsafe { *raw.add((x - bpp) as usize) };
                    let above = unsafe { *prev.add(x as usize) };
                    let average = (u16::from(left) + u16::from(above)) / 2;
                    unsafe {
                        *raw.add(x as usize) = current.wrapping_add(average as u8);
                    }
                }
            }
            4 => {
                for x in 0..bpp {
                    let current = unsafe { *raw.add(x as usize) };
                    let above = unsafe { *prev.add(x as usize) };
                    unsafe { *raw.add(x as usize) = current.wrapping_add(above) };
                }
                for x in bpp..len {
                    let current = unsafe { *raw.add(x as usize) };
                    let left = unsafe { *raw.add((x - bpp) as usize) };
                    let above = unsafe { *prev.add(x as usize) };
                    let upper_left = unsafe { *prev.add((x - bpp) as usize) };
                    unsafe {
                        *raw.add(x as usize) =
                            current.wrapping_add(cp_paeth(left, above, upper_left));
                    }
                }
            }
            _ => return 0,
        }
        prev = raw;
        raw = raw.wrapping_add(len as usize);
    }
    1
}
