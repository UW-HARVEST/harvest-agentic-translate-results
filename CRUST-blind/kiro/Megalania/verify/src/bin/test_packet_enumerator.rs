use Megalania::lzma_packet::{LZMAPacket, LZMAPacketType};
use Megalania::lzma_state::{LZMAProperties, LZMAState};
use Megalania::packet_enumerator::PacketEnumerator;

fn run_with_stack<F: FnOnce() + Send + 'static>(f: F) {
    let builder = std::thread::Builder::new().stack_size(8 * 1024 * 1024);
    let handler = builder.spawn(f).unwrap();
    handler.join().unwrap();
}

#[test]
fn test_position_0() {
    run_with_stack(|| {
        let data: &[u8] = b"hello hello";
        let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
        let state = LZMAState::new(data, props);
        let pe = PacketEnumerator::new(data);

        let mut packets: Vec<LZMAPacket> = Vec::new();
        pe.for_each(&state, |_state, packet| {
            packets.push(packet);
        });

        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].packet_type, LZMAPacketType::Literal);
        assert_eq!(packets[0].dist, 0);
        assert_eq!(packets[0].len, 1);
    });
}

#[test]
fn test_position_6() {
    run_with_stack(|| {
        let data: &[u8] = b"hello hello";
        let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
        let mut state = LZMAState::new(data, props);
        state.position = 6;
        let pe = PacketEnumerator::new(data);

        let mut packets: Vec<LZMAPacket> = Vec::new();
        pe.for_each(&state, |_state, packet| {
            packets.push(packet);
        });

        assert_eq!(packets.len(), 5);

        assert_eq!(packets[0].packet_type, LZMAPacketType::Literal);
        assert_eq!(packets[0].dist, 0);
        assert_eq!(packets[0].len, 1);

        assert_eq!(packets[1].packet_type, LZMAPacketType::Match);
        assert_eq!(packets[1].dist, 5);
        assert_eq!(packets[1].len, 2);

        assert_eq!(packets[2].packet_type, LZMAPacketType::Match);
        assert_eq!(packets[2].dist, 5);
        assert_eq!(packets[2].len, 3);

        assert_eq!(packets[3].packet_type, LZMAPacketType::Match);
        assert_eq!(packets[3].dist, 5);
        assert_eq!(packets[3].len, 4);

        assert_eq!(packets[4].packet_type, LZMAPacketType::Match);
        assert_eq!(packets[4].dist, 5);
        assert_eq!(packets[4].len, 5);
    });
}

fn main() {}
