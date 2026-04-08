use Megalania::range_encoder::RangeEncoder;
use Megalania::probability_model::encode_bit;
use Megalania::probability::PROB_INIT_VAL;
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
fn test_range_encoder_output() {
    // C ground truth: encode_bit(0), encode_bit(1), encode_bit(0) then flush -> [0x00, 0x41, 0xff, 0xfb, 0xe0]
    let mut out = BufOutput { data: Vec::new() };
    let mut prob = PROB_INIT_VAL;
    {
        let mut enc = RangeEncoder::new(&mut out);
        encode_bit(false, &mut prob, &mut enc);
        encode_bit(true, &mut prob, &mut enc);
        encode_bit(false, &mut prob, &mut enc);
        enc.flush();
    }
    assert_eq!(out.data.len(), 5);
    assert_eq!(out.data, vec![0x00, 0x41, 0xff, 0xfb, 0xe0]);
}

fn main() {}
