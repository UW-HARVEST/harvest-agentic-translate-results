fn print_line(line: &str) {
    println!("{}", line);
}

fn print_int_line(int_number: i32) {
    println!("{}", int_number);
}

fn bad() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let int_sum: i32 = 0;
    print_int_line(int_sum);
    let _ = int_one + int_two; // Bug: result discarded, not assigned to int_sum
    print_int_line(int_sum);
}

fn good() {
    let int_one: i32 = 1;
    let int_two: i32 = 1;
    let mut int_sum: i32 = 0;
    print_int_line(int_sum);
    int_sum = int_one + int_two;
    print_int_line(int_sum);
}

fn main() {
    print_line("Calling good()...");
    good();
    print_line("Finished good()");
    print_line("Calling bad()...");
    bad();
    print_line("Finished bad()");
}
