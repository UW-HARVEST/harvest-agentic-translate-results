use std::ffi::c_void;
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicPtr, Ordering};

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
    pub w: i32,
    pub h: i32,
    pub pix: *mut cp_pixel_t,
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

static CP_ERROR_REASON: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());

static CP_FIXED_TABLE: [u8; 288 + 32] = [
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

static CP_PERMUTATION_ORDER: [u8; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
static CP_LEN_EXTRA_BITS: [u8; 29 + 2] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0];
static CP_LEN_BASE: [u32; 29 + 2] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0];
static CP_DIST_EXTRA_BITS: [u8; 30 + 2] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 0, 0];
static CP_DIST_BASE: [u32; 30 + 2] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0];

struct CpState<'a> {
    bits: u64,
    count: i32,
    words: &'a [u32],
    word_count: i32,
    word_index: i32,
    bits_left: i32,
    final_word_available: bool,
    final_word: u32,
    out_ptr: *mut u8,
    out_pos: usize,
    out_end: usize,
    begin: *mut u8,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: i32,
    ndst: i32,
    nlen: i32,
}

fn set_error_reason(msg: &'static [u8]) {
    CP_ERROR_REASON.store(msg.as_ptr() as *mut u8, Ordering::Relaxed);
}

fn cp_would_overflow(s: &CpState<'_>, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

fn cp_ptr(s: &CpState<'_>) -> *const u8 {
    assert!((s.bits_left & 7) == 0);
    let base = s.words.as_ptr() as *const u8;
    let offset = (s.word_index as usize) * 4;
    base.wrapping_add(offset).wrapping_sub((s.count as usize) / 8)
}

fn cp_peak_bits(s: &mut CpState<'_>, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = s.words[s.word_index as usize];
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            assert!(s.word_index <= s.word_count);
        } else if s.final_word_available {
            let word = s.final_word;
            s.bits |= (word as u64) << s.count;
            s.count += s.bits_left;
            s.final_word_available = false;
        }
    }
    s.bits
}

fn cp_consume_bits(s: &mut CpState<'_>, num_bits_to_read: i32) -> u32 {
    assert!(s.count >= num_bits_to_read);
    let bits = if num_bits_to_read == 32 {
        (s.bits & u32::MAX as u64) as u32
    } else {
        (s.bits & (((1u64) << num_bits_to_read) - 1)) as u32
    };
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

fn cp_read_bits(s: &mut CpState<'_>, num_bits_to_read: i32) -> u32 {
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

fn cp_build(s: Option<&mut CpState<'_>>, tree: &mut [u32], lens: &[u8], sym_count: i32) -> i32 {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    for n in 0..sym_count as usize {
        counts[lens[n] as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if let Some(state) = s.as_ref() {
        let _ = state;
    }
    let mut s = s;
    if let Some(state) = s.as_mut() {
        state.lookup.fill(0);
    }
    for i in 0..sym_count as usize {
        let len = lens[i] as usize;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(state) = s.as_mut() {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        state.lookup[j] = (((len as u16) << 9) | i as u16) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    }
    first[15]
}

fn cp_stored(s: &mut CpState<'_>) -> bool {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        set_error_reason(b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0");
        return false;
    }
    if s.bits_left / 8 <= len as i32 {
        set_error_reason(b"Stored block extends beyond end of input stream.\0");
        return false;
    }
    let p = cp_ptr(s);
    if s.out_pos + len as usize > s.out_end {
        set_error_reason(b"Attempted to overwrite out buffer while outputting a string.\0");
        return false;
    }
    unsafe {
        ptr::copy_nonoverlapping(p, s.out_ptr.add(s.out_pos), len as usize);
    }
    s.out_pos += len as usize;
    true
}

fn cp_fixed(s: &mut CpState<'_>) -> bool {
    s.nlit = cp_build(Some(s), &mut [0; 0], &[], 0);
    s.nlit = cp_build(Some(s), &mut s.lit, &CP_FIXED_TABLE[..288], 288);
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32);
    true
}

fn cp_decode(s: &mut CpState<'_>, tree: &[u32], mut hi: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = ((cp_rev16(bits as u32) as u64) << 16 | 0xFFFF) as u32;
    let mut lo = 0i32;
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
    assert!((search >> len) == (key >> len));
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

fn cp_dynamic(s: &mut CpState<'_>) -> bool {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as i32;
    let ndst = 1 + cp_read_bits(s, 5) as i32;
    let nlen = 4 + cp_read_bits(s, 4) as i32;
    for i in 0..nlen as usize {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    s.nlen = cp_build(None, &mut s.len, &lenlens, 19);
    let mut lens = [0u8; 288 + 32];
    let mut n = 0usize;
    while n < (nlit + ndst) as usize {
        let sym = cp_decode(s, &s.len, s.nlen);
        match sym {
            16 => {
                let repeat = 3 + cp_read_bits(s, 2) as usize;
                for _ in 0..repeat {
                    lens[n] = lens[n - 1];
                    n += 1;
                }
            }
            17 => {
                let repeat = 3 + cp_read_bits(s, 3) as usize;
                for _ in 0..repeat {
                    lens[n] = 0;
                    n += 1;
                }
            }
            18 => {
                let repeat = 11 + cp_read_bits(s, 7) as usize;
                for _ in 0..repeat {
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
    s.nlit = cp_build(Some(s), &mut s.lit, &lens[..nlit as usize], nlit);
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit as usize..(nlit + ndst) as usize], ndst);
    true
}

fn cp_block(s: &mut CpState<'_>) -> bool {
    loop {
        let symbol = cp_decode(s, &s.lit, s.nlit);
        if symbol < 256 {
            if s.out_pos + 1 > s.out_end {
                set_error_reason(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return false;
            }
            unsafe {
                *s.out_ptr.add(s.out_pos) = symbol as u8;
            }
            s.out_pos += 1;
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol as usize] as i32) + CP_LEN_BASE[symbol as usize];
            let distance_symbol = cp_decode(s, &s.dst, s.ndst);
            let backwards_distance = cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol as usize] as i32) + CP_DIST_BASE[distance_symbol as usize];
            if (s.out_pos as isize) - (backwards_distance as isize) < 0 {
                set_error_reason(b"Attempted to write before out buffer (invalid backwards distance).\0");
                return false;
            }
            if s.out_pos + length as usize > s.out_end {
                set_error_reason(b"Attempted to overwrite out buffer while outputting a string.\0");
                return false;
            }
            let src_pos = s.out_pos - backwards_distance as usize;
            let dst_pos = s.out_pos;
            s.out_pos += length as usize;
            unsafe {
                match backwards_distance {
                    1 => {
                        let value = *s.out_ptr.add(src_pos);
                        ptr::write_bytes(s.out_ptr.add(dst_pos), value, length as usize);
                    }
                    _ => {
                        for i in 0..length as usize {
                            let v = *s.out_ptr.add(src_pos + i);
                            *s.out_ptr.add(dst_pos + i) = v;
                        }
                    }
                }
            }
        } else {
            break;
        }
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn cp_inflate(in_ptr: *mut c_void, in_bytes: i32, out_ptr: *mut c_void, out_bytes: i32) -> i32 {
    if in_ptr.is_null() || out_ptr.is_null() || in_bytes < 0 || out_bytes < 0 {
        return 0;
    }
    let input = unsafe { slice::from_raw_parts(in_ptr as *const u8, in_bytes as usize) };
    let in_addr = input.as_ptr() as usize;
    let first_bytes = (((in_addr + 3) & !3).wrapping_sub(in_addr)).min(in_bytes as usize);
    let words_bytes = (in_bytes as usize).saturating_sub(first_bytes);
    let word_count = words_bytes / 4;
    let mut words = Vec::with_capacity(word_count);
    for i in 0..word_count {
        let base = first_bytes + i * 4;
        words.push(u32::from_ne_bytes([
            input[base],
            input[base + 1],
            input[base + 2],
            input[base + 3],
        ]));
    }
    let last_bytes = (in_bytes as usize).saturating_sub(first_bytes + word_count * 4);
    let mut bits = 0u64;
    for (i, b) in input[..first_bytes].iter().enumerate() {
        bits |= (*b as u64) << (i * 8);
    }
    let mut final_word = 0u32;
    for i in 0..last_bytes {
        final_word |= (input[in_bytes as usize - last_bytes + i] as u32) << (i * 8);
    }
    let mut s = CpState {
        bits,
        count: (first_bytes * 8) as i32,
        words: &words,
        word_count: word_count as i32,
        word_index: 0,
        bits_left: in_bytes * 8,
        final_word_available: last_bytes != 0,
        final_word,
        out_ptr: out_ptr as *mut u8,
        out_pos: 0,
        out_end: out_bytes as usize,
        begin: out_ptr as *mut u8,
        lookup: [0; 1 << 9],
        lit: [0; 288],
        dst: [0; 32],
        len: [0; 19],
        nlit: 0,
        ndst: 0,
        nlen: 0,
    };
    loop {
        let bfinal = cp_read_bits(&mut s, 1);
        let btype = cp_read_bits(&mut s, 2);
        match btype {
            0 => {
                if !cp_stored(&mut s) {
                    return 0;
                }
            }
            1 => {
                cp_fixed(&mut s);
                if !cp_block(&mut s) {
                    return 0;
                }
            }
            2 => {
                cp_dynamic(&mut s);
                if !cp_block(&mut s) {
                    return 0;
                }
            }
            3 => {
                set_error_reason(b"Detected unknown block type within input stream.\0");
                return 0;
            }
            _ => unreachable!(),
        }
        if bfinal != 0 {
            break;
        }
    }
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

struct CpRawPng<'a> {
    p: &'a [u8],
    pos: usize,
}

fn cp_make32(s: &[u8]) -> u32 {
    ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | (s[3] as u32)
}

fn cp_chunk<'a>(png: &mut CpRawPng<'a>, chunk: &[u8; 4], minlen: u32) -> Option<&'a [u8]> {
    if png.pos + 8 > png.p.len() {
        return None;
    }
    let len = cp_make32(&png.p[png.pos..png.pos + 4]);
    let start = png.pos;
    if &png.p[start + 4..start + 8] == chunk && len >= minlen {
        let offset = len as usize + 12;
        if png.pos + offset <= png.p.len() {
            png.pos += offset;
            return Some(&png.p[start + 8..start + 8 + len as usize]);
        }
    }
    None
}

fn cp_find<'a>(png: &mut CpRawPng<'a>, chunk: &[u8; 4], minlen: u32) -> Option<&'a [u8]> {
    while png.pos < png.p.len() {
        if png.pos + 8 > png.p.len() {
            return None;
        }
        let len = cp_make32(&png.p[png.pos..png.pos + 4]) as usize;
        let start = png.pos;
        png.pos = png.pos.saturating_add(len + 12);
        if png.pos <= png.p.len() && &png.p[start + 4..start + 8] == chunk && len as u32 >= minlen {
            return Some(&png.p[start + 8..start + 8 + len]);
        }
    }
    None
}

fn cp_unfilter(w: i32, h: i32, bpp: i32, raw: &mut [u8]) -> i32 {
    let len = (w * bpp) as usize;
    if h > 0 {
        let filter = raw[0];
        let row = &mut raw[1..1 + len];
        match filter {
            0 => {}
            1 => {
                for x in bpp as usize..len {
                    row[x] = row[x].wrapping_add(row[x - bpp as usize]);
                }
            }
            2 => {}
            3 => {
                for x in bpp as usize..len {
                    row[x] = row[x].wrapping_add(row[x - bpp as usize] / 2);
                }
            }
            4 => {
                for x in bpp as usize..len {
                    row[x] = row[x].wrapping_add(cp_paeth(row[x - bpp as usize], 0, 0));
                }
            }
            _ => return 0,
        }
    }
    for y in 1..h as usize {
        let prev_start = 1 + (y - 1) * (len + 1);
        let curr_start = 1 + y * (len + 1);
        let (left, right) = raw.split_at_mut(curr_start);
        let prev = &left[prev_start..prev_start + len];
        let filter = right[0];
        let row = &mut right[1..1 + len];
        match filter {
            0 => {}
            1 => {
                for x in 0..bpp as usize {
                    row[x] = row[x].wrapping_add(0);
                }
                for x in bpp as usize..len {
                    row[x] = row[x].wrapping_add(row[x - bpp as usize]);
                }
            }
            2 => {
                for x in 0..len {
                    row[x] = row[x].wrapping_add(prev[x]);
                }
            }
            3 => {
                for x in 0..bpp as usize {
                    row[x] = row[x].wrapping_add(prev[x] / 2);
                }
                for x in bpp as usize..len {
                    row[x] = row[x].wrapping_add(((row[x - bpp as usize] as u16 + prev[x] as u16) / 2) as u8);
                }
            }
            4 => {
                for x in 0..bpp as usize {
                    row[x] = row[x].wrapping_add(prev[x]);
                }
                for x in bpp as usize..len {
                    row[x] = row[x].wrapping_add(cp_paeth(row[x - bpp as usize], prev[x], prev[x - bpp as usize]));
                }
            }
            _ => return 0,
        }
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn convert_pix(bpp: i32, w: i32, h: i32, src: *mut u8, dst: *mut cp_pixel_t) {
    if src.is_null() || dst.is_null() || bpp <= 0 || w < 0 || h < 0 {
        return;
    }
    let mut src_ptr = src;
    let mut dst_ptr = dst;
    for _y in 0..h {
        unsafe {
            src_ptr = src_ptr.add(1);
        }
        for _x in 0..w {
            unsafe {
                let pixel = match bpp {
                    1 => cp_make_pixel(*src_ptr, *src_ptr, *src_ptr),
                    2 => cp_make_pixel_a(*src_ptr, *src_ptr, *src_ptr, *src_ptr.add(1)),
                    3 => cp_make_pixel(*src_ptr, *src_ptr.add(1), *src_ptr.add(2)),
                    4 => cp_make_pixel_a(*src_ptr, *src_ptr.add(1), *src_ptr.add(2), *src_ptr.add(3)),
                    _ => return,
                };
                *dst_ptr = pixel;
                dst_ptr = dst_ptr.add(1);
                src_ptr = src_ptr.add(bpp as usize);
            }
        }
    }
}
