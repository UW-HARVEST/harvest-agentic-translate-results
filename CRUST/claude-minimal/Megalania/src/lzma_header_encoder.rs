use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;

fn lzma_encode_header_properties(lzma_state: &LZMAState) -> u8 {
    let p = &lzma_state.properties;
    (p.pb * 5 + p.lp) * 9 + p.lc
}

pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let props = lzma_encode_header_properties(lzma_state);
    output.write(&[props]);

    // Match the C version (htole32(0x400000)) - dictionary size as little-endian u32.
    let dictsize: u32 = 0x400000;
    output.write(&dictsize.to_le_bytes());

    // The C version writes the data size as a uint64_t but assigns from
    // htole32 of (uint32_t)data_size, so the upper bytes are platform
    // dependent. We replicate the documented behavior of writing the
    // data size as a little-endian u64.
    let outsize: u64 = lzma_state.data.len() as u64;
    output.write(&outsize.to_le_bytes());
}
