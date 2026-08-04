use Megalania::encoder_interface::EncoderInterface;
use Megalania::output_interface::OutputInterface;
use Megalania::probability::{Prob, PROB_INIT_VAL};
use Megalania::probability_model::encode_bit;
use Megalania::range_encoder::RangeEncoder;

struct ByteCollector {
    bytes: Vec<u8>,
}
impl OutputInterface for ByteCollector {
    fn write(&mut self, data: &[u8]) -> bool {
        self.bytes.extend_from_slice(data);
        true
    }
}

#[test]
fn test_range_encoder_zero_bits() {
    // C output: range_encoder bits=5 zeros prob=1024 -> bytes=5: 00 00 00 00 00
    let mut out = ByteCollector { bytes: vec![] };
    {
        let mut enc = RangeEncoder::new(&mut out);
        let mut p: Prob = PROB_INIT_VAL;
        for _ in 0..5 {
            encode_bit(false, &mut p, &mut enc);
        }
        enc.flush();
    }
    assert_eq!(out.bytes, vec![0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_range_encoder_direct_bits() {
    // C output: range_encoder direct(0xABCDEF, 24) -> 00 ab cd ee fa 32 11 00
    let mut out = ByteCollector { bytes: vec![] };
    {
        let mut enc = RangeEncoder::new(&mut out);
        enc.encode_direct_bits(0xABCDEF, 24);
        enc.flush();
    }
    assert_eq!(
        out.bytes,
        vec![0x00, 0xab, 0xcd, 0xee, 0xfa, 0x32, 0x11, 0x00]
    );
}

#[test]
fn test_range_encoder_mix() {
    // C output: range_encoder mix:
    //   p=1024; encode_bit(1); encode_bit(0); encode_bit(1); direct(0x5,4)
    //   bytes=5: 00 a8 b5 50 00
    let mut out = ByteCollector { bytes: vec![] };
    {
        let mut enc = RangeEncoder::new(&mut out);
        let mut p: Prob = PROB_INIT_VAL;
        encode_bit(true, &mut p, &mut enc);
        encode_bit(false, &mut p, &mut enc);
        encode_bit(true, &mut p, &mut enc);
        enc.encode_direct_bits(0x5, 4);
        enc.flush();
    }
    assert_eq!(out.bytes, vec![0x00, 0xa8, 0xb5, 0x50, 0x00]);
}

#[test]
fn test_initial_state() {
    let mut out = ByteCollector { bytes: vec![] };
    let enc = RangeEncoder::new(&mut out);
    assert_eq!(enc.low, 0);
    assert_eq!(enc.range, 0xFFFFFFFF);
    assert_eq!(enc.cache, 0);
    assert_eq!(enc.cache_size, 1);
}

fn main() {}
