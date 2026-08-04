use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};

#[test]
fn test_literal_packet() {
    let p = LZMAPacket::literal_packet();
    assert_eq!(p.packet_type, LZMAPacketType::Literal);
    assert_eq!(p.dist, 0);
    assert_eq!(p.len, 1);
}

#[test]
fn test_match_packet() {
    let p = LZMAPacket::match_packet(42, 5);
    assert_eq!(p.packet_type, LZMAPacketType::Match);
    assert_eq!(p.dist, 42);
    assert_eq!(p.len, 5);
}

#[test]
fn test_short_rep_packet() {
    let p = LZMAPacket::short_rep_packet();
    assert_eq!(p.packet_type, LZMAPacketType::ShortRep);
    assert_eq!(p.dist, 0);
    assert_eq!(p.len, 1);
}

#[test]
fn test_long_rep_packet() {
    let p = LZMAPacket::long_rep_packet(2, 7);
    assert_eq!(p.packet_type, LZMAPacketType::LongRep);
    assert_eq!(p.dist, 2);
    assert_eq!(p.len, 7);
}

#[test]
fn test_cmp_equal_literals() {
    let a = LZMAPacket::literal_packet();
    let b = LZMAPacket::literal_packet();
    assert!(LZMAPacket::cmp(&a, &b));
}

#[test]
fn test_cmp_different_types() {
    let a = LZMAPacket::literal_packet();
    let b = LZMAPacket::short_rep_packet();
    // Same dist=0, len=1 but type differs
    assert!(!LZMAPacket::cmp(&a, &b));
}

#[test]
fn test_cmp_match_packets_equal() {
    let a = LZMAPacket::match_packet(10, 5);
    let b = LZMAPacket::match_packet(10, 5);
    assert!(LZMAPacket::cmp(&a, &b));
}

#[test]
fn test_cmp_match_diff_dist() {
    let a = LZMAPacket::match_packet(10, 5);
    let b = LZMAPacket::match_packet(11, 5);
    assert!(!LZMAPacket::cmp(&a, &b));
}

#[test]
fn test_cmp_match_diff_len() {
    let a = LZMAPacket::match_packet(10, 5);
    let b = LZMAPacket::match_packet(10, 6);
    assert!(!LZMAPacket::cmp(&a, &b));
}

#[test]
fn test_long_rep_packet_fields() {
    let p = LZMAPacket::long_rep_packet(0, 2);
    assert_eq!(p.packet_type, LZMAPacketType::LongRep);
    assert_eq!(p.dist, 0);
    assert_eq!(p.len, 2);
    let p2 = LZMAPacket::long_rep_packet(3, 100);
    assert_eq!(p2.dist, 3);
    assert_eq!(p2.len, 100);
}

fn main() {}
