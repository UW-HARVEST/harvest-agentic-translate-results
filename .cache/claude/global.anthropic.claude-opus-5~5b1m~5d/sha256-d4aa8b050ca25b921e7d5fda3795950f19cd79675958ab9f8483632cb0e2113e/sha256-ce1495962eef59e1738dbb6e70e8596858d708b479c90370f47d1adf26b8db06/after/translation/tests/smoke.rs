//! Human-readable known-answer vectors.
//!
//! A short, printable differential check that doubles as documentation of what
//! the two `.so`s actually return. The exhaustive coverage lives in
//! `phase_b_valid.rs` (valid paths) and `phase_c_errors.rs` (boundaries); this
//! file exists so `cargo test -- --nocapture` shows concrete evidence that the
//! C and Rust libraries agree, and prints which artifacts were loaded.

mod common;

#[test]
fn known_answer_vectors_agree() {
    let p = common::pair();
    println!("C    .so = {}", p.c.path.display());
    println!("Rust .so = {}", p.rust.path.display());

    // (input, seed) pairs spanning: empty, tail-only, exactly one wide block,
    // wide+tail, and an extreme seed.
    let vectors: &[(&[u8], u16)] = &[
        (b"", 0x0000),
        (b"", 0xFFFF),
        (b"a", 0x0000),
        (b"1234567", 0x0000),  // 7 bytes: tail only
        (b"12345678", 0x0000), // 8 bytes: one wide block
        (b"12345678", 0xFFFF),
        (b"123456789", 0x0000), // wide + 1-byte tail
        (b"The quick brown fox jumps over the lazy dog", 0x1234),
        (&[0x00; 16], 0x0000),
        (&[0xFF; 16], 0xFFFF),
    ];

    let mut failures = 0;
    for (data, seed) in vectors {
        let c = p.c.crc16(data, *seed);
        let r = p.rust.crc16(data, *seed);
        let mark = if c == r { "ok" } else { "** MISMATCH **" };
        if c != r {
            failures += 1;
        }
        println!(
            "  len={:<3} seed=0x{:04x} -> C=0x{:04x}  Rust=0x{:04x}  {}",
            data.len(),
            seed,
            c,
            r,
            mark
        );
    }
    assert_eq!(failures, 0, "{failures} known-answer vector(s) diverged");
}
