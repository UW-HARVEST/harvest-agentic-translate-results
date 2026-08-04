use Megalania::encoder_interface::EncoderInterface;
use Megalania::probability::{Prob, PROB_INIT_VAL};
use Megalania::probability_model::{
    encode_bit, encode_bit_tree, encode_bit_tree_reverse, encode_direct_bits,
};

#[derive(Default)]
struct CapEnc {
    bits: Vec<bool>,
    direct: Vec<(u32, u32)>,
}

impl EncoderInterface for CapEnc {
    fn encode_bit(&mut self, bit: bool, _prob: Prob) {
        self.bits.push(bit);
    }
    fn encode_direct_bits(&mut self, bits: u32, num_bits: u32) {
        self.direct.push((bits, num_bits));
    }
}

#[test]
fn test_encode_bit_zero_with_init_val() {
    // From C: encode_bit(0, 1024) -> p=1056
    let mut enc = CapEnc::default();
    let mut p: Prob = PROB_INIT_VAL;
    encode_bit(false, &mut p, &mut enc);
    assert_eq!(p, 1056);
    assert_eq!(enc.bits.len(), 1);
    assert!(!enc.bits[0]);
}

#[test]
fn test_encode_bit_one_with_init_val() {
    // From C: encode_bit(1, 1024) -> p=992
    let mut enc = CapEnc::default();
    let mut p: Prob = PROB_INIT_VAL;
    encode_bit(true, &mut p, &mut enc);
    assert_eq!(p, 992);
}

#[test]
fn test_encode_bit_zero_repeat() {
    // From C: 5 iterations encoding 0 from p=1024:
    // 1056, 1087, 1117, 1146, 1174
    let mut enc = CapEnc::default();
    let mut p: Prob = PROB_INIT_VAL;
    let expected = [1056, 1087, 1117, 1146, 1174];
    for &exp in expected.iter() {
        encode_bit(false, &mut p, &mut enc);
        assert_eq!(p, exp);
    }
}

#[test]
fn test_encode_bit_one_repeat() {
    // From C: 5 iterations encoding 1 from p=1024:
    // 992, 961, 931, 902, 874
    let mut enc = CapEnc::default();
    let mut p: Prob = PROB_INIT_VAL;
    let expected = [992, 961, 931, 902, 874];
    for &exp in expected.iter() {
        encode_bit(true, &mut p, &mut enc);
        assert_eq!(p, exp);
    }
}

#[test]
fn test_encode_bit_edge_zero_prob() {
    // From C:
    //   encode_bit(0, 0) -> p=64
    //   encode_bit(1, 0) -> p=0
    let mut enc = CapEnc::default();
    let mut p: Prob = 0;
    encode_bit(false, &mut p, &mut enc);
    assert_eq!(p, 64);

    let mut p: Prob = 0;
    encode_bit(true, &mut p, &mut enc);
    assert_eq!(p, 0);
}

#[test]
fn test_encode_bit_edge_2047_prob() {
    // From C:
    //   encode_bit(0, 2047) -> p=2047
    //   encode_bit(1, 2047) -> p=1984
    let mut enc = CapEnc::default();
    let mut p: Prob = 2047;
    encode_bit(false, &mut p, &mut enc);
    assert_eq!(p, 2047);

    let mut p: Prob = 2047;
    encode_bit(true, &mut p, &mut enc);
    assert_eq!(p, 1984);
}

#[test]
fn test_encode_bit_tree_value_10_4bits() {
    // From C: encode_bit_tree(10, probs, 4) emits bits 1,0,1,0
    // Final probs values (for indices 1..16):
    //   992 1024 1056 1024 1024 992 1024 1024 1024 1024 1024 1024 1056 1024 1024
    let mut enc = CapEnc::default();
    let mut probs = [PROB_INIT_VAL; 16];
    encode_bit_tree(10, &mut probs, 4, &mut enc);
    assert_eq!(enc.bits, vec![true, false, true, false]);
    let expected_post: [u16; 15] = [
        992, 1024, 1056, 1024, 1024, 992, 1024, 1024, 1024, 1024, 1024, 1024, 1056, 1024, 1024,
    ];
    for i in 0..15 {
        assert_eq!(probs[i + 1], expected_post[i], "probs[{}]", i + 1);
    }
}

#[test]
fn test_encode_bit_tree_reverse_value_10_4bits() {
    // From C: encode_bit_tree_reverse(10, probs, 4) emits bits 0,1,0,1
    // Final probs values (for indices 1..16):
    //   1056 992 1024 1024 1056 1024 1024 1024 1024 992 1024 1024 1024 1024 1024
    let mut enc = CapEnc::default();
    let mut probs = [PROB_INIT_VAL; 16];
    encode_bit_tree_reverse(10, &mut probs, 4, &mut enc);
    assert_eq!(enc.bits, vec![false, true, false, true]);
    let expected_post: [u16; 15] = [
        1056, 992, 1024, 1024, 1056, 1024, 1024, 1024, 1024, 992, 1024, 1024, 1024, 1024, 1024,
    ];
    for i in 0..15 {
        assert_eq!(probs[i + 1], expected_post[i], "probs[{}]", i + 1);
    }
}

#[test]
fn test_encode_direct_bits() {
    let mut enc = CapEnc::default();
    encode_direct_bits(0xABCD, 16, &mut enc);
    assert_eq!(enc.direct.len(), 1);
    assert_eq!(enc.direct[0], (0xABCD, 16));
}

fn main() {}
