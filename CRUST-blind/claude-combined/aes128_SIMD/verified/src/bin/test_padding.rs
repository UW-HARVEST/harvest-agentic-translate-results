use aes128_SIMD::padding;

#[test]
fn test_pad_buffer_partial_block() {
    let input = b"Hello";
    let mut output: Vec<u8> = Vec::new();
    let mut output_len: usize = 0;
    padding::pad_buffer(input, 5, &mut output, &mut output_len);
    assert_eq!(output_len, 16);
    assert_eq!(output.len(), 16);
    let expected = [
        0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B,
        0x0B,
    ];
    assert_eq!(output.as_slice(), &expected);
}

#[test]
fn test_pad_buffer_exact_block() {
    let mut input = [0u8; 16];
    for i in 0..16 {
        input[i] = (b'A' + i as u8) as u8;
    }
    let mut output: Vec<u8> = Vec::new();
    let mut output_len: usize = 0;
    padding::pad_buffer(&input, 16, &mut output, &mut output_len);
    assert_eq!(output_len, 32);
    assert_eq!(output.len(), 32);
    // first 16 bytes are the input
    for i in 0..16 {
        assert_eq!(output[i], (b'A' + i as u8) as u8);
    }
    for i in 16..32 {
        assert_eq!(output[i], 0x10);
    }
}

#[test]
fn test_pad_buffer_empty() {
    let input: [u8; 0] = [];
    let mut output: Vec<u8> = Vec::new();
    let mut output_len: usize = 0;
    padding::pad_buffer(&input, 0, &mut output, &mut output_len);
    assert_eq!(output_len, 16);
    assert_eq!(output.len(), 16);
    for i in 0..16 {
        assert_eq!(output[i], 0x10);
    }
}

#[test]
fn test_remove_padding_valid() {
    let padded = [0x01, 0x02, 0x03, 0x04, 0x04, 0x04, 0x04, 0x04];
    assert_eq!(padding::remove_padding(&padded, 8), 4);
}

#[test]
fn test_remove_padding_pad_value_out_of_range() {
    // Last byte is > 16 — return inputLen unchanged.
    let padded = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x20];
    assert_eq!(padding::remove_padding(&padded, 8), 8);
}

#[test]
fn test_remove_padding_pad_value_zero() {
    let padded = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x00];
    assert_eq!(padding::remove_padding(&padded, 8), 8);
}

#[test]
fn test_remove_padding_invalid_padding_byte_mismatch() {
    // Mismatched padding bytes (0x08 says 8 bytes of 0x08 but they aren't)
    // The C code wraps around because (inputLen - padValue) underflows or does
    // not match. With inputLen=8 and pad=0x08 we walk i=0..8 and find 0x01 != 0x08
    // so the function returns inputLen (8).
    let padded = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
    assert_eq!(padding::remove_padding(&padded, 8), 8);
}

#[test]
fn test_remove_padding_empty() {
    assert_eq!(padding::remove_padding(&[], 0), 0);
}

#[test]
fn test_remove_padding_full_pad_block() {
    // All 16 bytes are 0x10 — strips to 0 length.
    let padded = [0x10u8; 16];
    assert_eq!(padding::remove_padding(&padded, 16), 0);
}

#[test]
fn test_pad_then_remove_roundtrip() {
    let s = b"Some sample input";
    let mut padded: Vec<u8> = Vec::new();
    let mut padded_len: usize = 0;
    padding::pad_buffer(s, s.len(), &mut padded, &mut padded_len);
    let unpadded_len = padding::remove_padding(&padded, padded_len);
    assert_eq!(unpadded_len, s.len());
    assert_eq!(&padded[..unpadded_len], s);
}

fn main() {}
