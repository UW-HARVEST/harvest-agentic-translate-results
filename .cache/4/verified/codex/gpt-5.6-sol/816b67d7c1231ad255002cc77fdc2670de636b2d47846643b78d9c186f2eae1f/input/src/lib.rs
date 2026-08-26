use std::ffi::{c_char, c_int, c_void};
use std::ptr;

const fn fixed_table() -> [u8; 320] {
    let mut table = [0; 320];
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

#[unsafe(no_mangle)]
pub static mut cp_error_reason: *const c_char = ptr::null();

#[unsafe(no_mangle)]
pub static mut cp_fixed_table: [u8; 320] = fixed_table();

#[unsafe(no_mangle)]
pub static mut cp_permutation_order: [u8; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[unsafe(no_mangle)]
pub static mut cp_len_extra_bits: [u8; 31] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_len_base: [u32; 31] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_extra_bits: [u8; 32] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13, 0, 0,
];

#[unsafe(no_mangle)]
pub static mut cp_dist_base: [u32; 32] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577, 0, 0,
];

static ERR_STORED_COMPLEMENT: &[u8] =
    b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0";
static ERR_STORED_END: &[u8] = b"Stored block extends beyond end of input stream.\0";
static ERR_SYMBOL_OUTPUT: &[u8] = b"Attempted to overwrite out buffer while outputting a symbol.\0";
static ERR_DISTANCE: &[u8] =
    b"Attempted to write before out buffer (invalid backwards distance).\0";
static ERR_STRING_OUTPUT: &[u8] = b"Attempted to overwrite out buffer while outputting a string.\0";
static ERR_BLOCK_TYPE: &[u8] = b"Detected unknown block type within input stream.\0";

struct State {
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

impl State {
    fn zeroed() -> Self {
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

unsafe fn would_overflow(s: *const State, num_bits: c_int) -> bool {
    ((*s).bits_left + (*s).count) - num_bits < 0
}

unsafe fn input_ptr(s: *const State) -> *const u8 {
    assert_eq!((*s).bits_left & 7, 0);
    (*s).words
        .add((*s).word_index as usize)
        .cast::<u8>()
        .offset(-((*s).count / 8) as isize)
}

unsafe fn peek_bits(s: *mut State, num_bits_to_read: c_int) -> u64 {
    if (*s).count < num_bits_to_read {
        if (*s).word_index < (*s).word_count {
            let word = ptr::read((*s).words.add((*s).word_index as usize));
            (*s).word_index += 1;
            (*s).bits |= (word as u64) << (*s).count;
            (*s).count += 32;
            assert!((*s).word_index <= (*s).word_count);
        } else if (*s).final_word_available != 0 {
            (*s).bits |= ((*s).final_word as u64) << (*s).count;
            (*s).count += (*s).bits_left;
            (*s).final_word_available = 0;
        }
    }
    (*s).bits
}

unsafe fn consume_bits(s: *mut State, num_bits_to_read: c_int) -> u32 {
    assert!((*s).count >= num_bits_to_read);
    let bits = (*s).bits & ((1u64 << num_bits_to_read) - 1);
    (*s).bits >>= num_bits_to_read;
    (*s).count -= num_bits_to_read;
    (*s).bits_left -= num_bits_to_read;
    bits as u32
}

unsafe fn read_bits(s: *mut State, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!((*s).bits_left > 0);
    assert!((*s).count <= 64);
    assert!(!would_overflow(s, num_bits_to_read));
    peek_bits(s, num_bits_to_read);
    consume_bits(s, num_bits_to_read)
}

fn rev16(mut a: u32) -> u32 {
    a = ((a & 0xAAAA) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xCCCC) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xF0F0) >> 4) | ((a & 0x0F0F) << 4);
    ((a & 0xFF00) >> 8) | ((a & 0x00FF) << 8)
}

unsafe fn build_tree(s: *mut State, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0i32; 16];
    let mut first = [0i32; 16];
    let mut counts = [0i32; 16];

    for n in 0..sym_count {
        let len = *lens.add(n as usize) as usize;
        counts[len] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }
    if !s.is_null() {
        (*s).lookup.fill(0);
    }
    for i in 0..sym_count {
        let len = *lens.add(i as usize) as usize;
        if len != 0 {
            assert!(len < 16);
            let code = codes[len] as u32;
            codes[len] += 1;
            let slot = first[len] as usize;
            first[len] += 1;
            *tree.add(slot) = (code << (32 - len)) | ((i as u32) << 4) | len as u32;
            if !s.is_null() && len <= 9 {
                let mut j = (rev16(code) >> (16 - len)) as usize;
                while j < (1 << 9) {
                    (*s).lookup[j] = ((len << 9) | i as usize) as u16;
                    j += 1 << len;
                }
            }
        }
    }
    first[15]
}

unsafe fn stored_block(s: *mut State) -> bool {
    read_bits(s, (*s).count & 7);
    let len = read_bits(s, 16) as u16;
    let nlen = read_bits(s, 16) as u16;
    if len != !nlen {
        cp_error_reason = ERR_STORED_COMPLEMENT.as_ptr().cast();
        return false;
    }
    if (*s).bits_left / 8 > len as c_int {
        cp_error_reason = ERR_STORED_END.as_ptr().cast();
        return false;
    }
    let source = input_ptr(s);
    ptr::copy_nonoverlapping(source, (*s).out, len as usize);
    (*s).out = (*s).out.add(len as usize);
    true
}

unsafe fn fixed_block(s: *mut State) {
    (*s).nlit = build_tree(
        s,
        ptr::addr_of_mut!((*s).lit).cast::<u32>(),
        ptr::addr_of!(cp_fixed_table).cast::<u8>(),
        288,
    ) as u32;
    (*s).ndst = build_tree(
        ptr::null_mut(),
        ptr::addr_of_mut!((*s).dst).cast::<u32>(),
        ptr::addr_of!(cp_fixed_table).cast::<u8>().add(288),
        32,
    ) as u32;
}

unsafe fn decode(s: *mut State, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = peek_bits(s, 16);
    let search = (rev16(bits as u32) << 16) | 0xFFFF;
    let mut lo = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < *tree.add(guess as usize) {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = *tree.offset((lo - 1) as isize);
    let len = 32 - (key & 0xF);
    assert_eq!(search >> len, key >> len);
    let _code = consume_bits(s, (key & 0xF) as c_int);
    ((key >> 4) & 0xFFF) as c_int
}

unsafe fn dynamic_block(s: *mut State) {
    let mut lenlens = [0u8; 19];
    let nlit = 257 + read_bits(s, 5) as c_int;
    let ndst = 1 + read_bits(s, 5) as c_int;
    let nlen = 4 + read_bits(s, 4) as c_int;
    let permutation = ptr::addr_of!(cp_permutation_order).cast::<u8>();
    for i in 0..nlen {
        lenlens[*permutation.add(i as usize) as usize] = read_bits(s, 3) as u8;
    }
    (*s).nlen = build_tree(
        ptr::null_mut(),
        ptr::addr_of_mut!((*s).len).cast::<u32>(),
        lenlens.as_ptr(),
        19,
    ) as u32;

    let mut lens = std::mem::MaybeUninit::<[u8; 320]>::uninit();
    let lens_ptr = lens.as_mut_ptr().cast::<u8>();
    let mut n = 0;
    while n < nlit + ndst {
        let sym = decode(s, ptr::addr_of!((*s).len).cast::<u32>(), (*s).nlen as c_int);
        match sym {
            16 => {
                let mut i = 3 + read_bits(s, 2) as c_int;
                while i != 0 {
                    *lens_ptr.add(n as usize) = *lens_ptr.offset((n - 1) as isize);
                    i -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut i = 3 + read_bits(s, 3) as c_int;
                while i != 0 {
                    *lens_ptr.add(n as usize) = 0;
                    i -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut i = 11 + read_bits(s, 7) as c_int;
                while i != 0 {
                    *lens_ptr.add(n as usize) = 0;
                    i -= 1;
                    n += 1;
                }
            }
            _ => {
                *lens_ptr.add(n as usize) = sym as u8;
                n += 1;
            }
        }
    }
    (*s).nlit = build_tree(s, ptr::addr_of_mut!((*s).lit).cast::<u32>(), lens_ptr, nlit) as u32;
    (*s).ndst = build_tree(
        ptr::null_mut(),
        ptr::addr_of_mut!((*s).dst).cast::<u32>(),
        lens_ptr.add(nlit as usize),
        ndst,
    ) as u32;
}

unsafe fn compressed_block(s: *mut State) -> bool {
    loop {
        let mut symbol = decode(s, ptr::addr_of!((*s).lit).cast::<u32>(), (*s).nlit as c_int);
        if symbol < 256 {
            if ((*s).out as usize).wrapping_add(1) > (*s).out_end as usize {
                cp_error_reason = ERR_SYMBOL_OUTPUT.as_ptr().cast();
                return false;
            }
            *(*s).out = symbol as u8;
            (*s).out = (*s).out.add(1);
        } else if symbol > 256 {
            symbol -= 257;
            let len_extra = *ptr::addr_of!(cp_len_extra_bits)
                .cast::<u8>()
                .add(symbol as usize);
            let len_base = *ptr::addr_of!(cp_len_base)
                .cast::<u32>()
                .add(symbol as usize);
            let mut length = read_bits(s, len_extra as c_int).wrapping_add(len_base) as c_int;
            let distance_symbol =
                decode(s, ptr::addr_of!((*s).dst).cast::<u32>(), (*s).ndst as c_int);
            let dist_extra = *ptr::addr_of!(cp_dist_extra_bits)
                .cast::<u8>()
                .add(distance_symbol as usize);
            let dist_base = *ptr::addr_of!(cp_dist_base)
                .cast::<u32>()
                .add(distance_symbol as usize);
            let backwards_distance =
                read_bits(s, dist_extra as c_int).wrapping_add(dist_base) as c_int;

            if ((*s).out as usize).wrapping_sub(backwards_distance as usize) < (*s).begin as usize {
                cp_error_reason = ERR_DISTANCE.as_ptr().cast();
                return false;
            }
            if ((*s).out as usize).wrapping_add(length as usize) > (*s).out_end as usize {
                cp_error_reason = ERR_STRING_OUTPUT.as_ptr().cast();
                return false;
            }
            let mut source = (*s).out.offset(-(backwards_distance as isize));
            let mut destination = (*s).out;
            (*s).out = (*s).out.add(length as usize);
            if backwards_distance == 1 {
                ptr::write_bytes(destination, *source, length as usize);
            } else {
                while length != 0 {
                    *destination = *source;
                    destination = destination.add(1);
                    source = source.add(1);
                    length -= 1;
                }
            }
        } else {
            break;
        }
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn pinflate(
    input: *mut c_void,
    in_bytes: c_int,
    output: *mut c_void,
    out_bytes: c_int,
) -> c_int {
    let mut state = Box::new(State::zeroed());
    let s = state.as_mut() as *mut State;
    (*s).bits_left = in_bytes * 8;

    let input_address = input as usize;
    let first_bytes = ((input_address.wrapping_add(3) & !3).wrapping_sub(input_address)) as c_int;
    (*s).words = input.cast::<u8>().add(first_bytes as usize).cast::<u32>();
    (*s).word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;
    for i in 0..first_bytes {
        (*s).bits |= (*input.cast::<u8>().add(i as usize) as u64) << (i * 8);
    }
    (*s).final_word_available = if last_bytes != 0 { 1 } else { 0 };
    for i in 0..last_bytes {
        let byte = *input
            .cast::<u8>()
            .offset((in_bytes - last_bytes + i) as isize);
        (*s).final_word |= (byte as u32) << (i * 8);
    }
    (*s).count = first_bytes * 8;
    (*s).out = output.cast();
    (*s).out_end = (*s).out.offset(out_bytes as isize);
    (*s).begin = output.cast();

    loop {
        let bfinal = read_bits(s, 1);
        let btype = read_bits(s, 2);
        match btype {
            0 => {
                if !stored_block(s) {
                    return 0;
                }
            }
            1 => {
                fixed_block(s);
                if !compressed_block(s) {
                    return 0;
                }
            }
            2 => {
                dynamic_block(s);
                if !compressed_block(s) {
                    return 0;
                }
            }
            _ => {
                cp_error_reason = ERR_BLOCK_TYPE.as_ptr().cast();
                return 0;
            }
        }
        if bfinal != 0 {
            break;
        }
    }
    1
}
