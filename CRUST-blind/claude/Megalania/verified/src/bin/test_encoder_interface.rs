use Megalania::encoder_interface::EncoderInterface;
use Megalania::probability::Prob;

struct TestEnc {
    bit_calls: Vec<(bool, Prob)>,
    direct_calls: Vec<(u32, u32)>,
}

impl EncoderInterface for TestEnc {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        self.bit_calls.push((bit, prob));
    }
    fn encode_direct_bits(&mut self, bits: u32, num_bits: u32) {
        self.direct_calls.push((bits, num_bits));
    }
}

#[test]
fn test_encoder_trait_compiles() {
    let mut enc = TestEnc { bit_calls: vec![], direct_calls: vec![] };
    enc.encode_bit(true, 1024);
    enc.encode_direct_bits(7, 3);
    assert_eq!(enc.bit_calls, vec![(true, 1024u16)]);
    assert_eq!(enc.direct_calls, vec![(7u32, 3u32)]);
}

#[test]
fn test_encoder_trait_object() {
    let mut enc = TestEnc { bit_calls: vec![], direct_calls: vec![] };
    let dyn_enc: &mut dyn EncoderInterface = &mut enc;
    dyn_enc.encode_bit(false, 100);
    assert_eq!(enc.bit_calls.len(), 1);
    assert_eq!(enc.bit_calls[0], (false, 100u16));
}

fn main() {}
