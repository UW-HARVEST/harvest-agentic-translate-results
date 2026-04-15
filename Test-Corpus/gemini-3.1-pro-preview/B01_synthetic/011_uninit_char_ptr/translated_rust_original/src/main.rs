use std::io;

fn print_line(line: Option<&str>) {
    if let Some(l) = line {
        println!("{}", l);
    }
}

fn bad() {
    let data: Option<&str> = None;
    print_line(data);
}

fn good() {
    let data = Some("string");
    print_line(data);
}

fn main() {
    let mut x = 0;
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Some(token) = input.split_whitespace().next() {
            if let Ok(parsed) = token.parse::<i32>() {
                x = parsed;
            }
        }
    }

    if x != 0 {
        good();
    } else {
        bad();
    }
}
