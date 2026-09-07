//! Phase C — error/rejection-path differential tests, gated on `ERRORS.md`.
//!
//! The C has no explicit error return (see the grep recorded in `ERRORS.md`), so
//! each row here pins the *implicit* rejection behaviour: the out-of-declared-
//! bounds table reads, the aliased rows, and the generic C-API boundaries
//! (null pointer, undersized buffer, out-of-range enum-like field).
//!
//! Both sides are always reached through their `.so` exports.

mod common;

use common::{Pair, Rng};

fn h(i: u8, layer: u8, k: u8) -> [u8; 3] {
    [0, (i << 3) | (layer << 1), k << 4]
}

/// Row 1 — reserved layer `0b00` with i=0 and k=0..14 => flat offset -15..-1,
/// a read *before* the table. C reads zero padding => 0.
#[test]
fn errors_row1_reserved_layer_negative_offset() {
    let pair = Pair::load();
    let mut rng = Rng::new(1);
    for k in 0u8..15 {
        for _ in 0..32 {
            let buf = [rng.next_u8(), (rng.next_u8() & 0xF0) | 0x00 | (rng.next_u8() & 1), (k << 4) | (rng.next_u8() & 0xF)];
            // force i=0 (bit3 clear) and layer=0b00 (bits2..1 clear)
            let buf = [buf[0], buf[1] & !0x0E, buf[2]];
            assert_eq!((buf[1] >> 1) & 3, 0);
            assert_eq!(buf[1] & 0x8, 0);
            let v = pair.assert_same(&buf, "ERRORS row 1: offset -15..-1 (OOB before table)");
            assert_eq!(v, 0, "expected 0 from zero padding, got {v}");
        }
    }
}

/// Row 2 — reserved layer, i=0, k=15 => offset 0, folds back into the table at
/// `halfrate[0][0][0]` == 0.
#[test]
fn errors_row2_reserved_layer_wraps_to_offset_zero() {
    let pair = Pair::load();
    let v = pair.assert_same(&h(0, 0, 15), "ERRORS row 2: offset 0");
    assert_eq!(v, 0);
}

/// Row 3 — reserved layer with i=1 => offsets 30..45, silently aliasing
/// `halfrate[0][2][*]`. Pins the exact aliased sequence.
#[test]
fn errors_row3_reserved_layer_aliases_other_row() {
    let pair = Pair::load();
    let expected: [u32; 16] = [
        0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256, 0,
    ];
    for k in 0u8..16 {
        let v = pair.assert_same(&h(1, 0, k), "ERRORS row 3: aliased row");
        assert_eq!(
            v, expected[k as usize],
            "aliased read at k={k}: C/Rust agreed on {v} but table says {}",
            expected[k as usize]
        );
    }
}

/// Row 4 — "bad" bitrate nibble k=15 with a valid layer, (i,j) != (1,2): the
/// index runs one past the row and lands on the next row's first byte, which is
/// 0 in every row.
#[test]
fn errors_row4_bad_nibble_reads_next_row_first_byte() {
    let pair = Pair::load();
    for (i, layer) in [(0u8, 1u8), (0, 2), (0, 3), (1, 1), (1, 2)] {
        let v = pair.assert_same(&h(i, layer, 15), "ERRORS row 4: k=15 crosses into next row");
        assert_eq!(v, 0, "i={i} layer={layer}: expected 0, got {v}");
    }
}

/// Row 5 — k=15 with i=1, layer=0b11 => flat offset 90, one byte *past* the
/// 90-byte table. C reads `.rodata` alignment padding => 0.
#[test]
fn errors_row5_offset_90_past_end_of_table() {
    let pair = Pair::load();
    let buf = h(1, 3, 15);
    assert_eq!(u8::from(buf[1] & 0x8 != 0), 1);
    assert_eq!((buf[1] >> 1) & 3, 3);
    assert_eq!(buf[2] >> 4, 15);
    let v = pair.assert_same(&buf, "ERRORS row 5: offset 90 (OOB past table)");
    assert_eq!(v, 0, "expected 0 from rodata padding, got {v}");
}

// ---------------------------------------------------------------------------
// Row 6 — null pointer. Neither side checks, so both must fault identically.
// Compared by running each call in a forked child and matching the death
// signal, rather than by comparing return values (there are none).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

/// Runs `f` in a child process. Returns `Err(signal)` if it died by signal,
/// `Ok(exit_code)` otherwise.
unsafe fn run_isolated(f: impl FnOnce()) -> Result<i32, i32> { unsafe {
    let pid = fork();
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        f();
        _exit(0);
    }
    let mut status: i32 = 0;
    let r = waitpid(pid, &mut status, 0);
    assert_eq!(r, pid, "waitpid failed");
    if (status & 0x7f) != 0 {
        Err(status & 0x7f) // WTERMSIG
    } else {
        Ok((status >> 8) & 0xff) // WEXITSTATUS
    }
}}

#[test]
fn errors_row6_null_pointer_crash_parity() {
    let pair = Pair::load();
    let (c, rust) = (pair.c, pair.rust);

    let c_res = unsafe {
        run_isolated(|| {
            let v = c(std::ptr::null());
            std::hint::black_box(v);
        })
    };
    let rust_res = unsafe {
        run_isolated(|| {
            let v = rust(std::ptr::null());
            std::hint::black_box(v);
        })
    };

    assert_eq!(
        c_res, rust_res,
        "null-pointer behaviour diverges: C = {c_res:?}, Rust = {rust_res:?}"
    );
    assert_eq!(c_res, Err(11), "expected both to die with SIGSEGV(11), got {c_res:?}");
}

// ---------------------------------------------------------------------------
// Row 7 — undersized buffer / exact read footprint, enforced with a guard page.
// ---------------------------------------------------------------------------

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 2;
const MAP_ANONYMOUS: i32 = 0x20;

unsafe extern "C" {
    fn mmap(
        addr: *mut u8,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        off: i64,
    ) -> *mut u8;
    fn mprotect(addr: *mut u8, len: usize, prot: i32) -> i32;
}

/// Maps two pages, makes the second unreadable, and returns
/// (base, first_page_end). Any read at or past `first_page_end` faults.
unsafe fn guarded_pair_of_pages() -> (*mut u8, *mut u8) { unsafe {
    let page = 4096usize;
    let base = mmap(
        std::ptr::null_mut(),
        page * 2,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    assert!(!base.is_null() && base as isize != -1, "mmap failed");
    assert_eq!(mprotect(base.add(page), page, PROT_NONE), 0, "mprotect failed");
    (base, base.add(page))
}}

/// Both implementations must read *only* `h[1]` and `h[2]` — never `h[3]` or
/// beyond. With the 3-byte header butted against a guard page, any read past
/// `h[2]` segfaults. Both sides must survive and agree, for every `(h1, h2)`.
#[test]
fn errors_row7_reads_no_further_than_h2() {
    let pair = Pair::load();
    unsafe {
        let (_base, guard) = guarded_pair_of_pages();
        let hp = guard.sub(3); // h[2] is the final readable byte
        for h1 in 0u16..256 {
            for h2 in 0u16..256 {
                *hp.add(0) = 0xAA;
                *hp.add(1) = h1 as u8;
                *hp.add(2) = h2 as u8;
                let cv = (pair.c)(hp) as u32;
                let rv = (pair.rust)(hp) as u32;
                assert_eq!(
                    cv, rv,
                    "guard-page divergence at h1={h1:#04x} h2={h2:#04x}: C={cv} Rust={rv}"
                );
            }
        }
    }
}

/// A buffer with fewer than 3 readable bytes: the C has no length parameter and
/// dereferences `h[1]` unconditionally, so both must fault the same way.
#[test]
fn errors_row7b_undersized_buffer_crash_parity() {
    let pair = Pair::load();
    let (c, rust) = (pair.c, pair.rust);

    // Only 1 readable byte before the guard page => reading h[1] must fault.
    for readable in [0usize, 1, 2] {
        let c_res = unsafe {
            run_isolated(|| {
                let (_b, guard) = guarded_pair_of_pages();
                let hp = guard.sub(readable);
                std::hint::black_box(c(hp));
            })
        };
        let rust_res = unsafe {
            run_isolated(|| {
                let (_b, guard) = guarded_pair_of_pages();
                let hp = guard.sub(readable);
                std::hint::black_box(rust(hp));
            })
        };
        assert_eq!(
            c_res, rust_res,
            "undersized buffer ({readable} readable bytes): C = {c_res:?}, Rust = {rust_res:?}"
        );
    }
}

/// Row 8 — `h[0]` is never read; result must be invariant under it. (Also
/// covered in Phase B; repeated here so the row has its own test.)
#[test]
fn errors_row8_h0_never_read() {
    let pair = Pair::load();
    for h1 in [0x00u8, 0x02, 0x08, 0x0E, 0xFF] {
        for h2 in [0x00u8, 0x10, 0xF0, 0xFF] {
            let base = pair.assert_same(&[0, h1, h2], "ERRORS row 8 baseline");
            for h0 in 0u16..256 {
                let got = pair.assert_same(&[h0 as u8, h1, h2], "ERRORS row 8 sweep");
                assert_eq!(got, base, "h[0] affected result (h1={h1:#04x} h2={h2:#04x})");
            }
        }
    }
}

/// Row 9 — the out-of-range "enum" class. `h[1]`/`h[2]` are `uint8_t`, so every
/// one of the 256 values is representable and none can be rejected; the layer
/// field's reserved `0b00` is this API's no-valid-variant value. Sweep all 256
/// values of each byte against the other held at each edge value.
#[test]
fn errors_row9_no_invalid_encoding_is_rejected() {
    let pair = Pair::load();
    for fixed in [0x00u8, 0x0F, 0xF0, 0xFF, 0x08, 0x02] {
        for v in 0u16..256 {
            // vary h[1]
            pair.assert_same(&[0, v as u8, fixed], "ERRORS row 9: h1 sweep");
            // vary h[2]
            pair.assert_same(&[0, fixed, v as u8], "ERRORS row 9: h2 sweep");
        }
    }
}

/// Extra generic boundary: an "enum-ish" value one step past every documented
/// field range. The fields are 1-bit / 2-bit / 4-bit and saturate at their
/// masks, so the step-past value aliases back into range — assert both agree.
#[test]
fn errors_boundary_one_past_each_field_range() {
    let pair = Pair::load();
    // layer field is 2 bits: 4 would be "one past" but cannot be encoded; it
    // aliases to layer 0. Verify the aliasing matches on both sides.
    for layer_raw in 0u8..8 {
        let h1 = (layer_raw << 1) & 0xFF;
        for k_raw in 0u8..17 {
            let h2 = (k_raw << 4) & 0xFF; // k_raw = 16 aliases to k = 0
            pair.assert_same(&[0, h1, h2], "boundary: one past field ranges");
        }
    }
}
