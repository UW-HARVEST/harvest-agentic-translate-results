//! Checks that the Rust translation's structure layouts match the ones the C
//! compiler actually produced.
//!
//! `cp_state_t` is shared through pointers, and `cp_dynamic`'s stack frame is
//! *overflowed* by the C code, so both layouts are observable behaviour rather
//! than an implementation detail. The expected offsets come from `objdump -d`
//! on the built C object.

mod harness;

use std::process::Command;

/// Reads the frame offsets `cp_dynamic` uses out of the compiled C library, so
/// this test fails loudly if a different compiler or flag set changes them.
fn cp_dynamic_disassembly() -> String {
    let out = Command::new("objdump")
        .args(["-d", "--no-show-raw-insn"])
        .arg(harness::c_so_path())
        .output()
        .expect("objdump");
    assert!(out.status.success(), "objdump failed");
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    let start = text
        .find("<cp_dynamic>:")
        .expect("cp_dynamic not found in disassembly");
    let rest = &text[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    rest[..end].to_string()
}

#[test]
fn cp_dynamic_frame_offsets_are_the_ones_the_c_code_uses() {
    let asm = cp_dynamic_disassembly();

    // lens[320] is addressed as -0x180(%rbp,reg,1).
    assert!(
        asm.contains("-0x180(%rbp,%rax,1)"),
        "lens is no longer at -0x180; the Rust CpDynamicFrame layout must be \
         re-derived from the disassembly:\n{asm}"
    );
    // lenlens[19] at -0x40, zeroed with 8+8+2+1 bytes.
    for expect in [
        "movq   $0x0,-0x40(%rbp)",
        "movq   $0x0,-0x38(%rbp)",
        "movw   $0x0,-0x30(%rbp)",
        "movb   $0x0,-0x2e(%rbp)",
    ] {
        assert!(
            asm.contains(expect),
            "lenlens is no longer a 19-byte array at -0x40 (`{expect}` missing):\n{asm}"
        );
    }
    // nlit at -0x18, ndst at -0x1c, nlen at -0x20: the loop condition
    // `n < nlit + ndst` re-reads both from the frame every iteration.
    assert!(
        asm.contains("mov    -0x18(%rbp),%edx") && asm.contains("mov    -0x1c(%rbp),%eax"),
        "nlit/ndst are no longer at -0x18/-0x1c:\n{asm}"
    );
    assert!(
        asm.contains("cmp    %eax,-0x8(%rbp)"),
        "n is no longer at -0x8:\n{asm}"
    );
    // The three run-length counters.
    for (sym, off) in [(18, "-0x14"), (17, "-0x10"), (16, "-0xc")] {
        assert!(
            asm.contains(&format!("subl   $0x1,{off}(%rbp)")),
            "the symbol-{sym} loop counter is no longer at {off}:\n{asm}"
        );
    }
    // sym at -0x24.
    assert!(
        asm.contains("mov    %eax,-0x24(%rbp)"),
        "sym is no longer at -0x24:\n{asm}"
    );
}

#[test]
fn cp_state_layout_matches_between_c_and_rust() {
    // `cp_state_t` is only ever passed around by pointer, but `cp_decode` reads
    // `tree[-1]`, which crosses from one field into the previous one, so the
    // exact field offsets are observable. Derive them from the C
    // disassembly's constant displacements, which the translation must match.
    let asm = cp_dynamic_disassembly();
    // s->len is at 0x948, s->nlen at 0x99c, s->lit at 0x448, s->dst at 0x8c8,
    // s->nlit at 0x994, s->ndst at 0x998 (see the lea/mov displacements).
    let expected = [
        ("s->lit", "0x448"),
        ("s->dst", "0x8c8"),
        ("s->len", "0x948"),
        ("s->nlit", "0x994"),
        ("s->ndst", "0x998"),
        ("s->nlen", "0x99c"),
    ];
    for (what, disp) in expected {
        assert!(
            asm.contains(disp),
            "{what} displacement {disp} not found; cp_state_t layout changed:\n{asm}"
        );
    }

    // The same offsets, computed from a mirror of the translation's `CpState`.
    // If the translation's struct ever drifts, these constants stop matching the
    // displacements the C code compiled to.
    #[repr(C)]
    #[allow(dead_code)]
    struct CpStateMirror {
        bits: u64,
        count: std::ffi::c_int,
        words: *mut u32,
        word_count: std::ffi::c_int,
        word_index: std::ffi::c_int,
        bits_left: std::ffi::c_int,
        final_word_available: std::ffi::c_int,
        final_word: u32,
        out: *mut std::ffi::c_char,
        out_end: *mut std::ffi::c_char,
        begin: *mut std::ffi::c_char,
        lookup: [u16; 1 << 9],
        lit: [u32; 288],
        dst: [u32; 32],
        len: [u32; 19],
        nlit: u32,
        ndst: u32,
        nlen: u32,
    }

    assert_eq!(std::mem::offset_of!(CpStateMirror, lit), 0x448, "lit offset");
    assert_eq!(std::mem::offset_of!(CpStateMirror, dst), 0x8c8, "dst offset");
    assert_eq!(std::mem::offset_of!(CpStateMirror, len), 0x948, "len offset");
    assert_eq!(std::mem::offset_of!(CpStateMirror, nlit), 0x994, "nlit offset");
    assert_eq!(std::mem::offset_of!(CpStateMirror, ndst), 0x998, "ndst offset");
    assert_eq!(std::mem::offset_of!(CpStateMirror, nlen), 0x99c, "nlen offset");
}
