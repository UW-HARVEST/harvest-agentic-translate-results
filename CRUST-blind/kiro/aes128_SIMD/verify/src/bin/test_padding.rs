use aes128_SIMD::padding::{pad_buffer, remove_padding};

#[test]
fn test_pad_buffer_short() {
    let input = b"Hello";
    let mut output = Vec::new();
    let mut output_len = 0;
    pad_buffer(input, 5, &mut output, &mut output_len);
    assert_eq!(output_len, 16);
    assert_eq!(output, vec![
        0x48, 0x65, 0x6C, 0x6C, 0x6F,
        0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B,
    ]);
}

#[test]
fn test_pad_buffer_exact_block() {
    let input = b"0123456789ABCDEF";
    let mut output = Vec::new();
    let mut output_len = 0;
    pad_buffer(input, 16, &mut output, &mut output_len);
    assert_eq!(output_len, 32);
    assert_eq!(&output[..16], b"0123456789ABCDEF");
    assert_eq!(&output[16..], &[0x10u8; 16]);
}

#[test]
fn test_remove_padding() {
    let padded: Vec<u8> = vec![
        0x48, 0x65, 0x6C, 0x6C, 0x6F,
        0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B, 0x0B,
    ];
    assert_eq!(remove_padding(&padded, 16), 5);
}

#[test]
fn test_remove_padding_full_block() {
    let mut padded = b"0123456789ABCDEF".to_vec();
    padded.extend_from_slice(&[0x10u8; 16]);
    assert_eq!(remove_padding(&padded, 32), 16);
}

#[test]
fn test_remove_padding_empty() {
    assert_eq!(remove_padding(&[], 0), 0);
}

#[test]
fn test_remove_padding_invalid() {
    let bad = [0x41u8; 16]; // last byte 0x41=65 > 16
    assert_eq!(remove_padding(&bad, 16), 16);
}

fn main() {}
