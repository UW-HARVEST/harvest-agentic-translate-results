use Megalania::lzma_packet::LZMAPacketType;
use Megalania::lzma_state::{LZMAProperties, LZMAState};
use Megalania::packet_enumerator::PacketEnumerator;
use std::cell::RefCell;

#[derive(Default)]
struct Counts {
    literal: u32,
    match_: u32,
    short_rep: u32,
    long_rep: u32,
}

fn count_packets(data: &[u8], pos: usize, dists: [u32; 4]) -> Counts {
    let props = LZMAProperties { lc: 0, lp: 0, pb: 0 };
    let mut s = LZMAState::new(data, props);
    s.position = pos;
    s.dists = dists;

    let pe = PacketEnumerator::new(data);
    let counts = RefCell::new(Counts::default());
    pe.for_each(&s, |_state, packet| {
        let mut c = counts.borrow_mut();
        match packet.packet_type {
            LZMAPacketType::Literal => c.literal += 1,
            LZMAPacketType::Match => c.match_ += 1,
            LZMAPacketType::ShortRep => c.short_rep += 1,
            LZMAPacketType::LongRep => c.long_rep += 1,
            _ => (),
        }
    });
    counts.into_inner()
}

#[test]
fn test_hello_hello_pos6_no_match_dist() {
    // C: literal=1 match=4 short_rep=0 long_rep=0
    let data = b"hello hello";
    let c = count_packets(data, 6, [0, 0, 0, 0]);
    assert_eq!(c.literal, 1);
    assert_eq!(c.match_, 4);
    assert_eq!(c.short_rep, 0);
    assert_eq!(c.long_rep, 0);
}

#[test]
fn test_hello_hello_pos6_match_dist_5() {
    // C: literal=1 match=4 short_rep=1 long_rep=4
    let data = b"hello hello";
    let c = count_packets(data, 6, [5, 0, 0, 0]);
    assert_eq!(c.literal, 1);
    assert_eq!(c.match_, 4);
    assert_eq!(c.short_rep, 1);
    assert_eq!(c.long_rep, 4);
}

#[test]
fn test_pos_zero_returns_only_literal() {
    let data = b"hello hello";
    let c = count_packets(data, 0, [0, 0, 0, 0]);
    assert_eq!(c.literal, 1);
    assert_eq!(c.match_, 0);
    assert_eq!(c.short_rep, 0);
    assert_eq!(c.long_rep, 0);
}

#[test]
fn test_aaaaa_pos2_all_zero_dists() {
    // C: literal=1 match=4 short_rep=1 long_rep=8
    let data = b"aaaaa";
    let c = count_packets(data, 2, [0, 0, 0, 0]);
    assert_eq!(c.literal, 1);
    assert_eq!(c.match_, 4);
    assert_eq!(c.short_rep, 1);
    assert_eq!(c.long_rep, 8);
}

#[test]
fn test_aaaaa_pos3_dists_0_1_0_0() {
    // C: literal=1 match=3 short_rep=1 long_rep=4
    let data = b"aaaaa";
    let c = count_packets(data, 3, [0, 1, 0, 0]);
    assert_eq!(c.literal, 1);
    assert_eq!(c.match_, 3);
    assert_eq!(c.short_rep, 1);
    assert_eq!(c.long_rep, 4);
}

#[test]
fn test_memory_usage_is_positive() {
    let usage = PacketEnumerator::memory_usage(100);
    assert!(usage > 0);
}

#[test]
fn test_data_field_set() {
    let data = b"abcdef";
    let pe = PacketEnumerator::new(data);
    assert_eq!(pe.data, data);
}

fn main() {}
