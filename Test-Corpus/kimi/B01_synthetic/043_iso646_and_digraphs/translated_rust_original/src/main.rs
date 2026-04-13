use std::io;

fn driver(x: i32, y: i32) {
    let result = x | !y;
    print!("{}", result);
    println!();
}

fn main() {
    let mut x = 0;
    let mut y = 0;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    x = input.trim().parse().unwrap();
    
    input.clear();
    io::stdin().read_line(&mut input).unwrap();
    y = input.trim().parse().unwrap();
    
    driver(x, y);
}
