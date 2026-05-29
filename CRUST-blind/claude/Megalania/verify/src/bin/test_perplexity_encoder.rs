use Megalania::encoder_interface::EncoderInterface;
use Megalania::perplexity_encoder::PerplexityEncoder;

#[test]
fn test_encode_bit_zero_with_init_prob() {
    // C: encode_bit(0, 1024) -> perp=2048
    let mut perp = 0u64;
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_bit(false, 1024);
    }
    assert_eq!(perp, 2048);
}

#[test]
fn test_encode_bit_one_with_init_prob() {
    // C: encode_bit(1, 1024) -> perp=2048 (using LOG2_LOOKUP[2048-1024])
    let mut perp = 0u64;
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_bit(true, 1024);
    }
    assert_eq!(perp, 2048);
}

#[test]
fn test_encode_bit_zero_prob_zero() {
    // C: encode_bit(0, 0) -> perp += LOG2_LOOKUP[0] = 0
    let mut perp = 4096u64;
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_bit(false, 0);
    }
    assert_eq!(perp, 4096);
}

#[test]
fn test_encode_bit_one_prob_2048() {
    // C: encode_bit(1, 2048) -> perp += LOG2_LOOKUP[0] = 0
    let mut perp = 4096u64;
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_bit(true, 2048);
    }
    assert_eq!(perp, 4096);
}

#[test]
fn test_encode_direct_bits() {
    // C: encode_direct_bits(*, 8) adds 8 << 11 = 16384
    let mut perp = 4096u64;
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_direct_bits(0xFF, 8);
    }
    assert_eq!(perp, 4096 + 16384);
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_direct_bits(0, 4);
    }
    assert_eq!(perp, 4096 + 16384 + (4 << 11));
}

#[test]
fn test_full_sequence() {
    // Combine all of the above operations from the C output.
    let mut perp = 0u64;
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_bit(false, 1024);
    }
    assert_eq!(perp, 2048);
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_bit(true, 1024);
    }
    assert_eq!(perp, 4096);
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_bit(false, 0);
    }
    assert_eq!(perp, 4096);
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_bit(true, 2048);
    }
    assert_eq!(perp, 4096);
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_direct_bits(0xFF, 8);
    }
    assert_eq!(perp, 20480);
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        enc.encode_direct_bits(0, 4);
    }
    assert_eq!(perp, 28672);
}

fn main() {}
