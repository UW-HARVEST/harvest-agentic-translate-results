pub fn print_bin(n: u64) -> String {
    let mut s = String::with_capacity(64);
    for i in (0..64).rev() {
        if n & (1u64 << i) != 0 {
            s.push('1');
        } else {
            s.push('0');
        }
    }
    s
}
