// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH the C `.so` and the
// Rust `.so` through `libloading` and compares the results byte-for-byte, using
// many randomized inputs per row (deterministic SplitMix64, fixed seed).

mod common;

use common::*;
use std::ffi::c_int;

/// The `len` values the C code distinguishes: 0 / 1 / 2 / 3 / small / many /
/// large, plus values around glibc `memcpy` size-class thresholds.
const LENS: &[c_int] = &[
    0, 1, 2, 3, 4, 5, 7, 8, 16, 31, 32, 63, 64, 127, 128, 255, 256, 1024,
];

/// Repetitions per (row, len) pair.
const REPS: usize = 24;

// ---------------------------------------------------------------------------
// Row 1 — fma_array, four distinct buffers, every len, random full-range
// ---------------------------------------------------------------------------
#[test]
fn cfg_01_fma_distinct_all_lens_random() {
    let mut rng = Rng::new(1);
    for &len in LENS {
        for rep in 0..REPS {
            let n = len.max(0) as usize;
            // A single scratch buffer holds all four arrays back to back, so the
            // "four distinct buffers" case is offsets 0, n, 2n, 3n.
            // A margin element keeps the offsets valid even for len == 0.
            let total = 4 * n + 4;
            let mut scratch = vec![0 as c_int; total];
            rng.fill_full(&mut scratch);
            // Poison the destination region so we can see exactly what is written.
            for (i, s) in scratch[..n].iter_mut().enumerate() {
                *s = 0x5A5A_0000u32 as c_int ^ (i as c_int);
            }
            diff_fma_layout(
                &format!("row1 len={len} rep={rep}"),
                &scratch,
                (0, n + 1, 2 * n + 2, 3 * n + 3),
                len,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 2 — fma_array, large len (65536)
// ---------------------------------------------------------------------------
#[test]
fn cfg_02_fma_distinct_large() {
    let mut rng = Rng::new(2);
    let len: c_int = 65_536;
    let n = len as usize;
    for rep in 0..3 {
        let mut scratch = vec![0 as c_int; 4 * n + 4];
        rng.fill_full(&mut scratch);
        diff_fma_layout(
            &format!("row2 rep={rep}"),
            &scratch,
            (0, n + 1, 2 * n + 2, 3 * n + 3),
            len,
        );
    }
}

// ---------------------------------------------------------------------------
// Row 3 — fma_array, small-magnitude values (no signed overflow)
// ---------------------------------------------------------------------------
#[test]
fn cfg_03_fma_distinct_no_overflow() {
    let mut rng = Rng::new(3);
    for &len in &[1, 2, 3, 8, 64] {
        for rep in 0..REPS {
            let n = len as usize;
            let mut scratch = vec![0 as c_int; 4 * n + 4];
            rng.fill_small(&mut scratch);
            diff_fma_layout(
                &format!("row3 len={len} rep={rep}"),
                &scratch,
                (0, n + 1, 2 * n + 2, 3 * n + 3),
                len,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4 — fma_array, constant value patterns
// ---------------------------------------------------------------------------
#[test]
fn cfg_04_fma_distinct_constant_patterns() {
    let patterns: Vec<(&str, Box<dyn Fn(usize) -> c_int>)> = vec![
        ("zeros", Box::new(|_| 0)),
        ("ones", Box::new(|_| 1)),
        ("neg_ones", Box::new(|_| -1)),
        ("int_max", Box::new(|_| INT_MAX)),
        ("int_min", Box::new(|_| INT_MIN)),
        (
            "alt_min_max",
            Box::new(|i| if i % 2 == 0 { INT_MIN } else { INT_MAX }),
        ),
        ("two", Box::new(|_| 2)),
        ("pow2", Box::new(|i| 1 << (i % 31))),
        ("neg_pow2", Box::new(|i| -(1 << (i % 31)))),
    ];
    for (name, f) in &patterns {
        for &len in &[0, 1, 2, 3, 8, 64, 256] {
            let n = len.max(0) as usize;
            let total = 4 * n + 4;
            let scratch: Vec<c_int> = (0..total).map(|i| f(i)).collect();
            diff_fma_layout(
                &format!("row4 {name} len={len}"),
                &scratch,
                (0, n + 1, 2 * n + 2, 3 * n + 3),
                len,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 5-9 — the single-pointer aliasing patterns
// ---------------------------------------------------------------------------
fn alias_row(label: &str, seed: u64, offs_of: impl Fn(usize) -> (usize, usize, usize, usize)) {
    let mut rng = Rng::new(seed);
    for &len in &[0, 1, 2, 3, 8, 64, 1024] {
        for rep in 0..REPS.min(if len > 100 { 6 } else { REPS }) {
            let n = len.max(0) as usize;
            let mut scratch = vec![0 as c_int; 4 * n + 8];
            rng.fill_full(&mut scratch);
            diff_fma_layout(
                &format!("{label} len={len} rep={rep}"),
                &scratch,
                offs_of(n),
                len,
            );
        }
    }
}

#[test]
fn cfg_05_fma_alias_out_mul1() {
    // out == mul1, mul2 and add distinct
    alias_row("row5", 5, |n| (0, 0, n + 1, 2 * n + 2));
}

#[test]
fn cfg_06_fma_alias_out_mul2() {
    alias_row("row6", 6, |n| (0, n + 1, 0, 2 * n + 2));
}

#[test]
fn cfg_07_fma_alias_out_add() {
    alias_row("row7", 7, |n| (0, n + 1, 2 * n + 2, 0));
}

#[test]
fn cfg_08_fma_alias_mul1_mul2() {
    // squaring: mul1 == mul2, out and add distinct
    alias_row("row8", 8, |n| (0, n + 1, n + 1, 2 * n + 2));
}

#[test]
fn cfg_09_fma_alias_all_sources() {
    // mul1 == mul2 == add, out distinct: out[i] = x*x + x into a separate buffer
    alias_row("row9", 9, |n| (0, n + 1, n + 1, n + 1));
}

// ---------------------------------------------------------------------------
// Row 10 — the exact 4-way aliasing `inner` uses
// ---------------------------------------------------------------------------
#[test]
fn cfg_10_fma_alias_four_way() {
    alias_row("row10", 10, |_n| (0, 0, 0, 0));
    // and with the small-magnitude value shape too
    let mut rng = Rng::new(1010);
    for &len in &[0, 1, 2, 3, 8, 64, 1024] {
        for rep in 0..8 {
            let n = len.max(0) as usize;
            let mut scratch = vec![0 as c_int; n + 4];
            rng.fill_small(&mut scratch);
            diff_fma_layout(
                &format!("row10-small len={len} rep={rep}"),
                &scratch,
                (0, 0, 0, 0),
                len,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 11-13 — overlapping (non-zero offset) destinations
// ---------------------------------------------------------------------------
#[test]
fn cfg_11_fma_overlap_plus_one() {
    // out = base, sources = base + 1  (ascending loop order is observable)
    let mut rng = Rng::new(11);
    for &len in &[1, 2, 3, 4, 8, 17, 64, 256] {
        for rep in 0..REPS {
            let n = len as usize;
            let mut scratch = vec![0 as c_int; n + 8];
            rng.fill_full(&mut scratch);
            diff_fma_layout(
                &format!("row11 len={len} rep={rep}"),
                &scratch,
                (0, 1, 1, 1),
                len,
            );
        }
    }
}

#[test]
fn cfg_12_fma_overlap_minus_one() {
    // out = base + 1, sources = base
    let mut rng = Rng::new(12);
    for &len in &[1, 2, 3, 4, 8, 17, 64, 256] {
        for rep in 0..REPS {
            let n = len as usize;
            let mut scratch = vec![0 as c_int; n + 8];
            rng.fill_full(&mut scratch);
            diff_fma_layout(
                &format!("row12 len={len} rep={rep}"),
                &scratch,
                (1, 0, 0, 0),
                len,
            );
        }
    }
}

#[test]
fn cfg_13_fma_overlap_staggered() {
    // out = base, mul1 = base+1, mul2 = base+2, add = base+3
    let mut rng = Rng::new(13);
    for &len in &[1, 2, 3, 4, 5, 8, 17, 64, 256] {
        for rep in 0..REPS {
            let n = len as usize;
            let mut scratch = vec![0 as c_int; n + 8];
            rng.fill_full(&mut scratch);
            diff_fma_layout(
                &format!("row13 len={len} rep={rep}"),
                &scratch,
                (0, 1, 2, 3),
                len,
            );
            // also the mirrored, descending stagger
            diff_fma_layout(
                &format!("row13-mirror len={len} rep={rep}"),
                &scratch,
                (3, 2, 1, 0),
                len,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14 — exact write extent (nothing outside [0, len) is touched)
// ---------------------------------------------------------------------------
#[test]
fn cfg_14_fma_write_extent_exact() {
    const MARGIN: usize = 8;
    let mut rng = Rng::new(14);
    for &len in &[1, 7, 64] {
        for rep in 0..REPS {
            let n = len as usize;
            // layout: [margin][out n][margin][mul1 n][mul2 n][add n][margin]
            let total = MARGIN + n + MARGIN + 3 * n + MARGIN;
            let mut scratch = vec![0 as c_int; total];
            rng.fill_full(&mut scratch);
            let poison: Vec<c_int> = (0..MARGIN).map(|i| 0x0BAD_0000 | i as c_int).collect();
            scratch[..MARGIN].copy_from_slice(&poison);
            let o = MARGIN;
            scratch[o + n..o + n + MARGIN].copy_from_slice(&poison);
            let m1 = MARGIN + n + MARGIN;
            diff_fma_layout(
                &format!("row14 len={len} rep={rep}"),
                &scratch,
                (o, m1, m1 + n, m1 + 2 * n),
                len,
            );
            // Independently confirm both libraries left the margins alone.
            for lib in [c_lib(), rust_lib()] {
                let mut buf = scratch.clone();
                let base = buf.as_mut_ptr();
                unsafe {
                    (lib.fma_array)(
                        base.add(o),
                        base.add(m1),
                        base.add(m1 + n),
                        base.add(m1 + 2 * n),
                        len,
                    );
                }
                assert_eq!(&buf[..MARGIN], &poison[..], "{} clobbered left margin", lib.name);
                assert_eq!(
                    &buf[o + n..o + n + MARGIN],
                    &poison[..],
                    "{} clobbered right margin",
                    lib.name
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15 — driver, all len values, random full-range
// ---------------------------------------------------------------------------
#[test]
fn cfg_15_driver_all_lens_random() {
    let mut rng = Rng::new(15);
    let mut lens: Vec<c_int> = LENS.to_vec();
    lens.push(1000);
    for &len in &lens {
        for rep in 0..REPS {
            let mut data = vec![0 as c_int; len.max(0) as usize + 1];
            rng.fill_full(&mut data);
            diff_driver(&format!("row15 len={len} rep={rep}"), &data, len);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16 — driver, large len
// ---------------------------------------------------------------------------
#[test]
fn cfg_16_driver_large() {
    let mut rng = Rng::new(16);
    let len: c_int = 65_536;
    for rep in 0..3 {
        let mut data = vec![0 as c_int; len as usize];
        rng.fill_full(&mut data);
        diff_driver(&format!("row16 rep={rep}"), &data, len);
    }
}

// ---------------------------------------------------------------------------
// Row 17 — driver, small-magnitude values (no overflow)
// ---------------------------------------------------------------------------
#[test]
fn cfg_17_driver_no_overflow() {
    let mut rng = Rng::new(17);
    for &len in &[1, 2, 8, 64] {
        for rep in 0..REPS {
            let mut data = vec![0 as c_int; len as usize];
            rng.fill_small(&mut data);
            diff_driver(&format!("row17 len={len} rep={rep}"), &data, len);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 18 — driver, constant value patterns
// ---------------------------------------------------------------------------
#[test]
fn cfg_18_driver_constant_patterns() {
    let patterns: Vec<(&str, Box<dyn Fn(usize) -> c_int>)> = vec![
        ("zeros", Box::new(|_| 0)),
        ("ones", Box::new(|_| 1)),
        ("neg_ones", Box::new(|_| -1)),
        ("twos", Box::new(|_| 2)),
        ("int_max", Box::new(|_| INT_MAX)),
        ("int_min", Box::new(|_| INT_MIN)),
        (
            "alt_min_max",
            Box::new(|i| if i % 2 == 0 { INT_MIN } else { INT_MAX }),
        ),
        ("pow2", Box::new(|i| 1 << (i % 31))),
        ("neg_pow2", Box::new(|i| -(1 << (i % 31)))),
        ("ramp", Box::new(|i| i as c_int)),
        ("neg_ramp", Box::new(|i| -(i as c_int))),
    ];
    for (name, f) in &patterns {
        for &len in &[0, 1, 2, 3, 8, 33, 64, 256] {
            let data: Vec<c_int> = (0..len.max(1) as usize).map(|i| f(i)).collect();
            diff_driver(&format!("row18 {name} len={len}"), &data, len);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — printf digit-length spread (1..11 characters per line)
// ---------------------------------------------------------------------------
#[test]
fn cfg_19_driver_digit_length_spread() {
    // Values chosen so that x*x + x has a wide spread of decimal lengths,
    // including the widest possible output "-2147483648".
    let data: Vec<c_int> = vec![
        0, 1, -1, 2, -2, 3, 9, 10, -10, 99, 100, -100, 999, 1000, -1000, 9999, 10_000, 46_340,
        46_341, -46_340, -46_341, 65_535, 65_536, 100_000, 1_000_000, 46_341, INT_MAX, INT_MIN,
        INT_MAX - 1, INT_MIN + 1, -2_147_483_647, 2_147_483_646,
    ];
    let len = data.len() as c_int;
    diff_driver("row19 all", &data, len);
    // Also every prefix length, so line counts vary too.
    for l in 0..=len {
        diff_driver(&format!("row19 prefix={l}"), &data, l);
    }
    // And single-element calls, which isolate each formatted value.
    for (i, &v) in data.iter().enumerate() {
        diff_driver(&format!("row19 single[{i}]={v}"), &[v], 1);
    }
}

// ---------------------------------------------------------------------------
// Row 20 — overflow-boundary values for x*x + x
// ---------------------------------------------------------------------------
#[test]
fn cfg_20_driver_overflow_boundary_values() {
    let mut vals: Vec<c_int> = vec![
        46_340, 46_341, -46_340, -46_341, 46_342, -46_342, 65_535, 65_536, 0x8000, 0xFFFF,
        0x1_0000, 0x1_0001, 0x7FFF_FFFF, -0x8000_0000, 0x4000_0000, -0x4000_0000, 0x2000_0000,
        0xB504_F333u32 as c_int, 0x5A82_799Au32 as c_int, 1 << 16, (1 << 16) - 1, (1 << 16) + 1,
    ];
    // every power of two and its neighbours (products straddle 0x7FFFFFFF)
    for k in 0..31 {
        vals.push(1 << k);
        vals.push((1 << k) - 1);
        vals.push(-(1 << k));
    }
    let len = vals.len() as c_int;
    diff_driver("row20 all", &vals, len);
    for (i, &v) in vals.iter().enumerate() {
        diff_driver(&format!("row20 single[{i}]={v}"), &[v], 1);
    }
}

// ---------------------------------------------------------------------------
// Row 21 — driver with a byte-misaligned `data` pointer
// ---------------------------------------------------------------------------
#[test]
fn cfg_21_driver_misaligned_source() {
    let mut rng = Rng::new(21);
    for &len in &[1, 3, 17] {
        for shift in 1..4usize {
            for rep in 0..REPS {
                let nbytes = len as usize * 4 + 8;
                let mut raw = vec![0u8; nbytes];
                for b in raw.iter_mut() {
                    *b = (rng.next_u64() & 0xFF) as u8;
                }
                let p = unsafe { raw.as_ptr().add(shift) } as *const c_int;
                let label = format!("row21 len={len} shift={shift} rep={rep}");
                let c = capture_stdout(|| unsafe { (c_lib().driver)(p, len) });
                let r = capture_stdout(|| unsafe { (rust_lib().driver)(p, len) });
                assert!(
                    c == r,
                    "[{label}] misaligned driver stdout mismatch: {}",
                    describe_diff(&c, &r)
                );
                // Model check: read the unaligned ints exactly as memcpy would.
                let mut model = Vec::with_capacity(len as usize);
                for i in 0..len as usize {
                    let mut b = [0u8; 4];
                    b.copy_from_slice(&raw[shift + 4 * i..shift + 4 * i + 4]);
                    model.push(c_int::from_ne_bytes(b));
                }
                let expect = model_driver_stdout(&model);
                assert!(
                    c == expect,
                    "[{label}] disagrees with reference model: {}",
                    describe_diff(&c, &expect)
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 22 — driver over an exactly-sized heap source (no slack)
// ---------------------------------------------------------------------------
#[test]
fn cfg_22_driver_exact_sized_source() {
    let mut rng = Rng::new(22);
    for &len in &[1, 2, 3, 4, 5, 8, 16, 17, 64, 1000] {
        for rep in 0..REPS {
            let n = len as usize;
            // Guarded buffer: the element after the last one is a guard page, so
            // any read past the end would fault. A valid `len` must not fault.
            let mut g = GuardedInts::new(n);
            rng.fill_full(g.as_mut_slice());
            let data: Vec<c_int> = g.as_slice().to_vec();
            let label = format!("row22 len={len} rep={rep}");
            let c = capture_stdout(|| unsafe { (c_lib().driver)(g.ptr(), len) });
            let r = capture_stdout(|| unsafe { (rust_lib().driver)(g.ptr(), len) });
            assert!(
                c == r,
                "[{label}] stdout mismatch: {}",
                describe_diff(&c, &r)
            );
            let expect = model_driver_stdout(&data);
            assert!(
                c == expect,
                "[{label}] disagrees with reference model: {}",
                describe_diff(&c, &expect)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 23 — memcpy size-class sweep
// ---------------------------------------------------------------------------
#[test]
fn cfg_23_driver_memcpy_size_classes() {
    let mut rng = Rng::new(23);
    let byte_sizes: &[usize] = &[
        4, 8, 12, 16, 20, 24, 28, 32, 33, 34, 35, 36, 48, 64, 65, 96, 127, 128, 129, 192, 255, 256,
        257, 384, 512, 513, 640, 1024, 1025, 2048, 4096, 4097, 8192,
    ];
    for &nb in byte_sizes {
        let len = (nb / 4) as c_int;
        if len == 0 {
            continue;
        }
        for rep in 0..4 {
            let mut data = vec![0 as c_int; len as usize];
            rng.fill_full(&mut data);
            diff_driver(&format!("row23 bytes={nb} len={len} rep={rep}"), &data, len);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 24 — composed-pipeline equivalence: driver == fma_array(4-way) + printf
// ---------------------------------------------------------------------------
#[test]
fn cfg_24_pipeline_equivalence() {
    let mut rng = Rng::new(24);
    for &len in &[0, 1, 2, 3, 5, 8, 64, 257, 1024] {
        for rep in 0..8 {
            let n = len.max(0) as usize;
            let mut data = vec![0 as c_int; n + 1];
            rng.fill_full(&mut data);

            let mut printed: Vec<Vec<u8>> = Vec::new();
            let mut kernel: Vec<Vec<c_int>> = Vec::new();
            for lib in [c_lib(), rust_lib()] {
                // (a) the one-shot pipeline
                printed.push(capture_stdout(|| unsafe {
                    (lib.driver)(data.as_ptr(), len)
                }));
                // (b) the low-level kernel driven directly, 4-way aliased,
                //     exactly as the static `inner` does it
                let mut buf = data.clone();
                let p = buf.as_mut_ptr();
                unsafe { (lib.fma_array)(p, p, p, p, len) };
                kernel.push(buf);
            }
            let label = format!("row24 len={len} rep={rep}");
            assert!(
                printed[0] == printed[1],
                "[{label}] driver stdout mismatch: {}",
                describe_diff(&printed[0], &printed[1])
            );
            assert_eq!(kernel[0], kernel[1], "[{label}] fma_array buffer mismatch");
            // The pipeline's stdout must be exactly the kernel result formatted.
            for (i, k) in kernel.iter().enumerate() {
                let mut s = String::new();
                for &v in &k[..n] {
                    s.push_str(&v.to_string());
                    s.push('\n');
                }
                assert!(
                    printed[i] == s.as_bytes(),
                    "[{label}] lib#{i}: driver stdout != printf(fma_array(...)): {}",
                    describe_diff(&printed[i], s.as_bytes())
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25 — long interleaved call sequence on one library handle (state leaks)
// ---------------------------------------------------------------------------
#[test]
fn cfg_25_interleaved_call_sequence() {
    let mut rng = Rng::new(25);
    // Build one deterministic script of 200 calls, then replay it against each
    // library and compare the accumulated stdout stream.
    enum Op {
        Driver(Vec<c_int>, c_int),
        Fma(Vec<c_int>, (usize, usize, usize, usize), c_int),
    }
    let mut ops = Vec::new();
    for _ in 0..200 {
        let len = rng.below(40) as c_int;
        let n = len as usize;
        if rng.next_u64() & 1 == 0 {
            let mut d = vec![0 as c_int; n + 1];
            rng.fill_full(&mut d);
            ops.push(Op::Driver(d, len));
        } else {
            let mut s = vec![0 as c_int; 4 * n + 8];
            rng.fill_full(&mut s);
            let offs = match rng.below(4) {
                0 => (0, 0, 0, 0),
                1 => (0, n + 1, 2 * n + 2, 3 * n + 3),
                2 => (0, 1, 1, 1),
                _ => (1, 0, 0, 0),
            };
            ops.push(Op::Fma(s, offs, len));
        }
    }

    let replay = |lib: &Lib| -> (Vec<u8>, Vec<Vec<c_int>>) {
        let mut states = Vec::new();
        let out = capture_stdout(|| {
            for op in &ops {
                match op {
                    Op::Driver(d, len) => unsafe { (lib.driver)(d.as_ptr(), *len) },
                    Op::Fma(s, offs, len) => {
                        let mut buf = s.clone();
                        let b = buf.as_mut_ptr();
                        unsafe {
                            (lib.fma_array)(
                                b.add(offs.0),
                                b.add(offs.1),
                                b.add(offs.2),
                                b.add(offs.3),
                                *len,
                            )
                        };
                        states.push(buf);
                    }
                }
            }
        });
        (out, states)
    };
    let (co, cs) = replay(c_lib());
    let (ro, rs) = replay(rust_lib());
    assert!(
        co == ro,
        "row25 accumulated stdout mismatch: {}",
        describe_diff(&co, &ro)
    );
    assert_eq!(cs, rs, "row25 fma_array buffer states mismatch");
}
