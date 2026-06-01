use Megalania::lzma_packet::LZMAPacket;
use Megalania::lzma_packet_encoder::lzma_encode_packet;
use Megalania::lzma_state::{LZMAProperties, LZMAState};
use Megalania::perplexity_encoder::PerplexityEncoder;

fn make_state<'a>(data: &'a [u8]) -> LZMAState<'a> {
    let props = LZMAProperties { lc: 0, lp: 0, pb: 0 };
    LZMAState::new(data, props)
}

#[test]
fn test_encode_literal_at_pos_0() {
    let data = vec![0u8; 8];
    let mut s = make_state(&data);
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::literal_packet());
    }
    assert_eq!(perp, 18432);
    assert_eq!(s.position, 1);
    assert_eq!(s.ctx_state, 0);
}

#[test]
fn test_encode_literal_at_pos_0_byte_65() {
    let data = vec![65u8, 0, 0, 0, 0, 0, 0, 0];
    let mut s = make_state(&data);
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::literal_packet());
    }
    assert_eq!(perp, 18432);
    assert_eq!(s.position, 1);
    assert_eq!(s.ctx_state, 0);
}

#[test]
fn test_encode_literal_at_pos_0_byte_255() {
    let data = vec![255u8, 0, 0, 0, 0, 0, 0, 0];
    let mut s = make_state(&data);
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::literal_packet());
    }
    assert_eq!(perp, 18432);
    assert_eq!(s.position, 1);
    assert_eq!(s.ctx_state, 0);
}

#[test]
fn test_encode_match_dist_5_len_3() {
    let mut data = vec![0u8; 100];
    for i in 0..100 {
        data[i] = i as u8;
    }
    let mut s = make_state(&data);
    s.position = 5;
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::match_packet(5, 3));
    }
    assert_eq!(perp, 26624);
    assert_eq!(s.position, 8);
    assert_eq!(s.ctx_state, 7);
    assert_eq!(s.dists, [5, 0, 0, 0]);
}

#[test]
fn test_encode_match_dist_1_len_2() {
    let mut data = vec![0u8; 100];
    for i in 0..100 {
        data[i] = i as u8;
    }
    let mut s = make_state(&data);
    s.position = 1;
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::match_packet(1, 2));
    }
    assert_eq!(perp, 24576);
    assert_eq!(s.position, 3);
    assert_eq!(s.ctx_state, 7);
    assert_eq!(s.dists, [1, 0, 0, 0]);
}

#[test]
fn test_encode_match_dist_100_len_4() {
    let mut data = vec![0u8; 100];
    for i in 0..100 {
        data[i] = i as u8;
    }
    let mut s = make_state(&data);
    s.position = 50;
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::match_packet(100, 4));
    }
    assert_eq!(perp, 34816);
    assert_eq!(s.position, 54);
    assert_eq!(s.ctx_state, 7);
    assert_eq!(s.dists, [100, 0, 0, 0]);
}

#[test]
fn test_encode_short_rep_at_5() {
    let data = vec![0u8; 16];
    let mut s = make_state(&data);
    s.position = 5;
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::short_rep_packet());
    }
    assert_eq!(perp, 8192);
    assert_eq!(s.position, 6);
    assert_eq!(s.ctx_state, 9);
}

#[test]
fn test_encode_short_rep_at_0() {
    let data = vec![0u8; 16];
    let mut s = make_state(&data);
    s.position = 0;
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::short_rep_packet());
    }
    assert_eq!(perp, 8192);
    assert_eq!(s.position, 1);
    assert_eq!(s.ctx_state, 9);
}

#[test]
fn test_encode_long_rep_index_0() {
    let mut data = vec![0u8; 100];
    for i in 0..100 {
        data[i] = i as u8;
    }
    let mut s = make_state(&data);
    s.position = 10;
    s.dists = [1, 5, 10, 20];
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::long_rep_packet(0, 5));
    }
    assert_eq!(perp, 16384);
    assert_eq!(s.position, 15);
    assert_eq!(s.ctx_state, 8);
    assert_eq!(s.dists, [1, 5, 10, 20]);
}

#[test]
fn test_encode_long_rep_index_1() {
    let mut data = vec![0u8; 100];
    for i in 0..100 {
        data[i] = i as u8;
    }
    let mut s = make_state(&data);
    s.position = 10;
    s.dists = [1, 5, 10, 20];
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::long_rep_packet(1, 3));
    }
    assert_eq!(perp, 16384);
    assert_eq!(s.position, 13);
    assert_eq!(s.ctx_state, 8);
    assert_eq!(s.dists, [5, 1, 10, 20]);
}

#[test]
fn test_encode_long_rep_index_2() {
    let mut data = vec![0u8; 100];
    for i in 0..100 {
        data[i] = i as u8;
    }
    let mut s = make_state(&data);
    s.position = 20;
    s.dists = [1, 5, 10, 20];
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::long_rep_packet(2, 7));
    }
    assert_eq!(perp, 18432);
    assert_eq!(s.position, 27);
    assert_eq!(s.ctx_state, 8);
    assert_eq!(s.dists, [10, 1, 5, 20]);
}

#[test]
fn test_encode_long_rep_index_3() {
    let mut data = vec![0u8; 100];
    for i in 0..100 {
        data[i] = i as u8;
    }
    let mut s = make_state(&data);
    s.position = 30;
    s.dists = [1, 5, 10, 20];
    let mut perp: u64 = 0;
    {
        let mut enc = PerplexityEncoder::new(&mut perp);
        lzma_encode_packet(&mut s, &mut enc, LZMAPacket::long_rep_packet(3, 4));
    }
    assert_eq!(perp, 18432);
    assert_eq!(s.position, 34);
    assert_eq!(s.ctx_state, 8);
    assert_eq!(s.dists, [20, 1, 5, 10]);
}

fn main() {}
