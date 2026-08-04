pub fn pad_buffer(input: &[u8], input_len: usize, output: &mut Vec<u8>, output_len: &mut usize) {
    let input_len = input_len.min(input.len());
    let block_size = 16usize;
    let padded_len = ((input_len / block_size) + 1) * block_size;
    let pad_value = (padded_len - input_len) as u8;

    output.clear();
    output.extend_from_slice(&input[..input_len]);
    output.resize(padded_len, pad_value);
    *output_len = padded_len;
}
pub fn remove_padding(input: &[u8], input_len: usize) -> usize {
    let input_len = input_len.min(input.len());
    if input_len == 0 {
        return 0;
    }

    let pad_value = input[input_len - 1] as usize;
    if !(1..=16).contains(&pad_value) || pad_value > input_len {
        return input_len;
    }

    if input[(input_len - pad_value)..input_len]
        .iter()
        .all(|&byte| byte as usize == pad_value)
    {
        input_len - pad_value
    } else {
        input_len
    }
}
