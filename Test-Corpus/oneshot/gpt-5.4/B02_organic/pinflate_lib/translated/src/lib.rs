use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

#[repr(C)]
#[derive(Clone, Copy, Default)]
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

static CP_ERROR_REASON_STORAGE: &[u8] = b"\0";
pub static CP_ERROR_REASON: AtomicPtr<u8> = AtomicPtr::new(CP_ERROR_REASON_STORAGE.as_ptr() as *mut u8);

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
static CP_LEN_EXTRA_BITS: [u8; 31] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0];
static CP_LEN_BASE: [u32; 31] = [3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131, 163, 195, 227, 258, 0, 0];
static CP_DIST_EXTRA_BITS: [u8; 32] = [0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 0, 0];
static CP_DIST_BASE: [u32; 32] = [1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0];

struct CpState {
    bits: u64,
    count: i32,
    words: *const u32,
    word_count: i32,
    word_index: i32,
    bits_left: i32,
    final_word_available: bool,
    final_word: u32,
    out: *mut u8,
    out_end: *mut u8,
    begin: *mut u8,
    lookup: [u16; 1 << 9],
    lit: [u32; 288],
    dst: [u32; 32],
    len: [u32; 19],
    nlit: i32,
    ndst: i32,
    nlen: i32,
}

impl CpState {
    fn new() -> Self {
        Self {
            bits: 0,
            count: 0,
            words: ptr::null(),
            word_count: 0,
            word_index: 0,
            bits_left: 0,
            final_word_available: false,
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

fn set_error(msg: &'static [u8]) {
    CP_ERROR_REASON.store(msg.as_ptr() as *mut u8, Ordering::Relaxed);
}

fn cp_would_overflow(s: &CpState, num_bits: i32) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

fn cp_ptr(s: &CpState) -> *const u8 {
    assert!((s.bits_left & 7) == 0);
    unsafe { (s.words.add(s.word_index as usize) as *const u8).sub((s.count / 8) as usize) }
}

fn cp_peak_bits(s: &mut CpState, num_bits_to_read: i32) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = unsafe { *s.words.add(s.word_index as usize) };
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

fn cp_consume_bits(s: &mut CpState, num_bits_to_read: i32) -> u32 {
    assert!(s.count >= num_bits_to_read);
    let mask = if num_bits_to_read == 32 { u64::MAX } else { (1u64 << num_bits_to_read) - 1 };
    let bits = (s.bits & mask) as u32;
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

fn cp_rev16(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    a = ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8);
    a
}

fn cp_build(s: Option<&mut CpState>, tree: &mut [u32], lens: &[u8], sym_count: usize) -> i32 {
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
        let _ = state;
    }
    if let Some(state) = s {
        state.lookup.fill(0);
    }
    for (i, &len_u8) in lens.iter().take(sym_count).enumerate() {
        let len = len_u8 as usize;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            tree[slot] = (code << (32 - len)) | ((i as u32) << 4) | (len as u32);
            if let Some(state) = &mut s.as_ref().map(|st| st as *const _ as *mut CpState) {
                let state = unsafe { &mut **state };
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

fn cp_stored(s: &mut CpState) -> bool {
    cp_read_bits(s, s.count & 7);
    let len = cp_read_bits(s, 16) as u16;
    let nlen = cp_read_bits(s, 16) as u16;
    if len != !nlen {
        set_error(b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0");
        return false;
    }
    if s.bits_left / 8 > len as i32 {
        set_error(b"Stored block extends beyond end of input stream.\0");
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
    s.nlit = cp_build(Some(s), &mut s.lit, &CP_FIXED_TABLE, 288);
    s.ndst = cp_build(None, &mut s.dst, &CP_FIXED_TABLE[288..], 32);
    true
}

fn cp_decode(s: &mut CpState, tree: &[u32], mut hi: i32) -> i32 {
    let bits = cp_peak_bits(s, 16);
    let search = (cp_rev16(bits as u32) << 16) | 0xFFFF;
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

fn cp_dynamic(s: &mut CpState) -> bool {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + cp_read_bits(s, 5) as usize;
    let ndst = 1 + cp_read_bits(s, 5) as usize;
    let nlen = 4 + cp_read_bits(s, 4) as usize;
    for i in 0..nlen {
        lenlens[CP_PERMUTATION_ORDER[i] as usize] = cp_read_bits(s, 3) as u8;
    }
    s.nlen = cp_build(None, &mut s.len, &lenlens, 19);
    let mut lens = [0u8; 320];
    let mut n = 0usize;
    while n < nlit + ndst {
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
    s.nlit = cp_build(Some(s), &mut s.lit, &lens[..nlit], nlit);
    s.ndst = cp_build(None, &mut s.dst, &lens[nlit..nlit + ndst], ndst);
    true
}

fn cp_block(s: &mut CpState) -> bool {
    loop {
        let symbol = cp_decode(s, &s.lit, s.nlit);
        if symbol < 256 {
            if unsafe { s.out.add(1) } > s.out_end {
                set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return false;
            }
            unsafe {
                *s.out = symbol as u8;
                s.out = s.out.add(1);
            }
        } else if symbol > 256 {
            let symbol = (symbol - 257) as usize;
            let length = cp_read_bits(s, CP_LEN_EXTRA_BITS[symbol] as i32) as usize + CP_LEN_BASE[symbol] as usize;
            let distance_symbol = cp_decode(s, &s.dst, s.ndst) as usize;
            let backwards_distance = cp_read_bits(s, CP_DIST_EXTRA_BITS[distance_symbol] as i32) as usize + CP_DIST_BASE[distance_symbol] as usize;
            if (s.out as usize) < (s.begin as usize + backwards_distance) {
                set_error(b"Attempted to write before out buffer (invalid backwards distance).\0");
                return false;
            }
            if unsafe { s.out.add(length) } > s.out_end {
                set_error(b"Attempted to overwrite out buffer while outputting a string.\0");
                return false;
            }
            unsafe {
                let src = s.out.sub(backwards_distance);
                let dst = s.out;
                s.out = s.out.add(length);
                match backwards_distance {
                    1 => ptr::write_bytes(dst, *src, length),
                    _ => {
                        for i in 0..length {
                            *dst.add(i) = *src.add(i);
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
pub extern "C" fn pinflate(in_ptr: *mut c_void, in_bytes: i32, out_ptr: *mut c_void, out_bytes: i32) -> i32 {
    if in_ptr.is_null() || out_ptr.is_null() || in_bytes < 0 || out_bytes < 0 {
        return 0;
    }

    let _ = cp_make_pixel_a(0, 0, 0, 0);
    let _ = cp_make_pixel(0, 0, 0);

    let mut s = CpState::new();
    s.bits = 0;
    s.count = 0;
    s.word_index = 0;
    s.bits_left = in_bytes * 8;

    let in_addr = in_ptr as usize;
    let first_bytes = (((in_addr + 3) & !3usize) - in_addr) as i32;
    s.words = unsafe { (in_ptr as *const u8).add(first_bytes as usize) as *const u32 };
    s.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;

    unsafe {
        let in_bytes_ptr = in_ptr as *const u8;
        for i in 0..first_bytes as usize {
            s.bits |= (*in_bytes_ptr.add(i) as u64) << (i * 8);
        }
        s.final_word_available = last_bytes != 0;
        s.final_word = 0;
        for i in 0..last_bytes as usize {
            s.final_word |= (*in_bytes_ptr.add(in_bytes as usize - last_bytes as usize + i) as u32) << (i * 8);
        }
    }

    s.count = first_bytes * 8;
    s.out = out_ptr as *mut u8;
    s.out_end = unsafe { s.out.add(out_bytes as usize) };
    s.begin = out_ptr as *mut u8;

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
