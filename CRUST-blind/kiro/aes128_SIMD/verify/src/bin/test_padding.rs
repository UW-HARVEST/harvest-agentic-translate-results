use aes128_SIMD::padding;

#[test]
fn test_pad_buffer_short() {
    let input = b"Hello";
    let mut output = Vec::new();
    let mut output_len = 0;
    padding::pad_buffer(input, 5, &mut output, &mut output_len);
    assert_eq!(output_len, 16);
    assert_eq!(&output[..5], b"Hello");
    for i in 5..16 {
        assert_eq!(output[i], 11); // 16-5=11
    }
}

#[test]
fn test_pad_buffer_exact_block() {
    let input = [0x41u8; 16];
    let mut output = Vec::new();
    let mut output_len = 0;
    padding::pad_buffer(&input, 16, &mut output, &mut output_len);
    assert_eq!(output_len, 32);
    assert_eq!(&output[..16], &input);
    for i in 16..32 {
        assert_eq!(output[i], 16);
    }
}

#[test]
fn test_remove_padding() {
    let mut data = vec![0x48, 0x65, 0x6C, 0x6C, 0x6F];
    data.resize(16, 11);
    assert_eq!(padding::remove_padding(&data, 16), 5);
}

#[test]
fn test_remove_padding_empty() {
    assert_eq!(padding::remove_padding(&[], 0), 0);
}

#[test]
fn test_remove_padding_invalid_zero() {
    let mut data = [0x41u8; 16];
    data[15] = 0x00;
    assert_eq!(padding::remove_padding(&data, 16), 16);
}

#[test]
fn test_remove_padding_invalid_mismatch() {
    let mut data = [0x41u8; 16];
    data[15] = 0x02;
    // data[14] is 0x41, not 0x02 -> invalid
    assert_eq!(padding::remove_padding(&data, 16), 16);
}

#[test]
fn test_pad_then_remove() {
    let input = b"test data 12345";
    let mut output = Vec::new();
    let mut output_len = 0;
    padding::pad_buffer(input, input.len(), &mut output, &mut output_len);
    let unpadded = padding::remove_padding(&output, output_len);
    assert_eq!(unpadded, input.len());
    assert_eq!(&output[..unpadded], &input[..]);
}

fn main() {}
