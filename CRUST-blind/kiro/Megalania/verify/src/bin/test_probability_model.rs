use Megalania::probability_model::*;
use Megalania::probability::PROB_INIT_VAL;
use Megalania::perplexity_encoder::PerplexityEncoder;
use Megalania::encoder_interface::EncoderInterface;

#[test]
fn test_encode_bit_prob_changes() {
    // C ground truth: init=1024, after 0->1056, after 1->1023, after 0->1055, after 1->1023
    let mut prob = PROB_INIT_VAL;
    let mut perplexity: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        encode_bit(false, &mut prob, &mut enc);
    }
    assert_eq!(prob, 1056);
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        encode_bit(true, &mut prob, &mut enc);
    }
    assert_eq!(prob, 1023);
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        encode_bit(false, &mut prob, &mut enc);
    }
    assert_eq!(prob, 1055);
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        encode_bit(true, &mut prob, &mut enc);
    }
    assert_eq!(prob, 1023);
}

#[test]
fn test_encode_bit_perplexity() {
    // C ground truth: cumulative perplexity after each encode_bit
    let mut prob = PROB_INIT_VAL;
    let mut perplexity: u64 = 0;

    { let mut enc = PerplexityEncoder::new(&mut perplexity); encode_bit(false, &mut prob, &mut enc); }
    assert_eq!(perplexity, 2048);
    { let mut enc = PerplexityEncoder::new(&mut perplexity); encode_bit(true, &mut prob, &mut enc); }
    assert_eq!(perplexity, 4189);
    { let mut enc = PerplexityEncoder::new(&mut perplexity); encode_bit(false, &mut prob, &mut enc); }
    assert_eq!(perplexity, 6239);
    { let mut enc = PerplexityEncoder::new(&mut perplexity); encode_bit(true, &mut prob, &mut enc); }
    assert_eq!(perplexity, 8377);
}

#[test]
fn test_encode_bit_tree() {
    // C ground truth: encode_bit_tree(5, 6bits) -> perplexity=12288
    let mut probs = vec![PROB_INIT_VAL; 128];
    let mut perplexity: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        encode_bit_tree(5, &mut probs, 6, &mut enc);
    }
    assert_eq!(perplexity, 12288);
    assert_eq!(probs[1], 1056);
    assert_eq!(probs[2], 1056);
    assert_eq!(probs[3], 1024);
    assert_eq!(probs[4], 1056);
    assert_eq!(probs[5], 1024);
    assert_eq!(probs[6], 1024);
}

#[test]
fn test_encode_bit_tree_reverse() {
    // C ground truth: encode_bit_tree_reverse(13, 4bits) -> perplexity=8192
    let mut probs = vec![PROB_INIT_VAL; 32];
    let mut perplexity: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        encode_bit_tree_reverse(13, &mut probs, 4, &mut enc);
    }
    assert_eq!(perplexity, 8192);
    assert_eq!(probs[1], 992);
    assert_eq!(probs[2], 1024);
    assert_eq!(probs[3], 1056);
    assert_eq!(probs[4], 1024);
}

#[test]
fn test_encode_direct_bits() {
    // C ground truth: encode_direct_bits(0xAB, 8) -> perplexity=16384
    let mut perplexity: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        encode_direct_bits(0xAB, 8, &mut enc);
    }
    assert_eq!(perplexity, 16384);
}

fn main() {}
