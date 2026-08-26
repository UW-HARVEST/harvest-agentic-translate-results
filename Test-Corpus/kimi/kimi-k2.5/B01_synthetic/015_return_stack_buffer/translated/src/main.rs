use std::io;

fn print_line(line: Option<&str>) {
    if let Some(l) = line {
        println!("{}", l);
    }
}

fn helper_bad() -> Option<&'static str> {
    None
}

fn bad() {
    print_line(helper_bad());
}

fn helper_good1() -> &'static str {
    "helperGood1 string"
}

fn good() {
    print_line(Some(helper_good1()));
}

fn main() {
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let x: i32 = input.trim().parse().unwrap_or(0);

    if x != 0 {
        good();
    } else {
        bad();
    }
}