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

    // Note: C does `htole32(data_size)` then writes 8 bytes (uint64_t).
    // The original C code has a bug where it stores a 32-bit LE value in 64 bits.
    // We replicate this to match output exactly.
    let outsize_low: u32 = lzma_state.data.len() as u32;
    let mut buf = [0u8; 8];
    buf[..4].copy_from_slice(&outsize_low.to_le_bytes());
    output.write(&buf);
}
