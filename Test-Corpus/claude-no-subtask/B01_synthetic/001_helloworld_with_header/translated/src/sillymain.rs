use std::io::Write;

pub fn helloworld() -> i32 {
    print!("Hello World!\n");
    std::io::stdout().flush().ok();
    0
}
