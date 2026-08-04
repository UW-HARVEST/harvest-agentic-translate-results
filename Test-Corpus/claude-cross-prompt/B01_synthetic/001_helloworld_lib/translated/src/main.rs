fn helloworld() -> i32 {
    println!("Hello World!");
    0
}

fn main() {
    std::process::exit(helloworld());
}
