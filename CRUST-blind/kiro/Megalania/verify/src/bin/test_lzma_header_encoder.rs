use Megalania::lzma_header_encoder::lzma_encode_header;
use Megalania::lzma_state::{LZMAProperties, LZMAState};
use Megalania::output_interface::OutputInterface;

struct BufOutput {
    data: Vec<u8>,
}
impl OutputInterface for BufOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.data.extend_from_slice(data);
        true
    }
}

#[test]
fn test_header_bytes() {
    // C ground truth: [0x5d, 0x00, 0x00, 0x40, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    let data = [0x41u8, 0x42, 0x43, 0x44, 0x45];
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    let state = LZMAState::new(&data, props);

    let mut out = BufOutput { data: Vec::new() };
    lzma_encode_header(&state, &mut out);

    assert_eq!(out.data.len(), 13);
    assert_eq!(
        out.data,
        vec![0x5d, 0x00, 0x00, 0x40, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn test_header_properties_byte() {
    // props byte = (pb*5 + lp)*9 + lc = (2*5+0)*9+3 = 93 = 0x5d
    let data = [0x41u8];
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    let state = LZMAState::new(&data, props);

    let mut out = BufOutput { data: Vec::new() };
    lzma_encode_header(&state, &mut out);
    assert_eq!(out.data[0], 0x5d);
}

fn main() {}
