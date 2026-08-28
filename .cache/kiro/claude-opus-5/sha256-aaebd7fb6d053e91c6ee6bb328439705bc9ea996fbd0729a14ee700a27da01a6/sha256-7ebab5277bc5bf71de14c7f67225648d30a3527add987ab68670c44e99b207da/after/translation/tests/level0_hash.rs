//! Level 0: leaf functions with no dependencies —
//! `stbds_hash_string`, `stbds_hash_bytes`, `strkey`, `stbds_rand_seed`.

mod common;

use common::*;
use std::ffi::c_void;

#[test]
fn hash_string_matches() {
    let _g = serial();
    let (c, r) = apis();

    let seeds: [usize; 8] = [
        0,
        1,
        2,
        0x31415926,
        0xdead_beef,
        usize::MAX,
        0x8000_0000_0000_0000,
        0x0123_4567_89ab_cdef,
    ];

    let mut inputs: Vec<Vec<u8>> = Vec::new();
    inputs.push(b"".to_vec());
    inputs.push(b"a".to_vec());
    inputs.push(b"foo".to_vec());
    inputs.push(b"test_0".to_vec());
    inputs.push(b"test_123456".to_vec());
    inputs.push(b"The quick brown fox jumps over the lazy dog".to_vec());
    // high-bit bytes: `(unsigned char) *str` in C
    inputs.push(vec![0x80]);
    inputs.push(vec![0xff, 0xfe, 0x7f, 0x01]);
    inputs.push((1u8..=255u8).collect::<Vec<u8>>());
    for n in 0..64u32 {
        inputs.push(
            (0..n)
                .map(|i| ((i as u32).wrapping_mul(37).wrapping_add(1) % 255 + 1) as u8)
                .collect(),
        );
    }

    for inp in &inputs {
        let mut sc = CStr8::from_bytes(inp);
        let mut sr = CStr8::from_bytes(inp);
        for &seed in &seeds {
            let hc = unsafe { (c.hash_string)(sc.as_ptr(), seed) };
            let hr = unsafe { (r.hash_string)(sr.as_ptr(), seed) };
            assert_eq!(
                hc, hr,
                "stbds_hash_string mismatch for {:?} seed {:#x}",
                String::from_utf8_lossy(inp),
                seed
            );
        }
    }
}

#[test]
fn hash_bytes_matches() {
    let _g = serial();
    let (c, r) = apis();

    let seeds: [usize; 8] = [
        0,
        1,
        0x31415926,
        0xdead_beef_cafe_babe,
        usize::MAX,
        0x8000_0000_0000_0000,
        0x0706_0504_0302_0100,
        0x5555_5555_5555_5555,
    ];

    // Exercise every `len % 8` remainder and the sign-extension quirks of the
    // byte loader (bytes >= 0x80 in positions 3 and 7).
    let mut buffers: Vec<Vec<u8>> = Vec::new();
    for len in 0..40usize {
        buffers.push(vec![0u8; len]);
        buffers.push(vec![0xffu8; len]);
        buffers.push((0..len).map(|i| i as u8).collect());
        buffers.push((0..len).map(|i| (0x80 + i) as u8).collect());
        buffers.push((0..len).map(|i| (i as u8).wrapping_mul(31) ^ 0x80).collect());
        buffers.push(
            (0..len)
                .map(|i| if i % 4 == 3 { 0x80 } else { i as u8 })
                .collect(),
        );
        buffers.push(
            (0..len)
                .map(|i| if i % 8 == 7 { 0xff } else { 0x01 })
                .collect(),
        );
    }
    // A couple of long buffers
    buffers.push((0..257).map(|i| (i * 7) as u8).collect());
    buffers.push(vec![0x80u8; 128]);

    for buf in &buffers {
        let mut bc = buf.clone();
        let mut br = buf.clone();
        for &seed in &seeds {
            let hc =
                unsafe { (c.hash_bytes)(bc.as_mut_ptr() as *mut c_void, buf.len(), seed) };
            let hr =
                unsafe { (r.hash_bytes)(br.as_mut_ptr() as *mut c_void, buf.len(), seed) };
            assert_eq!(
                hc, hr,
                "stbds_hash_bytes mismatch len={} seed={:#x} buf={:02x?}",
                buf.len(),
                seed,
                buf
            );
        }
    }
}

#[test]
fn strkey_matches() {
    let _g = serial();
    let (c, r) = apis();

    for n in [
        0,
        1,
        9,
        10,
        99,
        100,
        12345,
        -1,
        -42,
        i32::MAX,
        i32::MIN,
        1_000_000,
    ] {
        let sc = unsafe { cstr((c.strkey)(n)) };
        let sr = unsafe { cstr((r.strkey)(n)) };
        assert_eq!(
            sc,
            sr,
            "strkey({}) mismatch: {:?} vs {:?}",
            n,
            String::from_utf8_lossy(&sc),
            String::from_utf8_lossy(&sr)
        );
        assert_eq!(sc, format!("test_{}", n).into_bytes());
    }
}

#[test]
fn rand_seed_affects_table_seed_identically() {
    let _g = serial();
    let (c, r) = apis();

    // The seed is only observable through the tables that consume it, so drive
    // `stbds_shmode_func` (which builds a fresh hash index) after seeding and
    // compare the resulting `seed` fields and the seed evolution.
    for &seed in &[0usize, 1, 0x31415926, usize::MAX, 0xabcd_ef01_2345_6789] {
        reset_seeds(&c, &r, seed);
        for _ in 0..6 {
            let tc = unsafe { (c.shmode_func)(ELEMSIZE, STBDS_SH_ARENA) };
            let tr = unsafe { (r.shmode_func)(ELEMSIZE, STBDS_SH_ARENA) };
            let sc = unsafe { snapshot_map(tc as *mut StrMapEntry) };
            let sr = unsafe { snapshot_map(tr as *mut StrMapEntry) };
            assert_eq!(sc, sr, "seeded table mismatch (seed {:#x})", seed);
            unsafe {
                (c.hmfree_func)(tc.cast::<StrMapEntry>().sub(1) as *mut c_void, ELEMSIZE);
                (r.hmfree_func)(tr.cast::<StrMapEntry>().sub(1) as *mut c_void, ELEMSIZE);
            }
        }
    }
}
