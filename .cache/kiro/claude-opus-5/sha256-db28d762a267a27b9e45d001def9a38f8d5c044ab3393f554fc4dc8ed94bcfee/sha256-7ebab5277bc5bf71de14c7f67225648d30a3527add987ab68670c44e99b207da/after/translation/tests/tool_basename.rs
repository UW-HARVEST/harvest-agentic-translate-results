//! Differential tests for `tool_basename`, the only public entry point of the
//! library (see `c_src/include/lib.h`). Every call goes through the exported
//! symbol of a dynamically loaded shared object — both for C and for Rust — so
//! the `#[no_mangle]` wrapper is exercised as an external caller would use it.

mod common;

use common::{assert_same, call, libs};

#[test]
fn empty_and_separator_only_inputs() {
    for input in [
        &b""[..],
        b"/",
        b"\\",
        b"//",
        b"\\\\",
        b"/\\",
        b"\\/",
        b"///",
        b"\\\\\\",
        b"/\\/",
        b"\\/\\",
    ] {
        assert_same(input);
    }
}

#[test]
fn no_separator_present() {
    for input in [
        &b"file"[..],
        b"a",
        b" ",
        b"file.txt",
        b"..",
        b".",
        b"...hidden...",
        b"a_very_long_name_without_any_separator_at_all_0123456789",
    ] {
        assert_same(input);
    }
}

#[test]
fn forward_slash_only() {
    for input in [
        &b"/usr/bin/curl"[..],
        b"a/b",
        b"a/b/",
        b"/a",
        b"./relative/path",
        b"../../up/two",
        b"/trailing/slash/",
        b"//double//slashes//file",
        b"/a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/file.ext",
    ] {
        assert_same(input);
    }
}

#[test]
fn backslash_only() {
    for input in [
        &b"C:\\Windows\\System32\\cmd.exe"[..],
        b"a\\b",
        b"a\\b\\",
        b"\\a",
        b".\\relative\\path",
        b"..\\..\\up\\two",
        b"\\trailing\\slash\\",
        b"\\\\server\\share\\file",
        b"C:\\a\\b\\c\\d\\e\\f\\g\\h\\i\\file.ext",
    ] {
        assert_same(input);
    }
}

#[test]
fn both_separators_mixed() {
    // The C code picks whichever separator occurs *later* in the string, so
    // exercise both orderings and near-adjacent placements.
    for input in [
        &b"a/b\\c"[..],
        b"a\\b/c",
        b"/a\\b",
        b"\\a/b",
        b"/a\\",
        b"\\a/",
        b"C:\\dir/sub\\file.txt",
        b"C:/dir\\sub/file.txt",
        b"mixed/path\\with/both\\separators",
        b"mixed\\path/with\\both/separators",
        b"/////\\\\\\\\\\",
        b"\\\\\\\\\\/////",
        b"x/\\y",
        b"x\\/y",
    ] {
        assert_same(input);
    }
}

#[test]
fn high_bit_and_control_bytes() {
    // `char` is signed on the usual targets, so bytes >= 0x80 are negative in
    // C's `strrchr` comparison. Make sure the Rust port agrees, and that no
    // byte other than the two separators is ever treated as one.
    for b in 1u8..=255 {
        if b == b'/' || b == b'\\' {
            continue;
        }
        assert_same(&[b]);
        assert_same(&[b, b'/', b]);
        assert_same(&[b'/', b, b'\\', b]);
        assert_same(&[b, b, b'\\', b, b'/', b]);
    }
}

#[test]
fn every_byte_after_each_separator() {
    for b in 1u8..=255 {
        assert_same(&[b'/', b]);
        assert_same(&[b'\\', b]);
        assert_same(&[b, b'/']);
        assert_same(&[b, b'\\']);
    }
}

#[test]
fn separator_at_every_position() {
    // Sweep a single separator through a fixed-length buffer, then two
    // separators through all ordered position pairs.
    const N: usize = 12;
    for sep in [b'/', b'\\'] {
        for i in 0..N {
            let mut v = vec![b'x'; N];
            v[i] = sep;
            assert_same(&v);
        }
    }
    for i in 0..N {
        for j in 0..N {
            if i == j {
                continue;
            }
            let mut v = vec![b'x'; N];
            v[i] = b'/';
            v[j] = b'\\';
            assert_same(&v);
        }
    }
}

#[test]
fn exhaustive_short_strings_over_separator_alphabet() {
    // All strings of length 0..=6 over {'a', '/', '\\'} — this covers every
    // possible ordering and multiplicity of the two separators for short input.
    let alphabet = [b'a', b'/', b'\\'];
    for len in 0..=6usize {
        let total = alphabet.len().pow(len as u32);
        for mut n in 0..total {
            let mut v = Vec::with_capacity(len);
            for _ in 0..len {
                v.push(alphabet[n % alphabet.len()]);
                n /= alphabet.len();
            }
            assert_same(&v);
        }
    }
}

#[test]
fn long_inputs() {
    for len in [255usize, 256, 1023, 1024, 4096, 65536] {
        let mut v = vec![b'q'; len];
        assert_same(&v);

        v[0] = b'/';
        assert_same(&v);

        v[0] = b'q';
        v[len - 1] = b'/';
        assert_same(&v);

        v[len - 1] = b'\\';
        assert_same(&v);

        v[len / 2] = b'/';
        v[len - 1] = b'\\';
        assert_same(&v);

        v[len / 2] = b'\\';
        v[len - 1] = b'/';
        assert_same(&v);
    }

    // Deeply nested path, alternating separators.
    let mut deep = Vec::new();
    for i in 0..1000 {
        deep.push(if i % 2 == 0 { b'/' } else { b'\\' });
        deep.extend_from_slice(b"seg");
    }
    assert_same(&deep);
    deep.extend_from_slice(b"/");
    assert_same(&deep);
}

#[test]
fn input_buffer_is_not_modified() {
    let l = libs();
    for input in [&b"/usr/bin/tool"[..], b"C:\\x\\y", b"plain", b"", b"/\\/\\"] {
        let c = call(l.c_tool_basename, input);
        let r = call(l.rust_tool_basename, input);

        let mut expected = input.to_vec();
        expected.push(0);
        assert_eq!(c.buffer_after, expected, "C mutated its input {input:?}");
        assert_eq!(r.buffer_after, expected, "Rust mutated its input {input:?}");
    }
}

#[test]
fn returned_pointer_aliases_the_input_buffer() {
    // The contract is that the result points *into* the caller's buffer; verify
    // the reported offset and tail agree with each other for both libraries.
    let l = libs();
    for input in [
        &b"/usr/local/bin/x"[..],
        b"C:\\Program Files\\app.exe",
        b"no-sep",
        b"trailing/",
        b"trailing\\",
    ] {
        for f in [l.c_tool_basename, l.rust_tool_basename] {
            let res = call(f, input);
            assert_eq!(&input[res.offset as usize..], &res.tail[..]);
        }
    }
}
