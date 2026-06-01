use fslib::fst::Fst;

#[test]
fn test_create() {
    let fst = Fst::new();
    assert_eq!(fst.sr_type, 0);
    assert_eq!(fst.n_states, 0);
    assert_eq!(fst.start, 0);
    assert_eq!(fst.flags, 0);
    assert_eq!(fst.states.len(), 0);
}

#[test]
fn test_add_state() {
    let mut fst = Fst::new();
    let s0 = fst.add_state();
    let s1 = fst.add_state();
    let s2 = fst.add_state();
    assert_eq!(s0, 0);
    assert_eq!(s1, 1);
    assert_eq!(s2, 2);
    assert_eq!(fst.n_states, 3);
}

#[test]
fn test_add_arc() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.add_arc(0, 2, 3, 4, 1.5);
    fst.add_arc(1, 2, 5, 6, 2.5);
    assert_eq!(fst.states[0].n_arcs, 2);
    assert_eq!(fst.states[1].n_arcs, 1);
    assert_eq!(fst.states[2].n_arcs, 0);
    assert_eq!(fst.get_n_arcs(), 3);
    let a = &fst.states[0].arcs[0];
    assert_eq!(a.state, 1);
    assert_eq!(a.ilabel, 1);
    assert_eq!(a.olabel, 2);
    assert_eq!(a.weight, 0.5);
    let a1 = &fst.states[0].arcs[1];
    assert_eq!(a1.state, 2);
    assert_eq!(a1.ilabel, 3);
    assert_eq!(a1.olabel, 4);
    assert_eq!(a1.weight, 1.5);
}

#[test]
fn test_set_final() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.set_final(1, 2.5);
    assert_eq!(fst.states[0].final_state, false);
    assert_eq!(fst.states[1].final_state, true);
    assert_eq!(fst.states[1].weight, 2.5);
}

#[test]
fn test_relabel_input() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.relabel(1, 100, 0); // input direction
    let a = &fst.states[0].arcs[0];
    assert_eq!(a.ilabel, 100);
    assert_eq!(a.olabel, 2);
}

#[test]
fn test_relabel_output() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.relabel(2, 200, 1); // output direction
    let a = &fst.states[0].arcs[0];
    assert_eq!(a.ilabel, 1);
    assert_eq!(a.olabel, 200);
}

#[test]
fn test_arc_sort_output() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.add_arc(0, 2, 3, 4, 1.5);
    fst.arc_sort(1); // outer/output sort
    assert_eq!(fst.flags, 2); // OSORT
    let a = &fst.states[0].arcs[0];
    assert_eq!(a.olabel, 2);
    let a1 = &fst.states[0].arcs[1];
    assert_eq!(a1.olabel, 4);
}

#[test]
fn test_arc_sort_input() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 5, 1, 0.5);
    fst.add_arc(0, 1, 3, 2, 0.5);
    fst.add_arc(0, 1, 1, 3, 0.5);
    fst.arc_sort(0); // input sort
    assert_eq!(fst.flags, 1); // ISORT
    assert_eq!(fst.states[0].arcs[0].ilabel, 1);
    assert_eq!(fst.states[0].arcs[1].ilabel, 3);
    assert_eq!(fst.states[0].arcs[2].ilabel, 5);
}

#[test]
fn test_stack() {
    let mut a = Fst::new();
    a.add_state();
    a.add_state();
    a.add_arc(0, 1, 1, 1, 0.5);
    a.set_final(1, 0.0);

    let mut b = Fst::new();
    b.add_state();
    b.add_state();
    b.add_arc(0, 1, 2, 2, 1.0);
    b.set_final(1, 0.0);

    a.stack(&b);
    assert_eq!(a.n_states, 4);
    assert_eq!(a.states[2].n_arcs, 1);
    let arc = &a.states[2].arcs[0];
    assert_eq!(arc.state, 3);
    assert_eq!(arc.ilabel, 2);
    assert_eq!(arc.olabel, 2);
}

#[test]
fn test_reverse() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 1, 0.5);
    fst.add_arc(1, 2, 2, 2, 1.0);
    fst.set_final(2, 0.0);
    fst.reverse();
    assert_eq!(fst.n_states, 3);
    assert_eq!(fst.start, 2);
    assert_eq!(fst.states[0].final_state, true);
    assert_eq!(fst.states[1].final_state, false);
    assert_eq!(fst.states[2].final_state, false);
    assert_eq!(fst.states[0].n_arcs, 0);
    assert_eq!(fst.states[1].n_arcs, 1);
    assert_eq!(fst.states[2].n_arcs, 1);
    let a = &fst.states[1].arcs[0];
    assert_eq!(a.state, 0);
    assert_eq!(a.ilabel, 1);
    assert_eq!(a.olabel, 1);
    let a2 = &fst.states[2].arcs[0];
    assert_eq!(a2.state, 1);
    assert_eq!(a2.ilabel, 2);
    assert_eq!(a2.olabel, 2);
}

#[test]
fn test_io_roundtrip() {
    use std::fs;
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.add_arc(1, 2, 3, 4, 1.0);
    fst.set_final(2, 0.0);
    let path = "/tmp/_rust_test_fst.bin";
    fst.fwrite(path).unwrap();
    let mut fst2 = Fst::new();
    fst2.fread(path).unwrap();
    assert_eq!(fst2.n_states, 3);
    assert_eq!(fst2.sr_type, 0);
    assert_eq!(fst2.states[0].n_arcs, 1);
    assert_eq!(fst2.states[1].n_arcs, 1);
    let a = &fst2.states[0].arcs[0];
    assert_eq!(a.state, 1);
    assert_eq!(a.ilabel, 1);
    assert_eq!(a.olabel, 2);
    assert_eq!(a.weight, 0.5);
    assert_eq!(fst2.states[2].final_state, true);
    assert_eq!(fst2.states[2].weight, 0.0);
    let _ = fs::remove_file(path);
}

fn main() {}
