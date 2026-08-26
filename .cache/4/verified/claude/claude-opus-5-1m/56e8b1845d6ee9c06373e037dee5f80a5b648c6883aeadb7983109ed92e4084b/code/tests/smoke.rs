//! Harness sanity check: both `.so`s load, all 13 symbols resolve, and the two
//! libraries really are independent objects.

mod common;
use common::*;

#[test]
fn smoke_both_libs_load_and_agree_on_a_basic_call() {
    with_libs(|p| {
        println!("C   .so = {}", p.c.path.display());
        println!("Rust.so = {}", p.rust.path.display());
        assert_eq!(p.c.get_count(), 0);
        assert_eq!(p.rust.get_count(), 0);
        assert_eq!(p.c.table_image().len(), NODE_TABLE_BYTES);
        assert_eq!(p.rust.table_image().len(), NODE_TABLE_BYTES);

        let cv = (p.c.inreftree)(1, 2, 3, 4);
        let rv = (p.rust.inreftree)(1, 2, 3, 4);
        assert_ret_eq(cv, rv, "inreftree(1,2,3,4)");
        assert_state_eq(p, "inreftree(1,2,3,4)");
        println!("inreftree(1,2,3,4) = {cv}");
    });
}
