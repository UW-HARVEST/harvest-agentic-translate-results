use fslib::fst::Fst;
use fslib::sort::fst_arc_sort;

#[test]
fn test_sort_by_input() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 5, 1, 0.5);
    fst.add_arc(0, 1, 3, 2, 0.5);
    fst.add_arc(0, 1, 1, 3, 0.5);
    fst_arc_sort(&mut fst, false);
    assert_eq!(fst.flags, 1); // ISORT
    assert_eq!(fst.states[0].arcs[0].ilabel, 1);
    assert_eq!(fst.states[0].arcs[1].ilabel, 3);
    assert_eq!(fst.states[0].arcs[2].ilabel, 5);
}

#[test]
fn test_sort_by_output() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 5, 0.5);
    fst.add_arc(0, 1, 2, 3, 0.5);
    fst.add_arc(0, 1, 3, 1, 0.5);
    fst_arc_sort(&mut fst, true);
    assert_eq!(fst.flags, 2); // OSORT
    assert_eq!(fst.states[0].arcs[0].olabel, 1);
    assert_eq!(fst.states[0].arcs[1].olabel, 3);
    assert_eq!(fst.states[0].arcs[2].olabel, 5);
}

fn main() {}
