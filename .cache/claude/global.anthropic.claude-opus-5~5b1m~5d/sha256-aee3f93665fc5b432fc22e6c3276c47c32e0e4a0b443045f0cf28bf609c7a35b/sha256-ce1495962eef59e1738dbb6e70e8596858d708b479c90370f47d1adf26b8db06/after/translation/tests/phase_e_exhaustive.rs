//! Extra-strength exhaustive differential sweeps.
//!
//! These are `#[ignore]`d because they take minutes; run them with
//!   cargo test --release --offline --test phase_e_exhaustive -- --ignored --nocapture
//! (also run in the default matrix via `run_all.sh` is NOT done — they are the
//! long-tail confirmation of the randomized rows in `CONFIGS.md`).

mod common;
use common::*;

/// Every one of the 2^32 possible `blocksize` values, C vs Rust.
#[test]
#[ignore]
fn exhaustive_size_memory_full_u32() {
    let l = libs();
    let mut bs: u32 = 0;
    loop {
        let c = unsafe { (l.c.size_memory)(bs) };
        let r = unsafe { (l.rust.size_memory)(bs) };
        if c != r {
            panic!("tflac_size_memory({bs}) mismatch: C={c:#010x} Rust={r:#010x}");
        }
        match bs.checked_add(1) {
            Some(n) => bs = n,
            None => break,
        }
    }
    println!("tflac_size_memory: all 2^32 inputs identical");
}

/// Every `blocksize` in `0..=70000` crossed with every `(min, max)`
/// partition-order pair in `0..=16` (covers both sides of the `> 15` check and
/// every reachable iteration count of the loop).
#[test]
#[ignore]
fn exhaustive_validate_blocksize_x_partition_orders() {
    let l = libs();
    let mut checked: u64 = 0;
    for bs in 0u32..=70_000 {
        for max_po in 0u8..=16 {
            for min_po in 0u8..=16 {
                let f = Fields {
                    blocksize: bs,
                    samplerate: 44_100,
                    channels: 2,
                    bitdepth: 16,
                    channel_mode: 1,
                    max_rice_value: 0,
                    min_partition_order: min_po,
                    max_partition_order: max_po,
                    partition_order: 0xEE,
                    padding: [0xAA, 0xBB, 0xCC],
                    cur_blocksize: 0xDEAD_BEEF,
                };
                let mut cbuf = f.to_raw();
                let mut rbuf = f.to_raw();
                let cret = unsafe { (l.c.validate)(cbuf.0.as_mut_ptr()) };
                let rret = unsafe { (l.rust.validate)(rbuf.0.as_mut_ptr()) };
                if cret != rret || cbuf != rbuf {
                    panic!(
                        "mismatch for {f:?}: C ret={cret} {:?} | Rust ret={rret} {:?}",
                        Fields::from_raw(cbuf),
                        Fields::from_raw(rbuf)
                    );
                }
                checked += 1;
            }
        }
    }
    println!("flac_validate: {checked} blocksize x partition-order cases identical");
}

/// Every `(channel_mode, max_rice_value)` byte pair crossed with every
/// `(channels, bitdepth)` pair in `0..=9` / `0..=33`.
#[test]
#[ignore]
fn exhaustive_validate_mode_x_rice_x_channels_x_bitdepth() {
    let l = libs();
    let mut checked: u64 = 0;
    for mode in 0u8..=255 {
        for rice in 0u8..=255 {
            for ch in 0u32..=9 {
                for bd in 0u32..=33 {
                    let f = Fields {
                        blocksize: 4096,
                        samplerate: 44_100,
                        channels: ch,
                        bitdepth: bd,
                        channel_mode: mode,
                        max_rice_value: rice,
                        min_partition_order: 3,
                        max_partition_order: 15,
                        partition_order: 0x77,
                        padding: [1, 2, 3],
                        cur_blocksize: 0x1234_5678,
                    };
                    let mut cbuf = f.to_raw();
                    let mut rbuf = f.to_raw();
                    let cret = unsafe { (l.c.validate)(cbuf.0.as_mut_ptr()) };
                    let rret = unsafe { (l.rust.validate)(rbuf.0.as_mut_ptr()) };
                    if cret != rret || cbuf != rbuf {
                        panic!(
                            "mismatch for {f:?}: C ret={cret} {:?} | Rust ret={rret} {:?}",
                            Fields::from_raw(cbuf),
                            Fields::from_raw(rbuf)
                        );
                    }
                    checked += 1;
                }
            }
        }
    }
    println!("flac_validate: {checked} mode/rice/channels/bitdepth cases identical");
}

/// Every `(min_partition_order, max_partition_order)` byte pair (all 65536),
/// including all the out-of-range ones.
#[test]
#[ignore]
fn exhaustive_validate_all_partition_order_byte_pairs() {
    let l = libs();
    for bs in [16u32, 4096, 32768, 65535, 49152, 1024] {
        for max_po in 0u8..=255 {
            for min_po in 0u8..=255 {
                let f = Fields {
                    blocksize: bs,
                    min_partition_order: min_po,
                    max_partition_order: max_po,
                    partition_order: 0x33,
                    ..Default::default()
                };
                let mut cbuf = f.to_raw();
                let mut rbuf = f.to_raw();
                let cret = unsafe { (l.c.validate)(cbuf.0.as_mut_ptr()) };
                let rret = unsafe { (l.rust.validate)(rbuf.0.as_mut_ptr()) };
                if cret != rret || cbuf != rbuf {
                    panic!(
                        "mismatch for {f:?}: C ret={cret} {:?} | Rust ret={rret} {:?}",
                        Fields::from_raw(cbuf),
                        Fields::from_raw(rbuf)
                    );
                }
            }
        }
    }
    println!("flac_validate: all 65536 partition-order byte pairs x 6 blocksizes identical");
}
