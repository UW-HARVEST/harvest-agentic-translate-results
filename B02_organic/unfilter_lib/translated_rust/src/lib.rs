use std::ffi::c_int;
use std::ptr;

// ============ Exported global data ============

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const u8 = ptr::null();

#[unsafe(no_mangle)]
pub static cp_fixed_table: [u8; 320] = [
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
pub static cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static cp_len_extra_bits: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static cp_len_base: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67,
    83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static cp_dist_extra_bits: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];

#[unsafe(no_mangle)]
pub static cp_dist_base: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

// ============ cp_inflate internals ============

struct CpState {
    bits: u64,
    count: i32,
    words: *const u32,
    word_count: i32,
    word_index: i32,
    bits_left: i32,
    final_word_available: i32,
    final_word: u32,
    out: *mut u8,
    out_end: *const u8,
    begin: *const u8,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

fn cp_would_overflow(s: &CpState, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

unsafe fn cp_ptr(s: &CpState) -> *const u8 {
    unsafe {
        (s.words.offset(s.word_index as isize) as *const u8).offset(-(s.count / 8) as isize)
    }
}

unsafe fn cp_peak_bits(s: &mut CpState, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            unsafe {
                let word = *s.words.offset(s.word_index as isize);
                s.bits |= (word as u64) << s.count;
                s.count += 32;
                s.word_index += 1;
            }
        } else if s.final_word_available != 0 {
            let word = s.final_word;
            s.bits |= (word as u64) << s.count;
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    let bits = (s.bits & (((1u64) << num_bits_to_read) - 1)) as u32;
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

unsafe fn cp_read_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    unsafe { cp_peak_bits(s, num_bits_to_read) };
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(a: u32) -> u32 {
    let a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    let a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    let a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8)
}

unsafe fn cp_build(
    s: *mut CpState,
    tree: *mut u32,
    lens: *const u8,
    sym_count: i32,
) -> i32 {
    let mut counts = [0i32; 16];
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];

    for n in 0..sym_count {
        unsafe {
            counts[*lens.offset(n as isize) as usize] += 1;
        }
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }

    if !s.is_null() {
        unsafe {
            ptr::write_bytes((*s).lookup.as_mut_ptr(), 0, 1 << 9);
        }
    }

    for i in 0..sym_count {
        unsafe {
            let l = *lens.offset(i as isize) as usize;
            if l != 0 {
                let code = codes[l];
                codes[l] += 1;
                let slot = first[l];
                first[l] += 1;
                *tree.offset(slot as isize) =
                    ((code as u32) << (32 - l)) | ((i as u32) << 4) | (l as u32);
                if !s.is_null() && l <= 9 {
                    let mut j = (cp_rev16(code as u32) >> (16 - l)) as i32;
                    while j < (1 << 9) {
                        *(*s).lookup.as_mut_ptr().offset(j as isize) =
                            ((l << 9) | i as usize) as u16;
                        j += 1 << l;
                    }
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut CpState) -> bool {
    unsafe {
        cp_read_bits(s, s.count & 7);
        let len = cp_read_bits(s, 16) as u16;
        let nlen = cp_read_bits(s, 16) as u16;
        if len != !nlen {
            cp_error_reason = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr();
            return false;
        }
        if !(s.bits_left / 8 <= len as i32) {
            cp_error_reason = b"Stored block extends beyond end of input stream.\0".as_ptr();
            return false;
        }
        let p = cp_ptr(s);
        ptr::copy_nonoverlapping(p, s.out, len as usize);
        s.out = s.out.add(len as usize);
        true
    }
}

unsafe fn cp_fixed(s: &mut CpState) {
    unsafe {
        let sp = s as *mut CpState;
        s.nlit = cp_build(
            sp,
            s.lit.as_mut_ptr(),
            cp_fixed_table.as_ptr(),
            288,
        ) as u32;
        s.ndst = cp_build(
            ptr::null_mut(),
            s.dst.as_mut_ptr(),
            cp_fixed_table.as_ptr().add(288),
            32,
        ) as u32;
    }
}

unsafe fn cp_decode(s: &mut CpState, tree: *const u32, hi: i32) -> i32 {
    unsafe {
        let bits = cp_peak_bits(s, 16);
        let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
        let mut lo = 0i32;
        let mut hi = hi;
        while lo < hi {
            let guess = (lo + hi) >> 1;
            if search < *tree.offset(guess as isize) {
                hi = guess;
            } else {
                lo = guess + 1;
            }
        }
        let key = *tree.offset((lo - 1) as isize);
        let _code = cp_consume_bits(s, (key & 0xF) as i32);
        ((key >> 4) & 0xFFF) as i32
    }
}

unsafe fn cp_dynamic(s: &mut CpState) -> bool {
    unsafe {
        let mut lenlens = [0u8; 19];
        let nlit = 257 + cp_read_bits(s, 5) as i32;
        let ndst = 1 + cp_read_bits(s, 5) as i32;
        let nlen = 4 + cp_read_bits(s, 4) as i32;
        for i in 0..nlen {
            lenlens[cp_permutation_order[i as usize] as usize] =
                cp_read_bits(s, 3) as u8;
        }
        s.nlen = cp_build(ptr::null_mut(), s.len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;

        let mut lens = [0u8; 320];
        let mut n = 0i32;
        while n < nlit + ndst {
            let sym = cp_decode(s, s.len.as_ptr(), s.nlen as i32);
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
        let sp = s as *mut CpState;
        s.nlit = cp_build(
            sp,
            s.lit.as_mut_ptr(),
            lens.as_ptr(),
            nlit,
        ) as u32;
        s.ndst = cp_build(
            ptr::null_mut(),
            s.dst.as_mut_ptr(),
            lens.as_ptr().add(nlit as usize),
            ndst,
        ) as u32;
        true
    }
}

unsafe fn cp_block(s: &mut CpState) -> bool {
    unsafe {
        loop {
            let symbol = cp_decode(s, s.lit.as_ptr(), s.nlit as i32);
            if symbol < 256 {
                if s.out.add(1) > s.out_end as *mut u8 {
                    cp_error_reason = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr();
                    return false;
                }
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            } else if symbol > 256 {
                let sym = symbol - 257;
                let length = cp_read_bits(s, cp_len_extra_bits[sym as usize] as i32) as i32
                    + cp_len_base[sym as usize] as i32;
                let distance_symbol = cp_decode(s, s.dst.as_ptr(), s.ndst as i32);
                let backwards_distance =
                    cp_read_bits(s, cp_dist_extra_bits[distance_symbol as usize] as i32) as i32
                        + cp_dist_base[distance_symbol as usize] as i32;
                if (s.out as *const u8).offset(-(backwards_distance as isize)) < s.begin {
                    cp_error_reason = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr();
                    return false;
                }
                if s.out.add(length as usize) > s.out_end as *mut u8 {
                    cp_error_reason = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr();
                    return false;
                }
                let mut src = s.out.offset(-(backwards_distance as isize));
                let dst_start = s.out;
                s.out = s.out.add(length as usize);
                if backwards_distance == 1 {
                    ptr::write_bytes(dst_start, *src, length as usize);
                } else {
                    let mut dst = dst_start;
                    let mut remaining = length;
                    while remaining > 0 {
                        *dst = *src;
                        dst = dst.add(1);
                        src = src.add(1);
                        remaining -= 1;
                    }
                }
            } else {
                break;
            }
        }
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cp_inflate(
    in_ptr: *const u8,
    in_bytes: c_int,
    out_ptr: *mut u8,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        let layout = std::alloc::Layout::new::<CpState>();
        let s_ptr = std::alloc::alloc_zeroed(layout) as *mut CpState;
        if s_ptr.is_null() {
            return 0;
        }
        let s = &mut *s_ptr;
        s.bits = 0;
        s.count = 0;
        s.word_index = 0;
        s.bits_left = in_bytes * 8;

        let first_bytes =
            ((((in_ptr as usize) + 3) & !3) - (in_ptr as usize)) as i32;
        s.words = in_ptr.add(first_bytes as usize) as *const u32;
        s.word_count = (in_bytes - first_bytes) / 4;
        let last_bytes = (in_bytes - first_bytes) & 3;

        for i in 0..first_bytes {
            s.bits |= (*in_ptr.add(i as usize) as u64) << (i * 8);
        }
        s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
        s.final_word = 0;
        for i in 0..last_bytes {
            s.final_word |=
                (*in_ptr.add((in_bytes - last_bytes + i) as usize) as u32) << (i * 8);
        }
        s.count = first_bytes * 8;

        s.out = out_ptr;
        s.out_end = out_ptr.add(out_bytes as usize);
        s.begin = out_ptr;

        loop {
            let bfinal = cp_read_bits(s, 1);
            let btype = cp_read_bits(s, 2);
            match btype {
                0 => {
                    if !cp_stored(s) {
                        std::alloc::dealloc(s_ptr as *mut u8, layout);
                        return 0;
                    }
                }
                1 => {
                    cp_fixed(s);
                    if !cp_block(s) {
                        std::alloc::dealloc(s_ptr as *mut u8, layout);
                        return 0;
                    }
                }
                2 => {
                    if !cp_dynamic(s) {
                        std::alloc::dealloc(s_ptr as *mut u8, layout);
                        return 0;
                    }
                    if !cp_block(s) {
                        std::alloc::dealloc(s_ptr as *mut u8, layout);
                        return 0;
                    }
                }
                3 => {
                    cp_error_reason = b"Detected unknown block type within input stream.\0".as_ptr();
                    std::alloc::dealloc(s_ptr as *mut u8, layout);
                    return 0;
                }
                _ => unreachable!(),
            }
            if bfinal != 0 {
                break;
            }
        }
        std::alloc::dealloc(s_ptr as *mut u8, layout);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, raw: *mut u8) -> c_int {
    let len = w * bpp;
    let mut raw = raw;
    let mut x: c_int;

    if h > 0 {
        let filter = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter {
            0 => {}
            1 => {
                x = bpp;
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*raw.offset((x - bpp) as isize));
                    }
                    x += 1;
                }
            }
            2 => {}
            3 => {
                x = bpp;
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add((*raw.offset((x - bpp) as isize)) / 2);
                    }
                    x += 1;
                }
            }
            4 => {
                x = bpp;
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) = (*raw.offset(x as isize))
                            .wrapping_add(cp_paeth(*raw.offset((x - bpp) as isize), 0, 0));
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }
    }

    let mut prev = raw;
    raw = unsafe { raw.offset(len as isize) };

    let mut y = 1;
    while y < h {
        let filter = unsafe { *raw };
        raw = unsafe { raw.add(1) };
        match filter {
            0 => {}
            1 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(0);
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*raw.offset((x - bpp) as isize));
                    }
                    x += 1;
                }
            }
            2 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                    }
                    x += 1;
                }
            }
            3 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add((*prev.offset(x as isize)) / 2);
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(
                            ((*raw.offset((x - bpp) as isize) as u16
                                + *prev.offset(x as isize) as u16)
                                / 2) as u8,
                        );
                    }
                    x += 1;
                }
            }
            4 => {
                x = 0;
                while x < bpp {
                    unsafe {
                        *raw.offset(x as isize) =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                    }
                    x += 1;
                }
                while x < len {
                    unsafe {
                        *raw.offset(x as isize) = (*raw.offset(x as isize)).wrapping_add(
                            cp_paeth(
                                *raw.offset((x - bpp) as isize),
                                *prev.offset(x as isize),
                                *prev.offset((x - bpp) as isize),
                            ),
                        );
                    }
                    x += 1;
                }
            }
            _ => return 0,
        }

        prev = raw;
        raw = unsafe { raw.offset(len as isize) };
        y += 1;
    }

    1
}
