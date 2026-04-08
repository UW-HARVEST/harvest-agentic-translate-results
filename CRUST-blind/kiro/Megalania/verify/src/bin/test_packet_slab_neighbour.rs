use Megalania::lzma_packet::LZMAPacketType;
use Megalania::lzma_state::{LZMAProperties, LZMAState};
use Megalania::packet_slab::PacketSlab;
use Megalania::packet_slab_neighbour::PacketSlabNeighbour;

fn run_with_stack<F: FnOnce() + Send + 'static>(f: F) {
    let builder = std::thread::Builder::new().stack_size(8 * 1024 * 1024);
    let handler = builder.spawn(f).unwrap();
    handler.join().unwrap();
}

#[test]
fn test_new_neighbour() {
    run_with_stack(|| {
        let data: &[u8] = b"hello hello";
        let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
        let state = LZMAState::new(data, props);
        let slab = PacketSlab::new(data.len());

        let neighbour = PacketSlabNeighbour::new(slab, state);
        assert_eq!(neighbour.perplexity, 0);
        assert_eq!(neighbour.undo_count(), 0);
    });
}

#[test]
fn test_undo_empty() {
    run_with_stack(|| {
        let data: &[u8] = b"hello hello";
        let props = LZMAProperties { lc: 3, lp: 0, pb: 2 };
        let state = LZMAState::new(data, props);
        let slab = PacketSlab::new(data.len());

        let mut neighbour = PacketSlabNeighbour::new(slab, state);
        neighbour.undo();
        assert_eq!(neighbour.undo_count(), 0);

        for p in neighbour.slab.packets().iter() {
            assert_eq!(p.packet_type, LZMAPacketType::Literal);
        }
    });
}

fn main() {}
