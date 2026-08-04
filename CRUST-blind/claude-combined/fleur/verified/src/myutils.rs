pub fn print_bin(n: u64) -> String {
    let mut s = String::with_capacity(64);
    let mut i: u64 = 1u64 << 63;
    while i > 0 {
        if n & i != 0 {
            s.push('1');
        } else {
            s.push('0');
        }
        i /= 2;
    }
    s
}
