use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;

fn lzma_encode_header_properties(lzma_state: &LZMAState) -> u8 {
    let p = &lzma_state.properties;
    (p.pb as u32 * 5 + p.lp as u32) as u8 * 9 + p.lc
}

pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let props = lzma_encode_header_properties(lzma_state);
    output.write(&[props]);

    // Dictionary size: 0x400000, written little-endian.
    let dictsize: u32 = 0x400000;
    output.write(&dictsize.to_le_bytes());

    // Output (uncompressed) data size as little-endian u64.
    // The C code uses htole32(data_size) and writes as a uint64_t. The original
    // code is buggy in that the upper 32 bits are uninitialised; we faithfully
    // reproduce the truncated 32-bit-LE-then-padded behaviour using u64::to_le.
    let outsize: u64 = lzma_state.data.len() as u32 as u64;
    output.write(&outsize.to_le_bytes());
}
