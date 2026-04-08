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
    let p = LZMAPacket::long_rep_packet(2, 10);
    assert_eq!(p.packet_type, LZMAPacketType::LongRep);
    assert_eq!(p.dist, 2);
    assert_eq!(p.len, 10);
}

#[test]
fn test_cmp_equal() {
    let a = LZMAPacket::literal_packet();
    let b = LZMAPacket::literal_packet();
    assert!(LZMAPacket::cmp(&a, &b));
}

#[test]
fn test_cmp_not_equal() {
    let a = LZMAPacket::literal_packet();
    let b = LZMAPacket::match_packet(42, 5);
    assert!(!LZMAPacket::cmp(&a, &b));
}

#[test]
fn test_packet_type_values() {
    assert_eq!(LZMAPacketType::Invalid as u32, 0);
    assert_eq!(LZMAPacketType::Literal as u32, 1);
    assert_eq!(LZMAPacketType::Match as u32, 2);
    assert_eq!(LZMAPacketType::ShortRep as u32, 3);
    assert_eq!(LZMAPacketType::LongRep as u32, 4);
}

fn main() {}
