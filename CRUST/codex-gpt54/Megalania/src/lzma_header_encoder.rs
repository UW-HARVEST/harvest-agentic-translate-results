use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;
pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let props = ((lzma_state.properties.pb * 5 + lzma_state.properties.lp) * 9)
        + lzma_state.properties.lc;
    output.write(&[props]);

    let dict_size = 0x400000u32.to_le_bytes();
    output.write(&dict_size);

    let out_size = (lzma_state.data.len() as u64).to_le_bytes();
    output.write(&out_size);
}
