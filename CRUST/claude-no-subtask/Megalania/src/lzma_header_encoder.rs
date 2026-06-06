use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;

fn lzma_encode_header_properties(lzma_state: &LZMAState) -> u8 {
    let p = &lzma_state.properties;
    (p.pb * 5 + p.lp) * 9 + p.lc
}

pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let props = lzma_encode_header_properties(lzma_state);
    output.write(&[props]);

    let dictsize: u32 = 0x400000;
    output.write(&dictsize.to_le_bytes());

    // Note the C code uses `htole32(lzma_state->data_size)` and stores it as 8 bytes.
    // We replicate that exact behavior: take only the lower 32 bits, then write 8 little-endian bytes.
    let outsize_low: u32 = (lzma_state.data.len() & 0xFFFF_FFFF) as u32;
    let outsize: u64 = outsize_low as u64;
    output.write(&outsize.to_le_bytes());
}
