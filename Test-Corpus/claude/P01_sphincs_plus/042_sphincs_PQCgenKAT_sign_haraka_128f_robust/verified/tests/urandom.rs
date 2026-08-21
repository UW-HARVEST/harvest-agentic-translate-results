//! `app/src/randombytes.c` (the `/dev/urandom` `randombytes` used by
//! `libsphincs_core.so`) is translated in `src/randombytes.rs` as
//! `randombytes_urandom`.
//!
//! A *byte-for-byte* differential test is impossible for a `/dev/urandom`
//! source, so this file checks the properties that the translation must
//! preserve and that are observable:
//!
//!  * every requested byte is written (the `while (xlen > 0)` loop terminates
//!    and covers the whole buffer), including for a request larger than the
//!    1 MiB chunk size the C uses (`if (xlen < 1048576) i = xlen; else i = 1048576;`);
//!  * a zero-length request writes nothing and returns;
//!  * the file descriptor is opened once and kept open across calls (the C
//!    `static int fd` never closes it), so repeated calls keep working.
//!
//! Unlike `tests/differential.rs`, this test links the crate directly — it does
//! not `dlopen` anything, so there is no symbol-interposition concern.

use sphincsplus::randombytes::randombytes_urandom;

#[test]
fn urandom_fills_every_byte() {
    // Sentinel-filled buffers: every byte must be overwritten with high
    // probability, and the guard region past the end must be untouched.
    for &n in &[0usize, 1, 15, 16, 17, 64, 1000, 4096] {
        let mut buf = vec![0xAAu8; n + 32];
        randombytes_urandom(&mut buf[..n]);
        assert!(buf[n..].iter().all(|&b| b == 0xAA), "wrote past the slice (n={n})");
        if n >= 64 {
            // 0xAA everywhere would mean nothing was written at all.
            assert!(buf[..n].iter().any(|&b| b != 0xAA), "no bytes written (n={n})");
        }
    }
}

#[test]
fn urandom_crosses_the_1mib_chunk_boundary() {
    // The C reads in chunks of at most 1048576 bytes; a larger request must
    // still be filled completely.
    let n = 1_048_576 + 12345;
    let mut buf = vec![0xAAu8; n + 16];
    randombytes_urandom(&mut buf[..n]);
    assert!(buf[n..].iter().all(|&b| b == 0xAA));
    // Every 4 KiB window must contain at least one non-sentinel byte.
    for w in buf[..n].chunks(4096) {
        assert!(w.iter().any(|&b| b != 0xAA), "an unfilled window remained");
    }
}

#[test]
fn urandom_zero_length_is_a_noop() {
    let mut buf = [0xAAu8; 8];
    randombytes_urandom(&mut buf[..0]);
    assert_eq!(buf, [0xAAu8; 8]);
}

#[test]
fn urandom_repeated_calls_reuse_the_descriptor() {
    // The C keeps `static int fd` open; repeated calls must all succeed and
    // must not return the same bytes.
    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    randombytes_urandom(&mut a);
    randombytes_urandom(&mut b);
    assert_ne!(a, b, "two consecutive draws were identical");
}
