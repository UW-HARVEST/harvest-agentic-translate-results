//! Level 2: `cp_inflate`.
//!
//! `cp_inflate` is the only exported entry point that reaches the bit reader
//! (`cp_read_bits` / `cp_peak_bits` / `cp_consume_bits` / `cp_ptr`), the Huffman
//! table builder (`cp_build`, `cp_rev16`), the decoder (`cp_decode`) and the
//! three block handlers (`cp_stored`, `cp_fixed`, `cp_dynamic`, `cp_block`) -
//! every other function in lib.c is `static` and therefore not observable
//! across the .so boundary.

mod common;

use common::{libs, InBuf, Pair};
use std::ffi::{c_int, c_void};

const FILL: u8 = 0xCD;

struct Outcome {
    rc: c_int,
    out: Vec<u8>,
    err: Option<Vec<u8>>,
}

fn one_side(
    lib: &common::Lib,
    inb: &mut InBuf,
    out_bytes: usize,
    cap: usize,
) -> Outcome {
    let mut out = vec![FILL; cap];
    lib.clear_error();
    let rc = unsafe {
        (lib.cp_inflate)(
            inb.ptr(),
            inb.len(),
            out.as_mut_ptr() as *mut c_void,
            out_bytes as c_int,
        )
    };
    Outcome {
        rc,
        out,
        err: lib.error_reason(),
    }
}

fn check(l: &Pair, tag: &str, data: &[u8], out_bytes: usize, cap: usize, align: usize) {
    // One shared input buffer => both libraries see the *same* pointer, so the
    // alignment-dependent `first_bytes` split inside cp_inflate is identical.
    let mut inb = InBuf::new(data, align);
    let c = one_side(&l.c, &mut inb, out_bytes, cap);
    let r = one_side(&l.rs, &mut inb, out_bytes, cap);

    assert_eq!(
        c.rc, r.rc,
        "{tag} (out_bytes={out_bytes} align={align}): return value C={} Rust={}",
        c.rc, r.rc
    );
    if c.out != r.out {
        let i = c
            .out
            .iter()
            .zip(r.out.iter())
            .position(|(a, b)| a != b)
            .unwrap();
        let lo = i.saturating_sub(8);
        let hi = (i + 8).min(c.out.len());
        panic!(
            "{tag} (out_bytes={out_bytes} align={align}): output differs at {i}\n  C   ={:02x?}\n  Rust={:02x?}",
            &c.out[lo..hi],
            &r.out[lo..hi]
        );
    }
    let ce = c.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned());
    let re = r.err.as_ref().map(|v| String::from_utf8_lossy(v).into_owned());
    assert_eq!(
        ce, re,
        "{tag} (out_bytes={out_bytes} align={align}): cp_error_reason differs"
    );
}

fn cap_for(raw_len: usize) -> usize {
    // A stored block memcpy's LEN (<= 65535) bytes without consulting out_end,
    // so keep at least that much slack past `out_bytes` inside the allocation.
    raw_len + 65600
}

#[test]
fn inflate_all_vectors_all_alignments() {
    let l = libs();
    for v in common::vectors() {
        let cap = cap_for(v.raw_len);
        for align in 0..4 {
            check(&l, &v.name, &v.data, cap, cap, align);
        }
    }
}

#[test]
fn inflate_exact_output_size() {
    let l = libs();
    for v in common::vectors() {
        let cap = cap_for(v.raw_len);
        check(&l, &v.name, &v.data, v.raw_len, cap, 0);
        check(&l, &v.name, &v.data, v.raw_len + 1, cap, 1);
    }
}

#[test]
fn inflate_truncated_output_buffer() {
    let l = infallible_libs();
    for v in common::vectors() {
        let cap = cap_for(v.raw_len);
        for out_bytes in [0usize, 1, 2, 3, v.raw_len / 4, v.raw_len / 2] {
            if out_bytes > v.raw_len {
                continue;
            }
            check(&l, &v.name, &v.data, out_bytes, cap, 2);
        }
        if v.raw_len > 0 {
            check(&l, &v.name, &v.data, v.raw_len - 1, cap, 3);
        }
    }
}

fn infallible_libs() -> std::sync::MutexGuard<'static, Pair> {
    libs()
}

/// The tables are non-`static` C globals, so a caller may patch them.  Verify
/// the Rust exports are read through the same storage rather than a private
/// copy.  Only `*_base` entries are perturbed: they are added *after* the extra
/// bits have been consumed, so the bit stream stays in sync and lowering them
/// can never push the writer past the bounds the C code already checks.
#[test]
fn patched_len_and_dist_base_tables_are_honoured() {
    let l = libs();
    let vectors: Vec<_> = common::vectors()
        .into_iter()
        .filter(|v| v.name.starts_with("crafted_all_len") || v.name.starts_with("crafted_all_dist"))
        .collect();
    assert!(!vectors.is_empty());

    unsafe {
        let saved_len: Vec<u32> = std::slice::from_raw_parts(l.c.cp_len_base, 31).to_vec();
        let saved_dist: Vec<u32> = std::slice::from_raw_parts(l.c.cp_dist_base, 31).to_vec();

        for (i, patch) in [(28usize, 100u32), (13, 5), (0, 3)] {
            *l.c.cp_len_base.add(i) = patch;
            *l.rs.cp_len_base.add(i) = patch;
        }
        // every distance becomes 1 -> forces the memset fast path
        for i in 0..30usize {
            *l.c.cp_dist_base.add(i) = 1;
            *l.rs.cp_dist_base.add(i) = 1;
        }

        for v in &vectors {
            let cap = cap_for(v.raw_len);
            check(&l, &format!("patched:{}", v.name), &v.data, cap, cap, 0);
        }

        for i in 0..31usize {
            *l.c.cp_len_base.add(i) = saved_len[i];
            *l.rs.cp_len_base.add(i) = saved_len[i];
            *l.c.cp_dist_base.add(i) = saved_dist[i];
            *l.rs.cp_dist_base.add(i) = saved_dist[i];
        }
    }
}

/// `cp_error_reason` must be left untouched by a successful call and must point
/// at byte-identical text after a failing one.
#[test]
fn error_reason_text_matches() {
    let l = libs();
    let mut seen = 0;
    for v in common::vectors() {
        let cap = cap_for(v.raw_len);
        let mut inb = InBuf::new(&v.data, 0);
        let c = one_side(&l.c, &mut inb, cap, cap);
        let r = one_side(&l.rs, &mut inb, cap, cap);
        assert_eq!(c.rc, r.rc, "{}", v.name);
        assert_eq!(c.err, r.err, "{}: error text", v.name);
        if c.rc == 0 {
            assert!(c.err.is_some(), "{}: failure without a reason", v.name);
            seen += 1;
        }
    }
    assert!(seen > 0, "no failing vectors exercised the error paths");
}
