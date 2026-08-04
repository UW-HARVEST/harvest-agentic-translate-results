use libpsbt::base64::{base62_encode, base64_decode, base64_encode};

#[test]
fn test_base64_encode_empty() {
    let mut out = [0u8; 16];
    let n = base64_encode(b"", &mut out).expect("encode empty");
    assert_eq!(n, 0);
    assert_eq!(out[0], 0); // nul terminator
}

#[test]
fn test_base64_encode_one_byte() {
    let mut out = [0u8; 16];
    let n = base64_encode(b"f", &mut out).expect("encode f");
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"Zg==");
    assert_eq!(out[n], 0);
}

#[test]
fn test_base64_encode_two_bytes() {
    let mut out = [0u8; 16];
    let n = base64_encode(b"fo", &mut out).expect("encode fo");
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"Zm8=");
}

#[test]
fn test_base64_encode_three_bytes() {
    let mut out = [0u8; 16];
    let n = base64_encode(b"foo", &mut out).expect("encode foo");
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"Zm9v");
}

#[test]
fn test_base64_encode_four_bytes() {
    let mut out = [0u8; 16];
    let n = base64_encode(b"foob", &mut out).expect("encode foob");
    assert_eq!(n, 8);
    assert_eq!(&out[..n], b"Zm9vYg==");
}

#[test]
fn test_base64_encode_five_bytes() {
    let mut out = [0u8; 16];
    let n = base64_encode(b"fooba", &mut out).expect("encode fooba");
    assert_eq!(n, 8);
    assert_eq!(&out[..n], b"Zm9vYmE=");
}

#[test]
fn test_base64_encode_six_bytes() {
    let mut out = [0u8; 16];
    let n = base64_encode(b"foobar", &mut out).expect("encode foobar");
    assert_eq!(n, 8);
    assert_eq!(&out[..n], b"Zm9vYmFy");
}

#[test]
fn test_base64_encode_hello_world() {
    let mut out = [0u8; 64];
    let n = base64_encode(b"Hello, World!", &mut out).expect("encode hello world");
    assert_eq!(n, 20);
    assert_eq!(&out[..n], b"SGVsbG8sIFdvcmxkIQ==");
}

#[test]
fn test_base64_encode_buffer_too_small() {
    let mut out = [0u8; 4];
    // "foo" -> "Zm9v" + nul = 5 bytes; capacity 4 should fail.
    assert_eq!(base64_encode(b"foo", &mut out), None);
}

#[test]
fn test_base64_decode_simple() {
    let mut out = [0u8; 16];
    let n = base64_decode(b"QUJD", &mut out).expect("decode QUJD");
    assert_eq!(n, 3);
    assert_eq!(&out[..n], b"ABC");
}

#[test]
fn test_base64_decode_one_pad() {
    let mut out = [0u8; 16];
    let n = base64_decode(b"QUI=", &mut out).expect("decode QUI=");
    assert_eq!(n, 2);
    assert_eq!(&out[..n], b"AB");
}

#[test]
fn test_base64_decode_two_pad() {
    let mut out = [0u8; 16];
    let n = base64_decode(b"QQ==", &mut out).expect("decode QQ==");
    assert_eq!(n, 1);
    assert_eq!(&out[..n], b"A");
}

#[test]
fn test_base64_decode_four_bytes() {
    let mut out = [0u8; 16];
    let n = base64_decode(b"QUJDRA==", &mut out).expect("decode QUJDRA==");
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"ABCD");
}

#[test]
fn test_base64_decode_hello_world() {
    let mut out = [0u8; 32];
    let n = base64_decode(b"SGVsbG8sIFdvcmxkIQ==", &mut out).expect("decode hello world");
    assert_eq!(n, 13);
    assert_eq!(&out[..n], b"Hello, World!");
}

#[test]
fn test_base64_decode_empty() {
    let mut out = [0u8; 16];
    assert_eq!(base64_decode(b"", &mut out), None);
}

#[test]
fn test_base64_decode_only_pads() {
    let mut out = [0u8; 16];
    assert_eq!(base64_decode(b"==", &mut out), None);
}

#[test]
fn test_base64_decode_non_multiple_of_4() {
    let mut out = [0u8; 16];
    // "ABC" has 3 valid chars (count=3 not multiple of 4) -> NULL
    assert_eq!(base64_decode(b"ABC", &mut out), None);
}

#[test]
fn test_base64_decode_skips_invalid() {
    let mut out = [0u8; 16];
    // base64-decode skips non-base64 chars; "abcd" alone -> 3 bytes
    let n = base64_decode(b"abcd", &mut out).expect("decode abcd");
    assert_eq!(n, 3);
    assert_eq!(out[0], 0x69);
    assert_eq!(out[1], 0xb7);
    assert_eq!(out[2], 0x1d);
}

#[test]
fn test_base64_roundtrip_random() {
    let inputs: &[&[u8]] = &[
        b"\x00",
        b"\x00\x01",
        b"\x00\x01\x02",
        b"\x00\x01\x02\x03",
        b"\xff\xfe\xfd",
        b"The quick brown fox jumps over the lazy dog",
    ];
    for &inp in inputs {
        let mut enc = [0u8; 256];
        let nenc = base64_encode(inp, &mut enc).expect("encode roundtrip");
        let mut dec = [0u8; 256];
        let ndec = base64_decode(&enc[..nenc], &mut dec).expect("decode roundtrip");
        assert_eq!(ndec, inp.len(), "decoded length mismatch");
        assert_eq!(&dec[..ndec], inp, "roundtrip mismatch");
    }
}

#[test]
fn test_base62_encode_empty() {
    let mut out = [0u8; 16];
    let n = base62_encode(b"", &mut out).expect("encode empty");
    assert_eq!(n, 0);
    assert_eq!(out[0], 0);
}

#[test]
fn test_base62_encode_one_byte_zero() {
    // C said: input 0x00 -> "00==" (4 bytes)
    let mut out = [0u8; 16];
    let n = base62_encode(b"\x00", &mut out).expect("encode 0x00");
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"00==");
}

#[test]
fn test_base62_encode_three_bytes_abc() {
    // C said: input 0x41,0x42,0x43 -> "GK93" (4 bytes)
    let mut out = [0u8; 16];
    let n = base62_encode(b"\x41\x42\x43", &mut out).expect("encode ABC");
    assert_eq!(n, 4);
    assert_eq!(&out[..n], b"GK93");
}

#[test]
fn test_base62_encode_four_bytes() {
    // C said: input 0x00,0x01,0x02,0x03 -> "00420m==" (8 bytes)
    let mut out = [0u8; 16];
    let n = base62_encode(b"\x00\x01\x02\x03", &mut out).expect("encode");
    assert_eq!(n, 8);
    assert_eq!(&out[..n], b"00420m==");
}

// NOTE: Base62 encoding is fundamentally broken: any byte whose 6-bit slice
// is >= 62 reads off the end of the 62-byte table. The C code reads OOB
// (undefined behavior). The Rust port correctly returns a panic/error on
// such inputs. We restrict tests to inputs whose 6-bit indices stay < 62.

fn main() {}
