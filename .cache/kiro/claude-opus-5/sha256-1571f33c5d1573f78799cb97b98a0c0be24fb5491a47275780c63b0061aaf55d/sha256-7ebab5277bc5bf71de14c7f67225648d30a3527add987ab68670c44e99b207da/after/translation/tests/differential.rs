//! Differential tests: C `libharvest-*.so` vs Rust `libcrc16_lib.so`.
//!
//! Ordered from the lowest-level behaviour upward:
//!   1. degenerate/empty input (no loop body runs)
//!   2. the byte-at-a-time tail loop (`len < 8`) — exercises table 0 only
//!   3. exactly one slice-by-8 block (`len == 8`) — exercises tables 0..7
//!   4. multi-block + tail, seeds, alignment, and large buffers
//!
//! Every call goes through `dlsym`'d `crc16` on both sides.

mod common;

use common::{load_pair, Rng};

// ---------------------------------------------------------------------------
// 1. Degenerate input
// ---------------------------------------------------------------------------

#[test]
fn empty_input_all_seeds() {
    let p = load_pair();
    // len == 0: neither loop body executes, the seed must come straight back.
    for seed in 0u32..=0xFFFF {
        let seed = seed as u16;
        let empty: [u8; 0] = [];
        let c = p.c.crc16_slice(&empty, seed);
        let r = p.rust.crc16_slice(&empty, seed);
        assert_eq!(c, r, "empty input, seed {seed:#06x}");
    }
}

#[test]
fn empty_input_null_pointer() {
    let p = load_pair();
    // The C code never dereferences `d` when len == 0, so a null pointer is a
    // legitimate call an external caller could make.
    for seed in [0u16, 1, 0x1234, 0x8000, 0xFFFF] {
        // SAFETY: len == 0, so neither implementation reads through `d`.
        let c = unsafe { p.c.crc16(std::ptr::null(), 0, seed) };
        // SAFETY: same.
        let r = unsafe { p.rust.crc16(std::ptr::null(), 0, seed) };
        assert_eq!(c, r, "null/len0, seed {seed:#06x}");
    }
}

// ---------------------------------------------------------------------------
// 2. Tail loop only (len 1..=7): table 0, `(crc << 8) ^ table0[(crc >> 8) ^ b]`
// ---------------------------------------------------------------------------

#[test]
fn single_byte_every_value_every_high_seed() {
    let p = load_pair();
    // One byte with len == 1 hits only the tail loop. Sweeping all 256 byte
    // values against all 256 high seed bytes covers every entry of table 0
    // and every index computation.
    for b in 0u16..=0xFF {
        let buf = [b as u8];
        for hi in 0u16..=0xFF {
            for lo in [0x00u16, 0x5A, 0xFF] {
                let seed = (hi << 8) | lo;
                let c = p.c.crc16_slice(&buf, seed);
                let r = p.rust.crc16_slice(&buf, seed);
                assert_eq!(c, r, "byte {b:#04x}, seed {seed:#06x}");
            }
        }
    }
}

#[test]
fn tail_lengths_one_to_seven_exhaustive_patterns() {
    let p = load_pair();
    let mut rng = Rng::new(0xC0FFEE);
    for len in 1usize..=7 {
        // Fixed edge patterns plus random ones.
        let mut cases: Vec<Vec<u8>> = vec![
            vec![0x00; len],
            vec![0xFF; len],
            vec![0x80; len],
            vec![0x01; len],
            (0..len).map(|i| i as u8).collect(),
            (0..len).map(|i| 0xFFu8 - i as u8).collect(),
        ];
        for _ in 0..200 {
            cases.push(rng.bytes(len));
        }
        for data in &cases {
            for seed in [0u16, 1, 0x00FF, 0xFF00, 0x8005, 0xBEEF, 0xFFFF] {
                let c = p.c.crc16_slice(data, seed);
                let r = p.rust.crc16_slice(data, seed);
                assert_eq!(c, r, "len {len}, data {data:02x?}, seed {seed:#06x}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 3. One slice-by-8 block: tables 0..7 and the `crc ^= d[0]<<8|d[1]` step
// ---------------------------------------------------------------------------

#[test]
fn single_block_walking_bytes() {
    let p = load_pair();
    // For each of the 8 positions in a block, sweep all 256 values with the
    // other positions held at 0 and at 0xFF. This isolates every table lookup
    // (`tables[7-i]` for position i) across both zero and saturated context.
    for pos in 0..8usize {
        for fill in [0x00u8, 0xFFu8] {
            for v in 0u16..=0xFF {
                let mut buf = [fill; 8];
                buf[pos] = v as u8;
                for seed in [0u16, 0x00FF, 0xFF00, 0xFFFF, 0x1234] {
                    let c = p.c.crc16_slice(&buf, seed);
                    let r = p.rust.crc16_slice(&buf, seed);
                    assert_eq!(
                        c, r,
                        "pos {pos}, fill {fill:#04x}, v {v:#04x}, seed {seed:#06x}"
                    );
                }
            }
        }
    }
}

#[test]
fn single_block_seed_sweep() {
    let p = load_pair();
    // `crc16 ^= d[0] << 8 | d[1]` then indexes tables[7]/[6] by the halves of
    // the result: sweep all 65536 seeds against a fixed block.
    let buf: [u8; 8] = [0x00, 0xFF, 0x7F, 0x80, 0x01, 0xFE, 0xAA, 0x55];
    for seed in 0u32..=0xFFFF {
        let seed = seed as u16;
        let c = p.c.crc16_slice(&buf, seed);
        let r = p.rust.crc16_slice(&buf, seed);
        assert_eq!(c, r, "single block, seed {seed:#06x}");
    }
}

// ---------------------------------------------------------------------------
// 4. Multi-block, tails, seeds, alignment, large buffers
// ---------------------------------------------------------------------------

#[test]
fn all_lengths_up_to_512() {
    let p = load_pair();
    let mut rng = Rng::new(0xDEADBEEF);
    let data = rng.bytes(512);
    for len in 0..=512usize {
        let slice = &data[..len];
        for seed in [0u16, 1, 0x00FF, 0xFF00, 0x8005, 0xBEEF, 0xFFFF] {
            let c = p.c.crc16_slice(slice, seed);
            let r = p.rust.crc16_slice(slice, seed);
            assert_eq!(c, r, "len {len}, seed {seed:#06x}");
        }
    }
}

#[test]
fn block_boundary_lengths() {
    let p = load_pair();
    let mut rng = Rng::new(0x5EED);
    let data = rng.bytes(4096);
    // Lengths straddling every multiple-of-8 boundary, where the split between
    // the slice-by-8 loop and the tail loop changes.
    for base in (0..=4088usize).step_by(8) {
        for delta in 0..8usize {
            let len = base + delta;
            if len > data.len() {
                continue;
            }
            let slice = &data[..len];
            let c = p.c.crc16_slice(slice, 0xACE1);
            let r = p.rust.crc16_slice(slice, 0xACE1);
            assert_eq!(c, r, "len {len}");
        }
    }
}

#[test]
fn unaligned_start_offsets() {
    let p = load_pair();
    let mut rng = Rng::new(0xA11A11);
    let data = rng.bytes(1024);
    // Slide the start offset so the pointer hits every alignment mod 16.
    for off in 0..64usize {
        for len in [0usize, 1, 7, 8, 9, 15, 16, 31, 64, 127, 256] {
            if off + len > data.len() {
                continue;
            }
            let slice = &data[off..off + len];
            for seed in [0u16, 0xFFFF, 0x1357] {
                let c = p.c.crc16_slice(slice, seed);
                let r = p.rust.crc16_slice(slice, seed);
                assert_eq!(c, r, "off {off}, len {len}, seed {seed:#06x}");
            }
        }
    }
}

#[test]
fn randomized_fuzz() {
    let p = load_pair();
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for i in 0..20_000u32 {
        let len = (rng.next_u64() % 300) as usize;
        let data = rng.bytes(len);
        let seed = rng.next_u16();
        let c = p.c.crc16_slice(&data, seed);
        let r = p.rust.crc16_slice(&data, seed);
        assert_eq!(c, r, "iter {i}, len {len}, seed {seed:#06x}");
    }
}

#[test]
fn large_buffers() {
    let p = load_pair();
    let mut rng = Rng::new(0xFACADE);
    for size in [64 * 1024usize, 1024 * 1024, 3 * 1024 * 1024 + 5] {
        let data = rng.bytes(size);
        for seed in [0u16, 0xFFFF, 0x2718] {
            let c = p.c.crc16_slice(&data, seed);
            let r = p.rust.crc16_slice(&data, seed);
            assert_eq!(c, r, "size {size}, seed {seed:#06x}");
        }
        // Degenerate content of the same size.
        for fill in [0x00u8, 0xFFu8] {
            let filled = vec![fill; size];
            let c = p.c.crc16_slice(&filled, 0x0000);
            let r = p.rust.crc16_slice(&filled, 0x0000);
            assert_eq!(c, r, "size {size}, fill {fill:#04x}");
        }
    }
}

#[test]
fn incremental_chaining_matches() {
    let p = load_pair();
    // Feeding the result back in as the seed is the intended streaming use;
    // both sides must agree at every step and with the one-shot call.
    let mut rng = Rng::new(0x600D_5EED);
    let data = rng.bytes(5000);
    for chunk in [1usize, 2, 3, 7, 8, 9, 16, 61, 512] {
        let mut c_acc = 0u16;
        let mut r_acc = 0u16;
        for part in data.chunks(chunk) {
            c_acc = p.c.crc16_slice(part, c_acc);
            r_acc = p.rust.crc16_slice(part, r_acc);
            assert_eq!(c_acc, r_acc, "chunk {chunk}");
        }
        let c_one = p.c.crc16_slice(&data, 0);
        let r_one = p.rust.crc16_slice(&data, 0);
        assert_eq!(c_one, r_one);
    }
}

#[test]
fn does_not_mutate_input_buffer() {
    let p = load_pair();
    let mut rng = Rng::new(0xB0B0);
    let data = rng.bytes(333);
    let before = data.clone();
    let _ = p.c.crc16_slice(&data, 0x1111);
    assert_eq!(data, before, "C mutated its input");
    let _ = p.rust.crc16_slice(&data, 0x1111);
    assert_eq!(data, before, "Rust mutated its input");
}
