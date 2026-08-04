use aes128_SIMD::padding::{pad_buffer, remove_padding};

#[test]
fn test_pad_5_bytes() {
    let input = [0x01u8, 0x02, 0x03, 0x04, 0x05];
    let mut output: Vec<u8> = Vec::new();
    let mut output_len: usize = 0;
    pad_buffer(&input, 5, &mut output, &mut output_len);
    assert_eq!(output_len, 16);
    assert_eq!(output.len(), 16);
    let expected: [u8; 16] = [
        0x01, 0x02, 0x03, 0x04, 0x05,
        0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B,
    ];
    assert_eq!(output.as_slice(), &expected);
}

#[test]
fn test_pad_full_block() {
    // When input length is exactly a block multiple, an entire extra block of
    // 0x10 padding bytes is appended.
    let input = [0xAAu8; 16];
    let mut output: Vec<u8> = Vec::new();
    let mut output_len: usize = 0;
    pad_buffer(&input, 16, &mut output, &mut output_len);
    assert_eq!(output_len, 32);
    assert_eq!(output.len(), 32);
    let mut expected = [0u8; 32];
    for v in expected.iter_mut().take(16) {
        *v = 0xAA;
    }
    for v in expected.iter_mut().skip(16) {
        *v = 0x10;
    }
    assert_eq!(output.as_slice(), &expected);
}

#[test]
fn test_pad_20_bytes() {
    let mut input = [0u8; 20];
    for (i, b) in input.iter_mut().enumerate() {
        *b = 0x10 + i as u8;
    }
    let mut output: Vec<u8> = Vec::new();
    let mut output_len: usize = 0;
    pad_buffer(&input, 20, &mut output, &mut output_len);
    assert_eq!(output_len, 32);
    let expected: [u8; 32] = [
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
        0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
        0x20, 0x21, 0x22, 0x23,
        0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C,
    ];
    assert_eq!(output.as_slice(), &expected);
}

#[test]
fn test_pad_zero_length() {
    // Input length 0 => output a single block of 16 0x10 bytes.
    let input: [u8; 0] = [];
    let mut output: Vec<u8> = Vec::new();
    let mut output_len: usize = 0;
    pad_buffer(&input, 0, &mut output, &mut output_len);
    assert_eq!(output_len, 16);
    let expected: [u8; 16] = [0x10; 16];
    assert_eq!(output.as_slice(), &expected);
}

#[test]
fn test_remove_padding_5_bytes() {
    let mut buf = [0u8; 16];
    for v in buf.iter_mut().take(11) {
        *v = 0xAA;
    }
    for v in buf.iter_mut().skip(11) {
        *v = 5;
    }
    assert_eq!(remove_padding(&buf, 16), 11);
}

#[test]
fn test_remove_padding_full_block() {
    let mut buf = [0u8; 32];
    for v in buf.iter_mut().take(16) {
        *v = 0x42;
    }
    for v in buf.iter_mut().skip(16) {
        *v = 16;
    }
    assert_eq!(remove_padding(&buf, 32), 16);
}

#[test]
fn test_remove_padding_invalid_zero() {
    let mut buf = [0xABu8; 16];
    buf[15] = 0;
    // pad value 0 is invalid; original length is returned.
    assert_eq!(remove_padding(&buf, 16), 16);
}

#[test]
fn test_remove_padding_invalid_too_large() {
    let mut buf = [0xABu8; 16];
    buf[15] = 17;
    // pad value > 16 is invalid; original length returned.
    assert_eq!(remove_padding(&buf, 16), 16);
}

#[test]
fn test_remove_padding_inconsistent() {
    let mut buf = [0xCCu8; 16];
    buf[15] = 4; // claims 4 bytes of padding but other bytes are 0xCC
    assert_eq!(remove_padding(&buf, 16), 16);
}

#[test]
fn test_remove_padding_zero_len() {
    let buf: [u8; 0] = [];
    assert_eq!(remove_padding(&buf, 0), 0);
}

#[test]
fn test_pad_remove_roundtrip() {
    let input = b"Hello, World!";
    let mut padded: Vec<u8> = Vec::new();
    let mut padded_len: usize = 0;
    pad_buffer(input, input.len(), &mut padded, &mut padded_len);
    // padded_len is a multiple of 16 and > input.len()
    assert_eq!(padded_len % 16, 0);
    assert!(padded_len > input.len());
    let removed = remove_padding(&padded, padded_len);
    assert_eq!(removed, input.len());
    assert_eq!(&padded[..removed], input);
}

fn main() {}
