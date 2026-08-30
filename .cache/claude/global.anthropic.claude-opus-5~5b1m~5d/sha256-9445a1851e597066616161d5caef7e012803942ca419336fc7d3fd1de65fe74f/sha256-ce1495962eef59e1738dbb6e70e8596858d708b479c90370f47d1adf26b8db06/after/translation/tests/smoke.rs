mod harness;

use harness::make::*;
use harness::*;

/// A minimal end-to-end sanity check of the harness itself plus the two
/// most basic shapes: a valid PNG and a bad signature.
#[test]
fn smoke() {
    let pair = load_pair();
    let mut rng = Rng::new(0xC0FFEE);
    let mut cases = Vec::new();

    // valid 4x3 RGBA, filter 0
    let raw = raw_scanlines(&mut rng, 4, 3, 4, &[0]);
    let spec = PngSpec::new(4, 3, 6, deflate_literals(&raw));
    cases.push(Case::png("valid 4x3 rgba", spec.build()));

    // bad signature
    let mut bad = spec.build();
    bad[1] = b'X';
    cases.push(Case::png("bad signature", bad));

    // truncated
    cases.push(Case::png("empty", Vec::new()));

    let out = run_same(&pair, &cases);
    // the valid one must actually decode
    match &out[0] {
        Outcome::Ret(v) => {
            assert_eq!(i32::from_le_bytes(v[0..4].try_into().unwrap()), 4, "w");
            assert_eq!(i32::from_le_bytes(v[4..8].try_into().unwrap()), 3, "h");
            assert_eq!(v[8], 1, "pix must be non-null: {:?}", out[0]);
            // 4*3 pixels + "<null>" (cp_error_reason untouched on success)
            assert_eq!(&v[9 + 48..], b"<null>");
            // filter 0 => pixels are the raw bytes
            let mut expect = Vec::new();
            for y in 0..3 {
                expect.extend_from_slice(&raw[y * 17 + 1..y * 17 + 17]);
            }
            assert_eq!(&v[9..9 + 48], &expect[..]);
        }
        o => panic!("valid png did not decode: {o:?}"),
    }
    match &out[1] {
        Outcome::Ret(v) => assert!(
            String::from_utf8_lossy(v).contains("incorrect file signature"),
            "{:?}",
            out[1]
        ),
        o => panic!("bad signature: {o:?}"),
    }
}
