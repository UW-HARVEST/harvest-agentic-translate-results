use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;

fn lzma_encode_header_properties(lzma_state: &LZMAState) -> u8 {
    let p = &lzma_state.properties;
    (p.pb * 5 + p.lp) * 9 + p.lc
}

pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let props = lzma_encode_header_properties(lzma_state);
    output.write(&[props]);

    // Note: in the C code this is `htole32(0x400000)`, then written as 4 bytes.
    let dictsize: u32 = 0x400000;
    output.write(&dictsize.to_le_bytes());

    // Note: the C code does `uint64_t outsize = htole32(lzma_state->data_size)`,
    // which converts to a uint32_t little-endian, then stored as a uint64_t.
    // So the upper 4 bytes are zero (because htole32 returns a uint32_t and is
    // implicitly widened to a uint64_t before storage). The end result is that
    // the bytes written are the lower 32 bits of the size, little endian, then
    // four zero bytes.
    let data_size_u32 = lzma_state.data.len() as u32;
    let outsize: u64 = data_size_u32 as u64;
    output.write(&outsize.to_le_bytes());
}
