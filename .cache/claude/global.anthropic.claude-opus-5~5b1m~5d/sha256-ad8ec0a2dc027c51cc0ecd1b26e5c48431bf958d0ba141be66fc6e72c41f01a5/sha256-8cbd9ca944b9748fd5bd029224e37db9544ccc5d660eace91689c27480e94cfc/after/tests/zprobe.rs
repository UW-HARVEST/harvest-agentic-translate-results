mod common;
use common::*;
#[test]
fn probe() {
    let (c, r) = both();
    for (p1,p2,p3,p4) in [(2024320644i32, 88653697i32, 0i32, 0i32),
                          (-1838743604, 88653697, -106139262, -1568269531)] {
        println!("\n=== p=({p1},{p2},{p3},{p4}) ===");
        // rebuild arity4 out of the C library's OWN exported helpers
        let mut vals = [p1,p2,p3,p4];
        let len1 = unsafe{(c.process_string)(cbuf(b"Hello").as_ptr())};
        let len2 = unsafe{(c.process_string)(cbuf(b"").as_ptr())};
        let mut result = len1 + len2;
        unsafe{(c.shift_array)(vals.as_mut_ptr(),4,1)};
        println!("len1={len1} len2={len2} vals={vals:?}");
        for i in 0..4 { result = result.wrapping_add(vals[i]); }
        println!("sum result={result}");
        let op = p1 % 4;
        let masked = unsafe{(c.apply_bitmask)(result, op)};
        println!("op={op} masked={masked}");
        let mut m=[0i32;12]; unsafe{(c.init_matrix)(m.as_mut_ptr())};
        let mut result = masked.wrapping_add(m[0]).wrapping_add(m[11]);
        println!("after matrix={result}");
        normalize_heap(true);
        let ca = unsafe{(c.compare_allocations)(p1,p2)};
        result = result.wrapping_add(ca);
        println!("compare_allocations={ca} -> {result}");
        if p3 != 0 { result = result.wrapping_mul(p3).wrapping_div(100); println!("after p3 div={result}"); }
        if p4 != 0 { result = result.wrapping_add(p4); println!("after p4={result}"); }
        println!("RECONSTRUCTED = {result}");
        normalize_heap(true);
        let cv = unsafe{(c.arity4)(p1,p2,p3,p4)};
        normalize_heap(true);
        let rv = unsafe{(r.arity4)(p1,p2,p3,p4)};
        println!("C arity4 = {cv}   Rust arity4 = {rv}");
    }
}
