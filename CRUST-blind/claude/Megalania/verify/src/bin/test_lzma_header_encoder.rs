use Megalania::lzma_header_encoder::lzma_encode_header;
use Megalania::lzma_state::{LZMAProperties, LZMAState, make_lzma_probability_model};
use Megalania::output_interface::OutputInterface;

struct ByteCollector {
    bytes: Vec<u8>,
}
impl OutputInterface for ByteCollector {
    fn write(&mut self, data: &[u8]) -> bool {
        self.bytes.extend_from_slice(data);
        true
    }
}

fn make_state<'a>(data: &'a [u8], props: LZMAProperties) -> LZMAState<'a> {
    LZMAState {
        data,
        properties: props,
        ctx_state: 0,
        dists: [0; 4],
        probs: make_lzma_probability_model(),
        position: 0,
    }
}

#[test]
fn test_header_props_3_0_2_size_3() {
    // C output: header(lc=3,lp=0,pb=2,size=3): len=13 bytes=
    //   5d 00 00 40 00 03 00 00 00 00 00 00 00
    let data = [1u8, 2, 3];
    let s = make_state(&data, LZMAProperties { lc: 3, lp: 0, pb: 2 });
    let mut out = ByteCollector { bytes: vec![] };
    lzma_encode_header(&s, &mut out);
    assert_eq!(
        out.bytes,
        vec![0x5d, 0x00, 0x00, 0x40, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn test_header_props_0_0_0_size_5() {
    // C output: header(lc=0,lp=0,pb=0,size=5): bytes=
    //   00 00 00 40 00 05 00 00 00 00 00 00 00
    let data = [0u8; 5];
    let s = make_state(&data, LZMAProperties { lc: 0, lp: 0, pb: 0 });
    let mut out = ByteCollector { bytes: vec![] };
    lzma_encode_header(&s, &mut out);
    assert_eq!(
        out.bytes,
        vec![0x00, 0x00, 0x00, 0x40, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

#[test]
fn test_header_props_1_2_3_size_256() {
    // C output: header(lc=1,lp=2,pb=3,size=0x100): bytes=
    //   9a 00 00 40 00 00 01 00 00 00 00 00 00
    let data = vec![0u8; 256];
    let s = make_state(&data, LZMAProperties { lc: 1, lp: 2, pb: 3 });
    let mut out = ByteCollector { bytes: vec![] };
    lzma_encode_header(&s, &mut out);
    assert_eq!(
        out.bytes,
        vec![0x9a, 0x00, 0x00, 0x40, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
}

fn main() {}
