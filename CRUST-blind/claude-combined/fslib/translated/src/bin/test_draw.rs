use fslib::fst::Fst;
use fslib::draw::fst_draw;

#[test]
fn test_draw_basic() {
    let mut fst = Fst::new();
    fst.add_state();
    fst.add_state();
    fst.add_arc(0, 1, 1, 2, 0.5);
    fst.set_final(1, 0.0);

    let path = "/tmp/_rust_test_draw.dot";
    let mut f = std::fs::File::create(path).unwrap();
    fst_draw(&fst, &mut f).unwrap();
    drop(f);
    let s = std::fs::read_to_string(path).unwrap();
    assert!(s.contains("digraph T"));
    assert!(s.contains("rankdir = LR"));
    assert!(s.contains("0 [label = \"0\""));
    assert!(s.contains("1 [label = \"1\", shape = doublecircle"));
    assert!(s.contains("0 -> 1 [ label = \"1:2/0.5\""));
    let _ = std::fs::remove_file(path);
}

fn main() {}
