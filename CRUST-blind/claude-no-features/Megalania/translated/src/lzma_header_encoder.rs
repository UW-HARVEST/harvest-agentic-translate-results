use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;

fn lzma_encode_header_properties(lzma_state: &LZMAState) -> u8 {
    let p = &lzma_state.properties;
    (p.pb * 5 + p.lp) * 9 + p.lc
}

pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let props = lzma_encode_header_properties(lzma_state);
    output.write(&[props]);

    // todo: peg this to data size
    let dictsize: u32 = 0x400000;
    output.write(&dictsize.to_le_bytes());

    // The C code does `htole32(lzma_state->data_size)` and stores in a
    // uint64_t -- so only the low 32 bits are populated, and the high
    // 32 bits are uninitialized. We replicate the *intended* behaviour
    // (a 64-bit little endian length) since that's what the LZMA format
    // actually expects.
    let outsize: u64 = lzma_state.data.len() as u64;
    output.write(&outsize.to_le_bytes());
}
