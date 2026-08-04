/// PKCS#7-style padding to a 16-byte block boundary. Always appends at least one byte.
pub fn pad_buffer(input: &[u8], input_len: usize, output: &mut Vec<u8>, output_len: &mut usize) {
    let block_size: usize = 16;
    let padded_len = (input_len / block_size + 1) * block_size;
    let pad_value = (padded_len - input_len) as u8;

    output.clear();
    output.reserve(padded_len);
    output.extend_from_slice(&input[..input_len]);
    output.resize(padded_len, pad_value);

    *output_len = padded_len;
}

/// Removes PKCS#7 padding from `input`. Returns the input length if padding is invalid.
pub fn remove_padding(input: &[u8], input_len: usize) -> usize {
    if input_len == 0 {
        return 0;
    }
    let pad_value = input[input_len - 1];
    if pad_value < 1 || pad_value > 16 {
        return input_len;
    }
    let pad_value_usize = pad_value as usize;
    if pad_value_usize > input_len {
        return input_len;
    }
    let start = input_len - pad_value_usize;
    for i in start..input_len {
        if input[i] != pad_value {
            return input_len;
        }
    }
    input_len - pad_value_usize
}
