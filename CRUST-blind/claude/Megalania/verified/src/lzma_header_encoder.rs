use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;

fn lzma_encode_header_properties(lzma_state: &LZMAState) -> u8 {
    let p = &lzma_state.properties;
    (p.pb * 5 + p.lp) * 9 + p.lc
}

pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let props = lzma_encode_header_properties(lzma_state);
    output.write(&[props]);

    // dictsize = htole32(0x400000)
    let dictsize: u32 = 0x400000;
    output.write(&dictsize.to_le_bytes());

    // The C code is `uint64_t outsize = htole32(lzma_state->data_size);`
    // which assigns the truncated 32-bit little-endian value to a 64-bit variable.
    // To replicate the exact byte sequence we emit on a little-endian host:
    //   - the lower 4 bytes are data_size truncated to u32 in little-endian
    //   - the upper 4 bytes are zero (the high 32 bits of the u64)
    // (This matches what the original code does on common little-endian platforms.)
    let data_size = lzma_state.data.len() as u64;
    let truncated_le: u32 = (data_size & 0xFFFFFFFF) as u32;
    let outsize: u64 = truncated_le as u64;
    output.write(&outsize.to_le_bytes());
}
