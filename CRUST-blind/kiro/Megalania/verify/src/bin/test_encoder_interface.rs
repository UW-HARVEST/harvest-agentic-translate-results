use Megalania::encoder_interface::EncoderInterface;
use Megalania::probability::{Prob, PROB_INIT_VAL};

struct MockEncoder {
    bits: Vec<(bool, Prob)>,
    direct: Vec<(u32, u32)>,
}

impl EncoderInterface for MockEncoder {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        self.bits.push((bit, prob));
    }
    fn encode_direct_bits(&mut self, bits: u32, num_bits: u32) {
        self.direct.push((bits, num_bits));
    }
}

#[test]
fn test_encoder_interface_trait() {
    let mut enc = MockEncoder { bits: Vec::new(), direct: Vec::new() };
    enc.encode_bit(true, PROB_INIT_VAL);
    enc.encode_direct_bits(0xAB, 8);
    assert_eq!(enc.bits.len(), 1);
    assert_eq!(enc.bits[0], (true, 1024));
    assert_eq!(enc.direct.len(), 1);
    assert_eq!(enc.direct[0], (0xAB, 8));
}

fn main() {}
