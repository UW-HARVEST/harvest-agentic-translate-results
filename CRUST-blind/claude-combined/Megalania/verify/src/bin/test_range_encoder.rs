use Megalania::encoder_interface::EncoderInterface;
use Megalania::output_interface::OutputInterface;
use Megalania::range_encoder::RangeEncoder;

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

fn run_bits(bits: &[bool], prob: u16) -> Vec<u8> {
    let mut output = BufOutput::new();
    let collected: Vec<u8> = {
        let mut enc = RangeEncoder::new(&mut output);
        for &b in bits {
            enc.encode_bit(b, prob);
        }
        enc.flush();
        output.data.clone()
    };
    collected
}

fn run_direct(bits: u32, num_bits: u32) -> Vec<u8> {
    let mut output = BufOutput::new();
    {
        let mut enc = RangeEncoder::new(&mut output);
        enc.encode_direct_bits(bits, num_bits);
        enc.flush();
    }
    output.data
}

fn run_just_flush() -> Vec<u8> {
    let mut output = BufOutput::new();
    {
        let mut enc = RangeEncoder::new(&mut output);
        enc.flush();
    }
    output.data
}

#[test]
fn test_just_flush() {
    let bytes = run_just_flush();
    assert_eq!(bytes, vec![0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_one_zero_bit_at_init() {
    // C: encode_bits 0 1024 -> bytes=0000000000
    let bytes = run_bits(&[false], 1024);
    assert_eq!(bytes, vec![0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_one_one_bit_at_init() {
    // C: encode_bits 1 1024 -> bytes=007ffffc00
    let bytes = run_bits(&[true], 1024);
    assert_eq!(bytes, vec![0x00, 0x7f, 0xff, 0xfc, 0x00]);
}

#[test]
fn test_four_zero_bits_at_init() {
    let bytes = run_bits(&[false; 4], 1024);
    assert_eq!(bytes, vec![0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_four_one_bits_at_init() {
    // C: encode_bits 1,1,1,1 1024 -> 00effffc00
    let bytes = run_bits(&[true; 4], 1024);
    assert_eq!(bytes, vec![0x00, 0xef, 0xff, 0xfc, 0x00]);
}

#[test]
fn test_alternating_bits_prob_100() {
    // C: encode_bits 0,1,0,1,0,1,0,1 100 -> 0000a3dba7ad30
    let bits = [false, true, false, true, false, true, false, true];
    let bytes = run_bits(&bits, 100);
    assert_eq!(bytes, vec![0x00, 0x00, 0xa3, 0xdb, 0xa7, 0xad, 0x30]);
}

#[test]
fn test_alternating_bits_prob_2000() {
    // C: encode_bits 1,0,1,0 2000 -> 00ffb8d03000
    let bits = [true, false, true, false];
    let bytes = run_bits(&bits, 2000);
    assert_eq!(bytes, vec![0x00, 0xff, 0xb8, 0xd0, 0x30, 0x00]);
}

#[test]
fn test_direct_bits_8_ff() {
    // C: encode_direct 0xff 8 -> 00fefffff800
    let bytes = run_direct(0xff, 8);
    assert_eq!(bytes, vec![0x00, 0xfe, 0xff, 0xff, 0xf8, 0x00]);
}

#[test]
fn test_direct_bits_32_value() {
    // C: encode_direct 0x12345678 32 -> 0012345675cba98800
    let bytes = run_direct(0x12345678, 32);
    assert_eq!(
        bytes,
        vec![0x00, 0x12, 0x34, 0x56, 0x75, 0xcb, 0xa9, 0x88, 0x00]
    );
}

fn main() {}
