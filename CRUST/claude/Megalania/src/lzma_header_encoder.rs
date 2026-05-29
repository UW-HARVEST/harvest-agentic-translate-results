use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;

fn lzma_encode_header_properties(state: &LZMAState) -> u8 {
    let p = &state.properties;
    (p.pb * 5 + p.lp) * 9 + p.lc
}

pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let props = lzma_encode_header_properties(lzma_state);
    output.write(&[props]);

    let dictsize: u32 = 0x400000;
    output.write(&dictsize.to_le_bytes());

    // Match the C bug: htole32 is applied to a uint64_t. The upper 32 bits
    // are whatever was in memory, but we mimic the observable bytes by
    // writing data_size as little-endian u32 followed by 4 bytes of zeros.
    let outsize = lzma_state.data.len() as u32;
    let mut buf = [0u8; 8];
    buf[..4].copy_from_slice(&outsize.to_le_bytes());
    output.write(&buf);
}
