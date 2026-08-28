#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_char, c_int, c_void};
use std::ptr;

const fn make_fixed_table() -> [u8; 320] {
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
pub static mut cp_fixed_table: [u8; 320] = make_fixed_table();

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

#[repr(C)]
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

unsafe fn table_u8<const N: usize>(table: *const [u8; N], index: usize) -> u8 {
    table.cast::<u8>().add(index).read()
}

unsafe fn table_u32<const N: usize>(table: *const [u32; N], index: usize) -> u32 {
    table.cast::<u32>().add(index).read()
}

unsafe fn set_error(reason: &'static [u8]) {
    cp_error_reason = reason.as_ptr().cast();
}

fn would_overflow(s: &State, num_bits: c_int) -> bool {
    (s.bits_left + s.count) - num_bits < 0
}

unsafe fn input_ptr(s: &State) -> *const u8 {
    assert_eq!(s.bits_left & 7, 0);
    s.words
        .add(s.word_index as usize)
        .cast::<u8>()
        .sub((s.count / 8) as usize)
}

unsafe fn peek_bits(s: &mut State, num_bits_to_read: c_int) -> u64 {
    if s.count < num_bits_to_read {
        if s.word_index < s.word_count {
            let word = s.words.add(s.word_index as usize).read();
            s.word_index += 1;
            s.bits |= (word as u64) << s.count;
            s.count += 32;
            assert!(s.word_index <= s.word_count);
        } else if s.final_word_available != 0 {
            s.bits |= (s.final_word as u64) << s.count;
            s.count += s.bits_left;
            s.final_word_available = 0;
        }
    }
    s.bits
}

fn consume_bits(s: &mut State, num_bits_to_read: c_int) -> u32 {
    assert!(s.count >= num_bits_to_read);
    let bits = s.bits & ((1_u64 << num_bits_to_read) - 1);
    s.bits >>= num_bits_to_read;
    s.count -= num_bits_to_read;
    s.bits_left -= num_bits_to_read;
    bits as u32
}

unsafe fn read_bits(s: &mut State, num_bits_to_read: c_int) -> u32 {
    assert!(num_bits_to_read <= 32);
    assert!(num_bits_to_read >= 0);
    assert!(s.bits_left > 0);
    assert!(s.count <= 64);
    assert!(!would_overflow(s, num_bits_to_read));
    peek_bits(s, num_bits_to_read);
    consume_bits(s, num_bits_to_read)
}

fn rev16(mut a: u32) -> u32 {
    a = ((a & 0xaaaa) >> 1) | ((a & 0x5555) << 1);
    a = ((a & 0xcccc) >> 2) | ((a & 0x3333) << 2);
    a = ((a & 0xf0f0) >> 4) | ((a & 0x0f0f) << 4);
    ((a & 0xff00) >> 8) | ((a & 0x00ff) << 8)
}

unsafe fn build(state: *mut State, tree: *mut u32, lens: *const u8, sym_count: c_int) -> c_int {
    let mut codes = [0_i32; 16];
    let mut first = [0_i32; 16];
    let mut counts = [0_i32; 16];

    for n in 0..sym_count {
        counts[lens.add(n as usize).read() as usize] += 1;
    }
    counts[0] = 0;
    codes[0] = 0;
    first[0] = 0;
    for n in 1..=15 {
        codes[n] = (codes[n - 1] + counts[n - 1]) << 1;
        first[n] = first[n - 1] + counts[n - 1];
    }

    if !state.is_null() {
        ptr::write_bytes((*state).lookup.as_mut_ptr(), 0, 1 << 9);
    }
    for i in 0..sym_count {
        let code_len = lens.add(i as usize).read() as usize;
        if code_len != 0 {
            assert!(code_len < 16);
            let code = codes[code_len] as u32;
            codes[code_len] += 1;
            let slot = first[code_len] as usize;
            first[code_len] += 1;
            tree.add(slot)
                .write((code << (32 - code_len)) | ((i as u32) << 4) | code_len as u32);
            if !state.is_null() && code_len <= 9 {
                let mut j = (rev16(code) >> (16 - code_len)) as usize;
                while j < (1 << 9) {
                    (*state).lookup[j] = ((code_len << 9) | i as usize) as u16;
                    j += 1 << code_len;
                }
            }
        }
    }
    first[15]
}

unsafe fn stored(s: &mut State) -> bool {
    read_bits(s, s.count & 7);
    let len = read_bits(s, 16) as u16;
    let nlen = read_bits(s, 16) as u16;
    if len != !nlen {
        set_error(
            b"Failed to find LEN and NLEN as complements within stored (uncompressed) stream.\0",
        );
        return false;
    }
    if s.bits_left / 8 > len as c_int {
        set_error(b"Stored block extends beyond end of input stream.\0");
        return false;
    }
    let source = input_ptr(s);
    ptr::copy_nonoverlapping(source, s.out, len as usize);
    s.out = s.out.add(len as usize);
    true
}

unsafe fn fixed(s: &mut State) -> bool {
    s.nlit = build(
        s,
        s.lit.as_mut_ptr(),
        ptr::addr_of!(cp_fixed_table).cast::<u8>(),
        288,
    ) as u32;
    s.ndst = build(
        ptr::null_mut(),
        s.dst.as_mut_ptr(),
        ptr::addr_of!(cp_fixed_table).cast::<u8>().add(288),
        32,
    ) as u32;
    true
}

unsafe fn decode(s: &mut State, tree: *const u32, mut hi: c_int) -> c_int {
    let bits = peek_bits(s, 16);
    let search = (rev16(bits as u32) << 16) | 0xffff;
    let mut lo = 0;
    while lo < hi {
        let guess = (lo + hi) >> 1;
        if search < tree.add(guess as usize).read() {
            hi = guess;
        } else {
            lo = guess + 1;
        }
    }
    let key = tree.add((lo - 1) as usize).read();
    let shift = 32 - (key & 0xf);
    assert_eq!(search >> shift, key >> shift);
    let _code = consume_bits(s, (key & 0xf) as c_int);
    ((key >> 4) & 0xfff) as c_int
}

unsafe fn dynamic(s: &mut State) -> bool {
    let mut lenlens = [0_u8; 19];
    let nlit = 257 + read_bits(s, 5) as c_int;
    let ndst = 1 + read_bits(s, 5) as c_int;
    let nlen = 4 + read_bits(s, 4) as c_int;
    for i in 0..nlen {
        let order = table_u8(ptr::addr_of!(cp_permutation_order), i as usize);
        lenlens[order as usize] = read_bits(s, 3) as u8;
    }
    s.nlen = build(ptr::null_mut(), s.len.as_mut_ptr(), lenlens.as_ptr(), 19) as u32;

    let mut lens = [0_u8; 288 + 32];
    let mut n = 0;
    while n < nlit + ndst {
        let symbol = decode(s, s.len.as_ptr(), s.nlen as c_int);
        match symbol {
            16 => {
                let mut repeats = 3 + read_bits(s, 2) as c_int;
                while repeats != 0 {
                    lens[n as usize] = lens[(n - 1) as usize];
                    repeats -= 1;
                    n += 1;
                }
            }
            17 => {
                let mut repeats = 3 + read_bits(s, 3) as c_int;
                while repeats != 0 {
                    lens[n as usize] = 0;
                    repeats -= 1;
                    n += 1;
                }
            }
            18 => {
                let mut repeats = 11 + read_bits(s, 7) as c_int;
                while repeats != 0 {
                    lens[n as usize] = 0;
                    repeats -= 1;
                    n += 1;
                }
            }
            _ => {
                lens[n as usize] = symbol as u8;
                n += 1;
            }
        }
    }
    s.nlit = build(s, s.lit.as_mut_ptr(), lens.as_ptr(), nlit) as u32;
    s.ndst = build(
        ptr::null_mut(),
        s.dst.as_mut_ptr(),
        lens.as_ptr().add(nlit as usize),
        ndst,
    ) as u32;
    true
}

unsafe fn block(s: &mut State) -> bool {
    loop {
        let mut symbol = decode(s, s.lit.as_ptr(), s.nlit as c_int);
        if symbol < 256 {
            if s.out.add(1) > s.out_end {
                set_error(b"Attempted to overwrite out buffer while outputting a symbol.\0");
                return false;
            }
            s.out.write(symbol as u8);
            s.out = s.out.add(1);
        } else if symbol > 256 {
            symbol -= 257;
            let length = read_bits(
                s,
                table_u8(ptr::addr_of!(cp_len_extra_bits), symbol as usize) as c_int,
            ) + table_u32(ptr::addr_of!(cp_len_base), symbol as usize);
            let distance_symbol = decode(s, s.dst.as_ptr(), s.ndst as c_int);
            let backwards_distance =
                read_bits(
                    s,
                    table_u8(ptr::addr_of!(cp_dist_extra_bits), distance_symbol as usize) as c_int,
                ) + table_u32(ptr::addr_of!(cp_dist_base), distance_symbol as usize);
            if (s.out as usize).wrapping_sub(backwards_distance as usize) < s.begin as usize {
                set_error(b"Attempted to write before out buffer (invalid backwards distance).\0");
                return false;
            }
            if s.out.add(length as usize) > s.out_end {
                set_error(b"Attempted to overwrite out buffer while outputting a string.\0");
                return false;
            }
            let mut source = s.out.sub(backwards_distance as usize);
            let mut destination = s.out;
            s.out = s.out.add(length as usize);
            if backwards_distance == 1 {
                ptr::write_bytes(destination, source.read(), length as usize);
            } else {
                let mut remaining = length;
                while remaining != 0 {
                    destination.write(source.read());
                    destination = destination.add(1);
                    source = source.add(1);
                    remaining -= 1;
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
    let mut state: Box<State> = Box::new(std::mem::zeroed());
    state.bits_left = in_bytes * 8;

    let input_address = input as usize;
    let first_bytes = (((input_address + 3) & !3) - input_address) as c_int;
    state.words = input.cast::<u8>().add(first_bytes as usize).cast::<u32>();
    state.word_count = (in_bytes - first_bytes) / 4;
    let last_bytes = (in_bytes - first_bytes) & 3;

    for i in 0..first_bytes {
        state.bits |= (*input.cast::<u8>().add(i as usize) as u64) << ((i * 8) as u32);
    }
    state.final_word_available = c_int::from(last_bytes != 0);
    for i in 0..last_bytes {
        state.final_word |= (*input.cast::<u8>().add((in_bytes - last_bytes + i) as usize) as u32)
            << ((i * 8) as u32);
    }
    state.count = first_bytes * 8;
    state.out = output.cast::<u8>();
    state.out_end = state.out.add(out_bytes as usize);
    state.begin = state.out;

    loop {
        let final_block = read_bits(&mut state, 1);
        let block_type = read_bits(&mut state, 2);
        match block_type {
            0 => {
                if !stored(&mut state) {
                    return 0;
                }
            }
            1 => {
                fixed(&mut state);
                if !block(&mut state) {
                    return 0;
                }
            }
            2 => {
                dynamic(&mut state);
                if !block(&mut state) {
                    return 0;
                }
            }
            3 => {
                set_error(b"Detected unknown block type within input stream.\0");
                return 0;
            }
            _ => unreachable!(),
        }
        if final_block != 0 {
            break;
        }
    }
    1
}
