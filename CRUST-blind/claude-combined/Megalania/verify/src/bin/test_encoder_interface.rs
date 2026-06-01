use Megalania::encoder_interface::EncoderInterface;
use Megalania::probability::Prob;

struct SimpleEncoder {
    pub bits_count: u32,
    pub direct_count: u32,
}

impl EncoderInterface for SimpleEncoder {
    fn encode_bit(&mut self, _bit: bool, _prob: Prob) {
        self.bits_count += 1;
    }
    fn encode_direct_bits(&mut self, _bits: u32, _num_bits: u32) {
        self.direct_count += 1;
    }
}

#[test]
fn test_encoder_interface_basic() {
    let mut e = SimpleEncoder {
        bits_count: 0,
        direct_count: 0,
    };
    e.encode_bit(true, 1024);
    e.encode_bit(false, 100);
    e.encode_direct_bits(0xff, 8);
    assert_eq!(e.bits_count, 2);
    assert_eq!(e.direct_count, 1);
}

fn main() {}
