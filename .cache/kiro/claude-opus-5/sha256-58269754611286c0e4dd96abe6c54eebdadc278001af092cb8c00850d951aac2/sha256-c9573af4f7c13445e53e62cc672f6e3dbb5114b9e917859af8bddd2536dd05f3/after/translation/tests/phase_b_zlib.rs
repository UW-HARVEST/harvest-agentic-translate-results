//! Phase B row C39 — real zlib-produced raw DEFLATE streams.
//!
//! Its own test binary: the C39 comparison must not run concurrently with the
//! table-mutation rows in `phase_b_tables.rs`, which temporarily rewrite the
//! exported `cp_len_base` / `cp_dist_base` globals of the shared `.so`.

mod common;

use common::*;

// ===========================================================================
// C39 — real zlib-produced raw DEFLATE streams
// ===========================================================================

#[test]
fn c39_zlib_corpus() {
    let p = load_pair();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/zlib_corpus.bin");
    let data = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("missing corpus {} ({e}); regenerate with tests/data/gen_corpus.py", path.display()));

    let mut off = 0usize;
    let rd32 = |d: &[u8], o: usize| -> usize {
        u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]]) as usize
    };
    let n = rd32(&data, off);
    off += 4;
    assert!(n > 0, "empty corpus");

    let mut mismatched_selfcheck = 0usize;
    for rec in 0..n {
        let raw_len = rd32(&data, off);
        let comp_len = rd32(&data, off + 4);
        off += 8;
        let raw = &data[off..off + raw_len];
        off += raw_len;
        let comp = &data[off..off + comp_len];
        off += comp_len;

        for align in 0..4usize {
            let out_bytes = raw_len + 16;
            let c = run_inflate(&p.c, comp, align, out_bytes, None);
            let r = run_inflate(&p.rust, comp, align, out_bytes, None);
            assert_eq!(
                c.ret, r.ret,
                "[C39 rec={rec} align={align}] ret\n C:{c:?}\n R:{r:?}"
            );
            assert_eq!(
                c.err, r.err,
                "[C39 rec={rec} align={align}] err\n C:{c:?}\n R:{r:?}"
            );
            assert_eq!(
                c.out, r.out,
                "[C39 rec={rec} align={align}] out mismatch\n C:{c:?}\n R:{r:?}"
            );
            // Independent check that the C really decompressed correctly
            // (only for streams the C accepts; stored blocks in the middle of a
            // stream are rejected by design, and cp_ptr misreads stored payloads
            // that live in the final partial word).
            if c.ret == 1 && &c.out[..raw_len] != raw {
                mismatched_selfcheck += 1;
            }
        }
    }
    println!(
        "C39: {n} records x 4 alignments compared; {mismatched_selfcheck} records where the C's \
         own output differs from zlib's (stored-block / cp_ptr quirk)"
    );
}
