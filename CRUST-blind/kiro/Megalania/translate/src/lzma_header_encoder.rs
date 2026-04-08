use crate::lzma_state::LZMAState;
use crate::output_interface::OutputInterface;
pub fn lzma_encode_header(lzma_state: &LZMAState, output: &mut dyn OutputInterface) {
    let p = &lzma_state.properties;
    let props: u8 = (p.pb * 5 + p.lp) * 9 + p.lc;
    output.write(&[props]);

    let dictsize: u32 = 0x400000; // todo: peg this to data size
    output.write(&dictsize.to_le_bytes());

    let outsize: u64 = lzma_state.data.len() as u64;
    output.write(&outsize.to_le_bytes());
}
