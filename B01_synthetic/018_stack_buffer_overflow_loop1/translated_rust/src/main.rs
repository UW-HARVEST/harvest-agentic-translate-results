fn main() {
    // Delegate to the lib's exported functions
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap_or(0);
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        driver::good();
    } else {
        driver::bad();
    }
}
