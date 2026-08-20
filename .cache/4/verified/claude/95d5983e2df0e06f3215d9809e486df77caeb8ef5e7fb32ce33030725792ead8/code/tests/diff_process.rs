//! Phase B — valid-path differential tests for `process_buffer_array`
//! (CONFIGS.md rows 62–71).
//!
//! The array is always allocated through the library's own
//! `init_buffer_array` and released through its own `free_buffer_array`, i.e.
//! the composed pipeline is driven exactly the way a C consumer would.

mod common;

use common::*;

/// What a `process_buffer_array` run produced: the return code plus the state
/// of every buffer in the array.
type Snapshot = (i64, Vec<(usize, u32, Vec<u8>)>);

/// `full = true` compares all 256 data bytes; `full = false` only the first
/// `length` bytes.  The latter is required for `OP_MERGE`, where the C original
/// copies a whole `buffer_t` from an *uninitialized* stack object, so every byte
/// past `merged.length` is indeterminate in C.
fn run_one(
    api: &Api,
    cap: i32,
    count: i32,
    op: i32,
    param: i32,
    bufs: &[BufferT],
    full: bool,
) -> Obs<Snapshot> {
    assert!(bufs.len() <= cap.max(0) as usize);
    observe(None, || unsafe {
        let arr = (api.init_buffer_array)(cap);
        assert!(!arr.is_null(), "init_buffer_array({}) returned NULL", cap);
        let storage = (*arr).buffers;
        for (i, b) in bufs.iter().enumerate() {
            *storage.add(i) = *b;
        }
        (*arr).count = count;

        let rc = (api.process_buffer_array)(arr, op, param) as i64;

        let mut snap = Vec::with_capacity(bufs.len());
        for i in 0..bufs.len() {
            let b = &*storage.add(i);
            let n = if full { 256 } else { b.length.min(256) };
            snap.push((b.length, b.checksum, b.data[..n].to_vec()));
        }
        (api.free_buffer_array)(arr);
        (rc, snap)
    })
}

#[allow(clippy::too_many_arguments)]
#[track_caller]
fn diff_process(
    what: &str,
    cap: i32,
    count: i32,
    op: i32,
    param: i32,
    setup: impl Fn() -> Vec<BufferT>,
    full: bool,
) {
    let (c, r) = both();
    let cb = setup();
    let rb = setup();
    for i in 0..cb.len() {
        assert!(
            cb[i].full_repr() == rb[i].full_repr(),
            "{}: setup not deterministic",
            what
        );
    }
    let co = run_one(c, cap, count, op, param, &cb, full);
    let ro = run_one(r, cap, count, op, param, &rb, full);
    same(what, &co, &ro);
}

/// A deterministic working set of `n` buffers with a variety of lengths.
fn mixed_bufs(seed: u64, n: usize, maxlen: usize) -> Vec<BufferT> {
    let mut g = Rng::new(seed);
    (0..n)
        .map(|i| {
            let len = match i % 5 {
                0 => 0,
                1 => 1,
                2 => g.below(maxlen + 1),
                3 => maxlen,
                _ => g.below(maxlen / 2 + 1),
            };
            g.buffer_len(len)
        })
        .collect()
}

// ========================================================== rows 62–63 =====

#[test]
fn row62_op_copy_various_counts() {
    let mut rng = Rng::new(0x62);
    for count in [1i32, 2, 3, 4, 5, 10, 17] {
        for _ in 0..8 {
            let seed = rng.next_u64();
            let n = count as usize;
            diff_process(
                "row62",
                count,
                count,
                OP_COPY,
                0,
                || mixed_bufs(seed, n, 120),
                true,
            );
        }
    }
    // count smaller than capacity
    for _ in 0..20 {
        let seed = rng.next_u64();
        diff_process("row62/partial", 10, 4, OP_COPY, 0, || mixed_bufs(seed, 10, 200), true);
    }
}

#[test]
fn row63_op_copy_with_corrupt_first_checksum() {
    let mut rng = Rng::new(0x63);
    for count in [2i32, 3, 6] {
        for _ in 0..10 {
            let seed = rng.next_u64();
            let n = count as usize;
            diff_process(
                "row63",
                count,
                count,
                OP_COPY,
                0,
                move || {
                    let mut v = mixed_bufs(seed, n, 100);
                    // Force the checksum warning on every inner buffer_copy.
                    v[0].checksum ^= 0xFFFF_FFFF;
                    if v[0].length == 0 {
                        v[0].length = 3;
                    }
                    v
                },
                true,
            );
        }
    }
}

// ============================================================== row 64 =====

#[test]
fn row64_op_reverse() {
    let mut rng = Rng::new(0x64);
    for count in [1i32, 2, 3, 5, 8, 10] {
        for _ in 0..10 {
            let seed = rng.next_u64();
            let n = count as usize;
            diff_process(
                "row64",
                count,
                count,
                OP_REVERSE,
                0,
                || mixed_bufs(seed, n, 256),
                true,
            );
        }
    }
}

// ========================================================== rows 65–66 =====

#[test]
fn row65_op_merge_even_count() {
    let mut rng = Rng::new(0x65);
    for count in [2i32, 4, 6, 8, 10] {
        for _ in 0..10 {
            let seed = rng.next_u64();
            let n = count as usize;
            // Keep every pair's combined length <= 256 so the happy path runs.
            diff_process(
                "row65",
                count,
                count,
                OP_MERGE,
                0,
                || mixed_bufs(seed, n, 100),
                false, // C copies an uninitialized stack tail
            );
        }
    }
}

#[test]
fn row66_op_merge_odd_count() {
    let mut rng = Rng::new(0x66);
    for count in [3i32, 5, 7, 9] {
        for _ in 0..10 {
            let seed = rng.next_u64();
            let n = count as usize;
            diff_process(
                "row66",
                count,
                count,
                OP_MERGE,
                0,
                || mixed_bufs(seed, n, 100),
                false,
            );
        }
    }
}

// ============================================================== row 67 =====

#[test]
fn row67_op_rotate_param_variations() {
    let mut rng = Rng::new(0x67);
    let params = [
        0i32,
        1,
        2,
        7,
        -1,
        -7,
        255,
        256,
        257,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
    ];
    for &param in &params {
        for count in [1i32, 3, 5] {
            let seed = rng.next_u64();
            let n = count as usize;
            diff_process(
                "row67",
                count,
                count,
                OP_ROTATE,
                param,
                || mixed_bufs(seed, n, 256),
                true,
            );
        }
    }
    for _ in 0..60 {
        let seed = rng.next_u64();
        let param = rng.i32();
        diff_process("row67/rand", 6, 6, OP_ROTATE, param, || mixed_bufs(seed, 6, 256), true);
    }
}

// ========================================================== rows 68–69 =====

#[test]
fn row68_op_checksum_all_consistent() {
    let mut rng = Rng::new(0x68);
    for count in [1i32, 2, 5, 9] {
        for _ in 0..10 {
            let seed = rng.next_u64();
            let n = count as usize;
            diff_process(
                "row68",
                count,
                count,
                OP_CHECKSUM,
                0,
                || mixed_bufs(seed, n, 256),
                true,
            );
        }
    }
}

#[test]
fn row69_op_checksum_some_corrupted() {
    let mut rng = Rng::new(0x69);
    for count in [1i32, 3, 6, 9] {
        for _ in 0..10 {
            let seed = rng.next_u64();
            let n = count as usize;
            let corrupt = rng.next_u64();
            diff_process(
                "row69",
                count,
                count,
                OP_CHECKSUM,
                0,
                move || {
                    let mut v = mixed_bufs(seed, n, 200);
                    let mut g = Rng::new(corrupt);
                    for b in v.iter_mut() {
                        if g.bool() {
                            b.checksum = g.next_u32();
                        }
                    }
                    v
                },
                true,
            );
        }
    }
}

// ============================================================== row 70 =====

#[test]
fn row70_negative_count_loops_never_run() {
    // `process_buffer_array` only rejects `count == 0`; a negative count makes
    // every `for` loop body unreachable, so COPY/REVERSE/ROTATE/CHECKSUM all
    // return 0 without touching anything.
    let mut rng = Rng::new(0x70);
    for &op in &[OP_COPY, OP_REVERSE, OP_ROTATE, OP_CHECKSUM] {
        for count in [-1i32, -2, -100, i32::MIN, i32::MIN + 1] {
            let seed = rng.next_u64();
            diff_process(
                "row70",
                4,
                count,
                op,
                3,
                || mixed_bufs(seed, 4, 50),
                true,
            );
        }
    }
}

// ============================================================== row 71 =====

#[test]
fn row71_array_allocated_by_the_other_library() {
    // The `buffer_array_t` layout and the malloc/free contract must be
    // interchangeable: allocate with one library, process with the other.
    let (c, r) = both();
    let mut rng = Rng::new(0x71);
    for &op in &[OP_COPY, OP_REVERSE, OP_CHECKSUM, OP_ROTATE] {
        for _ in 0..6 {
            let seed = rng.next_u64();
            let bufs = mixed_bufs(seed, 4, 100);

            // C allocates, Rust processes.
            let a = observe(None, || unsafe {
                let arr = (c.init_buffer_array)(4);
                let st = (*arr).buffers;
                for (i, b) in bufs.iter().enumerate() {
                    *st.add(i) = *b;
                }
                (*arr).count = 4;
                let rc = (r.process_buffer_array)(arr, op, 3) as i64;
                let snap: Vec<_> = (0..4)
                    .map(|i| {
                        let b = &*st.add(i);
                        (b.length, b.checksum, b.data.to_vec())
                    })
                    .collect();
                (c.free_buffer_array)(arr);
                (rc, snap)
            });

            // Rust allocates, C processes.
            let b2 = observe(None, || unsafe {
                let arr = (r.init_buffer_array)(4);
                let st = (*arr).buffers;
                for (i, b) in bufs.iter().enumerate() {
                    *st.add(i) = *b;
                }
                (*arr).count = 4;
                let rc = (c.process_buffer_array)(arr, op, 3) as i64;
                let snap: Vec<_> = (0..4)
                    .map(|i| {
                        let b = &*st.add(i);
                        (b.length, b.checksum, b.data.to_vec())
                    })
                    .collect();
                (r.free_buffer_array)(arr);
                (rc, snap)
            });

            same(&format!("row71/op={}", op), &a, &b2);
        }
    }
}
