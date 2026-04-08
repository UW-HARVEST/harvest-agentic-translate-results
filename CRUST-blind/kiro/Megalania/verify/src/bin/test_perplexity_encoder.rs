use Megalania::perplexity_encoder::PerplexityEncoder;
use Megalania::encoder_interface::EncoderInterface;

#[test]
fn test_encode_bit_50_50() {
    let mut perplexity: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        enc.encode_bit(false, 1024);
    }
    assert_eq!(perplexity, 2048);
}

#[test]
fn test_encode_bit_sequence() {
    // C ground truth sequence
    let mut perplexity: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        enc.encode_bit(false, 1024);
    }
    assert_eq!(perplexity, 2048);
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        enc.encode_bit(true, 1024);
    }
    assert_eq!(perplexity, 4096);
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        enc.encode_bit(false, 100);
    }
    assert_eq!(perplexity, 13017);
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        enc.encode_bit(true, 1900);
    }
    assert_eq!(perplexity, 20780);
}

#[test]
fn test_encode_direct_bits() {
    // C: after 4 bits + direct_bits(0xFF, 8) -> 37164
    let mut perplexity: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perplexity);
        enc.encode_bit(false, 1024);
        enc.encode_bit(true, 1024);
        enc.encode_bit(false, 100);
        enc.encode_bit(true, 1900);
        enc.encode_direct_bits(0xFF, 8);
    }
    assert_eq!(perplexity, 37164);
}

fn main() {}
