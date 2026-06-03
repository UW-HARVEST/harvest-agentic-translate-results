use libfor::for_gen::*;
use libfor::forLib;

fn generate_input(base: u32, length: u32, bits: u32) -> Vec<u32> {
    let mut inbuf = vec![0u32; length as usize];
    for i in 0..length {
        inbuf[i as usize] = if bits == 0 {
            base
        } else if bits == 32 {
            base + i
        } else {
            let max = (1u32 << bits).wrapping_sub(1);
            base + (i % max)
        };
    }
    inbuf
}

fn lowlevel_block_func(
    bits: u32,
    pack: fn(u32, &[u32], &mut [u8]) -> u32,
    unpack: fn(u32, &[u8], &mut [u32]) -> u32,
    in_data: &[u32],
    base: u32,
    length: u32,
) {
    let mut out = vec![0u8; 1024];
    let mut tmp = vec![0u32; 1024];
    let s1 = pack(base, in_data, &mut out);
    let s2 = unpack(base, &out, &mut tmp);
    assert_eq!(s1, s2, "size mismatch for bits={} length={}", bits, length);
    for i in 0..length as usize {
        assert_eq!(in_data[i], tmp[i], "data mismatch at {} for bits={}", i, bits);
    }
    for i in 0..length {
        assert_eq!(
            in_data[i as usize],
            forLib::for_select_bits(&out, base, bits, i),
            "select mismatch at {} for bits={}",
            i,
            bits
        );
    }
    for i in 0..length {
        let index =
            forLib::for_linear_search_bits(&out, length, base, bits, in_data[i as usize]);
        assert_eq!(
            in_data[i as usize], in_data[index as usize],
            "linsearch mismatch for bits={} value={}",
            bits, in_data[i as usize]
        );
    }
}

fn lowlevel_blockx_func(
    bits: u32,
    pack: fn(u32, &[u32], &mut [u8], u32) -> u32,
    unpack: fn(u32, &[u8], &mut [u32], u32) -> u32,
    in_data: &[u32],
    base: u32,
    length: u32,
) {
    let mut out = vec![0u8; 1024];
    let mut tmp = vec![0u32; 1024];
    let s1 = pack(base, in_data, &mut out, length);
    let s2 = unpack(base, &out, &mut tmp, length);
    assert_eq!(s1, s2);
    for i in 0..length as usize {
        assert_eq!(in_data[i], tmp[i], "data mismatch at {} for bits={}", i, bits);
    }
    for i in 0..length {
        assert_eq!(
            in_data[i as usize],
            forLib::for_select_bits(&out, base, bits, i)
        );
    }
    for i in 0..length {
        let index =
            forLib::for_linear_search_bits(&out, length, base, bits, in_data[i as usize]);
        assert_eq!(in_data[i as usize], in_data[index as usize]);
    }
}

#[test]
fn lowlevel_block32_all_bits() {
    for bits in 0..=32u32 {
        let in_data = generate_input(10, 32, bits);
        lowlevel_block_func(bits, FOR_PACK32[bits as usize], FOR_UNPACK32[bits as usize], &in_data, 10, 32);
    }
}

#[test]
fn lowlevel_block16_all_bits() {
    for bits in 0..=32u32 {
        let in_data = generate_input(10, 16, bits);
        lowlevel_block_func(bits, FOR_PACK16[bits as usize], FOR_UNPACK16[bits as usize], &in_data, 10, 16);
    }
}

#[test]
fn lowlevel_block8_all_bits() {
    for bits in 0..=32u32 {
        let in_data = generate_input(10, 8, bits);
        lowlevel_block_func(bits, FOR_PACK8[bits as usize], FOR_UNPACK8[bits as usize], &in_data, 10, 8);
    }
}

#[test]
fn lowlevel_blockx_all() {
    for bits in 0..32u32 {
        for b in 0u32..8 {
            let in_data = generate_input(10, 8, bits);
            lowlevel_blockx_func(bits, FOR_PACKX[bits as usize], FOR_UNPACKX[bits as usize], &in_data, 10, b);
        }
    }
}

fn highlevel_sorted(length: u32) {
    let mut in_data = vec![0u32; length as usize];
    let mut out = vec![0u8; 1024 * 10];
    let mut tmp = vec![0u32; 1024 * 10];
    for i in 0..length {
        in_data[i as usize] = 33 + i;
    }
    let s3 = forLib::for_compressed_size_sorted(&in_data, length);
    let s1 = forLib::for_compress_sorted(&in_data, &mut out, length);
    let s2 = forLib::for_uncompress(&out, &mut tmp, length);
    assert_eq!(s1, s2);
    assert_eq!(s2, s3);
    for i in 0..length as usize {
        assert_eq!(in_data[i], tmp[i], "highlevel sorted data mismatch at {}", i);
    }
    for i in 0..length {
        assert_eq!(in_data[i as usize], forLib::for_select(&out, i));
    }
    for i in 0..length {
        assert_eq!(i, forLib::for_linear_search(&out, length, in_data[i as usize]));
    }
    for i in 0..length {
        let mut actual = 0;
        let index = forLib::for_lower_bound_search(&out, length, in_data[i as usize], &mut actual);
        assert_eq!(in_data[i as usize], in_data[index as usize]);
        assert_eq!(actual, in_data[i as usize]);
    }
}

#[test]
fn highlevel_sorted_test() {
    for length in [0u32, 1, 2, 3, 16, 17, 32, 33, 64, 65, 128, 129, 256, 257, 1024, 1025, 1333] {
        highlevel_sorted(length);
    }
}

fn rnd_state(state: &mut u32) -> u32 {
    *state = ((*state).wrapping_mul(214013).wrapping_add(2531011) >> 16) & 32767;
    *state
}

fn highlevel_unsorted(length: u32, state: &mut u32) {
    let mut in_data = vec![0u32; length as usize];
    let mut out = vec![0u8; 1024 * 10];
    let mut tmp = vec![0u32; 1024 * 10];
    for i in 0..length {
        in_data[i as usize] = 7u32.wrapping_add(rnd_state(state).wrapping_sub(7));
    }
    let s3 = forLib::for_compressed_size_unsorted(&in_data, length);
    let s1 = forLib::for_compress_unsorted(&in_data, &mut out, length);
    let s2 = forLib::for_uncompress(&out, &mut tmp, length);
    assert_eq!(s1, s2);
    assert_eq!(s2, s3);
    for i in 0..length as usize {
        assert_eq!(in_data[i], tmp[i]);
    }
    for i in 0..length {
        assert_eq!(in_data[i as usize], forLib::for_select(&out, i));
    }
    for i in 0..length {
        let index = forLib::for_linear_search(&out, length, in_data[i as usize]);
        assert_eq!(in_data[i as usize], in_data[index as usize]);
    }
}

#[test]
fn highlevel_unsorted_test() {
    let mut state = 3u32;
    for length in [0u32, 1, 2, 3, 16, 17, 32, 33, 64, 65, 128, 129, 256, 257, 1024, 1025, 1333] {
        highlevel_unsorted(length, &mut state);
    }
}

#[test]
fn append_sorted_test() {
    const MAX: usize = 1000;
    let mut out1 = vec![0u8; MAX * 8];
    let mut out2 = vec![0u8; MAX * 8];
    let mut in_data = vec![0u32; MAX];
    for i in 0..MAX {
        in_data[i] = i as u32;
        let s1 = forLib::for_append_sorted(&mut out1, i as u32, in_data[i]);
        let s2 = forLib::for_compress_sorted(&in_data[..=i], &mut out2, (i + 1) as u32);
        assert_eq!(s1, s2, "size mismatch at i={}", i);
        for j in 0..s1 as usize {
            assert_eq!(out1[j], out2[j], "byte mismatch at i={} pos={}", i, j);
        }
    }
}

#[test]
fn append_unsorted_test() {
    const MAX: usize = 1000;
    let mut state = 3u32;
    let mut out1 = vec![0u8; MAX * 8];
    let mut out2 = vec![0u8; MAX * 8];
    let mut in_data = vec![0u32; MAX];
    for i in 0..MAX {
        in_data[i] = 7u32.wrapping_add(rnd_state(&mut state).wrapping_sub(7));
        let s1 = forLib::for_append_unsorted(&mut out1, i as u32, in_data[i]);
        let s2 = forLib::for_compress_unsorted(&in_data[..=i], &mut out2, (i + 1) as u32);
        assert_eq!(s1, s2, "size mismatch at i={}", i);
        for j in 0..s1 as usize {
            assert_eq!(out1[j], out2[j], "byte mismatch at i={} pos={}", i, j);
        }
    }
}

#[test]
fn append_sorted_bignum_test() {
    const MAX: usize = 10;
    let mut out1 = vec![0u8; MAX * 8];
    let mut out2 = vec![0u8; MAX * 8];
    let mut in_data = vec![0u32; MAX];
    for i in 0..MAX {
        in_data[i] = 1u32 << (17 + i);
        let s1 = forLib::for_append_sorted(&mut out1, i as u32, in_data[i]);
        let s2 = forLib::for_compress_sorted(&in_data[..=i], &mut out2, (i + 1) as u32);
        assert_eq!(s1, s2, "size mismatch at i={}", i);
        for j in 0..s1 as usize {
            assert_eq!(out1[j], out2[j], "byte mismatch at i={} pos={}", i, j);
        }
    }
}
