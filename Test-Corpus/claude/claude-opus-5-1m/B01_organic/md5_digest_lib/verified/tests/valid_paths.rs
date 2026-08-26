//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md` (C1..C18, minus C13/C14 which need the
//! guard-page/fork harness and live in `tests/error_paths.rs`).
//!
//! Every test drives BOTH shared objects through their exported `md5_digest`
//! symbol and compares the 16 output bytes plus the surrounding memory
//! byte-for-byte.

mod common;

use common::*;

/// Word values the truncation/shift arithmetic can distinguish.
const BOUNDARY: [u32; 14] = [
    0x0000_0000,
    0x0000_0001,
    0x0000_007F,
    0x0000_0080,
    0x0000_00FF,
    0x0000_0100,
    0x0000_7FFF,
    0x0000_8000,
    0x0000_FFFF,
    0x0001_0000,
    0x7FFF_FFFF,
    0x8000_0000,
    0xFFFF_FFFE,
    0xFFFF_FFFF,
];

// ---------------------------------------------------------------------------
// C1 — all-zero state, aligned, disjoint
// ---------------------------------------------------------------------------
#[test]
fn c01_all_zero_state() {
    let (c, r) = both();
    let m = Md5::default();
    assert_same(&c, &r, &m, 0, "C1 all-zero");
}

// ---------------------------------------------------------------------------
// C2 — all-ones state
// ---------------------------------------------------------------------------
#[test]
fn c02_all_ones_state() {
    let (c, r) = both();
    let m = Md5::new(u32::MAX, u32::MAX, u32::MAX, u32::MAX);
    assert_same(&c, &r, &m, 0, "C2 all-ones");
}

// ---------------------------------------------------------------------------
// C3 — one-hot BYTE lanes: isolates the out[4*i+j] = field_i >> 8*j mapping
// ---------------------------------------------------------------------------
#[test]
fn c03_one_hot_byte_lanes() {
    let (c, r) = both();
    for field in 0..4usize {
        for lane in 0..4usize {
            let w = 0xFFu32 << (8 * lane);
            let mut words = [0u32; 4];
            words[field] = w;
            let m = Md5::new(words[0], words[1], words[2], words[3]);
            let ctx = format!("C3 field={field} lane={lane}");
            assert_same(&c, &r, &m, 0, &ctx);
            // Also assert the byte actually landed in the expected slot, so a
            // matching-but-wrong mapping in both would still be visible.
            let (out, _) = call_with_guards(c.md5_digest, &m, 0);
            let idx = field * 4 + lane;
            for (i, b) in out.iter().enumerate() {
                let want = if i == idx { 0xFF } else { 0x00 };
                assert_eq!(*b, want, "{ctx}: byte {i}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C4 — one-hot BIT states: all 128 bit positions
// ---------------------------------------------------------------------------
#[test]
fn c04_one_hot_bits() {
    let (c, r) = both();
    for bit in 0..128usize {
        let mut words = [0u32; 4];
        words[bit / 32] = 1u32 << (bit % 32);
        let m = Md5::new(words[0], words[1], words[2], words[3]);
        assert_same(&c, &r, &m, 0, &format!("C4 bit={bit}"));
    }
    // and the complement of each single bit
    for bit in 0..128usize {
        let mut words = [u32::MAX; 4];
        words[bit / 32] = u32::MAX ^ (1u32 << (bit % 32));
        let m = Md5::new(words[0], words[1], words[2], words[3]);
        assert_same(&c, &r, &m, 0, &format!("C4 ~bit={bit}"));
    }
}

// ---------------------------------------------------------------------------
// C5 — boundary word values, swept per field and in random 4-tuples
// ---------------------------------------------------------------------------
#[test]
fn c05_boundary_word_values() {
    let (c, r) = both();
    // per-field sweep, other fields both 0 and MAX
    for filler in [0u32, u32::MAX, 0xDEAD_BEEF] {
        for field in 0..4usize {
            for v in BOUNDARY {
                let mut words = [filler; 4];
                words[field] = v;
                let m = Md5::new(words[0], words[1], words[2], words[3]);
                assert_same(
                    &c,
                    &r,
                    &m,
                    0,
                    &format!("C5 field={field} v={v:#010x} filler={filler:#010x}"),
                );
            }
        }
    }
    // random 4-tuples drawn from the boundary set
    let mut rng = Rng::new(SEED ^ 0xC5);
    for i in 0..3000 {
        let pick = |rng: &mut Rng| BOUNDARY[rng.below(BOUNDARY.len())];
        let m = Md5::new(pick(&mut rng), pick(&mut rng), pick(&mut rng), pick(&mut rng));
        assert_same(&c, &r, &m, 0, &format!("C5 tuple i={i}"));
    }
}

// ---------------------------------------------------------------------------
// C6 — bulk random value fuzz
// ---------------------------------------------------------------------------
#[test]
fn c06_random_states() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED);
    for i in 0..20_000 {
        let m = rng.state();
        assert_same(&c, &r, &m, 0, &format!("C6 i={i} (seed={SEED:#x})"));
    }
}

// ---------------------------------------------------------------------------
// C7 — field-order sensitivity: all 24 permutations of a distinguishable tuple
// ---------------------------------------------------------------------------
#[test]
fn c07_field_order_permutations() {
    let (c, r) = both();
    let base = [0x1122_3344u32, 0x5566_7788, 0x99AA_BBCC, 0xDDEE_FF00];
    let mut idx = [0usize, 1, 2, 3];
    let mut count = 0;
    // simple Heap's-algorithm-free explicit permutation loop
    for i0 in 0..4 {
        for i1 in 0..4 {
            for i2 in 0..4 {
                for i3 in 0..4 {
                    idx = [i0, i1, i2, i3];
                    let mut seen = [false; 4];
                    if idx.iter().any(|&i| std::mem::replace(&mut seen[i], true)) {
                        continue; // not a permutation
                    }
                    let m = Md5::new(base[i0], base[i1], base[i2], base[i3]);
                    assert_same(&c, &r, &m, 0, &format!("C7 perm={idx:?}"));
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 24, "expected 24 permutations, got {count} ({idx:?})");
}

// ---------------------------------------------------------------------------
// C8 — unaligned `m`
// ---------------------------------------------------------------------------
#[test]
fn c08_unaligned_m() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0xC8);
    for m_off in 0..16usize {
        for i in 0..400 {
            let m = rng.state();
            let bytes = m.to_bytes();

            let run = |f: Md5DigestFn| -> ([u8; 16], Vec<u8>) {
                // 48-byte scratch: struct at m_off, output in its own guarded
                // buffer. Deliberately a heap `Vec<u8>` (align 1) and not an
                // array: a stack `[u8; 48]` may be over-aligned by the
                // compiler, which would weaken the unaligned-`m` coverage.
                #[allow(clippy::useless_vec)]
                let mut src = vec![GUARD; 48];
                src[m_off..m_off + 16].copy_from_slice(&bytes);
                let mut dst = vec![GUARD; PAD * 2 + 16];
                unsafe {
                    f(
                        src.as_ptr().add(m_off) as *const Md5,
                        dst.as_mut_ptr().add(PAD),
                    )
                };
                let mut out = [0u8; 16];
                out.copy_from_slice(&dst[PAD..PAD + 16]);
                // the source must be untouched (m is const)
                assert_eq!(&src[m_off..m_off + 16], &bytes[..], "source modified");
                (out, dst)
            };

            let (c_out, c_buf) = run(c.md5_digest);
            let (r_out, r_buf) = run(r.md5_digest);
            assert_eq!(
                c_out, r_out,
                "C8 m_off={m_off} i={i} m={m:08x?}: C={c_out:02x?} Rust={r_out:02x?}"
            );
            assert_eq!(c_buf, r_buf, "C8 m_off={m_off} i={i}: guard bytes differ");
            assert_eq!(c_out, expected_le(&m), "C8 reference sanity");
        }
    }
}

// ---------------------------------------------------------------------------
// C9 — unaligned `out`
// ---------------------------------------------------------------------------
#[test]
fn c09_unaligned_out() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0xC9);
    for out_off in 0..16usize {
        for i in 0..400 {
            let m = rng.state();
            assert_same(&c, &r, &m, out_off, &format!("C9 out_off={out_off} i={i}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Overlap helper: single buffer holding both `m` and `out`
// ---------------------------------------------------------------------------
fn run_overlap(f: Md5DigestFn, init: &[u8], m_off: usize, out_off: usize) -> Vec<u8> {
    let mut buf = init.to_vec();
    let p = buf.as_mut_ptr();
    unsafe { f(p.add(m_off) as *const Md5, p.add(out_off)) };
    buf
}

fn assert_same_overlap(c: &Impl, r: &Impl, init: &[u8], m_off: usize, out_off: usize, ctx: &str) {
    let c_buf = run_overlap(c.md5_digest, init, m_off, out_off);
    let r_buf = run_overlap(r.md5_digest, init, m_off, out_off);
    assert_eq!(
        c_buf, r_buf,
        "overlap mismatch [{ctx}] m_off={m_off} out_off={out_off}\n  init: {init:02x?}\n  C   : {c_buf:02x?}\n  Rust: {r_buf:02x?}"
    );
}

// ---------------------------------------------------------------------------
// C10 — full aliasing: out == (tflac_u8 *)m
// ---------------------------------------------------------------------------
#[test]
fn c10_full_aliasing_in_place() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0x10);
    for i in 0..4000 {
        let m = rng.state();
        let m_off = rng.below(8);
        let mut init = vec![GUARD; 32];
        init[m_off..m_off + 16].copy_from_slice(&m.to_bytes());
        assert_same_overlap(&c, &r, &init, m_off, m_off, &format!("C10 i={i}"));

        // repeated in-place calls must keep converging identically
        let mut c_buf = init.clone();
        let mut r_buf = init.clone();
        for _ in 0..3 {
            c_buf = run_overlap(c.md5_digest, &c_buf, m_off, m_off);
            r_buf = run_overlap(r.md5_digest, &r_buf, m_off, m_off);
        }
        assert_eq!(c_buf, r_buf, "C10 repeated in-place i={i}");
    }
}

// ---------------------------------------------------------------------------
// C11 — partial forward overlap: out = (u8*)m + k
// ---------------------------------------------------------------------------
#[test]
fn c11_partial_forward_overlap() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0x11);
    for k in 1..16usize {
        for i in 0..300 {
            let m = rng.state();
            let m_off = 8usize;
            let mut init = vec![GUARD; 64];
            for (j, slot) in init.iter_mut().enumerate() {
                *slot = (j as u8).wrapping_mul(3).wrapping_add(1);
            }
            init[m_off..m_off + 16].copy_from_slice(&m.to_bytes());
            assert_same_overlap(&c, &r, &init, m_off, m_off + k, &format!("C11 k={k} i={i}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — partial backward overlap: out = (u8*)m - k
// ---------------------------------------------------------------------------
#[test]
fn c12_partial_backward_overlap() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0x12);
    for k in 1..16usize {
        for i in 0..300 {
            let m = rng.state();
            let m_off = 24usize;
            let mut init = vec![GUARD; 64];
            for (j, slot) in init.iter_mut().enumerate() {
                *slot = (j as u8).wrapping_mul(7).wrapping_add(5);
            }
            init[m_off..m_off + 16].copy_from_slice(&m.to_bytes());
            assert_same_overlap(&c, &r, &init, m_off, m_off - k, &format!("C12 k={k} i={i}"));
        }
    }
}

// ---------------------------------------------------------------------------
// C15 — storage-class matrix for `m` and `out`
// ---------------------------------------------------------------------------
static mut STATIC_SRC: [u8; 64] = [0; 64];
static mut STATIC_DST: [u8; 64] = [0; 64];

#[derive(Clone, Copy, Debug)]
enum Store {
    Stack,
    Heap,
    Static,
}

fn call_stored(f: Md5DigestFn, m: &Md5, src: Store, dst: Store) -> [u8; 16] {
    let bytes = m.to_bytes();
    let stack_src: [u8; 16];
    let mut stack_dst = [GUARD; 16];
    let mut heap_src: Box<[u8; 16]> = Box::new([0u8; 16]);
    let mut heap_dst: Box<[u8; 16]> = Box::new([GUARD; 16]);

    let src_ptr: *const u8 = match src {
        Store::Stack => {
            stack_src = bytes;
            stack_src.as_ptr()
        }
        Store::Heap => {
            *heap_src = bytes;
            heap_src.as_ptr()
        }
        Store::Static => unsafe {
            let p = (&raw mut STATIC_SRC) as *mut u8;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), p, 16);
            p as *const u8
        },
    };
    let dst_ptr: *mut u8 = match dst {
        Store::Stack => stack_dst.as_mut_ptr(),
        Store::Heap => heap_dst.as_mut_ptr(),
        Store::Static => unsafe {
            let p = (&raw mut STATIC_DST) as *mut u8;
            std::ptr::write_bytes(p, GUARD, 64);
            p
        },
    };

    unsafe { f(src_ptr as *const Md5, dst_ptr) };
    let mut out = [0u8; 16];
    unsafe { std::ptr::copy_nonoverlapping(dst_ptr as *const u8, out.as_mut_ptr(), 16) };
    out
}

#[test]
fn c15_storage_class_matrix() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0x15);
    let stores = [Store::Stack, Store::Heap, Store::Static];
    for src in stores {
        for dst in stores {
            for i in 0..200 {
                let m = rng.state();
                let c_out = call_stored(c.md5_digest, &m, src, dst);
                let r_out = call_stored(r.md5_digest, &m, src, dst);
                assert_eq!(
                    c_out, r_out,
                    "C15 src={src:?} dst={dst:?} i={i} m={m:08x?}: C={c_out:02x?} Rust={r_out:02x?}"
                );
                assert_eq!(c_out, expected_le(&m), "C15 reference sanity");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C16 — statelessness / call sequencing
// ---------------------------------------------------------------------------
#[test]
fn c16_statelessness_and_sequencing() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0x16);

    // Same output buffer reused across many different inputs.
    let mut c_buf = vec![GUARD; 16];
    let mut r_buf = vec![GUARD; 16];
    let probe = Md5::new(0x0123_4567, 0x89AB_CDEF, 0xFEDC_BA98, 0x7654_3210);
    let mut first_probe: Option<Vec<u8>> = None;

    for i in 0..2000 {
        let m = if i % 7 == 0 { probe } else { rng.state() };
        unsafe { (c.md5_digest)(&m as *const Md5, c_buf.as_mut_ptr()) };
        unsafe { (r.md5_digest)(&m as *const Md5, r_buf.as_mut_ptr()) };
        assert_eq!(c_buf, r_buf, "C16 i={i} m={m:08x?}");
        if i % 7 == 0 {
            match &first_probe {
                None => first_probe = Some(c_buf.clone()),
                Some(prev) => assert_eq!(
                    prev, &c_buf,
                    "C16 i={i}: repeated probe input produced a different result (hidden state?)"
                ),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C17 — reentrancy: concurrent calls from multiple threads
// ---------------------------------------------------------------------------
#[test]
fn c17_reentrancy_threads() {
    let (c, r) = both();
    let c_addr = c.md5_digest as usize;
    let r_addr = r.md5_digest as usize;

    let mut handles = Vec::new();
    for t in 0..4u64 {
        handles.push(std::thread::spawn(move || {
            let cf: Md5DigestFn = unsafe { std::mem::transmute(c_addr) };
            let rf: Md5DigestFn = unsafe { std::mem::transmute(r_addr) };
            let mut rng = Rng::new(SEED ^ (0x17_0000 + t));
            for i in 0..5000 {
                let m = rng.state();
                let mut co = [0u8; 16];
                let mut ro = [0u8; 16];
                unsafe { cf(&m as *const Md5, co.as_mut_ptr()) };
                unsafe { rf(&m as *const Md5, ro.as_mut_ptr()) };
                assert_eq!(co, ro, "C17 thread={t} i={i} m={m:08x?}");
            }
        }));
    }
    for h in handles {
        h.join().expect("thread panicked");
    }
}

// ---------------------------------------------------------------------------
// C18 — combined-axis fuzz: values x m-alignment x out-alignment x overlap
// ---------------------------------------------------------------------------
#[test]
fn c18_combined_axis_fuzz() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0x18);
    for i in 0..20_000 {
        let m = rng.state();
        let m_off = 24 + rng.below(16); // 24..39, arbitrary alignment
        let overlap_mode = rng.below(4);
        let out_off = match overlap_mode {
            0 => 4 + rng.below(16),            // fully disjoint, before m
            1 => 48 + rng.below(16),           // fully disjoint, after m
            2 => m_off,                        // fully aliased
            _ => m_off + rng.below(31) - 15,   // any partial overlap +-15
        };
        let mut init = vec![0u8; 96];
        for (j, slot) in init.iter_mut().enumerate() {
            *slot = (j as u8).wrapping_mul(11).wrapping_add(i as u8);
        }
        init[m_off..m_off + 16].copy_from_slice(&m.to_bytes());
        assert_same_overlap(
            &c,
            &r,
            &init,
            m_off,
            out_off,
            &format!("C18 i={i} mode={overlap_mode} (seed={:#x})", SEED ^ 0x18),
        );
    }
}
