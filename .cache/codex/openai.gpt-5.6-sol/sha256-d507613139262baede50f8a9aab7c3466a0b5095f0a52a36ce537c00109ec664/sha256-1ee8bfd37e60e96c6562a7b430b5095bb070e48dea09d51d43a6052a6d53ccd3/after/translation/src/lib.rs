use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;
use std::ptr;

unsafe extern "C" {
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dst: *mut c_void, src: *const c_void, count: usize) -> *mut c_void;
    fn memset(dst: *mut c_void, value: c_int, count: usize) -> *mut c_void;
    fn memcmp(left: *const c_void, right: *const c_void, count: usize) -> c_int;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

const fn fixed_table() -> [u8; 320] {
    let mut result = [0; 320];
    let mut i = 0;
    while i < 144 {
        result[i] = 8;
        i += 1;
    }
    while i < 256 {
        result[i] = 9;
        i += 1;
    }
    while i < 280 {
        result[i] = 7;
        i += 1;
    }
    while i < 288 {
        result[i] = 8;
        i += 1;
    }
    while i < 320 {
        result[i] = 5;
        i += 1;
    }
    result
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 320] = fixed_table();

#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

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

unsafe fn set_error(reason: &'static [u8]) {
    unsafe {
        ptr::write(
            ptr::addr_of_mut!(cp_error_reason),
            reason.as_ptr().cast::<c_char>(),
        );
    }
}

fn make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> CpPixel {
    CpPixel { r, g, b, a }
}

fn make_pixel(r: u8, g: u8, b: u8) -> CpPixel {
    make_pixel_a(r, g, b, 0xff)
}

unsafe fn would_overflow(s: *mut CpState, num_bits: c_int) -> bool {
    unsafe { ((*s).bits_left + (*s).count) - num_bits < 0 }
}

unsafe fn state_ptr(s: *mut CpState) -> *mut c_char {
    unsafe {
        assert!((*s).bits_left & 7 == 0);
        (*s).words
            .add((*s).word_index as usize)
            .cast::<c_char>()
            .sub(((*s).count / 8) as usize)
    }
}

unsafe fn peek_bits(s: *mut CpState, num_bits_to_read: c_int) -> u64 {
    unsafe {
        if (*s).count < num_bits_to_read {
            if (*s).word_index < (*s).word_count {
                let word = (*s).words.add((*s).word_index as usize).read();
                (*s).word_index += 1;
                (*s).bits |= (word as u64) << (*s).count;
                (*s).count += 32;
                assert!((*s).word_index <= (*s).word_count);
            } else if (*s).final_word_available != 0 {
                let word = (*s).final_word;
                (*s).bits |= (word as u64) << (*s).count;
                (*s).count += (*s).bits_left;
                (*s).final_word_available = 0;
            }
        }
        (*s).bits
    }
}

unsafe fn consume_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    unsafe {
        assert!((*s).count >= num_bits_to_read);
        let bits = (*s).bits & ((1u64 << num_bits_to_read) - 1);
        (*s).bits >>= num_bits_to_read;
        (*s).count -= num_bits_to_read;
        (*s).bits_left -= num_bits_to_read;
        bits as u32
    }
}

unsafe fn read_bits(s: *mut CpState, num_bits_to_read: c_int) -> u32 {
    unsafe {
        assert!(num_bits_to_read <= 32);
        assert!(num_bits_to_read >= 0);
        assert!((*s).bits_left > 0);
        assert!((*s).count <= 64);
        assert!(!would_overflow(s, num_bits_to_read));
        peek_bits(s, num_bits_to_read);
        consume_bits(s, num_bits_to_read)
    }
}

fn rev16(mut a: u32) -> u32 {
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8)
}

unsafe fn build_tree(s: *mut CpState, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    unsafe {
        for n in 0..sym_count {
            counts[*lens.add(n as usize) as usize] += 1;
        }
        counts[0] = 0;
        codes[0] = 0;
        first[0] = 0;
        for n in 1..=15 {
            codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
            first[n] = first[n - 1] + counts[n - 1];
        }
        if !s.is_null() {
            ptr::write_bytes(ptr::addr_of_mut!((*s).lookup).cast::<u16>(), 0, 1 << 9);
        }
        for i in 0..sym_count {
            let tree_len = *lens.add(i as usize) as c_int;
            if tree_len != 0 {
                assert!(tree_len < 16);
                let code = codes[tree_len as usize] as u32;
                codes[tree_len as usize] += 1;
                let slot = first[tree_len as usize];
                first[tree_len as usize] += 1;
                *tree.add(slot as usize) =
                    (code << (32 - tree_len)) | ((i as u32) << 4) | tree_len as u32;
                if !s.is_null() && tree_len <= 9 {
                    let mut j = (rev16(code) >> (16 - tree_len)) as c_int;
                    while j < (1 << 9) {
                        (*s).lookup[j as usize] = ((tree_len << 9) | i) as u16;
                        j += 1 << tree_len;
                    }
                }
            }
        }
        first[15]
    }
}

unsafe fn stored(s: *mut CpState) -> c_int {
    unsafe {
        read_bits(s, (*s).count & 7);
        let len = read_bits(s, 16) as u16;
        let nlen = read_bits(s, 16) as u16;
        if len != !nlen {
            set_error(
                b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0",
            );
            return 0;
        }
        if (*s).bits_left / 8 > len as c_int {
            set_error(b"Stored block extends beyond end of input stream.\0");
            return 0;
        }
        let p = state_ptr(s);
        memcpy((*s).out.cast(), p.cast(), len as usize);
        (*s).out = (*s).out.add(len as usize);
        1
    }
}

unsafe fn fixed(s: *mut CpState) -> c_int {
    unsafe {
        let lens = ptr::addr_of!(cp_fixed_table).cast::<u8>();
        (*s).nlit = build_tree(s, ptr::addr_of_mut!((*s).lit).cast::<u32>(), lens, 288) as u32;
        (*s).ndst = build_tree(
            ptr::null_mut(),
            ptr::addr_of_mut!((*s).dst).cast::<u32>(),
            lens.add(288),
            32,
        ) as u32;
        1
    }
}

unsafe fn decode(s: *mut CpState, tree: *mut u32, mut hi: c_int) -> c_int {
    unsafe {
        let bits = peek_bits(s, 16);
        let search = (rev16(bits as u32) << 16) | 0xffff;
        let mut lo = 0;
        while lo < hi {
            let guess = (lo + hi) >> 1;
            if search < *tree.add(guess as usize) {
                hi = guess;
            } else {
                lo = guess + 1;
            }
        }
        let key = *tree.add((lo - 1) as usize);
        let len = 32 - (key & 0xf);
        assert!((search >> len) == (key >> len));
        let code = consume_bits(s, (key & 0xf) as c_int);
        let _ = code;
        ((key >> 4) & 0xfff) as c_int
    }
}

unsafe fn dynamic(s: *mut CpState) -> c_int {
    let mut len_lens = [0u8; 19];
    let mut lens = [0u8; 288 + 32];
    unsafe {
        let nlit = 257 + read_bits(s, 5) as c_int;
        let ndst = 1 + read_bits(s, 5) as c_int;
        let nlen = 4 + read_bits(s, 4) as c_int;
        let permutation = ptr::addr_of!(cp_permutation_order).cast::<u8>();
        for i in 0..nlen {
            len_lens[*permutation.add(i as usize) as usize] = read_bits(s, 3) as u8;
        }
        (*s).nlen = build_tree(
            ptr::null_mut(),
            ptr::addr_of_mut!((*s).len).cast::<u32>(),
            len_lens.as_ptr(),
            19,
        ) as u32;
        let mut n = 0;
        while n < nlit + ndst {
            let sym = decode(
                s,
                ptr::addr_of_mut!((*s).len).cast::<u32>(),
                (*s).nlen as c_int,
            );
            match sym {
                16 => {
                    let mut i = 3 + read_bits(s, 2) as c_int;
                    while i != 0 {
                        lens[n as usize] = lens[(n - 1) as usize];
                        i -= 1;
                        n += 1;
                    }
                }
                17 => {
                    let mut i = 3 + read_bits(s, 3) as c_int;
                    while i != 0 {
                        lens[n as usize] = 0;
                        i -= 1;
                        n += 1;
                    }
                }
                18 => {
                    let mut i = 11 + read_bits(s, 7) as c_int;
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
        (*s).nlit = build_tree(
            s,
            ptr::addr_of_mut!((*s).lit).cast::<u32>(),
            lens.as_ptr(),
            nlit,
        ) as u32;
        (*s).ndst = build_tree(
            ptr::null_mut(),
            ptr::addr_of_mut!((*s).dst).cast::<u32>(),
            lens.as_ptr().add(nlit as usize),
            ndst,
        ) as u32;
        1
    }
}

unsafe fn block(s: *mut CpState) -> c_int {
    unsafe {
        loop {
            let mut symbol = decode(
                s,
                ptr::addr_of_mut!((*s).lit).cast::<u32>(),
                (*s).nlit as c_int,
            );
            if symbol < 256 {
                if ((*s).out as usize).wrapping_add(1) > (*s).out_end as usize {
                    set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                    return 0;
                }
                *(*s).out = symbol as c_char;
                (*s).out = (*s).out.add(1);
            } else if symbol > 256 {
                symbol -= 257;
                let len_extra = *ptr::addr_of!(cp_len_extra_bits)
                    .cast::<u8>()
                    .add(symbol as usize);
                let len_base = *ptr::addr_of!(cp_len_base)
                    .cast::<u32>()
                    .add(symbol as usize);
                let mut length = read_bits(s, len_extra as c_int) + len_base;
                let distance_symbol = decode(
                    s,
                    ptr::addr_of_mut!((*s).dst).cast::<u32>(),
                    (*s).ndst as c_int,
                );
                let dist_extra = *ptr::addr_of!(cp_dist_extra_bits)
                    .cast::<u8>()
                    .add(distance_symbol as usize);
                let dist_base = *ptr::addr_of!(cp_dist_base)
                    .cast::<u32>()
                    .add(distance_symbol as usize);
                let backwards_distance = read_bits(s, dist_extra as c_int) + dist_base;
                if ((*s).out as usize).wrapping_sub(backwards_distance as usize)
                    < (*s).begin as usize
                {
                    set_error(
                        b"Attempted to write before out buffer (invalid backwards distance).\0",
                    );
                    return 0;
                }
                if ((*s).out as usize).wrapping_add(length as usize) > (*s).out_end as usize {
                    set_error(b"Attempted to overwrite out buffer while outputting a string.\0");
                    return 0;
                }
                let mut src = (*s).out.sub(backwards_distance as usize);
                let mut dst = (*s).out;
                (*s).out = (*s).out.add(length as usize);
                if backwards_distance == 1 {
                    memset(dst.cast(), *src as c_int, length as usize);
                } else {
                    while length != 0 {
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
                        length -= 1;
                    }
                }
            } else {
                break;
            }
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    output: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        let s = calloc(1, size_of::<CpState>()).cast::<CpState>();
        (*s).bits = 0;
        (*s).count = 0;
        (*s).word_index = 0;
        (*s).bits_left = in_bytes.wrapping_mul(8);
        let input_addr = input as usize;
        let first_bytes = ((input_addr.wrapping_add(3) & !3).wrapping_sub(input_addr)) as c_int;
        (*s).words = input.cast::<u8>().add(first_bytes as usize).cast::<u32>();
        (*s).word_count = (in_bytes - first_bytes) / 4;
        let last_bytes = (in_bytes - first_bytes) & 3;
        for i in 0..first_bytes {
            (*s).bits |= (*input.cast::<u8>().add(i as usize) as u64) << (i.wrapping_mul(8) as u32);
        }
        (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
        (*s).final_word = 0;
        for i in 0..last_bytes {
            (*s).final_word |= (*input.cast::<u8>().add((in_bytes - last_bytes + i) as usize)
                as u32)
                << (i.wrapping_mul(8) as u32);
        }
        (*s).count = first_bytes * 8;
        (*s).out = output.cast::<c_char>();
        (*s).out_end = (*s).out.add(out_bytes as usize);
        (*s).begin = output.cast::<c_char>();
        let mut bfinal;
        loop {
            bfinal = read_bits(s, 1);
            let btype = read_bits(s, 2);
            match btype {
                0 => {
                    if stored(s) == 0 {
                        free(s.cast());
                        return 0;
                    }
                }
                1 => {
                    fixed(s);
                    if block(s) == 0 {
                        free(s.cast());
                        return 0;
                    }
                }
                2 => {
                    dynamic(s);
                    if block(s) == 0 {
                        free(s.cast());
                        return 0;
                    }
                }
                3 => {
                    set_error(b"Detected unknown block type within input stream.\0");
                    free(s.cast());
                    return 0;
                }
                _ => {}
            }
            if bfinal != 0 {
                break;
            }
        }
        free(s.cast());
        1
    }
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
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

#[repr(C)]
struct RawPng {
    p: *const u8,
    end: *const u8,
}

unsafe fn make32(s: *const u8) -> u32 {
    unsafe {
        ((*s as u32) << 24)
            | ((*s.add(1) as u32) << 16)
            | ((*s.add(2) as u32) << 8)
            | *s.add(3) as u32
    }
}

unsafe fn chunk(png: *mut RawPng, chunk_name: *const u8, min_len: u32) -> *const u8 {
    unsafe {
        let len = make32((*png).p);
        let start = (*png).p;
        if memcmp(start.add(4).cast(), chunk_name.cast(), 4) == 0 && len >= min_len {
            let offset = len.wrapping_add(12) as i32;
            let next = (*png).p.wrapping_offset(offset as isize);
            if next as usize <= (*png).end as usize {
                (*png).p = next;
                return start.add(8);
            }
        }
        ptr::null()
    }
}

unsafe fn find(png: *mut RawPng, chunk_name: *const u8, min_len: u32) -> *const u8 {
    unsafe {
        while ((*png).p as usize) < ((*png).end as usize) {
            let len = make32((*png).p);
            let start = (*png).p;
            (*png).p = (*png).p.wrapping_add(len.wrapping_add(12) as usize);
            if memcmp(start.add(4).cast(), chunk_name.cast(), 4) == 0
                && len >= min_len
                && ((*png).p as usize) <= ((*png).end as usize)
            {
                return start.add(8);
            }
        }
        ptr::null()
    }
}

unsafe fn unfilter(w: c_int, h: c_int, bpp: c_int, mut raw: *mut u8) -> c_int {
    unsafe {
        let len = w.wrapping_mul(bpp);
        if h > 0 {
            let filter = *raw;
            raw = raw.add(1);
            match filter {
                0 | 2 => {}
                1 => {
                    for x in bpp..len {
                        let value =
                            (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                        *raw.add(x as usize) = value;
                    }
                }
                3 => {
                    for x in bpp..len {
                        let value =
                            (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize) / 2);
                        *raw.add(x as usize) = value;
                    }
                }
                4 => {
                    for x in bpp..len {
                        let value = (*raw.add(x as usize)).wrapping_add(paeth(
                            *raw.add((x - bpp) as usize),
                            0,
                            0,
                        ));
                        *raw.add(x as usize) = value;
                    }
                }
                _ => return 0,
            }
        }
        let mut prev = raw;
        raw = raw.add(len as usize);
        for _ in 1..h {
            let filter = *raw;
            raw = raw.add(1);
            match filter {
                0 => {}
                1 => {
                    for x in bpp..len {
                        *raw.add(x as usize) =
                            (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                    }
                }
                2 => {
                    for x in 0..len {
                        *raw.add(x as usize) =
                            (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                    }
                }
                3 => {
                    for x in 0..bpp {
                        *raw.add(x as usize) =
                            (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize) / 2);
                    }
                    for x in bpp..len {
                        let average = (*raw.add((x - bpp) as usize) as c_int
                            + *prev.add(x as usize) as c_int)
                            / 2;
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(average as u8);
                    }
                }
                4 => {
                    for x in 0..bpp {
                        *raw.add(x as usize) =
                            (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                    }
                    for x in bpp..len {
                        let prediction = paeth(
                            *raw.add((x - bpp) as usize),
                            *prev.add(x as usize),
                            *prev.add((x - bpp) as usize),
                        );
                        *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(prediction);
                    }
                }
                _ => return 0,
            }
            prev = raw;
            raw = raw.add(len as usize);
        }
        1
    }
}

unsafe fn convert(bpp: c_int, w: c_int, h: c_int, mut src: *mut u8, mut dst: *mut CpPixel) {
    unsafe {
        for _ in 0..h {
            src = src.add(1);
            for _ in 0..w {
                *dst = match bpp {
                    1 => make_pixel(*src, *src, *src),
                    2 => make_pixel_a(*src, *src, *src, *src.add(1)),
                    3 => make_pixel(*src, *src.add(1), *src.add(2)),
                    4 => make_pixel_a(*src, *src.add(1), *src.add(2), *src.add(3)),
                    _ => CpPixel {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    },
                };
                dst = dst.add(1);
                src = src.add(bpp as usize);
            }
        }
    }
}

unsafe fn alpha_for_index(index: c_int, trns: *const u8, trns_len: u32) -> u8 {
    unsafe {
        if trns.is_null() || index as u32 >= trns_len {
            255
        } else {
            *trns.add(index as usize)
        }
    }
}

unsafe fn depalette(
    w: c_int,
    h: c_int,
    mut src: *mut u8,
    mut dst: *mut CpPixel,
    palette: *const u8,
    trns: *const u8,
    trns_len: u32,
) {
    unsafe {
        for _ in 0..h {
            src = src.add(1);
            for _ in 0..w {
                let c = *src as c_int;
                let entry = palette.add((c * 3) as usize);
                *dst = make_pixel_a(
                    *entry,
                    *entry.add(1),
                    *entry.add(2),
                    alpha_for_index(c, trns, trns_len),
                );
                src = src.add(1);
                dst = dst.add(1);
            }
        }
    }
}

unsafe fn chunk_byte_length(chunk_data: *const u8) -> u32 {
    unsafe { make32(chunk_data.sub(8)) }
}

fn out_size(img: &CpImage, bpp: c_int) -> c_int {
    img.w.wrapping_add(1).wrapping_mul(img.h).wrapping_mul(bpp)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> CpImage {
    unsafe {
        let mut img = CpImage {
            w: 0,
            h: 0,
            pix: ptr::null_mut(),
        };
        let mut png = RawPng {
            p: png_data,
            end: png_data.wrapping_offset(png_length as isize),
        };
        if memcmp(png.p.cast(), b"\x89PNG\r\n\x1a\n".as_ptr().cast(), 8) != 0 {
            set_error(b"incorrect file signature (is this a png file?)\0");
            return img;
        }
        png.p = png.p.add(8);
        let ihdr = chunk(&mut png, b"IHDR".as_ptr(), 13);
        if ihdr.is_null() {
            set_error(b"unable to find IHDR chunk\0");
            return img;
        }
        let bit_depth = *ihdr.add(8) as c_int;
        let color_type = *ihdr.add(9) as c_int;
        if bit_depth != 8 {
            set_error(b"only bit-depth of 8 is supported\0");
            return img;
        }
        let bpp = match color_type {
            0 => 1,
            2 => 3,
            3 => 1,
            4 => 2,
            6 => 4,
            _ => {
                set_error(b"unknown color type\0");
                return img;
            }
        };
        let w = (make32(ihdr) as c_int).wrapping_add(1);
        let h = make32(ihdr.add(4)) as c_int;
        if w < 1 {
            set_error(b"invalid IHDR chunk found, image width was less than 1\0");
            return img;
        }
        if h < 1 {
            set_error(b"invalid IHDR chunk found, image height was less than 1\0");
            return img;
        }
        if (w as i64) * (h as i64) * (size_of::<CpPixel>() as i64) >= c_int::MAX as i64 {
            set_error(b"image too large\0");
            return img;
        }
        let pix_bytes = w
            .wrapping_mul(h)
            .wrapping_mul(size_of::<CpPixel>() as c_int);
        img.w = w - 1;
        img.h = h;
        img.pix = malloc(pix_bytes as usize).cast::<CpPixel>();
        if img.pix.is_null() {
            set_error(b"unable to allocate raw image space\0");
            return img;
        }
        let compression = *ihdr.add(10);
        let filter = *ihdr.add(11);
        let interlace = *ihdr.add(12);
        if compression != 0 {
            set_error(b"only standard compression DEFLATE is supported\0");
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        if filter != 0 {
            set_error(b"only standard adaptive filtering is supported\0");
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        if interlace != 0 {
            set_error(b"interlacing is not supported\0");
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        let mut first = png.p;
        let palette = find(&mut png, b"PLTE".as_ptr(), 0);
        if palette.is_null() {
            png.p = first;
        } else {
            first = png.p;
        }
        let trns = find(&mut png, b"tRNS".as_ptr(), 0);
        if trns.is_null() {
            png.p = first;
        } else {
            first = png.p;
        }
        let mut data_len: c_int = 0;
        let mut idat = find(&mut png, b"IDAT".as_ptr(), 0);
        while !idat.is_null() {
            data_len = data_len.wrapping_add(chunk_byte_length(idat) as c_int);
            idat = chunk(&mut png, b"IDAT".as_ptr(), 0);
        }
        png.p = first;
        let data = malloc(data_len as usize).cast::<u8>();
        let mut offset: c_int = 0;
        idat = find(&mut png, b"IDAT".as_ptr(), 0);
        while !idat.is_null() {
            let len = chunk_byte_length(idat);
            memcpy(data.add(offset as usize).cast(), idat.cast(), len as usize);
            offset = offset.wrapping_add(len as c_int);
            idat = chunk(&mut png, b"IDAT".as_ptr(), 0);
        }
        if data.is_null() || data_len < 6 {
            set_error(b"corrupt zlib structure in DEFLATE stream\0");
            free(data.cast());
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        if (*data & 0x0f) != 0x08 {
            set_error(b"only zlib compression method (RFC 1950) is supported\0");
            free(data.cast());
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        if (*data & 0xf0) > 0x70 {
            set_error(b"innapropriate window size detected\0");
            free(data.cast());
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        if (*data.add(1) & 0x20) != 0 {
            set_error(b"preset dictionary is present and not supported\0");
            free(data.cast());
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        if out_size(&img, 4) < 1 {
            set_error(b"invalid image size found\0");
            free(data.cast());
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        if out_size(&img, bpp) < 1 {
            set_error(b"invalid image size found\0");
            free(data.cast());
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        let out = img
            .pix
            .cast::<u8>()
            .add((out_size(&img, 4) - out_size(&img, bpp)) as usize);
        if cp_inflate(data.add(2).cast(), data_len - 6, out.cast(), pix_bytes) == 0 {
            set_error(b"DEFLATE algorithm failed\0");
            free(data.cast());
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        if unfilter(img.w, img.h, bpp, out) == 0 {
            set_error(b"invalid filter byte found\0");
            free(data.cast());
            free(img.pix.cast());
            img.pix = ptr::null_mut();
            return img;
        }
        if color_type == 3 {
            if palette.is_null() {
                set_error(b"color type of indexed requires a PLTE chunk\0");
                free(data.cast());
                free(img.pix.cast());
                img.pix = ptr::null_mut();
                return img;
            }
            let trns_len = if trns.is_null() {
                0
            } else {
                chunk_byte_length(trns)
            };
            depalette(img.w, img.h, out, img.pix, palette, trns, trns_len);
        } else {
            convert(bpp, img.w, img.h, out, img.pix);
        }
        free(data.cast());
        img
    }
}
