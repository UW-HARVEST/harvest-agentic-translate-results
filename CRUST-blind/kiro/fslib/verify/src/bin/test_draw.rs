use fslib::fst::Fst;
use std::io::Read;

#[test]
fn test_fst_draw() {
    let mut fst = Fst::new();
    fst.compile_str("0\t1\t1\t2\t0.5\n1\t0.0");
    let path = "/tmp/test_draw_output.dot";
    {
        let mut f = std::fs::File::create(path).unwrap();
        fslib::draw::fst_draw(&fst, &mut f).unwrap();
    }
    let mut content = String::new();
    std::fs::File::open(path).unwrap().read_to_string(&mut content).unwrap();
    assert!(content.contains("digraph T {"));
    assert!(content.contains("rankdir = LR"));
    assert!(content.contains("0 -> 1"));
    assert!(content.contains("1:2/"));
    assert!(content.contains("doublecircle"));
    assert!(content.contains("}\n"));
    std::fs::remove_file(path).ok();
}

fn main() {}
