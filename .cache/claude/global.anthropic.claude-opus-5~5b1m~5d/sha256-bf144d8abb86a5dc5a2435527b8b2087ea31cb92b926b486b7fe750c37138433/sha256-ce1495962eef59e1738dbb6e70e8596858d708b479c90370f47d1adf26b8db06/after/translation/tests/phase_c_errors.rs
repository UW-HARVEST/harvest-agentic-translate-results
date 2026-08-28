//! Phase C — error-path differential tests, one test per `ERRORS.md` row
//! (E1 lives in `phase_c_null_ptr.rs` because observing UB needs a subprocess).
//!
//! `tool_basename` has no error return: the C has exactly one `return`
//! statement, no `assert`, no NULL check and no error enum. So each row here
//! pins down a *rejection-shaped* condition — the fall-through of the
//! `if/else if/else if` chain, the degenerate inputs, and the generic FFI
//! boundaries — and asserts the two implementations agree on the exact
//! sentinel (offset + string), not merely that "both did something".

mod common;

use common::{assert_same, assert_same_with_slack, model, Pretty, Rng};

/// Assert both implementations agree AND that the shared expectation holds.
fn expect(input: &[u8], want_offset: isize, want_result: &[u8]) {
    let out = assert_same(input);
    assert!(
        !out.was_null,
        "neither implementation may return NULL, but C did for {}",
        Pretty(input)
    );
    assert_eq!(
        out.offset,
        want_offset,
        "offset for {} — C/Rust agreed on {} but the C semantics require {}",
        Pretty(input),
        out.offset,
        want_offset
    );
    assert_eq!(out.result, want_result, "result string for {}", Pretty(input));
    assert_eq!(out.offset, model(input));
}

// ------------------------------------------------------------------ E2
#[test]
fn e2_empty_string() {
    // Both strrchr calls return NULL -> the whole if-chain is skipped and the
    // untouched `path` comes back. Offset 0, result "".
    expect(b"", 0, b"");
}

// ------------------------------------------------------------------ E3
#[test]
fn e3_no_separator_at_all() {
    for s in [
        &b"filename.txt"[..],
        b"a",
        b"no-separators-here",
        b"..",
        b".",
        b"C:file",
        b"\x01\x02\x03",
        b"\x7f\x80\xff",
    ] {
        expect(s, 0, s);
    }

    // randomized: a separator-free body always yields offset 0
    let mut rng = Rng::new(0xE003);
    for _ in 0..1_000 {
        let len = rng.range(0, 96);
        let body: Vec<u8> = (0..len).map(|_| rng.plain_byte()).collect();
        expect(&body, 0, &body);
    }
}

// ------------------------------------------------------------------ E4
#[test]
fn e4_trailing_separator_yields_empty() {
    // The returned pointer is the NUL terminator: valid, in-bounds, and NOT an
    // error sentinel.
    expect(b"dir/", 4, b"");
    expect(b"dir\\", 4, b"");
    expect(b"/usr/bin/", 9, b"");
    expect(b"C:\\Windows\\", 11, b"");
    expect(b"a/b/c/", 6, b"");
    expect(b"a\\b\\c\\", 6, b"");
    expect(b"mixed/dir\\", 10, b"");
    expect(b"mixed\\dir/", 10, b"");

    let mut rng = Rng::new(0xE004);
    for _ in 0..1_000 {
        let len = rng.range(1, 64);
        let mut body: Vec<u8> = (0..len).map(|_| rng.plain_byte()).collect();
        body[len - 1] = if rng.bool() { b'/' } else { b'\\' };
        expect(&body, len as isize, b"");
    }
}

// ------------------------------------------------------------------ E5
#[test]
fn e5_separator_only() {
    expect(b"/", 1, b"");
    expect(b"\\", 1, b"");
    // and runs made only of separators
    expect(b"//", 2, b"");
    expect(b"\\\\", 2, b"");
    expect(b"/\\", 2, b"");
    expect(b"\\/", 2, b"");
    expect(b"///", 3, b"");
    expect(b"\\\\\\", 3, b"");
    expect(b"/\\/\\/", 5, b"");
    for n in 1..=64usize {
        let a = vec![b'/'; n];
        expect(&a, n as isize, b"");
        let b = vec![b'\\'; n];
        expect(&b, n as isize, b"");
    }
}

// ------------------------------------------------------------------ E6
#[test]
fn e6_bytes_adjacent_to_separators() {
    // '.'=0x2E and '0'=0x30 bracket '/'=0x2F; '['=0x5B and ']'=0x5D bracket
    // '\\'=0x5C. None of them may be treated as a separator.
    for &b in &[0x2Eu8, 0x30, 0x5B, 0x5D] {
        expect(&[b], 0, &[b]);
        let s = vec![b; 16];
        expect(&s, 0, &s);
        // adjacent byte immediately before/after where a separator would be
        let mut t = b"abcXdef".to_vec();
        t[3] = b;
        expect(&t, 0, &t);
    }
    // sanity: swapping in the real separator DOES move the answer, so the test
    // above is actually discriminating.
    expect(b"abc/def", 4, b"def");
    expect(b"abc\\def", 4, b"def");
    expect(b"abc.def", 0, b"abc.def");
    expect(b"abc0def", 0, b"abc0def");
    expect(b"abc[def", 0, b"abc[def");
    expect(b"abc]def", 0, b"abc]def");

    let mut rng = Rng::new(0xE006);
    for _ in 0..1_000 {
        let len = rng.range(0, 48);
        let body: Vec<u8> = (0..len).map(|_| rng.pick(&[0x2Eu8, 0x30, 0x5B, 0x5D])).collect();
        expect(&body, 0, &body);
    }
}

// ------------------------------------------------------------------ E7
#[test]
fn e7_high_bit_bytes_are_not_separators() {
    // `char` is signed on x86-64. 0xAF == '/'|0x80 and 0xDC == '\\'|0x80: a
    // sign-extending or masking comparison bug would match them.
    for &b in &[0xAFu8, 0xDC, 0x80, 0xFE, 0xFF] {
        expect(&[b], 0, &[b]);
        let s = vec![b; 12];
        expect(&s, 0, &s);
    }
    expect(b"dir\xAFfile", 0, b"dir\xAFfile");
    expect(b"dir\xDCfile", 0, b"dir\xDCfile");
    // real separator after a high-bit byte still wins
    expect(b"\xAF/x", 2, b"x");
    expect(b"\xDC\\x", 2, b"x");
    // high-bit bytes after the real separator must not shift the answer
    expect(b"a/\xAF\xDC", 2, b"\xAF\xDC");

    // exhaustive sweep of every high-bit byte, alone and around a separator
    for b in 0x80u16..=0xFF {
        let b = b as u8;
        expect(&[b], 0, &[b]);
        let s = [b, b'/', b];
        expect(&s, 2, &[b]);
        let s = [b, b'\\', b];
        expect(&s, 2, &[b]);
    }
}

// ------------------------------------------------------------------ E8
#[test]
fn e8_oversized_input() {
    const N: usize = 1024 * 1024;

    // 1 MiB with no separator at all -> offset 0, no cap, no truncation
    let plain = vec![b'x'; N];
    expect(&plain, 0, &plain);

    // 1 MiB with the separator as the very last byte -> empty basename
    let mut trailing = vec![b'x'; N];
    trailing[N - 1] = b'/';
    expect(&trailing, N as isize, b"");
    let mut trailing = vec![b'x'; N];
    trailing[N - 1] = b'\\';
    expect(&trailing, N as isize, b"");

    // 1 MiB with the separator one byte from the end
    let mut near = vec![b'x'; N];
    near[N - 2] = b'/';
    expect(&near, (N - 1) as isize, b"x");

    // 1 MiB with the separator at the very front -> huge basename
    let mut front = vec![b'x'; N];
    front[0] = b'\\';
    let out = assert_same(&front);
    assert_eq!(out.offset, 1);
    assert_eq!(out.result.len(), N - 1);

    // both separators, far apart, in each order
    let mut both = vec![b'x'; N];
    both[N / 4] = b'\\';
    both[3 * N / 4] = b'/';
    expect(&both, (3 * N / 4 + 1) as isize, &vec![b'x'; N - 3 * N / 4 - 1]);
    let mut both = vec![b'x'; N];
    both[N / 4] = b'/';
    both[3 * N / 4] = b'\\';
    expect(&both, (3 * N / 4 + 1) as isize, &vec![b'x'; N - 3 * N / 4 - 1]);
}

// ------------------------------------------------------------------ E9
#[test]
fn e9_invalid_utf8_bytes() {
    // A port that routed through `str`/`from_utf8` would reject or panic here.
    const BAD: &[&[u8]] = &[
        b"\xff",
        b"\xfe\xff",
        b"\xc3",             // truncated 2-byte sequence
        b"\xe2\x82",         // truncated 3-byte sequence
        b"\xf0\x9f\x92",     // truncated 4-byte sequence
        b"\x80\x80\x80",     // lone continuation bytes
        b"\xed\xa0\x80",     // UTF-16 surrogate encoded as UTF-8 (invalid)
        b"\xc0\xaf",         // overlong encoding of '/'
        b"\xe0\x80\xaf",     // overlong encoding of '/'
        b"\xc1\x9c",         // overlong encoding of '\\'
    ];
    for s in BAD {
        expect(s, 0, s);

        // ...and the same bytes on both sides of each separator
        let mut with_slash = s.to_vec();
        with_slash.push(b'/');
        with_slash.extend_from_slice(s);
        expect(&with_slash, (s.len() + 1) as isize, s);

        let mut with_bs = s.to_vec();
        with_bs.push(b'\\');
        with_bs.extend_from_slice(s);
        expect(&with_bs, (s.len() + 1) as isize, s);
    }

    // overlong '/' encodings must NOT be decoded into a separator
    expect(b"a\xc0\xafb", 0, b"a\xc0\xafb");
    expect(b"a\xc1\x9cb", 0, b"a\xc1\x9cb");
}

// ------------------------------------------------------------------ E10
#[test]
fn e10_bytes_after_nul_are_invisible() {
    // The harness appends `slack` bytes of garbage (including both separators
    // and 0xFF) AFTER the NUL terminator. Both implementations must stop at the
    // terminator, so the answer must equal the no-slack answer exactly.
    let cases: &[&[u8]] = &[
        b"",
        b"a",
        b"nosep",
        b"dir/file",
        b"dir\\file",
        b"dir/",
        b"dir\\",
        b"/",
        b"\\",
    ];
    for s in cases {
        let baseline = assert_same(s);
        for slack in [1usize, 2, 3, 7, 16, 64] {
            let out = assert_same_with_slack(s, slack);
            assert_eq!(
                out.offset, baseline.offset,
                "bytes past the NUL changed the offset for {} (slack={slack})",
                Pretty(s)
            );
            assert_eq!(
                out.result, baseline.result,
                "bytes past the NUL changed the result for {} (slack={slack})",
                Pretty(s)
            );
        }
    }

    let mut rng = Rng::new(0xE010);
    for _ in 0..500 {
        let len = rng.range(0, 48);
        let mut body: Vec<u8> = (0..len).map(|_| rng.plain_byte()).collect();
        for _ in 0..rng.below(4) {
            if len > 0 {
                let at = rng.below(len);
                body[at] = if rng.bool() { b'/' } else { b'\\' };
            }
        }
        let baseline = assert_same(&body);
        let out = assert_same_with_slack(&body, rng.range(1, 32));
        assert_eq!(out.offset, baseline.offset);
        assert_eq!(out.result, baseline.result);
        assert_eq!(out.offset, model(&body));
    }
}

// ------------------------------------------------------- generic boundaries
#[test]
fn generic_no_enum_or_integer_parameter_exists() {
    // Phase C asks for out-of-range enum values across the FFI boundary. This
    // test documents mechanically that the ABI has no such parameter to abuse:
    // the sole export takes one pointer and returns one pointer.
    //
    // If the ABI ever grows an integer/enum parameter, this test's premise
    // breaks and the symbol-parity test in phase_d_symbols.rs will surface it.
    let c = common::c_driver();
    let r = common::rust_driver();
    assert_eq!(
        std::mem::size_of_val(&c.tool_basename),
        std::mem::size_of::<usize>()
    );
    assert_eq!(
        std::mem::size_of_val(&r.tool_basename),
        std::mem::size_of::<usize>()
    );
    // The C header is the ground truth for this claim; assert it is still what
    // we read it to be.
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/lib.h"),
    )
    .expect("c_src/include/lib.h must be readable");
    assert_eq!(
        header.trim(),
        "char *tool_basename(char *path);",
        "the public C ABI changed; ERRORS.md/CONFIGS.md must be re-derived"
    );
    assert!(
        !header.contains("enum") && !header.contains("int "),
        "public header gained an enum/int parameter: add out-of-range enum rows"
    );
}

#[test]
fn generic_never_returns_null_across_a_broad_corpus() {
    // The C's single `return path;` can never produce NULL for a non-NULL
    // argument. Pin that sentinel behaviour down for both implementations.
    let mut rng = Rng::new(0xBEEF);
    for _ in 0..3_000 {
        let len = rng.range(0, 64);
        let body: Vec<u8> = (0..len)
            .map(|_| loop {
                let b = rng.byte();
                if b != 0 {
                    return b;
                }
            })
            .collect();
        let out = assert_same(&body);
        assert!(!out.was_null, "returned NULL for {}", Pretty(&body));
        assert!(
            out.offset >= 0 && out.offset <= len as isize,
            "offset {} out of bounds for len {len}",
            out.offset
        );
    }
}

#[test]
fn generic_returned_pointer_is_always_in_bounds() {
    // The result must always be `path + k` with 0 <= k <= strlen(path), i.e. a
    // pointer into the caller's buffer (possibly one-past the last char, at the
    // NUL). Anything else would be a memory-safety divergence.
    let mut rng = Rng::new(0xF00D);
    for _ in 0..3_000 {
        let len = rng.range(0, 80);
        let mut body: Vec<u8> = (0..len).map(|_| rng.plain_byte()).collect();
        for _ in 0..rng.below(8) {
            if len > 0 {
                let at = rng.below(len);
                body[at] = if rng.bool() { b'/' } else { b'\\' };
            }
        }
        let out = assert_same(&body);
        assert!(out.offset >= 0, "negative offset {}", out.offset);
        assert!(
            out.offset <= body.len() as isize,
            "offset {} past the NUL for len {}",
            out.offset,
            body.len()
        );
        assert_eq!(
            out.result,
            &body[out.offset as usize..],
            "returned string is not the tail of the input at that offset"
        );
    }
}
