use std::ffi::{c_char, c_int, c_void};
use std::os::raw::{c_uint, c_ulonglong};
use std::ptr;
use std::alloc::{alloc, dealloc, Layout};
use std::mem::size_of;

#[repr(C)]
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

static mut cp_error_reason: *const c_char = ptr::null();

static cp_fixed_table: [u8; 320] = [
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

static cp_permutation_order: [u8; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

static cp_len_extra_bits: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

static cp_len_base: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

static cp_dist_extra_bits: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 0, 0,
];

static cp_dist_base: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

struct cp_state_t {
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
    lookup: [u16; 512],
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

fn cp_would_overflow(s: &cp_state_t, num_bits: c_int) -> c_int {
    ((s.bits_left + s.count) - num_bits < 0) as c_int
}

fn cp_ptr(s: &cp_state_t) -> *mut u8 {
    (s.words as usize + (s.word_index as usize * size_of::<u32>()) - ((s.count / 8) as usize)) as *mut u8
}

fn cp_peak_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { *s.words.add(s.word_index as usize) };
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            s.word_index += 1;
        } else if s.final_word_available != 0 {
            let word = s.final_word;
            s.bits |= (word as u64) << s.count;
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

fn cp_consume_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
    let bits = s.bits & (((1u64) << num_bits_to_read) - 1);
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits as u32
}

fn cp_read_bits(s: &mut cp_state_t, num_bits_to_read: c_int) -> u32 {
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

fn cp_build(s: Option<&mut cp_state_t>, tree: &mut [u32], lens: &[u8], sym_count: c_int) -> c_int {
    let mut codes: [c_int; 16] = [0; 16];
    let mut first: [c_int; 16] = [0; 16];
    let mut counts: [c_int; 16] = [0; 16];
    
    for n in 0..sym_count {
        counts[lens[n as usize] as usize] += 1;
    }
    
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    
    if let Some(st) = s {
        st.lookup.fill(0);
    }
    
    for i in 0..sym_count {
        let len = lens[i as usize] as usize;
        if len != 0 {
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(st) = s {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < 512 {
                        st.lookup[j] = ((len << 9) | (i as usize)) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    }
    
    first[15]
}

fn cp_stored(s: &mut cp_state_t) -> c_int {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    
    if len != !nlen {
        unsafe {
            cp_error_reason = "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        }
        return 0;
    }
    
    if s.bits_left / 8 > len as c_int {
        unsafe {
            cp_error_reason = "Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        }
        return 0;
    }
    
    let p = cp_ptr(s);
    unsafe {
        std::ptr::copy_nonoverlapping(p, s.out, len as usize);
        s.out = s.out.add(len as usize);
    }
    1
}

fn cp_fixed(s: &mut cp_state_t) -> c_int {
    s.nlit = cp_build(Some(s), &mut s.lit, &cp_fixed_table[..288], 288) as u32;
    s.ndst = cp_build(None, &mut s.dst, &cp_fixed_table[288..], 32) as u32;
    1
}

fn cp_decode(s: &mut cp_state_t, tree: &[u32], hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0;
    let mut hi = hi;
    
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
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

fn cp_dynamic(s: &mut cp_state_t) -> c_int {
    let mut lenlens: [u8; 19] = [0; 19];
    let nlit = 257 + cp_read_bits(s, 5) as c_int;
    let ndst = 1 + cp_read_bits(s, 5) as c_int;
    let nlen = 4 + cp_read_bits(s, 4) as c_int;
    
    for i in 0..nlen {
        lenlens[cp_permutation_order[i as usize] as usize] = cp_read_bits(s, 3) as u8;
    }
    
    s.nlen = cp_build(None, &mut s.len, &lenlens, 19) as u32;
    
    let mut lens: [u8; 320] = [0; 320];
    let mut n: usize = 0;
    
    while (n as c_int) < nlit + ndst {
        let sym = cp_decode(s, &s.len, s.nlen as c_int);
        match sym {
            16 => {
                let count = 3 + cp_read_bits(s, 2);
                for _ in 0..count {
                    lens[n] = lens[n - 1];
                    n += 1;
                }
            }
            17 => {
                let count = 3 + cp_read_bits(s, 3);
                for _ in 0..count {
                    lens[n] = 0;
                    n += 1;
                }
            }
            18 => {
                let count = 11 + cp_read_bits(s, 7);
                for _ in 0..count {
                    lens[n] = 0;
                    n += 1;
                }
            }
            _ => {
                lens[n] = sym as u8;
                n += 1;
            }
        }
    }
    
    s.nlit = cp_build(Some(s), &mut s.lit, &lens[..nlit as usize], nlit) as u32;
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit as usize..(nlit + ndst) as usize], ndst) as u32;
    1
}

fn cp_block(s: &mut cp_state_t) -> c_int {
    loop {
        let symbol = cp_decode(s, &s.lit, s.nlit as c_int);
        if symbol < 256 {
            unsafe {
                if s.out.add(1) > s.out_end {
                    cp_error_reason = "Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                    return 0;
                }
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let sym = symbol - 257;
            let length = cp_read_bits(s, cp_len_extra_bits[sym as usize] as c_int) + cp_len_base[sym as usize];
            let distance_symbol = cp_decode(s, &s.dst, s.ndst as c_int);
            let backwards_distance = cp_read_bits(s, cp_dist_extra_bits[distance_symbol as usize] as c_int) + cp_dist_base[distance_symbol as usize];
            
            unsafe {
                if (s.out as usize).wrapping_sub(backwards_distance as usize) < s.begin as usize {
                    cp_error_reason = "Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                    return 0;
                }
                if s.out.add(length as usize) > s.out_end {
                    cp_error_reason = "Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                    return 0;
                }
                
                let src = s.out.sub(backwards_distance as usize);
                let dst = s.out;
                s.out = s.out.add(length as usize);
                
                if backwards_distance == 1 {
                    let val = *src;
                    for i in 0..length {
                        *dst.add(i as usize) = val;
                    }
                } else {
                    for i in 0..length {
                        *dst.add(i as usize) = *src.add(i as usize);
                    }
                }
            }
        } else {
            break;
        }
    }
    1
}

fn cp_inflate(inp: *const c_void, in_bytes: c_int, outp: *mut c_void, out_bytes: c_int) -> c_int {
    let layout = Layout::new::<cp_state_t>();
    let s = unsafe { alloc(layout) as *mut cp_state_t };
    if s.is_null() {
        return 0;
    }
    
    unsafe {
        ptr::write_bytes(s as *mut u8, 0, size_of::<cp_state_t>());
        (*s).bits = 0;
        (*s).count = 0;
        (*s).word_index = 0;
        (*s).bits_left = in_bytes * 8;
        
        let in_ptr = inp as *const u8;
        let first_bytes = (((in_ptr as usize + 3) & !3) - in_ptr as usize) as c_int;
        (*s).words = (in_ptr as *const u32).add((first_bytes / 4) as usize);
        (*s).word_count = (in_bytes - first_bytes) / 4;
        let last_bytes = ((in_bytes - first_bytes) & 3) as c_int;
        
        for i in 0..first_bytes {
            (*s).bits |= (*in_ptr.add(i as usize) as u64) << (i * 8);
        }
        
        (*s).final_word_available = if last_bytes > 0 { 1 } else { 0 };
        (*s).final_word = 0;
        for i in 0..last_bytes {
            (*s).final_word |= (*in_ptr.add((in_bytes - last_bytes + i) as usize) as u32) << (i * 8);
        }
        
        (*s).count = first_bytes * 8;
        (*s).out = outp as *mut u8;
        (*s).out_end = (*s).out.add(out_bytes as usize);
        (*s).begin = outp as *mut u8;
        
        let mut count = 0;
        let mut bfinal;
        
        loop {
            bfinal = cp_read_bits(&mut *s, 1);
            let btype = cp_read_bits(&mut *s, 2);
            
            match btype {
                0 => {
                    if cp_stored(&mut *s) == 0 {
                        dealloc(s as *mut u8, layout);
                        return 0;
                    }
                }
                1 => {
                    cp_fixed(&mut *s);
                    if cp_block(&mut *s) == 0 {
                        dealloc(s as *mut u8, layout);
                        return 0;
                    }
                }
                2 => {
                    cp_dynamic(&mut *s);
                    if cp_block(&mut *s) == 0 {
                        dealloc(s as *mut u8, layout);
                        return 0;
                    }
                }
                _ => {
                    cp_error_reason = "Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
                    dealloc(s as *mut u8, layout);
                    return 0;
                }
            }
            
            count += 1;
            if bfinal != 0 {
                break;
            }
        }
        
        dealloc(s as *mut u8, layout);
        1
    }
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

fn cp_make32(s: *const u8) -> u32 {
    unsafe {
        ((*s as u32) << 24) | ((*s.add(1) as u32) << 16) | ((*s.add(2) as u32) << 8) | (*s.add(3) as u32)
    }
}

fn cp_chunk(png: &mut cp_raw_png_t, chunk: &[u8], minlen: u32) -> *const u8 {
    unsafe {
        let len = cp_make32(png.p);
        let start = png.p;
        if std::slice::from_raw_parts(start.add(4), 4) == chunk && len >= minlen {
            let offset = len + 12;
            if png.p.add(offset as usize) <= png.end {
                png.p = png.p.add(offset as usize);
                return start.add(8);
            }
        }
        ptr::null()
    }
}

fn cp_find(png: &mut cp_raw_png_t, chunk: &[u8], minlen: u32) -> *const u8 {
    unsafe {
        while png.p < png.end {
            let len = cp_make32(png.p);
            let start = png.p;
            png.p = png.p.add((len + 12) as usize);
            if std::slice::from_raw_parts(start.add(4), 4) == chunk && len >= minlen && png.p <= png.end {
                return start.add(8);
            }
        }
        ptr::null()
    }
}

fn cp_unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    unsafe {
        let len = (w * bpp) as usize;
        let mut prev: *mut u8;
        let mut x: usize;
        
        if h > 0 {
            let filter_type = *raw;
            raw = raw.add(1);
            match filter_type {
                0 => {}
                1 => {
                    for x in bpp as usize..len {
                        *raw.add(x) = raw.add(x).read().wrapping_add(raw.add(x - bpp as usize).read());
                    }
                }
                2 => {}
                3 => {
                    for x in bpp as usize..len {
                        *raw.add(x) = raw.add(x).read().wrapping_add(raw.add(x - bpp as usize).read() / 2);
                    }
                }
                4 => {
                    for x in bpp as usize..len {
                        *raw.add(x) = raw.add(x).read().wrapping_add(cp_paeth(raw.add(x - bpp as usize).read(), 0, 0));
                    }
                }
                _ => return 0,
            }
        }
        
        prev = raw;
        raw = raw.add(len);
        
        for _ in 1..h {
            let filter_type = *raw;
            raw = raw.add(1);
            match filter_type {
                0 => {}
                1 => {
                    for x in 0..bpp as usize {
                        *raw.add(x) = raw.add(x).read();
                    }
                    for x in bpp as usize..len {
                        *raw.add(x) = raw.add(x).read().wrapping_add(raw.add(x - bpp as usize).read());
                    }
                }
                2 => {
                    for x in 0..len {
                        *raw.add(x) = raw.add(x).read().wrapping_add(prev.add(x).read());
                    }
                }
                3 => {
                    for x in 0..bpp as usize {
                        *raw.add(x) = raw.add(x).read().wrapping_add(prev.add(x).read() / 2);
                    }
                    for x in bpp as usize..len {
                        *raw.add(x) = raw.add(x).read().wrapping_add(((raw.add(x - bpp as usize).read() as u16 + prev.add(x).read() as u16) / 2) as u8);
                    }
                }
                4 => {
                    for x in 0..bpp as usize {
                        *raw.add(x) = raw.add(x).read().wrapping_add(prev.add(x).read());
                    }
                    for x in bpp as usize..len {
                        *raw.add(x) = raw.add(x).read().wrapping_add(cp_paeth(
                            raw.add(x - bpp as usize).read(),
                            prev.add(x).read(),
                            prev.add(x - bpp as usize).read(),
                        ));
                    }
                }
                _ => return 0,
            }
            prev = raw;
            raw = raw.add(len);
        }
        1
    }
}

fn cp_convert(bpp: c_int, w: c_int, h: c_int, src: *mut u8, dst: *mut cp_pixel_t) {
    unsafe {
        let mut src = src;
        let mut dst = dst;
        for _ in 0..h {
            src = src.add(1);
            for _ in 0..w {
                match bpp {
                    1 => {
                        let v = *src;
                        *dst = cp_make_pixel(v, v, v);
                        src = src.add(1);
                    }
                    2 => {
                        *dst = cp_make_pixel_a(*src, *src, *src, *src.add(1));
                        src = src.add(2);
                    }
                    3 => {
                        *dst = cp_make_pixel(*src, *src.add(1), *src.add(2));
                        src = src.add(3);
                    }
                    4 => {
                        *dst = cp_make_pixel_a(*src, *src.add(1), *src.add(2), *src.add(3));
                        src = src.add(4);
                    }
                    _ => {}
                }
                dst = dst.add(1);
            }
        }
    }
}

fn cp_get_alpha_for_indexed_image(index: c_int, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() {
        255
    } else if (index as u32) >= trns_len {
        255
    } else {
        unsafe { *trns.add(index as usize) }
    }
}

fn cp_depalette(w: c_int, h: c_int, src: *mut u8, dst: *mut cp_pixel_t, plte: *const u8, trns: *const u8, trns_len: u32) {
    unsafe {
        let mut src = src;
        let mut dst = dst;
        for _ in 0..h {
            src = src.add(1);
            for x in 0..w {
                let c = *src as c_int;
                let r = *plte.add(c as usize * 3);
                let g = *plte.add(c as usize * 3 + 1);
                let b = *plte.add(c as usize * 3 + 2);
                let a = cp_get_alpha_for_indexed_image(c, trns, trns_len);
                *dst = cp_make_pixel_a(r, g, b, a);
                src = src.add(1);
                dst = dst.add(1);
            }
        }
    }
}

fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 {
    unsafe { cp_make32(chunk.sub(8)) }
}

fn cp_out_size(img: &cp_image_t, bpp: c_int) -> c_int {
    (img.w + 1) * img.h * bpp
}

#[unsafe(no_mangle)]
pub extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    const SIG: &[u8] = &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'];
    
    unsafe {
        let mut img = cp_image_t { w: 0, h: 0, pix: ptr::null_mut() };
        let mut png = cp_raw_png_t { p: png_data, end: png_data.add(png_length as usize) };
        
        if std::slice::from_raw_parts(png.p, 8) != SIG {
            cp_error_reason = "incorrect file signature (is this a png file?)\0".as_ptr() as *const c_char;
            return img;
        }
        png.p = png.p.add(8);
        
        let ihdr = cp_chunk(&mut png, b"IHDR", 13);
        if ihdr.is_null() {
            cp_error_reason = "unable to find IHDR chunk\0".as_ptr() as *const c_char;
            return img;
        }
        
        let bit_depth = *ihdr.add(8);
        let color_type = *ihdr.add(9);
        
        if bit_depth != 8 {
            cp_error_reason = "only bit-depth of 8 is supported\0".as_ptr() as *const c_char;
            return img;
        }
        
        let bpp = match color_type {
            0 => 1,
            2 => 3,
            3 => 1,
            4 => 2,
            6 => 4,
            _ => {
                cp_error_reason = "unknown color type\0".as_ptr() as *const c_char;
                return img;
            }
        };
        
        let w = (cp_make32(ihdr) + 1) as c_int;
        let h = cp_make32(ihdr.add(4)) as c_int;
        
        if w < 1 {
            cp_error_reason = "invalid IHDR chunk found, image width was less than 1\0".as_ptr() as *const c_char;
            return img;
        }
        if h < 1 {
            cp_error_reason = "invalid IHDR chunk found, image height was less than 1\0".as_ptr() as *const c_char;
            return img;
        }
        
        if (w as i64) * (h as i64) * (size_of::<cp_pixel_t>() as i64) >= i32::MAX as i64 {
            cp_error_reason = "image too large\0".as_ptr() as *const c_char;
            return img;
        }
        
        let pix_bytes = (w * h * size_of::<cp_pixel_t>() as c_int) as usize;
        img.w = w - 1;
        img.h = h;
        
        let layout = Layout::array::<cp_pixel_t>(pix_bytes / size_of::<cp_pixel_t>()).unwrap();
        img.pix = alloc(layout) as *mut cp_pixel_t;
        if img.pix.is_null() {
            cp_error_reason = "unable to allocate raw image space\0".as_ptr() as *const c_char;
            return img;
        }
        
        let compression = *ihdr.add(10);
        let filter = *ihdr.add(11);
        let interlace = *ihdr.add(12);
        
        if compression != 0 {
            cp_error_reason = "only standard compression DEFLATE is supported\0".as_ptr() as *const c_char;
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        if filter != 0 {
            cp_error_reason = "only standard adaptive filtering is supported\0".as_ptr() as *const c_char;
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        if interlace != 0 {
            cp_error_reason = "interlacing is not supported\0".as_ptr() as *const c_char;
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        
        let first = png.p;
        let plte = cp_find(&mut png, b"PLTE", 0);
        if plte.is_null() {
            png.p = first;
        }
        let first = png.p;
        let trns = cp_find(&mut png, b"tRNS", 0);
        if trns.is_null() {
            png.p = first;
        }
        
        let mut datalen: usize = 0;
        png.p = first;
        let mut idat = cp_find(&mut png, b"IDAT", 0);
        while !idat.is_null() {
            datalen += cp_get_chunk_byte_length(idat) as usize;
            idat = cp_chunk(&mut png, b"IDAT", 0);
        }
        
        png.p = first;
        let data_layout = Layout::array::<u8>(datalen).unwrap();
        let data = alloc(data_layout) as *mut u8;
        if data.is_null() {
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        
        let mut offset: usize = 0;
        let mut idat = cp_find(&mut png, b"IDAT", 0);
        while !idat.is_null() {
            let len = cp_get_chunk_byte_length(idat) as usize;
            std::ptr::copy_nonoverlapping(idat, data.add(offset), len);
            offset += len;
            idat = cp_chunk(&mut png, b"IDAT", 0);
        }
        
        if data.is_null() || datalen < 6 {
            cp_error_reason = "corrupt zlib structure in DEFLATE stream\0".as_ptr() as *const c_char;
            dealloc(data, data_layout);
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        
        if (*data & 0x0f) != 0x08 {
            cp_error_reason = "only zlib compression method (RFC 1950) is supported\0".as_ptr() as *const c_char;
            dealloc(data, data_layout);
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        
        if (*data & 0xf0) > 0x70 {
            cp_error_reason = "innapropriate window size detected\0".as_ptr() as *const c_char;
            dealloc(data, data_layout);
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        
        if (*data.add(1) & 0x20) != 0 {
            cp_error_reason = "preset dictionary is present and not supported\0".as_ptr() as *const c_char;
            dealloc(data, data_layout);
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        
        if cp_out_size(&img, 4) < 1 {
            cp_error_reason = "invalid image size found\0".as_ptr() as *const c_char;
            dealloc(data, data_layout);
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        
        if cp_out_size(&img, bpp) < 1 {
            cp_error_reason = "invalid image size found\0".as_ptr() as *const c_char;
            dealloc(data, data_layout);
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        
        let out = (img.pix as *mut u8).add(cp_out_size(&img, 4) as usize - cp_out_size(&img, bpp) as usize);
        
        if cp_inflate(data.add(2) as *const c_void, (datalen - 6) as c_int, out as *mut c_void, pix_bytes as c_int) == 0 {
            cp_error_reason = "DEFLATE algorithm failed\0".as_ptr() as *const c_char;
            dealloc(data, data_layout);
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        
        if cp_unfilter(img.w, img.h, bpp, out) == 0 {
            cp_error_reason = "invalid filter byte found\0".as_ptr() as *const c_char;
            dealloc(data, data_layout);
            dealloc(img.pix as *mut u8, layout);
            img.pix = ptr::null_mut();
            return img;
        }
        
        if color_type == 3 {
            if plte.is_null() {
                cp_error_reason = "color type of indexed requires a PLTE chunk\0".as_ptr() as *const c_char;
                dealloc(data, data_layout);
                dealloc(img.pix as *mut u8, layout);
                img.pix = ptr::null_mut();
                return img;
            }
            let trns_len = if trns.is_null() { 0 } else { cp_get_chunk_byte_length(trns) };
            cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
        } else {
            cp_convert(bpp, img.w, img.h, out, img.pix);
        }
        
        dealloc(data, data_layout);
        img
    }
}
