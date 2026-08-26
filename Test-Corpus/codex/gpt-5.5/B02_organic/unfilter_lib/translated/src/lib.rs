use std::ffi::{c_int, c_uchar, c_void};
use std::ptr;

static mut CP_ERROR_REASON: *const u8 = ptr::null();

const fn fixed_table() -> [u8; 288 + 32] {
    let mut table = [0_u8; 288 + 32];
    let mut i = 0;
    while i < 144 {
        table[i] = 8;
        i += 1;
    }
    while i < 256 {
        table[i] = 9;
        i += 1;
    }
    while i < 280 {
        table[i] = 7;
        i += 1;
    }
    while i < 288 {
        table[i] = 8;
        i += 1;
    }
    while i < 320 {
        table[i] = 5;
        i += 1;
    }
    table
}

static CP_FIXED_TABLE: [u8; 288 + 32] = fixed_table();
static CP_PERMUTATION_ORDER: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];
static CP_LEN_EXTRA_BITS: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];
static CP_LEN_BASE: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];
static CP_DIST_EXTRA_BITS: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];
static CP_DIST_BASE: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

struct CpState {
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
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: u32,
    ndst: u32,
    nlen: u32,
}

impl Default for CpState {
    fn default() -> Self {
        Self {
            bits: 0,
            count: 0,
            words: ptr::null(),
            word_count: 0,
            word_index: 0,
            bits_left: 0,
            final_word_available: 0,
            final_word: 0,
            out: ptr::null_mut(),
            out_end: ptr::null_mut(),
            begin: ptr::null_mut(),
            lookup: [0; 1 << 9],
            lit: [0; 288],
            dst: [0; 32],
            len: [0; 19],
            nlit: 0,
            ndst: 0,
            nlen: 0,
        }
    }
}

fn set_error(reason: &'static [u8]) {
    unsafe {
        CP_ERROR_REASON = reason.as_ptr();
    }
}

unsafe fn cp_ptr(s: &CpState) -> *const u8 {
    unsafe { (s.words.add(s.word_index as usize) as *const u8).offset(-(s.count / 8) as isize) }
}

unsafe fn cp_peak_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { ptr::read(s.words.add(s.word_index as usize)) };
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
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
    let bits = s.bits & ((1_u64 << num_bits_to_read) - 1);
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits as u32
}

unsafe fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    unsafe {
        cp_peak_bits(s, num_bits_to_read);
    }
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

fn cp_build(mut s: Option<&mut CpState>, tree: &mut [u32], lens: &[u8], sym_count: c_int) -> c_int {
    let mut codes = [0_i32; 16];
    let mut first = [0_i32; 16];
    let mut counts = [0_i32; 16];
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
    if let Some(st) = s.as_deref_mut() {
        st.lookup.fill(0);
    }
    for i in 0..sym_count as usize {
        let len = lens[i] as usize;
        if len != 0 {
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | len as u32;
            if let Some(st) = s.as_deref_mut()
                && len <= 9
            {
                let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                while j < (1 << 9) {
                    st.lookup[j] = ((len as u16) << 9) | i as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn cp_stored(s: &mut CpState) -> c_int {
    unsafe {
        cp_read_bits(s, s.count & 7);
        let len = cp_read_bits(s, 16) as u16;
        let nlen = cp_read_bits(s, 16) as u16;
        if len != !nlen {
            set_error(
                b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0",
            );
            return 0;
        }
        if !(s.bits_left / 8 <= len as c_int) {
            set_error(b"Stored block extends beyond end of input stream.\0");
            return 0;
        }
        let p = cp_ptr(s);
        ptr::copy_nonoverlapping(p, s.out, len as usize);
        s.out = s.out.add(len as usize);
    }
    1
}

fn cp_fixed(s: &mut CpState) -> c_int {
    let mut lit = [0_u32; 288];
    let nlit = cp_build(Some(s), &mut lit, &CP_FIXED_TABLE[..288], 288);
    s.lit = lit;
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32) as u32;
    s.nlit = nlit as u32;
    1
}

unsafe fn cp_decode(s: &mut CpState, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = unsafe { cp_peak_bits(s, 16) };
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < unsafe { *tree.add(guess as usize) } {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = unsafe { *tree.add((lo - 1) as usize) };
    let _code = cp_consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn cp_dynamic(s: &mut CpState) -> c_int {
    let mut lenlens = [0_u8; 19];
    let nlit = 257 + unsafe { cp_read_bits(s, 5) } as c_int;
    let ndst = 1 + unsafe { cp_read_bits(s, 5) } as c_int;
    let nlen = 4 + unsafe { cp_read_bits(s, 4) } as c_int;
    for i in 0..nlen as usize {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = unsafe { cp_read_bits(s, 3) } as u8;
    }
    s.nlen = cp_build(None, &mut s.len, &lenlens, 19) as u32;
    let mut lens = [0_u8; 288 + 32];
    let mut n = 0;
    while n < nlit + ndst {
        let tree = s.len.as_ptr();
        let sym = unsafe { cp_decode(s, tree, s.nlen as c_int) };
        match sym {
            16 => {
                let mut i = 3 + unsafe { cp_read_bits(s, 2) } as c_int;
                while i != 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + unsafe { cp_read_bits(s, 3) } as c_int;
                while i != 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + unsafe { cp_read_bits(s, 7) } as c_int;
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
    let mut lit = [0_u32; 288];
    let nlit_built = cp_build(Some(s), &mut lit, &lens, nlit);
    s.lit = lit;
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit as usize..], ndst) as u32;
    s.nlit = nlit_built as u32;
    1
}

unsafe fn cp_block(s: &mut CpState) -> c_int {
    loop {
        let tree = s.lit.as_ptr();
        let symbol = unsafe { cp_decode(s, tree, s.nlit as c_int) };
        if symbol < 256 {
            if unsafe { s.out.add(1) } > s.out_end {
                set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return 0;
            }
            unsafe {
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = unsafe {
                cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol as usize] as c_int)
                    + CP_LEN_BASE[symbol as usize]
            } as c_int;
            let tree = s.dst.as_ptr();
            let distance_symbol = unsafe { cp_decode(s, tree, s.ndst as c_int) };
            let backwards_distance = unsafe {
                cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol as usize] as c_int)
                    + CP_DIST_BASE[distance_symbol as usize]
            } as c_int;
            if unsafe { s.out.offset(-(backwards_distance as isize)) } < s.begin {
                set_error(b"Attempted to write before out buffer (invalid backwards distance).\0");
                return 0;
            }
            if unsafe { s.out.add(length as usize) } > s.out_end {
                set_error(b"Attempted to overwrite out buffer while outputting a string.\0");
                return 0;
            }
            unsafe {
                let mut src = s.out.offset(-(backwards_distance as isize));
                let mut dst = s.out;
                s.out = s.out.add(length as usize);
                match backwards_distance {
                    1 => ptr::write_bytes(dst, *src, length as usize),
                    _ => {
                        let mut remaining = length;
                        while remaining != 0 {
                            *dst = *src;
                            dst = dst.add(1);
                            src = src.add(1);
                            remaining -= 1;
                        }
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
pub unsafe extern "C" fn cp_inflate(
    input: *mut c_void,
    in_bytes: c_int,
    out: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut s = Box::<CpState>::default();
    let in_ptr = input as *const u8;
    s.bits_left = in_bytes.wrapping_mul(8);
    let addr = in_ptr as usize;
    let first_bytes = ((((addr + 3) & !3) - addr) as c_int).wrapping_sub(0);
    unsafe {
        s.words = in_ptr.offset(first_bytes as isize) as *const u32;
    }
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        unsafe {
            s.bits |= (*in_ptr.offset(i as isize) as u64) << (i * 8);
        }
    }
    s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
    s.final_word = 0;
    for i in 0..last_bytes {
        unsafe {
            s.final_word |=
                (*in_ptr.offset((in_bytes - last_bytes + i) as isize) as u32) << (i * 8);
        }
    }
    s.count = first_bytes * 8;
    s.out = out as *mut u8;
    unsafe {
        s.out_end = (out as *mut u8).offset(out_bytes as isize);
    }
    s.begin = out as *mut u8;
    loop {
        let bfinal = unsafe { cp_read_bits(&mut s, 1) };
        let btype = unsafe { cp_read_bits(&mut s, 2) };
        match btype {
            0 => {
                if unsafe { cp_stored(&mut s) } == 0 {
                    return 0;
                }
            }
            1 => {
                cp_fixed(&mut s);
                if unsafe { cp_block(&mut s) } == 0 {
                    return 0;
                }
            }
            2 => {
                unsafe {
                    cp_dynamic(&mut s);
                }
                if unsafe { cp_block(&mut s) } == 0 {
                    return 0;
                }
            }
            3 => {
                set_error(b"Detected unknown block type within input stream.\0");
                return 0;
            }
            _ => {}
        }
        if bfinal != 0 {
            break;
        }
    }
    1
}

fn cp_paeth(a: u8, b: u8, c: u8) -> u8 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn unfilter(w: c_int, h: c_int, bpp: c_int, mut raw: *mut c_uchar) -> c_int {
    let len = w.wrapping_mul(bpp);
    let mut x;
    unsafe {
        if h > 0 {
            let filter = *raw;
            raw = raw.add(1);
            match filter {
                0 => {}
                1 => {
                    x = bpp;
                    while x < len {
                        let v =
                            (*raw.offset(x as isize)).wrapping_add(*raw.offset((x - bpp) as isize));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                2 => {}
                3 => {
                    x = bpp;
                    while x < len {
                        let v = (*raw.offset(x as isize))
                            .wrapping_add(*raw.offset((x - bpp) as isize) / 2);
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                4 => {
                    x = bpp;
                    while x < len {
                        let v = (*raw.offset(x as isize)).wrapping_add(cp_paeth(
                            *raw.offset((x - bpp) as isize),
                            0,
                            0,
                        ));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                _ => return 0,
            }
        }
        let mut prev = raw;
        raw = raw.offset(len as isize);
        let mut y = 1;
        while y < h {
            let filter = *raw;
            raw = raw.add(1);
            match filter {
                0 => {}
                1 => {
                    x = 0;
                    while x < bpp {
                        let v = (*raw.offset(x as isize)).wrapping_add(0);
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                    while x < len {
                        let v =
                            (*raw.offset(x as isize)).wrapping_add(*raw.offset((x - bpp) as isize));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                2 => {
                    x = 0;
                    while x < bpp {
                        let v = (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                    while x < len {
                        let v = (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                3 => {
                    x = 0;
                    while x < bpp {
                        let v =
                            (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize) / 2);
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                    while x < len {
                        let avg = ((*raw.offset((x - bpp) as isize) as c_int
                            + *prev.offset(x as isize) as c_int)
                            / 2) as u8;
                        let v = (*raw.offset(x as isize)).wrapping_add(avg);
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                4 => {
                    x = 0;
                    while x < bpp {
                        let v = (*raw.offset(x as isize)).wrapping_add(*prev.offset(x as isize));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                    while x < len {
                        let v = (*raw.offset(x as isize)).wrapping_add(cp_paeth(
                            *raw.offset((x - bpp) as isize),
                            *prev.offset(x as isize),
                            *prev.offset((x - bpp) as isize),
                        ));
                        *raw.offset(x as isize) = v;
                        x += 1;
                    }
                }
                _ => return 0,
            }
            y += 1;
            prev = raw;
            raw = raw.offset(len as isize);
        }
    }
    1
}
