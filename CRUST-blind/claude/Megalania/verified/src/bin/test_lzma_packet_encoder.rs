use Megalania::encoder_interface::EncoderInterface;
use Megalania::lzma_packet::LZMAPacket;
use Megalania::lzma_packet_encoder::lzma_encode_packet;
use Megalania::lzma_state::{LZMAProperties, LZMAState, make_lzma_probability_model};
use Megalania::perplexity_encoder::PerplexityEncoder;

fn make_state<'a>(data: &'a [u8]) -> LZMAState<'a> {
    LZMAState {
        data,
        properties: LZMAProperties { lc: 0, lp: 0, pb: 0 },
        ctx_state: 0,
        dists: [0; 4],
        probs: make_lzma_probability_model(),
        position: 0,
    }
}

#[test]
fn test_literal_at_pos_0() {
    // C output: literal at pos 0: perp=18432, position=1, ctx_state=0
    let data: &[u8] = b"abcabcab";
    let mut s = make_state(data);
    let mut perp = 0u64;
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        let p = LZMAPacket::literal_packet();
        lzma_encode_packet(&mut s, &mut enc as &mut dyn EncoderInterface, p);
    }
    assert_eq!(perp, 18432);
    assert_eq!(s.position, 1);
    assert_eq!(s.ctx_state, 0);
}

#[test]
fn test_three_literals_then_match() {
    // C output:
    //   after 3 literals: perp=53603, pos=3, ctx=0
    //   after match(2,3): perp=78460, pos=6, ctx=7, dists=[2,0,0,0]
    let data: &[u8] = b"abcabcab";
    let mut s = make_state(data);
    let mut perp = 0u64;
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        for _ in 0..3 {
            lzma_encode_packet(
                &mut s,
                &mut enc as &mut dyn EncoderInterface,
                LZMAPacket::literal_packet(),
            );
        }
    }
    assert_eq!(perp, 53603);
    assert_eq!(s.position, 3);
    assert_eq!(s.ctx_state, 0);

    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        lzma_encode_packet(
            &mut s,
            &mut enc as &mut dyn EncoderInterface,
            LZMAPacket::match_packet(2, 3),
        );
    }
    assert_eq!(perp, 78460);
    assert_eq!(s.position, 6);
    assert_eq!(s.ctx_state, 7);
    assert_eq!(s.dists, [2, 0, 0, 0]);
}

#[test]
fn test_short_rep_at_pos_3() {
    // C output: short_rep at pos 3: perp=8192, pos=4, ctx=9
    let data: &[u8] = b"abcabcab";
    let mut s = make_state(data);
    s.position = 3;
    s.dists[0] = 2;
    let mut perp = 0u64;
    {
        let mut enc = PerplexityEncoder { perplexity: &mut perp };
        lzma_encode_packet(
            &mut s,
            &mut enc as &mut dyn EncoderInterface,
            LZMAPacket::short_rep_packet(),
        );
    }
    assert_eq!(perp, 8192);
    assert_eq!(s.position, 4);
    assert_eq!(s.ctx_state, 9);
}

fn main() {}
