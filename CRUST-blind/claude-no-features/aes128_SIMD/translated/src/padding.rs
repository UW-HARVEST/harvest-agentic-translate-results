pub fn pad_buffer(input: &[u8], input_len: usize, output: &mut Vec<u8>, output_len: &mut usize) {
    let block_size: usize = 16;
    let padded_len = (input_len / block_size + 1) * block_size;
    output.clear();
    output.reserve(padded_len);
    // Copy the input bytes (only up to input_len, mirroring C's memcpy).
    let copy_len = core::cmp::min(input_len, input.len());
    output.extend_from_slice(&input[..copy_len]);
    // If input.len() < input_len (which we don't expect), pad with zeros.
    while output.len() < input_len {
        output.push(0);
    }
    let pad_value = (padded_len - input_len) as u8;
    for _ in 0..pad_value {
        output.push(pad_value);
    }
    *output_len = padded_len;
}

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
