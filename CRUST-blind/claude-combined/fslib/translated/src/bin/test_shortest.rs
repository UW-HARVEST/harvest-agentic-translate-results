use fslib::fst::Fst;
use fslib::shortest::ShortestPath;

#[test]
fn test_shortest_chooses_smaller_weight_path() {
    let mut fst = Fst::new();
    for _ in 0..5 {
        fst.add_state();
    }
    fst.add_arc(0, 1, 1, 1, 1.0);
    fst.add_arc(1, 2, 2, 2, 2.0);
    fst.add_arc(0, 3, 3, 3, 0.5);
    fst.add_arc(3, 2, 4, 4, 0.5);
    fst.set_final(2, 0.0);
    fst.start = 0;

    let mut path = Fst::new();
    ShortestPath::find_shortest_path(&fst, &mut path);

    assert_eq!(path.n_states, 3);
    assert_eq!(path.states[2].final_state, true);
    assert_eq!(path.states[0].n_arcs, 1);
    let a0 = &path.states[0].arcs[0];
    assert_eq!(a0.ilabel, 3);
    assert_eq!(a0.olabel, 3);
    assert_eq!(a0.weight, 0.5);
    let a1 = &path.states[1].arcs[0];
    assert_eq!(a1.ilabel, 4);
    assert_eq!(a1.olabel, 4);
    assert_eq!(a1.weight, 0.5);
}

fn main() {}
