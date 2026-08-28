//! Level 2: buffer functions (`create_numeric_buffer`, `find_value_in_buffer`).

mod common;

use common::both;

/// Fill the buffer under a known sentinel pattern so that untouched bytes are
/// also compared; both implementations get an identical starting state.
fn filled(len: usize) -> Vec<u8> {
    (0..len).map(|i| (0xA5u8).wrapping_add(i as u8)).collect()
}

#[test]
fn create_numeric_buffer_matches() {
    let (c, rust) = both();

    let seeds: [i32; 25] = [
        0,
        1,
        -1,
        7,
        -7,
        42,
        -42,
        127,
        128,
        -128,
        255,
        256,
        -255,
        -256,
        1000,
        -1000,
        65535,
        -65536,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        i32::MAX - 1785,
        2147483000,
        -2147483000,
    ];
    let sizes: [i32; 12] = [0, 1, 2, 3, 8, 31, 32, 63, 100, 255, 256, 300];

    for &size in &sizes {
        // Allocate generously so a size larger than the "logical" length is
        // still in-bounds for both implementations.
        let cap = 512usize;
        for &seed in &seeds {
            let mut cbuf = filled(cap);
            let mut rbuf = filled(cap);
            c.create_numeric_buffer(&mut cbuf, size, seed);
            rust.create_numeric_buffer(&mut rbuf, size, seed);
            assert_eq!(
                cbuf, rbuf,
                "create_numeric_buffer(size={size}, seed={seed}) buffers differ"
            );
        }
    }

    // Non-positive sizes must be no-ops in both.
    for &size in &[0i32, -1, -100, i32::MIN] {
        let mut cbuf = filled(64);
        let mut rbuf = filled(64);
        c.create_numeric_buffer(&mut cbuf, size, 12345);
        rust.create_numeric_buffer(&mut rbuf, size, 12345);
        assert_eq!(cbuf, filled(64), "C mutated buffer for size={size}");
        assert_eq!(rbuf, filled(64), "Rust mutated buffer for size={size}");
    }
}

#[test]
fn find_value_in_buffer_matches() {
    let (c, rust) = both();

    // A set of buffers with varied content, including embedded NULs.
    let mut buffers: Vec<Vec<u8>> = vec![
        vec![],
        vec![0],
        vec![0, 0, 0, 0],
        vec![42],
        (0u8..=255).collect(),
        (0u8..=255).rev().collect(),
        vec![0xffu8; 64],
        b"hello, world\0trailing".to_vec(),
    ];
    // Buffers produced by the library's own generator.
    for seed in [0i32, 1, -1, 42, 255, i32::MAX, i32::MIN] {
        let mut b = vec![0u8; 256];
        c.create_numeric_buffer(&mut b, 256, seed);
        buffers.push(b);
    }

    let mut search_vals: Vec<i32> = Vec::new();
    for v in -300i32..=300 {
        search_vals.push(v);
    }
    search_vals.extend_from_slice(&[i32::MAX, i32::MIN, 0x1_0000, 0x1_00ff, -0x1_0000, 100, 42]);

    for buf in &buffers {
        for &sv in &search_vals {
            let a = c.find_value_in_buffer(buf, sv);
            let b = rust.find_value_in_buffer(buf, sv);
            assert_eq!(
                a, b,
                "find_value_in_buffer(len={}, search_val={sv}): C={a}, Rust={b}",
                buf.len()
            );
        }
    }

    // Sub-slice sizes: the search must respect `size`, not the allocation.
    let full: Vec<u8> = (0u8..=255).collect();
    for size in [0usize, 1, 5, 42, 43, 100, 255, 256] {
        for sv in [0i32, 1, 42, 99, 100, 254, 255, 256, -1, -214] {
            let a = c.find_value_in_buffer(&full[..size], sv);
            let b = rust.find_value_in_buffer(&full[..size], sv);
            assert_eq!(
                a, b,
                "find_value_in_buffer(size={size}, search_val={sv}): C={a}, Rust={b}"
            );
        }
    }
}
