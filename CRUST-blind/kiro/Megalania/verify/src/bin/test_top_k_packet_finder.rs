use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::lzma_state::{LZMAProperties, LZMAState};
use Megalania::packet_enumerator::PacketEnumerator;
use Megalania::top_k_packet_finder::TopKPacketFinder;

fn run_with_stack<F: FnOnce() + Send + 'static>(f: F) {
    let builder = std::thread::Builder::new().stack_size(8 * 1024 * 1024);
    let handler = builder.spawn(f).unwrap();
    handler.join().unwrap();
}

#[test]
fn test_find_at_position_0() {
    run_with_stack(|| {
        let data: &[u8] = b"hello hello";
        let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
        let state = LZMAState::new(data, props);
        let pe = PacketEnumerator::new(data);

        let mut finder = TopKPacketFinder::new(10, &pe);
        let mut next_packets = vec![LZMAPacket::literal_packet(); data.len()];
        finder.find(&state, &mut next_packets);

        // Only literal available, and it matches next_packets[0], so 0 candidates
        assert_eq!(finder.count(), 0);
    });
}

#[test]
fn test_find_at_position_6() {
    run_with_stack(|| {
        let data: &[u8] = b"hello hello";
        let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
        let mut state = LZMAState::new(data, props);
        state.position = 6;
        let pe = PacketEnumerator::new(data);

        let mut finder = TopKPacketFinder::new(10, &pe);
        let mut next_packets = vec![LZMAPacket::literal_packet(); data.len()];
        finder.find(&state, &mut next_packets);

        assert!(finder.count() > 0);
        assert!(finder.count() <= 10);

        let mut popped = Vec::new();
        while let Some(p) = finder.pop() {
            popped.push(p);
        }
        assert!(!popped.is_empty());
        for p in &popped {
            assert_eq!(p.packet_type, LZMAPacketType::Match);
            assert_eq!(p.dist, 5);
        }
    });
}

#[test]
fn test_pop_empty() {
    run_with_stack(|| {
        let data: &[u8] = b"hello hello";
        let pe = PacketEnumerator::new(data);
        let mut finder = TopKPacketFinder::new(10, &pe);
        assert_eq!(finder.pop(), None);
    });
}

fn main() {}
