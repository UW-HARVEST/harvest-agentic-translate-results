mod common;
#[test]
fn loads(){ let p = common::pair(); println!("{} {}", p.c.path.display(), p.rs.path.display()); }
