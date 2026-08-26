use std::ffi::{c_char, c_int, c_void};
use std::os::raw::{c_uint, c_ushort};
use std::ptr;

static mut CP_ERROR_REASON: *const c_char = ptr::null();

static CP_FIXED_TABLE: [u8; 320] = [
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

static CP_LEN_EXTRA_BITS: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

static CP_LEN_BASE: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0,
];

static CP_DIST_EXTRA_BITS: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 0, 0,
];

static CP_DIST_BASE: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

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

fn cp_would_overflow(s: &CpState, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

fn cp_ptr(s: &CpState) -> *const u8 {
    assert!(s.bits_left & 7 == 0);
    unsafe { (s.words.add(s.word_index as usize) as *const u8).sub((s.count / 8) as usize) }
}

fn cp_peak_bits(s: &mut CpState, num_bits_to_read: i32) -> u64 {
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

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    assert!(s.count >= num_bits_to_read);
    let bits = (s.bits & ((1u64 << num_bits_to_read) - 1)) as u32;
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits
}

fn cp_read_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!(s.bits_left > 0);
    assert!(s.count <= 64);
    assert!(!cp_would_overflow(s, num_bits_to_read));
    cp_peak_bits(s, num_bits_to_read);
    cp_consume_bits(s, num_bits_to_read)
}

fn cp_rev16(a: u32) -> u32 {
    let mut a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

fn cp_build(s: Option<&mut CpState>, tree: &mut [u32], lens: &[u8], sym_count: usize) -> usize {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];
    
    for n in 0..sym_count {
        counts[lens[n] as usize] += 1;
    }
    
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    
    if let Some(state) = s {
        state.lookup.fill(0);
    }
    
    for i in 0..sym_count {
        let len = lens[i] as usize;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len] as u32;
            let slot = first[len] as usize;
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
            codes[len] += 1;
            first[len] += 1;
        }
    }
    
    first[15] as usize
}

fn cp_stored(s: &mut CpState) -> bool {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    
    if len != !nlen {
        unsafe {
            CP_ERROR_REASON = b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0".as_ptr() as *const c_char;
        }
        return false;
    }
    
    if s.bits_left / 8 > len as i32 {
        unsafe {
            CP_ERROR_REASON = b"Stored block extends beyond end of input stream.\0".as_ptr() as *const c_char;
        }
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
    s.nlit = cp_build(Some(s), &mut s.lit, &CP_FIXED_TABLE[..288], 288) as u32;
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32) as u32;
    true
}

fn cp_decode(s: &mut CpState, tree: &[u32], hi: usize) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = ((cp_rev16(bits as u32) as u64) << 16) | 0xFFFF;
    let mut lo = 0;
    let mut hi = hi;
    
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < tree[guess] as u64 {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    
    let key = tree[lo - 1];
    let len = 32 - (key & 0xF);
    assert!((search >> len) == ((key as u64) >> len));
    let _code = cp_consume_bits(s, (key & 0xF) as i32);
    ((key >> 4) & 0xFFF) as i32
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
    
    let mut lens = [0u8; 320];
    let mut n = 0;
    
    while n < nlit + ndst {
        let sym = cp_decode(s, &s.len, s.nlen as usize);
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
    
    s.nlit = cp_build(Some(s), &mut s.lit, &lens[..nlit], nlit) as u32;
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit..nlit + ndst], ndst) as u32;
    true
}

fn cp_block(s: &mut CpState) -> bool {
    loop {
        let symbol = cp_decode(s, &s.lit, s.nlit as usize);
        if symbol < 256 {
            unsafe {
                if s.out.add(1) > s.out_end {
                    CP_ERROR_REASON = b"Attempted to overwrite out buffer while outputting a symbol.\0".as_ptr() as *const c_char;
                    return false;
                }
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let sym = (symbol - 257) as usize;
            let length = cp_read_bits(s, CP_LEN_EXTRA_BITS[sym] as i32) + CP_LEN_BASE[sym];
            let distance_symbol = cp_decode(s, &s.dst, s.ndst as usize) as usize;
            let backwards_distance = cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol] as i32) + CP_DIST_BASE[distance_symbol];
            
            unsafe {
                if s.out.sub(backwards_distance as usize) < s.begin {
                    CP_ERROR_REASON = b"Attempted to write before out buffer (invalid backwards distance).\0".as_ptr() as *const c_char;
                    return false;
                }
                if s.out.add(length as usize) > s.out_end {
                    CP_ERROR_REASON = b"Attempted to overwrite out buffer while outputting a string.\0".as_ptr() as *const c_char;
                    return false;
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
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn pinflate(in_buf: *mut c_void, in_bytes: c_int, out_buf: *mut c_void, out_bytes: c_int) -> c_int {
    let mut s = CpState {
        bits: 0,
        count: 0,
        words: ptr::null(),
        word_count: 0,
        word_index: 0,
        bits_left: in_bytes * 8,
        final_word_available: 0,
        final_word: 0,
        out: out_buf as *mut u8,
        out_end: unsafe { (out_buf as *mut u8).add(out_bytes as usize) },
        begin: out_buf as *mut u8,
        lookup: [0; 512],
        lit: [0; 288],
        dst: [0; 32],
        len: [0; 19],
        nlit: 0,
        ndst: 0,
        nlen: 0,
    };
    
    let in_ptr = in_buf as *const u8;
    let first_bytes = (((in_ptr as usize) + 3) & !3) - (in_ptr as usize);
    s.words = unsafe { (in_ptr.add(first_bytes) as *const u32) };
    s.word_count = ((in_bytes as usize - first_bytes) / 4) as i32;
    let last_bytes = (in_bytes as usize - first_bytes) & 3;
    
    for i in 0..first_bytes {
        s.bits |= (unsafe { *in_ptr.add(i) } as u64) << (i * 8);
    }
    
    s.final_word_available = if last_bytes > 0 { 1 } else { 0 };
    for i in 0..last_bytes {
        s.final_word |= (unsafe { *in_ptr.add(in_bytes as usize - last_bytes + i) } as u32) << (i * 8);
    }
    
    s.count = (first_bytes * 8) as i32;
    
    let mut bfinal;
    loop {
        bfinal = cp_read_bits(&mut s, 1);
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
            _ => {
                unsafe {
                    CP_ERROR_REASON = b"Detected unknown block type within input stream.\0".as_ptr() as *const c_char;
                }
                return 0;
            }
        }
        
        if bfinal != 0 {
            break;
        }
    }
    
    1
}
