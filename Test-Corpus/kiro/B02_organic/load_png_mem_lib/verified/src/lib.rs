use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;
use std::ptr;

#[repr(C)]
#[derive(Copy, Clone)]
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
pub static mut cp_error_reason: *const c_char = ptr::null();

#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static cp_fixed_table: [u8; 320] = [
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,9,9,9,9,9,9,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
    9,9,9,9,9,9,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
    7,7,7,7,7,8,8,8,8,8,8,8,8,5,5,5,5,5,5,5,5,5,5,5,5,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
];

#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static cp_permutation_order: [u8; 19] = [16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15];

#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static cp_len_extra_bits: [u8; 31] = [
    0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0,0,0
];

#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static cp_len_base: [u32; 31] = [
    3,4,5,6,7,8,9,10,11,13,15,17,19,23,27,31,
    35,43,51,59,67,83,99,115,131,163,195,227,258,0,0
];

#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static cp_dist_extra_bits: [u8; 32] = [
    0,0,0,0,1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10,11,11,12,12,13,13,0,0
];

#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static cp_dist_base: [u32; 32] = [
    1,2,3,4,5,7,9,13,17,25,33,49,65,97,129,193,
    257,385,513,769,1025,1537,2049,3073,4097,6145,8193,12289,16385,24577,0,0
];

struct CpState {
    bits: u64,
    count: i32,
    words: *mut u32,
    word_count: i32,
    word_index: i32,
    bits_left: i32,
    final_word_available: i32,
    final_word: u32,
    out: *mut u8,
    out_end: *mut u8,
    begin: *mut u8,
    lookup: [u16; 512],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

unsafe fn cp_ptr(s: *mut CpState) -> *mut u8 {
    let base = ((*s).words as *mut u8).add(((*s).word_index as usize) * 4);
    base.sub(((*s).count / 8) as usize)
}

unsafe fn cp_peak_bits(s: *mut CpState, num_bits_to_read: i32) {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = *(*s).words.add((*s).word_index as usize);
            (*s).word_index += 1;
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count += 32;
        } else if (*s).final_word_available != 0 {
            let word = (*s).final_word;
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count += (*s).bits_left;
            (*s).final_word_available = 0;
        }
    }
}

unsafe fn cp_consume_bits(s: *mut CpState, num_bits_to_read: i32) -> u32 {
    let bits = (*s).bits & (((1u64) << num_bits_to_read) - 1);
    (*s).bits >>= num_bits_to_read;
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits as u32
}

unsafe fn cp_read_bits(s: *mut CpState, num_bits_to_read: i32) -> u32 {
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(a: u32) -> u32 {
    let a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    let a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    let a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8)
}

unsafe fn cp_build(s: *mut CpState, tree: *mut u32, lens: *const u8, sym_count: i32) -> i32 {
    let mut counts = [0i32; 16];
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    for n in 0..sym_count {
        counts[*lens.add(n as usize) as usize] += 1;
    }
    counts[0] = 0; codes[0] = 0; first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if !s.is_null() {
        ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, 512);
    }
    for i in 0..sym_count {
        let l = *lens.add(i as usize) as usize;
        if l != 0 {
            let code = codes[l] as u32;
            codes[l] += 1;
            let slot = first[l] as usize;
            first[l] += 1;
            *tree.add(slot) = (code << (32 - l)) | ((i as u32) << 4) | (l as u32);
            if !s.is_null() && l <= 9 {
                let mut j = (cp_rev16(code) >> (16 - l)) as usize;
                while j < 512 {
                    (*s).lookup[j] = ((l << 9) | i as usize) as u16;
                    j += 1 << l;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: *mut CpState) -> i32 {
    cp_read_bits(s, (*s).count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    if !((*s).bits_left / 8 <= len as i32) {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    let p = cp_ptr(s);
    ptr::copy_nonoverlapping(p, (*s).out, len as usize);
    (*s).out = (*s).out.add(len as usize);
    1
}

unsafe fn cp_fixed(s: *mut CpState) -> i32 {
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), cp_fixed_table.as_ptr(), 288) as u32;
    (*s).ndst = cp_build(ptr::null_mut(), (*s).dst.as_mut_ptr(), cp_fixed_table.as_ptr().add(288), 32) as u32;
    1
}

unsafe fn cp_decode(s: *mut CpState, tree: *mut u32, hi: i32) -> i32 {
    cp_peak_bits(s, 16);
    let bits = (*s).bits;
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0i32;
    let mut hi = hi;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.add(guess as usize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.add((lo - 1) as usize);
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

unsafe fn cp_dynamic(s: *mut CpState) -> i32 {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen {
        lenlens[cp_permutation_order[i as usize] as usize] = cp_read_bits(s, 3) as u8;
    }
    (*s).nlen = cp_build(ptr::null_mut(), (*s).len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;
    let mut lens = [0u8; 320];
    let mut n = 0i32;
    while n < nlit + ndst {
        let sym = cp_decode(s, (*s).len.as_mut_ptr(), (*s).nlen as i32);
        match sym {
            16 => {
                let count = 3 + cp_read_bits(s, 2) as i32;
                for _ in 0..count {
                    lens[n as usize] = lens[(n - 1) as usize];
                    n += 1;
                }
            }
            17 => {
                let count = 3 + cp_read_bits(s, 3) as i32;
                for _ in 0..count {
                    lens[n as usize] = 0;
                    n += 1;
                }
            }
            18 => {
                let count = 11 + cp_read_bits(s, 7) as i32;
                for _ in 0..count {
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
    (*s).nlit = cp_build(s, (*s).lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
    (*s).ndst = cp_build(ptr::null_mut(), (*s).dst.as_mut_ptr(), lens.as_ptr().add(nlit as usize), ndst) as u32;
    1
}

unsafe fn cp_block(s: *mut CpState) -> i32 {
    loop {
        let symbol = cp_decode(s, (*s).lit.as_mut_ptr(), (*s).nlit as i32);
        if symbol < 256 {
            if !((*s).out.add(1) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                return 0;
            }
            *(*s).out = symbol as u8;
            (*s).out = (*s).out.add(1);
        } else if symbol > 256 {
            let sym = (symbol - 257) as usize;
            let length = cp_read_bits(s, cp_len_extra_bits[sym] as i32) as i32 + cp_len_base[sym] as i32;
            let distance_symbol = cp_decode(s, (*s).dst.as_mut_ptr(), (*s).ndst as i32) as usize;
            let backwards_distance = cp_read_bits(s, cp_dist_extra_bits[distance_symbol] as i32) as i32 + cp_dist_base[distance_symbol] as i32;
            if !((*s).out.offset(-(backwards_distance as isize)) >= (*s).begin) {
                cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                return 0;
            }
            if !((*s).out.add(length as usize) <= (*s).out_end) {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                return 0;
            }
            let src = (*s).out.offset(-(backwards_distance as isize));
            let dst = (*s).out;
            (*s).out = (*s).out.add(length as usize);
            match backwards_distance {
                1 => {
                    ptr::write_bytes(dst, *src, length as usize);
                }
                _ => {
                    let mut s_ptr = src;
                    let mut d_ptr = dst;
                    for _ in 0..length {
                        *d_ptr = *s_ptr;
                        d_ptr = d_ptr.add(1);
                        s_ptr = s_ptr.add(1);
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
pub unsafe extern "C" fn cp_inflate(in_ptr: *mut c_void, in_bytes: c_int, out_ptr: *mut c_void, out_bytes: c_int) -> c_int {
    let layout = std::alloc::Layout::new::<CpState>();
    let s = std::alloc::alloc_zeroed(layout) as *mut CpState;
    if s.is_null() { return 0; }
    (*s).bits = 0;
    (*s).count = 0;
    (*s).word_index = 0;
    (*s).bits_left = in_bytes * 8;

    let in_p = in_ptr as *mut u8;
    let first_bytes = ((((in_p as usize) + 3) & !3) - (in_p as usize)) as i32;
    (*s).words = in_p.add(first_bytes as usize) as *mut u32;
    (*s).word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;

    for i in 0..first_bytes {
        (*s).bits |= (*in_p.add(i as usize) as u64) << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    (*s).final_word = 0;
    for i in 0..last_bytes {
        (*s).final_word |= (*in_p.add((in_bytes - last_bytes + i) as usize) as u32) << (i * 8);
    }
    (*s).count = first_bytes * 8;

    (*s).out = out_ptr as *mut u8;
    (*s).out_end = (out_ptr as *mut u8).add(out_bytes as usize);
    (*s).begin = out_ptr as *mut u8;

    loop {
        let bfinal = cp_read_bits(s, 1);
        let btype = cp_read_bits(s, 2);
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    std::alloc::dealloc(s as *mut u8, layout);
                    return 0;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    std::alloc::dealloc(s as *mut u8, layout);
                    return 0;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    std::alloc::dealloc(s as *mut u8, layout);
                    return 0;
                }
            }
            3 => {
                cp_error_reason = b"Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
                std::alloc::dealloc(s as *mut u8, layout);
                return 0;
            }
            _ => {}
        }
        if bfinal != 0 { break; }
    }
    std::alloc::dealloc(s as *mut u8, layout);
    1
}

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let pa = (p - a as i32).abs();
    let pb = (p - b as i32).abs();
    let pc = (p - c as i32).abs();
    if pa <= pb && pa <= pc { a } else if pb <= pc { b } else { c }
}

struct CpRawPng {
    p: *const u8,
    end: *const u8,
}

unsafe fn cp_make32(s: *const u8) -> u32 {
    ((*s.add(0) as u32) << 24) | ((*s.add(1) as u32) << 16) | ((*s.add(2) as u32) << 8) | (*s.add(3) as u32)
}

unsafe fn cp_chunk(png: *mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if libc::memcmp(start.add(4) as *const c_void, chunk.as_ptr() as *const c_void, 4) == 0 && len >= minlen {
        let offset = len as isize + 12;
        if (*png).p.offset(offset) <= (*png).end {
            (*png).p = (*png).p.offset(offset);
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_find(png: *mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    while (*png).p < (*png).end {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        (*png).p = (*png).p.add(len as usize + 12);
        if libc::memcmp(start.add(4) as *const c_void, chunk.as_ptr() as *const c_void, 4) == 0 && len >= minlen && (*png).p <= (*png).end {
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_unfilter(w: i32, h: i32, bpp: i32, raw: *mut u8) -> i32 {
    let len = w * bpp;
    let mut raw = raw;
    if h > 0 {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                for x in bpp..len {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                }
            }
            2 => {}
            3 => {
                for x in bpp..len {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add((*raw.add((x - bpp) as usize)) / 2);
                }
            }
            4 => {
                for x in bpp..len {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(cp_paeth(*raw.add((x - bpp) as usize), 0, 0));
                }
            }
            _ => { return 0; }
        }
    }
    let mut prev = raw;
    raw = raw.add(len as usize);
    for _y in 1..h {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                for x in 0..bpp {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(0);
                }
                for x in bpp..len {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*raw.add((x - bpp) as usize));
                }
            }
            2 => {
                for x in 0..len {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                }
            }
            3 => {
                for x in 0..bpp {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add((*prev.add(x as usize)) / 2);
                }
                for x in bpp..len {
                    let v = ((*raw.add((x - bpp) as usize) as u32) + (*prev.add(x as usize) as u32)) / 2;
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(v as u8);
                }
            }
            4 => {
                for x in 0..bpp {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize));
                }
                for x in bpp..len {
                    *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(cp_paeth(*raw.add((x - bpp) as usize), *prev.add(x as usize), *prev.add((x - bpp) as usize)));
                }
            }
            _ => { return 0; }
        }
        prev = raw;
        raw = raw.add(len as usize);
    }
    1
}

unsafe fn cp_convert(bpp: i32, w: i32, h: i32, mut src: *mut u8, mut dst: *mut cp_pixel_t) {
    for _y in 0..h {
        src = src.add(1);
        for _x in 0..w {
            match bpp {
                1 => {
                    *dst = cp_pixel_t { r: *src, g: *src, b: *src, a: 0xFF };
                }
                2 => {
                    *dst = cp_pixel_t { r: *src, g: *src, b: *src, a: *src.add(1) };
                }
                3 => {
                    *dst = cp_pixel_t { r: *src, g: *src.add(1), b: *src.add(2), a: 0xFF };
                }
                4 => {
                    *dst = cp_pixel_t { r: *src, g: *src.add(1), b: *src.add(2), a: *src.add(3) };
                }
                _ => {}
            }
            src = src.add(bpp as usize);
            dst = dst.add(1);
        }
    }
}

unsafe fn cp_depalette(w: i32, h: i32, mut src: *mut u8, mut dst: *mut cp_pixel_t, plte: *const u8, trns: *const u8, trns_len: u32) {
    for _y in 0..h {
        src = src.add(1);
        for _x in 0..w {
            let c = *src as usize;
            let r = *plte.add(c * 3);
            let g = *plte.add(c * 3 + 1);
            let b = *plte.add(c * 3 + 2);
            let a = if trns.is_null() {
                255
            } else if c as u32 >= trns_len {
                255
            } else {
                *trns.add(c)
            };
            *dst = cp_pixel_t { r, g, b, a };
            src = src.add(1);
            dst = dst.add(1);
        }
    }
}

unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    cp_make32(chunk.sub(8))
}

fn cp_out_size(w: i32, h: i32, bpp: i32) -> i32 {
    (w + 1) * h * bpp
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    let sig: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    let mut img = cp_image_t { w: 0, h: 0, pix: ptr::null_mut() };
    #[allow(unused_assignments)]
    let mut data: *mut u8 = ptr::null_mut();

    let mut png = CpRawPng { p: png_data, end: png_data.add(png_length as usize) };

    // Check signature
    if libc::memcmp(png.p as *const c_void, sig.as_ptr() as *const c_void, 8) != 0 {
        cp_error_reason = b"incorrect file signature (is this a png file?)\0".as_ptr() as *const c_char;
        return img;
    }
    png.p = png.p.add(8);

    // IHDR
    let ihdr = cp_chunk(&mut png, b"IHDR", 13);
    if ihdr.is_null() {
        cp_error_reason = b"unable to find IHDR chunk\0".as_ptr() as *const c_char;
        return img;
    }

    let bit_depth = *ihdr.add(8);
    let color_type = *ihdr.add(9);
    if bit_depth != 8 {
        cp_error_reason = b"only bit-depth of 8 is supported\0".as_ptr() as *const c_char;
        return img;
    }

    let bpp: i32 = match color_type {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => {
            cp_error_reason = b"unknown color type\0".as_ptr() as *const c_char;
            return img;
        }
    };

    // BUG PRESERVED: w = cp_make32(ihdr) + 1, then img.w = w - 1
    let w = cp_make32(ihdr) as i32 + 1;
    let h = cp_make32(ihdr.add(4)) as i32;

    if !(w >= 1) {
        cp_error_reason = b"invalid IHDR chunk found, image width was less than 1\0".as_ptr() as *const c_char;
        return img;
    }
    if !(h >= 1) {
        cp_error_reason = b"invalid IHDR chunk found, image height was less than 1\0".as_ptr() as *const c_char;
        return img;
    }
    if !((w as i64 * h as i64 * std::mem::size_of::<cp_pixel_t>() as i64) < i32::MAX as i64) {
        cp_error_reason = b"image too large\0".as_ptr() as *const c_char;
        return img;
    }

    let pix_bytes = (w * h * std::mem::size_of::<cp_pixel_t>() as i32) as usize;
    img.w = w - 1;
    img.h = h;
    img.pix = libc::malloc(pix_bytes) as *mut cp_pixel_t;
    if img.pix.is_null() {
        cp_error_reason = b"unable to allocate raw image space\0".as_ptr() as *const c_char;
        return img;
    }

    let compression = *ihdr.add(10);
    let filter = *ihdr.add(11);
    let interlace = *ihdr.add(12);
    if compression != 0 {
        cp_error_reason = b"only standard compression DEFLATE is supported\0".as_ptr() as *const c_char;
        libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }
    if filter != 0 {
        cp_error_reason = b"only standard adaptive filtering is supported\0".as_ptr() as *const c_char;
        libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }
    if interlace != 0 {
        cp_error_reason = b"interlacing is not supported\0".as_ptr() as *const c_char;
        libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }

    // Find PLTE, tRNS
    let first = png.p;
    let plte = cp_find(&mut png, b"PLTE", 0);
    if plte.is_null() { png.p = first; }
    let first = png.p;
    let trns = cp_find(&mut png, b"tRNS", 0);
    if trns.is_null() { png.p = first; }
    let first = png.p;

    // Calculate total IDAT length
    let mut datalen: i32 = 0;
    {
        let mut idat = cp_find(&mut png, b"IDAT", 0);
        while !idat.is_null() {
            datalen += cp_get_chunk_byte_length(idat) as i32;
            idat = cp_chunk(&mut png, b"IDAT", 0);
        }
    }

    // Collect IDAT data
    png.p = first;
    data = libc::malloc(datalen as usize) as *mut u8;
    let mut offset: i32 = 0;
    {
        let mut idat = cp_find(&mut png, b"IDAT", 0);
        while !idat.is_null() {
            let len = cp_get_chunk_byte_length(idat);
            ptr::copy_nonoverlapping(idat, data.add(offset as usize), len as usize);
            offset += len as i32;
            idat = cp_chunk(&mut png, b"IDAT", 0);
        }
    }

    // Macro-style error helper - we use goto-like pattern with a closure
    // Validate zlib
    if data.is_null() || datalen < 6 {
        cp_error_reason = b"corrupt zlib structure in DEFLATE stream\0".as_ptr() as *const c_char;
        libc::free(data as *mut c_void); libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }
    if (*data & 0x0f) != 0x08 {
        cp_error_reason = b"only zlib compression method (RFC 1950) is supported\0".as_ptr() as *const c_char;
        libc::free(data as *mut c_void); libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }
    if (*data & 0xf0) > 0x70 {
        cp_error_reason = b"innapropriate window size detected\0".as_ptr() as *const c_char;
        libc::free(data as *mut c_void); libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }
    if (*data.add(1) & 0x20) != 0 {
        cp_error_reason = b"preset dictionary is present and not supported\0".as_ptr() as *const c_char;
        libc::free(data as *mut c_void); libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }
    if !(cp_out_size(img.w, img.h, 4) >= 1) {
        cp_error_reason = b"invalid image size found\0".as_ptr() as *const c_char;
        libc::free(data as *mut c_void); libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }
    if !(cp_out_size(img.w, img.h, bpp) >= 1) {
        cp_error_reason = b"invalid image size found\0".as_ptr() as *const c_char;
        libc::free(data as *mut c_void); libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }

    let out = (img.pix as *mut u8).add((cp_out_size(img.w, img.h, 4) - cp_out_size(img.w, img.h, bpp)) as usize);

    if cp_inflate(data.add(2) as *mut c_void, datalen - 6, out as *mut c_void, pix_bytes as c_int) == 0 {
        cp_error_reason = b"DEFLATE algorithm failed\0".as_ptr() as *const c_char;
        libc::free(data as *mut c_void); libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }
    if cp_unfilter(img.w, img.h, bpp, out) == 0 {
        cp_error_reason = b"invalid filter byte found\0".as_ptr() as *const c_char;
        libc::free(data as *mut c_void); libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
    }

    if color_type == 3 {
        if plte.is_null() {
            cp_error_reason = b"color type of indexed requires a PLTE chunk\0".as_ptr() as *const c_char;
            libc::free(data as *mut c_void); libc::free(img.pix as *mut c_void); img.pix = ptr::null_mut(); return img;
        }
        let trns_len = if trns.is_null() { 0 } else { cp_get_chunk_byte_length(trns) };
        cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
    } else {
        cp_convert(bpp, img.w, img.h, out, img.pix);
    }

    libc::free(data as *mut c_void);
    img
}
