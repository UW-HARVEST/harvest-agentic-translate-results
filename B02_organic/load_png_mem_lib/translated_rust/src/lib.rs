#![allow(non_camel_case_types, non_upper_case_globals, static_mut_refs)]
use std::ffi::c_int;
use std::ptr;

extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn calloc(nmemb: usize, size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }

#[repr(C)]
pub struct cp_image_t { pub w: c_int, pub h: c_int, pub pix: *mut cp_pixel_t }

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> cp_pixel_t { cp_pixel_t { r, g, b, a } }
fn cp_make_pixel(r: u8, g: u8, b: u8) -> cp_pixel_t { cp_pixel_t { r, g, b, a: 0xFF } }

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const u8 = ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 320] = [
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,
    8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,8,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
    9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
    7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,8,8,8,8,8,8,8,8,
    5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,5,
];

#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [16,17,18,0,8,7,9,6,10,5,11,4,12,3,13,2,14,1,15];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 31] = [0,0,0,0,0,0,0,0,1,1,1,1,2,2,2,2,3,3,3,3,4,4,4,4,5,5,5,5,0,0,0];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 31] = [3,4,5,6,7,8,9,10,11,13,15,17,19,23,27,31,35,43,51,59,67,83,99,115,131,163,195,227,258,0,0];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 32] = [0,0,0,0,1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10,11,11,12,12,13,13,0,0];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 32] = [1,2,3,4,5,7,9,13,17,25,33,49,65,97,129,193,257,385,513,769,1025,1537,2049,3073,4097,6145,8193,12289,16385,24577,0,0];

struct CpState {
    bits: u64, count: i32, words: *const u32, word_count: i32, word_index: i32,
    bits_left: i32, final_word_available: i32, final_word: u32,
    out: *mut u8, out_end: *const u8, begin: *const u8,
    lookup: [u16; 512], lit: [u32; 288], dst: [u32; 32], len: [u32; 19],
    nlit: u32, ndst: u32, nlen: u32,
}

impl CpState {
    unsafe fn peak_bits(&mut self, num_bits_to_read: i32) -> u64 {
        if self.count < num_bits_to_read {
            if self.word_index < self.word_count {
                let word = *self.words.offset(self.word_index as isize);
                self.word_index += 1;
                self.bits |= (word as u64) << self.count;
                self.count += 32;
            } else if self.final_word_available != 0 {
                self.bits |= (self.final_word as u64) << self.count;
                self.count += self.bits_left;
                self.final_word_available = 0;
            }
        }
        self.bits
    }
    fn consume_bits(&mut self, n: i32) -> u32 {
        let bits = (self.bits & ((1u64 << n) - 1)) as u32;
        self.bits >>= n; self.count -= n; self.bits_left -= n;
        bits
    }
    unsafe fn read_bits(&mut self, n: i32) -> u32 {
        self.peak_bits(n);
        self.consume_bits(n)
    }
    unsafe fn ptr(&self) -> *const u8 {
        (self.words.offset(self.word_index as isize) as *const u8).offset(-(self.count / 8) as isize)
    }
}

fn cp_rev16(a: u32) -> u32 {
    let a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    let a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    let a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8)
}

unsafe fn cp_build(s: *mut CpState, tree: *mut u32, lens: *const u8, sym_count: i32) -> i32 {
    let mut counts = [0i32; 16]; let mut codes = [0i32; 16]; let mut first = [0i32; 16];
    for n in 0..sym_count as usize { counts[*lens.add(n) as usize] += 1; }
    counts[0] = 0; codes[0] = 0; first[0] = 0;
    for n in 1..=15 { codes[n] = (codes[n-1] + counts[n-1]) << 1; first[n] = first[n-1] + counts[n-1]; }
    if !s.is_null() { ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, 512); }
    for i in 0..sym_count as usize {
        let len = *lens.add(i) as i32;
        if len != 0 {
            let code = codes[len as usize]; codes[len as usize] += 1;
            let slot = first[len as usize]; first[len as usize] += 1;
            *tree.add(slot as usize) = ((code as u32) << (32 - len)) | ((i as u32) << 4) | len as u32;
            if !s.is_null() && len <= 9 {
                let mut j = (cp_rev16(code as u32) >> (16 - len)) as usize;
                while j < 512 { (*s).lookup[j] = ((len << 9) | i as i32) as u16; j += 1 << len; }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut CpState) -> bool {
    s.read_bits(s.count & 7);
    let len = s.read_bits(16) as u16;
    let nlen = s.read_bits(16) as u16;
    if len != !nlen { cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr(); return false; }
    if !(s.bits_left / 8 <= len as i32) { cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr(); return false; }
    let p = s.ptr();
    ptr::copy_nonoverlapping(p, s.out, len as usize);
    s.out = s.out.add(len as usize);
    true
}

unsafe fn cp_fixed(s: &mut CpState) {
    s.nlit = cp_build(s, s.lit.as_mut_ptr(), cp_fixed_table.as_ptr(), 288) as u32;
    s.ndst = cp_build(ptr::null_mut(), s.dst.as_mut_ptr(), cp_fixed_table.as_ptr().add(288), 32) as u32;
}

unsafe fn cp_decode(s: &mut CpState, tree: *const u32, hi: i32) -> i32 {
    let bits = s.peak_bits(16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let (mut lo, mut hi) = (0i32, hi);
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.offset(guess as isize) { hi = guess; } else { lo = guess + 1; }
    }
    let key = *tree.offset((lo - 1) as isize);
    s.consume_bits((key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
}

unsafe fn cp_dynamic(s: &mut CpState) {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + s.read_bits(5) as i32;
    let ndst = 1 + s.read_bits(5) as i32;
    let nlen = 4 + s.read_bits(4) as i32;
    for i in 0..nlen as usize { lenlens[cp_permutation_order[i] as usize] = s.read_bits(3) as u8; }
    s.nlen = cp_build(ptr::null_mut(), s.len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;
    let mut lens = [0u8; 320];
    let mut n = 0i32;
    while n < nlit + ndst {
        let sym = cp_decode(s, s.len.as_ptr(), s.nlen as i32);
        match sym {
            16 => { let c = 3 + s.read_bits(2) as i32; for _ in 0..c { lens[n as usize] = lens[(n-1) as usize]; n += 1; } }
            17 => { let c = 3 + s.read_bits(3) as i32; for _ in 0..c { lens[n as usize] = 0; n += 1; } }
            18 => { let c = 11 + s.read_bits(7) as i32; for _ in 0..c { lens[n as usize] = 0; n += 1; } }
            _ => { lens[n as usize] = sym as u8; n += 1; }
        }
    }
    s.nlit = cp_build(s, s.lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
    s.ndst = cp_build(ptr::null_mut(), s.dst.as_mut_ptr(), lens.as_ptr().add(nlit as usize), ndst) as u32;
}

unsafe fn cp_block(s: &mut CpState) -> bool {
    loop {
        let sym = cp_decode(s, s.lit.as_ptr(), s.nlit as i32);
        if sym < 256 {
            if !(s.out.add(1) as *const u8 <= s.out_end) { cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr(); return false; }
            *s.out = sym as u8; s.out = s.out.add(1);
        } else if sym > 256 {
            let sym = sym - 257;
            let length = s.read_bits(cp_len_extra_bits[sym as usize] as i32) as i32 + cp_len_base[sym as usize] as i32;
            let dsym = cp_decode(s, s.dst.as_ptr(), s.ndst as i32);
            let back = s.read_bits(cp_dist_extra_bits[dsym as usize] as i32) as i32 + cp_dist_base[dsym as usize] as i32;
            if !(s.out.offset(-(back as isize)) as *const u8 >= s.begin) { cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr(); return false; }
            if !(s.out.add(length as usize) as *const u8 <= s.out_end) { cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr(); return false; }
            let mut src = s.out.offset(-(back as isize));
            let mut dst = s.out;
            s.out = s.out.add(length as usize);
            if back == 1 { ptr::write_bytes(dst, *src, length as usize); }
            else { for _ in 0..length { *dst = *src; dst = dst.add(1); src = src.add(1); } }
        } else { break; }
    }
    true
}

unsafe fn cp_inflate(in_ptr: *mut u8, in_bytes: i32, out_ptr: *mut u8, out_bytes: i32) -> bool {
    let s = &mut *(calloc(1, std::mem::size_of::<CpState>()) as *mut CpState);
    s.bits_left = in_bytes * 8;
    let first_bytes = ((((in_ptr as usize) + 3) & !3) - in_ptr as usize) as i32;
    s.words = in_ptr.add(first_bytes as usize) as *const u32;
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes { s.bits |= (*in_ptr.add(i as usize) as u64) << (i * 8); }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    for i in 0..last_bytes { s.final_word |= (*in_ptr.add((in_bytes - last_bytes + i) as usize) as u32) << (i * 8); }
    s.count = first_bytes * 8;
    s.out = out_ptr; s.out_end = out_ptr.add(out_bytes as usize); s.begin = out_ptr;
    loop {
        let bfinal = s.read_bits(1);
        let btype = s.read_bits(2);
        let ok = match btype {
            0 => cp_stored(s),
            1 => { cp_fixed(s); cp_block(s) }
            2 => { cp_dynamic(s); cp_block(s) }
            3 => { cp_error_reason = b"Detected unknown block type within input stream.\0".as_ptr(); false }
            _ => false,
        };
        if !ok { free(s as *mut CpState as *mut u8); return false; }
        if bfinal != 0 { break; }
    }
    free(s as *mut CpState as *mut u8);
    true
}

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = a as i32 + b as i32 - c as i32;
    let (pa, pb, pc) = ((p - a as i32).abs(), (p - b as i32).abs(), (p - c as i32).abs());
    if pa <= pb && pa <= pc { a } else if pb <= pc { b } else { c }
}

struct CpRawPng { p: *const u8, end: *const u8 }

fn cp_make32(s: *const u8) -> u32 {
    unsafe { ((*s as u32) << 24) | ((*s.add(1) as u32) << 16) | ((*s.add(2) as u32) << 8) | *s.add(3) as u32 }
}

unsafe fn cp_chunk(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    let len = cp_make32(png.p);
    let start = png.p;
    if std::slice::from_raw_parts(start.add(4), 4) == chunk && len >= minlen {
        let offset = len as usize + 12;
        if start.add(offset) <= png.end { png.p = start.add(offset); return start.add(8); }
    }
    ptr::null()
}

unsafe fn cp_find(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    while png.p < png.end {
        let len = cp_make32(png.p);
        let start = png.p;
        png.p = start.add(len as usize + 12);
        if std::slice::from_raw_parts(start.add(4), 4) == chunk && len >= minlen && png.p <= png.end {
            return start.add(8);
        }
    }
    ptr::null()
}

unsafe fn cp_unfilter(w: i32, h: i32, bpp: i32, raw: *mut u8) -> bool {
    let len = w * bpp;
    let mut raw = raw;
    if h > 0 {
        let f = *raw; raw = raw.add(1);
        match f {
            0 => {}
            1 => { for x in bpp..len { *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*raw.add((x-bpp) as usize)); } }
            2 => {}
            3 => { for x in bpp..len { *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*raw.add((x-bpp) as usize) / 2); } }
            4 => { for x in bpp..len { *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(cp_paeth(*raw.add((x-bpp) as usize), 0, 0)); } }
            _ => return false,
        }
    }
    let mut prev = raw;
    raw = raw.add(len as usize);
    for _y in 1..h {
        let f = *raw; raw = raw.add(1);
        match f {
            0 => {}
            1 => {
                for x in 0..bpp { *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(0); }
                for x in bpp..len { *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*raw.add((x-bpp) as usize)); }
            }
            2 => {
                for x in 0..len { *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize)); }
            }
            3 => {
                for x in 0..bpp { *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize) / 2); }
                for x in bpp..len { *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(((*raw.add((x-bpp) as usize) as u16 + *prev.add(x as usize) as u16) / 2) as u8); }
            }
            4 => {
                for x in 0..bpp { *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(*prev.add(x as usize)); }
                for x in bpp..len { *raw.add(x as usize) = (*raw.add(x as usize)).wrapping_add(cp_paeth(*raw.add((x-bpp) as usize), *prev.add(x as usize), *prev.add((x-bpp) as usize))); }
            }
            _ => return false,
        }
        prev = raw;
        raw = raw.add(len as usize);
    }
    true
}

unsafe fn cp_convert(bpp: i32, w: i32, h: i32, mut src: *mut u8, mut dst: *mut cp_pixel_t) {
    for _y in 0..h {
        src = src.add(1);
        for _x in 0..w {
            *dst = match bpp {
                1 => cp_make_pixel(*src, *src, *src),
                2 => cp_make_pixel_a(*src, *src, *src, *src.add(1)),
                3 => cp_make_pixel(*src, *src.add(1), *src.add(2)),
                4 => cp_make_pixel_a(*src, *src.add(1), *src.add(2), *src.add(3)),
                _ => cp_pixel_t { r: 0, g: 0, b: 0, a: 0 },
            };
            src = src.add(bpp as usize); dst = dst.add(1);
        }
    }
}

fn cp_get_alpha_for_indexed(index: i32, trns: *const u8, trns_len: u32) -> u8 {
    if trns.is_null() { 255 }
    else if (index as u32) >= trns_len { 255 }
    else { unsafe { *trns.add(index as usize) } }
}

unsafe fn cp_depalette(w: i32, h: i32, mut src: *mut u8, mut dst: *mut cp_pixel_t, plte: *const u8, trns: *const u8, trns_len: u32) {
    for _y in 0..h {
        src = src.add(1);
        for _x in 0..w {
            let c = *src as usize;
            *dst = cp_make_pixel_a(*plte.add(c*3), *plte.add(c*3+1), *plte.add(c*3+2), cp_get_alpha_for_indexed(c as i32, trns, trns_len));
            src = src.add(1); dst = dst.add(1);
        }
    }
}

fn cp_get_chunk_byte_length(chunk: *const u8) -> u32 { cp_make32(unsafe { chunk.sub(8) }) }
fn cp_out_size(img: &cp_image_t, bpp: i32) -> i32 { (img.w + 1) * img.h * bpp }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn load_png_mem(png_data: *const u8, png_length: c_int) -> cp_image_t {
    let sig = b"\x89PNG\r\n\x1a\n";
    let mut img = cp_image_t { w: 0, h: 0, pix: ptr::null_mut() };
    let mut data: *mut u8 = ptr::null_mut();

    macro_rules! err {
        ($reason:expr) => {{ cp_error_reason = $reason.as_ptr(); err_cleanup(&mut data, &mut img); return img; }};
    }
    macro_rules! check {
        ($cond:expr, $reason:expr) => { if !($cond) { err!($reason); } };
    }

    let mut png = CpRawPng { p: png_data, end: png_data.add(png_length as usize) };

    check!(std::slice::from_raw_parts(png.p, 8) == sig, b"incorrect file signature (is this a png file?)\0");
    png.p = png.p.add(8);

    let ihdr = cp_chunk(&mut png, b"IHDR", 13);
    check!(!ihdr.is_null(), b"unable to find IHDR chunk\0");

    let bit_depth = *ihdr.add(8);
    let color_type = *ihdr.add(9);
    check!(bit_depth == 8, b"only bit-depth of 8 is supported\0");

    let bpp: i32 = match color_type {
        0 => 1, 2 => 3, 3 => 1, 4 => 2, 6 => 4,
        _ => { err!(b"unknown color type\0"); }
    };

    let w = cp_make32(ihdr) as i32 + 1;
    let h = cp_make32(ihdr.add(4)) as i32;
    check!(w >= 1, b"invalid IHDR chunk found, image width was less than 1\0");
    check!(h >= 1, b"invalid IHDR chunk found, image height was less than 1\0");
    check!((w as i64 * h as i64 * std::mem::size_of::<cp_pixel_t>() as i64) < i32::MAX as i64, b"image too large\0");

    let pix_bytes = w as usize * h as usize * std::mem::size_of::<cp_pixel_t>();
    img.w = w - 1;
    img.h = h;
    img.pix = malloc(pix_bytes) as *mut cp_pixel_t;
    check!(!img.pix.is_null(), b"unable to allocate raw image space\0");

    check!(*ihdr.add(10) == 0, b"only standard compression DEFLATE is supported\0");
    check!(*ihdr.add(11) == 0, b"only standard adaptive filtering is supported\0");
    check!(*ihdr.add(12) == 0, b"interlacing is not supported\0");

    let first_saved = png.p;
    let plte = cp_find(&mut png, b"PLTE", 0);
    let first_saved = if plte.is_null() { png.p = first_saved; first_saved } else { png.p };
    let trns = cp_find(&mut png, b"tRNS", 0);
    let first_saved = if trns.is_null() { png.p = first_saved; first_saved } else { png.p };

    // Calculate total IDAT length
    let mut datalen: i32 = 0;
    let mut idat = cp_find(&mut png, b"IDAT", 0);
    while !idat.is_null() { datalen += cp_get_chunk_byte_length(idat) as i32; idat = cp_chunk(&mut png, b"IDAT", 0); }

    // Collect IDAT data
    png.p = first_saved;
    data = malloc(datalen as usize);
    let mut offset = 0i32;
    idat = cp_find(&mut png, b"IDAT", 0);
    while !idat.is_null() {
        let len = cp_get_chunk_byte_length(idat);
        ptr::copy_nonoverlapping(idat, data.add(offset as usize), len as usize);
        offset += len as i32;
        idat = cp_chunk(&mut png, b"IDAT", 0);
    }

    check!(!data.is_null() && datalen >= 6, b"corrupt zlib structure in DEFLATE stream\0");
    check!((*data & 0x0f) == 0x08, b"only zlib compression method (RFC 1950) is supported\0");
    check!((*data & 0xf0) <= 0x70, b"innapropriate window size detected\0");
    check!((*data.add(1) & 0x20) == 0, b"preset dictionary is present and not supported\0");
    check!(cp_out_size(&img, 4) >= 1, b"invalid image size found\0");
    check!(cp_out_size(&img, bpp) >= 1, b"invalid image size found\0");

    let out = (img.pix as *mut u8).add(cp_out_size(&img, 4) as usize).sub(cp_out_size(&img, bpp) as usize);

    check!(cp_inflate(data.add(2), datalen - 6, out, pix_bytes as i32), b"DEFLATE algorithm failed\0");
    check!(cp_unfilter(img.w, img.h, bpp, out), b"invalid filter byte found\0");

    if color_type == 3 {
        check!(!plte.is_null(), b"color type of indexed requires a PLTE chunk\0");
        let trns_len = if trns.is_null() { 0 } else { cp_get_chunk_byte_length(trns) };
        cp_depalette(img.w, img.h, out, img.pix, plte, trns, trns_len);
    } else {
        cp_convert(bpp, img.w, img.h, out, img.pix);
    }

    free(data);
    img
}

unsafe fn err_cleanup(data: &mut *mut u8, img: &mut cp_image_t) {
    free(*data); *data = ptr::null_mut();
    free(img.pix as *mut u8); img.pix = ptr::null_mut();
}
