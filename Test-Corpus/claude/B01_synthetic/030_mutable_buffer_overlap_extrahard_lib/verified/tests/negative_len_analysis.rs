// Analysis + gate for the one part of the C library whose outcome is NOT a
// property of the library: `driver(data, len)` with `len < 0`.
//
// What the C actually does (from `objdump -d` of the C `.so`, function `driver`):
//
//     push %rbp;  mov %rsp,%rbp;  push %rbx;  sub $0x28,%rsp     ; frame
//     mov %rsp,%rbx                                              ; save sp
//     movslq len,%rax;  lea 0(,%rax,4),%rdx                      ; (int64)len*4
//     mov $0x10,%rax; sub $1,%rax; add %rdx,%rax                 ; +15
//     mov $0x10,%rcx; mov $0,%edx; div %rcx; imul $0x10,%rax,%rax ; /16*16
//     sub %rax,%rsp                                              ; <== the VLA
//     mov %rsp,%rax; add $3,%rax; shr $2,%rax; shl $2,%rax        ; out = align4
//     ... call memcpy ; call inner ; mov %rbx,%rsp ; leave ; ret
//
// For `len < 0` the reserved size is the *wrapped* value `2^64 - round16(4|len|)`,
// so `sub %rax,%rsp` moves `%rsp` **upwards** by `K = round16(4|len|)` bytes --
// past `driver`'s own frame and into the caller's. Everything that happens next
// (the `call memcpy` return-address push at `out-8`, `memcpy`'s stores at `out`,
// the `call inner` push, and finally `ret` through the possibly-overwritten
// return address at `%rbp+8`) writes into memory that belongs to `driver`'s
// caller. Whether that is fatal, harmless, or turns into an endless loop is
// therefore a function of gcc's frame layout for `driver` **and of the caller's
// frame** -- not of anything the C source specifies.
//
// `d_neg_01` measures that directly; `d_neg_02`/`d_neg_03` are the properties
// that *are* well defined and that the Rust translation must (and does) match.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::ffi::c_int;

const NEG_LENS: &[c_int] = &[
    -1, -2, -4, -12, -16, -19, -20, -28, -31, -32, -64, -512, -1024, -65_536, INT_MIN,
];

/// Same call, four different amounts of caller stack in use.
#[inline(never)]
fn c_disp_pad0(p: *const c_int, len: c_int) -> Disposition {
    run_in_child(|| unsafe { (c_lib().driver)(p, len) })
}
#[inline(never)]
fn c_disp_pad1(p: *const c_int, len: c_int) -> Disposition {
    let pad = [0xA5u64; 8];
    let d = run_in_child(|| unsafe { (c_lib().driver)(p, len) });
    std::hint::black_box(&pad);
    d
}
#[inline(never)]
fn c_disp_pad2(p: *const c_int, len: c_int) -> Disposition {
    let pad = [0xA5u64; 40];
    let d = run_in_child(|| unsafe { (c_lib().driver)(p, len) });
    std::hint::black_box(&pad);
    d
}
#[inline(never)]
fn c_disp_pad3(p: *const c_int, len: c_int) -> Disposition {
    let pad = [0xA5u64; 200];
    let d = run_in_child(|| unsafe { (c_lib().driver)(p, len) });
    std::hint::black_box(&pad);
    d
}

/// The C library's termination for a negative `len` is decided by the *caller's*
/// stack frame, so it is not a behaviour any translation could reproduce. Proven,
/// not asserted: the same `(library, data, len)` is invoked from four call sites
/// that differ only in how much stack the caller has in use.
#[test]
fn d_neg_01_c_outcome_depends_on_the_callers_frame() {
    let data: Vec<c_int> = (0..4096).collect();
    let p = data.as_ptr();
    let mut unstable: Vec<(c_int, Vec<String>)> = Vec::new();
    for &len in NEG_LENS {
        let ds = [
            c_disp_pad0(p, len),
            c_disp_pad1(p, len),
            c_disp_pad2(p, len),
            c_disp_pad3(p, len),
        ];
        let set: BTreeSet<String> = ds.iter().map(|d| d.to_string()).collect();
        if set.len() > 1 {
            unstable.push((len, set.into_iter().collect()));
        }
    }
    println!(
        "negative lengths for which the *C library alone* terminates differently \
         depending only on the caller's frame ({} of {} tested):",
        unstable.len(),
        NEG_LENS.len()
    );
    for (len, outcomes) in &unstable {
        println!("  len={len:>12}  ->  {outcomes:?}");
    }
    assert!(
        !unstable.is_empty(),
        "expected the C library's negative-length outcome to be caller-frame \
         dependent (the VLA moves %rsp into the caller's frame); if this ever \
         becomes stable the ERRORS.md rationale for rows 14-16 must be revisited"
    );
}

/// The well-defined half, which IS differentially tested: for a negative `len`
/// the C's `inner` skips both of its loops, so the library must print nothing.
/// Verified for both implementations, for every tested negative length.
#[test]
fn d_neg_02_neither_library_prints_anything() {
    let data: Vec<c_int> = (0..4096).collect();
    let p = data.as_ptr();
    for &len in NEG_LENS {
        let (_, co) = run_in_child_capturing(|| unsafe { (c_lib().driver)(p, len) });
        let (_, ro) = run_in_child_capturing(|| unsafe { (rust_lib().driver)(p, len) });
        assert!(
            co.is_empty(),
            "C printed {:?} for len={len}, but `inner` skips both loops when len<0",
            hex_head(&co, 80)
        );
        assert!(
            ro.is_empty(),
            "Rust printed {:?} for len={len}",
            hex_head(&ro, 80)
        );
        assert_eq!(co, ro, "stdout mismatch for len={len}");
    }
}

/// The Rust translation's own behaviour for a negative `len` must at least be
/// *deterministic* and must never silently pretend the call succeeded: the C
/// source asks for a copy of `(size_t)(len * sizeof(int))` bytes, which cannot
/// possibly be satisfied, so the process must die. Checked from all four caller
/// frames, i.e. the Rust does not inherit the C's caller-frame sensitivity.
#[test]
fn d_neg_03_rust_is_deterministic_and_always_faults() {
    let data: Vec<c_int> = (0..4096).collect();
    let p = data.as_ptr();
    for &len in NEG_LENS {
        let mut seen = BTreeSet::new();
        for _ in 0..1 {
            seen.insert(rust_pad0(p, len).to_string());
            seen.insert(rust_pad1(p, len).to_string());
            seen.insert(rust_pad2(p, len).to_string());
            seen.insert(rust_pad3(p, len).to_string());
        }
        assert_eq!(
            seen.len(),
            1,
            "the Rust library must behave deterministically for len={len}, saw {seen:?}"
        );
        let only = seen.into_iter().next().unwrap();
        assert_eq!(
            only,
            Disposition::Signaled(libc::SIGSEGV).to_string(),
            "len={len}: the Rust library must fault rather than silently succeed"
        );
    }
}

#[inline(never)]
fn rust_pad0(p: *const c_int, len: c_int) -> Disposition {
    run_in_child(|| unsafe { (rust_lib().driver)(p, len) })
}
#[inline(never)]
fn rust_pad1(p: *const c_int, len: c_int) -> Disposition {
    let pad = [0xA5u64; 8];
    let d = run_in_child(|| unsafe { (rust_lib().driver)(p, len) });
    std::hint::black_box(&pad);
    d
}
#[inline(never)]
fn rust_pad2(p: *const c_int, len: c_int) -> Disposition {
    let pad = [0xA5u64; 40];
    let d = run_in_child(|| unsafe { (rust_lib().driver)(p, len) });
    std::hint::black_box(&pad);
    d
}
#[inline(never)]
fn rust_pad3(p: *const c_int, len: c_int) -> Disposition {
    let pad = [0xA5u64; 200];
    let d = run_in_child(|| unsafe { (rust_lib().driver)(p, len) });
    std::hint::black_box(&pad);
    d
}

/// `fma_array` -- the low-level entry point -- has no VLA, so its negative-length
/// behaviour *is* fully specified (the loop guard is simply false) and is
/// compared exactly, from all four caller frames.
#[test]
fn d_neg_04_fma_array_negative_len_is_exactly_reproducible() {
    let mut rng = Rng::new(777);
    for &len in NEG_LENS {
        for _ in 0..4 {
            let mut scratch = vec![0 as c_int; 64];
            rng.fill_full(&mut scratch);
            diff_fma_layout(&format!("fma neg len={len}"), &scratch, (0, 16, 32, 48), len);
            diff_fma_layout(&format!("fma neg 4way len={len}"), &scratch, (0, 0, 0, 0), len);
        }
    }
}
