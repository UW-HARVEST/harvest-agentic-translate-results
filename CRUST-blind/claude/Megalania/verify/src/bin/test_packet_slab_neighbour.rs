use Megalania::lzma_state::{LZMAProperties, LZMAState, make_lzma_probability_model};
use Megalania::packet_enumerator::PacketEnumerator;
use Megalania::packet_slab::PacketSlab;
use Megalania::packet_slab_neighbour::PacketSlabNeighbour;
use Megalania::top_k_packet_finder::TopKPacketFinder;

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
fn test_initial_state() {
    let data: &[u8] = b"abcabcabcab";
    let slab = PacketSlab::new(data.len());
    let init_state = make_state(data);
    let neighbour = PacketSlabNeighbour::new(slab, init_state);
    assert_eq!(neighbour.perplexity, 0);
    assert_eq!(neighbour.undo_count(), 0);
}

#[test]
fn test_undo_resets_undo_count() {
    let data: &[u8] = b"abcabcabcab";
    let slab = PacketSlab::new(data.len());
    let init_state = make_state(data);
    let mut neighbour = PacketSlabNeighbour::new(slab, init_state);
    // No undos yet, applying undo should be a no-op
    neighbour.undo();
    assert_eq!(neighbour.undo_count(), 0);
}

#[test]
fn test_generate_runs_and_can_undo() {
    let data: &[u8] = b"abcabcabcab";
    let pe = PacketEnumerator::new(data);
    let mut finder = TopKPacketFinder::new(10, &pe);
    let slab = PacketSlab::new(data.len());
    let init_state = make_state(data);
    let mut neighbour = PacketSlabNeighbour::new(slab, init_state);

    // Try a few iterations; the result is randomized, but should not panic and
    // the undo should always succeed in restoring the original slab content.
    for _ in 0..5 {
        let _ok = neighbour.generate(&mut finder);
        // Whatever the outcome, undo should leave undo_count unchanged after applying.
        neighbour.undo();
        assert_eq!(neighbour.undo_count(), 0);
        // Reset perplexity for next iteration
        neighbour.perplexity = 0;
    }
}

fn main() {}
