//! Exploratory sweep: report the exact boundaries where C and Rust agree.
//! Not an assertion test — it prints a map of the input domain.
mod common;
use common::Libs;

#[test]
#[ignore]
fn map_domain() {
    let l = Libs::load();
    println!("C   : {}", l.c_path.display());
    println!("Rust: {}", l.rust_path.display());

    let mut runs: Vec<(i32, i32, bool)> = Vec::new();
    let probe = |x: i32| -> bool { l.c(x).to_bits() == l.rust(x).to_bits() };

    let mut x = -32i32;
    let mut start = x;
    let mut cur = probe(x);
    while x <= 20000 {
        let ok = probe(x);
        if ok != cur {
            runs.push((start, x - 1, cur));
            start = x;
            cur = ok;
        }
        x += 1;
    }
    runs.push((start, 20000, cur));
    for (a, b, ok) in &runs {
        println!("[{a}, {b}] -> {}", if *ok { "MATCH" } else { "DIFFER" });
    }

}
