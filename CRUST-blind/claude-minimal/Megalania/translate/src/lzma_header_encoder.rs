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

    // The C code uses htole32 on a uint64_t which only fills the low 4 bytes;
    // mirror that behaviour here.
    let outsize: u64 = lzma_state.data.len() as u32 as u64;
    output.write(&outsize.to_le_bytes());
}
