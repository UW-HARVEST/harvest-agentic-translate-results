//! Harness self-check: both `.so`s load, export `ima_parse`, and agree on a
//! minimal well-formed stream.

mod common;

use common::*;

#[test]
fn both_shared_objects_expose_ima_parse() {
    let _c = c_ima_parse();
    let _r = rust_ima_parse();
}

#[test]
fn minimal_valid_stream_agrees() {
    let rng = Rng::new(1);
    let mut caf = Caf::valid_header(&rng);
    caf.desc([0; 4], &Desc::ima4());
    caf.pakt([0; 4], &Pakt::new(1024));
    caf.data([0; 4], 4096, 0, &[0u8; 64]);

    let out = assert_same("minimal", &caf.buf);
    assert_eq!(out.ret, 0, "expected success, got {:?}", out);
    assert_eq!(out.info.size(), 4096);
    assert_eq!(out.info.frame_count(), 1024);
    assert_eq!(out.info.channel_count(), 1);
    assert_eq!(out.info.blocks(), caf.expected_blocks(caf.buf.as_ptr()));
    // A big-endian 44100.0 read as a native double is a tiny subnormal, which
    // truncates to 0; byte-swapping 0 and bit-casting gives +0.0. This is the
    // C's behaviour for every realistic input.
    assert_eq!(out.info.sample_rate_bits(), 0);
    // Neither implementation may touch the 4 tail padding bytes.
    assert_eq!(out.info.tail_padding(), [POISON; 4]);
}

#[test]
fn error_returns_agree() {
    let rng = Rng::new(2);

    // -1: bad magic.
    let mut bad_magic = Caf::new(*b"CAFF", 1, 0);
    bad_magic.desc([0; 4], &Desc::ima4());
    bad_magic.pakt([0; 4], &Pakt::new(1));
    bad_magic.data([0; 4], 0, 0, &[]);
    assert_eq!(assert_same("bad_magic", &bad_magic.buf).ret, -1);

    // -2: bad version.
    let mut bad_ver = Caf::new(*b"caff", 2, 0);
    bad_ver.desc([0; 4], &Desc::ima4());
    bad_ver.pakt([0; 4], &Pakt::new(1));
    bad_ver.data([0; 4], 0, 0, &[]);
    assert_eq!(assert_same("bad_version", &bad_ver.buf).ret, -2);

    // -3: bad format id.
    let mut bad_fmt = Caf::valid_header(&rng);
    let mut d = Desc::ima4();
    d.format_id = *b"lpcm";
    bad_fmt.desc([0; 4], &d);
    bad_fmt.pakt([0; 4], &Pakt::new(1));
    bad_fmt.data([0; 4], 0, 0, &[]);
    assert_eq!(assert_same("bad_format", &bad_fmt.buf).ret, -3);
}
