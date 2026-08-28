//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! Ordered lowest-level first. `get_bits` is `static` in the C translation unit
//! so it is exercised indirectly through the narrowest possible
//! `dequantize_granule` configurations (one active band, `group_size == 1`),
//! then coverage widens to the full function.

mod common;

use common::*;
use std::ffi::c_int;

/// Tiny deterministic PRNG so both sides always see the same inputs.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// Builds a case with exactly one active band (`bitalloc[0]`), everything else
/// zero. This isolates a single `get_bits(bs, ba)` call per outer iteration.
fn single_band(label: String, ba: u8, group_size: c_int, start_pos: c_int) -> Case {
    let mut case = Case::new(label, group_size);
    case.sci.set_total_bands(1); // i in 0..2
    case.sci.set_bitalloc(0, ba);
    case.sci.set_bitalloc(1, 0);
    case.start_pos = start_pos;
    case
}

// ---------------------------------------------------------------------------
// Level 1: get_bits, reached through a single linear-quantised band.
// ---------------------------------------------------------------------------

#[test]
fn t01_get_bits_all_widths_all_bit_offsets() {
    let impls = load();
    for ba in 1u8..=16 {
        for start_pos in 0..=17 {
            compare(
                &impls,
                &single_band(format!("get_bits ba={ba} pos={start_pos}"), ba, 1, start_pos),
            );
        }
    }
}

#[test]
fn t02_get_bits_byte_aligned_and_odd_offsets_larger_groups() {
    let impls = load();
    for ba in 1u8..=16 {
        for &gs in &[1, 2, 3, 4, 7, 8, 12, 18, 32] {
            for &start_pos in &[0, 1, 3, 5, 7, 8, 9, 15, 31, 63, 1000, 1001] {
                compare(
                    &impls,
                    &single_band(format!("gb ba={ba} gs={gs} pos={start_pos}"), ba, gs, start_pos),
                );
            }
        }
    }
}

#[test]
fn t03_get_bits_at_and_past_limit() {
    let impls = load();
    // Drive `pos` right up to, onto, and past `limit` so the early-return path
    // (which still advances `pos`) is covered.
    for ba in 1u8..=16 {
        for &gs in &[1, 3, 8] {
            for delta in -3i32..=3 {
                let mut case =
                    single_band(format!("limit ba={ba} gs={gs} d={delta}"), ba, gs, 0);
                // Total bits the function will try to consume: 4 outer * gs reads.
                let want = ba as i32 * gs * 4;
                case.limit = want + delta;
                compare(&impls, &case);
            }
        }
    }
}

#[test]
fn t04_limit_zero_and_negative() {
    let impls = load();
    for &limit in &[-8, -1, 0, 1, 7, 8] {
        for ba in [1u8, 4, 9, 16] {
            let mut case = single_band(format!("lim={limit} ba={ba}"), ba, 4, 0);
            case.limit = limit;
            compare(&impls, &case);
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: the grouped ("escape") branch, ba >= 17.
// ---------------------------------------------------------------------------

#[test]
fn t05_grouped_branch_small_ba() {
    let impls = load();
    // ba = 17..=21 keeps `2 << (ba - 17)` and `mod + 2 - (mod >> 3)` inside the
    // range where the C shifts are fully defined.
    for ba in 17u8..=21 {
        for &gs in &[1, 2, 3, 4, 6, 12, 18, 36] {
            for &start_pos in &[0, 1, 3, 5, 7, 11, 64] {
                compare(
                    &impls,
                    &single_band(
                        format!("grouped ba={ba} gs={gs} pos={start_pos}"),
                        ba,
                        gs,
                        start_pos,
                    ),
                );
            }
        }
    }
}

#[test]
fn t06_grouped_branch_limit_pressure() {
    let impls = load();
    for ba in 17u8..=21 {
        for delta in -2i32..=2 {
            let mut case = single_band(format!("grouped lim ba={ba} d={delta}"), ba, 3, 0);
            let m: u32 = 2u32 << (ba - 17);
            let m = m + 1;
            let nbits = (m + 2 - (m >> 3)) as i32;
            case.limit = nbits * 4 + delta;
            compare(&impls, &case);
        }
    }
}

// ---------------------------------------------------------------------------
// Level 3: multiple bands -- exercises the `choff` toggle and its carry-over
// across the outer `j` loop.
// ---------------------------------------------------------------------------

fn multi_band(label: String, total_bands: u8, bitalloc: &[u8], group_size: c_int) -> Case {
    let mut case = Case::new(label, group_size);
    case.sci.set_total_bands(total_bands);
    for (i, &v) in bitalloc.iter().enumerate() {
        case.sci.set_bitalloc(i, v);
    }
    case
}

#[test]
fn t07_choff_toggle_odd_and_even_band_counts() {
    let impls = load();
    let mut rng = Rng::new(0x0BAD_C0DE_1234_5678);
    // total_bands 0..=32 keeps every `bitalloc[i]` inside the declared array.
    for total_bands in 0u8..=32 {
        let n = 2 * total_bands as usize;
        let mut ba = vec![0u8; n.max(1)];
        for v in ba.iter_mut() {
            // Mostly small widths, occasionally zero, occasionally grouped.
            *v = match rng.below(10) {
                0 => 0,
                1 => 17 + rng.below(3) as u8,
                _ => 1 + rng.below(16) as u8,
            };
        }
        for &gs in &[1, 3, 12, 18] {
            compare(
                &impls,
                &multi_band(format!("choff tb={total_bands} gs={gs}"), total_bands, &ba, gs),
            );
        }
    }
}

#[test]
fn t08_realistic_layer2_shapes() {
    let impls = load();
    // Layer-II style: total_bands in {8, 12, 27, 30}, 4-bit allocations.
    for &total_bands in &[8u8, 12, 27, 30, 32] {
        for &gs in &[3u32, 6, 12] {
            let mut ba = vec![0u8; 2 * total_bands as usize];
            let mut rng = Rng::new(total_bands as u64 * 7919 + gs as u64);
            for v in ba.iter_mut() {
                *v = rng.below(16) as u8; // 0..=15
            }
            compare(
                &impls,
                &multi_band(
                    format!("layer2 tb={total_bands} gs={gs}"),
                    total_bands,
                    &ba,
                    gs as c_int,
                ),
            );
        }
    }
}

#[test]
fn t09_total_bands_past_bitalloc_array() {
    let impls = load();
    // total_bands > 32 makes the C read `bitalloc[i]` for i >= 64, i.e. into
    // `scfcod` and beyond. The harness backs the struct with a large,
    // deterministically filled allocation so both sides read the same bytes.
    for &total_bands in &[33u8, 40, 48, 64, 65] {
        let n = 2 * total_bands as usize;
        let mut ba = vec![0u8; n];
        let mut rng = Rng::new(0xFEED_FACE_0000 + total_bands as u64);
        for v in ba.iter_mut() {
            *v = match rng.below(8) {
                0 => 0,
                1 => 17 + rng.below(4) as u8,
                _ => 1 + rng.below(16) as u8,
            };
        }
        for &gs in &[1, 4, 18] {
            compare(
                &impls,
                &multi_band(
                    format!("oob-bitalloc tb={total_bands} gs={gs}"),
                    total_bands,
                    &ba,
                    gs,
                ),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Level 4: degenerate group sizes and stereo_bands (unused by the C code).
// ---------------------------------------------------------------------------

#[test]
fn t10_group_size_zero_and_negative() {
    let impls = load();
    for &gs in &[0i32, -1, -3, -18] {
        for ba in [0u8, 1, 8, 16, 17, 20] {
            compare(
                &impls,
                &single_band(format!("gs={gs} ba={ba}"), ba, gs, 0),
            );
        }
    }
}

#[test]
fn t11_all_bitalloc_zero_and_total_bands_zero() {
    let impls = load();
    for &gs in &[0i32, 1, 18, 64] {
        let mut case = Case::new(format!("zeros gs={gs}"), gs);
        case.sci.set_total_bands(0);
        compare(&impls, &case);

        let mut case = Case::new(format!("tb32-allzero gs={gs}"), gs);
        case.sci.set_total_bands(32);
        compare(&impls, &case);
    }
}

#[test]
fn t12_stereo_bands_is_ignored() {
    let impls = load();
    for &sb in &[0u8, 1, 17, 255] {
        let mut case = single_band(format!("stereo_bands={sb}"), 9, 6, 0);
        case.sci.set_stereo_bands(sb);
        compare(&impls, &case);
    }
}

// ---------------------------------------------------------------------------
// Level 5: randomized differential fuzzing over the whole surface.
// ---------------------------------------------------------------------------

#[test]
fn t13_fuzz_defined_domain() {
    let impls = load();
    let mut rng = Rng::new(0xC0FFEE_1234_5678);
    for iter in 0..400 {
        let total_bands = rng.below(66) as u8;
        let gs = match rng.below(8) {
            0 => 0,
            1 => 1,
            2 => 18,
            3 => 36,
            _ => rng.below(40) as i32,
        };
        let n = (2 * total_bands as usize).max(1);
        let mut ba = vec![0u8; n];
        for v in ba.iter_mut() {
            *v = match rng.below(12) {
                0 | 1 => 0,
                2 => 17 + rng.below(5) as u8,
                _ => 1 + rng.below(16) as u8,
            };
        }
        let mut case = multi_band(format!("fuzz#{iter}"), total_bands, &ba, gs);
        case.start_pos = rng.below(4096) as c_int;
        case.bs_seed = rng.next_u64();
        // Sometimes clamp the limit hard to exercise the truncating path.
        if rng.below(3) == 0 {
            case.limit = case.start_pos + rng.below(2048) as c_int;
        }
        compare(&impls, &case);
    }
}

// ---------------------------------------------------------------------------
// Level 6: `ba` values outside the well-defined domain.
//
// For ba >= 22 the C computes `2 << (ba - 17)` on an `int`, which overflows.
// On this target that shift is performed with the count masked to 5 bits and
// the result truncated, which the Rust `2u32.wrapping_shl(..)` reproduces, so
// the two implementations still agree bit-for-bit.
//
// The exception is `ba >= 46` with `ba % 32` in {14, 15}: there the derived
// bit count is large enough that `bs->pos += n` overflows `int` and goes
// negative, which slips past the `pos > limit` guard and makes the C
// dereference a wildly out-of-range pointer. Both the C and the Rust build
// fault identically on those inputs (verified out-of-process), so there is no
// observable behaviour to compare and they are excluded here.
// ---------------------------------------------------------------------------

fn c_side_faults(ba: u8) -> bool {
    ba >= 46 && matches!(ba % 32, 14 | 15)
}

#[test]
fn t15_high_ba_overflowing_shifts() {
    let impls = load();
    for ba in 22u8..=45 {
        for &gs in &[1, 3, 18] {
            compare(
                &impls,
                &single_band(format!("high ba={ba} gs={gs}"), ba, gs, 0),
            );
        }
    }
}

#[test]
fn t16_every_ba_byte_value() {
    let impls = load();
    // Exhaustive over the whole `uint8_t` range at the configuration that was
    // verified not to fault on either side.
    for ba in 0u8..=255 {
        if c_side_faults(ba) {
            continue;
        }
        compare(&impls, &single_band(format!("exhaustive ba={ba}"), ba, 2, 0));
    }
}

// ---------------------------------------------------------------------------
// Exported-symbol parity between the two shared objects.
// ---------------------------------------------------------------------------

fn dynamic_defined_symbols(path: &std::path::Path) -> Vec<String> {
    let out = std::process::Command::new("nm")
        .arg("-D")
        .arg("--defined-only")
        .arg(path)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    let mut syms: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let a = it.next()?;
            let (kind, name) = match it.next() {
                Some(k) => (k, it.next()?),
                None => return None,
            };
            let _ = a;
            // Only real exported code/data, skipping compiler/runtime bookkeeping.
            if !matches!(kind, "T" | "t" | "D" | "d" | "B" | "b" | "R" | "r" | "W" | "V" | "i") {
                return None;
            }
            Some(name.to_string())
        })
        .collect();
    syms.sort();
    syms.dedup();
    syms
}

/// Symbols the C `.so` gets purely from libc/CRT glue; the Rust cdylib is not
/// expected to reproduce those.
fn is_toolchain_symbol(name: &str) -> bool {
    const EXACT: &[&str] = &[
        "_init",
        "_fini",
        "__bss_start",
        "_edata",
        "_end",
        "__gmon_start__",
        "_ITM_registerTMCloneTable",
        "_ITM_deregisterTMCloneTable",
        "__cxa_finalize",
        "__libc_csu_init",
        "__libc_csu_fini",
        "__gnu_lto_slim",
        "__odr_asan_gen_",
    ];
    EXACT.contains(&name)
        || name.starts_with("_ZN")
        || name.starts_with("__rust")
        || name.starts_with("rust_")
        || name.starts_with("_R")
        || name.starts_with("__gcc")
        || name.starts_with("__do_global")
        || name.starts_with("_DYNAMIC")
        || name.starts_with("_GLOBAL_OFFSET_TABLE_")
        || name.starts_with("__TMC_END__")
        || name.starts_with("__dso_handle")
        || name.starts_with("__gxx")
}

#[test]
fn t14_rust_so_exports_every_c_so_symbol() {
    let c_path = c_library_path();
    let rust_path = rust_library_path();
    let c_syms = dynamic_defined_symbols(&c_path);
    let rust_syms = dynamic_defined_symbols(&rust_path);

    // Sanity: the API entry point must be there on both sides.
    assert!(
        c_syms.iter().any(|s| s == "dequantize_granule"),
        "C .so does not export dequantize_granule; got {c_syms:?}"
    );
    assert!(
        rust_syms.iter().any(|s| s == "dequantize_granule"),
        "Rust .so does not export dequantize_granule; got {rust_syms:?}"
    );

    let missing: Vec<&String> = c_syms
        .iter()
        .filter(|s| !is_toolchain_symbol(s))
        .filter(|s| !rust_syms.contains(s))
        .collect();
    assert!(
        missing.is_empty(),
        "Rust .so ({}) is missing symbols exported by the C .so ({}): {:?}",
        rust_path.display(),
        c_path.display(),
        missing
    );
}
