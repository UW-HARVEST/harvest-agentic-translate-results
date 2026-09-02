//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their
//! `tool_basename` exports (loaded with `libloading`) and asserts byte-identical
//! results across many fixed-seed randomized inputs.

mod common;

use common::{assert_same, plain, Rng, SEED};

const SEP: [u8; 2] = *b"/\\";

/// Insert `sep` at `n` distinct random positions inside `buf`.
fn sprinkle(rng: &mut Rng, buf: &mut Vec<u8>, sep: u8, n: usize) {
    for _ in 0..n {
        if buf.is_empty() {
            buf.push(sep);
        } else {
            let i = rng.below(buf.len());
            buf[i] = sep;
        }
    }
}

// ---------------------------------------------------------------- row 1
#[test]
fn row01_empty_string() {
    assert_same(b"", "row01 empty");
}

// ---------------------------------------------------------------- row 2
#[test]
fn row02_no_separator_ascii() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..5_000 {
        let len = rng.range(1, 64);
        let s = plain(&mut rng, len, true);
        assert_same(&s, "row02 no-sep ascii");
    }
}

// ---------------------------------------------------------------- row 3
#[test]
fn row03_no_separator_high_bytes() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..5_000 {
        let len = rng.range(1, 64);
        let mut s = plain(&mut rng, len, false);
        // Guarantee at least one byte >= 0x80 (signed c_char hazard).
        let i = rng.below(s.len());
        s[i] = rng.range(0x80, 0xFF) as u8;
        assert_same(&s, "row03 no-sep high bytes");
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn row04_only_slash_single_interior() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..5_000 {
        let len = rng.range(3, 64);
        let mut s = plain(&mut rng, len, false);
        let i = rng.range(1, len - 2); // strictly interior
        s[i] = b'/';
        assert_same(&s, "row04 single interior '/'");
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn row05_only_slash_many() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..5_000 {
        let len = rng.range(8, 96);
        let mut s = plain(&mut rng, len, false);
        let n = rng.range(2, 8);
        sprinkle(&mut rng, &mut s, b'/', n);
        assert_same(&s, "row05 many '/'");
    }
}

// ---------------------------------------------------------------- row 6
#[test]
fn row06_only_slash_at_index_zero() {
    let mut rng = Rng::new(SEED ^ 6);
    assert_same(b"/", "row06 lone '/'");
    for _ in 0..5_000 {
        let len = rng.range(1, 64);
        let mut s = plain(&mut rng, len, false);
        s.insert(0, b'/');
        assert_same(&s, "row06 leading '/'");
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn row07_only_slash_trailing() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..5_000 {
        let len = rng.range(1, 64);
        let mut s = plain(&mut rng, len, false);
        s.push(b'/');
        assert_same(&s, "row07 trailing '/' -> empty basename");
    }
    // Trailing run of slashes.
    for n in 1..=8 {
        let mut s = b"abc".to_vec();
        s.extend(std::iter::repeat_n(b'/', n));
        assert_same(&s, "row07 trailing '/' run");
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn row08_only_backslash_single_interior() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..5_000 {
        let len = rng.range(3, 64);
        let mut s = plain(&mut rng, len, false);
        let i = rng.range(1, len - 2);
        s[i] = b'\\';
        assert_same(&s, "row08 single interior '\\'");
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn row09_only_backslash_many() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..5_000 {
        let len = rng.range(8, 96);
        let mut s = plain(&mut rng, len, false);
        let n = rng.range(2, 8);
        sprinkle(&mut rng, &mut s, b'\\', n);
        assert_same(&s, "row09 many '\\'");
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn row10_only_backslash_edges() {
    let mut rng = Rng::new(SEED ^ 10);
    assert_same(b"\\", "row10 lone '\\'");
    for _ in 0..2_500 {
        let len = rng.range(1, 64);
        let mut s = plain(&mut rng, len, false);
        s.insert(0, b'\\');
        assert_same(&s, "row10 leading '\\'");
    }
    for _ in 0..2_500 {
        let len = rng.range(1, 64);
        let mut s = plain(&mut rng, len, false);
        s.push(b'\\');
        assert_same(&s, "row10 trailing '\\'");
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn row11_both_slash_last() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..5_000 {
        let len = rng.range(6, 96);
        let mut s = plain(&mut rng, len, false);
        // last '\' strictly before last '/'
        let i_slash = rng.range(2, len - 1);
        let i_back = rng.below(i_slash);
        s[i_back] = b'\\';
        s[i_slash] = b'/';
        assert_same(&s, "row11 both, s1 > s2");
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn row12_both_backslash_last() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..5_000 {
        let len = rng.range(6, 96);
        let mut s = plain(&mut rng, len, false);
        let i_back = rng.range(2, len - 1);
        let i_slash = rng.below(i_back);
        s[i_slash] = b'/';
        s[i_back] = b'\\';
        assert_same(&s, "row12 both, s1 < s2");
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn row13_decoy_bytes_one_off_the_separators() {
    // 0x2E '.', 0x30 '0' bracket '/'; 0x5B '[', 0x5D ']' bracket '\'.
    const DECOYS: [u8; 4] = [0x2E, 0x30, 0x5B, 0x5D];
    let mut rng = Rng::new(SEED ^ 13);

    // Pure decoys, no real separator: must return the whole string.
    for _ in 0..3_000 {
        let len = rng.range(1, 64);
        let s: Vec<u8> = (0..len).map(|_| DECOYS[rng.below(4)]).collect();
        assert_same(&s, "row13 decoys only");
    }
    // Decoys plus one real separator somewhere.
    for _ in 0..4_000 {
        let len = rng.range(2, 64);
        let mut s: Vec<u8> = (0..len).map(|_| DECOYS[rng.below(4)]).collect();
        let i = rng.below(len);
        s[i] = SEP[rng.below(2)];
        assert_same(&s, "row13 decoys + one separator");
    }
    // Hand-picked adjacency cases.
    for s in [
        &b".0[]"[..],
        b"a.b/c.d",
        b"a[b\\c]d",
        b"./0/[/]",
        b".\\0\\[\\]",
        b"0/.",
        b"]\\[",
    ] {
        assert_same(s, "row13 fixed adjacency");
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn row14_adjacent_and_all_separator_strings() {
    for s in [
        &b"/\\"[..],
        b"\\/",
        b"//",
        b"\\\\",
        b"//\\\\//",
        b"\\\\//\\\\",
        b"a/\\b",
        b"a\\/b",
        b"/\\/\\/\\",
        b"\\/\\/\\/",
        b"/a\\",
        b"\\a/",
    ] {
        assert_same(s, "row14 fixed adjacency");
    }
    // All-separator strings of every length up to 32, every bit pattern up to 12.
    let mut rng = Rng::new(SEED ^ 14);
    for len in 1..=32usize {
        for _ in 0..64 {
            let s: Vec<u8> = (0..len).map(|_| SEP[rng.below(2)]).collect();
            assert_same(&s, "row14 all separators");
        }
    }
    for len in 1..=12usize {
        for mask in 0u32..(1 << len) {
            let s: Vec<u8> = (0..len)
                .map(|i| if mask >> i & 1 == 1 { b'\\' } else { b'/' })
                .collect();
            assert_same(&s, "row14 exhaustive separator patterns");
        }
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn row15_fully_random_bytes() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..10_000 {
        let len = rng.below(257);
        let s: Vec<u8> = (0..len).map(|_| rng.any_byte()).collect();
        assert_same(&s, "row15 fully random");
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn row16_oversized_inputs() {
    let mut rng = Rng::new(SEED ^ 16);
    for &len in &[4095usize, 4096, 4097, 8192, 65_536, 1_048_576] {
        let base = plain(&mut rng, len, false);

        // no separator
        assert_same(&base, "row16 long, no separator");

        for &sep in &SEP {
            // separator at first byte
            let mut s = base.clone();
            s[0] = sep;
            assert_same(&s, "row16 long, separator first");

            // separator at last byte
            let mut s = base.clone();
            s[len - 1] = sep;
            assert_same(&s, "row16 long, separator last");

            // random interior separators
            let mut s = base.clone();
            for _ in 0..16 {
                let i = rng.below(len);
                s[i] = sep;
            }
            assert_same(&s, "row16 long, random separators");
        }

        // both kinds, random positions, several draws
        for _ in 0..8 {
            let mut s = base.clone();
            for _ in 0..32 {
                let i = rng.below(len);
                s[i] = SEP[rng.below(2)];
            }
            assert_same(&s, "row16 long, mixed separators");
        }
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn row17_all_single_byte_strings() {
    for b in 1u8..=255 {
        assert_same(&[b], "row17 length-1 exhaustive");
    }
}

// ---------------------------------------------------------------- row 18
// Row 18 (returned pointer stays inside the caller's buffer; input buffer left
// unmodified) is asserted inside `assert_same`, so it is covered by every test
// above. This test pins the invariant explicitly for the boundary shapes.
#[test]
fn row18_pointer_and_buffer_contract() {
    use common::{call, libs};
    let l = libs();
    for input in [
        &b""[..],
        b"/",
        b"\\",
        b"a",
        b"a/",
        b"a\\",
        b"/a",
        b"\\a",
        b"a/b\\c",
        b"a\\b/c",
    ] {
        let c = call(l.c_basename, input);
        let r = call(l.rust_basename, input);
        assert_eq!(c, r, "row18 divergence on {input:?}");
        let off = c.offset.expect("pointer must be inside the buffer") as usize;
        assert!(off <= input.len(), "offset {off} > len {}", input.len());
        assert_eq!(c.result, &input[off..]);
        assert_eq!(c.buf_after, input, "input buffer was mutated");
        assert_eq!(r.buf_after, input, "input buffer was mutated");
    }
}

// ---------------------------------------------------------------- extra
/// The classic documented shapes, as a readable regression net.
#[test]
fn known_paths_spotcheck() {
    for s in [
        &b"/usr/local/bin/tool"[..],
        b"C:\\Windows\\System32\\cmd.exe",
        b"C:/Windows\\System32/cmd.exe",
        b"relative/path/file.txt",
        b"file.txt",
        b".",
        b"..",
        b"/",
        b"//",
        b"/.",
        b"trailing/",
        b"trailing\\",
        b"\\\\server\\share\\file",
        b"mixed/sep\\here/end\\",
    ] {
        assert_same(s, "spotcheck");
    }
}
