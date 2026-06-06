use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;

pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let p = &lzma_state.properties;
    let props: u8 = (p.pb * 5 + p.lp) * 9 + p.lc;
    output.write(&[props]);

    let dictsize: u32 = 0x400000;
    output.write(&dictsize.to_le_bytes());

    // Note: the C code uses `htole32(lzma_state->data_size)` and assigns that
    // to a uint64_t. The high 32 bits are uninitialized in C; we follow that
    // pattern by writing only the low 32 bits and zeroing the rest, since the
    // intent of the header field is the (little-endian) decompressed size.
    let outsize: u64 = (lzma_state.data.len() as u32) as u64;
    output.write(&outsize.to_le_bytes());
}
