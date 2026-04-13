use std::ffi::{c_char, c_int, c_void};
use std::os::raw::{c_uint, c_ulonglong};
use std::ptr;
use std::alloc::{alloc, dealloc, Layout};
use std::mem::size_of;

#[repr(C)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

pub static mut CP_ERROR_REASON: *const c_char = ptr::null();

pub static CP_FIXED_TABLE: [u8; 288 + 32] = [
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

pub static CP_PERMUTATION_ORDER: [u8; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];

pub static CP_LEN_EXTRA_BITS: [u8; 29 + 2] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0];

pub static CP_LEN_BASE: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31,
    35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

pub static CP_DIST_EXTRA_BITS: [u8; 30 + 2] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 0, 0];

pub static CP_DIST_BASE: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

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

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> CpPixel {
    CpPixel { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> CpPixel {
    CpPixel { r, g, b, a: 0xFF }
}

fn cp_would_overflow(s: &CpState, num_bits: c_int) -> c_int {
    ((s.bits_left + s.count) - num_bits < 0) as c_int
}

fn cp_ptr(s: &mut CpState) -> *mut c_char {
    assert!(s.bits_left & 7 == 0);
    unsafe {
        (s.words.add(s.word_index as usize) as *mut c_char).sub((s.count / 8) as usize)
    }
}

fn cp_peak_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { *s.words.add(s.word_index as usize) };
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            s.word_index += 1;
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

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(s.count >= num_bits_to_read);
    let bits = s.bits & (((1u64) << num_bits_to_read) - 1);
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits as u32
}

fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!(s.bits_left > 0);
    assert!(s.count <= 64);
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

fn cp_build(s: Option<&mut CpState>, tree: &mut [u32], lens: &[u8], sym_count: c_int) -> c_int {
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
    
    if let Some(state) = s {
        state.lookup.fill(0);
    }
    
    for i in 0..sym_count {
        let len = lens[i as usize] as c_int;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len as usize] as u32;
            codes[len as usize] += 1;
            let slot = first[len as usize] as usize;
            first[len as usize] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(state) = s {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        state.lookup[j] = ((len << 9) | i) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    }
    
    first[15]
}

fn cp_stored(s: &mut CpState) -> c_int {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    
    if len != !nlen {
        unsafe {
            CP_ERROR_REASON = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        }
        return 0;
    }
    
    if s.bits_left / 8 > len as c_int {
        unsafe {
            CP_ERROR_REASON = b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        }
        return 0;
    }
    
    let p = cp_ptr(s);
    unsafe {
        ptr::copy_nonoverlapping(p, s.out, len as usize);
        s.out = s.out.add(len as usize);
    }
    1
}

fn cp_fixed(s: &mut CpState) -> c_int {
    s.nlit = cp_build(Some(s), &mut s.lit, &CP_FIXED_TABLE[..288], 288) as u32;
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32) as u32;
    1
}

fn cp_decode(s: &mut CpState, tree: &[u32], hi: u32) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0;
    let hi = hi as c_int;
    
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < tree[guess as usize] {
            lo = guess;
        } else {
            lo = guess + 1;
        }
    }
    
    let key = tree[(lo - 1) as usize];
    let len = 32 - (key & 0xF);
    assert!((search >> len) == (key >> len));
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

fn cp_dynamic(s: &mut CpState) -> c_int {
    let mut lenlens: [u8; 19] = [0; 19];
    let nlit = 257 + cp_read_bits(s, 5) as c_int;
    let ndst = 1 + cp_read_bits(s, 5) as c_int;
    let nlen = 4 + cp_read_bits(s, 4) as c_int;
    
    for i in 0..nlen {
        lenlens[CP_PERMUTATION_ORDER[i as usize] as usize] = cp_read_bits(s, 3) as u8;
    }
    
    s.nlen = cp_build(None, &mut s.len, &lenlens, 19) as u32;
    
    let mut lens: [u8; 288 + 32] = [0; 288 + 32];
    let mut n: c_int = 0;
    
    while n < nlit + ndst {
        let sym = cp_decode(s, &s.len, s.nlen);
        match sym {
            16 => {
                let count = 3 + cp_read_bits(s, 2);
                for _ in 0..count {
                    lens[n as usize] = lens[(n - 1) as usize];
                    n += 1;
                }
            }
            17 => {
                let count = 3 + cp_read_bits(s, 3);
                for _ in 0..count {
                    lens[n as usize] = 0;
                    n += 1;
                }
            }
            18 => {
                let count = 11 + cp_read_bits(s, 7);
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
    
    s.nlit = cp_build(Some(s), &mut s.lit, &lens[..nlit as usize], nlit) as u32;
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit as usize..(nlit + ndst) as usize], ndst) as u32;
    1
}

fn cp_block(s: &mut CpState) -> c_int {
    loop {
        let symbol = cp_decode(s, &s.lit, s.nlit);
        if symbol < 256 {
            unsafe {
                if s.out.add(1) > s.out_end {
                    CP_ERROR_REASON = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                    return 0;
                }
                *s.out = symbol as c_char;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol as usize] as c_int) + CP_LEN_BASE[symbol as usize];
            let distance_symbol = cp_decode(s, &s.dst, s.ndst);
            let backwards_distance = cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol as usize] as c_int) + CP_DIST_BASE[distance_symbol as usize];
            
            unsafe {
                if (s.out as usize).wrapping_sub(backwards_distance as usize) < s.begin as usize {
                    CP_ERROR_REASON = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                    return 0;
                }
                if s.out.add(length as usize) > s.out_end {
                    CP_ERROR_REASON = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
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

#[unsafe(no_mangle)]
pub extern "C" fn cp_inflate(input: *mut c_void, in_bytes: c_int, output: *mut c_void, out_bytes: c_int) -> c_int {
    let layout = Layout::new::<CpState>();
    let s = unsafe { alloc(layout) as *mut CpState };
    if s.is_null() {
        return 0;
    }
    
    unsafe {
        ptr::write_bytes(s as *mut u8, 0, size_of::<CpState>());
        (*s).bits = 0;
        (*s).count = 0;
        (*s).word_index = 0;
        (*s).bits_left = in_bytes * 8;
        
        let in_ptr = input as *const u8;
        let aligned = ((in_ptr as usize + 3) & !3) - in_ptr as usize;
        let first_bytes = aligned as c_int;
        (*s).words = (in_ptr.add(first_bytes as usize) as *mut u32);
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
        (*s).out = output as *mut c_char;
        (*s).out_end = (*s).out.add(out_bytes as usize);
        (*s).begin = output as *mut c_char;
        
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
                    CP_ERROR_REASON = b"Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
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

struct CpRawPng {
    p: *const u8,
    end: *const u8,
}

fn cp_make32(s: *const u8) -> u32 {
    unsafe {
        ((*s.add(0) as u32) << 24) |
        ((*s.add(1) as u32) << 16) |
        ((*s.add(2) as u32) << 8) |
        (*s.add(3) as u32)
    }
}

fn cp_chunk(png: &mut CpRawPng, chunk: &[u8], minlen: u32) -> *const u8 {
    unsafe {
        let len = cp_make32(png.p);
        let start = png.p;
        
        let chunk_type = std::slice::from_raw_parts(start.add(4), 4);
        if chunk_type == chunk && len >= minlen {
            let offset = len + 12;
            if png.p.add(offset as usize) <= png.end {
                png.p = png.p.add(offset as usize);
                return start.add(8);
            }
        }
        ptr::null()
    }
}

fn cp_find(png: &mut CpRawPng, chunk: &[u8], minlen: u32) -> *const u8 {
    unsafe {
        while png.p < png.end {
            let len = cp_make32(png.p);
            let start = png.p;
            png.p = png.p.add((len + 12) as usize);
            
            let chunk_type = std::slice::from_raw_parts(start.add(4), 4);
            if chunk_type == chunk && len >= minlen && png.p <= png.end {
                return start.add(8);
            }
        }
        ptr::null()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len = (w * bpp) as usize;
    let mut raw = raw;
    
    if h > 0 {
        unsafe {
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
    }
    
    let mut prev = raw;
    unsafe {
        raw = raw.add(len);
    }
    
    for _ in 1..h {
        unsafe {
            let filter_type = *raw;
            raw = raw.add(1);
            
            match filter_type {
                0 => {}
                1 => {
                    for x in 0..bpp as usize {
                        *raw.add(x) = raw.add(x).read().wrapping_add(0);
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
                        let a = raw.add(x - bpp as usize).read();
                        let b = prev.add(x).read();
                        *raw.add(x) = raw.add(x).read().wrapping_add(((a as u16 + b as u16) / 2) as u8);
                    }
                }
                4 => {
                    for x in 0..bpp as usize {
                        *raw.add(x) = raw.add(x).read().wrapping_add(prev.add(x).read());
                    }
                    for x in bpp as usize..len {
                        let a = raw.add(x - bpp as usize).read();
                        let b = prev.add(x).read();
                        let c = prev.add(x - bpp as usize).read();
                        *raw.add(x) = raw.add(x).read().wrapping_add(cp_paeth(a, b, c));
                    }
                }
                _ => return 0,
            }
            
            prev = raw;
            raw = raw.add(len);
        }
    }
    
    1
}
