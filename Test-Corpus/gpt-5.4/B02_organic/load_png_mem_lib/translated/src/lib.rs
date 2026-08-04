use libc::{c_char, c_int, c_void};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicPtr, Ordering};

#[repr(C)]
#[derive(Copy, Clone, Default)]
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

impl Default for cp_image_t {
    fn default() -> Self {
        Self {
            w: 0,
            h: 0,
            pix: ptr::null_mut(),
        }
    }
}

static CP_ERROR_REASON_BYTES: &[u8] = b"\0";
#[unsafe(no_mangle)]
pub static cp_error_reason: AtomicPtr<c_char> = AtomicPtr::new(CP_ERROR_REASON_BYTES.as_ptr() as *mut c_char);

const CP_FIXED_TABLE: [u8; 288 + 32] = [
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

const CP_PERMUTATION_ORDER: [u8; 19] = [16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15];
const CP_LEN_EXTRA_BITS: [u8; 29 + 2] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0];
const CP_LEN_BASE: [u32; 29 + 2] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0];
const CP_DIST_EXTRA_BITS: [u8; 30 + 2] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 0, 0];
const CP_DIST_BASE: [u32; 30 + 2] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0];

struct CpState<'a> {
    bits: u64,
    count: i32,
    input: &'a [u8],
    input_pos: usize,
    bits_left: i32,
    out: &'a mut [u8],
    out_pos: usize,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

fn set_error(msg: &'static [u8]) {
    cp_error_reason.store(msg.as_ptr() as *mut c_char, Ordering::Relaxed);
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t {
    cp_pixel_t { r, g, b, a: 0xFF }
}

fn cp_would_overflow(s: &CpState<'_>, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

fn cp_peak_bits(s: &mut CpState<'_>, num_bits_to_read: i32) -> u64 {
    while s.count < num_bits_to_read && s.input_pos < s.input.len() {
        s.bits |= (s.input[s.input_pos] as u64) << s.count;
        s.input_pos += 1;
        s.count += 8;
    }
    s.bits
}

fn cp_consume_bits(s: &mut CpState<'_>, num_bits_to_read: i32) -> u32 {
    let mask = if num_bits_to_read == 32 {
        u64::MAX
    } else {
        (1u64 << num_bits_to_read) - 1
    };
    let bits = (s.bits & mask) as u32;
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

fn cp_read_bits(s: &mut CpState<'_>, num_bits_to_read: i32) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!(s.bits_left > 0);
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

fn cp_build(s: Option<&mut CpState<'_>>, tree: &mut [u32], lens: &[u8], sym_count: usize) -> i32 {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    for &len in lens.iter().take(sym_count) {
        counts[len as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if let Some(state) = s.as_ref() {
        let mut_lookup = unsafe { &mut *(&state.lookup as *const _ as *mut [u16; 1 << 9]) };
        mut_lookup.fill(0);
    }
    let mut s = s;
    for (i, &len_u8) in lens.iter().take(sym_count).enumerate() {
        let len = len_u8 as usize;
        if len != 0 {
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(state) = s.as_deref_mut() {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        state.lookup[j] = (((len as u16) << 9) | (i as u16)) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    }
    first[15]
}

fn cp_stored(s: &mut CpState<'_>) -> bool {
    let align = s.count & 7;
    if align != 0 {
        cp_read_bits(s, align);
    }
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        set_error(b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0");
        return false;
    }
    if (s.bits_left / 8) <= len as i32 {
        set_error(b"Stored block extends beyond end of input stream.\0");
        return false;
    }
    if s.count % 8 != 0 {
        return false;
    }
    let bytes_in_bitbuf = (s.count / 8) as usize;
    if s.input_pos < bytes_in_bitbuf {
        return false;
    }
    let ptr_pos = s.input_pos - bytes_in_bitbuf;
    let len_usize = len as usize;
    if ptr_pos + len_usize > s.input.len() || s.out_pos + len_usize > s.out.len() {
        return false;
    }
    s.out[s.out_pos..s.out_pos + len_usize].copy_from_slice(&s.input[ptr_pos..ptr_pos + len_usize]);
    s.out_pos += len_usize;
    s.input_pos = ptr_pos + len_usize;
    s.bits = 0;
    s.count = 0;
    s.bits_left -= (len as i32) * 8;
    true
}

fn cp_fixed(s: &mut CpState<'_>) -> bool {
    s.nlit = cp_build(Some(s), &mut s.lit, &CP_FIXED_TABLE, 288) as u32;
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32) as u32;
    true
}

fn cp_decode(s: &mut CpState<'_>, tree: &[u32], hi: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = ((cp_rev16(bits as u32) << 16) | 0xFFFF) as u32;
    let mut lo = 0i32;
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
    let len = key & 0xF;
    let _ = cp_consume_bits(s, len as i32);
    ((key >> 4) & 0xFFF) as i32
}

fn cp_dynamic(s: &mut CpState<'_>) -> bool {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as usize;
    let ndst = 1 + cp_read_bits(s, 5) as usize;
    let nlen = 4 + cp_read_bits(s, 4) as usize;
    for i in 0..nlen {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    s.nlen = cp_build(None, &mut s.len, &lenlens, 19) as u32;
    let mut lens = [0u8; 288 + 32];
    let mut n = 0usize;
    while n < nlit + ndst {
        let sym = cp_decode(s, &s.len, s.nlen as i32);
        match sym {
            16 => {
                let repeat = 3 + cp_read_bits(s, 2) as usize;
                let prev = lens[n - 1];
                for _ in 0..repeat {
                    lens[n] = prev;
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
    s.nlit = cp_build(Some(s), &mut s.lit, &lens, nlit) as u32;
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit..], ndst) as u32;
    true
}

fn cp_block(s: &mut CpState<'_>) -> bool {
    loop {
        let symbol = cp_decode(s, &s.lit, s.nlit as i32);
        if symbol < 256 {
            if s.out_pos + 1 > s.out.len() {
                set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return false;
            }
            s.out[s.out_pos] = symbol as u8;
            s.out_pos += 1;
        } else if symbol > 256 {
            let symbol = (symbol - 257) as usize;
            let length = cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol] as i32) + CP_LEN_BASE[symbol];
            let distance_symbol = cp_decode(s, &s.dst, s.ndst as i32) as usize;
            let backwards_distance = cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol] as i32) + CP_DIST_BASE[distance_symbol];
            let dist = backwards_distance as usize;
            let len = length as usize;
            if s.out_pos < dist {
                set_error(b"Attempted to write before out buffer (invalid backwards distance).\0");
                return false;
            }
            if s.out_pos + len > s.out.len() {
                set_error(b"Attempted to overwrite out buffer while outputting a string.\0");
                return false;
            }
            let src = s.out_pos - dist;
            let dst = s.out_pos;
            s.out_pos += len;
            if dist == 1 {
                let value = s.out[src];
                s.out[dst..dst + len].fill(value);
            } else {
                for i in 0..len {
                    let b = s.out[src + i];
                    s.out[dst + i] = b;
                }
            }
        } else {
            break;
        }
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn cp_inflate(in_ptr: *mut c_void, in_bytes: c_int, out_ptr: *mut c_void, out_bytes: c_int) -> c_int {
    if in_ptr.is_null() || out_ptr.is_null() || in_bytes < 0 || out_bytes < 0 {
        return 0;
    }
    let input = unsafe { slice::from_raw_parts(in_ptr as *const u8, in_bytes as usize) };
    let output = unsafe { slice::from_raw_parts_mut(out_ptr as *mut u8, out_bytes as usize) };
    let mut s = CpState {
        bits: 0,
        count: 0,
        input,
        input_pos: 0,
        bits_left: in_bytes * 8,
        out: output,
        out_pos: 0,
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
                set_error(b"Detected unknown block type within input stream.\0");
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
    p: usize,
    data: &'a [u8],
}

fn cp_make32(s: &[u8]) -> u32 {
    ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | (s[3] as u32)
}

fn cp_chunk<'a>(png: &mut CpRawPng<'a>, chunk: &[u8; 4], minlen: u32) -> Option<&'a [u8]> {
    if png.p + 8 > png.data.len() {
        return None;
    }
    let len = cp_make32(&png.data[png.p..png.p + 4]);
    let start = png.p;
    if &png.data[start + 4..start + 8] == chunk && len >= minlen {
        let offset = len as usize + 12;
        if png.p + offset <= png.data.len() {
            png.p += offset;
            return Some(&png.data[start + 8..start + 8 + len as usize]);
        }
    }
    None
}

fn cp_find<'a>(png: &mut CpRawPng<'a>, chunk: &[u8; 4], minlen: u32) -> Option<&'a [u8]> {
    while png.p + 8 <= png.data.len() {
        let len = cp_make32(&png.data[png.p..png.p + 4]) as usize;
        let start = png.p;
        png.p += len + 12;
        if png.p <= png.data.len() && &png.data[start + 4..start + 8] == chunk && len as u32 >= minlen {
            return Some(&png.data[start + 8..start + 8 + len]);
        }
    }
    None
}

fn cp_unfilter(w: i32, h: i32, bpp: i32, raw: &mut [u8]) -> bool {
    let len = (w * bpp) as usize;
    if h > 0 {
        match raw[0] {
            0 => {}
            1 => {
                for x in bpp as usize..len {
                    let v = raw[1 + x].wrapping_add(raw[1 + x - bpp as usize]);
                    raw[1 + x] = v;
                }
            }
            2 => {}
            3 => {
                for x in bpp as usize..len {
                    let v = raw[1 + x].wrapping_add(raw[1 + x - bpp as usize] / 2);
                    raw[1 + x] = v;
                }
            }
            4 => {
                for x in bpp as usize..len {
                    let v = raw[1 + x].wrapping_add(cp_paeth(raw[1 + x - bpp as usize], 0, 0));
                    raw[1 + x] = v;
                }
            }
            _ => return false,
        }
    }
    for y in 1..h as usize {
        let prev_row_start = (y - 1) * (len + 1);
        let row_start = y * (len + 1);
        let (before, after) = raw.split_at_mut(row_start);
        let prev = &before[prev_row_start + 1..prev_row_start + 1 + len];
        let row = &mut after[..len + 1];
        match row[0] {
            0 => {}
            1 => {
                for x in 0..bpp as usize {
                    row[1 + x] = row[1 + x].wrapping_add(0);
                }
                for x in bpp as usize..len {
                    row[1 + x] = row[1 + x].wrapping_add(row[1 + x - bpp as usize]);
                }
            }
            2 => {
                for x in 0..len {
                    row[1 + x] = row[1 + x].wrapping_add(prev[x]);
                }
            }
            3 => {
                for x in 0..bpp as usize {
                    row[1 + x] = row[1 + x].wrapping_add(prev[x] / 2);
                }
                for x in bpp as usize..len {
                    row[1 + x] = row[1 + x].wrapping_add(((row[1 + x - bpp as usize] as u16 + prev[x] as u16) / 2) as u8);
                }
            }
            4 => {
                for x in 0..bpp as usize {
                    row[1 + x] = row[1 + x].wrapping_add(prev[x]);
                }
                for x in bpp as usize..len {
                    row[1 + x] = row[1 + x].wrapping_add(cp_paeth(row[1 + x - bpp as usize], prev[x], prev[x - bpp as usize]));
                }
            }
            _ => return false,
        }
    }
    true
}

fn cp_convert(bpp: i32, w: i32, h: i32, src: &[u8], dst: &mut [cp_pixel_t]) {
    let mut si = 0usize;
    let mut di = 0usize;
    for _ in 0..h {
        si += 1;
        for _ in 0..w {
            match bpp {
                1 => dst[di] = cp_make_pixel(src[si], src[si], src[si]),
                2 => dst[di] = cp_make_pixel_a(src[si], src[si], src[si], src[si + 1]),
                3 => dst[di] = cp_make_pixel(src[si], src[si + 1], src[si + 2]),
                4 => dst[di] = cp_make_pixel_a(src[si], src[si + 1], src[si + 2], src[si + 3]),
                _ => {}
            }
            si += bpp as usize;
            di += 1;
        }
    }
}

fn cp_get_alpha_for_indexed_image(index: usize, trns: Option<&[u8]>) -> u8 {
    match trns {
        None => 255,
        Some(t) if index >= t.len() => 255,
        Some(t) => t[index],
    }
}

fn cp_depalette(w: i32, h: i32, src: &[u8], dst: &mut [cp_pixel_t], plte: &[u8], trns: Option<&[u8]>) {
    let mut si = 0usize;
    let mut di = 0usize;
    for _ in 0..h {
        si += 1;
        for _ in 0..w {
            let c = src[si] as usize;
            let r = plte[c * 3];
            let g = plte[c * 3 + 1];
            let b = plte[c * 3 + 2];
            let a = cp_get_alpha_for_indexed_image(c, trns);
            dst[di] = cp_make_pixel_a(r, g, b, a);
            si += 1;
            di += 1;
        }
    }
}

fn cp_out_size(img: &cp_image_t, bpp: i32) -> i32 {
    (img.w + 1) * img.h * bpp
}

#[unsafe(no_mangle)]
pub extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    let sig = b"\x89PNG\r\n\x1a\n";
    let mut img = cp_image_t::default();
    if png_data.is_null() || png_length < 0 {
        return img;
    }
    let png_slice = unsafe { slice::from_raw_parts(png_data, png_length as usize) };
    let mut png = CpRawPng { p: 0, data: png_slice };
    if png.data.len() < 8 || &png.data[..8] != sig {
        set_error(b"incorrect file signature (is this a png file?)\0");
        return img;
    }
    png.p += 8;
    let ihdr = match cp_chunk(&mut png, b"IHDR", 13) {
        Some(v) => v,
        None => {
            set_error(b"unable to find IHDR chunk\0");
            return img;
        }
    };
    let bit_depth = ihdr[8] as i32;
    let color_type = ihdr[9] as i32;
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
    let w = cp_make32(&ihdr[0..4]) as i32 + 1;
    let h = cp_make32(&ihdr[4..8]) as i32;
    if w < 1 {
        set_error(b"invalid IHDR chunk found, image width was less than 1\0");
        return img;
    }
    if h < 1 {
        set_error(b"invalid IHDR chunk found, image height was less than 1\0");
        return img;
    }
    let pix_bytes = match (w as i64)
        .checked_mul(h as i64)
        .and_then(|v| v.checked_mul(std::mem::size_of::<cp_pixel_t>() as i64))
    {
        Some(v) if v < i32::MAX as i64 => v as usize,
        _ => {
            set_error(b"image too large\0");
            return img;
        }
    };
    img.w = w - 1;
    img.h = h;
    let pix_ptr = unsafe { libc::malloc(pix_bytes) as *mut cp_pixel_t };
    if pix_ptr.is_null() {
        set_error(b"unable to allocate raw image space\0");
        return img;
    }
    img.pix = pix_ptr;
    let compression = ihdr[10];
    let filter = ihdr[11];
    let interlace = ihdr[12];
    if compression != 0 {
        set_error(b"only standard compression DEFLATE is supported\0");
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    if filter != 0 {
        set_error(b"only standard adaptive filtering is supported\0");
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    if interlace != 0 {
        set_error(b"interlacing is not supported\0");
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    let mut first = png.p;
    let plte = {
        let found = cp_find(&mut png, b"PLTE", 0);
        if found.is_none() {
            png.p = first;
        } else {
            first = png.p;
        }
        found
    };
    let trns = {
        let found = cp_find(&mut png, b"tRNS", 0);
        if found.is_none() {
            png.p = first;
        } else {
            first = png.p;
        }
        found
    };
    let mut datalen = 0usize;
    while let Some(idat) = cp_find(&mut png, b"IDAT", 0) {
        datalen += idat.len();
        while let Some(next) = cp_chunk(&mut png, b"IDAT", 0) {
            datalen += next.len();
        }
    }
    png.p = first;
    let mut data = vec![0u8; datalen];
    let mut offset = 0usize;
    if let Some(mut idat) = cp_find(&mut png, b"IDAT", 0) {
        loop {
            let len = idat.len();
            data[offset..offset + len].copy_from_slice(idat);
            offset += len;
            match cp_chunk(&mut png, b"IDAT", 0) {
                Some(next) => idat = next,
                None => break,
            }
        }
    }
    if datalen < 6 {
        set_error(b"corrupt zlib structure in DEFLATE stream\0");
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    if (data[0] & 0x0f) != 0x08 {
        set_error(b"only zlib compression method (RFC 1950) is supported\0");
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    if (data[0] & 0xf0) > 0x70 {
        set_error(b"innapropriate window size detected\0");
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    if (data[1] & 0x20) != 0 {
        set_error(b"preset dictionary is present and not supported\0");
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    if cp_out_size(&img, 4) < 1 || cp_out_size(&img, bpp) < 1 {
        set_error(b"invalid image size found\0");
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    let pix_u8 = unsafe { slice::from_raw_parts_mut(img.pix as *mut u8, pix_bytes) };
    let out_offset = (cp_out_size(&img, 4) - cp_out_size(&img, bpp)) as usize;
    let out_len = pix_bytes - out_offset;
    let out = &mut pix_u8[out_offset..out_offset + out_len];
    if cp_inflate(data[2..datalen - 4].as_ptr() as *mut c_void, (datalen - 6) as c_int, out.as_mut_ptr() as *mut c_void, pix_bytes as c_int) == 0 {
        set_error(b"DEFLATE algorithm failed\0");
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    if !cp_unfilter(img.w, img.h, bpp, out) {
        set_error(b"invalid filter byte found\0");
        unsafe { libc::free(img.pix as *mut c_void) };
        img.pix = ptr::null_mut();
        return img;
    }
    let dst = unsafe { slice::from_raw_parts_mut(img.pix, (img.w * img.h) as usize) };
    if color_type == 3 {
        let plte = match plte {
            Some(v) => v,
            None => {
                set_error(b"color type of indexed requires a PLTE chunk\0");
                unsafe { libc::free(img.pix as *mut c_void) };
                img.pix = ptr::null_mut();
                return img;
            }
        };
        cp_depalette(img.w, img.h, out, dst, plte, trns);
    } else {
        cp_convert(bpp, img.w, img.h, out, dst);
    }
    img
}
