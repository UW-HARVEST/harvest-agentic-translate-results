#![allow(dead_code, non_upper_case_globals, unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
struct cp_image_t {
    w: c_int,
    h: c_int,
    pix: *mut cp_pixel_t,
}

const fn make_fixed_table() -> [u8; 320] {
    let mut table = [0; 320];
    let mut i = 0;
    while i < 144 {
        table[i] = 8;
        i += 1;
    }
    while i < 256 {
        table[i] = 9;
        i += 1;
    }
    while i < 280 {
        table[i] = 7;
        i += 1;
    }
    while i < 288 {
        table[i] = 8;
        i += 1;
    }
    while i < 320 {
        table[i] = 5;
        i += 1;
    }
    table
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 288 + 32] = make_fixed_table();

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

static ERR_STORED_COMPLEMENT: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
static ERR_STORED_END: &[u8] = b"Stored block extends beyond end of input stream.\0";
static ERR_SYMBOL_OUTPUT: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.\0";
static ERR_BACKWARDS_DISTANCE: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
static ERR_STRING_OUTPUT: &[u8] = b"Attempted to overwrite out buffer while outputting a string.\0";
static ERR_BLOCK_TYPE: &[u8] = b"Detected unknown block type within input stream.\0";

#[inline]
unsafe fn set_error(message: &'static [u8]) {
    cp_error_reason = message.as_ptr().cast();
}

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

#[inline]
fn cp_would_overflow(s: &CpState, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

#[inline]
unsafe fn cp_ptr(s: &CpState) -> *const u8 {
    s.words
        .add(s.word_index as usize)
        .cast::<u8>()
        .offset(-((s.count / 8) as isize))
}

#[inline]
unsafe fn cp_peak_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = ptr::read(s.words.add(s.word_index as usize));
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
        } else if s.final_word_available != 0 {
            s.bits |= (s.final_word as u64) << s.count;
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

#[inline]
fn cp_consume_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    let bits = s.bits & ((1_u64 << num_bits_to_read) - 1);
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits as u32
}

#[inline]
unsafe fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    let _ = cp_would_overflow(s, num_bits_to_read);
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

#[inline]
fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8)
}

unsafe fn cp_build(s: *mut CpState, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0_i32; 16];
    let mut first = [0_i32; 16];
    let mut counts = [0_i32; 16];

    for n in 0..sym_count {
        counts[ptr::read(lens.add(n as usize)) as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if !s.is_null() {
        (*s).lookup.fill(0);
    }
    for i in 0..sym_count {
        let bit_len = ptr::read(lens.add(i as usize)) as usize;
        if bit_len != 0 {
            let code = codes[bit_len] as u32;
            codes[bit_len] += 1;
            let slot = first[bit_len];
            first[bit_len] += 1;
            ptr::write(
                tree.add(slot as usize),
                (code << (32 - bit_len)) | ((i as u32) << 4) | bit_len as u32,
            );
            if !s.is_null() && bit_len <= 9 {
                let mut j = (cp_rev16(code) >> (16 - bit_len)) as usize;
                while j < (1 << 9) {
                    (*s).lookup[j] = ((bit_len << 9) | i as usize) as u16;
                    j += 1 << bit_len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut CpState) -> bool {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        set_error(ERR_STORED_COMPLEMENT);
        return false;
    }
    if s.bits_left / 8 > len as c_int {
        set_error(ERR_STORED_END);
        return false;
    }
    let source = cp_ptr(s);
    ptr::copy_nonoverlapping(source, s.out, len as usize);
    s.out = s.out.add(len as usize);
    true
}

unsafe fn cp_fixed(s: &mut CpState) -> bool {
    s.nlit = cp_build(
        s,
        s.lit.as_mut_ptr(),
        ptr::addr_of!(cp_fixed_table).cast::<u8>(),
        288,
    ) as u32;
    s.ndst = cp_build(
        ptr::null_mut(),
        s.dst.as_mut_ptr(),
        ptr::addr_of!(cp_fixed_table).cast::<u8>().add(288),
        32,
    ) as u32;
    true
}

unsafe fn cp_decode(s: &mut CpState, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xffff;
    let mut lo = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < ptr::read(tree.add(guess as usize)) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = ptr::read(tree.add((lo - 1) as usize));
    let code = cp_consume_bits(s, (key & 0x0f) as c_int);
    let _ = code;
    ((key >> 4) & 0x0fff) as c_int
}

unsafe fn cp_dynamic(s: &mut CpState) -> bool {
    let mut lenlens = [0_u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as c_int;
    let ndst = 1 + cp_read_bits(s, 5) as c_int;
    let nlen = 4 + cp_read_bits(s, 4) as c_int;
    for i in 0..nlen {
        let order = ptr::read(
            ptr::addr_of!(cp_permutation_order)
                .cast::<u8>()
                .add(i as usize),
        );
        lenlens[order as usize] = cp_read_bits(s, 3) as u8;
    }
    s.nlen = cp_build(ptr::null_mut(), s.len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;

    let mut lens = [0_u8; 288 + 32];
    let mut n = 0;
    while n < nlit + ndst {
        let symbol = cp_decode(s, s.len.as_ptr(), s.nlen as c_int);
        match symbol {
            16 => {
                let mut count = 3 + cp_read_bits(s, 2) as c_int;
                while count != 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    count -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut count = 3 + cp_read_bits(s, 3) as c_int;
                while count != 0 {
                    lens[n as usize] = 0;
                    count -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut count = 11 + cp_read_bits(s, 7) as c_int;
                while count != 0 {
                    lens[n as usize] = 0;
                    count -= 1;
                    n += 1;
                }
            }
            _ => {
                lens[n as usize] = symbol as u8;
                n += 1;
            }
        }
    }
    s.nlit = cp_build(s, s.lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
    s.ndst = cp_build(
        ptr::null_mut(),
        s.dst.as_mut_ptr(),
        lens.as_ptr().add(nlit as usize),
        ndst,
    ) as u32;
    true
}

unsafe fn cp_block(s: &mut CpState) -> bool {
    loop {
        let mut symbol = cp_decode(s, s.lit.as_ptr(), s.nlit as c_int);
        if symbol < 256 {
            if (s.out as usize).wrapping_add(1) > s.out_end as usize {
                set_error(ERR_SYMBOL_OUTPUT);
                return false;
            }
            ptr::write(s.out, symbol as u8);
            s.out = s.out.add(1);
        } else if symbol > 256 {
            symbol -= 257;
            let len_extra = ptr::read(
                ptr::addr_of!(cp_len_extra_bits)
                    .cast::<u8>()
                    .add(symbol as usize),
            );
            let len_base = ptr::read(
                ptr::addr_of!(cp_len_base)
                    .cast::<u32>()
                    .add(symbol as usize),
            );
            let mut length = cp_read_bits(s, len_extra as c_int).wrapping_add(len_base) as c_int;
            let distance_symbol = cp_decode(s, s.dst.as_ptr(), s.ndst as c_int);
            let distance_extra = ptr::read(
                ptr::addr_of!(cp_dist_extra_bits)
                    .cast::<u8>()
                    .add(distance_symbol as usize),
            );
            let distance_base = ptr::read(
                ptr::addr_of!(cp_dist_base)
                    .cast::<u32>()
                    .add(distance_symbol as usize),
            );
            let backwards_distance =
                cp_read_bits(s, distance_extra as c_int).wrapping_add(distance_base) as c_int;

            if (s.out as usize).wrapping_sub(backwards_distance as usize) < s.begin as usize {
                set_error(ERR_BACKWARDS_DISTANCE);
                return false;
            }
            if (s.out as usize).wrapping_add(length as usize) > s.out_end as usize {
                set_error(ERR_STRING_OUTPUT);
                return false;
            }
            let mut source = s.out.offset(-(backwards_distance as isize));
            let mut destination = s.out;
            s.out = s.out.add(length as usize);
            if backwards_distance == 1 {
                ptr::write_bytes(destination, ptr::read(source), length as usize);
            } else {
                while length != 0 {
                    ptr::write(destination, ptr::read(source));
                    destination = destination.add(1);
                    source = source.add(1);
                    length -= 1;
                }
            }
        } else {
            break;
        }
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    output: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut state = Box::new(CpState::zeroed());
    let input = input.cast::<u8>();
    let output = output.cast::<u8>();

    state.bits_left = in_bytes * 8;
    let address = input as usize;
    let first_bytes = (((address + 3) & !3) - address) as c_int;
    state.words = input.add(first_bytes as usize).cast::<u32>();
    state.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        state.bits |= (ptr::read(input.add(i as usize)) as u64) << (i * 8);
    }
    state.final_word_available = c_int::from(last_bytes != 0);
    state.final_word = 0;
    for i in 0..last_bytes {
        state.final_word |=
            (ptr::read(input.offset((in_bytes - last_bytes + i) as isize)) as u32) << (i * 8);
    }
    state.count = first_bytes * 8;
    state.out = output;
    state.out_end = output.offset(out_bytes as isize);
    state.begin = output;

    loop {
        let bfinal = cp_read_bits(&mut state, 1);
        let btype = cp_read_bits(&mut state, 2);
        match btype {
            0 => {
                if !cp_stored(&mut state) {
                    return 0;
                }
            }
            1 => {
                cp_fixed(&mut state);
                if !cp_block(&mut state) {
                    return 0;
                }
            }
            2 => {
                cp_dynamic(&mut state);
                if !cp_block(&mut state) {
                    return 0;
                }
            }
            _ => {
                set_error(ERR_BLOCK_TYPE);
                return 0;
            }
        }
        if bfinal != 0 {
            break;
        }
    }
    1
}

#[inline]
fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

#[inline]
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xff }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn convert_pix(
    bpp: c_int,
    w: c_int,
    h: c_int,
    mut source: *mut u8,
    mut destination: *mut cp_pixel_t,
) {
    for _ in 0..h {
        source = source.add(1);
        for _ in 0..w {
            let pixel = match bpp {
                1 => Some(cp_make_pixel(
                    ptr::read(source),
                    ptr::read(source),
                    ptr::read(source),
                )),
                2 => Some(cp_make_pixel_a(
                    ptr::read(source),
                    ptr::read(source),
                    ptr::read(source),
                    ptr::read(source.add(1)),
                )),
                3 => Some(cp_make_pixel(
                    ptr::read(source),
                    ptr::read(source.add(1)),
                    ptr::read(source.add(2)),
                )),
                4 => Some(cp_make_pixel_a(
                    ptr::read(source),
                    ptr::read(source.add(1)),
                    ptr::read(source.add(2)),
                    ptr::read(source.add(3)),
                )),
                _ => None,
            };
            if let Some(pixel) = pixel {
                ptr::write(destination, pixel);
                destination = destination.add(1);
            }
            source = source.offset(bpp as isize);
        }
    }
}

#[inline]
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

struct CpRawPng {
    p: *const u8,
    end: *const u8,
}

#[inline]
unsafe fn cp_make32(source: *const u8) -> u32 {
    ((ptr::read(source) as u32) << 24)
        | ((ptr::read(source.add(1)) as u32) << 16)
        | ((ptr::read(source.add(2)) as u32) << 8)
        | ptr::read(source.add(3)) as u32
}

unsafe fn cp_chunk(png: &mut CpRawPng, chunk: *const c_char, min_len: u32) -> *const u8 {
    let len = cp_make32(png.p);
    let start = png.p;
    if std::slice::from_raw_parts(start.add(4), 4)
        == std::slice::from_raw_parts(chunk.cast::<u8>(), 4)
        && len >= min_len
    {
        let offset = len.wrapping_add(12) as c_int;
        if (png.p as usize).wrapping_add(offset as usize) <= png.end as usize {
            png.p = png.p.offset(offset as isize);
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: &mut CpRawPng, chunk: *const c_char, min_len: u32) -> *const u8 {
    while png.p < png.end {
        let len = cp_make32(png.p);
        let start = png.p;
        png.p = png.p.offset(len.wrapping_add(12) as isize);
        if std::slice::from_raw_parts(start.add(4), 4)
            == std::slice::from_raw_parts(chunk.cast::<u8>(), 4)
            && len >= min_len
            && png.p <= png.end
        {
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn add_byte(destination: *mut u8, value: u8) {
    ptr::write(destination, ptr::read(destination).wrapping_add(value));
}

unsafe fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, mut raw: *mut u8) -> c_int {
    let len = w * bpp;
    if h > 0 {
        let filter = ptr::read(raw);
        raw = raw.add(1);
        match filter {
            0 | 2 => {}
            1 => {
                for x in bpp..len {
                    add_byte(
                        raw.offset(x as isize),
                        ptr::read(raw.offset((x - bpp) as isize)),
                    );
                }
            }
            3 => {
                for x in bpp..len {
                    add_byte(
                        raw.offset(x as isize),
                        ptr::read(raw.offset((x - bpp) as isize)) / 2,
                    );
                }
            }
            4 => {
                for x in bpp..len {
                    add_byte(
                        raw.offset(x as isize),
                        cp_paeth(ptr::read(raw.offset((x - bpp) as isize)), 0, 0),
                    );
                }
            }
            _ => return 0,
        }
    }

    let mut previous = raw;
    raw = raw.offset(len as isize);
    for _ in 1..h {
        let filter = ptr::read(raw);
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                for x in bpp..len {
                    add_byte(
                        raw.offset(x as isize),
                        ptr::read(raw.offset((x - bpp) as isize)),
                    );
                }
            }
            2 => {
                for x in 0..len {
                    add_byte(
                        raw.offset(x as isize),
                        ptr::read(previous.offset(x as isize)),
                    );
                }
            }
            3 => {
                for x in 0..bpp {
                    add_byte(
                        raw.offset(x as isize),
                        ptr::read(previous.offset(x as isize)) / 2,
                    );
                }
                for x in bpp..len {
                    add_byte(
                        raw.offset(x as isize),
                        ((ptr::read(raw.offset((x - bpp) as isize)) as c_int
                            + ptr::read(previous.offset(x as isize)) as c_int)
                            / 2) as u8,
                    );
                }
            }
            4 => {
                for x in 0..bpp {
                    add_byte(
                        raw.offset(x as isize),
                        ptr::read(previous.offset(x as isize)),
                    );
                }
                for x in bpp..len {
                    add_byte(
                        raw.offset(x as isize),
                        cp_paeth(
                            ptr::read(raw.offset((x - bpp) as isize)),
                            ptr::read(previous.offset(x as isize)),
                            ptr::read(previous.offset((x - bpp) as isize)),
                        ),
                    );
                }
            }
            _ => return 0,
        }
        previous = raw;
        raw = raw.offset(len as isize);
    }
    1
}
