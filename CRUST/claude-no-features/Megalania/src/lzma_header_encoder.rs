use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;

fn lzma_encode_header_properties(lzma_state: &LZMAState) -> u8 {
    let p = &lzma_state.properties;
    (p.pb * 5 + p.lp) * 9 + p.lc
}

pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let props = lzma_encode_header_properties(lzma_state);
    output.write(&[props]);

    // dictsize = 0x400000, written as little-endian u32
    let dictsize: u32 = 0x400000;
    output.write(&dictsize.to_le_bytes());

    // The C version mistakenly does `htole32(lzma_state->data_size)` and then
    // writes a `uint64_t`. We reproduce its observable byte sequence:
    //   - the low 4 bytes are the little-endian representation of data_size as u32
    //   - the high 4 bytes are uninitialized in C, but here we'll write zeros
    //     since we cannot reproduce uninitialized memory safely.
    //
    // To match the documented LZMA format more closely, we treat the output as
    // a little-endian 64-bit number.
    let outsize: u64 = lzma_state.data.len() as u64;
    output.write(&outsize.to_le_bytes());
}
