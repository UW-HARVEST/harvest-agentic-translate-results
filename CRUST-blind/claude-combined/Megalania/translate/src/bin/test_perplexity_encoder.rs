use Megalania::encoder_interface::EncoderInterface;
use Megalania::perplexity_encoder::PerplexityEncoder;

#[test]
fn test_perplexity_encode_bit_zero_init_prob() {
    // From C: encode_bit_perp 0 1024 -> perp=2048
    let mut perp: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perp);
    enc.encode_bit(false, 1024);
    assert_eq!(perp, 2048);
}

#[test]
fn test_perplexity_encode_bit_one_init_prob() {
    let mut perp: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perp);
    enc.encode_bit(true, 1024);
    assert_eq!(perp, 2048);
}

#[test]
fn test_perplexity_encode_bit_zero_at_100() {
    let mut perp: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perp);
    enc.encode_bit(false, 100);
    assert_eq!(perp, 8921);
}

#[test]
fn test_perplexity_encode_bit_one_at_2000() {
    let mut perp: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perp);
    enc.encode_bit(true, 2000);
    assert_eq!(perp, 11089);
}

#[test]
fn test_perplexity_encode_bit_zero_at_zero() {
    let mut perp: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perp);
    enc.encode_bit(false, 0);
    assert_eq!(perp, 0);
}

#[test]
fn test_perplexity_encode_bit_one_at_one() {
    let mut perp: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perp);
    enc.encode_bit(true, 1);
    assert_eq!(perp, 1);
}

#[test]
fn test_perplexity_encode_direct_bits_8() {
    let mut perp: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perp);
    enc.encode_direct_bits(0xff, 8);
    // num_bits << 11 = 8 * 2048 = 16384
    assert_eq!(perp, 16384);
}

#[test]
fn test_perplexity_encode_direct_bits_zero() {
    let mut perp: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perp);
    enc.encode_direct_bits(0, 1);
    assert_eq!(perp, 2048);
}

#[test]
fn test_perplexity_accumulates() {
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        enc.encode_bit(false, 1024);
        enc.encode_bit(true, 1024);
        enc.encode_direct_bits(0, 4);
    }
    // 2048 + 2048 + 4*2048 = 12288
    assert_eq!(perp, 12288);
}

fn main() {}
