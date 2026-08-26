use std::ffi::{c_int, c_uchar, c_void};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy, Default)]
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

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const i8 = ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = [
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
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59,
    67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
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

impl Default for CpState {
    fn default() -> Self {
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

struct RawPng {
    p: *const u8,
    end: *const u8,
}

const ERR_STORED_COMPLEMENTS: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
const ERR_STORED_EXTENDS: &[u8] = b"Stored block extends beyond end of input stream.\0";
const ERR_OVERWRITE_SYMBOL: &[u8] =
    b"Attempted to overwrite out buffer while outputting a symbol.\0";
const ERR_BACK_DISTANCE: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
const ERR_OVERWRITE_STRING: &[u8] =
    b"Attempted to overwrite out buffer while outputting a string.\0";
const ERR_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";
const ERR_SIGNATURE: &[u8] = b"incorrect file signature (is this a png file?)\0";
const ERR_IHDR: &[u8] = b"unable to find IHDR chunk\0";
const ERR_BIT_DEPTH: &[u8] = b"only bit-depth of 8 is supported\0";
const ERR_COLOR_TYPE: &[u8] = b"unknown color type\0";
const ERR_WIDTH: &[u8] = b"invalid IHDR chunk found, image width was less than 1\0";
const ERR_HEIGHT: &[u8] = b"invalid IHDR chunk found, image height was less than 1\0";
const ERR_TOO_LARGE: &[u8] = b"image too large\0";
const ERR_ALLOC_RAW: &[u8] = b"unable to allocate raw image space\0";
const ERR_COMPRESSION: &[u8] = b"only standard compression DEFLATE is supported\0";
const ERR_FILTER: &[u8] = b"only standard adaptive filtering is supported\0";
const ERR_INTERLACE: &[u8] = b"interlacing is not supported\0";
const ERR_ZLIB: &[u8] = b"corrupt zlib structure in DEFLATE stream\0";
const ERR_ZLIB_METHOD: &[u8] = b"only zlib compression method (RFC 1950) is supported\0";
const ERR_WINDOW: &[u8] = b"innapropriate window size detected\0";
const ERR_DICT: &[u8] = b"preset dictionary is present and not supported\0";
const ERR_IMAGE_SIZE: &[u8] = b"invalid image size found\0";
const ERR_DEFLATE: &[u8] = b"DEFLATE algorithm failed\0";
const ERR_FILTER_BYTE: &[u8] = b"invalid filter byte found\0";
const ERR_PLTE: &[u8] = b"color type of indexed requires a PLTE chunk\0";

unsafe fn set_error(s: &'static [u8]) {
    unsafe {
        cp_error_reason = s.as_ptr() as *const i8;
    }
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xff }
}

fn cp_would_overflow(s: &CpState, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

unsafe fn cp_ptr(s: &CpState) -> *mut u8 {
    unsafe { (s.words.add(s.word_index as usize) as *mut u8).sub((s.count / 8) as usize) }
}

unsafe fn cp_peak_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { ptr::read(s.words.add(s.word_index as usize)) };
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
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
    let bits = s.bits & ((1u64 << num_bits_to_read) - 1);
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits as u32
}

unsafe fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    debug_assert!(s.bits_left > 0);
    debug_assert!(s.count <= 64);
    debug_assert!(!cp_would_overflow(s, num_bits_to_read));
    unsafe {
        cp_peak_bits(s, num_bits_to_read);
    }
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    a = ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8);
    a
}

unsafe fn cp_build(s: Option<&mut CpState>, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    for n in 0..sym_count {
        let len = unsafe { *lens.add(n as usize) } as usize;
        counts[len] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15usize {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    let mut s = s;
    if let Some(st) = s.as_deref_mut() {
        st.lookup.fill(0);
    }
    for i in 0..sym_count {
        let len = unsafe { *lens.add(i as usize) } as i32;
        if len != 0 {
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as usize;
            first[len as usize] += 1;
            unsafe {
                *tree.add(slot) = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            }
            if let Some(st) = s.as_deref_mut() {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        st.lookup[j] = ((len << 9) | i) as u16;
                        j += 1usize << len;
                    }
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut CpState) -> c_int {
    unsafe {
        cp_read_bits(s, s.count & 7);
        let len = cp_read_bits(s, 16) as u16;
        let nlen = cp_read_bits(s, 16) as u16;
        if len != !nlen {
            set_error(ERR_STORED_COMPLEMENTS);
            return 0;
        }
        if s.bits_left / 8 > len as c_int {
            set_error(ERR_STORED_EXTENDS);
            return 0;
        }
        let p = cp_ptr(s);
        ptr::copy_nonoverlapping(p, s.out, len as usize);
        s.out = s.out.add(len as usize);
    }
    1
}

unsafe fn cp_fixed(s: &mut CpState) -> c_int {
    unsafe {
        let fixed_ptr = ptr::addr_of_mut!(cp_fixed_table) as *const u8;
        let lit = s.lit.as_mut_ptr();
        s.nlit = cp_build(Some(s), lit, fixed_ptr, 288) as u32;
        s.ndst = cp_build(None, s.dst.as_mut_ptr(), fixed_ptr.add(288), 32) as u32;
    }
    1
}

unsafe fn cp_decode(s: &mut CpState, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = unsafe { cp_peak_bits(s, 16) };
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
    let code = cp_consume_bits(s, (key & 0xf) as c_int);
    let _ = code;
    ((key >> 4) & 0xfff) as c_int
}

unsafe fn cp_dynamic(s: &mut CpState) -> c_int {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + unsafe { cp_read_bits(s, 5) } as c_int;
    let ndst = 1 + unsafe { cp_read_bits(s, 5) } as c_int;
    let nlen = 4 + unsafe { cp_read_bits(s, 4) } as c_int;
    for i in 0..nlen {
        let idx = unsafe { cp_permutation_order[i as usize] } as usize;
        lenlens[idx] = unsafe { cp_read_bits(s, 3) } as u8;
    }
    s.nlen = unsafe { cp_build(None, s.len.as_mut_ptr(), lenlens.as_ptr(), 19) } as u32;
    let mut lens = [0u8; 288 + 32];
    let mut n = 0;
    while n < nlit + ndst {
        let sym = unsafe { cp_decode(s, s.len.as_ptr(), s.nlen as c_int) };
        match sym {
            16 => {
                let mut i = 3 + unsafe { cp_read_bits(s, 2) } as c_int;
                while i != 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + unsafe { cp_read_bits(s, 3) } as c_int;
                while i != 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + unsafe { cp_read_bits(s, 7) } as c_int;
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
    let lit = s.lit.as_mut_ptr();
    s.nlit = unsafe { cp_build(Some(s), lit, lens.as_ptr(), nlit) } as u32;
    s.ndst = unsafe { cp_build(None, s.dst.as_mut_ptr(), lens.as_ptr().add(nlit as usize), ndst) } as u32;
    1
}

unsafe fn cp_block(s: &mut CpState) -> c_int {
    loop {
        let mut symbol = unsafe { cp_decode(s, s.lit.as_ptr(), s.nlit as c_int) };
        if symbol < 256 {
            if unsafe { s.out.add(1) } > s.out_end {
                unsafe { set_error(ERR_OVERWRITE_SYMBOL) };
                return 0;
            }
            unsafe {
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            symbol -= 257;
            let length = unsafe {
                cp_read_bits(s, cp_len_extra_bits[symbol as usize] as c_int)
                    + cp_len_base[symbol as usize]
            } as c_int;
            let distance_symbol = unsafe { cp_decode(s, s.dst.as_ptr(), s.ndst as c_int) };
            let backwards_distance = unsafe {
                cp_read_bits(s, cp_dist_extra_bits[distance_symbol as usize] as c_int)
                    + cp_dist_base[distance_symbol as usize]
            } as c_int;
            if unsafe { s.out.offset(-(backwards_distance as isize)) } < s.begin {
                unsafe { set_error(ERR_BACK_DISTANCE) };
                return 0;
            }
            if unsafe { s.out.add(length as usize) } > s.out_end {
                unsafe { set_error(ERR_OVERWRITE_STRING) };
                return 0;
            }
            let mut src = unsafe { s.out.offset(-(backwards_distance as isize)) };
            let mut dst = s.out;
            unsafe {
                s.out = s.out.add(length as usize);
            }
            match backwards_distance {
                1 => unsafe {
                    ptr::write_bytes(dst, *src, length as usize);
                },
                _ => {
                    let mut l = length;
                    while l != 0 {
                        unsafe {
                            *dst = *src;
                            dst = dst.add(1);
                            src = src.add(1);
                        }
                        l -= 1;
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
    input: *mut c_void,
    in_bytes: c_int,
    output: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let s_ptr = unsafe { libc::calloc(1, size_of::<CpState>()) as *mut CpState };
    if s_ptr.is_null() {
        return 0;
    }
    let s = unsafe { &mut *s_ptr };
    *s = CpState::default();
    let in_ptr = input as *mut u8;
    s.bits_left = in_bytes * 8;
    let first_bytes = ((((in_ptr as usize) + 3) & !3) - (in_ptr as usize)) as c_int;
    s.words = unsafe { in_ptr.add(first_bytes as usize) } as *const u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        unsafe {
            s.bits |= (*in_ptr.add(i as usize) as u64) << (i * 8);
        }
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        unsafe {
            s.final_word |= (*in_ptr.add((in_bytes - last_bytes + i) as usize) as u32) << (i * 8);
        }
    }
    s.count = first_bytes * 8;
    s.out = output as *mut u8;
    s.out_end = unsafe { s.out.add(out_bytes as usize) };
    s.begin = output as *mut u8;
    let mut bfinal;
    loop {
        bfinal = unsafe { cp_read_bits(s, 1) } as c_int;
        let btype = unsafe { cp_read_bits(s, 2) } as c_int;
        match btype {
            0 => {
                if unsafe { cp_stored(s) } == 0 {
                    unsafe { libc::free(s_ptr as *mut c_void) };
                    return 0;
                }
            }
            1 => {
                unsafe { cp_fixed(s) };
                if unsafe { cp_block(s) } == 0 {
                    unsafe { libc::free(s_ptr as *mut c_void) };
                    return 0;
                }
            }
            2 => {
                unsafe { cp_dynamic(s) };
                if unsafe { cp_block(s) } == 0 {
                    unsafe { libc::free(s_ptr as *mut c_void) };
                    return 0;
                }
            }
            3 => {
                unsafe { set_error(ERR_UNKNOWN_BLOCK) };
                unsafe { libc::free(s_ptr as *mut c_void) };
                return 0;
            }
            _ => {}
        }
        if bfinal != 0 {
            break;
        }
    }
    unsafe { libc::free(s_ptr as *mut c_void) };
    1
}

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as c_int + b as c_int - c as c_int;
    let pa = (p - a as c_int).abs();
    let pb = (p - b as c_int).abs();
    let pc = (p - c as c_int).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

unsafe fn cp_make32(s: *const u8) -> u32 {
    unsafe {
        ((*s.add(0) as u32) << 24)
            | ((*s.add(1) as u32) << 16)
            | ((*s.add(2) as u32) << 8)
            | (*s.add(3) as u32)
    }
}

unsafe fn chunk_name_eq(start: *const u8, chunk: &[u8; 4]) -> bool {
    unsafe { ptr::read(start.add(4) as *const [u8; 4]) == *chunk }
}

unsafe fn cp_chunk(png: &mut RawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    let len = unsafe { cp_make32(png.p) };
    let start = png.p;
    if unsafe { chunk_name_eq(start, chunk) } && len >= minlen {
        let offset = len.wrapping_add(12) as usize;
        if unsafe { png.p.add(offset) } <= png.end {
            png.p = unsafe { png.p.add(offset) };
            return unsafe { start.add(8) };
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: &mut RawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    while png.p < png.end {
        let len = unsafe { cp_make32(png.p) };
        let start = png.p;
        png.p = unsafe { png.p.add(len.wrapping_add(12) as usize) };
        if unsafe { chunk_name_eq(start, chunk) } && len >= minlen && png.p <= png.end {
            return unsafe { start.add(8) };
        }
    }
    ptr::null()
}

unsafe fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, mut raw: *mut u8) -> c_int {
    let len = w * bpp;
    let mut x;
    if h > 0 {
        let filter = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    unsafe {
                        let v = (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    unsafe {
                        let v = (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize) / 2);
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    unsafe {
                        let v = (*raw.add(x as usize)).wrapping_add(cp_paeth(*raw.add((x - bpp) as usize), 0, 0));
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }
    }
    let mut prev = raw;
    raw = unsafe { raw.add(len as usize) };
    let mut y = 1;
    while y < h {
        let filter = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        let v = (*raw.add(x as usize)).wrapping_add(0);
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let v = (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        let v = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let v = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        let v = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize) / 2);
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let avg = ((*raw.add((x - bpp) as usize) as c_int + *prev.add(x as usize) as c_int) / 2) as u8;
                        let v = (*raw.add(x as usize)).wrapping_add(avg);
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        let v = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        let v = (*raw.add(x as usize)).wrapping_add(cp_paeth(
                            *raw.add((x - bpp) as usize),
                            *prev.add(x as usize),
                            *prev.add((x - bpp) as usize),
                        ));
                        *raw.add(x as usize) = v;
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }
        y += 1;
        prev = raw;
        raw = unsafe { raw.add(len as usize) };
    }
    1
}

unsafe fn cp_convert(bpp: c_int, w: c_int, h: c_int, mut src: *mut u8, mut dst: *mut cp_pixel_t) {
    for _y in 0..h {
        src = unsafe { src.add(1) };
        for _x in 0..w {
            unsafe {
                match bpp {
                    1 => *dst = cp_make_pixel(*src.add(0), *src.add(0), *src.add(0)),
                    2 => *dst = cp_make_pixel_a(*src.add(0), *src.add(0), *src.add(0), *src.add(1)),
                    3 => *dst = cp_make_pixel(*src.add(0), *src.add(1), *src.add(2)),
                    4 => *dst = cp_make_pixel_a(*src.add(0), *src.add(1), *src.add(2), *src.add(3)),
                    _ => {}
                }
                dst = dst.add(1);
                src = src.add(bpp as usize);
            }
        }
    }
}

unsafe fn cp_get_alpha_for_indexed_image(index: c_int, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() || index as u32 >= trns_len {
        255
    } else {
        unsafe { *trns.add(index as usize) }
    }
}

unsafe fn cp_depalette(
    w: c_int,
    h: c_int,
    mut src: *mut u8,
    mut dst: *mut cp_pixel_t,
    plte: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    for _y in 0..h {
        src = unsafe { src.add(1) };
        for _x in 0..w {
            unsafe {
                let c = *src as c_int;
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
}

unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    unsafe { cp_make32(chunk.sub(8)) }
}

fn cp_out_size(img: &cp_image_t, bpp: c_int) -> c_int {
    (img.w + 1) * img.h * bpp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(
    png_data: *const c_uchar,
    png_length: c_int,
) -> cp_image_t {
    let sig = b"\x89PNG\r\n\x1a\n";
    let bit_depth;
    let color_type;
    let bpp;
    let w;
    let h;
    let pix_bytes;
    let compression;
    let filter;
    let interlace;
    let mut datalen: c_int;
    let mut offset: c_int;
    let out: *mut u8;
    let mut img = cp_image_t {
        w: 0,
        h: 0,
        pix: ptr::null_mut(),
    };
    let data: *mut u8;
    let mut png = RawPng {
        p: png_data,
        end: unsafe { png_data.add(png_length as usize) },
    };

    if unsafe { libc::memcmp(png.p as *const c_void, sig.as_ptr() as *const c_void, 8) } != 0 {
        unsafe { set_error(ERR_SIGNATURE) };
        return img;
    }
    png.p = unsafe { png.p.add(8) };
    let ihdr = unsafe { cp_chunk(&mut png, b"IHDR", 13) };
    if ihdr.is_null() {
        unsafe { set_error(ERR_IHDR) };
        return img;
    }
    bit_depth = unsafe { *ihdr.add(8) } as c_int;
    color_type = unsafe { *ihdr.add(9) } as c_int;
    if bit_depth != 8 {
        unsafe { set_error(ERR_BIT_DEPTH) };
        return img;
    }
    bpp = match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => {
            unsafe { set_error(ERR_COLOR_TYPE) };
            return img;
        }
    };
    w = unsafe { cp_make32(ihdr) as c_int }.wrapping_add(1);
    h = unsafe { cp_make32(ihdr.add(4)) as c_int };
    if w < 1 {
        unsafe { set_error(ERR_WIDTH) };
        return img;
    }
    if h < 1 {
        unsafe { set_error(ERR_HEIGHT) };
        return img;
    }
    if (w as i64) * (h as i64) * (size_of::<cp_pixel_t>() as i64) >= c_int::MAX as i64 {
        unsafe { set_error(ERR_TOO_LARGE) };
        return img;
    }
    pix_bytes = w * h * size_of::<cp_pixel_t>() as c_int;
    img.w = w - 1;
    img.h = h;
    img.pix = unsafe { libc::malloc(pix_bytes as usize) as *mut cp_pixel_t };
    if img.pix.is_null() {
        unsafe { set_error(ERR_ALLOC_RAW) };
        return img;
    }
    compression = unsafe { *ihdr.add(10) } as c_int;
    filter = unsafe { *ihdr.add(11) } as c_int;
    interlace = unsafe { *ihdr.add(12) } as c_int;
    if compression != 0 {
        unsafe { set_error(ERR_COMPRESSION) };
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    if filter != 0 {
        unsafe { set_error(ERR_FILTER) };
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    if interlace != 0 {
        unsafe { set_error(ERR_INTERLACE) };
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }

    let mut first = png.p;
    let plte = unsafe { cp_find(&mut png, b"PLTE", 0) };
    if plte.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }
    let trns = unsafe { cp_find(&mut png, b"tRNS", 0) };
    if trns.is_null() {
        png.p = first;
    } else {
        first = png.p;
    }

    datalen = 0;
    let mut idat = unsafe { cp_find(&mut png, b"IDAT", 0) };
    while !idat.is_null() {
        let len = unsafe { cp_get_chunk_byte_length(idat) };
        datalen = datalen.wrapping_add(len as c_int);
        idat = unsafe { cp_chunk(&mut png, b"IDAT", 0) };
    }
    png.p = first;
    data = unsafe { libc::malloc(datalen as usize) as *mut u8 };
    offset = 0;
    idat = unsafe { cp_find(&mut png, b"IDAT", 0) };
    while !idat.is_null() {
        let len = unsafe { cp_get_chunk_byte_length(idat) };
        unsafe {
            ptr::copy_nonoverlapping(idat, data.add(offset as usize), len as usize);
        }
        offset = offset.wrapping_add(len as c_int);
        idat = unsafe { cp_chunk(&mut png, b"IDAT", 0) };
    }
    if data.is_null() || datalen < 6 {
        unsafe {
            set_error(ERR_ZLIB);
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
        }
        img.pix = ptr::null_mut();
        return img;
    }
    if unsafe { *data.add(0) } & 0x0f != 0x08 {
        unsafe {
            set_error(ERR_ZLIB_METHOD);
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
        }
        img.pix = ptr::null_mut();
        return img;
    }
    if unsafe { *data.add(0) } & 0xf0 > 0x70 {
        unsafe {
            set_error(ERR_WINDOW);
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
        }
        img.pix = ptr::null_mut();
        return img;
    }
    if unsafe { *data.add(1) } & 0x20 != 0 {
        unsafe {
            set_error(ERR_DICT);
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
        }
        img.pix = ptr::null_mut();
        return img;
    }
    if cp_out_size(&img, 4) < 1 {
        unsafe {
            set_error(ERR_IMAGE_SIZE);
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
        }
        img.pix = ptr::null_mut();
        return img;
    }
    if cp_out_size(&img, bpp) < 1 {
        unsafe {
            set_error(ERR_IMAGE_SIZE);
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
        }
        img.pix = ptr::null_mut();
        return img;
    }
    out = unsafe {
        (img.pix as *mut u8).add((cp_out_size(&img, 4) - cp_out_size(&img, bpp)) as usize)
    };
    if unsafe { cp_inflate(data.add(2) as *mut c_void, datalen - 6, out as *mut c_void, pix_bytes) } == 0 {
        unsafe {
            set_error(ERR_DEFLATE);
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
        }
        img.pix = ptr::null_mut();
        return img;
    }
    if unsafe { cp_unfilter(img.w, img.h, bpp, out) } == 0 {
        unsafe {
            set_error(ERR_FILTER_BYTE);
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
        }
        img.pix = ptr::null_mut();
        return img;
    }
    if color_type == 3 {
        if plte.is_null() {
            unsafe {
                set_error(ERR_PLTE);
                libc::free(data as *mut c_void);
                libc::free(img.pix as *mut c_void);
            }
            img.pix = ptr::null_mut();
            return img;
        }
        let trns_len = if trns.is_null() {
            0
        } else {
            unsafe { cp_get_chunk_byte_length(trns) }
        };
        unsafe { cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len) };
    } else {
        unsafe { cp_convert(bpp, img.w, img.h, out, img.pix) };
    }
    unsafe { libc::free(data as *mut c_void) };
    img
}
