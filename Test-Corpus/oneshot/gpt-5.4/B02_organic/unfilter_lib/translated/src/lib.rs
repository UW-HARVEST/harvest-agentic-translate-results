use std::ffi::c_void;
use std::os::raw::c_int;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct CpPixel {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[repr(C)]
struct CpImage {
    w: c_int,
    h: c_int,
    pix: *mut CpPixel,
}

fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> CpPixel {
    CpPixel { r, g, b, a }
}

fn cp_make_pixel(r: u8, g: u8, b: u8) -> CpPixel {
    CpPixel { r, g, b, a: 0xFF }
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

struct CpState {
    bits: u64,
    count: c_int,
    words: Vec<u32>,
    word_count: c_int,
    word_index: c_int,
    bits_left: c_int,
    final_word_available: bool,
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
    input_base: *const u8,
    input_len: usize,
    ptr_offset: isize,
}

fn set_error_reason(msg: &'static [u8]) {
    CP_ERROR_REASON.store(msg.as_ptr() as *mut u8, Ordering::Relaxed);
}

fn cp_would_overflow(s: &CpState, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

fn cp_ptr(s: &CpState) -> *const u8 {
    assert_eq!(s.bits_left & 7, 0);
    let off = s.ptr_offset - (s.count / 8) as isize;
    unsafe { s.input_base.offset(off) }
}

fn cp_peak_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
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

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    assert!(s.count >= num_bits_to_read);
    let bits = (s.bits & (((1u64) << num_bits_to_read) - 1)) as u32;
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
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

fn cp_build(s: Option<&mut CpState>, tree: &mut [u32], lens: &[u8], sym_count: usize) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    for &len in &lens[..sym_count] {
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
        let _ = state;
    }
    let mut s = s;
    if let Some(state) = s.as_mut() {
        state.lookup.fill(0);
    }
    for (i, &len_u8) in lens[..sym_count].iter().enumerate() {
        let len = len_u8 as usize;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | len as u32;
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

fn cp_stored(s: &mut CpState) -> bool {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        set_error_reason(b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0");
        return false;
    }
    if s.bits_left / 8 > len as c_int {
        set_error_reason(b"Stored block extends beyond end of input stream.\0");
        return false;
    }
    let p = cp_ptr(s);
    unsafe {
        ptr::copy_nonoverlapping(p, s.out, len as usize);
        s.out = s.out.add(len as usize);
    }
    true
}

fn cp_fixed(s: &mut CpState) -> bool {
    s.nlit = cp_build(Some(s), &mut [0; 0], &[], 0) as u32;
    s.nlit = cp_build(Some(s), &mut s.lit, &CP_FIXED_TABLE, 288) as u32;
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32) as u32;
    true
}

fn cp_decode(s: &mut CpState, tree: &[u32], hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = ((cp_rev16(bits as u32) << 16) | 0xFFFF) as u32;
    let mut lo = 0i32;
    let mut hi_mut = hi;
    while lo < hi_mut {
        let guess = (lo + hi_mut) >> 1;
        if search < tree[guess as usize] {
            hi_mut = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = tree[(lo - 1) as usize];
    let len = 32 - (key & 0xF);
    assert_eq!(search >> len, key >> len);
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

fn cp_dynamic(s: &mut CpState) -> bool {
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
        let sym = cp_decode(s, &s.len, s.nlen as c_int);
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
    s.nlit = cp_build(Some(s), &mut s.lit, &lens, nlit) as u32;
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit..], ndst) as u32;
    true
}

fn cp_block(s: &mut CpState) -> bool {
    loop {
        let symbol = cp_decode(s, &s.lit, s.nlit as c_int);
        if symbol < 256 {
            unsafe {
                if s.out.add(1) > s.out_end {
                    set_error_reason(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                    return false;
                }
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let symbol = (symbol - 257) as usize;
            let length = cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol] as c_int) + CP_LEN_BASE[symbol];
            let distance_symbol = cp_decode(s, &s.dst, s.ndst as c_int) as usize;
            let backwards_distance = cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol] as c_int) + CP_DIST_BASE[distance_symbol];
            unsafe {
                if s.out.offset(-(backwards_distance as isize)) < s.begin {
                    set_error_reason(b"Attempted to write before out buffer (invalid backwards distance).\0");
                    return false;
                }
                if s.out.add(length as usize) > s.out_end {
                    set_error_reason(b"Attempted to overwrite out buffer while outputting a string.\0");
                    return false;
                }
                let src = s.out.offset(-(backwards_distance as isize));
                let mut dst = s.out;
                s.out = s.out.add(length as usize);
                match backwards_distance {
                    1 => ptr::write_bytes(dst, *src, length as usize),
                    _ => {
                        let mut remaining = length as usize;
                        let mut srcp = src;
                        while remaining > 0 {
                            *dst = *srcp;
                            dst = dst.add(1);
                            srcp = srcp.add(1);
                            remaining -= 1;
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
pub extern "C" fn cp_inflate(in_ptr: *mut c_void, in_bytes: c_int, out_ptr: *mut c_void, out_bytes: c_int) -> c_int {
    if in_ptr.is_null() || out_ptr.is_null() || in_bytes < 0 || out_bytes < 0 {
        return 0;
    }

    let input = unsafe { std::slice::from_raw_parts(in_ptr as *const u8, in_bytes as usize) };
    let in_addr = in_ptr as usize;
    let first_bytes = (((in_addr + 3) & !3) - in_addr) as usize;
    let first_bytes = first_bytes.min(input.len());
    let aligned_start = first_bytes;
    let remaining = input.len().saturating_sub(aligned_start);
    let word_count = remaining / 4;
    let last_bytes = remaining & 3;

    let mut words = Vec::with_capacity(word_count);
    for chunk in input[aligned_start..aligned_start + word_count * 4].chunks_exact(4) {
        words.push(u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }

    let mut bits = 0u64;
    for (i, b) in input[..first_bytes].iter().enumerate() {
        bits |= (*b as u64) << (i * 8);
    }

    let mut final_word = 0u32;
    for i in 0..last_bytes {
        final_word |= (input[input.len() - last_bytes + i] as u32) << (i * 8);
    }

    let mut s = CpState {
        bits,
        count: (first_bytes * 8) as c_int,
        words,
        word_count: word_count as c_int,
        word_index: 0,
        bits_left: in_bytes * 8,
        final_word_available: last_bytes != 0,
        final_word,
        out: out_ptr as *mut u8,
        out_end: unsafe { (out_ptr as *mut u8).add(out_bytes as usize) },
        begin: out_ptr as *mut u8,
        lookup: [0; 1 << 9],
        lit: [0; 288],
        dst: [0; 32],
        len: [0; 19],
        nlit: 0,
        ndst: 0,
        nlen: 0,
        input_base: input.as_ptr(),
        input_len: input.len(),
        ptr_offset: aligned_start as isize,
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

#[repr(C)]
struct CpRawPng {
    p: *const u8,
    end: *const u8,
}

fn cp_make32(s: &[u8]) -> u32 {
    ((s[0] as u32) << 24) | ((s[1] as u32) << 16) | ((s[2] as u32) << 8) | s[3] as u32
}

fn cp_chunk(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    unsafe {
        let remaining = png.end.offset_from(png.p) as usize;
        if remaining < 8 {
            return ptr::null();
        }
        let head = std::slice::from_raw_parts(png.p, remaining);
        let len = cp_make32(&head[0..4]);
        let start = png.p;
        if &head[4..8] == chunk && len >= minlen {
            let offset = len as usize + 12;
            if png.p.add(offset) <= png.end {
                png.p = png.p.add(offset);
                return start.add(8);
            }
        }
        ptr::null()
    }
}

fn cp_find(png: &mut CpRawPng, chunk: &[u8; 4], minlen: u32) -> *const u8 {
    unsafe {
        while png.p < png.end {
            let remaining = png.end.offset_from(png.p) as usize;
            if remaining < 8 {
                break;
            }
            let head = std::slice::from_raw_parts(png.p, remaining);
            let len = cp_make32(&head[0..4]);
            let start = png.p;
            png.p = png.p.add(len as usize + 12);
            if &head[4..8] == chunk && len >= minlen && png.p <= png.end {
                return start.add(8);
            }
        }
        ptr::null()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    if raw.is_null() {
        return 0;
    }
    let len = match (w as isize).checked_mul(bpp as isize) {
        Some(v) if v >= 0 => v as usize,
        _ => return 0,
    };
    let h_usize = if h < 0 { return 0; } else { h as usize };
    unsafe {
        let mut rawp = raw;
        if h_usize > 0 {
            match *rawp {
                0 => {
                    rawp = rawp.add(1);
                }
                1 => {
                    rawp = rawp.add(1);
                    for x in bpp as usize..len {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(*rawp.add(x - bpp as usize));
                    }
                }
                2 => {
                    rawp = rawp.add(1);
                }
                3 => {
                    rawp = rawp.add(1);
                    for x in bpp as usize..len {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(*rawp.add(x - bpp as usize) / 2);
                    }
                }
                4 => {
                    rawp = rawp.add(1);
                    for x in bpp as usize..len {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(cp_paeth(*rawp.add(x - bpp as usize), 0, 0));
                    }
                }
                _ => return 0,
            }
        }
        let mut prev = rawp;
        rawp = rawp.add(len);
        for _y in 1..h_usize {
            match *rawp {
                0 => {
                    rawp = rawp.add(1);
                }
                1 => {
                    rawp = rawp.add(1);
                    let mut x = 0usize;
                    while x < bpp as usize {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(0);
                        x += 1;
                    }
                    while x < len {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(*rawp.add(x - bpp as usize));
                        x += 1;
                    }
                }
                2 => {
                    rawp = rawp.add(1);
                    let mut x = 0usize;
                    while x < bpp as usize {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(*prev.add(x));
                        x += 1;
                    }
                    while x < len {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(*prev.add(x));
                        x += 1;
                    }
                }
                3 => {
                    rawp = rawp.add(1);
                    let mut x = 0usize;
                    while x < bpp as usize {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(*prev.add(x) / 2);
                        x += 1;
                    }
                    while x < len {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(((*rawp.add(x - bpp as usize) as u16 + *prev.add(x) as u16) / 2) as u8);
                        x += 1;
                    }
                }
                4 => {
                    rawp = rawp.add(1);
                    let mut x = 0usize;
                    while x < bpp as usize {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(*prev.add(x));
                        x += 1;
                    }
                    while x < len {
                        *rawp.add(x) = (*rawp.add(x)).wrapping_add(cp_paeth(*rawp.add(x - bpp as usize), *prev.add(x), *prev.add(x - bpp as usize)));
                        x += 1;
                    }
                }
                _ => return 0,
            }
            prev = rawp;
            rawp = rawp.add(len);
        }
    }
    1
}
