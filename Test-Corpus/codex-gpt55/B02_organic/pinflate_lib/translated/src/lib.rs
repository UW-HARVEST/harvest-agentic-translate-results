use std::ffi::{c_char, c_int, c_void};
use std::ptr;

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

const CP_PERMUTATION_ORDER: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];
const CP_LEN_EXTRA_BITS: [u8; 29 + 2] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5,
    5, 5, 5, 0, 0, 0,
];
const CP_LEN_BASE: [u32; 29 + 2] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59,
    67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];
const CP_DIST_EXTRA_BITS: [u8; 30 + 2] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10,
    11, 11, 12, 12, 13, 13, 0, 0,
];
const CP_DIST_BASE: [u32; 30 + 2] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513,
    769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

static ERR_STORED_COMPLEMENT: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
static ERR_STORED_EXTENDS: &[u8] = b"Stored block extends beyond end of input stream.\0";
static ERR_SYMBOL_OVERWRITE: &[u8] =
    b"Attempted to overwrite out buffer while outputting a symbol.\0";
static ERR_BAD_DISTANCE: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
static ERR_STRING_OVERWRITE: &[u8] =
    b"Attempted to overwrite out buffer while outputting a string.\0";
static ERR_UNKNOWN_BLOCK: &[u8] = b"Detected unknown block type within input stream.\0";

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[derive(Clone)]
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
        cp_error_reason = reason.as_ptr() as *const c_char;
    }
}

fn cp_would_overflow(s: &CpState, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

fn cp_ptr(s: &CpState) -> *const u8 {
    debug_assert_eq!(s.bits_left & 7, 0);
    (s.words.wrapping_add(s.word_index as usize) as *const u8)
        .wrapping_sub((s.count / 8) as usize)
}

fn cp_peak_bits(s: &mut CpState, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { ptr::read_unaligned(s.words.add(s.word_index as usize)) };
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            debug_assert!(s.word_index <= s.word_count);
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
    debug_assert!(s.count >= num_bits_to_read);
    let bits = if num_bits_to_read == 64 {
        s.bits
    } else {
        s.bits & (((1_u64) << num_bits_to_read) - 1)
    };
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits as u32
}

fn cp_read_bits(s: &mut CpState, num_bits_to_read: c_int) -> u32 {
    debug_assert!(num_bits_to_read <= 32);
    debug_assert!(num_bits_to_read >= 0);
    debug_assert!(s.bits_left > 0);
    debug_assert!(s.count <= 64);
    debug_assert!(!cp_would_overflow(s, num_bits_to_read));
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    a = ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8);
    a
}

fn cp_build(s: Option<&mut CpState>, tree: &mut [u32], lens: &[u8], sym_count: c_int) -> c_int {
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
    let mut s = s;
    if let Some(st) = s.as_deref_mut() {
        st.lookup.fill(0);
    }
    for i in 0..sym_count as usize {
        let len = lens[i] as usize;
        if len != 0 {
            debug_assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(st) = s.as_deref_mut() {
                if len <= 9 {
                    let mut j = (cp_rev16(code) >> (16 - len)) as usize;
                    while j < (1 << 9) {
                        st.lookup[j] = (((len as u16) << 9) | (i as u16)) as u16;
                        j += 1 << len;
                    }
                }
            }
        }
    }
    first[15] as c_int
}

fn cp_stored(s: &mut CpState) -> c_int {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        set_error(ERR_STORED_COMPLEMENT);
        return 0;
    }
    if !(s.bits_left / 8 <= len as c_int) {
        set_error(ERR_STORED_EXTENDS);
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
    let mut lit = std::mem::replace(&mut s.lit, [0; 288]);
    s.nlit = cp_build(Some(s), &mut lit, &CP_FIXED_TABLE[..288], 288) as u32;
    s.lit = lit;
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32) as u32;
    1
}

fn cp_decode(s: &mut CpState, tree: &[u32], mut hi: c_int) -> c_int {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xffff;
    let mut lo = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < tree[guess as usize] {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = tree[(lo - 1) as usize];
    let len = 32 - (key & 0xf);
    debug_assert_eq!(search >> len, key >> len);
    let _code = cp_consume_bits(s, (key & 0xf) as c_int);
    ((key >> 4) & 0xfff) as c_int
}

fn cp_dynamic(s: &mut CpState) -> c_int {
    let mut lenlens = [0_u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as c_int;
    let ndst = 1 + cp_read_bits(s, 5) as c_int;
    let nlen = 4 + cp_read_bits(s, 4) as c_int;
    for i in 0..nlen as usize {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    s.nlen = cp_build(None, &mut s.len, &lenlens, 19) as u32;
    let mut lens = [0_u8; 288 + 32];
    let mut n = 0_i32;
    while n < nlit + ndst {
        let sym = {
            let len = s.len;
            cp_decode(s, &len, s.nlen as c_int)
        };
        match sym {
            16 => {
                let mut i = 3 + cp_read_bits(s, 2) as c_int;
                while i != 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + cp_read_bits(s, 3) as c_int;
                while i != 0 {
                    lens[n as usize] = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + cp_read_bits(s, 7) as c_int;
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
    let mut lit = std::mem::replace(&mut s.lit, [0; 288]);
    s.nlit = cp_build(Some(s), &mut lit, &lens[..nlit as usize], nlit) as u32;
    s.lit = lit;
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit as usize..], ndst) as u32;
    1
}

fn cp_block(s: &mut CpState) -> c_int {
    loop {
        let symbol = {
            let lit = s.lit;
            cp_decode(s, &lit, s.nlit as c_int)
        };
        if symbol < 256 {
            unsafe {
                if s.out.wrapping_add(1) > s.out_end {
                    set_error(ERR_SYMBOL_OVERWRITE);
                    return 0;
                }
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let symbol = symbol - 257;
            let length = cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol as usize] as c_int)
                + CP_LEN_BASE[symbol as usize];
            let distance_symbol = {
                let dst = s.dst;
                cp_decode(s, &dst, s.ndst as c_int)
            };
            let backwards_distance =
                cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol as usize] as c_int)
                    + CP_DIST_BASE[distance_symbol as usize];
            unsafe {
                if s.out.wrapping_sub(backwards_distance as usize) < s.begin {
                    set_error(ERR_BAD_DISTANCE);
                    return 0;
                }
                if s.out.wrapping_add(length as usize) > s.out_end {
                    set_error(ERR_STRING_OVERWRITE);
                    return 0;
                }
                let mut src = s.out.sub(backwards_distance as usize);
                let mut dst = s.out;
                s.out = s.out.add(length as usize);
                match backwards_distance {
                    1 => {
                        ptr::write_bytes(dst, *src, length as usize);
                    }
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
pub extern "C" fn pinflate(
    input: *mut c_void,
    in_bytes: c_int,
    output: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    unsafe {
        let mut s = Box::<CpState>::default();
        s.bits = 0;
        s.count = 0;
        s.word_index = 0;
        s.bits_left = in_bytes.wrapping_mul(8);
        let in_addr = input as usize;
        let first_bytes = (((in_addr + 3) & !3) - in_addr) as c_int;
        s.words = (input as *const u8).add(first_bytes as usize) as *const u32;
        s.word_count = (in_bytes - first_bytes) / 4;
        let last_bytes = (in_bytes - first_bytes) & 3;
        for i in 0..first_bytes {
            s.bits |= (*((input as *const u8).add(i as usize)) as u64) << (i * 8);
        }
        s.final_word_available = if last_bytes != 0 { 1 } else { 0 };
        s.final_word = 0;
        for i in 0..last_bytes {
            s.final_word |= (*((input as *const u8).add((in_bytes - last_bytes + i) as usize))
                as u32)
                << (i * 8);
        }
        s.count = first_bytes * 8;
        s.out = output as *mut u8;
        s.out_end = s.out.wrapping_add(out_bytes as usize);
        s.begin = output as *mut u8;
        let mut bfinal;
        loop {
            bfinal = cp_read_bits(&mut s, 1) as c_int;
            let btype = cp_read_bits(&mut s, 2) as c_int;
            match btype {
                0 => {
                    if cp_stored(&mut s) == 0 {
                        return 0;
                    }
                }
                1 => {
                    cp_fixed(&mut s);
                    if cp_block(&mut s) == 0 {
                        return 0;
                    }
                }
                2 => {
                    cp_dynamic(&mut s);
                    if cp_block(&mut s) == 0 {
                        return 0;
                    }
                }
                3 => {
                    set_error(ERR_UNKNOWN_BLOCK);
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
}
