use fslib::compile::{add_arc, add_final, compile_str_internal, trn, trt};
use fslib::fst::Fst;
use fslib::symt::SymTable;

#[test]
fn test_trn_valid_number() {
    let st = SymTable::new();
    assert_eq!(trn("0", &st), 0);
    assert_eq!(trn("1", &st), 1);
    assert_eq!(trn("42", &st), 42);
    assert_eq!(trn("12345", &st), 12345);
}

#[test]
fn test_trn_invalid_returns_max() {
    let st = SymTable::new();
    assert_eq!(trn("abc", &st), usize::MAX);
    assert_eq!(trn("12abc", &st), usize::MAX);
    assert_eq!(trn("", &st), usize::MAX);
    assert_eq!(trn("-5", &st), usize::MAX);
}

#[test]
fn test_trt_lookup() {
    let mut st = SymTable::new();
    st.add(0, "<eps>");
    st.add(1, "hello");
    st.add(2, "world");
    assert_eq!(trt("<eps>", &st), 0);
    assert_eq!(trt("hello", &st), 1);
    assert_eq!(trt("world", &st), 2);
    assert_eq!(trt("missing", &st), usize::MAX);
}

#[test]
fn test_add_arc_extends_states() {
    let mut f = Fst::new();
    add_arc(&mut f, 0, 5, 1, 2, 0.5);
    assert_eq!(f.n_states, 6); // states 0..5 created
    assert_eq!(f.states[0].n_arcs, 1);
    let arc = &f.states[0].arcs[0];
    assert_eq!(arc.state, 5);
    assert_eq!(arc.ilabel, 1);
    assert_eq!(arc.olabel, 2);
    assert_eq!(arc.weight, 0.5);
}

#[test]
fn test_add_final_extends_states() {
    let mut f = Fst::new();
    add_final(&mut f, 3, 0.75);
    assert_eq!(f.n_states, 4);
    let s = &f.states[3];
    assert_eq!(s.final_state, true);
    assert_eq!(s.weight, 0.75);
}

#[test]
fn test_compile_str_arc_with_weight() {
    let mut f = Fst::new();
    compile_str_internal(&mut f, "0\t1\t1\t2\t0.5\n");
    assert_eq!(f.n_states, 2);
    let arc = &f.states[0].arcs[0];
    assert_eq!(arc.state, 1);
    assert_eq!(arc.ilabel, 1);
    assert_eq!(arc.olabel, 2);
    assert_eq!(arc.weight, 0.5);
}

#[test]
fn test_compile_str_arc_no_weight() {
    // Without weight, uses sr.one (tropical, default = 0)
    let mut f = Fst::new();
    compile_str_internal(&mut f, "0\t1\t3\t4\n");
    assert_eq!(f.n_states, 2);
    let arc = &f.states[0].arcs[0];
    assert_eq!(arc.state, 1);
    assert_eq!(arc.ilabel, 3);
    assert_eq!(arc.olabel, 4);
    assert_eq!(arc.weight, 0.0); // tropical one
}

#[test]
fn test_compile_str_final_with_weight() {
    let mut f = Fst::new();
    compile_str_internal(&mut f, "2\t1.5\n");
    assert_eq!(f.n_states, 3);
    assert_eq!(f.states[2].final_state, true);
    assert_eq!(f.states[2].weight, 1.5);
}

#[test]
fn test_compile_str_final_no_weight() {
    let mut f = Fst::new();
    compile_str_internal(&mut f, "2\n");
    assert_eq!(f.n_states, 3);
    assert_eq!(f.states[2].final_state, true);
    assert_eq!(f.states[2].weight, 0.0); // tropical one
}

#[test]
fn test_compile_str_combined() {
    let mut f = Fst::new();
    compile_str_internal(&mut f, "0\t1\t1\t2\t0.5\n1\t2\t3\t4\n2\t1.5\n");
    assert_eq!(f.n_states, 3);
    let s0 = &f.states[0];
    assert_eq!(s0.arcs[0].state, 1);
    assert_eq!(s0.arcs[0].ilabel, 1);
    assert_eq!(s0.arcs[0].olabel, 2);
    assert_eq!(s0.arcs[0].weight, 0.5);
    let s1 = &f.states[1];
    assert_eq!(s1.arcs[0].state, 2);
    assert_eq!(s1.arcs[0].ilabel, 3);
    assert_eq!(s1.arcs[0].olabel, 4);
    assert_eq!(s1.arcs[0].weight, 0.0);
    let s2 = &f.states[2];
    assert_eq!(s2.final_state, true);
    assert_eq!(s2.weight, 1.5);
}

#[test]
fn test_compile_internal_with_symt() {
    use fslib::compile::compile_internal;
    let mut ist = SymTable::new();
    ist.add(0, "<eps>");
    ist.add(1, "a");
    ist.add(2, "b");
    let mut ost = SymTable::new();
    ost.add(0, "<eps>");
    ost.add(1, "x");
    ost.add(2, "y");
    let mut sst = SymTable::new();
    sst.add(0, "S0");
    sst.add(1, "S1");
    sst.add(2, "S2");
    sst.add(3, "<start>");

    let input = "S0\tS1\ta\tx\t0.5\nS1\tS2\tb\ty\nS2\t1.0\n";
    let mut br = std::io::Cursor::new(input.as_bytes());
    let mut f = Fst::new();
    compile_internal(&mut f, &mut br, Some(&ist), Some(&ost), Some(&sst), false).unwrap();
    // n_states reflects max state used (S2 = 2, but '<start>' has id 3)
    // Per C output we got: n_states=3 start=3
    assert_eq!(f.n_states, 3);
    assert_eq!(f.start, 3);
    let s0 = &f.states[0];
    assert_eq!(s0.arcs[0].state, 1);
    assert_eq!(s0.arcs[0].ilabel, 1);
    assert_eq!(s0.arcs[0].olabel, 1);
    assert_eq!(s0.arcs[0].weight, 0.5);
    let s1 = &f.states[1];
    assert_eq!(s1.arcs[0].state, 2);
    assert_eq!(s1.arcs[0].ilabel, 2);
    assert_eq!(s1.arcs[0].olabel, 2);
    let s2 = &f.states[2];
    assert_eq!(s2.final_state, true);
    assert_eq!(s2.weight, 1.0);
}

#[test]
fn test_compile_internal_acceptor_mode() {
    use fslib::compile::compile_internal;
    let mut ist = SymTable::new();
    ist.add(0, "<eps>");
    ist.add(1, "a");
    ist.add(2, "b");
    let mut sst = SymTable::new();
    sst.add(0, "S0");
    sst.add(1, "S1");
    sst.add(2, "S2");

    let input = "S0\tS1\ta\t0.5\nS1\tS2\tb\nS2\t1.0\n";
    let mut br = std::io::Cursor::new(input.as_bytes());
    let mut f = Fst::new();
    compile_internal(&mut f, &mut br, Some(&ist), None, Some(&sst), true).unwrap();
    assert_eq!(f.n_states, 3);
    let s0 = &f.states[0];
    // acceptor: ilabel == olabel
    assert_eq!(s0.arcs[0].state, 1);
    assert_eq!(s0.arcs[0].ilabel, 1);
    assert_eq!(s0.arcs[0].olabel, 1);
    assert_eq!(s0.arcs[0].weight, 0.5);
    let s1 = &f.states[1];
    assert_eq!(s1.arcs[0].state, 2);
    assert_eq!(s1.arcs[0].ilabel, 2);
    assert_eq!(s1.arcs[0].olabel, 2);
    assert_eq!(s1.arcs[0].weight, 0.0);
    let s2 = &f.states[2];
    assert_eq!(s2.final_state, true);
    assert_eq!(s2.weight, 1.0);
}

fn main() {}
