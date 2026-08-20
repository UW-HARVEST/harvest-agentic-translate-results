//! Differential tests for the `create_block` export.
//!
//! Covers CONFIGS.md rows 1-9 and 45, and ERRORS.md rows 10, 11 and 16.
//!
//! Note N1 from CONFIGS.md applies: the C function starts from an
//! *uninitialised* `DataBlock` and only `strcpy`s `strlen(name)+1` bytes, so
//! `name[strlen+1 ..]` is indeterminate stack garbage in C (empirically
//! non-zero).  Only the bytes the C code actually defines are compared.

mod common;

use common::*;
use core::ffi::c_char;

/// Compare the *defined* part of two returned `DataBlock`s.
fn cmp_block(ctx: &str, c: &DataBlock, r: &DataBlock, name_len: usize) {
    assert_eq!(c.id, r.id, "{ctx}: id mismatch");
    assert_eq!(c.flags, r.flags, "{ctx}: flags mismatch");
    // Defined region: the copied bytes plus the NUL terminator, clamped to the
    // field.  For name_len >= 31 the whole field was written.
    let n = (name_len + 1).min(32);
    let cn: Vec<u8> = c.name[..n].iter().map(|&b| b as u8).collect();
    let rn: Vec<u8> = r.name[..n].iter().map(|&b| b as u8).collect();
    assert_eq!(cn, rn, "{ctx}: name[0..{n}] mismatch");
}

fn call_both(c: &Impl, r: &Impl, id: i32, name: &[u8], flags: u8, ctx: &str) {
    // `name` must already be NUL terminated.
    assert_eq!(*name.last().unwrap(), 0, "test bug: name not NUL terminated");
    let len = name.len() - 1;
    unsafe {
        let a = (c.create_block)(id, name.as_ptr() as *const c_char, flags);
        let b = (r.create_block)(id, name.as_ptr() as *const c_char, flags);
        cmp_block(ctx, &a, &b, len);
    }
}

fn nul_terminated(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

#[test]
fn create_block_differential() {
    let (c, r) = both();
    let mut rng = Rng::new(SEED ^ 0x0000_0001);

    // ---- row 1: empty name, zero id, zero flags -------------------------
    call_both(&c, &r, 0, b"\0", 0, "row1/empty");
    for _ in 0..64 {
        call_both(&c, &r, 0, b"\0", rng.next_u8(), "row1/empty-rand-flags");
    }

    // ---- row 2: name length 1 -------------------------------------------
    for _ in 0..256 {
        let ch = 1 + (rng.below(255) as u8); // never 0 (would shorten the string)
        let name = nul_terminated(&[ch]);
        call_both(&c, &r, rng.next_i32(), &name, rng.next_u8(), "row2/len1");
    }

    // ---- row 3: name length 2..30, random ASCII -------------------------
    for _ in 0..600 {
        let len = 2 + rng.below(29) as usize; // 2..=30
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(0x20 + (rng.below(95) as u8)); // printable ASCII
        }
        let name = nul_terminated(&buf);
        call_both(&c, &r, rng.next_i32(), &name, rng.next_u8(), "row3/ascii");
    }

    // ---- row 4: name length exactly 31 (fills the field) ----------------
    for _ in 0..128 {
        let mut buf = Vec::with_capacity(31);
        for _ in 0..31 {
            buf.push(1 + rng.below(255) as u8);
        }
        let name = nul_terminated(&buf);
        call_both(&c, &r, rng.next_i32(), &name, rng.next_u8(), "row4/len31");
    }

    // ---- row 5 / ERRORS #11: length 32..35 ------------------------------
    // `strcpy` runs past `name[32]`, but `name` sits at offset 4 of a 40-byte
    // struct, so lengths <= 35 keep the NUL inside the object.  `flags` (offset
    // 36) is assigned *after* the strcpy, so it survives being clobbered.
    for len in 32..=35usize {
        for _ in 0..32 {
            let mut buf = Vec::with_capacity(len);
            for _ in 0..len {
                buf.push(1 + rng.below(255) as u8);
            }
            let name = nul_terminated(&buf);
            let id = rng.next_i32();
            let flags = rng.next_u8();
            call_both(&c, &r, id, &name, flags, &format!("row5/len{len}"));
        }
    }

    // ---- row 6: bytes with the high bit set (signed char) ---------------
    for len in 1..=31usize {
        for _ in 0..8 {
            let mut buf = Vec::with_capacity(len);
            for _ in 0..len {
                buf.push(0x80 | (rng.below(128) as u8));
            }
            let name = nul_terminated(&buf);
            call_both(
                &c,
                &r,
                rng.next_i32(),
                &name,
                rng.next_u8(),
                &format!("row6/highbit{len}"),
            );
        }
    }
    // explicit 0x01 / 0xFF mix
    for len in 1..=31usize {
        let buf: Vec<u8> = (0..len)
            .map(|i| if i % 2 == 0 { 0xFF } else { 0x01 })
            .collect();
        let name = nul_terminated(&buf);
        call_both(&c, &r, -1, &name, 0xFF, &format!("row6/mix{len}"));
    }

    // ---- row 7: exhaustive flags sweep ---------------------------------
    for f in 0u16..=255 {
        call_both(
            &c,
            &r,
            7,
            b"flagsweep\0",
            f as u8,
            &format!("row7/flags{f}"),
        );
    }

    // ---- row 8: id extremes x flags extremes ---------------------------
    for &id in &[
        0i32,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        1_000_000_000,
        -1_000_000_000,
        i32::MAX - 1,
        i32::MIN + 1,
    ] {
        for &f in &[0u8, 0x01, 0x0F, 0x55, 0xAA, 0xF0, 0xFF] {
            call_both(&c, &r, id, b"idsweep\0", f, &format!("row8/id{id}/f{f}"));
        }
    }

    // ---- row 9: fully random ------------------------------------------
    for i in 0..2000 {
        let len = rng.below(32) as usize; // 0..=31
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            buf.push(1 + rng.below(255) as u8);
        }
        let name = nul_terminated(&buf);
        call_both(
            &c,
            &r,
            rng.interesting_i32(),
            &name,
            rng.next_u8(),
            &format!("row9/rand{i}"),
        );
    }

    // ---- row 45 / ERRORS #16: out-of-range value in the narrow `flags` --
    // The ABI passes `uint8_t flags` in a 32-bit register slot; a caller that
    // declares the symbol with an `int` parameter can put arbitrary bits there.
    for &wide in &[
        0i32,
        0xFF,
        0x1FF,
        0x100,
        -1,
        0x7FFF_FF00,
        i32::MIN,
        0xDEAD_BEEFu32 as i32,
        256,
        257,
        -256,
        -255,
    ] {
        unsafe {
            let name = b"wideflags\0";
            let a = (c.create_block_wide)(11, name.as_ptr() as *const c_char, wide);
            let b = (r.create_block_wide)(11, name.as_ptr() as *const c_char, wide);
            cmp_block(&format!("row45/wide{wide:#x}"), &a, &b, name.len() - 1);
            // and the value really is the low byte
            assert_eq!(
                a.flags,
                (wide as u32 & 0xFF) as u8,
                "row45: C did not truncate to the low byte"
            );
        }
    }

    // ---- ERRORS #10: name == NULL -> SIGSEGV in both -------------------
    let (ca, ra) = fork_pair(|which, _buf| {
        let imp = if which { &r } else { &c };
        unsafe {
            let blk = (imp.create_block)(1, core::ptr::null(), 2);
            // Unreachable: strcpy dereferenced NULL.  Consume the value so the
            // call cannot be optimised out.
            core::hint::black_box(blk.id);
        }
        0
    });
    assert_eq!(
        ca.signal(),
        Some(11),
        "ERRORS#10: C create_block(NULL) should die with SIGSEGV, got {}",
        ca.describe()
    );
    assert_eq!(
        ra.signal(),
        ca.signal(),
        "ERRORS#10: Rust create_block(NULL) differs: C={} Rust={}",
        ca.describe(),
        ra.describe()
    );
}
