// Translated from c_src/src/driver.c
// Original: void driver(int x) { register int y = 2*x; y += 300; printf("%d\n", y); }

pub fn driver(x: i32) {
    let mut y: i32 = x.wrapping_mul(2);
    y = y.wrapping_add(300);
    println!("{}", y);
}
