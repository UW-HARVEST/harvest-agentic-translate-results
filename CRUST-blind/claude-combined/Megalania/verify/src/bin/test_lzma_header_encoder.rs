use Megalania::lzma_header_encoder::lzma_encode_header;
use Megalania::lzma_state::{LZMAProperties, LZMAState};
use Megalania::output_interface::OutputInterface;

struct BufOutput {
    pub data: Vec<u8>,
}

impl BufOutput {
    fn new() -> Self {
        Self { data: Vec::new() }
    }
}

impl OutputInterface for BufOutput {
    fn write(&mut self, data: &[u8]) -> bool {
        self.data.extend_from_slice(data);
        true
    }
}

fn encode_header_for(lc: u8, lp: u8, pb: u8, data_size: usize) -> Vec<u8> {
    let data = vec![0u8; data_size];
    let props = LZMAProperties { lc, lp, pb };
    let state = LZMAState::new(&data, props);
    let mut output = BufOutput::new();
    lzma_encode_header(&state, &mut output);
    output.data
}

#[test]
fn test_header_default_props() {
    // C: header 0 0 0 100 -> len=13 bytes=00000040006400000000000000
    let bytes = encode_header_for(0, 0, 0, 100);
    let expected: [u8; 13] = [
        0x00, 0x00, 0x00, 0x40, 0x00, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(bytes.len(), 13);
    assert_eq!(bytes, expected);
}

#[test]
fn test_header_props_3_0_2() {
    // C: header 3 0 2 1024 -> 5d000040000004000000000000
    let bytes = encode_header_for(3, 0, 2, 1024);
    let expected: [u8; 13] = [
        0x5d, 0x00, 0x00, 0x40, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn test_header_props_1_1_1() {
    // C: header 1 1 1 65536 -> 37000040000000010000000000
    let bytes = encode_header_for(1, 1, 1, 65536);
    let expected: [u8; 13] = [
        0x37, 0x00, 0x00, 0x40, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    assert_eq!(bytes, expected);
}

#[test]
fn test_header_first_byte_formula() {
    // (pb*5 + lp) * 9 + lc
    // pb=0, lp=0, lc=0 -> 0
    // pb=2, lp=0, lc=3 -> 5d (hex)
    let b = encode_header_for(0, 0, 0, 1);
    assert_eq!(b[0], 0);
    let b = encode_header_for(3, 0, 2, 1);
    assert_eq!(b[0], 0x5d);
    let b = encode_header_for(1, 1, 1, 1);
    assert_eq!(b[0], 0x37);
}

#[test]
fn test_header_dictsize_le() {
    // Bytes 1..5 should be 0x00 0x00 0x40 0x00 (little-endian 0x400000)
    let b = encode_header_for(0, 0, 0, 1);
    assert_eq!(&b[1..5], &[0x00, 0x00, 0x40, 0x00]);
}

#[test]
fn test_header_outsize_le_bytes() {
    // Bytes 5..13 should be data size (low 32 bits, le) plus 4 zero bytes
    let b = encode_header_for(0, 0, 0, 0x12345678);
    assert_eq!(&b[5..13], &[0x78, 0x56, 0x34, 0x12, 0x00, 0x00, 0x00, 0x00]);
}

fn main() {}
