use std::os::raw::{c_char, c_int, c_void};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = std::ptr::null();

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
pub static mut cp_permutation_order: [u8; 19] = [16, 17, 18, 0, 8,  7, 9,  6, 10, 5,
                                    11, 4,  12, 3, 13, 2, 14, 1, 15];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 29 + 2] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1,
                                     1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4,
                                     4, 4, 5, 5, 5, 5, 0, 0, 0];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 29 + 2] = [
    3,  4,  5,  6,  7,  8,  9,  10,  11,  13,  15,  17,  19,  23, 27, 31,
    35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 0,  0];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 30 + 2] = [0,  0,  0,  0,  1,  1,  2,  2,  3, 3, 4,
                                      4,  5,  5,  6,  6,  7,  7,  8,  8, 9, 9,
                                      10, 10, 11, 11, 12, 12, 13, 13, 0, 0];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 30 + 2] = [
    1,    2,    3,    4,    5,    7,     9,     13,    17,  25,   33,
    49,   65,   97,   129,  193,  257,   385,   513,   769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0,   0];

#[repr(C)]
struct cp_state_t {
    bits: u64,
    count: i32,
    words: *const u32,
    word_count: i32,
    word_index: i32,
    bits_left: i32,
    final_word_available: i32,
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

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

fn cp_would_overflow(s: &cp_state_t, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

unsafe fn cp_ptr(s: &cp_state_t) -> *mut u8 {
    assert!((s.bits_left & 7) == 0);
    (s.words.add(s.word_index as usize) as *mut u8).sub((s.count / 8) as usize)
}

unsafe fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = *s.words.add(s.word_index as usize);
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

unsafe fn cp_consume_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u32 {
    assert!(s.count >= num_bits_to_read);
    let bits = (s.bits & ((1u64 << num_bits_to_read) - 1)) as u32;
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: i32) -> u32 {
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

unsafe fn cp_build(s: *mut cp_state_t, tree: *mut u32, lens: *const u8, sym_count: i32) -> i32 {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
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
        (*s).lookup.fill(0);
    }
    for i in 0..sym_count {
        let len = *lens.add(i as usize) as i32;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len as usize];
            codes[len as usize] += 1;
            let slot = first[len as usize];
            first[len as usize] += 1;
            *tree.add(slot as usize) = ((code as u32) << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if !s.is_null() && len <= 9 {
                let mut j = cp_rev16(code as u32) >> (16 - len);
                while j < (1 << 9) {
                    (*s).lookup[j as usize] = ((len << 9) | i) as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut cp_state_t) -> i32 {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    if s.bits_left / 8 <= len as i32 {
        cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        return 0;
    }
    let p = cp_ptr(s);
    std::ptr::copy_nonoverlapping(p, s.out, len as usize);
    s.out = s.out.add(len as usize);
    1
}

unsafe fn cp_fixed(s: &mut cp_state_t) -> i32 {
    s.nlit = cp_build(s, s.lit.as_mut_ptr(), cp_fixed_table.as_ptr(), 288) as u32;
    s.ndst = cp_build(std::ptr::null_mut(), s.dst.as_mut_ptr(), cp_fixed_table.as_ptr().add(288), 32) as u32;
    1
}

unsafe fn cp_decode(s: &mut cp_state_t, tree: *const u32, mut hi: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
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
    let len = 32 - (key & 0xF);
    assert!((search >> len) == (key >> len));
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

unsafe fn cp_dynamic(s: &mut cp_state_t) -> i32 {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen {
        lenlens[cp_permutation_order[i as usize] as usize] = cp_read_bits(s, 3) as u8;
    }
    s.nlen = cp_build(std::ptr::null_mut(), s.len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;
    let mut lens = [0u8; 288 + 32];
    let mut n = 0;
    while n < nlit + ndst {
        let sym = cp_decode(s, s.len.as_ptr(), s.nlen as i32);
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as i32;
                while i > 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as i32;
                while i > 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as i32;
                while i > 0 {
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
    s.nlit = cp_build(s, s.lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
    s.ndst = cp_build(std::ptr::null_mut(), s.dst.as_mut_ptr(), lens.as_ptr().add(nlit as usize), ndst) as u32;
    1
}

unsafe fn cp_block(s: &mut cp_state_t) -> i32 {
    loop {
        let mut symbol = cp_decode(s, s.lit.as_ptr(), s.nlit as i32);
        if symbol < 256 {
            if s.out.add(1) > s.out_end {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                return 0;
            }
            *s.out = symbol as u8;
            s.out = s.out.add(1);
        } else if symbol > 256 {
            symbol -= 257;
            let mut length = cp_read_bits(s, cp_len_extra_bits[symbol as usize] as i32) as i32 + cp_len_base[symbol as usize] as i32;
            let distance_symbol = cp_decode(s, s.dst.as_ptr(), s.ndst as i32);
            let backwards_distance = cp_read_bits(s, cp_dist_extra_bits[distance_symbol as usize] as i32) as i32 + cp_dist_base[distance_symbol as usize] as i32;
            if s.out.sub(backwards_distance as usize) < s.begin {
                cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                return 0;
            }
            if s.out.add(length as usize) > s.out_end {
                cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                return 0;
            }
            let mut src = s.out.sub(backwards_distance as usize);
            let mut dst = s.out;
            s.out = s.out.add(length as usize);
            if backwards_distance == 1 {
                std::ptr::write_bytes(dst, *src, length as usize);
            } else {
                while length > 0 {
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

unsafe fn cp_inflate(in_data: *mut c_void, in_bytes: i32, out: *mut c_void, out_bytes: i32) -> i32 {
    let s_ptr = libc::calloc(1, std::mem::size_of::<cp_state_t>()) as *mut cp_state_t;
    if s_ptr.is_null() { return 0; }
    let s = &mut *s_ptr;
    
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;
    
    let in_ptr = in_data as *const u8;
    let in_addr = in_ptr as usize;
    let first_bytes = (((in_addr + 3) & !3) - in_addr) as i32;
    s.words = in_ptr.add(first_bytes as usize) as *const u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    
    for i in 0..first_bytes {
        s.bits |= (*in_ptr.add(i as usize) as u64) << (i * 8);
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        s.final_word |= (*in_ptr.add((in_bytes - last_bytes + i) as usize) as u32) << (i * 8);
    }
    s.count = first_bytes * 8;
    s.out = out as *mut u8;
    s.out_end = (out as *mut u8).add(out_bytes as usize);
    s.begin = out as *mut u8;
    
    let mut bfinal = 0;
    while bfinal == 0 {
        bfinal = cp_read_bits(s, 1);
        let btype = cp_read_bits(s, 2);
        match btype {
            0 => {
                if cp_stored(s) == 0 {
                    libc::free(s_ptr as *mut c_void);
                    return 0;
                }
            }
            1 => {
                cp_fixed(s);
                if cp_block(s) == 0 {
                    libc::free(s_ptr as *mut c_void);
                    return 0;
                }
            }
            2 => {
                cp_dynamic(s);
                if cp_block(s) == 0 {
                    libc::free(s_ptr as *mut c_void);
                    return 0;
                }
            }
            3 => {
                cp_error_reason = b"Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
                libc::free(s_ptr as *mut c_void);
                return 0;
            }
            _ => {}
        }
    }
    libc::free(s_ptr as *mut c_void);
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

struct cp_raw_png_t {
    p: *const u8,
    end: *const u8,
}

unsafe fn cp_make32(s: *const u8) -> u32 {
    ((*s.add(0) as u32) << 24) | ((*s.add(1) as u32) << 16) | ((*s.add(2) as u32) << 8) | (*s.add(3) as u32)
}

unsafe fn cp_chunk(png: &mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    let len = cp_make32(png.p);
    let start = png.p;
    if std::slice::from_raw_parts(start.add(4), 4) == chunk && len >= minlen {
        let offset = len + 12;
        if png.p.add(offset as usize) <= png.end {
            png.p = png.p.add(offset as usize);
            return start.add(8);
        }
    }
    std::ptr::null()
}

unsafe fn cp_find(png: &mut cp_raw_png_t, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    while png.p < png.end {
        let len = cp_make32(png.p);
        let start = png.p;
        png.p = png.p.add((len + 12) as usize);
        if std::slice::from_raw_parts(start.add(4), 4) == chunk && len >= minlen && png.p <= png.end {
            return start.add(8);
        }
    }
    std::ptr::null()
}

unsafe fn cp_unfilter(w: i32, h: i32, bpp: i32, mut raw: *mut u8) -> i32 {
    let len = w * bpp;
    let mut prev;
    if h > 0 {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                for x in bpp..len {
                    *raw.add(x as usize) = raw.add(x as usize).read().wrapping_add(raw.add((x - bpp) as usize).read());
                }
            }
            2 => {}
            3 => {
                for x in bpp..len {
                    *raw.add(x as usize) = raw.add(x as usize).read().wrapping_add(raw.add((x - bpp) as usize).read() / 2);
                }
            }
            4 => {
                for x in bpp..len {
                    *raw.add(x as usize) = raw.add(x as usize).read().wrapping_add(cp_paeth(raw.add((x - bpp) as usize).read(), 0, 0));
                }
            }
            _ => return 0,
        }
    }
    prev = raw;
    raw = raw.add(len as usize);
    for _y in 1..h {
        let filter = *raw;
        raw = raw.add(1);
        match filter {
            0 => {}
            1 => {
                for x in bpp..len {
                    *raw.add(x as usize) = raw.add(x as usize).read().wrapping_add(raw.add((x - bpp) as usize).read());
                }
            }
            2 => {
                for x in 0..len {
                    *raw.add(x as usize) = raw.add(x as usize).read().wrapping_add(prev.add(x as usize).read());
                }
            }
            3 => {
                for x in 0..bpp {
                    *raw.add(x as usize) = raw.add(x as usize).read().wrapping_add(prev.add(x as usize).read() / 2);
                }
                for x in bpp..len {
                    *raw.add(x as usize) = raw.add(x as usize).read().wrapping_add(((raw.add((x - bpp) as usize).read() as u16 + prev.add(x as usize).read() as u16) / 2) as u8);
                }
            }
            4 => {
                for x in 0..bpp {
                    *raw.add(x as usize) = raw.add(x as usize).read().wrapping_add(prev.add(x as usize).read());
                }
                for x in bpp..len {
                    *raw.add(x as usize) = raw.add(x as usize).read().wrapping_add(cp_paeth(raw.add((x - bpp) as usize).read(), prev.add(x as usize).read(), prev.add((x - bpp) as usize).read()));
                }
            }
            _ => return 0,
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
                    *dst = cp_make_pixel(*src.add(0), *src.add(0), *src.add(0));
                    dst = dst.add(1);
                }
                2 => {
                    *dst = cp_make_pixel_a(*src.add(0), *src.add(0), *src.add(0), *src.add(1));
                    dst = dst.add(1);
                }
                3 => {
                    *dst = cp_make_pixel(*src.add(0), *src.add(1), *src.add(2));
                    dst = dst.add(1);
                }
                4 => {
                    *dst = cp_make_pixel_a(*src.add(0), *src.add(1), *src.add(2), *src.add(3));
                    dst = dst.add(1);
                }
                _ => {}
            }
            src = src.add(bpp as usize);
        }
    }
}

unsafe fn cp_get_alpha_for_indexed_image(index: i32, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() {
        255
    } else if (index as u32) >= trns_len {
        255
    } else {
        *trns.add(index as usize)
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
            let a = cp_get_alpha_for_indexed_image(c as i32, trns, trns_len);
            *dst = cp_make_pixel_a(r, g, b, a);
            dst = dst.add(1);
            src = src.add(1);
        }
    }
}

unsafe fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    cp_make32(chunk.sub(8))
}

fn cp_out_size(img: &cp_image_t, bpp: i32) -> i32 {
    (img.w + 1) * img.h * bpp
}

#[unsafe(no_mangle)]
pub extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    unsafe {
        let sig = b"\x89PNG\r\n\x1a\n";
        let mut img = cp_image_t { w: 0, h: 0, pix: std::ptr::null_mut() };
        let mut data: *mut u8 = std::ptr::null_mut();
        
        let mut png = cp_raw_png_t {
            p: png_data,
            end: png_data.add(png_length as usize),
        };
        
        if std::slice::from_raw_parts(png.p, 8) != sig {
            cp_error_reason = b"incorrect file signature (is this a png file?)\0".as_ptr() as *const c_char;
            return img;
        }
        png.p = png.p.add(8);
        
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
        
        let bpp = match color_type {
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
        
        let w = cp_make32(ihdr) as i32 + 1;
        let h = cp_make32(ihdr.add(4)) as i32;
        
        if w < 1 {
            cp_error_reason = b"invalid IHDR chunk found, image width was less than 1\0".as_ptr() as *const c_char;
            return img;
        }
        if h < 1 {
            cp_error_reason = b"invalid IHDR chunk found, image height was less than 1\0".as_ptr() as *const c_char;
            return img;
        }
        if (w as i64) * (h as i64) * (std::mem::size_of::<cp_pixel_t>() as i64) >= i32::MAX as i64 {
            cp_error_reason = b"image too large\0".as_ptr() as *const c_char;
            return img;
        }
        
        let pix_bytes = w * h * std::mem::size_of::<cp_pixel_t>() as i32;
        img.w = w - 1;
        img.h = h;
        
        img.pix = libc::malloc(pix_bytes as usize) as *mut cp_pixel_t;
        if img.pix.is_null() {
            cp_error_reason = b"unable to allocate raw image space\0".as_ptr() as *const c_char;
            return img;
        }
        
        let compression = *ihdr.add(10);
        let filter = *ihdr.add(11);
        let interlace = *ihdr.add(12);
        
        if compression != 0 {
            cp_error_reason = b"only standard compression DEFLATE is supported\0".as_ptr() as *const c_char;
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        if filter != 0 {
            cp_error_reason = b"only standard adaptive filtering is supported\0".as_ptr() as *const c_char;
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        if interlace != 0 {
            cp_error_reason = b"interlacing is not supported\0".as_ptr() as *const c_char;
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        
        let mut first = png.p;
        let plte = cp_find(&mut png, b"PLTE", 0);
        if plte.is_null() {
            png.p = first;
        } else {
            first = png.p;
        }
        
        let trns = cp_find(&mut png, b"tRNS", 0);
        if trns.is_null() {
            png.p = first;
        } else {
            first = png.p;
        }
        
        let mut datalen = 0;
        let mut idat = cp_find(&mut png, b"IDAT", 0);
        while !idat.is_null() {
            let len = cp_get_chunk_byte_length(idat);
            datalen += len;
            idat = cp_chunk(&mut png, b"IDAT", 0);
        }
        
        png.p = first;
        data = libc::malloc(datalen as usize) as *mut u8;
        
        let mut offset = 0;
        idat = cp_find(&mut png, b"IDAT", 0);
        while !idat.is_null() {
            let len = cp_get_chunk_byte_length(idat);
            std::ptr::copy_nonoverlapping(idat, data.add(offset as usize), len as usize);
            offset += len;
            idat = cp_chunk(&mut png, b"IDAT", 0);
        }
        
        if data.is_null() || datalen < 6 {
            cp_error_reason = b"corrupt zlib structure in DEFLATE stream\0".as_ptr() as *const c_char;
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        
        if (*data.add(0) & 0x0f) != 0x08 {
            cp_error_reason = b"only zlib compression method (RFC 1950) is supported\0".as_ptr() as *const c_char;
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        if (*data.add(0) & 0xf0) > 0x70 {
            cp_error_reason = b"innapropriate window size detected\0".as_ptr() as *const c_char;
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        if (*data.add(1) & 0x20) != 0 {
            cp_error_reason = b"preset dictionary is present and not supported\0".as_ptr() as *const c_char;
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        
        if cp_out_size(&img, 4) < 1 {
            cp_error_reason = b"invalid image size found\0".as_ptr() as *const c_char;
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        if cp_out_size(&img, bpp) < 1 {
            cp_error_reason = b"invalid image size found\0".as_ptr() as *const c_char;
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        
        let out = (img.pix as *mut u8).add(cp_out_size(&img, 4) as usize).sub(cp_out_size(&img, bpp) as usize);
        
        if cp_inflate(data.add(2) as *mut c_void, (datalen - 6) as i32, out as *mut c_void, pix_bytes) == 0 {
            cp_error_reason = b"DEFLATE algorithm failed\0".as_ptr() as *const c_char;
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        
        if cp_unfilter(img.w, img.h, bpp, out) == 0 {
            cp_error_reason = b"invalid filter byte found\0".as_ptr() as *const c_char;
            libc::free(data as *mut c_void);
            libc::free(img.pix as *mut c_void);
            img.pix = std::ptr::null_mut();
            return img;
        }
        
        if color_type == 3 {
            if plte.is_null() {
                cp_error_reason = b"color type of indexed requires a PLTE chunk\0".as_ptr() as *const c_char;
                libc::free(data as *mut c_void);
                libc::free(img.pix as *mut c_void);
                img.pix = std::ptr::null_mut();
                return img;
            }
            let trns_len = if !trns.is_null() { cp_get_chunk_byte_length(trns) } else { 0 };
            cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
        } else {
            cp_convert(bpp, img.w, img.h, out, img.pix);
        }
        
        libc::free(data as *mut c_void);
        img
    }
}
