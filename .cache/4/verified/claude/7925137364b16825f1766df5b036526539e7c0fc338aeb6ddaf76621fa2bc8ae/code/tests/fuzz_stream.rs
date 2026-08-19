// Randomized stream fuzzing (Phases B and C combined).
//
// A grammar-based generator builds nasty stdin streams out of the fragments the
// C code actually distinguishes — valid tokens, signs without digits, junk
// bytes, every isspace() byte, overflow-sized digit runs, truncation at EOF —
// and both binaries are compared byte-for-byte.  Fixed seeds keep every failure
// reproducible.

mod common;

use common::*;

/// One random fragment of an input stream.
fn fragment(rng: &mut Rng) -> Vec<u8> {
    match rng.below(16) {
        // A plain decimal token, sometimes signed, sometimes zero-padded.
        0 | 1 | 2 | 3 => {
            let sign = *rng.choose(&["", "+", "-"]);
            let pad = "0".repeat(rng.below(4) as usize);
            let v = match rng.below(4) {
                0 => rng.below(4).to_string(),          // near the magic values
                1 => rng.below(1000).to_string(),
                2 => (rng.next_i32() as i64).abs().to_string(),
                _ => rng.below(u64::MAX).to_string(),
            };
            format!("{sign}{pad}{v}").into_bytes()
        }
        // Exactly the magic constants, to reach the deeper stages often.
        4 | 5 => (*rng.choose(&["1", "2", "3"])).as_bytes().to_vec(),
        // Whitespace run.
        6 | 7 | 8 => ws(rng, 3).into_bytes(),
        // A sign with no digits.
        9 => (*rng.choose(&["-", "+", "--", "+-", "-+"])).as_bytes().to_vec(),
        // Junk that must stop a conversion.
        10 => (*rng.choose(&[".", ",", "x", "abc", "e5", "0x", "/", ":", "'"]))
            .as_bytes()
            .to_vec(),
        // Arbitrary non-ASCII / control byte.
        11 => vec![rng.below(256) as u8],
        // Overflow-sized digit run.
        12 => {
            let n = 19 + rng.below(8) as usize;
            let mut s = String::new();
            if rng.below(2) == 0 {
                s.push('-');
            }
            for _ in 0..n {
                s.push((b'0' + rng.below(10) as u8) as char);
            }
            s.into_bytes()
        }
        // Values engineered to land on / near the magic constants after the
        // glibc narrowing, so the oracle actually observes value differences.
        13 => {
            let magic = 1 + rng.below(3) as i128;
            let k = 1 + rng.below(3) as u32;
            let base = 1i128 << (32 * k);
            let v = base * (1 + rng.below(3) as i128) + magic;
            if rng.below(2) == 0 {
                v.to_string().into_bytes()
            } else {
                format!("-{}", base * (1 + rng.below(3) as i128) - magic).into_bytes()
            }
        }
        // A long zero-padded token.
        14 => {
            let n = rng.below(500) as usize;
            format!("{}{}", "0".repeat(n), rng.below(5)).into_bytes()
        }
        // Newlines / CRLF.
        _ => (*rng.choose(&["\n", "\r\n", "\n\n", "\r", "\t\n"]))
            .as_bytes()
            .to_vec(),
    }
}

fn stream(rng: &mut Rng) -> Vec<u8> {
    let n = rng.below(8) as usize;
    let mut out = Vec::new();
    for _ in 0..n {
        out.extend_from_slice(&fragment(rng));
    }
    // Truncate at a random point every so often, to hit EOF mid-token.
    if rng.below(4) == 0 && !out.is_empty() {
        let keep = rng.below(out.len() as u64) as usize;
        out.truncate(keep);
    }
    out
}

#[test]
fn fuzz_random_streams() {
    let mut rng = Rng::new(0xF0F0);
    for _ in 0..1500 {
        let s = stream(&mut rng);
        assert_same(&s, "fuzz random stream");
    }
}

/// Same generator, but the stream is always prefixed by a valid "1 " so the
/// deeper stages (y, then z) are reached far more often.
#[test]
fn fuzz_streams_reaching_deeper_stages() {
    let mut rng = Rng::new(0xBEEF);
    for _ in 0..1500 {
        let mut s = b"1 ".to_vec();
        s.extend_from_slice(&stream(&mut rng));
        assert_same(&s, "fuzz deeper stages");
        let mut s2 = b"1 2 ".to_vec();
        s2.extend_from_slice(&stream(&mut rng));
        assert_same(&s2, "fuzz z stage");
    }
}

/// Pure random bytes — no structure at all.
#[test]
fn fuzz_arbitrary_bytes() {
    let mut rng = Rng::new(0xDEAD);
    for _ in 0..800 {
        let n = rng.below(24) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| rng.below(256) as u8).collect();
        assert_same(&bytes, "fuzz arbitrary bytes");
    }
    // Bias towards the bytes that matter: digits, signs, spaces, NUL.
    let alphabet: &[u8] = b"0123456789+- \t\n\r\x0b\x0c.,x\x00";
    for _ in 0..1500 {
        let n = rng.below(16) as usize;
        let bytes: Vec<u8> = (0..n).map(|_| *rng.choose(alphabet)).collect();
        assert_same(&bytes, "fuzz digit/sign alphabet");
    }
}

/// Every single-byte and two-byte input over the interesting alphabet, i.e. an
/// exhaustive sweep of the shortest streams (where EOF handling lives).
#[test]
fn exhaustive_short_streams() {
    let alphabet: &[u8] = b"0123456789+- \t\n\r\x0b\x0c.x\x00";
    for &a in alphabet {
        assert_same(&[a], "exhaustive 1-byte");
        for &b in alphabet {
            assert_same(&[a, b], "exhaustive 2-byte");
        }
    }
}
