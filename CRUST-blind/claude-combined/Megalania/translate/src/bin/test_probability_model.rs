use Megalania::encoder_interface::EncoderInterface;
use Megalania::probability::{Prob, PROB_INIT_VAL};
use Megalania::probability_model::{
    encode_bit, encode_bit_tree, encode_bit_tree_reverse, encode_direct_bits,
};

/// Capturing encoder records all encode_bit and encode_direct_bits calls.
struct CaptureEncoder {
    bits: Vec<(bool, Prob)>,
    direct: Vec<(u32, u32)>,
}

impl CaptureEncoder {
    fn new() -> Self {
        Self {
            bits: Vec::new(),
            direct: Vec::new(),
        }
    }
}

impl EncoderInterface for CaptureEncoder {
    fn encode_bit(&mut self, bit: bool, prob: Prob) {
        self.bits.push((bit, prob));
    }
    fn encode_direct_bits(&mut self, bits: u32, num_bits: u32) {
        self.direct.push((bits, num_bits));
    }
}

#[test]
fn test_encode_bit_zero_updates_prob_up() {
    // C: NUM_MOVE_BITS = 5, NUM_BIT_MODEL_TOTAL_BITS = 11
    // For prob = 1024 and bit = 0: v = 1024 + (2048 - 1024) >> 5 = 1024 + 32 = 1056
    let mut prob: Prob = PROB_INIT_VAL;
    let mut enc = CaptureEncoder::new();
    encode_bit(false, &mut prob, &mut enc);
    assert_eq!(prob, 1056);
    assert_eq!(enc.bits, vec![(false, 1024)]);
    assert_eq!(enc.direct.len(), 0);
}

#[test]
fn test_encode_bit_one_updates_prob_down() {
    // For prob = 1024 and bit = 1: v = 1024 - (1024 >> 5) = 1024 - 32 = 992
    let mut prob: Prob = PROB_INIT_VAL;
    let mut enc = CaptureEncoder::new();
    encode_bit(true, &mut prob, &mut enc);
    assert_eq!(prob, 992);
    assert_eq!(enc.bits, vec![(true, 1024)]);
}

#[test]
fn test_encode_bit_zero_at_zero_prob() {
    let mut prob: Prob = 0;
    let mut enc = CaptureEncoder::new();
    encode_bit(false, &mut prob, &mut enc);
    // v = 0 + (2048 - 0) >> 5 = 64
    assert_eq!(prob, 64);
    assert_eq!(enc.bits, vec![(false, 0)]);
}

#[test]
fn test_encode_bit_one_at_one_prob() {
    let mut prob: Prob = 1;
    let mut enc = CaptureEncoder::new();
    encode_bit(true, &mut prob, &mut enc);
    // v = 1 - (1 >> 5) = 1 - 0 = 1
    assert_eq!(prob, 1);
    assert_eq!(enc.bits, vec![(true, 1)]);
}

#[test]
fn test_encode_direct_bits_passes_through() {
    let mut enc = CaptureEncoder::new();
    encode_direct_bits(0b10110, 5, &mut enc);
    assert_eq!(enc.direct, vec![(0b10110, 5)]);
    assert_eq!(enc.bits.len(), 0);
}

#[test]
fn test_encode_bit_tree_5_3() {
    // bits=5 (binary 101), num_bits=3
    // Iteration: bit_index=2 -> bit = (5>>2)&1 = 1, m=1, encode (1, probs[1])
    //           bit_index=1 -> bit = (5>>1)&1 = 0, m=3, encode (0, probs[3])
    //           bit_index=0 -> bit = (5>>0)&1 = 1, m=6, encode (1, probs[6])
    let mut probs = [PROB_INIT_VAL; 8];
    let mut enc = CaptureEncoder::new();
    encode_bit_tree(5, &mut probs, 3, &mut enc);

    // expected (3 bits encoded)
    assert_eq!(enc.bits.len(), 3);
    assert_eq!(enc.bits[0], (true, 1024));
    assert_eq!(enc.bits[1], (false, 1024));
    assert_eq!(enc.bits[2], (true, 1024));

    // After encoding, probs[1]=992 (bit=1: 1024-32), probs[3]=1056, probs[6]=992
    let expected = [1024, 992, 1024, 1056, 1024, 1024, 992, 1024];
    for (i, e) in expected.iter().enumerate() {
        assert_eq!(probs[i], *e, "probs[{}] mismatch", i);
    }
}

#[test]
fn test_encode_bit_tree_reverse_5_3() {
    // bits=5 (0b101), num_bits=3, reverse iteration
    // i=0: bit = 5 & 1 = 1, m=1, encode (1, probs[1]); m=3; bits=2
    // i=1: bit = 2 & 1 = 0, m=3, encode (0, probs[3]); m=6; bits=1
    // i=2: bit = 1 & 1 = 1, m=6, encode (1, probs[6]); m=13; bits=0
    let mut probs = [PROB_INIT_VAL; 8];
    let mut enc = CaptureEncoder::new();
    encode_bit_tree_reverse(5, &mut probs, 3, &mut enc);

    let expected = [1024, 992, 1024, 1056, 1024, 1024, 992, 1024];
    for (i, e) in expected.iter().enumerate() {
        assert_eq!(probs[i], *e, "probs[{}] mismatch", i);
    }
}

#[test]
fn test_encode_bit_tree_zero_bits_4() {
    // bits=0, num_bits=4
    // bit_index=3: bit=0, m=1 -> probs[1]=1056
    // bit_index=2: bit=0, m=2 -> probs[2]=1056
    // bit_index=1: bit=0, m=4 -> probs[4]=1056
    // bit_index=0: bit=0, m=8 -> probs[8]=1056
    let mut probs = [PROB_INIT_VAL; 16];
    let mut enc = CaptureEncoder::new();
    encode_bit_tree(0, &mut probs, 4, &mut enc);
    let expected: [u16; 16] = [
        1024, 1056, 1056, 1024, 1056, 1024, 1024, 1024, 1056, 1024, 1024, 1024, 1024, 1024, 1024,
        1024,
    ];
    for (i, e) in expected.iter().enumerate() {
        assert_eq!(probs[i], *e, "probs[{}] mismatch", i);
    }
}

#[test]
fn test_encode_bit_tree_7_4() {
    let mut probs = [PROB_INIT_VAL; 16];
    let mut enc = CaptureEncoder::new();
    encode_bit_tree(7, &mut probs, 4, &mut enc);
    let expected: [u16; 16] = [
        1024, 1056, 992, 1024, 1024, 992, 1024, 1024, 1024, 1024, 1024, 992, 1024, 1024, 1024,
        1024,
    ];
    for (i, e) in expected.iter().enumerate() {
        assert_eq!(probs[i], *e, "probs[{}] mismatch", i);
    }
}

#[test]
fn test_encode_bit_tree_reverse_7_4() {
    let mut probs = [PROB_INIT_VAL; 16];
    let mut enc = CaptureEncoder::new();
    encode_bit_tree_reverse(7, &mut probs, 4, &mut enc);
    let expected: [u16; 16] = [
        1024, 992, 1024, 992, 1024, 1024, 1024, 992, 1024, 1024, 1024, 1024, 1024, 1024, 1024,
        1056,
    ];
    for (i, e) in expected.iter().enumerate() {
        assert_eq!(probs[i], *e, "probs[{}] mismatch", i);
    }
}

fn main() {}
