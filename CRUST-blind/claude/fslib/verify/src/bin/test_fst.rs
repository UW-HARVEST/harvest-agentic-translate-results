use fslib::fst::{Fst, EPS, FST_HEADER, ISORT, OSORT, SR_REAL, SR_TROPICAL, START_STATE};

#[test]
fn test_fst_constants() {
    assert_eq!(FST_HEADER, 0x66733031);
    assert_eq!(ISORT, 0x01);
    assert_eq!(OSORT, 0x02);
    assert_eq!(EPS, 0);
    assert_eq!(SR_TROPICAL, 0);
    assert_eq!(SR_REAL, 1);
    assert_eq!(START_STATE, "<start>");
}

#[test]
fn test_fst_new() {
    let f = Fst::new();
    assert_eq!(f.start, 0);
    assert_eq!(f.n_states, 0);
    assert_eq!(f.n_max, 0);
    assert_eq!(f.sr_type, SR_TROPICAL);
    assert_eq!(f.flags, 0);
    assert_eq!(f.states.len(), 0);
}

#[test]
fn test_fst_add_state() {
    let mut f = Fst::new();
    let s0 = f.add_state();
    assert_eq!(s0, 0);
    assert_eq!(f.n_states, 1);
    assert_eq!(f.states.len(), 1);
    let s1 = f.add_state();
    assert_eq!(s1, 1);
    assert_eq!(f.n_states, 2);
    let state0 = &f.states[0];
    assert_eq!(state0.n_arcs, 0);
    assert_eq!(state0.weight, 0.0);
    assert_eq!(state0.final_state, false);
    assert_eq!(state0.arcs.len(), 0);
}

#[test]
fn test_fst_add_arc() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    let a = f.add_arc(0, 1, 5, 6, 0.5);
    assert_eq!(a, 0);
    let state0 = &f.states[0];
    assert_eq!(state0.n_arcs, 1);
    let arc = &state0.arcs[0];
    assert_eq!(arc.state, 1);
    assert_eq!(arc.ilabel, 5);
    assert_eq!(arc.olabel, 6);
    assert_eq!(arc.weight, 0.5);
}

#[test]
fn test_fst_set_final() {
    let mut f = Fst::new();
    f.add_state();
    f.set_final(0, 0.25);
    let state = &f.states[0];
    assert_eq!(state.final_state, true);
    assert_eq!(state.weight, 0.25);
}

#[test]
fn test_fst_get_n_arcs() {
    let mut f = Fst::new();
    for _ in 0..4 {
        f.add_state();
    }
    f.add_arc(0, 1, 1, 1, 0.0);
    f.add_arc(0, 2, 2, 2, 0.0);
    f.add_arc(1, 2, 3, 3, 0.0);
    f.add_arc(2, 3, 4, 4, 0.0);
    f.add_arc(2, 3, 5, 5, 0.0);
    assert_eq!(f.get_n_arcs(), 5);
}

#[test]
fn test_fst_empty() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 1, 1, 0.0);
    f.start = 1;
    f.empty();
    assert_eq!(f.n_states, 0);
    assert_eq!(f.n_max, 0);
    assert_eq!(f.start, 0);
    assert_eq!(f.states.len(), 0);
}

#[test]
fn test_fst_arc_sort_input() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 5, 1, 0.0);
    f.add_arc(0, 1, 3, 2, 0.0);
    f.add_arc(0, 1, 1, 3, 0.0);
    f.add_arc(0, 1, 2, 4, 0.0);
    f.arc_sort(0); // input sort
    assert_eq!(f.flags, ISORT);
    let arcs = &f.states[0].arcs;
    assert_eq!(arcs[0].ilabel, 1);
    assert_eq!(arcs[0].olabel, 3);
    assert_eq!(arcs[1].ilabel, 2);
    assert_eq!(arcs[1].olabel, 4);
    assert_eq!(arcs[2].ilabel, 3);
    assert_eq!(arcs[2].olabel, 2);
    assert_eq!(arcs[3].ilabel, 5);
    assert_eq!(arcs[3].olabel, 1);
}

#[test]
fn test_fst_arc_sort_output() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 5, 1, 0.0);
    f.add_arc(0, 1, 3, 2, 0.0);
    f.add_arc(0, 1, 1, 3, 0.0);
    f.add_arc(0, 1, 2, 4, 0.0);
    // First sort by input (sets ISORT flag)
    f.arc_sort(0);
    f.arc_sort(1); // output sort, sets OSORT
    // C: flags is 0x03 after both sorts
    assert_eq!(f.flags, ISORT | OSORT);
    let arcs = &f.states[0].arcs;
    // Sorted by olabel: 1, 2, 3, 4
    assert_eq!(arcs[0].olabel, 1);
    assert_eq!(arcs[1].olabel, 2);
    assert_eq!(arcs[2].olabel, 3);
    assert_eq!(arcs[3].olabel, 4);
    // Corresponding ilabels
    assert_eq!(arcs[0].ilabel, 5);
    assert_eq!(arcs[1].ilabel, 3);
    assert_eq!(arcs[2].ilabel, 1);
    assert_eq!(arcs[3].ilabel, 2);
}

#[test]
fn test_fst_stack() {
    // a: 2 states, 1 arc; b: 2 states, 1 arc
    let mut a = Fst::new();
    a.add_state();
    a.add_state();
    a.add_arc(0, 1, 1, 1, 1.0);
    a.set_final(1, 0.0);
    let mut b = Fst::new();
    b.add_state();
    b.add_state();
    b.add_arc(0, 1, 2, 2, 2.0);
    b.set_final(1, 0.0);
    a.stack(&b);
    assert_eq!(a.n_states, 4);
    // state 0: arc -> 1
    let s0 = &a.states[0];
    assert_eq!(s0.n_arcs, 1);
    assert_eq!(s0.arcs[0].state, 1);
    assert_eq!(s0.arcs[0].ilabel, 1);
    assert_eq!(s0.arcs[0].olabel, 1);
    // state 1: final
    let s1 = &a.states[1];
    assert_eq!(s1.final_state, true);
    // state 2: arc -> 3 (offset by 2)
    let s2 = &a.states[2];
    assert_eq!(s2.n_arcs, 1);
    assert_eq!(s2.arcs[0].state, 3);
    assert_eq!(s2.arcs[0].ilabel, 2);
    assert_eq!(s2.arcs[0].olabel, 2);
    assert_eq!(s2.arcs[0].weight, 2.0);
    // state 3: final
    let s3 = &a.states[3];
    assert_eq!(s3.final_state, true);
}

#[test]
fn test_fst_copy() {
    let mut a = Fst::new();
    for _ in 0..3 {
        a.add_state();
    }
    a.start = 1;
    a.sr_type = SR_REAL;
    a.add_arc(0, 1, 1, 2, 0.5);
    a.set_final(2, 0.25);
    let mut copy = Fst::new();
    a.copy(&mut copy);
    assert_eq!(copy.start, 1);
    assert_eq!(copy.n_states, 3);
    assert_eq!(copy.sr_type, SR_REAL);
    let s0 = &copy.states[0];
    assert_eq!(s0.n_arcs, 1);
    assert_eq!(s0.final_state, false);
    assert_eq!(s0.arcs[0].state, 1);
    assert_eq!(s0.arcs[0].ilabel, 1);
    assert_eq!(s0.arcs[0].olabel, 2);
    assert_eq!(s0.arcs[0].weight, 0.5);
    let s2 = &copy.states[2];
    assert_eq!(s2.final_state, true);
    assert_eq!(s2.weight, 0.25);
}

#[test]
fn test_fst_relabel_input() {
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.add_arc(0, 1, 1, 5, 0.0);
    f.add_arc(1, 2, 5, 1, 0.0);
    f.relabel(1, 99, 0); // input direction
    let arcs0 = &f.states[0].arcs;
    let arcs1 = &f.states[1].arcs;
    assert_eq!(arcs0[0].ilabel, 99);
    assert_eq!(arcs0[0].olabel, 5);
    assert_eq!(arcs1[0].ilabel, 5);
    assert_eq!(arcs1[0].olabel, 1);
}

#[test]
fn test_fst_relabel_output() {
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.add_arc(0, 1, 1, 5, 0.0);
    f.add_arc(1, 2, 5, 1, 0.0);
    f.relabel(5, 88, 1); // output direction
    let arcs0 = &f.states[0].arcs;
    let arcs1 = &f.states[1].arcs;
    assert_eq!(arcs0[0].ilabel, 1);
    assert_eq!(arcs0[0].olabel, 88);
    assert_eq!(arcs1[0].ilabel, 5);
    assert_eq!(arcs1[0].olabel, 1);
}

#[test]
fn test_fst_union() {
    let mut a = Fst::new();
    a.add_state();
    a.add_state();
    a.add_arc(0, 1, 1, 1, 1.0);
    a.set_final(1, 0.0);
    let mut b = Fst::new();
    b.add_state();
    b.add_state();
    b.add_arc(0, 1, 2, 2, 2.0);
    b.set_final(1, 0.0);
    let result = a.union(&b);
    assert_eq!(result.n_states, 4);
    // a is now empty
    assert_eq!(a.n_states, 0);
}

#[test]
fn test_fst_write_read_roundtrip() {
    use std::fs::File;
    let tmpfile = "/tmp/test_fst_io.fst";
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.start = 1;
    f.sr_type = SR_REAL;
    f.add_arc(0, 1, 1, 2, 0.5);
    f.add_arc(1, 2, 3, 4, 1.5);
    f.set_final(2, 0.25);

    let mut writer = File::create(tmpfile).unwrap();
    f.write(&mut writer).unwrap();
    drop(writer);

    let mut f2 = Fst::new();
    let mut reader = File::open(tmpfile).unwrap();
    f2.read(&mut reader).unwrap();

    assert_eq!(f2.start, 1);
    assert_eq!(f2.n_states, 3);
    assert_eq!(f2.sr_type, SR_REAL);
    assert_eq!(f2.flags, 0);
    let s0 = &f2.states[0];
    assert_eq!(s0.n_arcs, 1);
    assert_eq!(s0.final_state, false);
    assert_eq!(s0.arcs[0].state, 1);
    assert_eq!(s0.arcs[0].ilabel, 1);
    assert_eq!(s0.arcs[0].olabel, 2);
    assert_eq!(s0.arcs[0].weight, 0.5);
    let s2 = &f2.states[2];
    assert_eq!(s2.n_arcs, 0);
    assert_eq!(s2.final_state, true);
    assert_eq!(s2.weight, 0.25);
    std::fs::remove_file(tmpfile).ok();
}

#[test]
fn test_fst_fwrite_fread() {
    let tmpfile = "/tmp/test_fst_fread.fst";
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 7, 8, 0.75);
    f.set_final(1, 0.0);
    f.fwrite(tmpfile).unwrap();

    let mut f2 = Fst::new();
    f2.fread(tmpfile).unwrap();
    assert_eq!(f2.n_states, 2);
    assert_eq!(f2.states[0].arcs[0].ilabel, 7);
    assert_eq!(f2.states[0].arcs[0].olabel, 8);
    assert_eq!(f2.states[0].arcs[0].weight, 0.75);
    assert_eq!(f2.states[1].final_state, true);
    std::fs::remove_file(tmpfile).ok();
}

#[test]
fn test_fst_compile_str() {
    let mut tmp = Fst::new();
    let s = "0\t1\t1\t2\t0.5\n1\t2\t3\t4\n2\t1.5\n";
    let f = tmp.compile_str(s);
    // After compile_str, tmp is reset, the result is in `f`
    assert_eq!(f.n_states, 3);
    let s0 = &f.states[0];
    assert_eq!(s0.n_arcs, 1);
    assert_eq!(s0.arcs[0].state, 1);
    assert_eq!(s0.arcs[0].ilabel, 1);
    assert_eq!(s0.arcs[0].olabel, 2);
    assert_eq!(s0.arcs[0].weight, 0.5);
    let s1 = &f.states[1];
    assert_eq!(s1.arcs[0].state, 2);
    assert_eq!(s1.arcs[0].ilabel, 3);
    assert_eq!(s1.arcs[0].olabel, 4);
    // weight defaults to sr.one (tropical) = 0
    assert_eq!(s1.arcs[0].weight, 0.0);
    let s2 = &f.states[2];
    assert_eq!(s2.final_state, true);
    assert_eq!(s2.weight, 1.5);
}

fn main() {}
