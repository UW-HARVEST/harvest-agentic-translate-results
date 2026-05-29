use fslib::fst::Fst;
use fslib::shortest::{states_hash, states_key_eq, ShortestPath};

#[test]
fn test_shortest_linear_path() {
    // 0 -> 1 -> 2 -> 3, 3 final
    let mut f = Fst::new();
    for _ in 0..4 {
        f.add_state();
    }
    f.start = 0;
    f.add_arc(0, 1, 1, 1, 1.0);
    f.add_arc(1, 2, 2, 2, 2.0);
    f.add_arc(2, 3, 3, 3, 3.0);
    f.set_final(3, 0.0);
    let mut path = Fst::new();
    ShortestPath::find_shortest_path(&f, &mut path);
    // Per probe: n_states=4
    assert_eq!(path.n_states, 4);
    let s0 = &path.states[0];
    assert_eq!(s0.n_arcs, 1);
    assert_eq!(s0.arcs[0].state, 1);
    assert_eq!(s0.arcs[0].ilabel, 1);
    assert_eq!(s0.arcs[0].olabel, 1);
    assert_eq!(s0.arcs[0].weight, 1.0);
    let s1 = &path.states[1];
    assert_eq!(s1.arcs[0].state, 2);
    assert_eq!(s1.arcs[0].ilabel, 2);
    assert_eq!(s1.arcs[0].olabel, 2);
    assert_eq!(s1.arcs[0].weight, 2.0);
    let s2 = &path.states[2];
    assert_eq!(s2.arcs[0].state, 3);
    assert_eq!(s2.arcs[0].ilabel, 3);
    assert_eq!(s2.arcs[0].olabel, 3);
    assert_eq!(s2.arcs[0].weight, 3.0);
    let s3 = &path.states[3];
    assert_eq!(s3.final_state, true);
    assert_eq!(s3.weight, 0.0);
    assert_eq!(s3.n_arcs, 0);
}

#[test]
fn test_shortest_picks_shortest() {
    // Two paths from 0 to 3:
    //   0->1->3 (cost 2+3=5)
    //   0->2->3 (cost 1+2=3) <- shorter
    let mut f = Fst::new();
    for _ in 0..4 {
        f.add_state();
    }
    f.start = 0;
    f.add_arc(0, 1, 1, 1, 2.0);
    f.add_arc(0, 2, 2, 2, 1.0);
    f.add_arc(1, 3, 3, 3, 3.0);
    f.add_arc(2, 3, 4, 4, 2.0);
    f.set_final(3, 0.0);
    let mut path = Fst::new();
    ShortestPath::find_shortest_path(&f, &mut path);
    assert_eq!(path.n_states, 3);
    // Path is 0 -> 1 -> 2 (in path FST), with arcs (2,2,1.0) then (4,4,2.0)
    let s0 = &path.states[0];
    assert_eq!(s0.arcs[0].state, 1);
    assert_eq!(s0.arcs[0].ilabel, 2);
    assert_eq!(s0.arcs[0].olabel, 2);
    assert_eq!(s0.arcs[0].weight, 1.0);
    let s1 = &path.states[1];
    assert_eq!(s1.arcs[0].state, 2);
    assert_eq!(s1.arcs[0].ilabel, 4);
    assert_eq!(s1.arcs[0].olabel, 4);
    assert_eq!(s1.arcs[0].weight, 2.0);
    let s2 = &path.states[2];
    assert_eq!(s2.final_state, true);
    assert_eq!(s2.n_arcs, 0);
}

#[test]
fn test_shortest_no_path_to_final() {
    // FST with final state unreachable
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.start = 0;
    f.add_arc(0, 1, 1, 1, 1.0);
    f.set_final(2, 0.0); // 2 unreachable
    let mut path = Fst::new();
    ShortestPath::find_shortest_path(&f, &mut path);
    // No path -> empty path
    assert_eq!(path.n_states, 0);
}

#[test]
fn test_shortest_path_new() {
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    let sp = ShortestPath::new(&f);
    assert_eq!(sp.weights.len(), 3);
    // Initialized to sr.zero (tropical zero = f32::MAX)
    for w in sp.weights.iter() {
        assert_eq!(*w, f32::MAX);
    }
    assert_eq!(sp.backtrack.len(), 3);
    for b in sp.backtrack.iter() {
        assert!(b.is_none());
    }
}

#[test]
fn test_states_hash_key_eq() {
    assert_eq!(states_hash(&5u32), 5u64);
    assert_eq!(states_hash(&100u32), 100u64);
    assert!(states_key_eq(&5u32, &5u32));
    assert!(!states_key_eq(&5u32, &10u32));
}

fn main() {}
