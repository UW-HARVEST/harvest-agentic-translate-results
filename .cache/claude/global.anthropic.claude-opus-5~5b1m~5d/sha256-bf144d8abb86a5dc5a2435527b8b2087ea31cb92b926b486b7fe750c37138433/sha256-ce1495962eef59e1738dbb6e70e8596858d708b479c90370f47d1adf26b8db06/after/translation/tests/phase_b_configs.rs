//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test drives BOTH shared objects through `libloading` and compares the
//! returned interior pointer's offset, the returned string bytes, and the
//! (un)modified input buffer. Inputs are generated from a fixed seed so a
//! failure is always reproducible.

mod common;

use common::{assert_same, model, Rng};

/// Helper: build a body of `len` bytes that contains neither separator nor NUL.
fn plain_body(rng: &mut Rng, len: usize) -> Vec<u8> {
    (0..len).map(|_| rng.plain_byte()).collect()
}

/// Cross-check that the test's own expectation matches the C oracle too.
fn check(input: &[u8]) {
    let c = assert_same(input);
    assert_eq!(
        c.offset,
        model(input),
        "test model disagrees with the C oracle for {}",
        common::Pretty(input)
    );
}

// ---------------------------------------------------------------- C1
#[test]
fn c01_no_separator() {
    let mut rng = Rng::new(0xC001);
    for _ in 0..2_000 {
        let len = rng.range(0, 64);
        let body = plain_body(&mut rng, len);
        check(&body);
    }
}

// ---------------------------------------------------------------- C2
#[test]
fn c02_single_slash_interior() {
    let mut rng = Rng::new(0xC002);
    for _ in 0..2_000 {
        let len = rng.range(1, 64);
        let mut body = plain_body(&mut rng, len);
        let at = rng.below(len);
        body[at] = b'/';
        check(&body);
    }
}

// ---------------------------------------------------------------- C3
#[test]
fn c03_single_backslash_interior() {
    let mut rng = Rng::new(0xC003);
    for _ in 0..2_000 {
        let len = rng.range(1, 64);
        let mut body = plain_body(&mut rng, len);
        let at = rng.below(len);
        body[at] = b'\\';
        check(&body);
    }
}

// ---------------------------------------------------------------- C4
#[test]
fn c04_many_slashes() {
    let mut rng = Rng::new(0xC004);
    for _ in 0..2_000 {
        let len = rng.range(2, 96);
        let mut body = plain_body(&mut rng, len);
        let n = rng.range(2, 12);
        for _ in 0..n {
            let at = rng.below(len);
            body[at] = b'/';
        }
        check(&body);
    }
}

// ---------------------------------------------------------------- C5
#[test]
fn c05_many_backslashes() {
    let mut rng = Rng::new(0xC005);
    for _ in 0..2_000 {
        let len = rng.range(2, 96);
        let mut body = plain_body(&mut rng, len);
        let n = rng.range(2, 12);
        for _ in 0..n {
            let at = rng.below(len);
            body[at] = b'\\';
        }
        check(&body);
    }
}

// ---------------------------------------------------------------- C6
#[test]
fn c06_both_slash_after_backslash() {
    // Forces the TRUE arm of `(s1 > s2) ? s1 + 1 : s2 + 1`.
    let mut rng = Rng::new(0xC006);
    for _ in 0..2_000 {
        let len = rng.range(2, 96);
        let mut body = plain_body(&mut rng, len);
        // last '\\' strictly before last '/'
        let bs = rng.below(len - 1);
        let sl = rng.range(bs + 1, len - 1);
        // extra earlier separators (strictly before `bs`) must not change the
        // answer; place them first so they cannot clobber `bs`/`sl`.
        for _ in 0..rng.below(4) {
            if bs > 0 {
                let at = rng.below(bs);
                body[at] = if rng.bool() { b'/' } else { b'\\' };
            }
        }
        body[bs] = b'\\';
        body[sl] = b'/';
        // keep the invariant: nothing after `sl` is a separator
        for b in body.iter_mut().skip(sl + 1) {
            if *b == b'/' || *b == b'\\' {
                *b = b'q';
            }
        }
        // ...and nothing between bs and sl is a '\\'
        for b in body.iter_mut().take(sl).skip(bs + 1) {
            if *b == b'\\' {
                *b = b'q';
            }
        }
        let s1 = body.iter().rposition(|&b| b == b'/').unwrap();
        let s2 = body.iter().rposition(|&b| b == b'\\').unwrap();
        assert!(s1 > s2, "row C6 setup invariant");
        check(&body);
    }
}

// ---------------------------------------------------------------- C7
#[test]
fn c07_both_backslash_after_slash() {
    // Forces the FALSE arm of the ternary.
    let mut rng = Rng::new(0xC007);
    for _ in 0..2_000 {
        let len = rng.range(2, 96);
        let mut body = plain_body(&mut rng, len);
        let sl = rng.below(len - 1);
        let bs = rng.range(sl + 1, len - 1);
        for _ in 0..rng.below(4) {
            if sl > 0 {
                let at = rng.below(sl);
                body[at] = if rng.bool() { b'/' } else { b'\\' };
            }
        }
        body[sl] = b'/';
        body[bs] = b'\\';
        for b in body.iter_mut().skip(bs + 1) {
            if *b == b'/' || *b == b'\\' {
                *b = b'q';
            }
        }
        for b in body.iter_mut().take(bs).skip(sl + 1) {
            if *b == b'/' {
                *b = b'q';
            }
        }
        let s1 = body.iter().rposition(|&b| b == b'/').unwrap();
        let s2 = body.iter().rposition(|&b| b == b'\\').unwrap();
        assert!(s2 > s1, "row C7 setup invariant");
        check(&body);
    }
}

// ---------------------------------------------------------------- C8
#[test]
fn c08_both_many_random_order() {
    let mut rng = Rng::new(0xC008);
    let mut saw_slash_last = false;
    let mut saw_backslash_last = false;
    for _ in 0..4_000 {
        let len = rng.range(2, 128);
        let mut body = plain_body(&mut rng, len);
        let n = rng.range(2, 20);
        for _ in 0..n {
            let at = rng.below(len);
            body[at] = if rng.bool() { b'/' } else { b'\\' };
        }
        if let (Some(a), Some(b)) = (
            body.iter().rposition(|&x| x == b'/'),
            body.iter().rposition(|&x| x == b'\\'),
        ) {
            if a > b {
                saw_slash_last = true;
            } else {
                saw_backslash_last = true;
            }
        }
        check(&body);
    }
    assert!(
        saw_slash_last && saw_backslash_last,
        "row C8 must reach both arms of the ternary"
    );
}

// ---------------------------------------------------------------- C9
#[test]
fn c09_separator_first_byte() {
    let mut rng = Rng::new(0xC009);
    for _ in 0..2_000 {
        let len = rng.range(1, 64);
        let mut body = plain_body(&mut rng, len);
        body[0] = if rng.bool() { b'/' } else { b'\\' };
        check(&body);
    }
}

// ---------------------------------------------------------------- C10
#[test]
fn c10_separator_last_byte() {
    // Empty basename: the returned pointer is the NUL terminator itself.
    let mut rng = Rng::new(0xC010);
    for _ in 0..2_000 {
        let len = rng.range(1, 64);
        let mut body = plain_body(&mut rng, len);
        body[len - 1] = if rng.bool() { b'/' } else { b'\\' };
        let out = assert_same(&body);
        assert_eq!(out.offset, len as isize, "trailing separator offset");
        assert!(out.result.is_empty(), "trailing separator must yield \"\"");
        assert_eq!(out.offset, model(&body));
    }
}

// ---------------------------------------------------------------- C11
#[test]
fn c11_adjacent_separator_runs() {
    let mut rng = Rng::new(0xC011);
    const RUNS: [&[u8]; 6] = [b"//", b"\\\\", b"/\\", b"\\/", b"///\\", b"\\\\//"];
    for _ in 0..2_000 {
        let head_len = rng.range(0, 24);
        let head = plain_body(&mut rng, head_len);
        let tail_len = rng.range(0, 24);
        let tail = plain_body(&mut rng, tail_len);
        let mut run: Vec<u8> = rng.pick(&RUNS).to_vec();
        // sometimes lengthen the run
        for _ in 0..rng.below(4) {
            run.push(if rng.bool() { b'/' } else { b'\\' });
        }
        let mut body = head;
        body.extend_from_slice(&run);
        body.extend_from_slice(&tail);
        check(&body);
    }
}

// ---------------------------------------------------------------- C12
#[test]
fn c12_tiny_lengths_exhaustive() {
    // length 0
    check(b"");
    // every possible single non-NUL byte
    for b in 1u16..=255 {
        check(&[b as u8]);
    }
    // and both separators explicitly, plus every 2-byte separator combination
    for a in [b'/', b'\\', b'a', 0xFFu8] {
        for b in [b'/', b'\\', b'b', 0x80u8] {
            check(&[a, b]);
        }
    }
}

// ---------------------------------------------------------------- C13
#[test]
fn c13_large_buffers() {
    let mut rng = Rng::new(0xC013);
    const SIZES: [usize; 4] = [64 * 1024, 256 * 1024, 700 * 1024, 1024 * 1024];
    for &len in &SIZES {
        // no separator anywhere in a large buffer
        let body = plain_body(&mut rng, len);
        check(&body);

        for _ in 0..3 {
            // separator at a random far-out offset
            let mut b2 = body.clone();
            let at = rng.range(len / 2, len - 1);
            b2[at] = b'/';
            check(&b2);

            let mut b3 = body.clone();
            let at = rng.range(len / 2, len - 1);
            b3[at] = b'\\';
            check(&b3);

            // both, far apart, random which is last
            let mut b4 = body.clone();
            let x = rng.range(len / 4, len - 2);
            let y = rng.range(x + 1, len - 1);
            if rng.bool() {
                b4[x] = b'\\';
                b4[y] = b'/';
            } else {
                b4[x] = b'/';
                b4[y] = b'\\';
            }
            check(&b4);

            // separator as the very last byte of a huge buffer
            let mut b5 = body.clone();
            b5[len - 1] = if rng.bool() { b'/' } else { b'\\' };
            check(&b5);
        }
    }
}

// ---------------------------------------------------------------- C14
#[test]
fn c14_separator_neighbour_bytes() {
    // 0x2E '.' and 0x30 '0' bracket '/' (0x2F); 0x5B '[' and 0x5D ']' bracket
    // '\\' (0x5C). A one-off in a byte comparison shows up here.
    const NEIGHBOURS: [u8; 4] = [0x2E, 0x30, 0x5B, 0x5D];
    let mut rng = Rng::new(0xC014);
    for _ in 0..3_000 {
        let len = rng.range(0, 48);
        let mut body: Vec<u8> = (0..len).map(|_| rng.pick(&NEIGHBOURS)).collect();
        // occasionally sprinkle a real separator so the row also covers the mix
        for _ in 0..rng.below(3) {
            if len > 0 {
                let at = rng.below(len);
                body[at] = if rng.bool() { b'/' } else { b'\\' };
            }
        }
        check(&body);
    }
}

// ---------------------------------------------------------------- C15
#[test]
fn c15_high_bit_bytes() {
    // `char` is signed on x86-64; `strrchr` compares as unsigned char. 0xAF and
    // 0xDC are '/'|0x80 and '\\'|0x80 respectively.
    let mut rng = Rng::new(0xC015);
    for _ in 0..3_000 {
        let len = rng.range(0, 48);
        let mut body: Vec<u8> = (0..len).map(|_| 0x80 | (rng.byte() & 0x7F)).collect();
        if len > 0 && rng.bool() {
            let at = rng.below(len);
            body[at] = if rng.bool() { 0xAF } else { 0xDC };
        }
        for _ in 0..rng.below(3) {
            if len > 0 {
                let at = rng.below(len);
                body[at] = if rng.bool() { b'/' } else { b'\\' };
            }
        }
        check(&body);
    }
    // explicit: the sign-extension traps on their own
    check(&[0xAF]);
    check(&[0xDC]);
    check(b"dir\xAFfile");
    check(b"dir\xDCfile");
    check(b"a\xAF/b");
    check(b"a\xDC\\b");
}

// ---------------------------------------------------------------- C16
#[test]
fn c16_arbitrary_bytes_fuzz() {
    let mut rng = Rng::new(0xC016);
    for _ in 0..5_000 {
        let len = rng.range(0, 80);
        // every byte except NUL; separators arise naturally
        let body: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.byte();
                if b != 0 {
                    return b;
                }
            })
            .collect();
        check(&body);
    }
}

// ---------------------------------------------------------------- C17
#[test]
fn c17_realistic_path_corpus() {
    const FIXED: &[&[u8]] = &[
        b"/usr/bin/tool",
        b"/usr/bin/",
        b"tool",
        b"./tool",
        b"../../tool",
        b"C:\\Windows\\System32\\cmd.exe",
        b"C:\\",
        b"C:/dir\\tool",
        b"C:\\dir/tool",
        b"\\\\host\\share\\file.txt",
        b"//server/share/file",
        b".hidden",
        b"/.hidden",
        b"\\.hidden",
        b"dir/.",
        b"dir\\..",
        b"a/b\\c/d\\e",
        b"e\\d/c\\b/a",
        b"name.with.dots.tar.gz",
        b"/",
        b"\\",
        b"//",
        b"\\\\",
        b"/\\",
        b"\\/",
        b"trailing.dot.",
        b" /leading space",
        b"weird\tname/\ttab",
    ];
    for f in FIXED {
        check(f);
    }

    let mut rng = Rng::new(0xC017);
    const PREFIXES: [&[u8]; 6] = [b"", b"/", b"C:\\", b"\\\\host\\", b"./", b"../"];
    for _ in 0..4_000 {
        let mut body: Vec<u8> = rng.pick(&PREFIXES).to_vec();
        let comps = rng.range(1, 8);
        for i in 0..comps {
            if i > 0 {
                // mixed separator styles within one path, sometimes doubled
                let seps: &[u8] = match rng.below(4) {
                    0 => b"/",
                    1 => b"\\",
                    2 => b"/\\",
                    _ => b"\\/",
                };
                body.extend_from_slice(seps);
            }
            let clen = rng.range(0, 10);
            for _ in 0..clen {
                body.push(rng.pick(&[
                    b'a', b'b', b'z', b'A', b'Z', b'0', b'9', b'.', b'-', b'_', b' ', 0xE2,
                ]));
            }
        }
        if rng.below(4) == 0 {
            body.push(if rng.bool() { b'/' } else { b'\\' });
        }
        check(&body);
    }
}

// ---------------------------------------------------------------- C18
#[test]
fn c18_repeated_application() {
    // Feed each implementation's own output back into itself, the way a real
    // consumer composes calls. Divergence in composition is invisible to
    // single-call rows.
    let mut rng = Rng::new(0xC018);
    for _ in 0..2_000 {
        let len = rng.range(0, 64);
        let mut body = plain_body(&mut rng, len);
        for _ in 0..rng.below(6) {
            if len > 0 {
                let at = rng.below(len);
                body[at] = if rng.bool() { b'/' } else { b'\\' };
            }
        }

        let mut current = body.clone();
        for round in 0..3 {
            let out = assert_same(&current);
            assert_eq!(
                out.offset,
                model(&current),
                "composition round {round} diverged from model"
            );
            current = out.result;
        }
    }
}

// ---------------------------------------------------------------- C19
#[test]
fn c19_call_order_and_no_mutation() {
    // Same buffer handed to C, then Rust, then C again: the answer must be
    // stable and the buffer untouched, in that exact order.
    use std::ffi::{c_char, CStr};

    let c = common::c_driver();
    let r = common::rust_driver();
    let mut rng = Rng::new(0xC019);

    for _ in 0..2_000 {
        let len = rng.range(0, 64);
        let mut body = plain_body(&mut rng, len);
        for _ in 0..rng.below(5) {
            if len > 0 {
                let at = rng.below(len);
                body[at] = if rng.bool() { b'/' } else { b'\\' };
            }
        }
        let pristine = {
            let mut v = body.clone();
            v.push(0);
            v
        };

        let mut buf = pristine.clone();
        let base = buf.as_mut_ptr() as *mut c_char;

        let o1 = unsafe { (c.tool_basename)(base).offset_from(base) };
        let o2 = unsafe { (r.tool_basename)(base).offset_from(base) };
        let o3 = unsafe { (c.tool_basename)(base).offset_from(base) };
        let s2 = unsafe { CStr::from_ptr(base.offset(o2)) }.to_bytes().to_vec();

        assert_eq!(o1, o2, "C then Rust on the SAME buffer diverged");
        assert_eq!(o1, o3, "C was not stable across an interleaved Rust call");
        assert_eq!(buf, pristine, "a call mutated the shared buffer");
        assert_eq!(o1, model(&body));
        assert_eq!(s2, body[o1 as usize..], "returned slice mismatch");
    }
}

// ---------------------------------------------------------------- C20
#[test]
fn c20_basename_length_zero_and_one() {
    // Interior-pointer arithmetic at the extremes: basename of length 1 and of
    // length 0, swept exhaustively over lengths 2..=64 for both separators.
    for len in 2..=64usize {
        for &sep in &[b'/', b'\\'] {
            // basename exactly 1 byte
            let mut b = vec![b'x'; len];
            b[len - 2] = sep;
            let out = assert_same(&b);
            assert_eq!(out.offset, (len - 1) as isize);
            assert_eq!(out.result, b"x");

            // basename empty
            let mut b = vec![b'x'; len];
            b[len - 1] = sep;
            let out = assert_same(&b);
            assert_eq!(out.offset, len as isize);
            assert!(out.result.is_empty());

            // separator at index 0 -> basename is the whole tail
            let mut b = vec![b'x'; len];
            b[0] = sep;
            let out = assert_same(&b);
            assert_eq!(out.offset, 1);
            assert_eq!(out.result.len(), len - 1);
        }
    }
}
