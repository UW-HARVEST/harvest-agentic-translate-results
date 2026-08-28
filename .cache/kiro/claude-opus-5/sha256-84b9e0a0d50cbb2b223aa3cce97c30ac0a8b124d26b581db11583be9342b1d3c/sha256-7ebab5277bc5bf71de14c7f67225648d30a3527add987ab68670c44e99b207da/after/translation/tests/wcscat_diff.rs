//! Differential tests for the exported `wcscat` symbol.
//!
//! Every call goes through `dlopen`/`dlsym` on both the C `.so` and the Rust
//! `cdylib`; nothing is invoked directly from the crate.

mod common;

use common::{Pair, Rng, WcharT, load_pair};

/// Sentinel filler used past `num_elem` to catch out-of-bounds writes.
const CANARY: WcharT = 0x7EAD_BEEF;

/// Builds a `dst` buffer: `content`, then a terminator-free canary tail.
fn dst_with_tail(content: &[WcharT], tail: usize) -> Vec<WcharT> {
    let mut v = content.to_vec();
    v.extend(std::iter::repeat_n(CANARY, tail));
    v
}

/// Null-terminated `src` from a plain slice.
fn src_z(content: &[WcharT]) -> Vec<WcharT> {
    let mut v = content.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Level 0: the early-return / argument-validation paths.
// ---------------------------------------------------------------------------

#[test]
fn null_dst_returns_einval() {
    let p = load_pair();
    let src = src_z(&[b'a' as WcharT, b'b' as WcharT]);

    // dst == NULL, with a variety of capacities and src values.
    for &n in &[0usize, 1, 2, 16, usize::MAX] {
        p.check("null dst, valid src", &[], n, Some(&src), true);
        p.check("null dst, null src", &[], n, None, true);
    }
}

#[test]
fn zero_capacity_returns_einval_without_writing() {
    let p = load_pair();
    let src = src_z(&[b'x' as WcharT]);
    let dst = dst_with_tail(&[b'A' as WcharT, 0], 6);

    // numElem == 0 is checked *before* the null-src branch, so nothing is
    // written even when src is NULL.
    p.check("zero capacity, valid src", &dst, 0, Some(&src), false);
    p.check("zero capacity, null src", &dst, 0, None, false);
}

#[test]
fn null_src_clears_first_element_and_returns_einval() {
    let p = load_pair();

    // Non-empty dst: dst[0] is zeroed, the rest is untouched.
    let dst = dst_with_tail(&[b'A' as WcharT, b'B' as WcharT, 0], 5);
    for &n in &[1usize, 2, 3, 8] {
        p.check("null src", &dst, n, None, false);
    }

    // Already-empty dst.
    let empty = dst_with_tail(&[0], 7);
    p.check("null src, empty dst", &empty, 8, None, false);

    // A capacity larger than the buffer is fine here: the null-src branch
    // only ever touches dst[0].
    p.check("null src, oversized capacity", &dst, usize::MAX, None, false);
}

// ---------------------------------------------------------------------------
// Level 1: the seek loop that finds the existing terminator.
// ---------------------------------------------------------------------------

#[test]
fn appends_to_empty_destination() {
    let p = load_pair();
    let dst = dst_with_tail(&[0], 15);

    for text in [
        vec![],
        vec![b'a' as WcharT],
        vec![b'h' as WcharT, b'i' as WcharT],
        (0..10).map(|i| b'a' as WcharT + i).collect(),
    ] {
        let src = src_z(&text);
        p.check("append to empty", &dst, 16, Some(&src), false);
    }
}

#[test]
fn appends_after_existing_content() {
    let p = load_pair();

    for prefix_len in 0usize..8 {
        let prefix: Vec<WcharT> = (0..prefix_len).map(|i| b'P' as WcharT + i as WcharT).collect();
        let mut content = prefix.clone();
        content.push(0);
        let dst = dst_with_tail(&content, 24 - content.len());

        for suffix_len in 0usize..8 {
            let suffix: Vec<WcharT> =
                (0..suffix_len).map(|i| b'S' as WcharT + i as WcharT).collect();
            let src = src_z(&suffix);
            p.check("append after content", &dst, 24, Some(&src), false);
        }
    }
}

#[test]
fn seek_stops_at_capacity_when_no_terminator_present() {
    let p = load_pair();

    // No zero within num_elem: the seek loop runs to `dst + numElem`, the copy
    // loop never executes, dst[0] is cleared and ERANGE is returned.
    let dst = dst_with_tail(&[], 16);
    let src = src_z(&[b'z' as WcharT]);
    for &n in &[1usize, 2, 8, 16] {
        p.check("unterminated dst", &dst, n, Some(&src), false);
    }
}

#[test]
fn terminator_exactly_at_capacity_boundary() {
    let p = load_pair();

    // dst holds `AB\0` but the capacity is 2, so the terminator sits *at* the
    // bound and is never seen: seek exhausts the capacity -> ERANGE.
    let dst = dst_with_tail(&[b'A' as WcharT, b'B' as WcharT, 0], 5);
    let src = src_z(&[b'C' as WcharT]);
    p.check("terminator at bound", &dst, 2, Some(&src), false);

    // With capacity 3 the terminator is visible, but there is no room for the
    // new terminator within the capacity -> also ERANGE.
    p.check("terminator visible, no room", &dst, 3, Some(&src), false);

    // Capacity 4 is exactly enough for "ABC\0".
    p.check("exact fit", &dst, 4, Some(&src), false);
}

// ---------------------------------------------------------------------------
// Level 2: the copy loop, exact fits, and the ERANGE truncation path.
// ---------------------------------------------------------------------------

#[test]
fn exact_fit_boundary_sweep() {
    let p = load_pair();

    // "AB\0" + "CD" needs 5 elements. Sweep capacities either side of that.
    let dst = dst_with_tail(&[b'A' as WcharT, b'B' as WcharT, 0], 13);
    let src = src_z(&[b'C' as WcharT, b'D' as WcharT]);

    for n in 1usize..=10 {
        p.check("exact fit sweep", &dst, n, Some(&src), false);
    }
}

#[test]
fn overflow_clears_first_element() {
    let p = load_pair();

    // src is far too long: the buffer fills, then dst[0] = 0 and ERANGE.
    let dst = dst_with_tail(&[b'A' as WcharT, 0], 14);
    let long: Vec<WcharT> = (0..32).map(|i| b'a' as WcharT + (i % 26)).collect();
    let src = src_z(&long);

    for &n in &[2usize, 3, 4, 8, 16] {
        p.check("overflow", &dst, n, Some(&src), false);
    }
}

#[test]
fn empty_src_writes_only_a_terminator() {
    let p = load_pair();
    let src = src_z(&[]);

    // Appending nothing still requires one slot for the terminator.
    let dst = dst_with_tail(&[b'A' as WcharT, b'B' as WcharT, 0], 5);
    for n in 1usize..=8 {
        p.check("empty src", &dst, n, Some(&src), false);
    }

    // Unterminated dst with an empty src: seek exhausts the capacity.
    let full = dst_with_tail(&[], 8);
    p.check("empty src, unterminated dst", &full, 4, Some(&src), false);
}

#[test]
fn unterminated_src_is_truncated_at_capacity() {
    let p = load_pair();

    // src has no terminator, but it is long enough that the copy loop always
    // fills dst before reading past the slice.
    let dst = dst_with_tail(&[b'A' as WcharT, 0], 14);
    let src: Vec<WcharT> = (0..64).map(|i| b'q' as WcharT + (i % 26)).collect();

    for &n in &[2usize, 5, 16] {
        p.check("unterminated src", &dst, n, Some(&src), false);
    }
}

#[test]
fn capacity_smaller_than_buffer_leaves_tail_untouched() {
    let p = load_pair();

    // The canary tail past `num_elem` must survive every path.
    let dst = dst_with_tail(&[b'A' as WcharT, 0], 30);
    let src = src_z(&[b'B' as WcharT, b'C' as WcharT]);

    for n in 1usize..=6 {
        p.check("tail untouched", &dst, n, Some(&src), false);
    }
}

// ---------------------------------------------------------------------------
// Level 3: wide-character value ranges.
// ---------------------------------------------------------------------------

#[test]
fn full_wchar_value_range() {
    let p = load_pair();

    // wchar_t is signed on Linux; make sure negative and extreme values, and
    // values whose low byte is zero, round-trip identically.
    let values: [WcharT; 10] = [
        1,
        -1,
        WcharT::MIN,
        WcharT::MAX,
        0x0000_0100,
        0x0001_0000,
        0x10FF_FF,
        0x7F,
        -0x8000,
        0x00FF_FF00,
    ];

    for &v in &values {
        let dst = dst_with_tail(&[v, 0], 14);
        let src = src_z(&[v, v.wrapping_neg(), v ^ 0x5A5A_5A5A]);
        p.check("wide values", &dst, 16, Some(&src), false);
        p.check("wide values, tight", &dst, 5, Some(&src), false);
        p.check("wide values, overflow", &dst, 4, Some(&src), false);
    }
}

#[test]
fn interior_zeros_in_destination_are_respected() {
    let p = load_pair();

    // The seek loop stops at the *first* zero, so trailing data past it is
    // overwritten by the copy.
    let dst = dst_with_tail(
        &[b'A' as WcharT, 0, b'X' as WcharT, b'Y' as WcharT, 0],
        11,
    );
    let src = src_z(&[b'1' as WcharT, b'2' as WcharT, b'3' as WcharT]);

    for n in 1usize..=8 {
        p.check("interior zeros", &dst, n, Some(&src), false);
    }
}

// ---------------------------------------------------------------------------
// Level 4: randomized differential fuzzing.
// ---------------------------------------------------------------------------

fn fuzz(p: &Pair, seed: u64, iterations: usize) {
    let mut rng = Rng::new(seed);

    for i in 0..iterations {
        let cap = 1 + rng.below(12);
        let tail = rng.below(6);

        // Random dst contents; may or may not contain a terminator.
        let mut dst: Vec<WcharT> = Vec::with_capacity(cap + tail);
        for _ in 0..cap {
            let r = rng.next_u64();
            dst.push(match r % 4 {
                0 => 0,
                1 => (r >> 8) as i32,
                2 => (b'a' as WcharT) + (r >> 8) as WcharT % 26,
                _ => -((r >> 8) as i32 & 0x7FFF),
            });
        }
        dst.extend(std::iter::repeat_n(CANARY, tail));

        // src is either terminated, or long enough to outlast the capacity.
        let terminated = rng.next_u64() % 2 == 0;
        let src_len = if terminated {
            rng.below(cap + 3)
        } else {
            cap + 2
        };
        let mut src: Vec<WcharT> = Vec::with_capacity(src_len + 1);
        for _ in 0..src_len {
            let r = rng.next_u64();
            src.push(match r % 3 {
                0 => (r >> 8) as i32,
                1 => (b'A' as WcharT) + (r >> 8) as WcharT % 26,
                _ => -((r >> 8) as i32 & 0xFFFF) - 1,
            });
        }
        if terminated {
            src.push(0);
        }

        let label = format!("fuzz seed={seed} iter={i}");
        p.check(&label, &dst, cap, Some(&src), false);

        // Occasionally also probe a capacity below the buffer length.
        if cap > 1 {
            let smaller = 1 + rng.below(cap);
            p.check(&label, &dst, smaller, Some(&src), false);
        }
    }
}

#[test]
fn randomized_differential() {
    let p = load_pair();
    for seed in [1u64, 0xDEAD_BEEF, 0x1234_5678_9ABC_DEF0, 42, 7] {
        fuzz(&p, seed, 400);
    }
}

// ---------------------------------------------------------------------------
// Level 5: exported-symbol parity between the two shared objects.
// ---------------------------------------------------------------------------

#[test]
fn rust_so_exports_every_c_symbol() {
    use std::process::Command;

    fn defined_dynamic_symbols(path: &std::path::Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only"])
            .arg(path)
            .output()
            .expect("failed to run nm");
        assert!(
            out.status.success(),
            "nm failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );

        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2).map(str::to_string))
            .collect()
    }

    let c_syms = defined_dynamic_symbols(&common::c_so_path());
    let rust_syms = defined_dynamic_symbols(&common::rust_so_path());

    assert!(
        c_syms.iter().any(|s| s == "wcscat"),
        "the C .so should export `wcscat`, got {c_syms:?}"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}\n  \
         C={c_syms:?}\n  Rust={rust_syms:?}"
    );
}
