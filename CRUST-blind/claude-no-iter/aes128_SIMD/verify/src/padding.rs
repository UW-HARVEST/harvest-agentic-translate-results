/// Apply PKCS#7 padding to `input` and write the padded buffer to `output`.
/// Always adds at least one byte of padding (i.e. exactly one full block when
/// the input length is already a multiple of 16).
pub fn pad_buffer(input: &[u8], input_len: usize, output: &mut Vec<u8>, output_len: &mut usize) {
    let block_size: usize = 16;
    let padded_len = ((input_len / block_size) + 1) * block_size;
    let pad_value = (padded_len - input_len) as u8;

    output.clear();
    output.reserve(padded_len);
    output.extend_from_slice(&input[..input_len]);
    for _ in input_len..padded_len {
        output.push(pad_value);
    }

    *output_len = padded_len;
}

/// Strip PKCS#7 padding from `input`. Returns the original (unpadded) length;
/// if the padding bytes look invalid, returns `input_len` unchanged.
pub fn remove_padding(input: &[u8], input_len: usize) -> usize {
    if input_len == 0 {
        return 0;
    }
    let pad_value = input[input_len - 1];
    if pad_value < 1 || pad_value > 16 {
        return input_len;
    }
    let pv = pad_value as usize;
    if pv > input_len {
        return input_len;
    }
    for i in (input_len - pv)..input_len {
        if input[i] != pad_value {
            return input_len;
        }
    }
    input_len - pv
}
