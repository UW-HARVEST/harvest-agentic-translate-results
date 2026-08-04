use fslib::draw::{fst_draw, fst_draw_sym};
use fslib::fst::Fst;
use fslib::symt::SymTable;
use std::fs::File;
use std::io::Read;

fn read_to_string(path: &str) -> String {
    let mut f = File::open(path).unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
    s
}

#[test]
fn test_draw_basic() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 1, 2, 0.5);
    f.set_final(1, 0.0);
    f.start = 0;

    let path = "/tmp/test_draw_basic.dot";
    let mut fp = File::create(path).unwrap();
    fst_draw(&f, &mut fp).unwrap();
    drop(fp);
    let s = read_to_string(path);
    // Header
    assert!(s.contains("digraph T {"));
    assert!(s.contains("rankdir = LR;"));
    assert!(s.contains("orientation = Landscape;"));
    // start state (0) is filled circle
    assert!(s.contains("\t0 [label = \"0\", shape = circle, style = filled ];\n"));
    // final state (1) is doublecircle
    assert!(s.contains("\t1 [label = \"1\", shape = doublecircle, style = filled ];\n"));
    // arc
    assert!(s.contains("\t\t0 -> 1 [ label = \"1:2/0.5\" ];\n"));
    // footer
    assert!(s.ends_with("}\n"));
    std::fs::remove_file(path).ok();
}

#[test]
fn test_draw_non_start_non_final() {
    let mut f = Fst::new();
    for _ in 0..3 {
        f.add_state();
    }
    f.start = 0;
    f.add_arc(0, 1, 1, 1, 0.0);
    f.add_arc(1, 2, 2, 2, 0.0);
    f.set_final(2, 0.0);

    let path = "/tmp/test_draw_middle.dot";
    let mut fp = File::create(path).unwrap();
    fst_draw(&f, &mut fp).unwrap();
    drop(fp);
    let s = read_to_string(path);
    // state 0 = start (filled), state 1 = solid, state 2 = doublecircle
    assert!(s.contains("\t0 [label = \"0\", shape = circle, style = filled ];\n"));
    assert!(s.contains("\t1 [label = \"1\", shape = circle, style = solid ];\n"));
    assert!(s.contains("\t2 [label = \"2\", shape = doublecircle, style = filled ];\n"));
    std::fs::remove_file(path).ok();
}

#[test]
fn test_draw_sym() {
    let mut f = Fst::new();
    f.add_state();
    f.add_state();
    f.add_arc(0, 1, 1, 1, 0.0);
    f.set_final(1, 0.0);
    f.start = 0;

    let mut ist = SymTable::new();
    ist.add(0, "<eps>");
    ist.add(1, "a");
    let mut ost = SymTable::new();
    ost.add(0, "<eps>");
    ost.add(1, "x");
    let mut sst = SymTable::new();
    sst.add(0, "S0");
    sst.add(1, "S1");

    let path = "/tmp/test_draw_sym.dot";
    let mut fp = File::create(path).unwrap();
    fst_draw_sym(&f, &mut fp, Some(&ist), Some(&ost), Some(&sst)).unwrap();
    drop(fp);
    let s = read_to_string(path);
    assert!(s.contains("\tS0 [label = \"S0\", shape = circle, style = filled ];\n"));
    assert!(s.contains("\tS1 [label = \"S1\", shape = doublecircle, style = filled ];\n"));
    assert!(s.contains("\t\tS0 -> S1 [ label = \"a:x/0\" ];\n"));
    std::fs::remove_file(path).ok();
}

fn main() {}
