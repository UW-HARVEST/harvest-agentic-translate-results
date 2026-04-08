use Megalania::lzma_packet::LZMAPacket;
use Megalania::lzma_packet_encoder::lzma_encode_packet;
use Megalania::lzma_state::{LZMAProperties, LZMAState};
use Megalania::perplexity_encoder::PerplexityEncoder;

#[test]
fn test_encode_5_literals() {
    // C ground truth: after 5 literals on "hello hello world", perplexity=87706
    let data = b"hello hello world";
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    let mut state = LZMAState::new(data, props);
    let mut perplexity: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perplexity);

    for _ in 0..5 {
        lzma_encode_packet(&mut state, &mut enc, LZMAPacket::literal_packet());
    }
    assert_eq!(state.position, 5);
    assert_eq!(state.ctx_state, 0);
    assert_eq!(perplexity, 87706);
}

#[test]
fn test_encode_6_literals() {
    // C ground truth: after 6 literals, perplexity=105797
    let data = b"hello hello world";
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    let mut state = LZMAState::new(data, props);
    let mut perplexity: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perplexity);

    for _ in 0..6 {
        lzma_encode_packet(&mut state, &mut enc, LZMAPacket::literal_packet());
    }
    assert_eq!(state.position, 6);
    assert_eq!(state.ctx_state, 0);
    assert_eq!(perplexity, 105797);
}

#[test]
fn test_encode_literals_then_match() {
    // C ground truth: after 6 literals + match(5,5), perplexity=132981
    let data = b"hello hello world";
    let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
    let mut state = LZMAState::new(data, props);
    let mut perplexity: u64 = 0;
    let mut enc = PerplexityEncoder::new(&mut perplexity);

    for _ in 0..6 {
        lzma_encode_packet(&mut state, &mut enc, LZMAPacket::literal_packet());
    }
    lzma_encode_packet(&mut state, &mut enc, LZMAPacket::match_packet(5, 5));

    assert_eq!(state.position, 11);
    assert_eq!(state.ctx_state, 7);
    assert_eq!(perplexity, 132981);
    assert_eq!(state.dists[0], 5);
    assert_eq!(state.dists[1], 0);
    assert_eq!(state.dists[2], 0);
    assert_eq!(state.dists[3], 0);
}

fn main() {}
